import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { useSessionInbox } from "./use-session-inbox";
import type { SessionInfo } from "@shared/types";
const panel: SessionInfo = { id: "s", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
let inbox: ReturnType<typeof useSessionInbox>;
// Compile-only assertions protect borrowed views without mutating runtime values.
function rejectInboxMutation(owner: ReturnType<typeof useSessionInbox>): void {
  // @ts-expect-error Queue behavior belongs to the Inbox owner.
  owner.messageQueue.followUpBehavior = "steer";
  // @ts-expect-error Borrowed queue targets cannot be replaced.
  owner.messageQueue.queuedMessages.session = [];
  // @ts-expect-error Borrowed queue arrays cannot grow.
  owner.messageQueue.queuedMessages.session.push({ id: "x", content: "", createdAt: 1 });
  // @ts-expect-error Borrowed queue entries cannot change.
  owner.messageQueue.queuedMessages.session[0].content = "changed";
  // @ts-expect-error Borrowed attachment arrays cannot grow.
  owner.messageQueue.queuedMessages.session[0].attachments!.push({ path: "changed" });
  // @ts-expect-error Borrowed attachment paths cannot change.
  owner.messageQueue.queuedMessages.session[0].attachments![0].path = "changed";
  // @ts-expect-error Borrowed nested mention metadata cannot change.
  owner.messageQueue.queuedMessages.session[0].attachments![0].mention!.text = "changed";
  // @ts-expect-error Borrowed send configuration cannot change.
  owner.messageQueue.queuedMessages.session[0].sendConfig!.agent = "changed";
  // @ts-expect-error Borrowed Inbox session records cannot be replaced.
  owner.inboxBySession.session = [];
  // @ts-expect-error Borrowed Inbox arrays cannot grow.
  owner.inboxBySession.session.push({ id: "x", text: "", createdAt: 1, attachmentCount: 0 });
  // @ts-expect-error Borrowed Inbox entries cannot change.
  owner.inboxBySession.session[0].text = "changed";
}
void rejectInboxMutation;

let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
afterEach(() => { act(() => root?.unmount()); container?.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
function mount(trace: string[] = []) {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  window.localStorage.clear();
  container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  const append = vi.fn(() => trace.push("status"));
  let attached = true;
  function Probe() {
    inbox = useSessionInbox({ panelFor: (workspace) => attached && workspace.id === panel.workspace.id && workspace.generation === panel.workspace.generation ? panel : null, panelForSession: () => attached ? panel : null, notify: () => trace.push("toast") },
      { appendTranscriptItem: append, appendUniqueTranscriptItem: append }, { currentAgentByWorkspace: {}, currentModelByWorkspace: {} }); return null;
  }
  act(() => root.render(<Probe />));
  return { close: () => { attached = false; }, append };
}
it("retains guarded terminal identity but clones optimistic missing removals", () => {
  window.vela = { inboxCancel: async () => {} } as unknown as typeof window.vela;
  mount(); const before = inbox.inboxBySession;
  act(() => inbox.removeTerminalInbox(panel.id, { inboxID: "missing" }));
  expect(inbox.inboxBySession).toBe(before);
  act(() => inbox.removeQueuedMessage(panel.workspace, "missing"));
  expect(inbox.inboxBySession).not.toBe(before);
});
it("prepares enqueue delivery before updater coercion and moves duplicates to the end", () => {
  mount(); const trace: string[] = [];
  const enqueue = (id: string, text: unknown) => inbox.receiveEnqueuedInbox(panel.id, { inboxID: id, item: { type: "user", payload: { text } } });
  act(() => { enqueue("a", "old"); enqueue("b", "other"); });
  const data = { get inboxID() { trace.push("id"); return "a"; }, get delivery() { trace.push("delivery"); return "queue"; }, get item() { trace.push("item"); return { type: "user", get payload() { trace.push("payload"); return { get files() { trace.push("files"); return []; }, text: { toString() { trace.push("coerce"); return "new"; } } }; } }; } };
  vi.spyOn(Date, "now").mockImplementation(() => { trace.push("clock"); return 50; });
  act(() => inbox.receiveEnqueuedInbox(panel.id, data));
  expect(trace.indexOf("delivery")).toBeLessThan(trace.indexOf("coerce"));
  expect(trace.indexOf("files")).toBeLessThan(trace.indexOf("coerce"));
  expect(trace.indexOf("coerce")).toBeLessThan(trace.indexOf("clock"));
  expect(inbox.inboxBySession[panel.id].map((entry) => entry.id)).toEqual(["b", "a"]);
  expect(inbox.inboxBySession[panel.id][1]).toMatchObject({ text: "new", delivery: "queue", createdAt: 50 });
});
it("starts native failure refresh before status then toast", async () => {
  const trace: string[] = [];
  window.vela = { inboxCancel: async () => {}, inboxSteer: async () => { throw new Error("steer failed"); }, inboxList: async () => { trace.push("refresh"); return []; } } as unknown as typeof window.vela;
  mount(trace);
  await act(async () => inbox.sendQueuedNow(panel.workspace, "native"));
  expect(trace).toEqual(["refresh", "status", "toast"]);
});
it("enqueues a captured retry after detachment without validating the panel", () => {
  window.vela = { inboxCancel: async () => {} } as unknown as typeof window.vela;
  const { close } = mount();
  act(() => inbox.enqueueFailedPrompt(panel, { content: "existing" }));
  const before = inbox.messageQueue;
  act(() => inbox.removeQueuedMessage({ ...panel.workspace, generation: 0 }, "queued-stale"));
  expect(inbox.messageQueue).toBe(before);
  close();
  act(() => { expect(inbox.enqueueFailedPrompt(panel, { content: "retry" })).toBe(true); });
  expect(Object.values(inbox.messageQueue.queuedMessages).flat().at(-1)).toMatchObject({ content: "retry" });
  act(() => { expect(inbox.popQueuedMessage({ ...panel.workspace, generation: 0 }, "missing")).toBeNull(); });
});
