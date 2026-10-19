// @vitest-environment node
import { createHash } from "node:crypto";
import { promises as fsp } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { expect, it, vi } from "vitest";
import { LatestGeneration } from "@shared/generation";
import type { BackendMessage } from "@shared/types";
import { VelaBackend, type SessionContext } from "./opencode";
import type { RecoveryStorage } from "./recovery-storage";

vi.mock("electron", () => ({ app: { getPath: () => tmpdir() }, shell: { trashItem: async (target: string) => fsp.rm(target) } }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

it.skipIf(process.env.VELA_MUTATION_BENCHMARK !== "1")("measures identical durable native mutation workflows", async () => {
  const root = await fsp.realpath(await fsp.mkdtemp(path.join(tmpdir(), "vela-mutation-benchmark-")));
  const rootKey = createHash("sha256").update(root).digest("hex").slice(0, 16);
  const recoveryRoot = path.join(path.dirname(root), `.vela-recovery-${rootKey}`);
  const phases: Record<string, number> = {};
  const backend = new VelaBackend((phase) => { phases[phase] = (phases[phase] ?? 0) + 1; });
  const workspace = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
  const context: SessionContext["watchContext"] = {
    root, workspace, sessionID: "benchmark", snapshots: new Map(), lastKnown: new Map(), hasGit: false, timers: new Map()
  };
  const state = backend as unknown as { contexts: Map<string, SessionContext>; recovery: RecoveryStorage; workspaceMutations?: unknown };
  state.contexts.set(workspace.id, {
    workspace, directory: root, sessionID: context.sessionID,
    sessionInfo: { id: context.sessionID, directory: root, workspace },
    watchContext: context, watcher: null, activations: new LatestGeneration()
  });
  const emissions: Record<string, number> = {};
  backend.onMessage((message) => {
    const kind = (message as BackendMessage).kind;
    emissions[kind] = (emissions[kind] ?? 0) + 1;
  });
  const before = "representative text\n".repeat(200);
  const after = "changed file bytes!\n".repeat(200);
  const digest = (content: string) => createHash("sha256").update(content).digest("hex");
  const samples: number[] = [];
  try {
    for (let sample = 0; sample < 9; sample += 1) {
      const sources = Array.from({ length: 3 }, (_, round) => `${sample}-${round}.txt`);
      await Promise.all(sources.map((source) => fsp.writeFile(path.join(root, source), before)));
      const start = performance.now();
      for (const source of sources) {
        const target = `renamed-${source}`;
        const created = `created-${source}`;
        await backend.writeFile(workspace, source, after, { id: source, workspaceID: workspace.id, revision: 1, expectedContent: before, overwrite: false });
        await backend.renamePath(workspace, source, target);
        await backend.createFile(workspace, created);
        await backend.deletePath(workspace, target);
        await backend.deletePath(workspace, created);
      }
      if (sample > 0) samples.push(performance.now() - start);
    }
    const transactions = await state.recovery.readRecoveryTransactions(root);
    const manifests: Record<string, number> = {};
    const artifactHashes: string[] = [];
    let artifactBytes = 0;
    for (const item of transactions) {
      const key = `${item.transaction.operation}:${item.transaction.phase}:${item.transaction.reason}`;
      manifests[key] = (manifests[key] ?? 0) + 1;
      for (const name of await fsp.readdir(item.directory)) {
        if (name === "manifest.json") continue;
        const bytes = await fsp.readFile(path.join(item.directory, name));
        artifactBytes += bytes.byteLength;
        artifactHashes.push(createHash("sha256").update(bytes).digest("hex"));
      }
    }
    const baselineHashes = [...context.snapshots.values()].map((baseline) => baseline.kind === "known" ? `${baseline.exists !== false}:${digest(baseline.content)}` : "unknown").sort();
    expect(emissions).toEqual({ "file-update": 162, recovery: 27 });
    expect(phases["save:temporary-ready"]).toBe(27);
    expect(manifests).toEqual({ "save:complete:saved": 27, "rename:complete:renamed": 27 });
    expect(await fsp.readdir(root)).toEqual([]);
    console.log(`VELA_MUTATION_RESULT ${JSON.stringify({
      variant: process.env.VELA_BENCHMARK_VARIANT ?? "current",
      implementation: state.workspaceMutations ? "WorkspaceMutations" : "VelaBackend",
      workload: { roundsPerSample: 3, operationsPerRound: ["save", "rename", "create", "delete-renamed", "delete-created"], warmups: 1, samples: 8 },
      fixture: { beforeBytes: Buffer.byteLength(before), beforeSha256: digest(before), afterBytes: Buffer.byteLength(after), afterSha256: digest(after) },
      retained: { snapshots: context.snapshots.size, lastKnown: context.lastKnown.size, identities: context.identities?.size ?? 0, timers: context.timers.size, baselineSha256: digest(JSON.stringify(baselineHashes)) },
      recovery: { manifests, phases, artifactBytes, artifactCount: artifactHashes.length, artifactsSha256: digest(JSON.stringify(artifactHashes.sort())) },
      emissions,
      samplesMs: samples
    })}`);
  } finally {
    await backend.stop();
    await Promise.all([root, recoveryRoot].map((directory) => fsp.rm(directory, { recursive: true, force: true })));
  }
}, 30000);
