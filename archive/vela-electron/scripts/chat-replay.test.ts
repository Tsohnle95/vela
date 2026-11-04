// @vitest-environment node
import { tmpdir } from "node:os";
import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ app: { getPath: () => tmpdir() }, shell: { trashItem: vi.fn() } }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { replayTranscript } from "../src/main/session-replay";
import { applyChatEvent, hydrateChatState, projectAssistantItems, type ChatDirectoryState } from "../src/renderer/src/chat-store";
import { mergeChatHistory } from "../src/renderer/src/chat-stream";

// Each side is typechecked by its owning project; this test exercises their handoff.
describe("main replay to renderer reconciliation", () => {
  it("reconciles streamed text with V2 history after reasoning without duplicating the answer", () => {
    const state: ChatDirectoryState = { message: {}, part: {}, session_status: {} };
    const event = (type: string, data: Record<string, unknown>): void => {
      applyChatEvent(state, "s", { id: type, type, created: 100, data: { sessionID: "s", assistantMessageID: "a", ...data } });
    };
    event("session.step.started", {});
    for (let ordinal = 0; ordinal < 3; ordinal++) {
      event("session.reasoning.started", { ordinal });
      event("session.reasoning.ended", { ordinal, text: `Thinking ${ordinal}` });
    }
    event("session.text.started", { ordinal: 0 });
    event("session.text.delta", { ordinal: 0, delta: "One final answer" });
    event("session.text.ended", { ordinal: 0, text: "One final answer", state: { phase: "final_answer" } });
    event("session.step.ended", { finish: "stop" });
    const history = replayTranscript([{
      id: "a", type: "assistant", time: { completed: 100 }, content: [
        ...[0, 1, 2].map((ordinal) => ({ type: "reasoning", text: `Thinking ${ordinal}` })),
        { type: "text", text: "One final answer", state: { phase: "final_answer" } }
      ]
    }]);
    const reconciled = mergeChatHistory(history, projectAssistantItems(state, "s"));
    hydrateChatState(state, "s", reconciled);
    hydrateChatState(state, "s", history);
    event("session.text.ended", { ordinal: 0, text: "One final answer", state: { phase: "final_answer" } });
    const assistant = projectAssistantItems(state, "s")[0];
    expect(assistant.kind).toBe("assistant");
    if (assistant.kind !== "assistant") throw new Error("Expected assistant");
    expect(assistant.parts.filter((part) => part.kind === "text")).toEqual([
      { kind: "text", id: "a:text:0", text: "One final answer", complete: true }
    ]);
    expect(assistant.parts.filter((part) => part.kind === "reasoning")).toHaveLength(3);
  });

});
