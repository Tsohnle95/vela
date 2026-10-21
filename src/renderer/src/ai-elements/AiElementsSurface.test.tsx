import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { AiElementsSurface } from "./AiElementsSurface";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const container = document.createElement("div");
document.body.append(container);
const root = createRoot(container);
afterEach(() => { act(() => root.render(null)); vi.restoreAllMocks(); });

it("retains interactive response state as streaming children update", () => {
  function Response({ text }: { text: string }) {
    const [expanded, setExpanded] = useState(false);
    return <button onClick={() => setExpanded(!expanded)} aria-expanded={expanded}>{text}</button>;
  }
  act(() => root.render(<AiElementsSurface><Response text="partial"/></AiElementsSurface>));
  const shadow = container.querySelector("[data-ai-elements-surface]")!.shadowRoot!;
  const button = shadow.querySelector("button")!;
  act(() => button.click());
  act(() => root.render(<AiElementsSurface><Response text="completed"/></AiElementsSurface>));
  expect(shadow.querySelector("button")).toBe(button);
  expect(button.textContent).toBe("completed");
  expect(button.getAttribute("aria-expanded")).toBe("true");
});

it("keeps Base UI tooltip portals inside the styled response boundary", async () => {
  await act(async () => root.render(<AiElementsSurface><Tooltip open><TooltipTrigger>Copy</TooltipTrigger><TooltipContent>Copy response</TooltipContent></Tooltip></AiElementsSurface>));
  const shadow = container.querySelector("[data-ai-elements-surface]")!.shadowRoot!;
  expect(shadow.querySelector('[data-ai-elements-portals] [data-slot="tooltip-content"]')?.textContent).toBe("Copy response");
  expect(document.querySelector('[data-slot="tooltip-content"]')).toBeNull();
});

it("applies the shared external URL policy to anchors inside the shadow tree", () => {
  const open = vi.spyOn(window, "open").mockImplementation(() => null);
  act(() => root.render(<AiElementsSurface><a href="https://example.com/reference">Reference</a><a href="javascript:alert(1)">Unsafe</a></AiElementsSurface>));
  const shadow = container.querySelector("[data-ai-elements-surface]")!.shadowRoot!;
  act(() => { shadow.querySelectorAll("a").forEach(link => link.click()); });
  expect(open).toHaveBeenCalledExactlyOnceWith("https://example.com/reference", "_blank", "noopener,noreferrer");
});
