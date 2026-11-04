// @vitest-environment node
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { applyLiveLauncher, liveLauncherPayload } from "./install-app.mjs";
import { MARKER_FILE, REPO_CONFIG_FILE, decideLaunch, resolveRepository, runBuild } from "./live-launcher.cjs";

const repoRoot = path.resolve(path.dirname(decodeURIComponent(new URL(import.meta.url).pathname)), "..");

function makeFixture() {
  const root = mkdtempSync(path.join(os.tmpdir(), "vela-live-"));
  mkdirSync(path.join(root, "src", "main"), { recursive: true });
  writeFileSync(path.join(root, "src", "main", "index.ts"), "export {};\n");
  return root;
}

function writeBuild(root, mtimeMs) {
  mkdirSync(path.join(root, "out", "main"), { recursive: true });
  const entry = path.join(root, "out", "main", "index.js");
  writeFileSync(entry, "// build\n");
  utimesSync(entry, new Date(mtimeMs), new Date(mtimeMs));
}

function writeCompleteBuild(root, output, marker) {
  for (const relative of ["main/index.js", "preload/index.js", "renderer/index.html"]) {
    const target = path.join(root, output, relative);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, marker);
  }
}

describe("live launcher build decision", () => {
  it("builds when no compiled build exists", () => {
    const root = makeFixture();
    try {
      expect(decideLaunch({ projectRoot: root })).toMatchObject({ action: "build" });
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("launches directly when the build is current", () => {
    const root = makeFixture();
    try {
      utimesSync(path.join(root, "src", "main", "index.ts"), new Date(1000), new Date(1000));
      writeBuild(root, 2000);
      expect(decideLaunch({ projectRoot: root })).toEqual({ action: "launch" });
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("builds when repository sources are newer than the build", () => {
    const root = makeFixture();
    try {
      utimesSync(path.join(root, "src", "main", "index.ts"), new Date(3000), new Date(3000));
      writeBuild(root, 2000);
      expect(decideLaunch({ projectRoot: root })).toMatchObject({ action: "build" });
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("ignores dependency and build output directories when scanning sources", () => {
    const root = makeFixture();
    try {
      utimesSync(path.join(root, "src", "main", "index.ts"), new Date(1000), new Date(1000));
      writeBuild(root, 2000);
      mkdirSync(path.join(root, "node_modules", "some-dep"), { recursive: true });
      writeFileSync(path.join(root, "node_modules", "some-dep", "newer.ts"), "export {};\n");
      expect(decideLaunch({ projectRoot: root })).toEqual({ action: "launch" });
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

describe("live launcher staged build", () => {
  it("installs a complete staged output only after it has built", () => {
    const root = makeFixture();
    try {
      mkdirSync(path.join(root, "out", "main"), { recursive: true });
      writeFileSync(path.join(root, "out", "main", "index.js"), "previous build");
      const spawnSync = (_node: string, args: string[]) => {
        const stage = args[args.indexOf("--outDir") + 1];
        writeCompleteBuild(root, stage, "updated build");
        return { status: 0 };
      };

      expect(runBuild({ projectRoot: root, spawnSync })).toBe(true);
      expect(readFileSync(path.join(root, "out", "main", "index.js"), "utf8")).toBe("updated build");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("keeps the previous output when compilation fails", () => {
    const root = makeFixture();
    try {
      mkdirSync(path.join(root, "out", "main"), { recursive: true });
      writeFileSync(path.join(root, "out", "main", "index.js"), "previous build");
      const spawnSync = (_node: string, args: string[]) => {
        const stage = args[args.indexOf("--outDir") + 1];
        mkdirSync(path.join(root, stage, "main"), { recursive: true });
        writeFileSync(path.join(root, stage, "main", "index.js"), "partial build");
        return { status: 1 };
      };

      expect(runBuild({ projectRoot: root, spawnSync })).toBe(false);
      expect(readFileSync(path.join(root, "out", "main", "index.js"), "utf8")).toBe("previous build");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

describe("live launcher payload", () => {
  it("replaces the packaged payload with the repository-backed launcher", () => {
    const fixture = makeFixture();
    const bundle = mkdtempSync(path.join(os.tmpdir(), "vela-bundle-"));
    try {
      const appDir = path.join(bundle, "Contents", "Resources", "app");
      mkdirSync(path.join(appDir, "out", "main"), { recursive: true });
      writeFileSync(path.join(appDir, "out", "main", "index.js"), "// frozen build\n");
      mkdirSync(path.join(appDir, "node_modules", "leftover"), { recursive: true });

      applyLiveLauncher(bundle, repoRoot);

      expect(existsSync(path.join(appDir, "out"))).toBe(false);
      expect(existsSync(path.join(appDir, "node_modules"))).toBe(false);
      const packageJson = JSON.parse(readFileSync(path.join(appDir, "package.json"), "utf8"));
      expect(packageJson.main).toBe("live-launcher.cjs");
      expect(packageJson.version).toBe(JSON.parse(readFileSync(path.join(repoRoot, "package.json"), "utf8")).version);
      expect(existsSync(path.join(appDir, "live-launcher.cjs"))).toBe(true);
      expect(existsSync(path.join(appDir, MARKER_FILE))).toBe(false);
    } finally {
      rmSync(fixture, { recursive: true, force: true });
      rmSync(bundle, { recursive: true, force: true });
    }
  });

  it("describes a package whose entry point is the launcher", () => {
    const payload = liveLauncherPayload(repoRoot);
    const parsed = JSON.parse(payload.packageJson);
    expect(parsed.main).toBe("live-launcher.cjs");
    expect(parsed.version).toBe(JSON.parse(readFileSync(path.join(repoRoot, "package.json"), "utf8")).version);
    expect(payload.launcherSource.endsWith(path.join("scripts", "live-launcher.cjs"))).toBe(true);
    expect(existsSync(payload.launcherSource)).toBe(true);
    const repoConfig = JSON.parse(payload.repoConfigJson);
    expect(repoConfig.projectRoot).toBe(repoRoot);
    expect(typeof repoConfig.node).toBe("string");
  });

  it("prefers the baked repository config over the bundle directory", () => {
    const fixture = makeFixture();
    try {
      writeFileSync(path.join(fixture, REPO_CONFIG_FILE), `${JSON.stringify({ projectRoot: "/somewhere/else", node: "/usr/bin/true" })}\n`);
      const repository = resolveRepository(fixture);
      expect(repository.root).toBe("/somewhere/else");
      expect(repository.node).toBe("/usr/bin/true");
    } finally {
      rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("falls back to the script's own repository when no config is present", () => {
    const fixture = makeFixture();
    try {
      expect(resolveRepository(fixture)).toEqual({ root: fixture });
    } finally {
      rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("ignores a malformed or empty repository config", () => {
    const fixture = makeFixture();
    try {
      writeFileSync(path.join(fixture, REPO_CONFIG_FILE), "{not json");
      expect(resolveRepository(fixture)).toEqual({ root: fixture });
      writeFileSync(path.join(fixture, REPO_CONFIG_FILE), "{}\n");
      expect(resolveRepository(fixture)).toEqual({ root: fixture });
    } finally {
      rmSync(fixture, { recursive: true, force: true });
    }
  });
});
