import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import type { ChatStreamEvent } from "./chat-stream";
import type { ChatSnapshotReadView } from "./chat-store";
import { emptyStreamingStore } from "./streaming";
import { useSessionExecution, useSessionSettlement } from "./use-session-execution";

function rejectLifecycleQueries(host: Parameters<typeof useSessionSettlement>[1]) {
  // @ts-expect-error Settlement cannot replace borrowed panels.
  host.readPanels()[0] = host.readPanels()[0];
  // @ts-expect-error Settlement cannot change captured workspace identities.
  host.readPanels()[0].workspace.id = "other";
}
void rejectLifecycleQueries;

type Activity = ReturnType<typeof useSessionExecution>;
type Port = Parameters<typeof useSessionExecution>[0];
const panel: SessionInfo = { id: "session", directory: "/fixture", workspace: { id: "workspace", generation: 1 } };
const previous: ChatSnapshotReadView = { message: {}, session_status: {} };
const activeEvent: ChatStreamEvent = { id: "active", type: "session.text.delta", created: 1, data: { sessionID: panel.id } };
let activity: Activity;

function rejectMutation(value: Activity) {
  // @ts-expect-error owned busy entries cannot be assigned
  value.busyBySession.session = true;
  // @ts-expect-error abort fields are borrowed readonly
  value.abortFor("session")!.acknowledged = true;
  // @ts-expect-error abort dictionary entries cannot be replaced
  value.sessionAbortFlags.session = { timestamp: 0, acknowledged: false };
  // @ts-expect-error streaming identifiers cannot be mutated
  value.streamingStore.streamingMessageIds.set("session", "message");
  // @ts-expect-error stream timestamps are borrowed readonly
  value.streamingStore.messageStreamStates.get("message")!.lastUpdateAt = 2;
}
void rejectMutation;

describe("session execution authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  let port: Port;
  let rows: TranscriptItem[];
  let nextStreaming: ReturnType<typeof emptyStreamingStore> | null;
  let calls: string[];
  const panelsRef = { current: [panel] };
  const readPanels = () => panelsRef.current;
  const notify = vi.fn();
  function Probe() {
    activity = useSessionExecution({ ...port });
    return null;
  }
  function SettledProbe() {
    activity = useSessionExecution({ ...port });
    useSessionSettlement(activity, {
      readPanels, transcriptFor: port.transcriptFor, completeLatest: port.completeLatest,
      projectSession: port.projectSession, notify
    });
    return null;
  }
  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.useFakeTimers();
    vi.setSystemTime(1000);
    rows = [];
    nextStreaming = null;
    calls = [];
    panelsRef.current = [panel];
    notify.mockReset();
    port = {
      transcriptFor: vi.fn(() => rows),
      snapshotSession: vi.fn(() => { calls.push("snapshot"); return previous; }),
      reduceSessionEvent: vi.fn(() => { calls.push("reduce"); return false; }),
      recordStatusFor: vi.fn(() => undefined),
      projectSession: vi.fn(() => { calls.push("project"); }),
      attachRetry: vi.fn(() => { calls.push("retry"); return false; }),
      appendRateLimitNotice: vi.fn(),
      completeLatest: vi.fn(() => true),
      deriveChangedStreaming: vi.fn(() => { calls.push("sync"); return nextStreaming; }),
      deriveStreaming: vi.fn(() => nextStreaming)
    };
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<Probe />));
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("makes local stop and new-prompt acknowledgements immediate but terminal ACKs render-delayed", () => {
    act(() => {
      activity.beginLocalAbort(panel.id, 1);
      expect(activity.abortFor(panel.id)?.acknowledged).toBe(false);
      activity.finishExecution(panel.id, { ...activeEvent, type: "session.execution.interrupted" });
      expect(activity.abortFor(panel.id)?.acknowledged).toBe(false);
      expect(activity.shouldIgnoreRuntimeWork(panel.id, activeEvent)).toBe(true);
    });
    expect(activity.abortFor(panel.id)?.acknowledged).toBe(true);
    act(() => {
      activity.beginLocalAbort(panel.id, 2);
      activity.acknowledgePromptAbort(panel.id);
      expect(activity.abortFor(panel.id)?.acknowledged).toBe(true);
      expect(activity.shouldIgnoreRuntimeWork(panel.id, activeEvent)).toBe(false);
    });
  });

  it("distinguishes seeded false from absent state and prunes only captured eligible IDs", () => {
    const incomplete: TranscriptItem = { kind: "assistant", id: "assistant", messageID: "assistant", parts: [], completed: false };
    expect(activity.busyFor("new")).toBeUndefined();
    act(() => activity.seedReplayBusy(panel.id, { ...incomplete, completed: true }));
    const unreadable = { ...incomplete };
    Object.defineProperty(unreadable, "completed", { get() { throw new Error("known status evaluated replay"); } });
    act(() => activity.seedReplayBusy(panel.id, unreadable));
    expect(activity.busyFor(panel.id)).toBe(false);
    act(() => activity.seedReplayBusy("foreign", incomplete));
    act(() => activity.pruneBusyState([], { [panel.id]: [] }));
    expect(activity.busyFor(panel.id)).toBe(false);
    expect(activity.busyFor("foreign")).toBeUndefined();
  });

  it("preserves immediate stream commits and keeps streams/clocks when dropping off-view activity", () => {
    nextStreaming = emptyStreamingStore();
    nextStreaming.streamingMessageIds.set(panel.id, "message");
    nextStreaming.messageStreamStates.set("message", { phase: "streaming", startedAt: 1, lastUpdateAt: 1 });
    act(() => {
      activity.syncStreaming(panel.id, previous);
      expect(activity.streamingMessageFor(panel.id)).toBe("message");
      activity.beginLocalAbort(panel.id, 2);
      activity.setSessionBusy(panel.id, true);
      activity.markStreamActivity(panel.id, 3);
    });
    act(() => {
      activity.dropClosedActivity(panel.id);
      expect(activity.abortFor(panel.id)?.acknowledged).toBe(false);
    });
    expect(activity.abortFor(panel.id)).toBeUndefined();
    expect(activity.busyFor(panel.id)).toBeUndefined();
    expect(activity.streamingMessageFor(panel.id)).toBe("message");
    expect(activity.activityAt(panel.id)).toBe(3);
  });

  it("retains both status reduction passes and suppresses both passes for late aborted work", () => {
    const retry: ChatStreamEvent = { id: "retry", type: "session.status", created: 1, data: { status: { type: "retry", attempt: 2, next: 5000 } } };
    act(() => activity.applyStatusEvent("foreign", retry));
    expect(calls).toEqual(["snapshot", "reduce", "sync", "retry", "snapshot", "reduce", "sync"]);
    expect(activity.busyFor("foreign")).toBe(true);
    calls = [];
    act(() => { activity.beginLocalAbort("foreign", 4); activity.applyStatusEvent("foreign", retry); });
    expect(calls).toEqual([]);
    expect(activity.abortFor("foreign")?.acknowledged).toBe(false);
  });

  it("does not restart settlement polling when render objects change and disposes its timer", async () => {
    await act(async () => root.unmount());
    root = createRoot(container);
    await act(async () => root.render(<SettledProbe />));
    expect(vi.getTimerCount()).toBe(1);
    act(() => activity.setSessionBusy(panel.id, true));
    await act(async () => { vi.advanceTimersByTime(500); });
    await act(async () => root.render(<SettledProbe />));
    expect(vi.getTimerCount()).toBe(1);
    await act(async () => { vi.advanceTimersByTime(500); });
    expect(activity.activityAt(panel.id)).toBe(2000);
    await act(async () => { vi.advanceTimersByTime(60000); });
    expect(activity.busyFor(panel.id)).toBe(false);
    expect(notify).toHaveBeenCalledWith("Turn stalled — recovered");
  });
});
