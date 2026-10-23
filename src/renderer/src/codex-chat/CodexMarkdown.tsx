import { memo, useEffect, useId, useLayoutEffect, useMemo, useRef, useSyncExternalStore, type CSSProperties, type ReactNode } from "react";
import { Marked } from "marked";
import {
  attachSegment,
  beginSegmentPass,
  createFadeTimeline,
  FADE_SEGMENT_DELAY_MS,
  FADE_SEGMENT_MAX_DELAY_MS,
  resolveSegmentKey,
  segmentDelay,
  splitSegments,
  type FadeTimeline
} from "./segment-fade";
import { StreamingTextBuffer } from "./streaming-buffer";
import { CodexCodeBlock } from "./CodexCodeBlock";

/**
 * Codex's streamed Markdown surface. Codex re-renders the whole message while
 * tokens arrive and makes arrival legible with the segment fade above; it does
 * not freeze or re-parse the tail, so prose never jumps between blocks.
 */

const codexMarked = new Marked({ breaks: true, gfm: true });

interface MdToken {
  type: string;
  raw?: string;
  text?: string;
  tokens?: MdToken[];
  items?: MdToken[];
  header?: MdToken[];
  rows?: MdToken[][];
  lang?: string;
  href?: string;
  title?: string | null;
  depth?: number;
  ordered?: boolean;
  start?: number;
  task?: boolean;
  checked?: boolean | null;
}

interface RenderContext {
  animate: boolean;
  adaptive: boolean;
  timeline: FadeTimeline;
  cursor: number;
  tone: "assistant" | "user";
}

function subscribeVisibility(listener: () => void): () => void {
  document.addEventListener("visibilitychange", listener);
  return () => document.removeEventListener("visibilitychange", listener);
}

function visibilitySnapshot(): boolean {
  return document.visibilityState !== "hidden";
}

function SegmentedText({ text, context }: { text: string; context: RenderContext }) {
  const segments = useMemo(() => splitSegments(text), [text]);
  return (
    <>
      {segments.map((segment) => {
        const index = context.cursor;
        context.cursor += 1;
        const segmentKey = resolveSegmentKey(context.timeline, index, segment);
        return context.animate
          ? <FadeSegment key={segmentKey} segmentKey={segmentKey} index={index} context={context} text={segment} />
          : <span key={segmentKey}>{segment}</span>;
      })}
    </>
  );
}

function FadeSegment({
  segmentKey,
  index,
  context,
  text
}: {
  segmentKey: string;
  index: number;
  context: RenderContext;
  text: string;
}) {
  const settled = context.timeline.settledKeys.has(segmentKey);
  if (settled) return <span>{text}</span>;
  const settle = (): void => {
    context.timeline.settledKeys.add(segmentKey);
  };
  if (context.adaptive) {
    return <span className="codex-fade" onAnimationEnd={settle}>{text}</span>;
  }
  const delay = context.timeline.delayMsByKey.get(segmentKey)
    ?? segmentDelay(index, FADE_SEGMENT_DELAY_MS, FADE_SEGMENT_MAX_DELAY_MS);
  const attach = (element: HTMLSpanElement | null): void => {
    if (!element) return;
    attachSegment({
      element,
      fadeSegmentDelayMs: FADE_SEGMENT_DELAY_MS,
      fadeSegmentMaxDelayMs: FADE_SEGMENT_MAX_DELAY_MS,
      segmentKey,
      timeline: context.timeline
    });
  };
  return (
    <span
      ref={attach}
      className="codex-fade"
      style={{ "--fade-delay": `${delay}ms` } as CSSProperties}
      onAnimationEnd={settle}
    >
      {text}
    </span>
  );
}

const WRAPPED_INLINE: Record<string, "strong" | "em" | "del"> = {
  strong: "strong",
  em: "em",
  del: "del"
};

function textOf(token: MdToken): string {
  return token.text ?? token.raw ?? "";
}

function renderImageToken(token: MdToken, path: string): ReactNode {
  return (
    <img
      key={path}
      className="codex-md-image"
      src={token.href}
      alt={token.text ?? ""}
      loading="lazy"
      decoding="async"
      referrerPolicy="no-referrer"
    />
  );
}

function renderLinkToken(token: MdToken, path: string, context: RenderContext): ReactNode {
  return (
    <a key={path} href={token.href ?? "#"} target="_blank" rel="noopener noreferrer">
      <InlineTokens tokens={token.tokens ?? []} path={path} context={context} />
    </a>
  );
}

function renderInlineToken(token: MdToken, path: string, context: RenderContext): ReactNode {
  const type = token.type;
  if (type === "text" || type === "escape" || type === "html") {
    return <SegmentedText key={path} text={textOf(token)} context={context} />;
  }
  const wrapper = WRAPPED_INLINE[type];
  if (wrapper !== undefined) {
    const Tag = wrapper;
    return <Tag key={path}><InlineTokens tokens={token.tokens ?? []} path={path} context={context} /></Tag>;
  }
  if (type === "codespan") {
    return <code key={path} className="codex-md-codespan">{textOf(token)}</code>;
  }
  if (type === "image") return renderImageToken(token, path);
  if (type === "link") return renderLinkToken(token, path, context);
  if (type === "br") return <br key={path} />;
  if (token.tokens && token.tokens.length > 0) {
    return <InlineTokens key={path} tokens={token.tokens} path={path} context={context} />;
  }
  return <SegmentedText key={path} text={textOf(token)} context={context} />;
}

function InlineTokens({
  tokens,
  path,
  context
}: {
  tokens: MdToken[];
  path: string;
  context: RenderContext;
}) {
  return <>{tokens.map((token, index) => renderInlineToken(token, `${path}.i${index}`, context))}</>;
}

function Inline({ token, path, context }: { token: MdToken; path: string; context: RenderContext }) {
  if (token.tokens && token.tokens.length > 0) {
    return <InlineTokens tokens={token.tokens} path={path} context={context} />;
  }
  return <SegmentedText text={token.text ?? ""} context={context} />;
}

function renderHeading(token: MdToken, path: string, context: RenderContext): ReactNode {
  const depth = Math.min(Math.max(token.depth ?? 1, 1), 6);
  const Tag = `h${depth}` as "h1";
  return (
    <Tag key={path} className="codex-md-heading">
      <Inline token={token} path={path} context={context} />
    </Tag>
  );
}

function renderParagraph(token: MdToken, path: string, context: RenderContext): ReactNode {
  return (
    <p key={path} className="codex-md-paragraph">
      <Inline token={token} path={path} context={context} />
    </p>
  );
}

function renderCode(token: MdToken, path: string, context: RenderContext): ReactNode {
  if (context.tone === "user") {
    return (
      <pre key={path} className="codex-md-pre">
        <code className={token.lang ? `language-${token.lang}` : undefined}>{token.text ?? ""}</code>
      </pre>
    );
  }
  return (
    <div key={path} className="codex-md-code-block">
      <CodexCodeBlock code={token.text ?? ""} lang={token.lang} streaming={context.animate} />
    </div>
  );
}

function renderBlockquote(token: MdToken, path: string, context: RenderContext): ReactNode {
  return (
    <blockquote key={path} className="codex-md-blockquote">
      {renderBlocks(token.tokens ?? [], path, context)}
    </blockquote>
  );
}

function renderList(token: MdToken, path: string, context: RenderContext): ReactNode {
  const Tag = token.ordered ? "ol" : "ul";
  return (
    <Tag key={path} className="codex-md-list" start={token.ordered ? token.start : undefined}>
      {(token.items ?? []).map((item, index) => {
        const itemPath = `${path}.li${index}`;
        return (
          <li key={itemPath} className="codex-md-list-item" data-task={item.task || undefined}>
            {item.task && (
              <input
                type="checkbox"
                className="codex-md-checkbox"
                checked={Boolean(item.checked)}
                readOnly
                tabIndex={-1}
                aria-hidden="true"
              />
            )}
            <span className="codex-md-list-body">
              {item.tokens && item.tokens.length > 0
                ? renderBlocks(item.tokens, itemPath, context)
                : <SegmentedText text={item.text ?? ""} context={context} />}
            </span>
          </li>
        );
      })}
    </Tag>
  );
}

function renderTable(token: MdToken, path: string, context: RenderContext): ReactNode {
  return (
    <div key={path} className="codex-md-table-container">
      <div
        className="codex-md-table-scroller"
        role="region"
        tabIndex={0}
        aria-label="Scrollable table"
        data-horizontal-scroll=""
        onKeyDown={(event) => {
          if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
          const step = event.altKey ? event.currentTarget.clientWidth : 40;
          event.currentTarget.scrollBy({
            left: event.key === "ArrowRight" ? step : -step,
            behavior: "instant" as ScrollBehavior
          });
          event.preventDefault();
        }}
      >
        <table className="codex-md-table">
          <thead>
            <tr>
              {(token.header ?? []).map((cell, index) => {
                const cellPath = `${path}.th${index}`;
                return (
                  <th key={cellPath}>
                    <InlineTokens tokens={cell.tokens ?? []} path={cellPath} context={context} />
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {(token.rows ?? []).map((row, rowIndex) => (
              <tr key={`${path}.tr${rowIndex}`}>
                {row.map((cell, cellIndex) => {
                  const cellPath = `${path}.td${rowIndex}.${cellIndex}`;
                  return (
                    <td key={cellPath}>
                      <InlineTokens tokens={cell.tokens ?? []} path={cellPath} context={context} />
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function renderFallbackBlock(token: MdToken, path: string, context: RenderContext): ReactNode {
  if (token.tokens && token.tokens.length > 0) {
    return (
      <p key={path} className="codex-md-paragraph">
        <InlineTokens tokens={token.tokens} path={path} context={context} />
      </p>
    );
  }
  if (token.text) {
    return (
      <p key={path} className="codex-md-paragraph">
        <SegmentedText text={token.text} context={context} />
      </p>
    );
  }
  return null;
}

function renderBlock(token: MdToken, path: string, context: RenderContext): ReactNode {
  switch (token.type) {
    case "space":
      return null;
    case "heading":
      return renderHeading(token, path, context);
    case "paragraph":
    case "text":
      return renderParagraph(token, path, context);
    case "code":
      return renderCode(token, path, context);
    case "blockquote":
      return renderBlockquote(token, path, context);
    case "list":
      return renderList(token, path, context);
    case "table":
      return renderTable(token, path, context);
    case "hr":
      return <hr key={path} className="codex-md-hr" />;
    default:
      return renderFallbackBlock(token, path, context);
  }
}

function renderBlocks(tokens: MdToken[], path: string, context: RenderContext): ReactNode[] {
  return tokens.map((token, index) => renderBlock(token, `${path}.b${index}`, context));
}

/**
 * `streaming` turns on Codex's segment fade. The timeline lives for the whole
 * message so settled segments stay settled as the text grows.
 */
export const CodexMarkdown = memo(function CodexMarkdown({
  text,
  streaming = false,
  className,
  tone = "assistant"
}: {
  text: string;
  streaming?: boolean;
  className?: string;
  tone?: "assistant" | "user";
}) {
  const streamId = useId();
  const visible = useSyncExternalStore(subscribeVisibility, visibilitySnapshot, () => true);
  const adaptive = streaming && visible;
  const bufferRef = useRef<StreamingTextBuffer | null>(null);
  bufferRef.current ??= new StreamingTextBuffer(text, streamId, adaptive);
  const buffer = bufferRef.current;
  const snapshot = useSyncExternalStore(buffer.subscribe, buffer.getSnapshot, buffer.getSnapshot);
  useLayoutEffect(() => {
    buffer.update(text, streamId, adaptive, 10);
  }, [buffer, text, streamId, adaptive]);
  useEffect(() => () => buffer.destroy(), [buffer]);

  const rendered = adaptive ? snapshot.text : text;
  const timeline = useRef<FadeTimeline | null>(null);
  timeline.current ??= createFadeTimeline();
  const tokens = useMemo(
    () => (rendered ? (codexMarked.lexer(rendered) as unknown as MdToken[]) : []),
    [rendered]
  );
  const context: RenderContext = {
    animate: streaming,
    adaptive,
    timeline: timeline.current,
    cursor: 0,
    tone
  };
  beginSegmentPass(context.timeline);
  return (
    <div
      className={className ? `codex-md ${className}` : "codex-md"}
      data-markdown-animated={streaming ? "" : undefined}
      data-adaptive={adaptive ? "" : undefined}
      data-markdown-text-style="assistant-message"
      data-markdown-text-tone={tone === "user" ? "user-message" : undefined}
    >
      {renderBlocks(tokens, "md", context)}
    </div>
  );
});
