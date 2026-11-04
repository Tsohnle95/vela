import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, SessionSelection } from "@shared/types";
import { StoreProvider, useStore } from "./store";
import { Composer } from "./components/Composer";

type Store = ReturnType<typeof useStore>;

let store: Store;
let selection: SessionSelection | null;

function Probe(): ReactNode {
  store = useStore();
  return null;
}

function info(directory: string, generation: number, agent?: string): SessionInfo {
  return {
    id: `session-${generation}`,
    directory,
    workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation },
    ...(agent ? { agent } : {})
  };
}

const AGENTS = [
  { id: "build", name: "Build" },
  { id: "plan", name: "Plan" },
  { id: "general", name: "General" }
];

const MODELS = [
  { id: "m1", providerID: "p1", name: "One" },
  { id: "m2", providerID: "p1", name: "Two" }
];

function api(overrides: Record<string, unknown> = {}): typeof window.vela {
  return {
    platform: "darwin",
    onMessage: () => () => {},
    health: async () => true,
    takePendingPaths: async () => [],
    state: async () => null,
    activeSessions: async () => [],
    models: async () => MODELS,
    modelDefault: async () => ({ id: "m2", providerID: "p1", name: "Two" }),
    sessionSelection: async () => selection,
    agents: async () => AGENTS,
    sessions: async () => [],
    openSession: async (directory: string, generation: number, agent?: string) => info(directory, generation, agent),
    closeSession: async () => {},
    readFile: async () => "content",
    listDir: async () => [],
    recoveryRecords: async () => [],
    switchAgent: async () => {},
    ...overrides
  } as unknown as typeof window.vela;
}

describe("agent and model picker state across sessions", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    selection = null;
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

  it("ignores and clears a stored DeepSeek runtime preference", async () => {
    window.localStorage.setItem("runtimeID", "deepseek");
    const openSession = vi.fn(async (directory: string, generation: number) => info(directory, generation));
    window.vela = api({ openSession });

    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));

    expect(openSession).toHaveBeenCalledWith("/one", expect.any(Number));
    expect(window.localStorage.getItem("runtimeID")).toBeNull();
  });

  it("falls back to build instead of carrying the previous session's plan selection", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    selection = { agent: { id: "plan", name: "plan" } };
    await act(async () => store.openSession("/one"));
    expect(store.currentAgent?.id).toBe("plan");

    selection = null;
    await act(async () => store.openSession("/two"));
    expect(store.currentAgent?.id).toBe("build");
  });

  it("shows the agent a session was created with when the session has no live selection", async () => {
    window.vela = api({
      openSession: async (directory: string, generation: number) => info(directory, generation, "plan")
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    expect(store.currentAgent?.id).toBe("plan");
  });

  it("shows the reopened session's selection, not the previous session's agent", async () => {
    window.vela = api({
      openSessionById: async (sessionID: string) => ({
        session: { ...info("/two", 2), id: sessionID },
        transcript: [],
        todos: [],
        usage: null
      })
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    selection = { agent: { id: "plan", name: "plan" } };
    await act(async () => store.openSession("/one"));
    expect(store.currentAgent?.id).toBe("plan");

    selection = { agent: { id: "general", name: "general" } };
    await act(async () => store.reopenSession("older"));
    expect(store.currentAgent?.id).toBe("general");
  });

  it("updates the displayed agent immediately after switchAgent", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    expect(store.currentAgent?.id).toBe("build");

    await act(async () => store.switchAgent("plan"));
    expect(store.currentAgent?.id).toBe("plan");
  });

  it("falls back to the default model instead of carrying the previous session's model", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    selection = { model: { id: "m1", providerID: "p1", name: "One" } };
    await act(async () => store.openSession("/one"));
    expect(store.currentModel?.id).toBe("m1");

    selection = null;
    await act(async () => store.openSession("/two"));
    expect(store.currentModel?.id).toBe("m2");
  });

  it("uses OpenCode's default model when the session selection is unavailable", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    selection = { model: { id: "m1", providerID: "p1", name: "One" } };
    await act(async () => store.openSession("/one"));
    expect(store.currentModel?.id).toBe("m1");

    selection = null;
    await act(async () => store.loadModels());
    expect(store.currentModel?.id).toBe("m2");
  });

  it("shows the correct agent in the picker button after switching sessions", async () => {
    window.vela = api();
    function Panel(): ReactNode {
      store = useStore();
      return <Composer />;
    }
    await act(async () => root.render(<StoreProvider><Panel /></StoreProvider>));
    const initialLabel = container.querySelector('button[title="Change agent"]')?.textContent ?? "";
    expect(initialLabel).toContain("Agent");

    selection = { agent: { id: "plan", name: "plan" } };
    await act(async () => store.openSession("/one"));
    const planLabel = container.querySelector('button[title="Change agent"]')?.textContent ?? "";
    expect(planLabel).toContain("Plan");

    selection = null;
    await act(async () => store.openSession("/two"));
    const buildLabel = container.querySelector('button[title="Change agent"]')?.textContent ?? "";
    expect(buildLabel).toContain("Build");
  });

  it("reopens a child session with its parentID so the header can navigate back", async () => {
    window.vela = api({
      openSessionById: async (sessionID: string) => ({
        session: { ...info("/root", 1), id: sessionID, parentID: "session-parent", title: "Run the tests", agent: "build" },
        transcript: [],
        todos: [],
        usage: null
      })
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/root"));
    expect(store.session?.parentID).toBeUndefined();

    await act(async () => store.reopenSession("session-child"));
    expect(store.session?.id).toBe("session-child");
    expect(store.session?.parentID).toBe("session-parent");
    expect(store.session?.title).toBe("Run the tests");

    await act(async () => store.reopenSession("session-parent"));
    expect(store.session?.id).toBe("session-parent");
  });

  it("does not attach an emitted replacement session before reopen completes", async () => {
    let messageHandler: ((message: { kind: "session"; session: SessionInfo }) => void) | null = null;
    let resolveOpen!: (value: {
      session: SessionInfo;
      transcript: [];
      todos: [];
      usage: null;
    }) => void;
    const child = { ...info("/root", 2), id: "session-child", parentID: "session-parent", title: "Inspect renderer" };
    const opening = new Promise<{
      session: SessionInfo;
      transcript: [];
      todos: [];
      usage: null;
    }>((resolve) => { resolveOpen = resolve; });
    window.vela = api({
      onMessage: (handler: typeof messageHandler) => {
        messageHandler = handler;
        return () => { messageHandler = null; };
      },
      openSessionById: async () => opening
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/root"));

    let pending!: Promise<SessionInfo | null>;
    await act(async () => {
      pending = store.reopenSession("session-child");
      await Promise.resolve();
    });
    await act(async () => messageHandler?.({ kind: "session", session: child }));
    expect(store.panels.map((panel) => panel.id)).toEqual(["session-1"]);

    await act(async () => {
      resolveOpen({ session: child, transcript: [], todos: [], usage: null });
      await pending;
    });
    expect(store.panels.map((panel) => panel.id)).toEqual(["session-child"]);
  });
});
