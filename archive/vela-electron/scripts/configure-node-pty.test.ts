// @vitest-environment node
import { chmodSync, mkdirSync, mkdtempSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { configureNodePty } from "./configure-node-pty.mjs";

describe("node-pty installation", () => {
  it("makes the macOS spawn helper executable", () => {
    const root = mkdtempSync(path.join(tmpdir(), "vela-node-pty-"));
    const directory = path.join(root, "node_modules", "node-pty", "prebuilds", "darwin-x64");
    const helper = path.join(directory, "spawn-helper");
    mkdirSync(directory, { recursive: true });
    writeFileSync(helper, "fixture");
    chmodSync(helper, 0o644);
    expect(configureNodePty("darwin", "x64", root)).toBe(helper);
    expect(statSync(helper).mode & 0o111).toBe(0o111);
  });

  it("supports a helper built from source", () => {
    const root = mkdtempSync(path.join(tmpdir(), "vela-node-pty-build-"));
    const directory = path.join(root, "node_modules", "node-pty", "build", "Release");
    const helper = path.join(directory, "spawn-helper");
    mkdirSync(directory, { recursive: true });
    writeFileSync(helper, "fixture");
    chmodSync(helper, 0o644);
    expect(configureNodePty("darwin", "arm64", root)).toBe(helper);
    expect(statSync(helper).mode & 0o111).toBe(0o111);
  });

  it("does not configure a helper on Linux", () => {
    expect(configureNodePty("linux", "x64", "unused")).toBeNull();
  });

  it("does not configure a helper on Windows", () => {
    expect(configureNodePty("win32", "x64", "unused")).toBeNull();
  });
});
