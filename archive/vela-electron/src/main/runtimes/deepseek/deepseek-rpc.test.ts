import { describe, expect, it, vi } from "vitest";
import { DeepSeekRpcClient, DeepSeekRpcError, deepSeekBaseUrl } from "./deepseek-rpc";

function rpcFetch(result: unknown, options: ResponseInit = {}): typeof fetch {
  return vi.fn(async (_request: Parameters<typeof fetch>[0], init?: RequestInit) => {
    const body = JSON.parse(String(init?.body)) as { rpcId: string };
    const value = typeof result === "function" ? result(body) : result;
    return new Response(JSON.stringify(value), {
      status: 200,
      headers: { "content-type": "application/json" },
      ...options
    });
  }) as typeof fetch;
}

describe("DeepSeekRpcClient", () => {
  it("allows only loopback HTTP endpoints", () => {
    expect(deepSeekBaseUrl("http://127.0.0.1:8080").host).toBe("127.0.0.1:8080");
    expect(deepSeekBaseUrl("http://127.42.0.9:8080").hostname).toBe("127.42.0.9");
    expect(() => deepSeekBaseUrl("https://127.0.0.1:8080")).toThrow(/loopback/);
    expect(() => deepSeekBaseUrl("http://example.com:8080")).toThrow(/loopback/);
    expect(() => deepSeekBaseUrl("http://user@127.0.0.1:8080")).toThrow(/loopback/);
  });

  it("sends the native envelope and validates correlation", async () => {
    const fetcher = rpcFetch(({ rpcId }: { rpcId: string }) => ({
      type: "server-response",
      rpcId,
      result: { ok: true, value: { version: "0.1.0-rc.7" } }
    }));
    const client = new DeepSeekRpcClient("http://127.0.0.1:8080", fetcher);
    await expect(client.call<{ version: string }>("host.describe", {})).resolves.toEqual({ version: "0.1.0-rc.7" });
    const [, init] = vi.mocked(fetcher).mock.calls[0];
    expect(JSON.parse(String(init?.body))).toMatchObject({ type: "client-request", method: "host.describe", payload: {} });
    expect(init?.redirect).toBe("manual");
  });

  it("rejects mismatched ids, redirects, HTML, and malformed JSON", async () => {
    const mismatch = new DeepSeekRpcClient("http://127.0.0.1:8080", rpcFetch({ type: "server-response", rpcId: "wrong", result: { ok: true, value: {} } }));
    await expect(mismatch.call("host.describe", {})).rejects.toThrow(/mismatched/);
    const redirect = new DeepSeekRpcClient("http://127.0.0.1:8080", vi.fn(async () => new Response(null, { status: 302, headers: { location: "/" } })) as typeof fetch);
    await expect(redirect.call("host.describe", {})).rejects.toThrow(/redirect/);
    const html = new DeepSeekRpcClient("http://127.0.0.1:8080", vi.fn(async () => new Response("<html>", { headers: { "content-type": "text/html" } })) as typeof fetch);
    await expect(html.call("host.describe", {})).rejects.toThrow(/non-JSON/);
    const malformed = new DeepSeekRpcClient("http://127.0.0.1:8080", vi.fn(async () => new Response("{", { headers: { "content-type": "application/json" } })) as typeof fetch);
    await expect(malformed.call("host.describe", {})).rejects.toThrow(/malformed/);
  });

  it("preserves native business errors", async () => {
    const client = new DeepSeekRpcClient("http://127.0.0.1:8080", rpcFetch(({ rpcId }: { rpcId: string }) => ({
      type: "server-response",
      rpcId,
      result: { ok: false, error: { code: "session-not-found", message: "missing", details: { sessionId: "s" } } }
    })));
    await expect(client.call("session.history", { sessionId: "s" })).rejects.toEqual(
      expect.objectContaining<Partial<DeepSeekRpcError>>({ code: "session-not-found", message: "missing" })
    );
  });

  it("parses native WebSocket frames", async () => {
    class FakeSocket extends EventTarget {
      constructor() {
        super();
        queueMicrotask(() => {
          this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "server-request", rpcId: "a", method: "events.mux", payload: { type: "session/subscribed" } }) }));
          this.dispatchEvent(new Event("close"));
        });
      }

      close(): void {
        this.dispatchEvent(new Event("close"));
      }
    }
    const sockets = vi.fn(() => new FakeSocket() as unknown as WebSocket);
    const client = new DeepSeekRpcClient("http://127.0.0.1:8080", fetch, sockets);
    const frames = [];
    for await (const frame of client.events("mux", new AbortController().signal)) frames.push(frame);
    expect(frames).toEqual([{ type: "server-request", rpcId: "a", method: "events.mux", payload: { type: "session/subscribed" } }]);
    expect(sockets).toHaveBeenCalledWith("ws://127.0.0.1:8080/api/events.mux");
  });
});
