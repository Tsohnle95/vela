import { useCallback, useRef, useState } from "react";
import type { SessionInfo, WorkspaceIdentity } from "@shared/types";
import type { PendingCreate } from "./store-contract";
import { compactExplorerSelection, movableExplorerPaths } from "./explorer-selection";

interface ExplorerHost {
  readFocusedSession: () => SessionInfo | null;
  panelFor: (workspace: WorkspaceIdentity) => SessionInfo | null;
  notify: (text: string, tone?: "info" | "error") => void;
  closeMenu: () => void;
}

interface ExplorerFiles {
  expandParent: (workspace: WorkspaceIdentity, parent: string) => void;
  hidePath: (workspace: WorkspaceIdentity, path: string) => void;
  restoreDetachedPath: (workspace: WorkspaceIdentity, path: string) => void;
  refreshTree: (dirs: string[], workspace?: WorkspaceIdentity) => Promise<void>;
  isHiddenPath: (workspace: WorkspaceIdentity, path: string) => boolean;
  unhidePath: (path: string) => void;
  recordCreatedEntry: (workspace: WorkspaceIdentity, parent: string, path: string, kind: "file" | "dir") => void;
  renameObservedPrefix: (workspace: WorkspaceIdentity, path: string, newPath: string) => void;
  moveObservedPrefix: (workspace: WorkspaceIdentity, path: string, newPath: string) => void;
  moveObservedPaths: (workspace: WorkspaceIdentity, remap: (path: string) => string) => void;
}

interface ExplorerEditor {
  cancelPrefix: (workspace: WorkspaceIdentity, path: string) => void;
  addCreatedFile: (workspace: WorkspaceIdentity, path: string) => void;
  removePathTabs: (workspace: WorkspaceIdentity, path: string) => void;
  removePathsTabs: (workspace: WorkspaceIdentity, paths: string[]) => void;
  renameTabs: (workspace: WorkspaceIdentity, path: string, newPath: string, name: string, capturedActive: string | null) => void;
  movePathTabs: (workspace: WorkspaceIdentity, path: string, newPath: string, name: string) => void;
  movePathsTabs: (workspace: WorkspaceIdentity, moved: Array<{ path: string; newPath: string; name: string }>) => ((path: string) => string);
  readRenderedActivePath: (workspace: WorkspaceIdentity) => string | null;
}

function ancestorDirs(path: string): string[] {
  const parts = path.split("/");
  const out: string[] = [];
  for (let i = parts.length; i > 0; i--) out.push(parts.slice(0, i).join("/"));
  return out;
}

export function useExplorerMutations(host: ExplorerHost, files: ExplorerFiles, editor: ExplorerEditor) {
  const { readFocusedSession, panelFor, notify: toast, closeMenu: closeCtxMenu } = host;
  const {
    expandParent, hidePath, restoreDetachedPath, refreshTree, isHiddenPath, unhidePath,
    recordCreatedEntry, renameObservedPrefix, moveObservedPrefix, moveObservedPaths
  } = files;
  const {
    cancelPrefix, addCreatedFile, removePathTabs, removePathsTabs,
    renameTabs, movePathTabs, movePathsTabs, readRenderedActivePath
  } = editor;
  const [pendingCreate, setPendingCreate] = useState<PendingCreate | null>(null);
  const [pendingRename, setPendingRename] = useState<{ path: string } | null>(null);
  const pendingCreateRef = useRef(pendingCreate);
  pendingCreateRef.current = pendingCreate;
  const pendingRenameRef = useRef(pendingRename);
  pendingRenameRef.current = pendingRename;

  const removeFromWorkspace = useCallback((path: string) => {
    const target = readFocusedSession()?.workspace;
    if (!target) return;
    const confirmed = window.confirm(`Remove "${path}" from Vela? The item will remain on disk.`);
    if (!confirmed) return;
    closeCtxMenu();
    hidePath(target, path);
    void window.vela.detachPath(target, path).then(() => {
      restoreDetachedPath(target, path);
      void refreshTree(ancestorDirs(path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : ""));
    }).catch((error) => toast(error instanceof Error ? error.message : String(error), "error"));
  }, [closeCtxMenu, refreshTree, toast, hidePath, restoreDetachedPath, readFocusedSession]);

  const startCreate = useCallback((parent: string, kind: "file" | "dir") => {
    closeCtxMenu();
    setPendingRename(null);
    setPendingCreate({ parent, kind });
    const target = readFocusedSession()?.workspace;
    if (!target) return;
    expandParent(target, parent);
  }, [expandParent, closeCtxMenu, readFocusedSession]);

  const startRename = useCallback((path: string) => {
    closeCtxMenu();
    setPendingCreate(null);
    setPendingRename({ path });
  }, [closeCtxMenu]);

  const revealInFileManager = useCallback(async (path: string): Promise<void> => {
    closeCtxMenu();
    const target = readFocusedSession()?.workspace;
    if (!target) return;
    try {
      await window.vela.revealInFileManager(target, path);
    } catch (err) {
      if (panelFor(target)) {
        toast(err instanceof Error ? err.message : String(err), "error");
      }
    }
  }, [closeCtxMenu, panelFor, toast, readFocusedSession]);

  const cancelPending = useCallback(() => {
    setPendingCreate(null);
    setPendingRename(null);
  }, []);

  const deleteEntry = useCallback(
    async (path: string, workspace?: WorkspaceIdentity) => {
      closeCtxMenu();
      const target = workspace ?? readFocusedSession()?.workspace;
      if (!target) return;
      cancelPrefix(target, path);
      try {
        await window.vela.deletePath(target, path);
      } catch (err) {
        if (panelFor(target)) {
          toast(err instanceof Error ? err.message : String(err), "error");
        }
        return;
      }
      if (!panelFor(target)) return;
      removePathTabs(target, path);
      const parent = path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
      void refreshTree(ancestorDirs(parent), target);
    },
    [toast, refreshTree, cancelPrefix, panelFor, removePathTabs, closeCtxMenu, readFocusedSession]
  );

  const deleteEntries = useCallback(async (paths: string[]): Promise<void> => {
    const workspace = readFocusedSession()?.workspace;
    if (!workspace) return;
    closeCtxMenu();
    const selected = compactExplorerSelection(paths);
    const deleted: string[] = [];
    for (const path of selected) {
      cancelPrefix(workspace, path);
      try {
        await window.vela.deletePath(workspace, path);
        deleted.push(path);
      } catch (err) {
        if (panelFor(workspace)) toast(err instanceof Error ? err.message : String(err), "error");
      }
    }
    if (deleted.length === 0 || !panelFor(workspace)) return;
    removePathsTabs(workspace, deleted);
    const parents = deleted.map((path) => path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "");
    void refreshTree(parents.flatMap((parent) => ancestorDirs(parent)), workspace);
  }, [closeCtxMenu, cancelPrefix, panelFor, toast, removePathsTabs, refreshTree, readFocusedSession]);

  const importPaths = useCallback(
    async (destDir: string, sources: string[]): Promise<void> => {
      const target = readFocusedSession()?.workspace;
      if (!target || sources.length === 0) return;
      try {
        const results = await window.vela.importExternal(target, destDir, sources);
        if (!panelFor(target)) return;
        for (const result of results) {
          if (result.imported) toast(`Imported ${result.rel}`);
          else if (isHiddenPath(target, result.rel)) {
            unhidePath(result.rel);
            continue;
          }
          else if (result.reason === "already exists" || result.reason === "already in the workspace") continue;
          else toast(`Could not import ${result.name}: ${result.reason ?? "unknown"}`, "error");
        }
        void refreshTree([...ancestorDirs(destDir)]);
      } catch (err) {
        if (panelFor(target)) toast(err instanceof Error ? err.message : String(err), "error");
      }
    },
    [toast, panelFor, refreshTree, isHiddenPath, unhidePath, readFocusedSession]
  );
  const dropIntoExplorer = useCallback(
    async (paths: string[]): Promise<void> => {
      if (paths.length === 0) return;
      await importPaths("", paths);
    },
    [importPaths]
  );
  const commitName = useCallback(
    async (name: string) => {
      const create = pendingCreateRef.current;
      const rename = pendingRenameRef.current;
      if (!create && !rename) return;
      const trimmed = name.trim();
      if (!trimmed) {
        cancelPending();
        return;
      }
      if (trimmed.includes("/") || trimmed === "." || trimmed === "..") {
        toast("Invalid name", "error");
        return;
      }
      const target = readFocusedSession()?.workspace;
      if (!target) return;
      try {
        if (create) {
          const targetPath = create.parent ? `${create.parent}/${trimmed}` : trimmed;
          await (create.kind === "file"
            ? window.vela.createFile(target, targetPath)
            : window.vela.createDir(target, targetPath));
          if (!panelFor(target)) return;
          recordCreatedEntry(target, create.parent, targetPath, create.kind);
          if (create.kind === "file") {
            addCreatedFile(target, targetPath);
          }
          void refreshTree(ancestorDirs(create.parent));
        } else if (rename) {
          const parent = rename.path.includes("/")
            ? rename.path.slice(0, rename.path.lastIndexOf("/"))
            : "";
          const newPath = parent ? `${parent}/${trimmed}` : trimmed;
          cancelPrefix(target, rename.path);
          await window.vela.renamePath(target, rename.path, trimmed);
          if (!panelFor(target)) return;
          renameTabs(target, rename.path, newPath, trimmed, readRenderedActivePath(target));
          renameObservedPrefix(target, rename.path, newPath);
          void refreshTree(ancestorDirs(parent));
        }
        cancelPending();
      } catch (err) {
        if (panelFor(target)) {
          toast(err instanceof Error ? err.message : String(err), "error");
        }
      }
    },
    [toast, refreshTree, cancelPending, cancelPrefix, panelFor, addCreatedFile, renameTabs, renameObservedPrefix, recordCreatedEntry, readRenderedActivePath, readFocusedSession]
  );

  const moveEntry = useCallback(
    async (path: string, destDir: string, workspace?: WorkspaceIdentity) => {
      const target = workspace ?? readFocusedSession()?.workspace;
      if (!target) return;
      const name = path.split("/").pop() ?? path;
      const newPath = destDir ? `${destDir}/${name}` : name;
      cancelPrefix(target, path);
      try {
        await window.vela.movePath(target, path, destDir);
      } catch (err) {
        if (panelFor(target)) {
          toast(err instanceof Error ? err.message : String(err), "error");
        }
        return;
      }
      if (!panelFor(target)) return;
      movePathTabs(target, path, newPath, name);
      moveObservedPrefix(target, path, newPath);
      const parent = path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
      void refreshTree([...ancestorDirs(parent), ...ancestorDirs(destDir)], target);
    },
    [toast, refreshTree, cancelPrefix, panelFor, movePathTabs, moveObservedPrefix, readFocusedSession]
  );

  const moveEntries = useCallback(async (paths: string[], destDir: string): Promise<void> => {
    const workspace = readFocusedSession()?.workspace;
    if (!workspace) return;
    const selected = movableExplorerPaths(paths, destDir);
    const moved: Array<{ path: string; newPath: string; name: string }> = [];
    for (const path of selected) {
      const name = path.split("/").pop() ?? path;
      const newPath = destDir ? `${destDir}/${name}` : name;
      cancelPrefix(workspace, path);
      try {
        await window.vela.movePath(workspace, path, destDir);
        moved.push({ path, newPath, name });
      } catch (err) {
        if (panelFor(workspace)) toast(err instanceof Error ? err.message : String(err), "error");
      }
    }
    if (moved.length === 0 || !panelFor(workspace)) return;
    const remap = movePathsTabs(workspace, moved);
    moveObservedPaths(workspace, remap);
    const parents = moved.map(({ path }) => path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "");
    void refreshTree([...parents.flatMap((parent) => ancestorDirs(parent)), ...ancestorDirs(destDir)], workspace);
  }, [cancelPrefix, panelFor, toast, movePathsTabs, moveObservedPaths, refreshTree, readFocusedSession]);

  return {
    pendingCreate, pendingRename, startCreate, startRename, cancelPending, commitName,
    deleteEntry, deleteEntries, removeFromWorkspace, importPaths, dropIntoExplorer,
    moveEntry, moveEntries, revealInFileManager
  };
}
