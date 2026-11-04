#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = fileURLToPath(new URL("../", import.meta.url));
const args = process.argv.slice(2);
if (args.length !== 0 && (args.length !== 2 || args[0] !== "--ref")) {
  throw new Error("Usage: node scripts/benchmark-chat-projection.mjs [--ref <git-commit>]");
}
const supportedNode = readFileSync(path.join(root, ".node-version"), "utf8").trim();
if (process.versions.node !== supportedNode) {
  throw new Error(`Run this benchmark with Node ${supportedNode}; active version is ${process.versions.node}`);
}
const git = (...gitArgs) => execFileSync("git", gitArgs, { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }).trim();
const ref = args[1] ? git("rev-parse", "--verify", `${args[1]}^{commit}`) : null;
const gitFiles = ref ? new Set(git("ls-tree", "-r", "--name-only", ref, "src").split("\n")) : null;
const plugins = ref ? [{
  name: "git-source-snapshot",
  setup(builder) {
    builder.onResolve({ filter: /^(?:\.\.?\/|@shared\/)/ }, (entry) => {
      const directory = entry.namespace === "git-source"
        ? path.posix.dirname(entry.importer)
        : path.relative(root, entry.resolveDir).split(path.sep).join("/");
      const modulePath = entry.path.startsWith("@shared/")
        ? `src/shared/${entry.path.slice("@shared/".length)}`
        : path.posix.normalize(path.posix.join(directory, entry.path));
      if (!modulePath.startsWith("src/")) return;
      const resolved = [modulePath, `${modulePath}.ts`, `${modulePath}.tsx`, `${modulePath}.js`, `${modulePath}/index.ts`]
        .find((candidate) => gitFiles.has(candidate));
      if (!resolved) throw new Error(`Module ${modulePath} does not exist at ${ref}`);
      return { path: resolved, namespace: "git-source" };
    });
    builder.onLoad({ filter: /.*/, namespace: "git-source" }, (entry) => ({
      contents: git("show", `${ref}:${entry.path}`),
      loader: entry.path.endsWith(".tsx") ? "tsx" : entry.path.endsWith(".json") ? "json" : "ts"
    }));
  }
}] : [];
const result = await build({
  stdin: {
    contents: 'export { applyChatEvent, projectAssistantItems } from "./src/renderer/src/chat-store.ts"; export { largeSessionEvents, retainedOutputChars } from "./src/renderer/src/large-session-fixture.ts";',
    resolveDir: root,
    sourcefile: "benchmark-chat-projection.ts"
  },
  bundle: true, platform: "node", format: "esm", write: false,
  alias: { "@shared": path.join(root, "src/shared") },
  plugins
});
const { applyChatEvent, projectAssistantItems, largeSessionEvents, retainedOutputChars } =
  await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString("base64")}`);
const historicalAssistants = 400;
const deltas = 500;
const tailID = "zz-active-assistant";
const delta = " next token";
const event = (id, type, data) => ({
  id, type, created: 2_000, data: { sessionID: "session", assistantMessageID: tailID, ...data }
});

function sample() {
  const state = { message: {}, part: {}, session_status: {} };
  for (const item of largeSessionEvents(historicalAssistants)) applyChatEvent(state, "session", item);
  applyChatEvent(state, "session", event("tail-step", "session.step.started", {}));
  applyChatEvent(state, "session", event("tail-text", "session.text.started", { ordinal: 0 }));
  let previous = projectAssistantItems(state, "session");
  let reducerMs = 0;
  let projectionMs = 0;
  let historicalIdentitiesReused = 0;
  let tailIdentityChanges = 0;
  const start = performance.now();
  for (let index = 0; index < deltas; index++) {
    const update = event(`delta-${index}`, "session.text.delta", { ordinal: 0, delta });
    let at = performance.now();
    applyChatEvent(state, "session", update);
    reducerMs += performance.now() - at;
    at = performance.now();
    const next = projectAssistantItems(state, "session");
    projectionMs += performance.now() - at;
    for (let item = 0; item < historicalAssistants; item++) {
      if (next[item] === previous[item]) historicalIdentitiesReused++;
    }
    if (next.at(-1) !== previous.at(-1)) tailIdentityChanges++;
    previous = next;
  }
  const elapsedMs = performance.now() - start;
  const tail = previous.at(-1);
  if (tail.id !== tailID || tail.parts[0].text !== delta.repeat(deltas)) throw new Error("Incorrect final streamed text");
  const outputChars = retainedOutputChars(previous);
  if (previous.length !== historicalAssistants + 1 || outputChars !== historicalAssistants * 32 * 1024) {
    throw new Error("Historical output was lost during the benchmark");
  }
  return { reducerMs, projectionMs, elapsedMs, historicalIdentitiesReused, tailIdentityChanges, outputChars };
}

for (let warmup = 0; warmup < 3; warmup++) sample();
const samples = Array.from({ length: 9 }, sample);
function distribution(field) {
  const sorted = samples.map((item) => item[field]).sort((a, b) => a - b);
  return {
    min: Number(sorted[0].toFixed(3)),
    median: Number(sorted[4].toFixed(3)),
    max: Number(sorted[8].toFixed(3))
  };
}
console.log(JSON.stringify({
  source: ref ?? "working tree", head: git("rev-parse", "HEAD"), node: process.version,
  workload: { historicalAssistants, historicalEvents: historicalAssistants * 6, deltas, deltaChars: delta.length, toolOutputChars: 32 * 1024, warmups: 3, samples: 9 },
  scope: "Reducer and assistant projection; setup excluded; excludes Store, React, IPC, Chromium layout and paint",
  reducerMs: distribution("reducerMs"), projectionMs: distribution("projectionMs"), elapsedMs: distribution("elapsedMs"),
  samples
}, null, 2));
