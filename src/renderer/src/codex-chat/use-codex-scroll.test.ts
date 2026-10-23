import { describe, expect, it } from "vitest";
import {
  BOTTOM_THRESHOLD_PX,
  distanceFromBottom,
  easeOutCubic
} from "./use-codex-scroll";

describe("distanceFromBottom", () => {
  it("measures from the floor and clamps at zero", () => {
    expect(distanceFromBottom({ scrollHeight: 1000, clientHeight: 200, scrollTop: 800 })).toBe(0);
    expect(distanceFromBottom({ scrollHeight: 1000, clientHeight: 200, scrollTop: 100 })).toBe(700);
    expect(distanceFromBottom({ scrollHeight: 1000, clientHeight: 200, scrollTop: 900 })).toBe(0);
  });

  it("uses a 24px bottom threshold", () => {
    expect(BOTTOM_THRESHOLD_PX).toBe(24);
  });
});

describe("easeOutCubic", () => {
  it("runs from 0 to 1 with a decelerating curve", () => {
    expect(easeOutCubic(0)).toBe(0);
    expect(easeOutCubic(1)).toBe(1);
    expect(easeOutCubic(0.5)).toBeGreaterThan(0.5);
  });
});
