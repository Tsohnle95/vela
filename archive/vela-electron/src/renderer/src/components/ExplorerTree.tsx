import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import type { TreeEntry } from "@shared/types";
import { useCtxMenu, useStore } from "../store";
import { EllipsisIcon, FilePlusIcon, FolderPlusIcon, PencilIcon, TrashIcon, TreeFileGlyph } from "./FileIcons";

// Each root projects its own file state while actions retain the store's ownership.
export const ExplorerStoreContext = createContext<ReturnType<typeof useStore> | null>(null);
function useExplorerStore(): ReturnType<typeof useStore> {
  const store = useStore();
  return useContext(ExplorerStoreContext) ?? store;
}

export interface DragHandlers {
  dragPaths: string[] | null;
  dropDir: string | null;
  isExternalDrop: (e: React.DragEvent) => boolean;
  onDragStart: (e: React.DragEvent, path: string) => void;
  onDragEnd: () => void;
  onDirDragOver: (e: React.DragEvent, dir: string) => void;
  onDirDrop: (e: React.DragEvent, dir: string) => void;
}

export interface SelectionHandlers {
  selectedPaths: Set<string>;
  onClick: (event: React.MouseEvent, path: string, activate: () => void) => void;
  onContextMenu: (event: React.MouseEvent, entry: TreeEntry) => void;
}

export function RowActions({ entry, allowFile = false, allowDir = false }: { entry: TreeEntry; allowFile?: boolean; allowDir?: boolean }): ReactNode {
  const { startCreate } = useExplorerStore();
  const { openCtxMenu } = useCtxMenu();
  const parent =
    entry.type === "directory"
      ? entry.path
      : entry.path.includes("/")
        ? entry.path.slice(0, entry.path.lastIndexOf("/"))
        : "";
  const menu = (e: Pick<React.MouseEvent, "clientX" | "clientY" | "stopPropagation">): void => {
    e.stopPropagation();
    openCtxMenu(e.clientX, e.clientY, entry);
  };
  return (
    <span className="tree-row-actions">
      {allowFile && (
        <button
          type="button"
          className="tree-row-action"
          title="New File"
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation();
            startCreate(parent, "file");
          }}
        >
          <FilePlusIcon />
        </button>
      )}
      {allowDir && (
        <button
          type="button"
          className="tree-row-action"
          title="New Folder"
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation();
            startCreate(parent, "dir");
          }}
        >
          <FolderPlusIcon />
        </button>
      )}
      <button
        type="button"
        className="tree-row-action"
        title="More actions…"
        onPointerDown={(e) => e.stopPropagation()}
        onMouseDown={(e) => {
          e.preventDefault();
          e.stopPropagation();
          menu(e);
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <EllipsisIcon />
      </button>
    </span>
  );
}

export function DirNode({
  entry,
  depth,
  drag,
  hiddenPaths,
  selection
}: {
  entry: TreeEntry;
  depth: number;
  drag: DragHandlers;
  hiddenPaths: Set<string>;
  selection: SelectionHandlers;
}): ReactNode {
  const {
    expanded,
    tree,
    toggleDir,
    prefetchDir,
    agentFiles,
    pendingRename,
    pendingCreate,
    commitName,
    cancelPending
  } = useExplorerStore();
  const isOpen = expanded.has(entry.path);
  const hasChanges = entry.path.split("/").some((_, i) => {
    const prefix = entry.path.split("/").slice(0, i + 1).join("/");
    return agentFiles.has(prefix);
  });

  if (pendingRename?.path === entry.path) {
    return (
      <TreeNameInput
        initial={entry.path.split("/").pop() ?? ""}
        isDir
        onCommit={(v) => void commitName(v)}
        onCancel={cancelPending}
      />
    );
  }

  return (
    <div>
      <div
        className={`tree-row dir ${isOpen ? "open" : ""} ${selection.selectedPaths.has(entry.path) ? "selected" : ""} ${drag.dropDir === entry.path ? "drop-target" : ""}`}
        data-entry-path={entry.path}
        draggable
        onClick={(e) => selection.onClick(e, entry.path, () => void toggleDir(entry.path))}
        onMouseEnter={() => prefetchDir(entry.path)}
        onContextMenu={(e) => selection.onContextMenu(e, entry)}
        onDragStart={(e) => drag.onDragStart(e, entry.path)}
        onDragEnd={drag.onDragEnd}
        onDragOver={(e) => drag.onDirDragOver(e, entry.path)}
        onDrop={(e) => drag.onDirDrop(e, entry.path)}
      >
        <span className={`tree-chevron ${isOpen ? "open" : ""}`} aria-hidden="true">
          <i className="codicon codicon-chevron-right" />
        </span>
        <span className="tree-name">{entry.path.split("/").pop()}</span>
        {hasChanges && <span className="tree-badge" />}
        <RowActions entry={entry} allowFile allowDir />
      </div>
      {isOpen && (
        <div className="tree-children">
          {(tree[entry.path] ?? []).filter((child) => !hiddenPaths.has(child.path)).map((child) =>
            child.type === "directory" ? (
              <DirNode key={child.path} entry={child} depth={depth + 1} drag={drag} hiddenPaths={hiddenPaths} selection={selection} />
            ) : (
              <FileNode key={child.path} entry={child} depth={depth + 1} drag={drag} selection={selection} />
            )
          )}
          {pendingCreate?.parent === entry.path && (
            <TreeNameInput
              initial={pendingCreate.kind === "file" ? "untitled.txt" : "untitled folder"}
              isDir={pendingCreate.kind === "dir"}
              onCommit={(v) => void commitName(v)}
              onCancel={cancelPending}
            />
          )}
        </div>
      )}
    </div>
  );
}

export function FileNode({
  entry,
  depth,
  drag,
  selection
}: {
  entry: TreeEntry;
  depth: number;
  drag: DragHandlers;
  selection: SelectionHandlers;
}): ReactNode {
  const { openFile, previewFile, activePath, tabs = [], agentFiles, pendingRename, commitName, cancelPending } =
    useExplorerStore();
  const name = entry.path.split("/").pop() ?? entry.path;
  const changed = agentFiles.has(entry.path);
  const unsaved = tabs.some((tab) => tab.path === entry.path && tab.dirty);
  const active = activePath === entry.path;

  if (pendingRename?.path === entry.path) {
    return (
      <TreeNameInput
        initial={name}
        isDir={false}
        onCommit={(v) => void commitName(v)}
        onCancel={cancelPending}
      />
    );
  }

  return (
    <div
      className={`tree-row file ${active ? "active" : ""} ${selection.selectedPaths.has(entry.path) ? "selected" : ""}`}
      data-entry-path={entry.path}
      draggable
      onClick={(e) => selection.onClick(e, entry.path, () => void previewFile(entry.path))}
      onDoubleClick={() => void openFile(entry.path)}
      onContextMenu={(e) => selection.onContextMenu(e, entry)}
      onDragStart={(e) => drag.onDragStart(e, entry.path)}
      onDragEnd={drag.onDragEnd}
      title={entry.path}
    >
      <TreeFileGlyph name={name} />
      <span className="tree-name">{name}</span>
      {unsaved && <span className="tree-unsaved-dot" title="Unsaved changes" aria-label="Unsaved changes" />}
      {changed && <span className="tree-badge changed" />}
      <RowActions entry={entry} />
    </div>
  );
}

export function TreeNameInput({
  initial,
  isDir,
  onCommit,
  onCancel
}: {
  initial: string;
  isDir: boolean;
  onCommit: (value: string) => void;
  onCancel: () => void;
}): ReactNode {
  const [value, setValue] = useState(initial);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  return (
    <div className="tree-row tree-input-row">
      {isDir ? <span className="tree-chevron empty" aria-hidden="true" /> : <TreeFileGlyph name={value || initial} />}
      <input
        ref={inputRef}
        className="tree-input"
        value={value}
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="off"
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") onCommit(value);
          else if (e.key === "Escape") onCancel();
        }}
        onBlur={() => onCommit(value)}
      />
    </div>
  );
}

export function ExplorerMenu({
  onOpenTerminal,
  selectedPaths,
  onDeleteSelection
}: {
  onOpenTerminal?: (directory: string) => void;
  selectedPaths?: string[];
  onDeleteSelection?: (paths: string[]) => void;
}): ReactNode {
  const { session, startCreate, startRename, revealInFileManager, deleteEntry, removeFromWorkspace, closePanel } = useExplorerStore();
  const { ctxMenu, closeCtxMenu } = useCtxMenu();
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!ctxMenu) return;
    const onDown = (e: PointerEvent): void => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) closeCtxMenu();
    };
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") closeCtxMenu();
    };
    document.addEventListener("pointerdown", onDown, true);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("keydown", onKey);
    };
  }, [ctxMenu, closeCtxMenu]);

  if (!ctxMenu) return null;
  const target = ctxMenu.target;
  const workspaceRoot = target?.type === "directory" && target.path === "";
  const workspaceSessionID = workspaceRoot ? ctxMenu.workspaceSessionID ?? session?.id : undefined;
  const activeSelection = selectedPaths ?? [];
  const deleteSelection = target && activeSelection.includes(target.path) ? activeSelection : [];
  const parent = target
    ? target.type === "directory"
      ? target.path
      : target.path.includes("/")
        ? target.path.slice(0, target.path.lastIndexOf("/"))
        : ""
    : "";
  const left = Math.min(ctxMenu.x, window.innerWidth - 190);
  const top = Math.min(ctxMenu.y, window.innerHeight - 150);

  return createPortal(
    <div className="ctx-menu" ref={menuRef} style={{ left, top }}>
      <button className="ctx-item" onClick={() => { closeCtxMenu(); onOpenTerminal?.(parent); }}>
        Open in Integrated Terminal
      </button>
      <button className="ctx-item" onClick={() => startCreate(parent, "file")}>
        <FilePlusIcon />
        New File…
      </button>
      <button className="ctx-item" onClick={() => startCreate(parent, "dir")}>
        <FolderPlusIcon />
        New Folder…
      </button>
      {target && (
        <button className="ctx-item" onClick={() => void revealInFileManager(target.path)}>
          Reveal in File Manager
        </button>
      )}
      {workspaceSessionID && (
        <button className="ctx-item" onClick={() => { closeCtxMenu(); closePanel(workspaceSessionID); }}>
          Remove Workspace
        </button>
      )}
      {target && !workspaceRoot && (
        <>
          <div className="ctx-sep" />
          <button className="ctx-item" onClick={() => startRename(target.path)}>
            <PencilIcon />
            Rename…
          </button>
          <button className="ctx-item" onClick={() => removeFromWorkspace(target.path)}>
            Remove from Workspace
          </button>
          <button className="ctx-item danger" onClick={() => {
            if (deleteSelection.length > 1 && onDeleteSelection) onDeleteSelection(deleteSelection);
            else void deleteEntry(target.path);
          }}>
            <TrashIcon />
            {deleteSelection.length > 1 ? `Delete ${deleteSelection.length} Items` : "Delete"}
          </button>
        </>
      )}
    </div>,
    document.body
  );
}
