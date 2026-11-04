# Performance methodology

## Reference machine

| Field | Value |
|---|---|
| Hardware | Apple M3 Pro, 12 cores, 19,327,352,832 bytes RAM |
| OS | macOS 27.0.1 (build 26A434), arm64 |
| Toolchain | rustc/cargo 1.98.1, Apple clang 21.0.0 |
| OpenCode | 2.1.4 |
| Build | `cargo build --release` / `cargo test --release` |

Measurements from other machines are not comparable and must record their own
row.

## Definitions

- **Warm switch latency** — wall time from the session-selection input event to
  the first frame in which the destination transcript is visible with correct
  text at the restored anchor. State commit alone is not acceptable.
- **Cold session open** — from selection to first correct frame when the
  session is not in the warm cache and must be loaded from OpenCode.
- **p50/p95** — nearest-rank percentile over at least 50 samples, reported
  separately. No averages-only reporting.
- **Idle CPU** — process CPU over 60 s with no streaming, no pointer input, and
  the window focused, then repeated unfocused.
- **Retained memory** — RSS plus platform physical footprint; report both when
  available. Cache residency is reported from the engine's retained-byte
  estimate.
- **Frame timing** — GPUI frame callback intervals and main-thread busy time;
  report dropped frames for a 120 Hz display.

## Mandatory workloads

1. Six large completed conversations, switched repeatedly (60+ switches).
2. Eight simultaneously active sessions with concurrent streaming.
3. Multiple high-frequency streams (text, reasoning, tool input) at once.
4. A very long conversation with extensive Markdown and tool output.
5. Repeated switching while each session keeps a distinct scroll anchor.
6. Terminal output burst during conversation streaming.
7. Browser tabs open while switching agent sessions.
8. Repeated workspace open/close.
9. Idle after prolonged use (10+ minutes).
10. Cold startup and session restoration.

Fixtures live in `crates/vela-core/tests/perf.rs` for projection/navigation and
will be extended with UI harnesses. Each result must name the exact command and
git revision.

## What is measured today

Two harnesses exist. The in-app harness measures **input → render commit** for
warm switches across six sessions:

```sh
VELA_NAV_IDS=id1,id2,id3,id4,id5,id6 VELA_NAV_WARM=3 VELA_NAV_OUT=/tmp/vela-nav.json ./scripts/bench-nav.sh
```

It preloads six sessions, cycles 72 times, and records the time from the
`open_session` call to the render pass that commits the destination transcript.
It includes the GPUI event loop and engine warm promotion; it excludes Metal
paint and compositing. Results are in `native-results.md`.

The core harness measures **projection and warm-cache arithmetic** without a
window:

```sh
cargo test -p vela-core --release --test perf -- --ignored --nocapture
```

It builds six sessions × 400 messages (1,600 blocks each), hydrates them,
inserts them into a 6-entry/256 MiB warm cache, switches 120 times, and streams
500 deltas into one part while measuring each projection.

This is a lower bound. It excludes GPUI layout, text shaping, Metal painting,
and window compositing. It must not be presented as end-to-end navigation
latency.

## End-to-end harness requirements (not yet implemented)

- Instrument `VelaApp` with a signpost at input dispatch and at the first frame
  where the destination revision is painted.
- Use a deterministic window size and font.
- Drive selection through the same code path as a pointer click, not by calling
  internal methods.
- Record GPU frame time and main-thread time separately.
- Capture RSS and `phys_footprint` before/after 1,000 switches.
- Run with animations enabled and reduced-motion, separately.

## Baseline policy

The original Electron app is measured only with the same machine, workload, and
metric definition. If the original cannot be instrumented non-destructively,
the baseline is recorded as blocked, not estimated. See
`vela-baseline.md`.


## UI screenshot capture

Visual checks launch on a non-main display (`VELA_WINDOW_SCREEN=non-main` or
`scripts/run-secondary.sh`) so test windows never flash on the primary monitor.
They use the native window's CGWindow ID rather than a desktop-region crop. The ID is resolved from `CGWindowListCopyWindowInfo` by owner name `vela`,
then captured with `screencapture -x -l <id> -o`. This prevents another app's
window or a notification from being mistaken for Vela. The reference Electron
window was captured the same way for side-by-side comparison. Screenshots are
taken at the native 1426×884 window size, then downscaled for docs.
