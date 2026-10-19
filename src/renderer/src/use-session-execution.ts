import type { SessionReadView } from "./store-contract";
import { useCallback, useEffect, useRef, useState } from "react";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import { formatFailure } from "@shared/errors";
import type { SessionAbortFlag } from "./assistant-status";
import type { ChatStreamEvent } from "./chat-stream";
import {
  buildRateLimitNotice, type ChatEventResult, type ChatSessionStatus,
  type ChatSnapshotReadView, type RateLimitAction
} from "./chat-store";
import { emptyStreamingStore, touchStreamingSession, type StreamingStore, type StreamingReadView } from "./streaming";

interface ExecutionRecords {
  readonly transcriptFor: (sessionID: string) => readonly TranscriptItem[];
  readonly snapshotSession: (sessionID: string) => ChatSnapshotReadView;
  readonly reduceSessionEvent: (sessionID: string, event: ChatStreamEvent) => ChatEventResult;
  readonly recordStatusFor: (sessionID: string) => Readonly<ChatSessionStatus> | undefined;
  readonly projectSession: (sessionID: string) => void;
  readonly attachRetry: (sessionID: string, retry: { attempt: number; message: string; next?: number }) => boolean;
  readonly appendRateLimitNotice: (sessionID: string, notice: TranscriptItem) => void;
  readonly completeLatest: (sessionID: string) => boolean;
  readonly deriveChangedStreaming: (sessionID: string, previous: ChatSnapshotReadView, current: StreamingStore) => StreamingStore | null;
  readonly deriveStreaming: (sessionID: string, current: StreamingStore) => StreamingStore | null;
}

const STREAM_SETTLE_MS = 60_000;
const STREAM_SETTLE_IDLE_MS = 2_000;
const STREAM_SETTLE_POLL_MS = 1_000;

function dropKey<T>(record: Record<string, T>, key: string): Record<string, T> {
  if (!(key in record)) return record;
  const next = { ...record };
  delete next[key];
  return next;
}

const ACTIVE_CHAT_STREAM_TYPES = new Set([
  "session.step.started",
  "session.text.started",
  "session.text.delta",
  "session.reasoning.started",
  "session.reasoning.delta",
  "session.tool.input.started",
  "session.tool.input.delta",
  "session.tool.called",
  "session.tool.progress",
  "session.retry.scheduled",
  "message.part.delta"
]);

function streamEventShowsActiveWork(event: ChatStreamEvent): boolean {
  if (ACTIVE_CHAT_STREAM_TYPES.has(event.type)) return true;
  if (event.type === "message.updated") {
    const info = (event.data.info ?? event.data) as Record<string, any>;
    const role = String(info.role ?? info.type ?? "assistant");
    return role === "assistant" && !info.time?.completed && !info.finish && !info.error;
  }
  if (event.type === "message.part.updated") {
    const part = (event.data.part ?? event.data) as Record<string, any>;
    const state = part.state as Record<string, any> | undefined;
    const status = String(state?.status ?? "");
    return !part.time?.completed && !["completed", "error", "failed", "success", "aborted", "timeout", "cancelled"].includes(status);
  }
  return false;
}

export function useSessionExecution(records: ExecutionRecords) {
  const {
    snapshotSession, reduceSessionEvent, recordStatusFor, projectSession: applyProjection,
    attachRetry, appendRateLimitNotice, deriveChangedStreaming, deriveStreaming
  } = records;
  const [busyBySession, setBusyBySession] = useState<Record<string, boolean>>({});
  const [streamingStore, setStreamingStore] = useState<StreamingStore>(() => emptyStreamingStore());
  const [sessionAbortFlags, setSessionAbortFlags] = useState<Record<string, SessionAbortFlag>>({});
  const busyBySessionRef = useRef(busyBySession);
  busyBySessionRef.current = busyBySession;
  const sessionAbortFlagsRef = useRef(sessionAbortFlags);
  sessionAbortFlagsRef.current = sessionAbortFlags;
  const lastStreamActivityRef = useRef<Record<string, number>>({});
  const streamingRef = useRef<StreamingStore>(streamingStore);
  const busyFor = useCallback((sessionID: string): boolean | undefined => busyBySessionRef.current[sessionID], []);
  const abortFor = useCallback((sessionID: string): Readonly<SessionAbortFlag> | undefined => sessionAbortFlagsRef.current[sessionID], []);
  const streamingMessageFor = useCallback((sessionID: string): string | null | undefined => streamingRef.current.streamingMessageIds.get(sessionID), []);
  const activityAt = useCallback((sessionID: string): number | undefined => lastStreamActivityRef.current[sessionID], []);
  const markStreamActivity = useCallback((sessionID: string, now = Date.now()): void => { lastStreamActivityRef.current[sessionID] = now; }, []);
  const setSessionBusy = useCallback((sessionID: string, value: boolean) => {
    setBusyBySession((current) => current[sessionID] === value
      ? current
      : { ...current, [sessionID]: value });
  }, []);

  const commitStreaming = useCallback((next: StreamingStore) => {
    streamingRef.current = next;
    setStreamingStore(next);
  }, []);

  const syncStreaming = useCallback((sessionID: string, previous: ChatSnapshotReadView) => {
    const next = deriveChangedStreaming(sessionID, previous, streamingRef.current);
    if (next) commitStreaming(next);
  }, [deriveChangedStreaming, commitStreaming]);

  const reconcileStreaming = useCallback((sessionID: string) => {
    const next = deriveStreaming(sessionID, streamingRef.current);
    if (next) commitStreaming(next);
  }, [deriveStreaming, commitStreaming]);

  const seedReplayBusy = useCallback((sessionID: string, running: TranscriptItem | undefined, aborted = false): void => {
    setBusyBySession((current) => sessionID in current
      ? current
      : { ...current, [sessionID]: !aborted && Boolean(running?.kind === "assistant" && !running.completed) });
  }, []);
  const pruneBusyState = useCallback((panels: readonly SessionInfo[], transcripts: Readonly<Record<string, TranscriptItem[]>>): void => {
    const open = new Set(panels.map((panel) => panel.id));
    setBusyBySession((current) => {
      const entries = Object.entries(current).filter(([id]) => open.has(id) || id in transcripts);
      return entries.length === Object.keys(current).length ? current : Object.fromEntries(entries);
    });
  }, []);
  const dropClosedActivity = useCallback((sessionID: string): void => {
    setBusyBySession((current) => dropKey(current, sessionID));
    setSessionAbortFlags((current) => dropKey(current, sessionID));
  }, []);
  const beginLocalAbort = useCallback((sessionID: string, timestamp: number): Readonly<SessionAbortFlag> => {
    const abortFlag = { timestamp, acknowledged: false };
    const nextAbortFlags = { ...sessionAbortFlagsRef.current, [sessionID]: abortFlag };
    sessionAbortFlagsRef.current = nextAbortFlags;
    setSessionAbortFlags(nextAbortFlags);
    return abortFlag;
  }, []);
  const acknowledgePromptAbort = useCallback((sessionID: string): void => {
    const abortFlag = sessionAbortFlagsRef.current[sessionID];
    if (abortFlag && !abortFlag.acknowledged) {
      const nextAbortFlags = { ...sessionAbortFlagsRef.current, [sessionID]: { ...abortFlag, acknowledged: true } };
      sessionAbortFlagsRef.current = nextAbortFlags;
      setSessionAbortFlags(nextAbortFlags);
    }
  }, []);
  // Keep terminal ACK render-delayed so same-batch late work remains suppressed.
  const acknowledgeTerminalAbort = useCallback((sessionID: string): void => {
    setSessionAbortFlags((current) => current[sessionID]
      ? { ...current, [sessionID]: { ...current[sessionID], acknowledged: true } }
      : current);
  }, []);
  const shouldIgnoreRuntimeWork = useCallback((sessionID: string, event: ChatStreamEvent): boolean =>
    sessionAbortFlagsRef.current[sessionID]?.acknowledged === false && streamEventShowsActiveWork(event), []);
  const noteRuntimeReduction = useCallback((targetSessionID: string, streamEvent: ChatStreamEvent, result: ChatEventResult): void => {
    if (result === true || (typeof result !== "boolean" && result.changed)) {
      lastStreamActivityRef.current[targetSessionID] = Date.now();
      applyProjection(targetSessionID);
      const abortFlag = sessionAbortFlagsRef.current[targetSessionID];
      const abortPending = Boolean(abortFlag && !abortFlag.acknowledged);
      const sessionStatus = recordStatusFor(targetSessionID);
      if (sessionStatus?.type === "idle" || sessionStatus?.type === "error") {
        setSessionBusy(targetSessionID, false);
      } else if (!abortPending && !busyBySessionRef.current[targetSessionID] && streamEventShowsActiveWork(streamEvent)) {
        setSessionBusy(targetSessionID, true);
      }
    }
  }, [applyProjection, recordStatusFor, setSessionBusy]);
  const touchDeltaStream = useCallback((sessionID: string, type: string): void => {
    if (type === "message.part.delta" || type === "session.text.delta" || type === "session.reasoning.delta" || type === "session.tool.input.delta") {
      const touched = touchStreamingSession(streamingRef.current, sessionID);
      if (touched) commitStreaming(touched);
    }
  }, [commitStreaming]);
  const startExecution = useCallback((sessionID: string, streamEvent: ChatStreamEvent): void => {
    if (sessionAbortFlagsRef.current[sessionID]?.acknowledged === false) return;
    const previous = snapshotSession(sessionID);
    reduceSessionEvent(sessionID, streamEvent);
    syncStreaming(sessionID, previous);
    // Reset the quiet clock across slow model prefill and tool gaps.
    lastStreamActivityRef.current[sessionID] = Date.now();
    setSessionBusy(sessionID, true);
  }, [snapshotSession, reduceSessionEvent, syncStreaming, setSessionBusy]);
  const finishExecution = useCallback((sessionID: string, streamEvent: ChatStreamEvent): void => {
    setSessionBusy(sessionID, false);
    const previous = snapshotSession(sessionID);
    const result = reduceSessionEvent(sessionID, streamEvent);
    if (result === true || (typeof result !== "boolean" && result.changed)) applyProjection(sessionID);
    syncStreaming(sessionID, previous);
    acknowledgeTerminalAbort(sessionID);
  }, [setSessionBusy, snapshotSession, reduceSessionEvent, applyProjection, syncStreaming, acknowledgeTerminalAbort]);
  const applyStatusEvent = useCallback((targetSessionID: string | undefined, streamEvent: ChatStreamEvent): void => {
    const data = streamEvent.data;
    const status = data.status as { type?: string; attempt?: number; message?: string; next?: number; action?: RateLimitAction } | undefined;
    const abortPending = Boolean(targetSessionID && sessionAbortFlagsRef.current[targetSessionID]?.acknowledged === false);
    const activeStatus = status?.type === "busy" || status?.type === "retry";
    if (targetSessionID && !(abortPending && activeStatus)) {
      const previous = snapshotSession(targetSessionID);
      reduceSessionEvent(targetSessionID, streamEvent);
      syncStreaming(targetSessionID, previous);
    }
    if (activeStatus && targetSessionID && !abortPending) {
      lastStreamActivityRef.current[targetSessionID] = Date.now();
    }
    if (status?.type === "busy" && targetSessionID && !abortPending) setSessionBusy(targetSessionID, true);
    if ((status?.type === "idle" || status?.type === "error") && targetSessionID) setSessionBusy(targetSessionID, false);
    if (status?.type === "retry" && targetSessionID && !abortPending) {
      setSessionBusy(targetSessionID, true);
      if (attachRetry(targetSessionID, {
        attempt: Number(status.attempt ?? 1),
        message: formatFailure(status.message, "VELA_RETRY_SCHEDULED", "Retrying"),
        next: status.next
      })) {
        applyProjection(targetSessionID);
      }
      if (status.action && Number(status.attempt ?? 1) === 1) {
        const notice = buildRateLimitNotice(status.action, targetSessionID);
        if (notice) {
          appendRateLimitNotice(targetSessionID, notice);
        }
      }
    }
    if (targetSessionID && !(abortPending && activeStatus)) {
      const previous = snapshotSession(targetSessionID);
      const result = reduceSessionEvent(targetSessionID, streamEvent);
      if (result === true || (typeof result !== "boolean" && result.changed)) applyProjection(targetSessionID);
      syncStreaming(targetSessionID, previous);
    }
  }, [snapshotSession, reduceSessionEvent, syncStreaming, setSessionBusy, attachRetry, applyProjection, appendRateLimitNotice]);
  const busyReadMap: Readonly<Record<string, boolean>> = busyBySession;
  const abortReadMap: Readonly<Record<string, Readonly<SessionAbortFlag>>> = sessionAbortFlags;
  const streamingReadView: StreamingReadView = streamingStore;
  return {
    busyBySession: busyReadMap, sessionAbortFlags: abortReadMap, streamingStore: streamingReadView,
    busyFor, abortFor, streamingMessageFor, activityAt, markStreamActivity,
    setSessionBusy, seedReplayBusy, pruneBusyState, dropClosedActivity,
    beginLocalAbort, acknowledgePromptAbort, syncStreaming, reconcileStreaming,
    shouldIgnoreRuntimeWork, noteRuntimeReduction, touchDeltaStream,
    startExecution, finishExecution, applyStatusEvent
  };
}

interface SettlementHost {
  readPanels: () => readonly SessionReadView[];
  transcriptFor: ExecutionRecords["transcriptFor"];
  completeLatest: ExecutionRecords["completeLatest"];
  projectSession: ExecutionRecords["projectSession"];
  notify: (text: string, tone?: "info" | "error") => void;
}

export function useSessionSettlement(activity: ReturnType<typeof useSessionExecution>, host: SettlementHost): void {
  const { busyFor, activityAt, markStreamActivity, reconcileStreaming, setSessionBusy } = activity;
  const { readPanels, transcriptFor, completeLatest, projectSession: applyProjection, notify } = host;
  useEffect(() => {
    const finalizeTrailingAssistant = (sessionID: string, reason: "settle" | "idle"): void => {
      const wasBusy = Boolean(busyFor(sessionID));
      completeLatest(sessionID);
      applyProjection(sessionID);
      reconcileStreaming(sessionID);
      setSessionBusy(sessionID, false);
      if (wasBusy) {
        const detail = reason === "settle" ? "Turn stalled — recovered" : "Recovered incomplete turn";
        notify(detail);
      }
    };
    const timer = setInterval(() => {
      const now = Date.now();
      for (const panel of readPanels()) {
        const transcript = transcriptFor(panel.id);
        const trailing = [...transcript].reverse().find((item) => item.kind === "assistant");
        const isBusy = Boolean(busyFor(panel.id));
        if (trailing?.kind !== "assistant" || trailing.completed) {
          if (!isBusy) continue;
          const lastActivity = activityAt(panel.id);
          if (lastActivity === undefined) {
            markStreamActivity(panel.id, now);
            continue;
          }
          if (now - lastActivity < STREAM_SETTLE_MS) continue;
          setSessionBusy(panel.id, false);
          notify("Turn stalled — recovered");
          continue;
        }
        if (!isBusy) {
          const lastActivity = activityAt(panel.id);
          if (lastActivity === undefined) {
            markStreamActivity(panel.id, now);
            continue;
          }
          if (now - lastActivity < STREAM_SETTLE_IDLE_MS) continue;
          finalizeTrailingAssistant(panel.id, "idle");
          continue;
        }
        if (trailing.retry) {
          const next = trailing.retry.next;
          if (typeof next === "number" && next > now && next - now < 60_000) continue;
        }
        const lastActivity = activityAt(panel.id);
        if (lastActivity === undefined) {
          markStreamActivity(panel.id, now);
          continue;
        }
        if (now - lastActivity < STREAM_SETTLE_MS) continue;
        finalizeTrailingAssistant(panel.id, "settle");
      }
    }, STREAM_SETTLE_POLL_MS);
    return () => clearInterval(timer);
  }, [completeLatest, applyProjection, reconcileStreaming, setSessionBusy, transcriptFor, busyFor, activityAt, markStreamActivity, notify, readPanels]);
}
