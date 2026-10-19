import { useState, type ReactNode } from "react";
import { useStore } from "../store";
import { droppedFilePaths } from "../drop";
import type { ProjectInfo, SessionSummary } from "@shared/types";
import { PlusIcon } from "./FileIcons";

function formatWhen(ts: number): string {
  if (!ts) return "";
  const mins = Math.floor((Date.now() - ts) / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days === 1) return "yesterday";
  if (days < 7) return new Date(ts).toLocaleDateString(undefined, { weekday: "short" });
  if (days < 30) return `${days}d ago`;
  return new Date(ts).toLocaleDateString();
}

const LIVE_WINDOW_MS = 60 * 60 * 1000;

function Chev(): ReactNode {
  return (
    <svg className="chev" viewBox="0 0 10 10" width="9" height="9" aria-hidden="true">
      <path d="M2 3.6 5 6.6 8 3.6" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function MarkSvg(): ReactNode {
  return (
    <svg className="mark-svg" viewBox="0 0 96 96" width="36" height="36" aria-hidden="true">
      <g fill="none" strokeLinecap="round">
        <ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(24 48 48)" stroke="color-mix(in srgb, var(--accent) 55%, var(--bg-panel))" strokeWidth="4.5" />
        <ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(-24 48 48)" stroke="var(--accent)" strokeWidth="5" />
        <circle cx="48" cy="48" r="9" fill="color-mix(in srgb, var(--accent) 72%, var(--text))" />
        <circle cx="62.1" cy="28.3" r="6" fill="var(--accent)" opacity="0.75" />
      </g>
    </svg>
  );
}

function FolderGlyph(): ReactNode {
  return (
    <svg viewBox="0 0 14 14" width="12" height="12" aria-hidden="true">
      <path d="M1.5 3.5h4l1.2 1.5h5.8v5.5a1 1 0 0 1-1 1h-10a1 1 0 0 1-1-1z" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinejoin="round" />
    </svg>
  );
}

type WorkspaceGroup = { directory: string; name: string; sessions: SessionSummary[] };

function workspaceGroups(sessions: SessionSummary[], projects: ProjectInfo[]): WorkspaceGroup[] {
  const byDirectory = new Map<string, { name: string; sessions: SessionSummary[] }>();
  for (const project of projects) {
    if (!project.directory) continue;
    byDirectory.set(project.directory, { name: project.name, sessions: [] });
  }
  for (const session of sessions) {
    const entry = byDirectory.get(session.directory);
    if (entry) entry.sessions.push(session);
  }
  return [...byDirectory.entries()]
    .map(([directory, { name, sessions: list }]) => ({
      directory,
      name,
      sessions: list.sort((left, right) => right.updatedAt - left.updatedAt)
    }))
    .sort((left, right) => {
      const leftTime = left.sessions[0]?.updatedAt ?? 0;
      const rightTime = right.sessions[0]?.updatedAt ?? 0;
      if (leftTime !== rightTime) return rightTime - leftTime;
      return left.name.localeCompare(right.name);
    });
}

export function Welcome(): ReactNode {
  const { selectFolder, openPaths, reopenSession, openSession, savedWorkspaces, saveWorkspace, sessions, connected } = useStore();
  const loading = !connected;
  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>({});
  const [closedSecs, setClosedSecs] = useState<Record<string, boolean>>({ recent: true, workspaces: true });

  const recentSessions = [...sessions].sort((left, right) => right.updatedAt - left.updatedAt).slice(0, 3);
  const groups = workspaceGroups(sessions, savedWorkspaces);

  const toggleGroup = (key: string): void =>
    setOpenGroups((current) => ({ ...current, [key]: !(current[key] ?? false) }));

  const toggleSec = (key: string): void =>
    setClosedSecs((current) => ({ ...current, [key]: !(current[key] ?? false) }));

  const isLive = (session: SessionSummary): boolean => Date.now() - session.updatedAt < LIVE_WINDOW_MS;

  const dropHandlers = {
    onDragOver: (e: React.DragEvent) => {
      const types = e.dataTransfer?.types;
      if (!types || !Array.from(types as ArrayLike<string>).includes("Files")) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    },
    onDrop: (e: React.DragEvent) => {
      e.preventDefault();
      const files = droppedFilePaths(e);
      if (files.length > 0) void openPaths(files);
    }
  };

  return (
    <div className="welcome" data-drag-region {...dropHandlers}>
      <div className="mock sk-tglass">
        <div className="twin">
          <div className="hero-col">
            <MarkSvg />
            <h1 className="title">Vela</h1>
            <p className="sub">One calm surface for coding agents.</p>
            <div className="cta-row">
              <button className="btn btn-primary" type="button" onClick={() => void selectFolder()}>
                Open a folder
              </button>
            </div>
          </div>

          <aside className="spanel" style={{ width: 248 }}>
            <div className={`sd-sec${closedSecs.recent ? " is-closed" : ""}`}>
              <button className="sd-sh style-dotcap" type="button" onClick={() => toggleSec("recent")}>
                <span className="sd-sh-label">Recent</span>
                <span className="sd-cnt">{recentSessions.length}</span>
                <Chev />
              </button>
              <div className="sd-body">
                {!loading && (
                  <ul className="num-list">
                    {recentSessions.map((session, index) => (
                      <li key={session.id} className={isLive(session) ? "is-live" : undefined}>
                        <button
                          className="rowlink"
                          type="button"
                          onClick={() => void reopenSession(session.id, false, session.directory)}
                          title={session.directory}
                        >
                          <span className="num-i">{String(index + 1).padStart(2, "0")}</span>
                          <span className="num-name">{session.title}</span>
                          <span className="num-when">{formatWhen(session.updatedAt)}</span>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>

            <div className={`sd-sec${closedSecs.workspaces ? " is-closed" : ""}`}>
              <div className="sd-sh-row">
                <button
                  className="sd-sh style-dotcap"
                  type="button"
                  aria-expanded={!closedSecs.workspaces}
                  onClick={() => toggleSec("workspaces")}
                >
                  <span className="sd-sh-label">Workspaces</span>
                  <span className="sd-cnt">{groups.length}</span>
                </button>
                {!closedSecs.workspaces && (
                  <button
                    className="sd-wg-new"
                    type="button"
                    title="Add a workspace"
                    aria-label="Add a workspace"
                    onClick={() => void saveWorkspace()}
                  >
                    <PlusIcon />
                  </button>
                )}
                <button
                  className="sd-sh-chevron"
                  type="button"
                  aria-label={`${closedSecs.workspaces ? "Expand" : "Collapse"} Workspaces`}
                  aria-expanded={!closedSecs.workspaces}
                  onClick={() => toggleSec("workspaces")}
                >
                  <Chev />
                </button>
              </div>
              <div className="sd-body">
                {!loading && groups.length === 0 && (
                  <div className="sessions-empty" style={{ padding: "8px 2px", fontSize: 11.5, color: "var(--text-faint)" }}>
                    No saved workspaces found.
                  </div>
                )}
                {!loading &&
                  groups.map((group) => {
                    const open = openGroups[group.directory] ?? false;
                    return (
                      <div className={`sd-grp${open ? " is-open" : ""}`} key={group.directory}>
                        <div className="sd-wgh-wrap">
                          <button
                            className="sd-wgh"
                            type="button"
                            aria-label={`Open workspace ${group.name}`}
                            onClick={() => void openSession(group.directory)}
                            title={group.directory}
                          >
                            <FolderGlyph />
                            <span className="sd-wgname">{group.name}</span>
                            <span className="sd-wgcnt">{group.sessions.length}</span>
                          </button>
                          <button
                            className={`sd-wgh-disclosure${open ? " open" : ""}`}
                            type="button"
                            aria-label={`${open ? "Collapse" : "Expand"} ${group.name} sessions`}
                            aria-expanded={open}
                            onClick={() => toggleGroup(group.directory)}
                          >
                            <Chev />
                          </button>
                        </div>
                        <div className="sd-kids-wrap">
                          {group.sessions.length === 0 ? (
                            <div className="sessions-empty" style={{ padding: "4px 0 6px 12px", fontSize: 11, color: "var(--text-faint)" }}>
                              No sessions yet.
                            </div>
                          ) : (
                            <ul className="rows sd-kids">
                              {group.sessions.map((session) => (
                                <li key={session.id} className={`row${isLive(session) ? " is-live" : ""}`}>
                                  <button
                                    className="rowlink"
                                    type="button"
                                    onClick={() => void reopenSession(session.id, false, session.directory)}
                                    title={session.title}
                                  >
                                    <span className="row-dot" />
                                    <span className="row-name">{session.title}</span>
                                    <span className="row-meta">{formatWhen(session.updatedAt)}</span>
                                  </button>
                                </li>
                              ))}
                            </ul>
                          )}
                        </div>
                      </div>
                    );
                  })}
              </div>
            </div>
          </aside>
        </div>
      </div>
    </div>
  );
}
