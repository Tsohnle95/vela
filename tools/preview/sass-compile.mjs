// Only the pinned compiler runs here. Rust owns watching, cancellation,
// context ownership, preferences and installing complete output files.
import { compile, compileString, NodePackageImporter } from "sass";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { sassPlan } from "./sass-plan.mjs";

export async function compileWorkspace(root) {
  const plan = await sassPlan(root);
  const outputs = [];
  for (const output of plan.outputs) {
    const warnings = [];
    try {
      const result = compile(output.source, {
        ...output.options,
        importers: [new NodePackageImporter(output.options.loadPaths[0])],
        logger: {
          warn: (message) => warnings.push(message),
          debug: (message) => warnings.push(message),
        },
      });
      let sourceMap;
      let css = result.css;
      if (result.sourceMap) {
        const map = result.sourceMap;
        map.sources = map.sources.map((source) => source.startsWith("file:")
          ? encodeURI(path.relative(path.dirname(output.output), fileURLToPath(source)).split(path.sep).join("/"))
          : source);
        sourceMap = JSON.stringify(map);
        css += `\n/*# sourceMappingURL=${encodeURIComponent(path.basename(output.output))}.map */\n`;
      } else {
        css += "\n";
      }
      outputs.push({ source: output.source, output: output.output, css, sourceMap, warnings });
    } catch (error) {
      outputs.push({ source: output.source, output: output.output, warnings, error: String(error) });
    }
  }
  return { convention: plan.convention, outputs };
}

function scssDiagnostic(severity, message, span) {
  const start = span?.start;
  const end = span?.end;
  return {
    line: start?.line ?? 1,
    column: (start?.column ?? 0) + 1,
    endLine: end?.line ?? start?.line ?? 1,
    endColumn: (end?.column ?? start?.column ?? 0) + 1,
    message: String(message),
    severity
  };
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(Buffer.from(chunk));
  return Buffer.concat(chunks).toString("utf8");
}

export async function validateBuffer(root, sourcePath, content) {
  const warnings = [];
  const syntax = path.extname(sourcePath).toLowerCase() === ".sass" ? "indented" : "scss";
  try {
    compileString(content, {
      url: pathToFileURL(sourcePath),
      syntax,
      loadPaths: [root, path.join(root, "node_modules")],
      importers: [new NodePackageImporter(root)],
      logger: {
        warn: (message, options) => warnings.push(scssDiagnostic("warning", message, options?.span)),
        debug: () => {}
      }
    });
    return { diagnostics: warnings };
  } catch (error) {
    return {
      diagnostics: [
        ...warnings,
        scssDiagnostic(
          "error",
          error?.sassMessage ?? error?.message ?? String(error),
          error?.span
        )
      ]
    };
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  try {
    if (process.argv[2] === "--validate") {
      const root = process.argv[3];
      const source = process.argv[4];
      if (!root || !path.isAbsolute(root) || !source || !path.isAbsolute(source)) {
        throw new Error("A canonical workspace root and source path are required");
      }
      process.stdout.write(JSON.stringify(await validateBuffer(root, source, await readStdin())));
    } else {
      const root = process.argv[2];
      if (!root || !path.isAbsolute(root)) throw new Error("A canonical workspace root is required");
      process.stdout.write(JSON.stringify(await compileWorkspace(root)));
    }
  } catch (error) {
    process.stdout.write(JSON.stringify({ error: String(error) }));
    process.exitCode = 1;
  }
}
