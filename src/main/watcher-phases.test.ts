import { promises as fsp } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import type { FileBaseline, WorkspaceIdentity } from "@shared/types";
import { LatestGeneration } from "@shared/generation";
import { VelaBackend, type SessionContext } from "./opencode";
import type { WorkspaceObservation } from "./workspace-observation";

vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));
vi.mock("electron", () => ({
  app: { getPath: () => "/tmp" },
  shell: { trashItem: vi.fn() }
}));

interface WatchContext {
  root: string;
  sessionID: string;
  workspace: WorkspaceIdentity;
  snapshots: Map<string, FileBaseline>;
  lastKnown: Map<string, string>;
  hasGit: boolean | null;
  timers: Map<string, ReturnType<typeof setTimeout>>;
}

type BackendHarness = {
  contexts: Map<string, SessionContext>;
  primary: string | null;
  observation: WorkspaceObservation<SessionContext>;
  handleServerEvent(type: string, data: unknown, location?: { directory?: string }): Promise<void>;
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

function sessionContext(workspace: WorkspaceIdentity, root: string): { context: WatchContext; entry: SessionContext } {
  const context: WatchContext = {
    root,
    sessionID: "session-one",
    workspace,
    snapshots: new Map<string, FileBaseline>(),
    lastKnown: new Map<string, string>(),
    hasGit: true,
    timers: new Map()
  };
  const entry: SessionContext = {
    workspace,
    sessionID: context.sessionID,
    directory: root,
    sessionInfo: { id: context.sessionID, directory: root, workspace },
    watchContext: context,
    watcher: null,
    activations: new LatestGeneration()
  };
  return { context, entry };
}

function harness(): { backend: VelaBackend; internals: BackendHarness; context: WatchContext; supersede: () => void } {
  const backend = new VelaBackend();
  const internals = backend as unknown as BackendHarness;
  const workspace = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
  const { context, entry } = sessionContext(workspace, "/workspace-one");
  internals.contexts = new Map([[workspace.id, entry]]);
  internals.primary = workspace.id;
  const supersede = (): void => {
    const next = { ...workspace, generation: workspace.generation + 1 };
    const { entry: replaced } = sessionContext(next, "/workspace-one");
    internals.contexts.set(workspace.id, replaced);
    internals.primary = workspace.id;
  };
  return { backend, internals, context, supersede };
}

describe("backend watcher generation phases", () => {
  it("drops routed filesystem work when the context is replaced during stat", async () => {
    const { backend, internals, context, supersede } = harness();
    const stat = deferred<Awaited<ReturnType<typeof fsp.stat>>>();
    vi.spyOn(fsp, "stat").mockReturnValueOnce(stat.promise);
    const messages: unknown[] = [];
    backend.onMessage((message) => messages.push(message));

    const pending = internals.handleServerEvent("filesystem.changed", { file: "file.txt", event: "change" }, { directory: context.root });
    supersede();
    stat.resolve({ isFile: () => true } as Awaited<ReturnType<typeof fsp.stat>>);
    await pending;

    expect(messages).toEqual([]);
    expect(context.lastKnown.size).toBe(0);
  });

  it("drops work when the context is replaced during read", async () => {
    const { backend, internals, context, supersede } = harness();
    vi.spyOn(fsp, "stat").mockResolvedValueOnce({ isFile: () => true } as Awaited<ReturnType<typeof fsp.stat>>);
    const read = deferred<string>();
    vi.spyOn(fsp, "readFile").mockReturnValueOnce(read.promise);
    const messages: unknown[] = [];
    backend.onMessage((message) => messages.push(message));

    const pending = internals.observation.onFsChanged(context, `${context.root}/file.txt`, "change");
    await Promise.resolve();
    supersede();
    read.resolve("new content");
    await pending;

    expect(messages).toEqual([]);
    expect(context.lastKnown.size).toBe(0);
  });

  it("drops mutation and emission when the context is replaced during Git baseline lookup", async () => {
    const { backend, internals, context, supersede } = harness();
    vi.spyOn(fsp, "stat").mockResolvedValueOnce({ isFile: () => true } as Awaited<ReturnType<typeof fsp.stat>>);
    vi.spyOn(fsp, "readFile").mockResolvedValueOnce("new content");
    const git = deferred<string | null>();
    vi.spyOn(internals.observation, "gitShow").mockReturnValueOnce(git.promise);
    const messages: unknown[] = [];
    backend.onMessage((message) => messages.push(message));

    const pending = internals.observation.onFsChanged(context, `${context.root}/file.txt`, "change");
    await Promise.resolve();
    await Promise.resolve();
    supersede();
    git.resolve("old content");
    await pending;

    expect(messages).toEqual([]);
    expect(context.snapshots.size).toBe(0);
  });

  it("drops deletion handling when the context is replaced during Git lookup", async () => {
    const { backend, internals, context, supersede } = harness();
    vi.spyOn(fsp, "stat").mockRejectedValueOnce(new Error("missing"));
    const git = deferred<string | null>();
    vi.spyOn(internals.observation, "gitShow").mockReturnValueOnce(git.promise);
    const messages: unknown[] = [];
    backend.onMessage((message) => messages.push(message));

    const pending = internals.observation.onFsChanged(context, `${context.root}/deleted.txt`, "unlink");
    await Promise.resolve();
    supersede();
    git.resolve("old content");
    await pending;

    expect(messages).toEqual([]);
    expect(context.snapshots.size).toBe(0);
  });
});

describe("watched file guards", () => {
  it("ignores agent/tooling directories so their internal writes never reach the renderer", () => {
    const { internals, context } = harness();
    const skip = (rel: string): boolean =>
      internals.observation.shouldSkip(`${context.root}/${rel}`, context.root);
    expect(skip(".opencode/session.jsonl")).toBe(true);
    expect(skip(".opencode/node_modules/zod/index.js")).toBe(true);
    expect(skip(".claude/foo")).toBe(true);
    expect(skip(".cursor/bar")).toBe(true);
    expect(skip("src/app.ts")).toBe(false);
    expect(skip("README.md")).toBe(false);
  });

  it("does not read or emit oversized files to avoid renderer memory and IPC floods", async () => {
    const { backend, internals, context } = harness();
    const readSpy = vi.spyOn(fsp, "readFile");
    vi.spyOn(fsp, "stat").mockResolvedValueOnce({
      isFile: () => true,
      size: 5 * 1024 * 1024
    } as Awaited<ReturnType<typeof fsp.stat>>);
    const messages: unknown[] = [];
    backend.onMessage((message) => messages.push(message));

    await internals.observation.onFsChanged(context, `${context.root}/huge.json`, "change");

    expect(readSpy).not.toHaveBeenCalled();
    expect(messages).toEqual([]);
  });
});
