import { afterEach, describe, expect, it, vi } from "vitest";
import { tuiCommandForRuntime } from "./tui-command";

describe("runtime TUI commands", () => {
  afterEach(() => vi.unstubAllEnvs());
  it("resumes OpenCode sessions", () => {
    expect(tuiCommandForRuntime("opencode", "session-1")).toEqual({ command: "opencode", args: ["--session", "session-1"], env: { OPENCODE_CLI_CONFIG_CONTENT: JSON.stringify({ theme: { name: "system", mode: "system" } }) } });
  });

  it("overrides only the embedded TUI palette while retaining other inline preferences", () => {
    vi.stubEnv("OPENCODE_CLI_CONFIG_CONTENT", JSON.stringify({ theme: { name: "catppuccin" }, animations: true, session: { verbosity: "low" } }));
    const command = tuiCommandForRuntime("opencode", "session-1");
    expect(JSON.parse(command.env!.OPENCODE_CLI_CONFIG_CONTENT!)).toEqual({ theme: { name: "system", mode: "system" }, animations: true, session: { verbosity: "low" } });
  });

  it("retains JSONC inline preferences supported by OpenCode", () => {
    vi.stubEnv("OPENCODE_CLI_CONFIG_CONTENT", '{ /* inline preference */ "animations": true, }');
    const command = tuiCommandForRuntime("opencode", "session-1");
    expect(JSON.parse(command.env!.OPENCODE_CLI_CONFIG_CONTENT!).animations).toBe(true);
  });

  it("rejects DeepSeek TUI sessions while that runtime is disabled", () => {
    expect(() => tuiCommandForRuntime("deepseek", "session-2")).toThrow("TUI is not supported");
  });

  it("rejects unknown runtimes", () => {
    expect(() => tuiCommandForRuntime("unknown", "session-3")).toThrow("TUI is not supported");
  });
});
