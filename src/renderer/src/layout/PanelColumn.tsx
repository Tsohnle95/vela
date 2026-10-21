import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import type { SessionInfo } from "@shared/types";
import { AgentPanel } from "../components/AgentPanel";
import { AGENT_MIN_W, type PanelSlot } from "../agent-panels";
import { useDragResize } from "./panel-interactions";

export function PanelColumn({
  session,
  sessionChoices,
  visibleSessionIDs,
  agentPanelLimitReached,
  agentModeActive,
  onSessionSelect,
  slot,
  isAnchor,
  freeMove,
  leftMin,
  leftMax,
  rightMax,
  isLast,
  onSlot,
  onFocus,
  onClose,
  onManualAdjust
}: {
  session: SessionInfo;
  sessionChoices?: SessionInfo[];
  visibleSessionIDs?: ReadonlySet<string>;
  agentPanelLimitReached?: boolean;
  agentModeActive?: boolean;
  onSessionSelect?: (sessionID: string) => void | Promise<void>;
  slot: PanelSlot;
  isAnchor: boolean;
  freeMove: boolean;
  leftMin: number;
  leftMax: number;
  rightMax: number;
  isLast: boolean;
  onSlot: React.Dispatch<React.SetStateAction<PanelSlot>>;
  onFocus: () => void;
  onClose: () => void;
  onManualAdjust?: () => void;
}): ReactNode {
  const [settling, setSettling] = useState(false);
  const columnRef = useRef<HTMLDivElement>(null);
  const slideLeftRef = useRef<number | null>(null);
  const settleTimerRef = useRef<number | null>(null);
  useEffect(() => () => {
    if (settleTimerRef.current !== null) window.clearTimeout(settleTimerRef.current);
  }, []);
  const settle = useCallback(() => {
    setSettling(true);
    if (settleTimerRef.current !== null) window.clearTimeout(settleTimerRef.current);
    settleTimerRef.current = window.setTimeout(() => {
      settleTimerRef.current = null;
      setSettling(false);
    }, 200);
  }, []);
  const open = useCallback(() => onSlot((current) => ({ ...current, open: true })), [onSlot]);
  const resizeRight = useDragResize(
    slot.width,
    (width) => onSlot((current) => ({ ...current, width: typeof width === "function" ? width(current.width) : width })),
    AGENT_MIN_W,
    rightMax,
    false,
    slot.open,
    slot.left,
    undefined,
    settle,
    (width) => {
      if (columnRef.current) columnRef.current.style.width = `${width}px`;
    }
  );
  const resizeLeft = useDragResize(
    slot.width,
    (width) => onSlot((current) => ({ ...current, width: typeof width === "function" ? width(current.width) : width })),
    AGENT_MIN_W,
    slot.left + slot.width - leftMin,
    true,
    slot.open,
    slot.left,
    (left) => onSlot((current) => ({ ...current, left, leftAnchored: true })),
    settle,
    (width, left) => {
      if (!columnRef.current) return;
      columnRef.current.style.width = `${width}px`;
      if (left !== null) {
        columnRef.current.style.left = `${left}px`;
      }
      if (isAnchor) columnRef.current.parentElement?.style.setProperty("--editor-right", `${width}px`);
    }
  );
  const slideBy = (delta: number): void => {
    const next = Math.min(leftMax, Math.max(leftMin, (slideLeftRef.current ?? slot.left) + delta));
    slideLeftRef.current = next;
    if (columnRef.current) columnRef.current.style.left = `${next}px`;
  };
  const finishSlide = (): void => {
    const left = slideLeftRef.current;
    slideLeftRef.current = null;
    if (left !== null) onSlot((current) => ({ ...current, left }));
    onManualAdjust?.();
  };
  const exitModeOnRelease = (event: React.MouseEvent): void => {
    event.preventDefault();
    const release = (): void => {
      window.removeEventListener("mouseup", release);
      onManualAdjust?.();
    };
    window.addEventListener("mouseup", release);
  };
  if (!slot.open) {
    return null;
  }
  return (
    <div ref={columnRef} data-grid-row={agentModeActive && slot.top > 0 ? "lower" : undefined} className={`agent-col ${settling ? "settling" : ""}`} style={{ left: `${slot.left}px`, top: `${slot.top}%`, bottom: "auto", width: `${slot.width}px`, height: `${slot.height}%` }}>
      <AgentPanel compactComposer={agentModeActive && (visibleSessionIDs?.size ?? 0) >= 3} session={session} sessionChoices={sessionChoices} visibleSessionIDs={visibleSessionIDs} agentPanelLimitReached={agentPanelLimitReached} agentModeActive={agentModeActive} onSessionSelect={onSessionSelect} isAnchor={isAnchor} onFocus={onFocus} onClose={onClose} onResizeLeft={freeMove ? exitModeOnRelease : resizeLeft} onResizeRight={freeMove ? exitModeOnRelease : isAnchor ? undefined : resizeRight} onPanelDrag={freeMove || !isAnchor ? slideBy : undefined} onPanelDragEnd={freeMove || !isAnchor ? finishSlide : undefined} />
    </div>
  );
}

