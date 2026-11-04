import { describe, expect, it } from "vitest";
import type { FileWriteIdentity, WorkspaceIdentity } from "@shared/types";
import { EditorPersistence, type SaveSnapshot } from "./editor-persistence";

const workspace = (id: string): WorkspaceIdentity => ({ id, generation: 1 });
const snapshot = (
  id: string,
  path: string,
  content: string,
  revision: number,
  expectedContent = "disk"
): SaveSnapshot => ({ workspace: workspace(id), path, content, expectedContent, revision });

function deferred(): { promise: Promise<void>; resolve: () => void; reject: (error: Error) => void } {
  let resolve!: () => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<void>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

async function flushPromises(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

describe("EditorPersistence", () => {
  it("writes only the snapshot passed to an explicit save", async () => {
    const writes: SaveSnapshot[] = [];
    const persistence = new EditorPersistence(async (value) => { writes.push(value); });
    const value = snapshot("one", "file.ts", "manual", 3);
    await persistence.save(value);

    expect(writes).toEqual([value]);
  });

  it("serializes same-file writes while unrelated completions may reverse", async () => {
    const first = deferred();
    const second = deferred();
    const calls: string[] = [];
    const persistence = new EditorPersistence(async (value) => {
      calls.push(value.content);
      await (value.path === "a" ? first.promise : second.promise);
    });
    const a1 = persistence.save(snapshot("one", "a", "a1", 1));
    const a2 = persistence.save(snapshot("one", "a", "a2", 2));
    const b = persistence.save(snapshot("one", "b", "b", 1));
    await flushPromises();
    expect(calls).toEqual(["a1", "b"]);

    second.resolve();
    await b;
    expect(calls).toEqual(["a1", "b"]);
    first.resolve();
    await Promise.all([a1, a2]);
    expect(calls).toEqual(["a1", "b", "a2"]);
  });

  it("uses a completed write as the next queued write precondition", async () => {
    const first = deferred();
    const identities: FileWriteIdentity[] = [];
    const persistence = new EditorPersistence(async (value, write) => {
      identities.push(write);
      if (value.revision === 1) await first.promise;
    });
    const one = persistence.save(snapshot("one", "file.ts", "one", 1));
    const two = persistence.save(snapshot("one", "file.ts", "two", 2));
    await flushPromises();
    first.resolve();
    await Promise.all([one, two]);

    expect(identities.map((write) => write.expectedContent)).toEqual(["disk", "one"]);
  });

  it("confirms echoes only when write id, workspace, revision, and content match", async () => {
    let identity!: FileWriteIdentity;
    const persistence = new EditorPersistence(async (_value, write) => { identity = write; });
    const value = snapshot("one", "file.ts", "ours", 4);
    await persistence.save(value);

    expect(persistence.classify(value.workspace, {
      workspace: value.workspace,
      sessionID: "session",
      path: value.path,
      baseline: { kind: "known", content: "disk" },
      content: "external",
      deleted: false,
      write: identity
    })).toBe("external");
    expect(persistence.classify(value.workspace, {
      workspace: value.workspace,
      sessionID: "session",
      path: value.path,
      baseline: { kind: "known", content: "disk" },
      content: "ours",
      deleted: false,
      write: { ...identity, revision: 5 }
    })).toBe("external");
    expect(persistence.classify(value.workspace, {
      workspace: value.workspace,
      sessionID: "session",
      path: value.path,
      baseline: { kind: "known", content: "disk" },
      content: "ours",
      deleted: false,
      write: identity
    })).toBe("echo");
    expect(persistence.classify(value.workspace, {
      workspace: value.workspace,
      sessionID: "session",
      path: value.path,
      baseline: { kind: "known", content: "disk" },
      content: "outside",
      deleted: false
    })).toBe("external");
  });

  it("does not report an in-flight save as successful after an external update", async () => {
    const pending = deferred();
    const value = snapshot("one", "file.ts", "ours", 4);
    const persistence = new EditorPersistence(async () => { await pending.promise; });
    const saving = persistence.save(value);
    await flushPromises();
    expect(persistence.classify(value.workspace, {
      workspace: value.workspace,
      sessionID: "session",
      path: value.path,
      baseline: { kind: "known", content: "disk" },
      content: "outside",
      deleted: false
    })).toBe("external");
    pending.resolve();
    await expect(saving).resolves.toBe("cancelled");
  });

  it("reconciles an old write update after close and reopen invalidated its generation", async () => {
    const pending = deferred();
    let identity!: FileWriteIdentity;
    const value = snapshot("one", "file.ts", "old write", 2);
    const persistence = new EditorPersistence(async (_snapshot, write) => {
      identity = write;
      await pending.promise;
    });
    const saving = persistence.save(value);
    await flushPromises();
    persistence.cancelPath(value.workspace, value.path);
    pending.resolve();
    await expect(saving).resolves.toBe("cancelled");

    expect(persistence.classify(value.workspace, {
      workspace: value.workspace,
      sessionID: "session",
      path: value.path,
      baseline: { kind: "known", content: "disk" },
      content: value.content,
      deleted: false,
      write: identity
    })).toBe("external");
  });

  it.each([
    ["close", (p: EditorPersistence) => p.cancelPath(workspace("one"), "dir/file.ts")],
    ["delete", (p: EditorPersistence) => p.cancelPrefix(workspace("one"), "dir")],
    ["rename", (p: EditorPersistence) => p.cancelPrefix(workspace("one"), "dir")],
    ["workspace switch", (p: EditorPersistence) => p.cancelWorkspace(workspace("one"))],
    ["reset or unmount", (p: EditorPersistence) => p.cancelAll()]
  ])("invalidates deferred completion on %s", async (_name, cancel) => {
    const pending = deferred();
    const writes: string[] = [];
    const persistence = new EditorPersistence(async (value) => {
      writes.push(value.content);
      await pending.promise;
    });
    const value = snapshot("one", "dir/file.ts", "content", 1);
    const saving = persistence.save(value);
    await flushPromises();

    cancel(persistence);
    pending.resolve();
    expect(await saving).toBe("cancelled");
    expect(writes).toEqual(["content"]);
  });
});
