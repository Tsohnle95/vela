import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "./ErrorBoundary";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function Bomb(): never {
  throw new Error("boom");
}

describe("renderer error boundary", () => {
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

  it("paints a visible error state instead of unmounting the app", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    act(() => root.render(<ErrorBoundary><Bomb /></ErrorBoundary>));
    expect(container.querySelector(".error-boundary")).not.toBeNull();
    expect(container.textContent).toContain("boom");
  });

  it("renders children when nothing throws", () => {
    act(() => root.render(<ErrorBoundary><p>fine</p></ErrorBoundary>));
    expect(container.textContent).toContain("fine");
  });
});
