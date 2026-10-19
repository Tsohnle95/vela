import { formatFailure, normalizeFailure } from "@shared/errors";

export type RawStreamEvent = Record<string, unknown>;

const FLUSH_FRAME_MS = 33;
const BACKPRESSURE_FLUSH_FRAME_MS = 200;
const BACKPRESSURE_MODE_MS = 10_000;
const STREAM_YIELD_MS = 8;
const DEFAULT_HEARTBEAT_TIMEOUT_MS = 30_000;
const DEFAULT_RECONNECT_DELAY_MS = 250;
const RETRY_BACKOFF_BASE_MS = 250;
const RETRY_BACKOFF_CAP_MS = 5_000;
const RETRY_BACKOFF_MAX_EXPONENT = 8;

type DirectoryQueue = {
  queue: RawStreamEvent[];
  coalesced: Map<string, number>;
  timer: ReturnType<typeof setTimeout> | undefined;
  last: number;
};

type AttemptAbortReason = "pipeline_stopped" | `sse_${string}` | null;

export interface StreamPipelineOptions {
  subscribe: (signal: AbortSignal) => Promise<AsyncIterable<RawStreamEvent>>;
  onEvents: (directory: string, events: RawStreamEvent[]) => void | Promise<void>;
  onStreamError?: (reason: string) => void;
  onStreamFailure?: (consecutiveFailures: number) => void;
  onReconnect?: () => void;
  onStreamEnd?: () => void;
  heartbeatTimeoutMs?: number;
  reconnectDelayMs?: number;
}

export interface StreamPipeline {
  run: (signal: AbortSignal) => Promise<void>;
  cleanup: () => void;
}

function normalizeEventType(payload: RawStreamEvent): RawStreamEvent {
  const type = payload.type;
  if (typeof type !== "string") return payload;
  const match = /^(.*)\.(\d+)$/.exec(type);
  if (!match || !match[1]) return payload;
  return { ...payload, type: match[1] };
}

function resolveEventPayload(payload: RawStreamEvent): RawStreamEvent | null {
  if (!payload || typeof payload !== "object") return null;
  if (typeof payload.type === "string") return payload;
  const inner = payload.payload;
  if (inner && typeof inner === "object" && typeof (inner as RawStreamEvent).type === "string") {
    return inner as RawStreamEvent;
  }
  return null;
}

function resolveEventDirectory(event: RawStreamEvent, payload: RawStreamEvent): string {
  const directory = event.directory;
  if (typeof directory === "string" && directory.length > 0) return directory;

  const location = event.location as { directory?: unknown } | undefined;
  if (typeof location?.directory === "string" && location.directory.length > 0) return location.directory;

  const properties = payload.properties as Record<string, unknown> | undefined;
  if (typeof properties?.directory === "string" && properties.directory.length > 0) return properties.directory;

  const info = properties?.info as Record<string, unknown> | undefined;
  if (typeof info?.directory === "string" && info.directory.length > 0) return info.directory;

  return "global";
}

function envelope(payload: RawStreamEvent): Record<string, unknown> {
  const properties = payload.properties;
  if (properties && typeof properties === "object") return properties as Record<string, unknown>;
  const data = payload.data;
  if (data && typeof data === "object") return data as Record<string, unknown>;
  return {};
}

function stringField(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function key(payload: RawStreamEvent): string | undefined {
  const type = payload.type;
  const data = envelope(payload);
  if (type === "session.status") return `session.status:${stringField(data.sessionID)}`;
  if (type === "session.updated") {
    const info = data.info as Record<string, unknown> | undefined;
    return info?.id ? `session.updated:${info.id}` : undefined;
  }
  if (type === "lsp.updated") return "lsp.updated";
  if (type === "message.part.delta") {
    return `message.part.delta:${stringField(data.messageID)}:${stringField(data.partID)}:${stringField(data.field)}`;
  }
  if (type === "session.text.delta") {
    return `session.text.delta:${stringField(data.sessionID)}:${stringField(data.assistantMessageID)}:${Number(data.ordinal ?? 0)}`;
  }
  if (type === "session.reasoning.delta") {
    return `session.reasoning.delta:${stringField(data.sessionID)}:${stringField(data.assistantMessageID)}:${Number(data.ordinal ?? 0)}`;
  }
  if (type === "session.tool.input.delta") {
    return `session.tool.input.delta:${stringField(data.sessionID)}:${stringField(data.assistantMessageID)}:${stringField(data.callID ?? data.id)}`;
  }
  if (type === "session.compaction.delta") return `session.compaction.delta:${stringField(data.sessionID)}`;
  return undefined;
}

function snapshotBarrierPrefix(payload: RawStreamEvent): string | undefined {
  const type = payload.type;
  if (type === "message.part.updated") {
    const part = (envelope(payload).part ?? (payload.data as Record<string, unknown> | undefined)?.part) as
      | Record<string, unknown>
      | undefined;
    const messageID = stringField(part?.messageID ?? envelope(payload).messageID);
    const partID = stringField(part?.id ?? envelope(payload).partID);
    if (messageID && partID) return `message.part.delta:${messageID}:${partID}:`;
    return undefined;
  }
  const data = envelope(payload);
  const sessionID = stringField(data.sessionID);
  const messageID = stringField(data.assistantMessageID ?? data.messageID);
  if (!sessionID || !messageID) return undefined;
  if (type === "session.text.ended") return `session.text.delta:${sessionID}:${messageID}:${Number(data.ordinal ?? 0)}`;
  if (type === "session.reasoning.ended") return `session.reasoning.delta:${sessionID}:${messageID}:${Number(data.ordinal ?? 0)}`;
  if (type === "session.tool.input.ended" || type === "session.tool.success" || type === "session.tool.failed") {
    return `session.tool.input.delta:${sessionID}:${messageID}:`;
  }
  return undefined;
}

function isAbortError(error: unknown): boolean {
  const direct =
    error instanceof DOMException && error.name === "AbortError" ||
    (typeof error === "object" && error !== null && (error as { name?: string }).name === "AbortError");
  if (direct) return true;
  const cause = (error as { cause?: unknown }).cause;
  return (
    cause instanceof DOMException && cause.name === "AbortError" ||
    (typeof cause === "object" && cause !== null && (cause as { name?: string }).name === "AbortError")
  );
}

export function createStreamPipeline(options: StreamPipelineOptions): StreamPipeline {
  const {
    subscribe,
    onEvents,
    onStreamError,
    onStreamFailure,
    onReconnect,
    onStreamEnd
  } = options;
  const heartbeatTimeoutMs = options.heartbeatTimeoutMs ?? DEFAULT_HEARTBEAT_TIMEOUT_MS;
  const reconnectDelayMs = options.reconnectDelayMs ?? DEFAULT_RECONNECT_DELAY_MS;
  const abort = new AbortController();
  let disconnected = false;

  const directories = new Map<string, DirectoryQueue>();

  const getOrCreateDir = (directory: string): DirectoryQueue => {
    let queue = directories.get(directory);
    if (queue) return queue;
    queue = { queue: [], coalesced: new Map(), timer: undefined, last: 0 };
    directories.set(directory, queue);
    return queue;
  };

  const flushDir = (directory: string): void => {
    const queue = directories.get(directory);
    if (!queue) return;
    if (queue.timer) {
      clearTimeout(queue.timer);
      queue.timer = undefined;
    }
    if (queue.queue.length === 0) return;

    const events = queue.queue;
    queue.queue = [];
    queue.coalesced.clear();

    queue.last = Date.now();
    void Promise.resolve(onEvents(directory, events))
      .catch((error) => onStreamError?.(formatFailure(error, "VELA_EVENT_DELIVERY_FAILED", "Event delivery failed")))
      .finally(() => {
        events.length = 0;
      });
  };

  const flushAll = (): void => {
    for (const directory of directories.keys()) flushDir(directory);
  };

  const scheduleDir = (directory: string): void => {
    const queue = getOrCreateDir(directory);
    if (queue.timer) return;
    const elapsed = Date.now() - queue.last;
    const flushFrameMs = Date.now() < backpressureUntil ? BACKPRESSURE_FLUSH_FRAME_MS : FLUSH_FRAME_MS;
    queue.timer = setTimeout(() => flushDir(directory), Math.max(0, flushFrameMs - elapsed));
  };

  const enqueueEvent = (directory: string, payload: RawStreamEvent): void => {
    const normalizedPayload = normalizeEventType(payload);
    const queue = getOrCreateDir(directory);

    const barrierPrefix = snapshotBarrierPrefix(normalizedPayload);
    if (barrierPrefix) {
      for (const pendingKey of queue.coalesced.keys()) {
        if (pendingKey.startsWith(barrierPrefix)) queue.coalesced.delete(pendingKey);
      }
    }

    if (
      normalizedPayload.type === "session.idle" ||
      normalizedPayload.type === "session.error" ||
      normalizedPayload.type === "session.created" ||
      normalizedPayload.type === "session.deleted"
    ) {
      const data = envelope(normalizedPayload);
      const info = data.info as Record<string, unknown> | undefined;
      const sessionID = stringField(data.sessionID) || stringField(info?.id);
      if (sessionID) {
        queue.coalesced.delete(`session.status:${sessionID}`);
        if (normalizedPayload.type === "session.created" || normalizedPayload.type === "session.deleted") {
          queue.coalesced.delete(`session.updated:${sessionID}`);
        }
      }
    }

    const pendingKey = key(normalizedPayload);
    if (pendingKey) {
      const index = queue.coalesced.get(pendingKey);
      if (index !== undefined) {
        const previous = queue.queue[index];
        const nextEnvelope = envelope(normalizedPayload);
        nextEnvelope.delta = String(envelope(previous).delta ?? "") + String(nextEnvelope.delta ?? "");
        queue.queue[index] = normalizedPayload;
        return;
      }
      queue.coalesced.set(pendingKey, queue.queue.length);
    }

    queue.queue.push(normalizedPayload);
    scheduleDir(directory);
  };

  const wait = (ms: number, signals: AbortSignal[]): Promise<void> => new Promise((resolve) => {
    if (ms <= 0 || signals.some((signal) => signal.aborted)) {
      resolve();
      return;
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    const done = (): void => {
      if (!timer) return;
      clearTimeout(timer);
      timer = undefined;
      for (const signal of signals) signal.removeEventListener("abort", done);
      resolve();
    };
    timer = setTimeout(done, ms);
    for (const signal of signals) signal.addEventListener("abort", done, { once: true });
  });

  const waitForRetry = (ms: number, signals: AbortSignal[]): Promise<void> =>
    wait(ms, signals);

  const computeRetryDelay = (failures: number): number => {
    if (failures <= 0) return 0;
    const exponent = Math.min(failures - 1, RETRY_BACKOFF_MAX_EXPONENT);
    return Math.min(RETRY_BACKOFF_CAP_MS, RETRY_BACKOFF_BASE_MS * 2 ** exponent);
  };

  let streamErrorLogged = false;
  let attempt: AbortController | undefined;
  let heartbeat: ReturnType<typeof setTimeout> | undefined;
  let attemptAbortReason: AttemptAbortReason = null;
  let consecutiveFailures = 0;
  let backpressureUntil = 0;

  const notifyDisconnected = (reason: string): void => {
    if (disconnected) return;
    disconnected = true;
    onStreamError?.(reason);
  };

  const markConnected = (): void => {
    disconnected = false;
    consecutiveFailures = 0;
    onReconnect?.();
  };

  const resetHeartbeat = (): void => {
    if (heartbeat) clearTimeout(heartbeat);
    heartbeat = setTimeout(() => {
      attemptAbortReason = "sse_heartbeat_timeout";
      if (onStreamEnd) onStreamEnd();
      attempt?.abort();
    }, heartbeatTimeoutMs);
  };

  const clearHeartbeat = (): void => {
    if (!heartbeat) return;
    clearTimeout(heartbeat);
    heartbeat = undefined;
  };

  const runAttempt = async (signal: AbortSignal): Promise<void> => {
    const events = await subscribe(signal);

    // `subscribe()` only builds the (lazy) SSE iterable; the stream is
    // established once it actually delivers. Marking connected earlier would
    // reset the failure accounting for an endpoint that never talks, so the
    // outage would never back off, notify once, or trigger recovery.
    let established = false;
    let yielded = Date.now();
    resetHeartbeat();

    for await (const event of events) {
      if (!established) {
        established = true;
        markConnected();
      }
      resetHeartbeat();
      streamErrorLogged = false;

      const payload = resolveEventPayload((event.payload as RawStreamEvent | undefined) ?? event);
      if (!payload) throw Object.assign(new Error("Live event stream returned an invalid event"), { code: "VELA_STREAM_INVALID_EVENT" });
      const directory = resolveEventDirectory(event, payload);
      enqueueEvent(directory, payload);

      if (Date.now() - yielded < STREAM_YIELD_MS) continue;
      yielded = Date.now();
      await wait(0, [signal]);
    }
  };

  const run = async (signal: AbortSignal): Promise<void> => {
    const signals = [signal, abort.signal];
    const aborted = (): boolean => signals.some((item) => item.aborted);
    const stopAttempt = (): void => {
      attemptAbortReason = "pipeline_stopped";
      attempt?.abort();
    };
    signal.addEventListener("abort", stopAttempt);
    abort.signal.addEventListener("abort", stopAttempt);
    try {
      while (!aborted()) {
        attempt = new AbortController();
        const chainAbort = (): void => attempt?.abort();
        signal.addEventListener("abort", chainAbort, { once: true });
        abort.signal.addEventListener("abort", chainAbort, { once: true });
        attemptAbortReason = null;
        let retryDelayMs = reconnectDelayMs;
        try {
          await runAttempt(attempt.signal);
          if (!aborted() && onStreamEnd) onStreamEnd();
        } catch (error) {
          if (attemptAbortReason === "sse_heartbeat_timeout" || attemptAbortReason === "pipeline_stopped") {
            retryDelayMs = 0;
          } else if (!isAbortError(error)) {
            consecutiveFailures += 1;
            // Every failed attempt reports its streak, not just the first one
            // per outage (onStreamError is de-duplicated): consumers use this
            // to decide when the bound endpoint is dead and must be replaced.
            onStreamFailure?.(consecutiveFailures);
            if (!streamErrorLogged) {
              streamErrorLogged = true;
              console.error("[vela] stream failed", error);
            }
            const failure = normalizeFailure(error, "VELA_STREAM_FAILED", "Live event stream failed");
            notifyDisconnected(formatFailure(failure));
            retryDelayMs = computeRetryDelay(consecutiveFailures);
          }
        } finally {
          signal.removeEventListener("abort", chainAbort);
          abort.signal.removeEventListener("abort", chainAbort);
          attempt = undefined;
          clearHeartbeat();
        }

        if (aborted()) return;
        if (attemptAbortReason === "sse_heartbeat_timeout") {
          retryDelayMs = 0;
          attemptAbortReason = null;
        }
        if (retryDelayMs > 0) await waitForRetry(retryDelayMs, signals);
      }
    } finally {
      signal.removeEventListener("abort", stopAttempt);
      abort.signal.removeEventListener("abort", stopAttempt);
      clearHeartbeat();
      flushAll();
    }
  };

  const cleanup = (): void => {
    clearHeartbeat();
    abort.abort();
    flushAll();
  };

  return { run, cleanup };
}
