# Mira — 25 motion studies

Open <http://127.0.0.1:8147/animations/> with the parent preview server running.
The gallery uses the exact existing Mira icon, cut into three cube layers with
SVG clipping, plus continuous vector trails, objects and surface lighting.
No bitmap master is edited. Production loading behavior is unchanged.

Each of the 25 entries in `motions.js` has a reserved motion rule and its own
choreography. Shared timing and drawing primitives preserve the identity of
the app icon; recolors, speed changes or different random seeds do not count
as additional concepts. The page explains the visual distinction for each.

- Click **Inspect** to see the compact 380 × 320 frame.
- Pause, replay, slow playback, or scrub any frame of the three-second cycle.
- **Preview 3-second launch** plays once and shows an empty workspace mockup.
- Save studies with the diamond button; favorites stay in local browser storage.
- Family filters compare related mechanisms. The Saved filter keeps a shortlist.
- Escape closes the inspector. Left/Right and Space work when focus is outside
  the explicit controls; native slider keys keep their normal meaning.
- Reduced-motion preferences start the gallery paused. Playback is opt-in.
- Offscreen studies pause drawing, the grid pauses while inspecting one study,
  and background tabs stop advancing visible motion.

`index.html`, `styles.css`, `gallery.js` and `motions.js` are self-contained
apart from `../mira-app-icon.png`, the unmodified shared master copy.
`verify.html` runs actual browser checks. `QA.md` records final review results.

If authorized to use an isolated headless browser, run `node verify.mjs` under
Node 22.23.2. It runs the browser suite and writes contact-sheet PNGs and a
JSON report into a fresh system temporary directory. It never opens a Mira
workspace or changes the installed app's profile.
