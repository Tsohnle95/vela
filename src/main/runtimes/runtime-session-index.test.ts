import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { RuntimeSessionIndex } from "./runtime-session-index";

const roots: string[] = [];

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

describe("RuntimeSessionIndex", () => {
  it("persists runtime identity across instances", async () => {
    const root = await mkdtemp(path.join(os.tmpdir(), "vela-runtime-index-"));
    roots.push(root);
    const file = path.join(root, "sessions.json");
    const index = new RuntimeSessionIndex(file);
    await index.put({ id: "s1", runtimeID: "deepseek", title: "Work", directory: "/repo", updatedAt: 10 });
    await index.touch("s1", 20);
    await expect(new RuntimeSessionIndex(file).get("s1")).resolves.toEqual({
      id: "s1",
      runtimeID: "deepseek",
      title: "Work",
      directory: "/repo",
      updatedAt: 20
    });
    expect(JSON.parse(await readFile(file, "utf8"))).toHaveLength(1);
  });
});
