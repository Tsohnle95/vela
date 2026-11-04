import path from "node:path";
import { realpathSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import postcss from "postcss";
import { compile } from "sass";
import { SourceMapConsumer, SourceMapGenerator } from "source-map-js";
import type { Plugin } from "vite";
import { APP_STYLES_START, APP_STYLES_END } from "../src/shared/app-styles";

interface SourceLocation { source: string; line: number; column: number }

/**
 * Canonicalize a module id for comparison. Vite passes ids with POSIX
 * separators (and can lead a Windows drive with a slash), while `realpathSync`
 * returns host-native separators and casing; normalize both before matching so
 * the transform still finds its entry on Windows.
 */
export function canonicalModuleId(value: string, caseInsensitive = process.platform === "win32"): string {
  const posix = value.replace(/^\/+([A-Za-z]:)/, "$1").replace(/\\/g, "/");
  return caseInsensitive ? posix.toLowerCase() : posix;
}

function nodeKey(node: postcss.ChildNode): string {
  if (node.type === "rule") return `rule:${node.selector}`;
  if (node.type === "decl") return `decl:${node.prop}`;
  if (node.type === "atrule") return `at:${node.name}:${node.params}`;
  return node.type;
}

// Vite 6 discards CSS maps during extraction. Carry locations through that
// extraction as comments, then replace them with a map of the final asset.
export function editableCssSourceMaps(entry: string): Plugin {
  const entryPath = realpathSync(entry);
  const entryKey = canonicalModuleId(entryPath);
  const locations: SourceLocation[] = [];
  let outputDirectory = "";
  const marker = /\/\* vela-css-source:(\d+) \*\//g;
  return {
    name: "vela-editable-css-source-maps",
    apply: "build",
    configResolved(config) { outputDirectory = path.resolve(config.root, config.build.outDir); },
    buildStart() { locations.length = 0; },
    transform(code, id) {
      if (canonicalModuleId(id) !== entryKey) return;
      const compiled = compile(entryPath, { sourceMap: true, style: "expanded" });
      if (!compiled.sourceMap) this.error(`Missing Sass source map for ${entryPath}`);
      const consumer = new SourceMapConsumer({ ...compiled.sourceMap, version: String(compiled.sourceMap.version) });
      const originals: postcss.ChildNode[] = [];
      postcss.parse(compiled.css).walk(node => { if (node.type !== "comment") originals.push(node); });
      let nextOriginal = 0;
      const insertions: { offset: number; text: string }[] = [];
      postcss.parse(code, { from: id }).walk((node) => {
        if (node.type === "comment" || !node.source?.start) return;
        const start = node.source.start;
        const sourceNode = originals[nextOriginal++];
        if (!sourceNode || nodeKey(sourceNode) !== nodeKey(node)) this.error("Compiled CSS structure changed before source mapping");
        const sourceStart = sourceNode.source!.start!;
        const original = consumer.originalPositionFor({ line: sourceStart.line, column: sourceStart.column - 1 });
        if (!original.source || original.line === null || original.column === null) return;
        const source = original.source.startsWith("file:") ? original.source
          : pathToFileURL(path.resolve(path.dirname(id), original.source)).href;
        const index = locations.push({ source, line: original.line, column: original.column }) - 1;
        insertions.push({ offset: start.offset!, text: `/* vela-css-source:${index} */` });
      });
      if (nextOriginal !== originals.length) this.error("Compiled CSS lost source nodes before source mapping");
      for (const { offset, text } of insertions.sort((a, b) => b.offset - a.offset)) {
        code = code.slice(0, offset) + text + code.slice(offset);
      }
      return { code: `${APP_STYLES_START}\n${code}\n${APP_STYLES_END}`, map: { mappings: "" } };
    },
    generateBundle(_options, bundle) {
      let emitted = false;
      for (const asset of Object.values(bundle)) {
        if (asset.type !== "asset" || !asset.fileName.endsWith(".css") || typeof asset.source !== "string") continue;
        const css = asset.source;
        if (!css.includes("/* vela-css-source:")) continue;
        const map = new SourceMapGenerator({ file: path.basename(asset.fileName) });
        let output = "";
        let offset = 0;
        let line = 1;
        let column = 0;
        for (const match of css.matchAll(marker)) {
          const fragment = css.slice(offset, match.index);
          output += fragment;
          const lines = fragment.split("\n");
          line += lines.length - 1;
          column = lines.length > 1 ? lines.at(-1)!.length : column + fragment.length;
          const location = locations[Number(match[1])];
          if (!location) this.error("Missing editable CSS source location");
          const source = path.relative(path.dirname(path.join(outputDirectory, asset.fileName)), fileURLToPath(location.source)).split(path.sep).join("/");
          map.addMapping({ generated: { line, column }, original: { line: location.line, column: location.column }, source });
          offset = match.index! + match[0].length;
        }
        output += css.slice(offset);
        const mapName = `${asset.fileName}.map`;
        asset.source = `${output}\n/*# sourceMappingURL=${path.basename(mapName)} */\n`;
        this.emitFile({ type: "asset", fileName: mapName, source: map.toString() });
        emitted = true;
      }
      if (!emitted) this.error("Editable stylesheet source map was not emitted");
    }
  };
}
