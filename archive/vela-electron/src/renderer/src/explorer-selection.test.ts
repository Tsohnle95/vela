import { describe, expect, it } from "vitest";
import { compactExplorerSelection, movableExplorerPaths } from "./explorer-selection";

describe("explorer selection operations", () => {
  it("removes duplicates and nested selections while retaining selection order", () => {
    expect(compactExplorerSelection(["alpha/child.txt", "beta", "alpha", "beta", ""]))
      .toEqual(["beta", "alpha"]);
  });

  it("filters no-op, self, and descendant moves from a multi-path drop", () => {
    expect(movableExplorerPaths(["alpha", "beta", "gamma/item.txt"], "alpha/sub"))
      .toEqual(["beta", "gamma/item.txt"]);
    expect(movableExplorerPaths(["alpha/item.txt", "beta"], "alpha"))
      .toEqual(["beta"]);
  });
});
