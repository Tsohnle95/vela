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

describe("store external files", () => {
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

  it("opens an in-workspace drop as a normal relative tab", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "relative", rel: "src/a.txt", content: "hello"
    }));
    window.vela = api({ openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/one/src/a.txt"));

    expect(store.tabs.map((t) => t.path)).toEqual(["src/a.txt"]);
    expect(store.tabs[0].standalone).toBeUndefined();
    expect(openExternal).toHaveBeenCalledWith(store.session!.workspace, "/one/src/a.txt");
  });

  it("opens an out-of-workspace drop as a writable standalone tab", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone", path: "/outside/notes.txt", content: "notes"
    }));
    window.vela = api({ openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));

    const tab = store.tabs.find((t) => t.path === "/outside/notes.txt")!;
    expect(tab.standalone).toBe(true);
    expect(store.activePath).toBe("/outside/notes.txt");
  });

  it("re-uses an already open standalone tab instead of duplicating it", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone", path: "/outside/notes.txt", content: "notes"
    }));
    window.vela = api({ openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));

    expect(store.tabs.filter((t) => t.path === "/outside/notes.txt")).toHaveLength(1);
  });

  it("saves a standalone tab through the absolute write channel", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone", path: "/outside/notes.txt", content: "v1"
    }));
    const writeStandalone = vi.fn(async () => {});
    window.vela = api({ openExternal, writeStandalone });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));
    act(() => store.editContent("/outside/notes.txt", "v2"));

    expect(writeStandalone).not.toHaveBeenCalled();
    expect(store.tabs.find((t) => t.path === "/outside/notes.txt")?.dirty).toBe(true);
    expect(store.unsavedEditorFileCount).toBe(1);

    await act(async () => store.saveTab("/outside/notes.txt"));

    expect(writeStandalone).toHaveBeenCalledWith("/outside/notes.txt", "v2", "v1", false);
    const tab = store.tabs.find((t) => t.path === "/outside/notes.txt")!;
    expect(tab.dirty).toBe(false);
    expect(store.unsavedEditorFileCount).toBe(0);
  });

  it("keeps edits off disk until the user saves, even after typing settles", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone", path: "/outside/notes.txt", content: "v1"
    }));
    const writeStandalone = vi.fn(async () => {});
    window.vela = api({ openExternal, writeStandalone });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));
    act(() => store.editContent("/outside/notes.txt", "v2"));

    expect(store.unsavedEditorFileCount).toBe(1);
    await act(async () => new Promise((resolve) => setTimeout(resolve, 750)));

    expect(writeStandalone).not.toHaveBeenCalled();
    expect(store.unsavedEditorFileCount).toBe(1);
    await act(async () => store.saveTab("/outside/notes.txt"));
    expect(writeStandalone).toHaveBeenCalledWith("/outside/notes.txt", "v2", "v1", false);
    expect(store.unsavedEditorFileCount).toBe(0);
  });

  it("saves dirty files in all workspaces without navigating to their tabs", async () => {
    const writeStandalone = vi.fn(async () => {});
    window.vela = api({
      openExternal: async (_workspace, path) => ({ kind: "standalone", path, content: "v1" }),
      writeStandalone
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/one.txt"));
    act(() => store.editContent("/outside/one.txt", "one edited"));
    await act(async () => store.openWorkspacePanel("/two"));
    await act(async () => store.openExternalPath("/outside/two.txt"));
    act(() => store.editContent("/outside/two.txt", "two edited"));
    const activeID = store.session!.id;
    let saved = false;
    await act(async () => { saved = await store.saveAllEditorFiles(); });
    expect(saved).toBe(true);
    expect(writeStandalone).toHaveBeenCalledWith("/outside/one.txt", "one edited", "v1", false);
    expect(writeStandalone).toHaveBeenCalledWith("/outside/two.txt", "two edited", "v1", false);
    expect(store.unsavedEditorFileCount).toBe(0);
    expect(store.session!.id).toBe(activeID);
  });

  it("does not approve quitting when a save fails", async () => {
    window.vela = api({
      openExternal: async (_workspace, path) => ({ kind: "standalone", path, content: "v1" }),
      writeStandalone: async () => { throw new Error("disk write failed"); }
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/one.txt"));
    act(() => store.editContent("/outside/one.txt", "edited"));
    let saved = true;
    await act(async () => { saved = await store.saveAllEditorFiles(); });
    expect(saved).toBe(false);
    expect(store.unsavedEditorFileCount).toBe(1);
  });

  it("keeps the app open if the buffer changes while Save All is writing", async () => {
    let finishWrite: (() => void) | undefined;
    window.vela = api({
      openExternal: async (_workspace, path) => ({ kind: "standalone", path, content: "v1" }),
      writeStandalone: () => new Promise<void>((resolve) => { finishWrite = resolve; })
    });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/one.txt"));
    act(() => store.editContent("/outside/one.txt", "first edit"));
    let saving = Promise.resolve(false);
    act(() => { saving = store.saveAllEditorFiles(); });
    await act(async () => Promise.resolve());
    expect(finishWrite).toBeDefined();
    act(() => store.editContent("/outside/one.txt", "newer edit"));
    let saved = true;
    await act(async () => { finishWrite?.(); saved = await saving; });
    expect(saved).toBe(false);
    expect(store.tabs[0].content).toBe("newer edit");
    expect(store.unsavedEditorFileCount).toBe(1);
  });

  it("saves dirty tabs before closing their workspace", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone", path: "/outside/notes.txt", content: "v1"
    }));
    let finishWrite: (() => void) | null = null;
    const writeStandalone = vi.fn(() => new Promise<void>((resolve) => { finishWrite = resolve; }));
    const closeSession = vi.fn(async () => {});
    window.vela = api({ openExternal, writeStandalone, closeSession });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));
    act(() => store.editContent("/outside/notes.txt", "v2"));

    act(() => store.closePanel(store.session!.id));
    await act(async () => Promise.resolve());

    expect(writeStandalone).toHaveBeenCalledWith("/outside/notes.txt", "v2", "v1", false);
    expect(closeSession).not.toHaveBeenCalled();
    expect(store.unsavedEditorFileCount).toBe(1);

    await act(async () => {
      finishWrite?.();
      await Promise.resolve();
    });

    expect(closeSession).toHaveBeenCalledOnce();
    expect(store.unsavedEditorFileCount).toBe(0);
  });

  it("clears the unsaved state when an edit is undone back to the saved content", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone", path: "/outside/notes.txt", content: "v1"
    }));
    window.vela = api({ openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));

    act(() => store.editContent("/outside/notes.txt", "v2"));
    expect(store.tabs.find((tab) => tab.path === "/outside/notes.txt")?.dirty).toBe(true);
    expect(store.unsavedEditorFileCount).toBe(1);

    act(() => store.editContent("/outside/notes.txt", "v1"));
    expect(store.tabs.find((tab) => tab.path === "/outside/notes.txt")?.dirty).toBe(false);
    expect(store.unsavedEditorFileCount).toBe(0);
  });

  it("keeps inactive-workspace dirty files in the close warning count", async () => {
    const openExternal = vi.fn(async (): Promise<ExternalOpenResult> => ({
      kind: "standalone", path: "/outside/notes.txt", content: "v1"
    }));
    window.vela = api({ openExternal });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.openExternalPath("/outside/notes.txt"));
    act(() => store.editContent("/outside/notes.txt", "unfinished"));
    expect(store.unsavedEditorFileCount).toBe(1);

    await act(async () => store.openSession("/two"));

    expect(store.session?.directory).toBe("/two");
    expect(store.unsavedEditorFileCount).toBe(1);
  });

  it("opens a single file as a single-file workspace and marks the sidebar", async () => {
    const init = info("/parent", 7);
    const openFileWorkspace = vi.fn(async () => ({ session: init, path: "index.html" }));
    window.vela = api({ openFileWorkspace: openFileWorkspace as VelaApi["openFileWorkspace"] });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openFileWorkspace("/parent/index.html"));

    expect(store.session?.workspace.id).toBe(init.workspace.id);
    expect(store.singleFile).toBe("index.html");
    expect(store.tabs.map((t) => t.path)).toEqual(["index.html"]);
    expect(store.activePath).toBe("index.html");
  });

  it("imports dropped paths into a destination folder and refreshes the tree", async () => {
    const importExternal = vi.fn(async () => [
      { name: "a.txt", rel: "vendor/a.txt", imported: true }
    ]);
    const listDir = vi.fn(async () => []);
    window.vela = api({ importExternal, listDir });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.importPaths("vendor", ["/outside/a.txt"]));

    expect(importExternal).toHaveBeenCalledWith(store.session!.workspace, "vendor", ["/outside/a.txt"]);
    expect(listDir).toHaveBeenCalled();
  });

  it("imports dropped files and folders into the workspace root", async () => {
    const importExternal = vi.fn(async () => [
      { name: "folder", rel: "folder", imported: true },
      { name: "a.txt", rel: "a.txt", imported: true }
    ]);
    const listDir = vi.fn(async () => []);
    window.vela = api({ importExternal, listDir });
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    await act(async () => store.dropIntoExplorer(["/outside/folder", "/outside/a.txt"]));

    expect(importExternal).toHaveBeenCalledWith(store.session!.workspace, "", ["/outside/folder", "/outside/a.txt"]);
  });
});
