import { describe, expect, it } from "vitest";
import { DeepSeekStreamProjector } from "./deepseek-stream";

function event(type: string, seq: number, data: Record<string, unknown>) {
  return { type, seq, time: seq * 10, data };
}

describe("DeepSeekStreamProjector", () => {
  it("projects native reasoning and text chunks into the shared stream", () => {
    const projector = new DeepSeekStreamProjector();
    const values = [
      ...projector.project("s1", event("step/start", 1, { turn: 2, step: 3 })),
      ...projector.project("s1", event("assistant/chunk", 2, { turn: 2, step: 3, chunk: { type: "block-start", index: 0, blockType: "reasoning" } })),
      ...projector.project("s1", event("assistant/chunk", 3, { turn: 2, step: 3, chunk: { type: "reasoning-delta", index: 0, text: "Inspect" } })),
      ...projector.project("s1", event("assistant/chunk", 4, { turn: 2, step: 3, chunk: { type: "block-end", index: 0, block: { type: "reasoning", text: "Inspect files" } } })),
      ...projector.project("s1", event("assistant/chunk", 5, { turn: 2, step: 3, chunk: { type: "block-start", index: 1, blockType: "text" } })),
      ...projector.project("s1", event("assistant/chunk", 6, { turn: 2, step: 3, chunk: { type: "text-delta", index: 1, text: "Done" } }))
    ];
    expect(values.map((value) => value.type === "stream.event" ? value.eventType : value.type)).toEqual([
      "session.step.started",
      "session.reasoning.started",
      "session.reasoning.delta",
      "session.reasoning.ended",
      "session.text.started",
      "session.text.delta"
    ]);
    expect(values[0]).toMatchObject({ data: { assistantMessageID: "deepseek:s1:00000002:00000003" } });
  });

  it("retains native tool views and correlates results with their assistant step", () => {
    const projector = new DeepSeekStreamProjector();
    projector.project("s1", event("tool/call", 1, {
      turn: 4,
      step: 1,
      callId: "call-1",
      name: "bash",
      arguments: "{\"command\":\"npm test\"}"
    }), { for: "call", view: { card: "terminal", command: "npm test" } });
    const values = projector.project("s1", event("tool/result", 2, {
      turn: 4,
      step: 2,
      message: {
        source: { kind: "tool", callId: "call-1" },
        content: [{ type: "tool-result", content: [{ type: "text", text: "passed" }] }]
      }
    }), { for: "result", view: { card: "terminal", exitCode: 0 } });
    expect(values).toEqual([expect.objectContaining({
      type: "stream.event",
      eventType: "session.tool.success",
      data: expect.objectContaining({
        assistantMessageID: "deepseek:s1:00000004:00000001",
        callID: "call-1",
        output: "passed",
        resultState: { card: "terminal", exitCode: 0 }
      })
    })]);
  });

  it("withdraws partial output before publishing a retry", () => {
    const projector = new DeepSeekStreamProjector();
    const values = projector.project("s1", event("llm/retry", 3, {
      turn: 7,
      step: 0,
      retry: 2,
      delayMs: 500,
      failure: { code: "TRANSPORT", message: "reset" }
    }));
    expect(values.map((value) => value.type === "stream.event" ? value.eventType : value.type)).toEqual([
      "message.removed",
      "session.retry.scheduled"
    ]);
    expect(values[1]).toMatchObject({ data: { attempt: 2, at: 530, error: { code: "TRANSPORT", message: "reset" } } });
  });

  it("keeps code dispatches nested under their root tool", () => {
    const projector = new DeepSeekStreamProjector();
    projector.project("s1", event("tool/call", 1, {
      turn: 1,
      step: 0,
      callId: "root",
      name: "run_code",
      arguments: "{}"
    }));
    projector.project("s1", event("tool/code-dispatch-start", 2, {
      turn: 1,
      step: 0,
      rootCallId: "root",
      parentCallId: "root",
      subCallId: "child",
      name: "read",
      arguments: { file_path: "README.md" }
    }));
    const values = projector.project("s1", event("tool/code-dispatch", 3, {
      turn: 1,
      step: 0,
      rootCallId: "root",
      parentCallId: "root",
      subCallId: "child",
      name: "read",
      arguments: { file_path: "README.md" },
      content: [{ type: "text", text: "hello" }]
    }));
    expect(values[0]).toMatchObject({
      type: "stream.event",
      eventType: "session.tool.progress",
      data: {
        callID: "root",
        metadata: { deepseek: { subCalls: [{ id: "child", title: "read", status: "success", output: "hello" }] } }
      }
    });
  });
});

it.each([
  ["{broken", "{broken"],
  ["null", null],
  ["false", false],
  ['{"file_path":"README.md"}', { file_path: "README.md" }],
  [{ file_path: "README.md" }, { file_path: "README.md" }]
])("retains live native tool arguments for %j", (argumentsValue, expected) => {
  const projector = new DeepSeekStreamProjector();
  expect(projector.project("s", event("tool/call", 1, { callId: "call", name: "read", arguments: argumentsValue })))
    .toMatchObject([{ type: "stream.event", eventType: "session.tool.called", data: { input: expected } }]);
});
