# Preview verification

Verified locally on October 23, 2026 under repository-supported Node 22.23.2.

- Reviewed original Design 24 in Chrome and its HTML/CSS before composing the collection.
- Rendered all 30 versions with isolated Electron profiles at 1440, 960, 700,
  390, and 320 CSS pixels: 150 layouts, zero horizontal-overflow failures,
  zero offscreen headline/action/product failures, all product images decoded,
  one primary heading per version, and zero renderer errors.
- Reviewed raster contact sheets of all 30 desktop and all 30 mobile layouts.
- Verified selector, previous/next wraparound, modal directory selection,
  Escape dismissal, arrow-key navigation, native select key ownership,
  task switching in version 14, and disclosure interaction in version 22.
- Verified the original Design 24 header remains borderless and its new
  collection link exists. Its pre-existing stylesheet edits were preserved.
- Created all 30 local directory thumbnails from the actual 1440px renders.
- Repository gate: 199 test files passed, 1,609 tests passed, 2 test files and
  4 tests skipped; typecheck, docs, architecture, and production build passed.

Run `node design/vela-product/design-24/verify.mjs` from the repository root
under Node 22.23.2 to repeat the layout/interaction check. It uses a temporary
profile, writes screenshots, contact sheets, generated preview JPEGs, and
`results.json` into a fresh system temporary directory, and does not connect
to OpenCode or the installed app profile. Review and copy `preview-NN.jpg`
to `previews/NN.jpg` if intentionally refreshing the directory thumbnails.

External links retain the original repository/release/setup destinations;
this check does not claim to download, install, or start a release. Browser
Back/Forward is implemented with the History API; platform-specific browser
gestures have not been exhaustively exercised.
