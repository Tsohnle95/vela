# Architecture decisions

## ADR-001 — A separate repository, non-destructive by default

**Decision:** `vela-native` is a sibling Git repository. The original checkout is
read-only during development and migration.

**Why:** the Electron app remains the behavioral oracle and must keep running
for comparison. Migration must never rewrite OpenCode's database or the
Electron profile.

**Consequences:** shared code is avoided unless its license is explicit. The
root Vela repository has no `LICENSE`, so only design tokens/facts and
independently written logic are used. If the upstream license is clarified,
portable logic can be reconsidered.

## ADR-002 — GPUI from the Zeron fork, pinned

**Decision:** depend on `zeronsh/zui` at
`361ff20b919fdfb80199ee6d9aed725300cb7d54`.

**Why:** the fork provides GPU-memory bounding, bounded backdrop blur,
`EdgeFade`, paint-time rounded clipping, transparent-window alpha fixes, and
macOS 26 blur behavior used by the reference implementation. Upstream GPUI
lacks some of these primitives. The SHA is exact and recorded in `Cargo.lock`.

**Consequences:** no shared Git history with upstream; upgrades are manual
patch ports. If the fork stalls, reverting to upstream is a contained change in
the UI crate, at the cost of those visual primitives. The `path` crate's GPL
license is in-tree but unused; GPUI itself is Apache-2.0.

## ADR-003 — Block-granular transcript, not message-granular

**Decision:** project messages into stable top-level blocks and virtualize rows.

**Why:** a single assistant message can contain thousands of lines. Rendering
it as one element defeats virtualization and makes layout scale with content.
Blocks let completed paragraphs keep identity while a streaming tail changes.

**Consequences:** block identity must remain stable across completion. Inline
Markdown styling is painted within a block; it does not change block
boundaries.

## ADR-004 — Engine-owned state with scoped notifications

**Decision:** the Tokio engine owns network, projection, permissions, and warm
state. It publishes `EngineUpdate` notifications; the GPUI entity reads
`Arc<SessionView>` and repaints only when its active revision changes.

**Why:** a single global invalidation would make eight streaming sessions
repaint one another. Per-session locks and revision pulls isolate cost.

**Consequences:** UI state that is purely visual (focus, list offset) can live in
the view; durable/shared state lives in the engine. Draft and scroll anchors are
mirrored to the engine for warm eviction.

## ADR-005 — Native OpenCode HTTP/SSE adapter, no Node sidecar

**Decision:** implement the documented V2 endpoints and SSE with `reqwest`.

**Why:** removing Node from the runtime is a core objective. A sidecar would
reintroduce the process and packaging complexity we are removing.

**Consequences:** protocol drift is our responsibility. Parsing is tolerant of
current and legacy envelopes. An integration test runs the full adapter against
an in-process mock service. Live validation used OpenCode 2.1.4.

## ADR-006 — Warm cache limited by count and bytes

**Decision:** default 6 warm sessions and 96 MiB, LRU, with the active session
exempt from eviction.

**Why:** MonoCode's warm-navigation principle (bounded retention, immediate
visual restore) translated to native state. A count-only limit lets one huge
transcript consume unbounded memory.

**Consequences:** eviction is O(entries), never on the render thread. Estimates
are conservative (UTF-16-style 2 bytes/char plus structure overhead) and are
reported in the sidebar.

## ADR-007 — Settings in atomic JSON; conversations stay with OpenCode

**Decision:** Vela Native stores only its own preferences, in a schema-versioned
JSON file written via temp file + rename. It does not copy transcripts or
credentials.

**Why:** duplicate ownership creates divergence and migration risk. OpenCode is
already the durable conversation owner.

**Consequences:** offline browsing of history requires OpenCode to be reachable.
Future metadata caching, if added, must be derived and disposable.

## ADR-008 — Native platform web view for the browser feature

**Decision:** keep browser chrome in GPUI and host only page content in
WKWebView/WebView2/WebKitGTK.

**Why:** Vela's browser feature must survive, but embedding a whole web UI shell
would repeat Electron's cost. A platform guest preserves real compatibility
(cookies, devtools, localhost) without giving page content a Vela bridge.

**Status:** implemented with a single WKWebView guest; multi-tab/profile
selection remains open. No page content is exposed to engine commands, and any
future agent-driven automation gets a separate, explicitly authorized surface.

## ADR-009 — `alacritty_terminal` + `portable-pty`

**Decision:** use Alacritty's mature terminal state machine and a portable PTY.

**Why:** terminal correctness (alternate screen, wide chars, escape sequences)
is expensive to reimplement and easy to get subtly wrong.

**Status:** implemented as a single native dock (PTY + ANSI grid); per-workspace
trays and tabs remain open.

## ADR-010 — Plain-text inline Markdown in the first slice

**Decision:** ship block detection and plain text rendering before inline
styling/syntax highlighting.

**Why:** block boundaries and virtualization are the performance-critical
parts; inline styling is paint-only and can land without changing layout
ownership.

**Consequences:** visual parity is incomplete (no bold/italic/inline code,
links, or highlighting). This is recorded as a parity gap, not treated as
complete.

## ADR-011 — No `unsafe` unless a platform requirement demands it

**Decision:** deny/review unsafe in owned crates. Platform integration (Wry,
WebKit) may require documented unsafe blocks at the boundary.

**Why:** safety and auditability. The browser profile boundary now contains one
documented WebKit configuration block: `MainThreadMarker` establishes thread
ownership, retained objects establish lifetime, and each tab gets a separate
configuration sharing only an explicitly nonpersistent website data store.

## ADR-012 — Reject Electron, Tauri, and a browser UI shell

**Decision:** GPUI native rendering is the presentation layer. A web view is
allowed only inside the embedded browser feature.

**Why:** startup, idle CPU, memory, and input latency targets; the assignment
explicitly forbids a browser-based shell.

## ADR-013 — Codex shell is the default; Classic remains selectable

**Decision:** ship the flat Codex shell as the default and retain the original
floating-card shell as an explicit Appearance option, persisted in
`settings.json` as `"shell": "codex" | "classic"`.

**Why:** the Codex shell is the current Vela interface and the visual reference.
Deleting the Classic implementation would strand users who prefer the original
chrome and would lose validated layout code. The shells share state and engine
commands; only layout/chrome differ.

**Consequences:** the Codex shell uses a 50px activity rail, one flush content
card, hairline-free left/top panel seams, a 44px titlebar, and a docked agent
panel. Classic uses the floating rounded panels with 10px gutters. The
Appearance selector writes the choice atomically; both directions are verified
in the running app. New surface work must be checked in Codex first and remain
usable in Classic.

## ADR-014 — Retained browser tabs and a session-only partition

**Decision:** each browser tab retains its own Wry webview, history, title and
loading state. On macOS, an application-owned `BrowserProfile` supplies the
same nonpersistent WebKit store to separate tab configurations.

**Why:** reloading one shared webview on tab switches loses form state and
history. Giving every tab a distinct incognito store also loses the original
app's shared session cookies. The original `vela-browser` partition is shared
across tabs and never persists to disk.

**Consequences:** hiding a tab preserves its native view; closing it drops the
view. The partition survives closing the last tab until the application exits.
Platform focus is checked before GPUI handles clipboard shortcuts; Cocoa edit
selectors handle the focused guest, and Undo/Redo use its native undo manager.
No page-to-engine IPC bridge is introduced. Windows and Linux behavior still
requires platform validation.

## ADR-015 — Workspace document ownership and recoverable writes

**Decision:** a workspace/path key owns its document independently of session
views. Buffers retain selection and delta undo history; each editor view owns
focus, wrapped-line geometry and its virtualized scroll list. Read and save
responses are matched to the issuing request and document.

**Why:** replacing the active buffer on every read discarded unsaved work.
Marking current text clean on a late save acknowledged content never written.
Rejecting a late acknowledgement alone also cannot prevent racing disk writes.

**Consequences:** writes for a document dispatch in order and acknowledge exact
snapshots. Recoverable writes hold the source, compare expected content, and
install with an exclusive hard link. Durable manifests retain the original and
an independent proposal, including on failures. Interrupted source holds are
reconciled on workspace initialization. Explicit overwrite preserves the held
external version. Conflicts pause ordinary saves and offer the reference
reload/keep-editing/overwrite controls. Watchers, unsaved journals, recovery UI,
retention and full close/relaunch handling still require completion.

## ADR-016 — Rendered native editor regressions

**Decision:** enable GPUI `test-support` only as a development dependency. Its
transitive proptest source remains pinned by GPUI to
`3dca198a8fef1b32e3a66f1e1897c955b4dc5b5b`; all resolved packages are locked.

**Why:** buffer unit tests cannot detect an input viewport placed below the
editor or a caret that never becomes visible after a document-end command.

**Consequences:** the large-file regression dispatches keyboard input and paints
the native editor, verifies the last caret is inside its viewport, and checks
that offscreen rows remain unrendered. Reintroducing the missing viewport
anchors reproduces the failure; restoring them passes. Live macOS interaction
checks remain necessary for platform focus, text composition and rendering.

## ADR-017 — Shared native surface and Settings geometry

**Decision:** use GPUI rounded paint scopes around composed surfaces. Codex
normal, sidebar-hidden and Settings states share one content-card builder;
Classic keeps its separate 16-point panels. Keep Settings shell/page sizing and
row primitives in `settings.rs`, with a rendered viewport regression.

**Why:** rounded backgrounds do not clip opaque descendants. Separately copied
shell geometry also allowed Settings to overflow the window by the rail width.

**Consequences:** original SVG icon geometry and the two-path Vela mark replace
font approximations in Settings. Its source radial gradient uses GPUI's native
SVG image decoder at the measured backing scale, with no browser UI shell. The
layout regression fails when the shared width constraints are removed and
passes after restoration. Paired real-app captures remain the acceptance gate;
functional and layout tests do not establish visual identity.

## ADR-018 — Native controls and shared window appearance

**Decision:** GPUI owns the app layout; public AppKit popup cells provide macOS
select-menu tracking, keyboard behavior and menu accessibility. The selected
Vela theme also sets the application's native appearance. Use the behavior of
the reference's actual Chromium version when matching its menus.

**Consequences:** menus run after releasing GPUI entity/app borrows because
AppKit enters a nested tracking loop. System selection colors are resolved
under the control's appearance. Neither a native menu nor a host regular-
expression evaluator introduces a browser app shell. JavaScriptCore's public
C API validates full-match HTML patterns with Unicode sets; it receives
JSON-encoded text/pattern data and has no DOM, UI or network bridge. If its
required syntax is unavailable, validation reports unavailability rather than
silently accepting a value. Other platforms remain unaccepted.

## ADR-019 — Session window dimensions belong to the native profile

**Decision:** persist normal session dimensions in a separate version2
`window-bounds.json`. GPUI debounces400ms; the engine serializes disk writes.
Do not store positions or maximized/minimized/fullscreen dimensions. Read-only
legacy import seeds this file once without replacing an existing native file.

**Consequences:** the ordinary app restores its dimensions or uses1280×800;
the comparison launcher can override its viewport or explicitly exercise the
saved dimensions. Storage/engine tests and live zoom/relaunch checks supplement
the still-required full resize/display-mode interaction acceptance.

## ADR-020 — User-specified flat inner Settings join

**Decision:** per the user's 2026-10-26 screenshot correction, Codex Settings
navigation and content meet flush inside their one shared outer rounded card.
Remove the page's redundant inner rounded border, clip and shadow. Classic
keeps its separate floating cards. This explicitly overrides the original
reference's rounded inner Settings page at that join.

**Validation:** all seven native Settings sections were inspected live, with
lossless before/after and section captures. The shared rendered layout check
and Clippy pass. This targeted correction does not accept a whole page.


## ADR-021 — Vite remains an app-owned project tool

The native engine owns preview target selection, process lifetime, loopback
ports, readiness, diagnostics and inventory. Vite6.4.3 and Sass1.102.0 retain
the reference's project-preview behavior. A pinned, checksum-verified official
Node22.23.2 executable and locked npm packages ship as preview resources,
launched only on request. They supply project tooling, with no UI shell or
OpenCode bridge; the core UI and transport remain Rust. No global install or
reference checkout is required. Preview process groups are terminated on stop,
workspace close and application shutdown. Production signing/notarization of
the complete bundle remains part of release acceptance.

AppKit quit does not necessarily return from GPUI's application loop. The
application-level quit hook invokes engine shutdown before yielding to GPUI's
bounded shutdown wait. Unix preview groups have synchronized registration and
closing so children cannot outlive quit, including helpers that ignore SIGTERM
and concurrent late starts. Engine completion supplements the synchronous
termination path. A real bundled preview/terminal quit check and a process-tree
regression protect this platform boundary.
