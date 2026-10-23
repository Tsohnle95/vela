/**
 * Codex's adaptive streaming buffer, ported from `Gpd` in the ChatGPT/Codex
 * webview bundle (`app-initial-*.js`).
 *
 * While a response streams, Codex does not paint raw network chunks. It feeds
 * the accumulated text through this buffer, which reveals characters at an
 * adaptive rate: the rate tracks arrival speed, then drains toward the target
 * on a 50ms tick and publishes on `requestAnimationFrame`. The result is one
 * continuous reveal instead of markdown re-flowing in bursts.
 *
 * The rate controller is intentionally reproduced as-is, including its
 * remaining-buffer feedback loop, because the feel depends on those constants.
 */

export interface StreamingBufferSnapshot {
  source: string;
  streamId: string;
  text: string;
  isComplete: boolean;
}

const DRAIN_TICK = 0.05;
const DRAIN_INTERVAL_MS = 50;

export class StreamingTextBuffer {
  private source: string;
  private streamId: string;
  private isActivelyStreaming: boolean;
  private bufferedLength: number;
  private snapshot: StreamingBufferSnapshot;
  private listeners = new Set<() => void>();
  private frame: number | undefined;
  private timeout: number | undefined;
  private arrivalIntervalFloor = 10;
  private rate = 1;
  private lastArrival: number | null = null;
  private lastDrain: number | null = null;
  private drainCarry = 0;
  private lastTargetLength: number;

  constructor(source: string, streamId: string, isActivelyStreaming: boolean) {
    this.source = source;
    this.streamId = streamId;
    this.isActivelyStreaming = isActivelyStreaming;
    this.bufferedLength = source.length;
    this.lastTargetLength = source.length;
    this.snapshot = { source, streamId, text: source, isComplete: !isActivelyStreaming };
  }

  getSnapshot = (): StreamingBufferSnapshot => this.snapshot;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  update(text: string, streamId: string, isActivelyStreaming: boolean, arrivalIntervalFloor: number): void {
    const reset = streamId !== this.streamId
      || !text.startsWith(this.source)
      || (!this.isActivelyStreaming && isActivelyStreaming);
    this.source = text;
    this.streamId = streamId;
    this.isActivelyStreaming = isActivelyStreaming;
    this.arrivalIntervalFloor = arrivalIntervalFloor;
    if (reset) {
      this.destroy();
      this.bufferedLength = text.length;
      this.rate = 1;
      this.lastArrival = null;
      this.lastDrain = null;
      this.drainCarry = 0;
      this.lastTargetLength = text.length;
      this.publish();
      return;
    }
    if (isActivelyStreaming) {
      if (this.shouldUpdate()) this.updateBufferedState();
      return;
    }
    this.destroy();
    this.bufferedLength = text.length;
    this.publish();
  }

  destroy(): void {
    window.clearTimeout(this.timeout);
    this.timeout = undefined;
    if (this.frame !== undefined) {
      window.cancelAnimationFrame(this.frame);
      this.frame = undefined;
    }
  }

  private shouldUpdate(): boolean {
    return this.bufferedLength < this.source.length || this.snapshot.isComplete !== this.isComplete();
  }

  private isComplete(): boolean {
    return !this.isActivelyStreaming && this.bufferedLength === this.source.length;
  }

  private updateBufferedState(): void {
    const now = performance.now() / 1000;
    const arrived = this.source.length - this.lastTargetLength;
    this.lastTargetLength = this.source.length;
    if (arrived > 0) {
      if (this.lastArrival === null) {
        this.rate = arrived;
      } else {
        const interval = Math.max(now - this.lastArrival, this.arrivalIntervalFloor);
        const observed = arrived / interval;
        this.rate = Math.max(1, 0.1 * observed + 0.9 * this.rate);
      }
      this.lastArrival = now;
    }
    const elapsed = this.lastDrain === null ? 0 : now - this.lastDrain;
    this.lastDrain = now;
    const remaining = this.source.length - this.bufferedLength;
    const feedbackSteps = (elapsed + this.drainCarry) / DRAIN_TICK;
    for (let step = 0; step < feedbackSteps; step += 1) {
      if (remaining !== 0 && remaining < this.rate * 1.5) {
        this.rate = (remaining + this.rate * 1.5) / 2;
      }
      if (remaining > this.rate / 1.2) {
        this.rate = (remaining + this.rate / 1.2) / 2;
      }
    }
    this.rate = Math.max(1, this.rate);
    const advance = elapsed * this.rate + this.drainCarry;
    const whole = Math.floor(advance);
    this.drainCarry = advance - whole;
    if (whole >= 1) {
      this.bufferedLength = Math.min(this.bufferedLength + whole, this.source.length);
    }
    if (this.bufferedLength === this.source.length) {
      this.lastDrain = null;
      this.drainCarry = 0;
    }
    if (this.snapshot.text.length !== this.bufferedLength || this.snapshot.isComplete !== this.isComplete()) {
      this.frame ??= window.requestAnimationFrame(() => {
        this.frame = undefined;
        this.publish();
      });
    }
    this.timeout ??= window.setTimeout(() => {
      this.timeout = undefined;
      if (this.shouldUpdate()) this.updateBufferedState();
    }, DRAIN_INTERVAL_MS);
  }

  private publish(): void {
    const text = this.source.slice(0, this.bufferedLength);
    const complete = this.isComplete();
    if (
      text === this.snapshot.text
      && this.source === this.snapshot.source
      && this.streamId === this.snapshot.streamId
      && complete === this.snapshot.isComplete
    ) return;
    this.snapshot = { source: this.source, streamId: this.streamId, text, isComplete: complete };
    for (const listener of this.listeners) listener();
  }
}
