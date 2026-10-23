import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { TranscriptItem } from "@shared/types";

/**
 * Codex's transcript scroll semantics, ported from `ThreadScrollLayout`.
 *
 * - distance from the floor is `max(0, scrollHeight - clientHeight - scrollTop)`
 * - "at the bottom" is a 24px threshold, not a ratio
 * - content growth follows the floor *instantly* while pinned; it never animates
 * - only an explicit scroll-to-bottom animates: 260ms easeOutCubic, then snap
 * - native scroll anchoring is disabled; the hook owns the position
 * - position is remembered as distance-from-bottom, not scrollTop
 */

export const BOTTOM_THRESHOLD_PX = 24;
export const SCROLL_TO_BOTTOM_DURATION_MS = 260;

const scrollMemory = new Map<string, number>();

/** Test hook: drop per-workspace positions between isolated renders. */
export function clearCodexScrollMemoryForTests(): void {
  scrollMemory.clear();
}

export function distanceFromBottom(element: {
  scrollHeight: number;
  clientHeight: number;
  scrollTop: number;
}): number {
  return Math.max(0, element.scrollHeight - element.clientHeight - element.scrollTop);
}

export function easeOutCubic(progress: number): number {
  return 1 - (1 - progress) ** 3;
}

function prefersReducedMotion(): boolean {
  return typeof window.matchMedia === "function"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function maxScrollTop(element: { scrollHeight: number; clientHeight: number }): number {
  return Math.max(0, element.scrollHeight - element.clientHeight);
}

export interface ScrollAnimationState {
  frame: number | null;
}

export function cancelScrollAnimation(state: ScrollAnimationState): void {
  if (state.frame === null) return;
  window.cancelAnimationFrame(state.frame);
  state.frame = null;
}

export function scrollToFloor(element: HTMLElement): void {
  element.scrollTop = maxScrollTop(element);
}

export function animateScrollToFloor(
  element: HTMLElement,
  state: ScrollAnimationState,
  onSettled: () => void
): void {
  const from = distanceFromBottom(element);
  if (prefersReducedMotion() || from <= BOTTOM_THRESHOLD_PX) {
    scrollToFloor(element);
    onSettled();
    return;
  }
  cancelScrollAnimation(state);
  const startedAt = performance.now();
  const step = (now: number): void => {
    const progress = Math.min(1, (now - startedAt) / SCROLL_TO_BOTTOM_DURATION_MS);
    const eased = easeOutCubic(progress);
    element.scrollTop = maxScrollTop(element) - from * (1 - eased);
    if (progress < 1) {
      state.frame = window.requestAnimationFrame(step);
      return;
    }
    state.frame = null;
    scrollToFloor(element);
    onSettled();
  };
  state.frame = window.requestAnimationFrame(step);
}

export function useCodexScroll(
  memoryKey: string | undefined,
  transcript: TranscriptItem[],
  busy: boolean,
  turnCount: number
) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  const lastDistance = useRef(0);
  const animation = useRef<ScrollAnimationState>({ frame: null });
  const [showButton, setShowButton] = useState(false);

  const settleOnFloor = (): void => {
    const element = scrollRef.current;
    if (!element) return;
    lastDistance.current = 0;
    setShowButton(false);
  };

  useLayoutEffect(() => {
    if (!memoryKey) return;
    const element = scrollRef.current;
    if (!element) return;
    const saved = scrollMemory.get(memoryKey);
    if (saved === undefined) {
      pinned.current = true;
      scrollToFloor(element);
      settleOnFloor();
      return;
    }
    element.scrollTop = Math.max(0, maxScrollTop(element) - saved);
    const distance = distanceFromBottom(element);
    lastDistance.current = distance;
    pinned.current = distance <= BOTTOM_THRESHOLD_PX;
    setShowButton(distance > BOTTOM_THRESHOLD_PX);
  }, [memoryKey]);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!pinned.current || !element) return;
    scrollToFloor(element);
    lastDistance.current = 0;
  }, [transcript, busy, turnCount]);

  useEffect(() => {
    const content = contentRef.current;
    const scroller = scrollRef.current;
    if (!content || !scroller || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      if (!pinned.current) return;
      scrollToFloor(scroller);
      lastDistance.current = 0;
    });
    observer.observe(content);
    observer.observe(scroller);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!memoryKey) return;
    return () => {
      cancelScrollAnimation(animation.current);
      const element = scrollRef.current;
      if (element) scrollMemory.set(memoryKey, distanceFromBottom(element));
    };
  }, [memoryKey]);

  const onScroll = (): void => {
    const element = scrollRef.current;
    if (!element) return;
    const distance = distanceFromBottom(element);
    if (distance <= BOTTOM_THRESHOLD_PX) {
      pinned.current = true;
    } else if (distance > lastDistance.current) {
      pinned.current = false;
    }
    lastDistance.current = distance;
    setShowButton(distance > BOTTOM_THRESHOLD_PX);
  };

  const scrollToBottom = (): void => {
    const element = scrollRef.current;
    if (!element) return;
    pinned.current = true;
    if (busy) {
      cancelScrollAnimation(animation.current);
      scrollToFloor(element);
      settleOnFloor();
      return;
    }
    animateScrollToFloor(element, animation.current, settleOnFloor);
    setShowButton(false);
  };

  return { scrollRef, contentRef, showButton, onScroll, scrollToBottom };
}
