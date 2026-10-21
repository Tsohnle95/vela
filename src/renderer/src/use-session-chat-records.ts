import { useCallback, useRef, useState } from "react";
import type { PendingPermissionRequest, PermissionReply, TranscriptItem } from "@shared/types";
import { retainSessionRecord } from "@shared/retention";
import { mergeChatHistory, reconcilePromptHistory, reduceChatStream, type ChatStreamEvent } from "./chat-stream";
import {
  applyChatEvent, attachRetryToLatestAssistant, completeLatestIncomplete,
  hydrateChatState, insertUserMessage, projectAssistantItems,
  reconcileAssistantProjection, snapshotChatState,
  type ChatDirectoryState, type ChatDirectoryReadView, type ChatEventResult,
  type ChatSessionStatus, type ChatSnapshotReadView
} from "./chat-store";
import { updateChangedStreamingSessions, updateStreamingState, type StreamingStore } from "./streaming";

type UserItem = Extract<TranscriptItem, { kind: "user" }>;

export function useSessionChatRecords(protectedSessionIDs: () => Set<string>) {
  const [transcriptsBySession, setTranscriptsBySession] = useState<Record<string, TranscriptItem[]>>({});
  const transcriptsBySessionRef = useRef(transcriptsBySession);
  transcriptsBySessionRef.current = transcriptsBySession;
  const chatStatesRef = useRef(new Map<string, ChatDirectoryState>());
  const updateSessionTranscript = useCallback(
    (sessionID: string, update: (items: TranscriptItem[]) => TranscriptItem[]) => {
      setTranscriptsBySession((current) => {
        const items = current[sessionID] ?? [];
        const next = update(items);
        return next === items
          ? current
          : retainSessionRecord(current, sessionID, next, protectedSessionIDs());
      });
    },
    [protectedSessionIDs]
  );
  const chatStateFor = useCallback((sessionID: string): ChatDirectoryState => {
    let state = chatStatesRef.current.get(sessionID);
    if (!state) {
      state = { message: {}, part: {}, session_status: {} };
      chatStatesRef.current.set(sessionID, state);
    }
    return state;
  }, []);
  const applyProjection = useCallback((sessionID: string) => {
    const state = chatStatesRef.current.get(sessionID);
    if (!state) return;
    const projected = projectAssistantItems(state, sessionID);
    const messages = state.message[sessionID] ?? [];
    updateSessionTranscript(sessionID, (previous) => reconcileAssistantProjection(previous, projected, messages));
  }, [updateSessionTranscript]);
  const transcriptFor = useCallback((sessionID: string): readonly TranscriptItem[] =>
    transcriptsBySessionRef.current[sessionID] ?? [], []);
  const hasTranscript = useCallback((sessionID: string): boolean => Boolean(transcriptsBySessionRef.current[sessionID]), []);
  const recordStatusFor = useCallback((sessionID: string): Readonly<ChatSessionStatus> | undefined =>
    chatStatesRef.current.get(sessionID)?.session_status[sessionID], []);
  const snapshotSession = useCallback((sessionID: string): ChatSnapshotReadView => snapshotChatState(chatStateFor(sessionID)), [chatStateFor]);
  const reduceSessionEvent = useCallback((sessionID: string, event: ChatStreamEvent): ChatEventResult =>
    applyChatEvent(chatStateFor(sessionID), sessionID, event), [chatStateFor]);
  const completeLatest = useCallback((sessionID: string): boolean => completeLatestIncomplete(chatStateFor(sessionID), sessionID), [chatStateFor]);
  const attachRetry = useCallback((sessionID: string, retry: { attempt: number; message: string; next?: number }): boolean =>
    attachRetryToLatestAssistant(chatStateFor(sessionID), sessionID, retry), [chatStateFor]);
  const deriveChangedStreaming = useCallback((sessionID: string, previous: ChatSnapshotReadView, current: StreamingStore): StreamingStore | null =>
    updateChangedStreamingSessions(chatStateFor(sessionID), previous, current), [chatStateFor]);
  const deriveStreaming = useCallback((sessionID: string, current: StreamingStore): StreamingStore | null =>
    updateStreamingState(chatStateFor(sessionID), current), [chatStateFor]);
  const prepareReopenedReplay = useCallback((sessionID: string, history: TranscriptItem[], completeAborted: boolean, authoritative = false): TranscriptItem[] => {
    const merged = authoritative ? history : mergeChatHistory(history, transcriptsBySessionRef.current[sessionID] ?? []);
    chatStatesRef.current.delete(sessionID);
    const draft = chatStateFor(sessionID);
    hydrateChatState(draft, sessionID, merged);
    if (completeAborted) completeLatestIncomplete(draft, sessionID);
    return merged;
  }, [chatStateFor]);
  const commitReplayTranscript = useCallback((sessionID: string, merged: TranscriptItem[]): void => {
    setTranscriptsBySession((current) => retainSessionRecord(current, sessionID, merged, protectedSessionIDs()));
  }, [protectedSessionIDs]);
  const mergeMaterializedRecords = useCallback((sessionID: string, history: TranscriptItem[], completeAborted: boolean): void => {
    const draft = chatStateFor(sessionID);
    hydrateChatState(draft, sessionID, history);
    if (completeAborted) completeLatestIncomplete(draft, sessionID);
  }, [chatStateFor]);
  const hydratePromptRecords = useCallback((sessionID: string, merged: TranscriptItem[]): void => {
    hydrateChatState(chatStateFor(sessionID), sessionID, merged);
  }, [chatStateFor]);
  const reconcilePromptTranscript = useCallback((sessionID: string, canonical: TranscriptItem[], userItem: UserItem): TranscriptItem[] => {
    const current = transcriptsBySessionRef.current[sessionID] ?? [];
    const merged = reconcilePromptHistory(canonical, current, userItem);
    updateSessionTranscript(sessionID, () => merged);
    return merged;
  }, [updateSessionTranscript]);
  const forgetRecords = useCallback((sessionID: string): void => { chatStatesRef.current.delete(sessionID); }, []);
  const appendTranscriptItem = useCallback((sessionID: string, item: TranscriptItem): void => {
    updateSessionTranscript(sessionID, (previous) => [...previous, item]);
  }, [updateSessionTranscript]);
  const appendUniqueTranscriptItem = useCallback((sessionID: string, item: TranscriptItem): void => {
    updateSessionTranscript(sessionID, (previous) => previous.some((existing) => existing.id === item.id) ? previous : [...previous, item]);
  }, [updateSessionTranscript]);
  const appendRateLimitNotice = useCallback((sessionID: string, notice: TranscriptItem): void => {
    updateSessionTranscript(sessionID, (prev) => {
      if (prev.some((item) => item.kind === "status" && item.id === notice.id)) return prev;
      return [...prev, notice];
    });
  }, [updateSessionTranscript]);
  const replaceTranscriptItemAtTail = useCallback((sessionID: string, item: TranscriptItem): void => {
    updateSessionTranscript(sessionID, (previous) => [...previous.filter((existing) => existing.id !== item.id), item]);
  }, [updateSessionTranscript]);
  const appendPromptUser = useCallback((sessionID: string, item: UserItem): void => {
    updateSessionTranscript(sessionID, (previous) => [...previous, item]);
    insertUserMessage(chatStateFor(sessionID), sessionID, item.id, item.text, item.model, item);
  }, [updateSessionTranscript, chatStateFor]);
  const promoteDeliveredInput = useCallback((sessionID: string, inboxID: string): void => {
    const buffered = (transcriptsBySessionRef.current[sessionID] ?? []).find((item) => item.kind === "pending-input" && item.id === inboxID);
    if (buffered?.kind === "pending-input" && buffered.inputType === "user") {
      insertUserMessage(chatStateFor(sessionID), sessionID, inboxID, buffered.text, buffered.model, buffered);
    }
  }, [chatStateFor]);
  const reduceAuxiliaryEvent = useCallback((sessionID: string, event: ChatStreamEvent): void => {
    updateSessionTranscript(sessionID, (previous) => reduceChatStream(previous, event));
  }, [updateSessionTranscript]);
  const addPermissionFromEvent = useCallback((sessionID: string, requestID: string, data: ChatStreamEvent["data"]): void => {
    updateSessionTranscript(sessionID, (prev) => {
      if (prev.some((i) => i.kind === "permission" && i.requestID === requestID)) return prev;
      return [
        ...prev,
        {
          kind: "permission",
          id: requestID,
          requestID,
          action: String(data.action ?? data.permission ?? "unknown"),
          resources: Array.isArray(data.resources ?? data.patterns)
            ? (data.resources ?? data.patterns).map(String)
            : [],
          pending: true
        }
      ];
    });
  }, [updateSessionTranscript]);
  const resolvePermission = useCallback((sessionID: string, requestID: string, reply: PermissionReply): void => {
    updateSessionTranscript(sessionID, (previous) => previous.map((item) => item.kind === "permission" && item.requestID === requestID
      ? { ...item, pending: false, resolvedWith: reply } : item));
  }, [updateSessionTranscript]);
  const reconcilePermissionRows = useCallback((sessionID: string, requests: PendingPermissionRequest[]): void => {
    const liveIDs = new Set(requests.map((request) => request.id));
    updateSessionTranscript(sessionID, (prev) => {
      let changed = false;
      let next = prev;
      for (const request of requests) {
        if (next.some((item) => item.kind === "permission" && item.requestID === request.id)) continue;
        changed = true;
        next = [
          ...next,
          {
            kind: "permission",
            id: request.id,
            requestID: request.id,
            action: request.action,
            resources: request.resources,
            pending: true
          }
        ];
      }
      if (next.some((item) => item.kind === "permission" && item.pending && !liveIDs.has(item.requestID))) {
        changed = true;
        next = next.map((item) =>
          item.kind === "permission" && item.pending && !liveIDs.has(item.requestID)
            ? { ...item, pending: false }
            : item
        );
      }
      return changed ? next : prev;
    });
  }, [updateSessionTranscript]);
  const rawChatReadMap: ReadonlyMap<string, ChatDirectoryReadView> = chatStatesRef.current;
  const transcriptReadMap: Readonly<Record<string, TranscriptItem[]>> = transcriptsBySession;
  return {
    transcriptsBySession: transcriptReadMap, rawChatReadMap, transcriptFor, hasTranscript, recordStatusFor,
    snapshotSession, reduceSessionEvent, completeLatest, attachRetry, projectSession: applyProjection,
    deriveChangedStreaming, deriveStreaming, prepareReopenedReplay, commitReplayTranscript,
    mergeMaterializedRecords, hydratePromptRecords, reconcilePromptTranscript, forgetRecords,
    appendTranscriptItem, appendUniqueTranscriptItem, replaceTranscriptItemAtTail,
    appendRateLimitNotice,
    appendPromptUser, promoteDeliveredInput, reduceAuxiliaryEvent,
    addPermissionFromEvent, resolvePermission, reconcilePermissionRows
  };
}
