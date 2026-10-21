import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import Editor from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import { languageForPath } from "../monaco";
import { wireEmmetKeys } from "../emmet-keys";
import { wireEditorNavigationKeys } from "../editor-navigation";
import { wireHtmlAttributeQuotes } from "../editor-language-input";
import { clearW3cMarkers } from "../w3c-validation";
import { useStore } from "../store";
import { useTheme } from "../theme";
import { registerEditor, unregisterEditor } from "../reveal";
import { droppedFilePaths, isExternalFileDrag } from "../drop";
import type { SessionInfo, Tab } from "@shared/types";
import type { EditorStatus } from "./editor-status";
import { IconChevronRight, IconClose } from "./icons";
import { EDITOR_GROUP_IDS, isEditorGroupID, type EditorGroupID } from "./editor-groups";
import { EDITOR_TAB_MIME, readEditorTabTransfer, TabBar } from "./EditorTabs";
import { EditorStatusMenu } from "./EditorStatusMenu";
import { BrowserStack } from "./BrowserPane";
import { useBrowserTabActions, useBrowserTabState } from "../browser-tabs-context";
import { useScrollReveal, useStatusReporter } from "../editor-activity";
import type { ValidationReport } from "../validation-report";

export const EDITOR_OPTIONS = {
  fontSize: 13,
  fontFamily: "ui-monospace, 'SFMono-Regular', 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace",
  minimap: { enabled: false },
  automaticLayout: true,
  fixedOverflowWidgets: true,
  quickSuggestions: { other: true, comments: false, strings: true },
  quickSuggestionsDelay: 25,
  suggestOnTriggerCharacters: true,
  suggestFontSize: 13,
  suggestLineHeight: 24,
  suggest: {
    showInlineDetails: true,
    showSnippets: true,
    showWords: true,
    selectionMode: "always"
  } as const,
  wordBasedSuggestions: "currentDocument" as const,
  tabCompletion: "on" as const,
  acceptSuggestionOnEnter: "on" as const,
  autoClosingQuotes: "languageDefined" as const,
  folding: true,
  glyphMargin: false,
  showFoldingControls: "mouseover" as const,
  foldingHighlight: true,
  tabSize: 2,
  scrollBeyondLastLine: false,
  smoothScrolling: true,
  cursorBlinking: "smooth" as const,
  padding: { top: 10, bottom: 10 },
  renderWhitespace: "none" as const,
  scrollbar: { verticalScrollbarSize: 14, verticalSliderSize: 6, horizontalScrollbarSize: 14, horizontalSliderSize: 6, useShadows: false },
  // The overview ruler draws its own border + marks over the scrollbar gutter;
  // the reference editor has neither, and the only divider should be the agent
  // panel's own border.
  overviewRulerBorder: false,
  overviewRulerLanes: 0,
  lineNumbersMinChars: 3,
  lineDecorationsWidth: 0,
  wordWrap: "on" as const
};

export function overflowWidgetsHost(): HTMLElement {
  let host = document.getElementById("vela-monaco-overflow");
  if (!host) {
    host = document.createElement("div");
    host.id = "vela-monaco-overflow";
    host.className = "monaco-editor vela-monaco-overflow-host";
    document.body.append(host);
  }
  return host;
}

function SplitRightIcon(): ReactNode {
  return (
    <svg className="os-icon editor-split-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="1.75" y="2.25" width="12.5" height="11.5" rx="1.4" />
      <path d="M8 2.5v11M10.25 6.1l2 1.9-2 1.9" />
    </svg>
  );
}

function beginGroupResize(
  grid: HTMLDivElement | null,
  axis: "x" | "y",
  setSplit: (percent: number) => void
): () => void {
  if (!grid) return () => {};
  const move = (next: MouseEvent): void => {
    const bounds = grid.getBoundingClientRect();
    const extent = axis === "x" ? bounds.width : bounds.height;
    if (extent <= 0) return;
    const offset = axis === "x" ? next.clientX - bounds.left : next.clientY - bounds.top;
    setSplit(clampGroupSplit(offset / extent * 100, extent));
  };
  const finish = (): void => {
    window.removeEventListener("mousemove", move);
    window.removeEventListener("mouseup", finish);
  };
  window.addEventListener("mousemove", move);
  window.addEventListener("mouseup", finish);
  return finish;
}

function useGroupResize(workspaceID: string | null, groupCount: number) {
  const groupGridRef = useRef<HTMLDivElement | null>(null);
  const stopResizeRef = useRef<() => void>(() => {});
  useEffect(() => () => stopResizeRef.current(), [workspaceID, groupCount]);
  const startResize = (axis: "x" | "y", update: (value: number) => void): void => {
    stopResizeRef.current();
    stopResizeRef.current = beginGroupResize(groupGridRef.current, axis, update);
  };
  return { groupGridRef, startResize };
}

function clampGroupSplit(percent: number, extent: number): number {
  const minimum = extent > 0 ? Math.min(160 / extent * 100, 100 / 3) : 100 / 3;
  return Math.max(minimum, Math.min(100 - minimum, percent));
}

function GroupResizeHandle({ axis, value, grid, onChange, onStart, height }: {
  axis: "x" | "y";
  value: number;
  grid: HTMLDivElement | null;
  onChange: (value: number) => void;
  onStart: () => void;
  height?: string;
}): ReactNode {
  const extent = axis === "x" ? grid?.clientWidth ?? 0 : grid?.clientHeight ?? 0;
  return <div className={`editor-group-resize editor-group-resize-${axis}`} role="separator"
    tabIndex={0} aria-label={axis === "x" ? "Resize editor columns" : "Resize editor rows"}
    aria-orientation={axis === "x" ? "vertical" : "horizontal"}
    aria-valuemin={clampGroupSplit(0, extent)} aria-valuemax={clampGroupSplit(100, extent)} aria-valuenow={value}
    onMouseDown={(event) => { event.preventDefault(); event.stopPropagation(); onStart(); }}
    onKeyDown={(event) => {
      const decrease = axis === "x" ? "ArrowLeft" : "ArrowUp";
      const increase = axis === "x" ? "ArrowRight" : "ArrowDown";
      if (event.key !== decrease && event.key !== increase) return;
      event.preventDefault();
      onChange(clampGroupSplit(value + (event.key === decrease ? -5 : 5), extent));
    }}
    style={axis === "x" ? { left: `${value}%`, height } : { top: `${value}%` }} />;
}

function BreadcrumbBar({
  tab,
  directory,
  editorStatus,
  onValidationComplete,
  actions
}: {
  tab: Tab;
  directory: string | undefined;
  editorStatus: EditorStatus | null;
  onValidationComplete?: (report: ValidationReport) => void;
  actions?: ReactNode;
}): ReactNode {
  const { saveTab } = useStore();
  const root = directory?.split(/[\\/]/).filter(Boolean).at(-1) ?? "workspace";
  const segments = tab.path.replace(/\\/g, "/").split("/").filter(Boolean);

  return (
    <div className="editor-breadcrumbs">
      <nav className="editor-breadcrumb-path" aria-label="File path">
        {[root, ...segments].map((segment, index, all) => (
          <span className="breadcrumb-segment" key={`${index}:${segment}`}>
            {index > 0 && <IconChevronRight />}
            <span
              className={index === all.length - 1 ? "breadcrumb-current" : "breadcrumb-ancestor"}
              aria-current={index === all.length - 1 ? "page" : undefined}
            >
              {segment}
            </span>
          </span>
        ))}
      </nav>
      <div className="editor-breadcrumb-actions">
        {tab.dirty && <span className="editor-dirty">unsaved</span>}
        {tab.dirty && !tab.conflict && (
          <button
            type="button"
            className="toolbar-btn"
            aria-label={`Save ${tab.name}`}
            title="Save this file"
            onClick={() => void saveTab(tab.path)}
          >Save</button>
        )}
        {tab.stale && <span className="editor-stale">changed on disk</span>}
        {tab.deleted && <span className="editor-deleted">deleted on disk</span>}
        {actions}
        <EditorStatusMenu tab={tab} editorStatus={editorStatus} onValidationComplete={onValidationComplete} />
      </div>
    </div>
  );
}

function EditorWithSave({
  tab,
  focused,
  onStatusChange
}: {
  tab: Tab;
  focused: boolean;
  onStatusChange: (status: EditorStatus) => void;
}): ReactNode {
  const { theme } = useTheme();
  const {
    editContent,
    saveTab,
    reloadTab,
    overwriteTab,
    mergeTab
  } = useStore();

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (focused && (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void saveTab(tab.path);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [focused, saveTab, tab.path]);

  const language = useMemo(() => languageForPath(tab.path), [tab.path]);
  const w3cFile = useMemo(() => /\.(?:html?|css)$/i.test(tab.path), [tab.path]);
  const options = useMemo(() => ({ ...EDITOR_OPTIONS, overflowWidgetsDomNode: overflowWidgetsHost() }), []);

  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const navigationListenerRef = useRef<{ dispose: () => void } | null>(null);
  const htmlQuoteListenerRef = useRef<{ dispose: () => void } | null>(null);
  const tabContent = tab.content;
  const tabPath = tab.path;
  const tabContentRef = useRef(tabContent);
  tabContentRef.current = tabContent;
  const watchCursor = useStatusReporter(tabPath, focused, onStatusChange);
  const { scrolling, watch: watchScroll } = useScrollReveal();

  const handleChange = useCallback((value: string | undefined): void => {
    if (value !== undefined) editContent(tabPath, value);
  }, [editContent, tabPath]);

  const handleMount = useCallback((ed: editor.IStandaloneCodeEditor): void => {
    editorRef.current = ed;
    registerEditor(tabPath, ed);
    wireEmmetKeys(ed);
    navigationListenerRef.current?.dispose();
    navigationListenerRef.current = wireEditorNavigationKeys(ed);
    htmlQuoteListenerRef.current?.dispose();
    htmlQuoteListenerRef.current = wireHtmlAttributeQuotes(ed);
    const model = ed.getModel();
    const latest = tabContentRef.current;
    if (model && model.getValue() !== latest) {
      model.pushEditOperations([], [{
        range: model.getFullModelRange(),
        text: latest
      }], () => null);
    }
    watchCursor(ed);
    watchScroll(ed);
  }, [tabPath, watchCursor, watchScroll]);

  useEffect(() => {
    const ed = editorRef.current;
    if (!ed || tab.dirty) return;
    const model = ed.getModel();
    if (!model || model.getValue() === tabContent) return;
    const selections = ed.getSelections();
    const scrollTop = ed.getScrollTop();
    const scrollLeft = ed.getScrollLeft();
    model.pushEditOperations(selections ?? [], [{
      range: model.getFullModelRange(),
      text: tabContent
    }], () => null);
    if (selections) {
      try {
        ed.setSelections(selections);
      } catch {
        // selection may be out of bounds after an external edit
      }
    }
    ed.setScrollTop(scrollTop);
    ed.setScrollLeft(scrollLeft);
  }, [tabContent, tab.dirty]);

  useEffect(() => {
    if (!w3cFile) return;
    clearW3cMarkers(tab.path);
    return () => clearW3cMarkers(tab.path);
  }, [tab.content, tab.path, w3cFile]);

  useEffect(() => () => {
    navigationListenerRef.current?.dispose();
    navigationListenerRef.current = null;
    htmlQuoteListenerRef.current?.dispose();
    htmlQuoteListenerRef.current = null;
    unregisterEditor(tabPath);
  }, [tabPath]);

  return (
    <div className={`editor-wrap${scrolling ? " scrolling" : ""}`}>
      {tab.deleted && (
        <div className="deleted-banner">
          This file was deleted from disk while you were viewing it.
        </div>
      )}

      {tab.conflict && (
        <div className="conflict-banner">
          <span>
            {tab.conflict.deleted
              ? "This file was deleted outside Vela. Your edits are safe and saving is paused."
              : "This file changed outside Vela. Your edits are safe and saving is paused."}
          </span>
          <div className="conflict-actions">
            <button onClick={() => reloadTab(tab.path)}>Reload disk version</button>
            {tab.conflict.resolution === "pending" ? (
              <button onClick={() => mergeTab(tab.path)}>Keep editing to merge</button>
            ) : (
              <button onClick={() => void overwriteTab(tab.path)}>Save merged content</button>
            )}
            <button className="danger" onClick={() => void overwriteTab(tab.path)}>Overwrite disk</button>
          </div>
        </div>
      )}

      <Editor
        theme={`vela-${theme}`}
        language={language}
        path={tab.path}
        defaultValue={tab.content}
        onMount={handleMount}
        options={options}
        onChange={handleChange}
      />
    </div>
  );
}

interface EditorGroupState {
  id: EditorGroupID;
  paths: string[];
  activePath: string | null;
}

interface EditorLayoutState {
  groups: EditorGroupState[];
  activeGroup: EditorGroupID;
  splitX: number;
  splitY: number;
}

function sameGroupLayout(left: EditorGroupState[], right: EditorGroupState[]): boolean {
  return left.length === right.length && left.every((group, index) => {
    const other = right[index];
    return other !== undefined && group.id === other.id && group.activePath === other.activePath &&
      group.paths.length === other.paths.length && group.paths.every((path, pathIndex) => path === other.paths[pathIndex]);
  });
}

function groupOwningPath(groups: EditorGroupState[], path: string): EditorGroupID {
  return groups.find((group) => group.paths.includes(path))?.id ?? "primary";
}

// Every group's tabs share the titlebar strip, so a split panel's files stay
// with the rest of the tabs instead of growing a second strip inside the panel.
function stripTabsFor(groups: EditorGroupState[], tabs: Tab[]): Tab[] {
  const owned = new Set<string>();
  const ordered: Tab[] = [];
  for (const group of groups) {
    for (const tab of tabs) {
      if (owned.has(tab.path) || !group.paths.includes(tab.path)) continue;
      owned.add(tab.path);
      ordered.push(tab);
    }
  }
  // A tab can exist for a moment before group membership catches up with it.
  return [...ordered, ...tabs.filter((tab) => !owned.has(tab.path))];
}

function editorGroupTitle(id: EditorGroupID, index: number): string {
  if (id === "primary") return "Primary editor";
  if (id === "secondary") return "Secondary editor";
  return `Editor group ${index + 1}`;
}

function sameEditorStatus(left: EditorStatus | null, right: EditorStatus): boolean {
  return left !== null && left.path === right.path && left.lineNumber === right.lineNumber &&
    left.column === right.column && left.tabSize === right.tabSize && left.insertSpaces === right.insertSpaces;
}

function EditorGroupActions({
  id,
  index,
  groupCount,
  onSplit,
  onCloseGroup
}: {
  id: EditorGroupID;
  index: number;
  groupCount: number;
  onSplit: (id: EditorGroupID) => void;
  onCloseGroup: (id: EditorGroupID) => void;
}): ReactNode {
  const splitLabel = id === "primary" ? "Split editor right" : `Split editor group ${index + 1} right`;
  const closeLabel = id === "secondary" ? "Close secondary editor group" : `Close editor group ${index + 1}`;
  return (
    <>
      <button
        type="button"
        className="editor-group-action"
        aria-label={splitLabel}
        title={groupCount >= EDITOR_GROUP_IDS.length ? "Vela supports up to four editor groups" : splitLabel}
        disabled={groupCount >= EDITOR_GROUP_IDS.length}
        onClick={() => onSplit(id)}
      ><SplitRightIcon /></button>
      {index > 0 && <button
        type="button"
        className="editor-group-action"
        aria-label={closeLabel}
        title={closeLabel}
        onClick={() => onCloseGroup(id)}
      ><IconClose /></button>}
    </>
  );
}

function EmptyEditorGroup({ actions }: { actions: ReactNode }): ReactNode {
  return (
    <>
      <div className="editor-breadcrumbs editor-breadcrumbs-bare">
        <div className="editor-breadcrumb-actions">{actions}</div>
      </div>
      <div className="editor-group-empty">
        <strong>Empty editor group</strong>
        <span>Open a file from the Explorer to show it here.</span>
      </div>
    </>
  );
}

function EditorGroup({
  id,
  index,
  groupCount,
  tabs,
  activePath,
  directory,
  focused,
  editorStatus,
  onStatusChange,
  onValidationComplete,
  onSelect,
  onCloseTab,
  onMoveTab,
  onActivate,
  onSplit,
  onCloseGroup,
  hostedStrip
}: {
  id: EditorGroupID;
  index: number;
  groupCount: number;
  tabs: Tab[];
  activePath: string | null;
  directory: string | undefined;
  focused: boolean;
  editorStatus: EditorStatus | null;
  hostedStrip: boolean;
  onStatusChange: (status: EditorStatus) => void;
  onValidationComplete?: (report: ValidationReport) => void;
  onSelect: (path: string) => void;
  onCloseTab: (path: string) => void;
  onMoveTab: (path: string, destination: EditorGroupID, source: EditorGroupID) => void;
  onActivate: (id: EditorGroupID) => void;
  onSplit: (id: EditorGroupID) => void;
  onCloseGroup: (id: EditorGroupID) => void;
}): ReactNode {
  const [tabDragOver, setTabDragOver] = useState(false);
  const activeTab = tabs.find((tab) => tab.path === activePath);
  const title = editorGroupTitle(id, index);
  const actions = (
    <EditorGroupActions
      id={id}
      index={index}
      groupCount={groupCount}
      onSplit={onSplit}
      onCloseGroup={onCloseGroup}
    />
  );
  const onTabDragOver = (event: React.DragEvent): void => {
    if (!Array.from(event.dataTransfer.types).includes(EDITOR_TAB_MIME)) return;
    event.preventDefault();
    event.stopPropagation();
    event.dataTransfer.dropEffect = "move";
    setTabDragOver(true);
  };
  const onTabDrop = (event: React.DragEvent): void => {
    const transfer = readEditorTabTransfer(event.dataTransfer);
    if (!transfer) return;
    event.preventDefault();
    event.stopPropagation();
    setTabDragOver(false);
    onMoveTab(transfer.path, id, transfer.source);
  };

  const tabBar = (
    <TabBar
      tabs={tabs}
      activePath={activePath}
      ariaLabel={title + " tabs"}
      groupFor={() => id}
      onSelect={onSelect}
      onClose={onCloseTab}
      onMoveTab={onMoveTab}
    />
  );

  return (
    <section
      className={`editor-group editor-group-${id}${focused ? " editor-group-focused" : ""}${tabDragOver ? " editor-group-drop-target" : ""}`}
      data-editor-group={id}
      role="region"
      aria-label={title}
      onMouseDownCapture={() => onActivate(id)}
      onFocusCapture={() => onActivate(id)}
      onDragOver={onTabDragOver}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setTabDragOver(false);
      }}
      onDrop={onTabDrop}
    >
      {hostedStrip ? null : tabBar}
      {activeTab ? <>
        <BreadcrumbBar tab={activeTab} directory={directory} editorStatus={editorStatus} onValidationComplete={onValidationComplete} actions={actions} />
        <EditorWithSave key={id + ":" + activeTab.path} tab={activeTab} focused={focused} onStatusChange={onStatusChange} />
      </> : <EmptyEditorGroup actions={actions} />}
    </section>
  );
}

function EditorEmptyState({ session }: { session: SessionInfo | null | undefined }): ReactNode {
  return (
    <div className="editor-empty">
      <p>{session
        ? "Select a file from the explorer to view or edit it."
        : "Open a workspace from the explorer to get started."}</p>
    </div>
  );
}

export function EditorPane({ onValidationComplete, onSelectSurface, tabsSlot = null }: { onSelectSurface?: () => void; onValidationComplete?: (report: ValidationReport) => void; tabsSlot?: HTMLDivElement | null }): ReactNode {
  const { tabs, activePath, openPaths, session, setActive, closeTab } = useStore();
  const { browserTabs, activeBrowserTabID } = useBrowserTabState();
  const { openBrowserTab, closeBrowserTab, activateBrowserTab, updateBrowserTab } = useBrowserTabActions();
  const workspaceID = session?.workspace.id ?? null;
  const [externalDrag, setExternalDrag] = useState(false);
  const [editorStatus, setEditorStatus] = useState<EditorStatus | null>(null);
  const reportEditorStatus = (status: EditorStatus): void => {
    setEditorStatus((current) => sameEditorStatus(current, status) ? current : status);
  };
  const [groups, setGroups] = useState<EditorGroupState[]>(() => [{
    id: "primary",
    paths: tabs.map((tab) => tab.path),
    activePath: activePath ?? tabs[0]?.path ?? null
  }]);
  const [activeGroup, setActiveGroup] = useState<EditorGroupID>("primary");
  const [splitX, setSplitX] = useState(50);
  const [splitY, setSplitY] = useState(50);
  const { groupGridRef, startResize } = useGroupResize(workspaceID, groups.length);
  const activeGroupRef = useRef<EditorGroupID>("primary");
  const [groupWorkspaceID, setGroupWorkspaceID] = useState(workspaceID);
  const layoutsByWorkspaceRef = useRef(new Map<string, EditorLayoutState>());
  const activePathRef = useRef(activePath);

  const focusGroup = useCallback((group: EditorGroupID): void => {
    activeGroupRef.current = group;
    setActiveGroup(group);
  }, []);

  useEffect(() => {
    if (groupWorkspaceID === workspaceID) return;
    const paths = tabs.map((tab) => tab.path);
    layoutsByWorkspaceRef.current.set(groupWorkspaceID ?? "", {
      groups: groups.map((group) => ({ ...group, paths: [...group.paths] })),
      activeGroup: activeGroupRef.current,
      splitX,
      splitY
    });
    const savedLayout = layoutsByWorkspaceRef.current.get(workspaceID ?? "");
    setGroupWorkspaceID(workspaceID);
    activePathRef.current = activePath;
    setGroups(savedLayout?.groups ?? [{ id: "primary", paths, activePath: activePath ?? paths[0] ?? null }]);
    setSplitX(savedLayout?.splitX ?? 50);
    setSplitY(savedLayout?.splitY ?? 50);
    focusGroup(savedLayout?.activeGroup ?? "primary");
  }, [activeGroup, activePath, focusGroup, groupWorkspaceID, groups, splitX, splitY, tabs, workspaceID]);

  useEffect(() => {
    if (groupWorkspaceID !== workspaceID || activePathRef.current === activePath) return;
    activePathRef.current = activePath;
    const owner = activePath ? groups.find((group) => group.paths.includes(activePath)) : undefined;
    const destination = owner?.id ?? activeGroupRef.current;
    setGroups((current) => current.map((group) => group.id === destination
      ? { ...group, activePath }
      : group));
    focusGroup(destination);
  }, [activePath, focusGroup, groupWorkspaceID, groups, workspaceID]);

  useEffect(() => {
    if (groupWorkspaceID !== workspaceID) return;
    const livePaths = new Set(tabs.map((tab) => tab.path));
    const next = groups.map((group) => ({ ...group, paths: group.paths.filter((path) => livePaths.has(path)) }));
    const assigned = new Set(next.flatMap((group) => group.paths));
    const unassigned = tabs.map((tab) => tab.path).filter((path) => !assigned.has(path));
    const destinationIndex = Math.max(0, next.findIndex((group) => group.id === activeGroupRef.current));
    next[destinationIndex] = {
      ...next[destinationIndex],
      paths: [...next[destinationIndex]!.paths, ...unassigned]
    };
    for (const group of next) {
      if (activePath && group.paths.includes(activePath)) {
        group.activePath = activePath;
        continue;
      }
      if (group.activePath && group.paths.includes(group.activePath)) continue;
      group.activePath = group.paths[0] ?? null;
    }
    if (!sameGroupLayout(groups, next)) setGroups(next);
  }, [activePath, groupWorkspaceID, groups, tabs, workspaceID]);

  const selectPath = (groupID: EditorGroupID, path: string): void => {
    setGroups((current) => current.map((group) => group.id === groupID ? { ...group, activePath: path } : group));
    focusGroup(groupID);
    // Selecting a file tab leaves browser mode even when that file was already
    // the active one.
    activateBrowserTab(null);
    if (path !== activePath) setActive(path);
  };

  const activateGroup = (groupID: EditorGroupID): void => {
    focusGroup(groupID);
    const path = groups.find((group) => group.id === groupID)?.activePath;
    if (path && path !== activePath) setActive(path);
  };

  const moveTabToGroup = (path: string, destination: EditorGroupID, source: EditorGroupID): void => {
    if (!tabs.some((tab) => tab.path === path) || source === destination) return;
    const sourceGroup = groups.find((group) => group.id === source);
    const destinationGroup = groups.find((group) => group.id === destination);
    if (!sourceGroup?.paths.includes(path) || destinationGroup?.paths.includes(path)) return;
    setGroups((current) => current.map((group) => {
      if (group.id === source) {
        const paths = group.paths.filter((item) => item !== path);
        return { ...group, paths, activePath: group.activePath === path ? paths[0] ?? null : group.activePath };
      }
      if (group.id === destination) return { ...group, paths: [...group.paths, path], activePath: path };
      return group;
    }));
    focusGroup(destination);
    if (path !== activePath) setActive(path);
  };

  const splitGroupAfter = (sourceID: EditorGroupID): void => {
    if (groups.length >= EDITOR_GROUP_IDS.length) return;
    const sourceIndex = groups.findIndex((group) => group.id === sourceID);
    if (sourceIndex < 0) return;
    const source = groups[sourceIndex]!;
    const newID = EDITOR_GROUP_IDS.find((id) => !groups.some((group) => group.id === id));
    if (!newID) return;
    const movePath = source.paths.find((path) => path !== source.activePath) ?? null;
    const next = groups.map((group) => {
      if (group.id !== sourceID || !movePath) return group;
      const paths = group.paths.filter((path) => path !== movePath);
      return { ...group, paths, activePath: group.activePath ?? paths[0] ?? null };
    });
    const newGroup: EditorGroupState = { id: newID, paths: movePath ? [movePath] : [], activePath: movePath };
    next.splice(sourceIndex + 1, 0, newGroup);
    setGroups(next);
    focusGroup(newID);
    if (movePath && movePath !== activePath) setActive(movePath);
  };

  const closeGroup = (groupID: EditorGroupID): void => {
    if (groups.length <= 1) return;
    const index = groups.findIndex((group) => group.id === groupID);
    if (index <= 0) return;
    const closing = groups[index]!;
    const neighborIndex = index - 1;
    const next = groups.filter((group) => group.id !== groupID);
    const neighbor = next[neighborIndex]!;
    const paths = [...neighbor.paths, ...closing.paths.filter((path) => !neighbor.paths.includes(path))];
    next[neighborIndex] = { ...neighbor, paths };
    setGroups(next);
    focusGroup(neighbor.id);
    if (neighbor.activePath && neighbor.activePath !== activePath) setActive(neighbor.activePath);
  };

  const onDragOver = (event: React.DragEvent): void => {
    if (!isExternalFileDrag(event)) return;
    event.preventDefault();
    event.stopPropagation();
    setExternalDrag(true);
    event.dataTransfer.dropEffect = "copy";
  };
  const onDrop = (event: React.DragEvent): void => {
    if (!isExternalFileDrag(event)) return;
    event.preventDefault();
    event.stopPropagation();
    setExternalDrag(false);
    const group = event.target instanceof Element
      ? event.target.closest<HTMLElement>("[data-editor-group]")?.dataset.editorGroup
      : undefined;
    if (isEditorGroupID(group)) activateGroup(group);
    const files = droppedFilePaths(event);
    if (files.length > 0) void openPaths(files);
  };
  const onDragLeave = (event: React.DragEvent): void => {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setExternalDrag(false);
  };

  const stripTabs = useMemo(() => stripTabsFor(groups, tabs), [groups, tabs]);
  const browserActive = activeBrowserTabID !== null;
  const stripTabBar = (
    <TabBar
      tabs={stripTabs} activePath={browserActive ? null : activePath} ariaLabel="Open editor tabs"
      groupFor={(path) => groupOwningPath(groups, path)}
      onSelect={(path) => { selectPath(groupOwningPath(groups, path), path); onSelectSurface?.(); }}
      onClose={closeTab} onMoveTab={moveTabToGroup}
      browsers={browserTabs} activeBrowserTabID={activeBrowserTabID}
      onSelectBrowser={(id) => { activateBrowserTab(id); onSelectSurface?.(); }} onCloseBrowser={closeBrowserTab}
      onNewBrowser={() => { openBrowserTab(); onSelectSurface?.(); }}
    />
  );

  return (
    <div
      className={`editor-pane${externalDrag ? " external-drop-active" : ""}`}
      onDragOverCapture={onDragOver}
      onDropCapture={onDrop}
      onDragEnterCapture={onDragOver}
      onDragLeaveCapture={onDragLeave}
    >
      {tabsSlot && createPortal(stripTabBar, tabsSlot)}
      <BrowserStack tabs={browserTabs} activeID={activeBrowserTabID} onState={updateBrowserTab} />
      <div className={`editor-stack${browserActive ? " concealed" : ""}`}>
      {tabs.length === 0 && groups.length === 1 ? (
        <EditorEmptyState session={session} />
      ) : (
        <div
          ref={groupGridRef}
          className={`editor-groups editor-groups-count-${groups.length}`}
          data-group-count={groups.length}
          style={groups.length > 1 ? {
            gridTemplateColumns: `${splitX}% minmax(0, 1fr)`,
            ...(groups.length > 2 ? { gridTemplateRows: `${splitY}% minmax(0, 1fr)` } : {})
          } : undefined}
        >
          {groups.map((group, index) => <EditorGroup
            key={group.id}
            id={group.id}
            index={index}
            groupCount={groups.length}
            tabs={tabs.filter((tab) => group.paths.includes(tab.path))}
            activePath={group.activePath}
            directory={session?.directory}
            focused={activeGroup === group.id}
            hostedStrip={tabsSlot !== null}
            onStatusChange={reportEditorStatus}
            editorStatus={editorStatus}
            onValidationComplete={onValidationComplete}
            onSelect={(path) => selectPath(group.id, path)}
            onCloseTab={closeTab}
            onMoveTab={moveTabToGroup}
            onActivate={activateGroup}
            onSplit={splitGroupAfter}
            onCloseGroup={closeGroup}
          />)}
          {groups.length > 1 && <GroupResizeHandle axis="x" value={splitX} grid={groupGridRef.current} onChange={setSplitX} onStart={() => startResize("x", setSplitX)} height={groups.length === 3 ? `${splitY}%` : undefined} />}
          {groups.length > 2 && <GroupResizeHandle axis="y" value={splitY} grid={groupGridRef.current} onChange={setSplitY} onStart={() => startResize("y", setSplitY)} />}
        </div>
      )}
      </div>
    </div>
  );
}
