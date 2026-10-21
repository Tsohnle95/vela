import JSON5 from "json5";
import type { RuntimeID } from "@shared/types";
import type { TerminalCommand } from "./terminal";

export function tuiCommandForRuntime(runtimeID: RuntimeID, sessionID: string): TerminalCommand {
  if (runtimeID === "opencode") {
    const inline = process.env.OPENCODE_CLI_CONFIG_CONTENT;
    const preferences = inline ? JSON5.parse(inline) as Record<string, unknown> : {};
    return {
      command: "opencode",
      args: ["--session", sessionID],
      env: {
        // OpenCode derives its system theme from xterm's OSC palette replies.
        // Override only the child CLI appearance; never rewrite global settings.
        OPENCODE_CLI_CONFIG_CONTENT: JSON.stringify({ ...preferences, theme: { name: "system", mode: "system" } })
      }
    };
  }
  throw new Error(`TUI is not supported by runtime ${runtimeID}`);
}
