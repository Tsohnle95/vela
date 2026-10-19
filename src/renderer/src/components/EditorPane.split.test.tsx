import { act, useEffect } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, Tab } from "@shared/types";
import { ThemeProvider } from "../theme";
import { BrowserTabsProvider, useBrowserTabActions } from "../browser-tabs-context";
import { EditorPane } from "./EditorPane";

const firstTab: Tab = {
  path: "src/one.ts",
  name: "one.ts",
  content: "one",
  saved: "one",
  baseline: null,
  dirty: false,
  stale: false,
  deleted: false,
  revision: 0,
  conflict: null,
  binary: false
};
const secondTab: Tab = { ...firstTab, path: "src/two.ts", name: "two.ts", content: "two", saved: "two" };
const thirdTab: Tab = { ...firstTab, path: "src/three.ts", name: "three.ts", content: "three", saved: "three" };
const fourthTab: Tab = { ...firstTab, path: "src/four.ts", name: "four.ts", content: "four", saved: "four" };
const jsTab: Tab = { ...firstTab, path: "src/library.js", name: "library.js", content: "document.", saved: "document." };
const EDITOR_TAB_MIME = "application/x-mira-editor-tab";
const capturedEditorOptions = vi.hoisted(() => vi.fn());

function createTabTransfer(): DataTransfer {
  const values = new Map<string, string>();
  const types: string[] = [];
  return {
    effectAllowed: "all",
    dropEffect: "none",
    files: [] as unknown as FileList,
    items: [] as unknown as DataTransferItemList,
    types,
    setData(type: string, value: string) {
      values.set(type, value);
      if (!types.includes(type)) types.push(type);
    },
    getData(type: string) { return values.get(type) ?? ""; }
  } as unknown as DataTransfer;
}

function dispatchTabDrag(element: Element, type: "dragstart" | "dragover" | "drop", dataTransfer: DataTransfer): void {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: dataTransfer });
  element.dispatchEvent(event);
}

const store = {
  tabs: [firstTab, secondTab],
  activePath: firstTab.path as string | null,
  browserTabs: [],
  activeBrowserTabID: null,
  openBrowserTab: vi.fn(() => "browser-1"),
  closeBrowserTab: vi.fn(),
  activateBrowserTab: vi.fn(),
  updateBrowserTab: vi.fn(),
  session: { directory: "/workspace", workspace: { id: "workspace-1", generation: 1 } } as Pick<SessionInfo, "directory" | "workspace"> | null,
  openPaths: vi.fn(),
  setActive: vi.fn(),
  closeTab: vi.fn(),
  editContent: vi.fn(),
  saveTab: vi.fn(),
  reloadTab: vi.fn(),
  overwriteTab: vi.fn(),
  mergeTab: vi.fn()
};
store.setActive.mockImplementation((path: string) => { store.activePath = path; });

vi.mock("../store", () => ({ useStore: () => store }));
vi.mock("../emmet-keys", () => ({ wireEmmetKeys: vi.fn() }));
vi.mock("../editor-navigation", () => ({ wireEditorNavigationKeys: vi.fn() }));
vi.mock("../editor-language-input", () => ({ wireHtmlAttributeQuotes: vi.fn() }));
vi.mock("../monaco", () => ({ languageForPath: (path: string) => path.endsWith(".js") ? "javascript" : "typescript" }));
vi.mock("../w3c-validation", () => ({ clearW3cMarkers: vi.fn() }));
vi.mock("@monaco-editor/react", () => ({
  default: ({ path, language, defaultValue, onMount, options }: { path: string; language: string; defaultValue: string; onMount?: (editor: unknown) => void; options?: unknown }) => {
    capturedEditorOptions(options);
    const model = {
      getValue: () => defaultValue,
      getOptions: () => ({ tabSize: 4, insertSpaces: true })
    };
    useEffect(() => { onMount?.({
      getPosition: () => ({ lineNumber: 8, column: 13 }),
      getModel: () => model,
      onDidChangeCursorPosition: () => ({ dispose: () => {} }),
      onDidScrollChange: () => ({ dispose: () => {} })
    }); }, [path]);
    return <div data-testid="editor" data-path={path} data-language={language} />;
  }
}));

describe("EditorPane split groups", () => {
  let container: HTMLDivElement;
  let root: Root;
  let freshBrowserSlot: HTMLDivElement | null;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
    freshBrowserSlot = null;
    store.session = { directory: "/workspace", workspace: { id: "workspace-1", generation: 1 } };
    store.tabs = [firstTab, secondTab, thirdTab];
    store.activePath = firstTab.path;
    store.setActive.mockClear();
    store.setActive.mockImplementation((path: string) => { store.activePath = path; });
    store.saveTab.mockClear();
    capturedEditorOptions.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    freshBrowserSlot?.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("hosts closable browser chips before any workspace is opened", () => {
    store.session = null;
    store.tabs = [];
    store.activePath = null;
    let browserActions!: ReturnType<typeof useBrowserTabActions>;
    function BrowserActionsProbe(): null {
      browserActions = useBrowserTabActions();
      return null;
    }
    const slot = document.createElement("div");
    freshBrowserSlot = slot;
    document.body.append(slot);
    act(() => root.render(<ThemeProvider><BrowserTabsProvider>
      <BrowserActionsProbe /><EditorPane tabsSlot={slot} />
    </BrowserTabsProvider></ThemeProvider>));
    act(() => { browserActions.openBrowserTab(); });
    expect(slot.querySelector('[role="tab"].browser')).not.toBeNull();
    expect(slot.querySelector<HTMLButtonElement>('[aria-label="Close New tab"]')).not.toBeNull();
    expect(container.querySelector(".editor-stack")?.classList).toContain("concealed");
    expect(container.querySelector(".editor-empty")?.textContent).toBe("Open a workspace from the explorer to get started.");
    act(() => slot.querySelector<HTMLButtonElement>('[aria-label="Close New tab"]')!.click());
    expect(slot.querySelector('[role="tab"].browser')).toBeNull();
    expect(container.querySelector(".editor-stack")?.classList).not.toContain("concealed");
    expect(container.querySelector(".editor-empty")?.textContent).toBe("Open a workspace from the explorer to get started.");
  });

  it("opens two side-by-side editor groups and closes the secondary group", () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());

    const groups = [...container.querySelectorAll<HTMLElement>(".editor-group")];
    expect(groups).toHaveLength(2);
    expect(groups[1].classList).toContain("editor-group-focused");
    expect(groups[0].querySelector("[data-testid=editor]")?.getAttribute("data-path")).toBe(firstTab.path);
    expect(groups[1].querySelector("[data-testid=editor]")?.getAttribute("data-path")).toBe(secondTab.path);
    expect([...groups[0].querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title)).toEqual([
      firstTab.path,
      thirdTab.path
    ]);
    expect([...groups[1].querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title)).toEqual([
      secondTab.path
    ]);
    expect(groups[0].querySelector(".editor-breadcrumb-actions .editor-group-action")).not.toBeNull();
    expect(groups[1].querySelector('.editor-breadcrumb-actions [aria-label="Close secondary editor group"]')).not.toBeNull();

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Close secondary editor group"]')!.click());
    expect(container.querySelectorAll(".editor-group")).toHaveLength(1);
    expect(container.querySelector("[data-testid=editor]")?.getAttribute("data-path")).toBe(firstTab.path);
    expect(container.querySelectorAll('[role="tab"]')).toHaveLength(3);
  });

  it("resizes split editor columns and rows by dragging their dividers", () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    const grid = container.querySelector<HTMLElement>(".editor-groups")!;
    vi.spyOn(grid, "getBoundingClientRect").mockReturnValue({ left: 0, top: 0, width: 800, height: 600 } as DOMRect);
    const columns = container.querySelector<HTMLElement>('[aria-label="Resize editor columns"]')!;
    act(() => columns.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 400 })));
    act(() => window.dispatchEvent(new MouseEvent("mousemove", { clientX: 550 })));
    act(() => window.dispatchEvent(new MouseEvent("mouseup", { clientX: 550 })));
    expect(grid.style.gridTemplateColumns).toBe("68.75% minmax(0, 1fr)");

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor group 2 right"]')!.click());
    const rows = container.querySelector<HTMLElement>('[aria-label="Resize editor rows"]')!;
    act(() => rows.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientY: 300 })));
    act(() => window.dispatchEvent(new MouseEvent("mousemove", { clientY: 420 })));
    act(() => window.dispatchEvent(new MouseEvent("mouseup", { clientY: 420 })));
    expect(grid.style.gridTemplateRows).toBe("70% minmax(0, 1fr)");
  });

  it("resizes from keyboard with clamped accessible values", () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane /></BrowserTabsProvider></ThemeProvider>));
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    const separator = container.querySelector<HTMLElement>('[aria-label="Resize editor columns"]')!;
    expect(separator.tabIndex).toBe(0);
    act(() => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
    expect(separator.getAttribute("aria-valuenow")).toBe("55");
    for (let step = 0; step < 30; step++) {
      act(() => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })));
    }
    expect(separator.getAttribute("aria-valuenow")).toBe(separator.getAttribute("aria-valuemin"));
    expect(Number(separator.getAttribute("aria-valuenow"))).toBeGreaterThan(0);
  });

  it("releases resize listeners on restart and unmount", () => {
    const remove = vi.spyOn(window, "removeEventListener");
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane /></BrowserTabsProvider></ThemeProvider>));
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    const separator = container.querySelector<HTMLElement>('[aria-label="Resize editor columns"]')!;
    act(() => separator.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    remove.mockClear();
    act(() => separator.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(remove).toHaveBeenCalledWith("mousemove", expect.any(Function));
    expect(remove).toHaveBeenCalledWith("mouseup", expect.any(Function));
    remove.mockClear();
    act(() => root.render(<></>));
    expect(remove).toHaveBeenCalledWith("mousemove", expect.any(Function));
    expect(remove).toHaveBeenCalledWith("mouseup", expect.any(Function));
  });

  it("can create four editor groups and disables splitting at the limit", () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor group 2 right"]')!.click());
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor group 3 right"]')!.click());

    expect(container.querySelectorAll(".editor-group")).toHaveLength(4);
    expect(container.querySelector<HTMLButtonElement>('[aria-label="Split editor group 4 right"]')?.disabled).toBe(true);
  });

  it("keeps a split panel's file tabs in the hosted strip instead of inside the panel", () => {
    const slot = document.createElement("div");
    container.append(slot);
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane tabsSlot={slot} /></BrowserTabsProvider></ThemeProvider>));
    expect([...slot.querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title))
      .toEqual([firstTab.path, secondTab.path, thirdTab.path]);
    expect(container.querySelector(".editor-group .tabbar")).toBeNull();

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    expect([...slot.querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title))
      .toEqual([firstTab.path, thirdTab.path, secondTab.path]);
    expect(slot.querySelector(".tab-group-divider")).not.toBeNull();
    expect(container.querySelector(".editor-group .tabbar")).toBeNull();

    act(() => slot.querySelector<HTMLElement>(`[role="tab"][title="${secondTab.path}"]`)!.click());
    expect(store.setActive).toHaveBeenLastCalledWith(secondTab.path);
  });

  it("restores each workspace's editor group layout after switching away and back", () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    expect(container.querySelectorAll(".editor-group")).toHaveLength(2);

    store.session = { directory: "/other", workspace: { id: "workspace-2", generation: 1 } };
    store.tabs = [fourthTab];
    store.activePath = fourthTab.path;
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));
    expect(container.querySelectorAll(".editor-group")).toHaveLength(1);

    store.session = { directory: "/workspace", workspace: { id: "workspace-1", generation: 1 } };
    store.tabs = [firstTab, secondTab, thirdTab];
    store.activePath = secondTab.path;
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    expect(container.querySelectorAll(".editor-group")).toHaveLength(2);
    expect(container.querySelector(".editor-group-secondary [data-testid=editor]")?.getAttribute("data-path"))
      .toBe(secondTab.path);
  });

  it("offers an explicit save action for a dirty file", () => {
    store.tabs = [{ ...firstTab, content: "unfinished", dirty: true, revision: 1 }];
    store.activePath = firstTab.path;
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    const save = container.querySelector<HTMLButtonElement>(`[aria-label="Save ${firstTab.name}"]`)!;
    expect(save).toBeTruthy();
    act(() => save.click());
    expect(store.saveTab).toHaveBeenCalledWith(firstTab.path);
  });

  it("reports cursor details from the focused Monaco editor", async () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".status-menu-button")!.click();
    });
    expect(container.querySelector('[data-testid="cursor-position"]')?.textContent).toBe("Ln 8, Col 13");
    expect(container.querySelector('[data-testid="indentation-status"]')?.textContent).toBe("Spaces: 4");

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    act(() => container.querySelector<HTMLButtonElement>('.editor-group-focused .status-menu-button')!.click());
    expect(container.querySelector('.editor-group-focused [data-testid="cursor-position"]')?.textContent).toBe("Ln 8, Col 13");
  });

  it("turns on automatic trigger suggestions for JavaScript editors", () => {
    store.tabs = [jsTab];
    store.activePath = jsTab.path;
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    expect(container.querySelector("[data-testid='editor']")?.getAttribute("data-language")).toBe("javascript");
    const options = capturedEditorOptions.mock.calls.at(-1)?.[0] as {
      quickSuggestions?: { other?: boolean };
      suggestOnTriggerCharacters?: boolean;
      suggest?: { selectionMode?: string };
      fixedOverflowWidgets?: boolean;
      overflowWidgetsDomNode?: HTMLElement;
      wordBasedSuggestions?: string;
      tabCompletion?: string;
    };
    expect(options.quickSuggestions?.other).toBe(true);
    expect(options.suggestOnTriggerCharacters).toBe(true);
    expect(options.suggest?.selectionMode).toBe("always");
    expect(options.fixedOverflowWidgets).toBe(true);
    expect(options.overflowWidgetsDomNode?.classList.contains("monaco-editor")).toBe(true);
    expect(options.overflowWidgetsDomNode?.parentElement).toBe(document.body);
    expect(options.wordBasedSuggestions).toBe("currentDocument");
    expect(options.tabCompletion).toBe("on");
  });

  it("routes new files and save shortcuts to the group focused from its editor surface", () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());

    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true })));
    expect(store.saveTab).toHaveBeenCalledTimes(1);
    expect(store.saveTab).toHaveBeenLastCalledWith(secondTab.path);

    const primary = container.querySelector<HTMLElement>('[aria-label="Primary editor"]')!;
    act(() => primary.querySelector<HTMLElement>("[data-testid=editor]")!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(primary.classList).toContain("editor-group-focused");
    act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", ctrlKey: true, bubbles: true })));
    expect(store.saveTab).toHaveBeenCalledTimes(2);
    expect(store.saveTab).toHaveBeenLastCalledWith(firstTab.path);
    let paths = [...container.querySelectorAll<HTMLElement>("[data-testid=editor]")].map((editor) => editor.dataset.path);
    expect(paths).toEqual([firstTab.path, secondTab.path]);

    const secondary = container.querySelector<HTMLElement>('[aria-label="Secondary editor"]')!;
    act(() => secondary.querySelector<HTMLElement>("[data-testid=editor]")!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(secondary.classList).toContain("editor-group-focused");
    paths = [...container.querySelectorAll<HTMLElement>("[data-testid=editor]")].map((editor) => editor.dataset.path);
    expect(paths).toEqual([firstTab.path, secondTab.path]);

    store.tabs = [...store.tabs, fourthTab];
    store.activePath = fourthTab.path;
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    paths = [...container.querySelectorAll<HTMLElement>("[data-testid=editor]")].map((editor) => editor.dataset.path);
    expect(paths).toEqual([firstTab.path, fourthTab.path]);
    const groups = [...container.querySelectorAll<HTMLElement>(".editor-group")];
    expect([...groups[0].querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title)).toEqual([
      firstTab.path,
      thirdTab.path
    ]);
    expect([...groups[1].querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title)).toEqual([
      secondTab.path,
      fourthTab.path
    ]);
  });

  it("routes a newly opened file into an initially empty secondary group", () => {
    store.tabs = [firstTab];
    store.activePath = firstTab.path;
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());
    expect(container.querySelector(".editor-group-secondary .editor-group-empty")).toBeTruthy();
    expect(container.querySelector('.editor-group-secondary [aria-label="Split editor group 2 right"]')).not.toBeNull();

    store.tabs = [firstTab, secondTab];
    store.activePath = secondTab.path;
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));

    const groups = [...container.querySelectorAll<HTMLElement>(".editor-group")];
    expect(groups[0].querySelector("[data-testid=editor]")?.getAttribute("data-path")).toBe(firstTab.path);
    expect(groups[1].querySelector("[data-testid=editor]")?.getAttribute("data-path")).toBe(secondTab.path);
    expect([...groups[0].querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title)).toEqual([firstTab.path]);
    expect([...groups[1].querySelectorAll<HTMLElement>('[role="tab"]')].map((tab) => tab.title)).toEqual([secondTab.path]);
  });

  it("moves a tab between split editor groups when dragged across them", () => {
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane  /></BrowserTabsProvider></ThemeProvider>));
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Split editor right"]')!.click());

    // The mock store has no subscription; publish the focus changed by the split.
    act(() => root.render(<ThemeProvider><BrowserTabsProvider><EditorPane /></BrowserTabsProvider></ThemeProvider>));

    const transfer = createTabTransfer();
    const primaryTab = container.querySelector<HTMLElement>(`.editor-group-primary [role="tab"][title="${firstTab.path}"]`)!;
    const secondaryGroup = container.querySelector<HTMLElement>(".editor-group-secondary")!;
    act(() => dispatchTabDrag(primaryTab, "dragstart", transfer));
    expect(transfer.types).toContain(EDITOR_TAB_MIME);
    act(() => dispatchTabDrag(secondaryGroup, "dragover", transfer));
    expect(secondaryGroup.classList).toContain("editor-group-drop-target");
    act(() => dispatchTabDrag(secondaryGroup, "drop", transfer));

    const primaryTabs = [...container.querySelectorAll<HTMLElement>(".editor-group-primary [role='tab']")];
    const secondaryTabs = [...container.querySelectorAll<HTMLElement>(".editor-group-secondary [role='tab']")];
    expect(primaryTabs.map((tab) => tab.title)).toEqual([thirdTab.path]);
    expect(secondaryTabs.map((tab) => tab.title)).toEqual([firstTab.path, secondTab.path]);
    expect(container.querySelector(".editor-group-secondary [data-testid='editor']")?.getAttribute("data-path"))
      .toBe(firstTab.path);
    expect(store.setActive).toHaveBeenLastCalledWith(firstTab.path);
  });
});
