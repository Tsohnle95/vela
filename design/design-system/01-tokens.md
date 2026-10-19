# 01 — Tokens

> **Source:** `src/renderer/src/styles/_foundation.scss:1`
> **Theme switch:** `src/renderer/src/theme.tsx:10` — `ThemeId = "original" | "paper"`, default `paper`, persisted as `vela.theme`, applied as `document.documentElement.dataset.theme`

Every visual value in the product is a CSS custom property on `:root`. Paper overrides live in `:root[data-theme="paper"]`. There are no SCSS variables — tokens are consumed at runtime so the theme can flip without recompilation.

---

## 1. Color tokens (complete matrix)

### 1.1 Surfaces

| Token | `original` (dark) `_foundation.scss:2` | `paper` (light) `_foundation.scss:74` | Used for |
|---|---|---|---|
| `--bg` | `#171412` | `#f4eee1` | App chrome, titlebar (`_layout.scss:56`), welcome canvas (`_welcome.scss:2`), error boundary (`_error-boundary.scss:9`) |
| `--bg-panel` | `#262220` | `#fbf7ec` | Sidebar (`_sidebar.scss:3`), welcome frame (`_welcome.scss:14`), settings cards (`_settings.scss:86`), `.app` background (`_layout.scss:8`) |
| `--bg-inset` | `#201d1b` | `#eee5d4` | Composer body (`_composer.scss:24`), settings page (`_settings.scss:8`), tabbar (`_editor.scss:38`), sidebar seg (`_sidebar.scss:145`), search fields |
| `--bg-elev` | `#2d2926` | `#fffaf0` | Context menus (`_sidebar.scss:580`), composer menu (`_composer.scss:423`), plugin menu (`_sessions.scss:88`), usage popup (`_agent.scss:257`), tabbar active pill fallback |
| `--bg-raised` | `#2d2926` | `#fffaf0` | Buttons (`_buttons.scss:2`), composer pills (`_composer.scss:385`), usage provider cards (`_agent.scss:487`), settings list (`_settings.scss:126`) — identical to `--bg-elev` in current build |
| `--bg-active-pill` | `#37322e` | `#e3d7c4` | Active tab (`_editor.scss:60`), active side-tab indicator, segmented control `.on` (`_sidebar.scss:166`), welcome tab `.on` (`_welcome.scss:265`) |
| `--bg-hover` | `rgba(255,255,255,0.05)` | `rgba(43,33,25,0.055)` | Every row hover: tree (`_sidebar.scss:451`), session (`_sessions.scss:283`), tab (`_editor.scss:57`), settings nav (`_sidebar.scss:99`) |
| `--bg-active` | `rgba(148,174,151,0.14)` | `rgba(97,122,104,0.13)` | Active row wash (`session-panels.css:921`), count pill (`_sidebar.scss:301`), modified badge (`_sidebar.scss:551`) |
| `--interactive-hover` | `rgba(255,255,255,0.06)` | `rgba(43,33,25,0.065)` | Settings-row hover, approval hover alias |
| `--panel-surface-color` | `#262220` | `#fbf7ec` | Sidebar bg alias (`_sidebar.scss:3`), editor pane bg (`_editor.scss:3`) |
| `--panel-float-color` | `rgba(38,34,32,0.92)` | `rgba(251,247,236,0.94)` | Agent panel floating bg (`_agent.scss:4`) — translucent so the backdrop blur shows |
| `--panel-surface-image` / `size` / `repeat` / `aura-x` | `none` / `auto` / `no-repeat` / `50%` | same | Hooks for optional paper texture / aura — currently neutral. Sidebar overrides `--panel-aura-x` to `0%` (`_sidebar.scss:2`), agent panel to `100%` (`_agent.scss:3`) — prepares a gradient aura per panel without adding markup. |

### 1.2 Agent panel aliases

These mirror surface tokens so `_opencode-chat.scss` can stay agnostic to theme:

| Token | original | paper |
|---|---|---|
| `--agent-bg-base` `_foundation.scss:15` | `#262220` | `#fbf7ec` |
| `--agent-bg-deep` `_foundation.scss:16` | `#1c1917` | `#f4eee1` |
| `--agent-bg-layer` `_foundation.scss:17` | `#37322e` | `#eee5d4` |
| `--agent-bg-hover` `_foundation.scss:18` | `rgba(255,255,255,0.05)` | `rgba(43,33,25,0.055)` |
| `--agent-bg-active` `_foundation.scss:19` | `rgba(255,255,255,0.08)` | `rgba(43,33,25,0.085)` |
| `--agent-border-muted` `_foundation.scss:20` | `rgba(255,255,255,0.05)` | `rgba(43,33,25,0.05)` |
| `--agent-border-base` `_foundation.scss:21` | `rgba(255,255,255,0.08)` | `rgba(43,33,25,0.09)` |
| `--agent-border-strong` `_foundation.scss:22` | `rgba(255,255,255,0.14)` | `rgba(43,33,25,0.16)` |
| `--agent-chat-ink` `_foundation.scss:23` | `#d5cfca` | `#68635d` |
| `--agent-composer-background` `_foundation.scss:24` | `linear-gradient(118deg, rgba(148,174,151,.045), transparent 34%), linear-gradient(300deg, rgba(196,181,154,.03), transparent 42%), #262220` | `linear-gradient(118deg, rgba(97,122,104,.06), transparent 38%), linear-gradient(300deg, rgba(148,133,113,.04), transparent 45%), #fffaf0` |
| `--agent-composer-border` `_foundation.scss:25` | `rgba(148,174,151,.18)` | `rgba(97,122,104,.24)` |
| `--agent-composer-shadow` `_foundation.scss:26` | `0 4px 16px rgba(0,0,0,.28), 0 0 16px rgba(148,174,151,.025)` | `0 7px 22px rgba(67,48,33,.10), inset 0 1px rgba(255,255,255,.58)` |
| `--agent-composer-focus-border` `_foundation.scss:27` | `rgba(148,174,151,.42)` | `rgba(97,122,104,.50)` |
| `--agent-composer-focus-shadow` `_foundation.scss:28` | `0 4px 16px rgba(0,0,0,.28), 0 0 0 3px rgba(148,174,151,.07), 0 0 18px rgba(148,174,151,.05)` | `0 8px 25px rgba(67,48,33,.11), 0 0 0 3px rgba(97,122,104,.10)` |
| `--agent-send-background` `_foundation.scss:29` | `linear-gradient(160deg, #9eb4a1, #778f7c)` | `linear-gradient(160deg, #708976, #566f5d)` |
| `--agent-send-hover` `_foundation.scss:30` | `linear-gradient(160deg, #b2c4b4, #89a08d)` | `linear-gradient(160deg, #7f9985, #627b69)` |
| `--agent-send-color` `_foundation.scss:31` | `#172019` | `#fffaf0` |
| `--agent-send-shadow` / `hover-shadow` `_foundation.scss:32` | `0 5px 14px rgba(119,143,124,.30)` / `.40` | `0 5px 14px rgba(86,111,93,.24)` / `0 6px 18px rgba(86,111,93,.32)` |

`_opencode-chat.scss:1` re-aliases these into `--v2-background-*` names consumed by the transcript components.

### 1.3 Text

| Token | original | paper |
|---|---|---|
| `--text` / `--fg` | `#e8e3dd` | `#2b2119` |
| `--text-base` | `#e8e3dd` | `#2b2119` — alias of `--text` for composer/queued chips |
| `--text-dim` | `#a8a29e` | `#6b5f50` |
| `--text-faint` / `--text-weak` | `#8f8880` | `#948571` — identical aliases |
| `--agent-chat-ink` | `#d5cfca` | `#68635d` — body copy inside agent panel |

Contrast notes: `text` on `bg-panel` meets AA in both themes. `faint` on `bg-inset` drops to ~3.5:1 — used only for non-essential labels (kickers, timestamps, counts) never for body copy.

### 1.4 Accent & semantics

| Token | original | paper | Meaning |
|---|---|---|---|
| `--accent` | `#9eb4a1` sage-light | `#617a68` sage-deep | Primary action, selection, focus, brand |
| `--accent-hover` | `#b2c4b4` | `#4f6757` | Hover for accent text / active states |
| `--accent-dim` | `rgba(158,180,161,.16)` | `rgba(97,122,104,.15)` | Active pill / chip / switch `on` background |
| `--accent-tint` | `rgba(158,180,161,.09)` | `rgba(97,122,104,.09)` | Hover wash tint, drop overlays |
| `--green` | `#a9cbad` | `#587657` | Success, live dot, input badge, context bar ok |
| `--red` | `#e2988a` | `#aa624f` | Destructive, fail dot, deleted file, error |
| `--yellow` | `#e5c084` | `#9c742f` | Warning, pending, stale, limit 60–80% |
| `--sky` | `#8fbcd9` | `#49708f` | Info, `observed` badge (`_sidebar.scss:557`) |

Semantic ladder for limits/context bars (`_agent.scss:330`): `ok` = green · `warn` = yellow · `danger` = red — consistent in usage popup and todo/context bars.

### 1.5 Borders

| Token | original | paper |
|---|---|---|
| `--border` | `rgba(255,255,255,.05)` | `rgba(43,33,25,.075)` |
| `--border-strong` | `rgba(255,255,255,.10)` | `rgba(43,33,25,.15)` |
| `--border-subtle` | `rgba(255,255,255,.04)` | `rgba(43,33,25,.05)` |
| `--agent-border-*` | see §1.2 |  |

All borders are 1 px hairlines. `--border-subtle` is for dividers inside a surface (tree indent, composer border) that should almost disappear. `--border-strong` is for floating containers (menus, popups, settings cards).

---

## 2. Radius

`_foundation.scss:59`

| Token | Value | Usage |
|---|---|---|
| `--radius-sm` | `9px` | Chips, small buttons, settings nav mark, toast (`_toasts.scss:16`), composer pills?  |
| `--radius-md` | `12px` | Settings cards (`_settings.scss:88`), composer menu (`_composer.scss:424`), welcome tabs (`_welcome.scss:242`) |
| `--radius-lg` | `16px` | Welcome frame (`_welcome.scss:228`), sidebar settings (`_sidebar.scss:127`), composer body (`_composer.scss:26`), settings page sections |
| `--radius-xl` | `20px` | Agent panel (`_agent.scss:8`), terminal tray (`_terminal.scss:38`) |
| `--radius-full` | `999px` | Tree row (`_sidebar.scss:444`), tabbar (`_editor.scss:36`), session row (`_sessions.scss:277`), composer send button, statusbar btn, pills, tab, search fields — the signature pill shape of the system |

Nothing in the product uses `0` or sharp corners except divider lines and the main window chrome.

---

## 3. Shadows

`_foundation.scss:65`

| Token | original | paper |
|---|---|---|
| `--shadow-sm` | `0 1px 2px rgba(0,0,0,.18), inset 0 1px 0 rgba(255,255,255,.03)` | `0 1px 2px rgba(67,48,33,.08), inset 0 1px 0 rgba(255,255,255,.45)` |
| `--shadow-md` | `0 2px 8px rgba(0,0,0,.10), 0 10px 30px rgba(0,0,0,.18)` | `0 2px 8px rgba(67,48,33,.08), 0 16px 38px rgba(67,48,33,.10)` |
| `--shadow-lg` | `0 2px 6px rgba(0,0,0,.07), 0 10px 24px rgba(0,0,0,.09)` | `0 2px 6px rgba(67,48,33,.06), 0 12px 28px rgba(67,48,33,.08)` |

Additionally, the composer reuses `--agent-composer-shadow` (see §1.2) which adds an accent-tinted glow — the only place an accent color enters a shadow. Everything else is neutral warm-black. In dark mode the shadows are brown-black (`#201b17`); in paper mode they are bark-brown (`rgba(67,48,33,…)`) — never pure black/white.

Where used: `shadow-sm` — segmented controls, settings close. `shadow-md` — menus, popups, context menus, terminal tray. `shadow-lg` — agent panel float (`_agent.scss:9`), welcome frame (`_welcome.scss:231`).

---

## 4. Motion

| Token | Value | Notes |
|---|---|---|
| `--ease` `_foundation.scss:69` | `cubic-bezier(0.2, 0, 0, 1)` — decelerate, ease-out | Default for every `transition` in the codebase |
| `100ms` | Hover foreground/border/background (`_sidebar.scss:451`, `_buttons.scss:10`) | Instant-but-soft |
| `120ms` | Color/bg on buttons, toggles, sections (`_sidebar.scss:170`, `_settings.scss:136`) | Slightly softer |
| `140–180ms` | Panel width, composer focus, activity dock height (`_layout.scss:38`, `_agent.scss:64`) | The only longer durations — reserved for structural motion |
| `200ms` | Tray height (`_terminal.scss:7`), `.agent-col.settling` (`_layout.scss:38`) | Drawer-scale motion, `cubic-bezier(0.16,1,0.3,1)` variant for tray |

No spring, no bounce, no scale-up. Reduced-motion (`@media (prefers-reduced-motion: reduce)`) is honored in the design study (`design/session-panels.css:1202`) — production styles inherit the same `--ease` via `var()` and collapse under the same media query if extended.

---

## 5. Typography tokens (summary)

Full scale in `02-typography.md`. Foundations tracked here:

- `--font-size-base` (`_opencode-chat.scss:20`) = `14px` — chat body
- `--font-size-small` = `12px` — chat metadata
- `--font-weight-regular` = `410`, `--font-weight-medium` = `490`
- `--line-height-large` = `21px`
- Global `body` `_foundation.scss:142` = `13px` system UI (`-apple-system`…)

---

## 6. How to add a token

1. Add the property in **both** `:root` and `:root[data-theme="paper"]` blocks in `_foundation.scss`.
2. Reference it with `var(--token)` — never introduce hex at the call site.
3. Document it here and in the component doc that consumes it.
4. If it is an agent/transcript colour, add the `--v2-*` alias in `_opencode-chat.scss:1` as well.

---

## 7. Contrast audit (spot checks)

| Pair | Paper ratio | Original ratio |
|---|---|---|
| `--text` on `--bg-panel` | ~14.2:1 | ~11.6:1 |
| `--text-dim` on `--bg-inset` | ~5.8:1 | ~4.9:1 |
| `--accent` text on `--bg-panel` | ~4.9:1 (paper) — accent text is limited to 12.5–14 px semibold and large labels | ~5.4:1 (original) |
| `--text-faint` on `--bg-panel` | ~3.5:1 — never body copy, only kicker/caption | ~3.3:1 |

> Values calculated at import — for an authoritative WCAG pass run a contrast checker against the computed `rgb()` of each variable. The system's calm comes at a mild contrast cost on `--text-faint` labels; these are intentionally secondary information.

---

## 8. Raw dump (for diffing)

```css
/* _foundation.scss:1 + :72 — exact declarations */
:root { --bg:#171412; --bg-panel:#262220; ... }
:root[data-theme="paper"] { --bg:#f4eee1; --bg-panel:#fbf7ec; ... }
```

Keep this document in sync with `_foundation.scss` — a line mismatch means either the code or the doc has drifted.
