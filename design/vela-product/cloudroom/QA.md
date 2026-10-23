# Local verification

Validated October 23, 2026 using Node 22.23.2 and an isolated Electron profile.

- All 20 new designs rendered at 1440, 1024, 768, 390, and 320 CSS pixels:
  100 layouts, zero horizontal-overflow failures, no offscreen primary
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
