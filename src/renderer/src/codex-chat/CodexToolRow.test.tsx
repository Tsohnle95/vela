import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ToolCallView } from "@shared/types";
import { CodexToolRow } from "./CodexToolRow";

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
  act(() => root.render(<CodexToolRow tool={value} />));
}

describe("CodexToolRow", () => {
  it("collapses an exec row to 'Ran <command>' and expands to the shell card", () => {
    render(tool({}));
    expect(container.querySelector(".codex-tool-summary")?.textContent).toBe("Ran ls");
    expect(container.querySelector(".codex-exec-card")).toBeNull();
    act(() => container.querySelector<HTMLButtonElement>(".codex-tool-head")?.click());
    expect(container.querySelector(".codex-exec-output")?.textContent).toContain("package.json");
    expect(container.querySelector(".codex-exec-status")?.textContent).toContain("Success");
  });

  it("summarises read rows without an expander", () => {
    render(tool({ title: "read", detail: "src/renderer/a.ts", inputValue: { filePath: "src/renderer/a.ts" }, output: "" }));
    expect(container.querySelector(".codex-tool-summary")?.textContent).toBe("Read src/renderer/a.ts");
    expect(container.querySelector(".codex-tool-head")?.hasAttribute("data-expandable")).toBe(false);
  });

  it("summarises web rows", () => {
    render(tool({ title: "websearch", detail: "codex parity", inputValue: { query: "codex parity" }, output: "" }));
    expect(container.querySelector(".codex-tool-summary")?.textContent).toBe("Searched the web for codex parity");
  });

  it("renders a patch row with a diff body", () => {
    render(tool({
      title: "edit",
      detail: "src/a.ts",
      inputValue: { filePath: "src/a.ts" },
      output: "@@ -1 +1 @@\n-old\n+new"
    }));
    expect(container.querySelector(".codex-tool-summary")?.textContent).toBe("Edited src/a.ts");
    act(() => container.querySelector<HTMLButtonElement>(".codex-tool-head")?.click());
    expect(container.querySelector(".codex-diff [data-kind='add']")?.textContent).toBe("+new");
    expect(container.querySelector(".codex-diff [data-kind='del']")?.textContent).toBe("-old");
  });

  it("shimmers the summary while running", () => {
    render(tool({ status: "running" }));
    expect(container.querySelector(".codex-tool-summary")?.hasAttribute("data-running")).toBe(true);
  });
});
