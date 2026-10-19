// Builds design/logo-final/index.html from marks.mjs.
// Static output — no scripts — opens straight from disk.
// Each finalist is presented as a shipped logo: hero, dark preview,
// app-icon chips, a favicon size ladder, and a wordmark lockup.

import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { MARKS } from "./marks.mjs";

const here = dirname(fileURLToPath(import.meta.url));

const onColor = (svg) =>
  svg
    .replaceAll("c-ink", "c-paper")
    .replaceAll("f-ink", "f-paper")
    .replaceAll("c-sage", "c-paper p75")
    .replaceAll("f-sage", "f-paper p75")
    .replaceAll("c-dim", "c-paper p55")
    .replaceAll("f-dim", "f-paper p55");

const wrap = (svg, size) =>
  `<svg viewBox="0 0 96 96" width="${size}" height="${size}" aria-hidden="true">${svg}</svg>`;

const LADDER = [128, 64, 48, 32, 24, 16];

const sections = MARKS.map((m) => {
  const tileSage = `<div class="chip chip-sage">${wrap(onColor(m.svg), 116)}</div>`;
  const tileInk = `<div class="chip chip-ink">${wrap(onColor(m.svg), 116)}</div>`;
  const ladder = LADDER.map(
    (px) => `<figure class="step"><div class="art">${wrap(m.svg, px)}</div><figcaption>${px}</figcaption></figure>`
  ).join("");
  return `
<section class="candidate" id="mark-${m.id}">
  <header>
    <h2>${m.id} · ${m.name}</h2>
    <p class="based">refined from ${m.basedOn}</p>
  </header>
  <p class="note">${m.note}</p>
  <div class="row">
    <figure class="panel hero-panel"><figcaption>Light</figcaption>${wrap(m.svg, 176)}</figure>
    <figure class="panel dark hero-panel"><figcaption>Dark</figcaption>${wrap(m.svg, 176)}</figure>
    <div class="chips">
      ${tileSage}
      ${tileInk}
    </div>
  </div>
  <div class="block">
    <h3>Size check</h3>
    <div class="ladder">${ladder}</div>
  </div>
  <div class="block">
    <h3>Lockup</h3>
    <div class="lockup">${wrap(m.svg, 30)}<span class="word">Vela</span></div>
    <div class="lockup dark-lockup">${wrap(m.svg, 30)}<span class="word">Vela</span></div>
  </div>
</section>`;
});

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1.0"/>
<title>Vela — Logo Finalists</title>
<style>
:root {
  --ink: #2b2119;
  --sage: #617a68;
  --muted: #8a7d6c;
  --paper: #fdfaf3;
  --bg: #f4eee1;
  --panel: #fbf7ec;
  --border: rgba(43, 33, 25, 0.12);
}
* { box-sizing: border-box; }
body {
  margin: 0;
  padding: 40px clamp(20px, 6vw, 80px) 80px;
  background: var(--bg);
  color: var(--ink);
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
}
header.page h1 { font-size: 26px; letter-spacing: -0.02em; margin: 0 0 6px; }
header.page p { margin: 0 0 36px; color: var(--muted); max-width: 70ch; line-height: 1.5; }
.candidate {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 18px;
  padding: 28px 30px 26px;
  margin-bottom: 34px;
}
.candidate header { display: flex; align-items: baseline; gap: 14px; flex-wrap: wrap; }
.candidate h2 { font-size: 19px; margin: 0; letter-spacing: -0.01em; }
.based { margin: 0; color: var(--muted); font-size: 12.5px; }
.note { margin: 10px 0 22px; color: var(--muted); font-size: 13.5px; max-width: 72ch; }
.row { display: flex; gap: 18px; flex-wrap: wrap; align-items: stretch; }
.panel {
  margin: 0;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: #fffdf7;
  padding: 14px;
  display: grid;
  place-items: center;
}
.panel figcaption { font-size: 10.5px; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); margin-top: 8px; }
.panel.dark { background: #171412; }
.panel.dark figcaption { color: #8f8880; }
.hero-panel { min-width: 210px; }
.chips { display: flex; gap: 16px; margin-left: auto; }
.chip {
  width: 148px; height: 148px;
  border-radius: 33px;
  display: grid; place-items: center;
  box-shadow: inset 0 0 0 1px rgba(255,255,255,0.08);
}
.chip-sage { background: linear-gradient(160deg, #6d8573, #55705e); }
.chip-ink { background: linear-gradient(160deg, #35291f, #211a13); }
.block { margin-top: 26px; }
.block h3 {
  margin: 0 0 12px; font-size: 11px; letter-spacing: 0.1em;
  text-transform: uppercase; color: var(--muted); font-weight: 600;
}
.ladder { display: flex; align-items: flex-end; gap: 26px; flex-wrap: wrap; }
.step { margin: 0; text-align: center; }
.step .art {
  background: #fffdf7;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px;
  display: grid; place-items: center;
  min-height: 60px;
}
.step figcaption { font-size: 11px; color: var(--muted); margin-top: 6px; font-variant-numeric: tabular-nums; }
.lockup {
  display: inline-flex; align-items: center; gap: 12px;
  background: #fffdf7; border: 1px solid var(--border);
  border-radius: 12px; padding: 12px 20px 12px 14px; margin-right: 14px;
}
.word { font-weight: 650; font-size: 21px; letter-spacing: -0.02em; }
.dark-lockup { background: #171412; border-color: rgba(255,255,255,0.09); }
.dark-lockup .word { color: #e8e3dd; }

/* mark palette */
.c-ink, .c-sage, .c-dim, .c-paper { fill: none; stroke-linecap: round; stroke-linejoin: round; }
.c-ink { stroke: var(--ink); }
.c-sage { stroke: var(--sage); }
.c-dim { stroke: var(--muted); }
.f-ink { fill: var(--ink); }
.f-sage { fill: var(--sage); }
.f-dim { fill: var(--muted); }
.p75 { opacity: 0.78; }
.p55 { opacity: 0.55; }

/* dark scopes re-map the theme-aware strokes */
.dark { --ink: #e8e3dd; --sage: #9eb4a1; --muted: #8f8880; }
</style>
</head>
<body>
<header class="page">
  <h1>Vela logo finalists</h1>
  <p>Eight finished candidates refined from your iteration-4 shortlist. Every mark is shown
  at shipping contexts — light and dark, app-icon chips, a size ladder down to favicon,
  and a wordmark lockup. Shortlist one (or two) and the next step is real asset
  production: icon.icns, trimmed PNGs, and SVG source.</p>
</header>
${sections.join("\n")}
</body>
</html>
`;

writeFileSync(join(here, "index.html"), html);
console.log(`Wrote index.html with ${MARKS.length} finalists.`);
