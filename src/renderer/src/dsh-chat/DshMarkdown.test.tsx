import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DshMarkdown } from "./DshMarkdown";

describe("DshMarkdown", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    if (root) await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("shows appended streaming text immediately and keeps frozen block DOM stable", async () => {
    await act(async () => root.render(<DshMarkdown text={"First paragraph."} streaming />));
    const first = container.querySelector("p");
    expect(first?.textContent).toBe("First paragraph.");

    await act(async () => root.render(<DshMarkdown text={"First paragraph.\n\nSecond paragraph."} streaming />));
    expect(container.textContent).toBe("First paragraph.\nSecond paragraph.");
    expect(container.querySelector("p")).toBe(first);
    expect(container.querySelectorAll("p")).toHaveLength(2);
  });

  it("swaps the streaming parse for the settled parse without changing the text", async () => {
    await act(async () => root.render(<DshMarkdown text={"**bold** text"} streaming />));
    expect(container.textContent).toBe("bold text");
    await act(async () => root.render(<DshMarkdown text={"**bold** text"} streaming={false} />));
    expect(container.textContent).toBe("bold text");
    expect(container.querySelector("strong")?.textContent).toBe("bold");
  });
});
