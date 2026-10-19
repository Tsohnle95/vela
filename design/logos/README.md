# Vela — App Logo Study (Iteration 4)

300 **living** marks for **Vela** in the calm **paper** theme. The brief
for this iteration: stop repeating flat geometric SVG, add life. The
shortlisted DNA (hex cells, honeycomb pyramids, crossed planes, vitals
pulses, omega tiles) is kept — but rendered with organic blobs, tapered
ribbons, isometric tone-shaded prisms, brush-weight letterforms, mitosis
stories, vines, creatures, and waves.

This is a study, not a brand decision. Every mark is a separate idea so the
set can be browsed, compared, and shortlisted.

**Refined finalists live in [`design/logo-final/`](../logo-final/index.html)** —
the shortlist distilled into eight finished candidates shown at shipping
contexts (size ladder, app-icon chips, dark/light, wordmark lockups).

## Viewing the gallery

Open `index.html` directly in any browser — no server needed. The page is
fully self-contained: `index.html` → `logos.css` → `logos.js` (a classic
script holding both the 300-entry dataset and the gallery runtime).

### Gallery controls

- **Search** — matches name, family, concept, spec, or id.
- **Family filter** — 15 families, 20 marks each.
- **Dark preview** — toggles the `paper` (light) / `dark` theme.
- **Shortlist** — click any card to add it (max 24); the tray at the bottom
  holds your picks and can be cleared.

## The families

| # | Family | Territory |
|---|---|---|
| 1 | **Atomic Velas** | Crossed Planes evolved — electrons, tails, precession. |
| 2 | **Hive Prisms** | Your hexes and pyramid patches in 3D tone-shaded form. |
| 3 | **Ribbon Flow** | Tapered bezier ribbons weaving, looping, pouring. |
| 4 | **Living Cells** | Organic blobs — mitosis, buds, vacuoles, colonies. |
| 5 | **Pulse Rivers** | Round Vitals grown into rivers of signal. |
| 6 | **Vela Systems** | Moons, shepherds, slingshots, terminators. |
| 7 | **Brush Omega** | The omega hand-brushed — blobs, drips, whisk tails. |
| 8 | **Agent Creatures** | Charming blob characters with eyes and moods. |
| 9 | **Vine Networks** | Nodes linked by stems, tendrils, and leaves. |
| 10 | **Living Stones** | Wobbling organic cairns and balanced pairs. |
| 11 | **Mitosis Stories** | Division as narrative — pinch, snap, drift, reunion. |
| 12 | **Wave Layers** | Crests, swells, aurora ribbons, dunes, rain. |
| 13 | **Flowing Constellations** | Nodes joined by curves instead of straight lines. |
| 14 | **Wild Hexes** | Hand-drawn wobbly hexes with moss, sprouts, honey. |
| 15 | **Living Tiles** | Icon tiles with organic cutouts — leaf, wave, bloom. |

## The data

`logos.js` exports nothing; it defines two arrays used by the appended
runtime:

- `LOGOS` — 300 entries, each:
  `{ id, name, family, familyId, concept, spec, svg }`.
- `FAMILIES` — `{ id, name, count }`.

Each `svg` is a parametric `viewBox="0 0 64 64"` drawing themed through class
hooks so it resolves against the active theme:

| Class | Meaning |
|---|---|
| `.l-ink` | Primary line / stroke |
| `.l-sage` | Accent (sage) line / stroke |
| `.l-dim` | Quiet supporting stroke |
| `.l-faint` | Faintest stroke / ghost |
| `.l-paper` | Paper-colored stroke (cut-out lines) |
| `.l-fill-ink` | Solid primary fill |
| `.l-fill-sage` | Solid sage fill |
| `.l-fill-dim` | Quiet fill |
| `.l-fill-paper` | Paper-colored fill (breaks a shape) |

## Regenerating

`generate-logos.mjs` is the single source. It builds all 300 marks
deterministically (seeded RNG, so output is stable) and writes `logos.js`,
appending the runtime so the published file is self-contained.

```sh
node design/logos/generate-logos.mjs
```

Editing the study means editing `generate-logos.mjs` (the `FAMILIES` array —
each family has 20 structurally distinct `variants`), never `logos.js` by hand.
The generator enforces invariants before writing: exactly 300 entries, all
ids unique, all names unique, all SVGs unique, every family at exactly 20.
