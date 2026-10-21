import { toolInput, consolidateSubagentTools, taskChildID, taskDescriptor, taskDispatchSignature, toolKey, parseSubagentTag, parseLegacyTaskText, subagentChildID, refDispatchSignature, isInternalSystemReminder } from "../components/timeline-model";
import type { ToolCallView, TranscriptItem, SessionSummary } from "@shared/types";
import type { FileNode, SourcePart, TestSuite } from "./patterns";
function record(value: unknown): Record<string, unknown> | null {
    return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;
}
function httpURL(value: unknown): value is string {
    if (typeof value !== "string")
        return false;
    try {
        return ["https:", "http:"].includes(new URL(value).protocol);
    }
    catch {
        return false;
    }
}
// Rich output is opt-in structured data; terminal prose is never promoted to citations or test results.
export function structuredToolOutput(tool: ToolCallView): {
    sources: SourcePart[];
    suites: TestSuite[];
    stack?: string;
} {
    const data = record(tool.providerResultState?.output) ?? record(tool.metadata?.output) ?? tool.metadata ?? {};
    const sources: SourcePart[] = Array.isArray(data.sources) ? data.sources.flatMap(value => {
        const source = record(value);
        return source && httpURL(source.url) ? [{ url: source.url, title: typeof source.title === "string" ? source.title : undefined, description: typeof source.description === "string" ? source.description : undefined }] : [];
    }) : [];
    const suites: TestSuite[] = Array.isArray(data.suites) ? data.suites.flatMap(value => {
        const suite = record(value);
        if (!suite || typeof suite.name !== "string" || !Array.isArray(suite.tests))
            return [];
        const tests = suite.tests.flatMap(value => {
            const test = record(value);
            return test && typeof test.name === "string" && ["passed", "failed", "skipped"].includes(String(test.status)) ? [{ name: test.name, status: test.status as "passed" | "failed" | "skipped", error: typeof test.error === "string" ? test.error : undefined }] : [];
        });
        return tests.length ? [{ name: suite.name, tests }] : [];
    }) : [];
    return { sources, suites, stack: typeof data.stack === "string" && /\n\s*at /.test(data.stack) ? data.stack : undefined };
}
export function fileTree(paths: string[]): FileNode[] {
    const nodes: FileNode[] = [];
    for (const path of [...new Set(paths)].sort()) {
        const segments = path.split("/").filter(Boolean);
        let children = nodes;
        segments.forEach((name, index) => {
            const prefix = segments.slice(0, index + 1).join("/");
            const existing = children.find(node => node.path === prefix);
            if (index === segments.length - 1) {
                if (!existing)
                    children.push({ type: "file", name, path: prefix });
            }
            else if (existing?.type === "folder")
                children = existing.children;
            else if (!existing) {
                const folder: FileNode = { type: "folder", name, path: prefix, children: [] };
                children.push(folder);
                children = folder.children;
            }
        });
    }
    return nodes;
}
export function structuredActivity(tool: ToolCallView): {
    plan?: {
        title: string;
        steps: {
            title: string;
            status: "pending" | "running" | "done";
        }[];
    };
    task?: {
        title: string;
        steps: {
            title: string;
            status: "done" | "running" | "error";
        }[];
    };
} {
    const data = record(tool.providerResultState?.output) ?? record(tool.metadata?.output) ?? tool.metadata ?? {};
    const result: ReturnType<typeof structuredActivity> = {};
    for (const kind of ["plan", "task"] as const) {
        const activity = record(data[kind]);
        const statuses = kind === "plan" ? ["pending", "running", "done"] : ["done", "running", "error"];
        if (!activity || typeof activity.title !== "string" || !Array.isArray(activity.steps))
            continue;
        if (!activity.steps.every(value => { const step = record(value); return step && typeof step.title === "string" && statuses.includes(String(step.status)); }))
            continue;
        // Every field is checked at this trust boundary; arbitrary provider objects stay generic output.
        if (kind === "plan")
            result.plan = activity as NonNullable<ReturnType<typeof structuredActivity>["plan"]>;
        else
            result.task = activity as NonNullable<ReturnType<typeof structuredActivity>["task"]>;
    }
    return result;
}


export function visibleAiTranscript(transcript: TranscriptItem[], sessions: SessionSummary[], sessionID?: string): TranscriptItem[] {
  const consolidated = consolidateSubagentTools(transcript, sessions, sessionID);
  const represented = new Set<string>();
  let turn=0;
  for (const item of consolidated) {
    if (item.kind === "user") turn++;
    if (item.kind !== "assistant") continue;
    for (const part of item.parts) {
      if (part.kind !== "tool" || !["task", "subagent"].includes(toolKey(part.tool.title))) continue;
      const descriptor=taskDescriptor(part.tool);
      const childID=taskChildID(part.tool,sessions,sessionID,descriptor);
      const signature=taskDispatchSignature(part.tool,descriptor);
      if (childID) represented.add(`${turn}:child:${childID}`);
      if (signature) represented.add(`${turn}:${signature}`);
    }
  }
  turn=0;
  return consolidated.filter(item => {
    if (item.kind === "pending-input") return false;
    if (item.kind === "user") turn++;
    if (item.kind !== "synthetic") return true;
    if (isInternalSystemReminder(item)) return false;
    const ref=parseSubagentTag(item.text) ?? parseLegacyTaskText(item.text);
    if (!ref) return true;
    const childID=subagentChildID(ref,sessions,sessionID);
    return !represented.has(`${turn}:child:${childID}`) && !represented.has(`${turn}:${refDispatchSignature(ref)}`);
  });
}

export function toolOutputText(tool: ToolCallView): string {
  if (tool.output) return tool.output;
  if (typeof tool.metadata?.output === "string") return tool.metadata.output;
  if (!tool.progress) return "";
  try {
    const progress=record(JSON.parse(tool.progress));
    if (typeof progress?.output === "string") return progress.output;
  } catch { /* A runtime may provide its progress directly as plain text. */ }
  return tool.progress;
}

export interface CitedText {
  text: string;
  citations: {end: number; source: {url: string; title: string; description: string}}[];
}
export function structuredCitedText(tool: ToolCallView): CitedText | null {
  const data=record(tool.providerResultState?.output) ?? record(tool.metadata?.output) ?? tool.metadata ?? {};
  if (typeof data.text !== "string" || !Array.isArray(data.citations)) return null;
  const text=data.text;
  const citations=data.citations.flatMap(value => {
    const citation=record(value);
    const source=record(citation?.source);
    if (!citation || !source || !httpURL(source.url) || !Number.isInteger(citation.end) || Number(citation.end) < 0 || Number(citation.end) > text.length) return [];
    return [{end: Number(citation.end), source: {url:source.url, title:typeof source.title === "string" ? source.title : new URL(source.url).hostname, description:typeof source.description === "string" ? source.description : ""}}];
  }).sort((a,b) => a.end-b.end);
  return citations.length ? {text,citations} : null;
}


export type ActivityState = Pick<import("../assistant-status").WorkingSummary,"statusText"|"isWaitingForPermission"|"retryInfo"|"activePartType">;
function liveActivityLabel(last:Extract<TranscriptItem,{kind:"assistant"}>["parts"][number]|undefined,tool:ToolCallView|undefined,activity:ActivityState|null|undefined,retry:Extract<TranscriptItem,{kind:"assistant"}>["retry"]) {
 if(activity?.isWaitingForPermission) return "Waiting for approval";
 if(activity?.retryInfo || (!activity && retry)) return retry?.message || "Retrying request";
 const phrase=activity?.statusText;
 if(phrase==="composing") return "Writing response";
 if(phrase) return phrase[0].toUpperCase()+phrase.slice(1);
 if(tool) return `Running ${tool.title}`;
 if(last?.kind==="reasoning" && !last.complete) return "Thinking";
 if(last?.kind==="text" && !last.complete) return "Writing response";
 return "Working";
}
export function liveActivityPresentation(transcript:TranscriptItem[],activity?:ActivityState|null) {
 const turn:Extract<TranscriptItem,{kind:"assistant"}>[]=[];
 for(let i=transcript.length-1;i>=0;i--) {
  const item=transcript[i];
  if(item.kind==="user") break;
  if(item.kind==="assistant") turn.push(item);
 }
 const parts=turn.flatMap(item=>[...item.parts].reverse());
 const canShowTool=!activity || activity.activePartType==="tool" || activity.activePartType==="editing";
 const part=canShowTool ? parts.find(part=>part.kind==="tool" && part.tool.status==="running") : undefined;
 const tool=part?.kind==="tool" ? part.tool : undefined;
 const input=tool ? toolInput(tool) : {};
 const detail=tool ? [input.command,input.filePath,input.path,input.pattern,input.query,tool.detail].find(value=>typeof value==="string" && value.trim()) as string|undefined : undefined;
 return {label:liveActivityLabel(parts[0],tool,activity,turn[0]?.retry),detail};
}
