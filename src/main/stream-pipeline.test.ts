import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createStreamPipeline, type RawStreamEvent } from "./stream-pipeline";

function textDelta(delta: string): RawStreamEvent {
  return {
    id: `evt-${delta}`,
    type: "session.text.delta",
    data: { sessionID: "ses_1", assistantMessageID: "msg_1", ordinal: 0, delta }
  };
}

function textEnded(text: string): RawStreamEvent {
  return {
    id: `evt-${text}`,
    type: "session.text.ended",
    data: { sessionID: "ses_1", assistantMessageID: "msg_1", ordinal: 0, text }
  };
}

function stepStarted(): RawStreamEvent {
  return {
    id: "evt-step",
    type: "session.step.started",
    data: { sessionID: "ses_1", assistantMessageID: "msg_1" }
  };
}

function partUpdated(text: string): RawStreamEvent {
  return {
    id: `evt-part-${text}`,
    type: "message.part.updated",
    properties: {
      part: { id: "prt_1", messageID: "msg_1", sessionID: "ses_1", type: "text", text }
    }
  };
}

function partDelta(delta: string): RawStreamEvent {
  return {
    id: `evt-part-d-${delta}`,
    type: "message.part.delta",
    properties: { messageID: "msg_1", partID: "prt_1", field: "text", delta }
  };
}

function statusEvent(type: "busy" | "retry"): RawStreamEvent {
  return {
    id: `evt-status-${type}`,
    type: "session.status",
    properties: {
      sessionID: "ses_1",
      status: type === "busy" ? { type } : { type, attempt: 1, message: "retrying", next: 1 }
    }
  };
}

function idleEvent(): RawStreamEvent {
  return { id: "evt-idle", type: "session.idle", properties: { sessionID: "ses_1" } };
}

function createSubscribe(events: RawStreamEvent[], options: { failFirst?: boolean } = {}): {
  subscribe: (signal: AbortSignal) => Promise<AsyncIterable<RawStreamEvent>>;
  calls: () => number;
} {
  let calls = 0;
  let index = 0;
  const subscribe = async (signal: AbortSignal): Promise<AsyncIterable<RawStreamEvent>> => {
    calls += 1;
    if (options.failFirst && calls === 1) throw new Error("stream failed");
    return {
      [Symbol.asyncIterator](): AsyncIterator<RawStreamEvent> {
        return {
          next: async () => {
            if (signal.aborted) return { done: true, value: undefined };
            if (index >= events.length) {
              await new Promise<void>((resolve) =>
                signal.addEventListener("abort", () => resolve(), { once: true })
              );
              return { done: true, value: undefined };
            }
            return { value: events[index++], done: false };
          }
        };
      }
    };
  };
  return { subscribe, calls: () => calls };
}

describe("createStreamPipeline", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("delivers one ordered batch per flush frame", async () => {
    const delivered: RawStreamEvent[][] = [];
    const { subscribe, calls } = createSubscribe([textDelta("a"), stepStarted(), textDelta("b")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: (_directory, events) => { delivered.push([...events]); }
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(40);
    pipeline.cleanup();
    await run;

    expect(delivered).toHaveLength(1);
    expect(delivered[0].map((event) => event.type)).toEqual([
      "session.text.delta",
      "session.step.started"
    ]);
    expect(calls()).toBe(1);
  });

  it("keeps every event in a batch alive for an async consumer", async () => {
    const delivered: string[] = [];
    const { subscribe } = createSubscribe([textDelta("a"), stepStarted(), textDelta("b")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: async (_directory, events) => {
        await Promise.resolve();
        delivered.push(...events.map((event) => event.type as string));
      }
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(40);
    pipeline.cleanup();
    await run;

    expect(delivered).toEqual(["session.text.delta", "session.step.started"]);
  });

  it("coalesces separated deltas into one event within a flush frame", async () => {
    const delivered: RawStreamEvent[][] = [];
    const { subscribe } = createSubscribe([textDelta("a"), stepStarted(), textDelta("b")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: (_directory, events) => { delivered.push([...events]); }
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(40);
    pipeline.cleanup();
    await run;

    const events = delivered.flat();
    expect(events.map((event) => event.type)).toEqual([
      "session.text.delta",
      "session.step.started"
    ]);
    expect((events[0].data as { delta: string }).delta).toBe("ab");
  });

  it("does not merge deltas across an intervening part snapshot", async () => {
    const delivered: RawStreamEvent[][] = [];
    const { subscribe } = createSubscribe([partUpdated("a"), partDelta("b"), partUpdated("ab"), partDelta("c")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: (_directory, events) => { delivered.push([...events]); }
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(40);
    pipeline.cleanup();
    await run;

    const events = delivered.flat();
    expect(events.map((event) =>
      event.type === "message.part.delta"
        ? `delta:${(event.properties as { delta: string }).delta}`
        : `updated:${((event.properties as { part: { text: string } }).part).text}`
    )).toEqual(["updated:a", "delta:b", "updated:ab", "delta:c"]);
  });

  it("does not merge session text deltas across an authoritative ended snapshot", async () => {
    const delivered: RawStreamEvent[][] = [];
    const { subscribe } = createSubscribe([textDelta("a"), textEnded("a"), textDelta("b")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: (_directory, events) => { delivered.push([...events]); }
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(40);
    pipeline.cleanup();
    await run;

    const events = delivered.flat();
    expect(events.map((event) => event.type)).toEqual([
      "session.text.delta",
      "session.text.ended",
      "session.text.delta"
    ]);
    expect((events[2].data as { delta: string }).delta).toBe("b");
  });

  it("does not coalesce session status across an idle barrier", async () => {
    const delivered: RawStreamEvent[][] = [];
    const { subscribe } = createSubscribe([statusEvent("busy"), idleEvent(), statusEvent("retry")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: (_directory, events) => { delivered.push([...events]); }
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(40);
    pipeline.cleanup();
    await run;

    const events = delivered.flat();
    expect(events.map((event) => event.type)).toEqual(["session.status", "session.idle", "session.status"]);
    expect((events[2].properties as { status: { type: string } }).status.type).toBe("retry");
  });

  it("reconnects after a heartbeat timeout with a fresh subscription", async () => {
    const { subscribe, calls } = createSubscribe([]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {}
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(31_000);
    await vi.advanceTimersByTimeAsync(1);
    pipeline.cleanup();
    await run;

    expect(calls()).toBeGreaterThanOrEqual(2);
  });

  it("reconnects silently when the client wraps the heartbeat abort as a transport error", async () => {
    const onStreamError = vi.fn();
    const errorLog = vi.spyOn(console, "error").mockImplementation(() => {});
    let calls = 0;
    const subscribe = async (signal: AbortSignal): Promise<AsyncIterable<RawStreamEvent>> => {
      calls += 1;
      return {
        [Symbol.asyncIterator](): AsyncIterator<RawStreamEvent> {
          return {
            next: async (): Promise<IteratorResult<RawStreamEvent>> => {
              if (signal.aborted) throw wrappedTransportAbort();
              await new Promise<void>((resolve) =>
                signal.addEventListener("abort", () => resolve(), { once: true })
              );
              throw wrappedTransportAbort();
            }
          };
        }
      };
    };
    const wrappedTransportAbort = (): Error =>
      Object.assign(new Error("Transport"), {
        name: "ClientError",
        cause: new Error("Connection closed after heartbeat timeout")
      });
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {},
      onStreamError
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(31_000);
    await vi.advanceTimersByTimeAsync(31_000);
    pipeline.cleanup();
    await run;
    errorLog.mockRestore();

    expect(calls).toBeGreaterThanOrEqual(3);
    expect(onStreamError).not.toHaveBeenCalled();
    expect(errorLog.mock.calls.some((args) => String(args[0]).includes("stream failed"))).toBe(false);
  });

  it("fires onStreamEnd when the SSE stream ends cleanly", async () => {
    const onStreamEnd = vi.fn();
    let attempt = 0;
    const subscribe = async (signal: AbortSignal): Promise<AsyncIterable<RawStreamEvent>> => {
      const current = attempt++;
      return {
        [Symbol.asyncIterator](): AsyncIterator<RawStreamEvent> {
          let yielded = false;
          return {
            next: async (): Promise<IteratorResult<RawStreamEvent>> => {
              // First attempt: yield one event, then end cleanly.
              if (current === 0 && !yielded) {
                yielded = true;
                return { value: textDelta("a"), done: false };
              }
              if (current === 0) return { done: true, value: undefined };
              // Reconnected attempts stay open until aborted.
              if (!signal.aborted) {
                await new Promise<void>((resolve) =>
                  signal.addEventListener("abort", () => resolve(), { once: true })
                );
              }
              return { done: true, value: undefined };
            }
          };
        }
      };
    };
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {},
      onStreamEnd
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(100);
    await vi.advanceTimersByTimeAsync(100);
    pipeline.cleanup();
    await run;

    expect(onStreamEnd).toHaveBeenCalled();
  });

  it("fires onStreamEnd when the heartbeat aborts a silent stream", async () => {
    const onStreamEnd = vi.fn();
    const { subscribe } = createSubscribe([]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {},
      onStreamEnd
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(31_000);
    await vi.advanceTimersByTimeAsync(1);
    pipeline.cleanup();
    await run;

    expect(onStreamEnd).toHaveBeenCalled();
  });

  it("backs off and retries after a stream failure", async () => {
    const onStreamError = vi.fn();
    const { subscribe, calls } = createSubscribe([], { failFirst: true });
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {},
      onStreamError
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(260);
    pipeline.cleanup();
    await run;

    expect(calls()).toBe(2);
    expect(onStreamError).toHaveBeenCalledOnce();
  });

  it("notifies once per outage when retries fail before any event arrives", async () => {
    const onStreamError = vi.fn();
    const onStreamFailure = vi.fn();
    const onReconnect = vi.fn();
    const subscribe = async (): Promise<AsyncIterable<RawStreamEvent>> => ({
      [Symbol.asyncIterator](): AsyncIterator<RawStreamEvent> {
        return { next: async () => { throw new Error("Transport"); } };
      }
    });
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {},
      onStreamError,
      onStreamFailure,
      onReconnect
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(3_000);
    pipeline.cleanup();
    await run;

    // An endpoint that never delivers an event is one outage: report once,
    // keep every failed attempt visible through the streak, and do not signal
    // a reconnect. Previously every retry cycle reset the accounting and
    // re-notified.
    expect(onStreamError).toHaveBeenCalledTimes(1);
    expect(onStreamFailure.mock.calls.map((call) => call[0])).toEqual([1, 2, 3, 4]);
    expect(onReconnect).not.toHaveBeenCalled();
  });

  it("resets the reported streak when a stream that delivered events drops", async () => {
    const onStreamError = vi.fn();
    const onStreamFailure = vi.fn();
    const onReconnect = vi.fn();
    let calls = 0;
    const subscribe = async (signal: AbortSignal): Promise<AsyncIterable<RawStreamEvent>> => {
      const attempt = calls++;
      const deliverEvent = attempt === 0 || attempt === 2;
      return {
        [Symbol.asyncIterator](): AsyncIterator<RawStreamEvent> {
          let delivered = false;
          return {
            next: async (): Promise<IteratorResult<RawStreamEvent>> => {
              if (signal.aborted) return { done: true, value: undefined };
              if (deliverEvent && !delivered) {
                delivered = true;
                return { value: textDelta(`a${attempt}`), done: false };
              }
              throw new Error("Transport");
            }
          };
        }
      };
    };
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {},
      onStreamError,
      onStreamFailure,
      onReconnect
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(3_000);
    pipeline.cleanup();
    await run;

    // Two live-then-dead cycles are two outages: each one re-notifies, each
    // live stream is signalled for re-materialization, and a delivered event
    // restarts the failure streak ([1,2] then [1,2] …).
    expect(onStreamError).toHaveBeenCalledTimes(2);
    expect(onStreamFailure.mock.calls.map((call) => call[0]).slice(0, 4)).toEqual([1, 2, 1, 2]);
    expect(onReconnect).toHaveBeenCalledTimes(2);
  });

  it("reports malformed stream events instead of dropping them", async () => {
    const onStreamError = vi.fn();
    const { subscribe } = createSubscribe([{}]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: () => {},
      onStreamError
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(1);
    pipeline.cleanup();
    await run;

    expect(onStreamError).toHaveBeenCalledWith("[VELA_STREAM_INVALID_EVENT] Live event stream returned an invalid event");
  });

  it("reports an asynchronous event delivery failure", async () => {
    const onStreamError = vi.fn();
    const { subscribe } = createSubscribe([textDelta("a")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: async () => { throw new Error("renderer bridge failed"); },
      onStreamError
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(40);
    pipeline.cleanup();
    await run;

    expect(onStreamError).toHaveBeenCalledWith("[VELA_EVENT_DELIVERY_FAILED] renderer bridge failed");
  });

  it("flushes queued events on cleanup", async () => {
    const delivered: RawStreamEvent[][] = [];
    const { subscribe } = createSubscribe([textDelta("a")]);
    const pipeline = createStreamPipeline({
      subscribe,
      onEvents: (_directory, events) => { delivered.push([...events]); }
    });

    const run = pipeline.run(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(1);
    pipeline.cleanup();
    await run;

    expect(delivered.flat().map((event) => event.type)).toEqual(["session.text.delta"]);
  });
});
