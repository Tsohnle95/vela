import { describe, expect, it } from "vitest";
import {
  attachSegment,
  beginSegmentCachePass,
  cachedSegments,
  createFadeTimeline,
  createSegmentCache,
  pruneSegmentCache,
  FADE_SEGMENT_DELAY_MS,
  FADE_SEGMENT_MAX_DELAY_MS,
  segmentDelay,
  splitSegments
} from "./segment-fade";

describe("splitSegments", () => {
  it("treats a word plus its trailing punctuation as one segment", () => {
    expect(splitSegments("Hello, world!")).toEqual(["Hello, ", "world!"]);
  });

  it("splits ASCII words without dropping whitespace", () => {
    expect(splitSegments("a b")).toEqual(["a ", "b"]);
    expect(splitSegments("")).toEqual([]);
  });

  it("falls back to whitespace-led words for non-ASCII text", () => {
    expect(splitSegments("你好世界")).toEqual(["你好世界"]);
    expect(splitSegments("élever un chat")).toEqual(["élever ", "un ", "chat"]);
  });
});

describe("segmentDelay", () => {
  it("steps by the configured delay and clamps at the cap", () => {
    expect(segmentDelay(0, FADE_SEGMENT_DELAY_MS, FADE_SEGMENT_MAX_DELAY_MS)).toBe(0);
    expect(segmentDelay(3, FADE_SEGMENT_DELAY_MS, FADE_SEGMENT_MAX_DELAY_MS)).toBe(48);
    expect(segmentDelay(10, FADE_SEGMENT_DELAY_MS, FADE_SEGMENT_MAX_DELAY_MS)).toBe(96);
  });
});

describe("attachSegment", () => {
  it("schedules consecutive segments 16ms apart and publishes --fade-delay", () => {
    const timeline = createFadeTimeline();
    const first = document.createElement("span");
    const second = document.createElement("span");

    attachSegment({
      element: first,
      fadeSegmentDelayMs: FADE_SEGMENT_DELAY_MS,
      fadeSegmentMaxDelayMs: FADE_SEGMENT_MAX_DELAY_MS,
      segmentKey: "a",
      timeline
    });
    attachSegment({
      element: second,
      fadeSegmentDelayMs: FADE_SEGMENT_DELAY_MS,
      fadeSegmentMaxDelayMs: FADE_SEGMENT_MAX_DELAY_MS,
      segmentKey: "b",
      timeline
    });

    expect(first.style.getPropertyValue("--fade-delay")).toBe("0ms");
    const secondDelay = Number.parseInt(second.style.getPropertyValue("--fade-delay"), 10);
    expect(secondDelay).toBeGreaterThanOrEqual(5);
    expect(secondDelay).toBeLessThanOrEqual(17);
    expect(timeline.delayMsByKey.get("b")).toBe(secondDelay);
  });

  it("caps the schedule at the maximum delay", () => {
    const timeline = createFadeTimeline();
    timeline.nextSegmentStartAtMs = performance.now() + 5_000;
    const element = document.createElement("span");
    attachSegment({
      element,
      fadeSegmentDelayMs: FADE_SEGMENT_DELAY_MS,
      fadeSegmentMaxDelayMs: FADE_SEGMENT_MAX_DELAY_MS,
      segmentKey: "far",
      timeline
    });
    const delay = Number.parseInt(element.style.getPropertyValue("--fade-delay"), 10);
    expect(delay).toBeLessThanOrEqual(96);
    expect(delay).toBeGreaterThan(90);
  });

  it("does not reschedule an element that already carries the segment", () => {
    const timeline = createFadeTimeline();
    const element = document.createElement("span");
    const options = {
      element,
      fadeSegmentDelayMs: FADE_SEGMENT_DELAY_MS,
      fadeSegmentMaxDelayMs: FADE_SEGMENT_MAX_DELAY_MS,
      segmentKey: "a",
      timeline
    };
    attachSegment(options);
    element.style.setProperty("--fade-delay", "fixed");
    attachSegment(options);
    expect(element.style.getPropertyValue("--fade-delay")).toBe("fixed");
  });
});


describe("cachedSegments", () => {
  it("returns the same array identity for the same text", () => {
    const cache = createSegmentCache();
    const first = cachedSegments(cache, "hello world");
    const second = cachedSegments(cache, "hello world");
    expect(second).toBe(first);
    expect(first).toEqual(["hello ", "world"]);
  });

  it("bypasses the cache for text above the size bound", () => {
    const cache = createSegmentCache();
    const long = "a ".repeat(3000);
    const first = cachedSegments(cache, long);
    const second = cachedSegments(cache, long);
    expect(second).not.toBe(first);
    expect(cache.byText.has(long)).toBe(false);
  });

  it("prunes entries unused by the current pass", () => {
    const cache = createSegmentCache();
    cachedSegments(cache, "first");
    beginSegmentCachePass(cache);
    cachedSegments(cache, "second");
    pruneSegmentCache(cache);
    expect(cache.byText.has("first")).toBe(false);
    expect(cache.byText.has("second")).toBe(true);
  });
});
