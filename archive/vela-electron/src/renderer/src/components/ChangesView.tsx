import { memo, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { AgentFileState } from "@shared/types";
import { useStore } from "../store";
import { OpenCodeFile } from "./OpenCodeFile";
import { FileIcon } from "./FileIcons";
import { IconChevronRight, IconClose } from "./icons";
import {
  buildChangeTreeRows,
  changeDiffStats,
  changeList,
  isHiddenChangePath,
  type ChangeFile,
  type ChangeTreeRow
} from "../changes-view-model";

// The editor-area Changes review surface: a directory tree of observed file
// changes beside stacked Pierre diffs, in the style of the reference app's
// changes panel. Per-file work (diff statistics and the Pierre diff itself) is
// deferred until a section nears the viewport: a large change set must not diff
// every file on mount, which previously froze the app when the panel opened.

const VIEWPORT_MARGIN = "600px 0px";

function togglePath(set: ReadonlySet<string>, value: string): Set<string> {
  const next = new Set(set);
  if (next.has(value)) next.delete(value);
  else next.add(value);
  return next;
}

/**
 * One IntersectionObserver for the whole view, keyed by each section's
 * `data-change-path`. Sections become permanently visible once they intersect,
 * so scrolling back does not re-trigger work. Where IntersectionObserver is
 * unavailable (tests/SSR) every section counts as visible.
 */
function useSectionVisibility(): {
  isVisible: (path: string) => boolean;
  observe: (element: HTMLElement) => void;
  unobserve: (element: HTMLElement) => void;
} {
  const [visible, setVisible] = useState<ReadonlySet<string>>(() => new Set());
  const supported = typeof IntersectionObserver !== "undefined";
  const registry = useRef(new Set<HTMLElement>());
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    if (!supported) return;
    const observer = new IntersectionObserver((entries) => {
      const entered: string[] = [];
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        observer.unobserve(entry.target);
        const path = (entry.target as HTMLElement).dataset.changePath;
        if (path) entered.push(path);
      }
      if (entered.length === 0) return;
      setVisible((current) => {
        let next: Set<string> | null = null;
        for (const path of entered) {
          if (current.has(path)) continue;
          next ??= new Set(current);
          next.add(path);
        }
        return next ?? current;
      });
    }, { rootMargin: VIEWPORT_MARGIN });
    observerRef.current = observer;
    for (const element of registry.current) observer.observe(element);
    return () => { observer.disconnect(); observerRef.current = null; };
  }, [supported]);

  const observe = useCallback((element: HTMLElement): void => {
    registry.current.add(element);
    observerRef.current?.observe(element);
  }, []);
  const unobserve = useCallback((element: HTMLElement): void => {
    registry.current.delete(element);
    observerRef.current?.unobserve(element);
  }, []);
  const isVisible = useCallback((path: string): boolean => !supported || visible.has(path), [supported, visible]);
  return { isVisible, observe, unobserve };
}

function changeStatusLabel(state: AgentFileState): string {
  if (state.deleted) return "Deleted";
  if (state.baseline.kind !== "known") return "Observed";
  return "Modified";
}

function ChangeStats({ state }: { state: AgentFileState }): ReactNode {
  const stats = useMemo(() => changeDiffStats(state), [state]);
  if (!stats) return null;
  return (
    <span className="change-stats" aria-label={`+${stats.additions} -${stats.deletions}`}>
      <span className="change-additions">+{stats.additions}</span>
      <span className="change-deletions">-{stats.deletions}</span>
    </span>
  );
}

function ChangeDiff({ file, wrap, visible }: { file: ChangeFile; wrap: boolean; visible: boolean }): ReactNode {
  const state = file.state;
  if (state.baseline.kind !== "known") {
    return (
      <div className="change-section-body">
        <p className="change-section-unavailable">
          Pre-change content wasn't observed for this file, so a diff isn't available.
        </p>
      </div>
    );
  }
  const after = state.deleted ? "" : state.content ?? "";
  return (
    <div className="change-section-body">
      {visible
        ? <OpenCodeFile mode="diff" file={file.path} before={state.baseline.content} after={after} overflow={wrap ? "wrap" : "scroll"} />
        : <div className="change-section-pending" aria-hidden="true" />}
    </div>
  );
}

function ChangeSection({ file, open, active, wrap, visible, onToggle, onOpenInEditor, onDismiss, onRegister, onUnregister }: {
  file: ChangeFile;
  open: boolean;
  active: boolean;
  wrap: boolean;
  visible: boolean;
  onToggle: (path: string) => void;
  onOpenInEditor: (path: string) => void;
  onDismiss: (path: string) => void;
  onRegister: (path: string, element: HTMLDivElement) => void;
  onUnregister: (path: string, element: HTMLDivElement) => void;
}): ReactNode {
  const path = file.path;
  const setRef = useCallback((element: HTMLDivElement | null): (() => void) | void => {
    if (!element) return;
    onRegister(path, element);
    return () => onUnregister(path, element);
  }, [path, onRegister, onUnregister]);
  return (
    <section className={`change-section${active ? " active" : ""}`} data-change-path={path} ref={setRef}>
      <header className="change-section-header">
        <button type="button" className="change-section-toggle" aria-expanded={open} onClick={() => onToggle(path)}>
          <span className={`change-section-chevron${open ? " open" : ""}`}><IconChevronRight /></span>
          <FileIcon name={file.name} isDir={false} />
          <span className="change-section-path">
            {file.directory && <span className="change-section-directory">{file.directory}/</span>}
            <span
              className="change-section-name"
              title="Open in editor"
              onClick={(event) => { event.stopPropagation(); onOpenInEditor(path); }}
            >{file.name}</span>
          </span>
        </button>
        <span className="change-section-status">{changeStatusLabel(file.state)}</span>
        {visible && <ChangeStats state={file.state} />}
        <button
          type="button"
          className="change-section-action"
          aria-label={`Clear ${path}`}
          title="Clear this change"
          onClick={() => onDismiss(path)}
        ><IconClose /></button>
      </header>
      {open && <ChangeDiff file={file} wrap={wrap} visible={visible} />}
    </section>
  );
}

const ChangeSectionMemo = memo(ChangeSection);

function ChangesTree({ rows, collapsed, selected, onToggleDirectory, onSelect }: {
  rows: ChangeTreeRow[];
  collapsed: ReadonlySet<string>;
  selected: string | null;
  onToggleDirectory: (path: string) => void;
  onSelect: (path: string) => void;
}): ReactNode {
  return (
    <nav className="changes-tree" aria-label="Changed files">
      {rows.map((row) => {
        const indent = { paddingInlineStart: `${8 + row.depth * 12}px` };
        return row.kind === "directory"
          ? <button
              key={row.key}
              type="button"
              className="changes-tree-row directory"
              style={indent}
              aria-expanded={!collapsed.has(row.path)}
              onClick={() => onToggleDirectory(row.path)}
            >
              <span className={`changes-tree-chevron${collapsed.has(row.path) ? "" : " open"}`}><IconChevronRight /></span>
              <span className="changes-tree-label">{row.label}</span>
            </button>
          : <button
              key={row.key}
              type="button"
              className={`changes-tree-row file${selected === row.path ? " active" : ""}`}
              style={indent}
              title={row.path}
              onClick={() => onSelect(row.path)}
            >
              <FileIcon name={row.label} isDir={false} />
              <span className="changes-tree-label">{row.label}</span>
            </button>;
      })}
    </nav>
  );
}

function ChangesToolbar({ count, allCollapsed, wrap, onToggleAll, onToggleWrap, onClearAll, onClose }: {
  count: number;
  allCollapsed: boolean;
  wrap: boolean;
  onToggleAll: () => void;
  onToggleWrap: () => void;
  onClearAll: () => void;
  onClose: () => void;
}): ReactNode {
  return (
    <div className="changes-toolbar">
      <span className="changes-title">Changes</span>
      <span className="changes-count">{count}</span>
      <span className="changes-spacer" />
      {count > 0 && <>
        <button type="button" className="changes-action" onClick={onToggleAll}>{allCollapsed ? "Expand all" : "Collapse all"}</button>
        <button
          type="button"
          className={`changes-action${wrap ? " on" : ""}`}
          aria-pressed={wrap}
          title={wrap ? "Disable line wrap" : "Enable line wrap"}
          onClick={onToggleWrap}
        >Wrap</button>
        <button type="button" className="changes-action danger" onClick={onClearAll}>Clear all</button>
      </>}
      <button type="button" className="changes-action" aria-label="Close changes view" title="Close changes view" onClick={onClose}><IconClose /></button>
    </div>
  );
}

export function ChangesView({ onClose }: { onClose: () => void }): ReactNode {
  const { session, agentFiles, hiddenPaths, dismissChange, dismissChanges, previewFile } = useStore();
  const files = useMemo(() => {
    const visible = changeList(agentFiles);
    // Paths detached from this workspace, or owned by a nested workspace that is
    // open separately, are hidden in the Explorer and must not resurface here.
    return hiddenPaths.size === 0 ? visible : visible.filter((file) => !isHiddenChangePath(file.path, hiddenPaths));
  }, [agentFiles, hiddenPaths]);
  const [collapsedDirs, setCollapsedDirs] = useState<ReadonlySet<string>>(() => new Set());
  const [collapsedSections, setCollapsedSections] = useState<ReadonlySet<string>>(() => new Set());
  const [wrap, setWrap] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);
  const sections = useRef(new Map<string, HTMLDivElement>());
  const rows = useMemo(() => buildChangeTreeRows(files, collapsedDirs), [files, collapsedDirs]);
  const allCollapsed = files.length > 0 && files.every((file) => collapsedSections.has(file.path));
  const { isVisible, observe, unobserve } = useSectionVisibility();

  const registerSection = useCallback((path: string, element: HTMLDivElement): void => {
    sections.current.set(path, element);
    observe(element);
  }, [observe]);

  const unregisterSection = useCallback((path: string, element: HTMLDivElement): void => {
    sections.current.delete(path);
    unobserve(element);
  }, [unobserve]);

  const toggleSection = useCallback((path: string): void => {
    setCollapsedSections((current) => togglePath(current, path));
  }, []);

  const reveal = (path: string): void => {
    setSelected(path);
    setCollapsedSections((current) => current.has(path) ? togglePath(current, path) : current);
    requestAnimationFrame(() => sections.current.get(path)?.scrollIntoView({ block: "start" }));
  };

  const openInEditor = useCallback((path: string): void => {
    onClose();
    void previewFile(path);
  }, [onClose, previewFile]);

  if (files.length === 0) {
    return (
      <section className="changes-view" aria-label="Changes">
        <ChangesToolbar count={0} allCollapsed={false} wrap={wrap}
          onToggleAll={() => {}} onToggleWrap={() => setWrap((value) => !value)} onClearAll={dismissChanges} onClose={onClose} />
        <div className="changes-empty">
          {session ? "No changes yet. Files the agent edits appear here." : "Open a workspace to review changes."}
        </div>
      </section>
    );
  }

  return (
    <section className="changes-view" aria-label="Changes">
      <ChangesToolbar
        count={files.length}
        allCollapsed={allCollapsed}
        wrap={wrap}
        onToggleAll={() => setCollapsedSections(allCollapsed ? new Set() : new Set(files.map((file) => file.path)))}
        onToggleWrap={() => setWrap((value) => !value)}
        onClearAll={() => { dismissChanges(); setSelected(null); }}
        onClose={onClose}
      />
      <div className="changes-body">
        <ChangesTree
          rows={rows}
          collapsed={collapsedDirs}
          selected={selected}
          onToggleDirectory={(path) => setCollapsedDirs((current) => togglePath(current, path))}
          onSelect={reveal}
        />
        <div className="changes-diffs">
          {files.map((file) => (
            <ChangeSectionMemo
              key={file.path}
              file={file}
              open={!collapsedSections.has(file.path)}
              active={selected === file.path}
              wrap={wrap}
              visible={isVisible(file.path)}
              onToggle={toggleSection}
              onOpenInEditor={openInEditor}
              onDismiss={dismissChange}
              onRegister={registerSection}
              onUnregister={unregisterSection}
            />
          ))}
        </div>
      </div>
    </section>
  );
}
