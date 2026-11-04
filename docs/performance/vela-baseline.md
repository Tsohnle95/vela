# Original Vela baseline

## Environment

- Apple M3 Pro, 12 cores, 18 GiB RAM, macOS 27.0.1 arm64.
- Original checkout at `ba995c142c353722945b0c116694155409dfd8a8`.
- Node available: 26.10.0; repository requires `>=22.23.2 <23`
  (`.node-version` = 22.23.2). No supported Node 22 runtime is installed.
- Electron 37 and OpenCode V2.1.4 are present; `out/main/index.js` exists.
- The checkout contains pre-existing user changes in
  `design/vela-product/styles.css` and `mockup-design/`; they were not touched.

## Status: blocked

No baseline numbers are recorded. Inventing them would violate the assignment.

Concrete blockers:

1. **Unsupported Node runtime.** The canonical gate requires Node 22.x; only
   Node 26.10.0 is on PATH. Running a modified build with a different runtime
   would not be a valid baseline for the shipped app.
2. **No non-invasive instrumentation.** Vela has no startup/navigation
   signposts or frame metrics exposed to an external harness. Measuring first
   frame would require modifying the original app, which is prohibited.
3. **No interactive display capture in this session.** `screencapture` could
   not create an image from the display, and AppleScript/System Events access
   was unavailable. Driving the Electron UI and confirming a visually correct
   frame is therefore not possible here.
4. **No supported automation harness.** No Playwright/Electron test driver is
   configured for the production app; `tests/browser` in the reference
   MonoCode repository is not part of Vela.

## What was verified

- The original repository is intact; `git status` still shows only the
  pre-existing user changes.
- The original is runnable in principle (`out/main/index.js` plus Electron 37),
  but the supported-toolchain and UI-automation requirements above prevent a
  trustworthy baseline in this environment.

## How to unblock

On a machine with the pinned Node 22.23.2 and an interactive macOS session:

1. Build (`npm run build:compile`) and install the packaged app.
2. Add **external** instrumentation (no functional edits): Electron
   `--remote-debugging-port`, CDP timeline markers, and a screen-recording
   harness.
3. Capture startup, cold/warm session switch, 1,000-switch memory, idle CPU,
   streaming frame timing, and terminal input latency with the workloads in
   `methodology.md`.
4. Record the exact build, machine, and method here before comparing with
   native results.
