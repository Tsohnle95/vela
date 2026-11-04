import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FileSidebar } from "./FileSidebar";

const store = {
  session: { id: "session", directory: "/workspace", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } },
  selectFolder: vi.fn(),
  tree: { "": [{ path: "dir", type: "directory" as const }, { path: "a.txt", type: "file" as const }] },
  toggleDir: vi.fn(),
  ensureRootOpen: vi.fn(),
  agentFiles: new Map(),
  openFile: vi.fn(),
  previewFile: vi.fn(),
  expanded: new Set([""]),
  hiddenPaths: new Set<string>(),
  pendingCreate: null,
  commitName: vi.fn(),
  cancelPending: vi.fn(),
  startCreate: vi.fn(),
  startRename: vi.fn(),
  deleteEntry: vi.fn(),
  removeFromWorkspace: vi.fn(),
  restoreRemovedFromWorkspace: vi.fn()
};

const ctxMenuApi = {
  ctxMenu: null as { x: number; y: number; target: { path: string; type: "file" | "directory" } | null } | null,
  openCtxMenu: vi.fn(),
  closeCtxMenu: vi.fn()
};

vi.mock("../store", () => ({
  useStore: () => store,
  useCtxMenu: () => ctxMenuApi
}));

describe("FileSidebar file creation", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    ctxMenuApi.ctxMenu = null;
    vi.clearAllMocks();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("offers file and folder creation on the workspace header", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const actions = container.querySelector<HTMLElement>(".section-trigger.explorer-trigger .tree-row-actions")!;
    expect(actions).toBeTruthy();
    const byTitle = (title: string): HTMLButtonElement =>
      [...actions.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.title === title)!;
    act(() => {
      byTitle("New File").click();
    });
    expect(store.startCreate).toHaveBeenCalledWith("", "file");
    act(() => {
      byTitle("New Folder").click();
    });
    expect(store.startCreate).toHaveBeenCalledWith("", "dir");
  });

  it("offers no creation actions in the explorer header", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    expect(container.querySelector(".section-actions")).toBeNull();
  });

  it("offers file and folder creation on subfolder rows", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const actions = container.querySelector<HTMLElement>(".tree-row.dir:not(.workspace-root) .tree-row-actions")!;
    const buttons = [...actions.querySelectorAll<HTMLButtonElement>("button")];
    const titles = buttons.map((button) => button.title);
    expect(titles).toContain("New File");
    expect(titles).toContain("New Folder");
    act(() => {
      buttons.find((button) => button.title === "New File")!.click();
      buttons.find((button) => button.title === "New Folder")!.click();
    });
    expect(store.startCreate).toHaveBeenCalledWith("dir", "file");
    expect(store.startCreate).toHaveBeenCalledWith("dir", "dir");
  });

  it("offers no creation actions on file rows", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const row = [...container.querySelectorAll<HTMLElement>(".tree-row.file")].find(
      (candidate) => candidate.textContent?.includes("a.txt")
    )!;
    const titles = [...row.querySelectorAll<HTMLButtonElement>("button")].map((button) => button.title);
    expect(titles).not.toContain("New File");
    expect(titles).not.toContain("New Folder");
  });

  it("starts a file from the context menu", () => {
    ctxMenuApi.ctxMenu = { x: 50, y: 60, target: { path: "dir", type: "directory" as const } };
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const item = [...document.body.querySelectorAll<HTMLButtonElement>(".ctx-item")].find(
      (b) => b.textContent === "New File…"
    )!;
    expect(item).toBeTruthy();
    act(() => {
      item.click();
    });
    expect(store.startCreate).toHaveBeenCalledWith("dir", "file");
  });
});
