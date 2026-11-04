#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { promises as fs } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const args = process.argv.slice(2);
let ref;
let output;
let repeats = 2;
let operation = "observation";
for (let index = 0; index < args.length; index += 1) {
  const option = args[index];
  const value = args[++index];
  if (value === undefined) throw new Error(`Missing value for ${option}`);
  if (option === "--ref") ref = value;
  else if (option === "--output") output = value;
  else if (option === "--repeats") repeats = Number(value);
  else if (option === "--operation") operation = value;
  else throw new Error(`Unknown option: ${option}`);
}
if (!ref || !/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(ref)) throw new Error("Use --ref with a valid baseline Git revision");
if (!Number.isSafeInteger(repeats) || repeats < 1 || repeats > 20) throw new Error("--repeats must be between 1 and 20");
if (!["observation", "mutations"].includes(operation)) throw new Error("--operation must be observation or mutations");
const supported = (await fs.readFile(path.join(root, ".node-version"), "utf8")).trim();
if (process.versions.node !== supported) throw new Error(`Run under Node ${supported}; active Node is ${process.versions.node}`);
const baseline = execFileSync("git", ["show", `${ref}:src/main/opencode.ts`], { cwd: root, encoding: "utf8" });
if (operation === "observation" && !/\b(?:private\s+)?async onFsChanged\s*\(/.test(baseline)) throw new Error("Baseline revision must precede the observation extraction and define backend onFsChanged");
if (operation === "mutations" && (!/async writeFile\s*\(/.test(baseline) || /\bworkspaceMutations\b/.test(baseline))) throw new Error("Baseline revision must precede the mutation extraction and define backend writeFile");
const testFile = `src/main/workspace-${operation === "mutations" ? "mutations" : "observation"}.benchmark.test.ts`;
const marker = operation === "mutations" ? "VELA_MUTATION_RESULT " : "VELA_OBSERVATION_RESULT ";
const currentImplementation = operation === "mutations" ? "WorkspaceMutations" : "WorkspaceObservation";
const directory = await fs.mkdtemp(path.join(tmpdir(), "vela-observation-benchmark-"));
try {
  const original = path.join(directory, "baseline-opencode.ts");
  const rebased = baseline.replace(/(\b(?:from|import)\s*)(["'])(\.[^"']+)\2/g, (_match, prefix, _quote, specifier) => {
    return `${prefix}${JSON.stringify(path.resolve(root, "src/main", specifier))}`;
  });
  await fs.writeFile(original, rebased);
  const packageAliases = ["electron", "@opencode/client", "@opencode/client/service"]
    .map((specifier) => `{ find: /^${specifier.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replaceAll("/", "\\/")}$/, replacement: ${JSON.stringify(fileURLToPath(import.meta.resolve(specifier)))} }`)
    .join(",");
  const runs = [];
  for (let repeat = 0; repeat < repeats; repeat += 1) {
    for (const variant of ["current", "baseline"]) {
      const config = path.join(directory, `${variant}.config.mjs`);
      const alias = variant === "baseline" ? `{ find: /^\\.\\/opencode$/, replacement: ${JSON.stringify(original)} }` : "";
      await fs.writeFile(config, `import base from ${JSON.stringify(path.join(root, "vitest.config.ts"))};
export default { ...base, root: ${JSON.stringify(root)}, resolve: { ...base.resolve, alias: [${alias}${alias ? "," : ""} ${packageAliases}, ...base.resolve.alias] } };\n`);
      const stdout = execFileSync(process.execPath, [require.resolve("vitest/vitest.mjs"), "run", testFile, "--config", config], {
        cwd: root,
        env: { ...process.env, VELA_OBSERVATION_BENCHMARK: operation === "observation" ? "1" : "0", VELA_MUTATION_BENCHMARK: operation === "mutations" ? "1" : "0", VELA_BENCHMARK_VARIANT: variant },
        encoding: "utf8",
        maxBuffer: 4 * 1024 * 1024
      });
      const result = stdout.split("\n").find((line) => line.startsWith(marker));
      if (!result) throw new Error(`${variant} benchmark did not emit measurements`);
      const measurement = JSON.parse(result.slice(marker.length));
      const implementation = variant === "baseline" ? "VelaBackend" : currentImplementation;
      if (measurement.variant !== variant || measurement.implementation !== implementation) throw new Error(`Wrong implementation measured for ${variant}`);
      runs.push({ repeat: repeat + 1, ...measurement });
    }
  }
  const expected = runs[0];
  for (const run of runs) {
    for (const key of ["workload", "fixture", "retained", "emissions", ...(operation === "mutations" ? ["recovery"] : [])]) {
      if (JSON.stringify(run[key]) !== JSON.stringify(expected[key])) throw new Error(`Workload or outcome differs for ${run.variant}: ${key}`);
    }
  }
  const digest = (content) => createHash("sha256").update(content).digest("hex");
  const sources = {
    baselineBackendSha256: digest(baseline),
    currentBackendSha256: digest(await fs.readFile(path.join(root, "src/main/opencode.ts"))),
    [operation === "mutations" ? "currentMutationSha256" : "currentObservationSha256"]: digest(await fs.readFile(path.join(root, `src/main/workspace-${operation === "mutations" ? "mutations" : "observation"}.ts`))),
    benchmarkSha256: digest(await fs.readFile(path.join(root, testFile)))
  };
  const timing = operation === "mutations" ? "Samples measure native durable save/rename/create/delete workflows, including recovery sync and guarded emissions, excluding file preparation and one warmup. Trash is modeled by native unlink in both variants." : "Samples measure native stat/read and observation processing, excluding file creation and one warmup. They do not measure OS notification delivery.";
  const report = { operation, node: process.versions.node, baselineRef: ref, sources, currentCommit: execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim(), note: `Current means the working tree. ${timing} Historical backend imports use current dependency modules to isolate the selected extraction. Order is current then baseline for each repeat; cache and system noise can affect results.`, runs };
  const json = `${JSON.stringify(report, null, 2)}\n`;
  if (output) await fs.writeFile(path.resolve(output), json);
  process.stdout.write(json);
} finally {
  await fs.rm(directory, { recursive: true, force: true });
}
