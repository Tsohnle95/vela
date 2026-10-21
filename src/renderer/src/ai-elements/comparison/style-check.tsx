import { useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { StoreProvider, useStore } from "../../store";
import { ThemeProvider } from "../../theme";
import { AgentPanel } from "../../components/AgentPanel";
import { AiElementsTimeline } from "../AiElementsTimeline";
import { demoSession, installComparisonBridge } from "./bridge";
import { replayEvents, projectReplay } from "./replay";
import type { BackendMessage, TranscriptItem } from "@shared/types";

const params = new URLSearchParams(location.search);
const reference = params.get("version") === "reference";
const dark = params.get("theme") !== "light";
const streaming = params.has("stream");
const historyStreaming = streaming && params.has("history");
const listeners = new Set<(message: BackendMessage) => void>();
const markdown = `Here's the lay of the land:\n\n## Project: Neptune AI Chatbot\n\nA **full-stack AI chatbot** with a vanilla JS frontend and Express backend.\n\n### Directory breakdown\n\n- \`server.js\` — starts the app\n- \`src/app.js\` — configures CORS and JSON parsing\n\n\`\`\`tsx\nconst theme = \"system\";\nexport const enabled = true;\n\`\`\`\n\n| Check | Result |\n| --- | --- |\n| Typecheck | Passed |\n`;
const longCode = `const documentationUrl = "https://example.com/${"unbroken-code-token".repeat(24)}";`;
const events = replayEvents("coding", "Inspect this repository.");
const fixture: TranscriptItem[] = projectReplay(events, events.length, "Inspect this repository.").transcript;
const last = fixture.at(-1);
if (last?.kind === "assistant") last.parts = [...last.parts, {kind:"text",id:"style-markdown",text:markdown+"\n\n```tsx\n"+longCode+"\n```",complete:true}];
fixture.push({kind:"user",id:"follow-up",text:"Show the next step."},{kind:"assistant",id:"follow-up-answer",messageID:"follow-up-answer",completed:true,parts:[{kind:"text",id:"follow-up-text",text:"Ready for the next step.",complete:true}]});
fixture.push({kind:"pending-input",id:"plan-reminder",inputType:"synthetic",text:"<system-reminder>You are in Plan mode. Discuss the plan with the user directly in the conversation.</system-reminder>"});
installComparisonBridge();
window.vela.onMessage = callback => { listeners.add(callback); return () => { listeners.delete(callback); }; };
window.localStorage.setItem("vela.theme", params.get("appearance") ?? (dark ? "codex-dark" : "codex"));
const history: TranscriptItem[] = [
  {kind:"user",id:"previous-user",text:"Explain the previous implementation."},
  {kind:"assistant",id:"previous-answer",messageID:"previous-answer",completed:true,parts:[{kind:"text",id:"previous-text",complete:true,text:Array.from({length:24},(_,index)=>`Paragraph ${index+1}. Existing conversation content must stay visible and clipped inside the viewport while a new response streams.`).join("\n\n")}]}
];
const initialTranscript = streaming ? historyStreaming ? history : [] : fixture;
window.vela.openSessionById = async () => ({ session: demoSession, transcript: initialTranscript, todos: [], usage: null });
window.vela.sessionTranscript = async () => ({ transcript: initialTranscript, todos: [] });

function StyleBody() {
  const store = useStore();
  const started = useRef(false);
  useEffect(() => {
    if (!started.current) {
      started.current = true;
      void store.addModelPanel("/comparison");
    }
  }, [store.addModelPanel]);
  useEffect(() => {
    if (!streaming || !store.session) return;
    const replay = replayEvents(historyStreaming ? "markdown" : "coding", "Inspect this repository.");
    let cursor = 0;
    const receive = (event: MessageEvent) => {
      if (event.source !== window || event.origin !== location.origin || event.data?.kind !== "vela-stream-step") return;
      const next = replay[cursor++];
      if (next) listeners.forEach(listener => listener({kind:"event",type:next.type,data:next}));
      document.documentElement.dataset.streamCursor = String(cursor);
      document.documentElement.dataset.streamLength = String(replay.length);
    };
    window.addEventListener("message", receive);
    document.documentElement.dataset.streamReady = "true";
    return () => window.removeEventListener("message", receive);
  }, [store.session]);
  return reference ? <div className="ai-elements bg-background text-foreground"><div className="flex min-w-0 flex-col gap-6 px-8 py-6" data-ai-elements-content=""><AiElementsTimeline transcript={fixture} busy={false} lastAssistantId="assistant" session={demoSession}/></div></div> : <AgentPanel session={store.session}/>;
}

async function mount() {
  if (reference) {
    await import("../ai-elements.css");
    await import("../../styles/_foundation.scss");
    await import("../app-presentation.css");
  } else {
    await import("../../styles/main.scss");
    await import("@vscode/codicons/dist/codicon.css");
  }
  document.body.style.cssText = "margin:0;height:100vh;overflow:hidden";
  document.getElementById("root")!.style.cssText = reference ? "height:100vh;overflow:hidden" : "height:100vh;overflow:auto";
  if (!reference) {
    const style = document.createElement("style");
    style.textContent = "#root > .agent-panel { height: 100%; }";
    document.head.append(style);
  }
  createRoot(document.getElementById("root")!).render(<StoreProvider><ThemeProvider><StyleBody/></ThemeProvider></StoreProvider>);
}
void mount();
