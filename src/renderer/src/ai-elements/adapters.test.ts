import { describe, expect, it } from "vitest";
import { fileTree, structuredToolOutput, structuredActivity, visibleAiTranscript, structuredCitedText, toolOutputText } from "./adapters";
import type { ToolCallView, TranscriptItem } from "@shared/types";
const tool: ToolCallView = { id: "t", title: "shell", detail: "npm test", status: "success" };
describe("AI Elements structured output boundaries", () => {
    it("does not infer structured tests, sources, or errors from arbitrary terminal text", () => {
        expect(structuredToolOutput({ ...tool, output: "Tests 2 passed\nhttps://example.com\nError: failed\n at fake (file.ts:1:2)" })).toEqual({ sources: [], suites: [], stack: undefined });
    });
    it("accepts real structured suites and excludes malformed records and unsafe source URLs", () => {
        expect(structuredToolOutput({ ...tool, metadata: { output: { sources: [{ url: "javascript:alert(1)" }, { url: "https://example.com/docs", title: "Docs" }, { url: "not a url" }], suites: [{ name: "unit", tests: [{ name: "valid", status: "passed" }, { name: "invalid", status: "maybe" }] }, { name: "empty", tests: [] }], stack: "Error: problem\n at run (app.ts:12:3)" } } })).toEqual({ sources: [{ url: "https://example.com/docs", title: "Docs", description: undefined }], suites: [{ name: "unit", tests: [{ name: "valid", status: "passed", error: undefined }] }], stack: "Error: problem\n at run (app.ts:12:3)" });
    });
    it("preserves source and test content instead of synthesizing success", () => {
        const result = structuredToolOutput({ ...tool, providerResultState: { output: { suites: [{ name: "real suite", tests: [{ name: "actual failure", status: "failed", error: "Expected 2; got 3" }] }] } } });
        expect(result.suites[0].tests[0]).toEqual({ name: "actual failure", status: "failed", error: "Expected 2; got 3" });
    });
});
describe("AI Elements changed-file tree", () => {
    it("deduplicates and nests real paths while keeping a stable sort order", () => {
        expect(fileTree(["src/z.ts", "README.md", "src/a.ts", "src/a.ts"])).toEqual([{ type: "file", name: "README.md", path: "README.md" }, { type: "folder", name: "src", path: "src", children: [{ type: "file", name: "a.ts", path: "src/a.ts" }, { type: "file", name: "z.ts", path: "src/z.ts" }] }]);
    });
});
describe("AI Elements task and plan mapping", () => {
    it("keeps provider supplied states and rejects malformed activity rather than inventing steps", () => {
        const plan = { title: "Migration plan", steps: [{ title: "Read source", status: "done" }, { title: "Implement", status: "running" }, { title: "Verify", status: "pending" }] };
        const task = { title: "Write tests", steps: [{ title: "Regression", status: "error" }] };
        expect(structuredActivity({ ...tool, metadata: { plan, task } })).toEqual({ plan, task });
        expect(structuredActivity({ ...tool, metadata: { plan: { title: "Bad", steps: [{ title: "Unknown", status: "success" }] } } })).toEqual({});
    });
});


describe("AI Elements delegated transcript visibility", () => {
 it("excludes internal pending inputs without dropping delivered synthetic content", () => {
  const transcript: TranscriptItem[] = [
   {kind:"pending-input",id:"plan",inputType:"synthetic",text:"<system-reminder>You are in Plan mode.</system-reminder>"},
   {kind:"pending-input",id:"queued",inputType:"user",text:"Queued request"},
   {kind:"synthetic",id:"delivered",text:"Visible runtime context"}
  ];
  expect(visibleAiTranscript(transcript,[])).toEqual([transcript[2]]);
  expect(transcript).toHaveLength(3);
 });
 it("uses the same delegation consolidation as the original renderer instead of exposing duplicate protocol messages", () => {
  const transcript: TranscriptItem[]=[{kind:"user",id:"u",text:"inspect"},{kind:"assistant",id:"a",messageID:"a",completed:true,parts:[{kind:"tool",id:"task",tool:{id:"task",title:"task",detail:"Inspect source",status:"running",input:JSON.stringify({agent:"research",description:"Inspect source"}),metadata:{sessionID:"child"}}}]},{kind:"synthetic",id:"protocol",text:'<subagent id="child" agent="research" description="Inspect source" state="completed" />'}];
  const result=visibleAiTranscript(transcript,[],"parent");
  expect(result.map(item => item.id)).toEqual(["u","a"]);
  expect(result[1].kind === "assistant" && result[1].parts[0].kind === "tool" && result[1].parts[0].tool.status).toBe("success");
  expect(transcript[1].kind === "assistant" && transcript[1].parts[0].kind === "tool" && transcript[1].parts[0].tool.status).toBe("running");
 });
});


describe("AI Elements streamed output and explicit citations", () => {
 it("reads partial terminal output from actual progress metadata without exposing its transport JSON", () => {
  expect(toolOutputText({...tool,status:"running",metadata:{output:"first line"},progress:'{"output":"first line"}'})).toBe("first line");
  expect(toolOutputText({...tool,status:"running",progress:'{"output":"second line"}'})).toBe("second line");
  expect(toolOutputText({...tool,output:"final",metadata:{output:"partial"}})).toBe("final");
 });
 it("requires explicit valid offset associations and web sources before rendering a citation", () => {
  expect(structuredCitedText({...tool,output:"A sentence with https://example.com"})).toBeNull();
  expect(structuredCitedText({...tool,metadata:{text:"Text.",citations:[{end:99,source:{url:"https://example.com"}},{end:5,source:{url:"javascript:alert(1)"}}]}})).toBeNull();
  expect(structuredCitedText({...tool,metadata:{text:"Text.",citations:[{end:5,source:{url:"https://example.com",title:"Source",description:"A real reference"}}]}})).toEqual({text:"Text.",citations:[{end:5,source:{url:"https://example.com",title:"Source",description:"A real reference"}}]});
 });
});
