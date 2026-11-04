import { parentPort } from "node:worker_threads";
import { compileSass } from "./sass-compile";
import type { SassCompileOptions } from "./sass-config";
parentPort?.on("message", ({ id, source, options }: { id: number; source: string; options: SassCompileOptions }) => {
  try { parentPort!.postMessage({ id, result: compileSass(source, options) }); }
  catch (error) { parentPort!.postMessage({ id, error: String(error) }); }
});
