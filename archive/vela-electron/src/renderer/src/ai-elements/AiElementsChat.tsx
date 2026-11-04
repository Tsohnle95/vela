import { safeExternalUrl } from "@shared/url-policy";
import { useStore, usePanel } from "../store";
import { AiElementsTimeline, type AiElementsTimelineProps } from "./AiElementsTimeline";
import { AiElementsActivity } from "./AiElementsActivity";
import { AiElementsComposer } from "./AiElementsComposer";
import { TodoQueue, AiElementsContext } from "./patterns";
import { MessageScroller, MessageScrollerProvider, MessageScrollerViewport, MessageScrollerContent, MessageScrollerButton } from "@/components/ui/message-scroller";
import "./ai-elements.css";

export function AiElementsChat(props: AiElementsTimelineProps & {onDemoPrompt?: (text: string) => void}) {
  const store = useStore();
  const session = props.session === undefined ? store.session : props.session;
  const view = usePanel(session?.workspace);
  const onDemoPrompt = props.onDemoPrompt;
  return <div className="ai-elements flex h-full min-h-0 w-full flex-col bg-background text-foreground" onClickCapture={event => {
    const link = event.target instanceof Element ? event.target.closest("a") : null;
    if (!link) return;
    event.preventDefault();
    const href = safeExternalUrl(link.getAttribute("href") ?? undefined);
    if (href) window.open(href, "_blank", "noopener,noreferrer");
  }}>
    <MessageScrollerProvider autoScroll>
      <MessageScroller>
        <MessageScrollerViewport>
          <MessageScrollerContent aria-busy={props.busy} className="gap-6 px-8 py-6">
            <AiElementsTimeline {...props} session={session} scrollerItems hideActivity/>
          </MessageScrollerContent>
        </MessageScrollerViewport>
        <MessageScrollerButton/>
      </MessageScroller>
    </MessageScrollerProvider>
    {(props.busy || props.transcript.length > 0) && <div className="ai-fixed-activity h-12 shrink-0 overflow-hidden px-8 py-1">
      <AiElementsActivity busy={props.busy} transcript={props.transcript} activity={props.activity} startedAt={props.startedAt}/>
    </div>}
    <div className="flex flex-col gap-3 px-8 pb-6">
      {view.sessionUsage && (view.currentModel?.limit?.context ?? 0) > 0 && <AiElementsContext usage={view.sessionUsage} limit={view.currentModel!.limit!.context}/>}
      {(view.todos ?? []).length > 0 && <TodoQueue todos={view.todos} streaming={props.busy}/>}
      <AiElementsComposer hasConversation={props.transcript.length > 0} session={session} onDemoPrompt={onDemoPrompt}/>
    </div>
  </div>;
}
