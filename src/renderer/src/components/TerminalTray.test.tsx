import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, SessionInfo, ViteServerInfo, ViteToggleResult } from "@shared/types";
import { TerminalTray } from "./TerminalTray";

const writes = vi.hoisted(() => vi.fn());
const focusTerminal = vi.hoisted(() => vi.fn());
const mockOpenFile = vi.hoisted(() => vi.fn(async () => {}));
const mockRequestReveal = vi.hoisted(() => vi.fn());
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 80;
    rows = 24;
    options = { theme: {} };
    loadAddon() {}
    open() {}
    onData() {}
    write(data: string) { writes(data); }
    focus() { focusTerminal(this); }
    dispose() {}
  }
}));
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { fit() {} } }));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));
vi.mock("../reveal", () => ({ requestReveal: mockRequestReveal }));

const session: SessionInfo = {
  id: "session",
  directory: "/workspace",
  workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 }
};
let activePath: string | null = null;
vi.mock("../store", () => ({ useStore: () => ({ session, activePath, openFile: mockOpenFile }) }));

function viteServer(entry: string, directory = "/workspace"): ViteServerInfo {
  return {
    id: JSON.stringify([session.workspace.id, directory, entry]),
    workspaceId: session.workspace.id,
    directory,
    entry,
    url: entry ? `http://127.0.0.1:5199/${entry}` : "http://127.0.0.1:5199/",
    port: 5199
  };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

describe("TerminalTray integration", () => {
  let container: HTMLDivElement;
  let root: Root;
  let listener: (message: BackendMessage) => void;
  let terminalId = "";
  const onClose = vi.fn();

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.stubGlobal("ResizeObserver", class { observe() {} disconnect() {} });
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => { callback(0); return 1; });
    let uuid = 0;
    vi.stubGlobal("crypto", { randomUUID: () => `aaaaaaaa-aaaa-4aaa-8aaa-${String(++uuid).padStart(12, "0")}` });
    writes.mockClear();
    focusTerminal.mockClear();
    mockOpenFile.mockClear();
    mockRequestReveal.mockClear();
    activePath = null;
    onClose.mockClear();
    window.vela = {
      onMessage: (callback: (message: BackendMessage) => void) => { listener = callback; return () => {}; },
      terminalStart: vi.fn(async (_workspace, id) => {
        terminalId = id;
        listener({ kind: "terminal-data", terminal: { id, data: "startup" } });
      }),
      terminalStop: vi.fn(async () => {}),
      terminalResize: vi.fn(async () => {}),
      terminalInput: vi.fn(async () => {}),
      viteToggle: vi.fn(async () => ({ running: true, server: viteServer("") })),
      viteServers: vi.fn(async () => [] as ViteServerInfo[]),
      viteStop: vi.fn(async () => {}),
      viteStopAll: vi.fn(async () => {})
    } as unknown as typeof window.vela;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("flushes data emitted during start and wires natural exit", async () => {
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));

    expect(writes).toHaveBeenCalledWith("startup");
    expect(container.textContent).toContain("Terminal 1");
    await act(async () => listener({ kind: "terminal-exit", terminal: { id: terminalId, exitCode: 0 } }));
    expect(container.textContent).toContain("No terminal open");
  });

  it("makes the leftmost tab select and focus the first shell", async () => {
    const onPanelChange = vi.fn();
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onPanelChange={onPanelChange} onClose={onClose} onExpand={() => {}} />));
    const firstShell = focusTerminal.mock.calls.at(-1)![0];
    expect(container.querySelector(".terminal-tab")?.textContent).toContain("Terminal 1");
    expect(container.querySelectorAll(".terminal-tab.active")).toHaveLength(1);

    await act(async () => container.querySelector<HTMLButtonElement>('[title="New terminal"]')!.click());
    const secondShell = focusTerminal.mock.calls.at(-1)![0];
    expect(secondShell).not.toBe(firstShell);
    expect(container.querySelectorAll(".terminal-host")).toHaveLength(2);

    focusTerminal.mockClear();
    await act(async () => container.querySelector<HTMLElement>(".terminal-tab")!.click());
    expect(focusTerminal).toHaveBeenLastCalledWith(firstShell);
    expect(onPanelChange).toHaveBeenLastCalledWith("terminal");
    expect(container.querySelectorAll(".terminal-tab.active")).toHaveLength(1);

    focusTerminal.mockClear();
    await act(async () => container.querySelector<HTMLElement>(".terminal-tab")!.click());
    expect(focusTerminal).toHaveBeenLastCalledWith(firstShell);
    expect(window.vela.terminalStart).toHaveBeenCalledTimes(2);
  });

  it("numbers the remaining shell from one after earlier tabs close", async () => {
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => container.querySelector<HTMLButtonElement>('[title="New terminal"]')!.click());
    await act(async () => container.querySelector<HTMLButtonElement>('[title="New terminal"]')!.click());
    expect([...container.querySelectorAll(".terminal-tab[role=tab]")].map((tab) => tab.textContent?.replace("×", ""))).toEqual([
      "Terminal 1", "Terminal 2", "Terminal 3"
    ]);

    await act(async () => container.querySelector<HTMLButtonElement>('[title="Close Terminal 1"]')!.click());
    await act(async () => container.querySelector<HTMLButtonElement>('[title="Close Terminal 2"]')!.click());
    expect(container.querySelector(".terminal-tab[role=tab]")?.textContent).toBe("Terminal 1×");
    expect(container.querySelector<HTMLButtonElement>('[title="Close Terminal 1"]')).not.toBeNull();
  });

  it("refocuses the existing shell when returning from Problems", async () => {
    const render = (activePanel: "terminal" | "problems") => <TerminalTray height={240} snapped={false} activePanel={activePanel} onClose={onClose} onExpand={() => {}} />;
    await act(async () => root.render(render("terminal")));
    const firstShell = focusTerminal.mock.calls.at(-1)![0];
    await act(async () => root.render(render("problems")));
    focusTerminal.mockClear();
    await act(async () => container.querySelector<HTMLElement>(".terminal-tab")!.click());
    await act(async () => root.render(render("terminal")));
    expect(focusTerminal).toHaveBeenLastCalledWith(firstShell);
    expect(window.vela.terminalStart).toHaveBeenCalledOnce();
  });

  it("returns to the terminal view when adding a shell from Problems", async () => {
    const onPanelChange = vi.fn();
    await act(async () => root.render(<TerminalTray height={240} snapped={false} activePanel="problems" onPanelChange={onPanelChange} onClose={onClose} onExpand={() => {}} />));
    await act(async () => container.querySelector<HTMLButtonElement>('[title="New terminal"]')!.click());
    expect(onPanelChange).toHaveBeenLastCalledWith("terminal");
    expect(window.vela.terminalStart).toHaveBeenCalledTimes(2);
  });

  it("shows detailed W3C diagnostics in the terminal area's Problems panel", async () => {
    const onPanelChange = vi.fn();
    await act(async () => root.render(
      <TerminalTray
        height={240}
        snapped={false}
        validationReport={{
          path: "src/index.html",
          diagnostics: [
            { line: 7, column: 5, endLine: 7, endColumn: 9, message: "Unexpected end tag", severity: "error", source: "w3c-html" },
            { line: 12, column: 1, endLine: 12, endColumn: 1, message: "Consider adding a lang attribute", severity: "warning", source: "w3c-html" }
          ]
        }}
        activePanel="problems"
        onPanelChange={onPanelChange}
        onClose={onClose}
        onExpand={() => {}}
      />
    ));

    expect(container.querySelectorAll(".problem-row")).toHaveLength(2);
    expect(container.querySelector('[data-testid="validation-problems-list"]')?.textContent)
      .toContain("Unexpected end tag");
    expect(container.querySelector('[data-testid="problems-tab"]')?.textContent).toBe("Problems 2");
    expect(container.querySelector(".problem-row")?.getAttribute("data-line")).toBe("7");
    expect(container.querySelector(".problem-row")?.getAttribute("data-column")).toBe("5");

    await act(async () => {
      container.querySelector<HTMLButtonElement>(".problem-row")!.click();
      await Promise.resolve();
    });
    expect(mockOpenFile).toHaveBeenCalledWith("src/index.html");
    expect(mockRequestReveal).toHaveBeenCalledWith("src/index.html", 7, 5);

    await act(async () => container.querySelector<HTMLButtonElement>(".terminal-tab")!.click());
    expect(onPanelChange).toHaveBeenCalledWith("terminal");
  });

  it("commits the empty view and closes the tray after the final close", async () => {
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    const close = container.querySelector<HTMLButtonElement>(".terminal-tab-close")!;
    await act(async () => close.click());

    expect(container.textContent).toContain("No terminal open");
    expect(onClose).toHaveBeenCalledOnce();
    expect(window.vela.terminalStop).toHaveBeenCalledWith(session.workspace, terminalId);
  });

  it("opens a requested terminal in the selected workspace folder", async () => {
    await act(async () => root.render(
      <TerminalTray
        height={240}
        snapped={false}
        request={{ id: 1, directory: "packages/web" }}
        onClose={onClose}
        onExpand={() => {}}
      />
    ));

    expect(window.vela.terminalStart).toHaveBeenCalledWith(
      session.workspace,
      "term-aaaaaaaa-aaaa-4aaa-8aaa-000000000002",
      "packages/web",
      100,
      24
    );
    expect(window.vela.terminalInput).toHaveBeenCalledWith(
      session.workspace,
      "term-aaaaaaaa-aaaa-4aaa-8aaa-000000000002",
      "cd -- '/workspace/packages/web'\r"
    );
    expect(container.textContent).toContain("web");
  });

  it("starts a shell with the measured terminal viewport", async () => {
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(function (this: HTMLElement) {
      if (this.classList.contains("terminal-body")) return 610;
      if (this.classList.contains("terminal-tray")) return 612;
      return 0;
    });
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockImplementation(function (this: HTMLElement) {
      if (this.classList.contains("terminal-body")) return 190;
      if (this.classList.contains("terminal-tray")) return 226;
      return 0;
    });

    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));

    expect(window.vela.terminalStart).toHaveBeenCalledWith(
      session.workspace,
      "term-aaaaaaaa-aaaa-4aaa-8aaa-000000000001",
      "",
      79,
      11
    );
  });

  it("shows a server button in the terminal header", async () => {
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));

    const button = container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!;
    expect(button.closest(".terminal-header")).not.toBeNull();
    expect(button.title).toBe("Serve this workspace with Vite and open it in a browser");
  });

  it("starts a server for the panel workspace when the server button is clicked", async () => {
    window.vela = {
      ...window.vela,
      viteServers: vi.fn(async () => [viteServer("")])
    } as unknown as typeof window.vela;
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!.click();
      await flush();
    });

    expect(window.vela.viteToggle).toHaveBeenCalledWith(session.workspace);
    const button = container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!;
    expect(button.title).toBe("http://127.0.0.1:5199/ — click to stop, right-click to manage");
    expect(button.classList.contains("running")).toBe(true);
  });

  it("opens the active HTML file through the workspace Vite server", async () => {
    activePath = "pages/demo.html";
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!.click();
      await flush();
    });

    expect(window.vela.viteToggle).toHaveBeenCalledWith(session.workspace, "pages/demo.html");
  });

  it("stops the current page server when the running button is clicked again", async () => {
    const running = [viteServer("")];
    window.vela = {
      ...window.vela,
      viteServers: vi.fn(async () => [...running]),
      viteToggle: vi.fn(async () => {
        running.length = 0;
        return { running: false };
      })
    } as unknown as typeof window.vela;

    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => { await flush(); });
    const button = container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!;
    expect(button.classList.contains("running")).toBe(true);

    await act(async () => {
      button.click();
      await flush();
    });

    expect(window.vela.viteToggle).toHaveBeenCalledWith(session.workspace);
    expect(button.classList.contains("running")).toBe(false);
    expect(container.querySelector(".terminal-notice")?.textContent).toBe("Vite server stopped");
  });

  it("shows the actionable Vite startup error", async () => {
    window.vela = {
      ...window.vela,
      viteToggle: vi.fn(async () => {
        throw new Error("Vite is running, but no page was found at /");
      })
    } as unknown as typeof window.vela;
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!.click();
      await flush();
    });

    expect(container.querySelector(".terminal-notice")?.textContent)
      .toBe("Vite is running, but no page was found at /");
  });

  it("disables the server button while the server starts", async () => {
    let resolveToggle!: (result: ViteToggleResult) => void;
    const pending = new Promise<ViteToggleResult>((resolve) => { resolveToggle = resolve; });
    window.vela = {
      ...window.vela,
      viteToggle: vi.fn(() => pending)
    } as unknown as typeof window.vela;

    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    const button = container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!;
    act(() => { button.click(); });
    expect(button.disabled).toBe(true);

    await act(async () => {
      resolveToggle({ running: true, server: viteServer("") });
      await pending;
      await flush();
    });
    expect(container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')?.disabled).toBe(false);
  });

  it("opens a stop menu on right-click without stopping yet", async () => {
    const viteStop = vi.fn(async () => {});
    window.vela = {
      ...window.vela,
      viteServers: vi.fn(async () => [viteServer("", "/workspace/docs")]),
      viteStop
    } as unknown as typeof window.vela;

    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => { await flush(); });
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!
        .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
      await flush();
    });

    expect(viteStop).not.toHaveBeenCalled();
    expect(container.querySelector('[data-testid="vite-menu"]')?.textContent).toBe("Stop docs/index.html");
  });

  it("stops the specific server from the menu item", async () => {
    const server = viteServer("", "/workspace/docs");
    const running = [server];
    const viteStop = vi.fn(async (serverID: string) => {
      running.splice(running.findIndex((item) => item.id === serverID), 1);
    });
    window.vela = {
      ...window.vela,
      viteServers: vi.fn(async () => [...running]),
      viteStop
    } as unknown as typeof window.vela;

    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => { await flush(); });
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!
        .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
      await flush();
    });

    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-menu"] button')!.click();
      await flush();
    });

    expect(viteStop).toHaveBeenCalledWith(server.id);
    expect(container.querySelector('[data-testid="vite-menu"]')).toBeNull();
    const button = container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!;
    expect(button.classList.contains("running")).toBe(false);
    expect(button.title).toBe("Serve this workspace with Vite and open it in a browser");
  });

  it("stops every server from the menu when more than one is running", async () => {
    const servers = [viteServer("a.html"), viteServer("", "/workspace/docs")];
    const viteStopAll = vi.fn(async () => { servers.length = 0; });
    window.vela = {
      ...window.vela,
      viteServers: vi.fn(async () => [...servers]),
      viteStopAll
    } as unknown as typeof window.vela;

    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => { await flush(); });
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!
        .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
      await flush();
    });

    const stopAll = [...container.querySelectorAll<HTMLButtonElement>('[data-testid="vite-menu"] button')]
      .find((button) => button.textContent === "Stop all servers")!;
    await act(async () => {
      stopAll.click();
      await flush();
    });

    expect(viteStopAll).toHaveBeenCalledOnce();
    expect(container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')?.classList.contains("running")).toBe(false);
  });

  it("shows no menu on right-click when no server is running", async () => {
    await act(async () => root.render(<TerminalTray height={240} snapped={false} onClose={onClose} onExpand={() => {}} />));
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="vite-btn"]')!
        .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
      await flush();
    });

    expect(container.querySelector('[data-testid="vite-menu"]')).toBeNull();
  });
});
