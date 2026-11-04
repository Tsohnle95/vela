import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectInfo, SavedWorkspaceSnapshot } from "@shared/types";
import type { VelaApi } from "../../preload";
import { useSavedWorkspaces } from "./use-saved-workspaces";

let bookmarks: ReturnType<typeof useSavedWorkspaces>;
const notify = vi.fn();
function Probe({ connected }: { connected: boolean }) {
  bookmarks = useSavedWorkspaces(connected, notify);
  return null;
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("saved workspace lifecycle", () => {
  let root: Root;
  let container: HTMLDivElement;
  let save: ReturnType<typeof vi.fn>;
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    notify.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    save = vi.fn(async (workspaces: ProjectInfo[]) => workspaces);
    window.vela = {
      savedWorkspaces: async () => ({ workspaces: [], initialized: true }),
      saveSavedWorkspaces: save,
      projects: vi.fn(async () => []),
      selectDirectory: async () => "/selected"
    } as unknown as VelaApi;
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("keeps failed reads untouched even when local bookmarks change", async () => {
    window.localStorage.setItem("vela.savedWorkspaces", JSON.stringify([{ directory: "/legacy", name: "Legacy" }]));
    window.vela.savedWorkspaces = vi.fn(async () => { throw new Error("unavailable"); });
    await act(async () => root.render(<Probe connected={false} />));
    await act(async () => bookmarks.saveWorkspace());
    expect(bookmarks.savedWorkspaces.map((item) => item.directory)).toEqual(["/legacy", "/selected"]);
    expect(save).not.toHaveBeenCalled();
    expect(JSON.parse(window.localStorage.getItem("vela.savedWorkspaces")!)).toEqual([{ directory: "/legacy", name: "Legacy" }]);
    expect(notify).toHaveBeenCalledWith(expect.stringContaining("stored data was left untouched"), "error");
  });

  it("cancels an older hydration on reconnect and replays pending mutations once", async () => {
    const old = deferred<SavedWorkspaceSnapshot>();
    const latest = deferred<SavedWorkspaceSnapshot>();
    window.vela.savedWorkspaces = vi.fn().mockReturnValueOnce(old.promise).mockReturnValueOnce(latest.promise);
    await act(async () => root.render(<Probe connected={false} />));
    act(() => bookmarks.rememberWorkspace("/new"));
    act(() => bookmarks.replaceSavedWorkspace("/old", "/replacement"));
    await act(async () => root.render(<Probe connected />));
    await act(async () => old.resolve({ initialized: true, workspaces: [{ directory: "/stale", name: "Stale" }] }));
    expect(save).not.toHaveBeenCalled();
    await act(async () => latest.resolve({ initialized: true, workspaces: [{ directory: "/old", name: "Old" }] }));
    expect(bookmarks.savedWorkspaces.map((item) => item.directory)).toEqual(["/new", "/replacement"]);
    expect(save).toHaveBeenLastCalledWith(bookmarks.savedWorkspaces);
  });

  it("protects hydrated local mutations from a reconnect snapshot", async () => {
    const reconnect = deferred<SavedWorkspaceSnapshot>();
    window.vela.savedWorkspaces = vi.fn()
      .mockResolvedValueOnce({ initialized: true, workspaces: [{ directory: "/old", name: "Old" }] })
      .mockReturnValueOnce(reconnect.promise);
    await act(async () => root.render(<Probe connected={false} />));
    await act(async () => root.render(<Probe connected />));
    act(() => bookmarks.removeWorkspace("/old"));
    act(() => bookmarks.rememberWorkspace("/new"));
    await act(async () => reconnect.resolve({ initialized: true, workspaces: [{ directory: "/old", name: "Old" }] }));
    expect(bookmarks.savedWorkspaces).toEqual([{ directory: "/new", name: "new" }]);
  });

  it("reports missing bridge and write failures without changing bookmark authority", async () => {
    window.vela.savedWorkspaces = undefined as unknown as VelaApi["savedWorkspaces"];
    await act(async () => root.render(<Probe connected={false} />));
    expect(notify).toHaveBeenCalledWith("Restart Vela to load saved workspaces from app storage.", "error");
    expect(save).not.toHaveBeenCalled();
    window.vela.savedWorkspaces = async () => ({ initialized: true, workspaces: [{ directory: "/durable", name: "Durable" }] });
    save.mockRejectedValue(new Error("write failed"));
    await act(async () => root.render(<Probe connected />));
    expect(bookmarks.savedWorkspaces).toEqual([{ directory: "/durable", name: "Durable" }]);
    expect(notify).toHaveBeenCalledWith(expect.stringContaining("could not be written"), "error");
  });

  it("keeps legacy data when the app process lacks the saved-workspace handler", async () => {
    window.localStorage.setItem("vela.savedWorkspaces", JSON.stringify([{ directory: "/legacy", name: "Legacy" }]));
    window.vela.savedWorkspaces = async () => { throw new Error("No handler registered for 'shell:saved-workspaces'"); };
    await act(async () => root.render(<Probe connected={false} />));
    expect(bookmarks.savedWorkspaces).toEqual([{ directory: "/legacy", name: "Legacy" }]);
    expect(save).not.toHaveBeenCalled();
    expect(notify).toHaveBeenCalledWith(expect.stringContaining("Cmd+R only refreshes the window"), "error");
  });
});
