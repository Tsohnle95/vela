import type { WorkspaceIdentity } from "@shared/types";

export class WorkspaceOperationCoordinator {
  private queues = new Map<string, Promise<void>>();

  async run<T>(workspace: WorkspaceIdentity, operation: () => Promise<T>): Promise<T> {
    const key = `${workspace.id}:${workspace.generation}`;
    const previous = this.queues.get(key) ?? Promise.resolve();
    let release!: () => void;
    const turn = new Promise<void>((resolve) => { release = resolve; });
    const queued = previous.catch(() => {}).then(() => turn);
    this.queues.set(key, queued);
    await previous.catch(() => {});
    try {
      return await operation();
    } finally {
      release();
      if (this.queues.get(key) === queued) this.queues.delete(key);
    }
  }
}
