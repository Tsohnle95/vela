import type { ReactNode } from "react";
import type { SessionInfo } from "@shared/types";
import type { SidebarTab } from "../components/FileSidebar";
import { RailBrowserButton } from "./RailBrowserButton";
import { railToolClass, railToolTitle, sidebarViews } from "./rail";

// The left application rail. It lives beside App rather than inside it so the
// shell module stays within its size budget; the view state and callbacks are
// owned by the shell and passed in.
export function ActivityRail({
  activeTab,
  sideOpen,
  settingsOpen,
  changesOpen,
  session,
  trayOpen,
  traySnapped,
  onSelectTab,
  onToggleTerminal,
  onOpenBrowser,
  onToggleChanges,
  onToggleSettings
}: {
  activeTab: SidebarTab;
  sideOpen: boolean;
  settingsOpen: boolean;
  changesOpen: boolean;
  session: SessionInfo | null | undefined;
  trayOpen: boolean;
  traySnapped: boolean;
  onSelectTab: (tab: SidebarTab) => void;
  onToggleTerminal: () => void;
  onOpenBrowser: () => void;
  onToggleChanges: () => void;
  onToggleSettings: () => void;
}): ReactNode {
  const { sessions: sessionsView, files: filesView } = sidebarViews(settingsOpen, sideOpen, activeTab);
  return (
    <nav className="activity-rail" aria-label="Workspace views">
      <button
        className={railToolClass(sessionsView)}
        data-panel-action="show-sessions" aria-label="Sessions"
        aria-pressed={sessionsView}
        title={railToolTitle(sessionsView, "Sessions")}
        onClick={() => onSelectTab("sessions")}
      >
        <i className="codicon codicon-comment-discussion" aria-hidden="true" />
      </button>
      <button
        className={railToolClass(filesView)}
        data-panel-action="show-files"
        aria-label="Files"
        aria-pressed={filesView}
        title={railToolTitle(filesView, "Files")}
        onClick={() => onSelectTab("files")}
      >
        <i className="codicon codicon-files" aria-hidden="true" />
      </button>
      <button
        className={`activity-tool ${trayOpen ? "active" : ""}`}
        title={!session
          ? "No workspace open"
          : trayOpen
            ? (traySnapped ? "Expand terminal (⌥O)" : "Hide terminal (⌥O)")
            : "Show terminal (⌥O)"}
        aria-label="Terminal"
        onClick={onToggleTerminal}
      >
        <i className="codicon codicon-terminal" aria-hidden="true" />
      </button>
      <button
        className={railToolClass(changesOpen)}
        data-panel-action="toggle-changes"
        aria-label="Changes"
        aria-pressed={changesOpen}
        title={railToolTitle(changesOpen, "Changes")}
        onClick={onToggleChanges}
      >
        <i className="codicon codicon-diff" aria-hidden="true" />
      </button>
      <RailBrowserButton onOpen={onOpenBrowser} />
      <span className="activity-spacer" aria-hidden />
      <button
        className={`activity-tool ${settingsOpen ? "active" : ""}`}
        data-panel-action="toggle-settings"
        title={settingsOpen ? "Close settings" : "Settings"}
        aria-label="Settings"
        aria-pressed={settingsOpen}
        onClick={onToggleSettings}
      >
        <i className="codicon codicon-settings-gear" aria-hidden="true" />
      </button>
    </nav>
  );
}
