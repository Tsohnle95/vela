// @vitest-environment node
import { describe, expect, it, vi } from "vitest";
import { VelaBackend } from "./opencode";

vi.mock("electron", () => ({ app: { getPath: () => "/fixture-home" }, shell: {} }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

function backend(reply: unknown): VelaBackend {
  return Object.assign(Object.create(VelaBackend.prototype), {
    runtimeAdapters: new Map(), client: { session: { get: vi.fn(async () => reply) } }
  });
}

describe("backend session usage acceptance", () => {
  it.each([1, true, "tokens", []])("retains truthy primitive and array token acceptance: %j", async (tokens) => {
    expect(await backend({ data: { tokens } }).sessionUsage("session")).toEqual({
      cost: 0, tokens: { input: 0, output: 0, reasoning: 0, cache: { read: 0, write: 0 } }
    });
  });

  it.each([undefined, null, false, 0, ""])("rejects missing/falsy tokens: %j", async (tokens) => {
    expect(await backend({ data: { tokens } }).sessionUsage("session")).toBeNull();
  });

  it("preserves raw cost/token/cache read order and a single cache snapshot", async () => {
    const reads: string[] = [];
    const tokens = {
      get input() { reads.push("input"); return 10; },
      get output() { reads.push("output"); return 2; },
      get reasoning() { reads.push("reasoning"); return 3; },
      get cache() { reads.push("cache"); return {
        get read() { reads.push("cache.read"); return 4; },
        get write() { reads.push("cache.write"); return 5; }
      }; }
    };
    const raw = {
      get cost() { reads.push("cost"); return 1; },
      get tokens() { reads.push("tokens"); return tokens; }
    };
    expect(await backend({ data: raw }).sessionUsage("session")).toEqual({
      cost: 1, tokens: { input: 10, output: 2, reasoning: 3, cache: { read: 4, write: 5 } }
    });
    expect(reads).toEqual(["cost", "tokens", "cache", "input", "output", "reasoning", "cache.read", "cache.write"]);
  });
});
