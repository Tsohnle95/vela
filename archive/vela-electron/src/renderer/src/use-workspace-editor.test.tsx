import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentFileState, FileUpdate, FileWriteIdentity, SessionInfo } from "@shared/types";
import type { VelaApi } from "../../preload";
import { useWorkspaceEditor } from "./use-workspace-editor";

function rejectLifecycleQueries(host: Parameters<typeof useWorkspaceEditor>[0]) {
  // @ts-expect-error Editor borrows focus metadata without mutation authority.
  host.readFocusedSession()!.directory = "/other";
  // @ts-expect-error Editor borrows readonly workspace identity.
  host.readPanels()[0].workspace.generation++;
  // @ts-expect-error Editor cannot replace borrowed panel entries.
  host.readPanels()[0] = host.readPanels()[0];
  // @ts-expect-error Generation-validated panel queries are readonly.
  host.panelFor(host.readPanels()[0].workspace)!.id = "other";
}
void rejectLifecycleQueries;

const first: SessionInfo = { id: "first", directory: "/first", workspace: { id: "workspace-first", generation: 1 } };
const second: SessionInfo = { id: "second", directory: "/second", workspace: { id: "workspace-second", generation: 2 } };
let editor: ReturnType<typeof useWorkspaceEditor>;
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("workspace editor authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  const sessionRef = { current: first as SessionInfo | null };
  const panelsRef = { current: [first, second] };
  const agentFilesRef = { current: {} as Record<string, Map<string, AgentFileState>> };
  const readAgentFile = (workspace: SessionInfo["workspace"], path: string) => agentFilesRef.current[workspace.id]?.get(path);
  const notify = vi.fn();
  const onFileActivate = vi.fn();
  const focusSession = vi.fn((id: string) => { sessionRef.current = panelsRef.current.find((panel) => panel.id === id) ?? null; });
  const panelFor = (workspace: SessionInfo["workspace"]) => panelsRef.current.find((panel) => panel.workspace.id === workspace.id && panel.workspace.generation === workspace.generation) ?? null;
  function Probe() {
    editor = useWorkspaceEditor({ readFocusedSession: () => sessionRef.current, readPanels: () => panelsRef.current, panelFor, focusSession, readAgentFile, notify, onFileActivate });
    return null;
  }
  const render = async () => act(async () => root.render(<Probe />));
  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    sessionRef.current = first;
    panelsRef.current = [first, second];
    agentFilesRef.current = {};
    notify.mockReset();
    onFileActivate.mockReset();
    focusSession.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    window.vela = {
      readFile: async () => "saved",
      writeFile: async () => {},
      writeStandalone: async () => {},
      openExternal: async () => ({ kind: "standalone", path: "/outside/file", content: "outside" })
    } as unknown as VelaApi;
    await render();
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("replaces the transient preview tab on the next preview open", async () => {
    await act(async () => editor.previewFile("one"));
    expect(editor.tabs.map((tab) => [tab.path, tab.preview])).toEqual([["one", true]]);

    await act(async () => editor.previewFile("two"));
    expect(editor.tabs.map((tab) => [tab.path, tab.preview])).toEqual([["two", true]]);
  });

  it("anchors a preview when it is opened again, edited, or another file is pinned", async () => {
    await act(async () => editor.previewFile("one"));
    await act(async () => editor.openFile("one"));
    expect(editor.tabs.map((tab) => [tab.path, tab.preview])).toEqual([["one", false]]);

    await act(async () => editor.previewFile("two"));
    act(() => editor.editContent("two", "dirty"));
    expect(editor.tabs.map((tab) => [tab.path, tab.preview])).toEqual([["one", false], ["two", false]]);

    // A pinned tab survives the next preview open.
    await act(async () => editor.previewFile("three"));
    expect(editor.tabs.map((tab) => [tab.path, tab.preview])).toEqual([["one", false], ["two", false], ["three", true]]);
  });

  it("re-activating an already open preview keeps it previewing", async () => {
    await act(async () => editor.previewFile("one"));
    await act(async () => editor.previewFile("one"));
    expect(editor.tabs.map((tab) => [tab.path, tab.preview])).toEqual([["one", true]]);
  });

  it("activates successful foreground files without changing existing dirty buffers", async () => {
    await act(async () => editor.openFile("file"));
    expect(onFileActivate).toHaveBeenCalledOnce();
    act(() => editor.editContent("file", "dirty"));
    onFileActivate.mockClear();
    await act(async () => editor.openFile("file"));
    expect(onFileActivate).toHaveBeenCalledOnce();
    expect(editor.tabs[0]).toMatchObject({ content: "dirty", dirty: true });
    onFileActivate.mockClear();
    act(() => editor.setActive("missing"));
    expect(onFileActivate).not.toHaveBeenCalled();
    act(() => editor.setActive("file"));
    expect(onFileActivate).toHaveBeenCalledOnce();
  });

  it("does not activate failed, binary, background or stale file reads", async () => {
    window.vela.readFile = async () => null;
    await act(async () => editor.openFile("missing"));
    window.vela.readFile = async () => "binary\u0000";
    await act(async () => editor.openFile("binary"));
    window.vela.readFile = async () => "background";
    await act(async () => editor.openFile("background", second.workspace));
    const read = deferred<string>();
    window.vela.readFile = async () => read.promise;
    const opening = editor.openFile("late");
    sessionRef.current = second;
    await render();
    await act(async () => { read.resolve("late"); await opening; });
    expect(onFileActivate).not.toHaveBeenCalled();
  });

  it("activates relative and standalone external files, and rejects closed external responses", async () => {
    window.vela.openExternal = async () => ({ kind: "relative", rel: "relative", content: "saved" });
    await act(async () => editor.openExternalPath("/first/relative"));
    expect(onFileActivate).toHaveBeenCalledOnce();
    window.vela.openExternal = async () => ({ kind: "standalone", path: "/outside/file", content: "outside" });
    await act(async () => editor.openExternalPath("/outside/file"));
    await act(async () => editor.openExternalPath("/outside/file"));
    expect(onFileActivate).toHaveBeenCalledTimes(3);
    onFileActivate.mockClear();
    const pending = deferred<Awaited<ReturnType<VelaApi["openExternal"]>>>();
    window.vela.openExternal = async () => pending.promise;
    const opening = editor.openExternalPath("/outside/late");
    panelsRef.current = [second];
    sessionRef.current = second;
    await render();
    await act(async () => {
      pending.resolve({ kind: "standalone", path: "/outside/late", content: "late" });
      expect(await opening).toBeNull();
    });
    expect(onFileActivate).not.toHaveBeenCalled();
    expect(editor.unsavedEditorFiles).toEqual([]);
  });

  it("rejects a captured read after its workspace generation is replaced", async () => {
    const read = deferred<string>();
    window.vela.readFile = async () => read.promise;
    const opening = editor.openFile("file");
    panelsRef.current = [second];
    sessionRef.current = second;
    await render();
    await act(async () => { read.resolve("old"); await opening; });
    expect(editor.tabs).toEqual([]);
    expect(editor.unsavedEditorFiles).toEqual([]);
    expect(notify).not.toHaveBeenCalled();
  });

  it("uses synchronous focused-session updates and the last-panel standalone fallback", async () => {
    sessionRef.current = second;
    await act(async () => editor.openFile("second-file"));
    expect(editor.tabs.map((tab) => tab.path)).toEqual(["second-file"]);
    sessionRef.current = null;
    await act(async () => editor.openExternalPath("/outside/file"));
    expect(focusSession).toHaveBeenCalledWith(second.id);
    expect(editor.tabs.map((tab) => tab.path)).toEqual(["second-file", "/outside/file"]);
  });

  it("retains dirty buffers when subscription cleanup cancels an in-flight save", async () => {
    const write = deferred<void>();
    window.vela.writeFile = async () => write.promise;
    await act(async () => editor.openFile("file"));
    act(() => editor.editContent("file", "changed"));
    const saving = editor.saveTab("file");
    await act(async () => { await Promise.resolve(); });
    editor.cancelAll();
    await act(async () => { write.resolve(); await saving; });
    expect(editor.tabs[0]).toMatchObject({ content: "changed", saved: "saved", dirty: true });
  });

  it("classifies echoes before applying tabs and keeps newer external conflicts", async () => {
    const write = deferred<void>();
    let identity!: FileWriteIdentity;
    window.vela.writeFile = async (_workspace, _path, _content, id) => { identity = id; return write.promise; };
    await act(async () => editor.openFile("file"));
    act(() => editor.editContent("file", "changed"));
    const saving = editor.saveTab("file");
    await act(async () => { await Promise.resolve(); });
    const echo: FileUpdate = { workspace: first.workspace, sessionID: first.id, path: "file", content: "changed", deleted: false, baseline: { kind: "known", content: "saved" }, write: identity };
    const origin = editor.prepareFileUpdate(echo);
    expect(origin).toBe("echo");
    act(() => editor.applyFileUpdate(echo, origin));
    expect(editor.tabs[0].dirty).toBe(true);
    const external = { ...echo, content: "external", write: undefined };
    act(() => editor.applyFileUpdate(external, editor.prepareFileUpdate(external)));
    await act(async () => { write.resolve(); await saving; });
    expect(editor.tabs[0]).toMatchObject({ content: "changed", dirty: true, conflict: { content: "external" } });
  });

  it("keeps dirty files whose workspace is closed in the unsaved inventory", async () => {
    await act(async () => editor.openFile("file"));
    act(() => editor.editContent("file", "changed"));
    panelsRef.current = [second];
    sessionRef.current = second;
    await render();
    expect(editor.unsavedEditorFiles).toMatchObject([{ workspaceID: first.workspace.id, sessionID: null, workspace: null, path: "file" }]);
    let saved = true;
    await act(async () => { saved = await editor.saveAllEditorFiles(); });
    expect(saved).toBe(false);
    expect(notify).toHaveBeenCalledWith(expect.stringContaining("its workspace is closed"), "error");
  });

  it("preserves captured-active rename semantics independently of the latest focused tab", async () => {
    await act(async () => editor.openFile("old"));
    await act(async () => editor.openFile("other"));
    expect(editor.activePath).toBe("other");
    act(() => editor.renameTabs(first.workspace, "old", "new", "new", "old"));
    expect(editor.activePath).toBe("new");
    expect(editor.tabs.map((tab) => tab.path)).toEqual(["new", "other"]);
  });
});
