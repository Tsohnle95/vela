import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const result = spawnSync(process.execPath, [
  fileURLToPath(import.meta.resolve("vitest/vitest.mjs")),
  "run", "src/main/workspace-observation.test.ts"
], {
  stdio: "inherit",
  env: { ...process.env, VELA_NATIVE_WATCH_TESTS: "1" }
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
