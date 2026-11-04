import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FileSidebar } from "./FileSidebar";

const store = {
  session: { id: "session", directory: "/workspace", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } },
  selectFolder: vi.fn(),
  tree: { "": [{ path: "folder", type: "directory" as const }] },
  toggleDir: vi.fn(),
  ensureRootOpen: vi.fn(),
  agentFiles: new Map(),
  openFile: vi.fn(),
  previewFile: vi.fn(),
  expanded: new Set([""]),
  openCtxMenu: vi.fn(),
  pendingCreate: null,
  commitName: vi.fn(),
  cancelPending: vi.fn(),
  startCreate: vi.fn(),
  startRename: vi.fn(),
  deleteEntry: vi.fn()
};

const ctxMenuApi = {
  ctxMenu: { x: 10, y: 10, target: { path: "folder", type: "directory" as const } },
  openCtxMenu: vi.fn(),
  closeCtxMenu: vi.fn()
};

vi.mock("../store", () => ({
  useStore: () => store,
  useCtxMenu: () => ctxMenuApi
}));

describe("FileSidebar rename actions", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("offers rename for directories", () => {
    act(() => root.render(<FileSidebar collapsed={false} onCollapse={() => {}} onDrag={() => {}} />));

    const rename = [...document.body.querySelectorAll<HTMLButtonElement>(".ctx-item")]
      .find((button) => button.textContent?.includes("Rename"));
    expect(rename).toBeTruthy();
    act(() => rename!.click());
    expect(store.startRename).toHaveBeenCalledWith("folder");
  });
});
