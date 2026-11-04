import { createContext, useContext, type ReactNode } from "react";

const PortalContainer = createContext<HTMLElement | null>(null);

export function AiElementsPortalProvider({ container, children }: { container: HTMLElement | null; children: ReactNode }) {
  return <PortalContainer.Provider value={container}>{children}</PortalContainer.Provider>;
}

export function useAiElementsPortalContainer(): HTMLElement | null {
  return useContext(PortalContainer);
}
