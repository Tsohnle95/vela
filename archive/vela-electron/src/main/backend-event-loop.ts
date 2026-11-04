export class BackendEventLoop {
  private generation = 0;
  private controller: AbortController | null = null;
  private running: Promise<void> | null = null;
  private pending: {
    run: (signal: AbortSignal, generation: number) => Promise<void>;
    onError?: (error: unknown) => void;
  } | null = null;

  start(run: (signal: AbortSignal, generation: number) => Promise<void>, onError?: (error: unknown) => void): void {
    if (this.running) {
      if (this.controller?.signal.aborted) this.pending = { run, onError };
      return;
    }
    this.launch(run, onError);
  }

  private launch(run: (signal: AbortSignal, generation: number) => Promise<void>, onError?: (error: unknown) => void): void {
    const generation = ++this.generation;
    const controller = new AbortController();
    this.controller = controller;
    const running = Promise.resolve().then(() => run(controller.signal, generation));
    this.running = running;
    void running.catch((error) => {
      if (!controller.signal.aborted) onError?.(error);
    }).catch(() => {});
    void running.finally(() => {
      if (this.running !== running) return;
      this.running = null;
      this.controller = null;
      const pending = this.pending;
      this.pending = null;
      if (pending) this.launch(pending.run, pending.onError);
    }).catch(() => {});
  }

  current(generation: number): boolean {
    return generation === this.generation && !this.controller?.signal.aborted;
  }

  async stop(): Promise<void> {
    ++this.generation;
    const controller = this.controller;
    const running = this.running;
    this.pending = null;
    controller?.abort();
    await running?.catch(() => {});
  }

  active(): boolean {
    return this.running !== null;
  }
}
