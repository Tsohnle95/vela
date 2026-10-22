import type { ToolCallView, TranscriptItem, SessionSummary, SessionInfo } from "@shared/types";

export type AssistantItem = Extract<TranscriptItem, { kind: "assistant" }>;
export type AssistantPart = AssistantItem["parts"][number];
export type VisibleTimelineItem = Exclude<TranscriptItem, { kind: "permission" | "pending-input" | "selection" | "system" }>;

export function isInternalSystemReminder(item: Extract<TranscriptItem, { kind: "synthetic" }>): boolean {
  return /<system-reminder(?:\s[^>]*)?>[\s\S]*<\/system-reminder>/i.test(item.text);
}

export function parseInput(value: string | undefined): Record<string, unknown> {
  if (!value) return {};
  try {
    const parsed = JSON.parse(value) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : {};
  } catch {
    const partial: Record<string, string> = {};
    for (const field of ["command", "filePath", "file_path", "path", "pattern", "query", "url", "description", "prompt"]) {
      const match = new RegExp(`"${field}"\\s*:\\s*"((?:\\\\.|[^"\\\\])*)`).exec(value);
      if (!match) continue;
      partial[field] = match[1]
        .replace(/\\n/g, "\n")
        .replace(/\\"/g, '"')
        .replace(/\\\\/g, "\\");
    }
    return partial;
  }
}

export function fileName(value: unknown): string {
  if (typeof value !== "string") return "";
  return value.split(/[\\/]/).pop() ?? value;
}

// OpenCode's `getDirectory` + `relativizeProjectPath`: the parent directory of a tool path
// (trailing slash kept), stripped of the session workspace prefix when it sits underneath it.
export function contextDirectory(path: string | undefined, session: SessionInfo | null): string {
  const raw = path && path.length > 0 ? path : "/";
  const trimmed = raw.replace(/[/\\]+$/, "");
  const directory = `${trimmed.split(/[\\/]/).slice(0, -1).join("/")}/`;
  if (!session) return directory;
  const workspace = session.directory.replace(/\\/g, "/").replace(/\/$/, "");
  if (workspace === "/") return directory;
  return directory.startsWith(`${workspace}/`) ? directory.slice(workspace.length) : directory;
}

export function workspaceFilePath(path: string, session: SessionInfo | null): string | null {
  const normalizedPath = path.replace(/\\/g, "/");
  if (!session) return normalizedPath;
  const directory = session.directory.replace(/\\/g, "/").replace(/\/$/, "");
  if (/^(?:\/|[A-Za-z]:\/)/.test(normalizedPath)) {
    const caseInsensitive = /^[A-Za-z]:\//.test(normalizedPath);
    const comparedPath = caseInsensitive ? normalizedPath.toLowerCase() : normalizedPath;
    const comparedDirectory = caseInsensitive ? directory.toLowerCase() : directory;
    if (!comparedPath.startsWith(`${comparedDirectory}/`)) return null;
    return normalizedPath.slice(directory.length + 1);
  }
  return normalizedPath.replace(/^\.\//, "");
}

export function titleCase(value: string): string {
  if (!value) return "Tool";
  return value.charAt(0).toUpperCase() + value.slice(1);
}

export function toolKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z_]/g, "");
}

export function toolInput(tool: ToolCallView): Record<string, unknown> {
  if (tool.inputValue && typeof tool.inputValue === "object" && !Array.isArray(tool.inputValue)) {
    return tool.inputValue as Record<string, unknown>;
  }
  return parseInput(tool.input);
}

export function effectiveToolKey(tool: ToolCallView): string {
  const explicit = toolKey(tool.title);
  if (!tool.metadata?.deepseek) return explicit || "tool";
  if (explicit && explicit !== "tool") return explicit;
  const input = toolInput(tool);
  if (typeof input.command === "string") return "shell";
  if (typeof input.filePath === "string" || typeof input.file_path === "string") return "read";
  if (typeof input.pattern === "string") return "grep";
  if (typeof input.query === "string") return "search";
  if (typeof input.url === "string") return "webfetch";
  if (typeof input.path === "string") return "inspect";
  if (typeof input.prompt === "string" || typeof input.description === "string") return "task";
  return explicit || "tool";
}

// Context reads are low-signal. Like OpenCode's `isContextGroupTool`, a
// contiguous run of these is folded into one `ContextToolGroup` so a burst of
// exploration never fills the transcript.
const CONTEXT_TOOL_KEYS = new Set(["read", "list", "glob", "grep"]);

export function isContextTool(tool: ToolCallView): boolean {
  return CONTEXT_TOOL_KEYS.has(effectiveToolKey(tool));
}

export function contextToolArgs(tool: ToolCallView): string[] {
  const name = effectiveToolKey(tool);
  const input = toolInput(tool);
  if (name === "read") {
    const args: string[] = [];
    if (typeof input.offset === "number") args.push(`offset=${input.offset}`);
    if (typeof input.limit === "number") args.push(`limit=${input.limit}`);
    return args;
  }
  if (name === "glob") return typeof input.pattern === "string" ? [`pattern=${input.pattern}`] : [];
  if (name === "grep") {
    const args: string[] = [];
    if (typeof input.pattern === "string") args.push(`pattern=${input.pattern}`);
    if (typeof input.include === "string") args.push(`include=${input.include}`);
    return args;
  }
  return [];
}

export function contextToolSummary(tools: ToolCallView[]): { read: number; search: number; list: number } {
  let read = 0;
  let search = 0;
  let list = 0;
  for (const tool of tools) {
    const name = effectiveToolKey(tool);
    if (name === "read") read += 1;
    else if (name === "glob" || name === "grep") search += 1;
    else if (name === "list") list += 1;
  }
  return { read, search, list };
}

export interface SubagentRef {
  id: string;
  agent: string;
  description: string;
  state: string;
}

export function parseSubagentTag(text: string): SubagentRef | null {
  const match = /<subagent\b([^>]*?)\/?>[\s\S]*?(?:<\/subagent>|$)/i.exec(text);
  if (!match) return null;
  const attrs = match[1] ?? "";
  const attr = (name: string): string => new RegExp(`${name}="([^"]*)"`, "i").exec(attrs)?.[1] ?? "";
  const id = attr("id");
  if (!id) return null;
  return { id, agent: attr("agent"), description: attr("description"), state: attr("state") };
}

export function parseLegacyTaskText(text: string): SubagentRef | null {
  const agent = /(?:^|\n)\s*agent\s*=\s*([^\s]+)/i.exec(text)?.[1] ?? "";
  if (!agent) return null;
  const prompt = /(?:^|\n)\s*prompt\s*=\s*(.+)$/im.exec(text)?.[1]?.trim() ?? "";
  return { id: "", agent, description: prompt, state: "" };
}

function matchChildSession(
  sessions: SessionSummary[],
  parentID: string | undefined,
  criteria: { description?: string; agent?: string }
): string {
  if (!parentID) return "";
  let candidates = sessions.filter((candidate) => candidate.parentID === parentID);
  if (criteria.description) {
    const described = candidates.filter((candidate) => candidate.title.startsWith(criteria.description!));
    if (described.length > 0) candidates = described;
    else if (!criteria.agent) return "";
  }
  if (criteria.agent) {
    const key = criteria.agent.toLowerCase();
    const byAgent = candidates.filter((candidate) =>
      candidate.agent?.toLowerCase() === key || candidate.title.includes(`@${criteria.agent}`)
    );
    if (byAgent.length === 0) return "";
    candidates = byAgent;
  }
  return candidates.sort((a, b) => b.updatedAt - a.updatedAt)[0]?.id ?? "";
}

export function subagentChildID(ref: SubagentRef, sessions: SessionSummary[], parentID: string | undefined): string {
  if (ref.id) return ref.id;
  return matchChildSession(sessions, parentID, {
    ...(ref.agent ? { agent: ref.agent } : {}),
    ...(ref.description ? { description: ref.description.slice(0, 60) } : {})
  });
}

export interface TaskDescriptor {
  agent: string;
  description: string;
  dispatchDescription: string;
}

export function taskDescriptor(tool: ToolCallView): TaskDescriptor {
  const input = parseInput(tool.input);
  const agent = typeof input.agent === "string" ? input.agent
    : typeof input.subagent_type === "string" ? input.subagent_type : "";
  const description = typeof input.description === "string" && input.description ? input.description
    : typeof input.prompt === "string" ? input.prompt.trim() : "";
  const dispatchDescription = typeof input.description === "string" ? input.description
    : typeof input.prompt === "string" ? input.prompt : "";
  return { agent, description, dispatchDescription };
}

export function taskChildID(tool: ToolCallView, sessions: SessionSummary[], parentID: string | undefined,
  request: TaskDescriptor = taskDescriptor(tool)): string {
  const metadataSession = typeof tool.metadata?.sessionId === "string"
    ? tool.metadata.sessionId
    : typeof tool.metadata?.sessionID === "string" ? tool.metadata.sessionID : "";
  return metadataSession || matchChildSession(sessions, parentID, {
    ...(request.description ? { description: request.description } : {}),
    ...(request.agent ? { agent: request.agent } : {})
  });
}

function dispatchSignature(agent: string, description: string): string {
  const normalizedAgent = agent.trim().toLowerCase();
  const normalizedDescription = description.trim().toLowerCase().replace(/\s+/g, " ");
  return normalizedAgent || normalizedDescription ? `dispatch:${normalizedAgent}:${normalizedDescription}` : "";
}

export function taskDispatchSignature(tool: ToolCallView, request: TaskDescriptor = taskDescriptor(tool)): string {
  return dispatchSignature(request.agent, request.dispatchDescription);
}

export function refDispatchSignature(ref: SubagentRef): string {
  return dispatchSignature(ref.agent, ref.description);
}

function mergeSubagentTool(first: ToolCallView, latest: ToolCallView): ToolCallView {
  return {
    ...first,
    ...latest,
    id: first.id,
    detail: latest.detail || first.detail,
    input: latest.input || first.input,
    inputValue: latest.inputValue ?? first.inputValue,
    output: latest.output ?? first.output,
    paths: latest.paths?.length ? latest.paths : first.paths,
    metadata: { ...first.metadata, ...latest.metadata }
  };
}

export function consolidateSubagentTools(
  transcript: TranscriptItem[],
  sessions: SessionSummary[],
  parentID: string | undefined
): TranscriptItem[] {
  const result: TranscriptItem[] = [];
  const represented = new Map<string, { item: AssistantItem; partIndex: number }>();
  for (const entry of transcript) {
    if (entry.kind !== "assistant") {
      if (entry.kind === "user") represented.clear();
      if (entry.kind === "synthetic") {
        const ref = parseSubagentTag(entry.text) ?? parseLegacyTaskText(entry.text);
        if (ref) {
          const childID = subagentChildID(ref, sessions, parentID);
          const key = childID ? `child:${childID}` : refDispatchSignature(ref);
          const previous = represented.get(key);
          const existing = previous?.item.parts[previous.partIndex];
          if (previous && existing?.kind === "tool" && ref.state) {
            const settledState = ref.state.toLowerCase();
            const status = ["failed", "error", "cancelled"].includes(settledState)
              ? "failed"
              : ["complete", "completed", "success"].includes(settledState)
                ? "success"
                : existing.tool.status;
            previous.item.parts[previous.partIndex] = {
              ...existing,
              tool: { ...existing.tool, status }
            };
          }
        }
      }
      result.push(entry);
      continue;
    }
    const item: AssistantItem = { ...entry, parts: [] };
    for (const part of entry.parts) {
      if (part.kind !== "tool" || !["task", "subagent"].includes(toolKey(part.tool.title))) {
        item.parts.push(part);
        continue;
      }
      const request = taskDescriptor(part.tool);
      const childID = taskChildID(part.tool, sessions, parentID, request);
      const signature = taskDispatchSignature(part.tool, request);
      const key = childID ? `child:${childID}` : signature || `call:${part.tool.id}`;
      const previous = represented.get(key);
      if (!previous) {
        represented.set(key, { item, partIndex: item.parts.length });
        item.parts.push(part);
        continue;
      }
      const existing = previous.item.parts[previous.partIndex];
      if (existing?.kind !== "tool") continue;
      previous.item.parts[previous.partIndex] = {
        ...existing,
        tool: mergeSubagentTool(existing.tool, part.tool)
      };
    }
    result.push(item);
  }
  return result;
}

export function toolPresentation(tool: ToolCallView): { title: string; subtitle: string; path?: string } {
  const name = effectiveToolKey(tool);
  const input = toolInput(tool);
  const native = tool.metadata?.deepseek;
  const nativeRecord = native && typeof native === "object" && !Array.isArray(native) ? native as Record<string, unknown> : null;
  const rawView = nativeRecord?.resultView ?? nativeRecord?.callView;
  const view = rawView && typeof rawView === "object" && !Array.isArray(rawView) ? rawView as Record<string, unknown> : null;
  // OpenCode's `GenericTool` fallback names an unrecognized tool by its raw id.
  let title = `Called \`${tool.title}\``;
  let subtitle = tool.detail.replace(/^\$\s*/, "");
  let path: string | undefined;
  if (name === "read") {
    title = "Read";
    path = typeof input.filePath === "string" ? input.filePath : typeof input.file_path === "string" ? input.file_path : tool.paths?.[0];
    subtitle = fileName(path);
  } else if (name === "list") {
    title = "List";
    path = typeof input.path === "string" ? input.path : tool.paths?.[0];
    subtitle = fileName(path);
  } else if (name === "inspect") {
    title = "Inspect";
    path = typeof input.path === "string" ? input.path : tool.paths?.[0];
    subtitle = typeof path === "string" ? path : subtitle;
  } else if (name === "glob") {
    title = "Glob";
    subtitle = typeof input.pattern === "string" ? input.pattern : subtitle;
  } else if (name === "grep") {
    title = "Grep";
    subtitle = typeof input.pattern === "string" ? input.pattern : subtitle;
  } else if (name === "webfetch") {
    title = "Webfetch";
    subtitle = typeof input.url === "string" ? input.url : subtitle;
  } else if (name === "websearch" || name === "search") {
    title = "Web Search";
    subtitle = typeof input.query === "string" ? input.query : subtitle;
  } else if (name === "task") {
    title = "Task";
    subtitle = typeof input.description === "string" ? input.description : subtitle;
  } else if (name === "bash" || name === "shell") {
    title = "Shell";
    subtitle = typeof input.command === "string" ? input.command : subtitle;
  } else if (name === "edit") {
    title = "Edit";
    path = typeof input.filePath === "string" ? input.filePath : typeof input.file_path === "string" ? input.file_path : tool.paths?.[0];
    subtitle = fileName(path);
  } else if (name === "write") {
    title = "Write";
    path = typeof input.filePath === "string" ? input.filePath : typeof input.file_path === "string" ? input.file_path : tool.paths?.[0];
    subtitle = fileName(path);
  } else if (name === "patch" || name === "apply_patch") {
    title = "Patch";
  } else if (name === "question") {
    title = "Questions";
  }
  if (view?.card === "terminal") {
    title = "Shell";
    subtitle = typeof view.title === "string" ? view.title : subtitle;
  } else if (view?.card === "read") {
    title = "Read";
    path = typeof view.path === "string" ? view.path : path;
    subtitle = typeof view.title === "string" ? view.title : fileName(path);
  } else if (view?.card === "search" && typeof view.title === "string") {
    subtitle = view.title;
  } else if (view?.card === "web" && typeof view.url === "string") {
    subtitle = view.url;
  }
  return { title, subtitle, path };
}

export function formatDuration(duration: number): string {
  const seconds = Math.max(0, Math.floor(duration / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return remainder ? `${minutes}m ${remainder}s` : `${minutes}m`;
}

export function progressText(progress: string): string {
  const direct = progress.trim();
  if (!direct) return "";
  try {
    const value = JSON.parse(direct) as unknown;
    if (!value || typeof value !== "object" || Array.isArray(value)) return direct;
    const record = value as Record<string, unknown>;
    for (const key of ["message", "status", "detail", "phase", "title"]) {
      if (typeof record[key] === "string" && record[key]) return record[key];
    }
  } catch {
    return direct;
  }
  return direct;
}


export interface EditFileEntry {
  file: string;
  patch?: string;
  before?: string;
  after?: string;
  content?: string;
  status?: string;
  additions?: number;
  deletions?: number;
}

export function editFileEntries(tool: ToolCallView): EditFileEntry[] {
  const input = toolInput(tool);
  const path = typeof input.filePath === "string" ? input.filePath : typeof input.path === "string" ? input.path : tool.paths?.[0] ?? "";
  const candidates = Array.isArray(tool.metadata?.files) ? tool.metadata.files
    : tool.metadata?.filediff ? [tool.metadata.filediff] : [];
  const entries = candidates.flatMap((entry): EditFileEntry[] => {
    if (!entry || typeof entry !== "object") return [];
    const value = entry as Record<string, unknown>;
    const file = [value.relativePath, value.file, value.filePath, path].find((field): field is string => typeof field === "string" && !!field);
    if (!file) return [];
    return [{
      file,
      patch: typeof value.patch === "string" ? value.patch : typeof value.diff === "string" ? value.diff : undefined,
      before: typeof value.before === "string" ? value.before : undefined,
      after: typeof value.after === "string" ? value.after : undefined,
      status: typeof value.type === "string" ? value.type : typeof value.status === "string" ? value.status : undefined,
      additions: typeof value.additions === "number" ? value.additions : undefined,
      deletions: typeof value.deletions === "number" ? value.deletions : undefined
    }];
  });
  if (entries.length) return entries;
  const native = tool.metadata?.deepseek as { resultView?: { diffs?: unknown[] }; callView?: { diffs?: unknown[] } } | undefined;
  const diffs = native?.resultView?.diffs ?? native?.callView?.diffs;
  if (Array.isArray(diffs)) return diffs.flatMap((entry): EditFileEntry[] => {
    if (!entry || typeof entry !== "object") return [];
    const value = entry as Record<string, unknown>;
    if (typeof value.path !== "string" || typeof value.newText !== "string") return [];
    return [{ file: value.path, before: typeof value.oldText === "string" ? value.oldText : "", after: value.newText }];
  });
  return [{
    file: path,
    before: typeof input.oldString === "string" ? input.oldString : "",
    after: typeof input.newString === "string" ? input.newString : "",
    content: typeof input.content === "string" ? input.content : undefined
  }];
}

export function isEditCardTool(tool: ToolCallView): boolean {
  return ["edit", "write", "patch", "apply_patch"].includes(toolKey(tool.title));
}

export type RenderablePart = Exclude<AssistantPart, { kind: "reasoning" }>;
export function renderableAssistantPart(part: AssistantPart): part is RenderablePart {
  if (part.kind === "reasoning") return false;
  if (part.kind === "tool") return toolKey(part.tool.title) !== "todowrite" && !(part.tool.title === "question" && part.tool.status === "running");
  return Boolean(part.text.trim());
}

// Port of OpenCode's `reasoningHeading`/`cleanHeading` (timeline rows) so the thinking row
// surfaces the same heading a user would see in OpenCode.
function cleanHeading(value: string): string {
  return value
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/[*_~]+/g, "")
    .trim();
}

function reasoningHeading(text: string): string | undefined {
  const markdown = text.replace(/\r\n?/g, "\n");

  const html = markdown.match(/<h[1-6][^>]*>([\s\S]*?)<\/h[1-6]>/i);
  if (html?.[1]) {
    const value = cleanHeading(html[1].replace(/<[^>]+>/g, " "));
    if (value) return value;
  }

  const atx = markdown.match(/^\s{0,3}#{1,6}[ \t]+(.+?)(?:[ \t]+#+[ \t]*)?$/m);
  if (atx?.[1]) {
    const value = cleanHeading(atx[1]);
    if (value) return value;
  }

  const setext = markdown.match(/^([^\n]+)\n(?:=+|-+)\s*$/m);
  if (setext?.[1]) {
    const value = cleanHeading(setext[1]);
    if (value) return value;
  }

  const strong = markdown.match(/^\s*(?:\*\*|__)(.+?)(?:\*\*|__)\s*$/m);
  if (strong?.[1]) {
    const value = cleanHeading(strong[1]);
    if (value) return value;
  }

  return undefined;
}

export function latestReasoningHeading(items: AssistantItem[]): string | undefined {
  let latest: string | undefined;
  for (const item of items) {
    for (const part of item.parts) {
      if (part.kind !== "reasoning" || !part.text) continue;
      const heading = reasoningHeading(part.text);
      if (heading) latest = heading;
    }
  }
  return latest;
}

export type TimelineTurn = {
  id: string;
  user?: Extract<TranscriptItem, { kind: "user" }>;
  body: Exclude<VisibleTimelineItem, { kind: "user" }>[];
};

export function buildTurns(timeline: VisibleTimelineItem[]): TimelineTurn[] {
  const turns: TimelineTurn[] = [];
  let current: TimelineTurn | undefined;
  for (const item of timeline) {
    if (item.kind === "user") {
      current = { id: item.id, user: item, body: [] };
      turns.push(current);
      continue;
    }
    if (!current) {
      current = { id: item.id, body: [] };
      turns.push(current);
    }
    current.body.push(item);
  }
  return turns;
}


function representedSubagents(
  transcript: TranscriptItem[],
  sessions: SessionSummary[],
  sessionID: string | undefined
): Set<string> {
  const ids = new Set<string>();
  let turn = 0;
  for (const item of transcript) {
    if (item.kind === "user") {
      turn += 1;
      continue;
    }
    if (item.kind !== "assistant") continue;
    for (const part of item.parts) {
      if (part.kind !== "tool" || !["task", "subagent"].includes(toolKey(part.tool.title))) continue;
      const request = taskDescriptor(part.tool);
      const childID = taskChildID(part.tool, sessions, sessionID, request);
      if (childID) ids.add(`${turn}:child:${childID}`);
      const signature = taskDispatchSignature(part.tool, request);
      if (signature) ids.add(`${turn}:${signature}`);
    }
  }
  return ids;
}

/** Rows that belong in the conversation surface, with subagent echoes folded in. */
export function visibleTranscript(
  transcript: TranscriptItem[],
  sessions: SessionSummary[],
  sessionID: string | undefined
): VisibleTimelineItem[] {
  const consolidated = consolidateSubagentTools(transcript, sessions, sessionID);
  const represented = representedSubagents(consolidated, sessions, sessionID);
  const visible: VisibleTimelineItem[] = [];
  let turn = 0;
  for (const item of consolidated) {
    if (item.kind === "user") turn += 1;
    if (item.kind === "permission" || item.kind === "pending-input" || item.kind === "selection" || item.kind === "system") {
      continue;
    }
    if (item.kind === "synthetic") {
      const ref = parseSubagentTag(item.text) ?? parseLegacyTaskText(item.text);
      const childID = ref ? subagentChildID(ref, sessions, sessionID) : "";
      if (childID && represented.has(`${turn}:child:${childID}`)) continue;
      if (ref && represented.has(`${turn}:${refDispatchSignature(ref)}`)) continue;
      if (isInternalSystemReminder(item)) continue;
    }
    visible.push(item);
  }
  return visible;
}
