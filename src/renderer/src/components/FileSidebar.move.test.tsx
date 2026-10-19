import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TreeEntry } from "@shared/types";
import { FileSidebar } from "./FileSidebar";

type MockSession = { id: string; directory: string; workspace: { id: string; generation: number } };

const session: MockSession = {
  id: "session",
  directory: "/workspace",
  workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 }
};

const initialTree: Record<string, TreeEntry[]> = {
  "": [
    { path: "alpha", type: "directory" as const },
    { path: "beta", type: "directory" as const },
    { path: "note.txt", type: "file" as const }
  ],
  alpha: [
    { path: "alpha/sub", type: "directory" as const },
    { path: "alpha/child.txt", type: "file" as const }
  ],
  "alpha/sub": []
};

const initialExpanded = new Set(["", "alpha", "alpha/sub"]);

const store = {
  session: session as MockSession | null,
  selectFolder: vi.fn(),
  tree: initialTree,
  toggleDir: vi.fn(),
  ensureRootOpen: vi.fn(),
  agentFiles: new Map(),
  openFile: vi.fn(),
  previewFile: vi.fn(),
  expanded: initialExpanded,
  openCtxMenu: vi.fn(),
  pendingCreate: null,
  commitName: vi.fn(),
  cancelPending: vi.fn(),
  startCreate: vi.fn(),
  startRename: vi.fn(),
  deleteEntry: vi.fn(),
  deleteEntries: vi.fn(),
  moveEntry: vi.fn(),
  moveEntries: vi.fn()
};

const ctxMenuApi = {
  ctxMenu: null as { x: number; y: number; target: TreeEntry | null } | null,
  openCtxMenu: vi.fn(),
  closeCtxMenu: vi.fn()
};

vi.mock("../store", () => ({
  useStore: () => store,
  useCtxMenu: () => ctxMenuApi
}));

function dragEvent(type: string): Event {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", {
    value: { setData: vi.fn(), effectAllowed: "", dropEffect: "" }
  });
  return event;
}

function row(container: HTMLElement, name: string): HTMLElement {
  const found = [...container.querySelectorAll<HTMLElement>(".tree-row")].find(
    (el) => el.querySelector(".tree-name")?.textContent === name
  );
  if (!found) throw new Error(`no tree row for ${name}`);
  return found;
}

describe("FileSidebar drag-and-drop moves", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    store.moveEntry.mockClear();
    store.moveEntries.mockClear();
    store.deleteEntry.mockClear();
    store.deleteEntries.mockClear();
    store.ensureRootOpen.mockClear();
    store.toggleDir.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    store.session = session;
    store.tree = initialTree;
    store.expanded = initialExpanded;
    store.ensureRootOpen.mockReset();
    vi.unstubAllGlobals();
  });

  it("ensure-opens the explorer root when the focused session changes", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));

    expect(store.ensureRootOpen).toHaveBeenCalledTimes(1);
    expect(store.toggleDir).not.toHaveBeenCalled();
  });

  it("re-arms the root guard when the last panel closes so a reopened session loads", () => {
    store.ensureRootOpen.mockImplementation(async () => {
      store.tree = { "": [{ path: "alpha", type: "directory" as const }] };
      store.expanded = new Set(["", "alpha"]);
    });
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    expect(store.ensureRootOpen).toHaveBeenCalledTimes(1);

    act(() => {
      store.session = null;
      store.tree = {};
      store.expanded = new Set();
      root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />);
    });
    expect(container.querySelector<HTMLButtonElement>(".sessions-new")?.textContent?.trim()).toBe("New Workspace");

    act(() => {
      store.session = {
        ...session,
        workspace: { id: "22222222-2222-4222-8222-222222222222", generation: 2 }
      };
      root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />);
    });
    expect(store.ensureRootOpen).toHaveBeenCalledTimes(2);

    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    expect(container.textContent).not.toContain("Loading…");
    expect(row(container, "alpha")).toBeTruthy();
  });

  it("creates a root file or folder from the workspace header hover actions", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const actions = container.querySelector<HTMLElement>(".section-trigger.explorer-trigger .tree-row-actions")!;
    const byTitle = (title: string): HTMLButtonElement =>
      [...actions.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.title === title)!;
    act(() => byTitle("New File").click());
    expect(store.startCreate).toHaveBeenCalledWith("", "file");
    act(() => byTitle("New Folder").click());
    expect(store.startCreate).toHaveBeenCalledWith("", "dir");
  });

  it("moves a file into a folder on drop", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "note.txt");
    const target = row(container, "beta");
    expect(source.draggable).toBe(true);

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => target.dispatchEvent(dragEvent("dragover")));
    expect(target.className).toContain("drop-target");
    act(() => target.dispatchEvent(dragEvent("drop")));

    expect(store.moveEntry).toHaveBeenCalledWith("note.txt", "beta");
    expect(store.moveEntry).toHaveBeenCalledTimes(1);
  });

  it("selects several rows with Command and moves them together", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const file = row(container, "note.txt");
    const folder = row(container, "alpha");
    act(() => file.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    act(() => folder.dispatchEvent(new MouseEvent("click", { bubbles: true, metaKey: true })));
    expect(file.className).toContain("selected");
    expect(folder.className).toContain("selected");

    const target = row(container, "beta");
    act(() => file.dispatchEvent(dragEvent("dragstart")));
    act(() => target.dispatchEvent(dragEvent("dragover")));
    act(() => target.dispatchEvent(dragEvent("drop")));

    expect(store.moveEntries).toHaveBeenCalledWith(["note.txt", "alpha"], "beta");
  });

  it("selects the visible row range with Shift and bulk deletes on Delete", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const first = row(container, "alpha");
    const last = row(container, "note.txt");
    act(() => first.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    act(() => last.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true })));
    expect(first.className).toContain("selected");
    expect(last.className).toContain("selected");
    expect(row(container, "beta").className).toContain("selected");

    const tree = container.querySelector<HTMLElement>(".tree")!;
    act(() => tree.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true })));

    expect(store.deleteEntries).toHaveBeenCalledWith(["alpha", "beta", "note.txt"]);
  });

  it("moves a folder into a sibling folder on drop", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "alpha");
    const target = row(container, "beta");

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => target.dispatchEvent(dragEvent("dragover")));
    act(() => target.dispatchEvent(dragEvent("drop")));

    expect(store.moveEntry).toHaveBeenCalledWith("alpha", "beta");
    expect(store.moveEntry).toHaveBeenCalledTimes(1);
  });

  it("drops onto the empty tree area to move to the session root", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "child.txt");
    const tree = container.querySelector<HTMLElement>(".tree")!;

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => tree.dispatchEvent(dragEvent("dragover")));
    expect(tree.querySelector(".tree-drop-space.active")).toBeTruthy();
    act(() => tree.dispatchEvent(dragEvent("drop")));

    expect(store.moveEntry).toHaveBeenCalledWith("alpha/child.txt", "");
  });

  it("rejects dropping a folder onto itself", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "alpha");

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => source.dispatchEvent(dragEvent("dragover")));
    act(() => source.dispatchEvent(dragEvent("drop")));

    expect(source.className).not.toContain("drop-target");
    expect(store.moveEntry).not.toHaveBeenCalled();
  });

  it("rejects dropping a folder into its own descendant", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "alpha");
    const descendant = row(container, "sub");

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => descendant.dispatchEvent(dragEvent("dragover")));
    act(() => descendant.dispatchEvent(dragEvent("drop")));

    expect(descendant.className).not.toContain("drop-target");
    expect(store.moveEntry).not.toHaveBeenCalled();
  });

  it("rejects dropping an entry onto its current parent", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "child.txt");
    const parent = row(container, "alpha");

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => parent.dispatchEvent(dragEvent("dragover")));
    act(() => parent.dispatchEvent(dragEvent("drop")));

    expect(parent.className).not.toContain("drop-target");
    expect(store.moveEntry).not.toHaveBeenCalled();
  });

  it("rejects dropping a file onto a file row", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "note.txt");
    const other = row(container, "child.txt");
    const tree = container.querySelector<HTMLElement>(".tree")!;

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => other.dispatchEvent(dragEvent("dragover")));
    act(() => other.dispatchEvent(dragEvent("drop")));

    expect(tree.className).not.toContain("drop-root");
    expect(store.moveEntry).not.toHaveBeenCalled();
  });

  it("clears drag state on dragend", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const source = row(container, "note.txt");
    const target = row(container, "beta");

    act(() => source.dispatchEvent(dragEvent("dragstart")));
    act(() => target.dispatchEvent(dragEvent("dragover")));
    expect(target.className).toContain("drop-target");
    act(() => source.dispatchEvent(dragEvent("dragend")));

    expect(target.className).not.toContain("drop-target");
  });
});
