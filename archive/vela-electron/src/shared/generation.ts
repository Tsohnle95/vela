import type { WorkspaceIdentity } from "./types";

export class LatestGeneration {
  private value = 0;

  accept(requested?: number): number {
    if (requested !== undefined) {
      if (!Number.isSafeInteger(requested) || requested < 1) throw new Error("invalid activation generation");
      this.value = Math.max(this.value + 1, requested);
      return this.value;
    }
    return ++this.value;
  }

  current(generation: number): boolean {
    return generation === this.value;
  }

  snapshot(): number {
    return this.value;
  }

  invalidate(): void {
    this.value += 1;
  }
}

export function sameWorkspace(
  expected: WorkspaceIdentity | null | undefined,
  actual: WorkspaceIdentity | null | undefined
): boolean {
  return Boolean(
    expected && actual && expected.id === actual.id && expected.generation === actual.generation
  );
}

export async function latestOnly<T>(
  generations: LatestGeneration,
  generation: number,
  work: Promise<T>
): Promise<T | undefined> {
  const value = await work;
  return generations.current(generation) ? value : undefined;
}

export class SingleFlight {
  private running: Promise<void> | null = null;

  start(run: () => Promise<void>): void {
    if (this.running) return;
    this.running = run().finally(() => {
      this.running = null;
    });
  }

  active(): boolean {
    return this.running !== null;
  }
}
