import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, WorkspaceIdentity } from "@shared/types";
import { APPEARANCES } from "../appearances";
import { ThemeProvider, useTheme } from "../theme";

const terminalWrites = vi.hoisted(() => vi.fn());
const terminalData = vi.hoisted(() => vi.fn());
const terminalOptions = vi.hoisted(() => vi.fn());

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 80;
    rows = 24;
    options: Record<string, unknown>;
    constructor(options: Record<string, unknown>) {
      this.options = { ...options };
      terminalOptions(this.options);
    }
    loadAddon() {}
    open() {}
    onData(callback: (data: string) => void) {
      terminalData.mockImplementation(callback);
      return { dispose() {} };
    }
    write(data: string) { terminalWrites(data); }
    dispose() {}
  }
}));
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { fit() {} } }));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

const workspace: WorkspaceIdentity = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };

describe("AgentTui", () => {
  let container: HTMLDivElement;
  let root: Root;
  let listener: (message: BackendMessage) => void;
  const onExit = vi.fn();
  const onError = vi.fn();
  const start = vi.fn(async () => {});
  const input = vi.fn(async () => {});
  const resize = vi.fn(async () => {});
  const stop = vi.fn(async () => {});

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    vi.stubGlobal("ResizeObserver", class {
      constructor(private readonly callback: ResizeObserverCallback) {}
      observe() { this.callback([], {} as ResizeObserver); }
      disconnect() {}
    });
    vi.stubGlobal("crypto", { randomUUID: () => "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa" });
    terminalWrites.mockClear();
    terminalData.mockReset();
    terminalOptions.mockClear();
    onExit.mockClear();
    onError.mockClear();
    start.mockClear();
    input.mockClear();
    resize.mockClear();
    stop.mockClear();
    window.vela = {
      onMessage: (callback: (message: BackendMessage) => void) => {
        listener = callback;
        return () => {};
      },
      agentTuiStart: start,
      agentTuiInput: input,
      agentTuiResize: resize,
      agentTuiStop: stop
    } as unknown as typeof window.vela;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("uses a denser terminal scale for narrow panels", async () => {
    const { tuiMetricsForWidth } = await import("./AgentTui");
    expect(tuiMetricsForWidth(426)).toEqual({ fontSize: 10, lineHeight: 1.15 });
    expect(tuiMetricsForWidth(640)).toEqual({ fontSize: 11, lineHeight: 1.2 });
    expect(tuiMetricsForWidth(900)).toEqual({ fontSize: 12, lineHeight: 1.25 });
  });

  it("removes explicit TUI background paints while preserving foreground styles", async () => {
    const { stripKittyTuiBackgrounds } = await import("./AgentTui");
    const state = { pending: "" };
    expect(stripKittyTuiBackgrounds("\u001b[1;38;2;231;231;238;48;2;2;2;4mtext\u001b[49m", state))
      .toBe("\u001b[1;38;2;231;231;238mtext\u001b[49m");
    expect(stripKittyTuiBackgrounds("\u001b[48;5;0mblack", state)).toBe("black");
    expect(stripKittyTuiBackgrounds("\u001b[44;97mclassic", state)).toBe("\u001b[97mclassic");
    expect(stripKittyTuiBackgrounds("\u001b[101mbright", state)).toBe("bright");
  });

  it("handles background sequences split across terminal data events", async () => {
    const { stripKittyTuiBackgrounds } = await import("./AgentTui");
    const state = { pending: "" };
    expect(stripKittyTuiBackgrounds("\u001b[48;2;2;", state)).toBe("");
    expect(stripKittyTuiBackgrounds("2;4mglass", state)).toBe("glass");
  });

  it("starts the runtime TUI, forwards input, and renders output", async () => {
    const { AgentTui } = await import("./AgentTui");
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /></ThemeProvider>));

    expect(start).toHaveBeenCalledWith(workspace, "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");
    expect(resize).toHaveBeenCalledWith(workspace, "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", 80, 24);

    await act(async () => listener({ kind: "terminal-data", terminal: { id: "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", data: "hello" } }));
    expect(terminalWrites).toHaveBeenCalledWith("hello");
    await act(async () => terminalData("\u0003"));
    expect(input).toHaveBeenCalledWith(workspace, "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "\u0003");
  });

  it("keeps Kitty's embedded terminal transparent across appearance changes", async () => {
    window.localStorage.setItem("vela.theme", "kitty");
    const { AgentTui } = await import("./AgentTui");
    function SwitchTheme() {
      const { setTheme } = useTheme();
      return <button onClick={() => setTheme("original")}>Original Dark</button>;
    }
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /><SwitchTheme /></ThemeProvider>));

    const options = terminalOptions.mock.calls[0][0];
    expect(options.allowTransparency).toBe(true);
    expect(options.theme.background).toBe("rgba(2, 2, 4, 0)");
    expect(options.fontFamily).toContain("FiraCode Nerd Font");
    expect(options.fontWeight).toBe(500);
    await act(async () => listener({ kind: "terminal-data", terminal: { id: "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", data: "\u001b[48;5;0mglass" } }));
    expect(terminalWrites).toHaveBeenLastCalledWith("glass");

    await act(async () => container.querySelector("button")!.click());
    const nextOptions = terminalOptions.mock.calls.at(-1)![0];
    expect(nextOptions.theme.background).toBe("rgba(38,34,32,.92)");
    expect(nextOptions.fontWeight).toBe(400);
    expect(stop).toHaveBeenCalledTimes(1);
    expect(start).toHaveBeenCalledTimes(2);
    await act(async () => listener({ kind: "terminal-data", terminal: { id: "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", data: "\u001b[48;5;0mdark" } }));
    expect(terminalWrites).toHaveBeenLastCalledWith("\u001b[48;5;0mdark");
  });

  it.each(APPEARANCES.filter(({ id }) => id !== "kitty"))("uses $name's agent surface and preserves semantic runtime backgrounds", async ({ id, colors }) => {
    window.localStorage.setItem("vela.theme", id);
    const { AgentTui } = await import("./AgentTui");
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /></ThemeProvider>));
    expect(terminalOptions.mock.calls[0][0].theme.background).toBe(colors.agent);
    expect(terminalOptions.mock.calls[0][0].theme.cyan).toBe(colors.accent);
    expect(terminalOptions.mock.calls[0][0].theme.foreground).toBe(colors.ink);
    expect(terminalOptions.mock.calls[0][0].theme.cursor).toBe(colors.accent);
    await act(async () => listener({ kind: "terminal-data", terminal: { id: "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", data: "\u001b[48;2;12;80;35mdiff\u001b[49m" } }));
    expect(terminalWrites).toHaveBeenLastCalledWith("\u001b[48;2;12;80;35mdiff\u001b[49m");
  });

  it("reports natural exit and stops the PTY on unmount", async () => {
    const { AgentTui } = await import("./AgentTui");
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /></ThemeProvider>));

    await act(async () => listener({ kind: "terminal-exit", terminal: { id: "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", exitCode: 0 } }));
    expect(onExit).toHaveBeenCalledWith(0);

    await act(async () => root.unmount());
    expect(stop).toHaveBeenCalledWith(workspace, "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");
  });

  it("re-sends the fitted size once the TUI has started", async () => {
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => { release = resolve; });
    start.mockReturnValueOnce(gate);
    const { AgentTui } = await import("./AgentTui");
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /></ThemeProvider>));

    // The pre-start resize lands on an unregistered terminal in main and is
    // dropped, so the PTY keeps the manager's default size until one follows
    // the spawn.
    const beforeStart = resize.mock.calls.length;
    await act(async () => { release(); await gate; });
    expect(resize.mock.calls.length).toBeGreaterThan(beforeStart);
    expect(resize.mock.calls.at(-1)).toEqual([workspace, "term-aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", 80, 24]);
  });

  it("ignores a retired launch rejection after switching appearances", async () => {
    let rejectLaunch!: (error: Error) => void;
    start.mockReturnValueOnce(new Promise<void>((_resolve, reject) => { rejectLaunch = reject; }));
    const { AgentTui } = await import("./AgentTui");
    function SwitchTheme() {
      const { setTheme } = useTheme();
      return <button onClick={() => setTheme("original")}>Switch</button>;
    }
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /><SwitchTheme /></ThemeProvider>));
    await act(async () => container.querySelector("button")!.click());
    await act(async () => rejectLaunch(new Error("retired CLI launch failed")));
    expect(onError).not.toHaveBeenCalled();
    expect(start).toHaveBeenCalledTimes(2);
  });

  it("finishes retiring the previous PTY before starting its replacement", async () => {
    let finishLaunch!: () => void;
    let finishStop!: () => void;
    start.mockReturnValueOnce(new Promise<void>((resolve) => { finishLaunch = resolve; }));
    stop.mockReturnValueOnce(new Promise<void>((resolve) => { finishStop = resolve; }));
    const { AgentTui } = await import("./AgentTui");
    function SwitchTheme() {
      const { setTheme } = useTheme();
      return <button onClick={() => setTheme("original")}>Switch</button>;
    }
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /><SwitchTheme /></ThemeProvider>));
    await act(async () => container.querySelector("button")!.click());
    expect(start).toHaveBeenCalledTimes(1);
    expect(stop).not.toHaveBeenCalled();
    await act(async () => finishLaunch());
    expect(stop).toHaveBeenCalledTimes(1);
    expect(start).toHaveBeenCalledTimes(1);
    await act(async () => finishStop());
    expect(start).toHaveBeenCalledTimes(2);
    expect(onError).not.toHaveBeenCalled();
  });

  it("returns a failed TUI launch to the GUI with an error", async () => {
    start.mockRejectedValueOnce(new Error("opencode was not found"));
    const { AgentTui } = await import("./AgentTui");
    await act(async () => root.render(<ThemeProvider><AgentTui workspace={workspace} onExit={onExit} onError={onError} /></ThemeProvider>));

    expect(onError).toHaveBeenCalledWith("opencode was not found");
  });
});
