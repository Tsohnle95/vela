import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ToolCallView } from "@shared/types";
import { OpenCodeGenericTool, OpenCodeSpecialTool, isDismissedOpenCodeQuestion, openCodeSpecialToolTitle } from "./OpenCodeSpecialTool";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const tool = (title: string, inputValue: Record<string, unknown> = {}, extra: Partial<ToolCallView> = {}): ToolCallView => ({
  id: "tool-1", title, inputValue, detail: "", status: "success", ...extra
});

describe("OpenCode ToolRegistry presentations", () => {
  let container: HTMLDivElement;
  let root: Root;
  const render = (value: ToolCallView) => act(() => root.render(<OpenCodeSpecialTool tool={value} />));
  const trigger = () => container.querySelector<HTMLButtonElement>('[data-slot="collapsible-trigger"]')!;
  const title = () => container.querySelector('[data-slot="basic-tool-tool-title"] [data-component="text-shimmer"]')?.getAttribute("aria-label");

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  it("shows webfetch's URL and external action only when settled, with no output disclosure", () => {
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    render(tool("webfetch", { url: "https://example.com/docs" }, { status: "running", output: "private fetched body" }));
    expect(title()).toBe("Webfetch");
    expect(container.querySelector("a")).toBeNull();
    expect(container.querySelector('[data-component="tool-action"]')).toBeNull();
    expect(container.querySelector('[data-slot="collapsible-arrow"]')).toBeNull();
    render(tool("webfetch", { url: "https://example.com/docs" }, { output: "private fetched body" }));
    const link = container.querySelector<HTMLAnchorElement>('[data-slot="basic-tool-tool-subtitle"]')!;
    expect(link.textContent).toBe("https://example.com/docs");
    expect(link.className).toBe("clickable subagent-link");
    expect(container.querySelector('[data-component="tool-action"]')).not.toBeNull();
    act(() => link.click());
    expect(open).toHaveBeenCalledWith("https://example.com/docs", "_blank", "noopener,noreferrer");
    expect(trigger().getAttribute("aria-expanded")).toBe("false");
    act(() => trigger().click());
    expect(container.querySelector('[data-slot="collapsible-content"]')).toBeNull();
    expect(container.textContent).not.toContain("private fetched body");
  });

  it("labels search providers and opens only a deduplicated list of result URLs", () => {
    const value = tool("websearch", { query: "streaming renderer" }, { status: "running", metadata: { provider: "exa" }, output: "Result https://one.test/docs, and https://two.test/next). Repeated https://one.test/docs" });
    render(value);
    expect(title()).toBe("Exa Web Search");
    expect(container.querySelector('[data-slot="basic-tool-tool-subtitle"]')?.className).toBe("exa-tool-query");
    expect(container.querySelector('[data-slot="basic-tool-tool-subtitle"]')?.textContent).toBe("streaming renderer");
    expect(container.querySelector('[data-slot="collapsible-arrow"]')).toBeNull();
    act(() => trigger().click());
    expect(trigger().getAttribute("aria-expanded")).toBe("false");
    render({ ...value, status: "success" });
    expect(container.querySelector('[data-component="exa-tool-output"]')).toBeNull();
    act(() => trigger().click());
    expect([...container.querySelectorAll('[data-slot="exa-tool-link"]')].map((link) => link.textContent))
      .toEqual(["https://one.test/docs", "https://two.test/next"]);
    expect(container.querySelector('[data-component="tool-io"]')).toBeNull();
    expect(container.textContent).not.toContain("Repeated");
    expect(openCodeSpecialToolTitle(tool("websearch", {}, { metadata: { provider: "parallel" } }))).toBe("Parallel Web Search");
    expect(openCodeSpecialToolTitle(tool("websearch", {}, { metadata: { provider: "unknown" } }))).toBe("Web Search");
  });

  it("renders skills as a capitalized title without input/output details", () => {
    render(tool("skill", { name: "code-review", extra: "not rendered" }, { output: "skill document", status: "running" }));
    expect(title()).toBe("code-review");
    expect(container.querySelector('[data-slot="basic-tool-tool-title"]')?.className).toBe("capitalize agent-title");
    expect(trigger().getAttribute("data-hide-details")).toBe("true");
    expect(container.querySelector('[data-component="text-shimmer"]')?.getAttribute("data-active")).toBe("true");
    expect(container.textContent).not.toContain("skill document");
    render(tool("skill"));
    expect(title()).toBe("Skill");
  });

  it("hides pending questions and opens completed answers with the original question labels", () => {
    const value = tool("question", { questions: [{ question: "Which language?" }, { question: "Which framework?" }] }, { status: "running" });
    render(value);
    expect(container.children).toHaveLength(0);
    render({ ...value, status: "success", metadata: { answers: [["TypeScript", "JavaScript"], []] } });
    expect(title()).toBe("Questions");
    expect(container.querySelector('[data-slot="basic-tool-tool-subtitle"]')?.textContent).toBe("2 answered");
    expect(trigger().getAttribute("aria-expanded")).toBe("true");
    expect([...container.querySelectorAll('[data-slot="question-text"]')].map((node) => node.textContent)).toEqual(["Which language?", "Which framework?"]);
    expect([...container.querySelectorAll('[data-slot="answer-text"]')].map((node) => node.textContent)).toEqual(["TypeScript, JavaScript", "(no answer)"]);
    act(() => trigger().click());
    expect(container.querySelector('[data-component="question-answers"]')).toBeNull();
  });

  it("renders dismissed questions as the upstream quiet right-aligned message", () => {
    const dismissed = tool("question", {}, { status: "failed", output: "Error: The user dismissed this question" });
    expect(isDismissedOpenCodeQuestion(dismissed)).toBe(true);
    expect(isDismissedOpenCodeQuestion(tool("question", {}, { status: "failed", output: "Internal failure" }))).toBe(false);
    render(dismissed);
    const message = container.querySelector('span')!;
    expect(message.textContent).toBe("Questions dismissed");
    expect(message.parentElement?.style.justifyContent).toBe("flex-end");
    expect(container.querySelector('[data-slot="collapsible-trigger"]')).toBeNull();
  });

  it("uses GenericTool's ordered label and first three scalar arguments, with no raw I/O", () => {
    const value = tool("custom_tool", {
      name: "ignored name", query: "second choice", description: "Selected label", path: "/tmp/ignored-label",
      limit: 40, verbose: false, options: { nested: true }, tags: ["ignored"], format: "text", fourth: "hidden"
    }, { output: "raw custom result" });
    act(() => root.render(<OpenCodeGenericTool tool={value} />));
    expect(title()).toBe("Called `custom_tool`");
    expect(container.querySelector('[data-slot="basic-tool-tool-subtitle"]')?.textContent).toBe("Selected label");
    expect([...container.querySelectorAll('[data-slot="basic-tool-tool-arg"]')].map((node) => node.textContent)).toEqual(["limit=40", "verbose=false", "format=text"]);
    expect(container.querySelector('[data-slot="collapsible-arrow"]')).toBeNull();
    act(() => trigger().click());
    expect(container.querySelector('[data-slot="collapsible-content"]')).toBeNull();
    expect(container.textContent).not.toContain("raw custom result");
  });

  it("keeps the existing HTTPS-only navigation boundary on untrusted tool links", () => {
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    render(tool("webfetch", { url: "javascript:alert(1)" }));
    const link = container.querySelector<HTMLAnchorElement>('a')!;
    expect(link.hasAttribute("href")).toBe(false);
    act(() => link.click());
    expect(open).not.toHaveBeenCalled();
  });
});
