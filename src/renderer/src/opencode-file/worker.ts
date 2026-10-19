// Adapted from anomalyco/opencode 03e67171, session-ui/src/pierre/worker.ts (MIT; see LICENSE).
import { registerCustomTheme } from "@pierre/diffs";
import { WorkerPoolManager } from "@pierre/diffs/worker";
import ShikiWorkerUrl from "@pierre/diffs/worker/worker.js?worker&url";
import { OpenCodeTheme } from "../opencode-markdown/marked-theme";

registerCustomTheme("OpenCode", () => Promise.resolve(OpenCodeTheme));

let unified: WorkerPoolManager | undefined;

export function getFileWorkerPool(): WorkerPoolManager | undefined {
  if (typeof window === "undefined" || typeof Worker === "undefined") return;
  if (unified) return unified;
  unified = new WorkerPoolManager(
    {
      workerFactory: () => new Worker(ShikiWorkerUrl, { type: "module" }),
      poolSize: 2
    },
    // Vela's renderer CSP disallows Wasm compilation; use Shiki's supported JS engine.
    { theme: "OpenCode", lineDiffType: "none", preferredHighlighter: "shiki-js" }
  );
  void unified.initialize();
  return unified;
}
