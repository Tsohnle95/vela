import { useEffect, useMemo, useState } from "react";
import { CircleDashedIcon, LoaderCircleIcon } from "lucide-react";
import type { TranscriptItem } from "@shared/types";
import { liveActivityPresentation, type ActivityState } from "./adapters";
export type { ActivityState } from "./adapters";

export function AiElementsActivity({busy,transcript,activity,startedAt}:{busy:boolean;transcript:TranscriptItem[];activity?:ActivityState|null;startedAt?:number|null}) {
 const [now,setNow]=useState(Date.now);
 useEffect(()=>{
  if(!busy) return;
  setNow(Date.now());
  const timer=setInterval(()=>setNow(Date.now()),1000);
  return ()=>clearInterval(timer);
 },[busy,startedAt]);
 const {label,detail}=useMemo(()=>liveActivityPresentation(transcript,activity),[transcript,activity]);
 if(!busy && !activity?.isWaitingForPermission) return null;
 const waiting=activity?.isWaitingForPermission;
 const elapsed=startedAt ? Math.max(0,Math.floor((now-startedAt)/1000)) : null;
 const Icon=waiting ? CircleDashedIcon : LoaderCircleIcon;
 return <div className="ai-live-activity" role="status" aria-live="polite" aria-atomic="true" data-ai-activity="">
  <Icon aria-hidden="true" className={waiting?"":"animate-spin"}/>
  <div className="min-w-0"><span data-ai-activity-label="">{label}</span>{detail && <div data-ai-activity-detail="">{detail}</div>}</div>
  {elapsed!==null && busy && <span className="ai-activity-elapsed" aria-hidden="true">{elapsed}s</span>}
 </div>;
}
