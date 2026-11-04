# Native performance results

All numbers below were produced in this environment; commands are reproducible.
No end-to-end frame-latency claim is made yet.

## Environment

Apple M3 Pro (12 cores), 18 GiB RAM, macOS 27.0.1 arm64, rustc 1.98.1,
OpenCode 2.1.4. Release profile for measurements.

## Core projection and warm-cache navigation

Command:

```sh
cargo test -p vela-core --release --test perf -- --ignored --nocapture
```

Fixture: 6 sessions × 400 messages, alternating user/assistant; each assistant
message has reasoning, three Markdown blocks (including a fenced code block),
and one completed tool call. 1,600 blocks per session. Warm cache: 6 entries,
256 MiB budget (the default app budget is 96 MiB).

Result (2026-10-23):

```json
{"sessions":6,"messages_per_session":400,"blocks":[1600,1600,1600,1600,1600,1600],
 "fixture_build_ms":13.20,"warm_retained_mb":2.37,
 "switch_p50_ms":0.145,"switch_p95_ms":0.613,
 "stream_500_deltas_ms":15.84}
```

Run-to-run variance on this VM: switch p50 0.15-0.18 ms, p95 0.20-0.61 ms,
and the 500-delta loop 14-16 ms. Both switch and delta timings exclude GPUI
paint.

Interpretation:

- `switch_*` is the core cost of taking a warm session, listing 1,600 projected
  blocks, and syncing layout order. It excludes GPUI paint. It is a lower bound,
  not the product's warm-switch target.
- `stream_500_deltas_ms` is 500 sequential text deltas applied to one assistant
  part with a full block-list read after each. That is ~28 µs per delta in the
  worst-case per-event pattern; the engine actually flushes at 16 ms cadence.
  Before removing per-delta message cloning and adding the append fast path,
  the same loop took 138.34 ms (a 9-10x reduction in this harness).
- `warm_retained_mb` is the engine's conservative estimate, not RSS.

## Functional verification

Total workspace test count: 53 passing (plus one ignored release benchmark).

- `cargo test -p vela-core` — 18 passing.
- `cargo test -p vela-opencode` — 6 passing.
- `cargo test -p vela-engine` — 8 settings/migration tests plus the full adapter/mock
  end-to-end test (`tests/engine_flow.rs`): service discovery, replay,
  live SSE delta, prompt POST, permission event/reply, terminal execution
  state, and PTY output/input routing.
- `cargo test -p vela-terminal` — 5 emulator tests and one real-shell PTY flow.
- `cargo test -p vela-ui` — 4 terminal/editor tests.
- `cargo test -p vela-browser` — navigation-policy security tests.
- `cargo test -p vela-engine` — Git status/diff parsing, path rejection, and
  untracked-file diffs.
- In-app navigation harness — 71 warm switches across six 400-message sessions,
  p95 22.3 ms to the next frame (two foreground runs).
- Live OpenCode 2.1.4: connect, create session, list, replay, and 7-block
  hydration verified in the running GPUI app against an isolated service.
- Native terminal: `cargo test -p vela-terminal` (5 emulator tests) and
  `pty_flow` (real shell → PTY → emulator) pass; the engine end-to-end test
  also routes `echo` output through `TerminalData` and stops the PTY.

## Warm navigation — in-app GPUI harness

Command (fixture imported into an isolated OpenCode service):

```sh
VELA_NAV_IDS=ses_bench_1,ses_bench_2,ses_bench_3,ses_bench_4,ses_bench_5,ses_bench_6 VELA_NAV_WARM=3 VELA_NAV_OUT=/tmp/vela-nav.json ./scripts/bench-nav.sh
```

Fixture: six sessions, each 400 messages (200 user + 200 assistant), every
assistant message carrying reasoning, a completed tool call, prose, a list, and
a code fence. Warm budget 3 → four live views and two warm entries. The harness
preloads all six, then performs 72 switches at a 120 ms cadence and records the
time from the switch call to the render pass that commits the destination
blocks. It is not a painted-frame measurement, but it includes the real GPUI
event loop, engine activation, list splice, and block-list read.

Result (2026-10-23, release, window foreground; two runs):

```json
{"samples":71,"p50_ms":12.920,"p95_ms":13.419,"max_ms":13.502,
 "frame_samples":71,"frame_p50_ms":21.207,"frame_p95_ms":21.710,"frame_max_ms":21.763,
 "open_views":4,"warm":2}
{"samples":71,"p50_ms":13.634,"p95_ms":14.008,"max_ms":14.514,
 "frame_samples":71,"frame_p50_ms":21.867,"frame_p95_ms":22.300,"frame_max_ms":22.748,
 "open_views":4,"warm":2}
```

RSS was ~109 MB immediately after each run. `p50/p95_ms` is input → render
commit; `frame_p*_ms` is input → the next frame callback after that render,
which includes frame scheduling and paint submission but not display
presentation. The window must be foreground: occluded runs throttled the
display link and measured 40–55 ms, so they are not used.

The 50 ms target is stated for the first visually correct frame. This result is
below it but is not a presentation-timestamp measurement; keep that caveat when
comparing.

## Visual and large-transcript verification

Release build, isolated OpenCode service, macOS 27:

- A 1,000-message imported session opened and scrolled to the latest turn; the
  window painted correctly at 1,426×884. Process RSS was ~115 MB shortly after
  launch. This is a single observation, not a controlled idle/memory benchmark.
- Warm switching from the large session to a retained demo session showed the
  destination transcript immediately and kept `2 open · 0 warm`; the regression
  test asserts that warm promotion does not require a network reload.
- Codex is the default shell: 50px rail, flush 202px sidebar, editor, and 325px
  agent panel, with no top/left panel borders. The Classic floating-card shell
  remains selectable in Appearance and persists across launches.
- The file explorer read the workspace root and the native editor opened and
  displayed `README.md` with line numbers in the split view. Typing inserted
  text, `⌘S` wrote the file to disk, and the saved bytes were checked.
- Terminal interaction was driven in the running window: `echo keyboard-test`
  plus Return round-tripped through the PTY and painted the output.
- The Changes tab listed a modified and an untracked file, and the diff pane
  rendered a unified diff for the modified file with colored additions.
- Queue list, steer, and cancel round-trip through the engine against the mock
  service; the model picker loaded the live OpenCode catalog (Big Pickle, DeepSeek V4.1
  Flash Free, GLM-5.3-Flash Free, and others) and the footer reflected the
  selected agent/model; the settings surface reported the live endpoint,
  session count, warm budget, and profile path. Queue chips were not visually
  exercised because producing a genuinely pending item requires a busy run.
- The embedded WKWebView loaded `https://example.com` inside the native panel
  with GPUI-drawn address/navigation chrome; navigation-policy tests reject
  file/javascript/data/credentialed/custom-scheme URLs.
- Screenshots were captured from the native window by CGWindow ID (not a
  desktop crop) after hiding unrelated apps; the set below is the Codex shell.
- Screenshots: [landing](../assets/native-landing.png),
  [transcript](../assets/native-transcript.png),
  [editor](../assets/native-editor.png),
  [browser](../assets/native-browser.png),
  [diff](../assets/native-diff.png),
  [picker](../assets/native-picker.png),
  [settings](../assets/native-settings.png),
  [terminal](../assets/native-terminal.png),
  [Classic shell](../assets/classic-shell.png).

## Not yet measured

- First-frame warm navigation (p50/p95) in the GPUI app.
- Idle CPU and physical footprint.
- Eight simultaneous streaming panels.
- Terminal throughput and input latency.
- Browser memory.
- 1,000-switch memory growth.
- Cold startup.

These require the UI instrumentation harness described in `methodology.md`.
