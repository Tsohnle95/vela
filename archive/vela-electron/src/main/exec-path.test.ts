import { describe, expect, it } from "vitest";
import path from "node:path";
import { applyExecPath, augmentedPath } from "./exec-path";

describe("execution PATH", () => {
  it("prepends the conventional user tool directories", () => {
    const home = path.join(path.sep, "home", "sam");
    const result = augmentedPath("/usr/bin:/bin", home, "darwin");
    const dirs = result.split(path.delimiter);
    expect(dirs[0]).toBe(path.join(home, ".local", "bin"));
    expect(dirs[1]).toBe(path.join(home, "bin"));
    expect(dirs).toContain("/opt/homebrew/bin");
    expect(dirs).toContain("/usr/local/bin");
    expect(dirs.slice(-2)).toEqual(["/usr/bin", "/bin"]);
  });

  it("does not duplicate directories already on PATH", () => {
    const home = path.join(path.sep, "home", "sam");
    const local = path.join(home, ".local", "bin");
    const result = augmentedPath(`${local}:/usr/bin`, home, "linux");
    expect(result.split(path.delimiter).filter((dir) => dir === local)).toHaveLength(1);
  });

  it("leaves the Windows PATH untouched", () => {
    expect(augmentedPath("C:\\Windows;C:\\Tools", "C:\\Users\\sam", "win32")).toBe("C:\\Windows;C:\\Tools");
  });

  it("applyExecPath rewrites process.env.PATH", () => {
    process.env.VELA_TEST_PATH = "sentinel";
    const original = process.env.PATH;
    try {
      process.env.PATH = "/usr/bin:/bin";
      applyExecPath();
      expect(process.env.PATH).not.toBe("/usr/bin:/bin");
      expect(process.env.PATH?.split(path.delimiter)).toContain("/usr/bin");
      expect(process.env.VELA_TEST_PATH).toBe("sentinel");
    } finally {
      process.env.PATH = original;
    }
  });
});
