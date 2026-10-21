import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { clearAgentScrollMemoryForTests, useAgentScroll } from "./agent-scroll";
import type { SessionInfo } from "@shared/types";

const session: SessionInfo = {id:"stream",directory:"/test",workspace:{id:"stream-workspace",generation:1}};
let container: HTMLDivElement;
let root: Root;
let resize: ResizeObserverCallback;
let frames: Map<number, FrameRequestCallback>;
let time: number;
(globalThis as typeof globalThis & {IS_REACT_ACT_ENVIRONMENT:boolean}).IS_REACT_ACT_ENVIRONMENT=true;
function Panel({busy=false}:{busy?:boolean}) {
 const {scrollRef,onScroll}=useAgentScroll(session,[],busy,[],true);
 return <div ref={scrollRef} onScroll={onScroll}><div data-slot="session-turn-list"/></div>;
}
function frame() {
 time+=16;
 const callbacks=[...frames.values()];frames.clear();
 act(()=>callbacks.forEach(callback=>callback(time)));
}
beforeEach(()=>{
 clearAgentScrollMemoryForTests();frames=new Map();time=0;let id=0;
 vi.spyOn(performance,"now").mockReturnValue(0);
 vi.stubGlobal("requestAnimationFrame",(callback:FrameRequestCallback)=>{frames.set(++id,callback);return id;});
 vi.stubGlobal("cancelAnimationFrame",(id:number)=>frames.delete(id));
 vi.stubGlobal("matchMedia",()=>({matches:false}));
 vi.stubGlobal("ResizeObserver",class {constructor(callback:ResizeObserverCallback){resize=callback;}observe(){}disconnect(){}});
 container=document.createElement("div");document.body.append(container);root=createRoot(container);
 act(()=>root.render(<Panel/>));
});
afterEach(()=>{act(()=>root.unmount());container.remove();vi.restoreAllMocks();vi.unstubAllGlobals();});
function viewport() {
 act(()=>root.render(<Panel busy/>));
 const scroll=container.firstElementChild as HTMLDivElement;
 Object.defineProperties(scroll,{clientHeight:{value:100},scrollHeight:{value:1000,configurable:true},scrollTop:{value:800,writable:true}});
 return scroll;
}
it("follows incoming height over frames and continues through its own scroll events",()=>{
 const scroll=viewport();act(()=>resize([],{} as ResizeObserver));
 expect(scroll.scrollTop).toBe(800);
 frame();expect(scroll.scrollTop).toBeGreaterThan(800);expect(scroll.scrollTop).toBeLessThan(900);
 act(()=>scroll.dispatchEvent(new Event("scroll")));
 Object.defineProperty(scroll,"scrollHeight",{value:1040});act(()=>resize([],{} as ResizeObserver));
 for(let n=0;n<30;n++)frame();
 expect(scroll.scrollTop).toBeCloseTo(940,0);
});
it("yields immediately when the reader scrolls away during an animated follow",()=>{
 const scroll=viewport();act(()=>resize([],{} as ResizeObserver));frame();
 scroll.scrollTop=300;act(()=>scroll.dispatchEvent(new Event("scroll")));
 for(let n=0;n<10;n++)frame();
 act(()=>resize([],{} as ResizeObserver));
 expect(scroll.scrollTop).toBe(300);
 expect(frames.size).toBe(0);
});
it("uses immediate following when reduced motion is requested",()=>{
 vi.stubGlobal("matchMedia",()=>({matches:true}));
 const scroll=viewport();act(()=>resize([],{} as ResizeObserver));
 expect(scroll.scrollTop).toBe(1000);
 expect(frames.size).toBe(0);
});
