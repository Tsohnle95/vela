#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readdir, readFile, mkdir, writeFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import pinnedStyles from "./pinned-upstream-styles.json" with { type: "json" };
import pinnedAiElements from "./pinned-upstream-ai-elements.json" with { type: "json" };

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const PRODUCTION_ROOTS = ["src/main", "src/preload", "src/renderer/src", "src/shared"];
const sourceExtension = /\.(?:[cm]?[jt]sx?|css|scss)$/;
const scriptExtension = /\.(?:[cm]?[jt]sx?)$/;
const testFile = /(?:^|\/)(?:__tests__|fixtures?|__fixtures__)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$|(?:^|\/)large-session-fixture\.ts$/;
const hash = (value) => createHash("sha256").update(value).digest("hex");
const portable = (value) => value.split(path.sep).join("/");

export function validatePinnedStyles(manifest) {
  const files = manifest?.files;
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest) ||
    typeof manifest.upstreamCommit !== "string" || !/^[a-f0-9]{40}$/.test(manifest.upstreamCommit) ||
    typeof manifest.provenance !== "string" || !manifest.provenance.trim() ||
    !files || typeof files !== "object" || Array.isArray(files) || !Object.keys(files).length ||
    !Object.entries(files).every(([file, digest]) => /^src\/renderer\/src\/styles\/opencode\/[a-z0-9-]+\.(?:css|scss)$/.test(file) &&
      typeof digest === "string" && /^[a-f0-9]{64}$/.test(digest))) {
    throw new Error("Invalid frozen upstream style inventory: require normalized stylesheet paths, lowercase SHA256 hashes and upstream provenance");
  }
}

validatePinnedStyles(pinnedStyles);

export function validatePinnedAiElements(manifest) {
  const allowed = /^src\/renderer\/src\/(?:components\/ui\/[a-z0-9-]+\.tsx|ai-elements\/(?:demos\/[a-z0-9-]+\.tsx|demos\/lib\/(?:markdown\.tsx|message-animations\.ts)|upstream\/(?:hooks\/use-highlighted-code\.ts|lib\/highlighter\.tsx)))$/;
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest) ||
    !/^https:\/\/github\.com\/shadcn-ui\/ui\/tree\/[a-f0-9]{40}$/.test(manifest.upstreamSnapshot ?? "") ||
    manifest.sourceManifest !== "src/renderer/src/ai-elements/upstream/manifest.json" ||
    typeof manifest.provenance !== "string" || !manifest.provenance.trim() ||
    !manifest.files || typeof manifest.files !== "object" || Array.isArray(manifest.files) ||
    !Object.keys(manifest.files).length || !Object.entries(manifest.files).every(([file, digest]) =>
      allowed.test(file) && typeof digest === "string" && /^[a-f0-9]{64}$/.test(digest))) {
    throw new Error("Invalid frozen AI Elements inventory: require explicit upstream source paths, lowercase SHA256 hashes and pinned source provenance");
  }
}

validatePinnedAiElements(pinnedAiElements);

export function pinnedAiElementDrift(inventory, manifest = pinnedAiElements) {
  const actual = new Map(inventory.map((file) => [file.path, file.sha256]));
  return Object.entries(manifest.files).filter(([file, expected]) => actual.get(file) !== expected)
    .map(([file, expected]) => ({ file, expected, actual: actual.get(file) ?? "missing" }));
}

export function sourceCategory(file) {
  if (!sourceExtension.test(file)) return "other";
  if (testFile.test(file)) return "tests-and-fixtures";
  if (Object.hasOwn(pinnedStyles.files, file)) return "pinned-upstream-styles";
  if (Object.hasOwn(pinnedAiElements.files, file)) return "pinned-upstream-ai-elements";
  if (PRODUCTION_ROOTS.some((root) => file.startsWith(`${root}/`))) return "production";
  if (file.startsWith("scripts/")) return "build-and-verification-scripts";
  if (/^(?:mockup-design|new design mock-up|design|assets)\//.test(file)) return "design-and-mockup-code";
  if (/(?:^|\/)plugins?\//.test(file)) return "plugins";
  return "auxiliary-code";
}

function sourceFile(file, content) {
  return ts.createSourceFile(file, content, ts.ScriptTarget.Latest, true,
    /\.[jt]sx$/.test(file) ? ts.ScriptKind.TSX : /\.[cm]?js$/.test(file) ? ts.ScriptKind.JS : ts.ScriptKind.TS);
}

function tokens(node, source, result = []) {
  if (node.kind >= ts.SyntaxKind.FirstJSDocNode && node.kind <= ts.SyntaxKind.LastJSDocNode) return result;
  const children = node.getChildren(source);
  if (children.length === 0 && node.kind >= ts.SyntaxKind.FirstToken && node.kind <= ts.SyntaxKind.LastToken && node.kind !== ts.SyntaxKind.EndOfFileToken) {
    const start = node.getStart(source);
    if (node.end > start) result.push({ start, end: node.end, text: source.text.slice(start, node.end), kind: node.kind });
  } else {
    for (const child of children) tokens(child, source, result);
  }
  return result;
}

export function styleCode(content, scss = true) {
  let result = "";
  let quote = "";
  let comment = "";
  for (let index = 0; index < content.length; index += 1) {
    const char = content[index];
    const next = content[index + 1];
    if (comment) {
      if (comment === "block" && char === "*" && next === "/") { result += "  "; index += 1; comment = ""; }
      else if (char === "\n") { result += char; if (comment === "line") comment = ""; }
      else result += " ";
    } else if (quote) {
      result += char;
      if (char === "\\" && next !== undefined) { result += next; index += 1; }
      else if (char === quote) quote = "";
    } else if (char === "\"" || char === "'") { quote = char; result += char; }
    else if (char === "/" && next === "*") { result += "  "; index += 1; comment = "block"; }
    else if (scss && char === "/" && next === "/" && content[index - 1] !== ":") { result += "  "; index += 1; comment = "line"; }
    else result += char;
  }
  return result;
}

function tokenCodeLines(node, source) {
  const lines = new Set();
  for (const token of tokens(node, source)) {
    let position = token.start;
    for (const line of token.text.split("\n")) {
      if (line.trim()) lines.add(source.getLineAndCharacterOfPosition(position).line);
      position += line.length + 1;
    }
  }
  return lines.size;
}

export function countCodeLines(content, file) {
  if (/\.(?:css|scss)$/.test(file)) return styleCode(content, file.endsWith(".scss")).split("\n").filter((line) => line.trim()).length;
  const source = sourceFile(file, content);
  return tokenCodeLines(source, source);
}

function walk(node, callback) {
  callback(node);
  ts.forEachChild(node, (child) => walk(child, callback));
}

function isFunction(node) {
  return Boolean(node) && (ts.isFunctionDeclaration(node) || ts.isFunctionExpression(node) || ts.isArrowFunction(node) ||
    ts.isMethodDeclaration(node) || ts.isConstructorDeclaration(node) || ts.isGetAccessorDeclaration(node) || ts.isSetAccessorDeclaration(node));
}

function functionName(node, source) {
  if (node.name) return node.name.getText(source);
  if (ts.isConstructorDeclaration(node)) return "constructor";
  if (ts.isVariableDeclaration(node.parent) || ts.isPropertyAssignment(node.parent)) return node.parent.name.getText(source);
  if (ts.isJsxExpression(node.parent) && ts.isJsxAttribute(node.parent.parent)) {
    const attribute = node.parent.parent;
    const element = attribute.parent.parent;
    const tag = ts.isJsxOpeningElement(element) || ts.isJsxSelfClosingElement(element) ? element.tagName.getText(source) : "jsx";
    return `${tag}.${attribute.name.getText(source)} callback`;
  }
  if (ts.isCallExpression(node.parent)) {
    const channel = node.parent.arguments[0];
    if (channel && ts.isStringLiteral(channel)) return `${node.parent.expression.getText(source).slice(0, 80)} callback:${channel.text}`;
    if (ts.isVariableDeclaration(node.parent.parent)) return node.parent.parent.name.getText(source);
    const first = node.body && ts.isBlock(node.body) ? node.body.statements[0] : null;
    const anchor = first && ts.isVariableStatement(first) ? first.declarationList.declarations[0]?.name.getText(source) : null;
    return `${node.parent.expression.getText(source).slice(0, 80)} callback${anchor ? `:${anchor}` : ""}`;
  }
  return "anonymous";
}

export function functionIdentity(node, source) {
  const owners = [];
  for (let parent = node.parent; parent; parent = parent.parent) {
    if (isFunction(parent)) owners.unshift(functionName(parent, source));
    else if (ts.isClassDeclaration(parent) && parent.name) owners.unshift(parent.name.getText(source));
  }
  return [...owners, functionName(node, source)].join("/");
}

export function functionMetrics(node, source) {
  let complexity = 1;
  let maxNesting = 0;
  const visit = (current, depth) => {
    if (current !== node && isFunction(current)) return;
    const decision = ts.isIfStatement(current) || ts.isForStatement(current) || ts.isForInStatement(current) || ts.isForOfStatement(current) ||
      ts.isWhileStatement(current) || ts.isDoStatement(current) || ts.isCatchClause(current) || ts.isConditionalExpression(current) || ts.isCaseClause(current);
    const logical = ts.isBinaryExpression(current) && [ts.SyntaxKind.AmpersandAmpersandToken, ts.SyntaxKind.BarBarToken, ts.SyntaxKind.QuestionQuestionToken].includes(current.operatorToken.kind);
    if (decision || logical) complexity += 1;
    const nested = decision || ts.isSwitchStatement(current);
    const nextDepth = depth + Number(nested);
    maxNesting = Math.max(maxNesting, nextDepth);
    ts.forEachChild(current, (child) => visit(child, nextDepth));
  };
  visit(node.body, 0);
  const start = source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1;
  const end = source.getLineAndCharacterOfPosition(node.end).line + 1;
  return {
    name: functionName(node, source), identity: functionIdentity(node, source), line: start, endLine: end, physicalLines: end - start + 1,
    codeLines: tokenCodeLines(node, source), nesting: maxNesting, cyclomatic: complexity
  };
}

export function dependencyCycles(files, dependencies, includeTypeOnly = true) {
  const graph = new Map(files.map((file) => [file, []]));
  for (const edge of dependencies) {
    if ((includeTypeOnly || !edge.typeOnly) && graph.has(edge.from) && graph.has(edge.to)) graph.get(edge.from).push(edge.to);
  }
  let nextIndex = 0;
  const indices = new Map();
  const low = new Map();
  const stack = [];
  const stacked = new Set();
  const cycles = [];
  const visit = (file) => {
    indices.set(file, nextIndex); low.set(file, nextIndex); nextIndex += 1;
    stack.push(file); stacked.add(file);
    for (const dependency of graph.get(file)) {
      if (!indices.has(dependency)) { visit(dependency); low.set(file, Math.min(low.get(file), low.get(dependency))); }
      else if (stacked.has(dependency)) low.set(file, Math.min(low.get(file), indices.get(dependency)));
    }
    if (low.get(file) !== indices.get(file)) return;
    const component = [];
    let member;
    do { member = stack.pop(); stacked.delete(member); component.push(member); } while (member !== file);
    if (component.length > 1 || graph.get(file).includes(file)) cycles.push(component.sort());
  };
  for (const file of files) if (!indices.has(file)) visit(file);
  return cycles.sort((a, b) => a[0].localeCompare(b[0]));
}

async function allFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true }).catch(() => []);
  const children = await Promise.all(entries.map(async (entry) => {
    const file = path.join(directory, entry.name);
    return entry.isDirectory() ? allFiles(file) : entry.isFile() ? [file] : [];
  }));
  return children.flat();
}

async function buildSize(root) {
  const files = await allFiles(path.join(root, "out"));
  const inventory = await Promise.all(files.map(async (file) => ({ path: portable(path.relative(root, file)), bytes: (await stat(file)).size })));
  inventory.sort((a, b) => a.path.localeCompare(b.path));
  return { logicalBytes: inventory.reduce((sum, file) => sum + file.bytes, 0), files: inventory.length, inventory };
}

export function importSpecifiers(source) {
  const imports = [];
  walk(source, (node) => {
    if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
      const clause = node.importClause;
      const bindings = clause?.namedBindings;
      const typeOnly = Boolean(clause?.isTypeOnly || !clause?.name && bindings && ts.isNamedImports(bindings) && bindings.elements.length > 0 && bindings.elements.every((binding) => binding.isTypeOnly));
      imports.push({ specifier: node.moduleSpecifier.text, typeOnly });
    } else if (ts.isExportDeclaration(node) && node.moduleSpecifier && ts.isStringLiteral(node.moduleSpecifier)) {
      const bindings = node.exportClause;
      const typeOnly = node.isTypeOnly || Boolean(bindings && ts.isNamedExports(bindings) && bindings.elements.length > 0 && bindings.elements.every((binding) => binding.isTypeOnly));
      imports.push({ specifier: node.moduleSpecifier.text, typeOnly });
    } else if (ts.isImportTypeNode(node) && ts.isLiteralTypeNode(node.argument) && ts.isStringLiteral(node.argument.literal)) {
      imports.push({ specifier: node.argument.literal.text, typeOnly: true });
    } else if (ts.isImportEqualsDeclaration(node) && ts.isExternalModuleReference(node.moduleReference)) {
      const argument = node.moduleReference.expression;
      if (argument && ts.isStringLiteralLike(argument)) imports.push({ specifier: argument.text, typeOnly: node.isTypeOnly });
    } else if (ts.isCallExpression(node) && (node.expression.kind === ts.SyntaxKind.ImportKeyword || ts.isIdentifier(node.expression) && node.expression.text === "require")) {
      const argument = node.arguments[0];
      if (argument && ts.isStringLiteralLike(argument)) imports.push({ specifier: argument.text, typeOnly: false });
      else imports.push({ computed: node.getText(source), line: source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1 });
    }
  });
  return imports;
}

export function ipcAuthorityReferences(source) {
  const references = [];
  walk(source, (node) => {
    const ipcName = ts.isIdentifier(node) && node.text === "ipcMain" || ts.isStringLiteralLike(node) && node.text === "ipcMain";
    if (ipcName) references.push({ line: source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1, kind: "raw-ipc" });
    if (!source.fileName.replaceAll("\\", "/").includes("/ipc/")) return;
    if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier) && node.moduleSpecifier.text === "electron") {
      const clause = node.importClause;
      const bindings = clause?.namedBindings;
      const onlyInvocationType = source.fileName.replaceAll("\\", "/").endsWith("/ipc/registration.ts") && !clause?.name &&
        bindings && ts.isNamedImports(bindings) && bindings.elements.length === 1 &&
        (clause?.isTypeOnly || bindings.elements[0].isTypeOnly) && (bindings.elements[0].propertyName ?? bindings.elements[0].name).text === "IpcMainInvokeEvent";
      if (!onlyInvocationType) references.push({ line: source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1, kind: "ipc-domain-electron-import" });
    }
    if (ts.isCallExpression(node) && (node.expression.kind === ts.SyntaxKind.ImportKeyword || ts.isIdentifier(node.expression) && node.expression.text === "require") &&
      node.arguments[0] && ts.isStringLiteralLike(node.arguments[0]) && node.arguments[0].text === "electron") {
      references.push({ line: source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1, kind: "ipc-domain-electron-import" });
    }
    if (ts.isImportEqualsDeclaration(node) && ts.isExternalModuleReference(node.moduleReference) &&
      node.moduleReference.expression && ts.isStringLiteralLike(node.moduleReference.expression) && node.moduleReference.expression.text === "electron") {
      references.push({ line: source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1, kind: "ipc-domain-electron-import" });
    }
  });
  return references;
}

export function ownershipRoutes(document) {
  const section = document.split("## Implementation ownership and fitness\n")[1]?.split(/\n## /)[0] ?? "";
  const routes = [];
  for (const line of section.split("\n")) {
    if (!line.startsWith("|")) continue;
    const [owner, entry] = line.split("|").slice(1).map((cell) => cell.trim());
    for (const match of entry?.matchAll(/`(src\/[^`]+)`/g) ?? []) routes.push({ owner, path: match[1] });
  }
  return routes;
}

function resolveLocal(specifier, from, options, files, root, host) {
  const resolved = ts.resolveModuleName(specifier, path.join(root, from), options, host).resolvedModule?.resolvedFileName;
  if (resolved) {
    const rel = portable(path.relative(root, resolved));
    if (files.has(rel) || !rel.startsWith("../") && !rel.includes("node_modules/") && sourceExtension.test(rel)) return rel;
  }
  const absolute = specifier.startsWith("@shared/") ? path.join(root, "src/shared", specifier.slice(8))
    : specifier.startsWith("@/") ? path.join(root, "src/renderer/src", specifier.slice(2))
      : specifier.startsWith(".") ? path.resolve(root, path.dirname(from), specifier) : null;
  if (!absolute) return null;
  const relative = portable(path.relative(root, absolute));
  const directory = path.posix.dirname(relative);
  const name = path.posix.basename(relative);
  return [relative, ...[".ts", ".tsx", ".js", ".scss", ".css"].map((extension) => relative + extension), `${relative}/index.ts`,
    `${directory}/_${name}.scss`, `${directory}/_${name.replace(/\.scss$/, "")}.scss`].find((candidate) => files.has(candidate)) ?? null;
}

function layer(file) {
  return file.startsWith("src/main/") ? "main" : file.startsWith("src/preload/") ? "preload" : file.startsWith("src/shared/") ? "shared" : "renderer";
}

function canonicalSymbol(checker, symbol) {
  if (!symbol) return null;
  return symbol.flags & ts.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol;
}

function exportCandidates(program, primary, root) {
  const checker = program.getTypeChecker();
  const exports = [];
  const symbolExports = new Map();
  for (const file of primary) {
    const source = program.getSourceFile(file.absolute);
    const module = source && checker.getSymbolAtLocation(source);
    if (!module) continue;
    for (const symbol of checker.getExportsOfModule(module)) {
      const canonical = canonicalSymbol(checker, symbol);
      const declaration = canonical?.declarations?.[0];
      if (!declaration || portable(path.relative(root, declaration.getSourceFile().fileName)) !== file.path) continue;
      const entry = { file: file.path, export: symbol.name, line: source.getLineAndCharacterOfPosition(declaration.getStart(source)).line + 1, productionConsumers: new Set(), testOrAuxiliaryConsumers: new Set() };
      exports.push(entry);
      const entries = symbolExports.get(canonical) ?? [];
      entries.push(entry); symbolExports.set(canonical, entries);
    }
  }
  for (const source of program.getSourceFiles()) {
    const consumer = portable(path.relative(root, source.fileName));
    if (consumer.startsWith("../") || consumer.includes("node_modules/")) continue;
    walk(source, (node) => {
      if (!ts.isIdentifier(node)) return;
      const symbol = canonicalSymbol(checker, checker.getSymbolAtLocation(node));
      for (const exported of symbolExports.get(symbol) ?? []) {
        if (consumer === exported.file) continue;
        if (sourceCategory(consumer) === "production") exported.productionConsumers.add(consumer);
        else exported.testOrAuxiliaryConsumers.add(consumer);
      }
    });
  }
  return exports.map((entry) => ({ ...entry, productionConsumers: [...entry.productionConsumers].sort(), testOrAuxiliaryConsumers: [...entry.testOrAuxiliaryConsumers].sort() }))
    .filter((entry) => entry.productionConsumers.length === 0).sort((a, b) => a.file.localeCompare(b.file) || a.export.localeCompare(b.export));
}

export function duplicateGroups(records) {
  const groups = new Map();
  for (const record of records) {
    const matches = groups.get(record.hash) ?? [];
    matches.push(record); groups.set(record.hash, matches);
  }
  return [...groups.values()].filter((entries) => entries.length > 1).map((entries) => ({
    hash: entries[0].hash, tokens: entries[0].tokens, codeLines: entries[0].codeLines,
    instances: entries.map(({ file, line, name }) => ({ file, line, name }))
  })).sort((a, b) => b.codeLines - a.codeLines);
}

export function functionFingerprint(node, source) {
  const signature = [...node.parameters.flatMap((parameter) => tokens(parameter, source)), ...tokens(node.body, source)];
  return { hash: hash(JSON.stringify(signature.map((token) => [token.kind, token.text]))), tokens: signature.length };
}

export async function analyzeRepository(root = repository, { ref, buildSnapshot, fast = false } = {}) {
  const tracked = execFileSync("git", ref ? ["ls-tree", "-r", "--name-only", "-z", ref] : ["ls-files", "-z"], { cwd: root, encoding: "utf8" }).split("\0").filter(Boolean);
  const productionFiles = ref ? [] : (await Promise.all(PRODUCTION_ROOTS.map((entry) => allFiles(path.join(root, entry))))).flat().map((file) => portable(path.relative(root, file)));
  const paths = [...new Set([...tracked.filter((file) => sourceExtension.test(file)), ...productionFiles.filter((file) => sourceExtension.test(file))])].sort();
  const inventory = await Promise.all(paths.map(async (file) => {
    const content = ref ? execFileSync("git", ["show", `${ref}:${file}`], { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 })
      : await readFile(path.join(root, file), "utf8").catch((error) => { if (error.code === "ENOENT") return null; throw error; });
    if (content === null) return null;
    return { path: file, absolute: path.join(root, file), category: sourceCategory(file), loc: countCodeLines(content, file), rawLines: content.split("\n").length - Number(content.endsWith("\n")), bytes: Buffer.byteLength(content), sha256: hash(content), content };
  })).then((files) => files.filter(Boolean));
  const primary = inventory.filter((file) => file.category === "production");
  const summaries = {};
  for (const file of inventory) {
    const total = summaries[file.category] ??= { files: 0, loc: 0, rawLines: 0, bytes: 0 };
    total.files += 1; total.loc += file.loc; total.rawLines += file.rawLines; total.bytes += file.bytes;
  }
  const options = { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler, jsx: ts.JsxEmit.ReactJSX,
    allowJs: true, skipLibCheck: true, baseUrl: root, paths: { "@shared/*": ["src/shared/*"], "@/*": ["src/renderer/src/*"] } };
  const host = ts.createCompilerHost(options, true);
  if (ref) {
    const sourceTexts = new Map(inventory.map((file) => [file.absolute, file.content]));
    const originalRead = host.readFile;
    const originalExists = host.fileExists;
    const originalSource = host.getSourceFile;
    const sourcePath = (file) => PRODUCTION_ROOTS.some((entry) => file.startsWith(path.join(root, entry) + path.sep));
    host.readFile = (file) => sourceTexts.get(file) ?? (sourcePath(file) ? undefined : originalRead(file));
    host.fileExists = (file) => sourceTexts.has(file) || (!sourcePath(file) && originalExists(file));
    host.getSourceFile = (file, languageVersion, onError, shouldCreateNewSourceFile) => sourceTexts.has(file)
      ? sourceFile(file, sourceTexts.get(file)) : sourcePath(file) ? undefined : originalSource(file, languageVersion, onError, shouldCreateNewSourceFile);
  }
  const program = ts.createProgram(inventory.filter((file) => scriptExtension.test(file.path) && !file.path.startsWith("mockup-design/") && !file.path.startsWith("new design mock-up/")).map((file) => file.absolute), options, host);
  const primaryPaths = new Set(primary.map((file) => file.path));
  const allSourcePaths = new Set(inventory.map((file) => file.path));
  const functions = [];
  const dependencies = [];
  const duplicateFunctions = [];
  const duplicateBlocks = [];
  const packageImports = new Set();
  const externalImports = [];
  const computedImports = [];
  const ipcReferences = [];
  for (const file of primary) {
    const source = scriptExtension.test(file.path) ? program.getSourceFile(file.absolute) ?? sourceFile(file.path, file.content) : null;
    const specifiers = source ? importSpecifiers(source) : [...file.content.matchAll(/@(?:use|forward|import)\s+["']([^"']+)["']/g)].map((match) => ({ specifier: match[1], typeOnly: false }));
    for (const entry of specifiers) {
      if (entry.computed) { computedImports.push({ file: file.path, ...entry }); continue; }
      const target = resolveLocal(entry.specifier, file.path, options, allSourcePaths, root, host);
      if (target) dependencies.push({ from: file.path, to: target, typeOnly: entry.typeOnly });
      else {
        externalImports.push({ file: file.path, specifier: entry.specifier, typeOnly: entry.typeOnly });
        if (!entry.specifier.startsWith(".") && !entry.specifier.startsWith("@/") && !entry.specifier.startsWith("@shared/") && !entry.specifier.startsWith("node:") && !entry.specifier.startsWith("sass:")) packageImports.add(entry.specifier.startsWith("@") ? entry.specifier.split("/").slice(0, 2).join("/") : entry.specifier.split("/")[0]);
      }
    }
    if (!source) continue;
    ipcReferences.push(...ipcAuthorityReferences(source).map((reference) => ({ file: file.path, ...reference })));
    const identities = new Map();
    walk(source, (node) => {
      if (isFunction(node) && node.body) {
        const metrics = { file: file.path, ...functionMetrics(node, source) };
        const occurrence = (identities.get(metrics.identity) ?? 0) + 1;
        identities.set(metrics.identity, occurrence);
        metrics.identity += `#${occurrence}`;
        functions.push(metrics);
        const fingerprint = functionFingerprint(node, source);
        if (metrics.codeLines >= 20 && fingerprint.tokens >= 80) duplicateFunctions.push({ ...metrics, ...fingerprint });
      }
      if (ts.isBlock(node) && !isFunction(node.parent)) {
        const blockTokens = tokens(node, source);
        const codeLines = tokenCodeLines(node, source);
        if (codeLines >= 20 && blockTokens.length >= 120) duplicateBlocks.push({ file: file.path, line: source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1,
          name: ts.SyntaxKind[node.parent.kind], codeLines, tokens: blockTokens.length, hash: hash(JSON.stringify(blockTokens.map((token) => [token.kind, token.text]))) });
      }
    });
  }
  const forbiddenEdges = dependencies.filter((edge) => primaryPaths.has(edge.to) && (
    layer(edge.from) === "shared" && layer(edge.to) !== "shared" || layer(edge.from) === "renderer" && ["main", "preload"].includes(layer(edge.to)) ||
    layer(edge.from) === "main" && ["renderer", "preload"].includes(layer(edge.to)) || layer(edge.from) === "preload" && ["main", "renderer"].includes(layer(edge.to))
  ));
  const packageJson = JSON.parse(ref ? execFileSync("git", ["show", `${ref}:package.json`], { cwd: root, encoding: "utf8" }) : await readFile(path.join(root, "package.json"), "utf8"));
  const ownershipDocument = ref ? execFileSync("git", ["show", `${ref}:docs/architecture.md`], { cwd: root, encoding: "utf8" })
    : await readFile(path.join(root, "docs/architecture.md"), "utf8").catch((error) => { if (error.code === "ENOENT") return ""; throw error; });
  return {
    version: 1, commit: execFileSync("git", ["rev-parse", ref ?? "HEAD"], { cwd: root, encoding: "utf8" }).trim(),
    methodology: {
      primaryRoots: PRODUCTION_ROOTS, extensions: ["ts", "tsx", "js", "mjs", "cjs", "css", "scss"],
      loc: "Physical lines containing at least one non-whitespace parsed TypeScript/JavaScript token; comment trivia and blank token lines excluded. CSS/SCSS uses quote-aware comment removal; CSS block comments and SCSS block/line comments excluded. Tokens inside multiline template strings remain code, including embedded text that looks like comments.",
      inventory: "Production roots enumerated from working tree; auxiliary source from git ls-files. Identical production categories apply to baseline and final, including new production modules. Handwritten d.ts and adapted opencode-file/opencode-markdown modules conservatively included.",
      exclusions: [{ rule: String(testFile), reason: "Tests, specs, fixtures, and explicit large-session-fixture.ts benchmark fixture." }, { files: Object.keys(pinnedStyles.files), reason: pinnedStyles.provenance, upstreamCommit: pinnedStyles.upstreamCommit }, { files: Object.keys(pinnedAiElements.files), reason: pinnedAiElements.provenance, upstreamSnapshot: pinnedAiElements.upstreamSnapshot, sourceManifest: pinnedAiElements.sourceManifest }],
      thresholds: { oversizedModuleCodeLines: 800, oversizedFunctionCodeLines: 100, excessiveFunctionNesting: 5, excessiveFunctionCyclomatic: 20, duplicateFunctionMinimumLines: 20, duplicateFunctionMinimumTokens: 80, duplicateBlockMinimumLines: 20, duplicateBlockMinimumTokens: 120 },
      functions: "Every function declaration/expression, arrow, method, constructor, getter and setter with body. Length includes nested function source; nesting/complexity exclude nested function bodies. Cyclomatic is 1 plus if/loop/catch/conditional/case and logical &&, ||, ?? nodes.",
      dependencies: "TypeScript resolution with repository aliases; imports, exports, import types, import-equals, literal dynamic imports and require calls (including constant templates); computed loaders reported separately for fail-closed enforcement. CSS/SCSS @use/@forward/@import plus TS stylesheet imports. typeOnly means explicitly type-only syntax, not inferred runtime erasure. Cycle counts are strongly connected components, not every simple cycle.",
      duplication: "Exact token sequence with whitespace/comments ignored; function signatures plus bodies and non-function blocks. No identifier renaming or semantic equivalence claims; nested overlapping matches cannot be summed as removable LOC.",
      exports: "TypeScript checker aliases resolved and cross-file identifier references counted across production, tests and scripts. No production consumer is an investigation candidate, not proof of dead code. Entry points, string-based access, worker entry files, public contracts, dormant adapters and transitive reexports require manual review.",
      dependenciesNotImported: "Package dependency with no direct primary-code import is only a candidate; scripts, stylesheet imports, workers, transitive integrations and packaging may require it.",
      build: "Sum of stat.size for regular files recursively under out; logical uncompressed bytes, separate from disk allocation/package/install size. Build provenance supplied by coordinator validation."
    },
    summaries,
    production: { ...summaries.production, targetLoc: Math.floor(summaries.production.loc * 0.7), inventory: primary.map(({ content, absolute, category, ...file }) => file) },
    excludedAndAuxiliaryInventory: inventory.filter((file) => file.category !== "production").map(({ content, absolute, ...file }) => file),
    oversizedModules: primary.filter((file) => file.loc > 800).map(({ path, loc, rawLines }) => ({ file: path, loc, rawLines })).sort((a, b) => b.loc - a.loc),
    functions: functions.sort((a, b) => b.codeLines - a.codeLines),
    oversizedFunctions: functions.filter((fn) => fn.codeLines > 100),
    complexityHotspots: functions.filter((fn) => fn.cyclomatic > 20 || fn.nesting > 5).sort((a, b) => b.cyclomatic - a.cyclomatic),
    dependencies: dependencies.sort((a, b) => a.from.localeCompare(b.from) || a.to.localeCompare(b.to)),
    externalImports,
    computedImports,
    ownershipRoutes: ownershipRoutes(ownershipDocument),
    ipcReferences,
    pinnedStyleDrift: inventory.filter((file) => file.category === "pinned-upstream-styles" && file.sha256 !== pinnedStyles.files[file.path])
      .map((file) => ({ file: file.path, expected: pinnedStyles.files[file.path], actual: file.sha256 })),
    pinnedAiElementDrift: ref ? [] : pinnedAiElementDrift(inventory),
    cycles: { valueImports: dependencyCycles([...primaryPaths], dependencies, false), includingTypeOnly: dependencyCycles([...primaryPaths], dependencies, true) },
    forbiddenEdges,
    duplication: { functions: duplicateGroups(duplicateFunctions), blocks: duplicateGroups(duplicateBlocks) },
    unusedExportCandidates: fast ? [] : exportCandidates(program, primary.filter((file) => scriptExtension.test(file.path)), root),
    dependencyCandidatesWithoutDirectProductionImport: Object.keys(packageJson.dependencies ?? {}).filter((name) => !packageImports.has(name)).sort(),
    build: fast ? null : buildSnapshot ? JSON.parse(await readFile(buildSnapshot, "utf8")) : await buildSize(root)
  };
}

async function main() {
  const outputIndex = process.argv.indexOf("--output");
  const output = outputIndex >= 0 ? process.argv[outputIndex + 1] : null;
  const refIndex = process.argv.indexOf("--ref");
  const buildIndex = process.argv.indexOf("--build-snapshot");
  const report = await analyzeRepository(repository, { ref: refIndex >= 0 ? process.argv[refIndex + 1] : undefined,
    buildSnapshot: buildIndex >= 0 ? process.argv[buildIndex + 1] : undefined });
  if (output) {
    const destination = path.resolve(repository, output);
    await mkdir(path.dirname(destination), { recursive: true });
    await writeFile(destination, JSON.stringify(report, null, 2) + "\n");
  }
  console.log(JSON.stringify({ commit: report.commit, production: { files: report.production.files, loc: report.production.loc, targetLoc: report.production.targetLoc }, oversizedModules: report.oversizedModules,
    oversizedFunctions: report.oversizedFunctions.length, complexityHotspots: report.complexityHotspots.length, cycles: report.cycles, forbiddenEdges: report.forbiddenEdges,
    duplicateFunctions: report.duplication.functions.length, duplicateBlocks: report.duplication.blocks.length, unusedExportCandidates: report.unusedExportCandidates.length,
    dependencyCandidates: report.dependencyCandidatesWithoutDirectProductionImport, build: { files: report.build.files, logicalBytes: report.build.logicalBytes } }, null, 2));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
