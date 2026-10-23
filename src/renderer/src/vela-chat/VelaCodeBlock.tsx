import { useEffect, useState, type ReactNode } from "react";
import { loadHighlighter } from "../ai-elements/upstream/lib/highlighter";
import { writeClipboard } from "../components/TimelineProse";

/**
 * A fenced code block that shows a plain surface while the fence is still
 * streaming and highlights once it settles, so it never re-tokenizes on every
 * chunk. The header carries the language, copy and wrap
 * controls.
 */
export function VelaCodeBlock({
  code,
  lang,
  streaming
}: {
  code: string;
  lang?: string;
  streaming: boolean;
}) {
  const [lines, setLines] = useState<ReactNode[][] | null>(null);
  const [copied, setCopied] = useState(false);
  const [wrap, setWrap] = useState(false);

  useEffect(() => {
    if (streaming) {
      setLines(null);
      return;
    }
    let cancelled = false;
    const language = lang ?? "text";
    void loadHighlighter(language)
      .then((highlighter) => (highlighter ? highlighter.highlight(code).lines : null))
      .then((next) => {
        if (!cancelled) setLines(next);
      })
      .catch(() => {
        if (!cancelled) setLines(null);
      });
    return () => {
      cancelled = true;
    };
  }, [code, lang, streaming]);

  const copy = (): void => {
    void writeClipboard(code).then((ok) => {
      if (!ok) return;
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <div className="vela-code" data-language={lang || undefined}>
      <div className="vela-code-header">
        <span className="vela-code-lang">{lang ?? "text"}</span>
        <span className="vela-code-actions">
          <button type="button" className="vela-code-action" onClick={() => setWrap((value) => !value)}>
            {wrap ? "No wrap" : "Wrap"}
          </button>
          <button type="button" className="vela-code-action" onClick={copy}>
            {copied ? "Copied" : "Copy"}
          </button>
        </span>
      </div>
      <pre className="vela-code-pre" data-wrap={wrap || undefined}>
        <code>
          {lines
            ? lines.map((line, lineIndex) => (
                <span key={lineIndex} className="vela-code-line">
                  {line.length > 0 ? line : "\u200b"}
                  {"\n"}
                </span>
              ))
            : code}
        </code>
      </pre>
    </div>
  );
}
