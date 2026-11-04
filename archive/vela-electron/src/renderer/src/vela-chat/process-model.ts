import type { AssistantPartView, ToolCallView, TranscriptItem } from "@shared/types";
import type { AssistantItem, TimelineTurn } from "../components/timeline-model";
import { effectiveToolKey, formatDuration, toolInput, toolPresentation } from "../components/timeline-model";

export type ProcessActivity =
  | "thinking"
  | "read"
  | "readImage"
  | "search"
  | "write"
  | "edit"
  | "commands"
  | "code"
  | "webSearch"
  | "webFetch"
  | "subagents"
  | "plan"
  | "questions"
  | "tools";

export interface ProcessCount {
  kind: ProcessActivity;
  count: number;
}

export interface TurnAnswer {
  item: AssistantItem;
  parts: Extract<AssistantPartView, { kind: "text" }>[];
}

export interface TurnProcess {
  /** Assistant and shell rows before the final answer, plus any reasoning in the answer step. */
  rows: TranscriptItem[];
  /** Non-process rows in the turn (status notes, compaction markers, notices). */
  events: TranscriptItem[];
  answer: TurnAnswer | null;
  counts: ProcessCount[];
  hasProcess: boolean;
  foldable: boolean;
  durationMs: number | undefined;
  live: LiveProcess;
}

export interface LiveProcess {
  running: boolean;
  activity: ProcessActivity;
  detail: string;
}

const DONE_LABELS: Record<ProcessActivity, string> = {
  thinking: "Analysis completed",
  read: "Read files",
  readImage: "Read images",
  search: "Searched code",
  write: "Wrote files",
  edit: "Edited files",
  commands: "Ran commands",
  code: "Ran code",
  webSearch: "Searched the web",
  webFetch: "Visited web pages",
  subagents: "Coordinated subagents",
  plan: "Updated the plan",
  questions: "Asked questions",
  tools: "Called tools"
};

const LIVE_LABELS: Record<ProcessActivity, string> = {
  thinking: "Analyzing the request",
  read: "Reading files",
  readImage: "Reading images",
  search: "Searching code",
  write: "Writing files",
  edit: "Editing files",
  commands: "Running commands",
  code: "Running code",
  webSearch: "Searching the web",
  webFetch: "Visiting web pages",
  subagents: "Coordinating subagents",
  plan: "Updating the plan",
  questions: "Waiting for your action",
  tools: "Calling tools"
};

const TOOL_ACTIVITIES: Record<string, ProcessActivity> = {
  read: "read",
  read_image: "readImage",
  grep: "search",
  glob: "search",
  list: "search",
  write: "write",
  edit: "edit",
  patch: "edit",
  apply_patch: "edit",
  bash: "commands",
  shell: "commands",
  terminal: "commands",
  run_code: "code",
  code: "code",
  websearch: "webSearch",
  web_search: "webSearch",
  search: "webSearch",
  webfetch: "webFetch",
  web_fetch: "webFetch",
  task: "subagents",
  subagent: "subagents",
  todowrite: "plan",
  todo_write: "plan",
  goal: "plan",
  plan: "plan",
  question: "questions",
  ask_user_question: "questions"
};

function lowerFirst(value: string): string {
  return value.length === 0 ? value : value[0]!.toLowerCase() + value.slice(1);
}

/** Reproduce Chat's closed process-group title: top three categories, no counts. */
export function processTitle(counts: readonly ProcessCount[]): string {
  const labels = counts.slice(0, 3).map(({ kind }) => DONE_LABELS[kind]);
  const first = labels[0];
  if (first === undefined) return DONE_LABELS.thinking;
  const second = labels[1];
  if (second === undefined) return first;
  if (labels.length === 2) return `${first} and ${lowerFirst(second)}`;
  const title = [first, ...labels.slice(1).map(lowerFirst)].join(", ");
  return counts.length > 3 ? `${title}, etc.` : title;
}

/** Live process-group title used while the turn is still running. */
export function liveLabel(activity: ProcessActivity): string {
  return LIVE_LABELS[activity];
}

/** Category a tool contributes to process grouping. */
export function activityOfTool(tool: ToolCallView): ProcessActivity {
  const key = effectiveToolKey(tool);
  return TOOL_ACTIVITIES[key]
    ?? (key.startsWith("terminal_") ? "commands" : key.endsWith("_inspect") ? "search" : "tools");
}

function isProcessItem(item: TranscriptItem): boolean {
  return item.kind === "assistant" || item.kind === "shell";
}

function assistantTextParts(item: AssistantItem): Extract<AssistantPartView, { kind: "text" }>[] {
  return item.parts.filter((part): part is Extract<AssistantPartView, { kind: "text" }> =>
    part.kind === "text" && Boolean(part.text.trim())
  );
}

function cloneAssistant(item: AssistantItem, parts: AssistantPartView[]): AssistantItem {
  return { ...item, parts };
}

interface AnswerExtraction {
  item: AssistantItem;
  start: number;
  parts: Extract<AssistantPartView, { kind: "text" }>[];
}

/** Trailing text of the last assistant step; a later tool-only step has no answer. */
function extractAnswer(body: TranscriptItem[]): AnswerExtraction | undefined {
  const item = [...body].reverse().find((candidate): candidate is AssistantItem => candidate.kind === "assistant");
  if (item === undefined || assistantTextParts(item).length === 0) return undefined;
  let lastTool = -1;
  for (let index = 0; index < item.parts.length; index += 1) {
    if (item.parts[index]?.kind === "tool") lastTool = index;
  }
  const start = item.parts.findIndex((part, index) =>
    part.kind === "text" && index > lastTool && Boolean(part.text.trim())
  );
  if (start < 0) return undefined;
  const parts = item.parts.filter((part, index): part is Extract<AssistantPartView, { kind: "text" }> =>
    part.kind === "text" && index >= start && Boolean(part.text.trim())
  );
  return { item, start, parts };
}

function partitionRows(body: TranscriptItem[], answer: AnswerExtraction | undefined): {
  rows: TranscriptItem[];
  events: TranscriptItem[];
} {
  const rows: TranscriptItem[] = [];
  const events: TranscriptItem[] = [];
  for (const item of body) {
    if (answer !== undefined && item === answer.item) {
      if (answer.start > 0) rows.push(cloneAssistant(item, item.parts.slice(0, answer.start)));
      continue;
    }
    if (isProcessItem(item)) rows.push(item);
    else events.push(item);
  }
  return { rows, events };
}

interface ActivityScan {
  counts: ProcessCount[];
  running: ProcessActivity | undefined;
  detail: string;
}

function scanActivity(rows: TranscriptItem[]): ActivityScan {
  const counts = new Map<ProcessActivity, number>();
  let running: ProcessActivity | undefined;
  let runningDetail = "";
  let latestReasoning = "";
  for (const item of rows) {
    if (item.kind === "shell") {
      counts.set("commands", (counts.get("commands") ?? 0) + 1);
      if (item.status === "running") {
        running = "commands";
        runningDetail = item.command;
      }
      continue;
    }
    if (item.kind !== "assistant") continue;
    for (const part of item.parts) {
      if (part.kind === "tool") {
        const activity = activityOfTool(part.tool);
        counts.set(activity, (counts.get(activity) ?? 0) + 1);
        if (part.tool.status === "running") {
          running = activity;
          runningDetail = toolDetail(part.tool);
        }
      } else if (part.kind === "reasoning" && part.text.trim()) {
        latestReasoning = latestParagraph(part.text);
      }
    }
  }
  return {
    counts: [...counts].map(([kind, count]) => ({ kind, count }))
      .sort((left, right) => right.count - left.count),
    running,
    detail: running === undefined ? latestReasoning : runningDetail
  };
}

function latestParagraph(text: string): string {
  const paragraphs = text.replace(/\r\n?/g, "\n").split(/\n[\t ]*\n+/);
  for (let index = paragraphs.length - 1; index >= 0; index -= 1) {
    const paragraph = paragraphs[index]?.replaceAll("**", "").replace(/\s+/g, " ").trim() ?? "";
    if (paragraph) return paragraph.slice(0, 160);
  }
  return "";
}

function toolDetail(tool: ToolCallView): string {
  const presentation = toolPresentation(tool);
  if (presentation.subtitle.trim()) return presentation.subtitle.trim().slice(0, 160);
  const input = toolInput(tool);
  for (const key of ["command", "path", "file_path", "pattern", "query", "url", "description", "prompt"]) {
    const value = input[key];
    if (typeof value === "string" && value.trim()) return value.trim().slice(0, 160);
  }
  return tool.detail.trim().slice(0, 160);
}

function turnDuration(turn: TimelineTurn, answer: AssistantItem | undefined, busy: boolean): number | undefined {
  if (busy) return undefined;
  const endedAt = answer?.completedAt ?? answer?.createdAt;
  const startedAt = turn.user?.createdAt;
  return endedAt !== undefined && startedAt !== undefined ? Math.max(1000, endedAt - startedAt) : undefined;
}

/**
 * Split one turn into the process rows Chat folds away and the final answer that
 * survives folding. Chat's model has one assistant step per model call; Vela's
 * runtime can keep commentary, tools, and the final text in one message, so the
 * trailing text after the last tool call is the answer and everything before it
 * is process.
 */
export function turnProcess(turn: TimelineTurn, busy: boolean): TurnProcess {
  const answer = extractAnswer(turn.body);
  const { rows, events } = partitionRows(turn.body, answer);
  const scan = scanActivity(rows);
  const last = answer?.item ?? [...rows].reverse().find((item): item is AssistantItem => item.kind === "assistant");
  const failed = Boolean(last?.error) || last?.interrupted === true;
  return {
    rows,
    events,
    answer: answer === undefined ? null : { item: answer.item, parts: answer.parts },
    counts: scan.counts,
    hasProcess: rows.some((item) => item.kind === "assistant"
      ? item.parts.length > 0
      : item.kind === "shell"),
    foldable: !busy && !failed && rows.length > 0,
    durationMs: turnDuration(turn, last, busy),
    live: {
      running: scan.running !== undefined,
      activity: scan.running ?? "thinking",
      detail: scan.detail
    }
  };
}

/** DSH's completed-turn control copy. */
export function processStatusLabel(process: TurnProcess): string {
  if (process.durationMs === undefined) return "Completed";
  return `Completed in ${formatDuration(process.durationMs)}`;
}

/** Time label for the running status; DSH keeps the first second visible. */
export function runningDuration(startedAt: number | undefined, now: number): string {
  if (startedAt === undefined) return "Deep diving ···";
  return `Deep diving for ${formatDuration(Math.max(1000, now - startedAt))} ···`;
}
