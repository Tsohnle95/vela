import type { TranscriptItem } from "@shared/types";
import { formatFailure, normalizeFailure } from "@shared/errors";
import { messagePresentation } from "@shared/message-presentation";
import type { ChatStreamEvent } from "./chat-stream";
import { Binary } from "./binary";
import {
  applyDelta, areMessageUpdateFieldsEqual, areSessionStatusesEqual, completeLatestIncomplete,
  errorText, failLatestIncomplete, hasMessage, insertMessage, insertPart, setChatSessionStatus,
  toolPartID, upsertPart,
  type ChatDirectoryState, type ChatEventResult, type ChatMessageRecord,
  type ChatPartRecord, type ChatSessionStatus
} from "./chat-records";

export {
  attachRetryToLatestAssistant, completeLatestIncomplete, findTurnStartedAt,
  insertMessage, insertUserMessage, snapshotChatState,
  type ChatDirectoryState, type ChatEventResult, type ChatMaterializationReason,
  type ChatMessageRecord, type ChatPartRecord, type ChatSessionStatus, type ChatStateSnapshot,
  type ChatMessageReadView, type ChatPartReadView, type ChatSnapshotReadView, type ChatDirectoryReadView
} from "./chat-records";
export { hydrateChatState, projectAssistantItems, reconcileAssistantProjection } from "./chat-projection";

export const SKIP_PARTS = new Set(["patch", "step-start", "step-finish"]);
const TERMINAL_TURN_FINISHES = new Set(["stop", "length", "content-filter", "error", "unknown"]);

function eventMessageID(data: Record<string, any>): string {
  return String(data.assistantMessageID ?? data.messageID ?? "");
}

function eventToolID(data: Record<string, any>): string {
  return String(data.callID ?? data.id ?? "");
}

function textPartID(messageID: string, type: "text" | "reasoning", ordinal: unknown): string {
  return `${messageID}:${type}:${Number(ordinal ?? 0)}`;
}

function sameTypeCount(draft: ChatDirectoryState, messageID: string, type: string): number {
  return (draft.part[messageID] ?? []).filter((part) => part.type === type).length;
}

function resolvedOrdinal(
  draft: ChatDirectoryState,
  messageID: string,
  type: "text" | "reasoning",
  ordinal: unknown
): number {
  if (ordinal !== undefined) return Number(ordinal);
  return Math.max(0, sameTypeCount(draft, messageID, type) - 1);
}

function eventOrdinal(data: Record<string, any>): number {
  return Number(data.ordinal ?? 0);
}

export interface RateLimitAction {
  reason?: string;
  message?: string;
  link?: string;
}

export function buildRateLimitNotice(action: RateLimitAction | null | undefined, sessionID: string): TranscriptItem | null {
  if (!action) return null;
  const reason = action.reason;
  if (reason !== "free_tier_limit" && reason !== "account_rate_limit") return null;
  const base = String(
    action.message ??
      (reason === "free_tier_limit"
        ? "OpenCode Go free usage limit reached."
        : "OpenCode Go usage limit reached.")
  );
  const link = typeof action.link === "string" && action.link ? action.link : "";
  const text = link && !base.includes(link) ? `${base} → ${link}` : base;
  return { kind: "status", id: `${sessionID}-ratelimit-${reason}`, text, tone: "error" };
}

function retryRecord(value: unknown): ChatMessageRecord["retry"] | undefined {
  if (!value || typeof value !== "object") return undefined;
  const raw = value as Record<string, unknown>;
  const attempt = Number(raw.attempt ?? 1);
  const message = errorText((raw as { error?: unknown }).error, "VELA_RETRY_SCHEDULED", "Retrying");
  const next = Number(raw.at ?? 0) || undefined;
  return { attempt, message, ...(next !== undefined ? { next } : {}) };
}

function basePart(messageID: string, sessionID: string, partID: string, type: string, created: number): ChatPartRecord {
  return {
    id: partID,
    messageID,
    sessionID,
    type,
    time: { created }
  };
}

function textSnapshot(
  messageID: string,
  sessionID: string,
  partID: string,
  type: "text" | "reasoning",
  text: string,
  created: number,
  state?: Record<string, unknown>
): ChatPartRecord {
  return {
    ...basePart(messageID, sessionID, partID, type, created),
    text,
    ...(state ? { state } : {}),
    time: { created, completed: created }
  };
}

export function applyChatEvent(draft: ChatDirectoryState, routedSessionID: string, event: ChatStreamEvent): ChatEventResult {
  const data = event.data;
  const sessionID = typeof data.sessionID === "string" ? data.sessionID : routedSessionID;

  switch (event.type) {
    case "session.status": {
      const raw = data.status as { type?: unknown; attempt?: unknown; message?: unknown; next?: unknown } | undefined;
      const status: ChatSessionStatus | null = raw?.type === "busy"
        ? { type: "busy" }
        : raw?.type === "idle"
          ? { type: "idle" }
          : raw?.type === "retry"
            ? { type: "retry", attempt: Number(raw.attempt ?? 1), message: formatFailure(raw.message, "VELA_RETRY_SCHEDULED", "Retrying"), next: Number(raw.next ?? 0) }
            : raw?.type === "error"
              ? { type: "error" }
              : null;
      if (!status) return false;
      if (areSessionStatusesEqual(draft.session_status[sessionID], status)) return false;
      draft.session_status = { ...draft.session_status, [sessionID]: status };
      return true;
    }
    case "session.idle": {
      const completed = completeLatestIncomplete(draft, sessionID);
      const status: ChatSessionStatus = { type: "idle" };
      if (!completed && areSessionStatusesEqual(draft.session_status[sessionID], status)) return false;
      draft.session_status = { ...draft.session_status, [sessionID]: status };
      return true;
    }
    case "session.error": {
      const failed = failLatestIncomplete(draft, sessionID, data.error, "VELA_SESSION_ERROR", "Session failed");
      return setChatSessionStatus(draft, sessionID, { type: "error" }) || failed;
    }
    case "message.updated": {
      const info = (data.info ?? data) as Record<string, any>;
      const messageID = String(info.id ?? "");
      const infoSessionID = typeof info.sessionID === "string" ? info.sessionID : sessionID;
      if (!messageID || !infoSessionID) return false;
      const next: ChatMessageRecord = {
        ...info,
        id: messageID,
        sessionID: infoSessionID,
        role: String(info.role ?? info.type ?? "assistant"),
        time: { created: Number(info.time?.created ?? event.created), completed: Number(info.time?.completed ?? 0) || undefined },
        finish: info.finish,
        ...(info.error ? { error: info.error } : {}),
        ...(retryRecord(info.retry) ? { retry: retryRecord(info.retry) } : {})
      };
      const terminal = next.role === "assistant" && (
        TERMINAL_TURN_FINISHES.has(String(next.finish ?? "")) || Boolean(next.error)
      );
      const statusChanged = terminal
        ? setChatSessionStatus(draft, infoSessionID, next.error ? { type: "error" } : { type: "idle" })
        : false;
      const messages = draft.message[infoSessionID] ?? [];
      const result = Binary.search(messages, messageID, (message) => message.id);
      if (result.found) {
        const existing = messages[result.index];
        if (areMessageUpdateFieldsEqual(existing, next)) return statusChanged;
        const replaced = [...messages];
        replaced[result.index] = next;
        draft.message[infoSessionID] = replaced;
      } else {
        const inserted = [...messages];
        inserted.splice(result.index, 0, next);
        draft.message[infoSessionID] = inserted;
      }
      return true;
    }
    case "message.removed": {
      const messageID = String(data.messageID ?? "");
      if (!messageID) return false;
      const messages = draft.message[sessionID];
      if (messages) {
        const result = Binary.search(messages, messageID, (message) => message.id);
        if (result.found) {
          const next = [...messages];
          next.splice(result.index, 1);
          if (next.length === 0) delete draft.message[sessionID];
          else draft.message[sessionID] = next;
        }
      }
      if (messageID in draft.part) delete draft.part[messageID];
      return true;
    }
    case "message.part.updated": {
      const part = (data.part ?? data) as Record<string, any>;
      if (SKIP_PARTS.has(String(part.type))) return false;
      const messageID = String(part.messageID ?? data.messageID ?? "");
      const partSessionID = typeof part.sessionID === "string" ? part.sessionID : sessionID;
      if (!messageID) return false;
      const missingOwningMessage = !hasMessage(draft, partSessionID, messageID);
      let next: ChatPartRecord = {
        ...part,
        id: String(part.id ?? ""),
        messageID,
        sessionID: partSessionID,
        type: String(part.type ?? "")
      };
      if (!next.id) return false;
      if (next.type === "tool") {
        const parts = draft.part[messageID] ?? [];
        const callID = String(part.callID ?? next.id);
        const canonicalID = toolPartID(messageID, callID);
        const canonical = Binary.search(parts, canonicalID, (item) => item.id);
        if (canonical.found) {
          const previous = parts[canonical.index];
          next = {
            ...previous,
            ...next,
            id: canonicalID,
            callID,
            state: { ...previous.state, ...next.state }
          };
        } else if (!Binary.search(parts, next.id, (item) => item.id).found) {
          next.id = canonicalID;
        }
      }
      upsertPart(draft, next);
      return missingOwningMessage
        ? {
            changed: true,
            materialization: {
              type: "incomplete-session-snapshot",
              reason: "missing-owning-message",
              sessionID: partSessionID,
              messageID,
              partID: next.id
            }
          }
        : true;
    }
    case "message.part.removed": {
      const messageID = String(data.messageID ?? "");
      const partID = String(data.partID ?? "");
      if (!messageID || !partID) return false;
      const parts = draft.part[messageID];
      if (!parts) return false;
      const result = Binary.search(parts, partID, (part) => part.id);
      if (!result.found) return false;
      const next = [...parts];
      next.splice(result.index, 1);
      if (next.length === 0) delete draft.part[messageID];
      else draft.part[messageID] = next;
      return true;
    }
    case "message.part.delta": {
      const deltaMessageID = String(data.messageID ?? "");
      const deltaPartID = String(data.partID ?? "");
      const parts = draft.part[deltaMessageID];
      let resolvedPartID = deltaPartID;
      if (parts && !Binary.search(parts, deltaPartID, (item) => item.id).found) {
        const alias = toolPartID(deltaMessageID, deltaPartID);
        if (Binary.search(parts, alias, (item) => item.id).found) resolvedPartID = alias;
      }
      return applyDelta(draft, {
        sessionID,
        messageID: deltaMessageID,
        partID: resolvedPartID,
        field: String(data.field ?? "text"),
        delta: String(data.delta ?? "")
      });
    }
    case "session.step.started": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const messages = draft.message[sessionID] ?? [];
      const next = messages.map((message) => {
        if (message.role !== "assistant" || message.id === messageID) return message;
        if (message.time?.completed || message.finish) return message;
        return { ...message, retry: undefined, time: { ...message.time, completed: event.created } };
      });
      draft.message[sessionID] = next;
      const previous = messages.find((message) => message.id === messageID);
      insertMessage(draft, sessionID, {
        ...previous,
        id: messageID,
        sessionID,
        role: "assistant",
        ...(typeof data.agent === "string" ? { agent: data.agent } : {}),
        ...(data.model && typeof data.model === "object" ? { model: data.model } : {}),
        ...(typeof data.parentID === "string" ? { parentID: data.parentID } : {}),
        time: { created: previous?.time.created ?? event.created },
        finish: undefined,
        error: undefined,
        retry: undefined,
        interrupted: undefined
      });
      setChatSessionStatus(draft, sessionID, { type: "busy" });
      return true;
    }
    case "session.step.ended": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const previous = draft.message[sessionID]?.find((message) => message.id === messageID);
      insertMessage(draft, sessionID, {
        ...previous,
        id: messageID,
        sessionID,
        role: "assistant",
        time: { created: previous?.time.created ?? event.created, completed: event.created },
        finish: data.finish
      });
      if (TERMINAL_TURN_FINISHES.has(String(data.finish ?? ""))) {
        setChatSessionStatus(draft, sessionID, { type: "idle" });
      }
      return true;
    }
    case "session.step.failed": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const previous = draft.message[sessionID]?.find((message) => message.id === messageID);
      insertMessage(draft, sessionID, {
        ...previous,
        id: messageID,
        sessionID,
        role: "assistant",
        time: { created: previous?.time.created ?? event.created, completed: event.created },
        ...messagePresentation({ error: data.error }),
        finish: "error",
        error: normalizeFailure(data.error, "VELA_STEP_FAILED", "Step failed"),
        retry: undefined
      });
      setChatSessionStatus(draft, sessionID, { type: "error" });
      return true;
    }
    case "session.text.started": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const seq = data.ordinal === undefined ? sameTypeCount(draft, messageID, "text") : eventOrdinal(data);
      const partID = textPartID(messageID, "text", seq);
      const parts = draft.part[messageID] ?? [];
      const result = Binary.search(parts, partID, (part) => part.id);
      if (result.found) return false;
      insertPart(draft, basePart(messageID, sessionID, partID, "text", event.created));
      return true;
    }
    case "session.text.delta":
      return applyDelta(draft, {
        sessionID,
        messageID: eventMessageID(data),
        partID: textPartID(eventMessageID(data), "text", resolvedOrdinal(draft, eventMessageID(data), "text", data.ordinal)),
        field: "text",
        delta: String(data.delta ?? "")
      });
    case "session.text.ended": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const seq = data.ordinal === undefined ? Math.max(0, sameTypeCount(draft, messageID, "text") - 1) : eventOrdinal(data);
      const partID = textPartID(messageID, "text", seq);
      const parts = draft.part[messageID] ?? [];
      const existing = Binary.search(parts, partID, (part) => part.id);
      const state = data.state && typeof data.state === "object"
        ? data.state as Record<string, unknown>
        : undefined;
      if (!existing.found) {
        insertPart(draft, textSnapshot(messageID, sessionID, partID, "text", String(data.text ?? ""), event.created, state));
        return true;
      }
      const previous = parts[existing.index];
      const previousText = typeof previous.text === "string" ? previous.text : "";
      const nextText = String(data.text ?? "");
      return upsertPart(draft, {
        ...textSnapshot(
          messageID,
          sessionID,
          partID,
          "text",
          nextText.length >= previousText.length ? nextText : previousText,
          event.created,
          state ?? previous.state
        ),
        time: { created: previous.time?.created ?? event.created, completed: event.created }
      });
    }
    case "session.reasoning.started": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const seq = data.ordinal === undefined ? sameTypeCount(draft, messageID, "reasoning") : eventOrdinal(data);
      const partID = textPartID(messageID, "reasoning", seq);
      const parts = draft.part[messageID] ?? [];
      const result = Binary.search(parts, partID, (part) => part.id);
      if (result.found) return false;
      insertPart(draft, basePart(messageID, sessionID, partID, "reasoning", event.created));
      return true;
    }
    case "session.reasoning.delta":
      return applyDelta(draft, {
        sessionID,
        messageID: eventMessageID(data),
        partID: textPartID(eventMessageID(data), "reasoning", resolvedOrdinal(draft, eventMessageID(data), "reasoning", data.ordinal)),
        field: "text",
        delta: String(data.delta ?? "")
      });
    case "session.reasoning.ended": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const seq = data.ordinal === undefined ? Math.max(0, sameTypeCount(draft, messageID, "reasoning") - 1) : eventOrdinal(data);
      const partID = textPartID(messageID, "reasoning", seq);
      const parts = draft.part[messageID] ?? [];
      const existing = Binary.search(parts, partID, (part) => part.id);
      if (!existing.found) {
        insertPart(draft, textSnapshot(messageID, sessionID, partID, "reasoning", String(data.text ?? ""), event.created));
        return true;
      }
      const previous = parts[existing.index];
      const previousText = typeof previous.text === "string" ? previous.text : "";
      const nextText = String(data.text ?? "");
      return upsertPart(draft, {
        ...textSnapshot(messageID, sessionID, partID, "reasoning", nextText.length >= previousText.length ? nextText : previousText, event.created),
        time: { created: previous.time?.created ?? event.created, completed: event.created }
      });
    }
    case "session.tool.input.started": {
      const messageID = eventMessageID(data);
      const callID = eventToolID(data);
      if (!messageID || !callID) return false;
      const partID = toolPartID(messageID, callID);
      const parts = draft.part[messageID] ?? [];
      const result = Binary.search(parts, partID, (part) => part.id);
      if (result.found) return false;
      insertPart(draft, {
        ...basePart(messageID, sessionID, partID, "tool", event.created),
        ...(typeof data.name === "string" && data.name ? { name: data.name } : {}),
        callID,
        state: { status: "running", input: "" }
      });
      return true;
    }
    case "session.tool.input.delta": {
      const messageID = eventMessageID(data);
      const callID = eventToolID(data);
      if (!messageID || !callID) return false;
      return applyDelta(draft, {
        sessionID,
        messageID,
        partID: toolPartID(messageID, callID),
        field: "input",
        delta: String(data.delta ?? "")
      });
    }
    case "session.tool.input.ended": {
      const messageID = eventMessageID(data);
      const callID = eventToolID(data);
      if (!messageID || !callID) return false;
      const partID = toolPartID(messageID, callID);
      const parts = draft.part[messageID] ?? [];
      const existing = Binary.search(parts, partID, (part) => part.id);
      if (!existing.found) {
        insertPart(draft, {
          ...basePart(messageID, sessionID, partID, "tool", event.created),
          callID,
          state: { status: "running", input: String(data.text ?? "") }
        });
        return true;
      }
      const previous = parts[existing.index];
      const previousInput = typeof previous.state?.input === "string" ? previous.state.input : "";
      const nextInput = String(data.text ?? "");
      return upsertPart(draft, {
        ...previous,
        state: {
          ...previous.state,
          input: nextInput.length >= previousInput.length ? nextInput : previousInput
        }
      });
    }
    case "session.tool.called": {
      const messageID = eventMessageID(data);
      const callID = eventToolID(data);
      if (!messageID || !callID) return false;
      const partID = toolPartID(messageID, callID);
      const parts = draft.part[messageID] ?? [];
      const existing = Binary.search(parts, partID, (part) => part.id);
      const previous = existing.found ? parts[existing.index] : undefined;
      const part: ChatPartRecord = {
        ...basePart(messageID, sessionID, partID, "tool", event.created),
        ...(typeof data.name === "string" && data.name ? { name: data.name } : {}),
        callID,
        state: {
          ...(previous?.state ?? {}),
          status: "running",
          input: data.input,
          ...(data.metadata ? { metadata: data.metadata } : {})
        },
        time: { created: previous?.time?.created ?? event.created },
        ...(typeof data.executed === "boolean" ? { executed: data.executed } : {}),
        ...(data.state && typeof data.state === "object" ? { providerState: data.state } : {})
      };
      if (!existing.found) {
        insertPart(draft, part);
        return true;
      }
      return upsertPart(draft, part);
    }
    case "session.tool.progress": {
      const messageID = eventMessageID(data);
      const callID = eventToolID(data);
      if (!messageID || !callID) return false;
      const partID = toolPartID(messageID, callID);
      const parts = draft.part[messageID] ?? [];
      const existing = Binary.search(parts, partID, (part) => part.id);
      if (!existing.found) {
        insertPart(draft, {
          ...basePart(messageID, sessionID, partID, "tool", event.created),
          callID,
          state: { status: "running", progress: data.progress ?? data.metadata, ...(data.metadata ? { metadata: data.metadata } : {}) }
        });
        return true;
      }
      const previous = parts[existing.index];
      const next = [...parts];
      next[existing.index] = {
        ...previous,
        state: {
          ...previous.state,
          progress: data.progress ?? data.metadata,
          ...(data.metadata ? { metadata: data.metadata } : {})
        }
      };
      draft.part[messageID] = next;
      return true;
    }
    case "session.tool.success":
    case "session.tool.failed": {
      const messageID = eventMessageID(data);
      const callID = eventToolID(data);
      if (!messageID || !callID) return false;
      const partID = toolPartID(messageID, callID);
      const parts = draft.part[messageID] ?? [];
      const existing = Binary.search(parts, partID, (part) => part.id);
      const previous = existing.found ? parts[existing.index] : undefined;
      const failed = event.type === "session.tool.failed";
      const next: ChatPartRecord = {
        ...basePart(messageID, sessionID, partID, "tool", event.created),
        ...(typeof previous?.name === "string" ? { name: previous.name } : {}),
        callID,
        state: {
          ...(previous?.state ?? {}),
          status: failed ? "error" : "completed",
          time: { start: previous?.time?.created ?? event.created, end: event.created },
          ...(data.metadata ? { metadata: data.metadata } : {}),
          progress: undefined,
          ...(data.content !== undefined ? { content: data.content } : {}),
          ...(data.output !== undefined ? { output: data.output } : {}),
          ...(failed ? { error: data.error } : {})
        },
        time: { created: previous?.time?.created ?? event.created, completed: event.created },
        ...(typeof data.executed === "boolean" ? { executed: data.executed } : {}),
        ...(data.resultState && typeof data.resultState === "object" ? { providerResultState: data.resultState } : {})
      };
      if (!existing.found) {
        insertPart(draft, next);
        return true;
      }
      return upsertPart(draft, next);
    }
    case "session.retry.scheduled": {
      const messageID = eventMessageID(data);
      if (!messageID) return false;
      const retry = {
        attempt: Number(data.attempt ?? 1),
        message: errorText(data.error, "VELA_RETRY_SCHEDULED", "Retrying"),
        next: Number(data.at ?? data.next ?? 0) || undefined
      };
      setChatSessionStatus(draft, sessionID, {
        type: "retry",
        attempt: retry.attempt,
        message: retry.message,
        next: retry.next ?? 0
      });
      const messages = draft.message[sessionID] ?? [];
      const result = Binary.search(messages, messageID, (message) => message.id);
      if (result.found) {
        const next = [...messages];
        next[result.index] = { ...messages[result.index], retry };
        draft.message[sessionID] = next;
      } else {
        insertMessage(draft, sessionID, { id: messageID, sessionID, role: "assistant", time: { created: event.created }, retry });
      }
      return true;
    }
    case "session.execution.started":
      return setChatSessionStatus(draft, sessionID, { type: "busy" });
    case "session.execution.succeeded": {
      const completed = completeLatestIncomplete(draft, sessionID);
      return setChatSessionStatus(draft, sessionID, { type: "idle" }) || completed;
    }
    case "session.execution.failed":
    case "session.execution.interrupted": {
      const failed = event.type === "session.execution.failed"
        ? failLatestIncomplete(draft, sessionID, data.error, "VELA_EXECUTION_FAILED", "Execution failed")
        : completeLatestIncomplete(draft, sessionID);
      return setChatSessionStatus(draft, sessionID, { type: "error" }) || failed;
    }
    default:
      return false;
  }
}
