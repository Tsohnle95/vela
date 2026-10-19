import { afterEach, describe, expect, it, vi } from "vitest";
import { droppedFilePaths } from "./drop";

function dragEventWith(files: unknown[]): { dataTransfer: { files: unknown[] } } {
  return { dataTransfer: { files } };
}

describe("droppedFilePaths", () => {
  const original = window.vela;

  afterEach(() => {
    (window as { vela?: unknown }).vela = original;
  });

  it("reads paths through the preload bridge when it is available", () => {
    (window as { vela?: unknown }).vela = {
      getPathForFile: (file: File) => `/bridged/${(file as { name?: string }).name}`
    } as never;
    const e = dragEventWith([{ name: "a.txt" } as unknown as File]);
    expect(droppedFilePaths(e as never)).toEqual(["/bridged/a.txt"]);
  });

  it("falls back to the legacy File.path property when no bridge exists", () => {
    (window as { vela?: unknown }).vela = undefined;
    const e = dragEventWith([{ path: "/legacy/a.txt" } as unknown as File]);
    expect(droppedFilePaths(e as never)).toEqual(["/legacy/a.txt"]);
  });

  it("skips files whose path cannot be resolved", () => {
    (window as { vela?: unknown }).vela = undefined;
    const e = dragEventWith([{ name: "no-path.txt" } as unknown as File]);
    expect(droppedFilePaths(e as never)).toEqual([]);
  });

  it("reads Finder URI-list drops when no File objects are exposed", () => {
    const e = {
      dataTransfer: {
        files: [],
        getData: (type: string) => type === "text/uri-list" ? "file:///Users/ty/advanced%20web-concepts\n" : ""
      }
    };
    expect(droppedFilePaths(e as never)).toEqual(["/Users/ty/advanced web-concepts"]);
  });
});
