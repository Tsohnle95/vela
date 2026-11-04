// @vitest-environment node
import path from "node:path";
import { describe, expect, it } from "vitest";
import { launch } from "./launch.mjs";

interface Invocation {
  command: string;
  args: string[];
  options: { env?: NodeJS.ProcessEnv };
}

function capture(platform: NodeJS.Platform): Invocation[] {
  const invocations: Invocation[] = [];
  const root = path.resolve("fixture-root");
  const spawnSync = (command: string, args: readonly string[], options: Invocation["options"]) => {
    invocations.push({ command, args: [...args], options });
    return { status: 0, signal: null, pid: 1, output: [], stdout: null, stderr: null };
  };
  expect(launch("dev", { platform, root, spawnSync })).toBe(0);
  return invocations;
}

describe("portable Electron launcher", () => {
  it("prepares and selects the custom app bundle only on Darwin", () => {
    const invocations = capture("darwin");
    expect(invocations).toHaveLength(2);
    expect(invocations[0].args[0].endsWith(path.join("scripts", "make-dev-app.mjs"))).toBe(true);
    expect(invocations[1].options.env?.ELECTRON_EXEC_PATH?.endsWith(
      path.join("dev", "Vela.app", "Contents", "MacOS", "Electron")
    )).toBe(true);
  });

  it.each(["linux", "win32"] as const)("uses plain Electron on %s", (platform) => {
    const invocations = capture(platform);
    expect(invocations).toHaveLength(1);
    expect(invocations[0].args.slice(-2)).toEqual([path.resolve("fixture-root", "node_modules", "electron-vite", "bin", "electron-vite.js"), "dev"]);
    expect(invocations[0].options.env?.ELECTRON_EXEC_PATH).toBeUndefined();
  });
});
