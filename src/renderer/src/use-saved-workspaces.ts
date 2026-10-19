import { useCallback, useEffect, useRef, useState } from "react";
import type { ProjectInfo } from "@shared/types";

const SAVED_WORKSPACES_KEY = "vela.savedWorkspaces";

function workspaceName(directory: string): string {
  return directory.split(/[\\/]/).filter(Boolean).pop() ?? directory;
}

function readSavedWorkspaceData(): { workspaces: ProjectInfo[]; present: boolean } {
  try {
    const raw = window.localStorage.getItem(SAVED_WORKSPACES_KEY);
    if (raw === null) return { workspaces: [], present: false };
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return { workspaces: [], present: false };
    const seen = new Set<string>();
    const workspaces = parsed.flatMap((item): ProjectInfo[] => {
      if (!item || typeof item !== "object") return [];
      const value = item as { directory?: unknown; name?: unknown };
      if (typeof value.directory !== "string" || !value.directory || seen.has(value.directory)) return [];
      seen.add(value.directory);
      return [{
        directory: value.directory,
        name: typeof value.name === "string" && value.name.trim() ? value.name : workspaceName(value.directory)
      }];
    });
    return { workspaces, present: true };
  } catch {
    return { workspaces: [], present: false };
  }
}

function readSavedWorkspaces(): ProjectInfo[] {
  return readSavedWorkspaceData().workspaces;
}

function mergeSavedWorkspaces(...lists: ProjectInfo[][]): ProjectInfo[] {
  const merged = new Map<string, ProjectInfo>();
  for (const list of lists) {
    for (const workspace of list) {
      if (!workspace?.directory || merged.has(workspace.directory)) continue;
      merged.set(workspace.directory, {
        directory: workspace.directory,
        name: workspace.name?.trim() || workspaceName(workspace.directory)
      });
    }
  }
  return [...merged.values()];
}

export function useSavedWorkspaces(connected: boolean, notify: (text: string, tone?: "info" | "error") => void) {
  const [savedWorkspaces, setSavedWorkspaces] = useState<ProjectInfo[]>(() => readSavedWorkspaces());
  const savedWorkspacesRef = useRef(savedWorkspaces);
  savedWorkspacesRef.current = savedWorkspaces;
  const savedWorkspacesHydratedRef = useRef(false);
  const pendingWorkspaceUpdatesRef = useRef<Array<(current: ProjectInfo[]) => ProjectInfo[]>>([]);
  const workspaceRevisionRef = useRef(0);
  const updateSavedWorkspaces = useCallback((update: (current: ProjectInfo[]) => ProjectInfo[]): void => {
    if (!savedWorkspacesHydratedRef.current) pendingWorkspaceUpdatesRef.current.push(update);
    workspaceRevisionRef.current += 1;
    const next = update(savedWorkspacesRef.current);
    savedWorkspacesRef.current = next;
    setSavedWorkspaces(next);
  }, []);
  const rememberWorkspace = useCallback((directory: string): void => {
    if (!directory) return;
    const saved = { directory, name: workspaceName(directory) };
    updateSavedWorkspaces((current) =>
      current.some((workspace) => workspace.directory === directory) ? current : [...current, saved]
    );
  }, [updateSavedWorkspaces]);

  const replaceSavedWorkspace = useCallback((previousDirectory: string, directory: string): void => {
    if (!directory) return;
    const saved = { directory, name: workspaceName(directory) };
    updateSavedWorkspaces((current) => {
      const remaining = current.filter((workspace) => workspace.directory !== previousDirectory);
      return remaining.some((workspace) => workspace.directory === directory)
        ? remaining
        : [...remaining, saved];
    });
  }, [updateSavedWorkspaces]);

  useEffect(() => {
    let cancelled = false;
    const legacy = readSavedWorkspaceData();
    const revision = workspaceRevisionRef.current;
    const storedWorkspaces = window.vela.savedWorkspaces?.();
    if (!storedWorkspaces) {
      notify("Restart Vela to load saved workspaces from app storage.", "error");
      return () => { cancelled = true; };
    }
    void storedWorkspaces
      .then(async (stored) => {
        const projects = !stored.initialized && !legacy.present
          ? await window.vela.projects().catch(() => [])
          : [];
        if (cancelled) return;
        if (savedWorkspacesHydratedRef.current && revision !== workspaceRevisionRef.current) return;
        let merged = stored.initialized
          ? stored.workspaces
          : mergeSavedWorkspaces(legacy.workspaces, projects);
        for (const update of pendingWorkspaceUpdatesRef.current) merged = update(merged);
        pendingWorkspaceUpdatesRef.current = [];
        savedWorkspacesRef.current = merged;
        savedWorkspacesHydratedRef.current = true;
        setSavedWorkspaces(merged);
      })
      .catch((error) => {
        if (cancelled) return;
        const detail = error instanceof Error ? error.message : String(error);
        if (/No handler registered for ['"]?shell:saved-workspaces['"]?/i.test(detail)) {
          notify("Vela's app process is older than this window. Quit and reopen Vela completely; Cmd+R only refreshes the window. Saved workspaces are still stored.", "error");
          return;
        }
        notify(`Saved workspaces could not be loaded; their stored data was left untouched. ${detail}`, "error");
      });
    return () => { cancelled = true; };
  }, [connected, notify]);

  useEffect(() => {
    if (!savedWorkspacesHydratedRef.current) return;
    try {
      window.localStorage.setItem(SAVED_WORKSPACES_KEY, JSON.stringify(savedWorkspaces));
    } catch {}
    const save = window.vela.saveSavedWorkspaces?.(savedWorkspaces);
    if (!save) return;
    void save.catch((error) => {
      const detail = error instanceof Error ? error.message : String(error);
      notify(`Saved workspaces could not be written to app storage. ${detail}`, "error");
    });
  }, [savedWorkspaces, notify]);

  const saveWorkspace = useCallback(async (): Promise<void> => {
    try {
      const directory = await window.vela.selectDirectory();
      if (!directory) return;
      const saved = { directory, name: workspaceName(directory) };
      updateSavedWorkspaces((current) => {
        if (current.some((workspace) => workspace.directory === directory)) return current;
        return [...current, saved];
      });
      notify(`Saved workspace ${saved.name}`);
    } catch (err) {
      notify(err instanceof Error ? err.message : String(err), "error");
    }
  }, [notify, updateSavedWorkspaces]);

  const removeWorkspace = useCallback((directory: string): void => {
    updateSavedWorkspaces((current) => current.filter((workspace) => workspace.directory !== directory));
  }, [updateSavedWorkspaces]);

  return { savedWorkspaces, rememberWorkspace, replaceSavedWorkspace, saveWorkspace, removeWorkspace };
}
