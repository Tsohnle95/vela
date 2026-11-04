// @vitest-environment node
import { LEGACY_RECOVERY_DIR, RecoveryStorage } from "./recovery-storage";
import { chmod, mkdir, mkdtemp, open, readFile, readdir, realpath, rename, rm, symlink, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  app: {
    getPath: () => tmpdir()
  },
  shell: {
    trashItem: async (target: string) => rm(target, { recursive: true }),
    openPath: async () => ""
  }
}));
vi.mock("@opencode/client", () => ({
  OpenCode: { make: vi.fn() }
}));
vi.mock("@opencode/client/service", () => ({
  Service: {}
}));

import { VelaBackend, type SessionContext } from "./opencode";
import type { BackendMessage, FileBaseline, WorkspaceIdentity } from "@shared/types";
import { LatestGeneration } from "@shared/generation";

const roots: string[] = [];
const workspace: WorkspaceIdentity = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
function observation(backend: VelaBackend) {
  return (backend as unknown as { observation: import("./workspace-observation").WorkspaceObservation<SessionContext> }).observation;
}

afterEach(async () => {
  await Promise.all(roots.splice(0).flatMap((root) => [root, recoveryRootFor(root)].map((location) => rm(location, { recursive: true, force: true }))));
});

function recoveryRootFor(root: string): string {
  const key = createHash("sha256").update(root).digest("hex").slice(0, 16);
  return path.join(path.dirname(root), `.vela-recovery-${key}`);
}

async function backendFixture(
  mutationPhase?: ConstructorParameters<typeof VelaBackend>[0]
): Promise<{
  backend: VelaBackend;
  root: string;
  messages: BackendMessage[];
}> {
  const root = await realpath(await mkdtemp(path.join(tmpdir(), "vela-mutation-")));
  roots.push(root);
  const backend = new VelaBackend(mutationPhase);
  const context = {
    root,
    sessionID: "session",
    workspace,
    snapshots: new Map<string, FileBaseline>(),
    lastKnown: new Map<string, string>(),
    hasGit: false,
    timers: new Map()
  };
  const state = backend as unknown as {
    contexts: Map<string, SessionContext>;
    primary: string | null;
  };
  state.contexts = new Map([[workspace.id, {
    workspace,
    sessionID: "session",
    directory: root,
    sessionInfo: { id: "session", directory: root, workspace },
    watchContext: context,
    watcher: null,
    activations: new LatestGeneration()
  }]]);
  state.primary = workspace.id;
  const messages: BackendMessage[] = [];
  backend.onMessage((message) => messages.push(message as BackendMessage));
  return { backend, root, messages };
}

const writeIdentity = (expectedContent: string) => ({
  id: "save-1",
  workspaceID: workspace.id,
  revision: 1,
  expectedContent,
  overwrite: false
});

async function recoveryContents(root: string): Promise<string[]> {
  const recoveryRoot = recoveryRootFor(root);
  const transactions = await readdir(recoveryRoot).catch(() => []);
  const contents: string[] = [];
  for (const transaction of transactions) {
    for (const artifact of ["temporary", "original", "proposed", "source"]) {
      const content = await readFile(path.join(recoveryRoot, transaction, artifact), "utf8").catch(() => null);
      if (content !== null) contents.push(content);
    }
  }
  return contents;
}

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;

async function ageTransaction(recoveryRoot: string, transaction: string, update: {
  ageMs: number;
  phase?: string;
  acknowledged?: string[];
}): Promise<void> {
  const manifestPath = path.join(recoveryRoot, transaction, "manifest.json");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as {
    createdAt: number;
    phase: string;
    acknowledged: string[];
  };
  manifest.createdAt = Date.now() - update.ageMs;
  if (update.phase !== undefined) manifest.phase = update.phase;
  if (update.acknowledged !== undefined) manifest.acknowledged = update.acknowledged;
  await writeFile(manifestPath, JSON.stringify(manifest));
}

function purgeExpired(root: string): Promise<void> {
  return new RecoveryStorage().purgeExpiredRecovery(root, () => true);
}

describe("direct mutation observed changes", () => {
  it("rejects a change before the source is held and restores the changed source", async () => {
    const { backend, root } = await backendFixture(async (phase, source) => {
      if (phase === "save:temporary-ready") await writeFile(source, "external-before-hold");
    });
    await writeFile(path.join(root, "save.txt"), "expected");

    await expect(backend.writeFile(workspace, "save.txt", "editor", writeIdentity("expected")))
      .rejects.toThrow("file changed on disk");

    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("external-before-hold");
    expect((await recoveryContents(root)).sort()).toEqual(["editor", "editor", "external-before-hold"].sort());
  });

  it("does not surface its own save as an external change", async () => {
    const holder: { backend?: VelaBackend } = {};
    const { backend, root, messages } = await backendFixture(async (phase, _source, target) => {
      if (phase !== "save:source-held" || !target) return;
      const context = (holder.backend as unknown as { contexts: Map<string, SessionContext> })
        .contexts.get(workspace.id)!.watchContext;
      // OpenCode's filesystem.changed (and the local watcher) can observe the
      // displaced file mid-save; that must not reach the renderer as an external change.
      await observation(holder.backend!).onFsChanged(context, target, "rename");
    });
    holder.backend = backend;
    const file = path.join(root, "save.txt");
    await writeFile(file, "expected");
    // Vela has observed the file (e.g. it is open in the editor), so a missing
    // path during the save would otherwise surface as an external deletion.
    observation(backend).recordSnapshot(
      (backend as unknown as { contexts: Map<string, SessionContext> }).contexts.get(workspace.id)!.watchContext,
      file,
      "expected"
    );
    messages.length = 0;

    await backend.writeFile(workspace, "save.txt", "editor", writeIdentity("expected"));

    expect(messages.flatMap((message) => (message.kind === "file-update" ? [message.file] : []))).toEqual([
      expect.objectContaining({ path: "save.txt", content: "editor", deleted: false, write: expect.objectContaining({ id: "save-1" }) })
    ]);
  });

  it("preserves held, proposed, and concurrently recreated files after the hold", async () => {
    const { backend, root } = await backendFixture(async (phase, _holding, target) => {
      if (phase === "save:source-held") await writeFile(target, "external-after-hold", { flag: "wx" });
    });
    await writeFile(path.join(root, "save.txt"), "expected");

    await expect(backend.writeFile(workspace, "save.txt", "editor", writeIdentity("expected")))
      .rejects.toThrow("recovery artifacts preserved");

    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("external-after-hold");
    expect((await recoveryContents(root)).sort()).toEqual(["editor", "editor", "expected"]);
  });

  it("preserves held, proposed, and concurrently recreated files after validation", async () => {
    const { backend, root } = await backendFixture(async (phase, _holding, target) => {
      if (phase === "save:held-validated") await writeFile(target, "external-after-validation", { flag: "wx" });
    });
    await writeFile(path.join(root, "save.txt"), "expected");

    await expect(backend.writeFile(workspace, "save.txt", "editor", writeIdentity("expected")))
      .rejects.toThrow("recovery artifacts preserved");

    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("external-after-validation");
    expect((await recoveryContents(root)).sort()).toEqual(["editor", "editor", "expected"]);
  });

  it("does not overwrite a target recreated after installation", async () => {
    const { backend, root } = await backendFixture(async (phase, _temporary, target) => {
      if (phase === "save:target-installed") {
        await unlink(target);
        await writeFile(target, "external-after-install", { flag: "wx" });
      }
    });
    await writeFile(path.join(root, "save.txt"), "expected");

    await expect(backend.writeFile(workspace, "save.txt", "editor", writeIdentity("expected")))
      .rejects.toThrow("recovery artifacts preserved");

    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("external-after-install");
    expect((await recoveryContents(root)).sort()).toEqual(["editor", "editor", "expected"]);
  });

  it("keeps the displaced inode linked so late open-handle writes remain recoverable", async () => {
    let sourceHandle: Awaited<ReturnType<typeof open>> | null = null;
    const { backend, root } = await backendFixture(async (phase, source) => {
      if (phase === "save:temporary-ready") sourceHandle = await open(source, "r+");
      if (phase === "save:target-installed") await sourceHandle!.write("late-open-handle", 0, "utf8");
    });
    await writeFile(path.join(root, "save.txt"), "expected-original-value");

    await backend.writeFile(workspace, "save.txt", "editor", writeIdentity("expected-original-value"));
    await sourceHandle!.close();

    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("editor");
    expect(await recoveryContents(root)).toContain("late-open-handlel-value");
  });

  it("persists acknowledgment without deleting artifact bytes and rejects forged ids", async () => {
    const { backend, root } = await backendFixture();
    await writeFile(path.join(root, "save.txt"), "original");
    await backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"));
    const savedRecords = await backend.listRecovery(workspace);
    expect(savedRecords.every((record) => record.acknowledged)).toBe(true);
    const original = savedRecords.find((record) => record.artifact === "original")!;

    await backend.acknowledgeRecovery(workspace, original.id);

    expect((await backend.listRecovery(workspace)).find((record) => record.id === original.id)?.acknowledged).toBe(true);
    expect(await recoveryContents(root)).toContain("original");
    await expect(backend.openRecovery(workspace, "../../outside:original")).rejects.toThrow("recovery record not found");
    await expect(backend.acknowledgeRecovery(workspace, "../../outside:original")).rejects.toThrow("invalid recovery record");
  });

  it("stores Git workspace recovery in metadata rather than project files", async () => {
    const { backend, root } = await backendFixture();
    await mkdir(path.join(root, ".git"));
    await writeFile(path.join(root, "save.txt"), "original");

    await backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"));

    expect(await readdir(path.join(root, ".git", "vela-recovery"))).toHaveLength(1);
    expect(await readdir(root)).toEqual([".git", "save.txt"]);
    expect((await backend.listRecovery(workspace)).some((record) => record.originalPath === "save.txt")).toBe(true);
  });

  it("can still inspect and acknowledge legacy workspace-local recovery", async () => {
    const { backend, root } = await backendFixture();
    await writeFile(path.join(root, "save.txt"), "original");
    await backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"));
    await rename(recoveryRootFor(root), path.join(root, LEGACY_RECOVERY_DIR));

    const record = (await backend.listRecovery(workspace)).find((item) => item.artifact === "original")!;
    expect(record.recoveryPath).toContain(LEGACY_RECOVERY_DIR);
    await backend.openRecovery(workspace, record.id);
    await backend.acknowledgeRecovery(workspace, record.id);
    expect((await backend.listRecovery(workspace)).find((item) => item.id === record.id)?.acknowledged).toBe(true);
    const state = backend as unknown as { client: unknown };
    state.client = { session: { create: vi.fn(async () => ({ id: "reactivated" })) } };
    const activated = await backend.openSession(root);
    expect(await readdir(path.join(root, LEGACY_RECOVERY_DIR)).catch(() => [])).toEqual([]);
    expect(await readdir(recoveryRootFor(root))).toHaveLength(1);
    expect((await backend.listRecovery(activated.workspace)).find((item) => item.id === record.id)).toBeDefined();
    await backend.stop();
  });

  it("emits persistent recovery records when the workspace is activated again", async () => {
    const { backend, root, messages } = await backendFixture();
    await writeFile(path.join(root, "save.txt"), "original");
    await backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"));
    messages.length = 0;
    const state = backend as unknown as { client: unknown };
    state.client = { session: { create: vi.fn(async () => ({ id: "reactivated" })) } };

    const activated = await backend.openSession(root);

    expect(messages.map((message) => message.kind)).toEqual(["session", "recovery"]);
    const recovery = messages[1].kind === "recovery" ? messages[1].recovery : null;
    expect(recovery?.workspace).toEqual(activated.workspace);
    expect(recovery?.records.some((record) => record.originalPath === "save.txt")).toBe(true);
    await backend.stop();
  });

  it("does not reject when the workspace root is gone before the purge runs", async () => {
    await expect(purgeExpired(path.join(tmpdir(), "vela-missing-root"))).resolves.toBeUndefined();
  });

  it("purges settled recovery transactions older than 24 hours on activation", async () => {
    const { backend, root } = await backendFixture();
    await writeFile(path.join(root, "save.txt"), "original");
    await backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"));
    const recoveryRoot = recoveryRootFor(root);
    await ageTransaction(recoveryRoot, (await readdir(recoveryRoot))[0], { ageMs: 25 * HOUR_MS });
    const state = backend as unknown as { client: unknown };
    state.client = { session: { create: vi.fn(async () => ({ id: "reactivated" })) } };

    await backend.openSession(root);

    await vi.waitFor(async () => {
      expect(await readdir(recoveryRoot).catch(() => [])).toEqual([]);
    });
    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("proposed");
    await backend.stop();
  });

  it("purges failed recovery transactions older than 24 hours", async () => {
    const { backend, root } = await backendFixture(async (phase) => {
      if (phase === "save:source-held") throw new Error("simulated crash");
    });
    await writeFile(path.join(root, "save.txt"), "original");
    await expect(backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"))).rejects.toThrow();
    const recoveryRoot = recoveryRootFor(root);
    await ageTransaction(recoveryRoot, (await readdir(recoveryRoot))[0], { ageMs: 25 * HOUR_MS });

    await purgeExpired(root);

    expect(await readdir(recoveryRoot).catch(() => [])).toEqual([]);
    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("original");
  });

  it("purges acknowledged interrupted transactions after 24 hours", async () => {
    const { backend, root } = await backendFixture(async (phase) => {
      if (phase === "save:source-held") throw new Error("simulated crash");
    });
    await writeFile(path.join(root, "save.txt"), "original");
    await expect(backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"))).rejects.toThrow();
    const recoveryRoot = recoveryRootFor(root);
    await ageTransaction(recoveryRoot, (await readdir(recoveryRoot))[0], {
      ageMs: 25 * HOUR_MS,
      phase: "source-held",
      acknowledged: ["original"]
    });

    await purgeExpired(root);

    expect(await readdir(recoveryRoot).catch(() => [])).toEqual([]);
  });

  it("retains fresh transactions and interrupted transactions younger than 7 days", async () => {
    const { backend, root } = await backendFixture();
    const paths = ["complete.txt", "failed.txt", "source-held.txt", "held-validated.txt"];
    for (const rel of paths) {
      await writeFile(path.join(root, rel), "original");
      await backend.writeFile(workspace, rel, "proposed", writeIdentity("original"));
    }
    const recoveryRoot = recoveryRootFor(root);
    const transactions = await readdir(recoveryRoot);
    await ageTransaction(recoveryRoot, transactions[0], { ageMs: HOUR_MS });
    await ageTransaction(recoveryRoot, transactions[1], { ageMs: HOUR_MS, phase: "failed", acknowledged: [] });
    await ageTransaction(recoveryRoot, transactions[2], { ageMs: 6 * DAY_MS, phase: "source-held", acknowledged: [] });
    await ageTransaction(recoveryRoot, transactions[3], { ageMs: 6 * DAY_MS, phase: "held-validated", acknowledged: [] });

    await purgeExpired(root);

    expect(await readdir(recoveryRoot)).toHaveLength(4);
    expect(await readFile(path.join(root, "complete.txt"), "utf8")).toBe("proposed");
    expect(await recoveryContents(root)).toContain("original");
  });

  it("purges interrupted recovery transactions older than 7 days", async () => {
    const { backend, root } = await backendFixture();
    for (const rel of ["source-held.txt", "held-validated.txt", "fresh.txt"]) {
      await writeFile(path.join(root, rel), "original");
      await backend.writeFile(workspace, rel, "proposed", writeIdentity("original"));
    }
    const recoveryRoot = recoveryRootFor(root);
    const transactions = await readdir(recoveryRoot);
    await ageTransaction(recoveryRoot, transactions[0], { ageMs: 8 * DAY_MS, phase: "source-held", acknowledged: [] });
    await ageTransaction(recoveryRoot, transactions[1], { ageMs: 8 * DAY_MS, phase: "held-validated", acknowledged: [] });
    await ageTransaction(recoveryRoot, transactions[2], { ageMs: HOUR_MS });

    await purgeExpired(root);

    expect(await readdir(recoveryRoot)).toHaveLength(1);
    expect(await readFile(path.join(root, "source-held.txt"), "utf8")).toBe("proposed");
  });

  it("keeps activation working when a retention deletion fails", async () => {
    const { backend, root } = await backendFixture();
    for (const rel of ["blocked.txt", "purged.txt"]) {
      await writeFile(path.join(root, rel), "original");
      await backend.writeFile(workspace, rel, "proposed", writeIdentity("original"));
    }
    const recoveryRoot = recoveryRootFor(root);
    const transactions = await readdir(recoveryRoot);
    const blocked = path.join(recoveryRoot, transactions[0]);
    await ageTransaction(recoveryRoot, transactions[0], { ageMs: 25 * HOUR_MS });
    await ageTransaction(recoveryRoot, transactions[1], { ageMs: 25 * HOUR_MS });
    await chmod(blocked, 0o500);
    const state = backend as unknown as { client: unknown };
    state.client = { session: { create: vi.fn(async () => ({ id: "reactivated" })) } };

    try {
      await backend.openSession(root);
      await vi.waitFor(async () => {
        expect(await readdir(recoveryRoot)).toEqual([transactions[0]]);
      });
    } finally {
      await chmod(blocked, 0o700).catch(() => {});
      await backend.stop();
    }
  });

  it.each([
    "save:source-held",
    "save:held-validated"
  ] as const)("reconciles a missing canonical path after a %s crash boundary", async (crashPhase) => {
    const { backend, root } = await backendFixture(async (phase) => {
      if (phase === crashPhase) throw new Error("simulated crash");
    });
    await writeFile(path.join(root, "save.txt"), "original");
    await expect(backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"))).rejects.toThrow();
    await unlink(path.join(root, "save.txt")).catch(() => {});

    const recoveryRoot = recoveryRootFor(root);
    const transaction = (await readdir(recoveryRoot))[0];
    const manifestPath = path.join(recoveryRoot, transaction, "manifest.json");
    const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as { phase: string };
    manifest.phase = crashPhase === "save:source-held" ? "source-held" : "held-validated";
    await writeFile(manifestPath, JSON.stringify(manifest));
    await new RecoveryStorage().reconcileRecovery(root);

    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("original");
  });

  it("does not replay completed recovery after an intentional delete", async () => {
    const { backend, root } = await backendFixture();
    await writeFile(path.join(root, "save.txt"), "original");
    await backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"));
    await unlink(path.join(root, "save.txt"));

    await new RecoveryStorage().reconcileRecovery(root);

    await expect(readFile(path.join(root, "save.txt"), "utf8")).rejects.toMatchObject({ code: "ENOENT" });
  });

  it("rejects recovery through an intermediate workspace symlink", async () => {
    const { backend, root } = await backendFixture(async (phase) => {
      if (phase === "save:source-held") throw new Error("simulated crash");
    });
    const outside = await mkdtemp(path.join(tmpdir(), "vela-recovery-outside-"));
    roots.push(outside);
    await writeFile(path.join(root, "save.txt"), "original");
    await expect(backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"))).rejects.toThrow();
    const recoveryRoot = recoveryRootFor(root);
    const transaction = (await readdir(recoveryRoot))[0];
    const manifestPath = path.join(recoveryRoot, transaction, "manifest.json");
    const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as { phase: string; originalPath: string };
    manifest.phase = "source-held";
    manifest.originalPath = "escape/payload";
    await writeFile(manifestPath, JSON.stringify(manifest));
    await symlink(outside, path.join(root, "escape"));

    await expect(new RecoveryStorage().reconcileRecovery(root))
      .rejects.toThrow("workspace symlinks are not allowed");
    await expect(readFile(path.join(outside, "payload"), "utf8")).rejects.toMatchObject({ code: "ENOENT" });
  });

  it("stops recovery before mutation when activation is superseded", async () => {
    const { backend, root } = await backendFixture(async (phase) => {
      if (phase === "save:source-held") throw new Error("simulated crash");
    });
    await writeFile(path.join(root, "save.txt"), "original");
    await expect(backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"))).rejects.toThrow();
    await unlink(path.join(root, "save.txt")).catch(() => {});
    const recoveryRoot = recoveryRootFor(root);
    const transaction = (await readdir(recoveryRoot))[0];
    const manifestPath = path.join(recoveryRoot, transaction, "manifest.json");
    const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as { phase: string };
    manifest.phase = "source-held";
    await writeFile(manifestPath, JSON.stringify(manifest));

    await expect(new RecoveryStorage().reconcileRecovery(root, () => false)).rejects.toThrow("activation superseded");
    await expect(readFile(path.join(root, "save.txt"), "utf8")).rejects.toMatchObject({ code: "ENOENT" });
  });

  it("does not overwrite a concurrent canonical target during reconciliation", async () => {
    const { backend, root } = await backendFixture(async (phase) => {
      if (phase === "save:source-held") throw new Error("simulated crash");
    });
    await writeFile(path.join(root, "save.txt"), "original");
    await expect(backend.writeFile(workspace, "save.txt", "proposed", writeIdentity("original"))).rejects.toThrow();
    await unlink(path.join(root, "save.txt"));
    await writeFile(path.join(root, "save.txt"), "concurrent", { flag: "wx" });

    await new RecoveryStorage().reconcileRecovery(root);

    expect(await readFile(path.join(root, "save.txt"), "utf8")).toBe("concurrent");
    expect(await recoveryContents(root)).toContain("original");
  });

  it("captures and emits deletion of a file not previously tracked by the watcher", async () => {
    const { backend, root, messages } = await backendFixture();
    await writeFile(path.join(root, "delete.txt"), "before delete");

    await backend.deletePath(workspace, "delete.txt");

    expect(messages).toEqual([{
      kind: "file-update",
      file: {
        workspace,
        sessionID: "session",
        path: "delete.txt",
        baseline: { kind: "known", content: "before delete" },
        content: null,
        deleted: true
      }
    }]);
  });

  it("emits a restored file with its effective baseline", async () => {
    const { backend, root, messages } = await backendFixture();
    const context = (backend as unknown as { contexts: Map<string, SessionContext> }).contexts.get(workspace.id)!.watchContext;
    context.hasGit = true;
    vi.spyOn(observation(backend), "gitShow")
      .mockResolvedValue("before");
    await writeFile(path.join(root, "restore.txt"), "changed");
    await observation(backend)
      .onFsChanged(context, path.join(root, "restore.txt"), "change");
    await writeFile(path.join(root, "restore.txt"), "before");
    await observation(backend)
      .onFsChanged(context, path.join(root, "restore.txt"), "change");

    expect(messages.map((message) => message.kind === "file-update" ? message.file?.content : null))
      .toEqual(["changed", "before"]);
  });

  it("refreshes a changed file when Git advances its baseline", async () => {
    const { backend, root, messages } = await backendFixture();
    const context = (backend as unknown as { contexts: Map<string, SessionContext> }).contexts.get(workspace.id)!.watchContext;
    context.hasGit = true;
    const gitShow = vi.spyOn(observation(backend), "gitShow")
      .mockResolvedValue("committed");
    const file = path.join(root, "commit.txt");
    context.snapshots.set(file, { kind: "known", content: "old commit" });
    await writeFile(file, "working tree");

    await observation(backend).refreshGitBaselines(context);

    const update = messages.find((message) => message.kind === "file-update");
    expect(update?.kind === "file-update" ? update.file : null).toMatchObject({
      path: "commit.txt",
      baseline: { kind: "known", content: "committed" },
      content: "working tree",
      deleted: false
    });
    expect(gitShow).toHaveBeenCalledWith(context, "commit.txt");
  });

  it("keeps a tracked empty deletion until Git refresh sees it absent", async () => {
    const { backend, root, messages } = await backendFixture();
    const context = (backend as unknown as { contexts: Map<string, SessionContext> }).contexts.get(workspace.id)!.watchContext;
    context.hasGit = true;
    const gitShow = vi.spyOn(observation(backend), "gitShow")
      .mockResolvedValueOnce("")
      .mockResolvedValueOnce(null);
    const file = path.join(root, "empty.txt");
    context.snapshots.set(file, { kind: "known", content: "" });
    await observation(backend).refreshGitBaselines(context);
    expect(messages.at(-1)).toMatchObject({
      kind: "file-update",
      file: { path: "empty.txt", baseline: { kind: "known", content: "" }, content: null, deleted: true }
    });

    await observation(backend).refreshGitBaselines(context);
    expect(messages.at(-1)).toMatchObject({
      kind: "file-update",
      file: { path: "empty.txt", baseline: { kind: "known", content: "", exists: false }, content: null, deleted: true }
    });
    expect(gitShow).toHaveBeenCalledTimes(2);
  });

  it("marks a created-then-deleted file as absent from its baseline", async () => {
    const { backend, messages } = await backendFixture();
    await backend.createFile(workspace, "created.txt");
    await backend.deletePath(workspace, "created.txt");

    expect(messages.map((message) => message.kind === "file-update" ? message.file : null)).toEqual([
      expect.objectContaining({ path: "created.txt", baseline: { kind: "known", content: "", exists: false }, content: "", deleted: false }),
      expect.objectContaining({ path: "created.txt", baseline: { kind: "known", content: "", exists: false }, content: null, deleted: true })
    ]);
  });

  it("emits old and new paths when renaming a previously untracked file", async () => {
    const { backend, root, messages } = await backendFixture();
    await writeFile(path.join(root, "old.txt"), "before rename");

    await backend.renamePath(workspace, "old.txt", "new.txt");

    expect(await readFile(path.join(root, "new.txt"), "utf8")).toBe("before rename");
    expect(messages.map((message) => message.kind === "file-update" ? message.file : null)).toEqual([
      {
        workspace,
        sessionID: "session",
        path: "old.txt",
        baseline: { kind: "known", content: "before rename" },
        content: null,
        deleted: true
      },
      {
        workspace,
        sessionID: "session",
        path: "new.txt",
        baseline: { kind: "known", content: "before rename" },
        content: "before rename",
        deleted: false
      }
    ]);
  });

  it("rejects an occupied file destination and preserves both files", async () => {
    const { backend, root, messages } = await backendFixture();
    await writeFile(path.join(root, "source.txt"), "source");
    await writeFile(path.join(root, "target.txt"), "target");

    await expect(backend.renamePath(workspace, "source.txt", "target.txt"))
      .rejects.toThrow("destination already exists: target.txt");

    expect(await readFile(path.join(root, "source.txt"), "utf8")).toBe("source");
    expect(await readFile(path.join(root, "target.txt"), "utf8")).toBe("target");
    expect(messages.map((message) => message.kind)).toEqual(["recovery"]);
  });

  it("renames a directory without creating file recovery state", async () => {
    const { backend, root, messages } = await backendFixture();
    await mkdir(path.join(root, "source"));
    await writeFile(path.join(root, "source", "value.txt"), "source");

    await backend.renamePath(workspace, "source", "target");

    await expect(readFile(path.join(root, "source", "value.txt"), "utf8")).rejects.toMatchObject({ code: "ENOENT" });
    expect(await readFile(path.join(root, "target", "value.txt"), "utf8")).toBe("source");
    expect(messages).toEqual([]);
  });

  it("rejects an occupied directory rename and preserves both directories", async () => {
    const { backend, root, messages } = await backendFixture();
    await mkdir(path.join(root, "source"));
    await mkdir(path.join(root, "target"));
    await writeFile(path.join(root, "source", "value.txt"), "source");
    await writeFile(path.join(root, "target", "value.txt"), "target");

    await expect(backend.renamePath(workspace, "source", "target"))
      .rejects.toThrow("destination already exists: target");

    expect(await readFile(path.join(root, "source", "value.txt"), "utf8")).toBe("source");
    expect(await readFile(path.join(root, "target", "value.txt"), "utf8")).toBe("target");
    expect(messages).toEqual([]);
  });

  it("preserves a concurrently recreated source and the held artifact when rename rolls back ambiguously", async () => {
    const { backend, root, messages } = await backendFixture(async (phase, _holding, source) => {
      if (phase === "rename:source-held") await writeFile(source, "concurrent source", { flag: "wx" });
    });
    await writeFile(path.join(root, "source.txt"), "source");
    await writeFile(path.join(root, "target.txt"), "occupied");

    await expect(backend.renamePath(workspace, "source.txt", "target.txt"))
      .rejects.toThrow("destination already exists: target.txt");

    expect(await readFile(path.join(root, "source.txt"), "utf8")).toBe("concurrent source");
    expect(await readFile(path.join(root, "target.txt"), "utf8")).toBe("occupied");
    expect(await recoveryContents(root)).toContain("source");
    expect(messages.map((message) => message.kind)).toEqual(["recovery"]);
  });

  it("moves a file across folders and emits delete and add file-updates", async () => {
    const { backend, root, messages } = await backendFixture();
    await mkdir(path.join(root, "source"));
    await mkdir(path.join(root, "target"));
    await writeFile(path.join(root, "source", "note.txt"), "before move");

    await backend.movePath(workspace, "source/note.txt", "target");

    expect(await readFile(path.join(root, "target", "note.txt"), "utf8")).toBe("before move");
    await expect(readFile(path.join(root, "source", "note.txt"), "utf8")).rejects.toMatchObject({ code: "ENOENT" });
    expect(messages.map((message) => (message.kind === "file-update" ? message.file : null))).toEqual([
      {
        workspace,
        sessionID: "session",
        path: "source/note.txt",
        baseline: { kind: "known", content: "before move" },
        content: null,
        deleted: true
      },
      {
        workspace,
        sessionID: "session",
        path: "target/note.txt",
        baseline: { kind: "known", content: "before move" },
        content: "before move",
        deleted: false
      }
    ]);
  });

  it("moves a directory tree into another folder and emits only the source deletion", async () => {
    const { backend, root, messages } = await backendFixture();
    await mkdir(path.join(root, "source", "nested"), { recursive: true });
    await mkdir(path.join(root, "target"));
    await writeFile(path.join(root, "source", "nested", "value.txt"), "payload");

    await backend.movePath(workspace, "source", "target");

    expect(await readFile(path.join(root, "target", "source", "nested", "value.txt"), "utf8")).toBe("payload");
    await expect(readFile(path.join(root, "source", "nested", "value.txt"), "utf8")).rejects.toMatchObject({ code: "ENOENT" });
    expect(messages.map((message) => (message.kind === "file-update" ? message.file : null))).toEqual([
      {
        workspace,
        sessionID: "session",
        path: "source",
        baseline: { kind: "unknown" },
        content: null,
        deleted: true
      }
    ]);
  });

  it("moves an entry to the workspace root", async () => {
    const { backend, root, messages } = await backendFixture();
    await mkdir(path.join(root, "docs"));
    await writeFile(path.join(root, "docs", "guide.txt"), "root");

    await backend.movePath(workspace, "docs/guide.txt", "");

    expect(await readFile(path.join(root, "guide.txt"), "utf8")).toBe("root");
    expect(messages.map((message) => (message.kind === "file-update" ? message.file?.path : null)))
      .toEqual(["docs/guide.txt", "guide.txt"]);
  });

  it("rejects the recovery directory as move source or destination", async () => {
    const { backend, root, messages } = await backendFixture();
    await mkdir(path.join(root, "docs"));
    await mkdir(path.join(root, ".vela-recovery"));
    await writeFile(path.join(root, "docs", "note.txt"), "x");

    await expect(backend.movePath(workspace, "docs", ".vela-recovery/archive"))
      .rejects.toThrow("cannot move into the recovery directory");
    await expect(backend.movePath(workspace, ".vela-recovery", "docs"))
      .rejects.toThrow("cannot move the recovery directory");
    await expect(backend.movePath(workspace, "docs/note.txt", ".vela-recovery"))
      .rejects.toThrow("cannot move into the recovery directory");

    expect(await readFile(path.join(root, "docs", "note.txt"), "utf8")).toBe("x");
    expect(await readdir(path.join(root, ".vela-recovery"))).toEqual([]);
    expect(messages).toEqual([]);
  });

  it("rejects missing, occupied, traversal, and self-descendant move destinations", async () => {
    const { backend, root, messages } = await backendFixture();
    await mkdir(path.join(root, "docs"));
    await mkdir(path.join(root, "target"));
    await mkdir(path.join(root, "docs", "nested"));
    await writeFile(path.join(root, "docs", "note.txt"), "x");
    await writeFile(path.join(root, "target", "note.txt"), "occupied");

    await expect(backend.movePath(workspace, "docs/note.txt", "missing")).rejects.toThrow("destination folder does not exist");
    await expect(backend.movePath(workspace, "docs/note.txt", "target")).rejects.toThrow("destination already exists: note.txt");
    await expect(backend.movePath(workspace, "docs", "docs/nested")).rejects.toThrow("cannot move a folder into itself");
    await expect(backend.movePath(workspace, "docs/note.txt", "docs")).rejects.toThrow("entry is already in that folder");
    await expect(backend.movePath(workspace, "docs", "")).rejects.toThrow("entry is already in that folder");
    await expect(backend.movePath(workspace, "docs/note.txt", "../outside")).rejects.toThrow("invalid workspace path");
    await expect(backend.movePath(workspace, "docs/note.txt", "/absolute")).rejects.toThrow("invalid workspace path");

    expect(await readFile(path.join(root, "docs", "note.txt"), "utf8")).toBe("x");
    expect(messages).toEqual([]);
  });

  it("routes global filesystem events only to their matching active workspace", async () => {
    const { backend, root, messages } = await backendFixture();
    const other = await mkdtemp(path.join(tmpdir(), "vela-other-"));
    roots.push(other);
    await writeFile(path.join(root, "active.txt"), "active");
    await writeFile(path.join(other, "foreign.txt"), "foreign");
    const handle = (backend as unknown as {
      handleServerEvent: (
        type: string,
        data: unknown,
        location?: { directory?: string }
      ) => Promise<void>;
    }).handleServerEvent.bind(backend);

    await handle("filesystem.changed", { file: "foreign.txt", event: "change" }, { directory: other });
    await handle("filesystem.changed", { file: "active.txt", event: "change" }, { directory: root });
    await handle("filesystem.changed", { file: "active.txt", event: "change" });

    expect(messages.map((message) => message.kind === "file-update" ? message.file?.path : null))
      .toEqual(["active.txt"]);
  });
});
