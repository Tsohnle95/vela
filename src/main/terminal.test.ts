// @vitest-environment node
import { describe, expect, it, vi } from "vitest";
import { defaultShell, TerminalManager, type TerminalCommand } from "./terminal";
import type { IPty } from "node-pty";

const workspace = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };

describe("TerminalManager capability ownership", () => {
  it("rejects unknown terminal operations", () => {
    const terminals = new TerminalManager();
    expect(() => terminals.write("term-1", "x", workspace)).toThrow("unknown terminal");
    expect(() => terminals.resize("term-1", 80, 24, workspace)).toThrow("unknown or unavailable terminal");
    expect(() => terminals.stop("term-1", workspace)).toThrow("unknown terminal");
  });

  it("publishes startup output and an early exit under the renderer-provided id before start resolves", async () => {
    let onData!: (data: string) => void;
    let onExit!: (event: { exitCode: number; signal?: number }) => void;
    const pty = {
      onData: (listener: typeof onData) => { onData = listener; return { dispose() {} }; },
      onExit: (listener: typeof onExit) => { onExit = listener; return { dispose() {} }; },
      write() {}, resize() {}, kill() {}
    } as unknown as IPty;
    const manager = new TerminalManager(vi.fn(() => {
      queueMicrotask(() => {
        onData("ready");
        onExit({ exitCode: 0 });
      });
      return pty;
    }) as never);
    const messages: unknown[] = [];
    manager.onMessage((message) => messages.push(message));
    const id = "term-11111111-1111-4111-8111-111111111111";

    await manager.start(id, "/tmp", workspace);
    await Promise.resolve();

    expect(messages).toEqual([
      { kind: "terminal-data", terminal: { id, data: "ready" } },
      { kind: "terminal-exit", terminal: { id, exitCode: 0 } }
    ]);
    expect(() => manager.write(id, "x", workspace)).toThrow("unknown terminal");
  });

  it("rejects a terminal owned by a stale workspace", () => {
    const terminals = new TerminalManager();
    const pty = { write: () => {}, resize: () => {}, kill: () => {} };
    const state = terminals as unknown as {
      terminals: Map<string, { pty: typeof pty; workspaceId: string }>;
    };
    state.terminals.set("term-1", { pty, workspaceId: workspace.id });
    const current = { id: "22222222-2222-4222-8222-222222222222", generation: 2 };
    expect(() => terminals.write("term-1", "x", current)).toThrow("stale terminal");
    expect(() => terminals.stop("term-1", current)).toThrow("stale terminal");
  });

  it("starts an explicit command when a TUI command is supplied", async () => {
    const pty = { onData: () => ({ dispose() {} }), onExit: () => ({ dispose() {} }), write() {}, resize() {}, kill() {} } as unknown as IPty;
    const spawnPty = vi.fn(() => pty);
    const manager = new TerminalManager(spawnPty as never);
    const command: TerminalCommand = { command: "opencode", args: ["--session", "session-1"], env: { OPENCODE_CLI_CONFIG_CONTENT: '{"theme":{"name":"system"}}' } };

    await manager.start("term-11111111-1111-4111-8111-111111111111", "/tmp", workspace, command);

    expect(spawnPty).toHaveBeenCalledWith("opencode", ["--session", "session-1"], expect.objectContaining({ cwd: "/tmp", env: expect.objectContaining(command.env) }));
  });

  it("uses the renderer's initial dimensions before the shell emits its first prompt", async () => {
    const pty = { onData: () => ({ dispose() {} }), onExit: () => ({ dispose() {} }), write() {}, resize() {}, kill() {} } as unknown as IPty;
    const spawnPty = vi.fn(() => pty);
    const manager = new TerminalManager(spawnPty as never);

    await manager.start("term-sized", "/tmp", workspace, undefined, { cols: 73, rows: 18 });

    expect(spawnPty).toHaveBeenCalledWith(expect.any(String), [], expect.objectContaining({ cols: 73, rows: 18 }));
  });

  it("stopAll kills, waits for exits, then detaches callbacks", async () => {
    const listeners = new Map<string, Array<(event: { exitCode: number }) => void>>();
    const killed: string[] = [];
    const spawnPty = vi.fn((...args: unknown[]) => {
      const marker = JSON.stringify(args.slice(0, 2));
      return {
        onData: () => ({ dispose() {} }),
        onExit: (listener: (event: { exitCode: number }) => void) => {
          const list = listeners.get(marker) ?? [];
          list.push(listener);
          listeners.set(marker, list);
          return { dispose() {} };
        },
        write() {}, resize() {},
        kill() { killed.push(marker); }
      };
    });
    const manager = new TerminalManager(spawnPty as never);
    await manager.start("term-a", "/tmp", workspace);
    await manager.start("term-b", "/tmp", workspace);

    const stopping = manager.stopAll(1000);
    await Promise.resolve();
    // Both children get SIGTERM; exits arriving drain the shutdown.
    for (const list of listeners.values()) for (const listener of list) listener({ exitCode: 0 });
    await stopping;

    expect(killed).toHaveLength(2);
    const state = manager as unknown as { terminals: Map<string, unknown> };
    expect(state.terminals.size).toBe(0);
  });

  it("stopAll detaches stragglers that never exit instead of hanging quit", async () => {
    vi.useFakeTimers();
    try {
      const pty = {
        onData: () => ({ dispose: vi.fn() }),
        onExit: () => ({ dispose: vi.fn() }),
        write() {}, resize() {},
        kill: vi.fn()
      } as unknown as IPty;
      const manager = new TerminalManager(vi.fn(() => pty) as never);
      await manager.start("term-stuck", "/tmp", workspace);

      const stopping = manager.stopAll(500);
      await vi.advanceTimersByTimeAsync(600);
      await stopping;

      expect(pty.kill).toHaveBeenCalled();
      const state = manager as unknown as { terminals: Map<string, unknown> };
      expect(state.terminals.size).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("terminal platform configuration", () => {
  it("selects each platform's normal interactive shell", () => {
    expect(defaultShell("darwin", {})).toBe("/bin/zsh");
    expect(defaultShell("linux", {})).toBe("/bin/bash");
    expect(defaultShell("win32", {})).toBe("powershell.exe");
    expect(defaultShell("darwin", { SHELL: "/opt/homebrew/bin/fish" })).toBe("/opt/homebrew/bin/fish");
    expect(defaultShell("win32", { COMSPEC: "C:\\Windows\\System32\\cmd.exe" })).toBe("C:\\Windows\\System32\\cmd.exe");
  });
});
