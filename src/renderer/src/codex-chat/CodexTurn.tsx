import { useEffect, useMemo, useState, type ReactNode } from "react";
import type { SessionInfo, ToolCallView, TranscriptItem } from "@shared/types";
import type { TimelineTurn } from "../components/timeline-model";
import { writeClipboard } from "../components/TimelineProse";
import { turnProcess, type TurnProcess } from "../dsh-chat/process-model";
import { CodexMarkdown } from "./CodexMarkdown";
import { CodexReasoning } from "./CodexReasoning";
import { CodexToolRow } from "./CodexToolRow";

function formatDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  if (total < 60) return `${total}s`;
  const minutes = Math.floor(total / 60);
  return `${minutes}m ${total % 60}s`;
}

function useNow(active: boolean): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!active) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [active]);
  return now;
}

function ShellRow({ item }: { item: Extract<TranscriptItem, { kind: "shell" }> }) {
  const tool: ToolCallView = {
    id: item.shellID,
    title: "shell",
    detail: item.command ? `$ ${item.command}` : "",
    status: item.status === "running" ? "running" : item.status === "exited" && (!item.exit || item.exit === 0)
      ? "success"
      : "failed",
    input: JSON.stringify({ command: item.command }),
    inputValue: { command: item.command },
    output: item.output
  };
  return <CodexToolRow tool={tool} />;
}

function processRow(
  item: TranscriptItem,
  index: number,
  streaming: boolean,
  lastAssistantId: string | null
): ReactNode {
  if (item.kind === "shell") return <ShellRow item={item} key={`${item.id}:${index}`} />;
  if (item.kind !== "assistant") return null;
  return item.parts.map((part, partIndex) => {
    if (part.kind === "reasoning") {
      if (!part.text.trim()) return null;
      return (
        <CodexReasoning
          key={`${part.id}:${partIndex}`}
          text={part.text}
          running={streaming && item.id === lastAssistantId && !part.complete}
        />
      );
    }
    if (part.kind === "tool") {
      return <CodexToolRow key={`${part.id}:${partIndex}`} tool={part.tool} />;
    }
    if (!part.text.trim()) return null;
    return (
      <CodexMarkdown
        key={`${part.id}:${partIndex}`}
        text={part.text}
        streaming={streaming && item.id === lastAssistantId && !item.completed}
      />
    );
  });
}

function TurnEvent({ item }: { item: TranscriptItem }) {
  switch (item.kind) {
    case "status":
      return <div className="codex-status-note" data-tone={item.tone}>{item.text}</div>;
    case "divider":
      return <div className="codex-turn-divider"><span>Session compacted</span></div>;
    case "compaction":
      return (
        <div className="codex-status-note" data-tone={item.status === "failed" ? "error" : "info"}>
          {item.status === "running" ? "Compacting context…" : item.status === "failed" ? "Compaction failed" : "Context compacted"}
        </div>
      );
    case "synthetic":
    case "skill":
      return (
        <div className="codex-context-row">
          {item.kind === "skill" && <span className="codex-context-label">Skill · {item.name}</span>}
          <CodexMarkdown text={item.text} />
        </div>
      );
    default:
      return null;
  }
}

function WorkDivider({
  process,
  running,
  startedAt,
  open,
  onToggle,
  children
}: {
  process: TurnProcess;
  running: boolean;
  startedAt: number | undefined;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  const now = useNow(running);
  const label = running
    ? `Working for ${formatDuration(Math.max(1000, startedAt === undefined ? 1000 : now - startedAt))}`
    : process.durationMs !== undefined
      ? `Worked for ${formatDuration(process.durationMs)}`
      : `${process.rows.length} previous messages`;
  return (
    <div className="codex-work" data-running={running || undefined}>
      <div className="codex-work-headrow">
        <button
          type="button"
          className="codex-work-head"
          aria-expanded={open}
          onClick={onToggle}
        >
          <span className="codex-work-label">{label}</span>
          <svg className="codex-work-chevron" viewBox="0 0 16 16" aria-hidden="true" data-open={open || undefined}>
            <path d="M6 4l4 4-4 4" />
          </svg>
        </button>
      </div>
      <div className="codex-work-rule" aria-hidden="true" />
      {open && <div className="codex-work-body">{children}</div>}
    </div>
  );
}

function Answer({
  answer,
  streaming,
  turn
}: {
  answer: NonNullable<TurnProcess["answer"]>;
  streaming: boolean;
  turn: TimelineTurn;
}) {
  const [copied, setCopied] = useState(false);
  const text = useMemo(() => answer.parts.map((part) => part.text).join("\n\n"), [answer.parts]);
  const elapsed = answer.item.completedAt !== undefined && turn.user?.createdAt !== undefined
    ? Math.max(0, answer.item.completedAt - turn.user.createdAt)
    : answer.item.completedAt !== undefined && answer.item.createdAt !== undefined
      ? Math.max(0, answer.item.completedAt - answer.item.createdAt)
      : undefined;
  const copy = (): void => {
    if (!text) return;
    void writeClipboard(text).then((ok) => {
      if (!ok) return;
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };
  return (
    <div className="codex-answer" data-streaming={streaming || undefined}>
      <CodexMarkdown text={text} streaming={streaming} />
      {answer.item.interrupted && <span className="codex-answer-stopped">Stopped</span>}
      {!streaming && (
        <div className="codex-answer-footer">
          <button type="button" className="codex-answer-copy" onClick={copy}>{copied ? "Copied" : "Copy"}</button>
          {elapsed !== undefined && <span className="codex-answer-meta">{formatDuration(elapsed)}</span>}
        </div>
      )}
    </div>
  );
}

/** One Codex turn: right-aligned prompt, a worked-for activity divider, then the answer. */
export function CodexTurn({
  turn,
  busy,
  lastAssistantId,
  session,
  isLast
}: {
  turn: TimelineTurn;
  busy: boolean;
  lastAssistantId: string | null;
  session: SessionInfo | null;
  isLast: boolean;
}) {
  const process = useMemo(() => turnProcess(turn, busy), [turn, busy]);
  const [workOpen, setWorkOpen] = useState(false);
  const running = process.live.running || (busy && isLast && process.answer === null);
  const showActivity = process.hasProcess || running;
  const open = running || workOpen;
  const streaming = busy && isLast && lastAssistantId !== null && process.answer?.item.id === lastAssistantId;
  const startedAt = turn.user?.createdAt ?? process.answer?.item.createdAt;
  return (
    <section className="codex-turn" data-turn={turn.id}>
      {turn.user && (
        <div className="codex-message" data-role="user">
          <div className="codex-user-bubble">{turn.user.text}</div>
        </div>
      )}
      {process.events.map((item) => <TurnEvent item={item} key={item.id} />)}
      {showActivity && (
        <WorkDivider
          process={process}
          running={running}
          startedAt={startedAt}
          open={open}
          onToggle={() => setWorkOpen(!open)}
        >
          {process.rows.map((item, index) => processRow(item, index, busy, lastAssistantId))}
        </WorkDivider>
      )}
      {process.answer && (
        <div className="codex-message" data-role="assistant">
          <Answer answer={process.answer} streaming={streaming || false} turn={turn} />
        </div>
      )}
      {process.answer?.item.retry && (
        <div className="codex-status-note" data-tone="info">
          {process.answer.item.retry.message} · attempt {process.answer.item.retry.attempt}
        </div>
      )}
      {process.answer?.item.error && <div className="codex-error-text">{process.answer.item.error}</div>}
    </section>
  );
}
