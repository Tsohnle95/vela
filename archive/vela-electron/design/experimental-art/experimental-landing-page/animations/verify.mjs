import { spawn } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import electron from 'electron';

const root=path.resolve(fileURLToPath(new URL('../',import.meta.url)));
const output=await mkdtemp(path.join(tmpdir(),'vela-motion-review-'));
const entry=path.join(output,'review.cjs');
const source=String.raw`
const {app,BrowserWindow}=require('electron');
const {createServer}=require('node:http');
const {readFile,writeFile}=require('node:fs/promises');
const path=require('node:path');
const assert=require('node:assert/strict');
const root=ROOT,output=OUTPUT;
app.setPath('userData',path.join(output,'profile'));
app.disableHardwareAcceleration();
app.commandLine.appendSwitch('no-proxy-server');
let server,win;const errors=[];
const wait=async(fn,label)=>{const end=Date.now()+18000;while(!(await fn())){if(Date.now()>end)throw Error('Timed out: '+label);await new Promise(r=>setTimeout(r,50));}};
const js=code=>win.webContents.executeJavaScript(code,true);
const watchdog=setTimeout(()=>{console.error('Motion review timed out');app.exit(1);},90000);
(async()=>{
await app.whenReady();
server=createServer(async(req,res)=>{try{const pathname=decodeURIComponent(new URL(req.url,'http://localhost').pathname);const file=path.resolve(root,'.'+pathname+(pathname.endsWith('/')?'index.html':''));if(!file.startsWith(root+path.sep))throw Error('Out of bounds');const body=await readFile(file);const ext=path.extname(file);res.writeHead(200,{'Content-Type':({'.html':'text/html','.js':'text/javascript','.css':'text/css','.png':'image/png','.svg':'image/svg+xml'})[ext]||'application/octet-stream'});res.end(body);}catch{res.writeHead(404);res.end('Not found');}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;
win=new BrowserWindow({show:false,width:1400,height:1100,webPreferences:{offscreen:true,backgroundThrottling:false,contextIsolation:true,sandbox:true,nodeIntegration:false}});
win.webContents.on('console-message',(_e,level,message)=>{if(level>=2)errors.push(message);});
await win.loadURL(origin+'/animations/verify.html');
await wait(()=>js("document.querySelector('#summary').textContent!=='Running…'"),'browser verification');
const checks=await js("({summary:document.querySelector('#summary').textContent,results:[...document.querySelectorAll('#results li')].map(n=>({pass:n.dataset.pass==='true',text:n.textContent}))})");
await writeFile(path.join(output,'checks.json'),JSON.stringify(checks,null,2));
assert(checks.results.length>=20,'Complete the browser suite');assert(checks.results.every(r=>r.pass),JSON.stringify(checks));
await win.loadURL(origin+'/animations/index.html');await wait(()=>js('window.velaMotionReview?.cards.length===25'),'gallery ready');
await js("if(velaMotionReview.playing)document.querySelector('#play-all').click();");
win.setSize(1900,1800);
await js("document.body.style.margin='0';document.body.style.padding='0';const sheet=document.createElement('div');sheet.id='motion-sheet';sheet.style.cssText='display:grid;grid-template-columns:repeat(5,380px);width:1900px;background:#090a1b';velaMotionReview.cards.forEach(c=>{const item=document.createElement('div');item.style.cssText='width:380px;height:350px;background:#090a1b;border:1px solid #31233e';const stage=c.stage.svg;stage.style.cssText='width:380px;height:320px;display:block';item.append(stage);const label=document.createElement('p');label.style.cssText='margin:0;padding:4px 14px;font:12px Arial;color:#d2bfdf';label.textContent=c.info.id+' / '+c.info.name;item.append(label);sheet.append(item);});document.body.replaceChildren(sheet);");
for(const phase of [.22,.67]){
 await js('velaMotionReview.cards.forEach(c=>c.stage.draw('+phase+'));new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
 for(let row=0;row<5;row++){const png=await win.webContents.capturePage({x:0,y:row*350,width:1900,height:350});await writeFile(path.join(output,'phase-'+phase+'-row-'+(row+1)+'.png'),png.toPNG());}
}
win.setSize(390,844);
await win.loadURL(origin+'/animations/index.html');await wait(()=>js('window.velaMotionReview?.cards.length===25'),'mobile ready');
assert(await js('document.documentElement.scrollWidth<=innerWidth'),'No mobile horizontal overflow');
await js("document.querySelector('.stage-button').click()");assert(await js('document.querySelector("dialog").getBoundingClientRect().width<=innerWidth'),'Inspector fits mobile');
await new Promise(r=>setTimeout(r,100));await writeFile(path.join(output,'mobile.png'),(await win.webContents.capturePage()).toPNG());
await win.webContents.debugger.attach('1.3');await win.webContents.debugger.sendCommand('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});
await win.loadURL(origin+'/animations/index.html');await wait(()=>js('window.velaMotionReview?.cards.length===25'),'reduced motion ready');assert(await js('!velaMotionReview.playing'),'Reduced motion starts paused');
const result={...checks,extra:['mobile width and dialog fit','reduced motion starts paused'],errors,output};await writeFile(path.join(output,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({summary:checks.summary,extra:result.extra,errors,output}));assert.equal(errors.length,0,'No browser console errors');
clearTimeout(watchdog);win.destroy();server.close();app.exit(0);
})().catch(error=>{console.error(error.stack);if(win&&!win.isDestroyed())win.destroy();server?.close();app.exit(1);});
`.replace('const root=ROOT,output=OUTPUT;',`const root=${JSON.stringify(root)},output=${JSON.stringify(output)};`);
await writeFile(entry,source);
const env={...process.env};delete env.ELECTRON_RUN_AS_NODE;
const child=spawn(electron,[entry],{env,stdio:'inherit'});
child.on('error',error=>{console.error(error);process.exitCode=1;});
child.on('exit',code=>{process.exitCode=code??1;});
