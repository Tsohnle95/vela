import { spawn } from "node:child_process";
import { mkdtemp, writeFile, mkdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import electron from "electron";

const output = process.env.VELA_STYLE_REPORT ?? "/private/tmp/vela-ai-style-verification";
await mkdir(output, { recursive: true });
const temporary = await mkdtemp(path.join(os.tmpdir(), "vela-ai-style-"));
const entry = path.join(temporary, "main.cjs");
const baseUrl = process.env.VELA_STYLE_URL ?? "http://127.0.0.1:5280";
const source = String.raw`
const {app,BrowserWindow} = require("electron");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
app.setPath("userData", PROFILE);
app.on("window-all-closed",()=>{});
const results=[];
const windows=[];
const wait = async (win, predicate) => {
 const end=Date.now()+20000;
 while (!(await win.webContents.executeJavaScript(predicate))) {
  if (Date.now()>end) throw new Error("Timed out: "+predicate);
  await new Promise(resolve=>setTimeout(resolve,50));
 }
};
const probe = () => {
 const root=document.querySelector("[data-ai-elements-surface]")?.shadowRoot ?? document;
 const content=root.querySelector("[data-ai-elements-content]");
 const rect=content.getBoundingClientRect();
 const selectors={surface:".ai-elements",content:"[data-ai-elements-content]",timeline:"[data-ai-elements=timeline]",message:"[data-slot=message]",heading:"h2",subheading:"h3",paragraph:"[data-slot=markdown] p",list:"[data-slot=markdown] ul",inlineCode:"[data-slot=markdown] li code",codeBlock:"[data-slot=code-block]",codeContent:"[data-slot=code-block-content]",codeLine:"[data-slot=code-block-line]",trace:"[data-slot=trace-title]",icon:"[data-slot=trace-icon] svg",table:"[data-slot=markdown] table"};
 const fields=["fontFamily","fontSize","fontWeight","lineHeight","letterSpacing","color","backgroundColor","paddingTop","paddingRight","paddingBottom","paddingLeft","marginTop","marginRight","marginBottom","marginLeft","rowGap","columnGap","borderRadius","borderWidth","boxSizing","listStyleType"];
 const styles={};
 const canvas=document.createElement("canvas");canvas.width=canvas.height=1;
 const context=canvas.getContext("2d",{willReadFrequently:true});
 const color=(value)=>{context.clearRect(0,0,1,1);context.fillStyle=value;context.fillRect(0,0,1,1);return Array.from(context.getImageData(0,0,1,1).data).join(",")};
 for (const [name,selector] of Object.entries(selectors)) {
  const element=root.querySelector(selector);
  if(!element) throw new Error("Missing "+name+" "+selector);
  const css=getComputedStyle(element);
  const box=element.getBoundingClientRect();
  styles[name]={...Object.fromEntries(fields.map(field=>[field,field==="color"||field==="backgroundColor"?color(css[field]):css[field]])),width:box.width,height:box.height,x:box.x-rect.x,y:box.y-rect.y};
 }
 const codeScrollers=Array.from(root.querySelectorAll('[data-slot=code-block-content]')).map(element=>({width:element.clientWidth,scrollWidth:element.scrollWidth,wrap:getComputedStyle(element).overflowWrap}));
 return {codeScrollers,checkpoints:root.querySelector("[data-ai-elements=timeline]").textContent.includes("Checkpoint"),styles,width:root.querySelector(".ai-elements").getBoundingClientRect().width,composer:!!document.querySelector(".composer"),suggestions:document.body.textContent.includes("Explain this repository")};
};
const verifyWorkspaceMask = async win => {
 const state = await win.webContents.executeJavaScript("(" + (() => {
  const panel=document.querySelector('.agent-panel');
  const surface=document.querySelector('[data-ai-elements-surface]');
  const shadow=surface.shadowRoot;
  const response=shadow.querySelector('.ai-elements');
  const heading=shadow.querySelector('h2');
  const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const context=canvas.getContext('2d');
  const color=value=>{context.clearRect(0,0,1,1);context.fillStyle=value;context.fillRect(0,0,1,1);return Array.from(context.getImageData(0,0,1,1).data).join(',')};
  const background=color(getComputedStyle(response).backgroundColor);
  const backgroundToken=color(getComputedStyle(response).getPropertyValue('--background').trim());
  const panelBackground=color(getComputedStyle(panel).backgroundColor);
  const workspace=document.createElement('div');workspace.className='workspace-area settings-suspended';workspace.style.cssText='position:absolute;inset:0';workspace.inert=true;
  const column=document.createElement('div');column.className='agent-col';column.style.cssText='left:0;right:0';
  panel.parentElement.append(workspace);column.append(panel);workspace.append(column);
  const settings=document.createElement('section');settings.className='settings-page';
  const exit=document.createElement('button');exit.textContent='Back to workspace';exit.style.cssText='position:absolute;top:80px;right:8px';settings.append(exit);workspace.parentElement.append(settings);
  const box=exit.getBoundingClientRect();
  const hiddenVisibility=getComputedStyle(heading).visibility;
  const hiddenPointerEvents=getComputedStyle(heading).pointerEvents;
  const opacity=getComputedStyle(workspace).opacity;
  const canExit=document.elementFromPoint(box.x+box.width/2,box.y+box.height/2)===exit;
  exit.onclick=()=>{workspace.classList.remove('settings-suspended');workspace.inert=false;settings.remove();};exit.click();
  return {background,backgroundToken,panelBackground,hiddenVisibility,hiddenPointerEvents,opacity,canExit,restoredVisibility:getComputedStyle(heading).visibility,preserved:surface.shadowRoot===shadow && shadow.querySelector('h2')===heading};
 }).toString()+")()");
 console.log("Workspace mask",JSON.stringify(state));
 assert.equal(state.background,"0,0,0,0","Response must let the panel background show through");
 assert.equal(state.backgroundToken,state.panelBackground,"Response background token must match the selected panel palette");
 assert.equal(state.hiddenVisibility,"hidden","Shadow response must inherit suspended visibility");
 assert.equal(state.hiddenPointerEvents,"none","Suspended response cannot receive pointer input");
 assert.equal(state.opacity,"0","Workspace must suppress descendant paint beneath settings");
 assert.equal(state.canExit,true,"Settings exit must receive the hit at its visible location");
 assert.equal(state.restoredVisibility,"visible","Closing settings restores the response");
 assert.equal(state.preserved,true,"Settings must preserve the mounted response and DOM state");
};
const capture = async (version,theme,width,appearance) => {
 const win=new BrowserWindow({show:false,width,height:900,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}});
 windows.push(win);
 console.log("Capture",version,theme,width);
 const errors=[];
 win.webContents.on("console-message",({level,message})=>{if(level==="error") {errors.push(message); console.error(message)}});
 await win.loadURL(BASE_URL+"/chat-comparison/style-check.html?version="+version+"&theme="+theme+(appearance?"&appearance="+appearance:""));
 await wait(win,version==="reference"?"!!document.querySelector('h2')":"!!document.querySelector('[data-ai-elements-surface]')?.shadowRoot?.querySelector('h2')");
 await win.webContents.executeJavaScript("document.fonts.load(" + JSON.stringify('14px "Geist Variable"') + ").then(()=>document.fonts.ready).then(()=>true)");
 await wait(win, "document.fonts.check(" + JSON.stringify('14px "Geist Variable"') + ")");
 await wait(win, "!!(document.querySelector('[data-ai-elements-surface]')?.shadowRoot??document).querySelector('[data-slot=code-block-line-content] span[style]')");
 await new Promise(resolve=>setTimeout(resolve,300));
 const data=await win.webContents.executeJavaScript("(()=>{try{return ("+probe.toString()+")()}catch(error){return {error:error.stack}}})()");
 if(data.error) throw new Error(data.error);
 console.log("Measured",version,theme,width);
 const crop=await win.webContents.executeJavaScript("new Promise(resolve=>{const scroll=document.querySelector('.agent-scroll');if(scroll)scroll.scrollTop=0;requestAnimationFrame(()=>requestAnimationFrame(()=>{const root=document.querySelector('[data-ai-elements-surface]')?.shadowRoot??document;const box=root.querySelector('.ai-elements').getBoundingClientRect();resolve({x:Math.floor(box.x),y:Math.floor(box.y),width:Math.floor(box.width),height:600})}))})");
 const image=await win.webContents.capturePage(crop);
 await fs.writeFile(OUTPUT+"/"+theme+"-"+width+"-"+version+".png",image.toPNG());
 return {win,data,errors};
};
(async()=>{
 await app.whenReady();
 for(const theme of ["dark","light"]) for(const width of [320,600]) {
  const actual=await capture("production",theme,width);
  const reference=await capture("reference",theme,Math.round(actual.data.width));
  assert.deepEqual(actual.errors.concat(reference.errors),[],"No browser errors");
  assert.equal(actual.data.checkpoints,false,"No automatic checkpoint or restore rows");
  assert.ok(actual.data.codeScrollers.length>=3,"Exercise short code, long code and tool output");
  for(const code of actual.data.codeScrollers) assert.ok(code.scrollWidth<=code.width+1,"Code must wrap without horizontal overflow: "+JSON.stringify(code));
  assert.equal(actual.data.composer,true,"Original composer must remain mounted");
  assert.equal(actual.data.suggestions,false,"No production suggested prompts");
  const differences=[];
  for(const [name,styles] of Object.entries(reference.data.styles)) for(const [field,expected] of Object.entries(styles)) {
   const value=actual.data.styles[name][field];
   if(typeof expected==="number" ? Math.abs(value-expected)>.1 : value!==expected) differences.push({element:name,field,expected,actual:value});
  }
  const before=actual.data.styles;
  await actual.win.webContents.executeJavaScript("(()=>{const style=document.createElement('style');style.textContent='h2,p,button,code {font-family:serif!important;font-size:40px!important;color:magenta!important;margin:99px!important;padding:77px!important}';document.head.append(style)})()");
  const isolated=await actual.win.webContents.executeJavaScript("("+probe.toString()+")()");
  assert.deepEqual(isolated.styles,before,"Host CSS must not enter the response tree");
  results.push({theme,width,contentWidth:actual.data.width,differences,errors:actual.errors.concat(reference.errors)});
  await fs.writeFile(OUTPUT+"/results.json",JSON.stringify(results,null,2));
  assert.deepEqual(differences,[],"Production must match reference: "+theme+" "+width);
  await verifyWorkspaceMask(actual.win);
  actual.win.destroy();reference.win.destroy();
 }
 const original = await capture("production","dark",360,"original");
 await verifyWorkspaceMask(original.win);
 original.win.destroy();
 const live = new BrowserWindow({show:false,width:360,height:500,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}});
 await live.loadURL(BASE_URL+"/chat-comparison/style-check.html?stream=1&theme=light");
 await wait(live,"document.documentElement.dataset.streamReady === 'true'");
 const motion = await live.webContents.executeJavaScript("(" + (async () => {
   const samples = [];
   let interpolated = 0;
   let firstWord = null;
   let replacedDuringStreaming = false;
   let firstParagraph = null;
   let paragraphReplaced = false;
   const replacements = [];
   let last = performance.now();
   let previousScroll = 0;
   let scrollFrames = 0;
   let sampling = true;
   let activityFrames=0;
   const labels=new Set();
   const frame = now => {
     samples.push(now-last); last=now;
     const shadow=document.querySelector('[data-ai-elements-surface]')?.shadowRoot;
     for(const word of shadow?.querySelectorAll('[data-sd-animate]')??[]) {
       const opacity=Number(getComputedStyle(word).opacity);
       if(opacity>0 && opacity<1) interpolated++;
       if(!firstWord) {firstWord=word; firstParagraph=word.closest("p");}
     }
     if(firstWord && !firstWord.isConnected) {if(Number(document.documentElement.dataset.streamCursor)<Number(document.documentElement.dataset.streamLength)) replacedDuringStreaming=true;if(!replacements.length) replacements.push({cursor:document.documentElement.dataset.streamCursor,word:firstWord.outerHTML,current:shadow?.querySelector('[data-slot=markdown] p')?.outerHTML});}
     if(firstParagraph && !firstParagraph.isConnected) paragraphReplaced=true;
     const status=shadow?.querySelector('[data-ai-activity]');
     if(status) {activityFrames++;labels.add(status.querySelector('[data-ai-activity-label]')?.textContent);}
     const scroll=document.querySelector('.agent-scroll');
     if(scroll && scroll.scrollTop>previousScroll && scroll.scrollHeight-scroll.clientHeight-scroll.scrollTop>0.5) scrollFrames++;
     previousScroll=scroll?.scrollTop??0;
     if(sampling) requestAnimationFrame(frame);
   };
   requestAnimationFrame(frame);
   for(let index=0;index<100;index++) {
     window.postMessage({kind:'vela-stream-step'},location.origin);
     await new Promise(resolve=>setTimeout(resolve,66));
     if(Number(document.documentElement.dataset.streamCursor)>=Number(document.documentElement.dataset.streamLength)) break;
   }
   await new Promise(resolve=>setTimeout(resolve,350));
   sampling=false;
   const shadow=document.querySelector('[data-ai-elements-surface]').shadowRoot;
   const css=getComputedStyle(shadow.querySelector('[data-slot=markdown] p'));
   const body=getComputedStyle(document.body);
   const scroll=document.querySelector('.agent-scroll');
   return {activityFrames,activityLabels:[...labels],activityAfterCompletion:!!shadow.querySelector("[data-ai-activity]"),interpolated,replacedDuringStreaming,paragraphReplaced,replacements,scrollFrames,frames:samples.length,averageFrameMs:samples.reduce((a,b)=>a+b,0)/samples.length,maxFrameMs:Math.max(...samples),font:css.fontFamily,folderFont:body.fontFamily,size:css.fontSize,folderSize:body.fontSize,weight:css.fontWeight,folderWeight:body.fontWeight,remainingScroll:scroll.scrollHeight-scroll.clientHeight-scroll.scrollTop,text:shadow.textContent};
 }).toString() + ")()");
 await fs.writeFile(OUTPUT+"/streaming.json",JSON.stringify(motion,null,2));
 assert.ok(motion.activityFrames>20,"Activity stays visible while the agent is working");
 assert.ok(motion.activityLabels.some(label=>label?.toLowerCase().includes("command")),"Show actual tool activity");
 assert.ok(motion.activityLabels.includes("Writing response"),"Show text generation phase");
 assert.equal(motion.activityAfterCompletion,false,"Remove live activity when the turn finishes");
 assert.ok(motion.interpolated>10,"New words must visibly interpolate across animation frames");
 assert.equal(motion.replacedDuringStreaming,false,"Previously streamed words must survive later chunks");
 assert.equal(motion.paragraphReplaced,false,"The paragraph must survive streaming and final animation cleanup");
 assert.equal(motion.font,motion.folderFont,"Response and folder font families must match");
 assert.equal(motion.size,motion.folderSize,"Response and folder font sizes must match");
 assert.equal(motion.weight,motion.folderWeight,"Response and folder weights must match");
 assert.ok(motion.scrollFrames>2,"Viewport following must interpolate across frames");
 assert.ok(motion.remainingScroll<=2,"Streaming must remain pinned to the bottom");
 assert.ok(motion.text.includes("The change is ready for review."),"Final streamed text must be complete");
 delete motion.text;
 await fs.writeFile(OUTPUT+"/streaming.json",JSON.stringify(motion,null,2));
 live.destroy();
 console.log("AI Elements style and streaming checks passed:",JSON.stringify(motion));
 app.exit(0);
})().catch(error=>{console.error(error);fs.writeFile(OUTPUT+"/results.json",JSON.stringify(results,null,2)).finally(()=>app.exit(1))});
setTimeout(()=>{console.error("Style smoke overall timeout");app.exit(1)},100000).unref();
`;
await writeFile(entry, source.replaceAll("PROFILE", JSON.stringify(path.join(temporary,"profile"))).replaceAll("OUTPUT",JSON.stringify(output)).replaceAll("BASE_URL",JSON.stringify(baseUrl)));
const child = spawn(electron,[entry],{stdio:"inherit",env:{...process.env,ELECTRON_RUN_AS_NODE:""}});
const code = await new Promise(resolve=>child.on("exit",resolve));
process.exitCode=typeof code === "number"?code:1;
