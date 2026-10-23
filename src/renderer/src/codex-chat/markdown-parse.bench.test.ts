import { describe, expect, it } from "vitest";
import { createParseCache, parseMarkdown } from "./markdown-parse";

describe("streaming parse budget", () => {
  it("keeps a 50KB message within the interactive budget", () => {
    const block = "## Heading\n\nSome **bold** text with `code` and a [link](https://example.com).\n\n- item one\n- item two\n\n";
    const document = block.repeat(400);
    expect(document.length).toBeGreaterThan(40_000);
    const cache = createParseCache();
    const chunks = 200;
    const startedAt = performance.now();
    for (let index = 1; index <= chunks; index += 1) {
      const end = Math.floor((document.length * index) / chunks);
      parseMarkdown(document.slice(0, end), true, cache);
    }
    const elapsed = performance.now() - startedAt;
    console.log(`[codex-chat] streamed ${document.length} chars in ${chunks} chunks: ${elapsed.toFixed(1)}ms`);
    expect(elapsed).toBeLessThan(2_000);
  });
});
