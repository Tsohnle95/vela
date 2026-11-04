import { useCallback, type ReactNode } from "react";
import { useBrowserTabActions } from "./browser-tabs-context";
import { StoreProvider } from "./store";

// The store needs the browser actions (opening or focusing a file leaves browser
// mode). They are stable, so handing them over as props keeps browser-tab state
// changes from re-rendering every store consumer — which is what makes opening
// and closing a tab instant.
export function BrowserStoreProvider({ children }: { children: ReactNode }): ReactNode {
  const actions = useBrowserTabActions();
  const onFileActivate = useCallback(() => actions.activateBrowserTab(null), [actions]);
  return <StoreProvider onFileActivate={onFileActivate}>{children}</StoreProvider>;
}
