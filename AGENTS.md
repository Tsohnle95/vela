# Vela — Agent Guide

Vela is a VS Code-style desktop GUI for coding agents: an Electron + React +
Monaco app that opens a repository, routes prompts through capability-aware
runtime adapters, streams agent progress, and shows live per-file diffs of
workspace changes observed during the active session. OpenCode V2 is the only
runtime enabled in the app; the DeepSeek Harness adapter remains dormant source.

Read this file first. Then use progressive disclosure: route to only the
module documentation and source needed for the task. The docs are designed to
be locally sufficient, not duplicated copies of the whole architecture.

## Quick start

```sh
npm install
npm run dev            # electron-vite dev with HMR
npm run typecheck      # tsc --noEmit for node + web configs
npm test               # Vitest unit/component tests in jsdom
npm run build          # compile then launch the production app
npm run build:compile  # compile only -> out/
npm run pack           # build + package installable Vela.app (macOS)
npm run check          # canonical verification gate
npm start              # run the existing production build
```

`opencode` must be on PATH (or an opencode service already running) for
OpenCode sessions.

## Context discipline

Use the smallest context that can safely answer the task.

1. Read `AGENTS.md`.
2. Route to the task-relevant doc(s) below.
3. Inspect the specific implementation files/symbols and nearby tests named by
   those docs.
4. Expand outward only when evidence shows another dependency or invariant is
   relevant.

Do not preload all docs, crawl the repository to "understand everything", or
read whole large files when a relevant symbol/section is sufficient. Do not
re-open information already established in the current task context.

## Module map

| Area | Path | Role |
|---|---|---|
| Main process | `src/main/index.ts` | Window, IPC handlers, backend wiring |
| Backend | `src/main/opencode.ts` | Runtime routing, session state, fs watching, baselines, OpenCode traffic |
| Runtime adapters | `src/main/runtimes/` | Adapter contract, capability manifests, durable runtime identity, dormant DeepSeek transport |
| Stream transport | `src/main/stream-pipeline.ts` | SSE batching, delta coalescing, snapshot barriers, heartbeat, reconnect |
| Provider usage | `src/main/provider-usage.ts` | Provider plan/rate-limit data |
| Sass compilation | `src/main/sass-watch.ts` | Local per-workspace compiler workers, preferences, and CSS disk output |
| Terminal | `src/main/terminal.ts` | `node-pty` manager for terminal tray and embedded agent TUI |
| Preload bridge | `src/preload/index.ts` | `window.vela` API exposed to renderer |
| Renderer store | `src/renderer/src/store.tsx` | UI state, concurrent sessions, backend event subscription |
| Chat store | `src/renderer/src/chat-store.ts` | Authoritative per-session message/part state and transcript projection |
| Streaming stack | `src/renderer/src/streaming.ts`, `session-activity.ts`, `assistant-status.ts` | Stream/session lifecycle and working status |
| Message queue | `src/renderer/src/message-queue.ts`, `messages/` | Native inbox follow-ups and local failure fallback |
| Renderer components | `src/renderer/src/components/` | Sidebar, editor, agent panels, TUI, welcome, terminal |
| Monaco setup | `src/renderer/src/monaco.ts` | Workers, theme, language mapping |
| Shared types | `src/shared/types.ts` | Contracts shared across main/preload/renderer |

## Task / symptom router

| Task or symptom | Read first | Likely implementation | Tests / validation |
|---|---|---|---|
| Agent response/stream/timeline is wrong | `docs/events.md`, relevant `docs/renderer.md` section | `chat-store.ts`, `streaming.ts`, `chat-stream.ts`, `store.tsx` | Tests/fixtures named by the routed docs; otherwise nearby stream/chat-state tests |
| Runtime/provider integration issue | `docs/architecture.md` runtime section, `docs/main.md` | `src/main/runtimes/`, `runtime-adapter.ts` | Runtime-adapter/provider tests or validation named by the routed docs |
| IPC/API change | `docs/main.md`, `docs/preload.md`, `docs/shared.md` | main handler → preload wrapper → shared type → renderer caller | Contract/IPC/bridge tests named by those docs |
| File watching/diff/baseline issue | `docs/architecture.md` diff section | `src/main/opencode.ts`, renderer store/editor state | Baseline/watch/diff tests or validation named by the architecture docs |
| Renderer state/component behavior | relevant `docs/renderer.md` section | owning store/action/component + nearby tests/SCSS | Owning store/component tests plus direct UI validation when acceptance is visual or interactive |
| Event contract mismatch | `docs/events.md` | stream pipeline, event normalization/reducer, fixtures | Event/normalization/reducer tests and relevant fixtures |
| Workspace Sass watching/CSS output | `docs/sass.md` | `src/main/sass-*`, `use-sass-watch.ts`, `EditorStatusMenu.tsx`, `TerminalTray.tsx` | Sass filesystem, worker and mirror UI checks named by the Sass guide |
| Terminal/TUI issue | terminal sections in `docs/main.md` and `docs/renderer.md` | `terminal.ts`, `TerminalTray.tsx`, `AgentTui.tsx` | Terminal/PTY tests and relevant runtime smoke validation |
| Build/start/package/debug issue | `docs/operations.md` | scripts/config and the failing subsystem | Relevant targeted check, then `npm run check`; use platform/runtime smoke where required |
| Cross-process flow is unclear | `docs/walkthrough.md` | follow the named connection points, then open canonical module docs | Tests/validation named along the traced producer → boundary → consumer path |

The Tests / validation column is a routing aid, not a second test inventory. Prefer stable test areas or validation routes over exhaustive filename lists. Canonical module documentation and the current source/test tree remain authoritative.

## Execution policy

Before editing, silently classify the work as **PATCH**, **FEATURE**,
**PROJECT**, or **BATCH**. Use the lightest mode that safely fits. Do not create
process artifacts merely to demonstrate compliance.

- **PATCH** — localized, well-understood change with clear acceptance criteria
  and straightforward verification. Inspect → edit → targeted validation →
  review diff → `npm run check` → commit.
- **FEATURE** — coordinated multi-file/component change or behavior requiring
  investigation. Make a concise in-context plan, implement in validated slices,
  then run the canonical gate and commit logical units.
- **PROJECT** — architectural/contract/security/concurrency/persistence change,
  migration, major uncertainty, or work expected to span several commits or
  contexts. Create and review a temporary task plan under `.agent/tasks/`, then
  implement phase by phase with verified Git checkpoints.
- **BATCH** — multiple independently completable outcomes. Triage the whole
  batch for dependencies/shared root causes, order or cluster it, then execute
  each logical unit using PATCH, FEATURE, or PROJECT rules. Do not keep every
  implementation active in one working context.

Classification is risk-adjusted, not category-triggered. Touching a shared
contract, persistence code, an IPC boundary, security-related code, or several
files does not by itself make work PROJECT-class. Escalate based on actual
ambiguity, blast radius, coupling, architectural novelty, failure cost,
testability, and the value of durable recovery state.

PATCH does not require `docs/agent-execution.md`. For a PATCH, the root guide,
routed domain documentation, relevant source, and nearby tests should normally
be sufficient. Load `docs/agent-execution.md` if the task escalates or when
working in FEATURE, PROJECT, or BATCH mode.

For detailed planning, escalation, delegation, and task-file rules, read
`docs/agent-execution.md` for FEATURE, PROJECT, or BATCH work.

### Escalate instead of patching blindly

Replan or move to a heavier mode when an architectural assumption proves
wrong, scope/coupling expands materially, tests reveal an unexpected dependency,
two repair attempts fail for the same underlying problem, unrelated refactors
start becoming necessary, or the agent can no longer state the invariant that
keeps the change correct.

Do not create a spec, design document, subagent hierarchy, or persistent plan
when direct execution is sufficient. Planning exists to reduce implementation
risk, not as mandatory ceremony.


### Implementation decision gate

Before editing, establish the intended observable outcome, the relevant
existing behavior, and the smallest coherent change that could achieve it.

For bug fixes, reproduce or otherwise establish the failure before changing
code when feasible. Distinguish observed behavior from suspected causes.
Do not modify code solely because it appears suspicious.

Prefer solutions that preserve existing architectural boundaries, contracts,
and ownership. Avoid adding abstractions, dependencies, or unrelated changes
without a demonstrated need.

Identify how the requested behavior will be verified before implementing it.
Use the narrowest meaningful check while iterating, then apply the required
completion gates. Passing unrelated checks does not establish behavioral
correctness.

Apply these rules proportionally in every execution mode. Simple changes
require no additional planning artifact.

## Architecture in one paragraph

The Electron **main process** owns the active OpenCode V2 connection and is the
only process that talks to OpenCode. The DeepSeek Harness implementation is
retained under `src/main/runtimes/deepseek/` but is not registered, probed, or
selectable by the app. The **renderer** keeps UI state and the authoritative
per-session chat projection. Main watches the repo and streams baseline/content
updates so the Changes/Diff UI reflects observed workspace changes while
preserving process and trust boundaries.

## Conventions

- Prefer self-explanatory code. Do not narrate obvious implementation with
  comments. Use concise comments for non-obvious invariants, protocol quirks,
  security assumptions, race-condition defenses, compatibility workarounds, or
  rationale that cannot be recovered from the code itself. Long-lived
  architecture belongs in docs; local correctness knowledge belongs beside the
  implementation it constrains.
- TypeScript strict; shared shapes live in `src/shared/types.ts` and are
  imported as `@shared/types`.
- IPC channels are named `shell:*`; shared backend message shapes live in
  `src/shared/types.ts`.
- OpenCode SDK calls remain isolated in `src/main/opencode.ts`; dormant
  DeepSeek-native code remains isolated under `src/main/runtimes/deepseek/`.
- Tree paths are session-relative, `/`-separated, with no trailing slash.
- `out/`, `node_modules/`, and `*.tsbuildinfo` are gitignored.

## Definition of done

Use targeted checks while iterating. Before a logical unit is considered done,
run `npm run check`; it runs typecheck, unit/component tests, docs checks, and
the production build. Never commit a knowingly broken checkpoint.

Canonical validation is authoritative only when run under the
repository-supported toolchain. Before treating `npm run check`, platform
tests, or other final validation as passing, confirm the active Node version
matches `.node-version` and satisfies `package.json` engines. An engine/version
mismatch requires rerunning final validation under the supported version.

The canonical gate does not prove behavior it does not exercise. When acceptance
depends on live UI appearance or interaction, full-process restart or recovery,
OS/platform behavior, or an external runtime/integration, run the narrowest
available check that directly exercises that behavior. If the required surface
cannot be verified in the current environment, state that gap explicitly rather
than claiming the behavior is fully verified.

## Git ownership and safety

The agent owns version control **for changes it creates**.

Investigation-only, review-only, audit-only, and planning-only work does not
create a commit unless the task intentionally produces repository changes.
Autonomous implementation work retains the checkpoint rules below.

- Record `git status` before editing.
- Never discard, reset, clean, stash, overwrite, or commit pre-existing user
  changes merely to obtain a clean tree.
- Preserve unrelated changes exactly as found.
- Commit each logically complete, independently sound unit after its required
  verification; large PROJECT work should normally have one checkpoint commit
  per completed phase.
- Include required project-brain updates in the same commit as the behavior or
  contract change that makes them necessary.
- Do not commit experimental debris or secrets.
- A commit is a recovery checkpoint. Only commit work you believe is correct.

## Documentation ownership

Permanent docs describe durable product truth; temporary task files describe
work in progress. Prefer one canonical owner for each class of fact and link to
it elsewhere rather than restating the invariant in different words.

| Truth | Canonical owner |
|---|---|
| Repository routing, conventions, execution entry rules | `AGENTS.md` |
| Execution modes, planning, delegation, replanning | `docs/agent-execution.md` |
| Cross-process architecture and architectural invariants | `docs/architecture.md` |
| Main-process behavior and IPC inventory | `docs/main.md` |
| Preload bridge contract | `docs/preload.md` |
| Renderer state/ownership/invariants | `docs/renderer.md` |
| Runtime event protocol and handling inventory | `docs/events.md` |
| Shared data contracts | `docs/shared.md` |
| Run/verify/debug procedures | `docs/operations.md` |
| End-to-end navigation through the system | `docs/walkthrough.md` (routing aid, not a second invariant owner) |
| Current multi-phase work | `.agent/tasks/<task>.md` |
| Runtime correctness | source + tests |
| Completed implementation history | Git |

## Docs maintenance (the project brain)

The permanent brain is `AGENTS.md` + `docs/`. Keep it aligned with code without
turning it into a prose clone of the implementation.

- `npm run docs:check` verifies documented surfaces and local references; it
  does not prove prose or runtime behavior. Tests own executable invariants.
  - Machine-readable facts should remain owned by their authoritative
  source/config/schema when one exists. Documentation should explain ownership,
  invariants, semantics, lifecycle, and routing rather than become a competing
  manually maintained source of the same fact.
- Repeat machine-readable facts in documentation only when the routing or
  explanatory value justifies the maintenance cost. Where practical, derive or
  mechanically verify those projections instead of relying on agents to keep
  parallel inventories synchronized by memory.
- Update the canonical owner when a durable architectural, contract, or
  operational truth changes. In non-owner docs, prefer a link/short routing
  note over duplicating the full explanation.
- Add handled/ignored events to `docs/events.md` as appropriate and keep the
  IPC/preload/shared inventories synchronized with their source surfaces.
- Pure refactors with no changed durable behavior or public/verifiable surface
  do not require documentation churn.
- `.agent/tasks/` is temporary working memory, not part of the permanent brain.
  On PROJECT completion, move only durable truths into canonical docs, ensure
  follow-up work lives in issues/backlog, then delete the completed task file.
