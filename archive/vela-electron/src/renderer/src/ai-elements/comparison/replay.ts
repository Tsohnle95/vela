import type { TranscriptItem } from "@shared/types";
import { applyChatEvent, projectAssistantItems, type ChatDirectoryState } from "../../chat-store";
import type { ChatStreamEvent } from "../../chat-stream";

export const comparisonSessionID = "comparison";
export type Scenario = "coding" | "markdown" | "failure" | "rich";
export function replayEvents(scenario: Scenario, prompt: string): ChatStreamEvent[] {
  const events: ChatStreamEvent[] = [];
  const push = (type: string, data: Record<string, unknown> = {}) => events.push({id:`event-${events.length}`, type, created: 1700000000000 + events.length * 100, data: {sessionID: comparisonSessionID, assistantMessageID:"assistant", ...data}});
  const chunks = (type: string, text: string, data: Record<string, unknown> = {}) => {
    for (const delta of text.match(/[\s\S]{1,10}/g) ?? []) push(type, {...data, delta});
  };
  push("session.step.started", {agent:"build",model:{id:"reference-model",providerID:"demo"}});
  push("session.reasoning.started");
  chunks("session.reasoning.delta", "I will inspect the current implementation, make the smallest change, and verify it with a focused test.");
  push("session.reasoning.ended", {text:"I will inspect the current implementation, make the smallest change, and verify it with a focused test."});
  const tool = (id: string, name: string, input: Record<string, unknown>, output: string, failed = false, metadata?: Record<string, unknown>) => {
    push("session.tool.input.started", {callID:id,name});
    push("session.tool.called", {callID:id,name,input});
    for (let n = 1; n <= 4; n++) push("session.tool.progress", {callID:id,metadata:{output:output.slice(0,Math.ceil(output.length*n/4))}});
    push(failed ? "session.tool.failed" : "session.tool.success", {callID:id, content:[{type:"text",text:output}], output, metadata, ...(failed ? {error:output} : {})});
  };
  if (scenario === "coding") {
    tool("read", "read", {filePath:"src/settings.tsx"}, "export function Settings() { return <main>Settings</main>; }");
    tool("search", "grep", {pattern:"ThemeProvider",path:"src"}, "src/App.tsx:12: <ThemeProvider defaultTheme=\"system\">");
    tool("terminal", "bash", {command:"npm run test -- settings"}, "RUN v3.2.7\n✓ settings.test.tsx (3 tests)\nTest Files  1 passed (1)\nTests       3 passed (3)");
  }
  if (scenario === "rich") richReplayTools(tool);
  if (scenario === "failure") tool("failed", "bash", {command:"npm run build"}, "TypeError: Cannot read properties of undefined (reading 'theme')\n    at Settings (src/settings.tsx:18:12)\n    at render (src/App.tsx:24:3)", true);
  const text = scenario === "rich" ? "This deterministic replay shows structured plans, task progress, edited files, sources, inline citations, test suites, and a captured stack trace.\n\nThe sample includes a failed test to make error presentation visible. These are replay fixtures; no commands ran and no files changed.\n\n```tsx\nconst { setTheme } = useTheme();\n```" : scenario === "markdown" ? `Here is a streaming Markdown response for **${prompt}**.\n\n### Code\n\n\`\`\`tsx\nexport function Settings() {\n  return <button onClick={() => setTheme(\"dark\")}>Dark mode</button>;\n}\n\`\`\`\n\n### Verification\n\n| Check | Result |\n| --- | --- |\n| TypeScript | Passed |\n| Component tests | 3 passed |\n\nA partially received **bold phrase**, an [external reference](https://ui.shadcn.com/docs/ai-elements), and a list:\n\n- Preserve the reader’s scroll position.\n- Keep code stable as tokens arrive.\n- Finish with the complete message.\n` : scenario === "failure" ? "The build failed in `Settings` because `theme` was undefined. The error and captured command output remain available above.\n\nI would check the provider boundary before changing the component." : "Added the theme toggle to the settings page.\n\n- Reused the existing theme provider.\n- Kept the system setting as the default.\n- Verified the toggle with **3 passing tests**.\n\n```tsx\nconst { setTheme } = useTheme();\n```\n\nThe change is ready for review.";
  push("session.text.started");
  chunks("session.text.delta", text);
  push("session.text.ended", {text});
  push("session.step.ended", {finish:"stop"});
  return events;
}
export function projectReplay(events: ChatStreamEvent[], cursor: number, prompt: string): {transcript: TranscriptItem[]; busy: boolean} {
  const state: ChatDirectoryState = {message:{},part:{},session_status:{}};
  for (const event of events.slice(0,cursor)) applyChatEvent(state, comparisonSessionID, event);
  const transcript: TranscriptItem[] = [{kind:"user",id:"user",text:prompt,createdAt:1700000000000},...projectAssistantItems(state, comparisonSessionID)];
  return {transcript,busy:cursor > 0 && cursor < events.length};
}

type ReplayTool = (id: string, name: string, input: Record<string, unknown>, output: string, failed?: boolean, metadata?: Record<string, unknown>) => void;
function richReplayTools(tool: ReplayTool): void {
  tool("plan", "plan", { goal: "Add a settings theme toggle" }, "Inspect, implement, and verify the settings toggle.", false, {
    plan: { title: "Add a settings theme toggle", steps: [
      { title: "Inspect the theme provider", status: "done" },
      { title: "Add the settings toggle", status: "done" },
      { title: "Verify behavior and report failures", status: "done" },
    ] },
  });
  tool("activity", "implementation", { directory: "src" }, "Implementation steps completed.", false, {
    task: { title: "Update settings", steps: [
      { title: "Read src/settings.tsx", status: "done" },
      { title: "Reuse the existing ThemeProvider", status: "done" },
      { title: "Add settings regression coverage", status: "done" },
    ] },
  });
  tool("edit", "edit", { filePath: "src/settings.tsx", oldString: "<main>Settings</main>", newString: "<main>Settings<ModeToggle /></main>" }, "Updated src/settings.tsx and added regression coverage.", false, {
    files: [
      { relativePath: "src/settings.tsx", before: "<main>Settings</main>", after: "<main>Settings<ModeToggle /></main>", additions: 1, deletions: 1, type: "modified" },
      { relativePath: "src/settings.test.tsx", before: "", after: 'it("follows the system theme", () => { expect(defaultTheme).toBe("system"); });', additions: 1, deletions: 0, type: "added" },
    ],
  });
  richReferenceTools(tool);
  richVerificationTools(tool);
}
function richReferenceTools(tool: ReplayTool): void {
  const source = { url: "https://ui.shadcn.com/docs/ai-elements", title: "shadcn/ui AI Elements", description: "Official compositions for streaming agent interfaces." };
  tool("sources", "web_search", { query: "shadcn UI AI Elements patterns" }, "Found the official AI Elements documentation.", false, { sources: [source, { url: "https://ui.shadcn.com/docs/components/base/trace", title: "Trace", description: "Shared step layout used by agent patterns." }] });
  const text = "AI Elements are compositions of shadcn/ui components. Trace provides the shared shape for agent steps.";
  tool("citations", "research_summary", { topic: "AI Elements architecture" }, text, false, {
    text, citations: [{ end: 53, source }, { end: text.length, source: { url: "https://ui.shadcn.com/docs/components/base/trace", title: "Trace", description: "Agent steps built from shared Trace primitives." } }],
  });
}
function richVerificationTools(tool: ReplayTool): void {
  const stack = "TypeError: Cannot read properties of undefined (reading 'theme')\n    at Settings (src/settings.tsx:18:12)\n    at render (src/App.tsx:24:3)";
  tool("terminal", "bash", { command: "npm run test -- settings" }, "RUN v3.2.7\n✓ uses system theme by default\n× renders without the provider\n↓ follows OS changes (device check)", true);
  tool("tests", "test_report", { report: "settings.test.tsx" }, "1 passed, 1 failed, 1 skipped.", false, {
    suites: [{ name: "settings.test.tsx", tests: [
      { name: "uses system theme by default", status: "passed" },
      { name: "renders without the provider", status: "failed", error: stack },
      { name: "follows OS changes (device check)", status: "skipped" },
    ] }],
  });
  tool("stack", "render_check", { filePath: "src/settings.tsx" }, stack, true, { stack });
}
