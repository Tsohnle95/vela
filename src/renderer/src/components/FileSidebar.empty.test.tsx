import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FileSidebar } from "./FileSidebar";

const store: Record<string, unknown> = {
  session: null,
  panels: [],
  focusSession: vi.fn(),
  selectFolder: vi.fn(),
  tree: {},
  toggleDir: vi.fn(),
  ensureRootOpen: vi.fn(),
  agentFiles: new Map(),
  openFile: vi.fn(),
  previewFile: vi.fn(),
  expanded: new Set<string>(),
  pendingCreate: null,
  commitName: vi.fn(),
  cancelPending: vi.fn(),
  moveEntry: vi.fn(),
  startCreate: vi.fn(),
  singleFile: null,
  importPaths: vi.fn(),
  dropIntoExplorer: vi.fn(),
  openWorkspacePanel: vi.fn(),
  dismissChange: vi.fn(),
  dismissChanges: vi.fn(),
  hiddenPaths: new Set<string>()
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

describe("FileSidebar empty workspace state", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    store.selectFolder = vi.fn();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("keeps the empty workspace message and working folder button without a logo", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));

    const cta = container.querySelector<HTMLButtonElement>(".tree-empty-workspace .btn");
    expect(container.querySelector(".tree-empty-workspace .mira-mark")).toBeNull();
    expect(container.querySelector(".tree-empty-workspace-text")?.textContent).toBe("No workspace open");
    expect(cta?.textContent).toBe("Open a workspace");

    act(() => {
      cta!.click();
    });
    expect(store.selectFolder as ReturnType<typeof vi.fn>).toHaveBeenCalledTimes(1);
  });
});
