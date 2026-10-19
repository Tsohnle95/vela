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
    { path: "note.txt", type: "file" as const }
  ],
  alpha: []
};

const initialExpanded = new Set(["", "alpha"]);

const store: Record<string, unknown> = {
  session: session as MockSession | null,
  selectFolder: vi.fn(),
  tree: initialTree,
  toggleDir: vi.fn(),
  ensureRootOpen: vi.fn(),
  agentFiles: new Map(),
  tabs: [],
  openFile: vi.fn(),
  previewFile: vi.fn(),
  expanded: initialExpanded,
  openCtxMenu: vi.fn(),
  pendingCreate: null,
  commitName: vi.fn(),
  cancelPending: vi.fn(),
  ctxMenu: null,
  closeCtxMenu: vi.fn(),
  startCreate: vi.fn(),
  startRename: vi.fn(),
  deleteEntry: vi.fn(),
  moveEntry: vi.fn(),
  singleFile: null,
  openExternalPath: vi.fn(),
  importPaths: vi.fn(),
  dropIntoExplorer: vi.fn(),
  addModelPanel: vi.fn(),
  openSession: vi.fn(),
  openWorkspacePanel: vi.fn()
};

const ctxMenuApi = {
  ctxMenu: null,
  openCtxMenu: vi.fn(),
  closeCtxMenu: vi.fn()
};

vi.mock("../store", () => ({
  useStore: () => store,
  useCtxMenu: () => ctxMenuApi
}));

function dropEvent(files: string[], type = "drop"): Event {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "target", { value: event.target, configurable: true });
  Object.defineProperty(event, "dataTransfer", {
    value: {
      types: files.length > 0 ? ["Files"] : [],
      files: files.map((path) => ({ path }) as unknown as File),
      setData: vi.fn(),
      effectAllowed: "",
      dropEffect: ""
    }
  });
  return event;
}

function row(container: HTMLElement, name: string): HTMLElement | null {
  return ([...container.querySelectorAll<HTMLElement>(".tree-row")].find(
    (el) => el.querySelector(".tree-name")?.textContent === name
  ) ?? null);
}

describe("FileSidebar single-file mode and external drops", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.vela = {
      externalKind: vi.fn(async (path: string) => ({ kind: /\.[^/]+$/.test(path) ? "file" as const : "directory" as const }))
    } as unknown as typeof window.vela;
    store.singleFile = null;
    store.panels = [];
    store.tabs = [];
    store.expanded = new Set(initialExpanded);
    store.openExternalPath = vi.fn();
    store.importPaths = vi.fn();
    store.dropIntoExplorer = vi.fn();
    store.openWorkspacePanel = vi.fn();
    store.ensureRootOpen = vi.fn();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    store.session = session;
    vi.unstubAllGlobals();
  });

  it("renders single-file mode with the file row and no explorer tree", () => {
    store.singleFile = "index.html";
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));

    expect(container.textContent).toContain("FILE");
    expect(container.textContent).toContain("index.html");
    expect(container.textContent).not.toContain("EXPLORER");
    expect(container.textContent).not.toContain("CHANGES");
    expect(container.textContent).not.toContain("alpha");
  });

  it("marks an unsaved file in the Explorer with an orange status dot", () => {
    store.tabs = [{ path: "note.txt", dirty: true }];
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));

    expect(row(container, "note.txt")?.querySelector(".tree-unsaved-dot")).toBeTruthy();
  });

  it("opens an external file dropped onto the empty explorer area", async () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const tree = container.querySelector<HTMLElement>(".tree")!;
    await act(async () => {
      tree.dispatchEvent(dropEvent(["/outside/notes.txt"]));
      await Promise.resolve();
    });

    expect(store.dropIntoExplorer).toHaveBeenCalledWith(["/outside/notes.txt"]);
    expect(store.importPaths).not.toHaveBeenCalled();
  });

  it("imports external files and folders dropped onto a folder row", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const alpha = row(container, "alpha")!;
    const paths = ["/outside/patch.ts", "/outside/advanced-web-concepts"];
    act(() => {
      alpha.dispatchEvent(dropEvent(paths));
    });

    expect(store.importPaths).toHaveBeenCalledWith("alpha", paths);
    expect(store.openExternalPath).not.toHaveBeenCalled();
    expect(store.openWorkspacePanel).not.toHaveBeenCalled();
  });

  it("opens an external folder as another workspace when the root is collapsed", async () => {
    store.expanded = new Set(["alpha"]);
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const tree = container.querySelector<HTMLElement>(".tree")!;
    await act(async () => {
      tree.dispatchEvent(dropEvent(["/outside/advanced-web-concepts"]));
      await Promise.resolve();
    });

    expect(store.openWorkspacePanel).toHaveBeenCalledWith("/outside/advanced-web-concepts");
    expect(store.dropIntoExplorer).not.toHaveBeenCalled();
  });

  it.each([false, true])("shows only the blank destination below unrelated roots (expanded=%s)", async (expanded) => {
    store.panels = [{ ...session, id: "other", directory: "/other" }, session];
    store.expanded = expanded ? new Set([""]) : new Set();
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const space = container.querySelector<HTMLElement>(".tree-drop-space")!;
    act(() => space.dispatchEvent(dropEvent(["/outside/incoming"], "dragover")));

    expect(space.classList.contains("active")).toBe(true);
    expect(space.textContent).toBe(expanded ? "Add to workspace" : "Open as new workspace");
    expect(container.querySelector(".tree")?.className).toBe("tree");
    expect(container.querySelector(".workspace-root.drop-target")).toBeNull();
    expect(space.previousElementSibling).toBeTruthy();

    await act(async () => {
      space.dispatchEvent(dropEvent(["/outside/incoming"]));
      await Promise.resolve();
    });
    if (expanded) {
      expect(store.dropIntoExplorer).toHaveBeenCalledWith(["/outside/incoming"]);
      expect(store.openWorkspacePanel).not.toHaveBeenCalled();
    } else {
      expect(store.openWorkspacePanel).toHaveBeenCalledWith("/outside/incoming");
      expect(store.dropIntoExplorer).not.toHaveBeenCalled();
    }
    expect(space.classList.contains("active")).toBe(false);
  });

  it("imports an external folder dropped directly on the collapsed root", async () => {
    store.expanded = new Set(["alpha"]);
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const workspace = container.querySelector<HTMLElement>(".section-trigger.explorer-trigger")!;
    act(() => {
      container.querySelector(".tree-drop-space")!.dispatchEvent(dropEvent(["/outside/advanced-web-concepts"], "dragover"));
      workspace.dispatchEvent(dropEvent(["/outside/advanced-web-concepts"], "dragover"));
    });
    expect(workspace.classList.contains("drop-target")).toBe(true);
    expect(container.querySelector(".tree-drop-space.active")).toBeNull();
    await act(async () => {
      workspace.dispatchEvent(dropEvent(["/outside/advanced-web-concepts"]));
      await Promise.resolve();
    });

    expect(store.dropIntoExplorer).toHaveBeenCalledWith(["/outside/advanced-web-concepts"]);
    expect(store.openWorkspacePanel).not.toHaveBeenCalled();
  });

  it("keeps an expanded workspace's empty drop area inside its group when another root follows it", () => {
    store.panels = [session, { ...session, id: "other", directory: "/other" }];
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));
    const space = container.querySelector<HTMLElement>(".tree-drop-space")!;
    act(() => space.dispatchEvent(dropEvent(["/outside/notes.txt"], "dragover")));
    const group = space.closest(".tree-workspace")!;
    expect(container.querySelector(".explorer-root-name")?.textContent).toBe("workspace");
    expect(group.nextElementSibling?.querySelector(".tree-name")?.textContent).toBe("other");
    expect(group.nextElementSibling?.classList.contains("drop-target")).toBe(false);
    expect(space.textContent).toBe("Add to workspace");
  });
});
