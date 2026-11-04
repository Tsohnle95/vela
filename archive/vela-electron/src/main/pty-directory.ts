// A PTY spawn with a working directory that no longer exists does not throw.
// node-pty reports an immediate exit code 1 with no output, which reaches the
// user as "Agent TUI exited with code 1" and no cause. Panels outlive their
// folder whenever a workspace is moved or renamed underneath an open panel, so
// resolve a usable directory before spawning and name the path when there is
// none.

export interface PtyDirectoryInput {
  captured: string;
  sessionDirectory: string | null;
  isDirectory: (directory: string) => Promise<boolean>;
}

export async function resolvePtyDirectory(input: PtyDirectoryInput): Promise<string> {
  const { captured, sessionDirectory, isDirectory } = input;
  if (await isDirectory(captured)) return captured;
  if (sessionDirectory && sessionDirectory !== captured && await isDirectory(sessionDirectory)) {
    return sessionDirectory;
  }
  throw new Error(
    `Workspace folder no longer exists: ${captured}. Reopen this panel on the folder's new location.`
  );
}
