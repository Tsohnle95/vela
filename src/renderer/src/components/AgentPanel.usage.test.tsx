import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelOption, ProviderUsageResult, SessionInfo, SessionUsage } from "@shared/types";

let currentSession: SessionInfo;
let currentUsage: SessionUsage | null;
let currentModel: ModelOption | null;
let currentProviderUsage: ProviderUsageResult[];
const refreshProviderUsage = vi.hoisted(() => vi.fn());

vi.mock("../store", () => ({
  useStore: () => ({
    busy: false,
    todos: [],
    transcript: [],
    session: currentSession,
    sessions: [],
    reopenSession: vi.fn(),
    sessionUsage: currentUsage,
    currentModel,
    models: [],
    switchModel: vi.fn(),
    agents: [],
    currentAgent: null,
    switchAgent: vi.fn(),
    loadAgents: vi.fn(),
    loadModels: vi.fn(),
    sendPrompt: vi.fn(),
    runCommand: vi.fn(),
    stop: vi.fn(),
    approvalMode: "ask",
    toggleApprovalMode: vi.fn(),
    providerUsage: currentProviderUsage,
    providerUsageLoading: false,
    refreshProviderUsage
  }),
  usePanel: () => ({
    session: currentSession,
    busy: false,
    transcript: [],
    todos: [],
    sessionUsage: currentUsage,
    models: [],
    currentModel,
    agents: [],
    currentAgent: null
  })
}));

import { AgentPanel } from "./AgentPanel";

function session(id: string, generation: number): SessionInfo {
  return {
    id,
    directory: `/workspace-${generation}`,
    workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation }
  };
}

function usage(input: number): SessionUsage {
  return {
    cost: 0,
    tokens: { input, output: 0, reasoning: 0, cache: { read: 0, write: 0 } }
  };
}

function usageWithCache(input: number, cacheRead: number, cacheWrite = 0): SessionUsage {
  return {
    cost: 0,
    tokens: { input, output: 0, reasoning: 0, cache: { read: cacheRead, write: cacheWrite } }
  };
}

function model(context: number): ModelOption {
  return { id: "m", providerID: "p", name: "Model", limit: { context } };
}

function toggle(container: HTMLDivElement): HTMLButtonElement {
  return container.querySelector(".agent-usage-toggle")!;
}

describe("agent panel usage tracker", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    currentSession = session("one", 1);
    currentUsage = usage(0);
    currentModel = model(100_000);
    currentProviderUsage = [];
    refreshProviderUsage.mockReset();
    refreshProviderUsage.mockResolvedValue(undefined);
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

  async function renderPanel() {
    await act(async () => root.render(<AgentPanel />));
    if (!container.querySelector('.agent-mode-menu')) {
      await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Agent panel options"]')!.click());
    }
  }

  it("places Context in the header menu instead of the composer", async () => {
    await act(async () => root.render(<AgentPanel />));
    expect(container.querySelector(".composer .agent-usage-toggle")).toBeNull();
    expect(toggle(container)).toBeNull();
    expect(refreshProviderUsage).not.toHaveBeenCalled();
    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Agent panel options"]')!.click());
    const entry = toggle(container);
    expect(entry.closest('.agent-header .agent-mode-menu')).not.toBeNull();
    expect(entry.getAttribute('role')).toBe('menuitem');
    expect(entry.textContent).toContain('Context');
    expect(entry.classList.contains('composer-selector')).toBe(false);
    expect(refreshProviderUsage).not.toHaveBeenCalled();
  });

  it("opens context below the header and dismisses it when reopening options or pressing Escape", async () => {
    await renderPanel();
    await act(async () => toggle(container).click());
    expect(container.querySelector('.agent-mode-menu')).toBeNull();
    const popup = container.querySelector('.agent-usage-popup')!;
    expect(popup.parentElement?.classList.contains('agent-panel')).toBe(true);
    expect(popup.closest('.composer')).toBeNull();
    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Agent panel options"]')!.click());
    expect(container.querySelector('.agent-usage-popup')).toBeNull();
    expect(toggle(container)).not.toBeNull();
    await act(async () => toggle(container).click());
    await act(async () => document.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape',bubbles:true})));
    expect(container.querySelector('.agent-usage-popup')).toBeNull();
  });

  it("colors the toggle glyph green when context fill is low", async () => {
    currentUsage = usage(10_000);
    await renderPanel();

    expect(toggle(container).classList.contains("ok")).toBe(true);
    expect(toggle(container).classList.contains("warn")).toBe(false);
    expect(toggle(container).classList.contains("danger")).toBe(false);
    expect(toggle(container).querySelector(".agent-usage-ring")).not.toBeNull();
  });

  it("colors the toggle glyph amber at 60% and red at 85% fill", async () => {
    currentUsage = usage(60_000);
    await renderPanel();
    expect(toggle(container).classList.contains("warn")).toBe(true);
    expect(toggle(container).classList.contains("danger")).toBe(false);

    currentUsage = usage(85_000);
    await renderPanel();
    expect(toggle(container).classList.contains("danger")).toBe(true);
    expect(toggle(container).classList.contains("warn")).toBe(false);
  });

  it("shows context-window utilization with a fill bar, percent, and token counts", async () => {
    currentUsage = usage(25_000);
    await renderPanel();
    await act(async () => toggle(container).click());

    const context = container.querySelector(".agent-usage-context")!;
    expect(context.querySelector(".agent-usage-context-percent")!.textContent).toBe("25%");
    expect(context.querySelector(".agent-usage-context-fill")!.getAttribute("style")).toContain("width: 25%");
    expect(context.querySelector(".agent-usage-context-fill")!.classList.contains("ok")).toBe(true);
    expect(context.querySelector(".agent-usage-context-counts")!.textContent).toContain("25.0k");
    expect(context.querySelector(".agent-usage-context-counts")!.textContent).toContain("100k tokens");
    expect(container.querySelector(".agent-usage-rows")).not.toBeNull();
    expect(container.querySelector(".agent-usage-scroll")).not.toBeNull();
  });

  it("caps the fill at 100% when input tokens exceed the context limit", async () => {
    currentUsage = usage(150_000);
    await renderPanel();
    await act(async () => toggle(container).click());

    const context = container.querySelector(".agent-usage-context")!;
    expect(context.querySelector(".agent-usage-context-percent")!.textContent).toBe("100%");
    expect(context.querySelector(".agent-usage-context-fill")!.getAttribute("style")).toContain("width: 100%");
    expect(context.querySelector(".agent-usage-context-fill")!.classList.contains("danger")).toBe(true);
  });

  it("hides the context row and stays neutral when the model has no context limit", async () => {
    currentUsage = usage(10_000);
    currentModel = { id: "m", providerID: "p", name: "Model" };
    await renderPanel();
    const classes = toggle(container).classList;
    await act(async () => toggle(container).click());

    expect(container.querySelector(".agent-usage-context")).toBeNull();
    expect(container.querySelector(".agent-usage-rows")).not.toBeNull();
    expect(classes.contains("ok")).toBe(false);
    expect(classes.contains("warn")).toBe(false);
    expect(classes.contains("danger")).toBe(false);
    expect(classes.contains("neutral")).toBe(true);
  });

  it("shows a cache percentage for cache read plus write tokens", async () => {
    currentUsage = usageWithCache(40_000, 10_000, 0);
    await renderPanel();
    await act(async () => toggle(container).click());

    const cache = container.querySelector(".agent-usage-cache-total")!;
    expect(cache.querySelector(".agent-usage-row-label")!.textContent).toBe("Cache");
    expect(cache.querySelector(".agent-usage-row-value")!.textContent).toContain("10.0k");
    expect(cache.querySelector(".agent-usage-row-value")!.textContent).toContain("20%");
  });

  it("shows the cache percentage from combined read and write tokens", async () => {
    currentUsage = usageWithCache(45_000, 10_000, 5_000);
    await renderPanel();
    await act(async () => toggle(container).click());

    const cache = container.querySelector(".agent-usage-cache-total")!;
    expect(cache.querySelector(".agent-usage-row-value")!.textContent).toContain("15.0k");
    expect(cache.querySelector(".agent-usage-row-value")!.textContent).toContain("25%");
  });

  it("omits the percentage when there is no cached traffic", async () => {
    currentUsage = usageWithCache(40_000, 0, 0);
    await renderPanel();
    await act(async () => toggle(container).click());

    const cache = container.querySelector(".agent-usage-cache-total")!;
    expect(cache.querySelector(".agent-usage-row-value")!.textContent).toContain("0");
    expect(cache.querySelector(".agent-usage-row-value")!.textContent).not.toContain("%");
  });

  it("shows provider quotas as the remaining percentage reported by the live UI", async () => {
    currentProviderUsage = [{
      provider: "openai",
      displayName: "OpenAI ChatGPT",
      status: "ok",
      snapshot: {
        windows: [
          { id: "5h", label: "5h", usedPercent: 55, windowMinutes: 300, resetsAt: null },
          { id: "weekly", label: "Weekly", usedPercent: 45, windowMinutes: 10_080, resetsAt: null }
        ],
        credits: null,
        planType: "plus",
        updatedAt: Date.now()
      }
    }];
    await renderPanel();
    await act(async () => toggle(container).click());

    expect([...container.querySelectorAll(".usage-window-value")].map((element) => element.textContent)).toEqual([
      "45% left",
      "55% left"
    ]);
  });

  it("refetches provider usage from the control and prevents duplicate in-flight requests", async () => {
    let resolveRefresh!: () => void;
    refreshProviderUsage.mockImplementation(() => new Promise<void>((resolve) => { resolveRefresh = resolve; }));
    await renderPanel();
    await act(async () => toggle(container).click());

    const refresh = container.querySelector<HTMLButtonElement>('[aria-label="Refresh provider usage"]')!;
    expect(refreshProviderUsage).toHaveBeenCalledTimes(1);
    expect(refresh.disabled).toBe(true);
    act(() => refresh.click());
    expect(refreshProviderUsage).toHaveBeenCalledTimes(1);

    await act(async () => resolveRefresh());
    act(() => refresh.click());
    expect(refreshProviderUsage).toHaveBeenCalledTimes(2);
    expect(refresh.querySelector(".codicon")?.classList.contains("spinning")).toBe(true);
  });
});
