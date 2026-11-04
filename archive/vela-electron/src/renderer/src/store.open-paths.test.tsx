import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, ExternalOpenResult, SessionInfo } from "@shared/types";
import { StoreProvider, useStore } from "./store";

type Store = ReturnType<typeof useStore>;

let store: Store;
let messageHandler: ((message: BackendMessage) => void) | null;

function Probe(): ReactNode {
  store = useStore();
  return null;
}

function info(directory: string, generation: number): SessionInfo {
  return {
    id: `session-${generation}`,
    directory,
    workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation }
  };
}

function api(overrides: Partial<VelaApi> = {}): VelaApi {
  return {
    platform: "darwin",
    onMessage: (handler) => {
      messageHandler = handler;
      return () => { messageHandler = null; };
    },
    health: async () => true,
    takePendingPaths: async () => [],
    state: async () => null,
    activeSessions: async () => [],
    models: async () => [],
    modelDefault: async () => null,
    sessionSelection: async () => null,
    agents: async () => [],
    sessions: async () => [],
    openSession: async (directory, generation) => info(directory, generation),
    openSessionById: async (sessionID: string) => ({
      session: { ...info("/reopened", 0), id: sessionID },
      transcript: [],
      todos: [],
      usage: null
    }),
    closeSession: async () => {},
    readFile: async () => "content",
    listDir: async () => [],
    externalKind: async () => ({ kind: "missing" }),
    ...overrides
  } as VelaApi;
}

describe("store open paths", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    messageHandler = null;
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("opens a dropped folder as the first session when nothing is open", async () => {
    const externalKind = vi.fn(async () => ({ kind: "directory" as const }));
    window.vela = api({ externalKind });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openPaths(["/dropped"]));

    expect(externalKind).toHaveBeenCalledWith("/dropped");
    expect(store.panels.map((panel) => panel.directory)).toEqual(["/dropped"]);
  });

  it("adds a dropped folder as a new panel when a session is open", async () => {
    const externalKind = vi.fn(async () => ({ kind: "directory" as const }));
    window.vela = api({ externalKind });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openPaths(["/two"]));

    expect(store.panels.map((panel) => panel.directory)).toEqual(["/one", "/two"]);
  });

  it("opens a dropped file as a single-file workspace when nothing is open", async () => {
    const externalKind = vi.fn(async () => ({ kind: "file" as const }));
    const session = info("/parent", 7);
    const openFileWorkspace = vi.fn(async () => ({ session, path: "index.html" }));
    window.vela = api({
      externalKind,
      openFileWorkspace: openFileWorkspace as VelaApi["openFileWorkspace"]
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openPaths(["/parent/index.html"]));

    expect(openFileWorkspace).toHaveBeenCalledWith("/parent/index.html", expect.any(Number));
  });

  it("opens a dropped file as a tab when a session is open", async () => {
    const externalKind = vi.fn(async () => ({ kind: "file" as const }));
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "relative", rel: "index.html", content: "<p>hi"
    }));
    window.vela = api({ externalKind, openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openPaths(["/one/index.html"]));

    expect(store.tabs.map((tab) => tab.path)).toEqual(["index.html"]);
  });

  it("opens a file on an attached session before that session has been focused", async () => {
    const externalKind = vi.fn(async () => ({ kind: "file" as const }));
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "relative", rel: "index.html", content: "<p>hi"
    }));
    window.vela = api({ externalKind, openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => messageHandler!({ kind: "session", session: info("/one", 1) }));
    await act(async () => messageHandler!({
      kind: "ui-command",
      command: "open-paths",
      data: ["/one/index.html"]
    }));

    expect(store.session?.directory).toBe("/one");
    expect(store.tabs.map((tab) => tab.path)).toEqual(["index.html"]);
    expect(store.activePath).toBe("index.html");
    expect(openExternal).toHaveBeenCalledWith(store.session!.workspace, "/one/index.html");
  });

  it("toasts when a dropped path no longer exists", async () => {
    window.vela = api({ externalKind: async () => ({ kind: "missing" as const }) });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openPaths(["/gone"]));

    expect(store.panels).toHaveLength(0);
    expect(store.toasts.some((toast) => toast.tone === "error")).toBe(true);
  });

  it("opens paths delivered as a backend message", async () => {
    const externalKind = vi.fn(async () => ({ kind: "directory" as const }));
    window.vela = api({ externalKind });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => {
      messageHandler!({ kind: "ui-command", command: "open-paths", data: ["/dropped", 42, "."] });
    });

    expect(store.panels.map((panel) => panel.directory)).toEqual(["/dropped"]);
  });

  it("opens paths queued before the renderer was ready", async () => {
    const externalKind = vi.fn(async () => ({ kind: "directory" as const }));
    const takePendingPaths = vi.fn(async () => ["/dropped"]);
    window.vela = api({ externalKind, takePendingPaths });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => {});

    expect(takePendingPaths).toHaveBeenCalledTimes(1);
    expect(store.panels.map((panel) => panel.directory)).toEqual(["/dropped"]);
  });
});
