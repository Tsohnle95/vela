import { spawn } from "node:child_process";
import { mkdtemp, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import electron from "electron";
const temporary = await mkdtemp(path.join(os.tmpdir(), "vela-agent-workspace-"));
const output = process.env.VELA_AGENT_REPORT ?? "/private/tmp/vela-agent-workspace-report";
await mkdir(output, { recursive: true });
const url = process.env.VELA_AGENT_TEST_URL ?? "http://127.0.0.1:5280/scripts/fixtures/agent-workspace.html";
const entry = path.join(temporary, "main.cjs");
const source = String.raw`
const {app,BrowserWindow}=require('electron');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
app.setPath('userData',PROFILE);
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
app.whenReady().then(async()=>{
 const win=new BrowserWindow({show:true,width:1440,height:1000,webPreferences:{contextIsolation:true,backgroundThrottling:false}});
 const errors=[];
 win.webContents.on('console-message',({level,message})=>{if(level==='error'){errors.push(message);console.error('Renderer:',message)}});
 const run=code=>win.webContents.executeJavaScript(code).catch(error=>{throw new Error(code+": "+error.message)});
 const wait=async code=>{for(let i=0;i<300;i++){if(await run(code))return;await pause(50)}throw new Error('Timed out '+code)};
 await win.loadURL(FIXTURE_URL);
 await wait('!!document.querySelector(".agent-panel .composer")');
 await run('document.querySelector("[data-panel-action=toggle-model-mode]").click()');
 for(let count=2;count<=4;count++) {
  await run('[...document.querySelectorAll("button[aria-label]")].find(x=>x.getAttribute("aria-label")==="Agent panel options").click()');
  await wait('!!document.querySelector("[role=menuitemcheckbox][aria-checked=false]")');
  await run('document.querySelector("[role=menuitemcheckbox][aria-checked=false]").click()');
  await wait('document.querySelectorAll(".agent-col > .agent-panel").length==='+count);
  if(count===2)assert.equal(await run('document.querySelectorAll(".agent-panel[data-compact=true]").length'),0,'Two panels retain full-size input');
 }
 await run('fixtureReplay()');
 await pause(500);
 const grid=await run(`+JSON.stringify(`(()=>[...document.querySelectorAll('.agent-col')].map(col=>{const p=col.querySelector('.agent-panel');const c=p.querySelector('.composer-body');const dock=p.querySelector('[data-component=session-prompt-dock]');const v=p.querySelector('[data-agent-response-viewport]')??p.querySelector('.agent-scroll');return {compact:p.dataset.compact,row:col.dataset.gridRow??'upper',border:getComputedStyle(p).borderTopWidth,panel:p.getBoundingClientRect().height,dock:dock.getBoundingClientRect().height,composer:c.getBoundingClientRect().height,conversation:v.getBoundingClientRect().height,radius:getComputedStyle(c).borderRadius,status:!!p.querySelector('[data-agent-stream-status]'),overflow:p.scrollWidth>p.clientWidth}}))()`)+String.raw`);
 console.log('Grid',JSON.stringify(grid));
 assert.equal(grid.length,4);
 for(const panel of grid){assert.equal(panel.compact,'true');assert.ok(panel.dock<90,'Compact input uses less than 90px');assert.ok(panel.conversation>panel.panel*.65,'Conversation owns most of each grid cell');assert.equal(panel.radius,'0px');assert.equal(panel.status,false,'Idle activity reserves no gap');assert.equal(panel.overflow,false);if(panel.row==='lower')assert.equal(panel.border,'1px')}
 await fs.writeFile(OUTPUT+'/grid.png',(await win.webContents.capturePage()).toPNG());
 for(let count=5;count<=8;count++) {
  await run('[...document.querySelectorAll("button[aria-label]")].find(x=>x.getAttribute("aria-label")==="Agent panel options").click()');
  await wait('!!document.querySelector("[role=menuitemcheckbox][aria-checked=false]")');
  await run('document.querySelector("[role=menuitemcheckbox][aria-checked=false]").click()');
  await wait('document.querySelectorAll(".agent-col > .agent-panel").length==='+count);
 }
 await pause(200);
 const narrow=await run('(()=>[...document.querySelectorAll(".agent-col > .agent-panel")].map(p=>{const dock=p.querySelector("[data-component=session-prompt-dock]");const box=p.getBoundingClientRect();return {width:box.width,dock:dock.getBoundingClientRect().height,overflow:p.scrollWidth>p.clientWidth,controls:[...p.querySelectorAll(".composer-toolbar button")].map(x=>{const b=x.getBoundingClientRect();return b.left>=box.left && b.right<=box.right})}}))()');
 for(const cell of narrow){assert.ok(cell.dock<110,'Narrow input remains compact');assert.equal(cell.overflow,false);assert.ok(cell.controls.every(Boolean),'All model/effort/send controls fit the cell')}
 console.log('Eight panels',JSON.stringify(narrow));
 await fs.writeFile(OUTPUT+'/grid-eight.png',(await win.webContents.capturePage()).toPNG());
 await run('document.querySelector("[data-panel-action=toggle-settings]").click()');
 await wait('!!document.querySelector(".appearance-art-control")');
 const settings=await run(`+JSON.stringify(`(()=>{const row=document.querySelector('.appearance-interface-row');const art=document.querySelector('.appearance-art-control');const a=art.getBoundingClientRect(),i=document.querySelector('.ui-choice').getBoundingClientRect();return {nav:[...document.querySelectorAll('.settings-nav-item')].map(x=>x.textContent.trim()),labels:[...document.querySelectorAll('.theme-card')].map(x=>x.getAttribute('aria-label')),artInRow:row.contains(art),artWidth:a.width,interfaceWidth:row.getBoundingClientRect().width,artY:a.y,choiceY:i.y}})()`)+String.raw`);
 console.log('Settings',JSON.stringify(settings));
 assert.ok(!settings.nav.includes('Plugins')&&!settings.nav.includes('Safety'));
 assert.ok(settings.labels.includes('Midnight')&&settings.labels.includes('Daybreak'));
 assert.equal(settings.artInRow,true);
 assert.ok(settings.artWidth<settings.interfaceWidth*.7,'Art is compact rather than a full-width card');
 await pause(200);
 await fs.writeFile(OUTPUT+'/settings.png',(await win.webContents.capturePage()).toPNG());
 const palettes=await run('[...document.querySelectorAll(".theme-card")].map(x=>x.getAttribute("aria-label"))');
 for(const palette of palettes) {
  await run('[...document.querySelectorAll(".theme-card")].find(x=>x.getAttribute("aria-label")==='+JSON.stringify(palette)+').click()');
  await pause(80);
  const cells=await run('(()=>[...document.querySelectorAll(".agent-panel[data-compact=true]")].map(p=>{const c=p.querySelector(".composer-body");return {radius:getComputedStyle(c).borderRadius,minimum:getComputedStyle(c).minHeight,background:getComputedStyle(c).backgroundColor,dock:p.querySelector("[data-component=session-prompt-dock]").getBoundingClientRect().height}}))()');
  assert.equal(cells.length,8);
  for(const cell of cells){assert.equal(cell.radius,'0px',palette+' keeps the compact shape');assert.equal(cell.minimum,'0px',palette+' keeps compact height');assert.equal(cell.background,'rgba(0, 0, 0, 0)',palette+' keeps a quiet transparent input');assert.ok(cell.dock<110,palette+' keeps the dock minimal')}
 }
 await run('[...document.querySelectorAll(".theme-card")].find(x=>x.getAttribute("aria-label")==="Kitty Glass").click()');
 await pause(100);
 await run('document.querySelector("[data-panel-action=toggle-settings]").click()');
 const glass=await run(`+JSON.stringify(`(()=>{const names={body:'body',app:'.app',rail:'.activity-rail',sidebar:'.sidebar',workspace:'.workspace-area',panel:'.agent-panel',conversation:'.agent-scroll'};return Object.fromEntries(Object.entries(names).map(([name,selector])=>{const e=document.querySelector(selector);return [name,e?getComputedStyle(e).backgroundColor:null]}))})()`)+String.raw`);
 console.log('Glass',JSON.stringify(glass));
 for(const [name,color] of Object.entries(glass)){if(color)assert.ok(color==='transparent'||color.startsWith('rgba('),name+' must retain transparency: '+color)}
 await pause(200);
 await fs.writeFile(OUTPUT+'/kitty.png',(await win.webContents.capturePage()).toPNG());
 await run('document.querySelector("[data-panel-action=toggle-settings]").click()');
 await run('[...document.querySelectorAll(".theme-card")].find(x=>x.getAttribute("aria-label")==="Observatory").click()');
 await run('document.querySelector("[data-panel-action=toggle-settings]").click()');
 for(const view of ['files','sessions']) {
  await run('(()=>{const button=document.querySelector("[data-panel-action=show-'+view+']");if(button.getAttribute("aria-pressed")!=="true")button.click()})()');
  await pause(200);
  const brand=await run('(()=>{const image=document.querySelector(".sidebar .observatory-wordmark");return {text:image?.alt,width:image?.getBoundingClientRect().width,display:image?getComputedStyle(image).display:null,topbar:!!document.querySelector(".titlebar .observatory-wordmark"),legacyText:getComputedStyle(document.querySelector(".sidebar-brand-text")).display}})()');
  assert.equal(brand.text,'Vela');assert.ok(brand.width>0);assert.notEqual(brand.display,'none');assert.equal(brand.topbar,false);assert.equal(brand.legacyText,'none');
  await fs.writeFile(OUTPUT+'/observatory-'+view+'.png',(await win.webContents.capturePage()).toPNG());
 }
 await fs.writeFile(OUTPUT+'/results.json',JSON.stringify({grid,settings,glass,errors},null,2));
 assert.equal(errors.length,0,'Fixture must not log renderer errors: '+errors.join('\n'));
 console.log('Agent workspace regression passed');
 app.exit(0);
}).catch(error=>{console.error(error);app.exit(1)});
setTimeout(()=>app.exit(1),60000).unref();
`;
await writeFile(entry, source.replaceAll("PROFILE", JSON.stringify(path.join(temporary, "profile"))).replaceAll("FIXTURE_URL", JSON.stringify(url)).replaceAll("OUTPUT", JSON.stringify(output)));
const child = spawn(electron, [entry], { stdio: "inherit", env: { ...process.env, ELECTRON_RUN_AS_NODE: "" } });
process.exitCode = await new Promise(resolve => child.on("exit", code => resolve(code ?? 1)));
