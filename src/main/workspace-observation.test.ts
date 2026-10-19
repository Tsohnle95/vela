// @vitest-environment node
import { promises as fsp } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, expect, it, vi } from "vitest";
import type { BackendMessage, WorkspaceIdentity } from "@shared/types";
import { WorkspaceObservation, type WatchSession } from "./workspace-observation";

const roots: string[] = [];
const sessions: WatchSession[] = [];
const active = new Map<string, WatchSession>();
const messages: BackendMessage[] = [];
const observation = new WorkspaceObservation<WatchSession>(
  (context) => active.get(context.workspace.id)?.watchContext === context,
  (session) => active.get(session.watchContext.workspace.id) === session,
  (message) => messages.push(message as BackendMessage)
);

afterEach(async () => {
  for (const session of sessions.splice(0)) observation.stopWatcher(session);
  vi.restoreAllMocks();
  vi.useRealTimers();
  active.clear();
  messages.length = 0;
  await Promise.all(roots.splice(0).map((root) => fsp.rm(root, { recursive: true, force: true })));
});

async function directory(): Promise<string> {
  const root = await fsp.realpath(await fsp.mkdtemp(path.join(tmpdir(), "vela-observation-")));
  roots.push(root);
  return root;
}

function open(root: string, generation: number): WatchSession {
  const workspace: WorkspaceIdentity = { id: "11111111-1111-4111-8111-111111111111", generation };
  const watchContext = observation.createContext(root, "session", workspace);
  watchContext.hasGit = false;
  const session: WatchSession = { directory: root, watchContext, watcher: null };
  active.set(workspace.id, session);
  sessions.push(session);
  observation.startWatcher(session);
  expect(session.watcher).not.toBeNull();
  return session;
}

it.skipIf(process.env.VELA_NATIVE_WATCH_TESTS !== "1")("closes native watchers and pending work, then reopens with fresh observation maps", async () => {
  const root = await directory();
  const file = path.join(root, "file.txt");
  await fsp.writeFile(file, "before");
  const first = open(root, 1);
  await fsp.writeFile(file, "first activation");
  await vi.waitFor(() => expect(messages).toContainEqual(expect.objectContaining({
    kind: "file-update", file: expect.objectContaining({ content: "first activation", workspace: first.watchContext.workspace })
  })), { timeout: 3000 });
  await fsp.writeFile(file, "pending close");
  observation.stopWatcher(first);
  active.delete(first.watchContext.workspace.id);
  expect(first.watcher).toBeNull();
  expect(first.watchContext.timers.size).toBe(0);
  const second = open(root, 2);
  expect(second.watchContext).not.toBe(first.watchContext);
  expect(second.watchContext.snapshots).not.toBe(first.watchContext.snapshots);
  expect(second.watchContext.lastKnown).not.toBe(first.watchContext.lastKnown);
  expect(second.watchContext.snapshots.size).toBe(0);
  messages.length = 0;
  await fsp.writeFile(file, "reopened activation");
  await vi.waitFor(() => expect(messages).toContainEqual(expect.objectContaining({
    kind: "file-update", file: expect.objectContaining({ content: "reopened activation", workspace: second.watchContext.workspace })
  })), { timeout: 3000 });
  expect(messages.every((message) => message.kind !== "file-update" || message.file?.workspace.generation === 2)).toBe(true);
  expect(first.watchContext.lastKnown.get(file)).toBe("first activation");
});

it.skipIf(process.env.VELA_NATIVE_WATCH_TESTS !== "1")("correlates native rename and directory move notifications using opened-file identity", async () => {
  const root = await directory();
  await fsp.mkdir(path.join(root, "nested"));
  const old = path.join(root, "old.txt");
  const renamed = path.join(root, "renamed.txt");
  const moved = path.join(root, "nested", "renamed.txt");
  await fsp.writeFile(old, "same bytes");
  const session = open(root, 1);
  observation.recordReadIdentity(session.watchContext, old, await fsp.stat(old));
  await fsp.rename(old, renamed);
  await vi.waitFor(() => expect(messages).toContainEqual(expect.objectContaining({
    kind: "file-update", file: expect.objectContaining({ path: "renamed.txt", movedFrom: "old.txt", content: "same bytes" })
  })), { timeout: 3000 });
  await fsp.rename(renamed, moved);
  await vi.waitFor(() => expect(messages).toContainEqual(expect.objectContaining({
    kind: "file-update", file: expect.objectContaining({ path: "nested/renamed.txt", movedFrom: "renamed.txt", content: "same bytes" })
  })), { timeout: 3000 });
  expect(session.watchContext.identities?.has(old)).toBe(false);
  expect(session.watchContext.identities?.has(renamed)).toBe(false);
  expect(session.watchContext.identities?.get(moved)).toBeTruthy();
});

it("preserves rename and move destination timing when a listener closes the activation during deletion", () => {
  const workspace: WorkspaceIdentity = { id: "test", generation: 1 };
  const context = observation.createContext("/workspace", "session", workspace);
  const session: WatchSession = { directory: context.root, watchContext: context, watcher: null };
  const baseline = { kind: "known" as const, content: "before" };
  const captured = { baseline, content: "after" };
  const closingObserver = new WorkspaceObservation<WatchSession>(
    () => active.has(workspace.id), () => true, () => active.delete(workspace.id)
  );
  active.set(workspace.id, session);
  closingObserver.recordRename(context, "/workspace/old", "/workspace/new", captured);
  expect(context.snapshots.get("/workspace/new")).toBe(baseline);
  expect(context.lastKnown.has("/workspace/new")).toBe(false);
  active.set(workspace.id, session);
  closingObserver.recordMove(context, "/workspace/old", "/workspace/moved", captured);
  expect(context.snapshots.has("/workspace/moved")).toBe(false);
  expect(context.lastKnown.has("/workspace/moved")).toBe(false);
});

it("ignores filesystem events for a path while Vela is writing it", async () => {
  vi.useFakeTimers();
  const context = observation.createContext("/workspace", "session", { id: "test", generation: 1 });
  const session: WatchSession = { directory: context.root, watchContext: context, watcher: null };
  active.set(context.workspace.id, session);
  sessions.push(session);
  const changed = vi.spyOn(observation, "onFsChanged").mockResolvedValue(undefined);
  const schedule = (observation as unknown as {
    scheduleWatch(context: typeof session.watchContext, abs: string, event: string): void;
  }).scheduleWatch.bind(observation);
  const file = path.join(context.root, "file.txt");

  observation.beginWrite(context, file);
  schedule(context, file, "change");
  await vi.advanceTimersByTimeAsync(300);
  expect(changed).not.toHaveBeenCalled();

  observation.endWrite(context, file);
  schedule(context, file, "change");
  await vi.advanceTimersByTimeAsync(300);
  expect(changed).toHaveBeenCalledExactlyOnceWith(context, file, "change");
});

it("ignores direct observation of a path while Vela is writing it", async () => {
  const context = observation.createContext("/workspace", "session", { id: "test", generation: 1 });
  const session: WatchSession = { directory: context.root, watchContext: context, watcher: null };
  active.set(context.workspace.id, session);
  sessions.push(session);
  const file = path.join(context.root, "file.txt");
  context.lastKnown.set(file, "saved");
  context.snapshots.set(file, { kind: "known", content: "saved" });

  observation.beginWrite(context, file);
  await observation.onFsChanged(context, file, "unlink");
  expect(messages).toEqual([]);

  observation.endWrite(context, file);
  await observation.onFsChanged(context, file, "unlink");
  expect(messages).toHaveLength(1);
});

it("coalesces at 200ms, suppresses imported destinations, and cancels pending callbacks on close", async () => {
  vi.useFakeTimers();
  const context = observation.createContext("/workspace", "session", { id: "test", generation: 1 });
  const session: WatchSession = { directory: context.root, watchContext: context, watcher: null };
  active.set(context.workspace.id, session);
  sessions.push(session);
  const changed = vi.spyOn(observation, "onFsChanged").mockResolvedValue(undefined);
  const schedule = (observation as unknown as {
    scheduleWatch(context: typeof session.watchContext, abs: string, event: string): void;
  }).scheduleWatch.bind(observation);
  const file = path.join(context.root, "file.txt");
  const imported = path.join(context.root, "imported");
  schedule(context, file, "rename");
  await vi.advanceTimersByTimeAsync(100);
  schedule(context, file, "change");
  await vi.advanceTimersByTimeAsync(199);
  expect(changed).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(1);
  expect(changed).toHaveBeenCalledExactlyOnceWith(context, file, "change");
  const restart = observation.beginImport(session);
  schedule(context, file, "change");
  expect(context.timers.size).toBe(0);
  observation.endImport(session, imported, restart);
  schedule(context, path.join(imported, "file.txt"), "change");
  expect(context.timers.size).toBe(0);
  schedule(context, file, "change");
  expect(context.timers.size).toBe(1);
  observation.stopWatcher(session);
  await vi.advanceTimersByTimeAsync(200);
  expect(context.timers.size).toBe(0);
  expect(changed).toHaveBeenCalledTimes(1);
});
