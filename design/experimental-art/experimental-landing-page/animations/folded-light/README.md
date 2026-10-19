# Folded Light / 50 motion studies

Preview: <http://127.0.0.1:8147/animations/folded-light/>

The source is `../../concepts/148-folded-light.svg`: its mint palette, four
planes, 380 × 320 frame, Vela wordmark, caption, and soft ground shadow. The
mint gradient is anchored in the source coordinate system so separating the
planes does not restart its colors on each wing. The source SVG and production application are not changed. The inherited floating
CSS animation is excluded; the atlas uses explicit, scrub-friendly SVG draws.

The five `studies-*.js` modules contain fifty named mechanisms with individual
setup/render functions. `engine.js` owns the source geometry, reusable SVG
primitives, unique definition IDs, and the settled final frame. Studies are
organized into folds, formation, light, material, and precision experiments.
Each includes a description and its specific visual rule. They are not random
seeds, palette swaps, or speed variations of one motion.

Use `?concept=01` through `?concept=50` for direct inspector links. The inspector
has pause, replay, quarter-speed playback, scrubbing, favorites, and a
three-second launch preview into an empty workspace. All studies hold the exact
source mark for the final 300 ms. Assembly studies retain a faint mark or
construction artwork from their first frame. Reduced-motion settings start the
gallery paused; playback is explicit. Hidden tabs and offscreen cards pause.

`contact-sheet.html` compares frames across all fifty studies. Use `?group=01`,
`11`, `21`, `31`, or `41` for ten at a time, and `&phase=35` for a fixed percent.
`verify.html` runs geometry, completion, determinism, route, playback, favorite,
launch-timing, cleanup, and mobile-width checks in an actual browser.
