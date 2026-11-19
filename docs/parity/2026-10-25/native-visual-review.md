# Native visual review — 2026-11-08

Scope: native-only visual inspection at 1280×800, isolated profile/workspace
(`/private/tmp/vela-visual-profile`, `/private/tmp/vela-visual-workspace`) with
the isolated OpenCode service on port 49375. Captures are lossless window PNGs
with matching JSON metadata under `native/`. No shared mouse/keyboard input was
used; screens were reached with `VELA_UI_ACTION`.

## Captures

| Capture | Native result |
|---|---|
| `initial` | Empty workspace: rail, sidebar, editor empty state, agent empty state, composer. |
| `file-readme` | README opened in the editor: tab, breadcrumb, explorer row, line numbers, split/overflow controls. |
| `settings-providers` | Preferences shell, section rail, Providers cards and Add-key actions. |
| `terminal` | Terminal tray open with tabs, prompt, and agent panel alongside. |
| `browser` | Browser tab, empty-address placeholder, rail/sidebar/agent chrome. |
| `model-picker` | Prompt model picker popover with search, provider group, favorites and current check. |
| `file-rust` | Rust file with syntax colors, breadcrumb and explorer nesting. |
| `narrow-file` | 1024×700 README view: sidebar, editor and agent panel all fit. |
| `800-file` | 800×600 README view. |
| `shell-classic` | Classic interface shell with rounded cards and 10px gutters. |
| `shell-codex` | Codex interface shell with edge-to-edge split surfaces. |
| `changes` | Changes review tab empty state. |
| `settings-appearance` | Appearance page: interface toggle, experimental art, full theme grid; Observatory uses Source's observatory.png art (`visual-settings-appearance-v4.png`). |
| `settings-model` | Model page: default model, safety toggle, OpenCode CLI/update cards. |
| `settings-servers` | Servers page. |
| `settings-mobile` | Mobile Setup page with phone-companion steps and pairing panel. |
| `settings-about` | About page with version and disabled updater. |
| `settings-voice` | Voice settings page. |

| `agent-mode-v6` | Full-width single Agent Mode panel: rail, centered empty state, composer. |

## Observations

- No clipping, overlap or missing chrome in the inspected surfaces at 1280×800.
- Inactive-window traffic lights render as hollow gray controls; the active
  `file-readme` capture shows colored lights, matching macOS window state.
- Settings cards and provider labels align cleanly; no text overflow observed.
- Model picker intentionally overlays the agent empty state and scrolls within
  its max height.
- Rust syntax colors and the nested `src/main.rs` breadcrumb render correctly.
- The Appearance grid renders all theme cards, including Observatory, without
  clipping at 1280×800; Observatory uses Source's `observatory.png` cover art
  instead of a generic skeleton.
- Model, Servers, Mobile Setup, About and Voice settings pages render without
  clipping at 1280×800; About correctly labels the development build and
  disabled updater.
- At 1024×700 the README editor, sidebar and agent panel still fit without
  overlap or clipped controls (`visual-narrow-file.png`).
- At 800×600 the panels still fit, but the breadcrumb collapses its segments to
  single letters (`v > f` in `visual-800-file.png`), losing the file path. This
  is a narrow-layout polish defect.
- Both shells render as expected: Classic uses rounded cards/gutters, Codex
  uses edge-to-edge surfaces with the same rail/sidebar/agent structure.
- Changes shows Source's empty state even with a modified Git file; this surface
  tracks files the agent edits, not Git status (matching current Source).
- Comparing stored `original/` pairs: settings-providers matches card-for-card
  and settings-about matches structurally; the Appearance runtime controls and
  theme grid match, with Observatory's source art now restored.
- Agent Mode automation enters the mode. The earlier single-panel capture showed
  a ~370px agent column with a blank right side (`visual-agent-mode-v5.png`);
  the panel now fills the workspace (`visual-agent-mode-v6.png`) after the
  flex/width fix.

## Limits

- The archived reference binary is absent, but 102 existing original captures
  live under `docs/parity/2026-10-25/original/`, with 91 names also present in
  `native/`. Those stored captures are the reference evidence; rebuilding the
  archive is unnecessary. This review's new fixture differs from the original
  fixture, so only structural/theme comparisons were drawn.
- These are native-only visual observations, not final parity acceptance.
- IME, VoiceOver, provider refresh, TUI and packaging remain unverified.
