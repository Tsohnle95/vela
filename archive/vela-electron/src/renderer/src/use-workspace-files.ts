import { useCallback, useEffect, useRef, useState } from "react";
import { HIDDEN_RECOVERY_DIRS, type AgentFileState, type FileUpdate, type RecoveryRecord, type SessionInfo, type TreeEntry, type WorkspaceIdentity } from "@shared/types";

type ObservedFileView = Readonly<Omit<AgentFileState, "baseline">> & {
  readonly baseline: Readonly<AgentFileState["baseline"]>;
};

interface WorkspaceFilesHost {
  renderSession: SessionInfo | null;
  readFocusedSession: () => SessionInfo | null;
  panelFor: (workspace: WorkspaceIdentity) => SessionInfo | null;
  readPanels: () => readonly SessionInfo[];
  notify: (text: string, tone?: "info" | "error") => void;
}

const HIDDEN_DIRS = new Set([
  ".git", "node_modules", ".next", ".venv", "__pycache__", ".cache", ".turbo", ".svn", ".hg", ".nx", ...HIDDEN_RECOVERY_DIRS
]);

function orderedEntries(entries: TreeEntry[]): TreeEntry[] {
  return entries.filter((e) => {
    const name = e.path.split("/").pop() ?? e.path;
    if (e.type === "directory") return !HIDDEN_DIRS.has(name);
    return name !== ".DS_Store";
  }).sort((a, b) => {
    if (a.type !== b.type) return a.type === "directory" ? -1 : 1;
    return a.path.localeCompare(b.path);
  });
}

const EMPTY_AGENT_FILES: Map<string, AgentFileState> = new Map();
const EMPTY_TREE: Record<string, TreeEntry[]> = {};
const EMPTY_EXPANDED: Set<string> = new Set();
export function useWorkspaceFiles(host: WorkspaceFilesHost) {
  const { renderSession: session, readFocusedSession, panelFor, readPanels, notify: toast } = host;
  const [agentFilesByWorkspace, setAgentFilesByWorkspace] = useState<Record<string, Map<string, AgentFileState>>>({});
  const [treeByWorkspace, setTreeByWorkspace] = useState<Record<string, Record<string, TreeEntry[]>>>({});
  const [expandedByWorkspace, setExpandedByWorkspace] = useState<Record<string, Set<string>>>({});
  const [hiddenPathsByWorkspace, setHiddenPathsByWorkspace] = useState<Record<string, Set<string>>>({});
  const [recoveryByWorkspace, setRecoveryByWorkspace] = useState<Record<string, RecoveryRecord[]>>({});
  const agentFilesByWorkspaceRef = useRef(agentFilesByWorkspace);
  agentFilesByWorkspaceRef.current = agentFilesByWorkspace;
  const expandedByWorkspaceRef = useRef(expandedByWorkspace);
  expandedByWorkspaceRef.current = expandedByWorkspace;
  const treeRefreshTimersRef = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const prefetchingRef = useRef(new Set<string>());
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; prefetchingRef.current.clear(); };
  }, []);
  const agentFiles = session ? agentFilesByWorkspace[session.workspace.id] ?? EMPTY_AGENT_FILES : EMPTY_AGENT_FILES;
  const tree = session ? treeByWorkspace[session.workspace.id] ?? EMPTY_TREE : EMPTY_TREE;
  const expanded = session ? expandedByWorkspace[session.workspace.id] ?? EMPTY_EXPANDED : EMPTY_EXPANDED;
  const hiddenPaths = session ? hiddenPathsByWorkspace[session.workspace.id] ?? EMPTY_EXPANDED : EMPTY_EXPANDED;
  const recoveryRecords = session ? recoveryByWorkspace[session.workspace.id] ?? [] : [];
  const setAgentFilesFor = useCallback((
    workspaceID: string,
    update: Map<string, AgentFileState> | ((prev: Map<string, AgentFileState>) => Map<string, AgentFileState>)
  ) => {
    setAgentFilesByWorkspace((current) => {
      const previous = current[workspaceID] ?? new Map<string, AgentFileState>();
      const next = typeof update === "function" ? update(previous) : update;
      return previous === next ? current : { ...current, [workspaceID]: next };
    });
  }, []);

  const setTreeFor = useCallback((workspaceID: string, update: (prev: Record<string, TreeEntry[]>) => Record<string, TreeEntry[]>) => {
    setTreeByWorkspace((current) => {
      const next = update(current[workspaceID] ?? {});
      return next === (current[workspaceID] ?? {}) ? current : { ...current, [workspaceID]: next };
    });
  }, []);

  const setExpandedFor = useCallback((workspaceID: string, next: Set<string>) => {
    setExpandedByWorkspace((current) =>
      current[workspaceID] === next ? current : { ...current, [workspaceID]: next });
  }, []);

  const setRecoveryFor = useCallback((workspaceID: string, records: RecoveryRecord[]) => {
    setRecoveryByWorkspace((current) =>
      current[workspaceID] === records ? current : { ...current, [workspaceID]: records });
  }, []);

  const hidePath = useCallback((workspace: WorkspaceIdentity, path: string): void => {
    setHiddenPathsByWorkspace((current) => {
      const next = new Set(current[workspace.id] ?? []);
      next.add(path);
      return { ...current, [workspace.id]: next };
    });
  }, []);

  const restoreDetachedPath = useCallback((workspace: WorkspaceIdentity, path: string): void => {
    setHiddenPathsByWorkspace((current) => {
      const next = new Set(current[workspace.id] ?? []);
      next.delete(path);
      return { ...current, [workspace.id]: next };
    });
  }, []);

  const hideNestedWorkspace = useCallback((directory: string): void => {
    const source = directory.replaceAll("\\", "/").replace(/\/+$/, "");
    for (const panel of readPanels()) {
      const parent = panel.directory.replaceAll("\\", "/").replace(/\/+$/, "");
      if (!source.startsWith(`${parent}/`)) continue;
      const relative = source.slice(parent.length + 1);
      hidePath(panel.workspace, relative);
    }
  }, [readPanels, hidePath]);

  const restoreNestedWorkspaceVisibility = useCallback((directory: string): void => {
    const childDirectory = directory.replaceAll("\\", "/").replace(/\/+$/, "");
    setHiddenPathsByWorkspace((current) => {
      let changed = false;
      const next = { ...current };
      for (const panel of readPanels()) {
        const parentDirectory = panel.directory.replaceAll("\\", "/").replace(/\/+$/, "");
        if (!childDirectory.startsWith(`${parentDirectory}/`)) continue;
        const relative = childDirectory.slice(parentDirectory.length + 1);
        const paths = current[panel.workspace.id];
        if (!paths?.has(relative)) continue;
        const restored = new Set(paths);
        restored.delete(relative);
        next[panel.workspace.id] = restored;
        changed = true;
      }
      return changed ? next : current;
    });
  }, [readPanels]);

  const isHiddenPath = useCallback((workspace: WorkspaceIdentity, path: string): boolean =>
    Boolean(hiddenPathsByWorkspace[workspace.id]?.has(path)), [hiddenPathsByWorkspace]);

  const readAgentFile = useCallback((workspace: WorkspaceIdentity, path: string): ObservedFileView | undefined =>
    agentFilesByWorkspaceRef.current[workspace.id]?.get(path), []);

  const expandParent = useCallback((workspace: WorkspaceIdentity, parent: string): void => {
    const current = expandedByWorkspaceRef.current[workspace.id] ?? new Set<string>();
    if (current.has(parent)) return;
    const next = new Set(current);
    next.add(parent);
    setExpandedFor(workspace.id, next);
  }, [setExpandedFor]);

  const recordCreatedEntry = useCallback((workspace: WorkspaceIdentity, parent: string, path: string, kind: "file" | "dir"): void => {
    setTreeFor(workspace.id, (prev) => {
      const current = prev[parent] ?? [];
      if (current.some((entry) => entry.path === path)) return prev;
      const entry: TreeEntry = { path, type: kind === "file" ? "file" : "directory" };
      return { ...prev, [parent]: orderedEntries([...current, entry]) };
    });
  }, [setTreeFor]);

  const renameObservedPrefix = useCallback((workspace: WorkspaceIdentity, path: string, newPath: string): void => {
    const current = agentFilesByWorkspaceRef.current[workspace.id] ?? new Map<string, AgentFileState>();
    const next = new Map<string, AgentFileState>();
    for (const [p, state] of current) {
      if (p === path || p.startsWith(`${path}/`)) {
        next.set(`${newPath}${p.slice(path.length)}`, state);
      } else {
        next.set(p, state);
      }
    }
    setAgentFilesFor(workspace.id, next);
  }, [setAgentFilesFor]);

  const moveObservedPrefix = useCallback((workspace: WorkspaceIdentity, path: string, newPath: string): void => {
    const current = agentFilesByWorkspaceRef.current[workspace.id] ?? new Map<string, AgentFileState>();
    const next = new Map<string, AgentFileState>();
    for (const [p, state] of current) {
      if (p === path || p.startsWith(`${path}/`)) {
        if (!state.deleted) next.set(`${newPath}${p.slice(path.length)}`, state);
      } else {
        next.set(p, state);
      }
    }
    setAgentFilesFor(workspace.id, next);
  }, [setAgentFilesFor]);

  const moveObservedPaths = useCallback((workspace: WorkspaceIdentity, remap: (path: string) => string): void => {
    setAgentFilesFor(workspace.id, (current) => {
      const next = new Map<string, AgentFileState>();
      for (const [path, state] of current) {
        const movedPath = remap(path);
        if (!state.deleted) next.set(movedPath, state);
      }
      return next;
    });
  }, [setAgentFilesFor]);

  const receiveFileUpdate = useCallback((f: FileUpdate): void => {
    setAgentFilesFor(f.workspace.id, (current) => {
      const next = new Map(current);
      const clean = f.baseline.kind === "known" && (
        (!f.deleted && f.content === f.baseline.content) ||
        (f.deleted && f.baseline.exists === false)
      );
      if (clean) next.delete(f.path);
      else next.set(f.path, { baseline: f.baseline, content: f.content, deleted: f.deleted });
      return next;
    });
  }, [setAgentFilesFor]);

  const receiveRecovery = useCallback((workspace: WorkspaceIdentity, records: RecoveryRecord[]): void => {
    setRecoveryFor(workspace.id, records);
  }, [setRecoveryFor]);

  const applyDirectoryListing = useCallback((workspace: WorkspaceIdentity, path: string, entries: TreeEntry[]): void => {
    if (!panelFor(workspace)) return;
    setTreeFor(workspace.id, (prev) => ({ ...prev, [path]: orderedEntries(entries) }));
  }, [panelFor, setTreeFor]);

  const scheduleFileRefresh = useCallback((f: FileUpdate): void => {
    const parents = new Set([
      f.path.includes("/") ? f.path.slice(0, f.path.lastIndexOf("/")) : "",
      ...(f.movedFrom
        ? [f.movedFrom.includes("/") ? f.movedFrom.slice(0, f.movedFrom.lastIndexOf("/")) : ""]
        : [])
    ]);
    const workspaceExpanded = expandedByWorkspaceRef.current[f.workspace.id] ?? new Set<string>();
    for (const parent of parents) {
      if (parent === f.path || !workspaceExpanded.has(parent)) continue;
      const expected = f.workspace;
      const key = `${expected.id}:${parent}`;
      const existing = treeRefreshTimersRef.current.get(key);
      if (existing) clearTimeout(existing);
      treeRefreshTimersRef.current.set(key, setTimeout(() => {
        treeRefreshTimersRef.current.delete(key);
        void window.vela.listDir(expected, parent)
          .then((entries) => applyDirectoryListing(expected, parent, entries))
          .catch(() => {});
      }, 50));
    }
  }, [applyDirectoryListing]);

  const loadRecovery = useCallback(async (workspace: WorkspaceIdentity) => {
    try {
      const records = await window.vela.recoveryRecords(workspace);
      if (panelFor(workspace)) receiveRecovery(workspace, records);
    } catch (error) {
      if (panelFor(workspace)) {
        toast(error instanceof Error ? error.message : String(error), "error");
      }
    }
  }, [toast, panelFor, receiveRecovery]);

  const toggleDir = useCallback(
    async (path: string, workspace?: WorkspaceIdentity) => {
      const target = workspace ?? readFocusedSession()?.workspace;
      if (!target) return;
      const current = expandedByWorkspaceRef.current[target.id] ?? new Set<string>();
      const isOpen = current.has(path);
      setExpandedFor(target.id, (() => {
        const next = new Set(current);
        if (isOpen) next.delete(path);
        else next.add(path);
        return next;
      })());
      if (isOpen) return;
      if (!treeByWorkspace[target.id]?.[path]) {
        try {
          const entries = await window.vela.listDir(target, path);
          applyDirectoryListing(target, path, entries);
        } catch (err) {
          toast(err instanceof Error ? err.message : String(err), "error");
        }
      }
    },
    [treeByWorkspace, toast, readFocusedSession, setExpandedFor, applyDirectoryListing]
  );

  const prefetchDir = useCallback((path: string, workspace?: WorkspaceIdentity): void => {
    const target = workspace ?? readFocusedSession()?.workspace;
    if (!mountedRef.current || !target || !panelFor(target) || treeByWorkspace[target.id]?.[path]) return;
    const key = JSON.stringify([target.id, target.generation, path]);
    if (prefetchingRef.current.has(key)) return;
    prefetchingRef.current.add(key);
    void window.vela.listDir(target, path).then((entries) => {
      if (!mountedRef.current || !panelFor(target)) return;
      setTreeFor(target.id, (prev) => !mountedRef.current || !panelFor(target) || prev[path]
        ? prev : { ...prev, [path]: orderedEntries(entries) });
    }).catch(() => {}).finally(() => prefetchingRef.current.delete(key));
  }, [readFocusedSession, panelFor, treeByWorkspace, setTreeFor]);

  const ensureRootOpen = useCallback(
    async (workspace?: WorkspaceIdentity): Promise<void> => {
      const target = workspace ?? readFocusedSession()?.workspace;
      if (!target) return;
      const current = expandedByWorkspaceRef.current[target.id] ?? new Set<string>();
      const wasOpen = current.has("");
      if (!wasOpen) setExpandedFor(target.id, new Set([...current, ""]));
      if (wasOpen || !treeByWorkspace[target.id]?.[""]) {
        try {
          const entries = await window.vela.listDir(target, "");
          applyDirectoryListing(target, "", entries);
        } catch (err) {
          toast(err instanceof Error ? err.message : String(err), "error");
        }
      }
    },
    [treeByWorkspace, toast, readFocusedSession, setExpandedFor, applyDirectoryListing]
  );

  const refreshTree = useCallback(async (dirs: string[], workspace?: WorkspaceIdentity): Promise<void> => {
    const unique = [...new Set(dirs)];
    const target = workspace ?? readFocusedSession()?.workspace;
    if (!target) return;
    const current = expandedByWorkspaceRef.current[target.id] ?? new Set<string>();
    if (current.has("") && !unique.includes("")) unique.push("");
    await Promise.all(
      unique.map(async (dir) => {
        try {
          const entries = await window.vela.listDir(target, dir);
          applyDirectoryListing(target, dir, entries);
        } catch {
          /* keep previous listing */
        }
      })
    );
  }, [readFocusedSession, applyDirectoryListing]);

  const unhidePath = useCallback((path: string): void => {
    const target = readFocusedSession();
    if (!target || !hiddenPathsByWorkspace[target.workspace.id]?.has(path)) return;
    setHiddenPathsByWorkspace((current) => {
      const next = new Set(current[target.workspace.id] ?? []);
      next.delete(path);
      return { ...current, [target.workspace.id]: next };
    });
  }, [hiddenPathsByWorkspace, readFocusedSession]);

  const dismissChange = useCallback((path: string): void => {
    const target = readFocusedSession()?.workspace;
    if (!target) return;
    const current = agentFilesByWorkspaceRef.current[target.id];
    if (!current?.has(path)) return;
    const next = new Map(current);
    next.delete(path);
    setAgentFilesFor(target.id, next);
  }, [setAgentFilesFor, readFocusedSession]);
  const dismissChanges = useCallback((): void => {
    const target = readFocusedSession()?.workspace;
    if (!target) return;
    const current = agentFilesByWorkspaceRef.current[target.id];
    if (!current || current.size === 0) return;
    setAgentFilesFor(target.id, new Map());
  }, [setAgentFilesFor, readFocusedSession]);
  const openRecovery = useCallback(async (id: string) => {
    const target = readFocusedSession()?.workspace;
    if (!target) return;
    try {
      await window.vela.openRecovery(target, id);
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }, [toast, readFocusedSession]);

  const acknowledgeRecovery = useCallback(async (id: string) => {
    const target = readFocusedSession()?.workspace;
    if (!target) return;
    try {
      await window.vela.acknowledgeRecovery(target, id);
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }, [toast, readFocusedSession]);

  const workspaceFiles = useCallback((workspace: WorkspaceIdentity) => ({
    tree: treeByWorkspace[workspace.id] ?? EMPTY_TREE,
    expanded: expandedByWorkspace[workspace.id] ?? EMPTY_EXPANDED,
    hiddenPaths: hiddenPathsByWorkspace[workspace.id] ?? EMPTY_EXPANDED,
    agentFiles: agentFilesByWorkspace[workspace.id] ?? EMPTY_AGENT_FILES
  }), [treeByWorkspace, expandedByWorkspace, hiddenPathsByWorkspace, agentFilesByWorkspace]);

  return {
    workspaceFiles, agentFiles, tree, expanded, hiddenPaths, recoveryRecords,
    readAgentFile, isHiddenPath, toggleDir, prefetchDir, ensureRootOpen, refreshTree, unhidePath,
    dismissChange, dismissChanges, loadRecovery, openRecovery, acknowledgeRecovery,
    hidePath, restoreDetachedPath, hideNestedWorkspace, restoreNestedWorkspaceVisibility,
    expandParent, recordCreatedEntry, renameObservedPrefix, moveObservedPrefix, moveObservedPaths,
    receiveFileUpdate, receiveRecovery, scheduleFileRefresh
  };
}
