import type { CSSProperties, ReactNode, RefObject } from "react";
import { MessageScroller } from "@shadcn/react/message-scroller";
import { AiElementsActivity } from "./AiElementsActivity";
import { AiElementsSurface } from "./AiElementsSurface";
import type { AiElementsTimelineProps } from "./AiElementsTimeline";

// Upstream owns scrolling; the app retains its existing panel and composer.
export function AgentResponseViewport({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <MessageScroller.Provider autoScroll defaultScrollPosition="end">
    <MessageScroller.Root data-agent-response-viewport="" style={{ position: "relative", display: "flex", flexDirection: "column", flex: 1, minHeight: 0, minWidth: 0, overflow: "hidden" }}>
      <MessageScroller.Viewport className="agent-scroll" style={{ ...style, display: "block", minHeight: 0, contain: "content" }}>
        <MessageScroller.Content data-slot="message-scroller-content" style={{ display: "flex", flexDirection: "column", gap: 10, height: "max-content", minHeight: "100%", minWidth: 0 }}>
          {children}
        </MessageScroller.Content>
      </MessageScroller.Viewport>
    </MessageScroller.Root>
  </MessageScroller.Provider>;
}

export function AgentScrollArea({ legacy, legacyRef, onLegacyScroll, showResponse, children }: {
  legacy: boolean; legacyRef: RefObject<HTMLDivElement | null>; onLegacyScroll: () => void; showResponse: boolean; children: ReactNode;
}) {
  if (legacy) return <div className="agent-scroll" ref={legacyRef} onScroll={onLegacyScroll}>{children}</div>;
  return <AgentResponseViewport style={showResponse ? { padding: 0 } : undefined}>{children}</AgentResponseViewport>;
}

type StatusProps = Pick<AiElementsTimelineProps, "busy" | "transcript" | "activity" | "startedAt"> & { legacy: boolean; visible: boolean };
export function AgentStreamStatus({ legacy, visible, ...props }: StatusProps) {
  if (legacy || !visible) return null;
  return <div data-agent-stream-status="" style={{ height: 48, flexShrink: 0, overflow: "hidden" }}>
    <AiElementsSurface compact><AiElementsActivity {...props}/></AiElementsSurface>
  </div>;
}
