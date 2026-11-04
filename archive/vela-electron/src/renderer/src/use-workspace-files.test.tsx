import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FileUpdate, SessionInfo, TreeEntry } from "@shared/types";
import type { VelaApi } from "../../preload";
import { useWorkspaceFiles } from "./use-workspace-files";

const first: SessionInfo = { id: "first", directory: "/parent", workspace: { id: "first-workspace", generation: 1 } };
const second: SessionInfo = { id: "second", directory: "/parent/child", workspace: { id: "second-workspace", generation: 2 } };
let files: ReturnType<typeof useWorkspaceFiles>;
// Compile-only assertions for the borrowed query; legacy Store maps remain compatible.
function readonlyObservedFile(view: NonNullable<ReturnType<typeof files.readAgentFile>>): void {
  // @ts-expect-error Observed content belongs to the file owner.
  view.content = "mutated";
  if (view.baseline.kind === "known") {
    // @ts-expect-error Known baseline content is borrowed without mutation authority.
    view.baseline.content = "mutated";
    // @ts-expect-error Known baseline existence is borrowed without mutation authority.
    view.baseline.exists = false;
  }
  // @ts-expect-error Baseline replacement belongs to the file owner.
  view.baseline = { kind: "unknown" };
}
void readonlyObservedFile;

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
function update(path: string, deleted = false, panel = first): FileUpdate {
  return { workspace: panel.workspace, sessionID: panel.id, path, deleted,
    baseline: { kind: "unknown" }, content: deleted ? null : "changed" };
}

describe("workspace file authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  let session: SessionInfo | null;
  let panels: SessionInfo[];
  const notify = vi.fn();
  const readFocusedSession = () => session;
  const readPanels = () => panels;
  const panelFor = (workspace: SessionInfo["workspace"]) => panels.find((panel) =>
    panel.workspace.id === workspace.id && panel.workspace.generation === workspace.generation) ?? null;
  function Probe() {
    files = useWorkspaceFiles({ renderSession: session, readFocusedSession, readPanels, panelFor, notify });
    return null;
  }
  const render = () => act(async () => root.render(<Probe />));
  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    session = first;
    panels = [first, second];
    notify.mockReset();
    window.vela = {
      listDir: async () => [], recoveryRecords: async () => [],
      openRecovery: async () => {}, acknowledgeRecovery: async () => {}
    } as unknown as VelaApi;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await render();
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("prefetches once without expansion and applies the shared filter and ordering", async () => {
    const pending = deferred<TreeEntry[]>();
    const list = vi.fn(() => pending.promise);
    window.vela.listDir = list;
    act(() => { files.prefetchDir("src"); files.prefetchDir("src"); });
    expect(list).toHaveBeenCalledOnce();
    expect(files.expanded.size).toBe(0);
    await act(async () => pending.resolve([
      { path: "src/z.ts", type: "file" }, { path: "src/node_modules", type: "directory" },
      { path: "src/a", type: "directory" }, { path: "src/.DS_Store", type: "file" }
    ]));
    expect(files.tree.src.map((entry) => entry.path)).toEqual(["src/a", "src/z.ts"]);
    act(() => files.prefetchDir("src"));
    expect(list).toHaveBeenCalledOnce();
  });

  it("does not replace a concurrent listing and retries silent failures", async () => {
    const pending = deferred<TreeEntry[]>();
    window.vela.listDir = vi.fn(() => pending.promise);
    act(() => files.prefetchDir("src"));
    act(() => files.recordCreatedEntry(first.workspace, "src", "src/current", "file"));
    await act(async () => pending.resolve([{ path: "src/old", type: "file" }]));
    expect(files.tree.src.map((entry) => entry.path)).toEqual(["src/current"]);
    const list = vi.fn(async () => { throw new Error("offline"); });
    window.vela.listDir = list;
    await act(async () => files.prefetchDir("other"));
    await act(async () => files.prefetchDir("other"));
    expect(list).toHaveBeenCalledTimes(2);
    expect(notify).not.toHaveBeenCalled();
  });

  it("deduplicates by generation and discards closed or replaced responses", async () => {
    const old = deferred<TreeEntry[]>();
    const fresh = deferred<TreeEntry[]>();
    const list = vi.fn().mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise);
    window.vela.listDir = list;
    act(() => files.prefetchDir("src"));
    const replacement = { ...first, workspace: { ...first.workspace, generation: 99 } };
    panels = [replacement, second];
    session = replacement;
    await render();
    act(() => files.prefetchDir("src"));
    expect(list).toHaveBeenCalledTimes(2);
    await act(async () => old.resolve([{ path: "src/old", type: "file" }]));
    expect(files.tree.src).toBeUndefined();
    panels = [second];
    await act(async () => fresh.resolve([{ path: "src/fresh", type: "file" }]));
    expect(files.tree.src).toBeUndefined();
  });

  it("ignores responses and captured prefetch actions after unmount", async () => {
    const pending = deferred<TreeEntry[]>();
    const list = vi.fn(() => pending.promise);
    window.vela.listDir = list;
    const prefetch = files.prefetchDir;
    act(() => prefetch("src"));
    await act(async () => root.unmount());
    await act(async () => pending.resolve([{ path: "src/late", type: "file" }]));
    prefetch("other");
    expect(list).toHaveBeenCalledOnce();
    expect(files.tree.src).toBeUndefined();
  });

  it("preserves empty projection identities and queries lazily", async () => {
    expect(files.expanded).toBe(files.hiddenPaths);
    const { agentFiles, tree, expanded, recoveryRecords } = files;
    await render();
    expect(files.agentFiles).toBe(agentFiles);
    expect(files.tree).toBe(tree);
    expect(files.expanded).toBe(expanded);
    expect(files.recoveryRecords).not.toBe(recoveryRecords);
    expect(files.readAgentFile(first.workspace, "missing")).toBeUndefined();
  });

  it("keeps last-render agent reads while applying queued current updates", () => {
    act(() => {
      files.receiveFileUpdate(update("first"));
      expect(files.readAgentFile(first.workspace, "first")).toBeUndefined();
      files.receiveFileUpdate(update("second"));
    });
    expect([...files.agentFiles.keys()]).toEqual(["first", "second"]);
    expect(files.readAgentFile({ ...first.workspace, generation: 99 }, "first")).toBe(files.agentFiles.get("first"));
  });

  it("preserves last-render expansion writes within one batch", () => {
    act(() => { files.expandParent(first.workspace, "a"); files.expandParent(first.workspace, "b"); });
    expect(files.expanded).toEqual(new Set(["b"]));
  });

  it("retains distinct rename, single move and bulk move deletion policies", () => {
    act(() => {
      files.receiveFileUpdate(update("dir/live"));
      files.receiveFileUpdate(update("dir/deleted", true));
      files.receiveFileUpdate(update("outside", true));
    });
    act(() => files.renameObservedPrefix(first.workspace, "dir", "renamed"));
    expect([...files.agentFiles.keys()]).toEqual(["renamed/live", "renamed/deleted", "outside"]);
    act(() => files.moveObservedPrefix(first.workspace, "renamed", "single"));
    expect([...files.agentFiles.keys()]).toEqual(["single/live", "outside"]);
    act(() => {
      files.receiveFileUpdate(update("queued"));
      files.moveObservedPaths(first.workspace, (path) => `bulk/${path}`);
    });
    expect([...files.agentFiles.keys()]).toEqual(["bulk/single/live", "bulk/queued"]);
  });

  it("reads live panels inside queued nested visibility restoration", () => {
    act(() => files.hidePath(first.workspace, "child"));
    // Hide another path so restoration queues behind an existing update rather than eager-evaluating.
    act(() => {
      files.hidePath(first.workspace, "another");
      panels = [];
      files.restoreNestedWorkspaceVisibility(second.directory);
      panels = [first];
    });
    expect(files.hiddenPaths).toEqual(new Set(["another"]));
  });

  it("preserves render-captured hidden queries and invocation-time focused unhide", async () => {
    act(() => files.hidePath(first.workspace, "first-hidden"));
    const captured = files.isHiddenPath;
    act(() => files.restoreDetachedPath(first.workspace, "first-hidden"));
    expect(captured(first.workspace, "first-hidden")).toBe(true);
    expect(files.isHiddenPath(first.workspace, "first-hidden")).toBe(false);
    act(() => files.hidePath(second.workspace, "second-hidden"));
    session = second;
    await render();
    session = first;
    act(() => files.unhidePath("second-hidden"));
    session = second;
    await render();
    expect(files.hiddenPaths.has("second-hidden")).toBe(true);
  });

  it("refreshes unique dirs concurrently including open root and silently preserves failed listings", async () => {
    const rootListing: TreeEntry[] = [{ path: "saved", type: "file" }];
    window.vela.listDir = async () => rootListing;
    await act(async () => files.ensureRootOpen());
    const pending = deferred<TreeEntry[]>();
    const listDir = vi.fn((_workspace, path: string) => path === "" ? Promise.reject(new Error("silent")) : pending.promise);
    window.vela.listDir = listDir;
    let refresh!: Promise<void>;
    act(() => { refresh = files.refreshTree(["a", "a", "b"]); });
    expect(listDir.mock.calls.map(([, path]) => path)).toEqual(["a", "b", ""]);
    await act(async () => { pending.resolve([]); await refresh; });
    expect(files.tree[""]).toEqual(rootListing);
    expect(notify).not.toHaveBeenCalled();
  });

  it("debounces two moved parents per workspace and ignores stale delayed success", async () => {
    vi.useFakeTimers();
    act(() => files.expandParent(first.workspace, "a"));
    act(() => files.expandParent(first.workspace, "b"));
    act(() => files.expandParent(second.workspace, "a"));
    const firstRead = deferred<TreeEntry[]>();
    const listDir = vi.fn((workspace: SessionInfo["workspace"]) => workspace.id === first.workspace.id ? firstRead.promise : Promise.resolve([]));
    window.vela.listDir = listDir;
    act(() => {
      files.scheduleFileRefresh({ ...update("b/file"), movedFrom: "a/file" });
      files.scheduleFileRefresh(update("b/file"));
      files.scheduleFileRefresh(update("a/other", false, second));
    });
    await act(async () => vi.advanceTimersByTimeAsync(49));
    expect(listDir).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTimeAsync(1));
    expect(listDir.mock.calls).toHaveLength(3);
    panels = [second];
    await act(async () => { firstRead.resolve([{ path: "stale", type: "file" }]); await firstRead.promise; });
    expect(files.tree).toEqual({});
  });

  it("does not add timer cleanup on unmount and keeps delayed failures silent", async () => {
    vi.useFakeTimers();
    act(() => files.expandParent(first.workspace, ""));
    const listDir = vi.fn(async () => { throw new Error("silent refresh"); });
    window.vela.listDir = listDir;
    act(() => files.scheduleFileRefresh(update("file")));
    await act(async () => root.unmount());
    await act(async () => vi.advanceTimersByTimeAsync(50));
    expect(listDir).toHaveBeenCalledTimes(1);
    expect(notify).not.toHaveBeenCalled();
  });

  it("retains observed deletion unless known baseline establishes restoration", () => {
    act(() => {
      files.receiveFileUpdate(update("unknown", true));
      files.receiveFileUpdate({ ...update("known", true), baseline: { kind: "known", content: "old" } });
      files.receiveFileUpdate({ ...update("created", true), baseline: { kind: "known", content: "", exists: false } });
    });
    expect([...files.agentFiles.keys()]).toEqual(["unknown", "known"]);
    act(() => files.receiveFileUpdate({ ...update("known"), baseline: { kind: "known", content: "changed" } }));
    expect([...files.agentFiles.keys()]).toEqual(["unknown"]);
  });
  it("orders sparse plain listings stably and preserves deferred malformed-entry errors", async () => {
    const firstTie: TreeEntry = { path: "same", type: "file" };
    const secondTie: TreeEntry = { path: "same", type: "file" };
    const listing: TreeEntry[] = new Array(8);
    listing[1] = firstTie;
    listing[2] = { path: "nested/.git", type: "directory" };
    listing[3] = { path: ".git", type: "file" };
    listing[4] = { path: "nested/.DS_Store", type: "file" };
    listing[5] = secondTie;
    listing[6] = { path: "folder", type: "directory" };
    window.vela.listDir = async () => listing;
    await act(async () => files.toggleDir("sparse"));
    expect(files.tree.sparse).toEqual([listing[6], listing[3], firstTie, secondTie]);
    expect(files.tree.sparse[2]).toBe(firstTie);
    expect(files.tree.sparse[3]).toBe(secondTie);
    expect(0 in listing).toBe(false);
    window.vela.listDir = async () => [];
    await act(async () => files.toggleDir("empty"));
    expect(files.tree.empty).toEqual([]);
    // Filter validation retains the old local e/path access and error text.
    const malformed = [null] as unknown as TreeEntry[];
    let expected = "";
    try { malformed.filter((e) => e.path.split("/").pop() !== ".DS_Store"); }
    catch (error) { expected = (error as Error).message; }
    window.vela.listDir = async () => malformed;
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    await expect((async () => { await act(async () => files.toggleDir("malformed")); })()).rejects.toThrow(expected);
    consoleError.mockRestore();
  });

});
