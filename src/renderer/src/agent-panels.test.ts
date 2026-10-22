import type { SessionInfo } from "@shared/types";
import { describe, expect, it } from "vitest";
import { agentPanelGrid, distributePanelSlots, MAX_AGENT_PANELS } from "./agent-panels";

describe("agent panel limits and grid", () => {
  it("supports eight panels in a two-row grid", () => {
    expect(MAX_AGENT_PANELS).toBe(8);
    expect(agentPanelGrid(8)).toEqual({ columns: 4, rows: 2 });
  });

  it.each([
    [1, { columns: 1, rows: 1 }],
    [2, { columns: 2, rows: 1 }],
    [3, { columns: 2, rows: 2 }],
    [4, { columns: 2, rows: 2 }],
    [5, { columns: 3, rows: 2 }],
    [6, { columns: 3, rows: 2 }],
    [7, { columns: 4, rows: 2 }]
  ])("lays out %i panels", (count, expected) => {
    expect(agentPanelGrid(count as number)).toEqual(expected);
  });
});

describe("agent grid row coverage", () => {
  it.each([3, 4, 5, 6, 7, 8])("fills each row with %i panels, including after resizing", (count) => {
    const panels = Array.from({ length: count }, (_, index) => ({ workspace: { id: String(index) } }) as SessionInfo);
    for (const area of [1480, 1637, 901]) {
      const slots = Object.values(distributePanelSlots({}, panels, area, false, true));
      for (const top of [0, 50]) {
        const row = slots.filter((slot) => slot.top === top).sort((a, b) => a.left - b.left);
        expect(row[0].left).toBe(0);
        row.forEach((slot, index) => {
          expect(slot.left + slot.width).toBe(index + 1 < row.length ? row[index + 1].left : area);
        });
        expect(Math.max(...row.map((slot) => slot.width)) - Math.min(...row.map((slot) => slot.width))).toBeLessThanOrEqual(1);
      }
    }
  });
});
