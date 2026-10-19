# 08 — Icons & Brand

> **Sources:** `src/renderer/src/components/VelaMark.tsx:3`, `resources/icon.svg:1`, `src/renderer/src/components/icons.tsx:1` (custom), `src/renderer/src/components/FileIcons.tsx:1`, `@vscode/codicons` `package.json:28`, `_foundation.scss:163`, `redesign/logo.html:1`, `_welcome.scss:51`, `_settings.scss:26`

Iconography is intentionally sparse — a thin monoline set on a soft surface. Almost every stroke glyph is a 1–1.5 px outline; filled glyphs appear only for badge state (live dot, dirty dot, provider monogram).

---

## 1. The mark — `VelaMark`

### 1.1 React component `VelaMark` `src/renderer/src/components/VelaMark.tsx:3`

```tsx
function VelaMark({ size = 46 }) {
  return (
    <svg viewBox="0 0 96 96" className="vela-mark" …>
      <path d="M48 10 … 48 10Z" fill="color-mix(in srgb, var(--accent) 72%, var(--text))" />
      <path d="M70 12 … 70 12Z" fill="var(--accent)" opacity=".75" />
    </svg>
  );
}
```

The mark pairs a four-point star with a smaller sparkle. Its two fills use the
active accent and text colors, so it adapts to the selected appearance.

### 1.2 Where it appears

| Host | Wrapper / overrides | Size | Colours |
|---|---|---|---|
| Titlebar `.titlebar-title .vela-mark` `_layout.scss:75` | bare, beside the `Vela` word | `16` | fixed sage hexes |
| Welcome frame `.welcome-mark` `_welcome.scss:51` | `58×58 radius16 28% accent border`; `.vela-mark` adds `drop-shadow 14 accent35%` `_welcome.scss:68` | `46` (default) | fixed sage hexes |
| Settings page `.settings-page-brand` `_settings.scss:26` | `54×54 radius-lg bg-panel border-strong shadow-sm` | `34` | fixed sage hexes |
| Settings about `.settings-about` `_settings.scss:217` | bare, beside the version text | `72` | fixed sage hexes |
| Editor empty state `.editor-empty-icon` `_editor.scss:213` | bare | `40` | fixed sage hexes |

A rule: **never recolour the mark by reaching into its `<path>` elements** — override `--omni-prompt` / `--omni-ground` on the ancestor and `color` for the outer frame. That keeps the two-stroke behaviour intact.

### 1.3 Production app icon `resources/icon.svg:1`

```svg
<svg viewBox="0 0 1024 1024">
  <rect x="88" y="88" width="848" height="848" rx="194" fill="url(#midnight)" />
  <rect … fill="url(#atmosphere)" stroke="#8da3c0" strokeOpacity=".18" />
  <path d="…" fill="url(#vela-star)" filter="url(#mark-shadow)" />
  <path d="…" fill="url(#star-core)" />
</svg>
```

The OS icon carries a luminous four-point star and smaller sparkle on a deep
blue gradient, within the rounded 848 px canvas. `resources/icon.svg` is the
source of truth; `npm run generate-icons` renders the PNG sizes and rebuilds
`icon.icns` with `iconutil`.

> The in-app mark and OS icon are separate compositions. Update `VelaMark.tsx`,
> `resources/logo/vela-mark.svg`, and `resources/logo/vela-lockup.svg` together
> when the product mark changes; update `resources/icon.svg` and regenerate
> the derived OS icon files independently.

### 1.4 Assets outside `src`

Beyond `resources/icon.*`, the repo contains a pre-production brand kitchen in `redesign/logo.html:1` (100 tile marks, dark bg `#171310`, burnt-clay gutter `#e8875f`) plus a name study `redesign/rebrand.md` — informative for mood but not the shipped system (see `10-audit.md`).

---

## 2. Custom monoline set `src/renderer/src/components/icons.tsx:1`

A hand-drawn set — **one `Icon` wrapper, one visual spec**:

```tsx
function Icon({ name, children, …rest }) {           /* icons.tsx:5 */
  return (
    <svg className={`os-icon codicon codicon-${name}`}
         viewBox="0 0 16 16" fill="none"
         stroke="currentColor" strokeWidth={1.5}
         strokeLinecap="round" strokeLinejoin="round" aria-hidden …>
      {children}
    </svg>
  );
}
```

Every exported icon is `16×16 @ 1.5` thin stroke, round caps — so custom icons and codicons are indistinguishable once painted, even though codicons carry their own `@vscode/codicons` font. The `codicon codicon-*` class ensures the custom icon and the codicon occupy the same icon taxonomy for `icons.tsx` consumer code — they differ only by how they paint (path vs font glyph) but share the `1em × 1em` box.

Representative members (the set grows organically — grep `export function Icon` to enumerate, currently ~25+):

| Name | Path `icons.tsx:line` | Meaning |
|---|---|---|
| `IconAdd` | `25` | `M8 3.2v9.6 M3.2 8h9.6` · plus |
| `IconArrowDown/Left/Right/Up` | `33/41/49/57` | `H/V 9.6 shaft + 3.6 chev 45°` |
| `IconCheck` | `65` | `L` tick |
| `IconChevronDown` | `73` | caret `4.5 6.2 → 8 9.8` |

Surface usage: small leading/pairing glyphs (add, chevron, refresh, check, panel affords). Size is typically the line-height `0.85–1 em` where used; tree rows pin at `16×16` exactly (`_sidebar.scss:502`). Custom icons always sit beside type (never alone as a landmark) — the only standalone glyph roles are the `VelaMark` and the agent-status dots.

Global icon sizing rule `_foundation.scss:163`:

```css
svg.os-icon { width:1em; height:1em; display:inline-block; flex-shrink:0; vertical-align:-0.15em; }
```

---

## 3. External icon font — `@vscode/codicons`

`package.json:28` `@vscode/codicons@0.0.46-24` is already bundled. They render as glyph spans `<span class="codicon codicon-…">` — used inside the agent panel (header actions `17×17` — `_agent.scss:230`), composer (`11–13` — `_composer.scss:324`), todo (`11` — `_opencode-chat.scss:1237`), and diff toolbar. Because the custom set shares `codicon` class names for `1em` sizing, a codicon and a custom icon drop into the same flex layout without offset.

> New glyphs should prefer a custom `Icon*` in `icons.tsx` rather than reaching for an arbitrary codicon — the 1.5 stroke custom set is calmer than the occasional 2.0 outline in codicons. Reuse an existing codicon only when the symbol is strongly idiomatic to VS Code (e.g., `codicon-check` for task done).

---

## 4. File icons — `src/renderer/src/components/FileIcons.tsx:1`

Maps filename / extension → `codicon` or custom icon. The tree recolors the returned icon per state:

- `file .si-icon` (`_sidebar.scss:516`) is `--text-faint` by default; on `.active` it becomes `--accent` (`_sidebar.scss:488`).
- `dir .si-icon` (`_sidebar.scss:505`) is `--text-dim`; on `.open` it becomes `--accent`.
- Size `16 × 16` `flex-shrink:0` (`_sidebar.scss:501`), never larger.

The file-icon callsite never adds its own fill — recolour is always assigned via `color` inheritance at the `.tree-row` parent.

---

## 5. Icon motion & embellishment

| Glyph | Motion | Spec |
|---|---|---|
| `agent-status-dot.working` | `agent-status-pulse 1.6 ease infinite 0.55→1 opacity` | `_agent.scss:146` |
| `agent-dot.live / panel-status-dot.live` | `green dot + alpha glow 6 rgba(accent 45%)` — no animation; static state glow | `_opencode-chat.scss:968` / `pulse-ring 1.8 ease infinite` on idle? `_agent.scss:42` → expand rings on `.agent-dot.busy` |
| `spinner` | `spin 0.7 linear` thin ring (`border 2 border-strong + top accent`) `11×11` | `_agent.scss:618` |
| `Icon*` chevrons / toggles | `transform 120(var(--ease)) rotate 90/180` per open | `_sidebar.scss:407` / `_sessions.scss:230` |
| `welcome-mark .vela-mark` | `filter drop-shadow 0 0 14 accent35%` | `_welcome.scss:68` |

Keep dot pulses infrequent — at most one pulsing dot per row (`agent-dot.live` alone) so the sidebar never looks like a loading kaleidoscope.

---

## 6. Rules for new icons

- Draw on a **16×16 box at stroke 1.5**, caps round, joins round — do not vary the stroke.
- Always export via `icons.tsx:5` wrapper / `codicon codicon-*` class — never inline a self-styled SVg with a private sizing.
- Never embed raster or two-colour icons inside the tree — the list's calm depends on one dim stroke. Use `--text-faint/dim/text` palette via CSS `color`; do not hardcode hex fill inside the SVG.
- The seeded `resources/icon.svg` squircle + glow is the **only** multi-colour logo allowed outside the `design/logos` exploration folder.

---

## 7. Quick reference

```
VelaMark             four-point star + smaller sparkle, active appearance-token fills

custom icons          16×16  stroke 1.5  round  currentColor
tree icon             16×16  actual size   os-icon·codicon
agent-header icon     17×17  .agent-usage-toggle .codicon _agent.scss:230
composer icon         11–14  .composer-* .codicon
```

When auditing icon usage run `grep -R 'codicon\|Icon' src/renderer --include='*.tsx' | wc -l` — the system should hover 60–90 references; materially more means a panel is probably icon-noisy.
