import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, ProjectInfo, SessionInfo, SessionInboxEntry } from "@shared/types";
import type { VelaApi } from "../../preload";
import { StoreProvider, useStore } from "./store";

let store: ReturnType<typeof useStore>;
let receive: (message: BackendMessage) => void;
const first: SessionInfo = { id: "session-1", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
const localID = "queued-local";
function Probe() { store = useStore(); return null; }
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}
function api(overrides: Partial<VelaApi> = {}): VelaApi {
  return {
    platform: "darwin", onMessage: (handler) => { receive = handler; return () => {}; },
    health: async () => true, takePendingPaths: async () => [], state: async () => null,
    activeSessions: async () => [], sessions: async () => [], models: async () => [],
    modelDefault: async () => null, sessionSelection: async () => null, agents: async () => [],
    savedWorkspaces: async () => [], setSavedWorkspaces: async (items: ProjectInfo[]) => items,
    openSession: async () => first, openSessionById: async () => ({ session: first, transcript: [], todos: [], usage: null }),
    closeSession: async () => {}, recoveryRecords: async () => [], listPermissions: async () => [], formsList: async () => [],
    inboxList: async () => [], inboxCancel: async () => {}, inboxSteer: async () => {},
    ...overrides
  } as VelaApi;
}
function seedLocalQueue() {
  window.localStorage.setItem("messageQueue", JSON.stringify({
    followUpBehavior: "queue", quarantinedLegacyMessages: {},
    queuedMessages: { [`${first.workspace.id}\n${first.id}`]: [{ id: localID, content: "local text", createdAt: 1, attachments: [{ path: "attached", mention: { start: 0, end: 1, text: "a" } }] }] }
  }));
}
function enqueueRemote(id = "remote", text = "remote text", delivery: "queue" | "steer" | null = "queue") {
  receive({ kind: "event", type: "session.inbox.enqueued", data: {
    id: `event-${id}`, type: "session.inbox.enqueued", created: 2,
    data: { sessionID: first.id, inboxID: id, ...(delivery ? { delivery } : {}), item: { type: "user", payload: { text } } }
  } });
}

describe("Store inbox and local queue authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });
  async function open(overrides: Partial<VelaApi> = {}) {
    window.vela = api(overrides);
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession(first.directory));
  }

  it("preserves immediate local pops versus last-render server inbox pops in one batch", async () => {
    seedLocalQueue();
    const inboxCancel = vi.fn(async () => { throw new Error("cancel remains silent"); });
    await open({ inboxCancel });
    act(() => enqueueRemote());
    let firstLocal, secondLocal, firstRemote, secondRemote;
    act(() => {
      firstLocal = store.popQueuedMessage(first.workspace, localID);
      secondLocal = store.popQueuedMessage(first.workspace, localID);
      firstRemote = store.popQueuedMessage(first.workspace, "remote");
      secondRemote = store.popQueuedMessage(first.workspace, "remote");
    });
    expect(firstLocal).toMatchObject({ content: "local text" });
    expect(secondLocal).toBeNull();
    expect(firstRemote).toEqual({ id: "remote", content: "remote text", createdAt: expect.any(Number) });
    expect(secondRemote).toEqual(firstRemote);
    expect(inboxCancel).toHaveBeenCalledTimes(2);
  });

  it("requeues a failed local send for its captured session after that panel closes", async () => {
    seedLocalQueue();
    const prompt = deferred<Awaited<ReturnType<VelaApi["prompt"]>>>();
    await open({ prompt: () => prompt.promise });
    let sending!: Promise<void>;
    act(() => { sending = store.sendQueuedNow(first.workspace, localID); });
    await act(async () => store.closePanel(first.id));
    expect(store.panels).toEqual([]);
    await act(async () => { prompt.reject(new Error("late failure")); await sending; });
    const persisted = JSON.parse(window.localStorage.getItem("messageQueue")!);
    expect(persisted.queuedMessages[`${first.workspace.id}\n${first.id}`]).toEqual([
      expect.objectContaining({ content: "local text", attachments: [{ path: "attached", mention: { start: 0, end: 1, text: "a" } }] })
    ]);
    expect(store.toasts.some((toast) => toast.text.includes("late failure"))).toBe(false);
  });

  it("ignores local queued-send success materialization and leaves busy state unchanged", async () => {
    seedLocalQueue();
    await open({ prompt: async () => ({ session: first, transcript: [{ kind: "status", id: "ignored", text: "ignored response", tone: "info" }], todos: [{ id: "ignored", content: "ignored", status: "pending", priority: "low" }], usage: null }) });
    expect(store.busy).toBe(false);
    await act(async () => store.sendQueuedNow(first.workspace, localID));
    expect(store.busy).toBe(false);
    expect(store.transcript).toContainEqual(expect.objectContaining({ kind: "user", text: "local text" }));
    expect(store.transcript.some((item) => item.id === "ignored")).toBe(false);
    expect(store.todos).toEqual([]);
  });

  it("stores a late inbox refresh after close and exposes it when the same session reopens", async () => {
    const initial = deferred<SessionInboxEntry[]>();
    const reopening = deferred<SessionInboxEntry[]>();
    const inboxList = vi.fn().mockImplementationOnce(() => initial.promise).mockImplementation(() => reopening.promise);
    await open({ inboxList });
    await act(async () => store.closePanel(first.id));
    await act(async () => { initial.resolve([{ id: "late", text: "retained", createdAt: 1, attachmentCount: 0 }]); await initial.promise; });
    await act(async () => store.reopenSession(first.id));
    let popped;
    act(() => { popped = store.popQueuedMessage(first.workspace, "late"); });
    expect(popped).toEqual({ id: "late", content: "retained", createdAt: 1 });
  });
  it("keeps queue persistence on no-op reorder and immediate ref progress when storage is denied", async () => {
    seedLocalQueue();
    await open();
    const original = window.localStorage.setItem.bind(window.localStorage);
    const persist = vi.spyOn(window.localStorage, "setItem");
    act(() => store.reorderQueuedMessage(first.workspace, localID, localID));
    expect(persist.mock.calls.filter(([key]) => key === "messageQueue")).toHaveLength(1);
    persist.mockClear();
    const deny = persist.mockImplementation((key, value) => {
      if (key === "messageQueue") throw new DOMException("denied", "SecurityError");
      original(key, value);
    });
    let firstPop, secondPop;
    act(() => {
      store.setFollowUpBehavior("steer");
      firstPop = store.popQueuedMessage(first.workspace, localID);
      secondPop = store.popQueuedMessage(first.workspace, localID);
    });
    expect(store.followUpBehavior).toBe("steer");
    expect(firstPop).toMatchObject({ content: "local text" });
    expect(secondPop).toBeNull();
    expect(deny.mock.calls.filter(([key]) => key === "messageQueue")).toHaveLength(3);
  });

  it("suppresses a late refresh error after close but toasts it for an attached session ID", async () => {
    const firstRead = deferred<SessionInboxEntry[]>();
    await open({ inboxList: () => firstRead.promise });
    await act(async () => store.closePanel(first.id));
    await act(async () => { firstRead.reject(new Error("closed read failure")); try { await firstRead.promise; } catch {} });
    expect(store.toasts.some((toast) => toast.text.includes("closed read failure"))).toBe(false);
    const secondRead = deferred<SessionInboxEntry[]>();
    window.vela.inboxList = () => secondRead.promise;
    await act(async () => store.openSession(first.directory));
    await act(async () => { secondRead.reject(new Error("attached read failure")); try { await secondRead.promise; } catch {} });
    expect(store.toasts.at(-1)?.text).toContain("attached read failure");
  });

  it("reconciles a normal successful prompt against only matching non-queue inbox entries", async () => {
    await open({ prompt: async () => ({ session: first, transcript: [], todos: [], usage: null }) });
    act(() => {
      enqueueRemote("queue", "same", "queue");
      enqueueRemote("steer", "same", "steer");
      enqueueRemote("unspecified", "same", null);
      enqueueRemote("other", "different", "queue");
    });
    await act(async () => store.sendPrompt("same"));
    expect(store.panelViews[first.workspace.id].queuedMessages.map((entry) => entry.id)).toEqual(["queue", "other"]);
  });

  it.each([
    { name: "valid", queue: '{"followUpBehavior":"steer","queuedMessages":{}}', saved: '[]', denied: "", expected: "steer" },
    { name: "malformed", queue: '{invalid', saved: '{invalid', denied: "", expected: "queue" },
    { name: "queue read denied", queue: '{"followUpBehavior":"steer"}', saved: '[]', denied: "messageQueue", expected: "queue" },
    { name: "saved-workspace read denied", queue: '{"followUpBehavior":"steer"}', saved: '[]', denied: "vela.savedWorkspaces", expected: "steer" }
  ])("initializes independent storage keys before hydration effects ($name)", async ({ queue, saved, denied, expected }) => {
    window.localStorage.setItem("messageQueue", queue);
    window.localStorage.setItem("vela.savedWorkspaces", saved);
    const trace: string[] = [];
    const original = window.localStorage.getItem.bind(window.localStorage);
    vi.spyOn(window.localStorage, "getItem").mockImplementation((key) => {
      trace.push(`read:${key}`);
      if (key === denied) throw new DOMException("denied", "SecurityError");
      return original(key);
    });
    await open({ savedWorkspaces: async () => { trace.push("hydrate:saved"); return { workspaces: [], initialized: true }; } });
    expect(store.followUpBehavior).toBe(expected);
    expect(trace.indexOf("read:messageQueue")).toBeLessThan(trace.indexOf("hydrate:saved"));
    expect(trace.indexOf("read:vela.savedWorkspaces")).toBeLessThan(trace.indexOf("hydrate:saved"));
  });

});
