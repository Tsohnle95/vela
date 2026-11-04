// @vitest-environment node
import { describe, expect, it, vi } from "vitest";
import { BackendEventLoop } from "./backend-event-loop";

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

async function turn(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe("BackendEventLoop", () => {
  it("reports unexpected run failures", async () => {
    const loop = new BackendEventLoop();
    const error = new Error("event loop failed");
    const onError = vi.fn();
    loop.start(async () => { throw error; }, onError);
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
    expect(onError).toHaveBeenCalledWith(error);
    expect(loop.active()).toBe(false);
  });

  it("deduplicates starts until a completed run releases the lifecycle", async () => {
    const loop = new BackendEventLoop();
    const subscription = deferred();
    let subscriptions = 0;
    const start = (): void => loop.start(async () => {
      subscriptions += 1;
      await subscription.promise;
    });
    start();
    start();
    start();
    await turn();
    expect(subscriptions).toBe(1);
    expect(loop.active()).toBe(true);
    subscription.resolve();
    await turn();
    expect(loop.active()).toBe(false);
  });

  it("waits for an aborted run to settle", async () => {
    const loop = new BackendEventLoop();
    const exited = deferred();
    const release = deferred();
    loop.start(async (signal) => {
      await new Promise<void>((resolve) => signal.addEventListener("abort", () => resolve(), { once: true }));
      await release.promise;
      exited.resolve();
    });
    await turn();
    const stopping = loop.stop();
    expect(loop.active()).toBe(true);
    release.resolve();
    await stopping;
    await exited.promise;
    expect(loop.active()).toBe(false);
  });

  it("invalidates streaming work before restart and never revives the old generation", async () => {
    const loop = new BackendEventLoop();
    const firstReady = deferred();
    const firstExit = deferred();
    const seen: string[] = [];
    let concurrent = 0;
    let maximum = 0;
    let firstGeneration = 0;
    loop.start(async (signal, generation) => {
      concurrent += 1;
      maximum = Math.max(maximum, concurrent);
      firstGeneration = generation;
      firstReady.resolve();
      await new Promise<void>((resolve) => signal.addEventListener("abort", () => resolve(), { once: true }));
      if (loop.current(generation)) seen.push("stale");
      concurrent -= 1;
      firstExit.resolve();
    });
    await firstReady.promise;
    const stopping = loop.stop();
    expect(loop.current(firstGeneration)).toBe(false);

    const restarted = deferred();
    loop.start(async (_signal, generation) => {
      concurrent += 1;
      maximum = Math.max(maximum, concurrent);
      expect(loop.current(generation)).toBe(true);
      seen.push("fresh");
      restarted.resolve();
      concurrent -= 1;
    });
    await stopping;
    await restarted.promise;
    await firstExit.promise;
    await turn();
    expect(seen).toEqual(["fresh"]);
    expect(maximum).toBe(1);
    expect(loop.active()).toBe(false);
  });
});
