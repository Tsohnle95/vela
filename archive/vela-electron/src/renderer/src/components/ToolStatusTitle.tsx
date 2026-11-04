import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { TextShimmer } from "./TextShimmer";

// Port of OpenCode's `tool-status-title.tsx`: crossfades a done label into an
// active label (optionally sharing a common prefix), animating the width so the
// surrounding text does not jump.
function common(active: string, done: string): { prefix: string; active: string; done: string } {
  const a = Array.from(active);
  const b = Array.from(done);
  let i = 0;
  while (i < a.length && i < b.length && a[i] === b[i]) i += 1;
  return { prefix: a.slice(0, i).join(""), active: a.slice(i).join(""), done: b.slice(i).join("") };
}

function contentWidth(el: HTMLElement | null | undefined): string | undefined {
  if (!el) return undefined;
  return `${Math.ceil(el.getBoundingClientRect().width)}px`;
}

export function ToolStatusTitle({
  active,
  activeText,
  doneText,
  split = true,
  className
}: {
  active: boolean;
  activeText: string;
  doneText: string;
  split?: boolean;
  className?: string;
}): ReactNode {
  const parts = common(activeText, doneText);
  const suffix = split && Array.from(parts.prefix).length >= 2 && parts.active.length > 0 && parts.done.length > 0;
  const activeTail = suffix ? parts.active : activeText;
  const doneTail = suffix ? parts.done : doneText;

  const [shownActive, setShownActive] = useState(active);
  const [animating, setAnimating] = useState(false);
  const [width, setWidth] = useState<string | undefined>(undefined);
  const activeRef = useRef<HTMLSpanElement>(null);
  const doneRef = useRef<HTMLSpanElement>(null);
  const widthRef = useRef<HTMLSpanElement>(null);
  const frameRef = useRef<number | undefined>(undefined);
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const mountedRef = useRef(false);

  useEffect(() => {
    const finish = (): void => {
      if (frameRef.current !== undefined) cancelAnimationFrame(frameRef.current);
      if (timerRef.current !== undefined) clearTimeout(timerRef.current);
      frameRef.current = undefined;
      timerRef.current = undefined;
      setAnimating(false);
      setWidth(undefined);
    };

    if (!mountedRef.current) {
      mountedRef.current = true;
      setShownActive(active);
      return finish;
    }

    const first = contentWidth(widthRef.current);
    finish();
    setShownActive(active);
    if (!first) return finish;

    setAnimating(true);
    setWidth(first);
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = undefined;
      const last = contentWidth(active ? activeRef.current : doneRef.current);
      if (!last) return finish();
      if (first !== last) setWidth(last);
      timerRef.current = setTimeout(finish, 600);
    });

    return finish;
  }, [active, activeTail, doneTail]);

  const activeNode = (
    <span data-slot="tool-status-active" ref={activeRef}>
      <TextShimmer text={activeTail} active={shownActive} />
    </span>
  );
  const doneNode = (
    <span data-slot="tool-status-done" ref={doneRef}>
      <TextShimmer text={doneTail} active={false} />
    </span>
  );
  const swapStyle: CSSProperties = width ? { width } : {};

  return (
    <span
      data-component="tool-status-title"
      data-active={shownActive ? "true" : "false"}
      data-ready={animating ? "true" : "false"}
      data-mode={suffix ? "suffix" : "swap"}
      className={className}
      aria-label={shownActive ? activeText : doneText}
    >
      {suffix ? (
        <span data-slot="tool-status-suffix">
          <span data-slot="tool-status-prefix">
            <TextShimmer text={parts.prefix} active={shownActive} />
          </span>
          <span data-slot="tool-status-tail" ref={widthRef} style={swapStyle}>
            {(animating || shownActive) && activeNode}
            {(animating || !shownActive) && doneNode}
          </span>
        </span>
      ) : (
        <span data-slot="tool-status-swap" ref={widthRef} style={swapStyle}>
          {(animating || shownActive) && activeNode}
          {(animating || !shownActive) && doneNode}
        </span>
      )}
    </span>
  );
}
