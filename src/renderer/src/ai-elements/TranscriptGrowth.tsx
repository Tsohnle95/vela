import { useLayoutEffect, useRef, type ReactNode } from "react";

/** Reveal line growth before moving the live edge, rather than easing a scroll
 * toward content that has already jumped by a whole line. */
export function TranscriptGrowth({ busy, children, onResize }: { busy: boolean; children: ReactNode; onResize?: () => void }) {
  const viewport = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const notify = useRef(onResize);
  notify.current = onResize;
  useLayoutEffect(() => {
    const outer = viewport.current;
    const inner = content.current;
    if (!outer || !inner || typeof ResizeObserver === "undefined") return;
    outer.style.height = `${inner.getBoundingClientRect().height}px`;
    let pending = 0;
    const observer = new ResizeObserver(() => {
      const height = `${inner.getBoundingClientRect().height}px`;
      if (outer.style.height !== height && !pending) {
        // Write the ancestor's target in the next animation frame, outside RO
        // delivery. Writing it here creates an ancestor resize feedback loop.
        pending = requestAnimationFrame(() => {
          pending = 0;
          const target = inner.getBoundingClientRect().height;
          const distance = Math.abs(target - outer.getBoundingClientRect().height);
          if (outer.style.transition && outer.style.transition !== "none") {
            outer.style.transitionDuration = `${Math.min(400, Math.max(120, distance / 0.22))}ms`;
          }
          outer.style.height = `${target}px`;
        });
      }
      notify.current?.();
    });
    observer.observe(inner);
    observer.observe(outer);
    return () => { observer.disconnect(); cancelAnimationFrame(pending); };
  }, []);
  useLayoutEffect(() => {
    const outer = viewport.current;
    if (!outer) return;
    const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    if (reduced) outer.style.transition = "none";
    else if (busy) outer.style.transition = "height 120ms linear";
    // Text pacing can still be draining briefly after the runtime finishes.
    const timer = busy ? undefined : setTimeout(() => { outer.style.transition = "none"; }, 350);
    return () => clearTimeout(timer);
  }, [busy]);
  return <div ref={viewport} data-transcript-growth="" style={{ overflow: "clip", minWidth: 0 }}>
    <div ref={content} className="flex min-w-0 flex-col gap-4">{children}</div>
  </div>;
}
