# Migration checkpoint: still in progress

No page has passed visual acceptance. The native application remains incomplete.
The current reference is the original Vela checkout at
`3157a41357f28137874d7298af6177e657df5afc`. Its application source tree is
unchanged from `f25a025f1d19acc26e20dfd3a0d22a1363155804`; the intervening
commit changes design reference artifacts. Existing capture manifests retain
the revision and source hashes recorded at capture time.

Both applications ran with the isolated OpenCode service on port 49555 and the
same disposable `/private/tmp/vela-test-workspace`. Native preferences remain
isolated in `/private/tmp/vela-parity-native`; the reference uses
`/private/tmp/vela-parity-original`. The original checkout and profile are not
migration write targets.

## Live editor checks

- Unicode paste, mouse/keyboard selection, native clipboard, undo/redo, retained
  dirty text across file switches, preview/pinned tabs and exact snapshot saves.
- A 6,001-line source fixture with a long wrapped Unicode line. Document-end
  navigation reaches its final line immediately; paired start/end PNGs record
  the same file and caret endpoints in the two real applications.
- Closing a dirty tab saves it before removal. Disk content and the completed
  recovery manifest were checked after the tab disappeared.
- An external write before save is preserved. Both the editor proposal and held
  external source remain in recovery; the tab stays open with a conflict.
- Keep editing to merge switches to Save merged content. Ordinary Cmd-S remains
  paused; explicit merged save writes the edited buffer and preserves the
  external source in recovery.
- Reload disk version replaces the buffer. Undo restores the previous local
  edits; redo restores the disk version.
- A deleted file pauses saving without clearing the editor. Explicit Overwrite
  disk recreates it; the independent recovery proposal was verified on disk.

These tests used only disposable fixture files. Watch-driven updates, autosave,
unsaved journaling, full window-close protection, recovery management, split
editors and language tooling still require completion.

## Rendered regression

`editor-regression.txt` records a deliberate reintroduction of the missing input
viewport anchors. The rendered keyboard/layout test fails with the caret outside
its input viewport, then passes after the fixed source is restored. It also
asserts that fewer than 200 rows are painted from the 6,001-line document.

## Lossless comparison

The user authorized macOS window captures on 2026-10-25. New PNGs come from
`scripts/capture-parity.sh`, using `screencapture -x -o -l <window-id>` without
activating, moving, resizing, cropping or masking a window. Each adjacent JSON
manifest records window identity/bounds, point/pixel size, source revision,
tracked-diff hash, image hash and native binary hash.

The displays initially had different backing scales (1280×800 versus 2560×1600
rasters for equal point sizes). Native was relaunched on the reference display;
paired captures are now 2560×1600. Neither capture is rescaled for comparison.
The comparator includes all RGBA channels, reports missing counterparts and
rejects a JPEG as proof of a lossless match, even with a misleading PNG suffix.
Older automation captures under `2026-10-24` include JPEG data and are historical
implementation evidence, not lossless acceptance.

The large-file start/end pairs differ. Shared discrepancies in those earlier
captures include the native bottom inset, editor header/gutter geometry, file-tree layout/status/icons,
source-font metrics, transcript reasoning/tool presentation, composer and
chrome. Additional editor capabilities are absent. Functional checks do not
establish visual identity. The current comparison report shows zero lossless
pixel matches; unpaired functional proof captures remain explicitly unmatched.

## Shared shell and Settings continuation

The reported square top-left corner is repaired with a native rounded paint
clip. Codex normal, sidebar-hidden and Settings views share one 10-point card;
the inner clip preserves its curved hairline. Classic retains separate
16-point panels, including its Settings navigation. Live PNGs record these
states. The extra Codex bottom inset and Settings rail divider are removed.

All seven Settings pages have paired lossless working captures. These are
checkpoints from multiple builds, not accepted pages. Some earlier captures
contain an editor tab while later captures have none. About was recaptured
after its latest fix with empty tab strips in both applications.

Settings now uses a shared native layout with the source width limits and
viewport padding. Navigation uses the original vector paths, and the two-path
Vela mark uses the source geometry and theme mix. The source elliptical radial
gradient is decoded by GPUI's native SVG image pipeline at the window's backing
scale; no browser shell is involved. About and Voice use shared setting rows.

The rendered Settings regression reproduces content overflowing the right
padding with the old flex sizing. It passes at 900 and 1280 points after the
shared row is constrained. `settings-regression.txt` records the controlled
failure and restored pass. The full workspace suite now passes 76 tests, with
one ignored performance measurement. The live About page now fits and wraps
its update copy; its typography and small remaining offsets still differ.

After restarting the reference, its recovery notice exposed the failed-save
fixture artifacts from earlier checks. `recovery-notice.png` and
`recovery-fixture-review.json` record that state and artifact hashes. Eleven
reviewed fixture artifacts were acknowledged through the original UI; their
files were retained. The native recovery notice is still missing.

Providers now loads the real scoped integration catalog. The actual service
shows the same OpenCode Go, CommandCode and OpenAI availability as the reference.
Paired catalog and empty key-form captures record this checkpoint. Native key
fields mask UTF-16 units, block copy/cut and exclude raw keys from accessibility
and platform text context. Both applications preserved six units for the same
Unicode paste; the automation tool's typing path dropped non-ASCII characters
in both applications and is not used as Unicode-input proof.

The loopback-only provider fixture exercised a successful key connection, a
rejection whose response deliberately echoed the disposable key, cancellation
and reopening before a delayed response, and credential removal. The new form
retained its sentinel after the old request completed. Removal Cancel sent no
delete; OK removed only the selected fixture credential. The adapter verifies
workspace-catalog membership before the runtime's global credential deletion.
`provider-fixture-results.json` records only fake credential metadata and key
hashes. Fixture captures are functional evidence, not paired page acceptance.
The confirmation sheet is also captured; macOS returned its parent composite
for the explicit sheet window ID, so that capture's selected-window point size
does not describe the returned raster extent.

The last full suite passed 81 tests, with one ignored performance measurement;
Clippy passed. OAuth ownership is under implementation and requires new checks.
Provider OAuth, usage, full form validation, Mobile Setup operations,
page-specific responsive layouts, appearance previews, typography and complete
interaction acceptance remain open. No page is visually accepted.


## Provider controls and input continuation, 2026-10-26

Provider checks now use the loopback-only fixture on port 49557. The reference
profile is `/private/tmp/vela-provider-original`. A separate `Vela Native QA`
bundle (`dev.vela.native.parity`) uses `/private/tmp/vela-provider-native-fields`
with Daybreak and a 1280×800-point window. The ordinary native bundle and its
current preferences are left available for inspection. The QA executable is
the same Rust build; only its bundle identity and isolated test profile differ.
Captures explicitly select the inspected window ID when multiple Vela windows
exist. The bounded menu watcher also requires both fixture process IDs.

Engine-owned OAuth attempts capture workspace/session ownership, poll automatic
flows, accept code retries and clean up cancelled or expired attempts. Real GUI
checks against the fixture covered code rejection then success in both apps,
native automatic completion, waiting-flow cancellation in both apps, and
leaving Providers while a delayed start is still returning. That last native
attempt was cancelled on the server without publishing a pending form or polling
it. `provider-fixture-results.json` contains only fixture metadata and hashes.
No genuine credentials were entered or changed.

Shared text fields now traverse with Tab/Shift-Tab. Composer Enter submits and
Shift-Enter inserts a newline; marked composition prevents submission. A
rendered regression checks those distinctions. Long single-line Unicode text
keeps the caret visible, and paint/hit-testing share its horizontal offset.
Single-line insertion removes CR/LF, and field limits count UTF-16 units across
replacement and committed IME composition, preserving one composition undo.

A shared focus wrapper retains control handles and reveals newly focused
controls. The rendered regression covers offscreen focus, manual scrolling
away from retained focus, and changes to disabled controls. Paired
`providers-fixture-keyboard-actions-before-reveal` and
`providers-fixture-keyboard-actions-revealed` captures record the actual defect
and the native correction. The latest focus-ring thickness and button font
changes still require a refreshed comparison. Settings reserves its eight-point
scrollbar gutter only when content overflows; content removal and viewport
changes are covered by a rendered test. Live thumb dragging remains unchecked.

The native authorization selector now opens an AppKit menu. Mouse opening,
keyboard selection and Escape dismissal were exercised. The first paired
`providers-fixture-system-menu` PNGs exposed a light/dark appearance difference
and a two-point width difference. The adapter now supplies the control's theme
and font; these changes still require fresh menu captures. A bounded watcher
captures the menu's actual layer-101 window after permission prompts have
closed, without resizing or cropping. It uses exact process IDs to exclude
other app windows.

Typed field constraints now block a request before submission. Tests cover
required values, numeric bounds and HTML's default step/base, UTF-16 minimum
length, ignored constraints on Boolean/multiple/external fields, hidden fields,
and full-match ECMAScript patterns including Unicode sets and backreferences.
macOS uses the public JavaScriptCore C API only for regular-expression
validation, with JSON-encoded data and no DOM, browser UI or network access.
Other platforms currently report pattern-validation unavailability; they have
not passed acceptance. A rendered form test exercises real keyboard checkbox
changes, dependent-field validation, multiple-selection arrows/ranges and
select-all, then verifies the emitted answer order.

The `--forms` fixture exposes text, numeric, Boolean, select, conditional,
multiple and external controls to both real apps. Initial inspection exposed
checkbox alignment, select geometry and inactive selection-color differences.
Native controls now retain field focus, use AppKit popup menus, use system
selection colors, and preserve option order across range/toggle selection.
These latest widget changes have passed the rendered test but still need
live rechecking and paired lossless captures.

Remaining provider gaps are explicit: native validation errors still use an
inline message where the reference shows a floating browser bubble; numeric
inputs lack the complete browser stepper/editing behavior; selection typeahead,
dragging, larger-option scrolling and all responsive states are unfinished;
provider usage remains absent. Accessibility still reports native text focus as
the window and does not expose every disabled control state correctly.

The dead-key check found a real password-field discrepancy. Both apps accept
the same Unicode paste and mask by UTF-16 length. With Option-E followed by E,
the reference fixture received ASCII `e`, while native received `é`; ordinary
label fields compose `é` in both apps. The fixture stores only the hashes and
lengths for these disposable values. Chromium disables its input context for
password fields (see its [native input implementation](https://raw.githubusercontent.com/chromium/chromium/main/content/app_shim_remote_cocoa/render_widget_host_view_cocoa.mm)).
A native platform input-context/security fix is still required; no character
stripping workaround was added.

The latest full workspace run passes 94 tests, with one ignored performance
measurement. No page or menu has passed visual acceptance. The migration's
remaining conversation, session/panel, editor/recovery, terminal, browser,
settings, platform and release work remains active.

## Native menus and window dimensions, 2026-10-26

The menu adapter now follows the actual Chromium 138 implementation used by
Electron 37: a popup cell hosts attributed menu items with explicit item fonts,
instead of assigning the font to the cell. The latter changes text metrics.
Paired `providers-fixture-region-versioned-menu` captures both measure 768×212
pixels and show matching option text size/spacing. Their selection highlight
colors still differ; no menu acceptance is claimed. The application now also
sets AppKit's Aqua/DarkAqua appearance when the selected Vela theme changes.

The current reference always mounts Layout and requests session window sizing,
even without an open session. The older matrix's 760×522 welcome requirement
was stale. The reference creates a 380×320 bootstrap window, then restores
normal session dimensions or uses 1280×800. Native now uses that session
default, minimum200×200 and the source's traffic-light position16,15. A
version2 `window-bounds.json` stores only dimensions. Its parser also accepts
the legacy direct shape, rejects unsupported wrapped versions and invalid
dimensions, and no longer clamps small legacy windows to900×600. First import
creates the native dimensions file only if absent, preserving existing and
future-format files and leaving the original profile untouched.

Native observes normal-window dimension changes, debounces them400ms and sends
writes through the engine's serialized command loop, away from rendering.
Minimized, zoomed and fullscreen states are excluded through GPUI/AppKit state.
The integration test sends22 consecutive dimensions and verifies the final
request reaches disk. The live macOS zoom action expanded the QA window to
1512×949; it did not write that maximized size. Returning to normal saved
1280×800, and relaunch with `--size=saved` restored1280×800. Native-only
`window-size-before-relaunch` and `window-size-after-relaunch` PNGs record the
restoration checkpoint, not paired visual acceptance. Mouse edge/corner drags
through the app automation tool did not resize the window; arbitrary resize,
minimize/fullscreen and multi-display centering still require live acceptance.

The full workspace suite now passes96 tests, with one ignored performance
measurement; formatting, Clippy with warnings denied and diff whitespace checks
pass. The comparison report currently includes47 capture names and zero exact
lossless pixel matches (before adding the two native-only restoration captures).
Many captures come from earlier builds. Neither the report nor this checkpoint
claims that the migration or a page is complete.

## Model selection, feedback and approval continuation

The loopback model fixture on49558 exposes ordinary, long Unicode and rejected
choices. The native Settings selector now opens the real AppKit catalog menu,
captures the owning session, and changes selection only after runtime success.
Both real apps retained the long choice after rejection. Native catalog loading
now restores the session's selected model ahead of its runtime default,
resolves its display metadata from the catalog, and preserves the response
variant. The engine integration test covers that precedence, transmitted
variant and retained selection after rejection. Catalog failures retain the
previous catalog and produce feedback rather than silently replacing it with
an empty list.

`toasts.rs` renders shared four-second feedback across the GPUI shell, keeping
at most four entries and removing them by stable identity. Its timer regression
verifies that an older expiry cannot remove a newer toast. Live rejection
feedback is visible in Settings. Paired `model-fixture-restored`,
`model-fixture-rejected` and `model-fixture-shared-rows` captures are working
checkpoints. The bounded feedback watcher observes only fixture operation
metadata, then captures explicitly selected windows during the real feedback
lifetime. It does not issue app actions, pause a toast or change an image.
Error wording differs between the native HTTP path and the reference's legacy
IPC wrapper. Toast rendering over the native browser guest remains unchecked.

The Model section now uses shared Settings rows, group typography, list
dividers and the source callout width/line spacing. Ask/Approve is functional
and persisted, defaulting to Ask. Only new permission events for the currently
open native session can be automatically allowed; old pending permissions and
closed/background warm sessions are excluded. Duplicate pending events do not
issue another automatic response. A loopback-only fixture on49559 exercised
Ask, enabling Approve with an existing request, new active/background requests,
restart while Approve is persisted, and a new request after restart. Only the
two new active-session requests received allow replies. No command executes in
this fixture. `approval-fixture-results.json` records the metadata, and both
disposable app profiles were returned to Ask through their controls.

The native CLI version badge is still a frozen placeholder, and Sync/Update
operations are unfinished. The selector's intrinsic width, remaining text
metrics, focus/accessibility details and complete responsive behavior remain
unaccepted. `model_settings.rs` now owns this section's UI. The current complete
workspace run passed97 tests with one ignored performance measurement; the
subsequent Settings seam edit also passed its rendered layout check and Clippy.

A color investigation checked the actual Metal layer and found it already
tagged `kCGColorSpaceSRGB` on this Mac. Explicitly retagging it did not change
the relevant colors. The redundant experiment and its added dependencies were
removed. `model-fixture-srgb` retains the experimental captures; it is not a
claimed color fix. Earlier paired Region menu differences were confined to
the selected row, including when both menus were opened by keyboard. Their
highlight difference remains open.

## User correction: flush inner Settings join

The user supplied
`/Users/tdawg/screenshots/snapshot/2026-10-26-151207-L09CwF/capture.png`
and explicitly requested no rounding or gap where Settings navigation meets
the right content pane. This replaces the reference's inner rounded Settings
page as the target for that join. Codex now paints the page directly inside
the shared outer card; its extra inner border,16-point clip and shadow are
removed. The outer shell's curved left corner from the first user reference
remains intact. Classic continues to own separate cards.

Live inspection covered Appearance, Providers, Voice, Model, Servers, Mobile
Setup and About. Native-only `settings-inner-seam-before`,
`settings-inner-seam-after` and section captures record the correction. These
are proof of the targeted join, not full page acceptance. During the check the
QA window was moved and resized externally to1280×810; its dimensions file
also records1280×810. The captures preserve that real viewport rather than
resizing it to match older1280×800 pairs. The original app and ordinary native
app were not repackaged for this change; the rebuilt QA bundle contains it.


## Runtime controls and comparison integrity

The installed CLI badge now uses a five-second engine-owned process probe.
Model Sync/Update have native confirmation, busy/disabled states and inline
feedback. The engine serializes service management, uses the connected profile
for CLI commands, validates the reported V2 service version, and replaces its
SSE stream after success. A runtime rebind retains the session view, draft, scroll
and selected model. Initial service startup also uses the explicit binary and
selected state root; a custom descriptor cannot start an unrelated inherited
service. Cancellation is honored while opening or backpressuring an SSE stream.

A disposable loopback fixture on49560 and CLI double exercised Update cancellation
without commands, matching-service Sync,2.1.4→2.1.5 upgrade/restart/rebind, updater
failure with recovered controls, and an already-current update.
`runtime-fixture-results.json` records the bounded command log. No real CLI was
upgraded and no real shared service was restarted. Reference matching-service
Sync also produced the same success text. The earlier
`runtime-sync-checkpoint` capture is a page checkpoint, not a dialog capture;
confirmation text was inspected through AX. Real CLI terminal handoff remains
unverified. The latest full Rust suite passed102 tests, one performance
measurement ignored.

`model-runtime-top` and the first `model-runtime-external` attempt used different
display ICC profiles and cannot establish parity. The CUA titlebar drag did not
move the reference window. macOS Window→Move to24G1WG4 did: fresh
`model-runtime-same-display` captures both measure1280×800 and have the same ICC
profile hash. The comparator now reports manifest integrity, point viewports,
window activation and ICC differences as review eligibility checks. Synthetic
checks cover exact RGBA, alpha-only differences, profile mismatch and stale
manifests. No pixel values are resized, converted between profiles or masked.

The Model select width discrepancy was traced to Chromium138's themed internal
spacing, in addition to the source CSS padding. Its intrinsic width includes4px
start spacing and16px end spacing (1px plus15px arrow); vertical padding adds1px
on each side. Native now follows this and rounds intrinsic text width upward.
Primary implementation: [LayoutThemeDefault](https://chromium.googlesource.com/chromium/src/+/138.0.7204.251/third_party/blink/renderer/core/layout/layout_theme_default.cc),
[MenuListIntrinsicInlineSize](https://chromium.googlesource.com/chromium/src/+/138.0.7204.251/third_party/blink/renderer/core/layout/layout_box.cc),
and[MenuListInnerElement](https://chromium.googlesource.com/chromium/src/+/138.0.7204.251/third_party/blink/renderer/core/html/forms/menu_list_inner_element.cc).
Visual checks of the revised control are in progress. No page is accepted.


## Shared chrome, Unicode transport and layout scheduling

The native titlebar now uses the reference's16-unit SVG paths,20px icons,36px
controls,2px action spacing and workspace-aligned tab slot. The Codex6px bottom
gradient is present. SVG paint now reads the active parent color directly;
GPUI's Svg skipped painting when its local color was unset, which the live
check caught and corrected. Rail icons use the source20px size, have accessible
labels, and participate in keyboard traversal. The Agent Mode and true multi-panel
features remain unfinished; matching the bot geometry does not complete them.

The rail now collapses an already-selected Sessions/Files view, routes Changes
to the actual main Changes surface, creates a new browser tab per activation,
and resets a newly opened Settings view to Appearance. Live QA confirmed Files
collapse/titlebar restoration, Changes routing and two new browser tabs, then
closed those disposable tabs. `model-chrome-layout` is a fresh matched Model
checkpoint; it still differs and is not accepted. Native's scrollbar gutter
shifted after runtime feedback, exposing a separate frame-scheduling defect.

An actual chunked HTTP regression split every byte of Japanese text, emoji and
combining marks. Before the fix the decoder produced replacement characters.
The decoder now buffers complete UTF-8 lines, handles CR/LF/CRLF across chunks,
ignores one initial BOM, and preserves SSE data whitespace and multiline data.
The byte-split HTTP regression and mixed-line-ending boundary checks pass.

The scrollbar probe previously called Window::refresh during prepaint, where
GPUI deliberately ignores it. A scheduling regression inspects callbacks during
the draw (before the test harness adds its own refresh) and failed before the
fix. Gutter changes and focus reveal now request a real next-frame callback.
All3 scroll tests pass. The full workspace passes105 tests with one performance
measurement ignored; live post-Sync settling verification is in progress.

Live settling verification passed after rebuilding the QA bundle. Full-window
lossless `model-gutter-before-sync` and `model-gutter-after-sync` captures retain
identical dropdown pixels (zero changed in its diagnostic region). The only
changed pixels in the visible content rows are212 pixels at the Sync button's
focus ring; there is no8px content shift. A paired reference after-Sync capture
is also retained. These region diagnostics do not mask the full-page report.
Clippy with `-D warnings`, formatting and diff checks passed after the changes.


## Vite preview implementation in progress

The Servers section was audited against ServerSettings.tsx: it lists app-owned
Vite previews, rather than MCP servers. A new native preview manager selects
confined HTML targets (including the reference's depth-limited index search),
allocates loopback ports5199–5218, probes HTTP readiness, captures bounded
diagnostics and owns the complete child process group. Stable leases protect
new starts from stale exit callbacks. Workspace scopes are registered before
background startup, so an immediately queued close also cancels pending work.

Three focused tests pass using real HTTP child processes: target selection and
confinement; multiple entries/encoded filenames/toggle/port release/late cleanup;
and404/timeout/early-exit diagnostics/application shutdown. The test expectation
was corrected for macOS's /var→/private/var canonical path. Engine inventory,
terminal preview control, context Stop actions and live Servers rows are wired
and compile. The manager and UI are not live-accepted yet.

Preview tools were installed only into this checkout. The official Node22.23.2
archive passed its SHA256 check. Vite6.4.3 runs with the reference's locked
transitive dependency records and Sass1.102.0. Packaging includes the Node
executable/license and npm resources. Real native preview/Sass/browser and
paired Servers captures are the next checks. No original app files were edited.

## Live preview checks and application quit

Real packaged Node/Vite served the nested HTML fixture and compiled Sass. The
native embedded browser rendered the page and changed its heading from blue to
green on a Sass edit without a manual reload; the fixture was restored to blue.
The default external browser also received the preview URL. Two independent
HTML entries ran on5199 and5200. Context Stop displayed above the embedded
browser and released its server; Settings Stop left the other server alive,
and Stop all released both ports. Paired `servers-preview-one` and
`servers-preview-two` captures show the reference inventory. The first native
two-server capture exposed a stretched Stop all button; it is retained as
`servers-preview-two-before-sizing`. The corrected intrinsic-width button was
rebuilt and inspected live. No page is visually accepted.

The actual QA window-close path exposed a process leak despite passing manager
tests: AppKit terminates without returning to cleanup after the GPUI run loop.
The owned Vite child was confirmed reparented to PID1 and then terminated.
Application quit now invokes engine shutdown through GPUI's app-level quit
hook. On Unix, a synchronized ownership registry closes process groups before
the framework's200ms shutdown allowance; a concurrent late start is rejected.
Terminal registration also checks shutdown while holding its ownership lock.
The engine exposes completion for its actor cleanup. A new regression kills a
SIGTERM-ignoring preview and helper without polling async cleanup. Real QA quit
then removed the recorded app, terminal and Vite PIDs, closed5199 and logged
`engine stopped`. `preview-fixture-results.json` records these observations.
The latest full workspace run passes109 tests, one performance measurement
ignored. Clippy with `-D warnings` and formatting pass. Resource relocation,
locale-sensitive entry ordering, production release acceptance and the full
visual matrix remain unfinished.

## Current editor reference correction

A fresh audit of current `use-workspace-editor.ts`, `editor-persistence.ts` and
`docs/renderer.md` establishes manual, revision-safe Save/⌘S. Typing stays in
memory; closing a dirty tab first saves it. Earlier migration entries calling
for autosave were stale and have been corrected. Watch-driven updates, source
tooling, unsaved-file journaling and full quit/close guards remain required.

## Native workspace observation

A native recursive watcher and serialized workspace read/write owner now
retain file identities, debounce paths, bound event queues, rescan overflow,
and distinguish exact save echoes from external writes. Multiple sessions
share a workspace owner; generations reject detached workers. Unique renames
correlate across separate batches, with ambiguous hardlinks left uncorrelated.
Git baseline reads are local, bounded and relative to the workspace cwd;
non-Git observations retain unknown baselines. Save baselines come from the
actual original held by the recovery transaction, including explicit overwrite
and recreation. A delayed save acknowledgement cannot replace a newer external
revision.

The full Rust suite passes117 tests with one performance measurement ignored.
New tests include actual native notifications, overlapping workspace ownership,
generation replacement, queue overflow without starvation, Git baseline refresh,
Unicode nested Git paths, rename/undo ownership, strong echo identity and held
original receipts. Symlinks, dependency/recovery directories, oversized watched
files and invalid UTF-8 do not enter the observed-text inventory.

Live paired checks passed clean reload, manual in-memory editing, dirty external
conflicts, a Unicode external rename with dirty text preserved, clean deletion
and recreation. Native reload preserved local edits in undo; rename retained
undo/redo and Save merged content wrote only the new path. No false conflict
followed that save's notifications. An already-expanded directory gained a
new file and removed moved-out files without reopening. Paired lossless
`observation-clean-reload`, `observation-dirty-conflict`,
`observation-dirty-rename` and `observation-clean-deletion` captures retain the
large remaining visual differences. `observation-fixture-results.json` records
the functional results. Native QA was then closed cleanly; its engine logged
shutdown. The ordinary Native app and its user work were left running.

Native windows can retain an old GPU frame while occluded. Raising the window
through its exposed accessibility Raise action revealed the current editor and
conflict state. Each subsequent rendered-state inspection raises the relevant
app before capturing; hidden AX content alone is not proof of visible state.
The source's hidden terminal subtree can likewise remain in AX after its tray
visibly closes.

The explorer comparison also found inherited native Git letters in place of
the reference's observed-change and unsaved dots. Source-derived dots were
rebuilt and checked live: external creation shows blue, editing adds orange
while disk stays unchanged, and Save clears only orange. Lossless
`observation-status-dots-dirty` and `observation-status-dots-saved` record the
native states. Clippy with `-D warnings`, formatting and diff checks pass.
Full tree geometry, selection,
icons/actions and editor presentation remain unfinished. Captured agent-input
baselines, complete baseline-aware Changes, journaling and close/quit guards
also remain required. No page is visually accepted.

## Close and quit protection, and workspace navigation

The missing guard was reproduced in the previous QA build: red Close lost a
disposable unsaved buffer. A single document coordinator now covers red Close,
menu/keyboard Quit, every retained workspace and pending snapshot writes. Save
All waits for acknowledgements, refuses conflicts/failures, and rechecks current
unsaved revisions before shutdown. Editor workspace leases survive transcript
warm-cache eviction; read/write authority is tied to the captured root, and a
closed clean buffer is pruned after its final save acknowledgement. Core and
engine regressions cover these boundaries and reject unrelated/released roots.

Real QA checks against the existing authenticated 2.1.4 service passed Cancel,
Escape, keyboard/menu Quit, Save All before exit, two-workspace Save All, conflict
refusal with both local and external text preserved, and Don't Save with disk
unchanged. During this check, opening another folder exposed an existing UI
handoff defect: the engine created a session while the UI retained its old active
ID. Explicit workspace-open completion now selects the requested session and
refreshes its file inventory; a real second-folder check and an engine regression
passed. The full Rust suite passes122 tests with one performance measurement
ignored. See `close-fixture-results.json` and the lossless close-guard captures.

The macOS applicationShouldTerminate hook installs successfully in the packaged
QA app, but actual Dock/OS Quit is not accepted: Dock automation timed out.
Delayed-save GUI races and an actual write-permission failure remain unchecked
in the GUI. Native warning dialogs initially used the wrong AppKit style and
omitted the source warning triangle; the rebuilt dialogs use the same Critical
style as Electron warning/error boxes. Initial button focus still needs matching.
Sheet captures now record an explicit parent window context and the full window
group, because macOS captures the complete dimmed parent when targeting a sheet.
Existing artifacts are never overwritten. No page is visually accepted.


## Explorer, editor chrome and overflow continuation (2026-10-27)

The explorer now renders original SVG file glyphs, source indentation/guides,
selection, observed/unsaved dots and lazy visible rows. Root/nested row heights
remain 30/29 points. Removing the engine's silent 1,000-entry cap exposed a
second real defect: the virtual list's unknown heights contributed zero, and
its first layout/width changes discarded supplied height hints. A rendered
regression reproduced the GUI failure (last reachable item6). The explorer now
uses exact known row geometry in a native scroll viewport and creates only
visible elements. Tests cover1,205/1,405 mixed-height rows, resizing, later
batches and removal at the bottom. The packaged app scrolled to file-1204.txt
and opened its actual content. The original reached the same file.

The first large-folder capture pair is ineligible for visual review: the source
launched on another display, producing a different ICC profile. It is retained,
not converted or masked. Source automation also resolved the dev bundle's shared
ID to /Applications/Vela.app on rebind, launching another process. No editing or
cleanup was performed in that installed app. A separately named/reference-ID QA
bundle now runs the same original renderer from its read-only checkout in a new
disposable profile; it avoids that ambiguous app identity. Matching-display
recapture remains active.

Editor geometry now includes the52-point header, full workspace breadcrumb,
manual Save control and Source13-point SF Mono. Fonts are registered from the
locally installed Terminal app; no Apple fonts are redistributed. Empty-agent
art uses the original208x161 asset and device-scale nearest-neighbor rendering.
Tab chrome uses original FileIcon SVGs, preview italics, dirty/close affordances
and the source width algorithm. Twelve real pinned files produce8 visible and
4 hidden tabs; menu scrolling, hidden selection and Escape passed live checks.
Enter initially toggled twice because a custom key-down action duplicated GPUI's
built-in key-up click. A single activation path and rendered press/release test
now pass; the rebuilt app's keyboard recheck is still pending. Emoji fallback
line metrics remain different, and mixed file/browser overflow is not accepted.

The last full Rust suite passed125 tests with one performance test ignored;
the subsequent keyboard regression passes separately. Workspace/all-target
Clippy with-D warnings passes. The comparison report currently contains98
capture names (including historical and unmatched evidence) and0 pixel matches.
These counts do not represent a completed page matrix. See
explorer-tabs-fixture-results.json. Full explorer actions, language tooling,
split editors, composer operations and the remaining migration scope are still
open. No page has passed final visual acceptance.


### Final explorer/tab validation in this continuation

The rebuilt GUI passes Enter/Space overflow activation exactly once per press,
and mixed menus retain/select twelve real files and three native browser tabs.
The shared scroll container now reserves the source's conditional eight-point
gutter. Both narrow selected rows span x57..264; tests also require short trees
to recover the full width. The source's collapsed blank-drop target contributes
two transparent border pixels to the scroll extent; including them aligns the
last filename's ink to y765..773 in both full-size captures.

A fresh comparison profile preserves the earlier QA profile's newly observed
composer draft without sending or clearing it. The uniquely identified reference
and native QA now have eligible same-display, 1280x800 pre-spacing and 900x700
narrow pairs. Both narrow windows contain the same twelve pinned files, four
visible tabs/eight hidden, a wrapped two-line editor and an open overflow menu.
The corrected full-size row pair has an activation mismatch and remains
ineligible, despite its useful geometric measurements. No screenshot was
converted, cropped or masked to conceal differences. Two source corner drags
did not resize the window; the narrow check uses an isolated saved-size fixture
and does not claim live OS resizing passed.

Capture comparison now requires stable before/after window inventories. Default
capture selection prefers uniquely named QA processes, avoiding installed apps.
The report contains 103 historical/current capture names and zero pixel matches.
The complete Rust suite passes **126 tests**, with one ignored performance test.
The first sandboxed run was blocked from sockets/native notifications; its log
is retained separately from the successful full-access run. Latest all-target
Clippy with `-D warnings`, formatting, shell syntax and diff checks pass.

Outstanding work includes emoji fallback line metrics, complete explorer actions,
editor/language/split capabilities, composer geometry/attachments and its no-op
Default strength control, all remaining features, paired mixed-browser validation
and full page/theme/shell/responsive acceptance. No page is visually accepted.


## Model response strength (2026-10-27)

The no-op Default composer control is replaced by advertised model variants.
Catalog parsing now matches the source's ordered string/object variant IDs and
excludes disabled/incomplete entries. Models without variants show no strength
control. Models with variants show Auto and the formatted current choice. The
new native strength menu dispatches the existing acknowledged session model
operation. Auto omits the variant; rejection keeps the acknowledged selection.
Selecting the same model preserves its strength. Composer code is separated into
composer.rs. Escape dismisses pickers even when focus is outside the text field.

The engine regression covers catalog enrichment, disabled entries, Auto payloads,
retained variant metadata and restoration of high. Real packaged GUI checks
passed hidden strength for an ordinary model, High selection, menu check state,
rejected variant with High retained, Auto reset, Escape and Auto restoration after
restart. The original loaded the same fixture and exposed the same choices.
Full-suite validation now passes **128 tests**, with one performance test ignored;
all-target Clippy with-D warnings, formatting, script syntax and diff checks pass.
See model-variant-fixture-results.json and rust-workspace-tests-variants-2026-10-27.txt.

Paired composer-strength-matched-display PNGs use the same900x700 viewport,
fixture and display. Menu position/width, composer sizing and empty-editor text
wrapping still differ. The prior no-op strength item listed above is resolved;
complete source model-menu behavior, attachments and remaining controls are not.
A Retina/negative-origin attempt rendered Native but exposed only window chrome
in AX; coordinate activation failed while keyboard Quit worked. This remains
unresolved and is not accepted as a successful interaction test. Both comparison
apps returned to the working24G1WG4 display. Source placement was changed through
its Window > Move to24G1WG4 menu after drag attempts proved ineffective. The source
renderer was transiently absent during a concurrent build; native CmdR reloaded
it once available. No source files were changed. No page is visually accepted.


### In-progress picker follow-up

Further real checks found that Back to model list unmounted a focused menu
control, stranding Escape and keyboard Quit. Persistent toolbar focus owners
now replace the transient focus, including model/agent/strength transitions,
Escape and successful selection. A rendered regression fails without recovery
(`picker-focus-without-recovery.txt`) and passes with it. Large-catalog UI
validation (30 models/45 agents) is still pending final selection: the last model
was visibly reachable and captured in `native/catalog-model-last.png`. Silent
12-model/40-agent caps were removed and fixed-height scrolling/accessible rows
added. Agent labels now wait for runtime acknowledgement.

The QA process later stopped before a rebind; getApp launched it without fixture
environment and briefly opened the ordinary native profile, showing a missing
demo-session error and no open editor. That instance was closed through its
native Close control; no file edits or prompts were made. Settings/import/window
state may have been accessed; it is not claimed untouched. A startup isolation
guard is being added before all state reads/service startup so a stopped QA
bundle cannot silently fall back to normal app state. Its tests and bundle
validation remain in progress. Always verify the QA process is running before
binding automation. The ordinary bundle was not replaced.


### Catalog and QA guard checks completed in the next continuation

The guarded bundle refuses an unconfigured direct launch before profile creation,
verified with a copied executable confined to a disposable directory. Both startup
guard unit tests pass; the ordinary executable remains unrestricted. Explicit
comparison launch connected to fixture49563. Real UI checks scrolled to and selected
model029 of30 and agent044 of45, with acknowledged toolbar labels. The final agent
was captured losslessly. Back from strength to the model list, Escape, and CmdQ
now pass in the packaged app; the engine logged shutdown and the process exited.
See catalog-focus-fixture-results.json. No page is visually accepted. Search,
favorites/preferences, remaining composer features and the full migration remain
unfinished. Composer/empty-editor geometry corrections are implemented and checked
live, but their full visual acceptance remains open.

Latest full Rust run: 131 passed, 1 ignored performance test; all-target
Clippy with `-D warnings`, formatting and diff checks pass.


## Functional model search continuation

Replaced the decorative search box with the shared native text input. Queries
match model name, ID and provider using trimmed, case-insensitive text, including
Unicode; query changes reset list offset. The selected-model row is filtered too,
and empty results display No models found. Search does not mutate runtime
selection until a result is chosen. Real GUI checks passed whitespace/case ID
matching, Unicode, all30 provider matches, empty feedback, filtered selection and
keyboard Quit. The engine logged shutdown. See model-search-fixture-results.json
and native/model-search-filtered.png. Source favorites/visibility, remaining menu
behavior and page acceptance are unfinished.


### 2026-10-27 continuation: persistent model preferences and composer cycling

Replaced the fake current-selection Favorites row and inactive gear with actual provider-qualified favorites, individual model visibility, provider visibility, and collapsible Favorites/provider groups. Native settings default older profiles to empty preference sets and persist them atomically. Current selections remain available when hidden; favorites are independent of visibility, matching the original. Shared search filters both views.

Live isolated catalog checks passed for favorite toggling without model switching, individual/provider hiding, restart persistence, and Shift+P favorite cycling. Ctrl+Tab acknowledged Auto → Low → High without inserting draft text. Both original and native apps were loaded against the 30-model/45-agent fixture. Heading spacing was corrected after comparing their actual menus. Lossless captures are retained; v2 has mismatched activation metadata and is explicitly ineligible for acceptance. No page is visually accepted.

Workspace suite before the last visual/shortcut follow-ups: 134 passed, 1 ignored performance test. Workspace all-target Clippy with warnings denied passed after the follow-ups. The initial sandbox-only engine run had socket/notification permission failures; the full outside-sandbox run passed. Additional ongoing work aligns the native agent menu and footer with the original, with live checks still required. See model-preferences-fixture-results.json.


### 2026-10-27 continuation: agent menu/footer and attachment staging

The native agent picker now uses the source composer anchor, 340-point limit, 14-point radius, padding, checkmarks, typography and real focus-reveal scrolling rather than the prior static header and smaller menu. A live 45-agent catalog reached entry 044 by wheel and keyboard, with Enter acknowledging the final selection. Footer controls now use source compact dimensions, Git branch/chevron glyphs, source arrow/stop vector geometry and send-ink tokens. Busy keeps a Stop button even when draft text exists; Enter remains the queued-send path.

Attachments now flow from native multi-file selection and external-path drop handling into staged chips and the engine request. Unicode names, deduplication, PNG thumbnail display, individual removal and file-only submission passed live checks against a new local fixture on 49564. The fixture records metadata only and never reads files or runs a model. The engine regression also verifies immediate and queued file-only requests preserve paths. Full workspace validation: 135 passed, 1 ignored performance test. Clippy all-targets with denied warnings passed after follow-ups.

Both apps staged the same two disposable files for a lossless paired capture. Both PNG manifests are stable, matching point/pixel sizes, color profile and inactive state; the case is eligible for review but differs on 31,366 pixels above the threshold (4.979 percent of the full window). It is not accepted. The comparison exposed native chip color/remove-glyph and short-model-width differences, now corrected in source pending a rebuilt live check. The native file picker is being changed from GPUI's modeless window to an AppKit sheet owned by the QA window to match the original HTML file input.

During checks, WindowServer reported every Vela QA window off screen while accessibility exposed stale images. Those intermediate interactions were not counted. The user brought QA back into view and the attachment checks above then ran against visible windows. Source attachment drafts remain staged in /private/tmp/vela-attachment-reference; native /private/tmp/vela-attachment-native staging cleared after the explicit local fixture submission. See composer-attachments-fixture-results.json for outstanding work.


### 2026-10-28 continuation: picker close ownership and persistent build cache

The owned file picker was live checked as an AppKit sheet. Selecting a path and canceling preserved the staged attachment draft. New full-window v2 chip captures and grouped sheet/parent captures remain ineligible because their activation metadata differs; no page is accepted. The first two-file pair remains eligible but different.

With an unsaved disposable document and the attachment picker open, explicit native menu Quit canceled the picker and presented the existing save/discard/cancel document guard. Cancel retained the exact dirty text. A second Quit followed by Don't Save exited successfully; the file's before/after SHA256 stayed identical. A lossless full-window grouped capture is retained as native/attachment-sheet-dirty-quit.png. Automated CmdQ while the picker was key also did not exit the original reference; this observation is not counted as a native-only failure. Application-global Quit fallback and owned-picker cancellation now feed the same document coordinator.

After a date rollover, Rustup attempted a component refresh and failed on an existing cargo-fmt binary. Further checks found 788 registry packages without manifests and 55 more incomplete packages in the temporary cache. A copy-on-write cache was created at target/migration-cache/rust, with the 843 incomplete package directories preserved under target/migration-cache/incomplete-packages; cargo fetch --locked restored dependencies. The original temporary cache was preserved. Use the explicit installed toolchain path to avoid Rustup's component auto-refresh:

```sh
export CARGO_HOME="$PWD/target/migration-cache/rust/cargo"
export RUSTUP_HOME="$PWD/target/migration-cache/rust/rustup"
export RUSTUP_TOOLCHAIN="$RUSTUP_HOME/toolchains/stable-aarch64-apple-darwin"
target/migration-cache/rust/cargo/bin/cargo test --offline --workspace
```

Rustc remains 1.99.1. The restored cache successfully checked the UI. The workspace suite passed after the inbox contracts were added (see inbox-workspace-tests.txt). Native server-inbox work is ongoing: remove the six-item cap, use the source 96-point scroll viewport below the composer, preserve attachment counts, and add server-entry Edit/Send now/Remove actions. Local durable follow-up queuing/reordering is still unfinished. The new eight-entry fixture is running on 49565 under /private/tmp/vela-inbox-fixture.

## 2026-10-28 durable queue, attachment contract and Quit routing

Native now persists failed queued prompts by workspace directory plus session, with Source limits (20 prompts / 50 targets), full files and mention spans. Local chips merge after runtime entries, with adjacent move controls, edit/restoration, remove and Send now. Runtime ACK removes a retry; an in-flight retry remains on disk until then. Real GUI checks covered rejection, reorder, restart, image restoration, re-rejection and successful retry. The source reproduced the same queued attachment failure against the strict fixture. Legacy queue import and exact visual review remain unfinished.

A source-contract audit found that the old native adapter sent `path` objects instead of the source API's encoded `uri` and basename. The adapter now validates regular files and the 10 MB limit, emits URI/name/mention, and the fixture rejects path-based payloads. Earlier path-only fixture assertions were internal pipeline checks, not end-to-end compatibility evidence. File-only UI submission now uses the source's fallback text. The hidden follow-up preference persists and drives queue/steer; the OpenCode reference manifest has steering=false, so native also hides its setting control.

Current V2 start/interrupt/success events were exercised live, including Stop with an unsent Unicode draft. Empty busy transcript now matches VelaTranscript's top-of-column Working/spinner geometry and stable placeholder. The busy captures have an activation mismatch and remain ineligible; they are not accepted screenshots.

The queue restart check found a real global Quit routing failure after a transient focus control disappeared. Global actions now defer through the explicit owning window; AppKit termination also uses that owner and fails closed on routing error. The rendered regression verifies a global Quit reaches an owner with no focused child and respects dirty state. Live queue-focus Quit exited, and dirty Quit displayed the save alert, Cancel preserved exact text, and Dont Save exited with the original disk hash unchanged. `native/queue-focus-dirty-quit.png` records the full window plus alert.

Validation: 142 workspace tests passed, one performance test ignored, and all-target clippy passed with -D warnings before the final visibility-only removal of the hidden setting row. No page is visually accepted. Next: matched post-restart local queue capture, then transcript terminal/settlement and complete VelaTurn rendering.


## 2026-10-28 transcript lifecycle, native rows and canonical replay

Native V2 retry/step failure, completion and quiet settlement now preserve partial text, clear streaming state and retain retry deadlines. Real QA checks matched retry/failure transitions, idle settlement after two seconds and stalled busy recovery at sixty seconds. The native transcript now projects independently virtualized work groups, reasoning disclosures, answer paragraphs, elapsed labels and Copy/Copied actions. Real clipboard paste verified the complete two-paragraph response. Full Markdown, code/tool presentation, pacing and other transcript actions remain incomplete.

A canonical replay audit found the native parser dropped the source-supported `{info,parts}` envelope and `role` discriminator. A red regression proved four messages became zero. The corrected parser preserves user text parts, canonical part IDs, independent ordinals, completion state, coded failures, and retry `{error,at}` metadata. A local HTTP regression also covers raw-array responses and Unicode continuation cursors; empty cursors terminate correctly. Interrupted execution now adds the source's informational notice while preserving partial output. The notice fix has automated coverage; a rebuilt interruption GUI comparison is still pending.

`canonical-replay-workspace-tests.txt`: 152 passed, one ignored performance test; `canonical-replay-clippy.txt`: all targets passed with denied warnings. The subsequent raw-array/pagination integration regression also passed. These results precede the ongoing Markdown parser changes.

Two diagnostic captures exposed differing fixture state and are not acceptance evidence: `transcript-reasoning-expanded-v2` had an Interrupted notice only in the reference and a reference inbox that was not restored; `transcript-canonical-replay` used part IDs that sorted the answer before reasoning during reference projection. The corrected canonical fixture uses chronological msg_/prt_ IDs. `transcript-canonical-replay-v2` has matching full 900x700 lossless PNGs, ICC profiles, inactive metadata and stable manifests. It differs on 177043 pixels exactly; 21635 pixels exceed the diagnostic threshold (3.4341 percent), MAE 2.29336. The comparison is eligible but NOT accepted. Typography, user bubble dimensions, composer strength width, icons and window chrome remain visibly different. Latest comparison report: 131 cases, zero exact matches.

Reference local queue recovery uses a fresh workspace UUID on reopen, so the earlier proposed matched post-restart local-queue capture cannot be treated as equivalent to native's directory/session recovery. Legacy queue import and that behavioral difference remain unresolved.

Current running canonical-history fixture is on 49566, state /private/tmp/vela-history-fixture, seeded by canonical-transcript-history.json. Both QA apps use the existing inbox profiles with this new fixture and no inbox entries. Reference session must be opened through Recents after restart; move its Window to 24G1WG4 before capture. Launch native second, capture reference inactive, activate reference via Window > Vela, then capture native inactive. Preserve all earlier immutable captures.

Work in progress: pinned pulldown-cmark 0.13.5 parser replaces the hand-written Markdown boundary scanner. The parser preserves nested list/quote/table containers and fence lengths; indented-code offset regression is being checked. Full native Markdown rendering is still pending and must be rebuilt and exercised before any acceptance claim.


## Structured Markdown and scrolling continuation

Pinned pulldown-cmark 0.13.5 now supplies balanced syntax events and top-level boundaries. Nested list/quote/table containers, longer fences and indented code have parser regressions. Native headings, scoped nested emphasis/italic/strikethrough, literal code delimiters, links, lists, quotes and tables render structurally. This remains partial: exact list/paragraph/quote/table sizing, inline code font/padding, image and KaTeX rendering, syntax highlighting, complete user Markdown and streaming pacing are unfinished.

Code blocks now expose real keyboard/mouse Copy/Copied and Wrap/No wrap actions with per-block state and horizontal overflow. Both apps passed live full code clipboard checks: Unicode, a 240-character line, blank line and final line were retained. Wrap controls changed layout. The GUI check exposed a real panic on transcript wheel scrolling (`RefCell already mutably borrowed`, GPUI list.rs:555). A rendered regression reproduced it (`transcript-scroll-before.txt`); the owner now samples list tail state after GPUI releases its mutable borrow. The passing regression is in transcript-scroll-after.txt. Rebuilt QA repeated long scrolling without crashing.

The structured renderer also exposed a prompt intrinsic-width collapse; native user inline text now measures directly. A rendered bubble regression verifies normal width and height. Live top-of-transcript inspection passed. The source's eight-point overflow gutter was missing from the virtual list. ListViewport now probes overflow outside the list borrow, reserves eight points only while needed, and supports a draggable/paging thumb. Its rendered regression covers content removal, restored overflow and window growth. The live rebuilt app now wraps the sample prompt identically to the source (two lines, same word break). Thumb drag and matched post-gutter capture still need checking.

Latest structured-markdown-workspace-tests.txt: 159 passed, one ignored performance test. structured-markdown-clippy.txt passed all targets with denied warnings. Native binary was built and QA packaged after these production changes. Current native QA is running against 49567 (/private/tmp/vela-markdown-fixture), with structured-markdown-history.json. Source QA uses the same fixture and the existing inbox reference profile. Both transcripts are scrolled to the top; source code is No wrap, native code currently still wrapped after its latest restart. No dirty documents or submitted prompts.

The immutable transcript-structured-markdown-top pair predates the gutter correction and has an activation mismatch, so it is ineligible. It differs on 9.8925 percent of pixels above threshold; it must not be counted as accepted. The comparator now has 132 cases, zero exact matches. Current visual gaps include paragraph/list rhythm, quote inset/wrapping, table intrinsic column sizing, inline code sizing, typography, composer controls and titlebar chrome.


### Post-gutter captures and return-to-bottom controls

`transcript-structured-markdown-v2` is a new eligible full-window pair: both stable/inactive PNG manifests match size and ICC profile. Activating Finder through its Window > screenshots action before capturing BOTH QA windows gives consistent inactive capture metadata without relying on QA launch activation. This pair still differs exactly on 209117 pixels and on 7.17254 percent above the diagnostic threshold; no acceptance. It precedes the latest list/table rhythm and return-to-bottom fixes. Comparator: 133 cases, zero exact matches.

Lossless pixel analysis confirmed the source quote uses the browser's default 40-point inline margins. Native now keeps these margins, content-driven table column widths, collapsed table borders and Source CSS start alignment. The eight-point gutter makes the user prompt wrap at the same words. Further measurements led to list marker placement outside item text, nesting markers, collapsed-margin rhythm compensation and table line-height inherited as 22.75 pixels rather than recalculated at the table's smaller font. Current differences remain inline code sizing/padding, font rasterization, some list/table/code spacing, composer and chrome.

Native now has Source's floating Scroll to bottom button, inclusive 24px threshold, 260ms cubic easing, immediate busy/reduced-motion path, and cancellation on manual scroll/session change. Scrollbar paging/drag informs the same owner as wheel events. Live return-to-bottom reached the final line and hid the button; dragging the thumb back to top restored the button and earlier text without a crash. A regression covers 24px/25px wheel thresholds. Long-history estimation, full scroll memory and animation interruption matrices are still pending.

Keyboard verification exposed double activation: GPUI's on_click already handles Enter/Space key down/up; redundant on_key_down handlers toggled twice. Code and transcript controls now use the built-in single activation and explicitly focus on accessibility click. A rendered generic code-control regression verifies Enter, Enter, Space each activate once. Live native QA reproduced the fixed behavior: clicking No wrap changed to Wrap, Enter changed back to No wrap, Space changed to Wrap. No prompt was submitted.

Current Native QA session is running the latest build against 49567 with code unwrapped (button label Wrap), scrolled near the lower transcript. A whitespace-only composer draft appeared during concurrent UI changes; it has not been submitted. Source remains on the same structured fixture, at the top with code unwrapped. The full workspace suite last passed 159 tests before the final keyboard regression and handlers; subsequent UI suite/clippy logs are being refreshed. New total should be 160 if all checks pass. Last latest binary packaged after keyboard fix; only formatting/test-source changes followed build. No screenshot after these latest control/rhythm changes yet.


### Final keyboard verification before inline layout work

Latest full workspace check passed 160 tests, one ignored performance test. UI-only suite passed 40 tests; all-target clippy passed with denied warnings. These cover the return-to-bottom controls, 24px threshold and final keyboard activation changes. Real native QA verified Enter and Space each toggle wrap once after the duplicate handler removal. Native QA running session now uses the same 49567 fixture, code unwrapped (Wrap label), near lower content, with the whitespace-only draft retained and no submission. The source QA is unchanged.

Next work: a native mixed-size inline text element is needed for Source codespans (0.875em plus 1px/5px padding and 4px radius). GPUI StyledText/TextRun exposes one font size per paragraph, so reducing only a font family or scaling the entire paragraph would not match. No mixed-size inline implementation is on disk yet. Existing native links are clickable, but complete keyboard/a11y/selection support remains pending. Full Markdown images/math/highlighting and all broader migration gaps remain.


### Mixed-size inline renderer continuation

`inline_flow.rs` is now implemented and used for paragraphs/user bubbles containing codespans. It shapes code at 0.875 of the inherited font size, applies 5px left/right and 1px vertical padding with a 4px rounded background, aligns baselines with surrounding prose, and uses Unicode line-break opportunities plus grapheme-safe long-token wrapping. Plain paragraphs retain the previous renderer. Links retain byte-range mouse activation in mixed paragraphs; keyboard/a11y/selection completion is still pending. The regression verifies 14px prose / 12.25px code, 10px total horizontal padding, explicit blank lines, Unicode ZWJ integrity and no dropped/duplicated text after narrow reflow. The focused test and all-target Clippy passed. The rebuilt live app painted the smaller codespan/padded background correctly in the formatted fixture. Full corpus and new lossless capture remain pending.

Current Native QA running session: 54012, fixture 49567. It is at the transcript top, with code currently wrapped (No wrap label) and empty composer at last observation. Source QA still runs session 52897, same fixture, top, code unwrapped. Build/package happened before the small follow-up language-label correction in source (empty fence/indented code now labels text, nonempty language info preserved). That caption-only correction still needs rebuilding. `mixed-inline-ui-tests.txt` is running; `mixed-inline-clippy.txt` passed before the caption correction. No screenshot after mixed inline work yet.

Image audit has begun only; no image implementation changes yet. GPUI is currently configured with NullHttpClient, so remote Markdown images need an actual client. The pinned zui fork supplies reqwest_client with TLS verification and its own runtime fallback; adding that crate or an equivalent adapter, URI/file/data-image mapping, lazy intrinsic sizing (max 200x200), accessible alt/failure display and the live image corpus is the next unfinished work.


## 2026-10-29 continuation: images and canonical tools

No page has visual acceptance. The report now contains 136 cases and zero exact matches.

- Restored Pillow 11.3.0 in ignored `target/migration-cache/python`; the temporary Python installation lost source files overnight. Use `PYTHONPATH=target/migration-cache/python` for comparison tooling.
- Markdown images resolve embedded PNG/SVG and percent-encoded local files asynchronously, retain their intrinsic aspect ratio within 200 by 200 without upscaling, and remeasure virtual rows when decoded. Live reference/native PNG, SVG and Unicode file fixtures render. Remote HTTP images intentionally follow the reference renderer CSP (`img-src self data:`); the temporary HTTP adapter and its added lockfile dependencies were removed.
- Broken-image glyphs are original Chromium resource 51431, extracted at 1x/2x with provenance, hashes and Chromium license. `scripts/sync-reference-broken-image.py` reproduces extraction; packaging includes the license/provenance. The frame now paints the agent surface behind its shadow, avoiding an incorrect dark interior. Strong Markdown uses 700 while heading styling remains 600.
- Reference Developer Tools confirmed `.SFNS-Regular`, 14px, weight 400, 22.75px line height. The font-family assumption is now verified. Developer Tools are closed. Panel widths were restored to 325px after inspection.
- Eligible full-window, inactive, same-ICC image pair `transcript-images-before-shadow-fix`: 183394 exact changed pixels, 28385 above 8 (4.50556%), MAE 2.61046. Eligible `transcript-images-shadow-fix`: 180565 exact changed pixels, 25345 above 8 (4.02302%), MAE 2.42372. Neither pair is accepted. Captures remain unmodified. Remaining offsets, typography and wider shell differences are visible.
- Full workspace image gate: 164 passed, one ignored performance test (`images-workspace-tests-unsandboxed-v2.txt`). The sandboxed attempt failed on blocked localhost/file-notification capabilities; the permitted integration rerun passed. Image Clippy and native packaging passed.
- Canonical tool regressions reproduced part-ID/call-ID confusion and duplicate rows when live updates followed restored history (`canonical-tools-before.txt`). Parser now preserves canonical identities, tool/name aliases, status aliases, output/content/error priority, state timestamps and input details. Live call-ID updates locate canonical parts; canonical snapshots replace prior call-ID placeholders and invalidate old cached blocks.
- Reference history hydration discards structured tool errors and displays `[VELA_TOOL_FAILED] Tool failed`. Native matches this history presentation while retaining the original error internally; direct live snapshots retain their real error output. Reference replay duration conversion (created + completed - ran) is covered. Core tests: 50 passed, one ignored performance test.
- `tool_view.rs` replaces the old placeholder renderer with Source tool-kind summaries, expandable execution output/status and patch/diff payloads. Old unused Markdown/notice rendering helpers were removed; terminal/diff helpers remain. Success/failure cards were exercised in both apps. The source footer computed height is 21px including 6px padding, confirming a 15px content line. Added the original checkmark geometry and accessible output labels.
- Tool workspace gate: 168 passed, one ignored performance test (`tool-view-workspace-tests.txt`). Clippy passed before the last checkmark/accessible-label/footer/history adjustment; rerun pending. Patch UI, nested large-output scrolling, keyboard/focus matrix, shimmer, full file/metadata retention and tool screenshot acceptance remain unfinished.
- Current fixture server: 49569, `/private/tmp/vela-tools-fixture`, `canonical-tools-history.json`; session 78329. Previous 49568 images fixture and earlier fixture servers remain intact. Reference QA is running on tool history with Work and both execution rows expanded, Files, no visible terminal, 900x700, panel 325px. Native QA was stopped for the final tool-card rebuild.

### Tool continuation after the first gate

- Final `tool-view-clippy-v3.txt` passes with warnings denied. Latest binary includes the reference 4px work gap plus 8px process-body padding (12px first process-row gap), success check asset, 15px footer content line, accessible output and vertical patch lines. Full 168-test workspace pass precedes only the final gap/diff flex layout adjustment; real GUI checked those changes.
- `canonical-tools-history-v2.json` adds a patch diff and a 240-line output, including a wide first line. Server 49570, state `/private/tmp/vela-tools2-fixture`, owned session 51117. Native QA session 75329; Reference QA session 39625. Both use the existing isolated profiles/config/data and fake CLI. Other fixture servers are preserved.
- Native and reference patch rendering show hunk, unchanged, deleted and added lines; both nested execution output viewports reach line 239. Native output remains bounded with the success footer outside its scrolling viewport. Horizontal scrolling, non-diff patch output, long diff scrolling, selection, complete keyboard/focus/a11y and shimmer remain pending.
- `transcript-tools-replay-v1` is NOT eligible: the successful execution card is collapsed in native and expanded in reference. Its raw captures are retained. `capture-reviews.json` records this semantic exclusion. Comparator now consumes exclusion reasons in addition to automatic capture checks; reviews can only disqualify cases, never mask pixels or grant acceptance.
- `transcript-tools-patch-v2` is NOT eligible due to ICC mismatch: the reference reopened on another monitor (bounds x510/y99), while native remained x2108/y163 on 24G1WG4. Reference was moved via Window > Move to 24G1WG4 and recaptured as v3. V2 diagnostic: 141519 exact changed pixels, 4.275873% above8, MAE2.661792. No acceptance.
- CUA screenshot-only observations invalidate AX indexes. Always read a fresh full AX tree before index-based actions. Raise native QA using the exposed window Raise action before interaction inspection; occlusion can otherwise leave a stale rendered window snapshot until exposed. Do not reuse stale menu/row indexes. In CUA, redeclaring `var refQA` later in the same cell shadows the old binding before earlier actions; split rebind into its own call.
- At recapture: both apps show Files, expanded Work and only the patch card open, large output collapsed, transcript top, 325px agent panel, 900x700 content. Reference DevTools closed. V3 metrics pending. No page visually accepted.

- `transcript-tools-patch-v3` is capture-valid: full 900x700 windows, both inactive, stable manifests, matched ICC59ee293f. 211387 exact changed pixels, 26302 above8 (4.1749206%), MAE2.5293067. This is not a visual match. Report: 139 cases, zero exact matches; semantic exclusion for tools-replay-v1 is applied. Both apps remain running on tools2 fixture with patch open and large output collapsed.


## 2026-10-29 structured user-message continuation

- Fixture 49571 (`user-markdown-history.json`, state `/private/tmp/vela-user-markdown-fixture`, server session48381) restores a user heading, strong/codespan, list, quote, embedded PNG, fenced Rust code and final paragraph, followed by an assistant answer. The old native build visibly flattened the list and dropped the image.
- Native user bubbles now use the shared structured renderer and the reference column/end alignment, keeping the 77% width cap. Plain-message intrinsic measurement remains direct. User code uses the source plain wrapped pre style (12.25px/18.375px, padding12, no assistant header/buttons). Root block margins collapse according to source CSS; marked paragraph margins in lists/quotes are represented explicitly. Image-only first/last paragraphs retain child image margins. Plain bubbles retain their label; rich descendants expose heading/text/image/code labels. Complete list/table accessibility remains pending.
- Markdown rendering now depends on App + a weak optional owner rather than a borrowed VelaApp Context. Copy/wrap updates route through that owner; keyboard single-activation regression still passes. This lets rendered user-layout tests exercise the actual shared bubble and document helpers without starting an engine. Tests cover narrow/wider widths and structural height.
- A new rendered flow regression reproduced trailing code whitespace incorrectly pushing a fitting word onto the next line (`inline-code-space-before.txt`). Flow now separates break spaces from words and excludes discarded spaces from wrap decisions; reflow preserves Unicode graphemes and non-whitespace text. Source and native now break the user codespan at the same word. All48 UI tests pass (`user-markdown-ui-tests-v4.txt`).
- Full workspace:171 passed, one ignored performance test (`user-markdown-workspace-tests.txt`), before only final 16px column top and border-box image geometry changes. Latest UI48 and Clippy with warnings denied pass after those final changes (`user-markdown-clippy-v3.txt`). Build/package passed.
- The presumed font compensation was removed: transcript top padding now matches source `.vela-column`16px. Reference DevTools confirmed a 400x100 PNG has a200x51.5 border box, content ratio inside1px borders. Native image fitting now constrains content to198px and adds2px borders; portrait is51.5x200, small16x12 content is18x14. Earlier image-fit tests and geometry assumptions (200x50) were incorrect and updated. Assistant image fixtures need recapturing after this shared correction.
- Current running native QA session22632 includes all latest changes. Reference QA session78256 on same49571 fixture; moved to24G1WG4 after restart. Both900x700, Files, no visible terminal,325px agent panel, full user message visible. DevTools closed. User-pair lossless capture in progress. Current visual gaps include a small image/remaining vertical offset, exact glyph raster/color and shell/composer differences; no page accepted.


### Drag-exit and input-focus defects found during capture review

- `transcript-user-markdown-v1` is NOT eligible for acceptance: native shows an external attachment drop overlay while reference does not. Preserve it as diagnostic. Raw metrics:255494 exact changed pixels,8.523016% above8,MAE3.474655. Added an exclusion to `capture-reviews.json`; rerun comparator after recapture. Report currently140 cases, zero exact matches.
- Actual framework FileDrop Entered/Exited regression reproduces stale attachment-owner state (`attachment-drag-exit-before.txt`). GPUI's Exited branch clears active_drag without dispatching a final mouse event or invalidating the attachment view. A drag-scoped owner lifetime observer checks the authoritative active drag and clears/invalidates the target when it ends; it is not a timeout that hides an active hint. Escape now cancels at the root capture phase before a focused child can consume it. Both platform-event/keyboard regressions pass (`attachment-drag-lifecycle-tests.txt`); the active-drag persistence assertion was strengthened afterward and needs the final UI rerun.
- Added heading names/levels, List/ListItem/ListMarker roles and positions, Table/Row/ColumnHeader/Cell roles and coordinates. Table headers use700; strong text inherits CSS relative weight (400->700,600/700->900) rather than flattening all strong runs to one weight. Native real AX now exposes named Heading and list markers/items. Table AX/live assistant code-button validation remains pending.
- Latest gate before the input-focus work:50 UI tests pass (`drag-and-markdown-semantics-ui-tests.txt`), Clippy warnings denied passes (`drag-and-markdown-semantics-clippy.txt`), build/package pass. Native running session38952 includes drag lifecycle and Markdown semantics, on49571 user fixture. Its current file dialog was opened only to verify Tab reached Attach, then cancelled without selecting files.
- Tab did reach Attach (Return opened its file dialog), but the retained input's old caret drawing could remain after focus moved. Input now subscribes once per owning window to focus/blur/activation notifications and invalidates its own cached paint; caret painting also requires an active window. Code is not yet in the running binary. Focus regression setup must activate and draw a real window frame; inactive test windows deliberately suppress focus-path events. Latest test is `input-focus-paint-after-v3.txt`, pending inspection. Earlier variants with inactive windows were not valid positive checks. Caret blinking, OS input indicators, selection color/IME and full input visual matrix remain unfinished.


### Input repaint and keyboard outline continuation

- Final input-focus regression activates/draws a real frame and checks blur, regain and deactivation invalidation. It passes with the subscriptions and fails when they are removed (`input-focus-paint-proof-without-fix.txt`). Live Tab reaches Attach and the subsequent lossless capture clears the composer caret. Immediate automation snapshots can still show stale pixels, so use exposed-window lossless captures for review. Caret blinking and IME/selection matrices remain pending.
- Composer Attach/agent/model/strength/send controls now have persistent focus handles and an external outline renderer. Reference Attach overrides the global 1px outline with 2px; both use a 1px offset. No layout size is changed. GPUI clears its keyboard modality on pointer movement, unlike CSS focus-visible. A per-window keyboard mode now persists through pointer motion and clears on pointer down. The real rendered-controls regression passes and fails against GPUI's original predicate (`keyboard-outline-motion-without-fix.txt`). Live Attach shows the blue outline.
- Latest UI suite:52 passed (`composer-outline-ui-tests-v2.txt`). All-target workspace Clippy passes with denied warnings (`composer-outline-clippy-v2.txt`). Build/package pass. This is not a fresh full-workspace test count; the last full gate remains171 passed/one ignored.
- `transcript-user-markdown-v2` is capture-valid, same ICC59ee293f,900x700,inactive, no drop overlay.232328 exact changed pixels;28161 above8(4.47%),MAE3.082620. No acceptance. Report144 cases,zero exact matches; diagnostic native-only focus captures increase the case count.
- Reference inspector reconfirms image border box200x51.5 and content198x49.5. GPUI Img paints its bitmap over the border area; native now separates the frame from the content image. This last change needs rebuilding/live recapture. Small vertical differences and toolbar truncation remain. Assistant tables/Copy/Wrap and image corpus still need shared-renderer rechecks.
- Current native QA session68688 includes focus changes but predates the new image frame; source session78256 on49571 user history. Source inspector is closed. Both isolated profiles are preserved, no files attached/submitted.


### Structured controls, Window menu and spacing continuation

- Fixture49572 (`structured-controls-history.json`, state `/private/tmp/vela-controls-fixture`, server session92613) adds checked/unchecked/nested tasks, strong table headers/body and long Unicode code. Native tasks previously flattened into separate Unicode checkbox-symbol lines. They now use13px noninteractive checkbox geometry, reference task/body spacing, dark/light contrast and direct/paragraph-contained marker detection. Task states do not leak from a nested child into a normal parent. Tight/loose/nested regression passes. Actual native checked/unchecked/nested boxes are visible (including the full-height Zoom view). Exact checkbox raster/layout acceptance is still pending.
- Real code Copy through the refactored weak-owner callback retained the Unicode fixture, all12 repeated columns, blank line and Last line. Verified by pasting into an empty native QA composer; the test draft was cleared and never sent. An earlier failed click pasted unrelated preexisting clipboard text; that draft was also cleared. Do not count the first attempt as code-copy evidence. Code Wrap callback needs a fresh live check after resetting viewport; previous click coordinates became stale after composer resizing.
- Added native Window menu Minimize(cmdM),Zoom,Bring All to Front. AppKit automatically adds window list, display movement, placement and fullscreen commands. Bring-to-front uses the application/window owner after deferral, avoiding a nested window borrow. Live Zoom expanded to1920px and restored900x700 at1930,163. Minimize/restore and full Window-action matrix are pending; invoking Minimize alone did not yet establish a verified minimized inventory state.
- Scoped transcript Home/End,PageUp/Down,CmdUp/Down and arrow scrolling implemented for transcript/code controls. Text inputs keep their own editing keys. Real rendered-list regression passes. Viewport height is recorded separately from transient gutter measurements so a page operation cannot use a stale fallback during reflow. Native live keyboard check pending.
- Fixed shared code-card background (body now uses inset), inherited22.75px header/button line-height,1px header separator and2px actions gap. Fixed image pixel content versus border frame. User image paragraph bottom margin now14px before another block,12px when it is the final image, matching the child/paragraph margin collapse. Removed duplicate rule margins. Assistant virtual rows now collapse actual block-specific margins instead of always using14px. Task-list parent ending in a nested list avoids adding a second paragraph-bottom margin.
- Spacing metadata cached per stable row ID/text revision; streaming revisions overwrite entries and session/history size changes bound retained old text. Cache revision/release regression passes (`markdown-spacing-cache-tests.txt`). Latest full UI55 passed before only the new cache-specific test, so currently55+one focused new test, not a fresh56-suite pass. Last all-target Clippy passes (`markdown-spacing-clippy.txt`); final formatting/test-only additions afterward need final gate. Current build `markdown-spacing-build.txt` includes runtime changes but not yet packaged/launched.
- Native QA session7738 is on49572 at900x700 after Zoom restoration; binary predates keyboard paging and final spacing/code-card changes. Reference QA session75663 remains on49572; at last on-screen window inventory it was absent, so do not accept new captures until it is exposed again. Native CUA can return stale rendered pixels while model/AX changes; trace logging proves focus=true/keyboard=true despite old screenshot. Window menu now provides an actual makeKeyAndOrderFront action. Need a fresh exposed-window comparison, not more acceptance based on cached pixels.
- User-markdown-v3 preserved as diagnostic: native Attach outline absent while Source outline visible. Added semantic exclusion. Comparator needs rerun after that exclusion. V2 remains eligible but differs; no page accepted. Report last regenerated145cases,zero exact matches (confirm from report rather than inferring count).
- Temporary trace diagnostics in `focus.rs` log keyboard outline geometry and scroll event metadata at TRACE only, no prompt text. Current live log `window-menu-live-trace.txt`; new binary will include scroll event tracing. Future launches should avoid shell redirection when unnecessary so the existing approved run-parity prefix can be used.


### 2026-10-29 evening: horizontal code and virtual-row integration

- Full permitted offline workspace gate passed179 tests,zero failures,one ignored performance test (`structured-controls-workspace-tests.txt`). This predates the horizontal-scroll additions below. Latest complete UI gate58 passed (`horizontal-drag-removal-ui-tests.txt`), Clippy denied warnings passed and build/package succeeded.
- Shared AutoScroll supports vertical/horizontal axes with the same8px gutter,4px inset thumb, hover, dragging and track paging. Horizontal code gutters notify their layout owner when added/removed. Text painted outside its no-wrap box previously did not expand GPUI's scroll range. Code now measures intrinsic line width and gives the text a corresponding minimum width. Actual widget regression passes and fails without the width fix (`code-intrinsic-width-without-fix.txt`). Wrapped code restores offset0.
- Removing overflow during an active thumb drag reproduced a retained drag (`horizontal-drag-removal-before.txt`). Shared prepaint now clears drag/hover ownership when the gutter disappears. Horizontal resize/drag and existing vertical suites pass.
- Code controls use shared1px keyboard outline/1px offset, no extra focused background. Attach outline corner radius now excludes its internal border correction (position includes the parent border, CSS outline radius does not). Assistant code text now exposes its full source in AX. Live AX includes full Unicode/repeated code text; Home navigation and Wrap callback verified in the real app. Native/Source Home put the same user/task/table/code structure at the top.
- `transcript-structured-controls-v1` is capture-valid900x700,inactive,sameICC59ee293f,stable full windows; both unwrapped (Wrap label),code Wrap keyboard focus,top,Files,no visible terminal,empty composer.197662 exact changed pixels;34370 above8(5.45556%),MAE3.557437. Not accepted. Report146 cases,zero exact matches. User-v3 semantic exclusion now applied.
- Baseline native code still ends about6px before Source despite the standalone code-widget test passing. Do not claim that its live gutter height is accepted. Added a real document-in-virtual-list test, and passed ListState/layout-owner notification directly into Markdown RenderContext rather than updating VelaApp through a weak owner for gutter layout. New `document()` helper is shared by production and integration fixture. Virtual row gutter insertion/removal test passes (`virtual-code-layout-test.txt`); full UI/Clippy/build are currently running under `virtual-code-layout-*`. This latest runtime layout change is NOT yet in a running QA binary. Native was stopped for repackaging.
- Reference QA running session17023 on49572,unwrapped,top,Wrap keyboard focus. Native just stopped session24097; last bundle has58-test horizontal implementation. Next: await virtual-code-layout gate,package,launchnative samefixture,verifyactualcodeend/scrollreach,recapturepairedv2beforeanycolorchanges.
- Pixel-frequency investigation: user-v2 has135626 pixels where Source(252,251,252) differs from Native(252,252,252),plus51183 Source(243,243,243) vs Native(241,241,241). A color-management hypothesis is pending, not an accepted correction. GPUI MetalRenderer creates BGRA8Unorm CAMetalLayer without setting colorspace. Installed Apple CAMetalLayer.h states nil disables color matching. No colorspace code/dependency changes made. Preserve before/after full-window capture methodology; do not tweak palette hex values based on a single display.
- objc2-quartz-core0.3.2 is cached/locked; objc2-core-graphics is not cached/locked. NSView.layer() is gated by objc2-quartz-core; CAMetalLayer.colorspace/setColorspace and NSColorSpace.CGColorSpace methods are gated by objc2-core-graphics. Use typed native APIs if adding this integration; do not edit GPUI dependency checkout. Color hypothesis remains behind completing the current layout check.


### Coupled gutter root cause found in real QA

- Direct ListState invalidation and the initial virtual-document test alone did NOT settle the real QA height. Do not treat that59-test gate as proof of a live gutter fix. Metadata-only trace in session57850 exposed a continuous loop: inner horizontal width/height274x100(no gutter)->282x108(gutter applied)->274x108(parent vertical gutter back)->274x100(horizontal removed). Extent remained positive1596/1604px throughout. Selected actual trace lines saved in `coupled-gutter-live-before.txt`. Native diagnostic run was stopped.
- Expanded virtual fixture includes ListViewport, but final settled-frame assertions alone still passed the old code. Strengthened parent-width test counts gutter ownership changes, proving it was removed/reinserted during a still-overflowing width change (`parent-width-probe-before-v3.txt` fails3 vs1). Replay of the exact live geometry also fails at274x108 (`recorded-gutter-loop-before.txt`). Earlier parent-width test v1/v2 and `coupled-gutters-before.txt` did not reproduce; preserve them as incomplete harness checks, not negative evidence.
- ScrollState now owns `observe_geometry`. For horizontal overflow, the scrollbar reserves height and cannot create the horizontal overflow it measures, so a positive measured extent keeps the gutter without a destructive width probe. Vertical behavior remains unchanged. All six focused gutter/integration tests pass (`coupled-gutters-after.txt`), including drag removal and both prior vertical regressions. Full UI/Clippy/build currently running in `coupled-gutters-*`.
- `transcript-structured-controls-v1` now explicitly excluded: native was oscillating, so its layout was not settled. Raw capture and diagnostic metrics retained. Regenerate report after next pair/review. No accepted page, no color-management edits. Next: package/launch axis fix with scroll trace, demonstrate finite settling and real horizontal scrolling, capture matchingSource/nativev2. Native currently stopped; source session17023 remains on49572,unwrapped,top,Wrap keyboard focus.


### Settled gutter verification and color-space probe

- Latest complete UI gate61 passed, Clippy warnings denied passed, native build/package passed (`coupled-gutters-*`). Live session74252 settled at274x108 with positive1604px horizontal extent and no repeated insertion/removal loop. Actual scrollbar drag reached the far edge and returned to offset0. Native and Source restored unwrapped code, transcript top and Wrap keyboard focus before capture.
- Eligible `transcript-structured-controls-v2`:900x700,inactive,sameICC59ee293f,stable full windows.196347 exact changed pixels;33034 above8(5.243492%),MAE3.4737357. Report147 cases,zero exact matches. V1 oscillating-layout exclusion applied.
- Tested typed app-level CAMetalLayer sRGB declaration without modifying dependency sources or palette. Actual layer inspection proved the existing profile already equals sRGB; no setter ran, and sampled backgrounds were unchanged in `native/transcript-srgb-probe.png`. Removed the ineffective adapter and its two direct dependencies. The experiment is diagnostic only; color mismatch remains unresolved. Diagnostic logs were emitted in session12592, which was stopped.
- Source DevTools inspection confirms root codex theme tokens; a read-only Console paste was blocked by its paste safeguard and was not executed. Did not disable the safeguard. Developer Tools closed. Table sizing, horizontal keyboard navigation and accessibility remain the next work.


### Native tables: keyboard and intrinsic sizing continuation

- Added focusable Scrollable table region with Left/Right40px and Alt+Left/Right viewport paging; horizontal AutoScroll owns track/drag/offset and virtual-row gutter remeasurement. Real rendered navigation/reset regression passes and fails without its key handler (`table-keyboard-without-handler.txt`). Initial table-keyboard-before was a harness compile error, not a reproducer.
- Replaced unconditional max-content table widths with Unicode min/max measurements and proportional flex growth/shrink, plus shared collapsed border ownership. Breakable prose now wraps rather than forcing3174px overflow; the regression fails with min-content disabled (`table-without-min-content.txt`).
- Table keyboard focus outline uses the shared1px external outline, with square corners. Keeping its canvas inside the scrolling box introduced4px spurious overflow; moved the outline to an outer focus region and kept the actual viewport separate. Both table regressions pass after that correction (`table-outline-layout-tests.txt`); final UI63 passed (`table-final-ui-tests.txt`). Earlier full workspace185 passed/one ignored and Clippy passed before final intrinsic/outline changes; no newer full-workspace count claimed.
- Fixture49573 now uses an unbroken WIDE identifier plus trailing Unicode and END_OF_TABLE sentinel, avoiding repeated underscore emphasis in the first attempted fixture. State `/private/tmp/vela-tables-fixture`, history `wide-table-history.json`, server79110. Source session93042 and native71630 were live on it. Source moved to24G1WG4 through Window menu. Live Source and native Alt+Right both reach the final column.
- Native CUA snapshots lag actual rendered input. Initial apparent failure to focus/scroll was disproved by the full lossless native table-keyboard-live-diagnostic capture, which showed focused outline and80px movement after twoRight inputs. Native far-edge lossless diagnostic confirms a real separate defect: Retained wraps its last letter, while Source retains the word on one line. Two-column/fake-font harness checks did not reproduce this (`table-shaping-before.txt` passes; three-columnv2 failed only an unrelated250px overflow expectation). Do not describe those as reproducing the live word split. Current harness is being tightened to the284px live viewport.
- Native also has an extra66x20 macOS Window indicator over its traffic lights; default capture selection properly rejected the ambiguous inventory. Explicitly selected main Vela window51541 for diagnostic only, preserving the full900x700 capture and complete inventory. table-far-edge-diagnostic pair is semantically excluded due to this native-only overlay/focus mismatch; no masking or acceptance.
- Next: inspect live intrinsic measurements, fix the actual word-fit ownership, rebuild and compare far-edge and left-edge states. Native71630 is currently running older table build; metadata-only table intrinsic trace build is pending. No color adapter remains. No page accepted.


### Live table-width cause isolated

- Trace native session4804 verified real font12.25px/line-height22.75px. Intrinsic columns: min/max3021.8674/3039.8674,75.32947/75.32947,118.27625/136.27625. The middle cell snapped to75px border box, leaving54px content for a54.32947px word. Also, unnormalized flex-shrink factors summed below1, so Flexbox intentionally left negative free space undistributed, retaining about15px excess in the first column. This explains both the split word and shifted final columns. The earlier possible per-character wrapping hypothesis is not established as this live defect cause.
- New helper normalizes shrink factors, preserving min/max-range weighting while distributing all negative space. Intrinsic bounds round upward to the actual device-pixel grid. Cell contents now vertically center like the reference table. Shared production/helper rendered regression using recorded widths fails3037.5 vs3022px with normalization removed (`table-recorded-sizing-without-normalization.txt`) and passes with it (`table-live-sizing-after-v2.txt`). Test platform defaults2x; recorded1x geometry is explicitly snapped as1x in that fixture. Previous v1 failure75.5 vs76 was the harness2x assumption.
- Three table tests pass: recorded sizing, breakable prose and arrow/paging/clamping/resize reset. Updated fixture starts at284px and widens to600px to genuinely remove overflow from the full three-column table; prose fixture350px accommodates the test font min-content widths. Earlier three-column test failure at250px only reflected remaining true overflow.
- Metadata-only diagnostic canvas currently remains in table cells for one live verification and should be removed afterward; no prompt text logged. Native4804 stopped; normalized build/package pending. Source93042 still on49573 at far-right table edge with focus outline. Need real before/after word fit and paired left/right full captures.


### Table fixes verified in the real app; fractional typography next

- Normalized/fitted build live native76565 rendered Retained on one line at the far edge, with last-column text at771px matching the reference last-column text start. Cell vertical centering now matches the reference presentation. Actual trace settles at min/max3022/3040,76/76,119/137 on1x. Real native and Source Alt+Right reached END_OF_TABLE; subsequent native mouse scrollbar drag also reached the last column.
- Diagnostic canvases and intrinsic TRACE/DEBUG logging were removed. Full UI64 passed before only diagnostic-hook removal (`table-normalized-ui-tests.txt`), and all three table regressions pass after removal (`table-clean-tests.txt`). All-target workspace Clippy denied warnings and clean build/package passed (`table-clean-{clippy,build}.txt`). No dependency-cache changes or color adapter remain.
- Current clean native92123 on49573 (INFO only); Source93042 same fixture, both Files900x700,325px panel,empty composer,table at far right with pointer focus/no blue outline. Native window51855/pid77412 at1930,163; Source51455/pid66305 at2430,217. Both on24G1WG4.
- `transcript-wide-table-end-v1` full pair retained with source/binary/ICC manifests:189756 exact changed pixels,27623 above8(4.384603%),MAE2.8979516. Excluded from acceptance: different app activation states and visible macOS control indicators over title bars. The indicator was also observed on Reference, so the earlier native-only description applies only to the earlier diagnostic capture, not to the OS behavior generally. CUA runtime reset did not remove the indicators; no privacy settings changed or pixels masked. Explicit main window IDs used because default capture guard rejects the additional66x20 Window.
- Comparator regenerated151 cases,zero exact matches; both table diagnostic exclusions applied. Native table is still about1px lower at its top and about3px lower after the table, and broader font/color/shell differences remain. No page accepted.
- Next work began: two-line actual Markdown regression for CSS22.75px fractional line-height accumulation. GPUI standard StyledText snaps line height before multiplying line count; shared InlineFlow already supports fractional metrics for codespans. Do not change the shared renderer until the new regression result is inspected. Pending test session61043, output markdown-fractional-line-height-before.txt. No shared renderer changes made yet.


### Fractional Markdown line height reproduced and fixed

- Actual rendered two-line paragraph regression fails45px vs CSS45.5px using the old StyledText path (`markdown-fractional-line-height-before.txt`). Switching all Markdown inline text to shared InlineFlow alone failed46px because TextStyle.line_height_in_pixels rounds to whole points (`markdown-fractional-line-height-after.txt`, first full UI64pass/1fail).
- InlineFlow now resolves line_height.to_pixels directly, preserving fractional leading until the containing block is laid out. All nonempty inline Markdown uses this shared shaper; empty inline content retains zero content height. Link targets/ranges and emphasis/codespan runs remain passed through it; AX labels/heading/list/table roles remain on the existing parent nodes. `markdown-fractional-line-height-after-v2.txt` passes the45.5px regression. Full UI65 passes (`markdown-fractional-ui-tests-v2.txt`), all-target Clippy warnings denied passes, diff-check passes. Build/package underway.
- Verified Source lexer explicitly uses breaks:true (`markdown-parse.ts:40`), so native SoftBreak/HardBreak newlines are correct and were not changed. No speculative whitespace correction made. Code block plain String/StyledText line-height and full link/selection/typography/large-paragraph performance matrix remain pending.
- Current native92123 stopped for the new renderer package. Source93042 remains same49573 fixture at table far right, pointer focus, empty composer. New build session78337 completed successfully before packaging. Next: launch new native, inspect actual table/header/body offsets, paired capture and mixed-inline/image corpus rechecks.


### Fractional-flow live regression and production bubble harness

- Live native47593 showed fractional renderer shrinking the user bubble from the reference219px to about199px. Native measured the longest painted wrapped line, then the column-aligned bubble shrank a second time. The existing handmade row-oriented bubble harness did not reproduce it, even with tighter width assertions (`markdown-flow-bubble-width-before.txt`, `markdown-flow-bubble-without-width-fix.txt` both pass). Those are incomplete harness evidence, not negative live evidence.
- Replaced the duplicate harness with actual production bubble()/user_inline(), and added test-only debug selectors to inspect its real border box. Actual production regression now fails209px vs218..219.5 with the width fix removed (`markdown-production-bubble-without-width-fix.txt`). New InlineFlow intrinsic measurement retains min(unwrapped longest-line width, available wrap width), excluding trailing break whitespace. A wrapped bubble keeps its constrained width; short content keeps its intrinsic width. Explicit newline measurement uses the longest line.
- Production bubble, short-message and structured-bubble tests pass (`markdown-production-bubble-after.txt`). Full final UI suite in markdown-production-flow-ui-tests.txt pending inspection; Clippy running in markdown-production-flow-clippy.txt. Native47593 is still the prior fractional build with the too-narrow bubble; do not capture it as final. New intrinsic-width fix not yet packaged.
- No Source whitespace changes: breaks:true verified and preserved. Code block String/StyledText fractional leading, native image/Markdown corpus, full interaction/selection and large-paragraph performance still need follow-up. Last report151cases,zero exact matches.

- Follow-up gate: full UI66 passed (`markdown-production-flow-ui-tests.txt`), longest-explicit-line short bubble equality passes (`markdown-short-bubble-longest-line.txt`), final all-target Clippy warnings denied passes (`markdown-production-flow-clippy-v2.txt`), build/package and diff-check pass. Initial Clippy failure was unnecessary_lazy_evaluations on a pure unwrap_or_else; replaced with unwrap_or. Runtime behavior unchanged by that lint fix. New native launch follows packaging; native47593 was stopped.


### Netflix interruption: stop all capture and GUI work

- User reported that current work prevented Netflix playback and froze the video. Immediately stopped native47528 and Reference93042; both exited. Reset CUA JS session. No further screen capture, UI automation, GUI app launch or rendered GUI tests while playback is in use. Continue code-only work. Do not assume the media resumed without user evidence. Active capture may interfere with protected streaming; exact cause not independently verified.
- Current source includes answer-scoped Markdown focus ownership and footer focus-within logic. Rendered regression passes and fails with scope visibility removed (`answer-footer-without-focus-within.txt`). Full UI67 passes (`answer-footer-ui-tests.txt`), Clippy denied warnings passes, native build completed (`answer-footer-build.txt`), but this newest footer build is NOT packaged or live-verified. Prior running clean app47528 stopped.
- `transcript-wide-table-end-v2` is a valid semantic comparison: both inactive, same table keyboard focus, full900x700,same ICC, both OS control indicators present. The differing footer is a real implementation defect and was NOT excluded. Raw190640 exact changed pixels,28381 above8(4.504921%),MAE2.9832659; report152cases,zero exact. Current footer fix still needs future live check when screen use is appropriate.
- New native BlockContext explicitly passes answer identity across virtual rows. Scoped control focus handles retain enter/exit subscriptions and invalidate their layout owner. Table pointer focus explicitly notifies that owner. User/reasoning contexts carry no answer scope. Regression covers adjacent answer code/table controls, Copy controls, out-of-answer reasoning and outside focus. No capture or app relaunch to validate this now.

### Resume with approved Accessibility input and one-shot captures

- The preceding stop instruction was superseded by the user's explicit request to resume and approval of scripts/parity-accessibility.swift. After closing QA apps and resetting CUA failed to restore Netflix video, the lingering owned SkyComputerUseService and its workers were stopped; the user confirmed playback recovered. Do not restart CUA or its persistent capture service. The new helper only targets the two QA bundle IDs, uses Accessibility trees and guarded QA input, and starts no recording session. Lossless one-shot macOS window screenshots remain explicitly authorized. Verified SkyComputerUseService absent during this continuation.
- Helper supports guarded AX actions, keyboard input, pointer movement/clicks and window bounds. Mouse delivery required explicit click count and a short event-delivery interval. Initial unverified actions were not accepted as successful. Native AX content is lazily exposed after activation. Label-substring inspection avoids truncating the large explorer tree. Raise occurs before deriving click bounds.
- Fractional code renderer now accumulates CSS leading without integer rounding: assistant12.5/18.75px and user12.25/18.375px. Production code-body regression fails98px vs99px before and passes after (code-fractional-leading-{before,after}.txt). UI68, Clippy and build pass. Raw text/Copy and existing shaping preserved; tab-size2, trailing-newline behavior and syntax highlighting still need parity work. No dependency checkout changes.
- Live footer focus-within passes with the table focused and pointer outside the answer (native/table-focus-outside-pointer-v1.png). Source/native transcript-wide-table-end-v3 are matching active900x700, Files, far-right keyboard-focused table, same ICC5284199b; no capture-service badges. Retained stays on one line and END_OF_TABLE is reachable. Full raw pair189897 exact changed pixels,27898 above8(4.428254%),MAE2.9725103. No page accepted. Report157 cases before later captures. Earlier fractional-footer-live-v1 has Sessions/Files mismatch; table-focus-outside-pointer-v1 has outline modality mismatch; both explicitly excluded as diagnostic pairs.
- Live comparison exposed another real footer spacing defect: unconditional14px trailing margin for every multi-block answer. Source CSS only overrides the last-child zero bottom margin for adjacent final paragraphs; final images retain their internal12px image spacing. Production row-projection regression fails20 vs6px for heading/table/final-paragraph before the fix (footer-final-margin-before.txt), then passes for table, adjacent paragraphs, single paragraph, final code and image. BlockSpacing now resolves document-tail spacing from actual Markdown nodes across virtual rows; reasoning tail uses the same rule. UI69 and workspace all-target Clippy denied warnings pass. Build footer-final-margin-build.txt pending live/package verification. Native QA closed cleanly with CmdQ; reference QA89530 remains running on fixture49573, far-right keyboard-focused table.

### Footer spacing live verification and code tab stops

- Footer final-margin build/package passed. Real native/reference transcript-wide-table-end-v4 confirms the extra14px is removed; remaining footer vertical difference follows the roughly3px accumulated table/body offset. Eligible active900x700/sameICC pair190695 exact changed,27818 above8(4.415556%),MAE2.9673437. Workspace tests initially failed8 engine tests under sandbox (socket PermissionDenied, no preview ports, filesystem event timeout); unrestricted rerun passes192 tests, one ignored (footer-final-margin-workspace-tests-unrestricted.txt). No accepted page.
- Current code fixture49574 state/private/tmp/vela-code-spacing-fixture, history code-spacing-history.json, server session23652. Reference95177 session22078 at2430,217; native98769 session1796 at1930,163. Both Files,325px agent,900x700,empty composer focus,pointer outside answer. Older49573 fixture remains idle; do not broad-kill processes. Native argv has workspace /private/tmp/vela-close-workspaces/many and final positional files (view), not a path containing spaces.
- Live native code tabs used default platform stops (B about637 vs reference624). PreformattedText now lays out preserved tabs against measured two-space stops with the CSS minimum half-character advance, using the actual shaped glyph widths including fallback emoji. Copy/AX retain raw tabs. Same layout determines no-wrap scroll extent. Non-tab text retains existing GPUI shaping; long tab-containing atoms split into bounded shaped row chunks rather than repeatedly cloning the remaining line. CSS reference: https://www.w3.org/TR/css-text-3/#white-space-phase-2 . Complete bidi/tab/soft-wrap/performance matrix remains open.
- Existing production geometry test code_tabs_use_two_space_stops passed BEFORE the fix with the test font (code-tab-stops-before.txt); it does NOT reproduce the native font defect. Real screenshot before/after is the reproducer. Added fractional stop/half-character rule checks. UI71 passes (code-tab-ui-tests-v2.txt). Initial test compile failed because glob-imported GPUI test macro shadowed the built-in attribute; use explicit core test attribute. Initial Clippy failed test-module placement; moved it to end. Final code-tab-clippy-v3.txt passes denied warnings. Native build/package passes; test-only module move occurred after packaging and changes no runtime code.
- Paired transcript-code-spacing-v2 confirms B,C,D tab positions now match the reference including emoji fallback. Valid active900x700 sameICC5284199b:188677 exact changed,26322 above8(4.178095%),MAE2.5670885. Report160cases,zero exactmatches. V1 code pair excluded because reference sidebar was collapsed while native sidebar was open. Native code-block overall vertical/color/font differences remain; no page accepted.
- Clipboard check initially read a different value after a managed approval delay; no clipboard contents printed and no cause assumed. New approved helper verify-copy atomically AXPresses a guarded QA Copy control, checks Copied acknowledgement, and compares clipboard to the raw QA code label without printing either. Native paths window/0/36 and window/0/37 pass matches_raw_code=true,expected_tabs=3. This avoids attributing changes during a delayed separate read to the app.

### Shared mouse authorization withdrawn

- User explicitly objected to moving their mouse and demanded use of a separate mouse. Immediately stopped all macOS mouse/keyboard automation. The Accessibility helper uses the shared physical cursor; earlier permission to use the helper does not authorize further shared input after this correction. Do not invoke helper clicks/hover/keys, other shared-input automation, or restart the persistent CUA capture service. Continue code work without taking user input. A future UI method must actually provide independent input and avoid the Netflix recording interference; do not assume the helper has a separate cursor.
- Wrapped-code fixture49575 is currently running (server session12619), state/private/tmp/vela-code-wrapped-tabs-fixture and code-wrapped-tabs-history.json. Native session51933 pid4008; reference1089 pid4024. These QA processes were left alone rather than moving or closing windows through shared input. Native transcript-wrapped-tabs-v1 PNG exists but capture validation failed because activation/bounds changed during the capture; it has no valid manifest and must remain diagnostic. Reference one-shot capture session54311 was already in flight when input authorization was withdrawn. No further UI actions authorized through shared input.

### Restored app-scoped Computer Use; actual Markdown font mismatch fixed

- User reset the desktop app and explicitly authorized retrying its independent app control. CUA getState now succeeds and app-scoped bindings to both QA apps work. Use cua_repl for UI; do not use scripts/parity-accessibility.swift input or CGEvent/global cursor synthesis. Earlier tool transport failures were real. Current authorization supersedes the prior no-CUA workaround, while the shared-mouse prohibition remains. Do not kill CUA workers as a routine reset or claim independent tool transport is unavailable without trying the actual cua_repl endpoint (it is not enumerated in ALL_TOOLS).
- Restarted wrapped-tabs fixture49575 session60508; source session2127 pid21143; current clean native session13758 pid24673, main53922. Source main53812 moved through Window menu to24G1WG4. No shared-input helper used. Reference session opened with CUA and Files selected. A source shell:switch-agent Unknown fixture endpoint toast appeared transiently; not treated as native failure or hidden from screenshots. Terminal tree remains present but dock is visually closed.
- Native rendered END_OF_CODE on an extra row. Width-retention experiment and its measured-width test passed but did NOT correct the real app. Reverted that experiment/test entirely; code-tab-width-after-v2.txt proves only the abandoned harness behavior, not a fix. Metadata geometry run session53105 showed actual width258px and character advance7.7270513px, disproving narrowed viewport as the live cause. Diagnostic hook removed; do not ship VELA_PARITY_CODE_LAYOUT_TRACE.
- Source DevTools read-only expressions through CUA (no safeguards disabled) measured .vela-code-pre font12.5px/18.75px, ui-monospace/SFMono-Regular/Menlo fallback, advance7.525634765625px. Explicit Menlo measurement matches. Native SF Mono7.7270513px was wrong for Markdown code. Added theme::markdown_code_font using Menlo and existing fallbacks; user/assistant code plus no-wrap intrinsic-width calculation use it. Editor font selection unchanged. Inline codespans already use FONT_MONO. DevTools closed after inspection.
- Real rebuilt native now matches reference wrapped line breaks, including emoji followed by END_OF_CODE on the same row. Native diagnostic build closed and clean rebuilt app launched. UI71 passes, workspace all-target Clippy denied warnings passes, build/package/diff-check pass (markdown-code-font-*). No new helper-only test claimed as a native font reproducer: real before/after screenshots and measured native/reference glyph widths establish this defect.
- Eligible full lossless transcript-wrapped-tabs-font-v2:900x700,bothinactive,sameICC5284199b,bothOScontrolindicatorsretained.195303exactchanged,32138above8(5.101270%),MAE3.3791425. Report162cases,zeroexactmatches; no accepted page. Explicit main IDs preserve full OS indicators after capture guard rejected the extra66x20 Window. These metrics cannot be compared as improvement percentages to other fixtures.
- No-wrap wheel input produced no movement in either app (both AX-target and viewport coordinate calls), so do not call it an app regression. Actual native gutter drag from640,329 to867,329 reaches END_OF_CODE; same reference drag at328 reaches the matching final text positions. Both remain no-wrap,far-right,answerfooterhovervisible. New paired transcript-tabbed-code-end-v1 captures underway. Next: finish manifests/report; re-enable wrapping and confirm reset; continue code typography/highlighting and the remaining feature/page matrix. No page accepted.

- Follow-up: transcript-tabbed-code-end-v1 pair is eligible, same inactive900x700/display/ICC,188301 exact changed,25543 above8,MAE2.6327389. CUA re-enabling wrap restores both apps to the start with matching wrapped sentinel line. Native language label also now uses the Markdown monospace font and lowercase transformation like Source .vela-code-lang. Label build/package/live check and Clippy denied warnings pass. Production UI71 passed for the preceding font change; label-only follow-up verified visually plus compile/Clippy. Eligible transcript-wrapped-tabs-font-v3 pair195304 exact changed,32144 above8,MAE3.3777734. Report164 cases after v3 (check regenerated current report);zero accepted pages.
- Probed AppKit UnifiedCompact toolbar style as possible traffic-light-size cause in temporary window_chrome module. Actual close-button frame was14x14 before AND after; live appearance did not improve. Removed module, initialization call and all probe hooks. No titlebar correction claimed. Initial probe compile failed inherent-vs-trait window_handle resolution; fully qualified HasWindowHandle fixed the probe. Probe session84077 remains running until clean final build/package replacement; do not leave it as the QA baseline. Native titlebar controls still differ, and exact color/baseline/page acceptance remains unfinished.

### Functionality-first priority; Appearance flow repaired

- Latest explicit user priority: implement desktop app functionality first, defer mobile logic, do full screenshot parity at the end. Updated completion-plan.md. Focused functional GUI checks remain appropriate; stop spending the implementation phase on broad visual capture matrices. No new mobile work. Shared-pointer input remains prohibited; CUA is restored and app-scoped input works. If CUA says app is not active, refresh with cua.getApp; do not fall back to CGEvent/helper input.
- Clean final Markdown build/package replaced the removed titlebar probe, current native later replaced again for Appearance below. Source DevTools read-only color query verified .vela-chat foreground#0d0d0d and user bubble color(srgb .0509804 .0509804 .0509804 / .05), code backgroundrgb243. Full captures sample agentwhite255 in both, bubble native241/source243, absolute code surface241 in both, left pane native252/252/252 vs source252/251/252. No color-token fudges made; alpha/color rendering discrepancy remains unresolved. Source DevTools closed.
- Native Appearance previously had fixed181px cards/two columns at900px while Source had three, fixed48px padding, incorrect row alignment, and no accessible content controls. New settings::ThemeGrid measures actual grid width and uses Source150px minimum tracks/12px gaps with retained column-count state. Resize regression verifies1/2/3/4 columns and resize-back. Viewport padding follows clamp20px/4vw/48px. Appearance now uses shared whole-page AutoScroll (header scrolls, actual gutter participates), left-aligned max760px content, correct paragraph18px leading, tracked heading, card16px caption row, percentage preview tracks. Interface/experimental rows match Source flex alignment.
- Theme cards now expose named RadioButton + toggled state; shell choices expose radio state; art exposes Switch + state; header/back control named. Replaced mouse-down-only activation with on_click for guarded Enter/Space activation. Live Daybreak/Midnight/Quiet Habitat selection, Tab, Space and Enter checks passed before adding retained focus handles. Focus values may be omitted by CUA until a specific control is focused; actual toggled updates appear on/off. Root focused element reporting often remains window0 while background keyboard input works; full accessibility still unaccepted.
- Appearance preview styling now follows Source: transparent unselected cards, shell-dependent neutral7% hover, selected/current hover border precedence, muted-opacity preview borders,3px accent bars with relative widths/opacities,32% terminal lane. Experimental switch34x19/15px thumb uses accent18% track and accent thumb when on; faint thumb/inset track off. Shared focus::outline now delegates to OutlineStyle with configurable offset; existing call semantics unchanged. Theme cards use2px outline/3px offset and no clipping outer card.
- Retained focus initially introduced a real regression: explicit handles were not tab stops, so Tab skipped theme cards. New production ThemeGrid.focus + shared FocusReveal regression failed before (appearance-retained-tab-stops-before.txt) and passes after registering tab_stop(true). Click also explicitly focuses the retained handle. FocusReveal::element generalizes the existing input wrapper without imposing an extra focus style. Actual CUA13 Tabs from Midnight reaches Observatory, scrolls it into view and shows blue outline; manual wheel scroll up works while final card retains focus. No scroll lock.
- Current gate73 UI tests pass, workspace all-target Clippy denied warnings passes, build/package passes (appearance-tab-registration-*). Native clean session57571 is running latest build on49575,Midnight,Codex,Appearance,scrolltop,Observatory keyboardfocus retained. Source21143 remains Midnight Appearance on same fixture. Reacquire CUA bindings if stale. No full Appearance screenshots saved since functionality-first correction. Appearance exact typography, remaining keyboard semantics/OS accessibility, responsive Classic and full page parity remain open.
- Next implementation: editor Find/search against shared document-owned text. Native editor.rs currently has selection/clipboard/IME/undo/reveal and no Find UI. TextBuffer.select(anchor,caret), replace(range,text), undo/redo own editing history. Full source/editor/search/splits/language tooling, terminal lifecycle/selection, explorer mutations, browser handoff, remaining desktop settings/runtime/session functionality remain active; mobile deferred.

## Desktop editor literal Find/Replace (functionality-first)

Implemented Unicode byte-safe literal search, case/whole-word options, next/previous wrap, match selection/reveal, replacement and atomic Replace All undo. Highlights cap at Monaco's 19,999 while navigation/replacement cover the full document. Source Replace behavior when the selection moves off a match is reproduced: first select the next result without editing.

Core tests passed; both rendered Find interaction and 6,001-line virtualized reveal regressions passed (`editor-find-ui-final.txt`). Workspace Clippy passed before the final small Replace fallback. Native QA app-scoped controls exercised Cmd+F, query input, Enter navigation, Replace All, dirty indication and single Cmd+Z restoration on a disposable file. No shared mouse/global input used. The live input tool omitted the emoji from injected replacement text, so that particular Unicode replacement is established by the rendered input regression rather than this live injection.

Still partial: regex/capture replacement, preserve-case, selection scope, workspace search and full keyboard/visual parity remain. Broad screenshot comparison and mobile work remain deferred per the user's priority.

## Desktop session filter

Source `SessionsPane.matchesQuery` matches lowercase session titles against a trimmed lowercase query; closing the header filter clears its query. Implemented the native header control and focused input, retained filtered session indices/list extent refreshed on query and engine session updates, stable row identity and accessible button labels. Actual native QA checks: unmatched query removes Provider fixture, `  PROVIDER  ` restores it, row remains operable, closing restores unfiltered results and reopening has an empty field. Added retained toggle focus on close and keyboard row activation; final keyboard live verification pending. No mobile work or broad screenshot audit.

Find/Replace workspace gate: 201 tests passed, one optional performance measurement ignored (`editor-find-workspace-tests.txt`). Replacement Enter/Command-Enter/Command-Option-Enter and Command-Shift-1 now dispatch replacement actions; rendered replacement Enter regression passed (`editor-find-keyboard-tests.txt`). Session-filter build and initial full-target workspace Clippy passed; retained-focus follow-up Clippy pending.

Session-filter follow-up: final retained-focus Clippy passed. Native QA close→Space reopened the filter and typed query reached it. Updated QA also verified Replace input Enter changed exactly one match, Command+Enter replaced the remaining matches, and two editor Undos restored the original text. Session sections: live Recents collapse removes the row and expansion restores it; Projects toggle updates; New Session opens the native folder chooser, cancelled without creating a session. The fixture has no saved projects, so project-row hide/show is not established by that empty-project check. UI suite rerun log: `session-sections-ui-tests.txt`.

Started persistent session pinning in `session_sidebar.rs` using the shared virtualized row renderer. Pinned and Recents are disjoint and both filtered; settings retain unavailable IDs. Compile passed, persistence/restart and native pin/unpin checks in progress. Pinned group uses a bounded, independently scrollable viewport during functionality implementation; its complete Source scroll/layout parity is still part of the deferred screenshot pass. Source local-storage pin import and project-history grouping remain unfinished.

Session pinning verification passed: native pin→Space unpin→Space repin preserved focus across groups; unmatched filter removed the pinned row; after actual Cmd+Q/relaunch, Unpin session remained available. Focus handles are retained by session ID. Settings backward compatibility and unavailable-ID preservation passed in `session-pinning-persistence-test.txt`; 75 UI tests and full-target workspace Clippy passed (`session-pinning-ui-tests.txt`, `session-pinning-clippy.txt`).

Discovered an existing `.take(6)` on saved-project rendering, removing bookmarks beyond six. Removed that truncation, provided retained scroll for all saved projects, accessible labels and keyboard activation. Build passed. Preparing a 12-project disposable profile for live last-entry reachability. The bounded Projects/Pinned viewports are functional implementation layout, not a visual parity acceptance.

Saved-project follow-up: live 12th entry reached by scrolling; after adding a 13th bookmark, 16 Tabs from the retained filter toggle scrolled/revealed and outlined the final project. Saved `project-12` through the native folder chooser, verified persistence and restart restoration, then saved the same directory again: count remained 13. Cancellation preserved that count, chooser reopened, and New Session worked after Save cancellation. Found queued duplicate dialogs during repeated activation and consolidated both folder flows behind shared pending-dialog ownership, reset on success/cancellation/error. Final pre-history build/package and Clippy passed. Workspace suite: 202 passed, one optional performance measurement ignored (`session-sidebar-workspace-tests.txt`).

Project-history controls started: session count, expansion/collapse, per-directory session rows with shared opening/pinning behavior. Separate focus identities avoid one session's project-history controls sharing the Pinned/Recents focus registration. Compile passed (`session-project-history-check.txt`); native history verification pending. Added a disposable Fixture history bookmark to the isolated QA profile for this check.

Project-history native verification passed: expanding Fixture history shows its session; pinning from that child updates Pinned and removes Recents, project pin control changes to Unpin, and Space unpins through the independent project focus. Collapsing hides the child; expanding an empty project shows No sessions yet. Source matching saved-workspace context menu implemented: right-click, Remove from Vela, Escape/outside dismissal, Return activation. Native QA removed Project 00's bookmark only; saved settings count 14→13, target directory still exists, active session intact. 75 UI tests passed after row refactoring (`session-project-history-menu-ui-tests.txt`); final context-menu Clippy passed (`session-project-menu-clippy.txt`). Source palette/layout/scroll-region acceptance remains deferred.

Session deletion implementation audit found a lifecycle defect before wiring the action: runtime SessionDeleted removes the view but leaves active_session pointing to it, allowing the same event to recreate an empty Background session. Started an integration regression covering active and warm deletion, stale navigation/history and inventory refresh. Reproducer result pending.

Session deletion lifecycle regression reproduced the active-session resurrection (`session-deletion-regression-before.txt`, failed), then passed after shared retirement (`session-deletion-regression-after.txt`). Retired session IDs reject stale opens, Loaded callbacks, inventory/permission refresh and later scoped events; active/warm/transcript/permissions/todos/usage ownership is removed, workspace observation detached and actor preview/OAuth close cleanup scheduled. Added SessionRemoved notification for native UI to clear conversation-owned state while retaining workspace Documents and dirty edits.

Engine Delete command now acknowledges request ID and session ID, keeps failed requests intact and retires state on successful HTTP acknowledgement even if the runtime omits its SSE event. Integration fixture deliberately rejects the first delete and holds a real replay response across the next successful deletion; test passed (`session-delete-ack-inflight-tests.txt`). Session context Pin/Delete actions and native irreversible-deletion confirmation compiled. A separately opted-in synthetic fixture on 49576 is now running for live cancellation/failure/retry/cleanup checks; it cannot forward requests or delete real sessions. Native live verification pending.

Session Delete native verification passed against separately opted-in synthetic port 49576: right-click session menu, Down/Return to confirmation; Cancel sent zero DELETE requests; first confirmed request deliberately returned503, preserving pinned session and transcript with visible error; retry succeeded, clearing transcript/session/pin and persisted last_session. Added an unsaved marker to the disposable editor before success: Cmd+Q still showed the global unsaved Save All/Don't Save/Cancel guard afterward. Cancel retained that guard on the next quit. Discarded only this test marker to close QA; original file on disk was unchanged. Fixture metadata records exactly rejected→deleted. No real sessions deleted.

Post-delete stale Send/Queue commands are now ignored before contacting runtime; extended real-loopback integration passed (`session-delete-stale-actions-tests.txt`). Full workspace deletion gate passed202 tests, one optional performance measurement ignored (`session-delete-workspace-tests.txt`); final Clippy/build packaging in progress. Native app closed after the unsaved-edit check. The deletion fixture remains synthetic/deleted; baseline comparison fixture49575 and reference QA remain unchanged.

Current desktop gaps still include literal-search regex/selection/workspace extensions, project relocation/context-menu keyboard opening and large-history performance, per-workspace close guards and simultaneous panel ownership, explorer mutations/drop, Git actions, terminal lifecycle/tabs, browser lifecycle/downloads and remaining runtime/settings integration. Mobile stays deferred. No page has received full screenshot parity acceptance; broad screenshot comparison remains the final phase.

Post-delete Save All audit found/fixed an additional ownership issue: removing document_sessions on retirement broke Save All and leaked the editor lease bookkeeping. Retain that workspace token until Documents releases the workspace; engine saves are already rooted in the independently retained editor observation lease. Built/packaged the correction. Native QA used a fresh synthetic workspace/session, edited 000-save-after-delete.txt, deleted the conversation, chose Save All on quit, then exited cleanly (engine stopped). On-disk contents exactly `before deletion\nsaved after deletion`. This establishes actual saving after deletion, beyond merely showing a dirty guard. Original comparison workspace/file and reference runtime unchanged.

Explorer creation implemented: root and directory New File/New Folder actions, selected default inline name, Enter/blur commit, Escape/empty cancellation, invalid-name feedback, retry after collision, captured workspace/session ownership and request acknowledgements. I/O runs on the serialized workspace worker; wrong scope and startup/submit failures acknowledge errors rather than leaving a pending UI. File creation uses exclusive create_new, folders create_dir, and both resolve/confine an existing parent. Successful files open pinned in the native editor; directories refresh/expand. Input is currently rendered below the root header; its exact Source tree-row placement/hover styling remains part of the deferred screenshot pass.

Checks passed: confinement/Unicode/collision tests (`explorer-create-confinement-tests.txt`), engine real-worker ack/scope/collision test (`explorer-create-engine-tests.txt`), native invalid path→Escape, duplicate keep.txt rejection→created.txt retry/open, new nested folder, Unicode nested file pasted and committed with Tab/focus loss. Disk verifies keep.txt unchanged, created files empty, cancelled untitled and escape paths absent. Full-target Clippy passed (`explorer-create-clippy.txt`); UI suite log `explorer-create-ui-tests.txt`. Current native QA runs the isolated explorer-create profile/runtime49577; source reference runtime49575 unchanged. No mobile changes or broad screenshot acceptance.

Rename foundation in progress (not exposed as a UI feature yet): recovery supports version-1 rename/source artifacts and exclusive held-source destination install, collision/interloper preservation, failed-source restoration, interrupted-source reconciliation and installed-rename completion using shared filesystem identity. Added Unicode file/directory, case-only file rename, collisions/invalid names, simultaneous external source/destination creation and two interruption tests. 14 recovery tests pass (`explorer-rename-recovery-tests-v2.txt`). Existing shared observation/save identity moved to files.rs rather than duplicating platform identity. Broader engine lib tests initially hit sandbox socket/notification restrictions (45 pass,8 denied); rerun unrestricted log `explorer-rename-engine-unit-tests-unrestricted.txt`. Need document/save/load reservations and prefix remapping before wiring a native Rename control; do not treat backend groundwork as feature completion. Directory rename currently follows Source's destination-check/rename behavior; full cross-process/platform race coverage remains unverified.

Rename UI/reservation work: Documents now reserves source/destination prefixes, waits for loads/saves, rejects open destination documents and remaps open/retained dirty buffers, tabs, active path, selection and undo without overwriting a destination. Seventeen document reservation tests passed before the receipt addition, including full/partial prior watcher remapping. Native context New File/New Folder/Rename controls and inline name form are wired to a captured scope, queued rename intents, request ACKs, deferred saves, editor/entity/tree remapping and missing-document reload. Global close waits for waiting/active renames; added a readiness regression.

Live rename exposed a false dirty conflict from treating our rename as an arbitrary external write. Fixed at workspace ownership: serialized rename captures tracked source identity/bytes, updates observer entries/snapshot before queued OS events run, emits tree change and sends verified RenameSnapshot receipts with the request ACK. Core only bypasses external-change conflict handling when both identity and saved baseline prove unchanged; unidentified/changed sources still conflict. Observer echo and core receipt regressions pass (`explorer-rename-receipt-observation-tests.txt`, `explorer-rename-receipt-document-tests.txt`); real engine file/folder scope/collision/receipt integration passed (`explorer-rename-receipt-integration-tests.txt`).

Retested native QA: dirty keep.txt→renamed.txt preserves edits without a conflict banner; save writes the new path; nested→folder-done carries the open Unicode child/dirty text; Undo restores its prior empty content, Redo restores the edit and Save writes folder-done/界🌍.txt. Disk confirms old file/folder paths absent and no duplicate save to an old path. Existing destination collision had already preserved source/editor and allowed retry. No broad screenshot acceptance.

Ordinary typing undo audit: real old QA typed hello world and one Undo left hello worl, reproducing per-character history. Implemented editor-only Monaco typing/space/deletion grouping with linear VecDeque deleted bytes, preserving atomic programmatic edits/paste/IME, selection boundaries and existing history bounds. Source Enter starts a new group and includes following text/auto-indent. Nine core grouping tests and rendered production typing/backspace test passed before adding Enter extension. Full workspace gate and latest build now in progress. Native QA closed cleanly after restoring its scratch file content.

Packaged typing-group native verification passed: typed hello world, Undo→hello, Undo→empty, Redo twice, two Backspaces and one Undo restored hello world, Enter+tail then Undo restored hello world with no trailing newline. Restored the empty scratch file and closed QA cleanly. Full workspace gate passed222 tests with one optional performance measurement ignored (`desktop-editor-rename-workspace-tests.txt`); final Clippy passed (`desktop-editor-rename-clippy.txt`), build/package passed. Latest packaged build includes rename receipts and editor grouping. QA app is closed; disposable explorer fixture49577 remains running. Native name form placement/hover styling and all broad screenshot parity remain deferred; mobile untouched.


Explorer internal moves implemented through the existing serialized rename/document-relocation pipeline. Requests capture an explicit destination parent; move preserves the source basename, rejects same-parent/self/descendant/root moves, recovery storage and collisions, and carries identified observer receipts for tracked descendants. Native rows provide GPUI internal drag payloads, folder/root/blank targets, compact multiselection, retained selected hidden children and cross-workspace rejection. Activation now occurs on click release so dragging an expanded folder does not collapse it. Source selection clearing and Ctrl/Cmd additive selection are retained. File-row and invalid folder targets consume the drag rather than falling through to the workspace root; a rendered production drop-surface regression exercises actual GPUI mouse down/move/up dispatch for valid, invalid, file, cross-workspace and blank-root drops.

Directory relocation now uses atomic no-replace rename on macOS/Linux; an injected destination-creation race proves the external empty directory and original child both survive. Windows uses its directory rename behavior, but Windows/Linux runtime checks remain unverified. File moves retain the existing source-hold/no-clobber recovery transaction. Creation now initializes recovery before claiming a name: the new startup regression restores an interrupted held source before rejecting a new file/folder at its original path. Automated selection, recovery, observer and real-worker move/ACK/scope checks pass. External import, Trash/delete, complete drag scrolling/a11y and visual acceptance remain open.

Live packaged move verification is pending: independent app control twice reported that the Mac is locked. An asynchronous unlock request is pending; no shared mouse/keyboard or alternate UI input was used. The unedited intermediate QA launch was stopped by exact PID before rebuilding, with no document edits in that launch. Final test/build/package results follow below. Mobile untouched; no screenshot page accepted.

Final explorer move gate passed230 tests, zero failures, one optional performance measurement ignored (`explorer-move-workspace-tests.txt`). Final Clippy (`explorer-move-clippy.txt`) and build (`explorer-move-build.txt`) passed; QA comparison package signed successfully. Focused production drop-surface/selection tests also passed (`explorer-move-selection-tests.txt`). Latest package includes internal moves, exclusive directory rename and startup creation recovery. Native GUI validation remains pending the requested Mac unlock; no live move is marked accepted.

Live explorer move verification passed after unlock. CUA coordinates use Retina screenshot pixels (1800×1400), not logical 900×700 points. Lossless one-shot capture `native/explorer-move-dirty-before-v1.png` established target coordinates because the CUA screenshot had a white Metal capture region. Dirty renamed.txt→folder-done retained text with no false conflict; Undo twice restored baseline, Redo twice and Save wrote only the new path. Root-header move back passed. folder-done→move-target carried the dirty Unicode child, retained Undo/Redo, and saved under the moved folder. Cmd multi-selection and Shift range selection moved both files in each direction. File destination and descendant drops left paths unchanged; occupied root destination showed destination-already-exists and preserved both byte sequences. Blank-area root drop passed. All original fixture paths/bytes restored; extra test obstacle and empty destination retained outside the workspace at `/private/tmp/vela-explorer-move-verified-artifacts`. QA exited cleanly. No screenshot parity page accepted.


Explorer More actions buttons now exist for each row and the workspace root, with retained per-workspace/per-path focus. Native QA reproduced an initial Escape focus bug (Space went into composer); explicit trigger focus fixed it. Rechecked Escape→Space reopening for both file/root, Down/Down/Return→Rename, and Escape cancellation. Root menu correctly omits Rename. Source exact menu geometry/complete actions remain unfinished.

Recoverable macOS Trash now uses NSFileManager on the serialized workspace worker, with no permanent-delete fallback. Requests carry captured workspace/session and a batch ID; invalid/missing/wrong-scope failures acknowledge. Document prefixes reserve against reads, saves and overlapping relocation, wait for in-flight I/O, retain all dirty buffers/undo on failure, and remove successful prefixes' tabs/buffers only once the batch finishes. Batched cleanup uses Source's original tab-order neighbor rule; failed prefixes survive. Deferred saves for removed prefixes are discarded so they cannot recreate the trashed entry. Observer marks tracked descendants deleted before queued notifications, retains Git baselines, and suppresses owned deletion echoes. Context Delete/Delete N Items, compact selection, and explorer-owned Backspace/Delete are implemented; input fields and action buttons do not inherit the destructive shortcut. Escape clears tree selection. Non-macOS Trash explicitly returns unsupported; Windows/Linux integration remains open.

Trash checks passed235 tests, one optional performance measurement ignored (`explorer-trash-workspace-tests.txt`), final Clippy and build passed (`explorer-trash-clippy.txt`, `explorer-trash-build.txt`). Native QA: explicit Delete of a dirty disposable file closed its tab; Delete of a folder with a dirty open child closed that child; Cmd-select two files→Backspace removed the selection. Read all four saved byte sequences from actual `/Users/tdawg/.Trash` and compared them to the independent `/private/tmp/vela-trash-qa-backup-20261030` copies; exact matches. Unsaved QA-only markers were intentionally discarded by explicit Delete, matching Source. All original fixture paths/bytes unchanged and QA quit cleanly. No real user files removed; Trash/backup test artifacts retained. Native log exposed a missing accessibility role on the focused explorer container; added Group/Workspace files semantics. Final accessibility-only build/check follow below. Broad screenshot acceptance and mobile remain deferred.


Explorer Reveal in File Manager implemented for file/folder/root menus. Resolution runs on the captured session/workspace's serialized I/O worker, confines paths, and emits a request-ID acknowledgement on success, stale scope, missing path, traversal or worker submission failure. UI handles only its owned requests and calls the native platform reveal API. Engine integration tests passed (`explorer-reveal-engine-tests.txt`); Clippy/build/package passed (`explorer-reveal-clippy.txt`, `explorer-reveal-build.txt`). Native keyboard menu Down/Down/Return opened Finder at the isolated workspace with renamed.txt selected and the correct path breadcrumb. Closed only that QA-created Finder window, leaving the existing screenshots window intact; QA quit cleanly. Explorer focus now announces Workspace files instead of an unlabeled focused node. Remaining explorer work includes external import/drop, terminal-at-folder handoff, detach/Remove from Workspace, remaining root/panel actions, complete keyboard/drag scrolling/a11y combinations, non-macOS Trash and full visual parity. Latest full workspace gate remains235 tests/one optional performance ignore, plus final focused native UI/engine checks. No Source writes, mobile work or accepted screenshot parity pages.


Terminal paste now routes native Edit/Paste and Cmd+V/Shift+Insert into the focused PTY, with Source xterm CR/LF normalization and bracketed-paste framing based on the live emulator mode. Input returns scrollback to the bottom. Plain Page Up/Down now reach terminal programs; Shift+Page Up/Down retains local scrollback. Added two protocol tests plus existing real PTY test (`terminal-paste-tests.txt`). Native clipboard command executed shell arithmetic and wrote exactly `42界🌍` to `/private/tmp/vela-terminal-paste-result-20261030.txt`. Disposable raw receiver confirmed plain Page Up bytes ESC[5~; another receiver sent Shift+Page Up then Page Down and recorded only ESC[6~, proving the scroll key did not leak into the PTY. QA quit cleanly before the next keyboard build.

Latest full workspace gate passed237 tests, one optional performance ignore (`desktop-explorer-terminal-workspace-tests.txt`). Further terminal audit reproduced a protocol defect in code: Ctrl+Left/Up/Enter were incorrectly encoded from the first letter of their key names. Replaced that logic with explicit named-key/function/navigation/C0 protocols in terminal_input.rs; removed the incorrect Cmd+Home scroll-to-bottom override. Added a read-only reference oracle script and generated 832 key/modifier/application-mode cases directly from Source's installed xterm Keyboard.ts. All cases and focused regressions pass (`terminal-keyboard-tests.txt`). Latest Clippy passes; final build/package/native Ctrl+Left check pending below. No full terminal parity claim: tabs/workspace lifecycle, sizing, text selection/copy/IME, TUI ownership, folder handoff, remaining output lifecycle and visual acceptance remain active.


Terminal named-key native verification passed: raw PTY receiver recorded Ctrl+Left exactly ESC[1;5D (`/private/tmp/vela-terminal-ctrl-left-result-20261031.bin`), not Ctrl+L. The 832 Source-generated modifier/application-cursor cases remain passing.

Terminal tabs/workspace ownership implemented in terminal_state.rs: one scoped, ordered set of independently retained emulator/focus views, UUID process IDs, data/exit lookup by ID, Source neighbor selection, individual tab stop, working New terminal, and separate tray hide. Workspace changes retire the previous set and late retired IDs cannot mutate new views. Hiding/reopening retains shells; last explicit tab close hides the tray, reopening shows the empty state and + starts a fresh shell. Static Problems/Sass controls are keyboard buttons. Open in Integrated Terminal appears first in explorer menus and resolves the selected folder/file's parent off-thread with directory validation and captured workspace ownership before creating a named tab. Stale scope/file/traversal cases are covered by engine integration.

Full terminal tab gate passed241 tests/one optional performance ignore (`terminal-tabs-workspace-tests.txt`) before the last header focus regression; final Clippy/build/package passed. Focused terminal tests now8 pass (`terminal-tabs-ui-tests.txt`), including actual 12-tab keyboard focus reveal and manual-scroll noninterference. Native first/second shell PIDs were15381/15443 with independent environment state; inactive first output retained and visible in `native/terminal-background-tab-v1.png`. Rail hide/reopen retained first PID and VELA_QA_TAB=first. Folder context opened actual cwd `/private/tmp/vela-explorer-create-workspace/folder-done`. Closing the first tab stopped15381 while15443 survived and the folder tab remained selected. These are disposable shells.

The live narrow dock exposed clipped header controls. Added a retained horizontal tab viewport, active/focused-item reveal and fixed visible add/preview/close controls. Native screenshot `native/terminal-scroll-header-v1.png` confirms visible controls with Terminal2 selected. Visible close-panel button hide/reopen retained PID17685 and VELA_QA_HIDE=retained; active-tab close selected its Source neighbor; last close hid the tray; reopen showed an empty tray and + created Terminal1. QA exited cleanly. Focus screenshots are functional evidence only, not paired visual acceptance. Next: terminal sizing/PTY resize; remaining text selection/copy/IME, TUI ownership and complete workspace/keyboard/output lifecycle acceptance stay open. Mobile deferred.


Responsive terminal fit implemented from measured Source terminal font (12px/15px lines) and viewport geometry, using Source padding/ruler/spare-column rules and Source IPC dimension bounds. Scheduled resize verifies active terminal ID, workspace, visible tray and active panel before updating emulator/PTY; hidden/unmeasurable viewports do not resize. PTY minimum now2 columns/1 row, matching the emulator. Real kernel resize regression passed37×9 and2×1. Central engine terminal input now rejects over1MiB before delivery, matching Source's UTF-8 byte cap; integration proves rejection keeps the shell usable.

Latest complete gate:245 tests passed, zero failures, one optional performance test ignored (`terminal-sizing-workspace-tests.txt`); Clippy/build/package passed (`terminal-sizing-clippy.txt`, `terminal-sizing-build.txt`). Native GUI shell reported `5 34:21189`, sidebar hidden→`5 65:21189`, restored→`5 34:21189`; same process across actual layout changes. Evidence files `/private/tmp/vela-terminal-fit-{initial,wide,restored}-20261031.txt`. Sidebar restored, QA exited cleanly. Terminal sizing is functionally verified; exact Source renderer metrics/Unicode cell painting, resizable/snap tray geometry, selection/copy/IME, TUI and remaining lifecycle/visual states remain open. No screenshot page accepted; mobile deferred.


Terminal tray resizing/snap implemented with window-scoped GPUI mouse observers and Source thresholds (30px snapped, 32px threshold, 520px maximum, retained last full height). Drag below the window hides the tray without stopping its PTY; expansion and rail reopening retain shell ownership. Browser placement follows actual tray height. Full gate passed247 tests, zero failures, one optional performance ignore (`terminal-tray-workspace-tests.txt`); Clippy/build/package passed. Native default→tall→snap→expand and below-window hide/reopen exercised; default/tall/expanded shell remained PID25366. Focus captures `native/terminal-default-tray-v1.png` and `native/terminal-expanded-tray-v1.png` are functional evidence, not visual acceptance.

One reopen shell command unexpectedly contained a previous command prefix; own QA last-two-command history captured the joined command. Kept this incident open. Diagnostic dispatch logs show paste origin/length and navigation modifier flags. A raw PTY receiver through actual snap→below-window hide→reopen recorded exactly `SECOND_PASTE` plus CR, with no gesture-injected keys. Controlled FIRST→resize→SECOND commands subsequently produced exact separate FIRST/SECOND files with one paste and plain Enter each. These checks do not establish the cause of the original intermittent restoration; no speculative key-protocol or timed paste-dedup change made. Evidence: `terminal-dispatch-native-log.txt`, `/private/tmp/vela-terminal-gesture-input-20261031.bin`, `/private/tmp/vela-terminal-dispatch-{first,second}-20261031.txt`.

Terminal output projection previously discarded Alacritty zero-width code points. Cell snapshots now retain ordered combining marks and the styled row includes their UTF-8 bytes in the owning ANSI run. Regressions exercise split-byte UTF-8, multiple accents, wide-cell attachment, cursor columns, resize and run byte boundaries (`terminal-combining-tests.txt`, two passing tests). Exact glyph placement/live packaged check and final gate follow. Mobile untouched; no paired screenshot page accepted.

Unicode output gate passed249 tests, zero failures, one optional performance ignore (`terminal-unicode-workspace-tests.txt`). Added standard DEC cursor visibility support afterward; final terminal library gate11/11 (`terminal-unicode-final-tests.txt`), final Clippy and build pass (`terminal-unicode-clippy.txt`, `terminal-unicode-build.txt`). Packaged native QA rendered red e with acute/cedilla, wide character plus acute and cafe with acute, while DEC hide-cursor suppressed the overlay: lossless `native/terminal-combining-hidden-cursor-v1.png`. Explicitly restored cursor visibility afterward. Whole-line shaping still does not establish exact fixed-column fallback glyph placement; selection/copy/IME and further terminal functionality remain open. No paired visual page accepted.


Terminal selection/Copy implemented using Alacritty-owned grid selection (simple, semantic word, whole line, block, Shift extension), including scrollback coordinates, wide spacers and combining marks. View-scoped geometry maps pointer halves to cell sides; GPUI window capture owns drag/release and never synthesizes shared input. Styled runs highlight selected cells; typing/paste clears selection. Cmd+C/native Edit Copy read only the active terminal selection. Wheel scrollback accumulates fractional pixels, consumes terminal scrolling and updates an active selection endpoint; matched Source's5,000-line history limit. Auto-scroll outside the viewport, complete mouse-reporting/TUI behavior, selection colors/fixed-column glyph geometry and IME remain open.

Full gate253 passed, zero failures, one optional performance ignore (`terminal-selection-workspace-tests.txt`); wheel hookup added afterward, final Clippy/build/package pass (`terminal-selection-final-clippy.txt`, `terminal-selection-build.txt`). Core regressions cover wrapped/combining/wide copying, spacer-only selection, word/line/reverse/block/Shift extension and scrollback; rendered-row regression protects selection background/run bytes. Native QA: double-click→Copy→composer paste exactly COPY_ALPHA; actual drag→Copy preserved `COPY_ALPHA é界 OMEGA`; triple-click preserved the full line plus newline. Native wheel scrolled sixty output lines back to SCROLL_036..043 (`native/terminal-selection-scrollback-v1.png`); Alt-click/Shift-extend block→Copy pasted exactly036\n037\n038. Cleared synthetic composer text and quit QA cleanly. Focused screenshots are functional evidence only; no paired page accepted, mobile deferred.

Terminal OS text input/composition implemented per terminal entity, with independently retained preedit buffers and subscriptions owned by the tab. Platform callbacks use UTF-16 ranges, shaped candidate bounds and pointer indices; preedit paints underlined at the shell cursor and never reaches the PTY until committed. The tab's captured UUID routes committed text; retired/missing/not-running views ignore it. Printable/dead keys use GPUI's OS input handler, named/control keys retain the reference-tested protocol path. Paste/control input cancels local preedit. Typing and composition reveal the bottom of scrollback.

Composition ownership regression verifies repeated preedit emits nothing, surrogate-pair replacement, empty cancellation, one commit (unmark afterward does not duplicate), and two independent terminal IDs. Terminal-focused suite12 passed including all832 reference key cases (`terminal-composition-terminal-tests.txt`). Clippy/build/package passed (`terminal-composition-clippy.txt`, `terminal-composition-build.txt`); latest full gate255 passed, zero failures, one optional performance ignore (`terminal-composition-workspace-tests.txt`). Native ASCII typing/paste executed; CUA typeText omitted non-ASCII in its synthetic text route, recorded `typed-`, so that path is not accepted as Unicode typing. Native Option+E then E correctly wrote exact UTF-8 café in `/private/tmp/vela-terminal-native-deadkey-20261031.txt`. Captured unfinished preedit (`native/terminal-native-preedit-v1.png`) then committed it; `/private/tmp/vela-terminal-native-preedit-20261031.txt` also exactly café. Full Japanese/Chinese candidate UI, long/multiline preedit placement, focus/tab/retirement/native cancellation combinations and visual parity remain open. No paired page accepted; mobile deferred.

Terminal final-output/exit relay fixed: separate PTY reader/wait threads can deliver EOF and exit in either order. Relay now forwards final queued output before exactly one natural-exit event, waits for the wait thread if output closes first, and bounds post-exit drain to500ms for a descendant-held slave. Deadline drain snapshots queue length so continuous descendants cannot make cleanup unbounded. Cancellation terminates the relay. Duplicate IDs reject without replacing the original PTY; forwarding/removal/exit publication checks the registered Arc identity under the registry lock, so stopped/retired readers cannot mutate or remove a same-ID replacement.

Two deterministic negative regressions fail with the former immediate-break relay (`terminal-relay-negative-tests.txt`) and pass after byte-for-byte restoration of the fix. A third covers an inherited open slave. Real engine integration verifies duplicate rejection, immediate same-ID stop/restart retaining REUSED:72 and exit29, and an entire180,000-byte generated burst plus computed FINAL:42 before exit23. Full workspace gate258 passed, zero failures, one optional performance ignore (`terminal-relay-workspace-tests.txt`); final Clippy/build/package passed (`terminal-relay-final-clippy.txt`, `terminal-relay-build.txt`). Native second-tab exit23 selected first; first PID86362 retained exactly in both `/private/tmp/vela-terminal-relay-{first,retained}-pid-20261031.txt`. Last natural exit29 left the Source-matching empty open tray. QA quit cleanly.

Editing recovery: an initial script accidentally matched the public terminal API instead of the private helper. Immediately restored the complete2,529-line engine source from Codex's working-tree snapshot (SHA256 fc56743edfadebdd3741471303d0fc40d11c9b1b272ed21a0ed83efe36a557d6), with surviving prefix/suffix verified exact. Recovery check passed (`terminal-engine-recovery-check.txt`); reapplied only the newline-anchored private helper. Before/after comparisons prove all text outside that helper is unchanged from the recovered working version. The final258-test gate above covers the restored engine and fix. No requested feature or unrelated migration edit was intentionally removed; no Source writes, mobile work or accepted paired screenshot pages.

Terminal selection outside-viewport auto-scroll implemented from Source xterm SelectionService's50px threshold,15-line maximum, JavaScript rounding and50ms interval. A per-drag generation and captured tab UUID prevent old timers affecting a new selection. Window-local pointer coordinates drive scrolling; release/missing button, hidden/snapped tray, inactive panel/tab/window, and retired tab terminate ownership. Normal selections extend to viewport edges; block selections keep columns. A real-grid regression traverses100 lines into history, stops inside the viewport, returns to the bottom, and preserves rectangular columns. Distance rounding/clamps and grid checks both pass (`terminal-autoscroll-final-tests.txt`); broader terminal-focused suite13 passed before the grid test (`terminal-autoscroll-tests.txt`). Final Clippy/build/package passed (`terminal-autoscroll-clippy.txt`, `terminal-autoscroll-build.txt`). Latest complete workspace gate remains258 plus these two new focused regressions.

Native QA generated AUTO_000..099, initially visible094..099 plus prompt (`native/terminal-autoscroll-before-v1.png`). Fast drag above the viewport selected visible history; a1000ms app-scoped held click in the upper viewport padding auto-scrolled and copied AUTO_056..093 plus the partial original094 anchor. After release and subsequent interaction, double-click/Copy still returned exactly AUTO_056, proving scrolling stopped. Cleared the synthetic composer and quit cleanly. No shared input, capture worker, Source edits, mobile logic or paired screenshot acceptance introduced. Remaining terminal work includes full fixed-cell glyph/cursor rendering, mouse reporting/alternate-screen wheel, complete TUI, advanced IME/lifecycle/a11y combinations and paired parity.

Explorer external import/drop implemented for current workspace folder/root/blank targets. Each selected source copies privately under the recovery coordinator, preserves originals and atomically claims the destination using exclusive directory rename or file hard-link. Destination collisions (including targets created during copying) survive. File/folder/binary/Unicode inputs, per-source10,000-file/2GiB budgets, nested symlink/dependency skips, metadata/traversal/current-workspace/ancestor-source guards and per-source outcomes are implemented. Staging follows the destination device on Unix mounts through the existing hidden recovery store; actual mounted-volume/Windows/Linux checks remain unverified. Ordinary failure cleans only the private staging directory; crash-orphan housekeeping remains unverified.

Captured session/workspace requests use the serialized observer worker and acknowledge stale owner, validation and submission failures. Imported text becomes its own known baseline (Source recordImported behavior); owned watcher echoes are suppressed while later external edits still publish. Existing destination documents cannot be overwritten by an import. UI owns request IDs, refreshes the captured parent/current tree, shows Source-style success/errors and silently skips ordinary collisions/already-in-workspace outcomes. Current folder/root/blank GPUI drop surfaces accept ExternalPaths and route the source list once. Close waits for import acknowledgements; shared pending Explorer I/O checks now also cover creation, rename and Trash requests/waiting queues.

Core import3 tests pass (`explorer-import-core-tests.txt`, file also includes an existing migration test). Observer import baseline/echo regression passes (`explorer-import-observer-tests.txt`). Actual rendered mouse down/move/up ExternalPaths routing reaches folder or blank root once (`explorer-import-drop-tests.txt`). Real engine integration tests wrong-scope acknowledgement and successful copied bytes/source preservation. Latest full gate265 passed, zero failures, one optional performance ignore (`explorer-import-workspace-tests.txt`), including the two new terminal auto-scroll checks. Final Clippy/build/package passed (`explorer-import-final-clippy.txt`, `explorer-import-build.txt`). Physical Finder-to-native external drag, no-session/new-workspace drop, full cross-root/file-row/native a11y/drag combinations and paired visual acceptance remain unfinished. Do not count the rendered drop test as physical OS drag acceptance. Original QA fixture files preserved; app currently closed. Mobile deferred; no paired page accepted.

Explorer Remove from Workspace now performs Source's physical detach into `~/.vela-detached/<UUID>-<basename>`, after its explicit confirmation; Cancel retains the menu and focus. It is distinct from Trash and does not discard editor tabs or undo. Serialized scoped requests reserve document prefixes against pending reads/saves, acknowledge all validation/scope/submission errors, release unchanged on failure and mark retained paths deleted/conflicted only on success. Dirty text and clean editor content remain; pending saves cannot silently recreate the detached prefix. Observer forgets detached baselines and suppresses owned watcher echoes. Quit waits for queued/in-flight detach. File/folder store, collision/failure, document-prefix retention/undo/neighbor/workspace and observer regressions pass; wrong-scope/missing/root engine requests acknowledge without moving data. Shared dirty external-conflict state now updates its deleted flag. Windows exclusive move primitive uses no-replace MoveFileW; Windows/Linux runtime checks remain unverified.

Native QA used only two newly created fixtures with independent `/private/tmp/vela-detach-qa-backup-20261031` copies. File Cancel preserved bytes/dirty text/menu; Return reopened the same confirmation; OK removed the tree row while retaining its tab/unsaved text. Undo/Redo survived. Folder detach retained the dirty Unicode child and Undo/Redo. Actual retained artifacts were byte-compared: `/Users/tdawg/.vela-detached/7bc04f51-7758-4276-b85f-46b0a74a18ee-detach-qa-file-20261031.txt` and `/Users/tdawg/.vela-detached/7e0130d9-3f68-4b73-8bb0-27e2dd3e6727-detach-qa-folder-20261031/界🌍.txt`. Both exact original bytes, unsaved QA markers intentionally stayed only in the editor. Undid markers to clean buffers before quitting. All original explorer fixture paths/bytes unchanged.

Live detach exposed a shared conflict-layout defect: native message flex minimum0 shrank to a glyph-wide column at296px, consuming the editor height. conflict_banner.rs now uses the Source CSS minimum-content word width. A production layout regression at296/448/1200px passes; removing that minimum reproduces zero editor height (`explorer-detach-conflict-layout-negative.txt`), then the fix was restored byte-for-byte and rechecked (`explorer-detach-conflict-layout-tests.txt`). Native dirty child editor is now clickable; Undo/Redo verified at296px. Banner/control exact Source narrow geometry and all conflict-state screenshot acceptance remain unfinished.

Initial post-sheet capture grouped an auxiliary window, yielding invalid aspect ratio (`native/explorer-detach-dirty-file-v1.png`, no accepted manifest). Preserved it as rejected diagnostic evidence. capture-parity.sh now supports explicit `--single-window` using screencapture's documented -a flag; manifests record attached_windows_included and the exact command. No resizing/cropping/masking. Rechecked lossless900×700 capture+manifest `native/explorer-detach-dirty-child-fixed-v1.png`. Native modal sheets require their own captures; a single-window main capture must not stand in for an omitted sheet. This is functional evidence, not paired page acceptance.

Final complete workspace gate270 passed, zero failures, one optional performance ignore (`explorer-detach-final-workspace-tests.txt`); Clippy/build/package passed (`explorer-detach-layout-clippy.txt`, `explorer-detach-layout-build.txt`). Prior detach gate269 passed before the new layout regression. App currently closed; retained QA artifacts/backups preserved. Mobile remains deferred; no paired screenshot page accepted.


Terminal context ownership now includes session ID as well as physical directory. Same-directory conversation switches retire the previous PTYs, preserve the selected sidebar, and start the next context shell while its tray is hidden. Directory acknowledgements also carry a generation, preventing A-to-B-to-A completions from targeting a replacement terminal. Native hidden/open tray shared PID1206; same-root switch and return produced1337 then1369, with prior shells confirmed stopped. Four focused model tests passed, full gate271 passed, Clippy/build/package passed (`terminal-context-*`). Hidden Source terminal exact initial geometry and full terminal rendering remain unaccepted.

Root Explorer Remove Workspace now closes the context separately from physical detach or runtime deletion. It captures dirty versions, refuses conflicts, waits for scoped Explorer/document I/O and exact save acknowledgements, and leaves the workspace open on changed versions, failures or timeout. Matching active UI/preview/terminal ownership is cleared; history/bookmarks survive. Reopening a context supersedes its pending older close. Sass cleanup now records its owning session, does not let another context disable it, and gates canceled workers' publication/output writes. Returning to the retained owner restores its checkbox. This is scoped cleanup of the existing single watcher, not full per-root Sass parity.

Native QA appended `workspace-close autosave marker 20261031` to the dedicated fixture without saving; Remove Workspace saved exact bytes and cleared its context. Sessions still contained both fixture conversations, and reopening retained history. A dirty edit plus external disk change blocked close with the Source conflict message, retained unsaved content, and preserved Undo/Redo. Reloaded the disk version of this disposable test file, closed normally, and confirmed owned shell53011 stopped. Reopening started fresh shell53390. QA then quit. Lossless900x700 functional captures: `native/workspace-close-conflict-v1.png` and `native/workspace-closed-v1.png` with manifests. Narrow conflict banner controls still overflow and are not visually accepted. Retained document/tab activation identity differs from Source and remains an explicit follow-up.

Complete workspace gate274 passed, zero failures, one optional performance ignore (`workspace-close-final-workspace-tests.txt`). Additional close/Sass-owner integration passed2 tests (`workspace-close-sass-owner-tests.txt`): initial added test attempted to enable an evicted context, then corrected to open that context explicitly before testing close cleanup. Final ownership guards and packaging are being rechecked. No paired visual page accepted. Mobile deferred.

Workspace-close final reopened-context and canceled-write guards rechecked:274 passed, zero failures, one optional performance ignore; final Clippy/build/package passed. Owned terminal53390 stopped when QA quit. Shared-root Sass ownership implementation is the next active slice; its first three state regressions and real two-root compiler/owner integration pass.


Sass singleton replaced with physical-root records and independent context owners (`sass_state.rs`). Same-root contexts share one generation-scoped worker and100-line log; distinct roots compile simultaneously. Disable affects the shared root, explicit close only unregisters its owner, and the final owner cancels the worker. Old generations cannot publish or write outputs. Cached watching state/logs restore on context navigation. Three state regressions pass; real-engine integration compiles two roots, closes one of two same-root owners, recompiles both after edits, disables only one root, closes/reopens with the persisted preference and confirms cleanup.

Native shared-root QA: fixture A enabled compilation of the new `sass-owner-qa-20261031.scss`; fixture B showed the retained watching state/logs (`native/sass-shared-second-owner-v1.png/json`, lossless900x700). Closed A; changing SCSS to#654321 still produced matching CSS for B. Closed B; next SCSS#abcdef left CSS at#654321 until a subsequent explicit launch enabled it. Native app quit cleanly. Full ownership-only gate277 passed (`sass-shared-workspace-tests.txt`), Clippy/build/package passed. No visual page accepted; narrow Sass panel clipping remains open.

Per-canonical-root Sass preferences now persist separately in native `sass-watch.json`, sorted version1 enabled roots, atomic replacement and last-valid backup. Corrupt primary recovers from backup without copying corrupt bytes over it; invalid/newer/unwritable settings fail without silently overwriting data. Three preference regressions pass; six combined Sass model/preference tests passed (`sass-preferences-tests.txt`). Actual QA quit/restart: enabled fixture generated#abcdef and stored its canonical root; after quitting changed SCSS to#bada55 and launched ordinary `files` action (no enabling automation). CSS automatically became#bada55, proving durable restoration. Mac locked before the next UI inspection; no further screenshot claimed. Native QA remains running with the isolated fake runtime, and an unlock request is pending. Shared input remains untouched.

Delayed-history regression exposed a preexisting engine deadlock: an in-flight response after explicit close reached ensure_view, whose warm-entry if-let retained a store write lock while reacquiring it. Preserved stack sample (`workspace-close-late-history-negative-sample.txt`) shows ensure_view parked on that write lock and the test waiting on read; stopped only positively identified owned testPID57677. Scoped the warm removal before re-locking and tagged history loads with UUID generations. Close/eviction invalidate their load, and only the current load can hydrate. The blocked-response close regression now passes without reopening context or restarting persisted Sass (`workspace-close-late-history-tests.txt`,2pass).

Latest full gate280 passed, zero failures, one optional performance ignore (`sass-preferences-workspace-tests.txt`), with final Clippy/build/package pass (`sass-preferences-{clippy,build,package}.txt`). Sass remains partial: bundled Dart Sass equivalence, mappings/config/source maps, dependency-triggered rebuilds, safe atomic output replacement, full UI controls/log layout and legacy preference import still unfinished. No paired page visually accepted. Mobile deferred.


Sass dependency/output slice implemented while Mac locked. Inventory includes partials (never outputs), tracks additions/edits/deletions, uses Source ignore names including all recovery directories, accepts project hidden sources and case-insensitive Sass extensions, and uses an iterative walk without former silent depth12/file2000 cutoffs. Entry compilation uses source paths, preserving nested relative imports, and includes root/node_modules load paths. Any inventory change rebuilds entrypoints. Deleted dependencies produce logged compiler errors without replacing previous CSS; restoring them recovers. Real integration checks nested partial-only color changes, missing-partial failure with retained CSS, no partial CSS output, and recovery (`sass-dependency-engine-tests.txt`,2pass).

CSS writes now stage a complete unique file and atomically rename after validating the unchanged canonical root, actual parent directories and regular CSS target; symlink output/parent/root and traversal/source-extension targets reject without touching external bytes. Temp files are cleaned on completion/failure. Two output and three inventory tests passed; all11 Sass ownership/preference/inventory/output model tests passed (`sass-dependency-output-tests.txt`). Scan/compile/output passes run in Tokio's blocking pool, with async polling/cancellation; retired workers remain gated from writes/publication. Latest complete gate285 passed, zero failures, one optional performance ignore (`sass-dependency-workspace-tests.txt`); Clippy/build pass. QA app remains the older packaged preference build because Mac locked; dependency/output native inspection and repackaging are pending unlock. Do not call these GUI accepted. Full Dart Sass/config/mappings/maps, logs/UI and legacy preferences remain unfinished. No paired visual page accepted; mobile deferred.


Pinned Dart Sass migration replaces embedded grass. Native Rust still owns watching/root owners, preferences, cancellation and file installation; bundled Node runs only the compiler/output planner and never package scripts. Added JSON5 exactly2.2.3 (same as Source), retained Sass1.102.0/Vite6.4.3; npm lock root metadata repaired by npm. `tools/preview/sass-plan.mjs` is the type-erased readable ESM port of Source sass-config.ts with recovery constants inlined; no runtime dependency on reference checkout. Helper covers explicit/legacy config, JSON5, compressed/expanded styles, workspace load paths, relative source maps, Live Sass formats/globs/output conventions, literal package mappings, diagnostics/warnings and partial compilation failures. Rust child cancellation kills its owned Node process (tested), validates/creates real output parent directories and atomically installs maps/CSS. Configuration edits now trigger inventory passes. Package resources/digest include both helper modules and pinned package dependencies.

Eight Node utility regressions pass (`sass-dart-tools-tests.txt`),15 Rust Sass core tests pass (`sass-dart-core-tests.txt`), and read-only execution of the actual Source planning/compile utilities against9 disposable cases matched9 plans and9 compiled/error outputs, CSS bytes, map bytes and warnings (`sass-dart-reference-comparison.txt`; reproducible `scripts/compare-sass-reference.mjs`). Previous complete gate289 passed before legacy import, then291 passed after (`sass-dart-final-workspace-tests.txt`). Clippy/build passed. These are compiler/service checks, not native screenshot acceptance.

Read-only legacy Sass import now runs at ordinary app startup using existing profile override/skip rules. It imports primary or valid backup only if native primary/backup do not exist, preserves source bytes, retains explicit native disabled choices, rejects corrupt source without creating a native file and atomically claims a staged destination without overwriting another instance's concurrent native save. QA's VELA_SKIP_LEGACY_IMPORT=1 bypasses it. Two import regressions added; no real legacy profile was read by the tests.

Sass status now travels as one immutable scoped SassWatchState (canonical directory/enabled/status/logs/error) instead of independent UI booleans/logs/editor-error writes. Starting/On/Error/Off and raw error recovery are owned by root state; failed compilation no longer appears healthy or leaks into editor file errors. Batch completion publishes status/logs together; a successful recovery clears status.error while preserving old error logs. Stop/start retains the root's log history. Limits match Source last100 logs and UTF16 slice8192, including a split-surrogate replacement glyph; old workers cannot clear a replacement's state. Status/model4 tests and real partial deletion/recovery assertions pass; full gate293 passed before extracting the rendered log component.

Editor status trigger and Sass item now expose Source accessibility labels and focus handles with Enter/Space and Escape return, and log role/labels allow independent native inspection. Live checks pending unlock. New `ui/sass_output.rs` preserves preformatted whitespace and Source12px/1.6 log leading,14px horizontal padding and8px row spacing; shared AutoScroll provides a real draggable8px gutter. Actual production renderer regression at296/448/1200px verifies all100 wrapped/multiline logs can reach the final row and clearing contracts/clamps the scroll area (`sass-output-layout-tests.txt`). No claim of native interaction or exact visuals for the new build. Final rendered gate/build/Clippy currently running. QA remains the prior preference build, Mac locked, unlock request pending. Source timestamps/warning-specific styling, selectable readonly log text (shared readonly selection infrastructure), status-menu Validate/pending/outside-dismiss behavior, watcher timing/large-root/platform checks and full paired acceptance remain open. Mobile deferred; no paired page accepted.


Rendered Sass/state batch finalized:294 complete workspace tests passed, zero failures, one optional performance ignore (`sass-status-rendered-workspace-tests.txt`); Clippy/build pass. Mouse activation now focuses the retained editor-status/Sass handles so subsequent keyboard input cannot accidentally edit the source buffer. Native GUI acceptance still pending. User answered the unlock request with Continue code work for now; stay in code, do not keep requesting unlock or use another input method.

Sass is now event-driven (`sass_watch.rs`), replacing repeated idle directory scans. One root-owned native watcher ignores access/CSS/map/dependency/recovery events, includes Sass partials/config and structural directory changes, coalesces bursts with a shared latest instant and one bounded wake, and retains up to100 pending errors. A250ms quiet window corresponds to Source write stability100ms plus debounce150ms; initial ready delay150ms. Changes arriving during a compilation remain pending for the next pass. Directory additions/removals now participate in inventory without directory-mtime/CSS rewrite loops. Watcher setup/OS-loop joins run off runtime event threads. Failed watchers keep enabled/error state but release their dead lease so an explicit start can create a new generation; registering another owner preserves existing failed/disabled root state instead of silently restarting it.

Real FS notification tests pass (`sass-events-tests.txt`,2pass): a burst settles to the last SCSS contents, CSS does not wake another pass, and cancellation ends waiting. Engine integration passes (`sass-events-engine-tests.txt`,2pass), including multiple roots, shared owners, partial deletion/recovery and close/reopen. Added config-only event coverage: writing vela-sass.json redirects real compressed CSS into newly created output directories with relative maps; deleting config restores sibling output without changing Sass content.

Output I/O no longer holds the global Sass state mutex. Per-physical-root write gates persist weakly across close/reopen generations while an old operation is alive; independent roots and snapshot reads remain unblocked, and replacement writes serialize after an already-started old operation. Cancellation is checked before staging and immediately before atomic rename, preserving prior CSS and cleaning staged files. Ownership and staged-cancellation regressions pass; Source shared status/log history and late generation checks remain intact. Latest complete workspace gate299 passed, zero failures, one optional performance ignore (`sass-events-complete-workspace-tests.txt`); final Clippy passed (`sass-events-complete-clippy.txt`), final build running. Earlier event gates297 passed before the per-root output/cancellation regressions. New compiler/status/events/viewport bundle has not replaced the running prior QA preference build because the user chose code work while locked. No native screenshot acceptance claimed. No page visually accepted; mobile deferred.


File-workspace backend added (`file_workspace.rs`, Engine::open_file_workspace). Absolute selected paths resolve to a regular real file, canonical parent and exact Unicode basename; symlinks follow the selected real file without moving/copying it. Directory/missing/relative targets reject before runtime creation. A captured oneshot reply returns the newly created conversation and file target; creation only adds inventory, leaving active context, views, observers, Sass and terminals untouched until the frontend accepts/activates the captured request. Closed receivers/cancellation skip remote creation when still possible; an already-created runtime conversation remains inventory without owning an invisible context.

Two target tests passed (`file-workspace-target-tests.txt`). The delayed-history probe passed before changing observation code (`file-workspace-history-tests.txt`; original diagnostic filename history-negative also retained). Existing ObservationPool.submit already attaches lazily, so no observer fix was necessary or made. Real integration proves bad targets create zero conversations, a Unicode file creates exactly one canonical-parent conversation, active context is preserved, and the file can be read after activation while history is deliberately blocked. Original file bytes remain unchanged. The earlier attempt to run that test was rejected only because automatic approval review hit its usage limit, not because the action was unsafe; after user continued, the same authorized local-fixture test ran successfully.

Complete file-backend workspace gate301 passed, zero failures, one optional performance ignore (`file-workspace-workspace-tests.txt`); Clippy passed (`file-workspace-clippy.txt`), build running. This is backend groundwork, not a completed frontend single-file workflow. Launch/Finder/drop routing, single-file sidebar and existing-workspace standalone files remain to wire. The latter requires separating document owning-context identity from physical storage paths; current Documents still shares directory-keyed buffers across conversations. Continue that ownership work without discarding independently retained dirty buffers/undo. User chose code work while locked; no more GUI attempts or repeated unlock requests for now. No new QA bundle installed, no paired page accepted, mobile deferred.


Document context ownership now separates `DocumentKey.workspace` (logical activation) from `directory` (physical root). Legacy constructor remains for physical explorer prefixes/requests; UI editor activations receive opaque UUID scopes per runtime context, retained through navigation and retired only after acknowledged workspace closure. Titlebar tabs, active editor, save/conflict/focus callbacks and new-file activation use logical scopes. Engine editor reads/writes, retained observation leases and Save All use physical directories. Explorer physical renames reserve/update all affected contexts while preserving each owning scope, tabs, undo and dirty state; receipts/watch updates fan out to every retained key at the physical path. Scoped close refuses pending I/O/modified/conflicted buffers and preserves sibling contexts on the same root. Core26 document tests pass, including independent same-file edits/undo/save echo conflicts, shared physical rename reservations and guarded scope retirement. UI close regression now uses two logical contexts on one physical folder. Full first ownership rerun passed (`document-context-workspace-recheck.txt`), UI check/Clippy/build passed, but subsequent full UI gate repeated the delayed-history file read timeout; this was not declared resolved by passing retries.

Found that ObservationPool's file-job thread synchronously initialized the macOS OS watcher before accepting any job. Watch setup now runs on a separate owned startup thread, transferring its watcher back through the generation worker queue; if the context closes before setup returns, the undeliverable watcher is dropped on that startup thread. File operations continue during startup or setup failure. On watcher readiness, a reconciliation rescan covers files read in the startup gap. Deterministic regression holds setup blocked and proves the original file is readable before release and after setup failure (`file-watcher-startup-tests.txt`,1pass); no longer relies on merely extending a timeout. Latest full gate `document-context-watcher-workspace-tests.txt` currently running. Native QA remains the old preference build; no package replaced or GUI input attempted because user selected code-only while Mac locked. Live same-directory tab/close/rename acceptance, full single-file frontend and full paired screenshots remain pending. Mobile deferred; no paired page accepted.


Final document/watcher gate307 passed, zero failures and one optional performance ignore (`document-context-final-workspace-tests.txt`); final Clippy/build pass. Startup-gap regression initially used a4s watcher-readiness deadline and exposed real macOS setup taking about3.8s in isolation; readiness now gets15s while the separate held-startup read keeps its strict2s limit. Original integration file-read3s deadline was not relaxed. Real pre-watch external edit is reconciled after readiness; jobs also remain available after an explicitly reported setup failure. Earlier failed gate/diagnostic logs are retained rather than replaced. No native GUI acceptance of new scope/watcher code yet.

Next Sass metadata slice in progress: replaced boolean error-only SassLine with timestamp milliseconds and typed Info/Warning/Error. Production lines timestamp at creation and retain metadata through bounded root history; compiler warnings are now distinct warnings. UI uses macOS localized medium time and `time · message`, Source warning yellow/error red/normal text color, preserving preformatted layout. Non-macOS local-time fallback exists but full locale/platform acceptance is not claimed. Added real `@warn` compiler assertion that warnings carry nonzero timestamps and leave healthy On state. Check passes (`sass-log-local-clock-check.txt`); complete metadata gate and Clippy running. Readonly log selection and full Sass status-menu interaction remain open, as do frontend file-workspace flows. No GUI input/capture attempted under user's code-only preference, no new bundle installed, mobile deferred.


Sass metadata gate307 passed, zero failures and one optional ignore (`sass-log-metadata-workspace-tests.txt`); Clippy/build pass. Real `@warn` produces timestamped Warning lines without changing healthy On state; rendered100-row layout test passes with timestamp prefixes and warning/error rows. Native output clock uses NSDateFormatter medium local time; non-macOS fallback uses local chrono clock, with locale-specific parity still unverified. Added outside-pointer dismissal to the status popover, exempting its measured trigger bounds so clicking the trigger still toggles closed. Uses the established GPUI canvas-bound pattern from the existing tab menu. Outside-dismiss Clippy/build pass; native interaction remains queued.

Sass menu toggle now waits for an explicit engine acknowledgement rather than mutating enabled optimistically. Engine async update_sass_watch returns the completed Starting/Off snapshot or the actual failure (including unregistered/invalid contexts and preference persistence errors); old fire-and-forget API still broadcasts state for existing callers. UI captures context UUID and request UUID, blocks repeated activation while that context's request is pending, and ignores obsolete replies after navigation/closure. Successful ACK reads latest engine state rather than overwriting newer On/Error broadcasts with an older Starting reply. Pending has dimmed presentation and accessibility description; GPUI's current div API lacks a disabled-state setter, so full disabled AX parity remains open. Real integration now asserts Starting enable ACK, missing-context failure and Off disable ACK. Toggle Clippy passes; full gate/build currently running (`sass-toggle-ack-workspace-tests.txt`, `sass-toggle-ack-build.txt`). No GUI interaction used; QA bundle still old preference build. Readonly Sass log selection, status Validate/LSP and exact menu metadata/visual acceptance remain unfinished. Full frontend file-workspace flow and other matrix gaps remain; mobile deferred.


Acknowledged Sass control gate307 passed, zero failures and one optional ignore; Clippy/build passed (`sass-toggle-ack-*`). Added a final pre-dispatch activation check inside the scheduled GPUI task: if navigation/root closure superseded the captured context before the task first runs, it clears only its own pending token and sends no engine command. Once dispatched, actor ordering and the response scope check protect closure/navigation. Final activation-guard Clippy passed (`sass-toggle-activation-clippy.txt`); final guard build running. No repeated broad test run for this small guarded-dispatch adjustment after the full gate passed. All latest QA GUI checks remain pending under code-only preference; do not package over the running old bundle or attempt shared input. No paired visual page accepted. Migration remains incomplete; continue functionality before screenshot phase, mobile deferred.

Final activation-guard build completed successfully (`sass-toggle-activation-build.txt`); owned-path diff whitespace check clean. Latest code is built but not installed into the running QA bundle.


User requested a durable migration map for another agent as subscription ends. Created `docs/migration/START-HERE.md` with copyable continuation prompt, read order, current307-test/Clippy/build evidence, source HEAD drift warning, immutable safety/GUI/fake-runtime constraints, detailed remaining checklist and exact next Changes resume point. Current Source HEAD verified `7df3bd45d19b8f116f0b8a25b7a26c0f8bcd69cf`, whereas matrix header is historical. Source design files are dirty; never reset reference. Last Changes work is inspection only: Source reviews observed file updates and clean-baseline/dismiss semantics; native currently renders Git status. No Changes implementation has been written yet. Do not invent staging/commit UI from stale broad inventory. Added durable repo-local `scripts/migration-cargo.sh`, replacing reliance on ephemeral tmp wrapper; syntax validated and offline workspace check running. Handoff explicitly notes all unfinished work includes untracked local files and is not committed/pushed, so a fresh GitHub clone alone is insufficient. No GUI/input/captures performed.

Handoff verification complete: checked-in toolchain wrapper syntax and offline workspace check passed (`handoff-toolchain-check.txt`); all START-HERE relative document links resolve.


Changes review implementation: `vela-engine::changes::Review` owns logical-context state, applies Source known-clean/unknown-baseline rules and local dismissal, compact directory rows, hidden prefixes and rename remapping. Eight domain/diff tests pass: exhaustive small-sequence minimum edit counts/reconstruction,25,000-line sparse edit retaining all lines, source-derived tree/clean/dismiss semantics and58 exact line-count fixtures from Source diff@8.0.2 (`changes-source-diff-tests.txt`, reproducible `scripts/reference-changes-diffs.cjs`). No production Source files/profiles were changed.

New GPUI ChangesView replaces primary Git-status-only review. Context UUIDs retain independent views; observations fan out by physical directory; snapshots seed only at context creation so cleared rows do not reappear on navigation. Root close retires its scope. Physical rename remaps review records, detach hides only the requesting context, and explicit recreation unhides that exact path. Clear controls never touch disk/editor buffers. Deleted-file reads now carry explicit FileContent.deleted; UI creates an empty deleted tab rather than indefinite Loading. Fixture integration tests missing-file access while replay is held.

Review tree and diff rows are virtualized. Diff/stat work and exact font/tab width measurement run in background with request UUID rejection; wrap-off has real scroll reach and wrap-on clears horizontal offset. Rendered1200-file regression initially failed because full row replacement lost the selected header after background completion (`changes-review-rendered-tests.txt`); fixed by preserving visible row identity/offset (`changes-review-rendered-anchor-tests.txt`,2pass). Further shaped-scroll and final horizontal/wrap tests pass (`changes-horizontal-wrap-tests.txt`,3pass), including stale jobs, collapse/clear, final reveal and unchanged input content. Initial test compile also revealed a GPUI test-macro recursion from wildcard imports; explicit test imports fixed it. Full initial Changes gate passed (`changes-review-workspace-tests.txt`); Clippy passed, initial app build ran. Follow-up hidden scope tests pass (`changes-hidden-scope-tests.txt`).

Review wiring exposed prior observer startup rescan publishing untouched files as Observed. WatcherReady now schedules reconciliation only for already tracked entries and emits a directory refresh separately. Real startup-gap test also asserts untouched neighbor absence (`changes-startup-observation-tests.txt`, running). Complete appropriate final checks after it passes. Source/visual gaps remain: Pierre syntax/intraline/hunks/EOF details, sticky headers, icons, per-section horizontal scrollers (native currently shared), locale sorting, readonly selection, full accessibility/focus/nested ownership and live new-bundle acceptance. Code-only preference respected; no GUI input/captures or new package installed. No page accepted; mobile deferred. START-HERE resume point updated rather than left as inspection-only.

## Desktop file launch, recovery notice and regex Find (2026-10-31)

Changes observer follow-up passed: `changes-startup-observation-tests.txt` has 11 observation tests green, including untouched neighbors absent after startup reconciliation. The subsequent complete Changes gate passed 318 tests, zero failures, one optional performance ignore (`changes-final-workspace-tests.txt`); final Clippy and app build passed (`changes-final-clippy.txt`, `changes-final-build.txt`). No GUI acceptance is implied and the visual gaps above remain.

Fresh-launch behavior now matches current Source rather than the older native assumption: a previous `last_session` is not reopened automatically. Recents still reopen by id. Explicit positional/`--workspace` paths, `VELA_WORKSPACE`, and app-scoped macOS open URLs enter a connection-ready queue. Directories retain the existing open-directory path. Files resolve and create their parent conversation through the captured `open_file_workspace` request, activate only the successful current request, retain a per-conversation single-file sidebar, and open the exact file after its context view exists. Finder files already inside the active workspace open as tabs; an outside file received while a conversation is active reports that standalone external-tab ownership is not implemented instead of stealing the context. UTF-8/percent-decoded Finder URL routing has a unit test.

Editor Find now exposes Source's regular-expression mode (`.*`, Alt+R). Regex queries support capture replacement, whole-word filtering, invalid-pattern feedback and zero-width matches; Replace All remains one atomic undo, while literal mode continues to treat `$` literally. New core tests cover captures, whole-word boundaries, invalid patterns, zero-width replacement and the existing highlight limit; a rendered Find test covers regex toggle, capture Replace All and undo.

Recovery now has an engine-owned load/acknowledge path and a native notice matching Source's unacknowledged artifact list with Open and Acknowledge controls. Records load on context activation and refresh after failed/conflicting saves and failed renames; file I/O runs off the UI thread. Acknowledgment preserves artifact bytes and rejects unknown/traversal IDs. Retention matches Source's 24-hour settled and 7-day interrupted windows and runs during once-per-workspace recovery initialization. Core tests cover acknowledgement bytes/rejection and expired transaction purge. Legacy recovery-location migration, live UI exercise and full visual acceptance remain open.

Final complete workspace gate: 324 passed, zero failures, one optional performance ignore (`file-launch-recovery-regex-workspace-tests.txt`). Clippy and app build evidence are `file-launch-recovery-regex-clippy.txt` and `file-launch-recovery-regex-build.txt`. All work remained code-only: no shared input, GUI inspection, screenshot, package installation or Source write. No page is accepted; mobile remains deferred. Remaining desktop slices include standalone outside-file tabs/panel ownership, editor preserve-case/selection/workspace search, full Changes presentation, recovery legacy migration, and the rest of the feature matrix before screenshot parity.

### Standalone external-file follow-up

The preceding note that outside files still lacked standalone ownership was superseded by a follow-up implementation in the same session. Files outside the active workspace now open as writable tabs attached to that conversation. Engine document reads/writes detect absolute tab paths and use bounded direct I/O with a private sibling temporary; the write path never creates workspace recovery beside the external file. Expected-content conflicts, missing-file recreation, Save All and the existing editor conflict controls apply. The editor surface now routes external drops through the same launch/file/directory path logic. Rendered/file-system tests cover standalone read/write/conflict/recreate and absence of recovery in the external parent; the engine integration test reads a Unicode external file without opening a workspace recovery store. Physical OS drag acceptance and simultaneous multi-path ordering remain.

Latest complete workspace gate: 325 passed, zero failures, one optional performance ignore (`standalone-drop-final-workspace-tests.txt`); the first follow-up Clippy correction is recorded below. Still no GUI/capture, package install or Source write; no page accepted and mobile deferred.

Clippy correction: the first `standalone-drop-final-clippy.txt` run failed on one unused test-local temporary, not product behavior. Removed only that binding and reran the complete suite (`standalone-drop-final2-workspace-tests.txt`, 325 passed, one ignored), Clippy and app build (`standalone-drop-final2-clippy.txt`, `standalone-drop-final2-build.txt`). The failing log is retained as the correction record.

## Session revert and usage meter (2026-10-31)

Session revert now uses the runtime's `stage`/`commit`/`clear` endpoints. `vela-opencode` exposes the three calls; the engine stages the selected message with file restoration, commits, and forces an authoritative `spawn_open` history reload. A commit failure clears the staged boundary and reports the error instead of leaving a transaction the native UI cannot reach. `RevertFinished` is request-scoped and rejected after navigation/retirement.

User transcript rows now expose Copy, Undo and Edit with independent retained focus identities. Copy writes the raw prompt. Undo stages/commits the message boundary. Edit does the same and, after success, restores the original prompt into the composer and draft store. Controls are visually disabled while a run is active and the engine also rejects busy reverts.

The agent panel's static usage glyph is now a real context control. It shows the selected model's context percentage when a limit exists and opens a session-usage popup with total cost, context bar/counts, input/output/reasoning/cache counts and total tokens. The popup dismisses on outside pointer or Escape. Provider usage windows/credits remain unimplemented because the native provider-usage backend is not present.

Engine integration covers the exact stage body (`messageID` plus `files:true`), successful commit, history reload, and a deliberately failed commit that invokes clear. Focused evidence is `revert-failure-engine-tests.txt`. The latest complete workspace gate is 326 passed, zero failures, one optional performance ignore (`revert-usage-final2-workspace-tests.txt`); Clippy/build evidence is `revert-usage-final2-clippy.txt`/`revert-usage-final2-build.txt`. No GUI/capture, package install or Source write; no page accepted and mobile deferred. Remaining revert work is the Source staged-revert dock/diff preview and visual acceptance; provider usage and live usage-popup acceptance remain open.

Clippy correction for this slice: the first final Clippy run failed on a redundant `SharedString::from` and a `clone` of the Copy `SessionUsage`; both were removed, then the complete suite and Clippy/build were rerun (`revert-usage-final2-*`). The failing log remains as the correction record.

## Pending agent forms (2026-10-31)

Forms now have a full native session lifecycle. `vela-opencode::forms` normalizes malformed optional metadata field-by-field; the client lists session forms and replies with the runtime's `{answer: {...}}` payload or cancels with DELETE. The engine owns a per-session form cache, refreshes off the UI thread, retires forms with their session, and emits request-scoped finished events.

The UI uses a dedicated `FormDock` entity: text/option, number, integer, boolean, multiselect and external controls; required-field gating; Submit/Dismiss; retained button focus; and stable text-input entities so the three-second refresh does not erase in-progress answers. Polling only follows the active session and is cleared on navigation/close. Engine integration covers normalized fields/defaults, reply body/ID, cancellation and refresh. A rendered test proves incomplete forms cannot submit, typed answers survive a refresh, and the exact answer object is emitted.

The first forms gate exposed an unrelated flaky Sass test that parsed a partially written PID file. It now ignores incomplete writes until a valid PID appears; the product code was unchanged. Final evidence: `forms-final2-workspace-tests.txt` (329 passed, one ignored), `forms-final2-clippy.txt`, `forms-final2-build.txt`. Still open: location-global and delegated child-session forms, conditional/hidden fields, custom option entry, min/max/pattern constraints and full live/visual acceptance. No GUI/capture, package install or Source write; no page accepted and mobile deferred.

Forms follow-up: the engine now merges session forms with location-global forms, preserves each form's true session ID, validates that a reply/cancel targets a cached form owned by the active workspace, and sends the percent-encoded `x-opencode-directory` header for non-active/global forms. The integration fixture covers a session form plus a global form, exact answer payloads, and the encoded directory header. Conditional/hidden fields and constraint metadata remain intentionally unimplemented because the current Source FormPrompt does not render them either. Latest complete gate remains 329 with Clippy/build pass (`forms-global-workspace-tests.txt`, `forms-global-clippy.txt`, `forms-global-build.txt`).

## Slash command execution (2026-10-31)

Composer sends beginning with `/name` now route through the engine instead of becoming prompt text. `compact`/`compress` invoke the runtime compaction endpoint; other names first resolve against runtime skills and activate the matching skill, otherwise they execute the runtime command with the remaining text as arguments. Engine completions are request-scoped and the UI reports failures without clearing unrelated input state. A production parser test covers command splitting and invalid names.

Engine integration adds a compact call, a runtime command with arguments, and a skill activation, asserting all three completion events and request bodies. The client exposes the new compact/command/skill/list-skill endpoints.

The source-style autocomplete menu, catalog loading, argument/file completion and runtime capability gating remain unimplemented and are recorded in the matrix. Validation initially caught two Clippy-only naming issues (`RunSlashCommand`/`SlashCommand` enum variants ending in `Command`); both were corrected without behavior change and the failing logs are retained. Final evidence: `slash-commands-final2-workspace-tests.txt` (330 passed, one ignored), `slash-commands-final2-clippy.txt`, `slash-commands-final2-build.txt`. No GUI/capture, package install or Source write; no page accepted and mobile deferred.

Slash autocomplete follow-up: the client now builds a location-scoped catalog of built-in, runtime command and skill entries; the engine refreshes it per active session. The composer renders a filtered slash menu with descriptions/kind, retains mouse execution, and intercepts Up/Down plus Enter/Tab for keyboard selection while preserving normal multiline Enter behavior when no menu is open. Command catalog normalization and the engine catalog refresh are covered by unit/integration tests. Argument completion, `@` file references, capability gating and exact visual acceptance remain open.

Latest complete workspace gate: 331 passed, zero failures, one optional performance ignore (`slash-autocomplete-final-workspace-tests.txt`); Clippy/build pass (`slash-autocomplete-final-clippy.txt`, `slash-autocomplete-final-build.txt`). No GUI/capture, package install or Source write; no page accepted and mobile deferred.

## Composer `@` file references (2026-10-31)

The composer now detects `@query` at the active caret, excluding characters typed after whitespace/inside an existing mention span. The engine issues a request-scoped `api/fs/find` search for the active session directory and filters hidden recovery-store components before publishing results.

The UI renders a bounded file menu with arrow-key plus Enter/Tab selection and click activation. Selecting a result replaces the trigger span with the exact `@relative/path` token, restores the caret, and stages a prompt file whose mention start/end/text match the inserted token. The engine integration fixture verifies ranked paths and recovery filtering. Path escaping, stale-scope/timing edge cases, live runtime delivery and complete keyboard/visual acceptance remain open.

Final evidence: `references-final-workspace-tests.txt` (331 passed, one ignored), `references-final-clippy.txt`, `references-final-build.txt`. No GUI/capture, package install or Source write; no page accepted and mobile deferred.

## Browser handoff, preserve-case and recovery import (2026-10-31)

Browser new-window requests are now delivered as a `BrowserEvent::External` only when the URL passes the existing credential-free HTTP(S) policy. The guest popup is still denied, and the GPUI owner hands the URL to the OS browser. Download policy remains denied.

Editor Find adds Source-style preserve-case replacement: lowercase matches produce lowercase replacements, uppercase matches uppercase, capitalized matches a capitalized replacement, and mixed case keeps the replacement unchanged. It is available from the `AB` control and Alt+C, with core replacement coverage.

Recovery initialization now imports valid transaction directories from `.vela-recovery`, `.mira-recovery`, `.openshell-recovery` and `.orbit-recovery` into the current recovery root once per workspace, never overwriting current transactions and leaving the legacy directory in place if an entry cannot move.

Final complete workspace gate: 333 passed, zero failures, one optional performance ignore (`preserve-browser-recovery-final-workspace-tests.txt`); Clippy/build pass (`preserve-browser-recovery-final-clippy.txt`, `preserve-browser-recovery-final-build.txt`). No GUI/capture, package install or Source write; no page accepted and mobile deferred.

Follow-up: browser permission requests are now explicitly denied through Wry's permission handler, matching the reference session policy. Editor Find also supports find-in-selection scope (Alt+L): the search pattern stays inside the captured selection for matching/navigation/replacement while preserving text outside it. A Clippy-only test literal (`[range]` to `vec![range]`) was corrected; all product behavior is unchanged.

Latest complete workspace gate: 334 passed, zero failures, one optional performance ignore (`scope-permission-final2-workspace-tests.txt`); Clippy/build pass (`scope-permission-final3-clippy.txt`, `scope-permission-final3-build.txt`). No GUI/capture, package install or Source write; no page accepted and mobile deferred.

## Agent TUI launcher (2026-10-31)

The terminal PTY layer now supports spawning an explicit program/argument/environment set in addition to the login shell. The engine exposes a terminal-command path with the same ownership, resize, bounded output, exit relay, selection and IME routing as ordinary terminals.

The terminal header has a `TUI` action that launches the configured `VELA_OPENCODE_BINARY` (or `opencode`) in a new tab named Agent TUI, inside the active workspace. A PTY regression proves explicit command arguments and environment variables reach the child. The source's embedded agent-panel TUI mode, runtime palette/capability gating and exact visual parity remain open.

Initial Clippy validation caught an enum variant ending in `Command`, an eight-argument API and a complex tuple; the command is now a `TerminalCommand` struct/`TerminalExec` variant with a private process descriptor. The failing log is retained. Final evidence: `tui-terminal-final2-workspace-tests.txt` (335 passed, one ignored), `tui-terminal-final2-clippy.txt`, `tui-terminal-final2-build.txt`. No GUI/capture, package install or Source write; no page accepted and mobile deferred.

Changes review follow-up: tree directories and files now sort case-insensitively with a stable original-case tie-breaker, matching Source's locale-oriented ordering for mixed-case paths, and section headers render the file-type glyph used by the Explorer. A core ordering regression covers mixed-case directories/files. Final gate: 336 passed, one ignored (`changes-sort-final-workspace-tests.txt`); Clippy/build pass (`changes-sort-final-clippy.txt`, `changes-sort-final-build.txt`). Sticky headers, per-section diff scrolling, exact Pierre syntax/intraline/hunks, readonly selection and paired visuals remain.

## W3C validation (2026-10-31)

The editor status menu now exposes Validate HTML/CSS. The engine posts bounded content to the same W3C HTML/CSS endpoints as Source, parses JSON Nu messages and both SOAP/XML and line-oriented CSS diagnostics, and returns typed diagnostics. The Problems panel displays the validated path plus line/column messages with error/warning styling. Local validator fixtures verify request content, both parser formats, line normalization and unsupported-extension behavior.

Remaining: editor markers, SCSS validation, cancellation/retry and live network acceptance. Final gate: 338 passed, zero failures, one ignored (`validation-final-workspace-tests.txt`); Clippy/build pass (`validation-final2-clippy.txt`, `validation-final2-build.txt`). A first Clippy pass flagged one needless borrow in the XML text extractor; it was removed without behavior change and Clippy/build rerun. No GUI/capture, package install or Source write; no page accepted and mobile deferred.

## Multi-panel activation foundation (2026-10-31)

Added a bounded eight-panel registry with unique session activation. Each panel retains its title, composer, transcript blocks/phase, permissions, todos, usage, model/agent selection, logical scroll offset and stick state. Switching saves the current snapshot and restores the target; opening the ninth distinct session is rejected. Closing a panel activates its neighbor and does not delete the conversation. Session removal retires its panel, and Session inventory updates refresh panel titles.

An accessible tab strip appears when more than one panel is open, with selected state, click activation and per-panel close controls. This is the activation foundation, not tiled simultaneous rendering: per-panel editor/focus ownership and arbitrary slot resizing remain open.

Model tests cover the eight-panel bound, duplicate activation, snapshot retention and neighbor close. Final complete gate: 339 passed, zero failures, one ignored (`panels-final-workspace-tests.txt`); Clippy/build pass (`panels-final2-clippy.txt`, `panels-final2-build.txt`). A Clippy-only bool-comparison test assertion was corrected before the passing run. No GUI/capture, package install or Source write; no page accepted and mobile deferred.

## Simultaneous tiled agent panels (2026-10-31)

Panels now render simultaneously instead of only swapping one active panel. `crates/vela-ui/src/panel_layout.rs` mirrors the reference grid: one or two panels use one row, three or more use two rows with `ceil(count/2)` columns, and an incomplete final row divides the full width. Slots expose percentage geometry and per-row width normalization; splitters clamp panels to a 15% minimum.

Every open tile renders its retained transcript snapshot and scroll container, a dedicated `TextInput` composer entity and focus handle, a header with activate/close controls, and a footer with an independent send action. Clicking a tile activates its session, restores its session-scoped document workspace/active editor tab, and focuses that tile's composer. The active tile keeps the complete rich transcript, forms dock and composer; inactive tiles use their snapshot plus the compact composer until activated. Inactive panel snapshots are refreshed from engine views on updates so they stay live.

Widths are stored per panel (`PanelState::width`), normalized per row and resized through a globally captured splitter drag. Tests cover reference grid shapes, incomplete-row coverage, width normalization, minimum clamping, row boundaries, snapshot preview tails and panel width retention. A GPUI test creates two real `TextInput` entities, focuses each in turn and proves both drafts/focus survive independently.

Remaining tiled-panel gaps: inactive tiles do not yet render rich inline tool/answer controls or per-tile attachments/`@`/slash menus; same-root panels share editor document buffers (different sessions still own separate workspace scopes); live visual acceptance is pending.

Final complete gate: 346 passed, zero failures, one ignored (`tiled-panels-workspace-tests.txt`); Clippy/build pass (`tiled-panels-clippy.txt`, `tiled-panels-build.txt`). No GUI/capture, package install or Source write; no page accepted and mobile deferred.

## Changes presentation, launch queue and terminal input (2026-10-31)

Changes review now extracts a reusable section header with distinct real/sticky
element IDs. The header above the visible diff stays pinned once its row scrolls
under the viewport, including collapse/open/clear controls and live diff counts.
A pure top-row helper covers partial headers, line/pending rows and empty lists,
and the 1,200-file rendered regression asserts the sticky header exists after
scrolling. Replaced line pairs now receive byte-range emphasis for their changed
middle, computed on char boundaries (Unicode covered) and rendered as a
background run; tabbed lines keep the preformatted fallback. Final lines without
a trailing newline render the unified-diff `\ No newline at end of file` marker.

The pending Finder/second-instance launch queue now mirrors Source
`PendingOpenPaths`: whitespace-only entries drop, paths trim and de-duplicate
across pushes, only the first ten are retained, and order is preserved. A model
regression covers trim/dedupe/cap/order.

Terminal input now supports DEC mouse reporting. `Emulator::encode_mouse`
implements click, drag and all-motion modes in X10 and SGR encodings, including
press/release, wheel, drag/move, button numbers and Shift/Alt/Control modifiers.
Left/right/middle presses, release, wheel and motion are routed from the terminal
grid/observer to the PTY, motion is de-duplicated by cell, and the cursor becomes
an arrow while the application owns the mouse. DECSET 1004 focus reporting sends
`ESC[I`/`ESC[O` from focus/blur subscriptions installed per terminal view.
Emulator regressions cover mode transitions, SGR/X10 bytes, wheel modifiers,
release and focus reporting.

Remaining Changes gaps: hunk/EOF presentation, per-section horizontal scrolling,
readonly diff selection and live paired acceptance. Remaining terminal gaps:
complete TUI/fixed-cell/IME lifecycle and live mouse/focus acceptance.

Final complete gate: 353 passed, zero failures, one ignored
(`changes-eof-final-workspace-tests.txt`); Clippy/build pass
(`changes-eof-final-clippy.txt`, `changes-eof-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Per-section scrolling and readonly diff selection (2026-10-31)

The Changes view no longer owns one shared horizontal scroller. Each file section
gets its own persisted `ScrollHandle`, and each diff row tracks that section's
handle with a minimum content width derived from its own diff. The wrap toggle
resets and re-lays out every section, and the wrapped layout has no horizontal
extent. A rendered regression opens two long files, verifies both section
handles can overflow, scrolls one and proves the other stays at offset zero.

Readonly diff selection stores byte-column anchor/cursor points. Mouse down
focuses the Changes view and starts a selection; drag updates the cursor across
rendered row events, Shift-click extends, and release ends the drag. The selected
range is normalized into inclusive row bounds with a zero-column end folding back
to the previous full line. Selected characters render with a selection background
while intraline emphasis keeps its own background, and line rows with tabs keep
the preformatted fallback. `Cmd+C` and a toolbar Copy action write the exact
selected characters joined by newlines. A rendered drag test selects characters
1–4 of an added line and asserts the copied text is `bcd`, plus the Copy control
appears.

`@pierre/diffs` hunk separators are only enabled for partial patches; the Changes
view supplies full before/after contents, so its non-partial reference path does
not produce hunks. Remaining Changes work is live/accessibility/paired acceptance
and legacy recovery presentation, not hunk generation.

Delivered launch paths now activate the running app and every Vela window before
processing, covering Finder/second-instance handoff at the app level; live
second-instance routing still needs desktop acceptance.

Final complete gate: 355 passed, zero failures, one ignored
(`changes-selection-activation-final-workspace-tests.txt`); Clippy/build pass
(`changes-selection-activation-final-clippy.txt`,
`changes-selection-activation-final-build.txt`). Clippy's type-complexity check
required a `LineGeometry` alias for the rendered line bounds map, and delivered
launch paths now activate the app/window before processing. No GUI/capture,
package install or Source write; no page accepted and mobile deferred.

## Sass readonly selection (2026-10-31)

Sass output moved from a stateless render function to a `SassOutput` entity that
owns the watch state, directory, scroll handle, focus and selection. The parent
syncs engine state into the entity; any real state/directory change clears the
selection and schedules a repaint. Log rows expose mouse-down/move/up handlers,
Shift extends from the existing anchor, release/out ends the drag, and selected
rows use the selection background. Cmd+C and a Copy control write the exact
displayed strings (localized timestamp + message) for the selected row range.
Each row remains preformatted so indentation and wrapped Unicode stay intact.

The existing 100-row wrapped viewport/scroll regression now runs against the
entity. A rendered drag test selects rows 0–2 and asserts the copied text and
copy control; Sass menu metadata now exposes toggled state, and the editor
status menu reports a Source-style error/warning/no-problems validation summary.
Validate/SCSS language diagnostics, disabled accessibility semantics, locale
metadata and paired visual acceptance remain.

Final complete gate: 357 passed, zero failures, one ignored
(`sass-validation-final-workspace-tests.txt`); Clippy/build pass
(`sass-validation-final-clippy.txt`, `sass-validation-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Native SCSS diagnostics (2026-10-31)

The status Validate action now routes `.scss`/`.sass` buffers to the pinned Sass
compiler instead of W3C. `sass-compile.mjs` gained a `--validate` mode that reads
the unsaved buffer from stdin and calls `compileString` with the real source URL,
workspace load paths, package importer and a warning logger. It returns
structured line/column diagnostics without writing output; compile errors become
error diagnostics and `@warn`/logger messages become warnings, both with source
`scss`.

The engine exposes `validate_scss`, resolves the session workspace root, and
reuses the existing `Validation` update so the status summary and Problems panel
need no parallel path. The UI accepts HTML/HTM/CSS through W3C and SCSS/Sass
through Sass, labels the action `Validate` with `W3C / Sass`, and rejects other
extensions. Compiler fixtures cover valid imports from a real workspace, an
invalid declaration and a warning; a UI routing unit covers case-insensitive
extensions.

Remaining Sass work is disabled accessibility semantics, exact locale/menu
metadata and paired visual/live acceptance.

Final complete gate: 358 passed, zero failures, one ignored
(`scss-diagnostics-final-workspace-tests.txt`); Clippy/build pass
(`scss-diagnostics-final-clippy.txt`, `scss-diagnostics-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Sass menu accessibility and metadata (2026-10-31)

The Sass status item now derives its label, description and disabled state from a
tested metadata helper matching Source: `Starting Sass…`, `Sass: Error`,
`Sass: On`, `Watch Sass`; the error description points at Sass Output and says
whether clicking stops watching or retries starting; the normal description says
Start/Stop and names the workspace directory. Toggled state remains exposed.

While a toggle is pending, the item dims, drops focus/cursor/activation handlers,
and its AccessKit node is marked disabled through an `a11y_synthetic_children`
parent-node mutation. Unit tests cover all metadata branches, plural/label
selection and the disabled/enabled AccessKit property. Remaining Sass work is
locale/platform and live/paired acceptance.

Final complete gate: 359 passed, zero failures, one ignored
(`sass-menu-a11y-workspace-tests.txt`); Clippy/build pass
(`sass-menu-a11y-clippy.txt`, `sass-menu-a11y-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Agent TUI capability check (2026-10-31)

`spawn_tui_terminal` now resolves the runtime program before creating a PTY tab.
A non-empty `VELA_OPENCODE_BINARY` wins and must be an executable file; bare names
search `PATH` entries; missing or non-executable programs show an explicit toast
and leave no empty terminal behind. The resolved absolute path is passed to the
existing explicit-command PTY path. Unit tests create executable and
non-executable fixtures and cover configured, PATH, missing and bare-name cases.

This is the TUI launch/failure slice; live TUI interaction, embedded agent-panel
appearance, palette/capability gating and full IME candidate acceptance remain.

Final complete gate: 360 passed, zero failures, one ignored
(`sass-tui-final-workspace-tests.txt`); Clippy/build pass
(`sass-tui-final-clippy.txt`, `sass-tui-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Terminal IME candidate and focus lifecycle (2026-10-31)

Terminal composition now exposes candidate bounds anchored to the emulator cursor
cell plus the shaped composition width, including astral text; UTF-16
text/range round-trips are covered. Preedit remains local until commit, and a
cancel clears marked text without emitting to the PTY. A per-view blur
subscription cancels composition, and terminal tab switches, active-tab close,
tray hide and tray close explicitly cancel the previous composition before focus
moves.

Tests cover candidate origin/width and UTF-16 cursor indexing, local cancel, and
the existing two-terminal commit isolation. Actual OS IME candidate interaction
and exact fixed-cell/live acceptance remain.

Final complete gate: 361 passed, zero failures, one ignored
(`terminal-ime-final-workspace-tests.txt`); Clippy/build pass
(`terminal-ime-final-clippy.txt`, `terminal-ime-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Terminal cursor shapes and wide-cell placement (2026-10-31)

The emulator exposes DECSCUSR cursor shapes (block, underline, beam, hollow
block, hidden) and blinking metadata. The terminal renderer draws block,
underline, beam and hollow cursors distinctly and omits hidden cursors. Cursor
position now snaps from a wide glyph's spacer cell back to the glyph column.

Tests cover all DECSCUSR shape/blink transitions, hidden-cursor mode and the
wide-spacer cursor coordinate. Blinking cadence, exact fixed-cell glyph placement
and live terminal acceptance remain.

Final complete gate: 362 passed, zero failures, one ignored
(`terminal-cursor-final-workspace-tests.txt`); Clippy/build pass
(`terminal-cursor-final-clippy.txt`, `terminal-cursor-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Focus-aware cursor blinking (2026-10-31)

Terminal views now own a `CursorBlink` entity. Only a blinking DECSCUSR style
with terminal focus and an active window starts a 530 ms timer; each tick toggles
the phase and notifies the terminal. Losing focus, changing to a steady style, or
hiding/closing the terminal resets the cursor to visible and cancels the timer.
The timer uses a generation token so a stale loop cannot affect a restarted one.

A clock-driven test advances the test executor through multiple intervals and
verifies on/off cadence, steady unfocused behavior, non-blinking styles and focus
restart. Rendering gates the cursor on the phase, and tab switch/close/tray-hide
paths cancel the previous blink. Live cadence/fixed-cell acceptance remains.

Final complete gate: 363 passed, zero failures, one ignored
(`terminal-blink-final-workspace-tests.txt`); Clippy/build pass
(`terminal-blink-final-clippy.txt`, `terminal-blink-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Fixed-cell terminal glyph placement (2026-10-31)

Terminal rows that contain a wide glyph, combining mark, emoji or other non-ASCII
cell now render through a fixed-cell path. Each grapheme is placed at
`column * cell_width`, wide cells span two cells, combining marks stay inside
their base cell and the next glyph starts after all occupied columns. Per-cell
backgrounds retain ANSI/selection styling, and wide cursors span both cells.

ASCII rows keep the faster shaped-line renderer, where monospace advance matches
the measured cell width. Tests cover wide/spacer detection, combining/non-ASCII
detection, emoji cell flags, and rendered bounds for `a界e+mark b` at exact
7.5px columns. ASCII/fallback-font alignment and live acceptance remain.

Final complete gate: 365 passed, zero failures, one ignored
(`terminal-fixed-cells-final-workspace-tests.txt`); Clippy/build pass
(`terminal-fixed-cells-final-clippy.txt`, `terminal-fixed-cells-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Universal fixed-cell terminal rows (2026-10-31)

The fixed-cell renderer now handles every terminal row, including ASCII and
fallback-font glyphs. Each cell's grapheme is anchored at exactly
`column * cell_width` in a measured cell box, so a fallback glyph cannot shift
subsequent columns. Wide/spacer and combining behavior remain as before. The
older shaped-line ASCII path and its detection helper were removed; line-run
projection remains as test-only legacy coverage.

Rendered tests verify `ab→de` lands at 7.5px intervals and retain previously
added wide/combining/emoji offsets. Full TUI repaint/performance and live
acceptance remain.

Final complete gate: 366 passed, zero failures, one ignored
(`terminal-ascii-cells-final-workspace-tests.txt`); Clippy/build pass
(`terminal-ascii-cells-final-clippy.txt`, `terminal-ascii-cells-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Browser tab labels and close selection (2026-10-31)

Browser tab labels now follow Source's `use-browser-tabs` rules: trimmed title
wins, then the parsed hostname with `www.` removed, `New tab` for an empty URL and
the raw string when URL parsing fails. Active-tab selection after closing any tab
uses a shared model: closing before the active tab shifts it left, closing the
active tab picks the same-or-last remaining index, and closing after it leaves the
active index unchanged. Unit tests cover all label and index cases, and the app's
close path uses the tested helper.

Live browser isolation/lifecycle, downloads/permissions and paired acceptance
remain.

Final complete gate: 368 passed, zero failures, one ignored
(`browser-tabs-final-workspace-tests.txt`); Clippy/build pass
(`browser-tabs-final-clippy.txt`, `browser-tabs-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Browser loading and error placeholders (2026-10-31)

Browser tabs now carry their own error state alongside loading/history. Pane
state follows Source precedence: error (even with an empty URL), then empty URL,
then loading, then ready. While loading or errored, the native WKWebView is
hidden so GPUI can show an animated spinner with the tab label or an error
message with a Try again action. Retrying clears the tab error, reloads an
existing guest or relays navigation through the address path, and tab activation
restores that tab's error state.

A unit test covers the state precedence; existing browser label/close/lifecycle
and overflow tests remain green. Live browser isolation, downloads/permissions
and paired visual acceptance remain.

Final complete gate: 369 passed, zero failures, one ignored
(`browser-placeholders-final-workspace-tests.txt`); Clippy/build pass
(`browser-placeholders-final-clippy.txt`, `browser-placeholders-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Browser guest failure events (2026-10-31)

`BrowserEvent::Failed` now carries WKWebView web-content process termination into
the per-tab error path on macOS/iOS. Tabs track `loading_started`; a load that
exceeds 30 seconds fails with a timeout message, and begin/finish/fail transitions
clear timers and stale errors. Active-tab polling mirrors tab errors into the
shared browser error state, retry restarts loading or navigation, and the pane's
error placeholder supplies Try again.

Lifecycle tests cover process-failure text, timeout failure, retry start and
successful finish cleanup. Live browser isolation/lifecycle and preview
acceptance remain.

Final complete gate: 370 passed, zero failures, one ignored
(`browser-failures-final-workspace-tests.txt`); Clippy/build pass
(`browser-failures-final-clippy.txt`, `browser-failures-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor indentation folding (2026-10-31)

The native editor computes fold ranges from indentation of non-blank lines,
supports nested ranges, and maintains a visible-line mapping separate from
logical line starts. Folded bodies are removed from the virtualized list while
line numbers remain the real logical numbers. A 16px gutter shows collapse/expand
chevrons at fold starts, and folding splices only the changed visible suffix.

Caret and Find reveal detect a hidden target line, unfold every folded range
containing it, then scroll to the visible row. Pure tests cover nested range
computation and folded visibility; a rendered test toggles a function-body fold,
asserts hidden rows disappear and restores them via caret reveal.

Language-aware folding, split editors, multi-cursor and LSP/tooling remain.

Final complete gate: 372 passed, zero failures, one ignored
(`editor-folding-final-workspace-tests.txt`); Clippy/build pass
(`editor-folding-final-clippy.txt`, `editor-folding-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor split view (2026-10-31)

The editor toolbar now exposes Split/Unsplit. Splitting selects the next open
document tab (wrapping) for the secondary pane, creates a distinct `CodeEditor`
entity with unique element identity, and focuses the new pane. Both panes share
the document store but keep independent view state; edits propagate on the next
draw via revision syncing. Closing/relocating the active or split document clears
or remaps the split target.

Rendered tests mount two editors for independent documents, edit both, and verify
each document updates independently while both views refresh. A model test covers
next-tab/wrap/single/empty selection. Multi-group tabs, drag resize, multi-cursor,
language tooling and Emmet remain.

Final complete gate: 374 passed, zero failures, one ignored
(`editor-split-final-workspace-tests.txt`); Clippy/build pass
(`editor-split-final-clippy.txt`, `editor-split-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor split divider resizing (2026-10-31)

The split divider is now draggable. The split container measures its live width,
pointer deltas update the left fraction with a 20–80% clamp, the drag state clears
on release/out, and the divider highlights while active. The split fraction
remains in memory across re-renders; persistence is still pending.

A rendered probe test drags the divider 100px and verifies the left pane grows and
the fraction exceeds 0.5. A model test covers positive/negative deltas, clamping
and zero-width fallback. Multi-group tabs, multi-cursor, language tooling and
Emmet remain.

Final complete gate: 375 passed, zero failures, one ignored
(`editor-split-resize-final-workspace-tests.txt`); Clippy/build pass
(`editor-split-resize-final-clippy.txt`, `editor-split-resize-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Brace-aware editor folding (2026-10-31)

Fold ranges now combine indentation and matching bracket ranges. A scanner tracks
single/double/backtick strings, escapes, line comments and block comments so
brace-like text inside them is ignored. Matching `{}`/`[]`/`()` pairs that span
lines produce folds through the closing line; same-start ranges merge with
indentation folds. This folds unindented SCSS-style nesting that indentation alone
missed while preserving existing gutter/caret-reveal behavior.

Pure tests cover comment/string filtering, correction of the existing nested fold
expectations and unindented brace nesting; rendered fold/caret tests remain green.
Language-provider folding, multi-group tabs, multi-cursor, language tooling and
Emmet remain.

Final complete gate: 376 passed, zero failures, one ignored
(`editor-brace-folding-final-workspace-tests.txt`); Clippy/build pass
(`editor-brace-folding-final-clippy.txt`, `editor-brace-folding-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor multi-cursor (2026-10-31)

The editor now supports multi-cursor editing. Cmd+D expands from the primary
selection (or the caret's Unicode word) to the next identical occurrence;
repeated presses add further non-overlapping matches. Typing, Backspace/Delete,
Enter and Tab apply to every range, while the new `TextBuffer::replace_many`
applies scattered non-overlapping ranges atomically and derives shifted caret
positions from the cumulative length delta. A scattered batch is one undo
transaction that restores the whole buffer in one step.

Secondary selections paint with the same readonly selection color and empty
secondary ranges paint carets. Escape, primary mouse clicks, navigation,
select-all, paste/cut/undo/redo and IME marking clear extras; secondary offsets
are rebuilt after each simultaneous edit.

Tests cover overlapping-range filtering, shifted caret results, one-step
undo/redo for scattered replacements, rendered Cmd+D selection, simultaneous
typing, simultaneous backspace, Escape clearing and rendered multi-cursor state.
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Multi-group editor tabs (2026-10-31)

The editor can now display up to four groups simultaneously. The Split control
targets the next open tab not already visible in a group, skipping assignments
and wrapping after the active tab; when every tab is visible it adds an empty
group with the Source empty-state copy instead of duplicating a document. Groups
can be closed individually or unsplit from the end without closing their tabs.

Layout follows Source geometry: two groups split into columns, three use a
full-width second row, and four use a 2x2 grid. Percent cells keep panes
deterministic while the X/Y fractions drive draggable dividers clamped to
20–80%; a live bounds canvas maps pointer deltas for both axes. Each pane has a
unique debug/AX identity and close control, and document renames/removals remap
or empty their group slots.

Model tests cover cell coverage for 1–4 groups, next-tab targeting with wrapping,
assignment exclusion and empty-group fallback. A rendered probe mounts three-
and four-cell grids and asserts exact quadrant/full-row bounds without overlap.
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

Final complete gate: 382 passed, zero failures, one ignored
(`editor-groups-final-workspace-tests.txt`); Clippy/build pass
(`editor-groups-final-clippy.txt`, `editor-groups-final-build.txt`).

## Settings integration inventories (2026-10-31)

Settings > Servers now reports workspace MCP servers, plugins and skills from the
connected runtime. The engine adds a request-scoped `LoadIntegrations` command
and `Integrations` update; the REST client reads `/api/mcp`, `/api/plugin` and
`/api/skill` and the new inventory parsers accept both raw arrays and `{data}`
wrappers. Empty names/ids are dropped and missing status/source values become
`unknown`, matching the reference filters.

The UI keeps a session-scoped `IntegrationInventory`. Opening Servers triggers a
load, shows per-section loading/error/empty states, renders name plus source and
status badges, and exposes Retry after failures. Session switches discard stale
payloads; successful lists remain until Retry or a new session.

Tests cover parser wrappers/malformed rows, row mapping/error clearing, and
rendered loading/error/empty/populated states. Live runtime endpoints and paired
visual acceptance remain.

Final complete gate: 385 passed, zero failures, one ignored
(`integrations-final-workspace-tests.txt`); Clippy/build pass
(`integrations-final-clippy.txt`, `integrations-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Native Emmet expansion (2026-10-31)

The editor now expands Emmet abbreviations on Tab and Enter for HTML and
CSS/SCSS/Less documents. The new native parser covers the reference cases:
HTML5 doctype, known tags, ids/classes, text, child and sibling chains, `*N`
repetition and common CSS number shorthands. Unknown tags, existing closing-tag
contexts and noise fall through to normal Tab/Enter handling; HTML expansions
inside `<script>` are skipped.

Snippet tab stops flatten on insertion: defaults are emitted, `${0}` becomes the
caret, and escaped dollars become literal. The abbreviation is replaced in the
same edit transaction. Tests mirror the source expansion fixtures, parsing scan
boundaries and insertion/caret placement; a rendered editor test expands `div`
and `ul>li*3` via Tab.

Final complete gate: 391 passed, zero failures, one ignored
(`emmet-final-workspace-tests.txt`); Clippy/build pass
(`emmet-final-clippy.txt`, `emmet-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Provider usage snapshot cards (2026-10-31)

The engine now reads the short-lived desktop usage bridge document
(`vela-usage.json`) from XDG data and macOS application-support candidates. It
enforces the reference 15-minute age cap, skips malformed/stale candidates, drops
provider-less rows and parses windows, credits, plan types and provider errors.

Opening the agent-panel session-usage popup requests a snapshot. The popup adds
provider cards with status dots, plan labels, percent-left usage bars, reset
wording, credit balances and loading/empty/error states. Session token/cost rows
remain above the provider section, and the content scrolls within the popup.

Tests cover snapshot parsing, stale/malformed fallback, formatter wording and
rendered loading/empty/populated cards. Live OAuth/key provider fetches,
environment fallbacks and credential refresh remain unfinished; no live provider
credentials were accessed.

Final complete gate: 395 passed, zero failures, one ignored
(`provider-usage-final-workspace-tests.txt`); Clippy/build pass
(`provider-usage-final-clippy.txt`, `provider-usage-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Sass localized clock formatting (2026-10-31)

Sass log timestamps now use `NSDateFormatter` with an explicit current locale and
the localized `jms` template. This includes seconds and follows the locale's
`j` hour cycle, matching `toLocaleTimeString()` semantics more closely than the
previous fixed medium time style, where a user's 12/24-hour preference could be
ignored.

A macOS model test pins `en_US` and `en_GB` at epoch UTC: the US string uses
`12:00:00` with AM while the UK string uses `00:00:00` without a meridiem marker;
a later timestamp differs. The rendered Sass drag-selection regression also
asserts the copied first row retains its localized clock prefix and separator.
Platform/live visual acceptance remains.

Final complete gate: 396 passed, zero failures, one ignored
(`sass-locale-final-workspace-tests.txt`); Clippy/build pass
(`sass-locale-final-clippy.txt`, `sass-locale-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Document-word completion (2026-10-31)

The native editor now provides a bounded word-based completion popup. Ctrl+Space
collects unique identifiers from the active buffer, computes the full word range
around the caret and ranks candidates by prefix match, frequency, length and
case-insensitive spelling. Arrow keys cycle; Enter, Tab or a click accepts by
replacing the whole word; typing re-filters; Escape, navigation, mouse-down and
IME marking dismiss the popup. It is disabled for multi-cursor edits.

Tests cover Unicode word ranges, deduplication, prefix filtering and ranking,
duplicate case variants, and a rendered editor flow that opens the popup on `al`
and accepts `alpha` with Enter. This is a local word provider, not a semantic
language server.

Final complete gate: 400 passed, zero failures, one ignored
(`completion-final-workspace-tests.txt`); Clippy/build pass
(`completion-final-clippy.txt`, `completion-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded JSON syntax diagnostics (2026-10-31)

JSON documents now receive local syntax diagnostics from `serde_json` whenever
the buffer revision changes. Parser line/column positions are normalized to
one-based editor coordinates, the offending virtual row gains a red gutter
stripe, and an accessible banner reports the JSON line, column and parser
message. Empty/whitespace documents are treated as clean, and fixing the buffer
clears the marker and banner on the next sync.

Model tests cover valid, empty, malformed, trailing-comma and coordinate cases.
A rendered editor test mounts malformed JSON, asserts the marker/banner, rewrites
the buffer as valid JSON and confirms both disappear. Other language diagnostics
and semantic LSP providers remain unfinished.

Final complete gate: 403 passed, zero failures, one ignored
(`json-diagnostics-final-workspace-tests.txt`); Clippy/build pass
(`json-diagnostics-final-clippy.txt`, `json-diagnostics-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JavaScript/TypeScript/CSS diagnostics (2026-10-31)

The local diagnostics slice now covers JavaScript, TypeScript, JSX/TSX, CSS and
SCSS in addition to JSON. A string/comment/template-aware scanner reports
unmatched closing brackets, unclosed openings and unterminated strings or block
comments with one-based line/column positions. Escapes and backtick templates are
skipped; SCSS line comments are recognized while CSS only treats `/* */` as
comments. Extension routing keeps JSON on `serde_json` and leaves unknown files
untouched.

Diagnostics reuse the existing red gutter marker and accessible banner, clearing
as soon as the buffer becomes valid. Model tests cover valid script/CSS inputs,
comments/strings/templates, extra and unclosed delimiters and extension routing;
a rendered JavaScript test verifies marker/banner creation and clearing.
Semantic language servers/workspace symbols remain unfinished.

Final complete gate: 406 passed, zero failures, one ignored
(`script-css-diagnostics-final-workspace-tests.txt`); Clippy/build pass
(`script-css-diagnostics-final-clippy.txt`, `script-css-diagnostics-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Language-marker folding (2026-10-31)

Folding now merges indentation, brace and comment region ranges. Nested
`// #region`/`// #endregion`, `/* #region */`, `<!-- #region -->` and `#region`
markers each produce an independent fold from marker start through the matching
end; unmatched starts/ends are ignored. Existing gutter controls and caret/find
reveal work on the merged ranges.

Model tests cover nested regions, CSS block comments, hash markers and unmatched
ends. A rendered JavaScript test opens a region fold and verifies the hidden rows
and retained following line. Full language-provider folding (language server
ranges) remains unfinished.

Final complete gate: 408 passed, zero failures, one ignored
(`language-folding-final-workspace-tests.txt`); Clippy/build pass
(`language-folding-final-clippy.txt`, `language-folding-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded document symbols (2026-10-31)

F12 now performs a local go-to-definition for the identifier at the caret.
Common declarations are recognized (`function`, `class`, `const`, `let`, `var`,
`interface`, `type`, `enum`, `def`, `fn`, `struct`, `trait`) including
`export`/`default`/`async` prefixes and generator `*`. A single match jumps
directly; multiple definitions or Shift+F12 reference results open an AccessKit
list with source-order locations, previews, arrow/Enter navigation, mouse jumps
and a close/empty state.

References are whole-identifier matches in the active document, bounded to 100
results. Model tests cover declaration forms, offsets/line columns, Unicode
identifiers, ordering and no-match cases; a rendered JavaScript test verifies
F12 jump and the Shift+F12 panel. This is document-local, not a cross-file
language server.

Final complete gate: 412 passed, zero failures, one ignored
(`symbols-final-workspace-tests.txt`); Clippy/build pass
(`symbols-final-clippy.txt`, `symbols-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Workspace symbol search (2026-10-31)

Cmd+T now indexes declarations from every open document in the active workspace.
The query comes from the identifier under the caret (empty means all), and the
results are case-insensitive prefix-filtered, sorted by name/path/offset and
bounded to 100 file-qualified entries. The AccessKit panel supports arrow/Enter
and mouse selection; choosing an entry emits `OpenSymbol {key, offset}` and the
app opens that tab, focuses its editor and places the caret.

Tests cover workspace filtering/sorting/limits/files and a rendered two-document
flow that finds a symbol in another open file and asserts the emitted target.
This is an open-document index, not a language-server workspace index.

Final complete gate: 414 passed, zero failures, one ignored
(`workspace-symbols-final-workspace-tests.txt`); Clippy/build pass
(`workspace-symbols-final-clippy.txt`, `workspace-symbols-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML/XML tag folding (2026-10-31)

Folding now includes nested HTML/XML elements. The scanner matches opening and
closing tag names across lines, handles `>` inside quoted attributes, ignores
comments, void/self-closing elements and raw `<script>`/`<style>` content, and
merges element ranges with indentation, brace and region folds. Uppercase tag
names match case-insensitively; unclosed elements do not fold.

Model tests cover nesting, quoted attributes, comments, raw scripts, uppercase
tags, voids and unclosed elements. A rendered HTML test collapses a `<div>` body
through the normal gutter control. Server-provided fold ranges remain.

Final complete gate: 416 passed, zero failures, one ignored
(`markup-folding-final-workspace-tests.txt`); Clippy/build pass
(`markup-folding-final-clippy.txt`, `markup-folding-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Slash-command argument completion (2026-10-31)

After `/command partial` the composer now detects a non-empty, whitespace-free
argument token (except compact/compress), requests workspace file matches from
the engine, prefix-ranks them case-insensitively and shows a bounded file menu.
Arrow/Enter/Tab and pointer selection insert the path plus a trailing space,
keep the command editable and clear the menu; Escape dismisses it. Stale search
responses are request-scoped, and argument state clears on send/session change.

Model tests cover trigger parsing, whitespace/name guards and path filtering.
A rendered test mounts the production menu helper, verifies rows and dispatches a
pointer selection. Runtime capability gating, quoting/escaping and visual
acceptance remain.

Final complete gate: 419 passed, zero failures, one ignored
(`slash-args-final-workspace-tests.txt`); Clippy/build pass
(`slash-args-final-clippy.txt`, `slash-args-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Cross-file go-to-definition fallback (2026-10-31)

F12 still resolves declarations in the active document first. When none exist,
the editor searches exact declaration names across every open workspace document;
a unique match emits `OpenSymbol` so the app opens/focuses that tab at the offset,
while multiple matches populate the workspace-symbol panel. The index is
open-document local and exact-name filtered, complementing Cmd+T prefix search.

Tests extend workspace-symbol exact filtering and add a rendered two-document
caller/definition flow that verifies the emitted cross-file target. Language
server semantic/type resolution remains unfinished.

Final complete gate: 420 passed, zero failures, one ignored
(`cross-file-definition-final-workspace-tests.txt`); Clippy/build pass
(`cross-file-definition-final-clippy.txt`, `cross-file-definition-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Slash-menu edge cases (2026-10-31)

Escape now dismisses the derived command and file-argument menus until the next
composer edit, using explicit `MenuDismissal` state shared by both. File-argument
completion checks the loaded command catalog before requesting paths, so unknown
commands no longer open a file menu. Catalog normalization deduplicates
case-insensitive names by builtin > runtime command > skill priority, preserving
the first entry within a priority.

Model tests cover dismissal transitions, command membership and duplicate
resolution; the production file-argument menu remains covered by its rendered
row/selection test. Quoting/escaping and visual acceptance remain.

Final complete gate: 422 passed, zero failures, one ignored
(`slash-menu-edges-final-workspace-tests.txt`); Clippy/build pass
(`slash-menu-edges-final-clippy.txt`, `slash-menu-edges-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Slash-command path escaping (2026-10-31)

Completed file arguments now escape literal spaces and backslashes when inserted
and unescape only those inserted sequences (`\ ` and `\\`) before sending the
command to the runtime. User-entered backslashes such as `\d` are preserved. A
shared insertion helper replaces the partial token, appends a trailing space and
returns the exact composer caret; the argument menu renders spaced paths as
selectable rows.

Model tests round-trip spaced/backslash paths and caret placement; the rendered
argument-menu test now includes a path containing a space. Broader runtime
capability gating, structured command arguments and visual acceptance remain.

Final complete gate: 423 passed, zero failures, one ignored
(`slash-quoting-final-workspace-tests.txt`); Clippy/build pass
(`slash-quoting-final-clippy.txt`, `slash-quoting-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded semantic hover (2026-10-31)

Pointing at an identifier now resolves a local declaration and shows an
AccessKit hover card near the pointer with declaration kind, source signature
and shallow inferred scalar type/line. Inference covers strings, numbers,
booleans, arrays, objects, functions, null and function/class/type declarations.
The popup clears over non-symbols and on keyboard/mouse interaction; it is
suppressed while completion/symbol panels are open.

Model tests cover declaration forms, type inference and unknown/whitespace
queries. A rendered editor test moves the pointer over a usage, asserts the
`answer`/`number` card, then moves off the symbol and confirms dismissal. Full
language-server semantic/type hover remains unfinished.

Final complete gate: 426 passed, zero failures, one ignored
(`hover-final-workspace-tests.txt`); Clippy/build pass
(`hover-final-clippy.txt`, `hover-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded signature help (2026-10-31)

The editor now shows signature help while the caret is inside a call to a locally
declared `function`, arrow function or function expression. Parameter names are
parsed from the declaration line, and the active parameter is the top-level
comma bucket at the caret; commas inside nested calls/arrays/objects or strings
do not advance it. The AccessKit card displays the full signature and active
parameter and disappears after the closing parenthesis. Declaration parameter
editing is excluded.

Model tests cover active-parameter tracking, nested/string commas, arrow and
function-expression declarations, unknown calls and declarations. A rendered JS
test verifies the card/active row and dismissal. Full language-server signature
and type metadata remain unfinished.

Final complete gate: 430 passed, zero failures, one ignored
(`signature-final-workspace-tests.txt`); Clippy/build pass
(`signature-final-clippy.txt`, `signature-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Workspace find-references (2026-10-31)

Shift+F12 now searches whole-word references across every open document in the
active workspace, not just the active buffer. Results are file-qualified, sorted
by directory/path/offset and bounded to 100, shown under a `References for X`
title with line previews. Arrow/Enter/mouse selection emits `OpenSymbol`; the
app opens/focuses the owning tab and places the caret at the reference, including
cross-file targets.

Model tests extend the workspace index with cross-file reference ordering and
limits. A rendered two-document test verifies the panel title/rows and that
navigating to the third result emits the other document's key/offset. Full
language-server reference semantics remain unfinished.

Final complete gate: 431 passed, zero failures, one ignored
(`workspace-references-final-workspace-tests.txt`); Clippy/build pass
(`workspace-references-final-clippy.txt`, `workspace-references-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Document occurrence highlights (2026-10-31)

The editor now highlights every whole-word occurrence of the identifier under a
collapsed caret, matching Monaco's default occurrences highlight. Matches are
source-ordered, Unicode-aware, exclude embedded identifiers and are bounded to
100 occurrences. A subtle accent background paints before find/selection colors
so stronger layers win, and the list recomputes on caret or buffer changes while
clearing whenever the primary selection is non-empty.

Model tests cover ordering, caret edges, Unicode identifiers and the limit. A
rendered editor test verifies `foo` highlights, switching the caret to `bar`, and
clearing when a range is selected. Full language-server document highlights
remain unfinished.

Final complete gate: 435 passed, zero failures, one ignored
(`occurrences-final-workspace-tests.txt`); Clippy/build pass
(`occurrences-final-clippy.txt`, `occurrences-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Provider credential form controls (2026-10-31)

Provider key forms now expose native numeric and integer steppers beside their
text inputs. One click follows HTML's step base (minimum, then default, then
zero) and unit step, keeps integer values integral, clamps to configured
minimum/maximum and disables at the bounds. Numeric answers normalize integral
values so JSON serializes `2` instead of `2.0`.

Validation now anchors a per-field bubble beneath the invalid control, keeps
focus/scroll reveal and clears on the next edit; service errors remain
form-level. The API-key input already uses the secure password path (masked
rendering, blocked copy/cut, password accessibility role). Model tests cover
step bases, clamping and invalid input; a rendered test verifies key/number
bubbles, stepper clicks, bound disabling and successful submission. Complete
select-menu behavior, live usage refresh and visual acceptance remain.

Final complete gate: 437 passed, zero failures, one ignored
(`provider-form-final-workspace-tests.txt`); Clippy/build pass
(`provider-form-final-clippy.txt`, `provider-form-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Provider select-menu behavior (2026-10-31)

Single-select option menus now render the same `Select` placeholder as the AppKit
menu plus the current option, highlight the keyboard cursor and support
arrows/Home/End (with Cmd/Ctrl bounds), Enter/Space acceptance, Escape dismissal
and pointer selection.

Multi-select menus now match native behavior: Space toggles the focused option
without moving, plain arrows/Home/End move focus without changing the selection,
Shift extends a contiguous range from the anchor, Cmd/Ctrl+click toggles
additively, Cmd/Ctrl+A selects all and Cmd/Ctrl+Home/End jumps to the bounds.
The inline menu no longer uses a deferred element, avoiding stale arena bounds
when it closes and reopens.

Model tests cover focus-only movement and shift anchoring. A rendered test
verifies the single placeholder/current highlight, keyboard acceptance and
clearing, then multi-select Space toggling, focus movement, Shift ranges,
Cmd+End/Space and Cmd+A. Live usage refresh and visual acceptance remain.
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

Final complete gate: 439 passed, zero failures, one ignored
(`provider-select-final-workspace-tests.txt`); Clippy/build pass
(`provider-select-final-clippy.txt`, `provider-select-final-build.txt`).

## Settings runtime handoff (2026-10-31)

Native preferences now use an engine-owned handoff instead of the UI writing
`settings.json` directly. `Engine::apply_settings` normalizes the snapshot,
updates the engine's in-memory `Settings`, resizes live warm-cache count/byte
limits and persists atomically on a blocking task. Save failures emit an event
that the app surfaces as a toast; `Engine::settings()` returns the shared
snapshot rather than re-reading disk.

`WarmCache::set_limits` evicts only when the new budgets require it, retaining
one oversized entry as before. Engine-flow tests verify persistence, in-memory
visibility, normalized invalid limits and live `warm_limits()`. A rendered app
test changes the shell through the settings path and waits for the engine
snapshot to report `classic`. Mobile pairing and live settings-page acceptance
remain.

Final complete gate: 442 passed, zero failures, one ignored
(`settings-handoff-final-workspace-tests.txt`); Clippy/build pass
(`settings-handoff-final-clippy.txt`, `settings-handoff-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded workspace rename (2026-10-31)

F2 now opens an inline rename prompt for the identifier at the caret. Whole-word
occurrences are collected from every open workspace document and bounded to
1000; identifier validation rejects empty, invalid and unchanged names, and
oversized/no-symbol plans report without editing.

Confirming re-plans against current buffers, then applies `replace_many` once per
document so each document is a single undo step. The active document keeps the
caret at the replacement matching the original caret; other open editors sync on
their next render. Escape or the close control cancels. Model tests cover
identifier rules, cross-file ranges, limits and error cases; a rendered
two-document test verifies invalid feedback, cross-document rename, caret
placement and local undo semantics. Full language-server rename remains
unfinished.

Final complete gate: 446 passed, zero failures, one ignored
(`rename-final-workspace-tests.txt`); Clippy/build pass
(`rename-final-clippy.txt`, `rename-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded TypeScript literal type diagnostics (2026-10-31)

TypeScript documents now receive a bounded semantic type check on annotated
primitive literals. `const`/`let`/`var` declarations compare their literal
initializer against string/number/boolean/null, `any`/`unknown`, union members,
array element types (`T[]`/`Array<T>`) and conservative object/function
annotations. Comments are stripped quote-aware; non-literal expressions and
unverifiable annotations are left alone.

Mismatches feed the existing red gutter marker and accessible banner with a
one-based literal column and message, clearing when the value is corrected.
Model tests cover unions, arrays, objects, comments, nested indentation and
non-literal expressions; a rendered TypeScript test verifies the marker/banner
and clearing. Full language-server type checking remains unfinished.

Final complete gate: 449 passed, zero failures, one ignored
(`semantic-types-final-workspace-tests.txt`); Clippy/build pass
(`semantic-types-final-clippy.txt`, `semantic-types-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded duplicate-declaration diagnostics (2026-10-31)

JavaScript and TypeScript documents now report duplicate top-level `const`,
`let`, `class` and `type` declarations at the later declaration, including the
first declaration line. `var`, function overloads, enums/interfaces (legal
merging), ambient `declare` statements and indented/nested scopes are excluded
to avoid false positives. A quote-aware mask removes line and block comments
while preserving string contents, newlines and byte offsets; Unicode names are
supported.

Diagnostics reuse the red marker and accessible banner and clear when the
duplicate is renamed. Model tests cover the supported declarations, every
exclusion, comments and Unicode; a rendered JavaScript test verifies the marker
and banner, then clears after renaming. Full language-server scope/type
analysis remains unfinished.

Final complete gate: 451 passed, zero failures, one ignored
(`duplicate-declarations-final-workspace-tests.txt`); Clippy/build pass
(`duplicate-declarations-final-clippy.txt`, `duplicate-declarations-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded unused-declaration diagnostics (2026-10-31)

JavaScript/TypeScript documents now report `const`/`let` declarations with no
whole-word use outside comments. Any other occurrence—including writes, strings
and JSX—counts as a use to remain conservative. Exported and underscore-prefixed
names, destructuring, multiple declarators, `for` heads and `.d.ts` declaration
files are excluded.

Diagnostics use the existing red marker and accessible banner and clear once the
value is used. Model tests cover the supported local/top-level forms, all
exclusions, comments, strings and extension routing; a rendered JavaScript test
verifies marker/banner creation and clearing after adding a use.

Final complete gate: 454 passed, zero failures, one ignored
(`unused-declarations-final-workspace-tests.txt`); Clippy/build pass
(`unused-declarations-final-clippy.txt`, `unused-declarations-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded unused-parameter diagnostics (2026-10-31)

JavaScript/TypeScript now report unused simple parameters on function
declarations, function expressions and arrow functions. Parameter lists handle
defaults, rest parameters and generic commas; destructured, `this` and
underscore-prefixed parameters are excluded, and any whole-word use in the
document (including template strings) counts. Signature-only declarations and
`.d.ts` files are skipped.

Diagnostics use the existing red marker and accessible banner and clear when the
parameter is used. Model tests cover functions, arrows, defaults, rest,
generics, overload signatures, comments and Unicode; a rendered JavaScript test
verifies marker/banner creation and clearing after using the parameter.

Final complete gate: 456 passed, zero failures, one ignored
(`unused-parameters-final-workspace-tests.txt`); Clippy/build pass
(`unused-parameters-final-clippy.txt`, `unused-parameters-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded quick fixes (2026-10-31)

Cmd+. now opens a Quick Fixes menu for the caret line's local diagnostics.
Available actions are intentionally safe: remove a single-line unused
`const`/`let` declaration (refused for multi-declarator, multi-line or exported
lines) and prefix an unused simple parameter with `_`. Arrow/Enter/Tab or a
pointer click applies the action through the normal editor undo transaction and
the diagnostic clears; Escape or any other key dismisses the menu.

Model tests cover action derivation, newline-inclusive removal, last-line
ranges, parameter identifier ranges, line ownership and refused/unsupported
cases. A rendered test applies both actions and verifies the buffer and cleared
diagnostics. Latest complete gate459 with Clippy/build pass
(`quick-fixes-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded missing-return diagnostics (2026-10-31)

TypeScript function declarations with a non-void return annotation now report a
body containing no `return` or `throw`. Overload signatures, generators,
object-literal return types, void-like/`never`/`any`/`unknown`, Promise<void>
and undefined unions, and unclosed bodies are excluded. Comments are masked;
nested returns count conservatively to avoid false positives.

Diagnostics reuse the red marker and accessible banner and clear once a return
is added. Model tests cover successful returns, throws, multi-line bodies,
overloads, generators, unions, unmasked comments and unclosed functions; a
rendered TypeScript test verifies marker/banner creation and clearing.

Final complete gate: 461 passed, zero failures, one ignored
(`missing-return-final-workspace-tests.txt`); Clippy/build pass
(`missing-return-final-clippy.txt`, `missing-return-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Bounded unused-import diagnostics (2026-10-31)

JavaScript/TypeScript single-line imports now report unused default, named,
aliased, type-only and namespace bindings. Side-effect imports, underscore
names, multi-line clauses and comments are excluded; any whole-word use in the
document—including type positions, re-exports and strings—counts as a use.
Clause parsing handles `import type`, `type` specifiers and `as` aliases.

Diagnostics reuse the red marker and accessible banner and clear when the import
is used. Model tests cover every binding form, aliases, comments, multiline
imports and Unicode; a rendered JavaScript test verifies marker/banner creation
and clearing after use.

Final complete gate: 463 passed, zero failures, one ignored
(`unused-imports-final-workspace-tests.txt`); Clippy/build pass
(`unused-imports-final-clippy.txt`, `unused-imports-final-build.txt`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Unused-import quick fix (2026-10-31)

Unused-import diagnostics now offer a Quick Fix. When an import line has exactly
one binding (default, namespace, type-only or a single named specifier), Cmd+.
offers "Remove unused import" and deletes the whole line including its newline.
Multi-binding imports are intentionally left without an action so comma/list
rewriting cannot corrupt the clause.

Model tests cover single-binding removal, multi-binding refusal and
column/line ownership. The rendered quick-fix test now removes an unused import,
verifies the resulting buffer and confirms the diagnostic banner clears.
Latest complete gate464 with Clippy/build pass (`import-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Multi-binding import quick fix (2026-10-31)

Unused named imports in multi-binding clauses now offer safe specifier removal.
Quick Fixes removes the specifier plus the correct adjacent comma for first,
middle and last positions, preserving the rest of the clause and its spacing.
Default-plus-named clauses and clauses containing comments are deliberately left
unmodified to avoid corrupting the import.

Model tests verify the resulting text for first/middle/last removal and refusal
cases. The rendered quick-fix test now removes one named binding from a
two-binding import and confirms the surviving import and cleared diagnostic.
Latest complete gate464 with Clippy/build pass
(`multi-import-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Default-plus-named import quick fix (2026-10-31)

Combined default-plus-named imports now support safe Quick Fix removal. An
unused named binding removes its specifier with the correct comma while keeping
the default; removing the only named binding drops the braces and preceding
comma; an unused default removes its comma and following space while preserving
`import type`. Commented clauses are still untouched.

Model tests verify default+named, only-named, unused-default and type-only
results. The rendered quick-fix flow removes an unused default and confirms the
surviving named import and cleared diagnostic. Latest complete gate464 with
Clippy/build pass (`default-import-quick-fix-final-*`).

Full-suite runs also exposed load-dependent delivery timeouts in the native
watcher test and the engine-flow Sass waits; both isolated cases passed. Their
waits were widened from 8s to 20s so slow FSEvents/Dart Sass delivery is not
mistaken for a lost event.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Default-plus-namespace import quick fix (2026-10-31)

Default-plus-namespace imports (`import Foo, * as ns from`) now parse both
bindings instead of treating the namespace as part of the default clause.
Quick Fixes removes an unused namespace together with its comma while keeping
the default, and removes an unused default together with its comma while keeping
`* as ns`. `import type` prefixes are preserved.

Model tests cover both binding diagnostics and both removal directions. A
rendered quick-fix test removes an unused namespace, confirms the surviving
default import and verifies the diagnostic clears. Latest complete gate464 with
Clippy/build pass (`namespace-import-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Multi-line import quick fix (2026-10-31)

Multi-line named import clauses are now parsed for unused-import diagnostics.
Each specifier reports its own line and column, including default-plus-multiline
named and `import type` forms. Comments remain masked and underscore-prefixed
names are ignored.

Quick Fixes removes a specifier's whole line, with or without a trailing comma,
leaving valid empty braces when the last binding is removed. Specifiers sharing
a line are diagnosed but deliberately not auto-removed. Model tests cover the
diagnostics and removal; a rendered test removes a multi-line specifier and
confirms the surviving clause and cleared banner. Latest complete gate465 with
Clippy/build pass (`multiline-import-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Shared-line import quick fix (2026-10-31)

Multi-line import entries that share a physical line now support precise Quick
Fix removal. The first, middle and last specifiers remove the correct adjacent
comma while preserving indentation and any trailing comma; a single-specifier
line still removes the whole line. Commented lines remain declined.

Model tests cover first, middle and last shared entries plus no-trailing-comma
cases. The rendered quick-fix flow removes a shared-line specifier and confirms
the surviving entry and cleared banner. Latest complete gate465 with Clippy/build
pass (`shared-import-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Next-line brace import quick fix (2026-10-31)

Imports whose opening brace is on a following line (`import` / `import D,` then
`{`) are now parsed for unused-import diagnostics. Default-plus-named and
`import type` forms are supported; unrelated braces, missing `from`, and
statements without a brace are ignored.

Quick Fixes removes whole or shared specifier lines and can remove the default
while preserving the braced list. Model tests cover diagnostics and
default/named removals; the rendered flow removes a specifier from a
brace-next-line import, verifies the surviving clause and clears the banner.
Latest complete gate465 with Clippy/build pass
(`next-line-brace-import-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## From-on-later-line import quick fix (2026-10-31)

Imports with `from` on a later line are now parsed for unused-import diagnostics
and Quick Fix removal, including `from` followed by a newline before the module
string. Same-line braces with a later `from`, braced defaults and imports later
in the file are covered; statement slicing keeps Quick Fix edits scoped to the
import.

Model tests cover diagnostics, named/default removal and a non-leading import;
the rendered flow removes a specifier where `from` is on the next line and
verifies the surviving clause and cleared banner. Latest complete gate466 with
Clippy/build pass (`from-next-line-import-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Runtime capability gating (2026-10-31)

The engine now reports an explicit OpenCode capability manifest alongside the
CLI version: attachments, commands, models, agents, permissions, provider
credentials, session fork/resume, steering (disabled for OpenCode) and TUI
(version-dependent). The app stores the flags from runtime-info.

Settings > Providers shows a managed-by-runtime callout and skips provider forms
when credential editing is unsupported. The Model safety section hides
permission and steering rows when their capabilities are false and renders a
Queue/Steer preference when steering is available. Model tests cover the manifest
flags; a rendered app test switches capabilities and verifies provider callout,
permission-row and steering-row visibility.

Latest complete gate468 with Clippy/build pass (`runtime-capabilities-final-*`).
No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate declaration quick fix (2026-10-31)

Duplicate top-level declaration diagnostics now offer a Quick Fix that renames
the later declaration to the first unused `name2`, `name3`, … identifier. Only
the duplicate declaration identifier is edited; references are intentionally
left untouched. Non-declaration contexts, mismatched lines and already-occupied
candidates are handled conservatively.

Model tests verify the range, replacement, collision skipping and refusals. A
rendered test renames an exported duplicate and confirms the diagnostic clears.
Latest complete gate470 with Clippy/build pass
(`duplicate-rename-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Type-mismatch quick fix (2026-10-31)

Type-mismatch diagnostics now offer two Quick Fixes: change a simple
primitive/union annotation to the literal's inferred primitive type, or remove
the type annotation entirely. Change-type is limited to
string/number/boolean/null literals and simple annotations; complex annotations
still offer removal. Only the annotation is edited, preserving declaration
spacing and the initializer.

Model tests verify both edits, union handling, complex annotations and invalid
positions. A rendered TypeScript test applies change-type and then removal,
confirming the diagnostic banner clears after each. Latest complete gate472 with
Clippy/build pass (`type-mismatch-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Missing-return quick fix (2026-10-31)

Missing-return diagnostics now offer two Quick Fixes: change the return type to
`void` (or `Promise<void>` for async functions), or remove the return type
annotation entirely. Only the annotation is edited, leaving the body and
initializer intact.

Model tests cover sync/async declarations and invalid positions. A rendered
TypeScript test applies change-type and then removal, verifying the diagnostic
banner clears after each. Latest complete gate474 with Clippy/build pass
(`missing-return-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Capability-gated composer and TUI affordances (2026-10-31)

Runtime capabilities now gate the composer's attach button, slash-command menu,
agent picker and the Agent TUI control. Each renders only when its advertised
capability is true; the slash menu also omits commands from its query when
commands are disabled.

The rendered app test keeps the terminal tray open, switches all four flags, and
verifies the controls appear with capabilities enabled and disappear when
disabled. Latest complete gate474 with Clippy/build pass
(`capability-affordances-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Syntax quick fixes (2026-10-31)

Syntax diagnostics now offer bounded Quick Fixes: append the missing closer for
unclosed braces, brackets and parentheses; remove an unexpected closing
delimiter; add a closing quote for unterminated strings; and close unterminated
block comments. Applied edits use the normal undo transaction.

Model tests cover each message and invalid positions. A rendered JavaScript test
adds a missing brace and then removes an unexpected one, verifying the banner
clears after each. Latest complete gate476 with Clippy/build pass
(`syntax-quick-fix-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Function-call argument diagnostics and quick fix (2026-10-31)

Locally declared top-level functions now report calls with too few or too many
arguments. One-line parameter lists define required/optional/default/rest
counts; comments are masked and nested commas/strings are ignored. Member calls
and duplicate declarations are excluded.

Quick Fixes insert missing `undefined` arguments or remove extra arguments with
exact spans. Model tests cover count ranges and exclusions; a rendered JavaScript
test fills one missing argument and trims two extra arguments, clearing the
banner each time. Latest complete gate479 with Clippy/build pass
(`argument-count-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Auto-import quick fix (2026-10-31)

Uppercase identifiers with no local declaration/import and exactly one matching
exported declaration in another open document now report "Cannot find name".
The Quick Fix inserts a named or default import using a computed relative path;
strings, comments, member accesses, object keys, ambiguous exports and
local/imported names are excluded.

Model tests cover resolution, exclusions, ambiguity and relative paths; a
rendered two-document test inserts an auto-import and clears the diagnostic.
Latest complete gate483 with Clippy/build pass (`auto-import-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Auto-import preamble and cross-document refresh (2026-10-31)

Auto-import insertion now skips a shebang and one `"use strict"`/`'use strict'`
directive, keeping the preamble intact. Editor diagnostics also fingerprint the
buffer revisions of every open workspace document and refresh auto-import
diagnostics when another document changes, without requiring an edit to the
active buffer.

Model tests cover insertion offsets. A rendered test imports after `"use strict"`
and observes the unresolved diagnostic appear/disappear as another document's
export is removed and restored. Latest complete gate485 with Clippy/build pass
(`auto-import-refresh-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Lowercase function auto-import (2026-10-31)

Lowercase identifiers called as functions now auto-import when exactly one other
open document exports them as a function. Non-call references, member calls,
arrow-function const exports and common globals remain excluded; uppercase
imports keep their broader behavior. Named and default function exports use the
appropriate import form.

Model tests cover named/default function exports and all exclusions; a rendered
test inserts a lowercase named import and clears the diagnostic. Latest complete
gate487 with Clippy/build pass (`lowercase-auto-import-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Lowercase arrow-export auto-import (2026-10-31)

Lowercase arrow and function-expression exports now count as function exports
for auto-import: `export const helper = () => ...`, async arrows,
`export const helper = function () {}` and async function expressions. Plain
values and conditional expressions are excluded, and uppercase behavior is
unchanged.

Model tests cover every arrow/function-expression form plus non-function
exclusions; a rendered two-document test imports an arrow-exported helper and
confirms the diagnostic clears. Latest complete gate488 with Clippy/build pass
(`arrow-auto-import-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate object-key diagnostics and quick fix (2026-10-31)

Object literals now report duplicate plain keys at the later key, including the
first key's line. Identifier and quoted keys are compared; computed keys,
spreads, methods, accessors, class/function blocks and non-object braces are
excluded. The check is bounded to object candidates in expression position and
reasonably sized bodies.

A safe Quick Fix removes the earlier duplicate when its value is a primitive
literal, preserving the last-wins object result; side-effecting or referenced
values are diagnosed without an action. Model tests cover detection, exclusions
and removal spans; a rendered JavaScript test removes the earlier duplicate and
clears the marker/banner. Latest complete gate496 with Clippy/build pass
(`object-keys-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate parameter diagnostics and quick fix (2026-10-31)

Duplicate simple parameter names in one-line function/arrow lists now report at
the later parameter. The Quick Fix prefixes the earlier shadowed parameter with
`_` when that name is unused elsewhere; this preserves arity and body resolution
because the later duplicate binding already wins. Destructured, `this`,
multiline and already-underscore-prefixed parameters are excluded.

Model tests cover offsets, defaults, rest, arrows, Unicode and exclusions; a
rendered JavaScript test applies the rename and clears the marker/banner. A
full-suite load timeout in the watcher-readiness test (isolated pass) was
addressed by widening that wait from 15s to 30s. Latest complete gate500 with
Clippy/build pass (`duplicate-parameters-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate import-specifier diagnostics and quick fix (2026-10-31)

Single-line ES imports now report duplicate local bindings within one import
statement. Identical named specifiers (`a`/`a`, `a as b`/`a as b`, duplicate
`type` specifiers) offer removal of the later redundant specifier with correct
first/middle/last comma handling. Default/namespace collisions and aliases that
bind different exports are reported without an action; type-only and value
bindings of the same name are kept distinct.

Model tests cover detection, removal spans, aliases, Unicode, comments and
bounded exclusions; a rendered test removes the redundant specifier and clears
the marker/banner. Latest complete gate507 with Clippy/build pass
(`duplicate-imports-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate export-specifier diagnostics and quick fix (2026-10-31)

Single-line named exports now report duplicate exported names within one
statement, including re-exports. Identical specifiers (`a`/`a`, `a as b`/`a as
b`, duplicate `type` specifiers) offer removal of the later redundant specifier
with first/middle/last comma handling. Aliases exporting the same name from
different locals are reported without an action; type-only and value exports
stay distinct.

Model tests cover removal spans, aliases, re-exports, Unicode, comments and
bounded exclusions; a rendered test removes the redundant specifier and clears
the marker/banner. Latest complete gate514 with Clippy/build pass
(`duplicate-exports-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate destructuring-binding diagnostics and quick fix (2026-10-31)

Single-line object-destructuring declarations (`const`/`let`/`var`) now report
duplicate local bindings, including exported patterns. Identical shorthand or
aliased properties offer removal of the later redundant property with
first/middle/last comma handling. Different keys, defaults, computed keys, rest
elements and nested patterns are reported conservatively or skipped.

Model tests cover removal spans, aliases, quoted keys, defaults and bounded
exclusions; a rendered test removes the redundant binding and clears the
marker/banner. Latest complete gate520 with Clippy/build pass
(`destructuring-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate class-member diagnostics and quick fix (2026-10-31)

Single-line class declarations now report duplicate members. Accessor pairs,
static/instance pairs and method/field pairs are kept distinct. Safe removal
drops the earlier method/accessor, or an earlier field whose initializer is a
primitive literal; constructors, private members and side-effecting field
initializers are diagnosed without an edit. Multiline classes, class
expressions, computed names, decorators and static blocks are excluded.

Model tests cover removal spans and exclusions; a rendered test removes the
earlier method and clears the marker/banner. Latest complete gate527 with
Clippy/build pass (`class-members-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Single-line class-expression member diagnostics (2026-10-31)

Single-line class expressions (`const A = class { … }`, named and parenthesized
forms, arrow-returned classes) now receive the same duplicate-member diagnostics
and safe removal as class declarations. Strings that merely contain `class` are
ignored via structural masking.

Model tests cover expression forms and false positives; a rendered test removes
the earlier method in a class expression and clears the marker/banner. Latest
complete gate529 with Clippy/build pass (`class-expression-members-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Multi-line class-member diagnostics (2026-10-31)

Multi-line class declarations and expressions now use the same duplicate-member
checks and safe removal as single-line classes. Earlier one-per-line members are
removed with their indentation and newline; shared-line members keep span
removal. Field parsing supports semicolon-terminated multi-line fields, with
side-effecting initializers still diagnostic-only.

Model tests cover multiline methods, fields, class expressions, accessor/static
pairs and unsafe cases; a rendered test removes an earlier method line and
clears the marker/banner. Latest complete gate532 with Clippy/build pass
(`multiline-class-members-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Quoted export-name duplicates (2026-10-31)

Duplicate export-name checks now cover quoted arbitrary module-export names
(`export { a as "read" }`), single- or double-quoted, including re-exports.
Quoted and identifier forms of the same name are compared together; identical
local bindings remove the later specifier, while different locals remain
diagnostic-only. Escaped string interiors stay bounded (raw interiors compared).

Model tests cover quoted duplicates, mixed quote styles, re-exports and
type-only separation; a rendered test removes a redundant quoted specifier and
clears the marker/banner. Latest complete gate535 with Clippy/build pass
(`quoted-export-names-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Quoted import-name duplicates (2026-10-31)

Duplicate import-binding checks now parse quoted imported names
(`import { "read" as read }`), single- or double-quoted, including mixed quote
styles and Unicode. Two specifiers binding the same local from the same quoted
name remove the later redundant specifier; different quoted sources stay
diagnostic-only, and type-only/value bindings remain distinct. Quoted imports
without a local alias are treated as invalid and skipped.

Model tests cover each form; a rendered test removes the redundant quoted
specifier and clears the marker/banner. Latest complete gate538 with
Clippy/build pass (`quoted-import-names-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate switch-case diagnostics (2026-10-31)

Switch statements now report duplicate literal case labels and duplicate
`default` clauses, including nested switches. Number/string/boolean/null/
undefined/identifier labels compare by value; complex expressions are skipped.

A safe Quick Fix removes a later duplicate label only when its clause is empty
(immediately followed by another label or the closing brace), so fallthrough and
statement execution are unchanged; non-empty duplicates stay diagnostic-only.
Model tests cover empty/non-empty clauses, defaults, nested switches,
strings/comments and complex labels; a rendered test removes an empty duplicate
case and clears the marker/banner. Latest complete gate546 with Clippy/build
pass (`switch-cases-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate enum-member diagnostics (2026-10-31)

TypeScript enums now report duplicate member names (identifier, quoted and
mixed forms) and duplicate numeric/string values, including auto-numbered
collisions following TypeScript's previous-value-plus-one rule.

A safe Quick Fix removes a later duplicate name only when both members carry the
same explicit literal initializer, preserving the name, value and subsequent
numbering; value aliases, differing values and computed initializers stay
diagnostic-only. Model tests cover names/values, auto/hex/string literals,
prefixes, comments and multiline/shared lines; a rendered test removes the later
duplicate and clears the marker/banner. Latest complete gate553 with Clippy/build
pass (`enum-members-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate interface-property diagnostics (2026-10-31)

TypeScript interface bodies now report duplicate property signatures, including
quoted names, multiline interfaces and exported/declared forms. Method overloads
remain legal and are skipped; property/method conflicts are reported.

A safe Quick Fix removes the later duplicate only when both declarations are
identical (readonly/optional flags and type text), preserving the interface
shape; differing types/modifiers stay diagnostic-only. Model tests cover
identical/differing members, overloads, prefixes and bounded exclusions; a
rendered test removes the duplicate property and clears the marker/banner.
Latest complete gate560 with Clippy/build pass (`interface-members-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate union-literal diagnostics (2026-10-31)

TypeScript union type aliases now report duplicate literal members
(string/number/boolean/null/undefined), including multiline leading-pipe unions
and `export`/`declare` aliases. Non-literal type references and complex members
are skipped.

Removing a duplicate union literal is always safe; the Quick Fix handles inline
first/middle/last positions and whole multiline literal lines. Model tests cover
values, quote styles, multiline bounds and exclusions; a rendered test removes
the later literal and clears the marker/banner. Latest complete gate567 with
Clippy/build pass (`union-members-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate inline object-type property diagnostics (2026-10-31)

Inline TypeScript object type literals (`type Name = { ... }`) now share the
interface member analysis: duplicate property signatures report at the later
member, method overloads are skipped and property/method conflicts are reported.

A safe Quick Fix removes a later duplicate only when readonly/optional flags and
type text are identical; differing declarations stay diagnostic-only. Model tests
cover identical/differing properties, quoted names, nested types, overloads and
bounded exclusions; a rendered test removes the duplicate property and clears the
marker/banner. Latest complete gate574 with Clippy/build pass
(`type-literal-properties-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Annotation object-type property diagnostics (2026-10-31)

Inline object type literals in annotation position (`const options: { … } =`)
now receive the same duplicate-property analysis. Variable declarations with a
simple annotated name are checked inline and across lines; exported declarations
are included. Object-literal initializers, ternaries, destructuring and `for`
bindings are excluded.

Model tests cover annotation forms and exclusions; a rendered test removes the
duplicate and clears the marker/banner. Latest complete gate577 with Clippy/build
pass (`annotation-type-properties-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Signature object-type property diagnostics (2026-10-31)

Inline object type literals in function and arrow signatures now receive the same
duplicate-property analysis: parameter annotations (including function
expressions) and return annotations. Default initializers, ternaries and
destructured annotations are excluded.

Model tests cover parameters, returns, arrows and exclusions; a rendered test
removes a duplicate parameter type property and clears the marker/banner. Latest
complete gate581 with Clippy/build pass (`signature-type-properties-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate generic type-parameter diagnostics (2026-11-01)

TypeScript generic type-parameter lists on `function`/`type`/`interface`/`class`
declarations now report duplicate names. A safe Quick Fix removes the later
duplicate only when both entries are plain identifiers (no constraint or
default), preserving name resolution; constrained/defaulted duplicates stay
diagnostic-only. Nested generics and arrow constraints are parsed without
confusing `=>` for a closing bracket.

Model tests cover declarations, multiline/shared-line removal,
constraints/defaults and string/comment bounds; a rendered test removes the
duplicate and clears the marker/banner. Latest complete gate588 with Clippy/build
pass (`type-parameters-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Arrow and method generic type-parameter diagnostics (2026-11-01)

Generic type-parameter checks now cover arrow functions and class/object methods
in addition to declarations. Arrow generics are found from `=>` back through the
parameter list and angle list; method generics require a method name plus a
direct body block, avoiding declaration lists and comparisons. The same
plain-only removal rule applies; constrained/defaulted duplicates stay
diagnostic-only.

Model tests cover arrows, async arrows, multiline lists, class/object/static
methods and exclusions; a rendered arrow test removes the duplicate and clears
the marker/banner. Latest complete gate591 with Clippy/build pass
(`method-type-parameters-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Return-typed method and anonymous function generics (2026-11-01)

Generic type-parameter checks now include return-typed class/object methods and
anonymous generic function expressions. Return annotations are skipped
conservatively when locating the method body; anonymous functions are parsed when
`<` follows the `function` keyword and are excluded from the method scan to
avoid double reports.

Model tests cover returned/object-return methods, anonymous functions and
exclusions; a rendered test removes the anonymous function duplicate and clears
the marker/banner. Latest complete gate593 with Clippy/build pass
(`returned-method-type-parameters-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Nested object-type property diagnostics (2026-11-01)

Nested inline object type literals now recurse through the shared type-member
analysis (bounded to eight levels). Duplicate properties inside nested properties
are reported with nested offsets and can use the same identical-member Quick Fix.
Interfaces and annotated/signature type literals share the recursion.

Model tests cover alias/annotation nesting, three levels, multiline removal and
differing-type refusal; a rendered test removes a nested duplicate and clears the
marker/banner. Latest complete gate596 with Clippy/build pass
(`nested-type-properties-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Method parameter duplicate/unused diagnostics (2026-11-01)

Duplicate-parameter and unused-parameter analysis now includes class and object
method parameter lists (including static/async/return-typed methods and function
expressions), found through member-context heuristics that avoid calls and
control-flow parentheses. The existing shadow-safe `_name` Quick Fix applies; a
setter accessor fixture now uses `_v` to stay intentionally clean.

Model tests cover methods/function expressions and exclusions; a rendered test
applies the method-parameter fix and clears the marker/banner. Latest complete
gate598 with Clippy/build pass (`method-parameter-duplicates-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Matching-bracket highlighting (2026-11-01)

The editor now highlights the matching bracket for the caret's `()`, `[]` or
`{}` pair, including mixed nesting and multiline pairs. Brackets inside strings
and comments are ignored via masking; hovering just after an opening or on a
closing bracket resolves the same pair. The subtle accent highlight paints
beneath find/selection layers.

Model tests cover nesting, caret edges, strings/comments, unbalanced input and
escaped quotes; a rendered test moves the caret across
bracket/comment/whitespace positions and verifies state. Latest complete gate603
with Clippy/build pass (`bracket-highlight-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Delegated-agent transcript cards (2026-11-01)

Synthetic transcript records now match the current Source flow. `<subagent>`
tags with explicit ids reopen the child even when the session inventory is
stale; legacy `agent=`/`prompt=` notices match a child under the active parent
by description and agent, preferring the latest `updatedAt`. Resolvable records
render as a clickable delegated-agent card showing the configured agent name and
description; a matching generic task-tool row is collapsed so the dispatch has
one card. Internal `<system-reminder>` scaffolding is no longer projected as
transcript content.

Eleven tests were added: tag/legacy parsing, explicit-id precedence, legacy
child filtering, card title/agent resolution, unresolved-record rejection,
internal-reminder detection, dispatch-signature matching between tool inputs and
completion tags, projection-level reminder hiding and task-row collapse, and a
rendered app test that clicks the card and verifies the child panel opens.
Latest complete gate614 with Clippy/build pass (`subagent-links-final-*`).

Source re-audit corrections: current `FormPrompt` has no `when`, hidden,
constraint or custom-option fields; there is no session-fork control; session
rename only updates a stored title from `session.renamed`; and current
`useAgentScroll` has no scroll-to-bottom button. Those matrix claims were
stale and have been corrected rather than implemented as invented features.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Task-tool delegated-agent cards (2026-11-01)

`task`/`subagent` tool parts now use the same delegated-agent card as synthetic
records. Core `ToolPart`/`Block` carry a child session read from tool-state
metadata `sessionId`/`sessionID` (including `session.tool.progress` metadata);
the card falls back to the existing parent/description/agent child matching when
metadata is absent. Repeated snapshots merge into the first card, and a later
`<subagent>` synthetic completion settles its status and is absorbed instead of
rendering a second entry. Unresolved task cards remain visible but have no click
action; resolved cards reopen the child session.

`ToolPart` is boxed inside `PartKind` after the added field pushed the enum over
Clippy's large-variant threshold. Eight tests were added or replaced: metadata
replay/progress propagation, tool-card resolution and tone mapping, unresolved
card behavior, snapshot merging, synthetic settlement/absorption, reminder
hiding, and a rendered app test that clicks both unresolved and resolved cards.

Clippy and the app build pass. The workspace suite reports 620 passed, zero
failures and one optional perf test ignored with
`end_to_end_replay_stream_prompt_and_permission` skipped because this machine's
macOS FSEvents service is stalled: watcher start/stop blocks in
`FSEventStreamStart`/`FSEventStreamStop` and the isolated test times out
(`task-tool-cards-fsevents-blocker.txt`). A post-restart full-suite rerun is
required to restore the 621-test gate.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Post-restart gate restoration (2026-11-01)

After the host restart, the isolated engine-flow test and the full workspace suite
pass again. Full gate: **621 passed, zero failures, one optional performance test
ignored** (`task-tool-cards-post-restart-workspace-tests.txt`); Clippy and app
build exit 0 (`task-tool-cards-post-restart-clippy.txt`,
`task-tool-cards-post-restart-build.txt`). The earlier FSEvents failure is
retained as environment evidence; no code workaround was added for it.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Source-shaped retry notice (2026-11-01)

Retry rows now match the current Source presentation instead of a single
`message · attempt` label: an animated 11px spinner sits beside a 12px/18px
message truncated to 80 characters and an 11px/16px faint `Retrying · attempt
N` line. Missing message or attempt text falls back to `Retrying`, and the pair
carries one accessibility label. Empty/omitted details no longer leave a stray
separator.

Model tests cover truncation, fallbacks and the projected status row; a rendered
app test checks the spinner/message/info selectors. Latest complete gate624 with
Clippy/build pass (`retry-notice-final-*`). Retry settlement and live/visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Live compaction lifecycle (2026-11-01)

Native transcripts now parse `session.compaction.started/delta/ended/failed`
instead of treating compaction as replay-only text. A started event creates one
compaction message; deltas append its summary; ended replaces the summary when
provided and marks a completion time; failed preserves the summary and records
the formatted error. Running compaction messages survive authoritative history
hydration, and replay compaction records parse as completed.

Assembly now emits a dedicated `Compaction` block with running/completed/failed
status and error output. The transcript renders Source's divider labels
(`Compacting session`, `Session compacted`, `Compaction failed`), a markdown
summary and a red failure note. Event parsing, live aggregation, failure,
replay, hydration, status projection and rendered selector tests were added.

Latest complete gate631 with Clippy/build pass (`compaction-final-*`). Running
text shimmer and live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Skill activation transcript events (2026-11-01)

`session.skill.activated` now parses into a native event carrying the skill id,
display name and markdown text, with id/`Skill` fallbacks. Live activations
create a dedicated `Skill` message/block; replay `skill` records preserve the
skill id, name, text and completion time. Live skill records survive
authoritative history hydration. The transcript renders one session-event row
with Source's `Skill activated · name` label, a mono skill id detail and the
markdown body. That row is included in the event projection rather than the
assistant process group.

Four core event/live/replay/hydration tests plus projection and rendered app
selector tests were added. Latest complete gate637 with Clippy/build pass
(`skill-transcript-final-*`). Live/visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Running-text shimmer (2026-11-01)

Running tool summaries and the `Compacting session` compaction label now use an
animated character-band shimmer. The implementation reproduces Source's 1200ms
muted-to-bright sweep with staggered per-character color animation; GPUI has no
gradient text fill. Labels longer than 80 characters use a single whole-text
pulse to keep element counts bounded. Inactive labels stay static, and GPUI's
reduced-motion handling renders the repeating animation's start state.

Two model color/spread tests and a rendered app test verify the tool and
compaction selectors appear only while running and disappear once statuses
settle. Latest complete gate640 with Clippy/build pass (`shimmer-final-*`).
Live and reduced-motion acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## PTY burst coalescing (2026-11-01)

The terminal relay now coalesces short PTY bursts before emitting UI updates.
A 16ms window, 64-chunk cap and 64 KiB byte target concatenate chunks in arrival
order; a single 16 KiB reader chunk may overshoot the byte target by one chunk.
Exit handling is biased ahead of output, and the 500ms post-exit drain deadline
wins over a continuous burst so a noisy child cannot retain the terminal
forever. Output EOF still waits for the exit code.

Two new relay tests cover 100 tiny writes coalescing without reordering and a
continuous output stream losing to the exit deadline. Existing final-data,
EOF and cancellation tests still pass. Latest complete gate642 with Clippy/build
pass (`terminal-coalescing-final-*`). Live TUI/burst acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Human-paced streaming reveal (2026-11-01)

Assistant streaming text now reveals at Source's cadence while the transcript
retains the full text. `TranscriptUi` stores a per-block visible prefix and the
first snapshot is shown in full, matching Source. Appends beyond the 512-byte
immediate window wait for a 24ms tick, then advance with 2/4/8/256-byte steps,
character-safe boundaries and whitespace/punctuation snapping. A retry/edit
that is not a prefix snaps to the new text; completion and session changes drop
the pacing state. Projection substitutes only the visible prefix, so copy and
replay still read the authoritative block.

Pure step/cadence tests and a projection test verify the delayed tick, bounded
steps, final snap and pacing-state cleanup. Latest complete gate644 with
Clippy/build pass (`paced-streaming-final-*`). Live typing-during-stream and
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Tool large-output and patch expansion (2026-11-01)

Tool expansion now follows Source's availability rule: any tool with input or
output (and exec/patch) can open, not just execution and patch cards. Non-shell
output is capped at 6000 characters with `… (truncated)`; shell output and patch
diffs remain complete. Expanded bodies are focusable `Scrollable content`
regions with 240px shell/patch caps and 280px generic caps. Generic text wraps,
shell output keeps horizontal flow, and patch diff rows retain colored
add/delete rows plus horizontal scrolling. The trigger always exposes
`aria_expanded`.

One model test covers the exact limit, oversized Unicode text and the truncation
marker; a rendered app test expands a 7000-character read output and a colored
patch diff and verifies both scroll regions. Latest complete gate646 with
Clippy/build pass (`tool-output-final-*`). Copy and complete visual/a11y
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Expanded tool copy action (2026-11-01)

Expanded tool cards now have a Source-shaped copy action at the top-right. It is
transparent until the card is hovered or the button is keyboard-focused, uses
the new copy/check vector icons, announces `Copy`/`Copied`, and clears the
confirmation after two seconds. Shell tools copy the full `$ command` header and
output; other tools copy the complete output rather than the truncated display
text. Copy state is per tool card and survives redraws.

Copy text tests cover shell formatting, untruncated output and non-shell output.
A rendered app test expands a shell card, clicks the action and verifies the
clipboard contains `$ printf hi\n\nhello`. Latest complete gate648 with
Clippy/build pass (`tool-copy-final-*`). Live clipboard/visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Generic tool IN/OUT expansion (2026-11-01)

Non-shell tool cards now expand into Source's two-section layout. Pretty input
and output each have their own `IN`/`OUT` label, wrapped mono body and
6000-character truncation; a divider separates them and empty sections are
omitted. The pair sits inside the focusable `Scrollable content` region and the
280px generic height cap. Shell output and patch diffs keep their specialized
bodies.

A rendered app test expands a read tool with input and oversize output and now
verifies the IN, OUT, scroll-region and patch-diff selectors together. Latest
complete gate648 with Clippy/build pass (`tool-io-final-*`). Live visual/a11y
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Tool-card accessibility semantics (2026-11-01)

Tool cards now separate interactive triggers from static summaries. Expandable
heads are focusable buttons with `aria_expanded`, a visible chevron and GPUI's
Enter/Space activation; non-expandable tools render as non-focusable labeled
summaries without button/tab-stop semantics. Expanded bodies retain the
focusable `Scrollable content` region.

A rendered app test expands with a pointer click, collapses with Enter and
reopens with Space, while also verifying the static card selector. Latest
complete gate649 with Clippy/build pass (`tool-a11y-final-*`). VoiceOver and
complete visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## W3C/Sass editor markers (2026-11-01)

Validation results now flow into the native editor instead of only the Problems
tray. W3C and Sass diagnostics map to a `Validation` diagnostic kind with source
labels, merge into the editor's marker/banner set, clear when a new validation
starts, and expire when the buffer revision changes. A rendered editor test
confirms the banner and line marker appear and disappear after an edit.

Validation request tracking now supersedes older in-flight requests for the same
document, preventing a late stale result from replacing newer markers. A pure
mapping test and a stale-request regression test were added. Latest complete
gate652 with Clippy/build pass (`validation-markers-final-*`). Explicit
cancellation, network-failure retry UX and live validator/visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Validation cancellation and retry UX (2026-11-01)

Validation requests now have an explicit lifecycle. While a request is pending,
the status menu action becomes Cancel validation; the app removes the request
before telling the engine to cancel it, so a late result is ignored. W3C
validation selects on a cancellation token and returns immediately; Sass
validation cancels its pinned validator child process. Results are scoped to the
active document, and a failed request exposes both a Failed status row and a
Retry validation action.

The W3C cancel test uses a pre-cancelled token, the Sass test launches a held
validator process and verifies cancellation terminates it, and app tests cover
pending-request selection and action labels. Latest complete gate655 with
Clippy/build pass (`validation-cancel-final-*`). Live validator/network and
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Legacy queued-message migration (2026-11-01)

Local queue storage now uses Source's v3 migration behavior. Loading an
unversioned or older `message-queues.json` moves all existing queues into a
persisted quarantine map, clears the active queues and rewrites the file with
`version: 3`. Quarantined prompts are never listed, claimed or delivered, and
remain isolated across restarts; new messages enqueue normally after migration.

A model test covers legacy load, quarantine contents, persisted version/schema,
restart isolation and post-migration enqueue. Latest complete gate656 with
Clippy/build pass (`legacy-queue-final-*`). Live imported-profile and visual
acceptance remain. The quarantine matches the current Source implementation,
which deliberately does not replay pre-native queued prompts.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Model-picker favorites/visibility rendered coverage (2026-11-01)

The existing model-picker preference implementation now has rendered coverage.
A generated model menu shows the favorite star; clicking it updates the
provider-qualified favorite set. The settings gear enters visibility mode, and
clicking a model row toggles its provider-qualified hidden preference. Provider
hiding continues through the same persisted preference path and remains covered
by the model visibility tests. Stable debug selectors were added for the menu,
rows, favorite control and settings gear.

The rendered app test verifies the menu appears, favorites update, settings mode
opens and hidden-model state updates. Latest complete gate657 with Clippy/build
pass (`model-picker-preferences-final-*`). Live provider refresh and complete
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Validation status-menu rendered states (2026-11-01)

Rendered status-menu coverage now exercises validation lifecycle labels. A
pending validation request renders `Cancel validation`; clicking it removes the
request and closes the menu. A failed validation renders `Retry validation`
plus the red Failed row. Stable selectors distinguish cancel, retry and default
actions. The app test opens a real document editor and status popover for each
state.

Latest complete gate658 with Clippy/build pass (`validation-menu-final-*`).
Live network/validator and visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Provider-level model visibility coverage (2026-11-01)

The model-settings view now has rendered coverage for provider-level visibility.
A two-provider menu toggles each provider's Hide control, verifies the persisted
`hidden_providers` set, returns to list mode and confirms the hidden provider's
model row is gone while the favorite row remains. Provider controls expose
stable debug selectors.

Latest complete gate658 with Clippy/build pass
(`model-provider-visibility-final-*`). Live provider refresh and complete visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Local queue-chip render and keyboard reorder (2026-11-01)

Local recovery chips now render when the runtime inbox is empty instead of
being hidden behind an inbox-only condition. A rendered app test seeds three v3
durable entries through a temporary engine profile, verifies the queue list,
reorders with the pointer, retains focus across the redraw and reorders again
with Enter. Action buttons now expose stable debug selectors and route through
the existing focus-control wrapper.

Latest complete gate659 with Clippy/build pass (`queue-chip-keyboard-final-*`).
Live imported-profile and visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Tool copy keyboard/focus coverage (2026-11-01)

Expanded tool-copy actions now have rendered keyboard focus and activation
coverage. The hidden action is revealed by mouse-copy or keyboard focus; after
the copied state is cleared, focus-visible styling keeps it visible. Pressing
Enter on the focused action copies the full shell command and output again.
Stable selectors distinguish hidden and visible states.

Latest complete gate660 with Clippy/build pass (`tool-copy-keyboard-final-*`).
Live clipboard and visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Mixed queue-chip interaction coverage (2026-11-01)

Rendered coverage now exercises runtime and local recovery queue chips together.
Runtime rows render first, expose edit/steer/remove controls without local
reorder controls, and editing a runtime chip restores its text to the composer
and removes it. Local recovery rows expose reorder/edit/send/remove actions.
Preview and action selectors make the checks deterministic.

Latest complete gate661 with Clippy/build pass (`queue-chip-mix-final-*`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Legacy follow-up behavior and settings coverage (2026-11-01)

The follow-up preference now accepts Source's legacy `immediate` value as
`steer`. A settings-load test covers that alias. The rendered model-settings test
also selects the Steer control, verifies the stored preference, and confirms the
runtime steering capability still gates the row.

Latest complete gate662 with Clippy/build pass (`follow-up-legacy-final-*`).
Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Staged revert recovery dock (2026-11-01)

A stage-then-commit failure no longer clears the runtime revert boundary. The
engine emits `RevertStaged`, retains the stage, and clears only when a later
commit succeeds, the user discards, or the session is removed. The composer now
renders Source's `Undo staged` dock with `Commit undo` and `Discard`; dedicated
commit/clear commands drive the retained stage.

Engine-flow coverage verifies retained staged state, commit success and explicit
discard. A rendered app test verifies both dock actions and cleanup. Latest
complete gate663 with Clippy/build pass (`staged-revert-final-*`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Diff-row accessibility labels (2026-11-01)

Changed-file rows now expose accessible labels for added, removed and context
lines, including line numbers and bounded text content. Selection/copy,
intraline emphasis, wrapping, sticky headers and virtualized rendering remain
unchanged. A model test covers the label variants and long-text cap; existing
rendered Changes tests continue to exercise row rendering, selection and copy.

Latest complete gate664 with Clippy/build pass (`diff-a11y-final-*`). Live
VoiceOver/platform and visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## User-message transcript attachments (2026-11-01)

`Block` and `Message` now carry prompt-file attachment names. The engine
derives names from submitted `PromptFile` entries when it inserts the optimistic
local user message, and replay parses user `file` content parts with a basename
fallback. Block assembly propagates the names to the user row, which renders
Source's right-aligned wrapping file cards: generic file glyph, truncated name,
220×48 cards with an 8px gap and an 82% column cap. Empty lists render nothing.

Coverage: a core test asserts local-user attachment names survive block
assembly, a replay test covers name and path fallbacks, and a rendered GPUI test
asserts card geometry, right alignment and wrapping. Latest complete gate667
with Clippy/build pass (`user-attachments-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Session-row close action (2026-11-01)

Session rows now render Source's close control when the conversation has an
open panel. The button is hover/focus-revealed, 20×20, carries
`Close the … session` accessibility text, stops propagation and calls the panel
close path rather than session deletion. Unpinned pin controls now follow
Source's hover/focus reveal too, while pinned rows keep the star visible.

A rendered app test seeds one open and one closed conversation, verifies only
the open row exposes the close selector, clicks it, and asserts the panel is
gone, the active id is cleared and the conversation remains listed. Latest
complete gate668 with Clippy/build pass (`session-close-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Language-tooling inventory correction (2026-11-01)

Current Source does not start an external language server. Its renderer imports
Monaco's bundled TypeScript/JavaScript/CSS/HTML/JSON workers and registers their
completion, hover, signature, definition, reference, symbol, rename, diagnostic,
formatting and inlay-hint providers. The only `lsp.updated` reference is stream
event plumbing. The parity matrix now marks the external-LSP row N/A and records
the real remaining gap under Editor capabilities as Monaco-worker-equivalent
semantics, rather than planning an engine-side LSP client Source does not have.

No code changes for this correction; gate668 evidence remains current.

## Bounded editor document formatting (2026-11-01)

`Shift+Alt+F` now formats the active document. JSON validates through the real
parser and is printed from the original token stream, preserving key order and
number spelling. JavaScript, TypeScript, CSS, SCSS and Less reindent by brace
depth and trim trailing whitespace; multiline strings, templates and block
comments are preserved byte-for-byte. The formatter skips JSX/TSX and HTML
because text whitespace is meaningful there.

The editor applies the result as one undoable edit and remaps the caret to the
same code position. Unit tests cover JSON order/number preservation, invalid
JSON no-ops, TS template/comment/string preservation, CSS strings/nesting and
JSX/HTML skipping. A rendered keyboard test asserts `Shift+Alt+F` reindents a
TypeScript document and `Cmd+Z` restores it. Latest complete gate674 with
Clippy/build pass (`editor-format-final-*`). Monaco spacing/range formatting and
live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor range formatting (2026-11-01)

`Cmd+K Cmd+F` now formats the editor selection. An empty selection follows
Monaco's Format Selection action by formatting the caret line. Selected ranges
expand to whole lines, inherit brace depth from the preceding document, and are
replaced without touching outside text. JSON property fragments are wrapped and
validated before being reindented; complete nested values format directly.

The edit is one undo step with the selection settled on the formatted range (or
the caret mapped inside the formatted line). Multiline strings, templates and
block comments remain untouched, and JSX/TSX/HTML remain unsupported. Unit
tests cover selected-line and caret-line ranges plus nested JSON fragments.
Rendered tests cover both chords and undo restoration. Latest complete gate679
with Clippy/build pass (`editor-range-format-final-*`). Monaco spacing
normalization, multi-cursor ranges and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Multi-cursor range formatting (2026-11-01)

`Cmd+K Cmd+F` now formats every primary and extra cursor in one transaction.
`TextBuffer::replace_many_with` applies independent replacement strings from the
end, computes each post-edit range, and commits one whole-buffer undo diff.
Each editor cursor expands to whole lines, inherits brace depth from its
preceding document, and is remapped to its formatted line/range. Same-line
overlapping cursors are skipped rather than merged; JSX/TSX/HTML remains a
no-op.

Coverage: a text-buffer test verifies distinct scattered edits, returned
ranges, selection and one-step undo/redo; a rendered editor test formats two
TypeScript selections, preserves both cursors and restores the original text
with one undo. Latest complete gate681 with Clippy/build pass
(`editor-multi-format-final-*`). Monaco spacing normalization and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Script spacing normalization (2026-11-01)

JavaScript/TypeScript formatting now normalizes intra-line spacing alongside
indentation. A small tokenizer enforces comma/semicolon/colon spacing, brace
interiors, control-flow keyword parentheses, and assignment/equality/arrow/
logical/compound-assignment operators. Regex literals, unary and optional
syntax, strings, comments and template lines are preserved; arithmetic,
comparison and ternary spacing is intentionally left unchanged so generics,
regex and unary semantics cannot be rewritten accidentally.

Two formatter tests cover the normalized style and idempotence plus unary,
optional chaining, regex, string, comment and template preservation. Latest
complete gate683 with Clippy/build pass (`editor-spacing-final-*`). Monaco
arithmetic/comparison/ternary spacing and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Same-line multi-cursor formatting merge (2026-11-01)

Multi-cursor formatting now merges expanded ranges that touch or overlap before
calling the formatter, matching Monaco's range merge step. The formatter runs
once on the merged lines, then each original cursor is remapped through the
merged replacement using line/column remapping. Two cursors on one line remain
two cursor ranges rather than dropping the later one.

A rendered test places two carets on a compact TypeScript line, runs
`Cmd+K Cmd+F`, asserts the normalized line and both cursor ranges, and restores
the original text with one undo. Latest complete gate684 with Clippy/build pass
(`editor-cursor-merge-final-*`). Arithmetic/comparison/ternary spacing,
JSX/HTML formatting and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Arithmetic, ternary and generator spacing (2026-11-01)

Script spacing now classifies `+`/`-` as binary only after an expression-ending
token, keeps unary operators attached to their operands, preserves
`function*`/`yield*` star attachment, and spaces multiplication/division/
remainder/exponent operators. Standalone `?` is treated as a ternary operator;
its matching colon is recognized by balanced-depth scanning so object/type
colons stay attached. Optional `?:`, `?.` and `?.()` forms are left alone.

`<`/`>` comparison and generic delimiters remain intentionally unnormalized:
without a TypeScript parser, adding spaces around them can change generic calls
or type assertions. Existing whitespace is preserved. Formatter tests cover
mixed precedence, nested ternaries, unary plus/minus, generator/yield stars,
optional forms and idempotence. Latest complete gate685 with Clippy/build pass
(`editor-arithmetic-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML document formatting (2026-11-01)

HTML files now format with `Shift+Alt+F`. A tag-aware line formatter tracks
element depth, outdents closing tags, treats void and self-closing elements as
leaves, and ignores `>` inside quoted attributes. When a `<pre>`, `<script>`,
`<style>` or `<textarea>` opens, subsequent raw lines are preserved exactly
until the matching closing tag; non-tag text lines also keep their original
whitespace.

A formatter test covers doctype/nesting, inline elements, void tags, quoted
attributes, raw blocks and idempotence. JSX/TSX remains skipped, and HTML
selection/range formatting is still pending. Latest complete gate686 with
Clippy/build pass (`editor-html-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Legacy workspace relocation (2026-11-01)

Legacy bookmark import now parses Source's v2 `locations` records. The resolver
checks the current path first, then follows `movedTo` alias chains (longest
matching root first, cycle-safe) while preserving nested workspace suffixes.
Without an alias it matches a recorded device/inode against directories under
up to four nearby ancestors, bounded to 500 entries per directory, and
relocates only on one unique match. Resolved paths are canonicalized and
bookmarks are deduplicated again after relocation; the Electron profile stays
read-only.

Tests cover multi-step alias chains, nested bookmarks, Unix inode recovery and
source immutability. Runtime moves after import, `.git/opencode` project-id
recovery and ambiguous matches still fall back to the stored directory. Latest
complete gate689 with Clippy/build pass (`workspace-relocation-final-*`).

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Runtime workspace relocation (2026-11-01)

Location records now persist in native settings with Source's `movedTo` shape
plus optional device/inode identity. They survive restart and are normalized and
deduplicated. Bookmark activation calls the resolver before opening: a moved
bookmark and every bookmark under the same old root are canonicalized, the
alias/current location is recorded, preferences are persisted, and the resolved
directory is opened (reusing an existing session when present). Repeated clicks
are idempotent.

Coverage: import aliases/chains and Unix inode recovery, runtime root/nested
bookmark repair with alias recording, settings JSON round-trip, and a second
runtime resolution that reports no further change. Latest complete gate691 with
Clippy/build pass (`workspace-live-relocation-final-*`). Ambiguous matches and
`.git/opencode` project-id recovery, plus live visual acceptance, remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML selection/range formatting (2026-11-01)

HTML document and selection formatting now share one tag-aware line state
machine. Range formatting computes the prefix's open-tag depth, formats only the
selected whole lines at that depth, and refuses selections that begin inside
`<pre>`, `<script>`, `<style>` or `<textarea>` raw content. Non-tag text lines
and raw blocks remain untouched.

Coverage: a formatter test formats a nested `<div>`/`<span>` range and verifies
a script-content selection is a no-op; a rendered editor test runs
`Cmd+K Cmd+F` on an HTML range and restores it with one undo. Latest complete
gate693 with Clippy/build pass (`editor-html-range-final-*`). JSX/TSX
formatting, `<`/`>` spacing and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Workspace project-ID recovery (2026-11-01)

Workspace location records now capture `.git/opencode` content when a root is
recorded. If a moved directory no longer matches by alias or device/inode,
relocation scans the same bounded nearby-directory set for a same-named
directory whose `.git/opencode` matches, relocating only on one unique match.
The project ID persists with the settings location table.

A test records a project, deletes it, recreates it under a new inode/path with
the same ID and verifies runtime resolution/repair. Existing alias, inode and
persistence tests remain green. Latest complete gate694 with Clippy/build pass
(`workspace-project-recovery-final-*`). Ambiguous-match reporting and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JSX/TSX document formatting (2026-11-01)

`.jsx` and `.tsx` files now format with `Shift+Alt+F`. A bounded line formatter
tracks script brace and parenthesis depth plus an open JSX tag stack. It
indents component/fragment/text lines, preserves attribute token spacing and
self-closing tags, and returns to ordinary script spacing/indentation once the
JSX stack closes. Inline JSX is recognized only at expression-start positions,
so common generic type/call forms are not mistaken for tags.

Tests cover a component with nested tags and a self-closing child, a fragment
with text, inline attribute preservation, ordinary spacing outside JSX, and a
rendered TSX document/undo path. JSX range formatting, ambiguous `<T>` type
assertions/generics and complex JSX-expression formatting remain bounded gaps.
Latest complete gate696 with Clippy/build pass (`editor-jsx-final-*`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JSX/TSX range formatting (2026-11-01)

JSX/TSX range formatting now reuses the document formatter's state machine.
The prefix supplies script brace depth, parenthesis depth, the open JSX tag
stack and literal/comment state; selected whole lines inherit that context and
format only their range. Selections beginning inside multiline
strings/templates/block comments are skipped.

Coverage: a formatter test formats a nested `<div>`/`<span>` range using the
surrounding `return (` depth, and a rendered editor test runs `Cmd+K Cmd+F` on
a TSX range and restores it with one undo. Latest complete gate698 with
Clippy/build pass (`editor-jsx-range-final-*`). Complex JSX expressions,
ambiguous `<T>` generics/assertions, `<`/`>` spacing and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Ambiguous workspace relocation reporting (2026-11-01)

Workspace identity lookup now returns an explicit outcome: resolved, missing or
ambiguous with candidate paths. Nearby-directory scanning no longer stops at a
missing intermediate parent, so project-ID recovery can still find candidates
under a deeper missing path. A multi-match is never guessed: bookmark activation
shows an error toast with up to three candidates (plus a remainder count) and
leaves settings untouched. Legacy import counts ambiguous entries and startup
logs the count.

A Unix test creates two directory symlinks with the same project ID under
different nearby ancestors, then asserts an ambiguous outcome, unchanged
bookmark, both candidates and the user-facing message. Latest complete gate699
with Clippy/build pass (`workspace-ambiguity-final-*`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Safe `<`/`>` comparison spacing (2026-11-01)

Single `<`/`>` now gain spaces only when the comparison is unambiguous enough
for a bounded formatter: numeric/string/regex operands, a closing `)`/`]`,
literal keywords, or a lowercase identifier compared against a numeric literal
(including signed literals). Generic/type arguments and ambiguous identifier
comparisons such as `a<b` keep their original spacing, avoiding changes to
`Array<T>`, `foo<T>()` or type-assertion-like forms.

A formatter test covers every normalized form plus unchanged generics and
ambiguous comparisons and verifies idempotence. Latest complete gate700 with
Clippy/build pass (`editor-comparison-final-*`). Ambiguous identifier
comparison spacing and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Unpaired identifier comparison spacing (2026-11-01)

Single `<`/`>` comparison classification now covers identifier-vs-identifier
forms without a numeric/string operand. A `<` with no later `>`/`>>`/`>>>` on
the line, and a `>` with no earlier `<`, are treated as unpaired comparisons
and spaced. Angle-paired forms stay untouched, keeping `Array<T>`, `foo<T>()`
and type arguments intact; chained/ambiguous paired comparisons retain their
spacing.

The formatter test now covers `a<b`/`a>b`, all prior safe forms, unchanged
generics and idempotence. Latest complete gate700 with Clippy/build pass
(`editor-comparison-identifiers-final-*`). Ambiguous angle-paired comparisons
and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Paired `<`/`>` comparison chains (2026-11-01)

Angle delimiters are now paired with a stack, including `>>`/`>>>` as generic
closers. A pair receives comparison spacing when both sides are value operands,
the left head is not an upper-case type name, the right side is not a
type/function/index suffix (`(`, `.`, `[`, `as`, `extends`, etc.), and the
contents carry no type-only indicators (`|`, `&`, `keyof`, `=>`, `:`, `?`).
This normalizes `a<b>c` and `a<b && c>d` while preserving `Array<T>`,
`foo<T>()`, `Array<T> as const` and nested `>>` generics.

The comparison formatter test now includes paired chains and type-suffix cases
alongside unpaired/safe forms and idempotence. Latest complete gate700 with
Clippy/build pass (`editor-comparison-pairs-final-*`). Residual type-vs-
comparison ambiguity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Paired comparison chains with type punctuation (2026-11-01)

Paired `<`/`>` classification now depends on the surrounding operands rather
than rejecting type punctuation inside the pair. When both sides are values and
the head is lower-case, chains whose middle contains `|`, `&`, `:` or `?`
normalize as comparisons (`a<b | c>d`, `a<b ? c : d>e`). Upper-case type heads,
type-keyword suffixes (`as`, `extends`, `instanceof`, `in`, `of`), calls,
indexing and `>>` generic closers remain untouched.

The formatter test covers both new chain shapes alongside the existing safe,
unpaired, generic, type-suffix and idempotence cases. Latest complete gate700
with Clippy/build pass (`editor-comparison-pairs2-final-*`). Residual type-vs-
comparison ambiguity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Type-assertion angle spacing (2026-11-01)

Comparison operands now require expression-ending tokens rather than any word,
so control/expression keywords cannot anchor a paired comparison. TypeScript
type assertions such as `return <Foo>bar`, `await <Foo>bar` and
`const x = <Foo>bar` keep `<Foo>` intact; ordinary identifier comparisons and
paired chains remain spaced.

A formatter test covers `return` and assignment assertion positions. Latest
complete gate701 with Clippy/build pass (`editor-type-assertion-final-*`).
Complex JSX-expression ambiguity and `.tsx` `<T>` generic-vs-tag ambiguity,
plus live visual acceptance, remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## `.tsx` generic-parameter disambiguation (2026-11-01)

JSX tag detection now excludes TypeScript generic parameter lists. A leading
`<...>` is treated as generics when a balanced parameter list contains a comma,
an `extends` constraint, or a default `=` followed by an argument list; quoted
JSX attributes are ignored during that scan, so normal attribute tags remain
JSX. Generic arrows format with ordinary TypeScript spacing and do not open a
JSX tag stack.

A formatter test covers `<T,>`, `<T extends object>` and `<T = string>` arrows,
plus idempotence; existing JSX attribute/group tests remain green. Latest
complete gate702 with Clippy/build pass (`editor-tsx-generics-final-*`).
Multi-line generic parameters, complex JSX expressions and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Multi-line `.tsx` generic parameters (2026-11-01)

JSX formatting now tracks generic parameter lists across lines. An expression
start ending in `<`, or a parameter line carrying comma/`extends`/default
markers, enters generic mode. Parameter lines indent by angle depth (capped at
32 lines), the closing `>` line returns to script depth, and a following `(`
sets arrow-argument indentation until the matching close. Quoted/comment content
does not affect angle depth.

Tests cover the depth helper and a multi-line `<T, U>` generic arrow with its
argument line and closing parenthesis, plus idempotence. Latest complete gate704
with Clippy/build pass (`editor-tsx-multiline-final-*`). Complex JSX
expressions, residual type-vs-comparison ambiguity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JSX expression-container indentation (2026-11-01)

JSX formatting now tracks `{...}` expression depth on JSX lines. Nested tags
inside an open expression indent one extra level, balanced inline expressions
leave depth unchanged, and lines whose closing braces exceed opens outdent
before rendering. Braces inside quoted strings are ignored; comments end the
line scan.

A formatter test covers a multiline `items.map(... => (<span>...` block,
nested tag indentation, closing `))}` outdent and idempotence. Latest complete
gate705 with Clippy/build pass (`editor-jsx-expressions-final-*`). Deeply nested
expression/parenthesis alignment, residual type-vs-comparison ambiguity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Multi-file launch/drop ordering (2026-11-01)

Launch path delivery now preserves every selected file instead of retaining only
the last one. Workspace files queue as ordered `DocumentKey` tab requests,
external files queue as ordered standalone tabs, and each render drains the
whole batch in order. A shared classifier canonicalizes paths, detects workspace
membership and normalizes nested relative paths; Explorer-created files append
to the same queue.

A unit test classifies two nested workspace files followed by an external file
and verifies order, relative paths and external membership. Latest complete
gate706 with Clippy/build pass (`multi-file-launch-final-*`). Live
second-instance routing, mixed directory/file activation and physical OS drag
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JSX ternary branch alignment (2026-11-01)

JSX expression formatting now outdents lines that begin with `)` while an
expression container is open, not only lines whose closing braces exceed opens.
Multiline ternary branches therefore align `) : (` with the opening
`{condition ? (` line, and the second branch tag keeps the expected nested
indent; the closing `)}` line outdents via brace depth.

A formatter test covers both ternary branches and idempotence. Latest complete
gate707 with Clippy/build pass (`editor-jsx-ternary-final-*`). Nested
expression alignment beyond one container level, residual type-vs-comparison
ambiguity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Nested JSX ternary indentation (2026-11-01)

JSX expression formatting now keeps a parenthesis stack that records whether a
`(` followed a ternary `?` or `:`. The depth beyond the first ternary branch is
added as nesting, and the stack decrements when that group closes. Nested
ternary branches step in one extra level per branch, while call/arrow/grouping
parentheses contribute no spurious depth.

A formatter test covers a two-level ternary with both else branches, closing
parentheses and idempotence. Latest complete gate708 with Clippy/build pass
(`editor-jsx-nested-ternary-final-*`). Other JSX expression alignment, residual
type-vs-comparison ambiguity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JSX nested arrow continuation alignment (2026-11-01)

JSX expression alignment now also covers nested arrow callbacks. The
parenthesis stack marks a `(` that follows `?`, `:`, `>`, `&` or `|` as a
continuation group; only depth beyond the first group is added as indentation.
Nested `items.map(... => (` / `item.children.map(... => (` callbacks therefore
step in one extra level each, while ordinary call/grouping parentheses still
contribute nothing, and closing parentheses unwind the stack.

A formatter test covers both nested arrows, the inner/outer closing parentheses
and idempotence. Latest complete gate709 with Clippy/build pass
(`editor-jsx-continuation-final-*`). Deeper JSX expression alignment, residual
type-vs-comparison ambiguity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Captured agent tool-input baselines (2026-11-01)

Tool calls now seed file baselines before an agent edits them, matching Source's
`session.tool.called` snapshot path. The engine walks the tool input for
`filePath`, `file_path` and `path` keys, including nested objects/arrays, keeps
the first occurrence and ignores HTTP values. Each readable workspace file is
captured under the watcher's confinement and 2 MiB limits into a known
baseline only when none exists, so later non-Git agent edits render as
before/after diffs instead of `Observed`. Existing and Git baselines are never
replaced; outside-root, unsafe, missing and oversized paths are ignored.

Four tests cover nested extraction/dedupe/HTTP exclusion, relative and absolute
capture, existing-baseline preservation and outside-root rejection. Latest
complete gate713 with Clippy/build pass
(`agent-tool-baselines-final-*`). Live tool-event timing, full Changes
projection and visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTTPS-only browser external handoff (2026-11-01)

Browser new-window requests now match Source's shared `safeExternalUrl` policy
instead of the broader guest navigation policy. Only credential-free HTTPS URLs
with a non-empty host are handed to the OS browser, and the normalized href is
used for the handoff. HTTP, embedded username/password, `javascript:`, `data:`,
`file:`, `mailto:`, `about:` and malformed values remain denied inside the
guest.

A browser unit test covers HTTPS paths, query/fragment and ports plus every
rejected scheme/credential form. Latest complete gate714 with Clippy/build pass
(`browser-external-https-final-*`). Downloads remain denied; popup edge cases
and live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Inactive-panel permission prompts (2026-11-01)

Permission cards are now session-scoped and rendered inside every tiled panel,
not only the active one. Inactive tiles show their own Allow/Always/Deny card
with a unique element namespace, and clicking a card or action activates that
panel to match Source's panel-wide mouse-down focus. Approve mode now
auto-replies for every session in the open panel registry (plus the active
session), matching Source's `panelForSession` check; closed warm sessions still
wait for an explicit reply.

A rendered two-panel test asserts the inactive card and actions exist, the
active tile does not borrow them, and clicking Deny brings the waiting tile
forward. Latest complete gate715 with Clippy/build pass
(`panel-permissions-final-*`). Per-tile forms, attachments, references, slash
menus, rich tool actions and live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Child-session back navigation and panel replacement (2026-11-01)

Delegated-agent cards and the new agent-header back control now follow Source's
`reopenSession`: an already-visible session is focused, otherwise the focused
panel is replaced in the same grid slot instead of appending a new panel.
Replacement preserves the slot's position and width. Child sessions render the
reference's back button (full-width row in the focused panel, compact arrow in
tiled headers) with `Back to <parent>` accessibility, and root sessions render
no control.

A rendered test covers a subagent card replacing the parent panel, the back
button returning to the parent, the grid staying at one panel, and the root
control disappearing. A panel-unit test covers width preservation, slot order
and focusing an already-open session without duplication. Latest complete
gate717 with Clippy/build pass (`child-back-navigation-final-*`). Live/paired
drill-down appearance and the remaining child transcript checks stay queued.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Source pin import from renderer localStorage (2026-11-01)

Source keeps `vela.pinnedSessions` only in Chromium localStorage, so pins now
migrate from the Electron profile without touching it. The engine reads the
LevelDB write-ahead `.log` files with their 32 KiB record framing, parses
`WriteBatch` puts/deletes and uses the highest sequence for the pin key. Values
decode from Chromium's Latin-1 and UTF-16 encodings; fragmented FIRST/MIDDLE/
LAST records reassemble before applying. If the key is no longer in a log
(compaction), a bounded raw scan of `.ldb` literals recovers the newest array.

The importer sets a persisted one-time marker after a readable store is
scanned. That makes native pins authoritative after migration, so unpinning in
native cannot be undone by a later launch re-reading Source's stale value. A
profile with no Local Storage store leaves the marker pending and retries next
launch. Main now saves settings when the pin scan runs even if there are no
workspaces to add.

Tests cover newest-batch selection, both encodings, later deletes, fragmented
records, the compacted-table fallback, malformed/truncated input, a missing
store, the one-time merge and the no-resurrection contract. A read-only probe
against the local Source profile recovered both pinned session ids from its
real Chromium log (`source-pin-import-real-profile.txt`). Latest complete
gate724 with Clippy/build pass (`source-pin-import-final-*`). A live
real-profile restart with persisted native pins and visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Focus-aware native caret blink (2026-11-01)

The shared `TextInput` now paints a blinking caret instead of a permanently
solid one. The 530 ms phase starts only while the input owns focus in an active
window; blur or window deactivation stops the timer and restores a steady
visible caret, so no background task runs for idle fields. Every key press,
pointer caret move and programmatic `set_text` restarts the visible phase, and
a nonempty selection still paints selection with no caret.

A deterministic GPUI timer test advances the clock through two blink phases and
verifies that blur leaves the caret steadily visible with no further toggles,
then checks a fresh visible phase on refocus. Latest complete gate725 with
Clippy/build pass (`caret-blink-final-*`). Live caret appearance remains queued
with the composer behavior matrix.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown table cells with mixed inline content and images (2026-11-01)

Table cells previously passed their body through the plain inline renderer,
which dropped images and left only their alt text. Cells containing images now
use the same ordered text/image projection as prose: styled runs and links
around each image stay intact, and block images stack vertically through the
shared markdown image loader at Source's 200px cap.

Auto-column sizing now resolves cell images and includes the widest fitted
image in both the minimum and maximum column width. A narrow table therefore
scrolls horizontally instead of clipping a 200px cell image. Table, cell and
image containers also expose debug selectors for rendered checks.

The rendered test proves bold/code/link text and an image coexist in a table,
text appears before and after the image, the image stays inside its cell and
resolves to a 200px fitted width, and a 200px content column overflows a
narrow table with a positive horizontal offset. All 18 markdown tests pass.
Latest complete gate726 with Clippy/build pass
(`markdown-table-images-final-*`). Reflow during asynchronous image loads,
platform table-header accessibility and live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Shift operators vs generic angle closers (2026-11-01)

The JS/TS spacing pass previously treated every `>>`/`>>>` as a generic
closer. Shift expressions therefore stayed unspaced (`a<<b`, `a>>b`), and a
mixed comparison/shift such as `a<b>>c` lost its comparison spacing too.
`<<`/`<<=` were not binary at all.

Shift tokens now record the `<` tokens they close. The classification reuses
the comparison heuristic against the outermost closed `<`: if it looks like a
comparison, the shift token and every angle it closed become binary; otherwise
the sequence stays a generic closer. A shift token with no open `<` is always
binary. This formats `a < b >> c` and `x = y << 2` while keeping
`Map<string, Array<number>>()` and `A<B<C>>` tight. `<<` and `<<=` are
unambiguous binary operators.

The formatter test covers `<<`, `>>`, `>>>`, `<<=`, `>>=`, mixed
comparison/shift expressions, nested generic calls/types and idempotence; all
27 formatter tests pass. Latest complete gate727 with Clippy/build pass
(`format-shift-operators-final-*`). TypeScript generic-instantiation vs
comparison for following `[`/`!` tokens and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Single bitwise operators in JS/TS spacing (2026-11-01)

Compound bitwise assignments were already classified as binary, but single
`&`, `|` and `^` were not, so `a&b`, `a|b`, `a^b`, `type T=A|B` and
`type U=A&B` stayed unspaced. They are now binary alongside their compound
forms. Expression XOR/AND/OR and TypeScript union/intersection types normalize
to `a & b`, `a | b`, `a ^ b`, `A | B`, `A & B` and
`Record<A | B, U & B>`; `&=`, `|=` and `^=` keep their existing spacing.

A formatter test covers expression and type positions plus idempotence; all 28
formatter tests pass. Latest complete gate728 with Clippy/build pass
(`format-operators-final-*`). The remaining generic-instantiation vs
comparison cases and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Generic instantiation vs comparison spacing (2026-11-01)

The angle-pair heuristic previously required the token after `>` to be a
value-ending operand, so TypeScript expressions such as `a<b>[c]` and `a<b>!c`
stayed tight even though the TypeScript parser reads them as comparisons. The
right operand is now tested with an expression-start predicate: `[`, `!`, `~`,
unary `+`/`-`, `++`/`--`, `typeof`/`void`/`await`/`new` and literal/object
starts all switch the pair to comparison spacing. Calls, `as`/`satisfies`,
`&&`/`||`/`??`/`**`, conditional and `?.`/`.` positions remain instantiation
expressions and stay tight, matching the TS 5.9 AST choices.

A type-context guard prevents the new `[` rule from spacing lowercase generic
types: `type`/`interface` declarations, `extends`/`implements`/`as`/`satisfies`
contexts and annotation colons before the statement's first assignment keep
`foo<bar>[]` tight. Object-literal values and ternary branches keep comparison
spacing because their colons appear after the assignment.

The formatter test covers comparison operands, instantiation/call/type
positions, aliases, indexed aliases, interface heritage, annotated
declarations, object values, ternary branches and idempotence; all 29
formatter tests pass. Latest complete gate729 with Clippy/build pass
(`format-generic-instantiation-final-*`). Lowercase generic type names in
parameter/default type positions and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Lowercase generic types in parameter and default positions (2026-11-01)

The type-context guard now recognizes the remaining lowercase generic
positions. It derives type context from unmatched parameter parentheses
(including callbacks nested in object literals), generic-parameter `=` or
`extends` clauses, return-type colons after `)`, and top-level annotation
colons. A default value after an annotation `=` resets to expression context,
so `(x: T = a<b>[c]) => a<b>[d]` spaces both comparisons.

`(x: foo<bar>[])`, `(x: foo<bar>[c])`,
`function f<T = foo<bar>[]>()`, `<T = foo<bar>[]>` generic arrows,
`class C<T = foo<bar>[]>` and `(x): foo<bar>[]` now stay tight, while
object-literal values, ternary branches, default values and arrow bodies still
format comparisons. The formatter test covers all of these and idempotence; all
30 formatter tests pass. Latest complete gate730 with Clippy/build pass
(`format-generic-types-final-*`). Lowercase generic types in class property
declarations and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Lowercase generics in class property declarations (2026-11-01)

Class property types previously had no type-context signal, so lowercase
generic types in fields such as `x?: foo<bar>[]` were spaced as comparisons.
The formatter now tracks class-body brace depth and generic parameter lists,
and treats a colon at the class-body depth as a property type. Optional `?:`
fields, nested object/function property types, method return signatures,
initializers, method bodies, static blocks and nested classes are handled. An
initializer `=` or a method/static block resets to expression context, so
`class A { x?: foo<bar>[c]; v: () => foo<bar>[]; }` stays tight while
`x = a<b>[c]` and comparisons inside methods still format.

Return-type context also now stops at a method body or arrow body, preventing
a method's return annotation from leaking type context into its body. The
focused test covers optional fields, nested object/function property types,
initializers, method bodies, static blocks, nested classes and idempotence; all
31 formatter tests pass. Latest complete gate731 with Clippy/build pass
(`format-class-properties-final-*`). Live visual acceptance and the remaining
worker-derived language tooling remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown table cell accessibility labels (2026-11-01)

Table cells had roles and row/column indices but no explicit accessible names,
so a screen reader had to infer a cell's content from descendant paint. Header
and body cells now carry a plain label derived from their rendered text; image
alt text is included, whitespace is collapsed, untitled images fall back to
"Image", and empty cells use "Column N header" or "Row R, column C" positional
names. Header cells keep the column-header role, body cells the cell role, and
rows/tables keep their row/column indices and counts.

A focused unit test covers header/body roles and labels for mixed
text/code/emphasis, image alt text, empty cells and image-only cells. The
existing rendered table test now also asserts header/body row and cell
presence. All 19 markdown tests pass. Latest complete gate732 with Clippy/build
pass (`markdown-table-a11y-final-*`). Platform header mapping, VoiceOver and
live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Explicit browser download policy (2026-11-01)

The browser guest previously denied every download. Downloads now follow an
explicit policy: only credential-free HTTP/HTTPS URLs are admitted; file,
data, javascript, blob, about, custom and credentialed URLs are denied before
any path is chosen. Allowed downloads save to the OS Downloads directory, using
wry's suggested filename when safe or the percent-decoded final URL segment
otherwise, with a `download` fallback. Path separators, NULs and control
characters are replaced, traversal components are discarded through
`file_name`, and existing files get numbered ` (n)` suffixes without
overwriting.

Tests cover accepted/rejected schemes, suggested-vs-URL filename preference,
Unicode/percent decoding, missing and traversal names, collision numbering and
destination assembly. Latest complete gate735 with Clippy/build pass
(`browser-downloads-policy-final-*`). In-app download feedback/shelf, popup
edge cases and live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Browser download completion feedback (2026-11-01)

Download completions now emit a browser event carrying the original URL, the
optional saved path and the success flag. The app maps that to the shared toast
surface: it prefers the saved filename, falls back to the percent-decoded URL
name, and uses `Download complete`/`Download failed` when no name is available.
Success toasts use normal styling; failures use error styling. This covers the
macOS case where wry reports no saved path.

A focused test covers saved-path names, URL-derived Unicode names, anonymous
success and named/anonymous failure text. The first full run hit the known
watcher-timing flake in
`native_events_follow_external_writes_save_echoes_multiple_owners_and_new_generations`;
the isolated rerun passed and the authoritative full rerun is green at gate736
with Clippy/build pass (`browser-download-feedback-final-*`). Richer download
management, popup edge cases and live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Locale-aware Vite preview ordering (2026-11-01)

Source sorts preview inventories with `localeCompare`, but the engine used raw
byte ordering. Preview servers now sort with a locale-aware case-insensitive
comparison: macOS delegates to Foundation's
`localizedCaseInsensitiveCompare`, other platforms use Unicode case folding.
Case variants of the same text order lowercase-first to match JavaScript's
default collation in common locales, and the comparison falls back to byte
order for deterministic results. Directories compare first, then entries;
sorting a list twice is idempotent.

The preview test covers locale ordering (`a` before `B`), lowercase case
tie-breaks, mixed-case directories and entries, directory/entry tiers and
idempotence. All preview tests pass. Latest complete gate737 with Clippy/build
pass (`preview-locale-sort-final-*`). Relocation/release validation and visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Locale-aware HTML entry search (2026-11-01)

Source's recursive HTML entry search sorts each directory with
`localeCompare` before selecting the shallowest fallback page. Native still
used raw filename bytes, so an uppercase file such as `Zeta.html` beat
`alpha.html` despite Source choosing the lowercase name. The preview search now
uses the shared localized/case-folded comparator for files and directories,
including same-depth directory traversal. `index.html` still wins immediately;
hidden and dependency directories remain skipped.

A focused test covers mixed-case root files and mixed-case same-depth
directories, asserting the lowercase entry wins and is returned with its
directory. All preview tests pass. Latest complete gate738 with Clippy/build
pass (`preview-entry-locale-final-*`). Relocation/release validation and visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Persistent browser download history and shelf (2026-11-01)

Browser downloads now have a persistent in-app history in addition to toast
feedback. Completed and failed entries are stored newest-first in
`browser-downloads.json` under the app data directory, loaded at startup,
capped at 20 and ignored when corrupt. Names prefer the saved filename, then
the URL-derived name. The browser toolbar exposes a download count; opening it
renders a shelf with up to three recent rows (success/failure icon, filename and
status), an older-count line, and Clear/Close actions. The native webview bounds
shrink while the shelf is open so the list is not hidden behind the guest.

Module tests cover save/load, cap, corruption, name fallback; the rendered test
covers opening the tray, listing both statuses and clearing it. Latest complete
gate741 with Clippy/build pass (`browser-download-tray-final-*`). Popup edge
cases and live/visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Explicit browser popup policy edge cases (2026-11-01)

Guest new-window requests now go through an explicit
`PopupAction::{External, Deny}` boundary used by the wry handler. Only
normalized credential-free HTTPS URLs with a host are handed to the OS; every
popup is still denied as a native window, matching Source's
`setWindowOpenHandler` behavior. Tests now cover whitespace trimming,
case/host normalization, default and non-default ports, IPv6 hosts, empty
usernames, username/password credentials, empty/malformed/relative URLs,
over-range ports and the full denied scheme set (HTTP, file, data, javascript,
blob, mailto, about, custom).

All browser tests pass. Latest complete gate742 with Clippy/build pass
(`browser-popup-policy-final-*`). Live/paired visual acceptance remains; live
OAuth/usage refresh and mobile remain separate open items.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Browser permission isolation script (2026-11-01)

The native guest already denied media capture through wry's permission
handler, but WebKit still exposes geolocation, notifications, clipboard,
device enumeration and permission queries to page scripts. A document-start
initialization script now runs in every frame and reports those APIs as
unavailable or denied before page code executes:

- geolocation reports PERMISSION_DENIED,
- Notification.permission is `denied` and requestPermission resolves denied,
- getUserMedia/getDisplayMedia reject with NotAllowedError and enumerateDevices
  returns no devices,
- clipboard read/write rejects,
- Permissions.query returns a denied status,
- USB, Bluetooth, serial, HID, MIDI, share and wakeLock are hidden.

The script syntax was checked with Node and a focused test asserts every
sensitive API is covered. Latest complete gate743 with Clippy/build pass
(`browser-permission-isolation-final-*`). Full platform permission/live
isolation acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Vite preview relocation labels (2026-11-01)

Preview stop menus stripped the current workspace root and fell back to an
empty prefix when a server no longer lived under that root. A relocated or
sibling server therefore looked like a workspace-relative entry, hiding its
stale directory. Labels now mirror Source's relative-server behavior: root
entries show only the HTML entry, entries under the current root show their
relative path, and servers under an old or sibling root show the full directory
(with trailing separators normalized). Port release on child exit/Stop remains
covered by the existing lifecycle tests.

A focused test covers root, relative, sibling-prefix, old-root and
trailing-separator directories. Latest complete gate744 with Clippy/build pass
(`preview-relocation-final-*`). Live preview acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Browser toolbar keyboard accessibility (2026-11-01)

The browser back, forward, reload/stop, downloads and close controls were
mouse-only divs, unlike Source's real buttons. Each now has a role, accessible
label, focus handle and tab stop; pointer activation focuses the control, and
Enter/Space invoke the same action. The downloads toggle uses the same focus
path, and the nav helper exposes a stable debug selector.

A focused rendered test covers pointer activation and Enter/Space activation
through a probe button. Latest complete gate745 with Clippy/build pass
(`browser-toolbar-a11y-final-*`). Paired visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown code-fence syntax highlighting (2026-11-01)

Markdown code blocks now paint syntax-colored runs instead of a single plain
style. A bounded, dependency-free tokenizer covers JSON, JavaScript,
TypeScript, JSX/TSX aliases, CSS/SCSS/Less, HTML/XML, Python, Bash, YAML, SQL,
Diff and Markdown. Tokens mark keywords, strings, numbers, comments, functions,
properties, tags/attributes and diff added/removed lines using GitHub
light/dark-style palette values. `PreformattedText` now accepts complete
gap-free styled runs while preserving tab stops, wrapping and intrinsic width;
tabs, unknown languages and plain fences fall back to the original single run.

Tests cover language aliases, JSON/script/CSS/HTML/Python/Bash/YAML/SQL/Diff
tokens, span ordering/UTF-8 boundaries, markdown run coverage and tab/unknown
fallbacks. Latest complete gate750 with Clippy/build pass
(`markdown-code-highlight-final-*`). Full Shiki grammar parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Inline code path/URL highlighting (2026-11-01)

Inline code now follows Source's `inlineCodeKind` behavior. HTTP(S) spans are
classified as URLs; filesystem-looking spans (slashes, relative prefixes,
common extensions or known filenames) are paths and use the accent color in
their styled runs. Plain inline code keeps normal ink. The classifier rejects
single-slash commands, whitespace, punctuation/operator spans, non-HTTP
schemes and version-like text.

The classifier has focused edge-case coverage (paths, URLs, schemes, commands,
versions, punctuation, whitespace, known filenames), and a markdown run test
checks accent-vs-ink coloring and the code font. Latest complete gate752 with
Clippy/build pass (`markdown-inline-code-final-*`). Full Linguist
extension/filename coverage and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Native Markdown math rendering (2026-11-01)

Inline and display math no longer fall back to raw TeX text. Inline math is
shaped with the math font; display math renders through a centered, labeled
`math-block` element. A bounded TeX-to-Unicode renderer covers Greek and
operator symbols, superscripts/subscripts, fractions, roots, text/style groups,
left/right delimiters and spacing commands, with readable fallbacks for unknown
commands and unmatched groups.

Parser tests cover symbols, scripts, fractions, roots, text groups,
delimiters, unknown commands and unmatched braces. Markdown tests check the
inline math font/rendered scripts and the display math block. Latest complete
gate756 with Clippy/build pass (`markdown-math-final-*`). Full KaTeX layout,
accessibility and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Newest provider-usage snapshot selection (2026-11-01)

`read_provider_usage_snapshot` documented "newest usable snapshot" but returned
the first readable valid candidate. It now parses every candidate and selects
the highest valid `generatedAt`, independent of XDG/macOS path order.
Malformed, stale and empty candidates are skipped and cannot displace an older
valid snapshot; equal timestamps retain the first candidate. Fixtures cover both
candidate orderings and malformed/empty fallback.

Latest complete gate757 with Clippy/build pass
(`provider-usage-newest-final-*`). Live OAuth/key provider fetches, environment
fallbacks and paired visual acceptance remain blocked on credentials/live
validation.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Math matrices, accents, spacing and source annotations (2026-11-01)

The bounded native math renderer now handles common matrix environments
(pmatrix/bmatrix/vmatrix/Bmatrix/cases/array) as row/cell text, accent commands
(hat/bar/vec/dot/ddot/tilde/overline/underline) with combining marks, and TeX
spacing (`~`, `\,`, `\:`, `\quad`, `\qquad`, `\hspace`). Parsing remains a
readable subset rather than a full KaTeX layout engine.

Inline and display math accessibility now exposes the rendered text as the
label and the original TeX as a `TeX: …` description. Inline math tracks its
source spans through the styled-run pipeline. Parser tests cover matrices,
accents and spacing; markdown tests cover inline source tracking and display
block rendering. Latest complete gate758 with Clippy/build pass
(`markdown-math-annotations-final-*`). Full KaTeX layout and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Per-entry browser download removal (2026-11-01)

The persistent download shelf gained per-row Remove actions in addition to
Clear/Close. Removal updates `browser-downloads.json` immediately, retains the
other entries, and closes the shelf when the final row is removed. Unknown ids
are no-ops in the history helper. Module tests cover remove-by-id and unknown
ids; the rendered tray test removes one of two entries and verifies the other
remains before clearing.

Latest complete gate759 with Clippy/build pass
(`browser-download-tray-actions-final-*`). Live/visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown link handoff policy (2026-11-01)

Inline/text and image links no longer pass arbitrary schemes to the OS opener.
They now use Source's `safeExternalUrl` policy: only credential-free HTTPS URLs
with a host are handed off, while HTTP, credentialed, javascript/data/file/
mailto/custom and malformed URLs remain in-app. Link styling and interaction
remain unchanged; only the external handoff is filtered.

The policy test covers accepted/normalized HTTPS and every blocked case.
Latest complete gate760 with Clippy/build pass
(`markdown-link-policy-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Reveal downloaded files from the tray (2026-11-01)

Download shelf rows with an absolute saved path now expose a Show action. It
runs the platform file-manager reveal command (`open -R` on macOS, Explorer
`/select` on Windows, `xdg-open` on the parent on other Unix systems); relative
or absent paths expose no action, and a failed spawn raises an error toast.
The platform command builder is unit-tested, and the rendered tray test seeds a
path-backed row and verifies Show appears only for that row.

Latest complete gate761 with Clippy/build pass
(`browser-download-reveal-final-*`). Live/visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Code-fence grammar fixture fixes (2026-11-01)

Fixture snippets exposed several tokenizer boundaries that were too broad or
missing. Numeric scanning now stops at `a-1` while accepting exponent signs;
JavaScript regex literals are recognized in expression positions; Python
identifiers no longer consume hyphens; Bash handles `${…}`, `$#`, `$?` and
other special variables; CSS marks properties only inside declaration blocks,
so `a:hover` is not mistaken for a property; and HTML tag/attribute names keep
hyphens.

A focused fixture test covers these cases and checks span ordering/UTF-8
boundaries. Latest complete gate762 with Clippy/build pass
(`markdown-grammar-fixtures-final-*`). Full Shiki grammar parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Template, custom-property and embedded-content highlighting (2026-11-01)

JS template literals now split literal chunks from `${…}` expressions; inner
expressions are highlighted with the JS grammar (nested strings/comments/braces
included) while literal text remains string-colored. CSS custom properties
(`--name`) are marked on declarations but not inside `var()` references.
HTML `<script>` and `<style>` blocks now run the JS/CSS tokenizers on their
contents while retaining tag/attribute highlighting.

Fixture assertions cover template interpolation, custom properties and
embedded script/style tokens plus span integrity. Latest complete gate762 with
Clippy/build pass (`markdown-grammar-embedded-final-*`). Full Shiki grammar
parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## YAML block scalars and SQL quoted identifiers (2026-11-01)

The code-fence tokenizer now understands YAML block scalars. A `|` or `>`
header with optional indentation/chomping modifiers opens a scalar; following
indented lines (blank lines included) are one string span until the indentation
dedents, so `#`, keys and numbers inside scalar content are not re-tokenized.
SQL now treats double-quoted, backtick and bracket delimited identifiers as
property tokens, honoring doubled-delimiter escapes (`""`, ````, `]]`), while
single-quoted strings remain strings with `''` escapes.

A focused fixture test covers both scalar styles/modifiers and dedent, literal
content, each SQL identifier form and UTF-8 span integrity. Latest complete
gate763 with Clippy/build pass (`markdown-yaml-sql-final-*`). Full Shiki grammar
parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Provider usage refresh control and cache total (2026-11-01)

The session-usage popup's Provider usage header now hosts a labeled refresh
control, matching Source's placement. It re-requests the provider bridge
snapshot and is visually disabled while a request is loading. The empty state
uses Source's "No providers connected…" wording, and the token breakdown adds
the missing Cache total row with its rounded share of total tokens.

A rendered provider-section test verifies the host refresh control is placed in
the header and invokes its callback; pure tests cover cache summary/source
wording. Latest complete gate765 with Clippy/build pass
(`provider-usage-refresh-final-*`). Live credential refresh and paired visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Engine reconnect after service stop or port move (2026-11-01)

The transport previously retried a dead OpenCode URL forever. The SSE reader now
emits a `server.disconnected` marker whenever an opened stream ends (suppressed
for deliberate cancellation). The engine moves to Connecting, polls the
configured service registration, builds a fresh authenticated client, cancels
the old event stream, starts a replacement, and rehydrates sessions,
permissions and open views through the normal connection path. `server.connected`
from a recovering transport also restores Ready.

A new engine-flow fixture moves the service registration to a second port and
then produces a second outage on that same endpoint. It asserts rehydration
calls on the new server, a Ready connection after each outage, and exactly one
replacement event stream per outage. Repeated full-suite runs intermittently hit
the known `end_to_end_replay_stream_prompt_and_permission` Sass timing flake
under parallel load; isolated reruns passed and the latest authoritative full
rerun is green at gate766 with Clippy/build pass
(`engine-reconnect-final-*`). Renderer reload UX and live daemon/restart
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Renderer-reload equivalent for reconnected engines (2026-11-01)

The native UI now mirrors Source's renderer-reload recovery after a backend
interruption. A recovery banner appears whenever the engine is not Ready
(Connecting, Disconnected or Error) with the last connection error and a
labeled Reload action. Reload re-reads the session inventory and clears or
rebuilds UI-owned catalogs, provider usage, forms, inbox, command/reference
state, preview inventory and file views from the engine. When a disconnected
engine returns to Ready, the same reload runs automatically and consumes the
recovery flag; engine transcripts, drafts and scroll ownership remain intact.

Rendered tests cover banner visibility across Connecting/Ready/Error and the
Reload path clearing stale sessions and catalogs while the service is still
reconnecting. Latest complete gate768 with Clippy/build pass
(`connection-recovery-ui-final-*`). Live daemon/window crash acceptance and
paired capture remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Settle-then-highlight markdown fences (2026-11-01)

Source keeps fenced code on a plain surface while the fence is still streaming
and runs Shiki only after the block settles. Native now carries the transcript
block's streaming flag into Markdown rendering: streaming fences skip syntax
tokenization entirely, while settled fences keep the existing GitHub-style
highlighted runs, wrap/copy controls and preformatted layout. This avoids
re-tokenizing a growing fence on every chunk.

The code-fence run test now asserts streaming returns no highlighted runs and
that the same text highlights once settled. Latest complete gate768 with
Clippy/build pass (`markdown-streaming-fence-final-*`). Full Shiki grammar
parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Generic tool content file items (2026-11-01)

Tool results can carry `{type:"file", uri, name, mime}` items in their `content`
arrays. These are now parsed from replay tool parts and live tool completion
events into ordered `ToolFile` records on the transcript block. Expanded tool
cards render a labeled FILES section with a file glyph, name (falling back to
the URI), MIME text and stable per-file selectors. Source's ExternalLink policy
is preserved: only credential-free HTTPS URIs are handed to the OS; file and
other schemes stay labeled but inert.

Parser/live-event tests and the rendered tool-card fixture cover text+file
content splitting, URI/name/mime retention and file-row presence. The full gate
intermittently hit the known watcher and Sass parallel-load flakes; isolated
runs passed and the latest authoritative full rerun is green at gate769 with
Clippy/build pass (`tool-content-files-final-*`). Remaining tool
metadata/duration presentation and live visual acceptance stay open.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Nested tool metadata sub-calls (2026-11-01)

Tool metadata can carry `deepseek.subCalls`. These are now parsed from replay
tool parts and live `session.tool.progress` metadata into ordered `ToolSubCall`
records (id, title, detail, status, input/inputValue, output). Expanded tool
cards render a `TOOL CALLS · N` section with compact nested rows showing title,
optional detail and Done/Failed/Running status.

Parser tests cover nested status mapping, input/output fields and malformed
entries; the rendered tool-card fixture asserts nested-row presence alongside
file rows. The full gate intermittently hit the known Sass parallel-load flake;
the latest authoritative full rerun is green at gate769 with Clippy/build pass
(`tool-subcalls-final-*`). Full recursive nested tool cards and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Expandable recursive nested tool cards (2026-11-01)

Nested `metadata.deepseek.subCalls` now render as collapsible tool cards rather
than static rows. Each card keeps the nested title, optional detail and
Done/Failed/Running status visible, and clicking the head toggles an IN/OUT
payload region with Source's 6000-character display cap. Nested open state is
stored alongside the outer tool-card keys, so expansion survives transcript
redraws.

Parser tests still cover status mapping and input/inputValue fallbacks; the
rendered fixture now verifies nested cards start collapsed and click-to-expand
reveals the nested payload. The full gate intermittently hit the known watcher
flake; the latest authoritative run is green at gate769 with Clippy/build pass
(`tool-subcall-recursive-final-*`). Deeper recursive metadata trees and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Recursive nested tool metadata with files (2026-11-01)

Sub-call metadata now recurses fully. `ToolSubCall` carries its own `content`
file items and nested `subCalls`, accepted directly or under
`metadata.deepseek`, and parsing is bounded to eight levels to reject
pathological metadata. Expanded nested cards render their own FILES rows and
recursively nested TOOL CALLS cards, each independently collapsible with its
IN/OUT payload and stable selectors.

Parser/depth tests cover nested files, child calls and the recursion limit; the
rendered fixture verifies outer expansion, nested FILES, recursively nested
cards and child payload expansion. The full gate intermittently hit the known
watcher and Sass parallel-load flakes; the latest authoritative rerun is green
at gate770 with Clippy/build pass (`tool-subcall-deep-final-*`). Remaining
duration presentation and live visual acceptance stay open.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Tool status footer, duration and shell metadata fallbacks (2026-11-02)

Completed tool cards now render a Source-style Success/Failed footer with the
check/close glyph and the recorded elapsed duration (`Ns` below a minute, then
`Nm Ns`). Running/streaming tools keep their shimmer and omit the footer;
nested cards retain their per-call status rows. Shell tools also use
`metadata.command` and `metadata.output` when input/output fields are missing,
matching Source's shell presentation fallback chain.

Duration formatting, rendered status-footer presence and shell metadata
fallback tests cover the path. The full gate hit the known Sass engine-flow
flake once; the authoritative rerun is green at gate772 with Clippy/build pass
(`tool-duration-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Additional bounded math operators and environments (2026-11-02)

The native TeX subset now renders aligned/gathered/split environments,
`\binom`, log/trig/limit operators, optional `\sqrt[n]` indices, style switches,
phantom groups and empty `\left.`/`\right.` delimiters. Rendering remains bounded
Unicode text rather than full KaTeX layout, with unknown commands preserved as
readable fallbacks.

The math parser test covers every added construct while retaining existing
matrix, accent and spacing coverage; inline/display markdown math tests still
pass. The full gate hit the known watcher flake once; the authoritative rerun is
green at gate773 with Clippy/build pass
(`markdown-math-operators-final-*`). Full KaTeX layout/accessibility and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Answer-footer agent/model/duration metadata (2026-11-02)

Transcript blocks now carry the message's agent and model ids, stamped during
part-block assembly so replay and live updates produce the same footer. The
answer footer joins Source's metadata row: capitalized agent label, model name
resolved from the live catalog (id fallback) and turn duration, separated by
` · `. The metadata row disappears when agent, model and duration are all absent.

Pure formatting, block-stamping and rendered present/absent tests cover the
path. Full-gate runs intermittently hit the known watcher and Sass flakes; the
authoritative rerun is green at gate776 with Clippy/build pass
(`answer-footer-meta-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Keyboard-accessible browser download shelf actions (2026-11-02)

The download shelf's Show, Remove, Clear and Close controls now have dedicated
focus handles, tab stops, and Enter/Space activation alongside pointer clicks.
Show still performs the platform file-manager reveal, Remove/Clear update the
persisted history, and Close only hides the shelf.

A rendered test focuses Remove, removes an entry with Enter, then focuses Clear
and empties the shelf with Space. The full gate hit the known watcher flake once;
the authoritative rerun is green at gate777 with Clippy/build pass
(`browser-download-actions-a11y-final-*`). Popup edge cases and live/visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS variables, HTML entities and YAML anchors (2026-11-02)

Code-fence highlighting now recognizes SCSS `$variables` and Less `@variables`
as property tokens while known at-rules (`@media`, `@mixin`, etc.) stay keywords
and CSS custom properties remain declaration properties. HTML character
entities (`&amp;`, `&copy;`, numeric forms) are string tokens in text and tag
positions. YAML anchors, aliases, tags and merge keys receive property/keyword
tokens, with block-scalar content still treated as literal text.

A focused fixture test covers each token kind, variable reuse, entity counts and
anchor suppression inside `|` scalars, plus UTF-8 span integrity. The full gate
hit the known watcher flake once; the authoritative rerun is green at gate778
with Clippy/build pass (`markdown-grammar-vars-entities-anchors-*`). Full Shiki
grammar parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Python decorators and f-string interpolation (2026-11-02)

Python code fences now color decorator names and recursively tokenize
interpolated f-string expressions. Single-letter and two-letter string prefixes
(`f`, `r`, `rf`, `fr`, `b`, etc.) are recognized, literal f-string chunks stay
string tokens, and `{...}` expressions use the Python grammar including nested
braces, quoted strings and comments. Raw f-strings preserve the same behavior.

A fixture test covers `@app.route`, f-string numbers/keywords and
raw-prefixed f-strings, plus UTF-8 span integrity. The full gate is green at
gate779 with Clippy/build pass
(`markdown-grammar-python-fstrings-*`). Full Shiki grammar parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CRLF-preserving editor documents (2026-11-02)

Editor documents now retain their original line endings. CRLF content is
normalized to LF in the internal buffer, so navigation, edits, undo and
formatting all operate consistently; saves re-encode LF back to CRLF. The
compare-and-write expectation keeps the raw on-disk text, avoiding false
conflicts on the first save. Clean external reloads re-detect CRLF/LF and update
the EOL used by subsequent saves.

A document test covers CRLF load normalization, editing/insertion, exact CRLF
save output, raw compare expectations and switching back to LF after an external
LF reload. Full-gate runs intermittently hit the known watcher/Sass flakes; the
authoritative rerun is green at gate780 with Clippy/build pass
(`editor-crlf-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor active-line number and gutter emphasis (2026-11-02)

Editor line numbers now follow Monaco's active-line presentation. The logical
line containing the caret gets a brighter, medium-weight number and a subtle
gutter background, updating when the caret moves between lines. Other line
numbers stay faint, and the existing full-row caret-line highlight is retained.

A rendered test creates a three-line document, places the caret on the second
line, verifies only that line number carries the active marker, then moves to the
third line and confirms the marker follows. The full gate hit the known watcher
flake once; the authoritative rerun is green at gate781 with Clippy/build pass
(`editor-active-line-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor bracket-pair colorization (2026-11-02)

Editor brackets now use Monaco-style pair colorization. A per-revision document
scan records each structural bracket's nesting depth after masking strings and
comments; closing brackets share their opener's depth. Each tab-expanded display
line is split into default and colored runs using repeating gold/orchid/blue
colors, while the existing matching-bracket background highlight still follows
the caret.

Pure tests cover alternating depths, string/comment exclusion, tab-offset
mapping and line clipping. Rendered coverage asserts the structural bracket set
and depths (including comment exclusion) as the caret moves across lines. The
full gate hit the known watcher flake once; the authoritative rerun is green at
gate783 with Clippy/build pass (`editor-bracket-colors-final-*`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JavaScript/TypeScript shebangs, decorators and private fields (2026-11-02)

JavaScript/TypeScript code fences now highlight shebang lines, decorator names,
private class fields and modern TypeScript modifier keywords. A leading `#!`
line is a comment, `@sealed` receives function coloring, `#count` receives
property coloring, and `abstract`, `accessor`, `asserts`, `infer`, `keyof`,
`override`, `using` and related modifiers are keywords. String/regex/template
handling remains unchanged.

A fixture test covers each token kind, decorator-with-arguments and span
integrity. Full-gate runs hit the known watcher flake once; the authoritative
rerun is green at gate784 with Clippy/build pass
(`markdown-grammar-script-final-*`). Full Shiki grammar parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS selectors and declaration-value highlighting (2026-11-02)

CSS/SCSS/Less fences now distinguish selectors from declarations. Class and id
selectors receive tag colors, known pseudo classes/elements are recognized even
after a descendant combinator, and attribute-selector names receive attribute
colors. Declaration values are tracked after the property colon so hex colors,
`url(...)` function names, units/numbers and `!important` are colored without
mistaking nested element selectors or URL schemes for properties.

A fixture test covers class/id/pseudo/attribute selectors, a nested `a:hover span`
rule, hex values, URL values, units and `!important`, plus span integrity. Full
gate runs hit the known watcher flake; the authoritative rerun is green at
gate785 with Clippy/build pass (`markdown-grammar-css-selectors-final-*`). Full
Shiki grammar parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Shell heredocs, ANSI-C strings and assignments (2026-11-02)

Bash code fences now highlight heredoc bodies, ANSI-C quoted strings,
assignment names and function definitions. `<<`/`<<-` headers accept bare or
quoted delimiters; body lines stay one string span through the terminator line,
`$'...'`/`$"..."` strings are single string tokens, `NAME=` assignments and
`name()` definitions receive property/function colors, and `#` only starts a
comment at a word boundary so `foo#bar` stays plain.

A fixture test covers each shape, a hash inside a word and UTF-8 span integrity.
The full gate hit the known watcher flake once; the authoritative rerun is green
at gate786 with Clippy/build pass (`markdown-grammar-bash-final-*`). Full Shiki
grammar parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## SQL dollar quotes, parameters, functions and column types (2026-11-02)

SQL code fences now highlight `$tag$...$tag$` and `$$...$$` dollar-quoted
strings as single string tokens, positional/named/placeholder parameters (`$1`,
`:name`, `@account`, `?`) as properties, identifiers followed by `(` as
functions, and common column types (integer, varchar, timestamp, jsonb, uuid,
etc.) as keywords. PostgreSQL `::` casts remain intact instead of reading the
second colon as a parameter.

A fixture test covers each shape, comments and UTF-8 span integrity. The full
gate is green at gate787 with Clippy/build pass
(`markdown-grammar-sql-final-*`). Full Shiki grammar parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML DOCTYPE, CDATA and processing instructions (2026-11-02)

HTML/XML fences now highlight document-level declarations. `<!DOCTYPE ...>`
and `<?...?>` processing instructions are single keyword spans, handling quoted
internals and DOCTYPE internal subsets; `<![CDATA[...]]>` is a single string
span so its contents are no longer reparsed as tags or attributes. Comments and
entities retain their existing behavior.

A fixture test covers all three declaration forms plus comments, entities and
UTF-8 span integrity. Full-gate runs hit the known watcher flake; the
authoritative rerun is green at gate788 with Clippy/build pass
(`markdown-grammar-html-final-*`). Full Shiki grammar parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown GFM URL and email autolinks (2026-11-02)

Markdown now autolinks bare GFM URLs and emails like Source's Marked
`gfm`/`breaks` configuration. `http(s)://`, `ftp://`, `www.` and plain email
addresses become inline link ranges; trailing punctuation and unbalanced
closing parentheses are trimmed while balanced parentheses stay in the URL.
Explicit links, images, inline code and fenced code are skipped. `www.` receives
Marked's `http://` href, and Source's HTTPS-only safe external-URL policy still
governs clickable handoff.

Core event tests cover URL/email links, code exclusion and balanced punctuation;
an inline-range rendering test verifies the displayed labels and destinations.
Full-gate runs hit the known watcher/Sass flakes; the authoritative rerun is
green at gate791 with Clippy/build pass (`markdown-autolink-final-*`). Full
Marked/GFM parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Autolink punctuation protection and soft-break parity (2026-11-02)

Bare URL/email autolinks now survive Markdown emphasis characters. Before
parsing, candidates outside code spans/fences, HTML tags, image/link labels and
explicit destinations are escaped for `_`, `*`, `~`, backticks and brackets, so
the parser yields one text run for URLs such as `https://example.com/a_b*c` and
addresses like `first_last@example.com`; the displayed text contains no escape
characters. Raw HTML attributes and explicit link destinations remain verbatim.

A rendered markdown test also locks Source's Marked `breaks: true` behavior: a
soft newline occupies a hard line rather than a space. The full gate hit the
known Sass engine-flow flake once; the authoritative rerun is green at gate794
with Clippy/build pass (`markdown-autolink-punctuation-final-*`). Full
Marked/GFM parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Python prefixed triple-quoted strings and f-strings (2026-11-02)

Python code fences now handle prefixed triple-quoted strings. `r"""..."""`,
`b'''...'''`, `fr"""..."""` and related prefixes stay single string spans across
embedded quotes and newlines. Triple-quoted f-strings close only at the matching
three-quote delimiter while still tokenizing `{...}` interpolation expressions;
escaped quotes and nested expression strings remain intact.

A fixture test covers raw triple strings, triple f-strings with numeric
interpolation, byte strings and UTF-8 span integrity. The full gate is green at
gate795 with Clippy/build pass (`markdown-grammar-python-triple-final-*`). Full
Shiki grammar parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Source inline math delimiters (2026-11-02)

Markdown now accepts Source's inline `\(...\)` math delimiters in addition to
the existing dollar math path. Conversion happens in the pre-parser line scan
outside inline code spans, fenced blocks and HTML tags, so `\(code\)` examples
stay literal while real formulas reach the native math renderer with their
`TeX:` source annotation. No display-math behavior changed.

Core event and rendered math-font tests cover delimiter handling and code
exclusion. The full gate hit the known Sass engine-flow and streaming-reveal
flakes; isolated reruns passed and the authoritative rerun is green at gate797
with Clippy/build pass (`markdown-math-delimiters-final-*`). Full KaTeX layout
and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## YAML document markers, directives and literals (2026-11-02)

YAML code fences now color document structure and scalar literals. `---` and
`...` document markers plus `%YAML`/`%TAG` directives are keywords, and `true`,
`false`, `yes`, `no`, `on`, `off`, `null` and `~` are literals outside keys,
quoted strings and block scalars. Existing key/property, anchor/alias/tag and
block-scalar behavior is unchanged.

A fixture test covers markers, directives, boolean/null forms, property
retention and UTF-8 span integrity. The full gate is green at gate798 with
Clippy/build pass (`markdown-grammar-yaml-literals-final-*`). Full Shiki grammar
parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Diff metadata line highlighting (2026-11-02)

Diff fences now distinguish patch metadata from changed lines. `diff --git`,
`index`, mode/rename/copy lines, similarity markers and the missing-newline
marker receive comment coloring; `---`/`+++` file headers and hunk headers stay
keywords, while added/removed lines keep their dedicated colors.

A fixture test covers each metadata form, headers, hunks, additions/removals and
UTF-8 span integrity. The full gate is green at gate799 with Clippy/build pass
(`markdown-grammar-diff-metadata-final-*`). Full Shiki grammar parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## ANSI code-fence highlighting (2026-11-02)

ANSI fences now follow Source's `ansi` highlighter path. SGR escape sequences
are stripped from the displayed text while the raw fence text remains the copy
payload. The tokenizer applies 16-color, 256-color and truecolor
foreground/background codes plus bold, dim, italic, underline, strikethrough and
reverse to shaped runs; escape bytes no longer occupy layout width.

Tokenizer, run-style and rendered code-surface tests cover SGR stripping, named
and table colors, decorations and cleaned layout. The full gate is green at
gate802 with Clippy/build pass (`markdown-ansi-final-*`). Full Shiki grammar
parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Terminal SGR dim and strikethrough (2026-11-02)

Terminal cells now preserve SGR 2 dim and SGR 9 strikethrough from the
emulator into styled rows. Dim multiplies foreground alpha, strikethrough adds
a matching rule, and SGR 0/22/29 reset them alongside the existing bold, italic,
underline, inverse and hidden attributes.

Emulator and terminal-row tests cover flag propagation, reset behavior and
byte-exact run boundaries. The full gate hit the known Sass engine-flow flake
once; the authoritative rerun is green at gate804 with Clippy/build pass
(`terminal-dim-strike-final-*`). Live terminal/TUI visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Terminal underline variants (2026-11-02)

Terminal cells now preserve alacritty's single, double, curly, dotted and
dashed underline variants through a new `UnderlineKind` instead of a boolean.
The kind flows from emulator cells into terminal run styling; GPUI rendering
maps single to 1px, double to 2px, curly to wavy, and dotted/dashed to the
nearest solid rule until richer painting is available.

Emulator and byte-exact row-run tests cover every SGR 4:x form and reset. The
full gate hit the known Sass engine-flow flake once; the authoritative rerun is
green at gate806 with Clippy/build pass
(`terminal-underline-variants-final-*`). Exact dotted/dashed painting and live
terminal acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Exact terminal double/dotted/dashed underline painting (2026-11-02)

Terminal underline variants now paint exactly where GPUI's single underline
style cannot express them. Single uses the shaped solid rule and curly the wavy
rule; double paints two 1px per-cell segments, dotted paints 1px dots every 3px,
and dashed paints 3px/2px segments. `UnderlineKind` still flows from emulator
cells through terminal run styling.

Emulator, row-run and rendered segment tests cover every SGR 4:x variant and
reset, including segment counts and differing double-rule heights. Full-gate
runs hit the known watcher flake; the authoritative rerun is green at gate807
with Clippy/build pass (`terminal-underline-painting-final-*`). Live terminal
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Terminal SGR 58/59 underline color (2026-11-02)

Terminal cells now carry explicit underline colors from SGR 58 through indexed,
truecolor or named alacritty colors. Terminal run styling preserves the color;
shaped solid/wavy underlines and exact double/dotted/dashed segments paint with
it, and SGR 59 restores the default foreground color.

Emulator and terminal-row tests cover explicit truecolor, reset and run color,
and the rendered segment fixture now includes an underline color. Full-gate runs
hit the known Sass engine-flow and streaming-reveal flakes; the authoritative
rerun is green at gate809 with Clippy/build pass
(`terminal-underline-color-final-*`). Live terminal acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Math blackboard, boxed, modulo and color commands (2026-11-02)

The bounded math renderer now covers additional common KaTeX commands:
`\mathbb` maps common sets to double-struck Unicode, `\boxed` wraps content in
brackets, `\pmod`/`\bmod`/`\mod` render modulo wording, and
`\textcolor`/`\colorbox`/`\fcolorbox`/`\color` consume color arguments while
keeping the readable text. Rendering remains bounded Unicode rather than full
KaTeX layout.

A parser test covers each command and the existing math suite still passes. Full
gate runs hit the known watcher flake; the authoritative rerun is green at
gate810 with Clippy/build pass (`markdown-math-font-commands-final-*`). Full
KaTeX layout/accessibility and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Expanded math relation, arrow and symbol coverage (2026-11-02)

The bounded math renderer now covers a wider common subset of KaTeX commands:
relations (`\ll`, `\gg`, `\sim`, `\simeq`, `\cong`, `\propto`, subsets,
supersets, precedence and turnstiles), arrows (`\mapsto`, `\Leftrightarrow`,
long/hook/up/down/diagonal arrows), big operators (`\iint`, `\iiint`, `\oint`,
`\bigcup`, `\bigcap`) and symbols (`\ell`, `\hbar`, `\aleph`, `\emptyset`,
`\angle`, `\square`, `\oplus`, `\otimes`, `\vdots`, `\ddots`). Unsupported
commands still remain readable.

A parser test covers the expanded set while retaining existing math behavior.
Full-gate runs hit the known watcher flake; the authoritative rerun is green at
gate811 with Clippy/build pass (`markdown-math-symbols-final-*`). Full KaTeX
layout/accessibility and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Math over/under braces and directional arrow accents (2026-11-02)

The bounded math renderer now covers over/under braces and directional arrow
accents. `\overbrace{x}^{n}` and `\underbrace{x}_{n}` append brace marks before
the existing script conversion, and
`\overrightarrow`/`\overleftarrow`/`\overleftrightarrow` plus under-arrow
variants apply combining marks. Unsupported commands remain readable.

A parser test covers each form and the rest of the math suite still passes. The
full gate is green at gate812 with Clippy/build pass
(`markdown-math-braces-final-*`). Full KaTeX layout/accessibility and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown fence structure and inline syntax highlighting (2026-11-02)

Markdown code fences now highlight their own document structure: heading
markers, blockquote/list/task markers, horizontal rules and fenced-code
delimiters are keywords; inline code is a string and link labels/URLs receive
function/string colors. Fenced bodies stay plain in this bounded grammar.

A fixture test covers headings, quotes, lists, tasks, rules, fences, inline code
and links with UTF-8 span integrity. The full gate is green at gate813 with
Clippy/build pass (`markdown-grammar-markdown-final-*`). Full Shiki grammar
parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown emphasis markers and inline HTML (2026-11-02)

Markdown fences now color emphasis markers (`**`, `__`, `*`, `_`, `~~`) and
inline HTML tags/attributes inside prose, while leaving underscores inside
words such as snake_case plain. Existing heading/list/quote/rule/fence/inline
code/link behavior is unchanged.

A fixture test covers markers, inline HTML, the snake_case exclusion and UTF-8
span integrity. The full gate is green at gate814 with Clippy/build pass
(`markdown-grammar-inline-final-*`). Full Shiki grammar parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML namespaced elements and unquoted attribute values (2026-11-02)

HTML/XML fences now handle namespaced element and attribute names containing
`:`/`.` and color unquoted attribute values such as `data-id=abc` alongside
quoted values. Boolean attributes remain attribute tokens.

A fixture test covers namespaced tags, hyphenated/namespaced attributes,
quoted/unquoted values and UTF-8 span integrity. The full gate is green at
gate815 with Clippy/build pass (`markdown-grammar-html-attrs-final-*`). Full
Shiki grammar parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor auto-closing pairs (2026-11-02)

The editor now auto-closes bracket and quote pairs. `()`, `[]`, `{}`, `""`,
`''` and backticks insert as a pair with the caret inside; typing an existing
closing character skips over it; Backspace removes an empty pair. Quotes only
auto-close at a token boundary, so word-internal apostrophes stay plain. The
logic is bypassed for marked/composition text and multi-character inserts.

Pure helper and rendered editor tests cover insertion, overtype and pair
erasure. The full gate is green at gate817 with Clippy/build pass
(`editor-auto-pairs-final-*`). Selection auto-surround and language-specific
quote suppression remain open, as does live visual acceptance.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor selection auto-surround and quote suppression (2026-11-02)

Editor auto-pairs now wrap selections: typing an opening bracket or quote over
a selection surrounds it and keeps the selected text selected. Quote
auto-closing is suppressed inside line/block comments and existing strings, and
Markdown disables quote pairing entirely; typed existing closers still
overtype and Backspace still removes empty pairs.

Pure helper and rendered editor tests cover wrapping, comment/string
suppression, Markdown behavior and retained selection. The full gate is green
at gate818 with Clippy/build pass (`editor-auto-pair-surround-final-*`). Exact
language-by-language Monaco quote policy and live visual acceptance remain open.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Terminal OSC 8 hyperlinks (2026-11-02)

Terminal cells now retain OSC 8 hyperlink URIs from alacritty and carry them
through line projection. Linked cells receive a default underline; only
credential-free HTTPS links are handed to the OS through Source's safe
external-URL policy, while other schemes remain visible but inert.

Emulator and terminal-row tests cover link attachment, reset and underline
rendering. Full-gate runs hit the known watcher and Sass engine-flow flakes;
the authoritative rerun is green at gate820 with Clippy/build pass
(`terminal-osc8-links-final-*`). Live terminal acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco-derived editor auto-pair language policy (2026-11-02)

Editor auto-pairing now follows Source's Monaco language configurations per
file extension: JavaScript/TypeScript bracket/quote rules with string/comment
exclusions, JSON's braces/brackets/double-quote-only set, HTML angle brackets
and all quotes, CSS-family comment/string suppression, Markdown's no-quote
bracket/angle set, and Python/Shell/YAML/SQL pair definitions. Selection
surrounding uses the matching language's surrounding pairs.

Policy/context and surrounding-pair tests cover the extension matrix and quote
rules. The full gate is green at gate820 with Clippy/build pass
(`editor-auto-pair-policy-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## YAML quoted keys (2026-11-02)

YAML fences now color quoted keys as properties when they precede a colon,
including single-quoted keys, list-item keys and flow-mapping keys. Quoted
values remain strings, and existing scalar/anchor/alias/tag/document-marker
behavior is unchanged.

A fixture test covers each key/value shape and UTF-8 span integrity. The full
gate is green at gate821 with Clippy/build pass
(`markdown-grammar-yaml-keys-final-*`). Full Shiki grammar parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor smart Enter indentation (2026-11-02)

Editor Enter now uses Monaco-style full auto-indent. Lines ending in
`{`/`[`/`(` add a two-space level, Python lines ending in `:` indent, HTML
opening tags indent, and a following closer moves to its own outdented line
with the caret on the indented blank line. Markdown/YAML keep plain indent
copying.

Pure and rendered tests cover braces, Python colons, HTML tags, nested
indentation and blank-line caret placement. The full gate hit the known watcher
flake once; the authoritative rerun is green at gate823 with Clippy/build pass
(`editor-enter-indent-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Python and Bash built-in highlighting (2026-11-02)

Python and Bash code fences now color common built-ins as function tokens.
Python covers `print`, `len`, `range`, `int`, `str`, collection/type helpers,
iteration helpers, `open`, `super` and common exception classes; Bash covers
`echo`, `printf`, `read`, directory/navigation built-ins, `test`,
`true`/`false`, process/job built-ins and declaration helpers.

Existing fixture assertions were extended for both sets. Full-gate runs hit the
known watcher flake; the authoritative rerun is green at gate823 with
Clippy/build pass (`markdown-grammar-builtins-final-*`). Full Shiki grammar
parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor focus-aware caret blinking (2026-11-02)

Editor carets now blink on a focus/window-aware 530ms cadence using the shared
terminal blink helper. The timer runs only while the editor has focus and the
window is active; it stays steadily visible when blurred, and typing/navigation
immediately restores visibility after the hidden phase.

A hidden-phase reset regression test covers the shared helper. Full-gate runs
hit the known streaming-reveal flake; the authoritative rerun is green at
gate824 with Clippy/build pass (`editor-cursor-blink-final-*`). Smooth opacity
fading versus Monaco's `cursorBlinking: smooth` and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco-style smooth editor caret fade (2026-11-02)

Editor carets now fade smoothly instead of hard-toggling. Each cycle holds full
visibility for about 530 ms, then fades over about 140 ms at 16 ms frames while
the editor is focused and the window active. Blur restores a steady visible
caret, and typing/navigation resets it visible for the next hold.

Pure tick and timed-entity tests cover the fade curve, reset and blur behavior.
Full-gate runs hit the known watcher flake; the authoritative rerun is green at
gate825 with Clippy/build pass (`editor-smooth-caret-final-*`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco-style smooth editor wheel scrolling (2026-11-02)

Editor wheel scrolling now follows Monaco's `smoothScrolling: true`. The native
list's immediate wheel target is captured, the viewport is rewound, and the
scroll advances in 16 ms frames with 35% easing until it reaches that target;
keyboard paging, caret reveal and other programmatic scrolling stay immediate.

A rendered timing test covers native-target rewind, first-frame movement and
convergence. Full-gate runs hit the known Sass engine-flow flake; the
authoritative rerun is green at gate826 with Clippy/build pass
(`editor-smooth-scroll-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor delayed quick suggestions (2026-11-02)

Editor quick suggestions now follow Source's `quickSuggestions` configuration
and 25ms delay. Typing an identifier character (including `_`, `$`, `.`)
schedules the document-word popup after 25ms; comments are excluded while
strings remain eligible, manual Ctrl+Space stays immediate, and intervening
keys cancel stale timers.

Pure trigger/comment tests and a rendered delayed-open test cover the path. The
full gate hit the known watcher flake once; the authoritative rerun is green at
gate828 with Clippy/build pass (`editor-quick-suggestions-final-*`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## SQL operator highlighting (2026-11-02)

SQL fences now color comparison, arithmetic and JSON/PostgreSQL operators
(`=`, `<>`, `>=`, `||`, `::`, `->>`, etc.) alongside keywords, strings,
identifiers, parameters and types. Line/block comments and parameter forms keep
precedence over operator detection.

A fixture test covers operators, comments, casts and UTF-8 span integrity. The
full gate hit the known watcher flake once; the authoritative rerun is green at
gate829 with Clippy/build pass
(`markdown-grammar-sql-operators-final-*`). Full Shiki grammar parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Language-aware editor quick suggestions (2026-11-02)

Editor quick suggestions now use language-aware word boundaries and triggers.
CSS/SCSS/Less retain hyphenated/custom-property tokens (plus SCSS `$` and Less
`@`), HTML/XML retain hyphen/colon/dot name punctuation, and only those
extensions treat `-` as a suggestion trigger; other languages keep the standard
identifier set. Completion ranges and document-word extraction share the same
rules.

Fixture tests cover word ranges, continuation completion and trigger rules.
Full-gate runs hit the known watcher flake; the authoritative rerun is green at
gate830 with Clippy/build pass
(`editor-quick-suggestion-languages-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## YAML flow-mapping keys (2026-11-02)

YAML flow mappings now color nested unquoted keys as properties when they
follow `{`/`,`; quoted flow keys use the same context. Top-level keys keep a
single property span, and boolean/null values remain keywords.

A fixture test covers nested/quoted flow keys, values and UTF-8 span integrity.
The full gate is green at gate831 with Clippy/build pass
(`markdown-grammar-yaml-flow-final-*`). Full Shiki grammar parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Caret-anchored completion popup (2026-11-02)

The editor completion popup now anchors below the caret line like Monaco
instead of the editor's bottom-left. Placement clamps within the editor and
flips above the caret when there is not enough space below. Rows use Source's
`suggestFontSize: 13` and `suggestLineHeight: 24`.

The rendered completion test now asserts the popup sits below the edited row.
The full gate hit the known watcher flake once; the authoritative rerun is green
at gate831 with Clippy/build pass (`editor-completion-popup-final-*`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Math big delimiters, cancel and text-style commands (2026-11-02)

The bounded math renderer now covers big delimiter commands (`\big`, `\Big`,
`\bigg`, `\Bigg`), cancel/strike fallbacks (`\cancel`, `\bcancel`, `\xcancel`,
`\sout`), text-style commands (`\textbf`, `\textit`, `\texttt`, `\textsf`,
`\emph`), math class wrappers and `\fbox`. Unsupported commands remain readable.

A parser test covers each form and the rest of the math suite passes. The full
gate is green at gate832 with Clippy/build pass
(`markdown-math-big-cancel-final-*`). Full KaTeX layout/accessibility and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Profile-accurate editor selection colors (2026-11-02)

Editor text selections and the active-line background now use the active
appearance profile's `selected` color, matching Source's Monaco theme mapping
for both `editor.selectionBackground` and `editor.lineHighlightBackground`. The
previous hard-coded accent selection fill and reduced-opacity line highlight are
removed.

A profile-color regression test covers the shared fill. Full-gate runs hit the
known watcher flakes; the authoritative rerun is green at gate833 with
Clippy/build pass (`editor-selection-colors-final-*`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## JavaScript/TypeScript JSDoc auto-close (2026-11-02)

The editor now auto-completes the JavaScript/TypeScript JSDoc opener. Typing
the third character of `/**` inserts ` */` and leaves the caret inside
`/** | */`. HTML/CSS/other languages and string contexts are unaffected.

Pure and rendered tests cover the language guard and insertion. Full-gate runs
hit the known watcher flake; the authoritative rerun is green at gate835 with
Clippy/build pass (`editor-jsdoc-pair-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML angle-pair auto-close vs surrounding policy (2026-11-02)

Editor `<`/`>` pairing now matches Monaco's configuration split. HTML/XML only
surrounds `< >` when wrapping a selection and does not auto-insert `>` on typing
`<`; Backspace does not erase a `< >` pair there. Markdown continues to
auto-close `<` because its own config lists it under auto-closing pairs.

Pure tests cover the policy distinction and erase behavior. Full-gate runs hit
the known watcher flake; the authoritative rerun is green at gate835 with
Clippy/build pass (`editor-html-angle-pair-final-*`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco comment commands (2026-11-02)

The editor now supports Monaco's comment commands. `Cmd+/` toggles line
comments using each language's configured token. CSS, HTML/XML and Markdown
have no line-comment token, so the same key falls back to their block comment,
matching Monaco's `toggleLineComment` command. `Shift+Alt+A` always toggles a
block comment, including an empty line where it inserts `/*  */` with the caret
inside.

Multi-line selections add comments at each non-blank line's aligned minimum
indent, remove a uniformly commented selection, ignore blank lines in mixed
selections, and collapse to one undo step. Pure and rendered tests cover
tokens, fallback, alignment, blank lines, block add/remove and undo.

The full gate hit the known watcher flake once; the authoritative rerun is
green at gate838 with Clippy/build pass (`editor-comment-toggle-final-*`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Composer/text-input selection colors (2026-11-02)

Shared native text inputs now paint selected text with Source's global
`::selection` rule: the active appearance's accent at 32% opacity. The previous
fixed blue fill is gone, so composer, find, rename and other native fields
follow every palette instead of only the default accent.

A pure palette test covers the accent channels and alpha for the default and
kitty palettes. The full gate is green at gate839 with Clippy/build pass
(`composer-selection-color-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco multi-cursor select-all and add-cursor commands (2026-11-02)

Multi-cursor editing now covers Monaco's `Cmd+Shift+L` select-all-occurrences
and `Cmd+Alt+Up`/`Cmd+Alt+Down` add-cursor-above/below.

Select-all keeps the original match primary. A collapsed caret expands to the
word and uses case-sensitive whole-word matching; a non-empty selection follows
the Find widget's case/whole-word options with literal occurrence matching
(ASCII case-insensitive where requested). Add-cursor translates every primary
and extra selection one logical line at the same grapheme column, deduplicates
targets, and supports selections as well as carets.

Pure and rendered tests cover matching options, grapheme-column translation,
multi-selection duplication and one-undo-step edits. The full gate hit the known
watcher flakes; the authoritative rerun is green at gate844 with Clippy/build
pass (`editor-multicursor-commands-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Rust fence grammar (2026-11-02)

Markdown fences now highlight Rust through a bounded native grammar: line and
nested block comments, attributes (`#[…]`), keywords/storage modifiers,
primitive and upper-camel types, lifetimes, `fn` definitions/calls, macros,
booleans, decimal/hex/octal/binary numbers with underscores and suffixes,
escaped strings, byte strings/chars, char literals and `r#"…"#` raw strings.

A new `Type` token uses GitHub-light/dark type colors. Fixture and palette tests
cover the grammar and color mapping. The full gate hit the known watcher flakes;
the authoritative rerun is green at gate846 with Clippy/build pass
(`markdown-rust-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Go fence grammar (2026-11-02)

Markdown fences now highlight Go through a bounded native grammar: line and
block comments, control-flow/storage keywords, primitive types, `type`
declarations, function definitions/method calls, built-ins through the call
heuristic, interpreted strings, raw backtick strings, rune literals,
booleans/nil/iota and decimal/hex/octal/binary/float/hex-float numbers with
underscores and imaginary suffixes.

A fixture test covers tokens and UTF-8 span integrity. The full gate hit the
known watcher flakes; the authoritative rerun is green at gate847 with
Clippy/build pass (`markdown-go-grammar-final-*`). Full Shiki parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Full Linguist inline-code classification (2026-11-02)

Inline-code path classification now uses Source's full Linguist-derived tables:
1,438 extensions, 301 exact filenames and 140 filename prefixes, plus the
`.d.ts` special case. This adds dotfiles such as `.gitignore`, prefixes such as
`Dockerfile.dev`, and the full extension set.

URL detection is now ASCII case-insensitive for HTTP(S), scheme rejection is
anchored to Source's regex, and single-segment `/name` text stays plain. The
fixture table covers uppercase URLs, schemes, dotfiles, prefixes, Windows paths,
forbidden punctuation and Source's numeric/non-path quirks.

The full gate is green at gate847 with Clippy/build pass
(`inline-code-linguist-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Composer mention-span reconciliation (2026-11-02)

Composer `@` mention spans now follow Source's `composer-completions.ts`
instead of stored insertion offsets. Trigger detection starts only when `@`
begins a whitespace-delimited token (so an earlier `@` owns the whole token),
and is suppressed while the caret is inside an existing mention span derived
from current text.

At send time, mention offsets are rebuilt from the trimmed composer text.
Mention-attached files whose `@rel` token was edited away are omitted; plain
staged files are still sent. Pure tests cover trigger/span rules and
stale-offset reconciliation, including multiple `@` characters in one token.

The full gate hit the known watcher flakes; the authoritative rerun is green at
gate849 with Clippy/build pass (`composer-mention-spans-final-*`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Java fence grammar (2026-11-02)

Markdown fences now highlight Java through a bounded native grammar: line and
block comments, annotations (including qualified names), keywords and module
declarations, primitive and upper-camel types, class/interface/enum/record
declarations, function/method calls, text blocks, escaped strings/chars,
booleans/null and decimal/hex/binary numbers with underscores, exponent and
type suffixes.

A fixture test covers tokens and UTF-8 span integrity. The full gate hit the
known watcher flakes; the authoritative rerun is green at gate850 with
Clippy/build pass (`markdown-java-grammar-final-*`). Full Shiki parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown C/C++ fence grammar (2026-11-02)

Markdown fences now highlight C and C++ through a shared C-family tokenizer:
line/block comments, preprocessor directives with quoted/angle headers, C/C++
keywords and primitive types, struct/union/enum/class declarations,
function/method/macro calls, prefixed escaped strings/chars, C++
`[[attributes]]`, raw strings with delimiters and apostrophe digit separators.

Fixture and UTF-8 span-integrity tests cover both languages. The full gate hit
the known watcher flake once; the authoritative rerun is green at gate852 with
Clippy/build pass (`markdown-c-cpp-grammar-final-*`). Full Shiki parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Ruby fence grammar (2026-11-02)

Markdown fences now highlight Ruby through a bounded native grammar: line and
`=begin`/`=end` block comments, keywords and built-ins, class/module/def names,
instance/class/global variables, symbols, single/double/backtick strings with
interpolation kept intact, percent literals, `<<~`/`<<-`/quoted heredocs,
booleans/nil and hex/binary/octal/decimal/float numbers with underscores and
rational/imaginary suffixes.

A fixture test covers tokens and UTF-8 span integrity. The full gate hit the
known watcher/preview flakes twice; the authoritative rerun is green at gate853
with Clippy/build pass (`markdown-ruby-grammar-final-*`). Full Shiki parity and
live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown PHP fence grammar (2026-11-02)

Markdown fences now highlight PHP through a bounded native grammar: `<?php`,
`<?=` and `?>` tags, `//`/`#`/`/* */` comments, keywords and primitive types,
class/interface/trait/enum and function declarations, method/static calls,
`$variables`, `#[attributes]`, single/double/backtick strings with
interpolation kept intact, `<<<` heredoc/nowdoc strings with indented or
semicolon-terminated markers, booleans/null and hex/binary/octal/decimal/float
numbers with underscores and exponents.

A fixture test covers tokens and UTF-8 span integrity. The full gate hit the
known watcher flakes once; the authoritative rerun is green at gate854 with
Clippy/build pass (`markdown-php-grammar-final-*`). Full Shiki parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Go to Line/Column (2026-11-02)

The editor now implements Monaco's Go to Line/Column command on `Ctrl+G`.
macOS keeps `Cmd+G` for Find Next, matching Monaco's platform bindings, and F3
remains Find Next everywhere.

The popup accepts `line`, `line,col`, `line:col` and `line#col`. Negative line
numbers count back from the end (`-1` is the last line), columns clamp to the
target line's graphemes, and the status label mirrors Monaco's valid/invalid
wording. Enter moves the caret and closes; Escape dismisses.

Pure and rendered tests cover parsing, offsets, opening, submission and
dismissal. The full gate hit the known watcher flake once; the authoritative
rerun is green at gate856 with Clippy/build pass (`editor-goto-line-final-*`).
Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Move Line Up/Down (2026-11-02)

The editor now supports Monaco's Move Line Up/Down on `Alt+Up`/`Alt+Down`. It
moves the caret's line or the selected whole-line block, preserves block bytes
and trailing-newline state, follows the moved block with the caret/selection,
and no-ops at the first/last boundary.

Monaco's context-sensitive re-indentation on moves remains a separate
unfinished behavior. Pure and rendered tests cover single-line, block,
selection, no-trailing-newline and boundary cases.

The full gate hit the known watcher flakes once; the authoritative rerun is
green at gate858 with Clippy/build pass (`editor-move-line-final-*`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Copy Line Up/Down (2026-11-02)

The editor now supports Monaco's Copy Line Up/Down on `Shift+Alt+Up` and
`Shift+Alt+Down`. It duplicates the caret's line or selected whole-line block
with Monaco's exact selection rules: copy-down inserts before the block and
shifts the selection with it, while copy-up inserts after the block and leaves
the cursor on the original line. Empty lines, multi-line blocks and
trailing-newline state are covered.

Pure and rendered tests mirror Monaco's own copy-line cases. The full gate hit
the known watcher flake once; the authoritative rerun is green at gate860 with
Clippy/build pass (`editor-copy-line-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Insert/Delete Line (2026-11-02)

The editor now supports Monaco's line insert/delete commands: `Cmd+Enter`
inserts a line below, `Cmd+Shift+Enter` inserts above with the current line's
indentation, and `Cmd+Shift+K` deletes the caret line or selected whole-line
block.

Delete-line handles following-newline removal, last-line preceding-newline
removal, empty documents, blank lines and multi-line selections. Language-
specific extra auto-indent on inserted lines remains separate.

Pure and rendered tests cover all boundaries and keybindings. The full gate hit
the known watcher flake once; the authoritative rerun is green at gate862 with
Clippy/build pass (`editor-line-insert-delete-final-*`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Join Lines (2026-11-02)

The editor now supports Monaco's Join Lines on macOS `Ctrl+J`, matching
Monaco's platform binding. It joins the caret line with the next line, dedents
the following line, inserts one space only when neither side already has
whitespace, collapses trailing whitespace to one space, ignores blank lines,
joins multi-line selections, and moves the caret/selection per Monaco's rules.
Last-line joins are no-ops.

Pure and rendered tests cover whitespace, blank lines, selections, caret
placement and boundaries. The full gate hit the known watcher flakes once; the
authoritative rerun is green at gate864 with Clippy/build pass
(`editor-join-lines-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Go to Bracket (2026-11-02)

The editor now supports Monaco's Go to Bracket on `Cmd+Shift+\`. It jumps to
the partner of a bracket at the primary caret, otherwise to the closing bracket
of the innermost enclosing pair, otherwise to the next structural bracket,
skipping strings and comments. Repeated presses alternate between partners.

Pure and rendered tests cover partners, enclosing pairs, next-bracket fallback,
unmatched brackets and comment masking. The full gate hit the known watcher and
engine-flow flakes twice; the authoritative rerun is green at gate866 with
Clippy/build pass (`editor-goto-bracket-final-*`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Indent/Outdent Line (2026-11-02)

The editor now supports Monaco's Indent Line / Outdent Line on `Cmd+]` /
`Cmd+[`. Indent adds two spaces to every selected line, skips empty lines in
multi-line selections and indents a lone empty line. Outdent removes one tab or
up to two leading spaces and skips unindented lines. Primary and extra
cursors/selections are remapped and remain selected.

Pure and rendered tests cover caret, selection, empty-line, tab and multi-cursor
mapping. The full gate hit the known watcher flakes once; the authoritative
rerun is green at gate868 with Clippy/build pass
(`editor-indent-outdent-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Go to Next/Previous Problem (2026-11-02)

The editor now supports Monaco's Go to Next/Previous Problem on `F8` /
`Shift+F8`. It sorts and deduplicates all current editor diagnostics (language,
auto-import and W3C/Sass validation), moves the primary caret to the
next/previous diagnostic with wraparound, reveals it and clears extra cursors.

Pure and rendered tests cover sorting, wrapping, empty diagnostics and shortcut
traversal. The full gate hit the known watcher flakes once; the authoritative
rerun is green at gate870 with Clippy/build pass
(`editor-problem-navigation-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Line auto-indent for Insert/Move Line (2026-11-02)

Move Line now reindents the moved block from its destination context, and
Insert Line Above/Below now inherit line-context auto-indent. Bounded rules
follow open `{`/`[`/`(` delimiters, Python `:` blocks and HTML open tags,
preserve relative indentation across multi-line moves, and remap selections
after indentation changes.

Pure and rendered tests cover insertion above/below, Python, HTML, single-line
and block moves into/out of blocks. The full gate hit the known watcher flakes
once; the authoritative rerun is green at gate872 with Clippy/build pass
(`editor-line-autoindent-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Retry settlement (2026-11-02)

Scheduled retry rows now settle when the turn ends instead of lingering.
`session.step.ended` clears retry state on completion, and non-retry
`session.status` phases (busy/idle/error) clear any scheduled retry.
Step-start, idle, failure and interrupt paths already did so.

Transcript tests cover step-end and busy-status settlement plus removal of the
Retry block. The full gate hit the known watcher and engine-flow flakes twice;
the authoritative rerun is green at gate873 with Clippy/build pass
(`retry-settlement-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Trim Trailing Whitespace (2026-11-02)

The editor now supports Monaco's Trim Trailing Whitespace on `Cmd+K Cmd+X`.
It trims trailing spaces/tabs across the document, skips the caret's line when
the caret sits at its end, starts trimming at the caret when it is inside
trailing whitespace, preserves whitespace inside multiline strings, and keeps
the caret mapped.

Pure and rendered tests cover caret, cursor-line, string and chord behavior. The
full gate hit the known watcher flakes once; the authoritative rerun is green
at gate875 with Clippy/build pass (`editor-trim-whitespace-final-*`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Regex-aware trailing-whitespace exclusion (2026-11-02)

Trim Trailing Whitespace now also protects JavaScript/TypeScript regex
literals. The regex scanner reuses the Markdown highlighter's string/template/
comment skipping, and unterminated regexes are capped to their own line so
later lines and division expressions still trim.

Pure and rendered tests cover unterminated and closed regexes, division and the
Cmd+K Cmd+X chord. The full gate hit the known watcher flakes once; the
authoritative rerun is green at gate876 with Clippy/build pass
(`trim-whitespace-regex-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Delete All Left/Right (2026-11-02)

The editor now supports Monaco's Delete All Left/Right. macOS `Cmd+Backspace`
deletes from the selection end back to the line start, or the previous line
break at column 1. `Ctrl+K` and `Cmd+Delete` delete to the line end, or the next
line break when already at the line end. Non-empty selections follow Monaco's
asymmetric rules, overlapping ranges merge, and primary/extra carets are
remapped.

Pure and rendered tests cover caret, selection, line-boundary and shortcut
cases. The full gate hit the known watcher flakes once; the authoritative rerun
is green at gate878 with Clippy/build pass (`editor-delete-all-final-*`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Kotlin fence grammar (2026-11-02)

Markdown fences now highlight Kotlin through a bounded native grammar: line
and nested block comments, annotations (including qualified names), keywords
and modifiers, class/interface/object/typealias declarations, fun names and
calls, upper-camel types, triple-quoted raw strings with interpolation intact,
escaped strings/chars, booleans/null and hex/binary/decimal/float numbers with
underscores and suffixes.

A fixture test covers tokens and UTF-8 span integrity. The full gate passed on
the first run at gate879 with Clippy/build pass
(`markdown-kotlin-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Swift fence grammar (2026-11-02)

Markdown fences now highlight Swift through a bounded native grammar: line and
nested block comments, attributes (including dotted names), keywords and
modifiers, actor/class/enum/extension/protocol/struct/typealias declarations,
func names and calls, upper-camel types, multiline strings, hash-delimited raw
strings, escaped strings with interpolation, compiler directives, booleans/nil
and hex/binary/octal/decimal/float numbers with underscores.

A fixture test covers tokens and UTF-8 span integrity. The full gate passed on
the first run at gate880 with Clippy/build pass
(`markdown-swift-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Dockerfile fence grammar (2026-11-02)

Markdown fences now highlight Dockerfiles through a bounded native grammar:
line-start `#` comments, case-insensitive instructions (FROM/AS/RUN/COPY/ENV/
EXPOSE/CMD and the broader Docker set), `$VAR` and `${VAR}` expansions as
properties, quoted strings, and line-start detection so instruction names
inside arguments stay plain.

A fixture test covers tokens and UTF-8 span integrity. The full gate passed on
the first run at gate881 with Clippy/build pass
(`markdown-dockerfile-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS/HTML semantic completion (2026-11-02)

The editor now offers bounded worker-equivalent semantic completion for
CSS/SCSS/Less and HTML/XML. CSS declaration contexts complete property names;
value contexts complete common keywords. HTML tag contexts complete tag names
until whitespace, then attribute names. Semantic candidates precede and
deduplicate against document words.

Pure tests cover property/value/tag/attribute contexts and deduplication; a
rendered test verifies the completion popup offers CSS properties. The full
`vela-ui` package passes (573 tests). Clippy and the app build pass.

The full workspace gate is temporarily blocked by host-level CPU starvation: a
stuck root `spindump` process has consumed a core for over a day, causing the
known timing-sensitive engine observation/engine-flow tests to exceed their
fixed timeouts. The watcher test passes when run alone. Re-run the workspace
gate once the host load clears (`editor-css-html-completion-final-workspace-attempt.txt`,
`editor-css-html-completion-watcher-isolated.txt`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown TOML fence grammar (2026-11-02)

Markdown fences now highlight TOML: `#` comments, table headers (including
array-table inner names), dotted/bare and inline-table keys as properties,
basic/literal and multiline strings, booleans and decimal/hex/octal/binary/
float numbers with underscores.

A fixture test covers tokens and UTF-8 span integrity. The full `vela-ui`
package passes (574 tests); Clippy and the app build pass. The workspace gate
remains host-blocked by the stuck root `spindump` process, so the timing-
sensitive engine tests still exceed their fixed timeouts
(`markdown-toml-grammar-final-workspace-tests.txt`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Host unblocked: gate 884 (2026-11-02)

The stuck root `spindump` process was cleared by a host reboot. After the
post-boot indexing load settled, the timing-sensitive watcher and engine-flow
tests passed, and the authoritative full workspace gate is green at
**884 passed, zero failures, one optional performance test ignored**
(`toml-css-final-workspace-tests.txt`).

Clippy and the app build also pass (`toml-css-final-clippy.txt`,
`toml-css-final-build.txt`). This validates the previously host-blocked
CSS/HTML semantic-completion and TOML grammar slices. Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown C# fence grammar (2026-11-02)

Markdown fences now highlight C# through a bounded native grammar: line/block
comments, `#region`-style directives, line-start attributes, keywords and
modifiers, class/struct/interface/enum/record/namespace/delegate declarations,
method/constructor calls, primitive and upper-camel types, verbatim/
interpolated/raw strings, escaped strings/chars, booleans/null and
hex/binary/decimal/float numbers with suffixes.

A fixture test covers tokens and UTF-8 span integrity. The full gate passed on
the first run at gate885 with Clippy/build pass
(`markdown-csharp-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Command palette and line transforms (2026-11-02)

The editor now has a bounded `F1` command palette over its implemented
commands, with case-insensitive filtering, arrow-key selection, Enter
execution, Escape dismissal and click support. It adds Monaco line transforms
Sort Ascending/Descending, Reverse Lines and Delete Duplicate Lines. A single
empty selection applies them to the whole document, line selections stay
scoped, and duplicate deletion keeps first occurrences.

Pure and rendered tests cover filtering, the transforms and palette execution.
The full gate hit the known watcher flakes once; the authoritative rerun is
green at gate887 with Clippy/build pass
(`editor-command-palette-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Case and transpose commands (2026-11-02)

The command palette now also exposes Monaco's Transform to Uppercase/Lowercase/
Title Case and Transpose Characters. Empty carets transform the word under the
caret while non-empty selections transform in place; Title Case lowercases and
capitalizes word boundaries while preserving apostrophe contractions.
Transpose swaps surrounding characters, crosses line ends with the newline,
advances the caret, skips non-empty selections and no-ops at the final line end.

Pure and rendered palette tests cover all four. The full gate passed on the
first run at gate888 with Clippy/build pass
(`editor-case-transpose-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Duplicate Selection and Select to Bracket (2026-11-02)

The command palette now also exposes Duplicate Selection and Select to Bracket.
Duplicate Selection copies selected text after itself and selects the copy, or
duplicates the caret line downward for an empty selection. Select to Bracket
selects the pair at the caret, the innermost enclosing pair, or the next
structural pair, skipping strings and comments.

Pure and rendered palette tests cover both. The full gate hit the known watcher
flake once; the authoritative rerun is green at gate890 with Clippy/build pass
(`editor-duplicate-select-bracket-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Tab/Shift+Tab indentation (2026-11-02)

Editor `Tab`/`Shift+Tab` now follows Monaco's split behavior: whole-line or
multi-line selections indent/outdent line blocks, carets advance to the next
2-space tab stop, and partial single-line selections are replaced with tab-stop
whitespace. Emmet still takes precedence for HTML/CSS expansions.

Pure and rendered tests cover line selections, shift-tab reversal, caret tab
stops and partial selections. The full gate passed on the first run at gate891
with Clippy/build pass (`editor-tab-indent-final-*`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Remove Brackets (2026-11-02)

The editor now supports Monaco's Remove Brackets on `Cmd+Alt+Backspace`, also
exposed in the command palette. It deletes the matching bracket pair at the
caret or active selection end, otherwise the innermost enclosing pair, keeps
the selection mapped, and does not use a next-bracket fallback.

Pure and rendered tests cover caret, full-selection and no-match cases. The
full gate passed on the first run at gate893 with Clippy/build pass
(`editor-remove-brackets-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco Go to Symbol in File (2026-11-02)

The editor now supports Monaco's Go to Symbol in File on `Cmd+Shift+O`. It
lists bounded document declarations in a filterable quick-pick panel; typing
filters by symbol preview/name case-insensitively, arrows navigate, Enter/Tab
jumps and focuses the editor, and Escape closes.

Pure and rendered tests cover filtering and jump-to-symbol. The full gate hit
the known watcher flake once; the authoritative rerun is green at gate894 with
Clippy/build pass (`editor-go-to-symbol-file-final-*`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Workspace symbol live filter (2026-11-02)

Workspace symbol search (`Cmd+T`) now has a live filter field. The initial
query is the word under the caret, typing re-runs the bounded cross-document
symbol search, arrows navigate, Enter/Tab emits the cross-file target and
refocuses the editor, and Escape closes.

Pure and rendered tests cover filtering and target emission. The full gate
passed on the first run at gate895 with Clippy/build pass
(`editor-workspace-symbol-filter-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Monaco word-case transforms (2026-11-02)

The command palette now also exposes Monaco's Transform to Snake, Camel, Pascal
and Kebab Case. Snake/Kebab separate lower-to-upper and acronym boundaries and
lowercase the result; Camel preserves acronym heads and lowercases a normal
first-word start; Pascal also maintains dot-separated segments and capitalizes
each boundary word. Underscore replacement follows Source's non-overlapping
regex scan.

Pure and rendered tests cover boundary cases and palette execution. The first
full run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate897 with Clippy/build pass
(`editor-case-styles-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown math stacked annotations and substacks (2026-11-02)

Markdown math now renders `\overset`/`\stackrel` above a base,
`\underset` below it, `\substack` rows joined with `; ` while preserving
nested groups and text commands, and `\boldsymbol`/`\bm`/`\pmb` unwrapped to
their argument.

Pure parser assertions and a rendered display-math test cover the forms. The
first full run hit the known watcher startup-gap flake; the isolated test and
the authoritative rerun are green at gate899 with Clippy/build pass
(`markdown-math-stack-final-*`). Full KaTeX layout and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Lua fence grammar (2026-11-02)

Markdown fences now highlight Lua: shebang/line/block comments, long-bracket
strings, control/logical/local keywords, built-in and dotted/colon library
calls, function declarations/calls, quoted strings, booleans/nil and
hex/decimal/float numeric forms.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Lua fence paints through the highlighted code body. The first full
run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate901 with Clippy/build pass
(`markdown-lua-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown R fence grammar (2026-11-02)

Markdown fences now highlight R: `#`/roxygen comments, raw strings,
single/double strings, control keywords, built-in calls, dotted function calls,
namespace calls, constants (`TRUE`/`FALSE`/`NULL`/`NA`/`Inf`/`NaN`) and
decimal/hex/exponent/complex numbers with `L`/`i` suffixes.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies an R fence paints through the highlighted code body. The first full
run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate903 with Clippy/build pass
(`markdown-r-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML attribute-value completion (2026-11-02)

Editor semantic completion now offers context-specific HTML attribute values
for `type`, `rel`, `target`, `method`, `autocomplete`, `inputmode`,
`crossorigin`, `decoding`, `dir`, `draggable`, `enctype`, `loading`, `scope`,
`shape`, `spellcheck`, `translate` and `wrap`. Values work in quoted and
unquoted values, completed values hand back to attribute-name completion, and
the tag/attribute catalog is broader.

Pure context tests and a rendered completion-popup test cover the path. The
first full run hit the known watcher startup-gap flake; the isolated test and
the authoritative rerun are green at gate905 with Clippy/build pass
(`editor-html-value-completion-final-*`). Full worker parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS property-aware value completion (2026-11-02)

Editor CSS semantic completion now offers property-aware values for
`display`, `position`, `cursor`, `overflow`, `font-weight`, `text-transform`,
`grid-auto-flow`, `pointer-events` and related properties, backed by a broad
generic fallback and a much larger property catalog.

Pure context tests and a rendered completion-popup test cover the path. The
first full run hit both known observation flakes; both passed in isolation and
the authoritative rerun is green at gate907 with Clippy/build pass
(`editor-css-value-completion-final-*`). Full worker parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS at-rule completion (2026-11-02)

Editor CSS semantic completion now offers at-rule names at the start of a
statement: `@charset`, `@container`, `@counter-style`, `@font-face`,
`@font-feature-values`, `@import`, `@keyframes`, `@layer`, `@media`,
`@namespace`, `@page`, `@property`, `@scope`, `@starting-style`, `@supports`
and `@viewport`. Declaration property/value completion remains unchanged.

Pure context tests and a rendered completion-popup test cover the path. The
first full run hit both known observation flakes; both passed in isolation and
the authoritative rerun is green at gate909 with Clippy/build pass
(`editor-css-at-rule-completion-final-*`). Full worker parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Scala fence grammar (2026-11-02)

Markdown fences now highlight Scala: line/nested block comments, annotations,
package/import and control/declaration keywords, class/trait/object/enum/type
declarations, `def`/call functions, upper-camel and primitive types,
triple/escaped strings, char and symbol literals, booleans/null and
hex/binary/decimal/exponent/suffixed numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Scala fence paints through the highlighted code body. The first full
run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate911 with Clippy/build pass
(`markdown-scala-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Dart fence grammar (2026-11-02)

Markdown fences now highlight Dart: line/nested block comments, annotations,
import/class/control keywords, class/enum/extension/mixin/typedef declarations,
called functions, primitive/upper-camel types, single/triple-quoted strings
with interpolation, booleans/null and hex/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Dart fence paints through the highlighted code body. The first full
run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate913 with Clippy/build pass
(`markdown-dart-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown PowerShell fence grammar (2026-11-02)

Markdown fences now highlight PowerShell: line/block comments, single/double/
here strings, `$variables` including scoped/global names, cmdlets and
hyphenated function calls/declarations, type and attribute brackets, control
keywords and hex/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a PowerShell fence paints through the highlighted code body. The first
full run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate915 with Clippy/build pass
(`markdown-powershell-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS value-function completion (2026-11-02)

Editor CSS value completion now offers property-aware functions: color
functions (`rgb(`, `hsl(`, `color-mix(`), image/gradient functions, filter
functions, transform functions, easing functions, and math/var helpers.
Property-specific keywords still sort before functions where both apply.

Pure context tests and a rendered completion-popup test cover the path. The
full gate passed on the first run at gate917 with Clippy/build pass
(`editor-css-function-completion-final-*`). Full worker parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Elixir fence grammar (2026-11-02)

Markdown fences now highlight Elixir: line comments, module attributes,
def/defp/defmacro/defmodule/defprotocol declarations, called and dotted module
functions, upper-camel aliases, heredocs, sigils, atoms, double/char-list
strings, booleans/nil and hex/binary/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies an Elixir fence paints through the highlighted code body. The full
gate passed on the first run at gate919 with Clippy/build pass
(`markdown-elixir-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML closing-tag completion (2026-11-02)

Editor HTML completion now uses the nearest unclosed ancestor for `</`
completion: typing `</di` inside `<div>` offers `div>` and accepting produces
`</div>`. Self-closing and void elements are ignored. With no open ancestor,
known HTML tags are offered.

Pure context tests and a rendered accept test cover the path. The full gate
passed on the first run at gate921 with Clippy/build pass
(`editor-html-close-tag-completion-final-*`). Full worker parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## HTML auto-close tag suggestion (2026-11-02)

Editor HTML completion now offers an auto-close suggestion after a completed
non-void start tag: `<div>` offers `</div>`. Typing `>` also triggers the 25ms
quick-suggestion path for HTML/XML. Void and self-closing tags are ignored.

Pure completion/trigger tests and a rendered accept test cover the path. The
full gate passed on the first run at gate922 with Clippy/build pass
(`editor-html-auto-close-tag-final-*`). Full worker parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Haskell fence grammar (2026-11-02)

Markdown fences now highlight Haskell: language pragmas, line/nested block
comments, module/import/control keywords, data/newtype/type/class/instance
declarations, top-level functions and common built-ins, upper-camel
types/constructors, strings/chars, booleans and hex/octal/decimal/exponent/
separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Haskell fence paints through the highlighted code body. The full
gate passed on the first run at gate924 with Clippy/build pass
(`markdown-haskell-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS custom-property completion (2026-11-02)

Editor CSS completion now collects document-defined custom properties
(`--name:`) from the current buffer, ignoring comments and strings. They appear
in property and value completion and are prioritized inside an open `var(`
call.

Pure completion tests and a rendered accept test cover the path. The full gate
passed on the first run at gate926 with Clippy/build pass
(`editor-css-custom-property-final-*`). Full worker parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown entity decoding (2026-11-02)

Markdown prose now decodes common named and numeric character references
(`&amp;`, `&lt;`, `&#35;`, `&#x1F600;`, etc.) before autolinking. Code spans,
fenced code and raw HTML keep literal entity text. Core event tests and a UI
flatten test cover prose/code distinctions, and a rendered prose-label test
covers the render path.

The first full runs hit the known observation startup-gap flake and the
engine-flow timeout; both passed when rerun in isolation, and the authoritative
full rerun is green at gate929 with Clippy/build pass
(`markdown-entity-decode-final-*`). Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Clojure fence grammar (2026-11-02)

Markdown fences now highlight Clojure: line comments, ns/control/special forms,
def/defn/defmacro/defmulti/defmethod names, hyphenated and namespaced symbols,
common built-ins, uppercase types, strings/chars, `:keywords`, booleans/nil and
hex/octal/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Clojure fence paints through the highlighted code body. The first
full run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate931 with Clippy/build pass
(`markdown-clojure-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Julia fence grammar (2026-11-02)

Markdown fences now highlight Julia: line/nested block comments, macros,
module/struct/abstract-type/function declarations, built-in calls, upper-camel
types, strings/chars/command literals, symbols, booleans and
hex/octal/binary/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Julia fence paints through the highlighted code body. The first full
run hit the known streaming-reveal timing flake; the isolated test and the
authoritative rerun are green at gate933 with Clippy/build pass
(`markdown-julia-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown OCaml fence grammar (2026-11-02)

Markdown fences now highlight OCaml: nested block comments,
let/let-rec/function definitions, type/module/struct/val declarations,
List/Array/String module calls and built-ins, upper-camel types and
constructors, normal and `{|…|}` strings, chars/type variables, booleans and
hex/octal/binary/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies an OCaml fence paints through the highlighted code body. The first
full run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate935 with Clippy/build pass
(`markdown-ocaml-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown table async image reflow (2026-11-02)

Table image sizing now shares the lazy-image cache key used by rendered
images. When an async image first resolves, the cache transition schedules the
transcript remeasure/notify path, so table columns reflow with the image
instead of waiting for another layout event.

A focused cache-transition test and the rendered mixed table-image test cover
the path. The first full run hit the known watcher startup-gap flake; the
isolated test and the authoritative rerun are green at gate936 with
Clippy/build pass (`markdown-table-image-reflow-final-*`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Erlang fence grammar (2026-11-02)

Markdown fences now highlight Erlang: `%` comments,
`-module`/`-export`/`-record`/`-spec` and related attributes, function clauses
and `module:function` chains, uppercase variables, built-in types,
normal/quoted-atom strings, `$char` literals, booleans and
base/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies an Erlang fence paints through the highlighted code body. The full
gate passed on the first run at gate938 with Clippy/build pass
(`markdown-erlang-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## CSS conditional-rule completion (2026-11-02)

Editor CSS completion now offers `@media` feature names inside `(...)` and
enumerated values after a feature colon (orientation, pointer/hover, color
scheme, reduced motion/contrast, forced colors, display mode,
scripting/update, overflow and more). Inside `@supports (...)`, CSS
property/value completion is reused.

Pure context tests and a rendered popup test cover the path. The first full
run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate940 with Clippy/build pass
(`editor-css-conditional-completion-final-*`). Full worker parity and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Elm fence grammar (2026-11-02)

Markdown fences now highlight Elm: line/nested block comments,
module/import/type/alias/case declarations, annotated and parameterized
top-level functions, standard module chains
(List/String/Maybe/Html/Browser/Cmd/Sub/etc.), upper-camel types and
constructors, strings/chars, True/False and hex/decimal/exponent/separated
numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies an Elm fence paints through the highlighted code body. The full gate
passed on the first run at gate942 with Clippy/build pass
(`markdown-elm-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown HCL/Terraform fence grammar (2026-11-03)

Markdown fences now highlight HCL/Terraform: `#`/`//`/block comments,
terraform/resource/data/provider/variable/output/module/locals and related
block keywords, attribute keys, `var`/`local`/`data`/`count`/`each`/`module`/
`path` references, lookup/format/collection functions, quoted strings,
heredocs, booleans/null and numeric forms.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies an HCL fence paints through the highlighted code body. The initial
Clippy/build invocation was interrupted mid-run; rerunning both completed
cleanly, and the full gate passed at gate944 with Clippy/build pass
(`markdown-hcl-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Nix fence grammar (2026-11-03)

Markdown fences now highlight Nix: `#`/block comments, let/inherit/in/rec and
control keywords, dotted attribute paths, import/map/builtins/lib functions,
quoted and indented strings, booleans/null and
hex/decimal/exponent/separated numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Nix fence paints through the highlighted code body. The full gate
passed on the first run at gate946 with Clippy/build pass
(`markdown-nix-grammar-final-*`). Full Shiki parity and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Puppet fence grammar (2026-11-03)

Markdown fences now highlight Puppet: `#`/block comments,
class/define/node/function/plan/type declarations, resource types, `=>`
attribute keys, `$variables`, include/template/sprintf/notice and related
functions, uppercase types, strings, booleans/undef and hex/decimal/exponent
numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a Puppet fence paints through the highlighted code body. The first
full run hit both known observation flakes; both passed in isolation and the
authoritative rerun is green at gate948 with Clippy/build pass
(`markdown-puppet-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown GraphQL fence grammar (2026-11-03)

Markdown fences now highlight GraphQL: `#` comments,
query/mutation/subscription/fragment/type/interface/union/enum/input/scalar/
schema/directive keywords, upper-camel operation/type/enum names,
`$variables`, `@directives`, normal/block strings, booleans/null and signed
decimal/exponent numbers.

A fixture test covers tokens and span integrity, and a rendered Markdown test
verifies a GraphQL fence paints through the highlighted code body. The first
full run hit the known watcher startup-gap flake; the isolated test and the
authoritative rerun are green at gate950 with Clippy/build pass
(`markdown-graphql-grammar-final-*`). Full Shiki parity and live visual
acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Vue fence grammar (2026-11-03)

Markdown fences now highlight Vue single-file components. The tokenizer
composes the HTML grammar, then highlights `:`/`@`/`v-` directive
attributes inside the outermost `<template>` range and paints
`{{ … }}` mustaches as expressions. `<script>`/`<style>` bodies reuse the
existing JavaScript/TypeScript and CSS grammars.

A fixture test covers embedded tags, directives, mustaches, scripts and
styles, plus span integrity, and a rendered Markdown test verifies a Vue
fence paints through the highlighted code body. The first Clippy run
rejected a nested `if` in the mustache scanner; collapsing it into a let
chain made Clippy/build pass (`markdown-vue-grammar-final-clippy.txt`,
`markdown-vue-grammar-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`markdown-vue-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate952 with 952 passed, zero failures and one ignored
performance test (`markdown-vue-grammar-final-workspace-tests.txt`). Full
Shiki parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Svelte fence grammar (2026-11-03)

Markdown fences now highlight Svelte components. `<script>`/`<style>`
bodies reuse the JavaScript/TypeScript and CSS grammars, directive
attributes (`on:`, `bind:`, `class:`, `use:`, `transition:`, etc.) stay
highlighted through the HTML grammar, and special tags
(`{#if}`/`{:else if}`/`{:else}`/`{#each}`/`{#await}`/`{@html}`/`{#key}`/
`{#snippet}`/`{/…}`) paint their keyword prefix. Plain `{…}`
interpolations paint their braces and tokenize the inner expression with
the JavaScript/TypeScript grammar. Unquoted directive expressions now
stay inside their tag so following attributes are still parsed.

A fixture test covers embedded tags, directives, special tags,
interpolations, scripts and styles, plus span integrity, and a rendered
Markdown test verifies a Svelte fence paints through the highlighted code
body. Clippy/build pass (`markdown-svelte-grammar-final-clippy.txt`,
`markdown-svelte-grammar-final-build.txt`). The first full run hit both
known observation flakes; both passed in isolation
(`markdown-svelte-grammar-final-isolated-flakes.txt`) and the
authoritative rerun is green at gate954 with 954 passed, zero failures and
one ignored performance test
(`markdown-svelte-grammar-final-workspace-tests.txt`). Full Shiki parity
and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor Expand Line Selection (2026-11-03)

The editor now supports Monaco's `expandLineSelection` action on `Cmd+L`
and in the command palette. Every invocation rewrites the primary and
each extra selection from the starting line's beginning to the start of
the next line, then stops at the document end. A selection ending on the
last line grows to the document end; a matching extra is removed so the
primary remains unique.

A pure range test covers repeated growth and mid-selection expansion, and
a rendered test drives three `Cmd+L` presses plus a second extra cursor
through the real key handler. Clippy/build pass
(`editor-expand-line-selection-final-clippy.txt`,
`editor-expand-line-selection-final-build.txt`). The first full run hit
both known observation flakes; both passed in isolation
(`editor-expand-line-selection-final-isolated-flakes.txt`) and the
authoritative rerun is green at gate955 with 955 passed, zero failures and
one ignored performance test
(`editor-expand-line-selection-final-workspace-tests.txt`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor fold commands (2026-11-03)

The editor now implements Monaco's folding keyboard commands over every
primary/extra selection start line: `Cmd+Alt+[` Fold, `Cmd+Alt+]` Unfold,
`Cmd+K Cmd+L` Toggle Fold, `Cmd+K Cmd+[`/`Cmd+K Cmd+]` Fold/Unfold
Recursively, `Cmd+K Shift+Cmd+L` Toggle Fold Recursively, `Cmd+K Cmd+0`
Fold All and `Cmd+K Cmd+J` Unfold All. Matching entries are exposed through
the `F1` palette. Fold targets the innermost uncollapsed containing region
or its first uncollapsed parent; unfold targets the innermost collapsed
region or its first collapsed parent. Recursive variants apply the same
state to the entire subtree, and all commands share the existing gutter
fold state and visible-line rebuild.

A pure target test covers inner-to-parent ordering and palette registration,
and a rendered test drives the real key handler through nested folds, extra
cursors, the `Cmd+K` chord, fold/unfold-all and both palette execution paths.
Clippy/build pass (`editor-fold-commands-final-clippy.txt`,
`editor-fold-commands-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`editor-fold-commands-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate957 with 957 passed, zero failures and one ignored
performance test (`editor-fold-commands-final-workspace-tests.txt`). Live
visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor folding category commands (2026-11-03)

The editor now implements the remaining Monaco folding actions:
`Cmd+K Cmd+/` Fold All Block Comments, `Cmd+K Cmd+8`/`Cmd+K Cmd+9`
Fold/Unfold All Regions, and `Cmd+K Cmd+-`/`Cmd+K Cmd+=` Fold/Unfold All
Except Selected. Matching entries are exposed through the `F1` palette.

Folding is now language-aware for block comments: JavaScript/TypeScript,
JSON/JSONC, CSS/SCSS/Less and HTML/XML emit multi-line comment ranges from
their configured comment tokens, while `#region` markers continue to produce
their own ranges. All Except Selected follows Monaco's
`setCollapseStateForRest` filter, preserving the innermost region at each
primary/extra selection start together with its ancestors and descendants
while applying the requested state to every unrelated fold.

A pure test covers block-comment gating by language, HTML/CSS comment folds,
region folds and except-selection containment; a rendered test drives the
real key handler through comment, region and except-selection commands plus
the block-comment palette path. Clippy/build pass
(`editor-fold-category-commands-final-clippy.txt`,
`editor-fold-category-commands-final-build.txt`). The first full run hit
both known observation flakes; both passed in isolation
(`editor-fold-category-commands-final-isolated-flakes.txt`) and the
authoritative rerun is green at gate959 with 959 passed, zero failures and
one ignored performance test
(`editor-fold-category-commands-final-workspace-tests.txt`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Provider usage snapshot environment (2026-11-03)

Provider usage candidates now match Source's environment behavior. A
non-empty `VELA_USAGE_SNAPSHOT` value replaces the default candidate list;
an empty value falls back to `XDG_DATA_HOME` (or `~/.local/share`) plus the
macOS application-support path. The candidate construction is now a pure
helper so override precedence, XDG/HOME fallbacks, the macOS path and absent
environment values are covered without mutating process environment.

Clippy/build pass (`provider-usage-snapshot-env-final-clippy.txt`,
`provider-usage-snapshot-env-final-build.txt`). The first full run hit both
known observation flakes; both passed in isolation. A later run hit the known
engine-flow timeout, which also passed in isolation
(`provider-usage-snapshot-env-final-isolated-flakes.txt`). The authoritative
rerun is green at gate960 with 960 passed, zero failures and one ignored
performance test (`provider-usage-snapshot-env-final-workspace-tests.txt`).
Live OAuth/key provider fetches and visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Astro fence grammar (2026-11-03)

Markdown fences now highlight Astro components. An opening `---` frontmatter
block tokenizes its TypeScript body between comment markers; HTML
tags/components and attributes reuse the HTML grammar; `<script>`/`<style>`
bodies reuse the JavaScript/TypeScript and CSS grammars; and `{…}` template
expressions paint braces plus TypeScript tokens. Frontmatter, script and
style braces stay untouched, and comments/attribute strings are masked before
expression detection.

A fixture test covers frontmatter, components, directives, expressions,
scripts, styles and span integrity, asserting that only template-expression
braces paint. A rendered Markdown test verifies an Astro fence paints through
the highlighted code body. Clippy/build pass
(`markdown-astro-grammar-final-clippy.txt`,
`markdown-astro-grammar-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`markdown-astro-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate962 with 962 passed, zero failures and one ignored
performance test (`markdown-astro-grammar-final-workspace-tests.txt`). Full
Shiki parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Zig fence grammar (2026-11-03)

Markdown fences now highlight Zig. The tokenizer covers line and
documentation comments, storage/structure/control/type keywords, `fn`
declarations and calls, uppercase types, `@builtin` functions, escaped
strings and char literals, line-leading `\\` multiline strings,
booleans/null, and hex/hex-float/decimal/octal/binary numbers with
underscores.

A fixture test covers comments, keywords, declarations, builtins, strings,
multiline strings and numeric forms plus span integrity. A rendered Markdown
test verifies a Zig fence paints through the highlighted code body.
Clippy/build pass (`markdown-zig-grammar-final-clippy.txt`,
`markdown-zig-grammar-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`markdown-zig-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate964 with 964 passed, zero failures and one ignored
performance test (`markdown-zig-grammar-final-workspace-tests.txt`). Full
Shiki parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Solidity fence grammar (2026-11-03)

Markdown fences now highlight Solidity. The tokenizer covers line, block and
NatSpec comments; pragma/import/control/declaration keywords;
contract/interface/library/struct/enum/event/error/type names;
functions/modifiers and calls; primitive-sized types (`uint256`, `bytes32`,
etc.); global variables/functions; time/currency constants; strings;
booleans; and decimal/hex/scientific numbers with underscores.

A fixture test covers comments, keywords, declarations, builtins, strings and
numeric forms plus span integrity. A rendered Markdown test verifies a
Solidity fence paints through the highlighted code body. Clippy rejected two
duplicate `else if` blocks; combining the conditions made Clippy/build pass
(`markdown-solidity-grammar-final-clippy.txt`,
`markdown-solidity-grammar-final-build.txt`). The first full run hit both
known observation flakes; both passed in isolation
(`markdown-solidity-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate966 with 966 passed, zero failures and one ignored
performance test (`markdown-solidity-grammar-final-workspace-tests.txt`). Full
Shiki parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown MDX fence grammar (2026-11-03)

Markdown fences now highlight MDX. YAML `---` and TOML `+++` frontmatter are
tokenized with their embedded grammars and string-coloured markers.
Line-leading `import`/`export` blocks reuse the TypeScript grammar, JSX/HTML
tags and attributes are highlighted, and Markdown structure, emphasis, inline
code and links remain. `{…}` expressions paint braces plus TypeScript tokens;
frontmatter, ESM and fenced-code braces are skipped.

A fixture test covers frontmatter, ESM, JSX, Markdown and expressions plus
span integrity, asserting that only MDX expressions paint braces. A rendered
Markdown test verifies an MDX fence paints through the highlighted code body.
Clippy/build pass (`markdown-mdx-grammar-final-clippy.txt`,
`markdown-mdx-grammar-final-build.txt`). The full run passed on the first
attempt at gate968 with 968 passed, zero failures and one ignored performance
test (`markdown-mdx-grammar-final-workspace-tests.txt`). Full Shiki parity and
live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Browser address normalization parity (2026-11-03)

Browser address normalization now follows Source's `browserAddress` decision
order. Explicit schemes are recognized only at the start of the trimmed value;
host-like input requires dot-separated ASCII `[\w-]` labels with an optional
numeric port; every other value becomes a DuckDuckGo search with
`encodeURIComponent`-compatible escaping. This changes non-numeric ports and
embedded scheme-like text from rejection to search, matching Source. The
existing credential-free HTTP(S)/`about:blank` policy still gates every result.

The focused browser test covers valid paths/ports, underscore hosts, invalid
and empty ports, embedded scheme text, explicit-scheme spelling preservation,
credentials, privileged schemes and malformed inputs. Clippy/build pass
(`browser-address-parity-final-clippy.txt`,
`browser-address-parity-final-build.txt`). The first full run hit the known
watcher startup-gap flake; it passed in isolation
(`browser-address-parity-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate968 with 968 passed, zero failures and one ignored
performance test (`browser-address-parity-final-workspace-tests.txt`).
Live/visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor Add Cursors to Line Ends (2026-11-03)

The editor now supports Monaco's `editor.action.insertCursorAtEndOfEachLineSelected`
on `Shift+Alt+I` and in the command palette. Non-empty selections are replaced
with cursors at the end of every covered line; if the selection ends past
column 1 on its final line, that exact selection end becomes another cursor.
Collapsed selections leave the editor unchanged, and duplicate targets are
removed while preserving generation order.

A pure target test covers collapsed, single-line, end-column, multi-line and
multi-selection cases. A rendered test drives the real key handler through a
multi-line selection, single-line selection and collapsed caret. Clippy
initially rejected one-element range arrays; a `std::iter::once` helper fixed
the tests. Clippy/build pass (`editor-add-cursors-line-ends-final-clippy.txt`,
`editor-add-cursors-line-ends-final-build.txt`). The first full run hit the
known watcher startup-gap flake; it passed in isolation
(`editor-add-cursors-line-ends-final-isolated-flakes.txt`) and the
authoritative rerun is green at gate970 with 970 passed, zero failures and one
ignored performance test
(`editor-add-cursors-line-ends-final-workspace-tests.txt`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor Add Cursors to Top and Bottom (2026-11-03)

The editor now supports Monaco's palette-only Add Cursors to Top and Add
Cursors to Bottom actions. Both replicate the primary selection's start/end
grapheme columns onto every line from the current line to the requested
document edge, clamp columns on shorter lines, keep the current line as the
primary selection and generate top cursors current-line first (descending
line order).

A pure target test covers bottom/top expansion, short-line clamping,
multi-line column replication and palette registration. A rendered test runs
both palette commands against real selections. Clippy/build pass
(`editor-add-cursors-top-bottom-final-clippy.txt`,
`editor-add-cursors-top-bottom-final-build.txt`). The first full run hit the
known watcher startup-gap flake; it passed in isolation
(`editor-add-cursors-top-bottom-final-isolated-flakes.txt`) and the
authoritative rerun is green at gate972 with 972 passed, zero failures and one
ignored performance test
(`editor-add-cursors-top-bottom-final-workspace-tests.txt`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor Add Selection to Previous Find Match (2026-11-03)

The editor now supports Monaco's palette-only Add Selection to Previous Find
Match action. A non-empty primary selection adds the nearest previous literal
occurrence before the earliest current selection without wrapping. A collapsed
caret first expands to and selects its word, matching Monaco's session start,
without adding another cursor.

A pure backward-search test covers single and multiple selections, no-wrap,
empty searches and palette registration. A rendered palette test covers
repeated leftward expansion, no-wrap and collapsed-caret word selection.
Clippy rejected one-element range arrays; a `std::iter::once` helper fixed the
test setup. Clippy/build pass (`editor-previous-find-match-final-clippy.txt`,
`editor-previous-find-match-final-build.txt`). The first full run hit the
known watcher startup-gap flake; it passed in isolation
(`editor-previous-find-match-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate974 with 974 passed, zero failures and one ignored
performance test
(`editor-previous-find-match-final-workspace-tests.txt`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor Move Last Selection to Find Match (2026-11-03)

The editor now supports Monaco's Move Last Selection to Next Find Match on
`Cmd+K Cmd+D` and Move Last Selection to Previous Find Match in the command
palette. The latest selection moves forward to the next literal occurrence, or
the earliest selection moves backward to the previous one; neither wraps. A
moved primary follows its replacement while later selections stay intact. A
collapsed caret first selects its word.

A pure forward-search test covers single/multiple selections, no-wrap, empty
inputs and palette registration. A rendered shortcut/palette test covers
multi-selection moves, lone-primary moves, preserved later selections and
collapsed-caret word selection. Clippy rejected one-element range vectors; a
`std::iter::once` helper fixed the test setup. Clippy/build pass
(`editor-move-find-match-final-clippy.txt`,
`editor-move-find-match-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`editor-move-find-match-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate976 with 976 passed, zero failures and one ignored
performance test
(`editor-move-find-match-final-workspace-tests.txt`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor Add Selection to Next Find Match palette action (2026-11-03)

The editor now exposes Monaco's Add Selection to Next Find Match through the
command palette alongside `Cmd+D`. Repeated invocations add successive literal
occurrences rightward without wrapping. A collapsed caret first selects its
word, then the next invocation adds the next occurrence; this now matches
Monaco's session start instead of immediately adding a match. The shared
forward-search helper also backs Move Last Selection to Next Find Match.

A pure forward-search/palette test covers single and multiple selections,
no-wrap and palette registration. A rendered palette test covers repeated
rightward expansion, no-wrap and collapsed-caret behavior. Clippy/build pass
(`editor-add-next-find-match-final-clippy.txt`,
`editor-add-next-find-match-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`editor-add-next-find-match-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate977 with 977 passed, zero failures and one ignored
performance test
(`editor-add-next-find-match-final-workspace-tests.txt`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Prisma fence grammar (2026-11-03)

Markdown fences now highlight Prisma. The tokenizer covers line,
documentation and block comments; generator/datasource/model/view/enum/
namespace/types blocks and their names; `type` aliases; line-start
field/config/enum names; scalar and upper-camel types; `@`/`@@` attributes;
function calls; strings; booleans; and numeric forms.

A fixture test covers comments, blocks, aliases, fields, attributes,
functions, strings and numbers plus span integrity. A rendered Markdown test
verifies a Prisma fence paints through the highlighted code body. Clippy/build
pass (`markdown-prisma-grammar-final-clippy.txt`,
`markdown-prisma-grammar-final-build.txt`). The full run passed on the first
attempt at gate979 with 979 passed, zero failures and one ignored performance
test (`markdown-prisma-grammar-final-workspace-tests.txt`). Full Shiki parity
and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Handlebars fence grammar (2026-11-03)

Markdown fences now highlight Handlebars. The tokenizer covers YAML
frontmatter, HTML tags/attributes, `{{! }}` and `{{!-- --}}` comments, block
start/end/else helpers, partials, variable paths, triple-stash expressions,
helper attributes/values, strings and numbers.

A fixture test covers frontmatter, HTML, comments, blocks, partials, variables,
attributes, strings and numbers plus span integrity. A rendered Markdown test
verifies a Handlebars fence paints through the highlighted code body.
Clippy/build pass (`markdown-handlebars-grammar-final-clippy.txt`,
`markdown-handlebars-grammar-final-build.txt`). The first full run hit the
known watcher startup-gap flake; it passed in isolation
(`markdown-handlebars-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate981 with 981 passed, zero failures and one ignored
performance test
(`markdown-handlebars-grammar-final-workspace-tests.txt`). Full Shiki parity
and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Liquid fence grammar (2026-11-03)

Markdown fences now highlight Liquid. The tokenizer covers HTML, `{% %}` tags
and `{{ }}` objects, comment/doc/raw blocks, embedded
style/stylesheet/schema/javascript bodies, control keywords, `name:`
attributes, `| filter` functions, variables, strings, numbers and operators.

A fixture test covers comments, tags, objects, filters, blocks, attributes,
properties, strings, numbers and embedded style/schema/javascript plus span
integrity. A rendered Markdown test verifies a Liquid fence paints through the
highlighted code body. Clippy/build pass
(`markdown-liquid-grammar-final-clippy.txt`,
`markdown-liquid-grammar-final-build.txt`). The full run passed on the first
attempt at gate983 with 983 passed, zero failures and one ignored performance
test (`markdown-liquid-grammar-final-workspace-tests.txt`). Full Shiki parity
and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Blade fence grammar (2026-11-03)

Markdown fences now highlight Blade. The tokenizer covers HTML, Blade
comments (`{{-- --}}`), directives, `{{ }}`/`{{{ }}}`/`{!! !!}` echoes, PHP
tags and embedded PHP expressions, `@php` and `@verbatim` blocks, directive
arguments, PHP variables, keywords, strings and numbers.

A fixture test covers comments, directives, echoes, PHP tags, `@php`,
`@verbatim`, variables, strings and numbers plus span integrity. A rendered
Markdown test verifies a Blade fence paints through the highlighted code body.
Clippy/build pass (`markdown-blade-grammar-final-clippy.txt`,
`markdown-blade-grammar-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`markdown-blade-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate985 with 985 passed, zero failures and one ignored
performance test (`markdown-blade-grammar-final-workspace-tests.txt`). Full
Shiki parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Templ fence grammar (2026-11-03)

Markdown fences now highlight Templ. The tokenizer covers Go declarations,
imports and control flow, `templ` declarations and function names, HTML
tags/attributes/comments, `@Component`/`@pkg.Call` expressions, brace
expressions, call expressions, Go types, strings and numbers.

A fixture test covers declarations, control flow, components, HTML,
comments, expressions, types, strings and numbers plus span integrity. A
rendered Markdown test verifies a Templ fence paints through the highlighted
code body. Clippy/build pass (`markdown-templ-grammar-final-clippy.txt`,
`markdown-templ-grammar-final-build.txt`). The full run passed on the first
attempt at gate987 with 987 passed, zero failures and one ignored performance
test (`markdown-templ-grammar-final-workspace-tests.txt`). Full Shiki parity
and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Pascal fence grammar (2026-11-03)

Markdown fences now highlight Pascal. The tokenizer covers case-insensitive
keywords and primitive types, function/procedure/constructor/destructor/
property declarations and names, `self`/`Result`, line/block/brace comments,
compiler directives, apostrophe-escaped strings, `#nn` character codes,
decimal/hex numbers, booleans and nil.

A fixture test covers comments, directives, keywords, declarations, variables,
types, strings and numeric forms plus span integrity. A rendered Markdown test
verifies a Pascal fence paints through the highlighted code body. Clippy/build
pass (`markdown-pascal-grammar-final-clippy.txt`,
`markdown-pascal-grammar-final-build.txt`). The first full run hit the known
watcher startup-gap flake; it passed in isolation
(`markdown-pascal-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate989 with 989 passed, zero failures and one ignored
performance test (`markdown-pascal-grammar-final-workspace-tests.txt`). Full
Shiki parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Makefile fence grammar (2026-11-03)

Markdown fences now highlight Makefile. The tokenizer covers comments,
variable assignments and operators, `$(...)`/`${...}` and automatic variables,
built-in functions, directives, targets/prerequisites and shell recipes.

A fixture test covers comments, assignments, variables/functions, directives,
targets and recipes plus span integrity. A rendered Markdown test verifies a
Makefile fence paints through the highlighted code body. Clippy rejected a
duplicate built-in property branch; removing it made Clippy/build pass
(`markdown-makefile-grammar-final-clippy.txt`,
`markdown-makefile-grammar-final-build.txt`). The first full run hit the known
watcher startup-gap flake; it passed in isolation
(`markdown-makefile-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate991 with 991 passed, zero failures and one ignored
performance test
(`markdown-makefile-grammar-final-workspace-tests.txt`). Full Shiki parity and
live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Twig fence grammar (2026-11-03)

Markdown fences now highlight Twig. The tokenizer covers HTML, `{# #}`
comments, `{% %}` statements and `{{ }}` print tags, block keywords,
filters/functions, hash attributes, variables/properties, strings, numbers and
raw/verbatim blocks.

A fixture test covers comments, statements, print tags, filters, functions,
hashes, properties, strings, numbers and verbatim/raw plus span integrity. A
rendered Markdown test verifies a Twig fence paints through the highlighted
code body. Clippy/build pass (`markdown-twig-grammar-final-clippy.txt`,
`markdown-twig-grammar-final-build.txt`). The first full run hit the known
watcher startup-gap flake; it passed in isolation
(`markdown-twig-grammar-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate993 with 993 passed, zero failures and one ignored
performance test (`markdown-twig-grammar-final-workspace-tests.txt`). Full
Shiki parity and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown Pug fence grammar (2026-11-03)

Markdown fences now highlight Pug. The tokenizer covers doctypes, comments,
tags/classes/ids/attributes, includes/extends/blocks, mixins and calls, flow
control, inline `#{}`/`!{}` interpolation, printed expressions and unbuffered
JS.

A fixture test covers doctypes, comments, tags, attributes, includes, mixins,
flow control, interpolation, expressions and span integrity. A rendered
Markdown test verifies a Pug fence paints through the highlighted code body.
Clippy rejected a single-element test loop; simplifying it made Clippy/build
pass (`markdown-pug-grammar-final-clippy.txt`,
`markdown-pug-grammar-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`markdown-pug-grammar-final-isolated-flakes.txt`) and the authoritative rerun
is green at gate995 with 995 passed, zero failures and one ignored performance
test (`markdown-pug-grammar-final-workspace-tests.txt`). Full Shiki parity and
live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Markdown INI fence grammar (2026-11-03)

Markdown fences now highlight INI/properties files. The tokenizer covers
`#`/`;` comments, `[section]` headers, `key = value` assignments and quoted
values.

A fixture test covers comments, sections, keys and strings plus span
integrity. A rendered Markdown test verifies an INI fence paints through the
highlighted code body. Clippy/build pass
(`markdown-ini-grammar-final-clippy.txt`,
`markdown-ini-grammar-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`markdown-ini-grammar-final-isolated-flakes.txt`) and the authoritative rerun
is green at gate997 with 997 passed, zero failures and one ignored performance
test (`markdown-ini-grammar-final-workspace-tests.txt`). Full Shiki parity and
live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Agent TUI exit notice (2026-11-03)

Agent TUI exits now match Source's panel notice behavior: `Agent TUI exited.`
for success or a missing code, and `Agent TUI exited with code N.` for nonzero
exits. The TUI terminal is removed, an empty terminal tray closes, and ordinary
shell-tab exits stay silent.

Pure copy tests cover success, missing-code and nonzero wording. An app-level
rendered test inserts a TUI tab and a shell tab, removes both, and asserts only
the TUI notice is emitted. Clippy/build pass (`agent-tui-exit-final-clippy.txt`,
`agent-tui-exit-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`agent-tui-exit-final-isolated-flakes.txt`) and the authoritative rerun is
green at gate999 with 999 passed, zero failures and one ignored performance
test (`agent-tui-exit-final-workspace-tests.txt`). Live visual acceptance
remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Editor tab drag between groups (2026-11-03)

Editor tabs can now be dragged between editor groups, matching Source's
`moveTabToGroup`. The tab is removed from any prior extra-group assignment,
the destination group shows it and receives focus, and dragging the active
main-group tab promotes the next unassigned tab into the main group. Invalid
destinations leave assignments untouched.

A pure assignment test covers extra/main moves, replacement and invalid
destinations. A rendered drag test opens three tabs, creates a second group,
drags the first titlebar tab onto that group, and asserts the new assignment
and promoted main tab. Clippy/build pass (`editor-tab-group-drag-final-clippy.txt`,
`editor-tab-group-drag-final-build.txt`). The full run passed on the first
attempt at gate1001 with 1001 passed, zero failures and one ignored performance
test (`editor-tab-group-drag-final-workspace-tests.txt`). Live visual
acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Inactive tile tool actions (2026-11-03)

Inactive tiled agent panels now expose expandable tool/shell rows. Each row
uses the Source-shaped tool summary, shows input/output payloads and status when
expanded, and offers a transient Copy action. Expansion and copied state are
stored per panel, so two tiles never share a row state, and expanding or copying
an inactive row does not steal the active panel's focus.

`panels::preview_rows` now records each row's source block index so the tiled
renderer can recover the original tool payload instead of the condensed text.
A pure test covers the tail indices, and a rendered test creates an inactive
panel with a successful shell tool, expands it, verifies the output and status,
checks Copy writes Source's `$ command\n\noutput` text, and confirms the active
panel is unchanged. Clippy/build pass
(`inactive-tile-tool-actions-final-clippy.txt`,
`inactive-tile-tool-actions-final-build.txt`). The first full run hit both known
observation flakes; both passed in isolation
(`inactive-tile-tool-actions-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate1002 with 1002 passed, zero failures and one ignored
performance test (`inactive-tile-tool-actions-final-workspace-tests.txt`).

Nested sub-call expansion and full transcript-wide actions in inactive tiles
remain, along with live visual acceptance.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Inactive tile composer attachments, references and slash menus (2026-11-03)

Inactive tiled composers now own their Source-shaped input aids. Each panel
keeps an independent attachment list with removable chips, an attach action,
`@` file-reference search results with keyboard/click insertion and mention
offsets, slash command filtering, and slash-argument path completion. Sending a
tile builds prompt files from that tile's references/attachments or runs its
slash command; it never borrows the active composer's staged files or menus.
Menu visibility, selection, pending requests and arguments are all per panel.

A rendered test opens two panels, stages and removes an attachment on the
inactive tile, inserts a reference returned through `EngineUpdate::References`,
selects a slash-command argument, and submits a slash command. Both reference
searches and command-argument searches are routed back to the tile that issued
them. Clippy/build pass (`inactive-tile-composer-final-clippy.txt`,
`inactive-tile-composer-final-build.txt`). The first full run hit one known
observation flake; it passed in isolation
(`inactive-tile-composer-final-isolated-flakes.txt`) and the authoritative rerun
is green at gate1003 with 1003 passed, zero failures and one ignored performance
test (`inactive-tile-composer-final-workspace-tests.txt`).

Full rich transcript controls (nested sub-calls and transcript-wide actions),
same-root editor isolation and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Inactive tile nested tool calls and files (2026-11-03)

Inactive tile tool rows now render Source-shaped file rows and recursively
expandable nested tool calls. Expanding a nested call shows its input/output,
files and further nested calls. Expansion state uses the panel's tool-open set
with a call path key, so top-level, nested and sibling calls are independent and
one panel never changes another panel's expansion.

The rendered inactive-tile test now includes a top-level file, a nested `Read`
call with a file, and a deeper `Grep` call. It asserts collapsed nested bodies,
expands both levels, verifies the file rows, and confirms another panel keeps
its nested calls collapsed. Clippy/build pass
(`inactive-tile-nested-tools-final-clippy.txt`,
`inactive-tile-nested-tools-final-build.txt`). The full run passed on the first
attempt at gate1003 with 1003 passed, zero failures and one ignored performance
test (`inactive-tile-nested-tools-final-workspace-tests.txt`).

Transcript-wide actions beyond tool rows, same-root editor isolation and live
visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Same-root editor isolation (2026-11-03)

Editor split-group assignments are now keyed by each activation-owned document
workspace instead of one global vector. Two tiled panels opening the same
physical root therefore keep independent tabs, active documents and split
panes. Document relocation and save-close paths update the owning workspace's
layout only, and reconciliation/limit checks operate on the current panel.

A rendered test opens `a.txt` and `b.txt` in two scopes of `/workspace`, splits
panel A, switches to panel B and asserts no split group is inherited, then
switches back to A and asserts its split group and `a.txt` active document are
restored. Clippy/build pass (`same-root-editor-isolation-final-clippy.txt`,
`same-root-editor-isolation-final-build.txt`). The first full run hit the two
observation flakes and the replay flake; all passed in isolation
(`same-root-editor-isolation-final-isolated-flakes.txt`) and the authoritative
rerun is green at gate1004 with 1004 passed, zero failures and one ignored
performance test (`same-root-editor-isolation-final-workspace-tests.txt`).

Transcript-wide actions beyond inactive tool rows and live visual acceptance
remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Inactive tile message actions and work rows (2026-11-03)

Inactive transcript rows now expose Source-shaped message actions and work
groups. User rows carry Copy, Undo and Edit: Copy writes the owning message text
and shows a transient label without focusing the tile; Undo/Edit focus the
owning tile and route a revert request to its session, with Edit preserving the
original prompt. Consecutive reasoning/activity/system rows collapse into one
per-panel work group, auto-open while the panel is running, and toggle
independently otherwise.

A rendered test opens an inactive panel with a user prompt, reasoning,
activity and answer. It verifies the three actions, expands the work group,
copies the prompt, checks Edit stages `fix it` on the owning session, then
switches back and confirms Undo targets the same session with no edit text.
Clippy/build pass (`inactive-tile-transcript-actions-final-clippy.txt`,
`inactive-tile-transcript-actions-final-build.txt`). The first full run hit the
two observation flakes; both passed in isolation, the rerun hit the known
streaming-pacing flake, it passed in isolation, and the authoritative rerun is
green at gate1005 with 1005 passed, zero failures and one ignored performance
test (`inactive-tile-transcript-actions-final-workspace-tests.txt`).

Inactive answer copy, forms/revert docks and live visual acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Inactive tile answers, forms and revert docks (2026-11-03)

Inactive tiles now expose answer Copy alongside message actions. Assistant rows
write their text to the clipboard with the same transient copied state and no
focus steal. Each panel also owns an independent `FormDock`: form snapshots are
stored per panel and routed from `EngineUpdate::Forms`, submitting/dismissing
focuses the owning tile before replying or cancelling, and form failures reach
the panel dock. Staged revert docks render per panel with commit/discard actions
that target that panel's session directly.

A rendered test opens an inactive panel with an answer, a pending form and a
staged revert. It copies the answer, clicks Commit undo and Discard, checks both
requests target the inactive session, then dismisses the form and verifies the
request owner plus activation. Clippy/build pass
(`inactive-tile-answer-forms-revert-final-clippy.txt`,
`inactive-tile-answer-forms-revert-final-build.txt`). The first full run hit the
known watcher-readiness flake; it passed in isolation
(`inactive-tile-answer-forms-revert-final-isolated-flakes.txt`) and the
authoritative rerun is green at gate1006 with 1006 passed, zero failures and one
ignored performance test
(`inactive-tile-answer-forms-revert-final-workspace-tests.txt`).

Live visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Revert busy and in-flight guards (2026-11-03)

Revert now follows Source's disabled and in-flight rules. Undo/Edit return
silently while a session run is active or any revert request is already pending
instead of emitting a toast or duplicate request. Starting either action emits
Source's `Undoing prompt…` or `Restoring prompt for editing…` notice. Commit and
discard ignore duplicate activation and dim their controls while a request is in
flight, for both the active composer dock and inactive tile docks. A staged
revert still renders over an empty transcript.

A focused rendered test covers the busy no-op and unchanged toast list, both
start notices, duplicate commit/discard calls producing one request, disabled
dock controls during an in-flight discard, and the empty-transcript dock.
Clippy/build pass (`revert-busy-inflight-final-clippy.txt`,
`revert-busy-inflight-final-build.txt`). The full run passed on the first attempt
at gate1007 with 1007 passed, zero failures and one ignored performance test
(`revert-busy-inflight-final-workspace-tests.txt`).

Live/visual acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Accessibility table header and focus outlines (2026-11-03)

Table header cells now carry a platform-visible `Column header` description in
addition to their `ColumnHeader` role, row/column indexes and table row/column
counts. This preserves header semantics on macOS, whose AccessKit adapter maps
both `ColumnHeader` and `RowHeader` to a plain cell. Body cells stay cells with
no header description.

Transcript action controls now paint the shared keyboard focus outline: tool
heads and copy actions, user message Copy/Undo/Edit, answer Copy,
scroll-to-bottom and delegated-agent cards. Explorer directory rows and the root
toggle now expose their expanded state to assistive technology. The existing
pure/rendered table accessibility test now asserts the header description
contract. Clippy/build pass (`accessibility-table-focus-final-clippy.txt`,
`accessibility-table-focus-final-build.txt`). The full run passed on the first
attempt at gate1007 with 1007 passed, zero failures and one ignored performance
test (`accessibility-table-focus-final-workspace-tests.txt`).

Live VoiceOver/platform focus acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Empty Files sidebar Open Workspace action (2026-11-03)

The empty Files sidebar now exposes Source's `Open Workspace` action instead of
only the previous dead-end message. It opens the same folder picker as New
Session, uses the New Session glyph geometry, and carries a short
"Choose a folder to start a session." hint. The empty editor surface already
matched Source's workspace/file-specific copy.

A rendered test opens a workspace-less app on the Files tab, asserts the
`explorer-open-workspace` action is present, and verifies no phantom workspace
directory is reported. Clippy/build pass (`workspace-empty-a11y-final-clippy.txt`,
`workspace-empty-a11y-final-build.txt`). The full run passed on the first attempt
at gate1008 with 1008 passed, zero failures and one ignored performance test
(`workspace-empty-a11y-final-workspace-tests.txt`).

Live empty-profile screenshot acceptance remains.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Slash-command runtime capability gating (2026-11-03)

Slash-looking composer text is now interpreted as a command only when the
runtime advertises the command capability, matching Source's `supportsCommands`
check. With the capability disabled, `/compact` and similar text go through
`prompt_with_behavior` as ordinary prompt text; with it enabled they still route
through the command API. The inactive tile send path uses the same gate.

A pure test covers `prompt_command` with commands enabled/disabled and plain
text, and a rendered send test verifies that the same `/compact` text produces
no command request when unsupported and one command request when supported.
Clippy/build pass (`slash-capability-gating-final-clippy.txt`,
`slash-capability-gating-final-build.txt`). The first full run hit the two
observation flakes; both passed in isolation, the rerun hit the known replay
flake, it passed in isolation, and the authoritative rerun is green at gate1010
with 1010 passed, zero failures and one ignored performance test
(`slash-capability-gating-final-workspace-tests.txt`).

Structured command arguments and live menu acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## Mixed launch-batch serialization (2026-11-03)

Launch paths now flow through a small queue instead of one synchronous loop.
Directory activation sets a launch-busy marker; subsequent files stay queued
until that directory's `WorkspaceOpened` event activates the context, then they
classify and open as tabs in it. File-only batches still start one single-file
workspace request and resume the queue when it finishes. This matches Source's
awaited `openPaths` ordering without adding a second async runtime.

A rendered test opens a temporary directory followed by a file inside it,
confirms the file stays queued while the directory is busy, injects the
`WorkspaceOpened` success event, renders, and asserts the file lands as a tab in
the directory context's document scope. Clippy/build pass
(`mixed-launch-batch-final-clippy.txt`,
`mixed-launch-batch-final-build.txt`). The full run passed on the first attempt
at gate1011 with 1011 passed, zero failures and one ignored performance test
(`mixed-launch-batch-final-workspace-tests.txt`).

Live second-instance routing and physical OS drag acceptance remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.

## User message identity metadata (2026-11-03)

Active user messages now render Source's right-aligned metadata row before the
Copy/Undo/Edit actions. The producing agent is capitalized, the model id is
resolved through the catalog to its display name, and identity fields join with
Source's non-breaking dot separators. Missing agent/model fields render no
identity text instead of empty gaps.

A pure `user_message_meta` test covers agent capitalization, catalog model
resolution and missing fields; a rendered transcript test asserts the
`user-message-meta-*` surface. Clippy/build pass
(`user-message-meta-final-clippy.txt`, `user-message-meta-final-build.txt`). The
first full run hit the known watcher-readiness flake; it passed in isolation
(`user-message-meta-final-isolated-flakes.txt`) and the authoritative rerun is
green at gate1013 with 1013 passed, zero failures and one ignored performance
test (`user-message-meta-final-workspace-tests.txt`).

User timestamps, text selection and the complete context-menu matrix remain.

No GUI/capture, package install or Source write; no page accepted and mobile
deferred.
