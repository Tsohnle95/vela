import { describe, expect, it } from "vitest";
import type { AgentFileState } from "@shared/types";
import { buildChangeTreeRows, changeDiffStats, changeList, isHiddenChangePath } from "./changes-view-model";

function state(overrides: Partial<AgentFileState> = {}): AgentFileState {
  return { baseline: { kind: "known", content: "old\n" }, content: "new\n", deleted: false, ...overrides };
}

describe("isHiddenChangePath", () => {
  it("matches the path itself and anything beneath it", () => {
    const hidden = new Set(["src/vendor", "docs/old.md"]);
    expect(isHiddenChangePath("src/vendor", hidden)).toBe(true);
    expect(isHiddenChangePath("src/vendor/deep/file.ts", hidden)).toBe(true);
    expect(isHiddenChangePath("docs/old.md", hidden)).toBe(true);
    expect(isHiddenChangePath("src/other.ts", hidden)).toBe(false);
    // A similarly-prefixed sibling is not hidden.
    expect(isHiddenChangePath("src/vendor-2/file.ts", hidden)).toBe(false);
    expect(isHiddenChangePath("docs/old.md.bak", hidden)).toBe(false);
  });

  it("is a no-op for an empty hidden set", () => {
    expect(isHiddenChangePath("anything.ts", new Set())).toBe(false);
  });
});

describe("changeList", () => {
  it("sorts by path and splits directory from name", () => {
    const files = changeList(new Map([
      ["src/beta.ts", state()],
      ["readme.md", state()],
      ["src/alpha.ts", state()]
    ]));
    expect(files.map((file) => file.path)).toEqual(["readme.md", "src/alpha.ts", "src/beta.ts"]);
    expect(files[1]).toMatchObject({ name: "alpha.ts", directory: "src" });
    expect(files[0]).toMatchObject({ name: "readme.md", directory: "" });
  });
});

describe("changeDiffStats", () => {
  it("counts added and removed lines for a modification", () => {
    expect(changeDiffStats(state({ baseline: { kind: "known", content: "a\nb\n" }, content: "a\nc\n" })))
      .toEqual({ additions: 1, deletions: 1 });
  });

  it("counts every line as removed for a deletion", () => {
    expect(changeDiffStats(state({ baseline: { kind: "known", content: "a\nb\n" }, content: null, deleted: true })))
      .toEqual({ additions: 0, deletions: 2 });
  });

  it("returns null without a known baseline", () => {
    expect(changeDiffStats(state({ baseline: { kind: "unknown" } }))).toBeNull();
  });
});

describe("buildChangeTreeRows", () => {
  const files = changeList(new Map([
    ["src/a.ts", state()],
    ["src/nested/b.ts", state()],
    ["readme.md", state()]
  ]));

  it("lists directories before files and nests by depth", () => {
    const rows = buildChangeTreeRows(files, new Set());
    expect(rows.map((row) => `${row.kind[0]}:${row.label}@${row.depth}`)).toEqual([
      "d:src@0",
      "d:nested@1",
      "f:b.ts@2",
      "f:a.ts@1",
      "f:readme.md@0"
    ]);
  });

  it("hides the subtree of a collapsed directory", () => {
    const rows = buildChangeTreeRows(files, new Set(["src"]));
    expect(rows.map((row) => row.label)).toEqual(["src", "readme.md"]);
  });

  it("collapses single-child directory chains", () => {
    const deep = changeList(new Map([["a/b/c/d.ts", state()]]));
    const rows = buildChangeTreeRows(deep, new Set());
    expect(rows).toEqual([
      { kind: "directory", key: "dir:a/b/c", depth: 0, label: "a/b/c", path: "a/b/c" },
      { kind: "file", key: "file:a/b/c/d.ts", depth: 1, label: "d.ts", path: "a/b/c/d.ts" }
    ]);
  });

  it("does not collapse a directory that also holds files", () => {
    const mixed = changeList(new Map([["p/x.ts", state()], ["p/q/y.ts", state()]]));
    const rows = buildChangeTreeRows(mixed, new Set());
    expect(rows.map((row) => row.label)).toEqual(["p", "q", "y.ts", "x.ts"]);
  });
});
