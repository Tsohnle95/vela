// @vitest-environment node
import { afterEach, describe, expect, it, vi } from "vitest";
import { LatestGeneration } from "@shared/generation";
import type { WorkspaceIdentity } from "@shared/types";

vi.mock("electron", () => ({ shell: { trashItem: vi.fn(), openPath: vi.fn() } }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { VelaBackend, type SessionContext } from "./opencode";

const workspace: WorkspaceIdentity = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };

function fixture(interrupt: () => Promise<unknown>): VelaBackend {
  const backend = new VelaBackend();
  const context: SessionContext = {
    workspace,
    sessionID: "session",
    directory: "/tmp/luno",
    sessionInfo: { id: "session", directory: "/tmp/luno", workspace },
    watchContext: {
      root: "/tmp/luno",
      sessionID: "session",
      workspace,
      snapshots: new Map(),
      lastKnown: new Map(),
      hasGit: false,
      timers: new Map()
    },
    watcher: null,
    activations: new LatestGeneration()
  };
  const state = backend as unknown as { client: unknown; contexts: Map<string, SessionContext> };
  state.client = { session: { interrupt } };
  state.contexts = new Map([[workspace.id, context]]);
  return backend;
}

describe("VelaBackend interrupt", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("accepts the no-content success returned by older OpenCode services", async () => {
    const interrupt = vi.fn(async () => {
      throw { reason: "UnexpectedStatus", cause: { status: 204 } };
    });
    const backend = fixture(interrupt);

    await expect(backend.interrupt(workspace)).resolves.toBeUndefined();
    expect(interrupt).toHaveBeenCalledWith({ sessionID: "session" });
  });

  it("keeps reporting real interrupt failures", async () => {
    const backend = fixture(async () => {
      throw { reason: "UnexpectedStatus", cause: { status: 500 } };
    });

    await expect(backend.interrupt(workspace)).rejects.toMatchObject({ reason: "UnexpectedStatus" });
  });
});
