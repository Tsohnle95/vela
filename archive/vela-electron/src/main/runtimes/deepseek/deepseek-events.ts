import type { AssistantPartView, SessionTranscript, TodoItem, ToolCallView, TranscriptItem } from "@shared/types";
import { formatFailure, normalizeFailure } from "@shared/errors";
import type { RuntimeEvent } from "../runtime-adapter";

interface HistoryEntry {
  event: {
    type: string;
    seq: number;
    time?: number;
    data: Record<string, unknown>;
  };
  view?: unknown;
}

export function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;
}

export function blocks(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.filter((item): item is Record<string, unknown> => record(item) !== null) : [];
}

function blockText(value: unknown): string {
  return blocks(value).filter((item) => item.type === "text").map((item) => String(item.text ?? "")).join("\n");
}

export function parsedArguments(value: unknown): unknown {
  if (typeof value !== "string") return value;
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function viewTitle(view: unknown, fallback: string): string {
  const direct = record(view);
  const nested = record(direct?.call);
  return String(nested?.title ?? direct?.title ?? fallback);
}

function viewPaths(view: unknown): string[] | undefined {
  const direct = record(view);
  const nested = record(direct?.call);
  const locations = nested?.locations ?? direct?.locations;
  if (!Array.isArray(locations)) return undefined;
  const paths = locations.flatMap((item) => {
    const location = record(item);
    return typeof location?.path === "string" ? [location.path] : [];
  });
  return paths.length > 0 ? paths : undefined;
}

function toolOutput(message: unknown): string {
  const content = record(message)?.content;
  return blocks(content).flatMap((item) => item.type === "tool-result" ? [blockText(item.content)] : []).join("\n");
}

function todoList(value: unknown): TodoItem[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((raw, index): TodoItem[] => {
    const item = record(raw);
    if (!item || typeof item.content !== "string") return [];
    const rawStatus = String(item.status ?? "pending");
    const status = rawStatus === "in-progress" ? "in_progress" : rawStatus;
    return [{
      id: typeof item.id === "string" ? item.id : `todo-${index}`,
      content: item.content,
      status: ["pending", "in_progress", "completed", "cancelled"].includes(status)
        ? status as TodoItem["status"]
        : "pending",
      ...(typeof item.priority === "string" ? { priority: item.priority } : {})
    }];
  });
}

function internalContext(source: Record<string, unknown> | null): boolean {
  return source?.kind === "agent-instructions" ||
    (source?.kind === "plugin" && source.plugin === "@deepseek-ai/dsh-system-prompt");
}

function turnFailure(data: Record<string, unknown>) {
  const reason = record(data.reason);
  if (reason?.kind !== "error") return null;
  const error = record(reason.error) ?? record(reason.failure);
  return normalizeFailure(error ?? reason.failure, "DEEPSEEK_TURN_FAILED", "DeepSeek Harness failed");
}

function turnError(data: Record<string, unknown>): string | null {
  const failure = turnFailure(data);
  return failure ? formatFailure(failure) : null;
}

export function deepSeekTranscript(entries: HistoryEntry[]): SessionTranscript {
  const transcript: TranscriptItem[] = [];
  const tools = new Map<string, ToolCallView>();
  let todos: TodoItem[] = [];
  const ordered = [...entries].sort((a, b) => a.event.seq - b.event.seq);
  for (const entry of ordered) {
    const { event } = entry;
    if (event.type === "user/message") {
      const source = record(event.data.source);
      const text = blockText(event.data.content);
      if (!text || internalContext(source)) continue;
      if (source?.kind === "user") transcript.push({ kind: "user", id: String(event.data.id ?? `deepseek-user-${event.seq}`), text });
      else transcript.push({ kind: "synthetic", id: `deepseek-context-${event.seq}`, text, ...(typeof source?.summary === "string" ? { description: source.summary } : {}) });
      continue;
    }
    if (event.type === "assistant/message") {
      const message = record(event.data.message);
      if (!message) continue;
      const content = blocks(message.content);
      const parts = content.flatMap((block, index): AssistantPartView[] => {
        const id = `deepseek-${event.seq}-${index}`;
        if ((block.type === "text" || block.type === "reasoning") && typeof block.text === "string") {
          return [{ kind: block.type, id, text: block.text, complete: true }];
        }
        if (block.type !== "tool-call") return [];
        const callID = String(block.id ?? id);
        const inputValue = parsedArguments(block.arguments);
        const tool: ToolCallView = {
          id: callID,
          title: String(block.name ?? "tool"),
          detail: "",
          status: "running",
          input: typeof inputValue === "string" ? inputValue : JSON.stringify(inputValue, null, 2),
          inputValue,
          startedAt: event.time,
          paths: viewPaths(entry.view)
        };
        tools.set(callID, tool);
        return [{ kind: "tool", id, tool }];
      });
      transcript.push({
        kind: "assistant",
        id: String(message.id ?? `deepseek-assistant-${event.seq}`),
        messageID: String(message.id ?? `deepseek-assistant-${event.seq}`),
        parts,
        completed: true
      });
      continue;
    }
    if (event.type === "tool/call") {
      const callID = String(event.data.callId ?? `deepseek-tool-${event.seq}`);
      const inputValue = parsedArguments(event.data.arguments);
      const tool: ToolCallView = {
        id: callID,
        title: viewTitle(entry.view, String(event.data.name ?? "tool")),
        detail: "",
        status: "running",
        input: typeof inputValue === "string" ? inputValue : JSON.stringify(inputValue, null, 2),
        inputValue,
        startedAt: event.time,
        paths: viewPaths(entry.view)
      };
      tools.set(callID, tool);
      const last = transcript.findLast((item) => item.kind === "assistant");
      if (last?.kind === "assistant" && !last.parts.some((part) => part.kind === "tool" && part.tool.id === callID)) {
        last.parts.push({ kind: "tool", id: `deepseek-tool-part-${event.seq}`, tool });
      }
      continue;
    }
    if (event.type === "tool/result") {
      const message = record(event.data.message);
      const source = record(message?.source);
      const callID = String(source?.callId ?? "");
      const tool = tools.get(callID);
      if (tool) {
        tool.status = event.data.error ? "failed" : "success";
        tool.output = toolOutput(message) || (event.data.error !== undefined
          ? formatFailure(event.data.error, "DEEPSEEK_TOOL_FAILED", "Tool failed")
          : "");
        tool.duration = event.time && tool.startedAt ? Math.max(0, event.time - tool.startedAt) : undefined;
        tool.title = viewTitle(entry.view, tool.title);
      }
      continue;
    }
    if (event.type === "turn/end") {
      const error = turnError(event.data);
      if (error) transcript.push({ kind: "status", id: `deepseek-error-${event.seq}`, text: error, tone: "error" });
      continue;
    }
    if (event.type === "todo/write") todos = todoList(event.data.todos);
  }
  return { transcript, todos };
}

export function deepSeekRuntimeEvent(payload: Record<string, unknown>): RuntimeEvent | null {
  if (payload.type === "host/session-status") return payload.running ? { type: "execution.started" } : { type: "execution.idle" };
  if (payload.type === "host/agent-error") {
    const failure = normalizeFailure(payload, "DEEPSEEK_AGENT_FAILED", "DeepSeek Harness failed");
    return { type: "execution.error", code: failure.code, message: failure.message };
  }
  if (payload.type !== "session/event") return null;
  const event = record(payload.event);
  if (event?.type === "turn/start") return { type: "execution.started" };
  if (event?.type === "turn/end") {
    const data = record(event.data);
    if (!data) return { type: "execution.error", code: "DEEPSEEK_TURN_INVALID", message: "DeepSeek Harness returned an invalid turn result" };
    const failure = turnFailure(data);
    return failure
      ? { type: "execution.error", code: failure.code, message: failure.message }
      : { type: "execution.idle" };
  }
  if (event?.type === "todo/write") {
    const data = record(event.data);
    return { type: "todo.updated", todos: Array.isArray(data?.todos) ? data.todos : [] };
  }
  if (["user/message", "assistant/message", "tool/call", "tool/result"].includes(String(event?.type))) {
    return { type: "transcript.changed" };
  }
  return null;
}
