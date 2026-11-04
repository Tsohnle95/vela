import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider, useTheme } from "../../theme";
import { StoreProvider } from "../../store";
import { ErrorBoundary } from "../../components/ErrorBoundary";
import { demoSession, installComparisonBridge } from "./bridge";
import { projectReplay, replayEvents, type Scenario } from "./replay";
import type { TranscriptItem } from "@shared/types";
import type { ComponentType } from "react";
import type { AiElementsTimelineProps } from "../AiElementsTimeline";
import { useAgentScroll } from "../../components/agent-scroll";

const params = new URLSearchParams(location.search);
const version = params.get("version") ?? "old";
installComparisonBridge();
type ReplayConfig = {kind:"vela-replay";cursor:number;scenario:Scenario;prompt:string;dark:boolean};
function ComparisonAppearance({dark}: {dark:boolean}) {
  const {setTheme} = useTheme();
  useEffect(() => setTheme(dark ? "codex-dark" : "codex"),[dark,setTheme]);
  return null;
}
function OldSurface({Timeline,transcript,busy}: {Timeline:ComponentType<AiElementsTimelineProps & {onDemoPrompt?: (text:string) => void}>;transcript:TranscriptItem[];busy:boolean}) {
  const {scrollRef,onScroll} = useAgentScroll(demoSession,transcript,busy,[]);
  return <div className="agent-panel" style={{height:"100vh",width:"100%",minWidth:0,background:"var(--bg-primary)"}}><div className="agent-scroll" ref={scrollRef} onScroll={onScroll}><Timeline transcript={transcript} busy={busy} lastAssistantId="assistant" session={null}/></div></div>;
}
async function mount() {
  if (version === "old") {
    await import("../../styles/main.scss");
    await import("@vscode/codicons/dist/codicon.css");
  } else {
    await import("../ai-elements.css");
    await import("./reference-theme.css");
  }
  document.body.style.margin = "0";
  document.body.style.height = "100vh";
  const Timeline = version === "old" ? (await import("../../components/OpenCodeTimeline")).OpenCodeTimeline : (await import("../AiElementsChat")).AiElementsChat;
  function Frame() {
    const [config,setConfig] = useState<ReplayConfig>({kind:"vela-replay",cursor:0,scenario:"coding",prompt:"Add a dark mode toggle to the settings page.",dark:true});
    useEffect(() => {
      const receive = (event:MessageEvent) => {if (event.origin === location.origin && event.source === parent && event.data?.kind === "vela-replay") setConfig(event.data);};
      window.addEventListener("message",receive);
      parent.postMessage({kind:"vela-frame-ready",version},location.origin);
      return () => window.removeEventListener("message",receive);
    },[]);
    useEffect(() => {
      document.documentElement.classList.toggle("dark",config.dark);
      document.documentElement.style.colorScheme = config.dark ? "dark" : "light";
      document.documentElement.dataset.theme = config.dark ? "dark" : "light";
    },[config.dark]);
    const replay = useMemo(() => projectReplay(replayEvents(config.scenario,config.prompt),config.cursor,config.prompt),[config.scenario,config.cursor,config.prompt]);
    useEffect(() => {parent.postMessage({kind:"vela-frame-rendered",version,cursor:config.cursor,projection:JSON.stringify(replay.transcript)},location.origin);},[replay,config.cursor]);
    return <StoreProvider><ThemeProvider><ComparisonAppearance dark={config.dark}/><ErrorBoundary>{version === "old" ? <OldSurface Timeline={Timeline} {...replay}/> : <div className="ai-elements h-dvh bg-background text-foreground"><Timeline {...replay} lastAssistantId="assistant" session={null} onDemoPrompt={text => parent.postMessage({kind:"vela-demo-prompt",text},location.origin)}/></div>}</ErrorBoundary></ThemeProvider></StoreProvider>;
  }
  createRoot(document.getElementById("root")!).render(<Frame/>);
}
void mount();
