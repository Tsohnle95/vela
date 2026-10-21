import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { TranscriptGrowth } from "./TranscriptGrowth";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

it.each([false, true])("notifies the scroll owner in the resize delivery and respects reduced motion (%s)", (reduced) => {
  vi.useFakeTimers();
  let resize: ResizeObserverCallback = () => {};
  let frame: FrameRequestCallback = () => {};
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => { frame = callback; return 1; });
  vi.stubGlobal("cancelAnimationFrame", vi.fn());
  const disconnect = vi.fn();
  vi.stubGlobal("ResizeObserver", class {
    constructor(callback: ResizeObserverCallback) { resize = callback; }
    observe() {}
    disconnect = disconnect;
  });
  vi.stubGlobal("matchMedia", () => ({ matches: reduced }));
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  const notify = vi.fn();
  try {
    act(() => root.render(<TranscriptGrowth busy onResize={notify}><p>Text</p></TranscriptGrowth>));
    const outer = host.firstElementChild as HTMLDivElement;
    const inner = outer.firstElementChild!;
    vi.spyOn(inner, "getBoundingClientRect").mockReturnValue({ height: 21 } as DOMRect);
    act(() => resize([], {} as ResizeObserver));
    expect(outer.style.height).toBe("0px");
    act(() => frame(16));
    act(() => resize([], {} as ResizeObserver));
    expect(outer.style.height).toBe("21px");
    expect(outer.style.transition).toBe(reduced ? "none" : "height 120ms linear");
    expect(notify).toHaveBeenCalledTimes(2);
    act(() => root.render(<TranscriptGrowth busy={false} onResize={notify}><p>Final text</p></TranscriptGrowth>));
    act(() => resize([], {} as ResizeObserver));
    expect(notify).toHaveBeenCalledTimes(3);
    expect(outer.style.transition).toBe(reduced ? "none" : "height 120ms linear");
    act(() => vi.advanceTimersByTime(350));
    expect(outer.style.transition).toBe("none");
  } finally {
    act(() => root.unmount()); expect(disconnect).toHaveBeenCalledOnce(); host.remove(); vi.useRealTimers(); vi.restoreAllMocks(); vi.unstubAllGlobals();
  }
});
