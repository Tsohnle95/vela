import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  MarkdownDelegateProvider,
  TextShimmer
} from "@deepseek-ai/dsh-client-ui-primitives";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import { useStore } from "../store";
import { buildTurns, visibleTranscript, workspaceFilePath } from "../components/timeline-model";
import { DshTurn } from "./DshTurn";
import { runningDuration } from "./process-model";
import { useDshScroll } from "./use-dsh-scroll";
import "./dsh-chat.css";
import "./dsh-tool-row.css";

export { clearDshScrollMemoryForTests } from "./use-dsh-scroll";

function RunningStatus({
  startedAt,
  waiting
}: {
  startedAt: number | undefined;
  waiting: boolean;
}) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [startedAt]);
  const label = waiting ? "Waiting for your action" : runningDuration(startedAt, now);
  return (
    <div className="dsh-running" data-chat-running="">
      <span className="dsh-visually-hidden" role="status" aria-live="polite" aria-atomic="true">
        Deep diving
      </span>
      <span className="dsh-running-divider" aria-hidden="true" />
      <span className="dsh-running-content">
        <span className="dsh-running-icon" aria-hidden="true">
          <span className="dsh-running-whale" />
          <svg className="dsh-running-whale-still" width="100%" height="100%" viewBox="0 0 16 16" fill="none">
            <path
              d="M8.844 13.742C8.967 12.328 8.45 10.4 8.45 9.65C8.45 8.94 8.88 8.43 9.6 8.43C11.285 8.43 12.106 8.281 12.685 8.104C13.71 7.791 14.585 6.768 15.055 5.945C15.137 5.803 14.99 5.641 14.829 5.671C13.829 5.86 12.828 5.376 11.827 4.978C10.659 4.514 9.491 4.707 8.935 4.876C8.805 4.915 8.658 4.819 8.636 4.686C8.468 3.643 7.405 2.615 5.498 2.238C4.54 2.048 3.748 1.574 3.347 1.202C3.252 1.113 3.088 1.125 3.03 1.242C2.628 2.059 2.168 3.82 5.248 6.115C5.82 6.494 6.31 6.785 6.574 7.637C6.72 8.104 6.157 9.168 6.061 9.368C5.157 11.27 5.089 12.19 4.926 13.742"
              stroke="currentColor"
              strokeWidth="1"
            />
          </svg>
        </span>
        <TextShimmer active className="dsh-running-text">{label}</TextShimmer>
      </span>
    </div>
  );
}

/**
 * The DeepSeek Harness transcript surface. It owns its scrollport so content
 * growth follows the floor only while the reader is at the floor, and turns
 * over to native anchoring the moment they move away.
 */
export function DshTranscript({
  transcript,
  busy,
  lastAssistantId,
  session,
  activity,
  startedAt,
  empty,
  showEmpty
}: {
  transcript: TranscriptItem[];
  busy: boolean;
  lastAssistantId: string | null;
  session: SessionInfo | null;
  activity?: { isWaitingForPermission?: boolean; statusText?: string | null; isWorking?: boolean } | null;
  startedAt?: number | null;
  empty?: ReactNode;
  showEmpty?: boolean;
}) {
  const { openFile, sessions } = useStore();
  const visible = useMemo(
    () => visibleTranscript(transcript, sessions, session?.id),
    [transcript, sessions, session?.id]
  );
  const turns = useMemo(() => buildTurns(visible), [visible]);
  const lastTurn = turns.at(-1);
  const runningStart = startedAt ?? lastTurn?.user?.createdAt;
  const { scrollRef, contentRef, following, onScroll, returnToBottom } = useDshScroll(
    session?.workspace.id,
    transcript,
    busy,
    turns.length
  );

  const delegate = useMemo(() => ({
    openExternalLink: (href: string): void => {
      window.open(href, "_blank", "noopener,noreferrer");
    },
    openFile: (path: string): void => {
      const target = workspaceFilePath(path, session);
      if (!target) return;
      void openFile(target, session?.workspace);
    }
  }), [openFile, session]);

  return (
    <div className="dsh-chat" data-dsh-chat="">
      <div
        ref={scrollRef}
        className="dsh-scroll"
        data-dsh-following={following ? "" : undefined}
        onScroll={onScroll}
      >
        <div ref={contentRef} className="dsh-column">
          {showEmpty && turns.length === 0 && empty}
          <MarkdownDelegateProvider {...delegate}>
            {turns.map((turn, index) => (
              <DshTurn
                key={turn.id}
                turn={turn}
                busy={busy}
                lastAssistantId={lastAssistantId}
                session={session}
                isLast={index === turns.length - 1}
              />
            ))}
            {busy && (
              <RunningStatus
                startedAt={runningStart ?? undefined}
                waiting={activity?.isWaitingForPermission === true}
              />
            )}
          </MarkdownDelegateProvider>
        </div>
      </div>
      {!following && (
        <div className="dsh-to-bottom-slot">
          <button type="button" className="dsh-to-bottom" aria-label="Back to bottom" onClick={returnToBottom}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path d="M4 6l4 4 4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
        </div>
      )}
    </div>
  );
}
