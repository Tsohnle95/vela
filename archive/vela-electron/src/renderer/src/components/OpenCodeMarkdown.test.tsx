import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { InProcessMarkdownWorker } from "../opencode-markdown/markdown-test-worker";
import { OpenCodeMarkdown } from "./OpenCodeMarkdown";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("OpenCodeMarkdown", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("Worker", InProcessMarkdownWorker);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  const render = async (text: string, streaming = true) => {
    await act(async () => root.render(<OpenCodeMarkdown text={text} streaming={streaming} cacheKey="test-message" />));
  };

  it("keeps finished prose nodes and completed code nodes while the live tail grows", async () => {
    const prefix = "# Heading\n\nFirst **paragraph**.\n\n```ts\nconst value = 1\n```\n\n";
    await render(prefix + "A live");
    await vi.waitFor(() => expect(container.querySelector("h1")?.textContent).toBe("Heading"), { timeout: 5000 });
    await vi.waitFor(() => expect(container.querySelector(".shiki span")?.textContent).toBe("const"));
    const heading = container.querySelector("h1");
    const code = container.querySelector("code");
    const token = code?.firstElementChild;
    await render(prefix + "A live **answer**.");
    await vi.waitFor(() => expect([...container.querySelectorAll("p strong")].at(-1)?.textContent).toBe("answer"));
    expect(container.querySelector("h1")).toBe(heading);
    expect(container.querySelector("code")).toBe(code);
    expect(code?.firstElementChild).toBe(token);
    expect(container.querySelectorAll('[data-slot="markdown-copy-button"]')).toHaveLength(1);
  });

  it("updates open code through Shiki's stable tokens and resets a non-prefix replacement", async () => {
    await render("```ts\nconst value = 1;\n");
    await vi.waitFor(() => expect(container.querySelector("code")?.textContent).toBe("const value = 1;\n"), { timeout: 5000 });
    await vi.waitFor(() => expect(container.querySelector("code span")?.getAttribute("style")).toContain("--syntax-keyword"));
    const first = container.querySelector("code span");
    await render("```ts\nconst value = 1;\nconsole.log(value);\n");
    await vi.waitFor(() => expect(container.querySelector("code")?.textContent).toContain("console.log(value);"));
    expect(container.querySelector("code span")).toBe(first);
    await render("```ts\nlet other = 2;\n");
    await vi.waitFor(() => expect(container.querySelector("code")?.textContent).toBe("let other = 2;\n"));
    expect(container.querySelector("code span")).not.toBe(first);
    await render("```ts\nlet other = 2;\n```", false);
    await vi.waitFor(() => expect(container.querySelector('[data-markdown-complete="true"]')).not.toBeNull());
  });

  it("uses upstream code decorations, copies code, and keeps Vela's external URL policy", async () => {
    const open = vi.spyOn(window, "open").mockImplementation(() => null);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    await render("```sh\npwd\n```\n\n`src/index.ts` and `https://opencode.ai`.\n\n[Unsafe](javascript:alert) [HTTP](http://example.com) [Safe](https://example.com)\n\n<img src=x onerror=alert(1)><script>alert(1)</script>");
    await vi.waitFor(() => expect(container.querySelector('[data-code-kind="shell"]')).not.toBeNull(), { timeout: 5000 });
    expect(container.querySelector('[data-inline-code-kind="path"]')?.textContent).toBe("src/index.ts");
    expect(container.querySelector('[data-inline-code-kind="url"]')?.parentElement?.tagName).toBe("A");
    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("img")?.hasAttribute("onerror")).toBe(false);
    const links = [...container.querySelectorAll("a")];
    expect(links.find((link) => link.textContent === "Unsafe")?.getAttribute("href")).toBeNull();
    expect(links.find((link) => link.textContent === "HTTP")?.getAttribute("href")).toBeNull();
    links.find((link) => link.textContent === "Safe")?.click();
    expect(open).toHaveBeenCalledWith("https://example.com/", "_blank", "noopener,noreferrer");
    await act(async () => (container.querySelector('[data-slot="markdown-copy-button"] button') as HTMLButtonElement).click());
    expect(writeText).toHaveBeenCalledWith("pwd");
    expect(container.querySelector('[data-slot="markdown-copy-button"]')?.getAttribute("data-copied")).toBe("true");
  });
});
