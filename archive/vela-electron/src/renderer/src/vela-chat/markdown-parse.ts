import { Marked } from "marked";

/**
 * Streaming parse strategy, ported from Codex's `qSd`/`JSd`/`YSd`:
 * while text grows, only the tail after the last safe block boundary is
 * lexed again; earlier tokens are reused by identity so React sees unchanged
 * blocks. A boundary is only trusted outside fenced code and outside list,
 * quote, table, or indented continuations, where a tail lex would change
 * block context.
 */

export interface MdToken {
  type: string;
  raw?: string;
  text?: string;
  tokens?: MdToken[];
  items?: MdToken[];
  header?: MdToken[];
  rows?: MdToken[][];
  lang?: string;
  href?: string;
  title?: string | null;
  depth?: number;
  ordered?: boolean;
  start?: number;
  task?: boolean;
  checked?: boolean | null;
}

export interface ParseCache {
  prefixes: Map<string, MdToken[]>;
  lastText: string;
}

const PREFIX_CACHE_LIMIT = 100;

const inlineMath = /^\\\(((?:\\.|[^\\\n])*?)\\\)/;
const blockMath = /^\$\$\n([\s\S]+?)\n\$\$(?:\n|$)/;

const lexer = new Marked({ breaks: true, gfm: true });
lexer.use({
  extensions: [
    {
      name: "inlineMath",
      level: "inline",
      start(src) {
        const index = src.indexOf("\\(");
        return index === -1 ? undefined : index;
      },
      tokenizer(src) {
        const match = src.match(inlineMath);
        if (!match) return undefined;
        return { type: "inlineMath", raw: match[0], text: match[1].trim(), displayMode: false } as never;
      }
    },
    {
      name: "blockMath",
      level: "block",
      tokenizer(src) {
        const match = src.match(blockMath);
        if (!match) return undefined;
        return { type: "blockMath", raw: match[0], text: match[1].trim(), displayMode: true } as never;
      }
    }
  ]
});

function lex(text: string): MdToken[] {
  return lexer.lexer(text) as unknown as MdToken[];
}

export function createParseCache(): ParseCache {
  return { prefixes: new Map(), lastText: "" };
}

export function resetParseCache(cache: ParseCache): void {
  cache.prefixes.clear();
  cache.lastText = "";
}

function remember(cache: ParseCache, text: string, tokens: MdToken[]): void {
  cache.prefixes.set(text, tokens);
  cache.lastText = text;
  while (cache.prefixes.size > PREFIX_CACHE_LIMIT) {
    const oldest = cache.prefixes.keys().next().value;
    if (oldest === undefined) break;
    cache.prefixes.delete(oldest);
  }
}

function isFenceLine(line: string): boolean {
  const trimmed = line.trimStart();
  return trimmed.startsWith("```") || trimmed.startsWith("~~~");
}

function isContinuationLine(line: string): boolean {
  if (/^[\t ]/.test(line)) return true;
  return /^(?:[-+*](?:[\t ]|$)|\d+(?:[.)](?:[\t ]|$)|$)|>|\|)/.test(line);
}

function previousLineBefore(text: string, boundary: number): string {
  let cursor = boundary - 2;
  while (cursor > 0) {
    const newline = text.lastIndexOf("\n", cursor - 1);
    const lineStart = newline < 0 ? 0 : newline + 1;
    const line = text.slice(lineStart, cursor + 1);
    if (line.trim()) return line;
    cursor = newline < 0 ? 0 : newline;
  }
  return "";
}

/** Last blank-line boundary whose tail can be lexed independently. */
export function safePrefixEnd(text: string): number {
  let best = 0;
  let fenceOpen = false;
  let blankSeen = false;
  let previous = "";
  let offset = 0;
  while (offset <= text.length) {
    const newline = text.indexOf("\n", offset);
    const lineEnd = newline === -1 ? text.length : newline;
    const line = text.slice(offset, lineEnd);
    if (line.trim() === "") {
      blankSeen = true;
    } else {
      if (blankSeen && !fenceOpen && !/^[\t ]/.test(line) && !isContinuationLine(previous)) {
        best = offset;
      }
      blankSeen = false;
      previous = line;
      if (isFenceLine(line)) fenceOpen = !fenceOpen;
    }
    if (newline === -1) break;
    offset = newline + 1;
  }
  return best;
}

function longestPrefix(cache: ParseCache, text: string): { prefix: string; tokens: MdToken[] } | null {
  if (cache.lastText && text.startsWith(cache.lastText)) {
    const tokens = cache.prefixes.get(cache.lastText);
    if (tokens !== undefined) return { prefix: cache.lastText, tokens };
  }
  let best: { prefix: string; tokens: MdToken[] } | null = null;
  for (const [candidate, tokens] of cache.prefixes) {
    if (candidate.length > text.length || !text.startsWith(candidate)) continue;
    if (best === null || candidate.length > best.prefix.length) best = { prefix: candidate, tokens };
  }
  return best;
}

function parsePrefix(prefix: string, cache: ParseCache): MdToken[] {
  const exact = cache.prefixes.get(prefix);
  if (exact !== undefined) return exact;
  const reusable = longestPrefix(cache, prefix);
  const rest = reusable === null ? prefix : prefix.slice(reusable.prefix.length);
  const tokens = reusable === null ? lex(prefix) : [...reusable.tokens, ...lex(rest)];
  remember(cache, prefix, tokens);
  return tokens;
}

/**
 * Parse `text`, reusing the longest cached prefix while streaming. The result
 * is identical to a full lex; only the work differs.
 */
export function parseMarkdown(text: string, streaming: boolean, cache: ParseCache): MdToken[] {
  if (!text) return [];
  const exact = cache.prefixes.get(text);
  if (exact !== undefined) return exact;
  const split = streaming ? safePrefixEnd(text) : 0;
  if (split <= 0) {
    const tokens = lex(text);
    remember(cache, text, tokens);
    return tokens;
  }
  const prefixTokens = parsePrefix(text.slice(0, split), cache);
  const tokens = [...prefixTokens, ...lex(text.slice(split))];
  remember(cache, text, tokens);
  return tokens;
}
