import { describe, expect, it, vi } from "vitest";
import type { editor } from "monaco-editor";
import { registerEditor, requestReveal, unregisterEditor } from "./reveal";

function createEditor(maxColumn = 16): editor.IStandaloneCodeEditor {
  return {
    getModel: () => ({
      getLineCount: () => 8,
      getLineMaxColumn: () => maxColumn
    }),
    revealPositionInCenter: vi.fn(),
    setPosition: vi.fn(),
    focus: vi.fn()
  } as unknown as editor.IStandaloneCodeEditor;
}

describe("editor diagnostic navigation", () => {
  it("reveals the exact reported line and column, clamped to the model", () => {
    const target = createEditor(12);
    registerEditor("styles.css", target);
    requestReveal("styles.css", 4, 30);

    expect(target.revealPositionInCenter).toHaveBeenCalledWith({ lineNumber: 4, column: 12 });
    expect(target.setPosition).toHaveBeenCalledWith({ lineNumber: 4, column: 12 });
    expect(target.focus).toHaveBeenCalledOnce();
    unregisterEditor("styles.css");
  });

  it("waits for a newly opened file's editor before applying the pending diagnostic location", () => {
    const target = createEditor();
    requestReveal("index.html", 6, 9);
    registerEditor("index.html", target);

    expect(target.revealPositionInCenter).toHaveBeenCalledWith({ lineNumber: 6, column: 9 });
    expect(target.setPosition).toHaveBeenCalledWith({ lineNumber: 6, column: 9 });
    unregisterEditor("index.html");
  });
});
