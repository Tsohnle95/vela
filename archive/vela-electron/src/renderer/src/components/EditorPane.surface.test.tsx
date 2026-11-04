import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, Tab } from "@shared/types";
import { EditorPane } from "./EditorPane";
import { ThemeProvider } from "../theme";
import { BrowserTabsProvider } from "../browser-tabs-context";

const tab: Tab = {
  path: "src/page.ts",
  name: "page.ts",
  content: "after",
  saved: "after",
  baseline: { kind: "unknown" },
  dirty: false,
  stale: false,
  deleted: false,
  revision: 0,
  conflict: null,
  binary: false
};
const session: SessionInfo = {
  id: "session",
  directory: "/workspace",
  workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 }
};
const store = {
  session,
  selectFolder: vi.fn(),
  tree: { "": [] },
  toggleDir: vi.fn(),
  ensureRootOpen: vi.fn(),
  openFile: vi.fn(),
  expanded: new Set<string>(),
  openCtxMenu: vi.fn(),
  pendingCreate: null,
  commitName: vi.fn(),
  cancelPending: vi.fn(),
  tabs: [tab],
  activePath: tab.path,
  browserTabs: [],
  activeBrowserTabID: null,
  openBrowserTab: vi.fn(() => "browser-1"),
  closeBrowserTab: vi.fn(),
  activateBrowserTab: vi.fn(),
  updateBrowserTab: vi.fn(),
  setActive: vi.fn(),
  closeTab: vi.fn(),
  editContent: vi.fn(),
  saveTab: vi.fn(),
  reloadTab: vi.fn(),
  overwriteTab: vi.fn(),
  mergeTab: vi.fn()
};

const ctxMenuApi = { ctxMenu: null, openCtxMenu: vi.fn(), closeCtxMenu: vi.fn() };

vi.mock("../store", () => ({
  useStore: () => store,
  useCtxMenu: () => ctxMenuApi
}));
vi.mock("../emmet-keys", () => ({ wireEmmetKeys: vi.fn() }));
vi.mock("@monaco-editor/react", () => ({
  default: () => <div data-testid="editor" />
}));
vi.mock("../monaco", () => ({
  languageForPath: () => "typescript",
  monaco: { editor: { setModelMarkers: vi.fn() } }
}));
vi.mock("../diagnostics", () => ({
  createDiagnosticsScheduler: () => ({ schedule: vi.fn(), cancel: vi.fn() }),
  isHtmlFile: () => false,
  validateHtmlContent: () => []
}));

describe("editor surface", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
    store.tabs = [tab];
    store.activePath = tab.path;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  const render = (): void => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane /></BrowserTabsProvider></ThemeProvider>));
  };
  const actionLabels = (): string[] =>
    [...container.querySelectorAll<HTMLButtonElement>(".editor-breadcrumb-actions button")].map((button) => button.textContent?.trim() ?? "");

  it("opens a file in the edit surface without any diff toggle", () => {
    render();

    expect(container.querySelector("[data-testid=editor]")).toBeTruthy();
    expect(container.querySelector("[data-testid=diff-editor]")).toBeNull();
    expect(container.querySelector(".editor-breadcrumbs")?.textContent).toContain("workspace");
    expect(container.querySelector(".editor-breadcrumbs")?.textContent).toContain("page.ts");
    expect(actionLabels()).not.toContain("Edit");
    expect(actionLabels()).not.toContain("Diff");
    expect(actionLabels().some((label) => label.includes("Diff unavailable"))).toBe(false);
  });

  it("omits save status text while retaining the dirty file save action", () => {
    store.tabs = [{ ...tab, dirty: true }];
    render();
    expect(container.querySelector(".editor-dirty")).toBeNull();
    const save = container.querySelector<HTMLButtonElement>('[aria-label="Save page.ts"]');
    expect(save).not.toBeNull();
    act(() => save!.click());
    expect(store.saveTab).toHaveBeenCalledWith(tab.path);
  });

  it("keeps the edit surface for a tab with a known baseline", () => {
    const knownTab: Tab = { ...tab, baseline: { kind: "known", content: "before" } };
    store.tabs = [knownTab];
    store.activePath = knownTab.path;

    render();

    expect(container.querySelector("[data-testid=editor]")).toBeTruthy();
    expect(container.querySelector("[data-testid=diff-editor]")).toBeNull();
    expect(actionLabels()).not.toContain("Diff");
  });
});
