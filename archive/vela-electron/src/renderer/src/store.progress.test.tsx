import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, ProviderUsageResult, SessionInfo, SessionUsage, TranscriptItem, TodoItem } from "@shared/types";
import type { VelaApi } from "../../preload";
import { StoreProvider, useStore } from "./store";
let store: ReturnType<typeof useStore>; let receive: (message: BackendMessage) => void;
const panel: SessionInfo = { id: "session-1", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
const usage = (input: number): SessionUsage => ({ cost: 1, tokens: { input, output: 1, reasoning: 0, cache: { read: 0, write: 0 } } });
function Probe() { store = useStore(); return null; }
function deferred<T>() { let resolve!: (value: T) => void; let reject!: (error: Error) => void; const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; }); return { promise, resolve, reject }; }
function api(overrides: Partial<VelaApi> = {}): VelaApi {
  return { platform: "darwin", onMessage: (handler) => { receive = handler; return () => {}; },
    health: async () => true, takePendingPaths: async () => [], state: async () => null,
    activeSessions: async () => [], sessions: async () => [], models: async () => [], modelDefault: async () => null,
    sessionSelection: async () => null, agents: async () => [], savedWorkspaces: async () => ({ workspaces: [], initialized: true }),
    openSession: async () => panel, openSessionById: async () => ({ session: panel, transcript: [], todos: [], usage: usage(20000) }),
    closeSession: async () => {}, recoveryRecords: async () => [], listPermissions: async () => [], formsList: async () => [],
    inboxList: async () => [], sessionUsage: async () => null, ...overrides } as VelaApi;
}
function event(type: string, data: Record<string, unknown>) { receive({ kind: "event", type, data: { id: `event-${type}`, type, created: 1, data } }); }
const provider = [{ provider: "openai", displayName: "OpenAI", status: "ok", snapshot: null }] satisfies ProviderUsageResult[];
describe("Store progress persistence and concurrency", () => {
  let root: Root; let container: HTMLDivElement;
  beforeEach(() => { vi.useFakeTimers(); vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true); window.localStorage.clear(); container = document.createElement("div"); document.body.append(container); root = createRoot(container); });
  afterEach(async () => { await act(async () => root.unmount()); if (vi.isFakeTimers()) { vi.clearAllTimers(); vi.useRealTimers(); } container.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
  async function mount(overrides: Partial<VelaApi> = {}, open = false) { window.vela = api(overrides); await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>)); if (open) await act(async () => store.addModelPanel(panel.directory)); }
  it("suppresses a stale provider failure and finally while the newest refresh is pending", async () => {
    const older = deferred<ProviderUsageResult[]>(); const newer = deferred<ProviderUsageResult[]>();
    const providerUsage = vi.fn().mockImplementationOnce(() => older.promise).mockImplementationOnce(() => newer.promise);
    await mount({ providerUsage }); let first!: Promise<void>; let second!: Promise<void>;
    act(() => { first = store.refreshProviderUsage(); second = store.refreshProviderUsage(); });
    await act(async () => { older.reject(new Error("stale failure")); await first; });
    expect(store.providerUsageLoading).toBe(true); expect(store.toasts.some((t) => t.text.includes("stale failure"))).toBe(false);
    await act(async () => { newer.resolve(provider); await second; }); expect(store.providerUsage).toEqual(provider); expect(store.providerUsageLoading).toBe(false);
  });
  it("retains old provider results on the current refresh failure and clears loading", async () => {
    const providerUsage = vi.fn().mockResolvedValueOnce(provider).mockRejectedValueOnce("current failure");
    await mount({ providerUsage }); await act(async () => store.refreshProviderUsage()); const before = store.providerUsage;
    await act(async () => store.refreshProviderUsage()); expect(store.providerUsage).toBe(before); expect(store.providerUsageLoading).toBe(false); expect(store.toasts.at(-1)?.text).toBe("current failure");
  });
  it.each(["malformed", "read denied", "write denied"])("catches compaction baseline storage failure (%s)", async (failure) => {
    window.localStorage.setItem("compactionBaseline", failure === "malformed" ? "{broken" : JSON.stringify({ [panel.id]: 7000 }));
    const read = window.localStorage.getItem.bind(window.localStorage); const write = window.localStorage.setItem.bind(window.localStorage);
    vi.spyOn(window.localStorage, "getItem").mockImplementation((key) => { if (key === "compactionBaseline" && failure === "read denied") throw new Error("denied"); return read(key); });
    vi.spyOn(window.localStorage, "setItem").mockImplementation((key, value) => { if (key === "compactionBaseline" && failure === "write denied") throw new Error("denied"); write(key, value); });
    await mount({}, true); expect(store.panelViews[panel.workspace.id].compactionBaseline).toBe(failure === "write denied" ? 7000 : null);
  });
  it("normalizes usage accessors before a missing target guard", async () => {
    await mount(); const trace: string[] = [];
    act(() => event("session.usage.updated", { get tokens() { trace.push("tokens"); return { get cache() { trace.push("cache"); return {}; }, get input() { trace.push("input"); return 1; } }; }, get cost() { trace.push("cost"); return 0; } }));
    expect(trace).toEqual(["tokens", "cache", "cost", "input"]); expect(store.sessionUsage).toBeNull();
  });
  it("snapshots compaction from last-render usage rather than a queued same-batch usage update", async () => {
    vi.useFakeTimers(); await mount({}, true);
    act(() => { event("session.usage.updated", { sessionID: panel.id, tokens: { input: 90000 } }); event("session.compaction.ended", { sessionID: panel.id }); });
    expect(store.sessionUsage?.tokens.input).toBe(90000); expect(store.panelViews[panel.workspace.id].compactionBaseline).toBe(20000);
  });
  it("discards a late usage refresh after close and keeps a refresh failure silent", async () => {
    vi.useFakeTimers(); const pending = deferred<SessionUsage | null>(); const sessionUsage = vi.fn(() => pending.promise);
    await mount({ sessionUsage }, true); act(() => event("session.compaction.failed", { sessionID: panel.id }));
    expect(sessionUsage).toHaveBeenCalledTimes(1); await act(async () => store.closePanel(panel.id));
    await act(async () => { pending.reject(new Error("usage unavailable")); try { await pending.promise; } catch {} });
    expect(store.toasts.some((t) => t.text.includes("usage unavailable"))).toBe(false); expect(store.panels).toEqual([]);
  });
  it("reads reopened usage again inside the deferred receipt instead of capturing its guard value", async () => {
    let reads = 0;
    await mount({ openSessionById: async () => ({ session: panel, transcript: [], todos: [], get usage() { reads += 1; return usage(reads * 20000); } }) }, true);
    expect(reads).toBe(2); expect(store.sessionUsage?.tokens.input).toBe(40000);
  });
  it("rejects a late successful usage refresh after close while retaining the existing usage record", async () => {
    vi.useFakeTimers(); const pending = deferred<SessionUsage | null>();
    await mount({ sessionUsage: () => pending.promise }, true);
    act(() => event("session.compaction.failed", { sessionID: panel.id }));
    await act(async () => store.closePanel(panel.id));
    await act(async () => { pending.resolve(usage(99000)); await pending.promise; });
    window.vela.openSessionById = async () => ({ session: panel, transcript: [], todos: [], usage: null });
    await act(async () => store.reopenSession(panel.id));
    expect(store.sessionUsage?.tokens.input).toBe(20000);
  });
  it.each([
    { name: "metadata preference", title: "Todo Write", metadata: { todos: [{ id: "metadata", content: "metadata", status: "pending", priority: "low" }] }, input: '{"todos":[{"id":"input","content":"input","status":"pending","priority":"low"}]}', ids: ["metadata"] },
    { name: "malformed input", title: "todowrite", metadata: {}, input: "{broken", ids: [] },
    { name: "non-matching tool", title: "read", metadata: {}, input: "{}", ids: ["existing"] }
  ])("uses the latest busy todo tool fallback ($name)", async ({ title, metadata, input, ids }) => {
    const initial: TodoItem[] = [{ id: "existing", content: "existing", status: "pending", priority: "low" }];
    const transcript: TranscriptItem[] = [{ kind: "assistant", id: "assistant", messageID: "assistant", completed: false,
      parts: [{ kind: "tool", id: "tool", tool: { id: "tool", title, detail: "", status: "success", metadata, input } }] }];
    await mount({ openSessionById: async () => ({ session: panel, transcript, todos: initial, usage: null }) }, true);
    expect(store.todos.map((todo) => todo.id)).toEqual(ids);
    if (ids[0] === "metadata") {
      act(() => event("todo.updated", { sessionID: panel.id, todos: [{ id: "external", content: "external", status: "pending", priority: "low" }] }));
      act(() => event("permission.asked", { sessionID: panel.id, id: "pending", action: "read", resources: [] }));
      expect(store.todos.map((todo) => todo.id)).toEqual(["external"]);
    }
  });

});
