import { useEffect, useRef, useState, type ReactNode } from "react";
import { useCtxMenu, useStore } from "../store";
import { FileIcon } from "./FileIcons";
import { droppedFilePaths, isExternalFileDrag } from "../drop";
import { compactExplorerSelection, movableExplorerPaths } from "../explorer-selection";
import type { SessionInfo, TreeEntry } from "@shared/types";
import { SessionsPane } from "./SessionsPane";
import { IconSquarePen } from "./icons";
import {
  DirNode,
  ExplorerStoreContext,
  ExplorerMenu,
  FileNode,
  RowActions,
  TreeNameInput,
  type DragHandlers,
  type SelectionHandlers
} from "./ExplorerTree";

export type SidebarTab = "sessions" | "files";

const EMPTY_HIDDEN_PATHS = new Set<string>();
const EXPLORER_PATHS_MIME = "application/x-vela-explorer-paths";

function explorerPathsFromDrag(e: React.DragEvent): string[] {
  try {
    const value: unknown = JSON.parse(e.dataTransfer.getData(EXPLORER_PATHS_MIME));
    return Array.isArray(value) ? value.filter((path): path is string => typeof path === "string" && path.length > 0) : [];
  } catch {
    return [];
  }
}

function explorerStoreFor(store: ReturnType<typeof useStore>, panel: SessionInfo): ReturnType<typeof useStore> {
  const focused = panel.id === store.session?.id;
  const view = store.workspaceFiles?.(panel.workspace) ?? store;
  return {
    ...store, ...view, session: panel,
    hiddenPaths: view.hiddenPaths ?? EMPTY_HIDDEN_PATHS,
    pendingCreate: focused ? store.pendingCreate : null,
    pendingRename: focused ? store.pendingRename : null,
    activePath: focused ? store.activePath : null,
    tabs: focused ? store.tabs : [],
    toggleDir: (path) => store.toggleDir(path, panel.workspace),
    prefetchDir: (path) => store.prefetchDir(path, panel.workspace)
  };
}

export function FileSidebar({
  collapsed,
  onCollapse,
  onDrag,
  initialTab = "files",
  tab,
  onOpenTerminal
}: {
  collapsed: boolean;
  onCollapse: (open: boolean) => void;
  onDrag: (e: React.MouseEvent) => void;
  initialTab?: SidebarTab;
  tab?: SidebarTab;
  onOpenTerminal?: (directory: string) => void;
}): ReactNode {
  void onCollapse;
  const store = useStore();
  const {
    session,
    panels = [],
    focusSession,
    toggleDir,
    ensureRootOpen,
    openFile,
    tabs = [],
    expanded,
    pendingCreate,
    commitName,
    cancelPending,
    deleteEntry,
    deleteEntries,
    moveEntry,
    moveEntries,
    startCreate,
    singleFile,
    importPaths,
    dropIntoExplorer,
    openWorkspacePanel
  } = store;
  const { openCtxMenu } = useCtxMenu();
  const [fallbackTab] = useState<SidebarTab>(initialTab);
  const activeTab = tab ?? fallbackTab;
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const selectionAnchor = useRef<string | null>(null);
  const selectionWorkspace = useRef<string | null>(null);
  const treeRef = useRef<HTMLDivElement>(null);
  const [dragPaths, setDragPaths] = useState<string[] | null>(null);
  const dragPathsRef = useRef<string[] | null>(null);
  const [dropDir, setDropDir] = useState<string | null>(null);
  const [externalDrop, setExternalDrop] = useState(false);
  const [blankMoveDrop, setBlankMoveDrop] = useState(false);
  const multiRoot = panels.length > 1;
  const orderedPanels = panels.length > 0 ? panels : session ? [session] : [];
  const rootName = session?.directory.split("/").filter(Boolean).pop() ?? "workspace";
  const rootOpen = expanded.has("");
  const blankDropImports = Boolean(session && !singleFile && rootOpen);
  const blankDropLabel = blankDropImports ? `Add to ${rootName}` : "Open as new workspace";
  const loadedRoots = useRef(new Set<string>());
  const dragWorkspace = useRef<string | null>(null);
  const dropWorkspace = useRef<string | null>(null);
  const selectedPathSet = new Set(selectedPaths);

  useEffect(() => {
    if (selectionWorkspace.current === session?.workspace.id) return;
    selectionWorkspace.current = session?.workspace.id ?? null;
    setSelectedPaths([]);
    selectionAnchor.current = null;
  }, [session?.workspace.id]);

  const focusTree = (): void => {
    treeRef.current?.focus({ preventScroll: true });
  };

  const selection: SelectionHandlers = {
    selectedPaths: selectedPathSet,
    onClick: (event, path, activate) => {
      const additive = event.metaKey || event.ctrlKey;
      if (!event.shiftKey && !additive) {
        setSelectedPaths([path]);
        selectionAnchor.current = path;
        focusTree();
        activate();
        return;
      }

      event.preventDefault();
      focusTree();
      if (event.shiftKey) {
        const visiblePaths = Array.from(treeRef.current?.querySelectorAll<HTMLElement>("[data-entry-path]") ?? [])
          .map((row) => row.dataset.entryPath ?? "");
        const anchor = selectionAnchor.current ?? path;
        const anchorIndex = visiblePaths.indexOf(anchor);
        const pathIndex = visiblePaths.indexOf(path);
        const range = anchorIndex < 0 || pathIndex < 0
          ? [path]
          : visiblePaths.slice(Math.min(anchorIndex, pathIndex), Math.max(anchorIndex, pathIndex) + 1);
        setSelectedPaths((current) => additive
          ? compactExplorerSelection([...current, ...range])
          : range);
      } else {
        setSelectedPaths((current) => current.includes(path)
          ? current.filter((selected) => selected !== path)
          : [...current, path]);
        selectionAnchor.current = path;
      }
    },
    onContextMenu: (event, entry) => {
      event.preventDefault();
      event.stopPropagation();
      if (!selectedPathSet.has(entry.path)) {
        setSelectedPaths([entry.path]);
        selectionAnchor.current = entry.path;
      }
      openCtxMenu(event.clientX, event.clientY, entry);
    }
  };

  const deleteSelection = (paths: string[]): void => {
    const compact = compactExplorerSelection(paths);
    setSelectedPaths([]);
    selectionAnchor.current = null;
    if (typeof deleteEntries === "function" && compact.length > 1) {
      void deleteEntries(compact);
    } else {
      for (const path of compact) void deleteEntry(path);
    }
  };

  const moveSelection = (paths: string[], destination: string): void => {
    const movable = movableExplorerPaths(paths, destination);
    if (movable.length === 0) return;
    setSelectedPaths([]);
    selectionAnchor.current = null;
    if (movable.length > 1 && typeof moveEntries === "function") {
      void moveEntries(movable, destination);
    } else {
      for (const path of movable) void moveEntry(path, destination);
    }
  };

  const onTreeKeyDown = (event: React.KeyboardEvent<HTMLDivElement>): void => {
    const target = event.target as HTMLElement;
    if (event.key === "Escape") {
      setSelectedPaths([]);
      selectionAnchor.current = null;
      return;
    }
    if (event.key !== "Delete" && event.key !== "Backspace") return;
    if (target.closest("input, textarea, button, [contenteditable='true']") || selectedPaths.length === 0) return;
    event.preventDefault();
    deleteSelection(selectedPaths);
  };

  useEffect(() => {
    const keys = new Set<string>();
    for (const panel of orderedPanels) {
      const key = `${panel.id}::${panel.directory}`;
      keys.add(key);
      if (!loadedRoots.current.has(key)) void ensureRootOpen(panel.workspace);
    }
    loadedRoots.current = keys;
  }, [panels, session, ensureRootOpen]);

  const drag: DragHandlers = {
    dragPaths,
    dropDir,
    isExternalDrop: isExternalFileDrag,
    onDragStart: (e, path) => {
      if ((e.target as HTMLElement).closest("button")) {
        e.preventDefault();
        return;
      }
      const paths = compactExplorerSelection(dragWorkspace.current === session?.workspace.id && selectedPathSet.has(path) ? selectedPaths : [path]);
      e.dataTransfer.setData("text/plain", paths[0] ?? path);
      e.dataTransfer.setData(EXPLORER_PATHS_MIME, JSON.stringify(paths));
      e.dataTransfer.effectAllowed = "move";
      dragPathsRef.current = paths;
      setDragPaths(paths);
    },
    onDragEnd: () => {
      dragWorkspace.current = null;
      setBlankMoveDrop(false);
      dragPathsRef.current = null;
      setDragPaths(null);
      setDropDir(null);
    },
    onDirDragOver: (e, dir) => {
      setBlankMoveDrop(false);
      if (drag.isExternalDrop(e)) {
        e.stopPropagation();
        e.preventDefault();
        e.dataTransfer.dropEffect = "copy";
        setDropDir(dir);
        setExternalDrop(false);
        return;
      }
      const draggedPaths = dragPathsRef.current ?? dragPaths;
      if (!draggedPaths?.length) return;
      e.stopPropagation();
      if (movableExplorerPaths(draggedPaths, dir).length === 0) {
        setDropDir(null);
        return;
      }
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      setDropDir(dir);
    },
    onDirDrop: (e, dir) => {
      setBlankMoveDrop(false);
      e.preventDefault();
      e.stopPropagation();
      const external = droppedFilePaths(e);
      const draggedPaths = dragPathsRef.current ?? dragPaths;
      dragPathsRef.current = null;
      setDragPaths(null);
      setDropDir(null);
      setExternalDrop(false);
      if (external.length > 0) {
        void (dir === "" ? dropIntoExplorer(external) : importPaths(dir, external));
        return;
      }
      const sources = explorerPathsFromDrag(e);
      const selection = sources.length > 0 ? sources : draggedPaths;
      if (!selection?.length) return;
      moveSelection(selection, dir);
    }
  };

  const onTreeDragOver = (e: React.DragEvent): void => {
    const draggedPaths = dragPathsRef.current ?? dragPaths;
    if (!draggedPaths?.length && !drag.isExternalDrop(e)) return;
    if (drag.isExternalDrop(e)) {
      setBlankMoveDrop(false);
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      if ((e.target as HTMLElement).closest(".tree-row")) {
        setDropDir(null);
        setExternalDrop(false);
        return;
      }
      setDropDir(null);
      setExternalDrop(true);
      return;
    }
    if ((e.target as HTMLElement).closest(".tree-row")) {
      setBlankMoveDrop(false);
      setDropDir(null);
      return;
    }
    if (!draggedPaths?.length) return;
    if (movableExplorerPaths(draggedPaths, "").length === 0) {
      setBlankMoveDrop(false);
      setDropDir(null);
      return;
    }
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    setBlankMoveDrop(true);
    setDropDir("");
  };

  const onTreeDrop = (e: React.DragEvent): void => {
    setBlankMoveDrop(false);
    e.preventDefault();
    const external = droppedFilePaths(e);
    const draggedPaths = dragPathsRef.current ?? dragPaths;
    dragPathsRef.current = null;
    const targetRow = (e.target as HTMLElement).closest(".tree-row");
    if (external.length === 0 && targetRow && !targetRow.hasAttribute("data-explorer-root")) return;
    setDragPaths(null);
    setDropDir(null);
    setExternalDrop(false);
    if (external.length > 0) {
      if (targetRow) return;
      if (blankDropImports) {
        void dropIntoExplorer(external);
        return;
      }
      void Promise.all(external.map(async (path) => {
        const kind = await window.vela.externalKind(path).catch(() => ({ kind: "missing" as const }));
        if (kind.kind === "directory") {
          await openWorkspacePanel(path);
        } else if (kind.kind === "file") {
          await dropIntoExplorer([path]);
        }
      }));
      return;
    }
    const sources = explorerPathsFromDrag(e);
    const selection = sources.length > 0 ? sources : draggedPaths;
    if (!selection?.length) return;
    moveSelection(selection, "");
  };

  const onTreeDragEnter = (e: React.DragEvent): void => {
    if (drag.isExternalDrop(e)) setExternalDrop(!(e.target as HTMLElement).closest(".tree-row"));
  };

  const onTreeDragLeave = (e: React.DragEvent): void => {
    if (!e.currentTarget.contains(e.relatedTarget as Node | null)) {
      setBlankMoveDrop(false);
      setExternalDrop(false);
      setDropDir(null);
    }
  };

  const dropSpace = <div className={`tree-drop-space ${externalDrop || blankMoveDrop ? "active" : ""}`}>
    {(externalDrop || blankMoveDrop) && <span className="tree-drop-label">
      {externalDrop ? blankDropLabel : `Move into ${session?.directory.split("/").filter(Boolean).pop() ?? "workspace"}`}
    </span>}
  </div>;

  if (collapsed) {
    return null;
  }

  if (singleFile && session && !multiRoot) {
    const name = singleFile.split("/").pop() ?? singleFile;
    const singleFileUnsaved = tabs.some((tab) => tab.path === singleFile && tab.dirty);
    return (
      <div className="sidebar">
        <div className="sidebar-header">
          <span className="sidebar-title" title={`${session.directory}/${singleFile}`}>
            <span className="sidebar-title-dot" aria-hidden />
            <span className="sidebar-title-name">{name}</span>
          </span>
        </div>
        <div className="section-trigger">
          <span className="section-toggle open">FILE</span>
        </div>
        <div className="sidebar-section explorer">
          <div
             className="tree"
             onDragEnter={onTreeDragEnter}
             onDragOver={onTreeDragOver}
             onDrop={onTreeDrop}
             onDragLeave={onTreeDragLeave}
            onContextMenu={(e) => {
              e.preventDefault();
              openCtxMenu(e.clientX, e.clientY, null);
            }}
          >
            <div
              className={`tree-row file active${singleFileUnsaved ? " unsaved" : ""}`}
              onClick={() => void openFile(singleFile)}
              title={singleFile}
            >
              <FileIcon name={name} isDir={false} />
              <span className="tree-name">{name}</span>
              {singleFileUnsaved && <span className="tree-unsaved-dot" title="Unsaved changes" aria-label="Unsaved changes" />}
            </div>
            <div className={`tree-drop-space ${externalDrop ? "active" : ""}`}>
              {externalDrop && <span className="tree-drop-label">Open as new workspace</span>}
            </div>
          </div>
        </div>
        <ExplorerMenu onOpenTerminal={onOpenTerminal} />
      </div>
    );
  }

  return (
    <div className="sidebar files-sidebar">
      {activeTab === "sessions" ? (
        <SessionsPane />
      ) : (
        <>
          <div className="section-trigger files-brand-header">
            <span className="sidebar-brand">Vela</span>
          </div>

       {orderedPanels.length > 0 && !multiRoot && <div
         className={`section-trigger with-actions explorer-trigger ${rootOpen ? "open" : ""} ${dropDir === "" ? "drop-target" : ""}`}
         data-explorer-root=""
         title={session?.directory}
         onDragOver={(e) => drag.onDirDragOver(e, "")}
         onDrop={(e) => drag.onDirDrop(e, "")}
         onDragLeave={(e) => {
           if (drag.isExternalDrop(e)) setDropDir(null);
         }}
         onContextMenu={(e) => {
           e.preventDefault();
           e.stopPropagation();
           openCtxMenu(e.clientX, e.clientY, { path: "", type: "directory" });
         }}
       >
         <button
           className="section-chevron-button explorer-chevron"
           aria-label="Toggle workspace files"
           aria-expanded={rootOpen}
           onClick={() => {
             if (session) void toggleDir("");
           }}
         >
           <i className={`codicon codicon-chevron-right ${rootOpen ? "open" : ""}`} aria-hidden="true" />
         </button>
         <button
           className={`section-toggle ${rootOpen ? "open" : ""}`}
           aria-expanded={rootOpen}
           onClick={() => {
             if (session) void toggleDir("");
           }}
         >
           <span className="explorer-root-name">{rootName}</span>
         </button>
         {session && <RowActions entry={{ path: "", type: "directory" }} allowFile allowDir />}
       </div>}
        <div className="sidebar-section explorer">
          <div
             ref={treeRef}
             className="tree"
             tabIndex={0}
             onKeyDown={onTreeKeyDown}
             onDragEnter={onTreeDragEnter}
             onDragOver={onTreeDragOver}
             onDrop={onTreeDrop}
             onDragLeave={onTreeDragLeave}
            onContextMenu={(e) => {
              if ((e.target as HTMLElement).closest(".tree-row")) return;
              e.preventDefault();
              openCtxMenu(e.clientX, e.clientY, null);
            }}
          >
            {orderedPanels.length === 0 && (
              session ? <div className="tree-empty">Loading…</div> : (
                <div className="sessions-actions">
                  <button
                    type="button"
                    className="sessions-new"
                    onClick={() => void store.selectFolder()}
                    title="Choose a folder to open as a workspace"
                  >
                    <IconSquarePen />
                    Open Workspace
                  </button>
                </div>
              )
            )}
            {orderedPanels.map((panel) => {
              const focused = panel.id === session?.id;
              const view = explorerStoreFor(store, panel);
              const panelOpen = view.expanded.has("");
              const focus = (): void => { if (!focused) focusSession(panel.id); };
              const rejectCrossRootMove = (event: React.DragEvent): void => {
                if (dragWorkspace.current && dragWorkspace.current !== panel.workspace.id && !drag.isExternalDrop(event)) {
                  event.stopPropagation();
                  event.dataTransfer.dropEffect = "none";
                }
              };
              const panelSelection: SelectionHandlers = focused ? selection : {
                ...selection,
                selectedPaths: new Set(),
                onClick: (event, path, activate) => {
                  selectionWorkspace.current = panel.workspace.id;
                  setSelectedPaths([path]);
                  selectionAnchor.current = path;
                  focusTree();
                  if (!event.metaKey && !event.ctrlKey && !event.shiftKey) activate();
                },
                onContextMenu: (event, entry) => {
                  event.preventDefault();
                  event.stopPropagation();
                  selectionWorkspace.current = panel.workspace.id;
                  setSelectedPaths([entry.path]);
                  selectionAnchor.current = entry.path;
                  openCtxMenu(event.clientX, event.clientY, entry);
                }
              };
              const panelDrag = { ...drag, dropDir: dropWorkspace.current === panel.workspace.id ? dropDir : null };
              return <ExplorerStoreContext.Provider key={panel.id} value={view}>
                <div
                  className={`tree-workspace ${panelOpen ? "expanded" : ""}`}
                  data-workspace-directory={panel.directory}
                  onClickCapture={focus}
                  onContextMenuCapture={focus}
                  onDragStartCapture={() => { focus(); dragWorkspace.current = panel.workspace.id; }}
                  onDragOverCapture={(event) => { dropWorkspace.current = panel.workspace.id; rejectCrossRootMove(event); }}
                  onDropCapture={(event) => { rejectCrossRootMove(event); if (!event.isPropagationStopped()) focus(); }}
                >
                  {multiRoot && <div
                    className={`tree-row dir workspace-root ${panelOpen ? "open" : ""} ${panelDrag.dropDir === "" ? "drop-target" : ""}`}
                    title={panel.directory}
                    role="button"
                    tabIndex={0}
                    aria-expanded={panelOpen}
                    onKeyDown={(event) => {
                      if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) {
                        event.preventDefault(); event.stopPropagation(); focus();
                        void toggleDir("", panel.workspace);
                      }
                    }}
                    onClick={() => void toggleDir("", panel.workspace)}
                    onContextMenu={(event) => {
                      event.preventDefault(); event.stopPropagation();
                      openCtxMenu(event.clientX, event.clientY, { path: "", type: "directory" }, panel.id);
                    }}
                    onDragOver={(event) => drag.onDirDragOver(event, "")}
                    onDrop={(event) => drag.onDirDrop(event, "")}
                  >
                    <span className={`tree-chevron ${panelOpen ? "open" : ""}`} aria-hidden="true"><i className="codicon codicon-chevron-right" /></span>
                    <span className="tree-name explorer-root-name">{panel.directory.split("/").filter(Boolean).pop() ?? "workspace"}</span>
                    <RowActions entry={{ path: "", type: "directory" }} allowFile allowDir />
                  </div>}
                  {panelOpen && (view.tree[""] ?? []).filter((child) => !view.hiddenPaths.has(child.path)).map((child) =>
                    child.type === "directory" ? (
                      <DirNode key={child.path} entry={child} depth={0} drag={panelDrag} hiddenPaths={view.hiddenPaths} selection={panelSelection} />
                    ) : (
                      <FileNode key={child.path} entry={child} depth={0} drag={panelDrag} selection={panelSelection} />
                    )
                  )}
                  {focused && pendingCreate?.parent === "" && <TreeNameInput
                    initial={pendingCreate.kind === "file" ? "untitled.txt" : "untitled folder"}
                    isDir={pendingCreate.kind === "dir"}
                    onCommit={(value) => void commitName(value)} onCancel={cancelPending}
                  />}
                  {focused && blankDropImports && dropSpace}
                </div>
              </ExplorerStoreContext.Provider>;
            })}
            {!blankDropImports && dropSpace}
          </div>
        </div>
        </>
      )}
      <ExplorerMenu onOpenTerminal={onOpenTerminal} selectedPaths={selectedPaths} onDeleteSelection={deleteSelection} />
    </div>
  );
}
