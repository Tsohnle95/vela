import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, ToolCallView } from "@shared/types";
import { ContextToolGroup, ToolPart } from "./OpenCodeTool";

const store = vi.hoisted(() => ({ agents: [], sessions: [], openFile: vi.fn(), focusSession: vi.fn(), reopenSession: vi.fn() }));
vi.mock("../store", () => ({ useStore: () => store }));
vi.mock("./OpenCodeFile", () => ({ OpenCodeFile: () => <div data-component="file-preview" /> }));
vi.mock("motion", () => ({ animate: vi.fn(() => ({ stop: vi.fn() })) }));

const session: SessionInfo = { id: "session", directory: "/repo", workspace: { id: "workspace", generation: 7 } };
function tool(path = "/repo/src/file.ts"): ToolCallView {
  return { id: "call", title: "read", detail: "", status: "success", input: JSON.stringify({ filePath: path }) };
}

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("tool file activation ownership", () => {
  let root: Root;
  let container: HTMLDivElement;
  beforeEach(() => {
    vi.clearAllMocks();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => setTimeout(() => callback(0), 0));
    vi.stubGlobal("cancelAnimationFrame", clearTimeout);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it.each(["edit", "error", "native", "context"])("focuses the owning session before opening a %s path", (kind) => {
    const value = tool();
    if (kind === "edit") value.title = "edit";
    if (kind === "error") { value.status = "failed"; value.output = "Error: read: Not found"; }
    if (kind === "native") value.metadata = { deepseek: { resultView: { card: "read", path: "/repo/src/file.ts", title: "file.ts" } } };
    act(() => root.render(kind === "context" ? <ContextToolGroup tools={[value]} session={session} /> : <ToolPart tool={value} session={session} />));
    if (kind === "context") act(() => container.querySelector<HTMLButtonElement>('[data-slot="collapsible-trigger"]')!.click());
    const path = kind === "edit" ? '[data-slot="message-part-title-filename"]' : '[data-slot="basic-tool-tool-subtitle"]';
    act(() => container.querySelector<HTMLElement>(path)!.click());
    expect(store.focusSession).toHaveBeenCalledExactlyOnceWith("session");
    expect(store.openFile).toHaveBeenCalledExactlyOnceWith("src/file.ts", session.workspace);
    expect(store.focusSession.mock.invocationCallOrder[0]).toBeLessThan(store.openFile.mock.invocationCallOrder[0]);
  });

  it.each(["/outside/file.ts", "/repo-other/file.ts", "/repo"])("rejects an unconfined %s path before focusing", (path) => {
    act(() => root.render(<ToolPart tool={{ ...tool(path), title: "edit" }} session={session} />));
    act(() => container.querySelector<HTMLElement>('[data-slot="message-part-title-filename"]')!.click());
    expect(store.focusSession).not.toHaveBeenCalled();
    expect(store.openFile).not.toHaveBeenCalled();
  });

  it("retains the shell controller's disclosure state across failed and successful branches", () => {
    const shell = { ...tool(), title: "shell", input: '{"command":"pwd"}', output: "/repo", status: "running" as const };
    act(() => root.render(<ToolPart tool={shell} session={session} />));
    act(() => container.querySelector<HTMLButtonElement>('[data-slot="collapsible-trigger"]')!.click());
    expect(container.querySelector('[data-slot="bash-pre"]')?.textContent).toBe("$ pwd\n\n/repo");
    act(() => root.render(<ToolPart tool={{ ...shell, status: "failed", output: "Error: shell: Failed" }} session={session} />));
    expect(container.querySelector('[data-kind="tool-error-card"]')).not.toBeNull();
    act(() => root.render(<ToolPart tool={{ ...shell, status: "success" }} session={session} />));
    expect(container.querySelector<HTMLButtonElement>('[data-slot="collapsible-trigger"]')!.getAttribute("aria-expanded")).toBe("true");
    expect(container.querySelector('[data-slot="bash-pre"]')?.textContent).toBe("$ pwd\n\n/repo");
  });

  it("keeps dormant native calls nested and renders partial command JSON without losing terminal cleanup", () => {
    const native = { ...tool(), title: "code", metadata: { deepseek: { subCalls: [null, {}, {
      id: "nested", title: "shell", input: '{"command":"echo nested"', output: "\u001b[32mNested\u001b[0m\r\n"
    }] } } };
    act(() => root.render(<ToolPart tool={native} session={session} />));
    act(() => container.querySelector<HTMLButtonElement>('[data-slot="collapsible-trigger"]')!.click());
    const nested = container.querySelector<HTMLElement>('[data-timeline-part-id="nested"]')!;
    expect(container.querySelectorAll('[data-component="nested-tool-calls"]')).toHaveLength(1);
    act(() => nested.querySelector<HTMLButtonElement>('[data-slot="collapsible-trigger"]')!.click());
    expect(nested.querySelector('[data-slot="bash-pre"]')?.textContent).toBe("$ echo nested\n\nNested\n");
  });

  it("keeps hidden todo, dismissed-question, error and edit routing in the established order", () => {
    act(() => root.render(<ToolPart tool={{ ...tool(), title: "todowrite", status: "failed" }} session={session} />));
    expect(container.textContent).toBe("");
    act(() => root.render(<ToolPart tool={{ ...tool(), title: "question", status: "failed", output: "Error: User dismissed this question" }} session={session} />));
    expect(container.querySelector('[data-kind="tool-error-card"]')).toBeNull();
    expect(container.textContent).toContain("dismissed");
    for (const title of ["task", "edit"]) {
      act(() => root.render(<ToolPart tool={{ ...tool(), title, status: "failed", output: "Error: Failed" }} session={session} />));
      expect(container.querySelector('[data-kind="tool-error-card"]')).not.toBeNull();
      expect(container.querySelector('[data-component="task-tool-card"]')).toBeNull();
      expect(container.querySelector('[data-component="edit-tool"]')).toBeNull();
    }
  });
});

describe("eager tool formatting failure behavior", () => {
  it.each(["task", "edit", "todowrite", "shell"])("retains cyclic input serialization failure before the %s branch", (title) => {
    const input: Record<string, unknown> = {};
    input.cycle = input;
    expect(() => renderToStaticMarkup(<ToolPart tool={{ ...tool(), title, inputValue: input }} session={session} />)).toThrow(TypeError);
  });

  it("retains BigInt serialization failure before rendering a failed tool card", () => {
    expect(() => renderToStaticMarkup(<ToolPart tool={{ ...tool(), status: "failed", inputValue: { number: 1n } }} session={session} />)).toThrow(TypeError);
  });
});
