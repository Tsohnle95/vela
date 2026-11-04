const { spawnSync } = require("node:child_process");
const { randomUUID } = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const MARKER_FILE = ".vela-live-build-failed";
const REPO_CONFIG_FILE = ".vela-repo.json";
const SOURCE_EXTENSIONS = /\.(?:ts|tsx|js|mjs|cjs|scss|sass|css|html|json)$/;
const REQUIRED_BUILD_FILES = ["main/index.js", "preload/index.js", "renderer/index.html"];
const SKIPPED_DIRECTORIES = new Set([
  "node_modules",
  "out",
  "dev",
  "release",
  "dist",
  "build",
  "coverage"
]);

function safeStat(target) {
  try {
    return fs.statSync(target);
  } catch {
    return null;
  }
}

function newestSourceMtime(projectRoot) {
  let newest = 0;
  const consider = (target) => {
    const stats = safeStat(target);
    if (stats?.isFile() && SOURCE_EXTENSIONS.test(target)) newest = Math.max(newest, stats.mtimeMs);
  };
  const walk = (directory) => {
    let entries;
    try {
      entries = fs.readdirSync(directory, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (entry.name.startsWith(".") || SKIPPED_DIRECTORIES.has(entry.name)) continue;
      const full = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else {
        consider(full);
      }
    }
  };
  walk(path.join(projectRoot, "src"));
  consider(path.join(projectRoot, "electron.vite.config.ts"));
  return newest;
}

function decideLaunch(options = {}) {
  const projectRoot = options.projectRoot;
  const sourceNewest = options.sourceNewest ?? newestSourceMtime(projectRoot);
  const outStats = options.outStats ?? safeStat(path.join(projectRoot, "out", "main", "index.js"));
  if (!outStats) return { action: "build", reason: "no compiled build found in the repository" };
  if (sourceNewest > outStats.mtimeMs) return { action: "build", reason: "repository sources changed since the last build" };
  return { action: "launch" };
}

function resolveRepository(appRoot = path.resolve(__dirname)) {
  try {
    const config = JSON.parse(fs.readFileSync(path.join(appRoot, REPO_CONFIG_FILE), "utf8"));
    if (typeof config.projectRoot === "string") {
      return { root: config.projectRoot, node: typeof config.node === "string" ? config.node : undefined };
    }
  } catch {
    // development layout: this script lives in the repository itself
  }
  return { root: appRoot };
}

function runBuild(options = {}) {
  const projectRoot = options.projectRoot;
  const spawn = options.spawnSync ?? spawnSync;
  const stageName = `.vela-launcher-stage-${randomUUID()}`;
  const stagePath = path.join(projectRoot, stageName);
  let result;
  try {
    result = spawn(
      options.node ?? process.execPath,
      [path.join(projectRoot, "node_modules", "electron-vite", "bin", "electron-vite.js"), "build", "--outDir", stageName],
      {
        cwd: projectRoot,
        stdio: "inherit",
        env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" }
      }
    );
  } catch (error) {
    fs.rmSync(stagePath, { recursive: true, force: true });
    console.error(`[live-launcher] could not run build: ${error.message}`);
    return false;
  }
  if (result.status !== 0 || !REQUIRED_BUILD_FILES.every((file) => {
    const details = safeStat(path.join(stagePath, file));
    return details?.isFile() && details.size > 0;
  })) {
    fs.rmSync(stagePath, { recursive: true, force: true });
    return false;
  }

  const outputPath = path.join(projectRoot, "out");
  const backupPath = path.join(projectRoot, `.vela-launcher-backup-${randomUUID()}`);
  let movedOutput = false;
  try {
    try {
      fs.renameSync(outputPath, backupPath);
      movedOutput = true;
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    fs.renameSync(stagePath, outputPath);
  } catch (error) {
    if (movedOutput) {
      try {
        fs.renameSync(backupPath, outputPath);
      } catch (restoreError) {
        console.error(`[live-launcher] could not restore previous build: ${restoreError.message}`);
      }
    }
    fs.rmSync(stagePath, { recursive: true, force: true });
    console.error(`[live-launcher] could not install staged build: ${error.message}`);
    return false;
  }
  if (movedOutput) {
    try {
      fs.rmSync(backupPath, { recursive: true, force: true });
    } catch (error) {
      console.error(`[live-launcher] could not remove previous build backup: ${error.message}`);
    }
  }
  return true;
}

function warn(message, buttons, options = {}) {
  if (options.dialog) {
    try {
      const choice = options.dialog.showMessageBoxSync({
        type: "warning",
        title: "Vela",
        message,
        buttons,
        defaultId: 0,
        cancelId: buttons.length - 1
      });
      return choice;
    } catch {
      // fall through to console
    }
  }
  console.log(`[live-launcher] ${message}`);
  return 0;
}

async function main(options = {}) {
  const repository = resolveRepository();
  const projectRoot = repository.root;
  const dialog = options.dialog ?? (() => {
    try {
      return require("electron").dialog;
    } catch {
      return null;
    }
  })();
  const marker = path.join(projectRoot, MARKER_FILE);
  const decision = decideLaunch({ projectRoot });

  if (decision.action === "launch") {
    fs.rmSync(marker, { force: true });
    console.log("[live-launcher] repository build is current");
  } else {
    const previousFailure = fs.existsSync(marker);
    console.log(`[live-launcher] ${decision.reason}; rebuilding …`);
    const ok = runBuild({ projectRoot, node: repository.node });
    if (ok) {
      fs.rmSync(marker, { force: true });
      console.log("[live-launcher] build complete");
    } else {
      fs.writeFileSync(marker, "");
      const message = previousFailure
        ? `The automatic Vela build failed again (${decision.reason}). Start the last known good build instead?`
        : `The automatic Vela build failed (${decision.reason}). Start the last known good build instead? Fix the build or delete ${MARKER_FILE} in the repository to stop seeing this prompt.`;
      const choice = warn(message, ["Use last working build", "Quit"], { dialog });
      if (choice === 1) return 1;
    }
  }

  const entryPoint = path.join(projectRoot, "out", "main", "index.js");
  if (!fs.existsSync(entryPoint)) {
    warn("No Vela build exists yet and the automatic build failed.", ["Quit"], { dialog });
    return 1;
  }
  require(entryPoint);
  return 0;
}

if (require.main === module) {
  main().then((code) => {
    if (code !== 0) process.exitCode = code;
  });
}

module.exports = { MARKER_FILE, REPO_CONFIG_FILE, newestSourceMtime, decideLaunch, resolveRepository, runBuild, main };
