import type { VelaBackend } from "../opencode";
import type { FileWriteIdentity, WorkspaceIdentity } from "@shared/types";
import { absoluteFilePath, absoluteFilePaths, fileContent, workspaceId } from "../workspace-security";
import { fileWriteIdentity, movePayload, selectionId, workspacePath } from "../ipc-schema";
import type { TrustedIpcRegistrar } from "./registration";

export function registerFilesIpc(handleTrusted: TrustedIpcRegistrar, backend: VelaBackend): void {

  handleTrusted("shell:open-external", async (_e, workspace: WorkspaceIdentity, file: string) => {
    workspaceId(workspace);
    return backend.resolveExternalOpen(workspace, absoluteFilePath(file));
  });

  handleTrusted("shell:stat-external", async (_e, file: string) =>
    backend.statExternal(absoluteFilePath(file))
  );

  handleTrusted("shell:fs-write-standalone", async (
    _e,
    file: string,
    content: string,
    expectedContent: string,
    overwrite: boolean
  ) =>
    backend.writeStandaloneFile(absoluteFilePath(file), fileContent(content), fileContent(expectedContent), overwrite)
  );

  handleTrusted("shell:fs-import", async (
    _e,
    workspace: WorkspaceIdentity,
    destDir: string,
    sources: string[]
  ) => {
    workspaceId(workspace);
    return backend.importExternal(workspace, workspacePath(workspace, destDir, true).rel, absoluteFilePaths(sources));
  });

  handleTrusted("shell:fs-list", async (_e, workspace: WorkspaceIdentity, rel: string) => {
    const target = workspacePath(workspace, rel, true);
    return backend.listDir(target.workspace, target.rel);
  });

  handleTrusted("shell:fs-reveal", async (_e, workspace: WorkspaceIdentity, rel: string) => {
    const target = workspacePath(workspace, rel, true);
    return backend.revealInFileManager(target.workspace, target.rel);
  });

  handleTrusted("shell:fs-read", async (_e, workspace: WorkspaceIdentity, rel: string) => {
    const target = workspacePath(workspace, rel);
    return backend.readFile(target.workspace, target.rel);
  });

  handleTrusted("shell:fs-write", async (
    _e,
    workspace: WorkspaceIdentity,
    rel: string,
    content: string,
    write: FileWriteIdentity
  ) => {
    const target = workspacePath(workspace, rel);
    return backend.writeFile(target.workspace, target.rel, fileContent(content), fileWriteIdentity(write, target.workspace));
  });

  handleTrusted("shell:fs-create-file", async (_e, workspace: WorkspaceIdentity, rel: string) =>
    backend.createFile(workspace, rel)
  );

  handleTrusted("shell:fs-create-dir", async (_e, workspace: WorkspaceIdentity, rel: string) =>
    backend.createDir(workspace, rel)
  );

  handleTrusted("shell:fs-delete", async (_e, workspace: WorkspaceIdentity, rel: string) =>
    backend.deletePath(workspace, rel)
  );

  handleTrusted("shell:fs-detach", async (_e, workspace: WorkspaceIdentity, rel: string) =>
    backend.detachPath(workspace, rel)
  );

  handleTrusted("shell:fs-rename", async (_e, workspace: WorkspaceIdentity, rel: string, newName: string) =>
    backend.renamePath(workspace, rel, newName)
  );

  handleTrusted("shell:fs-move", async (_e, workspace: WorkspaceIdentity, rel: string, newParent: string) => {
    const target = movePayload(workspace, rel, newParent);
    return backend.movePath(target.workspace, target.rel, target.newParent);
  });

  handleTrusted("shell:recovery-list", async (_e, workspace: WorkspaceIdentity) =>
    backend.listRecovery(workspace)
  );

  handleTrusted("shell:recovery-open", async (_e, workspace: WorkspaceIdentity, id: string) =>
    backend.openRecovery(workspace, selectionId(id, "recovery id"))
  );

  handleTrusted("shell:recovery-acknowledge", async (_e, workspace: WorkspaceIdentity, id: string) =>
    backend.acknowledgeRecovery(workspace, selectionId(id, "recovery id"))
  );
}
