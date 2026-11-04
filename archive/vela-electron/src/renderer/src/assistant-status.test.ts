import { describe, expect, it } from "vitest";
import {
  createParsedStatus,
  decodeParsedStatus,
  encodeParsedStatus,
  getActiveAssistantContext,
  resolveAssistantStatus
} from "./assistant-status";
import type { ChatMessageRecord, ChatPartRecord } from "./chat-store";
import { IDLE_ACTIVITY } from "./session-activity";

function text(id: string, content: string, complete = false): ChatPartRecord {
  return { id, messageID: "msg_1", sessionID: "s", type: "text", text: content, time: complete ? { end: 1 } : {} };
}

function reasoning(id: string, complete = false): ChatPartRecord {
  return { id, messageID: "msg_1", sessionID: "s", type: "reasoning", text: "", time: complete ? { end: 1 } : {} };
}

function tool(id: string, name: string, status: "running" | "completed" | "pending" = "running"): ChatPartRecord {
  return { id, messageID: "msg_1", sessionID: "s", type: "tool", name, state: { status } };
}

function syntheticText(id: string, content: string): ChatPartRecord {
  return { id, messageID: "msg_1", sessionID: "s", type: "text", text: content, synthetic: true };
}

const BUSY = { phase: "busy" as const, isWorking: true, isBusy: true, isCooldown: false };

describe("assistant status", () => {
  it("reports composing while text streams", () => {
    expect(createParsedStatus([text("p1", "Hello", false)], "a1")).toEqual({
      activePartType: "text",
      activeToolName: undefined,
      statusText: "composing",
      isGenericStatus: false
    });
  });

  it("reports thinking while reasoning streams", () => {
    expect(createParsedStatus([reasoning("p1", false)], "a1")).toEqual({
      activePartType: "reasoning",
      activeToolName: undefined,
      statusText: "thinking",
      isGenericStatus: false
    });
  });

  it("reports the running tool phrase", () => {
    expect(createParsedStatus([tool("p1", "bash")], "a1")).toEqual({
      activePartType: "tool",
      activeToolName: "bash",
      statusText: "running command",
      isGenericStatus: false
    });
  });

  it("classifies editing tools separately", () => {
    expect(createParsedStatus([tool("p1", "edit")], "a1")).toEqual({
      activePartType: "editing",
      activeToolName: "edit",
      statusText: "editing file",
      isGenericStatus: false
    });
  });

  it("ignores fully synthetic messages", () => {
    expect(createParsedStatus([syntheticText("p1", "context")], "a1")).toEqual({
      activePartType: undefined,
      activeToolName: undefined,
      statusText: expect.any(String),
      isGenericStatus: true
    });
  });

  it("falls back to a stable generic phrase when nothing is active", () => {
    const first = createParsedStatus([text("p1", "done", true)], "a1");
    const second = createParsedStatus([text("p1", "done", true)], "a1");

    expect(first.isGenericStatus).toBe(true);
    expect(first.statusText).toBe(second.statusText);
  });

  it("prefers the last running part", () => {
    expect(createParsedStatus([tool("p1", "bash", "completed"), text("p2", "x", false)], "a1").activePartType).toBe("text");
    expect(createParsedStatus([text("p2", "x", true), tool("p1", "bash")], "a1").activePartType).toBe("tool");
  });

  it("round-trips the parsed status signature", () => {
    const parsed = createParsedStatus([tool("p1", "bash")], "a1");
    expect(decodeParsedStatus(encodeParsedStatus(parsed))).toEqual(parsed);
  });

  it("resolves the active assistant model from the parent user message", () => {
    const messages: ChatMessageRecord[] = [
      { id: "u1", sessionID: "s", role: "user", time: { created: 1 }, model: { providerID: "openai", modelID: "gpt-5" } },
      { id: "a1", sessionID: "s", role: "assistant", time: { created: 2 }, parentID: "u1" }
    ];

    expect(getActiveAssistantContext(messages)).toEqual({
      assistantId: "a1",
      model: { providerId: "openai", modelId: "gpt-5" }
    });
  });

  it("returns a null model without a parent user message", () => {
    expect(getActiveAssistantContext([
      { id: "a1", sessionID: "s", role: "assistant", time: { created: 2 } }
    ])).toEqual({ assistantId: "a1", model: null });
  });

  it("builds a streaming working summary from busy activity", () => {
    const snapshot = resolveAssistantStatus({
      assistantId: "a1",
      model: { providerId: "openai", modelId: "gpt-5" },
      activity: BUSY,
      parts: [text("p1", "Hello", false)],
      pendingPermissions: 0,
      abortFlag: null,
      retryInfo: null
    });

    expect(snapshot).toMatchObject({
      activeModel: { providerId: "openai", modelId: "gpt-5" },
      forming: { isActive: true, characterCount: 0 },
      working: {
        activity: "streaming",
        isWorking: true,
        isStreaming: true,
        lifecyclePhase: "streaming",
        statusText: "composing",
        canAbort: true,
        activePartType: "text"
      }
    });
  });

  it("builds a tooling summary and a retry summary", () => {
    expect(resolveAssistantStatus({
      assistantId: "a1",
      model: null,
      activity: BUSY,
      parts: [tool("p1", "bash")],
      pendingPermissions: 0,
      abortFlag: null,
      retryInfo: null
    })).toMatchObject({ working: { activity: "tooling", statusText: "running command" } });

    expect(resolveAssistantStatus({
      assistantId: "a1",
      model: null,
      activity: { phase: "retry", isWorking: true, isBusy: false, isCooldown: false },
      parts: [],
      pendingPermissions: 0,
      abortFlag: null,
      retryInfo: { attempt: 2, next: 5_000 }
    })).toMatchObject({ working: { retryInfo: { attempt: 2, next: 5_000 } } });
  });

  it("reports waiting for permission", () => {
    expect(resolveAssistantStatus({
      assistantId: "a1",
      model: null,
      activity: IDLE_ACTIVITY,
      parts: [],
      pendingPermissions: 1,
      abortFlag: null,
      retryInfo: null
    })).toMatchObject({ working: { statusText: "waiting for permission", isWaitingForPermission: true, canAbort: false } });
  });

  it("prefers the question overlay when a question is pending", () => {
    expect(resolveAssistantStatus({
      assistantId: "a1",
      model: null,
      activity: BUSY,
      parts: [text("p1", "x", false)],
      pendingPermissions: 1,
      pendingQuestions: 1,
      abortFlag: null,
      retryInfo: null
    })).toMatchObject({ working: { statusText: null, isWorking: false, canAbort: false } });
  });

  it("reports an unacknowledged abort", () => {
    const snapshot = resolveAssistantStatus({
      assistantId: "a1",
      model: null,
      activity: BUSY,
      parts: [],
      pendingPermissions: 0,
      abortFlag: { timestamp: 1_000, acknowledged: false },
      retryInfo: null
    });

    expect(snapshot).toMatchObject({
      working: { wasAborted: true, abortActive: true, activity: "idle", isWorking: false, canAbort: false }
    });
  });

  it("returns null without an assistant", () => {
    expect(resolveAssistantStatus({
      assistantId: null,
      model: null,
      activity: IDLE_ACTIVITY,
      parts: [],
      pendingPermissions: 0,
      abortFlag: null,
      retryInfo: null
    })).toBeNull();
  });
});
