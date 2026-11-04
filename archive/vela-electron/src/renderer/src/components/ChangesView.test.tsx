import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentFileState } from "@shared/types";
import { ChangesView } from "./ChangesView";

const mocks = vi.hoisted(() => ({
  previewFile: vi.fn(),
  dismissChange: vi.fn(),
  dismissChanges: vi.fn(),
  files: new Map<string, AgentFileState>(),
  hidden: new Set<string>()
}));

vi.mock("../store", () => ({
  useStore: () => ({
    session: { id: "session", directory: "/workspace", workspace: { id: "ws", generation: 1 } },
    agentFiles: mocks.files,
    hiddenPaths: mocks.hidden,
    previewFile: mocks.previewFile,
    dismissChange: mocks.dismissChange,
    dismissChanges: mocks.dismissChanges
  })
}));

vi.mock("./OpenCodeFile", () => ({
  OpenCodeFile: ({ file }: { file: string }) => <div data-testid="diff" data-file={file} />
}));

let container: HTMLDivElement;
let root: Root;
const onClose = vi.fn();

function render(): void {
  act(() => root.render(<ChangesView onClose={onClose} />));
}

function seed(): void {
  mocks.files.set("src/a.ts", { baseline: { kind: "known", content: "old\n" }, content: "new\n", deleted: false });
  mocks.files.set("gone.ts", { baseline: { kind: "known", content: "old\n" }, content: null, deleted: true });
  mocks.files.set("unknown.ts", { baseline: { kind: "unknown" }, content: "new\n", deleted: false });
}

describe("ChangesView", () => {
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    mocks.files.clear();
    mocks.hidden.clear();
    vi.clearAllMocks();
    Object.defineProperty(Element.prototype, "scrollIntoView", { configurable: true, value: vi.fn() });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("shows an empty state without changes", () => {
    render();
    expect(container.querySelector(".changes-empty")).not.toBeNull();
    expect(container.querySelector('[data-testid="diff"]')).toBeNull();
  });

  it("renders the file tree and a diff for each known change", () => {
    seed();
    render();

    expect(container.querySelectorAll(".changes-tree-row.file")).toHaveLength(3);
    const diffs = [...container.querySelectorAll('[data-testid="diff"]')].map((node) => node.getAttribute("data-file"));
    expect(diffs).toEqual(["gone.ts", "src/a.ts"]);
    expect(container.querySelectorAll(".change-section-status").length).toBe(3);
  });

  it("marks files without a known baseline as unavailable instead of diffing", () => {
    seed();
    render();

    const unknown = [...container.querySelectorAll(".change-section")]
      .find((section) => section.getAttribute("data-change-path") === "unknown.ts")!;
    expect(unknown.querySelector(".change-section-unavailable")?.textContent).toContain("isn't available");
    expect(unknown.querySelector('[data-testid="diff"]')).toBeNull();
    expect(unknown.querySelector(".change-section-status")?.textContent).toBe("Observed");
  });

  it("clears a single change and every change", () => {
    seed();
    render();

    act(() => container.querySelector<HTMLButtonElement>('[aria-label="Clear gone.ts"]')!.click());
    expect(mocks.dismissChange).toHaveBeenCalledWith("gone.ts");

    act(() => [...container.querySelectorAll<HTMLButtonElement>(".changes-action")]
      .find((button) => button.textContent === "Clear all")!.click());
    expect(mocks.dismissChanges).toHaveBeenCalledTimes(1);
  });

  it("opens a file in the editor and closes the view", () => {
    seed();
    render();

    const name = [...container.querySelectorAll<HTMLElement>(".change-section-name")]
      .find((node) => node.textContent === "a.ts")!;
    act(() => name.click());

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(mocks.previewFile).toHaveBeenCalledWith("src/a.ts");
  });

  it("omits changes hidden for the workspace", () => {
    seed();
    mocks.hidden.add("src");
    render();

    const paths = [...container.querySelectorAll(".change-section")].map((section) => section.getAttribute("data-change-path"));
    expect(paths).toEqual(["gone.ts", "unknown.ts"]);
  });

  it("collapses a section and collapses all from the toolbar", () => {
    seed();
    render();

    act(() => container.querySelector<HTMLButtonElement>(".change-section-toggle")!.click());
    expect(container.querySelectorAll('[data-testid="diff"]')).toHaveLength(1);

    act(() => [...container.querySelectorAll<HTMLButtonElement>(".changes-action")]
      .find((button) => button.textContent === "Collapse all")!.click());
    expect(container.querySelectorAll('[data-testid="diff"]')).toHaveLength(0);
  });
});
