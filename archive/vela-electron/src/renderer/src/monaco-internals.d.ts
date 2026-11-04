declare module "monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetController2.js" {
  import type { editor } from "monaco-editor";

  export class SnippetController2 {
    static ID: string;
    static get(editor: editor.ICodeEditor): SnippetController2 | null;
    insert(template: string, opts?: {}): void;
    isInSnippet(): boolean;
    cancel(): void;
  }
}