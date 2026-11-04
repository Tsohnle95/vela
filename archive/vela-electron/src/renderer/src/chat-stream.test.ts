import { describe, expect, it } from "vitest";
import type { TranscriptItem } from "@shared/types";
import {
  hasCompletedPromptResponse,
  mergeChatHistory,
  partFromProjection,
  reconcilePromptHistory,
  reduceChatStream,
  type ChatStreamEvent
} from "./chat-stream";

function event(id: string, type: string, data: Record<string, unknown>): ChatStreamEvent {
  return { id, type, created: 100, data };
}

describe("chat stream auxiliary items", () => {
  it("promotes the canonical input timestamp while retaining the optimistic selection", () => {
    const optimistic: TranscriptItem = {
      kind: "user", id: "user-90", text: "hello", createdAt: 90, agent: "build", model: { id: "m", providerID: "p" }
    };
    const enqueued = reduceChatStream([optimistic], event("e", "session.inbox.enqueued", {
      inboxID: "u", item: { type: "user", payload: { text: "hello" } }
    }));
    const delivered = reduceChatStream(enqueued, event("d", "session.inbox.delivered", { inboxID: "u" }));
    expect(delivered).toEqual([{
      kind: "user", id: "u", text: "hello", createdAt: 100, agent: "build", model: { id: "m", providerID: "p" }
    }]);
    expect(mergeChatHistory([{ kind: "user", id: "u", text: "hello", createdAt: 95 }], delivered)).toEqual([{
      kind: "user", id: "u", text: "hello", createdAt: 95, agent: "build", model: { id: "m", providerID: "p" }
    }]);
  });

  it("preserves complete live shell output", () => {
    const output = "start\n" + "x".repeat(32 * 1024) + "\nend";
    const transcript = reduceChatStream([], event("shell-end", "session.shell.ended", {
      shell: { id: "shell-1", command: "build", status: "exited", exit: 0 },
      output: { output }
    }));

    expect(transcript[0]).toMatchObject({ kind: "shell", output });
  });

  it("preserves complete streamed tool output across completion", () => {
    const output = "start\n" + "x".repeat(32 * 1024) + "\nend";
    for (const status of ["running", "completed"]) {
      expect(partFromProjection({
        id: "tool-1", type: "tool", tool: "bash", state: { status, output }
      }, 100)).toMatchObject({ kind: "tool", tool: { output } });
    }
  });

  it("preserves live semantic chronology around replayed assistants", () => {
    const assistant = (id: string): TranscriptItem => ({
      kind: "assistant", id, messageID: id, parts: [], completed: true
    });
    const history = [assistant("a1"), assistant("a2")];
    const live: TranscriptItem[] = [
      { kind: "shell", id: "shell", shellID: "shell", command: "pwd", status: "exited" },
      assistant("a1"),
      { kind: "skill", id: "skill", skill: "review", name: "Review", text: "" },
      assistant("a2"),
      { kind: "compaction", id: "compact", status: "completed", reason: "auto", summary: "done" }
    ];

    expect(mergeChatHistory(history, live).map((item) => item.id)).toEqual([
      "shell", "a1", "skill", "a2", "compact"
    ]);
  });

  it("keeps subagent synthetic completion entries with their description", () => {
    const transcript = reduceChatStream([], event("synthetic", "session.synthetic", {
      sessionID: "session-parent",
      text: '<subagent id="session-child" state="completed" description="Run the tests">\n</subagent>',
      description: "Run the tests"
    }));

    expect(transcript[0]).toEqual({
      kind: "synthetic",
      id: "synthetic",
      text: '<subagent id="session-child" state="completed" description="Run the tests">\n</subagent>',
      description: "Run the tests"
    });
  });

  it("preserves a subagent synthetic entry when history is merged over the live stream", () => {
    const history: TranscriptItem[] = [{
      kind: "synthetic",
      id: "synthetic-1",
      text: '<subagent id="session-child" state="completed" description="Run the tests">\n</subagent>',
      description: "Run the tests"
    }];
    const live: TranscriptItem[] = [];

    expect(mergeChatHistory(history, live)).toEqual(history);
  });

  it("keeps distinct completed assistant messages with the same response", () => {
    const response = (id: string): TranscriptItem => ({
      kind: "assistant",
      id,
      messageID: id,
      completed: true,
      parts: [{ kind: "text", id: `${id}:text`, text: "Same response", complete: true }]
    });

    expect(mergeChatHistory([], [response("assistant-1"), response("assistant-2")])).toEqual([
      response("assistant-1"), response("assistant-2")
    ]);
  });

  it("reconciles a completed canonical prompt without losing live tool detail", () => {
    const optimistic: TranscriptItem = { kind: "user", id: "user-100", text: "inspect streaming" };
    const tool: TranscriptItem = {
      kind: "assistant",
      id: "assistant-1",
      messageID: "assistant-1",
      completed: false,
      parts: [{
        kind: "tool",
        id: "tool-1",
        tool: { id: "tool-1", title: "read", detail: "/repo/src/store.tsx", status: "success", input: "{}" }
      }]
    };
    const history: TranscriptItem[] = [
      { kind: "user", id: "remote-user-1", text: "inspect streaming" },
      {
        kind: "assistant",
        id: "assistant-1",
        messageID: "assistant-1",
        completed: true,
        parts: [{ kind: "text", id: "text-1", text: "Done", complete: true }]
      }
    ];

    const merged = reconcilePromptHistory(history, [optimistic, tool], optimistic);
    expect(merged.filter((item) => item.kind === "user")).toEqual([history[0]]);
    expect(merged[1]).toMatchObject({
      kind: "assistant",
      completed: true,
      parts: [
        { kind: "text", text: "Done" },
        { kind: "tool", tool: { title: "read", detail: "/repo/src/store.tsx" } }
      ]
    });
    expect(hasCompletedPromptResponse(history, "inspect streaming", 0)).toBe(true);
    expect(hasCompletedPromptResponse(history, "inspect streaming", 1)).toBe(false);
  });
});
