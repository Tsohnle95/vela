import { useState } from "react";
import { CodexMarkdown } from "./CodexMarkdown";

/**
 * A Codex reasoning block: collapsed to a single secondary row once the agent
 * moves on, open while it is still thinking.
 */
export function CodexReasoning({ text, running }: { text: string; running: boolean }) {
  const [manualOpen, setManualOpen] = useState<boolean | null>(null);
  const open = manualOpen ?? running;
  return (
    <div className="codex-reasoning" data-running={running || undefined}>
      <button
        type="button"
        className="codex-reasoning-head"
        aria-expanded={open}
        onClick={() => setManualOpen(!open)}
      >
        <svg className="codex-reasoning-glyph" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M6.2 2.6a3.2 3.2 0 0 1 4.3 3c0 2.1-1.6 2.6-1.6 4.4H6.5c0-1.8-1.6-2.3-1.6-4.4 0-.9.4-1.7 1.3-3zM6.4 12h2.4M6.8 14h1.6" />
        </svg>
        <span className="codex-reasoning-label">{running ? "Thinking" : "Reasoning"}</span>
        {running && <span className="codex-tool-spinner" aria-hidden="true" />}
        <svg className="codex-tool-chevron" viewBox="0 0 16 16" aria-hidden="true" data-open={open || undefined}>
          <path d="M6 4l4 4-4 4" />
        </svg>
      </button>
      {open && (
        <div className="codex-reasoning-body">
          <CodexMarkdown text={text} streaming={running} className="codex-md-reasoning" />
        </div>
      )}
    </div>
  );
}
