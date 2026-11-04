import { useCallback, useEffect, useRef, useState } from "react";
import type { editor } from "monaco-editor";
import type { EditorStatus } from "./components/editor-status";

// Publishes the focused editor's cursor/indentation status and re-publishes it
// whenever focus moves between editor groups.
export function useStatusReporter(
  path: string,
  focused: boolean,
  onStatusChange: (status: EditorStatus) => void
): (ed: editor.IStandaloneCodeEditor) => void {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const listenerRef = useRef<{ dispose: () => void } | null>(null);
  const focusedRef = useRef(focused);
  focusedRef.current = focused;

  const publish = useCallback((ed: editor.IStandaloneCodeEditor | null): void => {
    if (!focusedRef.current || !ed) return;
    const position = ed.getPosition();
    const model = ed.getModel();
    if (!position || !model) return;
    const modelOptions = model.getOptions();
    onStatusChange({
      path,
      lineNumber: position.lineNumber,
      column: position.column,
      tabSize: modelOptions.tabSize,
      insertSpaces: modelOptions.insertSpaces
    });
  }, [onStatusChange, path]);

  useEffect(() => {
    publish(editorRef.current);
  }, [focused, publish]);

  useEffect(() => () => {
    listenerRef.current?.dispose();
    listenerRef.current = null;
  }, []);

  return useCallback((ed: editor.IStandaloneCodeEditor): void => {
    editorRef.current = ed;
    listenerRef.current?.dispose();
    listenerRef.current = ed.onDidChangeCursorPosition(() => publish(ed));
    publish(ed);
  }, [publish]);
}

// The overlay scrollbar is only revealed while the editor is being scrolled;
// `scrolling` clears itself shortly after the last scroll event.
export function useScrollReveal(): { scrolling: boolean; watch: (ed: editor.IStandaloneCodeEditor) => void } {
  const listenerRef = useRef<{ dispose: () => void } | null>(null);
  const idleRef = useRef<number | null>(null);
  const [scrolling, setScrolling] = useState(false);

  useEffect(() => () => {
    listenerRef.current?.dispose();
    listenerRef.current = null;
    if (idleRef.current !== null) window.clearTimeout(idleRef.current);
  }, []);

  const watch = useCallback((ed: editor.IStandaloneCodeEditor): void => {
    listenerRef.current?.dispose();
    listenerRef.current = ed.onDidScrollChange(() => {
      setScrolling(true);
      if (idleRef.current !== null) window.clearTimeout(idleRef.current);
      idleRef.current = window.setTimeout(() => {
        idleRef.current = null;
        setScrolling(false);
      }, 900);
    });
  }, []);

  return { scrolling, watch };
}
