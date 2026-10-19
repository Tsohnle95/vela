import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo } from "@shared/types";
import { BrowserStoreProvider } from "./browser-store-provider";
import { BrowserTabsProvider, useBrowserTabActions, useBrowserTabState } from "./browser-tabs-context";
import { useStore } from "./store";

type VelaApi = Window["vela"];

let store: ReturnType<typeof useStore>;
let actions: ReturnType<typeof useBrowserTabActions>;
let browserState: ReturnType<typeof useBrowserTabState>;
let storeRenders: number;

function StoreProbe(): ReactNode {
  store = useStore();
  storeRenders++;
  return null;
}

function BrowserProbe(): ReactNode {
  actions = useBrowserTabActions();
  browserState = useBrowserTabState();
  return null;
}

function bridge(): VelaApi {
  const sessions = new Map<string, SessionInfo>();
  const fixture: Partial<VelaApi> = {
    platform: "darwin",
    onMessage: () => () => {},
    health: async () => true,
    takePendingPaths: async () => [],
    activeSessions: async () => [],
    runtimes: async () => [],
    state: async () => null,
    models: async () => [],
    modelDefault: async () => null,
    sessionSelection: async () => null,
    agents: async () => [],
    sessions: async () => [],
    savedWorkspaces: async () => ({ initialized: true, workspaces: [] }),
    openSession: async (directory, generation) => {
      const session = { id: `session-${generation}`, directory,
        workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation } };
      sessions.set(session.id, session);
      return session;
    },
    openSessionById: async (id) => ({ session: sessions.get(id)!, transcript: [], todos: [], usage: null }),
    inboxList: async () => [],
    recoveryRecords: async () => [],
    closeSession: async () => {},
    readFile: async (_workspace, path) => path === "missing.ts" ? null : "content",
    listDir: async () => []
  };
  return fixture as VelaApi;
}

describe("browser and Store integration", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.useFakeTimers();
    window.localStorage.clear();
    window.vela = bridge();
    storeRenders = 0;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(
      <BrowserTabsProvider>
        <BrowserStoreProvider><StoreProbe /><BrowserProbe /></BrowserStoreProvider>
      </BrowserTabsProvider>
    ));
    await act(async () => store.openSession("/foreground"));
    expect(store.toasts.filter((toast) => toast.tone === "error")).toEqual([]);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("isolates 100 browser title/loading updates from actual Store consumers", async () => {
    let id = "";
    act(() => { id = actions.openBrowserTab("https://example.com"); });
    const baseline = storeRenders;
    for (let index = 0; index < 100; index++) {
      await act(async () => actions.updateBrowserTab(id, { title: `Title ${index}`, loading: index % 2 === 0 }));
    }
    expect(browserState.browserTabs[0]).toMatchObject({ title: "Title 99", loading: false });
    expect(storeRenders).toBe(baseline);
  });

  it("returns to files after an Explorer open and reselecting the current file", async () => {
    act(() => { actions.openBrowserTab(); });
    await act(async () => store.openFile("current.ts"));
    expect(store.activePath).toBe("current.ts");
    expect(browserState.activeBrowserTabID).toBeNull();
    let id = "";
    act(() => { id = actions.openBrowserTab(); });
    expect(browserState.activeBrowserTabID).toBe(id);
    act(() => store.setActive("current.ts"));
    expect(browserState.activeBrowserTabID).toBeNull();
    expect(store.activePath).toBe("current.ts");
  });

  it("retains browser focus on failed and background workspace file opens", async () => {
    await act(async () => store.addModelPanel("/background"));
    const background = store.session!.workspace;
    const foreground = store.panels.find((panel) => panel.directory === "/foreground")!;
    act(() => store.focusSession(foreground.id));
    let id = "";
    act(() => { id = actions.openBrowserTab(); });
    await act(async () => store.openFile("missing.ts"));
    expect(browserState.activeBrowserTabID).toBe(id);
    expect(store.toasts.at(-1)?.text).toBe("Could not read missing.ts");
    await act(async () => store.openFile("background.ts", background));
    expect(browserState.activeBrowserTabID).toBe(id);
    expect(store.session?.id).toBe(foreground.id);
    expect(store.tabs.some((tab) => tab.path === "background.ts")).toBe(false);
    const backgroundPanel = store.panels.find((panel) => panel.workspace.id === background.id)!;
    act(() => store.focusSession(backgroundPanel.id));
    expect(store.tabs.some((tab) => tab.path === "background.ts")).toBe(true);
  });
});
