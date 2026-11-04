import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { useStore } from "../store";
import type { TranscriptItem, SessionInfo } from "@shared/types";
import type { AssistantItem, AssistantPart } from "./timeline-model";
import { TextShimmer } from "./TextShimmer";
import { OpenCodeMarkdown as Markdown } from "./OpenCodeMarkdown";
import { OpenCodeIcon } from "./OpenCodeIcon";

export async function writeClipboard(text: string): Promise<boolean> {
  const clipboard = typeof navigator === "undefined" ? undefined : navigator.clipboard;
  if (!clipboard?.writeText) return false;
  return clipboard.writeText(text).then(() => true, () => false);
}

const TEXT_RENDER_PACE_MS = 24;
const TEXT_RENDER_IMMEDIATE = 512;
const TEXT_RENDER_SNAP = /[\s.,!?;:)\]]/;

function nextPacedEnd(text: string, start: number): number {
  const remaining = text.length - start;
  const step = remaining <= 12 ? 2 : remaining <= 48 ? 4 : remaining <= 96 ? 8 : Math.min(256, Math.ceil(remaining / 4));
  const end = Math.min(text.length, start + step);
  const max = Math.min(text.length, end + 8);
  for (let index = end; index < max; index += 1) {
    if (TEXT_RENDER_SNAP.test(text[index] ?? "")) return index + 1;
  }
  return end;
}

function PacedMarkdown({ text, streaming, cacheKey }: { text: string; streaming: boolean; cacheKey: string }): ReactNode {
  const [visible, setVisible] = useState(text);
  const shown = useRef(text);
  const latest = useRef({ text, streaming });
  const timeout = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  latest.current = { text, streaming };

  const sync = (value: string): void => {
    shown.current = value;
    setVisible(value);
  };
  const clear = (): void => {
    if (timeout.current === undefined) return;
    clearTimeout(timeout.current);
    timeout.current = undefined;
  };
  const run = (): void => {
    timeout.current = undefined;
    const current = latest.current;
    if (!current.streaming || !current.text.startsWith(shown.current) || current.text.length <= shown.current.length
      || current.text.length - shown.current.length <= TEXT_RENDER_IMMEDIATE) {
      sync(current.text);
      return;
    }
    const end = nextPacedEnd(current.text, shown.current.length);
    sync(current.text.slice(0, end));
    if (end < current.text.length) timeout.current = setTimeout(run, TEXT_RENDER_PACE_MS);
  };

  useLayoutEffect(() => {
    if (!streaming || !text.startsWith(shown.current) || text.length <= shown.current.length
      || text.length - shown.current.length <= TEXT_RENDER_IMMEDIATE) {
      clear();
      sync(text);
      return;
    }
    if (timeout.current === undefined) timeout.current = setTimeout(run, TEXT_RENDER_PACE_MS);
  }, [text, streaming]);

  useEffect(() => () => clear(), []);
  return <Markdown text={visible} streaming={streaming} cacheKey={cacheKey} />;
}

function CopyResponse({ text, target = "response" }: { text: string; target?: "response" | "message" }): ReactNode {
  const [copied, setCopied] = useState(false);
  const copy = async (): Promise<void> => {
    if (!text) return;
    if (!await writeClipboard(text)) return;
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  const label = copied ? "Copied" : `Copy ${target}`;
  return (
    <div data-component="tooltip-v2-trigger" data-tooltip={label}>
    <button data-component="icon-button-v2" data-size="normal" data-variant="ghost-muted"
      data-slot={target === "message" ? "user-message-copy-button" : "text-part-copy-button"}
      aria-label={label} onMouseDown={(event) => event.preventDefault()} onClick={() => void copy()}>
      <OpenCodeIcon name={copied ? "check" : "outline-copy"} />
    </button>
    </div>
  );
}

export function TextPart({ part, streaming, showCopy, message, duration }: {
  part: Extract<AssistantPart, { kind: "text" }>;
  streaming: boolean;
  showCopy: boolean;
  message: AssistantItem;
  duration?: number;
}): ReactNode {
  const { models } = useStore();
  const text = part.text.trim();
  if (!text) return null;
  const model = models?.find((model) => model.id === message.model?.id && model.providerID === message.model?.providerID)?.name ?? message.model?.id;
  const elapsed = duration ?? (message.completedAt !== undefined && message.createdAt !== undefined ? message.completedAt - message.createdAt : undefined);
  const seconds = elapsed !== undefined && elapsed >= 0 ? Math.round(elapsed / 1000) : undefined;
  const time = seconds === undefined ? "" : seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  const meta = [message.agent && message.agent[0].toUpperCase() + message.agent.slice(1), model, time, message.interrupted && "Interrupted"].filter(Boolean).join(" · ");
  return (
    <div data-component="text-part" data-timeline-part-id={part.id}>
      <div data-slot="text-part-body"><PacedMarkdown text={text} streaming={streaming} cacheKey={`${message.id}:${part.id}`} /></div>
      {showCopy && (
        <div data-slot="text-part-copy-wrapper" data-interrupted={message.interrupted ? "" : undefined}>
          <CopyResponse text={part.text} />
          {meta && <span data-slot="text-part-meta" className="text-12-regular text-text-weak">{meta}</span>}
        </div>
      )}
    </div>
  );
}

export function TimelineRow({ tag, children, previous }: { tag: string; children: ReactNode; previous?: boolean }): ReactNode {
  return (
    <div data-timeline-row={tag} data-previous-assistant-part={previous ? "true" : undefined} className="opencode-row">
      <div data-component="session-turn">
        <div data-slot="session-turn-message-container">{children}</div>
      </div>
    </div>
  );
}

const THINKING_LABEL = "Thinking";

function ThinkingHeading({ text }: { text: string }): ReactNode {
  const [state, setState] = useState({ current: text, old: "", width: "auto", swapping: false });
  const [ready, setReady] = useState(false);
  const root = useRef<HTMLSpanElement>(null);
  const entering = useRef<HTMLSpanElement>(null);
  const leaving = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    setState((previous) => {
      if (previous.current === text) return previous;
      if (text.startsWith(previous.current)) return { ...previous, current: text };
      return { ...previous, current: text, old: previous.current, swapping: true };
    });
  }, [text]);

  useLayoutEffect(() => {
    const width = Math.max(entering.current?.scrollWidth ?? 0, leaving.current?.scrollWidth ?? 0);
    if (width > 0) setState((previous) => {
      const prior = Number.parseFloat(previous.width);
      return Number.isFinite(prior) && prior >= width ? previous : { ...previous, width: `${width}px` };
    });
    if (!state.swapping) return;
    if (typeof requestAnimationFrame !== "function") {
      setState((previous) => ({ ...previous, swapping: false }));
      return;
    }
    const frame = requestAnimationFrame(() => {
      void root.current?.offsetHeight;
      setState((previous) => ({ ...previous, swapping: false }));
    });
    return () => cancelAnimationFrame(frame);
  }, [state.current, state.old, state.swapping]);

  useEffect(() => {
    let cancelled = false;
    let frame: number | undefined;
    const finish = (): void => {
      if (cancelled) return;
      if (typeof requestAnimationFrame !== "function") {
        setReady(true);
        return;
      }
      frame = requestAnimationFrame(() => setReady(true));
    };
    const fonts = document.fonts;
    if (fonts) void fonts.ready.finally(finish);
    else finish();
    return () => {
      cancelled = true;
      if (frame !== undefined) cancelAnimationFrame(frame);
    };
  }, []);

  return (
    <span
      ref={root}
      data-component="text-reveal"
      data-slot="session-turn-thinking-heading"
      data-ready={ready ? "true" : "false"}
      data-swapping={state.swapping ? "true" : "false"}
      aria-label={text}
      style={{ "--text-reveal-duration": "700ms", "--text-reveal-travel": "25px" } as CSSProperties}
    >
      <span data-slot="text-reveal-track" style={{ width: state.width }}>
        <span data-slot="text-reveal-entering" ref={entering} aria-hidden="true">{state.current}</span>
        <span data-slot="text-reveal-leaving" ref={leaving} aria-hidden="true">{state.old}</span>
      </span>
    </span>
  );
}

export function SessionTurnThinking({ heading }: { heading?: string }): ReactNode {
  return (
    <TimelineRow tag="AssistantWorking">
      <div data-slot="session-turn-thinking" role="status" aria-live="polite">
        <TextShimmer text={THINKING_LABEL} tone="thinking" />
        {heading && <ThinkingHeading text={heading} />}
      </div>
    </TimelineRow>
  );
}

export function UserMessage({ item, session, busy }: { item: Extract<TranscriptItem, { kind: "user" }>; session: SessionInfo | null; busy: boolean }): ReactNode {
  const { stageRevert, models } = useStore();
  const model = models?.find((model) => model.id === item.model?.id && model.providerID === item.model?.providerID)?.name ?? item.model?.id;
  const head = [item.agent && item.agent[0].toUpperCase() + item.agent.slice(1), model].filter(Boolean).join("\u00a0·\u00a0");
  const stamp = item.createdAt === undefined ? "" : new Intl.DateTimeFormat(undefined, { timeStyle: "short" }).format(item.createdAt);
  return (
    <TimelineRow tag="UserMessage">
      <div data-slot="session-turn-message-content" aria-live="off">
        <div data-component="user-message">
          {item.text && <div data-slot="user-message-body"><div data-slot="user-message-text" dir="auto">{item.text}</div></div>}
          {item.attachments && item.attachments.length > 0 && (
            <div data-slot="user-message-attachments">
              {item.attachments.map((attachment) => (
                <div data-slot="user-message-attachment" data-type="file" key={attachment.name}>
                  <div data-slot="user-message-attachment-file"><span className="codicon codicon-file" /><span data-slot="user-message-attachment-name">{attachment.name}</span></div>
                </div>
              ))}
            </div>
          )}
          {item.text && <div data-slot="user-message-copy-wrapper">
            {(head || stamp) && <span data-slot="user-message-meta-wrap">
              {head && <span data-slot="user-message-meta" className="text-12-regular text-text-weak">{head}</span>}
              {head && stamp && <span className="text-12-regular text-text-weak">{ "\u00a0·\u00a0" }</span>}
              {stamp && <span data-slot="user-message-meta-tail" className="text-12-regular text-text-weak">{stamp}</span>}
            </span>}
            {session && <div data-component="tooltip-v2-trigger" data-tooltip="Revert message"><button data-component="icon-button-v2" data-size="normal" data-variant="ghost-muted" aria-label="Revert message" disabled={busy}
              onMouseDown={(event) => event.preventDefault()} onClick={() => void stageRevert(session.workspace, item.id)}><OpenCodeIcon name="reset" /></button></div>}
            <CopyResponse text={item.text} target="message" />
          </div>}
        </div>
      </div>
    </TimelineRow>
  );
}

