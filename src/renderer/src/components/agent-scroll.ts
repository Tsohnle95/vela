import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import type { SessionInfo, TranscriptItem } from "@shared/types";

interface AgentScrollMemory {
  scrollTop: number;
  atBottom: boolean;
  observedTop: number;
}

// Preserve the reader's place when a workspace panel remounts.
const agentScrollMemory = new Map<string, AgentScrollMemory>();

/** Test hook: drop remount-persisted scroll positions between isolated renders. */
export function clearAgentScrollMemoryForTests(): void {
  agentScrollMemory.clear();
}

function scrollMemoryKey(session: SessionInfo | null | undefined): string | null {
  return session?.workspace.id ?? null;
}


export function useAgentScroll(activeSession: SessionInfo | null, transcript: TranscriptItem[], busy: boolean,
  queuedMessages: { id: string; content: string }[] | undefined, smooth = false) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const atBottomRef = useRef(true);
  const observedTopRef = useRef(0);
  const memoryKey = scrollMemoryKey(activeSession);
  const animationRef = useRef<number | null>(null);
  const busyRef = useRef(busy);
  const followUntilRef = useRef(0);
  busyRef.current = busy;
  const scrollToBottom = (): void => {
    const el = scrollRef.current;
    if (!el) return;
    if (smooth && (busyRef.current || animationRef.current !== null)) {
      // Follow the rendered height before paint. Easing here separately from
      // line growth makes the activity row jump down, then recoil upward.
      el.scrollTop = Math.max(0, el.scrollHeight - el.clientHeight);
      observedTopRef.current = el.scrollTop;
      if (animationRef.current === null) {
        const follow = (now: number): void => {
          animationRef.current = null;
          if (!atBottomRef.current) return;
          if (busyRef.current) followUntilRef.current = now + 350;
          el.scrollTop = Math.max(0, el.scrollHeight - el.clientHeight);
          observedTopRef.current = el.scrollTop;
          if (memoryKey) agentScrollMemory.set(memoryKey, { scrollTop: el.scrollTop, atBottom: true, observedTop: el.scrollTop });
          // Keep sampling while streaming: Shadow DOM content can grow without
          // changing the observed flex item's border box.
          if (busyRef.current || now < followUntilRef.current) animationRef.current = requestAnimationFrame(follow);
        };
        animationRef.current = requestAnimationFrame(follow);
      }
      return;
    }
    if (animationRef.current !== null) cancelAnimationFrame(animationRef.current);
    animationRef.current = null;
    el.scrollTop = el.scrollHeight;
    observedTopRef.current = el.scrollTop;
    atBottomRef.current = true;
    if (memoryKey) agentScrollMemory.set(memoryKey, { scrollTop: el.scrollTop, atBottom: true, observedTop: observedTopRef.current });
  };

  useLayoutEffect(() => () => {
    if (animationRef.current !== null) cancelAnimationFrame(animationRef.current);
    animationRef.current = null;
  }, [memoryKey]);

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

  const followRef = useRef(scrollToBottom);
  followRef.current = scrollToBottom;
  useAgentResizeFollow(scrollRef, atBottomRef, followRef);

  useReaderScrollIntent(scrollRef, atBottomRef, animationRef, memoryKey);

  const onScroll = (): void => {
    const el = scrollRef.current;
    if (!el) return;
    const floor = Math.max(0, el.scrollHeight - el.clientHeight);
    const movedByReader = el.scrollTop < Math.min(observedTopRef.current, floor) - 0.5;
    const atBottom = floor - el.scrollTop <= 25 || (!movedByReader && atBottomRef.current);
    if (!movedByReader && atBottom) {
      atBottomRef.current = true;
      scrollToBottom();
      return;
    }
    atBottomRef.current = atBottom;
    observedTopRef.current = el.scrollTop;
    if (memoryKey) agentScrollMemory.set(memoryKey, { scrollTop: el.scrollTop, atBottom, observedTop: el.scrollTop });
  };

  return { scrollRef, onScroll, onContentResize: () => { if (atBottomRef.current) scrollToBottom(); } };
}

function useReaderScrollIntent(
  scrollRef: RefObject<HTMLDivElement | null>,
  atBottomRef: RefObject<boolean>,
  animationRef: RefObject<number | null>,
  memoryKey: string | null
): void {
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const release = () => {
      atBottomRef.current = false;
      if (animationRef.current !== null) cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    };
    const wheel = (event: WheelEvent) => { if (event.deltaY < 0) release(); };
    const key = (event: KeyboardEvent) => {
      if (["ArrowUp", "PageUp", "Home"].includes(event.key)) release();
    };
    let touchY = 0;
    const touchStart = (event: TouchEvent) => { touchY = event.touches[0]?.clientY ?? 0; };
    const touchMove = (event: TouchEvent) => {
      const next = event.touches[0]?.clientY ?? touchY;
      if (next > touchY) release();
      touchY = next;
    };
    el.addEventListener("wheel", wheel, { passive: true });
    el.addEventListener("keydown", key);
    el.addEventListener("touchstart", touchStart, { passive: true });
    el.addEventListener("touchmove", touchMove, { passive: true });
    return () => {
      el.removeEventListener("wheel", wheel);
      el.removeEventListener("keydown", key);
      el.removeEventListener("touchstart", touchStart);
      el.removeEventListener("touchmove", touchMove);
    };
  }, [memoryKey]);
}

function useAgentResizeFollow(
  scrollRef: RefObject<HTMLDivElement | null>,
  atBottomRef: RefObject<boolean>,
  followRef: RefObject<() => void>
): void {
  useEffect(() => {
    const el = scrollRef.current;
    const content = el?.querySelector<HTMLElement>('[data-slot="session-turn-list"]');
    if (!el || !content || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      if (atBottomRef.current) followRef.current();
    });
    observer.observe(content);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
}
