import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, SessionInfo, TranscriptItem } from "@shared/types";
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

function transcriptEvent(sessionID: string, text: string): void {
  messageHandler!({
    kind: "event",
    type: "session.synthetic",
    data: { sessionID, text }
  });
}

function usageEvent(sessionID: string, cost: number): void {
  messageHandler!({
    kind: "event",
    type: "session.usage.updated",
    data: {
      sessionID,
      cost,
      tokens: { input: cost, output: 0, reasoning: 0, cache: { read: 0, write: 0 } }
    }
  });
}

async function flush(): Promise<void> {
  await act(async () => new Promise((resolve) => setTimeout(resolve, 30)));
}

describe("open panel retention", () => {
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

  it("never evicts open panel transcripts or usage when subagent streams flood in", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.addModelPanel("/two"));
    await act(async () => store.addModelPanel("/three"));
    expect(store.panels).toHaveLength(3);

    await act(async () => {
      for (const panel of store.panels) {
        transcriptEvent(panel.id, `hello from ${panel.id}`);
        usageEvent(panel.id, 10);
      }
      for (let index = 1; index <= 6; index += 1) {
        transcriptEvent(`subagent-${index}`, "sub");
        usageEvent(`subagent-${index}`, index);
      }
    });
    await flush();

    for (const panel of store.panels) {
      const view = store.panelViews[panel.workspace.id];
      expect(view.transcript.length).toBeGreaterThan(0);
      expect(view.sessionUsage?.cost).toBe(10);
    }
  });

  it("re-hydrates an open panel whose transcript record is missing when focused", async () => {
    const replay: TranscriptItem[] = [{ kind: "user", id: "user-1", text: "earlier prompt" }];
    const openSessionById = vi.fn(async (sessionID: string) => ({
      session: { ...info("/one", 1), id: sessionID },
      transcript: replay,
      todos: [],
      usage: { cost: 5, tokens: { input: 5, output: 0, reasoning: 0, cache: { read: 0, write: 0 } } }
    }));
    window.vela = api({ openSessionById });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.addModelPanel("/two"));
    const first = store.panels[0];

    await act(async () => store.focusSession(first.id));
    await flush();

    expect(openSessionById).toHaveBeenCalledWith(first.id, expect.any(Number));
    const view = store.panelViews[first.workspace.id];
    expect(view.transcript).toEqual(replay);
    expect(view.sessionUsage?.cost).toBe(5);
  });

  it("re-hydrates a closed panel whose record was evicted while closed when reopened", async () => {
    const replay: TranscriptItem[] = [{ kind: "user", id: "user-2", text: "older history" }];
    const openSessionById = vi.fn(async (sessionID: string) => ({
      session: { ...info("/one", 1), id: sessionID },
      transcript: replay,
      todos: [],
      usage: null
    }));
    window.vela = api({ openSessionById });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    const first = store.panels[0];
    await act(async () => { transcriptEvent(first.id, "live"); });
    await flush();
    await act(async () => store.addModelPanel("/two"));
    await act(async () => store.closePanel(first.id));

    await act(async () => {
      for (let index = 1; index <= 6; index += 1) {
        transcriptEvent(`subagent-${index}`, "sub");
      }
    });
    await flush();

    await act(async () => store.reopenSession(first.id));
    await flush();

    expect(openSessionById).toHaveBeenCalledWith(first.id, expect.any(Number), undefined, true);
    const reopened = store.panels.find((panel) => panel.id === first.id);
    expect(reopened).toBeDefined();
    expect(store.panelViews[reopened!.workspace.id].transcript).toEqual(replay);
  });
});
