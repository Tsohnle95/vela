import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BrowserTab } from "../use-browser-tabs";
import { BrowserPane, browserAddress } from "./BrowserPane";
import { TabBar } from "./EditorTabs";

const browserTab: BrowserTab = {
  id: "browser-1",
  url: "",
  title: "",
  loading: false,
  canGoBack: false,
  canGoForward: false,
  error: null
};

function type(input: HTMLInputElement, value: string): void {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

interface FakeWebview extends HTMLElement {
  loadURL: ReturnType<typeof vi.fn>;
  getURL: () => string;
  canGoBack: () => boolean;
  canGoForward: () => boolean;
  goBack: ReturnType<typeof vi.fn>;
  goForward: ReturnType<typeof vi.fn>;
  reload: ReturnType<typeof vi.fn>;
  stop: ReturnType<typeof vi.fn>;
}

describe("browser address", () => {
  it("passes explicit schemes through and defaults bare hosts to https", () => {
    expect(browserAddress("http://localhost:3000/app")).toBe("http://localhost:3000/app");
    expect(browserAddress("example.com")).toBe("https://example.com");
    expect(browserAddress("docs.example.com/guide?a=1")).toBe("https://docs.example.com/guide?a=1");
    expect(browserAddress("localhost:5173")).toBe("https://localhost:5173");
  });

  it("searches for anything that is not a URL", () => {
    expect(browserAddress("neptune dark spot")).toBe("https://duckduckgo.com/?q=neptune%20dark%20spot");
    expect(browserAddress("")).toBeNull();
  });
});

describe("browser tabs in the strip and pane", () => {
  let root: Root;
  let container: HTMLDivElement;
  const created: FakeWebview[] = [];

  function fakeWebview(): FakeWebview {
    const element = document.createElement("div") as unknown as FakeWebview;
    element.loadURL = vi.fn(async () => {});
    element.getURL = () => "about:blank";
    element.canGoBack = () => false;
    element.canGoForward = () => false;
    element.goBack = vi.fn();
    element.goForward = vi.fn();
    element.reload = vi.fn();
    element.stop = vi.fn();
    return element;
  }

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    created.length = 0;
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
    const createElement = document.createElement.bind(document);
    vi.spyOn(document, "createElement").mockImplementation(((tag: string) => {
      if (tag === "webview") {
        const element = fakeWebview();
        created.push(element);
        return element;
      }
      return createElement(tag as keyof HTMLElementTagNameMap);
    }) as typeof document.createElement);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("renders a browser chip next to the add control and reports clicks", async () => {
    const onSelectBrowser = vi.fn();
    const onCloseBrowser = vi.fn();
    const onNewBrowser = vi.fn();
    await act(async () => root.render(
      <TabBar
        tabs={[]}
        activePath={null}
        ariaLabel="Open editor tabs"
        onSelect={() => {}}
        onClose={() => {}}
        onMoveTab={() => {}}
        browsers={[{ ...browserTab, url: "https://example.com", title: "Example Domain" }]}
        activeBrowserTabID="browser-1"
        onSelectBrowser={onSelectBrowser}
        onCloseBrowser={onCloseBrowser}
        onNewBrowser={onNewBrowser}
      />
    ));

    const chip = container.querySelector<HTMLElement>(".tab.browser")!;
    expect(chip.textContent).toContain("Example Domain");
    expect(chip.classList.contains("active")).toBe(true);
    await act(async () => chip.click());
    expect(onSelectBrowser).toHaveBeenCalledWith("browser-1");

    await act(async () => chip.querySelector<HTMLButtonElement>(".tab-close")!.click());
    expect(onCloseBrowser).toHaveBeenCalledWith("browser-1");

    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="New browser tab"]')!.click());
    expect(onNewBrowser).toHaveBeenCalledOnce();
  });

  async function nextFrame(): Promise<void> {
    await act(async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    });
  }

  it("opens with no browser work at all and creates the guest on first navigation", async () => {
    const onState = vi.fn();
    await act(async () => root.render(<BrowserPane tab={browserTab} active onState={onState} />));
    await nextFrame();

    // A new tab must not pay for a guest: nothing is attached until it loads.
    expect(created).toHaveLength(0);
    expect(container.querySelector(".browser-placeholder")).not.toBeNull();
    expect(container.querySelector<HTMLButtonElement>('[aria-label="Back"]')!.disabled).toBe(true);

    const input = container.querySelector<HTMLInputElement>(".browser-address input")!;
    expect(input.placeholder).toBe("Search or enter a URL");
    await act(async () => { input.focus(); type(input, "example.com"); });
    await act(async () => {
      container.querySelector<HTMLFormElement>(".browser-address")!
        .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });

    expect(onState).toHaveBeenCalledWith({ url: "https://example.com", error: null, loading: true });
    expect(input.value).toBe("https://example.com");
    await nextFrame();
    expect(created).toHaveLength(1);
    expect(created[0]!.getAttribute("src")).toBe("https://example.com");
  });

  it("veils the frame while a page is loading instead of flashing blank", async () => {
    await act(async () => root.render(
      <BrowserPane tab={{ ...browserTab, url: "https://example.com", loading: true }} active onState={() => {}} />
    ));

    const veil = container.querySelector(".browser-placeholder.loading");
    expect(veil).not.toBeNull();
    expect(veil!.textContent).toContain("Loading example.com");
    expect(veil!.querySelector(".browser-spinner")).not.toBeNull();
    expect(created).toHaveLength(0);
  });

  it("reloads and stops the guest once it exists", async () => {
    await act(async () => root.render(<BrowserPane tab={browserTab} active onState={() => {}} />));
    const input = container.querySelector<HTMLInputElement>(".browser-address input")!;
    await act(async () => type(input, "example.com"));
    await act(async () => {
      container.querySelector<HTMLFormElement>(".browser-address")!
        .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });
    await nextFrame();

    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Reload"]')!.click());
    expect(created[0]!.reload).toHaveBeenCalledOnce();

    await act(async () => root.render(
      <BrowserPane tab={{ ...browserTab, url: "https://example.com", loading: true }} active onState={() => {}} />
    ));
    expect(container.querySelector('[aria-label="Reload"]')).toBeNull();
    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Stop loading"]')!.click());
    expect(created[0]!.stop).toHaveBeenCalledOnce();
  });

  it("loads a URL supplied when the tab is opened", async () => {
    await act(async () => root.render(<BrowserPane tab={{ ...browserTab, url: "https://example.com/initial" }} active onState={() => {}} />));
    await nextFrame();
    expect(created).toHaveLength(1);
    expect(created[0]!.getAttribute("src")).toBe("https://example.com/initial");
  });

  it("preserves a main-frame error through stop-loading and ignores subframe failures", async () => {
    function Harness() {
      const [tab, setTab] = useState({ ...browserTab, url: "http://localhost:54321/missing" });
      return <BrowserPane tab={tab} active onState={(patch) => setTab((current) => ({ ...current, ...patch }))} />;
    }
    await act(async () => root.render(<Harness />));
    await nextFrame();
    const fail = (isMainFrame: boolean) => Object.assign(new Event("did-fail-load"), { isMainFrame, errorCode: -102, errorDescription: "Connection refused" });
    await act(async () => { created[0]!.dispatchEvent(fail(false)); });
    expect(container.textContent).not.toContain("Connection refused");
    await act(async () => {
      created[0]!.dispatchEvent(fail(true));
      created[0]!.dispatchEvent(new Event("did-stop-loading"));
    });
    expect(container.querySelector(".browser-placeholder.error")?.textContent).toContain("Connection refused");
    expect(container.querySelector<HTMLInputElement>("input")!.value).toBe("http://localhost:54321/missing");
    await act(async () => { created[0]!.dispatchEvent(new Event("did-start-loading")); });
    expect(container.querySelector(".browser-placeholder.error")).toBeNull();
  });

  it("rejects privileged URLs without creating a guest", async () => {
    const onState = vi.fn();
    await act(async () => root.render(<BrowserPane tab={{ ...browserTab, url: "file:///private/tmp/secret" }} active onState={onState} />));
    await nextFrame();
    expect(created).toHaveLength(0);
    expect(onState).toHaveBeenCalledWith(expect.objectContaining({ loading: false, error: expect.stringContaining("HTTP") }));
    for (const url of ["file:///tmp/private", "javascript:alert(1)", "data:text/html,hello", "https://user:pass@example.com", "custom://x"]) expect(browserAddress(url)).toBeNull();
  });

  it("cancels deferred guest creation when its tab closes", async () => {
    await act(async () => root.render(<BrowserPane tab={{ ...browserTab, url: "https://example.com" }} active onState={() => {}} />));
    await act(async () => root.render(null));
    await nextFrame();
    expect(created).toHaveLength(0);
  });

  it("retries the submitted URL after stopping before the guest attaches", async () => {
    let nextID = 0;
    const frames = new Map<number, FrameRequestCallback>();
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => { frames.set(++nextID, callback); return nextID; });
    vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
    function Harness() {
      const [tab, setTab] = useState(browserTab);
      return <BrowserPane tab={tab} active onState={(patch) => setTab((current) => ({ ...current, ...patch }))} />;
    }
    await act(async () => root.render(<Harness />));
    const input = container.querySelector<HTMLInputElement>("input")!;
    await act(async () => type(input, "https://example.com/retry"));
    await act(async () => { container.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })); });
    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Stop loading"]')!.click());
    expect(frames.size).toBe(0);
    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Reload"]')!.click());
    await act(async () => { for (const callback of frames.values()) callback(0); frames.clear(); });
    expect(created).toHaveLength(1);
    expect(created[0]!.getAttribute("src")).toBe("https://example.com/retry");
    expect(container.querySelector(".browser-placeholder.error")).toBeNull();
  });

});
