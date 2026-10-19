import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Tab } from "@shared/types";
import { EditorStatusMenu } from "./EditorStatusMenu";
import { applyW3cMarkers, clearW3cMarkers } from "../w3c-validation";
import type { EditorStatus } from "./editor-status";

const htmlTab: Tab = {
  path: "index.html",
  name: "index.html",
  content: "<p>hi",
  saved: "<p>hi",
  baseline: null,
  deleted: false,
  dirty: false,
  stale: false,
  revision: 0,
  conflict: null,
  binary: false
};
const cssTab: Tab = { ...htmlTab, path: "styles.css", name: "styles.css", content: "body { color: red; }" };
const scssTab: Tab = { ...htmlTab, path: "docs/css/layout/_chat.scss", name: "_chat.scss", content: "$color: red; .chat { color: $color; }" };
const tsTab: Tab = { ...htmlTab, path: "app.ts", name: "app.ts", content: "const x = 1;" };
const scssState = vi.hoisted(() => ({
  getEditor: vi.fn(),
  updateOptions: vi.fn(),
  markers: [] as Array<Record<string, unknown>>
}));

const store: { tabs: Tab[]; activePath: string | null } = {
  tabs: [htmlTab],
  activePath: htmlTab.path
};

vi.mock("../store", () => ({ useStore: () => store }));
vi.mock("../w3c-validation", () => ({
  applyW3cMarkers: vi.fn(),
  clearW3cMarkers: vi.fn()
}));
vi.mock("../reveal", () => ({ getEditor: scssState.getEditor }));
vi.mock("../monaco", () => ({
  monaco: {
    MarkerSeverity: { Warning: 4, Error: 8 },
    editor: { getModelMarkers: vi.fn(() => scssState.markers) }
  }
}));

describe("editor status menu validation", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    store.tabs = [htmlTab];
    store.activePath = htmlTab.path;
    scssState.getEditor.mockReset();
    scssState.updateOptions.mockReset();
    scssState.markers = [];
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  const renderMenu = async (node: ReactNode = <EditorStatusMenu />): Promise<void> => {
    await act(async () => root.render(node));
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".status-menu-button")!.click();
    });
  };


  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("validates the local split-group tab rather than the globally focused file", async () => {
    const validate = vi.fn(async () => []);
    vi.stubGlobal("vela", { validateW3c: validate });
    store.tabs = [tsTab, htmlTab];
    store.activePath = tsTab.path;
    await renderMenu(<EditorStatusMenu tab={htmlTab} editorStatus={{ path: tsTab.path, lineNumber: 9, column: 7, tabSize: 4, insertSpaces: false }} />);
    expect(container.querySelector(".status-menu-path")?.textContent).toBe(htmlTab.path);
    expect(container.querySelector('[data-testid="cursor-position"]')?.textContent).toContain("Ln 1");
    await act(async () => container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!.click());
    expect(validate).toHaveBeenCalledWith(htmlTab.path, htmlTab.content);
  });

  it("keeps the validation control visible and explains when the active file is unsupported", async () => {
    store.tabs = [];
    store.activePath = null;
    await renderMenu();
    const noFile = container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!;
    expect(noFile).not.toBeNull();
    expect(noFile.disabled).toBe(true);
    expect(noFile.title).toContain("Open an HTML, CSS, or SCSS file");

    store.tabs = [tsTab];
    store.activePath = tsTab.path;
    await act(async () => root.render(<EditorStatusMenu />));
    expect(container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')?.disabled).toBe(true);
  });

  it("runs the W3C validator for an open HTML file and shows the marker counts", async () => {
    const diagnostics = [
      { line: 1, column: 1, endLine: 1, endColumn: 2, message: "bad", severity: "error", source: "w3c-html" },
      { line: 2, column: 1, endLine: 2, endColumn: 2, message: "warn", severity: "warning", source: "w3c-html" },
      { line: 3, column: 1, endLine: 3, endColumn: 2, message: "also bad", severity: "error", source: "w3c-html" }
    ];
    window.vela = { validateW3c: vi.fn(async () => diagnostics) } as unknown as typeof window.vela;

    await renderMenu();
    const button = container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!;
    expect(button.disabled).toBe(false);
    expect(button.closest(".status-menu-popover")).not.toBeNull();

    await act(async () => {
      button.click();
      await Promise.resolve();
    });

    expect(window.vela.validateW3c).toHaveBeenCalledWith("index.html", "<p>hi");
    expect(applyW3cMarkers).toHaveBeenCalledWith("index.html", diagnostics);
    expect(container.querySelector('[data-testid="validate-result"]')?.textContent).toBe("2 errors, 1 warning");
  });

  it("runs the W3C validator for an open CSS file", async () => {
    const diagnostics = [
      { line: 4, column: 9, endLine: 4, endColumn: 10, message: "Unknown property", severity: "error" as const, source: "w3c-css" as const }
    ];
    window.vela = { validateW3c: vi.fn(async () => diagnostics) } as unknown as typeof window.vela;
    store.tabs = [cssTab];
    store.activePath = cssTab.path;

    await renderMenu();
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!.click();
      await Promise.resolve();
    });

    expect(window.vela.validateW3c).toHaveBeenCalledWith(cssTab.path, cssTab.content);
    expect(applyW3cMarkers).toHaveBeenCalledWith(cssTab.path, diagnostics);
    expect(container.querySelector('[data-testid="validate-result"]')?.textContent).toBe("1 error");
  });

  it("validates an open SCSS file with Monaco's Sass diagnostics", async () => {
    const localModel = {
      uri: { toString: () => "file:///workspace/docs/css/layout/_chat.scss" },
      isDisposed: () => false,
      getLanguageId: () => "scss"
    };
    const diagnostics = [
      { line: 6, column: 8, endLine: 6, endColumn: 13, message: "Expected expression.", severity: "error" as const, source: "monaco-scss" as const }
    ];
    scssState.getEditor.mockReturnValue({ getModel: () => localModel, updateOptions: scssState.updateOptions });
    scssState.markers = [{
      owner: "scss",
      severity: 8,
      startLineNumber: 6,
      startColumn: 8,
      endLineNumber: 6,
      endColumn: 13,
      message: "Expected expression."
    }];
    store.tabs = [scssTab];
    store.activePath = scssTab.path;

    await renderMenu();
    const button = container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!;
    expect(button.disabled).toBe(false);
    await act(async () => {
      button.click();
      await Promise.resolve();
    });

    expect(scssState.getEditor).toHaveBeenCalledWith(scssTab.path);
    expect(applyW3cMarkers).not.toHaveBeenCalled();
    expect(container.querySelector('[data-testid="validate-result"]')?.textContent).toBe("1 error");
    expect(button.textContent).toContain("Hide errors");
    act(() => button.click());
    expect(scssState.updateOptions).toHaveBeenCalledWith({ renderValidationDecorations: "off" });
    expect(button.textContent).toContain("Show errors");
    act(() => button.click());
    expect(scssState.updateOptions).toHaveBeenCalledWith({ renderValidationDecorations: "on" });
  });

  it("opens the detailed report and toggles validation squiggles from the status control", async () => {
    const diagnostics = [
      { line: 2, column: 4, endLine: 2, endColumn: 8, message: "Unexpected end tag", severity: "error" as const, source: "w3c-html" as const }
    ];
    const onValidationComplete = vi.fn();
    window.vela = { validateW3c: vi.fn(async () => diagnostics) } as unknown as typeof window.vela;
    await renderMenu(<EditorStatusMenu onValidationComplete={onValidationComplete} />);

    const button = container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!;
    await act(async () => {
      button.click();
      await Promise.resolve();
    });
    expect(onValidationComplete).toHaveBeenCalledWith({ path: "index.html", diagnostics });
    expect(button.textContent).toContain("Hide errors");

    act(() => button.click());
    expect(clearW3cMarkers).toHaveBeenCalledWith("index.html");
    expect(button.textContent).toContain("Show errors");

    act(() => button.click());
    expect(applyW3cMarkers).toHaveBeenLastCalledWith("index.html", diagnostics);
    expect(button.textContent).toContain("Hide errors");
  });

  it("reports a clean file and resets the result when the content changes", async () => {
    window.vela = { validateW3c: vi.fn(async () => []) } as unknown as typeof window.vela;

    await renderMenu();
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!.click();
      await Promise.resolve();
    });
    expect(container.querySelector('[data-testid="validate-result"]')?.textContent).toBe("No problems");

    store.tabs = [{ ...htmlTab, content: "<p>changed" }];
    await renderMenu();
    expect(container.querySelector('[data-testid="validate-result"]')).toBeNull();
  });

  it("shows a failure state when the validator rejects", async () => {
    window.vela = { validateW3c: vi.fn(async () => { throw new Error("network down"); }) } as unknown as typeof window.vela;

    await renderMenu();
    await act(async () => {
      container.querySelector<HTMLButtonElement>('[data-testid="validate-btn"]')!.click();
      await Promise.resolve();
    });

    expect(container.querySelector('[data-testid="validate-result"]')?.textContent).toBe("Validation failed");
  });

  it("shows cursor, indentation, and encoding details for the active editor", async () => {
    const status: EditorStatus = {
      path: htmlTab.path,
      lineNumber: 123,
      column: 61,
      tabSize: 4,
      insertSpaces: true
    };
    await renderMenu(<EditorStatusMenu editorStatus={status} />);

    expect(container.querySelector('[data-testid="cursor-position"]')?.textContent).toBe("Ln 123, Col 61");
    expect(container.querySelector('[data-testid="indentation-status"]')?.textContent).toBe("Spaces: 4");
    expect(container.querySelector('[data-testid="encoding-status"]')?.textContent).toBe("UTF-8");
    expect(container.querySelector('[data-testid="cursor-position"]')?.closest(".status-menu-meta")).not.toBeNull();
  });

  it("does not display cursor details from another file", async () => {
    const status: EditorStatus = {
      path: "other.ts",
      lineNumber: 99,
      column: 40,
      tabSize: 8,
      insertSpaces: false
    };
    await renderMenu(<EditorStatusMenu editorStatus={status} />);

    expect(container.querySelector('[data-testid="cursor-position"]')?.textContent).toBe("Ln 1, Col 1");
    expect(container.querySelector('[data-testid="indentation-status"]')?.textContent).toBe("Spaces: 2");
  });
});
