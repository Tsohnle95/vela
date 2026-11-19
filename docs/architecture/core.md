# Core domain (`vela-core`)

**Owns:** framework-independent Vela domain types and algorithms: OpenCode event
normalization, transcript projection, Markdown block boundaries, row-layout
arithmetic, warm-session retention, workspace documents, text editing, search,
appearance palettes, and terminal-tray gesture math.

**Does not own:** GPUI rendering, HTTP/SSE transport, filesystem I/O, PTY
processes, browser views, persistence files, or UI focus/selection state.

`vela-core` has no GPUI, network, PTY, database, or platform dependency. This is
an enforceable architecture rule, not a convention. See
[`crates/vela-core/src/lib.rs`](../../crates/vela-core/src/lib.rs).

## Module map

| Module | Responsibility | Read when |
|---|---|---|
| `event.rs` | Tolerant normalization of current `data`, legacy `properties`, and numbered OpenCode event names into `AgentEvent` | event ingestion or protocol drift |
| `transcript.rs` | Message/part model and renderable `Block` projection; replay, live deltas, tools, reasoning, errors, retries, queue and revert states | transcript identity or streaming behavior |
| `conversation.rs` | Chronological turns and nodes; message-level answer text, completion and interruption metadata | prose/tool order and answer footer semantics |
| `run_log.rs` | Serializable event identity, sequence/run ownership, ingestion deduplication and deterministic replay | stream replay and checkpoints |
| `activity.rs` | Cached human summaries, categories and disclosure/side-effect policy | compact tool traces and mandatory notices |
| `markdown.rs` | Streaming-friendly Markdown block boundaries and immutable prepared syntax | Markdown row splitting, parsing and cache admission |
| `layout.rs` | Measured-height cache, prefix offsets, hit-testing, visible-range math | virtualization or scroll arithmetic |
| `warm.rs` | Bounded LRU warm-session cache | navigation, retention, memory budgets |
| `session.rs` | Session summary, phase, usage, runtime ID | session list/status |
| `document.rs` | Activation-scoped document buffers, tabs, save/conflict/relocation state | editor ownership, save, external changes |
| `text.rs` | UTF-8 storage, UTF-16 platform ranges, grapheme navigation, selection, undo/redo | text input or editor semantics |
| `search.rs` | Unicode-safe literal/regex Find ranges and bounded highlights | Find/Replace |
| `appearance.rs` | Original palette IDs and colors | theme data |
| `tray.rs` | Terminal tray drag/snap-height math | terminal tray gestures |

## Transcript projection invariants

- **Blocks, not messages, are the render unit.** Each message part projects to
  one or more `Block`s. User text is one bubble row; assistant text is split
  into top-level Markdown blocks; reasoning, tools, shell, compaction, skill,
  activity, retry, and error rows are typed blocks.
- **Block identity is stable.** Text block IDs are `{part_id}#{index}` (user
  text uses `#0`); tool/reasoning rows use the part ID; synthetic rows use
  `{message_id}:activity`, `:error`, or `:retry`. Stable IDs let the list
  splices rows instead of resetting, and let layout retain measured heights.
- **Completed work is reused.** The per-part cache stores byte ranges and
  projected blocks. If a growing part still matches its previous prefix, only
  the last block is reparsed. A snapshot replacement reparses that part only.
- **Text is shared, not copied per row.** Blocks hold `Arc<str>`; cloning the
  block list is pointer-copy work, not character-copy work.
- **Streaming fences stay plain until settled.** An unclosed Markdown fence is
  not promoted to a styled code block mid-stream.
- **Projection is display-independent.** No GPUI element, font, or window state
  belongs in `transcript.rs`.

`Transcript::apply` accepts normalized `AgentEvent`s; `hydrate` accepts replay
messages. A matching tool call retains its first exposed part/block ID even
when authoritative history supplies a different runtime part ID. A fresh
history-only transcript starts with that history's IDs; native event-log replay
reproduces the recorded lifetime identities. Both paths must preserve tool
status and retained failure details. Test live-first and replay-first paths
when changing either.

`run_log::EventLog` accepts source event IDs once per session. ID-less events
receive namespace/sequence IDs; identical token bytes are never duplicate
identity. The envelope stores receipt/source time once, so replay reads no
clock. History checkpoints, optimistic prompts and quiet-settlement inputs are
serializable records too. Ingestion deduplication is separate from intentional
overlap reapplication after HTTP history reconciliation. Acknowledged payloads
can be drained; source ID/run metadata lasts until session retirement. The
engine owns persistence and acknowledgement.

`activity::ActivityCache` reuses summaries of unchanged tool payloads. Quiet
successes remain inspectable; failures, warnings, subagents and consequential
side effects remain surfaced. Projection preserves source/spawn
order regardless of completion order.
Routine read/check shell sequences can include quoted arguments, pipes and
semicolon/conditional separators. Presentation classification never authorizes
execution. Successful unsupported scripts/loops remain in the shared summary;
known destructive/external operations surface individually with human labels.
Runtime warnings are diagnostic lines, not arbitrary mentions in file contents.
Raw commands require explicit disclosure.

## Markdown boundaries

`markdown::split_ranges` / `split_blocks` find top-level boundaries for
paragraphs, headings, lists, and fenced code. They never split inside a fence
and tolerate an unclosed fence while streaming. `PreparedMarkdown` resolves
references against the original message part, retains exact source separately
from provisional display repairs, and reuses unchanged syntax. Its limits
bound retained derived syntax, not raw transcript bytes or process memory.
The engine prepares snapshots; `vela-ui::markdown_view` owns native styling,
selection, and geometry. A snapshot miss in a transcript renders shared raw
text, with no inline parsing on the UI thread.

## Layout and warm cache

`LayoutCache` keys measured heights by stable block ID plus a content
fingerprint. A changed block loses only its own measurement. Prefix offsets
provide O(log n) row-offset and visible-range math.

`WarmCache` is bounded by both entry count (default 6) and estimated retained
bytes (default 96 MiB). An entry retains the session summary, projected
transcript, draft, scroll anchor, follow flag, and model/agent selection.
Reopening a warm session is synchronous: the retained projection is promoted
first, then authoritative history reconciles asynchronously. A background
session may update its retained transcript; it must not silently reopen a panel
the user closed.

## Documents and text

`Documents` owns buffers keyed by `DocumentKey { workspace, directory, rel }`.
`workspace` is the UI activation/context identity, not the physical disk root.
Two panels pointing at the same directory receive distinct activation scopes,
so they keep independent buffers, tabs, selection, and undo history; a session
view borrows buffers from its own scope. An asynchronous response can only
update the document that issued its request. Preview tabs pin on a text edit,
including when the edit is undone; selection movement alone does not pin.

`TextBuffer` stores UTF-8, exposes UTF-16 ranges to platform input, navigates by
grapheme, and owns bounded undo/redo. Selection offsets are always UTF-8 byte
boundaries. Platform IME/composition state belongs to the UI, not this crate.

External changes are classified by `Documents::observe`: a clean buffer reloads;
a dirty buffer keeps text and pauses saves behind conflict controls. Rename,
removal, and workspace close use explicit relocation/removal transactions so
in-flight I/O cannot resurrect a stale path.

## Tests

```sh
cargo test -p vela-core
cargo test -p vela-core --release --test perf -- --ignored --nocapture
```

Inline tests cover event normalization, projection, Markdown, layout, warm
eviction, documents, text, search, and palettes. The ignored `perf` test emits
warm-switch and streaming measurements; it is not part of the default gate.

## Neighboring owners

- Protocol ingestion: [opencode.md](opencode.md)
- Engine state, persistence, and file recovery: [engine.md](engine.md)
- Markdown/transcript rendering: [ui.md](ui.md)
- Editor views and language tooling: [editor.md](editor.md)
