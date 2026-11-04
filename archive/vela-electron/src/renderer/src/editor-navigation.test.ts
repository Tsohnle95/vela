import type { editor } from "monaco-editor";
import { describe, expect, it, vi } from "vitest";
import { wireEditorNavigationKeys } from "./editor-navigation";

describe("wireEditorNavigationKeys", () => {
  it("routes command/control arrows to the editor that received the key event", () => {
    const node = document.createElement("div");
    const position = { lineNumber: 2, column: 4 };
    const setPosition = vi.fn();
    const revealPositionInCenterIfOutsideViewport = vi.fn();
    const target = {
      getDomNode: () => node,
      getModel: () => ({
        getLineCount: () => 3,
        getLineMaxColumn: (lineNumber: number) => [0, 6, 8, 5][lineNumber]
      }),
      getPosition: () => position,
      setPosition,
      revealPositionInCenterIfOutsideViewport
    } as unknown as editor.IStandaloneCodeEditor;

    const listener = wireEditorNavigationKeys(target);
    const macOS = /Mac|iPhone|iPad/i.test(navigator.platform);
    const modifier = macOS ? { metaKey: true } : { ctrlKey: true };
    for (const key of ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"]) {
      node.dispatchEvent(new KeyboardEvent("keydown", { key, ...modifier, bubbles: true, cancelable: true }));
    }

    expect(setPosition.mock.calls.map(([next]) => next)).toEqual([
      { lineNumber: 1, column: 1 },
      { lineNumber: 3, column: 5 },
      { lineNumber: 2, column: 1 },
      { lineNumber: 2, column: 8 }
    ]);
    expect(revealPositionInCenterIfOutsideViewport).toHaveBeenCalledTimes(4);

    listener.dispose();
    expect(node.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", ...modifier, bubbles: true, cancelable: true }))).toBe(true);
    expect(setPosition).toHaveBeenCalledTimes(4);
  });
});
