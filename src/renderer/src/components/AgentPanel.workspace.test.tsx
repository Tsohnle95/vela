import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, TranscriptItem } from "@shared/types";

let currentSession: SessionInfo;
let panelTranscript: TranscriptItem[];
let panelQueuedMessages: { id: string; content: string; createdAt: number }[];
let resizeCallback: ResizeObserverCallback | undefined;
const reopenSessionMock = vi.fn(async () => {});

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 80;
    rows = 24;
    options = { theme: {}, fontFamily: "" };
    loadAddon() {}
    open() {}
    onData() { return { dispose() {} }; }
    write() {}
    dispose() {}
  }
}));
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { fit() {} } }));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

vi.mock("../store", () => ({
  useStore: () => ({
    session: currentSession,
    approvalMode: "ask",
    toggleApprovalMode: vi.fn(),
    models: [],
    currentModel: null,
    switchModel: vi.fn(),
    agents: [],
    currentAgent: null,
    switchAgent: vi.fn(),
    loadAgents: vi.fn(),
    loadModels: vi.fn(),
    sendPrompt: vi.fn(),
    runCommand: vi.fn(),
    stop: vi.fn(),
    busy: false,
    sessions: currentSession.parentID ? [{ id: currentSession.parentID, title: "Main agent", directory: currentSession.directory, updatedAt: 1 }] : [],
    reopenSession: reopenSessionMock,
    providerUsage: [],
    providerUsageLoading: false,
    refreshProviderUsage: vi.fn()
  }),
  usePanel: () => ({
    session: currentSession,
    busy: currentSession.id === "working",
    transcript: panelTranscript,
    queuedMessages: panelQueuedMessages,
    todos: [],
    sessionUsage: null,
    models: [],
    currentModel: null,
    agents: [],
    currentAgent: null,
    assistantStatus: currentSession.id === "working" ? { isWorking: true, statusText: "running command" } : null
  })
}));

import { AgentPanel } from "./AgentPanel";
import { Composer } from "./Composer";
import { AppearanceSettings } from "./AppearanceSettings";
import { clearAgentScrollMemoryForTests } from "./agent-scroll";
import { ThemeProvider } from "../theme";

function session(id: string, generation: number): SessionInfo {
  return {
    id,
    directory: `/workspace-${generation}`,
    workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation }
  };
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("composer workspace continuations", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    clearAgentScrollMemoryForTests();
    currentSession = session("one", 1);
    panelTranscript = [];
    panelQueuedMessages = [];
    resizeCallback = undefined;
    vi.stubGlobal("ResizeObserver", class {
      constructor(callback: ResizeObserverCallback) {
        resizeCallback = callback;
      }
      observe() {}
      disconnect() {}
    });
    reopenSessionMock.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("switches the composer control from stop to send when the panel becomes idle", async () => {
    currentSession = session("working", 1);
    await act(async () => root.render(<Composer session={currentSession} />));
    const running = container.querySelector<HTMLButtonElement>(".composer-send");
    expect(running?.classList.contains("stop")).toBe(true);
    expect(running?.title).toBe("running command");

    currentSession = session("idle", 1);
    await act(async () => root.render(<Composer session={currentSession} />));
    const idle = container.querySelector<HTMLButtonElement>(".composer-send");
    expect(idle?.classList.contains("stop")).toBe(false);
    expect(idle?.title).toBe("Type a prompt first");
  });

  it("draws the empty-state mark and prompt only while the panel has no conversation", async () => {
    panelTranscript = [];
    await act(async () => root.render(<AgentPanel />));

    const mark = container.querySelector<HTMLImageElement>(".agent-empty-art");
    expect(mark).not.toBeNull();
    expect(mark!.getAttribute("src")).toContain("agent-station");
    expect(container.querySelector(".agent-empty p")?.textContent).toBe("Tell the agent what to work on.");

    panelTranscript = [{ kind: "user", id: "user-1", text: "Inspect" }];
    await act(async () => root.render(<AgentPanel />));
    expect(container.querySelector(".agent-empty-art")).toBeNull();
    expect(container.textContent).not.toContain("Tell the agent what to work on.");
  });

  it("removes the empty-panel art and centers its prompt when experimental art is disabled", async () => {
    panelTranscript = [];
    await act(async () => root.render(<ThemeProvider><><AppearanceSettings /><AgentPanel /></></ThemeProvider>));

    const toggle = container.querySelector<HTMLButtonElement>('[role="switch"][aria-label="Experimental art"]')!;
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    expect(container.querySelector(".agent-empty-art")).not.toBeNull();

    await act(async () => toggle.click());

    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(window.localStorage.getItem("vela.experimentalArt")).toBe("false");
    expect(document.documentElement.dataset.experimentalArt).toBe("false");
    expect(container.querySelector(".agent-empty-art")).toBeNull();
    expect(container.querySelector(".agent-empty p")?.textContent).toBe("Tell the agent what to work on.");
  });

  it("closes the panel from its options menu", async () => {
    const close = vi.fn();
    await act(async () => root.render(<AgentPanel onClose={close} />));

    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    const item = [...container.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')]
      .find((entry) => entry.textContent?.includes("Close panel"));
    expect(item).not.toBeUndefined();
    await act(async () => item!.click());
    expect(close).toHaveBeenCalledOnce();
  });

  it("switches the agent panel into the runtime TUI from its mode menu", async () => {
    const agentTuiStart = vi.fn(async () => {});
    window.vela = {
      onMessage: vi.fn(() => () => {}),
      agentTuiStart,
      agentTuiInput: vi.fn(async () => {}),
      agentTuiResize: vi.fn(async () => {}),
      agentTuiStop: vi.fn(async () => {})
    } as unknown as Window["vela"];
    await act(async () => root.render(<AgentPanel />));

    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    const tuiItem = [...container.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]')]
      .find((item) => item.textContent?.startsWith("TUI"));
    expect(tuiItem).not.toBeUndefined();
    await act(async () => tuiItem!.click());

    expect(container.querySelector(".agent-tui")).not.toBeNull();
    expect(agentTuiStart).toHaveBeenCalledWith(currentSession.workspace, expect.stringMatching(/^term-/));

    await act(async () => root.unmount());
    root = createRoot(container);
    await act(async () => root.render(<AgentPanel />));
    expect(container.querySelector(".agent-tui")).not.toBeNull();
  });

  it("selects the normal-mode panel from the options menu", async () => {
    const other = session("two", 2);
    const onSessionSelect = vi.fn();
    await act(async () => root.render(
      <AgentPanel
        session={currentSession}
        sessionChoices={[currentSession, other]}
        visibleSessionIDs={new Set([currentSession.id])}
        onSessionSelect={onSessionSelect}
      />
    ));

    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    const item = [...container.querySelectorAll<HTMLButtonElement>('[role="menuitemcheckbox"]')]
      .find((candidate) => candidate.textContent?.includes("workspace-2"));
    expect(item).not.toBeUndefined();
    expect(item?.getAttribute("aria-checked")).toBe("false");
    await act(async () => item!.click());

    expect(onSessionSelect).toHaveBeenCalledWith(other.id);
    expect(container.querySelector('[role="menu"]')).toBeNull();
  });

  it("follows resized stream content only until the reader scrolls away", async () => {
    await act(async () => root.render(<AgentPanel />));
    const scroll = container.querySelector<HTMLDivElement>(".agent-scroll")!;
    Object.defineProperties(scroll, {
      clientHeight: { configurable: true, value: 100 },
      scrollHeight: { configurable: true, value: 500, writable: true },
      scrollTop: { configurable: true, value: 0, writable: true }
    });

    panelTranscript = [{ kind: "user", id: "user-1", text: "Inspect" }];
    await act(async () => root.render(<AgentPanel />));
    expect(scroll.scrollTop).toBe(500);

    Object.defineProperty(scroll, "scrollHeight", { configurable: true, value: 700, writable: true });
    await act(async () => resizeCallback?.([], {} as ResizeObserver));
    expect(scroll.scrollTop).toBe(700);

    scroll.scrollTop = 300;
    await act(async () => scroll.dispatchEvent(new Event("scroll")));
    Object.defineProperty(scroll, "scrollHeight", { configurable: true, value: 900, writable: true });
    await act(async () => resizeCallback?.([], {} as ResizeObserver));
    expect(scroll.scrollTop).toBe(300);
  });

  it("keeps the transcript pinned to the bottom when a queued message grows the prompt dock", async () => {
    panelTranscript = [{ kind: "user", id: "user-1", text: "Inspect" }];
    await act(async () => root.render(<AgentPanel />));
    const scroll = container.querySelector<HTMLDivElement>(".agent-scroll")!;
    Object.defineProperties(scroll, {
      clientHeight: { configurable: true, value: 100, writable: true },
      scrollHeight: { configurable: true, value: 500, writable: true },
      scrollTop: { configurable: true, value: 500, writable: true }
    });

    scroll.scrollTop = 350;
    panelQueuedMessages = [{ id: "queued-1", content: "Continue with the audit", createdAt: Date.now() }];
    await act(async () => root.render(<AgentPanel />));

    expect(scroll.scrollTop).toBe(500);
  });

  it("restores a scrolled-up position across a remount instead of jumping to the bottom", async () => {
    currentSession = session("scroll", 9);
    panelTranscript = [{ kind: "user", id: "user-1", text: "Inspect" }];
    await act(async () => root.render(<AgentPanel />));
    const scroll = container.querySelector<HTMLDivElement>(".agent-scroll")!;
    Object.defineProperties(scroll, {
      clientHeight: { configurable: true, value: 100 },
      scrollHeight: { configurable: true, value: 500, writable: true },
      scrollTop: { configurable: true, value: 500, writable: true }
    });

    scroll.scrollTop = 200;
    await act(async () => scroll.dispatchEvent(new Event("scroll")));

    // Settings open/close unmounts the workspace area; remount the panel.
    await act(async () => root.unmount());
    root = createRoot(container);
    await act(async () => root.render(<AgentPanel />));
    const restored = container.querySelector<HTMLDivElement>(".agent-scroll")!;
    Object.defineProperties(restored, {
      clientHeight: { configurable: true, value: 100, writable: true },
      scrollHeight: { configurable: true, value: 500, writable: true },
      scrollTop: { configurable: true, value: 200, writable: true }
    });
    restored.scrollTop = 200;
    await act(async () => restored.dispatchEvent(new Event("scroll")));

    // New content arrives while the reader sits mid-conversation: the panel
    // must not yank them to the live edge.
    panelTranscript = [
      { kind: "user", id: "user-1", text: "Inspect" },
      { kind: "user", id: "user-2", text: "Continue" }
    ];
    await act(async () => root.render(<AgentPanel />));
    expect(restored.scrollTop).toBe(200);
  });

  it("returns from a child agent session to its parent", async () => {
    currentSession = { ...session("child", 1), id: "child", parentID: "parent", title: "Review changes", agent: "review" };
    await act(async () => root.render(<AgentPanel />));

    await act(async () => container.querySelector<HTMLButtonElement>(".agent-session-back")!.click());
    expect(reopenSessionMock).toHaveBeenCalledWith("parent");
  });

  it("discards slash completions returned after a workspace switch", async () => {
    const commands = deferred<{ name: string; description?: string }[]>();
    window.vela = {
      commands: vi.fn(() => commands.promise)
    } as unknown as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector("textarea")!;
    await act(async () => {
      input.value = "/";
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });

    currentSession = session("two", 2);
    await act(async () => root.render(<Composer />));
    await act(async () => commands.resolve([{ name: "old-command" }]));

    expect(container.querySelector(".composer-completions")).toBeNull();
  });

  it("attaches dropped images and shows a thumbnail preview", async () => {
    window.vela = {
      getPathForFile: vi.fn((file: File) => (file as File & { path?: string }).path ?? ""),
      readImagePreview: vi.fn(async () => "data:image/png;base64,AAA")
    } as unknown as Window["vela"];
    await act(async () => root.render(<Composer />));
    const composer = container.querySelector(".composer")!;
    const drop = new Event("drop", { bubbles: true }) as DragEvent;
    Object.defineProperty(drop, "dataTransfer", {
      value: { types: ["Files"], items: [], files: [{ path: "/tmp/pic.png" }] }
    });
    await act(async () => composer.dispatchEvent(drop));

    expect(container.querySelector(".composer-attachment")).not.toBeNull();
    const thumb = container.querySelector<HTMLImageElement>(".composer-attachment-thumb");
    expect(thumb).not.toBeNull();
    expect(thumb!.src).toBe("data:image/png;base64,AAA");
  });
});
