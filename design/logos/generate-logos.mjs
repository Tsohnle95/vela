// Procedural generator for the Vela brand-logo study, iteration 4.
// Produces design/logos/logos.js: 300 DISTINCT living marks distilled from
// the iteration-3 shortlist (tri-trace hex, honeycomb pyramids, crossed
// planes, hub-pad meshes, vitals pulses, omega tiles).
// This iteration answers one brief: ADD LIFE. Organic bezier blobs, tapered
// ribbon strokes, isometric tone-shaded prisms, brush-weight omegas,
// mitosis and growth stories, creatures, vines, and waves — motion over
// static geometry, while keeping the calm two-tone ink/sage discipline.

import { writeFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));

function mulberry32(a) {
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const TAU = Math.PI * 2;
const f = (n) => Math.round(n * 100) / 100;
const pt = (cx, cy, r, a) => [cx + r * Math.cos(a), cy + r * Math.sin(a)];

const line = (x1, y1, x2, y2, cls, w) =>
  `<line class="${cls}" x1="${f(x1)}" y1="${f(y1)}" x2="${f(x2)}" y2="${f(y2)}"${w ? ` style="stroke-width:${w}"` : ""}/>`;
const circ = (cx, cy, r, cls, w) =>
  `<circle class="${cls}" cx="${f(cx)}" cy="${f(cy)}" r="${f(r)}"${w ? ` style="stroke-width:${w}"` : ""}/>`;
const dot = (cx, cy, r, cls) => `<circle class="${cls}" cx="${f(cx)}" cy="${f(cy)}" r="${f(r)}"/>`;
const poly = (pts, cls, w) =>
  `<polygon class="${cls}" points="${pts.map((p) => p.map(f).join(",")).join(" ")}"${w ? ` style="stroke-width:${w}"` : ""}/>`;
const path = (d, cls, w) =>
  `<path class="${cls}" d="${d}"${w ? ` style="stroke-width:${w}"` : ""}/>`;
const rectR = (x, y, w, h, r, cls, w2) =>
  `<rect class="${cls}" x="${f(x)}" y="${f(y)}" width="${f(w)}" height="${f(h)}" rx="${f(r)}"${w2 ? ` style="stroke-width:${w2}"` : ""}/>`;
const arc = (cx, cy, r, a0, a1, cls, w) => {
  const [x0, y0] = pt(cx, cy, r, a0);
  const [x1, y1] = pt(cx, cy, r, a1);
  const large = (((a1 - a0) % TAU) + TAU) % TAU > Math.PI ? 1 : 0;
  return `<path class="${cls}" d="M ${f(x0)} ${f(y0)} A ${f(r)} ${f(r)} 0 ${large} 1 ${f(x1)} ${f(y1)}"${w ? ` style="stroke-width:${w}"` : ""}/>`;
};
const ell = (cx, cy, rx, ry, cls = "l-sage", sw = 1.6, rot = 0) =>
  `<ellipse class="${cls}" cx="${f(cx)}" cy="${f(cy)}" rx="${f(rx)}" ry="${f(ry)}"${rot ? ` transform="rotate(${rot} ${f(cx)} ${f(cy)})"` : ""} style="fill:none;stroke-width:${sw}"/>`;
const raw = (s) => s;

const reg = (cx, cy, r, n, rot = -Math.PI / 2) =>
  Array.from({ length: n }, (_, i) => pt(cx, cy, r, rot + (i * TAU) / n));
const starPts = (cx, cy, ro, ri, n, rot = -Math.PI / 2) =>
  Array.from({ length: n * 2 }, (_, i) =>
    pt(cx, cy, i % 2 ? ri : ro, rot + (i * Math.PI) / n)
  );

const wrap = (inner) =>
  `<svg class="logo-svg" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">${inner}</svg>`;

const tile = (cls = "l-fill-sage", inset = 6, rad = 14) =>
  rectR(inset, inset, 64 - inset * 2, 64 - inset * 2, rad, cls);

const hexStroke = (cx, cy, r, cls = "l-ink", w = 2.8) => poly(reg(cx, cy, r, 6), cls, w);
const hexFill = (cx, cy, r, cls = "l-fill-sage") => poly(reg(cx, cy, r, 6), cls);

const omegaArc = (cx, cy, r, gapDeg = 50, cls = "l-ink", w = 2.4, foot = 5) => {
  const g = (gapDeg * Math.PI) / 180;
  const [x0, y0] = pt(cx, cy, r, Math.PI / 2 + g);
  const [x1, y1] = pt(cx, cy, r, Math.PI / 2 - g);
  return (
    arc(cx, cy, r, Math.PI / 2 + g, Math.PI / 2 - g, cls, w) +
    line(x0, y0, x0 + foot, y0, cls, w) +
    line(x1, y1, x1 - foot, y1, cls, w)
  );
};

const gt = (x, y, s, cls = "l-ink", sw = 3) =>
  path(`M${f(x)} ${f(y - s)} L${f(x + s * 0.85)} ${f(y)} L${f(x)} ${f(y + s)}`, cls, sw);

const spark4 = (cx, cy, ro, ri, cls = "l-fill-sage") => poly(starPts(cx, cy, ro, ri, 4), cls);

function nodeAt(x, y, r = 3.2, cls = "l-fill-ink") {
  return dot(x, y, r, cls);
}
function link(x1, y1, x2, y2, w = 1.8, cls = "l-dim") {
  return line(x1, y1, x2, y2, cls, w);
}

const bgCircle = (cx, cy, r) => `<circle cx="${f(cx)}" cy="${f(cy)}" r="${f(r)}" fill="var(--bg-elev)" stroke="none"/>`;
const paperFill = (d) => `<path class="l-fill-paper" d="${d}"/>`;
const strokeOnly = (d, cls = "l-ink", w = 3) => `<path d="${d}" fill="none" class="${cls}" style="stroke-width:${w}"/>`;

// ---- organic & dimensional helpers -----------------------------------------

function blobPath(cx, cy, r, wob, rng, n = 8) {
  const pts = Array.from({ length: n }, (_, i) => {
    const a = (i / n) * TAU;
    const rr = r * (1 - wob / 2 + rng() * wob);
    return pt(cx, cy, rr, a);
  });
  let d = "";
  for (let i = 0; i < n; i++) {
    const p0 = pts[(i - 1 + n) % n];
    const p1 = pts[i];
    const p2 = pts[(i + 1) % n];
    const p3 = pts[(i + 2) % n];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    if (i === 0) d += `M ${f(p1[0])} ${f(p1[1])} `;
    d += `C ${f(c1[0])} ${f(c1[1])}, ${f(c2[0])} ${f(c2[1])}, ${f(p2[0])} ${f(p2[1])} `;
  }
  return d + "Z";
}
const blob = (cx, cy, r, wob, rng, cls = "l-fill-ink", n = 8) =>
  `<path class="${cls}" d="${blobPath(cx, cy, r, wob, rng, n)}"/>`;

function ribbon(p0, p1, p2, p3, wStart, wEnd, cls = "l-fill-ink") {
  const N = 22;
  const up = [];
  const down = [];
  for (let i = 0; i <= N; i++) {
    const t = i / N;
    const mt = 1 - t;
    const x = mt * mt * mt * p0[0] + 3 * mt * mt * t * p1[0] + 3 * mt * t * t * p2[0] + t * t * t * p3[0];
    const y = mt * mt * mt * p0[1] + 3 * mt * mt * t * p1[1] + 3 * mt * t * t * p2[1] + t * t * t * p3[1];
    const dx = 3 * mt * mt * (p1[0] - p0[0]) + 6 * mt * t * (p2[0] - p1[0]) + 3 * t * t * (p3[0] - p2[0]);
    const dy = 3 * mt * mt * (p1[1] - p0[1]) + 6 * mt * t * (p2[1] - p1[1]) + 3 * t * t * (p3[1] - p2[1]);
    const len = Math.hypot(dx, dy) || 1;
    const w = ((wStart + (wEnd - wStart) * t) / 2) || 0.6;
    up.push([x + (-dy / len) * w, y + (dx / len) * w]);
    down.push([x - (-dy / len) * w, y - (dx / len) * w]);
  }
  return poly([...up, ...down.reverse()], cls);
}

function sineStroke(x0, x1, y, amp, waves, cls = "l-ink", w = 3) {
  const N = 48;
  let d = "";
  for (let i = 0; i <= N; i++) {
    const t = i / N;
    const x = x0 + (x1 - x0) * t;
    const yy = y + amp * Math.sin(t * waves * TAU);
    d += (i === 0 ? "M" : "L") + ` ${f(x)} ${f(yy)} `;
  }
  return strokeOnly(d, cls, w);
}

function hexPrism(cx, cy, r, h, topCls = "l-fill-sage", rightCls = "l-fill-dim", leftCls = "l-fill-ink") {
  const v = reg(cx, cy, r, 6);
  const b = v.map(([x, y]) => [x, y + h]);
  return (
    poly([v[2], v[3], b[3], b[2]], rightCls) +
    poly([v[3], v[4], b[4], b[3]], leftCls) +
    poly(v, topCls)
  );
}

function leafAt(x, y, len, ang, cls = "l-fill-sage") {
  const tip = pt(x, y, len, ang);
  const c1 = pt(x, y, len * 0.55, ang - 0.55);
  const c2 = pt(x, y, len * 0.55, ang + 0.55);
  return `<path class="${cls}" d="M ${f(x)} ${f(y)} Q ${f(c1[0])} ${f(c1[1])} ${f(tip[0])} ${f(tip[1])} Q ${f(c2[0])} ${f(c2[1])} ${f(x)} ${f(y)} Z"/>`;
}

function eyes(cx, y, sep = 7, r = 2.2, cls = "l-fill-paper") {
  return dot(cx - sep / 2, y, r, cls) + dot(cx + sep / 2, y, r, cls);
}

const FAMILIES = [
  {
    id: "atomic-velas",
    name: "Atomic Velas",
    seed: 3201,
    variants: [
      { vname: "Electron Trio Angled", concept: "Three planes of work circling one purpose.", spec: "Fan of tilted velas with riding electrons.", draw: () => { const on = (deg, a) => { const rr = deg * Math.PI / 180; const x = 20 * Math.cos(a), y = 8 * Math.sin(a); return [32 + x * Math.cos(rr) - y * Math.sin(rr), 32 + x * Math.sin(rr) + y * Math.cos(rr)]; }; return ell(32, 32, 20, 8, "l-ink", 2.6, 0) + ell(32, 32, 20, 8, "l-sage", 2.4, 60) + ell(32, 32, 20, 8, "l-dim", 2.2, -60) + (() => { const p = on(0, -0.5); return nodeAt(p[0], p[1], 3.2, "l-fill-ink"); })() + (() => { const p = on(60, 2.1); return nodeAt(p[0], p[1], 3, "l-fill-sage"); })() + (() => { const p = on(-60, 0.9); return nodeAt(p[0], p[1], 2.8); })() + nodeAt(32, 32, 4, "l-fill-ink"); } },
      { vname: "Tilted System Live", concept: "One vela caught mid-flight.", spec: "Bold tilted ellipse, electron with trail.", draw: () => { const rot = 24; const ex = 32 + 19 * Math.cos(-0.7), ey = 32 + 9 * Math.sin(-0.7); return raw(`<g transform="rotate(${rot} 32 32)">`) + ell(32, 32, 19, 9, "l-ink", 3) + nodeAt(51, 32, 3.6, "l-fill-sage") + line(46, 36, 42, 40, "l-faint", 2) + nodeAt(32, 32, 4.4, "l-fill-ink") + raw(`</g>`); } },
      { vname: "Twin Ring Atom", concept: "Two disciplines, one nucleus.", spec: "Concentric rings with opposite electrons.", draw: () => circ(32, 32, 11, "l-sage", 2.6) + circ(32, 32, 18, "l-ink", 3) + nodeAt(43, 32, 3, "l-fill-sage") + nodeAt(14, 32, 3.6, "l-fill-ink") + nodeAt(32, 32, 3.4, "l-fill-ink") },
      { vname: "Planet With Two Moons", concept: "A busy little solar system.", spec: "Core world, ring, two moons at phases.", draw: () => nodeAt(32, 34, 9, "l-fill-ink") + ell(32, 34, 17, 6, "l-sage", 2.8) + nodeAt(15, 30, 3.4, "l-fill-dim") + nodeAt(48, 39, 3, "l-fill-sage") },
      { vname: "Electron Swarm", concept: "Busy in here.", spec: "Dashed velas with four electrons varied.", draw: () => [[10, "l-faint"], [16, "l-dim"]].map(([r, cls]) => `<circle cx="32" cy="32" r="${r}" fill="none" class="${cls}" style="stroke-width:1.6;stroke-dasharray:3 4"/>`).join("") + nodeAt(pt(32, 32, 10, -1)[0], pt(32, 32, 10, -1)[1], 2.6, "l-fill-sage") + nodeAt(pt(32, 32, 16, 1.2)[0], pt(32, 32, 16, 1.2)[1], 3) + nodeAt(pt(32, 32, 16, 3.6)[0], pt(32, 32, 16, 3.6)[1], 2.8) + nodeAt(32, 32, 3.4, "l-fill-ink") },
      { vname: "Wing Velas Pair", concept: "Two planes lifting one center.", spec: "Parallel tilted ellipses offset vertically.", draw: () => ell(32, 22, 17, 6.5, "l-ink", 2.8, -18) + ell(32, 42, 17, 6.5, "l-sage", 2.8, -18) + link(25, 27, 25, 37, 1.8, "l-faint") + link(39, 27, 39, 37, 1.8, "l-faint") },
      { vname: "Electron Tail Sprint", concept: "Speed you can see.", spec: "Vela electron dragging fading dashes.", draw: () => circ(32, 32, 15, "l-faint", 1.8) + nodeAt(pt(32, 32, 15, -0.5)[0], pt(32, 32, 15, -0.5)[1], 4, "l-fill-sage") + line(21, 21, 25, 24, "l-dim", 2.4) + line(15, 28, 19, 29.5, "l-faint", 2) + nodeAt(32, 32, 3.4, "l-fill-ink") },
      { vname: "Nested Tilt Rings", concept: "Depth from three tilts.", spec: "Ellipses stepping through rotations.", draw: () => ell(32, 32, 20, 7, "l-faint", 2) + ell(32, 32, 16, 7, "l-dim", 2.4, 40) + ell(32, 32, 12, 7, "l-ink", 2.8, -40) + spark4(32, 32, 4.5, 1.8) },
      { vname: "Atom Over Tide", concept: "Science above the swell.", spec: "Compact atom riding a sine sea.", draw: () => circ(32, 22, 9, "l-ink", 2.8) + ell(32, 22, 15, 5.5, "l-sage", 2.4, -18) + nodeAt(46, 19, 2.8, "l-fill-sage") + sineStroke(10, 54, 50, 3.5, 1.6, "l-ink", 3) },
      { vname: "Ellipse Escalator", concept: "Scale as progress.", spec: "Four growing ellipses alternating tilt.", draw: () => [[9, 0, "l-faint"], [13, 35, "l-dim"], [17, 0, "l-sage"], [21, -35, "l-ink"]].map(([r, rot, cls]) => ell(32, 32, r, r * 0.42, cls, 2.4, rot)).join("") + nodeAt(32, 32, 2.8, "l-fill-ink") },
      { vname: "Precession Wobble", concept: "Stable motion that never repeats.", spec: "Ring with off-axis inner ellipse and stray moon.", draw: () => circ(32, 32, 16, "l-ink", 2.8) + ell(38, 30, 8, 5, "l-sage", 2.4, 30) + nodeAt(45, 26, 2.8, "l-fill-sage") + nodeAt(32, 32, 2.6, "l-fill-dim") },
      { vname: "Spark Nucleus Atom", concept: "Energy at the center of order.", spec: "Two clean ellipses around a spark.", draw: () => ell(32, 32, 18, 7, "l-ink", 2.8, 55) + ell(32, 32, 18, 7, "l-ink", 2.8, -55) + spark4(32, 32, 6, 2.2) },
      { vname: "Moon Chain Vela", concept: "Travelers sharing one road.", spec: "Three moons spaced on single ellipse.", draw: () => ell(32, 32, 19, 9, "l-dim", 2.2) + [-0.6, 1.5, 3.4].map((a, i) => nodeAt(pt(32, 32, 19, a)[0], pt(32, 32, 19, a)[1], 3.4 - i * 0.5, i === 1 ? "l-fill-sage" : "l-fill-ink")).join("") },
      { vname: "Intersection Dots", concept: "Where paths meet, things happen.", spec: "Crossed rings with dots at both crossings.", draw: () => circ(24, 32, 12, "l-ink", 2.8) + circ(40, 32, 12, "l-sage", 2.8) + nodeAt(32, 23, 3, "l-fill-ink") + nodeAt(32, 41, 3, "l-fill-ink") },
      { vname: "Constellation Chords", concept: "Satellites talking along the way.", spec: "Vela dots linked by soft chords.", draw: () => ell(32, 32, 19, 10, "l-faint", 1.8) + [[-0.4], [1.3], [2.9]].map(([a], i) => nodeAt(pt(32, 32, 19, a)[0], pt(32, 32, 19, a)[1], 3.2, i === 1 ? "l-fill-sage" : "l-fill-ink")).join("") + strokeOnly(`M ${pt(32, 32, 19, -0.4).map(f).join(" ")} Q ${f(32)} ${f(44)} ${pt(32, 32, 19, 1.3).map(f).join(" ")}`, "l-dim", 1.4) },
      { vname: "Heavy Core Whisper Rings", concept: "Mass speaks quietly.", spec: "Large solid core, hairline velas.", draw: () => circ(32, 32, 20, "l-faint", 1.4) + circ(32, 32, 14, "l-dim", 1.8) + nodeAt(32, 32, 8, "l-fill-ink") + nodeAt(46, 26, 2.4, "l-fill-sage") },
      { vname: "Dual Systems Linked", concept: "Two teams, one shared arc.", spec: "Small atom systems joined by a Vela arc.", draw: () => circ(16, 24, 6, "l-ink", 2.4) + ell(16, 24, 11, 4.5, "l-dim", 1.8, 20) + circ(48, 42, 6, "l-ink", 2.4) + ell(48, 42, 11, 4.5, "l-dim", 1.8, 20) + strokeOnly("M22 33 Q32 38 43 36", "l-sage", 2.6) },
      { vname: "Figure Eight Loop", concept: "Work that flows both ways.", spec: "Tangent ellipses forming a figure eight.", draw: () => ell(21, 32, 12, 7, "l-ink", 2.8) + ell(43, 32, 12, 7, "l-sage", 2.8) + nodeAt(32, 32, 3, "l-fill-ink") + nodeAt(11, 32, 2.6, "l-fill-dim") + nodeAt(53, 32, 2.6, "l-fill-dim") },
      { vname: "Polar And Equatorial", concept: "Every direction covered.", spec: "Vertical and horizontal ellipse cross.", draw: () => ell(32, 32, 8, 20, "l-sage", 2.8) + ell(32, 32, 20, 8, "l-ink", 3) + nodeAt(32, 12, 3, "l-fill-sage") + nodeAt(52, 32, 3, "l-fill-ink") },
      { vname: "Comet Circles Home", concept: "The long way back around.", spec: "Comet head circling into its ellipse.", draw: () => ell(32, 34, 16, 9, "l-dim", 2) + nodeAt(47, 26, 4.2, "l-fill-sage") + strokeOnly("M47 26 Q52 18 44 14", "l-dim", 2) },
    ],
  },
  {
    id: "hive-prisms",
    name: "Hive Prisms",
    seed: 3202,
    variants: [
      { vname: "Solo Prism", concept: "One cell with real weight.", spec: "Isometric hex prism, three-tone faces.", draw: () => hexPrism(32, 26, 17, 12) },
      { vname: "Prism Step Duo", concept: "Two tiers of the same build.", spec: "Stepped prisms, tall behind short.", draw: () => hexPrism(40, 24, 13, 10) + hexPrism(26, 34, 13, 8, "l-fill-ink", "l-fill-dim", "l-fill-sage") },
      { vname: "Prism Tower", concept: "Stacked to be noticed.", spec: "Three shrinking prisms stacked.", draw: () => hexPrism(32, 44, 16, 6) + hexPrism(32, 30, 12.5, 5) + hexPrism(32, 19, 9, 4, "l-fill-ink", "l-fill-dim", "l-fill-sage") },
      { vname: "Twin Prisms Bridge", concept: "Two towers sharing a span.", spec: "Prism pair linked by top slab.", draw: () => hexPrism(18, 28, 11, 14) + hexPrism(46, 28, 11, 14) + rectR(22, 20, 20, 5, 2, "l-fill-sage") },
      { vname: "Ascending Prism Stair", concept: "Built upward step by step.", spec: "Three prisms climbing rightward.", draw: () => hexPrism(14, 46, 10, 4) + hexPrism(32, 38, 10, 7) + hexPrism(50, 30, 10, 10) },
      { vname: "Great And Small Prisms", concept: "Senior and apprentice.", spec: "Large prism with tiny companion.", draw: () => hexPrism(27, 27, 16, 10) + hexPrism(50, 48, 7, 4, "l-fill-ink", "l-fill-dim", "l-fill-sage") },
      { vname: "Prism Under Orb", concept: "The work holding the idea aloft.", spec: "Prism balancing a floating orb.", draw: () => hexPrism(32, 36, 15, 9) + nodeAt(32, 14, 6, "l-fill-sage") },
      { vname: "Saddle Twin Prisms", concept: "A valley worth sitting in.", spec: "Two prisms forming a saddle gap.", draw: () => hexPrism(19, 30, 12, 9) + hexPrism(45, 30, 12, 9) + line(29, 42, 35, 42, "l-faint", 2) },
      { vname: "Diamond Cluster Prisms", concept: "Four cells, one formation.", spec: "Diamond arrangement of small prisms and cell.", draw: () => hexPrism(32, 10, 8, 3, "l-fill-ink", "l-fill-dim", "l-fill-sage") + hexPrism(51, 32, 8, 3, "l-fill-ink", "l-fill-dim", "l-fill-sage") + hexPrism(32, 54, 8, 3, "l-fill-ink", "l-fill-dim", "l-fill-sage") + hexStroke(13, 32, 8) },
      { vname: "Leaning Stack Prisms", concept: "Character through imbalance.", spec: "Sideways-offset prism stack.", draw: () => hexPrism(38, 44, 14, 6) + hexPrism(30, 32, 12, 5) + hexPrism(25, 22, 9, 4, "l-fill-ink", "l-fill-dim", "l-fill-sage") },
      { vname: "Prism Cast Shadow", concept: "Grounded by its own weight.", spec: "Prism with detached shadow slab.", draw: () => rectR(14, 52, 36, 5, 2.5, "l-fill-faint") + hexPrism(32, 32, 16, 10) },
      { vname: "Crown Totem Prism", concept: "Small crown, large presence.", spec: "Small prism atop broad one.", draw: () => hexPrism(32, 38, 16, 9) + hexPrism(32, 17, 8, 4, "l-fill-ink", "l-fill-dim", "l-fill-sage") },
      { vname: "Gateway Prism Towers", concept: "An entrance worth building.", spec: "Tower prisms under lintel slab.", draw: () => hexPrism(16, 32, 11, 16) + hexPrism(48, 32, 11, 16) + rectR(12, 12, 40, 6, 3, "l-fill-sage") },
      { vname: "Wireframe Prism Study", concept: "The same idea, drawn as structure.", spec: "Outline-only hex prism.", draw: () => { const v = reg(32, 26, 17, 6); const b = v.map(([x, y]) => [x, y + 12]); return strokeOnly(`${v.map((p) => p.map(f).join(",")).join(" L")} Z`, "l-ink", 2.8) + strokeOnly(`M ${v[2].map(f).join(" ")} L ${b[2].map(f).join(" ")} L ${b[3].map(f).join(" ")} L ${v[3].map(f).join(" ")}`, "l-ink", 2) + strokeOnly(`M ${v[3].map(f).join(" ")} L ${v[4].map(f).join(" ")} M ${b[1].map(f).join(" ")} L ${b[2].map(f).join(" ")}`, "l-faint", 1.8); } },
      { vname: "Baseline Prism Row", concept: "Products on the shelf.", spec: "Three varied prisms on one rule.", draw: () => line(6, 55, 58, 55, "l-ink", 3) + hexPrism(17, 43, 9, 4) + hexPrism(33, 41, 10, 6) + hexPrism(49, 45, 8, 3, "l-fill-ink", "l-fill-dim", "l-fill-sage") },
      { vname: "Beacon Prism Summit", concept: "Done building; now shining.", spec: "Tall prism crowned with beacon dot and ticks.", draw: () => hexPrism(32, 38, 15, 10) + nodeAt(32, 15, 4, "l-fill-sage") + [[-0.6], [Math.PI + 0.6]].map(([a]) => { const [x1, y1] = pt(32, 15, 6, a - 0.3); const [x2, y2] = pt(32, 15, 9, a); const [x3, y3] = pt(32, 15, 6, a + 0.3); return strokeOnly(`M ${f(x1)} ${f(y1)} Q ${f(x2)} ${f(y2)} ${f(x3)} ${f(y3)}`, "l-dim", 2); }).join("") },
      { vname: "Prism On Pedestal Slab", concept: "Presented with care.", spec: "Prism standing on a wide pedestal slab.", draw: () => hexPrism(32, 30, 14, 9) + rectR(10, 48, 44, 6, 3, "l-fill-ink") },
      { vname: "Prism Pair Vela", concept: "Two cells circling each other.", spec: "Small prisms on an elliptical dance.", draw: () => ell(32, 32, 20, 10, "l-faint", 1.8) + hexPrism(12, 30, 7, 3, "l-fill-ink", "l-fill-dim", "l-fill-sage") + hexPrism(50, 36, 7, 3, "l-fill-sage", "l-fill-ink", "l-fill-dim") },
      { vname: "Prism And Vine", concept: "Even structures grow things.", spec: "Prism with vine curling up its side.", draw: () => hexPrism(28, 34, 14, 8) + strokeOnly("M42 52 C48 44 44 36 47 28 Q49 22 45 18", "l-sage", 2.6) + leafAt(46, 26, 7, -Math.PI / 3) },
      { vname: "Nested Prism Rings", concept: "A cell within a cell within a cell.", spec: "Three concentric prism outlines.", draw: () => hexStroke(32, 30, 21) + hexStroke(32, 30, 14, "l-dim", 2.2) + hexStroke(32, 30, 7.5, "l-sage", 2.4) },
    ],
  },
  {
    id: "ribbon-flow",
    name: "Ribbon Flow",
    seed: 3203,
    variants: [
      { vname: "S Ribbon Bead", concept: "Energy threading the work.", spec: "Tapered ribbon weaving through a bead dot.", draw: () => ribbon([10, 14], [40, 14], [22, 50], [54, 50], 7, 2.5, "l-fill-ink") + nodeAt(32, 32, 5, "l-fill-sage") },
      { vname: "Loop The Loop Ribbon", concept: "One flourish that circles back.", spec: "Ribbon looping around center core.", draw: () => ribbon([8, 46], [26, 8], [44, 56], [56, 18], 6.5, 2.2, "l-fill-ink") + circ(32, 32, 7, "l-sage", 3) },
      { vname: "Infinity Ribbon Tied", concept: "Endless capacity, tied at the waist.", spec: "Figure-eight ribbon with center knot dot.", draw: () => strokeOnly("M32 32 C22 18 8 22 8 32 C8 42 22 46 32 32 C42 18 56 22 56 32 C56 42 42 46 32 32 Z", "l-ink", 3.4) + nodeAt(32, 32, 4, "l-fill-sage") },
      { vname: "Weave Through Walls", concept: "Threading between constraints.", spec: "Hex outline with ribbon passing over-under.", draw: () => hexStroke(32, 32, 21) + ribbon([6, 40], [26, 44], [38, 22], [60, 26], 6, 2.4, "l-fill-ink") + raw(`<rect x="28" y="30" width="9" height="9" fill="var(--bg-elev)" stroke="none"/>`) + nodeAt(32, 34, 3.4, "l-fill-sage") },
      { vname: "Merge Split Ribbons", concept: "Many voices, one line.", spec: "Thin ribbons converging into thick exit.", draw: () => ribbon([8, 14], [24, 16], [34, 24], [46, 30], 2.2, 6, "l-fill-ink") + ribbon([8, 50], [24, 48], [34, 38], [46, 32], 2.2, 6, "l-fill-sage") + poly([[44, 26], [56, 31], [44, 36]], "l-fill-ink") },
      { vname: "Cascade Trio Falls", concept: "Terrace water.", spec: "Three short cascading ribbons.", draw: () => ribbon([14, 12], [26, 18], [20, 26], [30, 32], 5.5, 1.8, "l-fill-ink") + ribbon([30, 22], [42, 28], [36, 36], [46, 42], 5.5, 1.8, "l-fill-sage") + ribbon([18, 40], [30, 46], [24, 52], [34, 58], 5.5, 1.8, "l-fill-dim") },
      { vname: "Encircle Sweep", concept: "Almost holding everything.", spec: "Open C sweep around solid orb.", draw: () => ribbon([46, 12], [62, 30], [46, 52], [18, 46], 6, 2, "l-fill-ink") + nodeAt(30, 32, 8, "l-fill-sage") },
      { vname: "Knot At Center", concept: "Commitment tied once.", spec: "Crossing ribbon with knot node.", draw: () => ribbon([8, 12], [44, 20], [18, 44], [56, 52], 5.5, 2.2, "l-fill-ink") + nodeAt(31, 32, 4.4, "l-fill-sage") },
      { vname: "Inward Spiral Ribbon", concept: "Focus winding down.", spec: "Ribbon spiraling to center point.", draw: () => { let d = "M 54 32 "; let r = 22; for (let i = 0; i < 3; i++) { d += `A ${f(r)} ${f(r * 0.72)} 0 0 1 ${f(32 - (r -= 7))} 32 A ${f(r)} ${f(r * 0.72)} 0 0 1 ${f(32)} 32 `; } return strokeOnly(d, "l-ink", 3.4); } },
      { vname: "Valley Dip Ribbon", concept: "Composure between peaks.", spec: "Ribbon dipping between two triangles.", draw: () => poly([[8, 20], [18, 8], [28, 20]], "l-fill-dim") + poly([[36, 20], [46, 8], [56, 20]], "l-fill-dim") + ribbon([8, 34], [22, 52], [42, 52], [56, 34], 5.5, 2.2, "l-fill-ink") },
      { vname: "Double Helix Strands", concept: "Two strands, one code.", spec: "Intertwined crossing strands with rung ties.", draw: () => ribbon([14, 8], [50, 24], [14, 40], [50, 56], 4.5, 3, "l-fill-ink") + ribbon([50, 8], [14, 24], [50, 40], [14, 56], 4.5, 3, "l-fill-sage") + line(24, 18, 40, 18, "l-faint", 1.6) + line(28, 32, 36, 32, "l-faint", 1.6) + line(24, 46, 40, 46, "l-faint", 1.6) },
      { vname: "Exit Arrow Ribbon", concept: "Shipping it out the corner.", spec: "Ribbon ending in arrowhead at frame edge.", draw: () => ribbon([10, 50], [30, 46], [42, 34], [50, 20], 6.5, 2.5, "l-fill-ink") + poly([[46, 16], [57, 13], [54, 24]], "l-fill-sage") },
      { vname: "Wave Rider Ribbon", concept: "Surfing three swells.", spec: "Ribbon riding over repeated bumps.", draw: () => sineStroke(8, 56, 40, 6, 2.4, "l-faint", 3) + ribbon([10, 30], [24, 18], [40, 18], [54, 30], 6, 2.4, "l-fill-ink") },
      { vname: "Tether Swing Dot", concept: "Swinging on its own thread.", spec: "Bead swinging from a ribbon arc.", draw: () => strokeOnly("M12 14 Q32 10 50 22", "l-dim", 2.4) + ribbon([50, 22], [56, 34], [48, 44], [36, 46], 4.5, 1.6, "l-fill-ink") + nodeAt(34, 47, 4, "l-fill-sage") },
      { vname: "Forked End Ribbon", concept: "One path, many outcomes.", spec: "Ribbon splitting into two rounded tips.", draw: () => ribbon([8, 32], [24, 30], [36, 28], [44, 26], 6, 3.5, "l-fill-ink") + nodeAt(50, 20, 3.6, "l-fill-sage") + nodeAt(50, 34, 3.6, "l-fill-sage") + link(44, 26, 47, 22, 3, "l-ink") + link(44, 27, 47, 32, 3, "l-ink") },
      { vname: "Halo Sweep Orb", concept: "Ring half-written.", spec: "Broad sweep wrapping a sphere.", draw: () => ribbon([12, 40], [16, 16], [48, 8], [56, 26], 6.5, 2, "l-fill-ink") + nodeAt(32, 36, 9, "l-fill-sage") },
      { vname: "Fold Back Ribbon", concept: "Return trips matter.", spec: "Ribbon heading out then folding under.", draw: () => ribbon([8, 20], [34, 16], [48, 22], [52, 34], 6, 3, "l-fill-ink") + ribbon([52, 34], [54, 44], [40, 50], [24, 48], 3, 5.5, "l-fill-sage") },
      { vname: "Triple Strand Braid", concept: "Three strands, one rope.", spec: "Over-under braid of three strokes.", draw: () => strokeOnly("M12 12 C36 22 28 30 12 38 C36 48 28 54 52 44", "l-ink", 3) + strokeOnly("M20 8 C40 18 36 26 20 34 C44 46 40 50 56 40", "l-sage", 3) + strokeOnly("M32 10 C48 20 44 28 32 36", "l-dim", 2.4) },
      { vname: "Pour Into Cup", concept: "Everything delivered somewhere.", spec: "Tapered stream pouring into bracket cup.", draw: () => ribbon([26, 8], [34, 18], [30, 28], [33, 40], 5.5, 2.2, "l-fill-ink") + strokeOnly("M20 44 V52 Q20 58 27 58 H37 Q44 58 44 52 V44", "l-ink", 3.4) + nodeAt(33, 51, 3, "l-fill-sage") },
      { vname: "Round Trip Comet", concept: "Out and home again.", spec: "Looping comet ribbon returning to origin.", draw: () => nodeAt(14, 44, 4, "l-fill-sage") + ribbon([14, 40], [20, 16], [48, 12], [54, 28], 6, 2.2, "l-fill-ink") + strokeOnly("M54 28 Q56 42 40 46 Q28 49 18 46", "l-dim", 2.2) },
    ],
  },
  {
    id: "living-cells",
    name: "Living Cells",
    seed: 3204,
    variants: [
      { vname: "Petri Solo", concept: "One healthy cell at rest.", spec: "Organic blob with off-center nucleus.", draw: (rng) => blob(30, 32, 18, 0.24, rng) + nodeAt(35, 29, 5.5, "l-fill-paper") },
      { vname: "First Mitosis", concept: "Growth means dividing.", spec: "Pinched peanut cell with center seam.", draw: () => strokeOnly("M26 12 C40 12 46 20 44 32 C46 44 40 52 26 52 C12 52 6 44 8 32 C6 20 12 12 26 12 Z", "l-ink", 3) + strokeOnly("M32 14 C37 23 37 41 32 50", "l-dim", 2.2) },
      { vname: "Cell Family Buds", concept: "Parent cell, new buds.", spec: "Large blob with two attached buds.", draw: (rng) => blob(28, 34, 16, 0.2, rng) + blob(48, 22, 7, 0.25, rng, "l-fill-sage") + blob(51, 43, 5, 0.25, rng, "l-fill-dim") },
      { vname: "Twin Nuclei Merge", concept: "Two cores deciding to share.", spec: "Blob containing two nuclei touching.", draw: (rng) => blob(32, 32, 19, 0.22, rng, "l-fill-sage") + nodeAt(27, 31, 5, "l-fill-ink") + nodeAt(38, 33, 5, "l-fill-ink") },
      { vname: "Wobble Trio Diagonal", concept: "Three cells drifting together.", spec: "Blobs stepping down the diagonal.", draw: (rng) => blob(14, 16, 8, 0.28, rng, "l-fill-dim") + blob(32, 32, 11, 0.24, rng) + blob(51, 49, 9, 0.26, rng, "l-fill-sage") },
      { vname: "Vacuole Cell", concept: "Holding a little in reserve.", spec: "Blob with two paper vacuoles.", draw: (rng) => blob(32, 32, 18, 0.26, rng, "l-fill-ink") + dot(26, 26, 4, "l-fill-paper") + dot(38, 38, 3, "l-fill-paper") },
      { vname: "Fission Line Trio", concept: "One became three.", spec: "Three blobs in fission row, shrinking gap.", draw: (rng) => blob(12, 32, 9, 0.24, rng, "l-fill-dim") + blob(32, 32, 10, 0.24, rng) + blob(52, 32, 9, 0.24, rng, "l-fill-sage") },
      { vname: "Sprouting Cell", concept: "Even cells dream of leaves.", spec: "Blob with leaf sprout on top.", draw: (rng) => blob(32, 38, 16, 0.24, rng) + line(32, 22, 32, 14, "l-ink", 2.6) + leafAt(32, 14, 9, -Math.PI / 2 - 0.5) + leafAt(32, 14, 9, -Math.PI / 2 + 0.5, "l-fill-dim") },
      { vname: "Binary Blob Vela", concept: "Two blobs sharing one center.", spec: "Organic binary system.", draw: (rng) => blob(21, 30, 10, 0.26, rng) + blob(43, 36, 8, 0.28, rng, "l-fill-sage") + ell(32, 33, 17, 13, "l-faint", 1.6) },
      { vname: "Polarity Cilia", concept: "Sensing the current.", spec: "Blob with directional cilia ticks.", draw: (rng) => blob(30, 32, 16, 0.22, rng) + [[8, 24], [6, 32], [8, 40]].map(([x, y]) => line(x, y, x + 5, y, "l-dim", 2)).join("") },
      { vname: "Colony Patch", concept: "A small settlement of cells.", spec: "Five tiny cells clustered organically.", draw: (rng) => blob(22, 22, 7, 0.3, rng, "l-fill-dim") + blob(40, 18, 6, 0.3, rng) + blob(48, 36, 7, 0.3, rng, "l-fill-sage") + blob(28, 42, 6.5, 0.3, rng) + blob(38, 52, 5, 0.3, rng, "l-fill-dim") },
      { vname: "Peanut Pinch", concept: "The moment before two.", spec: "Single pinched outline cell.", draw: () => strokeOnly("M22 12 C34 12 30 26 32 32 C34 38 30 52 22 52 C12 52 10 42 12 32 C10 22 12 12 22 12 Z M42 12 C54 12 56 22 54 32 C56 42 54 52 42 52 C34 52 34 40 32 32 C34 24 34 12 42 12 Z", "l-ink", 3) },
      { vname: "Amoeba Reach", concept: "Reaching for what's next.", spec: "Blob extending pseudopod toward dot.", draw: (rng) => blob(26, 34, 15, 0.3, rng) + strokeOnly("M36 26 Q46 18 52 16", "l-ink", 4) + nodeAt(55, 15, 3.4, "l-fill-sage") },
      { vname: "Rising Bubbles", concept: "Upward and outward.", spec: "Three organic bubbles rising.", draw: (rng) => blob(40, 50, 9, 0.22, rng, "l-fill-dim") + blob(30, 30, 7, 0.26, rng) + blob(38, 13, 5, 0.3, rng, "l-fill-sage") },
      { vname: "Double Membrane Cell", concept: "Well protected inside and out.", spec: "Wobbly outer membrane, smooth inner ring.", draw: (rng) => blob(32, 32, 19, 0.28, rng) + circ(32, 32, 12, "l-ink", 2.4) + nodeAt(32, 32, 4, "l-fill-sage") },
      { vname: "Spore Release Trail", concept: "Sending spores into the world.", spec: "Parent cell with trailing released spores.", draw: (rng) => blob(20, 40, 13, 0.24, rng) + nodeAt(38, 30, 3.4) + nodeAt(47, 22, 2.8, "l-fill-sage") + nodeAt(54, 14, 2.2, "l-fill-dim") },
      { vname: "Zen Pebble Stack Organic", concept: "Balance, but softer.", spec: "Wobbling pebbles stacked.", draw: (rng) => blob(32, 50, 11, 0.18, rng, "l-fill-dim") + blob(31, 33, 9, 0.2, rng) + blob(33, 18, 6.5, 0.22, rng, "l-fill-sage") },
      { vname: "Growth Ring Cell", concept: "Older and wiser each cycle.", spec: "Blob outline with inner echo rings.", draw: (rng) => blob(32, 32, 19, 0.22, rng, "l-fill-ink") + blob(32, 32, 12, 0.2, rng, "l-fill-sage") + nodeAt(33, 33, 4.5, "l-fill-paper") },
      { vname: "Yeast Budding Chain", concept: "Generation after generation.", spec: "Chain of budding circles shrinking.", draw: (rng) => blob(16, 40, 11, 0.22, rng) + blob(33, 32, 8, 0.24, rng, "l-fill-sage") + blob(45, 25, 5.5, 0.26, rng) },
      { vname: "Directional Drift Cell", concept: "Moving with intent.", spec: "Blob with motion streaks behind.", draw: (rng) => blob(36, 32, 15, 0.24, rng, "l-fill-sage") + line(8, 26, 17, 26, "l-dim", 2.4) + line(5, 34, 16, 34, "l-faint", 2.2) + line(9, 42, 18, 42, "l-dim", 2) },
    ],
  },
  {
    id: "pulse-rivers",
    name: "Pulse Rivers",
    seed: 3205,
    variants: [
      { vname: "Heartbeat Riverbanks", concept: "The pulse held by steady banks.", spec: "ECG line between two bank rules.", draw: () => line(8, 14, 56, 14, "l-faint", 2) + line(8, 50, 56, 50, "l-faint", 2) + strokeOnly("M10 32 H22 L28 20 L34 44 L39 32 H54", "l-ink", 3.4) },
      { vname: "Triple Sine Stack", concept: "Signal in three voices.", spec: "Stacked sines growing amplitude.", draw: () => sineStroke(10, 54, 18, 3, 2.2, "l-faint", 2.4) + sineStroke(10, 54, 31, 5.5, 2.2, "l-sage", 3) + sineStroke(10, 54, 45, 8, 2.2, "l-ink", 3.6) },
      { vname: "Spike To Swell", concept: "Alarm melting into calm.", spec: "Flatline spike easing into smooth wave.", draw: () => strokeOnly("M8 32 H22 L28 22 L33 40 L37 30 Q42 24 47 30 T56 30", "l-ink", 3.2) },
      { vname: "Calm Pulse III", concept: "Your favorite, softened and enlarged.", spec: "One gentle spike with sage node.", draw: () => strokeOnly("M8 34 H22 L28 21 L35 46 L40 34 H56", "l-ink", 3.8) + nodeAt(28, 21, 3.4, "l-fill-sage") },
      { vname: "Beat Markers Line", concept: "Counting the moments that matter.", spec: "Waveform with dots on every peak.", draw: () => strokeOnly("M8 36 H16 L21 26 L26 36 H31 L36 22 L41 36 H46 L51 27 H56", "l-ink", 3) + nodeAt(21, 26, 2.6, "l-fill-sage") + nodeAt(36, 22, 2.6, "l-fill-sage") + nodeAt(51, 27, 2.6, "l-fill-dim") },
      { vname: "Interleaved Frequencies", concept: "Fast thoughts over slow ones.", spec: "High and low frequency waves crossing.", draw: () => sineStroke(8, 56, 26, 4, 4, "l-ink", 2.8) + sineStroke(8, 56, 38, 3, 1.6, "l-sage", 3) },
      { vname: "Echo Fade Spikes", concept: "Shouting once, then whispering.", spec: "Three spikes fading in sequence.", draw: () => strokeOnly("M8 38 L13 24 L18 38", "l-ink", 3.4) + strokeOnly("M24 38 L29 28 L34 38", "l-sage", 3) + strokeOnly("M40 38 L45 31 L50 38", "l-dim", 2.6) },
      { vname: "Mountain Range Wave", concept: "A skyline made of signal.", spec: "Continuous jagged range line.", draw: () => strokeOnly("M6 48 L15 30 L23 40 L32 20 L41 36 L49 26 L58 44", "l-ink", 3.2) },
      { vname: "Bars To Melody", concept: "Levels learning to sing.", spec: "Bar tops forming a sine melody.", draw: () => [12, 22, 32, 42, 52].map((x, i) => { const h = 12 + 9 * (1 + Math.sin(i * 1.3)); return rectR(x - 3.5, 50 - h, 7, h, 3, i === 2 ? "l-fill-sage" : "l-fill-ink"); }).join("") },
      { vname: "Capsuled Vital", concept: "Health check, contained.", spec: "Pulse inside stadium outline.", draw: () => rectR(7, 17, 50, 30, 15, "l-ink", 3) + strokeOnly("M15 32 H24 L28 25 L33 39 L37 32 H49", "l-sage", 3.2) },
      { vname: "Single Breath Wave", concept: "Inhale. Exhale. Continue.", spec: "One slow large breathing curve.", draw: () => strokeOnly("M8 40 C18 10 30 10 32 32 C34 54 46 54 56 24", "l-ink", 3.4) },
      { vname: "Seismo Between Rails", concept: "Everything recorded, nothing judged.", spec: "Jitter trace between two rails.", draw: () => line(8, 14, 56, 14, "l-faint", 1.8) + line(8, 50, 56, 50, "l-faint", 1.8) + strokeOnly("M10 32 L15 24 L19 38 L23 20 L28 44 L33 26 L37 40 L42 22 L47 36 L52 28 L55 32", "l-dim", 2) + nodeAt(33, 26, 2.6, "l-fill-sage") },
      { vname: "Self Crossing Wave", concept: "One line, one decision.", spec: "Wave crossing itself at center knot.", draw: () => strokeOnly("M8 40 C22 16 30 16 32 32 C34 48 42 48 56 24", "l-ink", 3.2) + nodeAt(32, 32, 3, "l-fill-sage") },
      { vname: "Two Rivers Confluence", concept: "Where currents agree.", spec: "Two sines merging into one channel.", draw: () => sineStroke(8, 24, 24, 4, 1.2, "l-ink", 3) + sineStroke(8, 24, 40, 4, 1.2, "l-sage", 3) + strokeOnly("M26 26 C38 26 40 32 54 32 M26 38 C38 38 40 32 54 32", "l-ink", 3) },
      { vname: "Syncopated Spikes", concept: "Rhythm you don't expect.", spec: "Irregular beat pattern.", draw: () => strokeOnly("M8 38 H14 L18 28 L22 38 H27 L30 18 L34 38 H42 L45 31 H56", "l-ink", 3.2) },
      { vname: "Deep Water Swell", concept: "Slow power below.", spec: "Deep swells with rising bubbles.", draw: () => sineStroke(6, 58, 36, 7, 1.2, "l-ink", 3.4) + nodeAt(20, 20, 2.4, "l-fill-dim") + nodeAt(30, 14, 1.8, "l-fill-dim") + nodeAt(43, 18, 2.1, "l-fill-dim") },
      { vname: "Wave Into Arrow", concept: "Signal becoming direction.", spec: "Waveform resolving into arrowhead.", draw: () => strokeOnly("M8 32 Q16 20 24 32 T40 32", "l-ink", 3.2) + poly([[40, 24], [54, 32], [40, 40]], "l-fill-sage") },
      { vname: "Vital Tag Badge", concept: "Status: alive, in a tag.", spec: "Pointed tag framing a small pulse.", draw: () => strokeOnly("M14 12 H50 V40 L32 52 L14 40 Z", "l-ink", 3) + strokeOnly("M22 28 H28 L31 23 L35 33 L38 28 H42", "l-sage", 2.8) },
      { vname: "Envelope Guides Wave", concept: "Discipline sets the range.", spec: "Converging guides containing a wave.", draw: () => line(8, 16, 56, 26, "l-faint", 2) + line(8, 48, 56, 38, "l-faint", 2) + sineStroke(10, 54, 32, 6, 2, "l-ink", 3.2) },
      { vname: "Revival At The End", concept: "Never fully flat.", spec: "Long flatline with hopeful rise.", draw: () => line(8, 34, 34, 34, "l-dim", 3.2) + strokeOnly("M34 34 L40 22 L45 42 L50 30 L54 26", "l-sage", 3.4) },
    ],
  },
  {
    id: "vela-systems",
    name: "Vela Systems",
    seed: 3206,
    variants: [
      { vname: "Ringed World Two Moons", concept: "A system in full operation.", spec: "Ringed planet with moons at phases.", draw: () => nodeAt(30, 32, 10, "l-fill-ink") + ell(32, 32, 20, 7, "l-sage", 3) + nodeAt(52, 26, 3.4, "l-fill-sage") + nodeAt(9, 40, 2.8, "l-fill-dim") },
      { vname: "Inner Outer Shells", concept: "Close work and far horizons.", spec: "Two velas; inner one moon, outer two.", draw: () => circ(32, 32, 8, "l-ink", 2.4) + circ(32, 32, 13, "l-dim", 1.8) + circ(32, 32, 19, "l-ink", 2.6) + nodeAt(pt(32, 32, 13, 0.5)[0], pt(32, 32, 13, 0.5)[1], 3) + nodeAt(pt(32, 32, 19, -0.8)[0], pt(32, 32, 19, -0.8)[1], 3.2, "l-fill-sage") + nodeAt(pt(32, 32, 19, 2.4)[0], pt(32, 32, 19, 2.4)[1], 3) },
      { vname: "Tilted System Fan", concept: "Three approaches to one truth.", spec: "Fan of three tilted ellipses with core.", draw: () => ell(32, 32, 19, 8, "l-ink", 2.6, 0) + ell(32, 32, 19, 8, "l-sage", 2.6, 55) + ell(32, 32, 19, 8, "l-dim", 2.4, -55) + nodeAt(32, 32, 4, "l-fill-ink") },
      { vname: "Binary Stars Barred", concept: "Two brights, one balance beam.", spec: "Stars at bar ends rotating common center.", draw: () => line(14, 32, 50, 32, "l-faint", 1.8) + spark4(14, 32, 6, 2.4, "l-fill-ink") + spark4(50, 32, 6, 2.4, "l-fill-sage") + circ(32, 32, 4.5, "l-dim", 2) },
      { vname: "Precession Trio", concept: "The slow turn nobody sees.", spec: "Same ellipse at three precession angles.", draw: () => ell(32, 32, 18, 8, "l-faint", 1.8, -25) + ell(32, 32, 18, 8, "l-dim", 2.2, 15) + ell(32, 32, 18, 8, "l-ink", 2.8, 55) + nodeAt(32, 32, 3.4, "l-fill-ink") },
      { vname: "Moon Phase Arc", concept: "Companions along the way.", spec: "Dashed arc with three moon dots.", draw: () => `<path d="M 12 44 A 22 22 0 0 1 52 44" fill="none" class="l-faint" style="stroke-width:2;stroke-dasharray:3 4"/>` + nodeAt(12, 44, 3.4) + nodeAt(32, 22, 3, "l-fill-sage") + nodeAt(52, 44, 2.6, "l-fill-dim") },
      { vname: "Gap Ring Particles", concept: "Order carved out of the ring.", spec: "Dotted ring with swept gap and particles.", draw: () => `<circle cx="32" cy="32" r="16" fill="none" class="l-dim" style="stroke-width:3;stroke-dasharray:2 5"/>` + arc(32, 32, 16, Math.PI * 0.75, Math.PI * 1.35, "l-ink", 3.2) + nodeAt(47, 21, 2.6, "l-fill-sage") + nodeAt(51, 27, 2.2, "l-fill-sage") },
      { vname: "Retrograde Visitor", concept: "One going against the grain.", spec: "Counter arrows on nested velas.", draw: () => circ(32, 32, 18, "l-dim", 2.4) + strokeOnly("M17 28 Q24 22 32 24 M47 36 Q40 42 32 40", "l-sage", 2.6) + poly([[15, 24], [18, 29], [21, 24]], "l-fill-sage") + poly([[49, 40], [46, 35], [43, 40]], "l-fill-sage") + nodeAt(32, 32, 4, "l-fill-ink") },
      { vname: "Lagrange Triangle", concept: "Stable spots worth knowing.", spec: "Two bodies plus equilibrium points.", draw: () => nodeAt(18, 32, 5.5, "l-fill-ink") + nodeAt(46, 32, 4, "l-fill-sage") + [[32, 18], [32, 46], [32, 32]].map(([x, y], i) => nodeAt(x, y, i === 2 ? 2.8 : 2.2, i === 2 ? "l-fill-sage" : "l-fill-dim")).join("") },
      { vname: "Slingshot Curve", concept: "Borrowing speed from the pass.", spec: "Inbound curve whipped around planet.", draw: () => nodeAt(40, 38, 9, "l-fill-ink") + strokeOnly("M8 10 Q34 12 44 26 Q50 36 40 44 Q30 50 22 44", "l-ink", 3) + nodeAt(8, 10, 3, "l-fill-sage") },
      { vname: "Resonance Chain Dots", concept: "Keeping time, perfectly.", spec: "Three moons at harmonic positions.", draw: () => circ(32, 32, 8, "l-ink", 2.4) + circ(32, 32, 14, "l-dim", 1.8) + circ(32, 32, 20, "l-dim", 1.8) + nodeAt(pt(32, 32, 14, -1.2)[0], pt(32, 32, 14, -1.2)[1], 2.8, "l-fill-sage") + nodeAt(pt(32, 32, 20, -1.2)[0], pt(32, 32, 20, -1.2)[1], 3) },
      { vname: "Polar Grid System", concept: "Everything measured from above.", spec: "Concentric rings with radial spokes.", draw: () => [8, 14, 20].map((r) => circ(32, 32, r, r === 14 ? "l-dim" : "l-ink", r === 8 ? 2 : 1.8)).join("") + [0, 90, 180, 270].map((d) => { const a = (d * Math.PI) / 180; return line(pt(32, 32, 8, a)[0], pt(32, 32, 8, a)[1], pt(32, 32, 20, a)[0], pt(32, 32, 20, a)[1], "l-faint", 1.4) + nodeAt(pt(32, 32, 20, a)[0], pt(32, 32, 20, a)[1], 2.4); }).join("") },
      { vname: "Eccentric Focus Vela", concept: "Off-center on purpose.", spec: "Ellipse with core at focus point.", draw: () => ell(36, 32, 20, 11, "l-ink", 2.8) + nodeAt(24, 32, 4.4, "l-fill-sage") + nodeAt(56, 32, 3.4) },
      { vname: "Shepherded Debris Ring", concept: "Kept in line by two guardians.", spec: "Ring flanked by shepherd moons.", draw: () => `<circle cx="32" cy="32" r="14" fill="none" class="l-dim" style="stroke-width:2.4;stroke-dasharray:2 4"/>` + nodeAt(32, 13, 3.4, "l-fill-sage") + nodeAt(32, 51, 3.4, "l-fill-ink"),
      },
      { vname: "Capture Spiral Descent", concept: "Falling into a good vela.", spec: "Spiral settling onto circular path.", draw: () => strokeOnly("M54 12 Q58 30 44 40 Q32 48 20 40 Q12 33 18 26", "l-dim", 2.2) + circ(32, 32, 12, "l-ink", 2.8) + nodeAt(44, 40, 3.2, "l-fill-sage") },
      { vname: "Escape Vector Marked", concept: "Knowing when to leave.", spec: "Vela with tangential departure arrow.", draw: () => circ(32, 32, 14, "l-dim", 2.4) + nodeAt(32, 32, 5, "l-fill-ink") + nodeAt(45, 22, 3.4, "l-fill-sage") + line(48, 19, 55, 12, "l-sage", 2.8) + poly([[56, 10], [57, 16], [51, 14]], "l-fill-sage") },
      { vname: "Torus Pair Offset", concept: "Fields overlapping in depth.", spec: "Two offset elliptical tori.", draw: () => ell(26, 28, 16, 7, "l-dim", 2.6) + ell(38, 37, 16, 7, "l-ink", 3) + nodeAt(38, 37, 3, "l-fill-sage") },
      { vname: "Day Night Terminator", concept: "One world, two truths.", spec: "Disc split by terminator with moon companion.", draw: () => circ(28, 32, 14, "l-ink", 3) + paperFill("M28 18 A14 14 0 0 1 28 46 Z") + nodeAt(50, 24, 3.4, "l-fill-sage") },
      { vname: "Gravity Lens Bend", concept: "Heavy ideas bend the paths near them.", spec: "Straight rays curving around massive core.", draw: () => nodeAt(32, 34, 7, "l-fill-ink") + strokeOnly("M6 20 Q32 26 58 20", "l-dim", 2.2) + strokeOnly("M6 48 Q32 42 58 48", "l-dim", 2.2) + nodeAt(6, 20, 2.4) + nodeAt(58, 48, 2.4, "l-fill-sage") },
      { vname: "Transfer Burn Vela", concept: "The climb between two altitudes.", spec: "Transfer arc linking inner and outer rings.", draw: () => circ(32, 36, 12, "l-dim", 2) + circ(32, 30, 21, "l-faint", 1.8) + nodeAt(32, 24, 3.4, "l-fill-sage") + strokeOnly("M22 44 Q18 32 28 22", "l-sage", 2.8) },
    ],
  },
  {
    id: "brush-omega",
    name: "Brush Omega",
    seed: 3207,
    variants: [
      { vname: "Calligraphy Omega", concept: "Written by a confident hand.", spec: "Thick crown tapering to fine feet.", draw: () => { const g = (55 * Math.PI) / 180; const [x0, y0] = pt(32, 32, 14, Math.PI / 2 + g); const [xm, ym] = pt(32, 32, 14, -Math.PI / 2); const [x1, y1] = pt(32, 32, 14, Math.PI / 2 - g); return raw(`<path d="M ${f(x0)} ${f(y0)} A 14 14 0 0 1 ${f(xm)} ${f(ym)}" fill="none" class="l-ink" style="stroke-width:5.6"/>`) + raw(`<path d="M ${f(xm)} ${f(ym)} A 14 14 0 0 1 ${f(x1)} ${f(y1)}" fill="none" class="l-ink" style="stroke-width:3"/>`) + line(x0, y0, x0 + 5, y0, "l-sage", 2.4) + line(x1, y1, x1 - 5, y1, "l-sage", 2.4); } },
      { vname: "Ink Blob Feet Omega", concept: "The brush pressed down at the end.", spec: "Omega with filled blob terminals.", draw: () => { const g = (58 * Math.PI) / 180; return arc(32, 31, 13, Math.PI / 2 + g, Math.PI / 2 - g, "l-ink", 3.4) + nodeAt(pt(32, 31, 13, Math.PI / 2 + g)[0] + 1, pt(32, 31, 13, Math.PI / 2 + g)[1], 3.4, "l-fill-ink") + nodeAt(pt(32, 31, 13, Math.PI / 2 - g)[0] - 1, pt(32, 31, 13, Math.PI / 2 - g)[1], 3.4, "l-fill-sage"); } },
      { vname: "Dry Brush Omega", concept: "Ink running low, character high.", spec: "Stroke with flecks breaking away.", draw: () => omegaArc(32, 32, 14, 55, "l-ink", 4, 5) + dot(18, 18, 1.8, "l-fill-dim") + dot(47, 16, 1.4, "l-fill-dim") + dot(50, 44, 1.6, "l-fill-faint") },
      { vname: "Whisk Tail Omega", concept: "Signed with a flick.", spec: "Right foot whisking outward.", draw: () => { const g = (52 * Math.PI) / 180; const [x0, y0] = pt(30, 33, 13, Math.PI / 2 + g); const [x1, y1] = pt(30, 33, 13, Math.PI / 2 - g); return arc(30, 33, 13, Math.PI / 2 + g, Math.PI / 2 - g, "l-ink", 3.4) + line(x0, y0, x0 + 4.5, y0, "l-ink", 3.4) + strokeOnly(`M ${x1} ${y1} q 4 -1 8 -5`, "l-sage", 3); } },
      { vname: "Enso Omega Dot", concept: "One circle of everything, one point of focus.", spec: "Open heavy arc with detached sage dot.", draw: () => arc(32, 32, 15, Math.PI * 0.62, Math.PI * 2.35, "l-ink", 4.8) + nodeAt(40, 46, 3.4, "l-fill-sage") },
      { vname: "Curled Feet Omega", concept: "Feet that curl with satisfaction.", spec: "Terminals curling under gently.", draw: () => { const g = (54 * Math.PI) / 180; const [x0, y0] = pt(32, 31, 13, Math.PI / 2 + g); const [x1, y1] = pt(32, 31, 13, Math.PI / 2 - g); return arc(32, 31, 13, Math.PI / 2 + g, Math.PI / 2 - g, "l-ink", 3.4) + strokeOnly(`M ${x0} ${y0} q 3 3 6 1`, "l-ink", 3) + strokeOnly(`M ${x1} ${y1} q -3 3 -6 1`, "l-ink", 3); } },
      { vname: "Dripping Wet Omega", concept: "Fresh ink announces itself.", spec: "Heavy omega with one drip below foot.", draw: () => omegaArc(32, 29, 13, 55, "l-ink", 4.4, 5) + path("M22 45 C22 49 21 51 20 53 C19 51 18 49 19 46 Z", "l-fill-ink") },
      { vname: "Speed Line Omega", concept: "Moving even at rest.", spec: "Omega with trailing speed streaks.", draw: () => omegaArc(38, 32, 12, 55, "l-ink", 3.4, 4.5) + line(8, 24, 20, 24, "l-dim", 2.4) + line(6, 32, 17, 32, "l-faint", 2.2) + line(9, 40, 21, 40, "l-dim", 2.2) },
      { vname: "Grass Rooted Omega", concept: "Grounded where it stands.", spec: "Grass ticks sprouting at the feet.", draw: () => omegaArc(32, 28, 13, 55, "l-ink", 3.4, 5) + line(20, 48, 20, 42, "l-sage", 2.4) + line(25, 48, 25, 44, "l-dim", 2) + line(39, 48, 39, 43, "l-dim", 2) + line(44, 48, 44, 41, "l-sage", 2.4) },
      { vname: "Feather Barbs Omega", concept: "Lightness along the shoulder.", spec: "Fine barbs off the crown arc.", draw: () => omegaArc(32, 34, 13, 55, "l-ink", 3.2, 4.5) + [[20, 20], [26, 16], [38, 16]].map(([x, y]) => line(x, y, x - 3, y - 5, "l-faint", 1.6)).join("") },
      { vname: "Smoke Rise Omega", concept: "Still warm from the kiln.", spec: "Wisps curling up from the opening.", draw: () => omegaArc(32, 40, 12, 60, "l-ink", 3.2, 4) + strokeOnly("M28 22 Q26 16 30 12 M36 22 Q38 16 35 11", "l-faint", 2) },
      { vname: "Rooted Word Omega", concept: "Feet that became roots.", spec: "Branching root strokes below.", draw: () => omegaArc(32, 27, 13, 55, "l-ink", 3.4, 4.5) + strokeOnly("M26 41 V48 M26 48 q -3 2 -5 5 M26 48 q 3 2 5 4 M38 41 V48 M38 48 q -3 2 -5 5 M38 48 q 3 1 5 4", "l-dim", 2) },
      { vname: "Shoulder Wing Omega", concept: "Ready to take flight.", spec: "Small wing barbs at left shoulder.", draw: () => omegaArc(34, 33, 13, 55, "l-ink", 3.4, 4.5) + leafAt(21, 22, 8, Math.PI * 1.15) + leafAt(21, 22, 6, Math.PI * 1.4, "l-fill-dim") },
      { vname: "Crown Splash Omega", concept: "Droplets from a perfect landing.", spec: "Specks arcing over the letter.", draw: () => omegaArc(32, 37, 12, 55, "l-ink", 3.4, 4.5) + nodeAt(20, 18, 2, "l-fill-dim") + nodeAt(27, 14, 1.7, "l-fill-dim") + nodeAt(37, 13, 2, "l-fill-sage") + nodeAt(45, 17, 1.7, "l-fill-dim") },
      { vname: "Vine Foot Omega", concept: "The right foot grew a leaf.", spec: "Vine terminal with single leaf.", draw: () => { const g = (55 * Math.PI) / 180; const [x0, y0] = pt(29, 33, 13, Math.PI / 2 + g); const [x1, y1] = pt(29, 33, 13, Math.PI / 2 - g); return arc(29, 33, 13, Math.PI / 2 + g, Math.PI / 2 - g, "l-ink", 3.4) + line(x0, y0, x0 + 4, y0, "l-ink", 3.4) + strokeOnly(`M ${x1} ${y1} q 4 1 7 -3`, "l-sage", 3) + leafAt(x1 + 6, y1 - 3, 6, -Math.PI / 4); } },
      { vname: "Charcoal Double Omega", concept: "Sketch first, refine second.", spec: "Rough double-pass stroke offset.", draw: () => omegaArc(30, 31, 13, 55, "l-faint", 3.4, 4) + omegaArc(34, 34, 13, 55, "l-ink", 3.4, 4) },
      { vname: "Ember Speck Omega", concept: "Cooling embers hold their heat.", spec: "Specks drifting from the upper arc.", draw: () => omegaArc(32, 36, 13, 55, "l-ink", 3.4, 4.5) + nodeAt(18, 16, 1.8, "l-fill-sage") + nodeAt(26, 11, 1.4, "l-fill-dim") + nodeAt(34, 9, 1.8, "l-fill-sage") + nodeAt(43, 12, 1.4, "l-fill-faint") },
      { vname: "Brush Omega On Tile", concept: "Your tile pick, hand-brushed.", spec: "Sage squircle with brushed paper omega.", draw: () => tile() + omegaArc(32, 33, 12, 56, "l-paper", 3.6, 5) + dot(45, 45, 2.6, "l-fill-paper") },
      { vname: "Sunrise Cradled Omega", concept: "Every day begins inside it.", spec: "Rising sun disc in the omega opening.", draw: () => omegaArc(32, 30, 13, 62, "l-ink", 3.4, 4.5) + nodeAt(32, 36, 6, "l-fill-sage") + line(20, 46, 44, 46, "l-dim", 2.6) },
      { vname: "Twin Brush Omegas", concept: "Two strokes of the same brush.", spec: "Bold ink omega beside sage companion.", draw: () => omegaArc(23, 33, 10, 56, "l-ink", 3.4, 4) + omegaArc(44, 33, 8.5, 58, "l-sage", 3, 3.5) },
    ],
  },
  {
    id: "agent-creatures",
    name: "Agent Creatures",
    seed: 3208,
    variants: [
      { vname: "Waving Agent Blob", concept: "Hello — I'm working on it.", spec: "Blob with eyes and raised wave stub.", draw: (rng) => blob(30, 38, 16, 0.22, rng) + eyes(29, 34) + strokeOnly("M44 26 Q50 20 54 22", "l-sage", 3.4) },
      { vname: "Sleepy Cell", concept: "Long tasks need naps.", spec: "Blob with closed-eye arcs.", draw: (rng) => blob(32, 36, 16, 0.24, rng) + arc(25, 33, 3.5, Math.PI * 1.15, Math.PI * 1.85, "l-paper", 2) + arc(37, 33, 3.5, Math.PI * 1.15, Math.PI * 1.85, "l-paper", 2) },
      { vname: "Curious Looker", concept: "What's over there?", spec: "Blob with offset curious pupils.", draw: (rng) => blob(30, 32, 17, 0.24, rng) + nodeAt(27, 30, 2.6, "l-fill-paper") + nodeAt(35, 30, 2.6, "l-fill-paper") },
      { vname: "Sprout Head", concept: "Growing something new today.", spec: "Blob sprouting a two-leaf shoot.", draw: (rng) => blob(32, 40, 15, 0.24, rng) + line(32, 25, 32, 19, "l-dim", 2.4) + leafAt(32, 18, 8, -Math.PI / 2 - 0.55) + leafAt(32, 18, 8, -Math.PI / 2 + 0.55, "l-fill-dim") },
      { vname: "Runner Cell", concept: "Deadline mode engaged.", spec: "Leaning blob with speed lines.", draw: (rng) => blob(38, 34, 14, 0.24, rng) + line(10, 24, 20, 24, "l-dim", 2.4) + line(7, 32, 18, 32, "l-faint", 2.2) + line(11, 40, 21, 40, "l-faint", 2) },
      { vname: "Thinker Antenna", concept: "Idea incoming.", spec: "Blob with single antenna and tip dot.", draw: (rng) => blob(32, 38, 15, 0.24, rng) + strokeOnly("M36 22 Q40 16 45 14", "l-dim", 2.4) + spark4(47, 12, 4, 1.6) },
      { vname: "Two Cells Talking", concept: "Agents coordinating politely.", spec: "Facing blobs with speech ticks.", draw: (rng) => blob(20, 36, 11, 0.24, rng) + blob(45, 34, 9, 0.26, rng, "l-fill-sage") + line(33, 22, 31, 17, "l-dim", 2) + line(36, 21, 36, 16, "l-dim", 2) + line(39, 22, 41, 17, "l-dim", 2) },
      { vname: "Littlest Cell", concept: "Small agent, big eyes.", spec: "Tiny blob with large pupils.", draw: (rng) => blob(32, 38, 12, 0.24, rng) + nodeAt(28, 34, 3, "l-fill-paper") + nodeAt(36, 34, 3, "l-fill-paper") },
      { vname: "Determined Cell", concept: "Focused. Unbothered. Shipping.", spec: "Flat-bottom blob with straight gaze.", draw: (rng) => raw(`<path class="l-fill-ink" d="${blobPath(32, 36, 16, 0.16, rng)}"/>`) + rectR(18, 48, 28, 4, 2, "l-fill-dim") + nodeAt(26, 33, 2.6, "l-fill-paper") + nodeAt(38, 33, 2.6, "l-fill-paper") },
      { vname: "Juggler Cell", concept: "Three sessions at once.", spec: "Blob tossing dots overhead.", draw: (rng) => blob(32, 42, 13, 0.22, rng) + nodeAt(20, 18, 3, "l-fill-sage") + nodeAt(32, 10, 3, "l-fill-dim") + nodeAt(44, 18, 3) },
      { vname: "Snail Pace Cell", concept: "Slow is smooth.", spec: "Cell with spiral shell on its back.", draw: (rng) => blob(40, 40, 12, 0.2, rng) + arc(22, 36, 9, -Math.PI * 0.5, Math.PI * 1.15, "l-ink", 3) + arc(22, 36, 4.5, -Math.PI * 0.5, Math.PI * 1.3, "l-sage", 2.6) + line(14, 48, 50, 48, "l-faint", 2) },
      { vname: "Balloon Lift Cell", concept: "Lightening the load.", spec: "Cell carried by a small balloon overhead.", draw: (rng) => blob(30, 42, 12, 0.22, rng) + nodeAt(40, 14, 6, "l-fill-sage") + strokeOnly("M38 21 Q36 28 33 31", "l-dim", 2) },
      { vname: "Corner Peeker", concept: "Checking in from off-screen.", spec: "Cell peeking around a frame edge.", draw: (rng) => rectR(10, 10, 44, 44, 12, "l-ink", 2.6) + blob(22, 40, 11, 0.24, rng) + nodeAt(20, 37, 2.4, "l-fill-paper") + nodeAt(27, 36, 2.4, "l-fill-paper") },
      { vname: "Meditation Hover", concept: "Calm is a feature.", spec: "Closed-eye cell floating over rings.", draw: (rng) => blob(32, 30, 14, 0.22, rng) + arc(26, 29, 3, Math.PI * 1.15, Math.PI * 1.85, "l-paper", 2) + arc(38, 29, 3, Math.PI * 1.15, Math.PI * 1.85, "l-paper", 2) + ell(32, 50, 12, 3.5, "l-sage", 2.4) + ell(32, 50, 19, 5.5, "l-faint", 1.6) },
      { vname: "Excited Burst Cell", concept: "It compiled on the first try.", spec: "Cell with radiating excitement ticks.", draw: (rng) => blob(32, 38, 14, 0.24, rng) + [[-135, "l-dim"], [-90, "l-sage"], [-45, "l-dim"], [180, "l-faint"]].map(([deg, cls]) => { const a = (deg * Math.PI) / 180; const [x1, y1] = pt(32, 34, 18, a); const [x2, y2] = pt(32, 34, 23, a); return line(x1, y1, x2, y2, cls, 2.4); }).join("") },
      { vname: "Reading Time Cell", concept: "Reading the docs first.", spec: "Cell behind open book lines.", draw: (rng) => blob(32, 30, 13, 0.22, rng) + path("M14 44 Q23 39 32 44 Q41 39 50 44 V54 Q41 49 32 54 Q23 49 14 54 Z", "l-fill-sage") + line(32, 44, 32, 53, "l-ink", 2.4) },
      { vname: "Rainy Day Cell", concept: "Working through the weather.", spec: "Cell under umbrella with drops.", draw: (rng) => blob(32, 42, 13, 0.22, rng) + arc(32, 26, 15, Math.PI, TAU, "l-ink", 3) + line(32, 26, 32, 20, "l-ink", 2.6) + line(18, 32, 18, 34, "l-dim", 2) + line(46, 32, 46, 34, "l-dim", 2) },
      { vname: "Star Gazer Cell", concept: "Looking up for once.", spec: "Upturned cell under one bright star.", draw: (rng) => blob(30, 40, 13, 0.22, rng) + nodeAt(27, 37, 2.4, "l-fill-paper") + nodeAt(35, 36, 2.4, "l-fill-paper") + spark4(46, 16, 5.5, 2) },
      { vname: "High Five Duo", concept: "Shipped it — together.", spec: "Two blobs meeting at a raised stub.", draw: (rng) => blob(18, 42, 11, 0.22, rng) + blob(46, 42, 11, 0.24, rng, "l-fill-sage") + strokeOnly("M26 30 L30 22 M38 30 L34 22", "l-dim", 2.8) + nodeAt(32, 18, 3.4, "l-fill-sage") },
      { vname: "Night Shift Owl", concept: "Some of us work best at 2am.", spec: "Round owl-cell with wide night eyes.", draw: (rng) => blob(32, 36, 15, 0.2, rng) + nodeAt(26, 33, 3.4, "l-fill-sage") + nodeAt(38, 33, 3.4, "l-fill-sage") + nodeAt(26, 33, 1.4, "l-fill-ink") + nodeAt(38, 33, 1.4, "l-fill-ink") },
    ],
  },
  {
    id: "vine-networks",
    name: "Vine Networks",
    seed: 3209,
    variants: [
      { vname: "Vine Link Duo", concept: "Connection that grows between us.", spec: "Curved stem linking two node buds.", draw: () => strokeOnly("M12 46 Q32 40 40 24 T54 14", "l-ink", 3) + nodeAt(12, 46, 4) + nodeAt(54, 14, 4, "l-fill-sage") + leafAt(38, 28, 8, -Math.PI / 3) },
      { vname: "Budding Branch Net", concept: "The network is alive.", spec: "Branching stem with bud terminals.", draw: () => strokeOnly("M14 54 V30 M14 42 Q26 36 30 22 M14 48 Q34 44 44 30 M30 22 q 4 -6 10 -8", "l-ink", 2.8) + nodeAt(30, 22, 3.2, "l-fill-sage") + nodeAt(44, 30, 3.2) + nodeAt(40, 15, 2.8, "l-fill-sage") },
      { vname: "Tendril Hex", concept: "A hex grown, not drawn.", spec: "Hex walls with one curling tendril corner.", draw: () => { const v = reg(32, 33, 18, 6); return strokeOnly(`M ${v[1].map(f).join(" ")} L ${v[2].map(f).join(" ")} L ${v[3].map(f).join(" ")} L ${v[4].map(f).join(" ")} L ${v[5].map(f).join(" ")}`, "l-ink", 2.8) + strokeOnly(`M ${v[0].map(f).join(" ")} Q ${f(v[0][0] + 8)} ${f(v[0][1] - 6)} ${f(v[1][0])} ${f(v[1][1])}`, "l-ink", 2.8) + strokeOnly(`M ${v[5].map(f).join(" ")} Q ${f(v[5][0] - 2)} ${f(v[5][1] - 10)} ${f(v[5][0] - 9)} ${f(v[5][1] - 8)} q -5 2 -3 6`, "l-sage", 2.4); } },
      { vname: "Leaf Chain Stem", concept: "Progress you can pick.", spec: "Stem with alternating leaves.", draw: () => strokeOnly("M16 52 C24 40 28 30 32 16", "l-ink", 2.8) + leafAt(25, 36, 8, Math.PI * 0.85) + leafAt(29, 27, 7, Math.PI * 0.15, "l-fill-dim") + nodeAt(32, 14, 3.4, "l-fill-sage") },
      { vname: "Potted Hex Sprout", concept: "Your cell, planted.", spec: "Hex pot with sprouting stem and leaves.", draw: () => path("M18 40 H46 L41 56 H23 Z", "l-fill-ink") + strokeOnly("M32 40 V24", "l-dim", 2.6) + leafAt(32, 24, 9, -Math.PI / 2 - 0.5) + leafAt(32, 24, 9, -Math.PI / 2 + 0.55, "l-fill-dim") },
      { vname: "Reaching Tendrils Pair", concept: "Two ways to say hello.", spec: "Mirror tendrils reaching upward.", draw: () => strokeOnly("M20 54 C16 40 22 30 18 20 q -2 -6 3 -9", "l-ink", 2.8) + strokeOnly("M44 54 C48 40 42 30 46 20 q 2 -6 -3 -9", "l-sage", 2.8) + nodeAt(17, 43, 2.8, "l-fill-dim") + nodeAt(47, 43, 2.8, "l-fill-dim") },
      { vname: "Blossom Node Branch", concept: "Endpoints worth waiting for.", spec: "Curved branch with blossom dots.", draw: () => strokeOnly("M10 50 Q30 44 36 28 T58 16", "l-ink", 2.8) + nodeAt(36, 28, 3.4, "l-fill-sage") + nodeAt(58, 16, 3) + nodeAt(22, 46, 2.4, "l-fill-dim") },
      { vname: "Ivy Corner Frame", concept: "Growth along the edges.", spec: "Corner bracket with trailing ivy and leaves.", draw: () => strokeOnly("M12 26 V12 H26", "l-ink", 3) + strokeOnly("M12 26 Q10 34 16 38 Q22 42 21 48", "l-sage", 2.6) + leafAt(14, 33, 7, Math.PI * 0.95) + leafAt(20, 45, 6, Math.PI * 0.55, "l-fill-dim") },
      { vname: "Root System Down", concept: "Strength starts underground.", spec: "Trunk bar with branching roots.", draw: () => rectR(24, 12, 16, 8, 3, "l-fill-ink") + line(32, 20, 32, 34, "l-dim", 2.6) + strokeOnly("M32 34 Q26 40 20 52 M32 34 Q32 44 32 54 M32 34 Q40 42 46 50", "l-dim", 2) },
      { vname: "Swing Vine Node", concept: "Playful momentum.", spec: "Vine arc with hanging swing node.", draw: () => arc(32, 10, 24, Math.PI * 0.25, Math.PI * 0.75, "l-dim", 2.2) + link(19, 29, 24, 40, 2.4, "l-ink") + nodeAt(24, 43, 3.6, "l-fill-sage") },
      { vname: "Branch Merge Organic", concept: "Even trees know how to merge.", spec: "Two branches growing into one trunk.", draw: () => strokeOnly("M18 12 Q22 26 30 36 M46 12 Q42 26 34 36 M32 36 V56", "l-ink", 3) + leafAt(20, 20, 7, Math.PI * 1.2) + leafAt(44, 20, 7, -Math.PI * 0.2, "l-fill-dim") },
      { vname: "Fern Unfurl Tip", concept: "Opening at its own pace.", spec: "Fiddlehead spiral unfurling leaflets.", draw: () => strokeOnly("M14 52 Q18 20 40 16 Q54 14 54 26 Q54 34 46 34 Q40 34 40 28", "l-ink", 2.8) + [[22, 44], [26, 34], [32, 26]].map(([x, y], i) => leafAt(x, y, 7, -Math.PI / 2 - i * 0.2, i === 1 ? "l-fill-dim" : "l-fill-sage")).join("") },
      { vname: "Cluster On Curves", concept: "Fruit of the network.", spec: "Node cluster hanging from curved stems.", draw: () => strokeOnly("M32 8 Q30 20 22 28 M32 8 Q34 22 40 30 M32 8 Q32 24 32 34", "l-dim", 2) + nodeAt(22, 31, 3.6) + nodeAt(41, 33, 3.6, "l-fill-sage") + nodeAt(32, 37, 3.6) },
      { vname: "Willow Drape Lines", concept: "Shade for everything below.", spec: "Draping strands with tip dots.", draw: () => strokeOnly("M12 10 Q32 18 52 10", "l-ink", 3) + [[18, 13, 16], [26, 16, 26], [38, 16, 30], [46, 13, 40]].map(([x, y, drop]) => strokeOnly(`M ${x} ${y} Q ${x - 2} ${y + drop / 2} ${x - 4} ${y + drop}`, "l-dim", 2) + nodeAt(x - 4, y + drop + 2, 2.4, "l-fill-sage")).join("") },
      { vname: "Pollinator Path", concept: "From bloom to bloom.", draw: () => circ(14, 46, 5.5, "l-fill-ink") + circ(50, 16, 5.5, "l-fill-sage") + `<path d="M 20 42 Q 34 34 44 22" fill="none" class="l-dim" style="stroke-width:2;stroke-dasharray:2 5"/>`, spec: "Dotted flight path between two blooms." },
      { vname: "Circuit Meets Leaf", concept: "Where the trace ends, life begins.", spec: "Trace terminating in a leaf.", draw: () => strokeOnly("M8 40 H26 Q34 40 38 32", "l-ink", 3) + dot(7, 40, 3.4, "l-fill-ink") + leafAt(38, 32, 11, -Math.PI / 3) },
      { vname: "Segmented Stalk", concept: "Growth in honest increments.", spec: "Bamboo-style segments with rings.", draw: () => line(32, 8, 32, 56, "l-ink", 3.4) + [18, 32, 46].map((y) => line(26, y, 38, y, "l-sage", 2.8)).join("") + leafAt(32, 14, 9, -Math.PI / 4) },
      { vname: "Gourd Trio Patch", concept: "Three good things growing.", draw: (rng) => blob(18, 44, 8, 0.2, rng, "l-fill-dim") + blob(38, 48, 9, 0.2, rng) + blob(52, 38, 7, 0.2, rng, "l-fill-sage") + strokeOnly("M38 39 q 2 -6 8 -8 M18 35 q 0 -4 4 -6", "l-dim", 2), spec: "Round gourds with curly tendrils." },
      { vname: "Sunflower Seed Face", concept: "Turn toward the light.", spec: "Petal ring around seeded center.", draw: () => { let s = ""; for (let i = 0; i < 10; i++) { const a = (i / 10) * TAU - Math.PI / 2; const px = 32 + Math.cos(a) * 14, py = 30 + Math.sin(a) * 14; const deg = (a * 180) / Math.PI + 90; s += ell(px, py, 3.4, 7, i % 2 ? "l-sage" : "l-dim", 2, deg); } return s + circ(32, 30, 7, "l-ink", 2.6); } },
      { vname: "Moss Edge Hex", concept: "Soft life on hard structure.", spec: "Hex outline with moss bumps along edges.", draw: () => hexStroke(32, 32, 20) + [[32, 12], [49, 21], [49, 43]].map(([x, y]) => nodeAt(x, y, 3.4, "l-fill-sage")).join("") + nodeAt(15, 21, 2.8, "l-fill-dim") },
    ],
  },
  {
    id: "living-stones",
    name: "Living Stones",
    seed: 3210,
    variants: [
      { vname: "Wobble Stack Trio", concept: "Balance with personality.", spec: "Organic pebbles stacked off-axis.", draw: (rng) => blob(33, 49, 11, 0.2, rng, "l-fill-dim") + blob(30, 32, 9, 0.22, rng) + blob(34, 18, 6.5, 0.24, rng, "l-fill-sage") },
      { vname: "Zen Pair Grass", concept: "Two stones, some grass, peace.", spec: "Pebble pair with grass ticks.", draw: (rng) => blob(24, 42, 12, 0.2, rng) + blob(44, 46, 8, 0.24, rng, "l-fill-sage") + line(14, 54, 14, 47, "l-dim", 2.2) + line(19, 55, 19, 50, "l-dim", 1.8) },
      { vname: "Impossible Overhang", concept: "Held by intention alone.", spec: "Small stone under dramatic overhang.", draw: (rng) => blob(38, 24, 13, 0.18, rng) + nodeAt(30, 43, 5.5, "l-fill-sage") },
      { vname: "Tall Cairn Five", concept: "Five good decisions stacked.", spec: "Tall wobbling cairn of five stones.", draw: (rng) => blob(32, 54, 10, 0.16, rng, "l-fill-dim") + blob(31, 42, 8.5, 0.18, rng) + blob(33, 31, 7, 0.2, rng, "l-fill-sage") + blob(30, 21, 5.5, 0.22, rng) + nodeAt(32, 11, 3.4, "l-fill-sage") },
      { vname: "Natural Stone Arch", concept: "Doorways made by patience.", spec: "Organic arch of stacked stones.", draw: (rng) => blob(16, 46, 10, 0.2, rng, "l-fill-dim") + blob(48, 46, 10, 0.2, rng, "l-fill-dim") + blob(20, 26, 8, 0.22, rng) + blob(44, 26, 8, 0.22, rng) + blob(32, 14, 8, 0.24, rng, "l-fill-sage") },
      { vname: "Pebbles And Ripples", concept: "Landed, still spreading.", spec: "Two pebbles with ripple rings.", draw: () => nodeAt(28, 40, 8, "l-fill-ink") + nodeAt(44, 34, 5.5, "l-fill-dim") + circ(28, 44, 13, "l-faint", 1.6) + circ(44, 39, 9, "l-faint", 1.4) },
      { vname: "Flat Disc Stack", concept: "Sliced river stones.", spec: "Three organic discs stacked tall.", draw: (rng) => blob(32, 48, 12, 0.16, rng, "l-fill-dim") + blob(31, 32, 11, 0.18, rng, "l-fill-sage") + blob(33, 17, 9, 0.18, rng) },
      { vname: "Stone With Sprout", concept: "Life finds a way around anything.", spec: "Boulder with sprout curling beside.", draw: (rng) => blob(28, 38, 15, 0.22, rng) + strokeOnly("M44 52 Q48 42 45 32 Q44 26 48 22", "l-sage", 2.6) + leafAt(47, 27, 7, -Math.PI / 3) },
      { vname: "Caught Lean Tower", concept: "The second before it settles.", spec: "Dramatic leaning stone on base.", draw: (rng) => blob(24, 46, 11, 0.18, rng, "l-fill-dim") + raw(`<g transform="rotate(-14 40 30)">`) + blob(40, 28, 12, 0.2, rng) + raw(`</g>`) },
      { vname: "Round Stone Gate", concept: "A circle you can walk through.", spec: "Bold round gate with center void.", draw: () => circ(32, 32, 18, "l-ink", 6) + nodeAt(32, 52, 2.6, "l-fill-dim") },
      { vname: "Raked Sand Stones", concept: "Order drawn around the wild.", spec: "Stones on rippled sand lines.", draw: () => sineStroke(8, 56, 20, 3, 3, "l-faint", 2) + sineStroke(8, 56, 32, 3, 3, "l-faint", 2) + nodeAt(26, 44, 7, "l-fill-ink") + nodeAt(44, 47, 4.5, "l-fill-sage"),
      },
      { vname: "Cliff Edge Boulder", concept: "Right at the edge, still fine.", spec: "Boulder balanced at a ledge corner.", draw: (rng) => poly([[8, 54], [40, 54], [40, 40], [8, 40]], "l-fill-ink") + blob(48, 34, 9, 0.2, rng, "l-fill-sage") + nodeAt(46, 47, 2.6, "l-fill-dim") },
      { vname: "Three Standing Stones", concept: "A very old meeting room.", spec: "Three standing stones in a row.", draw: (rng) => blob(16, 42, 7.5, 0.16, rng, "l-fill-dim") + blob(32, 38, 9, 0.18, rng) + blob(48, 43, 7, 0.16, rng, "l-fill-sage") },
      { vname: "Moss Cap Pair", concept: "Soft life crowns hard stone.", spec: "Two stones wearing moss caps.", draw: (rng) => blob(24, 42, 11, 0.2, rng) + blob(44, 44, 9, 0.22, rng, "l-fill-dim") + blob(23, 32, 6, 0.24, rng, "l-fill-sage") + blob(43, 35, 5, 0.24, rng, "l-fill-sage") },
      { vname: "Skipping Stone Arc", concept: "Momentum makes it dance.", spec: "Skipping trajectory with splash rings.", draw: () => strokeOnly("M8 40 Q22 18 36 30 T58 24", "l-ink", 3) + nodeAt(36, 30, 3.4, "l-fill-sage") + circ(22, 34, 5, "l-faint", 1.6), spec: "Skipping trajectory with splash rings." },
      { vname: "Balanced Mobile Stones", concept: "Equilibrium as an art piece.", spec: "Hanging balance of two stones on a beam.", draw: (rng) => line(32, 8, 32, 18, "l-dim", 2.2) + line(18, 22, 46, 22, "l-ink", 3) + line(18, 22, 18, 30, "l-dim", 2) + line(46, 22, 46, 30, "l-dim", 2) + blob(18, 35, 6.5, 0.2, rng, "l-fill-sage") + blob(46, 35, 6.5, 0.2, rng), spec: "Hanging balance of two stones." },
      { vname: "River Flow Stones", concept: "Water writes around what stays.", spec: "Flow lines parting around stones.", draw: () => nodeAt(26, 32, 7, "l-fill-ink") + nodeAt(44, 36, 5, "l-fill-dim") + strokeOnly("M8 22 Q24 20 40 26 Q48 29 56 27", "l-sage", 2.4) + strokeOnly("M8 44 Q22 46 36 43 Q46 41 56 44", "l-sage", 2.4) },
      { vname: "Stone Lantern Silhouette", concept: "A quiet light keeper.", spec: "Stacked lantern shapes with glow dot.", draw: () => rectR(24, 48, 16, 6, 2, "l-fill-ink") + rectR(21, 36, 22, 10, 3, "l-fill-dim") + hexFill(32, 27, 9, "l-fill-sage") + nodeAt(32, 27, 3, "l-fill-paper") },
      { vname: "Pebble Mouse Friend", concept: "Even rocks can be cute here.", spec: "Pebbles forming a tiny mouse friend.", draw: (rng) => blob(34, 44, 11, 0.18, rng) + circ(22, 36, 6.5, "l-fill-ink") + circ(18, 28, 3, "l-fill-ink") + strokeOnly("M45 44 q 8 -2 10 -8", "l-dim", 2.2) },
      { vname: "Small Gratitude Stack", concept: "Small stack, full heart.", spec: "Mini trio with rising sun tick.", draw: (rng) => blob(32, 48, 8.5, 0.2, rng, "l-fill-dim") + blob(32, 35, 7, 0.22, rng) + blob(32, 24, 5.5, 0.24, rng, "l-fill-sage") + arc(32, 8, 6, Math.PI * 1.15, Math.PI * 1.85, "l-faint", 2) },
    ],
  },
  {
    id: "mitosis-stories",
    name: "Mitosis Stories",
    seed: 3211,
    variants: [
      { vname: "The First Pinch", concept: "Every team was once one person.", spec: "Circle pinching into two lobes.", draw: () => strokeOnly("M32 12 C44 12 50 20 48 32 C50 44 44 52 32 52 C20 52 14 44 16 32 C14 20 20 12 32 12 Z", "l-ink", 3) + strokeOnly("M32 13 C28 22 28 42 32 51", "l-dim", 2.2) },
      { vname: "Almost Two", concept: "So close to becoming.", spec: "Two circles with thin bridge.", draw: () => circ(21, 32, 11, "l-ink", 3) + circ(43, 32, 11, "l-sage", 3) + line(31, 32, 33, 32, "l-ink", 2.6) },
      { vname: "Just Separated", concept: "Independent, recently.", spec: "Separated circles with fading ghost bridge.", draw: () => circ(18, 32, 10.5, "l-ink", 3) + circ(46, 32, 10.5, "l-sage", 3) + line(30, 32, 34, 32, "l-faint", 2) },
      { vname: "Growth Stages Row", concept: "Then, now, next.", spec: "Three-stage fission sequence.", draw: () => dot(12, 32, 5, "l-fill-ink") + circ(29, 32, 7.5, "l-ink", 2.8) + line(36.5, 32, 37.5, 32, "l-dim", 2) + nodeAt(45, 26, 4, "l-fill-sage") + nodeAt(53, 38, 4, "l-fill-sage") },
      { vname: "Directional Division", concept: "Growing apart, on purpose.", spec: "Split pair with outward arrows.", draw: () => circ(22, 32, 9, "l-ink", 2.8) + circ(42, 32, 9, "l-sage", 2.8) + poly([[8, 27], [1, 32], [8, 37]], "l-fill-ink") + poly([[56, 27], [63, 32], [56, 37]], "l-fill-sage") },
      { vname: "Shared Center Venn", concept: "What we still hold in common.", spec: "Overlapping circles with shared core fill.", draw: () => `<circle cx="25" cy="32" r="12" fill="var(--accent)" opacity="0.55" stroke="none"/>` + `<circle cx="39" cy="32" r="12" fill="var(--accent)" opacity="0.55" stroke="none"/>` + strokeOnly("M25 20 A12 12 0 0 1 25 44 M39 20 A12 12 0 0 0 39 44", "l-ink", 2.6) },
      { vname: "Uneven Daughters", concept: "Not all splits are equal.", spec: "Large daughter beside small daughter.", draw: () => circ(24, 34, 13, "l-ink", 3) + nodeAt(47, 28, 7, "l-fill-sage") + link(34, 30, 41, 29, 2, "l-faint") },
      { vname: "Sequential Budding", concept: "One after another after another.", spec: "Diagonal budding chain shrinking.", draw: (rng) => blob(16, 46, 10, 0.2, rng) + blob(32, 34, 8, 0.22, rng, "l-fill-sage") + blob(44, 24, 6, 0.24, rng, "l-fill-dim") },
      { vname: "Snap Moment Sparks", concept: "Release takes energy.", spec: "Separated pair with spark ticks between.", draw: () => circ(19, 32, 9.5, "l-ink", 3) + circ(45, 32, 9.5, "l-sage", 3) + [[-40], [0], [40]].map(([deg]) => { const a = (deg * Math.PI) / 180; return line(pt(32, 32, 4, a)[0], pt(32, 32, 4, a)[1], pt(32, 32, 8, a)[0], pt(32, 32, 8, a)[1], "l-dim", 2); }).join("") },
      { vname: "New Vela Pair", concept: "Apart, but still circling each other.", spec: "Separated cells on shared ellipse.", draw: () => ell(32, 32, 20, 9, "l-faint", 1.8) + nodeAt(13, 32, 6, "l-fill-ink") + nodeAt(51, 32, 5, "l-fill-sage") },
      { vname: "Snapped Membrane Curls", concept: "Breaks leave their marks.", spec: "Bridge remnant curled at both ends.", draw: () => circ(17, 32, 10, "l-ink", 3) + circ(47, 32, 10, "l-sage", 3) + strokeOnly("M27 32 q -3 -4 -6 -2 M37 32 q 3 -4 6 -2", "l-dim", 2.2) },
      { vname: "Population Bloom Grid", concept: "From one to many, quickly.", spec: "1-2-4 mini sequence.", draw: () => nodeAt(10, 32, 4.4, "l-fill-ink") + nodeAt(26, 26, 3.4) + nodeAt(26, 38, 3.4) + nodeAt(46, 22, 3) + nodeAt(56, 30, 3, "l-fill-sage") + nodeAt(46, 40, 3) + nodeAt(58, 42, 3) },
      { vname: "Tug Of Division", concept: "Pulling toward new things.", spec: "Cells pulling bridge taut between them.", draw: () => circ(16, 32, 9.5, "l-ink", 3) + circ(48, 32, 9.5, "l-sage", 3) + strokeOnly("M25 32 Q32 26 39 32", "l-dim", 2.4) },
      { vname: "Gentle Drift Trails", concept: "Separation at a kind pace.", spec: "Drifting cells with soft trails.", draw: () => nodeAt(14, 30, 6, "l-fill-ink") + nodeAt(50, 36, 5.5, "l-fill-sage") + strokeOnly("M20 31 Q26 30 30 30", "l-faint", 1.8) + strokeOnly("M44 35 Q38 34 34 34", "l-faint", 1.8) },
      { vname: "Reunion Approach", concept: "Some splits run backwards.", spec: "Two cells moving together again.", draw: () => circ(13, 32, 9, "l-ink", 3) + circ(51, 32, 9, "l-sage", 3) + poly([[25, 27], [30, 32], [25, 37]], "l-fill-ink") + poly([[39, 27], [34, 32], [39, 37]], "l-fill-sage") },
      { vname: "Cores Go With Halves", concept: "Everyone takes something.", spec: "Daughters each carrying a nucleus.", draw: () => circ(20, 32, 11, "l-ink", 3) + circ(44, 32, 11, "l-sage", 3) + nodeAt(20, 32, 3, "l-fill-ink") + nodeAt(44, 32, 3, "l-fill-ink") },
      { vname: "Yeast Chain Asymmetric", concept: "Generations of different sizes.", spec: "Three-node budding chain diagonal.", draw: (rng) => blob(14, 48, 9, 0.2, rng) + blob(31, 38, 7, 0.22, rng, "l-fill-sage") + blob(43, 27, 5.5, 0.24, rng, "l-fill-dim") + link(21, 44, 26, 41, 1.8, "l-faint") + link(37, 34, 39, 31, 1.8, "l-faint") },
      { vname: "Timeline Dots Division", concept: "The whole story in four beats.", spec: "Dotted timeline with stage dots growing.", draw: () => line(8, 32, 56, 32, "l-faint", 2) + nodeAt(12, 32, 3, "l-fill-ink") + nodeAt(26, 32, 3.6, "l-fill-dim") + nodeAt(40, 32, 4.2, "l-fill-sage") + nodeAt(54, 32, 3, "l-fill-ink") },
      { vname: "Binary Fission Bold", concept: "Half and half, cleanly.", spec: "Bold two-tone circle splitting.", draw: () => path("M32 10 A22 22 0 0 0 32 54 Z", "l-fill-ink") + path("M32 10 A22 22 0 0 1 32 54 Z", "l-fill-sage") + rectR(30.5, 10, 3, 44, 1.5, "l-fill-paper") },
      { vname: "Fresh Starts Pair", concept: "New beginnings, twice over.", spec: "Small fresh cells with dawn tick above.", draw: () => nodeAt(22, 42, 8, "l-fill-ink") + nodeAt(44, 44, 7, "l-fill-sage") + arc(33, 14, 7, Math.PI * 1.1, Math.PI * 1.9, "l-dim", 2.4) },
    ],
  },
  {
    id: "wave-layers",
    name: "Wave Layers",
    seed: 3212,
    variants: [
      { vname: "Three Crest Bands", concept: "The classic, given motion.", spec: "Layered wave bands with curled tips.", draw: () => strokeOnly("M8 20 Q16 12 24 18 T40 16", "l-faint", 2.4) + strokeOnly("M8 34 Q17 25 27 32 T46 30", "l-sage", 3) + strokeOnly("M8 48 Q19 37 31 45 T54 42", "l-ink", 3.6) },
      { vname: "Deep Swell Birds", concept: "Big water, small sky.", spec: "Slow swell with two bird ticks.", draw: () => sineStroke(6, 58, 42, 7, 1.1, "l-ink", 3.6) + strokeOnly("M22 18 q4 -5 8 0 M34 14 q4 -5 8 0", "l-dim", 2.2) },
      { vname: "Sound Ribbons Rising", concept: "Louder as it goes.", spec: "Four ribbon waves growing amplitude.", draw: () => [0, 1, 2, 3].map((i) => sineStroke(10 + i * 2, 54 - i, 16 + i * 10, 2 + i * 2.4, 2, i === 3 ? "l-ink" : i === 2 ? "l-sage" : "l-dim", 2.6)).join("") },
      { vname: "Ocean Depth Bands", concept: "Deeper and darker below.", spec: "Tone-shaded stacked wave bands.", draw: () => path("M6 26 Q21 16 32 24 T58 24 V30 H6 Z", "l-fill-faint") + path("M6 36 Q21 28 32 35 T58 33 V44 H6 Z", "l-fill-sage") + path("M6 47 Q21 41 32 46 T58 44 V56 H6 Z", "l-fill-ink") },
      { vname: "Crest Curl Spiral", concept: "One perfect breaking wave.", spec: "Wave body with spiral curl tip.", draw: () => strokeOnly("M8 44 Q22 16 40 20 Q52 23 50 32 Q49 39 42 38 Q37 37 38 32", "l-ink", 3.4) },
      { vname: "Tide Pools Spill", concept: "Each level finds the next.", spec: "Stepped pools with spill lines.", draw: () => [[10, 22, 20], [26, 32, 16], [44, 42, 12]].map(([x, y, wd], i) => line(x, y, x + wd, y, ["l-faint", "l-sage", "l-ink"][i], 3.4) + line(x + wd - 3, y + 2, x + wd + 2, y + 9, "l-dim", 1.8)).join("") },
      { vname: "Small Wave Big Wave", concept: "Start small; end enormous.", spec: "Scale contrast left to right.", draw: () => sineStroke(8, 24, 40, 3, 2.4, "l-dim", 2.4) + sineStroke(28, 56, 36, 9, 1.2, "l-ink", 3.8) },
      { vname: "Harmonic Interference", concept: "Two signals making a third.", draw: () => sineStroke(8, 56, 22, 5, 2.4, "l-faint", 1.8) + sineStroke(8, 56, 22, 5, 1.4, "l-dim", 1.8), spec: "Two faint waves over their combined result." },
      { vname: "Paper Ocean Stripes", concept: "Cut-paper sea, full bleed.", spec: "Full-width wavy stripes alternating tones.", draw: () => path("M6 18 Q21 8 32 18 T58 18 V26 H6 Z", "l-fill-faint") + path("M6 30 Q21 20 32 30 T58 30 V40 H6 Z", "l-fill-sage") + path("M6 43 Q21 33 32 43 T58 43 V56 H6 Z", "l-fill-ink") },
      { vname: "Meander River Banks", concept: "Wandering, but going somewhere.", spec: "Meander line between bank rules.", draw: () => line(8, 14, 56, 14, "l-faint", 1.8) + strokeOnly("M10 34 Q18 22 26 34 T42 34 T58 34", "l-ink", 3.4) + line(8, 50, 56, 50, "l-faint", 1.8) },
      { vname: "Surf Foam Dots", concept: "Where the wave finishes work.", spec: "Wave band dissolving into foam dots.", draw: () => sineStroke(6, 40, 32, 5, 1.4, "l-ink", 3.4) + nodeAt(44, 28, 3, "l-fill-dim") + nodeAt(50, 34, 2.4, "l-fill-sage") + nodeAt(55, 29, 1.8, "l-fill-faint") },
      { vname: "Wind Through Grass", concept: "Weather you can feel.", spec: "Wind lines bending grass strokes.", draw: () => strokeOnly("M10 16 Q30 10 50 16", "l-faint", 2) + strokeOnly("M14 26 Q32 20 48 26", "l-dim", 2) + [[20, 54], [30, 55], [40, 54]].map(([x, y], i) => strokeOnly(`M ${x} ${y} Q ${x + (i % 2 ? 5 : -5)} ${y - 10} ${x + (i % 2 ? 10 : -9)} ${y - 15}`, "l-sage", 2.8)).join("") },
      { vname: "Spectrum Fan Arcs", concept: "Every frequency at once.", spec: "Broadcast arcs fanning from source.", draw: () => nodeAt(14, 44, 4.4, "l-fill-ink") + arc(14, 44, 11, -Math.PI * 0.5, -Math.PI * 0.05, "l-dim", 2.2) + arc(14, 44, 18, -Math.PI * 0.5, -Math.PI * 0.03, "l-sage", 2.6) + arc(14, 44, 25, -Math.PI * 0.5, 0, "l-faint", 2) },
      { vname: "Seismic Strata Jagged", concept: "The record of every shock.", spec: "Jagged layered strata bands.", draw: () => strokeOnly("M6 20 L16 26 L26 18 L38 27 L48 20 L58 25", "l-faint", 2) + strokeOnly("M6 34 L14 40 L28 32 L40 41 L52 33 L58 38", "l-sage", 2.6) + strokeOnly("M6 48 L18 55 L30 46 L42 55 L58 48", "l-ink", 3.2) },
      { vname: "Flag In Real Wind", concept: "Cloth that behaves like cloth.", spec: "Flag with flowing double curves.", draw: () => line(16, 8, 16, 54, "l-ink", 3.2) + strokeOnly("M16 12 C30 8 38 16 52 12 C44 20 46 24 52 26 C38 22 30 28 16 26", "l-sage", 3) },
      { vname: "Rising Smoke Curls", concept: "Signals drifting home.", spec: "Stacked smoke curls rising.", draw: () => strokeOnly("M26 54 Q22 46 28 40 Q34 34 30 28", "l-faint", 2.2) + strokeOnly("M34 54 Q38 44 32 36 Q28 30 34 24", "l-dim", 2.6) + strokeOnly("M42 52 Q46 44 40 38", "l-faint", 2) },
      { vname: "Desert Dune Curves", concept: "Wind-sculpted patience.", spec: "Overlapping dune curves.", draw: () => strokeOnly("M6 40 Q20 26 34 38 T58 36", "l-sage", 3) + strokeOnly("M6 50 Q22 38 40 48 Q50 53 58 50", "l-ink", 3.4) },
      { vname: "Rain Rings Pond", concept: "Every drop starts a story.", spec: "Rain ticks above expanding rings.", draw: () => line(20, 10, 18, 16, "l-dim", 2.2) + line(34, 8, 32, 14, "l-dim", 2.2) + line(46, 12, 44, 18, "l-dim", 2.2) + circ(30, 34, 6, "l-sage", 2.2) + circ(30, 34, 12, "l-faint", 1.8) + circ(30, 34, 18, "l-faint", 1.4) },
      { vname: "Aurora Ribbon Columns", concept: "Light dancing straight up.", spec: "Vertical aurora ribbons with stars.", draw: () => strokeOnly("M18 52 Q14 34 20 14", "l-sage", 3.4) + strokeOnly("M32 54 Q28 32 34 12", "l-dim", 3) + strokeOnly("M46 52 Q50 36 44 16", "l-faint", 2.6) + nodeAt(52, 12, 2, "l-fill-sage") },
      { vname: "Single Melody Line", concept: "One tune, hummed quietly.", spec: "Elegant melody wave with note dots.", draw: () => strokeOnly("M8 36 C18 22 26 22 32 32 C38 42 46 42 56 28", "l-ink", 3.2) + nodeAt(32, 32, 2.8, "l-fill-sage") + line(46, 30, 46, 20, "l-dim", 2) + nodeAt(44, 19, 2.2, "l-fill-dim") },
    ],
  },
  {
    id: "flowing-constellations",
    name: "Flowing Constellations",
    seed: 3213,
    variants: [
      { vname: "Arc Link Constellation", concept: "Straight lines are a choice; curves are too.", spec: "Nodes linked by soft arcs.", draw: () => nodeAt(12, 44, 4) + nodeAt(34, 16, 3.6, "l-fill-sage") + nodeAt(54, 40, 4) + strokeOnly("M12 44 Q20 24 34 16", "l-dim", 2.2) + strokeOnly("M34 16 Q48 22 54 40", "l-dim", 2.2) },
      { vname: "Comet Chain Nodes", concept: "Every node mid-motion.", spec: "Three nodes each trailing small tails.", draw: () => [[14, 42], [32, 30], [50, 18]].map(([x, y], i) => nodeAt(x, y, 3.6 - i * 0.4, i === 2 ? "l-fill-sage" : "l-fill-ink") + line(x - 7, y + 5, x - 3, y + 2.5, "l-faint", 1.8)).join("") },
      { vname: "Dance Of Five", concept: "Five agents swirling one task.", spec: "Nodes on curved paths around center.", draw: () => nodeAt(32, 32, 3.4, "l-fill-sage") + [[-0.4, 15], [1.8, 19], [3.4, 13]].map(([a, r]) => strokeOnly(`M ${pt(32, 32, r + 5, a - 0.9).map(f).join(" ")} Q ${pt(32, 32, r, a).map(f).join(" ")} ${pt(32, 32, r - 4, a + 0.7).map(f).join(" ")}`, "l-faint", 1.6) + nodeAt(pt(32, 32, r, a)[0], pt(32, 32, r, a)[1], 3)).join("") },
      { vname: "Migration Route Dots", concept: "The long journey has stops.", spec: "Dotted route with stopover nodes.", draw: () => `<path d="M10 48 Q26 40 30 28 Q36 12 52 14" fill="none" class="l-faint" style="stroke-width:2;stroke-dasharray:2 5"/>` + nodeAt(10, 48, 3.4) + nodeAt(30, 28, 3.4, "l-fill-sage") + nodeAt(52, 14, 3.8) },
      { vname: "Spiral Gather Points", concept: "Everything converging, gently.", spec: "Dots spiraling into the gather point.", draw: () => nodeAt(34, 33, 4.4, "l-fill-sage") + [[46, 12, 1], [56, 30, 2], [48, 50, 3], [26, 55, 4], [10, 40, 5], [10, 18, 6]].map(([x, y, k]) => strokeOnly(`M ${f(x)} ${f(y)} Q ${f((x + 34) / 2)} ${f((y + 33) / 2)} ${f(31)} ${f(35 - k)}`, "l-faint", 1.4) + dot(x, y, 2.6)).join("") },
      { vname: "Curved Triad Arrows", concept: "A cycle that never stops.", spec: "Triad with curved flow arrows.", draw: () => { const P = reg(32, 34, 15, 3); let s = ""; P.forEach(([x, y], i) => { const [nx, ny] = P[(i + 1) % 3]; const mx = (x + nx) / 2, my = (y + ny) / 2; s += strokeOnly(`M ${x} ${y} Q ${mx + 6} ${my} ${nx} ${ny}`, "l-dim", 2); }); P.forEach(([x, y], i) => (s += nodeAt(x, y, 3.4, i === 0 ? "l-fill-sage" : "l-fill-ink"))); return s; } },
      { vname: "Pendulum Swing Arcs", concept: "Back and forth is still progress.", spec: "Swing arcs with bob at the end.", draw: () => arc(32, 8, 26, Math.PI * 0.68, Math.PI * 1.32, "l-faint", 1.8) + line(32, 8, pt(32, 8, 26, Math.PI * 1.25)[0], pt(32, 8, 26, Math.PI * 1.25)[1], "l-dim", 2.2) + nodeAt(pt(32, 8, 26, Math.PI * 1.25)[0], pt(32, 8, 26, Math.PI * 1.25)[1] + 2, 4.4, "l-fill-sage") },
      { vname: "Race Track Staggered", concept: "Same track, different moments.", spec: "Oval track with staggered runner nodes.", draw: () => rectR(14, 20, 36, 24, 12, "l-ink", 2.8) + nodeAt(26, 20, 3.6, "l-fill-sage") + nodeAt(44, 44, 3.6) },
      { vname: "Gravity Well Bend", concept: "Mass changes every path near it.", spec: "Streamlines bending around heavy core.", draw: () => nodeAt(32, 32, 6.5, "l-fill-ink") + strokeOnly("M8 18 Q32 24 56 18", "l-dim", 2.2) + strokeOnly("M8 46 Q32 40 56 46", "l-dim", 2.2) + strokeOnly("M8 32 H20 M44 32 H56", "l-sage", 2.6) },
      { vname: "Wind Map Streams", concept: "Flow that reads the room.", spec: "Streamlines flowing past an obstacle.", draw: () => circ(32, 32, 7, "l-fill-sage") + strokeOnly("M8 22 Q22 20 32 23 Q44 27 56 22", "l-dim", 2) + strokeOnly("M8 42 Q22 44 32 41 Q44 37 56 42", "l-dim", 2) },
      { vname: "Vela Rendezvous Curves", concept: "Two routes, one meeting.", spec: "Curved approaches converging at node.", draw: () => strokeOnly("M8 12 Q36 16 44 30", "l-ink", 2.8) + strokeOnly("M56 50 Q46 40 44 30", "l-sage", 2.8) + nodeAt(45, 32, 4.4, "l-fill-ink") },
      { vname: "Silk Thread Beads", concept: "One thread through everything.", spec: "Single elegant curve threading beads.", draw: () => sineStroke(8, 56, 32, 8, 1.1, "l-dim", 2.2) + [[16, 38], [32, 24], [48, 38]].map(([x, y], i) => nodeAt(x, y, 3.4, i === 1 ? "l-fill-sage" : "l-fill-ink")).join("") },
      { vname: "River Delta Split", concept: "One source, many destinations.", spec: "Path branching organically into delta.", draw: () => strokeOnly("M30 8 V22 M30 22 Q22 32 14 44 M30 22 V44 M30 22 Q38 34 50 48", "l-ink", 3) + nodeAt(30, 8, 3.2, "l-fill-sage") + nodeAt(14, 46, 2.6) + nodeAt(30, 47, 2.6) + nodeAt(51, 50, 2.6, "l-fill-sage") },
      { vname: "Murmuration Band", concept: "Many small agents, one shape.", spec: "Dot swarm flowing in a band.", draw: () => [[12, 40], [17, 35], [23, 31], [29, 29], [36, 28], [43, 30], [49, 34], [54, 39], [47, 42], [39, 40], [31, 38], [24, 42], [40, 35], [34, 33], [44, 37]].map(([x, y]) => dot(x, y, 1.8, "l-fill-ink")).join("") },
      { vname: "Signature Swirl", concept: "One confident gesture.", spec: "Bold swirl stroke ending in dot.", draw: () => strokeOnly("M10 46 Q20 14 40 16 Q54 18 50 30 Q46 40 36 36 Q28 33 34 26", "l-ink", 3.6) + nodeAt(35, 25, 3, "l-fill-sage") },
      { vname: "Chosen Curved Path", concept: "Between two routes, life picks the curve.", spec: "Dashed straight option, chosen sage curve.", draw: () => raw(`<line x1="8" y1="46" x2="56" y2="14" stroke="none" class="l-faint" style="stroke-width:1.8;stroke-dasharray:3 4"/>`) + strokeOnly("M8 46 Q30 44 56 14", "l-sage", 3.2) + nodeAt(8, 46, 3.2) + nodeAt(56, 14, 3.6, "l-fill-ink") },
      { vname: "Twin Comet Meet", concept: "Two arrivals, one moment.", spec: "Curving comets meeting at shared node.", draw: () => strokeOnly("M10 14 Q28 18 40 28", "l-ink", 2.8) + strokeOnly("M54 50 Q44 42 42 34", "l-sage", 2.8) + nodeAt(41, 30, 4.4, "l-fill-ink") + nodeAt(9, 12, 2.6, "l-fill-dim") },
      { vname: "Whisk Focus Lines", concept: "Quick attention on one thing.", spec: "Whisk strokes converging on focal dot.", draw: () => nodeAt(40, 32, 5, "l-fill-sage") + [[10, 18], [8, 32], [11, 46]].map(([x, y], i) => line(x, y, x + 16, y + (i - 1) * 2, "l-dim", 2.4)).join("") },
      { vname: "Threaded Node Ribbon", concept: "Nodes strung like lanterns.", spec: "Ribbon weaving through four beads.", draw: () => sineStroke(8, 56, 32, 9, 1.3, "l-dim", 2.2) + [[16, 40], [30, 22], [42, 38], [52, 20]].map(([x, y], i) => nodeAt(x, y, 3.4, i % 2 ? "l-fill-sage" : "l-fill-ink")).join("") },
      { vname: "Signal Chaser Curve", concept: "Chasing a signal around the bend.", spec: "Curved chase path ending at source dot.", draw: () => strokeOnly("M8 50 Q26 46 34 32 Q40 20 52 18", "l-ink", 2.8) + nodeAt(54, 16, 4.4, "l-fill-sage") + nodeAt(10, 49, 2.6, "l-fill-dim") },
    ],
  },
  {
    id: "wild-hexes",
    name: "Wild Hexes",
    seed: 3214,
    variants: [
      { vname: "Hand Drawn Hex", concept: "Your hex, drawn by hand.", spec: "Wobbly ink hexagon outline.", draw: (rng) => raw(`<path d="${blobPath(32, 32, 18, 0.06, rng, 6)}" fill="none" class="l-ink" style="stroke-width:3.2"/>`) },
      { vname: "Hex With Sprout Corner", concept: "Structure growing something.", spec: "Hex with leaf sprouting from top vertex.", draw: () => hexStroke(32, 36, 17) + strokeOnly("M32 19 V12", "l-dim", 2.4) + leafAt(32, 11, 8, -Math.PI / 2 - 0.5) },
      { vname: "Blob Hex Hybrid", concept: "Half geometry, half life.", spec: "Hex melting into organic blob side.", draw: () => raw(`<path class="l-fill-ink" d="M 32 12 L 51 22 L 51 44 L 32 53 L 24 48 C 16 44 18 36 20 32 C 16 26 18 16 26 15 Z"/>`) + nodeAt(38, 32, 4, "l-fill-paper") },
      { vname: "Double Wobble Hexes", concept: "Two hands drew two hexes.", spec: "Overlapping wobbly hex pair.", draw: (rng) => raw(`<path d="${blobPath(25, 32, 14, 0.08, rng, 6)}" fill="none" class="l-ink" style="stroke-width:3"/>`) + raw(`<path d="${blobPath(40, 32, 14, 0.08, rng, 6)}" fill="none" class="l-sage" style="stroke-width:3"/>`) },
      { vname: "Hex With Grass Base", concept: "Settled long enough to grow grass.", spec: "Hex sitting in grass ticks.", draw: () => hexStroke(32, 28, 16) + line(18, 50, 18, 44, "l-sage", 2.4) + line(24, 52, 24, 47, "l-dim", 2) + line(32, 53, 32, 48, "l-sage", 2.4) + line(40, 52, 40, 47, "l-dim", 2) + line(47, 50, 47, 45, "l-sage", 2.4) },
      { vname: "Honeycomb Wobble Patch", concept: "Real hives aren't perfect either.", spec: "Three wobbly cells clustered.", draw: (rng) => raw(`<path d="${blobPath(19, 22, 10, 0.07, rng, 6)}" fill="none" class="l-ink" style="stroke-width:2.8"/>`) + raw(`<path d="${blobPath(45, 22, 10, 0.07, rng, 6)}" fill="none" class="l-ink" style="stroke-width:2.8"/>`) + blob(32, 43, 10, 0.08, rng, "l-fill-sage", 6) },
      { vname: "Ink Wash Hex", concept: "Brush and geometry together.", spec: "Hex with wash-filled lower half.", draw: () => hexStroke(32, 32, 19) + `<clipPath id="iwh"><path d="${reg(32, 32, 19, 6).map((p) => p.map(f).join(",")).join(" L")} Z"/></clipPath><rect x="10" y="32" width="44" height="22" clip-path="url(#iwh)" fill="var(--accent)" opacity="0.5" stroke="none"/>` },
      { vname: "Hex Trail Wanderer", concept: "The cell moved — you can see where.", spec: "Hex with dotted wander path leaving it.", draw: () => hexStroke(20, 40, 12) + `<path d="M30 30 Q38 22 40 16 T52 12" fill="none" class="l-dim" style="stroke-width:2;stroke-dasharray:2 4"/>` + nodeAt(53, 11, 3.4, "l-fill-sage") },
      { vname: "Breathing Hex Frames", concept: "Expanding and contracting.", spec: "Concentric wobble frames pulsing outward.", draw: (rng) => raw(`<path d="${blobPath(32, 32, 10, 0.05, rng, 6)}" fill="none" class="l-ink" style="stroke-width:2.6"/>`) + raw(`<path d="${blobPath(32, 32, 16, 0.05, rng, 6)}" fill="none" class="l-dim" style="stroke-width:2.2"/>`) + raw(`<path d="${blobPath(32, 32, 22, 0.05, rng, 6)}" fill="none" class="l-faint" style="stroke-width:1.8"/>`) },
      { vname: "Tilted Hex Stack Loose", concept: "Stacked by someone in a hurry.", spec: "Loose offset hex stack.", draw: (rng) => raw(`<path d="${blobPath(28, 46, 13, 0.05, rng, 6)}" fill="none" class="l-ink" style="stroke-width:2.8"/>`) + raw(`<g transform="rotate(-8 34 26)"><path d="${blobPath(34, 26, 11, 0.05, rng, 6)}" fill="none" class="l-sage" style="stroke-width:2.8"/></g>`) },
      { vname: "Hex And Friend Circle", concept: "Different shapes, same team.", spec: "Wobbly hex beside wobbly circle.", draw: (rng) => raw(`<path d="${blobPath(23, 32, 13, 0.06, rng, 6)}" fill="none" class="l-ink" style="stroke-width:3"/>`) + raw(`<path d="${blobPath(43, 32, 11, 0.3, rng)}" fill="none" class="l-sage" style="stroke-width:3"/>`) },
      { vname: "Open Hex Sketch Passes", concept: "Still sketching, always.", spec: "Multi-pass sketch hex lines.", draw: (rng) => raw(`<path d="${blobPath(32, 32, 18, 0.05, rng, 6)}" fill="none" class="l-ink" style="stroke-width:2.8"/>`) + raw(`<path d="${blobPath(32, 32, 18, 0.09, rng, 6)}" fill="none" class="l-faint" style="stroke-width:1.8"/>`) },
      { vname: "Wild Hex Scatter Trio", concept: "Cells wherever they land.", spec: "Three scattered wobbly cells varied size.", draw: (rng) => raw(`<path d="${blobPath(15, 15, 8, 0.07, rng, 6)}" fill="none" class="l-faint" style="stroke-width:2.2"/>`) + blob(40, 30, 11, 0.07, rng, "l-fill-ink", 6) + raw(`<path d="${blobPath(48, 52, 7.5, 0.07, rng, 6)}" fill="none" class="l-sage" style="stroke-width:2.6"/>`) },
      { vname: "Hex Sprout Inside Out", concept: "Life pushing through structure.", spec: "Leaf breaking out of hex wall.", draw: () => hexStroke(30, 34, 16) + leafAt(44, 22, 9, -Math.PI / 3, "l-fill-sage") + strokeOnly("M40 27 Q43 24 45 20", "l-dim", 2.2) },
      { vname: "Rain On Hex Roof", concept: "Shelter doing its job.", spec: "Pointy-top hex as roof with rain drops.", draw: () => poly([[12, 30], [32, 14], [52, 30]], "l-ink", 3.2) + line(20, 38, 20, 43, "l-dim", 2.2) + line(32, 40, 32, 45, "l-dim", 2.2) + line(44, 38, 44, 43, "l-dim", 2.2) },
      { vname: "Hive Drip Honey", concept: "The good stuff coming out.", spec: "Hex cell dripping from base vertex.", draw: () => hexStroke(32, 24, 16) + strokeOnly("M32 40 V48", "l-sage", 3) + nodeAt(32, 52, 3.4, "l-fill-sage") },
      { vname: "Sketchy Tri Trace Hex", concept: "Tri Trace Hex, but alive now.", spec: "Wobbly hex redrawn with center pad and ticks.", draw: (rng) => raw(`<path d="${blobPath(32, 32, 19, 0.05, rng, 6)}" fill="none" class="l-ink" style="stroke-width:3"/>`) + nodeAt(32, 32, 4.4, "l-fill-sage") + line(32, 13, 32, 20, "l-dim", 2) + line(17, 40, 23, 36, "l-dim", 2) + line(47, 40, 41, 36, "l-dim", 2) },
      { vname: "Loose Hive Court Ring", concept: "Court Hive grown wild.", spec: "Ring of six wobbling cells.", draw: (rng) => reg(32, 32, 17, 6).map(([x, y], i) => raw(`<path d="${blobPath(x, y, 8, 0.07, rng, 6)}" fill="none" class="${i === 0 ? "l-sage" : "l-ink"}" style="stroke-width:2.4"/>`)).join("") },
      { vname: "Hex Kite String", concept: "Structure, but make it playful.", spec: "Hex kite flying on a curved string.", draw: () => hexStroke(40, 20, 11, "l-ink", 3) + strokeOnly("M32 30 Q24 42 14 50", "l-dim", 2.2) + line(40, 31, 40, 36, "l-dim", 2) },
      { vname: "Sleepy Hex Moon Night", concept: "Even hexes rest.", spec: "Hex with closed-eye arcs under crescent.", draw: () => hexStroke(32, 38, 16) + arc(26, 36, 3, Math.PI * 1.15, Math.PI * 1.85, "l-paper", 2) + arc(38, 36, 3, Math.PI * 1.15, Math.PI * 1.85, "l-paper", 2) + raw(`<path d="M46 10 A8 8 0 1 0 54 20 A10 10 0 0 1 46 10 Z" class="l-fill-sage" stroke="none"/>`) },
    ],
  },
  {
    id: "living-tiles",
    name: "Living Tiles",
    seed: 3215,
    variants: [
      { vname: "Leaf Cut Tile", concept: "An icon that grows.", spec: "Sage tile with leaf void.", draw: () => tile() + leafAt(32, 40, 16, -Math.PI / 2, "l-fill-paper") },
      { vname: "Blob Window Tile", concept: "Organic light through the icon.", spec: "Ink tile with blob window.", draw: (rng) => tile("l-fill-ink") + blob(32, 32, 12, 0.24, rng, "l-fill-paper") },
      { vname: "Wave Cut Tile II", concept: "Water through everything.", spec: "Wave channel across sage tile.", draw: () => tile() + strokeOnly("M10 32 Q20 22 30 32 T50 32", "l-paper", 5) },
      { vname: "Sprout Tile", concept: "New growth on the home screen.", spec: "Tile with stem-and-leaves cutout.", draw: () => tile("l-fill-ink") + strokeOnly("M32 48 V34", "l-paper", 3.4) + leafAt(32, 33, 10, -Math.PI / 2 - 0.5, "l-fill-paper") + leafAt(32, 33, 10, -Math.PI / 2 + 0.5, "l-fill-paper") },
      { vname: "Pulse Tile Living", concept: "Vitals on the icon face.", spec: "Pulse channel on sage tile.", draw: () => tile() + strokeOnly("M12 32 H22 L27 23 L32 41 L37 32 H52", "l-paper", 3.6) },
      { vname: "Moon Phase Tile", concept: "Night shift icon.", spec: "Crescent bite on ink tile.", draw: () => tile("l-fill-ink") + bgCircle(42, 24, 12) },
      { vname: "Cell Division Tile", concept: "Multiplying already.", spec: "Two-lobe void on sage tile.", draw: () => tile() + circ(24, 32, 9, "l-fill-paper") + circ(41, 32, 9, "l-fill-paper") + rectR(29, 28, 7, 8, 2, "l-fill-sage") },
      { vname: "Comet Trail Tile", concept: "Fast by design.", spec: "Head dot with tapered trail on ink tile.", draw: () => tile("l-fill-ink") + ribbon([14, 44], [28, 34], [38, 26], [48, 18], 2, 6.5, "l-fill-paper") },
      { vname: "Vela Tile Live", concept: "Electrons on the icon.", spec: "Ellipse vela with electron on tile.", draw: () => tile() + ell(32, 32, 16, 8, "l-paper", 3) + nodeAt(46, 27, 3.6, "l-fill-paper") + nodeAt(32, 32, 4, "l-fill-paper") },
      { vname: "Omega Brush Tile", concept: "The word, brushed onto the tile.", spec: "Brushed paper omega with flick tail.", draw: () => tile() + omegaArc(30, 32, 11, 56, "l-paper", 3.4, 4) + strokeOnly("M42 44 q 5 -2 8 -6", "l-sage", 3) },
      { vname: "Mountain Dawn Tile", concept: "First light on the range.", spec: "Peak silhouette over dawn disc.", draw: () => tile() + nodeAt(40, 22, 8, "l-fill-paper") + paperFill("M12 52 L26 28 L34 40 L42 26 L52 52 Z") },
      { vname: "Rain Tile Calm", concept: "Weather inside the icon.", spec: "Cloud void with rain slots.", draw: (rng) => tile("l-fill-ink") + blob(32, 24, 10, 0.2, rng, "l-fill-paper") + rectR(22, 40, 4, 9, 2, "l-fill-paper") + rectR(31, 42, 4, 8, 2, "l-fill-paper") + rectR(40, 40, 4, 7, 2, "l-fill-paper") },
      { vname: "Balanced Stones Tile", concept: "Calm stacked into the icon.", spec: "Stone stack silhouette on tile.", draw: (rng) => tile("l-fill-ink") + blob(32, 48, 10, 0.16, rng, "l-fill-paper") + blob(32, 33, 8, 0.18, rng, "l-fill-paper") + blob(32, 20, 6, 0.2, rng, "l-fill-paper") },
      { vname: "Kite Play Tile", concept: "Weekend mode exists.", spec: "Kite and string cutout on tile.", draw: () => tile() + poly([[40, 12], [52, 24], [40, 36], [28, 24]], "l-fill-paper") + strokeOnly("M36 34 Q28 44 18 50", "l-paper", 2.8) },
      { vname: "Fish Swim Tile", concept: "Something alive down there.", spec: "Simple fish silhouette void.", draw: () => tile() + paperFill("M14 32 Q26 22 38 30 Q44 26 50 24 Q47 32 50 40 Q44 38 38 34 Q26 42 14 32 Z") },
      { vname: "Egg Nest Tile", concept: "Care, built in.", spec: "Nest cradle holding one egg.", draw: () => tile() + strokeOnly("M14 38 Q32 52 50 38", "l-paper", 3.6) + ell(32, 34, 8, 10, "l-paper", 3.4) },
      { vname: "Sun Rise Tile Warm", concept: "Morning, every morning.", spec: "Sun half-disc over horizon slot.", draw: () => tile("l-fill-ink") + path("M18 36 A14 14 0 0 1 46 36 Z", "l-fill-paper") + rectR(12, 38, 40, 5, 2.5, "l-fill-paper") },
      { vname: "Whisk Tail Tile", concept: "Motion captured at the edge.", spec: "Swoosh exiting corner of tile.", draw: () => tile() + ribbon([12, 46], [28, 40], [40, 28], [50, 14], 6, 2.2, "l-fill-paper") },
      { vname: "Heartbeat Grid Tile", concept: "Four pulses, one system.", spec: "Mini pulse grid on dark tile.", draw: () => tile("l-fill-ink") + [18, 32, 46].map((y, i) => strokeOnly(`M${12 + (i % 2) * 4} ${y} h6 l3 -5 l4 9 l3 -4 h${8 - (i % 2) * 2}`, "l-paper", 2.4)).join("") },
      { vname: "Full Bloom Tile", concept: "Ready to open the app store.", spec: "Flower bloom cutout centered.", draw: () => tile() + reg(32, 32, 12, 6).map(([x, y]) => ell((32 + x) / 2, (32 + y) / 2, 4, 7, "l-paper", 3, (Math.atan2(y - 32, x - 32) * 180) / Math.PI + 90)).join("") + nodeAt(32, 32, 4.4, "l-fill-paper") },
    ],
  },
];

let NEXT = 1;
const LOGOS = [];
const usedNames = new Set();
FAMILIES.forEach((fam) => {
  fam.variants.forEach((v) => {
    const idNum = NEXT++;
    let name = v.vname;
    let guard = 0;
    while (usedNames.has(name) && guard++ < 50) name = `${v.vname} ${guard}`;
    usedNames.add(name);
    const rng = mulberry32(fam.seed + idNum * 977);
    LOGOS.push({
      id: String(idNum).padStart(3, "0"),
      name,
      family: fam.name,
      familyId: fam.id,
      concept: typeof v.concept === "function" ? v.concept(rng) : v.concept,
      spec: typeof v.spec === "function" ? v.spec(rng) : v.spec,
      svg: wrap(v.draw(rng)),
    });
  });
});

if (LOGOS.length !== 300) throw new Error(`Expected 300 logos, got ${LOGOS.length}`);
const ids = new Set(LOGOS.map((l) => l.id));
if (ids.size !== 300) throw new Error(`Duplicate ids: ${ids.size}`);
const svgSet = new Set(LOGOS.map((l) => l.svg));
if (svgSet.size !== 300) throw new Error(`Duplicate SVGs: ${300 - svgSet.size}`);
const famCounts = FAMILIES.map((fam) => ({ id: fam.id, name: fam.name, count: fam.variants.length }));
famCounts.forEach((fc) => {
  if (fc.count !== 20) throw new Error(`Family ${fc.id} has ${fc.count} variants`);
});

const header = `// AUTO-GENERATED by generate-logos.mjs — do not edit by hand.
// 300 DISTINCT living Vela logo marks, iteration 4.
// Fields: id, name, family, familyId, concept, spec, svg.
const LOGOS = ${JSON.stringify(LOGOS, null, 0)};
const FAMILIES = ${JSON.stringify(famCounts, null, 0)};
`;

const runtime = readFileSync(join(__dirname, "logos-runtime.js"), "utf8");
writeFileSync(
  join(__dirname, "logos.js"),
  header + "\n// ===== gallery runtime (appended) =====\n" + runtime,
  "utf8"
);
console.log(`Wrote logo data: ${LOGOS.length} entries across ${FAMILIES.length} families.`);
