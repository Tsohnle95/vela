# Codex UI parity audit

Source of truth: the running Vela 0.1.0 development build on this Mac, not
memory or the design tokens. Each page was captured from the live app window by
CGWindow ID at 1426×884, then compared with the native window at the same size.

Original captures are in `docs/reference/original/`.

## Method

1. Resolve the original window ID with `CGWindowListCopyWindowInfo` (owner
   `Vela`) and capture with `screencapture -x -l <id> -o`.
2. Drive the original UI with AXRaise plus pointer/keyboard actions; hide
   unrelated windows first.
3. Capture the same page in `vela-native` by window ID.
4. Record exact deltas (chrome, spacing, type, controls, empty states) here.
5. Do not mark a page done until a fresh capture matches the reference at the
   same 1426×884 size.

## Page inventory and current state

| Page / surface | Original capture | Native state | Main deltas | Status |
|---|---|---|---|---|
| Codex workspace (Files + editor + agent) | `original/editor.png` | Sidebar 230; code x≈329; line numbers x≈312; composer y≈778; selectors right; inline bold/code runs | multi-tab file titlebar; syntax coverage | **Near pixel** |
| Agent panel header/transcript/composer | `original/transcript.png` | Header removed (floating ellipsis only); borderless panel; flat activity rows; user bubble, headings, bold/inline code, code-block toolbar matched | worked-for timing row; list markers | **Near pixel** |
| Model picker | `original/picker.png` | Full popover at x=1122/top 377/bottom 717/285px with search, response strength, favorites, provider groups | row height 28 vs ~30; agent picker still compact | **Near pixel** |
| Sessions / landing | `original/landing.png` | Sidebar content at x=79; New Session icon/text at x=82/110; inline Projects/Recents chevrons; top-right robot + bordered add + panel buttons; agent left border | row height 27 vs 28 | **Near pixel** |
| Changes / diff | `original/diff.png` | Two-pane center Changes: 223px file list + full-file inline diff, line numbers 1..N, added-line tint, `MODIFIED +N -M` badge, header actions | change-count data differences; diff syntax coverage | **Near pixel** |
| Embedded browser | `original/browser.png` | Multi-tab titlebar, globe New tab tabs, blank empty state, URL entry, plus, close, webview bounds | tab widths/labels, address styling, tab overflow | **Near pixel** |
| Terminal 1 | `original/terminal.png` | Implemented light, tabs, PTY | tab spacing, close glyph, tray shadow | **Implemented** |
| Problems | `original/terminal.png` tab | Implemented empty state | copy/styling not captured from original | **Partial** |
| Sass Output | `original/sass-output.png` | Real grass compilation, CSS output, live log lines, Watch Sass menu; tray bottom at y=870 matching reference | output-path configuration, partials/import edge cases | **Implemented** |
| Appearance | `original/settings-appearance.png` | Title x=473/y≈85; 181×114 cards at x=473/y≈259; settings nav square at window bottom; no white gap | Interface block text sits ~25px high | **Near pixel** |
| Providers | `original/settings-providers.png` | 2-column cards; title x=474, cards x=432, nav content boundary x=281 matching | card width 425 vs ≈430; row copy offsets | **Near pixel** |
| Voice | `original/settings-voice.png` | Implemented unavailable card at matching header rhythm | card/heading y within ~20px | **Near pixel** |
| Model | `original/settings-model.png` | Default/Safety rows plus one grouped OpenCode card with hairlines | row heights/vertical gaps; sync actions no-op | **Near pixel** |
| Servers | `original/settings-servers.png` | Implemented empty state + hint | copy/vertical alignment | **Near pixel** |
| Mobile Setup | `original/settings-mobile.png` | Two-column page; cards start x≈432/y≈221 matching; pairing card widened to 390 | QR art, exact left/right widths, copy | **Near pixel** |
| About | `original/settings-about.png` | Implemented version + updates | spacing, icon art | **Near pixel** |

Confirmed: the Codex agent panel has **no border**; the sidebar/editor seam is
borderless. Only tonal surface differences remain.

## Detailed deltas

### Terminal
- Vela terminal background is white/light (`#fbfbfb`-ish), text dark.
- Tabs: `TERMINAL 1 ×`, `PROBLEMS`, `SASS OUTPUT`, `+`, globe, chevron collapse.
- Header is a thin light strip; body fills the tray.
- Native currently uses a dark body and static tab labels.

### Sass Output
- Header directory line: workspace path in monospace.
- Empty state: `Enable Watch Sass in the footer to compile saved Sass files to CSS.`
- Watch Sass lives in the editor breadcrumb `⋯` status menu, not a footer.
- Native has neither the tab content nor the status menu.

### Settings shell
- Nav is a floating 229px card with `PREFERENCES` kicker, `Settings` title,
  36px items, rounded 10 active pills.
- Content is a rounded surface to the right; non-Appearance pages use a subtle
  blue-tinted surface and a shared header (star orb, blue `VELA PREFERENCES`
  kicker, 30px title, description, `Back to workspace` button).
- Appearance is the exception: plain surface, simple `Appearance` title + close,
  interface segmented control, experimental-art switch, large theme cards.
- Native nav is 202px, static, and all non-Appearance pages are absent.

### Settings pages
- Providers: 2-column cards (`OpenCode Go`, `Command Code`, `OpenAI`), badges
  `AVAILABLE`/`CONNECTED`, `Add key`, connected row, plan text.
- Voice: one `Voice input` card with `UNAVAILABLE`.
- Model: `Default model` select; Safety `Ask`/`Approve`; OpenCode CLI block,
  installed version, Sync, Update.
- Servers: `No servers running` card and a start-preview card.
- Mobile Setup: two-column hero/instructions + pairing card with QR placeholder
  and `Generate pairing QR`.
- About: Vela version card + `APP UPDATES` / development build row.

## Execution plan

1. **Sass + terminal** — done for the surface: light terminal palette; real tab
   state; Sass directory/copy; breadcrumb `⋯` menu with Watch Sass toggle.
   Remaining: actual Sass compiler watch/log stream, tab spacing/close glyph.
2. **Settings shell** — done: floating 229px nav, gradient content surface,
   shared header, working section navigation.
3. **Settings pages** — all seven pages exist with the reference structure.
   Remaining pixel work: body alignment, card grouping, QR art, type scale.
4. **Appearance parity** — runtime block removed; cards enlarged. Remaining:
   detailed mini-layout art and exact card/nav metrics.
5. **Workspace polish** — tab strip, diff gutters/colors, browser/profile
   chrome, agent tool rows, terminal tray/header details.
6. **Pixel pass** — recapture every page at 1426×884, overlay against references,
   correct remaining spacing/type/color deltas. Not complete.

## Round progress

- Captured every Settings page from the live original and native window by
  CGWindow ID.
- Implemented all seven Settings pages and working nav selection.
- Implemented light terminal, tab switching, Problems placeholder, Sass Output
  directory/copy, and the breadcrumb Watch Sass menu.
- Confirmed the Codex agent panel and sidebar/editor seam are borderless.
- Aligned settings body under section titles (72px indent), set the shared
  page rhythm to 42px top padding + 42px gap, and matched Providers/Mobile
  header/card positions to within ~10px.
- Rebuilt Appearance theme cards from the reference CSS: whole-card border,
  8px padding, 68px preview, 12px gap, 24px label row, selected card fill and
  check.
- Aligned the settings shell to measured pixels: no outer gutter, rail at x=0,
  nav borders at x=70/280, content at x=281. Appearance title/cards at x≈472;
  Providers title x=474 and cards x=432. Remaining: rail right-edge border,
  small provider/card typography, QR artwork, and no-op backend actions.
- Captured Sessions from the live original. Measured the workspace sidebar at
  230px and the agent panel at ~324px; updated the native sidebar to 230px, so
  the editor starts at x≈280 and the agent panel at x≈1101, matching Vela.
- Found and fixed an architectural mismatch: Vela’s Changes view is a center
  workspace panel with a `Changes 0` header and empty state. Native now toggles
  a center panel, keeps the file sidebar, and uses 40px inner padding.
- Measured the workspace sidebar at 230px and corrected it; editor/agent
  boundaries now match at x≈280/1101.
- Fixed the embedded browser’s 30px inset and changed its titlebar tab to
  Vela’s globe `New tab`. Added real browser tabs with blank empty state, URL
  entry, close, plus, tab titles, and tab switching. Remaining: exact tab
  widths/labels and overflow behavior.
- Addressed the annotated deltas: settings nav now reaches the window bottom with
  square bottom corners, no white gap; top-right robot/add/panel controls;
  larger New Session icon; inline Projects/Recents chevrons; agent left border.
- Matched Appearance x/y: content starts x=473, cards at y≈259; header title
  at y≈85.
- Reworked the settings shell to Vela’s real CSS metrics: 225px nav at x=55,
  77px page padding, 54px brand, 36px header rule, flat gray surface, 448px
  two-column provider cards.
- Widened the Codex composer to 304px and moved the connector beside `+`.
- Removed the 48px agent header entirely; the panel now starts at the titlebar
  with only a floating ellipsis, matching Vela, and is visible on landing.
- Flattened tool calls into borderless activity rows (`read · done`).
- Rebuilt the diff as Vela’s two-pane Changes surface with a file list and
  full-file inline diff (line numbers 1..N, added-line tint, stat badge).
- Matched the empty agent copy to `Tell the agent what to work on.`
- Implemented the native Sass watcher with `grass`: watches saved `.scss/.sass`
  files, compiles to sibling CSS, and streams log lines into Sass Output.
- Added the Codex content bottom inset (14px), placing the terminal tray at
  y=746–870 and the agent panel bottom at 870, matching Vela.
- Matched Sessions chrome and vertical rhythm: square pencil icon, search glyph,
  Projects/Recents chevrons, and rows aligned within ~5px.
- Rebuilt Appearance previews to the reference pane structure: rail, files,
  editor bars, terminal strip, agent pane inside a 68px frame.
- Added inline Markdown runs: bold spans and inline-code pills render in both the
  editor and transcript; headings retain syntax color. Transcript code blocks
  now carry Vela’s `No wrap` / `Copy` toolbar.
- Rebuilt the model picker as Vela’s full popover (search, response strength,
  favorites, provider groups, stars) and matched its geometry: x=1122, top 377,
  bottom 717, width 285.
- Captured the editor view. Breadcrumb, line numbers, and panel boundaries are
  close; remaining gap is multi-tab titlebar, inline Markdown styling, and the
  composer focus stroke (removed).
- Known blockers to pixel identity: Appearance theme art, provider card width,
  Model row grouping, Mobile right-card width/QR art, Sass compiler (surface
  only), status-menu contents, terminal/agent micro-typography.

## Evidence index

- `original/settings-appearance.png`
- `original/settings-providers.png`
- `original/settings-voice.png`
- `original/settings-model.png`
- `original/settings-servers.png`
- `original/settings-mobile.png`
- `original/settings-about.png`
- `original/terminal.png`
- `original/sass-output.png`
- Native captures: `docs/reference/native/*.png`
- Published screenshots: `docs/assets/native-*.png`
