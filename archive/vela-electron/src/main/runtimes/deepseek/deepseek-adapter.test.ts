import { describe, expect, it, vi } from "vitest";
import { DeepSeekRuntimeAdapter, deepSeekStartupUrl } from "./deepseek-adapter";
import { DeepSeekRpcClient } from "./deepseek-rpc";

function client(responses: Record<string, unknown>): DeepSeekRpcClient {
  return {
    call: vi.fn(async (method: string) => responses[method]),
    events: vi.fn()
  } as unknown as DeepSeekRpcClient;
}

describe("DeepSeekRuntimeAdapter", () => {
  it("recognizes only the explicit startup line", () => {
    expect(deepSeekStartupUrl("booting\ndsh web: http://127.0.0.1:4123\n")).toBe("http://127.0.0.1:4123");
    expect(deepSeekStartupUrl("visit http://127.0.0.1:4123")).toBeNull();
  });

  it("reports the verified rc.7 capability subset", async () => {
    const rpc = client({ "host.describe": { version: "0.1.0-rc.7" } });
    const adapter = new DeepSeekRuntimeAdapter({ directory: "/repo", client: rpc });
    await expect(adapter.connect()).resolves.toBe(true);
    expect(adapter.manifest).toMatchObject({
      protocolVersion: 1,
      id: "deepseek",
      version: "0.1.0-rc.7",
      available: true,
      capabilities: { models: true, sessionResume: true, steering: false, attachments: false, permissions: false, tui: false }
    });
  });

  it("maps native sessions and model reasoning efforts", async () => {
    const rpc = client({
      "session.list": {
        items: [
          { sessionId: "blank", updatedAt: 1, blank: true, cwd: "/repo" },
          { sessionId: "s1", updatedAt: 2, blank: false, cwd: "/repo", projections: { values: { title: "Fix tests" } } }
        ]
      },
      "session.models": {
        current: { provider: "deepseek-official", model: "deepseek-chat", reasoningEffort: "high" },
        groups: [{ id: "deepseek-official", models: [{ id: "deepseek-chat", name: "DeepSeek Chat", reasoning: { efforts: [{ id: "low" }, { id: "high" }] } }] }]
      }
    });
    const adapter = new DeepSeekRuntimeAdapter({ directory: "/repo", client: rpc });
    await expect(adapter.listSessions()).resolves.toEqual([{ id: "s1", runtimeID: "deepseek", title: "Fix tests", directory: "/repo", updatedAt: 2 }]);
    await expect(adapter.listModels("s1")).resolves.toEqual([{ id: "deepseek-chat", providerID: "deepseek-official", name: "DeepSeek Chat", variants: ["low", "high"], variant: "high" }]);
  });

  it("publishes native child sessions through the shared session stream", async () => {
    const rpc = client({});
    rpc.events = vi.fn(async function* (stream: "mux" | "host") {
      if (stream === "host") {
        yield {
          rpcId: "child-added",
          payload: {
            type: "host/session-added",
            sessionId: "child-1",
            parentSessionId: "parent-1",
            origin: "subagent",
            cwd: "/repo",
            agentPreset: "researcher"
          }
        };
      }
    }) as DeepSeekRpcClient["events"];
    const adapter = new DeepSeekRuntimeAdapter({ directory: "/repo", client: rpc });
    const controller = new AbortController();
    const iterator = adapter.subscribe(controller.signal)[Symbol.asyncIterator]();
    const next = await iterator.next();
    controller.abort();
    expect(next.value).toMatchObject({
      sessionID: "child-1",
      event: {
        type: "stream.event",
        eventType: "session.created",
        data: {
          sessionID: "child-1",
          parentID: "parent-1",
          agent: "researcher",
          origin: "subagent"
        }
      }
    });
  });
});
