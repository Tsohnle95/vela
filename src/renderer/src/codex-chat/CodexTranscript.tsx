import { useMemo, type ReactNode } from "react";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import { useStore } from "../store";
import { buildTurns, visibleTranscript } from "../components/timeline-model";
import { CodexTurn } from "./CodexTurn";
import { useCodexScroll } from "./use-codex-scroll";
import "./codex-chat.css";

export { clearCodexScrollMemoryForTests } from "./use-codex-scroll";

function Working({ waiting }: { waiting: boolean }) {
  return (
    <div className="codex-working" data-chat-running="">
      <span className="codex-working-spinner" aria-hidden="true" />
      <span className="codex-working-label">{waiting ? "Waiting for your action" : "Working"}</span>
    </div>
  );
}

/**
 * Codex's transcript surface: one centered 48rem column, a 16px item rhythm,
 * and Codex's 24px floor-ownership scroll semantics.
 */
export function CodexTranscript({
  transcript,
  busy,
  lastAssistantId,
  session,
  activity,
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
  const { sessions } = useStore();
  const visible = useMemo(
    () => visibleTranscript(transcript, sessions, session?.id),
    [transcript, sessions, session?.id]
  );
  const turns = useMemo(() => buildTurns(visible), [visible]);
  const { scrollRef, contentRef, showButton, onScroll, scrollToBottom } = useCodexScroll(
    session?.workspace.id,
    transcript,
    busy,
    turns.length
  );

  return (
    <div className="codex-chat" data-codex-chat="">
      <div
        ref={scrollRef}
        className="codex-scroll"
        data-codex-following={showButton ? undefined : ""}
        onScroll={onScroll}
      >
        <div ref={contentRef} className="codex-column">
          {showEmpty && turns.length === 0 && empty}
          {turns.map((turn, index) => (
            <CodexTurn
              key={turn.id}
              turn={turn}
              busy={busy}
              lastAssistantId={lastAssistantId}
              session={session}
              isLast={index === turns.length - 1}
            />
          ))}
          {busy && turns.length === 0 && <Working waiting={activity?.isWaitingForPermission === true} />}
        </div>
      </div>
      <div className="codex-to-bottom-slot" data-visible={showButton || undefined}>
        <button
          type="button"
          className="codex-to-bottom"
          aria-label="Scroll to bottom"
          aria-hidden={!showButton}
          tabIndex={showButton ? 0 : -1}
          onClick={scrollToBottom}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M4 6l4 4 4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </div>
    </div>
  );
}
