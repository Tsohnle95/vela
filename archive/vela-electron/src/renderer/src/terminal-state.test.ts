import { describe, expect, it } from "vitest";
import { PendingTerminalOutput, removeTerminal, terminalDirectoryCommand, terminalSizeForViewport, type TerminalTabs } from "./terminal-state";

const tabs: TerminalTabs = {
  terms: [
    { id: "one", name: "Terminal 1" },
    { id: "two", name: "Terminal 2" },
    { id: "three", name: "Terminal 3" }
  ],
  activeId: "two"
};

describe("terminal tab lifecycle", () => {
  it("commits an empty state when the final terminal closes", () => {
    expect(removeTerminal({ terms: [tabs.terms[0]], activeId: "one" }, "one")).toEqual({
      terms: [],
      activeId: null
    });
  });

  it("selects the next neighbor after a natural exit", () => {
    expect(removeTerminal(tabs, "two")).toEqual({
      terms: [tabs.terms[0], tabs.terms[2]],
      activeId: "three"
    });
  });

  it("keeps the active terminal when a background terminal exits", () => {
    expect(removeTerminal(tabs, "one").activeId).toBe("two");
  });
});

describe("terminal directory fallback", () => {
  it("changes an older Unix terminal process into the requested folder", () => {
    expect(terminalDirectoryCommand("darwin", "/workspace/it's here", "packages/web")).toBe(
      "cd -- '/workspace/it'\\''s here/packages/web'\r"
    );
  });

  it("uses a literal PowerShell path on Windows", () => {
    expect(terminalDirectoryCommand("win32", "C:\\workspace", "packages/web's")).toBe(
      "Set-Location -LiteralPath 'C:\\workspace\\packages\\web''s'\r"
    );
  });

  it("does nothing for the workspace root", () => {
    expect(terminalDirectoryCommand("linux", "/workspace", "")).toBeNull();
  });
});

describe("initial terminal sizing", () => {
  it("uses the available terminal viewport with room for xterm padding and its ruler", () => {
    expect(terminalSizeForViewport(610, 190, 7.2, 15)).toEqual({ cols: 79, rows: 11 });
  });

  it("keeps a minimum valid PTY size for a very small viewport", () => {
    expect(terminalSizeForViewport(10, 10, 7.2, 15)).toEqual({ cols: 2, rows: 1 });
  });
});

describe("pending terminal output", () => {
  it("buffers startup output only for a known terminal and flushes it on registration", () => {
    const output = new PendingTerminalOutput();
    output.awaitRegistration("known");

    expect(output.write("stale", "discarded")).toBe(false);
    expect(output.write("known", "hello")).toBe(true);
    expect(output.register("known")).toEqual(["hello"]);
    expect(output.write("known", "late")).toBe(false);
  });

  it("keeps the newest complete chunks within byte and chunk limits", () => {
    const output = new PendingTerminalOutput(5, 2);
    output.awaitRegistration("term");
    output.write("term", "aa");
    output.write("term", "bb");
    output.write("term", "ccc");

    expect(output.register("term")).toEqual(["bb", "ccc"]);
  });

  it("expires stale registrations and clears session state", () => {
    let now = 0;
    const output = new PendingTerminalOutput(100, 10, 50, () => now);
    output.awaitRegistration("old");
    output.write("old", "old data");
    now = 51;
    expect(output.write("old", "new data")).toBe(false);

    output.awaitRegistration("session-terminal");
    output.write("session-terminal", "data");
    output.clear();
    expect(output.register("session-terminal")).toEqual([]);
  });
});
