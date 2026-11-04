import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ToolCallView } from "@shared/types";
import { VelaToolRow } from "./VelaToolRow";

let container: HTMLDivElement;
let root: Root;

function tool(overrides: Partial<ToolCallView>): ToolCallView {
  return {
    id: "call-1",
    title: "bash",
    detail: "ls",
    status: "success",
    input: "{}",
    inputValue: { command: "ls" },
    output: "src\npackage.json",
    ...overrides
  };
}

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

function render(value: ToolCallView): void {
  act(() => root.render(<VelaToolRow tool={value} />));
}

describe("VelaToolRow", () => {
  it("collapses an exec row to 'Ran <command>' and expands to the shell card", () => {
    render(tool({}));
    expect(container.querySelector(".vela-tool-summary")?.textContent).toBe("Ran ls");
    expect(container.querySelector(".vela-exec-card")).toBeNull();
    act(() => container.querySelector<HTMLButtonElement>(".vela-tool-head")?.click());
    expect(container.querySelector(".vela-exec-output")?.textContent).toContain("package.json");
    expect(container.querySelector(".vela-exec-status")?.textContent).toContain("Success");
  });

  it("summarises read rows without an expander", () => {
    render(tool({ title: "read", detail: "src/renderer/a.ts", inputValue: { filePath: "src/renderer/a.ts" }, output: "" }));
    expect(container.querySelector(".vela-tool-summary")?.textContent).toBe("Read src/renderer/a.ts");
    expect(container.querySelector(".vela-tool-head")?.hasAttribute("data-expandable")).toBe(false);
  });

  it("summarises web rows", () => {
    render(tool({ title: "websearch", detail: "vela parity", inputValue: { query: "vela parity" }, output: "" }));
    expect(container.querySelector(".vela-tool-summary")?.textContent).toBe("Searched the web for vela parity");
  });

  it("renders a patch row with a diff body", () => {
    render(tool({
      title: "edit",
      detail: "src/a.ts",
      inputValue: { filePath: "src/a.ts" },
      output: "@@ -1 +1 @@\n-old\n+new"
    }));
    expect(container.querySelector(".vela-tool-summary")?.textContent).toBe("Edited src/a.ts");
    act(() => container.querySelector<HTMLButtonElement>(".vela-tool-head")?.click());
    expect(container.querySelector(".vela-diff [data-kind='add']")?.textContent).toBe("+new");
    expect(container.querySelector(".vela-diff [data-kind='del']")?.textContent).toBe("-old");
  });

  it("shimmers the summary while running", () => {
    render(tool({ status: "running" }));
    expect(container.querySelector(".vela-tool-summary")?.hasAttribute("data-running")).toBe(true);
  });
});
