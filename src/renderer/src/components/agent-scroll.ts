import { useEffect, useLayoutEffect, useRef } from "react";
import type { SessionInfo, TranscriptItem } from "@shared/types";

interface AgentScrollMemory {
  scrollTop: number;
  atBottom: boolean;
  observedTop: number;
}

// Remount-safe scroll position per workspace. Opening Settings unmounts the
// whole workspace area (see Layout in App.tsx), so in-memory scroll refs alone
// lose the reader's place and a fresh mount re-pins to the bottom.
const agentScrollMemory = new Map<string, AgentScrollMemory>();

/** Test hook: drop remount-persisted scroll positions between isolated renders. */
export function clearAgentScrollMemoryForTests(): void {
  agentScrollMemory.clear();
}

function scrollMemoryKey(session: SessionInfo | null | undefined): string | null {
  return session?.workspace.id ?? null;
}

export function useAgentScroll(activeSession: SessionInfo | null, transcript: TranscriptItem[], busy: boolean,
  queuedMessages: { id: string; content: string }[] | undefined) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const atBottomRef = useRef(true);
  const observedTopRef = useRef(0);
  const memoryKey = scrollMemoryKey(activeSession);
  const scrollToBottom = (): void => {
    const el = scrollRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
    observedTopRef.current = el.scrollTop;
    atBottomRef.current = true;
    if (memoryKey) agentScrollMemory.set(memoryKey, { scrollTop: el.scrollTop, atBottom: true, observedTop: observedTopRef.current });
  };

  const transcriptTip = transcript.at(-1);
  const transcriptTipParts = transcriptTip?.kind === "assistant" ? transcriptTip.parts.length : 0;
  const queueSignature = (queuedMessages ?? []).map((message) => `${message.id}:${message.content.length}`).join(",");
  const followSignature = `${transcript.length}:${transcriptTip?.id ?? ""}:${transcriptTipParts}:${busy ? 1 : 0}:${queueSignature}`;

  // Restore the reader's place after a remount (e.g. Settings unmounts the
  // workspace area). Runs before the follow effect below so a restored
  // mid-conversation position is not treated as pinned-to-bottom.
  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (!el || !memoryKey) return;
    const saved = agentScrollMemory.get(memoryKey);
    if (!saved) return;
    atBottomRef.current = saved.atBottom;
    observedTopRef.current = saved.observedTop;
    if (saved.atBottom) {
      scrollToBottom();
    } else {
      el.scrollTop = Math.min(saved.scrollTop, Math.max(0, el.scrollHeight - el.clientHeight));
      observedTopRef.current = el.scrollTop;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [memoryKey]);

  useEffect(() => {
    if (!memoryKey) return;
    return () => {
      const el = scrollRef.current;
      agentScrollMemory.set(memoryKey, {
        scrollTop: el?.scrollTop ?? observedTopRef.current,
        atBottom: atBottomRef.current,
        observedTop: observedTopRef.current
      });
    };
  }, [memoryKey]);

  useLayoutEffect(() => {
    if (atBottomRef.current) scrollToBottom();
  }, [followSignature]);

  useEffect(() => {
    const el = scrollRef.current;
    const content = el?.querySelector<HTMLElement>('[data-slot="session-turn-list"]');
    if (!el || !content || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      if (atBottomRef.current) scrollToBottom();
    });
    observer.observe(content);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const onScroll = (): void => {
    const el = scrollRef.current;
    if (!el) return;
    const floor = Math.max(0, el.scrollHeight - el.clientHeight);
    const movedByReader = Math.abs(el.scrollTop - Math.min(observedTopRef.current, floor)) > 0.5;
    const atBottom = movedByReader ? floor - el.scrollTop <= 25 : atBottomRef.current;
    if (!movedByReader && atBottom) {
      scrollToBottom();
      return;
    }
    atBottomRef.current = atBottom;
    observedTopRef.current = el.scrollTop;
    if (memoryKey) agentScrollMemory.set(memoryKey, { scrollTop: el.scrollTop, atBottom, observedTop: el.scrollTop });
  };

  return { scrollRef, onScroll };
}
