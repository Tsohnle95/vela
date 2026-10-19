# Vela Design System

> **Source version:** `src/renderer/src/styles/_foundation.scss:1` + `main.scss:1` (5328 lines across 16 partials)
> **Themes:** `original` (dark) + `paper` (light, default) — toggled via `src/renderer/src/theme.tsx:10` (`document.documentElement.dataset.theme`, persisted as `vela.theme` in localStorage)
> **Brand mark:** `src/renderer/src/components/VelaMark.tsx:3`

This folder is the authoritative field guide to the current calm design language. Every token, component, and rule below is traced to its source file and line. The system is informally called **Paper / Calm** — warm parchment surfaces, muted sage, no harsh contrasts, generous radius, soft shadows, system type.

---

## 1. Philosophy

| Principle | What it means in practice |
|---|---|
| **Calm, not cold** | Warm parchment (`#fbf7ec` / `#f4eee1`) beats stark white. Sage `#617a68` beats electric blue. Every accent is desaturated and mixed with the surface color. |
| **Paper, not plastic** | No glassy cards or hard drop shadows. Surfaces are matte, layered by subtle tint steps (`bg` → `inset` → `panel` → `elev` → `raised`). Borders are translucent hairlines (`0.05–0.15` alpha). |
| **Space as hierarchy** | Hierarchy is whitespace + weight, not chrome. 9/12/16/20 px radii, 8 px scrollbars, 2 px separators, 1 px borders — nothing shouts. |
| **System type** | `-apple-system` UI stack everywhere except the welcome hero (Cormorant Garamond serif) and code (ui-monospace). Titles are 400–525 weight — never bold. |
| **Transparent lists** | File trees, session rows, composer menus are transparent items on flat surfaces with hover washes. No stacked cards, prominent borders, or shadows inside panels. (See `design/README.md:5`.) |
| **Accessible, not loud** | Focus rings are 1 px `@accent` outlines. Motion is `180–200 ms cubic-bezier(0.2,0,0,1)` — perceptible but never bouncy. Dark theme mirrors the same tokens with inverted luminance. |

---

## 2. Module map

| File | Role | Lines |
|---|---|---|
| `_foundation.scss:1` | CSS custom property tokens — the single source of truth | 169 |
| `_layout.scss:1` | Titlebar, `.main-row` grid, `.agent-col` floating panels | 155 |
| `_sidebar.scss:1` | Sidebar chrome, settings nav, side tabs, tree & ctx-menu | 664 |
| `_sessions.scss:1` | Sessions pane, project folding, session rows | 375 |
| `_editor.scss:1` | Tab bar, toolbar, Monaco host, empty state | 234 |
| `_agent.scss:1` | Floating agent panel, header, activity dock, usage popup | 666 |
| `_composer.scss:1` | Prompt composer, chips, menus, pills | 619 |
| `_opencode-chat.scss:1` | Chat transcript: turns, timeline, markdown, code, tools, todo | 1381 |
| `_welcome.scss:1` | Marketing/welcome splash, CTA, runtime picker | 403 |
| `_settings.scss:1` | Settings page, theme cards, provider grid | 233 |
| `_terminal.scss:1` | Bottom terminal tray, tray-area, tab strip | 190 |
| `_statusbar.scss:1` | Bottom status bar, validation spinner | 86 |
| `_buttons.scss:1` | Shared `.btn` family | 28 |
| `_toasts.scss:1` | Toast stack, recovery notice | 66 |
| `_scrollbars.scss:1` | Custom thin scrollbars | 3 |
| `_error-boundary.scss:1` | Crash fallback page | 40 |

`main.scss:1` is the import manifest (`@use "foundation"; @use "layout"; ...`).

---

## 3. How to use this folder

| Document | Covers |
|---|---|
| [01 — Tokens](01-tokens.md) | All CSS custom properties: colors, radii, shadows, motion, z-index. Both themes side-by-side. |
| [02 — Typography](02-typography.md) | Font stacks, scale, weights, spacing, monospace & serif usage. |
| [03 — Layout & Surfaces](03-layout-and-surfaces.md) | Titlebar, grid, sidebar/editor/agent columns, resizing handles, floating panels. |
| [04 — Navigation & Lists](04-navigation-and-lists.md) | Sessions pane, file tree, editor tabs, side tabs, context menus. |
| [05 — Chat & Transcript](05-chat-and-transcript.md) | Timeline, turns, user/assistant messages, markdown, code, thinking, tools, todos, dock prompt. |
| [06 — Composer & Input](06-composer-and-input.md) | Composer anatomy, inputs, attachments, chips, menus, pills, send states. |
| [07 — Overlays & Feedback](07-overlays-and-feedback.md) | Modals, popups (usage, plugin menu, composer menus), toasts, recovery notice, error boundary, statusbar. |
| [08 — Icons & Brand](08-icons-and-brand.md) | `VelaMark`, codicons, `icons.tsx` custom icons, `FileIcons.tsx`, resource `icon.svg`. |
| [09 — Motion, Elevation & Interaction](09-motion-elevation-interaction.md) | Transitions, hover/focus/active patterns, elevation model, drag & resize handles, focus rings. |

A companion [change log / audit appendix](10-audit.md) notes where the live product diverges from older redesign explorations (`redesign/rebrand.md`, `resources/icon.svg`).

---

## 4. Quick token preview

```
Paper (light, default)        Original (dark)
bg          #f4eee1            #171412
bg-panel    #fbf7ec            #262220
bg-inset    #eee5d4            #201d1b
bg-elev     #fffaf0            #2d2926
text        #2b2119            #e8e3dd
text-dim    #6b5f50            #a8a29e
text-faint  #948571            #8f8880
accent      #617a68            #9eb4a1
border      rgba(43,33,25,.075) rgba(255,255,255,.05)
radius  9 · 12 · 16 · 20 · 999px     ease  cubic-bezier(0.2,0,0,1)
```

The full tables live in `01-tokens.md`.

---

## 5. Theme switcher

```tsx
// src/renderer/src/theme.tsx:10
export type ThemeId = "original" | "paper";          // stored as "vela.theme"
document.documentElement.dataset.theme = theme;       // :root[data-theme="paper"] overrides :root
```

Every color token is declared twice — once in `:root` (`_foundation.scss:1`) and once in `:root[data-theme="paper"]` (`_foundation.scss:72`). Any new token must be added in both blocks. See `01-tokens.md` for the complete matrix.

---

## 6. Conventions that must not drift

- **Shared colors** — all components reference `var(--*)` tokens; never hard-coded hex.
- **Path convention** — tree paths are `/`-separated, relative to the session directory, no trailing slashes (see `AGENTS.md`).
- **IPC contract** — renderer state changes through `window.vela` (preload bridge); OpenCode traffic stays inside `src/main/opencode.ts`; DeepSeek traffic inside `src/main/runtimes/deepseek/`.
- **No code comments** — knowledge belongs here, not in the stylesheet.

---

*Generated by direct inspection of `src/renderer/src/styles/*` on 2026-08-23. When styles change, update this folder in the same commit — `docs/check` does not yet cover design-system drift.*
