import { useLayoutEffect, type ReactNode } from "react";
import ReactDOM from "react-dom/client";
import "@vscode/codicons/dist/codicon.css";
import "@fontsource/cormorant-garamond/500.css";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./styles/main.scss";

function LaunchReady({ children }: { children: ReactNode }): ReactNode {
  useLayoutEffect(() => {
    document.querySelector("#launch-screen")?.remove();
    performance.mark("mira-launch-workspace");
  }, []);
  return children;
}

export function mountApp(): void {
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <LaunchReady>
      <ErrorBoundary><App /></ErrorBoundary>
    </LaunchReady>
  );
}
