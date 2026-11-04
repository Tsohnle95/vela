import { spawn, type IDisposable, type IPty } from "node-pty";
import type { WorkspaceIdentity } from "@shared/types";

interface TerminalSize {
  cols: number;
  rows: number;
}

export interface PtyHandle {
  id: string;
  pty: IPty;
}

export interface TerminalCommand {
  command: string;
  args: string[];
  env?: NodeJS.ProcessEnv;
}

export function defaultShell(platform: NodeJS.Platform, env: NodeJS.ProcessEnv): string {
  if (platform === "darwin") return env.SHELL ?? "/bin/zsh";
  if (platform === "win32") return env.COMSPEC ?? "powershell.exe";
  return env.SHELL ?? "/bin/bash";
}

export class TerminalManager {
  private terminals = new Map<string, {
    pty: IPty;
    workspaceId: string;
    dataSubscription: IDisposable;
    exitSubscription: IDisposable;
  }>();
  private listeners = new Set<(msg: unknown) => void>();

  constructor(private readonly spawnPty: typeof spawn = spawn) {}

  onMessage(cb: (msg: unknown) => void): () => void {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  private emit(msg: unknown): void {
    for (const cb of this.listeners) cb(msg);
  }

  async start(
    id: string,
    directory: string,
    workspace: WorkspaceIdentity,
    command?: TerminalCommand,
    initialSize: TerminalSize = { cols: 100, rows: 24 }
  ): Promise<void> {
    if (this.terminals.has(id)) throw new Error("terminal already exists");
    const executable = command?.command ?? defaultShell(process.platform, process.env);
    const args = command?.args ?? [];
    const pty = this.spawnPty(executable, args, {
      name: "xterm-256color",
      cols: initialSize.cols,
      rows: initialSize.rows,
      cwd: directory,
      env: { ...process.env, ...command?.env, TERM: "xterm-256color", COLORTERM: "truecolor" }
    });
    const terminal = {
      pty,
      workspaceId: workspace.id,
      dataSubscription: pty.onData((data) => {
      this.emit({ kind: "terminal-data", terminal: { id, data } });
      }),
      exitSubscription: pty.onExit(({ exitCode }) => {
      this.terminals.delete(id);
      this.emit({ kind: "terminal-exit", terminal: { id, exitCode } });
      })
    };
    this.terminals.set(id, terminal);
  }

  private get(id: string, workspace: WorkspaceIdentity): IPty {
    const terminal = this.terminals.get(id);
    if (!terminal) throw new Error("unknown terminal");
    if (terminal.workspaceId !== workspace.id) throw new Error("stale terminal");
    return terminal.pty;
  }

  write(id: string, data: string, workspace: WorkspaceIdentity): void {
    this.get(id, workspace).write(data);
  }

  resize(id: string, cols: number, rows: number, workspace: WorkspaceIdentity): void {
    try {
      this.get(id, workspace).resize(cols, rows);
    } catch {
      throw new Error("unknown or unavailable terminal");
    }
  }

  private dispose(id: string): void {
    const terminal = this.terminals.get(id);
    if (!terminal) return;
    this.terminals.delete(id);
    // Disposing the subscriptions detaches node-pty's ThreadSafeFunction
    // callbacks. Queued exit/data callbacks must run before this, or a
    // late exit event fires into a torn-down Node environment and aborts
    // the process on quit (SIGABRT via Napi::Error::ThrowAsJavaScriptException).
    terminal.dataSubscription.dispose();
    terminal.exitSubscription.dispose();
  }

  stop(id: string, workspace?: WorkspaceIdentity): void {
    const terminal = this.terminals.get(id);
    if (!terminal) throw new Error("unknown terminal");
    if (workspace && terminal.workspaceId !== workspace.id) throw new Error("stale terminal");
    try {
      terminal.pty.kill();
    } catch {
      // fall through to disposal
    }
    this.dispose(id);
  }

  async stopAll(shutdownTimeoutMs = 3000): Promise<void> {
    const live = [...this.terminals.entries()];
    if (live.length === 0) return;
    // Reap every child first: an exit event queued after Node teardown
    // begins throws a JS exception into the dying environment and aborts
    // the process. Wait (bounded) for exits to arrive, then detach.
    const exits = await Promise.all(live.map(([id, terminal]) => new Promise<void>((resolve) => {
      if (!this.terminals.has(id)) {
        resolve();
        return;
      }
      const timer = setTimeout(resolve, shutdownTimeoutMs);
      const done = (): void => {
        clearTimeout(timer);
        // The shared handler already deleted the entry; guard keeps a
        // double-fire harmless.
        if (this.terminals.has(id)) this.terminals.delete(id);
        resolve();
      };
      const previous = terminal.exitSubscription;
      terminal.exitSubscription = terminal.pty.onExit(() => {
        try {
          previous.dispose();
        } catch {
          // never block shutdown on disposal
        }
        done();
      });
      try {
        terminal.pty.kill();
      } catch {
        done();
      }
    })));
    void exits;
    for (const [id, terminal] of live) {
      if (!this.terminals.has(id)) continue;
      try {
        terminal.pty.kill();
      } catch {
        // already gone; disposal below still detaches callbacks
      }
      this.dispose(id);
    }
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
