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
 return {styles,width:root.querySelector(".ai-elements").getBoundingClientRect().width,composer:!!document.querySelector(".composer"),suggestions:document.body.textContent.includes("Explain this repository")};
};
const capture = async (version,theme,width) => {
 const win=new BrowserWindow({show:false,width,height:900,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false}});
 windows.push(win);
 console.log("Capture",version,theme,width);
 const errors=[];
 win.webContents.on("console-message",({level,message})=>{if(level==="error") {errors.push(message); console.error(message)}});
 await win.loadURL(BASE_URL+"/chat-comparison/style-check.html?version="+version+"&theme="+theme);
 await wait(win,version==="reference"?"!!document.querySelector('h2')":"!!document.querySelector('[data-ai-elements-surface]')?.shadowRoot?.querySelector('h2')");
 await win.webContents.executeJavaScript("document.fonts.load(" + JSON.stringify('14px "Geist Variable"') + ").then(()=>document.fonts.ready).then(()=>true)");
 await wait(win, "document.fonts.check(" + JSON.stringify('14px "Geist Variable"') + ")");
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
 for(const theme of ["light","dark"]) for(const width of [320,600]) {
  const actual=await capture("production",theme,width);
  const reference=await capture("reference",theme,Math.round(actual.data.width));
  assert.deepEqual(actual.errors.concat(reference.errors),[],"No browser errors");
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
  actual.win.destroy();reference.win.destroy();
 }
 console.log("AI Elements style parity passed: light/dark at 320/600px, 15 element types, computed typography/color/spacing/geometry, host CSS isolation.");
 app.exit(0);
})().catch(error=>{console.error(error);fs.writeFile(OUTPUT+"/results.json",JSON.stringify(results,null,2)).finally(()=>app.exit(1))});
setTimeout(()=>{console.error("Style smoke overall timeout");app.exit(1)},100000).unref();
`;
await writeFile(entry, source.replaceAll("PROFILE", JSON.stringify(path.join(temporary,"profile"))).replaceAll("OUTPUT",JSON.stringify(output)).replaceAll("BASE_URL",JSON.stringify(baseUrl)));
const child = spawn(electron,[entry],{stdio:"inherit",env:{...process.env,ELECTRON_RUN_AS_NODE:""}});
const code = await new Promise(resolve=>child.on("exit",resolve));
process.exitCode=typeof code === "number"?code:1;
