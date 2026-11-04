import { afterEach, expect, it } from "vitest";
import { compile } from "sass";

const styles: HTMLStyleElement[] = [];
function style(file: string): void {
  const element = document.createElement("style");
  element.textContent = compile(`src/renderer/src/styles/${file}.scss`).css;
  document.head.append(element);
  styles.push(element);
}
afterEach(() => { styles.splice(0).forEach((element) => element.remove()); document.body.innerHTML = ""; });
it("keeps Explorer layout out of the Monaco suggestions container", () => {
  style("_sidebar");
  document.body.innerHTML = '<div class="sidebar"><div class="tree" id="explorer"></div></div><div class="monaco-editor"><div class="suggest-widget"><div class="tree" id="suggestions"></div></div></div>';
  expect(getComputedStyle(document.getElementById("explorer")!).display).toBe("flex");
  expect(getComputedStyle(document.getElementById("suggestions")!).display).not.toBe("flex");
  expect(getComputedStyle(document.getElementById("suggestions")!).paddingTop).not.toBe("2px");
});
it("keeps file and browser tabs visible during Changes review", () => {
  style("_changes");
  document.body.innerHTML = '<div class="app changes-open"><div class="titlebar-tabs-slot"><button>index.ts</button><button>Browser</button></div><div class="workspace-area"><div class="editor-pane"></div><div class="changes-view"></div></div></div>';
  expect(getComputedStyle(document.querySelector(".titlebar-tabs-slot")!).display).not.toBe("none");
  expect(getComputedStyle(document.querySelector(".editor-pane")!).visibility).toBe("hidden");
});
it("draws the missing horizontal divider in the lower row of an agent grid", () => {
  style("_codex");
  document.body.innerHTML = '<div data-shell="codex"><div class="app agent-mode"><div class="agent-col" data-grid-row="lower"><div class="agent-panel"></div></div></div></div>';
  expect(getComputedStyle(document.querySelector(".agent-panel")!).borderTopWidth).toBe("1px");
});
