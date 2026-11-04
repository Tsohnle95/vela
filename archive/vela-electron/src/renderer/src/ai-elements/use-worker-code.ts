import { useEffect, useId, useState } from "react";
import type { HighlightRequest, HighlightResult } from "./highlight-protocol";

let worker: Worker | undefined;
let failed = false;
let sequence = 0;
const listeners = new Map<string,(result:HighlightResult)=>void>();
function send(request:HighlightRequest) {
 if(failed || typeof Worker === "undefined") return;
 try {
  if(!worker) {
   worker = new Worker(new URL("./highlight.worker.ts",import.meta.url),{type:"module"});
   worker.onmessage = (event:MessageEvent<HighlightResult>) => listeners.get(event.data.key)?.(event.data);
   worker.onerror = () => {failed=true;worker?.terminate();worker=undefined;};
  }
  worker.postMessage(request);
 } catch {failed=true;worker?.terminate();worker=undefined;}
}
export function useWorkerCode(code:string, language:string) {
 const key=useId();
 const [result,setResult]=useState<HighlightResult>();
 useEffect(()=>{
  listeners.set(key,setResult);
  return ()=>{listeners.delete(key);send({type:"dispose",key});};
 },[key]);
 useEffect(()=>{send({type:"highlight",key,id:++sequence,code,language});},[key,code,language]);
 return result?.language===language && code.startsWith(result.code) ? result : undefined;
}
