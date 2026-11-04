import { describe, expect, it } from "vitest";
import { knownBaseline, observedBaseline, preserveBaseline, unknownBaseline } from "./file-baseline";

describe("observed file baseline lifecycle", () => {
  it("keeps the pre-tool baseline through later tool and editor saves", () => {
    const tool = preserveBaseline(undefined, knownBaseline("before tool"));
    const secondTool = preserveBaseline(tool, knownBaseline("after first tool"));
    const editorSave = preserveBaseline(secondTool, knownBaseline("before editor save"));

    expect(editorSave).toEqual(knownBaseline("before tool"));
  });

  it("uses an empty known baseline for a file created by a structured tool or editor", () => {
    expect(preserveBaseline(undefined, knownBaseline("", false))).toEqual({ kind: "known", content: "", exists: false });
  });

  it("retains the established baseline when the file is deleted", () => {
    const beforeDelete = preserveBaseline(undefined, knownBaseline("original"));
    expect(preserveBaseline(beforeDelete, unknownBaseline)).toBe(beforeDelete);
  });

  it("carries the first baseline when a tracked file is renamed", () => {
    const oldPath = preserveBaseline(undefined, knownBaseline("original"));
    const newPath = preserveBaseline(undefined, oldPath);
    expect(newPath).toBe(oldPath);
  });

  it("uses Git HEAD for a first-observed shell change in a Git workspace", () => {
    expect(observedBaseline(true, "head content")).toEqual(knownBaseline("head content"));
  });

  it("treats a Git-untracked creation as an empty pre-session file", () => {
    expect(observedBaseline(true, null)).toEqual({ kind: "known", content: "", exists: false });
  });

  it("keeps an empty tracked Git file as existing", () => {
    expect(observedBaseline(true, "")).toEqual(knownBaseline(""));
  });

  it("marks a first-observed shell change unknown in a non-Git workspace", () => {
    expect(observedBaseline(false, null)).toEqual(unknownBaseline);
  });
});
