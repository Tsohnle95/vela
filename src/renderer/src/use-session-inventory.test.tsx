import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, SessionSummary } from "@shared/types";
import type { VelaApi } from "../../preload";
import { useSessionInventory } from "./use-session-inventory";

const first: SessionInfo = { id: "first", directory: "/first", workspace: { id: "first-workspace", generation: 1 } };
const second: SessionInfo = { id: "second", directory: "/second", workspace: { id: "second-workspace", generation: 2 } };
const history: SessionSummary = { id: first.id, title: "First", directory: first.directory, updatedAt: 1 };
let inventory: ReturnType<typeof useSessionInventory>;
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, failed) => { resolve = done; reject = failed; });
  return { promise, resolve, reject };
}

describe("session inventory authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  const notify = vi.fn();
  function Probe() {
    inventory = useSessionInventory(notify);
    return null;
  }
  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    notify.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    window.vela = {
      activeSessions: vi.fn(async () => []),
      sessions: vi.fn(async () => [])
    } as unknown as VelaApi;
    await act(async () => root.render(<Probe />));
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("does not fetch by itself and preserves stale refresh return values without committing them", async () => {
    expect(window.vela.activeSessions).not.toHaveBeenCalled();
    expect(window.vela.sessions).not.toHaveBeenCalled();
    const older = deferred<SessionInfo[]>();
    const newer = deferred<SessionInfo[]>();
    window.vela.activeSessions = vi.fn().mockReturnValueOnce(older.promise).mockReturnValueOnce(newer.promise);
    const oldTask = inventory.refreshActiveSessions();
    const newTask = inventory.refreshActiveSessions();
    await act(async () => {
      newer.resolve([second]);
      await newTask;
      expect(inventory.activeSessionFor(second.id)).toBe(second);
    });
    let staleResult: SessionInfo[] = [];
    await act(async () => { older.resolve([first]); staleResult = await oldTask; });
    expect(staleResult).toEqual([first]);
    expect(inventory.activeSessions).toEqual([second]);
    expect(inventory.activeSessionFor(first.id)).toBeUndefined();
  });

  it("returns an empty failed refresh without clearing the known inventory", async () => {
    act(() => inventory.addActiveSession(first));
    window.vela.activeSessions = async () => { throw new Error("offline"); };
    let result: SessionInfo[] = [second];
    await act(async () => { result = await inventory.refreshActiveSessions(); });
    expect(result).toEqual([]);
    expect(inventory.activeSessions).toEqual([first]);
    expect(inventory.activeSessionFor(first.id)).toBe(first);
    expect(notify).not.toHaveBeenCalled();
  });

  it("protects newer history from stale success but still reports an older failed request", async () => {
    const older = deferred<SessionSummary[]>();
    const newer = deferred<SessionSummary[]>();
    window.vela.sessions = vi.fn().mockReturnValueOnce(older.promise).mockReturnValueOnce(newer.promise);
    const oldTask = inventory.loadSessions();
    const newTask = inventory.loadSessions();
    await act(async () => { newer.resolve([history]); await newTask; });
    await act(async () => { older.resolve([{ ...history, title: "Stale" }]); await oldTask; });
    expect(inventory.summaryFor(first.id)?.title).toBe("First");
    const failed = deferred<SessionSummary[]>();
    window.vela.sessions = vi.fn().mockReturnValueOnce(failed.promise).mockResolvedValueOnce([{ ...history, title: "Newest" }]);
    const failingTask = inventory.loadSessions();
    await act(async () => inventory.loadSessions());
    await act(async () => { failed.reject(new Error("old failure")); await failingTask; });
    expect(inventory.sessions[0].title).toBe("Newest");
    expect(notify).toHaveBeenCalledWith("old failure", "error");
  });

  it("keeps local upserts ordered and gives named readers the current ref snapshots", async () => {
    act(() => { inventory.addActiveSession(first); inventory.addActiveSession(second); });
    const before = inventory.activeSessions;
    act(() => inventory.addActiveSession(first));
    expect(inventory.activeSessions).toBe(before);
    const updated = { ...first, title: "Updated" };
    act(() => inventory.addActiveSession(updated));
    expect(inventory.activeSessions).toEqual([updated, second]);
    expect(inventory.activeSessionFor(first.id)).toBe(updated);
    act(() => inventory.removeActiveSession(first.id));
    expect(inventory.activeSessions).toEqual([second]);
    expect(inventory.activeSessionFor(first.id)).toBeUndefined();
    window.vela.sessions = async () => [history];
    await act(async () => inventory.loadSessions());
    expect(inventory.summaryFor(first.id)).toBe(history);
  });

  it("applies history events in order without changing refresh request authority", async () => {
    const loading = deferred<SessionSummary[]>();
    window.vela.sessions = async () => loading.promise;
    const task = inventory.loadSessions();
    const child: SessionSummary = { ...history, id: "child", parentID: first.id, title: "Child" };
    act(() => { inventory.upsertSummary(history); inventory.upsertSummary(child); });
    expect(inventory.sessions).toEqual([child, history]);
    act(() => inventory.upsertSummary({ ...child, title: "Updated child" }));
    act(() => inventory.renameSummary(first.id, "Renamed", 3));
    expect(inventory.sessions.map((item) => item.title)).toEqual(["Updated child", "Renamed"]);
    act(() => inventory.removeSummary(child.id));
    expect(inventory.summaryFor(child.id)).toBeUndefined();
    await act(async () => { loading.resolve([{ ...history, title: "Fetched" }]); await task; });
    expect(inventory.summaryFor(first.id)?.title).toBe("Fetched");
  });

  it("keeps history readers on the last committed render until event updates commit", async () => {
    window.vela.sessions = async () => [history];
    await act(async () => inventory.loadSessions());
    act(() => {
      inventory.renameSummary(first.id, "Renamed", 2);
      expect(inventory.summaryFor(first.id)).toBe(history);
    });
    expect(inventory.summaryFor(first.id)).toMatchObject({ title: "Renamed", updatedAt: 2 });
  });
});
