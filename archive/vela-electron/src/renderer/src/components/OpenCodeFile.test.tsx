import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OpenCodeFile } from "./OpenCodeFile";
import { resolveFileDiff } from "../opencode-file/diff-source";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("OpenCodeFile", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    const report = console.error;
    vi.spyOn(console, "error").mockImplementation((...args: unknown[]) => {
      // jsdom 26 cannot parse Pierre's CSS layers; browser stylesheet rendering is checked live.
      if (String(args[0]).includes("Could not parse CSS stylesheet")) return;
      report(...args);
    });
    vi.stubGlobal("Worker", undefined);
    vi.stubGlobal("ResizeObserver", class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    });
    Object.defineProperty(CSSStyleSheet.prototype, "replaceSync", { configurable: true, value: vi.fn() });
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() })));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    document.documentElement.style.removeProperty("color-scheme");
  });

  const shadow = (): ShadowRoot | null | undefined => container.querySelector("diffs-container")?.shadowRoot;

  it("renders actual Pierre unified rows and syntax tokens, then updates the same diff host", async () => {
    const onRendered = vi.fn();
    await act(async () => root.render(<OpenCodeFile mode="diff" file="counter.ts" before="const count = 1;\n" after="const count = 2;\n" onRendered={onRendered} />));
    await vi.waitFor(() => expect(shadow()?.querySelectorAll("[data-line]").length).toBe(2), { timeout: 10000 });
    expect(shadow()?.querySelector('[data-line][data-line-type="change-deletion"]')?.textContent).toContain("1");
    expect(shadow()?.querySelector('[data-line][data-line-type="change-addition"]')?.textContent).toContain("2");
    await vi.waitFor(() => expect(shadow()?.querySelector('[data-line] span[style*="--syntax-keyword"]')?.textContent).toBe("const"));
    await vi.waitFor(() => expect(onRendered).toHaveBeenCalled());
    const host = container.querySelector("diffs-container");
    await act(async () => root.render(<OpenCodeFile mode="diff" file="counter.ts" before="const count = 1;\n" after="const count = 3;\n" />));
    await vi.waitFor(() => expect(shadow()?.querySelector('[data-line][data-line-type="change-addition"]')?.textContent).toContain("3"));
    expect(container.querySelector("diffs-container")).toBe(host);
  });

  it("renders write contents as a file and tears down a replaced viewer", async () => {
    await act(async () => root.render(<OpenCodeFile mode="text" file="hello.ts" contents={'export const message = "hello";\n'} />));
    await vi.waitFor(() => expect(shadow()?.querySelector("[data-line]")?.textContent).toContain('"hello"'), { timeout: 10000 });
    expect(shadow()?.querySelector("[data-file]")).not.toBeNull();
    expect(shadow()?.querySelector("[data-diff]")).toBeNull();
    const oldHost = container.querySelector("diffs-container");
    await act(async () => root.render(<OpenCodeFile mode="diff" file="hello.ts" before="old\n" after="new\n" />));
    await vi.waitFor(() => expect(shadow()?.querySelector("[data-diff]")).not.toBeNull());
    expect(container.querySelector("diffs-container")).not.toBe(oldHost);
    expect(oldHost?.isConnected).toBe(false);
  });

  it("keeps partial patch line numbers and full patch no-newline semantics", () => {
    const partial = resolveFileDiff({ file: "a.ts", patch: "@@ -40,2 +40,2 @@\n context\n-old\n+new\n" });
    expect(partial.isPartial).toBe(true);
    expect(partial.hunks[0]?.additionStart).toBe(40);
    const full = resolveFileDiff({ file: "a.ts", patch: "diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n" });
    expect(full.isPartial).toBe(false);
    expect(full.deletionLines.join("")).toBe("old");
    expect(full.additionLines.join("")).toBe("new");
  });

  it("uses the narrow viewport gutter rule and follows Vela's selected color scheme", async () => {
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() })));
    document.documentElement.style.colorScheme = "light";
    await act(async () => root.render(<OpenCodeFile mode="diff" file="a.ts" before="one\n" after="two\n" />));
    await vi.waitFor(() => expect(shadow()?.querySelector("[data-diff][data-disable-line-numbers]")).not.toBeNull());
    await vi.waitFor(() => expect(container.querySelector("diffs-container")?.getAttribute("data-color-scheme")).toBe("light"));
    document.documentElement.style.colorScheme = "dark";
    await vi.waitFor(() => expect(container.querySelector("diffs-container")?.getAttribute("data-color-scheme")).toBe("dark"));
  });
});
