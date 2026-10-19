// @vitest-environment node
import { createHash } from "node:crypto";
import { promises as fsp } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, expect, it, vi } from "vitest";
import { LatestGeneration } from "@shared/generation";
import type { BackendMessage, FileWriteIdentity, WorkspaceIdentity } from "@shared/types";
import { VelaBackend, type SessionContext } from "./opencode";

vi.mock("electron", () => ({ app: { getPath: () => tmpdir() }, shell: { trashItem: async (target: string) => fsp.rm(target, { recursive: true }) } }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

const roots: string[] = [];
const backends: VelaBackend[] = [];
const workspace: WorkspaceIdentity = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };

afterEach(async () => {
  vi.restoreAllMocks();
  await Promise.all(backends.splice(0).map((backend) => backend.stop()));
  await Promise.all(roots.splice(0).map((root) => fsp.rm(root, { recursive: true, force: true })));
});

function entry(root: string, identity: WorkspaceIdentity): SessionContext {
  return {
    workspace: identity, sessionID: "session", directory: root,
    sessionInfo: { id: "session", directory: root, workspace: identity },
    watchContext: { root, workspace: identity, sessionID: "session", snapshots: new Map(), lastKnown: new Map(), hasGit: false, timers: new Map() },
    watcher: null, activations: new LatestGeneration()
  };
}

async function fixture(phase?: ConstructorParameters<typeof VelaBackend>[0]) {
  const root = await fsp.realpath(await fsp.mkdtemp(path.join(tmpdir(), "vela-mutation-owner-")));
  const key = createHash("sha256").update(root).digest("hex").slice(0, 16);
  roots.push(root, path.join(path.dirname(root), `.vela-recovery-${key}`));
  const backend = new VelaBackend(phase);
  backends.push(backend);
  const state = backend as unknown as { contexts: Map<string, SessionContext>; mutations: { run: (workspace: WorkspaceIdentity, operation: () => Promise<unknown>) => Promise<unknown> } };
  state.contexts.set(workspace.id, entry(root, workspace));
  const messages: BackendMessage[] = [];
  backend.onMessage((message) => messages.push(message as BackendMessage));
  return { backend, root, state, messages };
}

function writeIdentity(expectedContent: string): FileWriteIdentity {
  return { id: "save", workspaceID: workspace.id, revision: 1, expectedContent, overwrite: false };
}

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

it("rejects invalid save content and write identity before entering the workspace queue", async () => {
  const { backend, state } = await fixture();
  const queued = vi.spyOn(state.mutations, "run");
  await expect(backend.writeFile(workspace, "file.txt", "text", { ...writeIdentity("before"), revision: -1 }))
    .rejects.toThrow("invalid file write identity");
  await expect(backend.writeFile(workspace, "file.txt", "x".repeat(8 * 1024 * 1024 + 1), writeIdentity("before")))
    .rejects.toThrow("invalid or oversized file content");
  expect(queued).not.toHaveBeenCalled();
});

it("rechecks queued old generations while a replacement generation proceeds independently", async () => {
  const held = deferred();
  const reached = deferred();
  const { backend, root, state, messages } = await fixture(async (phase) => {
    if (phase === "save:temporary-ready") { reached.resolve(); await held.promise; }
  });
  await fsp.writeFile(path.join(root, "save.txt"), "before");
  const saving = backend.writeFile(workspace, "save.txt", "after", writeIdentity("before"));
  const savingResult = saving.catch((error: Error) => error);
  await reached.promise;
  const queued = backend.createFile(workspace, "stale.txt");
  const queuedResult = queued.catch((error: Error) => error);
  try {
    const replacement = { ...workspace, generation: 2 };
    state.contexts.set(workspace.id, entry(root, replacement));
    await backend.createFile(replacement, "fresh.txt");
    expect(await fsp.readFile(path.join(root, "fresh.txt"), "utf8")).toBe("");
    expect(messages).toHaveLength(1);
    expect(messages[0]).toMatchObject({ kind: "file-update", file: { workspace: replacement } });
  } finally {
    held.resolve();
    await Promise.allSettled([saving, queued]);
  }
  expect(await savingResult).toBeInstanceOf(Error);
  expect((await savingResult as Error).message).toContain("stale workspace; recovery artifacts preserved");
  expect((await queuedResult as Error).message).toBe("stale workspace");
  expect(await fsp.readFile(path.join(root, "save.txt"), "utf8")).toBe("before");
  expect(await fsp.stat(path.join(root, "stale.txt")).catch(() => null)).toBeNull();
  expect(messages).toHaveLength(1);
});

it("preserves the backend phase receiver and save failure file-update, recovery, rejection order", async () => {
  let backend!: VelaBackend;
  const events: string[] = [];
  const result = await fixture(function (this: unknown, phase) {
    expect(this).toBe(backend);
    if (phase === "save:temporary-ready") throw new Error("phase failure");
  });
  backend = result.backend;
  await fsp.writeFile(path.join(result.root, "file.txt"), "before");
  backend.onMessage((message) => events.push((message as BackendMessage).kind));
  await expect(backend.writeFile(workspace, "file.txt", "after", writeIdentity("before")).catch((error) => {
    events.push("rejected");
    throw error;
  })).rejects.toThrow("phase failure; recovery artifacts preserved");
  expect(events).toEqual(["file-update", "recovery", "rejected"]);
  expect(result.messages[0]).toMatchObject({ file: { content: "before", deleted: false } });
  expect(await fsp.readFile(path.join(result.root, "file.txt"), "utf8")).toBe("before");
});

it("preserves rename failure recovery-only emission and phase receiver without overwriting occupied files", async () => {
  let backend!: VelaBackend;
  const phases: string[] = [];
  const result = await fixture(function (this: unknown, phase) { expect(this).toBe(backend); phases.push(phase); });
  backend = result.backend;
  await fsp.writeFile(path.join(result.root, "source.txt"), "source");
  await fsp.writeFile(path.join(result.root, "target.txt"), "target");
  const events: string[] = [];
  backend.onMessage((message) => events.push((message as BackendMessage).kind));
  await expect(backend.renamePath(workspace, "source.txt", "target.txt").catch((error) => {
    events.push("rejected");
    throw error;
  })).rejects.toThrow("destination already exists: target.txt");
  expect(events).toEqual(["recovery", "rejected"]);
  expect(phases).toEqual(["rename:source-inspected", "rename:source-held"]);
  expect(await fsp.readFile(path.join(result.root, "source.txt"), "utf8")).toBe("source");
  expect(await fsp.readFile(path.join(result.root, "target.txt"), "utf8")).toBe("target");
});

async function rejectionOrder(result: Promise<unknown>, markerAfter: number): Promise<string[]> {
  const events: string[] = [];
  const rejected = result.catch(() => { events.push("rejected"); });
  await new Promise<void>((resolve) => {
    let tick = 0;
    const next = (): void => {
      tick += 1;
      if (tick < markerAfter) queueMicrotask(next);
      else { events.push("marker"); resolve(); }
    };
    queueMicrotask(next);
  });
  await rejected;
  return events;
}

it("preserves public invalid-save rejection before the next queued marker", async () => {
  const { backend } = await fixture();
  const result = backend.writeFile(workspace, "file.txt", "text", { ...writeIdentity("before"), revision: -1 });
  expect(await rejectionOrder(result, 1)).toEqual(["rejected", "marker"]);
});

it("preserves public stale-generation rejection before the original coordinator's queued marker", async () => {
  const { backend } = await fixture();
  const result = backend.createFile({ ...workspace, generation: 2 }, "stale.txt");
  expect(await rejectionOrder(result, 5)).toEqual(["rejected", "marker"]);
});
