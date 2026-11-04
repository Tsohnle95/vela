import type { FileUpdate, FileWriteIdentity, WorkspaceIdentity } from "@shared/types";

export interface SaveSnapshot {
  workspace: WorkspaceIdentity;
  path: string;
  content: string;
  expectedContent: string;
  revision: number;
  overwrite?: boolean;
  standalone?: boolean;
}

export type SaveResult = "saved" | "cancelled";
export type FileUpdateOrigin = "echo" | "external";

type Writer = (snapshot: SaveSnapshot, write: FileWriteIdentity) => Promise<void>;

export class EditorPersistence {
  private queues = new Map<string, Promise<void>>();
  private epochs = new Map<string, number>();
  private expected = new Map<string, FileWriteIdentity & { content: string }>();
  private persisted = new Map<string, string>();
  private conflicts = new Map<string, number>();
  private sequence = 0;

  constructor(private readonly writer: Writer) {}

  async save(snapshot: SaveSnapshot): Promise<SaveResult> {
    const key = this.key(snapshot.workspace, snapshot.path);
    const epoch = this.epochs.get(key) ?? 0;
    const conflict = this.conflicts.get(key) ?? 0;
    const previous = this.queues.get(key) ?? Promise.resolve();
    let result: SaveResult = "cancelled";
    const task = previous.catch(() => {}).then(async () => {
      if ((this.epochs.get(key) ?? 0) !== epoch) return;
      const write = {
        id: `${snapshot.workspace.id}:${++this.sequence}`,
        workspaceID: snapshot.workspace.id,
        revision: snapshot.revision,
        expectedContent: this.persisted.get(key) ?? snapshot.expectedContent,
        overwrite: snapshot.overwrite ?? false
      };
      this.expected.set(key, { ...write, content: snapshot.content });
      try {
        await this.writer(snapshot, write);
      } catch (error) {
        if (this.expected.get(key)?.id === write.id) this.expected.delete(key);
        throw error;
      }
      this.persisted.set(key, snapshot.content);
      if ((this.epochs.get(key) ?? 0) === epoch && (this.conflicts.get(key) ?? 0) === conflict) result = "saved";
    });
    this.queues.set(key, task);
    try {
      await task;
      return result;
    } finally {
      if (this.queues.get(key) === task) this.queues.delete(key);
    }
  }

  classify(workspace: WorkspaceIdentity, update: FileUpdate): FileUpdateOrigin {
    const key = this.key(workspace, update.path);
    if (!update.write) {
      this.conflicts.set(key, (this.conflicts.get(key) ?? 0) + 1);
      if (update.content === null) this.persisted.delete(key);
      else this.persisted.set(key, update.content);
      return "external";
    }
    const expected = this.expected.get(key);
    if (
      expected?.id === update.write.id &&
      expected.workspaceID === update.write.workspaceID &&
      expected.revision === update.write.revision &&
      expected.content === update.content
    ) {
      this.expected.delete(key);
      return "echo";
    }
    this.conflicts.set(key, (this.conflicts.get(key) ?? 0) + 1);
    if (update.content === null) this.persisted.delete(key);
    else this.persisted.set(key, update.content);
    return "external";
  }

  cancelPath(workspace: WorkspaceIdentity, path: string): void {
    this.cancelKey(this.key(workspace, path));
  }

  cancelPrefix(workspace: WorkspaceIdentity, path: string): void {
    const prefix = this.key(workspace, path);
    for (const key of this.keys()) {
      if (key === prefix || key.startsWith(`${prefix}/`)) this.cancelKey(key);
    }
  }

  cancelWorkspace(workspace: WorkspaceIdentity): void {
    const prefix = `${workspace.id}\u0000`;
    for (const key of this.keys()) {
      if (key.startsWith(prefix)) this.cancelKey(key);
    }
  }

  cancelAll(): void {
    for (const key of this.keys()) this.cancelKey(key);
  }

  async idle(workspace: WorkspaceIdentity, path: string): Promise<void> {
    await this.queues.get(this.key(workspace, path))?.catch(() => {});
  }

  private key(workspace: WorkspaceIdentity, path: string): string {
    return `${workspace.id}\u0000${path}`;
  }

  private keys(): Set<string> {
    return new Set([
      ...this.queues.keys(),
      ...this.expected.keys(),
      ...this.persisted.keys(),
      ...this.epochs.keys(),
      ...this.conflicts.keys()
    ]);
  }

  private cancelKey(key: string): void {
    this.expected.delete(key);
    this.persisted.delete(key);
    this.epochs.set(key, (this.epochs.get(key) ?? 0) + 1);
    this.conflicts.delete(key);
  }

}
