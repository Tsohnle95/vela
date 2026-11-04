import { describe, expect, it } from "vitest";
import type { SessionSummary, ToolCallView, TranscriptItem } from "@shared/types";
import { consolidateSubagentTools, parseInput, taskChildID, taskDescriptor, taskDispatchSignature } from "./timeline-model";

function task(id: string, input: Record<string, unknown>, metadata?: Record<string, unknown>): ToolCallView {
  return { id, title: "task", detail: "", status: "running", input: JSON.stringify(input), metadata };
}

function assistant(id: string, tools: ToolCallView[]): Extract<TranscriptItem, { kind: "assistant" }> {
  return { kind: "assistant", id, messageID: id, completed: false,
    parts: tools.map((tool) => ({ kind: "tool", id: tool.id, tool })) };
}

const child: SessionSummary = { id: "child", directory: "/repo", title: "Inspect source", agent: "build", parentID: "parent", updatedAt: 100 };

describe("timeline task descriptors", () => {
  it("keeps an empty display description's prompt fallback separate from the dispatch signature", () => {
    const tool = task("call", { agent: "build", description: "", prompt: "  Inspect source  " });
    const descriptor = taskDescriptor(tool);
    expect(descriptor).toEqual({ agent: "build", description: "Inspect source", dispatchDescription: "" });
    expect(taskChildID(tool, [child], "parent", descriptor)).toBe("child");
    expect(taskDispatchSignature(tool, descriptor)).toBe("dispatch:build:");
  });

  it("preserves raw input precedence, agent aliases, whitespace and metadata ID priority", () => {
    const tool = task("call", { subagent_type: "BUILD", prompt: "  Inspect source  " }, { sessionId: "first", sessionID: "second" });
    tool.inputValue = { agent: "other", description: "Different" };
    expect(taskDescriptor(tool)).toEqual({ agent: "BUILD", description: "Inspect source", dispatchDescription: "  Inspect source  " });
    expect(taskDispatchSignature(tool)).toBe("dispatch:build:inspect source");
    expect(taskChildID(tool, [], "parent")).toBe("first");
    expect(taskDescriptor(task("empty-agent", { agent: "", subagent_type: "build" })).agent).toBe("");
  });

  it("retains partial JSON field recovery and ignores scalar/array inputs", () => {
    expect(parseInput('{"filePath":"src/a.ts","command":"echo \\"hello\\"')).toMatchObject({
      filePath: "src/a.ts", command: 'echo "hello"'
    });
    expect(parseInput("[]")).toEqual({});
    expect(parseInput("42")).toEqual({});
    expect(parseInput("not JSON")).toEqual({});
    expect(taskDescriptor({ ...task("partial", {}), input: '{"agent":"build","description":"Inspect' }))
      .toEqual({ agent: "", description: "Inspect", dispatchDescription: "Inspect" });
  });
});

describe("delegation consolidation ownership", () => {
  it.each(["completed", "success", "failed", "error", "cancelled", "running"])(
    "settles a represented tool from a %s synthetic row without mutating either input projection",
    (state) => {
      const first = assistant("a", [task("first", { agent: "build", description: "Inspect source" }, { sessionID: "child", first: true })]);
      const latest = assistant("b", [{ ...task("latest", { agent: "build", description: "Inspect source" }, { sessionID: "child", latest: true }), output: "Done" }]);
      const synthetic: TranscriptItem = { kind: "synthetic", id: "settled", text: `<subagent id="child" agent="build" state="${state}"></subagent>` };
      const transcript = [first, latest, synthetic];
      const snapshot = structuredClone(transcript);
      Object.freeze(first.parts);
      Object.freeze(first);
      Object.freeze(latest.parts);
      Object.freeze(latest);
      const result = consolidateSubagentTools(transcript, [child], "parent");
      const expected = ["failed", "error", "cancelled"].includes(state) ? "failed"
        : ["completed", "success"].includes(state) ? "success" : "running";
      expect(result[0]).toMatchObject({ parts: [{ tool: { id: "first", status: expected, output: "Done", metadata: { first: true, latest: true } } }] });
      expect(result[1]).toMatchObject({ parts: [] });
      expect(result[2]).toBe(synthetic);
      expect(transcript).toEqual(snapshot);
      expect(result[0]).not.toBe(first);
      expect(result[1]).not.toBe(latest);
    }
  );

  it("retains distinct children and repeated dispatches after a user turn boundary", () => {
    const first = assistant("a", [task("first", { agent: "build", description: "Inspect source" }, { sessionID: "child" })]);
    const other = assistant("b", [task("other", { agent: "build", description: "Inspect source" }, { sessionID: "other-child" })]);
    const user: TranscriptItem = { kind: "user", id: "u", text: "Again" };
    const repeated = assistant("c", [task("repeated", { agent: "build", description: "Inspect source" }, { sessionID: "child" })]);
    const result = consolidateSubagentTools([first, other, user, repeated], [child], "parent");
    expect(result.filter((item) => item.kind === "assistant").map((item) => item.parts.length)).toEqual([1, 1, 1]);
    expect(result[2]).toBe(user);
  });

  it("deduplicates the same dispatch with missing, arriving or stale child graphs", () => {
    const transcript = [assistant("a", [task("first", { agent: "build", description: "Inspect source" })]),
      assistant("b", [task("latest", { agent: "build", description: "Inspect source" })])];
    const snapshot = structuredClone(transcript);
    for (const sessions of [[], [child], [{ ...child, parentID: "other-parent" }]]) {
      const result = consolidateSubagentTools(transcript, sessions, "parent");
      expect(result[0]).toMatchObject({ parts: [{ tool: { id: "first" } }] });
      expect(result[1]).toMatchObject({ parts: [] });
      expect(transcript).toEqual(snapshot);
    }
  });
});
