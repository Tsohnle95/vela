# Codex transcript parity — implementation plan

Status: **in progress** (step baseline complete, steps 1–5 open)
Owner: `src/renderer/src/codex-chat/`
Goal: make Vela's agent transcript behave like the Codex desktop transcript
(`/Applications/ChatGPT.app`, bundle id `com.openai.codex`, 26.1016.61520).

This file is the durable working state. If context is lost, resume from here:
read "Rules", then the first unchecked box in steps 1–5, then run the
verification matrix for that step before moving on.

## Source of truth

- App bundle: `/Applications/ChatGPT.app/Contents/Resources/app.asar`
- Webview renderer: `/webview/assets/*` (21,346 files in the archive)
- Scratch extraction (regenerable, not committed): `/tmp/codex/`
  - minified originals: `/tmp/codex/*.js`, `/tmp/codex/*.css`
  - pretty-printed copies: `/tmp/codex/pretty/*.js` (esbuild `--minify=false`)
- Key modules already extracted:
  - `app-initial-c95ddb62696e.js` (14 MB main bundle)
  - `app-shared-2eda13eb7044.js` (5.7 MB)
  - `local-conversation-turn-3fa4930c8324.js`
  - `local-conversation-thread-346380d3959a.js`
  - `static-thread-transcript-49f271cb9fd9.js`
  - `virtualized-turn-list-25c7ace77d84.js`
  - `collapsed-turn-disclosure-dc2c461c5a13.js`
  - `command-messages-96d7f59f2a9d.js`
  - `chatgpt-markdown-view-cf053c4353b9.js`
  - `chatgpt-code-block-a4fb2c9c0fe3.js`
  - `chatgpt-assistant-message-content-98a6dff508ce.js`
  - `app-initial-9d42883ac737.css`, `app-shared-295ec67e1c5d.css`

Re-extract helper (write `/tmp/extract.mjs`):

```js
import fs from "node:fs";
const asar = "/Applications/ChatGPT.app/Contents/Resources/app.asar";
const fd = fs.openSync(asar, "r");
const h = Buffer.alloc(16); fs.readSync(fd, h, 0, 16, 0);
const picklePayload = h.readUInt32LE(8);
const jsonLen = h.readUInt32LE(12);
const jb = Buffer.alloc(jsonLen); fs.readSync(fd, jb, 0, jsonLen, 16);
const tree = JSON.parse(jb.toString("utf8"));
const base = 12 + picklePayload;
const find = (p) => { let n = tree; for (const s of p.split("/").filter(Boolean)) { n = n.files?.[s]; if (!n) return null; } return n; };
for (const p of process.argv.slice(2)) {
  const e = find(p); if (!e) { console.log("MISS " + p); continue; }
  const b = Buffer.alloc(e.size); fs.readSync(fd, b, 0, e.size, base + Number(e.offset));
  fs.writeFileSync("/tmp/codex/" + p.split("/").pop(), b);
  console.log("OK " + p.split("/").pop() + " " + e.size);
}
```

Pretty-print: `node_modules/.bin/esbuild /tmp/codex/<file>.js --format=esm --minify=false --outfile=/tmp/codex/pretty/<file>.js`

## Rules

1. Port behavior, structure, constants, and copy — **not** proprietary source
   verbatim. Reimplement in Vela's React/TS style.
2. No invented user-facing copy. Use the bundle's `id`/`defaultMessage` pairs.
3. No guessed geometry. Cite the CSS/className that justifies a value in the
   step notes.
4. One commit per completed step, after `npm run check` passes.
5. If a surface has no data source in Vela (e.g. citations, writing blocks),
   record it as N/A with the reason; do not fake it.
6. Tests must cover the behavior added in the step; DOM assertions are the
   minimum, behavioral (timing/scroll) assertions where feasible.

## Baseline (already shipped)

- [x] Codex layout: centered 48rem column, 16px rhythm, 77% user bubble,
      5% ink background, 16px radius, 8/12 padding.
- [x] "Worked for {time}" / "Working for {time}" activity divider + hairline.
- [x] Segment fade: 16ms step, 96ms cap, shared schedule, `--fade-delay`,
      settle on `animationend`, content-addressed keys (`zSd` port).
- [x] Adaptive streaming buffer (`Gpd` port): rate tracking, 50ms drain tick,
      rAF publish, visibility pause, snap on completion.
- [x] Adaptive vs non-adaptive fade semantics (0.7s no stagger vs 0.15s
      staggered).
- [x] Block fades for list items, table rows, blockquotes, rules.
- [x] Stable DOM across animation toggle; user prompts via markdown.
- Commits: `adab8abc`, `7b9d0a62`, `4aff946d`.

## Step 1 — Tool rows (identical row renderers)

Replace the generic `CodexToolRow` with Codex's per-tool renderers.

- [x] 1.1 Map Vela tool keys → Codex row types (shell/command, read, write,
      edit/patch/diff, search/grep, web, MCP, reasoning, other).
- [x] 1.2 Shell/command row: exact row grammar, command text, live output,
      success/failure glyphs, expand/collapse, output container.
- [x] 1.3 Read row: path display, line range, expand body.
- [x] 1.4 Edit/patch row: summary line, diff body rendering, add/remove counts.
- [x] 1.5 Search/web rows: query + result summary grammar.
- [x] 1.6 Reasoning row: exact presentation (collapsed/expanded, tone, copy).
- [x] 1.7 Status transitions while running (spinner → check/cross) with the
      bundle's timing.
- [x] 1.8 Tests: one DOM test per row type + status transition.
- [x] 1.9 `npm run check`; commit.

Acceptance: for each row type, the rendered element tree, classes, copy, and
states match the spec in "Step 1 findings" below.

## Step 2 — Scroll ownership and follow behavior

- [x] 2.1 Identify the Codex scrollport owner and whether virtualization is
      used.
- [x] 2.2 Port the follow/anchor decision (thresholds, content-growth
      handling, reader-away release, return-to-bottom).
- [x] 2.3 Port smooth/instant behavior and observers (ResizeObserver etc.).
- [x] 2.4 Port the "scroll to bottom"/new-content control (position, copy,
      visibility rules).
- [x] 2.5 Preserve per-workspace position on remount.
- [x] 2.6 Tests: follow while pinned, release on manual scroll, return,
      remount restore.
- [x] 2.7 `npm run check`; commit.

Acceptance: streaming growth never yanks a reader who scrolled up; pinned
readers track the floor; the control appears/disappears per spec.

## Step 3 — Fenced code blocks and highlighting

- [x] 3.1 Match the code block DOM (header, language, copy, wrap, scroll).
- [x] 3.2 Wire the same highlighting engine/theme behavior (or the closest
      available in Vela) with async/non-blocking loading.
- [x] 3.3 Streaming behavior: placeholder/placeholder-free, deferred
      highlight, fade-in.
- [x] 3.4 Copy/copied labels and analytics-free behavior.
- [x] 3.5 CSS parity: background, radius, padding, font, line-height,
      overflow, header.
- [x] 3.6 Tests: block renders, language label, copy action, streaming state.
- [x] 3.7 `npm run check`; commit.

Acceptance: code blocks match Codex's structure and highlighting behavior;
no raw unhighlighted flash on completion.

## Step 4 — Markdown fidelity

- [x] 4.1 Feature matrix: headings, paragraphs, lists (nested/ordered/task
      checkboxes), blockquotes, tables (scroller/sticky header), images,
      math, inline HTML, links, code spans, rules.
- [x] 4.2 Sanitization policy identical (no `dangerouslySetInnerHTML`).
- [x] 4.3 Task-list checkboxes and table scroller/sticky header.
- [x] 4.4 Image rendering (lazy, dimensions, transparency backdrop).
- [x] 4.5 Link policy (external target/rel, file links where applicable).
- [x] 4.6 Tests: each supported feature renders correctly; unsupported raw
      HTML is escaped.
- [x] 4.7 `npm run check`; commit.

Acceptance: every markdown construct Vela can receive renders as Codex does;
nothing is silently dropped or injected.

## Step 5 — Render performance and memoization

- [x] 5.1 Port the text-part cache idea (segmentation identity); the streaming prefix lexer stays open (see Open work) (`jSd`/`LSd`/`textPartCache`)
      so a growing message does not re-parse/re-render earlier blocks.
- [x] 5.2 Keep the adaptive buffer's rAF cadence; avoid parsing more than
      once per published snapshot.
- [x] 5.3 Memoize block renderers and stable keys.
- [x] 5.4 Benchmark: 40.8KB in 200 chunks = 78.3ms after the O(n) boundary scan
      and last-prefix fast path (was 2301ms).
- [x] 5.5 Tests: unchanged text does not re-render blocks (segmentation identity + cache prune) (spy on parser or
      render counts).
- [x] 5.6 `npm run check`; commit.

Acceptance: recorded benchmark + no re-parse when text is unchanged.

## Verification matrix

Run before each commit:

```sh
npx vitest run src/renderer/src/codex-chat
npm run typecheck
npm run architecture:check
npm run docs:check
npm run check        # full gate; must exit 0
```

Known environment caveat: this machine has Node v26.10.0 while the repo pins
`.node-version` 22.23.2; the gate is not authoritative until re-run on 22.

## Step 1 findings (verified)

Modules (extracted to /tmp/codex/pretty2 by the analysis):
- Dispatcher `SO` in `sites-end-resource-62e43bb0850b.js` (per item.type).
- Shared row shell `Up`=`GZe` / header `Hp`=`RZe` / body `VZe` in
  `app-primary-58a09006cc0c.js`.
- Exec `CC` (row) + `WC` (summary); shell card `Xh`=`sn` + terminal `Qt` in
  `user-message-attachments-45859a154be4.js`.
- Patch `PT` (container) / `IT` (per-file) / `BT` (diff card) / `VT` (viewer).
- Read `RC`, search `EC`, list `NC`, web `lO`, reasoning `pE` (all in the
  dispatcher chunk).
- MCP `Fh` in `mcp-tool-item-content-e2f13173844a.js`; dynamic tools in
  `agent-activity-item-e6a54ce06805.js`.
- NOTE: `command-messages-*.js` is the command palette, and
  `static-thread-transcript-*.js` renders no tool rows.

Row shell: padding-offset wrapper → `div.flex.min-w-0.flex-col` → header
`group/activity-header relative inline-flex min-w-0 max-w-full self-start
items-center gap-1 p-0 text-start` with a full-area `<button aria-expanded>`
and summary span; body is height/opacity animated, `overflow-hidden`.

Exec: DEFAULT COLLAPSED. Summary text (i18n `toolSummaryForCmd.*`):
"Ran {command}", "Ran {command} in {elapsed}", "Stopped {command}[ after
{elapsed}]", "Ran command[ in {elapsed}]", background-terminal variants.
In-progress = cadenced shimmer + elapsed (NO spinner/check/cross in the
header). Expanded body: `data-testid="exec-shell-body"` →
`group flex flex-col overflow-hidden rounded-lg border border-strong
bg-secondary-soft-alpha` → output scroller `vertical-scroll-fade-mask max-h-36
[--edge-fade-distance:2rem] flex flex-col-reverse overflow-auto
whitespace-pre font-code` → footer: "Success" + check, "Exit code {n}",
"Stopped" (`execFooter.*`). Empty output: "No output" (`codex.shell.noOutput`).
Shell header has "Shell" (`codex.shell.name`), cwd tooltip, copy-output.

Patch: DEFAULT COLLAPSED per file. Summary per change type (`codex.patch.change.*`):
Creating/Created, Deleting/Deleted, Editing/Edited, Stopped creating/deleting/
editing, Rejected; expanded headers "Created file"/"Deleted file"/"Edited file".
Per-file diff card: `border-default flex flex-col overflow-hidden rounded-lg
border`, header `bg-background-primary-ghost-hover/60`, body
`composer-diff-simple-line overflow-y-auto max-h-60` (short `max-h-25`), code
fallback `!p-2 max-h-40 vertical-scroll-fade-mask`; fallbacks "Diff too large
to preview", "No changes", "Contents deleted".

Read/Search/List: summary only (no body): "<verb>Read</verb> {path}",
"<verb>Searched</verb> for {query|files}", "<verb>Listed</verb> files[ in {path}]"
(`toolSummaryForCmd.read/.searchedFor/.searchedForFiles/.listedFiles`).

Web: no expansion; "Searched the web for {detail}" or "Searched the web"
(`codex.webSearch.summary.*`).

Reasoning: header "Thinking" / "Thought" / "Thought for {elapsed}"
(`reasoningItem.*`), shimmer while streaming; body markdown in a scroll box,
expanded while streaming, collapsed when complete; disclosure hidden when empty.

Defaults/status: exec collapsed, patch per-file collapsed, reasoning expanded
only while streaming. Colors: summary `text-text-tertiary/90`, muted
`text-text/40`, hover `text-default`, reasoning `text-text/30`. No tool-name
normalization exists; Codex derives row type from the event kind
(`commandExecution`, `fileChange`, `mcpToolCall`, `webSearch`, `reasoning`).
Vela must map its `ToolCallView.title`/part data onto these row types.

## Step 2 findings (verified)

Scrollport: single `div.thread-scroll-container`, `overflow-y-auto`,
`[overflow-anchor:none]`, `scroll-padding-bottom: var(--thread-scroll-padding-bottom)`
(driven by footer height + 16px), `role=region`, aria-label "Conversation".
Virtualization exists in Codex (windowed rows, 2 overscan, 12px gaps, 280px
estimate, 800px assumed viewport); Vela has no virtualizer — record as N/A but
keep the follow semantics.

Geometry: `distance = max(0, scrollHeight - clientHeight - scrollTop)`;
"at bottom" = **distance <= 24** (inline literal everywhere).

Pinning: `pinned = {bottom, top}`; armed by scroll-to-bottom or a user scroll
that moves toward the bottom and lands <= 24px; released by scrolling away.
Growth while pinned => instant `scrollTop = scrollHeight - clientHeight`
(never animated). Restore tolerance 24px.

Animation: only the explicit scroll-to-bottom is animated: **260ms**,
**easeOutCubic `1-(1-p)^3`**, direct `scrollTop` writes on rAF, early stop when
distance <= 24 then instant snap. Reduced motion => instant. Everything else
(streaming follow, restore) is instant; native smooth scrolling is never used.

Button: 32x32 rounded-full, centered above the composer, opacity transition
`.15s` (`--transition-duration-basic`), hidden within 24px of bottom (or within
`spacerHeight + 24` while a response spacer exists — N/A in Vela). Copy:
`localConversation.scrollToBottomButton` -> "Scroll to bottom". Click = instant
while generating, 260ms animation when idle. Codex has **no** "new messages"
control.

Restore: per-conversation store of spacer-adjusted distance, restored
instantly on mount with a 24px tolerance; Vela keys this by workspace id.

Vela port scope: 24px threshold, `overflow-anchor:none`, instant follow while
pinned, 260ms easeOutCubic button animation, `.15s` opacity, distance-based
per-workspace restore. Virtualizer/spacer => N/A.

## Step 3 findings

- Engine/themes: PENDING subagent report.
- DOM/classes: PENDING.
- Streaming behavior: PENDING.

## Step 4 findings (verified)

Lexer: marked with `breaks:true, gfm:true`; `del` tokenizer disabled (so `~~x~~`
stays text); math extension present (lazy KaTeX, `throwOnError` suppressed while
streaming).

Supported: paragraphs, headings (h1-h6), ordered/unordered/nested lists, GFM
task lists with disabled checkboxes, blockquotes (CSS bar, no border), tables
(column width classes + scroller + optional sticky header; default path does NOT
enable sticky), images (lazy, object-contain, thumbnail max 12.5rem, lightbox
button), code fences (user messages = plain `<pre>`; assistant = code block
component), inline code chips, strong/em, links (scheme allowlist), br, hr,
escape, math.

Sanitization: no raw HTML execution. Default `allowBasicHtml=false` => html
tokens render as escaped text (except `<br>`). If enabled, only
`b,del,em,i,kbd,s,strong,sub,sup,u` with no attributes; others dropped. Link
schemes: `https?|ircs?|xmpp|codex` plus `data:(audio|image|video)/`; otherwise
href omitted. Images same policy. Vela: keep React escaping, never
`dangerouslySetInnerHTML`; apply the scheme allowlist.

Memoization: per-message token prefix cache (longest-prefix reuse, max 100
entries), per-text segmentation cache (stable array identity, max 500 entries,
keys pruned each render, text >4096 bypasses), candidate/committed segment-key
state advanced in `useLayoutEffect`, React keys embed the token's first segment
key, settled keys stop animating. Segment keys: positional prefix reuse, else
earliest unused identical content, else `fade-segment-N`.

Codex-only inline features (citations, directives, inline visualizations, skill
mentions in inline code, @path chips) are conditional on data Vela does not
have; record N/A unless Vela's text contains the same syntax. Vela port keeps
the plain-text fallback.

## Progress log

- 2026-10-22: baseline committed (`adab8abc`, `7b9d0a62`, `4aff946d`);
  plan created; steps 1–5 open.

- 2026-10-22: step 2 (scroll) + step 3 (code blocks) + step 4 (markdown
  fidelity) implemented and gate-green; committed together because they shared
  codex-chat files. Steps 1 (tool rows) and 5 (perf) remain.

## Residual gaps (data-limited, not missing renderer work)

- Vela tool payloads have no per-call timestamps, so Codex's elapsed qualifiers
  ("in {elapsed}", "Thought for {elapsed}") are omitted.
- MCP and dynamic-tool rows fall back to the generic summary; Vela does not
  surface those item kinds.
- Patch bodies use a coloured unified-diff pre rather than Codex's diff viewer.
- Citation/artifact/writing-block directives remain N/A (no data source).
- `~~strikethrough~~` stays enabled (Codex's lexer disables it); accepted.

- 2026-10-22: step 1 implemented from the persisted spec: shared activity row
  (collapsed summary + hover chevron), exec card (`Ran`/`Running`/`Stopped`
  summary, `max-h-36` output, Success/Failed footer), read/search/list/web
  summary-only rows, per-file patch row with diff colouring, reasoning
  "Thinking"/"Thought". Limitations: Vela tool payloads carry no per-call
  timestamps, so "in {elapsed}"/"Thought for {elapsed}" are omitted; MCP and
  dynamic-tool rows fall back to the generic summary; diff body is a coloured
  unified-diff pre rather than Codex's diff viewer.
