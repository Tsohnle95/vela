// @vitest-environment node
import { describe, expect, it } from "vitest";
import { WorkspaceOperationCoordinator } from "./operation-coordinator";

const one = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
const two = { id: "22222222-2222-4222-8222-222222222222", generation: 2 };

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("WorkspaceOperationCoordinator", () => {
  it("serializes write, delete, rename, and create work within a workspace", async () => {
    const coordinator = new WorkspaceOperationCoordinator();
    const gate = deferred();
    const calls: string[] = [];
    const first = coordinator.run(one, async () => { calls.push("write:start"); await gate.promise; calls.push("write:end"); });
    const second = coordinator.run(one, async () => { calls.push("rename"); });
    const third = coordinator.run(one, async () => { calls.push("delete"); });
    await Promise.resolve();
    await Promise.resolve();
    expect(calls).toEqual(["write:start"]);
    gate.resolve();
    await Promise.all([first, second, third]);
    expect(calls).toEqual(["write:start", "write:end", "rename", "delete"]);
  });

  it("does not block independent workspaces", async () => {
    const coordinator = new WorkspaceOperationCoordinator();
    const gate = deferred();
    const calls: string[] = [];
    const blocked = coordinator.run(one, async () => { await gate.promise; calls.push("one"); });
    await coordinator.run(two, async () => { calls.push("two"); });
    expect(calls).toEqual(["two"]);
    gate.resolve();
    await blocked;
  });
});
