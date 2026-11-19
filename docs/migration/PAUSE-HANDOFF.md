# Pause / resume handoff

Snapshot: **2026-11-07**, canonical repository `/Users/tdawg/code-repositories/vela`,
branch `main`. The native working tree, including previously untracked source,
tests, assets, scripts, and parity evidence, was imported and committed here.
The legacy Electron app is archived at `archive/vela-electron/`; the pre-merge
history is tagged `pre-native-merge`.

## Verified state

- Latest authoritative gate: **1074 passed, 0 failed, 3 ignored**.
  - `docs/parity/2026-10-25/codex-conversation-width-gate.txt` (includes docs, workspace
    Clippy, workspace tests and the debug app build)
- `bash scripts/verify.sh --release` is green at 1072/0/3 with the current
  session/layout work, and builds the optimized app
  (`docs/parity/2026-10-25/release-gate-1072.txt`); this is build verification
  only, not signing, notarization, packaging or visual acceptance.
- The feature matrix remains the sole owner of per-feature status; do not copy
  fixed counts there.
- No page has passed live visual acceptance.

Repository policy is [`AGENTS.md`](../../AGENTS.md); execution behavior is
[`docs/agent-execution.md`](../agent-execution.md). Re-run the canonical gate
with `bash scripts/verify.sh` (or `--release`); the offline migration wrapper
(`bash scripts/migration-cargo.sh ...`) remains available when its cache exists.
Known intermittent tests are listed in `START-HERE.md`; rerun them isolated
before re-running the full workspace gate.

## Repository merge

- Canonical checkout: `/Users/tdawg/code-repositories/vela`.
- Legacy Electron app: `archive/vela-electron/` (read-only reference).
- Pre-merge Electron HEAD: tag `pre-native-merge` (`7df3bd45`).
- Original native checkout: `/Users/tdawg/code-repositories/vela-native`
  (backup only; no longer the working repository).
- Build output, local toolchains, and `node_modules/` live under the sibling
  `build-junk/` directory, not in this repository. Recreate or delete them
  independently of source.
- The imported tree includes `docs/parity/` evidence and the migration notes.

## Resume documents

Read these in order:

1. `docs/migration/START-HERE.md` — current checkpoint, decisions, and the
   feature-area status log.
2. `docs/migration/feature-parity-matrix.md` — remaining Partial/Planned rows
   and their Source controls.
3. `docs/migration/completion-plan.md` — chronological completed slices.
4. `docs/parity/2026-10-25/checkpoint.md` — detailed evidence for recent slices.

## Evidence layout

- All current raw evidence lives under `docs/parity/2026-10-25/` and is roughly
  123 MB.
- Files ending in `-final-workspace-tests.txt`, `-final-clippy.txt`,
  `-final-build.txt`, and `-final-isolated-flakes.txt` are the authoritative
  gate records for their slice.
- Older or exploratory captures in the same directory are not authoritative.
- Do not delete evidence by name alone; preserve the `*-final-*` set.

## Next code-only candidates

The matrix is the source of truth; start with rows that can be finished without
GUI, credentials, or a signed build:

- **`@` file references** — large-catalog and remaining keyboard edge cases;
  preserve Source's raw-path insertion and mention-span semantics.
- **Workspace bookmarks / relocation** — large-history performance and
  context/keyboard behavior; do not invent new bookmark controls.
- **Queue chips** — matched source comparison for mixed runtime/local rows;
  current behavior is already covered by rendered tests.
- **Message replay** — presentation, shell, `replayTodos` and interrupted
  markers are closed in core/rendered/engine-flow tests. Live `state.progress`
  is retained as data only; the visible line was reverted with the tool-card
  capture correction. Remaining work is live visual acceptance.
- **Copy response/message** — the live `VelaTurn` exposes only the answer
  Copy/Copied action plus duration; user bubbles have no metadata, timestamp,
  attachment chips or row actions, and staged revert has no live trigger.
  Remaining work is user text selection/context-menu visuals and paired
  captures; do not re-add `TimelineProse`-derived rows.
- **Tool card presentation** — converged on the captured live `VelaToolRow`
  (summary rows, exec/patch-only expansion, exec footer, patch diff). Do not
  reintroduce `OpenCodeTool`-derived IN/OUT, copy, FILES, TOOL CALLS,
  duration or progress-line sections; remaining work is paired visuals and
  VoiceOver.
- **Transcript divider token** — verified dead: the `divider` item type has a
  `Session compacted` renderer but no producer in the current Source renderer.
  Do not add core modeling for it; compaction is the plain status note.
- **Compaction baseline** — now persisted per session in engine settings and
  saved when first inferred. Remaining work is only rendered/visual acceptance
  of the note and context percentage.
- **Tiled transcripts** — Source renders the full `VelaTranscript`/`AgentPanel`
  inside every tiled panel. Assistant and user text now use the real
  `markdown_view` surfaces, expanded work bodies use shared tool cards, the full
  history renders, and each tile follows its tail with a scroll-to-bottom
  control and close/reopen distance memory. Tiles are virtualized with a
  per-panel `gpui::list`; the block-level `turnProcess` split is covered, and the
  remaining scroll difference is the full `useVelaScroll` behavior.
- **Remaining structural code slices** — beyond live acceptance, the known
  code-only work is: finish full KaTeX layout coverage. Tile virtualization,
  block-level `turnProcess` projection and Agent Mode horizontal free movement
  are implemented; Source's resize handles leave Agent Mode rather than
  resizing vertically. The per-panel composer toolbar (attach/model/agent/strength,
  with compact Prompt options and picker overlays) is now implemented and
  tested. Browser download history/shelf is a native
  affordance, pending a paid visual decision against Source.
- **Layout polish** — sidebar and agent panel edges are resizable; a
  screenshot-by-screenshot spacing pass is still needed, and the 800×600 editor
  breadcrumb collapses to single letters.
- **Streaming reveal** — the live `VelaMarkdown` keeps full text and fades
  settled segments; do not reintroduce the `TimelineProse` PacedMarkdown
  24 ms prefix projection. Per-segment opacity fade in GPUI is still open and
  is visual work.
- **Markdown live-surface audit** — fences are now gated to the live
  `VelaCodeBlock` highlighter set; the `bundledLanguages` tokenizers from the
  unmounted `opencode-markdown` engine are test-only and must not be wired
  back in. Re-audit inline code, KaTeX math, tables, images and segment fade
  against `VelaMarkdown.tsx`/`VelaMarkdown`'s custom parser and the captures
  before extending them.
- **Send/stop morph** — transcript terminal/settlement edge cases.
- **File explorer / external import** — simultaneous multi-path ordering and
  remaining traversal/symlink edges; engine import already has basic symlink
  protections.
- **Git status/branch** — first audit Source controls. Earlier audit found no
  Source branch-switching/staging/commit UI, so do not invent one; document the
  finding and only add Source-backed behavior.
- **Accessibility** — platform focus/keyboard matrix; table header description
  and transcript focus outlines are already implemented.

## Blocked / user-assisted work

These require runtime, display, credentials, or release access and should not be
claimed as complete from code tests:

- Live/paired visual acceptance for every page and theme/size/zoom.
- VoiceOver and platform focus acceptance.
- Live provider OAuth/key refresh and provider usage fetches.
- Live WebKit permission/isolation, downloads, and browser lifecycle checks.
- Full PTY/IME, candidate-window, and TUI appearance acceptance.
- Packaging, signing, notarization, install, and update paths.
- Mobile companion/shared-session work (deferred).

## Repo-organisation guardrails

- Keep `docs/parity/2026-10-25/` intact; it is the raw evidence trail.
- Reference implementation: `archive/vela-electron/` is read-only. The active
  native app is the repository root.
- Do not delete `archive/vela-electron/`; it preserves the Source behavior that
  the native app is matched against.
- If files are moved, update the evidence paths in `START-HERE.md`, the matrix,
  `completion-plan.md`, and `checkpoint.md` together.
