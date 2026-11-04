import { spawn } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import electron from "electron";

const temporary = await mkdtemp(path.join(os.tmpdir(), "vela-editor-suggestions-"));
const entry = path.join(temporary, "main.cjs");
const url = process.env.VELA_EDITOR_TEST_URL ?? "http://127.0.0.1:5280/scripts/fixtures/editor-suggestions.html";
await writeFile(entry, `
const {app,BrowserWindow}=require('electron');
const assert=require('node:assert/strict');
app.setPath('userData',${JSON.stringify(path.join(temporary, "profile"))});
const pause=ms=>new Promise(r=>setTimeout(r,ms));
app.whenReady().then(async()=>{
 const win=new BrowserWindow({show:true,width:1200,height:850,webPreferences:{contextIsolation:true,backgroundThrottling:false}});
 const run=code=>win.webContents.executeJavaScript(code);
 await win.loadURL(${JSON.stringify(url)});
 for(let i=0;i<200 && !await run('!!window.testEditor');i++) await pause(50);
 assert.ok(await run('!!window.testEditor'),'Fixture mounted');
 win.focus();
 for(const theme of ['vela-original','vs']) {
  await run('testMonaco.editor.setTheme('+JSON.stringify(theme)+');testEditor.focus();testEditor.trigger("test","editor.action.triggerSuggest",{})');
  for(let i=0;i<200 && !await run('!!document.querySelector(".suggest-widget.visible .monaco-list-row")');i++) await pause(50);
  const metrics=await run(\`(()=>{const widget=document.querySelector('.suggest-widget.visible');const tree=widget.querySelector('.tree');const rows=[...widget.querySelectorAll('.monaco-list-row')];const box=widget.getBoundingClientRect();const list=widget.querySelector('.monaco-list');return {height:box.height,visible:rows.filter(x=>{const b=x.getBoundingClientRect();return b.top>=box.top && b.bottom<=box.bottom}).length,padding:getComputedStyle(tree).padding,display:getComputedStyle(tree).display,first:rows[0].textContent,x:box.x+box.width/2,y:box.y+box.height/2,tabs:getComputedStyle(document.querySelector('.titlebar-tabs-slot')).display};})()\`);
  console.log(theme,JSON.stringify(metrics));
  assert.ok(metrics.visible>=8,'Show at least eight readable suggestions');
  assert.equal(metrics.padding,'0px','Explorer padding must not leak into Monaco');
  assert.notEqual(metrics.display,'flex','Explorer flex layout must not leak into Monaco');
  assert.notEqual(metrics.tabs,'none','Changes keeps titlebar tabs visible');
  win.webContents.sendInputEvent({type:'mouseWheel',x:Math.round(metrics.x),y:Math.round(metrics.y),deltaY:-400,wheelTicksY:-4,canScroll:true});
  await pause(250);
  const first=await run('document.querySelector(".suggest-widget.visible .monaco-list-row").textContent');
  assert.notEqual(first,metrics.first,'Wheel scroll exposes remaining suggestions inside the list');
  win.webContents.sendInputEvent({type:'keyDown',keyCode:'Down'});
  win.webContents.sendInputEvent({type:'keyUp',keyCode:'Down'});
  win.webContents.sendInputEvent({type:'keyDown',keyCode:'Tab'});
  win.webContents.sendInputEvent({type:'keyUp',keyCode:'Tab'});
  await pause(150);
  assert.notEqual(await run('testEditor.getValue()'),'document.','Keyboard accepts a completion');
  await run('testEditor.setValue("document.");testEditor.setPosition({lineNumber:1,column:10})');
 }
 console.log('Editor suggestions and Changes tab visibility passed');
 app.exit(0);
}).catch(error=>{console.error(error);app.exit(1)});
setTimeout(()=>app.exit(1),45000).unref();
`);
const child = spawn(electron, [entry], { stdio: "inherit", env: { ...process.env, ELECTRON_RUN_AS_NODE: "" } });
process.exitCode = await new Promise((resolve) => child.on("exit", (code) => resolve(code ?? 1)));
