# Vela animation lab

A self-contained comparison set of twenty full-screen WebGL2 background
animations. Each animation opens two ways:

- **Vela page** — the complete Vela product page with the animation fixed
  behind it: `vela.html?effect=<slug>`
- **Pure animation** — the full-screen field with a minimal overlay:
  `effects.html?effect=<slug>`

Start at [the gallery](index.html), which renders live thumbnails from the
same shaders and marks the five saved picks.

## Collection

Saved picks: Silk, Prism, Vortex, Nebula, Aurora.

New directions: Rift, Tide, Halo, Ember, Mirage, Lumen, Circuit, Dunes, Bloom,
Static, Tunnel, Lattice, Comet, Mist, Radiant.

## Notes

- `effect-engine.js` owns the shared GLSL helpers, all twenty shaders, the
  renderer, and the thumbnail renderer.
- The Vela page dependencies (`fragments.js`, `vela-page.js`,
  `vela-base.css`, `vela-product.css`, `vela-effect.css`) are local copies so
  the lab does not depend on the Cloudroom collection.
- Use Left/Right to change animations. On the Vela page, `H` hides the lab
  controls and `?clean=1` starts with them hidden.
- Assets resolve through `../assets`. Serve `design/vela-product` over HTTP or
  open the pages from disk; WebGL2 is required.
