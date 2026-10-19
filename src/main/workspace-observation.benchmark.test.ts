// @vitest-environment node
import { promises as fsp } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";
import { expect, it, vi } from "vitest";
import { LatestGeneration } from "@shared/generation";
import { VelaBackend, type SessionContext } from "./opencode";

vi.mock("electron", () => ({ app: { getPath: () => tmpdir() }, shell: {} }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

it.skipIf(process.env.VELA_OBSERVATION_BENCHMARK !== "1")("measures identical native observation workloads", async () => {
  const root = await fsp.realpath(await fsp.mkdtemp(path.join(tmpdir(), "vela-observation-bench-")));
  const backend = new VelaBackend();
  const workspace = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
  const context: SessionContext["watchContext"] = {
    root, workspace, sessionID: "benchmark", snapshots: new Map(), lastKnown: new Map(), hasGit: false, timers: new Map()
  };
  const state = backend as unknown as {
    contexts: Map<string, SessionContext>;
    observation?: import("./workspace-observation").WorkspaceObservation<SessionContext>;
    onFsChanged?(context: SessionContext["watchContext"], abs: string, event: string): Promise<void>;
  };
  state.contexts.set(workspace.id, {
    workspace, directory: root, sessionID: context.sessionID,
    sessionInfo: { id: context.sessionID, directory: root, workspace },
    watchContext: context, watcher: null, activations: new LatestGeneration()
  });
  const observe = state.observation
    ? state.observation.onFsChanged.bind(state.observation)
    : state.onFsChanged?.bind(state);
  if (!observe) throw new Error("benchmark backend has no observation entry point");
  let emissions = 0;
  backend.onMessage(() => { emissions += 1; });
  const files = Array.from({ length: 20 }, (_, index) => path.join(root, `${index}.txt`));
  const content = "representative text\n".repeat(200);
  try {
    await Promise.all(files.map((file) => fsp.writeFile(file, content)));
    const samples: number[] = [];
    for (let sample = 0; sample < 9; sample += 1) {
      const start = performance.now();
      for (let event = 0; event < 200; event += 1) {
        await observe(context, files[event % files.length], "change");
      }
      if (sample > 0) samples.push(performance.now() - start);
    }
    expect(emissions).toBe(20);
    expect(context.snapshots.size).toBe(20);
    console.log(`VELA_OBSERVATION_RESULT ${JSON.stringify({
      variant: process.env.VELA_BENCHMARK_VARIANT ?? "current",
      implementation: state.observation ? "WorkspaceObservation" : "VelaBackend",
      workload: { files: files.length, eventsPerSample: 200, warmups: 1, samples: 8 },
      fixture: { bytesPerFile: Buffer.byteLength(content), sha256: createHash("sha256").update(content).digest("hex") },
      retained: { snapshots: context.snapshots.size, lastKnown: context.lastKnown.size, identities: context.identities?.size ?? 0, timers: context.timers.size },
      emissions,
      samplesMs: samples
    })}`);
  } finally {
    await backend.stop();
    await fsp.rm(root, { recursive: true, force: true });
  }
});
