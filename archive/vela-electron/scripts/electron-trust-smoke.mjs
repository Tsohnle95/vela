import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";

if (process.platform !== "darwin") {
  console.log("[electron-trust-smoke] skipped: hidden BrowserWindow coverage runs on macOS CI");
  process.exit(0);
}

const electron = path.join(process.cwd(), "node_modules", ".bin", "electron");
const userData = await mkdtemp(path.join(os.tmpdir(), "vela-trust-"));
const child = spawn(electron, [".", `--user-data-dir=${userData}`], {
  cwd: process.cwd(),
  env: { ...process.env, VELA_TRUST_SMOKE: "1" },
  stdio: ["ignore", "pipe", "pipe"]
});
let output = "";
child.stdout.on("data", (chunk) => { output += chunk; process.stdout.write(chunk); });
child.stderr.on("data", (chunk) => { output += chunk; process.stderr.write(chunk); });
const timeout = setTimeout(() => child.kill(), 30_000);
const code = await new Promise((resolve) => child.once("exit", resolve));
clearTimeout(timeout);
await rm(userData, { recursive: true, force: true });
if (code !== 0 || !output.includes("[vela-trust-smoke]") || !output.includes('"untrustedIpcRejected":true')) {
  throw new Error(`Electron trust smoke failed with exit ${code}`);
}
