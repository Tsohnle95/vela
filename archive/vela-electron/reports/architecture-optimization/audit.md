# Architecture campaign evidence

Baseline source: `a1c54a35c3856e56209110cf4db493754699ed00`.
This report records campaign findings and measurements. Durable ownership and
routing remain in `AGENTS.md` and the existing canonical `docs/` owners.

## Baseline and scope

`baseline.json` freezes methodology, categories, file hashes, code inventories,
function metrics, dependencies, and export candidates. Primary production is
38,430 nonblank, noncomment physical code lines in 152 files. The target is
26,901 lines. Tests/fixtures, generated code and the explicitly pinned upstream
OpenCode stylesheet set are excluded consistently. Adapted OpenCode renderer
implementations, handwritten declarations, dormant runtime source, and the
1,908-line Linguist extension/filename data table remain included conservatively.
New production files remain in the same scope; relocation is not reduction.

The initial supported-toolchain gate passed 989 tests in 123 files, typecheck,
documentation inventory checks, and production compilation. Native macOS PTY
and renderer trust checks also passed. Coverage percentage is not configured;
these results do not establish every live interaction or external runtime path.

The static baseline has 11 modules above 800 code lines, 37 functions above
100 code lines, and 54 complexity/nesting hotspots. There are no runtime import
strongly connected components and one explicit type-only markdown cycle.
The substantial exact function/block duplicate detector initially reports zero;
it does not detect all short helpers, renamed clones, or semantic equivalence.
Manual investigation confirmed duplicate tool metadata/content/path/detail
normalization, usage normalization and lifecycle/presentation sequences.
Export-reference and dependency candidates are investigation aids, not deletion
authorization: entry points, workers, contracts, tests, auxiliary code and dormant
adapters can legitimately have no direct production importer.

## Responsibility maps and implementation order

| Candidate | Existing responsibilities | Cohesive boundaries and retained authority |
|---|---|---|
| `store.tsx` | Session attachment/restoration/focus; editor/filesystem state; persistence; canonical chat plus auxiliary transcript; event routing; panel projections; React context | Extract pure reconciliation and panel projection before state ownership changes. Keep optimistic/abort/watchdog busy policy distinct from server status. Preserve captured workspace and focus generations, unsaved protection, failed-read hydration and detached busy sessions. Event subscription retains one lifecycle owner. |
| `chat-store.ts` | Canonical message/part mechanics, V2/legacy event reduction, transcript hydration and assistant projection | Record mechanics own replacement/ordering/delta overlap/terminal preservation. Projection/hydration depends on those records. Public reducer remains the stable event entry point. Cache validity requires both message-record and raw part-array identity, scoped to owning state. |
| `opencode.ts` | SDK/service management, context/session authority, SSE integration, replay, filesystem observation, mutation/recovery, runtime routing, catalogs and interaction calls | Extract pure replay first, recovery storage second, then evaluate observation/mutation ownership. Keep SDK communication and authoritative context/generation routing together. Storage receives captured roots/current predicates; mutation sequencing and post-await target checks remain at callers. Dormant injected runtime behavior is retained. |
| `index.ts` | Electron lifecycle/window trust, DevTools, unsaved-close/update protocol, IPC wiring, Sass/preview/mobile integration | Group file, agent and terminal registrars through the unchanged centralized trusted gate. Keep dialog generation-before-await, window refs, lifecycle/update/preview integration in their existing host owner. Domain registrars never import the entry point or raw IPC registration. |
| `provider-usage.ts` | Credential discovery, provider-specific requests/normalization, disk snapshots and ordered fallback | Credential discovery and provider groups are potential owners. Candidate credential-path duplication is equivalent; provider serialization and fallback must be characterized before changing concurrency. Preserve expired/network/auth failure visibility and plugin fallback. |
| `AgentPanel.tsx` | Composer/completions/model preferences, usage, GUI/TUI mode, scroll memory, prompt dock and session composition | Composer domain/menu ownership, usage presentation, scroll and dismiss lifecycle are separate reasons to change. Keep keyboard, optimistic clearing, attachment spans, stale completion gates, GUI-only approval and TUI recovery behavior unchanged. Avoid transferring the entire component into another oversized hook. |
| `OpenCodeTimeline.tsx` | Pure tool/child-session/turn derivation, prose pacing, tool/edit/error/delegation presenters, chronological composition | Separate pure tool/delegation/turn models and cohesive presentation. Preserve partial JSON input, nested dormant-runtime tools, branch precedence, context grouping, delegation dedup and message/part keys. Lazy work must preserve failure and shell behavior. |
| `App.tsx` | Drag/tray lifecycle, panel columns, remembered geometry/mode, caps/distribution, shell composition and unload protection | Separate interaction ownership and pure geometry from shell composition. Keep direct DOM drag previews, one final state commit, singleton/session swap memory, parent geometry, manual mode exit and mounted editor/settings state. Grid paths have distinct filtering/sidebar/restore semantics. |

Backend replay precedes recovery; recovery precedes any mutation extraction.
Canonical records precede projection optimization; pure reconciliation precedes
identity reuse. UI transformations precede presenter migration. Each slice uses
exclusive file ownership, targeted characterization, independent diff review,
supported canonical verification and a recovery commit.

## Characterization and critical invariants

Backend tests include replay retention, main-to-renderer reconciliation,
runtime routing, multi-context/session retention, workspace target/security,
watcher phases and direct mutation crash/race/symlink/retention scenarios.
Registrar characterization covers malformed payload rejection before backend
calls, unchanged prompt failures/defaults, workspace authorization order,
terminal defaults and cleanup after a stale post-start check. Native trust
smoke exercises a handler registered through the extracted domain path.

Renderer tests cover record replacement, overlap reconciliation, terminal
tool snapshots, hydration/removal/reinsertion, matching IDs in isolated states,
previous projection immutability, parent attribution and cloned user metadata,
stream settle, concurrent restoration, permissions/forms, usage and catalog
selection. Component/layout tests protect keyboard/menu/TUI/file activation,
delegation/context grouping and direct DOM resize behavior.

IPC channels/payloads/errors, preload isolation, confined paths, no-replace
filesystem installation, fsync/recovery, session/workspace generations, concurrent
session routing, stream ordering/reconnect, unsaved edits and retained adapter
capabilities are protected. `AGENTS.md`, execution policy, documentation ownership
and runtime selection semantics are not being redesigned.

## Performance evidence and limits

The existing 2,400-event/400-turn fixture retains 13,107,200 tool-output
characters and renders 800 rows. Its first independent baseline measured
9.04 ms reducer, 0.24 ms derivation and 558.14 ms React/jsdom construction.
These timings are samples, not Chromium layout/paint or end-to-end latency.

`scripts/benchmark-chat-projection.mjs` reproduces 400 historical assistants,
32 KiB output per tool, and 500 tail deltas, with three warmups and nine samples.
It supports baseline Git source through `--ref` and checks unchanged final text,
tail identity changes and retained content. Concurrent validation increased
timing variability, so sequential baseline/final runs are required for acceptance.
Prototype timings are not reported as an integrated application improvement.

Broad Store context subscriptions still rerender consumers on unrelated state
updates; stable projections alone do not establish selective subscriptions.
Sharing same-root filesystem observation could reduce reads, but per-session
baselines and watcher lifetimes are intentional. No shared-watcher rewrite is
justified without a representative benchmark. Existing unrelated lifecycle or
memo dependency defects are recorded separately rather than bundled into extraction.

## Reduction constraints

Extraction improves ownership but generally saves few lines. Initial bounded
audits support hundreds of genuinely redundant lines, not 11,529. Protocol
variants, failed-request handling, generations, recovery artifacts, editor
conflict protection, intentionally retained runtime source, data tables and
presentation/cascade compatibility cannot be discarded to fill that gap.
Final measurements must show actual gross removals and net reduction; no 30%
claim is justified by the discovery evidence alone.

Design experiments, mockups and prototype plugins have no established deletion
policy. They remain preserved. No-production-consumer exports and test-only
helpers likewise require evidence beyond an import count before removal.
