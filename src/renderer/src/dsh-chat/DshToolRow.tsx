import { useMemo, useState, type ReactNode } from "react";
import {
  DiffBlock,
  DisclosureRow,
  IconAgentPresetOutlineRegular,
  IconApiOutlineRegular,
  IconBrowseOutlineRegular,
  IconCodeOutlineRegular,
  IconEditOutlineRegular,
  IconSearchOutlineRegular,
  IconSparkleRegular,
  TerminalBlock,
  TextShimmer,
  diffTotals
} from "@deepseek-ai/dsh-client-ui-primitives";
import type { SessionInfo, ToolCallView } from "@shared/types";
import stripAnsi from "strip-ansi";
import { useStore } from "../store";
import {
  editFileEntries,
  effectiveToolKey,
  isEditCardTool,
  taskChildID,
  taskDescriptor,
  toolInput,
  toolPresentation,
  workspaceFilePath
} from "../components/timeline-model";
import { DIFF_LABELS, TERMINAL_LABELS } from "./labels";

type RowVariant = "search" | "read" | "bash" | "write" | "edit" | "code" | "subagent" | "others";
type ToolState = "running" | "ok" | "error";
type DiffHunk = { path: string; oldText: string | null; newText: string };

const VARIANT_ICONS: Record<RowVariant, ReactNode> = {
  search: <IconSearchOutlineRegular size={14} />,
  read: <IconBrowseOutlineRegular size={14} />,
  bash: <IconApiOutlineRegular size={14} />,
  write: <IconEditOutlineRegular size={14} />,
  edit: <IconEditOutlineRegular size={14} />,
  code: <IconCodeOutlineRegular size={14} />,
  subagent: <IconAgentPresetOutlineRegular size={14} />,
  others: <IconSparkleRegular size={14} />
};

const VARIANT_BY_KEY: Record<string, RowVariant> = {
  read: "read",
  webfetch: "read",
  web_fetch: "read",
  grep: "search",
  glob: "search",
  list: "search",
  websearch: "search",
  web_search: "search",
  write: "write",
  edit: "edit",
  patch: "edit",
  apply_patch: "edit",
  bash: "bash",
  shell: "bash",
  terminal: "bash",
  run_code: "code",
  code: "code",
  task: "subagent",
  subagent: "subagent"
};

function rowVariant(tool: ToolCallView): RowVariant {
  const key = effectiveToolKey(tool);
  return VARIANT_BY_KEY[key] ?? (key.startsWith("terminal_") ? "bash" : "others");
}

function rowState(tool: ToolCallView): ToolState {
  return tool.status === "running" ? "running" : tool.status === "failed" ? "error" : "ok";
}

function oneLine(value: string): string {
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized.length > 240 ? `${normalized.slice(0, 239)}…` : normalized;
}

function inputText(tool: ToolCallView): string {
  if (tool.inputValue === undefined) return tool.input ?? "";
  if (typeof tool.inputValue === "string") return tool.inputValue;
  try {
    return JSON.stringify(tool.inputValue, null, 2);
  } catch {
    return String(tool.inputValue);
  }
}

function nativeSubCalls(tool: ToolCallView): ToolCallView[] {
  const native = tool.metadata?.deepseek;
  const record = native && typeof native === "object" && !Array.isArray(native) ? native as Record<string, unknown> : null;
  if (!Array.isArray(record?.subCalls)) return [];
  return record.subCalls.flatMap((item): ToolCallView[] => {
    if (!item || typeof item !== "object" || Array.isArray(item)) return [];
    const value = item as Record<string, unknown>;
    if (typeof value.id !== "string" || typeof value.title !== "string") return [];
    const status = value.status === "success" || value.status === "failed" ? value.status : "running";
    return [{
      id: value.id,
      title: value.title,
      detail: typeof value.detail === "string" ? value.detail : "",
      status,
      ...(typeof value.input === "string" ? { input: value.input } : {}),
      ...(value.inputValue !== undefined ? { inputValue: value.inputValue } : {}),
      ...(typeof value.output === "string" ? { output: value.output } : {}),
      ...(typeof value.startedAt === "number" ? { startedAt: value.startedAt } : {})
    }];
  });
}

function Summary({
  summary,
  title,
  filePath,
  target,
  totals,
  state,
  onOpen
}: {
  summary: string;
  title: string;
  filePath: string | undefined;
  target: boolean;
  totals: { added: number; removed: number } | null;
  state: ToolState;
  onOpen: (path: string) => void;
}) {
  if (summary === "" && totals === null && filePath === undefined) return null;
  return (
    <>
      <span className="dsh-tool-separator" data-shimmer-decoration aria-hidden />
      {filePath && target ? (
        <button
          type="button"
          className="dsh-tool-summary dsh-tool-file-link"
          onClick={(event) => {
            event.stopPropagation();
            onOpen(filePath);
          }}
        >
          <TextShimmer>{summary || filePath}</TextShimmer>
        </button>
      ) : (
        <span className="dsh-tool-summary" data-error={state === "error" || undefined}>
          <TextShimmer>{summary || title}</TextShimmer>
        </span>
      )}
      {totals !== null && (
        <span className="dsh-tool-diff-stat">
          <span className="dsh-diff-added">{`+${totals.added}`}</span>
          {" "}
          <span className="dsh-diff-removed">{`-${totals.removed}`}</span>
        </span>
      )}
    </>
  );
}

function ToolBody({
  tool,
  variant,
  state,
  session,
  childSession,
  files,
  diffs,
  patch,
  input,
  output,
  subCalls,
  onOpen
}: {
  tool: ToolCallView;
  variant: RowVariant;
  state: ToolState;
  session: SessionInfo | null;
  childSession: string;
  files: ReturnType<typeof editFileEntries>;
  diffs: DiffHunk[];
  patch: string | undefined;
  input: string;
  output: string;
  subCalls: ToolCallView[];
  onOpen: (path: string) => void;
}) {
  const { reopenSession } = useStore();
  const request = variant === "subagent" ? taskDescriptor(tool) : null;
  const command = typeof toolInput(tool).command === "string" ? String(toolInput(tool).command) : tool.detail;
  return (
    <div className="dsh-tool-body">
      {request && (
        <div className="dsh-tool-task">
          {request.description && <p className="dsh-tool-task-description">{request.description}</p>}
          {childSession && (
            <button type="button" className="dsh-tool-action" onClick={() => void reopenSession(childSession)}>
              Open agent
            </button>
          )}
        </div>
      )}
      {variant === "bash" ? (
        <TerminalBlock
          command={command}
          cwd={session?.directory}
          output={output || undefined}
          running={state === "running"}
          labels={TERMINAL_LABELS}
        />
      ) : diffs.length > 0 ? (
        <DiffBlock diffs={diffs} labels={DIFF_LABELS} />
      ) : patch ? (
        <pre className="dsh-tool-patch">{patch}</pre>
      ) : (
        <GenericBody
          files={files}
          input={input}
          output={output}
          state={state}
          onOpen={onOpen}
        />
      )}
      {subCalls.length > 0 && (
        <div className="dsh-tool-subcalls">
          {subCalls.map((subCall) => <DshToolRow key={subCall.id} tool={subCall} session={session} />)}
        </div>
      )}
    </div>
  );
}

function GenericBody({
  files,
  input,
  output,
  state,
  onOpen
}: {
  files: ReturnType<typeof editFileEntries>;
  input: string;
  output: string;
  state: ToolState;
  onOpen: (path: string) => void;
}) {
  return (
    <>
      {files.length > 0 && (
        <div className="dsh-tool-files">
          {files.map((file) => (
            <button key={file.file} type="button" className="dsh-tool-file" onClick={() => onOpen(file.file)}>
              <span className="dsh-tool-file-path">{file.file}</span>
              {file.additions !== undefined && (
                <span className="dsh-tool-diff-stat">
                  <span className="dsh-diff-added">{`+${file.additions}`}</span>
                  {" "}
                  <span className="dsh-diff-removed">{`-${file.deletions ?? 0}`}</span>
                </span>
              )}
            </button>
          ))}
        </div>
      )}
      {(input || output) && (
        <div className="dsh-io-card">
          {input && (
            <div className="dsh-io-section">
              <span className="dsh-io-label">IN</span>
              <pre className="dsh-io-text">{input}</pre>
            </div>
          )}
          {input && output && <span className="dsh-io-divider" />}
          {output && (
            <div className="dsh-io-section">
              <span className="dsh-io-label">OUT</span>
              <pre className="dsh-io-text" data-error={state === "error" || undefined}>{output}</pre>
            </div>
          )}
        </div>
      )}
    </>
  );
}

/**
 * One quiet 24px tool/process row. This is the same visual grammar as DSH's
 * ToolRow: icon, title, dot separator, truncating summary, hover chevron, and
 * an expanded body rendered lazily. It intentionally has no entrance motion.
 */
export function DshToolRow({
  tool,
  session
}: {
  tool: ToolCallView;
  session: SessionInfo | null;
}) {
  const { openFile, sessions } = useStore();
  const [expanded, setExpanded] = useState(false);
  const presentation = useMemo(() => toolPresentation(tool), [tool]);
  const variant = rowVariant(tool);
  const state = rowState(tool);
  const name = effectiveToolKey(tool);
  const childSession = variant === "subagent"
    ? taskChildID(tool, sessions, session?.id, taskDescriptor(tool))
    : "";
  const files = useMemo(() => (isEditCardTool(tool) ? editFileEntries(tool) : []), [tool]);
  const input = useMemo(() => inputText(tool), [tool]);
  const output = useMemo(() => stripAnsi(tool.output ?? "").replace(/\r\n?/g, "\n"), [tool.output]);
  const subCalls = useMemo(() => nativeSubCalls(tool), [tool]);
  const diffs = useMemo(() => files.flatMap((file) => {
    if (file.after === undefined && file.content === undefined) return [];
    return [{ path: file.file, oldText: file.before ?? "", newText: file.after ?? file.content ?? "" }];
  }), [files]);
  const totals = useMemo(() => (diffs.length > 0 ? diffTotals(diffs) : null), [diffs]);
  const patch = files.find((file) => file.patch)?.patch;
  const summary = oneLine(presentation.subtitle || (variant === "bash" ? input : ""));
  const filePath = presentation.path;
  const target = Boolean(filePath && workspaceFilePath(filePath, session));
  const expandable = Boolean(input || output || files.length > 0 || subCalls.length > 0 || variant === "subagent");
  const openPath = (path: string): void => {
    const resolved = workspaceFilePath(path, session);
    if (resolved) void openFile(resolved, session?.workspace);
  };

  return (
    <div className="dsh-tool" data-tool={name} data-state={state} data-timeline-part-id={tool.id}>
      <DisclosureRow
        rowClassName="dsh-tool-row"
        leadingClassName="dsh-tool-leading"
        titleClassName="dsh-tool-title"
        icon={VARIANT_ICONS[variant]}
        title={presentation.title}
        running={state === "running"}
        open={expanded}
        expandable={expandable}
        expandOnRowClick
        onToggle={() => setExpanded((value) => !value)}
        collapsedContent={(
          <Summary
            summary={summary}
            title={presentation.title}
            filePath={filePath}
            target={target}
            totals={totals}
            state={state}
            onOpen={openPath}
          />
        )}
      >
        {expanded && (
          <ToolBody
            tool={tool}
            variant={variant}
            state={state}
            session={session}
            childSession={childSession}
            files={files}
            diffs={diffs}
            patch={patch}
            input={input}
            output={output}
            subCalls={subCalls}
            onOpen={openPath}
          />
        )}
      </DisclosureRow>
    </div>
  );
}
