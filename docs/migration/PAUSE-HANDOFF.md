# Pause / resume handoff

Snapshot: **2026-11-03**, canonical repository `/Users/tdawg/code-repositories/vela`,
branch `main`. The native working tree, including previously untracked source,
tests, assets, scripts, and parity evidence, was imported and committed here.
The legacy Electron app is archived at `archive/vela-electron/`; the pre-merge
history is tagged `pre-native-merge`.

## Verified state

- Latest authoritative gate: **1013 passed, 0 failed, 1 ignored**.
  - `docs/parity/2026-10-25/user-message-meta-final-workspace-tests.txt`
- Clippy: `docs/parity/2026-10-25/user-message-meta-final-clippy.txt`
- App build: `docs/parity/2026-10-25/user-message-meta-final-build.txt`
- Flake isolation: `docs/parity/2026-10-25/user-message-meta-final-isolated-flakes.txt`
- Feature matrix: 65 Partial, 23 Implemented, 7 Planned, 2 N/A.
- No page has passed live visual acceptance.

Re-run the gate with:

```sh
bash scripts/migration-cargo.sh clippy --workspace --all-targets -- -D warnings
bash scripts/migration-cargo.sh build -p vela
bash scripts/migration-cargo.sh test --workspace
```

Known intermittent tests are listed in `START-HERE.md`; rerun them isolated
before re-running the full workspace gate.

## Repository merge

- Canonical checkout: `/Users/tdawg/code-repositories/vela`.
- Legacy Electron app: `archive/vela-electron/` (read-only reference).
- Pre-merge Electron HEAD: tag `pre-native-merge` (`7df3bd45`).
- Original native checkout: `/Users/tdawg/code-repositories/vela-native`
  (backup only; no longer the working repository).
- Build output (`target/`), local toolchains (`.tools/`), and `node_modules/`
  were intentionally not imported. Rebuild them from source when needed.
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
- **Composer model/agent menus** — remaining keyboard edge cases and menu
  appearance; Source menu behavior is authoritative.
- **Queue chips** — matched source comparison for mixed runtime/local rows;
  current behavior is already covered by rendered tests.
- **Message replay** — tool/selection/shell presentation gaps discovered against
  Source replay fixtures.
- **Copy response/message** — user text selection/context-menu matrix. User
  agent/model metadata is implemented; timestamps and selection remain.
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
