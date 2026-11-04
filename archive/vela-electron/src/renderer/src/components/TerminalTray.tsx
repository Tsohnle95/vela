import { useCallback, useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { useStore } from "../store";
import { useSassWatch } from "../use-sass-watch";
import { useOptionalTheme } from "../theme";
import { terminalThemeForAppearance } from "../appearances";
import { IconAdd, IconChevronDown, IconChevronUp, IconGlobe } from "./icons";
import type { ViteServerInfo, WorkspaceIdentity } from "@shared/types";
import { PendingTerminalOutput, removeTerminal, terminalDirectoryCommand, terminalSizeForViewport, type TerminalSize, type TerminalTabs, type TerminalView } from "../terminal-state";
import { requestReveal } from "../reveal";
import type { ValidationReport } from "../validation-report";

const TERMINAL_FONT_FAMILY = "ui-monospace, 'SFMono-Regular', 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace";
const TERMINAL_FONT_SIZE = 12;
const TERMINAL_LINE_HEIGHT = 1.25;

function initialTerminalSize(tray: HTMLElement | null): TerminalSize {
  const body = tray?.querySelector<HTMLElement>(".terminal-body");
  if (!tray || !body) return { cols: 100, rows: 24 };

  const width = body.clientWidth || Math.max(0, tray.clientWidth - 2);
  const header = tray.querySelector<HTMLElement>(".terminal-header");
  const height = body.clientHeight || Math.max(0, tray.clientHeight - (header?.offsetHeight ?? 0) - 2);
  if (width <= 0) return { cols: 100, rows: 24 };

  const probe = document.createElement("span");
  probe.textContent = "W";
  probe.style.cssText = `position:fixed;visibility:hidden;white-space:pre;font:${TERMINAL_FONT_SIZE}px/${TERMINAL_FONT_SIZE * TERMINAL_LINE_HEIGHT}px ${TERMINAL_FONT_FAMILY}`;
  document.body.append(probe);
  const measuredCellWidth = probe.getBoundingClientRect().width;
  probe.remove();

  return terminalSizeForViewport(
    width,
    height > 0 ? height : 24 * TERMINAL_FONT_SIZE * TERMINAL_LINE_HEIGHT + 12,
    measuredCellWidth > 0 ? measuredCellWidth : TERMINAL_FONT_SIZE * 0.6,
    TERMINAL_FONT_SIZE * TERMINAL_LINE_HEIGHT
  );
}

function relativeServerDirectory(root: string, directory: string): string {
  const base = root.replace(/[\\/]+$/, "");
  if (directory === base) return "";
  if (directory.startsWith(`${base}/`) || directory.startsWith(`${base}\\`)) {
    return directory.slice(base.length + 1).split(/[\\/]/).join("/");
  }
  return directory;
}

function viteServerLabel(server: ViteServerInfo, root: string): string {
  const relative = relativeServerDirectory(root, server.directory);
  const entry = server.entry || "index.html";
  return relative ? `${relative}/${entry}` : entry;
}

interface TermInstanceProps {
  id: string;
  active: boolean;
  height: number;
  focusRevision: number;
  workspace: WorkspaceIdentity;
  onRegister: (id: string, writer: (data: string) => void) => void;
  onUnregister: (id: string) => void;
}

function TermInstance({ id, active, height, focusRevision, workspace, onRegister, onUnregister }: TermInstanceProps): ReactNode {
  const theme = useOptionalTheme()?.theme ?? "prism";
  const hostRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const term = new Terminal({
      fontFamily: TERMINAL_FONT_FAMILY,
      fontSize: TERMINAL_FONT_SIZE,
      lineHeight: TERMINAL_LINE_HEIGHT,
      cursorBlink: true,
      scrollback: 5000,
      allowTransparency: true,
      theme: terminalThemeForAppearance(theme)
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    termRef.current = term;
    fitRef.current = fit;
    term.open(host);
    try {
      fit.fit();
    } catch {
      /* hidden */
    }

    term.onData((data) => {
      void window.vela.terminalInput(workspace, id, data);
    });

    onRegister(id, (data) => term.write(data));

    const ro = new ResizeObserver(() => {
      try {
        fit.fit();
      } catch {
        /* hidden */
      }
      void window.vela.terminalResize(workspace, id, term.cols, term.rows).catch(() => {});
    });
    ro.observe(host);

    void window.vela.terminalResize(workspace, id, term.cols, term.rows).catch(() => {});

    return () => {
      onUnregister(id);
      ro.disconnect();
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
      void window.vela.terminalStop(workspace, id).catch(() => {});
    };
  }, [id, workspace, onRegister, onUnregister]);

  useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    term.options.theme = terminalThemeForAppearance(theme);
  }, [theme]);

  useEffect(() => {
    if (!active) return;
    const term = termRef.current;
    const fit = fitRef.current;
    if (!term || !fit) return;
    const frame = requestAnimationFrame(() => {
      try {
        fit.fit();
      } catch {
        /* hidden */
      }
      void window.vela.terminalResize(workspace, id, term.cols, term.rows).catch(() => {});
      term.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [active, height, focusRevision, id, workspace]);

  return (
    <div className={`terminal-host ${active ? "" : "hidden"}`} ref={hostRef} />
  );
}

export function TerminalTray({
  height,
  snapped,
  request,
  validationReport = null,
  activePanel = "terminal",
  onPanelChange = () => {},
  onClose,
  onExpand
}: {
  height: number;
  snapped: boolean;
  request?: { id: number; directory: string } | null;
  validationReport?: ValidationReport | null;
  activePanel?: "terminal" | "problems" | "sass";
  onPanelChange?: (panel: "terminal" | "problems" | "sass") => void;
  onClose: () => void;
  onExpand: () => void;
}): ReactNode {
  const { session, activePath, openFile } = useStore();
  const sass = useSassWatch(session);
  const workspace = session!.workspace;
  const sessionDirectory = session!.directory;
  const [{ terms, activeId }, setTabs] = useState<TerminalTabs>({ terms: [], activeId: null });
  const [focusRevision, setFocusRevision] = useState(0);
  const [notice, setNotice] = useState("");
  const [viteStarting, setViteStarting] = useState(false);
  const [viteServers, setViteServers] = useState<ViteServerInfo[]>([]);
  const [viteMenu, setViteMenu] = useState<{ x: number; y: number } | null>(null);
  const bootTokenRef = useRef(0);
  const handledRequestRef = useRef<number | null>(null);
  const trayRef = useRef<HTMLDivElement>(null);
  const writersRef = useRef<Map<string, (data: string) => void>>(new Map());
  const pendingOutputRef = useRef(new PendingTerminalOutput());

  const onRegister = useCallback((id: string, writer: (data: string) => void): void => {
    writersRef.current.set(id, writer);
    for (const data of pendingOutputRef.current.register(id)) writer(data);
  }, []);

  const onUnregister = useCallback((id: string): void => {
    writersRef.current.delete(id);
    pendingOutputRef.current.remove(id);
  }, []);

  useEffect(() => {
    const off = window.vela.onMessage((msg) => {
      if (msg.kind === "terminal-data") {
        const terminal = msg.terminal;
        const writer = writersRef.current.get(terminal.id);
        if (writer) writer(terminal.data);
        else pendingOutputRef.current.write(terminal.id, terminal.data);
      } else if (msg.kind === "terminal-exit") {
        const id = msg.terminal.id;
        writersRef.current.delete(id);
        pendingOutputRef.current.remove(id);
        setTabs((current) => removeTerminal(current, id));
      }
    });
    return off;
  }, []);

  useEffect(() => {
    setViteStarting(false);
    setViteServers([]);
    setViteMenu(null);
  }, [workspace.id]);

  const refreshServers = useCallback(async (): Promise<void> => {
    try {
      const running = await window.vela.viteServers();
      setViteServers(running.filter((server) => server.workspaceId === workspace.id));
    } catch {
      setViteServers([]);
    }
  }, [workspace.id]);

  useEffect(() => {
    void refreshServers();
  }, [refreshServers]);

  const createTerminal = useCallback(async (directory = ""): Promise<void> => {
    onPanelChange("terminal");
    setNotice("");
    const id = `term-${crypto.randomUUID()}`;
    pendingOutputRef.current.awaitRegistration(id);
    const name = directory.split("/").filter(Boolean).pop() ?? "Terminal";
    setTabs((current) => ({ terms: [...current.terms, { id, name, defaultName: !directory }], activeId: id }));
    try {
      const size = initialTerminalSize(trayRef.current);
      const startedDirectory = await window.vela.terminalStart(workspace, id, directory, size.cols, size.rows);
      if (directory && !startedDirectory) {
        const command = terminalDirectoryCommand(window.vela.platform, sessionDirectory, directory);
        if (command) await window.vela.terminalInput(workspace, id, command);
      }
    } catch (err) {
      pendingOutputRef.current.remove(id);
      setTabs((current) => removeTerminal(current, id));
      setNotice(err instanceof Error ? err.message : "Could not start a terminal");
    }
  }, [onPanelChange, sessionDirectory, workspace]);

  useEffect(() => {
    const token = ++bootTokenRef.current;
    setTabs({ terms: [], activeId: null });
    writersRef.current.clear();
    pendingOutputRef.current.clear();
    void (async () => {
      const id = `term-${crypto.randomUUID()}`;
      pendingOutputRef.current.awaitRegistration(id);
      setTabs({ terms: [{ id, name: "Terminal", defaultName: true }], activeId: id });
      try {
        const size = initialTerminalSize(trayRef.current);
        await window.vela.terminalStart(workspace, id, "", size.cols, size.rows);
        if (token !== bootTokenRef.current) {
          void window.vela.terminalStop(workspace, id).catch(() => {});
          return;
        }
      } catch (err) {
        if (token === bootTokenRef.current) {
          pendingOutputRef.current.remove(id);
          setTabs((current) => removeTerminal(current, id));
          setNotice(err instanceof Error ? err.message : "Could not start a terminal");
        }
      }
    })();
    return () => {
      bootTokenRef.current++;
    };
  }, [workspace]);

  useEffect(() => {
    if (!request || handledRequestRef.current === request.id) return;
    handledRequestRef.current = request.id;
    void createTerminal(request.directory);
  }, [createTerminal, request]);

  const openVite = (): void => {
    if (viteStarting) return;
    setNotice("");
    setViteStarting(true);
    const entryPath = activePath && !activePath.startsWith("/") && /\.html?$/i.test(activePath)
      ? activePath
      : undefined;
    const toggle = entryPath
      ? window.vela.viteToggle(workspace, entryPath)
      : window.vela.viteToggle(workspace);
    void toggle
      .then(async (result) => {
        if (!result.running) setNotice("Vite server stopped");
        await refreshServers();
      })
      .catch((error) => {
        setNotice(error instanceof Error ? error.message : "Could not start the Vite server");
      })
      .finally(() => setViteStarting(false));
  };

  const stopVite = (serverID: string): void => {
    void window.vela.viteStop(serverID)
      .then(() => refreshServers())
      .catch(() => {});
  };

  const stopAllVite = (): void => {
    void window.vela.viteStopAll()
      .then(() => refreshServers())
      .catch(() => {});
  };

  const closeTerminal = (id: string): void => {
    void window.vela.terminalStop(workspace, id).catch(() => {});
    const next = removeTerminal({ terms, activeId }, id);
    writersRef.current.delete(id);
    pendingOutputRef.current.remove(id);
    setTabs(next);
    if (next.terms.length === 0) onClose();
  };

  const terminalTab = (term: TerminalView): ReactNode => {
    const name = term.defaultName ? `Terminal ${terms.indexOf(term) + 1}` : term.name;
    return (
      <span
        key={term.id}
        className={`terminal-tab ${activePanel === "terminal" && term.id === activeId ? "active" : ""}`}
        role="tab"
        tabIndex={0}
        aria-selected={activePanel === "terminal" && term.id === activeId}
        onClick={() => {
          onPanelChange("terminal");
          setTabs((current) => ({ ...current, activeId: term.id }));
          setFocusRevision((current) => current + 1);
        }}
        onKeyDown={(event) => {
          if (event.target !== event.currentTarget || (event.key !== "Enter" && event.key !== " ")) return;
          event.preventDefault();
          event.currentTarget.click();
        }}
      >
        {name}
        <button
          className="terminal-tab-close"
          title={`Close ${name}`}
          onClick={(event) => {
            event.stopPropagation();
            closeTerminal(term.id);
          }}
        >
          ×
        </button>
      </span>
    );
  };

  return (
    <div ref={trayRef} className="terminal-tray" style={{ "--terminal-height": `${height}px` } as CSSProperties}>
      <div
        className={`terminal-header ${snapped ? "snapped" : ""}`}
        title={snapped ? "Click to expand terminal" : undefined}
        onClick={snapped ? onExpand : undefined}
      >
        {snapped && (
          <span className="terminal-snap-hint" title="Drag up or click to expand">
            <IconChevronUp />
          </span>
        )}
        {terms[0] ? terminalTab(terms[0]) : (
          <button
            type="button"
            className={`terminal-tab ${activePanel === "terminal" ? "active" : ""}`}
            aria-pressed={activePanel === "terminal"}
            onClick={() => onPanelChange("terminal")}
          >
            Terminal
          </button>
        )}
        <button
          type="button"
          className={`terminal-tab ${activePanel === "problems" ? "active" : ""}`}
          aria-pressed={activePanel === "problems"}
          data-testid="problems-tab"
          onClick={() => onPanelChange("problems")}
        >
          Problems{validationReport?.diagnostics.length ? ` ${validationReport.diagnostics.length}` : ""}
        </button>
        {terms.slice(1).map(terminalTab)}
        <button className={`terminal-tab ${activePanel === "sass" ? "active" : ""}`} aria-pressed={activePanel === "sass"} onClick={() => onPanelChange("sass")}>Sass Output</button>
        <button className="terminal-add" title="New terminal" onClick={() => void createTerminal()}>
          <IconAdd />
        </button>
        <button
          className={`terminal-add${viteServers.length > 0 ? " running" : ""}`}
          title={viteStarting
            ? "Starting the Vite server…"
            : viteServers.length === 1
              ? `${viteServers[0].url} — click to stop, right-click to manage`
              : viteServers.length > 1
                ? `${viteServers.length} Vite servers running — click to toggle this page, right-click to manage`
                : "Serve this workspace with Vite and open it in a browser"}
          disabled={viteStarting}
          data-testid="vite-btn"
          onClick={() => openVite()}
          onContextMenu={(e) => {
            if (viteServers.length === 0) return;
            e.preventDefault();
            setViteMenu({ x: e.clientX, y: e.clientY });
          }}
        >
          <IconGlobe />
        </button>
        {notice && <span className="terminal-notice" title={notice}>{notice}</span>}
        <button className="terminal-close" title="Close the terminal panel (⌥O)" onClick={onClose}>
          <IconChevronDown />
        </button>
      </div>
      <div className={`terminal-body ${snapped || activePanel !== "terminal" ? "hidden" : ""}`}>
        {terms.map((term) => (
          <TermInstance
            key={term.id}
            id={term.id}
            active={activePanel === "terminal" && term.id === activeId}
            height={height}
            focusRevision={focusRevision}
            workspace={workspace}
            onRegister={onRegister}
            onUnregister={onUnregister}
          />
        ))}
        {terms.length === 0 && <div className="terminal-empty">No terminal open. Press + to start one.</div>}
      </div>
      {activePanel === "problems" && (
        <ValidationProblems
          report={validationReport}
          hidden={snapped}
          onOpen={(path, line, column) => {
            if (typeof openFile !== "function") return;
            void openFile(path).then(() => {
              requestAnimationFrame(() => requestReveal(path, line, column));
            });
          }}
        />
      )}
      {activePanel === "sass" && !snapped && <div className="sass-output" role="log" aria-label={`Sass output for ${sessionDirectory}`}>
        <div className="sass-output-root">{sass.state?.directory ?? sessionDirectory}</div>
        {sass.state?.error && !sass.state.logs.some((log) => log.level === "error") && <pre className="error">{sass.state.error}</pre>}
        {!sass.state?.logs.length && !sass.state?.error && <p>Enable Watch Sass in the footer to compile saved Sass files to CSS.</p>}
        {sass.state?.logs.map((log, index) => <pre key={`${log.time}:${index}`} className={log.level}>{new Date(log.time).toLocaleTimeString()} · {log.message}</pre>)}
      </div>}
      {viteMenu && (
        <>
          <div
            className="vite-menu-layer"
            onClick={() => setViteMenu(null)}
            onContextMenu={(e) => {
              e.preventDefault();
              setViteMenu(null);
            }}
          />
          <div className="ctx-menu above" style={{ left: viteMenu.x, top: viteMenu.y }} data-testid="vite-menu">
            {viteServers.map((server) => (
              <button
                key={server.id}
                className="ctx-item"
                onClick={() => {
                  setViteMenu(null);
                  stopVite(server.id);
                }}
              >
                Stop {viteServerLabel(server, sessionDirectory)}
              </button>
            ))}
            {viteServers.length > 1 && (
              <button
                className="ctx-item"
                onClick={() => {
                  setViteMenu(null);
                  stopAllVite();
                }}
              >
                Stop all servers
              </button>
            )}
          </div>
        </>
      )}
    </div>
  );
}

function ValidationProblems({
  report,
  hidden,
  onOpen
}: {
  report: ValidationReport | null;
  hidden: boolean;
  onOpen: (path: string, line: number, column: number) => void;
}): ReactNode {
  const hiddenClass = hidden ? " hidden" : "";
  if (!report) {
    return <div className={`terminal-body problems-empty${hiddenClass}`}>Run HTML/CSS validation to list problems here.</div>;
  }
  if (report.error) {
    return (
      <div className={`terminal-body problems-empty problems-failed${hiddenClass}`} data-testid="validation-error-detail">
        <strong>Validation could not finish</strong>
        <span>{report.error}</span>
        <code>{report.path}</code>
      </div>
    );
  }
  if (report.diagnostics.length === 0) {
    return (
      <div className={`terminal-body problems-empty${hiddenClass}`} data-testid="validation-clean-detail">
        <strong>No validation problems found</strong>
        <code>{report.path}</code>
      </div>
    );
  }
  const errors = report.diagnostics.filter((diagnostic) => diagnostic.severity === "error").length;
  const warnings = report.diagnostics.length - errors;
  const summary = [
    errors > 0 ? `${errors} error${errors === 1 ? "" : "s"}` : "",
    warnings > 0 ? `${warnings} warning${warnings === 1 ? "" : "s"}` : ""
  ].filter(Boolean).join(" · ");
  return (
    <div className={`terminal-body problems-list${hiddenClass}`} data-testid="validation-problems-list">
      <div className="problems-summary">
        <div className="problems-summary-file">
          <span className="problems-summary-title">W3C validation</span>
          <code title={report.path}>{report.path}</code>
        </div>
        <span className="problems-summary-count">{summary}</span>
      </div>
      {report.diagnostics.map((diagnostic, index) => (
        <button
          type="button"
          className={`problem-row ${diagnostic.severity === "error" ? "severity-error" : "severity-warning"}`}
          key={`${diagnostic.line}:${diagnostic.column}:${index}`}
          data-line={diagnostic.line}
          data-column={diagnostic.column}
          title={`${report.path}:${diagnostic.line}:${diagnostic.column} — ${diagnostic.message}`}
          onClick={() => onOpen(report.path, diagnostic.line, diagnostic.column)}
        >
          <span className="problem-icon" aria-hidden="true">{diagnostic.severity === "error" ? "×" : "!"}</span>
          <span className="problem-content">
            <span className="problem-message">{diagnostic.message}</span>
            <span className="problem-source">{report.path}</span>
          </span>
          <span className="problem-location">Ln {diagnostic.line}, Col {diagnostic.column}</span>
        </button>
      ))}
    </div>
  );
}
