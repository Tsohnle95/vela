import { spawn } from "node:child_process";
import { mkdtemp, writeFile, mkdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import electron from "electron";

const output = process.env.VELA_COMPACT_REPORT ?? "/private/tmp/vela-compact-designs";
await mkdir(output, { recursive: true });
const temporary = await mkdtemp(path.join(os.tmpdir(), "vela-compact-gallery-"));
const entry = path.join(temporary, "main.cjs");
const source = String.raw`
const { app, BrowserWindow } = require("electron");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
app.setPath("userData", PROFILE);
const watchdog = setTimeout(()=>app.exit(1), 45000);
(async()=>{
 await app.whenReady();
 const win = new BrowserWindow({show:false,width:1280,height:1000,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}});
 const errors=[];
 win.webContents.on("console-message",({level,message})=>{if(level==="error")errors.push(message)});
 await win.loadFile(FIXTURE);
 const settle=()=>win.webContents.executeJavaScript("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
 const results=[];
 for(const theme of ["dark","light"]) for(const width of [320,360]) for(const expanded of [false,true]) {
  const measurements=await win.webContents.executeJavaScript("("+((theme,width,expanded)=>{
   document.documentElement.dataset.theme=theme;
   let style=document.querySelector('#measurement-style');
   if(!style){style=document.createElement('style');style.id='measurement-style';document.head.append(style)}
   style.textContent='.catalog{grid-template-columns:repeat(3,'+width+'px)}.catalog .agent{width:'+width+'px;height:350px}.catalog .concept{width:'+width+'px}';
   const visible=el=>{const css=getComputedStyle(el);const box=el.getBoundingClientRect();return css.display!=='none'&&css.visibility!=='hidden'&&box.width>0&&box.height>0};
   return [...document.querySelectorAll('.catalog .agent')].map(agent=>{
    agent.classList.toggle('expanded',expanded);
    const box=agent.getBoundingClientRect();
    const overflow=[];
    for(const el of agent.querySelectorAll('input,textarea,button,select,.input-zone,.reveal')) {
     if(!visible(el))continue;
     const r=el.getBoundingClientRect();
     if(r.left<box.left-1||r.right>box.right+1||r.top<box.top-1||r.bottom>box.bottom+1)overflow.push({element:el.tagName,classes:el.className,left:r.left-box.left,right:r.right-box.left,top:r.top-box.top,bottom:r.bottom-box.top});
    }
    return {design:Number(agent.dataset.design),width:box.width,height:box.height,scrollWidth:agent.scrollWidth,clientWidth:agent.clientWidth,conversationHeight:agent.querySelector('.conversation').getBoundingClientRect().height,inputVisible:visible(agent.querySelector('input,textarea')),overflow};
   });
  }).toString()+")("+JSON.stringify(theme)+","+width+","+expanded+")");
  results.push({theme,width,expanded,measurements});
  await fs.writeFile(OUTPUT+'/results.json',JSON.stringify(results,null,2));
  assert.equal(measurements.length,30);
  for(const m of measurements){
   const label=theme+' '+width+' '+(expanded?'expanded':'collapsed')+' design '+m.design;
   assert.deepEqual(m.overflow,[],label+' controls must stay inside panel');
   assert.ok(m.scrollWidth<=m.clientWidth+1,label+' horizontal overflow');
   assert.ok(m.conversationHeight>=150,label+' conversation retains space');
   if(expanded)assert.equal(m.inputVisible,true,label+' input remains visible');
  }
 }
 await win.webContents.executeJavaScript("document.querySelector('#measurement-style').remove(); document.querySelectorAll('.agent').forEach(x=>x.classList.remove('expanded'));document.documentElement.dataset.theme='dark';window.scrollTo(0,0)");
 await settle();
 await fs.writeFile(OUTPUT+'/overview-dark.png',(await win.webContents.capturePage()).toPNG());
 for(const count of [3,4]){
  await win.webContents.executeJavaScript("window.velaCompactGallery.setCount("+count+");window.velaCompactGallery.select(1);document.querySelector('#selected-title').scrollIntoView();");
  await settle();
  await fs.writeFile(OUTPUT+'/selected-'+count+'-dark.png',(await win.webContents.capturePage()).toPNG());
 }
 await win.webContents.executeJavaScript("document.documentElement.dataset.theme='light';window.scrollTo(0,0)");
 await settle();
 await fs.writeFile(OUTPUT+'/overview-light.png',(await win.webContents.capturePage()).toPNG());
 assert.deepEqual(errors,[],"No browser errors");
 console.log(JSON.stringify({passed:true,measurements:240,designs:30,themes:2,widths:[320,360],heights:[350],states:['collapsed','expanded'],output:OUTPUT}));
 clearTimeout(watchdog);win.destroy();app.exit(0);
})().catch(error=>{console.error(error);clearTimeout(watchdog);app.exit(1)});
`;
await writeFile(entry, source.replaceAll("PROFILE",JSON.stringify(path.join(temporary,"profile"))).replaceAll("FIXTURE",JSON.stringify(path.resolve("scripts/fixtures/compact-agent-designs.html"))).replaceAll("OUTPUT",JSON.stringify(output)));
const env={...process.env};delete env.ELECTRON_RUN_AS_NODE;
const child=spawn(electron,[entry],{env,stdio:"inherit"});
child.on("error",error=>{console.error(error);process.exitCode=1});
child.on("exit",code=>{process.exitCode=code??1});
