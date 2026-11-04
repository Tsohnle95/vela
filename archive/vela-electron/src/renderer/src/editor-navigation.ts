import type { editor } from "monaco-editor";

export function wireEditorNavigationKeys(target: editor.IStandaloneCodeEditor): { dispose: () => void } {
  const node = target.getDomNode();
  if (!node) return { dispose: () => {} };

  const macOS = /Mac|iPhone|iPad/i.test(navigator.platform);
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.defaultPrevented || event.isComposing || event.shiftKey || event.altKey) return;
    const commandPressed = macOS
      ? event.metaKey && !event.ctrlKey
      : event.ctrlKey && !event.metaKey;
    if (!commandPressed) return;

    const model = target.getModel();
    const position = target.getPosition();
    if (!model || !position) return;

    let destination: { lineNumber: number; column: number };
    if (event.key === "ArrowUp") {
      destination = { lineNumber: 1, column: 1 };
    } else if (event.key === "ArrowDown") {
      const lineNumber = model.getLineCount();
      destination = { lineNumber, column: model.getLineMaxColumn(lineNumber) };
    } else if (event.key === "ArrowLeft") {
      destination = { lineNumber: position.lineNumber, column: 1 };
    } else if (event.key === "ArrowRight") {
      destination = {
        lineNumber: position.lineNumber,
        column: model.getLineMaxColumn(position.lineNumber)
      };
    } else {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
    target.setPosition(destination);
    target.revealPositionInCenterIfOutsideViewport(destination);
  };

  node.addEventListener("keydown", onKeyDown, true);
  return { dispose: () => node.removeEventListener("keydown", onKeyDown, true) };
}
