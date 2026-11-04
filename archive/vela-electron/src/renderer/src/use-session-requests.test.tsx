import { act, Component, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { PendingPermissionRequest, SessionInfo } from "@shared/types";
import { useSessionRequests } from "./use-session-requests";
const first: SessionInfo = { id: "one", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
const second: SessionInfo = { id: "two", directory: "/two", workspace: { id: "22222222-2222-4222-8222-222222222222", generation: 2 } };
let requests: ReturnType<typeof useSessionRequests>;
// These compile-only errors protect the borrowed forms without runtime mutation.
function rejectFormsMutation(owner: ReturnType<typeof useSessionRequests>): void {
  // @ts-expect-error Borrowed form session records cannot be replaced.
  owner.formsBySession.one = [];
  // @ts-expect-error Borrowed form arrays cannot grow.
  owner.formsBySession.one.push(owner.formsBySession.one[0]);
  // @ts-expect-error Borrowed form entries cannot change.
  owner.formsBySession.one[0].title = "changed";
  // @ts-expect-error Borrowed field arrays cannot grow.
  owner.formsBySession.one[0].fields.push(owner.formsBySession.one[0].fields[0]);
  // @ts-expect-error Borrowed fields cannot change.
  owner.formsBySession.one[0].fields[0].key = "changed";
  // @ts-expect-error Borrowed option arrays cannot grow.
  owner.formsBySession.one[0].fields[0].options!.push({ value: "x", label: "x" });
  // @ts-expect-error Borrowed nested options cannot change.
  owner.formsBySession.one[0].fields[0].options![0].value = "changed";
}
void rejectFormsMutation;
function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}
class Boundary extends Component<{ children: ReactNode; failed: (error: Error) => void }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch(error: Error) { this.props.failed(error); }
  render() { return this.state.failed ? null : this.props.children; }
}
let root: Root; let container: HTMLDivElement;
beforeEach(() => { vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true); window.localStorage.clear(); container = document.createElement("div"); document.body.append(container); root = createRoot(container); });
afterEach(() => { act(() => root.unmount()); container.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
function mount(panels: SessionInfo[] = [first], failed = vi.fn()) {
  let current = panels; let focused: SessionInfo | null = panels[0] ?? null;
  const notify = vi.fn(); const resolvePermission = vi.fn(); const reconcilePermissionRows = vi.fn();
  const host = { panelFor: (workspace: SessionInfo["workspace"]) => current.find((p) => p.workspace.id === workspace.id && p.workspace.generation === workspace.generation) ?? null,
    panelForSession: (id: string) => current.find((p) => p.id === id) ?? null, readPanels: () => current, readFocusedSession: () => focused, notify };
  function Probe() { requests = useSessionRequests(host, { resolvePermission, reconcilePermissionRows }); return null; }
  act(() => root.render(<Boundary failed={failed}><Probe /></Boundary>));
  return { notify, resolvePermission, reconcilePermissionRows, detach: () => { current = []; focused = null; } };
}
it("initializes without invoking lazy host readers", () => {
  const read = vi.fn(() => { throw new Error("host invoked during initialization"); });
  function Probe() { requests = useSessionRequests({ panelFor: read, panelForSession: read, readPanels: read, readFocusedSession: read, notify: read }, { resolvePermission: read, reconcilePermissionRows: read }); return null; }
  act(() => root.render(<Probe />)); expect(read).not.toHaveBeenCalled(); expect(requests.approvalMode).toBe("ask");
});
it("resolves a successful permission reply for its captured panel after detachment", async () => {
  const reply = deferred<void>(); const permissionReply = vi.fn(() => reply.promise);
  window.vela = { permissionReply } as unknown as typeof window.vela;
  const { detach, resolvePermission } = mount();
  let sending!: Promise<void>; act(() => { sending = requests.replyPermission("request", "always"); }); detach();
  await act(async () => { reply.resolve(); await sending; });
  expect(permissionReply).toHaveBeenCalledWith(first.workspace, "request", "always", first.id);
  expect(resolvePermission).toHaveBeenCalledWith(first.id, "request", "always");
});
it("fetches unique workspaces in parallel and reconciles captured panels in original order after close", async () => {
  const sameWorkspace = { ...first, id: "third" };
  const firstRead = deferred<PendingPermissionRequest[]>(); const secondRead = deferred<PendingPermissionRequest[]>();
  const listPermissions = vi.fn((workspace: SessionInfo["workspace"]) => workspace.id === first.workspace.id ? firstRead.promise : secondRead.promise);
  window.vela = { listPermissions } as unknown as typeof window.vela;
  const { detach, reconcilePermissionRows } = mount([first, second, sameWorkspace]);
  let refresh!: Promise<void>; act(() => { refresh = requests.reconcilePermissions(); });
  expect(listPermissions).toHaveBeenCalledTimes(2); detach();
  await act(async () => { secondRead.reject(new Error("failed workspace")); firstRead.resolve([{ id: "live", sessionID: first.id, action: "read", resources: [] }]); await refresh; });
  expect(reconcilePermissionRows.mock.calls).toEqual([[first.id, [{ id: "live", sessionID: first.id, action: "read", resources: [] }]], [sameWorkspace.id, []]]);
});
it("retains terminal no-op identity while optimistic missing dismissal clones", () => {
  window.vela = { formCancel: async () => {} } as unknown as typeof window.vela;
  mount(); const before = requests.formsBySession;
  act(() => requests.removeTerminalForms([first.id], "missing")); expect(requests.formsBySession).toBe(before);
  act(() => requests.dismissForm(first.workspace, "missing")); expect(requests.formsBySession).not.toBe(before);
});
it("keeps normalized form identity and places repeated receipts last for all owner IDs", () => {
  mount(); const form = { id: "a", sessionID: "external", title: "A", fields: [] };
  act(() => { requests.receiveForms([first.id, second.id], form); requests.receiveForms([first.id], { ...form, id: "b" }); requests.receiveForms([first.id, second.id], form); });
  expect(requests.formsBySession[first.id].map((f) => f.id)).toEqual(["b", "a"]);
  expect(requests.formsBySession[first.id][1]).toBe(form); expect(requests.formsBySession[second.id][0]).toBe(form);
});
it("leaves approval persistence failure uncaught inside the toggle updater", () => {
  const failed = vi.fn(); vi.spyOn(console, "error").mockImplementation(() => {}); mount([first], failed);
  vi.spyOn(window.localStorage, "setItem").mockImplementation(() => { throw new Error("toggle denied"); });
  act(() => requests.toggleApprovalMode());
  expect(failed).toHaveBeenCalledWith(expect.objectContaining({ message: "toggle denied" }));
});
