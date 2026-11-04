import { useEffect, useRef, type ReactNode } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import type { WorkspaceIdentity } from "@shared/types";
import { useOptionalTheme } from "../theme";
import { agentTerminalThemeForAppearance } from "../appearances";

type AnsiSanitizerState = { pending: string };

export function tuiMetricsForWidth(width: number): { fontSize: number; lineHeight: number } {
  if (width > 0 && width <= 520) return { fontSize: 10, lineHeight: 1.15 };
  if (width > 0 && width <= 720) return { fontSize: 11, lineHeight: 1.2 };
  return { fontSize: 12, lineHeight: 1.25 };
}

function stripSgrBackgrounds(sequence: string): string {
  const match = /^\u001b\[([0-9:;]*)([ -\/]*?)m$/.exec(sequence);
  if (!match) return sequence;
  const parts = match[1].split(";");
  const kept: string[] = [];
  for (let index = 0; index < parts.length; index += 1) {
    const part = parts[index];
    if (part.includes(":")) {
      const fields = part.split(":");
      if (fields[0] === "48") continue;
      kept.push(part);
      continue;
    }
    if (part === "48") {
      if (parts[index + 1] === "5") index += 2;
      else if (parts[index + 1] === "2") index += 4;
      continue;
    }
    const code = Number(part);
    if ((code >= 40 && code <= 47) || (code >= 100 && code <= 107)) continue;
    kept.push(part);
  }
  return kept.length > 0 ? `\u001b[${kept.join(";")}m` : "";
}

export function stripKittyTuiBackgrounds(data: string, state: AnsiSanitizerState): string {
  const value = state.pending + data;
  state.pending = "";
  let output = "";
  let cursor = 0;
  while (cursor < value.length) {
    const start = value.indexOf("\u001b[", cursor);
    if (start < 0) {
      output += value.slice(cursor);
      break;
    }
    output += value.slice(cursor, start);
    let final = start + 2;
    while (final < value.length && (value.charCodeAt(final) < 0x40 || value.charCodeAt(final) > 0x7e)) final += 1;
    if (final >= value.length) {
      state.pending = value.slice(start);
      break;
    }
    const sequence = value.slice(start, final + 1);
    output += sequence.endsWith("m") ? stripSgrBackgrounds(sequence) : sequence;
    cursor = final + 1;
  }
  return output;
}

export function AgentTui({
  workspace,
  onExit,
  onError
}: {
  workspace: WorkspaceIdentity;
  onExit: (exitCode: number | null) => void;
  onError: (message: string) => void;
}): ReactNode {
  const theme = useOptionalTheme()?.theme ?? "prism";
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const lifecycleRef = useRef<Promise<void>>(Promise.resolve());
  const onExitRef = useRef(onExit);
  const onErrorRef = useRef(onError);
  const themeRef = useRef(theme);
  const ansiStateRef = useRef<AnsiSanitizerState>({ pending: "" });
  onExitRef.current = onExit;
  onErrorRef.current = onError;
  themeRef.current = theme;

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const id = `term-${crypto.randomUUID()}`;
    const metrics = tuiMetricsForWidth(host.clientWidth);
    const terminal = new Terminal({
      fontFamily: theme === "kitty" ? "'FiraCode Nerd Font', ui-monospace, 'SFMono-Regular', 'SF Mono', Menlo, Consolas, monospace" : "ui-monospace, 'SFMono-Regular', 'SF Mono', Menlo, Consolas, monospace",
      fontSize: metrics.fontSize,
      lineHeight: metrics.lineHeight,
      cursorBlink: true,
      scrollback: 5000,
      allowTransparency: true,
      fontWeight: theme === "kitty" ? 500 : 400,
      theme: agentTerminalThemeForAppearance(theme)
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);
    terminalRef.current = terminal;
    ansiStateRef.current.pending = "";

    const resize = (): void => {
      try {
        const nextMetrics = tuiMetricsForWidth(host.clientWidth);
        terminal.options.fontSize = nextMetrics.fontSize;
        terminal.options.lineHeight = nextMetrics.lineHeight;
        fit.fit();
      } catch {
        return;
      }
      void window.vela.agentTuiResize(workspace, id, terminal.cols, terminal.rows).catch(() => {});
    };
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(host);
    terminal.onData((data) => {
      void window.vela.agentTuiInput(workspace, id, data).catch(() => {});
    });
    const off = window.vela.onMessage((message) => {
      if (message.kind === "terminal-data" && message.terminal.id === id) {
        const data = themeRef.current === "kitty"
          ? stripKittyTuiBackgrounds(message.terminal.data, ansiStateRef.current)
          : message.terminal.data;
        terminal.write(data);
      }
      if (message.kind === "terminal-exit" && message.terminal.id === id) onExitRef.current(message.terminal.exitCode);
    });
    resize();
    let disposed = false;
    const start = lifecycleRef.current.then(async () => {
      if (disposed) return false;
      await window.vela.agentTuiStart(workspace, id);
      return true;
    });
    void start
      .then((started) => {
        // The PTY starts at the terminal manager's default size, so the fitted
        // size has to follow the spawn. Resizing before the start lands on an
        // unregistered terminal and is dropped, which left the TUI drawing only
        // the default rows with the rest of the panel blank.
        if (started && terminalRef.current === terminal) resize();
      })
      .catch((error: unknown) => {
        if (terminalRef.current === terminal) onErrorRef.current(error instanceof Error ? error.message : "Could not start the agent TUI");
      });

    return () => {
      disposed = true;
      off();
      resizeObserver.disconnect();
      terminal.dispose();
      terminalRef.current = null;
      // An IPC launch may still be awaiting its runtime command. Retire it
      // after it finishes, then allow the next palette's CLI view to start.
      lifecycleRef.current = start.catch(() => false).then(async (started) => {
        if (started) await window.vela.agentTuiStop(workspace, id).catch(() => {});
      });
    };
  }, [workspace.id, workspace.generation, theme]);

  useEffect(() => {
    const terminal = terminalRef.current;
    if (!terminal) return;
    if (theme !== "kitty") ansiStateRef.current.pending = "";
    terminal.options.theme = agentTerminalThemeForAppearance(theme);
    terminal.options.fontWeight = theme === "kitty" ? 500 : 400;
    terminal.options.fontFamily = theme === "kitty" ? "'FiraCode Nerd Font', ui-monospace, 'SFMono-Regular', 'SF Mono', Menlo, Consolas, monospace" : "ui-monospace, 'SFMono-Regular', 'SF Mono', Menlo, Consolas, monospace";
  }, [theme]);

  return <div className="agent-tui"><div className="agent-tui-host" ref={hostRef} /></div>;
}
