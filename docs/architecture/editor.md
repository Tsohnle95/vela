# Editor, explorer, and workspace review

**Owns:** the native source editor view, tabs/editor groups, file explorer,
workspace file mutations, observed-change review, Git status/diff presentation,
and bounded local language tooling.

**Does not own:** buffer/history truth (`vela-core::document`/`text`), file I/O
and recovery (`vela-engine`), the archived Electron implementation, or a
language server. Current Source uses Monaco workers; native implements bounded
local providers and deliberately does not embed Monaco or an external LSP.

## Ownership split

| Layer | Owner | Truth |
|---|---|---|
| Document buffer, tabs, dirty/conflict state | `vela-core::document::Documents` | text, baseline, requested save, relocation/removal transactions |
| Text editing primitives | `vela-core::text::TextBuffer` | UTF-8 text, UTF-16 platform ranges, grapheme selection, undo/redo |
| Editor view | `vela-ui::editor::CodeEditor` | focus, caret/selection geometry, scroll, folds, completion/menu UI |
| Pane layout | `editor_groups.rs`, `tabs.rs` | which document is active in each group; preview/pinned presentation |
| File tree | `explorer*.rs` | expanded/selected rows, lazy directory loads, create/rename/remove/drop requests |
| Filesystem | `vela-engine::files`, `observation`, `recovery`, `imports`, `detach`, `trash` | confinement, watches, atomic writes, recovery, external changes |
| Changes review | `changes_view.rs`, `engine::changes`, `git.rs` | context-scoped observed changes; read-only Git status/diff |

A document key is activation-scoped: `{ workspace, directory, rel }`, where
`workspace` is a UI context identity rather than a physical root. Two panels
pointing at the same directory get separate activation scopes, so their buffers,
tabs, selection, and undo history are independent. A session view borrows the
documents from its own scope.

## Editor behavior

- One logical line is wrapped and virtualized; only visible lines are painted.
- Native input supports Unicode, selection, clipboard, undo/redo, composition
  ranges, and platform UTF-16 accessibility ranges. OS IME sessions remain a
  live-validation gap.
- Multi-cursor, occurrence selection, add-cursor-above/below, line-end cursors,
  selection growth, and simultaneous typing/deletion are supported with
  Monaco-derived tests.
- Preview tabs become pinned on a text edit, including undo back to the
  original text. Selection movement alone does not pin.
- Closing a dirty document requires the close/save coordinator; closing a tab
  does not silently destroy its buffer.
- Save is explicit (`⌘S`), matching the current reference. Saves are ordered,
  compare-and-write transactions; a conflict pauses ordinary saves and exposes
  reload/merge/overwrite controls.
- Editor groups support up to four panes, source-shaped 2-column/2×2 layouts,
  draggable dividers, tab moves, and next-unassigned-tab targeting.

## Language tooling modules

| Concern | Modules |
|---|---|
| Syntax/grammar/folding | `syntax.rs`, `brackets.rs`, `inline_code.rs` |
| Diagnostics | `diagnostics.rs`, `semantic.rs`, duplicate validators (`calls`, `class_members`, `destructuring`, `enums`, `export_duplicates`, `import_duplicates`, `interfaces`, `object_keys`, `switch_cases`, `type_*`) |
| Completion/editing aids | `completion.rs`, `emmet.rs`, `auto_import.rs`, `code_actions.rs` |
| Navigation/understanding | `hover.rs`, `signature.rs`, `symbols.rs`, `rename.rs`, `highlights.rs` |
| Formatting | `formatting.rs` |
| Find/replace | `vela-core::search`, editor find UI |
| Validation | `vela-engine::validation` (W3C HTML/CSS), `preview` Sass diagnostics |

These are bounded, local implementations. Do not describe them as Monaco or a
full language server. Source's Monaco worker set remains the reference; the
parity matrix records unsupported diagnostics/completion/folding cases.

## Explorer and file mutations

- The explorer renders a lazy, virtualized tree with source-derived glyphs,
  selection/status dots, and per-workspace expanded/scroll state.
- Reads use session-relative paths and the engine's confined resolver.
- Rename, remove, and detach initiate a document-prefix reservation in
  `Documents` before the engine mutation, then reconcile open tabs/documents on
  the result. Import sends its bounded engine command directly without a
  document reservation. Drag-to-folder uses the rename path.
- Removal uses OS Trash or a recoverable transaction; never permanent deletion
  as a fallback.
- External single-file tabs use absolute-path `read_standalone`/`write_standalone`
  and deliberately bypass workspace confinement and recovery; do not describe
  them as workspace-confined saves.
- External drops are captured with their originating workspace; asynchronous
  results cannot retarget another workspace.
- Dirty descendants block or coordinate folder rename/removal; Save All and
  relocation states are explicit.

## Changes and Git

`changes_view.rs` is an observed-file review surface. It is **not** a Git
staging/commit UI. It shows context-scoped changes against the observation
baseline, can open file diffs, and keeps sibling contexts independent.

`vela-engine::git` provides read-only status and unified diffs. Branch mutation,
staging, commit, and non-Git baseline tracking are out of scope until they can
reuse Vela's document/recovery guarantees. Audit the archived Source controls
before adding any VCS UI.

## Invariants

- UI renders from `Documents`; async reads/writes carry request/document
  identity and stale responses are rejected.
- No filesystem I/O on the GPUI thread.
- Clean external changes may reload; dirty external changes must preserve local
  text and expose conflict controls.
- Rename/removal/workspace close must not resurrect stale paths through
  in-flight I/O.
- Same-root panels keep separate activation scopes, buffers, tabs, selection,
  and undo history.
- Syntax/language behavior must remain deterministic and bounded; no unbounded
  file scans or worker-thread leaks.

## Tests

```sh
cargo test -p vela-ui editor
cargo test -p vela-ui explorer
cargo test -p vela-ui changes
cargo test -p vela-core document
cargo test -p vela-engine files
cargo test -p vela-engine observation
cargo test -p vela-engine recovery
```

Inline editor tests cover navigation, selection, multi-cursor, folding,
formatting, completion, diagnostics, quick fixes, Emmet, symbols, rename,
brackets, and rendered viewport regressions. Explorer tests cover lazy rows,
selection, create/rename/remove/import/detach, collisions, dirty descendants,
and async workspace ownership. Engine tests cover confinement, observation
generation, rename/remove transactions, recovery, and conflict behavior.

## Neighboring owners

- Documents/text/search primitives: [core.md](core.md)
- File/observation/recovery engine: [engine.md](engine.md)
- Shell/panel/focus integration: [ui.md](ui.md)
- Reference behavior: `archive/vela-electron/src/renderer/src/` (read-only)
- Feature status and remaining Monaco gaps:
  [feature parity matrix](../migration/feature-parity-matrix.md)
