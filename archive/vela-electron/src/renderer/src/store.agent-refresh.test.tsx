import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo } from "@shared/types";
import { StoreProvider, useStore } from "./store";

type Store = ReturnType<typeof useStore>;

let store: Store;
let agentList: { id: string; name: string }[];
let modelList: { id: string; providerID: string; name: string; variants?: string[]; limit?: { context: number } }[];
let messageHandler: ((msg: unknown) => void) | null;

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

function api(): typeof window.vela {
  return {
    platform: "darwin",
    onMessage: (handler: (msg: unknown) => void) => {
      messageHandler = handler;
      return () => {
        messageHandler = null;
      };
    },
    health: async () => true,
    takePendingPaths: async () => [],
    state: async () => null,
    activeSessions: async () => [],
    models: async () => modelList,
    modelDefault: async () => null,
    sessionSelection: async () => null,
    agents: async () => agentList,
    sessions: async () => [],
    openSession: async (directory: string, generation: number) => info(directory, generation),
    closeSession: async () => {},
    readFile: async () => "content",
    listDir: async () => [],
    recoveryRecords: async () => [],
    switchAgent: async () => {}
  } as unknown as typeof window.vela;
}

function updatedEvent(type: string): { kind: "event"; type: string; data: unknown } {
  return {
    kind: "event",
    type,
    data: { id: `${type}-1`, type, created: Date.now(), data: {}, location: { directory: "/one" } }
  };
}

function chatEvent(type: string, data: Record<string, unknown>): { kind: "event"; type: string; data: unknown } {
  return {
    kind: "event",
    type,
    data: { id: `${type}-${Date.now()}`, type, created: Date.now(), data, location: { directory: "/one" } }
  };
}

describe("picker catalogs refresh on server update events", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    agentList = [];
    modelList = [];
    messageHandler = null;
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

  it("refetches agents when agent.updated arrives", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    expect(store.agents).toHaveLength(0);

    agentList = [
      { id: "build", name: "Build" },
      { id: "plan", name: "Plan" }
    ];
    expect(messageHandler).not.toBeNull();
    await act(async () => {
      messageHandler!(updatedEvent("agent.updated"));
      await new Promise((resolve) => setTimeout(resolve, 30));
    });

    expect(store.agents.map((agent) => agent.id)).toEqual(["build", "plan"]);
  });

  it("refetches models when catalog.updated arrives", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    expect(store.models).toHaveLength(0);

    modelList = [{ id: "m1", providerID: "p1", name: "One" }];
    expect(messageHandler).not.toBeNull();
    await act(async () => {
      messageHandler!(updatedEvent("catalog.updated"));
      await new Promise((resolve) => setTimeout(resolve, 30));
    });

    expect(store.models.map((model) => model.id)).toEqual(["m1"]);

    modelList = [{ id: "m1", providerID: "p1", name: "One", variants: ["high"], limit: { context: 200000 } }];
    await act(async () => store.loadModels());
    expect(store.models[0]).toMatchObject({ variants: ["high"], limit: { context: 200000 } });
  });

  it("projects live text deltas into the visible transcript immediately", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));

    await act(async () => {
      messageHandler!(chatEvent("session.step.started", { sessionID: "session-1", assistantMessageID: "assistant-1" }));
      messageHandler!(chatEvent("session.text.started", { sessionID: "session-1", assistantMessageID: "assistant-1", ordinal: 0 }));
      messageHandler!(chatEvent("session.text.delta", { sessionID: "session-1", assistantMessageID: "assistant-1", ordinal: 0, delta: "Streaming now" }));
    });

    const assistant = store.transcript.find((item) => item.kind === "assistant");
    const text = assistant?.kind === "assistant" ? assistant.parts.find((part) => part.kind === "text") : null;
    expect(text?.kind === "text" ? text.text : null).toBe("Streaming now");
  });
});
