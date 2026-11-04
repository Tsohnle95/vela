import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { ErrorBoundary } from "../../components/ErrorBoundary";
import { replayEvents, type Scenario } from "./replay";
import "./comparison.css";
const Gallery = lazy(() => import("./gallery"));

function Comparison() {
  const oldFrame = useRef<HTMLIFrameElement>(null);
  const newFrame = useRef<HTMLIFrameElement>(null);
  const [mode,setMode] = useState<"compare"|"gallery">("compare");
  const [cursor,setCursor] = useState(0);
  const [playing,setPlaying] = useState(true);
  const [speed,setSpeed] = useState(1);
  const [scenario,setScenario] = useState<Scenario>("coding");
  const [prompt,setPrompt] = useState("Add a dark mode toggle to the settings page.");
  const [draft,setDraft] = useState(prompt);
  const [dark,setDark] = useState(true);
  const [ready,setReady] = useState(0);
  const [acks,setAcks] = useState<Record<string,{cursor:number;projection:string}>>({});
  const total = useMemo(() => replayEvents(scenario,prompt).length,[scenario,prompt]);
  useEffect(() => {
    const handler = (event:MessageEvent) => {
      if (event.origin !== location.origin || ![oldFrame.current?.contentWindow,newFrame.current?.contentWindow].includes(event.source as Window)) return;
      if (event.data?.kind === "vela-demo-prompt" && typeof event.data.text === "string") {setPrompt(event.data.text);setDraft(event.data.text);setCursor(0);setPlaying(true);}
      if (event.data?.kind === "vela-frame-ready") setReady(value => value + 1);
      if (event.data?.kind === "vela-frame-rendered") setAcks(value => ({...value,[event.data.version]:event.data}));
    };
    window.addEventListener("message",handler);
    return () => window.removeEventListener("message",handler);
  },[]);
  useEffect(() => {
    const state = {kind:"vela-replay",cursor,scenario,prompt,dark};
    for (const frame of [oldFrame,newFrame]) frame.current?.contentWindow?.postMessage(state,location.origin);
  },[cursor,scenario,prompt,dark,ready,mode]);
  useEffect(() => {
    if (!playing || mode !== "compare" || ready < 2) return;
    const timer = window.setInterval(() => setCursor(value => Math.min(total,value+1)),100/speed);
    return () => clearInterval(timer);
  },[playing,total,speed,mode,ready]);
  useEffect(() => {if (cursor === total) setPlaying(false);},[cursor,total]);
  const restart = () => {setCursor(0);setPlaying(true);};
  const synchronized = acks.old?.cursor === cursor && acks.new?.cursor === cursor && acks.old?.projection === acks.new?.projection;
  return <main className={dark ? "comparison-shell dark" : "comparison-shell"}>
    <header className="comparison-header"><div><span className="eyebrow">VELA · RENDERER LAB</span><h1>The same stream. Two renderers.</h1><p>Existing Vela renderer alongside the new shadcn AI Elements renderer.</p></div><div className="header-actions"><button onClick={() => setDark(value => !value)}>{dark ? "Light" : "Dark"} appearance</button><a href="https://ui.shadcn.com/docs/ai-elements" target="_blank" rel="noreferrer">Upstream reference ↗</a></div></header>
    <nav className="comparison-tabs" aria-label="Comparison views"><button aria-pressed={mode === "compare"} onClick={() => setMode("compare")}>Side-by-side stream</button><button aria-pressed={mode === "gallery"} onClick={() => setMode("gallery")}>All 21 original patterns</button><span>Official source · Base UI · Rhea</span></nav>
    <div style={{display:mode === "compare" ? "contents":"none"}}>
      <section className="replay-toolbar" aria-label="Shared replay controls"><label>Scenario<select value={scenario} onChange={event => {setScenario(event.target.value as Scenario);restart();}}><option value="coding">Coding · reasoning + tools + prose</option><option value="markdown">Markdown · code + tables + lists</option><option value="failure">Failure · captured command error</option><option value="rich">Rich results · plans, sources, tests + files</option></select></label><button onClick={() => setPlaying(value => !value)}>{playing ? "Pause":"Play"}</button><button onClick={restart}>Replay</button><button onClick={() => {setPlaying(false);setCursor(value => Math.min(total,value+1));}}>Step</button><label>Speed<select value={speed} onChange={event => setSpeed(Number(event.target.value))}><option value={0.5}>0.5×</option><option value={1}>1×</option><option value={2}>2×</option><option value={4}>4×</option></select></label><input aria-label="Replay position" type="range" min={0} max={total} value={cursor} onChange={event => {setPlaying(false);setCursor(Number(event.target.value));}}/><output>{cursor} / {total}</output><span className="sync-status">{synchronized ? "● Synchronized":"○ Loading"}</span></section>
      <section className="renderer-grid"><article className="renderer-pane"><header><div><span className="version-label">BEFORE</span><h2>Existing Vela</h2></div><span>Current renderer + Markdown worker</span></header><iframe title="Existing Vela chat renderer" ref={oldFrame} src="./frame.html?version=old"/></article><article className="renderer-pane"><header><div><span className="version-label">AFTER</span><h2>shadcn AI Elements</h2></div><span>Upstream primitives + Streamdown</span></header><iframe title="New AI Elements chat renderer" ref={newFrame} src="./frame.html?version=new"/></article></section>
      <form className="shared-prompt" onSubmit={event => {event.preventDefault();setPrompt(draft.trim() || prompt);restart();}}><label htmlFor="prompt">Shared prompt</label><input id="prompt" value={draft} onChange={event => setDraft(event.target.value)}/><button>Run comparison</button></form><footer className="comparison-note">Deterministic OpenCode event replay, using Vela’s actual reducer and both renderer implementations. No model request or workspace edits. Use the pattern gallery for all 21 original interactive examples.</footer>
    </div>
    {mode === "gallery" && <ErrorBoundary><Suspense fallback={<p className="gallery-loading">Loading original examples…</p>}><Gallery dark={dark}/></Suspense></ErrorBoundary>}
  </main>;
}
createRoot(document.getElementById("root")!).render(<Comparison/>);
