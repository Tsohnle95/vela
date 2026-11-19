# Archive instructions

This subtree is the **read-only reference implementation** for the native Vela
migration. It is not an active product checkout.

- Do not edit, format, build, package, install, or run lifecycle commands against
  this tree as if it were the current app.
- Repository policy, setup, and verification are owned by the root
  [`AGENTS.md`](../AGENTS.md). The nested
  [`vela-electron/AGENTS.md`](vela-electron/AGENTS.md) is a historical artifact
  describing the old Electron app; it is not operative for current work.
- Use this tree to verify behavior, visual structure, copy, and source
  semantics before implementing the native equivalent.
- Do not copy Electron/React/Tauri back into the native app or embed an editor
  WebView.
- New captures, evidence, and migration notes belong under `docs/parity/` and
  `docs/migration/`, not inside this archive.
