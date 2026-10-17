# Orbit — 30 ways to arrive

Thirty distinct, local launch-screen prototypes inspired by
`../ChatGPT Image Oct 13, 2026, 03_23_53 PM.png`.

The reference supplies the violet palette, bright star, orbital geometry, pixel
texture, and sense of space. The concepts explore different compositions,
typography, scales, objects, landscapes, and degrees of detail. The requested
Orbit name is used throughout, although the reference artwork says Mira.

## Preview

From this folder:

```sh
npm start
```

Open <http://127.0.0.1:8147/>. The gallery contains all 30 numbered designs.
Click a card to run its launch experience. Each page paints its local artwork,
holds it for three seconds after image decode and first paint, then shows an
empty desktop workspace. There is no session/file picker on the launch screen.
The workspace waits for **Choose workspace**; selecting a folder only shows its
name in this local prototype. No files are uploaded and no agent is started.

Use **Hold artwork** to inspect a design, **Replay 3s launch** to restart it,
and the arrow buttons to move between concepts. Keyboard shortcuts: Left/Right,
Space to hold/resume, R to replay. `?still=1` holds the art on initial load;
`?immersive=1` hides the review controls. They can be combined.

## Files

- `index.html`, `gallery.css`, `gallery.js`: responsive gallery and filters.
- `concepts/`: 30 separate HTML previews and 30 original SVG artworks.
- `preview.css`, `preview.js`: shared three-second lifecycle and empty shell.
- `concepts.json`: titles, descriptions, and links.
- `build.mjs`: reproducibly generates the gallery, pages, and SVG compositions.
  Run `npm run build` after changing the artwork definitions.

Open `verify.html` on the preview server to run the browser checks for artwork
decode, three-second timing, the empty destination, holding, replay, navigation,
and immersive mode.

Everything is self-contained, with system fonts and no external dependencies,
fonts, analytics, image-generation service, or CDN. `inspiration.png` is a copy
of the supplied reference, so the gallery's reference link works locally.

These are isolated design experiments. The installed app and its production
startup behavior are not changed. The desktop shell is a browser prototype of
the requested destination; it does not connect to Electron or OpenCode.
