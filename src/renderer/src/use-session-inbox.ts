import { useCallback, useRef, useState } from "react";
import type { AgentOption, ModelOption, SessionInboxEntry, SessionInfo, TranscriptItem, UserAttachment, WorkspaceIdentity } from "@shared/types";
import {
  addToQueue, createMessageQueueTarget, loadMessageQueueState, persistMessageQueueState,
  removeFromQueue, reorderQueue, popToInput,
  type FollowUpBehavior, type MessageQueueState, type MessageQueueReadView,
  type MessageQueueTarget, type QueuedMessage
} from "./message-queue";
import { formatFailure } from "@shared/errors";

interface InboxHost {
  panelFor: (workspace: WorkspaceIdentity) => SessionInfo | null;
  panelForSession: (sessionID: string) => SessionInfo | null;
  notify: (text: string, tone?: "info" | "error") => void;
}

interface InboxTranscript {
  appendTranscriptItem: (sessionID: string, item: TranscriptItem) => void;
  appendUniqueTranscriptItem: (sessionID: string, item: TranscriptItem) => void;
}

interface InboxSelection {
  currentAgentByWorkspace: Readonly<Record<string, Readonly<Pick<AgentOption, "id">> | null>>;
  currentModelByWorkspace: Readonly<Record<string, Readonly<Pick<ModelOption, "id" | "providerID">> | null>>;
}

export function useSessionInbox(host: InboxHost, transcript: InboxTranscript, selection: InboxSelection) {
  const { panelFor, panelForSession, notify: toast } = host;
  const { appendTranscriptItem, appendUniqueTranscriptItem } = transcript;
  const { currentAgentByWorkspace, currentModelByWorkspace } = selection;
  const [inboxBySession, setInboxBySession] = useState<Record<string, SessionInboxEntry[]>>({});
  const [messageQueue, setMessageQueue] = useState<MessageQueueState>(() => loadMessageQueueState());

  const inboxBySessionRef = useRef(inboxBySession);
  inboxBySessionRef.current = inboxBySession;
  const messageQueueRef = useRef<MessageQueueState>(messageQueue);
  const commitQueue = useCallback((next: MessageQueueState) => {
    messageQueueRef.current = next;
    setMessageQueue(next);
    persistMessageQueueState(next);
  }, []);

  const removeInboxEntry = useCallback((sessionID: string, messageID: string): void => {
    setInboxBySession((current) => ({
      ...current,
      [sessionID]: (current[sessionID] ?? []).filter((entry) => entry.id !== messageID)
    }));
  }, []);

  const setFollowUpBehavior = useCallback((behavior: FollowUpBehavior) => {
    commitQueue({ ...messageQueueRef.current, followUpBehavior: behavior });
  }, [commitQueue]);

  const queueTargetFor = useCallback((workspace: WorkspaceIdentity): MessageQueueTarget | null => {
    const panel = panelFor(workspace);
    return panel ? createMessageQueueTarget(panel.id, panel.workspace.id) : null;
  }, [panelFor]);

  const removeQueuedMessage = useCallback((workspace: WorkspaceIdentity, messageID: string) => {
    const target = queueTargetFor(workspace);
    if (!target) return;
    if (!messageID.startsWith("queued-")) {
      removeInboxEntry(target.sessionID, messageID);
      window.vela.inboxCancel(workspace, messageID).catch(() => {});
      return;
    }
    commitQueue(removeFromQueue(messageQueueRef.current, target, messageID));
  }, [queueTargetFor, commitQueue, removeInboxEntry]);

  const popQueuedMessage = useCallback((workspace: WorkspaceIdentity, messageID: string): QueuedMessage | null => {
    const target = queueTargetFor(workspace);
    if (!target) return null;
    if (!messageID.startsWith("queued-")) {
      const entry = (inboxBySessionRef.current[target.sessionID] ?? []).find(
        (candidate) => candidate.id === messageID
      );
      if (!entry) return null;
      removeInboxEntry(target.sessionID, messageID);
      window.vela.inboxCancel(workspace, messageID).catch(() => {});
      return { id: entry.id, content: entry.text, createdAt: entry.createdAt };
    }
    const popped = popToInput(messageQueueRef.current, target, messageID);
    commitQueue(popped.state);
    return popped.message;
  }, [queueTargetFor, commitQueue, removeInboxEntry]);

  const reorderQueuedMessage = useCallback((workspace: WorkspaceIdentity, fromID: string, toID: string) => {
    const target = queueTargetFor(workspace);
    if (!target) return;
    commitQueue(reorderQueue(messageQueueRef.current, target, fromID, toID));
  }, [queueTargetFor, commitQueue]);

  const refreshInbox = useCallback(async (sessionID: string): Promise<void> => {
    const panel = panelForSession(sessionID);
    if (!panel) return;
    try {
      const entries = await window.vela.inboxList(panel.workspace);
      setInboxBySession((current) => ({ ...current, [sessionID]: entries }));
    } catch (error) {
      if (panelForSession(sessionID)) {
        toast(formatFailure(error, "VELA_INBOX_REFRESH_FAILED", "Queued prompts could not be refreshed"), "error");
      }
    }
  }, [panelForSession, toast]);

  const sendQueuedNow = useCallback(async (workspace: WorkspaceIdentity, messageID: string): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel) return;
    if (!messageID.startsWith("queued-")) {
      removeInboxEntry(panel.id, messageID);
      try {
        await window.vela.inboxSteer(workspace, messageID);
      } catch (err) {
        void refreshInbox(panel.id);
        const failure = formatFailure(err, "VELA_INBOX_STEER_FAILED", "Queued prompt could not be sent");
        appendUniqueTranscriptItem(panel.id, { kind: "status", id: `queued-steer-error-${messageID}`, text: failure, tone: "error" });
        if (panelFor(workspace)) toast(failure, "error");
      }
      return;
    }
    const popped = popQueuedMessage(workspace, messageID);
    if (!popped) return;
    const promptText = popped.content || "Review the attached files.";
    const attachments: UserAttachment[] = (popped.attachments ?? []).map((file) => ({
      name: file.path.split(/[\\/]/).pop() ?? file.path
    }));
    const createdAt = Date.now();
    const selectedAgent = currentAgentByWorkspace[panel.workspace.id];
    const selectedModel = currentModelByWorkspace[panel.workspace.id];
    appendTranscriptItem(panel.id, {
        kind: "user",
        id: `user-${createdAt}`,
        text: promptText,
        createdAt,
        ...(selectedAgent ? { agent: selectedAgent.id } : {}),
        ...(selectedModel ? { model: { id: selectedModel.id, providerID: selectedModel.providerID } } : {}),
        ...(attachments.length > 0 ? { attachments } : {})
    });
    try {
      await window.vela.prompt(workspace, promptText, popped.attachments ?? []);
    } catch (err) {
      const failure = formatFailure(err, "VELA_PROMPT_FAILED", "Queued prompt failed");
      const queueTarget = createMessageQueueTarget(panel.id, panel.workspace.id);
      if (queueTarget) commitQueue(addToQueue(messageQueueRef.current, queueTarget, {
        content: promptText,
        attachments: popped.attachments
      }));
      appendTranscriptItem(panel.id, { kind: "status", id: `queued-error-${messageID}`, text: failure, tone: "error" });
      if (panelFor(workspace)) toast(failure, "error");
    }
  }, [panelFor, popQueuedMessage, appendTranscriptItem, appendUniqueTranscriptItem, toast, refreshInbox, commitQueue, currentAgentByWorkspace, currentModelByWorkspace, removeInboxEntry]);

  const readFollowUpBehavior = useCallback((): FollowUpBehavior => messageQueueRef.current.followUpBehavior, []);

  const enqueueFailedPrompt = useCallback((
    captured: Readonly<Pick<SessionInfo, "id" | "workspace">>,
    message: Omit<QueuedMessage, "id" | "createdAt">
  ): boolean => {
    const target = createMessageQueueTarget(captured.id, captured.workspace.id);
    if (!target) return false;
    commitQueue(addToQueue(messageQueueRef.current, target, message));
    return true;
  }, [commitQueue]);

  const reconcileDeliveredPrompt = useCallback((sessionID: string, text: string): void => {
    setInboxBySession((current) => {
      const list = current[sessionID];
      if (!list?.some((entry) => entry.text === text && entry.delivery !== "queue")) return current;
      return { ...current, [sessionID]: list.filter((entry) => entry.text !== text || entry.delivery === "queue") };
    });
  }, []);

  const receiveEnqueuedInbox = useCallback((sessionID: string, data: Record<string, any>): void => {
    const item = data.item as Record<string, any> | undefined;
    if (item?.type === "user") {
      const payload = item.payload as Record<string, any> | undefined;
      const files = Array.isArray(payload?.files) ? payload.files : [];
      const delivery = data.delivery === "queue"
        ? "queue"
        : data.delivery === "steer"
          ? "steer"
          : typeof item.delivery === "string" && (item.delivery === "queue" || item.delivery === "steer")
            ? item.delivery
            : undefined;
      setInboxBySession((current) => ({
        ...current,
        [sessionID]: [
          ...(current[sessionID] ?? []).filter((entry) => entry.id !== data.inboxID),
          {
            id: data.inboxID,
            text: String(payload?.text ?? ""),
            attachmentCount: files.length,
            createdAt: Date.now(),
            ...(delivery ? { delivery } : {})
          }
        ]
      }));
    }
  }, []);

  const removeTerminalInbox = useCallback((sessionID: string, data: Record<string, any>): void => {
    setInboxBySession((current) => {
      const list = current[sessionID];
      if (!list?.some((entry) => entry.id === data.inboxID)) return current;
      return { ...current, [sessionID]: list.filter((entry) => entry.id !== data.inboxID) };
    });
  }, []);

  return {
    messageQueue: messageQueue as MessageQueueReadView,
    inboxBySession: inboxBySession as Readonly<Record<string, readonly Readonly<SessionInboxEntry>[]>>,
    refreshInbox, setFollowUpBehavior, removeQueuedMessage, popQueuedMessage, reorderQueuedMessage, sendQueuedNow,
    readFollowUpBehavior, enqueueFailedPrompt, reconcileDeliveredPrompt, receiveEnqueuedInbox, removeTerminalInbox
  };
}
