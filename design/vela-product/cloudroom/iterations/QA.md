# Verification

Checked October 25, 2026 under Node 22.23.2 with isolated Electron profiles.

- 30 new iterations: 15 from #1 and 15 from #2, plus both retained references.
- All 32 pages rendered at 1440, 1024, 768, 390, and 320 CSS pixels: 160
  layouts checked for horizontal overflow, offscreen headlines/actions/images,
  image decoding, and the absence of frames/added artwork in the 30 new
  iterations. Reference #1 deliberately keeps its original layout and type
  with the current screenshots.
- Headline size, weight, and line-height checked against Cloudroom’s measured
  responsive scale at each width for the 30 new iterations and reference #2.
  Reference #1 keeps its original 110px desktop cap.
- All 32 designs exercised with all 24 palettes: 768 page/palette combinations.
  Page foreground/background and primary action colors are checked against
  the selected tokens. Every palette meets 4.5:1 for body/description/button
  text and 3:1 for the large accent heading.
- Checked independent per-design palette choices, full reload, URL restoration,
  browser Back, arrow navigation, native palette-key ownership, wraparound,
  directory selection, and collapsing/restoring review controls.
- Reproduced the reported shortcut failure with a real Down keypress after
  focusing a navigation button. The previous handler ignored button/select
  targets. Real Up/Down keypresses now check buttons, the design selector, and
  the palette selector, including unchanged design selection. Text editing,
  open dialogs, and Left/Right native control ownership remain protected.
- Reviewed rendered desktop/mobile contact sheets and palette sheets for both
  families. All directory thumbnails come from actual renders.
- `verify-reference.mjs` extracts #1 from original commit `7c0a724b` and renders
  it alongside the current reference in fresh windows at 1904, 1440, 1024, 768,
  390, and 320 pixels. All six comparisons report identical headline, hero, and
  art geometry, the current light and dark screenshots, and no secondary-image
  frame. Fresh windows avoid transient resize-time image resampling.
- The #1 URL keeps its original layout styles with the current imagery; #2
  retains its corrected 80px desktop cap and unframed images. Palette controls
  remain separate from the product rendering during the reference comparison.

Normal history/reload checks run before the deliberately dense palette matrix.
This keeps normal navigation verification separate from Chromium’s history
update rate limiting during hundreds of automated changes.

The repository’s `npm run check` passes under the supported toolchain.
External repository/release/setup links retain their existing destinations;
these checks do not install Vela or start OpenCode. Cross-browser validation
is limited to the available Chrome/Electron surfaces.
