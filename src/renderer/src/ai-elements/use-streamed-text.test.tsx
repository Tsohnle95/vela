import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useStreamedText } from "./use-streamed-text";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let host: HTMLDivElement;
let root: Root;
let time: number;
let frames: Map<number, FrameRequestCallback>;
function Text({ text, streaming = true }: { text: string; streaming?: boolean }) {
  return <p>{useStreamedText(text, streaming)}</p>;
}
function render(text: string, streaming = true) {
  act(() => root.render(<Text text={text} streaming={streaming}/>));
}
function frame() {
  time += 16;
  const callbacks = [...frames.values()];
  frames.clear();
  act(() => callbacks.forEach(callback => callback(time)));
  return host.textContent!;
}
beforeEach(() => {
  time = 0; frames = new Map(); let id = 0;
  vi.spyOn(performance, "now").mockImplementation(() => time);
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => { frames.set(++id, callback); return id; });
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
  vi.stubGlobal("matchMedia", () => ({ matches: false }));
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); host.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });

it("spreads irregular bursts over frames and drains the final chunk without a jump", () => {
  const source = "A uniformly paced response with a substantial provider burst. ".repeat(8);
  render("");
  render(source.slice(0, 150));
  const lengths = Array.from({ length: 10 }, () => frame().length);
  expect(new Set(lengths).size).toBe(10);
  expect(lengths[9]).toBeLessThan(150);
  render(source);
  const beforeCompletion = host.textContent!.length;
  render(source, false);
  expect(host.textContent!.length).toBe(beforeCompletion);
  for (let n = 0; n < 22; n++) frame();
  expect(host.textContent).toBe(source);
  expect(frames.size).toBe(0);
});
it("does not animate restored history or replay a corrected snapshot", () => {
  render("History is already complete", false);
  expect(host.textContent).toBe("History is already complete");
  render("History is already complete and now expanding", true); frame();
  render("A corrected authoritative snapshot", true);
  expect(host.textContent).toBe("A corrected authoritative snapshot");
  expect(frames.size).toBe(0);
});
it("respects reduced motion and cancels pending work on unmount", () => {
  vi.stubGlobal("matchMedia", () => ({ matches: true }));
  render("Immediate text");
  expect(host.textContent).toBe("Immediate text");
  expect(frames.size).toBe(0);
  vi.stubGlobal("matchMedia", () => ({ matches: false }));
  render("Immediate text with another incoming burst");
  expect(frames.size).toBe(1);
  act(() => root.render(null));
  expect(frames.size).toBe(0);
});

it("does not replay an in-progress transcript when its panel remounts", () => {
  render("Existing text from an already active session", true);
  expect(host.textContent).toBe("Existing text from an already active session");
  expect(frames.size).toBe(0);
  render("Existing text from an already active session with new tokens", true);
  expect(host.textContent).toBe("Existing text from an already active session");
  expect(frames.size).toBe(1);
});
