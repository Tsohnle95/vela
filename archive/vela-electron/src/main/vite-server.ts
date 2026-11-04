import { spawn as nodeSpawn } from "node:child_process";
import { get } from "node:http";
import { createServer } from "node:net";
import { existsSync } from "node:fs";
import { readdir } from "node:fs/promises";
import path from "node:path";
import type { VitePreview, ViteServerInfo } from "@shared/types";

export type { VitePreview, ViteServerInfo };

export interface ViteChild {
  kill: () => void;
  once: (event: string, listener: (code?: number | null, signal?: NodeJS.Signals | null) => void) => void;
  diagnostics: () => string;
}

export interface ViteManagerDeps {
  launch: (directory: string, port: number) => ViteChild;
  findPort: (firstPort: number) => Promise<number>;
  waitReady: (url: string) => Promise<void>;
}

export const VITE_FIRST_PORT = 5199;
const VITE_PORT_ATTEMPTS = 20;
const VITE_READY_TIMEOUT_MS = 15000;
const VITE_READY_POLL_MS = 150;

export function resolveViteCommand(
  appPath: string,
  moduleDirectory: string,
  electronPath = process.execPath,
  platform = process.platform,
  exists: (file: string) => boolean = existsSync
): { command: string; prefix: string[] } {
  const candidates = [
    path.join(appPath, "node_modules", "vite", "bin", "vite.js"),
    path.resolve(moduleDirectory, "../../node_modules/vite/bin/vite.js")
  ];
  const bin = candidates.find(exists);
  if (bin) return { command: electronPath, prefix: [bin] };
  return { command: platform === "win32" ? "npx.cmd" : "npx", prefix: ["vite"] };
}

export function viteHttpError(url: string, status: number): Error | null {
  if (status >= 200 && status < 400) return null;
  const pathname = new URL(url).pathname;
  if (status === 404) {
    return new Error(
      `Vite is running, but no page was found at ${pathname}. Open an HTML file and try again, or add index.html at the workspace root.`
    );
  }
  return new Error(`Vite returned HTTP ${status} while loading ${pathname}`);
}

const HTML_ENTRY_IGNORES = new Set(["node_modules", ".git"]);
const HTML_ENTRY_MAX_DEPTH = 6;

/**
 * Find the shallowest HTML page under a workspace so the preview can serve a
 * directory that has no root index.html. Dependency/VCS/hidden directories are
 * skipped and symlinks are never followed, so the entry stays inside the
 * workspace. `index.html` wins over any other page at the same or shallower
 * depth.
 */
export async function findHtmlEntry(root: string): Promise<string | null> {
  let fallback: string | null = null;
  let level = [root];
  for (let depth = 0; depth <= HTML_ENTRY_MAX_DEPTH && level.length > 0; depth += 1) {
    const directories: string[] = [];
    for (const directory of level) {
      const entries = await readdir(directory, { withFileTypes: true }).catch(() => null);
      if (!entries) continue;
      for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
        if (entry.name.startsWith(".") || HTML_ENTRY_IGNORES.has(entry.name)) continue;
        const absolute = path.join(directory, entry.name);
        if (entry.isDirectory()) {
          directories.push(absolute);
        } else if (entry.isFile() && /\.html?$/i.test(entry.name)) {
          if (entry.name.toLowerCase() === "index.html") return absolute;
          fallback ??= absolute;
        }
      }
    }
    level = directories;
  }
  return fallback;
}

export function probeFreePort(firstPort: number): Promise<number> {
  return new Promise((resolve, reject) => {
    const tryPort = (port: number, attemptsLeft: number): void => {
      if (attemptsLeft <= 0) {
        reject(new Error("no free port for the Vite preview server"));
        return;
      }
      const server = createServer();
      server.once("error", () => {
        server.close();
        tryPort(port + 1, attemptsLeft - 1);
      });
      server.once("listening", () => {
        server.close(() => resolve(port));
      });
      server.listen(port, "127.0.0.1");
    };
    tryPort(firstPort, VITE_PORT_ATTEMPTS);
  });
}

export function pollHttpReady(url: string, timeoutMs = VITE_READY_TIMEOUT_MS): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = (): void => {
      const req = get(url, (res) => {
        res.resume();
        const status = res.statusCode ?? 0;
        const failure = viteHttpError(url, status);
        if (!failure) {
          resolve();
          return;
        }
        reject(failure);
      });
      req.setTimeout(1_000, () => req.destroy(new Error("Vite readiness request timed out")));
      req.once("error", () => {
        if (Date.now() >= deadline) {
          reject(new Error("Vite preview server did not become ready"));
          return;
        }
        setTimeout(attempt, VITE_READY_POLL_MS);
      });
    };
    attempt();
  });
}

export function defaultViteDeps(command: string, prefixArgs: string[], spawnImpl: typeof nodeSpawn = nodeSpawn): ViteManagerDeps {
  return {
    launch: (directory, port) => {
      const child = spawnImpl(
        command,
        [...prefixArgs, "serve", directory, "--port", String(port), "--strictPort", "--host", "127.0.0.1"],
        { cwd: directory, stdio: ["ignore", "ignore", "pipe"], env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" } }
      );
      let stderr = "";
      child.stderr?.setEncoding("utf8");
      child.stderr?.on("data", (chunk: string | Buffer) => {
        stderr = `${stderr}${String(chunk)}`.slice(-4_000);
      });
      return {
        kill: () => { child.kill(); },
        once: (event, listener) => { child.once(event, listener); },
        diagnostics: () => stderr.replace(/\u001b\[[0-9;]*m/g, "").trim()
      };
    },
    findPort: (firstPort) => probeFreePort(firstPort),
    waitReady: (url) => pollHttpReady(url)
  };
}

/**
 * A server is identified by the workspace and the exact page it serves, so one
 * workspace can run several concurrent previews for different HTML entries.
 */
export function viteServerKey(workspaceId: string, directory: string, entry: string): string {
  return JSON.stringify([workspaceId, directory, entry]);
}

interface RunningServer {
  child: ViteChild;
  preview: VitePreview;
  workspaceId: string;
  directory: string;
  entry: string;
}

export class VitePreviewManager {
  private servers = new Map<string, RunningServer>();

  constructor(private readonly deps: ViteManagerDeps, private readonly firstPort = VITE_FIRST_PORT) {}

  running(key: string): VitePreview | null {
    const entry = this.servers.get(key);
    return entry ? entry.preview : null;
  }

  async start(key: string, workspaceId: string, directory: string, entryPath = ""): Promise<VitePreview> {
    const existing = this.servers.get(key);
    if (existing && existing.directory === directory && existing.entry === entryPath) {
      existing.preview = this.preview(existing.preview.port, entryPath);
      try {
        await this.deps.waitReady(existing.preview.url);
      } catch (error) {
        this.stop(key);
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(this.withDiagnostics(message, existing.child));
      }
      return existing.preview;
    }
    if (existing) this.stop(key);
    const port = await this.deps.findPort(this.firstPort);
    const preview = this.preview(port, entryPath);
    const child = this.deps.launch(directory, port);
    let rejectExit: (error: Error) => void = () => undefined;
    const earlyExit = new Promise<never>((_, reject) => {
      rejectExit = reject;
    });
    const entry: RunningServer = { child, preview, workspaceId, directory, entry: entryPath };
    child.once("exit", (code, signal) => {
      if (this.servers.get(key) === entry) this.servers.delete(key);
      const reason = code !== undefined && code !== null ? ` with code ${code}` : signal ? ` from ${signal}` : "";
      rejectExit(new Error(this.withDiagnostics(`Vite preview server exited${reason} before becoming ready`, child)));
    });
    this.servers.set(key, entry);
    try {
      await Promise.race([this.deps.waitReady(preview.url), earlyExit]);
    } catch (error) {
      this.stop(key);
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(this.withDiagnostics(message, child));
    }
    return entry.preview;
  }

  list(): ViteServerInfo[] {
    return [...this.servers.entries()]
      .map(([id, server]) => ({
        id,
        workspaceId: server.workspaceId,
        directory: server.directory,
        entry: server.entry,
        url: server.preview.url,
        port: server.preview.port
      }))
      .sort((a, b) => a.directory.localeCompare(b.directory) || a.entry.localeCompare(b.entry));
  }

  stopWorkspace(workspaceId: string): void {
    for (const [key, server] of [...this.servers.entries()]) {
      if (server.workspaceId === workspaceId) this.stop(key);
    }
  }

  private preview(port: number, entryPath: string): VitePreview {
    const pathname = entryPath
      ? `/${entryPath.split("/").filter(Boolean).map(encodeURIComponent).join("/")}`
      : "/";
    return { url: `http://127.0.0.1:${port}${pathname}`, port };
  }

  private withDiagnostics(message: string, child: ViteChild): string {
    const diagnostics = child.diagnostics();
    return diagnostics && !message.includes(diagnostics) ? `${message}: ${diagnostics}` : message;
  }

  stop(key: string): void {
    const entry = this.servers.get(key);
    if (!entry) return;
    this.servers.delete(key);
    try {
      entry.child.kill();
    } catch {
      return;
    }
  }

  async stopAll(): Promise<void> {
    for (const key of [...this.servers.keys()]) this.stop(key);
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
