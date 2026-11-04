import type { MessagePresentation } from "@shared/types";
import { formatFailure, normalizeFailure } from "@shared/errors";
import { Binary } from "./binary";

const DELTA_OVERLAP_FIELDS = ["text", "output", "input"] as const;
export const FINAL_TOOL_STATUSES = new Set(["completed", "error", "aborted", "failed", "timeout", "cancelled"]);

export interface ChatMessageRecord {
  id: string;
  sessionID: string;
  role: string;
  time: { created?: number; completed?: number; end?: number };
  finish?: unknown;
  error?: unknown;
  retry?: { attempt: number; message: string; next?: number };
  [key: string]: unknown;
}

export function findTurnStartedAt(
  messages: readonly ChatMessageReadView[] | undefined,
  fallbackMessageID?: string
): number | null {
  if (!messages) return null;
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const record = messages[index];
    if (record.role === "user") {
      return typeof record.time.created === "number" ? record.time.created : null;
    }
  }
  const fallback = fallbackMessageID ? messages.find((record) => record.id === fallbackMessageID) : undefined;
  return typeof fallback?.time.created === "number" ? fallback.time.created : null;
}

export interface ChatPartRecord {
  id: string;
  messageID: string;
  sessionID?: string;
  type: string;
  seq?: number;
  text?: string;
  state?: Record<string, unknown>;
  time?: { created?: number; completed?: number; end?: number };
  [key: string]: unknown;
}

export type ChatSessionStatus =
  | { type: "busy" }
  | { type: "idle" }
  | { type: "error" }
  | { type: "retry"; attempt: number; message: string; next: number };

export interface ChatDirectoryState {
  message: Record<string, ChatMessageRecord[]>;
  part: Record<string, ChatPartRecord[]>;
  session_status: Record<string, ChatSessionStatus>;
}

export type ChatMaterializationReason = "missing-owning-message" | "orphan-delta" | "missing-delta-part";

export type ChatEventResult =
  | boolean
  | {
      changed: boolean;
      materialization: {
        type: "incomplete-session-snapshot";
        reason: ChatMaterializationReason;
        sessionID?: string;
        messageID: string;
        partID?: string;
      };
    };

export interface ChatStateSnapshot {
  message: Record<string, ChatMessageRecord[]>;
  session_status: Record<string, ChatSessionStatus>;
}

export interface ChatMessageReadView extends Readonly<ChatMessageRecord> {
  readonly time: Readonly<ChatMessageRecord["time"]>;
  readonly retry?: Readonly<NonNullable<ChatMessageRecord["retry"]>>;
}

export interface ChatPartReadView extends Readonly<ChatPartRecord> {
  readonly time?: Readonly<NonNullable<ChatPartRecord["time"]>>;
  readonly state?: Readonly<Record<string, unknown>>;
}

export interface ChatSnapshotReadView {
  readonly message: Readonly<Record<string, readonly ChatMessageReadView[]>>;
  readonly session_status: Readonly<Record<string, Readonly<ChatSessionStatus>>>;
}

export interface ChatDirectoryReadView extends ChatSnapshotReadView {
  readonly part: Readonly<Record<string, readonly ChatPartReadView[]>>;
}

export function toolPartID(messageID: string, callID: string): string {
  return `${messageID}:tool:${callID}`;
}

export function errorText(error: unknown, fallbackCode = "VELA_RUNTIME_FAILURE", fallbackMessage = "Operation failed"): string {
  return formatFailure(error, fallbackCode, fallbackMessage);
}

function partFieldValue(part: ChatPartRecord, field: string): string | undefined {
  if (field === "text") return typeof part.text === "string" ? part.text : undefined;
  const value = part.state?.[field];
  return typeof value === "string" ? value : undefined;
}

function setPartFieldValue(part: ChatPartRecord, field: string, value: string): ChatPartRecord {
  if (field === "text") return { ...part, text: value };
  return { ...part, state: { ...part.state, [field]: value } };
}

function getUpdatedDeltaFields(previous: ChatPartRecord, next: ChatPartRecord): string[] {
  const dedupeFields: string[] = [];
  for (const field of DELTA_OVERLAP_FIELDS) {
    const previousValue = partFieldValue(previous, field);
    const nextValue = partFieldValue(next, field);
    if (!previousValue || !nextValue) continue;
    if (nextValue === previousValue || nextValue.startsWith(previousValue) || previousValue.startsWith(nextValue)) {
      dedupeFields.push(field);
    }
  }
  return dedupeFields;
}

function getPartEndTime(part: ChatPartRecord): number | undefined {
  const stateEnd = part.state?.time as { end?: unknown } | undefined;
  if (typeof stateEnd?.end === "number") return stateEnd.end;
  const timeCompleted = part.time?.completed;
  return typeof timeCompleted === "number" ? timeCompleted : undefined;
}

export function getToolStatus(part: ChatPartRecord): string | undefined {
  if (part.type !== "tool") return undefined;
  const status = part.state?.status;
  return typeof status === "string" ? status : undefined;
}

function shouldPreserveExistingPart(previous: ChatPartRecord, next: ChatPartRecord): boolean {
  if (previous.type !== "tool" || next.type !== "tool") return false;

  const previousStatus = getToolStatus(previous);
  const nextStatus = getToolStatus(next);
  if (previousStatus && FINAL_TOOL_STATUSES.has(previousStatus) && (!nextStatus || !FINAL_TOOL_STATUSES.has(nextStatus))) {
    return true;
  }
  if (previousStatus && FINAL_TOOL_STATUSES.has(previousStatus) && nextStatus && FINAL_TOOL_STATUSES.has(nextStatus) && previousStatus !== nextStatus) {
    return true;
  }

  const previousEnd = getPartEndTime(previous);
  const nextEnd = getPartEndTime(next);
  if (typeof previousEnd === "number" && typeof nextEnd !== "number") return true;

  return false;
}

export function areSessionStatusesEqual(left: ChatSessionStatus | undefined, right: ChatSessionStatus): boolean {
  if (left === right) return true;
  if (!left || left.type !== right.type) return false;
  if (left.type === "retry" && right.type === "retry") {
    return left.attempt === right.attempt && left.message === right.message && left.next === right.next;
  }
  return true;
}

export function setChatSessionStatus(draft: ChatDirectoryState, sessionID: string, status: ChatSessionStatus): boolean {
  if (areSessionStatusesEqual(draft.session_status[sessionID], status)) return false;
  draft.session_status = { ...draft.session_status, [sessionID]: status };
  return true;
}

export function failLatestIncomplete(
  draft: ChatDirectoryState,
  sessionID: string,
  error: unknown,
  fallbackCode: string,
  fallbackMessage: string
): boolean {
  const messages = draft.message[sessionID];
  if (!messages) return false;
  const failure = normalizeFailure(error, fallbackCode, fallbackMessage);
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role !== "assistant") continue;
    const incomplete = !message.time?.completed && !message.finish;
    if (!incomplete && !message.retry) continue;
    const next = [...messages];
    next[index] = {
      ...message,
      ...(incomplete ? { time: { ...message.time, completed: Date.now() }, error: failure } : {}),
      retry: undefined
    };
    draft.message[sessionID] = next;
    return true;
  }
  return false;
}

function areJsonEquivalent(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (left === undefined || right === undefined) return left === right;
  try {
    return JSON.stringify(left) === JSON.stringify(right);
  } catch {
    return false;
  }
}

export function areMessageUpdateFieldsEqual(existing: ChatMessageRecord, next: ChatMessageRecord): boolean {
  if (existing.role !== next.role) return false;
  if (existing.finish !== next.finish) return false;
  if (existing.time?.created !== next.time?.created) return false;
  if (existing.time?.completed !== next.time?.completed) return false;

  const fields = ["summary", "error", "cost", "tokens", "structured", "model", "modelID", "providerID", "parentID", "tools", "format", "variant", "agent", "system"];
  for (const field of fields) {
    if (!areJsonEquivalent(existing[field], next[field])) return false;
  }

  return true;
}

export function hasMessage(draft: ChatDirectoryState, sessionID: string, messageID: string): boolean {
  const messages = draft.message[sessionID];
  if (!messages) return false;
  return Binary.search(messages, messageID, (message) => message.id).found;
}

export function insertMessage(draft: ChatDirectoryState, sessionID: string, message: ChatMessageRecord): void {
  const messages = draft.message[sessionID] ?? [];
  const result = Binary.search(messages, message.id, (item) => item.id);
  const next = [...messages];
  if (result.found) {
    next[result.index] = message;
  } else {
    next.splice(result.index, 0, message);
  }
  draft.message[sessionID] = next;
}

export function insertUserMessage(
  draft: ChatDirectoryState,
  sessionID: string,
  id: string,
  text: string,
  model?: Record<string, unknown>,
  presentation?: MessagePresentation
): void {
  insertMessage(draft, sessionID, {
    id,
    sessionID,
    role: "user",
    time: { created: presentation?.createdAt ?? Date.now() },
    ...(model && typeof model === "object" ? { model } : {}),
    ...(presentation?.agent !== undefined ? { agent: presentation.agent } : {})
  });
}

function maxPartSeq(parts: ChatPartRecord[]): number {
  return parts.reduce((max, part) => Math.max(max, typeof part.seq === "number" ? part.seq : 0), 0);
}

export function insertPart(draft: ChatDirectoryState, part: ChatPartRecord): void {
  const parts = draft.part[part.messageID] ?? [];
  const result = Binary.search(parts, part.id, (item) => item.id);
  const next = [...parts];
  if (result.found) {
    next[result.index] = part;
  } else {
    const staged = typeof part.seq === "number" ? part : { ...part, seq: maxPartSeq(parts) + 1 };
    next.splice(result.index, 0, staged);
  }
  draft.part[part.messageID] = next;
}

export function upsertPart(draft: ChatDirectoryState, next: ChatPartRecord): boolean {
  const parts = draft.part[next.messageID];
  if (!parts) {
    draft.part[next.messageID] = [next];
    return true;
  }
  const result = Binary.search(parts, next.id, (part) => part.id);
  if (result.found) {
    const previous = parts[result.index];
    if (shouldPreserveExistingPart(previous, next)) return false;
    const dedupeFields = getUpdatedDeltaFields(previous, next);
    const carried = { ...next, ...(typeof previous.seq === "number" ? { seq: previous.seq } : {}) };
    const replaced = dedupeFields.length > 0
      ? { ...carried, __dedupeNextDeltaFields: dedupeFields }
      : carried;
    const replacedParts = [...parts];
    replacedParts[result.index] = replaced;
    draft.part[next.messageID] = replacedParts;
    return true;
  }
  const inserted = [...parts];
  inserted.splice(result.index, 0, { ...next, seq: typeof next.seq === "number" ? next.seq : maxPartSeq(parts) + 1 });
  draft.part[next.messageID] = inserted;
  return true;
}

export function applyDelta(
  draft: ChatDirectoryState,
  props: { sessionID?: string; messageID: string; partID: string; field: string; delta: string }
): ChatEventResult {
  const parts = draft.part[props.messageID];
  if (!parts) {
    return {
      changed: false,
      materialization: {
        type: "incomplete-session-snapshot",
        reason: "orphan-delta",
        sessionID: props.sessionID,
        messageID: props.messageID,
        partID: props.partID
      }
    };
  }
  const result = Binary.search(parts, props.partID, (part) => part.id);
  if (!result.found) {
    return {
      changed: false,
      materialization: {
        type: "incomplete-session-snapshot",
        reason: "missing-delta-part",
        sessionID: props.sessionID,
        messageID: props.messageID,
        partID: props.partID
      }
    };
  }
  const existing = parts[result.index];
  const dedupeFields = (existing as unknown as { __dedupeNextDeltaFields?: string[] }).__dedupeNextDeltaFields ?? [];
  const shouldDedupe = dedupeFields.includes(props.field);
  const existingValue = partFieldValue(existing, props.field);
  const merged = shouldDedupe && existingValue && existingValue.endsWith(props.delta)
    ? existingValue
    : (existingValue ?? "") + props.delta;
  const updated = setPartFieldValue(existing, props.field, merged) as unknown as { __dedupeNextDeltaFields?: string[] } & ChatPartRecord;
  updated.__dedupeNextDeltaFields = dedupeFields.filter((field) => field !== props.field);
  const next = [...parts];
  next[result.index] = updated as ChatPartRecord;
  draft.part[props.messageID] = next;
  return true;
}

export function completeLatestIncomplete(draft: ChatDirectoryState, sessionID: string): boolean {
  const messages = draft.message[sessionID];
  if (!messages) return false;
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role !== "assistant") continue;
    const incomplete = !message.time?.completed && !message.finish;
    if (!incomplete && !message.retry) continue;
    const next = [...messages];
    next[index] = {
      ...message,
      ...(incomplete ? { time: { ...message.time, completed: Date.now() } } : {}),
      retry: undefined
    };
    draft.message[sessionID] = next;
    return true;
  }
  return false;
}

export function attachRetryToLatestAssistant(
  draft: ChatDirectoryState,
  sessionID: string,
  retry: { attempt: number; message: string; next?: number }
): boolean {
  const messages = draft.message[sessionID];
  if (!messages) return false;
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role !== "assistant") continue;
    const next = [...messages];
    next[index] = { ...message, retry };
    draft.message[sessionID] = next;
    return true;
  }
  return false;
}

export function snapshotChatState(draft: ChatDirectoryState): ChatStateSnapshot {
  return {
    message: { ...draft.message },
    session_status: { ...draft.session_status }
  };
}

