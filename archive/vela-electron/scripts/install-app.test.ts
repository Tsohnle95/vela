// @vitest-environment node
import path from "node:path";
import { describe, expect, it } from "vitest";
import { installApp } from "./install-app.mjs";

interface Invocation {
  file: string;
  args: string[];
  options: { cwd?: string; env?: NodeJS.ProcessEnv; maxBuffer?: number };
}

const root = path.resolve("fixture-root");
const builderCli = path.join(root, "node_modules", "electron-builder", "cli.js");
const viteBin = path.join(root, "node_modules", "electron-vite", "bin", "electron-vite.js");
const packagerScript = path.join(root, "scripts", "package-mobile-server.mjs");
const builtApp = path.join(root, "release", "mac-arm64", "Vela.app");
const destination = path.join("/Applications", "Vela.app");

function harness(platform: NodeJS.Platform, existing: Record<string, boolean> = {}) {
  const invocations: Invocation[] = [];
  const removed: string[] = [];
  const launcherOps: string[] = [];
  const timeline: string[] = [];
  const execFileSync = (file: string, args: string[], options: Invocation["options"]) => {
    invocations.push({ file, args: [...args], options });
    timeline.push(file);
    return { status: 0, stdout: "", stderr: "" };
  };
  const existsSync = (p: string) => existing[p] ?? p === path.join(root, "release");
  const rmSync = (p: string, options?: { recursive?: boolean; force?: boolean }) => {
    expect(options?.recursive).toBe(true);
    expect(options?.force).toBe(true);
    removed.push(p);
  };
  const readdirSync = (p: string) => {
    expect(p).toBe(path.join(root, "release"));
    return ["mac-arm64"];
  };
  const liveLauncherIo = {
    rm: (target: string) => {
      launcherOps.push(`rm ${target}`);
      timeline.push(`io rm ${path.relative(root, target)}`);
    },
    mkdir: (target: string) => {
      launcherOps.push(`mkdir ${target}`);
      timeline.push(`io mkdir ${path.relative(root, target)}`);
    },
    copy: (from: string, to: string) => {
      launcherOps.push(`copy ${from} -> ${to}`);
      timeline.push(`io copy ${path.basename(to)}`);
    },
    write: (target: string) => {
      launcherOps.push(`write ${target}`);
      timeline.push(`io write ${path.basename(target)}`);
    }
  };
  return {
    run: (packOnly = false) => installApp({ platform, root, version: "0.1.0", packOnly, execFileSync, existsSync, rmSync, readdirSync, liveLauncherIo }),
    invocations,
    removed,
    launcherOps,
    timeline
  };
}

const appPayload = path.join(builtApp, "Contents", "Resources", "app");

function expectedLauncherOps(): string[] {
  return [
    `rm ${path.join(appPayload, "out")}`,
    `rm ${path.join(appPayload, "node_modules")}`,
    `rm ${path.join(appPayload, "resources")}`,
    `mkdir ${appPayload}`,
    `copy ${path.join(root, "scripts", "live-launcher.cjs")} -> ${path.join(appPayload, "live-launcher.cjs")}`,
    `write ${path.join(appPayload, "package.json")}`,
    `write ${path.join(appPayload, ".vela-repo.json")}`
  ];
}

describe("pack and install app script", () => {
  it("builds, packages, signs, and installs over an existing copy on darwin", () => {
    const { run, invocations, removed, launcherOps } = harness("darwin", { [builderCli]: true, [builtApp]: true, [destination]: true });
    const result = run();

    expect(result).toEqual({ ok: true, message: `Installed Vela to ${destination}` });
    expect(invocations).toHaveLength(5);
    expect(invocations[0].file).toBe(process.execPath);
    expect(invocations[0].args).toEqual([viteBin, "build"]);
    expect(invocations[1].file).toBe(process.execPath);
    expect(invocations[1].args).toEqual([packagerScript]);
    expect(invocations[2].file).toBe(process.execPath);
    expect(invocations[2].args).toEqual([builderCli, "--mac", "dir", "--config", "electron-builder.yml", "--projectDir", root]);
    expect(invocations[3].file).toBe("codesign");
    expect(invocations[3].args).toEqual(["--force", "--deep", "--sign", "-", builtApp]);
    expect(invocations[4].file).toBe("cp");
    expect(invocations[4].args).toEqual(["-R", builtApp, destination]);
    expect(removed).toEqual([destination]);
    expect(launcherOps).toEqual(expectedLauncherOps());
    for (const invocation of invocations.slice(0, 3)) {
      expect(invocation.options.cwd).toBe(root);
      expect(invocation.options.env?.ELECTRON_RUN_AS_NODE).toBe("1");
    }
    expect(invocations[3].options).toBeUndefined();
    expect(invocations[4].options).toBeUndefined();
  });

  it("skips the copy and install when asked to only pack", () => {
    const { run, invocations, removed, launcherOps } = harness("darwin", { [builderCli]: true, [builtApp]: true });
    const result = run(true);

    expect(result).toEqual({ ok: true, message: `Packaged ${path.relative(root, builtApp)}` });
    expect(invocations).toHaveLength(4);
    expect(invocations[3].file).toBe("codesign");
    expect(removed).toEqual([]);
    expect(launcherOps).toEqual(expectedLauncherOps());
  });

  it("swaps in the live launcher before signing so the bundle verifies", () => {
    const { run, timeline } = harness("darwin", { [builderCli]: true, [builtApp]: true });
    run(true);
    const viteBuild = path.join(root, "node_modules", "electron-vite", "bin", "electron-vite.js");
    expect(timeline.indexOf(viteBuild)).toBeLessThan(timeline.indexOf("codesign"));
    expect(timeline.indexOf("io copy live-launcher.cjs")).toBeLessThan(timeline.indexOf("codesign"));
    expect(timeline[timeline.length - 1]).toBe("codesign");
  });

  it("does not remove a missing destination before installing", () => {
    const { run, removed } = harness("darwin", { [builderCli]: true, [builtApp]: true });
    expect(run().ok).toBe(true);
    expect(removed).toEqual([]);
  });

  it("refuses to run on non-darwin platforms", () => {
    for (const platform of ["linux", "win32"] as const) {
      const { run, invocations } = harness(platform);
      expect(run()).toEqual({ ok: false, message: "Install app is macOS-only" });
      expect(invocations).toEqual([]);
    }
  });

  it("reports a missing builder install before running anything", () => {
    const { run, invocations } = harness("darwin");
    expect(run()).toEqual({ ok: false, message: "electron-builder is not installed — run npm install first" });
    expect(invocations).toEqual([]);
  });

  it("reports when packaging produced no bundle and never signs or copies", () => {
    const { run, invocations } = harness("darwin", { [builderCli]: true });
    expect(run()).toEqual({ ok: false, message: "Packaging did not produce a Vela.app bundle under release/" });
    expect(invocations).toHaveLength(3);
  });

  it("propagates build tool failures so the CLI entry reports them", () => {
    const execFileSync = () => {
      throw new Error("builder exploded");
    };
    expect(() => installApp({
      platform: "darwin",
      root,
      version: "0.1.0",
      execFileSync,
      existsSync: () => true,
      rmSync: () => {},
      readdirSync: () => []
    })).toThrow("builder exploded");
  });
});
