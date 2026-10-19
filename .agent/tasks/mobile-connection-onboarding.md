# Desktop-to-mobile connection onboarding

Status: ACTIVE

## Goal

Make Vela Desktop's Mobile Setup a clear walkthrough that can generate a real pairing QR for Vela Mobile.

## Acceptance criteria

- [ ] Mobile Setup shows the exact phone steps and a QR generated from the companion server's one-time pairing session.
- [ ] The desktop renderer never receives the mobile UI password or a reusable device token.
- [ ] Missing companion-server setup and startup failures have actionable explanations.
- [ ] Existing Vela main/preload trust boundaries and app behavior remain unchanged.

## Relevant context

- Canonical docs: `docs/main.md`, `docs/preload.md`, `docs/operations.md`.
- Implementation: `src/main/mobile-server.ts`, `src/main/index.ts`, `src/preload/index.ts`, `src/renderer/src/components/SettingsPage.tsx`.
- Tests: `src/main/mobile-server.test.ts`, `src/renderer/src/components/SettingsPage.test.tsx`.

## Invariants / constraints

- Network and password-file access stay in main behind trusted IPC.
- Read the mobile password only from the local status file, use it only to create a short-lived session, and never log or return it.
- Pairing secret is one-time and expires; only the encoded QR payload crosses IPC.
- Preserve the unrelated in-progress OpenCode presentation changes in this worktree.

## Phases

| Phase | Scope | Status | Validation | Commit |
|---|---|---|---|---|
| 1 | Companion discovery/status and authenticated pairing IPC | pending | MobileServer tests + typecheck | — |
| 2 | Desktop guided QR setup UI and tests | pending | SettingsPage tests + build | — |
| 3 | User-facing operations guide | pending | docs check + canonical gate | — |

## Validation plan

- Targeted checks: focused Vitest suites, `npm run typecheck`, `npm run build:compile`.
- Integration/platform/manual: scan generated desktop QR with Android Vela Mobile when the device route is available.
- Final gate: `npm run check`.

## Decisions

- 2026-09-28 — Use the pairing URI returned by `vela-mobile` rather than duplicate its versioned payload encoding in the Electron renderer.
- 2026-09-28 — In development, find a sibling `vela-mobile` checkout before asking users to configure a path manually.

## Open risks / blockers

- None yet.

## Completion

- [ ] All acceptance criteria satisfied
- [ ] Final integration validation passed
- [ ] Durable truths updated in canonical docs
- [ ] Follow-up work moved to issues/backlog
- [ ] Final Git state reviewed
