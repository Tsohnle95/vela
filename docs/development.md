# Development

Canonical owner for setup, isolated testing, verification, parity operations,
and operational limitations. Architecture boundaries live in
[`architecture.md`](architecture.md); agent routing and safety live in
[`AGENTS.md`](../AGENTS.md); feature status remains in the
[feature parity matrix](migration/feature-parity-matrix.md).

## Prerequisites

- macOS 13+ (primary target), Xcode command line tools with the macOS SDK.
- Rust stable from [`rust-toolchain.toml`](../rust-toolchain.toml), including
  `rustfmt` and `clippy`. `rustup` is recommended and is what CI uses.
- OpenCode V2 on `PATH` (tested with 2.1.4), or `VELA_OPENCODE_BINARY`.
- Python 3.7+ for the documentation-link gate (`VELA_PYTHON` overrides the
  executable).
- Node is **not** required to build or run the Rust app. Node is used by the
  pinned preview/Sass utilities under `tools/preview/` and by the reference
  helper scripts.

## Build and run

```sh
bash scripts/verify.sh                 # canonical local gate
bash scripts/verify.sh --release       # CI/pre-package gate
cargo run -p vela -- --workspace /path/to/repository
```

`scripts/verify.sh` finds Cargo through `PATH`, `$HOME/.cargo/bin`, or
`VELA_CARGO`. It checks documentation links, Clippy, the workspace tests, and an
app build. `--check-format` adds rustfmt verification to that full gate; it is
opt-in because the imported tree is not rustfmt-clean. Do not reformat unrelated
files in a behavior change. Keep detailed commands here; root `AGENTS.md` and the
execution protocol only summarize and route to this owner.

`--workspace` is optional. Without it the app opens the empty/welcome surface;
prior conversations remain available from Recents. It does not automatically
reopen the last conversation.

Release build and run:

```sh
cargo build --release -p vela
source scripts/build-env.sh
"$VELA_TARGET_DIR/release/vela" --workspace /path/to/repository
```

## Open the development app

Use the managed launcher instead of exporting isolated roots by hand:

```sh
./scripts/dev-app.sh                 # current directory as the workspace
./scripts/dev-app.sh /path/to/repo   # explicit workspace
```

It builds and packages the debug app, creates a persistent dev profile at
`~/Library/Application Support/Vela Dev`, starts an isolated OpenCode service on
a free loopback port, and launches the app with logs in that profile. It never
uses the production Vela or OpenCode data.

For a self-contained double-clickable app:

```sh
./scripts/install-dev-app.sh
open "$HOME/Applications/Vela Dev.app"
```

The installer copies the packaged debug app into `~/Applications/Vela Dev.app`,
installs the Vela icon, and makes the bundle independent of `build-junk`. All
runtime state — profile, OpenCode config/data/state, downloads, logs — lives in
one folder: `~/Library/Application Support/Vela Dev`. Quit with Cmd+Q before
reinstalling; the installer refuses to replace a running app.

## Build outputs

Generated files stay outside the source tree:

- Cargo output: `../build-junk/vela-target`.
- Preview toolchain/dependencies/cache: `../build-junk/vela-preview`.
- Migration Rust cache: `../build-junk/vela-migration-cache`.
- Parity app bundles: `../build-junk/vela-target/{debug,release,parity}`.

`.cargo/config.toml` sets Cargo's target directory for direct `cargo` commands.
Repository scripts source `scripts/build-env.sh`, which exports
`VELA_BUILD_JUNK`, `VELA_TARGET_DIR`, `VELA_PREVIEW_ROOT`,
`VELA_MIGRATION_CACHE`, and `CARGO_TARGET_DIR`. Override `VELA_BUILD_JUNK` or
`CARGO_TARGET_DIR` for a different location. Delete `build-junk` whenever you
want; nothing in it is tracked or required. If preview tooling was installed,
`tools/preview/node_modules` becomes a broken symlink after that deletion;
rerun `scripts/setup-preview-toolchain.sh` or remove the symlink.

## Tests

Use the canonical gate:

```sh
bash scripts/verify.sh
```

For focused iteration:

```sh
cargo test -p vela-core
cargo test -p vela-opencode
cargo test -p vela-engine --test engine_flow
cargo test -p vela-terminal --test pty_flow
cargo test -p vela-ui
cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/check-docs.py
cargo fmt --all --check   # formatting-only check; currently fails on imported debt
```

To reproduce transcript stress timings with synthetic data:

```sh
# Native window and platform frame clock; run 1 and 8 separately.
cargo run --locked --release -p vela-ui --features transcript-bench --example transcript-bench -- 1
cargo run --locked --release -p vela-ui --features transcript-bench --example transcript-bench -- 8
# Eight visible panels with one streaming; compare with `8 ordinary 8`.
cargo run --locked --release -p vela-ui --features transcript-bench --example transcript-bench -- 8 ordinary 1

# Deterministic renderer and adapter timings without a native window.
VELA_STRESS_PANELS=8 cargo test --locked --release -p vela-ui --lib streamed_transcript_release_stress -- --ignored --nocapture
```

Use the isolated environment below. The fixture also creates its own temporary
service registration, engine profile, and loopback server. The native example
opens one disposable window, injects composer input directly into that window,
and exits after the workload. It does not capture the screen. Native
scene timing measures the end of GPUI's draw for the frame with the observed
revision. It does not measure GPU completion or monitor scanout.
Pass a second argument of `ordinary`, `paragraph`, `fence`, or `table` to
isolate the final payload. The default `combined` also sends a 1 MiB tool result
with the tool disclosure closed. The `tool` case opens that output without the
oversized Markdown sections.
Set `VELA_TRANSCRIPT_VISUAL_CHECK=1` with `8 tool 8` to exercise composer
typing and a wheel event in the fixture window, verify the reading anchor and
settled tile bounds, and keep the settled window visible for eight seconds.

The `--offline` migration wrapper remains available when the repo-local cache
exists:

```sh
bash scripts/migration-cargo.sh check --workspace
bash scripts/migration-cargo.sh test --workspace
bash scripts/migration-cargo.sh clippy --workspace --all-targets -- -D warnings
bash scripts/migration-cargo.sh build -p vela
```

It prefers `build-junk/vela-migration-cache/rust`, falls back to installed
Cargo, and does not handle `fmt` with `--offline`.

Notable coverage:

- `vela-core`: Markdown block splitting, event normalization, transcript
  streaming/tool lifecycle, layout arithmetic, warm eviction.
- `vela-opencode`: service version gate, session parsing, SSE framing.
- `vela-engine`: atomic settings, legacy migration, and a full mock-service
  end-to-end flow (`tests/engine_flow.rs`) covering discovery, replay, live
  deltas, prompt submission, permissions, and execution state.
- `vela-core/tests/perf.rs`: release-only projection/navigation benchmark.

## Debugging

```sh
RUST_LOG=vela=debug,vela_engine=debug,vela_opencode=debug \
  cargo run -p vela -- --workspace /tmp/vela-test-workspace
```

Diagnostic logs go to stderr; keep credentials, cookies, prompt bodies and
recovery contents out of them. Native normalized replay journals live under
the isolated profile's `stream-journal/` and contain private conversation/tool
contents. Never inspect real profile journals to understand configuration.
OpenCode stays authoritative; see [engine ownership](architecture/engine.md).

Native streaming regressions (rather than browser tests against the archive):

```sh
cargo test -p vela-core run_log
cargo test -p vela-core activity
cargo test -p vela-engine --test streaming_publication
cargo test -p vela-ui transcript_stability_tests
```

These cover 10k deltas, duplicate IDs/disk replay, 100+ call disclosure, delegate
order, Markdown growth and measured reading anchors. GPUI bounds, offsets and
row render counters replace DOM anchor/scroll/render instrumentation. They do
not establish live visual acceptance.

Useful boundaries:

- Connection/service symptoms: inspect the registration path and
  `VELA_OPENCODE_SERVICE_FILE`; do not edit a service registration by hand.
- Engine/session symptoms: reproduce with `cargo test -p vela-engine --test
  engine_flow` and the isolated fixture before touching a real profile.
- Filesystem/editor symptoms: use a disposable workspace; inspect document
  ownership and observation generation before changing UI behavior.
- Rendering/input symptoms: use the development-only `VELA_UI_ACTION` hooks
  below, then a dedicated parity profile for window-level checks.
- Prior evidence: read the tail of
  [`parity/2026-10-25/checkpoint.md`](parity/2026-10-25/checkpoint.md); old
  “running” entries are superseded by later results.

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

Vela's own profile is `~/Library/Application Support/vela-native` on macOS. Delete that directory only when you intend to lose local preferences.

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

This copies the original dev Electron bundle into
`build-junk/vela-target/parity/`, changes only
its app identity and ad hoc signature, and runs the reference renderer without
editing it. Reference tooling defaults to `archive/vela-electron`; set
`VELA_REFERENCE_ROOT` to use a separate pre-merge Electron checkout instead. The
archived `dev/Vela.app` and `node_modules` are local ignored build artifacts, not
tracked source: restore them from a backup, or build in a separate writable
pre-merge checkout and point `VELA_REFERENCE_ROOT` there. Do not build inside
`archive/vela-electron`. Bind
automation to `dev.vela.reference.parity`. The original dev app shares its
identity with `/Applications/Vela.app`; rebinding by path can launch or select
that installed app instead. Use a dedicated profile and verify the process/window
identity. Reference window placement is restored by the original app; the
native-only origin option does not position Electron.

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
source scripts/build-env.sh
VELA_UI_ACTION=terminal "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
VELA_UI_ACTION=files "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
VELA_UI_ACTION=changes "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
VELA_UI_ACTION=diff:README.md "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
VELA_UI_ACTION=browser "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
VELA_UI_ACTION=picker "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
VELA_UI_ACTION=settings "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
VELA_UI_ACTION=session:<session-id> "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
```

The hook runs once after the active session is available and has no effect when
unset. It is not a privileged control surface; it only invokes the same UI
actions a user click invokes.

## Legacy data

On startup, Vela reads the original Electron profile
(`~/Library/Application Support/Vela/saved-workspaces.json` and
`window-bounds.json`) and merges workspace bookmarks into its own settings. The
import:

- never writes to the legacy directory;
- is idempotent (merge by directory);
- ignores unsupported future schemas and corrupt files;
- does not read credentials, cookies, or transcripts.

Run `cargo test -p vela-engine migration` for the safety tests.

## Architecture rules

The canonical rules are owned by [`AGENTS.md`](../AGENTS.md) and
[`architecture.md`](architecture.md). In short: no GPUI in the inner crates, no
new or blocking I/O on the render thread, no lock across an await, stable
transcript identity, engine-owned persistence/credentials (with the documented
UI bookmark/download exceptions as debt), and no undocumented `unsafe`. Pin new
Git dependencies to an exact revision and record the decision in
[`decisions.md`](migration/decisions.md) and `Cargo.lock`.

## Window placement during visual checks

The primary display is reserved for the user's own work. Launch visual-test
windows on a non-main display:

```sh
./scripts/run-secondary.sh --workspace /tmp/demo
# or
source scripts/build-env.sh
VELA_WINDOW_SCREEN=non-main "$VELA_TARGET_DIR/release/vela" --workspace /tmp/demo
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

Feature-level status is owned by the
[feature parity matrix](migration/feature-parity-matrix.md). The limitations
below are operational or acceptance constraints, not a second feature matrix.

- No page has passed live visual acceptance. Static, rendered, and unit tests do
  not establish screenshot parity.
- OS IME/candidate behavior, VoiceOver/platform focus, WKWebView
  permission/download/lifecycle behavior, full TUI/PTY appearance, and real
  browser interactions require live validation.
- Live provider OAuth, key refresh, and usage fetches require credentials and
  network access; do not substitute real credentials in fixtures.
- Packaging, signing, notarization, install, and update paths require release
  access; the checked-in package scripts are development helpers, not a shipped
  release pipeline.
- Mobile companion and shared-session work is deferred/planned.
- Git is read-only: status and unified diffs, no staging, commit, branch
  switching, or non-Git baseline tracking.
- Agent-driven browser automation is not implemented; the browser remains a
  user-facing native surface without a page-to-engine bridge.
- Windows and Linux have not been built or validated.
- Performance measurements exclude painted-frame/compositing cost and do not
  yet instrument idle CPU. Quote [`performance/methodology.md`](performance/methodology.md)
  before using numbers.
- Editor language tooling is bounded and local, not Monaco or an external LSP;
  see [`architecture/editor.md`](architecture/editor.md) for the supported
  subset and matrix rows for remaining gaps.
- Codex is the default shell. Classic is intentionally retained under
  **Settings → Appearance**; do not remove it when changing Codex chrome.
- `surface.rs` clips composed panels with GPUI's rounded paint scope. A rounded
  background plus rectangular overflow clipping cannot contain opaque descendants.
  `settings.rs` owns the shared Settings layout, decoration and rendered width
  regression; its shell row must retain both `min_w_0()` and `w_full()`.

## Fixture and preview tools

Runtime control checks can use `provider-fixture-server.py --port=49560
--state=/private/tmp/vela-runtime-fixture --models --runtime`. This creates a
disposable `bin/opencode` double in that state directory. Pass its path using
`run-parity.sh --opencode=/private/tmp/vela-runtime-fixture/bin/opencode`; reference
launches also put that directory first on PATH. The double only edits its own
fixture files. Its delay, next version, failure and package-manager prompt can
be configured in `runtime-fixture.json`; command metadata is recorded separately.
Never substitute the real CLI when exercising upgrade/restart fixture controls.

For Vite preview development, run `bash scripts/setup-preview-toolchain.sh` once.
This installs the verified Node archive, npm cache, and locked dependencies under
`build-junk/vela-preview`; `tools/preview/node_modules` is only a symlink to the
external dependency directory. Packaging
now includes these resources and refuses to replace a running app bundle. Close
the QA window before packaging; leave the ordinary app's active work alone.
Preview lifecycle tests use disposable Python HTTP child processes, including
Unicode page URLs, multiple entries, failed startup, timeout, workspace-close
cancellation and cleanup after application shutdown.
