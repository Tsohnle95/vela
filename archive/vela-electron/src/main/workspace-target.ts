import type { WorkspaceIdentity } from "@shared/types";
import { assertWorkspace } from "./workspace-security";

export interface ActiveWorkspaceTarget {
  workspace: WorkspaceIdentity;
  sessionID: string;
  directory: string;
}

export function captureWorkspaceTarget(
  expected: WorkspaceIdentity,
  active: { workspace: WorkspaceIdentity | null; sessionID: string | null; directory: string | null }
): ActiveWorkspaceTarget {
  const workspace = assertWorkspace(expected, active.workspace);
  if (!active.sessionID || !active.directory) throw new Error("no active session");
  return { workspace, sessionID: active.sessionID, directory: active.directory };
}

export function assertWorkspaceTarget(
  target: ActiveWorkspaceTarget,
  active: { workspace: WorkspaceIdentity | null; sessionID: string | null; directory: string | null }
): void {
  assertWorkspace(target.workspace, active.workspace);
  if (target.sessionID !== active.sessionID || target.directory !== active.directory) throw new Error("stale workspace");
}

export function assertPermissionSession(target: ActiveWorkspaceTarget, sessionID: string): void {
  if (sessionID !== target.sessionID) throw new Error("stale permission session");
}
