import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  DisclosureRow,
  IconAgentPresetOutlineRegular,
  IconApiOutlineRegular,
  IconArchiveOutlineRegular,
  IconBrowseOutlineRegular,
  IconChevronDownOutlineRegular,
  IconChevronUpOutlineRegular,
  IconCodeOutlineRegular,
  IconEditOutlineRegular,
  IconGlobeOutlineRegular,
  IconPlanOutlineRegular,
  IconQuestionOutlineRegular,
  IconSearchOutlineRegular,
  IconSparkleRegular,
  IconThinkOutlineRegular,
  TextShimmer
} from "@deepseek-ai/dsh-client-ui-primitives";
import type { SessionInfo, ToolCallView, TranscriptItem } from "@shared/types";
import { useStore } from "../store";
import type { TimelineTurn } from "../components/timeline-model";
import { DshMarkdown } from "./DshMarkdown";
import { DshReasoning } from "./DshReasoning";
import { DshToolRow } from "./DshToolRow";
import { DshUserMessage } from "./DshUserMessage";
import { writeClipboard } from "../components/TimelineProse";
import {
  liveLabel,
  processStatusLabel,
  processTitle,
  turnProcess,
  type ProcessActivity,
  type TurnAnswer,
  type TurnProcess
} from "./process-model";

const PROCESS_ICONS: Record<ProcessActivity, ReactNode> = {
  thinking: <IconThinkOutlineRegular />,
  read: <IconBrowseOutlineRegular size={14} />,
  readImage: <IconBrowseOutlineRegular size={14} />,
  search: <IconSearchOutlineRegular size={14} />,
  edit: <IconEditOutlineRegular size={14} />,
  write: <IconEditOutlineRegular size={14} />,
  commands: <IconApiOutlineRegular />,
  code: <IconCodeOutlineRegular size={14} />,
  webSearch: <IconGlobeOutlineRegular />,
  webFetch: <IconBrowseOutlineRegular size={14} />,
  subagents: <IconAgentPresetOutlineRegular size={14} />,
  plan: <IconPlanOutlineRegular />,
  questions: <IconQuestionOutlineRegular />,
  tools: <IconSparkleRegular size={14} />
};

function ShellRow({ item, session }: { item: Extract<TranscriptItem, { kind: "shell" }>; session: SessionInfo | null }) {
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
  return <DshToolRow tool={tool} session={session} />;
}

function processRow(item: TranscriptItem, index: number, streaming: boolean, lastAssistantId: string | null, session: SessionInfo | null): ReactNode {
  if (item.kind === "shell") return <ShellRow item={item} session={session} key={`${item.id}:${index}`} />;
  if (item.kind !== "assistant") return null;
  return item.parts.map((part, partIndex) => {
    if (part.kind === "reasoning") {
      if (!part.text.trim()) return null;
      return (
        <DshReasoning
          key={`${part.id}:${partIndex}`}
          text={part.text}
          running={streaming && item.id === lastAssistantId && !part.complete}
          resetKey={`${item.id}:${part.id}`}
        />
      );
    }
    if (part.kind === "tool") {
      return <DshToolRow key={`${part.id}:${partIndex}`} tool={part.tool} session={session} />;
    }
    if (!part.text.trim()) return null;
    return (
      <div className="dsh-process-text" key={`${part.id}:${partIndex}`}>
        <DshMarkdown
          text={part.text}
          streaming={streaming && item.id === lastAssistantId && !item.completed}
        />
      </div>
    );
  });
}

function ProcessGroup({
  process,
  session,
  lastAssistantId,
  busy
}: {
  process: TurnProcess;
  session: SessionInfo | null;
  lastAssistantId: string | null;
  busy: boolean;
}) {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!process.foldable) setOpen(false);
  }, [process.foldable]);
  const activity = process.live.running ? process.live.activity : process.counts[0]?.kind ?? "thinking";
  const label = process.live.running ? liveLabel(activity) : processTitle(process.counts);
  const detail = process.foldable ? "" : process.live.detail;
  const title = detail === "" ? label : `${label} · ${detail}`;
  return (
    <div className="dsh-process-group" data-live={!process.foldable || undefined}>
      <button
        type="button"
        className="dsh-process-header"
        aria-expanded={open}
        onClick={(event) => {
          event.currentTarget.focus();
          setOpen((value) => !value);
        }}
      >
        <span className="dsh-process-leading" aria-hidden="true">
          <span className="dsh-process-activity">{PROCESS_ICONS[activity]}</span>
          <span className="dsh-process-chevron">
            {open ? <IconChevronUpOutlineRegular /> : <IconChevronDownOutlineRegular />}
          </span>
        </span>
        <TextShimmer active={!process.foldable}>
          <TextShimmer className="dsh-process-label">{title}</TextShimmer>
        </TextShimmer>
      </button>
      {open && (
        <div className="dsh-process-body">
          {process.rows.map((item, index) => processRow(item, index, busy, lastAssistantId, session))}
        </div>
      )}
    </div>
  );
}

function CompactionRow({ item }: { item: Extract<TranscriptItem, { kind: "compaction" }> }) {
  const [open, setOpen] = useState(false);
  const running = item.status === "running";
  const title = running ? "Compacting context…" : item.status === "failed" ? "Compaction failed" : "Context compacted";
  return (
    <div className="dsh-compaction">
      <DisclosureRow
        icon={<IconArchiveOutlineRegular size={14} />}
        title={title}
        running={running}
        open={open}
        expandable={Boolean(item.summary) || Boolean(item.error)}
        expandOnRowClick
        onToggle={() => setOpen((value) => !value)}
      >
        {open && (
          <div className="dsh-compaction-body">
            {item.summary && <DshMarkdown text={item.summary} streaming={running} />}
            {item.error && <p className="dsh-error-text">{item.error}</p>}
          </div>
        )}
      </DisclosureRow>
    </div>
  );
}

function TurnEvent({ item }: { item: TranscriptItem }) {
  switch (item.kind) {
    case "status":
      return <div className="dsh-status-note" data-tone={item.tone}>{item.text}</div>;
    case "divider":
      return <div className="dsh-turn-divider"><span>Session compacted</span></div>;
    case "compaction":
      return <CompactionRow item={item} />;
    case "synthetic":
    case "skill":
      return (
        <div className="dsh-context-row">
          {item.kind === "skill" && <span className="dsh-context-label">Skill · {item.name}</span>}
          <DshMarkdown text={item.text} />
        </div>
      );
    default:
      return null;
  }
}

function Answer({
  answer,
  streaming,
  turn
}: {
  answer: TurnAnswer;
  streaming: boolean;
  turn: TimelineTurn;
}) {
  const { models } = useStore();
  const [copied, setCopied] = useState(false);
  const text = answer.parts.map((part) => part.text).join("\n\n");
  const elapsed = answer.item.completedAt !== undefined && turn.user?.createdAt !== undefined
    ? Math.max(0, answer.item.completedAt - turn.user.createdAt)
    : answer.item.completedAt !== undefined && answer.item.createdAt !== undefined
      ? Math.max(0, answer.item.completedAt - answer.item.createdAt)
      : undefined;
  const seconds = elapsed === undefined ? undefined : Math.round(elapsed / 1000);
  const duration = seconds === undefined ? "" : seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  const model = models?.find((entry) => entry.id === answer.item.model?.id && entry.providerID === answer.item.model?.providerID)?.name
    ?? answer.item.model?.id;
  const meta = [answer.item.agent && answer.item.agent[0].toUpperCase() + answer.item.agent.slice(1), model, duration]
    .filter(Boolean).join(" · ");
  const copy = (): void => {
    if (!text) return;
    void writeClipboard(text).then((ok) => {
      if (!ok) return;
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };
  return (
    <div className="dsh-answer" data-streaming={streaming || undefined}>
      {answer.parts.map((part, index) => (
        <DshMarkdown key={`${part.id}:${index}`} text={part.text} streaming={streaming} />
      ))}
      {answer.item.interrupted && <span className="dsh-answer-stopped">Stopped</span>}
      {!streaming && (
        <div className="dsh-answer-footer">
          <button type="button" className="dsh-answer-copy" onClick={copy}>{copied ? "Copied" : "Copy"}</button>
          {meta && <span className="dsh-answer-meta">{meta}</span>}
        </div>
      )}
    </div>
  );
}

/** One transcript turn with DSH's whole-turn process fold and answer boundary. */
export function DshTurn({
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
  const [processExpanded, setProcessExpanded] = useState(false);
  useEffect(() => {
    if (!process.foldable) setProcessExpanded(false);
  }, [process.foldable]);
  const open = !process.foldable || processExpanded;
  const streaming = busy && isLast && lastAssistantId !== null
    && process.answer?.item.id === lastAssistantId;
  return (
    <div className="dsh-turn" data-turn={turn.id}>
      {turn.user && <DshUserMessage item={turn.user} session={session} busy={busy} />}
      {process.events.map((item) => <TurnEvent item={item} key={item.id} />)}
      {process.hasProcess && process.foldable && (
        <button
          type="button"
          className="dsh-process-status"
          data-open={open || undefined}
          aria-expanded={open}
          onClick={() => setProcessExpanded((value) => !value)}
        >
          <span className="dsh-process-status-label">{processStatusLabel(process)}</span>
          <span className="dsh-process-status-chevron">
            {open ? <IconChevronUpOutlineRegular /> : <IconChevronDownOutlineRegular />}
          </span>
        </button>
      )}
      {open && process.hasProcess && (
        <ProcessGroup process={process} session={session} lastAssistantId={lastAssistantId} busy={busy} />
      )}
      {process.answer && (
        <Answer answer={process.answer} streaming={streaming || false} turn={turn} />
      )}
      {process.answer?.item.retry && (
        <div className="dsh-status-note" data-tone="info">
          {process.answer.item.retry.message} · attempt {process.answer.item.retry.attempt}
        </div>
      )}
      {process.answer?.item.error && <div className="dsh-error-text">{process.answer.item.error}</div>}
    </div>
  );
}

