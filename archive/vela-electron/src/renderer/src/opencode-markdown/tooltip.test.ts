import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mountTooltip } from "./tooltip";

describe("OpenCode TooltipV2 DOM adapter", () => {
  let root: HTMLDivElement;
  let cleanup: () => void;
  const pointer = (node: HTMLElement, type: string, relatedTarget: HTMLElement | null = null) => {
    node.dispatchEvent(new MouseEvent(type, { bubbles: true, relatedTarget }));
  };
  const trigger = (label: string, parent = root) => {
    const element = document.createElement("div");
    element.dataset.component = "tooltip-v2-trigger";
    element.dataset.tooltip = label;
    const button = document.createElement("button");
    button.textContent = label;
    element.append(button);
    parent.append(element);
    vi.spyOn(element, "getBoundingClientRect").mockReturnValue(new DOMRect(50, 100, 20, 20));
    return element;
  };

  beforeEach(() => {
    vi.useFakeTimers();
    root = document.createElement("div");
    document.body.append(root);
    cleanup = mountTooltip(root);
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) {
      return this.hasAttribute("data-popper-positioner") ? 80 : this.hasAttribute("data-tooltip") ? 20 : 1024;
    });
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) {
      return this.hasAttribute("data-popper-positioner") ? 22 : this.hasAttribute("data-tooltip") ? 20 : 768;
    });
    vi.spyOn(document.documentElement, "clientWidth", "get").mockReturnValue(1024);
    vi.spyOn(document.documentElement, "clientHeight", "get").mockReturnValue(768);
  });

  afterEach(() => {
    cleanup();
    root.remove();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("opens after 400ms, closes without a delay, and skips the next delay for 300ms", async () => {
    const first = trigger("Copy");
    const second = trigger("Revert");
    pointer(first, "pointerover");
    await vi.advanceTimersByTimeAsync(399);
    expect(root.querySelector('[role="tooltip"]')).toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    const content = root.querySelector<HTMLElement>('[role="tooltip"]')!;
    expect(content.textContent).toBe("Copy");
    expect(content.dataset.component).toBe("tooltip-v2");
    expect(content.parentElement?.hasAttribute("data-popper-positioner")).toBe(true);
    expect(first.getAttribute("aria-describedby")).toBe(content.id);
    pointer(first, "pointerout", second);
    expect(content.hasAttribute("data-closed")).toBe(true);
    expect(first.hasAttribute("aria-describedby")).toBe(false);
    pointer(second, "pointerover", first);
    expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Revert");
    pointer(second, "pointerout");
    await vi.advanceTimersByTimeAsync(301);
    pointer(first, "pointerover");
    expect(root.querySelector('[role="tooltip"]')).toBeNull();
    await vi.advanceTimersByTimeAsync(400);
    expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Copy");
  });

  it("uses immediate keyboard focus and suppresses reopening after click until focus and pointer leave", async () => {
    const element = trigger("Copy");
    const button = element.querySelector("button")!;
    button.focus();
    expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Copy");
    pointer(element, "pointerover");
    pointer(button, "pointerdown");
    button.click();
    expect(root.querySelector('[role="tooltip"][data-expanded]')).toBeNull();
    element.dataset.tooltip = "Copied";
    await vi.advanceTimersByTimeAsync(400);
    expect(root.querySelector('[role="tooltip"]')).toBeNull();
    pointer(element, "pointerout");
    pointer(element, "pointerover");
    await vi.advanceTimersByTimeAsync(400);
    expect(root.querySelector('[role="tooltip"]')).toBeNull();
    pointer(element, "pointerout");
    button.blur();
    pointer(element, "pointerover");
    await vi.advanceTimersByTimeAsync(400);
    expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Copied");
  });

  it("uses Floating UI's 4px gutter, 8px overflow padding, and flip placement", async () => {
    const element = trigger("Copy");
    element.querySelector("button")!.focus();
    await vi.waitFor(() => expect(root.querySelector<HTMLElement>("[data-popper-positioner]")?.style.transform).toBe("translate3d(20px, 74px, 0)"));
    pointer(element, "pointerout");
    element.querySelector("button")!.blur();
    vi.mocked(element.getBoundingClientRect).mockReturnValue(new DOMRect(2, 6, 20, 20));
    element.querySelector("button")!.focus();
    const positioner = root.querySelector<HTMLElement>('[data-popper-positioner]:has([data-expanded])')!;
    await vi.waitFor(() => expect(positioner.style.transform).toBe("translate3d(8px, 30px, 0)"));
    expect(positioner.style.getPropertyValue("--kb-popper-content-transform-origin")).toBe("top center");
    expect(positioner.style.getPropertyValue("--kb-popper-content-overflow-padding")).toBe("8px");
  });

  it("updates labels, closes for expanded controls and scrolling, and ignores touch hover", async () => {
    const element = trigger("Copy");
    const button = element.querySelector("button")!;
    const touch = new MouseEvent("pointerover", { bubbles: true });
    Object.defineProperty(touch, "pointerType", { value: "touch" });
    element.dispatchEvent(touch);
    await vi.advanceTimersByTimeAsync(500);
    expect(root.querySelector('[role="tooltip"]')).toBeNull();
    button.focus();
    element.dataset.tooltip = "Copied";
    await Promise.resolve();
    expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Copied");
    button.setAttribute("aria-expanded", "true");
    await Promise.resolve();
    expect(root.querySelector('[role="tooltip"][data-expanded]')).toBeNull();
    button.removeAttribute("aria-expanded");
    button.blur();
    await Promise.resolve();
    button.focus();
    expect(root.querySelector('[role="tooltip"][data-expanded]')).not.toBeNull();
    root.dispatchEvent(new Event("scroll"));
    expect(root.querySelector('[role="tooltip"][data-expanded]')).toBeNull();
    button.blur();
    button.focus();
    expect(root.querySelector('[role="tooltip"][data-expanded]')).not.toBeNull();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(root.querySelector('[role="tooltip"][data-expanded]')).toBeNull();
  });

  it("shares delay skipping across mounted roots and gives nested mounts sole ownership", async () => {
    const first = trigger("First");
    const nested = document.createElement("div");
    root.append(nested);
    const releaseNested = mountTooltip(nested, root);
    try {
      const second = trigger("Second", nested);
      pointer(first, "pointerover");
      await vi.advanceTimersByTimeAsync(400);
      pointer(first, "pointerout", second);
      pointer(second, "pointerover", first);
      expect(root.querySelectorAll('[role="tooltip"][data-expanded]')).toHaveLength(1);
      expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Second");
      releaseNested();
      expect(root.querySelector('[role="tooltip"][data-expanded]')).toBeNull();
    } finally {
      releaseNested();
    }
  });

  it("lets keyboard focus supersede a pending hover even across separate roots", async () => {
    const first = trigger("First");
    const nested = document.createElement("div");
    root.append(nested);
    const releaseNested = mountTooltip(nested, root);
    try {
      const second = trigger("Second", nested);
      pointer(first, "pointerover");
      await vi.advanceTimersByTimeAsync(100);
      second.querySelector("button")!.focus();
      expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Second");
      await vi.advanceTimersByTimeAsync(400);
      expect(root.querySelectorAll('[role="tooltip"][data-expanded]')).toHaveLength(1);
      expect(root.querySelector('[role="tooltip"][data-expanded]')?.textContent).toBe("Second");
    } finally {
      releaseNested();
    }
  });
});
