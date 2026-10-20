#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";
import { builtinModules } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeRepository, dependencyCycles, sourceCategory } from "./architecture-analysis.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const configurationPath = path.join(root, "scripts/architecture-baseline.json");
const builtins = new Set(builtinModules.flatMap((name) => [name, `node:${name}`]));
const metrics = ["codeLines", "cyclomatic", "nesting"];
const functionKey = (fn) => `${fn.file}:${fn.identity}`;
const edgeKey = (edge) => `${edge.from}>${edge.to}:${edge.typeOnly ? "type" : "value"}`;
const cloneKey = (group) => group.hash;

function cycleKey(component, edges) {
  const members = new Set(component);
  return [...new Set(edges.filter((edge) => members.has(edge.from) && members.has(edge.to)).map(edgeKey))].sort().join("|");
}

function matchesFile(file, rule) {
  return rule.file ? file === rule.file : file.startsWith(rule.prefix);
}

function matchesPath(file, prefix) {
  return /\.(?:[cm]?[jt]sx?|css|scss)$/.test(prefix) ? file === prefix : file.startsWith(prefix);
}

function matchesExternal(specifier, allowed) {
  return /[:/]$/.test(allowed) ? specifier.startsWith(allowed) : specifier === allowed;
}

function validPath(value, directoryAllowed = false) {
  return typeof value === "string" && value.startsWith("src/") && !value.includes("\\") &&
    !/[\s\u0000-\u001f\u007f]/.test(value) && !value.includes("//") &&
    !value.split("/").some((part) => part === "." || part === "..") &&
    (/\.(?:[cm]?[jt]sx?|css|scss)$/.test(value) || directoryAllowed && value.endsWith("/"));
}

export function configurationViolations(configuration) {
  const errors = [];
  const fail = (subject, detail) => errors.push({ rule: "configuration-schema", subject, detail });
  const object = (value) => value !== null && typeof value === "object" && !Array.isArray(value);
  const text = (value) => typeof value === "string" && Boolean(value.trim());
  const integer = (value, minimum = 1) => Number.isSafeInteger(value) && value >= minimum;
  if (!object(configuration)) { fail("manifest", "Expected an object"); return errors; }
  for (const section of ["limits", "modules", "functions", "functionExceptions", "duplicates"]) {
    if (!object(configuration[section])) fail(section, "Required object is missing or invalid");
  }
  for (const section of ["knownFiles", "edgeExceptions", "cycleExceptions", "privateBoundaries", "pureOwners"]) {
    if (!Array.isArray(configuration[section])) fail(section, "Required array is missing or invalid");
  }
  if (errors.length) return errors;
  for (const metric of ["module", ...metrics]) if (!integer(configuration.limits[metric])) fail(`limits.${metric}`, "Expected a positive integer");
  const rationale = (entry, subject) => {
    if (!object(entry)) { fail(subject, "Expected an allowance object"); return false; }
    if (!text(entry.reason)) errors.push({ rule: "exception-rationale", subject, detail: "Allowance needs a specific rationale" });
    if (entry.origin !== undefined && !text(entry.origin)) fail(subject, "Origin must be a nonempty string");
    return true;
  };
  for (const [file, entry] of Object.entries(configuration.modules)) {
    if (!validPath(file)) fail(file, "Expected a production file path");
    if (rationale(entry, file) && !integer(entry.loc)) fail(file, "Expected a positive LOC budget");
  }
  for (const section of ["functions", "functionExceptions"]) {
    for (const [identity, entry] of Object.entries(configuration[section])) {
      const separator = identity.indexOf(":");
      if (separator < 0 || !validPath(identity.slice(0, separator)) || !text(identity.slice(separator + 1))) fail(identity, "Expected file:symbol identity");
      if (!rationale(entry, identity)) continue;
      for (const metric of metrics) if (entry[metric] !== undefined && !integer(entry[metric], metric === "nesting" ? 0 : 1)) fail(identity, `Invalid ${metric} budget`);
    }
  }
  if (new Set(configuration.knownFiles).size !== configuration.knownFiles.length || configuration.knownFiles.some((file) => !validPath(file))) fail("knownFiles", "Expected unique production file paths");
  for (const entry of configuration.edgeExceptions) {
    if (rationale(entry, "edgeExceptions") && (!validPath(entry.from) || !validPath(entry.to) || typeof entry.typeOnly !== "boolean")) fail("edgeExceptions", "Invalid edge path or mode");
  }
  for (const entry of configuration.cycleExceptions) {
    if (rationale(entry, "cycleExceptions") && (!["valueImports", "includingTypeOnly"].includes(entry.mode) || !text(entry.key))) fail("cycleExceptions", "Invalid cycle mode or key");
  }
  for (const kind of ["functions", "blocks"]) {
    if (!object(configuration.duplicates[kind])) { fail(`duplicates.${kind}`, "Expected a clone budget map"); continue; }
    for (const [key, copies] of Object.entries(configuration.duplicates[kind])) if (!text(key) || !integer(copies)) fail(key, "Expected positive clone count");
  }
  for (const entry of configuration.privateBoundaries) {
    if (!object(entry) || !text(entry.owner) || !validPath(entry.targetPrefix, true) || !Array.isArray(entry.allowedFrom) || !entry.allowedFrom.length || entry.allowedFrom.some((file) => !validPath(file, true))) fail("privateBoundaries", "Expected named owner and explicit production paths");
  }
  for (const entry of configuration.pureOwners) {
    if (!object(entry) || Boolean(entry.file) === Boolean(entry.prefix) || (entry.file ? !validPath(entry.file) : !validPath(entry.prefix, true) || !entry.prefix.endsWith("/")) || !Array.isArray(entry.allowedLocal) || entry.allowedLocal.some((file) => !validPath(file, true)) || !Array.isArray(entry.allowedExternal) || entry.allowedExternal.some((name) => !text(name))) fail("pureOwners", "Expected one exact file or directory selector and explicit dependency lists");
  }
  return errors;
}

export function architectureViolations(report, configuration) {
  const invalidConfiguration = configurationViolations(configuration);
  if (invalidConfiguration.length) return invalidConfiguration;
  const violations = [];
  const add = (rule, subject, detail) => violations.push({ rule, subject, detail });
  for (const drift of report.pinnedStyleDrift ?? []) add("pinned-style-integrity", drift.file, `Pinned upstream content changed: expected ${drift.expected}, actual ${drift.actual}. Review upstream provenance before updating the frozen inventory.`);
  for (const drift of report.pinnedAiElementDrift ?? []) add("pinned-ai-elements-integrity", drift.file, `Pinned AI Elements content changed: expected ${drift.expected}, actual ${drift.actual}. Review upstream provenance before updating the frozen inventory.`);
  const fileSet = new Set(report.production.inventory.map((file) => file.path));
  const routes = report.ownershipRoutes ?? [];
  const hasOwner = (file) => routes.some((route) => route.owner?.trim() && (route.path.endsWith("/") ? file.startsWith(route.path) : file === route.path));
  if (fileSet.size && !routes.length) add("ownership-route", "docs/architecture.md", "Canonical named ownership routes are missing");
  for (const boundary of configuration.privateBoundaries) {
    const targets = [...fileSet].filter((file) => matchesPath(file, boundary.targetPrefix));
    if (!targets.length) add("owner-target", boundary.targetPrefix, "Private owner target does not exist; repair or remove the stale rule");
    else if (routes.length && targets.some((file) => !hasOwner(file))) add("ownership-route", boundary.targetPrefix, "Private owner needs a canonical responsibility route");
  }
  for (const owner of configuration.pureOwners) {
    const targets = [...fileSet].filter((file) => matchesFile(file, owner));
    if (!targets.length) add("owner-target", owner.file ?? owner.prefix, "Dependency owner target does not exist; repair or remove the stale rule");
    else if (routes.length && targets.some((file) => !hasOwner(file))) add("ownership-route", owner.file ?? owner.prefix, "Dependency owner needs a canonical responsibility route");
  }
  for (const route of routes) {
    if (!route.owner?.trim() || !validPath(route.path, true) || ![...fileSet].some((file) => route.path.endsWith("/") ? file.startsWith(route.path) : file === route.path)) add("ownership-route", route.path, "Canonical ownership route must name an existing production owner");
  }
  for (const imported of report.computedImports ?? []) add("computed-import", `${imported.file}:${imported.line}`, "Use an explicit literal dependency so authority and cycle checks can inspect it");
  for (const reference of report.ipcReferences ?? []) {
    if (reference.file !== "src/main/index.ts") add("ipc-authority", `${reference.file}:${reference.line}`, reference.kind);
  }
  for (const file of report.production.inventory) {
    const limit = configuration.modules[file.path]?.loc ?? configuration.limits.module;
    if (file.loc > limit) add("module-size", file.path, `${file.loc} code lines exceeds ${limit}`);
    if (routes.length && !configuration.knownFiles.includes(file.path) && !hasOwner(file.path)) {
      add("ownership-route", file.path, "New module needs a named owner in the existing canonical architecture table");
    }
    if (!configuration.knownFiles.includes(file.path) && /(?:^|\/)(?:helpers?|utils?|misc|common)(?:\.[^/]+)?(?:\/|$)/.test(file.path)) {
      add("ownership-name", file.path, "Use the owning feature or responsibility in the filename");
    }
  }
  for (const fn of report.functions) {
    const identity = functionKey(fn);
    const budget = configuration.functions[identity] ?? configuration.functionExceptions[identity];
    for (const metric of metrics) {
      const limit = budget?.[metric] ?? configuration.limits[metric];
      if (fn[metric] > limit) add(`function-${metric}`, identity, `${fn[metric]} exceeds ${limit}`);
    }
  }
  for (const edge of report.dependencies) {
    const exception = configuration.edgeExceptions.find((entry) => entry.from === edge.from && entry.to === edge.to && entry.typeOnly === edge.typeOnly);
    if (exception) continue;
    if (!fileSet.has(edge.to) && !["pinned-upstream-styles", "pinned-upstream-ai-elements"].includes(sourceCategory(edge.to))) add("production-scope", edge.from, `Production cannot depend on excluded implementation ${edge.to}`);
    if (report.forbiddenEdges.some((forbidden) => edgeKey(forbidden) === edgeKey(edge))) add("layer-direction", edge.from, edgeKey(edge));
    for (const boundary of configuration.privateBoundaries) {
      if (!matchesPath(edge.to, boundary.targetPrefix)) continue;
      if (!boundary.allowedFrom.some((prefix) => matchesPath(edge.from, prefix))) add("private-owner", edge.from, `${edge.to} belongs to ${boundary.owner}`);
    }
    for (const owner of configuration.pureOwners) {
      if (matchesFile(edge.from, owner) && !owner.allowedLocal.some((prefix) => matchesPath(edge.to, prefix))) add("pure-owner", edge.from, `Cannot depend on ${edge.to}`);
    }
  }
  for (const imported of report.externalImports ?? []) {
    const privileged = builtins.has(imported.specifier) || imported.specifier.startsWith("node:") || ["electron", "electron-updater", "node-pty"].includes(imported.specifier.split("/")[0]);
    if (privileged && (imported.file.startsWith("src/renderer/") || imported.file.startsWith("src/shared/"))) add("privileged-import", imported.file, imported.specifier);
    if ((imported.specifier === "@opencode/client" || imported.specifier.startsWith("@opencode/client/")) && imported.file !== "src/main/opencode.ts") add("sdk-owner", imported.file, "OpenCode SDK belongs to src/main/opencode.ts");
    for (const owner of configuration.pureOwners) {
      if (matchesFile(imported.file, owner) && !owner.allowedExternal.some((prefix) => matchesExternal(imported.specifier, prefix))) add("pure-owner", imported.file, `Cannot depend on ${imported.specifier}`);
    }
  }
  for (const mode of ["valueImports", "includingTypeOnly"]) {
    const edges = mode === "valueImports" ? report.dependencies.filter((edge) => !edge.typeOnly) : report.dependencies;
    for (const component of dependencyCycles([...fileSet], edges, true)) {
      const key = cycleKey(component, edges);
      if (!configuration.cycleExceptions.some((entry) => entry.mode === mode && entry.key === key)) add("dependency-cycle", component.join(", "), mode);
    }
  }
  for (const kind of ["functions", "blocks"]) {
    for (const group of report.duplication[kind]) {
      const prior = configuration.duplicates[kind][cloneKey(group)] ?? 1;
      if (group.instances.length > prior) add("new-duplication", group.instances.map((entry) => entry.file).join(", "), `${kind}: ${group.instances.length} copies of ${group.codeLines} lines exceeds ${prior}`);
    }
  }
  return violations;
}

export function ratchetConfiguration(report, configuration) {
  const violations = architectureViolations(report, configuration);
  if (violations.length > 0) throw new Error("Cannot ratchet a failing architecture check");
  const next = structuredClone(configuration);
  const currentFiles = new Map(report.production.inventory.map((file) => [file.path, file]));
  const currentFunctions = new Map(report.functions.map((fn) => [functionKey(fn), fn]));
  for (const [file, budget] of Object.entries(next.modules)) {
    const current = currentFiles.get(file);
    if (!current || current.loc <= next.limits.module) delete next.modules[file];
    else budget.loc = Math.min(budget.loc, current.loc);
  }
  for (const section of ["functions", "functionExceptions"]) {
    for (const [identity, budget] of Object.entries(next[section])) {
      const current = currentFunctions.get(identity);
      if (!current || metrics.every((metric) => current[metric] <= next.limits[metric])) { delete next[section][identity]; continue; }
      for (const metric of metrics) budget[metric] = Math.min(budget[metric] ?? next.limits[metric], Math.max(next.limits[metric], current[metric]));
    }
  }
  next.knownFiles = [...currentFiles.keys()].sort();
  for (const kind of ["functions", "blocks"]) {
    const current = new Map(report.duplication[kind].map((group) => [cloneKey(group), group.instances.length]));
    for (const key of Object.keys(next.duplicates[kind])) {
      if (!current.has(key)) delete next.duplicates[kind][key];
      else next.duplicates[kind][key] = Math.min(next.duplicates[kind][key], current.get(key));
    }
  }
  next.cycleExceptions = next.cycleExceptions.filter((exception) => {
    const edges = exception.mode === "valueImports" ? report.dependencies.filter((edge) => !edge.typeOnly) : report.dependencies;
    return dependencyCycles([...currentFiles.keys()], edges, true).some((component) => cycleKey(component, edges) === exception.key);
  });
  return next;
}

async function main() {
  const configuration = JSON.parse(await readFile(configurationPath, "utf8"));
  const report = await analyzeRepository(root, { fast: true });
  const violations = architectureViolations(report, configuration);
  if (violations.length > 0) {
    console.error(`architecture check failed (${violations.length} violations)`);
    for (const violation of violations) console.error(`  ${violation.rule}: ${violation.subject}: ${violation.detail}`);
    process.exitCode = 1;
    return;
  }
  if (process.argv.includes("--ratchet")) {
    await writeFile(configurationPath, JSON.stringify(ratchetConfiguration(report, configuration), null, 2) + "\n");
    console.log("Architecture budgets ratcheted downward; review and commit the configuration diff.");
  } else console.log(`Architecture check passed: ${report.production.files} production files; budgets, boundaries, cycles, exact duplication.`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
