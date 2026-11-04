# Migration completion

Agent continuation entry point: [START-HERE.md](START-HERE.md). It records the current resume point and supersedes stale historical status counts.

The acceptance target is the current `/Users/tdawg/code-repositories/vela`
application, using Rust/GPUI/Tokio and native platform views here. The original
checkout is read-only. A screenshot that resembles a page does not establish
functional parity, and an implemented backend operation does not establish UI
parity.

## Acceptance and evidence

- Compare the same workspace, data, shell, theme, viewport and state in both
  running applications. Save paired captures, differences and an evidence
  manifest. Keep transient values identifiable rather than hiding discrepancies.
- Cover the current empty workspace, sessions, Files, editor, Changes/diff, agent conversation and
  all its docks/popovers, terminal/Problems/Sass, browser, every Settings page,
  both shells, every theme, and narrow/resized layouts.
- Exercise the action behind each visible control. Track unsupported operations
  explicitly; no static control or fabricated status counts as completion.
- Run the Rust workspace tests, formatting and Clippy, plus native app interaction
  checks. Preserve existing data and concurrent work.

## Ordered work

2026-10-30 user priority update: implement and exercise desktop application
functionality first. Defer mobile logic. Perform the full screenshot comparison
parity matrix after the desktop functionality work; use focused UI checks during
implementation when needed to establish that a feature works.

1. Establish repeatable isolated profiles and current live reference captures;
   reconcile the existing parity matrix with current source (it is stale).
2. Repair shared input, selection, clipboard, IME and editing ownership before
   extending individual controls. Implement file tabs and safe dirty-state
   lifecycle, then complete explorer/editor operations and watching.
3. Complete session navigation/actions, panel ownership, conversation rendering,
   composer catalogs/attachments, forms, queue, usage and runtime settings.
4. Complete workspace Git/recovery, terminal lifecycle/tabs/TUI, browser tab
   isolation and previews, Sass, providers and remaining desktop settings. Mobile
   pairing and all other mobile logic remain deferred by user instruction.
5. Complete shell/theme/responsive parity, package the app, and recapture the
   entire acceptance matrix. Resolve all visual and interaction discrepancies.

## Current checkpoint

- Started 2026-10-24 from clean working tree.
- Existing documentation claims “near pixel” while also naming unresolved
  offsets and no-op controls. These are prior observations, not acceptance.
- Native source has browser tab chrome and Sass compilation that the older
  feature matrix still calls planned; both require fresh interaction checks.
- Both builds have been checked against the dedicated OpenCode profile on port 49555.
  Provider checks currently use a loopback-only fixture on 49557, with isolated profiles.
- Shared native input, 15 source palettes, retained browser tabs and the macOS
  ephemeral partition are implemented; live clipboard/history/partition checks
  passed. They are not visually accepted yet.
- Workspace documents retain dirty text and undo across navigation. Saves
  acknowledge exact snapshots, serialize per document, compare the held disk
  source, preserve recovery proposals and reconcile interrupted transactions.
- Source editing uses a virtualized wrapped-line list and platform input. A
  rendered regression reproduces the large-file viewport failure when its fix
  is removed and passes after restoration. The full workspace suite passes
  (latest full run: 270 tests, one ignored performance measurement;
  `docs/parity/2026-10-25/explorer-detach-final-workspace-tests.txt`).
- Explicit save-conflict resolution and the shared rounded shell are implemented.
  A rendered Settings regression protects viewport sizing at 900 and 1280 points.
  The user's 2026-10-26 correction makes the inner Codex Settings join flat and
  flush while the shared outer shell retains its curved corner.
- Model selection restores the session choice/variant and waits for runtime
  success. Timed global feedback and persisted event-time Ask/Approve behavior
  passed fixture checks. The CLI version is probed off the UI thread. Engine-owned Sync/Update now
  scope commands to the connected profile, rebind the SSE connection and retain
  drafts, scroll and model choice. Cancellation, update, failure and already-current
  results passed disposable runtime GUI checks. Full Model page acceptance, real
  CLI restart/terminal handoff and release-path validation remain unfinished.
- Providers uses live integration data. Native masked key entry, connection,
  request ownership and confirmed credential removal passed real GUI checks
  against a loopback fixture; the real catalog matches the source provider list.
  Engine-owned OAuth cancellation, polling and code retry are implemented and
  exercised against the fixture. Typed field validation and native menus are
  implemented; their remaining behavior and visual differences are tracked in
  the current parity checkpoint. Provider usage remains unfinished.
- The current reference uses manual Save/⌘S; earlier autosave inventory was
  stale. Preserve manual, revision-safe persistence. Native observation now
  passes live paired clean/dirty/rename/deletion/recreation checks and retains
  undo across renames. Source/language tooling, complete Changes/recovery
  presentation, remaining OS quit and failure-path checks, all remaining
  operations and complete page/shell/theme/responsive acceptance remain active.
- The user authorized lossless macOS window captures on 2026-10-25. Capture
  scripts record window identity, point/pixel sizes, encoding and binary hashes;
  comparisons include alpha and never resize or mask mismatched images.

- Explorer continuation: the full filesystem inventory and exact-height native
  viewport pass a1,205-file GUI check and rendered resize/growth/removal
  regression. Source glyphs, editor header/font/art and responsive tab overflow
  are implemented in part. Mouse/hidden-item/Escape checks passed; keyboard
  regression and packaged Enter/Space checks pass; mixed file/browser native
  interaction and paired narrow startup layout pass. Paired mixed-browser,
  emoji and true live OS resize validation remain active. No page is visually accepted.

- Model variants now flow from runtime catalog into the composer. No-op Default
  is gone; Auto/High/rejection and session restoration passed real QA checks.
  Full source model-menu behavior, composer geometry, attachments and Retina
  accessibility remain open. Latest fixture results are in
  `docs/parity/2026-10-25/model-variant-fixture-results.json`.


- Root Remove Workspace now saves captured dirty versions, blocks conflicts,
  waits for owned file operations and closes context resources without deleting
  conversation history/bookmarks. Native autosave, conflict refusal, Undo/Redo,
  retained history and fresh-shell reopen passed. Gate274 passed; follow-up
  ownership guards are being rechecked. Physical detach remains distinct.
  Document/tab activation identity and simultaneous panels remain unfinished.
- Terminal ownership includes session and generation, including same-directory
  switches and late directory callbacks. Native context-shell retirement passed.
  Sass close cleanup now respects its session owner; full per-root shared
  compiler/persistence is still partial. No paired visual page accepted.

- Sass now shares workers/logs among same-root context owners, independently
  runs other roots, unregisters on explicit close and stops at the last owner.
  Versioned per-root preferences restore compilation on ordinary app restart.
  Model, real two-root compiler and native close/restart checks pass. Gate280
  passed; Clippy/build/package pass. A delayed-history close regression also
  reproduced and fixed a warm-cache lock deadlock and stale hydration ownership.
  Dart Sass/config/dependency/output/UI and legacy preference parity remain open.

- Sass compiler/config flow now uses the exact pinned Dart Sass/JSON5 versions;
  read-only Source utility comparisons match9 disposable plans/outputs including
  CSS/maps/warnings/errors. Native Rust owns service and safe output writes.
  Legacy preferences import once without changing Source or replacing native
  choices. Scoped status/error recovery and retained100-line logs implemented;
  real rendered narrow/wide scrolling passes. Latest complete gate293 passed,
  rendered follow-up gate running. New packaged UI acceptance waits for Mac unlock;
  timestamps/warning styling/readonly selection and full menu/timing/platform
  checks remain open. No page visually accepted.

- Sass uses coalesced native events instead of idle scans; real config-only
  changes, mapped CSS/maps, deletion/recovery and multi-root lifecycle pass.
  Root output gates keep slow I/O off the global state lock, serialize reopened
  generations and check cancellation before rename. Gate299 and Clippy pass;
  build finishing. User chose code work while locked, so new packaged UI checks
  remain queued. Full log metadata/selection/menu and platform acceptance remain
  open; no paired page accepted and mobile remains deferred.

- File-workspace canonical preparation backend passes real delayed-history access
  checks. Frontend launch/Finder/drop/single-file routing remains unfinished.
- Activation UUIDs now separate editor tabs/buffers/undo from physical directories;
  shared-root observations and physical mutations reach all scopes. Context close
  waits for acknowledged edits and retires only its own clean scope. Slow macOS
  watcher setup no longer gates file jobs; deterministic startup/failure/gap
  regressions pass. Complete gate307, Clippy and build pass; live acceptance queued.
- Sass timestamp/typed-warning presentation is being completed; readonly selection,
  status-menu behavior and full paired visual acceptance remain open.

- Sass logs now retain timestamps and Info/Warning/Error levels, with native Mac
  localized clock display and Source colors. Real @warn and100-row rendered
  regressions pass. Outside-pointer dismissal and engine-acknowledged toggles
  now guard pending/repeated/obsolete requests. Complete gate307 and Clippy/build
  pass; final pre-dispatch guard build finishing. Live menu acceptance, disabled
  AX semantics, readonly log selection, Validate and locale/platform parity remain.

- Changes startup observation now reconciles only tracked files, so untouched
  neighbors are not invented as observed changes. Complete gate318 and final
  Clippy/build pass. Desktop work then moved to Source's fresh-launch behavior:
  no automatic last-conversation reopen, with explicit launch/Finder paths
  queued until the runtime is ready. File launch creates its parent conversation
  only after a successful captured request, exposes a single-file sidebar and
  opens the file after context activation. Outside files arriving while another
  conversation is active still need standalone tab ownership.
- Recovery has an engine-owned records/acknowledge flow, a native unacknowledged
  artifact notice with Open/Acknowledge, refresh after failed saves/renames and
  Source's 24-hour settled / 7-day interrupted retention. Legacy location
  migration and live UI acceptance remain. Editor Find adds regex mode with
  capture replacement, Alt+R, whole-word filtering, invalid-pattern feedback and
  zero-width-safe atomic Replace All; preserve-case and selection/workspace
  search remain. Latest complete gate324, Clippy and app build pass
  (`file-launch-recovery-regex-*`). No GUI input/capture, package install, Source
  write or mobile work; no page accepted.

- Follow-up: a file outside the active workspace now opens as a standalone
  writable tab attached to that conversation. Absolute reads/writes use a
  private sibling temporary and never create workspace recovery in the external
  parent; missing files can be recreated and expected-content conflicts are
  reported. Editor-surface external drops reuse the same path routing.
  File launch/Finder routing, single-file sidebar and standalone tabs are no
  longer blocking desktop functionality. Live physical OS drag acceptance,
  simultaneous multi-path ordering and external watch/close acceptance remain.
  Latest complete gate325 with Clippy/build pass (`standalone-drop-final2-*`).

- Session revert now stages the selected user-message boundary with file
  restoration, commits, forces an authoritative history reload, and restores an
  edited prompt into the composer. User rows expose Copy/Undo/Edit with retained
  focus identities. A failed stage/commit clears the temporary staged boundary
  and reports the error; the Source staged-revert dock and diff preview remain.
  Engine integration covers stage body/files, successful commit/reload and a
  forced commit failure that clears the staged boundary; latest complete gate326
  with Clippy/build pass (`revert-usage-final2-*`).
- Session usage/context meter now uses the existing runtime usage and selected
  model limit: the agent panel exposes a context percentage/control and a popup
  with cost, context bar/counts, token rows and total. Dismissal and accessibility
  wiring are present; provider usage windows/credits remain deferred.
- Pending forms now have a native dock. Session forms are polled off the UI
  thread, normalized field-by-field, and rendered with text/option, number,
  integer, boolean, multiselect and external controls. Required answers gate
  Submit; Dismiss cancels; refreshes retain typed answers. Engine integration
  covers list parsing, reply payloads and cancellation. Session and location-global
  forms are merged; global replies carry the encoded workspace directory and the
  form's true session ID. Conditional/hidden fields, custom options and complete
  constraint validation remain open. Latest complete gate329 with Clippy/build
  pass (`forms-global-*`).
- Composer slash input now executes compact/compress built-ins, runtime commands
  and skill activation through request-scoped engine commands; engine integration
  covers all three routes. A filtered command/skill menu supports arrow-key plus
  Enter/Tab selection and click execution. Argument/file completion and capability
  gating remain. Latest complete gate331 with Clippy/build pass
  (`slash-autocomplete-final-*`).
- Composer `@` references now use runtime file search with recovery-directory
  filtering, a keyboard/click file menu, exact token insertion/caret restore and
  prompt-file mention offsets. Argument/path escaping, stale-scope and full
  runtime/visual acceptance remain. Latest complete gate331 with Clippy/build
  pass (`references-final-*`).
- Editor Find adds preserve-case replacement (`AB`, Alt+C) and find-in-selection
  scope (Alt+L) on top of literal and regex replacement. Browser new-window
  requests now hand credential-free HTTP(S) links to the OS browser while
  denying guest popups and permissions; downloads remain denied. Recovery imports
  older `.vela/.mira/.openshell/.orbit-recovery` transaction directories once
  without overwriting current transactions. Find also supports selection scope
  (Alt+L). Latest complete gate334 with Clippy/build pass
  (`scope-permission-final3-*`).
- The terminal header TUI action launches the configured runtime CLI in a new PTY
  tab using the integrated terminal’s input/IME, resize, selection, ownership and
  exit handling. Embedded agent-panel TUI mode and exact palette/visual parity
  remain. Latest complete gate335 with Clippy/build pass
  (`tui-terminal-final2-*`).
- Changes review now uses case-insensitive stable path sorting and section-header
  file icons. Latest complete gate336 with Clippy/build pass
  (`changes-sort-final-*`).
- HTML/CSS W3C validation is now available from the editor status menu; bounded
  HTML/CSS requests parse JSON/SOAP/text diagnostics and render them in the
  Problems panel. The menu also reports a Source-style error/warning/no-problems
  summary. Editor markers, SCSS validation, cancellation and live network
  acceptance remain. Latest complete gate338 with Clippy/build pass
  (`validation-final2-*`).
- Multi-panel activation foundation: a bounded eight-panel registry stores each
  panel's session/title/transcript/composer/permissions/todos/usage/model/agent/
  scroll snapshot; an accessible tab strip switches panels and closing activates
  the neighbor without deleting the conversation.
- Simultaneous tiled panels now use the reference grid shapes and render every
  open panel at once: each tile keeps its transcript snapshot/scroll container,
  composer input and focus handle, the active tile retains the rich
  transcript/forms/composer, inactive tiles have a compact composer and send,
  and splitter drags persist per-panel width fractions with minimums. Per-session
  document scopes retain each panel's editor workspace and active tab. Inactive
  rich inline controls and per-tile attachment/reference menus remain. Latest
  complete gate346 with Clippy/build pass (`tiled-panels-*`).
- Changes review adds sticky section headers with retained controls,
  intraline emphasis on changed middles of replaced line pairs, EOF-newline
  markers, independent per-section horizontal offsets and character-level
  readonly drag selection with Cmd+C/toolbar copy. The launch queue
  now trims, de-duplicates, caps at ten, preserves order and activates the
  app/window for delivered paths like Source
  `PendingOpenPaths`. Terminal DEC mouse click/drag/all-motion reporting encodes
  X10 and SGR press/release/wheel/drag/move with modifiers, routes PTY input, and
  DECSET 1004 focus changes send focus-in/out notifications. Latest complete
  gate355 with Clippy/build pass (`changes-selection-activation-final-*`).
- Sass output is now an entity with state sync, line-range readonly drag
  selection over wrapped logs, Cmd+C/Copy and a menu toggled state. The 100-log
  viewport regression still passes, and a rendered drag test copies the exact
  selected timestamps/messages across three rows. Validate/SCSS language
  diagnostics, disabled semantics, exact locale metadata and paired visuals
  remain. Latest complete gate357 with Clippy/build pass
  (`sass-validation-final-*`).
- Native SCSS/Sass validation now compiles the active unsaved buffer through the
  pinned Sass compiler without writing output. Relative imports resolve from the
  real source path and workspace root, structured warnings/errors map to the
  existing diagnostic model, and the status menu/Problems panel use the same
  summary path as W3C HTML/CSS. Compiler fixtures cover successful imports,
  compile errors and `@warn` warnings; the status action routes HTML/CSS to W3C
  and SCSS/Sass to Sass. Latest complete gate358 with Clippy/build pass
  (`scss-diagnostics-final-*`).
- The Sass status item now derives exact Source label/description/title data,
  exposes toggled state, marks its AccessKit node disabled while a toggle is
  pending, and drops focus/cursor/activation handlers in that state. Metadata
  and AccessKit-node unit tests cover every label/error/description/pending case.
  Remaining Sass work is locale/platform and live/paired acceptance. Latest
  complete gate359 with Clippy/build pass (`sass-menu-a11y-*`).
- Agent TUI now resolves its program before creating a PTY: a configured
  `VELA_OPENCODE_BINARY` is checked for executability, bare names search PATH,
  and a missing CLI produces an explicit toast with no empty terminal tab.
  Executable/PATH/missing/non-executable cases are unit-tested. Latest complete
  gate360 with Clippy/build pass (`sass-tui-final-*`).
- Terminal IME composition now anchors candidate bounds to the terminal cursor
  cell and composed glyph widths, round-trips UTF-16 ranges, and cancels preedit
  locally without committing on blur. Composition is explicitly discarded when
  switching terminal tabs, closing the active tab, hiding the tray or closing the
  terminal. Candidate-bounds, range, cancel and existing per-tab commit tests
  pass. Real IME candidate interaction and exact fixed-cell/live acceptance
  remain. Latest complete gate361 with Clippy/build pass
  (`terminal-ime-final-*`).
- Terminal cursors now expose DECSCUSR block, underline, beam, hollow-block and
  hidden shapes plus blinking metadata, render the matching shape, and snap a
  cursor sitting on a wide glyph's spacer back to that glyph. Emulator tests
  cover all shape/blink transitions and wide-spacer cursor placement. Blinking
  cadence and live fixed-cell acceptance remain. Latest complete gate362 with
  Clippy/build pass (`terminal-cursor-final-*`).
- Cursor blinking is now a dedicated focus-aware entity. Only blinking styles with
  terminal focus and an active window start a 530 ms timer; phase toggles repaint
  the terminal, while unfocused or steady styles stay visible and stop the timer.
  A clock-driven model test covers toggle cadence, focus loss, steady styles and
  restart. Live blinking/fixed-cell acceptance remains. Latest complete gate363
  with Clippy/build pass (`terminal-blink-final-*`).
- Terminal rows containing wide, combining, emoji or non-ASCII glyphs now use a
  fixed-cell renderer: each grapheme is positioned at its exact column offset,
  wide glyphs span two cells, combining marks stay in their base cell and a
  following glyph starts at the next occupied column. Wide cursors span both
  cells. Pure detection and rendered bounds tests verify `a界e+mark b` columns
  and focused cell widths. ASCII/fallback-font alignment and live acceptance
  remain. Latest complete gate365 with Clippy/build pass
  (`terminal-fixed-cells-final-*`).
- Every terminal cell now uses the fixed-cell renderer, including ASCII and
  fallback-font glyphs; each grapheme is anchored at its column offset and
  measured cell box. Rendered tests verify `ab→de` at exact 7.5px columns and
  wide/combining/emoji placement. Full TUI/live repaint and acceptance remain.
  Latest complete gate366 with Clippy/build pass (`terminal-ascii-cells-final-*`).
- Browser tab metadata and lifecycle now match Source: labels prefer trimmed
  titles, then `www.`-stripped hostnames, `New tab` for an empty URL and the raw
  URL for unparseable values. Closing any tab selects the same neighbor as
  `use-browser-tabs` (shift active index left when an earlier tab closes, pick
  the same-or-last index when the active one closes). Unit tests cover label and
  close-index rules. Live browser isolation/preview acceptance remains. Latest
  complete gate368 with Clippy/build pass (`browser-tabs-final-*`).
- Browser panes now mirror Source placeholders: hidden guest plus spinner while
  loading, per-tab error text with a Try again button, and retry resetting/paging
  the guest. String precedence is tested (error, empty, loading, ready), and tab
  activation carries its own error state. Live browser/isolation/preview
  acceptance remains. Latest complete gate369 with Clippy/build pass
  (`browser-placeholders-final-*`).
- Browser guest failures now reach the per-tab error surface. WKWebView process
  termination emits a `Failed` event, 30-second loading stalls fail with a timeout
  message, per-tab begin/finish/fail transitions clear timers and errors
  correctly, and retry restarts loading. Lifecycle tests cover termination
  messages, timeout transitions and successful finish. Live browser isolation
  and preview acceptance remain. Latest complete gate370 with Clippy/build pass
  (`browser-failures-final-*`).
- The editor now supports indentation folding. Fold ranges are computed from
  non-blank line indentation, nested folds hide only their own bodies, the
  virtualized list maps visible rows back to logical lines with real line
  numbers, and a gutter chevron toggles each fold. Caret/find reveal expands any
  fold containing the target line before scrolling. Model tests cover nested
  ranges/visibility and a rendered test covers gutter toggle plus caret-driven
  expansion. Language-aware folding, splits, multi-cursor and LSP/tooling remain.
  Latest complete gate372 with Clippy/build pass (`editor-folding-final-*`).
- The editor now has a two-pane split view. The Split/Unsplit toolbar action
  selects the next open document tab (wrapping to the first) for the secondary
  pane, creates a distinct editor entity per pane, and focuses the new pane.
  Editor instances have unique element identities so both views mount safely;
  each pane has its own scroll/fold/find state while sharing the document store.
  Rendered tests cover two independent documents updating in their own panes and
  next-tab selection. Multi-group tabs, drag resize, multi-cursor, language
  tooling and Emmet remain. Latest complete gate374 with Clippy/build pass
  (`editor-split-final-*`).
- The split divider is draggable. Split bounds are measured from the live pane,
  pointer deltas update the left fraction with a 20–80% clamp, dragging state is
  cleared on release/out, and the active divider is highlighted. A rendered probe
  test drags the divider and verifies pane growth/fraction; a model test covers
  positive/negative/zero-width/clamp math. Multi-group tabs, multi-cursor,
  language tooling and Emmet remain. Latest complete gate375 with Clippy/build
  pass (`editor-split-resize-final-*`).
- Editor folding is now brace-aware as well as indentation-aware. A scanner
  ignores braces inside single/double/backtick strings and line/block comments,
  emits matching `{}`/`[]`/`()` ranges across lines, and merges same-start ranges
  with indentation folds. Closing braces fold with their body; caret reveal and
  gutter behavior remain. Model tests cover comment/string filtering and
  unindented SCSS-style nesting. Language-provider folding, multi-group tabs,
  multi-cursor, language tooling and Emmet remain. Latest complete gate376 with
  Clippy/build pass (`editor-brace-folding-final-*`).
- Multi-cursor editing adds Cmd+D next-occurrence selection starting from the
  caret word, simultaneous typing/deletion/Enter/Tab across all cursors, and a
  single undo transaction for a scattered edit batch. Secondary selections and
  carets render inline; Escape, mouse clicks, navigation and undo/redo clear
  extras, while IME marking collapses back to the primary cursor. Core tests
  verify shifted caret results and one-step undo for non-overlapping scattered
  ranges; rendered editor tests cover occurrence selection, typing, deletion,
  Escape and undo. Latest complete gate379 with Clippy/build pass
  (`multicursor-final-*`).
- Multi-group editor tabs now support up to four simultaneous panes. `add_editor_group`
  targets the next open tab not already displayed, retaining an empty group when
  all tabs are visible; closing a group removes it without closing documents.
  The grid uses Source percent geometry: two columns, a full-width third row,
  or a 2x2 grid, with draggable X/Y dividers, per-group close buttons and an
  empty-group placeholder. Model tests cover grid coverage, wrap targeting,
  assignment exclusion and the four-group cap; a rendered probe verifies
  three- and four-cell geometry. Latest complete gate382 with Clippy/build pass
  (`editor-groups-final-*`).
- Settings > Servers now owns a session-scoped integration inventory. The engine
  requests MCP servers, plugins and skills from the connected runtime, parses
  array/data payloads while dropping malformed rows, and returns loading/error
  updates. The UI shows three headed lists with source/status badges, empty copy,
  loading rows, an error row plus Retry, and ignores stale responses after a
  session switch. Parser and rendered state tests cover wrappers, fallbacks and
  loading/error/empty/populated rendering. Latest complete gate385 with
  Clippy/build pass (`integrations-final-*`).
- The native editor now expands Emmet abbreviations on Tab/Enter for HTML and
  CSS/SCSS/Less. The parser supports doctype, tags, id/class, text, child/sibling
  chains, `*N` repetition and common CSS number shorthands; unknown tags and
  noise are rejected, and `<script>` content is guarded. Snippets flatten tab
  stops with the caret at `$0`, and the abbreviation is replaced atomically.
  Parser/insertion tests mirror the reference cases; a rendered editor test tabs
  through `div` and `ul>li*3`. Latest complete gate391 with Clippy/build pass
  (`emmet-final-*`).
- Provider usage now consumes the desktop bridge snapshot. The engine reads the
  XDG/macOS `vela-usage.json` candidates, ignores malformed/stale documents older
  than 15 minutes, filters provider-less rows and parses windows, credits, plan
  and error fields. The session-usage popup requests the snapshot on open and
  renders provider cards with percent-left bars, reset wording, plan/status dots,
  credits and loading/empty/error states. Tests cover snapshot parsing/fallback
  and formatter/rendered cards. Live OAuth/key fetches and credential refresh
  remain. Latest complete gate395 with Clippy/build pass (`provider-usage-final-*`).
- Sass log timestamps now use a localized `jms` date template with the explicit
  current locale, covering seconds and the locale's 12/24-hour cycle instead of a
  fixed medium time style. Tests pin `en_US` and `en_GB` at epoch UTC and assert
  seconds, meridiem presence/absence; the rendered selection test verifies the
  localized clock prefix survives copied rows. Platform/live visual acceptance
  remains. Latest complete gate396 with Clippy/build pass (`sass-locale-final-*`).
- The editor now has bounded document-word completion. Ctrl+Space opens a
  popup for the full identifier range around the caret; candidates are unique
  document words ranked by prefix match, frequency, length and spelling, with
  cycling arrows and Enter/Tab/mouse acceptance. Typing re-filters, while
  navigation, Escape, clicks and IME marking dismiss it. Model tests cover
  Unicode ranges/filtering; a rendered test expands `al` to `alpha` in a real
  editor popup. Semantic LSP providers and workspace symbols remain. Latest
  complete gate400 with Clippy/build pass (`completion-final-*`).
- JSON documents now get bounded local syntax diagnostics without a language
  server. The editor parses on revision changes, maps parser line/column to
  one-based editor coordinates, marks the offending row with a red gutter stripe
  and shows an accessible banner; fixing the document clears both immediately.
  Model tests cover valid/empty/invalid JSON and coordinates; a rendered test
  verifies marker/banner creation and clearing. Other languages and semantic
  LSP tooling remain. Latest complete gate403 with Clippy/build pass
  (`json-diagnostics-final-*`).
- JavaScript/TypeScript and CSS/SCSS now add bounded local delimiter
  diagnostics. A string/comment/template-aware scanner reports unmatched closing
  brackets, unclosed openings and unterminated strings/comments with one-based
  positions; extension routing selects JS/TS/JSX/TSX, CSS or SCSS and respects
  SCSS line comments. The editor reuses the red marker/banner path and clears
  fixed documents. Model tests cover valid code, comments/templates, unclosed
  and extra delimiters; a rendered JS test verifies marker/banner clearing.
  Semantic LSP remains. Latest complete gate406 with Clippy/build pass
  (`script-css-diagnostics-final-*`).
- Editor folding now also understands comment region markers. Nested
  `// #region`/`// #endregion`, `/* #region */`, `<!-- #region -->` and `#region`
  markers produce independent folds, merged with indentation and brace ranges
  through the existing gutter/caret-reveal path. Model tests cover nesting,
  CSS/hash markers and unmatched ends; a rendered JS test collapses a region.
  Full language-provider folding and semantic LSP remain. Latest complete
  gate408 with Clippy/build pass (`language-folding-final-*`).
- The editor now has bounded document-symbol navigation. F12 locates the
  identifier under the caret and jumps directly when a common declaration
  (`function`/`class`/`const`/`let`/`var`/type-style, including export/async
  prefixes) is unique; Shift+F12 lists whole-word references. Ambiguous or
  missing results open an AccessKit list with arrow/Enter/mouse navigation and
  an empty state. Model tests cover declarations, references, Unicode words and
  offsets; a rendered JS test verifies direct definition jumps and the
  references panel. Cross-file semantic LSP remains. Latest complete gate412
  with Clippy/build pass (`symbols-final-*`).
- Workspace symbol search indexes declarations from every open document in the
  active workspace. Cmd+T filters by the identifier under the caret (or lists
  all when the caret is not on one), sorts/dedupes file-qualified results and
  opens a bounded panel. Enter/arrow/mouse selection emits a cross-file
  open-at-offset event; the app opens the owning tab, restores its editor and
  places the caret. Model tests cover query filtering, ordering, limits and file
  keys; a rendered two-document test verifies panel contents and emitted
  `OpenSymbol` targets. Language-server semantic/type tooling remains. Latest
  complete gate414 with Clippy/build pass (`workspace-symbols-final-*`).
- HTML/XML element folding was added to the language folding provider. A
  tag-aware scanner matches nested elements across lines, handles quoted `>` in
  attributes, ignores comments, void/self-closing tags and raw script/style
  content, and merges with indentation/brace/region ranges. Model tests cover
  nesting, comments, raw scripts, uppercase tags, voids and unclosed elements;
  a rendered HTML test collapses a `<div>` body. Server-provided fold ranges
  remain. Latest complete gate416 with Clippy/build pass (`markup-folding-final-*`).
- Slash commands now support workspace file argument completion. After
  `/command partial` with a non-empty whitespace-free token, the engine searches
  workspace files; results are prefix-ranked, capped and shown in a shared menu
  with arrow/Enter/Tab/click selection. Accepting inserts the path plus a space,
  keeps the command in the composer and clears the menu; compact/compress do not
  open file arguments. Model tests cover trigger parsing/filtering; a rendered
  menu test verifies production rows and pointer dispatch. Command capability
  gating, quoting and visual acceptance remain. Latest complete gate419 with
  Clippy/build pass (`slash-args-final-*`).
- Slash-menu edge cases are covered: Escape now dismisses command and file
  argument menus until the composer is edited again; file-argument completion
  only runs when the command exists in the loaded catalog; and catalog parsing
  deduplicates case-insensitive names by builtin > command > skill priority,
  retaining the first description within a priority. Model tests cover
  dismissal transitions, command membership and duplicate resolution.
  Quoting/escaping and visual acceptance remain. Latest complete gate422 with
  Clippy/build pass (`slash-menu-edges-final-*`).
- Completed slash-command file arguments now escape spaces and backslashes at
  insertion and unescape only those inserted sequences before runtime delivery;
  unrelated user backslashes such as regex escapes remain intact. The shared
  insertion helper returns the exact caret offset, and the argument menu renders
  paths containing spaces. Model tests round-trip spaces/backslashes and a
  rendered menu test includes a spaced path. Broader capability gating and
  visual acceptance remain. Latest complete gate423 with Clippy/build pass
  (`slash-quoting-final-*`).
- The editor now has bounded declaration hover. Pointing at a document
  identifier resolves its local declaration and shows kind, source signature and
  shallow inferred scalar type (`string`/`number`/`boolean`/`array`/`object`/
  `function`/`null`) with a line number. The popup appears near the pointer and
  clears over non-symbols, on navigation or interaction. Model tests cover
  declaration kinds/inference and unknown words; a rendered mouse-hover test
  verifies presentation and dismissal. Full semantic/type-server hover remains.
  Latest complete gate426 with Clippy/build pass (`hover-final-*`).
- Signature help now resolves local `function`, arrow and function-expression
  declarations and appears while the caret is inside a known call. The active
  parameter follows top-level commas while ignoring nested calls/arrays/objects
  and string commas. The card shows the full signature and active parameter and
  disappears after the closing parenthesis; declaration parameter editing is
  excluded. Model tests cover parameters/nesting/edge cases and a rendered JS
  test verifies the card and dismissal. Full language-server signatures remain.
  Latest complete gate430 with Clippy/build pass (`signature-final-*`).
- Shift+F12 now searches whole-word references across every open workspace
  document instead of only the active buffer. Results are file-qualified and
  sorted by path/offset, shown under a `References for X` title with previews,
  and selecting one emits `OpenSymbol` so the app opens/focuses the owning tab
  at the reference. Model tests cover cross-file ordering/limits; a rendered
  two-document test verifies rows and a cross-file jump. Full language-server
  reference semantics remain. Latest complete gate431 with Clippy/build pass
  (`workspace-references-final-*`).
- The editor now highlights whole-word occurrences of the identifier under a
  collapsed caret, matching Monaco's default occurrence highlight. Matches are
  source-ordered, Unicode-aware, skip embedded identifiers and cap at 100; the
  subtle accent background paints before selections so the primary selection and
  find matches keep their stronger colors. Occurrences recompute on caret/edit
  changes and clear for non-collapsed selections. Model tests cover ordering,
  Unicode/edges and limits; a rendered test verifies caret-word updates and
  clearing on selection. Latest complete gate435 with Clippy/build pass
  (`occurrences-final-*`).
- Provider credential forms now render native numeric/integer steppers beside
  their inputs. Steppers follow HTML's base (minimum, then default, then zero)
  and unit step, stay integral for integer fields, clamp to min/max and disable
  at bounds. Numeric JSON answers normalize integral values so they serialize as
  `2` rather than `2.0`. Validation errors now render a per-field bubble anchored
  under the offending control instead of only a form-level line; the bubble
  clears on edit, keeps focus/scroll reveal, and the API-key field already uses
  the secure password input (masked value, copy/cut blocked, AX password role).
  Model tests cover step bases/clamping; a rendered test verifies key/number
  bubbles, stepper clicks, bounds and a successful submission. Complete
  select-menu behavior and visual acceptance remain. Latest complete gate437
  with Clippy/build pass (`provider-form-final-*`).
- Provider option menus now cover complete keyboard selection. The inline
  single-select menu includes a `Select` placeholder plus the current option,
  highlights the active cursor and supports arrows/Home/End/Cmd+Home/End,
  Enter/Space acceptance, Escape dismissal and pointer selection. Multi-select
  Space toggles the focused option without moving, plain arrows/Home/End move
  focus without changing the selection, Shift extends ranges, Cmd/Ctrl+A
  selects all and Cmd/Ctrl+Home/End jumps to the bounds. Model tests cover
  focus-only movement and shift anchoring; a rendered test verifies single
  placeholder/current selection plus multi toggle/range/select-all behavior.
  Live usage refresh and visual acceptance remain. Latest complete gate439 with
  Clippy/build pass (`provider-select-final-*`).
- Native preferences now hand off to the engine rather than saving directly from
  the UI. `Engine::apply_settings` normalizes limits, updates the in-memory
  settings snapshot and live warm-cache budgets, persists atomically off the
  async thread and emits a save-error event the app surfaces as a toast.
  `Engine::settings()` now returns that handoff snapshot, and the settings UI
  routes every preference write through it. Warm-cache tests cover limit
  eviction; engine-flow tests verify persistence, normalization and live
  limits; a rendered app test changes the shell and observes the engine handoff.
  Mobile pairing and live page acceptance remain. Latest complete gate442 with
  Clippy/build pass (`settings-handoff-final-*`).
- The editor now supports bounded workspace rename. F2 reads the identifier at
  the caret, collects whole-word occurrences from every open workspace document
  (bounded to 1000), and opens an inline rename prompt with the current name
  selected. Invalid, empty, unchanged, oversized and no-symbol cases are
  reported without editing. Confirming re-plans against current buffers, applies
  a scattered atomic edit per document and keeps the active caret on the
  replacement matching the original caret; undo stays document-local. Model
  tests cover identifier validation, cross-file ranges, limits and error cases;
  a rendered two-document test verifies invalid feedback, cross-file rename,
  caret placement and one-step local undo. Full language-server rename remains.
  Latest complete gate446 with Clippy/build pass (`rename-final-*`).
- TypeScript documents now get bounded semantic type diagnostics for annotated
  primitive literals. `const`/`let`/`var` declarations with a literal
  initializer compare against string/number/boolean/null/any/unknown unions,
  array element types and conservative object/function annotations; comments
  are ignored. Mismatches reuse the existing red marker and accessible banner,
  clearing when the literal is fixed. Model tests cover unions, arrays,
  objects, comments, nested indentation and non-literal expressions; a rendered
  TypeScript test verifies the mismatch and clearing. Full language-server type
  checking remains. Latest complete gate449 with Clippy/build pass
  (`semantic-types-final-*`).
- JavaScript/TypeScript files now report duplicate top-level `const`, `let`,
  `class` and `type` declarations at the later declaration, naming the first
  line. `var`, functions (overloads), enums/interfaces (legal merging), ambient
  declarations and indented/nested scopes are excluded to avoid false
  positives; quote-aware comment masking covers line/block comments and string
  contents. Diagnostics reuse the existing marker/banner path and clear when
  the duplicate is renamed. Model tests cover each exclusion and Unicode names;
  a rendered JavaScript test verifies the marker/banner and clearing. Full
  language-server scope/type analysis remains. Latest complete gate451 with
  Clippy/build pass (`duplicate-declarations-final-*`).
- JavaScript/TypeScript documents now flag unused `const`/`let` declarations
  when no whole-word use exists outside comments. Any occurrence (including
  writes, strings or JSX) counts as a use to stay conservative. Exported and
  underscore-prefixed names, destructuring, multiple declarators, `for` heads
  and TypeScript declaration files are excluded. Diagnostics reuse the existing
  marker/banner path and clear once the value is used. Model tests cover the
  exclusions and routing; a rendered JavaScript test verifies the marker/banner
  and clearing. Latest complete gate454 with Clippy/build pass
  (`unused-declarations-final-*`).
- JavaScript/TypeScript now report unused simple parameters on function
  declarations, function expressions and arrows. Defaults, rest parameters and
  generic commas are parsed; destructured, `this` and underscore-prefixed
  parameters are excluded, and any whole-word use (including template strings)
  counts. Signature-only declarations and `.d.ts` files are skipped. Model tests
  cover functions, arrows, defaults, rest, generics, overload signatures,
  comments and Unicode; a rendered JavaScript test verifies the marker/banner
  and clearing after the parameter is used. Latest complete gate456 with
  Clippy/build pass (`unused-parameters-final-*`).
- The editor now offers bounded quick fixes from local diagnostics. Cmd+. opens
  a keyboard/mouse menu for the caret line: remove a single-line unused `const`
  or `let` declaration (multi-declarator, multi-line and exported lines are
  refused) or prefix an unused parameter with `_`. Applying an action replaces
  the exact range through the normal undo transaction and clears the diagnostic.
  Model tests cover derivation, line ownership and unsafe cases; a rendered test
  applies both fixes and verifies clearing. Latest complete gate459 with
  Clippy/build pass (`quick-fixes-final-*`).
- TypeScript function declarations now report a non-void annotated body with no
  `return` or `throw`. Overload signatures, generators, object-literal return
  types, void-like/never/any/unknown and Promise<void>/undefined unions and
  unclosed bodies are excluded; nested returns count conservatively. Diagnostics
  reuse the marker/banner path and clear once a return is added. Model tests
  cover bodies, throws, overloads, generators, unions and unclosed functions; a
  rendered TypeScript test verifies the marker/banner and clearing. Latest
  complete gate461 with Clippy/build pass (`missing-return-final-*`).
- JavaScript/TypeScript single-line imports now report unused default, named,
  aliased, type-only and namespace bindings. Side-effect imports, underscore
  names, multi-line clauses and comments are excluded; any whole-word use
  (including type positions, re-exports and strings) counts. Model tests cover
  every binding form, aliases, comments, multiline imports and Unicode; a
  rendered JavaScript test verifies marker/banner creation and clearing after
  use. Latest complete gate463 with Clippy/build pass (`unused-imports-final-*`).
- Unused-import diagnostics now participate in Quick Fixes. Cmd+. removes the
  whole line when the import has exactly one binding (default, namespace,
  type-only or single named specifier); multi-binding imports are deliberately
  left without an action to avoid comma/list rewriting. Model tests cover
  removal, multi-binding/wrong-column refusals; the rendered quick-fix test now
  applies the import removal and verifies clearing. Latest complete gate464 with
  Clippy/build pass (`import-quick-fix-final-*`).
- Multi-binding named imports now support safe specifier removal in Quick Fixes.
  First, middle and last specifiers remove the correct adjacent comma while
  preserving the rest of the clause; default-plus-named clauses and clauses
  containing comments are deliberately left unmodified. Model tests verify
  resulting text for each position and refusal cases; the rendered quick-fix
  test removes one named binding and confirms the surviving import and cleared
  diagnostic. Latest complete gate464 with Clippy/build pass
  (`multi-import-quick-fix-final-*`).
- Combined default-plus-named imports now support Quick Fix removal too. Unused
  named bindings remove the correct specifier and comma while keeping the
  default; removing the only named binding also removes the braces and preceding
  comma, and an unused default removes its comma plus following space while
  preserving `import type`. Commented clauses remain untouched. Model tests
  cover default+named, only-named, default removal and type-only imports; the
  rendered quick-fix flow removes an unused default and verifies the surviving
  named import and cleared diagnostic. Latest complete gate464 with Clippy/build
  pass (`default-import-quick-fix-final-*`).
- Default-plus-namespace imports now parse both bindings and support Quick Fix
  removal: an unused namespace removes its comma clause while keeping the
  default, and an unused default removes its comma while keeping `* as ns`.
  `import type` prefixes are preserved. Model tests cover both removal
  directions and diagnostics; a rendered test removes an unused namespace and
  confirms the surviving default and cleared diagnostic. Latest complete
  gate464 with Clippy/build pass (`namespace-import-quick-fix-final-*`).
- Multi-line named import clauses are now parsed for unused-import diagnostics;
  each specifier reports its own line/column, including default-plus-multiline
  named and `import type` forms. Quick Fixes removes a specifier's whole line
  (with or without a trailing comma), leaving empty braces valid; specifiers
  sharing a line are diagnosed but not auto-removed. Model tests cover
  diagnostics and line removal; a rendered test removes a multi-line specifier
  and confirms the surviving clause and cleared diagnostic. Latest complete
  gate465 with Clippy/build pass (`multiline-import-quick-fix-final-*`).
- Multi-line import entries sharing one line now get precise Quick Fix removal.
  First, middle and last specifiers remove the correct comma while preserving
  indentation; single-specifier lines still remove the whole line. Model tests
  cover first/middle/last and no-trailing-comma cases; the rendered flow removes
  a shared-line specifier and verifies the surviving entry. Latest complete
  gate465 with Clippy/build pass (`shared-import-quick-fix-final-*`).
- Imports whose opening brace is on a following line (`import` / `import D,` then
  `{`) are now parsed for unused-import diagnostics, including default-plus-named
  and `import type` forms. Quick Fixes removes whole or shared specifier lines and
  can remove the default while preserving the braced list. Model tests cover
  diagnostics and default/named removals; the rendered flow removes a specifier
  from a brace-next-line import and verifies the surviving clause. Latest
  complete gate465 with Clippy/build pass (`next-line-brace-import-final-*`).
- Imports with `from` on a later line (including `from` followed by a newline
  before the module string) are now parsed for unused-import diagnostics and
  Quick Fix removal. This covers same-line braces with a later `from`, braced
  defaults, and imports later in the file; statement slicing keeps edits scoped
  to the import. Model tests cover diagnostics, specifier/default removal and a
  non-leading import; the rendered flow removes a specifier where `from` is on
  the next line. Latest complete gate466 with Clippy/build pass
  (`from-next-line-import-final-*`).
- Runtime capability flags now come from the engine's OpenCode manifest model.
  Runtime-info carries provider-credential, permission, steering, model,
  attachment, command, agent, resume/fork and TUI flags computed from the
  connected CLI. Settings > Providers shows a managed-by-runtime callout when
  credential editing is unsupported; the Model safety section hides permissions
  and steering rows when disabled and renders a Queue/Steer preference when
  steering is available. Model and rendered tests cover the manifest flags and
  each visibility transition. Latest complete gate468 with Clippy/build pass
  (`runtime-capabilities-final-*`).
- Runtime capabilities now gate composer affordances and the Agent TUI. The
  attach button, slash-command menu, agent picker and TUI control each render
  only when the reported capability is true; commands are omitted from the
  slash menu query when disabled. The rendered app test keeps the terminal tray
  open, switches all four flags and verifies the controls appear and disappear.
  Latest complete gate474 with Clippy/build pass (`capability-affordances-final-*`).
- Duplicate top-level declarations now offer a Quick Fix that renames the later
  declaration to the first unused `name2`, `name3`, … identifier. Only the
  duplicate declaration is edited; references remain untouched. Model tests
  verify range/replacement, collision skipping and refusals; a rendered test
  renames an exported duplicate and verifies the diagnostic clears. Latest
  complete gate470 with Clippy/build pass (`duplicate-rename-quick-fix-final-*`).
- Type-mismatch diagnostics now offer "Change type to '<literal>'" for simple
  primitive/union annotations and "Remove type annotation" for any annotated
  declaration. The change replaces only the annotation; removal keeps the
  spacing before `=`. Model tests verify both edits, unions, complex
  annotations and refusals; a rendered TypeScript test applies the change and
  then the removal, verifying the banner clears each time. Latest complete
  gate472 with Clippy/build pass (`type-mismatch-quick-fix-final-*`).
- Missing-return diagnostics now offer "Change return type to 'void'"
  (`Promise<void>` for async functions) or "Remove return type annotation".
  Both edit only the annotation and clear the diagnostic. Model tests cover
  sync/async declarations and refusals; a rendered TypeScript test applies
  change-type and removal. Latest complete gate474 with Clippy/build pass
  (`missing-return-quick-fix-final-*`).
- Syntax diagnostics now offer Quick Fixes: append the missing closer for
  unclosed braces/brackets/parentheses, remove an unexpected closer, add a
  closing quote for unterminated strings, and close unterminated block comments.
  Model tests cover each message and invalid positions; a rendered JavaScript
  test adds a missing brace and then removes an unexpected one, clearing the
  banner each time. Latest complete gate476 with Clippy/build pass
  (`syntax-quick-fix-final-*`).
- Locally declared functions now report underflow/overflow calls. Top-level
  declarations with one-line parameter lists are indexed; optional/default/rest
  parameters set the allowed range, and comments, strings, nested commas and
  member calls are handled. Quick Fixes insert missing `undefined` arguments or
  remove extra arguments with exact spans. Model tests cover counts, defaults,
  rest, nested commas and exclusions; a rendered JavaScript test fills and
  trims calls, clearing the banner. Latest complete gate479 with Clippy/build
  pass (`argument-count-final-*`).
- Uppercase identifiers with no local declaration/import and exactly one matching
  exported declaration in another open document now report "Cannot find name".
  The Quick Fix inserts a named or default import using a computed relative
  path; strings, comments, member accesses, object keys, ambiguous exports and
  local/imported names are excluded. Model tests cover resolution, exclusions
  and relative paths; a rendered two-document test inserts an auto-import and
  clears the diagnostic. Latest complete gate483 with Clippy/build pass
  (`auto-import-final-*`).
- Auto-import insertion now skips a shebang and one `"use strict"`/`'use strict'`
  directive. Editor diagnostics also detect changes to other open documents via
  a workspace revision fingerprint and refresh unresolved-identifier results
  without requiring an edit to the active buffer. Model tests cover insertion
  offsets; a rendered test imports after `"use strict"` and observes the
  diagnostic appear/disappear as another document's export is removed and
  restored. Latest complete gate485 with Clippy/build pass
  (`auto-import-refresh-final-*`).
- Lowercase identifiers called as functions now auto-import when exactly one
  other open document exports them as a function. Non-call references, member
  calls, arrow-function const exports and common globals are still excluded;
  uppercase imports behave as before. Model tests cover named/default function
  exports and all exclusions; a rendered test inserts a lowercase named import
  and clears the diagnostic. Latest complete gate487 with Clippy/build pass
  (`lowercase-auto-import-final-*`).
- Lowercase arrow/function-expression exports (`export const helper = () => ...`,
  async arrows, `function` expressions) now count as function exports for the
  same auto-import heuristic. Initializers are inspected conservatively: a
  leading `function`/`async function` or parenthesized/identifier arrow shape
  qualifies, while plain values and conditional expressions do not. Model tests
  cover each export form and non-function exclusions; a rendered test imports an
  arrow-exported helper and clears the diagnostic. Latest complete gate488 with
  Clippy/build pass (`arrow-auto-import-final-*`).
- Object-literal duplicate keys now report at the later key with the first key's
  line. Plain identifier and quoted keys are compared; computed keys, spreads,
  methods, accessors, class/function blocks and non-object braces are excluded.
  A safe Quick Fix removes the earlier duplicate when its value is a primitive
  literal, preserving the last-wins result; side-effecting or referenced values
  are diagnosed without an action. Model tests cover detection, exclusions and
  removal spans; a rendered JavaScript test removes the earlier duplicate and
  clears the marker/banner. Latest complete gate496 with Clippy/build pass
  (`object-keys-final-*`).
- Duplicate simple function/arrow parameter names now report at the later
  parameter with a shadow-safe Quick Fix. The earlier parameter is renamed to
  `_name` (only when that name is unused in the document), which cannot change
  body resolution or arity because the later binding already wins. Destructured,
  `this`, multiline and already-underscored parameters are excluded. Model tests
  cover offsets, defaults/rest, arrows, Unicode and exclusions; a rendered test
  applies the rename and clears the marker/banner. The watcher-readiness wait
  was widened from 15s to 30s after a full-suite load timeout (isolated pass).
  Latest complete gate500 with Clippy/build pass (`duplicate-parameters-final-*`).
- Single-line ES imports now report duplicate local bindings within one import
  statement. Identical named specifiers (`a`/`a`, `a as b`/`a as b`, duplicate
  `type` specifiers) get a safe removal of the later redundant specifier with
  correct first/middle/last comma handling. Default/namespace collisions and
  aliases binding different exports are reported without an action; type-only
  and value bindings of the same name are kept distinct. Model tests cover
  detection, removal spans, aliases, Unicode, comments and bounded exclusions; a
  rendered test removes the redundant specifier and clears the marker/banner.
  Latest complete gate507 with Clippy/build pass (`duplicate-imports-final-*`).
- Single-line named exports now report duplicate exported names within one
  statement, including re-exports. Identical specifiers (`a`/`a`, `a as b`/`a as
  b`, duplicate `type` specifiers) offer removal of the later redundant specifier
  with first/middle/last comma handling. Aliases exporting the same name from
  different locals are reported without an action; type-only and value exports
  stay distinct. Model tests cover removal spans, aliases, re-exports, Unicode,
  comments and bounded exclusions; a rendered test removes the redundant
  specifier and clears the marker/banner. Latest complete gate514 with
  Clippy/build pass (`duplicate-exports-final-*`).
- Single-line object-destructuring declarations (`const`/`let`/`var`) now report
  duplicate local bindings, including exported patterns. Identical shorthand or
  aliased properties offer removal of the later redundant property with
  first/middle/last comma handling; different keys, defaults, computed keys,
  rest elements and nested patterns are reported conservatively or skipped.
  Model tests cover removal spans, aliases, quoted keys, defaults and bounded
  exclusions; a rendered test removes the redundant binding and clears the
  marker/banner. Latest complete gate520 with Clippy/build pass
  (`destructuring-final-*`).
- Single-line class declarations now report duplicate members. Accessor pairs,
  static/instance pairs and method/field pairs are kept distinct. Safe removal
  drops the earlier method/accessor, or an earlier field whose initializer is a
  primitive literal; constructors, private members and side-effecting field
  initializers are diagnosed without an edit. Multiline classes, class
  expressions, computed names, decorators and static blocks are excluded. Model
  tests cover removal spans and exclusions; a rendered test removes the earlier
  method and clears the marker/banner. Latest complete gate527 with Clippy/build
  pass (`class-members-final-*`).
- Single-line class expressions (`const A = class { … }`, named and
  parenthesized forms, arrow-returned classes) now receive the same duplicate
  member diagnostics and safe removal. Strings that merely contain `class` are
  ignored via structural masking. Model tests cover expression forms and false
  positives; a rendered test removes the earlier method in a class expression
  and clears the marker/banner. Latest complete gate529 with Clippy/build pass
  (`class-expression-members-final-*`).
- Multi-line class declarations and expressions now use the same duplicate-member
  checks and safe removal. Earlier one-per-line members are removed with their
  indentation/newline; shared-line members keep the existing span removal. Field
  parsing supports semicolon-terminated multi-line fields, with side-effecting
  initializers still diagnostic-only. Model tests cover multiline methods,
  fields, class expressions, accessor/static pairs and unsafe cases; a rendered
  test removes an earlier method line and clears the marker/banner. Latest
  complete gate532 with Clippy/build pass (`multiline-class-members-final-*`).
- Duplicate export-name checks now cover quoted arbitrary module-export names
  (`export { a as "read" }`), single or double quoted, including re-exports.
  Quoted and identifier forms of the same name are compared together; identical
  local bindings remove the later specifier, while different locals stay
  diagnostic-only. Escaped string interiors remain bounded. Model tests cover
  quoted duplicates, mixed quote styles, re-exports and type-only separation; a
  rendered test removes a redundant quoted specifier and clears the
  marker/banner. Latest complete gate535 with Clippy/build pass
  (`quoted-export-names-final-*`).
- Duplicate import-binding checks now parse quoted imported names
  (`import { "read" as read }`), single- or double-quoted, including mixed quote
  styles and Unicode. Two specifiers binding the same local from the same quoted
  name remove the later redundant specifier; different quoted sources stay
  diagnostic-only, and type-only/value bindings remain distinct. Quoted
  imports without a local alias are treated as invalid and skipped. Model tests
  cover each form; a rendered test removes the redundant quoted specifier and
  clears the marker/banner. Latest complete gate538 with Clippy/build pass
  (`quoted-import-names-final-*`).
- Switch statements now report duplicate literal case labels and duplicate
  `default` clauses, including nested switches. Number/string/boolean/null/
  undefined/identifier labels compare by value; complex expressions are skipped.
  A safe Quick Fix removes a later duplicate label only when its clause is empty
  (immediately followed by another label or the closing brace), so fallthrough
  and statement execution are unchanged; non-empty duplicates stay
  diagnostic-only. Model tests cover empty/non-empty clauses, defaults, nested
  switches, strings/comments and complex labels; a rendered test removes an
  empty duplicate case and clears the marker/banner. Latest complete gate546
  with Clippy/build pass (`switch-cases-final-*`).
- TypeScript enums now report duplicate member names (identifier, quoted and
  mixed forms) and duplicate numeric/string values, including auto-numbered
  collisions following TypeScript's previous-value-plus-one rule. A safe Quick
  Fix removes a later duplicate name only when both members carry the same
  explicit literal initializer, preserving the name, value and subsequent
  numbering; value aliases, differing values and computed initializers stay
  diagnostic-only. Model tests cover names/values, auto/hex/string literals,
  prefixes, comments and multiline/shared lines; a rendered test removes the
  later duplicate and clears the marker/banner. Latest complete gate553 with
  Clippy/build pass (`enum-members-final-*`).
- TypeScript interface bodies now report duplicate property signatures, including
  quoted names, multiline interfaces and exported/declared forms. Method
  overloads remain legal and are skipped; property/method conflicts are
  reported. A safe Quick Fix removes the later duplicate only when both
  declarations are identical (readonly/optional flags and type text), preserving
  the interface shape; differing types/modifiers stay diagnostic-only. Model
  tests cover identical/differing members, overloads, prefixes and bounded
  exclusions; a rendered test removes the duplicate property and clears the
  marker/banner. Latest complete gate560 with Clippy/build pass
  (`interface-members-final-*`).
- TypeScript union type aliases now report duplicate literal members
  (string/number/boolean/null/undefined), including multiline leading-pipe
  unions and `export`/`declare` aliases. Non-literal type references and complex
  members are skipped. Removing a duplicate union literal is always safe; the
  Quick Fix handles inline first/middle/last positions and whole multiline
  literal lines. Model tests cover values, quote styles, multiline bounds and
  exclusions; a rendered test removes the later literal and clears the
  marker/banner. Latest complete gate567 with Clippy/build pass
  (`union-members-final-*`).
- Inline TypeScript object type literals (`type Name = { ... }`) now share the
  interface member analysis: duplicate property signatures report at the later
  member, method overloads are skipped and property/method conflicts are
  reported. A safe Quick Fix removes a later duplicate only when readonly/
  optional flags and type text are identical; differing declarations stay
  diagnostic-only. Model tests cover identical/differing properties, quoted
  names, nested types, overloads and bounded exclusions; a rendered test removes
  the duplicate property and clears the marker/banner. Latest complete gate574
  with Clippy/build pass (`type-literal-properties-final-*`).
- Inline object type literals in annotation position (`const options: { … } =`)
  now receive the same duplicate-property analysis. Variable declarations with a
  simple annotated name are checked inline and across lines; exported
  declarations are included. Object-literal initializers, ternaries,
  destructuring and `for` bindings are excluded. Model tests cover annotation
  forms and exclusions; a rendered test removes the duplicate and clears the
  marker/banner. Latest complete gate577 with Clippy/build pass
  (`annotation-type-properties-final-*`).
- Inline object type literals in function and arrow signatures now receive the
  same duplicate-property analysis: parameter annotations (including function
  expressions) and return annotations. Default initializers, ternaries and
  destructured annotations are excluded. Model tests cover parameters, returns,
  arrows and exclusions; a rendered test removes a duplicate parameter type
  property and clears the marker/banner. Latest complete gate581 with
  Clippy/build pass (`signature-type-properties-final-*`).
- TypeScript generic type-parameter lists on `function`/`type`/`interface`/`class`
  declarations now report duplicate names. A safe Quick Fix removes the later
  duplicate only when both entries are plain identifiers (no constraint or
  default), preserving name resolution; constrained/defaulted duplicates stay
  diagnostic-only. Nested generics and arrow constraints are parsed without
  confusing `=>` for a closing bracket. Model tests cover declarations,
  multiline/shared-line removal, constraints/defaults and string/comment bounds;
  a rendered test removes the duplicate and clears the marker/banner. Latest
  complete gate588 with Clippy/build pass (`type-parameters-final-*`).
- Generic type-parameter checks now cover arrow functions and class/object
  methods in addition to declarations. Arrow generics are found from `=>` back
  through the parameter list and angle list; method generics require a method
  name plus a direct body block, avoiding declaration lists and comparisons.
  The same plain-only removal rule applies; constrained/defaulted duplicates stay
  diagnostic-only. Model tests cover arrows, async arrows, multiline lists,
  class/object/static methods and exclusions; a rendered arrow test removes the
  duplicate and clears the marker/banner. Latest complete gate591 with
  Clippy/build pass (`method-type-parameters-final-*`).
- Generic type-parameter checks now include return-typed class/object methods
  and anonymous generic function expressions. Return annotations are skipped
  conservatively when locating the method body; anonymous functions are parsed
  when `<` follows the `function` keyword and are excluded from the method scan
  to avoid double reports. Model tests cover returned/object-return methods,
  anonymous functions and exclusions; a rendered test removes the anonymous
  function duplicate and clears the marker/banner. Latest complete gate593 with
  Clippy/build pass (`returned-method-type-parameters-final-*`).
- Nested inline object type literals now recurse through the shared type-member
  analysis (bounded to eight levels). Duplicate properties inside nested
  properties are reported with nested offsets and can use the same identical-
  member Quick Fix. Interfaces and annotated/signature type literals share the
  recursion. Model tests cover alias/annotation nesting, three levels, multiline
  removal and differing-type refusal; a rendered test removes a nested duplicate
  and clears the marker/banner. Latest complete gate596 with Clippy/build pass
  (`nested-type-properties-final-*`).
- Duplicate-parameter and unused-parameter analysis now includes class and object
  method parameter lists (including static/async/return-typed methods and
  function expressions), found through member-context heuristics that avoid
  calls and control-flow parentheses. The existing shadow-safe `_name` Quick Fix
  applies; a setter accessor fixture now uses `_v` to stay intentionally clean.
  Model tests cover methods/function expressions and exclusions; a rendered test
  applies the method-parameter fix and clears the marker/banner. Latest complete
  gate598 with Clippy/build pass (`method-parameter-duplicates-final-*`).
- The editor now highlights the matching bracket for the caret's `()`, `[]` or
  `{}` pair, including mixed nesting and multiline pairs. Brackets inside strings
  and comments are ignored via masking; hovering just after an opening or on a
  closing bracket resolves the same pair. The subtle accent highlight paints
  beneath find/selection layers. Model tests cover nesting, caret edges,
  strings/comments, unbalanced input and escaped quotes; a rendered test moves
  the caret across bracket/comment/whitespace positions and verifies state.
  Latest complete gate603 with Clippy/build pass (`bracket-highlight-final-*`).
- Transcript synthetic records now match Source's delegated-agent flow. Current
  `<subagent id agent description state>` tags and legacy `agent=`/`prompt=`
  notices parse into agent cards; explicit child ids open even with a stale
  session inventory, while legacy notices match children under the active parent
  by description/agent and prefer the latest update. A matching task-tool row
  collapses into the card so one clickable entry remains, and internal
  `<system-reminder>` scaffolding is hidden. Model tests cover tag/legacy
  parsing, child resolution, card titles/agent names and dispatch signatures; a
  rendered app test clicks the card and verifies the child panel opens. Latest
  complete gate614 with Clippy/build pass (`subagent-links-final-*`).
- Native watcher and engine-flow event waits were widened from 8s to 20s. Both
  failures were load-dependent delivery timeouts during full-suite runs; the
  isolated cases already passed. The wider bound preserves real failure
  detection without treating slow FSEvents/Sass delivery as missing events.
- Go-to-definition now falls back to exact declarations in other open workspace
  documents when the active document has none. A unique match emits the existing
  cross-file open-at-offset event; ambiguous matches populate the workspace
  symbol panel. The app opens/focuses the owning tab and caret. Model tests cover
  exact-name filtering; a rendered two-document F12 test verifies the emitted
  target. Language-server semantic/type resolution remains. Latest complete
  gate420 with Clippy/build pass (`cross-file-definition-final-*`).
- Non-synthetic `task`/`subagent` tools now render the delegated-agent card
  without waiting for a synthetic record. `ToolPart`/`Block` carry the child
  session from tool-state metadata (`sessionId`/`sessionID`, also read from
  progress events), the tool card resolves legacy dispatches by description and
  agent, repeated snapshots merge into the first card, and a matching synthetic
  completion settles the card status while its row is absorbed. Unresolved
  cards remain visible but inert. `ToolPart` is boxed inside `PartKind` to keep
  the transcript enum compact after the extra field. Model tests cover metadata
  replay/progress, tool-card resolution and tones, and snapshot merging; a
  rendered app test verifies unresolved cards stay inert and resolved cards open
  the child panel. Clippy/build pass and 620 workspace tests pass; the engine
  flow test skipped from that run is blocked by a stalled macOS FSEvents service,
  recorded in `task-tool-cards-fsevents-blocker.txt` for a post-restart rerun
  (`task-tool-cards-final-*`).
- Post-restart validation restored the full gate for the task-tool-card slice:
  621 workspace tests pass with zero failures, and Clippy/app build exit 0
  (`task-tool-cards-post-restart-*`). The preceding FSEvents failure was
  environmental and required no code change.
- Retry notices now use Source's inline presentation: an animated spinner, the
  error message truncated to 80 characters, and a separate `Retrying · attempt
  N` line with 12/18 and 11/16 typography, muted/faint colors and one combined
  accessibility label. Missing message/attempt text falls back to `Retrying`.
  Model and rendered app tests cover truncation, fallbacks, status projection
  and the rendered spinner/message/info selectors. Latest complete gate624 with
  Clippy/build pass (`retry-notice-final-*`).
- Compaction now has a live lifecycle. `session.compaction.started/delta/ended/
  failed` parse into native events and aggregate into one compaction message
  with running/completed/failed status, accumulated or final summary text,
  completion time and failure text. A running notice survives authoritative
  history hydration; replay compaction records become completed notices.
  Assembly emits a dedicated `Compaction` block, and the UI renders Source's
  divider label variants, markdown summary and error note. Five core lifecycle/
  replay tests plus model/projection and rendered app tests cover the path.
  Latest complete gate631 with Clippy/build pass (`compaction-final-*`). Running
  shimmer and live/visual acceptance remain.
- Skill activation events now reach the transcript. `session.skill.activated`
  parses id/name/text with id and `Skill` fallbacks, live records create a
  dedicated `Skill` block, and replay `skill` records preserve skill id, name,
  text and completion time. Live skill records survive authoritative history
  hydration. The UI renders Source's `Skill activated · name`
  label, mono skill id detail and markdown body inside one session-event row.
  Event, live/replay/hydration transcript, projection and rendered selector
  tests cover the path. Latest complete gate637 with Clippy/build pass
  (`skill-transcript-final-*`). Live/visual acceptance remains.
- Running text now uses a Source-shaped shimmer instead of a static title.
  Tool summaries with a running/streaming status and the `Compacting session`
  label sweep a bright band across otherwise muted text on a 1200ms loop. GPUI
  has no gradient text fill, so each character carries a staggered color
  animation; labels longer than 80 characters use one whole-text pulse instead
  of thousands of elements. Inactive labels remain static and GPUI's reduced-
  motion handling leaves the start state in place. Model color tests and a
  rendered app test verify active/inactive tool and compaction selectors.
  Latest complete gate640 with Clippy/build pass (`shimmer-final-*`). Live and
  reduced-motion acceptance remain.
- PTY output relay now coalesces short bursts instead of publishing every
  reader chunk. A 16ms collection window, 64-chunk cap and 64 KiB byte target
  concatenate chunks in order while reducing terminal UI updates; the reader's
  16 KiB chunk remains the maximum single-batch overshoot. Exit handling is
  biased ahead of output and a 500ms post-exit deadline wins over continuous
  output, so cleanup cannot be starved. EOF still waits for the exit code, and
  the final drain delivers already-queued bytes. Tests cover order-preserving
  coalescing, exit-before-final-data, EOF/exit ordering and a continuous burst
  against the exit deadline. Latest complete gate642 with Clippy/build pass
  (`terminal-coalescing-final-*`). Live TUI/burst acceptance remains.
- Assistant streaming text now uses Source's human-paced reveal without
  mutating the authoritative transcript. `TranscriptUi` keeps a per-block
  visible prefix; the first 512 bytes appear immediately, then a 24ms timer
  advances with Source's 2/4/8/256-byte steps, character-safe boundaries and
  whitespace/punctuation snapping. Non-prefix edits, completion and session
  changes reset the prefix to full text. `project` substitutes the visible
  prefix, so copy/replay still use the complete block. Pure step/cadence tests
  and a projection test cover the reveal and final snap. Latest complete
  gate644 with Clippy/build pass (`paced-streaming-final-*`). Live
  typing-during-stream and visual acceptance remain.
- Tool cards now expand whenever they carry input or output, matching Source,
  instead of only for exec and patch. Non-shell output truncates at 6000
  characters with `… (truncated)`; shell output and patch diffs remain complete
  in bounded regions (240px shell/patch, 280px generic). Expanded bodies expose
  a focusable `Scrollable content` region, generic text wraps, shell output
  keeps horizontal flow, and patch rows retain their colored additions/deletions
  and horizontal scrolling. A model test covers the limit/boundary/Unicode
  truncation and a rendered app test checks the truncated and diff regions.
  Latest complete gate646 with Clippy/build pass (`tool-output-final-*`). Copy
  and complete visual/a11y acceptance remain.
- Expanded tool cards now expose a Source-style copy action. The button sits at
  the card's top-right, is hidden until card hover or keyboard focus, changes
  from a copy to a check icon and `Copy` to `Copied` label for two seconds, and
  remains keyboard-focusable. Shell tools copy the full `$ command` header plus
  output; other tools copy the complete output even when the display is
  truncated at 6000 characters. Model tests cover shell/general copy text and
  a rendered app test clicks the action and verifies the clipboard contents.
  Latest complete gate648 with Clippy/build pass (`tool-copy-final-*`). Live
  clipboard/visual acceptance remains.
- Generic non-shell tool cards now render Source's IN/OUT presentation. Input
  and output use separate `IN`/`OUT` labels, a divider, wrapped mono text and
  the same 6000-character display cap as Source; empty sections are omitted.
  The pair lives inside the existing focusable `Scrollable content` region and
  keeps the 280px generic height cap. Shell and patch bodies preserve their
  specialized command/output and colored diff treatments. Model tests cover
  truncation, and the rendered expansion test now checks both IN and OUT
  selectors alongside the scroll region and patch diff. Latest complete gate648
  with Clippy/build pass (`tool-io-final-*`). Live visual/a11y acceptance
  remains.
- Tool-card accessibility now matches the interactive/static distinction Source
  exposes. Expandable heads remain focusable buttons with `aria_expanded`, a
  chevron and Enter/Space activation; tools without input/output or an
  exec/patch body render as non-focusable labeled summaries instead of inert
  buttons. Expanded bodies keep the focusable `Scrollable content` region. A
  rendered app test checks pointer expansion, Enter collapse, Space reopening
  and the static disabled-card selector. Latest complete gate649 with
  Clippy/build pass (`tool-a11y-final-*`). VoiceOver and complete visual
  acceptance remain.
- W3C and Sass validation results now update native editor diagnostics as well
  as the Problems tray. Engine diagnostics map to `Validation` entries with
  W3C/Sass source labels, merge into the editor marker/banner set, clear when a
  new validation starts, and expire when the buffer revision changes. Starting
  a validation for a document now replaces its older in-flight request IDs, so
  late results cannot overwrite current markers. Pure mapping/stale-request
  tests and a rendered editor test cover the marker lifecycle. Latest complete
  gate652 with Clippy/build pass (`validation-markers-final-*`). Explicit
  cancellation, network-failure retry UX and live validator/visual acceptance
  remain.
- Validation now has explicit cancellation and failure recovery. While a W3C or
  Sass request is pending, the status menu's validation action becomes Cancel
  validation and removes the request before asking the engine to abort it.
  W3C validation selects on an already-cancelled token and returns immediately;
  Sass validation cancels the pinned validator child process. The app scopes
  summaries/failures to the active document, exposes a failed-status row and
  labels the action Retry validation after a network/compiler error. Request
  labels/stale tracking, W3C cancellation and Sass process termination are
  covered by tests. Latest complete gate655 with Clippy/build pass
  (`validation-cancel-final-*`). Live network/validator and visual acceptance
  remain.
- Local queue persistence now matches Source's v3 migration semantics. An
  unversioned or older `message-queues.json` is read, its queues move into a
  persisted quarantine map, the active queue starts empty, and the migrated
  file is immediately rewritten with `version: 3`. Quarantined prompts never
  appear in `list`, `claim`, or delivery, survive restarts, and new current
  messages resume normally. A model test covers migration, persisted schema,
  restart isolation and post-migration enqueue. Latest complete gate656 with
  Clippy/build pass (`legacy-queue-final-*`). Live imported-profile and visual
  acceptance remain.
- Model-picker favorites and visibility preferences now have rendered coverage.
  A favorite star toggles the provider-qualified persisted favorite set; the
  settings gear enters visibility mode and toggles a model's persisted hidden
  state. Provider hiding uses the same preference path and remains covered by
  the existing model visibility tests. Added stable debug selectors for the
  menu, model rows, favorite controls and settings gear. Latest complete gate657
  with Clippy/build pass (`model-picker-preferences-final-*`). Live provider
  refresh and complete visual acceptance remain.
- Validation status-menu states now have rendered coverage. With a pending
  request, the menu renders Cancel validation and clicking it clears request
  tracking and closes the menu; with a failed request, it renders Retry
  validation plus the red Failed row. Stable debug selectors distinguish the
  cancel, retry and default action variants. The test opens a real document
  editor/status popover and verifies both states. Latest complete gate658 with
  Clippy/build pass (`validation-menu-final-*`). Live network/validator and
  visual acceptance remain.
- Provider-level model visibility now has rendered coverage. The settings view
  exposes per-provider Hide/Show controls; a rendered app test toggles two
  providers, verifies both `hidden_providers` entries, returns to list mode and
  confirms the hidden provider's model row is absent while the favorited model
  remains available. Stable debug selectors identify provider visibility
  controls. Latest complete gate658 with Clippy/build pass
  (`model-provider-visibility-final-*`). Live provider refresh and complete
  visual acceptance remain.
- Local recovery queue chips now render even when the runtime inbox is empty;
  the composer checks the engine's durable queue before deciding whether to
  mount the inbox. A rendered app test seeds three v3 local queue items through
  a temporary engine profile, reorders with the pointer, keeps focus across the
  redraw and reorders again with Enter. Action buttons expose stable debug
  selectors. Latest complete gate659 with Clippy/build pass
  (`queue-chip-keyboard-final-*`). Live imported-profile/visual acceptance
  remains.
- Expanded tool-card copy now has keyboard focus/activation coverage. The copy
  control stays hidden until the copied state or keyboard focus reveals it;
  once the copied state is cleared, focus-visible styling keeps it visible.
  Pressing Enter on the focused action re-copies the full shell command and
  output. Stable `tool-copy`/`tool-copy-visible` selectors distinguish the
  states. Latest complete gate660 with Clippy/build pass
  (`tool-copy-keyboard-final-*`). Live clipboard/visual acceptance remains.
- Mixed runtime/local queue chips now have rendered interaction coverage. A
  seeded runtime inbox and v3 local recovery queue render runtime rows first,
  expose edit/steer/remove without local reorder controls, expose local reorder
  controls, and edit a runtime row back into the composer while removing the
  chip. Stable preview/action selectors support the assertions. Latest complete
  gate661 with Clippy/build pass (`queue-chip-mix-final-*`). Live visual
  acceptance remains.
- Legacy follow-up behavior now accepts Source's `immediate` value as `steer`.
  Settings load coverage verifies the alias, and the rendered model-settings
  test selects the Steer control, verifies the stored preference and keeps the
  runtime steering capability as the row gate. Latest complete gate662 with
  Clippy/build pass (`follow-up-legacy-final-*`). Live visual acceptance remains.
- Revert failure recovery now matches Source's staged dock. A stage-then-commit
  failure leaves the runtime stage in place, emits `RevertStaged`, and the
  composer shows `Undo staged` with `Commit undo` and `Discard`. Dedicated
  engine commands commit or clear the retained stage; the dock clears on
  success, discard, or session removal. Engine-flow coverage verifies retention,
  commit and discard, and a rendered test verifies both dock actions. Latest
  complete gate663 with Clippy/build pass (`staged-revert-final-*`). Live visual
  acceptance remains.
- Changed-file diff rows now carry accessibility labels. Added, removed and
  context lines announce their line number and bounded text; long content is
  capped at 160 characters plus an ellipsis. Existing selection, copy,
  intraline emphasis, wrapping and virtualized rendering are unchanged. Model
  label tests plus the existing rendered Changes suite cover the path. Latest
  complete gate664 with Clippy/build pass (`diff-a11y-final-*`). Live
  VoiceOver/platform and visual acceptance remain.
- User transcript rows now render prompt-file attachments. Optimistic local
  prompts derive names from submitted `PromptFile` entries, replayed user
  messages derive names from `file` content parts (falling back to the path
  basename), and block assembly carries the names into the rendered row.
  Source's right-aligned wrapping file cards use a generic file glyph, a
  truncated name and 220×48 geometry; empty attachment lists render nothing.
  Local-user, replay and rendered wrap/alignment tests cover the path. Latest
  complete gate667 with Clippy/build pass (`user-attachments-final-*`). Live
  drag/drop and visual acceptance remain.
- Session sidebar rows now expose Source's close action for conversations with
  an open panel. The 20×20 control appears on row hover or keyboard focus,
  carries `Close the … session` accessibility text, stops row activation, and
  closes only that panel; the conversation remains in Recents and is not
  deleted. Unpinned pin controls now use Source's hover/focus reveal while
  pinned rows stay visible. A rendered app test covers closed/open row states,
  click closure, active-panel fallback and conversation retention. Latest
  complete gate668 with Clippy/build pass (`session-close-final-*`). Live
  visual acceptance remains.
- The native editor now has bounded `Shift+Alt+F` document formatting. JSON
  validates through the real parser and is pretty-printed from the original
  token stream so key order and number spelling survive. JavaScript,
  TypeScript, CSS, SCSS and Less normalize indentation by brace depth and trim
  trailing whitespace while leaving multiline strings, templates and block
  comments byte-for-byte intact. The change is one undo step and the caret maps
  back to the same code position. JSX/TSX and HTML are skipped because text
  whitespace is meaningful; Monaco spacing normalization, range formatting and
  full worker parity remain. Unit tests plus a rendered keyboard/undo test
  cover the path. Latest complete gate674 with Clippy/build pass
  (`editor-format-final-*`). Live visual acceptance remains.
- Editor selection formatting now follows Source's `Cmd+K Cmd+F` chord. The
  selection expands to whole lines, inherits brace depth from the prefix, and
  is replaced as one undoable edit with outside text untouched. An empty
  selection formats the caret line, matching Monaco's Format Selection action.
  JSON property fragments are wrapped and validated before being reindented.
  Multiline strings, templates and block comments remain byte-for-byte intact;
  JSX/TSX and HTML remain skipped. Unit tests cover selection/caret ranges and
  nested JSON fragments; rendered tests cover both chords and undo. Latest
  complete gate679 with Clippy/build pass (`editor-range-format-final-*`).
  Monaco spacing normalization, multi-cursor range formatting and live visual
  acceptance remain.
- Multi-cursor selection formatting now applies every primary/extra cursor in one
  undo transaction. Each cursor expands to whole lines, formats against its
  surrounding brace depth, and is remapped to the formatted line/range; a
  scattered text-buffer replacement API keeps the edit atomic. Same-line
  overlapping cursors are skipped rather than merged, and unsupported
  JSX/TSX/HTML remains a no-op. A text-buffer regression covers independent
  replacement/undo ranges; a rendered editor test formats two selections and
  restores both with one undo. Latest complete gate681 with Clippy/build pass
  (`editor-multi-format-final-*`). Monaco spacing normalization and live visual
  acceptance remain.
- TypeScript/JavaScript formatting now normalizes intra-line spacing in addition
  to indentation. It enforces one space after commas/semicolons/colons and
  around assignment, equality, arrow, logical and compound-assignment
  operators; spaces brace interiors and control-flow keyword parentheses; and
  preserves regex literals, unary/optional syntax, strings, comments and
  template lines. Idempotence and preservation tests cover the tokenizer.
  Arithmetic/comparison/ternary spacing remains intentionally unnormalized to
  avoid changing generics, regex and unary semantics. Latest complete gate683
  with Clippy/build pass (`editor-spacing-final-*`). Live visual acceptance
  remains.
- Touching/overlapping multi-cursor ranges now merge before formatting, matching
  Source's range merge step. Each original cursor is mapped through the merged
  replacement using line/column remapping, so two cursors on one line survive
  as distinct selections instead of the later one being dropped. The merged
  edit still commits as one undo step. A rendered test puts two carets on one
  TypeScript line, formats once, preserves both cursors and restores the line
  with one undo. Latest complete gate684 with Clippy/build pass
  (`editor-cursor-merge-final-*`). Arithmetic/comparison/ternary spacing,
  JSX/HTML formatting and live visual acceptance remain.
- Script formatting now normalizes arithmetic and exponent operators, ternaries,
  generator stars and unary operators with token-context classification.
  `+`/`-` are binary only after an expression-ending token, `function*`/`yield*`
  keep their stars attached, ternary colons are distinguished from object/type
  colons by balanced-depth scanning, and optional `?:`/`?.`/`?.()` forms remain
  untouched. `<`/`>` are deliberately left alone because comparison/generic
  ambiguity needs a parser; whitespace already present around them is kept.
  Formatter tests cover mixed precedence, ternaries, unary plus/minus, generator
  and yield stars, optional forms and idempotence. Latest complete gate685 with
  Clippy/build pass (`editor-arithmetic-final-*`). Live visual acceptance
  remains.
- HTML document formatting now reindents tag structure by depth, handles
  void/self-closing tags, quoted attribute `>` characters and closing-tag
  outdent. `<pre>`, `<script>`, `<style>` and `<textarea>` contents are
  preserved byte-for-byte, and non-tag text lines keep their original
  whitespace. The formatter is idempotent on its output. JSX/TSX remains
  skipped because text whitespace is meaningful, and HTML selection/range
  formatting is not yet implemented. Latest complete gate686 with Clippy/build
  pass (`editor-html-final-*`). Live visual acceptance remains.
- Legacy workspace import now reads Source's v2 `locations` table and resolves
  moved bookmarks before merging them. Multi-step `movedTo` chains and nested
  subpaths are followed; when no alias exists, a recorded device/inode is
  matched against nearby directories (four ancestors, bounded directory
  scans) and relocation occurs only on a unique match. Resolved paths are
  canonicalized and re-deduplicated, and the legacy profile is never written.
  Tests cover alias plus nested relocation, relocation chains, Unix inode
  recovery and source immutability. Runtime moves after import, project-id
  recovery and ambiguous matches still fall back to the stored path. Latest
  complete gate689 with Clippy/build pass (`workspace-relocation-final-*`).
  Live visual acceptance remains.
- Workspace relocation now runs at bookmark-click time, not only during legacy
  import. The settings file persists Source-style location records (aliases plus
  device/inode identity), survives restart, and is normalized/deduplicated.
  Clicking a stale bookmark resolves it, repairs every bookmark under the moved
  root, records the alias and canonical location, persists preferences and
  opens the resolved directory; an existing session under that canonical path
  is reused. Alias repair is idempotent on repeated clicks. Tests cover import
  relocation, runtime alias/root repair, persistence round-trip and idempotence.
  Ambiguous inode matches and `.git/opencode` project-id recovery remain
  fallbacks. Latest complete gate691 with Clippy/build pass
  (`workspace-live-relocation-final-*`). Live visual acceptance remains.
- HTML selection/range formatting now shares the tag-aware state machine with
  document formatting. The prefix supplies starting tag depth, selected lines
  inherit that depth, raw `<pre>`/`<script>`/`<style>`/`<textarea>` regions
  reject range edits, and output remains idempotent. A formatter unit test and
  a rendered `Cmd+K Cmd+F`/undo test cover a nested `<div>`/`<span>` range.
  JSX/TSX remains skipped; `<`/`>` spacing and live visual acceptance remain.
  Latest complete gate693 with Clippy/build pass (`editor-html-range-final-*`).
- Workspace relocation now records each root's `.git/opencode` project ID
  alongside device/inode identity. When a moved directory has a different
  inode, nearby same-named directories are matched by project ID exactly as
  Source does, with relocation only on a unique match. The identifier survives
  settings restarts, and a test removes/recreates a project under a new inode
  with the same ID to prove recovery. Ambiguous matches still fall back to the
  stored path. Latest complete gate694 with Clippy/build pass
  (`workspace-project-recovery-final-*`). Live visual acceptance remains.
- JSX/TSX files now format with `Shift+Alt+F`. A bounded JSX
  line formatter tracks script brace/paren depth plus an open-tag stack,
  indents component and fragment lines, reindents JSX text, preserves
  attribute spacing/self-closing tags, and switches back to ordinary script
  spacing/indentation outside JSX. Inline JSX is detected only in
  expression-start positions so generic `Foo<Bar>`/`foo<T>()` are not treated
  as tags. Type assertions/ambiguous `<T>` lines, JSX range formatting and
  complex expression formatting remain bounded gaps. Tests cover components,
  fragments, text, self-closing tags, attribute preservation and a rendered
  TSX document/undo path. Latest complete gate696 with Clippy/build pass
  (`editor-jsx-final-*`). Live visual acceptance remains.
- JSX/TSX selection formatting now reuses the document formatter's JSX state.
  The prefix supplies script brace depth, parenthesis depth, open tag stack and
  literal/comment state; selected lines inherit that state and format only their
  own range. Selections starting inside multiline strings/templates/comments
  are skipped. A formatter test covers a nested `<div>`/`<span>` range and a
  rendered `Cmd+K Cmd+F`/undo test covers TSX. Complex JSX expressions,
  ambiguous `<T>` assertions/generics, `<`/`>` spacing and live visual
  acceptance remain. Latest complete gate698 with Clippy/build pass
  (`editor-jsx-range-final-*`).
- Workspace relocation now distinguishes unique, missing and ambiguous matches.
  Identity search keeps every inode/project-ID candidate instead of collapsing a
  multi-match to "not found"; runtime bookmark activation shows a bounded error
  toast listing up to three candidate paths and leaves the bookmark unchanged.
  Legacy import counts ambiguous bookmarks and startup logs the count. Tests
  build two valid project-ID candidates through directory symlinks and assert
  the outcome, unchanged bookmark and candidate message. Latest complete gate699
  with Clippy/build pass (`workspace-ambiguity-final-*`). Live visual acceptance
  remains.
- Single `<`/`>` spacing now normalizes safe comparisons without rewriting
  generic/type arguments. The classifier spaces a comparator when the left
  operand is numeric/string/regex, a closing `)`/`]`, or a literal keyword; it
  also handles a lowercase identifier against a numeric literal (including a
  signed literal). Ambiguous identifier comparisons such as `a<b` and generic
  forms such as `Array<T>` retain their original spacing. A formatter test
  covers all safe forms, unchanged generics/ambiguous comparisons and
  idempotence. Latest complete gate700 with Clippy/build pass
  (`editor-comparison-final-*`). Live visual acceptance remains.
- Identifier comparisons no longer require numeric/string operands: a `<` with no
  later `>`/`>>`/`>>>` on the line and a `>` with no earlier `<` are treated as
  comparisons, so `a<b` and `a>b` normalize while angle-paired generic/type
  forms such as `Array<T>` and `foo<T>()` stay untouched. Safe numeric/string/
  literal heuristics still cover paired comparison shapes. The formatter test now
  includes both unpaired directions plus unchanged generics. Latest complete
  gate700 with Clippy/build pass
  (`editor-comparison-identifiers-final-*`). Ambiguous angle-paired comparisons
  and live visual acceptance remain.
- Paired `<`/`>` spacing now recognizes comparison chains. Angle delimiters are
  paired with a stack (including `>>`/`>>>` generic closers), then a pair is
  spaced as a comparison when both sides are value operands, the left side is
  not an upper-case type head, the right side is not a type suffix/call/index
  marker (`(`, `.`, `[`, `as`, `extends`, etc.) and the contents do not contain
  type indicators (`|`, `&`, `keyof`, `=>`, `:`, `?`). This covers `a<b>c` and
  `a<b && c>d` while leaving `Array<T>`, `foo<T>()`, `Array<T> as const` and
  nested `>>` generics untouched. Latest complete gate700 with Clippy/build
  pass (`editor-comparison-pairs-final-*`). Residual type-vs-comparison
  ambiguity and live visual acceptance remain.
- Paired comparison classification no longer rejects contents containing type
  punctuation when both sides are value operands. Lower-case heads followed by
  another value now normalize chains with bitwise or ternary contents
  (`a<b | c>d`, `a<b ? c : d>e`) while upper-case type heads, type suffixes
  (`as`/`extends`/`instanceof`/`in`/`of`), calls/indexing and `>>` generics stay
  untouched. Latest complete gate700 with Clippy/build pass
  (`editor-comparison-pairs2-final-*`). Residual type-vs-comparison ambiguity
  and live visual acceptance remain.
- Comparison operand classification now excludes expression-introducing keywords,
  so TypeScript type assertions such as `return <Foo>bar`, `await <Foo>bar` and
  `const x = <Foo>bar` keep their angle spacing instead of being mistaken for
  chained comparisons. Ordinary identifier comparisons and paired chains still
  normalize. A formatter test covers assertion positions after `return` and
  assignment. Latest complete gate701 with Clippy/build pass
  (`editor-type-assertion-final-*`). Complex JSX-expression and `.tsx` `<T>`
  ambiguity plus live visual acceptance remain.
- `.tsx` generic parameter lists are no longer mistaken for JSX tags.
  Single-line `<T,>`, `<T extends U>` and defaulted `<T = U>` forms followed by
  an argument list are classified as TypeScript, and their comma/angle spacing
  is preserved without opening a JSX stack. JSX attributes containing `=` or
  `extends` strings remain tags. A formatter test covers comma, constraint and
  default generic arrows plus idempotence. Latest complete gate702 with
  Clippy/build pass (`editor-tsx-generics-final-*`). Multi-line generic
  parameters, complex JSX expressions and live visual acceptance remain.
- Multi-line `.tsx` generic parameter lists are now tracked across lines. A
  `= <`/parameter line with comma, `extends` or default markers enters generic
  mode, subsequent lines indent by angle depth (capped at 32 lines), the closing
  `>` line returns to script depth, and a following `(` establishes arrow
  argument indentation until it closes. Start-depth and full arrow output tests
  cover comma parameters, constraint/default markers and idempotence. Latest
  complete gate704 with Clippy/build pass (`editor-tsx-multiline-final-*`).
  Complex JSX expressions, residual type-vs-comparison ambiguity and live
  visual acceptance remain.
- JSX expression containers now participate in indentation. Balanced `{...}`
  depth is tracked on JSX lines (ignoring quoted strings/comments), nested tags
  indent one level while an expression is open, and lines whose closing braces
  exceed opens outdent before formatting. This handles multiline
  `{items.map(... => (<span>...`)` blocks and closes them correctly. A formatter
  test covers nested tag/container indentation and idempotence. Latest complete
  gate705 with Clippy/build pass (`editor-jsx-expressions-final-*`). Deeply
  nested JSX expression/parenthesis alignment, residual type-vs-comparison
  ambiguity and live visual acceptance remain.
- Multi-file launch/drop delivery no longer overwrites a single pending file.
  Workspace files accumulate in an ordered tab-request vector, external files
  accumulate in an ordered standalone-tab vector, and render drains both in
  order. A pure classifier centralizes canonicalization, workspace-membership
  and relative-path normalization; Explorer-created files feed the same queue.
  A unit test covers two nested workspace files followed by an external file
  and preserves order/membership. Latest complete gate706 with Clippy/build pass
  (`multi-file-launch-final-*`). Live second-instance routing, mixed
  directory/file activation and physical OS drag acceptance remain.
- JSX expression-container alignment now handles multiline ternary branches.
  A line beginning with `)` while an expression container is open outdents one
  level, so `) : (` aligns with the `{condition ? (` line and the following
  branch tag indents correctly; the closing `)}` line outdents via brace depth.
  A formatter test covers both branches and idempotence. Latest complete gate707
  with Clippy/build pass (`editor-jsx-ternary-final-*`). Nested expression
  alignment beyond one container level, residual type-vs-comparison ambiguity
  and live visual acceptance remain.
- Nested JSX ternary branches now accumulate indentation. A parenthesis stack
  records whether each `(` followed `?`/`:`, exposes only the depth beyond the
  first branch as extra indentation, and decrements when that group closes.
  Nested ternary branches therefore step in one more level per branch while
  arrow/call/grouping parentheses do not add spurious depth. A formatter test
  covers a two-level ternary with both else branches and idempotence. Latest
  complete gate708 with Clippy/build pass
  (`editor-jsx-nested-ternary-final-*`). Other JSX expression alignment,
  residual type-vs-comparison ambiguity and live visual acceptance remain.
- JSX nested arrow continuations now accumulate alignment. The parenthesis stack
  marks a `(` following `?`, `:`, `>`, `&` or `|` as a continuation group, and
  only depth beyond the first group adds indentation. Nested arrow callbacks
  such as `items.map(... => (` / `item.children.map(... => (` therefore step in
  one extra level each and their closing parentheses return to the outer level.
  A formatter test covers the two nested arrows, both closing parentheses and
  idempotence. Latest complete gate709 with Clippy/build pass
  (`editor-jsx-continuation-final-*`). Deeper JSX expression alignment, residual
  type-vs-comparison ambiguity and live visual acceptance remain.
- Agent tool calls now seed pre-edit file baselines. `session.tool.called`
  inputs are walked for `filePath`, `file_path` and `path` keys (nested objects
  and arrays included, HTTP values excluded, first occurrence retained). Each
  readable workspace file is captured under the watcher size/confinement rules
  only when no baseline exists, so a later non-Git agent edit becomes a
  before/after diff instead of `Observed`; existing/Git baselines and
  outside-root or unsafe paths are left alone. Unit tests cover extraction,
  dedupe, relative/absolute capture, preservation and outside-root rejection.
  Latest complete gate713 with Clippy/build pass
  (`agent-tool-baselines-final-*`). Live tool-event timing, full Changes
  projection and visual acceptance remain.
- Guest new-window requests now follow Source's `safeExternalUrl` policy. The
  browser hands the OS only a credential-free HTTPS URL with a host, using the
  normalized href; HTTP, embedded credentials, `javascript:`, `data:`, `file:`,
  `mailto:`, `about:` and malformed values are denied inside the guest. Tests
  cover accepted paths/ports and every rejected scheme/credential case. Latest
  complete gate714 with Clippy/build pass
  (`browser-external-https-final-*`). Downloads remain denied; popup edge cases
  and live/visual acceptance remain.
- Inactive tiled panels now show and answer their own permission prompts. The
  permission-card renderer is scoped by session and element namespace, so each
  tile gets Allow/Always/Deny actions without sharing ids or state; clicking a
  card or action activates that panel, matching Source's panel-wide mouse-down
  focus. Approve mode now auto-replies for any session present in the open panel
  registry (plus the active session), rather than only the focused tile, while
  closed warm sessions remain ask-based. A rendered two-panel test asserts the
  inactive card/actions exist, the active tile does not borrow them, and Deny
  brings the waiting panel forward. Latest complete gate715 with Clippy/build
  pass (`panel-permissions-final-*`). Per-tile forms, attachments, references,
  slash menus, rich tool actions and live/visual acceptance remain.
- Child-session drill-down now matches Source's `reopenSession`. Delegated-agent
  cards focus an already-visible child; otherwise they replace the focused panel
  in the same slot instead of appending a panel. Replacement preserves the
  slot's position and width. Child sessions render the reference's
  agent-header back control (a compact arrow in tiled headers) that returns to
  the parent with the same focus-or-replace rule, while root sessions render
  none. A rendered test covers card drill-down, parent-panel replacement, the
  back button and its absence on roots; a panel test covers width preservation
  and no-duplicate focusing. Latest complete gate717 with Clippy/build pass
  (`child-back-navigation-final-*`). Live/paired drill-down appearance and the
  remainder of child transcript parity remain.
- Source pins now migrate from the Electron profile. Source stores
  `vela.pinnedSessions` only in renderer localStorage, so the importer reads
  Chromium's LevelDB write-ahead `.log` framing and `WriteBatch` entries,
  decodes Latin-1 and UTF-16 values, applies later deletes and reassembles
  fragmented records. A bounded raw scan of compacted `.ldb` literals covers
  profiles whose log has already been compacted. A persisted one-time marker
  makes native pin state authoritative after the scan, so an unpin is never
  resurrected; the source profile is only read. Unit tests cover newest-batch
  ordering, both value encodings, deletes, fragmentation, table fallback,
  malformed records and a missing store; migration tests cover the one-time
  merge and the no-resurrection contract. Latest complete gate724 with
  Clippy/build pass (`source-pin-import-final-*`). A live real-profile restart
  and visual acceptance remain.
- The shared native text input now blinks its caret on Source's browser
  cadence. The 530 ms phase starts only when the input is focused in an active
  window, stops and restores a steady visible caret on blur or deactivation,
  and restarts on any key press, pointer caret move or programmatic
  `set_text`. Selection painting is unchanged and still suppresses the caret.
  A deterministic timer test covers the blink, blur and refocus transitions.
  Latest complete gate725 with Clippy/build pass (`caret-blink-final-*`). Live
  caret appearance remains queued with the composer matrix.
- Markdown table cells now render Source's mixed inline content. A cell that
  contains images is split into ordered text/image parts, so bold, italic,
  code and link runs before/after each image stay intact instead of collapsing
  to alt text. Block images stack vertically through the shared markdown image
  loader at the existing 200px cap. `table_widths` now resolves image content
  and includes the widest fitted image in each column's minimum and maximum, so
  narrow tables scroll horizontally instead of clipping the image. A rendered
  test asserts styled text/image presence, before/after text, containment,
  resolved image sizing and intrinsic-width overflow; all markdown tests pass.
  Latest complete gate726 with Clippy/build pass
  (`markdown-table-images-final-*`). Reflow during asynchronous image loads,
  platform table-header accessibility and live/visual acceptance remain.
- JavaScript/TypeScript spacing now separates shift operators from generic
  angle closers. `<<`, `<<=`, `>>`, `>>>`, `>>=` and `>>>=` use the existing
  comparison heuristic against the outermost closed `<`, so mixed expressions
  such as `a < b >> c` gain spaces while `Map<string, Array<number>>()` and
  nested generic types stay tight. Shift tokens with no generic opens are
  always treated as operators. A formatter test covers all six shift forms,
  mixed comparison/shift expressions, generic calls/types and idempotence.
  Latest complete gate727 with Clippy/build pass
  (`format-shift-operators-final-*`). The remaining TypeScript
  generic-instantiation vs comparison cases (tokens such as `[` or `!` after
  `>`) and live visual acceptance remain.
- Single bitwise operators now space consistently with their compound forms
  and TypeScript type syntax. `&`, `|` and `^` are binary in expressions
  (`a & b`, `a | b`, `a ^ b`) and in union/intersection types (`A | B`,
  `A & B`, `Record<A | B, U & B>`), while `&=`, `|=` and `^=` keep their
  existing spacing. A formatter test covers all six forms and idempotence.
  Latest complete gate728 with Clippy/build pass
  (`format-operators-final-*`). The remaining generic-instantiation vs
  comparison cases and live visual acceptance remain.
- Generic instantiation vs comparison now matches TypeScript's parser. When
  `>` is followed by an expression-start token such as `[`, `!`, unary
  `+`/`-`/`~`/`++`/`--`, `typeof`/`void`/`await`/`new`, an object/array
  literal or a primitive literal, the angle pair spaces as a comparison
  (`a < b > !c`, `a < b > [0]`). Calls, `as`/`satisfies`, `&&`/`||`/`??`/`**`,
  conditional and `?.`/`.` positions remain instantiation expressions and stay
  tight. A type-context guard keeps `type T = foo<bar>[]`, indexed type
  aliases, interface heritage and annotated declarations tight while object
  literal values and ternary branches still gain comparison spacing. A
  formatter test covers these cases and idempotence. Latest complete gate729
  with Clippy/build pass (`format-generic-instantiation-final-*`). Lowercase
  generic type names in parameter/default positions and live visual acceptance
  remain.
- Lowercase generic types in parameter, default and return positions now stay
  tight. Type context is recognized from unmatched parameter parentheses
  (including callbacks nested in object literals), generic-parameter
  `=`/`extends` clauses, return-type colons after `)`, and top-level annotation
  colons. Default values after an annotation `=` reset back to expression
  context, so `(x: T = a<b>[c]) => a<b>[d]` spaces both comparisons while
  `(x: foo<bar>[])`, `function f<T = foo<bar>[]>()`, generic arrows, class
  defaults and `(x): foo<bar>[]` stay tight. A formatter test covers these
  cases and idempotence. Latest complete gate730 with Clippy/build pass
  (`format-generic-types-final-*`). Lowercase generic types in class property
  declarations and live visual acceptance remained at that gate.
- Lowercase generic types in class property declarations now stay tight. The
  detector tracks class-body brace depth and generic parameter lists, handles
  optional `?:` fields, nested object/function property types, method return
  signatures, initializers and method/static blocks, and nested classes. A
  property initializer or method/static block resets to expression context, so
  `class A { x?: foo<bar>[c]; v: () => foo<bar>[]; }` stays tight while
  `x = a<b>[c]` and method-body comparisons still format. The focused class
  test covers optional fields, nested object/function types, initializers,
  method bodies, static blocks, nested classes and idempotence; all 31
  formatter tests pass. Latest complete gate731 with Clippy/build pass
  (`format-class-properties-final-*`). Live visual acceptance and the remaining
  worker-derived language tooling remain.
- Markdown table header and body cells now expose accessible labels derived
  from their rendered text, including image alt text, with positional fallbacks
  for empty cells. Header cells retain the column-header role, body cells the
  cell role, rows carry row indices and tables carry row/column counts. A
  focused test covers the role/label projection (mixed text/code/emphasis,
  image alt text, empty and untitled-image fallbacks) and a rendered test checks
  header/body row and cell presence. All 19 markdown tests pass. Latest
  complete gate732 with Clippy/build pass (`markdown-table-a11y-final-*`).
  Platform header mapping, VoiceOver and live/visual acceptance remain.
- Browser downloads now have an explicit policy. Credential-free HTTP/HTTPS
  downloads save to the OS Downloads directory; suggested filenames are
  sanitized, URL fallbacks are percent-decoded and Unicode-safe, traversal and
  control characters are stripped, and collisions get numbered suffixes.
  File/data/javascript/blob/about/custom and credentialed URLs are denied
  before a path is written. Unit tests cover accepted/rejected schemes,
  filename preference, Unicode fallback, traversal stripping, empty-name
  fallback and collision numbering. Latest complete gate735 with Clippy/build
  pass (`browser-downloads-policy-final-*`). In-app download feedback/shelf,
  popup edge cases and live/visual acceptance remained at that gate.
- Browser download completions now surface as app toasts. A completion event
  carries the URL, optional saved path and success flag; the UI prefers the
  saved filename, falls back to the percent-decoded URL name, and uses
  `Download complete`/`Download failed` when no name is available. Successes
  use the normal toast styling and failures use error styling through the
  shared toast surface. A focused test covers saved-path, URL-derived Unicode,
  anonymous success and named/anonymous failure text. Latest complete gate736
  with Clippy/build pass (`browser-download-feedback-final-*`). Richer download
  management, popup edge cases and live/visual acceptance remain.
- Browser downloads now have a persistent in-app history and shelf. Completed
  and failed entries are stored newest-first in `browser-downloads.json` under
  the app data directory, loaded at startup, capped at 20 and ignored when
  corrupt. The toolbar exposes a download count; opening it renders a shelf
  with up to three recent rows (status icon/text and filename), an older-count
  line, and Clear/Close actions. The webview bounds shrink while the shelf is
  open. Module tests cover save/load, cap, corruption, name selection; a
  rendered test covers opening, listing both statuses and clearing the tray.
  Latest complete gate741 with Clippy/build pass (`browser-download-tray-final-*`).
  Popup edge cases and live/visual acceptance remain.
- The download shelf now supports per-entry removal in addition to Clear and
  Close. Removing a row updates the persisted history immediately; removing the
  last entry closes the shelf and hides the toolbar count. Module tests cover
  remove-by-id/unknown-id behavior; the rendered tray test removes one entry,
  verifies the other remains, then clears the history. Latest complete gate759
  with Clippy/build pass (`browser-download-tray-actions-final-*`). Live/visual
  acceptance remains.
- Browser popup handling now has an explicit `PopupAction::{External, Deny}`
  boundary used by the native new-window handler. Tests cover whitespace
  trimming, scheme/host normalization, default/non-default ports, IPv6, empty
  usernames, credentialed URLs, empty/malformed/relative URLs and every denied
  scheme; only normalized credential-free HTTPS is handed to the OS and every
  popup is denied as a native window. Latest complete gate742 with Clippy/build
  pass (`browser-popup-policy-final-*`). Live/visual acceptance remains.
- Browser guest permission isolation now covers sensitive Web APIs that WebKit
  exposes even when the native media-capture handler denies capture. A
  document-start script runs in every frame and reports geolocation,
  notifications, media/display capture, device enumeration, clipboard
  read/write, permissions queries and USB/Bluetooth/serial/HID/MIDI/share/
  wake-lock as unavailable or denied. The native permission handler remains a
  second layer. A focused test asserts coverage of every API; the script was
  syntax-checked with Node. Latest complete gate743 with Clippy/build pass
  (`browser-permission-isolation-final-*`). Full platform permission/live
  isolation acceptance remains.
- Browser toolbar controls are now accessible real buttons rather than
  mouse-only divs. Back, forward, reload/stop, downloads and close have roles,
  labels, focus handles and tab stops; pointer activation focuses the control,
  and Enter/Space activate it. A focused rendered test drives pointer and both
  keyboard activations. Latest complete gate745 with Clippy/build pass
  (`browser-toolbar-a11y-final-*`). Paired visual acceptance remains.
- Markdown code fences now have native syntax coloring. A bounded tokenizer
  covers JSON, JavaScript/TypeScript/JSX/TSX, CSS/SCSS/Less, HTML/XML, Python,
  Bash, YAML, SQL, Diff and Markdown, marking keywords, strings, numbers,
  comments, functions, properties, tags/attributes and diff lines with
  GitHub-light/dark-style colors. Preformatted layout paints complete gap-free
  runs; tabs, unknown languages and plain fences keep the original style.
  Tests cover aliases, token spans, UTF-8 boundaries, markdown run coverage and
  the tab/unknown fallbacks. Latest complete gate750 with Clippy/build pass
  (`markdown-code-highlight-final-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- Inline code now uses Source's path/URL classification. HTTP(S) spans are
  recognized as URLs; filesystem-looking spans (slashes, `./`/`../`, common
  extensions or known filenames) are paths and paint with the accent color,
  while plain code keeps normal ink. Single-slash commands, whitespace,
  punctuation/operator spans, non-HTTP schemes and version-like text stay
  uncolored. Classifier edge cases and run colors are tested. Latest complete
  gate752 with Clippy/build pass (`markdown-inline-code-final-*`). Full
  Linguist extension/filename coverage and live visual acceptance remain.
- Markdown math now renders natively. Inline math is shaped with the math font
  and display math renders as a centered, labeled block. A bounded TeX subset
  converts Greek/operator symbols, superscripts/subscripts, fractions, roots,
  text/style groups and left/right delimiters to readable Unicode, preserving
  unknown commands instead of dropping content. Parser edge cases, inline font
  selection and display-block rendering are tested. Latest complete gate756
  with Clippy/build pass (`markdown-math-final-*`). Full KaTeX layout,
  accessibility and live visual acceptance remain.
- Math rendering now covers common matrices, accents and TeX spacing:
  `pmatrix`/`bmatrix`/`vmatrix`/`Bmatrix`/`cases`/`array` render as bounded
  row/cell text, accent commands attach combining marks, and `~`, `\,`, `\:`,
  `\quad`/`\qquad`/`\hspace` produce readable spaces. Inline and display math
  labels expose rendered text plus a `TeX: …` source annotation. Parser,
  inline-source and display-block tests pass. Latest complete gate758 with
  Clippy/build pass (`markdown-math-annotations-final-*`). Full KaTeX layout
  and live visual acceptance remain.
- Provider usage snapshot selection now matches the documented “newest usable
  snapshot” rule. All readable candidates are parsed and the highest valid
  `generatedAt` wins regardless of XDG/macOS path order; malformed, stale and
  empty candidates never displace an older valid file, and ties keep the first
  candidate. Fixtures cover both orderings and malformed/empty fallback. Latest
  complete gate757 with Clippy/build pass
  (`provider-usage-newest-final-*`). Live OAuth/key fetches and paired visual
  acceptance remain blocked on credentials/live validation.
- Vite preview inventory ordering now follows Source's `localeCompare` instead
  of byte order. macOS uses Foundation's localized case-insensitive comparison;
  other platforms fall back to Unicode case folding. Case variants sort
  lowercase-first, directories then entries, and repeated sorting is stable.
  A preview test covers mixed-case directories/entries, locale ordering,
  case tie-breaks and idempotence. Latest complete gate737 with Clippy/build
  pass (`preview-locale-sort-final-*`). Relocation/release validation and
  visual acceptance remain.
- The same locale-aware ordering now drives recursive HTML entry search.
  Source sorts directory entries with `localeCompare` before choosing the
  shallowest fallback HTML page, so uppercase filenames/directories no longer
  win by byte order. Native uses the shared localized/case-folded comparator
  for same-depth files and directory traversal. A focused test covers
  mixed-case root files and same-depth directories. Latest complete gate738
  with Clippy/build pass (`preview-entry-locale-final-*`). Relocation/release
  validation and visual acceptance remain.
- Vite preview stop menus now validate relocation context when labeling
  servers: entries under the current workspace stay relative, root entries omit
  the directory, and stale servers under an old or sibling root show their full
  directory instead of appearing workspace-relative. Child-exit and Stop port
  release remain covered by lifecycle tests. The focused label test covers
  root/relative/sibling/old-root paths and trailing separators. Latest complete
  gate744 with Clippy/build pass (`preview-relocation-final-*`). Live preview
  acceptance remains.
- Markdown links and image links now route through Source's `safeExternalUrl`
  policy instead of opening arbitrary schemes through the OS. Only
  credential-free HTTPS URLs with a host are handed off; HTTP, credentialed,
  `javascript:`/`data:`/`file:`/`mailto:`/custom and malformed URLs remain
  in-app. The policy test covers accepted/normalized HTTPS and every blocked
  case. Latest complete gate760 with Clippy/build pass
  (`markdown-link-policy-final-*`). Live visual acceptance remains.
- The download shelf now offers Show for entries with an absolute saved path.
  It invokes the platform file-manager reveal command (`open -R` on macOS,
  Explorer `/select` on Windows, `xdg-open` on the parent elsewhere); relative
  or missing paths expose no action, and a spawn failure raises an error toast.
  Command construction is unit-tested, and the rendered tray test verifies
  Show appears only for path-backed entries. Latest complete gate761 with
  Clippy/build pass (`browser-download-reveal-final-*`). Live/visual acceptance
  remains.
- Code-fence grammar fixtures exposed and fixed several tokenization gaps:
  numeric scanning now stops at `a-1` while accepting exponent signs, JavaScript
  regex literals are recognized in expression positions, Python identifiers no
  longer absorb hyphens, Bash handles `${…}`/`$#`/`$?` variables, CSS only marks
  properties inside declaration blocks (not `a:hover` selectors), and HTML
  attributes/tags keep hyphenated names. Fixture tests assert token kinds and
  UTF-8 span integrity. Latest complete gate762 with Clippy/build pass
  (`markdown-grammar-fixtures-final-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- Code-fence grammar coverage now includes JavaScript template interpolation
  (inner expressions use the JS grammar while literal chunks stay strings),
  CSS custom properties (marked only on declarations, not `var()` references)
  and embedded `<script>`/`<style>` content highlighted with the JS/CSS
  grammars. Fixture assertions cover all three plus span integrity. Latest
  complete gate762 with Clippy/build pass (`markdown-grammar-embedded-final-*`).
  Full Shiki grammar parity and live visual acceptance remain.
- YAML block scalars and SQL quoted identifiers are now covered by the fence
  grammar. `|`/`>` headers (with indentation/chomping modifiers) start a scalar
  whose indented continuation lines stay one string token until the indentation
  dedents, so keys, numbers and `#` inside are not re-tokenized. SQL
  double-quoted, backtick and bracket identifiers are property tokens and honor
  doubled-delimiter escapes, while single-quoted strings remain string tokens.
  A focused fixture test covers headers/modifiers, dedent, literal content and
  each identifier form plus span integrity. Latest complete gate763 with
  Clippy/build pass (`markdown-yaml-sql-final-*`). Full Shiki grammar parity and
  live visual acceptance remain.
- The session-usage popup now matches Source's provider section more closely.
  A host-owned, labeled refresh control sits in the Provider usage header,
  triggers a new bridge-snapshot request and is disabled while loading. The
  empty state uses Source's "No providers connected…" wording, and the token
  breakdown adds the Cache total with its rounded share of total tokens. The
  header refresh control and cache formatting have focused rendered/pure tests.
  Latest complete gate765 with Clippy/build pass
  (`provider-usage-refresh-final-*`). Live credential refresh and paired visual
  acceptance remain.
- Engine crash recovery now re-resolves the OpenCode service instead of
  retrying a stale endpoint indefinitely. The SSE reader emits a disconnect
  marker when an opened stream ends (suppressed during deliberate cancellation);
  the engine marks the connection as connecting, polls the configured service
  file, creates a fresh client and event stream, and rehydrates sessions,
  permissions and open views. A later `server.connected` restores Ready even if
  the original transport recovered first. The engine-flow fixture moves the
  service to a new port and then triggers a same-endpoint outage, asserting
  rehydration calls and exactly one replacement stream per outage. Latest
  complete gate766 with Clippy/build pass (`engine-reconnect-final-*`). Renderer
  reload UX and live daemon/restart acceptance remained at that gate.
- Native now mirrors Source's renderer reload after a backend crash. A recovery
  banner appears for Connecting/Disconnected/Error states with a Reload action;
  Reload re-reads sessions and clears/rebuilds UI-owned catalogs, provider
  usage, forms/inbox/command references, panels and file views from the engine.
  After a disconnect, the first Ready transition automatically performs the same
  reload and consumes the recovery flag, preserving engine transcripts and
  drafts. Rendered tests cover banner visibility by connection state and the
  Reload state rebuild. Latest complete gate768 with Clippy/build pass
  (`connection-recovery-ui-final-*`). Live daemon/window crash acceptance
  remains.
- Markdown fences now follow Source's settle-then-highlight behavior. While a
  transcript block is still streaming, the fence renders on the plain code
  surface; once the block settles, the bounded tokenizer builds gap-free styled
  runs. This avoids re-tokenizing the growing fence on every chunk while keeping
  wrap/copy controls and layout unchanged. The code-fence run test covers the
  streaming fallback alongside the existing highlight cases. Latest complete
  gate768 with Clippy/build pass (`markdown-streaming-fence-final-*`). Full
  Shiki grammar parity and live visual acceptance remain.
- Generic tool results now retain file items from their `content` arrays. Replay
  parsing and live `session.tool.*` completion events produce ordered
  URI/name/mime `ToolFile` records on the tool block; expanded cards render a
  labeled FILES section with a file glyph, name fallback to URI, MIME text and
  stable per-file selectors. Only credential-free HTTPS file links are handed to
  the OS, matching Source's ExternalLink policy; other URIs remain labeled but
  inert. Parser/live-event and rendered file-row tests cover the path. Latest
  complete gate769 with Clippy/build pass (`tool-content-files-final-*`). Remaining
  tool metadata/duration presentation and live visual acceptance stay open.
- Tool metadata now includes Source's `deepseek.subCalls`. Replay tool parts
  and live `session.tool.progress` metadata parse nested id/title/detail/
  status/input/output records into ordered `ToolSubCall`s. Expanded tool cards
  render a `TOOL CALLS · N` section. Each nested row is collapsible like
  Source's recursive tool card: title, optional detail and Done/Failed/Running
  status stay visible, and expanding reveals IN/OUT payloads in a bounded
  nested region. Status mapping and input/inputValue fallbacks are parser-tested;
  rendered fixtures assert collapsed-by-default behavior and click-to-expand
  payloads. Latest complete gate769 with Clippy/build pass
  (`tool-subcall-recursive-final-*`). Deeper recursive metadata trees and live
  visual acceptance remain open.
- Nested tool metadata now recurses fully. Sub-call records carry their own
  `content` file items and `subCalls` children (direct or under
  `metadata.deepseek`), parsed to a bounded depth of eight to reject pathological
  metadata. Expanded nested cards render their own FILES rows and recursively
  nested TOOL CALLS cards, each independently collapsible with IN/OUT payloads.
  Parser/depth tests and rendered coverage for outer/nested expansion and nested
  files pass. Latest complete gate770 with Clippy/build pass
  (`tool-subcall-deep-final-*`). Remaining duration presentation and live visual
  acceptance stay open.
- Completed tool cards now render a status footer with Source's Success/Failed
  language and check/close glyph, plus the recorded elapsed duration using the
  `Ns` / `Nm Ns` format. Running tools keep the shimmer and omit the footer;
  nested cards retain their own status rows. Shell tools also fall back to
  `metadata.command` and `metadata.output` when input/output fields are absent,
  matching Source's shell metadata presentation. Duration-format tests, the
  rendered footer assertion and shell metadata fallback tests pass. Latest
  complete gate772 with Clippy/build pass (`tool-duration-final-*`). Live visual
  acceptance remains.
- The bounded math renderer now covers more common KaTeX constructs:
  aligned/gathered/split environments, `\binom`, log/trig/limit operators,
  optional `\sqrt[n]` indices, style switches, phantom groups and empty
  `\left.`/`\right.` delimiters. Rendering remains Unicode text rather than
  full KaTeX layout, with readable fallbacks for unsupported commands. The math
  parser test covers each added construct and preserves existing matrices,
  accents and spacing behavior. Latest complete gate773 with Clippy/build pass
  (`markdown-math-operators-final-*`). Full KaTeX layout/accessibility and live
  visual acceptance remain.
- Answer footers now match Source's `TextPart` metadata row. Transcript blocks
  carry the message agent id and model id, stamped during part-block assembly so
  replay and live updates agree. The footer joins a capitalized agent label, a
  catalog-resolved model name (falling back to the id) and turn duration with
  ` · `, and suppresses the row when all fields are absent. Pure formatting,
  block-stamping and rendered present/absent tests pass. Latest complete gate776
  with Clippy/build pass (`answer-footer-meta-final-*`). Live visual acceptance
  remains.
- The browser download shelf's Show, Remove, Clear and Close actions are now
  keyboard-accessible: each is a focusable tab stop with an Enter/Space
  activation path in addition to pointer clicks. A rendered test focuses the
  Remove handle, removes an entry with Enter, then focuses Clear and empties the
  shelf with Space. Latest complete gate777 with Clippy/build pass
  (`browser-download-actions-a11y-final-*`). Popup edge cases and live/visual
  acceptance remain.
- Code-fence grammar coverage now includes CSS-family variables and document
  literals: SCSS `$variables` and Less `@variables` receive property coloring
  (known at-rules such as `@media`/`@mixin` stay keywords, CSS custom properties
  remain declarations), HTML character entities (`&amp;`, `&copy;`, numeric
  forms) become string tokens in text and tag positions, and YAML anchors,
  aliases, tags and merge keys receive property/keyword tokens while block
  scalar bodies stay literal. A focused fixture test covers each token kind and
  span integrity, including anchor suppression inside `|` scalars. Latest
  complete gate778 with Clippy/build pass
  (`markdown-grammar-vars-entities-anchors-*`). Full Shiki grammar parity and
  live visual acceptance remain.
- Python code fences now highlight decorators and interpolated f-strings.
  Decorator names receive function coloring, and `f`/`rf`/`fr` string prefixes
  keep their literal text as string tokens while `{...}` expressions are
  recursively tokenized with the Python grammar (nested braces, quoted strings
  and comments included). A fixture test covers decorator `@app.route`,
  f-string expressions and raw-prefixed f-strings with span integrity. Latest
  complete gate779 with Clippy/build pass
  (`markdown-grammar-python-fstrings-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- Editor documents now preserve their original EOL. CRLF files load into
  LF-normalized buffers so navigation, edits, undo and formatting operate
  uniformly; saves encode LF back to CRLF while compare-and-write expectations
  retain the raw on-disk text. Clean external reloads re-detect CRLF/LF and
  switch later saves to the new EOL. A document test exercises normalized
  editing, exact CRLF output, raw expectations and LF re-detection. Latest
  complete gate780 with Clippy/build pass (`editor-crlf-final-*`). Live visual
  acceptance remains.
- Editor line numbers now follow Monaco's active-line presentation. The logical
  line containing the caret receives a brighter medium-weight number and a
  subtle gutter background, while other numbers remain faint. A rendered test
  moves the caret between lines and asserts the active-number selector follows.
  Latest complete gate781 with Clippy/build pass
  (`editor-active-line-final-*`). Live visual acceptance remains.
- Editor brackets now match Monaco's bracket-pair colorization. A per-revision
  document scan records each structural bracket's nesting depth after masking
  strings and comments; the close side of a pair shares its opener's depth.
  Rendering splits each tab-expanded display line into default and bracket runs
  with repeating gold/orchid/blue colors, while the existing matching-bracket
  background highlight still follows the caret. Pure depth/run tests cover
  alternating levels, string/comment exclusion, tab mapping and line clipping;
  rendered coverage asserts the structural set and depths as the caret moves.
  Latest complete gate783 with Clippy/build pass (`editor-bracket-colors-final-*`).
  Live visual acceptance remains.
- JavaScript/TypeScript fence highlighting now covers shebang lines, decorators,
  private fields and modern TypeScript modifier keywords. `#!` first lines are
  comment tokens, `@name` decorators receive function coloring, `#field`
  declarations receive property coloring, and `abstract`, `accessor`, `asserts`,
  `infer`, `keyof`, `override`, `using` and related modifiers are keywords.
  A fixture test covers each token kind plus span integrity. Latest complete
  gate784 with Clippy/build pass (`markdown-grammar-script-final-*`). Full Shiki
  grammar parity and live visual acceptance remain.
- CSS-family fence highlighting now covers selectors and declaration values.
  Class/id selectors and known pseudo classes/elements receive tag colors,
  attribute-selector names receive attribute colors, and declaration state is
  tracked so values can color hex colors, `url(...)` function names and
  `!important` without mistaking nested element selectors or URL schemes for
  properties. A fixture test covers selectors, pseudo selectors, attributes,
  hex values, URLs, units and `!important`, plus span integrity. Latest complete
  gate785 with Clippy/build pass (`markdown-grammar-css-selectors-final-*`).
  Full Shiki grammar parity and live visual acceptance remain.
- Shell fences now highlight heredoc bodies, ANSI-C quoted strings,
  assignment names and function definitions. `<<`/`<<-` headers accept quoted
  or bare delimiters, body lines stay one string span through the terminator,
  `$'...'`/`$"..."` strings are strings rather than split fragments, `NAME=`
  assignments and `name()` definitions receive property/function colors, and
  `#` only starts a comment at a word boundary. A fixture test covers each
  shape plus a hash inside a word and span integrity. Latest complete gate786
  with Clippy/build pass (`markdown-grammar-bash-final-*`). Full Shiki grammar
  parity and live visual acceptance remain.
- SQL fences now cover dollar-quoted strings, parameters and richer symbols.
  `$tag$...$tag$` and `$$...$$` bodies are single string tokens; `$1`, `:name`,
  `@account` and `?` placeholders receive property colors; identifiers followed
  by `(` receive function colors; and common column types (integer, varchar,
  timestamp, jsonb, uuid, etc.) join the keyword set. `::` casts remain intact
  rather than misreading the second colon as a parameter. A fixture test covers
  each shape plus span integrity. Latest complete gate787 with Clippy/build
  pass (`markdown-grammar-sql-final-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- HTML/XML fences now highlight document-level declarations. `<!DOCTYPE ...>`
  and `<?...?>` processing instructions are single keyword spans (including
  quoted internals and DOCTYPE internal subsets), while `<![CDATA[...]]>`
  content is a single string span instead of being reparsed as tags and
  attributes. A fixture test covers all three plus comments/entities and span
  integrity. Latest complete gate788 with Clippy/build pass
  (`markdown-grammar-html-final-*`). Full Shiki grammar parity and live visual
  acceptance remain.
- Markdown now autolinks bare GFM URLs and emails like Source's Marked
  `gfm`/`breaks` configuration. `http(s)://`, `ftp://`, `www.` and plain email
  addresses become inline link ranges; trailing punctuation and unbalanced
  closing parentheses are trimmed, balanced parentheses stay in the URL, and
  explicit links, images, inline code and fenced code are skipped. `www.` gets
  the same `http://` href Marked uses, and Source's safe-URL policy still limits
  clickable handoff to credential-free HTTPS. Core event and inline-range tests
  cover the path. Latest complete gate791 with Clippy/build pass
  (`markdown-autolink-final-*`). Full Marked/GFM parity and live visual
  acceptance remain.
- Bare autolinks now survive Markdown emphasis characters. Before parsing, URL
  and email candidates outside code spans/fences, HTML tags, image/link labels
  and explicit destinations are escaped for `_`, `*`, `~`, backticks and
  brackets; the parser then yields one text run so common URLs (including
  Wikipedia-style underscores) and `first_last@example.com` autolink whole and
  display without escape characters. A rendered test also locks Marked's
  `breaks: true` behavior: a soft newline occupies a hard line, not a space.
  Latest complete gate794 with Clippy/build pass
  (`markdown-autolink-punctuation-final-*`). Full Marked/GFM parity and live
  visual acceptance remain.
- Python fences now handle prefixed triple-quoted strings. Raw, byte and
  formatted triple-quoted literals with `r`/`b`/`f` combinations stay single
  string spans across embedded quotes and newlines, and triple-quoted
  f-strings close only at the matching three-quote delimiter while still
  tokenizing `{...}` interpolation expressions. A fixture test covers raw
  triple strings, triple f-strings with numbers, byte strings and span
  integrity. Latest complete gate795 with Clippy/build pass
  (`markdown-grammar-python-triple-final-*`). Full Shiki grammar parity and
  live visual acceptance remain.
- Markdown math now accepts Source's inline `\(...\)` delimiters as well as
  existing dollar math. The conversion runs outside inline code and fenced
  blocks, preserving literal `\(code\)` examples while rendering real
  formulas in the math font with `TeX:` source annotations. Core event and
  rendered math-font tests cover the path. Latest complete gate797 with
  Clippy/build pass (`markdown-math-delimiters-final-*`). Full KaTeX layout,
  remaining Marked/Shiki parity and live visual acceptance remain.
- YAML fences now color document structure and literals. `---`/`...`
  document markers and `%YAML`/`%TAG` directives are keywords; `true`,
  `false`, `yes`, `no`, `on`, `off`, `null` and `~` are literals outside
  keys, quoted strings and block scalars. Keys retain property coloring.
  A fixture test covers markers, directives, booleans/null forms and span
  integrity. Latest complete gate798 with Clippy/build pass
  (`markdown-grammar-yaml-literals-final-*`). Full Shiki grammar parity and
  live visual acceptance remain.
- Diff fences now color patch metadata separately from changes. `diff --git`,
  `index`, mode/rename/copy lines, similarity markers and `\ No newline at
  end of file` receive comment coloring; `---`/`+++` and hunk headers stay
  keywords, and added/removed lines keep their colors. A fixture test covers
  every metadata form plus span integrity. Latest complete gate799 with
  Clippy/build pass (`markdown-grammar-diff-metadata-final-*`). Full Shiki
  grammar parity and live visual acceptance remain.
- ANSI code fences now use Source's `ansi` highlighter path. SGR escapes are
  stripped from displayed text while the raw fence remains the copy payload;
  16-color, 256-color and truecolor foreground/background codes plus bold,
  dim, italic, underline, strikethrough and reverse are applied to shaped
  runs. Tokenizer, run-style and rendered code-surface tests cover the path.
  Latest complete gate802 with Clippy/build pass (`markdown-ansi-final-*`).
  Full Shiki grammar parity and live visual acceptance remain.
- Terminal cells now carry SGR 2 dim and SGR 9 strikethrough flags from the
  emulator into styled rows. Dim reduces foreground alpha, strikethrough adds
  a matching rule, and both reset through SGR 0/22/29. Emulator and
  terminal-row tests cover flag propagation, reset and run boundaries. Latest
  complete gate804 with Clippy/build pass (`terminal-dim-strike-final-*`).
  Live terminal/TUI visual acceptance remains.
- Terminal cells now preserve alacritty's single, double, curly, dotted and
  dashed underline variants instead of collapsing them to a boolean.
  `UnderlineKind` flows from emulator cells through terminal run styling;
  GPUI rendering maps single to a 1px rule, double to a 2px rule, curly to a
  wavy rule, and dotted/dashed to the nearest solid rule until richer
  painting exists. Emulator and byte-exact row-run tests cover each SGR 4:x
  variant and reset. Latest complete gate806 with Clippy/build pass
  (`terminal-underline-variants-final-*`). Exact dotted/dashed painting and
  live terminal acceptance remain.
- Terminal underline variants now paint exactly where GPUI's single
  underline style cannot express them. Single uses the shaped solid rule and
  curly the wavy rule; double paints two 1px cell segments, dotted paints
  1px dots every 3px, and dashed paints 3px/2px segments. `UnderlineKind`
  still flows from emulator cells through terminal run styling. Emulator,
  row-run and rendered segment tests cover every SGR 4:x variant and reset.
  Latest complete gate807 with Clippy/build pass
  (`terminal-underline-painting-final-*`). Live terminal acceptance remains.
- Terminal underline colors now follow SGR 58/59. Cells carry indexed,
  truecolor or named underline colors from alacritty, terminal run styling
  preserves them, shaped solid/wavy underlines and exact
  double/dotted/dashed segments paint with the explicit color, and SGR 59
  restores the default foreground color. Emulator and terminal-row tests
  cover explicit truecolor, reset and run color. Latest complete gate809
  with Clippy/build pass (`terminal-underline-color-final-*`). Live terminal
  acceptance remains.
- The bounded math renderer now covers more common KaTeX commands:
  `\mathbb` maps common sets to double-struck Unicode, `\boxed` wraps in
  brackets, `\pmod`/`\bmod`/`\mod` render modulo wording, and
  `\textcolor`/`\colorbox`/`\fcolorbox`/`\color` consume color arguments
  while keeping readable text. Rendering remains Unicode text rather than
  full KaTeX layout. A parser test covers each command. Latest complete
  gate810 with Clippy/build pass (`markdown-math-font-commands-final-*`).
  Full KaTeX layout/accessibility and live visual acceptance remain.
- The bounded math renderer's symbol set now covers common relation,
  arrow, big-operator and miscellaneous TeX commands (`\ll`, `\sim`,
  `\subseteq`, `\mapsto`, `\Leftrightarrow`, `\oint`, `\bigcup`,
  `\ell`, `\aleph`, `\emptyset`, `\angle`, `\square`, `\oplus`,
  `\vdots`, `\ddots`, and related forms). Unsupported commands still fall
  back to readable text. A parser test covers the expanded set. Latest
  complete gate811 with Clippy/build pass (`markdown-math-symbols-final-*`).
  Full KaTeX layout/accessibility and live visual acceptance remain.
- The bounded math renderer now covers over/under braces and directional
  arrow accents. `\overbrace{x}^{n}` and `\underbrace{x}_{n}` append brace
  marks before script conversion, and
  `\overrightarrow`/`\overleftarrow`/`\overleftrightarrow` plus under-arrow
  variants apply combining marks. A parser test covers each form. Latest
  complete gate812 with Clippy/build pass (`markdown-math-braces-final-*`).
  Full KaTeX layout/accessibility and live visual acceptance remain.
- Markdown fences now highlight document structure. Heading markers,
  blockquote/list/task markers, horizontal rules and fenced-code delimiters
  are keywords; inline code is a string and link labels/URLs receive
  function/string colors. Fenced bodies stay plain in this bounded grammar.
  A fixture test covers every shape with span integrity. Latest complete
  gate813 with Clippy/build pass (`markdown-grammar-markdown-final-*`). Full
  Shiki grammar parity and live visual acceptance remain.
- Markdown fences now color emphasis markers (`**`/`__`/`*`/`_`/`~~`) and
  inline HTML tags/attributes inside prose, while leaving underscores
  inside words such as snake_case plain. A fixture test covers markers,
  inline HTML, underscore boundary handling and span integrity. Latest
  complete gate814 with Clippy/build pass (`markdown-grammar-inline-final-*`).
  Full Shiki grammar parity and live visual acceptance remain.
- HTML/XML fences now handle namespaced element/attribute names containing
  `:`/`.` and color unquoted attribute values such as `data-id=abc`
  alongside quoted values. A fixture test covers namespaced tags,
  hyphenated/namespaced attributes, quoted/unquoted values and span
  integrity. Latest complete gate815 with Clippy/build pass
  (`markdown-grammar-html-attrs-final-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- The editor now auto-closes bracket and quote pairs like Monaco's
  language-defined behavior. `()`, `[]`, `{}`, `""`, `''` and backticks
  insert as a pair with the caret inside; typing an existing closing
  character skips over it; Backspace removes an empty pair; quotes only
  auto-close at a token boundary so word-internal apostrophes stay plain.
  Pure helper and rendered editor tests cover insertion, overtype and pair
  erasure. Latest complete gate817 with Clippy/build pass
  (`editor-auto-pairs-final-*`). Selection auto-surround and
  language-specific quote suppression remain open.
- Editor auto-pairs now wrap selections and suppress quote auto-closing
  inside line/block comments and existing strings, with Markdown disabling
  quote pairing entirely. Typing an opening pair over a selection surrounds
  it and keeps the selected text selected. Pure helper and rendered tests
  cover wrapping, comment/string suppression and Markdown behavior. Latest
  complete gate818 with Clippy/build pass
  (`editor-auto-pair-surround-final-*`). Exact language-by-language Monaco
  quote policy and live visual acceptance remain open.
- Terminal OSC 8 hyperlinks now attach to every covered cell and survive
  line projection. Linked cells receive a default underline; credential-free
  HTTPS links are clickable through Source's safe external-URL policy while
  other schemes remain visible but inert. Emulator and terminal-row tests
  cover attachment, reset and underline rendering. Latest complete gate820
  with Clippy/build pass (`terminal-osc8-links-final-*`). Live terminal
  acceptance remains.
- Editor auto-pairing now follows Source's Monaco language configurations per
  extension: JavaScript/TypeScript brace/quote policies, JSON's restricted
  set, HTML angle brackets and all quotes, CSS-family comment/string
  suppression, Markdown's no-quote bracket/angle set, and Python/Shell/YAML/
  SQL pair definitions. Policy/context and surrounding-pair tests cover the
  extension matrix. Latest complete gate820 with Clippy/build pass
  (`editor-auto-pair-policy-final-*`). Live visual acceptance remains.
- YAML fences now color quoted keys as properties when followed by a
  colon, including single-quoted, list-item and flow-mapping keys. Quoted
  values remain strings. A fixture test covers each shape plus span
  integrity. Latest complete gate821 with Clippy/build pass
  (`markdown-grammar-yaml-keys-final-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- Editor Enter now uses Monaco-style full auto-indent. Lines ending in
  `{`/`[`/`(` add a two-space level, Python lines ending in `:` indent,
  HTML opening tags indent, and a following closer moves to its own
  outdented line with the caret on the indented blank line. Markdown/YAML
  keep plain indent copying. Pure and rendered tests cover braces, Python,
  HTML, nested indentation and caret placement. Latest complete gate823
  with Clippy/build pass (`editor-enter-indent-final-*`). Live visual
  acceptance remains.
- Python and Bash fences now color common built-ins as functions. Python
  covers `print`, `len`, `range`, `int`, `str`, collection/type helpers,
  iteration helpers, `open`, `super` and common exception classes; Bash
  covers `echo`, `printf`, `read`, directory/navigation built-ins, `test`,
  `true`/`false`, process/job built-ins and declaration helpers. Fixture
  assertions cover both sets. Latest complete gate823 with Clippy/build pass
  (`markdown-grammar-builtins-final-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- Editor carets now blink on a focus/window-aware 530ms cadence. The timer
  runs only while the editor is focused and the window active, stays steady
  when blurred, and typing/navigation resets the caret visible. A hidden-
  phase reset regression test covers the shared blink helper. Latest
  complete gate824 with Clippy/build pass (`editor-cursor-blink-final-*`).
  Smooth opacity fading versus Monaco's `cursorBlinking: smooth` and live
  visual acceptance remain.
- Editor carets now use Monaco-style smooth opacity fading: a roughly
  530ms hold followed by an approximately 140ms fade. The timer runs only
  while the editor is focused and the window active, blur restores a steady
  visible caret, and typing/navigation resets it visible for the next hold.
  Pure tick and timed-entity tests cover fade timing, reset and blur. Latest
  complete gate825 with Clippy/build pass (`editor-smooth-caret-final-*`).
  Live visual acceptance remains.
- Editor wheel scrolling now honors Monaco's `smoothScrolling: true`. The
  list's immediate native wheel target is captured and rewound, then the
  viewport advances in 16 ms frames with 35% easing until convergence;
  keyboard paging and caret reveal remain immediate. A rendered timing test
  covers native-target rewind, first-frame movement and convergence. Latest
  complete gate826 with Clippy/build pass (`editor-smooth-scroll-final-*`).
  Live visual acceptance remains.
- Editor quick suggestions now follow Source's `quickSuggestions` and 25ms
  delay. Typing an identifier character (including `_`, `$`, `.`) schedules
  the document-word popup, comments are excluded, strings remain eligible,
  manual Ctrl+Space is immediate, and intervening keys cancel stale timers.
  Pure trigger/comment and rendered delayed-open tests cover the path.
  Latest complete gate828 with Clippy/build pass
  (`editor-quick-suggestions-final-*`). Live visual acceptance remains.
- SQL fences now color comparison, arithmetic and JSON/PostgreSQL
  operators (`=`, `<>`, `>=`, `||`, `::`, `->>`, etc.) alongside keywords,
  strings, identifiers, parameters and types. Line/block comments and
  parameter forms keep precedence. A fixture test covers operators,
  comments, casts and span integrity. Latest complete gate829 with
  Clippy/build pass (`markdown-grammar-sql-operators-final-*`). Full Shiki
  grammar parity and live visual acceptance remain.
- Editor quick suggestions now use language-aware word boundaries and
  triggers. CSS/SCSS/Less retain hyphenated/custom-property tokens (plus
  SCSS `$` and Less `@`), HTML/XML retain hyphen/colon/dot name punctuation,
  and only those extensions treat `-` as a suggestion trigger. Completion
  ranges and document-word extraction use the same language rules. Fixture
  tests cover ranges, continuation completion and trigger rules. Latest
  complete gate830 with Clippy/build pass
  (`editor-quick-suggestion-languages-final-*`). Live visual acceptance
  remains.
- YAML flow mappings now color nested unquoted keys as properties when they
  follow `{`/`,`; quoted flow keys use the same context. Top-level keys keep
  a single property span, and boolean/null values remain keywords. A fixture
  test covers nested/quoted flow keys, values and span integrity. Latest
  complete gate831 with Clippy/build pass
  (`markdown-grammar-yaml-flow-final-*`). Full Shiki grammar parity and live
  visual acceptance remain.
- The completion popup now anchors below the caret line instead of the
  editor's bottom-left, clamps within the viewport, and flips above the
  caret when there is not enough space below. Rows use Source's
  `suggestFontSize: 13` and `suggestLineHeight: 24`. The rendered completion
  test asserts placement relative to the edited row. Latest complete
  gate831 with Clippy/build pass (`editor-completion-popup-final-*`). Live
  visual acceptance remains.
- The bounded math renderer now covers big delimiter commands
  (`\big`, `\Big`, `\bigg`, `\Bigg`), cancel/strike fallbacks
  (`\cancel`, `\bcancel`, `\xcancel`, `\sout`), text-style commands
  (`\textbf`, `\textit`, `\texttt`, `\textsf`, `\emph`), math class
  wrappers and `\fbox`. A parser test covers each form. Latest complete
  gate832 with Clippy/build pass (`markdown-math-big-cancel-final-*`). Full
  KaTeX layout/accessibility and live visual acceptance remain.
- Editor text selections and the active-line background now use the active
  appearance profile's `selected` color, matching Source's Monaco mapping
  for `editor.selectionBackground` and `editor.lineHighlightBackground`.
  The hard-coded accent selection fill and reduced-opacity line highlight
  are gone, with a profile-color regression test. Latest complete gate833
  with Clippy/build pass (`editor-selection-colors-final-*`). Live visual
  acceptance remains.
- The editor now auto-completes the JavaScript/TypeScript JSDoc opener:
  typing the third character of `/**` inserts ` */` and leaves the caret
  inside `/** | */`. HTML/CSS/other languages and string contexts are
  unaffected. Pure and rendered tests cover the language guard and
  insertion. Latest complete gate835 with Clippy/build pass
  (`editor-jsdoc-pair-final-*`). Live visual acceptance remains.
- Editor `<`/`>` pairing now matches Monaco's configuration split: HTML/XML
  surrounds `< >` only when wrapping a selection and does not auto-insert
  `>` on typing `<`; Backspace does not erase a `< >` pair there. Markdown
  continues to auto-close `<` because its own config lists it under
  auto-closing pairs. Pure tests cover the distinction. Latest complete
  gate835 with Clippy/build pass (`editor-html-angle-pair-final-*`). Live
  visual acceptance remains.
- Editor comment commands now follow Monaco: `Cmd+/` toggles each
  language's line-comment token, falling back to the configured block
  comment for CSS, HTML/XML and Markdown. `Shift+Alt+A` always toggles a
  block comment. Multi-line selections align insertion at the minimum
  indent, ignore blank lines in mixed selections, remove uniformly
  commented selections and collapse to one undo step. Pure and rendered
  tests cover the commands. Latest complete gate838 with Clippy/build
  pass (`editor-comment-toggle-final-*`). Live visual acceptance remains.
- Shared text inputs now match Source's global `::selection` rule: the
  active appearance's accent at 32% opacity, replacing the fixed blue
  fill. Composer, find, rename and other native fields therefore follow
  every palette. A pure palette test covers channels and alpha. Latest
  complete gate839 with Clippy/build pass
  (`composer-selection-color-final-*`). Live visual acceptance remains.
- Multi-cursor editing now adds Monaco's select-all-occurrences and
  add-cursor-above/below commands. `Cmd+Shift+L` keeps the original
  match primary; a collapsed caret expands to a case-sensitive whole
  word, while a non-empty selection follows the Find widget's
  case/whole-word options. `Cmd+Alt+Up/Down` duplicates every primary
  and extra selection one logical line at the same grapheme column and
  deduplicates targets. Pure and rendered tests cover matching options,
  translation, multi-selection duplication and one-undo-step edits.
  Latest complete gate844 with Clippy/build pass
  (`editor-multicursor-commands-final-*`). Live visual acceptance
  remains.
- Markdown fences now highlight Rust: line and nested block comments,
  attributes, storage keywords, primitive/upper-camel types, lifetimes,
  `fn` definitions and calls, macros, boolean and numeric literals with
  underscores/suffixes, escaped/byte strings, char literals and raw
  strings. A new `Type` token uses GitHub-light/dark type colors.
  Fixture and palette tests cover the grammar. Latest complete gate846
  with Clippy/build pass (`markdown-rust-grammar-final-*`). Full Shiki
  parity and live visualization acceptance remain.
- Markdown fences now highlight Go: line/block comments, control-flow
  and storage keywords, primitive types, `type` declarations,
  functions/method calls, built-ins via the call heuristic, interpreted
  and raw backtick strings, rune literals, booleans/nil/iota and
  decimal/hex/octal/binary/float/hex-float numbers with underscores and
  imaginary suffixes. Fixture and span-integrity tests cover the
  grammar. Latest complete gate847 with Clippy/build pass
  (`markdown-go-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Inline-code path classification now carries Source's full
  Linguist-derived data: 1,438 extensions, 301 exact filenames, 140
  prefixes and the `.d.ts` special case, including dotfiles and
  `Dockerfile.dev`-style prefixes. HTTP(S) URL detection is ASCII
  case-insensitive, scheme rejection is anchored, and single-segment
  `/name` text stays plain. The fixture table covers all rules and
  Source quirks. Latest complete gate847 with Clippy/build pass
  (`inline-code-linguist-final-*`). Live visual acceptance remains.
- Composer `@` mention spans now follow Source's `composer-completions.ts`
  instead of stored insertion offsets. A trigger starts only when `@`
  begins a whitespace-delimited token and is suppressed inside an
  existing mention; spans are derived from current text. At send,
  mention offsets are rebuilt from the trimmed composer and tokens
  edited away are omitted while plain attachments remain. Pure tests
  cover trigger/span rules and stale-offset reconciliation. Latest
  complete gate849 with Clippy/build pass
  (`composer-mention-spans-final-*`). Live visual acceptance remains.
- Markdown fences now highlight Java: line/block comments, annotations
  (including qualified names), keywords and module declarations,
  primitive and upper-camel types, class/interface/enum/record
  declarations, function/method calls, text blocks, escaped strings and
  chars, booleans/null and decimal/hex/binary numbers with underscores,
  exponent and type suffixes. Fixture and span-integrity tests cover the
  grammar. Latest complete gate850 with Clippy/build pass
  (`markdown-java-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight C and C++ through a shared C-family
  tokenizer: line/block comments, preprocessor directives with
  quoted/angle headers, C/C++ keywords and primitive types,
  struct/union/enum/class declarations, function/method/macro calls,
  prefixed escaped strings/chars, C++ `[[attributes]]`, raw strings with
  delimiters and apostrophe digit separators. Fixture and
  span-integrity tests cover both languages. Latest complete gate852
  with Clippy/build pass (`markdown-c-cpp-grammar-final-*`). Full Shiki
  parity and live visual acceptance remain.
- Markdown fences now highlight Ruby: line and `=begin`/`=end` block
  comments, keywords and built-ins, class/module/def names, variables,
  symbols, single/double/backtick strings with interpolation, percent
  literals, heredocs, booleans/nil and hex/binary/octal/decimal/float
  numbers with underscores and rational/imaginary suffixes. Fixture and
  span-integrity tests cover the grammar. Latest complete gate853 with
  Clippy/build pass (`markdown-ruby-grammar-final-*`). Full Shiki parity
  and live visual acceptance remain.
- Markdown fences now highlight PHP: `<?php`/`<?=`/`?>` tags,
  `//`/`#`/`/* */` comments, keywords and primitive types,
  class/interface/trait/enum and function declarations, method/static
  calls, `$variables`, `#[attributes]`, single/double/backtick strings
  with interpolation, `<<<` heredoc/nowdoc strings, booleans/null and
  hex/binary/octal/decimal/float numbers. Fixture and span-integrity
  tests cover the grammar. Latest complete gate854 with Clippy/build
  pass (`markdown-php-grammar-final-*`). Full Shiki parity and live
  visual acceptance remain.
- The editor now implements Monaco's Go to Line/Column on `Ctrl+G`.
  macOS keeps `Cmd+G` for Find Next, matching Monaco's platform
  bindings. The popup accepts `line`, `line,col`, `line:col` and
  `line#col`, counts negative lines back from the end, clamps columns
  to the target line's graphemes, shows Monaco's valid/invalid labels,
  reveals the caret, and closes on Enter/Escape. Pure and rendered tests
  cover parsing, offsets, opening, submission and dismissal. Latest
  complete gate856 with Clippy/build pass (`editor-goto-line-final-*`).
  Live visual acceptance remains.
- The editor now supports Monaco's Move Line Up/Down on `Alt+Up` and
  `Alt+Down`. It moves the caret's line or the selected whole-line block,
  preserves block bytes and trailing-newline state, follows the moved
  block with the caret/selection, and no-ops at the first/last boundary.
  Monaco's context-sensitive re-indentation on moves remains unfinished.
  Pure and rendered tests cover single-line, block, selection,
  no-trailing-newline and boundary cases. Latest complete gate858 with
  Clippy/build pass (`editor-move-line-final-*`). Live visual acceptance
  remains.
- The editor now supports Monaco's Copy Line Up/Down on `Shift+Alt+Up`
  and `Shift+Alt+Down`. It duplicates the caret line or selected
  whole-line block with Monaco's exact selection rules: copy-down
  inserts before the block and shifts the selection with it, while
  copy-up inserts after the block and leaves the cursor on the original
  line. Empty lines, multi-line blocks and trailing-newline state are
  covered. Pure and rendered tests mirror Monaco's copy-line cases.
  Latest complete gate860 with Clippy/build pass
  (`editor-copy-line-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's line insert/delete commands:
  `Cmd+Enter` inserts below, `Cmd+Shift+Enter` inserts above with the
  current line's indentation, and `Cmd+Shift+K` deletes the caret line
  or selected whole-line block. Delete-line handles following-newline
  removal, last-line preceding-newline removal, empty documents, blank
  lines and multi-line selections. Language-specific extra auto-indent
  on inserted lines remains unfinished. Pure and rendered tests cover
  boundaries and keybindings. Latest complete gate862 with Clippy/build
  pass (`editor-line-insert-delete-final-*`). Live visual acceptance
  remains.
- The editor now supports Monaco's Join Lines on macOS `Ctrl+J`. It
  joins the caret line with the next line, dedents the following line,
  inserts one space only when neither side already has whitespace,
  collapses trailing whitespace to one space, ignores blank lines,
  joins multi-line selections, and moves the caret/selection per
  Monaco's rules. Last-line joins are no-ops. Pure and rendered tests
  cover whitespace, blanks, selections and boundaries. Latest complete
  gate864 with Clippy/build pass (`editor-join-lines-final-*`). Live
  visual acceptance remains.
- The editor now supports Monaco's Go to Bracket on `Cmd+Shift+\`.
  It jumps to the partner of a bracket at the primary caret, otherwise
  to the closing bracket of the innermost enclosing pair, otherwise to
  the next structural bracket, skipping strings and comments. Repeated
  presses alternate between partners. Pure and rendered tests cover the
  partner, enclosing, next-bracket, unmatched and comment-masking
  cases. Latest complete gate866 with Clippy/build pass
  (`editor-goto-bracket-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's Indent Line / Outdent Line on
  `Cmd+]` / `Cmd+[`. Indent adds two spaces to every selected line,
  skips empty lines in multi-line selections and indents a lone empty
  line; outdent removes one tab or up to two leading spaces and skips
  unindented lines. Primary and extra cursors/selections are remapped
  and remain selected. Pure and rendered tests cover caret, selection,
  empty-line, tab and multi-cursor mapping. Latest complete gate868
  with Clippy/build pass (`editor-indent-outdent-final-*`). Live visual
  acceptance remains.
- The editor now supports Monaco's Go to Next/Previous Problem on `F8`
  / `Shift+F8`. It sorts and deduplicates current language, auto-import
  and W3C/Sass validation diagnostics, moves the primary caret with
  wraparound, reveals the target and clears extra cursors. Pure and
  rendered tests cover ordering, wrap, empty diagnostics and shortcut
  traversal. Latest complete gate870 with Clippy/build pass
  (`editor-problem-navigation-final-*`). Live visual acceptance remains.
- Move Line now reindents the moved block from its destination context,
  and Insert Line Above/Below now inherit line-context auto-indent.
  Bounded rules follow open `{`/`[`/`(` delimiters, Python `:` blocks
  and HTML open tags, preserve relative indentation across multi-line
  moves, and remap selections after indentation changes. Pure and
  rendered tests cover insertion above/below, Python, HTML, single-line
  and block moves into/out of blocks. Latest complete gate872 with
  Clippy/build pass (`editor-line-autoindent-final-*`). Live visual
  acceptance remains.
- Scheduled retry rows now settle when the turn ends instead of
  lingering. `session.step.ended` clears retry state on completion, and
  non-retry `session.status` phases (busy/idle/error) clear any scheduled
  retry; step-start, idle, failure and interrupt paths already did so.
  Transcript tests cover step-end and busy-status settlement plus
  removal of the Retry block. Latest complete gate873 with Clippy/build
  pass (`retry-settlement-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's Trim Trailing Whitespace on
  `Cmd+K Cmd+X`. It trims trailing spaces/tabs across the document,
  skips the caret's line when the caret sits at its end, starts trimming
  at the caret inside trailing whitespace, preserves whitespace inside
  multiline strings, and keeps the caret mapped. Pure and rendered tests
  cover caret, cursor-line, string and chord behavior. Latest complete
  gate875 with Clippy/build pass
  (`editor-trim-whitespace-final-*`). Live visual acceptance remains.
- Trim Trailing Whitespace now also protects JavaScript/TypeScript
  regex literals, capping unterminated regexes to their line so later
  lines and division expressions still trim. Pure and rendered tests
  cover unterminated/closed regexes, division and the chord. Latest
  complete gate876 with Clippy/build pass
  (`trim-whitespace-regex-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's Delete All Left/Right. macOS
  `Cmd+Backspace` deletes from the selection end back to the line start
  (or the previous line break at column 1), while `Ctrl+K` and
  `Cmd+Delete` delete to the line end (or the next line break at the
  line end). Non-empty selections follow Monaco's asymmetric rules,
  overlapping ranges merge, and primary/extra carets are remapped. Pure
  and rendered tests cover caret, selection, line-boundary and shortcut
  cases. Latest complete gate878 with Clippy/build pass
  (`editor-delete-all-final-*`). Live visual acceptance remains.
- Markdown fences now highlight Kotlin: line and nested block comments,
  annotations (including qualified names), keywords and modifiers,
  class/interface/object/typealias declarations, fun names and calls,
  upper-camel types, triple-quoted raw strings with interpolation,
  escaped strings/chars, booleans/null and hex/binary/decimal/float
  numbers. Fixture and span-integrity tests cover the grammar. Latest
  complete gate879 with Clippy/build pass
  (`markdown-kotlin-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Swift: line and nested block comments,
  attributes (including dotted names), keywords and modifiers,
  actor/class/enum/extension/protocol/struct/typealias declarations,
  func names and calls, upper-camel types, multiline strings,
  hash-delimited raw strings, escaped strings with interpolation,
  compiler directives, booleans/nil and hex/binary/octal/decimal/float
  numbers. Fixture and span-integrity tests cover the grammar. Latest
  complete gate880 with Clippy/build pass
  (`markdown-swift-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Dockerfiles: line-start `#` comments,
  case-insensitive instructions (FROM/AS/RUN/COPY/ENV/EXPOSE/CMD and the
  broader Docker set), `$VAR`/`${VAR}` expansions as properties, quoted
  strings, and line-start detection so instruction names inside
  arguments stay plain. Fixture and span-integrity tests cover the
  grammar. Latest complete gate881 with Clippy/build pass
  (`markdown-dockerfile-grammar-final-*`). Full Shiki parity and live
  visual acceptance remain.
- The editor now offers bounded worker-equivalent semantic completion for
  CSS/SCSS/Less and HTML/XML. CSS declaration contexts complete property
  names; value contexts complete common keywords. HTML tag contexts
  complete tag names until whitespace, then attribute names. Semantic
  candidates precede and deduplicate against document words. Pure and
  rendered tests pass. Rebooting the host cleared the stuck root `spindump`
  process; the authoritative workspace gate is green at **884 passed**
  with Clippy/build pass (`toml-css-final-*`). Live visual acceptance
  remains.
- Markdown fences now highlight TOML: `#` comments, table headers
  (including array-table inner names), dotted/bare and inline-table keys
  as properties, basic/literal and multiline strings, booleans and
  decimal/hex/octal/binary/float numbers. Fixture and span-integrity
  tests pass. Rebooting the host cleared the stuck root `spindump` process;
  the authoritative workspace gate is green at **884 passed** with
  Clippy/build pass (`toml-css-final-*`).
- Markdown fences now highlight C#: line/block comments,
  `#region`-style directives, line-start attributes, keywords and
  modifiers, class/struct/interface/enum/record/namespace/delegate
  declarations, method/constructor calls, primitive and upper-camel
  types, verbatim/interpolated/raw strings, escaped strings/chars,
  booleans/null and numeric forms with suffixes. Fixture and
  span-integrity tests cover the grammar. Latest complete gate885 with
  Clippy/build pass (`markdown-csharp-grammar-final-*`). Full Shiki
  parity and live visual acceptance remain.
- The editor now has a bounded `F1` command palette over its implemented
  commands, with case-insensitive filtering, arrow-key selection, Enter
  execution, Escape dismissal and click support. It adds Monaco line
  transforms Sort Ascending/Descending, Reverse Lines and Delete
  Duplicate Lines; a single empty selection applies them to the whole
  document, line selections stay scoped, and duplicate deletion keeps
  first occurrences. Pure and rendered tests cover filtering, transforms
  and execution. Latest complete gate887 with Clippy/build pass
  (`editor-command-palette-final-*`). Live visual acceptance remains.
- The command palette now also exposes Monaco's Transform to Uppercase/
  Lowercase/Title Case and Transpose Characters. Empty carets transform
  the word under the caret while non-empty selections transform in
  place; Title Case preserves apostrophe contractions. Transpose swaps
  surrounding characters, crosses line ends with the newline, advances
  the caret and no-ops at the final line end. Pure and rendered palette
  tests cover all four. Latest complete gate888 with Clippy/build pass
  (`editor-case-transpose-final-*`). Live visual acceptance remains.
- The command palette now also exposes Duplicate Selection and Select to
  Bracket. Duplicate Selection copies selected text after itself and
  selects the copy, or duplicates the caret line downward for an empty
  selection. Select to Bracket selects the pair at the caret, the
  innermost enclosing pair, or the next structural pair, skipping
  strings/comments. Pure and rendered palette tests cover both. Latest
  complete gate890 with Clippy/build pass
  (`editor-duplicate-select-bracket-final-*`). Live visual acceptance
  remains.
- Editor `Tab`/`Shift+Tab` now follows Monaco's split behavior: whole-
  line or multi-line selections indent/outdent line blocks, carets
  advance to the next 2-space tab stop, and partial single-line
  selections are replaced with tab-stop whitespace. Emmet still takes
  precedence for HTML/CSS expansions. Pure and rendered tests cover
  line selections, shift-tab reversal, caret tab stops and partial
  selections. Latest complete gate891 with Clippy/build pass
  (`editor-tab-indent-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's Remove Brackets on
  `Cmd+Alt+Backspace`, also exposed in the command palette. It deletes
  the matching pair at the caret/active selection end or the innermost
  enclosing pair, keeps the selection mapped and does not use a
  next-bracket fallback. Pure and rendered tests cover caret,
  full-selection and no-match cases. Latest complete gate893 with
  Clippy/build pass (`editor-remove-brackets-final-*`). Live visual
  acceptance remains.
- The editor now supports Monaco's Go to Symbol in File on `Cmd+Shift+O`.
  It lists bounded document declarations in a filterable quick-pick
  panel; typing filters by symbol preview/name case-insensitively,
  arrows navigate, Enter/Tab jumps and focuses the editor, and Escape
  closes. Pure and rendered tests cover filtering and jump-to-symbol.
  Latest complete gate894 with Clippy/build pass
  (`editor-go-to-symbol-file-final-*`). Live visual acceptance remains.
- Workspace symbol search (`Cmd+T`) now has a live filter field. The
  initial query is the word under the caret, typing re-runs the bounded
  cross-document symbol search, arrows navigate, Enter/Tab emits the
  cross-file target and refocuses the editor, and Escape closes. Pure
  and rendered tests cover filtering and target emission. Latest
  complete gate895 with Clippy/build pass
  (`editor-workspace-symbol-filter-final-*`). Live visual acceptance
  remains.
- The command palette now also exposes Monaco's Transform to Snake/Camel/
  Pascal/Kebab Case. Snake/Kebab separate lower-to-upper and acronym
  boundaries and lowercase the result; Camel preserves acronym heads
  and lowercases a normal first-word start; Pascal also maintains
  dot-separated segments and capitalizes each boundary word. Pure and
  rendered tests cover boundary cases and palette execution. Latest
  complete gate897 with Clippy/build pass (`editor-case-styles-final-*`).
  Live visual acceptance remains.
- Markdown math now renders stacked annotations and substacks:
  `\overset`/`\stackrel` place the first argument above the base,
  `\underset` places it below, `\substack{a \\ b}` joins rows with
  `; ` while preserving nested groups and text commands, and
  `\boldsymbol`/`\bm`/`\pmb` unwrap to their argument. Pure parser
  tests and a rendered display-math test cover the forms. Latest
  complete gate899 with Clippy/build pass (`markdown-math-stack-final-*`).
  Live visual acceptance remains.
- Markdown fences now highlight Lua: shebang/line/block comments,
  long-bracket strings, control/logical/local keywords, built-in and
  dotted/colon library calls, function declarations/calls, quoted
  strings, booleans/nil and hex/decimal/float numeric forms. Fixture
  and rendered fence tests cover the grammar. Latest complete gate901
  with Clippy/build pass (`markdown-lua-grammar-final-*`). Full Shiki
  parity and live visual acceptance remain.
- Markdown fences now highlight R: `#`/roxygen comments, raw strings,
  control keywords, built-in and dotted/namespace calls, constants
  (`TRUE`/`FALSE`/`NULL`/`NA`/`Inf`/`NaN`) and decimal/hex/exponent/
  complex numbers with `L`/`i` suffixes. Fixture and rendered fence
  tests cover the grammar. Latest complete gate903 with Clippy/build
  pass (`markdown-r-grammar-final-*`). Full Shiki parity and live
  visual acceptance remain.
- Editor HTML semantic completion now offers context-specific attribute values
  for `type`, `rel`, `target`, `method`, `autocomplete`, `inputmode`,
  `crossorigin`, `decoding`, `dir`, `draggable`, `enctype`, `loading`,
  `scope`, `shape`, `spellcheck`, `translate` and `wrap`, and the tag/
  attribute catalog is broader. Values work in quoted and unquoted
  values; completed values hand back to attribute-name completion.
  Pure context tests and a rendered popup test cover the path. Latest
  complete gate905 with Clippy/build pass
  (`editor-html-value-completion-final-*`). Full worker parity and live
  visual acceptance remain.
- Editor CSS semantic completion now offers property-aware values for
  display, position, cursor, overflow, font-weight, text-transform,
  grid-auto-flow, pointer-events and related properties, with an
  expanded generic fallback and a much broader property catalog.
  Pure context tests and a rendered popup test cover the path. Latest
  complete gate907 with Clippy/build pass
  (`editor-css-value-completion-final-*`). Full worker parity and live
  visual acceptance remain.
- Editor CSS semantic completion now offers at-rule names at the start
  of a statement: `@charset`, `@container`, `@counter-style`, `@font-face`,
  `@font-feature-values`, `@import`, `@keyframes`, `@layer`, `@media`,
  `@namespace`, `@page`, `@property`, `@scope`, `@starting-style`,
  `@supports` and `@viewport`. Pure context tests and a rendered popup
  test cover the path. Latest complete gate909 with Clippy/build pass
  (`editor-css-at-rule-completion-final-*`). Full worker parity and live
  visual acceptance remain.
- Markdown fences now highlight Scala: line/nested block comments,
  annotations, package/import and control/declaration keywords,
  class/trait/object/enum/type declarations, `def`/call functions,
  upper-camel and primitive types, triple/escaped strings, char and
  symbol literals, booleans/null and hex/binary/decimal/exponent/
  suffixed numbers. Fixture and rendered fence tests cover the grammar.
  Latest complete gate911 with Clippy/build pass
  (`markdown-scala-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Dart: line/nested block comments,
  annotations, import/class/control keywords, class/enum/extension/
  mixin/typedef declarations, called functions, primitive/upper-camel
  types, single/triple-quoted interpolation strings, booleans/null and
  hex/decimal/exponent/separated numbers. Fixture and rendered fence
  tests cover the grammar. Latest complete gate913 with Clippy/build
  pass (`markdown-dart-grammar-final-*`). Full Shiki parity and live
  visual acceptance remain.
- Markdown fences now highlight PowerShell: line/block comments,
  single/double/here strings, `$variables` including scoped/global
  names, cmdlets and hyphenated function calls/declarations, type and
  attribute brackets, control keywords and hex/decimal/exponent/
  separated numbers. Fixture and rendered fence tests cover the grammar.
  Latest complete gate915 with Clippy/build pass
  (`markdown-powershell-grammar-final-*`). Full Shiki parity and live
  visual acceptance remain.
- Editor CSS value completion now offers property-aware functions:
  color functions (`rgb(`, `hsl(`, `color-mix(`), image/gradient
  functions, filter functions, transform functions, easing functions,
  and math/var helpers. Property-specific keywords still sort before
  functions where both apply. Pure context tests and a rendered popup
  test cover the path. Latest complete gate917 with Clippy/build pass
  (`editor-css-function-completion-final-*`). Full worker parity and
  live visual acceptance remain.
- Markdown fences now highlight Elixir: line comments, module
  attributes, def/defp/defmacro/defmodule/defprotocol declarations,
  called and dotted module functions, upper-camel aliases, heredocs,
  sigils, atoms, double/char-list strings, booleans/nil and hex/binary/
  decimal/exponent/separated numbers. Fixture and rendered fence tests
  cover the grammar. Latest complete gate919 with Clippy/build pass
  (`markdown-elixir-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Editor HTML closing-tag completion now uses the nearest unclosed
  ancestor: typing `</di` inside `<div>` offers `div>` and accepting
  produces `</div>`. Self-closing and void elements are ignored. With
  no open ancestor, known HTML tags are offered. Pure context tests and
  a rendered accept test cover the path. Latest complete gate921 with
  Clippy/build pass (`editor-html-close-tag-completion-final-*`). Full
  worker parity and live visual acceptance remain.
- Editor HTML completion now offers an auto-close suggestion after a
  completed non-void start tag: `<div>` offers `</div>`. `>` also
  triggers the 25ms quick-suggestion path for HTML/XML. Void and
  self-closing tags are ignored. Pure completion/trigger tests and a
  rendered accept test cover the path. Latest complete gate922 with
  Clippy/build pass (`editor-html-auto-close-tag-final-*`). Full worker
  parity and live visual acceptance remain.
- Markdown fences now highlight Haskell: language pragmas,
  line/nested block comments, module/import/control keywords,
  data/newtype/type/class/instance declarations, top-level functions
  and common built-ins, upper-camel types/constructors, strings/chars,
  booleans and hex/octal/decimal/exponent/separated numbers. Fixture
  and rendered fence tests cover the grammar. Latest complete gate924
  with Clippy/build pass (`markdown-haskell-grammar-final-*`). Full
  Shiki parity and live visual acceptance remain.
- Editor CSS completion now collects document-defined custom properties
  (`--name:`) from the current buffer, ignoring comments and strings.
  They appear in property and value completion and are prioritized
  inside an open `var(` call. Pure completion tests and a rendered
  accept test cover the path. Latest complete gate926 with Clippy/build
  pass (`editor-css-custom-property-final-*`). Full worker parity and
  live visual acceptance remain.
- Markdown prose now decodes common named and numeric character
  references (`&amp;`, `&lt;`, `&#35;`, `&#x1F600;`, etc.) before
  autolinking. Code spans, fenced code and raw HTML keep literal entity
  text. Core event tests and a UI flatten test cover prose/code
  distinctions, and a rendered prose-label test covers the render path.
  Latest complete gate929 with Clippy/build pass
  (`markdown-entity-decode-final-*`). Live visual acceptance remains.
- Markdown fences now highlight Clojure: line comments, ns/control/
  special forms, def/defn/defmacro/defmulti/defmethod names, symbols
  including hyphenated and namespaced vars, common built-ins, uppercase
  types, strings/chars, `:keywords`, booleans/nil and hex/octal/decimal/
  exponent/separated numbers. Fixture and rendered fence tests cover the
  grammar. Latest complete gate931 with Clippy/build pass
  (`markdown-clojure-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Julia: line/nested block comments,
  macros, module/struct/abstract-type/function declarations, built-in
  calls, upper-camel types, strings/chars/command literals, symbols,
  booleans and hex/octal/binary/decimal/exponent/separated numbers.
  Fixture and rendered fence tests cover the grammar. Latest complete
  gate933 with Clippy/build pass (`markdown-julia-grammar-final-*`).
  Full Shiki parity and live visual acceptance remain.
- Markdown fences now highlight OCaml: nested block comments,
  let/let-rec/function definitions, type/module/struct/val declarations,
  List/Array/String module calls and built-ins, upper-camel types and
  constructors, normal and `{|…|}` strings, chars/type variables,
  booleans and hex/octal/binary/decimal/exponent/separated numbers.
  Fixture and rendered fence tests cover the grammar. Latest complete
  gate935 with Clippy/build pass (`markdown-ocaml-grammar-final-*`).
  Full Shiki parity and live visual acceptance remain.
- Markdown table image sizing now shares the lazy-image cache key used by
  rendered images. When an async image first resolves, the cache
  transition schedules the transcript remeasure/notify path, so table
  columns reflow with the image instead of waiting for another layout
  event. A focused cache test and the rendered mixed table-image test
  cover the path. Latest complete gate936 with Clippy/build pass
  (`markdown-table-image-reflow-final-*`). Live visual acceptance
  remains.
- Markdown fences now highlight Erlang: `%` comments,
  `-module`/`-export`/`-record`/`-spec` attributes, function clauses
  and `module:function` chains, uppercase variables, built-in types,
  normal/quoted-atom strings, `$char` literals, booleans and
  base/decimal/exponent/separated numbers. Fixture and rendered fence
  tests cover the grammar. Latest complete gate938 with Clippy/build
  pass (`markdown-erlang-grammar-final-*`). Full Shiki parity and live
  visual acceptance remain.
- Editor CSS conditional completion now offers `@media` feature names
  inside `(...)` and enumerated values after a feature colon
  (orientation, pointer/hover, color scheme, reduced motion/contrast,
  forced colors, display mode, scripting/update, overflow and more).
  Inside `@supports (...)`, CSS property/value completion is reused.
  Pure context tests and a rendered popup test cover the path. Latest
  complete gate940 with Clippy/build pass
  (`editor-css-conditional-completion-final-*`). Full worker parity and
  live visual acceptance remain.
- Markdown fences now highlight Elm: line/nested block comments,
  module/import/type/alias/case declarations, annotated and
  parameterized top-level functions, standard module chains
  (List/String/Maybe/Html/Browser/Cmd/Sub/etc.), upper-camel types and
  constructors, strings/chars, True/False and hex/decimal/exponent/
  separated numbers. Fixture and rendered fence tests cover the grammar.
  Latest complete gate942 with Clippy/build pass
  (`markdown-elm-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight HCL/Terraform: `#`/`//`/block
  comments, terraform/resource/data/provider/variable/output/module/
  locals and related block keywords, attribute keys,
  `var`/`local`/`data`/`count`/`each`/`module`/`path` references,
  lookup/format/collection functions, quoted strings, heredocs,
  booleans/null and numeric forms. Fixture and rendered fence tests
  cover the grammar. Latest complete gate944 with Clippy/build pass
  (`markdown-hcl-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Nix: `#`/block comments,
  let/inherit/in/rec and control keywords, dotted attribute paths,
  import/map/builtins/lib functions, quoted and indented strings,
  booleans/null and hex/decimal/exponent/separated numbers. Fixture
  and rendered fence tests cover the grammar. Latest complete gate946
  with Clippy/build pass (`markdown-nix-grammar-final-*`). Full Shiki
  parity and live visual acceptance remain.
- Markdown fences now highlight Puppet: `#`/block comments,
  class/define/node/function/plan/type declarations, resource types,
  `=>` attribute keys, `$variables`, include/template/sprintf/notice and
  related functions, uppercase types, strings, booleans/undef and
  hex/decimal/exponent numbers. Fixture and rendered fence tests cover
  the grammar. Latest complete gate948 with Clippy/build pass
  (`markdown-puppet-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight GraphQL: `#` comments,
  query/mutation/subscription/fragment/type/interface/union/enum/input/
  scalar/schema/directive keywords, upper-camel operation/type/enum
  names, `$variables`, `@directives`, normal/block strings,
  booleans/null and signed decimal/exponent numbers. Fixture and
  rendered fence tests cover the grammar. Latest complete gate950 with
  Clippy/build pass (`markdown-graphql-grammar-final-*`). Full Shiki
  parity and live visual acceptance remain.
- Markdown fences now highlight Vue single-file components: the
  `<template>` range composes the HTML grammar, adds `:`/`@`/`v-`
  directive attributes and highlights `{{ … }}` mustaches as
  expressions, while `<script>`/`<style>` bodies reuse the
  JavaScript/TypeScript and CSS grammars. Fixture and rendered fence
  tests cover the grammar. The first full run hit both known observation
  flakes; both passed in isolation and the authoritative rerun is green
  at gate952 with Clippy/build pass (`markdown-vue-grammar-final-*`).
  Full Shiki parity and live visual acceptance remain.
- Markdown fences now highlight Svelte components: `<script>`/`<style>`
  bodies reuse the JavaScript/TypeScript and CSS grammars, directive
  attributes stay highlighted through the HTML grammar, special tags
  (`{#if}`, `{:else}`, `{#each}`, `{#await}`, `{@html}`, closers) paint
  their keyword prefix, and `{…}` interpolations tokenize the inner
  expression as JavaScript/TypeScript. Fixture and rendered fence tests
  cover the grammar. The first full run hit both known observation
  flakes; both passed in isolation and the authoritative rerun is green
  at gate954 with Clippy/build pass (`markdown-svelte-grammar-final-*`).
  Full Shiki parity and live visual acceptance remain.
- The editor now supports Monaco's `Cmd+L` Expand Line Selection, also
  exposed in the command palette. Each invocation grows the primary and
  every extra selection from the starting line to the beginning of the
  following line, stops at the document end, and keeps a matching extra
  out of the primary. A pure range test and a rendered multi-cursor test
  cover repeated expansion. The first full run hit both known observation
  flakes; both passed in isolation and the authoritative rerun is green
  at gate955 with Clippy/build pass
  (`editor-expand-line-selection-final-*`). Live visual acceptance
  remains.
- The editor now supports Monaco's folding commands on every primary/extra
  selection start line: `Cmd+Alt+[` Fold, `Cmd+Alt+]` Unfold, `Cmd+K Cmd+L`
  Toggle Fold, `Cmd+K Cmd+[`/`Cmd+K Cmd+]` Fold/Unfold Recursively,
  `Cmd+K Shift+Cmd+L` Toggle Fold Recursively, `Cmd+K Cmd+0` Fold All and
  `Cmd+K Cmd+J` Unfold All, plus matching palette entries. Fold selects the
  innermost uncollapsed region or its first uncollapsed parent; unfold
  selects the innermost collapsed region or its first collapsed parent.
  Recursive variants apply the same state to the whole subtree. Pure target
  tests and a rendered keyboard/palette test cover nested folds, extra
  cursors and every command path. The first full run hit both known
  observation flakes; both passed in isolation and the authoritative rerun
  is green at gate957 with Clippy/build pass
  (`editor-fold-commands-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's remaining folding categories: `Cmd+K
  Cmd+/` Fold All Block Comments, `Cmd+K Cmd+8`/`Cmd+K Cmd+9` Fold/Unfold
  All Regions, and `Cmd+K Cmd+-`/`Cmd+K Cmd+=` Fold/Unfold All Except
  Selected, with matching palette entries. Language-aware folding now emits
  multi-line block-comment ranges for JavaScript/TypeScript, JSON/JSONC,
  CSS/SCSS/Less and HTML/XML, while `#region` markers keep their own ranges. All Except
  Selected preserves the innermost region at each primary/extra selection
  start together with its ancestors and descendants. Pure category tests and
  a rendered keyboard/palette test cover comments, regions, except-selected
  and the block-comment palette path. The first full run hit both known
  observation flakes; both passed in isolation and the authoritative rerun
  is green at gate959 with Clippy/build pass
  (`editor-fold-category-commands-final-*`). Live visual acceptance remains.
- Provider usage snapshot candidates now honor Source's `VELA_USAGE_SNAPSHOT`
  override. A non-empty override is the sole candidate; an empty value falls
  back to `XDG_DATA_HOME` (or `~/.local/share`) plus the macOS
  application-support path. The candidate builder is pure and tested for
  override precedence, XDG/HOME fallbacks, the macOS path and absent
  environment values. The first full run hit both known observation flakes;
  both passed in isolation. A later run hit the known engine-flow timeout,
  which also passed in isolation, and the authoritative rerun is green at
  gate960 with Clippy/build pass
  (`provider-usage-snapshot-env-final-*`). Live OAuth/key provider fetches and
  visual acceptance remain.
- Markdown fences now highlight Astro components. An opening `---`
  frontmatter block tokenizes TypeScript between comment markers, HTML
  tags/components and attributes reuse the HTML grammar, `<script>`/`<style>`
  bodies reuse the JavaScript/TypeScript and CSS grammars, and `{…}` template
  expressions paint their braces plus TypeScript tokens. Frontmatter, script
  and style braces stay untouched. Fixture and rendered fence tests cover the
  grammar. The first full run hit both known observation flakes; both passed
  in isolation and the authoritative rerun is green at gate962 with
  Clippy/build pass (`markdown-astro-grammar-final-*`). Full Shiki parity and
  live visual acceptance remain.
- Markdown fences now highlight Zig: line/documentation comments, storage,
  structure, control and type keywords, `fn` declarations/calls, uppercase
  and `@builtin` functions, escaped strings/chars, line-leading `\\`
  multiline strings, booleans/null, and hex/hex-float/decimal/octal/binary
  numbers with underscores. Fixture and rendered fence tests cover the
  grammar. The first full run hit both known observation flakes; both passed
  in isolation and the authoritative rerun is green at gate964 with
  Clippy/build pass (`markdown-zig-grammar-final-*`). Full Shiki parity and
  live visual acceptance remain.
- Markdown fences now highlight Solidity: line/block/NatSpec comments,
  pragma/import/control and declaration keywords,
  contract/interface/library/struct/enum/event/error/type names,
  functions/modifiers and calls, primitive-sized types, global
  variables/functions, time/currency constants, strings, booleans and
  decimal/hex/scientific numbers with underscores. Fixture and rendered fence
  tests cover the grammar. Clippy initially rejected two duplicate `else if`
  blocks; combining the conditions made Clippy/build pass. The first full run
  hit both known observation flakes; both passed in isolation and the
  authoritative rerun is green at gate966 with Clippy/build pass
  (`markdown-solidity-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight MDX: YAML `---` and TOML `+++` frontmatter
  tokenize their embedded language between string markers, line-leading
  `import`/`export` blocks reuse the TypeScript grammar, JSX/HTML tags and
  attributes are highlighted, Markdown structure/emphasis/links remain, and
  `{…}` expressions paint braces plus TypeScript tokens. Frontmatter, ESM and
  fenced-code braces stay untouched. Fixture and rendered fence tests cover the
  grammar. The full run passed on the first attempt at gate968 with
  Clippy/build pass (`markdown-mdx-grammar-final-*`). Full Shiki parity and
  live visual acceptance remain.
- Browser address normalization now follows Source's `browserAddress` decision
  order: explicit schemes are anchored to the start, host-like input requires
  dot-separated ASCII word/hyphen labels with an optional numeric port, and
  everything else becomes a DuckDuckGo search with
  `encodeURIComponent`-compatible escaping. Non-numeric ports and scheme-like
  prose now search instead of being rejected, while the existing
  credential-free HTTP(S)/`about:blank` policy remains enforced. The first
  full run hit the known watcher startup-gap flake; it passed in isolation and
  the authoritative rerun is green at gate968 with Clippy/build pass
  (`browser-address-parity-final-*`). Live/visual acceptance remains.
- The editor now supports Monaco's `Shift+Alt+I` Add Cursors to Line Ends,
  also exposed in the command palette. Non-empty selections are replaced with
  cursors at the end of every covered line; the final line gets its exact
  selection end when it sits past column 1, while collapsed selections are
  unchanged. Pure target tests and a rendered keyboard test cover multi-line,
  single-line, end-column and collapsed-selection cases. The first full run
  hit the known watcher startup-gap flake; it passed in isolation and the
  authoritative rerun is green at gate970 with Clippy/build pass
  (`editor-add-cursors-line-ends-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's palette-only Add Cursors to Top and Add
  Cursors to Bottom actions. They replicate the primary selection's start/end
  grapheme columns onto every line from the current line to the document edge,
  clamp columns on short lines, keep the current line primary and generate top
  cursors current-line first. Pure target tests and a rendered palette test
  cover bottom/top expansion, short-line clamping and multi-line column
  replication. The first full run hit the known watcher startup-gap flake; it
  passed in isolation and the authoritative rerun is green at gate972 with
  Clippy/build pass (`editor-add-cursors-top-bottom-final-*`). Live visual
  acceptance remains.
- The editor now supports Monaco's palette-only Add Selection to Previous Find
  Match action. A non-empty selection adds the nearest previous literal
  occurrence before the earliest current selection without wrapping; a
  collapsed caret first selects its word. Pure backward-search tests and a
  rendered palette test cover repeated leftward expansion, no-wrap and
  collapsed-caret behavior. Clippy initially rejected one-element range
  arrays; a `std::iter::once` helper fixed the tests. The first full run hit
  the known watcher startup-gap flake; it passed in isolation and the
  authoritative rerun is green at gate974 with Clippy/build pass
  (`editor-previous-find-match-final-*`). Live visual acceptance remains.
- The editor now supports Monaco's Move Last Selection to Next Find Match on
  `Cmd+K Cmd+D` and Move Last Selection to Previous Find Match in the palette.
  The latest selection moves forward to the next literal occurrence, or the
  earliest selection moves backward to the previous one, without wrapping. A
  moved primary follows its replacement and other selections stay intact.
  Pure forward-search tests and a rendered shortcut/palette test cover
  multi-selection moves, lone-primary moves, preserved later selections and
  collapsed-caret word selection. Clippy rejected one-element range vectors;
  a `std::iter::once` helper fixed the test setup. The first full run hit both
  known observation flakes; both passed in isolation and the authoritative
  rerun is green at gate976 with Clippy/build pass
  (`editor-move-find-match-final-*`). Live visual acceptance remains.
- The editor now exposes Monaco's Add Selection to Next Find Match as a
  command-palette action alongside `Cmd+D`. Repeated invocations add
  successive literal occurrences rightward without wrapping. A collapsed caret
  first selects its word, then the next invocation adds the next occurrence.
  The shared forward-search helper also backs Move Last Selection to Next Find
  Match. Pure forward-search/palette tests and a rendered palette test cover
  repeated expansion, no-wrap and collapsed-caret behavior. The first full run
  hit both known observation flakes; both passed in isolation and the
  authoritative rerun is green at gate977 with Clippy/build pass
  (`editor-add-next-find-match-final-*`). Live visual acceptance remains.
- Markdown fences now highlight Prisma: line/documentation/block comments,
  generator/datasource/model/view/enum/namespace/types blocks, `type` aliases,
  line-start field/config/enum names, scalar and upper-camel types, `@`/`@@`
  attributes, function calls, strings, booleans and numeric forms. Fixture and
  rendered fence tests cover the grammar. The full run passed on the first
  attempt at gate979 with Clippy/build pass
  (`markdown-prisma-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Handlebars: YAML frontmatter, HTML
  tags/attributes, `{{! }}`/`{{!-- --}}` comments, block start/end/else
  helpers, partials, variables and triple-stash expressions, helper
  attributes/values, strings and numbers. Fixture and rendered fence tests
  cover the grammar. The first full run hit the known watcher startup-gap
  flake; it passed in isolation and the authoritative rerun is green at
  gate981 with Clippy/build pass (`markdown-handlebars-grammar-final-*`).
  Full Shiki parity and live visual acceptance remain.
- Markdown fences now highlight Liquid: HTML, `{% %}` tags and `{{ }}`
  objects, comment/doc/raw blocks, embedded style/stylesheet/schema/javascript
  bodies, control keywords, `name:` attributes, `| filter` functions,
  variables, strings, numbers and operators. Fixture and rendered fence tests
  cover the grammar. The full run passed on the first attempt at gate983 with
  Clippy/build pass (`markdown-liquid-grammar-final-*`). Full Shiki parity and
  live visual acceptance remain.
- Markdown fences now highlight Blade: HTML, Blade comments, directives and
  `{{ }}`/`{!! !!}` echoes, PHP tags and embedded PHP expressions,
  `@php`/`@verbatim` blocks, directive arguments, PHP variables, keywords,
  strings and numbers. Fixture and rendered fence tests cover the grammar.
  Clippy/build pass. The first full run hit both known observation flakes;
  both passed in isolation and the authoritative rerun is green at gate985
  with Clippy/build pass (`markdown-blade-grammar-final-*`). Full Shiki parity
  and live visual acceptance remain.
- Markdown fences now highlight Templ: Go declarations/imports/control flow,
  `templ` declarations and function names, HTML tags/attributes/comments,
  `@Component`/`@pkg.Call` expressions, brace expressions, call expressions
  and Go types/strings/numbers. Fixture and rendered fence tests cover the
  grammar. The full run passed on the first attempt at gate987 with
  Clippy/build pass (`markdown-templ-grammar-final-*`). Full Shiki parity and
  live visual acceptance remain.
- Markdown fences now highlight Pascal: case-insensitive keywords/types,
  function/procedure/constructor/destructor/property names, `self`/`Result`,
  comments and compiler directives, apostrophe-escaped strings and `#nn` char
  codes, decimal/hex numbers, booleans and nil. Fixture and rendered fence
  tests cover the grammar. The first full run hit the known watcher
  startup-gap flake; it passed in isolation and the authoritative rerun is
  green at gate989 with Clippy/build pass (`markdown-pascal-grammar-final-*`).
  Full Shiki parity and live visual acceptance remain.
- Markdown fences now highlight Makefile: comments, variable assignments and
  operators, `$(...)`/`${...}` and automatic variables, built-in functions,
  directives, targets/prerequisites and shell recipes. Fixture and rendered
  fence tests cover the grammar. Clippy rejected a duplicate built-in property
  branch; removing it made Clippy/build pass. The first full run hit the known
  watcher startup-gap flake; it passed in isolation and the authoritative
  rerun is green at gate991 with Clippy/build pass
  (`markdown-makefile-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Twig: HTML, `{# #}` comments, `{% %}`
  statements and `{{ }}` print tags, block keywords, filters/functions, hash
  attributes, variables/properties, strings, numbers and raw/verbatim blocks.
  Fixture and rendered fence tests cover the grammar. Clippy/build pass. The
  first full run hit the known watcher startup-gap flake; it passed in
  isolation and the authoritative rerun is green at gate993 with Clippy/build
  pass (`markdown-twig-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight Pug: doctypes, comments, tags/classes/ids/
  attributes, includes/extends/blocks, mixins and calls, flow control, inline
  `#{}`/`!{}` interpolation, printed expressions and unbuffered JS. Fixture and
  rendered fence tests cover the grammar. Clippy rejected a single-element
  loop; simplifying it made Clippy/build pass. The first full run hit both
  known observation flakes; both passed in isolation and the authoritative
  rerun is green at gate995 with Clippy/build pass
  (`markdown-pug-grammar-final-*`). Full Shiki parity and live visual
  acceptance remain.
- Markdown fences now highlight INI/properties files: `#`/`;` comments,
  `[section]` headers, `key = value` assignments and quoted values. Fixture and
  rendered fence tests cover the grammar. The first full run hit both known
  observation flakes; both passed in isolation and the authoritative rerun is
  green at gate997 with Clippy/build pass (`markdown-ini-grammar-final-*`).
  Full Shiki parity and live visual acceptance remain.
- Agent TUI exits now match Source's panel notice behavior: `Agent TUI
  exited.` for success or a missing code, and `Agent TUI exited with code N.`
  for nonzero exits. The TUI terminal is removed, an empty terminal tray
  closes, and ordinary shell-tab exits remain silent. Pure copy tests and an
  app-level rendered exit test cover both TUI and shell tabs. The first full
  run hit both known observation flakes; both passed in isolation and the
  authoritative rerun is green at gate999 with Clippy/build pass
  (`agent-tui-exit-final-*`). Live visual acceptance remains.
- Editor tabs can now be dragged between editor groups, matching Source's
  `moveTabToGroup`: the tab leaves its previous extra-group assignment, the
  destination group shows it, the destination editor takes focus, and moving
  the active main-group tab promotes the next unassigned tab. Pure assignment
  tests and a rendered drag test cover extra/main moves and invalid
  destinations. The full run passed on the first attempt at gate1001 with
  Clippy/build pass (`editor-tab-group-drag-final-*`). Live visual acceptance
  remains.
- Inactive tiled agent panels now expose expandable tool/shell rows with
  Source-style summaries, input/output payloads, status and a transient Copy
  action. Expansion and copied state are per panel, and inspecting a row does
  not steal the active panel. A pure preview-index test and a rendered
  expand/copy test cover the path. The first full run hit both known observation
  flakes; both passed in isolation and the authoritative rerun is green at
  gate1002 with Clippy/build pass (`inactive-tile-tool-actions-final-*`).
  Nested sub-call expansion and full transcript-wide actions in inactive tiles
  remain, along with live visual acceptance.
- Inactive tiled composer now keeps independent attachments, `@` file
  references and slash menus. Each tile stages/removes attachment chips,
  searches references and inserts `@path` with mention offsets, filters slash
  commands and command arguments with keyboard/click selection, and sends its
  own prompt files or slash command without borrowing the active composer.
  A rendered test covers attachment removal, reference insertion, argument
  completion and slash submission. The first full run hit one known observation
  flake; it passed in isolation and the authoritative rerun is green at gate1003
  with Clippy/build pass (`inactive-tile-composer-final-*`). Full rich
  transcript controls, same-root editor isolation and live visual acceptance
  remain.
- Inactive tile tool rows now expose file links and recursively expandable
  nested tool calls. Each nested call shows its own input/output and files,
  keeps independent expansion state keyed by panel and call path, and does not
  steal the active panel. A rendered test covers top-level file rows, nested
  expansion, recursive expansion and per-panel isolation. The full run passed
  on the first attempt at gate1003 with Clippy/build pass
  (`inactive-tile-nested-tools-final-*`). Transcript-wide actions beyond tool
  rows, same-root editor isolation and live visual acceptance remain.
- Same-root tiled panels now keep independent editor split-group assignments.
  Editor groups are keyed by the activation-owned document workspace instead of
  one global vector, so renaming/closing documents update only the owning
  panel's layout and a panel switching through a sibling on the same physical
  root returns to its own tabs and panes. A rendered test opens `a.txt` and
  `b.txt` in two scopes of `/workspace`, splits panel A, switches to B (no split
  group), and switches back to A with its group and active document restored.
  The first full run hit the two observation flakes and the replay flake; all
  passed in isolation and the authoritative rerun is green at gate1004 with
  Clippy/build pass (`same-root-editor-isolation-final-*`). Live visual
  acceptance remains.
- Inactive transcript rows now expose message Copy, Undo and Edit plus
  collapsible work groups for consecutive reasoning/activity/system rows. Copy
  uses the owning message text with a transient copied label and no focus
  steal; Undo/Edit focus the owning tile and route the revert request to its
  session, with Edit carrying the original prompt. Work groups auto-open while
  running and otherwise expand/collapse per panel. A rendered test covers all
  three message actions and work expansion. The first full run hit the two
  observation flakes; both passed in isolation, the rerun hit the known
  streaming-pacing flake, it passed in isolation, and the authoritative rerun
  is green at gate1005 with Clippy/build pass
  (`inactive-tile-transcript-actions-final-*`). Inactive answer copy,
  forms/revert docks and live visual acceptance remain.
- Inactive tiles now expose answer Copy, independent pending form docks and
  staged revert docks. Answer copy writes the assistant text without stealing
  focus. Each panel owns a `FormDock` fed by its own forms snapshot; submitting
  or dismissing first focuses the owning tile, then routes the reply/cancel to
  that session. Revert commit/discard target the panel session directly. A
  rendered test covers all four paths. The first full run hit the known
  watcher-readiness flake; it passed in isolation and the authoritative rerun is
  green at gate1006 with Clippy/build pass
  (`inactive-tile-answer-forms-revert-final-*`). Live visual acceptance remains.
- Revert now matches Source's disabled/in-flight semantics. Undo/Edit are
  silent no-ops while a run is active or any revert request is pending, and
  starting either action shows the `Undoing prompt…` / `Restoring prompt for
  editing…` notice. Commit/discard dedupe duplicate activation and dim while a
  request is in flight; an empty transcript still renders its staged dock. A
  focused rendered test covers the busy no-op, both notices, duplicate
  commit/discard requests, disabled dock controls and the empty-transcript dock.
  The full run passed on the first attempt at gate1007 with Clippy/build pass
  (`revert-busy-inflight-final-*`). Live/visual acceptance remains.
- Accessibility gained platform-aware table header descriptions and shared
  keyboard focus outlines. Header cells retain the `ColumnHeader` role,
  row/column indexes and table counts while adding a `Column header`
  description, because AccessKit's macOS adapter maps both header roles to plain
  cells. Transcript action controls (tool heads, copy/copy-answer, user actions,
  scroll-to-bottom and delegated cards) now use the shared focus outline;
  explorer directory rows and the root toggle expose expanded state. The table
  role/description contract is covered by the existing table accessibility test.
  The full run passed on the first attempt at gate1007 with Clippy/build pass
  (`accessibility-table-focus-final-*`). Live VoiceOver/platform focus
  acceptance remains.
- The empty Files sidebar now offers Source's `Open Workspace` action instead
  of only a dead-end message. It opens the same folder picker as New Session and
  carries a short "Choose a folder to start a session" hint; the empty editor
  surface already uses Source's workspace/file-specific copy. A rendered test
  covers the empty-profile action. The full run passed on the first attempt at
  gate1008 with Clippy/build pass (`workspace-empty-a11y-final-*`). Live
  empty-profile screenshot acceptance remains.
- Slash text is now interpreted as a command only when the runtime advertises
  the command capability. With commands unavailable, `/compact` and similar text
  are sent as ordinary prompts; with commands available they route through the
  command API. A pure test covers the gate and a rendered send test verifies both
  paths. The first full run hit the two observation flakes; both passed in
  isolation, the rerun hit the known replay flake, it passed in isolation, and
  the authoritative rerun is green at gate1010 with Clippy/build pass
  (`slash-capability-gating-final-*`). Structured command arguments and live
  menu acceptance remain.
- Mixed launch batches now serialize around asynchronous directory activation.
  A file queued after a directory waits for `WorkspaceOpened`, then opens as a
  tab in that newly active context instead of racing it as a standalone
  workspace; file-only batches still stop at the single-file workspace request.
  A rendered test covers directory-then-file ordering. The full run passed on
  the first attempt at gate1011 with Clippy/build pass
  (`mixed-launch-batch-final-*`). Live second-instance routing and physical OS
  drag acceptance remain.
- Active user messages now render Source's right-aligned identity metadata:
  the agent is capitalized, the model id resolves through the catalog to its
  display name, and fields join with Source's non-breaking separators before
  Copy/Undo/Edit. A pure metadata test covers resolution and missing fields; a
  rendered row test asserts the metadata surface. The first full run hit the
  known watcher-readiness flake; it passed in isolation and the authoritative
  rerun is green at gate1013 with Clippy/build pass
  (`user-message-meta-final-*`). User timestamps, text selection and the
  complete context-menu matrix remain.
