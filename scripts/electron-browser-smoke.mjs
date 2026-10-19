import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { build } from "esbuild";
import electron from "electron";

if (process.platform !== "darwin") {
  console.log("[electron-browser-smoke] skipped: native guest coverage runs on macOS CI");
  process.exit(0);
}

const temporary = await mkdtemp(path.join(os.tmpdir(), "vela-browser-smoke-"));
const entry = path.join(temporary, "main.cjs");
const maliciousPreload = path.join(temporary, "malicious.cjs");
await writeFile(maliciousPreload, 'require("electron").contextBridge.exposeInMainWorld("vela", { compromised: true });');
const root = process.cwd();
const source = String.raw`
const { app, BrowserWindow, session, shell } = require("electron");
const { createServer } = require("node:http");
const assert = require("node:assert/strict");
const { attachBrowserGuards } = require(BROWSER_GUARDS);
const { BROWSER_PARTITION } = require(BROWSER_POLICY);
const { isTrustedIpcSender, trustedApplicationLocation } = require(APP_SECURITY);
const profile = PROFILE;
const maliciousPreload = MALICIOUS_PRELOAD;
app.setPath("userData", profile);
app.commandLine.appendSwitch("no-proxy-server");
const wait = async (predicate, label) => {
  const deadline = Date.now() + 8000;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error("Timed out: " + label);
    await new Promise(resolve => setTimeout(resolve, 20));
  }
};
let host;
let server;
let watchdog = setTimeout(() => { console.error("Browser smoke overall timeout"); app.exit(1); }, 25000);
(async () => {
  await app.whenReady();
  const external = [];
  shell.openExternal = async url => { external.push(url); };
  server = createServer((req, res) => {
    if (req.url === "/redirect") {
      res.writeHead(302, { Location: "http://user:secret@127.0.0.1:" + server.address().port + "/forbidden" });
      res.end();
      return;
    }
    res.writeHead(200, { "Content-Type": "text/html", "Permissions-Policy": "geolocation=(self)" });
    res.end("<!doctype html><title>Browser isolation smoke</title><body>Guest</body>");
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const origin = "http://127.0.0.1:" + server.address().port;
  const browserSession = session.fromPartition(BROWSER_PARTITION);
  const installed = {};
  const calls = { request: 0, check: 0 };
  for (const [method, kind] of [["setPermissionRequestHandler", "request"], ["setPermissionCheckHandler", "check"], ["setDevicePermissionHandler", "device"]]) {
    const original = browserSession[method].bind(browserSession);
    browserSession[method] = handler => {
      installed[kind] = handler;
      return original(handler ? (...args) => { if (kind in calls) calls[kind]++; return handler(...args); } : null);
    };
  }
  host = new BrowserWindow({ show: false, webPreferences: { webviewTag: true, contextIsolation: true, sandbox: true, nodeIntegration: false } });
  attachBrowserGuards(host.webContents);
  assert.equal(typeof installed.request, "function");
  assert.equal(typeof installed.check, "function");
  assert.equal(typeof installed.device, "function");
  assert.equal(installed.device({ deviceType: "usb", origin, device: {} }), false);
  let forbiddenAttach = false;
  let guest;
  let attachmentPreferences;
  const blocked = { navigation: 0, frame: 0, redirect: 0 };
  host.webContents.on("will-attach-webview", (event, preferences, params) => {
    if (params.src.startsWith("file:")) forbiddenAttach = event.defaultPrevented;
    else attachmentPreferences = { ...preferences };
  });
  host.webContents.on("did-attach-webview", (_event, attached) => {
    guest = attached;
    attached.on("will-navigate", (event) => { if (event.defaultPrevented) blocked.navigation++; });
    attached.on("will-frame-navigate", (event) => { if (event.defaultPrevented) blocked.frame++; });
    attached.on("will-redirect", (event) => { if (event.defaultPrevented) blocked.redirect++; });
    // Observe the production callback without allowing any OS browser launch.
    // The already installed callback is exercised by real window.open below;
    // did-create-window must therefore never fire.
    attached.on("did-create-window", () => { throw new Error("Guest created a popup"); });
  });
  await host.loadURL(origin + "/host");
  await host.webContents.executeJavaScript('document.body.innerHTML = ""; const invalid = document.createElement("webview"); invalid.src = "file:///tmp/vela-browser-forbidden.html"; document.body.append(invalid);');
  await wait(() => forbiddenAttach, "blocked initial file URL");
  assert.equal(guest, undefined);
  await host.webContents.executeJavaScript('document.body.innerHTML = ""; const view = document.createElement("webview"); view.setAttribute("partition", "persist:attacker"); view.setAttribute("preload", ' + JSON.stringify(maliciousPreload) + '); view.setAttribute("nodeintegration", ""); view.setAttribute("nodeintegrationinsubframes", ""); view.setAttribute("allowpopups", ""); view.setAttribute("webpreferences", "nodeIntegrationInWorker=yes,contextIsolation=no,sandbox=no,webSecurity=no,allowRunningInsecureContent=yes,webviewTag=yes"); view.src = ' + JSON.stringify(origin + "/guest") + '; document.body.append(view);');
  await wait(() => guest && !guest.isLoading() && guest.getURL() === origin + "/guest", "guest ready");
  const preferences = guest.getLastWebPreferences();
  for (const key of ["nodeIntegration", "nodeIntegrationInSubFrames", "nodeIntegrationInWorker", "webviewTag", "allowRunningInsecureContent"]) {
    assert.equal(attachmentPreferences[key], false, "attachment " + key);
    assert.notEqual(preferences[key], true, "effective " + key);
  }
  for (const key of ["contextIsolation", "sandbox", "webSecurity"]) assert.equal(preferences[key], true, key);
  assert.ok(!preferences.preload, "malicious preload removed");
  assert.equal(guest.session, browserSession, "dedicated partition is authoritative");
  assert.equal(guest.session.isPersistent(), false);
  assert.deepEqual(await guest.executeJavaScript('({ bridge: typeof window.vela, require: typeof require, process: typeof process })'), { bridge: "undefined", require: "undefined", process: "undefined" });
  assert.deepEqual(await guest.executeJavaScript('new Promise(resolve => { const url = URL.createObjectURL(new Blob(["postMessage({ require: typeof require, process: typeof process })"], {type: "text/javascript"})); const worker = new Worker(url); worker.onmessage = event => { worker.terminate(); URL.revokeObjectURL(url); resolve(event.data); }; })'), { require: "undefined", process: "undefined" });
  const trusted = trustedApplicationLocation(host.webContents.getURL());
  assert.equal(isTrustedIpcSender({ sender: host.webContents, senderFrame: host.webContents.mainFrame }, host.webContents, trusted), true);
  assert.equal(isTrustedIpcSender({ sender: guest, senderFrame: guest.mainFrame }, host.webContents, trusted), false);
  assert.equal(await guest.executeJavaScript('navigator.permissions.query({ name: "geolocation" }).then(result => result.state)'), "denied");
  assert.equal(await guest.executeJavaScript('new Promise(resolve => navigator.geolocation.getCurrentPosition(() => resolve("allowed"), error => resolve(error.code)))'), 1);
  assert.ok(calls.check > 0, "real permission check handler exercised");
  assert.ok(calls.request > 0, "real permission request handler exercised");
  await guest.executeJavaScript('location.href = ' + JSON.stringify(origin.replace('http://', 'http://user:secret@') + '/forbidden') + '; void 0');
  await wait(() => blocked.navigation + blocked.frame > 0, "blocked top-level navigation");
  assert.equal(guest.getURL(), origin + "/guest");
  await guest.executeJavaScript('const frame = document.createElement("iframe"); frame.src = ' + JSON.stringify(origin + '/child') + '; document.body.append(frame);');
  await wait(() => guest.mainFrame.frames.some(frame => frame.url === origin + "/child"), "child frame ready");
  const childFrame = guest.mainFrame.frames.find(frame => frame.url === origin + "/child");
  const beforeFrame = blocked.frame;
  await childFrame.executeJavaScript('location.href = "data:text/html,forbidden"; void 0');
  await wait(() => blocked.frame > beforeFrame, "blocked child-frame navigation");
  assert.equal(childFrame.url, origin + "/child");
  await guest.executeJavaScript('window.open("https://example.com/smoke-popup"); void 0');
  await wait(() => external.length === 1, "popup routed through external policy");
  assert.deepEqual(external, ["https://example.com/smoke-popup"]);
  await guest.executeJavaScript('window.open("http://127.0.0.1/smoke-popup"); void 0');
  await guest.executeJavaScript('location.href = ' + JSON.stringify(origin + "/redirect") + '; void 0');
  await wait(() => blocked.redirect > 0, "blocked credentialed redirect");
  assert.equal(guest.getURL(), origin + "/guest");
  assert.equal(external.length, 1);
  console.log("[electron-browser-smoke] " + JSON.stringify({ hardenedPreferences: true, preloadRemoved: true, guestBridgeAbsent: true, isolatedSession: true, permissionCheckDenied: true, permissionRequestDenied: true, deviceHandlerDenied: true, guestIpcIdentityRejected: true, initialFileBlocked: true, navigationBlocked: true, childFrameBlocked: true, popupDeniedAndMocked: true, redirectBlocked: true }));
})().then(() => {
  clearTimeout(watchdog);
  host?.destroy();
  server?.close();
  app.exit(0);
}).catch(error => {
  clearTimeout(watchdog);
  console.error(error);
  host?.destroy();
  server?.close();
  app.exit(1);
});
`;
try {
  await build({
    stdin: { contents: source
      .replace("BROWSER_GUARDS", JSON.stringify(path.join(root, "src/main/browser-guards.ts")))
      .replace("BROWSER_POLICY", JSON.stringify(path.join(root, "src/main/browser-policy.ts")))
      .replace("APP_SECURITY", JSON.stringify(path.join(root, "src/main/security.ts")))
      .replace("PROFILE", JSON.stringify(path.join(temporary, "profile")))
      .replace("MALICIOUS_PRELOAD", JSON.stringify(maliciousPreload)),
    resolveDir: root, sourcefile: "electron-browser-smoke-entry.cjs" },
    bundle: true, platform: "node", format: "cjs", external: ["electron"], outfile: entry,
    tsconfig: path.join(root, "tsconfig.node.json"), logLevel: "silent"
  });
  const env = { ...process.env };
  delete env.ELECTRON_RUN_AS_NODE;
  const child = spawn(electron, [entry], { env, stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  for (const stream of [child.stdout, child.stderr]) stream.on("data", chunk => { output += chunk; process.stdout.write(chunk); });
  const timeout = setTimeout(() => child.kill("SIGKILL"), 35000);
  const result = await new Promise(resolve => {
    child.once("error", error => resolve({ error }));
    child.once("close", (code, signal) => resolve({ code, signal }));
  });
  clearTimeout(timeout);
  if (result.error) throw result.error;
  if (result.code !== 0 || !output.includes("[electron-browser-smoke]")) throw new Error("Electron browser smoke failed: " + JSON.stringify(result));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
