import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TodoItem } from "@shared/types";
import { OpenCodeTodoDock } from "./OpenCodeTodoDock";

describe("OpenCodeTodoDock", () => {
  const containers: HTMLDivElement[] = [];

  beforeEach(() => vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true));

  afterEach(() => {
    for (const container of containers) container.remove();
    containers.length = 0;
    vi.unstubAllGlobals();
  });

  it("does not turn ordinary tool activity into a todo plan", () => {
    const container = document.createElement("div");
    containers.push(container);
    document.body.append(container);
    const root = createRoot(container);
    act(() => root.render(<OpenCodeTodoDock todos={[]} />));

    expect(container.querySelector("[data-component='session-todo-dock']")).toBeNull();
    act(() => root.unmount());
  });

  it("shows only the agent's structured todo plan", () => {
    const container = document.createElement("div");
    containers.push(container);
    document.body.append(container);
    const root = createRoot(container);
    const todos: TodoItem[] = [
      { id: "inspect", content: "Inspect the layout", status: "completed" },
      { id: "fix", content: "Fix the overflow", status: "in_progress" }
    ];

    act(() => root.render(<OpenCodeTodoDock todos={todos} />));

    expect(container.querySelector("[data-slot='session-todo-progress']")?.textContent).toBe("1 of 2 steps completed");
    expect([...container.querySelectorAll("[data-slot='todo-content']")].map((item) => item.textContent)).toEqual([
      "Inspect the layout",
      "Fix the overflow"
    ]);
    act(() => root.unmount());
  });
});
