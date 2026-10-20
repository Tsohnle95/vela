// @vitest-environment node
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { describe, expect, it } from "vitest";
import pins from "./pinned-upstream-ai-elements.json";

const root = path.resolve(__dirname, "..");
const upstream = "src/renderer/src/ai-elements/upstream";
const read = (file: string) => readFile(path.join(root, file), "utf8");
const hash = (content: string) => createHash("sha256").update(content).digest("hex");

describe("AI Elements upstream source parity", () => {
  it("keeps all 21 runnable pattern demos identical to their original code blocks after local import resolution", async () => {
    const sources = JSON.parse(await read(`${upstream}/manifest.json`)).sources;
    const demos = Object.keys(pins.files).filter((file) => /\/demos\/[^/]+\.tsx$/.test(file));
    expect(demos).toHaveLength(21);
    for (const file of demos) {
      const name = path.basename(file, ".tsx");
      const original = await read(`${upstream}/patterns/${name}.md`);
      const source = sources.find((entry: { url: string }) => entry.url === `https://ui.shadcn.com/docs/ai-elements/base/${name}.md`);
      expect(hash(original), name).toBe(source.sha256);
      const code = original.split("```tsx\n")[1].split("\n```")[0]
        .replaceAll("@/lib/ai", "./lib/ai")
        .replaceAll("@/lib/message-animations", "./lib/message-animations")
        .replaceAll("@/components/markdown", "./lib/markdown");
      expect((await read(file)).trim(), name).toBe(code.trim());
    }
  });

  it("keeps every installed upstream file frozen to its reviewed hash", async () => {
    for (const [file, expected] of Object.entries(pins.files)) {
      expect(hash(await read(file)), file).toBe(expected);
    }
  });

  it("keeps Markdown and animation helpers identical after installation import resolution", async () => {
    const markdown = (await read(`${upstream}/helpers/markdown.tsx.txt`))
      .replace('from "cn"', 'from "@/ai-elements/class-names"')
      .replace("@/registry/bases/base/hooks/use-highlighted-code", "@/ai-elements/upstream/hooks/use-highlighted-code")
      .replace("@/styles/base-rhea/ui/code-block", "@/components/ui/code-block");
    expect(await read("src/renderer/src/ai-elements/demos/lib/markdown.tsx")).toBe(markdown);
    expect(await read("src/renderer/src/ai-elements/demos/lib/message-animations.ts"))
      .toBe(await read(`${upstream}/helpers/message-animations.ts.txt`));
  });

  it("retains the original registry payloads with their upstream source hashes", async () => {
    const sources = JSON.parse(await read(`${upstream}/manifest.json`)).sources;
    const registry = sources.filter((entry: { url: string }) => entry.url.startsWith("https://ui.shadcn.com/r/styles/base-rhea/"));
    expect(registry.length).toBeGreaterThan(20);
    for (const entry of registry) {
      const file = `${upstream}/registry/${entry.url.split("/").at(-1)}`;
      expect(hash(await read(file)), file).toBe(entry.sha256);
    }
  });
});
