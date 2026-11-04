import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, ProviderUsageResult, SessionInfo, SessionUsage } from "@shared/types";
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

function usageFor(id: string): SessionUsage {
  return {
    cost: 1.25,
    tokens: {
      input: id.length * 1000,
      output: 500,
      reasoning: 0,
      cache: { read: 200, write: 0 }
    }
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
      transcript: [{
        kind: "user",
        id: `user-${sessionID}`,
        text: "hello"
      }],
      todos: [],
      usage: usageFor(sessionID)
    }),
    closeSession: async () => {},
    readFile: async () => "content",
    listDir: async () => [],
    listPermissions: async () => [],
    ...overrides
  } as VelaApi;
}

describe("store panel usage hydration", () => {
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

  it("hydrates usage and history for a model panel added via the add control", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.addModelPanel("/one"));
    expect(store.sessionUsage).toEqual(usageFor(store.activeSessionID!));
    expect(store.transcript).toHaveLength(1);
    expect(store.transcript[0].kind).toBe("user");
  });

  it("rejects primitive event tokens while retaining array token acceptance", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.addModelPanel("/one"));
    const before = store.sessionUsage;
    const send = async (id: string, tokens: unknown) => act(async () => {
      messageHandler!({ kind: "event", type: "session.usage.updated", data: {
        id, created: Date.now(), data: { sessionID: store.activeSessionID!, tokens }
      } });
    });
    await send("primitive-tokens", "tokens");
    expect(store.sessionUsage).toEqual(before);
    await send("array-tokens", Object.assign([], { input: 13 }));
    expect(store.sessionUsage).toEqual({ cost: 0, tokens: { input: 13, output: 0, reasoning: 0, cache: { read: 0, write: 0 } } });
  });

  it("preserves event token/cache/cost read order and a single cache snapshot", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.addModelPanel("/one"));
    const reads: string[] = [];
    const data = {
      sessionID: store.activeSessionID!,
      get cost() { reads.push("cost"); return 1; },
      get tokens() { reads.push("tokens"); return {
        get input() { reads.push("input"); return 10; },
        get output() { reads.push("output"); return 2; },
        get reasoning() { reads.push("reasoning"); return 3; },
        get cache() { reads.push("cache"); return {
          get read() { reads.push("cache.read"); return 4; },
          get write() { reads.push("cache.write"); return 5; }
        }; }
      }; }
    };
    await act(async () => messageHandler!({ kind: "event", type: "session.usage.updated", data: { id: "accessor-usage", created: Date.now(), data } }));
    expect(store.sessionUsage).toEqual({ cost: 1, tokens: { input: 10, output: 2, reasoning: 3, cache: { read: 4, write: 5 } } });
    expect(reads).toEqual(["tokens", "cache", "cost", "input", "output", "reasoning", "cache.read", "cache.write"]);
  });

  it("hydrates usage for the replacement session when a panel swaps workspaces", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.addModelPanel("/one"));
    const first = store.session!;
    await act(async () => { void store.changePanelDirectory(first.workspace, "/two"); });
    await act(async () => {});
    expect(store.panels).toHaveLength(1);
    expect(store.sessionUsage).toEqual(usageFor(store.activeSessionID!));
    expect(store.transcript).toHaveLength(1);
  });

  it("hydrates usage for the folder-picker add path", async () => {
    window.vela = api({ selectFolder: async () => info("/picked", 7) });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.selectAddPanel());
    expect(store.activeSessionID).toBe("session-7");
    expect(store.sessionUsage).toEqual(usageFor("session-7"));
  });

  it("refreshes usage and snapshots the baseline when compaction ends, with a delayed backstop", async () => {
    vi.useFakeTimers();
    try {
      const postCompaction: SessionUsage = {
        cost: 0,
        tokens: { input: 12_000, output: 100, reasoning: 0, cache: { read: 0, write: 0 } }
      };
      const sessionUsage = vi.fn(async () => postCompaction);
      window.vela = api({ sessionUsage });
      await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
      await act(async () => store.addModelPanel("/one"));
      const sessionID = store.activeSessionID!;
      const workspaceID = store.session!.workspace.id;

      await act(async () => {
        messageHandler!({
          kind: "event",
          type: "session.usage.updated",
          data: {
            id: "u-high",
            created: Date.now(),
            data: {
              sessionID,
              cost: 2,
              tokens: { input: 190_000, output: 9_000, reasoning: 0, cache: { read: 0, write: 0 } }
            }
          }
        });
      });
      expect(store.sessionUsage?.tokens.input).toBe(190_000);

      sessionUsage.mockClear();
      await act(async () => {
        messageHandler!({
          kind: "event",
          type: "session.compaction.ended",
          data: { id: "c-end", created: Date.now(), data: { sessionID } }
        });
      });
      // Immediate poll applied the post-compaction snapshot against the
      // pre-compaction baseline, so context fill drops instead of sticking.
      expect(sessionUsage).toHaveBeenCalledTimes(1);
      expect(store.sessionUsage?.tokens.input).toBe(12_000);
      expect(store.panelViews[workspaceID]?.compactionBaseline).toBe(190_000);

      // Delayed backstop re-polls in case the first read raced the commit.
      await act(async () => { vi.advanceTimersByTime(8_000); });
      expect(sessionUsage).toHaveBeenCalledTimes(2);
      expect(store.sessionUsage?.tokens.input).toBe(12_000);
    } finally {
      vi.useRealTimers();
    }
  });

  it("re-polls usage after a manual compact command, promptly and as a backstop", async () => {
    vi.useFakeTimers();
    try {
      const sessionUsage = vi.fn(async () => usageFor("session-1"));
      window.vela = api({ sessionUsage, runCommand: async () => {} });
      await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
      await act(async () => store.addModelPanel("/one"));

      sessionUsage.mockClear();
      await act(async () => { await store.runCommand("compact"); });
      await act(async () => { vi.advanceTimersByTime(1_500); });
      expect(sessionUsage).toHaveBeenCalledTimes(1);
      await act(async () => { vi.advanceTimersByTime(8_500); });
      expect(sessionUsage).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps the newest provider usage refresh when requests finish out of order", async () => {
    let resolveFirst!: (value: ProviderUsageResult[]) => void;
    let resolveSecond!: (value: ProviderUsageResult[]) => void;
    const providerUsage = vi.fn()
      .mockImplementationOnce(() => new Promise<ProviderUsageResult[]>((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise<ProviderUsageResult[]>((resolve) => { resolveSecond = resolve; }));
    window.vela = api({ providerUsage });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));

    let first!: Promise<void>;
    let second!: Promise<void>;
    await act(async () => {
      first = store.refreshProviderUsage();
      second = store.refreshProviderUsage();
    });
    const current = [{ provider: "openai", displayName: "OpenAI", status: "ok", snapshot: null }] satisfies ProviderUsageResult[];
    const stale = [{ provider: "anthropic", displayName: "Claude", status: "ok", snapshot: null }] satisfies ProviderUsageResult[];
    await act(async () => {
      resolveSecond(current);
      await second;
      resolveFirst(stale);
      await first;
    });

    expect(store.providerUsage).toEqual(current);
    expect(store.providerUsageLoading).toBe(false);
  });
});
