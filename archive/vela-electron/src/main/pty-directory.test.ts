import { describe, expect, it } from "vitest";
import { resolvePtyDirectory } from "./pty-directory";

const existing = (...dirs: string[]) => async (directory: string) => dirs.includes(directory);

describe("PTY spawn directory", () => {
  it("prefers the panel's captured workspace root", async () => {
    await expect(resolvePtyDirectory({
      captured: "/work/panel",
      sessionDirectory: "/work/moved",
      isDirectory: existing("/work/panel", "/work/moved")
    })).resolves.toBe("/work/panel");
  });

  it("follows the session when the panel's folder was moved underneath it", async () => {
    await expect(resolvePtyDirectory({
      captured: "/work/old/mobile",
      sessionDirectory: "/work/mobile",
      isDirectory: existing("/work/mobile")
    })).resolves.toBe("/work/mobile");
  });

  it("names the missing folder instead of spawning into it", async () => {
    await expect(resolvePtyDirectory({
      captured: "/work/old/mobile",
      sessionDirectory: null,
      isDirectory: existing()
    })).rejects.toThrow("Workspace folder no longer exists: /work/old/mobile");
  });

  it("rejects when the session resolves to the same missing folder", async () => {
    await expect(resolvePtyDirectory({
      captured: "/work/old/mobile",
      sessionDirectory: "/work/old/mobile",
      isDirectory: existing()
    })).rejects.toThrow("Workspace folder no longer exists: /work/old/mobile");
  });
});
