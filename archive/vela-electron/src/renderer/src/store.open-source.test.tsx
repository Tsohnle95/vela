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
    ...overrides
  } as VelaApi;
}

describe("store open-source ui-command", () => {
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

  it("opens the containing directory as a session when none is active", async () => {
    const openSession = vi.fn(async (directory: string, generation: number) => info(directory, generation));
    const readFile = vi.fn(async () => "content");
    window.vela = api({ openSession, readFile });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    expect(store.session).toBeNull();

    await act(async () => messageHandler!({
      kind: "ui-command",
      command: "open-source",
      path: "/work/project/src/styles/main.css",
      line: 24
    }));

    expect(openSession).toHaveBeenCalledWith("/work/project/src/styles", expect.any(Number));
    expect(store.session?.directory).toBe("/work/project/src/styles");
    expect(store.tabs.map((tab) => tab.path)).toEqual(["main.css"]);
    expect(store.activePath).toBe("main.css");
    expect(readFile).toHaveBeenCalledWith(store.session!.workspace, "main.css");
  });

  it("ignores a relative open-source path when no session is active", async () => {
    const openSession = vi.fn(async (directory: string, generation: number) => info(directory, generation));
    window.vela = api({ openSession });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));

    await act(async () => messageHandler!({
      kind: "ui-command",
      command: "open-source",
      path: "src/styles/main.css",
      line: 24
    }));

    expect(openSession).not.toHaveBeenCalled();
    expect(store.session).toBeNull();
    expect(store.tabs).toEqual([]);
  });

  it("opens an app source as a standalone tab without replacing the active workspace", async () => {
    const openSession = vi.fn(async (directory: string, generation: number) => info(directory, generation));
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone",
      path: "/vela/src/renderer/styles/main.scss",
      content: ".main {}"
    }));
    window.vela = api({ openSession, openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/luno"));
    const workspace = store.session!.workspace;
    const sessionID = store.session!.id;
    openSession.mockClear();

    await act(async () => messageHandler!({
      kind: "ui-command",
      command: "open-source",
      path: "/vela/src/renderer/styles/main.scss",
      line: 3
    }));

    expect(openSession).not.toHaveBeenCalled();
    expect(store.session?.id).toBe(sessionID);
    expect(store.session?.workspace).toBe(workspace);
    expect(openExternal).toHaveBeenCalledWith(workspace, "/vela/src/renderer/styles/main.scss");
    expect(store.tabs.map((tab) => tab.path)).toEqual(["/vela/src/renderer/styles/main.scss"]);
    expect(store.tabs[0].standalone).toBe(true);
    expect(store.activePath).toBe("/vela/src/renderer/styles/main.scss");
  });

  it("opens an app source as a normal tab when the active workspace contains it", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "relative",
      rel: "src/renderer/styles/main.scss",
      content: ".main {}"
    }));
    const readFile = vi.fn(async () => ".main {}");
    window.vela = api({ openExternal, readFile });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/vela"));

    await act(async () => messageHandler!({
      kind: "ui-command",
      command: "open-source",
      path: "/vela/src/renderer/styles/main.scss",
      line: 8
    }));

    expect(openExternal).toHaveBeenCalledWith(store.session!.workspace, "/vela/src/renderer/styles/main.scss");
    expect(readFile).toHaveBeenCalledWith(store.session!.workspace, "src/renderer/styles/main.scss");
    expect(store.tabs.map((tab) => tab.path)).toEqual(["src/renderer/styles/main.scss"]);
    expect(store.tabs[0].standalone).toBeUndefined();
  });

  it("ignores a relative source command when a session is active", async () => {
    const openExternal = vi.fn();
    window.vela = api({ openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/work"));

    await act(async () => messageHandler!({
      kind: "ui-command",
      command: "open-source",
      path: "src/main.css",
      line: 8
    }));

    expect(openExternal).not.toHaveBeenCalled();
    expect(store.tabs).toEqual([]);
  });
});
