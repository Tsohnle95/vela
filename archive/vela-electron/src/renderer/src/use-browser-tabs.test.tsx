import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { browserTabLabel, useBrowserTabs, type BrowserTabs } from "./use-browser-tabs";

describe("browser tabs", () => {
  let root: Root;
  let container: HTMLDivElement;
  let tabs: BrowserTabs;

  function Probe(): null {
    tabs = useBrowserTabs();
    return null;
  }
  const render = async (): Promise<void> => act(async () => root.render(<Probe />));

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await render();
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("opens an empty tab, activates it, and names it New tab", async () => {
    await act(async () => { tabs.openBrowserTab(); });

    expect(tabs.browserTabs).toHaveLength(1);
    expect(tabs.activeBrowserTabID).toBe(tabs.browserTabs[0]!.id);
    expect(tabs.browserTabs[0]).toMatchObject({ url: "", title: "", loading: false, error: null });
    expect(browserTabLabel(tabs.browserTabs[0]!)).toBe("New tab");
  });

  it("starts loading when opened at a URL and labels it by host", async () => {
    await act(async () => { tabs.openBrowserTab("https://www.example.com/docs"); });

    expect(tabs.browserTabs[0]!.loading).toBe(true);
    expect(browserTabLabel(tabs.browserTabs[0]!)).toBe("example.com");
  });

  it("prefers the page title once the guest reports one", async () => {
    await act(async () => { tabs.openBrowserTab("https://example.com"); });
    const id = tabs.browserTabs[0]!.id;
    await act(async () => { tabs.updateBrowserTab(id, { title: "Example Domain", loading: false }); });

    expect(browserTabLabel(tabs.browserTabs[0]!)).toBe("Example Domain");
  });

  it("ignores updates that would not change the tab", async () => {
    await act(async () => { tabs.openBrowserTab("https://example.com"); });
    const id = tabs.browserTabs[0]!.id;
    const before = tabs.browserTabs;
    await act(async () => { tabs.updateBrowserTab(id, { loading: true }); });

    expect(tabs.browserTabs).toBe(before);
  });

  it("activates a neighbour when the active tab is closed", async () => {
    let firstID = "";
    let secondID = "";
    await act(async () => { firstID = tabs.openBrowserTab("https://a.example"); });
    await act(async () => { secondID = tabs.openBrowserTab("https://b.example"); });
    expect(tabs.activeBrowserTabID).toBe(secondID);

    await act(async () => { tabs.closeBrowserTab(secondID); });
    expect(tabs.browserTabs.map((tab) => tab.id)).toEqual([firstID]);
    expect(tabs.activeBrowserTabID).toBe(firstID);

    await act(async () => { tabs.closeBrowserTab(firstID); });
    expect(tabs.browserTabs).toHaveLength(0);
    expect(tabs.activeBrowserTabID).toBeNull();
  });

  it("leaves the active tab alone when a background tab closes", async () => {
    let firstID = "";
    let secondID = "";
    await act(async () => { firstID = tabs.openBrowserTab(); });
    await act(async () => { secondID = tabs.openBrowserTab(); });
    await act(async () => { tabs.activateBrowserTab(firstID); });
    await act(async () => { tabs.closeBrowserTab(secondID); });

    expect(tabs.activeBrowserTabID).toBe(firstID);
  });

  it("can be deactivated back to the editor", async () => {
    await act(async () => { tabs.openBrowserTab("https://example.com"); });
    await act(async () => { tabs.activateBrowserTab(null); });

    expect(tabs.activeBrowserTabID).toBeNull();
    expect(tabs.browserTabs).toHaveLength(1);
  });

  it("ignores unknown activations and late updates after a tab closes", async () => {
    let id = "";
    await act(async () => { id = tabs.openBrowserTab("https://example.com"); });
    await act(async () => tabs.activateBrowserTab("missing"));
    expect(tabs.activeBrowserTabID).toBe(id);
    await act(async () => {
      tabs.closeBrowserTab(id);
      tabs.updateBrowserTab(id, { title: "Late title" });
    });
    expect(tabs.browserTabs).toEqual([]);
    expect(tabs.activeBrowserTabID).toBeNull();
  });
});
