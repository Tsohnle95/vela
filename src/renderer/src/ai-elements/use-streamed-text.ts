import { useLayoutEffect, useRef, useState } from "react";

// Presentation only: the transcript remains authoritative and receives every
// event immediately. A short buffer spreads bursty provider chunks over frames.
export function useStreamedText(text: string, streaming: boolean): string {
  const reducedMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  const [visible, setVisible] = useState(text);
  const state = useRef({ target: text, position: visible.length, visible, rate: 80, arrival: 0, frame: 0 });

  useLayoutEffect(() => {
    const current = state.current;
    const now = performance.now();
    const correction = !text.startsWith(current.target);
    if (reducedMotion || correction || (!streaming && current.frame === 0)) {
      cancelAnimationFrame(current.frame);
      Object.assign(current, { target: text, position: text.length, visible: text, frame: 0 });
      setVisible(text);
      return;
    }
    if (text !== current.target || current.position < text.length) {
      const added = text.length - current.target.length;
      const interval = Math.max(200, Math.min(300, now - current.arrival));
      // Catch up within 300ms, including the final chunk; never replay history.
      current.rate = Math.max(80, added * 1000 / interval, (text.length - current.position) / 0.3);
      current.arrival = now;
      current.target = text;
    }
    if (current.frame || current.position >= text.length) return;
    let previous = now;
    const advance = (time: number) => {
      current.position = Math.min(current.target.length, current.position + current.rate * Math.min(32, time - previous) / 1000);
      previous = time;
      let end = Math.floor(current.position);
      // Do not split a Unicode surrogate pair between frames.
      if (end > 0 && /[\uD800-\uDBFF]/.test(current.target[end - 1])) end--;
      current.visible = current.target.slice(0, end);
      setVisible(current.visible);
      current.frame = current.position < current.target.length ? requestAnimationFrame(advance) : 0;
    };
    current.frame = requestAnimationFrame(advance);
  }, [text, streaming, reducedMotion]);

  useLayoutEffect(() => () => {
    cancelAnimationFrame(state.current.frame);
    state.current.frame = 0;
  }, []);
  return visible;
}
