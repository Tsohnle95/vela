import { spawn } from "node-pty";

const shell = process.platform === "win32" ? process.env.COMSPEC ?? "powershell.exe" : process.env.SHELL ?? (process.platform === "darwin" ? "/bin/zsh" : "/bin/bash");
const marker = `vela-pty-${process.pid}`;
const pty = spawn(shell, [], {
  name: "xterm-256color",
  cols: 80,
  rows: 24,
  cwd: process.cwd(),
  env: { ...process.env, TERM: "xterm-256color" }
});
let output = "";
const timeout = setTimeout(() => {
  console.error(`PTY smoke timed out: ${output}`);
  pty.kill();
  process.exit(1);
}, 10_000);
pty.onData((data) => {
  output += data;
});
pty.onExit(({ exitCode }) => {
  clearTimeout(timeout);
  if (exitCode !== 0 || !output.includes(marker)) {
    console.error(`PTY smoke failed (${exitCode}): ${output}`);
    process.exit(1);
    return;
  }
  process.exit(0);
});
pty.write(process.platform === "win32" ? `echo ${marker}\r\nexit\r\n` : `printf '${marker}\\n'\nexit\n`);
