import { marked, type Tokens } from "marked";
import remend from "remend";
import { completedProjection, type Block, type Projection } from "./markdown-projection";

function refs(text: string): boolean {
  if (!text.includes("]:")) return false;
  return /^[ \t]{0,3}\[[^\]]+\]:[ \t]*(?:\S+|\r?\n[ \t]+\S+)/m.test(text);
}

function language(value: string | undefined): string | undefined {
  return value?.trim().split(/\s+/, 1)[0] || undefined;
}

function openCode(raw: string): string {
  const newline = raw.indexOf("\n");
  return newline < 0 ? "" : raw.slice(newline + 1);
}

function open(raw: string): boolean {
  const match = raw.match(/^[ \t]{0,3}(`{3,}|~{3,})/);
  if (!match) return false;
  const mark = match[1];
  if (!mark) return false;
  const char = mark[0];
  const size = mark.length;
  const last = raw.trimEnd().split("\n").at(-1)?.trim() ?? "";
  return !new RegExp(`^[\\t ]{0,3}${char}{${size},}[\\t ]*$`).test(last);
}

function closesFence(raw: string, suffix: string): boolean {
  const mark = raw.match(/^[ \t]{0,3}(`{3,}|~{3,})/)?.[1];
  if (!mark) return suffix.includes("```") || suffix.includes("~~~");
  return `${raw.slice(-(mark.length - 1))}${suffix}`.includes(mark);
}

function heal(text: string): string {
  return remend(text, { linkMode: "text-only" });
}

export function stream(text: string, live: boolean): Block[] {
  if (!live) return completedProjection(text).blocks;
  if (refs(text)) return [{ raw: text, src: heal(text), mode: "live" }] satisfies Block[];
  const tokens = marked.lexer(text);
  let tail = -1;
  for (let index = tokens.length - 1; index >= 0; index -= 1) {
    if (tokens[index]?.type !== "space") {
      tail = index;
      break;
    }
  }
  if (tail < 0) return [{ raw: text, src: heal(text), mode: "live" }] satisfies Block[];
  const last = tokens[tail];
  if (!last) return [{ raw: text, src: heal(text), mode: "live" }] satisfies Block[];

  const result: Block[] = [];
  for (let index = 0; index < tail; index++) {
    const token = tokens[index];
    if (!token || token.type === "space") continue;
    let raw = token.raw;
    while (tokens[index + 1]?.type === "space" && index + 1 < tail) raw += tokens[++index]!.raw;
    if (token.type === "code") {
      const code = token as Tokens.Code;
      result.push({ raw, src: code.text, mode: "code", language: language(code.lang), complete: true });
      continue;
    }
    result.push({ raw, src: raw, mode: "full" });
  }

  const raw = tokens
    .slice(tail)
    .map((token) => token.raw)
    .join("");
  if (last.type !== "code") return [...result, { raw, src: heal(raw), mode: "live" }];

  const code = last as Tokens.Code;
  if (!open(code.raw)) {
    return [...result, { raw, src: code.text, mode: "code", language: language(code.lang), complete: true }];
  }
  return [...result, { raw, src: openCode(code.raw), mode: "code", language: language(code.lang) }];
}

export function project(previous: Projection | undefined, text: string, live: boolean): Projection {
  if (!live) {
    const current =
      previous?.text === text
        ? previous
        : previous && text.startsWith(previous.text)
          ? project(previous, text, true)
          : undefined;
    if (!current) return completedProjection(text);
    return {
      text,
      blocks: current.blocks.map((block) => {
        if (block.mode === "live") return { raw: block.raw, src: block.raw, mode: "full" };
        if (block.mode === "code" && !block.complete) return { ...block, complete: true };
        return block;
      })
    };
  }
  if (!previous || !text.startsWith(previous.text)) return { text, blocks: stream(text, live) };
  const tail = previous.blocks.at(-1);
  const suffix = text.slice(previous.text.length);
  if (!suffix || tail?.mode !== "code" || tail.complete || closesFence(tail.raw, suffix)) {
    return { text, blocks: stream(text, live) };
  }
  return {
    text,
    blocks: [
      ...previous.blocks.slice(0, -1),
      {
        ...tail,
        raw: tail.raw + suffix,
        src: tail.src + suffix
      }
    ]
  };
}
