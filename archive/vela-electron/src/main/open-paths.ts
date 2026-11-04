import { isAbsolute } from "node:path";

const MAX_PENDING_PATHS = 10;

export class PendingOpenPaths {
  private paths: string[] = [];
  private rendererReady = false;

  setRendererReady(ready: boolean): void {
    this.rendererReady = ready;
  }

  takeIfRendererReady(): string[] {
    return this.rendererReady ? this.take() : [];
  }

  push(items: string[]): void {
    for (const item of items) {
      const path = item.trim();
      if (!path || this.paths.includes(path)) continue;
      if (this.paths.length >= MAX_PENDING_PATHS) return;
      this.paths.push(path);
    }
  }

  take(): string[] {
    const pending = this.paths;
    this.paths = [];
    return pending;
  }

  get size(): number {
    return this.paths.length;
  }
}

export function collectLaunchPaths(argv: string[], exists: (path: string) => boolean, executablePath?: string): string[] {
  return argv.filter((arg) => {
    if (!arg || arg.startsWith("-") || arg.startsWith("+")) return false;
    if (!isAbsolute(arg)) return false;
    if (arg === executablePath) return false;
    try {
      return exists(arg);
    } catch {
      return false;
    }
  });
}
