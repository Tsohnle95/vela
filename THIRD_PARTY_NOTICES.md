# Third-party notices

Vela Native is an independent implementation. It does not copy source code from
the original Vela Electron application (which has no root license file in the
audited checkout); it reuses product design tokens as facts and implements
behavior independently.

## GPUI

- `gpui`, `gpui_platform`, and related crates from
  [zeronsh/zui](https://github.com/zeronsh/zui), pinned to
  `361ff20b919fdfb80199ee6d9aed725300cb7d54`.
- The fork is extracted from Zed's GPUI
  (`zed-industries/zed@f14fea9` plus the comet patch line) and is licensed under
  Apache-2.0. The in-tree `path` crate is GPL-3.0-or-later; Vela Native does not
  depend on it.
- Zed GPUI: Copyright 2022-2025 Zed Industries, Inc., Apache-2.0.

## Codicons

- `crates/vela-ui/assets/codicon.ttf` is the VS Code Codicon icon font from
  [microsoft/vscode-codicons](https://github.com/microsoft/vscode-codicons),
  licensed under Creative Commons Attribution 4.0 International. The full
  license is bundled at `crates/vela-ui/assets/codicon-LICENSE.txt`.
- The checked-in font has one local modification: an `m` cmap entry is mapped to
  an existing glyph because GPUI's macOS font loader rejects icon fonts without
  an `m` character. Icon glyphs are unchanged.

## Reference projects

The following were studied for architecture and performance techniques. No
source was copied.

- [Zeron](https://github.com/zeronsh/zeron) — MIT, Copyright 2026 Wing. Ideas
  adapted: block-granular virtualized transcript, stable row identities,
  scroll-follow behavior, per-session cached views, GPU memory bounding.
- [MonoCode](https://github.com/hardbeat920/monocode) — MIT, Copyright 2026
  Nick. Ideas adapted: bounded warm-session retention with an estimated byte
  budget, idle detach after the switch has painted, repeatable navigation
  performance tests.
- Original [Vela](https://github.com/Tsohnle95/vela) — product behavior and
  design tokens only. No license file was present at the audited revision.

## Rust dependencies

Dependency licenses are recorded by `cargo metadata` / `cargo deny` when added.
Every dependency is pinned by `Cargo.lock`. Direct dependencies use permissive
licenses (MIT/Apache-2.0/BSD); the GPUI fork's Apache-2.0 and the unused GPL
`path` crate are called out above.
