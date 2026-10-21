import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, TranscriptItem } from "@shared/types";
const store = vi.hoisted(() => ({ sessions: [], panelViews: {}, replyPermission: vi.fn(), stageRevert: vi.fn(), notify: vi.fn(), openFile: vi.fn(), reopenSession: vi.fn() }));
vi.mock("../store", () => ({ useStore: () => store }));
vi.mock("./demos/lib/markdown", () => ({ Markdown: ({ children, isAnimating }: {
        children: string;
        isAnimating: boolean;
    }) => <div data-testid="markdown" data-streaming={String(isAnimating)}>{children}</div> }));
vi.mock("../components/TimelineProse", () => ({ writeClipboard: vi.fn().mockResolvedValue(undefined) }));
vi.mock("@/components/ui/message-scroller", () => ({MessageScrollerItem: ({children, messageId, scrollAnchor}: {children:ReactNode;messageId:string;scrollAnchor:boolean}) => <div data-scroller-item={messageId} data-anchor={String(scrollAnchor)}>{children}</div>}));
import { AiElementsPermission, AiElementsTranscript, AiElementsTool } from "./AiElementsTimeline";
import { ReasoningRow } from "./patterns";
const session = { id: "session-b", directory: "/tmp/b", workspace: { id: "workspace-b", generation: 1 } } satisfies SessionInfo;
let container: HTMLDivElement;
let root: Root;
(globalThis as typeof globalThis & {
    IS_REACT_ACT_ENVIRONMENT: boolean;
}).IS_REACT_ACT_ENVIRONMENT = true;
beforeEach(() => { vi.clearAllMocks(); container = document.createElement("div"); document.body.append(container); root = createRoot(container); });
afterEach(() => { act(() => root.unmount()); container.remove(); });
function render(node: ReactNode) { act(() => root.render(node)); return { rerender: (next: ReactNode) => act(() => root.render(next)) }; }
const screen = {
    getByRole: (_role: string, { name }: {
        name: string;
    }) => { const button = Array.from(container.querySelectorAll("button")).find(button => button.getAttribute("aria-label") === name || button.textContent === name); if (!button)
        throw new Error(`No button ${name}`); return button; },
    getByTestId: (id: string) => container.querySelector<HTMLElement>(`[data-testid="${id}"]`)!
};
const fireEvent = { click: (target: HTMLElement) => act(() => target.click()) };
describe("AI Elements runtime bindings", () => {
    it("routes an approval to the transcript's session rather than the focused session", () => {
        render(<AiElementsPermission session={session} item={{ kind: "permission", id: "p", requestID: "request", action: "Run command", resources: ["npm test"], pending: true }}/>);
        fireEvent.click(screen.getByRole("button", { name: "Approve" }));
        expect(store.replyPermission).toHaveBeenCalledWith("request", "once", "session-b");
    });
    it("prevents an unowned approval from falling back to the focused session", () => {
        render(<AiElementsPermission session={null} item={{ kind: "permission", id: "p", requestID: "request", action: "Run command", resources: [], pending: true }}/>);
        fireEvent.click(screen.getByRole("button", { name: "Approve" }));
        expect(store.replyPermission).not.toHaveBeenCalled();
    });
    it("retains message-owned markdown streaming until assistant completion", () => {
        const transcript: TranscriptItem[] = [{ kind: "assistant", id: "a", messageID: "a", parts: [{ kind: "text", id: "text", text: "hello", complete: true }], completed: false }];
        const view = render(<AiElementsTranscript transcript={transcript} busy lastAssistantId="a" session={session}/>);
        expect(screen.getByTestId("markdown").dataset.streaming).toBe("true");
        view.rerender(<AiElementsTranscript transcript={[{ ...transcript[0], completed: true } as TranscriptItem]} busy={false} lastAssistantId="a" session={session}/>);
        expect(screen.getByTestId("markdown").dataset.streaming).toBe("false");
    });
    it("uses the existing runtime-backed stage-revert flow for Edit", () => {
        render(<AiElementsTranscript transcript={[{ kind: "user", id: "u", text: "change this" }]} busy={false} lastAssistantId={null} session={session}/>);
        fireEvent.click(screen.getByRole("button", { name: "Edit" }));
        expect(store.stageRevert).toHaveBeenCalledWith(session.workspace, "u");
    });
    it("opens reasoning during streaming and respects a manual disclosure after completion", async () => {
        const view = render(<ReasoningRow text="Actual reasoning" running/>);
        const trigger = screen.getByRole("button", { name: "Thinking" });
        expect(trigger.getAttribute("aria-expanded")).toBe("true");
        fireEvent.click(trigger);
        view.rerender(<ReasoningRow text="Actual reasoning completed" running={false}/>);
        expect(screen.getByRole("button", { name: "Thought" }).getAttribute("aria-expanded")).toBe("false");
    });
});


describe("AI Elements streaming chronology", () => {
 it("anchors user messages with stable IDs only when rendered inside the chat scroller", () => {
  const transcript: TranscriptItem[]=[{kind:"user",id:"u",text:"request"},{kind:"assistant",id:"a",messageID:"a",completed:true,parts:[{kind:"text",id:"p",text:"reply",complete:true}]}];
  const view=render(<AiElementsTranscript transcript={transcript} busy={false} lastAssistantId="a" session={session} />);
  expect(container.querySelector("[data-scroller-item]")).toBeNull();
  view.rerender(<AiElementsTranscript transcript={transcript} busy={false} lastAssistantId="a" session={session} scrollerItems />);
  expect(container.querySelector('[data-scroller-item="u"]')?.getAttribute("data-anchor")).toBe("true");
  expect(container.querySelector('[data-scroller-item="a"]')?.getAttribute("data-anchor")).toBe("false");
 });
 it("does not invent a failed tool when the user interrupted a pending operation", () => {
  render(<AiElementsTranscript transcript={[{kind:"assistant",id:"a",messageID:"a",completed:true,interrupted:true,parts:[{kind:"tool",id:"t",tool:{id:"t",title:"shell",detail:"npm test",status:"running"}}]}]} busy={false} lastAssistantId="a" session={session} />);
  expect(container.textContent).toContain("Stopped npm test");
  expect(container.querySelector('[data-slot="trace-line"]')?.getAttribute("data-status")).not.toBe("error");
 });
 it("keeps an earlier non-completed tool running when another assistant message arrives", () => {
  render(<AiElementsTranscript transcript={[{kind:"assistant",id:"first",messageID:"first",completed:false,parts:[{kind:"tool",id:"t",tool:{id:"t",title:"shell",detail:"npm test",status:"running"}}]},{kind:"assistant",id:"last",messageID:"last",completed:false,parts:[]}]} busy lastAssistantId="last" session={session} />);
  expect(container.textContent).toContain("Running npm test");
  expect(container.querySelector('[data-slot="trace-line"]')?.getAttribute("data-status")).toBe("running");
 });
});


it("opens the actual child session from the new subagent affordance", () => {
 render(<AiElementsTool tool={{id:"t",title:"task",detail:"Inspect source",status:"success",input:JSON.stringify({agent:"research"}),metadata:{sessionID:"child-session"}}} session={session} streaming={false} />);
 fireEvent.click(screen.getByRole("button",{name:"Open agent"}));
 expect(store.reopenSession).toHaveBeenCalledWith("child-session");
});


import { replayEvents, projectReplay } from "./comparison/replay";
import { applyChatEvent, projectAssistantItems, type ChatDirectoryState } from "../chat-store";
import type { ChatStreamEvent } from "../chat-stream";
function projectedTool(name:string,input:Record<string,unknown>,metadata:Record<string,unknown>): TranscriptItem[] {
 const state: ChatDirectoryState={message:{},part:{},session_status:{}};
 const events: Array<[string,Record<string,unknown>]>=[
 ["session.step.started",{}],
 ["session.tool.called",{callID:"tool",name,input}],
 ["session.tool.success",{callID:"tool",output:"Actual output",metadata}],
 ["session.step.ended",{finish:"stop"}]
 ];
 events.forEach(([type,data],index) => applyChatEvent(state,session.id,{id:String(index),type,created:index+1,data:{sessionID:session.id,assistantMessageID:"assistant",...data}} as ChatStreamEvent));
 return projectAssistantItems(state,session.id);
}
describe("AI Elements actual event projection integration", () => {
 it("shows the real shell command and partial output through applyChatEvent/projectAssistantItems", () => {
  const events=replayEvents("coding","update settings");
  const progress=events.findIndex(event => event.type === "session.tool.progress" && event.data.callID === "terminal");
  const replay=projectReplay(events,progress+1,"update settings");
  render(<AiElementsTranscript {...replay} lastAssistantId="assistant" session={session} />);
  expect(container.textContent).toContain("Running npm run test -- settings");
  expect(container.textContent).toContain("RUN v3.2.7");
  expect(container.textContent).not.toContain('{"output":');
  expect(container.textContent).toContain("Researching");
 });
 it("exposes changed files from actual edit metadata and opens the file in the owning workspace", () => {
  const transcript=projectedTool("edit",{filePath:"src/settings.tsx",oldString:"old",newString:"new"},{files:[{relativePath:"src/settings.tsx",before:"old",after:"new"}]});
  render(<AiElementsTranscript transcript={transcript} busy={false} lastAssistantId="assistant" session={session} />);
  fireEvent.click(screen.getByRole("button",{name:"Ran src/settings.tsx"}));
  fireEvent.click(screen.getByRole("button",{name:"settings.tsx"}));
  expect(store.openFile).toHaveBeenCalledWith("src/settings.tsx",session.workspace);
 });
 it("renders provider supplied inline citation associations without inventing sources from prose", () => {
  const text="React streams content.";
  const transcript=projectedTool("research",{query:"React streaming"},{output:{text,citations:[{end:text.length,source:{url:"https://react.dev/reference/react/Suspense",title:"Suspense",description:"React Suspense reference"}}]}});
  render(<AiElementsTranscript transcript={transcript} busy={false} lastAssistantId="assistant" session={session} />);
  expect(container.textContent).toContain(text);
  expect(screen.getByRole("button",{name:"react.dev"})).toBeTruthy();
 });
});


it("omits empty Copy controls while retaining an available retry action", () => {
 const transcript: TranscriptItem[] = [{kind:"assistant",id:"a",messageID:"a",completed:true,parts:[],error:"Request failed"}];
 const retry = vi.fn();
 const view=render(<AiElementsTranscript transcript={transcript} busy={false} lastAssistantId="a" session={session}/>);
 expect(container.querySelector('[aria-label="Copy"]')).toBeNull();
 expect(container.querySelector('[data-slot="message-footer"]')).toBeNull();
 view.rerender(<AiElementsTranscript transcript={transcript} busy={false} lastAssistantId="a" session={session} onRetry={retry}/>);
 fireEvent.click(screen.getByRole("button",{name:"Retry"}));
 expect(retry).toHaveBeenCalledWith("a");
 expect(container.querySelector('[aria-label="Copy"]')).toBeNull();
});

it("retains an expanded tool's DOM and state when the next streamed tool arrives", () => {
 const tool = {kind:"tool" as const,id:"one",tool:{id:"one",title:"bash",detail:"npm test",status:"running" as const,output:"first output"}};
 const item = {kind:"assistant" as const,id:"a",messageID:"a",completed:false,parts:[tool]};
 const view=render(<AiElementsTranscript transcript={[item]} busy lastAssistantId="a" session={session}/>);
 const original=Array.from(container.querySelectorAll('[data-slot="trace-title"]')).find(node=>node.textContent?.includes("npm test"))!;
 view.rerender(<AiElementsTranscript transcript={[{...item,parts:[tool,{...tool,id:"two",tool:{...tool.tool,id:"two",detail:"npm run build"}}]}]} busy lastAssistantId="a" session={session}/>);
 expect(Array.from(container.querySelectorAll('[data-slot="trace-title"]')).find(node=>node.textContent?.includes("npm test"))).toBe(original);
});

it("does not insert checkpoint or restore controls between turns",()=>{
 render(<AiElementsTranscript transcript={[{kind:"assistant",id:"a",messageID:"a",completed:true,parts:[{kind:"text",id:"p",text:"Done",complete:true}]},{kind:"user",id:"u",text:"Next"}]} busy={false} lastAssistantId="a" session={session}/>);
 expect(container.textContent).not.toContain("Checkpoint");
 expect(container.querySelector('[aria-label="Restore to Checkpoint"]')).toBeNull();
});
