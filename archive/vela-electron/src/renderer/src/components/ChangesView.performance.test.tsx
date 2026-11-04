import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentFileState } from "@shared/types";
import { ChangesView } from "./ChangesView";

const mocks = vi.hoisted(() => ({
  files: new Map<string, AgentFileState>(),
  diffLines: vi.fn(() => [{ value: "line\n", added: true, count: 1 }])
}));

// The per-file diff statistics are the expensive synchronous work; count them.
vi.mock("diff", () => ({ diffLines: mocks.diffLines }));

vi.mock("../store", () => ({
  useStore: () => ({
    session: { id: "session", directory: "/workspace", workspace: { id: "ws", generation: 1 } },
    agentFiles: mocks.files,
    hiddenPaths: new Set<string>(),
    previewFile: vi.fn(),
    dismissChange: vi.fn(),
    dismissChanges: vi.fn()
  })
}));

vi.mock("./OpenCodeFile", () => ({
  OpenCodeFile: ({ file }: { file: string }) => <div data-testid="diff" data-file={file} />
}));

interface FakeObserver {
  targets: Set<Element>;
  trigger(element: Element): void;
}
const observers: FakeObserver[] = [];

class FakeIntersectionObserver implements FakeObserver {
  targets = new Set<Element>();
  root = null;
  rootMargin = "";
  thresholds: number[] = [];
  constructor(private readonly callback: IntersectionObserverCallback) { observers.push(this); }
  observe(element: Element): void { this.targets.add(element); }
  unobserve(element: Element): void { this.targets.delete(element); }
  disconnect(): void { this.targets.clear(); }
  takeRecords(): IntersectionObserverEntry[] { return []; }
  trigger(element: Element): void {
    this.callback(
      [{ isIntersecting: true, target: element } as unknown as IntersectionObserverEntry],
      this as unknown as IntersectionObserver
    );
  }
}

const FILE_COUNT = 40;

let container: HTMLDivElement;
let root: Root;

describe("ChangesView viewport work", () => {
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.stubGlobal("IntersectionObserver", FakeIntersectionObserver);
    observers.length = 0;
    mocks.diffLines.mockClear();
    mocks.files.clear();
    for (let index = 0; index < FILE_COUNT; index += 1) {
      mocks.files.set(`src/file-${index}.ts`, {
        baseline: { kind: "known", content: "before\n".repeat(400) },
        content: "after\n".repeat(400),
        deleted: false
      });
    }
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  function render(): void {
    act(() => root.render(<ChangesView onClose={() => {}} />));
  }

  it("does not diff any file while every section is outside the viewport", () => {
    render();

    expect(container.querySelectorAll(".change-section")).toHaveLength(FILE_COUNT);
    expect(mocks.diffLines).not.toHaveBeenCalled();
    expect(container.querySelectorAll(".change-stats")).toHaveLength(0);
    expect(container.querySelectorAll('[data-testid="diff"]')).toHaveLength(0);
    // One shared observer for the whole view, not one per changed file.
    expect(observers).toHaveLength(1);
  });

  it("computes stats and mounts the diff only for sections that become visible", () => {
    render();
    const section = container.querySelector<HTMLElement>('[data-change-path="src/file-7.ts"]')!;

    act(() => observers[0].trigger(section));

    expect(mocks.diffLines).toHaveBeenCalledTimes(1);
    expect(container.querySelectorAll(".change-stats")).toHaveLength(1);
    const diff = container.querySelector('[data-testid="diff"]');
    expect(diff?.getAttribute("data-file")).toBe("src/file-7.ts");
  });
});
