import type { ReactNode, Ref } from "react";
import { IconAgentMode, IconPanelLeft, IconPanelLeftDashed, IconPanelRight, IconPanelRightDashed, IconSquarePlus } from "./icons";

export function Titlebar({
  sidebarOpen,
  agentMode,
  agentPanelOpen,
  onToggleSidebar,
  onToggleAgentMode,
  onToggleAgentPanel,
  onAddAgentPanel,
  canAddAgentPanel,
  leadingRef,
  tabsSlotRef
}: {
  sidebarOpen: boolean;
  agentMode: boolean;
  agentPanelOpen: boolean;
  onToggleSidebar: () => void;
  onToggleAgentMode: () => void;
  onToggleAgentPanel: () => void;
  onAddAgentPanel: () => void;
  canAddAgentPanel: boolean;
  leadingRef?: Ref<HTMLSpanElement>;
  tabsSlotRef?: Ref<HTMLDivElement>;
}): ReactNode {
  return (
    <div className="titlebar">
      <span className="titlebar-leading-actions" ref={leadingRef}>
        <button
          className="icon-btn"
          data-panel-action="toggle-sidebar"
          title={sidebarOpen ? "Hide left panel" : "Show left panel"}
          aria-label={sidebarOpen ? "Hide left panel" : "Show left panel"}
          aria-pressed={sidebarOpen}
          onClick={onToggleSidebar}
        >
          {sidebarOpen ? <IconPanelLeft /> : <IconPanelLeftDashed />}
        </button>
      </span>
      <div className="titlebar-tabs-slot" ref={tabsSlotRef} />
      <span className="titlebar-actions">
        <button
          className={`icon-btn ${agentMode ? "on" : ""}`}
          data-panel-action="toggle-model-mode"
          aria-label={agentMode ? "Exit Agent Mode" : "Enter Agent Mode"}
          aria-pressed={agentMode}
          title={agentMode
            ? "Exit Agent Mode — restore the previous panel layout"
            : "Agent Mode — collapse the sidebar and split models across the workspace"}
          onClick={(event) => {
            event.preventDefault();
            onToggleAgentMode();
          }}
        >
          <IconAgentMode />
        </button>
        <button
          className="icon-btn"
          data-panel-action="add-model-panel"
          title={canAddAgentPanel ? "Add agent panel (choose a folder)" : "Agent panel limit reached"}
          aria-label="Add model panel"
          disabled={!canAddAgentPanel}
          onClick={onAddAgentPanel}
        >
          <IconSquarePlus />
        </button>
        <button
          className="icon-btn"
          data-panel-action="toggle-agent-panel"
          title={agentPanelOpen ? "Hide agent panel" : "Show agent panel"}
          aria-label={agentPanelOpen ? "Hide agent panel" : "Show agent panel"}
          aria-pressed={agentPanelOpen}
          onClick={onToggleAgentPanel}
        >
          {agentPanelOpen ? <IconPanelRight /> : <IconPanelRightDashed />}
        </button>
      </span>
    </div>
  );
}
