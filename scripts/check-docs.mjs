#!/usr/bin/env node

import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import ts from "typescript";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (file) => readFile(path.join(root, file), "utf8");
const exists = async (file) => {
  try {
    await stat(path.join(root, file));
    return true;
  } catch {
    return false;
  }
};
const errors = [];
const check = (ok, message) => {
  if (!ok) errors.push(message);
};
const parseSource = async (file) => ts.createSourceFile(
  file,
  await read(file),
  ts.ScriptTarget.Latest,
  true,
  file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS
);
const visit = (node, callback) => {
  callback(node);
  ts.forEachChild(node, (child) => visit(child, callback));
};
const unique = (values) => [...new Set(values)];
const duplicates = (values) => unique(values.filter((value, index) => values.indexOf(value) !== index));
const checkInventory = (document, inventory, values) => {
  for (const value of duplicates(values)) check(false, `${document} ${inventory} contains duplicate row ${value}`);
};
const tableColumn = (markdown, heading, pattern) => {
  const start = markdown.indexOf(heading);
  if (start < 0) return [];
  const end = markdown.indexOf("\n## ", start + heading.length);
  return [...markdown.slice(start, end < 0 ? undefined : end).matchAll(pattern)].map((match) => match[1]);
};

const packageJson = JSON.parse(await read("package.json"));
const docsFiles = (await readdir(path.join(root, "docs")))
  .filter((file) => file.endsWith(".md"))
  .sort()
  .map((file) => `docs/${file}`);
const documentationFiles = ["AGENTS.md", "README.md", ...docsFiles];
const documentation = new Map(await Promise.all(documentationFiles.map(async (file) => [file, await read(file)])));

for (const [file, markdown] of documentation) {
  for (const match of markdown.matchAll(/!?(?:\[[^\]]*\])\(([^)]+)\)/g)) {
    const target = match[1].trim().replace(/^<|>$/g, "").split(/[?#]/, 1)[0];
    if (!target || /^(?:https?:|mailto:|#)/.test(target)) continue;
    const resolved = path.resolve(path.dirname(path.join(root, file)), decodeURIComponent(target));
    check(resolved.startsWith(`${root}${path.sep}`) && await exists(path.relative(root, resolved)), `${file} links missing local target ${target}`);
  }
  for (const match of markdown.matchAll(/`npm run ([\w:-]+)`/g)) {
    check(match[1] in packageJson.scripts, `${file} references missing package script npm run ${match[1]}`);
  }
  for (const match of markdown.matchAll(/`([^`\n]+\.(?:ts|tsx|js|mjs|md|json):\d+(?:[-,]\d+)*)`/g)) {
    check(false, `${file} contains unstable numeric source reference ${match[1]}`);
  }
  for (const table of markdown.matchAll(/(?:^\|.*\|\n){3,}/gm)) {
    const keys = table[0].trim().split("\n").slice(2).map((row) => row.split("|")[1]?.trim()).filter(Boolean);
    for (const key of duplicates(keys)) check(false, `${file} table contains duplicate first-column inventory row ${key}`);
  }
}

check(
  packageJson.scripts.check === "npm run typecheck && npm test && npm run docs:check && npm run architecture:check && npm run build:compile",
  "package.json check script must run typecheck, tests, documented surfaces, architecture fitness, and the compile-only build"
);
check(packageJson.scripts["architecture:check"] === "node scripts/check-architecture.mjs", "Architecture fitness must use the reviewed checker");
check(packageJson.engines?.node === ">=22.23.2 <23", "package.json engines.node must be >=22.23.2 <23");
check((await read(".node-version")).trim() === "22.23.2", ".node-version must select 22.23.2");
const workflow = await read(".github/workflows/check.yml");
check(workflow.includes("node-version-file: .node-version"), "CI must select Node through .node-version");
check(workflow.includes("- run: npm run check"), "CI must run the canonical gate including architecture fitness");
check(documentation.get("README.md").includes("Node 22.23.2"), "README.md must state the supported Node floor");
check(documentation.get("docs/operations.md").includes("Node 22.23.2"), "docs/operations.md must state the supported Node floor");

const agents = documentation.get("AGENTS.md");
const modulePaths = tableColumn(
  agents,
  "## Module map",
  /^\| [^|`]+ \| `([^`|]+)` \|/gm
);
checkInventory("AGENTS.md", "module map", modulePaths);
for (const file of modulePaths) check(await exists(file), `AGENTS.md module map lists missing path ${file}`);

const ipcOwners = ["src/main/index.ts", "src/main/ipc/files.ts", "src/main/ipc/agent.ts", "src/main/ipc/terminal.ts"];
const ipcChannels = [];
for (const owner of ipcOwners) visit(await parseSource(owner), (node) => {
  if (!ts.isCallExpression(node) || node.arguments.length === 0) return;
  const expression = node.expression;
  const registered = ts.isIdentifier(expression) && expression.text === "handleTrusted"
    || ts.isPropertyAccessExpression(expression)
      && ts.isIdentifier(expression.expression)
      && expression.expression.text === "ipcMain"
      && expression.name.text === "handle";
  const channel = node.arguments[0];
  if (registered && ts.isStringLiteral(channel) && channel.text.startsWith("shell:")) ipcChannels.push(channel.text);
});
const mainDoc = documentation.get("docs/main.md");
const mainTable = tableColumn(mainDoc, "## IPC surface", /^\| `(shell:[\w-]+)` \|/gm);
checkInventory("main IPC owners", "IPC registrations", ipcChannels);
checkInventory("docs/main.md", "IPC table", mainTable);
for (const channel of unique(ipcChannels)) check(mainTable.includes(channel), `docs/main.md IPC table is missing ${channel}`);
for (const channel of unique(mainTable)) check(ipcChannels.includes(channel), `docs/main.md lists unregistered IPC channel ${channel}`);

const opencode = await parseSource("src/main/opencode.ts");
const backendClass = opencode.statements.find((node) => ts.isClassDeclaration(node) && node.name?.text === "VelaBackend");
const publicMethods = backendClass?.members
  .filter(ts.isMethodDeclaration)
  .filter((method) => !method.modifiers?.some((modifier) => modifier.kind === ts.SyntaxKind.PrivateKeyword))
  .map((method) => method.name.getText(opencode)) ?? [];
const methodDoc = tableColumn(mainDoc, "Public methods", /^\| `(\w+)\([^)]*\)` \|/gm);
checkInventory("src/main/opencode.ts", "VelaBackend public methods", publicMethods);
checkInventory("docs/main.md", "VelaBackend method table", methodDoc);
for (const method of publicMethods) check(methodDoc.includes(method), `docs/main.md method table is missing ${method}()`);
for (const method of methodDoc) check(publicMethods.includes(method), `docs/main.md lists non-public VelaBackend method ${method}()`);

const preload = await parseSource("src/preload/index.ts");
const apiDeclaration = preload.statements
  .filter(ts.isVariableStatement)
  .flatMap((statement) => [...statement.declarationList.declarations])
  .find((declaration) => ts.isIdentifier(declaration.name) && declaration.name.text === "api");
const apiMembers = apiDeclaration && ts.isObjectLiteralExpression(apiDeclaration.initializer)
  ? apiDeclaration.initializer.properties.map((property) => property.name?.getText(preload)).filter(Boolean)
  : [];
const preloadDoc = documentation.get("docs/preload.md");
const preloadTable = tableColumn(preloadDoc, "## Contract", /^\| `(\w+)(?:\([^)]*\))?` \|/gm);
checkInventory("src/preload/index.ts", "window.vela members", apiMembers);
checkInventory("docs/preload.md", "window.vela contract", preloadTable);
for (const member of apiMembers) check(preloadTable.includes(member), `docs/preload.md contract is missing ${member}`);
for (const member of preloadTable) check(apiMembers.includes(member), `docs/preload.md lists unexposed window.vela member ${member}`);

const eventCases = [];
for (const file of ["src/renderer/src/store.tsx", "src/renderer/src/chat-stream.ts"]) {
  const source = await parseSource(file);
  visit(source, (node) => {
    if (ts.isCaseClause(node) && ts.isStringLiteral(node.expression) && /^[a-z][\w-]*\.[\w.-]+$/.test(node.expression.text)) eventCases.push(node.expression.text);
  });
}
const eventsDoc = documentation.get("docs/events.md");
const handledEvents = tableColumn(eventsDoc, "## Events the renderer handles", /^\| `([\w.-]+)` \|/gm);
const notHandledSection = eventsDoc.slice(eventsDoc.indexOf("## Events forwarded but NOT handled"), eventsDoc.indexOf("## Main-process event handling"));
const notHandledEvents = [...notHandledSection.matchAll(/`((?:session|permission|filesystem|project|plugin|command|skill|mcp|vcs|websearch|pty|question|form|tui|reference|integration|catalog|agent|model|installation|config|usage|shell)\.[\w.*-]+)`/g)].map((match) => match[1]);
checkInventory("docs/events.md", "handled event table", handledEvents);
checkInventory("docs/events.md", "not-handled event inventory", notHandledEvents);
for (const event of unique(eventCases)) {
  const inventoried = handledEvents.includes(event) || notHandledEvents.some((pattern) => pattern.endsWith(".*") ? event.startsWith(pattern.slice(0, -1)) : pattern === event);
  check(inventoried, `docs/events.md does not inventory handled event ${event}`);
}
for (const event of handledEvents) check(eventCases.includes(event), `docs/events.md claims unsupported handled event ${event}`);
for (const event of notHandledEvents.filter((event) => !event.endsWith(".*"))) check(!eventCases.includes(event), `docs/events.md lists handled event ${event} as not handled`);

const shared = await parseSource("src/shared/types.ts");
const backendKinds = [];
visit(shared, (node) => {
  if (!ts.isPropertySignature(node) || node.name?.getText(shared) !== "kind" || !node.type) return;
  const collect = (type) => {
    if (ts.isLiteralTypeNode(type) && ts.isStringLiteral(type.literal)) backendKinds.push(type.literal.text);
    if (ts.isUnionTypeNode(type)) type.types.forEach(collect);
  };
  collect(node.type);
});
const messageKinds = ["event", "file-update", "session", "ui-command", "terminal-data", "terminal-exit"];
const documentedKinds = [...documentation.get("docs/shared.md").matchAll(/"(event|file-update|session|ui-command|terminal-data|terminal-exit)"/g)].map((match) => match[1]);
for (const kind of unique(backendKinds.filter((kind) => messageKinds.includes(kind)))) {
  check(documentedKinds.includes(kind), `docs/shared.md IPC envelope is missing BackendMessage kind ${kind}`);
}

if (errors.length > 0) {
  console.error(`documented surface presence check failed (${errors.length} problem${errors.length === 1 ? "" : "s"}):\n`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}

console.log("documented surface presence check OK");
