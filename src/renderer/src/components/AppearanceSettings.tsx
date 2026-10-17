import type { ReactNode } from "react";
import { APPEARANCES, type UiShell, useTheme } from "../theme";

const UI_SHELLS: { id: UiShell; name: string; description: string }[] = [
  { id: "codex", name: "Codex interface", description: "Flat shell: icon rail, tabs in the top bar, agent panel docked right." },
  { id: "classic", name: "Classic interface", description: "The original chrome: floating cards and side-by-side panels." }
];

// Interface and colour are independent choices: the shell decides the chrome,
// the appearance only supplies colours, and either shell runs every appearance.
export function AppearanceSettings(): ReactNode {
  const { theme, setTheme, uiShell, setUiShell } = useTheme();

  return (
    <section className="settings-section" aria-label="Appearance settings">
      <div className="ui-choice" role="radiogroup" aria-label="Interface">
        {UI_SHELLS.map((option) => (
          <button
            key={option.id}
            className={`ui-choice-card ${uiShell === option.id ? "selected" : ""}`}
            role="radio"
            aria-checked={uiShell === option.id}
            onClick={() => setUiShell(option.id)}
          >
            <strong>{option.name}</strong>
            <small>{option.description}</small>
          </button>
        ))}
      </div>
      <div className="theme-grid" role="radiogroup" aria-label="Color theme">
        {APPEARANCES.map((option) => (
          <button
            key={option.id}
            className={`theme-card ${theme === option.id ? "selected" : ""}`}
            role="radio"
            aria-checked={theme === option.id}
            onClick={() => setTheme(option.id)}
          >
            <span
              className={`theme-preview theme-preview-${option.id}`}
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
            <span className="theme-card-copy"><strong>{option.name}</strong><small>{option.description}</small></span>
            <span className="theme-swatches">{[option.colors.base, option.colors.pane, option.colors.accent, option.colors.good, option.colors.muted].map((color) => <i key={color} style={{ background: color }} />)}</span>
            <span className="theme-check">{theme === option.id ? "Selected" : "Select"}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
