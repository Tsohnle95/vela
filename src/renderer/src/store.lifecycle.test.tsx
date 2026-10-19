import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, SessionInfo } from "@shared/types";
import { StoreProvider, useStore } from "./store";

let store: ReturnType<typeof useStore>;
let receive: (message: BackendMessage) => void;
function Probe() { store = useStore(); return null; }
function info(directory: string, generation: number, id = `session-${generation}`): SessionInfo {
  return { id, directory, workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation } };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
function api(overrides: Partial<VelaApi> = {}): VelaApi {
  return {
    platform: "darwin", onMessage: (handler) => { receive = handler; return () => {}; },
    health: async () => true, takePendingPaths: async () => [], state: async () => null,
    activeSessions: async () => [], sessions: async () => [], models: async () => [],
    modelDefault: async () => null, sessionSelection: async () => null, agents: async () => [],
    savedWorkspaces: async () => ({ initialized: true, workspaces: [] }),
    saveSavedWorkspaces: async () => {}, formsList: async () => [], inboxList: async () => [],
    listPermissions: async () => [], recoveryRecords: async () => [], closeSession: async () => {},
    openSession: async (directory, request) => info(directory, request),
    openSessionById: async (id) => ({ session: info("/replay", 0, id), transcript: [], todos: [], usage: null }),
    ...overrides
  } as VelaApi;
}

describe("Store lifecycle acquisition authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    container.remove();
  });
  async function mount(overrides: Partial<VelaApi> = {}) {
    window.vela = api(overrides);
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
  }

  it("shares the monotonic request sequence between acquisition and transcript hydration", async () => {
    const calls: [string, number][] = [];
    await mount({
      openSession: async (directory, request) => { calls.push(["open", request]); return info(directory, request); },
      selectFolder: async (request) => { calls.push(["picker", request]); return info("/picked", request); },
      openSessionById: async (id, request) => {
        calls.push(["hydrate", request]);
        return { session: info("/replay", 0, id), transcript: [], todos: [], usage: null };
      }
    });
    await act(async () => store.addModelPanel("/one"));
    await act(async () => store.selectFolder());
    expect(calls).toEqual([["open", 1], ["hydrate", 2], ["picker", 3], ["hydrate", 4]]);
    expect(store.session?.directory).toBe("/picked");
  });

  it.each(["open", "add", "pick-add", "pick-replace", "swap"] as const)(
    "preserves ordered content and catalog launches for %s",
    async (operation) => {
      const trace: string[] = [];
      await mount({
        openSessionById: async (id) => {
          trace.push("hydrate");
          return { session: info("/replay", 0, id), transcript: [], todos: [], usage: null };
        },
        selectFolder: async (request) => info("/picked", request),
        inboxList: async () => { trace.push("inbox"); return []; },
        formsList: async () => { trace.push("forms"); return []; },
        recoveryRecords: async () => { trace.push("recovery"); return []; },
        models: async () => { trace.push("models"); return []; },
        agents: async () => { trace.push("agents"); return []; },
        sessions: async () => { trace.push("history"); return []; }
      });
      if (operation === "swap") await act(async () => store.addModelPanel("/initial"));
      trace.length = 0;
      await act(async () => {
        if (operation === "open") await store.openSession("/next");
        if (operation === "add") await store.addModelPanel("/next");
        if (operation === "pick-add") await store.selectAddPanel();
        if (operation === "pick-replace") await store.selectFolder();
        if (operation === "swap") await store.changePanelDirectory(store.session!.workspace, "/next");
      });
      expect(trace).toEqual(["hydrate", "inbox", "forms", "recovery", "models", "agents", "history"]);
      expect(store.session?.directory).toBe(operation.startsWith("pick") ? "/picked" : "/next");
    }
  );

  it("a cancelled replacement picker still supersedes an outstanding additive acquisition", async () => {
    const pending = deferred<SessionInfo>();
    const stale = info("/late", 1);
    const closeSession = vi.fn(async () => {});
    await mount({ openSession: () => pending.promise, selectFolder: async () => null, closeSession });
    let opening!: Promise<void>;
    act(() => { opening = store.addModelPanel("/late"); });
    await act(async () => store.selectFolder());
    await act(async () => { pending.resolve(stale); await opening; });
    expect(store.panels).toEqual([]);
    expect(store.activeSessionID).toBeNull();
    expect(closeSession).toHaveBeenCalledExactlyOnceWith(stale.workspace);
  });

  it("retains reopen suppression until both same-session acquisitions settle", async () => {
    type Replay = Awaited<ReturnType<VelaApi["openSessionById"]>>;
    const older = deferred<Replay>();
    const newer = deferred<Replay>();
    const openSessionById = vi.fn().mockImplementationOnce(() => older.promise).mockImplementationOnce(() => newer.promise);
    const closeSession = vi.fn(async () => {});
    await mount({ openSessionById, closeSession });
    let first!: Promise<SessionInfo | null>;
    let second!: Promise<SessionInfo | null>;
    act(() => { first = store.reopenSession("target"); second = store.reopenSession("target"); });
    act(() => receive({ kind: "session", session: info("/event-before", 3, "target") }));
    expect(store.panels).toEqual([]);
    const latest = info("/latest", 2, "target");
    await act(async () => { newer.resolve({ session: latest, transcript: [], todos: [], usage: null }); await second; });
    act(() => receive({ kind: "session", session: info("/event-between", 4, "target") }));
    expect(store.panels[0].directory).toBe("/latest");
    const stale = info("/stale", 1, "target");
    await act(async () => { older.resolve({ session: stale, transcript: [], todos: [], usage: null }); await first; });
    expect(closeSession).toHaveBeenCalledExactlyOnceWith(stale.workspace);
    act(() => receive({ kind: "session", session: info("/event-after", 5, "target") }));
    expect(store.panels[0].directory).toBe("/event-after");
  });

  it("captures dirty close writes before a later edit and keeps the panel when that revision cannot finish", async () => {
    const write = deferred<void>();
    const writeFile = vi.fn<VelaApi["writeFile"]>(() => write.promise);
    const closeSession = vi.fn(async () => {});
    await mount({ readFile: async () => "original", writeFile, closeSession });
    await act(async () => store.addModelPanel("/dirty"));
    const workspace = store.session!.workspace;
    await act(async () => store.openFile("notes.txt", workspace));
    act(() => store.editContent("notes.txt", "first edit"));
    await act(async () => store.closePanel(store.session!.id));
    expect(writeFile).toHaveBeenCalledOnce();
    expect(writeFile.mock.calls[0]).toEqual([workspace, "notes.txt", "first edit", expect.objectContaining({ expectedContent: "original", revision: 1 })]);
    act(() => store.editContent("notes.txt", "second edit"));
    await act(async () => write.resolve());
    expect(store.panels).toHaveLength(1);
    expect(store.tabs[0].content).toBe("second edit");
    expect(closeSession).not.toHaveBeenCalled();
  });

  it("retains a dirty panel after failed persistence and exposes the original save failure", async () => {
    const closeSession = vi.fn(async () => {});
    await mount({ readFile: async () => "original", writeFile: async () => { throw new Error("disk unavailable"); }, closeSession });
    await act(async () => store.addModelPanel("/dirty"));
    await act(async () => store.openFile("notes.txt"));
    act(() => store.editContent("notes.txt", "edited"));
    await act(async () => store.closePanel(store.session!.id));
    expect(store.panels).toHaveLength(1);
    expect(store.toasts.at(-1)?.text).toBe("Failed to save notes.txt: disk unavailable");
    expect(closeSession).not.toHaveBeenCalled();
  });

  it("refuses dirty conflicted close before attempting save or backend teardown", async () => {
    const writeFile = vi.fn(async () => {});
    const closeSession = vi.fn(async () => {});
    await mount({ readFile: async () => "original", writeFile, closeSession });
    await act(async () => store.addModelPanel("/conflict"));
    const panel = store.session!;
    await act(async () => store.openFile("notes.txt"));
    act(() => store.editContent("notes.txt", "local"));
    act(() => receive({ kind: "file-update", file: { workspace: panel.workspace, sessionID: panel.id, path: "notes.txt", baseline: { kind: "known", content: "original" }, content: "external", deleted: false } }));
    act(() => store.closePanel(panel.id));
    expect(store.tabs[0].conflict).not.toBeNull();
    expect(store.toasts.at(-1)?.text).toBe("Resolve external file conflicts before closing this workspace.");
    expect(writeFile).not.toHaveBeenCalled();
    expect(closeSession).not.toHaveBeenCalled();
    expect(store.panels).toHaveLength(1);
  });

  it("preserve-busy detaches a panel while retaining its live inventory and transcript", async () => {
    const closeSession = vi.fn(async () => {});
    await mount({ closeSession });
    await act(async () => store.addModelPanel("/busy"));
    const panel = store.session!;
    act(() => receive({ kind: "event", type: "session.execution.started", data: { id: "execution", created: Date.now(), data: { sessionID: panel.id } } }));
    expect(store.busy).toBe(true);
    act(() => store.closePanel(panel.id, true));
    expect(store.panels).toEqual([]);
    expect(store.activeSessions.some((item) => item.id === panel.id)).toBe(true);
    expect(closeSession).not.toHaveBeenCalled();
    await act(async () => store.reopenSession(panel.id, true));
    expect(store.panels[0].id).toBe(panel.id);
    expect(store.activeSessionID).toBeNull();
  });

  it("closes an inventory-only detached context using its existing workspace identity", async () => {
    const closeSession = vi.fn(async () => {});
    await mount({ closeSession });
    await act(async () => store.addModelPanel("/old"));
    const previous = store.session!;
    await act(async () => store.openSession("/new"));
    expect(store.activeSessions.some((item) => item.id === previous.id)).toBe(true);
    act(() => store.closePanel(previous.id));
    expect(closeSession).toHaveBeenCalledExactlyOnceWith(previous.workspace);
    expect(store.activeSessions.some((item) => item.id === previous.id)).toBe(false);
    expect(store.session?.directory).toBe("/new");
  });

  it("releases failed swap suppression while retaining the attached target and reporting its failure", async () => {
    await mount();
    await act(async () => store.addModelPanel("/target"));
    const target = store.session!;
    window.vela.openSession = async () => { throw new Error("swap failed"); };
    await act(async () => store.changePanelDirectory(target.workspace, "/failure"));
    expect(store.toasts.at(-1)?.text).toBe("swap failed");
    expect(store.session?.id).toBe(target.id);
    const event = info("/after-failure", 8, "event-session");
    act(() => receive({ kind: "session", session: event }));
    expect(store.panels.some((panel) => panel.id === event.id)).toBe(true);
  });

  it("silent reopen survives newer activation and attaches without taking focus", async () => {
    type Replay = Awaited<ReturnType<VelaApi["openSessionById"]>>;
    const replay = deferred<Replay>();
    const closeSession = vi.fn(async () => {});
    await mount({ openSessionById: (id) => id === "silent" ? replay.promise : Promise.resolve({ session: info("/hydrate", 0, id), transcript: [], todos: [], usage: null }), closeSession });
    let opening!: Promise<SessionInfo | null>;
    act(() => { opening = store.reopenSession("silent", true); });
    await act(async () => store.openSession("/user"));
    const silent = info("/silent", 9, "silent");
    await act(async () => { replay.resolve({ session: silent, transcript: [], todos: [], usage: null }); await opening; });
    expect(store.panels.map((panel) => panel.directory)).toEqual(["/user", "/silent"]);
    expect(store.session?.directory).toBe("/user");
    expect(closeSession).not.toHaveBeenCalled();
  });


  it("finishes a captured dirty close after focus moves to another attached panel", async () => {
    const write = deferred<void>();
    const closeSession = vi.fn(async () => {});
    await mount({ readFile: async () => "original", writeFile: () => write.promise, closeSession });
    await act(async () => store.addModelPanel("/closing"));
    const closing = store.session!;
    await act(async () => store.openFile("notes.txt"));
    act(() => store.editContent("notes.txt", "edited"));
    await act(async () => store.closePanel(closing.id));
    await act(async () => store.addModelPanel("/neighbor"));
    const neighbor = store.session!;
    await act(async () => write.resolve());
    expect(closeSession).toHaveBeenCalledExactlyOnceWith(closing.workspace);
    expect(store.panels.map((panel) => panel.id)).toEqual([neighbor.id]);
    expect(store.activeSessionID).toBe(neighbor.id);
  });

  it("starts deletion while dirty-close persistence is still pending", async () => {
    const write = deferred<void>();
    const trace: string[] = [];
    const closeSession = vi.fn(async () => { trace.push("close"); });
    await mount({ readFile: async () => "original", writeFile: () => { trace.push("save"); return write.promise; }, deleteSession: async () => { trace.push("delete"); }, closeSession });
    await act(async () => store.addModelPanel("/delete"));
    const closing = store.session!;
    await act(async () => store.openFile("notes.txt"));
    act(() => store.editContent("notes.txt", "edited"));
    vi.spyOn(window, "confirm").mockReturnValue(true);
    await act(async () => store.deleteSession(closing.id));
    expect(trace).toEqual(["delete", "save"]);
    expect(store.panels).toHaveLength(1);
    await act(async () => write.resolve());
    expect(trace).toEqual(["delete", "save", "close"]);
    expect(store.panels).toEqual([]);
  });

  it("suppresses a failed swap toast after its captured target has closed", async () => {
    const pick = deferred<SessionInfo>();
    await mount();
    await act(async () => store.addModelPanel("/target"));
    const target = store.session!;
    const failure = new Promise<SessionInfo>((_resolve, reject) => { pick.promise.then(() => reject(new Error("late swap failed"))); });
    window.vela.openSession = () => failure;
    let swapping!: Promise<void>;
    act(() => { swapping = store.changePanelDirectory(target.workspace, "/missing"); });
    act(() => store.closePanel(target.id));
    await act(async () => { pick.resolve(target); await swapping; });
    expect(store.toasts.some((toast) => toast.text === "late swap failed")).toBe(false);
    const later = info("/after-closed-failure", 8);
    act(() => receive({ kind: "session", session: later }));
    expect(store.panels).toEqual([later]);
  });


  it("does not read the session envelope again when a pending swap suppresses attachment", async () => {
    const picker = deferred<SessionInfo | null>();
    await mount({ selectFolder: () => picker.promise });
    await act(async () => store.addModelPanel("/target"));
    let swapping!: Promise<void>;
    act(() => { swapping = store.selectPanelDirectory(store.session!.workspace); });
    const incoming = info("/suppressed", 8);
    const readSession = vi.fn(() => incoming);
    const envelope = { kind: "session" } as BackendMessage;
    Object.defineProperty(envelope, "session", { get: readSession });
    act(() => receive(envelope));
    expect(readSession).toHaveBeenCalledTimes(2);
    expect(store.panels.some((panel) => panel.id === incoming.id)).toBe(false);
    await act(async () => { picker.resolve(null); await swapping; });
  });

});
