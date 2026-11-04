import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExternalLink } from "./ExternalLink";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("controlled external links", () => {
  let container: HTMLDivElement;
  let root: Root;

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

  it("prevents in-app navigation and opens safe Markdown targets externally", () => {
    const open = vi.spyOn(window, "open").mockImplementation(() => null);
    act(() => root.render(<ExternalLink href="https://example.com/docs">Docs</ExternalLink>));
    const anchor = container.querySelector("a")!;
    const click = new MouseEvent("click", { bubbles: true, cancelable: true });

    anchor.dispatchEvent(click);

    expect(click.defaultPrevented).toBe(true);
    expect(open).toHaveBeenCalledWith("https://example.com/docs", "_blank", "noopener,noreferrer");
  });

  it.each(["javascript:alert(1)", "file:///tmp/tool-output.txt", "http://example.com/"])(
    "renders unsafe Markdown and tool targets inert: %s",
    (href) => {
      const open = vi.spyOn(window, "open").mockImplementation(() => null);
      act(() => root.render(<ExternalLink href={href}>Attachment</ExternalLink>));
      const anchor = container.querySelector("a")!;
      const click = new MouseEvent("click", { bubbles: true, cancelable: true });

      anchor.dispatchEvent(click);

      expect(anchor.hasAttribute("href")).toBe(false);
      expect(click.defaultPrevented).toBe(true);
      expect(open).not.toHaveBeenCalled();
    }
  );
});
