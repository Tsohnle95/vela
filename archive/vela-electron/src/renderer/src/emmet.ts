import { expandAbbreviation } from "emmet-monaco-es";

const HTML_LANGUAGES = new Set(["html"]);
const STYLESHEET_LANGUAGES = new Set(["css", "scss", "less"]);

const COMMON_TAGS = new Set([
  "body", "head", "html", "address", "blockquote", "dd", "div", "section", "article", "aside",
  "header", "footer", "nav", "menu", "dl", "dt", "fieldset", "form", "frame", "frameset",
  "h1", "h2", "h3", "h4", "h5", "h6", "iframe", "noframes", "object", "ol", "p", "ul",
  "applet", "center", "dir", "hr", "pre", "a", "abbr", "acronym", "area", "b", "base",
  "basefont", "bdo", "big", "br", "button", "caption", "cite", "code", "col", "colgroup",
  "del", "dfn", "em", "font", "i", "img", "input", "ins", "isindex", "kbd", "label", "legend",
  "li", "link", "map", "meta", "noscript", "optgroup", "option", "param", "q", "s", "samp",
  "script", "select", "small", "span", "strike", "strong", "style", "sub", "sup", "table",
  "tbody", "td", "textarea", "tfoot", "th", "thead", "title", "tr", "tt", "u", "var", "canvas",
  "main", "figure", "plaintext", "figcaption", "hgroup", "details", "summary", "lorem"
]);

const ABBR_CHAR = /[a-zA-Z0-9!.\-_:$#@%^*+>~()[\]{}=\\/|,]/;
const MARKUP_START = /^[a-zA-Z!(),.#{\[]/;
const STYLESHEET_START = /^-?[a-zA-Z!@#,]/;

export interface EmmetSnippet {
  abbr: string;
  startColumn: number;
  snippet: string;
}

export function emmetSnippetAt(beforeCaret: string, language: string): EmmetSnippet | null {
  if (!HTML_LANGUAGES.has(language) && !STYLESHEET_LANGUAGES.has(language)) return null;
  let start = beforeCaret.length;
  while (start > 0 && ABBR_CHAR.test(beforeCaret[start - 1])) start--;
  const abbr = beforeCaret.slice(start);
  if (!abbr) return null;
  const snippet = expandedSnippet(abbr, language);
  if (!snippet) return null;
  return { abbr, startColumn: start + 1, snippet };
}

function expandedSnippet(abbr: string, language: string): string | null {
  const stylesheet = STYLESHEET_LANGUAGES.has(language);
  if (stylesheet ? !STYLESHEET_START.test(abbr) : !MARKUP_START.test(abbr)) return null;
  if (abbr.startsWith("!") && /[^!]/.test(abbr)) return null;
  let expanded: string;
  try {
    expanded = expandAbbreviation(abbr, {
      type: stylesheet ? "stylesheet" : "markup",
      syntax: stylesheet ? "css" : "html",
      options: {
        "output.field": (index: number, placeholder: string) =>
          `\${${index}${placeholder ? `:${placeholder}` : ""}}`
      }
    });
  } catch {
    return null;
  }
  if (!expanded || isNoise(abbr, expanded, stylesheet)) return null;
  return restoreTabStops(expanded);
}

function isNoise(abbr: string, expanded: string, stylesheet: boolean): boolean {
  const withoutTabStops = expanded.replace(/\$\{\d+(?::[^}]*)?\}/g, "");
  if (stylesheet) {
    const compact = (value: string): string => value.replace(/\s/g, "");
    const unresolved = compact(`${abbr};`);
    return compact(withoutTabStops) === unresolved || compact(withoutTabStops) === compact(`${abbr}: ;`);
  }
  const lower = abbr.toLowerCase();
  if (abbr === ".") return false;
  if (abbr.endsWith(">") || abbr.startsWith(">")) return true;
  if (COMMON_TAGS.has(lower)) return false;
  if (/[-,:]/.test(abbr) && !/--|::/.test(abbr) && !abbr.endsWith(":")) return false;
  if (/^\w+\.$/.test(abbr) && !COMMON_TAGS.has(abbr.slice(0, -1).toLowerCase())) return true;
  return withoutTabStops.toLowerCase() === `<${lower}></${lower}>`;
}

function restoreTabStops(expanded: string): string {
  let snippet = expanded;
  const title = /<title>([^<$]+)<\/title>/.exec(snippet);
  if (title) snippet = snippet.replace(title[0], `<title>\${1:${title[1]}}</title>`);
  const body = /<body>((?:\r?\n)[\t ]*)(\r?\n)<\/body>/.exec(snippet);
  if (body) {
    snippet = snippet.replace(body[0], `<body>${body[1]}\${0}${body[2]}</body>`);
  } else if (!snippet.includes("${")) {
    const emptyPair = />(<\/[a-zA-Z][^>]*>)/.exec(snippet);
    if (emptyPair && emptyPair.index !== undefined) {
      const insertAt = emptyPair.index + 1;
      snippet = `${snippet.slice(0, insertAt)}\${0}${snippet.slice(insertAt)}`;
    } else {
      snippet = `${snippet}\${0}`;
    }
  }
  return snippet.replace(/(?<!\\)\$(?!\{)/g, `\\$1`);
}