import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, WorkspaceIdentity } from "@shared/types";
import type { VelaApi } from "../../preload";
import { useExplorerMutations } from "./use-explorer-mutations";

const first: SessionInfo = { id: "first", directory: "/first", workspace: { id: "first-workspace", generation: 1 } };
const second: SessionInfo = { id: "second", directory: "/second", workspace: { id: "second-workspace", generation: 2 } };
let explorer: ReturnType<typeof useExplorerMutations>;
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

describe("Explorer mutation authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  let focused: SessionInfo | null;
  let attached: SessionInfo[];
  const notify = vi.fn();
  const closeMenu = vi.fn();
  const readFocusedSession = vi.fn(() => focused);
  const panelFor = (workspace: WorkspaceIdentity) => attached.find((panel) => panel.workspace.id === workspace.id && panel.workspace.generation === workspace.generation) ?? null;
  const files = {
    expandParent: vi.fn(), hidePath: vi.fn(), restoreDetachedPath: vi.fn(),
    refreshTree: vi.fn(async (_dirs: string[], _workspace?: WorkspaceIdentity) => {}),
    isHiddenPath: vi.fn((_workspace: WorkspaceIdentity, _path: string) => false),
    unhidePath: vi.fn(), recordCreatedEntry: vi.fn(), renameObservedPrefix: vi.fn(),
    moveObservedPrefix: vi.fn(), moveObservedPaths: vi.fn()
  };
  const remap = (path: string) => `dest/${path}`;
  const editor = {
    cancelPrefix: vi.fn(), addCreatedFile: vi.fn(), removePathTabs: vi.fn(), removePathsTabs: vi.fn(),
    renameTabs: vi.fn(), movePathTabs: vi.fn(), movePathsTabs: vi.fn(() => remap),
    readRenderedActivePath: vi.fn((_workspace: WorkspaceIdentity) => "old/file")
  };
  function Probe() {
    explorer = useExplorerMutations({ readFocusedSession, panelFor, notify, closeMenu }, files, editor);
    return null;
  }
  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.clearAllMocks();
    focused = first;
    attached = [first, second];
    files.isHiddenPath.mockReturnValue(false);
    window.vela = {
      createFile: async () => {}, createDir: async () => {}, renamePath: async () => {},
      deletePath: async () => {}, movePath: async () => {}, detachPath: async () => {},
      importExternal: async () => [], revealInFileManager: async () => {}
    } as unknown as VelaApi;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<Probe />));
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("does not eagerly read host callbacks and keeps cancellation stable across intents", () => {
    expect(readFocusedSession).not.toHaveBeenCalled();
    expect(closeMenu).not.toHaveBeenCalled();
    const cancel = explorer.cancelPending;
    act(() => explorer.startCreate("parent", "file"));
    expect(explorer.pendingCreate).toEqual({ parent: "parent", kind: "file" });
    expect(explorer.cancelPending).toBe(cancel);
    expect(files.expandParent).toHaveBeenCalledWith(first.workspace, "parent");
    act(() => explorer.startRename("old/file"));
    expect(explorer.pendingCreate).toBeNull();
    expect(explorer.pendingRename).toEqual({ path: "old/file" });
    closeMenu.mockClear();
    act(() => cancel());
    expect(explorer.pendingRename).toBeNull();
    expect(closeMenu).not.toHaveBeenCalled();
  });

  it("commits last-render intent, rejects invalid names, and retains failed creation intent", async () => {
    const createFile = vi.fn(async () => { throw new Error("creation failed"); });
    window.vela.createFile = createFile;
    await act(async () => {
      explorer.startCreate("", "file");
      await explorer.commitName("same-batch");
    });
    expect(createFile).not.toHaveBeenCalled();
    await act(async () => explorer.commitName("../invalid"));
    expect(notify).toHaveBeenLastCalledWith("Invalid name", "error");
    expect(explorer.pendingCreate).not.toBeNull();
    await act(async () => explorer.commitName("file"));
    expect(notify).toHaveBeenLastCalledWith("creation failed", "error");
    expect(explorer.pendingCreate).not.toBeNull();
    expect(editor.addCreatedFile).not.toHaveBeenCalled();
    await act(async () => explorer.commitName("  "));
    expect(explorer.pendingCreate).toBeNull();
  });

  it("samples rendered active selection only after rename completion and before observed remap", async () => {
    const pending = deferred<void>();
    window.vela.renamePath = () => pending.promise;
    act(() => explorer.startRename("old/file"));
    let completion!: Promise<void>;
    act(() => { completion = explorer.commitName("new"); });
    expect(editor.cancelPrefix).toHaveBeenCalledWith(first.workspace, "old/file");
    expect(editor.readRenderedActivePath).not.toHaveBeenCalled();
    await act(async () => { pending.resolve(); await completion; });
    expect(editor.renameTabs).toHaveBeenCalledWith(first.workspace, "old/file", "old/new", "new", "old/file");
    expect(editor.readRenderedActivePath.mock.invocationCallOrder[0]).toBeLessThan(editor.renameTabs.mock.invocationCallOrder[0]);
    expect(editor.renameTabs.mock.invocationCallOrder[0]).toBeLessThan(files.renameObservedPrefix.mock.invocationCallOrder[0]);
    expect(files.refreshTree).toHaveBeenCalledWith(["old"]);
  });

  it("keeps bulk moves serial and commits only successful paths after partial failures", async () => {
    const firstMove = deferred<void>();
    const movePath = vi.fn(async (_workspace: WorkspaceIdentity, path: string) => {
      if (path === "a") return firstMove.promise;
      if (path === "c") throw new Error("blocked");
    });
    window.vela.movePath = movePath;
    let completion!: Promise<void>;
    act(() => { completion = explorer.moveEntries(["a", "b", "c"], "dest"); });
    expect(movePath.mock.calls.map(([, path]) => path)).toEqual(["a"]);
    expect(editor.cancelPrefix).toHaveBeenCalledWith(first.workspace, "a");
    await act(async () => { firstMove.resolve(); await completion; });
    expect(movePath.mock.calls.map(([, path]) => path)).toEqual(["a", "b", "c"]);
    expect(editor.movePathsTabs).toHaveBeenCalledWith(first.workspace, [
      { path: "a", newPath: "dest/a", name: "a" }, { path: "b", newPath: "dest/b", name: "b" }
    ]);
    expect(files.moveObservedPaths).toHaveBeenCalledWith(first.workspace, remap);
    expect(notify).toHaveBeenLastCalledWith("blocked", "error");
    expect(files.refreshTree).toHaveBeenCalledWith(["", "", "dest"], first.workspace);
  });

  it("continues bulk IPC after a workspace becomes stale while suppressing final projections and errors", async () => {
    const pending = deferred<void>();
    const deletePath = vi.fn(async (_workspace: WorkspaceIdentity, path: string) => {
      if (path === "a") return pending.promise;
      throw new Error("stale failure");
    });
    window.vela.deletePath = deletePath;
    let completion!: Promise<void>;
    act(() => { completion = explorer.deleteEntries(["a", "b"]); });
    attached = [second];
    focused = second;
    await act(async () => { pending.resolve(); await completion; });
    expect(deletePath.mock.calls.map(([, path]) => path)).toEqual(["a", "b"]);
    expect(editor.removePathsTabs).not.toHaveBeenCalled();
    expect(files.refreshTree).not.toHaveBeenCalled();
    expect(notify).not.toHaveBeenCalled();
  });

  it("keeps detach restoration on original workspace and leaves refresh focus resolution deferred", async () => {
    const pending = deferred<void>();
    window.vela.detachPath = () => pending.promise;
    vi.spyOn(window, "confirm").mockReturnValue(true);
    act(() => explorer.removeFromWorkspace("parent/file"));
    expect(files.hidePath).toHaveBeenCalledWith(first.workspace, "parent/file");
    focused = second;
    await act(async () => { pending.resolve(); await pending.promise; });
    expect(files.restoreDetachedPath).toHaveBeenCalledWith(first.workspace, "parent/file");
    expect(files.refreshTree).toHaveBeenCalledWith(["parent"]);
  });

  it("retains captured import visibility queries with focused unhide/default refresh", async () => {
    const pending = deferred<Awaited<ReturnType<VelaApi["importExternal"]>>>();
    window.vela.importExternal = () => pending.promise;
    files.isHiddenPath.mockReturnValue(true);
    let completion!: Promise<void>;
    act(() => { completion = explorer.importPaths("dest", ["/outside"]); });
    focused = second;
    await act(async () => {
      pending.resolve([{ imported: false, rel: "hidden", name: "hidden", reason: "already exists" }]);
      await completion;
    });
    expect(files.isHiddenPath).toHaveBeenCalledWith(first.workspace, "hidden");
    expect(files.unhidePath).toHaveBeenCalledWith("hidden");
    expect(files.refreshTree).toHaveBeenCalledWith(["dest"]);
    expect(notify).not.toHaveBeenCalled();
  });

  it("preserves stale completion and single-operation cancellation ordering", async () => {
    const pending = deferred<void>();
    const deletePath = vi.fn(() => pending.promise);
    window.vela.deletePath = deletePath;
    let completion!: Promise<void>;
    act(() => { completion = explorer.deleteEntry("file"); });
    expect(editor.cancelPrefix.mock.invocationCallOrder[0]).toBeLessThan(deletePath.mock.invocationCallOrder[0]);
    attached = [second];
    await act(async () => { pending.reject(new Error("stale")); await completion; });
    expect(editor.removePathTabs).not.toHaveBeenCalled();
    expect(notify).not.toHaveBeenCalled();
  });
});
