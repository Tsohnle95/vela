import type { SidebarTab } from "../components/FileSidebar";

export interface SidebarViews {
  sessions: boolean;
  files: boolean;
}

export function sidebarViews(settingsOpen: boolean, sideOpen: boolean, sideTab: SidebarTab): SidebarViews {
  return {
    sessions: !settingsOpen && sideOpen && sideTab === "sessions",
    files: !settingsOpen && sideOpen && sideTab === "files"
  };
}

export function sidebarViewVisible(settingsOpen: boolean, sideOpen: boolean, sideTab: SidebarTab, view: SidebarTab): boolean {
  return !settingsOpen && sideOpen && sideTab === view;
}

export function railToolClass(active: boolean): string {
  return `activity-tool${active ? " active" : ""}`;
}

export function railToolTitle(active: boolean, label: string): string {
  return active ? `Hide ${label}` : label;
}
