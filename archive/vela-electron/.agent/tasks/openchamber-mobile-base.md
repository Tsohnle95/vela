# OpenChamber-mobile base for Vela/Vela

Status: ACTIVE (started 2026-10-16)

## Goal

Make the (rebranded) OpenChamber v2.3.3 mobile app pair with **Vela Desktop** exactly like
OpenChamber Mobile pairs with OpenChamber Desktop — by adopting upstream's mobile app + web server
and **embedding the server in-process in Vela Desktop**. No OpenChamber Desktop.

## Acceptance criteria

- [ ] Vela Mobile (rebranded OpenChamber mobile) pairs with Vela Desktop and connects, from anywhere
      over Tailscale.
- [ ] Vela Desktop hosts the server **in-process** (no external checkout, no system-Node dependency).
- [ ] Relay off (Tailscale reachability).
- [ ] User-facing identity rebranded (app name/id/icon/scheme); internal `OPENCHAMBER_*` names may remain.

## Base

- Reference clone: `/Users/tdawg/code-repositories/openchamber` @ tag `v2.3.3` (commit `7521e71`).
- **Adopt:** `packages/web` (server), `packages/mobile` (Capacitor shell), `packages/ui`, `packages/sdk`.
- **Do not adopt:** `packages/electron` (OpenChamber Desktop).
- Toolchain: bun (upstream pins 1.4.3; 1.3.14 installed OK here), Node ≥24.14 (have 26.10.0), TS 7,
  React 19, Vite 7, Express 5. Already on `@opencode/client 2.0.29`.

## Phases

| Phase | Scope | Status | Validation |
|---|---|---|---|
| 1 | Baseline: install + web build + run server | DONE | server serves `/health`, pairing routes |
| 2 | Rebrand map + minimal rebrand (mobile identity) | DONE | APK label/app id verified |
| 3 | Build rebranded Android app | DONE | APK built, installed, **connected** |
| 4 | Embed server in Vela Desktop (Electron node), wire OpenCode backend | DONE | gate + live host |
| 5 | Verify pairing over Tailscale (from anywhere) | DONE | phone paired to Vela-hosted server |
| 6 | Release plumbing + MIT attribution | pending | signed APK |

## Rebrand map (evidence)

- `packages/mobile`: 17 files. Key: `capacitor.config.json` (`appId com.openchamber.app`,
  `appName OpenChamber`), `android/app/src/main/res/values/strings.xml`,
  `android/app/build.gradle` (`namespace`/`applicationId`), `AndroidManifest.xml`,
  `ios/App/App/Info.plist` (`CFBundleIdentifier`, scheme `openchamber`), Java pkg
  `android/.../java/com/openchamber/app/*` (`MainActivity`, `OpenChamberWebView`, `FileSharePlugin`),
  Swift `OpenChamberAppShortcuts.swift`, `package.json` (`@openchamber/mobile`).
- Pairing scheme `openchamber://` lives in **`packages/ui/src/lib/connectionPayload.ts`** only
  (encode L196, parse L204, string-parse L227) + native scheme registration.
- `packages/ui` (661 files) / `packages/web` (490 files) are dominated by `OPENCHAMBER_*` env-var
  references — **keep those internal names**; do not global-rename.

## Decisions

- 2026-10-16 — Adopt OpenChamber v2.3.3 as the mobile base; retire the `vela-mobile` v1.20 fork.
- 2026-10-16 — Rebrand **user-facing identity only**; keep internal `OPENCHAMBER_*` env names + internals
  (keeps upstream rebasable).
- 2026-10-16 — Reachability = **Tailscale** (installed on Mac + phone); relay stays off.
- 2026-10-16 — Integrate by **embedding `@openchamber/web` in Vela Desktop's main process**
  (`startWebUiServer`), mirroring `packages/electron/main.mjs`.

## Integration reference

`packages/electron/main.mjs` (~L1589): sets `OPENCHAMBER_HOST`, `OPENCHAMBER_DIST_DIR`,
`OPENCHAMBER_RUNTIME=desktop`, `OPENCHAMBER_UI_PASSWORD`, `OPENCHAMBER_OPENCODE_CWD`,
`OPENCHAMBER_DESKTOP_NOTIFY`, then `startWebUiServer({ port, host, uiPassword, attachSignals:false,
exitOnShutdown:false, apiOnly:false, builtInExtensionsDir, onDesktopNotification, getIsWindowFocused,
getDesktopRuntimeConfig, desktopUpdater })`. Vela Desktop will replicate this and wire OpenCode via
`OPENCODE_HOST`/external backend.

## Risks / notes

- bun 1.3.14 vs upstream 1.4.3 (install succeeded; watch for build-time issues).
- Broad `ui`/`web` churn if env vars are renamed — avoid.
- Server must run on Electron's Node (proven: OpenChamber Desktop bundles it).
- MIT attribution must ship with the app.

## Session log

- 2026-10-16 — Cloned upstream @ `v2.3.3` → `/Users/tdawg/code-repositories/openchamber`.
- 2026-10-16 — `bun install` OK (bun 1.3.14). Web build OK (`packages/web/dist/mobile.html` emitted).
- 2026-10-16 — **Server verified**: booted in isolation (`OPENCHAMBER_DATA_DIR=/tmp/...`,
  `OPENCHAMBER_UI_PASSWORD=testpass`, port 3111). `/health` → `status:ok`, `runtime:web`,
  `openCodeRunning:true`, capabilities incl. `realtime.sse.v1`/`realtime.websocket.global-events.v1`.
  `POST /auth/session` → authenticated. `POST /api/client-auth/pairing/sessions` → pairing +
  `server.candidates:[{type:lan,url:http://127.0.0.1:3111,priority:10}]`. (v2.3.3 does **not** return
  `connectionUrl`; the UI builds the URI.)
  - Note: server spawned an `opencode serve` child (killed). Isolated data dir removed after test.
- 2026-10-16 — Toolchain: JDK 21 `~/Library/Java/JavaVirtualMachines/temurin-21.0.12.1+1.jdk/...`;
  Android SDK `~/Library/Android/sdk` (platforms 35/37.0). **Wrapper defaults
  (`/opt/homebrew/...`) are empty on this machine** → pass `JAVA_HOME`/`ANDROID_HOME` overrides.
- 2026-10-16 — Device: `SM_G781W` (`RFCNC0C4FBF`) on **Freedom Mobile LTE**, Tailscale 1.98.8
  installed, OpenChamber 2.3.3 (build 209) installed. Mac Tailscale currently **stopped**.

### Install-safety note (important)

Our build uses app id `com.openchamber.app`, identical to the installed OpenChamber. Installing it
would **replace the user's app and its pairings** (and signature mismatch would block `-r`). **Rebrand
the app id (e.g. `com.vela.mobile`) before any device install** so it installs alongside OpenChamber.
- 2026-10-16 — **Rebrand applied** (minimal, rebasable): appId `com.openchamber.app` →
  `com.vela.mobile`, app name → `Vela`. Android Java package moved `com/openchamber/app` →
  `com/vela/mobile` (7 files), `build.gradle` namespace+applicationId, `strings.xml`,
  `activity_main.xml`, `capacitor.config.json`. **Removed** upstream's committed
  `google-services.json` (OpenChamber's Firebase project; push out of scope). Kept internal
  `OPENCHAMBER_*` env names and the `openchamber://` scheme (used by session links + tests).
- 2026-10-16 — **Rebranded APK built**: `package: com.vela.mobile`, `application-label: Vela`.
  Android sync needed `cap sync android` (upstream `sync` also tries iOS/CocoaPods, absent here).
- 2026-10-16 — **Installed + connected**: uninstalled the old Vela Mobile 0.4.0 (was also
  `com.vela.mobile`), installed the new build. Drove the in-app "Connect by address" flow against a
  local server via `adb reverse` (phone on LTE). App reached the server, showed the password prompt,
  authenticated, and **loaded the full workspace UI** ("New session" / "What are we working on?").
  Server issued a `mobile` client token (`authMethod: password`, `lastTransport: direct`,
  `devicePlatform: android`). **Client↔server connection path proven end-to-end.**
- 2026-10-16 — Mac Tailscale **could not be started from the CLI** (`Tailscale up` left it "stopped");
  the GUI app must start the network extension. Tailnet IP is `100.89.86.73`.

## Validation plan

### Server packaging (2026-10-16)

- Single-file bundling was rejected: the server lazily `require`s deps (zod's `./impl/format`) and
  reads ~51 sibling assets via `import.meta.url`/`__dirname` (e.g. `settings-registry.json`), so
  esbuild output fails at runtime. Dependencies are 1.5 GB dev tree, too big to ship.
- **Chosen approach**: `scripts/package-mobile-server.mjs` stages `packages/web` + `packages/sdk`
  into `out/mobile-server/`, rewrites the workspace dep to `file:../sdk`, and runs
  `bun install --production` for a minimal closure (**354 MB**, 182 packages). Entry:
  `node_modules/@openchamber/web/server/index.js`.
- `electron-builder.yml`: ship it at `Resources/mobile-server`. electron-builder's `extraResources`
  matcher **drops `node_modules`**, so the full tree is copied by an **`afterPack` hook**
  (`scripts/after-pack.cjs`); `out/mobile-server` is excluded from the app `files`.
  `scripts/install-app.mjs` runs the packaging script before electron-builder (`npm run pack`).
- `src/main/mobile-server.ts`: `resolveServerHome` now checks `process.resourcesPath/mobile-server`
  first (packaged), then `VELA_MOBILE_HOME` / sibling / default checkouts; carries an explicit
  `entry` path. Verified the staged server serves `/health` under `ELECTRON_RUN_AS_NODE`.
- **Verified packaged**: `npm run pack` → `release/mac-arm64/Vela.app` ships
  `Resources/mobile-server/node_modules/@openchamber/web/server/index.js` (**339 MB**) and the
  app's own binary runs it (`ELECTRON_RUN_AS_NODE=1 Vela.app/Contents/MacOS/Vela <entry>`) serving
  `/health` (2.3.3).
- `vitest.config.ts` now excludes `out/**` and `release/**` (the packaged server's own tests must
  not be collected). `package-mobile-server.mjs` strips `*.test.*` from the staged tree.
- `npm run check` green (typecheck, tests, docs, architecture, build:compile) after updating
  `install-app.test.ts` for the added packaging invocation.

### Full rebrand (2026-10-16)

- **UI text**: codemod `\bOpenChamber\b` → `Vela` across `packages/ui/src`, `packages/web/src`,
  `packages/web/*.html`, `packages/mobile` (252 files, 3125 replacements). Compound identifiers
  (`OpenChamberLogo`, `OpenChamberPage`, …) and internal env names (`OPENCHAMBER_*`) preserved.
- **Deep-link scheme**: `openchamber://` → `vela://` in the app (`connectionPayload`, `sessionLinks`,
  `markdownCore`, `deepLinks` `DEEP_LINK_SCHEME`, QR scan, connection welcome), iOS `Info.plist`
  + widget/control Swift, and Vela's pairing builder (`src/main/mobile-server.ts`). Internal
  `openchamber-ui://` (Electron/VS Code origin) and `openchamber:` event names intentionally kept.
- **Icons/splash**: generated from `vela/resources/icon.png` (navy `#18263C`) — Android launcher
  (5 densities, adaptive background + foreground, monochrome removed), iOS `AppIcon`,
  Android/iOS splash, web PWA/favicons, mobile source assets.
- **Logo**: in-app `OpenChamberLogo` now renders Vela's mark; web `public/*.svg` + the web
  pre-paint loader replaced with Vela's mark; iOS display name + bundle ids `com.vela.mobile`.
- **Verification**: fork `type-check` green (all packages), `ui`/`mobile` lint green, UI tests
  865/867 files (the 2 failures are 4 pre-existing, environment-sensitive markdownCore perf tests —
  confirmed identical on the pristine tree). Rebuilt APK: `com.vela.mobile` / label **Vela**,
  in-app "Connect to Vela" + Vela mark. **Re-paired the rebranded app to Vela Desktop over
  Tailscale with the new `vela://` scheme** — redeem succeeded, workspace loaded.

### Phase 4/5 completion (2026-10-16)

- Rewrote `src/main/mobile-server.ts`: Vela now launches `packages/web/server/index.js` using
  **Electron's own Node** (`ELECTRON_RUN_AS_NODE=1`, `process.execPath`) — **no system Node**. It sets
  `OPENCHAMBER_DATA_DIR`, `OPENCHAMBER_UI_PASSWORD` (persisted at
  `~/Library/Application Support/VelaMobile/ui-password`), `OPENCHAMBER_HOST=0.0.0.0`,
  `OPENCHAMBER_RELAY_HOST=off`, and `OPENCODE_HOST/USERNAME/PASSWORD` from the desktop daemon.
- `createPairingQr()` now builds the `openchamber://connect?v=2&p=<base64url>` URI itself from
  `{pairingId, secret, candidates, label, fingerprint, expiresAt}`, advertising a **Tailscale
  address first** (100.64/10), then LAN. Kept the IPC/type contract unchanged; updated
  `mobile-server.test.ts` (8 tests) and Mobile Setup copy.
- **Gate green**: `typecheck`, `test` (1425 passed), `docs:check`, `architecture:check`,
  `build:compile`.
- **Live proof**: launched the dev build; Vela hosted the server (`/health` →
  `openchamberVersion: 2.3.3`, listening `0.0.0.0:3011`) and auto-attached to Vela's OpenCode
  daemon. Minted a pairing session from the Vela-hosted server (candidate
  `http://100.89.86.73:3011`), redeemed it from the phone, and the app connected and loaded the
  workspace. Vela's server listed exactly one client (`authMethod: pairing`, `devicePlatform: android`,
  `lastTransport: direct`).

### Outstanding polish

- Full UI-string + icon rebrand (currently identity-level only; in-app copy still says
  "OpenChamber").
- Consider making the fork's `sync` Android-first (`cap sync android`) so `build:android:debug`
  works without CocoaPods.
- Packaging: ship the server inside `Vela.app` (currently discovered from the sibling/`VELA_MOBILE_HOME`
  checkout in dev).
- Optional: run-as-node script via a bundled launcher for release builds.

### Tailscale + pairing milestone (2026-10-16)

- Tailscale up on both: Mac `100.89.86.73` (`tys-macbook-pro-1`), phone `100.126.156.52`
  (`tys-s20-fe`). Ping phone from Mac 0% loss; server reachable at the Mac tailnet IP.
- App connected **over Tailscale** (phone on Freedom LTE) after entering
  `http://100.89.86.73:3111` + password → workspace UI loaded, server issued a `mobile` client
  (`authMethod: password`, `lastTransport: direct`).
- **Pairing redeem proven**: minted a real pairing session (`serverUrl=http://100.89.86.73:3111`),
  built the `openchamber://connect?v=2&p=…` URI, and drove it through the app's address field (which
  also accepts pairing URIs). Server marked the session `usedAt` and created a `mobile` client with
  `authMethod: pairing`. Relaunch auto-connected. (UI-driving via `adb input` caused an extra
  app-exit, but the protocol succeeded.)
- Note: the v2.3.3 server **does not** return `connectionUrl`; the client UI builds the URI. Vela
  Desktop will need to build `openchamber://connect?…` from `{pairingId, secret, candidates}`.

- Server: run `startWebUiServer`, assert `/health` + `POST /api/client-auth/pairing/sessions`.
- Mobile: build debug APK; manual pair against the running server over Tailscale.
- Vela: `npm run check` after the embedding change; keep `mobile-server.test.ts` contract.

### Vela rebrand + Android verification (2026-10-17)

- Product renamed Vela → Vela by the user mid-session. Mobile app rebranded to **Vela**:
  app id `com.vela.mobile`, label "Vela", Java package `com/vela/mobile`, deep-link scheme
  `vela://`, all UI text, app names. iOS left as-is (out of scope). Desktop builder
  (`src/main/mobile-server.ts`) + tests updated to `vela://connect`.
- Disabled the mobile self-updater toast (`MobileAppUpdateToast`) — it advertised upstream
  OpenChamber's Android build.
- **Verified on a physical Android device** (Freedom LTE → Tailscale → Vela desktop server):
  pairing via `vela://connect` redeemed; auto-connect on launch; agent prompt → **streamed
  "PONG" in 1.5s** (Nemotron); model picker + switch; session list; workspace Git/Changes,
  Files, Terminal (real PTY I/O `VELA_PTY_OK`), MCP; error surfacing
  (`provider.no-route` shown cleanly for an unroutable model).
- Finding: `commandcode/deepseek-v4.1-flash` (the persisted default) is unroutable on the
  current opencode daemon (provider credentials) — desktop/provider config, surfaced correctly
  by the app; switching to `opencode/nemotron-3.5-super-vl-free` works.
- Gaps: reconnect auto-retry while open (app drops to the connect screen; relaunch/tap reconnects);
  OpenCode 2.0.32 update banner (informational); QR camera scan not automated; push off.
