import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { Tab } from "@shared/types";
import { stripLayout, TabBar } from "./EditorTabs";

// Mirrors the reference strip's sizing rules: tabs share the available width
// (90px floor, 240px ceiling); once the floor no longer fits, the remainder
// moves into the overflow menu.
describe("stripLayout", () => {
  it("moves every tab into overflow when no width remains", () => {
    expect(stripLayout(0, 5)).toEqual({ visibleCount: 0, hiddenCount: 5 });
  });

  it("accounts for browser chips' wider minimum", () => {
    expect(stripLayout(300, 0, 0, 4)).toEqual({ visibleCount: 2, hiddenCount: 2 });
  });

  it("keeps all tabs when they fit at the minimum width", () => {
    expect(stripLayout(480, 5)).toEqual({ visibleCount: 5, hiddenCount: 0 });
  });

  it("hides the tabs that cannot fit at the minimum width", () => {
    expect(stripLayout(400, 5)).toEqual({ visibleCount: 4, hiddenCount: 1 });
  });

  it("keeps controls reachable when even one tab cannot fit", () => {
    expect(stripLayout(80, 4)).toEqual({ visibleCount: 0, hiddenCount: 4 });
  });

  it("reserves the overflow button's width before fitting tabs", () => {
    expect(stripLayout(1000, 12)).toEqual({ visibleCount: 10, hiddenCount: 2 });
  });

  it("fits file tabs around the width browser chips and the add control claim", () => {
    expect(stripLayout(600, 5)).toEqual({ visibleCount: 5, hiddenCount: 0 });
    expect(stripLayout(600, 5, 150 + 30)).toEqual({ visibleCount: 4, hiddenCount: 1 });
    expect(stripLayout(200, 5, 180)).toEqual({ visibleCount: 0, hiddenCount: 5 });
  });
});


describe("mixed tab overflow", () => {
  let root: Root;
  let container: HTMLDivElement;
  let width: number;
  let resize: () => void;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    width = 280;
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(() => width);
    vi.stubGlobal("ResizeObserver", class {
      constructor(callback: () => void) { resize = callback; }
      observe() {}
      disconnect() {}
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("marks a preview tab so it reads as transient", () => {
    width = 1200;
    const tabs = [
      { path: "one.ts", name: "one.ts", content: "", saved: "", preview: true },
      { path: "two.ts", name: "two.ts", content: "", saved: "" }
    ] as Tab[];
    act(() => root.render(createElement(TabBar, { tabs, activePath: "one.ts", ariaLabel: "Tabs", onSelect: vi.fn(), onClose: vi.fn(), onMoveTab: vi.fn() })));
    expect(container.querySelectorAll(".tab.preview")).toHaveLength(1);
    expect(container.querySelector(".tab.preview .tab-name")?.textContent).toBe("one.ts");
  });

  it("places the browser add button immediately after browser chips and omits it for file-only strips", () => {
    width = 1200;
    const tabs = [{ path: "one.ts", name: "one.ts", content: "", saved: "" }] as Tab[];
    const browsers = ["one", "two"].map(id => ({ id, url: "", title: id, loading: false, error: null, canGoBack: false, canGoForward: false }));
    const props = { tabs, activePath: "one.ts", ariaLabel: "Tabs", onSelect: vi.fn(), onClose: vi.fn(), onMoveTab: vi.fn(), onNewBrowser: vi.fn() };
    act(() => root.render(createElement(TabBar, { ...props, browsers })));
    const add = container.querySelector(".tab-add")!;
    expect(add.parentElement?.className).toBe("tab-list");
    expect(add.previousElementSibling?.classList.contains("browser")).toBe(true);
    expect(container.querySelectorAll(".tab.browser")).toHaveLength(2);
    act(() => root.render(createElement(TabBar, props)));
    expect(container.querySelector(".tab-add")).toBeNull();
  });

  it("keeps the browser add control next to a browser when files and browsers overflow together", () => {
    width = 280;
    const tabs = Array.from({ length: 8 }, (_, index) => ({ path: `${index}.ts`, name: `${index}.ts`, content: "", saved: "" })) as Tab[];
    const browsers = ["one", "two"].map(id => ({ id, url: "", title: id, loading: false, error: null, canGoBack: false, canGoForward: false }));
    act(() => root.render(createElement(TabBar, { tabs, browsers, activePath: null, ariaLabel: "Tabs", onSelect: vi.fn(), onClose: vi.fn(), onMoveTab: vi.fn(), onNewBrowser: vi.fn() })));
    expect(container.querySelectorAll(".tab.browser")).toHaveLength(1);
    expect(container.querySelectorAll(".tab:not(.browser)")).toHaveLength(1);
    expect(container.querySelector(".tab-add")?.previousElementSibling?.classList.contains("browser")).toBe(true);
  });

  it("keeps files, browser tabs and add reachable after shrinking to zero", () => {
    const onSelect = vi.fn();
    const onSelectBrowser = vi.fn();
    const onNewBrowser = vi.fn();
    const tabs = Array.from({ length: 8 }, (_, index) => ({ path: `file-${index}.ts`, name: `file-${index}.ts` } as Tab));
    const browsers = Array.from({ length: 8 }, (_, index) => ({ id: `browser-${index}`, url: `https://example.com/${index}`,
      title: `Browser ${index}`, loading: false, canGoBack: false, canGoForward: false, error: null }));
    act(() => root.render(createElement(TabBar, { tabs, activePath: null, ariaLabel: "Tabs", browsers,
      onSelect, onClose: vi.fn(), onMoveTab: vi.fn(), onSelectBrowser, onNewBrowser })));
    expect(container.querySelectorAll('[role="tab"]')).toHaveLength(2);
    act(() => container.querySelector<HTMLButtonElement>('.tab-overflow-button')!.click());
    const items = () => [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
    expect(items()).toHaveLength(16);
    act(() => items().find((item) => item.textContent === "Browser 7")!.click());
    expect(onSelectBrowser).toHaveBeenCalledWith("browser-7");
    width = 0;
    act(() => resize());
    expect(container.querySelectorAll('[role="tab"]')).toHaveLength(0);
    act(() => container.querySelector<HTMLButtonElement>('.tab-overflow-button')!.click());
    act(() => items().find((item) => item.textContent === "file-7.ts")!.click());
    expect(onSelect).toHaveBeenCalledWith("file-7.ts");
    act(() => container.querySelector<HTMLButtonElement>('[aria-label="New browser tab"]')!.click());
    expect(onNewBrowser).toHaveBeenCalledOnce();
  });
});
