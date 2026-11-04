import type { RuntimeEvent } from "../runtime-adapter";
import { blocks, parsedArguments as parseArguments, record } from "./deepseek-events";

interface NativeEvent {
  type: string;
  seq: number;
  time?: number;
  data: Record<string, unknown>;
}

interface StreamEnvelope {
  type: "stream.event";
  eventType: string;
  data: Record<string, unknown>;
  created: number;
}

function resultContent(message: Record<string, unknown> | null): Array<Record<string, unknown>> {
  return blocks(message?.content).flatMap((item) => item.type === "tool-result" ? blocks(item.content) : []);
}

function resultFailed(message: Record<string, unknown> | null, data: Record<string, unknown>): boolean {
  const source = record(message?.source);
  return data.error !== undefined || source?.isError === true || message?.isError === true;
}

function stream(eventType: string, data: Record<string, unknown>, created: number): StreamEnvelope {
  return { type: "stream.event", eventType, data, created };
}

function coordinates(data: Record<string, unknown>): { turn: number; step: number } {
  return { turn: Number(data.turn ?? 0), step: Number(data.step ?? 0) };
}

function messageID(sessionID: string, data: Record<string, unknown>): string {
  const { turn, step } = coordinates(data);
  return `deepseek:${sessionID}:${String(turn).padStart(8, "0")}:${String(step).padStart(8, "0")}`;
}

export class DeepSeekStreamProjector {
  private readonly callMessages = new Map<string, string>();
  private readonly callViews = new Map<string, unknown>();
  private readonly blockCalls = new Map<string, string>();
  private readonly subCalls = new Map<string, Map<string, Record<string, unknown>>>();

  project(sessionID: string, event: NativeEvent, view?: unknown): RuntimeEvent[] {
    const created = event.time ?? Date.now();
    const data = event.data;
    const assistantMessageID = messageID(sessionID, data);
    if (event.type === "step/start") {
      return [stream("session.step.started", { assistantMessageID }, created)];
    }
    if (event.type === "step/end") {
      return [stream("session.step.ended", { assistantMessageID }, created)];
    }
    if (event.type === "assistant/chunk") return this.chunk(sessionID, assistantMessageID, record(data.chunk), created);
    if (event.type === "assistant/message") return this.message(sessionID, assistantMessageID, record(data.message), created, view);
    if (event.type === "tool/call") {
      const callID = String(data.callId ?? "");
      if (!callID) return [];
      this.callMessages.set(callID, assistantMessageID);
      const callView = record(view)?.view ?? view;
      this.callViews.set(callID, callView);
      return [stream("session.tool.called", {
        assistantMessageID,
        callID,
        name: String(data.name ?? "tool"),
        input: parseArguments(data.arguments),
        state: callView,
        metadata: { deepseek: { callView } }
      }, created)];
    }
    if (event.type === "tool/result") {
      const message = record(data.message);
      const source = record(message?.source);
      const callID = String(source?.callId ?? "");
      if (!callID) return [];
      const owner = this.callMessages.get(callID) ?? assistantMessageID;
      const content = resultContent(message);
      const resultView = record(view)?.view ?? view;
      const terminalFailure = record(resultView)?.card === "terminal" &&
        (Number(record(resultView)?.exitCode ?? 0) !== 0 || typeof record(resultView)?.signal === "string");
      return [stream(resultFailed(message, data) || terminalFailure ? "session.tool.failed" : "session.tool.success", {
        assistantMessageID: owner,
        callID,
        content,
        output: content.filter((item) => item.type === "text").map((item) => String(item.text ?? "")).join("\n"),
        error: data.error,
        resultState: resultView,
        metadata: { deepseek: { callView: this.callViews.get(callID), resultView, subCalls: this.subCallList(callID) } }
      }, created)];
    }
    if (event.type === "tool/code-dispatch-start" || event.type === "tool/code-dispatch") {
      const rootCallID = String(data.rootCallId ?? "");
      const subCallID = String(data.subCallId ?? "");
      if (!rootCallID || !subCallID) return [];
      const children = this.subCalls.get(rootCallID) ?? new Map<string, Record<string, unknown>>();
      const content = blocks(data.content);
      const failed = data.isError === true;
      children.set(subCallID, {
        id: subCallID,
        title: String(data.name ?? "tool"),
        detail: "",
        status: event.type === "tool/code-dispatch-start" ? "running" : failed ? "failed" : "success",
        inputValue: data.arguments,
        input: JSON.stringify(data.arguments ?? {}, null, 2),
        output: content.filter((item) => item.type === "text").map((item) => String(item.text ?? "")).join("\n"),
        startedAt: created
      });
      this.subCalls.set(rootCallID, children);
      return [stream("session.tool.progress", {
        assistantMessageID: this.callMessages.get(rootCallID) ?? assistantMessageID,
        callID: rootCallID,
        progress: event.type === "tool/code-dispatch-start" ? "Running delegated command" : undefined,
        metadata: { deepseek: { callView: this.callViews.get(rootCallID), subCalls: this.subCallList(rootCallID) } }
      }, created)];
    }
    if (event.type === "llm/retry") {
      const failure = record(data.failure);
      return [
        stream("message.removed", { messageID: assistantMessageID }, created),
        stream("session.retry.scheduled", {
          assistantMessageID,
          attempt: Number(data.retry ?? 1),
          at: created + Number(data.delayMs ?? 0),
          error: failure ?? data.failure
        }, created)
      ];
    }
    return [];
  }

  private subCallList(callID: string): Record<string, unknown>[] {
    return [...(this.subCalls.get(callID)?.values() ?? [])];
  }

  private chunk(sessionID: string, assistantMessageID: string, chunk: Record<string, unknown> | null, created: number): RuntimeEvent[] {
    if (!chunk) return [];
    const ordinal = Number(chunk.index ?? 0);
    if (chunk.type === "block-start") {
      if (chunk.blockType === "text") return [stream("session.text.started", { assistantMessageID, ordinal }, created)];
      if (chunk.blockType === "reasoning") return [stream("session.reasoning.started", { assistantMessageID, ordinal }, created)];
      return [];
    }
    if (chunk.type === "text-delta") return [stream("session.text.delta", { assistantMessageID, ordinal, delta: String(chunk.text ?? "") }, created)];
    if (chunk.type === "reasoning-delta") return [stream("session.reasoning.delta", { assistantMessageID, ordinal, delta: String(chunk.text ?? "") }, created)];
    if (chunk.type === "tool-call-delta") {
      const key = `${sessionID}:${assistantMessageID}:${String(ordinal)}`;
      const callID = String(chunk.id ?? this.blockCalls.get(key) ?? "");
      if (!callID) return [];
      const first = !this.blockCalls.has(key);
      this.blockCalls.set(key, callID);
      this.callMessages.set(callID, assistantMessageID);
      return [
        ...(first ? [stream("session.tool.input.started", { assistantMessageID, callID, name: String(chunk.name ?? "tool") }, created)] : []),
        stream("session.tool.input.delta", { assistantMessageID, callID, delta: String(chunk.argumentsDelta ?? "") }, created)
      ];
    }
    if (chunk.type !== "block-end") return [];
    const block = record(chunk.block);
    if (!block) return [];
    if (block.type === "text") return [stream("session.text.ended", { assistantMessageID, ordinal, text: String(block.text ?? "") }, created)];
    if (block.type === "reasoning") return [stream("session.reasoning.ended", { assistantMessageID, ordinal, text: String(block.text ?? "") }, created)];
    if (block.type !== "tool-call") return [];
    const callID = String(block.id ?? "");
    if (!callID) return [];
    this.callMessages.set(callID, assistantMessageID);
    return [
      stream("session.tool.input.ended", { assistantMessageID, callID, text: String(block.arguments ?? "") }, created),
      stream("session.tool.called", { assistantMessageID, callID, name: String(block.name ?? "tool"), input: parseArguments(block.arguments) }, created)
    ];
  }

  private message(sessionID: string, assistantMessageID: string, message: Record<string, unknown> | null, created: number, view?: unknown): RuntimeEvent[] {
    if (!message) return [];
    const callView = record(view)?.view ?? view;
    const events: RuntimeEvent[] = [stream("message.updated", {
      info: { id: assistantMessageID, sessionID, role: "assistant", time: { created, completed: created } }
    }, created)];
    for (const [ordinal, block] of blocks(message.content).entries()) {
      const partID = `${assistantMessageID}:${String(ordinal).padStart(8, "0")}`;
      if (block.type === "text" || block.type === "reasoning") {
        events.push(stream("message.part.updated", {
          part: { id: partID, messageID: assistantMessageID, sessionID, type: block.type, text: String(block.text ?? ""), time: { created, completed: created } }
        }, created));
      } else if (block.type === "tool-call") {
        const callID = String(block.id ?? partID);
        this.callMessages.set(callID, assistantMessageID);
        this.callViews.set(callID, callView);
        events.push(stream("message.part.updated", {
          part: {
            id: partID,
            messageID: assistantMessageID,
            sessionID,
            type: "tool",
            callID,
            name: String(block.name ?? "tool"),
            state: { status: "running", input: parseArguments(block.arguments), metadata: { deepseek: { callView } } },
            time: { created }
          }
        }, created));
      }
    }
    return events;
  }
}
