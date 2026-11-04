import { useLayoutEffect, useRef, type CSSProperties, type ReactNode, type RefObject } from "react";
import { MessageScroller, useMessageScroller, useMessageScrollerScrollable } from "@shadcn/react/message-scroller";
import { AiElementsActivity } from "./AiElementsActivity";
import { AiElementsSurface } from "./AiElementsSurface";
import type { AiElementsTimelineProps } from "./AiElementsTimeline";

// Upstream owns scrolling; the app retains its existing panel and composer.
export function AgentResponseViewport({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <MessageScroller.Provider autoScroll defaultScrollPosition="end">
    <StreamingViewport style={style}>{children}</StreamingViewport>
  </MessageScroller.Provider>;
}

function StreamingViewport({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  const content = useRef<HTMLDivElement>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const { scrollToEnd } = useMessageScroller();
  const { end } = useMessageScrollerScrollable();
  const following = useRef(!end);
  useLayoutEffect(() => { following.current = !end; }, [end]);
  useLayoutEffect(() => {
    if (following.current) scrollToEnd({ behavior: "auto" });
  }, [children, scrollToEnd]);
  useLayoutEffect(() => {
    if (!content.current || !viewport.current || typeof ResizeObserver === "undefined") return;
    // Upstream defers its resize follow to the next rAF. Shadow transcript growth
    // must reach the same upstream scroll owner before this frame is painted.
    const observer = new ResizeObserver(() => {
      if (following.current) scrollToEnd({ behavior: "auto" });
    });
    observer.observe(content.current);
    observer.observe(viewport.current);
    return () => observer.disconnect();
  }, [scrollToEnd]);
  return (
    <MessageScroller.Root data-agent-response-viewport="" style={{ position: "relative", display: "flex", flexDirection: "column", flex: 1, minHeight: 0, minWidth: 0, overflow: "hidden" }}>
      <MessageScroller.Viewport ref={viewport} className="agent-scroll" style={{ ...style, display: "block", minHeight: 0, contain: "content" }}>
        <MessageScroller.Content ref={content} data-slot="message-scroller-content" style={{ display: "flex", flexDirection: "column", gap: 10, height: "max-content", minHeight: "100%", minWidth: 0 }}>
          {children}
        </MessageScroller.Content>
      </MessageScroller.Viewport>
    </MessageScroller.Root>
  );
}

export function AgentScrollArea({ legacy, legacyRef, onLegacyScroll, showResponse, children }: {
  legacy: boolean; legacyRef: RefObject<HTMLDivElement | null>; onLegacyScroll: () => void; showResponse: boolean; children: ReactNode;
}) {
  if (legacy) return <div className="agent-scroll" ref={legacyRef} onScroll={onLegacyScroll}>{children}</div>;
  return <AgentResponseViewport style={showResponse ? { padding: 0 } : undefined}>{children}</AgentResponseViewport>;
}

type StatusProps = Pick<AiElementsTimelineProps, "busy" | "transcript" | "activity" | "startedAt"> & { legacy: boolean; visible: boolean };
export function AgentStreamStatus({ legacy, visible, ...props }: StatusProps) {
  if (legacy || !visible || (!props.busy && !props.activity?.isWaitingForPermission)) return null;
  return <div data-agent-stream-status="" style={{ flexShrink: 0, overflow: "hidden" }}>
    <AiElementsSurface compact><AiElementsActivity {...props}/></AiElementsSurface>
  </div>;
}
