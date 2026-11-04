import { mkdtemp, realpath, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RuntimeAdapter } from "./runtimes/runtime-adapter";
import { RuntimeSessionIndex } from "./runtimes/runtime-session-index";

vi.mock("electron", () => ({ shell: { trashItem: vi.fn(), openPath: vi.fn() } }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { VelaBackend } from "./opencode";

const roots: string[] = [];

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

function adapter(directory: string): RuntimeAdapter {
  return {
    manifest: {
      protocolVersion: 1,
      id: "deepseek",
      name: "DeepSeek Harness",
      version: "0.1.0-rc.7",
      available: true,
      capabilities: {
        attachments: false,
        commands: false,
        models: true,
        agents: false,
        permissions: false,
        providerCredentials: false,
        sessionFork: false,
        sessionResume: true,
        steering: false,
        tui: false
      }
    },
    connect: vi.fn(async () => true),
    createSession: vi.fn(async () => ({ id: "deepseek-session", directory })),
    listSessions: vi.fn(async () => []),
    sessionInfo: vi.fn(async () => ({ id: "deepseek-session", directory })),
    sessionTranscript: vi.fn(async () => ({ transcript: [], todos: [] })),
    prompt: vi.fn(async () => {}),
    interrupt: vi.fn(async () => {}),
    listModels: vi.fn(async () => [{ id: "deepseek-chat", providerID: "deepseek-official", name: "DeepSeek Chat" }] ),
    sessionSelection: vi.fn(async () => ({ model: { id: "deepseek-chat", providerID: "deepseek-official", name: "DeepSeek Chat" } })),
    switchModel: vi.fn(async () => {}),
    subscribe: async function* () {},
    stop: vi.fn(async () => {})
  };
}

describe("runtime routing", () => {
  it("keeps DeepSeek disabled in the application backend", async () => {
    const directory = await mkdtemp(path.join(os.tmpdir(), "vela-disabled-runtime-"));
    roots.push(directory);
    const runtimeIndex = new RuntimeSessionIndex(path.join(directory, "runtime-sessions.json"));
    await runtimeIndex.put({
      id: "legacy-deepseek-session",
      runtimeID: "deepseek",
      title: "Legacy session",
      directory,
      updatedAt: Date.now()
    });
    const backend = new VelaBackend(() => {}, undefined, runtimeIndex);

    await expect(backend.openSession(directory, 1, "deepseek")).rejects.toThrow("DeepSeek Harness runtime is disabled");
    await expect(backend.listSessions()).resolves.toEqual([]);
    await expect(backend.runtimeManifests()).resolves.toEqual([
      expect.objectContaining({ id: "opencode" })
    ]);
    await backend.stop();
  });

  it("keeps workspace services in core and routes agent operations by session runtime", async () => {
    const directory = await mkdtemp(path.join(os.tmpdir(), "vela-runtime-"));
    roots.push(directory);
    const runtime = adapter(directory);
    const backend = new VelaBackend(() => {}, () => runtime, new RuntimeSessionIndex(path.join(directory, "runtime-sessions.json")));
    const session = await backend.openSession(directory, 1, "deepseek");
    expect(session).toMatchObject({ id: "deepseek-session", runtimeID: "deepseek" });
    await expect(backend.workspaceDirectory(session.workspace)).resolves.toBe(session.directory);
    await backend.prompt(session.workspace, "hello");
    expect(runtime.prompt).toHaveBeenCalledWith("deepseek-session", "hello");
    await expect(backend.listModels(session.workspace)).resolves.toEqual([
      { id: "deepseek-chat", providerID: "deepseek-official", name: "DeepSeek Chat" }
    ]);
    await backend.stop();
    expect(runtime.stop).toHaveBeenCalledOnce();

    const reopenedRuntime = adapter(directory);
    const reopenedBackend = new VelaBackend(
      () => {},
      () => reopenedRuntime,
      new RuntimeSessionIndex(path.join(directory, "runtime-sessions.json"))
    );
    const reopened = await reopenedBackend.openSessionById("deepseek-session", 2);
    expect(reopened.session).toMatchObject({ id: "deepseek-session", runtimeID: "deepseek" });
    expect(reopenedRuntime.connect).toHaveBeenCalledOnce();
    await reopenedBackend.stop();
  });

  it("remaps a cold-reopened session to the selected runtime on the same directory", async () => {
    const directory = await realpath(await mkdtemp(path.join(os.tmpdir(), "vela-runtime-remap-")));
    roots.push(directory);
    const client = {
      session: {
        create: vi.fn(async ({ location }: { location: { directory: string } }) => ({
          id: `opencode-${location.directory.split("/").pop()}`
        })),
        get: vi.fn(async ({ sessionID }: { sessionID: string }) => ({
          id: sessionID,
          location: { directory }
        }))
      },
      message: { list: vi.fn(async () => []) },
      file: { read: vi.fn(async () => "content") }
    };
    const runtime = adapter(directory);
    const backend = new VelaBackend(() => {}, () => runtime, new RuntimeSessionIndex(path.join(directory, "runtime-sessions.json")));
    const state = backend as unknown as { client: unknown };
    state.client = client;
    const opencode = await backend.openSession(directory, 1);
    expect(opencode.runtimeID ?? "opencode").toBe("opencode");
    await backend.closeSession(opencode.workspace);

    const remapped = await backend.openSessionById(opencode.id, 2, "deepseek");
    expect(remapped.session).toMatchObject({ runtimeID: "deepseek", directory });
    expect(remapped.session.id).not.toBe(opencode.id);
    expect(runtime.createSession).toHaveBeenCalledWith(directory);
    expect(client.session.get).toHaveBeenCalledWith({ sessionID: opencode.id });
    await backend.stop();
  });
});
