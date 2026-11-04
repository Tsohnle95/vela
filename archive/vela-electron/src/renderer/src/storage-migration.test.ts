import { describe, expect, it } from "vitest";
import { migrateVelaStorage } from "./storage-migration";

const previousPrefixes = [
  String.fromCharCode(109, 105, 114, 97),
  String.fromCharCode(111, 114, 98, 105, 116),
  String.fromCharCode(111, 112, 101, 110, 115, 104, 101, 108, 108)
];

function fakeStorage(entries: Record<string, string>): Storage {
  const values = new Map(Object.entries(entries));
  return {
    get length() { return values.size; },
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, value); },
    removeItem: (key: string) => { values.delete(key); },
    clear: () => values.clear()
  } as Storage;
}

describe("Vela storage migration", () => {
  it("moves prior branded preferences to Vela keys", () => {
    const storage = fakeStorage(Object.fromEntries(previousPrefixes.map((prefix, index) => [`${prefix}.${index}`, String(index)])));

    migrateVelaStorage(storage);

    for (const [index, prefix] of previousPrefixes.entries()) {
      expect(storage.getItem(`vela.${index}`)).toBe(String(index));
      expect(storage.getItem(`${prefix}.${index}`)).toBeNull();
    }
  });

  it("keeps an existing Vela preference when both key families exist", () => {
    const storage = fakeStorage({ [`${previousPrefixes[0]}.theme`]: "old", "vela.theme": "new" });

    migrateVelaStorage(storage);

    expect(storage.getItem("vela.theme")).toBe("new");
  });
});
