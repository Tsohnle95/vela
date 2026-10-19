import * as monaco from "monaco-editor";
import { loader } from "@monaco-editor/react";
import { emmetCSS, emmetHTML } from "emmet-monaco-es";
import { APPEARANCES } from "./appearances";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

declare global {
  interface Window {
    MonacoEnvironment?: {
      getWorker?: (moduleId: string, label: string) => Worker;
    };
  }
}

window.MonacoEnvironment = {
  getWorker(_moduleId: string, label: string): Worker {
    if (label === "json") return new jsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new cssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new htmlWorker();
    if (label === "typescript" || label === "javascript") return new tsWorker();
    return new editorWorker();
  }
};

loader.config({ monaco });

const javascriptDefaults = monaco.languages.typescript.javascriptDefaults;
javascriptDefaults.setEagerModelSync(true);
javascriptDefaults.setCompilerOptions({
  // Monaco's replacement settings must retain this default for editor model URIs.
  allowNonTsExtensions: true,
  allowJs: true,
  target: monaco.languages.typescript.ScriptTarget.ES2020,
  lib: ["es2020", "dom", "dom.iterable"]
});
javascriptDefaults.setDiagnosticsOptions({
  noSemanticValidation: false,
  noSyntaxValidation: false,
  noSuggestionDiagnostics: false
});

emmetHTML(monaco, ["html"]);
emmetCSS(monaco, ["css", "scss", "less"]);

const PHP_KEYWORDS = [
  "abstract", "array", "as", "break", "case", "catch", "class", "clone", "const", "continue",
  "declare", "default", "do", "echo", "else", "elseif", "enum", "extends", "false", "final",
  "finally", "fn", "for", "foreach", "function", "if", "implements", "include", "include_once",
  "instanceof", "interface", "match", "namespace", "new", "null", "private", "protected",
  "public", "readonly", "require", "require_once", "return", "static", "switch", "throw",
  "trait", "true", "try", "use", "while", "yield"
];
const PHP_FUNCTIONS = [
  "array_filter", "array_keys", "array_map", "array_merge", "array_pop", "array_push",
  "array_reduce", "array_search", "array_values", "count", "date", "empty", "explode",
  "file_exists", "file_get_contents", "file_put_contents", "filter_var", "htmlspecialchars",
  "implode", "in_array", "is_array", "is_null", "isset", "json_decode", "json_encode",
  "preg_match", "preg_replace", "print_r", "sprintf", "str_contains", "str_replace",
  "strlen", "strpos", "strtolower", "strtoupper", "substr", "trim", "unset", "var_dump"
];

monaco.languages.registerCompletionItemProvider("php", {
  provideCompletionItems(model, position) {
    const range = model.getWordUntilPosition(position);
    const replacement = new monaco.Range(position.lineNumber, range.startColumn, position.lineNumber, range.endColumn);
    return {
      suggestions: [
        ...PHP_KEYWORDS.map((label) => ({ label, kind: monaco.languages.CompletionItemKind.Keyword, insertText: label, range: replacement })),
        ...PHP_FUNCTIONS.map((label) => ({ label, kind: monaco.languages.CompletionItemKind.Function, insertText: label, range: replacement }))
      ]
    };
  }
});

for (const { id, scheme, colors } of APPEARANCES) {
  const legacyColors = id === "kitty" ? {
    "editor.lineHighlightBackground": "#343a5526",
    "editorLineNumber.foreground": "#626b78",
    "editor.selectionBackground": "#2e4d78",
    "diffEditor.insertedTextBackground": "#5bd69a20",
    "diffEditor.removedTextBackground": "#ff4b6720",
    "diffEditor.insertedLineBackground": "#5bd69a14",
    "diffEditor.removedLineBackground": "#ff4b6714",
    "scrollbarSlider.background": "#e7e7ee17",
    "scrollbarSlider.hoverBackground": "#e7e7ee2b"
  } : id === "original" ? {
    "editor.lineHighlightBackground": "#2d2926",
    "editorLineNumber.foreground": "#57534e",
    "editor.selectionBackground": "#4a352c",
    "diffEditor.insertedTextBackground": "#9dc2a11f",
    "diffEditor.removedTextBackground": "#e2988a1f",
    "diffEditor.insertedLineBackground": "#9dc2a117",
    "diffEditor.removedLineBackground": "#e2988a17",
    "scrollbarSlider.hoverBackground": "#ffffff26"
  } : {};
  const variable = colors.syntaxVariable ?? colors.syntaxFunction;
  const attribute = colors.syntaxAttribute ?? colors.syntaxNumber;
  const type = colors.syntaxType ?? variable;
  const constant = colors.syntaxConstant ?? colors.syntaxNumber;
  const operator = colors.syntaxOperator ?? colors.syntaxKeyword;
  const punctuation = colors.syntaxPunctuation ?? operator;
  const tag = colors.syntaxTag ?? colors.syntaxFunction;
  const decorator = colors.syntaxDecorator ?? colors.syntaxFunction;
  const namespace = colors.syntaxNamespace ?? colors.syntaxNumber;
  const regexp = colors.syntaxRegexp ?? colors.syntaxString;
  monaco.editor.defineTheme(`vela-${id}`, {
    base: scheme === "light" ? "vs" : "vs-dark",
    inherit: true,
    rules: [
      // Token colors mirror the reference editor's Pierre theme; comments are
      // plain (not italic) there.
      { token: "comment", foreground: colors.syntaxComment.slice(1) },
      { token: "keyword", foreground: colors.syntaxKeyword.slice(1) },
      // JSON literals (true/false/null) use the reference's cyan literal color.
      { token: "keyword.json", foreground: operator.slice(1) },
      { token: "storage", foreground: colors.syntaxKeyword.slice(1) },
      { token: "operator", foreground: operator.slice(1) },
      { token: "delimiter", foreground: punctuation.slice(1) },
      { token: "punctuation", foreground: punctuation.slice(1) },
      { token: "string", foreground: colors.syntaxString.slice(1) },
      { token: "number", foreground: colors.syntaxNumber.slice(1) },
      { token: "constant", foreground: constant.slice(1) },
      { token: "predefined", foreground: constant.slice(1) },
      { token: "type", foreground: type.slice(1) },
      { token: "type.identifier", foreground: type.slice(1) },
      { token: "variable", foreground: variable.slice(1) },
      { token: "identifier", foreground: variable.slice(1) },
      { token: "function", foreground: colors.syntaxFunction.slice(1) },
      { token: "namespace", foreground: namespace.slice(1) },
      { token: "regexp", foreground: regexp.slice(1) },
      { token: "annotation", foreground: decorator.slice(1) },
      { token: "attribute", foreground: attribute.slice(1) },
      { token: "attribute.name", foreground: attribute.slice(1) },
      { token: "string.key", foreground: tag.slice(1) },
      { token: "string.value", foreground: colors.syntaxString.slice(1) },
      { token: "tag", foreground: tag.slice(1) },
      { token: "meta", foreground: colors.syntaxFunction.slice(1) },
      // The built-in base themes define deeper token nodes (e.g.
      // `string.key.json`) that would otherwise shadow the rules above; mirror
      // them so the reference palette also wins for JSON, YAML, HTML/XML, and
      // SCSS tokens.
      { token: "string.key.json", foreground: tag.slice(1) },
      { token: "string.value.json", foreground: colors.syntaxString.slice(1) },
      { token: "string.yaml", foreground: colors.syntaxString.slice(1) },
      { token: "string.html", foreground: colors.syntaxString.slice(1) },
      { token: "string.sql", foreground: colors.syntaxString.slice(1) },
      { token: "key", foreground: variable.slice(1) },
      { token: "number.hex", foreground: colors.syntaxNumber.slice(1) },
      { token: "keyword.flow", foreground: colors.syntaxKeyword.slice(1) },
      { token: "keyword.flow.scss", foreground: colors.syntaxKeyword.slice(1) },
      { token: "attribute.value", foreground: colors.syntaxString.slice(1) },
      { token: "attribute.value.html", foreground: colors.syntaxString.slice(1) },
      { token: "attribute.value.xml", foreground: colors.syntaxString.slice(1) },
      { token: "attribute.value.number", foreground: colors.syntaxNumber.slice(1) },
      { token: "attribute.value.unit", foreground: colors.syntaxNumber.slice(1) },
      { token: "delimiter.html", foreground: punctuation.slice(1) },
      { token: "delimiter.xml", foreground: punctuation.slice(1) },
      { token: "metatag", foreground: punctuation.slice(1) },
      { token: "metatag.html", foreground: punctuation.slice(1) },
      { token: "metatag.xml", foreground: punctuation.slice(1) },
      { token: "metatag.content.html", foreground: tag.slice(1) },
      { token: "operator.scss", foreground: operator.slice(1) },
      { token: "operator.sql", foreground: operator.slice(1) },
      { token: "operator.swift", foreground: operator.slice(1) },
      { token: "predefined.sql", foreground: constant.slice(1) }
    ],
    colors: {
      "editor.background": colors.editorOpaque,
      "editor.foreground": colors.ink,
      "editor.lineHighlightBackground": colors.selected,
      "editorLineNumber.foreground": colors.faint,
      "editorCursor.foreground": colors.accent,
      "editor.selectionBackground": colors.selected,
      "editorGutter.background": colors.editorOpaque,
      "diffEditor.insertedTextBackground": `${colors.good}24`,
      "diffEditor.removedTextBackground": `${colors.danger}24`,
      "diffEditor.insertedLineBackground": `${colors.good}16`,
      "diffEditor.removedLineBackground": `${colors.danger}16`,
      "diffEditorOverview.insertedForeground": `${colors.good}b3`,
      "diffEditorOverview.removedForeground": `${colors.danger}b3`,
      "diffEditor.diagonalFill": colors.editorOpaque,
      "scrollbarSlider.background": scheme === "light" ? "#00000018" : "#ffffff17",
      "scrollbarSlider.hoverBackground": scheme === "light" ? "#0000002b" : "#ffffff2b",
      "minimap.background": colors.editorOpaque,
      ...legacyColors
    }
  });
}

export { monaco };

const LANGUAGE_BY_EXT: Record<string, string> = {
  ts: "typescript",
  tsx: "typescript",
  mts: "typescript",
  cts: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  json: "json",
  jsonc: "json",
  css: "css",
  scss: "scss",
  sass: "sass",
  less: "less",
  html: "html",
  htm: "html",
  md: "markdown",
  markdown: "markdown",
  py: "python",
  rb: "ruby",
  go: "go",
  rs: "rust",
  java: "java",
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  hpp: "cpp",
  cs: "csharp",
  php: "php",
  swift: "swift",
  kt: "kotlin",
  sh: "shell",
  bash: "shell",
  zsh: "shell",
  fish: "shell",
  yml: "yaml",
  yaml: "yaml",
  toml: "ini",
  ini: "ini",
  xml: "xml",
  sql: "sql",
  dockerfile: "dockerfile",
  graphql: "graphql",
  vue: "html",
  svelte: "html",
  txt: "plaintext",
  log: "plaintext",
  diff: "diff"
};

export function languageForPath(p: string): string {
  const base = p.split("/").pop() ?? "";
  const lower = base.toLowerCase();
  if (lower === "dockerfile") return "dockerfile";
  const ext = lower.includes(".") ? lower.slice(lower.lastIndexOf(".") + 1) : "";
  return LANGUAGE_BY_EXT[ext] ?? "plaintext";
}
