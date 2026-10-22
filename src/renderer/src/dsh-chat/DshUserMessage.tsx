import { useState } from "react";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import { useStore } from "../store";
import { OpenCodeIcon } from "../components/OpenCodeIcon";
import { writeClipboard } from "../components/TimelineProse";

/** DSH's right-aligned user bubble. It has no entrance motion. */
export function DshUserMessage({
  item,
  session,
  busy
}: {
  item: Extract<TranscriptItem, { kind: "user" }>;
  session: SessionInfo | null;
  busy: boolean;
}) {
  const { stageRevert } = useStore();
  const [copied, setCopied] = useState(false);
  const copy = (): void => {
    if (!item.text) return;
    void writeClipboard(item.text).then((ok) => {
      if (!ok) return;
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };
  const stamp = item.createdAt === undefined
    ? ""
    : new Intl.DateTimeFormat(undefined, { timeStyle: "short" }).format(item.createdAt);

  return (
    <div className="dsh-user-row">
      <div className="dsh-user-stack">
        {item.text && <div className="dsh-user-bubble">{item.text}</div>}
        {item.attachments && item.attachments.length > 0 && (
          <div className="dsh-user-attachments">
            {item.attachments.map((attachment) => (
              <span className="dsh-user-attachment" key={attachment.name}>{attachment.name}</span>
            ))}
          </div>
        )}
        <div className="dsh-user-actions">
          {stamp && <span className="dsh-user-time">{stamp}</span>}
          {item.text && (
            <button type="button" className="dsh-icon-action" title={copied ? "Copied" : "Copy message"} onClick={copy}>
              <OpenCodeIcon name={copied ? "check" : "outline-copy"} />
            </button>
          )}
          {session && (
            <button
              type="button"
              className="dsh-icon-action"
              title="Revert message"
              aria-label="Revert message"
              disabled={busy}
              onClick={() => void stageRevert(session.workspace, item.id)}
            >
              <OpenCodeIcon name="reset" />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
