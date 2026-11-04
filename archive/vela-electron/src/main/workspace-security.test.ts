// @vitest-environment node
import { mkdtemp, mkdir, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { rm } from "node:fs/promises";
import {
  MAX_TERMINAL_COLUMNS,
  MAX_TERMINAL_INPUT_BYTES,
  MAX_WORKSPACE_FILE_BYTES,
  assertWorkspace,
  canonicalWorkspaceRoot,
  confinedAbsolutePath,
  confinedPath,
  fileContent,
  fileName,
  relativePath,
  terminalDimensions,
  terminalId,
  terminalInput,
  workspaceId
} from "./workspace-security";

const identity = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
const stale = { id: "22222222-2222-4222-8222-222222222222", generation: 2 };
const roots: string[] = [];

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function tempRoot(): Promise<string> {
  const root = await mkdtemp(path.join(tmpdir(), "vela-security-"));
  roots.push(root);
  return root;
}

describe("workspace argument validation", () => {
  it("accepts only the current immutable identity shape", () => {
    expect(workspaceId(identity)).toBe(identity.id);
    expect(assertWorkspace(identity, identity)).toBe(identity);
    expect(() => assertWorkspace(stale, identity)).toThrow("stale workspace");
    expect(() => assertWorkspace({ ...identity, generation: 2 }, identity)).toThrow("stale workspace");
    for (const value of [null, identity.id, {}, { id: "short" }, { ...identity, extra: true }]) {
      expect(() => workspaceId(value)).toThrow("invalid workspace identity");
    }
  });

  it("rejects absolute, traversal, malformed, empty and oversized paths", () => {
    expect(relativePath("src/index.ts")).toBe("src/index.ts");
    expect(relativePath("", true)).toBe("");
    for (const value of ["", "/etc/passwd", "C:/Windows", "../secret", "a/../secret", "a//b", "./a", "a\\b", "a\0b", "x".repeat(4097), 1]) {
      expect(() => relativePath(value)).toThrow("invalid workspace path");
    }
  });

  it("rejects malformed names and oversized content", () => {
    expect(fileName("new.ts")).toBe("new.ts");
    for (const value of ["", ".", "..", "a/b", "a\\b", "a\0b", "x".repeat(256), null]) {
      expect(() => fileName(value)).toThrow("invalid name");
    }
    expect(fileContent("ok")).toBe("ok");
    expect(() => fileContent("x".repeat(MAX_WORKSPACE_FILE_BYTES + 1))).toThrow("oversized");
    expect(() => fileContent(new Uint8Array())).toThrow("invalid");
  });
});

describe("workspace confinement", () => {
  it("canonicalizes a workspace and allows ordinary descendants", async () => {
    const root = await tempRoot();
    await mkdir(path.join(root, "src"));
    await writeFile(path.join(root, "src", "index.ts"), "");
    expect(await canonicalWorkspaceRoot(root)).toBe(await canonicalWorkspaceRoot(`${root}/.`));
    expect(await confinedPath(root, "src/index.ts")).toBe(path.join(root, "src", "index.ts"));
  });

  it("rejects every existing intermediate symlink, including a missing target below it", async () => {
    const root = await tempRoot();
    const outside = await tempRoot();
    await symlink(outside, path.join(root, "linked"));
    await expect(confinedPath(root, "linked/existing.txt")).rejects.toThrow("symlinks are not allowed");
    await expect(confinedPath(root, "linked/new/deep.txt")).rejects.toThrow("symlinks are not allowed");
  });

  it("confines privileged source reads to the canonical application root", async () => {
    const root = await tempRoot();
    const source = path.join(root, "src.css");
    await writeFile(source, "body {}");
    expect(await confinedAbsolutePath(root, source)).toBe(source);
    await expect(confinedAbsolutePath(root, path.join(root, "..", "secret"))).rejects.toThrow("outside");
    await expect(confinedAbsolutePath(root, "relative.css")).rejects.toThrow("invalid source path");
  });
});

describe("terminal argument validation", () => {
  it("accepts bounded terminal IDs, input and dimensions", () => {
    expect(terminalId("term-11111111-1111-4111-8111-111111111111")).toBe("term-11111111-1111-4111-8111-111111111111");
    expect(terminalInput("ls\n")).toBe("ls\n");
    expect(terminalDimensions(120, 40)).toEqual({ cols: 120, rows: 40 });
  });

  it("rejects malformed IDs, oversized input and invalid dimensions", () => {
    for (const value of ["", "term-1", "other-11111111-1111-4111-8111-111111111111", "term-11111111-1111-6111-8111-111111111111", 1]) {
      expect(() => terminalId(value)).toThrow("invalid terminal id");
    }
    expect(() => terminalInput("x".repeat(MAX_TERMINAL_INPUT_BYTES + 1))).toThrow("oversized");
    expect(() => terminalInput(null)).toThrow("invalid");
    for (const [cols, rows] of [[0, 1], [1, 0], [-1, 1], [1.5, 1], [MAX_TERMINAL_COLUMNS + 1, 1], [1, Infinity]]) {
      expect(() => terminalDimensions(cols, rows)).toThrow("invalid terminal dimensions");
    }
  });
});
