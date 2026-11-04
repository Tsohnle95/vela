import type { editor } from "monaco-editor";
import type { W3cDiagnostic } from "@shared/types";
import { monaco } from "./monaco";
import { getEditor } from "./reveal";

const MARKER_OWNER = "w3c";

function toMarker(diagnostic: W3cDiagnostic): editor.IMarkerData {
  return {
    startLineNumber: diagnostic.line,
    startColumn: diagnostic.column,
    endLineNumber: diagnostic.endLine,
    endColumn: diagnostic.endColumn,
    message: diagnostic.message,
    severity: diagnostic.severity === "warning" ? 4 : 8,
    source: diagnostic.source
  };
}

function modelFor(path: string): editor.ITextModel | null {
  const model = getEditor(path)?.getModel();
  return model && !model.isDisposed() ? model : null;
}

export function applyW3cMarkers(path: string, diagnostics: W3cDiagnostic[]): void {
  const model = modelFor(path);
  if (!model) return;
  monaco.editor.setModelMarkers(model, MARKER_OWNER, diagnostics.map(toMarker));
}

export function clearW3cMarkers(path: string): void {
  const model = modelFor(path);
  if (!model) return;
  monaco.editor.setModelMarkers(model, MARKER_OWNER, []);
}