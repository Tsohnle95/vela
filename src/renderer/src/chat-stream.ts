import type { AssistantPartView, ToolCallView, TranscriptItem } from "@shared/types";
import { messagePresentation } from "@shared/message-presentation";
import { formatFailure } from "@shared/errors";
import { retainToolContent } from "@shared/retention";
import { collectFilePaths as paths, record as metadata, toolContentViews, toolDetailText as toolDetail } from "@shared/tool-presentation";

export interface ChatStreamEvent {
  id: string;
  type: string;
  created: number;
  data: Record<string, any>;
  ownerSessionIDs?: string[];
}

function errorText(error: unknown, fallbackCode = "VELA_RUNTIME_FAILURE", fallbackMessage = "Operation failed"): string {
  return formatFailure(error, fallbackCode, fallbackMessage);
}

function stringify(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function toolOutput(data: Record<string, any>): string {
  const content = data.content ?? data.output;
  if (Array.isArray(content)) {
    return content
      .filter((item) => item?.type === "text")
      .map((item) => String(item.text ?? ""))
      .join("\n");
  }
  return stringify(content);
}

function inferTool(input: unknown): string {
  if (!input || typeof input !== "object") return "tool";
  const value = input as Record<string, unknown>;
  if (typeof value.tool === "string") return value.tool;
  if (typeof value.name === "string") return value.name;
  if ("command" in value) return "bash";
  if ("filePath" in value || "file_path" in value) return "file";
  if ("query" in value) return "search";
  if ("url" in value) return "web";
  if ("agent" in value && ("prompt" in value || "description" in value)) return "subagent";
  if ("prompt" in value) return "prompt";
  return "tool";
}

export function partFromProjection(part: Record<string, any>, created: number): AssistantPartView | null {
  const id = String(part.id ?? "");
  if (!id) return null;
  if (part.type === "text" || part.type === "reasoning") {
    const commentary = part.type === "text" && part.state?.phase === "commentary";
    return {
      kind: commentary ? "reasoning" : part.type,
      id,
      text: String(part.text ?? ""),
      complete: Boolean(part.time?.end ?? part.time?.completed)
    };
  }
  if (part.type !== "tool") return null;
  const state = (part.state ?? {}) as Record<string, any>;
  const input = state.input;
  const status = ["completed", "success"].includes(String(state.status))
    ? "success"
    : ["error", "failed", "aborted", "timeout", "cancelled"].includes(String(state.status))
      ? "failed"
      : "running";
  const startedAt = Number(part.time?.created ?? state.time?.start ?? created);
  const completedAt = Number(part.time?.completed ?? state.time?.end ?? 0);
  const output = Array.isArray(state.content)
    ? toolOutput({ content: state.content })
    : status === "success"
      ? stringify(state.output)
      : status === "failed"
        ? errorText(state.error, "VELA_TOOL_FAILED", "Tool failed")
        : stringify(state.output);
  const callID = String(part.callID ?? part.id);
  return {
    kind: "tool",
    id,
    tool: {
      id: callID,
      title: String(part.name ?? part.tool ?? inferTool(input)),
      detail: toolDetail(input),
      status,
      input: stringify(input),
      output,
      startedAt,
      ...(completedAt ? { duration: Math.max(0, completedAt - startedAt) } : {}),
      ...(state.progress !== undefined ? { progress: stringify(state.progress) } : {}),
      paths: paths(input),
      metadata: metadata(state.metadata),
      inputValue: input,
      content: retainToolContent(toolContentViews(state.content)),
      ...(typeof part.executed === "boolean" ? { executed: part.executed } : {}),
      providerState: metadata(part.providerState),
      providerResultState: metadata(part.providerResultState)
    }
  };
}

function promoteInput(items: TranscriptItem[], inputID: string): TranscriptItem[] {
  const pendingIndex = items.findIndex((item) => item.kind === "pending-input" && item.id === inputID);
  if (pendingIndex === -1) return items;
  const pending = items[pendingIndex];
  if (pending.kind !== "pending-input") return items;
  if (pending.inputType === "synthetic") {
    const promoted: Extract<TranscriptItem, { kind: "synthetic" }> = {
      kind: "synthetic",
      id: pending.id,
      text: pending.text,
      ...(pending.description ? { description: pending.description } : {})
    };
    return items.map((item, index) => index === pendingIndex ? promoted : item);
  }
  const promoted: Extract<TranscriptItem, { kind: "user" }> = {
    kind: "user",
    id: pending.id,
    text: pending.text,
    ...messagePresentation(pending),
    ...(pending.attachments?.length ? { attachments: pending.attachments } : {})
  };
  const optimisticIndex = items.findIndex((item) =>
    item.kind === "user" && item.id.startsWith("user-") && item.text === promoted.text
  );
  if (optimisticIndex === -1) {
    return items.map((item, index) => index === pendingIndex ? promoted : item);
  }
  return items.flatMap((item, index) => {
    if (index === optimisticIndex) return [{ ...messagePresentation(item), ...promoted }];
    if (index === pendingIndex) return [];
    return [item];
  });
}

function upsertItem(items: TranscriptItem[], item: TranscriptItem): TranscriptItem[] {
  const index = items.findIndex((current) => current.id === item.id);
  if (index === -1) return [...items, item];
  return items.map((current, itemIndex) => itemIndex === index ? item : current);
}

function updateLatestCompaction(
  items: TranscriptItem[],
  update: (item: Extract<TranscriptItem, { kind: "compaction" }>) => Extract<TranscriptItem, { kind: "compaction" }>
): TranscriptItem[] {
  let index = -1;
  for (let itemIndex = items.length - 1; itemIndex >= 0; itemIndex -= 1) {
    if (items[itemIndex].kind === "compaction") {
      index = itemIndex;
      break;
    }
  }
  if (index === -1) return items;
  return items.map((item, itemIndex) =>
    itemIndex === index && item.kind === "compaction" ? update(item) : item
  );
}

export function reduceChatStream(items: TranscriptItem[], event: ChatStreamEvent): TranscriptItem[] {
  const data = event.data;
  switch (event.type) {
    case "session.inbox.enqueued": {
      const item = data.item as Record<string, any> | undefined;
      const payload = item?.payload as Record<string, any> | undefined;
      const inputID = String(data.inboxID ?? event.id);
      if (!item || !payload) return items;
      if (item.type === "user") {
        const files = Array.isArray(payload.files) ? payload.files as Record<string, unknown>[] : [];
        const attachments = files.flatMap((file): { name: string }[] =>
          typeof file.name === "string" && file.name ? [{ name: file.name }] : []
        );
        return upsertItem(items, {
          kind: "pending-input",
          id: inputID,
          inputType: "user",
          text: String(payload.text ?? ""),
          createdAt: event.created,
          ...messagePresentation(payload),
          ...(attachments.length > 0 ? { attachments } : {})
        });
      }
      if (item.type === "synthetic") {
        return upsertItem(items, {
          kind: "pending-input",
          id: inputID,
          inputType: "synthetic",
          text: String(payload.text ?? ""),
          ...(typeof payload.description === "string" ? { description: payload.description } : {})
        });
      }
      return items;
    }
    case "session.inbox.cancelled":
      return items.filter((item) =>
        item.kind !== "pending-input" || item.id !== String(data.inboxID ?? "")
      );
    case "session.inbox.delivered":
      return promoteInput(items, String(data.inboxID ?? ""));
    case "session.agent.selected":
      return upsertItem(items, {
        kind: "selection",
        id: event.id,
        selection: "agent",
        title: "Agent switched",
        detail: String(data.agent ?? "")
      });
    case "session.model.selected": {
      const model = data.model as Record<string, unknown> | undefined;
      const detail = [model?.providerID, model?.id].filter((part) => typeof part === "string" && part).join(" / ");
      return upsertItem(items, {
        kind: "selection",
        id: event.id,
        selection: "model",
        title: "Model switched",
        detail
      });
    }
    case "session.synthetic":
      return upsertItem(items, {
        kind: "synthetic",
        id: event.id,
        text: String(data.text ?? ""),
        ...(typeof data.description === "string" ? { description: data.description } : {})
      });
    case "session.skill.activated":
      return upsertItem(items, {
        kind: "skill",
        id: event.id,
        skill: String(data.id ?? ""),
        name: String(data.name ?? data.id ?? "Skill"),
        text: String(data.text ?? "")
      });
    case "session.shell.started": {
      const shell = data.shell as Record<string, any> | undefined;
      if (!shell) return items;
      return upsertItem(items, {
        kind: "shell",
        id: event.id,
        shellID: String(shell.id ?? event.id),
        command: String(shell.command ?? ""),
        status: "running"
      });
    }
    case "session.shell.ended": {
      const shell = data.shell as Record<string, any> | undefined;
      if (!shell) return items;
      const shellID = String(shell.id ?? "");
      const output = data.output as Record<string, unknown> | undefined;
      const index = items.findIndex((item) => item.kind === "shell" && item.shellID === shellID);
      const item: Extract<TranscriptItem, { kind: "shell" }> = {
        kind: "shell",
        id: index >= 0 ? items[index].id : event.id,
        shellID: shellID || event.id,
        command: String(shell.command ?? ""),
        status: ["exited", "timeout", "killed"].includes(String(shell.status))
          ? shell.status as "exited" | "timeout" | "killed"
          : "exited",
        ...(typeof output?.output === "string" ? { output: output.output } : {}),
        ...(typeof shell.exit === "number" ? { exit: shell.exit } : {})
      };
      return index >= 0
        ? items.map((current, itemIndex) => itemIndex === index ? item : current)
        : [...items, item];
    }
    case "session.compaction.started":
      return upsertItem(items, {
        kind: "compaction",
        id: event.id,
        status: "running",
        reason: data.reason === "manual" ? "manual" : "auto",
        summary: "",
        recent: String(data.recent ?? "")
      });
    case "session.compaction.delta":
      return updateLatestCompaction(items, (item) => ({
        ...item,
        summary: item.summary + String(data.text ?? "")
      }));
    case "session.compaction.ended": {
      const updated = updateLatestCompaction(items, (item) => ({
        ...item,
        status: "completed",
        reason: data.reason === "manual" ? "manual" : "auto",
        summary: String(data.text ?? item.summary),
        recent: String(data.recent ?? item.recent ?? "")
      }));
      return updated === items
        ? [...items, {
            kind: "compaction",
            id: event.id,
            status: "completed",
            reason: data.reason === "manual" ? "manual" : "auto",
            summary: String(data.text ?? ""),
            recent: String(data.recent ?? "")
          }]
        : updated;
    }
    case "session.compaction.failed": {
      const updated = updateLatestCompaction(items, (item) => ({
        ...item,
        status: "failed",
        reason: data.reason === "manual" ? "manual" : "auto",
        error: errorText(data.error, "VELA_COMPACTION_FAILED", "Compaction failed")
      }));
      return updated === items
        ? [...items, {
            kind: "compaction",
            id: event.id,
            status: "failed",
            reason: data.reason === "manual" ? "manual" : "auto",
            summary: "",
            error: errorText(data.error, "VELA_COMPACTION_FAILED", "Compaction failed")
          }]
        : updated;
    }
    default:
      return items;
  }
}

function mergeAssistantPart(history: AssistantPartView, live: AssistantPartView): AssistantPartView {
  if (history.kind !== live.kind) return live;
  if (history.kind === "text" && live.kind === "text") {
    return {
      ...live,
      text: live.text.length >= history.text.length ? live.text : history.text,
      complete: history.complete || live.complete
    };
  }
  if (history.kind === "reasoning" && live.kind === "reasoning") {
    return {
      ...live,
      text: live.text.length >= history.text.length ? live.text : history.text,
      complete: history.complete || live.complete
    };
  }
  if (history.kind !== "tool" || live.kind !== "tool") return live;
  const historyTerminal = history.tool.status !== "running";
  const liveTerminal = live.tool.status !== "running";
  if (historyTerminal && !liveTerminal) return history;
  return { ...history, ...live, tool: { ...history.tool, ...live.tool } };
}

function mergeAssistant(
  history: Extract<TranscriptItem, { kind: "assistant" }>,
  live: Extract<TranscriptItem, { kind: "assistant" }>
): Extract<TranscriptItem, { kind: "assistant" }> {
  const parts = [...history.parts];
  for (const livePart of live.parts) {
    const index = parts.findIndex((part) => part.id === livePart.id);
    if (index === -1) parts.push(livePart);
    else parts[index] = mergeAssistantPart(parts[index], livePart);
  }
  return {
    ...history,
    ...live,
    parts,
    completed: history.completed || live.completed,
    retry: live.retry ?? history.retry,
    error: live.error ?? history.error
  };
}

export function mergeChatHistory(history: TranscriptItem[], live: TranscriptItem[]): TranscriptItem[] {
  const matches = (left: TranscriptItem, right: TranscriptItem): boolean =>
    left.id === right.id || (
      left.kind === "assistant" && right.kind === "assistant" && left.messageID === right.messageID
    );
  const result = live.map((item) => {
    const current = history.find((candidate) => matches(candidate, item));
    if (current?.kind === "user" && item.kind === "user") return { ...item, ...current };
    return current?.kind === "assistant" && item.kind === "assistant"
      ? mergeAssistant(current, item)
      : current ?? item;
  });
  for (let historyIndex = 0; historyIndex < history.length; historyIndex += 1) {
    const item = history[historyIndex];
    if (result.some((current) => matches(current, item))) continue;
    const next = history.slice(historyIndex + 1).find((candidate) => result.some((current) => matches(current, candidate)));
    if (next) result.splice(result.findIndex((current) => matches(current, next)), 0, item);
    else result.push(item);
  }
  return result;
}

export function reconcilePromptHistory(
  history: TranscriptItem[],
  live: TranscriptItem[],
  optimisticUser: Extract<TranscriptItem, { kind: "user" }>
): TranscriptItem[] {
  const withOptimistic = live.some((item) => item.id === optimisticUser.id) ? live : [...live, optimisticUser];
  const remoteUsers = new Map<string, number>();
  for (const item of history) {
    if (item.kind === "user") remoteUsers.set(item.text, (remoteUsers.get(item.text) ?? 0) + 1);
  }
  const local = withOptimistic.filter((item) => {
    if (item.kind !== "user" || !item.id.startsWith("user-")) return true;
    const count = remoteUsers.get(item.text) ?? 0;
    if (count === 0) return true;
    remoteUsers.set(item.text, count - 1);
    return false;
  });
  return mergeChatHistory(history, local);
}

export function hasCompletedPromptResponse(
  history: TranscriptItem[],
  promptText: string,
  existingRemoteUsers: number
): boolean {
  const users = history.filter((item) => item.kind === "user" && item.text === promptText);
  const lastUser = users.at(-1);
  const lastUserIndex = lastUser ? history.lastIndexOf(lastUser) : -1;
  return users.length > existingRemoteUsers
    && lastUserIndex >= 0
    && history.slice(lastUserIndex + 1).some((item) => item.kind === "assistant" && item.completed);
}
