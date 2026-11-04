import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FileSidebar } from "./FileSidebar";

const store = {
  session: { id: "session", directory: "/workspace", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } },
  panels: [] as Array<{ id: string; directory: string; workspace: { id: string; generation: number } }>,
  focusSession: vi.fn(),
  closePanel: vi.fn(),
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
  revealInFileManager: vi.fn(),
  deleteEntry: vi.fn(),
  removeFromWorkspace: vi.fn(),
  restoreRemovedFromWorkspace: vi.fn()
};

const ctxMenuApi = {
  ctxMenu: null as { x: number; y: number; target: { path: string; type: "file" | "directory" } | null; workspaceSessionID?: string } | null,
  openCtxMenu: vi.fn(),
  closeCtxMenu: vi.fn()
};

vi.mock("../store", () => ({
  useStore: () => store,
  useCtxMenu: () => ctxMenuApi
}));

describe("FileSidebar context menu open/close", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    ctxMenuApi.ctxMenu = null;
    store.panels = [];
    store.session = { id: "session", directory: "/workspace", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
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

  it("opens the menu when More actions is pressed", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const row = container.querySelector<HTMLElement>(".tree-row.dir:not(.workspace-root)")!;
    const button = [...row.querySelectorAll<HTMLButtonElement>(".tree-row-action")].find(
      (b) => b.title === "More actions…"
    )!;
    expect(button).toBeTruthy();
    act(() => {
      button.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 50, clientY: 60 }));
    });
    expect(ctxMenuApi.openCtxMenu).toHaveBeenCalledTimes(1);
    expect(ctxMenuApi.openCtxMenu.mock.calls[0][2]).toEqual({ path: "dir", type: "directory" });
  });

  it("closes the menu when clicking outside", () => {
    ctxMenuApi.ctxMenu = { x: 50, y: 60, target: { path: "a.txt", type: "file" as const } };
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    expect(ctxMenuApi.closeCtxMenu).not.toHaveBeenCalled();
    act(() => {
      document.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
    });
    expect(ctxMenuApi.closeCtxMenu).toHaveBeenCalledTimes(1);
  });

  it("opens the integrated terminal at a folder or a file's parent folder", () => {
    const onOpenTerminal = vi.fn();
    ctxMenuApi.ctxMenu = { x: 50, y: 60, target: { path: "dir", type: "directory" } };
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} onOpenTerminal={onOpenTerminal} />));
    act(() => document.body.querySelector<HTMLButtonElement>(".ctx-item")!.click());
    expect(onOpenTerminal).toHaveBeenCalledWith("dir");

    ctxMenuApi.ctxMenu = { x: 50, y: 60, target: { path: "dir/a.txt", type: "file" } };
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} onOpenTerminal={onOpenTerminal} />));
    act(() => document.body.querySelector<HTMLButtonElement>(".ctx-item")!.click());
    expect(onOpenTerminal).toHaveBeenLastCalledWith("dir");
  });

  it("offers reveal in file manager for a selected explorer item", () => {
    ctxMenuApi.ctxMenu = { x: 50, y: 60, target: { path: "dir/a.txt", type: "file" } };
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const item = [...document.body.querySelectorAll<HTMLButtonElement>(".ctx-item")].find(
      (button) => button.textContent === "Reveal in File Manager"
    )!;
    expect(item).toBeTruthy();
    act(() => item.click());
    expect(store.revealInFileManager).toHaveBeenCalledWith("dir/a.txt");
  });

  it("removes the right-clicked nonfocused workspace root", () => {
    const active = { id: "active", directory: "/workspace", workspace: { id: "active-workspace", generation: 1 } };
    const secondary = { id: "secondary", directory: "/second", workspace: { id: "second-workspace", generation: 2 } };
    store.session = active;
    store.panels = [active, secondary];
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));

    const row = [...container.querySelectorAll<HTMLElement>(".tree-row.workspace-root")]
      .find((item) => item.title === secondary.directory)!;
    act(() => row.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 30, clientY: 40 })));

    expect(ctxMenuApi.openCtxMenu).toHaveBeenCalledWith(30, 40, { path: "", type: "directory" }, secondary.id);
    ctxMenuApi.ctxMenu = {
      x: 30,
      y: 40,
      target: { path: "", type: "directory" },
      workspaceSessionID: secondary.id
    };
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const remove = [...document.body.querySelectorAll<HTMLButtonElement>(".ctx-item")]
      .find((button) => button.textContent === "Remove Workspace")!;
    act(() => remove.click());

    expect(store.closePanel).toHaveBeenCalledWith(secondary.id);
    expect(store.closePanel).not.toHaveBeenCalledWith(active.id);
  });

  it("renders the menu outside the sidebar so ancestors cannot clip it", () => {
    ctxMenuApi.ctxMenu = { x: 50, y: 60, target: { path: "dir", type: "directory" } };
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const menu = document.body.querySelector(".ctx-menu")!;
    expect(menu).toBeTruthy();
    expect(container.contains(menu)).toBe(false);
  });
});
