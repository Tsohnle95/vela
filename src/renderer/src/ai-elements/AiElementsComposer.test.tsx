import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo } from "@shared/types";
import { AiElementsComposer } from "./AiElementsComposer";

const { store, view } = vi.hoisted(() => ({
  store: {
    runtimes: [], availableModels: [], lastModel: null,
    sendPrompt: vi.fn().mockResolvedValue(undefined), runCommand: vi.fn().mockResolvedValue(undefined),
    switchModel: vi.fn().mockResolvedValue(undefined), stop: vi.fn().mockResolvedValue(undefined), loadModels: vi.fn(),
  },
  view: { models: [], currentModel: null, transcript: [], busy: false, queuedMessages: [] },
}));
vi.mock("../store", () => ({ useStore: () => store, usePanel: () => view }));
(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const session: SessionInfo = { id: "chat", directory: "/repo", workspace: { id: "repo", generation: 4 }, runtimeID: "opencode" };

describe("AI Elements composer bindings", () => {
  let container: HTMLDivElement;
  let root: Root;
  beforeEach(() => {
    vi.clearAllMocks();
    view.busy = false;
    container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  });
  afterEach(() => { act(() => root.unmount()); container.remove(); });
  function render(demo?: (text: string) => void) { act(() => root.render(<AiElementsComposer session={session} onDemoPrompt={demo} />)); }
  function submit(text: string) {
    const textarea = container.querySelector("textarea")!;
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(textarea, text);
      textarea.dispatchEvent(new Event("input", { bubbles: true }));
    });
    act(() => container.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
  }
  it("sends normal prompts to the owning workspace and clears the field", () => {
    render(); submit("Review the source");
    expect(store.sendPrompt).toHaveBeenCalledWith("Review the source", [], session.workspace);
    expect(container.querySelector("textarea")!.value).toBe("");
  });
  it("routes slash commands through the existing command action", () => {
    render(); submit("/review recent changes");
    expect(store.runCommand).toHaveBeenCalledWith("review", "recent changes", session.workspace);
    expect(store.sendPrompt).not.toHaveBeenCalled();
  });
  it("keeps follow-up submission available while busy and separately binds stop", () => {
    view.busy = true; render(); submit("Also check the tests");
    expect(store.sendPrompt).toHaveBeenCalledWith("Also check the tests", [], session.workspace);
    act(() => (container.querySelector('button[aria-label="Stop"]') as HTMLButtonElement).click());
    expect(store.stop).toHaveBeenCalledWith(session.workspace);
  });
  it("submits the native attachment path rather than a display filename", () => {
    Object.defineProperty(window, "vela", { configurable: true, value: { getPathForFile: () => "/repo/assets/diagram.png" } });
    render();
    const input = container.querySelector('input[type="file"]')!;
    Object.defineProperty(input, "files", { configurable: true, value: [new File(["image"], "diagram.png")] });
    act(() => input.dispatchEvent(new Event("change", { bubbles: true })));
    act(() => container.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
    expect(store.sendPrompt).toHaveBeenCalledWith("", [{ path: "/repo/assets/diagram.png" }], session.workspace);
  });
  it("keeps replay prompts local and disables real runtime controls", () => {
    const demo = vi.fn(); render(demo); submit("Compare this stream");
    expect(demo).toHaveBeenCalledWith("Compare this stream");
    expect(store.sendPrompt).not.toHaveBeenCalled(); expect(store.runCommand).not.toHaveBeenCalled();
    expect((container.querySelector('button[aria-label="Attach files"]') as HTMLButtonElement).disabled).toBe(true);
  });
});
