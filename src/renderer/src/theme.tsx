import { createContext, useContext, useLayoutEffect, useMemo, useState, type ReactNode } from "react";
import { APPEARANCES, appearanceForId, type AppearanceColors, type AppearanceId } from "./appearances";

export type ThemeId = AppearanceId;

// Two independent axes: the *appearance* supplies colours, the *shell* decides
// the chrome (the Codex shell is the new UI, `classic` is the original card
// chrome). Every appearance works in either shell.
export type UiShell = "codex" | "classic";

export { APPEARANCES };

interface ThemeContextValue {
  theme: ThemeId;
  setTheme: (theme: ThemeId) => void;
  uiShell: UiShell;
  setUiShell: (shell: UiShell) => void;
}

const THEME_KEY = "mira.theme";
const SHELL_KEY = "mira.ui";
const LEGACY_THEME_KEY = "orbit.theme";
const LEGACY_SHELL_KEY = "orbit.ui";
const ThemeContext = createContext<ThemeContextValue | null>(null);

// File-tree glyph colours for dark schemes; the light set is the foundation
// default. Design constant, not a palette.
const DARK_TREE_ICONS: Record<string, string> = {
  "--trees-icon-gray": "#adadb1",
  "--trees-icon-red": "#ff6762",
  "--trees-icon-vermilion": "#d5512f",
  "--trees-icon-orange": "#ffa359",
  "--trees-icon-yellow": "#ffd452",
  "--trees-icon-green": "#5ecc71",
  "--trees-icon-teal": "#64d1db",
  "--trees-icon-cyan": "#68cdf2",
  "--trees-icon-blue": "#69b1ff",
  "--trees-icon-indigo": "#9d6afb",
  "--trees-icon-purple": "#d568ea",
  "--trees-icon-pink": "#ff678d",
  "--trees-icon-mauve": "#79697b"
};

const OBSERVATORY_VARIABLES: Record<string, string> = {
  "--trees-icon-gray": "#b5a9ef",
  "--trees-icon-indigo": "#aa83ff",
  "--trees-icon-purple": "#f143ef",
  "--trees-icon-pink": "#f143ef",
  "--trees-icon-yellow": "#ffd658",
  "--agent-composer-border": "#8d70df",
  "--agent-send-background": "#a58aff"
};

function storedShell(): UiShell {
  return (window.localStorage.getItem(SHELL_KEY) ?? window.localStorage.getItem(LEGACY_SHELL_KEY)) === "classic" ? "classic" : "codex";
}

function storedTheme(): ThemeId {
  const value = window.localStorage.getItem(THEME_KEY) ?? window.localStorage.getItem(LEGACY_THEME_KEY);
  const stored = APPEARANCES.find((appearance) => appearance.id === value);
  if (stored) return stored.id;
  if (value === "paper") return "papertrail";
  return "codex";
}

function neutralHover(shell: UiShell, colors: AppearanceColors, amount: number): string {
  const base = shell === "codex" ? colors.ink : colors.accent;
  return `color-mix(in srgb, ${base} ${amount}%, transparent)`;
}

function accentMix(colors: AppearanceColors, amount: number): string {
  return `color-mix(in srgb, ${colors.accent} ${amount}%, transparent)`;
}

// Tokens the Codex shell needs on top of any palette's colours.
function codexShellVariables(colors: AppearanceColors, scheme: "dark" | "light"): Record<string, string> {
  return {
    "--workspace-background": colors.base,
    "--agent-composer-border": colors.lineStrong,
    "--agent-send-background": colors.accent,
    "--agent-send-hover": `color-mix(in srgb, ${colors.accent} 88%, ${scheme === "light" ? "black" : "white"})`,
    ...(scheme === "dark" ? DARK_TREE_ICONS : {})
  };
}

function originalVariables(colors: AppearanceColors): Record<string, string> {
  return {
    "--workspace-background": colors.base,
    "--agent-composer-background": "linear-gradient(118deg, rgba(148, 174, 151, 0.045), transparent 34%), linear-gradient(300deg, rgba(196, 181, 154, 0.03), transparent 42%), #262220",
    "--agent-composer-border": "rgba(148, 174, 151, 0.18)",
    "--agent-composer-shadow": "0 4px 16px rgba(0, 0, 0, 0.28), 0 0 16px rgba(148, 174, 151, 0.025)"
  };
}

// Variables applied by the last call, so switching palettes or shells cannot
// leave stale inline tokens behind (an appearance's glass overrides, the shell's
// dark glyph set) on the document element.
let appliedVariables: string[] = [];

function applyAppearance(theme: ThemeId, uiShell: UiShell): void {
  const { colors, scheme } = appearanceForId(theme);
  const root = document.documentElement;
  const variables: Record<string, string> = {
    "--bg": colors.base,
    "--bg-panel": colors.pane,
    "--bg-inset": colors.tab,
    "--bg-elev": colors.card,
    "--bg-active-pill": colors.selected,
    "--bg-hover": neutralHover(uiShell, colors, 7),
    "--bg-active": neutralHover(uiShell, colors, 16),
    "--vscode-menu-foreground": colors.ink,
    "--vscode-menu-background": colors.card,
    "--vscode-menu-selectionForeground": colors.ink,
    "--vscode-menu-selectionBackground": colors.selected,
    "--vscode-menu-border": colors.lineStrong,
    "--vscode-menu-separatorBackground": colors.line,
    "--vscode-disabledForeground": colors.faint,
    "--panel-surface-color": colors.pane,
    "--panel-float-color": colors.agent,
    "--panel-surface-image": "none",
    "--panel-surface-size": "auto",
    "--panel-surface-repeat": "no-repeat",
    "--workspace-background": `radial-gradient(ellipse at 66% 15%, ${colors.glow} 0, transparent 44%), ${colors.base}`,
    "--rail-surface": colors.rail,
    "--top-surface": colors.top,
    "--editor-surface": colors.editor,
    "--terminal-surface": colors.console,
    "--status-surface": colors.status,
    "--agent-bg-base": colors.agent,
    "--agent-bg-deep": colors.base,
    "--agent-bg-layer": colors.card,
    "--agent-bg-hover": neutralHover(uiShell, colors, 7),
    "--agent-bg-active": neutralHover(uiShell, colors, 13),
    "--agent-user-bubble": colors.user,
    "--agent-border-muted": colors.lineSoft,
    "--agent-border-base": colors.line,
    "--agent-border-strong": colors.lineStrong,
    "--agent-chat-ink": colors.muted,
    "--agent-composer-background": colors.compose,
    "--agent-composer-border": `color-mix(in srgb, ${colors.accent} 48%, ${colors.line})`,
    "--agent-composer-shadow": colors.composeShadow,
    "--agent-composer-focus-border": colors.accent,
    "--agent-composer-focus-shadow": `0 8px 26px rgba(0,0,0,.28), 0 0 0 3px ${accentMix(colors, 15)}`,
    "--agent-send-background": `linear-gradient(160deg, ${colors.accent}, color-mix(in srgb, ${colors.accent} 78%, ${colors.base}))`,
    "--agent-send-hover": `linear-gradient(160deg, color-mix(in srgb, ${colors.accent} 84%, white), ${colors.accent})`,
    "--agent-send-color": colors.sendInk,
    "--agent-send-shadow": `0 5px 16px ${accentMix(colors, 30)}`,
    "--agent-send-hover-shadow": `0 6px 20px ${accentMix(colors, 40)}`,
    "--text": colors.ink,
    "--text-dim": colors.muted,
    "--text-faint": colors.faint,
    "--text-base": colors.ink,
    "--text-weak": colors.faint,
    "--accent": colors.accent,
    "--accent-hover": `color-mix(in srgb, ${colors.accent} 84%, white)`,
    "--accent-dim": accentMix(colors, 18),
    "--accent-tint": accentMix(colors, 10),
    "--green": colors.good,
    "--red": colors.danger,
    "--yellow": colors.syntaxString,
    "--sky": colors.syntaxFunction,
    "--border": colors.line,
    "--border-strong": colors.lineStrong,
    "--border-subtle": colors.lineSoft,
    "--fg": colors.ink,
    "--bg-raised": colors.card,
    "--interactive-hover": neutralHover(uiShell, colors, 7),
    "--on-accent": colors.sendInk,
    "--danger": colors.danger,
    "--danger-bg": `color-mix(in srgb, ${colors.danger} 14%, transparent)`,
    "--streaming-ink": colors.accent
  };
  if (theme === "kitty") Object.assign(variables, {
    "--bg": "transparent",
    "--workspace-background": "transparent",
    "--panel-surface-color": "rgba(2, 2, 4, 0.08)",
    "--panel-float-color": "rgba(14, 18, 28, 0.44)",
    "--panel-surface-image": "radial-gradient(900px 580px at 100% 0%, rgba(0, 162, 206, 0.10), transparent 66%), linear-gradient(135deg, rgba(67, 36, 43, 0.20), transparent 46%)",
    "--panel-surface-size": "cover",
    "--vscode-menu-background": "rgba(16, 22, 36, 0.96)",
    "--agent-bg-base": "rgba(2, 2, 4, 0.38)",
    "--agent-bg-deep": "rgba(2, 2, 4, 0.18)",
    "--agent-bg-layer": "rgba(52, 58, 85, 0.38)",
    "--agent-composer-background": "linear-gradient(118deg, rgba(0, 162, 206, 0.10), transparent 36%), linear-gradient(300deg, rgba(67, 36, 43, 0.15), transparent 44%), rgba(14, 18, 28, 0.34)",
    "--agent-composer-border": "rgba(0, 162, 206, 0.32)",
    "--agent-composer-shadow": "0 7px 24px rgba(0, 0, 0, 0.38), inset 0 1px rgba(231, 231, 238, 0.06)",
    "--agent-composer-focus-border": "rgba(0, 162, 206, 0.64)",
    "--agent-composer-focus-shadow": "0 8px 26px rgba(0, 0, 0, 0.42), 0 0 0 3px rgba(0, 162, 206, 0.12)"
  });
  if (theme === "original") Object.assign(variables, originalVariables(colors));
  if (uiShell === "codex") Object.assign(variables, codexShellVariables(colors, scheme));
  if (theme === "observatory") Object.assign(variables, OBSERVATORY_VARIABLES);
  root.dataset.theme = theme;
  root.dataset.shell = uiShell;
  root.style.colorScheme = scheme;
  for (const property of appliedVariables) if (!(property in variables)) root.style.removeProperty(property);
  for (const [property, color] of Object.entries(variables)) root.style.setProperty(property, color);
  appliedVariables = Object.keys(variables);
}

export function ThemeProvider({ children }: { children: ReactNode }): ReactNode {
  const [theme, setTheme] = useState<ThemeId>(storedTheme);
  const [uiShell, setUiShell] = useState<UiShell>(storedShell);

  useLayoutEffect(() => {
    const appearance = appearanceForId(theme);
    applyAppearance(theme, uiShell);
    window.localStorage.setItem(THEME_KEY, theme);
    window.localStorage.setItem(SHELL_KEY, uiShell);
    // Keep the native appearance (macOS window vibrancy and native chrome) on
    // the theme's side of light/dark; a light system appearance would
    // otherwise wash the transparent glass profiles out with a near-white
    // material. The main process applies this to `nativeTheme.themeSource`.
    void window.openshell?.setAppearance?.(appearance.scheme)?.catch(() => {});
  }, [theme, uiShell]);

  const value = useMemo<ThemeContextValue>(
    () => ({ theme, setTheme, uiShell, setUiShell }),
    [theme, uiShell]
  );
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("useTheme must be used within ThemeProvider");
  return value;
}

export function useOptionalTheme(): ThemeContextValue | null {
  return useContext(ThemeContext);
}
