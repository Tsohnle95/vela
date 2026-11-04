// @vitest-environment node
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  app: { getPath: () => tmpdir() },
  shell: { trashItem: vi.fn(), openPath: async () => "" }
}));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { VelaBackend } from "./opencode";
import { RuntimeSessionIndex } from "./runtimes/runtime-session-index";

interface RawSession {
  id?: string;
  title?: string;
  parentID?: string;
  agent?: string;
  tokens?: { input?: number; output?: number; reasoning?: number; cache?: { read?: number; write?: number } };
  location?: { directory?: string };
  time?: { updated?: number; created?: number };
}

const roots: string[] = [];

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

function session(overrides: RawSession): RawSession {
  return { id: `ses_${Math.random().toString(36).slice(2, 10)}`, ...overrides };
}

function recent(): RawSession["time"] {
  return { updated: Date.now() - 60_000, created: Date.now() - 120_000 };
}

function old(): RawSession["time"] {
  const ago = Date.now() - 45 * 24 * 60 * 60 * 1000;
  return { updated: ago, created: ago };
}

function usedTokens(): RawSession["tokens"] {
  return { input: 100, output: 50, reasoning: 0, cache: { read: 10, write: 0 } };
}

function pagedClient(pages: Array<{ data: RawSession[]; next?: string | null }>): unknown {
  const remove = vi.fn(async () => {});
  const list = vi.fn(async (...calls: unknown[]) => {
    const input = calls[0] as { cursor?: string } | undefined;
    const index = input?.cursor ? Number(input.cursor) : 0;
    return { data: pages[index]?.data ?? [], cursor: { next: pages[index]?.next ?? null } };
  });
  return { session: { list, remove }, message: { list: vi.fn(async () => []) } };
}

async function fixture(client: unknown): Promise<VelaBackend> {
  const root = await mkdtemp(path.join(tmpdir(), "vela-retention-index-"));
  roots.push(root);
  const backend = new VelaBackend(
    () => {},
    () => { throw new Error("Runtime adapter is not used in retention tests"); },
    new RuntimeSessionIndex(path.join(root, "runtime-sessions.json"))
  );
  (backend as unknown as { client: unknown }).client = client;
  return backend;
}

describe("session retention", () => {
  it("keeps old conversations in history and hides conversation-less sessions", async () => {
    const time = recent();
    const keep = session({ title: "Real work", tokens: undefined, location: { directory: "/w/keep" }, time });
    const promptedButUntitled = session({ tokens: usedTokens(), location: { directory: "/w/prompted" }, time });
    const phantom = session({ location: { directory: "/w/phantom" }, time: recent() });
    const oldSession = session({ title: "Old chat", tokens: usedTokens(), location: { directory: "/w/old" }, time: old() });
    const client = pagedClient([{ data: [keep, promptedButUntitled, phantom, oldSession] }]);
    const backend = await fixture(client);

    const summaries = await backend.listSessions();

    expect(summaries.map((s) => s.id)).toEqual([keep.id, promptedButUntitled.id, oldSession.id]);
    expect(summaries[0].title).toBe("Real work");
    expect(summaries[1].title).toBe("prompted");
    expect(summaries[2].title).toBe("Old chat");
    expect((client as { session: { remove: ReturnType<typeof vi.fn> } }).session.remove).not.toHaveBeenCalled();
  });

  it("keeps paging until the session service has no more pages", async () => {
    const time = recent();
    const fillers = Array.from({ length: 3 }, () =>
      session({ title: "Filler", tokens: usedTokens(), location: { directory: "/w/fill" }, time })
    );
    const late = session({ title: "Late find", tokens: usedTokens(), location: { directory: "/w/late" }, time });
    const phantoms = Array.from({ length: 4 }, () => session({ location: { directory: "/w/x" }, time: recent() }));
    const client = pagedClient(
      [
        { data: [...phantoms.slice(0, 2), fillers[0]], next: "1" },
        { data: [...phantoms.slice(2), fillers[1]], next: "2" },
        { data: [fillers[2], late], next: null }
      ]
    );
    const backend = await fixture(client);

    const summaries = await backend.listSessions();

    expect(summaries.map((s) => s.title)).toEqual(["Filler", "Filler", "Filler", "Late find"]);
    expect((client as { session: { list: ReturnType<typeof vi.fn> } }).session.list).toHaveBeenCalledTimes(3);
  });

  it("returns conversation history beyond the former 30-session cap", async () => {
    const firstPage = Array.from({ length: 31 }, (_, index) => session({
      id: `ses_${index}`,
      title: `Chat ${index}`,
      tokens: usedTokens(),
      location: { directory: `/w/${index}` },
      time: recent()
    }));
    const older = session({ id: "ses_older", title: "Older chat", tokens: usedTokens(), location: { directory: "/w/older" }, time: old() });
    const backend = await fixture(pagedClient([
      { data: firstPage, next: "1" },
      { data: [older], next: null }
    ]));

    const summaries = await backend.listSessions();

    expect(summaries).toHaveLength(32);
    expect(summaries.some((summary) => summary.id === older.id)).toBe(true);
  });

  it("retries a failed history fetch once and throws instead of returning an empty transcript", async () => {
    const transient = { session: { list: vi.fn(async () => ({ data: [], cursor: {} })) }, message: { list: vi.fn()
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce({ data: [{ id: "m1", type: "user", text: "hello" }] }) } };
    const backend = await fixture(transient);
    const recovered = await backend.sessionTranscript("ses_x");
    expect(recovered.transcript.map((item) => item.kind)).toEqual(["user"]);
    expect(transient.message.list).toHaveBeenCalledTimes(2);

    const alwaysFailing = { session: { list: vi.fn(async () => ({ data: [], cursor: {} })) }, message: { list: vi.fn(async () => {
      throw new Error("down");
    }) } };
    const flaky = await fixture(alwaysFailing);
    await expect(flaky.sessionTranscript("ses_y")).rejects.toThrow("could not load conversation history");
    expect(alwaysFailing.message.list).toHaveBeenCalledTimes(2);
  });

  it("follows message.list pagination so long conversations load completely", async () => {
    const pageOf = (start: number, count: number) =>
      Array.from({ length: count }, (_, index) => ({ id: `m${start + index}`, type: "user", text: `page item ${start + index}` }));
    let calls = 0;
    const paged = { session: { list: vi.fn(async () => ({ data: [], cursor: {} })) }, message: { list: vi.fn(async (...inputs: unknown[]) => {
      const input = inputs[0] as { cursor?: string } | undefined;
      if (!input?.cursor) {
        expect(input).toEqual({ sessionID: "ses_long", order: "asc" });
        calls += 1;
        return { data: pageOf(0, 50), cursor: { next: "cursor-page-2" } };
      }
      expect(input).toEqual({ sessionID: "ses_long", cursor: "cursor-page-2" });
      calls += 1;
      return { data: pageOf(50, 39), cursor: { next: null } };
    }) } };
    const backend = await fixture(paged);

    const recovered = await backend.sessionTranscript("ses_long");

    expect(calls).toBe(2);
    expect(recovered.transcript).toHaveLength(89);
    expect(recovered.transcript.at(-1)).toMatchObject({ kind: "user", text: "page item 88" });
  });

});
