import type { SessionInfo } from "@shared/types";

export const MAX_AGENT_PANELS = 8;

export function agentPanelGrid(panelCount: number): { columns: number; rows: number } {
  if (panelCount < 3) return { columns: Math.max(1, panelCount), rows: 1 };
  return {
    columns: Math.ceil(panelCount / 2),
    rows: 2
  };
}

/** Each row divides the full width, including an incomplete final row. */
export function agentPanelGridSlot(panelCount: number, index: number, areaW: number): Pick<PanelSlot, "left" | "width" | "top" | "height"> {
  const { columns, rows } = agentPanelGrid(panelCount);
  const row = Math.floor(index / columns);
  const column = index % columns;
  const rowColumns = Math.min(columns, panelCount - row * columns);
  const left = Math.floor(areaW * column / rowColumns);
  const right = Math.floor(areaW * (column + 1) / rowColumns);
  return { left, width: right - left, top: row * (100 / rows), height: 100 / rows };
}

export const AGENT_DEFAULT_W = 325;
export const AGENT_MIN_W = 280;
export const EDITOR_MIN_VISIBLE_W = 180;

export interface PanelSlot {
  open: boolean;
  width: number;
  left: number;
  leftAnchored?: boolean;
  top: number;
  height: number;
}

export function panelSlotFor(
  panel: SessionInfo,
  panels: SessionInfo[],
  slots: Record<string, PanelSlot>,
  remembered: PanelSlot | undefined,
  areaW: number,
  inAgentMode: boolean
): PanelSlot {
  const anchorId = panels[0]?.workspace.id ?? null;
  const stored = slots[panel.workspace.id] ?? remembered;
  if (inAgentMode && panels.length === 1) {
    return {
      open: true,
      width: Math.max(0, areaW),
      left: 0,
      top: 0,
      height: 100
    };
  }
  if (panel.workspace.id === anchorId) {
    const open = stored?.open ?? true;
    const requestedWidth = stored ? (open ? stored.width : 0) : AGENT_DEFAULT_W;
    const width = !inAgentMode && open && requestedWidth <= AGENT_DEFAULT_W && !stored?.leftAnchored
      ? Math.min(requestedWidth, Math.max(0, areaW - EDITOR_MIN_VISIBLE_W - 10))
      : requestedWidth;
    if (inAgentMode && stored) {
      return {
        open: true,
        width: stored.width,
        left: stored.left,
        top: stored.top,
        height: stored.height
      };
    }
    if (open && stored?.left === 0 && stored.leftAnchored) {
      return { open: true, width: areaW, left: 0, top: 0, height: 100 };
    }
    return { open, width, left: Math.max(0, areaW - width), top: 0, height: 100 };
  }
  if (stored) {
    return inAgentMode && !stored.open
      ? { ...stored, open: true, width: Math.max(AGENT_DEFAULT_W, stored.width) }
      : stored;
  }
  const anchorStored = anchorId ? slots[anchorId] : undefined;
  const anchorWidth = anchorStored ? (anchorStored.open ? anchorStored.width : 0) : AGENT_DEFAULT_W;
  const anchorLeft = Math.max(0, areaW - anchorWidth);
  let left = anchorLeft - AGENT_DEFAULT_W;
  for (const other of panels) {
    if (other.workspace.id === panel.workspace.id || other.workspace.id === anchorId) continue;
    const s = slots[other.workspace.id];
    if (!s) continue;
    left = Math.min(left, s.left - AGENT_DEFAULT_W);
  }
  return { open: true, width: AGENT_DEFAULT_W, left: Math.max(0, left), top: 0, height: 100 };
}

export function distributePanelSlots(
  current: Record<string, PanelSlot>,
  targetPanels: SessionInfo[],
  areaW: number,
  singleRestore: boolean,
  modelMode: boolean
): Record<string, PanelSlot> {
  const anchorId = targetPanels[0]?.workspace.id ?? null;
  const openIDs = modelMode
    ? targetPanels.map((panel) => panel.workspace.id)
    : targetPanels
      .filter((panel) => panel.workspace.id === anchorId || (current[panel.workspace.id]?.open ?? true))
      .map((panel) => panel.workspace.id);
  if (openIDs.length === 0) return current;
  if (singleRestore && openIDs.length === 1 && anchorId) {
    return { ...current, [anchorId]: { open: true, width: AGENT_DEFAULT_W, left: 0, top: 0, height: 100 } };
  }
  const total = areaW;
  const grid = modelMode && openIDs.length >= 3;
  if (grid) {
    const next = { ...current };
    [...openIDs].reverse().forEach((id, index) => {
      next[id] = { open: true, leftAnchored: false, ...agentPanelGridSlot(openIDs.length, index, total) };
    });
    return next;
  }
  const { columns } = agentPanelGrid(openIDs.length);
  const width = Math.max(AGENT_MIN_W, Math.floor(total / columns));
  const anchorW = Math.max(AGENT_MIN_W, total - width * (columns - 1));
  const next: Record<string, PanelSlot> = {};
  let boundary = Math.max(0, areaW - anchorW);
  for (const panel of [...targetPanels].reverse()) {
    const id = panel.workspace.id;
    if (id === anchorId || !openIDs.includes(id)) continue;
    boundary -= width;
    next[id] = {
      open: true,
      width,
      left: Math.max(0, boundary),
      leftAnchored: false,
      top: 0,
      height: 100
    };
  }
  if (anchorId) {
    next[anchorId] = {
      open: true,
      width: anchorW,
      left: Math.max(0, areaW - anchorW),
      leftAnchored: false,
      top: 0,
      height: 100
    };
  }
  return { ...current, ...next };
}
