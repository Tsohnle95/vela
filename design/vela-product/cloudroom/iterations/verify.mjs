import { spawn } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import electron from 'electron';

const root = path.dirname(fileURLToPath(import.meta.url));
const output = await mkdtemp(path.join(os.tmpdir(), 'vela-iterations-30-'));
const runner = path.join(output, 'runner.cjs');
const source = String.raw`
const {app,BrowserWindow}=require('electron');
const fs=require('node:fs/promises');
const assert=require('node:assert/strict');
app.setPath('userData',OUTPUT+'/profile');
const watchdog=setTimeout(()=>app.exit(1),240000);
(async()=>{
 await app.whenReady();
 const win=new BrowserWindow({show:false,width:1440,height:1000,useContentSize:true,webPreferences:{sandbox:true,contextIsolation:true,nodeIntegration:false,backgroundThrottling:false}});
 const errors=[]; win.webContents.on('console-message',({level,message})=>{if(level==='error'){errors.push(message);console.error(message)}});
 const js=code=>win.webContents.executeJavaScript(code).catch(e=>{console.error('Expression:',code);throw e});
 const settle=()=>js('document.fonts.ready.then(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))))');
 const select=async(id)=>{await js('document.querySelector("#design").value="'+id+'";document.querySelector("#design").dispatchEvent(new Event("change"))');await settle();await js('Promise.all([...document.querySelectorAll("main img")].map(i=>i.decode()))');};
 const color=async(id)=>{await js('document.querySelector("#palette").value="'+id+'";document.querySelector("#palette").dispatchEvent(new Event("change"))');await settle();};
 await win.loadFile(ROOT+'/index.html');await settle();
 const ids=await js('window.velaIterations.map(d=>d.id).sort((a,b)=>a-b)');
 assert.equal(ids.length,32);assert.equal(new Set(ids).size,32);
 assert.equal(await js('window.velaIterations.filter(d=>!d.reference&&d.family===1).length'),15);
 assert.equal(await js('window.velaIterations.filter(d=>!d.reference&&d.family===2).length'),15);
 assert.equal(await js('window.velaPalettes.length'),24);
 console.log('Checking saved palettes and navigation');
 // Exercise normal navigation before the deliberately dense 768-change style matrix.
 await select(21);await color('ultraviolet');await select(22);await color('paper');await select(21);
 assert.equal(await js('document.querySelector("#palette").value'),'ultraviolet');
 await new Promise(resolve=>{win.webContents.once('did-finish-load',resolve);win.reload()});await settle();assert.equal(await js('document.querySelector("#palette").value'),'ultraviolet');
 await select(22);assert.equal(await js('document.querySelector("#palette").value'),'paper');
 await select(21);await color('cyan');await color('coral');
 await js('new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error("Back did not fire popstate")),5000);addEventListener("popstate",()=>{clearTimeout(timer);requestAnimationFrame(()=>resolve(true))},{once:true});history.back()})');await settle();assert.equal(await js('document.querySelector("#palette").value'),'cyan');
 await js('document.querySelector("#main").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowRight",bubbles:true}))');assert.equal(await js('document.querySelector("#design").value'),'22');
 await js('document.querySelector("#palette").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowRight",bubbles:true}))');assert.equal(await js('document.querySelector("#design").value'),'22');
 await color('mint');
 await js('document.querySelector("#main").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowUp",bubbles:true,cancelable:true}))');assert.equal(await js('document.querySelector("#palette").value'),'violet-field');
 await js('document.querySelector("#main").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowDown",bubbles:true,cancelable:true}))');assert.equal(await js('document.querySelector("#palette").value'),'mint');
 await js('document.querySelector("#main").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowDown",bubbles:true,cancelable:true}))');assert.equal(await js('document.querySelector("#palette").value'),'mono');
 assert.equal(await js('document.querySelector("#design").value'),'22');assert.equal(await js('new URL(location.href).searchParams.get("palette")'),'mono');
 await js('document.querySelector("#palette").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowUp",bubbles:true,cancelable:true}))');assert.equal(await js('document.querySelector("#palette").value'),'mono');
 await select(21);await select(22);assert.equal(await js('document.querySelector("#palette").value'),'mono');
 await select(1);await js('document.querySelector("#previous").click()');assert.equal(await js('document.querySelector("#design").value'),'50');await js('document.querySelector("#next").click()');assert.equal(await js('document.querySelector("#design").value'),'1');
 console.log('Checking thumbnail directory');
 win.setContentSize(320,1050);await settle();await js('document.querySelector("#browse").click();document.querySelectorAll(".gallery-card img").forEach(i=>i.loading="eager")');await js('Promise.all([...document.querySelectorAll(".gallery-card img")].map(i=>i.decode()))');assert.equal(await js('document.querySelectorAll(".gallery-card").length'),32);assert.ok(await js('document.querySelector("dialog").scrollWidth<=document.querySelector("dialog").clientWidth+1'));await fs.writeFile(OUTPUT+'/gallery-mobile.png',(await win.webContents.capturePage()).toPNG());
 await js('document.querySelectorAll(".gallery-card")[17].click()');assert.equal(await js('document.querySelector("#design").value'),'36');
 await js('document.querySelector("#hide-controls").click()');assert.equal(await js('document.querySelector(".iteration-controls").hidden'),true);await js('document.querySelector("#restore-controls").click()');assert.equal(await js('document.querySelector(".iteration-controls").hidden'),false);
 await js('localStorage.removeItem("vela-release-iteration-palettes-v1")');await win.loadFile(ROOT+'/index.html',{query:{design:'21',palette:'mint'}});await settle();
 console.log('Checking 160 layouts');
 const measurements=[];const paletteFailures=[];
 for(const width of [1440,1024,768,390,320]){
  win.setContentSize(width,width<801?1050:1000);
  for(const id of ids){
   await select(id);
   const m=await js('('+(()=>{
    const h=document.querySelector('h1'),c=getComputedStyle(h);
    const expected=innerWidth>1080?Math.max(54,Math.min(80,innerWidth*.058)):innerWidth>800?Math.max(54,Math.min(68,innerWidth*.063)):innerWidth>640?72:Math.max(50,Math.min(72,innerWidth*.12));
    const outside=[...document.querySelectorAll('main h1,main .button,main .app-shot img,.iteration-controls')].filter(el=>{const r=el.getBoundingClientRect();return r.left < -1||r.right>innerWidth+1}).map(e=>e.className);
    const frames=[...document.querySelectorAll('.app-shot')].filter(el=>{const s=getComputedStyle(el),i=getComputedStyle(el.querySelector('img'));return parseFloat(s.borderTopWidth)>0||parseFloat(s.paddingLeft)>0||s.backgroundColor!=='rgba(0, 0, 0, 0)'||parseFloat(i.borderTopWidth)>0||i.filter!=='none'}).length;
    return {width:innerWidth,title:document.title,overflow:document.documentElement.scrollWidth-innerWidth,outside,frames,fontSize:parseFloat(c.fontSize),fontWeight:c.fontWeight,typeMatches:Math.abs(parseFloat(c.fontSize)-expected)<.05&&c.fontWeight==='550'&&Math.abs(parseFloat(c.lineHeight)-expected*1.04)<.06,images:[...document.querySelectorAll('main img')].every(i=>i.naturalWidth===1420),headings:document.querySelectorAll('h1').length,ornaments:document.querySelectorAll('main svg,.home-orbit,.home-art>.vela-mark').length};
   }).toString()+')()');
   measurements.push({id,...m});
   await fs.writeFile(OUTPUT+'/results.json',JSON.stringify({measurements,errors},null,2));
   if(width===1440||width===390){
    await js('document.querySelector(".iteration-controls").style.visibility="hidden"');await settle();
    const capture=await win.webContents.capturePage();
    await fs.writeFile(OUTPUT+'/'+width+'-'+id+'.png',capture.toPNG());
    if(width===1440)await fs.writeFile(OUTPUT+'/preview-'+String(id).padStart(2,'0')+'.jpg',capture.resize({width:500}).toJPEG(85));
    await js('document.querySelector(".iteration-controls").style.visibility=""');
   }
  }
 }
 console.log('Checking 768 palette combinations');
 // Every layout can actually apply every palette, including non-default light fields.
 win.setContentSize(1440,1000);
 for(const id of ids){
  await select(id);
  const failed=await js('('+(()=>{
   const failures=[];
   for(const p of window.velaPalettes){
    const picker=document.querySelector('#palette');picker.value=p.id;picker.dispatchEvent(new Event('change'));
    const s=getComputedStyle(document.body),button=getComputedStyle(document.querySelector('main .button'));
    const rgb=hex=>'rgb('+hex.slice(1).match(/../g).map(x=>parseInt(x,16)).join(', ')+')';
    if(s.backgroundColor!==rgb(p.bg)||s.color!==rgb(p.fg)||button.backgroundColor!==rgb(p.accent)||button.color!==rgb(p.button))failures.push(p.id);
   }
   return failures;
  }).toString()+')()');
  if(failed.length)paletteFailures.push({id,failed});
  await color('mint');
 }
 const palettes=await js('window.velaPalettes');
 const luminance=hex=>hex.slice(1).match(/../g).map(x=>parseInt(x,16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((a,v,i)=>a+v*[.2126,.7152,.0722][i],0);
 const contrast=(a,b)=>{const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)};
 for(const p of palettes){assert.ok(contrast(p.fg,p.bg)>=4.5,p.id+' body contrast');assert.ok(contrast(p.muted,p.bg)>=4.5,p.id+' description contrast');assert.ok(contrast(p.accent,p.bg)>=3,p.id+' large accent contrast');assert.ok(contrast(p.accent,p.button)>=4.5,p.id+' button contrast');}
 // Preview swatches use the same actual screenshot and page composition.
 for(const id of [21,36])for(const p of palettes){await select(id);await color(p.id);await js('document.querySelector(".iteration-controls").style.visibility="hidden"');await settle();const capture=await win.webContents.capturePage();await fs.writeFile(OUTPUT+'/palette-'+id+'-'+p.id+'.jpg',capture.resize({width:400}).toJPEG(85));await js('document.querySelector(".iteration-controls").style.visibility=""');}
 if(REFRESH){await fs.mkdir(ROOT+'/previews',{recursive:true});for(const id of ids)await fs.copyFile(OUTPUT+'/preview-'+String(id).padStart(2,'0')+'.jpg',ROOT+'/previews/'+String(id).padStart(2,'0')+'.jpg');}
 // Original #1 and #2 keep their URLs and now share the corrected image treatment/type.
 for(const id of [1,2]){win.setContentSize(1440,1000);await win.loadFile(ROOT+'/../index.html',{query:{design:String(id)}});await settle();assert.equal(await js('getComputedStyle(document.querySelector("h1")).fontSize'),'80px');assert.equal(await js('document.querySelectorAll(".home-orbit,.home-art>.vela-mark").length'),0);assert.ok(await js('[...document.querySelectorAll(".app-shot")].every(e=>getComputedStyle(e).backgroundColor==="rgba(0, 0, 0, 0)"&&parseFloat(getComputedStyle(e).paddingLeft)===0)'));}
 const failures=measurements.filter(m=>m.overflow>1||m.outside.length||m.frames||!m.typeMatches||!m.images||m.headings!==1||m.ornaments);
 for(const width of [1440,390]){
  const cards=ids.map(id=>'<figure><img src="'+width+'-'+id+'.png"><figcaption>'+id+'</figcaption></figure>').join('');
  await fs.writeFile(OUTPUT+'/sheet-'+width+'.html','<html><style>*{box-sizing:border-box}body{margin:0;padding:15px;background:#d8ded5;display:grid;grid-template-columns:repeat(4,1fr);gap:15px}figure{margin:0;background:white;overflow:hidden}img{display:block;width:100%}figcaption{font:15px monospace;padding:8px}</style>'+cards+'</html>');win.setContentSize(width===1440?1600:800,width===1440?2650:4700);await win.loadFile(OUTPUT+'/sheet-'+width+'.html');await js('Promise.all([...document.images].map(i=>i.decode()))');await settle();await fs.writeFile(OUTPUT+'/contact-'+width+'.png',(await win.webContents.capturePage()).toPNG());
 }
 for(const id of [21,36]){const cards=palettes.map(p=>'<figure><img src="palette-'+id+'-'+p.id+'.jpg"><figcaption>'+p.name+'</figcaption></figure>').join('');await fs.writeFile(OUTPUT+'/colors-'+id+'.html','<html><style>*{box-sizing:border-box}body{margin:0;padding:15px;background:#d8ded5;display:grid;grid-template-columns:repeat(4,1fr);gap:15px}figure{margin:0;background:white}img{display:block;width:100%}figcaption{font:14px sans-serif;padding:9px}</style>'+cards+'</html>');win.setContentSize(1600,1950);await win.loadFile(OUTPUT+'/colors-'+id+'.html');await js('Promise.all([...document.images].map(i=>i.decode()))');await settle();await fs.writeFile(OUTPUT+'/palettes-'+id+'.png',(await win.webContents.capturePage()).toPNG());}
 await fs.writeFile(OUTPUT+'/results.json',JSON.stringify({measurements,failures,paletteFailures,errors,paletteCombinations:ids.length*palettes.length,interactions:'passed'},null,2));
 console.log(JSON.stringify({iterations:30,references:2,layouts:measurements.length,palettes:24,paletteCombinations:768,failures,paletteFailures,errors,output:OUTPUT}));
 assert.deepEqual(failures,[]);assert.deepEqual(paletteFailures,[]);assert.deepEqual(errors,[]);
 clearTimeout(watchdog);win.close();setImmediate(()=>app.quit());
})().catch(e=>{console.error(e);clearTimeout(watchdog);app.exit(1)});
`;
 await writeFile(runner, source.replaceAll('OUTPUT', JSON.stringify(output)).replaceAll('ROOT', JSON.stringify(root)).replaceAll('REFRESH', String(process.argv.includes('--refresh-previews'))));
const env = { ...process.env }; delete env.ELECTRON_RUN_AS_NODE;
const child = spawn(electron, [runner], { env, stdio: 'inherit' });
child.on('error', error => { console.error(error); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
