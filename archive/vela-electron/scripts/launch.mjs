import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export function launch(mode, options = {}) {
  const platform = options.platform ?? process.platform;
  const run = options.spawnSync ?? spawnSync;
  const projectRoot = options.root ?? root;
  const env = { ...process.env };
  delete env.ELECTRON_EXEC_PATH;

  if (platform === "darwin") {
    const prepare = run(process.execPath, [path.join(projectRoot, "scripts", "make-dev-app.mjs")], {
      cwd: projectRoot,
      stdio: "inherit"
    });
    if (prepare.error) throw prepare.error;
    if (prepare.status !== 0) return prepare.status ?? 1;
    env.ELECTRON_EXEC_PATH = path.join(projectRoot, "dev", "Vela.app", "Contents", "MacOS", "Electron");
  }

  const result = run(process.execPath, [path.join(projectRoot, "node_modules", "electron-vite", "bin", "electron-vite.js"), mode], {
    cwd: projectRoot,
    env,
    stdio: "inherit"
  });
  if (result.error) throw result.error;
  return result.status ?? 1;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const mode = process.argv[2];
  if (mode !== "dev" && mode !== "preview") {
    console.error("usage: node scripts/launch.mjs <dev|preview>");
    process.exitCode = 1;
  } else {
    process.exitCode = launch(mode);
  }
}
