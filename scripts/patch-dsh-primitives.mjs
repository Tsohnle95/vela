// DeepSeek Harness' published primitives bundle targets React 18, where
// `inert=""` marks the decorative shimmer overlay inert. React 19 treats an
// empty string as a boolean-attribute warning, so rewrite that one attribute to
// the boolean form after install. The implementation is otherwise untouched.
import { readFile, writeFile } from "node:fs/promises";

const target = "node_modules/@deepseek-ai/dsh-client-ui-primitives/lib/index.js";
const source = await readFile(target, "utf8");
if (source.includes('inert: ""')) {
  await writeFile(target, source.replaceAll('inert: ""', "inert: true"));
} else if (!source.includes("inert: true")) {
  throw new Error(`DSH primitives compatibility target changed: ${target}`);
}
