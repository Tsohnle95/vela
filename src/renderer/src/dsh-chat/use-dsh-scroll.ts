import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { TranscriptItem } from "@shared/types";

interface ScrollMemory {
  top: number;
  atBottom: boolean;
}

const scrollMemory = new Map<string, ScrollMemory>();

/** Test hook: drop per-workspace positions between isolated renders. */
export function clearDshScrollMemoryForTests(): void {
  scrollMemory.clear();
}

/**
 * DSH's floor-aware transcript scroll ownership. Content growth follows the
 * floor only while the reader owns the tail; any reader movement releases that
 * ownership and lets native scroll anchoring preserve the visible position.
 */
export function useDshScroll(
  memoryKey: string | undefined,
  transcript: TranscriptItem[],
  busy: boolean,
  turnCount: number
) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const follow = useRef(true);
  const observedTop = useRef(0);
  const [following, setFollowingState] = useState(true);

  const setFollowing = (value: boolean): void => {
    follow.current = value;
    setFollowingState(value);
  };

  const scrollToBottom = (): void => {
    const element = scrollRef.current;
    if (!element) return;
    element.scrollTop = element.scrollHeight;
    observedTop.current = element.scrollTop;
  };

  useLayoutEffect(() => {
    if (!memoryKey) return;
    const saved = scrollMemory.get(memoryKey);
    if (!saved) {
      setFollowing(true);
      scrollToBottom();
      return;
    }
    const element = scrollRef.current;
    if (!element) return;
    element.scrollTop = Math.min(saved.top, Math.max(0, element.scrollHeight - element.clientHeight));
    observedTop.current = element.scrollTop;
    setFollowing(saved.atBottom);
  }, [memoryKey]);

  useLayoutEffect(() => {
    if (follow.current) scrollToBottom();
  }, [transcript, busy, turnCount]);

  useEffect(() => {
    const content = contentRef.current;
    const scroller = scrollRef.current;
    if (!content || !scroller || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      if (follow.current) scrollToBottom();
    });
    observer.observe(content);
    observer.observe(scroller);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!memoryKey) return;
    return () => {
      const element = scrollRef.current;
      scrollMemory.set(memoryKey, {
        top: element?.scrollTop ?? observedTop.current,
        atBottom: follow.current
      });
    };
  }, [memoryKey]);

  const onScroll = (): void => {
    const element = scrollRef.current;
    if (!element) return;
    const floor = Math.max(0, element.scrollHeight - element.clientHeight);
    const moved = Math.abs(element.scrollTop - observedTop.current) > 0.5;
    const atBottom = floor - element.scrollTop <= 25;
    if (!moved) {
      if (follow.current) scrollToBottom();
      return;
    }
    observedTop.current = element.scrollTop;
    setFollowing(atBottom);
  };

  const returnToBottom = (): void => {
    setFollowing(true);
    scrollToBottom();
  };

  return { scrollRef, contentRef, following, onScroll, returnToBottom };
}
