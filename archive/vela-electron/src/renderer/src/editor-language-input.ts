import type { editor } from "monaco-editor";

function isAttributeAssignment(prefix: string): boolean {
  let tagStart = -1;
  let quote: "'" | '"' | null = null;

  for (let index = 0; index < prefix.length; index += 1) {
    const character = prefix[index];
    if (tagStart === -1) {
      if (character === "<") tagStart = index;
      continue;
    }

    if (quote) {
      if (character === quote) quote = null;
      continue;
    }

    if (character === "'" || character === '"') {
      quote = character;
    } else if (character === ">") {
      tagStart = -1;
    }
  }

  if (tagStart === -1 || quote) return false;
  const tag = prefix.slice(tagStart + 1);
  if (/^\s*[!?/]/.test(tag)) return false;
  return /(?:^|\s)[^\s=<>/]+\s*=\s*$/.test(tag);
}

export function wireHtmlAttributeQuotes(target: editor.IStandaloneCodeEditor): { dispose: () => void } {
  return target.onDidChangeModelContent((event) => {
    if (!event.changes.some((change) => change.text === "=")) return;
    queueMicrotask(() => {
      const model = target.getModel();
      const position = target.getPosition();
      if (!model || model.getLanguageId() !== "html" || !position) return;

      const prefix = model.getValueInRange({
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column
      });
      const normalizedPrefix = prefix.toLowerCase();
      if (
        normalizedPrefix.lastIndexOf("<script") > normalizedPrefix.lastIndexOf("</script") ||
        normalizedPrefix.lastIndexOf("<style") > normalizedPrefix.lastIndexOf("</style")
      ) return;
      if (!isAttributeAssignment(prefix)) return;

      const nextCharacter = model.getValueInRange({
        startLineNumber: position.lineNumber,
        startColumn: position.column,
        endLineNumber: position.lineNumber,
        endColumn: Math.min(position.column + 1, model.getLineMaxColumn(position.lineNumber))
      });
      if (nextCharacter === '"' || nextCharacter === "'") return;

      target.executeEdits("vela.html-auto-close-attribute", [{
        range: {
          startLineNumber: position.lineNumber,
          startColumn: position.column,
          endLineNumber: position.lineNumber,
          endColumn: position.column
        },
        text: '""'
      }]);
      target.setPosition({ lineNumber: position.lineNumber, column: position.column + 1 });
    });
  });
}
