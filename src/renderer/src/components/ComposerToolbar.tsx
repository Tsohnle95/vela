import type { ReactNode, RefObject } from "react";
import { IconAdd, IconArrowUp, IconChevronDown, IconGitBranch, IconStop } from "./icons";

type ComposerToolbarProps = {
  supportsAttachments: boolean;
  attachmentInputRef: RefObject<HTMLInputElement | null>;
  addAttachmentPaths: (paths: string[]) => void;
  agentName?: string;
  agentMenuOpen: boolean;
  toggleAgentMenu: () => void;
  modelName: string;
  modelMenuOpen: boolean;
  toggleModelMenu: () => void;
  responseStrength: string;
  showResponseStrength: boolean;
  strengthMenuOpen: boolean;
  toggleStrengthMenu: () => void;
  controls?: ReactNode;
  hideSend?: boolean;
  busy: boolean;
  statusText?: string | null;
  canSend: boolean;
  onSend: () => void;
  onStop: () => void;
};

export function ComposerToolbar({
  supportsAttachments,
  attachmentInputRef,
  addAttachmentPaths,
  agentName,
  agentMenuOpen,
  toggleAgentMenu,
  modelName,
  modelMenuOpen,
  toggleModelMenu,
  responseStrength,
  showResponseStrength,
  strengthMenuOpen,
  toggleStrengthMenu,
  controls,
  hideSend,
  busy,
  statusText,
  canSend,
  onSend,
  onStop
}: ComposerToolbarProps): ReactNode {
  const sendTitle = busy ? statusText ?? "Stop the agent" : canSend ? "Send (Enter)" : "Type a prompt first";

  return (
    <div className="composer-toolbar">
      <div className="composer-chips composer-toolbar-start">
        {supportsAttachments && (
          <>
            <input
              ref={attachmentInputRef}
              className="composer-file-input"
              type="file"
              multiple
              hidden
              onChange={(event) => {
                addAttachmentPaths([...event.currentTarget.files ?? []]
                  .map((file) => window.vela.getPathForFile(file))
                  .filter(Boolean));
                event.currentTarget.value = "";
              }}
            />
            <button className="composer-attach" title="Attach files" aria-label="Attach files" onClick={() => attachmentInputRef.current?.click()}>
              <IconAdd />
            </button>
          </>
        )}
        <button
          className={`composer-selector agent ${agentMenuOpen ? "open" : ""}`}
          title="Change agent"
          aria-label={`Change agent${agentName ? `: ${agentName}` : ""}`}
          onClick={toggleAgentMenu}
        >
          <IconGitBranch />
          <span>{agentName ?? "Agent"}</span>
          <IconChevronDown />
        </button>
      </div>
      <div className="composer-actions composer-toolbar-end">
        <button
          className={`composer-selector model ${modelMenuOpen ? "open" : ""}`}
          title="Change model and response strength"
          aria-label={`Change model: ${modelName}`}
          onClick={toggleModelMenu}
        >
          <span>{modelName}</span>
          <IconChevronDown />
        </button>
        {showResponseStrength && (
          <button
            className={`composer-selector strength ${strengthMenuOpen ? "open" : ""}`}
            title="Change response strength"
            aria-label={`Change response strength: ${responseStrength}`}
            onClick={toggleStrengthMenu}
          >
            <span>{responseStrength}</span>
            <IconChevronDown />
          </button>
        )}
        {controls}
        {!hideSend && <button
          className={`composer-send ${busy ? "stop" : ""}`}
          title={sendTitle}
          aria-label={busy ? "Stop the agent" : "Send prompt"}
          disabled={!busy && !canSend}
          onClick={busy ? onStop : onSend}
        >
          {busy ? <IconStop /> : <IconArrowUp />}
        </button>}
      </div>
    </div>
  );
}

