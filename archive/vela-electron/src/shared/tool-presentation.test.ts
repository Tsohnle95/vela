import { describe, expect, it } from "vitest";
import { collectFilePaths, record, toolContentViews, toolDetailText } from "./tool-presentation";

describe("tool presentation normalization", () => {
  it("collects ordered unique lexical paths without treating URLs as workspace paths", () => {
    expect(collectFilePaths({
      filePath: "src/a.ts",
      nested: [{ file_path: "src/b.ts" }, { path: "src/a.ts" }, { path: "https://example.com" }],
      other: { path: "src/c.ts" },
      url: "src/ignored.ts"
    })).toEqual(["src/a.ts", "src/b.ts", "src/c.ts"]);
    expect(collectFilePaths(null)).toEqual([]);
  });

  it("retains detail precedence and command prefixes", () => {
    expect(toolDetailText({ filePath: "a", file_path: "b", command: "pwd" })).toBe("a");
    expect(toolDetailText({ file_path: "b", command: "pwd" })).toBe("b");
    expect(toolDetailText({ command: "pwd" })).toBe("$ pwd");
    expect(toolDetailText({ path: "a" })).toBe("");
    expect(toolDetailText(null)).toBe("");
  });

  it("preserves content order, coercion and file metadata while rejecting unsupported blocks", () => {
    expect(toolContentViews([
      null, { type: "text", text: 42 }, { type: "image", uri: "ignored" },
      { type: "file", uri: "file:///a", mime: "text/plain", name: "a" },
      { type: "file", uri: "file:///b", mime: "text/plain", name: 1 },
      { type: "file", uri: "file:///c" }, { type: "text" }
    ])).toEqual([
      { type: "text", text: "42" },
      { type: "file", uri: "file:///a", mime: "text/plain", name: "a" },
      { type: "file", uri: "file:///b", mime: "text/plain" },
      { type: "text", text: "" }
    ]);
    expect(toolContentViews([])).toBeUndefined();
    expect(toolContentViews("text")).toBeUndefined();
  });

  it("preserves record identity and rejects arrays/primitives", () => {
    const metadata = { nested: { value: 1 } };
    expect(record(metadata)).toBe(metadata);
    for (const value of [null, undefined, [], "text", 0]) expect(record(value)).toBeUndefined();
  });
});
