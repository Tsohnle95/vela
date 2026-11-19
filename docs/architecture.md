# Vela architecture

> **Status:** current native architecture, 2026-11-03. This document describes
> the system that exists, not migration acceptance. Feature status, remaining
> gaps, and live-validation debt are owned by the
> [feature parity matrix](migration/feature-parity-matrix.md). No page has
> passed final visual acceptance.

## 1. What Vela is

Vela is a macOS-first native desktop application for coding agents. It is the
native successor to the archived Electron app in `archive/vela-electron/`.
The presentation layer is Rust + GPUI; the runtime protocol is OpenCode V2;
the editor, terminal, and browser are native surfaces rather than embedded
application frameworks.

Vela is not a generic chat client, a visual redesign, or an Electron/Tauri
wrapper. Product behavior and visual tokens come from the archived reference.
When this document and current source disagree, source wins; when source and
the reference disagree, the parity matrix records the gap.

## 2. Documentation ownership

| Document | Owns | Read when |
|---|---|---|
| [`AGENTS.md`](../AGENTS.md) | repository-wide agent entry point, routing, safety, setup/verification summary | every task |
| [`docs/agent-execution.md`](agent-execution.md) | PATCH/FEATURE/PROJECT/BATCH protocol, replanning, delegation, task memory, validation | planning or resuming work |
| **This file** | system boundaries, dependency direction, runtime topology, cross-domain invariants | orienting in an unfamiliar area |
| [`docs/architecture/core.md`](architecture/core.md) | pure domain model: events, transcript projection, Markdown boundaries, layout, warm cache, documents/text | transcript, Markdown, layout, text, warm-session work |
| [`docs/architecture/opencode.md`](architecture/opencode.md) | OpenCode service discovery, HTTP client, SSE transport, typed protocol parsing | runtime protocol, service lifecycle, event ingestion |
| [`docs/architecture/engine.md`](architecture/engine.md) | application orchestration, store/state ownership, persistence, files, observation, recovery, Sass, preview, providers, terminal relay | engine commands/updates, lifecycle, filesystem, settings, recovery |
| [`docs/architecture/ui.md`](architecture/ui.md) | GPUI shell, panels, sidebar, agent conversation surfaces, settings/providers, terminal/browser chrome, input/focus/accessibility | UI behavior or visual work outside the editor |
| [`docs/architecture/editor.md`](architecture/editor.md) | source editor, tabs/groups, explorer, changes/diffs, language tooling | editor, explorer, file mutation, diagnostics, formatting |
| [`docs/development.md`](development.md) | setup, isolated profiles, build/test/parity commands, operational limitations | running, debugging, packaging, visual checks |
| [`docs/migration/START-HERE.md`](migration/START-HERE.md) | current migration resume point and working rules | resuming product-parity work |
| [`docs/migration/feature-parity-matrix.md`](migration/feature-parity-matrix.md) | per-feature status, evidence, risk, and required acceptance | deciding whether a feature is complete |
| [`docs/migration/decisions.md`](migration/decisions.md) | durable architecture decisions and rejected alternatives | changing a pinned or cross-cutting design |
| [`docs/parity/2026-10-25/checkpoint.md`](parity/2026-10-25/checkpoint.md) | append-only execution evidence | verifying a prior claim |

`README.md` is public/product-facing. It must not become an independent agent
manual.

## 3. Repository and dependency map

```text
apps/vela/                 binary: CLI/path launch, settings load, migration,
                           Tokio runtime, engine spawn, GPUI run, shutdown
crates/vela-core/          pure domain: events, transcript, Markdown boundaries,
                           layout, warm cache, documents, text, search, palettes
crates/vela-opencode/      OpenCode V2 discovery, HTTP client, SSE, typed parsing
crates/vela-engine/        orchestration: commands, store, persistence, files,
                           observation, recovery, Git, Sass, preview, runtime,
                           providers, queue, terminal relay
crates/vela-terminal/      PTY + terminal emulator (no GPUI)
crates/vela-browser/       native WKWebView/Wry host and URL policy
crates/vela-ui/            GPUI presentation layer and UI-owned view state
archive/vela-electron/     read-only Electron reference implementation
docs/                      architecture, operations, migration, parity evidence
scripts/                   verification and parity tooling
```

Dependency direction:

```text
apps/vela ──> vela-ui ──> vela-engine ──> vela-opencode ──> vela-core
     │             │            └────────> vela-terminal
     │             ├────────> vela-browser
     │             └────────> vela-core / vela-opencode / vela-terminal
     └────────> vela-engine
```

`vela-core` has no GPUI, network, PTY, or platform dependency. `vela-opencode`
and `vela-terminal` know nothing about GPUI. `vela-browser` owns a native view
but no engine commands. `vela-engine` never imports GPUI. `vela-ui` is the
outermost composition layer.

## 4. Runtime topology

- **GPUI main thread:** owns `VelaApp`, focus, layout, input, panels, and
  virtualized lists. It reads engine snapshots and sends commands; it does not
  await HTTP, disk, PTY, database, compiler, or WebView work.
- **Tokio runtime:** four worker threads created in `apps/vela/src/main.rs`.
  Owns service discovery, HTTP, SSE, projection, persistence, filesystem
  workers, recovery transactions, Sass, previews, provider flows, and PTYs.
- **Engine:** a cloneable handle over `Arc<Inner>`. Commands enter through an
  unbounded channel; state changes are announced with `EngineUpdate`
  notifications. Some updates also carry targeted payloads (terminal bytes,
  file content, directory entries); full state is pulled through snapshots.
- **OpenCode shared service:** a separate process normally reached over
  authenticated loopback HTTP + SSE. Vela discovers a registered V2 service or
  starts `opencode serve --service`. The registration file is trusted local
  state; the adapter does not independently enforce loopback. OpenCode owns
  conversations, credentials, and service registration.
- **Native surfaces:** one WKWebView per browser tab through Wry; one PTY per
  terminal tab through `portable-pty`; the terminal grid is interpreted by
  `alacritty_terminal`.

## 5. Primary end-to-end flows

### Session open and prompt

```text
UI action
  -> Engine command (open / prompt / interrupt / queue)
  -> vela-opencode Client (authenticated HTTP, normally loopback)
  -> OpenCode session/message/prompt endpoints
  -> SSE frames
  -> vela-core event normalization
  -> transcript projection + warm-cache update
  -> EngineUpdate::Session(id)
  -> VelaApp pulls SessionView blocks/revision
  -> GPUI repaints only affected visible rows
```

`vela-core::transcript` projects message parts into stable renderable blocks.
The engine batches ordinary deltas on a 16 ms cadence and flushes critical
events immediately. Reconnect emits a synthetic `server.connected` so open
sessions can be re-materialized instead of trusting missed deltas.

### File open, edit, save, and external change

```text
Explorer/tab action
  -> engine file command
  -> confined workspace read (bounded)
  -> vela-core::document::Documents owns the buffer
  -> editor view owns focus/selection/scroll
  -> save command with expected content
  -> recovery transaction / compare-and-write
  -> FileSaved or conflict notification
  -> UI reload, conflict controls, or recovery record
```

Observation is per attached workspace, debounced, bounded, and serialized with
reads/writes. A clean external change reloads; a dirty change preserves local
text and pauses saves behind conflict controls. See
[editor.md](architecture/editor.md) and [engine.md](architecture/engine.md).

### Settings and provider lifecycle

Settings are schema-versioned JSON in Vela's own profile, written atomically.
Provider credentials and OAuth remain engine-owned; the UI renders typed forms
and redacted state. A settings change applies to the live engine and reports
success or an error; it never rewrites the legacy Electron profile.

## 6. State ownership and persistence

- `vela-engine::store::Store` owns the session index, live `SessionView`
  handles, warm cache, permissions, todos, usage, connection state, protocol
  drift, and preview servers.
- Each `SessionView` owns its transcript, draft, scroll anchor, model, and agent
  under short-lived locks. No store lock is held across an await.
- `vela-core::warm::WarmCache` is bounded by both session count and estimated
  bytes. It retains summary, projected transcript, draft, scroll anchor, and
  selection so a warm switch paints before the network responds.
- Vela's profile is `vela-native` under the platform data directory
  (`VELA_DATA_DIR` overrides it). Settings and window dimensions are
  schema-versioned and atomically replaced.
- OpenCode owns the transcript database and service registration. Vela does not
  duplicate them.
- The legacy Electron profile is read-only input for one-time, idempotent
  imports of workspace bookmarks/locations, window size, Sass preferences, and
  renderer-localStorage pins.

## 7. Cross-boundary changes

For any change crossing a boundary, trace the full path:

1. producer/UI action or external event;
2. contract (type, command, update, event, file format);
3. transport/channel;
4. state owner and persistence;
5. consumers/renderers;
6. failure, cancellation, restart, and recovery behavior;
7. focused tests plus the affected integration flow.

Do not fix only the visible endpoint. Prompt, file, settings, provider,
terminal, and browser changes usually cross at least UI → engine → adapter or
platform → state → UI.

## 8. Security and trust boundaries

- The OpenCode client uses the service registration's URL and basic-auth
  credentials. The registration file is trusted local state; the adapter
  currently validates the reported V2 version/PID but does not independently
  require a loopback host. Credentials stay in the adapter/engine boundary.
- No engine control HTTP server is exposed by Vela. Preview servers are
  app-owned loopback Vite children, not an engine command API. Any future mobile
  or background endpoint must use authenticated loopback IPC and a documented
  lifecycle.
- HTML/CSS validation is an explicit outbound exception: when the user invokes
  validation, document text is posted to the W3C validator endpoints in
  `vela-engine/src/validation.rs`. Do not describe Vela as fully offline.
- The browser has no Node, preload, or page-to-engine bridge. Navigation,
  popups, downloads, and external handoff are scheme/host constrained; only
  credential-free HTTP(S) is allowed, and external handoff is HTTPS-only.
- Browser tabs share an application-owned nonpersistent WebKit data store on
  macOS; history and cookies do not become a durable user profile.
- Workspace file access is session-relative, canonicalized, traversal-rejected,
  symlink-rejected, and bounded. Writes use recoverable compare-and-write
  transactions. External single-file tabs are an explicit exception: they
  read/write absolute paths and deliberately bypass workspace recovery.
- Untrusted Markdown is parsed into native elements; raw HTML is not injected
  into a web surface. External links are credential-free HTTPS only.
- No `unsafe` is allowed in owned crates without a documented platform
  requirement.

## 9. Toolchain and platform

- Rust stable from [`rust-toolchain.toml`](../rust-toolchain.toml), with
  `rustfmt` and `clippy`. Formatting verification is opt-in until the imported
  tree's pre-existing rustfmt debt is cleared as a dedicated change.
- Dependencies are locked by [`Cargo.lock`](../Cargo.lock); do not add a second
  version pin outside the manifest/toolchain owners.
- GPUI is pinned to the Zeron `zui` fork revision recorded in the workspace
  manifest. The fork supplies blur/edge-fade/clipping behavior; see
  [ADR-002](migration/decisions.md).
- macOS arm64 is the primary target. CI runs on `macos-15`. Windows and Linux
  have not been built or validated and no parity is claimed there.
- OpenCode V2 must be on `PATH` or supplied via `VELA_OPENCODE_BINARY`.

## 10. Verification and truth

The canonical local/CI gate is:

```sh
bash scripts/verify.sh
```

It checks documentation links, Clippy, the workspace tests, and an app build.
`--release` selects the release build. `--check-format` adds rustfmt
verification for a dedicated formatting change; formatting is not enforced in
the normal gate because the imported tree has pre-existing rustfmt drift.
Direct GUI, VoiceOver, provider, browser, PTY, packaging, and mobile acceptance
remain separate and must be reported as unverified when the required access is
unavailable.

Status claims belong in the parity matrix, not in this overview. A passing test
proves only the behavior it exercises; it does not establish visual parity.
