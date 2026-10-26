import { execFileSync, spawn } from 'node:child_process';
import { mkdtemp, mkdir, symlink, writeFile } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import electron from 'electron';

const root = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(root, '../../../..');
const output = await mkdtemp(path.join(os.tmpdir(), 'vela-reference-layout-'));
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
 const metrics=async(win)=>win.webContents.executeJavaScript('('+(()=>{
  const pick=selectors=>Object.fromEntries(selectors.map(s=>{const e=document.querySelector(s),r=e.getBoundingClientRect(),c=getComputedStyle(e);return [s,{x:r.x,y:r.y,width:r.width,height:r.height,transform:c.transform,fontSize:c.fontSize,fontWeight:c.fontWeight}]}));
  const hero=document.querySelector('.home-screen img'),product=document.querySelector('.home-product .app-shot img'),frame=document.querySelector('.home-product .app-shot'),f=getComputedStyle(frame);
  return {geometry:pick(['.home-hero','.home-hero h1','.home-art']),hero:{src:hero.getAttribute('src'),natural:[hero.naturalWidth,hero.naturalHeight]},product:{src:product.getAttribute('src'),natural:[product.naturalWidth,product.naturalHeight]},frame:{padding:f.padding,border:f.borderTopWidth,background:f.backgroundColor},overflow:document.documentElement.scrollWidth-innerWidth};
 }).toString()+')()');
 for(const width of [1904,1440,1024,768,390,320]){
  const original=new BrowserWindow({...options,width}),current=new BrowserWindow({...options,width});
  await ready(original,BASELINE+'/design/vela-product/cloudroom/index.html',{design:'1'});
  await ready(current,ROOT+'/index.html',{design:'1',palette:'mint'});
  const before=await metrics(original),after=await metrics(current);
  await fs.writeFile(OUTPUT+'/'+width+'-reference.png',(await current.webContents.capturePage()).toPNG());
  results.push({width,geometryEqual:JSON.stringify(before.geometry)===JSON.stringify(after.geometry),hero:after.hero,product:after.product,frame:after.frame,overflow:after.overflow});
  original.close();current.close();
 }
 await fs.writeFile(OUTPUT+'/results.json',JSON.stringify(results,null,2));
 console.log(JSON.stringify({baseline:'7c0a724b',results,output:OUTPUT}));
 for(const result of results){
  assert.equal(result.geometryEqual,true,'original type and spacing at '+result.width);
  assert.match(result.hero.src,/vela-light\.png$/,'light hero at '+result.width);
  assert.deepEqual(result.hero.natural,[1619,971],'hero dimensions at '+result.width);
  assert.match(result.product.src,/vela-dark\.png$/,'dark product image at '+result.width);
  assert.deepEqual(result.product.natural,[1619,971],'product image dimensions at '+result.width);
  assert.equal(result.frame.padding,'0px','product image has no mat at '+result.width);
  assert.equal(result.frame.border,'0px','product image has no border at '+result.width);
  assert.equal(result.frame.background,'rgba(0, 0, 0, 0)','product image has no fill at '+result.width);
  assert.ok(result.overflow<=1,'horizontal overflow at '+result.width);
 }
 clearTimeout(watchdog);setImmediate(()=>app.quit());
})().catch(e=>{console.error(e);clearTimeout(watchdog);app.exit(1)});
`;
const runner = path.join(output, 'runner.cjs');
await writeFile(runner, source.replaceAll('OUTPUT', JSON.stringify(output)).replaceAll('BASELINE', JSON.stringify(baseline)).replaceAll('ROOT', JSON.stringify(root)));
const env = { ...process.env }; delete env.ELECTRON_RUN_AS_NODE;
const child = spawn(electron, [runner], { env, stdio: 'inherit' });
child.on('error', error => { console.error(error); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
