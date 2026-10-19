// Hand-refined Vela logo finalists, distilled from the iteration-4
// shortlist (atomic-velas family plus infinity ribbon and drift pair).
// Each mark is authored on a 96-grid with fixed optical weights:
// primary vela 5, secondary vela 4-4.5, core radius 8-10, electrons 5.5-6.
// Two renderings per mark: `svg` for page backgrounds (theme-aware classes)
// and `svgOnColor` for app-icon tiles (all-paper strokes and fills).

const S = (d, cls, w) => `<path d="${d}" fill="none" class="${cls}"${w ? ` style="stroke-width:${w}"` : ""}/>`;
const C = (cx, cy, r, cls, extra = "") => `<circle cx="${cx}" cy="${cy}" r="${r}" class="${cls}"${extra}/>`;
const ELL = (cx, cy, rx, ry, deg, cls, w) =>
  `<ellipse cx="${cx}" cy="${cy}" rx="${rx}" ry="${ry}" transform="rotate(${deg} ${cx} ${cy})" fill="none" class="${cls}" style="stroke-width:${w}"/>`;
const STAR = (cx, cy, ro, ri, cls) => {
  const pts = [];
  for (let i = 0; i < 8; i++) {
    const r = i % 2 ? ri : ro;
    const a = -Math.PI / 2 + (i * Math.PI) / 4;
    pts.push(`${(cx + r * Math.cos(a)).toFixed(2)},${(cy + r * Math.sin(a)).toFixed(2)}`);
  }
  return `<polygon points="${pts.join(" ")}" class="${cls}"/>`;
};

const onEll = (cx, cy, rx, ry, deg, a) => {
  const rr = (deg * Math.PI) / 180;
  const x = rx * Math.cos(a);
  const y = ry * Math.sin(a);
  return [cx + x * Math.cos(rr) - y * Math.sin(rr), cy + x * Math.sin(rr) + y * Math.cos(rr)];
};

function velastandard() {
  const e1 = onEll(48, 48, 36, 15, -24, -0.95);
  return (
    ELL(48, 48, 36, 15, 24, "c-sage", 4.5) +
    ELL(48, 48, 36, 15, -24, "c-ink", 5) +
    C(48, 48, 9, "f-ink") +
    C(e1[0].toFixed(1), e1[1].toFixed(1), 6, "f-sage")
  );
}

function electronTrio() {
  const specs = [
    { deg: 0, cls: "c-ink", w: 4.5, a: -0.6 },
    { deg: 60, cls: "c-sage", w: 4, a: 2.3 },
    { deg: -60, cls: "c-dim", w: 3.5, a: 0.9 },
  ];
  let s = "";
  for (const sp of specs) {
    s += ELL(48, 48, 37, 14.5, sp.deg, sp.cls, sp.w);
    const p = onEll(48, 48, 37, 14.5, sp.deg, sp.a);
    s += C(p[0].toFixed(1), p[1].toFixed(1), sp.cls === "c-sage" ? 5.5 : 4.5, sp.cls === "c-dim" ? "f-dim" : sp.cls === "c-sage" ? "f-sage" : "f-ink");
  }
  return s + C(48, 48, 8, "f-ink");
}

function twoMoons() {
  const m1 = onEll(46, 52, 30, 11, -18, -2.25);
  const m2 = onEll(46, 52, 30, 11, -18, 0.65);
  return (
    ELL(46, 52, 30, 11, -18, "c-sage", 5) +
    C(m1[0].toFixed(1), m1[1].toFixed(1), 6.5, "f-sage") +
    C(46, 52, 13, "f-ink") +
    C(m2[0].toFixed(1), m2[1].toFixed(1), 4.5, "f-dim") +
    C(80, 24, 2.5, "f-dim")
  );
}

function sparkNucleus() {
  return (
    ELL(48, 48, 34, 13.5, 55, "c-sage", 4.5) +
    ELL(48, 48, 34, 13.5, -55, "c-ink", 4.5) +
    STAR(48, 48, 11.5, 4.4, "f-sage")
  );
}

function polarEquatorial() {
  const top = onEll(48, 48, 14, 33, 0, -Math.PI / 2);
  const right = onEll(48, 48, 33, 14, 0, 0.15);
  return (
    ELL(48, 48, 14, 33, 0, "c-sage", 4.5) +
    ELL(48, 48, 33, 14, 0, "c-ink", 5) +
    C(48, 48, 7.5, "f-ink") +
    C(top[0].toFixed(1), top[1].toFixed(1), 5.5, "f-sage") +
    C(right[0].toFixed(1), right[1].toFixed(1), 4.5, "f-dim")
  );
}

function infinityVela() {
  return (
    S(
      "M48 48 C38 32 20 34 20 48 C20 62 38 64 48 48 C58 32 76 34 76 48 C76 62 58 64 48 48 Z",
      "c-ink",
      5
    ) +
    C(34, 36.5, 6, "f-sage") +
    C(62, 59.5, 4.5, "f-dim") +
    C(48, 48, 6.5, "f-ink")
  );
}

function driftPair() {
  return (
    strokeTrail("M18 70 Q27 66 32 60", "c-dim", 3) +
    strokeTrail("M12 62 Q21 59 26 54", "c-dim", 2.5) +
    C(42, 56, 12, "f-ink") +
    C(45, 53, 4.5, "f-paper") +
    strokeTrail("M56 46 Q52 43 50 40", "c-dim", 2.2) +
    C(66, 36, 8, "f-sage")
  );
}
const strokeTrail = (d, cls, w) => S(d, cls, w);

function lagrangePoints() {
  const tri = [
    [72, 26],
    [60, 74],
    [22, 64],
  ];
  const dashed = (pts) =>
    `<path d="M ${pts.map((p) => p.join(" ")).join(" L ")} Z" fill="none" class="c-dim" style="stroke-width:1.8;stroke-dasharray:3 5"/>`;
  return (
    dashed(tri) +
    node(72, 26, 3.2, "f-dim") +
    node(60, 74, 3.2, "f-dim") +
    node(22, 64, 3.2, "f-dim") +
    C(32, 40, 10, "f-ink") +
    C(62, 60, 6.5, "f-sage")
  );
}
const node = (x, y, r, cls) => C(x, y, r, cls);

export const MARKS = [
  { id: "01", name: "Vela Standard", basedOn: "Tilted System Live · Tilted System Fan", svg: velastandard(), note: "The flagship: one bold vela carrying an electron, one quiet counter-vela, solid core." },
  { id: "02", name: "Electron Trio", basedOn: "Electron Trio Angled", svg: electronTrio(), note: "Three planes, three electrons, one nucleus — maximum system energy." },
  { id: "03", name: "Two Moons", basedOn: "Planet With Two Moons", svg: twoMoons(), note: "A world with company: ring, near moon, far moon, distant star." },
  { id: "04", name: "Spark Nucleus", basedOn: "Spark Nucleus Atom", svg: sparkNucleus(), note: "Purest mark: two velas, no electrons, intelligence at the center." },
  { id: "05", name: "Polar & Equatorial", basedOn: "Polar And Equatorial", svg: polarEquatorial(), note: "Vertical meets horizontal — full coverage around the core." },
  { id: "06", name: "Infinity Vela", basedOn: "Infinity Ribbon Tied", svg: infinityVela(), note: "Endless capacity tied at the waist; electrons riding both lobes." },
  { id: "07", name: "Drift Pair", basedOn: "Directional Drift Cell", svg: driftPair(), note: "Coordinator and agent drifting together, trails showing the way they came." },
  { id: "08", name: "Lagrange Points", basedOn: "Lagrange Triangle", svg: lagrangePoints(), note: "Two bodies and the stable points between them — orchestration as physics." },
];
