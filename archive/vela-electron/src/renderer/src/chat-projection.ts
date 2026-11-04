import type { AssistantPartView, TranscriptItem } from "@shared/types";
import { formatFailure } from "@shared/errors";
import { messagePresentation } from "@shared/message-presentation";
import { partFromProjection } from "./chat-stream";
import { Binary } from "./binary";
import {
  errorText, FINAL_TOOL_STATUSES, getToolStatus, insertMessage, insertPart, toolPartID,
  type ChatDirectoryState, type ChatMessageRecord, type ChatPartRecord
} from "./chat-records";

type AssistantItem = Extract<TranscriptItem, { kind: "assistant" }>;
interface CachedAssistant {
  parts: ChatPartRecord[] | undefined;
  item: AssistantItem;
}

const projectionCaches = new WeakMap<ChatDirectoryState, WeakMap<ChatMessageRecord, CachedAssistant>>();

export function reconcileAssistantProjection(
  previous: TranscriptItem[],
  projected: TranscriptItem[],
  messages: ChatMessageRecord[]
): TranscriptItem[] {
  const userMetadata = new Map(messages
    .filter((message) => message.role === "user")
    .map((message) => [message.id, messagePresentation(message)]));
  const pending = new Map(projected.map((item) => [item.id, item] as const));
  const result: TranscriptItem[] = [];
  for (const item of previous) {
    if (item.kind === "assistant") {
      const next = pending.get(item.id);
      if (next) {
        result.push(next);
        pending.delete(item.id);
      }
      continue;
    }
    result.push(item.kind === "user" ? { ...item, ...userMetadata.get(item.id) } : item);
  }
  for (const item of projected) {
    if (pending.has(item.id)) result.push(item);
  }
  let parentID: string | undefined;
  const users = new Map<string, Extract<TranscriptItem, { kind: "user" }>>();
  return result.map((item) => {
    if (item.kind === "user") {
      parentID = item.id;
      users.set(item.id, item);
    } else if (item.kind === "synthetic" && item.description?.trim()) {
      parentID = item.id;
    } else if (item.kind === "shell") {
      parentID = undefined;
    } else if (item.kind === "assistant") {
      const ownerID = item.parentID ?? parentID;
      const user = ownerID ? users.get(ownerID) : undefined;
      if (user) {
        if (item.agent !== undefined) user.agent = item.agent;
        if (item.model) user.model = item.model;
      }
      return ownerID && ownerID !== item.parentID ? { ...item, parentID: ownerID } : item;
    }
    return item;
  });
}

export function projectAssistantItems(draft: ChatDirectoryState, sessionID: string): TranscriptItem[] {
  let cache = projectionCaches.get(draft);
  if (!cache) {
    cache = new WeakMap();
    projectionCaches.set(draft, cache);
  }
  const out: TranscriptItem[] = [];
  for (const message of draft.message[sessionID] ?? []) {
    if (message.role !== "assistant") continue;
    const parts = draft.part[message.id];
    const previous = cache.get(message);
    // Part deltas replace only the parts array; message snapshots replace the message.
    if (previous && previous.parts === parts) {
      out.push(previous.item);
      continue;
    }
    const item = projectAssistantMessage(message, parts ?? []);
    cache.set(message, { parts, item });
    out.push(item);
  }
  return out;
}

function projectAssistantMessage(message: ChatMessageRecord, storedParts: ChatPartRecord[]): AssistantItem {
  // OpenCode's canonical parts are ID-ordered; V2 stream parts use synthetic IDs
  // and retain the order in which the server reported their steps.
  const orderedParts = storedParts.every((part) => part.id.startsWith("prt_"))
    ? storedParts
    : storedParts
        .map((part, index) => ({ part, index }))
        .sort((left, right) => {
          const leftSeq = typeof left.part.seq === "number" ? left.part.seq : Number.MAX_SAFE_INTEGER;
          const rightSeq = typeof right.part.seq === "number" ? right.part.seq : Number.MAX_SAFE_INTEGER;
          return leftSeq === rightSeq ? left.index - right.index : leftSeq - rightSeq;
        })
        .map(({ part }) => part);
  const projected = orderedParts.flatMap((part): AssistantPartView[] => {
    const view = partFromProjection(part as Record<string, any>, Number(message.time?.created ?? 0));
    return view ? [view] : [];
  });
  const parts: AssistantPartView[] = [];
  const toolIndexes = new Map<string, number>();
  for (const part of projected) {
    if (part.kind !== "tool") {
      parts.push(part);
      continue;
    }
    const existingIndex = toolIndexes.get(part.tool.id);
    if (existingIndex === undefined) {
      toolIndexes.set(part.tool.id, parts.length);
      parts.push(part);
      continue;
    }
    const existing = parts[existingIndex];
    if (existing.kind !== "tool") continue;
    parts[existingIndex] = {
      ...part,
      tool: {
        ...existing.tool,
        ...part.tool,
        title: part.tool.title.toLowerCase().replace(/[^a-z]/g, "") === "tool" ? existing.tool.title : part.tool.title,
        detail: part.tool.detail || existing.tool.detail,
        input: part.tool.input || existing.tool.input,
        inputValue: part.tool.inputValue ?? existing.tool.inputValue,
        output: part.tool.output ?? existing.tool.output,
        paths: part.tool.paths?.length ? part.tool.paths : existing.tool.paths,
        metadata: part.tool.metadata ?? existing.tool.metadata
      }
    };
  }
  const retry = message.retry;
  return {
    kind: "assistant",
    id: message.id,
    messageID: message.id,
    parts,
    completed: Boolean(message.time?.completed ?? message.finish ?? (message.error && !message.retry)),
    ...messagePresentation(message),
    ...(retry && typeof retry === "object"
      ? {
          retry: {
            attempt: Number((retry as Record<string, unknown>).attempt ?? 1),
            message: formatFailure((retry as Record<string, unknown>).message, "VELA_RETRY_SCHEDULED", "Retrying"),
            next: Number((retry as Record<string, unknown>).next ?? 0) || undefined
          }
        }
      : {}),
    ...(message.error ? { error: errorText(message.error) } : {})
  };
}

function recordFromView(messageID: string, sessionID: string, view: Extract<TranscriptItem, { kind: "assistant" }>["parts"][number]): ChatPartRecord {
  if (view.kind === "text" || view.kind === "reasoning") {
    return {
      id: view.id,
      messageID,
      sessionID,
      type: view.kind,
      text: view.text,
      time: view.complete ? { completed: 1 } : {}
    };
  }
  const tool = view.tool;
  const callID = tool.id;
  return {
    id: toolPartID(messageID, callID),
    messageID,
    sessionID,
    type: "tool",
    ...(tool.title !== "tool" ? { name: tool.title } : {}),
    callID,
    state: {
      status: tool.status === "running" ? "running" : tool.status === "success" ? "completed" : "error",
      input: tool.inputValue ?? tool.input ?? "",
      ...(tool.output !== undefined ? { output: tool.output } : {}),
      ...(tool.metadata ? { metadata: tool.metadata } : {}),
      ...(tool.progress !== undefined ? { progress: tool.progress } : {})
    },
    time: { created: tool.startedAt, ...(tool.duration !== undefined && tool.startedAt ? { completed: tool.startedAt + tool.duration } : {}) },
    ...(tool.executed !== undefined ? { executed: tool.executed } : {}),
    ...(tool.providerState ? { providerState: tool.providerState } : {}),
    ...(tool.providerResultState ? { providerResultState: tool.providerResultState } : {})
  };
}

export function hydrateChatState(draft: ChatDirectoryState, sessionID: string, transcript: TranscriptItem[]): void {
  for (const item of transcript) {
    if (item.kind !== "assistant" && item.kind !== "user") continue;
    const messages = draft.message[sessionID] ?? [];
    const messageResult = Binary.search(messages, item.id, (message) => message.id);
    const hydratedMessage: ChatMessageRecord = {
      id: item.id,
      sessionID,
      role: item.kind,
      ...messagePresentation(item),
      time: {
        ...(typeof item.createdAt === "number" ? { created: item.createdAt } : {}),
        ...(typeof item.completedAt === "number" ? { completed: item.completedAt } : {})
      },
      ...(item.kind === "assistant" && item.completed ? { finish: "stop" } : {}),
      ...(item.kind === "assistant" && item.error ? { error: { message: item.error } } : {}),
      ...(item.kind === "assistant" && item.retry ? { retry: item.retry } : {})
    };
    if (messageResult.found) {
      const previous = messages[messageResult.index];
      const next = [...messages];
      next[messageResult.index] = {
        ...previous,
        ...hydratedMessage,
        time: { ...previous.time, ...hydratedMessage.time },
        error: previous.error ?? hydratedMessage.error,
        retry: previous.retry ?? hydratedMessage.retry
      };
      draft.message[sessionID] = next;
    } else {
      insertMessage(draft, sessionID, hydratedMessage);
    }
    if (item.kind !== "assistant") continue;
    for (const view of item.parts) {
      const record = recordFromView(item.id, sessionID, view);
      const parts = draft.part[item.id] ?? [];
      const partResult = Binary.search(parts, record.id, (part) => part.id);
      if (!partResult.found) {
        insertPart(draft, record);
        continue;
      }
      const previous = parts[partResult.index];
      const next = [...parts];
      if (previous.type === "tool" && record.type === "tool") {
        const previousStatus = getToolStatus(previous);
        const previousFinished = Boolean(previousStatus && FINAL_TOOL_STATUSES.has(previousStatus));
        next[partResult.index] = previousFinished
          ? {
              ...previous,
              ...(record.name ? { name: record.name } : {}),
              state: { ...record.state, ...previous.state },
              time: { ...record.time, ...previous.time }
            }
          : { ...previous, ...record, state: { ...previous.state, ...record.state }, time: { ...previous.time, ...record.time } };
      } else if ((previous.type === "text" || previous.type === "reasoning") && record.type === previous.type) {
        next[partResult.index] = {
          ...record,
          text: typeof previous.text === "string" && previous.text.length > String(record.text ?? "").length
            ? previous.text
            : record.text,
          time: { ...record.time, ...(previous.time?.completed ? { completed: previous.time.completed } : {}) }
        };
      } else {
        next[partResult.index] = record;
      }
      draft.part[item.id] = next;
    }
  }
}
