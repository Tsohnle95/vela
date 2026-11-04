import { useCallback, useMemo, useRef, useState } from "react";
import type { SessionInfo, SessionRevertStage, TodoItem, TranscriptItem, WorkspaceIdentity } from "@shared/types";

interface RevertHost {
  panelFor: (workspace: WorkspaceIdentity) => SessionInfo | null;
  notify: (text: string, tone?: "info" | "error") => void;
  busyFor: (sessionID: string) => boolean | undefined;
  transcriptFor: (sessionID: string) => readonly TranscriptItem[];
  prepareReopenedReplay: (sessionID: string, history: TranscriptItem[], completeAborted: boolean, authoritative?: boolean) => TranscriptItem[];
  commitReplayTranscript: (sessionID: string, transcript: TranscriptItem[]) => void;
  reconcileStreaming: (sessionID: string) => void;
  setSessionBusy: (sessionID: string, busy: boolean) => void;
  setTodosFor: (workspaceID: string, todos: TodoItem[]) => void;
}

export function useSessionRevert(host: RevertHost) {
  const { panelFor, notify, busyFor, transcriptFor, prepareReopenedReplay, commitReplayTranscript, reconcileStreaming, setSessionBusy, setTodosFor } = host;
  const revertInFlight = useRef(new Set<string>());
  const [restoredPrompts, setRestoredPrompts] = useState<Record<string, { text: string }>>({});
  const [stagedReverts, setStagedReverts] = useState<Record<string, SessionRevertStage | null>>({});
  const clearStagedRevert = useCallback(async (workspace: WorkspaceIdentity): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel) return;
    try {
      await window.vela.revertClear(workspace);
    } catch (err) {
      notify(err instanceof Error ? err.message : String(err), "error");
    } finally {
      setStagedReverts((current) => ({ ...current, [panel.id]: null }));
    }
  }, [panelFor, notify]);

  const commitStagedRevert = useCallback(async (workspace: WorkspaceIdentity): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel || revertInFlight.current.has(panel.id)) return;
    revertInFlight.current.add(panel.id);
    try {
      await window.vela.revertCommit(workspace);
      setStagedReverts((current) => ({ ...current, [panel.id]: null }));
      const snapshot = await window.vela.sessionTranscript(panel.id);
      if (!panelFor(workspace)) return;
      const transcript = prepareReopenedReplay(panel.id, snapshot.transcript, false, true);
      commitReplayTranscript(panel.id, transcript);
      reconcileStreaming(panel.id);
      setSessionBusy(panel.id, false);
      setTodosFor(workspace.id, snapshot.todos);
    } catch (err) {
      notify(err instanceof Error ? err.message : String(err), "error");
    } finally {
      revertInFlight.current.delete(panel.id);
    }
  }, [panelFor, notify, prepareReopenedReplay, commitReplayTranscript, reconcileStreaming, setSessionBusy, setTodosFor]);

  const clearRestoredPrompt = useCallback((workspace: WorkspaceIdentity): void => {
    const panel = panelFor(workspace);
    if (!panel) return;
    setRestoredPrompts((current) => {
      const next = { ...current };
      delete next[panel.id];
      return next;
    });
  }, [panelFor]);

  const stageRevert = useCallback(async (workspace: WorkspaceIdentity, messageID: string, edit = false): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel || busyFor(panel.id) || revertInFlight.current.has(panel.id)) return;
    const prompt = transcriptFor(panel.id).find((item) => item.kind === "user" && item.id === messageID);
    revertInFlight.current.add(panel.id);
    notify(edit ? "Restoring prompt for editing…" : "Undoing prompt…");
    let staged: SessionRevertStage | undefined;
    try {
      staged = await window.vela.revertStage(workspace, messageID, true);
      if (!panelFor(workspace)) return;
      await window.vela.revertCommit(workspace);
      staged = undefined;
      if (!panelFor(workspace)) return;
      setStagedReverts((current) => ({ ...current, [panel.id]: null }));
      // A committed runtime revert is authoritative: merging the old live rows
      // would resurrect the messages that the runtime just removed.
      const snapshot = await window.vela.sessionTranscript(panel.id);
      if (!panelFor(workspace)) return;
      const transcript = prepareReopenedReplay(panel.id, snapshot.transcript, false, true);
      commitReplayTranscript(panel.id, transcript);
      reconcileStreaming(panel.id);
      setSessionBusy(panel.id, false);
      setTodosFor(workspace.id, snapshot.todos);
      if (edit && prompt?.kind === "user") setRestoredPrompts((current) => ({ ...current, [panel.id]: { text: prompt.text } }));
    } catch (err) {
      const recovery = staged;
      if (recovery && panelFor(workspace)) setStagedReverts((current) => ({ ...current, [panel.id]: recovery }));
      notify(err instanceof Error ? err.message : String(err), "error");
    } finally {
      revertInFlight.current.delete(panel.id);
    }
  }, [panelFor, busyFor, transcriptFor, notify, prepareReopenedReplay, commitReplayTranscript, reconcileStreaming, setSessionBusy, setTodosFor]);

  const actions = useMemo(() => ({ stageRevert, commitStagedRevert, clearStagedRevert, clearRestoredPrompt }), [stageRevert, commitStagedRevert, clearStagedRevert, clearRestoredPrompt]);
  return { stagedReverts, restoredPrompts, actions };
}
