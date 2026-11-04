# Development

## Prerequisites

- macOS 13+ (primary target), Xcode command line tools with the macOS SDK.
- Rust stable (tested with 1.98.1). `rustup` is recommended.
- OpenCode V2 on `PATH` (tested with 2.1.4).
- Node is **not** required to build or run Vela Native.

## Build and run

```sh
cargo run -p vela -- --workspace /path/to/repository
```

`--workspace` is optional. Without it the app restores the last session from
its own settings; otherwise it shows the welcome surface.

Release build:

```sh
cargo build --release -p vela
./target/release/vela --workspace /path/to/repository
```

## Tests

```sh
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets
```

Notable coverage:

- `vela-core`: Markdown block splitting, event normalization, transcript
  streaming/tool lifecycle, layout arithmetic, warm eviction.
- `vela-opencode`: service version gate, session parsing, SSE framing.
- `vela-engine`: atomic settings, legacy migration, and a full mock-service
  end-to-end flow (`tests/engine_flow.rs`) covering discovery, replay, live
  deltas, prompt submission, permissions, and execution state.
- `vela-core/tests/perf.rs`: release-only projection/navigation benchmark.

## Isolated OpenCode testing

Never point development runs at a production OpenCode profile when testing
lifecycle behavior. Use an isolated state/config root and a dedicated service
port:

```sh
export XDG_STATE_HOME=/tmp/vela-native-state
export XDG_DATA_HOME=/tmp/vela-native-ocdata
export XDG_CONFIG_HOME=/tmp/vela-native-occonfig
mkdir -p "$XDG_STATE_HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME"
opencode service set port 49555
opencode service start
cargo run -p vela -- --workspace /tmp/vela-test-workspace
```

`VELA_OPENCODE_SERVICE_FILE` overrides the registration file path, which is how
the integration test injects its mock service.

Vela Native's own profile is `~/Library/Application Support/vela-native` on
macOS. Delete that directory only when you intend to lose local preferences.

Use `VELA_DATA_DIR=/path/to/test-profile` for isolated native preferences without
changing the process home directory. `VELA_SKIP_LEGACY_IMPORT=1` prevents test
runs from importing real workspace bookmarks; `VELA_LEGACY_PROFILE_DIR` selects
an explicit legacy fixture when testing migration.

For paired screenshot work, build and run a distinct native app bundle:

```sh
cargo build -p vela
bash scripts/package-macos.sh debug
bash scripts/run-parity.sh browser
# Keep the same display/backing scale as a measured reference window:
bash scripts/run-parity.sh --origin=-1404,266 settings:about
```

The parity launcher uses a dedicated native profile, the isolated OpenCode
roots above, a 1280×800 window on the non-main display, and quits when its test
window closes. Override `VELA_PARITY_PROFILE`, `VELA_PARITY_WORKSPACE`,
`VELA_PARITY_SIZE`, `VELA_PARITY_STATE`, `VELA_PARITY_CONFIG` and
`VELA_PARITY_DATA` to use other fixtures. `VELA_WINDOW_SIZE=widthxheight` is
also available for direct launches (minimum 200×200). Normal launches restore
`window-bounds.json` from the native profile, or use 1280×800. The parity
launcher accepts `--size=widthxheight`; `--size=saved` removes its viewport
override so restoration can be exercised.

Use `bash scripts/package-macos.sh debug --comparison` and launch with
`bash scripts/run-parity.sh --comparison ...` to isolate the QA process from
an ordinary native app under inspection. This gives the same executable a
distinct `dev.vela.native.parity` bundle identity. The QA executable requires
the comparison flag, isolated data/config/state roots and disabled legacy import
before reading settings or opening OpenCode; launch it through the parity script. Always select an explicit
window ID for captures when multiple Vela windows exist.

Give the original reference a separate identity too:

```sh
bash scripts/package-reference-macos.sh
bash scripts/run-parity.sh --reference-comparison --profile=/private/tmp/vela-reference-qa
```

This copies the original dev Electron bundle into `target/parity/`, changes only
its app identity and ad hoc signature, and runs the original checkout's renderer
without editing it. Bind automation to `dev.vela.reference.parity`. The original
dev app shares its identity with `/Applications/Vela.app`; rebinding by path can
launch or select that installed app instead. Use a dedicated profile and verify
the process/window identity. Reference window placement is restored by the
original app; the native-only origin option does not position Electron.

Use `VELA_PARITY_ORIGIN=x,y` to launch on the reference window's display. Point
sizes alone are insufficient when two displays use different backing scales.
`swift scripts/parity-window-info.swift` lists only the Vela window identities
and bounds. For full-resolution lossless window-only captures:

```sh
bash scripts/capture-parity.sh original editor
bash scripts/capture-parity.sh native editor
```

The capture helper does not activate, move or resize either window. It requires
screen capture permission and writes PNGs with JSON provenance manifests.
Automation JPEGs are useful checkpoints but cannot establish lossless parity.

After saving paired PNGs into an evidence directory's `original/` and `native/`
folders, run `python3 scripts/compare-screenshots.py <evidence-directory>`
(requires Pillow). View `comparison/index.html` and the raw `report.json`.
Different dimensions and missing captures are reported, never normalized away.
RGBA differences include transparency. A decoded JPEG equality is never counted
as a lossless pixel match.
Different ICC profiles are ineligible for visual review; recapture on the same
display instead of converting either image. Historical captures without stable
before/after window inventories are retained but cannot establish acceptance.

## UI smoke hooks

For deterministic screenshots and interaction checks without OS accessibility
permissions, the app accepts two development-only environment variables:

```sh
VELA_UI_ACTION=terminal ./target/release/vela --workspace /tmp/demo
VELA_UI_ACTION=files ./target/release/vela --workspace /tmp/demo
VELA_UI_ACTION=changes ./target/release/vela --workspace /tmp/demo
VELA_UI_ACTION=diff:README.md ./target/release/vela --workspace /tmp/demo
VELA_UI_ACTION=browser ./target/release/vela --workspace /tmp/demo
VELA_UI_ACTION=picker ./target/release/vela --workspace /tmp/demo
VELA_UI_ACTION=settings ./target/release/vela --workspace /tmp/demo
VELA_UI_ACTION=session:<session-id> ./target/release/vela --workspace /tmp/demo
```

The hook runs once after the active session is available and has no effect when
unset. It is not a privileged control surface; it only invokes the same UI
actions a user click invokes.

## Legacy data

On startup, Vela Native reads the original Electron profile
(`~/Library/Application Support/Vela/saved-workspaces.json` and
`window-bounds.json`) and merges workspace bookmarks into its own settings. The
import:

- never writes to the legacy directory;
- is idempotent (merge by directory);
- ignores unsupported future schemas and corrupt files;
- does not read credentials, cookies, or transcripts.

Run `cargo test -p vela-engine migration` for the safety tests.

## Architecture rules

- No GPUI dependency in `vela-core`, `vela-opencode`, or `vela-engine`.
- No network/disk/PTY work on the GPUI thread. Use engine commands and
  `EngineUpdate`.
- Never hold a store lock across an await.
- Prefer incremental projection over full transcript rebuilds. If you add a
  mutation path, keep block identity stable or explicitly mark the assembly
  structural.
- Pin new Git dependencies to an exact revision and record them in
  `docs/migration/decisions.md` and `Cargo.lock`.
- No `unsafe` in owned crates without a documented platform requirement.

## Window placement during visual checks

The primary display is reserved for the user's own work. Launch visual-test
windows on a non-main display:

```sh
./scripts/run-secondary.sh --workspace /tmp/demo
# or
VELA_WINDOW_SCREEN=non-main ./target/release/vela --workspace /tmp/demo
```

The app now defaults to the largest non-main display when more than one is
connected. Set `VELA_WINDOW_SCREEN=main` to opt back in to the primary display.
`VELA_WINDOW_ORIGIN=x,y` overrides both. Window-ID screenshots work regardless
of which display the window occupies.

## Benchmarks

```sh
./scripts/bench-core.sh
VELA_NAV_IDS=id1,id2,id3,id4,id5,id6 ./scripts/bench-nav.sh
```

The first runs the release projection harness and prints JSON. The second drives
the real GPUI event loop through six imported sessions and writes warm-switch
timings. Both exclude painted-frame/compositing cost; see
`docs/performance/methodology.md` before quoting results.

## Current limitations

- Inline Markdown styling, syntax highlighting, links, and copy actions are not
  rendered yet.
- Composer/address fields use native text input, Unicode selection, clipboard,
  undo/redo and platform composition ranges. An OS IME session remains unverified.
- The terminal is a single dock: no per-workspace trays, tabs, selection/copy,
  mouse wheel, or TUI launcher yet.
- The source editor now has workspace-owned buffers, ordered snapshot saves,
  preview/pinned tabs and native text editing. Syntax/language services, search,
  multi-cursor, recovery management and full close/quit guards remain unfinished.
  Save/⌘S deliberately persists manually, matching the current reference.
  Native workspace observation now reloads clean buffers, preserves dirty
  conflicts, follows unique renames and refreshes expanded explorer directories;
  real-app acceptance is in progress. The watcher uses pinned
  [notify8.2.0](https://docs.rs/notify/8.2.0/notify/); its native notification
  tests run against actual filesystem changes rather than mocked callbacks.
- Browser tabs retain native views and history. On macOS they share an
  application-owned nonpersistent partition; clipboard, native undo/redo,
  navigation, retained inputs, cookie sharing and restart isolation were tested
  with local fixtures. External-link handoff and agent automation remain gaps.
- Git is read-only: status and unified diffs, no staging, commit, branch
  switching, or non-Git baseline tracking.
- Follow-ups queue while a run is active. Chips support steer and cancel, but
  queued text cannot be edited or reordered.
- Settings expose persisted shell/theme/experimental-art choices and runtime
  diagnostics. Provider/runtime/voice/mobile operations remain unfinished.
- Sass watch uses the pinned bundled Dart Sass 1.102.0 and JSON5 2.2.3 tools.
  Rust owns shared root contexts, durable preferences, dependency/configuration
  inventory, cancellation and atomic CSS/map writes. Planning supports explicit
  `vela-sass.json`, legacy names, Live Sass workspace settings and literal
  package mappings; package scripts never execute. Run the compiler utility
  checks with `.tools/preview/node/bin/node --test tools/preview/sass-tools.test.mjs`.
  `scripts/compare-sass-reference.mjs` compares disposable fixture plans/CSS/maps
  directly with the read-only reference utilities. Full native UI and
  cross-platform acceptance remain open.

- Forms and mobile pairing are not implemented. The parity matrix records each
  gap.
- Codex is the default shell. Classic is intentionally retained under
  **Settings → Appearance**; do not remove it when changing the Codex chrome.
- `surface.rs` clips composed panels with GPUI's rounded paint scope. A rounded
  background plus rectangular overflow clipping cannot contain opaque descendants.
  `settings.rs` owns the shared Settings layout, decoration and rendered width
  regression; its shell row must retain both `min_w_0()` and `w_full()`.
- Windows and Linux have not been built or tested.
- End-to-end navigation timing and idle CPU are not yet instrumented.

Runtime control checks can use `provider-fixture-server.py --port=49560
--state=/private/tmp/vela-runtime-fixture --models --runtime`. This creates a
disposable `bin/opencode` double in that state directory. Pass its path using
`run-parity.sh --opencode=/private/tmp/vela-runtime-fixture/bin/opencode`; reference
launches also put that directory first on PATH. The double only edits its own
fixture files. Its delay, next version, failure and package-manager prompt can
be configured in `runtime-fixture.json`; command metadata is recorded separately.
Never substitute the real CLI when exercising upgrade/restart fixture controls.

For Vite preview development, run `bash scripts/setup-preview-toolchain.sh` once.
This installs only under `.tools/preview` and `tools/preview/node_modules`, with
a verified official Node archive and the locked reference versions. Packaging
now includes these resources and refuses to replace a running app bundle. Close
the QA window before packaging; leave the ordinary app's active work alone.
Preview lifecycle tests use disposable Python HTTP child processes, including
Unicode page URLs, multiple entries, failed startup, timeout, workspace-close
cancellation and cleanup after application shutdown.
