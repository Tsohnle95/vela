import { useCallback, useRef, useState, type Dispatch, type RefObject, type SetStateAction } from "react";
import type { OpenFileWorkspaceResult, SessionInfo, SessionSummary, Tab, WorkspaceIdentity } from "@shared/types";
import { sameWorkspace } from "@shared/generation";
import type { SaveSnapshot } from "./editor-persistence";

import type { SessionReadView } from "./store-contract";

type ReopenedSnapshot = Awaited<ReturnType<typeof window.vela.openSessionById>>;
const lifecycleAuthority: unique symbol = Symbol("session lifecycle authority");

interface LifecycleInternals {
  setPanels: Dispatch<SetStateAction<SessionInfo[]>>;
  setWorkspaceOnlyPanelIDs: Dispatch<SetStateAction<Set<string>>>;
  setActiveSessionID: Dispatch<SetStateAction<string | null>>;
  panelsRef: RefObject<SessionInfo[]>;
  sessionRef: RefObject<SessionInfo | null>;
  requestSeqRef: RefObject<number>;
  activationSeqRef: RefObject<number>;
  userActivatedRef: RefObject<boolean>;
  swapPendingRef: RefObject<boolean>;
  replacingSessionIDsRef: RefObject<Map<string, number>>;
}

export function useSessionLifecycleAuthority() {
  const [panels, setPanels] = useState<SessionInfo[]>([]);
  const [workspaceOnlyPanelIDs, setWorkspaceOnlyPanelIDs] = useState<Set<string>>(() => new Set());
  const [activeSessionID, setActiveSessionID] = useState<string | null>(null);
  const activeWorkspaceID = activeSessionID
    ? (panels.find((panel) => panel.id === activeSessionID)?.workspace.id ?? null)
    : null;
  const session = activeWorkspaceID
    ? (panels.find((panel) => panel.workspace.id === activeWorkspaceID) ?? null)
    : null;
  const panelsRef = useRef(panels);
  const sessionRef = useRef<SessionInfo | null>(session);
  const requestSeqRef = useRef(0);
  const activationSeqRef = useRef(0);
  const userActivatedRef = useRef(false);
  const swapPendingRef = useRef(false);
  const replacingSessionIDsRef = useRef(new Map<string, number>());
  const protectedSessionIDs = useCallback(
    (): Set<string> => new Set(panelsRef.current.map((panel) => panel.id)), []);
  const panelFor = useCallback((workspace: WorkspaceIdentity): SessionReadView | null =>
    panelsRef.current.find((panel) => sameWorkspace(panel.workspace, workspace)) ?? null, []);
  const panelForSession = useCallback((sessionID: string): SessionReadView | null =>
    panelsRef.current.find((panel) => panel.id === sessionID) ?? null, []);
  const readFocusedSession = useCallback((): SessionReadView | null => sessionRef.current, []);
  const readPanels = useCallback((): readonly SessionReadView[] => panelsRef.current, []);
  const workspaceOfSession = useCallback((sessionID: string): WorkspaceIdentity | null =>
    panelsRef.current.find((panel) => panel.id === sessionID)?.workspace ?? null, []);
  const nextRequest = useCallback(() => ++requestSeqRef.current, []);
  const hasUserActivated = useCallback(() => userActivatedRef.current, []);
  const hasPendingSwap = useCallback(() => swapPendingRef.current, []);
  const hasPendingReplacement = useCallback((sessionID: string) => replacingSessionIDsRef.current.has(sessionID), []);
  const mirrorRenderedState = useCallback((renderedPanels: readonly SessionInfo[], renderedSession: SessionInfo | null) => {
    panelsRef.current = renderedPanels as SessionInfo[];
    sessionRef.current = renderedSession;
  }, []);
  return {
    panels: panels as readonly SessionReadView[], workspaceOnlyPanelIDs: workspaceOnlyPanelIDs as ReadonlySet<string>, activeSessionID, session: session as SessionReadView | null,
    protectedSessionIDs, panelFor, panelForSession, readFocusedSession, readPanels,
    workspaceOfSession, nextRequest, hasUserActivated, hasPendingSwap, hasPendingReplacement, mirrorRenderedState,
    [lifecycleAuthority]: { setPanels, setWorkspaceOnlyPanelIDs, setActiveSessionID,
      panelsRef, sessionRef, requestSeqRef, activationSeqRef, userActivatedRef,
      swapPendingRef, replacingSessionIDsRef } as unknown
  } as const;
}

interface LifecycleInventory {
  addActiveSession: (info: SessionInfo) => void;
  removeActiveSession: (sessionID: string) => void;
  activeSessionFor: (sessionID: string) => SessionInfo | undefined;
  summaryFor: (sessionID: string) => SessionSummary | undefined;
  loadSessions: () => Promise<void>;
}
interface LifecycleBookmarks {
  rememberWorkspace: (directory: string) => void;
  replaceSavedWorkspace: (previous: string, next: string) => void;
}
interface LifecycleFiles {
  hideNestedWorkspace: (directory: string) => void;
  restoreNestedWorkspaceVisibility: (directory: string) => void;
  loadRecovery: (workspace: WorkspaceIdentity) => Promise<void>;
}
interface LifecycleEditor {
  dirtyTabsFor: (workspaceID: string) => Tab[];
  saveCapturedSnapshot: (snapshot: SaveSnapshot) => Promise<boolean>;
  setSingleFileFor: (workspaceID: string, path: string) => void;
  openFile: (path: string, workspace?: WorkspaceIdentity) => Promise<void>;
}
interface LifecycleCompletion {
  refreshInbox: (sessionID: string) => Promise<void>;
  refreshForms: (sessionID: string) => Promise<void>;
  loadModels: (workspace?: WorkspaceIdentity) => Promise<void>;
  loadAgents: (workspace?: WorkspaceIdentity) => Promise<void>;
}
interface LifecycleReplay {
  hasTranscript: (sessionID: string) => boolean;
  hydrateTranscript: (sessionID: string) => Promise<void>;
  applyReopenedSnapshot: (snapshot: ReopenedSnapshot) => void;
}
interface LifecycleExecution {
  busyFor: (sessionID: string) => boolean | undefined;
  dropClosedActivity: (sessionID: string) => void;
}
interface LifecycleHost {
  notify: (text: string, tone?: "info" | "error") => void;
  closeMenu: () => void;
  cancelPending: () => void;
  forgetClosedSessionRecords: (sessionID: string) => void;
}

export function useSessionLifecycle(
  authority: ReturnType<typeof useSessionLifecycleAuthority>,
  inventory: LifecycleInventory, bookmarks: LifecycleBookmarks, files: LifecycleFiles,
  editor: LifecycleEditor, completion: LifecycleCompletion, replay: LifecycleReplay,
  execution: LifecycleExecution, host: LifecycleHost
) {
  const { panelFor, panelForSession } = authority;
  const { setPanels, setWorkspaceOnlyPanelIDs, setActiveSessionID, panelsRef, sessionRef,
    requestSeqRef, activationSeqRef, userActivatedRef, swapPendingRef, replacingSessionIDsRef } = authority[lifecycleAuthority] as LifecycleInternals;
  const { addActiveSession, removeActiveSession, activeSessionFor, summaryFor, loadSessions } = inventory;
  const { rememberWorkspace, replaceSavedWorkspace } = bookmarks;
  const { hideNestedWorkspace, restoreNestedWorkspaceVisibility, loadRecovery } = files;
  const { dirtyTabsFor, saveCapturedSnapshot, setSingleFileFor, openFile } = editor;
  const { refreshInbox, refreshForms, loadModels, loadAgents } = completion;
  const { hasTranscript, hydrateTranscript, applyReopenedSnapshot } = replay;
  const { busyFor, dropClosedActivity } = execution;
  const { notify: toast, closeMenu: closeCtxMenu, cancelPending, forgetClosedSessionRecords } = host;
  const attachPanel = useCallback((info: SessionInfo, saveWorkspace = true): void => {
    if (saveWorkspace) rememberWorkspace(info.directory);
    panelsRef.current = panelsRef.current.some((panel) => panel.id === info.id)
      ? panelsRef.current.map((panel) => (panel.id === info.id ? info : panel))
      : [...panelsRef.current, info];
    setPanels(panelsRef.current);
    addActiveSession(info);
  }, [addActiveSession, rememberWorkspace]);

  const focusSession = useCallback((sessionID: string, hydrate = true): void => {
    const panel = panelsRef.current.find((candidate) => candidate.id === sessionID);
    if (!panel) return;
    userActivatedRef.current = true;
    sessionRef.current = panel;
    setActiveSessionID(sessionID);
    closeCtxMenu();
    cancelPending();
    if (hydrate && !hasTranscript(sessionID)) void hydrateTranscript(sessionID);
  }, [hydrateTranscript, closeCtxMenu, hasTranscript]);

  const ejectPanelFromView = useCallback((sessionID: string): SessionInfo | null => {
    const closing = panelsRef.current.find((panel) => panel.id === sessionID) ?? null;
    if (!closing) return null;
    panelsRef.current = panelsRef.current.filter((panel) => panel.id !== sessionID);
    setPanels(panelsRef.current);
    restoreNestedWorkspaceVisibility(closing.directory);
    setWorkspaceOnlyPanelIDs((current) => {
      if (!current.has(sessionID)) return current;
      const next = new Set(current);
      next.delete(sessionID);
      return next;
    });
    if (sessionRef.current?.id === sessionID) {
      const neighbor = panelsRef.current[panelsRef.current.length - 1] ?? null;
      sessionRef.current = neighbor;
      setActiveSessionID(neighbor?.id ?? null);
    }
    return closing;
  }, [restoreNestedWorkspaceVisibility]);

  const detachPanel = useCallback((sessionID: string): void => {
    // Replacement without teardown: the session leaves the visible panels
    // but its backend context, Open now entry, and chat state stay alive so
    // the user can jump back to it. Only an explicit close tears it down.
    ejectPanelFromView(sessionID);
  }, [ejectPanelFromView]);

  const finishClosePanel = useCallback((sessionID: string, preserveBusy: boolean): void => {
    const closing = panelsRef.current.find((panel) => panel.id === sessionID);
    const active = activeSessionFor(sessionID);
    if (!closing && active) {
      removeActiveSession(sessionID);
      dropClosedActivity(sessionID);
      void window.vela.closeSession(active.workspace).catch(() => {});
      return;
    }
    const ejected = ejectPanelFromView(sessionID);
    if (!ejected) return;
    const keepActive = preserveBusy && Boolean(busyFor(sessionID));
    if (!keepActive) {
      removeActiveSession(sessionID);
      forgetClosedSessionRecords(sessionID);
    }
    if (!keepActive) {
      void window.vela.closeSession(ejected.workspace).catch(() => {});
    }
  }, [removeActiveSession, ejectPanelFromView, activeSessionFor, forgetClosedSessionRecords, dropClosedActivity, busyFor]);

  const closePanel = useCallback((sessionID: string, preserveBusy = false): void => {
    const closing = panelsRef.current.find((panel) => panel.id === sessionID);
    const active = activeSessionFor(sessionID);
    const workspace = closing?.workspace ?? active?.workspace;
    const dirtyTabs = workspace
      ? dirtyTabsFor(workspace.id)
      : [];
    if (dirtyTabs.length > 0 && workspace) {
      if (dirtyTabs.some((tab) => tab.conflict)) {
        toast("Resolve external file conflicts before closing this workspace.", "error");
        return;
      }
      const snapshots = dirtyTabs.map((tab) => ({
        workspace,
        path: tab.path,
        content: tab.content,
        expectedContent: tab.saved,
        revision: tab.revision,
        standalone: tab.standalone
      }));
      void Promise.all(snapshots.map((snapshot) => saveCapturedSnapshot(snapshot))).then((saved) => {
        if (saved.every(Boolean)) finishClosePanel(sessionID, preserveBusy);
      });
      return;
    }
    finishClosePanel(sessionID, preserveBusy);
  }, [finishClosePanel, toast, activeSessionFor]);

  const replacePanels = useCallback((next: SessionInfo): void => {
    for (const panel of [...panelsRef.current]) {
      if (panel.id !== next.id) detachPanel(panel.id);
    }
    attachPanel(next);
  }, [attachPanel, detachPanel]);

  const deleteSession = useCallback(async (sessionID: string): Promise<void> => {
    const panel = panelForSession(sessionID);
    const summary = summaryFor(sessionID);
    const title = panel?.title ?? summary?.title ?? sessionID;
    if (!window.confirm(`Delete session "${title}"? This permanently removes it from opencode and cannot be undone.`)) return;
    try {
      if (panel) closePanel(sessionID);
      await window.vela.deleteSession(sessionID);
      toast(`Deleted session ${title}`);
      void loadSessions();
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [panelForSession, closePanel, toast, loadSessions, summaryFor]);

  const replacePanel = useCallback((workspace: WorkspaceIdentity, info: SessionInfo): boolean => {
    const index = panelsRef.current.findIndex((panel) => sameWorkspace(panel.workspace, workspace));
    if (index === -1) return false;
    rememberWorkspace(info.directory);
    // Detach without teardown (see detachPanel): the replaced session keeps
    // its backend context and Open now entry so it stays jumpable; only an
    // explicit close ends it.
    ejectPanelFromView(panelsRef.current[index].id);
    // The eject above already removed the old entry, so the survivors after
    // `index` have shifted down one slot.
    panelsRef.current = [
      ...panelsRef.current.slice(0, index),
      info,
      ...panelsRef.current.slice(index).filter((panel) => !sameWorkspace(panel.workspace, info.workspace))
    ];
    setPanels(panelsRef.current);
    userActivatedRef.current = true;
    sessionRef.current = info;
    setActiveSessionID(info.id);
    addActiveSession(info);
    return true;
  }, [addActiveSession, ejectPanelFromView, rememberWorkspace]);

  const launchSessionContent = useCallback((info: SessionInfo): void => {
    void hydrateTranscript(info.id);
    void refreshInbox(info.id);
    void refreshForms(info.id);
    void loadRecovery(info.workspace);
  }, [hydrateTranscript, refreshInbox, refreshForms, loadRecovery]);

  const launchSessionCatalog = useCallback((info: SessionInfo): void => {
    void loadModels(info.workspace);
    void loadAgents(info.workspace);
    void loadSessions();
  }, [loadModels, loadAgents, loadSessions]);

  const swapPanelTo = useCallback((workspace: WorkspaceIdentity, info: SessionInfo): void => {
    if (!replacePanel(workspace, info)) {
      void window.vela.closeSession(info.workspace).catch(() => {});
      return;
    }
    launchSessionContent(info);
    launchSessionCatalog(info);
  }, [replacePanel, launchSessionContent, launchSessionCatalog]);

  const openSession = useCallback(
    async (dir: string): Promise<SessionInfo | null> => {
      const request = ++requestSeqRef.current;
      const activation = ++activationSeqRef.current;
      try {
        const info = await window.vela.openSession(dir, request);
        if (activation !== activationSeqRef.current) {
          await window.vela.closeSession(info.workspace).catch(() => {});
          return null;
        }
        if (dir !== info.directory) replaceSavedWorkspace(dir, info.directory);
        userActivatedRef.current = true;
        replacePanels(info);
        sessionRef.current = info;
        setActiveSessionID(info.id);
        launchSessionContent(info);
        toast(`Opened ${info.directory}`);
        launchSessionCatalog(info);
        return info;
      } catch (err) {
        toast(err instanceof Error ? err.message : String(err), "error");
        return null;
      }
    },
    [replacePanels, replaceSavedWorkspace, toast, launchSessionContent, launchSessionCatalog]
  );

  const addModelPanel = useCallback(
    async (dir: string) => {
      const request = ++requestSeqRef.current;
      const activation = activationSeqRef.current;
      try {
        const info = await window.vela.openSession(dir, request);
        if (activation !== activationSeqRef.current) {
          await window.vela.closeSession(info.workspace).catch(() => {});
          return;
        }
        hideNestedWorkspace(dir);
        attachPanel(info);
        userActivatedRef.current = true;
        sessionRef.current = info;
        setActiveSessionID(info.id);
        launchSessionContent(info);
        launchSessionCatalog(info);
      } catch (err) {
        toast(err instanceof Error ? err.message : String(err), "error");
      }
    },
    [hideNestedWorkspace, attachPanel, toast, launchSessionContent, launchSessionCatalog]
  );

  const openWorkspacePanel = useCallback(async (dir: string): Promise<void> => {
    await addModelPanel(dir);
    const opened = sessionRef.current;
    if (!opened) return;
    setWorkspaceOnlyPanelIDs((current) => {
      const next = new Set(current);
      next.add(opened.id);
      return next;
    });
  }, [addModelPanel]);

  const selectAddPanel = useCallback(async (): Promise<SessionInfo | null> => {
    const request = ++requestSeqRef.current;
    const activation = activationSeqRef.current;
    try {
      const info = await window.vela.selectFolder(request);
      if (!info) return null;
      if (activation !== activationSeqRef.current) {
        await window.vela.closeSession(info.workspace).catch(() => {});
        return null;
      }
      attachPanel(info);
      userActivatedRef.current = true;
      sessionRef.current = info;
      setActiveSessionID(info.id);
      launchSessionContent(info);
      launchSessionCatalog(info);
      return info;
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), "error");
      return null;
    }
  }, [attachPanel, toast, launchSessionContent, launchSessionCatalog]);

  const swapPanelWorkspace = useCallback(
    async (workspace: WorkspaceIdentity, pick: (request: number) => Promise<SessionInfo | null>) => {
      const request = ++requestSeqRef.current;
      const activation = activationSeqRef.current;
      swapPendingRef.current = true;
      try {
        const info = await pick(request);
        if (!info) return;
        if (activation !== activationSeqRef.current) {
          await window.vela.closeSession(info.workspace).catch(() => {});
          return;
        }
        swapPanelTo(workspace, info);
      } catch (err) {
        if (panelFor(workspace)) toast(err instanceof Error ? err.message : String(err), "error");
      } finally {
        swapPendingRef.current = false;
      }
    },
    [swapPanelTo, toast, panelFor]
  );

  const selectPanelDirectory = useCallback(
    (workspace: WorkspaceIdentity) =>
      swapPanelWorkspace(workspace, (request) => window.vela.selectFolder(request)),
    [swapPanelWorkspace]
  );

  const changePanelDirectory = useCallback(
    (workspace: WorkspaceIdentity, dir: string) =>
      swapPanelWorkspace(workspace, (request) => window.vela.openSession(dir, request)),
    [swapPanelWorkspace]
  );

  const selectFolder = useCallback(async () => {
    const request = ++requestSeqRef.current;
    const activation = ++activationSeqRef.current;
    try {
      const info = await window.vela.selectFolder(request);
      if (info) {
        if (activation !== activationSeqRef.current) {
          await window.vela.closeSession(info.workspace).catch(() => {});
          return;
        }
        userActivatedRef.current = true;
        replacePanels(info);
        sessionRef.current = info;
        setActiveSessionID(info.id);
        launchSessionContent(info);
        toast(`Opened ${info.directory}`);
        launchSessionCatalog(info);
      }
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [replacePanels, toast, launchSessionContent, launchSessionCatalog]);

  const reopenSession = useCallback(
    async (sessionID: string, silent = false, directoryHint?: string): Promise<SessionInfo | null> => {
      const existing = panelForSession(sessionID);
      if (existing) {
        if (!hasTranscript(sessionID)) void hydrateTranscript(sessionID);
        if (!silent) {
          focusSession(sessionID);
          void loadSessions();
        }
        return existing;
      }
      const request = ++requestSeqRef.current;
      const activation = silent ? 0 : ++activationSeqRef.current;
      const targetWorkspace = silent ? null : sessionRef.current?.workspace ?? null;
      const previousDirectory = directoryHint ?? summaryFor(sessionID)?.directory;
      if (!silent) {
        replacingSessionIDsRef.current.set(sessionID, (replacingSessionIDsRef.current.get(sessionID) ?? 0) + 1);
      }
      try {
        const reopened = silent
          ? await window.vela.openSessionById(sessionID, request)
          : await window.vela.openSessionById(sessionID, request, undefined, true);
        if (!silent && activation !== activationSeqRef.current) {
          await window.vela.closeSession(reopened.session.workspace).catch(() => {});
          return null;
        }
        if (silent) {
          attachPanel(reopened.session, false);
        } else if (targetWorkspace) {
          if (!replacePanel(targetWorkspace, reopened.session)) {
            await window.vela.closeSession(reopened.session.workspace).catch(() => {});
            return null;
          }
        } else {
          replacePanels(reopened.session);
          userActivatedRef.current = true;
          sessionRef.current = reopened.session;
          setActiveSessionID(reopened.session.id);
        }
        if (!silent && previousDirectory && previousDirectory !== reopened.session.directory) {
          replaceSavedWorkspace(previousDirectory, reopened.session.directory);
        }
        void loadRecovery(reopened.session.workspace);
        applyReopenedSnapshot(reopened);
        void loadModels(reopened.session.workspace);
        void loadAgents(reopened.session.workspace);
        void loadSessions();
        return reopened.session;
      } catch (err) {
        if (!silent) toast(err instanceof Error ? err.message : String(err), "error");
        return null;
      } finally {
        if (!silent) {
          const pending = replacingSessionIDsRef.current.get(sessionID) ?? 0;
          if (pending <= 1) replacingSessionIDsRef.current.delete(sessionID);
          else replacingSessionIDsRef.current.set(sessionID, pending - 1);
        }
      }
    },
    [
    attachPanel,
    replacePanels,
    replacePanel,
    replaceSavedWorkspace,
    focusSession,
    panelForSession,
    hasTranscript,
    toast,
    loadModels,
    loadAgents,
    loadSessions,
    loadRecovery,
    hydrateTranscript,
    summaryFor,
    applyReopenedSnapshot,
  ]
  );

  const attachFileWorkspace = useCallback(
    async (result: OpenFileWorkspaceResult, activation: number): Promise<SessionInfo | null> => {
      if (activation !== activationSeqRef.current) {
        await window.vela.closeSession(result.session.workspace).catch(() => {});
        return null;
      }
      userActivatedRef.current = true;
      replacePanels(result.session);
      sessionRef.current = result.session;
      setActiveSessionID(result.session.id);
      setSingleFileFor(result.session.workspace.id, result.path);
      void hydrateTranscript(result.session.id);
      void loadRecovery(result.session.workspace);
      void loadModels(result.session.workspace);
      void loadAgents(result.session.workspace);
      void loadSessions();
      await openFile(result.path, result.session.workspace);
      return result.session;
    },
    [replacePanels, setSingleFileFor, hydrateTranscript, loadRecovery, loadModels, loadAgents, loadSessions, openFile]
  );
  const selectFile = useCallback(
    async (): Promise<void> => {
      const request = ++requestSeqRef.current;
      const activation = ++activationSeqRef.current;
      try {
        const result = await window.vela.selectFile(request);
        if (!result) return;
        const info = await attachFileWorkspace(result, activation);
        if (info) toast(`Opened ${result.path}`);
      } catch (err) {
        toast(err instanceof Error ? err.message : String(err), "error");
      }
    },
    [attachFileWorkspace, toast]
  );
  const openFileWorkspace = useCallback(
    async (file: string): Promise<SessionInfo | null> => {
      const request = ++requestSeqRef.current;
      const activation = ++activationSeqRef.current;
      try {
        const result = await window.vela.openFileWorkspace(file, request);
        const info = await attachFileWorkspace(result, activation);
        if (info) toast(`Opened ${result.path}`);
        return info;
      } catch (err) {
        toast(err instanceof Error ? err.message : String(err), "error");
        return null;
      }
    },
    [attachFileWorkspace, toast]
  );
  return { attachPanel, focusSession, closePanel, deleteSession, openSession, addModelPanel,
    openWorkspacePanel, selectAddPanel, selectPanelDirectory, changePanelDirectory,
    selectFolder, reopenSession, selectFile, openFileWorkspace };
}
