import { useState } from "react";
import type { ToolCallView } from "@shared/types";
import { toolKey } from "../components/timeline-model";

type RowKind = "exec" | "read" | "search" | "list" | "patch" | "web" | "generic";

const EXEC = new Set(["bash", "shell", "terminal", "run_code", "code"]);
const READ = new Set(["read", "read_image"]);
const SEARCH = new Set(["grep", "search_code"]);
const LIST = new Set(["glob", "list", "list_files"]);
const PATCH = new Set(["write", "edit", "patch", "apply_patch"]);
const WEB = new Set(["websearch", "web_search", "search", "webfetch", "web_fetch"]);

function rowKind(key: string): RowKind {
  if (EXEC.has(key)) return "exec";
  if (READ.has(key)) return "read";
  if (SEARCH.has(key)) return "search";
  if (LIST.has(key)) return "list";
  if (PATCH.has(key)) return "patch";
  if (WEB.has(key)) return "web";
  return "generic";
}

function field(tool: ToolCallView, names: string[]): string | undefined {
  const values = tool.inputValue as Record<string, unknown> | undefined;
  for (const name of names) {
    const value = values?.[name];
    if (typeof value === "string" && value.trim()) return value;
  }
  return undefined;
}

function commandOf(tool: ToolCallView): string {
  const command = field(tool, ["command", "cmd"]) ?? tool.detail ?? "";
  return command.replace(/^\$\s*/, "").trim();
}

function pathOf(tool: ToolCallView): string {
  return field(tool, ["filePath", "file_path", "path"]) ?? tool.detail ?? "";
}

function queryOf(tool: ToolCallView): string {
  return field(tool, ["query", "pattern"]) ?? tool.detail ?? "";
}

function verb(kind: RowKind, status: ToolCallView["status"]): string {
  const running = status === "running";
  const failed = status === "failed";
  if (kind === "exec") return running ? "Running" : failed ? "Stopped" : "Ran";
  if (kind === "patch") return running ? "Editing" : failed ? "Rejected" : "Edited";
  return "";
}

function summaryFor(tool: ToolCallView, kind: RowKind): string {
  const command = commandOf(tool);
  switch (kind) {
    case "exec":
      return command ? `${verb(kind, tool.status)} ${command}` : `${verb(kind, tool.status)} command`;
    case "read":
      return `Read ${pathOf(tool)}`.trim();
    case "search":
      return queryOf(tool) ? `Searched for ${queryOf(tool)}` : "Searched for files";
    case "list":
      return pathOf(tool) ? `Listed files in ${pathOf(tool)}` : "Listed files";
    case "patch":
      return `${verb(kind, tool.status)} ${pathOf(tool)}`.trim();
    case "web":
      return tool.detail?.trim() ? `Searched the web for ${tool.detail.trim()}` : "Searched the web";
    default:
      return tool.detail?.trim() ? `${tool.title} ${tool.detail.trim()}` : tool.title;
  }
}

function StatusFooter({ status }: { status: ToolCallView["status"] }) {
  if (status === "running") return null;
  return (
    <div className="vela-exec-footer">
      <span className="vela-exec-status">
        {status === "failed" ? (
          "Failed"
        ) : (
          <>
            <svg className="vela-exec-check" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3.5 8.5l3 3 6-7" />
            </svg>
            Success
          </>
        )}
      </span>
    </div>
  );
}

function DiffBody({ text }: { text: string }) {
  return (
    <pre className="vela-diff">
      {text.split("\n").map((line, index) => (
        <div
          key={index}
          data-kind={
            line.startsWith("@@")
              ? "hunk"
              : line.startsWith("+") && !line.startsWith("+++")
                ? "add"
                : line.startsWith("-") && !line.startsWith("---")
                  ? "del"
                  : undefined
          }
        >
          {line || " "}
        </div>
      ))}
    </pre>
  );
}

function ExecBody({ tool }: { tool: ToolCallView }) {
  const output = tool.output ?? "";
  return (
    <div className="vela-exec-card" data-testid="exec-shell-body">
      <div className="vela-exec-output">
        <pre>{output || "No output"}</pre>
      </div>
      <StatusFooter status={tool.status} />
    </div>
  );
}

function PatchBody({ tool }: { tool: ToolCallView }) {
  const body = tool.output ?? "";
  const looksLikeDiff = /(^|\n)(@@ |\+\+\+ |--- )/.test(body);
  return (
    <div className="vela-patch-card">
      {looksLikeDiff ? <DiffBody text={body} /> : <pre className="vela-patch-output">{body || "No changes"}</pre>}
    </div>
  );
}

/** One activity row: collapsed summary with an expandable payload. */
export function VelaToolRow({ tool }: { tool: ToolCallView }) {
  const key = toolKey(tool.title);
  const kind = rowKind(key);
  const [open, setOpen] = useState(false);
  const running = tool.status === "running";
  const body = kind === "exec" || kind === "patch" ? tool.output ?? "" : "";
  const expandable = kind === "exec" || kind === "patch" || (kind === "generic" && Boolean(body));
  return (
    <div className="vela-tool" data-kind={kind} data-status={tool.status}>
      <button
        type="button"
        className="vela-tool-head"
        aria-expanded={expandable ? open : undefined}
        data-expandable={expandable || undefined}
        onClick={() => {
          if (expandable) setOpen((value) => !value);
        }}
      >
        <span className="vela-tool-summary" data-running={running || undefined}>
          {summaryFor(tool, kind)}
        </span>
        {expandable && (
          <svg className="vela-tool-chevron" viewBox="0 0 16 16" aria-hidden="true" data-open={open || undefined}>
            <path d="M6 4l4 4-4 4" />
          </svg>
        )}
      </button>
      {open && expandable && (
        <div className="vela-tool-body">
          {kind === "patch" ? <PatchBody tool={tool} /> : <ExecBody tool={tool} />}
        </div>
      )}
    </div>
  );
}
