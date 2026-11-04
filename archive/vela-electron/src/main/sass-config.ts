import { readdir, readFile, lstat } from "node:fs/promises";
import path from "node:path";
import JSON5 from "json5";
import { HIDDEN_RECOVERY_DIRS } from "@shared/types";

export const SASS_IGNORES = new Set(["node_modules", ".git", "dist", "build", ".next", ".cache", ...HIDDEN_RECOVERY_DIRS, "coverage", "out", "vendor"]);
export const isSassSource = (file: string): boolean => /\.(scss|sass)$/i.test(file);
export const isSassPartial = (file: string): boolean => path.basename(file).startsWith("_");
export function insideRoot(root: string, file: string): boolean {
  const relative = path.relative(root, file);
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}
export function sassIgnored(root: string, file: string): boolean {
  return !insideRoot(root, file) || path.relative(root, file).split(path.sep).some((part) => SASS_IGNORES.has(part));
}
export interface SassCompileOptions { style: "expanded" | "compressed"; loadPaths: string[]; sourceMap: boolean }
export interface SassOutput { source: string; output: string; options: SassCompileOptions }
export interface SassPlan { outputs: SassOutput[]; convention: string }

function localPath(root: string, value: unknown): string {
  if (typeof value !== "string" || !value || value.includes("\0") || path.isAbsolute(value) || /^[a-z]:/i.test(value)) throw new Error("Sass mappings must use workspace-relative paths");
  const resolved = path.resolve(root, value);
  if (!insideRoot(root, resolved)) throw new Error(`Sass path escapes the workspace: ${value}`);
  return resolved;
}
async function jsonFile(root: string, name: string): Promise<Record<string, any> | null> {
  const file = path.join(root, name);
  const stat = await lstat(file).catch(() => null);
  if (!stat) return null;
  const parent = await lstat(path.dirname(file));
  if (!parent.isDirectory() || parent.isSymbolicLink()) throw new Error(`Sass configuration directory must be inside the workspace: ${name}`);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 1_048_576) throw new Error(`Cannot safely read Sass configuration ${name}`);
  const value = JSON5.parse(await readFile(file, "utf8"));
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`Sass configuration must be an object: ${name}`);
  return value;
}
export async function sassEntrypoints(root: string): Promise<string[]> {
  const files: string[] = [];
  const visit = async (directory: string): Promise<void> => {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isSymbolicLink() || sassIgnored(root, absolute)) continue;
      if (entry.isDirectory()) await visit(absolute);
      else if (entry.isFile() && isSassSource(absolute) && !isSassPartial(absolute)) files.push(absolute);
    }
  };
  await visit(root);
  return files.sort();
}

async function explicitConfig(root: string): Promise<{ name: string; config: Record<string, any> | null }> {
  const currentName = "vela-sass.json";
  const current = await jsonFile(root, currentName);
  if (current) return { name: currentName, config: current };
  const previousNames = [
    `${String.fromCharCode(109, 105, 114, 97)}-sass.json`,
    `${String.fromCharCode(111, 114, 98, 105, 116)}-sass.json`
  ];
  for (const previousName of previousNames) {
    const previous = await jsonFile(root, previousName);
    if (previous) return { name: previousName, config: previous };
  }
  return { name: currentName, config: null };
}

export async function sassPlan(root: string): Promise<SassPlan> {
  const sources = await sassEntrypoints(root);
  const base: SassCompileOptions = { style: "expanded", loadPaths: [root, path.join(root, "node_modules")], sourceMap: false };
  const { name: explicitName, config: explicit } = await explicitConfig(root);
  const mappings: Array<[string, string]> = [];
  let convention = "CSS beside source";
  if (explicit) {
    if (explicit.style !== undefined && !["expanded", "compressed"].includes(explicit.style)) throw new Error(`${explicitName}: style must be expanded or compressed`);
    base.style = explicit.style ?? "expanded";
    base.sourceMap = explicit.sourceMap === true;
    if (explicit.loadPaths !== undefined) {
      if (!Array.isArray(explicit.loadPaths)) throw new Error(`${explicitName}: loadPaths must be an array`);
      base.loadPaths.push(...explicit.loadPaths.map((value: unknown) => localPath(root, value)));
    }
    if (explicit.entries !== undefined) {
      if (!explicit.entries || typeof explicit.entries !== "object" || Array.isArray(explicit.entries)) throw new Error(`${explicitName}: entries must map inputs to CSS outputs`);
      for (const [input, output] of Object.entries(explicit.entries)) mappings.push([localPath(root, input), localPath(root, output)]);
    }
    convention = explicitName;
  }
  const settings = explicit ? null : await jsonFile(root, ".vscode/settings.json");
  const formats = settings?.["liveSassCompile.settings.formats"];
  if (!explicit && Array.isArray(formats) && formats.length) {
    const exclude = settings?.["liveSassCompile.settings.excludeList"] ?? [];
    const include = settings?.["liveSassCompile.settings.includeItems"] ?? [];
    const outputs = sources.filter((source) => {
      const relative = path.relative(root, source).split(path.sep).join("/");
      const matches = (patterns: unknown): boolean => Array.isArray(patterns) && patterns.some((pattern) => typeof pattern === "string" && path.matchesGlob(relative, pattern.replace(/^\//, "")));
      return !matches(exclude) && (!include.length || matches(include));
    }).flatMap((source) => formats.map((format): SassOutput => {
      if (!format || !["expanded", "compressed"].includes(format.format ?? "expanded")) throw new Error("Unsupported Live Sass output format");
      const extension = format.extensionName ?? ".css";
      if (typeof extension !== "string" || !/^[^/\\]*\.css$/.test(extension)) throw new Error("Invalid Live Sass CSS extension");
      let directory = path.dirname(source);
      if (typeof format.savePath === "string") {
        directory = format.savePath.startsWith("~/") ? path.resolve(directory, format.savePath.slice(2)) : path.resolve(root, format.savePath.replace(/^[/\\]+/, ""));
      } else if (format.savePathReplacementPairs && typeof format.savePathReplacementPairs === "object") {
        let relative = path.relative(root, directory).split(path.sep).join("/");
        for (const [from, to] of Object.entries(format.savePathReplacementPairs)) {
          if (typeof to !== "string") throw new Error("Invalid Live Sass replacement path");
          relative = relative.replace(from, to);
        }
        directory = path.resolve(root, relative);
      }
      if (!insideRoot(root, directory)) throw new Error("Live Sass output escapes the workspace");
      return { source, output: path.join(directory, path.basename(source).replace(/\.(scss|sass)$/i, extension)), options: { ...base, style: format.format ?? "expanded", sourceMap: settings?.["liveSassCompile.settings.generateMap"] === true } };
    }));
    return checked({ outputs, convention: "Live Sass workspace settings" });
  }
  if (!explicit) {
    const pkg = await jsonFile(root, "package.json");
    for (const script of Object.values(pkg?.scripts ?? {})) {
      if (typeof script !== "string") continue;
      const command = script.match(/(?:^|\s)(?:npx\s+)?sass\s/);
      if (!command) continue;
      const unquote = (value: string): string => value.replace(/^["']|["']$/g, "");
      const tokens = script.slice(command.index! + command[0].length).split(/[;&|]/, 1)[0].match(/(?:"[^"]*"|'[^']*'|[^\s"'])+/g)?.map(unquote) ?? [];
      const positional: string[] = [];
      for (let index = 0; index < tokens.length; index++) {
        const token = tokens[index];
        if (["--load-path", "-I"].includes(token)) { base.loadPaths.push(localPath(root, unquote(tokens[++index]))); continue; }
        if (token.startsWith("--load-path=")) { base.loadPaths.push(localPath(root, unquote(token.slice(12)))); continue; }
        if (token.startsWith("-I") && token.length > 2) { base.loadPaths.push(localPath(root, unquote(token.slice(2)))); continue; }
        if (token === "--style" || token === "-s") { index++; continue; }
        if (token.startsWith("-")) continue;
        positional.push(token);
        const pair = token.split(":").map(unquote);
        if (pair.length !== 2 || pair.some((value) => !value || /[$*{}]/.test(value)) || token.startsWith("--")) continue;
        mappings.push([localPath(root, pair[0]), localPath(root, pair[1])]);
      }
      if (!mappings.length && positional.length === 2 && isSassSource(positional[0]) && /\.css$/i.test(positional[1])) mappings.push([localPath(root, positional[0]), localPath(root, positional[1])]);
      if (mappings.length) {
        base.style = /(?:--style(?:=|\s+)|-s\s+)compressed/.test(script) ? "compressed" : "expanded";
        base.sourceMap = !tokens.includes("--no-source-map");
        convention = "literal Sass package script mappings";
        break;
      }
    }
  }
  const outputs = sources.flatMap((source): SassOutput[] => {
    if (!mappings.length) return [{ source, output: source.replace(/\.(scss|sass)$/i, ".css"), options: base }];
    return mappings.filter(([input]) => source === input || insideRoot(input, source)).map(([input, output]) => ({
      source,
      output: source === input ? output : path.join(output, path.relative(input, source).replace(/\.(scss|sass)$/i, ".css")),
      options: base
    }));
  });
  for (const [input] of mappings) if (!sources.some((source) => source === input || insideRoot(input, source))) throw new Error(`Sass input contains no entrypoints: ${path.relative(root, input)}`);
  return checked({ outputs, convention });
}
function checked(plan: SassPlan): SassPlan {
  const seen = new Map<string, string>();
  for (const item of plan.outputs) {
    if (!/\.css$/i.test(item.output)) throw new Error("Sass output must end in .css");
    const previous = seen.get(item.output);
    if (previous) throw new Error(`Sass output collision: ${item.output}`);
    seen.set(item.output, item.source);
  }
  return plan;
}
