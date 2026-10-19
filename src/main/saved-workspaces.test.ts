// @vitest-environment node
import { mkdir, mkdtemp, readFile, realpath, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { SavedWorkspaceStore } from "./saved-workspaces";

const roots: string[] = [];

async function fixture(): Promise<{ root: string; store: SavedWorkspaceStore }> {
  const root = await realpath(await mkdtemp(path.join(tmpdir(), "vela-saved-workspaces-")));
  roots.push(root);
  return { root, store: new SavedWorkspaceStore(path.join(root, "settings", "workspaces.json")) };
}

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

describe("SavedWorkspaceStore", () => {
  it("returns an empty list only when neither the primary file nor backup exists", async () => {
    const { store } = await fixture();
    await expect(store.read()).resolves.toEqual([]);
    await expect(store.load()).resolves.toEqual({ workspaces: [], initialized: false });
  });

  it("atomically saves normalized workspaces and reads them after a new store is created", async () => {
    const { root, store } = await fixture();
    const values = [
      { directory: "/projects/vela", name: "Vela" },
      { directory: "/projects/vela", name: "Duplicate" },
      { directory: "/projects/neptune", name: "" },
      { directory: "", name: "Invalid" }
    ];
    const expected = [
      { directory: "/projects/vela", name: "Vela" },
      { directory: "/projects/neptune", name: "neptune" }
    ];

    await expect(store.save(values)).resolves.toEqual(expected);
    await expect(new SavedWorkspaceStore(path.join(root, "settings", "workspaces.json")).read()).resolves.toEqual(expected);
    await expect(store.load()).resolves.toEqual({ workspaces: expected, initialized: true });
    expect(await readFile(path.join(root, "settings", "workspaces.json"), "utf8")).toContain('"version": 2');
  });

  it("recovers from a corrupt primary using the last known good backup", async () => {
    const { root, store } = await fixture();
    const file = path.join(root, "settings", "workspaces.json");
    const expected = [{ directory: "/projects/vela", name: "Vela" }];
    await store.save(expected);
    await store.save([...expected, { directory: "/projects/neptune", name: "Neptune" }]);
    await writeFile(file, "not json");

    await expect(store.read()).resolves.toEqual(expected);
  });

  it("refuses to overwrite when both the primary file and backup are corrupt", async () => {
    const { root, store } = await fixture();
    const file = path.join(root, "settings", "workspaces.json");
    await mkdir(path.dirname(file), { recursive: true });
    await writeFile(file, "broken primary");
    await writeFile(`${file}.bak`, "broken backup");

    await expect(store.read()).rejects.toThrow("Could not read saved workspaces");
    await expect(store.save([])).rejects.toThrow("Could not read saved workspaces");
    await expect(readFile(file, "utf8")).resolves.toBe("broken primary");
  });

  it("serializes overlapping saves so the latest change wins", async () => {
    const { store } = await fixture();
    const first = store.save([{ directory: "/one", name: "One" }]);
    const second = store.save([{ directory: "/two", name: "Two" }]);
    await Promise.all([first, second]);
    await expect(store.read()).resolves.toEqual([{ directory: "/two", name: "Two" }]);
  });
  it("keeps filesystem identity when an equivalent canonical path is relinked", async () => {
    const { root, store } = await fixture();
    const directory = path.join(root, "workspace");
    await mkdir(directory);
    await store.save([{ directory, name: "Workspace" }]);
    await store.relocate(`${directory}/.`, directory);
    const moved = path.join(root, "renamed");
    await rename(directory, moved);
    await expect(store.resolve(directory, [])).resolves.toBe(moved);
  });

  it("recovers a moved non-Git folder by filesystem identity and persists the new location", async () => {
    const { root, store } = await fixture();
    const original = path.join(root, "original");
    const moved = path.join(root, "renamed");
    await mkdir(original);
    await store.save([{ directory: original, name: "My workspace" }]);
    await rename(original, moved);
    const target = await realpath(moved);

    await expect(store.resolve(original, [])).resolves.toBe(target);
    const restarted = new SavedWorkspaceStore(path.join(root, "settings", "workspaces.json"));
    await expect(restarted.read()).resolves.toEqual([{ directory: target, name: "My workspace" }]);
    await expect(restarted.resolve(original, [])).resolves.toBe(target);
    await restarted.save([{ directory: original, name: "Stale browser" }]);
    await expect(restarted.read()).resolves.toEqual([{ directory: target, name: "Stale browser" }]);
  });

  it("recovers legacy project paths using project identity and preserves nested folders", async () => {
    const { root, store } = await fixture();
    const original = path.join(root, "portfolio", "neptune");
    const moved = path.join(root, "neptune");
    const nested = path.join(moved, "docs");
    await mkdir(path.join(root, "portfolio"));
    await mkdir(path.join(moved, ".git"), { recursive: true });
    await mkdir(nested);
    await writeFile(path.join(moved, ".git", "opencode"), "project-neptune");
    await store.save([{ directory: original, name: "Neptune" }, { directory: path.join(original, "docs"), name: "Docs" }]);

    await expect(store.resolve(path.join(original, "docs"), [{ id: "project-neptune", canonical: original }], "project-neptune"))
      .resolves.toBe(await realpath(nested));
    await expect(store.read()).resolves.toEqual([
      { directory: await realpath(moved), name: "Neptune" },
      { directory: await realpath(nested), name: "Docs" }
    ]);
    await store.save([]);
    await expect(new SavedWorkspaceStore(path.join(root, "settings", "workspaces.json")).resolve(original, []))
      .resolves.toBe(await realpath(moved));
  });

  it("does not redirect a missing folder to a same-named different project", async () => {
    const { root, store } = await fixture();
    const original = path.join(root, "portfolio", "neptune");
    await mkdir(path.join(root, "neptune", ".git"), { recursive: true });
    await writeFile(path.join(root, "neptune", ".git", "opencode"), "different-project");
    const bookmarks = [{ directory: original, name: "Neptune" }];
    await store.save(bookmarks);
    await expect(store.resolve(original, [{ id: "original-project", canonical: original }], "original-project")).resolves.toBeNull();
    await expect(store.read()).resolves.toEqual(bookmarks);
  });

  it("does not choose between multiple nearby folders with the same project identity", async () => {
    const { root, store } = await fixture();
    const original = path.join(root, "portfolio", "missing", "neptune");
    await mkdir(path.join(root, "portfolio", "neptune", ".git"), { recursive: true });
    await mkdir(path.join(root, "neptune", ".git"), { recursive: true });
    await writeFile(path.join(root, "portfolio", "neptune", ".git", "opencode"), "project-neptune");
    await writeFile(path.join(root, "neptune", ".git", "opencode"), "project-neptune");
    await expect(store.resolve(original, [{ id: "project-neptune", canonical: original }], "project-neptune")).resolves.toBeNull();
  });

  it("uses the project catalog's new root without dropping a nested session path", async () => {
    const { root, store } = await fixture();
    const original = path.join(root, "portfolio", "neptune");
    const moved = path.join(root, "neptune");
    await mkdir(path.join(moved, "docs"), { recursive: true });
    await expect(store.resolve(path.join(original, "docs"), [{ id: "project-neptune", canonical: moved }], "project-neptune"))
      .resolves.toBe(await realpath(path.join(moved, "docs")));
    await expect(store.resolve(original, [])).resolves.toBe(await realpath(moved));
  });

});
