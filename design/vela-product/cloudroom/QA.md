# Local verification

Validated October 27, 2026 using Node 22.23.2 and an isolated Electron profile.

- Design 01 is the standalone Vela IDE composition. Three OpenBot-inspired
  takes extend it: Aurora (a cool split hero), Paper (a light editorial
  treatment), and Studio (a monochrome tool-like treatment). Design 01 mounts
  the reference WebGL beam: delayed intro, drop from the header download
  control, wrap around the preview, edge spill, fog, dust, and pointer lens.
  Five beam studies reuse the same engine with different sources, palettes,
  presets, and timing: Mint Drop, Brand Mark, Signal, Deep Haze, and CTA Bloom.
  Ten Vela-page background treatments reuse the full-screen Aurora, Ink, Nebula,
  Caustics, Vortex, Prism, Smoke, Silk, Grid, and Particles shaders behind the
  product page and preview. The animation and source rim were checked in
  Electron at 1440 and 390 CSS pixels. Each design keeps one primary heading,
  avoids horizontal overflow at 1440, 1024, 768, 390 and 320 CSS pixels, loads
  all in-page product images, and preserves working platform, FAQ, download,
  source and navigation actions.
- All 38 designs rendered at 1440, 1024, 768, 390, and 320 CSS pixels:
  190 layouts, zero horizontal-overflow failures, no offscreen primary
  headings/actions/product images, one primary heading per design, all
  product images decoded, and zero renderer errors.
- Reviewed actual-render contact sheets of every desktop and mobile design.
- The original 25 designs and the previous 30 interpretations were checked
  at 1440 and 390 pixels: 110 legacy layouts checked for overflow and removal
  of the screenshot’s annotation text, eyebrow labels, screenshot labels,
  design commentary, and header metadata.
- Verified previous/next wraparound, native selector ownership, arrow-key
  navigation, gallery selection, Escape dismissal, and collapsing/restoring
  review controls. Version 14’s native radio selector changes the visible
  feature explanation while keeping the real app screenshot intact.
- The previous 30-design gallery thumbnails were regenerated after metadata
  removal. New gallery thumbnails come from the actual 1440px renderings.
- `npm run check` passed under the supported toolchain: typecheck, 199 test
  files and 1,609 tests, docs and architecture checks, and production build.
  Two test files and four tests were skipped by the existing test setup.

External product actions retain the existing Vela GitHub repository,
releases, and setup destinations. Preview checks do not download or install
Vela, start OpenCode, or establish release availability. The renderer checks
exercise Chromium/Electron; exhaustive cross-browser testing is outside this
local check.
