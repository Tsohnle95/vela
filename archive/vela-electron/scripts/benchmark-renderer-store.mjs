#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { JSDOM } from "jsdom";

const repository = fileURLToPath(new URL("../", import.meta.url));
const args = process.argv.slice(2);
let reference;
let operation = "streaming";
const selectedOptions = new Set();
for (let index = 0; index < args.length; index += 2) {
  if (!args[index + 1]) throw new Error("Expected a value for benchmark option");
  if (selectedOptions.has(args[index])) throw new Error(`Repeated option: ${args[index]}`);
  selectedOptions.add(args[index]);
  if (args[index] === "--ref" && !reference) reference = args[index + 1];
  else if (args[index] === "--operation") operation = args[index + 1];
  else throw new Error(`Unknown or repeated option: ${args[index]}`);
}
if (!["streaming", "workspace-files", "session-inbox", "session-requests", "session-progress"].includes(operation)) throw new Error(`Unknown operation: ${operation}`);
const supportedNode = readFileSync(path.join(repository, ".node-version"), "utf8").trim();
if (process.versions.node !== supportedNode) throw new Error(`Use Node ${supportedNode}; active version is ${process.versions.node}`);
const git = (...gitArgs) => execFileSync("git", gitArgs, { cwd: repository, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }).trim();
const ref = reference ? git("rev-parse", "--verify", `${reference}^{commit}`) : null;
const files = ref ? new Set(git("ls-tree", "-r", "--name-only", ref, "src").split("\n")) : null;
const plugins = ref ? [{
  name: "historical-store-source",
  setup(builder) {
    builder.onResolve({ filter: /.*/ }, (entry) => {
      const directory = entry.namespace === "git-source" ? path.posix.dirname(entry.importer)
        : path.relative(repository, entry.resolveDir).split(path.sep).join("/");
      const modulePath = entry.path.startsWith("@shared/") ? `src/shared/${entry.path.slice(8)}`
        : entry.path.startsWith(".") ? path.posix.normalize(path.posix.join(directory, entry.path))
          : path.isAbsolute(entry.path) ? path.relative(repository, entry.path).split(path.sep).join("/") : null;
      if (!modulePath?.startsWith("src/")) return { path: entry.path, external: true };
      const resolved = [modulePath, `${modulePath}.ts`, `${modulePath}.tsx`, `${modulePath}.js`, `${modulePath}/index.ts`]
        .find((candidate) => files.has(candidate));
      if (!resolved) throw new Error(`Missing ${modulePath} at ${ref}`);
      return { path: resolved, namespace: "git-source" };
    });
    builder.onLoad({ filter: /.*/, namespace: "git-source" }, (entry) => ({
      contents: git("show", `${ref}:${entry.path}`), loader: entry.path.endsWith(".tsx") ? "tsx" : "ts"
    }));
  }
}] : [];
const built = await build({
  stdin: { contents: 'export { StoreProvider, useStore } from "./src/renderer/src/store.tsx"; export { largeSessionEvents, retainedOutputChars } from "./src/renderer/src/large-session-fixture.ts";', resolveDir: repository },
  bundle: true, platform: "node", format: "cjs", write: false, jsx: "automatic", packages: "external",
  alias: { "@shared": path.join(repository, "src/shared") }, plugins
});
const require = createRequire(path.join(repository, "package.json"));
const compiled = { exports: {} };
new Function("require", "module", "exports", built.outputFiles[0].text)(require, compiled, compiled.exports);
const { StoreProvider, useStore, largeSessionEvents, retainedOutputChars } = compiled.exports;
const React = require("react");
const { createRoot } = require("react-dom/client");
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

async function sample() {
  const dom = new JSDOM('<div id="root"></div>', { url: "http://fixture.local" });
  for (const name of ["window", "document", "HTMLElement", "localStorage"]) globalThis[name] = dom.window[name];
  let handler, store, renders = 0;
  const session = { id: "session", directory: "/fixture", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
  const queueKey = `${session.workspace.id}\n${session.id}`;
  let queueWrites = 0;
  let approvalWrites = 0;
  const cancelled = [];
  const formReplies = [], permissionReplies = [];
  let providerReads = 0;
  const providerResult = [{ provider: "openai", displayName: "OpenAI", status: "ok", snapshot: null }];
  if (operation === "session-inbox") {
    localStorage.setItem("messageQueue", JSON.stringify({ followUpBehavior: "queue", quarantinedLegacyMessages: {},
      queuedMessages: { [queueKey]: Array.from({ length: 20 }, (_, index) => ({ id: `queued-${index}`, content: `Local ${index}`, createdAt: index + 1 })) } }));
    const setItem = dom.window.Storage.prototype.setItem;
    dom.window.Storage.prototype.setItem = function (key, value) {
      if (key === "messageQueue") queueWrites++;
      return setItem.call(this, key, value);
    };
  }
  if (operation === "session-requests") {
    const setItem = dom.window.Storage.prototype.setItem;
    dom.window.Storage.prototype.setItem = function (key, value) {
      if (key === "approvalMode") approvalWrites++;
      return setItem.call(this, key, value);
    };
  }
  const activeSessions = [session];
  const empty = async () => [];
  let directoryReads = 0;
  const writes = [];
  const entries = operation === "workspace-files" ? Array.from({ length: 1000 }, (_, index) => ({ path: `entry-${999 - index}`, type: index % 5 ? "file" : "directory" })) : [];
  if (operation === "workspace-files") entries.push({ path: "node_modules", type: "directory" }, { path: ".DS_Store", type: "file" });
  const entryOrder = JSON.stringify(entries);
  const diskText = (file) => `Disk content for ${file}\n`;
  window.vela = {
    platform: "darwin", onMessage: (callback) => { handler = callback; return () => { handler = null; }; },
    health: async () => true, takePendingPaths: empty, state: async () => null,
    activeSessions: operation !== "streaming" ? async () => activeSessions : empty,
    models: empty, modelDefault: async () => null, sessionSelection: async () => null, agents: empty, sessions: empty,
    openSession: async () => session, openSessionById: async () => ({ session, transcript: [], todos: [] }),
    closeSession: async () => {}, readFile: async (_workspace, file) => operation === "workspace-files" ? diskText(file) : "",
    listDir: async () => { directoryReads++; return operation === "workspace-files" ? entries : []; }, listPermissions: empty,
    writeFile: async (...args) => { writes.push(args); },
    runtimes: empty, projects: empty, providerUsage: async () => { providerReads++; return operation === "session-progress" ? providerResult : null; },
    savedWorkspaces: async () => ({ initialized: true, workspaces: [] }), saveSavedWorkspaces: async () => {},
    sessionUsage: async () => null, sessionTranscript: async () => ({ transcript: [], todos: [] }),
    inboxList: empty, formsList: empty, recoveryList: empty, reportUnsavedEditorFiles: () => {},
    ...(operation !== "streaming" ? { recoveryRecords: empty } : {}),
    ...(operation === "session-inbox" ? { inboxCancel: async (...args) => { cancelled.push(args); } } : {}),
    ...(operation === "session-requests" ? {
      formReply: async (...args) => { formReplies.push(args); },
      permissionReply: async (...args) => { permissionReplies.push(args); }
    } : {})
  };
  function Probe() { store = useStore(); renders++; return null; }
  const root = createRoot(document.querySelector("#root"));
  try {
    await React.act(async () => root.render(React.createElement(StoreProvider, null, React.createElement(Probe))));
    let start = performance.now();
    await React.act(async () => store.addModelPanel("/fixture"));
    const openMs = performance.now() - start;
    if (operation === "session-progress") {
      const previousRenders = renders;
      const emit = (type, data, index) => handler({ kind: "event", type,
        data: { id: `${type}-${index}`, created: index + 1, data: { sessionID: session.id, ...data } } });
      start = performance.now();
      for (let index = 0; index < 100; index++) {
        await React.act(async () => emit("session.usage.updated", { cost: index / 100,
          tokens: { input: 20000 + index, output: index, reasoning: 2, cache: { read: 3, write: 4 } } }, index));
        if (store.sessionUsage?.tokens.input !== 20000 + index || store.sessionUsage.cost !== index / 100) throw new Error("Usage normalization or ownership changed");
      }
      const usageMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 100; index++) {
        await React.act(async () => emit("todo.updated", { todos: [{ id: `todo-${index}`, content: `Task ${index}`, status: "pending", priority: "low" }] }, index));
        if (store.todos.length !== 1 || store.todos[0].id !== `todo-${index}`) throw new Error("Workspace todo projection changed");
      }
      const todosMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 100; index++) await React.act(async () => store.refreshProviderUsage());
      const providerMs = performance.now() - start;
      const view = store.panelViews[session.workspace.id];
      if (providerReads !== 100 || store.providerUsage !== providerResult || store.providerUsageLoading ||
        view.sessionUsage.tokens.input !== 20099 || view.todos[0].content !== "Task 99" ||
        store.panels[0].id !== session.id || JSON.parse(localStorage.getItem("compactionBaseline"))[session.id] !== undefined) throw new Error("Progress final state or persistence changed");
      return { openMs, usageMs, todosMs, providerMs, updateRenders: renders - previousRenders,
        providerReads, finalInputTokens: view.sessionUsage.tokens.input, todoCount: view.todos.length, providerCount: store.providerUsage.length };
    }
    if (operation === "session-requests") {
      const previousRenders = renders;
      const emit = (type, data, id) => handler({ kind: "event", type,
        data: { id: `${type}-${id}`, created: 1, velaSessionIDs: [session.id], data: { sessionID: session.id, ...data } } });
      start = performance.now();
      for (let index = 0; index < 100; index++) {
        const id = `form-${index}`;
        await React.act(async () => emit("form.created", { form: { id, sessionID: "global", title: id,
          fields: [{ key: "answer", type: "string", required: true }] } }, id));
        const pending = store.panelViews[session.workspace.id].pendingForms;
        if (pending.length !== 1 || pending[0].sessionID !== "global" || pending[0].fields[0].key !== "answer") throw new Error("Form attribution or fields changed");
        await React.act(async () => store.submitForm(session.workspace, id, { answer: `Answer ${index}` }, "global"));
        if (store.panelViews[session.workspace.id].pendingForms.length) throw new Error("Form submission removal changed");
      }
      const formsMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 100; index++) {
        const id = `permission-${index}`;
        await React.act(async () => emit("permission.asked", { id, action: "edit", resources: ["notes.txt"] }, id));
        await React.act(async () => store.replyPermission(id, "once", session.id));
      }
      const permissionsMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 100; index++) await React.act(async () => store.toggleApprovalMode());
      const preferenceMs = performance.now() - start;
      const rows = store.transcript.filter((item) => item.kind === "permission");
      if (rows.length !== 100 || rows.some((item, index) => item.requestID !== `permission-${index}` || item.pending || item.resolvedWith !== "once")) throw new Error("Permission rows changed");
      if (formReplies.length !== 100 || formReplies.some((args, index) => args[0] !== session.workspace || args[1] !== `form-${index}` || args[2].answer !== `Answer ${index}` || args[3] !== "global")) throw new Error("Form reply contract changed");
      if (permissionReplies.length !== 100 || permissionReplies.some((args, index) => args[0] !== session.workspace || args[1] !== `permission-${index}` || args[2] !== "once" || args[3] !== session.id)) throw new Error("Permission reply contract changed");
      if (approvalWrites !== 100 || localStorage.getItem("approvalMode") !== "ask" || store.approvalMode !== "ask") throw new Error("Approval persistence changed");
      return { openMs, formsMs, permissionsMs, preferenceMs, updateRenders: renders - previousRenders,
        formReplies: formReplies.length, permissionReplies: permissionReplies.length, approvalWrites, permissionRows: rows.length };
    }
    if (operation === "session-inbox") {
      const previousRenders = renders;
      start = performance.now();
      for (let index = 0; index < 100; index++) {
        const inboxID = `native-${index}`;
        const emit = (type, data) => handler({ kind: "event", type,
          data: { id: `${type}-${index}`, created: index + 1, data: { sessionID: session.id, inboxID, ...data } } });
        await React.act(async () => emit("session.inbox.enqueued", {
          delivery: "queue", item: { type: "user", payload: { text: `Native ${index}`, files: [] } }
        }));
        let popped;
        await React.act(async () => { popped = store.popQueuedMessage(session.workspace, inboxID); });
        if (popped?.id !== inboxID || popped.content !== `Native ${index}`) throw new Error("Native inbox pop changed");
        await React.act(async () => emit("session.inbox.cancelled", {}));
      }
      const inboxMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 100; index++) await React.act(async () => store.setFollowUpBehavior(index % 2 ? "queue" : "steer"));
      for (let index = 0; index < 20; index++) {
        let popped;
        await React.act(async () => { popped = store.popQueuedMessage(session.workspace, `queued-${index}`); });
        if (popped?.content !== `Local ${index}`) throw new Error("Local queue pop changed");
      }
      await React.act(async () => {
        if (store.popQueuedMessage(session.workspace, "queued-missing") !== null) throw new Error("Missing local pop changed");
        store.reorderQueuedMessage(session.workspace, "queued-missing", "queued-also-missing");
      });
      const localQueueMs = performance.now() - start;
      const persisted = JSON.parse(localStorage.getItem("messageQueue"));
      if (queueWrites !== 122 || persisted.followUpBehavior !== "queue" || (persisted.queuedMessages[queueKey] ?? []).length) throw new Error("Queue persistence changed");
      if (cancelled.length !== 100 || cancelled.some((args, index) => args[0] !== session.workspace || args[1] !== `native-${index}`)) throw new Error("Inbox cancellation contract changed");
      return { openMs, inboxMs, localQueueMs, updateRenders: renders - previousRenders, queueWrites, cancellationCount: cancelled.length };
    }
    if (operation === "workspace-files") {
      const previousRenders = renders;
      start = performance.now();
      for (let index = 0; index < 500; index++) {
        const file = `group/file-${index}.txt`;
        await React.act(async () => handler({ kind: "file-update", file: {
          workspace: session.workspace, sessionID: session.id, path: file,
          baseline: { kind: "known", content: `Baseline ${index}` }, content: diskText(file), deleted: false
        } }));
      }
      const observationsMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 20; index++) await React.act(async () => store.openFile(`group/file-${index}.txt`));
      const editorOpenMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 10; index++) {
        const file = `group/file-${index}.txt`;
        await React.act(async () => store.editContent(file, `Edited ${index}`));
        await React.act(async () => store.saveTab(file));
      }
      const editorSaveMs = performance.now() - start;
      start = performance.now();
      for (let index = 0; index < 100; index++) await React.act(async () => store.ensureRootOpen());
      const directoryMs = performance.now() - start;
      for (let index = 0; index < 3; index++) await React.act(async () => store.toggleDir("cached"));
      const expected = [...entries].filter((entry) => !["node_modules", ".DS_Store"].includes(entry.path))
        .sort((a, b) => a.type !== b.type ? a.type === "directory" ? -1 : 1 : a.path.localeCompare(b.path));
      if (directoryReads !== 101 || JSON.stringify(entries) !== entryOrder || JSON.stringify(store.tree[""]) !== JSON.stringify(expected)) throw new Error("Directory workload changed");
      if (store.agentFiles.size !== 500 || store.tabs.length !== 20 || store.tabs.some((tab) => tab.dirty)) throw new Error("File/editor workload changed");
      for (let index = 0; index < 20; index++) {
        const tab = store.tabs.find((tab) => tab.path === `group/file-${index}.txt`);
        if (tab.baseline.content !== `Baseline ${index}` || tab.content !== (index < 10 ? `Edited ${index}` : diskText(tab.path))) throw new Error("Editor content or baseline changed");
      }
      if (writes.length !== 10 || writes.some((args, index) => args[1] !== `group/file-${index}.txt` || args[2] !== `Edited ${index}` || args[3].expectedContent !== diskText(args[1]))) throw new Error("Save expectations changed");
      return { openMs, observationsMs, editorOpenMs, editorSaveMs, directoryMs, updateRenders: renders - previousRenders,
        observedFiles: store.agentFiles.size, editorTabs: store.tabs.length, directoryReads, writeCount: writes.length };
    }
    await React.act(async () => {
      for (const event of largeSessionEvents(100)) handler({ kind: "event", type: event.type, data: event });
    });
    const emit = (type, data, id) => handler({ kind: "event", type, data: { id, created: Date.now(), data: { sessionID: session.id, ...data } } });
    await React.act(async () => {
      emit("session.step.started", { assistantMessageID: "zz-tail" }, "start");
      emit("session.text.started", { assistantMessageID: "zz-tail", ordinal: 0 }, "text");
    });
    const previousRenders = renders;
    start = performance.now();
    for (let index = 0; index < 100; index++) {
      await React.act(async () => emit("session.text.delta", { assistantMessageID: "zz-tail", ordinal: 0, delta: " token" }, `delta-${index}`));
    }
    const streamingMs = performance.now() - start;
    const assistants = store.transcript.filter((item) => item.kind === "assistant");
    const outputChars = retainedOutputChars(store.transcript);
    if (assistants.length !== 101 || outputChars !== 100 * 32768 || assistants.at(-1).parts[0].text !== " token".repeat(100)) throw new Error("Benchmark transcript content changed");
    return { openMs, streamingMs, updateRenders: renders - previousRenders, assistantCount: assistants.length, outputChars };
  } finally {
    await React.act(async () => root.unmount());
    dom.window.close();
  }
}
for (let index = 0; index < 3; index++) await sample();
const samples = [];
for (let index = 0; index < 9; index++) samples.push(await sample());
const distribution = (field) => {
  const sorted = samples.map((sample) => sample[field]).sort((a, b) => a - b);
  return { min: sorted[0], median: sorted[4], max: sorted[8] };
};
console.log(JSON.stringify({
  source: ref ?? "working tree", head: git("rev-parse", "HEAD"), node: process.version,
  bundleSha256: createHash("sha256").update(built.outputFiles[0].text).digest("hex"),
  scope: "Actual Store processing and React/jsdom context consumer commits; mocked bridge; excludes filesystem, IPC, provider latency, Chromium layout and paint",
  operation,
  workload: operation === "streaming" ? { assistants: 100, setupEvents: 600, toolOutputChars: 32768, deltas: 100, warmups: 3, samples: 9 }
    : operation === "session-progress" ? { usageEvents: 100, todoEvents: 100, providerRefreshes: 100, warmups: 3, samples: 9 }
    : operation === "session-requests" ? { formRequests: 100, permissionRequests: 100, approvalToggles: 100, warmups: 3, samples: 9 }
    : operation === "session-inbox" ? { nativeEnqueuesAndPops: 100, followUpChanges: 100, localPops: 20, missingPop: 1, noOpReorder: 1, warmups: 3, samples: 9 }
      : { fileUpdates: 500, editorOpens: 20, editorSaves: 10, rootReads: 100, listingEntries: 1002, cachedToggles: 3, warmups: 3, samples: 9 },
  ...Object.fromEntries((operation === "streaming" ? ["openMs", "streamingMs"]
    : operation === "session-progress" ? ["openMs", "usageMs", "todosMs", "providerMs"]
    : operation === "session-requests" ? ["openMs", "formsMs", "permissionsMs", "preferenceMs"]
    : operation === "session-inbox" ? ["openMs", "inboxMs", "localQueueMs"]
      : ["openMs", "observationsMs", "editorOpenMs", "editorSaveMs", "directoryMs"]).map((field) => [field, distribution(field)])), samples
}, null, 2));
