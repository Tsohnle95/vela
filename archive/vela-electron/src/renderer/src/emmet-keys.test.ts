import type { editor } from "monaco-editor";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { wireEmmetKeys } from "./emmet-keys";

const mocks = vi.hoisted(() => ({ insert: vi.fn() }));

vi.mock("monaco-editor", () => ({
  KeyCode: { Tab: 2, Enter: 3, KEY_IN_COMPOSITION: 109 },
  Range: class {
    constructor(..._args: number[]) {}
  },
  editor: { EditorOption: { readOnly: 1 } }
}));
vi.mock("monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetController2.js", () => ({
  SnippetController2: { get: () => ({ isInSnippet: () => false, insert: mocks.insert }) }
}));
vi.mock("./emmet", () => ({
  emmetSnippetAt: () => ({ abbr: "div", startColumn: 1, snippet: "<div>${0}</div>" })
}));

function fixture(suggestionVisible: boolean): {
  dispatch: (keyCode: number) => { preventDefault: ReturnType<typeof vi.fn>; stopPropagation: ReturnType<typeof vi.fn> };
} {
  const node = document.createElement("div");
  if (suggestionVisible) node.innerHTML = '<div class="suggest-widget visible"></div>';
  let onKeyDown: (event: unknown) => void = () => {};
  const target = {
    onKeyDown: (listener: (event: unknown) => void) => { onKeyDown = listener; },
    getDomNode: () => node,
    getSelections: () => [{}],
    getPosition: () => ({ lineNumber: 1, column: 4 }),
    getModel: () => ({ getLineContent: () => "div", getLanguageId: () => "html", getValueInRange: () => "" }),
    getOption: () => false,
    setSelection: vi.fn()
  } as unknown as editor.IStandaloneCodeEditor;
  wireEmmetKeys(target);
  return {
    dispatch: (keyCode) => {
      const event = { keyCode, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false,
        preventDefault: vi.fn(), stopPropagation: vi.fn() };
      onKeyDown(event);
      return event;
    }
  };
}

describe("Emmet and Monaco suggestions", () => {
  beforeEach(() => {
    mocks.insert.mockClear();
    document.getElementById("vela-monaco-overflow")?.remove();
  });

  it.each([2, 3])("leaves key %i to Monaco while suggestions are visible", (keyCode) => {
    const { dispatch } = fixture(true);
    const event = dispatch(keyCode);
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(event.stopPropagation).not.toHaveBeenCalled();
    expect(mocks.insert).not.toHaveBeenCalled();
  });

  it("leaves Tab to Monaco when the list is in the overflow host", () => {
    const { dispatch } = fixture(false);
    const host = document.createElement("div");
    host.id = "vela-monaco-overflow";
    host.innerHTML = '<div class="suggest-widget visible"></div>';
    document.body.append(host);
    const event = dispatch(2);
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(mocks.insert).not.toHaveBeenCalled();
    host.remove();
  });

  it.each([16, 18])("leaves arrow key %i to Monaco", (keyCode) => {
    const event = fixture(true).dispatch(keyCode);
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(mocks.insert).not.toHaveBeenCalled();
  });

  it("still expands an abbreviation when the suggestion list is closed", () => {
    const event = fixture(false).dispatch(2);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(mocks.insert).toHaveBeenCalledWith("<div>${0}</div>");
  });
});
