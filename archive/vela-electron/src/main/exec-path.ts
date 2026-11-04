import os from "node:os";
import path from "node:path";

// GUI-launched macOS/Linux apps inherit a minimal PATH from the window server
// (macOS launches them with `/usr/bin:/bin:/usr/sbin:/sbin`), so user-installed
// runtimes like `opencode` (~/.opencode/bin) are invisible to `execFile` probes
// and `node-pty` spawns even though the app's own service connection works.
// Augment PATH with the conventional user tool directories so runtime probing,
// the OpenCode service fallback, the embedded TUI, and the terminal tray all
// resolve the same executables the user's shell would.
const POSIX_HOME_DIRS = [
  [".local", "bin"],
  ["bin"],
  [".bun", "bin"],
  [".deno", "bin"],
  [".cargo", "bin"],
  [".volta", "bin"],
  [".npm-global", "bin"],
  [".local", "share", "pnpm"],
  [".opencode", "bin"],
  ["Library", "pnpm"]
] as const;

const POSIX_SYSTEM_DIRS = ["/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin", "/usr/local/sbin"];

export function augmentedPath(
  current: string | undefined,
  home: string = os.homedir(),
  platform: NodeJS.Platform = process.platform
): string {
  const existing = (current ?? "").split(path.delimiter).filter(Boolean);
  // Windows GUI processes inherit the user's PATH, so leave it untouched.
  if (platform === "win32") return existing.join(path.delimiter);
  const additions = [
    ...POSIX_HOME_DIRS.map((parts) => path.join(home, ...parts)),
    ...POSIX_SYSTEM_DIRS
  ];
  const seen = new Set(existing);
  const prepended = additions.filter((dir) => {
    if (seen.has(dir)) return false;
    seen.add(dir);
    return true;
  });
  return [...prepended, ...existing].join(path.delimiter);
}

export function applyExecPath(): void {
  process.env.PATH = augmentedPath(process.env.PATH);
}
