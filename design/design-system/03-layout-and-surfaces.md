# 03 — Layout & Surfaces

> **Sources:** `_layout.scss:1`, `_sidebar.scss:1`, `_editor.scss:1`, `_agent.scss:1`, `_terminal.scss:1`, `_statusbar.scss:1`, `_welcome.scss:1`, `_foundation.scss:1`

---

## 1. Global shell

```
 ┌─────────────────────────────────────────────┐ fixed z:50  height 34  drag
 │ titlebar  — traffic-lights + VelaMark + actions                                  │  _layout.scss:46
 ├──────────────┬────────────────────┬──────────┤
 │              │                    │ agent-col│  absolute z:200 • floating card
 │  sidebar     │   editor-pane      │ floating │  backdrop-blur(18px) • radius-xl
 │  panel-      │   (tabbar +        │ over the │  box-shadow: var(--shadow-lg)
 │  surface-    │    monaco +        │  editor  │  _agent.scss:1
 │  color       │    toolbar)        │
 │  border-right│                    │
 ├──────────────┴────────────────────┴──────────┤
 │  tray-area  — terminal-tray (collapsible)   │  z:60–70  height: var(--tray-height)  _terminal.scss:1
 ├─────────────────────────────────────────────┤
 │ statusbar  26px  transparent  z:60          │  _statusbar.scss:1
 └─────────────────────────────────────────────┘
```

### 1.1 `.app`

`_layout.scss:1`

```css
.app {
  position: relative; z-index: 1;
  display: flex; flex-direction: column;
  height: 100%; padding-top: 34px;      /* reserves titlebar space */
  background: var(--bg-panel);
}
```

`html, body, #root` are each `height: 100%; overflow: hidden` (`_foundation.scss:132`) — there is no document scroll; every pane manages its own scroll.

### 1.2 Titlebar

`_layout.scss:46`

- Fixed `top: 0; height: 34px; padding: 0 14px; gap: 10px; display: flex; align-items: center; background: var(--bg); border-bottom: none; z-index: 50; -webkit-app-region: drag`.
- `.darwin .titlebar { padding-left: 78px; }` — reserves macOS traffic lights.
- Title: `.titlebar-title` `12.5 / 650 / 0.14em uppercase dim` (`_layout.scss:64`) with nested `.vela-mark { flex-shrink: 0 }` (`_layout.scss:75`) — the mark carries its own fixed sage hexes.
- Actions on the right are ` -webkit-app-region: no-drag` (`_layout.scss:81`). Icon-button `.on` uses `background: var(--accent-dim); color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent)` (`_layout.scss:91`).

### 1.3 Main row

`_layout.scss:11`

```css
.main-row {
  display: grid;
  grid-template-columns: var(--pane-columns);   /* set by pane-layout JS hook */
  flex: 1; min-height: 0;
}
.workspace-area { display: flex; background: transparent; min-width: 0; min-height: 0; overflow: hidden; } /* _layout.scss:19 */
```

`--pane-columns` is driven by JS pane-layout (not a static token) — it models the sidebar / editor / agent split. Dividers and resizers are overlaid (see §4).

---

## 2. Sidebar vs Editor vs Agent — surface model

| Panel | Background | Border | Radius | Key vars | File |
|---|---|---|---|---|---|
| **Sidebar** `.sidebar` | `var(--panel-surface-color)` (paper `#fbf7ec`, dark `#262220`) + `image: var(--panel-surface-image)` | `border-right: 1px solid var(--border)` | none (flush to left edge); internal sections radius via `--radius-*` | `--panel-aura-x: 0%` | `_sidebar.scss:1` |
| **Editor** `.editor-pane` | `var(--panel-surface-color)` (`_editor.scss:3`) | none (tabbar carries inset) | none; tabbar is pill (`--radius-full`) floating inside `12px 14px` inset | `--editor-right` for agent-col overlay | `_editor.scss:1` |
| **Agent** `.agent-panel` | `var(--panel-float-color)` with `backdrop-filter: blur(18px)` | `1px solid var(--border)` + `--shadow-lg` | `--radius-xl` (`20px`) | `--panel-aura-x: 100%`, `--panel-surface-color: none` (image disabled) | `_agent.scss:1` |
| **Welcome** `.welcome` | `var(--bg)` with 2 radial gradients (sage + sky) | none | `.welcome-frame` has `--radius-lg 22px` | `--canvas / --surface / --hairline` aliases of `--bg`/`--bg-panel`/`--border` | `_welcome.scss:20` |
| **Settings** `.settings-page` | `var(--bg-inset)` with 1 radial accent-tint gradient | per-card borders | `--radius-lg 22` for header frame? `16` for sections | section width `min(920px, 100%)` | `_settings.scss:1` |
| **Terminal tray** `.terminal-tray` | `var(--bg-panel)` | `border-radius: var(--radius-xl) var(--radius-xl) 0 0` + `box-shadow: 0 -4px 12px …, 0 -16px 40px …` | `20 20 0 0` | height `var(--terminal-height)`; outer `tray-area` toggles via `height: var(--tray-height)` | `_terminal.scss:33` |

**Key principle:** sidebar and editor are matte and flush; only the agent panel *floats* with glass/blur + shadow. This makes the AI chat feel like an overlay rather than equal chrome — hierarchy is literal depth.

The `panel-surface-image` / `--panel-aura-x` plumbing (`_foundation.scss:12` + `_sidebar.scss:2` + `_agent.scss:3`) is a hook for an optional warm aura — currently neutral ("none") but reserved so a texture can be added without restructuring markup.

---

## 3. Sidebar anatomy

`_sidebar.scss:1` (664 lines — the largest non-chat partial)

```
.sidebar
 ├─ .sidebar-header          12 12 8 16 · flex gap 8
 │   ├─ .sidebar-title      12.5/600/-  + dot 7px sage / glow       _sidebar.scss:14
 │   └─ .sidebar-header-actions
 ├─ .side-tabs               border-bottom 1 hairline; first-child inset _sidebar.scss:199
 │   └─ .side-tab            10.5/700/0.14em uppercase · active ::after 2px accent bar _sidebar.scss:242
 ├─ .sidebar-section.changes height: var(--changes-height) _sidebar.scss:293
 │   ├─ .section-trigger     with-actions? hover bg _sidebar.scss:319
 │   │   ├─ .section-toggle  10/700/0.12em uppercase · gap 6         _sidebar.scss:374
 │   │   └─ .section-actions visible on hover/focus-within            _sidebar.scss:336
 │   └─ .changes-list        overflow-y auto · padding 2 6 8           _sidebar.scss:409
 ├─ .sidebar-vdivider        height 5 row-resize + border-top hairline  _sidebar.scss:415
 └─ .sidebar-section.explorer  flex:1 min-height:0
     ├─ .section-trigger     (same)
     └─ .tree                2 8 14 · position relative                _sidebar.scss:423
         └─ .tree-row (+modifiers: dir/file/active/open/deleted/workspace-root/drop-target) _sidebar.scss:437
              gap 7 · height 28 · radius-full · margin 1 2 · padding 4 10
              .tree-children margin-left 14 + border-left 1           _sidebar.scss:528
```

Sub-surfaces:

- **Settings sidebar** `.settings-sidebar-header` (`20 18 16 padding`, border-bottom) + `.settings-nav` (`12 10` pad) + `.settings-nav-item` (`38 min-height`, `6 9 pad`, `12/600`) — active uses `accent-dim` (`_sidebar.scss:100`).
- **Floating settings popover** `.sidebar-settings` (`abs bottom calc(100%+8)`, `10 pad`, `border-strong`, `shadow-md`, `radius-lg`) with segmented switch/slider controls (`_sidebar.scss:148`).

---

## 4. Sessions pane (navigational filing cabinet, not a chat list)

`_sessions.scss:1`

- Container `.sessions-pane`: `flex column 6 8 14` + `overflow-y: auto` + `--session-list-row-height: 28px` (`_sessions.scss:2`).
- Action row `.sessions-actions` (`2 4 8 pad`, `gap 3`) with `sessions-new` (accent rounded), `sessions-file` (square icon), `sessions-plugins` (+ popover `.sessions-plugins-menu` `min 220 × max 300`, border-strong + shadow-lg) (`_sessions.scss:22`).
- Section folding `.sessions-section` (`gap 0`, `margin-top 3`) — trigger reuses `.section-toggle` pattern.
- **Project grouping**: `.sessions-project-head` (`height var(--session-list-row-height)`, `margin 0 2`, `radius-full`, `hover: bg-hover`) plus chevron rotation (`rotate 90` for `.open`) and inline new-button that fades in on hover (`opacity 0→1` — `_sessions.scss:255`).
- **Session rows** `.sessions-row` (`height 28`, `gap 7`, `pad 4 8`, `margin 0 2`, `radius-full`, `hover: bg-hover`) with states `.focused` (text + dot glow), `.agent-dot`, `.sessions-row-pin.pinned` (accent).

---

## 5. Editor pane

`_editor.scss:1`

- **Tab bar** `.tabbar`: `12 14 0 pad` · `4 gap` · `bg-inset` + `border-subtle` · `radius-full` · `scrollbar-width: none`; tabs overflow-x.
  - Tab `.tab`: `5 13 pad` · `12.5 dim` · `radius-full` · `max 220`; `hover` bg-hover; `.active` uses `bg-active-pill` + `0 2px 8px` shadow (`_editor.scss:59`).
  - Dirty dot `.tab-dirty` (`7×7 accent #e8875f? Actually `var(--accent)` + glow) (`_editor.scss:72`).
- **Monaco host** `.editor-wrap { flex:1 flex-col min-height:0 }` (`_editor.scss:101`); diff overview is `scaleX(0.5)` (`_editor.scss:103`).
- **Toolbar** `.editor-toolbar`: `6 16 pad` · `border-bottom 1px --border` · `12 dim`; left group has path `tabular-nums` + state hints (`dirty yellow`, `stale yellow`, `deleted red`); right group has pill `.toolbar-btn` (`11.5/500`, `3 12 pad`, `radius-full`, `.on = accent-dim/accent/accent-border`).
- **State banners** (conditional render): `.deleted-banner` (red 0.1), `.conflict-banner` (yellow mix, `danger .red`) — both 1 px bottom border (`_editor.scss:157`).
- **Empty state** `.editor-empty` (`flex centered`), with icon at `42px faint + drop-shadow accent 18px`; container-queries collapse sub/details at `460px`/`280px` (`_editor.scss:222`).

---

## 6. Agent panel (floating)

`_agent.scss:1`

- Shell `.agent-panel`: `position relative; backdrop-filter: blur(18px); border 1/--border; radius-xl; shadow-lg; container-type inline-size` — the blur + translucency makes it read as a sheet above the editor.
- Header `.agent-header`: `12 16 10 pad`, `border-bottom 1` (`_agent.scss:17`); dots, identity lines, status, and turn timer rearrange under container queries (`@container 420/340` drop timer, then collapse status text — `_agent.scss:191`).
- Inner regions: `.agent-scroll` (see 05-chat); `.agent-input-wrap` (`10 12 pad`, `gap 8`, `border-top 1`); `.composer` anchored bottom; floating popups `.agent-usage-popup`, `.composer-menu`, and `.composer-completions` (each absolute `z-index: 60`, `border-strong` + `shadow-md` + `radius-lg`).
- Width & positioning are driven by `.agent-col` in `_layout.scss:28` (`position: absolute; top:0; bottom:0; z-index: 200; > .agent-panel flex:1`). The `.settling` transition runs `width/left 180ms var(--ease)` — the only place the grid animates.

---

## 7. Terminal tray & statusbar

### Tray `_terminal.scss:1`

- Outer `.tray-area { height: 0; overflow: hidden; transition height 200ms (0.16,1,0.3,1) }` — opening sets `.open { height: var(--tray-height) }`. The `.dragging` class suppresses transition.
- Divider `.tray-divider { height: 5; cursor: row-resize }` — the only draggable handle.
- Frame `.terminal-tray { height: var(--terminal-height); bg-panel; border-radius-xl t-only; shadow 2-layers }` (`_terminal.scss:33`).
- Header `.terminal-header { 8 12 4 pad }`; tabs `.terminal-tab` (`4 12 pad`, `10.5/600`) plus add/close pills (`22×22`, `radius-full`); body `flex:1` with hidden variants, `padding 6 8` on `.terminal-host` (`_terminal.scss:163`).

### Statusbar `_statusbar.scss:1`

- `flex space-between gap:10; height 26; 0 10 pad; background transparent; border-top none; font 11.5 dim; z:60`.
- Button `.statusbar-btn { 2 10 pad radius-full gap6 }` (`_statusbar.scss:25`) mirrors the tree-row control metaphor.
- Validation glyphs: `.validate-spinner` (`10×10 ring accent`) + `.validate-result.{clean,has-errors,has-warnings,failed}` colours (green/red/yellow/red).

---

## 8. Welcome & settings surfaces

- **Welcome** `.welcome` fills the whole app (`height:100%, padding 48`) with radial sage+sky on `var(--bg)` (`_welcome.scss:20`); centred `max-width 1120 grid 5fr/6fr gap 88` collapsing to single column below `900px` (`_welcome.scss:392`). Hero mark is `58×58 radius 16 bordered + accent blend + drop-shadow` (`_welcome.scss:51`).
- **Settings** `.settings-page`: `clamp(34–76)` padding on `var(--bg-inset)` + sage-tint radial (`_settings.scss:1`), constrained `920px` centred sections. Responsive breakpoint `760px` stacks columns (`_settings.scss:224`).

---

## 9. Spacing & invariants

- **Hairlines** are universally `1px solid var(--border/strong/subtle)` — never `0.5` outside chat markdown `pre` (which intentionally uses `0.5` for delicate code frames).
- **Pills dominate** — most interactive controls are `radius-full` (tree row, session row, tab, tabbar, search, input, toolbar btn, statusbar btn, pills). A few large containers break to `12/16/20`.
- **Containers** — `.agent-panel` and `.editor-pane` declare `container-type: inline-size` so responsive collapsing uses `@container` not `vw`; the rest uses classic `@media`.
- **Scroll regions** are all `overflow: auto; overscroll-behavior: contain/y` + `scrollbar-gutter: stable` where possible to prevent layout jitter when scrollbars appear.
