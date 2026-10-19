import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, SessionInfo } from "@shared/types";
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

async function seedChanges(): Promise<void> {
  await act(async () => store.openSession("/one"));
  const workspace = store.session!.workspace;
  const sessionID = store.session!.id;
  await act(async () => messageHandler!({
    kind: "file-update",
    file: { workspace, sessionID, path: "a.txt", baseline: { kind: "known", content: "old" }, content: "new", deleted: false }
  }));
  await act(async () => messageHandler!({
    kind: "file-update",
    file: { workspace, sessionID, path: "gone.txt", baseline: { kind: "known", content: "old" }, content: null, deleted: true }
  }));
}

describe("store dismiss changes", () => {
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

  it("dismisses a single change and keeps the rest", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await seedChanges();
    expect([...store.agentFiles.keys()].sort()).toEqual(["a.txt", "gone.txt"]);

    act(() => store.dismissChange("gone.txt"));

    expect([...store.agentFiles.keys()]).toEqual(["a.txt"]);
  });

  it("ignores dismissing an unknown path", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await seedChanges();

    act(() => store.dismissChange("missing.txt"));

    expect([...store.agentFiles.keys()].sort()).toEqual(["a.txt", "gone.txt"]);
  });

  it("dismisses every change at once", async () => {
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await seedChanges();

    act(() => store.dismissChanges());

    expect(store.agentFiles.size).toBe(0);
  });

  it("opens a deleted change as a deleted tab instead of erroring", async () => {
    window.vela = api({ readFile: async () => null });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await seedChanges();
    const errorsBefore = store.toasts.filter((toast) => toast.tone === "error").length;

    await act(async () => store.openFile("gone.txt"));

    const tab = store.tabs.find((t) => t.path === "gone.txt")!;
    expect(tab.deleted).toBe(true);
    expect(tab.baseline).toEqual({ kind: "known", content: "old" });
    expect(store.toasts.filter((toast) => toast.tone === "error")).toHaveLength(errorsBefore);
  });
});
