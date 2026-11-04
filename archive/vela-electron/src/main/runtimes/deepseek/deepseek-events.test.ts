import { describe, expect, it } from "vitest";
import { blocks, deepSeekRuntimeEvent, deepSeekTranscript, parsedArguments, record } from "./deepseek-events";

describe("deepSeekTranscript", () => {
  it("projects user, assistant, tool, and todo events", () => {
    const result = deepSeekTranscript([
      { event: { type: "user/message", seq: 1, data: { id: "u1", source: { kind: "user" }, content: [{ type: "text", text: "Fix it" }] } } },
      { event: { type: "assistant/message", seq: 2, time: 10, data: { message: { id: "a1", content: [{ type: "reasoning", text: "Inspect" }, { type: "tool-call", id: "c1", name: "bash", arguments: "{\"command\":\"npm test\"}" }] } } } },
      { event: { type: "tool/result", seq: 3, time: 20, data: { message: { source: { kind: "tool", callId: "c1" }, content: [{ type: "tool-result", content: [{ type: "text", text: "passed" }] }] } } } },
      { event: { type: "todo/write", seq: 4, data: { todos: [{ id: "t1", content: "Test", status: "in-progress" }] } } }
    ]);
    expect(result.transcript[0]).toMatchObject({ kind: "user", text: "Fix it" });
    expect(result.transcript[1]).toMatchObject({
      kind: "assistant",
      parts: [
        { kind: "reasoning", text: "Inspect" },
        { kind: "tool", tool: { id: "c1", status: "success", output: "passed", duration: 10 } }
      ]
    });
    expect(result.todos).toEqual([{ id: "t1", content: "Test", status: "in_progress" }]);
  });

  it("keeps plugin context distinct from user prompts", () => {
    const result = deepSeekTranscript([
      { event: { type: "user/message", seq: 1, data: { source: { kind: "plugin", summary: "Files changed" }, content: [{ type: "text", text: "context" }] } } }
    ]);
    expect(result.transcript).toEqual([{ kind: "synthetic", id: "deepseek-context-1", text: "context", description: "Files changed" }]);
  });

  it("hides internal context and projects turn failures", () => {
    const result = deepSeekTranscript([
      { event: { type: "user/message", seq: 1, data: { source: { kind: "agent-instructions" }, content: [{ type: "text", text: "instructions" }] } } },
      { event: { type: "user/message", seq: 2, data: { source: { kind: "plugin", plugin: "@deepseek-ai/dsh-system-prompt" }, content: [{ type: "text", text: "runtime context" }] } } },
      { event: { type: "turn/end", seq: 3, data: { reason: { kind: "error", error: { message: "Provider is not configured: openai" } } } } }
    ]);
    expect(result.transcript).toEqual([
      { kind: "status", id: "deepseek-error-3", text: "[DEEPSEEK_TURN_FAILED] Provider is not configured: openai", tone: "error" }
    ]);
  });

  it("normalizes only verified native lifecycle events", () => {
    expect(deepSeekRuntimeEvent({ type: "host/session-status", sessionId: "s", running: true })).toEqual({ type: "execution.started" });
    expect(deepSeekRuntimeEvent({ type: "session/event", sessionId: "s", event: { type: "assistant/message" } })).toEqual({ type: "transcript.changed" });
    expect(deepSeekRuntimeEvent({ type: "session/event", sessionId: "s", event: { type: "turn/end", data: { reason: { kind: "error", error: { message: "Provider failed" } } } } })).toEqual({ type: "execution.error", code: "DEEPSEEK_TURN_FAILED", message: "Provider failed" });
    expect(deepSeekRuntimeEvent({ type: "session/event", sessionId: "s", event: { type: "unknown" } })).toBeNull();
  });
});

describe("native event value acceptance", () => {
  it.each([
    ["{broken", "{broken"],
    ["null", null],
    ["false", false],
    ['{"file_path":"README.md"}', { file_path: "README.md" }],
    [{ file_path: "README.md" }, { file_path: "README.md" }]
  ])("retains JSON parse and passthrough behavior for %j", (argumentsValue, expected) => {
    const result = deepSeekTranscript([{ event: {
      type: "assistant/message", seq: 1,
      data: { message: { content: [null, [], { type: "tool-call", id: "call", name: "read", arguments: argumentsValue }] } }
    } }]);
    const assistant = result.transcript[0];
    expect(assistant?.kind).toBe("assistant");
    if (assistant?.kind !== "assistant") throw new Error("assistant transcript missing");
    expect(assistant.parts).toHaveLength(1);
    expect(assistant.parts[0]).toMatchObject({ kind: "tool", tool: { inputValue: expected } });
  });

  it("ignores an array masquerading as an assistant message", () => {
    const message = Object.assign([], { content: [{ type: "text", text: "hidden" }] });
    expect(deepSeekTranscript([{ event: { type: "assistant/message", seq: 1, data: { message } } }]).transcript).toEqual([]);
  });
});

it("preserves native record identity, fresh filtered arrays, and nonstring argument identity", () => {
  const value = { type: "text", text: "body" };
  expect(record(value)).toBe(value);
  expect(record([])).toBeNull();
  expect(record(null)).toBeNull();
  expect(blocks(null)).not.toBe(blocks(null));
  const filtered = blocks([null, [], value]);
  expect(filtered).toEqual([value]);
  expect(filtered[0]).toBe(value);
  expect(parsedArguments(value)).toBe(value);
});
