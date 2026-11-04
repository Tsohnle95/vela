import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { defaultViteDeps, findHtmlEntry, resolveViteCommand, viteHttpError, viteServerKey, VitePreviewManager, type ViteChild, type ViteManagerDeps } from "./vite-server";

interface FakeChild extends ViteChild {
  killed: boolean;
  fireExit: () => void;
}

function fakeChild(): FakeChild {
  const listeners = new Map<string, Array<() => void>>();
  const child: FakeChild = {
    killed: false,
    fireExit: () => {
      for (const listener of listeners.get("exit") ?? []) listener();
    },
    kill: () => {
      child.killed = true;
    },
    diagnostics: () => "",
    once: (event, listener) => {
      const list = listeners.get(event) ?? [];
      list.push(listener);
      listeners.set(event, list);
    }
  };
  return child;
}

function setup(deps?: Partial<ViteManagerDeps>): { manager: VitePreviewManager; deps: ViteManagerDeps; children: FakeChild[] } {
  const children: FakeChild[] = [];
  const full: ViteManagerDeps = {
    launch: vi.fn((_directory: string, _port: number) => {
      const child = fakeChild();
      children.push(child);
      return child;
    }),
    findPort: vi.fn(async (first: number) => first),
    waitReady: vi.fn(async () => undefined),
    ...deps
  };
  return { manager: new VitePreviewManager(full), deps: full, children };
}

describe("VitePreviewManager", () => {
  it("starts a loopback server and reuses it for the same workspace directory", async () => {
    const { manager, deps, children } = setup();
    const first = await manager.start("k1", "ws-1", "/repo/a");
    expect(first).toEqual({ url: "http://127.0.0.1:5199/", port: 5199 });
    expect(deps.launch).toHaveBeenCalledTimes(1);
    expect(deps.launch).toHaveBeenCalledWith("/repo/a", 5199);

    const second = await manager.start("k1", "ws-1", "/repo/a");
    expect(second).toEqual(first);
    expect(deps.launch).toHaveBeenCalledTimes(1);
    expect(children).toHaveLength(1);
  });

  it("serves the active HTML file and safely encodes its URL", async () => {
    const { manager, deps } = setup();
    const preview = await manager.start("k1", "ws-1", "/repo/a", "pages/My demo.html");

    expect(preview).toEqual({ url: "http://127.0.0.1:5199/pages/My%20demo.html", port: 5199 });
    expect(deps.waitReady).toHaveBeenCalledWith(preview.url);
  });

  it("restarts the server when the same key changes directory", async () => {
    const { manager, deps, children } = setup();
    await manager.start("k1", "ws-1", "/repo/a");
    await manager.start("k1", "ws-1", "/repo/b");
    expect(deps.launch).toHaveBeenCalledTimes(2);
    expect(children[0].killed).toBe(true);
    expect(children[1].killed).toBe(false);
  });

  it("runs a separate server per page target in one workspace", async () => {
    const { manager, deps, children } = setup();
    await manager.start("k-docs", "ws-1", "/repo/docs", "");
    await manager.start("k-a", "ws-1", "/repo/a", "a.html");
    await manager.start("k-b", "ws-1", "/repo/b", "b.html");
    expect(deps.launch).toHaveBeenCalledTimes(3);
    expect(manager.list().map((server) => server.entry)).toEqual(["a.html", "b.html", ""]);
    const ids = manager.list().map((server) => server.id);
    expect(ids).toEqual(["k-a", "k-b", "k-docs"]);
    expect(manager.list()[0]).toMatchObject({ workspaceId: "ws-1", port: 5199 });
    expect(children.filter((child) => child.killed)).toHaveLength(0);
  });

  it("stopWorkspace only stops that workspace's servers", async () => {
    const { manager, children } = setup();
    await manager.start("k1", "ws-1", "/repo/a");
    await manager.start("k2", "ws-1", "/repo/b");
    await manager.start("k3", "ws-2", "/repo/c");
    manager.stopWorkspace("ws-1");
    expect(children.map((child) => child.killed)).toEqual([true, true, false]);
    expect(manager.list().map((server) => server.id)).toEqual(["k3"]);
  });

  it("stop kills the server and unknown keys are a no-op", async () => {
    const { manager, deps, children } = setup();
    manager.stop("missing");
    expect(deps.launch).not.toHaveBeenCalled();
    await manager.start("k1", "ws-1", "/repo/a");
    manager.stop("k1");
    expect(children[0].killed).toBe(true);
    expect(manager.running("k1")).toBeNull();
  });

  it("a failed readiness check stops the server and throws", async () => {
    const { manager, children } = setup({
      waitReady: vi.fn(async () => {
        throw new Error("never ready");
      })
    });
    await expect(manager.start("k1", "ws-1", "/repo/a")).rejects.toThrow("never ready");
    expect(children[0].killed).toBe(true);
    expect(manager.running("k1")).toBeNull();
  });

  it("an early server exit cleans up and throws", async () => {
    let child!: FakeChild;
    const { manager } = setup({
      launch: vi.fn(() => {
        child = fakeChild();
        return child;
      }),
      waitReady: vi.fn(() => new Promise<void>(() => undefined))
    });
    const pending = manager.start("k1", "ws-1", "/repo/a");
    await Promise.resolve();
    child.fireExit();
    await expect(pending).rejects.toThrow("exited before becoming ready");
    expect(manager.running("k1")).toBeNull();
  });

  it("includes Vite stderr when startup fails", async () => {
    const child = fakeChild();
    child.diagnostics = () => "failed to load config: missing-plugin";
    const { manager } = setup({
      launch: vi.fn(() => child),
      waitReady: vi.fn(() => new Promise<void>(() => undefined))
    });
    const pending = manager.start("k1", "ws-1", "/repo/a");
    await Promise.resolve();
    child.fireExit();

    await expect(pending).rejects.toThrow("missing-plugin");
  });

  it("stopAll kills every running server", async () => {
    const { manager, children } = setup();
    await manager.start("k1", "ws-1", "/repo/a");
    await manager.start("k2", "ws-2", "/repo/b");
    await manager.stopAll();
    expect(children.map((child) => child.killed)).toEqual([true, true]);
    expect(manager.running("k1")).toBeNull();
    expect(manager.running("k2")).toBeNull();
  });
});

describe("viteServerKey", () => {
  it("distinguishes workspace, directory, and entry", () => {
    expect(viteServerKey("ws", "/repo", "")).not.toBe(viteServerKey("ws", "/repo", "a.html"));
    expect(viteServerKey("ws", "/repo", "a.html")).not.toBe(viteServerKey("ws", "/other", "a.html"));
    expect(viteServerKey("ws", "/repo", "a.html")).toBe(viteServerKey("ws", "/repo", "a.html"));
  });
});

describe("viteHttpError", () => {
  it("accepts successful pages and rejects a false-ready 404 with guidance", () => {
    expect(viteHttpError("http://127.0.0.1:5199/demo.html", 200)).toBeNull();
    expect(viteHttpError("http://127.0.0.1:5199/", 404)?.message)
      .toContain("Open an HTML file and try again");
  });
});

describe("findHtmlEntry", () => {
  const roots: string[] = [];

  afterEach(async () => {
    await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
  });

  async function tree(files: Record<string, string>): Promise<string> {
    const root = await mkdtemp(path.join(tmpdir(), "vela-vite-"));
    roots.push(root);
    for (const [relative, content] of Object.entries(files)) {
      const absolute = path.join(root, relative);
      await mkdir(path.dirname(absolute), { recursive: true });
      await writeFile(absolute, content);
    }
    return root;
  }

  it("returns null when the workspace has no page", async () => {
    const root = await tree({ "readme.md": "# hi", "src/main.ts": "export {}" });
    expect(await findHtmlEntry(root)).toBeNull();
  });

  it("prefers a root index.html", async () => {
    const root = await tree({ "index.html": "<html></html>", "docs/index.html": "<html></html>" });
    expect(await findHtmlEntry(root)).toBe(path.join(root, "index.html"));
  });

  it("finds a nested index.html and skips dependency directories", async () => {
    const root = await tree({
      "node_modules/pkg/index.html": "<html></html>",
      "docs/index.html": "<html></html>"
    });
    expect(await findHtmlEntry(root)).toBe(path.join(root, "docs", "index.html"));
  });

  it("prefers any index.html over a shallower non-index page", async () => {
    const root = await tree({ "about.html": "<html></html>", "docs/index.html": "<html></html>" });
    expect(await findHtmlEntry(root)).toBe(path.join(root, "docs", "index.html"));
  });

  it("falls back to the shallowest HTML file when no index exists", async () => {
    const root = await tree({ "site/about.html": "<html></html>", "deep/nested/page.html": "<html></html>" });
    expect(await findHtmlEntry(root)).toBe(path.join(root, "site", "about.html"));
  });
});

describe("defaultViteDeps", () => {
  it("launches Vite as plain node without a second app instance", () => {
    const seen: Array<{ args: string[]; options: { cwd?: unknown; env?: NodeJS.ProcessEnv; stdio?: unknown } }> = [];
    const spawnImpl = ((...call: [string, string[], { cwd?: unknown; env?: NodeJS.ProcessEnv; stdio?: unknown }]) => {
      seen.push({ args: call[1], options: call[2] });
      return { ...fakeChild(), stderr: null };
    }) as unknown as Parameters<typeof defaultViteDeps>[2];
    const deps = defaultViteDeps("/repo/electron", ["/repo/node_modules/vite/bin/vite.js"], spawnImpl);
    deps.launch("/repo/a", 5199);
    expect(seen).toHaveLength(1);
    expect(seen[0].args).toEqual([
      "/repo/node_modules/vite/bin/vite.js",
      "serve",
      "/repo/a",
      "--port",
      "5199",
      "--strictPort",
      "--host",
      "127.0.0.1"
    ]);
    expect(seen[0].options.cwd).toBe("/repo/a");
    expect(seen[0].options.env?.ELECTRON_RUN_AS_NODE).toBe("1");
    expect(seen[0].options.stdio).toEqual(["ignore", "ignore", "pipe"]);
  });
});

describe("resolveViteCommand", () => {
  it("finds Vite beside the repository build used by the installed live launcher", () => {
    const repositoryBin = "/repo/node_modules/vite/bin/vite.js";
    expect(resolveViteCommand("/Applications/Vela.app/Contents/Resources/app", "/repo/out/main", "/electron", "darwin", (file) => file === repositoryBin))
      .toEqual({ command: "/electron", prefix: [repositoryBin] });
  });

  it("prefers a Vite bundled beside the packaged app and falls back to npx", () => {
    const packagedBin = "/app/node_modules/vite/bin/vite.js";
    expect(resolveViteCommand("/app", "/app/out/main", "/electron", "darwin", (file) => file === packagedBin))
      .toEqual({ command: "/electron", prefix: [packagedBin] });
    expect(resolveViteCommand("/app", "/app/out/main", "/electron", "win32", () => false))
      .toEqual({ command: "npx.cmd", prefix: ["vite"] });
  });
});
