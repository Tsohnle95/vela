# Orbit — 130 ways to arrive

The original 30 launch-screen prototypes, plus 100 independently composed
designs created by three agents under a shared non-overlap contract.

The original reference is `../ChatGPT Image Oct 13, 2026, 03_23_53 PM.png`.
The expansion also draws on the other screenshots in `experimental-art`:
violet light, pixel texture, habitats, cosmic paths, glass and astronomical
space. The requested Orbit name is used throughout, although those references
say Mira. Each addition records the inspected source filenames in its notes.

## Preview

From this folder:

```sh
npm start
```

Open <http://127.0.0.1:8147/>. The gallery contains all 130 numbered designs.
Filter to **Original 30** or **New 100**, search a number/name/art direction, or
use **All treatments** to narrow by light/dark artwork.
Click a card to run its launch experience. Each page paints its local artwork,
holds it for three seconds after image decode and first paint, then shows an
empty desktop workspace. There is no session/file picker on the launch screen.
The workspace waits for **Choose workspace**; selecting a folder only shows its
name in this local prototype. No files are uploaded and no agent is started.

Use **Hold artwork** to inspect a design, **Replay 3s launch** to restart it,
and the arrow buttons to move between concepts. **Design notes** holds the
artwork and explains its composition, material, source and nearest comparison.
Keyboard shortcuts: Left/Right,
Space to hold/resume, R to replay. `?still=1` holds the art on initial load;
`?immersive=1` hides the review controls. They can be combined.

## Files

- `index.html`, `gallery.css`, `gallery.js`: responsive gallery and filters.
- `concepts/`: 130 separate HTML previews and 130 original SVG artworks.
- `preview.css`, `preview.js`: shared three-second lifecycle and empty shell.
- `concepts.json`: titles, descriptions, and links.
- `build.mjs`: reproducibly generates the gallery, pages, and SVG compositions.
  Run `npm run build` after changing the artwork definitions.
- `expansion/environments.mjs`: agent-created concepts 31–64.
- `expansion/graphic-objects.mjs`: agent-created concepts 65–97.
- `expansion/systems.mjs`: agent-created concepts 98–130.
- `expansion/AGENT-PROMPT.md`: the actual shared prompt; prohibits template,
  recolor, seed, font and crop swaps from counting as new concepts.
- `expansion/registry.json`: 100 reserved subjects/compositions, shared by all
  designers before implementation.
- `expansion/original-art-baseline.json`: original 30 artwork hashes. Build
  refuses to alter their artwork while integrating additions.
- `process.html`: readable prompt and all 100 briefs.
- `contact-sheet.html`: compact art-only comparison view; click to hold a design.
- `QA.md`: completed checks and the closest visual comparisons reviewed.

Run `npm run verify:files` to check all SVGs, links, source filenames, original
hashes, navigation, counts and required design metadata.
Open `verify.html` on the preview server to run the browser checks for artwork
decode, three-second timing, the empty destination, holding, replay, navigation,
and immersive mode, plus the expanded gallery filters and search.

Everything is self-contained, with system fonts and no external dependencies,
fonts, analytics, image-generation service, or CDN. `inspiration.png` is a copy
of the supplied reference, so the gallery's reference link works locally.

These are isolated design experiments. The installed app and its production
startup behavior are not changed. The desktop shell is a browser prototype of
the requested destination; it does not connect to Electron or OpenCode.
