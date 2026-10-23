import { execFileSync, spawn } from 'node:child_process';
import { mkdtemp, mkdir, symlink, writeFile } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import electron from 'electron';

const root = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(root, '../../../..');
const output = await mkdtemp(path.join(os.tmpdir(), 'vela-original-reference-'));
const baseline = path.join(output, 'original');
await mkdir(baseline);
const archive = execFileSync('git', ['archive', '7c0a724b', 'design/vela-product/cloudroom'], { cwd: repo, maxBuffer: 20 * 1024 * 1024 });
execFileSync('tar', ['-x', '-C', baseline], { input: archive });
await symlink(path.resolve(root, '../../assets'), path.join(baseline, 'design/vela-product/assets'));
const source = String.raw`
const {app,BrowserWindow}=require('electron');
const fs=require('node:fs/promises');
const assert=require('node:assert/strict');
app.setPath('userData',OUTPUT+'/profile');
const watchdog=setTimeout(()=>app.exit(1),90000);
(async()=>{
 await app.whenReady();
 const options={show:false,width:1904,height:947,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}};
 const results=[];
 const ready=async(win,file,query)=>{
  await win.loadFile(file,{query});
  await win.webContents.executeJavaScript('document.fonts.ready.then(()=>Promise.all([...document.querySelectorAll("main img")].map(i=>i.decode())))');
  await win.webContents.executeJavaScript('document.querySelectorAll(".design-controls,.iteration-controls,#restore-controls").forEach(e=>e.style.visibility="hidden");window.scrollTo(0,0);new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>r(true))))');
 };
 for(const width of [1904,1440,1024,768,390,320]){
  const original=new BrowserWindow({...options,width}),current=new BrowserWindow({...options,width});
  await ready(original,BASELINE+'/design/vela-product/cloudroom/index.html',{design:'1'});
  await ready(current,ROOT+'/index.html',{design:'1',palette:'mint'});
  const before=await original.webContents.capturePage(),after=await current.webContents.capturePage();
  const a=before.toBitmap(),b=after.toBitmap();let pixels=0;
  for(let i=0;i<Math.min(a.length,b.length);i+=4)if(a[i]!==b[i]||a[i+1]!==b[i+1]||a[i+2]!==b[i+2]||a[i+3]!==b[i+3])pixels++;
  const geometry=async(win)=>win.webContents.executeJavaScript('('+(()=>{
   const selectors=['.home-hero','.home-hero h1','.home-art','.home-art>.vela-mark','.home-screen','.home-product','.home-product>.app-shot'];
   return Object.fromEntries(selectors.map(s=>{const e=document.querySelector(s),r=e.getBoundingClientRect(),c=getComputedStyle(e);return [s,{x:r.x,y:r.y,width:r.width,height:r.height,transform:c.transform,fontSize:c.fontSize,fontWeight:c.fontWeight,padding:c.padding,border:c.border,background:c.backgroundColor}]}));
  }).toString()+')()');
  const beforeGeometry=await geometry(original),afterGeometry=await geometry(current);
  await fs.writeFile(OUTPUT+'/'+width+'-original.png',before.toPNG());await fs.writeFile(OUTPUT+'/'+width+'-current.png',after.toPNG());
  results.push({width,changedPixels:pixels,sameLength:a.length===b.length,geometryEqual:JSON.stringify(beforeGeometry)===JSON.stringify(afterGeometry),beforeGeometry,afterGeometry});
  original.close();current.close();
 }
 await fs.writeFile(OUTPUT+'/results.json',JSON.stringify(results,null,2));
 console.log(JSON.stringify({baseline:'7c0a724b',results:results.map(({width,changedPixels,sameLength,geometryEqual})=>({width,changedPixels,sameLength,geometryEqual})),output:OUTPUT}));
 for(const result of results){assert.equal(result.geometryEqual,true,'original geometry at '+result.width);assert.equal(result.sameLength,true);assert.equal(result.changedPixels,0,'pixel equality at '+result.width)}
 clearTimeout(watchdog);setImmediate(()=>app.quit());
})().catch(e=>{console.error(e);clearTimeout(watchdog);app.exit(1)});
`;
const runner = path.join(output, 'runner.cjs');
await writeFile(runner, source.replaceAll('OUTPUT', JSON.stringify(output)).replaceAll('BASELINE', JSON.stringify(baseline)).replaceAll('ROOT', JSON.stringify(root)));
const env = { ...process.env }; delete env.ELECTRON_RUN_AS_NODE;
const child = spawn(electron, [runner], { env, stdio: 'inherit' });
child.on('error', error => { console.error(error); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
