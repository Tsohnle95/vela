import { act } from "react";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo } from "@shared/types";

let currentSession: SessionInfo;
let panelBusy = false;
const stop = vi.fn(async () => {});
const runCommand = vi.fn(async () => {});
const sendPrompt = vi.fn(async () => {});

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
    sendPrompt,
    runCommand,
    stop,
    busy: false
  }),
  usePanel: () => ({
    session: currentSession,
    busy: panelBusy,
    transcript: [],
    todos: [],
    sessionUsage: null,
    models: [],
    currentModel: null,
    agents: [],
    currentAgent: null
  })
}));

import { Composer } from "./Composer";

function session(id: string, generation: number): SessionInfo {
  return {
    id,
    directory: `/workspace-${generation}`,
    workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation }
  };
}

function type(input: HTMLTextAreaElement, value: string): void {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

async function flush(): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 20));
  });
}

function pressEnter(input: HTMLTextAreaElement): Promise<void> {
  return act(async () => {
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  });
}

describe("composer slash and mention completions", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    currentSession = session("one", 1);
    panelBusy = false;
    stop.mockClear();
    runCommand.mockClear();
    sendPrompt.mockClear();
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

  it("keeps the running agent stop action in the split writing row", async () => {
    panelBusy = true;
    window.vela = {} as Window["vela"];
    await act(async () => root.render(<Composer compact />));
    const buttons = container.querySelectorAll<HTMLButtonElement>('[aria-label="Stop the agent"]');
    expect(buttons).toHaveLength(1);
    expect(buttons[0].classList.contains("composer-split-send")).toBe(true);
    await act(async () => buttons[0].click());
    expect(stop).toHaveBeenCalledExactlyOnceWith(currentSession.workspace);
  });

  it("uses split controls without remounting the draft when compact mode changes", async () => {
    window.vela = {} as Window["vela"];
    await act(async () => root.render(<Composer compact />));
    const input = container.querySelector("textarea")!;
    await act(async () => type(input, "Keep this draft"));
    const options = container.querySelector<HTMLButtonElement>('[aria-label="Prompt options"]');
    expect(options).not.toBeNull();
    expect(container.querySelector(".composer-toolbar")).toBeNull();
    await act(async () => options!.click());
    expect(options!.getAttribute("aria-expanded")).toBe("true");
    const attach = container.querySelector<HTMLButtonElement>('[aria-label="Attach files"]')!;
    expect(attach).not.toBeNull();
    attach.focus();
    await act(async () => attach.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(container.querySelector(".composer-toolbar")).toBeNull();
    expect(document.activeElement).toBe(options);
    await act(async () => root.render(<Composer />));
    expect(container.querySelector("textarea")).toBe(input);
    expect(input.value).toBe("Keep this draft");
    expect(container.querySelector(".composer-toolbar")).not.toBeNull();
    await act(async () => root.render(<Composer compact />));
    await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Prompt options"]')!.click());
    await act(async () => document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(container.querySelector(".composer-toolbar")).toBeNull();
    await pressEnter(input);
    expect(sendPrompt).toHaveBeenCalledWith("Keep this draft", [], currentSession.workspace);
  });

  it("lists slash commands including built-ins and runs the selected one", async () => {
    window.vela = {
      commands: vi.fn(async () => [
        { name: "compact", description: "Summarize the session to free up context", kind: "command" },
        { name: "project-cmd", description: "Project command" }
      ])
    } as unknown as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector("textarea")!;
    await act(async () => {
      type(input, "/");
    });
    await flush();

    const items = [...container.querySelectorAll(".composer-completions .composer-menu-item")];
    expect(items.map((item) => item.textContent)).toEqual([
      "compactSummarize the session to free up context",
      "project-cmdProject command"
    ]);

    await pressEnter(input);
    expect(runCommand).toHaveBeenCalledWith("compact", "", currentSession.workspace);
    expect(sendPrompt).not.toHaveBeenCalled();
  });

  it("filters slash completions as the query grows", async () => {
    window.vela = {
      commands: vi.fn(async () => [
        { name: "compact", description: "Summarize the session to free up context", kind: "command" },
        { name: "project-cmd" }
      ])
    } as unknown as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector("textarea")!;
    await act(async () => {
      type(input, "/");
    });
    await act(async () => {
      type(input, "/com");
    });
    await flush();

    const labels = [...container.querySelectorAll(".composer-completions .composer-completion-label")];
    expect(labels.map((item) => item.textContent)).toEqual(["compact"]);

    await pressEnter(input);
    expect(runCommand).toHaveBeenCalledWith("compact", "", currentSession.workspace);
  });

  it("runs a leading-slash command typed without picking from the menu", async () => {
    window.vela = {
      commands: vi.fn(async () => [
        { name: "compact", description: "Summarize the session to free up context", kind: "command" }
      ])
    } as unknown as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector("textarea")!;
    await act(async () => {
      type(input, "/compact please");
    });

    await pressEnter(input);
    expect(runCommand).toHaveBeenCalledWith("compact", "please", currentSession.workspace);
  });

  it("sends double-slash text as a prompt instead of running `/` as a command", async () => {
    window.vela = {
      commands: vi.fn(async () => [
        { name: "compact", description: "Summarize the session to free up context", kind: "command" }
      ])
    } as unknown as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector("textarea")!;
    await act(async () => {
      type(input, "//");
    });

    await pressEnter(input);
    expect(runCommand).not.toHaveBeenCalled();
    expect(sendPrompt).toHaveBeenCalledWith("//", [], currentSession.workspace);
  });

  it("clears ordinary text before the optimistic session update can rerender the composer", async () => {
    window.vela = {} as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector<HTMLTextAreaElement>("textarea")!;
    await act(async () => {
      type(input, "ordinary prompt");
    });

    let inputDuringSubmission = "";
    sendPrompt.mockImplementationOnce(async () => {
      flushSync(() => root.render(<Composer />));
      inputDuringSubmission = container.querySelector<HTMLTextAreaElement>("textarea")!.value;
    });

    await pressEnter(input);
    expect(sendPrompt).toHaveBeenCalledOnce();
    expect(inputDuringSubmission).toBe("");
    expect(input.value).toBe("");
  });

  it("clears a typed command before synchronous updates and preserves its rejection notice", async () => {
    window.vela = {} as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector<HTMLTextAreaElement>("textarea")!;
    await act(async () => type(input, "/compact please"));
    let inputDuringSubmission = "unsubmitted";
    runCommand.mockImplementationOnce(async () => {
      flushSync(() => root.render(<Composer />));
      inputDuringSubmission = container.querySelector<HTMLTextAreaElement>("textarea")!.value;
      throw new Error("Command failed");
    });

    await pressEnter(input);
    expect(runCommand).toHaveBeenCalledWith("compact", "please", currentSession.workspace);
    expect(inputDuringSubmission).toBe("");
    expect(input.value).toBe("");
    expect(container.querySelector(".composer-notice")?.textContent).toBe("Command failed");
    expect(sendPrompt).not.toHaveBeenCalled();
  });

  it("assembles a PromptFile with a mention span for a chosen file mention", async () => {
    window.vela = {
      references: vi.fn(async () => [
        { name: "foo.ts", path: "/workspace-1/src/foo.ts", rel: "src/foo.ts" }
      ])
    } as unknown as Window["vela"];
    await act(async () => root.render(<Composer />));
    const input = container.querySelector("textarea")!;
    await act(async () => {
      type(input, "explain ");
    });
    await act(async () => {
      type(input, "explain @");
    });
    await flush();

    const item = container.querySelector(".composer-completions .composer-menu-item")!;
    expect(item.textContent).toContain("src/foo.ts");
    await act(async () => {
      item.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    });

    expect(input.value).toBe("explain @src/foo.ts");

    await pressEnter(input);
    expect(runCommand).not.toHaveBeenCalled();
    expect(sendPrompt).toHaveBeenCalledWith("explain @src/foo.ts", [
      { path: "/workspace-1/src/foo.ts", mention: { start: 8, end: 19, text: "@src/foo.ts" } }
    ], currentSession.workspace);
  });
});
