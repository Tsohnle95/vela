# Delivery verification

## Scope

142 numbered experiments: original 01–30, environments 31–64, graphic objects
65–97, systems 98–130, and twelve compact Mira variations 131–142. Production app code is outside this change.

## Executed checks

- `node build.mjs` under Node 22.23.2: produces all 142 previews and SVGs;
  refuses unapproved changes to the corrected first-thirty artwork baseline or incomplete/overlapping reservations.
- `python3 verify-files.py`: all 142 SVGs parse, have distinct hashes and valid
  local definitions; every link resolves; navigation wraps correctly; all 100
  additions have source and differentiation metadata. The first 30 hashes
  match the baseline refreshed for the Mira branding correction. All SVG
  wordmarks pass the old-name scan; twelve compact sizes and source references
  also pass.
- `verify.html` in native Chrome: **22/22 passed**. All 142 images decoded.
  Observed transition: **3001 ms after artwork decode and paint**. Checks cover
  viewport fit, an actual **360 × 360 px** compact window, Mira branding,
  empty workspace destination, hold, replay, notes,
  immersive mode, original-to-new navigation, wraparound, all three batch
  filters and search.
- Initial 30 also passed their earlier 10-check browser suite in Firefox.
- `npm run check` under repository-supported Node 22.23.2: typecheck, unit and
  component tests (**173 files / 1,433 tests passed**, 2 files / 4 tests skipped),
  documentation checks, architecture checks, production build.

## Visual review

The twelve compact variations were inspected in native Chrome at gallery scale,
with concept 131 also inspected at its actual 360 × 360 px splash size. The
window sits centrally against the desktop background instead of filling the
page. All twelve keep the supplied reference’s pixel stars, orbital trails,
violet light and textured MIRA lettering, with varied compositions and
300–420 px native widths. The earlier concepts were corrected to Mira too;
old route slugs are retained so existing review links keep working.

The earlier gallery review:

All original 30 and all 100 additions were inspected in the actual browser
gallery/contact sheet. Concepts 126 and 127 were revised and inspected at full
size: the former now has continuous organic channels, and the latter's full
Mira wordmark stays legible. Full-size inspection also caught image-grid
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

- <http://127.0.0.1:8147/> — compact Mira variations (default).
- <http://127.0.0.1:8147/concepts/131-reference-spark.html?still=1> — actual-size held splash.
- <http://127.0.0.1:8147/index.html?group=all> — full gallery.
- <http://127.0.0.1:8147/contact-sheet.html?group=new> — compare the new 100.
- <http://127.0.0.1:8147/verify.html> — rerun the browser checks.

The desktop destination and folder selection are prototypes. They do not start
an Electron/OpenCode session or alter the installed app's startup behavior.
