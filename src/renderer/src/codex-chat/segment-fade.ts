/**
 * Codex's streamed-text reveal, ported from the ChatGPT/Codex webview bundle
 * (`K_d`/`q_d`/`Y_d`/`$_d`/`evd` in `app-initial-*.js`).
 *
 * Each newly rendered word segment fades in. Segments are scheduled on one
 * shared 16ms step so a burst of words staggers out instead of popping at once,
 * and the schedule is capped 96ms ahead of "now" so a long burst never lags
 * behind the stream. Once a segment's animation ends it is settled and never
 * animates again, including across re-renders of the growing message.
 */

export const FADE_SEGMENT_DELAY_MS = 16;
export const FADE_SEGMENT_MAX_DELAY_MS = 96;

export interface FadeTimeline {
  delayMsByKey: Map<string, number>;
  startAtMsByKey: Map<string, number>;
  segmentKeyByElement: WeakMap<HTMLElement, string>;
  nextSegmentStartAtMs: number;
  settledKeys: Set<string>;
}

export function createFadeTimeline(): FadeTimeline {
  return {
    delayMsByKey: new Map(),
    startAtMsByKey: new Map(),
    segmentKeyByElement: new WeakMap(),
    nextSegmentStartAtMs: 0,
    settledKeys: new Set()
  };
}

export interface AttachSegmentOptions {
  element: HTMLElement;
  fadeSegmentDelayMs: number;
  fadeSegmentMaxDelayMs: number;
  segmentKey: string;
  timeline: FadeTimeline;
}

/**
 * Assign the segment its slot on the shared schedule and publish the resulting
 * delay through `--fade-delay`, which the fade CSS consumes.
 */
export function attachSegment({
  element,
  fadeSegmentDelayMs,
  fadeSegmentMaxDelayMs,
  segmentKey,
  timeline
}: AttachSegmentOptions): void {
  if (timeline.segmentKeyByElement.get(element) === segmentKey) return;
  const now = performance.now();
  const existingStart = timeline.startAtMsByKey.get(segmentKey);
  const scheduledStart = existingStart ?? Math.min(
    Math.max(timeline.nextSegmentStartAtMs, now),
    Number.isFinite(fadeSegmentMaxDelayMs) ? now + fadeSegmentMaxDelayMs : Number.POSITIVE_INFINITY
  );
  const delay = Math.round(scheduledStart - now);
  if (existingStart === undefined) {
    timeline.startAtMsByKey.set(segmentKey, scheduledStart);
    timeline.delayMsByKey.set(segmentKey, delay);
    timeline.nextSegmentStartAtMs = scheduledStart + fadeSegmentDelayMs;
  }
  timeline.segmentKeyByElement.set(element, segmentKey);
  element.style.setProperty("--fade-delay", `${delay}ms`);
}

/** Delay used before a segment has been observed by `attachSegment`. */
export function segmentDelay(
  segmentIndex: number,
  fadeSegmentDelayMs: number,
  fadeSegmentMaxDelayMs: number
): number {
  return Math.min(segmentIndex * fadeSegmentDelayMs, fadeSegmentMaxDelayMs);
}

function isWordChar(code: number): boolean {
  return (code >= 48 && code <= 57) || (code >= 65 && code <= 90) || (code >= 97 && code <= 122);
}

function isAscii(text: string): boolean {
  for (let index = 0; index < text.length; index += 1) {
    if (text.charCodeAt(index) > 127) return false;
  }
  return true;
}

/**
 * ASCII text splits into alphanumeric runs, with punctuation and whitespace
 * glued onto the preceding run. Non-ASCII text falls back to whitespace-led
 * word tokens so scripts without ASCII word boundaries still get word-sized
 * segments.
 */
export function splitSegments(text: string): string[] {
  if (isAscii(text)) return splitAsciiSegments(text);
  const matched = text.match(/\s*\S+(?:\s+|$)/g) ?? [];
  return matched.length > 0 || text.length === 0 ? matched : [text];
}

function splitAsciiSegments(text: string): string[] {
  const segments: string[] = [];
  let index = 0;
  while (index < text.length) {
    if (isWordChar(text.charCodeAt(index))) {
      const start = index;
      while (index < text.length && isWordChar(text.charCodeAt(index))) index += 1;
      segments.push(text.slice(start, index));
      continue;
    }
    const target = Math.max(segments.length - 1, 0);
    segments[target] ??= "";
    segments[target] += text[index];
    index += 1;
  }
  return segments;
}
