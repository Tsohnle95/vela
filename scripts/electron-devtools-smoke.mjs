import { spawn } from "node:child_process";
import { cp, mkdir, mkdtemp, realpath, rm, symlink } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { build } from "esbuild";
import electron from "electron";

if (process.platform !== "darwin") {
  console.log("[electron-devtools-smoke] skipped: native DevTools coverage runs on macOS");
  process.exit(0);
}
const root = process.cwd();
const temporary = await realpath(await mkdtemp(path.join(os.tmpdir(), "vela-devtools-smoke-")));
const entry = path.join(temporary, "main.cjs");
const fixture = path.join(temporary, "checkout");
await symlink(path.join(root, "node_modules"), path.join(temporary, "node_modules"), "dir");
await mkdir(path.join(fixture, "src/renderer/src"), { recursive: true });
await mkdir(path.join(fixture, "out"), { recursive: true });
await cp(path.join(root, "src/renderer/src/styles"), path.join(fixture, "src/renderer/src/styles"), { recursive: true });
await cp(path.join(root, "src/renderer/src/assets/opencode"), path.join(fixture, "src/renderer/src/assets/opencode"), { recursive: true });
await cp(path.join(root, "out/renderer"), path.join(fixture, "out/renderer"), { recursive: true });
await cp(path.join(root, "out/main/sass-worker.js"), path.join(temporary, "sass-worker.js"));
const source = String.raw`
const { app, BrowserWindow, ipcMain } = require("electron");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { DevToolsController } = require(CONTROLLER);
const { resolveRunningAppSource } = require(RESOLVER);
const { AppStylesController } = require(APP_STYLES);
const root = ROOT;
const sourceRoot = FIXTURE;
app.setPath("userData", PROFILE);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const wait = async (predicate, label) => {
  const deadline = Date.now() + 30000;
  while (!(await predicate())) {
    if (Date.now() > deadline) throw new Error("Timed out: " + label);
    await delay(50);
  }
};
const watchdog = setTimeout(() => app.exit(2), 90000);
(async () => {
  await app.whenReady();
  const session = { id: "devtools-smoke", directory: path.join(root, "design"), workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
  const sourcePath = path.join(sourceRoot, "src/renderer/src/styles/_sessions.scss");
  const content = fs.readFileSync(sourcePath, "utf8");
  const line = content.split("\n").findIndex(value => value === ".sessions-actions {") + 1;
  assert.ok(line > 0);
  let externalRead = null;
  let saves = 0;
  let healthChecked = false;
  // Keep this integration offline: use the real preload/store/editor with a
  // fixture backend. Only the app-owned source resolver reads stylesheet paths.
  const channels = new Set([...fs.readFileSync(path.join(root, "src/preload/index.ts"), "utf8").matchAll(/ipcRenderer.invoke\("([^"]+)"/g)].map(match => match[1]));
  for (const channel of channels) ipcMain.handle(channel, async (_event, ...args) => {
    if (channel === "shell:health") { healthChecked = true; return true; }
    if (channel === "shell:state") return session;
    if (channel === "shell:active-sessions") return [session];
    if (channel === "shell:open-session-id") return { session, transcript: [], todos: [], usage: null };
    if (channel === "shell:session-transcript") return [];
    if (["shell:model-default", "shell:session-selection", "shell:session-usage", "shell:provider-usage"].includes(channel)) return null;
    if (channel === "shell:open-external") {
      assert.equal(args[1], sourcePath);
      externalRead = args[1];
      return { kind: "standalone", path: sourcePath, content: fs.readFileSync(sourcePath, "utf8") };
    }
    if (channel === "shell:fs-write-standalone") {
      assert.equal(args[0], sourcePath);
      assert.equal(args[2], fs.readFileSync(sourcePath, "utf8"));
      fs.writeFileSync(sourcePath, args[1]);
      saves++;
      return;
    }
    return [];
  });
  // Keep rendering live while the window is hidden: Chromium throttles paint and
  // rAF for backgrounded windows, which leaves Monaco's view lines unpainted (and
  // times the smoke out) on slower CI runners.
  const win = new BrowserWindow({ show: false, width: 1280, height: 900, webPreferences: { preload: path.join(root, "out/preload/index.js"), contextIsolation: true, sandbox: true, backgroundThrottling: false } });
  const wc = win.webContents;
  wc.on("console-message", (_event, level, message) => { if (level >= 2) console.error("renderer:", message); });
  assert.notEqual(app.getAppPath(), root, "The fixture must preserve the installed wrapper/check-out split");
  const requests = [];
  const controller = new DevToolsController(async (file, title) => {
    const resolved = await resolveRunningAppSource(path.join(sourceRoot, "out/main"), file, title);
    requests.push({ file, resolved });
    console.log("source-resolution", JSON.stringify({ appPath: app.getAppPath(), file, resolved }));
    return resolved;
  });
  controller.attach(wc);
  await win.loadFile(path.join(sourceRoot, "out/renderer/index.html"));
  await wait(() => healthChecked, "renderer subscription");
  wc.send("shell:message", { kind: "session", session });
  await wait(() => wc.executeJavaScript('Boolean(document.querySelector(".sidebar-brand"))'), "Vela sidebar").catch(async error => {
    console.error(await wc.executeJavaScript('document.body.innerText'));
    throw error;
  });
  const sessionsPoint = await wc.executeJavaScript('(()=>{const r=document.querySelector("[data-panel-action=show-sessions]").getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2)};})()');
  wc.sendInputEvent({ type: "mouseDown", ...sessionsPoint, button: "left", clickCount: 1 });
  wc.sendInputEvent({ type: "mouseUp", ...sessionsPoint, button: "left", clickCount: 1 });
  await wait(() => wc.executeJavaScript('Boolean(document.querySelector(".sessions-actions"))'), "Sessions actions");
  wc.openDevTools({ mode: "bottom" });
  await wait(() => wc.devToolsWebContents && !wc.devToolsWebContents.isLoading(), "DevTools");
  const point = await wc.executeJavaScript('(()=>{const r=document.querySelector(".sessions-actions").getBoundingClientRect();return {x:Math.round(r.x+1),y:Math.round(r.y+1)};})()');
  wc.inspectElement(point.x, point.y);
  const dt = wc.devToolsWebContents;
  const clickRule = "(" + function(sourceLine) {
    function walk(root) {
      for (const el of root.querySelectorAll("*")) {
        if (el.shadowRoot) { const found = walk(el.shadowRoot); if (found) return found; }
        if ((el.getAttribute("title") || "").endsWith("/_sessions.scss:" + sourceLine)) { const r=el.getBoundingClientRect(); return { x:Math.round(r.x+r.width/2), y:Math.round(r.y+r.height/2) }; }
      }
      return false;
    }
    return walk(document);
  }.toString() + ")(" + line + ")";
  let sourcePoint;
  await wait(async () => sourcePoint = await dt.executeJavaScript(clickRule), "mapped Styles link");
  dt.sendInputEvent({ type: "mouseDown", ...sourcePoint, button: "left", clickCount: 1 });
  dt.sendInputEvent({ type: "mouseUp", ...sourcePoint, button: "left", clickCount: 1 });
  await wait(() => externalRead === sourcePath, "editor source read");
  await wait(() => wc.executeJavaScript('Boolean([...document.querySelectorAll(".tab.active")].find(el => el.title === ' + JSON.stringify(sourcePath) + '))'), "active stylesheet tab");
  await wait(() => wc.executeJavaScript('Boolean(document.querySelector(".monaco-editor .view-lines")?.textContent.includes("sessions-actions"))'), "Monaco source at rule");
  assert.equal(requests.at(-1).resolved, sourcePath);
  const padding = () => wc.executeJavaScript('getComputedStyle(document.querySelector(".sessions-actions")).paddingTop');
  const baseline = await padding();
  await wc.executeJavaScript('window.__appStylesSentinel = "preserved"');
  const save = async text => {
    const prior = saves;
    const position = await wc.executeJavaScript('(()=>{const r=document.querySelector(".monaco-editor").getBoundingClientRect();return {x:Math.round(r.x+120),y:Math.round(r.y+30)};})()');
    wc.sendInputEvent({ type: "mouseDown", ...position, button: "left", clickCount: 1 });
    wc.sendInputEvent({ type: "mouseUp", ...position, button: "left", clickCount: 1 });
    wc.focus();
    await wc.executeJavaScript('document.querySelector(".monaco-editor .native-edit-context, .monaco-editor textarea.inputarea")?.focus()');
    wc.sendInputEvent({ type: "keyDown", keyCode: "a", modifiers: ["meta"] });
    wc.sendInputEvent({ type: "keyUp", keyCode: "a", modifiers: ["meta"] });
    await wc.executeJavaScript('(()=>{const data=new DataTransfer();data.setData("text/plain",' + JSON.stringify(text) + ');document.activeElement.dispatchEvent(new ClipboardEvent("paste",{clipboardData:data,bubbles:true,cancelable:true}));})()');
    await wait(() => wc.executeJavaScript('Boolean(document.querySelector(".tab.active .tab-dirty"))'), "editor dirty state");
    wc.sendInputEvent({ type: "keyDown", keyCode: "s", modifiers: ["meta"] });
    wc.sendInputEvent({ type: "keyUp", keyCode: "s", modifiers: ["meta"] });
    await wait(() => saves > prior, "Monaco save");
    const saved = fs.readFileSync(sourcePath, "utf8");
    if (saved !== text) console.error("save mismatch", JSON.stringify({ expectedLength: text.length, savedLength: saved.length, expectedPadding: text.indexOf("padding-top: 37px"), savedPadding: saved.indexOf("padding-top: 37px") }));
    assert.ok(saved === text, "Monaco save must preserve the pasted source exactly");
  };
  const changed = content.replace(/(\.sessions-actions \{[\s\S]*?)(\n\})/, "$1\n  padding-top: 37px;$2");
  assert.notEqual(changed, content);
  await save(changed);
  assert.equal(await padding(), baseline, "Reproduces saved source without a live-style owner");
  const errors = [];
  const appStyles = new AppStylesController(sourceRoot,
    data => wc.send("shell:message", { kind: "ui-command", command: "reload-app-styles", data }),
    message => { errors.push(message); wc.send("shell:message", { kind: "ui-command", command: "app-styles-error", data: message }); }
  );
  await appStyles.start();
  await wait(async () => await padding() === "37px", "saved padding applied");
  await save(changed.replace("padding-top: 37px", "padding-top: 43px"));
  await wait(async () => await padding() === "43px", "second saved padding applied");
  await save(changed + "\n.broken { padding: ; invalid");
  await wait(() => errors.length > 0, "compile failure reported");
  assert.equal(await padding(), "43px", "Invalid Sass preserves last valid CSS");
  await save(content);
  await wait(async () => await padding() === baseline, "removed padding and error recovery");
  assert.equal(await wc.executeJavaScript('window.__appStylesSentinel'), "preserved");
  await appStyles.stop();

  console.log('[electron-devtools-smoke] ' + JSON.stringify({ stylesLinkOpenedEditor: true, savedPaddingUpdated: true, errorRecovery: true, source: sourcePath, line }));
  wc.closeDevTools();
  clearTimeout(watchdog);
  app.exit(0);
})().catch(error => { console.error(error); app.exit(1); });
`;
try {
  await build({ stdin: { contents: source.replace("CONTROLLER", JSON.stringify(path.join(root, "src/main/devtools-controller.ts"))).replace("RESOLVER", JSON.stringify(path.join(root, "src/main/source-resolver.ts"))).replace("APP_STYLES", JSON.stringify(path.join(root, "src/main/app-styles.ts"))).replace("FIXTURE", JSON.stringify(fixture)).replace("ROOT", JSON.stringify(root)).replace("PROFILE", JSON.stringify(path.join(temporary, "profile"))), resolveDir: root }, bundle: true, platform: "node", format: "cjs", external: ["electron", "sass", "chokidar", "postcss", "postcss-value-parser", "source-map-js"], outfile: entry });
  const child = spawn(electron, [entry], { cwd: root, env: { ...process.env, NODE_PATH: path.join(root, "node_modules") }, stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  child.stdout.on("data", chunk => { output += chunk; process.stdout.write(chunk); });
  child.stderr.on("data", chunk => { output += chunk; process.stderr.write(chunk); });
  const timeout = setTimeout(() => child.kill("SIGKILL"), 95000);
  const code = await new Promise((resolve, reject) => { child.once("error", reject); child.once("exit", resolve); });
  clearTimeout(timeout);
  if (code !== 0 || !output.includes('"stylesLinkOpenedEditor":true')) throw new Error(`DevTools smoke failed with exit ${code}`);
} finally {
  await rm(temporary, { recursive: true, force: true });
}
