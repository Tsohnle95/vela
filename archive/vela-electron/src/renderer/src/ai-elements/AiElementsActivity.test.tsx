import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AiElementsActivity, type ActivityState } from "./AiElementsActivity";
import type { TranscriptItem } from "@shared/types";
let container:HTMLDivElement;
let root:Root;
const state:ActivityState={statusText:"preparing response",isWaitingForPermission:false,retryInfo:null,activePartType:undefined};
(globalThis as typeof globalThis & {IS_REACT_ACT_ENVIRONMENT:boolean}).IS_REACT_ACT_ENVIRONMENT=true;
beforeEach(()=>{container=document.createElement("div");document.body.append(container);root=createRoot(container);});
afterEach(()=>{act(()=>root.unmount());container.remove();vi.useRealTimers();});
it("shows activity immediately before the first token, then removes it on completion",()=>{
 act(()=>root.render(<AiElementsActivity transcript={[]} busy activity={state}/>));
 expect(container.querySelector('[role="status"]')?.textContent).toBe("Preparing response");
 expect(container.querySelector(".animate-spin")).not.toBeNull();
 act(()=>root.render(<AiElementsActivity transcript={[]} busy={false} activity={null}/>));
 expect(container.querySelector('[role="status"]')).toBeNull();
});
it("shows the actual running command and updates to writing without stale tool details",()=>{
 const transcript:TranscriptItem[]=[{kind:"assistant",id:"a",messageID:"a",completed:false,parts:[{kind:"tool",id:"t",tool:{id:"t",title:"bash",detail:"npm test",status:"running",input:JSON.stringify({command:"npm test"})}}]}];
 act(()=>root.render(<AiElementsActivity transcript={transcript} busy activity={{...state,activePartType:"tool",statusText:"running command"}}/>));
 expect(container.textContent).toBe("Running commandnpm test");
 act(()=>root.render(<AiElementsActivity transcript={transcript} busy activity={{...state,activePartType:"text",statusText:"composing"}}/>));
 expect(container.textContent).toBe("Writing response");
 expect(container.querySelector('[data-ai-activity-detail]')).toBeNull();
});
it("shows approval and retry phases instead of implying active generation",()=>{
 act(()=>root.render(<AiElementsActivity transcript={[]} busy={false} activity={{...state,isWaitingForPermission:true}}/>));
 expect(container.textContent).toBe("Waiting for approval");
 expect(container.querySelector(".animate-spin")).toBeNull();
 act(()=>root.render(<AiElementsActivity transcript={[]} busy activity={{...state,retryInfo:{attempt:2}}}/>));
 expect(container.textContent).toBe("Retrying request");
});
it("keeps the elapsed indicator moving during a quiet tool call",()=>{
 vi.useFakeTimers();vi.setSystemTime(10000);
 act(()=>root.render(<AiElementsActivity transcript={[]} busy startedAt={5000}/>));
 expect(container.querySelector('[aria-hidden="true"].ai-activity-elapsed')?.textContent).toBe("5s");
 act(()=>vi.advanceTimersByTime(3000));
 expect(container.querySelector('.ai-activity-elapsed')?.textContent).toBe("8s");
});
