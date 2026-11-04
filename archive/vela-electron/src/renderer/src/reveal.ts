import type { editor } from "monaco-editor";

type RevealTarget = { path: string; line: number; column: number };

const editors = new Map<string, editor.IStandaloneCodeEditor>();
let pending: RevealTarget | null = null;

function reveal(ed: editor.IStandaloneCodeEditor, line: number, column: number): void {
  const model = ed.getModel();
  const lineCount = model?.getLineCount() ?? Math.max(1, line);
  const lineNumber = Math.max(1, Math.min(lineCount, Math.floor(line)));
  const maxColumn = model?.getLineMaxColumn(lineNumber) ?? Math.max(1, column);
  const position = { lineNumber, column: Math.max(1, Math.min(maxColumn, Math.floor(column))) };
  ed.revealPositionInCenter(position);
  ed.setPosition(position);
  ed.focus();
}

export function registerEditor(path: string, ed: editor.IStandaloneCodeEditor): void {
  editors.set(path, ed);
  if (pending && pending.path === path) {
    const { line, column } = pending;
    pending = null;
    reveal(ed, line, column);
  }
}

export function unregisterEditor(path: string): void {
  editors.delete(path);
}

export function getEditor(path: string): editor.IStandaloneCodeEditor | null {
  return editors.get(path) ?? null;
}

export function requestReveal(path: string, line: number, column = 1): void {
  const ed = editors.get(path);
  if (ed) {
    reveal(ed, line, column);
  } else {
    pending = { path, line, column };
  }
}
