# Codex redesign integration

Source snapshot: `ee8d298` from `/Users/tdawg/.local/share/opencode/worktree/b865fa/vela-codex-redesign`.
Target starts at completed refactor `0cf9652`; common ancestor `3e63792433ef7a6d2ff04540a7181458d953d7a5`.
Integrated production checkpoint: `7f28dad`. Independent cumulative review approved the result; checks and explicit verification limits follow. [Source delta inventory](source-delta.json) records every changed source path and its integration disposition.

## Integrated features

- Independent Codex/Classic interface and persisted appearance selection, Codex light/dark palettes, syntax-token colors and stale-token cleanup. Classic restores card chrome, not the former component layout.
- Always-visible activity rail, titlebar panel toggles and shared editor strip; session Pinned/Projects/Recents, filtering/pinning and workspace actions.
- Workspace-name tree root, language glyphs/colors and generation-aware directory hover prefetch.
- Compact agent controls, usage in the composer, responsive narrow-panel controls and pixel Neptune empty state.
- Grouped titlebar file tabs, breadcrumbs, split geometry/transfer, editor scroll reveal and status menu with validation/Sass controls relocated from the footer.
- Independent browser state/actions, lazy isolated guests, URL/search field, history/reload/stop, retained background tabs and file activation returning to the editor.
- Dev bundle parent-directory creation repair. Main profile, install/build flow, shared public IPC and sole OpenCode/runtime authority remain intact.

## Integration architecture

The redesign branched before the final Store extraction. Its earlier Store was not copied.
WorkspaceFiles owns prefetch; Editor emits a stable optional successful foreground-file notification. BrowserStoreProvider translates it to browser selection; URL/title/loading updates do not reach Store consumers. The actual-provider test processes 100 separate browser updates without another Store consumer render. Background, failed and stale file opens retain browser focus.

Main owns browser attachment/navigation/permission policy. Guests have no application preload, Node/frame/worker authority or privileged IPC identity. Their nonpersistent session denies permission checks/requests/devices before attachment. Existing app-window trust and 95 public IPC channels remain unchanged. The browser feature intentionally adds untrusted-web rendering within those restricted guests.

New modules have named canonical routes, private import boundaries and selected pure-owner dependency restrictions. Existing limits were retained except explicitly reviewed feature growth: createWindow152→154 composition lines; Store1172→1173, StoreBody1015→1016 and facade106→107; WorkspaceFiles304→322 inclusive lines; Editor502→518 and openFileC24→26. These increases reflect actual feature/correctness checks rather than refactor debt rebasing. Every new module remains within default size/function limits.

## Demonstrated corrections while integrating

- Initial-URL tabs now create a guest; invalid privileged/credentialed schemes show a useful error.
- Main-frame load failures survive stop-loading; subframe failures do not replace the page error.
- Stop before deferred attachment retains the submitted target for Reload. Blurring the address field after submission retains the latest committed URL.
- Browser tab selection/closing uses a single state transition; nonexistent activations and late updates are ignored.
- Browser/file overflow exposes every tab, keeps the add button reachable and portals the menu above clipping ancestors.
- Browser strips remain available without a workspace. Split menus validate their local tab instead of another group's focused file.
- Split drag listeners clean up on restart/unmount/layout change; separators support keyboard resizing.
- Composer usage popup is allowed to extend above its anchor rather than being clipped.

## Excluded source-workspace scaffolding

The source's unconditional `vela-design-copy` development profile default was not installed into main: main retains its normal saved sessions/settings. Source task ledgers and stale architecture documentation/budgets were not copied. The source checkout is read-only throughout. No design experiment or dormant runtime source was deleted.

## Verification

- Appearance checkpoint `c0b6af7`: 20 targeted tests/typecheck, isolated 1,331-test canonical gate, native palette/shell independence and independent review.
- Browser security checkpoint `e7665c3`: 13 focused tests, isolated 1,338-test canonical gate, full macOS platform gate including actual guest isolation smoke and independent review.
- Integrated working tree: supported Node 22.23.2 canonical gate passed 1,382 tests in 165 passing files, four existing skipped tests in two files, node/web types, docs, architecture and production build. Full macOS platform gate passed, including Electron browser smoke.
- Real Electron browser smoke exercises malicious preference/preload stripping, guest/worker isolation, real geolocation denial, guest IPC identity rejection and blocked initial/frame/navigation/redirect targets. Popup OS opening is mocked; device checks invoke the installed denial handler without physical hardware. Native smoke runs on macOS and explicitly skips elsewhere.
- Disposable native app uses the actual compiled renderer/preload and mutation backend, mock runtime/catalog/forms/usage responses, its own profile and temporary workspace. It verified live HTTPS navigation, browser/file switching and retained guests, shared strip and overflow selection, split editors/local status, composer usage popup, session pin/unpin, Agent Mode rail restoration and disk-backed save with UUID/generation/revision/expected-content checks and empty unsaved list. A fresh empty-history fixture showed Neptune, dispatched/completed a streamed mock reply, closed/reopened it through Recents, and closed a browser after the last workspace closed. Failed-load address/retry behavior was checked after rebuilding.

## Remaining verification limits

Paid-provider prompting, physical device grants/mobile hardware, signed updates, Windows/Linux native browser behavior and full application quit/recovery dialogs are not established by these checks. Native titlebar/file-to-group/explorer drag gestures require separate direct confirmation; attempted titlebar drags produced no confirmed move event, so no successful native drag is claimed; automated transfer/drop and filesystem/platform checks cover their respective paths. The user's normal running Vela/profile and redesign workspace are not replaced or reset.

## Measurements and validation provenance

Production scope is unchanged: 39,177→41,470 LOC (+2,293) and 193→211 production files.
The same three oversized modules remain; Store adds only one prefetch facade line
(1,172→1,173), OpenCode and the static language table remain unchanged. Both
runtime and type-only imports remain acyclic. No new runtime dependency was added.
[Metrics and production hashes](metrics.json) record this integration's snapshot;
these feature additions have no code-reduction target.

A final gate exposed two existing test-fixture issues: the launcher payload test
checked the checkout's ignored build-failure flag instead of its packaged app
fixture, and Layout's incomplete bridge mock generated a toast after jsdom
teardown. The assertion now checks the packaged target and the mock implements
saved-workspace loading/saving. No production launcher/timer policy was changed,
and the checkout's real ignored flag was not deleted. Both targeted 61-test and
full canonical retries passed. Sandboxed watcher tests emitted EMFILE warnings;
full native platform coverage passed outside that sandbox. A disposable
empty-history QA setup briefly omitted sibling mock declarations; it was
corrected and the fresh UI checks repeated successfully.

Browser hardening follows Electron's [security guidance](https://www.electronjs.org/docs/latest/tutorial/security/)
and [session permission APIs](https://www.electronjs.org/docs/latest/api/session),
with installed Electron types and real guest checks as implementation evidence.

Final build inventory: 720 regular files / 50,417,406 uncompressed bytes. No installer size claim.
