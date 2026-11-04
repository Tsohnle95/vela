import { Worker } from "node:worker_threads";
import path from "node:path";
import type { SassCompileOptions } from "./sass-config";
import type { SassCompileResult } from "./sass-compile";
export interface SassCompiler {
  compile(source: string, options: SassCompileOptions): Promise<SassCompileResult>;
  close(): Promise<void>;
}
export class SassWorkerCompiler implements SassCompiler {
  private worker = new Worker(path.join(__dirname, "sass-worker.js"));
  private sequence = 0;
  private closed = false;
  private pending = new Map<number, { resolve: (result: SassCompileResult) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> }>();
  constructor() {
    this.worker.on("message", ({ id, result, error }) => {
      const job = this.pending.get(id);
      if (!job) return;
      this.pending.delete(id);
      clearTimeout(job.timer);
      if (error) job.reject(new Error(error));
      else job.resolve(result);
    });
    this.worker.on("error", (error) => this.fail(error));
    this.worker.on("exit", () => this.fail(new Error("Sass compiler stopped")));
  }
  private fail(error: Error): void {
    this.closed = true;
    for (const job of this.pending.values()) { clearTimeout(job.timer); job.reject(error); }
    this.pending.clear();
  }
  compile(source: string, options: SassCompileOptions): Promise<SassCompileResult> {
    if (this.closed) return Promise.reject(new Error("Sass compiler is stopped; toggle Watch Sass off and on to restart"));
    const id = ++this.sequence;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.fail(new Error("Sass compilation exceeded 30 seconds")); void this.worker.terminate(); }, 30_000);
      this.pending.set(id, { resolve, reject, timer });
      this.worker.postMessage({ id, source, options });
    });
  }
  async close(): Promise<void> { this.fail(new Error("Sass watching stopped")); await this.worker.terminate(); }
}
