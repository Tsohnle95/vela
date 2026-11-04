// @vitest-environment node
import { mkdir, mkdtemp, readFile, realpath, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

const electronMocks = vi.hoisted(() => ({ showItemInFolder: vi.fn() }));

vi.mock("electron", () => ({
  app: { getPath: () => tmpdir() },
  shell: { trashItem: vi.fn(), openPath: async () => "", showItemInFolder: electronMocks.showItemInFolder }
}));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { VelaBackend, type SessionContext } from "./opencode";
import type { BackendMessage, FileBaseline, WorkspaceIdentity } from "@shared/types";
import { LatestGeneration } from "@shared/generation";

const roots: string[] = [];
const workspace: WorkspaceIdentity = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function fixture(registerRoot = true): Promise<{ backend: VelaBackend; root: string; messages: BackendMessage[] }> {
  const root = await realpath(await mkdtemp(path.join(tmpdir(), "vela-external-")));
  roots.push(root);
  const backend = new VelaBackend();
  const context = {
    root,
    sessionID: "session",
    workspace,
    snapshots: new Map<string, FileBaseline>(),
    lastKnown: new Map<string, string>(),
    hasGit: false,
    timers: new Map()
  };
  const state = backend as unknown as {
    contexts: Map<string, SessionContext>;
    primary: string | null;
    client: unknown;
  };
  state.client = {
    session: { create: vi.fn(async ({ location }: { location: { directory: string } }) => ({ id: `sess-${path.basename(location.directory)}` })) }
  };
  if (registerRoot) {
    state.contexts = new Map([[workspace.id, {
      workspace,
      sessionID: "session",
      directory: root,
      sessionInfo: { id: "session", directory: root, workspace },
      watchContext: context,
      watcher: null,
      activations: new LatestGeneration()
    }]]);
    state.primary = workspace.id;
  }
  const messages: BackendMessage[] = [];
  backend.onMessage((message) => messages.push(message as BackendMessage));
  return { backend, root, messages };
}

describe("resolveExternalOpen", () => {
  it("classifies an in-workspace file as relative", async () => {
    const { backend, root } = await fixture();
    await writeFile(path.join(root, "inside.txt"), "hello");
    const result = await backend.resolveExternalOpen(workspace, path.join(root, "inside.txt"));
    expect(result).toEqual({ kind: "relative", rel: "inside.txt", content: "hello" });
  });

  it("classifies an out-of-workspace file as standalone", async () => {
    const { backend, root } = await fixture();
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-outside-")));
    roots.push(outside);
    await writeFile(path.join(outside, "notes.txt"), "notes");
    const result = await backend.resolveExternalOpen(workspace, path.join(outside, "notes.txt"));
    expect(result).toEqual({ kind: "standalone", path: path.join(outside, "notes.txt"), content: "notes" });
  });

  it("rejects non-file paths", async () => {
    const { backend, root } = await fixture();
    await mkdir(path.join(root, "adir"));
    await expect(backend.resolveExternalOpen(workspace, path.join(root, "adir"))).rejects.toThrow("not a file");
  });

  it("rejects files larger than the workspace cap", async () => {
    const { backend, root } = await fixture();
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-huge-")));
    roots.push(outside);
    const big = path.join(outside, "big.txt");
    await writeFile(big, "x".repeat(9 * 1024 * 1024));
    await expect(backend.resolveExternalOpen(workspace, big)).rejects.toThrow("too large");
  });
});

describe("revealInFileManager", () => {
  it("reveals existing files and folders without changing workspace state", async () => {
    const { backend, root } = await fixture();
    const file = path.join(root, "inside.txt");
    const folder = path.join(root, "folder");
    await writeFile(file, "hello");
    await mkdir(folder);

    await backend.revealInFileManager(workspace, "inside.txt");
    await backend.revealInFileManager(workspace, "folder");

    expect(electronMocks.showItemInFolder).toHaveBeenNthCalledWith(1, file);
    expect(electronMocks.showItemInFolder).toHaveBeenNthCalledWith(2, folder);
  });

  it("rejects missing or outside items before invoking the file manager", async () => {
    electronMocks.showItemInFolder.mockClear();
    const { backend, root } = await fixture();

    await expect(backend.revealInFileManager(workspace, "gone.txt")).rejects.toThrow("no longer available");
    await expect(backend.revealInFileManager(workspace, "../outside.txt")).rejects.toThrow("invalid workspace path");
    expect(electronMocks.showItemInFolder).not.toHaveBeenCalled();
  });
});

describe("writeStandaloneFile", () => {
  it("atomically writes content to the real path", async () => {
    const { backend, root } = await fixture();
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-write-")));
    roots.push(outside);
    const file = path.join(outside, "tracked.txt");
    await writeFile(file, "old");
    await backend.writeStandaloneFile(file, "new", "old", false);
    expect(await readFile(file, "utf8")).toBe("new");
  });

  it("rejects a mismatched expected content unless overwriting", async () => {
    const { backend, root } = await fixture();
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-write-")));
    roots.push(outside);
    const file = path.join(outside, "tracked.txt");
    await writeFile(file, "actual");
    await expect(backend.writeStandaloneFile(file, "new", "expected", false)).rejects.toThrow("file changed on disk");
    expect(await readFile(file, "utf8")).toBe("actual");
    await backend.writeStandaloneFile(file, "new", "expected", true);
    expect(await readFile(file, "utf8")).toBe("new");
  });

  it("recreates a file that was deleted on disk", async () => {
    const { backend, root } = await fixture();
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-write-")));
    roots.push(outside);
    const file = path.join(outside, "tracked.txt");
    await writeFile(file, "old");
    await rm(file);
    await backend.writeStandaloneFile(file, "fresh", "old", false);
    expect(await readFile(file, "utf8")).toBe("fresh");
  });
});

describe("importExternal", () => {
  it("copies an external file into the workspace and seeds clean baselines", async () => {
    const { backend, root, messages } = await fixture();
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-import-")));
    roots.push(outside);
    const source = path.join(outside, "source.txt");
    await writeFile(source, "content");
    const results = await backend.importExternal(workspace, "", [source]);
    expect(results).toEqual([{ name: "source.txt", rel: "source.txt", imported: true }]);
    expect(await readFile(path.join(root, "source.txt"), "utf8")).toBe("content");
    const state = backend as unknown as { contexts: Map<string, SessionContext> };
    const watchContext = state.contexts.get(workspace.id)!.watchContext;
    expect(watchContext.snapshots.get(path.join(root, "source.txt"))).toEqual({ kind: "known", content: "content" });
    expect(watchContext.lastKnown.get(path.join(root, "source.txt"))).toBe("content");
    expect(messages.length).toBe(0);
  });

  it("recursively copies a folder into a destination directory", async () => {
    const { backend, root } = await fixture();
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-import-")));
    roots.push(outside);
    await mkdir(path.join(outside, "pkg"));
    await writeFile(path.join(outside, "pkg", "a.ts"), "aa");
    await writeFile(path.join(outside, "pkg", "b.ts"), "bb");
    await writeFile(path.join(outside, "pkg", "empty.ts"), "");
    const results = await backend.importExternal(workspace, "vendor", [path.join(outside, "pkg")]);
    expect(results).toEqual([{ name: "pkg", rel: "vendor/pkg", imported: true }]);
    expect(await readFile(path.join(root, "vendor/pkg/a.ts"), "utf8")).toBe("aa");
    expect(await readFile(path.join(root, "vendor/pkg/b.ts"), "utf8")).toBe("bb");
    const state = backend as unknown as { contexts: Map<string, SessionContext> };
    const watchContext = state.contexts.get(workspace.id)!.watchContext;
    expect(watchContext.snapshots.get(path.join(root, "vendor/pkg/empty.ts"))).toEqual({ kind: "known", content: "" });
    expect(watchContext.lastKnown.get(path.join(root, "vendor/pkg/empty.ts"))).toBe("");
  });

  it("refuses destinations that already exist and in-workspace sources", async () => {
    const { backend, root } = await fixture();
    await writeFile(path.join(root, "taken.txt"), "x");
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-import-")));
    roots.push(outside);
    await writeFile(path.join(outside, "taken.txt"), "y");
    await writeFile(path.join(root, "in-root.txt"), "z");
    const dup = await backend.importExternal(workspace, "", [path.join(outside, "taken.txt")]);
    expect(dup[0].imported).toBe(false);
    expect(dup[0].reason).toBe("already exists");
    const inRoot = await backend.importExternal(workspace, "", [path.join(root, "in-root.txt")]);
    expect(inRoot[0].imported).toBe(false);
    expect(inRoot[0].reason).toBe("already in the workspace");
  });

  it("imports into a deeper destination folder", async () => {
    const { backend, root } = await fixture();
    await mkdir(path.join(root, "sub"));
    const outside = await realpath(await mkdtemp(path.join(tmpdir(), "vela-import-")));
    roots.push(outside);
    await writeFile(path.join(outside, "deep.txt"), "deep");
    const results = await backend.importExternal(workspace, "sub/nested", [path.join(outside, "deep.txt")]);
    expect(results[0].rel).toBe("sub/nested/deep.txt");
    expect(await readFile(path.join(root, "sub/nested/deep.txt"), "utf8")).toBe("deep");
  });
});

describe("openFileWorkspace", () => {
  it("opens a session on the file's parent folder and returns the relative path", async () => {
    const { backend, root } = await fixture(false);
    const file = path.join(root, "index.html");
    await writeFile(file, "<main></main>");
    const result = await backend.openFileWorkspace(file, 1);
    expect(result.session.directory).toBe(await realpath(root));
    expect(result.path).toBe("index.html");
  });
});

describe("statExternal", () => {
  it("classifies files, directories, and missing paths", async () => {
    const { backend, root } = await fixture();
    await writeFile(path.join(root, "f.txt"), "x");
    await mkdir(path.join(root, "d"));
    expect((await backend.statExternal(path.join(root, "f.txt"))).kind).toBe("file");
    expect((await backend.statExternal(path.join(root, "d"))).kind).toBe("directory");
    expect((await backend.statExternal(path.join(root, "missing"))).kind).toBe("missing");
  });
});

describe("readFile", () => {
  it("returns content for existing files and null for missing ones", async () => {
    const { backend, root } = await fixture();
    await writeFile(path.join(root, "f.txt"), "hello");
    expect(await backend.readFile(workspace, "f.txt")).toBe("hello");
  });

  it("returns null instead of throwing for deleted files", async () => {
    const { backend } = await fixture();
    expect(await backend.readFile(workspace, "gone.txt")).toBeNull();
  });

  it("correlates an externally renamed open file by filesystem identity", async () => {
    const { backend, root, messages } = await fixture();
    const oldPath = path.join(root, "old.txt");
    const newPath = path.join(root, "new.txt");
    await writeFile(oldPath, "hello");
    expect(await backend.readFile(workspace, "old.txt")).toBe("hello");
    await rename(oldPath, newPath);

    const context = (backend as unknown as { contexts: Map<string, SessionContext> })
      .contexts.get(workspace.id)!.watchContext;
    const observer = (backend as unknown as {
      observation: import("./workspace-observation").WorkspaceObservation<SessionContext>;
    }).observation;
    const onFsChanged = observer.onFsChanged.bind(observer);
    await onFsChanged(context, newPath, "rename");
    await onFsChanged(context, oldPath, "rename");

    expect(messages).toEqual([
      expect.objectContaining({
        kind: "file-update",
        file: expect.objectContaining({ path: "old.txt", content: null, deleted: true })
      }),
      expect.objectContaining({
        kind: "file-update",
        file: expect.objectContaining({
          path: "new.txt",
          movedFrom: "old.txt",
          content: "hello",
          deleted: false
        })
      })
    ]);
  });
});
