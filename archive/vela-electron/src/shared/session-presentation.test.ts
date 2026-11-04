import { describe, expect, it } from "vitest";
import { normalizeTodos, sessionUsageValues } from "./session-presentation";

describe("session presentation values", () => {
  it("retains todo defaults, source indices, whitespace and string priorities", () => {
    expect(normalizeTodos(null)).toEqual([]);
    expect(normalizeTodos({ content: "wrong envelope" })).toEqual([]);
    expect(normalizeTodos([
      null, {}, { content: "" }, { content: "Work", status: "unknown", id: "", priority: "" },
      { content: " ", status: "cancelled", id: "explicit", priority: 4 },
      Object.assign([], { content: "Array record", status: "in_progress" })
    ])).toEqual([
      { id: "todo-3", content: "Work", status: "pending", priority: "" },
      { id: "explicit", content: " ", status: "cancelled" },
      { id: "todo-5", content: "Array record", status: "in_progress" }
    ]);
  });

  it("zeros nonfinite and nonnumeric usage while retaining finite negative values", () => {
    expect(sessionUsageValues(NaN, { input: 3, output: Infinity, reasoning: "9" }, { read: -2, write: undefined })).toEqual({
      cost: 0, tokens: { input: 3, output: 0, reasoning: 0, cache: { read: -2, write: 0 } }
    });
    expect(sessionUsageValues(-1, {}, undefined)).toEqual({
      cost: -1, tokens: { input: 0, output: 0, reasoning: 0, cache: { read: 0, write: 0 } }
    });
  });

  it("uses the supplied cache snapshot without rereading the tokens cache", () => {
    const tokens = { input: 1, get cache(): never { throw new Error("Cache already captured"); } };
    expect(sessionUsageValues(2, tokens, { read: 3, write: 4 })).toEqual({
      cost: 2, tokens: { input: 1, output: 0, reasoning: 0, cache: { read: 3, write: 4 } }
    });
  });
});
