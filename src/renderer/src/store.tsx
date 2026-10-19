import { normalizeTodos } from "@shared/session-presentation";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  memo,
  useMemo,
  useRef,
  useState,
  type ReactNode
} from "react";
import { normalizePendingForm } from "@shared/forms";
import { formatFailure, normalizeFailure } from "@shared/errors";
import type {
  BackendMessage,
  PermissionReply,
  PromptDelivery,
  SessionRevertStage,
  PromptFile,
  ProviderUsageResult,
  RuntimeManifest,
  SessionInfo,
  SessionSummary,
  TodoItem,
  TranscriptItem,
  TreeEntry,
  UserAttachment,
  WorkspaceIdentity
} from "@shared/types";
import type { ChatStreamEvent } from "./chat-stream";
import { IDLE_ACTIVITY, resolveSessionActivity } from "./session-activity";
import { requestReveal } from "./reveal";
import { useSessionLifecycleAuthority, useSessionLifecycle } from "./use-session-lifecycle";
import { useSavedWorkspaces } from "./use-saved-workspaces";
import { useExplorerMutations } from "./use-explorer-mutations";
import { useWorkspaceFiles } from "./use-workspace-files";
import { useWorkspaceEditor } from "./use-workspace-editor";
import { useSessionSelection } from "./use-session-selection";
import { useSessionProgress } from "./use-session-progress";
import { useSessionRequests } from "./use-session-requests";
import { useSessionInbox } from "./use-session-inbox";
import { useSessionInventory } from "./use-session-inventory";
import { useSessionChatRecords } from "./use-session-chat-records";
import { useSessionExecution, useSessionSettlement } from "./use-session-execution";
import { projectPanelViews } from "./panel-projection";
import type { Toast, CtxMenuState, CtxMenuApi, PanelView, Store } from "./store-contract";

export type { Toast, CtxMenuState, CtxMenuApi, PendingCreate, PanelView, UnsavedEditorFile } from "./store-contract";

const CtxMenuContext = createContext<CtxMenuApi | null>(null);

export function useCtxMenu(): CtxMenuApi {
  const ctx = useContext(CtxMenuContext);
  if (!ctx) throw new Error("useCtxMenu must be used within StoreProvider");
  return ctx;
}

const Ctx = createContext<Store | null>(null);

const AUX_CHAT_STREAM_TYPES = new Set([
  "session.inbox.enqueued",
  "session.inbox.delivered",
  "session.inbox.cancelled",
  "session.agent.selected",
  "session.model.selected",
  "session.synthetic",
  "session.skill.activated",
  "session.shell.started",
  "session.shell.ended",
  "session.compaction.started",
  "session.compaction.delta",
  "session.compaction.ended",
  "session.compaction.failed"
]);

function normalizeStreamEvent(msg: BackendMessage): ChatStreamEvent | null {  if (msg.kind !== "event") return null;
  const event = msg.data as Record<string, any> | undefined;
  const data = (event?.data ?? event?.properties ?? event) as Record<string, any> | undefined;
  const rawType = msg.type ?? event?.type ?? event?.event ?? "";
  const type = rawType === "permission.v2.asked"
    ? "permission.asked"
    : rawType === "permission.v2.replied"
      ? "permission.replied"
      : rawType;
  if (!data || !type) return null;
  return {
    id: String(event?.id ?? `${type}-${Date.now()}`),
    type,
    created: Number(event?.created ?? Date.now()),
    data,
    ...(Array.isArray(event?.velaSessionIDs)
      ? { ownerSessionIDs: event.velaSessionIDs.filter((value: unknown): value is string => typeof value === "string") }
      : {})
  };
}

function failureForStreamEvent(type: string, data: Record<string, any>): { error: unknown; code: string; message: string } | null {
  if (type === "session.step.failed") return { error: data.error, code: "VELA_STEP_FAILED", message: "Step failed" };
  if (type === "session.execution.failed") return { error: data.error, code: "VELA_EXECUTION_FAILED", message: "Execution failed" };
  if (type === "session.tool.failed") return { error: data.error, code: "VELA_TOOL_FAILED", message: "Tool failed" };
  if (type === "session.retry.scheduled") return { error: data.error, code: "VELA_RETRY_SCHEDULED", message: "Retry scheduled" };
  if (type === "message.updated" && data.info?.error) return { error: data.info.error, code: "VELA_ASSISTANT_FAILED", message: "Assistant failed" };
  if (type === "message.part.updated") {
    const part = (data.part ?? data) as Record<string, any>;
    const state = part.state as Record<string, any> | undefined;
    if (["error", "failed", "aborted", "timeout", "cancelled"].includes(String(state?.status))) {
      return { error: state?.error, code: "VELA_TOOL_FAILED", message: "Tool failed" };
    }
  }
  return null;
}

let toastId = 0;

const STALE_SESSION_LAYOUT_KEY = "vela.sessionLayout";
const LEGACY_STALE_SESSION_LAYOUT_KEY = "vela.sessionLayout";

export function StoreProvider({ children, onFileActivate }: { children: ReactNode; onFileActivate?: () => void }): ReactNode {
  const [ctxMenu, setCtxMenu] = useState<CtxMenuState | null>(null);
  const openCtxMenu = useCallback((x: number, y: number, target: TreeEntry | null, workspaceSessionID?: string) => {
    setCtxMenu({ x, y, target, workspaceSessionID });
  }, []);
  const closeCtxMenu = useCallback(() => setCtxMenu(null), []);
  const ctxMenuApi = useMemo<CtxMenuApi>(
    () => ({ ctxMenu, openCtxMenu, closeCtxMenu }),
    [ctxMenu, openCtxMenu, closeCtxMenu]
  );
  const body = useMemo(() => <StoreBody closeCtxMenu={closeCtxMenu} onFileActivate={onFileActivate}>{children}</StoreBody>, [children, closeCtxMenu, onFileActivate]);
  return (
    <CtxMenuContext.Provider value={ctxMenuApi}>
      {body}
    </CtxMenuContext.Provider>
  );
}

const StoreBody = memo(function StoreBody({ children, closeCtxMenu, onFileActivate }: { children: ReactNode; closeCtxMenu: () => void; onFileActivate?: () => void }): ReactNode {
  const lifecycle = useSessionLifecycleAuthority();
  const { panels, workspaceOnlyPanelIDs, activeSessionID, session,
    protectedSessionIDs, panelFor, panelForSession, readFocusedSession, readPanels,
    workspaceOfSession, nextRequest, hasUserActivated, hasPendingSwap, hasPendingReplacement, mirrorRenderedState } = lifecycle;
  const [connected, setConnected] = useState(false);
  const [runtimes, setRuntimes] = useState<RuntimeManifest[]>([]);
  const {
    transcriptsBySession, rawChatReadMap, transcriptFor, hasTranscript, recordStatusFor,
    snapshotSession, reduceSessionEvent, completeLatest, attachRetry, projectSession: applyProjection,
    deriveChangedStreaming, deriveStreaming, prepareReopenedReplay, commitReplayTranscript,
    mergeMaterializedRecords, hydratePromptRecords, reconcilePromptTranscript, forgetRecords,
    appendTranscriptItem, appendUniqueTranscriptItem, replaceTranscriptItemAtTail,
    appendRateLimitNotice,
    appendPromptUser, promoteDeliveredInput, reduceAuxiliaryEvent,
    addPermissionFromEvent, resolvePermission, reconcilePermissionRows
  } = useSessionChatRecords(protectedSessionIDs);
  const execution = useSessionExecution({
    transcriptFor, snapshotSession, reduceSessionEvent, recordStatusFor,
    projectSession: applyProjection, attachRetry, appendRateLimitNotice, completeLatest,
    deriveChangedStreaming, deriveStreaming
  });
  const {
    busyBySession, sessionAbortFlags, streamingStore,
    busyFor, abortFor, streamingMessageFor, markStreamActivity,
    setSessionBusy, seedReplayBusy, pruneBusyState, dropClosedActivity,
    beginLocalAbort, acknowledgePromptAbort, syncStreaming, reconcileStreaming,
    shouldIgnoreRuntimeWork, noteRuntimeReduction, touchDeltaStream,
    startExecution, finishExecution, applyStatusEvent
  } = execution;
  const [stagedReverts, setStagedReverts] = useState<Record<string, SessionRevertStage | null>>({});
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toast = useCallback((text: string, tone: "info" | "error" = "info") => {
    const id = ++toastId;
    setToasts((prev) => [...prev.slice(-3), { id, text, tone }]);
    setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 4000);
  }, []);

  const {
    approvalMode, formsBySession, refreshForms, submitForm, dismissForm,
    reconcilePermissions, replyPermission, toggleApprovalMode, readApprovalMode,
    receiveForms, removeTerminalForms
  } = useSessionRequests(
    { panelFor, panelForSession, readFocusedSession, readPanels, notify: toast },
    { reconcilePermissionRows, resolvePermission }
  );
  const {
    todosByWorkspace, usageBySession, compactionBaselineBySession, providerUsage, providerUsageLoading,
    setTodosFor, recordUsage, refreshSessionUsage, refreshProviderUsage, normalizeSessionUsage,
    persistRenderedBaseline, inferRenderedBaselines, inferMaterializedBaseline,
    snapshotCompactionBaseline, removeClosedBaseline, applyBusyTodoFallback
  } = useSessionProgress({ panelForSession, protectedSessionIDs, notify: toast });
  useEffect(() => {
    try {
      window.localStorage.removeItem("runtimeID");
    } catch {}
  }, []);
  useEffect(() => {
    persistRenderedBaseline(compactionBaselineBySession);
  }, [compactionBaselineBySession]);
  const busy = session ? Boolean(busyBySession[session.id]) : false;
  const todos = session ? (todosByWorkspace[session.workspace.id] as TodoItem[] | undefined ?? []) : [];
  const transcript = session ? transcriptsBySession[session.id] ?? [] : [];
  const sessionUsage = session ? usageBySession[session.id] ?? null : null;
  const refreshRuntimes = useCallback(async (): Promise<RuntimeManifest[]> => {
    const items = await window.vela.runtimes();
    setRuntimes(items);
    return items;
  }, []);

  useEffect(() => {
    void refreshRuntimes().catch(() => setRuntimes([]));
  }, [refreshRuntimes]);

  useEffect(() => {
    pruneBusyState(panels, transcriptsBySession);
  }, [panels, transcriptsBySession, pruneBusyState]);

  useEffect(() => {
    inferRenderedBaselines(panels, transcriptsBySession, usageBySession, compactionBaselineBySession);
  }, [panels, transcriptsBySession, usageBySession, compactionBaselineBySession]);

  mirrorRenderedState(panels, session);
  const startupRestoreStartedRef = useRef(false);

  const {
    activeSessions, sessions, refreshActiveSessions, addActiveSession,
    removeActiveSession, loadSessions, activeSessionFor, summaryFor,
    upsertSummary, renameSummary, removeSummary
  } = useSessionInventory(toast);

  useEffect(() => {
    const timer = setInterval(() => void refreshActiveSessions(), 1000);
    return () => clearInterval(timer);
  }, [refreshActiveSessions]);

  const {
    workspaceFiles, agentFiles, tree, expanded, hiddenPaths, recoveryRecords,
    readAgentFile, isHiddenPath, toggleDir, prefetchDir, ensureRootOpen, refreshTree, unhidePath,
    dismissChange, dismissChanges, loadRecovery, openRecovery, acknowledgeRecovery,
    hidePath, restoreDetachedPath, hideNestedWorkspace, restoreNestedWorkspaceVisibility,
    expandParent, recordCreatedEntry, renameObservedPrefix, moveObservedPrefix, moveObservedPaths,
    receiveFileUpdate, receiveRecovery, scheduleFileRefresh
  } = useWorkspaceFiles({ renderSession: session, readFocusedSession, panelFor, readPanels, notify: toast });

  const materializingRef = useRef(new Set<string>());

  useSessionSettlement(execution, {
    readPanels, transcriptFor, completeLatest, projectSession: applyProjection, notify: toast
  });

  const { savedWorkspaces, rememberWorkspace, replaceSavedWorkspace, saveWorkspace, removeWorkspace } = useSavedWorkspaces(connected, toast);
  const {
    models, currentModel, agents, currentAgent, availableModels, lastModel,
    modelsByWorkspace, currentModelByWorkspace, agentsByWorkspace, currentAgentByWorkspace,
    loadModels, switchModel, loadAgents, switchAgent,
    applyModelSelection, applyAgentSelection
  } = useSessionSelection({ readFocusedSession, panelFor, notify: toast });

  const {
    messageQueue, inboxBySession, refreshInbox, setFollowUpBehavior,
    removeQueuedMessage, popQueuedMessage, reorderQueuedMessage, sendQueuedNow,
    readFollowUpBehavior, enqueueFailedPrompt, reconcileDeliveredPrompt, receiveEnqueuedInbox, removeTerminalInbox
  } = useSessionInbox(
    { panelFor, panelForSession, notify: toast },
    { appendTranscriptItem, appendUniqueTranscriptItem },
    { currentAgentByWorkspace, currentModelByWorkspace }
  );

  const materializeSession = useCallback(async (sessionID: string): Promise<void> => {
    const panel = panelForSession(sessionID);
    if (!panel || materializingRef.current.has(sessionID)) return;
    materializingRef.current.add(sessionID);
    try {
      const [snapshot, usage] = await Promise.all([
        window.vela.sessionTranscript(sessionID),
        window.vela.sessionUsage
          ? window.vela.sessionUsage(sessionID).catch(() => null)
          : Promise.resolve(null)
      ]);
      if (!panelForSession(sessionID)) return;
      const aborted = abortFor(sessionID)?.acknowledged === false;
      mergeMaterializedRecords(sessionID, snapshot.transcript, aborted);
      applyProjection(sessionID);
      reconcileStreaming(sessionID);
      setTodosFor(panel.workspace.id, snapshot.todos);
      if (usage) {
        recordUsage(sessionID, () => usage);
        inferMaterializedBaseline(sessionID, snapshot.transcript, usage);
      }
      const trailing = [...snapshot.transcript].reverse().find((item) => item.kind === "assistant");
      if (aborted || !trailing || trailing.completed) setSessionBusy(sessionID, false);
    } catch (error) {
      if (panelForSession(sessionID)) {
        toast(formatFailure(error, "VELA_SESSION_MATERIALIZE_FAILED", "Session state could not be refreshed"), "error");
      }
    } finally {
      materializingRef.current.delete(sessionID);
    }
  }, [panelForSession, mergeMaterializedRecords, applyProjection, reconcileStreaming, setTodosFor, setSessionBusy, recordUsage, inferMaterializedBaseline, toast, abortFor]);

  const hydrateTranscript = useCallback(
    async (sessionID: string): Promise<void> => {
      const request = nextRequest();
      try {
        const reopened = await window.vela.openSessionById(sessionID, request);
        if (!panelForSession(sessionID)) return;
        const aborted = abortFor(sessionID)?.acknowledged === false;
        const merged = prepareReopenedReplay(sessionID, reopened.transcript, aborted);
        reconcileStreaming(sessionID);
        commitReplayTranscript(sessionID, merged);
        const running = [...reopened.transcript].reverse().find((item) => item.kind === "assistant");
        seedReplayBusy(sessionID, running, aborted);
        if (aborted) applyProjection(sessionID);
        setTodosFor(reopened.session.workspace.id, reopened.todos);
        if (reopened.usage) {
          recordUsage(sessionID, () => reopened.usage!);
        }
      } catch (err) {
        if (panelForSession(sessionID)) {
          toast(formatFailure(err, "VELA_SESSION_OPEN_FAILED", "Session could not be opened"), "error");
        }
      }
    },
    [toast, panelForSession, prepareReopenedReplay, commitReplayTranscript, reconcileStreaming, applyProjection, setTodosFor, recordUsage, abortFor, seedReplayBusy]
  );

  const clearStagedRevert = useCallback(async (workspace: WorkspaceIdentity): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel) return;
    try {
      await window.vela.revertClear(workspace);
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), "error");
    } finally {
      setStagedReverts((current) => ({ ...current, [panel.id]: null }));
    }
  }, [panelFor, toast]);

  const commitStagedRevert = useCallback(async (workspace: WorkspaceIdentity): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel) return;
    try {
      await window.vela.revertCommit(workspace);
      setStagedReverts((current) => ({ ...current, [panel.id]: null }));
      await hydrateTranscript(panel.id);
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [panelFor, toast, hydrateTranscript]);

  const stageRevert = useCallback(async (workspace: WorkspaceIdentity, messageID: string): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel) return;
    try {
      const staged = await window.vela.revertStage(workspace, messageID, true);
      setStagedReverts((current) => ({ ...current, [panel.id]: staged }));
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [panelFor, toast]);


  // Editor initializes before lifecycle workflows; this stable bridge is invoked only after render.
  const focusEditorSession = useCallback((sessionID: string, hydrate = true): void => {
    focusSession(sessionID, hydrate);
  }, [hydrateTranscript, closeCtxMenu, hasTranscript]);

  const {
    tabs, activePath, singleFile, unsavedEditorFiles, unsavedEditorFileCount,
    readRenderedActivePath, dirtyTabsFor, doSaveRef,
    openFile, previewFile, openExternalPath, setActive, saveAllEditorFiles,
    closeTab, saveTab, editContent, reloadTab, overwriteTab, mergeTab,
    setSingleFileFor, addCreatedFile, removePathTabs, removePathsTabs,
    renameTabs, movePathTabs, movePathsTabs, prepareFileUpdate, applyFileUpdate,
    cancelPrefix, cancelAll
  } = useWorkspaceEditor({ readFocusedSession, readPanels, panelFor, focusSession: focusEditorSession, readAgentFile, notify: toast, onFileActivate });

  const {
    pendingCreate, pendingRename, startCreate, startRename, cancelPending, commitName,
    deleteEntry, deleteEntries, removeFromWorkspace, importPaths, dropIntoExplorer,
    moveEntry, moveEntries, revealInFileManager
  } = useExplorerMutations(
    { readFocusedSession, panelFor, notify: toast, closeMenu: closeCtxMenu },
    { expandParent, hidePath, restoreDetachedPath, refreshTree, isHiddenPath, unhidePath,
      recordCreatedEntry, renameObservedPrefix, moveObservedPrefix, moveObservedPaths },
    { cancelPrefix, addCreatedFile, removePathTabs, removePathsTabs, renameTabs,
      movePathTabs, movePathsTabs, readRenderedActivePath }
  );

  const applyReopenedSnapshot = useCallback((reopened: Awaited<ReturnType<typeof window.vela.openSessionById>>): void => {
    const merged = prepareReopenedReplay(reopened.session.id, reopened.transcript, false);
    reconcileStreaming(reopened.session.id);
    commitReplayTranscript(reopened.session.id, merged);
    const running = [...reopened.transcript].reverse().find((item) => item.kind === "assistant");
    seedReplayBusy(reopened.session.id, running);
    setTodosFor(reopened.session.workspace.id, reopened.todos);
    if (reopened.usage) {
      recordUsage(reopened.session.id, () => reopened.usage!);
    }
  }, [prepareReopenedReplay, reconcileStreaming, commitReplayTranscript, seedReplayBusy, setTodosFor, recordUsage]);
  const forgetClosedSessionRecords = useCallback((sessionID: string): void => {
    forgetRecords(sessionID);
    materializingRef.current.delete(sessionID);
    removeClosedBaseline(sessionID);
  }, [forgetRecords, removeClosedBaseline]);
  const saveCapturedSnapshot = useCallback((snapshot: Parameters<typeof doSaveRef.current>[0]) =>
    doSaveRef.current(snapshot), []);

  const {
    attachPanel, focusSession, closePanel, deleteSession, openSession, addModelPanel,
    openWorkspacePanel, selectAddPanel, selectPanelDirectory, changePanelDirectory,
    selectFolder, reopenSession, selectFile, openFileWorkspace
  } = useSessionLifecycle(lifecycle,
    { addActiveSession, removeActiveSession, activeSessionFor, summaryFor, loadSessions },
    { rememberWorkspace, replaceSavedWorkspace },
    { hideNestedWorkspace, restoreNestedWorkspaceVisibility, loadRecovery },
    { dirtyTabsFor, saveCapturedSnapshot, setSingleFileFor, openFile },
    { refreshInbox, refreshForms, loadModels, loadAgents },
    { hasTranscript, hydrateTranscript, applyReopenedSnapshot },
    { busyFor, dropClosedActivity },
    { notify: toast, closeMenu: closeCtxMenu, cancelPending, forgetClosedSessionRecords }
  );

  useEffect(() => {
    if (connected) void loadSessions();
  }, [connected, loadSessions]);

  const sendPrompt = useCallback(
    async (text: string, files: PromptFile[] = [], workspace?: WorkspaceIdentity) => {
      const target = workspace ?? readFocusedSession()?.workspace;
      const panel = target ? panelFor(target) : null;
      if (!panel || !target) {
        toast("[VELA_PROMPT_NO_SESSION] No active session is available for this prompt", "error");
        return;
      }
      const t = text.trim();
      if (!t && files.length === 0) {
        toast("[VELA_PROMPT_EMPTY] Enter a prompt or attach a file", "error");
        return;
      }
      const promptText = t || "Review the attached files.";
      acknowledgePromptAbort(panel.id);
      const transcript = transcriptFor(panel.id);
      const trailingAssistant = [...transcript].reverse().find((item) => item.kind === "assistant");
      const trailingAssistantIncomplete = trailingAssistant?.kind === "assistant" && !trailingAssistant.completed;
      const activity = resolveSessionActivity({
        sessionId: panel.id,
        statusType: trailingAssistantIncomplete && streamingMessageFor(panel.id)
          ? (trailingAssistant?.kind === "assistant" && trailingAssistant.retry ? "retry" : "busy")
          : undefined,
        trailingAssistantIncomplete,
        pendingPermissions: transcript.filter((item) => item.kind === "permission" && item.pending).length
      });
      const delivery: PromptDelivery | undefined = activity.isWorking
        ? readFollowUpBehavior()
        : undefined;
      if (delivery === "queue") toast(`Queued: ${promptText}`);
      const attachments: UserAttachment[] = files.map((file) => ({
        name: file.path.split(/[\\/]/).pop() ?? file.path
      }));
      const createdAt = Date.now();
      const selectedAgent = currentAgentByWorkspace[panel.workspace.id];
      const selectedModel = currentModelByWorkspace[panel.workspace.id];
      const userItem: TranscriptItem = {
        kind: "user",
        id: `user-${createdAt}`,
        text: promptText,
        createdAt,
        ...(selectedAgent ? { agent: selectedAgent.id } : {}),
        ...(selectedModel ? { model: { id: selectedModel.id, providerID: selectedModel.providerID } } : {}),
        ...(attachments.length > 0 ? { attachments } : {})
      };
      appendPromptUser(panel.id, userItem);
      setTodosFor(panel.workspace.id, []);
      setSessionBusy(panel.id, true);
      const applyCanonicalTranscript = (refreshed: Awaited<ReturnType<typeof window.vela.prompt>>): void => {
        const merged = reconcilePromptTranscript(panel.id, refreshed.transcript, userItem);
        markStreamActivity(panel.id);
        hydratePromptRecords(panel.id, merged);
        reconcileStreaming(panel.id);
      };
      try {
        const refreshed = await window.vela.prompt(target, promptText, files, delivery);
        if (panelFor(target)) {
          applyCanonicalTranscript(refreshed);
          reconcileDeliveredPrompt(panel.id, promptText);
        }
      } catch (err) {
        const failure = normalizeFailure(err, "VELA_PROMPT_FAILED", "Prompt failed");
        const failureText = formatFailure(failure);
        const failureID = `prompt-error-${userItem.id}`;
        appendUniqueTranscriptItem(panel.id, { kind: "status", id: failureID, text: failureText, tone: "error" });
        if (panelFor(target)) {
          if (delivery === "queue") {
            if (enqueueFailedPrompt(panel, { content: promptText, attachments: files })) {
              toast(`${failureText}; queued locally: ${promptText}`, "error");
              return;
            }
          }
          setSessionBusy(panel.id, false);
          toast(failureText, "error");
        }
      }
    },
    [
    toast,
    panelFor,
    setTodosFor,
    appendUniqueTranscriptItem,
    reconcilePromptTranscript,
    setSessionBusy,
    enqueueFailedPrompt,
    readFollowUpBehavior,
    reconcileDeliveredPrompt,
    hydratePromptRecords,
    appendPromptUser,
    reconcileStreaming,
    currentAgentByWorkspace,
    currentModelByWorkspace,
    transcriptFor,
    acknowledgePromptAbort,
    streamingMessageFor,
    markStreamActivity,
  ]
  );

  const runCommand = useCallback(async (name: string, args = "", workspace?: WorkspaceIdentity) => {
    const target = workspace ?? readFocusedSession()?.workspace;
    if (!target) return;
    try {
      await window.vela.runCommand(target, name, args);
      if (name === "compact" || name === "compress") {
        toast("Compacting session…");
        const panel = panelFor(target);
        if (panel) {
          setTimeout(() => void refreshSessionUsage(panel.id), 1500);
          // The compaction event may arrive late (or its immediate poll may
          // race the server commit); re-poll once more as a backstop.
          setTimeout(() => void refreshSessionUsage(panel.id), 10_000);
        }
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (panelFor(target)) {
        toast(message, "error");
        throw error;
      }
    }
  }, [panelFor, toast, refreshSessionUsage]);

  const stop = useCallback(async (workspace?: WorkspaceIdentity) => {
    const target = workspace ?? readFocusedSession()?.workspace;
    const panel = target ? panelFor(target) : null;
    if (!panel || !target) return;
    const abortFlag = beginLocalAbort(panel.id, Date.now());
    const previous = snapshotSession(panel.id);
    const interrupted = reduceSessionEvent(panel.id, {
      id: `local-interrupt-${abortFlag.timestamp}`,
      type: "session.execution.interrupted",
      created: abortFlag.timestamp,
      data: { sessionID: panel.id }
    });
    if (interrupted === true || (typeof interrupted !== "boolean" && interrupted.changed)) {
      applyProjection(panel.id);
    }
    syncStreaming(panel.id, previous);
    setSessionBusy(panel.id, false);
    await window.vela.interrupt(target).catch((error) => {
      toast(formatFailure(error, "VELA_INTERRUPT_FAILED", "The prompt could not be interrupted"), "error");
    });
  }, [setSessionBusy, panelFor, toast, snapshotSession, reduceSessionEvent, applyProjection, syncStreaming, beginLocalAbort]);

  useEffect(() => {
    applyBusyTodoFallback(panels, busyBySession, transcriptsBySession);
  }, [panels, busyBySession, transcriptsBySession, setTodosFor]);

  const openSourceTarget = useCallback(
    async (path: string, line: number): Promise<void> => {
      if (!path.startsWith("/")) return;
      const active = readFocusedSession()?.workspace ?? null;
      if (active) {
        const openedPath = await openExternalPath(path, active);
        if (openedPath) requestReveal(openedPath, line);
        return;
      }
      const dir = path.slice(0, path.lastIndexOf("/"));
      if (!dir || dir === path) return;
      const opened = await openSession(dir);
      if (!opened) return;
      const rel = path.slice(dir.length + 1) || path;
      await openFile(rel, opened.workspace);
      requestReveal(rel, line);
    },
    [openSession, openFile, openExternalPath]
  );

  const openPaths = useCallback(async (paths: string[]): Promise<void> => {
    const clean = paths
      .map((path) => path.trim())
      .filter((path) => path.length > 0 && path.startsWith("/"));
    if (clean.length === 0) return;
    for (const absolutePath of clean) {
      let kind: "file" | "directory" | "missing";
      try {
        kind = (await window.vela.externalKind(absolutePath)).kind;
      } catch (err) {
        toast(err instanceof Error ? err.message : String(err), "error");
        continue;
      }
      try {
        if (kind === "directory") {
          if (readPanels().length === 0) await openSession(absolutePath);
          else await openWorkspacePanel(absolutePath);
        } else if (kind === "file") {
          if (readPanels().length === 0) await openFileWorkspace(absolutePath);
          else await openExternalPath(absolutePath);
        } else {
          toast(`${absolutePath} is not available`, "error");
        }
      } catch (err) {
        toast(err instanceof Error ? err.message : String(err), "error");
      }
    }
  }, [openSession, openWorkspacePanel, openFileWorkspace, openExternalPath, toast]);

  useEffect(() => {
    const processMessage = (msg: BackendMessage): void => {
      if (msg.kind === "session") {
        if (msg.session) {
          addActiveSession(msg.session);
          if (!hasPendingSwap() && !hasPendingReplacement(msg.session.id)) {
            attachPanel(msg.session);
          }
        }
        return;
      }
      if (msg.kind === "ui-command") {
        if (msg.command === "open-source" && typeof msg.path === "string" && typeof msg.line === "number") {
          void openSourceTarget(msg.path, msg.line);
        } else if (msg.command === "open-paths" && Array.isArray(msg.data)) {
          void openPaths(msg.data.filter((entry): entry is string => typeof entry === "string"));
        }
        return;
      }
      if (msg.kind === "recovery") {
        if (msg.recovery && panelFor(msg.recovery.workspace)) {
          receiveRecovery(msg.recovery.workspace, msg.recovery.records);
        }
        return;
      }
      if (msg.kind === "file-update") {
        const f = msg.file!;
        const panel = panelFor(f.workspace);
        if (!panel || f.sessionID !== panel.id) return;
        const origin = prepareFileUpdate(f);
        receiveFileUpdate(f);
        applyFileUpdate(f, origin);
        scheduleFileRefresh(f);
        return;
      }
      if (msg.kind !== "event") return;
      const streamEvent = normalizeStreamEvent(msg);
      if (!streamEvent) return;
      const { data, type } = streamEvent;
      const formRecord = (data as Record<string, any>).form as Record<string, any> | undefined;
      const formSessionID = formRecord && typeof formRecord.sessionID === "string" ? formRecord.sessionID : undefined;
      const altSessionID = typeof (data as Record<string, any>).sessionId === "string"
        ? (data as Record<string, any>).sessionId as string
        : undefined;
      const addressedSessionID = typeof data.sessionID === "string"
        ? data.sessionID as string
        : (altSessionID ?? formSessionID);
      const formEvent = type === "form.created" || type === "form.replied" || type === "form.cancelled";
      // Main attaches `velaSessionIDs` when a form belongs to a session Vela
      // does not own directly (a location-global form, a delegated child, or an
      // external `opencode` TUI sharing the directory). Route those to the open
      // panels for the location; keep the form's true sessionID so replies and
      // cancels reach the owning session.
      const locationFormOwnerIDs = formEvent
        ? (streamEvent.ownerSessionIDs ?? []).filter((sessionID) => panelForSession(sessionID))
        : [];
      const locationForm = formEvent && locationFormOwnerIDs.length > 0;
      // The global SSE stream includes external `opencode` terminal
      // sessions and child/subagent streams. Interactive prompts must never
      // be misattributed to the focused panel: only route forms,
      // permissions, and inbox items for sessions Vela has open, unless a
      // form was explicitly attributed to this location by main.
      if ((formEvent ||
        type === "permission.asked" || type === "permission.replied" ||
        type === "session.inbox.enqueued" || type === "session.inbox.delivered" ||
        type === "session.inbox.cancelled") &&
        (locationForm
          ? false
          : (!addressedSessionID || !panelForSession(addressedSessionID)))) {
        return;
      }
      const targetSessionID = locationForm
        ? (locationFormOwnerIDs.includes(readFocusedSession()?.id ?? "") ? readFocusedSession()!.id : locationFormOwnerIDs[0])
        : (addressedSessionID ?? readFocusedSession()?.id);
      const targetWorkspace = targetSessionID ? workspaceOfSession(targetSessionID) : null;
      const active = Boolean(targetSessionID && targetSessionID === readFocusedSession()?.id);
      if (type === "session.inbox.delivered" && targetSessionID && typeof data.inboxID === "string") {
        promoteDeliveredInput(targetSessionID, data.inboxID);
      }

      if (type === "session.inbox.enqueued" && targetSessionID && typeof data.inboxID === "string") {
        receiveEnqueuedInbox(targetSessionID, data);
      }
      if (
        (type === "session.inbox.cancelled" || type === "session.inbox.delivered") &&
        targetSessionID &&
        typeof data.inboxID === "string"
      ) {
        removeTerminalInbox(targetSessionID, data);
      }

      switch (type) {
        case "form.created": {
          const rawForm = (data.form as Record<string, any> | undefined) ?? {};
          const trueSessionID = typeof rawForm.sessionID === "string" ? rawForm.sessionID : targetSessionID;
          const normalized = normalizePendingForm({ ...rawForm, sessionID: trueSessionID });
          const ownerIDs = locationForm ? locationFormOwnerIDs : targetSessionID ? [targetSessionID] : [];
          if (normalized && ownerIDs.length > 0 &&
              (locationForm || normalized.sessionID === "global" || normalized.sessionID === targetSessionID)) {
            receiveForms(ownerIDs, normalized);
          }
          break;
        }
        case "form.replied":
        case "form.cancelled": {
          const formID = typeof data.id === "string" ? data.id : "";
          const ownerIDs = locationForm ? locationFormOwnerIDs : targetSessionID ? [targetSessionID] : [];
          if (ownerIDs.length > 0 && formID) {
            removeTerminalForms(ownerIDs, formID);
          }
          break;
        }
      }

      if (targetSessionID && AUX_CHAT_STREAM_TYPES.has(type)) {
        reduceAuxiliaryEvent(targetSessionID, streamEvent);
        if (type === "session.compaction.ended") {
          snapshotCompactionBaseline(targetSessionID);
          void refreshSessionUsage(targetSessionID);
          // Backstop: the usage snapshot may lag the compaction commit on
          // slow/local backends. Re-poll once so the context display cannot
          // stay pinned to its pre-compaction value. refreshSessionUsage
          // no-ops when the panel is gone.
          setTimeout(() => void refreshSessionUsage(targetSessionID), 8_000);
        } else if (type === "session.compaction.failed") {
          void refreshSessionUsage(targetSessionID);
        }
      }

      switch (type) {
        case "session.step.started":
        case "session.step.ended":
        case "session.step.failed":
        case "session.text.started":
        case "session.text.delta":
        case "session.text.ended":
        case "session.reasoning.started":
        case "session.reasoning.delta":
        case "session.reasoning.ended":
        case "session.tool.input.started":
        case "session.tool.input.delta":
        case "session.tool.input.ended":
        case "session.tool.called":
        case "session.tool.progress":
        case "session.tool.success":
        case "session.tool.failed":
        case "session.retry.scheduled":
        case "message.updated":
        case "message.removed":
        case "message.part.updated":
        case "message.part.delta":
        case "message.part.removed": {
          if (!targetSessionID) break;
          if (shouldIgnoreRuntimeWork(targetSessionID, streamEvent)) {
            break;
          }
          const previous = snapshotSession(targetSessionID);
          const result = reduceSessionEvent(targetSessionID, streamEvent);
          noteRuntimeReduction(targetSessionID, streamEvent, result);
          const materialization = typeof result === "boolean" ? undefined : result.materialization;
          if (materialization) void materializeSession(materialization.sessionID ?? targetSessionID);
          const failure = failureForStreamEvent(type, data);
          if (failure && targetSessionID && result === false) {
            const failureText = formatFailure(failure.error, failure.code, failure.message);
            appendUniqueTranscriptItem(targetSessionID, { kind: "status", id: `${streamEvent.id}-failure`, text: failureText, tone: "error" });
          }
          syncStreaming(targetSessionID, previous);
          touchDeltaStream(targetSessionID, type);
          break;
        }
        case "server.connected":
        case "global.disposed": {
          void loadSessions();
          for (const panel of readPanels()) {
            void materializeSession(panel.id);
            void refreshForms(panel.id);
          }
          void reconcilePermissions();
          break;
        }
        case "global.error": {
          toast(formatFailure(data.error, "VELA_GLOBAL_FAILURE", "Background operation failed"), "error");
          break;
        }
        case "session.created": {
          const location = data.location as { directory?: string } | undefined;
          const directory = location?.directory ?? streamEvent.data.location?.directory;
          if (!targetSessionID || typeof directory !== "string") break;
          const summary: SessionSummary = {
            id: targetSessionID,
            title: typeof data.title === "string" && data.title.trim()
              ? data.title
              : directory.split(/[\\/]/).pop() ?? directory,
            directory,
            updatedAt: streamEvent.created,
            ...(typeof data.parentID === "string" ? { parentID: data.parentID } : {}),
            ...(typeof data.agent === "string" ? { agent: data.agent } : {})
          };
          upsertSummary(summary);
          break;
        }
        case "session.renamed": {
          if (!targetSessionID || typeof data.title !== "string") break;
          renameSummary(targetSessionID, data.title, streamEvent.created);
          break;
        }
        case "session.deleted": {
          if (targetSessionID) removeSummary(targetSessionID);
          break;
        }
        case "session.usage.updated":
        case "session.usage.recorded": {
          const usage = normalizeSessionUsage(data);
          if (!targetSessionID || !usage) break;
          recordUsage(targetSessionID, () => usage);
          break;
        }
        case "session.execution.started": {
          if (targetSessionID) startExecution(targetSessionID, streamEvent);
          break;
        }
        case "session.execution.succeeded":
        case "session.execution.failed":
        case "session.execution.interrupted": {
          if (targetSessionID) {
            finishExecution(targetSessionID, streamEvent);

          }
          if (active && targetWorkspace) setTodosFor(targetWorkspace.id, []);
          const ok = type === "session.execution.succeeded";
          if (!ok && targetSessionID) {
            const errorText = type === "session.execution.failed"
              ? formatFailure(data.error, "VELA_EXECUTION_FAILED", "Execution failed")
              : data.error
                ? formatFailure(data.error, "VELA_EXECUTION_INTERRUPTED", "Execution interrupted")
                : "Interrupted";
            replaceTranscriptItemAtTail(targetSessionID, {
                kind: "status",
                id: `${streamEvent.id}-end`,
                text: errorText,
                tone: type === "session.execution.interrupted" ? "info" : "error"
            });
          }
          break;
        }
        case "session.idle": {
          if (targetSessionID) {
            finishExecution(targetSessionID, streamEvent);

          }
          if (active && targetWorkspace) setTodosFor(targetWorkspace.id, []);
          break;
        }
        case "session.error": {
          if (targetSessionID) {
            finishExecution(targetSessionID, streamEvent);
            const errorText = formatFailure(data.error, "VELA_SESSION_ERROR", "Session failed");
            appendUniqueTranscriptItem(targetSessionID, { kind: "status", id: `${streamEvent.id}-error`, text: errorText, tone: "error" });
          }
          break;
        }
        case "todo.updated": {
          if (targetWorkspace) setTodosFor(targetWorkspace.id, normalizeTodos(data.todos));
          break;
        }
        case "session.model.selected": {
          if (!targetWorkspace) break;
          applyModelSelection(targetWorkspace, data.model as { id?: string; providerID?: string; variant?: string } | undefined);
          break;
        }
        case "session.agent.selected": {
          if (!targetWorkspace) break;
          applyAgentSelection(targetWorkspace, data.agent as string | undefined);
          break;
        }
        case "agent.updated": {
          if (targetWorkspace) void loadAgents(targetWorkspace);
          else void loadAgents();
          break;
        }
        case "catalog.updated":
        case "models-dev.refreshed": {
          if (targetWorkspace) void loadModels(targetWorkspace);
          else void loadModels();
          break;
        }
        case "session.status": {
          applyStatusEvent(targetSessionID, streamEvent);
          break;
        }
        case "permission.asked": {
          const requestID = String(data.id);
          const automatic = readApprovalMode() === "approve";
          if (!targetSessionID) break;
          addPermissionFromEvent(targetSessionID, requestID, data);
          const panel = targetSessionID ? panelForSession(targetSessionID) : null;
          if (automatic && panel) {
            void window.vela.permissionReply(panel.workspace, requestID, "once", panel.id);
          }
          break;
        }
        case "permission.replied": {
          const requestID = String(data.requestID ?? data.id);
          const reply = (data.reply as PermissionReply | undefined) ?? "reject";
          if (!targetSessionID) break;
          resolvePermission(targetSessionID, requestID, reply);
          break;
        }
      }
    };

    const off = window.vela.onMessage((msg: BackendMessage) => {
      processMessage(msg);
    });

    let healthTimer: ReturnType<typeof setTimeout> | null = null;
    let cancelled = false;
    const tryConnect = async (): Promise<void> => {
      if (cancelled) return;
      const ok = await window.vela.health().catch(() => false);
      if (cancelled) return;
      if (ok) {
        setConnected(true);
        return;
      }
      healthTimer = setTimeout(() => void tryConnect(), 2000);
    };
    void tryConnect();
    const permissionTimer = setInterval(() => void reconcilePermissions(), 3000);
    const formTimer = setInterval(() => {
      for (const panel of readPanels()) void refreshForms(panel.id);
    }, 3000);
    void window.vela.takePendingPaths().then((paths) => {
      if (paths.length > 0) void openPaths(paths);
    }).catch(() => {});
    if (!startupRestoreStartedRef.current) {
      startupRestoreStartedRef.current = true;
      void (async () => {
        await window.vela.health().catch(() => false);
        if (cancelled) return;
        try {
          [STALE_SESSION_LAYOUT_KEY, LEGACY_STALE_SESSION_LAYOUT_KEY].forEach((key) => window.localStorage.removeItem(key));
        } catch {}
        // Fresh launches start in an empty IDE: only live backend
        // contexts (e.g. after a renderer reload) are re-attached. Sessions
        // from a previous process stay reachable via recents and Open now.
        const liveSessions = await refreshActiveSessions();
        const restored: SessionInfo[] = [];
        for (const live of liveSessions) {
          if (cancelled) return;
          const session = await reopenSession(live.id, true);
          if (session) restored.push(session);
        }
        if (cancelled) return;
        if (!hasUserActivated() && restored.length > 0) {
          focusSession(restored[restored.length - 1]!.id, false);
        }
      })();
    }
    return () => {
      cancelled = true;
      clearInterval(permissionTimer);
      clearInterval(formTimer);
      if (healthTimer !== null) clearTimeout(healthTimer);
      off();
      cancelAll();
    };
  }, [
    attachPanel,
    addActiveSession,
    focusSession,
    openSourceTarget,
    loadModels,
    loadAgents,
    applyModelSelection,
    applyAgentSelection,
    loadRecovery,
    reopenSession,
    appendUniqueTranscriptItem,
    replaceTranscriptItemAtTail,
    reduceAuxiliaryEvent,
    resolvePermission,
    addPermissionFromEvent,
    promoteDeliveredInput,
    receiveEnqueuedInbox,
    removeTerminalInbox,
    panelFor,
    panelForSession,
    workspaceOfSession,
    recordUsage,
    setTodosFor,
    receiveRecovery,
    receiveFileUpdate,
    scheduleFileRefresh,
    prepareFileUpdate,
    applyFileUpdate,
    snapshotSession,
    reduceSessionEvent,
    materializeSession,
    refreshSessionUsage,
    snapshotCompactionBaseline,
    normalizeSessionUsage,
    reconcilePermissions,
    refreshForms,
    readApprovalMode,
    receiveForms,
    removeTerminalForms,
    cancelAll,
    refreshActiveSessions,
    loadSessions,
    upsertSummary,
    renameSummary,
    removeSummary,
    openPaths,
    shouldIgnoreRuntimeWork,
    noteRuntimeReduction,
    syncStreaming,
    touchDeltaStream,
    startExecution,
    finishExecution,
    applyStatusEvent,
  ]);

  useEffect(() => {
    if (!connected || !readFocusedSession()) return;
    void loadModels();
    void loadAgents();
  }, [connected, loadModels, loadAgents]);


  const panelViews = useMemo<Record<string, PanelView>>(() => projectPanelViews(panels, {
    busyBySession,
    transcriptsBySession,
    todosByWorkspace,
    usageBySession,
    compactionBaselineBySession,
    modelsByWorkspace,
    currentModelByWorkspace,
    agentsByWorkspace,
    currentAgentByWorkspace,
    streamingStore,
    messageQueue,
    inboxBySession,
    formsBySession,
    stagedReverts,
    sessionAbortFlags,
    chatStates: rawChatReadMap
  }), [
    panels,
    busyBySession,
    transcriptsBySession,
    todosByWorkspace,
    usageBySession,
    compactionBaselineBySession,
    modelsByWorkspace,
    currentModelByWorkspace,
    agentsByWorkspace,
    currentAgentByWorkspace,
    streamingStore,
    messageQueue,
    formsBySession,
    stagedReverts,
    sessionAbortFlags
  ]);

  const value = useMemo<Store>(
    () => ({
      session,
      connected,
      runtimes,
      refreshRuntimes,
      busy,
      todos,
      transcript,
      sessionUsage,
      providerUsage: providerUsage as ProviderUsageResult[],
      providerUsageLoading,
      tabs,
      unsavedEditorFileCount,
      unsavedEditorFiles,
      activePath,
      singleFile,
      agentFiles, workspaceFiles,
      tree,
      expanded,
      hiddenPaths,
      toasts, notify: toast,
      recoveryRecords,
      models,
      availableModels,
      lastModel,
      currentModel,
      agents,
      currentAgent,
      approvalMode,
      followUpBehavior: messageQueue.followUpBehavior,
      setFollowUpBehavior,
      sessions,
      savedWorkspaces,
      saveWorkspace,
      removeWorkspace,
      activeSessions,
      panels: panels as SessionInfo[],
      workspaceOnlyPanelIDs: workspaceOnlyPanelIDs as Set<string>,
      panelViews,
      activeSessionID,
      focusSession,
      closePanel,
      openSession,
      addModelPanel,
      openWorkspacePanel,
      selectAddPanel,
      selectFolder,
      selectFile,
      openFileWorkspace,
      openExternalPath,
      importPaths,
      dropIntoExplorer,
      openPaths,
      dismissChange,
      dismissChanges,
      selectPanelDirectory,
      changePanelDirectory,
      reopenSession,
      deleteSession,
      loadSessions,
      sendPrompt,
      runCommand,
      stop,
      refreshProviderUsage,
      loadModels,
      switchModel,
      loadAgents,
      switchAgent,
      toggleApprovalMode,
      openFile,
      previewFile,
      closeTab,
      setActive,
      editContent,
      saveTab,
      saveAllEditorFiles,
      reloadTab,
      overwriteTab,
      mergeTab,
      toggleDir,
      prefetchDir,
      ensureRootOpen,
      revealInFileManager,
      replyPermission,
      removeQueuedMessage,
      popQueuedMessage,
      sendQueuedNow,
      reorderQueuedMessage,
      submitForm,
      dismissForm,
      stageRevert,
      commitStagedRevert,
      clearStagedRevert,
      pendingCreate,
      pendingRename,
      startCreate,
      startRename,
      cancelPending,
      commitName,
      deleteEntry,
      deleteEntries,
      removeFromWorkspace,
      moveEntry,
      moveEntries,
      openRecovery,
      acknowledgeRecovery
    }),
    [
      session, connected, runtimes, refreshRuntimes, busy, todos, transcript, sessionUsage, providerUsage, providerUsageLoading, tabs, unsavedEditorFileCount, unsavedEditorFiles, activePath, singleFile, agentFiles, tree, expanded, hiddenPaths, toasts, toast, recoveryRecords,
      models, availableModels, lastModel, currentModel, agents, currentAgent, approvalMode, messageQueue.followUpBehavior, setFollowUpBehavior, sessions, savedWorkspaces, saveWorkspace, removeWorkspace, activeSessions, panels, workspaceOnlyPanelIDs, panelViews, activeSessionID,
      focusSession, closePanel, openSession, addModelPanel, openWorkspacePanel, selectAddPanel, selectFolder, selectFile, openFileWorkspace, openExternalPath, importPaths, dropIntoExplorer, selectPanelDirectory, changePanelDirectory, reopenSession, loadSessions, sendPrompt, runCommand, stop, refreshProviderUsage, loadModels, switchModel,
      loadAgents, switchAgent, toggleApprovalMode,
      openFile, previewFile, closeTab, setActive, revealInFileManager,
      editContent, saveTab, saveAllEditorFiles, reloadTab, overwriteTab, mergeTab, workspaceFiles, toggleDir, prefetchDir, ensureRootOpen, replyPermission,
      startCreate, startRename, cancelPending, commitName, deleteEntry, deleteEntries, removeFromWorkspace, moveEntry, moveEntries, openRecovery, acknowledgeRecovery,
      removeQueuedMessage, popQueuedMessage, sendQueuedNow, reorderQueuedMessage,
      submitForm, dismissForm,
      stageRevert, commitStagedRevert, clearStagedRevert,
      pendingCreate, pendingRename
    ]
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
});

export function useStore(): Store {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useStore must be used within StoreProvider");
  return ctx;
}

const EMPTY_VIEW: PanelView = {
  session: null,
  busy: false,
  transcript: [],
  todos: [],
  sessionUsage: null,
  compactionBaseline: null,
  models: [],
  currentModel: null,
  agents: [],
  currentAgent: null,
  activity: IDLE_ACTIVITY,
  assistantStatus: null,
  forming: null,
  activeModel: null,
  streaming: null,
  turnStartedAt: null,
  queuedCount: 0,
  queuedMessages: [],
  pendingForms: [],
  stagedRevert: null
};

export function usePanel(workspace: WorkspaceIdentity | null | undefined): PanelView {
  const store = useContext(Ctx);
  if (!store) throw new Error("usePanel must be used within StoreProvider");
  if (!workspace) return EMPTY_VIEW;
  return store.panelViews[workspace.id] ?? EMPTY_VIEW;
}
