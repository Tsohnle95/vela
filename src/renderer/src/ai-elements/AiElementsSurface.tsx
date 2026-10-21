import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import styles from "./ai-elements.css?inline";
import presentation from "./app-presentation.css?inline";
import { safeExternalUrl } from "@shared/url-policy";
import { AiElementsPortalProvider } from "./portal-container";

export function AiElementsSurface({ children }: { children: ReactNode }) {
  const host = useRef<HTMLDivElement>(null);
  const [root, setRoot] = useState<ShadowRoot | null>(null);
  const [container, setContainer] = useState<HTMLDivElement | null>(null);
  const [dark, setDark] = useState(false);

  useLayoutEffect(() => {
    if (!host.current) return;
    // Chromium registers @property only in document stylesheets, including for shadow utilities.
    if (!document.querySelector("style[data-ai-elements-properties]")) {
      const properties = document.createElement("style");
      properties.dataset.aiElementsProperties = "";
      properties.textContent = styles.match(/@property\s+[\w-]+\s*\{[^}]*\}/g)?.join("\n") ?? "";
      document.head.append(properties);
    }
    setRoot(host.current.shadowRoot ?? host.current.attachShadow({ mode: "open" }));
    const html = document.documentElement;
    const syncTheme = () => setDark(getComputedStyle(html).colorScheme === "dark");
    syncTheme();
    const observer = new MutationObserver(syncTheme);
    observer.observe(html, { attributes: true, attributeFilter: ["class", "style"] });
    return () => observer.disconnect();
  }, []);

  return <div ref={host} data-ai-elements-surface="" style={{ minWidth: 0 }}>
    {root && createPortal(<>
      <style>{`:host { all: initial; display: block; min-width: 0; }\n${styles}\n${presentation}`}</style>
      <div onClickCapture={(event) => {
        const link = event.target instanceof Element ? event.target.closest("a") : null;
        if (!link) return;
        event.preventDefault();
        const href = safeExternalUrl(link.getAttribute("href") ?? undefined);
        if (href) window.open(href, "_blank", "noopener,noreferrer");
      }} className={`ai-elements min-w-0 bg-background text-foreground${dark ? " dark" : ""}`} style={{ colorScheme: dark ? "dark" : "light" }}>
        <AiElementsPortalProvider container={container}>
          <div className="flex min-w-0 flex-col gap-6 px-8 py-6" data-ai-elements-content="">{children}</div>
          <div ref={setContainer} data-ai-elements-portals="" />
        </AiElementsPortalProvider>
      </div>
    </>, root)}
  </div>;
}
