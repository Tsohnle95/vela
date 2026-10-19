import { useCallback, useRef, useState } from "react";
import type { ApprovalMode, FormAnswers, PendingFormRequest, PendingPermissionRequest, PermissionReply, SessionInfo, WorkspaceIdentity } from "@shared/types";
import type { FormRequestReadView } from "./store-contract";

interface RequestsHost {
  panelFor: (workspace: WorkspaceIdentity) => SessionInfo | null;
  panelForSession: (sessionID: string) => SessionInfo | null;
  readFocusedSession: () => SessionInfo | null;
  readPanels: () => readonly SessionInfo[];
  notify: (text: string, tone?: "info" | "error") => void;
}
interface RequestsChat {
  reconcilePermissionRows: (sessionID: string, requests: PendingPermissionRequest[]) => void;
  resolvePermission: (sessionID: string, requestID: string, reply: PermissionReply) => void;
}

export function useSessionRequests(host: RequestsHost, chat: RequestsChat) {
  const { panelFor, panelForSession, readFocusedSession, readPanels, notify: toast } = host;
  const { reconcilePermissionRows, resolvePermission } = chat;
  const [formsBySession, setFormsBySession] = useState<Record<string, PendingFormRequest[]>>({});
  const [approvalMode, setApprovalMode] = useState<ApprovalMode>(
    () => (window.localStorage.getItem("approvalMode") === "approve" ? "approve" : "ask")
  );
  const approvalModeRef = useRef<ApprovalMode>(approvalMode);
  approvalModeRef.current = approvalMode;

  const refreshForms = useCallback(async (sessionID: string): Promise<void> => {
    const panel = panelForSession(sessionID);
    if (!panel) return;
    try {
      const forms = await window.vela.formsList(panel.workspace);
      setFormsBySession((current) => ({ ...current, [sessionID]: forms }));
    } catch {}
  }, [panelForSession]);

  const reconcilePermissions = useCallback(async (): Promise<void> => {
    const panels = readPanels();
    if (panels.length === 0) return;
    const workspaces = new Map<string, WorkspaceIdentity>();
    for (const panel of panels) workspaces.set(panel.workspace.id, panel.workspace);
    const fetched = new Map<string, PendingPermissionRequest[]>();
    const successfulWorkspaces = new Set<string>();
    await Promise.all([...workspaces.values()].map(async (workspace) => {
      const requests = await window.vela.listPermissions(workspace).catch(() => null);
      if (!requests) return;
      successfulWorkspaces.add(workspace.id);
      for (const request of requests) {
        const list = fetched.get(request.sessionID) ?? [];
        list.push(request);
        fetched.set(request.sessionID, list);
      }
    }));
    for (const panel of panels) {
      // A failed permission-list request is not proof that the session has no
      // pending approvals. Keep the event-backed card until a later poll can
      // confirm the request is gone.
      if (!successfulWorkspaces.has(panel.workspace.id)) continue;
      const requests = fetched.get(panel.id) ?? [];
      reconcilePermissionRows(panel.id, requests);
    }
  }, [readPanels, reconcilePermissionRows]);

  const submitForm = useCallback(async (
    workspace: WorkspaceIdentity,
    formID: string,
    answers: FormAnswers,
    formSessionID?: string
  ): Promise<void> => {
    const panel = panelFor(workspace);
    if (!panel) return;
    setFormsBySession((current) => ({
      ...current,
      [panel.id]: (current[panel.id] ?? []).filter((form) => form.id !== formID)
    }));
    try {
      await window.vela.formReply(workspace, formID, answers, formSessionID);
    } catch (err) {
      void refreshForms(panel.id);
      if (panelFor(workspace)) toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [panelFor, refreshForms, toast]);

  const dismissForm = useCallback((workspace: WorkspaceIdentity, formID: string, formSessionID?: string): void => {
    const panel = panelFor(workspace);
    if (!panel) return;
    setFormsBySession((current) => ({
      ...current,
      [panel.id]: (current[panel.id] ?? []).filter((form) => form.id !== formID)
    }));
    window.vela.formCancel(workspace, formID, formSessionID).catch(() => {});
  }, [panelFor]);

  const replyPermission = useCallback(
    async (requestID: string, reply: PermissionReply, sessionID?: string) => {
      const sid = sessionID ?? readFocusedSession()?.id;
      const panel = sid ? panelForSession(sid) : null;
      if (!panel) return;
      try {
        await window.vela.permissionReply(panel.workspace, requestID, reply, panel.id);
      } catch (err) {
        toast(err instanceof Error ? err.message : String(err), "error");
        return;
      }
      resolvePermission(panel.id, requestID, reply);
    },
    [toast, panelForSession, readFocusedSession, resolvePermission]
  );

  const toggleApprovalMode = useCallback(() => {
    setApprovalMode((current) => {
      const next = current === "approve" ? "ask" : "approve";
      // Persist from the updater so each queued toggle writes its own value, but
      // never mutate `approvalModeRef` here: the ref must keep tracking the last
      // rendered mode so a permission event batched with pending toggles reads
      // the rendered approval, not an updater that has not committed yet.
      window.localStorage.setItem("approvalMode", next);
      return next;
    });
  }, []);

  const readApprovalMode = useCallback((): ApprovalMode => approvalModeRef.current, []);

  const receiveForms = useCallback((ownerIDs: readonly string[], normalized: PendingFormRequest): void => {
    setFormsBySession((current) => {
      const next = { ...current };
      for (const ownerID of ownerIDs) {
        next[ownerID] = [
          ...(current[ownerID] ?? []).filter((form) => form.id !== normalized.id),
          normalized
        ];
      }
      return next;
    });
  }, []);

  const removeTerminalForms = useCallback((ownerIDs: readonly string[], formID: string): void => {
    setFormsBySession((current) => {
      let changed = false;
      const next = { ...current };
      for (const ownerID of ownerIDs) {
        const list = current[ownerID];
        if (!list?.some((form) => form.id === formID)) continue;
        changed = true;
        next[ownerID] = list.filter((form) => form.id !== formID);
      }
      return changed ? next : current;
    });
  }, []);

  return {
    approvalMode, formsBySession: formsBySession as Readonly<Record<string, readonly FormRequestReadView[]>>,
    refreshForms, submitForm, dismissForm, reconcilePermissions, replyPermission,
    toggleApprovalMode, readApprovalMode, receiveForms, removeTerminalForms
  };
}
