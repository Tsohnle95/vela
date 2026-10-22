import { afterEach, expect, it } from "vitest";
import { compile } from "sass";

afterEach(() => { document.head.innerHTML = ""; document.body.innerHTML = ""; document.documentElement.removeAttribute("data-theme"); });

function mount(): void {
  const style = document.createElement("style");
  style.textContent = compile("src/renderer/src/styles/_agent.scss").css + compile("src/renderer/src/styles/_foundation.scss").css;
  document.head.append(style);
  document.body.innerHTML = '<section class="agent-panel" data-panel-mode="gui"><header class="agent-header"><button>Options</button></header><div class="agent-scroll">Conversation</div></section>';
}

it("lets conversation scrolling reach the panel top beside the options control", () => {
  mount();
  const header = getComputedStyle(document.querySelector(".agent-header")!);
  expect(header.position).toBe("absolute");
  expect(header.top).toBe("0px");
  expect(header.right).toBe("0px");
  expect(header.width).toBe("max-content");
});

it("does not reprocess the window glass separately for each agent pane", () => {
  mount();
  document.documentElement.dataset.theme = "kitty";
  const panel = getComputedStyle(document.querySelector(".agent-panel")!);
  expect(panel.backgroundColor).toBe("rgba(0, 0, 0, 0)");
  // jsdom does not compute backdrop filters; the native smoke checks final paint.
  const rules = [...document.querySelector("style")!.sheet!.cssRules] as CSSStyleRule[];
  const glass = rules.find((rule) => rule.selectorText === ':root[data-theme=kitty] .agent-panel');
  expect(glass?.style.getPropertyValue("backdrop-filter")).toBe("none");
  expect(glass?.style.getPropertyValue("-webkit-backdrop-filter")).toBe("none");
  expect(panel.boxShadow).toBe("none");
});

it("keeps TUI controls above terminal cells", () => {
  mount();
  document.querySelector(".agent-panel")!.setAttribute("data-panel-mode", "tui");
  expect(getComputedStyle(document.querySelector(".agent-header")!).position).toBe("relative");
});
