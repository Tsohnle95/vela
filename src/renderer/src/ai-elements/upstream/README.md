# shadcn AI Elements source

The `patterns/` files are the original 21 pattern pages from
<https://ui.shadcn.com/docs/ai-elements>, including their complete runnable demo
source. `registry/` contains the original `base-rhea` registry payloads for the
primitives those demos compose. `manifest.json` records source URLs and SHA-256
hashes. The upstream MIT license is retained in `LICENSE.md`.

Refresh these files and the installed primitives from the repository root with
`node scripts/vendor-ai-elements.mjs`.

The generated files in `components/ui/` retain upstream JSX and utility classes.
Installation resolves registry imports to local paths, resolves `cn` to the
project utility, and selects the Lucide icon variant from registry placeholders.
Portal positioners receive an isolation class so their theme follows the new
surface. Theme tokens, radius scales and Geist font selection come from the
official site snapshots in `styles/`; `ai-elements.css` scopes their application
and Tailwind's preflight to the new surface.
The syntax-highlighting hook and helper retain their original source here.

The application adapters live outside this directory. They connect Vela's
runtime events to these components. The reference demos retain their scripted
AI SDK conversations separately from Vela's runtime integration.
