import { describe, expect, it } from "vitest";
import { createMarkdownWorkerHandler } from "./markdown-engine";
import { applyMarkdownWorkerResponse, type MarkdownWorkerRequest, type MarkdownWorkerResponse } from "./markdown-worker-protocol";

function engine() {
  let id = 0;
  const pending = new Map<number, (response: MarkdownWorkerResponse) => void>();
  const handle = createMarkdownWorkerHandler((response) => {
    pending.get(response.id)?.(response);
    pending.delete(response.id);
  });
  return (request: Omit<Extract<MarkdownWorkerRequest, { type: "parse" }>, "id"> |
    Omit<Extract<MarkdownWorkerRequest, { type: "highlight" }>, "id">) => {
    const requestID = ++id;
    const result = new Promise<MarkdownWorkerResponse>((resolve) => pending.set(requestID, resolve));
    handle({ ...request, id: requestID });
    return result;
  };
}

describe("OpenCode Markdown worker", () => {
  it("uses upstream Marked link attributes, KaTeX, and actual OpenCode Shiki token colors", async () => {
    const send = engine();
    const links = await send({ type: "parse", text: "[OpenCode](https://opencode.ai)" });
    expect(links).toMatchObject({
      type: "parse",
      html: '<p><a href="https://opencode.ai" class="external-link" target="_blank" rel="noopener noreferrer">OpenCode</a></p>\n'
    });
    const math = await send({ type: "parse", text: "\\(x^2\\)\n\n$$\nx^2\n$$\n" });
    expect(math.type).toBe("parse");
    if (math.type !== "parse") throw new Error("Expected parsed math");
    expect(math.html).toContain('class="katex"');
    expect(math.html).toContain('class="katex-display"');
    const code = await send({ type: "parse", text: "```ts\nconst value = 1\n```\n" });
    expect(code.type).toBe("parse");
    if (code.type !== "parse") throw new Error("Expected parsed code");
    expect(code.html).toContain('class="shiki OpenCode"');
    expect(code.html).toContain('style="color:var(--syntax-keyword)"');
    expect(code.html).not.toContain('tabindex=');
  });

  it("accumulates real incremental Shiki tokens, then replaces them with the complete tokenization", async () => {
    const send = engine();
    const firstText = "const value = 1;\n";
    const nextText = firstText + "console.log(value);\n";
    const first = await send({ type: "highlight", key: "code", language: "ts", text: firstText });
    expect(first.type).toBe("highlight");
    if (first.type !== "highlight") throw new Error("Expected highlighted code");
    const firstState = applyMarkdownWorkerResponse(undefined, first);
    expect([...firstState.stable, ...firstState.unstable].map(([text]) => text).join("")).toBe(firstText);

    const next = await send({ type: "highlight", key: "code", language: "ts", text: nextText });
    if (next.type !== "highlight") throw new Error("Expected highlighted append");
    expect(next.reset).toBe(false);
    const nextState = applyMarkdownWorkerResponse(firstState, next);
    expect([...nextState.stable, ...nextState.unstable].map(([text]) => text).join("")).toBe(nextText);

    const complete = await send({ type: "highlight", key: "code", language: "ts", text: nextText, complete: true });
    if (complete.type !== "highlight") throw new Error("Expected complete highlighting");
    expect(complete.reset).toBe(true);
    expect(complete.unstable).toEqual([]);
    expect(complete.stable.map(([text]) => text).join("")).toBe(nextText);
  });
});
