import { spawn } from "node:child_process";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import electron from "electron";

const temporary = await mkdtemp(path.join(os.tmpdir(), "vela-stream-motion-"));
const output = process.env.VELA_MOTION_REPORT ?? "/private/tmp/vela-stream-motion";
const base = process.env.VELA_STYLE_URL ?? "http://127.0.0.1:5280";
await mkdir(output, { recursive: true });

// Sample after ResizeObserver delivery. rAF alone runs before layout observers
// and can report geometry that is corrected before the frame is painted.
async function capture(mode) {
  const frames = [];
  let recording = true;
  const sample = () => {
    const scroll = document.querySelector(".agent-scroll");
    const shadow = document.querySelector("[data-ai-elements-surface]")?.shadowRoot;
    const statusRoot = document.querySelector("[data-agent-stream-status] [data-ai-elements-surface]")?.shadowRoot ?? shadow;
    const row = statusRoot?.querySelector("[data-ai-activity]");
    const label = row?.querySelector("[data-ai-activity-label]");
    const spinner = row?.querySelector("svg")?.getBoundingClientRect();
    if (label && spinner) frames.push({ time: performance.now(), label: label.textContent, y: label.getBoundingClientRect().top, spinnerY: spinner.y + spinner.height / 2, floor: scroll.scrollHeight - scroll.clientHeight, top: scroll.scrollTop });
    if (recording) requestAnimationFrame(() => setTimeout(sample, 0));
  };
  requestAnimationFrame(() => setTimeout(sample, 0));
  for (let index = 0; index < 100; index++) {
    window.postMessage({ kind: "vela-stream-step" }, location.origin);
    await new Promise(resolve => setTimeout(resolve, 110));
    if (index === 3 && mode === "released") {
      const scroll = document.querySelector(".agent-scroll");
      scroll.dispatchEvent(new WheelEvent("wheel", { deltaY: -100, bubbles: true }));
      scroll.scrollTop = 0;
      scroll.dispatchEvent(new Event("scroll"));
    }
    if (Number(document.documentElement.dataset.streamCursor) >= Number(document.documentElement.dataset.streamLength)) break;
  }
  await new Promise(resolve => setTimeout(resolve, 500));
  recording = false;
  const phases = {};
  for (let index = 1; index < frames.length; index++) {
    const previous = frames[index - 1], current = frames[index];
    if (!["Thinking", "Writing response"].includes(current.label) || previous.label !== current.label || previous.floor <= 5 || current.floor <= 5) continue;
    if (current.time - frames[0].time < 500) continue;
    const phase = phases[current.label] ??= { frames: 0, maxJump: 0, maxSpinnerJump: 0, movedFrames: 0 };
    phase.frames++;
    const delta = Math.abs(current.y - previous.y);
    phase.maxJump = Math.max(phase.maxJump, delta);
    phase.maxSpinnerJump = Math.max(phase.maxSpinnerJump, Math.abs(current.spinnerY - previous.spinnerY));
    if (delta > 0.05) phase.movedFrames++;
  }
  return { mode, phases, frames, viewportVisible: getComputedStyle(document.querySelector(".agent-scroll")).visibility === "visible" };
}

async function captureSubmission() {
  const frames = [];
  let recording = true;
  const snapshot = await window.vela.sessionTranscript();
  let calls = 0;
  window.vela.prompt = async () => {
    calls++;
    await new Promise(resolve => setTimeout(resolve, 800));
    return snapshot;
  };
  const input = document.querySelector("textarea");
  input.focus();
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value").set.call(input, "Read only Enter submission regression");
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await new Promise(resolve => setTimeout(resolve, 80));
  const sample = () => {
    const panel = document.querySelector(".agent-panel").getBoundingClientRect();
    const composer = document.querySelector(".composer").getBoundingClientRect();
    const viewport = document.querySelector(".agent-scroll");
    frames.push({ panelY: panel.y, panelHeight: panel.height, composerBottom: composer.bottom, scroll: viewport.scrollTop });
    if (recording) requestAnimationFrame(() => setTimeout(sample, 0));
  };
  sample();
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", code: "Enter", bubbles: true }));
  await new Promise(resolve => setTimeout(resolve, 1600));
  recording = false;
  const shadow = document.querySelector("[data-ai-elements-surface]").shadowRoot;
  return {
    calls, frames, input: input.value,
    viewportVisible: getComputedStyle(document.querySelector(".agent-scroll")).visibility === "visible",
    toolCalls: shadow.querySelectorAll("[data-tool-call]").length,
    customHeightAnimation: !!shadow.querySelector("[data-transcript-growth]"),
  };
}

const main = String.raw`
const {app,BrowserWindow}=require("electron");
const fs=require("node:fs/promises");
const assert=require("node:assert/strict");
app.setPath("userData",PROFILE);
app.on("window-all-closed",()=>{});
(async()=>{
 await app.whenReady();
 for(const mode of ["following","released"]) {
  const win=new BrowserWindow({show:false,width:320,height:300,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}});
  const errors=[];
  win.webContents.on("console-message",({level,message})=>{if(level==="error") errors.push(message);});
  await win.loadURL(BASE+"/chat-comparison/style-check.html?stream=1&theme=dark");
  for(let attempt=0;attempt<400;attempt++) {
   if(await win.webContents.executeJavaScript("document.documentElement.dataset.streamReady === 'true'")) break;
   if(attempt===399) throw new Error("Replay did not initialize");
   await new Promise(resolve=>setTimeout(resolve,50));
  }
  const result=await win.webContents.executeJavaScript("("+CAPTURE+")("+JSON.stringify(mode)+")");
  await fs.writeFile(OUTPUT+"/"+mode+".json",JSON.stringify(result,null,2));
  await fs.writeFile(OUTPUT+"/"+mode+".png",(await win.webContents.capturePage()).toPNG());
  console.log(JSON.stringify({mode,phases:result.phases}));
  assert.equal(result.viewportVisible,true,"Upstream pending-scroll state settles and reveals the viewport");
  for(const label of ["Thinking","Writing response"]) {
   const phase=result.phases[label];
   assert.ok(phase?.frames>15,"Exercise wrapped lines during "+label);
   const tolerance=1.1;
   assert.ok(phase.maxJump<=tolerance,"Activity label jumps during "+label+": "+phase.maxJump+"px");
   assert.ok(phase.maxSpinnerJump<=tolerance,"Spinner jumps during "+label+": "+phase.maxSpinnerJump+"px");
  }
  assert.deepEqual(errors,[],"No browser errors during streaming");
  if(mode==="released") assert.ok(result.frames.at(-1).top<1,"Scrolling away must remain authoritative");
  win.destroy();
 }
 const submit=new BrowserWindow({show:false,width:360,height:500,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}});
 const errors=[];
 submit.webContents.on("console-message",({level,message})=>{if(level==="error") errors.push(message);});
 await submit.loadURL(BASE+"/chat-comparison/style-check.html");
 for(let attempt=0;attempt<400;attempt++) {
  const ready=await submit.webContents.executeJavaScript("!!window.vela?.sessionTranscript && !!document.querySelector('textarea') && !!document.querySelector('[data-ai-elements-surface]')?.shadowRoot?.querySelector('[data-tool-call]')");
  if(ready) break;
  if(attempt===399) throw new Error("Submission fixture did not initialize: "+JSON.stringify(errors));
  await new Promise(resolve=>setTimeout(resolve,50));
 }
 const submission=await submit.webContents.executeJavaScript("("+SUBMIT+")()");
 await fs.writeFile(OUTPUT+"/submission.json",JSON.stringify(submission,null,2));
 assert.equal(submission.calls,1,"Enter reaches the real Composer/store submission path exactly once");
 assert.equal(submission.viewportVisible,true,"Submission leaves the viewport visible");
 assert.equal(submission.input,"","Submission clears the original composer");
 assert.equal(submission.customHeightAnimation,false,"No custom transcript height animation");
 assert.equal(submission.toolCalls,3,"Each tool remains an individual visible row");
 for(const field of ["panelY","panelHeight","composerBottom"]) {
  const values=submission.frames.map(frame=>frame[field]);
  assert.ok(Math.max(...values)-Math.min(...values)<=1,"Submission moves "+field);
 }
 for(let index=1;index<submission.frames.length;index++) assert.ok(submission.frames[index].scroll>=submission.frames[index-1].scroll-1,"Submission scroll rises then descends");
 assert.deepEqual(errors,[],"No browser errors during submission");
 console.log("Enter submission: stable panel/composer, individual tools, no custom height animation");
 submit.destroy();
 app.exit(0);
})().catch(error=>{console.error(error);app.exit(1)});
`.replace("PROFILE", JSON.stringify(path.join(temporary, "profile"))).replaceAll("BASE", JSON.stringify(base)).replaceAll("OUTPUT", JSON.stringify(output)).replace("CAPTURE", JSON.stringify(capture.toString())).replace("SUBMIT", JSON.stringify(captureSubmission.toString()));
await writeFile(path.join(temporary, "main.cjs"), main);
const child = spawn(electron, [path.join(temporary, "main.cjs")], { stdio: "inherit", env: { ...process.env, ELECTRON_RUN_AS_NODE: undefined } });
child.on("exit", code => process.exit(code ?? 1));
