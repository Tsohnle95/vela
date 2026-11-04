import { describe, expect, it } from "vitest";
import {
  DEFAULT_SESSION_SIZE,
  isWindowView,
  isWindowSize,
  parseSessionBounds,
  serializeSessionBounds
} from "./window-sizing";

describe("isWindowSize", () => {
  it("accepts finite sizes at or above the minimum", () => {
    expect(isWindowSize({ width: 200, height: 200 })).toBe(true);
    expect(isWindowSize({ width: 1280.4, height: 800 })).toBe(true);
  });

  it("rejects undersized, non-finite, and malformed values", () => {
    expect(isWindowSize({ width: 199, height: 400 })).toBe(false);
    expect(isWindowSize({ width: Number.NaN, height: 400 })).toBe(false);
    expect(isWindowSize({ width: "800", height: 400 })).toBe(false);
    expect(isWindowSize(null)).toBe(false);
    expect(isWindowSize("760x522")).toBe(false);
  });
});

describe("parseSessionBounds", () => {
  it("reads the versioned session profile", () => {
    expect(parseSessionBounds(serializeSessionBounds({ width: 1000, height: 700 }))).toEqual({
      width: 1000,
      height: 700
    });
  });

  it("migrates the legacy flat shape into the session profile", () => {
    expect(parseSessionBounds(JSON.stringify({ width: 900, height: 640 }))).toEqual({
      width: 900,
      height: 640
    });
  });

  it("returns null for empty, corrupt, or invalid payloads", () => {
    expect(parseSessionBounds(null)).toBeNull();
    expect(parseSessionBounds("")).toBeNull();
    expect(parseSessionBounds("{oops")).toBeNull();
    expect(parseSessionBounds(JSON.stringify({ version: 2, session: { width: 10, height: 10 } }))).toBeNull();
    expect(parseSessionBounds(JSON.stringify({ version: 2 }))).toBeNull();
  });
});

describe("isWindowView", () => {
  it("narrows to the two known views", () => {
    expect(isWindowView("landing")).toBe(true);
    expect(isWindowView("session")).toBe(true);
    expect(isWindowView("workspace")).toBe(false);
    expect(isWindowView(undefined)).toBe(false);
  });
});

describe("DEFAULT_SESSION_SIZE", () => {
  it("is itself a valid window size", () => {
    expect(isWindowSize(DEFAULT_SESSION_SIZE)).toBe(true);
  });
});
