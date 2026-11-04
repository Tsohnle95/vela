import { performance } from "node:perf_hooks";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LARGE_SESSION_OUTPUT_CHARS, largeSessionEvents, reduceLargeSession, retainedOutputChars, timelineDomSizeProxy } from "./large-session-fixture";
import { OpenCodeTimeline } from "./components/OpenCodeTimeline";

vi.mock("./store", () => ({
  useStore: () => ({
    agents: [], sessions: [], session: null, reopenSession: vi.fn(), openFile: vi.fn(), replyPermission: vi.fn()
  })
}));

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => {
  document.body.replaceChildren();
});

describe("large session performance fixture", () => {
  it("reports deterministic reducer, derivation, DOM proxy, and retention measurements", () => {
    const events = largeSessionEvents();
    reduceLargeSession();
    const samples = Array.from({ length: 5 }, () => {
      const reduceStart = performance.now();
      const transcript = reduceLargeSession();
      const reducerMs = performance.now() - reduceStart;
      const deriveStart = performance.now();
      const domRows = timelineDomSizeProxy(transcript);
      const retainedChars = retainedOutputChars(transcript);
      const derivedTimelineMs = performance.now() - deriveStart;
      return { transcript, reducerMs, derivedTimelineMs, domRows, retainedChars };
    });
    const median = (values: number[]) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
    const reducerMs = median(samples.map((sample) => sample.reducerMs));
    const derivedTimelineMs = median(samples.map((sample) => sample.derivedTimelineMs));
    const { transcript, domRows, retainedChars } = samples.at(-1)!;

    console.info(JSON.stringify({
      events: events.length,
      transcriptItems: transcript.length,
      samples: samples.length,
      medianReducerMs: Number(reducerMs.toFixed(2)),
      medianDerivedTimelineMs: Number(derivedTimelineMs.toFixed(2)),
      domRows,
      retainedOutputChars: retainedChars
    }));

    expect(transcript).toHaveLength(400);
    expect(domRows).toBe(800);
    expect(reducerMs).toBeLessThan(100);
    expect(derivedTimelineMs).toBeLessThan(10);
    expect(domRows).toBeLessThanOrEqual(1_000);
    expect(retainedChars).toBe(400 * LARGE_SESSION_OUTPUT_CHARS);
  });

  it("constructs the representative timeline React tree and jsdom rows within a generous budget", () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    const transcript = reduceLargeSession();
    const renderStart = performance.now();

    act(() => root.render(
      <OpenCodeTimeline transcript={transcript} busy={false} lastAssistantId={null} />
    ));
    const reactRenderAndDomMs = performance.now() - renderStart;
    const actualDomRows = container.querySelectorAll("[data-timeline-row]").length;

    console.info(JSON.stringify({
      scope: "React reconciliation and jsdom DOM construction; excludes Chromium layout and paint",
      reactRenderAndDomMs: Number(reactRenderAndDomMs.toFixed(2)),
      actualDomRows
    }));
    expect(actualDomRows).toBe(800);
    expect(reactRenderAndDomMs).toBeLessThan(5_000);
    act(() => root.unmount());
  }, 10_000);
});
