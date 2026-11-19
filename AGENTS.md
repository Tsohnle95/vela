# AGENTS.md

Vela is a macOS-first native desktop coding-agent app: Rust, GPUI, Tokio,
OpenCode V2, a native editor, a PTY terminal, and platform web views. The
Electron implementation is archived under `archive/vela-electron/` and is a
**read-only behavioral reference**.

The migration is incomplete. **No page has passed final visual acceptance.**
Feature status is owned by
[`docs/migration/feature-parity-matrix.md`](docs/migration/feature-parity-matrix.md);
do not infer completion from compiling code or passing unit tests.

## Read in this order

1. This file: routing, safety, and the command summary. Detailed commands are
   owned by [`docs/development.md`](docs/development.md).
2. [`docs/agent-execution.md`](docs/agent-execution.md): execution modes,
   planning, validation, delegation, and recovery.
3. [`archive/AGENTS.md`](archive/AGENTS.md) when reading the reference tree.
4. The task-relevant domain owner from the architecture map below.
5. Specific source symbols/tests. Expand only when evidence requires it.

Do not preload all docs, reopen established facts, or ingest the archive,
`docs/parity/`, generated output, dependencies, or the whole UI crate for a
localized change.

## Repository map

| Area | Implementation | Responsibility |
|---|---|---|
| Binary | `apps/vela/src/main.rs` | path launch, settings/migration, Tokio runtime, engine spawn, GPUI run, shutdown |
| Domain | `crates/vela-core` | events, transcript blocks, Markdown boundaries, layout, warm cache, documents, text, search |
| Runtime adapter | `crates/vela-opencode` | service discovery/lifecycle, authenticated HTTP, SSE, typed protocol parsing |
| Engine | `crates/vela-engine` | commands/updates, store, persistence, files, observation, recovery, Git, Sass, preview, providers, terminal relay |
| Terminal | `crates/vela-terminal` | PTY process + emulator/grid/selection |
| Browser | `crates/vela-browser` | WKWebView/Wry host, URL/popup/download policy |
| UI | `crates/vela-ui` | GPUI shell, panels, transcript/composer, settings/providers, terminal/browser chrome, editor/explorer |
| Reference | `archive/vela-electron` | read-only Electron source and historical manual |

Canonical owners:

- System boundaries: [`docs/architecture.md`](docs/architecture.md)
- Core/domain: [`docs/architecture/core.md`](docs/architecture/core.md)
- Adapter/protocol: [`docs/architecture/opencode.md`](docs/architecture/opencode.md)
- Engine/state/recovery: [`docs/architecture/engine.md`](docs/architecture/engine.md)
- UI shell/agent surfaces: [`docs/architecture/ui.md`](docs/architecture/ui.md)
- Editor/explorer/language: [`docs/architecture/editor.md`](docs/architecture/editor.md)
- Setup/verification/parity operations: [`docs/development.md`](docs/development.md)
- Current migration resume point: [`docs/migration/START-HERE.md`](docs/migration/START-HERE.md)
- Durable decisions: [`docs/migration/decisions.md`](docs/migration/decisions.md)

## Commands

```sh
git status --short                     # always inspect before editing
cargo run -p vela -- --workspace PATH  # run against an isolated test workspace
bash scripts/verify.sh                 # canonical gate: docs, clippy, tests, debug build
bash scripts/verify.sh --release       # CI/pre-package gate: release build
bash scripts/verify.sh --check-format  # full gate plus rustfmt verification
python3 scripts/check-docs.py          # routed links, paths, and symbols
```

Build output goes to the sibling `build-junk/` directory via
`.cargo/config.toml` and `scripts/build-env.sh`; delete it freely. Do not add
generated artifacts to the repository.

Targeted checks while iterating:

```sh
cargo test -p vela-core
cargo test -p vela-opencode
cargo test -p vela-engine --test engine_flow
cargo test -p vela-terminal --test pty_flow
cargo test -p vela-ui
cargo clippy --workspace --all-targets -- -D warnings
```

The imported tree is not rustfmt-clean under the supported stable toolchain
(`rust-toolchain.toml`, locally Rust 1.98.1). Do not run a wholesale
`cargo fmt --all` as part of an unrelated change; formatting debt is handled as
its own mechanical slice. The canonical gate therefore checks formatting only
when `--check-format` is explicit.

The offline migration wrapper remains available:
`bash scripts/migration-cargo.sh <check|test|clippy|build|fmt> ...`.
It prefers `build-junk/vela-migration-cache/rust` and runs Cargo offline. Do
not treat a missing cache or an approval-service limit as a product failure.

OpenCode V2 must be on `PATH` or supplied with `VELA_OPENCODE_BINARY`. Use
isolated `XDG_*`, `VELA_DATA_DIR`, and `VELA_SKIP_LEGACY_IMPORT` roots for
lifecycle/visual tests; see [development.md](docs/development.md). Never point
destructive tests at a production profile.

## Symptom router

| Symptom/task | Read first | Likely source | Tests |
|---|---|---|---|
| Transcript row, stream delta, tool/reasoning card | core, ui | `vela-core/src/transcript.rs`, `vela-ui/src/transcript_view.rs`, `tool_view.rs` | inline core/ui tests, `engine_flow` |
| Markdown block, inline layout, links, images, math | core, ui | `vela-core/src/markdown.rs`, `vela-ui/src/markdown_view.rs`, `inline_flow.rs`, `markdown_images.rs` | inline core/ui tests |
| Session open, warm switch, reconnect, queue | engine, opencode | `vela-engine/src/engine.rs`, `store.rs`, `message_queue.rs`, `vela-opencode/src/stream.rs` | `engine_flow`, core warm/perf |
| Service discovery, SSE, HTTP contract | opencode | `vela-opencode/src/service.rs`, `client.rs`, `stream.rs` | adapter tests, `engine_flow` |
| Settings, window size, legacy import | engine | `persist.rs`, `window_state.rs`, `migration.rs`, `legacy_pins.rs` | inline engine tests |
| File save/conflict/recovery/watcher | engine, editor | `files.rs`, `observation.rs`, `recovery.rs`, `vela-core/src/document.rs` | engine file/observation/recovery tests |
| Editor, tabs, groups, explorer, language tooling | editor | `vela-ui/src/editor.rs`, `explorer*.rs`, `tabs.rs`, `formatting.rs`, `syntax.rs`, `diagnostics.rs` | inline UI tests, engine file tests |
| Panel layout, sidebar, composer, forms, providers | ui | `app.rs`, `panels.rs`, `panel_layout.rs`, `composer.rs`, `forms.rs`, `providers.rs` | inline UI/rendered tests |
| Terminal input/emulation/tray | ui, engine | `vela-terminal`, `terminal_relay.rs`, `terminal_*.rs` | `pty_flow`, inline terminal tests |
| Browser tabs/downloads/isolation | ui, browser | `vela-browser/src/lib.rs`, `browser_downloads.rs` | inline policy/download tests |
| Sass/preview/runtime | engine, ui | `sass_*.rs`, `preview.rs`, `runtime.rs`, settings/Sass UI | inline engine tests, fixture GUI |
| Packaging/visual parity | development, matrix | `scripts/package-*`, `scripts/run-parity.sh`, `scripts/capture-parity.sh` | paired lossless captures |
| Toolchain/CI | this file, development | `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `scripts/verify.sh`, `.github/workflows/ci.yml` | `scripts/verify.sh` |

## Non-negotiable safety

- Inspect `git status` before editing. Preserve pre-existing and unrelated work.
- Never reset, clean, stash, overwrite, or discard user changes.
- Do not commit or push unless the user explicitly asks.
- Do not modify `archive/vela-electron/`. Its `AGENTS.md` is historical and is
  **not operative**; this root policy applies to the whole repository.
- Never read, copy, log, or commit credentials, tokens, cookies, personal
  transcripts, or recovery contents merely to understand configuration.
- Production Vela profiles, OpenCode data, credentials, recovery stores, and
  fixture artifacts are user data. Use isolated roots for tests.
- Do not use the user's shared mouse or keyboard. Live GUI interaction is
  restricted to approved app-scoped controls; one-shot window captures are
  acceptable only when the user has approved the current method.
- Do not invent features or controls from stale inventory. Audit current
  archived Source and the parity matrix first.
- Do not replace the UI with Electron/React/Tauri or embed an editor WebView.

## Architecture rules

- No GPUI dependency in `vela-core`, `vela-opencode`, or `vela-engine`.
- No new or blocking network/disk/PTY/compiler/WebView work on the GPUI thread.
  Use engine commands and `EngineUpdate`; the known UI persistence exceptions
  below are debt, not precedent.
- Never hold a store lock across an await.
- Preserve stable transcript block IDs and virtualized-list splicing unless the
  assembly is explicitly structural.
- Keep per-panel session, composer, scroll, focus, forms, permissions, and revert
  state isolated; same-root panels must not share editor activation scopes.
- Settings/credentials/OAuth/recovery are engine-owned. Known exception: session
  bookmark updates and browser download history are still saved by UI code; do
  not extend that pattern, and move new persistence behind engine commands.
- Pin new Git dependencies to an exact revision and record the decision in
  `docs/migration/decisions.md` and `Cargo.lock`.
- No `unsafe` in owned crates without a documented platform requirement.

## Execution modes

Use the lightest mode that keeps expected repair cost acceptable:

- **PATCH**: localized, low-coupling, clear verification. No task file.
- **FEATURE**: coordinated multi-file work with understood architecture. Keep a
  concise in-context plan; persistent memory only when handoff warrants it.
- **PROJECT**: architecture, shared contracts, persistence, security,
  concurrency, recovery, major integration, or substantial uncertainty. Create
  a reviewed `.agent/tasks/` file before coding.
- **BATCH**: multiple independent outcomes. Triage once, then classify and run
  each unit separately.

Replan when an assumption fails, scope expands materially, a new durable
contract appears, two fixes fail for the same cause, or the invariant becomes
unclear. Full protocol: [docs/agent-execution.md](docs/agent-execution.md).

## Definition of done

- Acceptance behavior is implemented and the stated preserved behavior still
  works.
- Targeted tests/checks pass; the canonical gate passes, or the report states
  exactly what could not run and why.
- Contracts, state ownership, failure/recovery, and cross-boundary consumers
  were reviewed, not just the visible endpoint.
- Durable docs are updated only where truth changed.
- No unrelated files, secrets, generated debris, or user data are included.
- Live visual/platform/provider/browser/PTY/packaging claims are marked
  unverified unless actually exercised.
- Temporary task files are completed and deleted; Git owns finished history.

## Documentation ownership

Update the canonical owner, not a duplicate: feature status → parity matrix;
setup/verification → development; system/domain design → architecture docs;
migration state → START-HERE/checkpoint; durable decisions → decisions.md;
agent behavior → `docs/agent-execution.md`; repository routing → this file.

If you add a new durable domain with independent state, contracts, lifecycle, or
recurring routing value, give it one canonical document and link it here. Do not
create docs for directory symmetry.
