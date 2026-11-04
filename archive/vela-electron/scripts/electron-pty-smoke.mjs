import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import electron from "electron";

function killProcessTree(child) {
  if (typeof child.pid !== "number") return;
  if (process.platform === "win32") {
    spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"], { stdio: "ignore" });
    return;
  }
  try {
    process.kill(-child.pid, "SIGTERM");
  } catch {
    child.kill("SIGTERM");
  }
}

if (process.argv[2] !== "--child") {
  const child = spawn(electron, [fileURLToPath(import.meta.url), "--child"], {
    env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
    stdio: "inherit",
    detached: process.platform !== "win32",
    windowsHide: true
  });
  const result = await new Promise((resolve) => {
    let settled = false;
    let timeout;
    const finish = (value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      resolve(value);
    };
    timeout = setTimeout(() => {
      killProcessTree(child);
      finish({ timedOut: true });
    }, 20_000);
    child.once("error", (error) => finish({ error }));
    child.once("close", (code, signal) => finish({ code, signal }));
  });
  if (result.error) throw result.error;
  if (result.timedOut) {
    console.error("Electron PTY smoke timed out");
    process.exit(1);
  }
  process.exitCode = result.code ?? 1;
} else {
  const { spawn } = await import("node-pty");
  const shell = process.platform === "win32" ? process.env.COMSPEC ?? "powershell.exe" : process.env.SHELL ?? (process.platform === "darwin" ? "/bin/zsh" : "/bin/bash");
  const marker = `vela-electron-pty-${process.pid}`;
  const pty = spawn(shell, [], {
    name: "xterm-256color",
    cols: 80,
    rows: 24,
    cwd: process.cwd(),
    env: { ...process.env, TERM: "xterm-256color" }
  });
  let output = "";
  const timeout = setTimeout(() => {
    console.error(`Electron PTY smoke timed out: ${output}`);
    pty.kill();
    process.exit(1);
  }, 10_000);
  pty.onData((data) => {
    output += data;
  });
  pty.onExit(({ exitCode }) => {
    clearTimeout(timeout);
    if (exitCode !== 0 || !output.includes(marker)) {
      console.error(`Electron PTY smoke failed (${exitCode}): ${output}`);
      process.exit(1);
      return;
    }
    process.exit(0);
  });
  pty.write(process.platform === "win32" ? `echo ${marker}\r\nexit\r\n` : `printf '${marker}\\n'\nexit\n`);
}
