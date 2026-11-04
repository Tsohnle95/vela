import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { SessionInfo, SessionUsage, TranscriptItem } from "@shared/types";
import { useSessionProgress } from "./use-session-progress";
const panel: SessionInfo = { id: "one", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
const usage: SessionUsage = { cost: 1, tokens: { input: 20000, output: 1, reasoning: 0, cache: { read: 0, write: 0 } } };
let progress: ReturnType<typeof useSessionProgress>;
// Compile-only assertions preserve borrowed records and their nested metadata.
function rejectProgressMutation(owner: ReturnType<typeof useSessionProgress>): void {
  // @ts-expect-error Borrowed todo records cannot be replaced.
  owner.todosByWorkspace.one = [];
  // @ts-expect-error Borrowed todo arrays cannot grow.
  owner.todosByWorkspace.one.push(owner.todosByWorkspace.one[0]);
  // @ts-expect-error Borrowed todo entries cannot change.
  owner.todosByWorkspace.one[0].content = "changed";
  // @ts-expect-error Borrowed usage records cannot be replaced.
  owner.usageBySession.one = usage;
  // @ts-expect-error Borrowed usage entries cannot change.
  owner.usageBySession.one.cost = 2;
  // @ts-expect-error Borrowed token counts cannot change.
  owner.usageBySession.one.tokens.input = 2;
  // @ts-expect-error Borrowed nested cache counts cannot change.
  owner.usageBySession.one.tokens.cache.read = 2;
  // @ts-expect-error Borrowed baseline records cannot change.
  owner.compactionBaselineBySession.one = 2;
  // @ts-expect-error Borrowed provider arrays cannot grow.
  owner.providerUsage.push(owner.providerUsage[0]);
  // @ts-expect-error Borrowed provider entries cannot change.
  owner.providerUsage[0].status = "stale";
  // @ts-expect-error Borrowed provider snapshots cannot change.
  owner.providerUsage[0].snapshot!.updatedAt = 2;
  // @ts-expect-error Borrowed provider window arrays cannot grow.
  owner.providerUsage[0].snapshot!.windows.push(owner.providerUsage[0].snapshot!.windows[0]);
  // @ts-expect-error Borrowed nested provider windows cannot change.
  owner.providerUsage[0].snapshot!.windows[0].usedPercent = 2;
  // @ts-expect-error Borrowed nested credits cannot change.
  owner.providerUsage[0].snapshot!.credits!.balance = "changed";
  // @ts-expect-error Borrowed nested provider errors cannot change.
  owner.providerUsage[0].error!.message = "changed";
}
void rejectProgressMutation;
let root: Root; let container: HTMLDivElement;
beforeEach(() => { vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true); window.localStorage.clear(); container = document.createElement("div"); document.body.append(container); root = createRoot(container); });
afterEach(() => { act(() => root.unmount()); container.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
function mount(trace: string[] = []) {
  const protectedSessionIDs = vi.fn(() => { trace.push("protected"); return new Set([panel.id]); });
  function Probe() { progress = useSessionProgress({ panelForSession: () => panel, protectedSessionIDs, notify: vi.fn() }); return null; }
  act(() => root.render(<Probe />)); return { protectedSessionIDs };
}
it("reads a usage receipt only inside its deferred updater before protected IDs", () => {
  const trace: string[] = []; mount(trace); let phase = "before";
  act(() => {
    progress.setTodosFor(panel.workspace.id, []);
    progress.recordUsage(panel.id, () => { trace.push(phase); return usage; });
    phase = "after call";
  });
  expect(trace).toEqual(["after call", "protected"]); expect(progress.usageBySession[panel.id]).toBe(usage);
});
it("keeps last-render usage for same-batch compaction snapshots and removes only the baseline on close", () => {
  mount(); const todos = [{ id: "todo", content: "todo", status: "pending" as const, priority: "low" as const }];
  act(() => { progress.recordUsage(panel.id, () => usage); progress.setTodosFor(panel.workspace.id, todos); });
  act(() => { progress.recordUsage(panel.id, () => ({ ...usage, tokens: { ...usage.tokens, input: 90000 } })); progress.snapshotCompactionBaseline(panel.id); });
  expect(progress.compactionBaselineBySession[panel.id]).toBe(20000);
  const usageBefore = progress.usageBySession; const todosBefore = progress.todosByWorkspace;
  act(() => progress.removeClosedBaseline(panel.id)); expect(progress.compactionBaselineBySession[panel.id]).toBeUndefined();
  expect(progress.usageBySession).toBe(usageBefore); expect(progress.todosByWorkspace).toBe(todosBefore);
  const baseline = progress.compactionBaselineBySession; act(() => progress.removeClosedBaseline("missing")); expect(progress.compactionBaselineBySession).toBe(baseline);
});
it("keeps materialized transcript scanning before the existing-baseline guard", () => {
  window.localStorage.setItem("compactionBaseline", JSON.stringify({ [panel.id]: 7 })); mount(); const trace: string[] = [];
  const transcript = [{ get kind() { trace.push("scan"); return "compaction"; } }] as TranscriptItem[];
  const guardedUsage = { cost: 1, get tokens() { trace.push("tokens"); return usage.tokens; } };
  act(() => progress.inferMaterializedBaseline(panel.id, transcript, guardedUsage)); expect(trace).toEqual(["scan"]);
  trace.length = 0;
  act(() => progress.inferRenderedBaselines([panel], { [panel.id]: transcript }, { [panel.id]: guardedUsage }, progress.compactionBaselineBySession));
  expect(trace).toEqual([]);
});
it("requires input strictly above 10000 and subtracts the original window once", () => {
  mount(); const transcript: TranscriptItem[] = [{ kind: "compaction", id: "c", status: "completed", reason: "manual", summary: "" }];
  act(() => progress.inferMaterializedBaseline(panel.id, transcript, { ...usage, tokens: { ...usage.tokens, input: 10000 } }));
  expect(progress.compactionBaselineBySession[panel.id]).toBeUndefined();
  act(() => progress.inferMaterializedBaseline(panel.id, transcript, { ...usage, tokens: { ...usage.tokens, input: 10001 } }));
  expect(progress.compactionBaselineBySession[panel.id]).toBe(1);
});
it("propagates todo metadata serialization failure without changing the todo record", () => {
  mount(); const circular: unknown[] = []; circular.push(circular);
  const transcript: TranscriptItem[] = [{ kind: "assistant", id: "a", messageID: "a", completed: false, parts: [{ kind: "tool", id: "tool", tool: { id: "tool", title: "todowrite", detail: "", status: "running", metadata: { todos: circular } } }] }];
  const before = progress.todosByWorkspace;
  expect(() => progress.applyBusyTodoFallback([panel], { [panel.id]: true }, { [panel.id]: transcript })).toThrow(TypeError);
  expect(progress.todosByWorkspace).toBe(before);
});
