// @vitest-environment node
import { describe, expect, it, vi } from "vitest";
import { movePathToTrash } from "./trash";

describe("movePathToTrash", () => {
  it.each(["/workspace/file.txt", "/workspace/folder"])("moves %s to Trash", async (path) => {
    const trashItem = vi.fn(async () => {});

    await movePathToTrash(path, trashItem);

    expect(trashItem).toHaveBeenCalledOnce();
    expect(trashItem).toHaveBeenCalledWith(path);
  });

  it("preserves the original Trash failure without attempting another action", async () => {
    const error = new Error("Trash volume is unavailable");
    const trashItem = vi.fn(async () => { throw error; });

    await expect(movePathToTrash("/workspace/folder", trashItem)).rejects.toBe(error);
    expect(trashItem).toHaveBeenCalledOnce();
  });
});
