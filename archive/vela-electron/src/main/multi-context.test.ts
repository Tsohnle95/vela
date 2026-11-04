// @vitest-environment node
import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  app: { getPath: () => tmpdir() },
  shell: { trashItem: vi.fn(), openPath: async () => "" }
}));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { VelaBackend, SessionWorkspaceNotFoundError } from "./opencode";
import { SavedWorkspaceStore } from "./saved-workspaces";
import type { BackendMessage, WorkspaceIdentity } from "@shared/types";

const roots: string[] = [];

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function fixture(): Promise<{ backend: VelaBackend; messages: BackendMessage[] }> {
  const backend = new VelaBackend();
  const messages: BackendMessage[] = [];
  backend.onMessage((message) => messages.push(message as BackendMessage));
  return { backend, messages };
}

function clientWith(sessions: Record<string, string>): unknown {
  const directoryCreates = new Map<string, number>();
  return {
    session: {
      create: vi.fn(async ({ location }: { location: { directory: string } }) => {
        const candidates = Object.entries(sessions).filter(([, directory]) => directory === location.directory);
        const index = directoryCreates.get(location.directory) ?? 0;
        directoryCreates.set(location.directory, index + 1);
        return { id: candidates[index]?.[0] ?? `new-${location.directory.split("/").pop()}-${index}` };
      }),
      get: vi.fn(async ({ sessionID }: { sessionID: string }) => ({
        id: sessionID,
        location: { directory: sessions[sessionID] }
      }))
    },
    message: { list: vi.fn(async () => []) },
    file: { read: vi.fn(async () => "content") }
  };
}

describe("concurrent session contexts", () => {
  it("opens two sessions on different directories and routes events to the matching context", async () => {
    const one = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-one-")));
    const two = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-two-")));
    roots.push(one, two);
    await writeFile(path.join(one, "one.txt"), "one before");
    await writeFile(path.join(two, "two.txt"), "two before");
    const sessions = { "session-one": one, "session-two": two };
    const { backend, messages } = await fixture();
    const client = clientWith(sessions);
    const state = backend as unknown as { client: unknown };
    state.client = client;

    const first = await backend.openSession(one, 1);
    const second = await backend.openSession(two, 2);

    expect(first.id).toBe("session-one");
    expect(second.id).toBe("session-two");
    expect(await backend.activeSessions()).toHaveLength(2);

    const handle = (backend as unknown as {
      handleServerEvent: (
        type: string,
        data: unknown,
        location?: { directory?: string }
      ) => Promise<void>;
    }).handleServerEvent.bind(backend);
    messages.length = 0;
    await handle("filesystem.changed", { file: "two.txt", event: "change" }, { directory: two });
    await handle("filesystem.changed", { file: "one.txt", event: "change" }, { directory: one });

    const paths = messages
      .filter((message) => message.kind === "file-update")
      .map((message) => (message.kind === "file-update" ? message.file : null));
    expect(paths.map((file) => file?.path).sort()).toEqual(["one.txt", "two.txt"]);
    const byPath = new Map(paths.map((file) => [file?.path, file]));
    expect(byPath.get("one.txt")?.sessionID).toBe(first.id);
    expect(byPath.get("two.txt")?.sessionID).toBe(second.id);
    expect(byPath.get("one.txt")?.workspace.id).toBe(first.workspace.id);
    expect(byPath.get("two.txt")?.workspace.id).toBe(second.workspace.id);
  });

  it("reuses the context for an already-open session and keeps its workspace identity", async () => {
    const one = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-idem-")));
    roots.push(one);
    const { backend, messages } = await fixture();
    const client = clientWith({ "session-one": one });
    const state = backend as unknown as { client: unknown };
    state.client = client;

    const first = await backend.openSession(one, 1);
    messages.length = 0;
    const reopened = await backend.openSessionById(first.id, 2);

    expect(reopened.session.workspace).toEqual(first.workspace);
    expect(reopened.session.id).toBe(first.id);
    expect(messages).toEqual([]);
    expect(await backend.activeSessions()).toHaveLength(1);
  });

  it("keeps independent snapshot state for two sessions sharing one directory", async () => {
    const shared = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-shared-")));
    roots.push(shared);
    await writeFile(path.join(shared, "shared.txt"), "before");
    const { backend, messages } = await fixture();
    const sessions = { "session-one": shared, "session-two": shared };
    const client = clientWith(sessions);
    const state = backend as unknown as { client: unknown };
    state.client = client;

    const first = await backend.openSession(shared, 1);
    const second = await backend.openSession(shared, 2);
    expect(first.workspace.id).not.toBe(second.workspace.id);

    const handle = (backend as unknown as {
      handleServerEvent: (
        type: string,
        data: unknown,
        location?: { directory?: string }
      ) => Promise<void>;
    }).handleServerEvent.bind(backend);
    messages.length = 0;
    await handle("filesystem.changed", { file: "shared.txt", event: "change" }, { directory: shared });

    const updates = messages.filter((message) => message.kind === "file-update");
    expect(updates).toHaveLength(2);
    const sessionIDs = updates.map((message) => (message.kind === "file-update" ? message.file?.sessionID : null)).sort();
    expect(sessionIDs).toEqual([first.id, second.id].sort());
  });

  it("replaces the context when the same session re-activates on a different directory", async () => {
    const one = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-move-")));
    roots.push(one);
    await mkdir(path.join(one, "a"));
    await mkdir(path.join(one, "b"));
    const { backend, messages } = await fixture();
    const client = clientWith({ "session-one": path.join(one, "a") });
    const state = backend as unknown as { client: unknown };
    state.client = client;

    const first = await backend.openSession(path.join(one, "a"), 1);
    messages.length = 0;
    (client as { session: { get: (opts: { sessionID: string }) => Promise<unknown> } }).session.get = vi.fn(async () => ({
      id: "session-one",
      location: { directory: path.join(one, "b") }
    }));
    const moved = await backend.openSessionById(first.id, 2);

    expect(moved.session.directory).toBe(path.join(one, "b"));
    expect(moved.session.workspace.id).not.toBe(first.workspace.id);
    expect(await backend.activeSessions()).toHaveLength(1);
    expect(messages.map((message) => message.kind)).toEqual(["session", "recovery"]);
    await backend.stop();
  });

  it("repairs a session directory after its project folder is renamed", async () => {
    const parent = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-renamed-")));
    const current = path.join(parent, "project-current");
    const previous = path.join(parent, "project-previous");
    await mkdir(current);
    roots.push(parent);
    const { backend } = await fixture();
    const move = vi.fn(async () => {});
    (backend as unknown as { client: unknown }).client = {
      session: {
        get: vi.fn(async () => ({ id: "session-one", projectID: "project-one", location: { directory: previous } })),
        move
      },
      project: { list: vi.fn(async () => [{ id: "project-one", canonical: current }]) },
      message: { list: vi.fn(async () => []) }
    };

    const reopened = await backend.openSessionById("session-one", 1);

    expect(reopened.session.directory).toBe(current);
    expect(move).toHaveBeenCalledWith({ sessionID: "session-one", directory: current });
    await backend.stop();
  });

  it("relinks a closed session to a user-selected replacement workspace", async () => {
    const parent = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-relink-")));
    const selected = path.join(parent, "replacement");
    const missing = path.join(parent, "deleted");
    await mkdir(selected);
    roots.push(parent);
    const { backend } = await fixture();
    let directory = missing;
    const move = vi.fn(async ({ directory: next }: { directory: string }) => {
      directory = next;
    });
    (backend as unknown as { client: unknown }).client = {
      session: {
        get: vi.fn(async () => ({ id: "session-one", location: { directory } })),
        move
      },
      message: { list: vi.fn(async () => []) }
    };

    const reopened = await backend.openSessionByIdInDirectory("session-one", selected, 1);

    expect(move).toHaveBeenCalledWith({ sessionID: "session-one", directory: selected });
    expect(reopened.session.directory).toBe(selected);
    expect(reopened.session.id).toBe("session-one");
    await backend.stop();
  });

  it("repairs history, bookmarks and sibling sessions after a moved project and restart", async () => {
    const root = await realpath(await mkdtemp(path.join(tmpdir(), "vela-history-moved-")));
    roots.push(root);
    const previous = path.join(root, "portfolio", "neptune");
    const current = path.join(root, "neptune");
    await mkdir(path.join(current, ".git"), { recursive: true });
    await mkdir(path.join(current, "docs"));
    await writeFile(path.join(current, ".git", "opencode"), "neptune-id");
    const file = path.join(root, "saved.json");
    const bookmarks = new SavedWorkspaceStore(file);
    await bookmarks.save([{ directory: previous, name: "Neptune" }]);
    const locations: Record<string, string> = { one: previous, two: path.join(previous, "docs") };
    const move = vi.fn(async ({ sessionID, directory }: { sessionID: string; directory: string }) => { locations[sessionID] = directory; });
    const client = {
      project: { list: vi.fn(async () => [{ id: "neptune-id", canonical: previous }]) },
      session: {
        list: vi.fn(async () => Object.entries(locations).map(([id, directory]) => ({ id, projectID: "neptune-id", title: id, time: { updated: 1 }, location: { directory } }))),
        get: vi.fn(async ({ sessionID }: { sessionID: string }) => ({ id: sessionID, projectID: "neptune-id", location: { directory: locations[sessionID] } })),
        move
      },
      message: { list: vi.fn(async () => []) }
    };
    const backend = new VelaBackend(undefined, undefined, undefined, bookmarks);
    (backend as unknown as { client: unknown }).client = client;
    expect((await backend.listSessions()).map((session) => session.directory)).toEqual([current, path.join(current, "docs")]);
    expect(move).not.toHaveBeenCalled();
    expect(await bookmarks.read()).toEqual([{ directory: current, name: "Neptune" }]);
    expect((await backend.openSessionById("one", 1)).session.id).toBe("one");
    await backend.stop();

    const restarted = new VelaBackend(undefined, undefined, undefined, new SavedWorkspaceStore(file));
    (restarted as unknown as { client: unknown }).client = client;
    client.project.list.mockResolvedValue([]);
    const reopened = await restarted.openSessionById("two", 1);
    expect(reopened.session.directory).toBe(path.join(current, "docs"));
    expect(reopened.session.id).toBe("two");
    expect(move).toHaveBeenLastCalledWith({ sessionID: "two", directory: path.join(current, "docs") });
    await restarted.stop();
  });

  it("identifies a session whose old workspace cannot be resolved", async () => {
    const parent = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-missing-")));
    roots.push(parent);
    const { backend } = await fixture();
    (backend as unknown as { client: unknown }).client = {
      session: { get: vi.fn(async () => ({ id: "session-one", location: { directory: path.join(parent, "deleted") } })) }
    };

    await expect(backend.openSessionById("session-one", 1)).rejects.toBeInstanceOf(SessionWorkspaceNotFoundError);
  });

  it("omits missing projects and deduplicates canonical directories", async () => {
    const current = await realpath(await mkdtemp(path.join(tmpdir(), "vela-project-current-")));
    roots.push(current);
    const { backend } = await fixture();
    (backend as unknown as { client: unknown }).client = {
      project: { list: vi.fn(async () => [
        { canonical: path.join(current, "missing"), name: "Missing" },
        { canonical: current, name: "Current" },
        { canonical: current, name: "Duplicate" }
      ]) }
    };

    await expect(backend.listProjects()).resolves.toEqual([{ directory: current, name: "Current" }]);
  });

  it("routes tool snapshots to the session that called the tool", async () => {
    const one = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-tool-one-")));
    const two = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-tool-two-")));
    roots.push(one, two);
    await writeFile(path.join(one, "one.txt"), "one");
    await writeFile(path.join(two, "two.txt"), "two");
    const { backend } = await fixture();
    const sessions = { "session-one": one, "session-two": two };
    const client = clientWith(sessions);
    const state = backend as unknown as { client: unknown };
    state.client = client;

    const first = await backend.openSession(one, 1);
    await backend.openSession(two, 2);
    const contextMap = backend as unknown as {
      contexts: Map<string, { sessionID: string; watchContext: { snapshots: Map<string, unknown> } }>;
    };
    const firstContext = [...contextMap.contexts.values()].find((context) => context.sessionID === first.id)!;
    expect(firstContext.watchContext.snapshots.size).toBe(0);

    const handle = (backend as unknown as {
      handleServerEvent: (
        type: string,
        data: unknown,
        location?: { directory?: string }
      ) => Promise<void>;
    }).handleServerEvent.bind(backend);
    await handle("session.tool.called", { sessionID: first.id, input: { filePath: "one.txt" } });

    expect([...firstContext.watchContext.snapshots.keys()].some((key) => key.endsWith("one.txt"))).toBe(true);
    await backend.stop();
  });

  it("routes filesystem events reported through a symlinked root to the canonical context", async () => {
    const real = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-real-")));
    const links = await mkdtemp(path.join(tmpdir(), "vela-multi-links-"));
    roots.push(real, links);
    await writeFile(path.join(real, "note.txt"), "before");
    const link = path.join(links, "root");
    await symlink(real, link);
    const { backend, messages } = await fixture();
    const client = clientWith({ "session-one": link });
    const state = backend as unknown as { client: unknown };
    state.client = client;

    const first = await backend.openSession(link, 1);
    expect(first.directory).toBe(real);

    const handle = (backend as unknown as {
      handleServerEvent: (
        type: string,
        data: unknown,
        location?: { directory?: string }
      ) => Promise<void>;
    }).handleServerEvent.bind(backend);
    messages.length = 0;
    await handle("filesystem.changed", { file: "note.txt", event: "change" }, { directory: link });

    const updates = messages.filter((message) => message.kind === "file-update");
    expect(updates).toHaveLength(1);
    const file = updates[0].kind === "file-update" ? updates[0].file : null;
    expect(file?.path).toBe("note.txt");
    expect(file?.sessionID).toBe(first.id);
    await backend.stop();
  });

  it("closes a session context, stopping its watcher, and recreates a fresh context on reopen", async () => {
    const one = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-close-")));
    roots.push(one);
    const { backend } = await fixture();
    const client = clientWith({ "session-one": one });
    const state = backend as unknown as { client: unknown };
    state.client = client;

    const first = await backend.openSession(one, 1);
    const contextMap = backend as unknown as {
      contexts: Map<string, { watcher: { close: () => void } | null }>;
    };
    const firstContext = contextMap.contexts.get(first.workspace.id)!;
    expect(firstContext.watcher).toBeTruthy();
    const closeSpy = vi.spyOn(firstContext.watcher!, "close");

    await backend.closeSession(first.workspace);

    expect(closeSpy).toHaveBeenCalled();
    expect(contextMap.contexts.has(first.workspace.id)).toBe(false);
    expect(await backend.activeSessions()).toHaveLength(0);

    const reopened = await backend.openSessionById(first.id, 2);

    expect(reopened.session.workspace.id).not.toBe(first.workspace.id);
    expect(await backend.activeSessions()).toHaveLength(1);
    expect(contextMap.contexts.get(reopened.session.workspace.id)?.watcher).toBeTruthy();
    await backend.stop();
  });

  it("deletes an opencode session server-side and closes its context", async () => {
    const one = await realpath(await mkdtemp(path.join(tmpdir(), "vela-multi-delete-")));
    roots.push(one);
    const { backend } = await fixture();
    const remove = vi.fn(async () => {});
    const client = {
      ...(clientWith({ "session-one": one }) as Record<string, unknown>),
      session: {
        ...(clientWith({ "session-one": one }) as { session: Record<string, unknown> }).session,
        remove
      }
    };
    (backend as unknown as { client: unknown }).client = client;

    const first = await backend.openSession(one, 1);
    expect(await backend.activeSessions()).toHaveLength(1);

    await backend.deleteSession(first.id);

    expect(remove).toHaveBeenCalledWith({ sessionID: first.id });
    expect(await backend.activeSessions()).toHaveLength(0);
    await backend.stop();
  });

  it("refuses to delete DeepSeek sessions without a delete RPC", async () => {
    const dir = await realpath(await mkdtemp(path.join(tmpdir(), "vela-runtime-sessions-")));
    roots.push(dir);
    const { RuntimeSessionIndex } = await import("./runtimes/runtime-session-index");
    const { backend } = await fixture();
    const backendWithIndex = new VelaBackend(
      () => {},
      () => { throw new Error("no runtime"); },
      new RuntimeSessionIndex(path.join(dir, "runtime-sessions.json"))
    );
    void backend;
    (backendWithIndex as unknown as { client: unknown }).client = clientWith({});
    const index = (backendWithIndex as unknown as {
      runtimeSessionIndex: { put: (record: unknown) => Promise<void> };
    }).runtimeSessionIndex;
    await index.put({ id: "deepseek-one", runtimeID: "deepseek", title: "d", directory: "/tmp", updatedAt: Date.now() });

    await expect(backendWithIndex.deleteSession("deepseek-one")).rejects.toThrow("DeepSeek");
    await backendWithIndex.stop();
  });
});
