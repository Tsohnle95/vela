import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { HighlightRequest, HighlightResult } from "./highlight-protocol";

(globalThis as typeof globalThis & {IS_REACT_ACT_ENVIRONMENT:boolean}).IS_REACT_ACT_ENVIRONMENT=true;
it("keeps every streamed code character visible while highlighting runs off-thread",async()=>{
 const requests:HighlightRequest[]=[];
 let receive:(event:{data:HighlightResult})=>void;
 vi.stubGlobal("Worker",class {
  set onmessage(callback:typeof receive){receive=callback;}
  postMessage(request:HighlightRequest){requests.push(request);}
  terminate(){}
 });
 const {StreamingMarkdown}=await import("./StreamingMarkdown");
 const container=document.createElement("div");document.body.append(container);const root=createRoot(container);
 try {
  const render=async(code:string)=>{await act(async()=>root.render(<StreamingMarkdown isAnimating>{'```tsx\n'+code+'\n```'}</StreamingMarkdown>));};
  await render("const x = 1;");
  const request=requests.find((item):item is Extract<HighlightRequest,{type:"highlight"}>=>item.type==="highlight")!;
  expect(request.code).toBe("const x = 1;");
  await act(async()=>receive({data:{...request,lines:[[{text:request.code,style:{color:"red"}}]]}}));
  expect(container.querySelector('[data-slot="code-block-line"]')?.textContent).toBe("const x = 1;");
  await render("const x = 1; // still streaming");
  expect(container.querySelector('[data-slot="code-block-line"]')?.textContent).toBe("const x = 1; // still streaming");
  expect(container.querySelector('[data-slot="code-block-line"] [style]')?.getAttribute("style")).toContain("red");
  await render("const replacement = true;");
  expect(container.querySelector('[data-slot="code-block-line"]')?.textContent).toBe("const replacement = true;");
  expect(container.querySelector('[data-slot="code-block-line"] [style]')).toBeNull();
 } finally {act(()=>root.unmount());container.remove();vi.unstubAllGlobals();}
 expect(requests.at(-1)?.type).toBe("dispose");
});
