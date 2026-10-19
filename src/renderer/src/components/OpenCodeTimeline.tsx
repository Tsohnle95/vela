import { memo, useLayoutEffect, useMemo, useRef, type ReactNode } from "react";
import { useStore } from "../store";
import type { ToolCallView, TranscriptItem, SessionInfo } from "@shared/types";
import { TextShimmer } from "./TextShimmer";
import { OpenCodeMarkdown as Markdown } from "./OpenCodeMarkdown";
import { mountTooltip } from "../opencode-markdown/tooltip";
import { ToolPart, ContextToolGroup, SubagentLink } from "./OpenCodeTool";
import { TextPart, TimelineRow, SessionTurnThinking, UserMessage } from "./TimelineProse";
import { toolKey, isContextTool, renderableAssistantPart, consolidateSubagentTools, taskDescriptor, taskChildID,
  taskDispatchSignature, parseSubagentTag, parseLegacyTaskText, subagentChildID, refDispatchSignature,
  isInternalSystemReminder, buildTurns, latestReasoningHeading,
  type AssistantItem, type AssistantPart, type RenderablePart, type VisibleTimelineItem, type TimelineTurn } from "./timeline-model";

// OpenCode groups all assistant messages of a turn into one assistant part stream before
// folding context runs. Vela renders a contiguous run of assistant transcript items through
// this component so context runs can span message boundaries.
function AssistantRun({
  items,
  streaming,
  copyPartID,
  session,
  duration
}: {
  items: AssistantItem[];
  streaming: boolean;
  copyPartID: string | null;
  session: SessionInfo | null;
  duration?: number;
}): ReactNode {
  const rows: ReactNode[] = [];
  type Entry = { kind: "part"; part: RenderablePart; message: AssistantItem } | { kind: "context"; id: string; tools: ToolCallView[] };
  const groups: Entry[] = [];
  for (const message of items) for (const part of message.parts.filter(renderableAssistantPart)) {
    if (part.kind === "tool" && isContextTool(part.tool)) {
      const last = groups.at(-1);
      if (last?.kind === "context") last.tools.push(part.tool);
      else groups.push({ kind: "context", id: part.id, tools: [part.tool] });
    } else groups.push({ kind: "part", part, message });
  }
  for (let index = 0; index < groups.length; index += 1) {
    const group = groups[index];
    const part = group.kind === "part" ? group.part : undefined;
    const key = group.kind === "context" ? group.id : group.part.id;
    rows.push(
      <TimelineRow tag={part?.kind === "text" ? "AssistantMessage" : "AssistantActivity"} previous={index > 0} key={key}>
        <div data-slot="session-turn-assistant-content">
          {group.kind === "context" ? <ContextToolGroup tools={group.tools} session={session} working={streaming && index === groups.length - 1} />
            : group.part.kind === "text" ? <TextPart part={group.part} streaming={!group.message.completed}
                showCopy={group.part.id === copyPartID} message={group.message} duration={duration} />
            : <ToolPart tool={group.part.tool} session={session} />}
        </div>
      </TimelineRow>
    );
  }

  for (const item of items) {
    if (item.retry) {
      rows.push(
        <TimelineRow tag="Retry" previous key={`${item.id}:retry`}>
          <div data-slot="session-turn-retry">
            <span className="spinner" />
            <div>
              <div data-slot="session-turn-retry-message">{item.retry.message.slice(0, 80)}</div>
              <div data-slot="session-turn-retry-info">Retrying · attempt {item.retry.attempt}</div>
            </div>
          </div>
        </TimelineRow>
      );
    }
    if (item.error) {
      rows.push(
        <TimelineRow tag="Error" previous key={`${item.id}:error`}>
          <div data-component="card" data-variant="error" className="error-card">{item.error.replace(/^Error:\s*/, "")}</div>
        </TimelineRow>
      );
    }
  }
  return <>{rows}</>;
}

function renderTurnBody(
  body: TimelineTurn["body"],
  busy: boolean,
  lastAssistantId: string | null,
  session: SessionInfo | null,
  turnStartedAt?: number
): ReactNode[] {
  const nodes: ReactNode[] = [];
  const completedAt = body.filter((item): item is AssistantItem => item.kind === "assistant").at(-1)?.completedAt;
  const duration = turnStartedAt !== undefined && completedAt !== undefined ? completedAt - turnStartedAt : undefined;
  const copyPartID = busy ? null : body
    .flatMap((item) => item.kind === "assistant" ? item.parts : [])
    .filter((part): part is Extract<AssistantPart, { kind: "text" }> => part.kind === "text" && Boolean(part.text.trim()))
    .at(-1)?.id ?? null;
  let run: AssistantItem[] = [];
  const flush = (): void => {
    if (run.length === 0) return;
    const items = run;
    run = [];
    nodes.push(
      <AssistantRun
        items={items}
        streaming={busy && items.some((item) => item.id === lastAssistantId)}
        copyPartID={copyPartID}
        duration={duration}
        session={session}
        key={`assistant-run:${items[0].id}`}
      />
    );
  };
  for (const item of body) {
    if (item.kind === "assistant") {
      run.push(item);
      continue;
    }
    flush();
    nodes.push(<TimelineEvent item={item} session={session} key={item.id} />);
  }
  flush();
  return nodes;
}

function TimelineEvent({
  item,
  session
}: {
  item: Exclude<VisibleTimelineItem, { kind: "user" | "assistant" }>;
  session: SessionInfo | null;
}): ReactNode {
  const { sessions } = useStore();
  if (item.kind === "status") {
    const icon = item.tone === "error" ? "codicon-error" : item.tone === "success" ? "codicon-check" : "codicon-info";
    return (
      <TimelineRow tag="StatusNote">
        <div data-component="session-note" data-tone={item.tone}>
          <span className={`codicon ${icon}`} data-slot="session-note-icon" />
          <span data-slot="session-note-text">{item.text}</span>
        </div>
      </TimelineRow>
    );
  }
  if (item.kind === "divider") {
    return (
      <TimelineRow tag="TurnDivider">
        <div data-component="compaction-part">
          <span data-slot="compaction-part-line" />
          <span data-slot="compaction-part-label">Session compacted</span>
          <span data-slot="compaction-part-line" />
        </div>
      </TimelineRow>
    );
  }
  if (item.kind === "shell") {
    return (
      <TimelineRow tag="ShellMessage">
        <div data-slot="session-turn-assistant-content">
          <ToolPart session={session} tool={{
            id: item.shellID,
            title: "shell",
            detail: item.command ? `$ ${item.command}` : "",
            status: item.status === "running" ? "running" : item.status === "exited" && (!item.exit || item.exit === 0)
              ? "success"
              : "failed",
            input: JSON.stringify({ command: item.command }),
            inputValue: { command: item.command },
            output: item.output
          }} />
        </div>
      </TimelineRow>
    );
  }
  if (item.kind === "compaction") {
    const label = item.status === "running"
      ? "Compacting session"
      : item.status === "failed"
        ? "Compaction failed"
        : "Session compacted";
    return (
      <TimelineRow tag="Compaction">
        <div data-component="compaction-message" data-status={item.status}>
          <div data-component="compaction-part">
            <span data-slot="compaction-part-line" />
            <span data-slot="compaction-part-label">
              {item.status === "running" ? <TextShimmer text={label} /> : label}
            </span>
            <span data-slot="compaction-part-line" />
          </div>
          {item.summary && <Markdown text={item.summary} streaming={item.status === "running"} />}
          {item.error && (
            <div data-component="session-note" data-tone="error">
              <span className="codicon codicon-error" data-slot="session-note-icon" />
              <span data-slot="session-note-text">{item.error}</span>
            </div>
          )}
        </div>
      </TimelineRow>
    );
  }
  if (item.kind === "synthetic") {
    const ref = parseSubagentTag(item.text) ?? parseLegacyTaskText(item.text);
    if (ref && subagentChildID(ref, sessions, session?.id)) {
      return (
        <TimelineRow tag="SubagentLink">
          <SubagentLink item={item} session={session} />
        </TimelineRow>
      );
    }
  }
  const label = item.kind === "skill"
    ? `Skill activated · ${item.name}`
    : item.kind === "synthetic"
      ? item.description || "System message"
      : "System message";
  const text = item.text;
  return (
    <TimelineRow tag="SessionEvent">
      <div data-component="session-message" data-kind={item.kind}>
        <div data-slot="session-message-label">{label}</div>
        {item.kind === "skill" && item.skill && (
          <div data-slot="session-message-detail">{item.skill}</div>
        )}
        {text && <Markdown text={text} streaming={false} />}
      </div>
    </TimelineRow>
  );
}

export function PermissionPrompt({ item, session }: { item: Extract<TranscriptItem, { kind: "permission" }>; session: SessionInfo | null }): ReactNode {
  const { replyPermission } = useStore();
  if (!item.pending) return null;
  return (
    <div data-component="dock-prompt" data-kind="permission">
      <div data-slot="permission-header">Permission required</div>
      <div data-slot="permission-action">{item.action}</div>
      {item.resources.map((resource) => <code key={resource}>{resource}</code>)}
      <div data-slot="permission-actions">
        <button className="btn btn-primary" onClick={() => void replyPermission(item.requestID, "once", session?.id)}>Allow once</button>
        <button className="btn" onClick={() => void replyPermission(item.requestID, "always", session?.id)}>Always</button>
        <button className="btn btn-danger" onClick={() => void replyPermission(item.requestID, "reject", session?.id)}>Deny</button>
      </div>
    </div>
  );
}

export const OpenCodeTimeline = memo(function OpenCodeTimeline({
  transcript,
  busy,
  lastAssistantId,
  session
}: {
  transcript: TranscriptItem[];
  busy: boolean;
  lastAssistantId: string | null;
  session?: SessionInfo | null;
}): ReactNode {
  const store = useStore();
  const root = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => root.current ? mountTooltip(root.current) : undefined, []);
  useLayoutEffect(() => {
    const html = document.documentElement;
    const sync = (): void => {
      if (root.current) root.current.dataset.colorScheme = getComputedStyle(html).colorScheme === "light" ? "light" : "dark";
    };
    const observer = new MutationObserver(sync);
    observer.observe(html, { attributes: true, attributeFilter: ["style", "class", "data-theme"] });
    sync();
    return () => observer.disconnect();
  }, []);
  const activeSession = session === undefined ? store.session : session;
  const consolidatedTranscript = useMemo(
    () => consolidateSubagentTools(transcript, store.sessions, activeSession?.id),
    [transcript, store.sessions, activeSession?.id]
  );
  const representedSubagents = useMemo(() => {
    const ids = new Set<string>();
    let turn = 0;
    for (const item of consolidatedTranscript) {
      if (item.kind === "user") {
        turn += 1;
        continue;
      }
      if (item.kind !== "assistant") continue;
      for (const part of item.parts) {
        if (part.kind !== "tool" || !["task", "subagent"].includes(toolKey(part.tool.title))) continue;
        const request = taskDescriptor(part.tool);
        const childID = taskChildID(part.tool, store.sessions, activeSession?.id, request);
        if (childID) ids.add(`${turn}:child:${childID}`);
        const signature = taskDispatchSignature(part.tool, request);
        if (signature) ids.add(`${turn}:${signature}`);
      }
    }
    return ids;
  }, [consolidatedTranscript, store.sessions, activeSession?.id]);
  const timeline = useMemo(() => {
    const visible: VisibleTimelineItem[] = [];
    let turn = 0;
    for (const item of consolidatedTranscript) {
      if (item.kind === "user") turn += 1;
      if (item.kind === "permission" || item.kind === "pending-input" || item.kind === "selection" || item.kind === "system") {
        continue;
      }
      if (item.kind === "synthetic") {
        const ref = parseSubagentTag(item.text) ?? parseLegacyTaskText(item.text);
        const childID = ref ? subagentChildID(ref, store.sessions, activeSession?.id) : "";
        if (childID && representedSubagents.has(`${turn}:child:${childID}`)) continue;
        if (ref && representedSubagents.has(`${turn}:${refDispatchSignature(ref)}`)) continue;
      }
      if (item.kind !== "synthetic" || !isInternalSystemReminder(item)) visible.push(item);
    }
    return visible;
  }, [consolidatedTranscript, representedSubagents, store.sessions, activeSession?.id]);
  const turns = useMemo(() => buildTurns(timeline), [timeline]);
  const activeAssistants = useMemo(
    () => (turns.at(-1)?.body ?? []).filter((item): item is AssistantItem => item.kind === "assistant"),
    [turns]
  );
  const thinkingHeading = useMemo(() => latestReasoningHeading(activeAssistants), [activeAssistants]);
  const activeTurnError = activeAssistants.some((item) => Boolean(item.error));
  const activeTurnRetry = activeAssistants.some((item) => Boolean(item.retry));
  const showThinking = busy && !activeTurnError && !activeTurnRetry;

  return (
    <div ref={root} data-slot="session-turn-list" className="opencode-timeline" data-new-layout="">
      {turns.map((turn, index) => {
        return (
          <div data-component="session-turn-group" key={turn.id}>
            {turn.user && index > 0 && <div data-timeline-row="TurnGap" aria-hidden="true" />}
            {turn.user && <UserMessage item={turn.user} session={activeSession} busy={busy} />}
            {renderTurnBody(turn.body, busy && index === turns.length - 1, lastAssistantId, activeSession, turn.user?.createdAt)}
          </div>
        );
      })}
      {showThinking && <SessionTurnThinking heading={thinkingHeading} />}
    </div>
  );
});
