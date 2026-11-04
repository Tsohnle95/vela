import { compile, NodePackageImporter } from "sass";
import type { SassCompileOptions } from "./sass-config";
export interface SassCompileResult { css: string; sourceMap?: string; warnings: string[] }
export function compileSass(source: string, options: SassCompileOptions): SassCompileResult {
  const warnings: string[] = [];
  const result = compile(source, {
    ...options,
    importers: [new NodePackageImporter(options.loadPaths[0])],
    logger: { warn: (message) => warnings.push(message), debug: (message) => warnings.push(message) }
  });
  return { css: result.css, ...(result.sourceMap ? { sourceMap: JSON.stringify(result.sourceMap) } : {}), warnings };
}
