import { spawn } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import electron from 'electron';

const root = path.dirname(fileURLToPath(import.meta.url));
const output = await mkdtemp(path.join(os.tmpdir(), 'vela-cloudroom-20-'));
const runner = path.join(output, 'runner.cjs');
const source = String.raw`
const {app,BrowserWindow}=require('electron');
const fs=require('node:fs/promises');
const assert=require('node:assert/strict');
app.setPath('userData',OUTPUT+'/profile');
const watchdog=setTimeout(()=>app.exit(1),180000);
(async()=>{
 await app.whenReady();
 const win=new BrowserWindow({show:false,width:1440,height:1000,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}});
 const errors=[];win.webContents.on('console-message',({level,message})=>{if(level==='error'){errors.push(message);console.error('Renderer:',message)}});
 const js=code=>win.webContents.executeJavaScript(code).catch(error=>{console.error('Failed renderer expression:',code);throw error});
 const settle=()=>js('document.fonts.ready.then(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))))');
 const results=[];
 for(const width of [1440,1024,768,390,320]){
  win.setContentSize(width,width<701?1050:1000);
  await win.loadFile(ROOT+'/index.html',{query:{design:'1'}});
  await settle();
  assert.equal(await js('window.velaDirections.length'),20);
  for(let version=1;version<=20;version++){
   await js('document.querySelector("#design").value="'+version+'";document.querySelector("#design").dispatchEvent(new Event("change"))');
   await settle();
   await js('Promise.all([...document.querySelectorAll("main img")].map(i=>i.decode()))');
   const m=await js('('+(()=>{
    const outside=[...document.querySelectorAll('main h1,main .primary,main .secondary,.app-shot img,.design-controls')].filter(e=>{const r=e.getBoundingClientRect();return r.left < -1 || r.right>innerWidth+1}).map(e=>({tag:e.tagName,cls:e.className,left:e.getBoundingClientRect().left,right:e.getBoundingClientRect().right}));
    return {title:document.title,width:innerWidth,overflow:document.documentElement.scrollWidth-innerWidth,outside,headings:document.querySelectorAll('h1').length,images:[...document.querySelectorAll('main img')].every(i=>i.complete&&i.naturalWidth>0),select:document.querySelector('#design').value,links:[...document.querySelectorAll('a')].every(a=>a.getAttribute('href'))};
   }).toString()+')()');
   results.push({version,...m});
   await fs.writeFile(OUTPUT+'/results.json',JSON.stringify({results,errors},null,2));
   if(width===1440||width===390){
    await js('document.querySelector(".design-controls").style.visibility="hidden"');await settle();
    const capture=await win.webContents.capturePage();
    await fs.writeFile(OUTPUT+'/'+width+'-'+String(version).padStart(2,'0')+'.png',capture.toPNG());
    if(width===1440)await fs.writeFile(OUTPUT+'/preview-'+String(version).padStart(2,'0')+'.jpg',capture.resize({width:500}).toJPEG(85));
    await js('document.querySelector(".design-controls").style.visibility=""');
   }
  }
 }
 // Release pages stay free of presentation metadata, including the earlier collections.
 for(const collection of [{file:ROOT+'/../index.html',key:'design',count:25},{file:ROOT+'/../design-24/index.html',key:'version',count:30}]){
  for(const width of [1440,390]){
   win.setContentSize(width,1000);
   for(let item=1;item<=collection.count;item++){
    await win.loadFile(collection.file,{query:{[collection.key]:String(item)}});await settle();
    const text=await js('document.querySelector("main").textContent');
    assert.doesNotMatch(text,/A NOTE ON CONTEXT|SOURCE \/ 001|Write in the editor\. Think with your agent\.|Keep the conversation next to the code\.|Reading the original\.|SPECIMEN 009/);
    assert.equal(await js('document.querySelectorAll("main .annotation,main .frame-label,main .screen-meta,main .screen-caption,main .label,main .design-note,main .eyebrow,.header-note").length'),0);
    assert.ok(await js('document.documentElement.scrollWidth<=innerWidth+1'),'legacy page overflow '+collection.file+' '+item+' '+width);
   }
  }
 }
 win.setContentSize(320,1000); await win.loadFile(ROOT+'/index.html',{query:{design:'20'}});await settle();
 await js('document.querySelector("#next").click()');assert.equal(await js('document.querySelector("#design").value'),'1');
 await js('document.querySelector("#previous").click()');assert.equal(await js('document.querySelector("#design").value'),'20');
 await js('document.querySelector("#hide-controls").click()');assert.equal(await js('document.querySelector(".design-controls").hidden'),true);
 await js('document.querySelector("#restore-controls").click()');assert.equal(await js('document.querySelector(".design-controls").hidden'),false);
 await js('document.querySelector("#browse").click(); document.querySelectorAll(".gallery-card img").forEach(i=>i.loading="eager")'); await js('Promise.all([...document.querySelectorAll(".gallery-card img")].map(i=>i.decode()))'); assert.equal(await js('document.querySelector("dialog").open'),true); await fs.writeFile(OUTPUT+'/directory-mobile.png',(await win.webContents.capturePage()).toPNG()); win.setContentSize(1440,1000); await settle(); await fs.writeFile(OUTPUT+'/directory-desktop.png',(await win.webContents.capturePage()).toPNG()); win.setContentSize(320,1000);await settle();
 assert.ok(await js('document.querySelector("dialog").scrollWidth<=document.querySelector("dialog").clientWidth+1'));
 await js('document.querySelectorAll("[data-design]")[13].click()');assert.equal(await js('document.querySelector("#design").value'),'14');
 await js('document.querySelector("#c14-agent").click()');assert.notEqual(await js('getComputedStyle(document.querySelector(".c14-panel-agent")).display'),'none');
 await js('document.querySelector("#main").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowRight",bubbles:true}))');assert.equal(await js('document.querySelector("#design").value'),'15');
 await js('document.querySelector("#design").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowRight",bubbles:true}))');assert.equal(await js('document.querySelector("#design").value'),'15');
 await js('document.querySelector("#browse").click()');win.webContents.sendInputEvent({type:'keyDown',keyCode:'Escape'});win.webContents.sendInputEvent({type:'keyUp',keyCode:'Escape'});await settle();assert.equal(await js('document.querySelector("dialog").open'),false);
 const failures=results.filter(x=>x.overflow>1||x.outside.length||x.headings!==1||!x.images||!x.links);
 await fs.writeFile(OUTPUT+'/results.json',JSON.stringify({results,errors,failures,interactions:'passed'},null,2));
 // Raster contact sheets of actual rendered desktop and mobile compositions.
 for(const width of [1440,390]){
  const cards=Array.from({length:20},(_,i)=>'<figure><img src="'+width+'-'+String(i+1).padStart(2,'0')+'.png"><figcaption>'+String(i+1).padStart(2,'0')+'</figcaption></figure>').join('');
  await fs.writeFile(OUTPUT+'/sheet-'+width+'.html','<html><style>*{box-sizing:border-box}body{margin:0;padding:15px;background:#d8e0d3;display:grid;grid-template-columns:repeat(4,1fr);gap:15px}figure{margin:0;background:white;overflow:hidden}img{display:block;width:100%}figcaption{font:16px monospace;padding:8px}</style>'+cards+'</html>');
  win.setContentSize(width===1440?1600:800,width===1440?1660:3000);
  await win.loadFile(OUTPUT+'/sheet-'+width+'.html');await js('Promise.all([...document.images].map(i=>i.decode()))');await settle();
  await fs.writeFile(OUTPUT+'/contact-'+width+'.png',(await win.webContents.capturePage()).toPNG());
 }
 console.log(JSON.stringify({compositions:20,viewports:5,measurements:results.length,failures,errors,output:OUTPUT}));
 assert.deepEqual(failures,[]);assert.deepEqual(errors,[]);
 clearTimeout(watchdog);win.close();setImmediate(()=>app.quit());
})().catch(e=>{console.error(e);clearTimeout(watchdog);app.exit(1)});
`;
await writeFile(runner, source.replaceAll('OUTPUT', JSON.stringify(output)).replaceAll('ROOT', JSON.stringify(root)));
const env = { ...process.env };
delete env.ELECTRON_RUN_AS_NODE;
const child = spawn(electron, [runner], { env, stdio: 'inherit' });
child.on('error', error => { console.error(error); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
