import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, ProjectInfo, SessionInfo } from "@shared/types";
import { StoreProvider, useStore } from "./store";

type Store = ReturnType<typeof useStore>;

let store: Store;
let messageHandler: ((message: BackendMessage) => void) | null;

function Probe(): ReactNode {
  store = useStore();
  return null;
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
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
    projects: async () => [],
    savedWorkspaces: async () => ({ workspaces: [], initialized: true }),
    saveSavedWorkspaces: async (workspaces: ProjectInfo[]) => workspaces,
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

describe("per-panel workspace selection", () => {
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

  it("persists and removes Vela workspace bookmarks without touching the folder", async () => {
    const selectDirectory = vi.fn(async () => "/saved/workspace");
    const saveSavedWorkspaces = vi.fn(async (workspaces: ProjectInfo[]) => workspaces);
    const deletePath = vi.fn(async () => {});
    const detachPath = vi.fn(async () => {});
    window.vela = api({ selectDirectory, deletePath, detachPath, saveSavedWorkspaces });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));

    await act(async () => store.saveWorkspace());
    expect(store.savedWorkspaces).toEqual([{ directory: "/saved/workspace", name: "workspace" }]);
    expect(JSON.parse(window.localStorage.getItem("vela.savedWorkspaces") ?? "[]")).toEqual(store.savedWorkspaces);
    expect(window.vela.saveSavedWorkspaces).toHaveBeenLastCalledWith(store.savedWorkspaces);

    await act(async () => store.removeWorkspace("/saved/workspace"));
    expect(store.savedWorkspaces).toEqual([]);
    expect(deletePath).not.toHaveBeenCalled();
    expect(detachPath).not.toHaveBeenCalled();
  });

  it("waits for app storage and preserves explicitly removed OpenCode projects", async () => {
    const stored = deferred<{ workspaces: ProjectInfo[]; initialized: boolean }>();
    const saveSavedWorkspaces = vi.fn(async (workspaces: ProjectInfo[]) => workspaces);
    const projects = vi.fn(async () => [{ directory: "/opencode", name: "OpenCode" }]);
    window.localStorage.setItem("vela.savedWorkspaces", JSON.stringify([
      { directory: "/legacy", name: "Legacy" }
    ]));
    window.vela = api({
      savedWorkspaces: () => stored.promise,
      saveSavedWorkspaces,
      projects
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));

    expect(saveSavedWorkspaces).not.toHaveBeenCalled();

    await act(async () => {
      stored.resolve({ workspaces: [{ directory: "/durable", name: "Durable" }], initialized: true });
      await stored.promise;
    });

    const expected = [{ directory: "/durable", name: "Durable" }];
    expect(store.savedWorkspaces).toEqual(expected);
    expect(saveSavedWorkspaces).toHaveBeenLastCalledWith(expected);
    expect(JSON.parse(window.localStorage.getItem("vela.savedWorkspaces") ?? "[]")).toEqual(expected);
    expect(projects).not.toHaveBeenCalled();
  });

  it("keeps additions and removals made while durable storage is loading", async () => {
    const stored = deferred<{ workspaces: ProjectInfo[]; initialized: boolean }>();
    window.localStorage.setItem("vela.savedWorkspaces", JSON.stringify([{ directory: "/removed", name: "Removed" }]));
    window.vela = api({
      savedWorkspaces: () => stored.promise,
      selectDirectory: async () => "/added"
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    act(() => store.removeWorkspace("/removed"));
    await act(async () => store.saveWorkspace());
    await act(async () => {
      stored.resolve({ workspaces: [{ directory: "/removed", name: "Removed" }, { directory: "/kept", name: "Kept" }], initialized: true });
      await stored.promise;
    });
    expect(store.savedWorkspaces).toEqual([{ directory: "/kept", name: "Kept" }, { directory: "/added", name: "added" }]);
  });

  it("does not resurrect removed bookmarks from another renderer's old localStorage", async () => {
    window.localStorage.setItem("vela.savedWorkspaces", JSON.stringify([{ directory: "/removed", name: "Removed" }]));
    window.vela = api({ savedWorkspaces: async () => ({ workspaces: [], initialized: true }) });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    expect(store.savedWorkspaces).toEqual([]);
    expect(window.localStorage.getItem("vela.savedWorkspaces")).toBe("[]");
  });

  it("imports OpenCode projects on first use when no legacy bookmark list exists", async () => {
    const projects = [{ directory: "/recovered", name: "Recovered" }];
    const saveSavedWorkspaces = vi.fn(async (workspaces: ProjectInfo[]) => workspaces);
    window.vela = api({
      savedWorkspaces: async () => ({ workspaces: [], initialized: false }),
      saveSavedWorkspaces,
      projects: vi.fn(async () => projects)
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));

    expect(store.savedWorkspaces).toEqual(projects);
    expect(saveSavedWorkspaces).toHaveBeenLastCalledWith(projects);
  });

  it("adds a panel from the folder picker without closing existing panels", async () => {
    const closeSession = vi.fn(async () => {});
    const picked = info("/picked", 7);
    const selectFolder = vi.fn(async () => picked);
    window.vela = api({ closeSession, selectFolder });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));

    await act(async () => store.selectAddPanel());

    expect(closeSession).not.toHaveBeenCalled();
    expect(selectFolder).toHaveBeenCalledWith(expect.any(Number));
    expect(store.panels).toHaveLength(2);
    expect(store.panels[1].directory).toBe("/picked");
    expect(store.session?.directory).toBe("/picked");
  });

  it("keeps an existing file root visible when a saved workspace is added, then restores focus when it is closed", async () => {
    const closeSession = vi.fn(async () => {});
    window.vela = api({ closeSession });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/first"));
    const first = store.panels[0];

    await act(async () => store.openWorkspacePanel("/second"));

    const second = store.panels[1];
    expect(store.panels.map((panel) => panel.directory)).toEqual(["/first", "/second"]);
    expect(store.workspaceOnlyPanelIDs.has(second.id)).toBe(true);
    expect(store.session?.directory).toBe("/second");

    await act(async () => store.closePanel(second.id));

    expect(closeSession).toHaveBeenCalledWith(second.workspace);
    expect(store.panels.map((panel) => panel.directory)).toEqual(["/first"]);
    expect(store.session?.id).toBe(first.id);
  });

  it("swaps one panel to a picked folder and detaches its context into Open now", async () => {
    const closeSession = vi.fn(async () => {});
    const picked = info("/picked", 4);
    const selectFolder = vi.fn(async () => picked);
    window.vela = api({ closeSession, selectFolder });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.addModelPanel("/two"));
    expect(store.panels).toHaveLength(2);
    const first = store.panels[0];
    const swappedWorkspace = first.workspace;
    const untouchedWorkspace = store.panels[1].workspace;

    await act(async () => store.selectPanelDirectory(swappedWorkspace));

    expect(selectFolder).toHaveBeenCalledWith(expect.any(Number));
    expect(closeSession).not.toHaveBeenCalled();
    expect(store.panels).toHaveLength(2);
    expect(store.panels[0].directory).toBe("/picked");
    expect(store.panels[1].directory).toBe("/two");
    expect(store.activeSessions.map((active) => active.id)).toContain(first.id);
    expect(store.activeSessions.map((active) => active.id)).toContain(picked.id);
    expect(store.session?.id).toBe(picked.id);
    expect(store.panelViews[swappedWorkspace.id]).toBeUndefined();
    expect(store.panelViews[untouchedWorkspace.id]).toBeDefined();
  });

  it("swap drops the swapped panel's per-workspace editor and tree state", async () => {
    const closeSession = vi.fn(async () => {});
    const picked = info("/picked", 4);
    window.vela = api({
      closeSession,
      selectFolder: vi.fn(async () => picked),
      readFile: async (_workspace: unknown, path: string) => `content of ${path}`
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openFile("keep.txt"));
    const first = store.panels[0];
    const oldWorkspace = first.workspace;

    await act(async () => store.selectPanelDirectory(oldWorkspace));

    expect(closeSession).not.toHaveBeenCalled();
    expect(store.panels).toHaveLength(1);
    expect(store.panels[0].directory).toBe("/picked");
    expect(store.tabs.map((tab) => tab.path)).toEqual([]);
  });

  it("keeps a running session active when its panel changes workspace", async () => {
    const closeSession = vi.fn(async () => {});
    const picked = info("/picked", 4);
    window.vela = api({
      closeSession,
      selectFolder: vi.fn(async () => picked)
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    const running = store.panels[0];
    await act(async () => {
      messageHandler?.({
        kind: "event",
        type: "session.status",
        data: {
          id: "running-status",
          created: Date.now(),
          data: { sessionID: running.id, status: { type: "busy" } }
        }
      });
    });

    await act(async () => store.selectPanelDirectory(running.workspace));

    expect(closeSession).not.toHaveBeenCalledWith(running.workspace);
    expect(store.panels.map((panel) => panel.id)).toEqual([picked.id]);
    expect(store.activeSessions.map((active) => active.id)).toEqual([running.id, picked.id]);
  });

  it("changes a panel to a directory without a picker", async () => {
    const closeSession = vi.fn(async () => {});
    const openSession = vi.fn(async (directory: string, generation: number) => info(directory, generation));
    window.vela = api({ closeSession, openSession });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.addModelPanel("/two"));
    const swappedWorkspace = store.panels[0].workspace;

    await act(async () => store.changePanelDirectory(swappedWorkspace, "/three"));

    expect(openSession).toHaveBeenCalledWith("/three", expect.any(Number));
    expect(closeSession).not.toHaveBeenCalled();
    expect(store.panels).toHaveLength(2);
    expect(store.panels[0].directory).toBe("/three");
    expect(store.panels[1].directory).toBe("/two");
    expect(store.session?.directory).toBe("/three");
  });

  it("folds a mid-swap session event into the in-place swap instead of appending a panel", async () => {
    const picked = deferred<SessionInfo>();
    const closeSession = vi.fn(async () => {});
    window.vela = api({
      closeSession,
      selectFolder: vi.fn(() => picked.promise)
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.addModelPanel("/two"));
    const swappedWorkspace = store.panels[0].workspace;
    let pending!: Promise<void>;
    await act(async () => { pending = store.selectPanelDirectory(swappedWorkspace); });
    const pickedInfo = info("/picked", 4);
    await act(async () => { messageHandler?.({ kind: "session", session: pickedInfo }); });
    await act(async () => picked.resolve(pickedInfo));
    await act(async () => pending);

    expect(closeSession).not.toHaveBeenCalled();
    expect(store.panels).toHaveLength(2);
    expect(store.panels.map((panel) => panel.directory)).toEqual(["/picked", "/two"]);
    expect(new Set(store.panels.map((panel) => panel.workspace.id)).size).toBe(2);
  });

  it("closes a stale swap result that completes after a full workspace replacement", async () => {
    const picked = deferred<SessionInfo>();
    const closeSession = vi.fn(async () => {});
    window.vela = api({
      closeSession,
      selectFolder: vi.fn(() => picked.promise)
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    const first = store.panels[0];
    let pending!: Promise<void>;
    await act(async () => { pending = store.selectPanelDirectory(first.workspace); });
    await act(async () => store.openSession("/replacement"));
    await act(async () => picked.resolve(info("/picked", 9)));
    await act(async () => pending);

    expect(closeSession).toHaveBeenCalledWith(info("/picked", 9).workspace);
    expect(store.panels.map((panel) => panel.directory)).toEqual(["/replacement"]);
    expect(store.session?.directory).toBe("/replacement");
  });

  it("detaches idle panels into Open now on full replacement until explicitly closed", async () => {
    const closeSession = vi.fn(async () => {});
    window.vela = api({ closeSession });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    const first = store.panels[0];

    await act(async () => store.openSession("/two"));
    const second = store.panels[0];

    // Replacement never tears down: the old session stays jumpable.
    expect(closeSession).not.toHaveBeenCalled();
    expect(store.panels.map((panel) => panel.id)).toEqual([second.id]);
    expect(store.activeSessions.map((active) => active.id)).toEqual([first.id, second.id]);

    // Jumping back detaches the current panel instead of closing it.
    await act(async () => store.reopenSession(first.id));
    expect(closeSession).not.toHaveBeenCalledWith(second.workspace);
    expect(store.panels.map((panel) => panel.id)).toEqual([first.id]);
    expect(store.activeSessions.map((active) => active.id)).toContain(second.id);

    // Only an explicit close ends the backend link.
    await act(async () => store.closePanel(second.id));
    expect(closeSession).toHaveBeenCalledWith(second.workspace);
    expect(store.activeSessions.map((active) => active.id)).not.toContain(second.id);
  });
});
