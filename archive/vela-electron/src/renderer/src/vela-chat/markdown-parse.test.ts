import { describe, expect, it } from "vitest";
import { createParseCache, parseMarkdown, safePrefixEnd } from "./markdown-parse";

function full(text: string) {
  return parseMarkdown(text, false, createParseCache());
}

describe("safePrefixEnd", () => {
  it("cuts after a plain paragraph", () => {
    expect(safePrefixEnd("Para.\n\nMore text")).toBe(7);
  });

  it("does not cut inside a fenced block", () => {
    const text = "Para.\n\n```ts\nconst a = 1;\n\nconst b = 2;\n```";
    expect(safePrefixEnd(text)).toBe(7);
  });

  it("does not cut inside a list", () => {
    expect(safePrefixEnd("Intro.\n\n- one\n\n- two")).toBe(8);
  });

  it("returns zero when no boundary is safe", () => {
    expect(safePrefixEnd("one growing line")).toBe(0);
  });
});

describe("parseMarkdown", () => {
  it("matches a full lex at every growth step", () => {
    const cache = createParseCache();
    const steps = [
      "Intro.",
      "Intro.\n\nSecond paragraph.",
      "Intro.\n\nSecond paragraph.\n\n- one\n- two",
      "Intro.\n\nSecond paragraph.\n\n- one\n- two\n\n```ts\nconst a = 1;\n```"
    ];
    for (const text of steps) {
      expect(JSON.stringify(parseMarkdown(text, true, cache))).toBe(JSON.stringify(full(text)));
    }
  });

  it("reuses prefix token identity while streaming", () => {
    const cache = createParseCache();
    const first = parseMarkdown("Alpha paragraph.", true, cache);
    const second = parseMarkdown("Alpha paragraph.\n\nBeta paragraph.", true, cache);
    expect(second[0]).toBe(first[0]);
  });

  it("reuses the exact cached parse for completed text", () => {
    const cache = createParseCache();
    const first = parseMarkdown("Done.", true, cache);
    expect(parseMarkdown("Done.", false, cache)).toBe(first);
  });
});
