import { describe, expect, it } from "vitest";
import { LatestGeneration, latestOnly, sameWorkspace, SingleFlight } from "./generation";

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("latest generation", () => {
  it("assigns generations at acceptance and makes the latest request win", () => {
    const generations = new LatestGeneration();
    const first = generations.accept();
    const second = generations.accept();

    expect(first).toBe(1);
    expect(second).toBe(2);
    expect(generations.current(first)).toBe(false);
    expect(generations.current(second)).toBe(true);
    expect(generations.snapshot()).toBe(second);
  });

  it("makes a later folder action supersede an earlier unresolved dialog", async () => {
    const generations = new LatestGeneration();
    const firstDialog = deferred<string>();
    const first = generations.accept();
    const firstResult = latestOnly(generations, first, firstDialog.promise);
    const second = generations.accept();
    const secondResult = latestOnly(generations, second, Promise.resolve("second folder"));
    firstDialog.resolve("first folder");
    await expect(firstResult).resolves.toBeUndefined();
    await expect(secondResult).resolves.toBe("second folder");
  });

  it("invalidates accepted startup work", () => {
    const generations = new LatestGeneration();
    const startup = generations.accept();
    generations.invalidate();
    expect(generations.current(startup)).toBe(false);
  });

  it("discards an overlapping activation or reopen that finishes stale", async () => {
    const generations = new LatestGeneration();
    const slow = deferred<string>();
    const first = generations.accept();
    const stale = latestOnly(generations, first, slow.promise);
    const second = generations.accept();
    const current = latestOnly(generations, second, Promise.resolve("second"));
    slow.resolve("first");

    await expect(stale).resolves.toBeUndefined();
    await expect(current).resolves.toBe("second");
  });

  it("checks identity after every watcher await phase", async () => {
    const generations = new LatestGeneration();
    const stat = deferred<string>();
    const generation = generations.accept();
    const mutation = latestOnly(generations, generation, stat.promise);
    generations.accept();
    stat.resolve("old workspace content");
    await expect(mutation).resolves.toBeUndefined();
  });
});

describe("single-flight startup", () => {
  it("prevents parallel subscriptions across repeated macOS reactivation", async () => {
    const flight = new SingleFlight();
    const loop = deferred<void>();
    let starts = 0;
    const start = (): void => flight.start(async () => {
      starts += 1;
      await loop.promise;
    });

    start();
    start();
    start();
    expect(starts).toBe(1);
    expect(flight.active()).toBe(true);
    loop.resolve();
    await loop.promise;
  });
});

describe("workspace response identity", () => {
  it("rejects stale generations even when an id is reused", () => {
    expect(sameWorkspace(
      { id: "11111111-1111-4111-8111-111111111111", generation: 1 },
      { id: "11111111-1111-4111-8111-111111111111", generation: 2 }
    )).toBe(false);
  });
});
