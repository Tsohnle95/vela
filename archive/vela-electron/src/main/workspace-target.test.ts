// @vitest-environment node
import { describe, expect, it } from "vitest";
import { assertPermissionSession, assertWorkspaceTarget, captureWorkspaceTarget } from "./workspace-target";

const workspace = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };

describe("workspace mutation capture", () => {
  it("captures the session target before awaits and rejects a later activation", () => {
    const first = { workspace, sessionID: "one", directory: "/one" };
    const target = captureWorkspaceTarget(workspace, first);
    expect(() => assertWorkspaceTarget(target, first)).not.toThrow();
    expect(() => assertWorkspaceTarget(target, {
      workspace: { id: "22222222-2222-4222-8222-222222222222", generation: 2 },
      sessionID: "two",
      directory: "/two"
    })).toThrow("stale workspace");
  });

  it("rejects a permission target whose session is not the captured active session", () => {
    const target = captureWorkspaceTarget(workspace, { workspace, sessionID: "one", directory: "/one" });
    expect(() => assertPermissionSession(target, "one")).not.toThrow();
    expect(() => assertPermissionSession(target, "child")).toThrow("stale permission session");
  });
});
