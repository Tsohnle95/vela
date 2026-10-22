import { memo } from "react";
import { MarkdownText } from "@deepseek-ai/dsh-client-ui-primitives";
import { MARKDOWN_LABELS } from "./labels";

/**
 * DeepSeek Harness' exact streaming Markdown renderer. While `streaming` is
 * true it freezes every top-level block except the trailing two and re-parses
 * only the source tail, so appending a chunk never remounts prior prose.
 */
export const DshMarkdown = memo(function DshMarkdown({
  text,
  streaming = false,
  variant = "body"
}: {
  text: string;
  streaming?: boolean;
  variant?: "body" | "compact";
}) {
  return (
    <MarkdownText
      text={text}
      streaming={streaming}
      labels={MARKDOWN_LABELS}
      variant={variant}
    />
  );
});
