// @vitest-environment node
import { tmpdir } from "node:os";
import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  app: { getPath: () => tmpdir() },
  shell: { trashItem: vi.fn() }
}));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { replayTranscript } from "./session-replay";

describe("replay retention", () => {
  it("assigns type-specific ordinals to interleaved V2 parts while retaining repeated text and explicit IDs", () => {
    const transcript = replayTranscript([{
      id: "a", type: "assistant", time: { completed: 100 }, content: [
        { type: "reasoning", text: "Plan" },
        { type: "text", text: "Repeated text" },
        { type: "reasoning", text: "More thinking" },
        { type: "text", text: "Repeated text" },
        { type: "text", id: "prt_existing", text: "Canonical part" }
      ]
    }]);
    expect(transcript[0]).toMatchObject({ parts: [
      { id: "a:reasoning:0" }, { id: "a:text:0" }, { id: "a:reasoning:1" }, { id: "a:text:1" }, { id: "prt_existing" }
    ] });
  });

  it("replays V2 message presentation with upstream parent and selection inheritance", () => {
    const transcript = replayTranscript([
      { id: "agent", type: "agent-switched", agent: "plan" },
      { id: "model", type: "model-switched", model: { id: "old", providerID: "p" } },
      { id: "u", type: "user", text: "hello", time: { created: 1000 } },
      {
        id: "a", type: "assistant", agent: "build", model: { id: "actual", providerID: "p" },
        time: { created: 1100, completed: 4300 }, content: [{ type: "text", text: "hello back" }]
      },
      { id: "u2", type: "user", text: "another question", time: { created: 5000 } }
    ]);
    expect(transcript.find((item) => item.id === "u")).toMatchObject({
      createdAt: 1000, agent: "build", model: { id: "actual", providerID: "p" }
    });
    expect(transcript.find((item) => item.id === "a")).toMatchObject({
      createdAt: 1100, completedAt: 4300, agent: "build", model: { id: "actual", providerID: "p" }, parentID: "u"
    });
    expect(transcript.find((item) => item.id === "u2")).toMatchObject({
      createdAt: 5000, agent: "build", model: { id: "actual", providerID: "p" }
    });
  });

  it("retains legacy presentation and V2 interruption semantics", () => {
    const transcript = replayTranscript([
      {
        info: { id: "u", role: "user", agent: "build", model: { modelID: "m", providerID: "p" }, time: { created: 1000 } },
        parts: [{ type: "text", text: "hello" }]
      },
      {
        info: {
          id: "a", role: "assistant", agent: "build", modelID: "m", providerID: "p", parentID: "u",
          time: { created: 1100, completed: 1500 }, error: { name: "MessageAbortedError", data: { message: "Stopped" } }
        },
        parts: [{ id: "text", type: "text", text: "partial" }]
      },
      {
        id: "a2", type: "assistant", time: { created: 1700, completed: 2000 },
        agent: "build", model: { id: "m", providerID: "p" }, error: { type: "Interrupted", message: "Stopped" }, content: []
      }
    ]);
    expect(transcript[0]).toMatchObject({ agent: "build", model: { id: "m", providerID: "p" }, createdAt: 1000 });
    expect(transcript[1]).toMatchObject({ interrupted: true, parentID: "u", model: { id: "m", providerID: "p" }, completedAt: 1500 });
    expect(transcript[2]).toMatchObject({ interrupted: true, completedAt: 2000 });
  });

  it("reconstructs failed tools and assistant messages as settled", () => {
    const transcript = replayTranscript([{
      info: {
        id: "assistant-1",
        role: "assistant",
        error: { message: "provider failed" }
      },
      parts: [{
        id: "tool-1",
        type: "tool",
        callID: "call-1",
        tool: "read",
        state: { status: "failed", error: { message: "File not found" } }
      }]
    }]);

    expect(transcript[0]).toMatchObject({
      kind: "assistant",
      completed: true,
      error: "[VELA_ASSISTANT_FAILED] provider failed",
      parts: [{ kind: "tool", tool: { status: "failed" } }]
    });
  });

  it("preserves complete projected tool output while retaining file content", () => {
    const output = "start\n" + "x".repeat(32 * 1024) + "\nend";
    const transcript = replayTranscript([{
      info: { id: "assistant-1", role: "assistant" },
      parts: [{
        id: "tool-1",
        type: "tool",
        callID: "call-1",
        tool: "read",
        state: {
          status: "completed",
          output,
          content: [
            { type: "text", text: output },
            { type: "file", uri: "file:///result", mime: "text/plain", name: "result.txt" }
          ]
        }
      }]
    }]);
    const assistant = transcript[0];
    const part = assistant?.kind === "assistant" ? assistant.parts[0] : undefined;

    expect(part?.kind === "tool" ? part.tool.output : "").toBe(output);
    expect(part?.kind === "tool" ? part.tool.content : undefined).toEqual([
      { type: "file", uri: "file:///result", mime: "text/plain", name: "result.txt" }
    ]);
  });

  it("preserves complete replayed shell output", () => {
    const output = "start\n" + "x".repeat(32 * 1024) + "\nend";
    expect(replayTranscript([{
      id: "shell-1", type: "shell", command: "build", status: "exited", output: { output }
    }])).toMatchObject([{ kind: "shell", output }]);
  });
});
