export interface TerminalView {
  id: string;
  name: string;
  defaultName?: boolean;
}

export interface TerminalTabs {
  terms: TerminalView[];
  activeId: string | null;
}

export interface TerminalSize {
  cols: number;
  rows: number;
}

export function terminalSizeForViewport(
  width: number,
  height: number,
  cellWidth: number,
  cellHeight: number
): TerminalSize {
  // Match FitAddon's 16px host padding + 14px overview ruler, then leave one
  // extra column so the shell's first prompt cannot wrap before the first resize.
  return {
    cols: Math.max(2, Math.floor((width - 30) / cellWidth) - 1),
    rows: Math.max(1, Math.floor((height - 12) / cellHeight))
  };
}

export function terminalDirectoryCommand(platform: string, workspaceDirectory: string, relativeDirectory: string): string | null {
  if (!relativeDirectory) return null;
  if (platform === "win32") {
    const absolute = `${workspaceDirectory.replace(/[\\/]+$/, "")}\\${relativeDirectory.replaceAll("/", "\\")}`;
    return `Set-Location -LiteralPath '${absolute.replaceAll("'", "''")}'\r`;
  }
  const absolute = `${workspaceDirectory.replace(/\/+$/, "")}/${relativeDirectory}`;
  return `cd -- '${absolute.replaceAll("'", "'\\''")}'\r`;
}

export function removeTerminal(tabs: TerminalTabs, id: string): TerminalTabs {
  const index = tabs.terms.findIndex((term) => term.id === id);
  if (index === -1) return tabs;
  const terms = tabs.terms.filter((term) => term.id !== id);
  if (tabs.activeId !== id) return { terms, activeId: tabs.activeId };
  return { terms, activeId: terms[Math.min(index, terms.length - 1)]?.id ?? null };
}

interface BufferedOutput {
  chunks: string[];
  bytes: number;
  expiresAt: number;
}

export class PendingTerminalOutput {
  private readonly pending = new Set<string>();
  private readonly buffers = new Map<string, BufferedOutput>();
  private readonly encoder = new TextEncoder();

  constructor(
    private readonly maxBytes = 256 * 1024,
    private readonly maxChunks = 64,
    private readonly maxAgeMs = 10_000,
    private readonly now = () => Date.now()
  ) {}

  awaitRegistration(id: string): void {
    this.cleanup();
    this.pending.add(id);
    this.buffers.set(id, { chunks: [], bytes: 0, expiresAt: this.now() + this.maxAgeMs });
  }

  write(id: string, data: string): boolean {
    this.cleanup();
    if (!this.pending.has(id)) return false;
    const buffer = this.buffers.get(id);
    if (!buffer) return false;
    const bytes = this.encoder.encode(data).byteLength;
    if (bytes > this.maxBytes) return true;
    while (buffer.chunks.length && (buffer.chunks.length >= this.maxChunks || buffer.bytes + bytes > this.maxBytes)) {
      const removed = buffer.chunks.shift()!;
      buffer.bytes -= this.encoder.encode(removed).byteLength;
    }
    if (buffer.chunks.length < this.maxChunks && buffer.bytes + bytes <= this.maxBytes) {
      buffer.chunks.push(data);
      buffer.bytes += bytes;
    }
    return true;
  }

  register(id: string): string[] {
    this.cleanup();
    this.pending.delete(id);
    const chunks = this.buffers.get(id)?.chunks ?? [];
    this.buffers.delete(id);
    return chunks;
  }

  remove(id: string): void {
    this.pending.delete(id);
    this.buffers.delete(id);
  }

  clear(): void {
    this.pending.clear();
    this.buffers.clear();
  }

  private cleanup(): void {
    const now = this.now();
    for (const [id, buffer] of this.buffers) {
      if (buffer.expiresAt > now) continue;
      this.buffers.delete(id);
      this.pending.delete(id);
    }
  }
}
