// @vitest-environment node
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { IpcMainInvokeEvent } from "electron";
import type { VelaBackend } from "../opencode";
import type { TerminalManager } from "../terminal";
import type { TrustedIpcRegistrar } from "./registration";
import { registerAgentIpc } from "./agent";
import { registerFilesIpc } from "./files";
import { registerTerminalIpc } from "./terminal";

const workspace = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
const terminalID = "term-22222222-2222-4222-8222-222222222222";
const roots: string[] = [];
afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

function handlers(backend: Partial<VelaBackend>, terminals: Partial<TerminalManager> = {}) {
  const callbacks = new Map<string, (event: IpcMainInvokeEvent, ...args: any[]) => unknown>();
  const register: TrustedIpcRegistrar = (channel, callback) => {
    if (callbacks.has(channel)) throw new Error(`duplicate IPC channel ${channel}`);
    callbacks.set(channel, callback);
  };
  registerFilesIpc(register, backend as VelaBackend);
  registerAgentIpc(register, backend as VelaBackend);
  registerTerminalIpc(register, backend as VelaBackend, terminals as TerminalManager);
  return {
    channels: [...callbacks.keys()],
    invoke: (channel: string, ...args: unknown[]) => callbacks.get(channel)!({} as IpcMainInvokeEvent, ...args)
  };
}

describe("domain IPC registration", () => {
  it("registers each extracted channel once through the supplied trusted gate", () => {
    const { channels } = handlers({});
    expect(channels).toHaveLength(58);
    expect(channels).toEqual(expect.arrayContaining(["shell:state", "shell:fs-write", "shell:prompt", "shell:terminal-start"]));
  });

  it("rejects malformed file paths and prompt payloads before calling the backend", async () => {
    const readFile = vi.fn();
    const prompt = vi.fn();
    const { invoke } = handlers({ readFile, prompt });
    await expect(invoke("shell:fs-read", workspace, "../escape")).rejects.toThrow("invalid workspace path");
    await expect(invoke("shell:prompt", workspace, 42)).rejects.toMatchObject({ name: "VELA_PROMPT_FAILED" });
    expect(readFile).not.toHaveBeenCalled();
    expect(prompt).not.toHaveBeenCalled();
  });

  it("preserves prompt attachment defaults and normalized backend failures", async () => {
    const prompt = vi.fn().mockResolvedValue({ transcript: [], todos: [] });
    const { invoke } = handlers({ prompt });
    await invoke("shell:prompt", workspace, "hello");
    expect(prompt).toHaveBeenCalledWith(workspace, "hello", [], undefined);
    prompt.mockRejectedValue({ code: "PROVIDER_LIMIT", message: "try later" });
    await expect(invoke("shell:prompt", workspace, "hello")).rejects.toMatchObject({
      name: "PROVIDER_LIMIT", code: "PROVIDER_LIMIT", message: "try later"
    });
  });

  it("preserves command validation after workspace authorization", async () => {
    const workspaceDirectory = vi.fn().mockRejectedValue(new Error("stale workspace"));
    const runCommand = vi.fn();
    const { invoke } = handlers({ workspaceDirectory, runCommand });
    await expect(invoke("shell:run-command", workspace, null)).rejects.toThrow("stale workspace");
    expect(runCommand).not.toHaveBeenCalled();
  });

  it("keeps terminal defaults and stops a started PTY when its captured workspace becomes stale", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "vela-ipc-"));
    roots.push(root);
    const workspaceDirectory = vi.fn().mockResolvedValueOnce(root).mockRejectedValueOnce(new Error("stale workspace"));
    const start = vi.fn().mockResolvedValue(undefined);
    const stop = vi.fn();
    const { invoke } = handlers({ workspaceDirectory }, { start, stop });
    await expect(invoke("shell:terminal-start", workspace, terminalID)).rejects.toThrow("stale workspace");
    expect(start).toHaveBeenCalledWith(terminalID, root, workspace, undefined, { cols: 100, rows: 24 });
    expect(stop).toHaveBeenCalledWith(terminalID, workspace);
  });

  it("stops a TUI on stale post-start authorization and retains the original failure", async () => {
    const command = { command: "opencode", args: ["attach"] };
    const start = vi.fn().mockResolvedValue(undefined);
    const stop = vi.fn();
    const { invoke } = handlers({
      ptyDirectory: vi.fn().mockResolvedValue("/workspace"),
      tuiCommand: vi.fn().mockResolvedValue(command),
      workspaceDirectory: vi.fn().mockRejectedValue(new Error("stale workspace"))
    }, { start, stop });
    await expect(invoke("shell:agent-tui-start", workspace, terminalID)).rejects.toThrow("stale workspace");
    expect(start).toHaveBeenCalledWith(terminalID, "/workspace", workspace, command);
    expect(stop).toHaveBeenCalledWith(terminalID, workspace);
  });
});
