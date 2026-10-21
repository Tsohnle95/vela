import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node-pty";

const report = await mkdtemp(join(tmpdir(), "vela-tui-theme-"));
const palettes = [
  { name: "daybreak", background: "#ffffff", foreground: "#0d0d0d", accent: "#3a83f7" },
  { name: "midnight", background: "#181818", foreground: "#ffffff", accent: "#63a8f8" }
];
const rgb = (hex) => hex.slice(1).match(/../g).map((byte) => byte + byte).join("/");
const sgr = (hex) => hex.slice(1).match(/../g).map((byte) => parseInt(byte, 16)).join(";");

for (const palette of palettes) {
  const child = spawn(process.env.OPENCODE_BIN ?? "opencode", [], {
    name: "xterm-256color", cols: 100, rows: 36, cwd: report,
    env: {
      ...process.env, TERM: "xterm-256color", COLORTERM: "truecolor",
      OPENCODE_CLI_CONFIG_CONTENT: JSON.stringify({ theme: { name: "system", mode: "system" }, attention: { sound: false } })
    }
  });
  let output = "";
  let queries = 0;
  let exited = false;
  const exit = new Promise((resolve) => child.onExit(() => { exited = true; resolve(); }));
  child.onData((data) => {
    output += data;
    for (const match of data.matchAll(/\x1b\](10|11);\?(?:\x07|\x1b\\)/g)) {
      queries += 1;
      child.write(`\x1b]${match[1]};rgb:${rgb(match[1] === "10" ? palette.foreground : palette.background)}\x1b\\`);
    }
    for (const match of data.matchAll(/\x1b\]4;(\d+);\?(?:\x07|\x1b\\)/g)) {
      queries += 1;
      const index = Number(match[1]);
      const color = index === 0 ? palette.background : index === 7 || index === 15 ? palette.foreground : palette.accent;
      child.write(`\x1b]4;${index};rgb:${rgb(color)}\x1b\\`);
    }
    if (data.includes("\x1b[6n")) child.write("\x1b[1;1R");
    if (data.includes("\x1b[c")) child.write("\x1b[?1;2c");
  });
  try {
    await new Promise((resolve) => setTimeout(resolve, 6_000));
    assert.equal(exited, false, `${palette.name}: OpenCode exited before rendering`);
    const finalFrame = output.slice(output.lastIndexOf("\x1b[?2026h"));
    assert.ok(queries >= 18, `${palette.name}: missing OSC terminal palette queries`);
    assert.ok(finalFrame.includes(`38;2;${sgr(palette.foreground)}m`), `${palette.name}: incorrect foreground`);
    assert.ok(finalFrame.includes(`38;2;${sgr(palette.accent)}m`), `${palette.name}: incorrect accent`);
    assert.ok(finalFrame.includes("\x1b[49m"), `${palette.name}: base surface does not use terminal background`);
    console.log(`${palette.name}: actual OpenCode TUI used terminal colors (${queries} OSC queries)`);
  } finally {
    await writeFile(join(report, `${palette.name}.ansi`), output);
    if (!exited) child.kill();
    await exit;
  }
}
console.log(`TUI color evidence: ${report}`);
