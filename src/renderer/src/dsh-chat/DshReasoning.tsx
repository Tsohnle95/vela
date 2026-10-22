import { useEffect, useMemo, useState } from "react";
import {
  DisclosureRow,
  IconThinkOutlineRegular,
  TextShimmer
} from "@deepseek-ai/dsh-client-ui-primitives";
import { DshMarkdown } from "./DshMarkdown";

function firstLine(text: string): string {
  const newline = text.indexOf("\n");
  return newline === -1 ? text : text.slice(0, newline);
}

/** Latest completed paragraph's first line, ported from DSH's ReasoningRow. */
function latestCompletedParagraphFirstLine(text: string): string {
  let summary = "";
  let paragraphStart = 0;
  const separator = /\r?\n(?:[\t ]*\r?\n)+/g;
  while (true) {
    const nextParagraph = separator.exec(text);
    const paragraphEnd = nextParagraph === null ? text.length : nextParagraph.index + nextParagraph[0].indexOf("\n");
    const newline = text.indexOf("\n", paragraphStart);
    if (newline !== -1 && newline <= paragraphEnd) {
      const candidate = text.slice(paragraphStart, newline).trim();
      if (candidate !== "") summary = candidate;
    }
    if (nextParagraph === null) return summary;
    paragraphStart = nextParagraph.index + nextParagraph[0].length;
  }
}

const THINK_ICON = <IconThinkOutlineRegular size={14} />;

/** DSH's collapsed reasoning disclosure with its streaming first-line preview. */
export function DshReasoning({
  text,
  running,
  resetKey
}: {
  text: string;
  running: boolean;
  resetKey: string;
}) {
  const [expanded, setExpanded] = useState(false);
  useEffect(() => {
    setExpanded(false);
  }, [resetKey]);

  const summaryText = running ? latestCompletedParagraphFirstLine(text) : firstLine(text);
  const summary = useMemo(() => summaryText.replaceAll("**", ""), [summaryText]);
  const preview = !expanded && summary !== "";
  const collapsedContent = (
    <>
      <span className="dsh-reasoning-separator" data-shimmer-decoration aria-hidden />
      <span className="dsh-reasoning-summary" data-streaming={running || undefined}>
        <span className="dsh-reasoning-summary-text">
          <TextShimmer>{summary}</TextShimmer>
        </span>
      </span>
    </>
  );

  return (
    <div
      className="dsh-reasoning"
      data-variant="think"
      data-state={running ? "running" : "ok"}
      data-expanded={expanded || undefined}
      data-preview={preview || undefined}
    >
      {running && <span className="dsh-visually-hidden">Running</span>}
      <DisclosureRow
        rowClassName="dsh-reasoning-row"
        leadingClassName="dsh-reasoning-leading"
        titleClassName="dsh-reasoning-title"
        icon={THINK_ICON}
        title="Think"
        running={running}
        open={expanded}
        expandable
        expandOnRowClick
        onToggle={() => setExpanded((value) => !value)}
        collapsedContent={collapsedContent}
      >
        {expanded && (
          <div className="dsh-reasoning-body">
            <DshMarkdown text={text} streaming={running} variant="compact" />
          </div>
        )}
      </DisclosureRow>
    </div>
  );
}
