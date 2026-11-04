import { describe, expect, it } from "vitest";
import { parseAgentMentions } from "./agent-mentions";

describe("agent mentions", () => {
  it("returns no mention when there are no non-primary agents", () => {
    expect(parseAgentMentions("@builder run it", [])).toEqual({ sanitizedText: "@builder run it", mention: null });
    expect(parseAgentMentions("hello", [{ id: "a", name: "builder" }])).toEqual({ sanitizedText: "hello", mention: null });
  });

  it("finds the first @-mention of a non-primary agent", () => {
    const result = parseAgentMentions("ask @reviewer about @builder", [
      { id: "a", name: "builder", mode: "primary" },
      { id: "b", name: "reviewer", mode: "subagent" }
    ]);

    expect(result.mention?.name).toBe("reviewer");
    expect(result.mention?.source).toEqual({ value: "@reviewer", start: 4, end: 13 });
  });

  it("ignores mentions that are not on word boundaries", () => {
    const result = parseAgentMentions("email@reviewer.com", [
      { id: "b", name: "reviewer", mode: "subagent" }
    ]);

    expect(result.mention).toBeNull();
  });
});
