import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { REPO_CONFIG_FILE } from "./live-launcher.cjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const APP_NAME = "Vela.app";

function findBuiltApp(projectRoot, exists, readdir) {
  const release = path.join(projectRoot, "release");
  if (!exists(release)) return null;
  for (const entry of readdir(release)) {
    const candidate = path.join(release, entry, APP_NAME);
    if (exists(candidate)) return candidate;
  }
  return null;
}

function projectVersion(projectRoot) {
  const packageJson = JSON.parse(readFileSync(path.join(projectRoot, "package.json"), "utf8"));
  if (typeof packageJson.version !== "string" || !packageJson.version) {
    throw new Error("Vela package.json has no app version");
  }
  return packageJson.version;
}

export function liveLauncherPayload(projectRoot, version = projectVersion(projectRoot)) {
  return {
    packageJson: JSON.stringify({
      name: "vela",
      productName: "Vela",
      version,
      private: true,
      main: "live-launcher.cjs"
    }, null, 2),
    repoConfigJson: `${JSON.stringify({
      projectRoot,
      node: process.execPath
    }, null, 2)}\n`,
    launcherSource: path.join(projectRoot, "scripts", "live-launcher.cjs")
  };
}

export const liveLauncherIo = {
  rm: (target) => rmSync(target, { recursive: true, force: true }),
  mkdir: (target) => mkdirSync(target, { recursive: true }),
  copy: (from, to) => copyFileSync(from, to),
  write: (target, content) => writeFileSync(target, content)
};

export function applyLiveLauncher(bundlePath, projectRoot, io = liveLauncherIo, version = projectVersion(projectRoot)) {
  const appDir = path.join(bundlePath, "Contents", "Resources", "app");
  for (const entry of ["out", "node_modules", "resources"]) {
    io.rm(path.join(appDir, entry));
  }
  io.mkdir(appDir);
  const payload = liveLauncherPayload(projectRoot, version);
  io.copy(payload.launcherSource, path.join(appDir, "live-launcher.cjs"));
  io.write(path.join(appDir, "package.json"), payload.packageJson);
  io.write(path.join(appDir, REPO_CONFIG_FILE), payload.repoConfigJson);
}

export function installApp(options = {}) {
  const projectRoot = options.root ?? root;
  const applicationsDir = options.applicationsDir ?? "/Applications";
  const platform = options.platform ?? process.platform;
  const packOnly = options.packOnly ?? false;
  const version = options.version ?? projectVersion(projectRoot);
  const run = options.execFileSync ?? execFileSync;
  const exists = options.existsSync ?? existsSync;
  const rm = options.rmSync ?? rmSync;
  const readdir = options.readdirSync ?? readdirSync;

  if (platform !== "darwin") {
    return { ok: false, message: "Install app is macOS-only" };
  }

  const builderCli = path.join(projectRoot, "node_modules", "electron-builder", "cli.js");
  if (!exists(builderCli)) {
    return { ok: false, message: "electron-builder is not installed — run npm install first" };
  }

  const runOptions = {
    cwd: projectRoot,
    env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
    maxBuffer: 64 * 1024 * 1024
  };

  run(process.execPath, [path.join(projectRoot, "node_modules", "electron-vite", "bin", "electron-vite.js"), "build"], runOptions);
  run(process.execPath, [path.join(projectRoot, "scripts", "package-mobile-server.mjs")], runOptions);
  run(process.execPath, [builderCli, "--mac", "dir", "--config", "electron-builder.yml", "--projectDir", projectRoot], runOptions);

  const built = findBuiltApp(projectRoot, exists, readdir);
  if (!built) {
    return { ok: false, message: "Packaging did not produce a Vela.app bundle under release/" };
  }

  applyLiveLauncher(built, projectRoot, options.liveLauncherIo, version);
  run("codesign", ["--force", "--deep", "--sign", "-", built]);

  if (packOnly) {
    return { ok: true, message: `Packaged ${path.relative(projectRoot, built)}` };
  }

  const destination = path.join(applicationsDir, APP_NAME);
  if (exists(destination)) rm(destination, { recursive: true, force: true });
  run("cp", ["-R", built, destination]);

  return { ok: true, message: `Installed Vela to ${destination}` };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const result = installApp({ packOnly: process.argv.includes("--pack-only") });
    console.log(JSON.stringify(result));
    process.exit(result.ok ? 0 : 1);
  } catch (error) {
    console.log(JSON.stringify({ ok: false, message: error instanceof Error ? error.message : String(error) }));
    process.exit(1);
  }
}
