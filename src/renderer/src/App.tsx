import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { useAppStyles } from "./app-styles";
import { useStore } from "./store";
import { IconChevronDown } from "./components/icons";
import type { SessionInfo } from "@shared/types";
import type { ValidationReport } from "./validation-report";
import { FileSidebar, type SidebarTab } from "./components/FileSidebar";
import { EditorPane } from "./components/EditorPane";
import { AgentPanel } from "./components/AgentPanel";
import { TerminalTray } from "./components/TerminalTray";
import { RecoveryNotice } from "./components/RecoveryNotice";
import { Titlebar } from "./components/Titlebar";
import { SettingsPage } from "./components/SettingsPage";
import { SettingsSidebar, type SettingsSection } from "./components/SettingsSidebar";
import { protectEditorUnload } from "./editor-close-guard";
import { ThemeProvider } from "./theme";
import { agentPanelGrid, panelSlotFor, distributePanelSlots, MAX_AGENT_PANELS, AGENT_DEFAULT_W, AGENT_MIN_W, EDITOR_MIN_VISIBLE_W, type PanelSlot } from "./agent-panels";
import { PanelColumn } from "./layout/PanelColumn";
import { sidebarViewVisible } from "./layout/rail";
import { useDragResize, useTitlebarTabsOffset, useTrayHeight } from "./layout/panel-interactions";
import { ActivityRail } from "./layout/ActivityRail";
import { ChangesView } from "./components/ChangesView";
import { BrowserTabsProvider, useBrowserTabActions } from "./browser-tabs-context";
import { BrowserStoreProvider } from "./browser-store-provider";

const SIDE_MIN_W = 230;
const SIDE_MAX_W = 520;
const SIDE_DEFAULT_W = 234;

function clampSideWidth(width: number): number {
  return Math.max(SIDE_MIN_W, width);
}
function EmptyTerminalTray({ onClose }: { onClose: () => void }): ReactNode {
  return (
    <div className="terminal-tray">
      <div className="terminal-header">
        <span className="terminal-notice">Open a workspace to use the terminal.</span>
        <button className="terminal-close" title="Close the terminal panel (⌥O)" onClick={onClose}>
          <IconChevronDown />
        </button>
      </div>
    </div>
  );
}

function Layout({ children }: { children?: ReactNode }): ReactNode {
  const {
    session,
    activeSessions,
    panels: allPanels,
    workspaceOnlyPanelIDs,
    activeSessionID,
    focusSession,
    closePanel,
    reopenSession,
    selectAddPanel
  } = useStore();
  const { openBrowserTab } = useBrowserTabActions();
  const agentPanels = useMemo(() => {
    const agentPanels = allPanels.filter((panel) => !workspaceOnlyPanelIDs.has(panel.id));
    const active = allPanels.find((panel) => panel.id === activeSessionID);
    if (!active || !workspaceOnlyPanelIDs.has(active.id)) return agentPanels;
    return [active, ...agentPanels.slice(1)];
  }, [allPanels, workspaceOnlyPanelIDs, activeSessionID]);
  const [sideOpen, setSideOpen] = useState(true);
  const [sideW, setSideW] = useState(SIDE_DEFAULT_W);
  const [slots, setSlots] = useState<Record<string, PanelSlot>>({});
  const [pendingModelPanels, setPendingModelPanels] = useState(0);
  const trayAreaRef = useRef<HTMLDivElement>(null);
  const workspaceAreaRef = useRef<HTMLDivElement>(null);
  const workspaceHostRef = useRef<HTMLDivElement>(null);
  const titlebarLeadingRef = useRef<HTMLSpanElement>(null);
  const [tabsSlot, setTabsSlot] = useState<HTMLDivElement | null>(null);
  const { height: trayH, open: trayOpen, snapped: traySnapped, dragging: trayDragging, toggle: toggleTray, show: showTray, close: closeTray, expand: expandTray, onDrag: trayDrag } = useTrayHeight(
    (height) => {
      trayAreaRef.current?.style.setProperty("--tray-height", `${height}px`);
      workspaceAreaRef.current?.style.setProperty("--tray-row", `${height}px`);
    }
  );
  const [terminalRequest, setTerminalRequest] = useState<{ id: number; directory: string } | null>(null);
  const [sideTab, setSideTab] = useState<SidebarTab>("files");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [changesOpen, setChangesOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState<SettingsSection>("appearance");
  const [validationReport, setValidationReport] = useState<ValidationReport | null>(null);
  const [terminalPanel, setTerminalPanel] = useState<"terminal" | "problems" | "sass">("terminal");
  const [agentModeActive, setAgentModeActive] = useState(false);
  const [agentModePanelIDs, setAgentModePanelIDs] = useState<string[]>([]);
  const [emptyAgentOpen, setEmptyAgentOpen] = useState(true);
  const [emptyAgentWidth, setEmptyAgentWidth] = useState(AGENT_DEFAULT_W);
  const mainRowRef = useRef<HTMLDivElement>(null);
  const [measuredAreaW, setMeasuredAreaW] = useState(0);
  const [winW, setWinW] = useState(() => window.innerWidth);
  const emptyAgentRef = useRef<HTMLDivElement>(null);
  const prevSidebarRef = useRef<{ open: boolean; width: number } | null>(null);
  const inAgentMode = agentModeActive;

  const codingPanel = useMemo(
    () => allPanels.find((panel) => panel.id === activeSessionID) ?? agentPanels[0] ?? null,
    [activeSessionID, agentPanels, allPanels]
  );
  const panels = useMemo(() => {
    if (!inAgentMode) return codingPanel ? [codingPanel] : [];
    const attached = new Map(agentPanels.map((panel) => [panel.id, panel]));
    return agentModePanelIDs
      .map((id) => attached.get(id))
      .filter((panel): panel is SessionInfo => Boolean(panel));
  }, [agentModePanelIDs, agentPanels, codingPanel, inAgentMode]);
  const sessionChoices = useMemo(() => {
    const seen = new Set<string>();
    const choices: SessionInfo[] = [];
    for (const choice of [...activeSessions, ...allPanels]) {
      if (seen.has(choice.id)) continue;
      seen.add(choice.id);
      choices.push(choice);
    }
    return choices;
  }, [activeSessions, allPanels]);

  useEffect(() => {
    if (!inAgentMode) return;
    setAgentModePanelIDs((current) => {
      const validIDs = new Set(agentPanels.map((panel) => panel.id));
      const missingIndex = current.findIndex((id) => !validIDs.has(id));
      const valid = current.filter((id) => validIDs.has(id));
      const focused = activeSessionID ? agentPanels.find((panel) => panel.id === activeSessionID) : null;
      if (missingIndex >= 0 && focused && !valid.includes(focused.id)) {
        valid.splice(Math.min(missingIndex, valid.length), 0, focused.id);
      }
      if (valid.length > 0) return valid;
      return codingPanel ? [codingPanel.id] : [];
    });
  }, [activeSessionID, agentPanels, codingPanel, inAgentMode]);

  const sideShown = sideOpen ? sideW : 0;
  const areaW = measuredAreaW > 0
    ? measuredAreaW
    : Math.max(0, winW - sideShown - (sideOpen ? 1 : 0));

  useLayoutEffect(() => {
    const area = workspaceAreaRef.current;
    if (!area) return;
    const measure = (): void => {
      if (area.clientWidth > 0) setMeasuredAreaW(area.clientWidth);
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(area);
    return () => observer.disconnect();
  }, [settingsOpen]);

  useTitlebarTabsOffset(titlebarLeadingRef, workspaceHostRef, [sideOpen, sideW, settingsOpen, winW]);

  useEffect(() => {
    const onResize = (): void => setWinW(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  const wasOpenWorkspacesRef = useRef(0);
  useEffect(() => {
    if (allPanels.length > 0 && wasOpenWorkspacesRef.current === 0) setSideW(SIDE_MIN_W);
    wasOpenWorkspacesRef.current = allPanels.length;
  }, [allPanels.length]);

  // Opening or dropping a workspace lands the user on the file tree rather
  // than the session switcher. Track workspace identities rather than panel
  // count so subagent panels in an already-open workspace do not steal focus.
  const seenWorkspacesRef = useRef<Set<string>>(new Set<string>());
  useEffect(() => {
    let opened = false;
    for (const panel of allPanels) {
      if (seenWorkspacesRef.current.has(panel.workspace.id)) continue;
      seenWorkspacesRef.current.add(panel.workspace.id);
      opened = true;
    }
    if (opened) setSideTab("files");
  }, [allPanels]);

  const prevPanelsRef = useRef<SessionInfo[] | null>(null);
  const sessionSlotsRef = useRef(new Map<string, PanelSlot>());
  const rememberedSlot = (panel: SessionInfo): PanelSlot | undefined =>
    sessionSlotsRef.current.get(panel.id) ??
    (panel.parentID ? sessionSlotsRef.current.get(panel.parentID) : undefined);
  useLayoutEffect(() => {
    const prev = prevPanelsRef.current;
    prevPanelsRef.current = panels;
    if (!prev || prev.length !== panels.length) return;
    // Changing the selected singleton in normal coding mode is a view swap,
    // not a workspace replacement. Keep the target's own slot instead of
    // migrating the previous panel's geometry onto it.
    if (!inAgentMode && panels.length === 1 && prev[0]?.id !== panels[0]?.id) return;
    for (let index = 0; index < panels.length; index += 1) {
      const before = prev[index];
      const after = panels[index];
      if (before.id === after.id) continue;
      setSlots((current) => {
        const migrated = current[before.workspace.id];
        if (!migrated || current[after.workspace.id] === migrated) return current;
        const next = { ...current, [after.workspace.id]: migrated };
        delete next[before.workspace.id];
        return next;
      });
    }
  }, [panels]);

  useLayoutEffect(() => {
    for (const panel of panels) {
      const slot = slots[panel.workspace.id];
      if (slot) sessionSlotsRef.current.set(panel.id, slot);
    }
  }, [panels, slots]);

  useEffect(() => {
    setSlots((current) => {
      const anchorId = panels[0]?.workspace.id ?? null;
      const anchorStored = anchorId ? current[anchorId] : undefined;
      const anchorWidth = anchorStored ? (anchorStored.open ? anchorStored.width : 0) : AGENT_DEFAULT_W;
      const anchorLeft = Math.max(0, areaW - anchorWidth);
      const next: Record<string, PanelSlot> = {};
      let changed = false;
      for (const panel of panels) {
        const id = panel.workspace.id;
        const existing = current[id] ?? rememberedSlot(panel);
        if (existing) {
          next[id] = existing;
          continue;
        }
        if (id === anchorId) {
          next[id] = { open: true, width: AGENT_DEFAULT_W, left: anchorLeft, top: 0, height: 100 };
          changed = true;
          continue;
        }
        let left = anchorLeft - AGENT_DEFAULT_W;
        for (const other of panels) {
          if (other.workspace.id === id || other.workspace.id === anchorId) continue;
          const s = next[other.workspace.id] ?? current[other.workspace.id];
          if (!s) continue;
          left = Math.min(left, s.left - AGENT_DEFAULT_W);
        }
        next[id] = { open: true, width: AGENT_DEFAULT_W, left: Math.max(0, left), top: 0, height: 100 };
        changed = true;
      }
      if (Object.keys(current).length !== Object.keys(next).length) changed = true;
      return changed ? next : current;
    });
  }, [panels, areaW]);

  const slotShown = (panel: SessionInfo): number => {
    const slot = slots[panel.workspace.id] ?? rememberedSlot(panel) ?? { open: true, width: AGENT_DEFAULT_W, left: 0, top: 0, height: 100 };
    return slot.open ? slot.width : 0;
  };
  const slotFor = (panel: SessionInfo): PanelSlot =>
    panelSlotFor(panel, panels, slots, slots[panel.workspace.id] ? undefined : rememberedSlot(panel), areaW, inAgentMode);

  const ordered = [...panels].sort((a, b) => slotFor(a).left - slotFor(b).left);

  const singlePanel = panels.length <= 1;
  const agentShown = singlePanel && panels.length === 1
    ? slotShown(panels[0])
    : 0;
  const sideMax = Math.max(SIDE_MIN_W, Math.min(SIDE_MAX_W, areaW + sideShown - agentShown));

  const sideCapRef = useRef<number | null>(null);
  const agentCapRef = useRef<number | null>(null);

  const slotsRef = useRef(slots);
  slotsRef.current = slots;

  useLayoutEffect(() => {
    const current = slotsRef.current;
    const shown = (panel: SessionInfo): number => {
      const slot = current[panel.workspace.id] ?? rememberedSlot(panel) ?? { open: true, width: AGENT_DEFAULT_W, left: 0, top: 0, height: 100 };
      return slot.open ? slot.width : 0;
    };
    if (prevSidebarRef.current !== null) {
      const avail = Math.max(0, areaW + sideShown);
      const openIDs = panels
        .filter((panel) => current[panel.workspace.id]?.open ?? true)
        .map((panel) => panel.workspace.id);
      if (openIDs.length === 0) return;
      const gridIDs = [...panels].reverse()
        .map((panel) => panel.workspace.id)
        .filter((id) => openIDs.includes(id));
      const { columns, rows } = agentPanelGrid(gridIDs.length);
      const base = Math.max(AGENT_MIN_W, Math.floor(avail / columns));
      setSlots((slots) => {
        const next = { ...slots };
        gridIDs.forEach((id, index) => {
          const column = index % columns;
          const row = Math.floor(index / columns);
          next[id] = {
            ...(next[id] ?? { open: true, width: base, left: 0, top: 0, height: 100 }),
            open: true,
            width: gridIDs.length === 1
              ? AGENT_DEFAULT_W
              : column === columns - 1
                ? Math.max(AGENT_MIN_W, avail - base * (columns - 1))
                : base,
            left: column * base,
            top: row * (100 / rows),
            height: 100 / rows
          };
        });
        return next;
      });
      return;
    }
    if (panels.length <= 1) {
      if (panels.length === 0) return;
      const panel0 = panels[0];
      const panelOpen = current[panel0.workspace.id]?.open ?? true;
      if (!sideOpen && !panelOpen) return;
      const avail = Math.max(0, areaW + sideShown);
      const agentShownNow = panelOpen ? shown(panel0) : 0;
      const agentLimit = Math.max(0, avail - sideShown);
      const sideLimit = Math.max(0, Math.min(SIDE_MAX_W, avail - agentShownNow));
      const agentAnchored = panelOpen && agentCapRef.current !== null && agentShownNow >= agentCapRef.current - 1;
      const sideAnchored = sideOpen && sideCapRef.current !== null && sideW >= sideCapRef.current - 1;
      if (panelOpen) agentCapRef.current = agentLimit;
      if (sideOpen) sideCapRef.current = sideLimit;
      const total = sideShown + agentShownNow;
      if (total <= avail) return;
      if (!sideOpen || !panelOpen) {
        if (panelOpen) setSlotWidth(panel0.workspace.id, agentLimit);
        else setSideW(clampSideWidth(sideLimit));
        return;
      }
      if (agentAnchored !== sideAnchored) {
        if (agentAnchored) setSlotWidth(panel0.workspace.id, agentLimit);
        else setSideW(clampSideWidth(sideLimit));
        return;
      }
      if (agentAnchored) {
        const nextAgent = Math.max(AGENT_MIN_W, agentLimit);
        setSlotWidth(panel0.workspace.id, nextAgent);
        setSideW(clampSideWidth(Math.min(sideShown, avail - nextAgent)));
        return;
      }
      const nextSide = clampSideWidth(Math.round((sideShown * avail) / total));
      const nextAgent = Math.max(AGENT_MIN_W, avail - nextSide);
      setSideW(nextSide);
      setSlotWidth(panel0.workspace.id, nextAgent);
      return;
    }
    const avail = areaW;
    const openIDs = panels
      .filter((panel) => current[panel.workspace.id]?.open)
      .map((panel) => panel.workspace.id);
    if (openIDs.length === 0) return;
    const totalShown = panels.reduce((sum, panel) => sum + shown(panel), 0);
    if (totalShown > avail) {
      const base = Math.max(AGENT_MIN_W, Math.floor(avail / openIDs.length));
      for (const id of openIDs) setSlotWidth(id, Math.min(current[id]?.width ?? base, base));
    }
    const anchorId = panels[0]?.workspace.id ?? null;
    const anchorStored = anchorId ? current[anchorId] : undefined;
    const anchorShown = anchorStored ? (anchorStored.open ? anchorStored.width : 0) : AGENT_DEFAULT_W;
    const anchorLeft = Math.max(0, areaW - anchorShown);
    const others = panels
      .filter((panel) => panel.workspace.id !== anchorId)
      .sort((a, b) => (current[b.workspace.id]?.left ?? 0) - (current[a.workspace.id]?.left ?? 0));
    let boundary = anchorLeft;
    for (const panel of others) {
      const slot = current[panel.workspace.id];
      if (!slot) continue;
      const w = slot.open ? slot.width : 0;
      const capped = Math.max(0, Math.min(slot.left, boundary - w));
      if (capped !== slot.left) setSlotLeft(panel.workspace.id, capped);
      boundary = Math.min(boundary, capped);
    }
  }, [areaW, sideOpen, sideW, panels]);

  const sideDrag = useDragResize(
    sideW,
    setSideW,
    SIDE_MIN_W,
    sideMax,
    false,
    sideOpen,
    undefined,
    undefined,
    undefined,
    (width) => mainRowRef.current?.style.setProperty("--pane-columns", `${width}px 1px minmax(0,1fr)`)
  );
  const emptyAgentDrag = useDragResize(
    emptyAgentWidth,
    setEmptyAgentWidth,
    AGENT_MIN_W,
    areaW,
    true,
    emptyAgentOpen,
    undefined,
    undefined,
    undefined,
    (width) => {
      if (emptyAgentRef.current) emptyAgentRef.current.style.width = `${width}px`;
      emptyAgentRef.current?.parentElement?.style.setProperty("--editor-right", `${width}px`);
    }
  );

  const cols = [
    sideOpen ? `${sideW}px` : "0px",
    sideOpen ? "1px" : "0px",
    "minmax(0,1fr)"
  ].join(" ");

  const setSidebarOpen = (open: boolean): void => {
    setSideOpen(open);
  };

  const anchorId = panels[0]?.workspace.id ?? null;
  const anchorOpen = anchorId ? slots[anchorId]?.open ?? true : emptyAgentOpen;
  const emptyAgentShown = Math.min(emptyAgentWidth, Math.max(0, areaW - EDITOR_MIN_VISIBLE_W - 10));
  // Once the leftmost agent panel reaches the workspace's left edge the editor
  // has no visible column left. Keep it mounted but stop painting so its 1px
  // border cannot peek through the panel's rounded left corners.
  const editorRight = ordered.length > 0
    ? Math.max(0, areaW - slotFor(ordered[0]).left)
    : emptyAgentOpen && !inAgentMode ? emptyAgentShown : 0;
  const editorCovered = areaW > 0 && areaW - editorRight - 10 <= 2;
  const setSlotOpen = (id: string | null, open: boolean): void => {
    if (!id) return;
    setSlots((current) => {
      const slot = current[id] ?? { open: true, width: AGENT_DEFAULT_W, left: 0, top: 0, height: 100 };
      if (slot.open === open) return current;
      return { ...current, [id]: { ...slot, open } };
    });
  };

  const distributeEvenly = useCallback((singleRestore: boolean, targetPanels: SessionInfo[] = panels, modelMode = prevSidebarRef.current !== null): void => {
    setSlots((current) => distributePanelSlots(current, targetPanels, areaW, singleRestore, modelMode));
  }, [areaW, panels]);

  const previousPanelCountRef = useRef(panels.length);
  useEffect(() => {
    if (inAgentMode && panels.length > previousPanelCountRef.current) {
      distributeEvenly(false, panels, true);
    }
    previousPanelCountRef.current = panels.length;
  }, [distributeEvenly, inAgentMode, panels, sideShown]);

  useLayoutEffect(() => {
    if (inAgentMode && sideOpen) distributeEvenly(false, panels, true);
  }, [distributeEvenly, inAgentMode, panels, sideOpen, sideW]);

  const addModelPanel = (): void => {
    if (panels.length + pendingModelPanels >= MAX_AGENT_PANELS) return;
    setPendingModelPanels((count) => count + 1);
    void selectAddPanel()
      .then((opened) => {
        if (!opened) return;
        if (!inAgentMode) toggleAgentMode();
        setAgentModePanelIDs((current) => current.includes(opened.id) ? current : [...current, opened.id]);
      })
      .finally(() => setPendingModelPanels((count) => Math.max(0, count - 1)));
  };

  const selectAgentSession = useCallback(async (sessionID: string): Promise<void> => {
    const attached = allPanels.find((panel) => panel.id === sessionID);
    if (!inAgentMode) {
      if (attached) {
        focusSession(sessionID);
      } else {
        await reopenSession(sessionID);
      }
      return;
    }
    const opened = attached ?? await reopenSession(sessionID, true);
    if (!opened) return;
    if (!panels.some((panel) => panel.id === opened.id) && panels.length >= MAX_AGENT_PANELS) return;
    setAgentModePanelIDs((current) => current.includes(opened.id) ? current : [...current, opened.id]);
    focusSession(opened.id);
  }, [allPanels, focusSession, inAgentMode, panels, reopenSession]);

  const toggleAgentMode = (): void => {
    if (!inAgentMode) {
      const seed = codingPanel;
      setAgentModePanelIDs(seed ? [seed.id] : []);
      prevSidebarRef.current = { open: sideOpen, width: sideW };
      setAgentModeActive(true);
      setSettingsOpen(false);
      setSideOpen(false);
      distributeEvenly(false, seed ? [seed] : [], true);
    } else {
      const prev = prevSidebarRef.current;
      if (!prev) return;
      const focused = allPanels.find((panel) => panel.id === activeSessionID) ?? codingPanel;
      setAgentModePanelIDs(focused ? [focused.id] : []);
      prevSidebarRef.current = null;
      setAgentModeActive(false);
      setSideOpen(prev.open);
      setSideW(prev.width);
      distributeEvenly(true, focused ? [focused] : [], false);
      if (focused) focusSession(focused.id);
    }
  };

  const leaveAgentModeForManualAdjustment = (): void => {
    if (!inAgentMode) return;
    const focused = allPanels.find((panel) => panel.id === activeSessionID) ?? codingPanel;
    setAgentModePanelIDs(focused ? [focused.id] : []);
    prevSidebarRef.current = null;
    setAgentModeActive(false);
  };

  const closeAgentPanel = (panel: SessionInfo): void => {
    if (inAgentMode && panels.length > 1) {
      setAgentModePanelIDs((current) => current.filter((id) => id !== panel.id));
      closePanel(panel.id);
      return;
    }
    if (!inAgentMode && agentPanels.length > 1) {
      closePanel(panel.id);
      return;
    }
    if (inAgentMode) {
      const previous = prevSidebarRef.current;
      prevSidebarRef.current = null;
      setAgentModeActive(false);
      if (previous) {
        setSideOpen(previous.open);
        setSideW(previous.width);
      }
    }
    setSlotOpen(panel.workspace.id, false);
  };

  const activateSidebarTab = (tab: SidebarTab): void => {
    if (inAgentMode) {
      toggleAgentMode();
      setSettingsOpen(false);
      setSideTab(tab);
      setSidebarOpen(true);
      return;
    }
    // Clicking the already-active rail view collapses the sidebar (the titlebar toggle moved to the rail).
    if (sidebarViewVisible(settingsOpen, sideOpen, sideTab, tab)) {
      setSidebarOpen(false);
      return;
    }
    setSettingsOpen(false);
    setSideTab(tab);
    setSidebarOpen(true);
  };

  const openSettings = (): void => {
    setSidebarOpen(true);
    setSettingsSection("appearance");
    setChangesOpen(false);
    setSettingsOpen(true);
  };

  const setSlotWidth = (id: string, width: number): void => {
    setSlots((current) => {
      const slot = current[id] ?? { open: true, width: AGENT_DEFAULT_W, left: 0, top: 0, height: 100 };
      if (slot.width === width) return current;
      return { ...current, [id]: { ...slot, width } };
    });
  };

  const setSlotLeft = (id: string, left: number): void => {
    setSlots((current) => {
      const slot = current[id] ?? { open: true, width: AGENT_DEFAULT_W, left, top: 0, height: 100 };
      if (slot.left === left) return current;
      return { ...current, [id]: { ...slot, left } };
    });
  };

  const toggleSidebarFromTitlebar = (): void => {
    if (inAgentMode) {
      toggleAgentMode();
      setSideOpen(true);
      return;
    }
    setSidebarOpen(!sideOpen);
  };

  const toggleAgentPanel = (): void => {
    if (anchorId) setSlotOpen(anchorId, !anchorOpen);
    else setEmptyAgentOpen(!anchorOpen);
  };

  return (
    <div className={`app ${inAgentMode ? "agent-mode" : ""} ${changesOpen ? "changes-open" : ""}`}>
      <Titlebar
        sidebarOpen={!inAgentMode && sideOpen}
        agentMode={inAgentMode}
        agentPanelOpen={anchorOpen}
        onToggleSidebar={toggleSidebarFromTitlebar}
        onToggleAgentMode={toggleAgentMode}
        onToggleAgentPanel={toggleAgentPanel}
        onAddAgentPanel={addModelPanel}
        canAddAgentPanel={panels.length + pendingModelPanels < MAX_AGENT_PANELS}
        leadingRef={titlebarLeadingRef}
        tabsSlotRef={setTabsSlot}
      />

      <div
        ref={mainRowRef}
        className={sideOpen ? "main-row" : "main-row sidebar-closed"}
        style={{ "--pane-columns": cols } as CSSProperties}
      >
        <ActivityRail
          activeTab={sideTab}
          sideOpen={sideOpen}
          settingsOpen={settingsOpen}
          changesOpen={changesOpen}
          session={session}
          trayOpen={trayOpen}
          traySnapped={traySnapped}
          onSelectTab={activateSidebarTab}
          onToggleTerminal={toggleTray}
          onOpenBrowser={() => openBrowserTab()}
          onToggleChanges={() => { setSettingsOpen(false); setChangesOpen((value) => !value); }}
          onToggleSettings={() => settingsOpen ? setSettingsOpen(false) : openSettings()}
        />
        <div className={`sidebar-slot ${sideOpen ? "" : "collapsed"}`}>
          {settingsOpen ? <SettingsSidebar
            section={settingsSection}
            onSectionChange={setSettingsSection}
          /> : <FileSidebar
            collapsed={!sideOpen}
            onCollapse={setSidebarOpen}
            onDrag={sideDrag}
            onOpenTerminal={(directory) => {
              setTerminalRequest((current) => ({ id: (current?.id ?? 0) + 1, directory }));
              showTray();
            }}
            tab={sideTab}
          />}
        </div>
        <div className={`divider ${sideOpen ? "" : "collapsed"}`} onMouseDown={sideDrag} style={{ pointerEvents: sideOpen ? undefined : "none" }} />
        <div className="workspace-host" ref={workspaceHostRef}>
          <div
          ref={workspaceAreaRef}
          inert={settingsOpen}
          className={`workspace-area ${settingsOpen ? "settings-suspended" : ""} ${ordered.some((panel) => slotShown(panel) > 0) || (panels.length === 0 && emptyAgentOpen) ? "agent-open" : ""} ${editorCovered ? "editor-covered" : ""} ${trayDragging ? "tray-dragging" : ""}`}
          style={
            {
              "--editor-right": `${editorRight}px`,
              "--tray-row": trayOpen ? `${trayH}px` : "0px",
              "--tray-gap": trayOpen ? "10px" : "0px"
            } as CSSProperties
          }
        >
          <EditorPane
            tabsSlot={tabsSlot}
            onValidationComplete={(report) => {
              setValidationReport(report);
              setTerminalPanel("problems");
              showTray();
            }}
          />
          {changesOpen && <ChangesView onClose={() => setChangesOpen(false)} />}
          {ordered.map((panel, index) => {
            const s = slotFor(panel);
            const anchorId = panels[0]?.workspace.id ?? null;
            const isAnchor = panel.workspace.id === anchorId;
            const leftN = index > 0 ? ordered[index - 1] : null;
            const rightN = index < ordered.length - 1 ? ordered[index + 1] : null;
            const leftMin = Math.max(0, leftN ? slotFor(leftN).left + slotShown(leftN) : 0);
            const leftMax = rightN ? slotFor(rightN).left - slotShown(panel) : areaW - slotShown(panel);
            const rightMax = (rightN ? slotFor(rightN).left : areaW) - s.left;
            return (
              <PanelColumn
                key={panel.workspace.id}
                session={panel}
                sessionChoices={sessionChoices}
                visibleSessionIDs={new Set(panels.map((candidate) => candidate.id))}
                agentPanelLimitReached={inAgentMode && panels.length >= MAX_AGENT_PANELS}
                agentModeActive={inAgentMode}
                onSessionSelect={selectAgentSession}
                slot={s}
                isAnchor={isAnchor}
                freeMove={inAgentMode}
                leftMin={leftMin}
                leftMax={leftMax}
                rightMax={rightMax}
                isLast={index === ordered.length - 1}
                onSlot={(update) =>
                  setSlots((current) => {
                    const base = current[panel.workspace.id] ?? { open: true, width: AGENT_DEFAULT_W, left: 0, top: 0, height: 100 };
                    return { ...current, [panel.workspace.id]: typeof update === "function" ? update(base) : update };
                  })
                }
                onFocus={() => focusSession(panel.id)}
                onClose={() => closeAgentPanel(panel)}
                onManualAdjust={leaveAgentModeForManualAdjustment}
              />
            );
          })}
          {panels.length === 0 && emptyAgentOpen && (
            <div
              ref={emptyAgentRef}
              className={`agent-col empty-agent-col ${inAgentMode ? "agent-mode-empty" : ""}`}
              style={inAgentMode ? undefined : { width: `${emptyAgentShown}px`, right: "0px" }}
            >
              <AgentPanel
                onClose={() => setEmptyAgentOpen(false)}
                onResizeLeft={emptyAgentDrag}
              />
            </div>
          )}
          <div
            ref={trayAreaRef}
            className={`tray-area ${trayOpen ? "open" : ""} ${trayDragging ? "dragging" : ""}`}
            style={{ "--tray-height": `${trayH}px` } as CSSProperties}
          >
            <div className="tray-inner">
              <div className="tray-divider" onMouseDown={trayDrag} title="Drag to resize" />
              {session ? (
                <TerminalTray
                  height={trayH}
                  snapped={traySnapped}
                  request={terminalRequest}
                  validationReport={validationReport}
                  activePanel={terminalPanel}
                  onPanelChange={setTerminalPanel}
                  onClose={closeTray}
                  onExpand={expandTray}
                />
              ) : (
                <EmptyTerminalTray onClose={closeTray} />
              )}
            </div>
          </div>
          </div>
          {settingsOpen && <SettingsPage section={settingsSection} onClose={() => setSettingsOpen(false)} />}
        </div>
      </div>

      <Toasts />
      <RecoveryNotice />
    </div>
  );
}

function Toasts(): ReactNode {
  const { toasts } = useStore();
  return (
    <div className="toasts">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.tone}`}>
          {t.text}
        </div>
      ))}
    </div>
  );
}

export default function App(): ReactNode {
  useEffect(() => {
    document.documentElement.classList.toggle(
      "darwin",
      window.vela.platform === "darwin"
    );
  }, []);

  return (
    <ThemeProvider>
      <BrowserTabsProvider>
        <BrowserStoreProvider>
          <Root />
        </BrowserStoreProvider>
      </BrowserTabsProvider>
    </ThemeProvider>
  );
}

function Root(): ReactNode {
  useAppStyles();
  const { unsavedEditorFileCount, unsavedEditorFiles, saveAllEditorFiles } = useStore();

  useEffect(() => {
    if (unsavedEditorFileCount === 0) return;
    const preventClose = (event: BeforeUnloadEvent): void => {
      protectEditorUnload(event, unsavedEditorFileCount);
    };
    window.addEventListener("beforeunload", preventClose);
    return () => window.removeEventListener("beforeunload", preventClose);
  }, [unsavedEditorFileCount]);

  useEffect(() => {
    if (typeof window.vela.onSaveAllEditorFiles !== "function") return;
    return window.vela.onSaveAllEditorFiles(async () => {
      const saved = await saveAllEditorFiles();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      return saved;
    });
  }, [saveAllEditorFiles]);

  const unsavedFilePaths = unsavedEditorFiles.map((file) => file.displayPath);
  const unsavedFilePathsKey = JSON.stringify(unsavedFilePaths);
  useEffect(() => {
    const reportUnsaved = window.vela.reportUnsavedEditorFiles;
    if (typeof reportUnsaved === "function") void reportUnsaved(unsavedFilePaths).catch(() => {});
  }, [unsavedFilePathsKey]);

  useEffect(() => {
    void window.vela.windowView("session").catch(() => {});
  }, []);

  return <Layout />;
}
