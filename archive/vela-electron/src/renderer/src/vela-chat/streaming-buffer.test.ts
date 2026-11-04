import { afterEach, describe, expect, it, vi } from "vitest";
import { StreamingTextBuffer } from "./streaming-buffer";

describe("StreamingTextBuffer", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows the current text immediately on construction", () => {
    const buffer = new StreamingTextBuffer("hello", "s1", true);
    expect(buffer.getSnapshot().text).toBe("hello");
    expect(buffer.getSnapshot().isComplete).toBe(false);
    buffer.destroy();
  });

  it("snaps to the full text when streaming stops", () => {
    const buffer = new StreamingTextBuffer("hello", "s1", true);
    buffer.update("hello world", "s1", false, 10);
    expect(buffer.getSnapshot().text).toBe("hello world");
    expect(buffer.getSnapshot().isComplete).toBe(true);
  });

  it("resets when the stream identity changes", () => {
    const buffer = new StreamingTextBuffer("hello", "s1", true);
    buffer.update("other", "s2", true, 10);
    expect(buffer.getSnapshot().text).toBe("other");
    expect(buffer.getSnapshot().streamId).toBe("s2");
  });

  it("drains toward the target instead of jumping to it", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "requestAnimationFrame", "performance"] });
    const buffer = new StreamingTextBuffer("a", "s1", true);
    buffer.update("a".repeat(200), "s1", true, 10);
    const before = buffer.getSnapshot().text.length;
    vi.advanceTimersByTime(400);
    const after = buffer.getSnapshot().text.length;
    expect(after).toBeGreaterThan(before);
    expect(after).toBeLessThanOrEqual(200);
    buffer.destroy();
  });

  it("notifies subscribers only when the revealed text changes", () => {
    const buffer = new StreamingTextBuffer("hello", "s1", true);
    const listener = vi.fn();
    const unsubscribe = buffer.subscribe(listener);
    buffer.update("hello", "s1", true, 10);
    expect(listener).not.toHaveBeenCalled();
    buffer.update("hello!", "s1", false, 10);
    expect(listener).toHaveBeenCalledTimes(1);
    unsubscribe();
    buffer.destroy();
  });
});
