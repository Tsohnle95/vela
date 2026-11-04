import fsp from "node:fs/promises";
import type { VelaBackend } from "../opencode";
import type { TerminalManager } from "../terminal";
import type { WorkspaceIdentity } from "@shared/types";
import { confinedPath, terminalDimensions, terminalId, terminalInput } from "../workspace-security";
import { workspacePath } from "../ipc-schema";
import type { TrustedIpcRegistrar } from "./registration";

export function registerTerminalIpc(handleTrusted: TrustedIpcRegistrar, backend: VelaBackend, terminals: TerminalManager): void {

  handleTrusted("shell:terminal-start", async (_e, workspace: WorkspaceIdentity, requestedId: string, rel = "", cols = 100, rows = 24) => {
    const target = workspacePath(workspace, rel, true);
    const root = await backend.workspaceDirectory(target.workspace);
    const directory = await confinedPath(root, target.rel, true);
    const stat = await fsp.stat(directory);
    if (!stat.isDirectory()) throw new Error("terminal path is not a directory");
    const id = terminalId(requestedId);
    const dimensions = terminalDimensions(cols, rows);
    await terminals.start(id, directory, target.workspace, undefined, dimensions);
    try {
      await backend.workspaceDirectory(target.workspace);
    } catch (error) {
      terminals.stop(id, target.workspace);
      throw error;
    }
    return directory;
  });

  handleTrusted("shell:agent-tui-start", async (_e, workspace: WorkspaceIdentity, requestedId: string) => {
    const directory = await backend.ptyDirectory(workspace);
    const command = await backend.tuiCommand(workspace);
    const id = terminalId(requestedId);
    await terminals.start(id, directory, workspace, command);
    try {
      await backend.workspaceDirectory(workspace);
    } catch (error) {
      terminals.stop(id, workspace);
      throw error;
    }
  });

  handleTrusted("shell:terminal-input", async (_e, workspace: WorkspaceIdentity, id: string, data: string) => {
    await backend.workspaceDirectory(workspace);
    terminals.write(terminalId(id), terminalInput(data), workspace);
  });

  handleTrusted("shell:terminal-resize", async (_e, workspace: WorkspaceIdentity, id: string, cols: number, rows: number) => {
    await backend.workspaceDirectory(workspace);
    const dimensions = terminalDimensions(cols, rows);
    terminals.resize(terminalId(id), dimensions.cols, dimensions.rows, workspace);
  });

  handleTrusted("shell:terminal-stop", async (_e, workspace: WorkspaceIdentity, id: string) => {
    await backend.workspaceDirectory(workspace);
    terminals.stop(terminalId(id), workspace);
  });
}
