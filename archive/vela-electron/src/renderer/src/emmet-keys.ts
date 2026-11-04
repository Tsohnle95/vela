import * as monaco from "monaco-editor";
import type { editor } from "monaco-editor";
import { SnippetController2 } from "monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetController2.js";
import { emmetSnippetAt } from "./emmet";

function insideScriptTag(model: editor.ITextModel, position: monaco.Position): boolean {
  const before = model.getValueInRange(new monaco.Range(1, 1, position.lineNumber, position.column));
  const open = before.lastIndexOf("<script");
  if (open < 0) return false;
  return before.lastIndexOf("</script>") < open;
}

export function wireEmmetKeys(target: editor.IStandaloneCodeEditor): void {
  target.onKeyDown((e) => {
    if (e.keyCode === monaco.KeyCode.KEY_IN_COMPOSITION) return;
    if (e.keyCode !== monaco.KeyCode.Tab && e.keyCode !== monaco.KeyCode.Enter) return;
    if (e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return;
    if (
      target.getDomNode()?.querySelector(".suggest-widget.visible") ||
      document.getElementById("vela-monaco-overflow")?.querySelector(".suggest-widget.visible")
    ) return;
    if ((target.getSelections() ?? []).length !== 1) return;
    const snippetController = SnippetController2.get(target);
    if (!snippetController || snippetController.isInSnippet()) return;
    const position = target.getPosition();
    const model = target.getModel();
    if (!position || !model || target.getOption(monaco.editor.EditorOption.readOnly)) return;
    const beforeCaret = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
    const found = emmetSnippetAt(beforeCaret, model.getLanguageId());
    if (!found) return;
    if (model.getLanguageId() === "html" && insideScriptTag(model, position)) return;
    e.preventDefault();
    e.stopPropagation();
    target.setSelection(
      new monaco.Range(position.lineNumber, found.startColumn, position.lineNumber, position.column)
    );
    snippetController.insert(found.snippet);
  });
}
