import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { usePanel, useStore } from "../store";
import { OpenCodeTimeline, PermissionPrompt } from "./OpenCodeTimeline";
import { FormPrompt } from "./FormPrompt";
import { OpenCodeTodoDock } from "./OpenCodeTodoDock";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import { IconArrowLeft, IconMoreHorizontal } from "./icons";
import { AgentTui } from "./AgentTui";
import { MAX_AGENT_PANELS } from "../agent-panels";
import { Composer } from "./Composer";
import { TurnTimer, useAgentUsage } from "./AgentUsage";
import stationMark from "../assets/agent-station.png";
import lightStationMark from "../assets/agent-station-light.png";
import { useAgentScroll } from "./agent-scroll";
import { useMenuDismiss } from "./use-menu-dismiss";

type PanelMode = "gui" | "tui";


function panelModeStorageKey(sessionID: string): string {
  return `mira.agent-panel-mode.${sessionID}`;
}

function legacyPanelModeStorageKey(sessionID: string): string {
  return `orbit.agent-panel-mode.${sessionID}`;
}

function readPanelMode(sessionID: string | undefined): PanelMode {
  if (!sessionID) return "gui";
  try {
    const mode = window.localStorage.getItem(panelModeStorageKey(sessionID)) ?? window.localStorage.getItem(legacyPanelModeStorageKey(sessionID));
    return mode === "tui" ? "tui" : "gui";
  } catch {
    return "gui";
  }
}

function savePanelMode(sessionID: string | undefined, mode: PanelMode): void {
  if (!sessionID) return;
  try {
    window.localStorage.setItem(panelModeStorageKey(sessionID), mode);
  } catch {
    return;
  }
}

export function AgentPanel({
  session,
  sessionChoices,
  visibleSessionIDs,
  agentPanelLimitReached,
  agentModeActive,
  onSessionSelect,
  isAnchor: _isAnchor,
  onFocus,
  onClose,
  onResizeLeft,
  onResizeRight,
  onPanelDrag,
  onPanelDragEnd
}: {
  session?: SessionInfo | null;
  sessionChoices?: SessionInfo[];
  visibleSessionIDs?: ReadonlySet<string>;
  agentPanelLimitReached?: boolean;
  agentModeActive?: boolean;
  onSessionSelect?: (sessionID: string) => void | Promise<void>;
  isAnchor?: boolean;
  onFocus?: () => void;
  onClose?: () => void;
  onResizeLeft?: (e: React.MouseEvent) => void;
  onResizeRight?: (e: React.MouseEvent) => void;
  onPanelDrag?: (delta: number) => void;
  onPanelDragEnd?: () => void;
}): ReactNode {
  const {
    session: storeSession,
    sessions,
    reopenSession,
    providerUsage,
    providerUsageLoading,
    refreshProviderUsage,
    commitStagedRevert,
    clearStagedRevert,
    runtimes
  } = useStore();
  const activeSession = session === undefined ? storeSession : session;
  const view = usePanel(activeSession?.workspace);
  const { busy, todos, transcript, sessionUsage, compactionBaseline, currentModel, turnStartedAt, assistantStatus } = view;
  const { scrollRef, onScroll } = useAgentScroll(activeSession, transcript, busy, view.queuedMessages);
  const panelRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLDivElement>(null);
  const panelDragRef = useRef<number | null>(null);
  const [panelMode, setPanelMode] = useState<PanelMode>(() => readPanelMode(activeSession?.id));
  const [modeMenuOpen, setModeMenuOpen] = useState(false);
  const [tuiNotice, setTuiNotice] = useState("");
  const runtime = runtimes?.find((item) => item.id === (activeSession?.runtimeID ?? "opencode"));
  const tuiAvailable = Boolean(activeSession) && (runtime?.capabilities.tui ?? (activeSession?.runtimeID ?? "opencode") === "opencode");
  const startPanelDrag = (event: React.MouseEvent<HTMLDivElement>): void => {
    if (!onPanelDrag || (event.target as HTMLElement).closest("button")) return;
    event.preventDefault();
    const startX = event.clientX;
    let hasDragged = false;
    panelDragRef.current = startX;
    const move = (moveEvent: MouseEvent): void => {
      if (panelDragRef.current === null) return;
      if (!hasDragged) {
        if (Math.abs(moveEvent.clientX - startX) < 4) return;
        hasDragged = true;
      }
      const delta = moveEvent.clientX - panelDragRef.current;
      panelDragRef.current = moveEvent.clientX;
      onPanelDrag(delta);
    };
    const stop = (): void => {
      panelDragRef.current = null;
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", stop);
      if (hasDragged) onPanelDragEnd?.();
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", stop);
  };
  const parent = activeSession?.parentID ? sessions.find((item) => item.id === activeSession.parentID) : undefined;

  useEffect(() => {
    const savedMode = readPanelMode(activeSession?.id);
    setPanelMode(savedMode === "tui" && tuiAvailable ? "tui" : "gui");
    setModeMenuOpen(false);
    setTuiNotice("");
  }, [activeSession?.id, activeSession?.workspace.id, activeSession?.workspace.generation, tuiAvailable]);

  const { usageButton, usagePopup } = useAgentUsage({ sessionUsage, compactionBaseline, currentModel,
    providerUsage, providerUsageLoading, refreshProviderUsage, panelRef });

  useMenuDismiss(modeMenuOpen, panelRef, useCallback(() => setModeMenuOpen(false), []));

  const choosePanelMode = (next: "gui" | "tui"): void => {
    setModeMenuOpen(false);
    if (next === "tui") {
      if (!tuiAvailable) {
        setTuiNotice("TUI is unavailable for this runtime.");
        return;
      }
      setTuiNotice("");
      setPanelMode("tui");
      savePanelMode(activeSession?.id, "tui");
      return;
    }
    setTuiNotice("");
    setPanelMode("gui");
    savePanelMode(activeSession?.id, "gui");
  };

  const chooseSession = (sessionID: string): void => {
    void onSessionSelect?.(sessionID);
    setModeMenuOpen(false);
  };

  const handleTuiExit = (exitCode: number | null): void => {
    setPanelMode("gui");
    savePanelMode(activeSession?.id, "gui");
    setTuiNotice(exitCode === 0 ? "Agent TUI exited." : `Agent TUI exited${exitCode === null ? "" : ` with code ${exitCode}`}.`);
  };

  const handleTuiError = (message: string): void => {
    setPanelMode("gui");
    savePanelMode(activeSession?.id, "gui");
    setTuiNotice(message);
  };

  const lastAssistantId = useMemo(() => {
    for (let i = transcript.length - 1; i >= 0; i -= 1) {
      const item = transcript[i];
      if (item.kind === "assistant") return item.id;
    }
    return null;
  }, [transcript]);

  const conversationStarted = useMemo(
    () => transcript.some((item) => item.kind === "user" || item.kind === "assistant"),
    [transcript]
  );

  const pendingPermission = useMemo(
    () => [...transcript]
      .reverse()
      .find((item): item is Extract<TranscriptItem, { kind: "permission" }> => item.kind === "permission" && item.pending),
    [transcript]
  );

  // Approvals and forms are interactive: the surface the user is watching owns
  // them. The embedded TUI renders its own prompts in the terminal, so drawing
  // the panel dock there would ask the same question twice.
  const interactivePrompts = (
    <>
      {pendingPermission && <PermissionPrompt item={pendingPermission} session={activeSession} />}
      {activeSession && (view.pendingForms ?? []).map((form) => (
        <FormPrompt key={form.id} form={form} workspace={activeSession.workspace} />
      ))}
    </>
  );

  return (
    <div
      className="agent-panel"
      data-conversation={conversationStarted}
      data-panel-mode={panelMode}
      ref={panelRef}
      onMouseDownCapture={onFocus}
    >
      <div className="observatory-scene" aria-hidden="true" />
      {onResizeLeft && <div className="panel-resize-handle panel-resize-left" onMouseDown={onResizeLeft} />}
      {onResizeRight && <div className="panel-resize-handle panel-resize-right" onMouseDown={onResizeRight} />}
      <div className="agent-header" ref={headerRef} onMouseDown={startPanelDrag}>
        {activeSession?.parentID && (
          <button
            className="icon-btn agent-session-back"
            title={`Back to ${parent?.title ?? "parent session"}`}
            aria-label={`Back to ${parent?.title ?? "parent session"}`}
            onClick={() => void reopenSession(activeSession.parentID!)}
          >
            <IconArrowLeft />
          </button>
        )}
        <TurnTimer startedAt={turnStartedAt} />
        <AgentPanelOptionsMenu
          open={modeMenuOpen}
          panelMode={panelMode}
          tuiAvailable={tuiAvailable}
          sessionChoices={sessionChoices}
          visibleSessionIDs={visibleSessionIDs}
          agentModeActive={agentModeActive}
          agentPanelLimitReached={agentPanelLimitReached}
          onToggle={() => setModeMenuOpen((value) => !value)}
          onDismiss={() => setModeMenuOpen(false)}
          onChoosePanelMode={choosePanelMode}
          onChooseSession={chooseSession}
          onClose={onClose}
        />
      </div>

      {panelMode === "tui" && activeSession ? (
        <AgentTui workspace={activeSession.workspace} onExit={handleTuiExit} onError={handleTuiError} />
      ) : (
        <>
          <div className="agent-scroll" ref={scrollRef} onScroll={onScroll}>
            {tuiNotice && <div className="agent-tui-notice">{tuiNotice}</div>}
            {!conversationStarted && (
              <div className="agent-empty">
                <img
                  className="agent-empty-art agent-empty-art-dark"
                  src={stationMark}
                  alt=""
                  aria-hidden="true"
                  draggable={false}
                  decoding="async"
                />
                <img
                  className="agent-empty-art agent-empty-art-light"
                  src={lightStationMark}
                  alt=""
                  aria-hidden="true"
                  draggable={false}
                  decoding="async"
                />
                <p>Tell the agent what to work on.</p>
              </div>
            )}
            <OpenCodeTimeline
              transcript={transcript}
              busy={busy}
              lastAssistantId={lastAssistantId}
              session={activeSession}
            />
          </div>

          <div data-component="session-prompt-dock">
            {interactivePrompts}
            {view.stagedRevert && activeSession && (
              <div data-component="dock-prompt" data-kind="revert">
                <div data-slot="permission-header">Undo staged</div>
                <p className="provider-note">Messages and file changes after this point will be removed.</p>
                <div data-slot="permission-actions">
                  <button className="btn btn-danger" onClick={() => void commitStagedRevert(activeSession.workspace)}>Commit undo</button>
                  <button className="btn" onClick={() => void clearStagedRevert(activeSession.workspace)}>Discard</button>
                </div>
              </div>
            )}
            <OpenCodeTodoDock todos={todos} />
            <Composer session={activeSession} controls={usageButton} overlay={usagePopup} />
          </div>
        </>
      )}
    </div>
  );
}

// The panel's options surface: mode switches, the sessions carried by Agent
// Mode, and panel close. Extracted from the panel body so the header identity
// and the menu stay independently readable.
function AgentPanelOptionsMenu({
  open,
  panelMode,
  tuiAvailable,
  sessionChoices,
  visibleSessionIDs,
  agentModeActive,
  agentPanelLimitReached,
  onToggle,
  onDismiss,
  onChoosePanelMode,
  onChooseSession,
  onClose
}: {
  open: boolean;
  panelMode: PanelMode;
  tuiAvailable: boolean;
  sessionChoices?: SessionInfo[];
  visibleSessionIDs?: ReadonlySet<string>;
  agentModeActive?: boolean;
  agentPanelLimitReached?: boolean;
  onToggle: () => void;
  onDismiss: () => void;
  onChoosePanelMode: (next: PanelMode) => void;
  onChooseSession: (sessionID: string) => void;
  onClose?: () => void;
}): ReactNode {
  return (
    <div className="agent-header-actions">
      {open && (
        <div className="agent-mode-menu" role="menu" aria-label="Agent panel options">
          <button
            role="menuitemradio"
            aria-checked={panelMode === "gui"}
            className={panelMode === "gui" ? "selected" : ""}
            onClick={() => onChoosePanelMode("gui")}
          >
            <span>GUI</span>
            <small>Mira agent panel</small>
          </button>
          <button
            role="menuitemradio"
            aria-checked={panelMode === "tui"}
            className={`${panelMode === "tui" ? "selected" : ""} ${!tuiAvailable ? "disabled" : ""}`}
            title={tuiAvailable ? "Open the runtime TUI in this panel" : "TUI is unavailable for this runtime"}
            onClick={() => onChoosePanelMode("tui")}
          >
            <span>TUI</span>
            <small>{tuiAvailable ? "Terminal interface" : "Unavailable"}</small>
          </button>
          {sessionChoices && sessionChoices.length > 0 && (
            <>
              <div className="agent-mode-menu-divider" />
              <div className="agent-mode-menu-heading">Active sessions</div>
              <AgentPanelSessionRows
                choices={sessionChoices}
                visibleSessionIDs={visibleSessionIDs}
                agentModeActive={agentModeActive}
                agentPanelLimitReached={agentPanelLimitReached}
                onChooseSession={onChooseSession}
              />
            </>
          )}
          {onClose && (
            <>
              <div className="agent-mode-menu-divider" />
              <button
                role="menuitem"
                onClick={() => {
                  onDismiss();
                  onClose();
                }}
              >
                <span>Close panel</span>
                <small>{agentModeActive ? "Remove this model panel" : "Hide the agent panel"}</small>
              </button>
            </>
          )}
        </div>
      )}
      <button
        className={`icon-btn agent-mode-menu-toggle ${open ? "open" : ""}`}
        title="Agent panel options"
        aria-label="Agent panel options"
        aria-expanded={open}
        onClick={onToggle}
      >
        <IconMoreHorizontal />
      </button>
    </div>
  );
}

// The sessions the options menu can switch to. Rows stay individually
// addressed because Agent Mode tracks which of them already has a panel.
function AgentPanelSessionRows({
  choices,
  visibleSessionIDs,
  agentModeActive,
  agentPanelLimitReached,
  onChooseSession
}: {
  choices: SessionInfo[];
  visibleSessionIDs?: ReadonlySet<string>;
  agentModeActive?: boolean;
  agentPanelLimitReached?: boolean;
  onChooseSession: (sessionID: string) => void;
}): ReactNode {
  const sessionLabel = (choice: SessionInfo): string =>
    choice.title ?? choice.directory.split(/[\\/]/).filter(Boolean).pop() ?? choice.agent ?? choice.id;
  return (
    <>
      {choices.map((choice) => (
        (() => {
          const visible = visibleSessionIDs?.has(choice.id) ?? false;
          const disabled = Boolean(agentModeActive && agentPanelLimitReached && !visible);
          return (
            <button
              key={choice.id}
              role="menuitemcheckbox"
              aria-checked={visible}
              className={`${visible ? "selected" : ""} ${disabled ? "disabled" : ""}`}
              title={disabled ? `Agent panel limit reached (${MAX_AGENT_PANELS})` : choice.directory}
              disabled={disabled}
              onClick={() => onChooseSession(choice.id)}
            >
              <span>{sessionLabel(choice)}</span>
              <small>{disabled ? "Agent panel limit reached" : visible ? (agentModeActive ? "In Agent Mode" : "Current coding workspace") : agentModeActive ? "Add to Agent Mode" : "Switch workspace"}</small>
            </button>
          );
        })()
      ))}
    </>
  );
}
