# Selected directions and palette exploration

Open [the collection](index.html) directly from disk. It contains **30 new
iterations** and retains **#1 and #2 as references**. The earlier 20-design
collection remains available from the directory.

- **Reference #1** and **Reference #2** preserve the selected compositions,
  with the requested headline sizing and screenshot cleanup applied.
- **1.01–1.15** are new split-layout iterations of #1 (URL IDs 21–35).
- **2.01–2.15** are new centered, wide-product iterations of #2 (IDs 36–50).
- Use Left/Right outside interactive controls to browse. The iteration
  selector groups the references and both families. **All designs** opens
  a thumbnail directory; Escape closes it.
- The **Palette** dropdown offers 24 standard, vibrant, pastel, and saturated
  choices. A selection is remembered separately for each design in browser
  storage. The URL also records the selected design and palette, so it can
  be shared or restored. Browsers that disable storage retain choices for
  the current page session and through the explicit URL.
- The × collapses preview controls. **Designs & colors** restores them.

The actual Cloudroom headline scale was checked in its public stylesheet:
[Cloudroom CSS](https://www.cloudroom.dev/_next/static/immutable/chunks/1i1n0tmy8ynd5.css).
The shared refinements use an 80px desktop cap, weight 550, line-height 1.04,
and -4px tracking, with the source’s 1080px, 800px, and 640px responsive
breakpoints. Individual iterations do not override the primary type scale.

Screenshots are the unmodified real Vela image. There are no colored frames,
padding surfaces, outlines, or decorative shapes behind them. The only
presentation effects are neutral elevation, spacing, and occasional slight
perspective. Palettes change page colors and actions; they do not recolor
the screenshot or the existing app icon. The previous prohibition on
decorative labels, annotations, and design commentary still applies.

`palettes.js` owns the color tokens. `model.js` owns shared factual fragments
and the two references. `layouts-a/b/c` own the 30 compositions. `app.js` owns
selection, independent palette choices, keyboard navigation, and history.
The parent `refinements.css` applies the same corrections to the original
#1/#2 URLs and this collection.

Run `node design/vela-product/cloudroom/iterations/verify.mjs` under Node
22.23.2 from the repository root. Add `--refresh-previews` to explicitly
regenerate the 32 directory thumbnails after reviewing changed layouts.
The check otherwise writes only into a fresh temporary directory and uses
an isolated Electron profile. See [QA.md](QA.md).
