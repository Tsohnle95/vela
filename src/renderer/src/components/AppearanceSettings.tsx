import type { ReactNode } from "react";
import { APPEARANCES, type UiShell, useTheme } from "../theme";

const UI_SHELLS: { id: UiShell; name: string; description: string }[] = [
  { id: "codex", name: "Codex interface", description: "Flat shell: icon rail, tabs in the top bar, agent panel docked right." },
  { id: "classic", name: "Classic interface", description: "The original chrome: floating cards and side-by-side panels." }
];

// Interface and colour are independent choices: the shell decides the chrome,
// the appearance only supplies colours, and either shell runs every appearance.
export function AppearanceSettings(): ReactNode {
  const { theme, setTheme, uiShell, setUiShell, experimentalArt, setExperimentalArt } = useTheme();

  return (
    <section className="settings-section appearance-settings" aria-label="Appearance settings">
      <div className="appearance-interface-row">
        <div className="appearance-label">
          <h2>Interface</h2>
          <p>Choose your workspace layout.</p>
        </div>
        <div className="ui-choice" role="radiogroup" aria-label="Interface">
          {UI_SHELLS.map((option) => (
            <button
              key={option.id}
              className={`ui-choice-card ${uiShell === option.id ? "selected" : ""}`}
              role="radio"
              aria-checked={uiShell === option.id}
              title={option.description}
              onClick={() => setUiShell(option.id)}
            >
              <strong>{option.name}</strong>
            </button>
          ))}
        </div>
      </div>
      <div className="appearance-label appearance-theme-heading">
        <h2>Color theme</h2>
        <p>Applies to your workspace, editor and terminal.</p>
      </div>
      <div className="theme-grid" role="radiogroup" aria-label="Color theme">
        {APPEARANCES.map((option) => (
          <button
            key={option.id}
            className={`theme-card ${theme === option.id ? "selected" : ""}`}
            role="radio"
            aria-checked={theme === option.id}
            aria-label={option.name}
            title={option.description}
            onClick={() => setTheme(option.id)}
          >
            <span
              className={`theme-preview theme-preview-${option.id}`}
              aria-hidden="true"
              style={{
                "--preview-base": option.colors.base,
                "--preview-pane": option.colors.pane,
                "--preview-editor": option.colors.editor,
                "--preview-agent": option.colors.agent,
                "--preview-terminal": option.colors.console,
                "--preview-accent": option.colors.accent,
                "--preview-good": option.colors.good,
                "--preview-muted": option.colors.muted
              } as React.CSSProperties}
            >
              <span className="theme-preview-rail" />
              <span className="theme-preview-sidebar" />
              <span className="theme-preview-editor"><i /><i /><i /></span>
              <span className="theme-preview-agent" />
              <span className="theme-preview-terminal" />
            </span>
            <span className="theme-card-copy"><strong>{option.name}</strong></span>
            <span className="theme-check" aria-hidden="true">
              {theme === option.id && <i className="codicon codicon-check" />}
            </span>
          </button>
        ))}
      </div>
      <div className="settings-list">
        <div className="settings-list-row">
          <div>
            <strong>Experimental art</strong>
            <small>Show optional artwork in the empty agent panel.</small>
          </div>
          <button
            type="button"
            className={`settings-switch ${experimentalArt ? "on" : ""}`}
            role="switch"
            aria-label="Experimental art"
            aria-checked={experimentalArt}
            title={`${experimentalArt ? "Disable" : "Enable"} experimental art`}
            onClick={() => setExperimentalArt(!experimentalArt)}
          >
            <span />
          </button>
        </div>
      </div>
    </section>
  );
}
