# Verification

Checked October 23, 2026 under Node 22.23.2 with isolated Electron profiles.

- 30 new iterations: 15 from #1 and 15 from #2, plus both retained references.
- All 32 pages rendered at 1440, 1024, 768, 390, and 320 CSS pixels: 160
  layouts checked for horizontal overflow, offscreen headlines/actions/images,
  screenshot frames, added artwork, and image decoding.
- Headline size, weight, and line-height checked against Cloudroom’s measured
  responsive scale at each width. The desktop cap is 80px, replacing the
  previous 110px title in #1.
- All 32 designs exercised with all 24 palettes: 768 page/palette combinations.
  Page foreground/background and primary action colors are checked against
  the selected tokens. Every palette meets 4.5:1 for body/description/button
  text and 3:1 for the large accent heading.
- Checked independent per-design palette choices, full reload, URL restoration,
  browser Back, arrow navigation, native palette-key ownership, wraparound,
  directory selection, and collapsing/restoring review controls.
- Reviewed rendered desktop/mobile contact sheets and palette sheets for both
  families. All directory thumbnails come from actual renders.
- Original #1/#2 URLs also checked for 80px desktop titles, no decorative
  logo/orbits behind the hero screenshot, and no colored screenshot frames.

Normal history/reload checks run before the deliberately dense palette matrix.
This keeps normal navigation verification separate from Chromium’s history
update rate limiting during hundreds of automated changes.

The repository’s `npm run check` passes under the supported toolchain.
External repository/release/setup links retain their existing destinations;
these checks do not install Vela or start OpenCode. Cross-browser validation
is limited to the available Chrome/Electron surfaces.
