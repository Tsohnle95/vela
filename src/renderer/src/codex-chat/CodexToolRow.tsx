import { useState, type ReactNode } from "react";
import type { ToolCallView } from "@shared/types";
import { toolKey } from "../components/timeline-model";
import { activityOfTool, type ProcessActivity } from "../dsh-chat/process-model";

function toolGlyph(activity: ProcessActivity): ReactNode {
  switch (activity) {
    case "read":
    case "readImage":
      return <path d="M3 2.5h6.5L12 5v8.5H3zM9 2.5V5h3" />;
    case "edit":
    case "write":
      return <path d="M2.5 11.5l1-3 7-7 2 2-7 7zM9.5 2.5l2 2" />;
    case "search":
      return <path d="M7 2a5 5 0 1 0 0 10A5 5 0 0 0 7 2zM10.5 10.5L14 14" />;
    case "webSearch":
    case "webFetch":
      return <path d="M8 2a6 6 0 1 0 0 12A6 6 0 0 0 8 2zM2 8h12M8 2c2 2 2 10 0 12M8 2c-2 2-2 10 0 12" />;
    case "subagents":
      return <path d="M4 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM12 2a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM4 10a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM5.5 6.5l4.5-2M5.5 9.5l4.5 2" />;
    case "plan":
      return <path d="M3 3h10M3 8h10M3 13h6" />;
    case "commands":
      return <path d="M2.5 3.5h11v9h-11zM5 7l2 1.5-2 1.5M8.5 10h2.5" />;
    case "code":
      return <path d="M5.5 4.5L2.5 8l3 3.5M10.5 4.5l3 3.5-3 3.5" />;
    case "questions":
      return <path d="M5.5 6a2.5 2.5 0 1 1 3.4 2.3c-.6.3-.9.7-.9 1.4M8 12.4v.1" />;
    default:
      return <path d="M8 2l1.4 4.6L14 8l-4.6 1.4L8 14l-1.4-4.6L2 8l4.6-1.4z" />;
  }
}

function statusGlyph(tool: ToolCallView): ReactNode {
  if (tool.status === "running") return <span className="codex-tool-spinner" aria-hidden="true" />;
  if (tool.status === "failed") {
    return (
      <svg className="codex-tool-status" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" />
      </svg>
    );
  }
  return (
    <svg className="codex-tool-status" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3.5 8.5l3 3 6-7" />
    </svg>
  );
}

/** One Codex-style tool call: a compact row that expands to its raw payload. */
export function CodexToolRow({ tool }: { tool: ToolCallView }) {
  const [open, setOpen] = useState(false);
  const kind = toolKey(tool.title);
  const activity = activityOfTool(tool);
  const detail = tool.detail?.replace(/\s+/g, " ").trim() ?? "";
  const body = tool.output ?? "";
  const expandable = body.trim().length > 0;
  return (
    <div className="codex-tool" data-status={tool.status} data-kind={kind}>
      <button
        type="button"
        className="codex-tool-head"
        aria-expanded={expandable ? open : undefined}
        onClick={() => {
          if (expandable) setOpen((value) => !value);
        }}
        data-expandable={expandable || undefined}
      >
        <span className="codex-tool-leading" aria-hidden="true">
          <svg className="codex-tool-glyph" viewBox="0 0 16 16">{toolGlyph(activity)}</svg>
          {statusGlyph(tool)}
        </span>
        <span className="codex-tool-title">{tool.title}</span>
        {detail && <span className="codex-tool-detail">{detail}</span>}
        {expandable && (
          <svg className="codex-tool-chevron" viewBox="0 0 16 16" aria-hidden="true" data-open={open || undefined}>
            <path d="M6 4l4 4-4 4" />
          </svg>
        )}
      </button>
      {open && expandable && (
        <pre className="codex-tool-body">
          <code>{body.length > 8000 ? `${body.slice(0, 8000)}\n…` : body}</code>
        </pre>
      )}
    </div>
  );
}
