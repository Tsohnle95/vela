import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, SessionInfo } from "@shared/types";
import type { VelaApi } from "../../preload";
import { StoreProvider, useStore } from "./store";
import { Composer } from "./components/Composer";
import type { TranscriptItem } from "@shared/types";
let store: ReturnType<typeof useStore>;
let receive: (message: BackendMessage) => void;
const panel: SessionInfo = { id: "session-1", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
function Probe() { store = useStore(); return <Composer />; }
function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}
function api(overrides: Partial<VelaApi> = {}): VelaApi {
  return {
    platform: "darwin", onMessage: (handler) => { receive = handler; return () => {}; },
    health: async () => true, takePendingPaths: async () => [], state: async () => null,
    activeSessions: async () => [], sessions: async () => [], models: async () => [],
    modelDefault: async () => null, sessionSelection: async () => null, agents: async () => [],
    savedWorkspaces: async () => ({ workspaces: [], initialized: true }),
    openSession: async () => panel,
    openSessionById: async () => ({ session: panel, transcript: [], todos: [], usage: null }),
    closeSession: async () => {}, recoveryRecords: async () => [], listPermissions: async () => [],
    formsList: async () => [], formReply: async () => {}, formCancel: async () => {},
    inboxList: async () => [], permissionReply: async () => {}, ...overrides
  } as VelaApi;
}
const history: TranscriptItem[] = [
  { kind: "user", id: "first", text: "keep" },
  { kind: "user", id: "second", text: "edit me", attachments: [{ name: "file.ts" }] }
];
describe("prompt undo", () => {
  let root: Root; let container: HTMLDivElement;
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true); window.localStorage.clear();
    container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  });
  afterEach(async () => { await act(async () => root.unmount()); container.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
  async function open(overrides: Partial<VelaApi> = {}) {
    window.vela = api({ ...overrides });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.reopenSession(panel.id));
  }
  it("removes the selected prompt through runtime commit without resurrecting local history", async () => {
    let committed = false;
    const revertStage = vi.fn(async () => ({ messageID: "second" }));
    const revertCommit = vi.fn(async () => { committed = true; });
    await open({ revertStage, revertCommit,
      openSessionById: async () => ({ session: panel, transcript: committed ? history.slice(0, 1) : history, todos: [], usage: null }),
      sessionTranscript: async () => ({ transcript: committed ? history.slice(0, 1) : history, todos: [] })
    });
    expect(store.panelViews[panel.workspace.id].transcript).toEqual(history);
    await act(async () => store.stageRevert(panel.workspace, "second"));
    expect(revertStage).toHaveBeenCalledWith(panel.workspace, "second", true);
    expect(revertCommit).toHaveBeenCalledWith(panel.workspace);
    expect(store.panelViews[panel.workspace.id].transcript).toEqual(history.slice(0, 1));
    expect(store.panelViews[panel.workspace.id].stagedRevert).toBeNull();
  });
  it("restores the selected prompt into the input only after runtime edit succeeds", async () => {
    let committed = false;
    await open({ revertStage: async () => ({ messageID: "second" }), revertCommit: async () => { committed = true; },
      openSessionById: async () => ({ session: panel, transcript: history, todos: [], usage: null }),
      sessionTranscript: async () => ({ transcript: committed ? history.slice(0, 1) : history, todos: [] }) });
    await act(async () => store.stageRevert(panel.workspace, "second", true));
    expect(container.querySelector("textarea")?.value).toBe("edit me");
    expect(store.panelViews[panel.workspace.id].transcript).toEqual(history.slice(0, 1));
  });
  it("coalesces repeated clicks while the runtime undo is pending", async () => {
    const staged = deferred<{ messageID: string }>();
    const revertStage = vi.fn(() => staged.promise);
    const revertCommit = vi.fn(async () => {});
    await open({ revertStage, revertCommit,
      openSessionById: async () => ({ session: panel, transcript: history, todos: [], usage: null }),
      sessionTranscript: async () => ({ transcript: history.slice(0, 1), todos: [] }) });
    let pending!: Promise<void>;
    act(() => { pending = store.stageRevert(panel.workspace, "second"); });
    await act(async () => store.stageRevert(panel.workspace, "second"));
    await act(async () => { staged.resolve({ messageID: "second" }); await pending; });
    expect(revertStage).toHaveBeenCalledTimes(1);
    expect(revertCommit).toHaveBeenCalledTimes(1);
  });
  it("blocks manual retry during automatic commit and allows retry after failure", async () => {
    const committing = deferred<void>();
    const revertCommit = vi.fn().mockImplementationOnce(() => committing.promise).mockResolvedValue(undefined);
    await open({ revertStage: async () => ({ messageID: "second" }), revertCommit,
      openSessionById: async () => ({ session: panel, transcript: history, todos: [], usage: null }),
      sessionTranscript: async () => ({ transcript: history.slice(0, 1), todos: [] }) });
    let pending!: Promise<void>;
    await act(async () => { pending = store.stageRevert(panel.workspace, "second"); await Promise.resolve(); });
    expect(revertCommit).toHaveBeenCalledTimes(1);
    expect(store.panelViews[panel.workspace.id].stagedRevert).toBeNull();
    await act(async () => store.commitStagedRevert(panel.workspace));
    expect(revertCommit).toHaveBeenCalledTimes(1);
    await act(async () => { committing.reject(new Error("Commit failed")); await pending; });
    expect(store.panelViews[panel.workspace.id].stagedRevert).toEqual({ messageID: "second" });
    await act(async () => store.commitStagedRevert(panel.workspace));
    expect(revertCommit).toHaveBeenCalledTimes(2);
    expect(store.panelViews[panel.workspace.id].stagedRevert).toBeNull();
    expect(store.panelViews[panel.workspace.id].transcript).toEqual(history.slice(0, 1));
  });
  it("does not commit, remove, or restore text when runtime staging fails", async () => {
    const revertCommit = vi.fn(async () => {});
    await open({ revertStage: async () => { throw new Error("Cannot stage"); }, revertCommit,
      openSessionById: async () => ({ session: panel, transcript: history, todos: [], usage: null }) });
    await act(async () => store.stageRevert(panel.workspace, "second", true));
    expect(revertCommit).not.toHaveBeenCalled();
    expect(store.panelViews[panel.workspace.id].transcript).toEqual(history);
    expect(container.querySelector("textarea")?.value).toBe("");
    expect(store.toasts.at(-1)?.text).toBe("Cannot stage");
  });
  it("keeps the conversation and exposes errors when runtime commit fails", async () => {
    await open({ revertStage: async () => ({ messageID: "second" }), revertCommit: async () => { throw new Error("Cannot undo"); },
      openSessionById: async () => ({ session: panel, transcript: history, todos: [], usage: null }) });
    await act(async () => store.stageRevert(panel.workspace, "second"));
    expect(store.panelViews[panel.workspace.id].transcript).toEqual(history);
    expect(store.toasts.at(-1)?.text).toBe("Cannot undo");
    expect(store.panelViews[panel.workspace.id].stagedRevert).toEqual({ messageID: "second" });
  });
});
