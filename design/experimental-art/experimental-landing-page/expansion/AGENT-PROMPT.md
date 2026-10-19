# Shared designer prompt — 100 additional Vela launches

You are one of three designers creating a reserved portion of an additional 100
launch designs. The user explicitly requires every concept to be completely
different, including from the 30 already built. Treat similarity as a defect.

Read the repository's AGENTS.md and docs/agent-execution.md. Read the original
concepts.json and build.mjs to know the existing 30. Read ALL reservations in
registry.json, not only your assignments. Inspect the reference contact sheet
and at least three relevant source images with view_image. These are coding
prototypes: use original SVG compositions, optionally incorporating carefully
cropped supplied artwork with no captured app controls or old Vela text.

## Non-overlap contract

Your assigned ID range and reserved dominant subject belong only to you. Never
take another row's subject, including another designer's. A generic celestial
logo, centered star above a wordmark, planet with a thin ring, wireframe tunnel,
waveform, postage stamp, vinyl record, monolith, or other original-30 composition
is already taken and cannot be reused as the primary design.

For each concept, specify these before drawing: dominant silhouette, page
composition, camera/framing, type hierarchy, material/mark-making, and narrative.
It must differ from its nearest neighbor in at least FOUR of those SIX dimensions.
Color changes, alternate seeds, moved labels, switched fonts, new crops, a new
background, or a different object on the same page template do not qualify.

Use shared drawing primitives only for low-level geometry, never a shared
parameterized scene/template. Author each scene independently. Within your set,
use at least TEN different composition structures; do not use one structure for
more than three concepts. Vary whitespace, title position/scale, full-bleed versus
framed illustration, edge cropping, vertical versus diagonal movement, asymmetry,
typographic weight and density. At thumbnail size, each design should be
recognizable without reading its number/name. Avoid repeating a centered icon
and a title underneath. Make intentional, polished desktop splash art, not a
collection of diagrams with the same small labels in four corners.

The user's screenshots supply pixel texture, violet light, cosmic depth,
architecture, astronauts, mathematical paths, glass and space. Borrow their
visual language; do not blindly put every source screenshot in the same frame.
Some light, warm, monochrome or print treatments are welcome where coherent.
Use the requested **Vela** name. No invented version number, claims, fake
runtime checks or fake percentage indicators. No session/file picker on the art.

## Delivery contract

Work only in your assigned `expansion/<domain>.mjs` plus any explicitly assigned
asset subfolder. Root owns gallery, build.mjs, preview.js/CSS and generated pages.
Do not modify app source or other designers' files. Do not stage or commit.

Export `default` as an array of concept objects:

```js
{
  id: '31', // exact reserved ID as a string, no leading zero
  name: 'Reserved name',
  family: 'Short art-direction label',
  description: 'Concrete sentence describing what is visible.',
  bg: '#101020',
  tone: 'dark', // or 'light'
  source: ['exact filename.png'],
  composition: 'Where the main masses and text sit',
  silhouette: 'Distinctive outline / visual gesture',
  typography: 'Type scale and arrangement',
  material: 'Drawing / texture technique',
  nearest: 'ID/name of visually nearest concept and concrete differences',
  art: '<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="900" viewBox="0 0 1440 900">…</svg>'
}
```

Keep source filenames factual and inspect them. Use system fonts. Avoid external
network resources. Namespace SVG IDs or ensure no conflicting IDs within one
artwork. XML must parse (no duplicated attributes); escape text. Avoid filters
that disappear in browsers. Aim for an SVG under 500 KB, unless detail earns it.
Prefer code-native SVG to generating 100 raster images. No raw screenshot
thumbnails masquerading as new art. Do not call image-generation services.

Validate your module import, object/ID counts, SVG XML, and locally inspect the
result as feasible. Root will run browser checks and visual cross-set review.
Do not claim visual acceptance based only on import/XML. Report any uncertain
similarities or weak concepts explicitly so they can be revised. Before delivery,
compare EVERY concept against the global reservations and original 30; redesign
anything reading as the same silhouette or the same layout with swapped parts.

Send progress after the first substantive slice and when finished. Deliver all
assigned concepts, not a subset. In your completion, list count, owned files,
validation, source references, and the strongest nearest-neighbor distinctions.
