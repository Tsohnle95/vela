import { describe, expect, it } from "vitest";
import {
  applyChatEvent,
  attachRetryToLatestAssistant,
  completeLatestIncomplete,
  hydrateChatState,
  projectAssistantItems,
  reconcileAssistantProjection,
  type ChatDirectoryState
} from "./chat-store";
import type { TranscriptItem } from "@shared/types";

type Assistant = Extract<TranscriptItem, { kind: "assistant" }>;

function send(draft: ChatDirectoryState, type: string, data: Record<string, unknown>, created = 100): void {
  applyChatEvent(draft, "s", { id: type, type, created, data: { sessionID: "s", ...data } });
}

function start(sessionID = "s"): ChatDirectoryState {
  const draft: ChatDirectoryState = { message: {}, part: {}, session_status: {} };
  send(draft, "session.step.started", { sessionID, assistantMessageID: "a" });
  return draft;
}

function assistant(draft: ChatDirectoryState, sessionID = "s"): Assistant {
  const item = projectAssistantItems(draft, sessionID)[0];
  if (item?.kind !== "assistant") throw new Error("Expected an assistant projection");
  return item;
}

describe("assistant projection characterization", () => {
  it("reuses unchanged rows and invalidates only the changed assistant across sessions", () => {
    const draft = start();
    send(draft, "session.text.ended", { assistantMessageID: "a", text: "Historical" });
    send(draft, "session.step.ended", { assistantMessageID: "a", finish: "stop" });
    send(draft, "session.step.started", { assistantMessageID: "b" });
    send(draft, "session.text.started", { assistantMessageID: "b" });
    send(draft, "session.step.started", { sessionID: "other", assistantMessageID: "c" });
    const [history, tail] = projectAssistantItems(draft, "s");
    const other = assistant(draft, "other");
    expect(projectAssistantItems(draft, "s")).toEqual([history, tail]);
    expect(assistant(draft)).toBe(history);
    expect(projectAssistantItems(draft, "s")[1]).toBe(tail);

    send(draft, "session.text.delta", { assistantMessageID: "b", delta: "New text" });
    const updated = projectAssistantItems(draft, "s");
    expect(updated[0]).toBe(history);
    expect(updated[1]).not.toBe(tail);
    expect(updated[1]).toMatchObject({ parts: [{ text: "New text" }] });
    expect(assistant(draft, "other")).toBe(other);
  });

  it("updates part-only deltas without changing an earlier projection or owning message", () => {
    const draft = start();
    send(draft, "session.text.started", { assistantMessageID: "a", ordinal: 0 });
    send(draft, "session.reasoning.started", { assistantMessageID: "a", ordinal: 0 });
    const messages = draft.message.s;
    const before = assistant(draft);

    send(draft, "session.text.delta", { assistantMessageID: "a", ordinal: 0, delta: "Visible text" });
    send(draft, "session.reasoning.delta", { assistantMessageID: "a", ordinal: 0, delta: "Reasoning" });

    expect(draft.message.s).toBe(messages);
    expect(assistant(draft).parts).toMatchObject([
      { kind: "text", text: "Visible text" },
      { kind: "reasoning", text: "Reasoning" }
    ]);
    expect(before.parts).toMatchObject([{ text: "" }, { text: "" }]);
  });

  it("updates message metadata, retry and completion while the parts remain unchanged", () => {
    const draft = start();
    send(draft, "session.text.ended", { assistantMessageID: "a", text: "Response" });
    const parts = draft.part.a;
    const before = assistant(draft);
    send(draft, "message.updated", {
      info: {
        id: "a", sessionID: "s", role: "assistant", parentID: "u", agent: "plan",
        model: { id: "model", providerID: "provider" }, time: { created: 90 }
      }
    });
    expect(assistant(draft)).toMatchObject({ parentID: "u", agent: "plan", createdAt: 90 });
    attachRetryToLatestAssistant(draft, "s", { attempt: 2, message: "Try again", next: 500 });
    expect(assistant(draft).retry).toMatchObject({ attempt: 2, next: 500 });
    completeLatestIncomplete(draft, "s");

    expect(draft.part.a).toBe(parts);
    expect(assistant(draft).completed).toBe(true);
    expect(assistant(draft).retry).toBeUndefined();
    expect(before).toMatchObject({ completed: false, createdAt: 100 });
    expect(before.parentID).toBeUndefined();
  });

  it("reconciles canonical text snapshots, trailing overlap and subsequent deltas", () => {
    const draft = start();
    send(draft, "message.part.updated", {
      part: { id: "prt_text", messageID: "a", sessionID: "s", type: "text", text: "hello" }
    });
    const before = assistant(draft);
    send(draft, "message.part.updated", {
      part: { id: "prt_text", messageID: "a", sessionID: "s", type: "text", text: "hello world" }
    });
    send(draft, "message.part.delta", { messageID: "a", partID: "prt_text", field: "text", delta: " world" });
    expect(assistant(draft).parts).toMatchObject([{ text: "hello world" }]);
    send(draft, "message.part.delta", { messageID: "a", partID: "prt_text", field: "text", delta: "!" });
    expect(assistant(draft).parts).toMatchObject([{ text: "hello world!" }]);
    expect(before.parts).toMatchObject([{ text: "hello" }]);
  });

  it("updates tool progress and completion without allowing a late running snapshot to regress it", () => {
    const draft = start();
    send(draft, "session.tool.input.started", { assistantMessageID: "a", callID: "c", name: "bash" });
    const before = assistant(draft);
    send(draft, "session.tool.called", { assistantMessageID: "a", callID: "c", input: { command: "pwd" } });
    send(draft, "session.tool.progress", { assistantMessageID: "a", callID: "c", progress: "Running" });
    expect(assistant(draft).parts).toMatchObject([{ tool: { status: "running", progress: "Running" } }]);
    send(draft, "session.tool.success", {
      assistantMessageID: "a", callID: "c", content: [{ type: "text", text: "/workspace" }]
    }, 200);
    send(draft, "message.part.updated", {
      part: { id: "a:tool:c", messageID: "a", sessionID: "s", type: "tool", state: { status: "running" } }
    });
    expect(assistant(draft).parts).toMatchObject([{ tool: { status: "success", output: "/workspace" } }]);
    expect(before.parts).toMatchObject([{ tool: { status: "running" } }]);
  });

  it("hydrates metadata and completion while retaining longer live text", () => {
    const draft = start();
    send(draft, "session.text.started", { assistantMessageID: "a", ordinal: 0 });
    send(draft, "session.text.delta", { assistantMessageID: "a", ordinal: 0, delta: "Longer live response" });
    const before = assistant(draft);
    hydrateChatState(draft, "s", [{
      kind: "assistant", id: "a", messageID: "a", completed: true, agent: "plan", createdAt: 90,
      parts: [{ kind: "text", id: "a:text:0", text: "Longer", complete: true }]
    }]);
    expect(assistant(draft)).toMatchObject({
      completed: true, agent: "plan", createdAt: 90,
      parts: [{ text: "Longer live response", complete: true }]
    });
    expect(before).toMatchObject({ completed: false, createdAt: 100, parts: [{ complete: false }] });
  });

  it("invalidates failures without mutating a retained live projection", () => {
    const draft = start();
    send(draft, "session.text.ended", { assistantMessageID: "a", text: "Partial" });
    const before = assistant(draft);
    send(draft, "session.step.failed", { assistantMessageID: "a", error: { message: "Provider failed" } }, 200);
    const failed = assistant(draft);
    expect(failed).not.toBe(before);
    expect(failed).toMatchObject({ completed: true, error: "[VELA_STEP_FAILED] Provider failed" });
    expect(before).toMatchObject({ completed: false, parts: [{ text: "Partial" }] });
    expect(before.error).toBeUndefined();
  });

  it("handles missing parts, removal and reinsertion without retaining superseded values", () => {
    const draft = start();
    expect(assistant(draft).parts).toEqual([]);
    send(draft, "message.part.updated", {
      part: { id: "prt_text", messageID: "a", sessionID: "s", type: "text", text: "First" }
    });
    expect(assistant(draft).parts).toMatchObject([{ text: "First" }]);
    send(draft, "message.part.removed", { messageID: "a", partID: "prt_text" });
    expect(assistant(draft).parts).toEqual([]);
    send(draft, "message.removed", { messageID: "a" });
    expect(projectAssistantItems(draft, "s")).toEqual([]);
    send(draft, "session.step.started", { assistantMessageID: "a", agent: "plan" });
    send(draft, "message.part.updated", {
      part: { id: "prt_text", messageID: "a", sessionID: "s", type: "text", text: "Replacement" }
    });
    expect(assistant(draft)).toMatchObject({ agent: "plan", parts: [{ text: "Replacement" }] });
  });

  it("orders canonical parts by ID and synthetic stream parts by arrival", () => {
    const canonical = start();
    for (const [id, text] of [["prt_z", "Second"], ["prt_a", "First"]]) {
      send(canonical, "message.part.updated", { part: { id, messageID: "a", sessionID: "s", type: "text", text } });
      assistant(canonical);
    }
    expect(assistant(canonical).parts).toMatchObject([{ text: "First" }, { text: "Second" }]);
    const synthetic = start();
    send(synthetic, "session.reasoning.ended", { assistantMessageID: "a", text: "First" });
    assistant(synthetic);
    send(synthetic, "session.text.ended", { assistantMessageID: "a", text: "Second" });
    expect(assistant(synthetic).parts).toMatchObject([{ kind: "reasoning", text: "First" }, { kind: "text", text: "Second" }]);
  });

  it("keeps independent states with matching message and part IDs isolated", () => {
    const first = start();
    const second = start();
    for (const [draft, text] of [[first, "First workspace"], [second, "Second workspace"]] as const) {
      send(draft, "session.text.ended", { assistantMessageID: "a", text });
    }
    expect(assistant(first).parts).toMatchObject([{ text: "First workspace" }]);
    expect(assistant(second).parts).toMatchObject([{ text: "Second workspace" }]);
    expect(assistant(first).parts).toMatchObject([{ text: "First workspace" }]);
  });
});

describe("transcript reconciliation characterization", () => {
  const response = (id: string, fields: Partial<Assistant> = {}): Assistant => ({
    kind: "assistant", id, messageID: id, parts: [], completed: false, ...fields
  });

  it("clones users before canonical metadata and responding agent/model backfill", () => {
    const user: TranscriptItem = { kind: "user", id: "u", text: "Prompt", agent: "old" };
    const auxiliary: TranscriptItem = { kind: "status", id: "notice", text: "Retrying", tone: "info" };
    const model = { id: "model", providerID: "provider" };
    const projected = response("a", { agent: "plan", model });
    const result = reconcileAssistantProjection([user, auxiliary], [projected], [{
      id: "u", sessionID: "s", role: "user", time: { created: 90 }, agent: "canonical"
    }]);

    expect(result[0]).toMatchObject({ agent: "plan", model, createdAt: 90 });
    expect(result[0]).not.toBe(user);
    expect(user).toEqual({ kind: "user", id: "u", text: "Prompt", agent: "old" });
    expect(result[1]).toBe(auxiliary);
    expect(result[2]).toMatchObject({ parentID: "u" });
    expect(projected.parentID).toBeUndefined();
  });

  it("retains auxiliary chronology, replaces/removes old assistants and respects input ownership boundaries", () => {
    const user: TranscriptItem = { kind: "user", id: "u", text: "Prompt" };
    const synthetic: TranscriptItem = { kind: "synthetic", id: "synthetic", text: "Continue", description: "Follow-up" };
    const blankSynthetic: TranscriptItem = { kind: "synthetic", id: "blank", text: "Context", description: "  " };
    const shell: TranscriptItem = { kind: "shell", id: "shell", shellID: "sh", command: "pwd", status: "exited" };
    const projected = [response("a"), response("b"), response("c"), response("d", { parentID: "u" }), response("new")];
    const result = reconcileAssistantProjection([
      user, response("gone"), response("a"), synthetic, response("b"), blankSynthetic,
      shell, response("c"), response("d")
    ], projected, []);

    expect(result.map((item) => item.id)).toEqual(["u", "a", "synthetic", "b", "blank", "shell", "c", "d", "new"]);
    expect(result[1]).toMatchObject({ parentID: "u" });
    expect(result[3]).toMatchObject({ parentID: "synthetic" });
    expect(result[6]).toBe(projected[2]);
    expect(result[7]).toBe(projected[3]);
    expect(result[8]).toBe(projected[4]);
    expect(projected[0].parentID).toBeUndefined();
    expect(projected[1].parentID).toBeUndefined();
    expect(reconcileAssistantProjection([user, synthetic, blankSynthetic], [response("after-blank")], []).at(-1))
      .toMatchObject({ parentID: "synthetic" });
  });
});
