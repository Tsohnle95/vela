import { describe, expect, it } from "vitest";
import { applyChatEvent, findTurnStartedAt, hydrateChatState, projectAssistantItems, type ChatDirectoryState, type ChatMessageRecord } from "./chat-store";
import type { ChatStreamEvent } from "./chat-stream";
import type { TranscriptItem } from "@shared/types";

function event(id: string, type: string, data: Record<string, unknown>): ChatStreamEvent {
  return { id, type, created: 100, data };
}

function state(): ChatDirectoryState {
  return { message: {}, part: {}, session_status: {} };
}

function record(id: string, role: string, created?: number): ChatMessageRecord {
  return { id, sessionID: "s", role, time: created === undefined ? {} : { created } };
}

describe("findTurnStartedAt", () => {
  it("returns the newest user message timestamp", () => {
    const messages = [record("u1", "user", 1000), record("a1", "assistant", 1100), record("u2", "user", 2000)];
    expect(findTurnStartedAt(messages)).toBe(2000);
  });

  it("falls back to the given assistant message when no user message exists", () => {
    const messages = [record("a1", "assistant", 1500)];
    expect(findTurnStartedAt(messages, "a1")).toBe(1500);
  });

  it("returns null when timestamps are missing or messages are absent", () => {
    expect(findTurnStartedAt(undefined)).toBeNull();
    expect(findTurnStartedAt([])).toBeNull();
    expect(findTurnStartedAt([record("u1", "user")])).toBeNull();
    expect(findTurnStartedAt([record("a1", "assistant")], "missing")).toBeNull();
  });
});

describe("applyChatEvent", () => {
  it("preserves the started model, agent, and timestamp through completion and hydration", () => {
    const draft = state();
    applyChatEvent(draft, "s", {
      ...event("start", "session.step.started", {
        sessionID: "s", assistantMessageID: "a", agent: "build",
        model: { id: "model-a", providerID: "provider-a" }, started: 900
      }),
      created: 1000
    });
    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      agent: "build", model: { id: "model-a", providerID: "provider-a" }, createdAt: 1000, completed: false
    });
    applyChatEvent(draft, "s", {
      ...event("end", "session.step.ended", { sessionID: "s", assistantMessageID: "a", finish: "stop" }),
      created: 4500
    });
    const projected = projectAssistantItems(draft, "s");
    expect(projected[0]).toMatchObject({
      agent: "build", model: { id: "model-a", providerID: "provider-a" },
      createdAt: 1000, completedAt: 4500, completed: true
    });
    const restored = state();
    hydrateChatState(restored, "s", projected);
    expect(projectAssistantItems(restored, "s")).toEqual(projected);
  });

  it("reopens a failed step without losing the original creation time", () => {
    const draft = state();
    applyChatEvent(draft, "s", {
      ...event("start", "session.step.started", {
        sessionID: "s", assistantMessageID: "a", agent: "build", model: { id: "m", providerID: "p" }, started: 900
      }), created: 1000
    });
    applyChatEvent(draft, "s", {
      ...event("failed", "session.step.failed", {
        sessionID: "s", assistantMessageID: "a", error: { type: "Interrupted", message: "Stopped" }
      }), created: 2000
    });
    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({ interrupted: true, completed: true });
    applyChatEvent(draft, "s", {
      ...event("restart", "session.step.started", {
        sessionID: "s", assistantMessageID: "a", agent: "plan", model: { id: "m2", providerID: "p" }, started: 2900
      }), created: 3000
    });
    const item = projectAssistantItems(draft, "s")[0];
    expect(item).toMatchObject({ createdAt: 1000, completed: false, agent: "plan", model: { id: "m2", providerID: "p" } });
    expect(item).not.toHaveProperty("interrupted");
    expect(item).not.toHaveProperty("error");
    expect(item).not.toHaveProperty("completedAt");
  });

  it("retains canonical legacy model, parent, and interruption metadata", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("first", "message.updated", {
      info: { id: "a", sessionID: "s", role: "assistant", time: { created: 1000 } }
    }));
    applyChatEvent(draft, "s", event("metadata", "message.updated", {
      info: {
        id: "a", sessionID: "s", role: "assistant", time: { created: 900 },
        modelID: "m", providerID: "p", parentID: "u"
      }
    }));
    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      model: { id: "m", providerID: "p" }, parentID: "u", createdAt: 900
    });
    applyChatEvent(draft, "s", event("abort", "message.updated", {
      info: {
        id: "a", sessionID: "s", role: "assistant", time: { created: 900, completed: 1500 },
        modelID: "m", providerID: "p", parentID: "u", error: { name: "MessageAbortedError", data: { message: "Stopped" } }
      }
    }));
    const projected = projectAssistantItems(draft, "s");
    expect(projected[0]).toMatchObject({ interrupted: true, completedAt: 1500 });
    const restored = state();
    hydrateChatState(restored, "s", projected);
    expect(projectAssistantItems(restored, "s")[0]).toMatchObject({ interrupted: true, completedAt: 1500 });
  });

  it("replaces hydrated message timestamps with canonical history without inventing missing duration", () => {
    const draft = state();
    hydrateChatState(draft, "s", [
      { kind: "user", id: "u", text: "hello", createdAt: 1000, agent: "build", model: { id: "m", providerID: "p" } },
      { kind: "assistant", id: "a", messageID: "a", parts: [], completed: true }
    ]);
    expect(draft.message.s.find((message) => message.id === "u")).toMatchObject({
      time: { created: 1000 }, model: { id: "m", providerID: "p" }
    });
    expect(projectAssistantItems(draft, "s")[0]).not.toHaveProperty("completedAt");
    hydrateChatState(draft, "s", [{
      kind: "assistant", id: "a", messageID: "a", parts: [], completed: true,
      createdAt: 1100, completedAt: 3500, agent: "plan", model: { id: "m", providerID: "p" }, parentID: "u"
    }]);
    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      createdAt: 1100, completedAt: 3500, agent: "plan", model: { id: "m", providerID: "p" }, parentID: "u"
    });
  });

  it("reports orphan materialization when a delta arrives before parts", () => {
    const result = applyChatEvent(state(), "s", event("d", "message.part.delta", {
      sessionID: "s", messageID: "msg_1", partID: "prt_1", field: "text", delta: "hello"
    }));

    expect(result).toEqual({
      changed: false,
      materialization: { type: "incomplete-session-snapshot", reason: "orphan-delta", sessionID: "s", messageID: "msg_1", partID: "prt_1" }
    });
  });

  it("reports missing-part materialization when the delta part is unknown", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("u", "message.part.updated", {
      sessionID: "s",
      part: { id: "prt_2", messageID: "msg_1", sessionID: "s", type: "text", text: "" }
    }));
    const result = applyChatEvent(draft, "s", event("d", "message.part.delta", {
      sessionID: "s", messageID: "msg_1", partID: "prt_1", field: "text", delta: "hello"
    }));

    expect(result).toEqual({
      changed: false,
      materialization: { type: "incomplete-session-snapshot", reason: "missing-delta-part", sessionID: "s", messageID: "msg_1", partID: "prt_1" }
    });
  });

  it("requests materialization when a part update has no owning message", () => {
    const draft = state();
    const result = applyChatEvent(draft, "s", event("u", "message.part.updated", {
      sessionID: "s",
      part: { id: "prt_1", messageID: "msg_1", sessionID: "s", type: "text", text: "hello" }
    }));

    expect(draft.part.msg_1.map((part) => part.id)).toEqual(["prt_1"]);
    expect(result).toEqual({
      changed: true,
      materialization: {
        type: "incomplete-session-snapshot",
        reason: "missing-owning-message",
        sessionID: "s",
        messageID: "msg_1",
        partID: "prt_1"
      }
    });
  });

  it("applies a part update without materialization when the owning message exists", () => {
    const draft = state();
    draft.message.s = [{ id: "msg_1", sessionID: "s", role: "assistant", time: { created: 1 } }];
    const result = applyChatEvent(draft, "s", event("u", "message.part.updated", {
      sessionID: "s",
      part: { id: "prt_1", messageID: "msg_1", sessionID: "s", type: "text", text: "hello" }
    }));

    expect(draft.part.msg_1.map((part) => part.id)).toEqual(["prt_1"]);
    expect(result).toBe(true);
  });

  it("skips duplicate session status events", () => {
    const draft = state();
    const busy = event("status", "session.status", { sessionID: "s", status: { type: "busy" } });

    expect(applyChatEvent(draft, "s", busy)).toBe(true);
    expect(applyChatEvent(draft, "s", busy)).toBe(false);
  });

  it("records an errored session status as not busy and stops treating the session as running", () => {
    const draft = state();
    expect(applyChatEvent(draft, "s", event("b", "session.status", { sessionID: "s", status: { type: "busy" } }))).toBe(true);
    expect(draft.session_status.s).toEqual({ type: "busy" });

    expect(applyChatEvent(draft, "s", event("e", "session.status", { sessionID: "s", status: { type: "error" } }))).toBe(true);
    expect(draft.session_status.s).toEqual({ type: "error" });

    const again = event("e2", "session.status", { sessionID: "s", status: { type: "error" } });
    expect(applyChatEvent(draft, "s", again)).toBe(false);
  });

  it("skips unchanged message updates", () => {
    const draft = state();
    draft.message.s = [{ id: "msg_1", sessionID: "s", role: "assistant", time: { created: 1, completed: 2 } }];
    const update = event("u", "message.updated", {
      info: { id: "msg_1", sessionID: "s", role: "assistant", time: { created: 1, completed: 2 } }
    });

    expect(applyChatEvent(draft, "s", update)).toBe(false);
  });

  it("streams session text deltas into an ordered part and dedupes the ended snapshot", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("t", "session.text.started", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0 }));
    applyChatEvent(draft, "s", event("d1", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "Hello" }));
    applyChatEvent(draft, "s", event("d2", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: " world" }));
    applyChatEvent(draft, "s", event("end", "session.text.ended", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, text: "Hello world" }));
    applyChatEvent(draft, "s", event("d3", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: " world" }));
    applyChatEvent(draft, "s", event("se", "session.step.ended", { sessionID: "s", assistantMessageID: "msg_1" }));

    const items = projectAssistantItems(draft, "s");

    expect(items).toHaveLength(1);
    expect(items[0]).toMatchObject({
      kind: "assistant",
      messageID: "msg_1",
      completed: true,
      parts: [{ kind: "text", id: "msg_1:text:0", text: "Hello world", complete: true }]
    });
  });

  it("projects each reasoning delta into the live Think text", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("r", "session.reasoning.started", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0 }));
    applyChatEvent(draft, "s", event("d1", "session.reasoning.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "Inspecting" }));
    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      completed: false,
      parts: [{ kind: "reasoning", text: "Inspecting", complete: false }]
    });

    applyChatEvent(draft, "s", event("d2", "session.reasoning.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: " the events" }));
    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ kind: "reasoning", text: "Inspecting the events", complete: false }]
    });
  });

  it("projects OpenCode commentary text as real Think progress", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("text", "session.text.ended", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      ordinal: 0,
      text: "Stage 1: Inspecting components",
      state: { phase: "commentary" }
    }));

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ kind: "reasoning", text: "Stage 1: Inspecting components", complete: true }]
    });
  });

  it("keeps final-answer text out of the Think line", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("text", "session.text.ended", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      ordinal: 0,
      text: "Final response",
      state: { phase: "final_answer" }
    }));

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ kind: "text", text: "Final response", complete: true }]
    });
  });

  it("uses step and final-message records as authoritative turn status", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    expect(draft.session_status.s).toEqual({ type: "busy" });

    applyChatEvent(draft, "s", event("done", "message.updated", {
      info: {
        id: "msg_1",
        sessionID: "s",
        role: "assistant",
        time: { created: 100, completed: 200 },
        finish: "stop"
      }
    }));
    expect(draft.session_status.s).toEqual({ type: "idle" });
  });

  it("keeps execution busy across tool steps until the execution terminal event", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("execution", "session.execution.started", { sessionID: "s" }));
    expect(draft.session_status.s).toEqual({ type: "busy" });

    applyChatEvent(draft, "s", event("step", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("tools", "session.step.ended", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      finish: "tool-calls"
    }));
    expect(draft.session_status.s).toEqual({ type: "busy" });

    applyChatEvent(draft, "s", event("done", "session.execution.succeeded", { sessionID: "s" }));
    expect(draft.session_status.s).toEqual({ type: "idle" });
  });

  it("clears a retry marker when an execution is interrupted", () => {
    const draft = state();
    draft.message.s = [{
      id: "msg_1",
      sessionID: "s",
      role: "assistant",
      time: { created: 1 },
      retry: { attempt: 2, message: "Rate limit exceeded", next: 2_000 }
    }];

    applyChatEvent(draft, "s", event("interrupted", "session.execution.interrupted", { sessionID: "s" }));

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({ completed: true });
    expect(projectAssistantItems(draft, "s")[0]).not.toHaveProperty("retry");
  });

  it("does not duplicate a delta already included in a message part snapshot", () => {
    const draft = state();
    draft.message.s = [{ id: "msg_1", sessionID: "s", role: "assistant", time: { created: 1 } }];
    applyChatEvent(draft, "s", event("u", "message.part.updated", {
      sessionID: "s",
      part: { id: "prt_1", messageID: "msg_1", sessionID: "s", type: "text", text: "Hello" }
    }));
    applyChatEvent(draft, "s", event("d", "message.part.delta", {
      sessionID: "s", messageID: "msg_1", partID: "prt_1", field: "text", delta: " world"
    }));
    applyChatEvent(draft, "s", event("u2", "message.part.updated", {
      sessionID: "s",
      part: { id: "prt_1", messageID: "msg_1", sessionID: "s", type: "text", text: "Hello world" }
    }));
    applyChatEvent(draft, "s", event("d2", "message.part.delta", {
      sessionID: "s", messageID: "msg_1", partID: "prt_1", field: "text", delta: " world"
    }));

    expect(draft.part.msg_1[0].text).toBe("Hello world");
  });

  it("streams tool input, applies the called snapshot, and finishes the card", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("t", "session.tool.input.started", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", name: "bash" }));
    applyChatEvent(draft, "s", event("d", "session.tool.input.delta", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", delta: "ls -la" }));
    applyChatEvent(draft, "s", event("c", "session.tool.called", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", input: { command: "ls -la" } }));
    applyChatEvent(draft, "s", event("ok", "session.tool.success", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", content: [{ type: "text", text: "done" }] }));

    const items = projectAssistantItems(draft, "s");

    expect(items[0]).toMatchObject({
      parts: [{
        kind: "tool",
        tool: { title: "bash", status: "success", output: "done", input: JSON.stringify({ command: "ls -la" }, null, 2) }
      }]
    });
  });

  it("projects failed tool snapshots as settled failures", () => {
    const draft = state();
    draft.message.s = [{ id: "msg_1", sessionID: "s", role: "assistant", time: { created: 1 } }];
    applyChatEvent(draft, "s", event("failed", "message.part.updated", {
      sessionID: "s",
      part: {
        id: "part_1",
        messageID: "msg_1",
        sessionID: "s",
        type: "tool",
        name: "read",
        callID: "call_1",
        state: { status: "failed", error: { message: "File not found" } }
      }
    }));

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ kind: "tool", tool: { status: "failed" } }]
    });
  });

  it("settles a failed step and records its session error state", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("failed", "session.step.failed", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      error: { message: "Provider failed" }
    }));

    expect(draft.session_status.s).toEqual({ type: "error" });
    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({ completed: true, error: "[VELA_STEP_FAILED] Provider failed" });
  });

  it("returns to retrying when a failed step schedules another attempt", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("failed", "session.step.failed", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      error: { message: "Provider failed" }
    }));
    applyChatEvent(draft, "s", event("retry", "session.retry.scheduled", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      attempt: 2,
      error: { message: "Retrying" },
      next: 200
    }));

    expect(draft.session_status.s).toEqual({
      type: "retry",
      attempt: 2,
      message: "[VELA_RETRY_SCHEDULED] Retrying",
      next: 200
    });
  });

  it("preserves a finished tool against a stale running snapshot", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("t", "session.tool.input.started", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1" }));
    applyChatEvent(draft, "s", event("ok", "session.tool.success", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", content: [{ type: "text", text: "done" }] }));
    applyChatEvent(draft, "s", event("c", "session.tool.called", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", input: { command: "ls" } }));

    const items = projectAssistantItems(draft, "s");

    expect(items[0]).toMatchObject({
      parts: [{ kind: "tool", tool: { status: "success", output: "done" } }]
    });
  });

  it("reports orphan materialization for legacy session text deltas", () => {
    const result = applyChatEvent(state(), "s", event("d", "session.text.delta", {
      sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "hi"
    }));

    expect(result).toEqual({
      changed: false,
      materialization: { type: "incomplete-session-snapshot", reason: "orphan-delta", sessionID: "s", messageID: "msg_1", partID: "msg_1:text:0" }
    });
  });

  it("keeps parts sorted by id through binary insertion", () => {
    const draft = state();
    draft.message.s = [{ id: "msg_1", sessionID: "s", role: "assistant", time: { created: 1 } }];
    for (const partID of ["prt_3", "prt_1", "prt_2"]) {
      applyChatEvent(draft, "s", event(partID, "message.part.updated", {
        sessionID: "s",
        part: { id: partID, messageID: "msg_1", sessionID: "s", type: "text", text: partID }
      }));
    }

    expect(draft.part.msg_1.map((part) => part.id)).toEqual(["prt_1", "prt_2", "prt_3"]);
  });

  it("completes only the latest incomplete assistant on execution success", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("a", "session.step.started", { sessionID: "s", assistantMessageID: "msg_a" }));
    applyChatEvent(draft, "s", event("b", "session.step.started", { sessionID: "s", assistantMessageID: "msg_b" }));
    applyChatEvent(draft, "s", event("x", "session.execution.succeeded", { sessionID: "s" }));

    const items = projectAssistantItems(draft, "s");

    expect(items[0]).toMatchObject({ messageID: "msg_a", completed: true });
    expect(items[1]).toMatchObject({ messageID: "msg_b", completed: true });
  });

  it("completes the trailing incomplete assistant when the session goes idle", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("u", "message.updated", {
      info: { id: "usr_1", sessionID: "s", role: "user", time: { created: 1 } }
    }));
    applyChatEvent(draft, "s", event("a", "message.updated", {
      info: { id: "msg_1", sessionID: "s", role: "assistant", time: { created: 2 } }
    }));

    const result = applyChatEvent(draft, "s", event("i", "session.idle", { sessionID: "s" }));
    const items = projectAssistantItems(draft, "s");

    expect(result).toBe(true);
    expect(items[0]).toMatchObject({ messageID: "msg_1", completed: true });
    expect(draft.session_status.s).toEqual({ type: "idle" });
  });

  it("completes the trailing incomplete assistant on session error", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("a", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("e", "session.error", { sessionID: "s", error: { message: "boom" } }));

    const items = projectAssistantItems(draft, "s");

    expect(items[0]).toMatchObject({ messageID: "msg_1", completed: true, error: "[VELA_SESSION_ERROR] boom" });
    expect(draft.session_status.s).toEqual({ type: "error" });
  });

  it("reports no change for a repeated idle with nothing left to complete", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("i1", "session.idle", { sessionID: "s" }));

    expect(applyChatEvent(draft, "s", event("i2", "session.idle", { sessionID: "s" }))).toBe(false);
  });

  it("hydrates history without regressing longer live text", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("t", "session.text.started", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0 }));
    applyChatEvent(draft, "s", event("d", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "Hello world" }));

    const history: TranscriptItem[] = [{
      kind: "assistant",
      id: "msg_1",
      messageID: "msg_1",
      parts: [{ kind: "text", id: "msg_1:text:0", text: "Hello", complete: false }],
      completed: false
    }];
    hydrateChatState(draft, "s", history);

    expect(draft.part.msg_1[0].text).toBe("Hello world");
  });

  it("projects assistant retry and error state", () => {
    const draft = state();
    draft.message.s = [{
      id: "msg_1",
      sessionID: "s",
      role: "assistant",
      time: { created: 1 },
      retry: { attempt: 2, message: "boom", next: 30_000 },
      error: { message: "Step failed" }
    }];

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      kind: "assistant",
      completed: false,
      retry: { attempt: 2, message: "[VELA_RETRY_SCHEDULED] boom", next: 30_000 },
      error: "[VELA_RUNTIME_FAILURE] Step failed"
    });
  });

  it("keeps distinct text parts and assistant messages even when their content repeats", () => {
    const draft = state();
    draft.message.s = [
      { id: "assistant-1", sessionID: "s", role: "assistant", time: { completed: 2 } },
      { id: "assistant-2", sessionID: "s", role: "assistant", time: { completed: 3 } }
    ];
    draft.part["assistant-1"] = [
      { id: "text-1", messageID: "assistant-1", type: "text", text: "Same response", time: { completed: 2 } },
      { id: "text-2", messageID: "assistant-1", type: "text", text: "Same response", time: { completed: 2 } }
    ];
    draft.part["assistant-2"] = [
      { id: "text-3", messageID: "assistant-2", type: "text", text: "Same response", time: { completed: 3 } }
    ];

    expect(projectAssistantItems(draft, "s")).toMatchObject([
      { id: "assistant-1", parts: [{ id: "text-1" }, { id: "text-2" }] },
      { id: "assistant-2", parts: [{ id: "text-3" }] }
    ]);
  });

  it("projects canonical OpenCode parts in ID order when snapshots arrive out of order", () => {
    const draft = state();
    draft.message.s = [{ id: "msg_1", sessionID: "s", role: "assistant", time: { completed: 3 } }];
    for (const [id, text] of [["prt_003", "Third"], ["prt_002", "Second"], ["prt_001", "First"]]) {
      applyChatEvent(draft, "s", event(id, "message.part.updated", {
        sessionID: "s", part: { id, messageID: "msg_1", sessionID: "s", type: "text", text }
      }));
    }

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ id: "prt_001", text: "First" }, { id: "prt_002", text: "Second" }, { id: "prt_003", text: "Third" }]
    });
  });
});

describe("tool part id aliasing", () => {
  const seedLiveToolRow = (draft: ChatDirectoryState): void => {
    applyChatEvent(draft, "s", event("t1", "session.tool.input.started", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      id: "call_1",
      name: "bash"
    }));
    applyChatEvent(draft, "s", event("t2", "session.tool.called", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      callID: "call_1",
      name: "bash",
      input: { command: "echo hi" }
    }));
  };

  it("merges a server tool snapshot onto the live synthetic-id row", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("m1", "message.updated", {
      sessionID: "s",
      info: { id: "msg_1", sessionID: "s", role: "assistant", time: { created: 100 } }
    }));
    seedLiveToolRow(draft);

    applyChatEvent(draft, "s", event("u", "message.part.updated", {
      sessionID: "s",
      part: { id: "call_1", messageID: "msg_1", sessionID: "s", type: "tool", state: { status: "completed" } }
    }));

    const parts = draft.part.msg_1;
    expect(parts).toHaveLength(1);
    expect(parts[0].id).toBe("msg_1:tool:call_1");
    expect(String(parts[0].state?.status)).toBe("completed");
  });

  it("merges a server part id onto its call id without losing the live tool name", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("m1", "message.updated", {
      sessionID: "s",
      info: { id: "msg_1", sessionID: "s", role: "assistant", time: { created: 100 } }
    }));
    seedLiveToolRow(draft);

    applyChatEvent(draft, "s", event("u", "message.part.updated", {
      sessionID: "s",
      part: {
        id: "part_1",
        callID: "call_1",
        messageID: "msg_1",
        sessionID: "s",
        type: "tool",
        state: { status: "completed", output: "done" }
      }
    }));

    expect(draft.part.msg_1).toHaveLength(1);
    expect(draft.part.msg_1[0]).toMatchObject({
      id: "msg_1:tool:call_1",
      name: "bash",
      callID: "call_1",
      state: { status: "completed", input: { command: "echo hi" }, output: "done" }
    });
  });

  it("projects one enriched tool when stale state still contains duplicate call ids", () => {
    const draft = state();
    draft.message.s = [{ id: "msg_1", sessionID: "s", role: "assistant", time: { created: 1, completed: 2 } }];
    draft.part.msg_1 = [
      {
        id: "msg_1:tool:call_1",
        messageID: "msg_1",
        sessionID: "s",
        type: "tool",
        name: "grep",
        callID: "call_1",
        state: { status: "running", input: { pattern: "needle" } }
      },
      {
        id: "part_1",
        messageID: "msg_1",
        sessionID: "s",
        type: "tool",
        callID: "call_1",
        state: { status: "completed", output: "match" }
      }
    ];

    const item = projectAssistantItems(draft, "s")[0];
    expect(item.kind).toBe("assistant");
    if (item.kind !== "assistant") return;
    expect(item.parts).toHaveLength(1);
    expect(item.parts[0]).toMatchObject({
      kind: "tool",
      tool: { id: "call_1", title: "grep", status: "success", inputValue: { pattern: "needle" }, output: "match" }
    });
  });

  it("applies server deltas addressed by raw call id to the live row", () => {
    const draft = state();
    seedLiveToolRow(draft);

    const result = applyChatEvent(draft, "s", event("d", "message.part.delta", {
      sessionID: "s", messageID: "msg_1", partID: "call_1", field: "input", delta: "{\"command\":"
    }));

    expect(result).toBe(true);
    expect(draft.part.msg_1).toHaveLength(1);
  });

  it("hydrates history tool parts under canonical ids so live events merge", () => {
    const draft = state();
    const transcript: TranscriptItem[] = [{
      kind: "assistant",
      id: "msg_1",
      messageID: "msg_1",
      completed: true,
      parts: [{
        kind: "tool",
        id: "call_1",
        tool: { id: "call_1", title: "bash", detail: "", status: "success", input: "{}" }
      }]
    }];
    hydrateChatState(draft, "s", transcript);
    expect(draft.part.msg_1.map((part) => part.id)).toEqual(["msg_1:tool:call_1"]);

    applyChatEvent(draft, "s", event("t3", "session.tool.success", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      callID: "call_1",
      output: "done"
    }));

    expect(draft.part.msg_1).toHaveLength(1);
    expect(String(draft.part.msg_1[0].state?.output)).toBe("done");
  });

  it("enriches a finished generic live tool with its authoritative history name", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("input", "session.tool.input.started", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1" }));
    applyChatEvent(draft, "s", event("done", "session.tool.success", {
      sessionID: "s",
      assistantMessageID: "msg_1",
      callID: "call_1",
      output: "file contents"
    }));

    hydrateChatState(draft, "s", [{
      kind: "assistant",
      id: "msg_1",
      messageID: "msg_1",
      completed: true,
      parts: [{
        kind: "tool",
        id: "call_1",
        tool: {
          id: "call_1",
          title: "read",
          detail: "README.md",
          status: "success",
          input: JSON.stringify({ path: "/repo/README.md" }),
          inputValue: { path: "/repo/README.md" },
          output: "file contents"
        }
      }]
    }]);

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ kind: "tool", tool: { title: "read", status: "success", output: "file contents" } }]
    });
  });
});

describe("OpenCode TUI streaming parity", () => {
  it("concatenates deltas verbatim without stripping repeated characters", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("t", "session.text.started", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0 }));
    applyChatEvent(draft, "s", event("d1", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "Paragraph one.\n" }));
    applyChatEvent(draft, "s", event("d2", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "\nParagraph two." }));

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ kind: "text", text: "Paragraph one.\n\nParagraph two." }]
    });
  });

  it("keeps trailing whitespace from a delta instead of eating it as an overlap", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("t", "session.text.started", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0 }));
    applyChatEvent(draft, "s", event("d1", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "word" }));
    applyChatEvent(draft, "s", event("d2", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", ordinal: 0, delta: "d and more" }));

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [{ kind: "text", text: "wordd and more" }]
    });
  });

  it("resolves ordinal-less text segments by arrival like the TUI", () => {
    const draft = state();
    applyChatEvent(draft, "s", event("start", "session.step.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("t0", "session.text.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("d0a", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", delta: "First" }));
    applyChatEvent(draft, "s", event("e0", "session.text.ended", { sessionID: "s", assistantMessageID: "msg_1", text: "First segment." }));
    applyChatEvent(draft, "s", event("tool", "session.tool.called", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", name: "read", input: { path: "README.md" } }));
    applyChatEvent(draft, "s", event("tool-done", "session.tool.success", { sessionID: "s", assistantMessageID: "msg_1", callID: "call_1", output: "contents" }));
    applyChatEvent(draft, "s", event("t1", "session.text.started", { sessionID: "s", assistantMessageID: "msg_1" }));
    applyChatEvent(draft, "s", event("d1a", "session.text.delta", { sessionID: "s", assistantMessageID: "msg_1", delta: "Second" }));
    applyChatEvent(draft, "s", event("e1", "session.text.ended", { sessionID: "s", assistantMessageID: "msg_1", text: "Second segment." }));

    expect(projectAssistantItems(draft, "s")[0]).toMatchObject({
      parts: [
        { kind: "text", id: "msg_1:text:0", text: "First segment." },
        { kind: "tool", tool: { title: "read", status: "success" } },
        { kind: "text", id: "msg_1:text:1", text: "Second segment." }
      ]
    });
  });
});
