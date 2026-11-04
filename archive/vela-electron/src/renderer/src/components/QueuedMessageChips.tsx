import { useCallback } from "react";
import type { PromptFile, WorkspaceIdentity } from "@shared/types";
import { usePanel, useStore } from "../store";
import { IconArrowDown, IconArrowRight, IconArrowUp, IconClose, IconEdit } from "./icons";

function firstLine(content: string): string {
  const lines = content.split("\n");
  const first = lines[0] ?? "";
  const maxLength = 100;
  if (first.length > maxLength) {
    return `${first.substring(0, maxLength)}...`;
  }
  return first + (lines.length > 1 ? "..." : "");
}

export function QueuedMessageChips({
  workspace,
  onEditMessage
}: {
  workspace: WorkspaceIdentity;
  onEditMessage: (content: string, attachments: PromptFile[]) => void;
}): React.ReactNode {
  const view = usePanel(workspace);
  const store = useStore();
  const queuedMessages = view.queuedMessages ?? [];

  const handleEdit = useCallback((messageID: string) => {
    const popped = store.popQueuedMessage(workspace, messageID);
    if (popped) onEditMessage(popped.content, popped.attachments ?? []);
  }, [store, workspace, onEditMessage]);

  if (queuedMessages.length === 0) return null;

  return (
    <div className="queued-chips">
      <div className="queued-chips-head">
        <span>Queued messages</span>
        <span className="queued-chips-count">{queuedMessages.length}</span>
      </div>
      <div className="queued-chips-list">
        {queuedMessages.map((message, index) => {
          const attachmentCount = message.attachmentCount ?? message.attachments?.length ?? 0;
          const previousLocal = index > 0 && queuedMessages[index - 1].id.startsWith("queued-") && message.id.startsWith("queued-");
          const nextLocal = index < queuedMessages.length - 1 && queuedMessages[index + 1].id.startsWith("queued-") && message.id.startsWith("queued-");
          return (
            <div key={message.id} className="queued-chip">
              <span className="queued-chip-text" title={message.content}>
                {firstLine(message.content)}
                {attachmentCount > 0 && <span className="queued-chip-attachments">{attachmentCount} attachment{attachmentCount === 1 ? "" : "s"}</span>}
              </span>
              {previousLocal && (
                <button
                  className="queued-chip-button"
                  title="Move up"
                  onClick={() => store.reorderQueuedMessage(workspace, message.id, queuedMessages[index - 1].id)}
                >
                  <IconArrowUp />
                </button>
              )}
              {nextLocal && (
                <button
                  className="queued-chip-button"
                  title="Move down"
                  onClick={() => store.reorderQueuedMessage(workspace, message.id, queuedMessages[index + 1].id)}
                >
                  <IconArrowDown />
                </button>
              )}
              <button className="queued-chip-button" title="Edit in composer" onClick={() => handleEdit(message.id)}>
                <IconEdit />
              </button>
              <button className="queued-chip-button" title="Send now" onClick={() => void store.sendQueuedNow(workspace, message.id)}>
                <IconArrowRight />
              </button>
              <button className="queued-chip-button" title="Remove" onClick={() => store.removeQueuedMessage(workspace, message.id)}>
                <IconClose />
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
