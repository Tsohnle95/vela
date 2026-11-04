import { readFileSync } from "node:fs";
import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";

const fixture = readFileSync("scripts/fixtures/compact-agent-designs.html", "utf8");
function gallery(): JSDOM {
  return new JSDOM(fixture, { runScripts: "dangerously", pretendToBeVisual: true });
}

describe("compact agent design exploration", () => {
  it("provides thirty numbered concepts with distinct composer structure and explanations", () => {
    const dom = gallery();
    const doc = dom.window.document;
    const concepts = [...doc.querySelectorAll(".catalog .concept")];
    expect(concepts).toHaveLength(30);
    expect(doc.querySelectorAll(".overview [data-select]")).toHaveLength(30);
    expect(new Set(concepts.map((concept) => concept.querySelector(".input-zone")!.className)).size).toBe(30);
    expect(new Set(concepts.map((concept) => concept.querySelector("h3")!.textContent)).size).toBe(30);
    for (const [index, concept] of concepts.entries()) {
      expect(concept.querySelector(".agent")?.getAttribute("data-design")).toBe(String(index + 1));
      expect(concept.querySelector(".note")!.textContent!.length).toBeGreaterThan(70);
      expect(concept.querySelector(".conversation")).not.toBeNull();
    }
    dom.window.close();
  });

  it("supports sending a local instruction and revealing controls in every concept", () => {
    const dom = gallery();
    const doc = dom.window.document;
    for (const agent of doc.querySelectorAll(".catalog .agent")) {
      const option = agent.querySelector<HTMLButtonElement>('[data-action="options"]')!;
      option.click();
      expect(agent.classList.contains("expanded")).toBe(true);
      expect(option.getAttribute("aria-expanded")).toBe("true");
      const field = agent.querySelector<HTMLInputElement | HTMLTextAreaElement>("input,textarea")!;
      field.value = "Keep this instruction visible <script> as text.";
      const form = agent.querySelector("form")!;
      form.dispatchEvent(new dom.window.Event("submit", { bubbles: true, cancelable: true }));
      expect(agent.querySelector(".conversation")!.lastElementChild!.textContent).toContain(field.value || "Keep this instruction visible <script> as text.");
      expect(agent.querySelector(".conversation script")).toBeNull();
      expect(field.value).toBe("");
      option.click();
      expect(agent.classList.contains("expanded")).toBe(false);
    }
    dom.window.close();
  });

  it("switches palettes, panel counts and selected design", () => {
    const dom = gallery();
    const doc = dom.window.document;
    expect(doc.querySelectorAll("#stage .agent")).toHaveLength(3);
    doc.querySelector<HTMLButtonElement>("#four")!.click();
    expect(doc.querySelectorAll("#stage .agent")).toHaveLength(4);
    doc.querySelector<HTMLButtonElement>('.overview [data-select="29"]')!.click();
    expect([...doc.querySelectorAll("#stage .agent")].every((panel) => panel.getAttribute("data-design") === "30")).toBe(true);
    doc.querySelector<HTMLButtonElement>("#theme")!.click();
    expect(doc.documentElement.dataset.theme).toBe("light");
    doc.querySelector<HTMLButtonElement>("#three")!.click();
    expect(doc.querySelectorAll("#stage .agent")).toHaveLength(3);
    dom.window.close();
  });
});
