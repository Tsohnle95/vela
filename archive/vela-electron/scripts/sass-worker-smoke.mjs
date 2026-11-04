import { Worker } from "node:worker_threads";
import { mkdtemp, writeFile, readFile, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import assert from "node:assert/strict";

const root = await mkdtemp(path.join(os.tmpdir(), "vela-sass-worker-"));
const worker = new Worker(path.resolve("out/main/sass-worker.js"));
try {
  await writeFile(path.join(root, "_tone.scss"), "$color: red;");
  const source = path.join(root, "main.scss");
  await writeFile(source, "@use './tone'; body { color: tone.$color; }");
  const result = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Compiled Sass worker did not respond")), 10_000);
    worker.once("error", (error) => { clearTimeout(timer); reject(error); });
    worker.once("message", (message) => { clearTimeout(timer); message.error ? reject(new Error(message.error)) : resolve(message.result); });
    worker.postMessage({ id: 1, source, options: { style: "expanded", sourceMap: false, loadPaths: [root] } });
  });
  await writeFile(path.join(root, "main.css"), result.css);
  assert.match(await readFile(path.join(root, "main.css"), "utf8"), /color: red/);
  console.log("[sass-worker-smoke] bundled compiler and built worker generated CSS successfully");
} finally { await worker.terminate(); await rm(root, { recursive: true, force: true }); }
