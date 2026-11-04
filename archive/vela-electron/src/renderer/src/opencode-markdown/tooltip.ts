// DOM adapter for OpenCode TooltipV2 at 03e67171ab2dc1e7f16e8cebfbc7f778f61b89f0 (MIT; see LICENSE).
import { autoUpdate, computePosition, flip, offset, shift, size } from "@floating-ui/dom";

const roots = new WeakMap<HTMLElement, { count: number; dispose: () => void }>();
let rootCount = 0;
let nextID = 0;
let warmUntil = 0;
let active: { close: () => void; isOpen: () => boolean } | undefined;

/** Installs TooltipV2 behavior for wrapper elements carrying a data-tooltip label. */
export function mountTooltip(root: HTMLElement, portalRoot = root): () => void {
  const existing = roots.get(root);
  if (existing) {
    existing.count++;
    return release;
  }
  rootCount++;
  const doc = root.ownerDocument;
  const win = doc.defaultView!;
  const blocked = new Set<HTMLElement>();
  const exits = new Map<HTMLDivElement, ReturnType<typeof setTimeout>>();
  let trigger: HTMLElement | undefined;
  let hovered: HTMLElement | undefined;
  let pending: ReturnType<typeof setTimeout> | undefined;
  let view: { positioner: HTMLDivElement; content: HTMLDivElement; cleanup: () => void } | undefined;
  let disposed = false;
  const controller = { close: () => close(), isOpen: () => !!view };

  const expanded = (element: HTMLElement) => !!element.querySelector('[aria-expanded="true"], [data-expanded]');
  const inside = (element: HTMLElement, target: EventTarget | null) => target instanceof Node && element.contains(target);
  const ownedTrigger = (target: EventTarget | null) => {
    const element = target instanceof Element ? target.closest<HTMLElement>("[data-tooltip]") : null;
    if (!element || !root.contains(element)) return;
    for (let parent: HTMLElement | null = element; parent; parent = parent.parentElement) {
      if (roots.has(parent)) return parent === root ? element : undefined;
    }
  };
  const clearPending = () => {
    if (pending !== undefined) clearTimeout(pending);
    pending = undefined;
  };
  const clearExit = (positioner: HTMLDivElement) => {
    clearTimeout(exits.get(positioner));
    exits.delete(positioner);
    positioner.remove();
  };
  const close = () => {
    clearPending();
    trigger?.removeAttribute("aria-describedby");
    trigger?.removeAttribute("data-expanded");
    trigger?.setAttribute("data-closed", "");
    trigger = undefined;
    if (active === controller) active = undefined;
    if (!view) return;
    warmUntil = Date.now() + 300;
    const previous = view;
    view = undefined;
    previous.cleanup();
    previous.content.removeAttribute("data-expanded");
    previous.content.setAttribute("data-closed", "");
    if (win.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      previous.positioner.remove();
      return;
    }
    previous.content.addEventListener("animationend", () => clearExit(previous.positioner), { once: true });
    exits.set(previous.positioner, setTimeout(() => clearExit(previous.positioner), 80));
  };
  const show = (element: HTMLElement) => {
    clearPending();
    if (disposed || !root.contains(element) || blocked.has(element) || expanded(element)) return;
    if (view && trigger === element) return;
    active?.close();
    trigger = element;
    active = controller;
    const positioner = doc.createElement("div");
    positioner.setAttribute("data-popper-positioner", "");
    Object.assign(positioner.style, { position: "fixed", top: "0", left: "0", minWidth: "max-content", zIndex: "1000" });
    positioner.style.setProperty("--kb-popper-content-overflow-padding", "8px");
    const content = doc.createElement("div");
    content.id = `opencode-tooltip-${++nextID}-content`;
    content.setAttribute("role", "tooltip");
    content.dataset.component = element.dataset.component === "tooltip-trigger" ? "tooltip" : "tooltip-v2";
    content.dataset.placement = "top";
    content.setAttribute("data-expanded", "");
    content.style.position = "relative";
    content.style.setProperty("--kb-tooltip-content-transform-origin", "var(--kb-popper-content-transform-origin)");
    content.textContent = element.dataset.tooltip ?? "";
    const theme = element.closest("[data-theme]")?.getAttribute("data-theme");
    if (theme) content.setAttribute("data-theme", theme);
    positioner.append(content);
    portalRoot.append(positioner);
    element.setAttribute("aria-describedby", content.id);
    element.removeAttribute("data-closed");
    element.setAttribute("data-expanded", "");

    const update = async () => {
      const result = await computePosition(element, positioner, {
        placement: "top",
        strategy: "fixed",
        middleware: [
          offset(4),
          flip({ padding: 8 }),
          shift({ mainAxis: true, crossAxis: false, padding: 8 }),
          size({ padding: 8, apply({ availableWidth, availableHeight, rects }) {
            positioner.style.setProperty("--kb-popper-anchor-width", `${Math.round(rects.reference.width)}px`);
            positioner.style.setProperty("--kb-popper-content-available-width", `${Math.floor(availableWidth)}px`);
            positioner.style.setProperty("--kb-popper-content-available-height", `${Math.floor(availableHeight)}px`);
          } })
        ]
      });
      if (view?.positioner !== positioner) return;
      positioner.style.setProperty("--kb-popper-content-transform-origin", result.placement === "top" ? "bottom center" : "top center");
      positioner.style.transform = `translate3d(${Math.round(result.x)}px, ${Math.round(result.y)}px, 0)`;
    };
    view = { positioner, content, cleanup: () => {} };
    view.cleanup = autoUpdate(element, positioner, () => void update(), { elementResize: typeof ResizeObserver === "function" });
  };
  const enter = (element: HTMLElement, immediate = false) => {
    if (blocked.has(element) || expanded(element) || !element.dataset.tooltip) return;
    if (trigger === element) {
      if (immediate && !view) show(element);
      return;
    }
    const skip = immediate || active?.isOpen() || warmUntil > Date.now();
    active?.close();
    close();
    trigger = element;
    active = controller;
    if (skip) show(element);
    else pending = setTimeout(() => show(element), 400);
  };
  const pointerOver = (event: PointerEvent) => {
    if (event.pointerType === "touch") return;
    const element = ownedTrigger(event.target);
    if (!element || inside(element, event.relatedTarget)) return;
    hovered = element;
    enter(element);
  };
  const pointerOut = (event: PointerEvent) => {
    if (event.pointerType === "touch") return;
    const element = ownedTrigger(event.target);
    if (!element || inside(element, event.relatedTarget)) return;
    hovered = undefined;
    if (trigger === element) close();
    if (!inside(element, doc.activeElement) && !expanded(element)) blocked.delete(element);
  };
  const focusIn = (event: FocusEvent) => {
    const element = ownedTrigger(event.target);
    if (element) enter(element, true);
  };
  const focusOut = (event: FocusEvent) => {
    const element = ownedTrigger(event.target);
    if (!element || inside(element, event.relatedTarget)) return;
    if (trigger === element) close();
    if (hovered !== element && !expanded(element)) blocked.delete(element);
  };
  const arm = (event: Event) => {
    const element = ownedTrigger(event.target);
    if (!element) return;
    blocked.add(element);
    if (trigger === element) close();
  };
  const keyDown = (event: KeyboardEvent) => {
    if (event.key === "Enter" || event.key === " ") arm(event);
  };
  const escape = (event: KeyboardEvent) => {
    if (event.key === "Escape" && active === controller) close();
  };
  const scroll = (event: Event) => {
    if (trigger && event.target instanceof Node && event.target.contains(trigger)) close();
  };
  const observer = new MutationObserver(() => {
    blocked.forEach((element) => {
      if (!root.contains(element) || (hovered !== element && !inside(element, doc.activeElement) && !expanded(element))) blocked.delete(element);
    });
    if (!trigger) return;
    if (!root.contains(trigger) || expanded(trigger)) {
      if (expanded(trigger)) blocked.add(trigger);
      close();
      return;
    }
    if (view && view.content.textContent !== trigger.dataset.tooltip) view.content.textContent = trigger.dataset.tooltip ?? "";
  });
  observer.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-tooltip", "aria-expanded", "data-expanded"] });
  root.addEventListener("pointerover", pointerOver);
  root.addEventListener("pointerout", pointerOut);
  root.addEventListener("focusin", focusIn);
  root.addEventListener("focusout", focusOut);
  root.addEventListener("pointerdown", arm, true);
  root.addEventListener("click", arm, true);
  root.addEventListener("keydown", keyDown, true);
  doc.addEventListener("keydown", escape);
  win.addEventListener("scroll", scroll, true);
  roots.set(root, { count: 1, dispose: () => {
    disposed = true;
    observer.disconnect();
    close();
    exits.forEach((_timeout, positioner) => clearExit(positioner));
    root.removeEventListener("pointerover", pointerOver);
    root.removeEventListener("pointerout", pointerOut);
    root.removeEventListener("focusin", focusIn);
    root.removeEventListener("focusout", focusOut);
    root.removeEventListener("pointerdown", arm, true);
    root.removeEventListener("click", arm, true);
    root.removeEventListener("keydown", keyDown, true);
    doc.removeEventListener("keydown", escape);
    win.removeEventListener("scroll", scroll, true);
  } });
  return release;

  function release() {
    const entry = roots.get(root);
    if (!entry || --entry.count > 0) return;
    entry.dispose();
    roots.delete(root);
    if (--rootCount === 0) warmUntil = 0;
  }
}
