import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, ModelOption } from "@shared/types";
import type { AgentOption } from "@shared/types";

let currentSession: SessionInfo;
let currentAgents: AgentOption[];
let currentModels: ModelOption[];
let currentModel: ModelOption | null;
const loadAgentsMock = vi.fn();
const loadModelsMock = vi.fn();
const switchModelMock = vi.fn();

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
    models: currentModels,
    currentModel: currentModel,
    switchModel: switchModelMock,
    agents: currentAgents,
    currentAgent: null,
    switchAgent: vi.fn(),
    loadAgents: loadAgentsMock,
    loadModels: loadModelsMock,
    sendPrompt: vi.fn(),
    runCommand: vi.fn(),
    stop: vi.fn(),
    busy: false
  }),
  usePanel: () => ({
    session: currentSession,
    busy: false,
    transcript: [],
    todos: [],
    sessionUsage: null,
    models: currentModels,
    currentModel: currentModel,
    agents: currentAgents,
    currentAgent: null
  })
}));

import { Composer } from "./Composer";

describe("composer agent menu", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    loadAgentsMock.mockClear();
    loadModelsMock.mockClear();
    switchModelMock.mockClear();
    currentAgents = [];
    currentModels = [];
    currentModel = null;
    window.localStorage.clear();
    currentSession = {
      id: "one",
      directory: "/workspace",
      workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 }
    };
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

  it("disables correction and capitalization in composer text controls", async () => {
    currentModels = [{ id: "gpt", name: "GPT", providerID: "openai" }];
    await act(async () => root.render(<Composer />));

    const composer = container.querySelector<HTMLTextAreaElement>(".composer-input")!;
    expect(composer.getAttribute("spellcheck")).toBe("false");
    expect(composer.getAttribute("autocorrect")).toBe("off");
    expect(composer.getAttribute("autocapitalize")).toBe("off");

    const modelButton = container.querySelector<HTMLButtonElement>(
      'button[title="Change model and response strength"]'
    )!;
    await act(async () => modelButton.click());
    const search = container.querySelector<HTMLInputElement>(".composer-model-search")!;
    expect(search.getAttribute("spellcheck")).toBe("false");
    expect(search.getAttribute("autocorrect")).toBe("off");
    expect(search.getAttribute("autocapitalize")).toBe("off");
  });

  it("keeps the inset composer controls on one row and attaches files from its add button", async () => {
    await act(async () => root.render(<Composer />));

    const input = container.querySelector<HTMLInputElement>('.composer-file-input')!;
    const openPicker = vi.spyOn(input, "click");
    await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Attach files"]')!.click());
    expect(openPicker).toHaveBeenCalledOnce();
    expect(container.querySelector(".composer-chip-break")).toBeNull();

    vi.stubGlobal("vela", { getPathForFile: vi.fn(() => "/workspace/notes.txt") });
    Object.defineProperty(input, "files", { configurable: true, value: [new File(["notes"], "notes.txt")] });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(container.querySelector(".composer-attachment")?.textContent).toContain("notes.txt");
  });

  it("reloads agents and shows a hint when the list is empty", async () => {
    await act(async () => root.render(<Composer />));
    const button = container.querySelector<HTMLButtonElement>('button[title="Change agent"]')!;
    await act(async () => button.click());

    expect(loadAgentsMock).toHaveBeenCalledTimes(1);
    const menu = container.querySelector(".composer-menu");
    expect(menu?.textContent).toContain("No agents available");
    expect(menu?.querySelector(".composer-menu-item")).toBeNull();
  });

  it("lists agents without reloading when they are already loaded", async () => {
    currentAgents = [
      { id: "build", name: "Build" },
      { id: "plan", name: "Plan" }
    ];
    await act(async () => root.render(<Composer />));
    const button = container.querySelector<HTMLButtonElement>('button[title="Change agent"]')!;
    await act(async () => button.click());

    expect(loadAgentsMock).not.toHaveBeenCalled();
    const items = container.querySelectorAll(".composer-menu-item");
    expect(items).toHaveLength(2);
    expect(items[0].textContent).toContain("Build");
    expect(items[1].textContent).toContain("Plan");
  });

  it("shows a gold star for favorited models listed in the Favorites section", async () => {
    currentModels = [{ id: "gpt", name: "GPT", providerID: "openai" }];
    window.localStorage.setItem("favoriteModels", JSON.stringify(["openai::gpt"]));
    await act(async () => root.render(<Composer />));
    const modelButton = container.querySelector<HTMLButtonElement>(
      'button[title="Change model and response strength"]'
    )!;
    await act(async () => modelButton.click());

    const favoritesGroup = [...container.querySelectorAll(".composer-menu-group")].find((group) =>
      group.textContent?.includes("Favorites")
    );
    expect(favoritesGroup).not.toBeNull();
    const star = favoritesGroup!.querySelector(".composer-menu-star");
    expect(star).not.toBeNull();
    expect(star!.className).toContain("on");
  });

  it("keeps favorited models visible even when their provider is hidden", async () => {
    currentModels = [{ id: "gpt", name: "GPT", providerID: "openai" }];
    window.localStorage.setItem("favoriteModels", JSON.stringify(["openai::gpt"]));
    window.localStorage.setItem("hiddenProviders", JSON.stringify(["openai"]));
    await act(async () => root.render(<Composer />));
    const modelButton = container.querySelector<HTMLButtonElement>(
      'button[title="Change model and response strength"]'
    )!;
    await act(async () => modelButton.click());

    const favoritesGroup = [...container.querySelectorAll(".composer-menu-group")].find((group) =>
      group.textContent?.includes("Favorites")
    );
    expect(favoritesGroup).not.toBeNull();
    expect(favoritesGroup!.textContent).toContain("GPT");
  });

  it("toggles stars in either model list without selecting the row or closing the menu", async () => {
    currentModels = [{ id: "gpt", name: "GPT", providerID: "openai" }];
    await act(async () => root.render(<Composer />));
    await act(async () => container.querySelector<HTMLButtonElement>('button[title="Change model and response strength"]')!.click());
    await act(async () => container.querySelector<HTMLElement>('.composer-menu-star[title="Add to favorites"]')!.click());
    expect(switchModelMock).not.toHaveBeenCalled();
    expect(container.querySelector(".composer-menu")).not.toBeNull();
    expect(JSON.parse(window.localStorage.getItem("favoriteModels")!)).toEqual(["openai::gpt"]);
    const favoriteStar = container.querySelector<HTMLElement>(".composer-menu-favorites .composer-menu-star")!;
    expect(favoriteStar.classList.contains("on")).toBe(true);
    await act(async () => favoriteStar.click());
    expect(switchModelMock).not.toHaveBeenCalled();
    expect(JSON.parse(window.localStorage.getItem("favoriteModels")!)).toEqual([]);
    expect(container.querySelector(".composer-menu-favorites")).toBeNull();
  });

  it("dismisses menus only for outside mouse clicks or Escape", async () => {
    currentModels = [{ id: "gpt", name: "GPT", providerID: "openai" }];
    await act(async () => root.render(<Composer />));
    const toggle = container.querySelector<HTMLButtonElement>('button[title="Change model and response strength"]')!;
    await act(async () => toggle.click());
    await act(async () => container.querySelector(".composer-menu")!.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(container.querySelector(".composer-menu")).not.toBeNull();
    await act(async () => document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(container.querySelector(".composer-menu")).toBeNull();
    await act(async () => toggle.click());
    await act(async () => document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(container.querySelector(".composer-menu")).toBeNull();
  });

  it("reaches response strength from the model menu when the model has variants", async () => {
    currentModels = [{ id: "gpt", name: "GPT", providerID: "openai", variants: ["low", "high"] }];
    currentModel = { id: "gpt", name: "GPT", providerID: "openai", variants: ["low", "high"], variant: "high" };
    await act(async () => root.render(<Composer />));
    const modelButton = container.querySelector<HTMLButtonElement>(
      'button[title="Change model and response strength"]'
    )!;
    await act(async () => modelButton.click());

    const menu = container.querySelector<HTMLElement>(".composer-menu")!;
    const strength = menu.querySelector<HTMLButtonElement>('button[title="Change response strength"]')!;
    expect(strength).not.toBeNull();
    expect(strength.textContent).toContain("Response strength");
    expect(strength.textContent).toContain("High");

    await act(async () => strength.click());
    expect(container.querySelector(".variant-menu")).not.toBeNull();
  });
});
