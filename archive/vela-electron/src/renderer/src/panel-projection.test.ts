import { describe, expect, it } from "vitest";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import { projectPanelViews, type PanelProjectionSources } from "./panel-projection";
import { emptyMessageQueueState, getMessageQueueKey } from "./message-queue";
import { emptyStreamingStore } from "./streaming";

const panel: SessionInfo = { id: "session", directory: "/project", workspace: { id: "workspace", generation: 1 } };
const assistant: Extract<TranscriptItem, { kind: "assistant" }> = {
  kind: "assistant", id: "assistant", messageID: "assistant", parts: [], completed: false
};

function sources(): PanelProjectionSources {
  return {
    busyBySession: {}, transcriptsBySession: { session: [assistant] }, todosByWorkspace: {},
    usageBySession: {}, compactionBaselineBySession: {}, modelsByWorkspace: {},
    currentModelByWorkspace: {}, agentsByWorkspace: {}, currentAgentByWorkspace: {},
    streamingStore: emptyStreamingStore(), messageQueue: emptyMessageQueueState(),
    inboxBySession: {}, formsBySession: {}, stagedReverts: {}, sessionAbortFlags: {},
    chatStates: new Map([["session", {
      message: { session: [
        { id: "user", sessionID: "session", role: "user", time: { created: 10 }, model: { providerID: "provider", modelID: "model" } },
        { id: "assistant", sessionID: "session", role: "assistant", parentID: "user", time: { created: 20 } }
      ] },
      part: { assistant: [{ id: "text", messageID: "assistant", type: "text", text: "hello", time: {} }] },
      session_status: {}
    }]])
  };
}

describe("panel projection ownership", () => {
  it("distinguishes absent busy state from authoritative idle and uses raw parts", () => {
    const input = sources();
    const stream = { phase: "streaming" as const, startedAt: 20, lastUpdateAt: 21 };
    const streaming = emptyStreamingStore();
    streaming.streamingMessageIds.set(panel.id, assistant.id);
    streaming.messageStreamStates.set(assistant.id, stream);
    input.streamingStore = streaming;
    const projected = projectPanelViews([panel], input).workspace;
    expect(projected).toMatchObject({
      busy: true, turnStartedAt: 10, activeModel: { providerId: "provider", modelId: "model" },
      assistantStatus: { statusText: "composing", canAbort: true }
    });
    expect(projected.streaming).toBe(stream);
    expect(projected.transcript).toBe(input.transcriptsBySession.session);
    input.busyBySession = { session: false };
    expect(projectPanelViews([panel], input).workspace).toMatchObject({ busy: false, turnStartedAt: null });
    input.transcriptsBySession = { session: [{ ...assistant, completed: true }] };
    expect(projectPanelViews([panel], input).workspace.streaming).toBeNull();
  });

  it("preserves retry, permission waiting, and abort status independently", () => {
    const input = sources();
    input.busyBySession = { session: true };
    input.transcriptsBySession = { session: [{ ...assistant, retry: { attempt: 2, next: 500, message: "retry" } }] };
    expect(projectPanelViews([panel], input).workspace).toMatchObject({
      busy: true, activity: { phase: "retry" }, assistantStatus: { retryInfo: { attempt: 2, next: 500 } }
    });
    input.transcriptsBySession.session.push({ kind: "permission", id: "permission", requestID: "request", action: "read", resources: [], pending: true });
    expect(projectPanelViews([panel], input).workspace).toMatchObject({
      busy: false, assistantStatus: { isWaitingForPermission: true, canAbort: false }
    });
    input.transcriptsBySession = { session: [assistant] };
    input.sessionAbortFlags = { session: { timestamp: 100, acknowledged: false } };
    expect(projectPanelViews([panel], input).workspace).toMatchObject({
      busy: true, assistantStatus: { wasAborted: true, isWorking: false, canAbort: false }
    });
  });

  it("scopes queues by workspace and session while retaining form and revert identities", () => {
    const input = sources();
    const local = { id: "local", content: "follow up", createdAt: 2 };
    const queue = emptyMessageQueueState();
    input.messageQueue = queue;
    queue.queuedMessages[getMessageQueueKey({ workspaceID: panel.workspace.id, sessionID: panel.id })] = [local];
    queue.queuedMessages[getMessageQueueKey({ workspaceID: "other", sessionID: panel.id })] = [{ ...local, id: "wrong-workspace" }];
    input.inboxBySession = { session: [{ id: "native", text: "next", createdAt: 1, attachmentCount: 3 }] };
    input.formsBySession = { session: [{ id: "form", sessionID: "session", title: "Question", fields: [] }] };
    input.stagedReverts = { session: { messageID: "user" } };
    const projected = projectPanelViews([panel], input).workspace;
    expect(projected.queuedMessages).toEqual([{ id: "native", content: "next", createdAt: 1, attachmentCount: 3 }, local]);
    expect(projected.queuedMessages[1]).toBe(local);
    expect(projected.queuedCount).toBe(2);
    expect(projected.pendingForms).toBe(input.formsBySession.session);
    expect(projected.stagedRevert).toBe(input.stagedReverts.session);
    expect(projected.busy).toBe(true);
  });

  it("binds workspace catalogs and session records to the current generation only", () => {
    const input = sources();
    const replacement: SessionInfo = { ...panel, id: "replacement", workspace: { id: "replacement-workspace", generation: 2 } };
    input.todosByWorkspace = { workspace: [{ id: "todo", content: "old", status: "pending" }] };
    input.compactionBaselineBySession = { session: 100 };
    const views = projectPanelViews([replacement], input);
    expect(Object.keys(views)).toEqual(["replacement-workspace"]);
    expect(views[replacement.workspace.id]).toMatchObject({
      session: replacement, busy: false, transcript: [], todos: [], queuedCount: 0,
      compactionBaseline: null, sessionUsage: null, currentModel: null, currentAgent: null
    });
  });
});
