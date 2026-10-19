# Operations: running, verifying, and debugging

> **Document role:** canonical owner for running, verification, smoke testing, operational debugging, and recovery procedures.

Practical know-how for running the app, smoke-testing it, and debugging
renderer/main behavior without guessing. Everything here has been used
and verified in real sessions.

## Prerequisites

- Node 22.23.2 and npm. `.node-version`, `package.json` engines, and CI select
  the supported Node 22 range starting at 22.23.2, which satisfies the full
  locked dependency graph.
- `opencode` V2 on PATH (checked with `which opencode`). The app connects
  via `Service.discover()` and falls back to spawning
  `opencode serve --service` itself, so a service is not strictly
  required to be running beforehand.

## Running

```sh
npm run dev      # electron-vite dev with HMR (main/preload/renderer)
npm test         # Vitest unit/component tests in jsdom
npm run test:platform # native launcher, PTY, trust and filesystem lifecycle checks
npm run build    # compile then launch the production app (one command)
npm run build:compile # compile only -> out/ (no launch)
npm run generate-icons # regenerate the macOS ICNS from resources/icon.png
npm run pack     # build + package a real macOS app -> release/mac/Mira.app
npm run install-app # build + package + install Mira.app into /Applications (macOS)
npm run check    # typecheck, tests, docs check, and compile-only build
npm run smoke:opencode # isolated V2 service + session/event smoke (temp HOME)
npm start        # launch the existing production build with electron-vite preview
```

`npm run build` compiles into `out/` and immediately launches the result;
`npm run build:compile` is the compile-only form (used by CI and `npm run
check`) and `npm start` launches an existing build without rebuilding. The
portable Node launcher prepares and selects the branded app bundle on macOS and
uses plain Electron on Linux and Windows, without shell-specific environment
syntax. After a manual build you can also launch with `npx electron .`.
`npm run pack` and `npm run install-app` are macOS-only: both run
`scripts/install-app.mjs`, which builds `out/`, packages the app with
electron-builder (`electron-builder.yml`, `asar: false`, unpacked
`Contents/Resources/app` layout), and ad-hoc re-signs the bundle;
`install-app` then replaces `/Applications/Mira.app` (removing any
existing copy first, `cp -R` preserving the signature). The installed bundle is
a live launcher: `scripts/install-app.mjs` strips the packaged `out/`,
`node_modules/`, and `resources/` payload and replaces it with
`scripts/live-launcher.cjs` plus a two-line `package.json`, so the app keeps
its own Electron runtime and Dock icon but loads the main process from the
repository's `out/`, rebuilding automatically first when repository sources are
newer than the build (silently; if the rebuild fails it falls back to the last
known good build after confirming). On macOS,
`npm run install-app` drives the same script over the
`shell:install-app` channel. These local development/test builds have GitHub
release updates disabled. They keep loading this checkout's `out/`, which lets
you build and test local edits without the Mira updater fetching or changing
your source checkout. `release/` and `build/` are gitignored builder outputs.

Downloaded releases use a separate update path. Push a tag whose name matches
the app version (`v<package.json version>`, for example `v0.1.0`) to run
`.github/workflows/release.yml`. CI builds both macOS architectures, creates a
DMG and ZIP, signs and notarizes them, then publishes them to the existing
GitHub release repository with electron-builder's update metadata. Settings → About in a release
build checks the latest published stable release; a commit pushed to `main`
does not become an app update until it is included in a published release.
**Update Mira** downloads that release, shows progress in a standalone window,
then closes Mira while electron-updater installs the bundle and restarts the
app. This release updater never fetches, merges, builds, or writes to a Mira
source checkout or a user's workspace. It remains blocked while editor files
are unsaved. On failure the current app remains available and the popup offers
a Return to Mira action.

The release workflow accepts the Mira-prefixed repository Actions secrets for
a base64 encoded Developer ID Application `.p12` certificate, certificate
name and password, App Store Connect API key, key ID, and issuer ID. It falls
back to the existing ORBIT-prefixed values so current release credentials keep
working until they are renamed.
The app's release update feed is configured in `electron-builder.release.yml`;
local packaging remains in `electron-builder.yml`.
`npm run test:platform` also opts into the real workspace-observation
close/reopen and rename/move tests through `scripts/workspace-observation-smoke.mjs`.
The ordinary unit suite skips these OS event-delivery checks.

`npm run test:platform` also runs the hidden-window renderer trust smoke on
macOS. Linux and Windows run the launcher and Electron PTY coverage but skip the
GUI smoke because a normal `BrowserWindow` requires a display there; macOS CI is
the targeted GUI lifecycle host. The trust smoke uses the built main process,
bundled preload, local packaged renderer, and a local `data:` document. It
checks trusted IPC, same-frame external navigation denial, popup denial, and
untrusted-document IPC rejection without external network access. `npm run typecheck`
runs `tsc --noEmit` for both node and web configs. `npm run check` is the
canonical local and CI verification gate.

### Architecture fitness and measurement

`npm run architecture:check` runs automatically in `npm run check` and the
existing GitHub Actions verify job. It uses the installed TypeScript parser,
with no additional tool dependency. [Architecture ownership](architecture.md#implementation-ownership-and-fitness)
defines the protected boundaries; [the manifest](../scripts/architecture-baseline.json)
owns numeric limits, grandfathered debt budgets, and reviewed exceptions.

New production modules use the default module/function length, complexity, and
nesting budgets. Existing oversized owners and functions have frozen allowances;
moving or renaming a function does not transfer an allowance automatically.
Static IPC channel names and JSX element/attribute names anchor callback
identities so deleting or reordering registrations or handlers cannot transfer
their budgets. The check fails
new cycles, forbidden process/private-owner imports, privileged renderer/shared
imports, raw IPC authority outside main composition, imports of excluded
implementation, and substantial new exact token duplication. Duplication
detection ignores comments, whitespace, and function declaration names; it does
not claim to detect semantic equivalence or renamed local variables.

The manifest is validated before budgets are applied: malformed numbers, missing
sections and invalid dependency paths fail verification. Literal dynamic loaders
(including constant templates and import options) and import-equals participate
in dependency checks; computed loaders fail because their targets cannot be
verified. New production modules must match a named file or directory owner in
the existing [canonical ownership table](architecture.md#implementation-ownership-and-fitness).
Directory routes end in `/`; sibling prefixes do not inherit ownership. Existing
known files remain baseline debt, while missing route targets fail verification.
Active private and dependency-constrained owners retain canonical routes after
ratcheting; misspelled selectors or removal of the ownership table also fail.
The documented cyclomatic metric counts logical binary operators; short-circuit
assignment operators are not included in that frozen measurement.

The retained upstream stylesheet exclusion uses the exact paths and SHA256 hashes
in [the pinned inventory](../scripts/pinned-upstream-styles.json), rather than a
directory exemption. Unlisted files in that directory count as production and
receive the normal architectural checks. Present pinned files are checked for content drift; deletion does not count as
production savings and still requires normal import/build and diff review.
Changed pinned content fails verification;
upstream upgrades require an explicit provenance and measurement review before
updating the inventory. The architecture ratchet never updates these pins.

After a verified extraction, run `npm run architecture:ratchet` and review the
manifest diff in the same checkpoint. Ratcheting first requires a passing check;
it only lowers/removes debt allowances and records current production paths.
It never blesses new violations. An unavoidable exception needs a specific
rationale and independent diff review; keep it confined to the existing
responsibility and remove it when that debt is resolved.

`npm run architecture:measure -- --output /private/tmp/mira-architecture.json`
records the exact source inventory, token-bearing noncomment production lines,
function hotspots, dependencies, clone groups, unused-export candidates, and
logical bytes under `out/`. Build first to refresh the build-size measurement.
Use `--ref <commit>` to measure source from a frozen checkpoint and
`--build-snapshot <json>` to supply its separately captured build inventory.
The report includes methodology and separate auxiliary/excluded categories;
unused-export or dependency candidates require caller/entry-point review before
removal. Baseline and final production scope must remain identical, including
adapted handwritten code and all newly introduced production modules.

`node scripts/benchmark-renderer-store.mjs [--ref <commit>]` measures the actual
Store event path and React/jsdom context commits against an isolated mocked
bridge. It retains full output, records consumer renders, uses three warmups and
nine samples, and includes source/bundle provenance. Its timings exclude native
IPC, filesystem/provider latency and Chromium layout/paint. Use paired processes
and report ranges; a relative difference in a single cold run is insufficient
evidence of a performance regression or gain.

The platform smoke scripts fail boundedly: the direct PTY and Electron-hosted
PTY checks have 10-second child watchdogs, the Electron parent kills the full
child process tree on timeout, and the macOS trust check has a 30-second
watchdog. The GitHub Actions platform-smoke job also has a 10-minute job
timeout so a native-process failure cannot leave CI running indefinitely.

Mira is macOS-first with supported development/runtime launch on macOS,
Linux, and Windows. CI runs launcher configuration tests and a real
Electron-hosted `node-pty` input/output/exit smoke on all three. This verifies
the native module and shell path but is not a GUI smoke test; window behavior
still requires the human checklist below. Terminals use the user's normal
interactive shell (`SHELL`, `COMSPEC`, or the platform default), not login mode.
This matches integrated-terminal expectations and avoids re-running login
session initialization for every tab. `node-pty` 1.1.0 uses Node-API, and the
Electron-hosted smoke verifies the locked binary directly, so no
`@electron/rebuild` lifecycle is required. The portable `postinstall` only
restores execute permission on node-pty's packaged macOS `spawn-helper`, which
the npm tarball does not preserve; Linux uses node-pty's direct `forkpty`
implementation and does not need that helper.

## Updating the OpenCode client

`@opencode/client` is pinned to an exact stable V2 release. The pinned contract
and the installed `opencode` binary drift independently: the binary is whatever
is on PATH (or an already-running service), while the compile-time contract
moves only when this procedure runs. Run `npm run client:drift` on a weekly
cadence — and before adopting any new protocol feature — to compare the pin
against the published `latest` tag; it exits nonzero when a newer stable
release exists.

The runtime binary is not pinned. Mira probes `opencode --version` on its
augmented PATH, accepts a registered V2 service during ordinary connection,
and otherwise asks `Service.ensure` to start a compatible daemon. In Settings
→ Model, **Sync installed version** asks `Service.ensure` for the exact version
of the CLI Mira resolves, then reconnects the event stream. This replaces a
stale same-major daemon when needed, including one upgraded outside Mira.

**Update to latest** runs OpenCode's own `opencode upgrade` command, then
performs the same exact-version sync. OpenCode chooses the install method it
recognizes, so Mira does not duplicate its installer logic. Installations
managed by a package manager that OpenCode does not recognize must be updated
through that package manager. Both Settings actions restart the shared
OpenCode service when its version differs; active agent runs in Mira or other
OpenCode clients may be interrupted, so Mira confirms before proceeding.

When the event contract moves to a new V2 major line, raise
`MIN_SUPPORTED_SERVER_MAJOR` in `src/main/opencode.ts` so discovery and
`Service.ensure` refuse servers too old to speak it; the predicate receives the
service's reported version string. Update it only in an explicit dependency
commit:

1. Run `npm install --save-exact @opencode/client@<version>` on the supported
   Node version and review that the lockfile changes only the intended client,
   protocol, schema, and necessary transitive packages.
2. Review generated client method signatures used by `src/main/opencode.ts`
   and service discovery/authentication imports. Adapt that isolation boundary
   deliberately rather than bypassing types.
3. Review protocol event changes against `docs/events.md` and the replay/event
   fixtures in the main and renderer tests. Add or update captured protocol
   fixtures for every changed event shape, including handled and intentionally
   ignored events.
4. Run `npm run check` and `npm run test:platform`; run `npm run smoke:opencode`
   to exercise discovery, session create/read/delete, and the event stream
   against the installed CLI in an isolated HOME. Exercise the human GUI smoke
   checklist when service or streaming behavior changed.

## Usage data plugin (prototype)

`plugin/mira-usage.ts` is an OpenCode plugin that runs inside the OpenCode
server process. It resolves active provider connections through the supported
integration API (`ctx.integration.connection.active/resolve`) instead of Mira
reading OpenCode's credential storage directly, fetches ChatGPT and Claude
usage, and writes `{generatedAt, results}` — `results` matching
`ProviderUsageResult[]` — to `${XDG_DATA_HOME ?? ~/.local/share}/opencode/mira-usage.json`
(override with `MIRA_USAGE_SNAPSHOT`). `fetchProviderUsage()` in
`src/main/provider-usage.ts` always requests live data for credentials it can
resolve and uses a snapshot younger than 15 minutes only as a per-provider
fallback. This keeps explicit refreshes live without losing plugin-supplied
providers when direct credential resolution is unavailable.

Install it by copying (or symlinking) the file into OpenCode's global plugins
directory or listing its path under `plugins` in `opencode.json(c)`, then
restart the service; verify the snapshot file appears within a few minutes.
The prototype covers two providers; porting the remaining adapters from
`provider-usage.ts` and eventually deleting the scraping path is tracked as
follow-up work.

Provider credential discovery and refresh ordering are characterized with
isolated fixtures in `src/main/provider-credentials.test.ts` and
`src/main/provider-usage.orchestration.test.ts`. They mock credential reads,
SQLite, and HTTP; tests do not contact provider endpoints or inspect real
credentials. Sequential refresh is deliberate: parallel requests would alter
which later providers are contacted after unexpected parsing failures.

## Large-session benchmark

Run the deterministic renderer fixture independently with:

```sh
npx vitest run src/renderer/src/large-session.performance.test.ts --reporter=verbose
```

The JSON lines report reducer/update latency, derived timeline latency, an
estimated timeline-row proxy, retained output characters, and actual React/jsdom
rendering for 2,400 fixed events. On 2026-09-29, with complete supplied output
retained once per tool result, the fixture measured 6.05 ms, 0.16 ms, 800 rows,
and 13,107,200 retained characters. See [renderer retention](renderer.md) for
the transcript lifecycle policy. The test performs one
warmup and gates the median of five measured runs, reducing scheduler and JIT
noise seen in the full suite. Its deliberately generous CI budgets are a
100 ms median reducer/update time and 10 ms median derivation time. The
structural proxy budget is at most 1,000 estimated rows; an output assertion
also checks that no supplied text is lost or duplicated. The representative
400-turn timeline produces 800 actual
`data-timeline-row` DOM nodes. Its deliberately generous 5,000 ms budget covers
React reconciliation and jsdom DOM construction and is intended to catch gross
regressions without becoming machine-speed flaky. It explicitly does not
measure Chromium layout, paint, compositor work, or browser memory.
Compare trends using the same Node version and machine rather than treating the
median or jsdom timing as a cross-machine browser benchmark.

## Smoke test checklist

1. `which opencode` — binary present.
2. `npm run build` (compiles then launches) with output captured to a log:
   `nohup npm run build > /tmp/mira-smoke.log 2>&1 &`
3. After ~10s check the log for `[mira]` console.error lines
   (`[mira] event loop error:` means the SSE subscription failed).
4. Verify the backend spawned its service:
   `pgrep -fl "opencode serve"` and
   `lsof -nP -iTCP -sTCP:LISTEN | grep -i opencode`.
5. On macOS, verify the Electron window is alive:
   `pgrep -f "Mira.app/Contents/MacOS/Electron"`. Use Task Manager or the
   platform process monitor on Windows/Linux.
6. GUI pass (needs a human): open a folder, send a prompt, confirm the
   agent dot turns green / titlebar says "working", text streams, tool
   cards appear with names and elapsed timers, and a file the agent
   touches shows a diff.

## Connecting the mobile companion

1. Keep Mira Desktop open and open **Settings → Mobile Setup**.
2. Follow the three on-screen steps, then choose **Generate pairing QR**.
3. Open the mobile companion app on the phone, choose **Scan QR code**, and scan the code shown by Mira Desktop.

The pairing QR is single-use and expires after 10 minutes. It advertises a
reachable direct address and a relay route when available; use a network
path that both devices can reach (for example, the same Wi-Fi or a shared
Tailnet). Keep Mira Desktop open while using the phone. When Mira can attach
the companion server to its shared OpenCode service, desktop and phone use the
same sessions.

If Mobile Setup reports that the companion checkout is missing, a source
checkout looks for the OpenChamber repository beside Mira or in the usual
`code-repositories` and `coding-projects` folders. Set `MIRA_MOBILE_HOME` to
the checkout containing `packages/web` and restart Mira to use another
location. An offline status includes the companion folder and server log path;
restart Mira after correcting the setup. If QR generation reports a pairing
error, quit and reopen Mira Desktop so it restarts the service from the current
companion checkout.

The manual URL/password flow remains available in the mobile companion for standalone
server deployments. Do not use `localhost` as the phone's server address.

## Manual recovery

Mira stores new save and file-rename transactions under
`<workspace>/.git/mira-recovery/` for ordinary Git repositories, or a hidden
`.mira-recovery-<path-hash>` directory beside the workspace on the same
filesystem. A non-Git workspace at a filesystem mount point uses
`<workspace>/.mira-recovery/` for same-filesystem durability. Legacy
`<workspace>/.openshell-recovery/` and `<workspace>/.orbit-recovery/`
transactions remain readable and are hidden from Explorer and file watching.
Mira moves each
validated legacy transaction to new storage on workspace activation when its
destination is free. Each transaction contains `manifest.json` plus one or more
artifacts named `original`, `temporary`, `proposed`, or `source`. On
activation, Mira purges settled transactions (`complete`, `failed`,
acknowledged) older than 24 hours and interrupted ones (`source-held`,
`held-validated`) older than 7 days. Younger artifacts are never removed
automatically, and Acknowledge never deletes bytes on its own.

1. Use the recovery notice's Open action to inspect each artifact.
2. Compare `originalPath`, the current canonical file, and the artifact before
   manually copying any bytes.
3. Use Acknowledge when review is complete. This updates `manifest.json` and
   hides the notice; it does not delete the artifact or directory.
4. If the UI cannot open the workspace, inspect the transaction directories
   directly. Do not remove them until applications that may retain old file
   descriptors have exited and the bytes have been reviewed. Once past the
   retention windows they are removed automatically on the next activation.

Activation may hard-link a held original back to a missing canonical path only
for an interrupted `source-held` or `held-validated` transaction, where
Mira is known to have removed that path. It never overwrites an existing
path and never replays completed, failed, or acknowledged history. If both
canonical and recovery versions exist, manual comparison is required. Normal
successful transactions retain their bytes but are acknowledged automatically,
so they do not create persistent recovery notices; their transaction
directories are purged once past the 24-hour retention window.

## Driving the renderer headlessly (CDP)

The UI can be inspected and driven over the Chrome DevTools Protocol.
Launch with a debug port, then evaluate JavaScript in the page:

```sh
npx electron . --remote-debugging-port=9222
curl -s http://127.0.0.1:9222/json/list   # find the page target + ws url
```

Node script pattern (Node 22 has a global `WebSocket`):

```js
const page = (await (await fetch("http://127.0.0.1:9222/json/list")).json())
  .find((t) => t.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
// send { id, method: "Runtime.evaluate", params: { expression, awaitPromise, returnByValue } }
// then read the matching { id } response from ws.onmessage
```

Useful expressions:

- `document.querySelectorAll('.agent-model option').length` — model dropdown state
- `window.openshell.openSession("/path/to/repo").then(JSON.stringify)` — open a session
  without clicking (the store reacts to the emitted `session` message)
- `getComputedStyle(...)`, `document.querySelector('.tree-row').innerText`,
  `[...document.querySelectorAll('.toast')].map(t => t.textContent)` — UI state checks

Caveat: `npm start` (electron-vite preview) does NOT forward
`--remote-debugging-port`; use `npx electron .` on the built output, or
`npm run dev` and attach to its window.

`node scripts/electron-devtools-smoke.mjs` (also part of `npm run test:platform`)
runs an isolated macOS Electron check against the compiled renderer. It selects
the Sessions actions container, clicks the mapped rule link with native mouse
input in the real DevTools Styles panel,
and verifies the owning SCSS file opens in a Monaco tab. It then edits and saves
padding through Monaco, verifies computed layout updates without a renderer
reload, and checks invalid-Sass preservation, recovery, and removed declarations.
It copies renderer output and application sources into an isolated fixture;
its backend stays offline and never edits checkout sources. The fixture keeps
Electron appPath separate from the executing checkout, matching the installed
live launcher. Rebuild before running it.

## Probing the OpenCode service directly

The service speaks JSON HTTP on a localhost port. Useful when the client
call "should work" but something fails — probe the raw API.

- Port: `lsof -nP -iTCP -sTCP:LISTEN | grep opencode` (the `opencode
  serve --service` process).
- Registration file: `${XDG_STATE_HOME:-~/.local/state}/opencode/service.json`
  holds `password`; the macOS desktop app also registers at
  `~/Library/Application Support/ai.opencode.desktop/opencode/service.json`.
- Auth: HTTP Basic, username `opencode`:
  `Authorization: "Basic " + btoa("opencode:" + password)`.
- Endpoints: `GET /api/model`, `GET /api/model/default`,
  `GET /api/fs/list?<query>`.
- **Query-serialization gotcha**: nested params use bracket notation —
  `location[directory]=/Users/ty/mira`. A JSON-stringified
  `location={"directory":...}` is rejected with
  `Expected object | undefined, got string at ["location"]`. The
  `@opencode/client` does this correctly itself
  (`appendQuery` in its generated code) — this only bites hand-rolled
  requests.

The V2 client package is importable from plain Node ESM (it emits explicit
`.js` specifiers), so scripted probes can use the SDK directly; raw `fetch`
against the routes above remains the quickest manual check.

The managed service binds one configured port (default 49374; change it with
`opencode service set port <port>`, which writes
`${XDG_CONFIG_HOME:-~/.config}/opencode/service.json`). A second isolated
service therefore needs its own port — `npm run smoke:opencode` allocates a
free loopback port and runs with a temporary HOME, so it never disturbs the
user's running daemon. The V2 install also keeps an `opencode2` compatibility
shim next to `opencode`; older clients that still spawn
`opencode2 serve --service` reach the same daemon through it.

## Known quirks worth remembering

- `model.list` / `model.default` accept an optional `location`; the app
  passes the session directory. Both work with 63 models enabled on the
  stock setup.
- The model dropdown only renders while a session is open (it gates on
  `session && models.length > 0`); it seeds `currentModel` from
  `model.default()` and live-updates from `session.model.selected`.
- Directory entries from `GET /api/fs/list` come with trailing slashes
  (`"src/"`); the backend normalizes them in `listDir` — the renderer
  must never receive raw entries.
- The renderer paints the compact Folded light splash with the Vela wordmark immediately, holds it
  for three visible seconds while loading the IDE, then opens an empty workspace
  without selecting a session. Workspace selection stays in the sidebar.
- `session.tool.called` events carry NO tool name — names come from
  `session.tool.input.started` (see `docs/events.md`).

## Local Sass worker verification

After `npm run build:compile`, run `npm run test:sass-worker` to exercise the
emitted worker with Mira's production Sass dependency in a disposable root.
It never installs workspace packages or contacts the network. For footer and
workspace-lifecycle verification use isolated mirror apps as described in
[the Sass service guide](sass.md).

### Embedded browser smoke

`node scripts/electron-browser-smoke.mjs` (also in `npm run test:platform`)
runs on macOS CI and launches an isolated Electron host and localhost guest, bundling the current
browser policy/guards. It exercises guest/worker isolation, permission denial,
actual guest IPC identity rejection and blocked attachment/navigation/redirects.
Popup OS launching is mocked; device denial does not prompt physical hardware.
Native macOS checks require an unlocked GUI outside restrictive shell sandboxes.
