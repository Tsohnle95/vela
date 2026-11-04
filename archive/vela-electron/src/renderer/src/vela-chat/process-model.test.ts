import { describe, expect, it } from "vitest";
import type { AssistantPartView, TranscriptItem } from "@shared/types";
import type { TimelineTurn } from "../components/timeline-model";
import { processTitle, turnProcess } from "./process-model";

function assistant(id: string, parts: AssistantPartView[], completed = true): Extract<TranscriptItem, { kind: "assistant" }> {
  return {
    kind: "assistant",
    id,
    messageID: id,
    parts,
    completed,
    createdAt: 1_000,
    completedAt: 3_000
  };
}

function turn(body: TranscriptItem[]): TimelineTurn {
  return {
    id: "turn-1",
    user: { kind: "user", id: "user-1", text: "Go", createdAt: 0 },
    body: body.filter((item): item is TimelineTurn["body"][number] => item.kind !== "user")
  };
}

const tool: AssistantPartView = {
  kind: "tool",
  id: "tool-1",
  tool: {
    id: "call-1",
    title: "bash",
    detail: "npm test",
    status: "success",
    input: JSON.stringify({ command: "npm test" }),
    inputValue: { command: "npm test" },
    output: "pass"
  }
};

describe("turnProcess", () => {
  it("treats trailing text after the last tool as the answer and folds the rest", () => {
    const model = turnProcess(turn([assistant("a", [
      { kind: "reasoning", id: "r", text: "Think first", complete: true },
      tool,
      { kind: "text", id: "t", text: "The answer", complete: true }
    ])]), false);
    expect(model.answer?.parts.map((part) => part.text)).toEqual(["The answer"]);
    expect(model.counts).toEqual([{ kind: "commands", count: 1 }]);
    expect(model.foldable).toBe(true);
    expect(model.hasProcess).toBe(true);
    expect(model.rows).toHaveLength(1);
  });

  it("does not answer from a later tool-only step", () => {
    const model = turnProcess(turn([
      assistant("a", [{ kind: "text", id: "commentary", text: "Checking", complete: true }]),
      assistant("b", [tool])
    ]), false);
    expect(model.answer).toBeNull();
    expect(model.rows).toHaveLength(2);
  });

  it("does not answer from text before a tool in the last step", () => {
    const model = turnProcess(turn([
      assistant("a", [
        { kind: "text", id: "commentary", text: "Checking", complete: true },
        tool
      ])
    ]), false);
    expect(model.answer).toBeNull();
  });

  it("stays open while the turn is live", () => {
    const liveTool: AssistantPartView = { ...tool, tool: { ...tool.tool, status: "running" } };
    const model = turnProcess(turn([assistant("a", [liveTool], false)]), true);
    expect(model.foldable).toBe(false);
    expect(model.live.running).toBe(true);
    expect(model.live.activity).toBe("commands");
    expect(model.live.detail).toContain("npm test");
  });

  it("composes the DSH closed group title", () => {
    expect(processTitle([])).toBe("Analysis completed");
    expect(processTitle([{ kind: "read", count: 2 }])).toBe("Read files");
    expect(processTitle([{ kind: "read", count: 2 }, { kind: "search", count: 1 }])).toBe("Read files and searched code");
    expect(processTitle([
      { kind: "read", count: 2 },
      { kind: "edit", count: 2 },
      { kind: "commands", count: 1 },
      { kind: "tools", count: 1 }
    ])).toBe("Read files, edited files, ran commands, etc.");
  });
});
