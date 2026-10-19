import { act, Component, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, PendingFormRequest, PendingPermissionRequest, SessionInfo } from "@shared/types";
import type { VelaApi } from "../../preload";
import { StoreProvider, useStore } from "./store";
let store: ReturnType<typeof useStore>;
let receive: (message: BackendMessage) => void;
const panel: SessionInfo = { id: "session-1", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
const form: PendingFormRequest = { id: "form", sessionID: "external", title: "Question", fields: [{ key: "answer", type: "string" }] };
function Probe() { store = useStore(); return null; }
function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}
function api(overrides: Partial<VelaApi> = {}): VelaApi {
  return {
    platform: "darwin", onMessage: (handler) => { receive = handler; return () => {}; },
    health: async () => true, takePendingPaths: async () => [], state: async () => null,
    activeSessions: async () => [], sessions: async () => [], models: async () => [],
    modelDefault: async () => null, sessionSelection: async () => null, agents: async () => [],
    savedWorkspaces: async () => ({ workspaces: [], initialized: true }),
    openSession: async () => panel,
    openSessionById: async () => ({ session: panel, transcript: [], todos: [], usage: null }),
    closeSession: async () => {}, recoveryRecords: async () => [], listPermissions: async () => [],
    formsList: async () => [], formReply: async () => {}, formCancel: async () => {},
    inboxList: async () => [], permissionReply: async () => {}, ...overrides
  } as VelaApi;
}
function event(type: string, data: Record<string, unknown>, ownerIDs?: string[]) {
  receive({ kind: "event", type, data: { id: `event-${type}`, type, created: 1, ...(ownerIDs ? { velaSessionIDs: ownerIDs } : {}), data } });
}
class Boundary extends Component<{ children: ReactNode; failed: (error: Error) => void }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch(error: Error) { this.props.failed(error); }
  render() { return this.state.failed ? null : this.props.children; }
}

describe("Store request timing and authority", () => {
  let root: Root; let container: HTMLDivElement;
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true); window.localStorage.clear();
    container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  });
  afterEach(async () => { await act(async () => root.unmount()); container.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
  async function open(overrides: Partial<VelaApi> = {}) {
    window.vela = api(overrides);
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession(panel.directory));
  }
  it.each(["approve", "ask", "invalid"])("reads approval before other persisted records (%s)", async (preference) => {
    window.localStorage.setItem("approvalMode", preference);
    const trace: string[] = []; const read = window.localStorage.getItem.bind(window.localStorage);
    vi.spyOn(window.localStorage, "getItem").mockImplementation((key) => { trace.push(key); return read(key); });
    await open();
    expect(store.approvalMode).toBe(preference === "approve" ? "approve" : "ask");
    expect(trace.indexOf("approvalMode")).toBeLessThan(trace.indexOf("compactionBaseline"));
    expect(trace.indexOf("approvalMode")).toBeLessThan(trace.indexOf("messageQueue"));
  });
  it("leaves denied approval initialization uncaught and prevents later storage reads", async () => {
    const trace: string[] = []; const failed = vi.fn();
    vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(window.localStorage, "getItem").mockImplementation((key) => { trace.push(key); if (key === "approvalMode") throw new Error("approval denied"); return null; });
    window.vela = api();
    await act(async () => root.render(<Boundary failed={failed}><StoreProvider><Probe /></StoreProvider></Boundary>));
    expect(failed).toHaveBeenCalledWith(expect.objectContaining({ message: "approval denied" }));
    expect(trace).toContain("approvalMode");
    expect(trace).not.toContain("compactionBaseline"); expect(trace).not.toContain("messageQueue");
  });
  it("retains rendered approval for events while batched toggles await their updaters", async () => {
    const permissionReply = vi.fn(async () => {}); await open({ permissionReply });
    const writes = vi.spyOn(window.localStorage, "setItem");
    act(() => {
      store.toggleApprovalMode(); store.toggleApprovalMode();
      event("permission.asked", { id: "same-batch", sessionID: panel.id, action: "read", resources: [] });
    });
    expect(permissionReply).not.toHaveBeenCalled();
    expect(store.approvalMode).toBe("ask");
    expect(writes.mock.calls.filter(([key]) => key === "approvalMode").map(([, value]) => value)).toEqual(["approve", "ask"]);
  });
  it("retains a late form refresh after close and exposes it on reopen", async () => {
    const late = deferred<PendingFormRequest[]>(); const next = deferred<PendingFormRequest[]>();
    const formsList = vi.fn().mockImplementationOnce(() => late.promise).mockImplementation(() => next.promise);
    await open({ formsList }); await act(async () => store.closePanel(panel.id));
    await act(async () => { late.resolve([form]); await late.promise; });
    await act(async () => store.reopenSession(panel.id));
    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([form]);
  });
  it("optimistically removes a form then starts failure refresh before surfacing its error", async () => {
    const reply = deferred<void>(); const trace: string[] = [];
    const formsList = vi.fn(async () => { trace.push("refresh"); return [form]; });
    await open({ formsList, formReply: () => reply.promise }); trace.length = 0;
    let sending!: Promise<void>;
    act(() => { sending = store.submitForm(panel.workspace, form.id, { answer: "yes" }, form.sessionID); });
    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([]);
    await act(async () => { reply.reject(new Error("reply failed")); await sending; });
    expect(trace).toEqual(["refresh"]); expect(store.toasts.at(-1)?.text).toBe("reply failed");
    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([form]);
  });
  it("suppresses a failed form reply refresh and toast after its panel closes", async () => {
    const reply = deferred<void>(); const formsList = vi.fn(async () => [form]);
    await open({ formsList, formReply: () => reply.promise });
    let sending!: Promise<void>;
    act(() => { sending = store.submitForm(panel.workspace, form.id, {}, form.sessionID); });
    await act(async () => store.closePanel(panel.id));
    await act(async () => { reply.reject(new Error("closed form failure")); await sending; });
    expect(formsList).toHaveBeenCalledTimes(1); expect(store.toasts.some((t) => t.text.includes("closed form failure"))).toBe(false);
  });
  it("dismisses with the true form session and catches cancellation failure silently", async () => {
    const formCancel = vi.fn(async () => { throw new Error("silent cancellation"); });
    await open({ formsList: async () => [form], formCancel });
    act(() => store.dismissForm({ ...panel.workspace, generation: 0 }, form.id, form.sessionID));
    expect(formCancel).not.toHaveBeenCalled(); expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([form]);
    await act(async () => store.dismissForm(panel.workspace, form.id, form.sessionID));
    expect(formCancel).toHaveBeenCalledWith(panel.workspace, form.id, form.sessionID);
    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([]);
    expect(store.toasts.some((t) => t.text.includes("silent cancellation"))).toBe(false);
  });
  it("captures the default permission target before await and globally toasts late failures", async () => {
    const reply = deferred<void>(); const permissionReply = vi.fn(() => reply.promise);
    await open({ permissionReply });
    let sending!: Promise<void>; act(() => { sending = store.replyPermission("request", "once"); });
    expect(permissionReply).toHaveBeenCalledWith(panel.workspace, "request", "once", panel.id);
    await act(async () => store.closePanel(panel.id));
    await act(async () => { reply.reject(new Error("late permission failure")); await sending; });
    expect(store.toasts.at(-1)?.text).toBe("late permission failure");
  });
  it("applies a successful captured permission list after the panel closes", async () => {
    const pending = deferred<PendingPermissionRequest[]>();
    const listPermissions = vi.fn(async (): Promise<PendingPermissionRequest[]> => []);
    await open({ listPermissions });
    listPermissions.mockImplementation(() => pending.promise);
    act(() => event("server.connected", {}));
    await act(async () => store.closePanel(panel.id));
    const readID = vi.fn(() => "late-list");
    await act(async () => { pending.resolve([{ get id() { return readID(); }, sessionID: panel.id, action: "read", resources: [] }]); await pending.promise; });
    expect(readID).toHaveBeenCalled();
    expect(store.panels).toEqual([]);
  });
});
