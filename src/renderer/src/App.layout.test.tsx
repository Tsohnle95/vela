import { act, Profiler } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, SessionInfo, TranscriptItem } from "@shared/types";
import App from "./App";

vi.mock("./components/EditorPane", () => ({
  EditorPane: ({ onSelectSurface }: { onSelectSurface?: () => void }) => <button data-testid="editor-tab" onClick={onSelectSurface}>File tab</button>
}));

vi.mock("./components/ChangesView", () => ({
  ChangesView: () => <div data-testid="changes-view" />
}));

vi.mock("./monaco", () => ({
  languageForPath: () => "plaintext",
  monaco: { editor: { setModelMarkers: vi.fn() } }
}));

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    loadAddon() {}
    open() {}
    onData() {}
    write() {}
    focus() {}
    dispose() {}
  }
}));
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { fit() {} } }));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

let messageHandlers: ((message: BackendMessage) => void)[] = [];

function dispatch(message: BackendMessage): void {
  for (const handler of messageHandlers) handler(message);
}

function info(directory: string, generation: number): SessionInfo {
  return {
    id: `session-${generation}`,
    directory,
    workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation }
  };
}

function api(): typeof window.vela {
  let openedGeneration = 3;
  return {
    platform: "darwin",
    onMessage: (handler: (msg: BackendMessage) => void) => {
      messageHandlers.push(handler);
      return () => { messageHandlers = messageHandlers.filter((h) => h !== handler); };
    },
    health: async () => true,
    takePendingPaths: async () => [],
    state: async () => info("/repo", 1),
    activeSessions: async () => [info("/repo", 1)],
    projects: async () => [],
    models: async () => [],
    modelDefault: async () => null,
    sessionSelection: async () => null,
    agents: async () => [],
    openSession: async (directory: string) => info(directory, ++openedGeneration),
    selectFolder: async () => info("/repo", ++openedGeneration),
    sessions: async () => [],
    savedWorkspaces: async () => ({ initialized: true, workspaces: [] }),
    saveSavedWorkspaces: async () => {},
    closeSession: async () => {},
    openSessionById: async () => ({ session: info("/repo", 1), transcript: [], todos: [], usage: null }),
    readFile: async () => "content",
    listDir: async () => [],
    listPermissions: async () => [],
    providerUsage: async () => [],
    recoveryRecords: async () => [],
    windowView: async () => {},
    reportUnsavedEditorFiles: async () => {}
  } as unknown as typeof window.vela;
}

function setWidth(value: number): void {
  Object.defineProperty(window, "innerWidth", { value, configurable: true });
  window.dispatchEvent(new Event("resize"));
}

function sideTab(container: HTMLElement, title: string): HTMLButtonElement {
  return container.querySelector<HTMLButtonElement>(`[data-panel-action="show-${title.toLowerCase()}"]`)!;
}

function toggleSidebar(container: HTMLElement): void {
  // The rail's active view button collapses the sidebar; an inactive one opens it.
  const active = container.querySelector<HTMLButtonElement>('.activity-tool[aria-pressed="true"][data-panel-action^="show-"]');
  (active ?? sideTab(container, "Files")).click();
}

describe("Layout panel sizing", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    messageHandlers = [];
    window.localStorage.clear();
    Object.defineProperty(window, "innerWidth", { value: 1480, configurable: true });
    window.vela = api();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  function agentCols(): HTMLElement[] {
    return [...container.querySelectorAll<HTMLElement>(".agent-col")];
  }

  function agentWidths(): number[] {
    return agentCols().map((col) => Number.parseFloat(col.style.width ?? "0"));
  }

  function agentLefts(): number[] {
    return agentCols().map((col) => Number.parseFloat(col.style.left ?? "0"));
  }

  function gridCols(): string[] {
    const grid = container.querySelector<HTMLElement>(".main-row")!;
    return (grid.style.getPropertyValue("--pane-columns") ?? "").split(" ");
  }

  async function enterAgentMode(...sessionsToAdd: string[]): Promise<void> {
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    for (const sessionName of sessionsToAdd) {
      await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
      const choices = [...container.querySelectorAll<HTMLButtonElement>('[role="menuitemcheckbox"]')]
        .filter((item) => item.getAttribute("aria-checked") !== "true");
      const choice = choices.find((item) => item.textContent?.includes(sessionName)) ?? choices[0];
      expect(choice).not.toBeUndefined();
      await act(async () => {
        choice!.click();
        await new Promise((resolve) => setTimeout(resolve, 20));
      });
    }
  }

  async function closePanelFromOptions(panelIndex = 0): Promise<void> {
    await act(async () => {
      container.querySelectorAll<HTMLButtonElement>('button[aria-label="Agent panel options"]')[panelIndex]!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    const item = [...container.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')]
      .find((entry) => entry.textContent?.includes("Close panel"));
    expect(item).not.toBeUndefined();
    await act(async () => {
      item!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
  }

  // The panel's own session is the checked entry; its state label differs
  // between coding mode ("Current coding workspace") and Agent Mode.
  function currentWorkspaceChoice(): string | undefined {
    return [...container.querySelectorAll<HTMLButtonElement>('[role="menuitemcheckbox"]')]
      .find((item) => /Current coding workspace|In Agent Mode/.test(item.querySelector("small")?.textContent ?? ""))
      ?.textContent;
  }

  it("keeps session and workspace opening actions out of the titlebar", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    expect(container.querySelector('.titlebar [title^="Sessions"]')).toBeNull();
    expect(container.querySelector('[title="Open another workspace"]')).toBeNull();
    expect(container.querySelector('[title="Open a single file"]')).toBeNull();
  });

  it("preserves the selected sidebar when a workspace opens", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 30)));

    expect(sideTab(container, "Files").className).toContain("active");
    expect(sideTab(container, "Sessions").className).not.toContain("active");

    await act(async () => {
      sideTab(container, "Sessions").click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(sideTab(container, "Sessions").className).toContain("active");

    await act(async () => {
      dispatch({ kind: "session", session: info("/fresh", 5) });
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(sideTab(container, "Sessions").className).toContain("active");
    expect(sideTab(container, "Files").className).not.toContain("active");
    await act(async () => sideTab(container, "Files").click());
    await act(async () => dispatch({ kind: "session", session: info("/from-files", 6) }));
    expect(sideTab(container, "Files").className).toContain("active");
  });

  it("toggles the left panel from the activity rail and keeps the editor slot", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const files = sideTab(container, "Files");
    expect(files.getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector(".main-row.sidebar-closed")).toBeNull();
    expect(container.querySelector(".sidebar.collapsed")).toBeNull();
    expect(container.querySelector(".workspace-area")).not.toBeNull();

    await act(async () => {
      files.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(files.getAttribute("aria-pressed")).toBe("false");
    expect(gridCols()).toEqual(["0px", "0px", "minmax(0,1fr)"]);
    expect(container.querySelector(".sidebar-slot.collapsed")).not.toBeNull();
    expect(container.querySelector(".sidebar")).toBeNull();
    expect(container.querySelector(".main-row.sidebar-closed")).not.toBeNull();
    expect(container.querySelector(".workspace-area")).not.toBeNull();

    await act(async () => {
      files.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(files.getAttribute("aria-pressed")).toBe("true");
    expect(gridCols()[0]).toBe("230px");
    expect(container.querySelector(".main-row.sidebar-closed")).toBeNull();
    expect(container.querySelector(".sidebar")).not.toBeNull();
  });

  it("opens and closes the left panel from the titlebar control", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const toggle = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-sidebar"]')!;
    expect(toggle.getAttribute("aria-label")).toBe("Hide left panel");
    expect(container.querySelector(".sidebar")).not.toBeNull();

    await act(async () => toggle.click());
    expect(toggle.getAttribute("aria-label")).toBe("Show left panel");
    expect(container.querySelector(".main-row.sidebar-closed")).not.toBeNull();
    expect(container.querySelector(".sidebar")).toBeNull();

    await act(async () => toggle.click());
    expect(toggle.getAttribute("aria-label")).toBe("Hide left panel");
    expect(container.querySelector(".main-row.sidebar-closed")).toBeNull();
    expect(container.querySelector(".sidebar")).not.toBeNull();
  });

  it("opens the Changes review view from the activity rail", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const button = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-changes"]')!;
    expect(button.getAttribute("aria-pressed")).toBe("false");
    expect(container.querySelector('[data-testid="changes-view"]')).toBeNull();

    await act(async () => {
      button.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector('[data-testid="changes-view"]')).not.toBeNull();
    expect(container.querySelector(".app")?.classList.contains("changes-open")).toBe(true);
    await act(async () => container.querySelector<HTMLButtonElement>('[data-testid="editor-tab"]')!.click());
    expect(container.querySelector('[data-testid="changes-view"]')).toBeNull();
    await act(async () => button.click());

    await act(async () => {
      button.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(button.getAttribute("aria-pressed")).toBe("false");
    expect(container.querySelector(".app")?.classList.contains("changes-open")).toBe(false);
  });

  it("opens Settings from the activity rail and keeps the file and session views", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const panelToggle = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-agent-panel"]')!;
    const settingsButton = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-settings"]')!;
    expect(panelToggle.closest(".titlebar-actions")).not.toBeNull();
    expect(panelToggle.previousElementSibling?.getAttribute("data-panel-action")).toBe("add-model-panel");
    expect(container.querySelector('[data-panel-action="toggle-sidebar"]')?.closest(".titlebar-leading-actions")).not.toBeNull();
    expect(settingsButton.closest(".activity-rail")).not.toBeNull();
    expect(settingsButton.getAttribute("aria-pressed")).toBe("false");

    const rail = container.querySelector<HTMLElement>(".activity-rail")!;
    expect(rail).not.toBeNull();
    expect(rail.querySelector('button[aria-label="Files"]')).not.toBeNull();
    expect(rail.querySelector('button[aria-label="Sessions"]')).not.toBeNull();
    expect(rail.querySelector('button[aria-label="Terminal"]')).not.toBeNull();
    expect(rail.querySelector('button[aria-label="Changes"]')).not.toBeNull();
    expect(rail.querySelector('button[aria-label="Settings"]')).not.toBeNull();
    // Changes sits between Terminal and the browser action.
    expect(rail.querySelector('[data-panel-action="toggle-changes"]')?.previousElementSibling?.getAttribute("aria-label")).toBe("Terminal");
    expect(container.querySelector(".side-tabs")).toBeNull();
    expect(sideTab(container, "Files")).not.toBeNull();
    expect(sideTab(container, "Sessions")).not.toBeNull();

    await act(async () => {
      sideTab(container, "Sessions").click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(sideTab(container, "Sessions").className).toContain("active");
    expect(container.querySelector(".activity-rail")).not.toBeNull();

    await act(async () => {
      settingsButton.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(settingsButton.getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector(".settings-sidebar")).not.toBeNull();
    expect(container.querySelector(".workspace-area.settings-suspended")).not.toBeNull();
    expect(container.querySelector(".workspace-area")?.hasAttribute("inert")).toBe(true);

    await act(async () => {
      settingsButton.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(settingsButton.getAttribute("aria-pressed")).toBe("false");
    expect(container.querySelector(".settings-sidebar")).toBeNull();
    expect(container.querySelector(".workspace-area.settings-suspended")).toBeNull();
    expect(container.querySelector(".workspace-area")?.hasAttribute("inert")).toBe(false);

    await act(async () => {
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(container.querySelector(".activity-rail")).not.toBeNull();
    expect(container.querySelector(".sidebar")).toBeNull();
    expect(sideTab(container, "Sessions").getAttribute("aria-pressed")).toBe("false");

    await act(async () => {
      sideTab(container, "Sessions").click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(container.querySelector(".activity-rail")).not.toBeNull();
    expect(container.querySelector(".sidebar")).not.toBeNull();
  });

  it("hides the sidebar but keeps the activity rail in agent mode", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    expect(container.querySelector(".activity-rail")).not.toBeNull();

    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelector(".app.agent-mode")).not.toBeNull();
    expect(container.querySelector(".activity-rail")).not.toBeNull();
    expect(container.querySelector(".sidebar")).toBeNull();
  });

  it("keeps the sidebar views available in agent mode and exits when used", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(container.querySelector(".app.agent-mode")).not.toBeNull();

    const sessions = sideTab(container, "Sessions");
    expect(sessions).not.toBeNull();
    expect(sessions.getAttribute("aria-pressed")).toBe("false");

    await act(async () => {
      sessions.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelector(".main-row.sidebar-closed")).toBeNull();
    expect(container.querySelector(".activity-rail")).not.toBeNull();
    expect(container.querySelector(".sidebar")).not.toBeNull();
  });

  it("settles with both panels fitting when the window is narrower than their combined width", async () => {
    setWidth(500);
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    expect(container.querySelector(".app")).not.toBeNull();
    const side = Number.parseFloat(gridCols()[0] ?? "0");
    const agent = agentWidths()[0] ?? 0;
    expect(side).toBe(230);
    expect(agent).toBeGreaterThanOrEqual(44);
  });

  it("converges without a crash when the window shrinks below the combined panel width", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    await act(async () => {
      setWidth(480);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelector(".app")).not.toBeNull();
    const side = Number.parseFloat(gridCols()[0] ?? "0");
    expect(side).toBe(230);
    expect(agentWidths()[0]).toBeGreaterThanOrEqual(44);
  });

  it("keeps an anchored agent panel tracking its cap as the window shrinks", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    expect(agentLefts()).toEqual([924]);

    await act(async () => {
      const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 52 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()[0]).toBeCloseTo(1249, 0);
    expect(agentLefts()[0]).toBeCloseTo(0, 0);

    await act(async () => {
      setWidth(900);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(Number.parseFloat(gridCols()[0] ?? "0")).toBeCloseTo(230, 0);
    expect(agentWidths()[0]).toBeCloseTo(669, 0);
  });

  it("previews panel resizing without committing React renders on mousemove", async () => {
    let commits = 0;
    await act(async () => root.render(
      <Profiler id="layout" onRender={() => { commits += 1; }}>
        <App />
      </Profiler>
    ));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
    await act(async () => {
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      await Promise.resolve();
    });
    const baseline = commits;

    await act(async () => {
      for (let clientX = 990; clientX >= 700; clientX -= 10) {
        window.dispatchEvent(new MouseEvent("mousemove", { clientX }));
      }
      await Promise.resolve();
    });

    expect(agentWidths()[0]).toBeCloseTo(625, 0);
    expect(commits).toBe(baseline);

    await act(async () => {
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await Promise.resolve();
    });

    expect(agentWidths()[0]).toBeCloseTo(625, 0);
    expect(commits).toBeGreaterThan(baseline);
  });

  it("keeps a left-expanded agent panel covering the editor when the window widens", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    await act(async () => {
      const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 0 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentLefts()[0]).toBe(0);
    expect(agentWidths()[0]).toBeCloseTo(1249, 0);

    await act(async () => {
      setWidth(1800);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentLefts()[0]).toBe(0);
    expect(agentWidths()[0]).toBeCloseTo(1569, 0);
    expect(container.querySelector<HTMLElement>(".workspace-area")!.style.getPropertyValue("--editor-right")).toBe("1569px");
  });

  it("keeps a left-expanded panel against either explorer tray", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    await act(async () => {
      const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 0 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentLefts()[0]).toBe(0);
    expect(agentWidths()[0]).toBeCloseTo(1480, 0);

    await act(async () => {
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(gridCols()[0]).toBe("230px");
    expect(agentLefts()[0]).toBe(0);
    expect(agentWidths()[0]).toBeCloseTo(1249, 0);
    expect(container.querySelector<HTMLElement>(".workspace-area")!.style.getPropertyValue("--editor-right")).toBe("1249px");
  });

  it("reopens the file pane from the activity rail after closing it", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(gridCols()).toEqual(["0px", "0px", "minmax(0,1fr)"]);
    expect(container.querySelector(".sidebar")).toBeNull();

    await act(async () => {
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(gridCols()[0]).toBe("230px");
    expect(container.querySelector(".sidebar")).not.toBeNull();
  });

  it("reserves editor space with Files closed on a narrow window", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      toggleSidebar(container);
      setWidth(500);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelector(".main-row.sidebar-closed .workspace-area")).not.toBeNull();
    expect(agentWidths()[0]).toBe(310);
    expect(agentLefts()[0]).toBe(190);
  });

  it("keeps the sidebar closed while dragging on the workspace after reopening it", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(gridCols()).toEqual(["0px", "0px", "minmax(0,1fr)"]);

    await act(async () => {
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(gridCols()[0]).toBe("230px");

    const divider = container.querySelector<HTMLElement>(".divider")!;
    await act(async () => {
      divider.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 230 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 300 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(Number.parseFloat(gridCols()[0])).toBeGreaterThanOrEqual(280);
  });

  it("keeps the anchored agent against the compact explorer tray", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
    await act(async () => {
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 52 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()[0]).toBeCloseTo(1249, 0);
    expect(agentLefts()[0]).toBeCloseTo(0, 0);
    expect(container.querySelector(".workspace-area.editor-covered")).not.toBeNull();

    await act(async () => {
      toggleSidebar(container);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()[0]).toBeCloseTo(1480, 0);
    expect(agentLefts()[0]).toBeCloseTo(0, 0);
    expect(container.querySelector(".workspace-area.editor-covered")).not.toBeNull();
  });

  it("insets the editor area to the free space left of the agent panels", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const editorRight = (): number =>
      Number.parseFloat(
        container.querySelector<HTMLElement>(".workspace-area")!.style.getPropertyValue("--editor-right") || "0"
      );

    expect(editorRight()).toBeCloseTo(325, 0);
    expect(container.querySelector(".workspace-area.editor-covered")).toBeNull();

    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(editorRight()).toBeCloseTo(325, 0);
    await enterAgentMode();
    expect(editorRight()).toBeCloseTo(1480, 0);

    const model = container.querySelectorAll<HTMLElement>(".agent-col")[0]!;
    await act(async () => {
      const handle = model.querySelector<HTMLElement>(".panel-resize-left")!;
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 671 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 500 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(editorRight()).toBeCloseTo(325, 0);
    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("lays out multiple session panels stacked against the right side", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);

    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(2);
    const cols = gridCols();
    expect(cols).toEqual(["0px", "0px", "minmax(0,1fr)"]);
    const [first, second] = agentWidths();
    expect(first).toBeGreaterThanOrEqual(44);
    expect(second).toBeGreaterThanOrEqual(44);
    expect(agentLefts()).toEqual([0, 740]);
  });

  it("switches the single coding-mode panel from the options menu", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    expect(currentWorkspaceChoice()).toContain("repo");
    const choice = [...container.querySelectorAll<HTMLButtonElement>('[role="menuitemcheckbox"]')]
      .find((item) => item.textContent?.includes("two"));
    expect(choice).not.toBeUndefined();
    await act(async () => choice!.click());

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    expect(currentWorkspaceChoice()).toContain("two");
  });

  it("adds an active session without disturbing Agent Mode geometry", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode();

    const beforeLefts = agentLefts();
    const beforeWidths = agentWidths();
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    const choice = [...container.querySelectorAll<HTMLButtonElement>('[role="menuitemcheckbox"]')]
      .find((item) => item.textContent?.includes("two"));
    expect(choice).not.toBeUndefined();
    await act(async () => choice!.click());

    expect(beforeLefts).toEqual([0]);
    expect(beforeWidths).toEqual([1480]);
    expect(agentLefts()).toEqual([0, 740]);
    expect(agentWidths()).toEqual([740, 740]);
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(2);

    await enterAgentMode();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    expect(agentWidths()).toEqual([325]);
    expect(agentLefts()).toEqual([924]);
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    expect(currentWorkspaceChoice()).toContain("two");
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());

    await enterAgentMode();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    expect(currentWorkspaceChoice()).toContain("two");
  });

  it("clamps both panels to their minimum width when the window is narrower than their combined width", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");
    await act(async () => {
      setWidth(700);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(2);
    const [first, second] = agentWidths();
    expect(first).toBeGreaterThanOrEqual(280);
    expect(second).toBeGreaterThanOrEqual(280);
  });

  it("adds a third model panel without disturbing the layout", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      dispatch({ kind: "session", session: info("/three", 3) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two", "three");

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(3);
    const cols = gridCols();
    expect(cols).toHaveLength(3);
    expect(agentWidths()).toEqual([1480, 740, 740]);
    expect(agentLefts()).toEqual([0, 0, 740]);
  });

  it("keeps a model panel at the 280px minimum when dragged below it", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    const handles = container.querySelectorAll<HTMLElement>(".agent-col .panel-resize-right");
    expect(handles).toHaveLength(2);
    await act(async () => {
      handles[0].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 950 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()).toEqual([740]);
    expect(agentLefts()).toEqual([740]);
    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("slides a model header away from the right-side stack", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    const headers = container.querySelectorAll<HTMLElement>(".agent-header");
    expect(headers).toHaveLength(2);

    await act(async () => {
      headers[0].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 500 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 470 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 440 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentLefts()).toEqual([740]);
    expect(agentWidths()).toEqual([740]);
    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("extends a model from its left edge without moving its right edge", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    const handles = container.querySelectorAll<HTMLElement>(".agent-col .panel-resize-left");
    expect(handles).toHaveLength(2);
    await act(async () => {
      handles[0].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 400 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 300 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()).toEqual([740]);
    expect(agentLefts()).toEqual([740]);
    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("restores the left border during a resize as soon as a gap opens", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    const add = container.querySelector<HTMLButtonElement>('button[aria-label="Add model panel"]')!;
    await act(async () => {
      add.click();
      await new Promise((resolve) => setTimeout(resolve, 30));
      add.click();
      add.click();
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentCols()).toHaveLength(1);
    expect(agentLefts()).toEqual([924]);
  });

  it("stops a panel at its neighbor's edge without touching it", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    const handles = container.querySelectorAll<HTMLElement>(".agent-col .panel-resize-right");
    await act(async () => {
      handles[0].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 1100 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()).toEqual([740]);
    expect(agentLefts()).toEqual([740]);
    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("stops the anchored panel at its neighbor's edge", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    const handles = container.querySelectorAll<HTMLElement>(".agent-col .panel-resize-left");
    await act(async () => {
      handles[1].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 900 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()).toEqual([740]);
    expect(agentLefts()).toEqual([740]);
    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("keeps the original agent panel right-anchored and resizes from its left edge", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const handles = container.querySelectorAll<HTMLElement>(".agent-col .panel-resize-left");
    await act(async () => {
      handles[0].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 1050 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()[0]).toBeCloseTo(280, 0);
    expect(agentLefts()[0]).toBeCloseTo(969, 0);

    const headers = container.querySelectorAll<HTMLElement>(".agent-header");
    await act(async () => {
      headers[0].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 500 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 560 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()[0]).toBeCloseTo(280, 0);
    expect(agentLefts()[0]).toBeCloseTo(969, 0);
  });

  it("keeps the anchored panel at 280px while the drag continues below the minimum", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    const leftHandles = container.querySelectorAll<HTMLElement>(".agent-col .agent-panel .panel-resize-left");
    await act(async () => {
      leftHandles[1].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 949 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 2000 }));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()).toEqual([740, 740]);
    expect(agentLefts()).toEqual([0, 740]);

    await act(async () => {
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentWidths()).toEqual([740]);
    expect(agentLefts()).toEqual([740]);
    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("reopens a collapsed agent panel from the titlebar toggle", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    const toggle = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-agent-panel"]')!;
    await act(async () => {
      toggle.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(toggle.getAttribute("aria-pressed")).toBe("false");
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(0);
    expect(agentWidths()).toEqual([]);

    await act(async () => {
      toggle.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(toggle.getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    expect(agentWidths()).toEqual([325]);
    expect(agentLefts()).toEqual([924]);
  });

  it("repeatedly toggles the single agent panel closed and open from the titlebar", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    for (let cycle = 0; cycle < 2; cycle += 1) {
      await act(async () => {
        container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-agent-panel"]')!.click();
        await new Promise((resolve) => setTimeout(resolve, 20));
      });
      expect(container.querySelectorAll(".agent-panel")).toHaveLength(0);
      expect(container.querySelectorAll(".agent-tray, .agent-sliver")).toHaveLength(0);

      const toggle = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-agent-panel"]')!;
      expect(toggle.getAttribute("aria-pressed")).toBe("false");

      await act(async () => {
        toggle.click();
        await new Promise((resolve) => setTimeout(resolve, 20));
      });
      expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
      expect(agentWidths()).toEqual([325]);
      expect(agentLefts()).toEqual([924]);
    }
  });

  it("hides the only agent tray from its options menu without closing the session", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    await closePanelFromOptions();

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(0);
    expect(container.querySelector(".workspace-area")).not.toBeNull();
    const toggle = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-agent-panel"]')!;
    expect(toggle.disabled).toBe(false);
    expect(toggle.getAttribute("aria-pressed")).toBe("false");

    await act(async () => {
      toggle.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
  });

  it("leaves agent mode when hiding its only agent tray", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(container.querySelector(".app.agent-mode")).not.toBeNull();

    await closePanelFromOptions();

    expect(container.querySelector(".app.agent-mode")).toBeNull();
    expect(container.querySelector(".sidebar")).not.toBeNull();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(0);
    expect(container.querySelector('[data-panel-action="toggle-agent-panel"]')?.getAttribute("aria-pressed")).toBe("false");
  });

  it("closes a model panel without leaving a collapsed strip behind", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(2);

    const handles = container.querySelectorAll<HTMLElement>(".agent-col .agent-panel .panel-resize-right");
    expect(handles.length).toBeGreaterThan(0);

    await closePanelFromOptions(0);

    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    expect(container.querySelectorAll(".agent-sliver")).toHaveLength(0);
    expect(agentWidths()).toEqual([1480]);
    expect(agentLefts()).toEqual([0]);
  });

  it("keeps the close control on every agent panel", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");

    const cols = agentCols();
    expect(cols.map((col) => Boolean(col.querySelector('button[aria-label="Agent panel options"]')))).toEqual([true, true]);
    expect(cols[0].querySelector(".agent-collapse")).toBeNull();
    expect(cols[1].querySelector(".agent-collapse")).toBeNull();

    // Every panel's options menu offers the close action.
    await act(async () => cols[1].querySelector<HTMLButtonElement>('button[aria-label="Agent panel options"]')!.click());
    expect([...container.querySelectorAll<HTMLElement>('[role="menuitem"]')]
      .some((item) => item.textContent?.includes("Close panel"))).toBe(true);
  });

  it("agent mode collapses the file tray, splits agents evenly, and restores the tray on exit", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    const modeButton = (): HTMLButtonElement =>
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!;

    await enterAgentMode("two");

    expect(agentWidths()).toEqual([740, 740]);
    expect(agentLefts()).toEqual([0, 740]);

    await act(async () => {
      modeButton().click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(gridCols()[0]).toBe("230px");
    expect(agentWidths()).toEqual([325]);
    expect(agentLefts()).toEqual([924]);
  });

  it("model mode fills the incomplete row with the third panel", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      dispatch({ kind: "session", session: info("/three", 3) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    await enterAgentMode("two", "three");

    expect(container.querySelectorAll('.agent-panel[data-compact="true"]')).toHaveLength(3);
    expect(container.querySelectorAll('.agent-col[data-grid-row="lower"]')).toHaveLength(1);
    expect(agentWidths()).toEqual([1480, 740, 740]);
    expect(agentLefts()).toEqual([0, 0, 740]);
    expect(agentCols().map((col) => col.style.top)).toEqual(["50%", "0%", "0%"]);
    expect(agentCols().map((col) => col.style.height)).toEqual(["50%", "50%", "50%"]);
    expect(container.querySelector<HTMLElement>(".workspace-area")!.style.getPropertyValue("--editor-right")).toBe("1480px");
  });

  it.each([3, 5, 7])("fills every Agent Mode row with %i panels after resizing", async (count) => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      for (let generation = 2; generation <= count; generation++) {
        dispatch({ kind: "session", session: info(`/repo-${generation}`, generation) });
      }
    });
    await enterAgentMode(...Array.from({ length: count - 1 }, (_, index) => `repo-${index + 2}`));
    expect(agentCols()).toHaveLength(count);
    for (const width of [1480, 1637, 901]) {
      await act(async () => setWidth(width));
      for (const top of ["0%", "50%"]) {
        const row = agentCols().filter((col) => col.style.top === top).sort((a, b) => Number.parseFloat(a.style.left) - Number.parseFloat(b.style.left));
        expect(row.length).toBeGreaterThan(0);
        expect(Number.parseFloat(row[0].style.left)).toBe(0);
        row.forEach((col, index) => {
          const right = Number.parseFloat(col.style.left) + Number.parseFloat(col.style.width);
          expect(right).toBe(index + 1 < row.length ? Number.parseFloat(row[index + 1].style.left) : width);
        });
      }
    }
  });

  it.each(["rail", "close"])("preserves Agent Mode and its collapsed sidebar through Settings (%s)", async (exit) => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      sideTab(container, "Sessions").click();
      dispatch({ kind: "session", session: info("/two", 2) });
    });
    await enterAgentMode("two");
    const beforeWidths = agentWidths();
    const settings = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-settings"]')!;
    await act(async () => settings.click());
    expect(container.querySelector(".settings-sidebar")).not.toBeNull();
    expect(Number.parseFloat(gridCols()[0])).toBeGreaterThan(0);
    await act(async () => {
      (exit === "rail" ? settings : container.querySelector<HTMLButtonElement>('.settings-close')!).click();
    });
    expect(container.querySelector(".app.agent-mode")).not.toBeNull();
    expect(gridCols()[0]).toBe("0px");
    expect(sideTab(container, "Sessions").getAttribute("aria-pressed")).toBe("false");
    expect(agentWidths()).toEqual(beforeWidths);
    await act(async () => container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!.click());
    expect(sideTab(container, "Sessions").getAttribute("aria-pressed")).toBe("true");
    expect(gridCols()[0]).toBe("230px");
  });

  it("keeps a manually widened normal-mode panel across Settings geometry changes", async () => {
    const observers = new Set<() => void>();
    vi.stubGlobal("ResizeObserver", class {
      constructor(private callback: () => void) {}
      observe() { observers.add(this.callback); }
      disconnect() { observers.delete(this.callback); }
    });
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(function (this: HTMLElement) {
      if (!this.classList.contains("workspace-area")) return 0;
      const columns = gridCols();
      return window.innerWidth - Number.parseFloat(columns[0]) - Number.parseFloat(columns[1]);
    });
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => sideTab(container, "Files").click());
    await act(async () => observers.forEach((callback) => callback()));
    await act(async () => {
      const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1200 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 145 }));
      window.dispatchEvent(new MouseEvent("mouseup"));
    });
    expect(agentWidths()).toEqual([1380]);
    const beforeLefts = agentLefts();
    await act(async () => container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-settings"]')!.click());
    await act(async () => observers.forEach((callback) => callback()));
    await act(async () => container.querySelector<HTMLButtonElement>('.settings-close')!.click());
    expect(gridCols()[0]).toBe("0px");
    expect(agentWidths()).toEqual([1380]);
    expect(agentLefts()).toEqual(beforeLefts);

    await act(async () => container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-settings"]')!.click());
    await act(async () => setWidth(1000));
    await act(async () => observers.forEach((callback) => callback()));
    await act(async () => container.querySelector<HTMLButtonElement>('.settings-close')!.click());
    expect(agentWidths()).toEqual([1000]);
  });

  it("preserves Changes and collapsed sidebar when closing Settings", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => sideTab(container, "Files").click());
    await act(async () => container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-changes"]')!.click());
    await act(async () => container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-settings"]')!.click());
    await act(async () => container.querySelector<HTMLButtonElement>('.settings-close')!.click());
    expect(gridCols()[0]).toBe("0px");
    expect(container.querySelector('[data-testid="changes-view"]')).not.toBeNull();
  });

  it("restores a single agent to its default right-anchored width after agent mode", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const modeButton = (): HTMLButtonElement =>
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!;

    await act(async () => {
      modeButton().click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(gridCols()[0]).toBe("0px");
    expect(agentWidths()).toEqual([1480]);
    expect(agentLefts()).toEqual([0]);
    expect(agentLefts()[0] + agentWidths()[0]).toBe(1480);

    await act(async () => {
      modeButton().click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(gridCols()[0]).toBe("230px");
    expect(agentWidths()).toEqual([325]);
    expect(agentLefts()).toEqual([924]);
  });

  it("exits agent mode without restoring when the user manually resizes a panel", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    const mode = container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!;

    await act(async () => {
      mode.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(mode.getAttribute("aria-pressed")).toBe("true");

    const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
    const widthBeforeDrag = agentWidths()[0];
    await act(async () => {
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 1000 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 1300 }));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(mode.getAttribute("aria-pressed")).toBe("true");
    expect(agentWidths()[0]).toBe(widthBeforeDrag);

    await act(async () => {
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(mode.getAttribute("aria-pressed")).toBe("false");
    expect(gridCols()[0]).toBe("0px");
  });

  it("reopens a collapsed model panel when entering model mode", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-agent-panel"]')!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(0);

    await enterAgentMode();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    expect(agentWidths()[0]).toBeGreaterThan(1000);
  });

  it("model mode duplicates panels from its add control and stops at eight", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    const modeButton = (): HTMLButtonElement =>
      container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!;

    await act(async () => {
      modeButton().click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    const add = (): HTMLButtonElement => container.querySelector<HTMLButtonElement>('button[aria-label="Add model panel"]')!;
    expect(add().getAttribute("aria-label")).toBe("Add model panel");
    const selectFolder = vi.spyOn(window.vela, "selectFolder");

    await act(async () => {
      add().click();
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(selectFolder).toHaveBeenCalledWith(expect.any(Number));
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(2);
    expect(agentWidths()).toEqual([740, 740]);

    for (let index = 0; index < 6; index += 1) {
      await act(async () => {
        add().click();
        await new Promise((resolve) => setTimeout(resolve, 30));
      });
    }
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(8);
    expect(agentCols().map((col) => col.style.height)).toEqual(Array(8).fill("50%"));
    expect(agentLefts()).toEqual([0, 0, 370, 370, 740, 740, 1110, 1110]);
    expect(add().disabled).toBe(true);

    await act(async () => {
      modeButton().click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(gridCols()[0]).toBe("230px");
    expect(agentWidths()).toEqual([325]);
    expect(agentLefts()).toEqual([924]);
  });

  it("keeps the agent add control beside Agent Mode and can add a panel from coding mode", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise(resolve => setTimeout(resolve, 20)));
    const mode = container.querySelector('[data-panel-action="toggle-model-mode"]')!;
    const add = container.querySelector<HTMLButtonElement>('.titlebar-actions [data-panel-action="add-model-panel"]')!;
    expect(mode.nextElementSibling).toBe(add);
    expect(add.classList.contains("icon-btn")).toBe(true);
    expect(container.querySelector('.activity-rail [data-panel-action="add-model-panel"]')).toBeNull();
    await act(async () => { add.click(); await new Promise(resolve => setTimeout(resolve, 30)); });
    expect(container.querySelector(".app")?.classList.contains("agent-mode")).toBe(true);
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(2);
  });

  it("only the plus control adds a model panel", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    const openSession = vi.spyOn(window.vela, "openSession");
    const modeButton = (): HTMLButtonElement => container.querySelector<HTMLButtonElement>('[data-panel-action="toggle-model-mode"]')!;

    await act(async () => {
      modeButton().click();
      await new Promise((resolve) => setTimeout(resolve, 20));
      modeButton().click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(openSession).not.toHaveBeenCalled();
    expect(container.querySelectorAll(".agent-panel")).toHaveLength(1);
    expect(container.querySelector('.titlebar-actions [data-panel-action="add-model-panel"]')).not.toBeNull();
  });

  it("preserves an enlarged tray across a session re-render", async () => {
    const parent = { ...info("/repo", 1), title: "Main agent" };
    window.vela = {
      ...api(),
      state: async () => parent,
      activeSessions: async () => [parent],
      sessions: async () => [{ id: parent.id, title: "Main agent", directory: "/repo", updatedAt: 2 }],
      openSessionById: async () => {
        dispatch({ kind: "session", session: parent });
        return { session: parent, transcript: [], todos: [], usage: null };
      }
    };

    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 40)));

    const handle = container.querySelector<HTMLElement>(".agent-col .panel-resize-left")!;
    await act(async () => {
      handle.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, clientX: 900 }));
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 780 }));
      window.dispatchEvent(new MouseEvent("mouseup", {}));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    const enlarged = agentWidths()[0];
    expect(enlarged).toBeGreaterThan(280);

    await act(async () => {
      await window.vela.openSessionById(parent.id, parent.workspace.generation);
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(agentWidths()).toEqual([enlarged]);
    window.history.replaceState(null, "", "/");
  });

  it("closing a quadrant tray restores the default side-by-side trays", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      dispatch({ kind: "session", session: info("/three", 3) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    await enterAgentMode("two", "three");

    expect(agentCols()).toHaveLength(3);
    expect(agentCols().map((col) => col.style.height)).toEqual(["50%", "50%", "50%"]);

    await closePanelFromOptions(0);

    expect(agentCols()).toHaveLength(2);
    expect(agentCols().map((col) => col.style.height)).toEqual(["100%", "100%"]);
    expect(agentWidths()).toEqual([740, 740]);
  });

  it("places agent mode beside the collapsed sidebar", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    await enterAgentMode("two");

    expect(gridCols()[0]).toBe("0px");
    expect(container.querySelector(".sidebar.collapsed")).toBeNull();
    expect(agentWidths()).toEqual([740, 740]);
    expect(agentLefts()).toEqual([0, 740]);
  });

  it("keeps model quadrants spanning the workspace after a window resize", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      dispatch({ kind: "session", session: info("/three", 3) });
      dispatch({ kind: "session", session: info("/four", 4) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    await enterAgentMode("two", "three", "four");

    await act(async () => {
      setWidth(900);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(agentCols().map((col) => col.style.left)).toEqual(["0px", "0px", "450px", "450px"]);
    expect(agentCols().map((col) => col.style.height)).toEqual(["50%", "50%", "50%", "50%"]);
    expect(agentWidths()).toEqual([450, 450, 450, 450]);
  });

  it("keeps back-to-back Agent Mode panels at their stops after focus", async () => {
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));
    await act(async () => {
      dispatch({ kind: "session", session: info("/two", 2) });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await enterAgentMode("two");
    expect(agentLefts()).toEqual([0, 740]);

    // Focusing the other live panel does not alter the Agent Mode layout.
    await act(async () => {
      agentCols()[1].querySelector<HTMLElement>(".agent-header")!.dispatchEvent(
        new MouseEvent("mousedown", { bubbles: true, clientX: 1000, clientY: 10 })
      );
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(agentLefts()).toEqual([0, 740]);
  });

  it("stays in the IDE with an Open Workspace action after closing the last session", async () => {
    await act(async () => root.render(<App />));
    // Let the startup restore populate the open-session list before switching
    // away from the default Files tab.
    await act(async () => new Promise((resolve) => setTimeout(resolve, 80)));

    expect(container.querySelector(".app")).not.toBeNull();
    expect(container.querySelector(".welcome")).toBeNull();

    await act(async () => {
      sideTab(container, "Sessions").click();
      await new Promise((resolve) => setTimeout(resolve, 80));
    });

    await act(async () => {
      container.querySelector<HTMLButtonElement>(".sessions-row-close")!.click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(container.querySelector(".app")).not.toBeNull();
    expect(container.querySelector(".welcome")).toBeNull();

    await act(async () => {
      sideTab(container, "Files").click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    const cta = container.querySelector<HTMLButtonElement>(".sessions-new");
    expect(cta?.textContent?.trim()).toBe("Open Workspace");

    const selectFolder = vi.spyOn(window.vela, "selectFolder");
    await act(async () => {
      cta!.click();
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(selectFolder).toHaveBeenCalled();
  });

  it("opens an empty IDE without a welcome picker or automatic session on cold start", async () => {
    window.vela = {
      ...api(),
      state: async () => null,
      activeSessions: async () => []
    };
    const selectFolder = vi.spyOn(window.vela, "selectFolder");
    const openSession = vi.spyOn(window.vela, "openSession");
    const windowView = vi.spyOn(window.vela, "windowView");
    await act(async () => root.render(<App />));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)));

    expect(selectFolder).not.toHaveBeenCalled();
    expect(openSession).not.toHaveBeenCalled();
    expect(windowView).toHaveBeenCalledExactlyOnceWith("session");
    expect(container.querySelector(".welcome")).toBeNull();
    expect(container.querySelector(".app")).not.toBeNull();
    expect(container.querySelector<HTMLButtonElement>(".sessions-new")?.textContent?.trim()).toBe("Open Workspace");
    expect(container.querySelector(".agent-col .agent-panel")).not.toBeNull();
    expect(container.querySelector(".welcome .rowlink")).toBeNull();
  });
});
