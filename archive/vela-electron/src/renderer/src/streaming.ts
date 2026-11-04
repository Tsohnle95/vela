import type { ChatDirectoryReadView, ChatMessageReadView, ChatSnapshotReadView } from "./chat-store";

export type StreamPhase = "streaming" | "cooldown" | "completed";

export interface MessageStreamState {
  phase: StreamPhase;
  startedAt: number;
  lastUpdateAt: number;
  completedAt?: number;
}

export interface StreamingStore {
  streamingMessageIds: Map<string, string | null>;
  messageStreamStates: Map<string, MessageStreamState>;
}

export interface StreamingReadView {
  readonly streamingMessageIds: ReadonlyMap<string, string | null>;
  readonly messageStreamStates: ReadonlyMap<string, Readonly<MessageStreamState>>;
}

export const STREAMING_HEARTBEAT_MS = 1000;

export function emptyStreamingStore(): StreamingStore {
  return {
    streamingMessageIds: new Map(),
    messageStreamStates: new Map()
  };
}

export function findTrailingAssistantMessage(messages: readonly ChatMessageReadView[] | undefined): ChatMessageReadView | null {
  if (!messages) return null;

  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (messages[index].role === "user") return null;
    if (messages[index].role === "assistant") return messages[index];
  }

  return null;
}

export function updateStreamingState(state: ChatDirectoryReadView, current: StreamingStore, now = Date.now()): StreamingStore | null {
  const nextStreamingIds = new Map<string, string | null>();
  const nextStreamStates = new Map(current.messageStreamStates);
  let changed = false;

  const busySessionIds = new Set<string>();
  for (const [sessionID, status] of Object.entries(state.session_status ?? {})) {
    if (status.type === "busy" || status.type === "retry") {
      busySessionIds.add(sessionID);
    }
  }

  const completeStreamingMessage = (sessionID: string, msgId: string) => {
    nextStreamingIds.set(sessionID, null);
    const existing = nextStreamStates.get(msgId);
    if (existing && existing.phase === "streaming") {
      nextStreamStates.set(msgId, {
        ...existing,
        phase: "completed",
        completedAt: now
      });
    }
    changed = true;
  };

  for (const sessionID of busySessionIds) {
    const messages = state.message[sessionID];
    if (!messages || messages.length === 0) continue;

    const streamingMsg = findTrailingAssistantMessage(messages);

    if (!streamingMsg) {
      const prevId = current.streamingMessageIds.get(sessionID);
      if (prevId) {
        completeStreamingMessage(sessionID, prevId);
      }
      continue;
    }

    const prevId = current.streamingMessageIds.get(sessionID);
    if (prevId && prevId !== streamingMsg.id) {
      const previous = nextStreamStates.get(prevId);
      if (previous && previous.phase === "streaming") {
        nextStreamStates.set(prevId, {
          ...previous,
          phase: "completed",
          completedAt: now
        });
        changed = true;
      }
    }
    if (prevId !== streamingMsg.id) changed = true;
    nextStreamingIds.set(sessionID, streamingMsg.id);

    const existing = nextStreamStates.get(streamingMsg.id);
    if (!existing || existing.phase !== "streaming") {
      nextStreamStates.set(streamingMsg.id, {
        phase: "streaming",
        startedAt: existing?.startedAt ?? now,
        lastUpdateAt: now
      });
      changed = true;
    } else if (now - existing.lastUpdateAt >= STREAMING_HEARTBEAT_MS) {
      nextStreamStates.set(streamingMsg.id, {
        ...existing,
        lastUpdateAt: now
      });
      changed = true;
    }
  }

  for (const [sessionID, msgId] of current.streamingMessageIds) {
    if (!msgId) continue;
    const isStillBusy = busySessionIds.has(sessionID);
    if (isStillBusy) continue;

    completeStreamingMessage(sessionID, msgId);
  }

  if (!changed) return null;
  return {
    streamingMessageIds: nextStreamingIds,
    messageStreamStates: nextStreamStates
  };
}

export function touchStreamingSession(current: StreamingStore, sessionID: string, now = Date.now()): StreamingStore | null {
  const messageID = current.streamingMessageIds.get(sessionID);
  if (!messageID) return null;
  const existing = current.messageStreamStates.get(messageID);
  if (!existing || existing.phase !== "streaming" || now - existing.lastUpdateAt < STREAMING_HEARTBEAT_MS) return null;

  const messageStreamStates = new Map(current.messageStreamStates);
  messageStreamStates.set(messageID, { ...existing, lastUpdateAt: now });
  return { ...current, messageStreamStates };
}

const collectChangedSessionIds = (
  current: Record<string, unknown> | undefined,
  previous: Record<string, unknown> | undefined,
  changed: Set<string>
): void => {
  const currentRecord = current ?? {};
  const previousRecord = previous ?? {};
  for (const sessionID of Object.keys(currentRecord)) {
    if (currentRecord[sessionID] !== previousRecord[sessionID]) changed.add(sessionID);
  }
  for (const sessionID of Object.keys(previousRecord)) {
    if (Object.prototype.hasOwnProperty.call(currentRecord, sessionID)) continue;
    changed.add(sessionID);
  }
};

export function updateChangedStreamingSessions(
  state: ChatDirectoryReadView,
  previous: ChatSnapshotReadView,
  current: StreamingStore,
  now = Date.now()
): StreamingStore | null {
  const statusChanged = state.session_status !== previous.session_status;
  const messagesChanged = state.message !== previous.message;
  if (!statusChanged && !messagesChanged) return null;

  const changedSessionIds = new Set<string>();
  if (statusChanged) {
    collectChangedSessionIds(
      state.session_status as Record<string, unknown> | undefined,
      previous.session_status as Record<string, unknown> | undefined,
      changedSessionIds
    );
  }
  if (messagesChanged) {
    collectChangedSessionIds(
      state.message as Record<string, unknown> | undefined,
      previous.message as Record<string, unknown> | undefined,
      changedSessionIds
    );
  }
  if (changedSessionIds.size === 0) return null;

  const nextStreamingIds = new Map(current.streamingMessageIds);
  const nextStreamStates = new Map(current.messageStreamStates);
  let changed = false;

  const complete = (sessionID: string, messageID: string) => {
    if (nextStreamingIds.get(sessionID) !== null) {
      nextStreamingIds.set(sessionID, null);
      changed = true;
    }
    const existing = nextStreamStates.get(messageID);
    if (existing?.phase === "streaming") {
      nextStreamStates.set(messageID, { ...existing, phase: "completed", completedAt: now });
      changed = true;
    }
  };

  for (const sessionID of changedSessionIds) {
    const previousMessageID = current.streamingMessageIds.get(sessionID);
    if (state.session_status?.[sessionID]?.type !== "busy" && state.session_status?.[sessionID]?.type !== "retry") {
      if (previousMessageID) complete(sessionID, previousMessageID);
      continue;
    }

    const streamingMessage = findTrailingAssistantMessage(state.message[sessionID]);

    if (!streamingMessage) {
      if (previousMessageID) complete(sessionID, previousMessageID);
      continue;
    }

    if (previousMessageID && previousMessageID !== streamingMessage.id) {
      complete(sessionID, previousMessageID);
    }
    if (nextStreamingIds.get(sessionID) !== streamingMessage.id) {
      nextStreamingIds.set(sessionID, streamingMessage.id);
      changed = true;
    }

    const existing = nextStreamStates.get(streamingMessage.id);
    if (!existing || existing.phase !== "streaming") {
      nextStreamStates.set(streamingMessage.id, {
        phase: "streaming",
        startedAt: existing?.startedAt ?? now,
        lastUpdateAt: now
      });
      changed = true;
    }
  }

  if (!changed) return null;
  return {
    streamingMessageIds: nextStreamingIds,
    messageStreamStates: nextStreamStates
  };
}
