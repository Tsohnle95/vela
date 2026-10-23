# Vela release directions

Open [the collection](index.html) directly from disk. Twenty standalone
compositions share Vela’s real screenshot, current folded mint mark, and
factual product content. No server, build, remote fonts, or external assets
are required.

The visual reference is [Cloudroom](https://www.cloudroom.dev/), reviewed live
on October 23, 2026: black canvas, very large sans-serif type, crisp primary
actions, a sparse navigation bar, bright accent color, geometric product
artwork, and substantial breathing room. The layouts adapt that approach to
Vela’s own identity. Cloudroom’s illustration, logo, source, and copy are not
included. Its cloud hosting, sandbox, pricing, model-support, and usage claims
are not claims about Vela.

Use the bottom selector or Left/Right keys outside interactive controls.
**All designs** opens the thumbnail directory. Escape closes it. The × hides
the review controls; **Designs** restores them. Add `?design=1&clean=1` to
start with collapsed controls. Version 14 has a keyboard-accessible native
radio selector for its editor, agent, and changes explanations. Browser
Back/Forward restores the selected design.

The product pages are presented as real releases. Do not add decorative
eyebrows, specimen IDs, marginal notes, screenshot labels, mock telemetry,
fake metrics, design rationale, or commentary. Keep meaningful headings,
useful product descriptions, real imagery, and functional actions. Design
names and navigation belong in the separate collection controls only.

The collection includes Home for the work; Everything in view; Close to the
code; A place to build; One working space; The big picture; Folded space;
Code, together; The shared workspace; In plain view; In orbit; The wordmark;
Good company; The working view; A new angle; After hours; Still point;
First look; Green room; and On your desktop.

Run `node design/vela-product/cloudroom/verify.mjs` from the repository root
under Node 22.23.2. The isolated Electron check renders all 20 layouts at five
widths, checks interactions, and verifies metadata removal across the prior
55 designs at desktop and phone widths. Output goes into a fresh temporary
directory. It never opens the installed app profile or contacts OpenCode.
See [QA.md](QA.md) for the verified scope.
