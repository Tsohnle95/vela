# GPUI presentation layer (`vela-ui`)

**Owns:** the native window and render tree, VelaApp state, shell/layout,
simultaneous agent panels, session sidebar, transcript/composer presentation,
Markdown rendering, settings/provider surfaces, terminal and browser chrome,
native text input, focus/keyboard routing, toasts, and accessibility semantics.

**Does not own:** network/disk/PTY work, transcript projection, service
discovery, durable persistence formats, file recovery, terminal escape
interpretation, or WebView policy. Those are engine/core/adapter/platform
responsibilities. Known UI persistence debt is listed below.

The editor/explorer surface is large enough to have its own owner:
[editor.md](editor.md).

## Entry points

| Entry point | Responsibility |
|---|---|
| `vela_ui::run` | creates the GPUI application/window and mounts `VelaApp` |
| `VelaApp` (`app.rs`) | central UI state, engine subscriptions, actions, render orchestration |
| `impl Render for VelaApp` | shell composition and panel/window layout |
| `EngineUpdate` handling | pulls engine snapshots and updates UI-owned state |
| `scripts/run-parity.sh` | isolated QA launch used for visual checks |

`app.rs` is a large composition root. Prefer a focused helper module when
adding behavior; do not grow the central match/render path unless the behavior
is genuinely shell-wide.

## Shell and panel map

| Area | Modules | Responsibility |
|---|---|---|
| Shell/theme | `theme.rs`, `surface.rs`, `icons.rs`, `typography.rs`, `pixel_art.rs` | tokens, rounded/clipped surfaces, original vector glyphs, shell chrome |
| Panels | `panel_layout.rs`, `panels.rs` | 1–8 panel grid geometry, activation, per-panel composer/scroll snapshots |
| Sidebar | `session_sidebar.rs` | pinned/recent sessions, project history, filtering, context menus |
| Settings | `settings.rs`, `model_settings.rs`, `preview_settings.rs` | shared Settings layout, appearance/model/preview sections |
| Window/lifecycle | `window_state.rs`, `close.rs`, `workspace_close.rs`, `platform_quit.rs` | window-size measurement/restore request, dirty-close/quit coordination |
| Platform | `platform_menu.rs`, `platform_prompt.rs`, `platform_quit.rs` | AppKit menus, file/folder panels, native termination |
| Feedback | `toasts.rs`, `shimmer.rs`, `focus.rs`, `scroll.rs` | transient feedback, running shimmer, focus outlines, auto-scroll |
| Input | `input.rs`, `multiple_selection.rs` | native text input/IME ranges, clipboard, undo, multi-select |

Codex is the default shell; Classic remains selectable in
**Settings → Appearance**. Both shells render the same state and commands. New
surfaces must be checked in Codex first and remain usable in Classic.

Panels retain independent session, transcript, composer, focus, scroll, forms,
permissions, and revert state. Same-root panels intentionally have separate
activation scopes, document buffers, tabs, selection/undo, and split groups;
never share editor panes merely because two panels point at the same directory.
Agent Mode gives the tiled grid the workspace beside the rail. The grid rows
divide the height available below the tabs, and each virtual transcript fills
the space above its panel's composer and interaction docks.

## Agent conversation surfaces

| Area | Modules | Responsibility |
|---|---|---|
| Transcript | `transcript_view.rs`, `transcript_viewport.rs`, `turn_anchor.rs` | shared virtualized rows, chronological activity, actions, copy/undo/edit, revert docks, reading and follow state |
| Tools | `tool_view.rs` | summaries, expandable input/output, shell/patch bodies, files, nested calls, copy, status |
| Markdown | `markdown_view.rs`, `inline_flow.rs`, `markdown_images.rs`, `math.rs` | native block/inline layout, tables, code, links, images, accessible text |
| Composer | `composer.rs`, `attachments.rs`, `model_picker.rs`, `slash_args.rs` | prompt input, model/agent/strength pickers, attachments, `@` refs, slash commands |
| Queue | `inbox.rs` | runtime inbox + durable local queue, steer/cancel/edit/retry/reorder |
| Forms | `forms.rs` | pending form dock; answers survive periodic refresh |
| Providers | `providers.rs`, `provider_oauth.rs`, `provider_usage.rs`, `provider_validation.rs` | credential forms, OAuth state, usage cards, field validation |
| Subagents | `subagent.rs` | child-session presentation and navigation |

Rendering invariants:

- Focused and inactive panels share a row projector and renderer. Prose stays
  in source order when tools or reasoning arrive. Details expand in place and
  answer footers follow message completion.
  Each work sequence between explanations has one compact disclosure line from
  its first activity, keyed by that activity for its lifetime. Reasoning and
  tools share that sequence across internal step/message boundaries. Default
  text is human (Read files, Edited files, Ran checks). Expanded work shows
  compact tool entries with workspace-relative paths and individual input/output
  disclosure. Internal thought headers and commentary copy footers stay out of
  mixed tool lists. Reasoning-only sequences can still be inspected. Headers and
  leaves stay siblings while collapsed; disclosure reveals existing leaf identities.
  The active summary paints one shaped label with a moving highlight,
  including between calls in the current work sequence. Changing the caption
  preserves the sweep position. Completed summaries and reduced motion use
  the same label without animation.
  Successful local work folds; failures, warnings, delegates and consequential
  operations stay surfaced. Distinct unresolved delegates never merge by
  matching descriptions; ambiguous navigation waits for runtime identity.
- Lists splice by stable row identity and invalidate only changed measurements.
  Prose rows and delegated agents have independent native view boundaries.
  Reuse uses actual list-measured sizes, retains them when temporarily offscreen,
  and invalidates on source/Markdown revision, UI interaction, explicit remeasure
  or layout/theme changes. A growing answer tail remains naturally measured.
  While a turn reservation is active, the final row uses natural measurement:
  its native list box includes borrowed space, which must never become a
  cached content height after more content arrives.
  The focused transcript reserves a fixed, symmetric scrollbar gutter.
  Overflow changes thumb visibility without moving or rewrapping the text.
  Wheel and thumb share the padded scroll range. Scrolling preserves settled
  row measurements, and the idle return-to-bottom animation runs on native
  window frames. Discrete mouse-wheel ticks ease across native frames in the
  receiving panel. Precise scrolling and reduced motion use GPUI's immediate
  scroll path. Precise input, keyboard navigation, or a scrollbar gesture
  supersedes queued wheel motion.
  A reader keeps their row and pixel offset where possible. Follow uses GPUI's
  native tail mode. A matching newly observed canonical prompt gets a temporary
  viewport reservation that a growing answer consumes; optimistic queue rows
  cannot start it. Removing or editing a queued prompt disarms pending matches
  for that text in its session, including ambiguous identical queued entries.
  Matching is best effort without a runtime request identifier.
- Streaming Markdown fences stay plain until settled. Newly appended prose
  starts legible and fades on the native frame clock. Bursts spread their
  reveal across a bounded interval. An in-flight fade can finish when the
  answer settles. Opacity changes paint only, leaving wrapping, selection,
  and scroll geometry intact. Reduced motion, replayed prose, and code paint
  immediately.
- Inline layout reuses unchanged shaped runs in the same window and element.
  Appends reuse the already-shaped prefix only when the bytes, style, and
  recomputed line-break boundaries still match. The derived cache is bounded
  independently of source and parser retention; offscreen glyphs are not painted.
  Probes with no inline width defer text wrapping until layout supplies a
  positive width.
  Oversized plain ASCII runs use native wrapping while preserving their full
  text. Repeated plain table cells share intrinsic measurements within a table.
  When a large plain assistant table's intrinsic columns fit, it keeps its
  full height and materializes only the visible rows. Wrapped, rich and
  selectable user tables retain their general renderer.
- Adjacent session invalidations are batched in a bounded foreground update.
  Each visible panel reads the latest engine snapshot; other events and lag
  recovery retain their delivery order.
  The session-owned viewport reuses row identifiers and updates measurements
  directly when the row sequence is unchanged or changes only at the end.
- External Markdown links open only credential-free HTTPS; other schemes stay
  in-app.
- Running/tool status must use the shared transcript semantics, not local
  guesses.
- Disclosure, code controls, keyboard navigation, selection, and scroll intent
  belong to their panel. Background updates do not activate or scroll another
  panel. Explicit latest navigation resumes following. Closing a panel records
  its workspace distance-from-bottom. Opening reserves that distance for the
  new session and applies it once its list has measured scrollable content.
  Explicit scroll intent supersedes a pending restore.
  User wheel/scrollbar or End navigation back to the actual live edge resumes
  that panel's following; passive geometry changes cannot resume it. Edge
  detection includes list padding so a one-pixel upward scroll releases follow.

## Terminal and browser chrome

- **Terminal:** `terminal_state.rs` owns an ordered per-workspace tab set;
  `terminal_tray.rs` owns drag/snap/hide geometry; `terminal_input.rs`,
  `terminal_selection.rs`, and `terminal_composition.rs` own key encoding,
  selection/copy, and IME isolation. The emulator/PTY live in
  `vela-terminal`; the engine relays bytes. Do not move escape parsing into
  GPUI.
- **Agent TUI:** `agent_tui.rs` owns the agent panel's session-keyed GUI/TUI
  mode and a dedicated embedded terminal registry. It is independent of the
  tray tab set, runs the runtime CLI with the session/theme contract, and
  reuses the shared grid, selection, clipboard and IME helpers. Persisted
  `Settings::agent_panel_modes` is the source of truth; switching to GUI,
  closing a panel or process exit stops the embedded PTY.
- **Browser:** `browser_downloads.rs` owns bounded persisted download history;
  `BrowserHost` in `vela-browser` owns WKWebView tabs. The native chrome owns
  address/history/loading/title, downloads shelf, and focus routing. No JS
  bridge or page-to-engine command surface exists.

Both surfaces are live-validation-sensitive: passing rendered unit tests does
not establish WKWebView isolation, IME behavior, or terminal/TUI appearance.

## Settings and provider state

Settings is an engine-owned persisted snapshot rendered by native controls.
The preferred path is an explicit apply command plus the engine's success/error.
Provider forms are typed by the catalog and validated before submission. OAuth
leases and credential removal are engine-owned; never infer success from a
local click.

Known persistence debt: the UI currently calls `Settings::save` after updating
recent-session bookmarks and writes `browser-downloads.json` synchronously on
the GPUI thread. These are existing exceptions, not the target architecture. Do
not extend them; move new persistence behind engine commands and blocking work
off the render thread.

Runtime capabilities gate controls that the connected runtime does not
advertise (for example steering, commands, attachments, agents, or TUI).
Unsupported controls must be disabled or absent, not silently fail.

## Input, focus, and accessibility

- Native text fields use the shared `input.rs` path for selections, clipboard,
  undo/redo, platform composition ranges, and UTF-16 accessibility ranges.
- Focus outlines are CSS-compatible and must remain visible outside the
  control border box.
- Interactive controls need roles, labels, expanded/selected/disabled state,
  and Enter/Space activation where applicable. Transcript tables, lists,
  images, code, explorer directories, and toolbar buttons have explicit
  semantics.
- Keyboard routing must not be intercepted by the native composer/address field
  when the guest WebView or terminal owns input.
- Accessibility acceptance is incomplete; live VoiceOver/platform focus remains
  in the parity matrix.

## Cross-boundary flows

| UI action | Engine/adapter path | State returned |
|---|---|---|
| Send/queue prompt | engine command → OpenCode client → SSE → core projection | `Session` update + transcript snapshot |
| Open/reopen session | engine open/promote warm → paged history | `Sessions`/`Session` |
| Edit/save file | `Documents` → engine file command → recovery/watcher | `FileSaved`/conflict/directory update |
| Apply settings | engine apply → normalize + atomic save | `SettingsApplied` |
| Connect provider | typed form → engine OAuth/key lease → client | provider/update events |
| Terminal key | engine → PTY → relay → emulator | `TerminalData` |
| Browser navigate | `BrowserHost` → WKWebView | browser events + native state |

## Tests

```sh
cargo test -p vela-ui
```

Inline tests cover text input, focus, panels, transcript/tool rows, Markdown
tables/links/images, composer/pickers, queue, forms, providers, settings,
terminal encoding/selection/composition, browser URL policy/downloads, and
rendered geometry regressions. These are behavioral/rendered tests, not live
window, VoiceOver, WKWebView, IME, or visual-parity acceptance.

## Neighboring owners

- Domain projection and Markdown boundaries: [core.md](core.md)
- Engine commands/state: [engine.md](engine.md)
- Editor/explorer/details: [editor.md](editor.md)
- Browser/terminal protocol crates: see `crates/vela-browser` and
  `crates/vela-terminal`
- Isolated launch/capture procedure: [development.md](../development.md)
