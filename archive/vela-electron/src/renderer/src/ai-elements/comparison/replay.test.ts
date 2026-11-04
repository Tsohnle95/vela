import { describe, expect, it } from "vitest";
import { structuredActivity, structuredToolOutput, structuredCitedText, fileTree } from "../adapters";
import type { ToolCallView } from "@shared/types";
import { projectReplay, replayEvents, type Scenario } from "./replay";
describe("comparison event replay", () => {
  it.each<Scenario>(["coding","markdown","failure","rich"])("reconstructs identical state after scrubbing backwards in %s", scenario => {
    const events = replayEvents(scenario,"Review this change");
    const middle = Math.floor(events.length/2);
    const first = projectReplay(events,middle,"Review this change");
    projectReplay(events,events.length,"Review this change");
    expect(projectReplay(events,middle,"Review this change")).toEqual(first);
    expect(first.busy).toBe(true);
    expect(projectReplay(events,events.length,"Review this change").busy).toBe(false);
  });
  it.each<Scenario>(["markdown", "rich"])("preserves whitespace through native deltas and final snapshot in %s", (scenario) => {
    const events = replayEvents(scenario,"hello");
    const end = events.findIndex(event => event.type === "session.text.ended");
    const live = projectReplay(events,end,"hello").transcript;
    const final = projectReplay(events,end+1,"hello").transcript;
    const text = (items:typeof live) => items.flatMap(item => item.kind === "assistant" ? item.parts.flatMap(part => part.kind === "text" ? [part.text]:[]) : []).join("");
    expect(text(live)).toEqual(text(final));
    expect(text(live)).toContain("\n\n");
    if (scenario === "markdown") expect(text(live)).toContain("\n\n### Verification\n\n| Check | Result |");
  });
  it("sends actual running output metadata before terminal success", () => {
    const events = replayEvents("coding","hello");
    const cursor = events.findIndex(event => event.type === "session.tool.progress" && event.data.callID === "terminal")+1;
    const assistant = projectReplay(events,cursor,"hello").transcript.find(item => item.kind === "assistant");
    expect(assistant?.kind).toBe("assistant");
    if (assistant?.kind !== "assistant") return;
    const tool = assistant.parts.find(part => part.kind === "tool" && part.tool.inputValue && JSON.stringify(part.tool.inputValue).includes("npm run test"));
    expect(tool?.kind).toBe("tool");
    if (tool?.kind === "tool") {
      expect(tool.tool.status).toBe("running");
      expect(tool.tool.metadata?.output).toMatch(/^RUN/);
    }
  });
  it("projects rich event metadata into the actual rendering adapters", () => {
    const events = replayEvents("rich", "Show the rich patterns");
    const { transcript } = projectReplay(events, events.length, "Show the rich patterns");
    const tools = transcript.flatMap(item => item.kind === "assistant" ? item.parts.flatMap(part => part.kind === "tool" ? [part.tool] : []) : []);
    function tool(title: string): ToolCallView {
      const value = tools.find(item => item.title === title);
      expect(value, title).toBeDefined();
      return value!;
    }
    expect(structuredActivity(tool("plan")).plan?.steps).toHaveLength(3);
    expect(structuredActivity(tool("implementation")).task?.steps.every(step => step.status === "done")).toBe(true);
    const rich = structuredToolOutput(tool("test_report"));
    expect(rich.suites[0].tests.map(test => test.status)).toEqual(["passed", "failed", "skipped"]);
    expect(structuredToolOutput(tool("web_search")).sources[0].url).toBe("https://ui.shadcn.com/docs/ai-elements");
    const cited = structuredCitedText(tool("research_summary"));
    expect(cited?.citations).toHaveLength(2);
    expect(cited?.citations.at(-1)?.end).toBe(cited?.text.length);
    expect(structuredToolOutput(tool("render_check")).stack).toContain("at Settings (src/settings.tsx:18:12)");
    expect(tool("render_check").status).toBe("failed");
    const files = tool("edit").metadata?.files as {relativePath: string}[];
    expect(fileTree(files.map(file => file.relativePath))).toEqual([{type: "folder", name: "src", path: "src", children: [
      {type: "file", name: "settings.test.tsx", path: "src/settings.test.tsx"},
      {type: "file", name: "settings.tsx", path: "src/settings.tsx"},
    ]}]);
  });

});
