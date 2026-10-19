# Delivery verification

## Scope

154 numbered experiments: original 01–30, environments 31–64, graphic objects
65–97, systems 98–130, twelve pixel Vela variations 131–142, and twelve slick vector studies 143–154. The checks below describe the gallery. Folded light (148) was subsequently
adopted as the production splash with only Vela and the small green caption
“integrated development environment”; production launch timing is covered by
`src/renderer/src/launch.test.ts` and the cold-start layout test.

## Executed checks

- `node build.mjs` under Node 22.23.2: produces all 154 previews and SVGs;
  refuses unapproved changes to the corrected first-thirty artwork baseline or incomplete/overlapping reservations.
- `python3 verify-files.py`: all 154 SVGs parse, have distinct hashes and valid
  local definitions; every link resolves; navigation wraps correctly; all 100
  additions have source and differentiation metadata. The first 30 hashes
  match the baseline refreshed for the Vela branding correction. All SVG
  wordmarks pass the old-name scan; twenty-four compact sizes and source references
  also pass.
- `verify.html` in native Chrome: **23/23 passed**. All 154 images decoded.
  Observed transition: **3001 ms after artwork decode and paint**. Checks cover
  viewport fit, an actual **380 × 320 px** compact window, Vela branding,
  empty workspace destination, hold, replay, notes,
  immersive mode, original-to-new navigation, wraparound, all four batch
  filters and search.
- Initial 30 also passed their earlier 10-check browser suite in Firefox.
- `npm run check` under repository-supported Node 22.23.2: typecheck, unit and
  component tests (**173 files / 1,433 tests passed**, 2 files / 4 tests skipped),
  documentation checks, architecture checks, production build.

## Visual review

The twelve slick studies were reviewed together in native Chrome. Their
materials and compositions range from a centered chrome M to a cropped blue
lens, large editorial type, a fine aperture, silk curves and a ceramic capsule.
They use continuous vector geometry, smooth shading, clean typography and
subtle motion with a reduced-motion override. The first slick study was also
reviewed at its native 380 × 320 px size.

The twelve compact variations were inspected in native Chrome at gallery scale,
with concept 131 also inspected at its actual 360 × 360 px splash size. The
window sits centrally against the desktop background instead of filling the
page. All twelve keep the supplied reference’s pixel stars, celestial trails,
violet light and textured VELA lettering, with varied compositions and
300–420 px native widths. The earlier concepts were corrected to Vela too;
old route slugs are retained so existing review links keep working.

The earlier gallery review:

All original 30 and all 100 additions were inspected in the actual browser
gallery/contact sheet. Concepts 126 and 127 were revised and inspected at full
size: the former now has continuous organic channels, and the latter's full
Vela wordmark stays legible. Full-size inspection also caught image-grid
overflow; artwork now fits inside the available viewport without cropping.
Versioned stylesheet/script URLs ensure rebuilds use the current preview code.

These close comparisons were specifically reviewed:

| Pair | Visible distinction |
|---|---|
| 34 / 57 | Broad canyon and fine suspended bridge versus narrow downward rain alley and wall typography |
| 42 / 64 | Narrow waterfall seam and split title versus broad diagonal terraces and edge-rotated title |
| 66 / 74 | Huge low warm-paper emboss versus tiny cold-metal engraving and reflection band |
| 67 / 81 | One large folded letter ribbon versus multiple fine ticker strips |
| 78 / 95 | One flat tangram bird versus ascending stages of physical paper folding |
| 80 / 96 | Compact maze letterforms versus giant open crossword L |
| 107 / 17 | Broad converging particle weather and low title versus thin rising comet and high title |
| 111 / 80 | Continuous square path and escaping line versus maze-shaped typography |
| 112 / 126 | Separate translucent membrane colonies versus opaque full-frame organic maze |
| 124 / 14 | Projected cuboid wire structure versus a wide constellation of stars |

Design uniqueness was evaluated through composition and visual judgment;
distinct file hashes alone were not treated as sufficient evidence.

## Reproduce

Run `npm start` in this folder, then open:

- <http://127.0.0.1:8147/> — slick Vela variations (default).
- <http://127.0.0.1:8147/concepts/143-liquid-signature.html?still=1> — actual-size smooth splash.
- <http://127.0.0.1:8147/index.html?group=compact> — earlier pixel variations.
- <http://127.0.0.1:8147/index.html?group=all> — full gallery.
- <http://127.0.0.1:8147/contact-sheet.html?group=new> — compare the new 100.
- <http://127.0.0.1:8147/verify.html> — rerun the browser checks.

The desktop destination and folder selection are prototypes. They do not start
an Electron/OpenCode session or alter the installed app's startup behavior.

## Folded light production selection

Concept 148 now shows only Vela and the small green caption
“integrated development environment.” The matching production SVG is
`src/renderer/src/assets/folded-light.svg` at the repository root.

The installed `/Applications/Vela.app` was launched against the compiled
checkout. Native inspection showed the 380 × 320 splash with the requested
caption, followed by an expanded empty IDE with **Open a workspace** in the
sidebar and no automatic session selection. Launch unit tests cover image
decode, visible paint, the three-second hold, slow IDE loading and background
visibility; the layout test covers the empty cold-start destination.

The production change passed `npm run check` under Node 22.23.2: 174 test
files / 1,436 tests, documentation and architecture checks, and the production
build (2 files / 4 tests skipped).

## App-icon comparison

The icon frame is the current launch presentation. It uses the exact
`resources/icon.png` master without modification. `launch-comparison.html`
shows it beside the preserved Folded light artwork at 380 × 320. Both were
visually inspected in the native Chrome comparison page. The old production
`folded-light.svg` remains byte-for-byte unchanged.

The installed Vela app was also checked: its initial frame exposed the cube
icon, Vela and caption, then handed off to the empty IDE without selecting a
workspace. The existing three-second coordinator is unchanged. The canonical
check passed under Node 22.23.2 (174 files / 1,436 tests passed, 2 files / 4
tests skipped), together with docs, architecture and production build checks.
