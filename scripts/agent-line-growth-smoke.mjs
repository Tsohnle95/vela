import { spawn } from "node:child_process";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import electron from "electron";

const temporary = await mkdtemp(path.join(os.tmpdir(), "vela-line-growth-"));
const output = process.env.VELA_MOTION_REPORT ?? "/private/tmp/vela-line-growth";
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
    const row = shadow?.querySelector("[data-ai-activity]");
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
      scroll.dispatchEvent(new WheelEvent("wheel", { deltaY: -100 }));
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
  return { mode, phases, frames };
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
  for(const label of ["Thinking","Writing response"]) {
   const phase=result.phases[label];
   assert.ok(phase?.frames>15,"Exercise wrapped lines during "+label);
   const tolerance=mode==="following"?1.1:4;
   assert.ok(phase.maxJump<=tolerance,"Activity label jumps during "+label+": "+phase.maxJump+"px");
   assert.ok(phase.maxSpinnerJump<=tolerance,"Spinner jumps during "+label+": "+phase.maxSpinnerJump+"px");
  }
  assert.deepEqual(errors,[],"No browser errors during line growth");
  win.destroy();
 }
 app.exit(0);
})().catch(error=>{console.error(error);app.exit(1)});
`.replace("PROFILE", JSON.stringify(path.join(temporary, "profile"))).replace("BASE", JSON.stringify(base)).replaceAll("OUTPUT", JSON.stringify(output)).replace("CAPTURE", JSON.stringify(capture.toString()));
await writeFile(path.join(temporary, "main.cjs"), main);
const child = spawn(electron, [path.join(temporary, "main.cjs")], { stdio: "inherit", env: { ...process.env, ELECTRON_RUN_AS_NODE: undefined } });
child.on("exit", code => process.exit(code ?? 1));
