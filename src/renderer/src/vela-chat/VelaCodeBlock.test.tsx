import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VelaCodeBlock } from "./VelaCodeBlock";

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

describe("VelaCodeBlock", () => {
  it("shows the language, code and both controls", () => {
    act(() => {
      root.render(<VelaCodeBlock code="const a = 1;" lang="ts" streaming />);
    });
    expect(container.querySelector(".vela-code-lang")?.textContent).toBe("ts");
    expect(container.textContent).toContain("const a = 1;");
    expect(container.textContent).toContain("Wrap");
    expect(container.textContent).toContain("Copy");
  });

  it("falls back to a plain text language label", () => {
    act(() => {
      root.render(<VelaCodeBlock code="hello" streaming />);
    });
    expect(container.querySelector(".vela-code-lang")?.textContent).toBe("text");
  });
});
