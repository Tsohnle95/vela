// Generates 50 landing concepts (001-050) distilled from shortlisted 001
// "Atelier Split": bare Vela mark (no container chip), fixed hierarchy
// (mark -> title -> one line -> one action -> calm recents), no feature
// lists, no drop hints. Emits concepts/*.html, landing-pages.css,
// landing-pages.js (legacy, removed) and index.html gallery.

import { writeFileSync, readdirSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));

// ---- shared builders --------------------------------------------------------

const mark = (size = 32) => `<svg class="mark-svg" viewBox="0 0 96 96" width="${size}" height="${size}" aria-hidden="true"><g fill="none" stroke-linecap="round"><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(24 48 48)" stroke="#9eb4a1" stroke-width="4.5"/><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(-24 48 48)" stroke="#617a68" stroke-width="5"/><circle cx="48" cy="48" r="9" fill="#46584b"/><circle cx="62.1" cy="28.3" r="6" fill="#9eb4a1"/></g></svg>`;

const RECENTS = [
  { name: "Refine sessions panel", project: "vela", when: "just now", live: true },
  { name: "Provider usage cleanup", project: "vela", when: "18m ago", live: true },
  { name: "Update runtime docs", project: "vela", when: "yesterday" },
  { name: "Polish daily notes", project: "atlas-notes", when: "2h ago" },
  { name: "Improve quick search", project: "atlas-notes", when: "Tue" },
];

const rowsList = ({ count = 5, ghost = false } = {}) =>
  `<ul class="rows${ghost ? " rows-ghost" : ""}">` +
  RECENTS.slice(0, count)
    .map(
      (r) => `<li class="row${r.live ? " is-live" : ""}"><span class="row-dot"></span><span class="row-name">${r.name}</span><span class="row-meta">${r.project} · ${r.when}</span></li>`
    )
    .join("") +
  `</ul>`;

const monoList = () =>
  `<ul class="mono-list">` +
  RECENTS.map(
    (r, i) =>
      `<li><span class="mono-i">${String(i + 1).padStart(2, "0")}</span><span class="mono-name">${r.name}</span><span class="mono-lead"></span><span class="mono-when">${r.when}</span></li>`
  ).join("") +
  `</ul>`;

const numList = () =>
  `<ul class="num-list">` +
  RECENTS.map(
    (r, i) =>
      `<li><span class="num-i">${String(i + 1).padStart(2, "0")}</span><span class="num-name">${r.name}</span><span class="num-when">${r.when}</span></li>`
  ).join("") +
  `</ul>`;

const cta = (primary = "Open a folder", ghost = null) =>
  `<div class="cta-row"><button class="btn btn-primary" type="button" tabindex="-1">${primary}</button>${ghost ? `<button class="btn btn-ghost" type="button" tabindex="-1">${ghost}</button>` : ""}</div>`;

const SUB = "Every agent. One surface.";

// ---- concept definitions ----------------------------------------------------

const CONCEPTS = [
  {
    id: "001", name: "Monolith", fam: "type",
    css: `.c-001 .mock{display:flex;flex-direction:column;align-items:center;text-align:center;padding:64px 40px 0;min-height:520px}
.c-001 .mark-svg{width:40px;height:40px;margin-bottom:26px}
.c-001 .title{font-family:var(--serif);font-size:64px;font-weight:500;line-height:1;letter-spacing:-0.02em;margin:0 0 12px}
.c-001 .sub{margin:0 0 30px;color:var(--text-dim);font-size:13.5px}
.c-001 .rows{position:absolute;left:0;right:0;bottom:0;margin:0;padding:14px 40px 16px;list-style:none;display:flex;gap:26px;justify-content:center;border-top:1px solid var(--border-subtle);background:var(--bg)}
.c-001 .rows li{display:flex;align-items:center;gap:7px;font-size:11px;color:var(--text-faint)}
.c-001 .row-dot{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.c-001 .rows .is-live .row-dot{background:var(--accent)}
.c-001 .rows .row-meta{display:none}
.c-001 .btn{margin-bottom:34px}`,
    body: `${mark(40)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({})}</div>`,
  },
  {
    id: "002", name: "Side Rail", fam: "split",
    css: `.c-002 .mock{display:flex;min-height:520px}
.c-002 .rail{width:76px;background:var(--accent);display:flex;flex-direction:column;align-items:center;padding:26px 0;gap:22px;flex:none}
.c-002 .rail .mark-svg{width:34px;height:34px}
.c-002 .rail .rail-line{width:1px;flex:1;background:rgba(253,250,243,0.25)}
.c-002 .rail .rail-vword{writing-mode:vertical-rl;color:rgba(253,250,243,0.7);font-size:10px;letter-spacing:0.34em;text-transform:uppercase}
.c-002 .main{flex:1;padding:52px 44px;display:flex;flex-direction:column}
.c-002 .title{font-family:var(--serif);font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-002 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-002 .rows{margin-top:auto;padding-top:18px;border-top:1px solid var(--border-subtle)}`,
    body: `<div class="rail">${mark(34)}<span class="rail-line"></span><span class="rail-vword">Vela</span></div><div class="main"><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder", "Open a file…")}<div class="rows-wrap">${rowsList({})}</div></div>`,
  },
  {
    id: "003", name: "Shared Baseline", fam: "split",
    css: `.c-003 .mock{display:flex;min-height:520px;padding:56px 48px;gap:48px}
.c-003 .left{flex:1.1;display:flex;flex-direction:column;justify-content:flex-start}
.c-003 .lockup{display:flex;align-items:center;gap:16px;margin-bottom:18px}
.c-003 .lockup .mark-svg{width:44px;height:44px}
.c-003 .title{font-family:var(--serif);font-size:52px;font-weight:500;line-height:0.95;letter-spacing:-0.02em;margin:0}
.c-003 .sub{margin:14px 0 26px;color:var(--text-dim);font-size:13px;max-width:30ch}
.c-003 .right{width:270px;flex:none;border-left:1px solid var(--border-subtle);padding-left:32px}
.c-003 .kicker{font-size:9.5px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);margin:0 0 14px}`,
    body: `<div class="left"><div class="lockup">${mark(44)}<h1 class="title">Vela</h1></div><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="right"><p class="kicker">Recent</p><div class="rows-wrap">${rowsList({})}</div></div>`,
  },
  {
    id: "004", name: "Watermark", fam: "atmos",
    css: `.c-004 .mock{position:relative;min-height:520px;display:grid;place-items:center;text-align:center;padding:60px 40px;overflow:hidden}
.c-004 .watermark{position:absolute;inset:0;display:grid;place-items:center;pointer-events:none}
.c-004 .watermark svg{width:340px;height:340px;opacity:0.055}
.c-004 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.c-004 .mark-svg{width:34px;height:34px;margin-bottom:22px}
.c-004 .title{font-family:var(--serif);font-size:56px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-004 .sub{margin:0 0 28px;color:var(--text-dim);font-size:13px}
.c-004 .rows{margin-top:44px;width:100%;max-width:360px}`,
    body: `<div class="watermark">${mark(340)}</div><div class="inner">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ ghost: true })}</div></div>`,
  },
  {
    id: "005", name: "Quiet Cross", fam: "frame",
    css: `.c-005 .mock{position:relative;min-height:520px;padding:0}
.c-005 .vline{position:absolute;left:50%;top:0;bottom:0;width:1px;background:var(--border-subtle)}
.c-005 .hline{position:absolute;top:50%;left:0;right:0;height:1px;background:var(--border-subtle)}
.c-005 .cell{position:absolute;display:flex;flex-direction:column;gap:10px}
.c-005 .cell-tl{top:44px;left:44px}
.c-005 .cell-tr{top:44px;right:44px;text-align:right;color:var(--text-faint);font-size:11px}
.c-005 .cell-bl{bottom:44px;left:44px;max-width:250px}
.c-005 .cell-br{bottom:44px;right:44px;width:250px}
.c-005 .mark-svg{width:30px;height:30px}
.c-005 .title{font-family:var(--serif);font-size:40px;font-weight:500;margin:0;letter-spacing:-0.02em}
.c-005 .sub{margin:0;color:var(--text-dim);font-size:12.5px}
.c-005 .center{position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);display:flex;flex-direction:column;align-items:center;gap:14px}`,
    body: `<span class="vline"></span><span class="hline"></span><div class="cell cell-tl">${mark(30)}</div><div class="cell cell-tr">v0.1 — macOS</div><div class="center">${cta("Open a folder")}</div><div class="cell cell-bl"><h1 class="title">Vela</h1><p class="sub">${SUB}</p></div><div class="cell cell-br"><div class="rows-wrap">${rowsList({ count: 4 })}</div></div>`,
  },
  {
    id: "006", name: "Margin Notes", fam: "list",
    css: `.c-006 .mock{display:flex;min-height:520px;padding:56px 48px;gap:56px}
.c-006 .left{flex:1;max-width:300px}
.c-006 .mark-svg{width:36px;height:36px;margin-bottom:24px}
.c-006 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-006 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-006 .right{width:240px;flex:none;border-left:1px solid var(--border-subtle);padding-left:30px;display:flex;flex-direction:column;justify-content:center}
.c-006 .mrow{padding:11px 0;border-bottom:1px solid var(--border-subtle);display:flex;flex-direction:column;gap:3px}
.c-006 .mrow:last-child{border-bottom:0}
.c-006 .mrow-name{font-family:var(--serif);font-style:italic;font-size:15.5px}
.c-006 .mrow-when{font-size:10.5px;color:var(--text-faint);letter-spacing:0.04em}
.c-006 .mrow.is-live .mrow-name{color:var(--accent)}`,
    body: `<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="right">${RECENTS.map((r) => `<div class="mrow${r.live ? " is-live" : ""}"><span class="mrow-name">${r.name}</span><span class="mrow-when">${r.project} — ${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "007", name: "Aperture", fam: "mark",
    css: `.c-007 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;text-align:center;padding:52px 40px}
.c-007 .aperture{width:132px;height:132px;border:1px solid var(--border-strong);border-radius:50%;display:grid;place-items:center;margin-bottom:30px}
.c-007 .aperture .mark-svg{width:56px;height:56px}
.c-007 .title{font-family:var(--serif);font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-007 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-007 .rows{width:100%;max-width:380px;margin-top:38px;border-top:1px solid var(--border-subtle);padding-top:8px}`,
    body: `<div class="aperture">${mark(56)}</div><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder", "Open a file…")}<div class="rows-wrap">${rowsList({})}</div>`,
  },
  {
    id: "008", name: "Duet", fam: "split",
    css: `.c-008 .mock{display:flex;min-height:520px}
.c-008 .half{flex:1;padding:56px 44px;display:flex;flex-direction:column}
.c-008 .half-a{justify-content:center;border-right:1px solid var(--border-subtle)}
.c-008 .half-a .mark-svg{width:40px;height:40px;margin-bottom:24px}
.c-008 .half-a .title{font-family:var(--serif);font-size:50px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-008 .half-a .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:26ch}
.c-008 .half-b{justify-content:center;background:var(--bg)}
.c-008 .kicker{font-size:9.5px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);margin:0 0 16px}`,
    body: `<div class="half half-a">${mark(40)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="half half-b"><p class="kicker">Recent sessions</p><div class="rows-wrap">${rowsList({})}</div></div>`,
  },
  {
    id: "009", name: "Underline", fam: "type",
    css: `.c-009 .mock{min-height:520px;padding:64px 56px;display:flex;flex-direction:column}
.c-009 .mark-svg{width:30px;height:30px;margin-bottom:30px}
.c-009 .title{font-family:var(--serif);font-size:68px;font-weight:500;line-height:1;margin:0;letter-spacing:-0.025em}
.c-009 .rule{width:180px;height:5px;background:var(--accent);margin:18px 0 20px;border-radius:2px}
.c-009 .sub{margin:0 0 28px;color:var(--text-dim);font-size:13px;max-width:44ch}
.c-009 .rows{margin-top:auto;border-top:1px solid var(--border-subtle);padding-top:10px}`,
    body: `${mark(30)}<h1 class="title">Vela</h1><div class="rule"></div><p class="sub">${SUB}</p>${cta("Open a folder", "Open a file…")}<div class="rows-wrap">${rowsList({})}</div>`,
  },
  {
    id: "010", name: "Dot Directory", fam: "list",
    css: `.c-010 .mock{display:flex;min-height:520px;padding:56px 48px;gap:60px}
.c-010 .left{width:250px;flex:none;display:flex;flex-direction:column;justify-content:center}
.c-010 .mark-svg{width:34px;height:34px;margin-bottom:22px}
.c-010 .title{font-family:var(--serif);font-size:44px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-010 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-010 .right{flex:1;display:flex;flex-direction:column;justify-content:center}
.c-010 .drow{display:flex;align-items:center;gap:16px;padding:15px 4px;border-bottom:1px solid var(--border-subtle)}
.c-010 .drow:last-child{border-bottom:0}
.c-010 .ddot{width:11px;height:11px;border-radius:50%;border:1.5px solid var(--border-strong);flex:none}
.c-010 .drow.is-live .ddot{background:var(--accent);border-color:var(--accent)}
.c-010 .dname{font-size:14px;font-weight:500}
.c-010 .dwhen{margin-left:auto;font-size:11px;color:var(--text-faint)}`,
    body: `<div class="left">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="right">${RECENTS.map((r) => `<div class="drow${r.live ? " is-live" : ""}"><span class="ddot"></span><span class="dname">${r.name}</span><span class="dwhen">${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "011", name: "Ledger", fam: "table",
    css: `.c-011 .mock{min-height:520px;padding:52px 48px 0;display:flex;flex-direction:column}
.c-011 .mark-svg{width:32px;height:32px;margin-bottom:20px}
.c-011 .title{font-family:var(--serif);font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-011 .sub{margin:0 0 22px;color:var(--text-dim);font-size:13px}
.c-011 .mono-wrap{margin-top:auto;border-top:1px solid var(--border-subtle);padding:10px 0 18px}`,
    body: `${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="mono-wrap">${monoList()}</div>`,
  },
  {
    id: "012", name: "Wide Gutter", fam: "split",
    css: `.c-012 .mock{display:flex;min-height:520px}
.c-012 .left{flex:1.4;padding:56px 48px;display:flex;flex-direction:column;justify-content:center}
.c-012 .mark-svg{width:36px;height:36px;margin-bottom:24px}
.c-012 .title{font-family:var(--serif);font-size:54px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-012 .sub{margin:0 0 28px;color:var(--text-dim);font-size:13px;max-width:30ch}
.c-012 .right{flex:1;background:var(--bg-inset);padding:56px 36px;display:flex;flex-direction:column;justify-content:center}
.c-012 .kicker{font-size:9.5px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);margin:0 0 14px}
.c-012 .blk{padding:13px 0;border-bottom:1px solid var(--border-subtle)}
.c-012 .blk:last-child{border-bottom:0}
.c-012 .blk-name{display:block;font-size:12.5px;font-weight:500}
.c-012 .blk-meta{display:block;margin-top:3px;font-size:10.5px;color:var(--text-faint)}`,
    body: `<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="right"><p class="kicker">Recent sessions</p>${RECENTS.map((r) => `<div class="blk"><span class="blk-name">${r.name}</span><span class="blk-meta">${r.project} · ${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "013", name: "Corner Post", fam: "frame",
    css: `.c-013 .mock{position:relative;min-height:520px}
.c-013 .q{position:absolute;display:flex;flex-direction:column;gap:12px}
.c-013 .q-tl{top:40px;left:44px}
.c-013 .q-tr{top:44px;right:44px}
.c-013 .q-bl{bottom:44px;left:44px;max-width:300px}
.c-013 .q-br{bottom:40px;right:44px;width:280px}
.c-013 .title{font-family:var(--serif);font-size:58px;font-weight:500;line-height:0.95;letter-spacing:-0.02em;margin:0}
.c-013 .sub{margin:10px 0 0;color:var(--text-dim);font-size:12.5px}`,
    body: `<div class="q q-tl">${mark(34)}</div><div class="q q-tr">${cta("Open a folder")}</div><div class="q q-bl"><h1 class="title">Vela</h1><p class="sub">${SUB}</p></div><div class="q q-br"><div class="rows-wrap">${rowsList({ count: 4 })}</div></div>`,
  },
  {
    id: "014", name: "Nave", fam: "type",
    css: `.c-014 .mock{min-height:520px;display:flex;justify-content:center}
.c-014 .nave{width:min(430px,74%);border-left:1px solid var(--border-subtle);border-right:1px solid var(--border-subtle);padding:60px 40px 44px;display:flex;flex-direction:column;align-items:center;text-align:center}
.c-014 .mark-svg{width:38px;height:38px;margin-bottom:24px}
.c-014 .title{font-family:var(--serif);font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-014 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:26ch}
.c-014 .foot{margin-top:auto;width:100%;border-top:1px solid var(--border-subtle);padding-top:8px}`,
    body: `<div class="nave">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="foot">${rowsList({})}</div></div>`,
  },
  {
    id: "015", name: "Baseline", fam: "type",
    css: `.c-015 .mock{min-height:520px;padding:56px;display:flex;flex-direction:column}
.c-015 .lockup{margin-top:auto}
.c-015 .mark-svg{width:30px;height:30px;margin-bottom:26px}
.c-015 .title{font-family:var(--serif);font-size:88px;font-weight:500;line-height:0.9;letter-spacing:-0.03em;margin:0}
.c-015 .rule{height:1px;background:var(--border-strong);margin-top:18px}
.c-015 .under{display:flex;align-items:flex-start;justify-content:space-between;gap:24px;margin-top:16px}
.c-015 .sub{margin:0;color:var(--text-dim);font-size:13px;max-width:40ch}
.c-015 .strip{margin-top:26px;border-top:1px solid var(--border-subtle);padding-top:12px;display:flex;gap:22px;justify-content:center;font-size:11px;color:var(--text-faint)}
.c-015 .ri{display:flex;align-items:center;gap:6px}
.c-015 .ri-dot{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.c-015 .ri.is-live .ri-dot{background:var(--accent)}`,
    body: `<div class="lockup">${mark(30)}<h1 class="title">Vela</h1><div class="rule"></div><div class="under"><p class="sub">${SUB}</p>${cta("Open a folder")}</div></div><div class="strip">${RECENTS.map((r) => `<span class="ri${r.live ? " is-live" : ""}"><span class="ri-dot"></span>${r.name}</span>`).join("")}</div>`,
  },
  {
    id: "016", name: "Twin Desk", fam: "split",
    css: `.c-016 .mock{display:flex;min-height:520px}
.c-016 .half{flex:1;padding:56px 46px;display:flex;flex-direction:column;justify-content:center}
.c-016 .half-b{background:var(--bg-inset)}
.c-016 .mark-svg{width:38px;height:38px;margin-bottom:24px}
.c-016 .title{font-family:var(--serif);font-size:50px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-016 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:28ch}
.c-016 .kicker{font-size:9.5px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);margin:0 0 16px}`,
    body: `<div class="half half-a">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="half half-b"><p class="kicker">Recent</p><div class="rows-wrap">${numList()}</div></div>`,
  },
  {
    id: "017", name: "Halo", fam: "mark",
    css: `.c-017 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:48px}
.c-017 .halo{position:relative;width:190px;height:190px;border:1px solid var(--border-strong);border-radius:50%;display:grid;place-items:center;margin-bottom:34px}
.c-017 .halo::before{content:"";position:absolute;inset:10px;border:1px solid var(--border-subtle);border-radius:50%}
.c-017 .halo .mark-svg{width:60px;height:60px}
.c-017 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-017 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-017 .rows{width:100%;max-width:300px;margin-top:36px}`,
    body: `<div class="halo">${mark(60)}</div><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4 })}</div>`,
  },
  {
    id: "018", name: "Spine", fam: "frame",
    css: `.c-018 .mock{position:relative;min-height:520px;padding:60px 56px 52px 96px;display:flex;flex-direction:column}
.c-018 .spine{position:absolute;left:0;top:0;bottom:0;width:10px;background:var(--accent)}
.c-018 .spine-label{position:absolute;left:30px;top:60px;writing-mode:vertical-rl;font-size:9.5px;letter-spacing:0.32em;text-transform:uppercase;color:var(--text-faint)}
.c-018 .mark-svg{width:36px;height:36px;margin-bottom:24px}
.c-018 .title{font-family:var(--serif);font-size:52px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-018 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:34ch}
.c-018 .rows{margin-top:auto;border-top:1px solid var(--border-subtle);padding-top:8px}`,
    body: `<span class="spine"></span><span class="spine-label">Vela</span>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({})}</div>`,
  },
  {
    id: "019", name: "Triptych Bands", fam: "stack",
    css: `.c-019 .mock{display:flex;flex-direction:column;min-height:520px}
.c-019 .band{padding:24px 48px}
.c-019 .band-mark{border-bottom:1px solid var(--border-subtle)}
.c-019 .band-mark .mark-svg{width:28px;height:28px}
.c-019 .band-main{flex:1;display:flex;flex-direction:column;justify-content:center;align-items:flex-start}
.c-019 .title{font-family:var(--serif);font-size:56px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-019 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13.5px;max-width:40ch}
.c-019 .band-recents{border-top:1px solid var(--border-subtle);display:flex;gap:26px;justify-content:center;font-size:11px;color:var(--text-faint)}
.c-019 .ri{display:flex;align-items:center;gap:6px}
.c-019 .ri-dot{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.c-019 .ri.is-live .ri-dot{background:var(--accent)}`,
    body: `<div class="band band-mark">${mark(28)}</div><div class="band band-main"><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="band band-recents">${RECENTS.map((r) => `<span class="ri${r.live ? " is-live" : ""}"><span class="ri-dot"></span>${r.name}</span>`).join("")}</div>`,
  },
  {
    id: "020", name: "Graph Paper", fam: "atmos",
    css: `.c-020 .mock{position:relative;min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:56px 48px;background-image:linear-gradient(color-mix(in srgb,var(--text) 6%,transparent) 1px,transparent 1px),linear-gradient(90deg,color-mix(in srgb,var(--text) 6%,transparent) 1px,transparent 1px);background-size:24px 24px}
.c-020 .tick{position:absolute;width:14px;height:14px}
.c-020 .t-tl{top:18px;left:18px;border-top:1px solid var(--border-strong);border-left:1px solid var(--border-strong)}
.c-020 .t-tr{top:18px;right:18px;border-top:1px solid var(--border-strong);border-right:1px solid var(--border-strong)}
.c-020 .t-bl{bottom:18px;left:18px;border-bottom:1px solid var(--border-strong);border-left:1px solid var(--border-strong)}
.c-020 .t-br{bottom:18px;right:18px;border-bottom:1px solid var(--border-strong);border-right:1px solid var(--border-strong)}
.c-020 .mark-svg{width:36px;height:36px;margin-bottom:22px}
.c-020 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-020 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-020 .rows{width:100%;max-width:320px;margin-top:34px}`,
    body: `<span class="tick t-tl"></span><span class="tick t-tr"></span><span class="tick t-bl"></span><span class="tick t-br"></span>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4, ghost: true })}</div>`,
  },
  {
    id: "021", name: "Cards Row", fam: "cards",
    css: `.c-021 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px 48px}
.c-021 .mark-svg{width:36px;height:36px;margin-bottom:22px}
.c-021 .title{font-family:var(--serif);font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-021 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.c-021 .cards{display:flex;gap:12px;margin-top:36px}
.c-021 .card{width:172px;text-align:left;background:var(--bg);border:1px solid var(--border);border-radius:12px;padding:14px 16px}
.c-021 .card-name{font-size:12px;font-weight:500}
.c-021 .card-meta{margin-top:4px;font-size:10.5px;color:var(--text-faint);display:flex;align-items:center;gap:6px}
.c-021 .card-dot{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.c-021 .card.is-live .card-dot{background:var(--accent)}`,
    body: `${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="cards">${RECENTS.slice(0, 3).map((r) => `<div class="card${r.live ? " is-live" : ""}"><div class="card-name">${r.name}</div><div class="card-meta"><span class="card-dot"></span>${r.project} · ${r.when}</div></div>`).join("")}</div>`,
  },
  {
    id: "022", name: "Marquee", fam: "type",
    css: `.c-022 .mock{display:flex;min-height:520px;padding:56px;gap:48px}
.c-022 .left{flex:1.6;display:flex;flex-direction:column;justify-content:center}
.c-022 .mark-svg{width:34px;height:34px;margin-bottom:30px}
.c-022 .title{font-family:var(--serif);font-size:110px;font-weight:500;line-height:0.85;letter-spacing:-0.04em;margin:0}
.c-022 .right{width:250px;flex:none;border-left:1px solid var(--border-subtle);padding-left:36px;display:flex;flex-direction:column;justify-content:center;gap:22px}
.c-022 .sub{margin:0;color:var(--text-dim);font-size:13px}`,
    body: `<div class="left">${mark(34)}<h1 class="title">Vela</h1></div><div class="right"><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 3 })}</div></div>`,
  },
  {
    id: "023", name: "Footnotes", fam: "list",
    css: `.c-023 .mock{min-height:520px;padding:56px 54px 34px;display:flex;flex-direction:column}
.c-023 .mark-svg{width:32px;height:32px;margin-bottom:22px}
.c-023 .title{font-family:var(--serif);font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-023 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-023 .notes{margin-top:auto;border-top:1px solid var(--border-subtle);padding-top:14px;display:grid;grid-template-columns:1fr 1fr;gap:2px 32px}
.c-023 .note{display:flex;align-items:baseline;gap:8px;font-size:11.5px;color:var(--text-dim);padding:5px 0}
.c-023 .fn-i{font-family:var(--serif);font-style:italic;color:var(--accent);font-size:13px;min-width:16px}
.c-023 .fn-when{margin-left:auto;color:var(--text-faint);font-size:10.5px}`,
    body: `${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="notes">${RECENTS.map((r, i) => `<div class="note"><span class="fn-i">${i + 1}.</span><span>${r.name}</span><span class="fn-when">${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "024", name: "Velas", fam: "mark",
    css: `.c-024 .mock{position:relative;min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;overflow:hidden;padding:52px 48px}
.c-024 .velas{position:absolute;top:-150px;left:50%;transform:translateX(-50%);pointer-events:none}
.c-024 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.c-024 .mark-svg{width:42px;height:42px;margin-bottom:22px}
.c-024 .title{font-family:var(--serif);font-size:54px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-024 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-024 .rows{width:100%;max-width:340px;margin-top:34px}`,
    body: `<svg class="velas" width="700" height="360" viewBox="0 0 700 360" aria-hidden="true"><g fill="none"><ellipse cx="350" cy="180" rx="330" ry="120" transform="rotate(-14 350 180)" stroke="var(--accent)" stroke-opacity="0.22" stroke-width="1.5"/><ellipse cx="350" cy="180" rx="250" ry="86" transform="rotate(-14 350 180)" stroke="var(--border-strong)" stroke-width="1"/><circle cx="565" cy="95" r="5" fill="var(--accent)" fill-opacity="0.55"/></g></svg><div class="inner">${mark(42)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({})}</div></div>`,
  },
  {
    id: "025", name: "Inset Panel", fam: "split",
    css: `.c-025 .mock{display:flex;min-height:520px;padding:52px;gap:40px;background:var(--bg)}
.c-025 .left{flex:1.2;display:flex;flex-direction:column;justify-content:center}
.c-025 .mark-svg{width:38px;height:38px;margin-bottom:24px}
.c-025 .title{font-family:var(--serif);font-size:54px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-025 .sub{margin:0 0 28px;color:var(--text-dim);font-size:13px;max-width:28ch}
.c-025 .panel{width:290px;flex:none;background:var(--bg-panel);border:1px solid var(--border);border-radius:16px;box-shadow:var(--shadow-sm);padding:26px 28px;display:flex;flex-direction:column;justify-content:center}
.c-025 .kicker{font-size:9.5px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);margin:0 0 14px}`,
    body: `<div class="left">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="panel"><p class="kicker">Recent sessions</p><div class="rows-wrap">${rowsList({})}</div></div>`,
  },
  {
    id: "026", name: "Ticker", fam: "atmos",
    css: `.c-026 .mock{position:relative;min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:72px 48px 52px}
.c-026 .ticker{position:absolute;top:0;left:0;right:0;display:flex;align-items:center;justify-content:center;gap:8px;padding:10px 16px;border-bottom:1px solid var(--border-subtle);font-size:10.5px;color:var(--text-faint);white-space:nowrap;overflow:hidden}
.c-026 .live-dot{width:6px;height:6px;border-radius:50%;background:var(--accent);animation:c-026-pulse 2.4s ease-in-out infinite}
@keyframes c-026-pulse{0%,100%{box-shadow:0 0 0 0 var(--accent-dim)}50%{box-shadow:0 0 0 5px var(--accent-dim)}}
.c-026 .mark-svg{width:36px;height:36px;margin-bottom:22px}
.c-026 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-026 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-026 .rows{width:100%;max-width:320px;margin-top:34px}`,
    body: `<div class="ticker"><span class="live-dot"></span><span>Refine sessions panel&nbsp;&nbsp;·&nbsp;&nbsp;Provider usage cleanup&nbsp;&nbsp;·&nbsp;&nbsp;3 quiet</span></div>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4, ghost: true })}</div>`,
  },
  {
    id: "027", name: "Small Caps", fam: "type",
    css: `.c-027 .mock{display:flex;min-height:520px;padding:60px 52px;gap:52px}
.c-027 .left{flex:1;display:flex;flex-direction:column;justify-content:center}
.c-027 .mark-svg{width:26px;height:26px;margin-bottom:20px}
.c-027 .kicker{font-size:10px;font-weight:600;letter-spacing:0.28em;text-transform:uppercase;color:var(--accent);margin:0 0 12px}
.c-027 .title{font-family:var(--serif);font-size:42px;font-weight:500;margin:0 0 10px;letter-spacing:-0.01em}
.c-027 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:30ch}
.c-027 .right{width:260px;flex:none;border-left:1px solid var(--border-subtle);padding-left:32px;display:flex;flex-direction:column;justify-content:center}`,
    body: `<div class="left">${mark(26)}<p class="kicker">Every agent</p><h1 class="title">Vela</h1><p class="sub">One surface.</p>${cta("Open a folder")}</div><div class="right"><div class="rows-wrap">${rowsList({})}</div></div>`,
  },
  {
    id: "028", name: "Meridian", fam: "split",
    css: `.c-028 .mock{position:relative;min-height:520px;padding:56px 52px 56px 230px;display:flex;flex-direction:column;justify-content:center}
.c-028 .rail{position:absolute;left:150px;top:64px;bottom:64px;width:1px;background:var(--border-subtle)}
.c-028 .node{position:absolute;left:146px;width:9px;height:9px;border-radius:50%;border:1.5px solid var(--border-strong);background:var(--bg-panel);transform:translateY(-50%)}
.c-028 .node.is-live{background:var(--accent);border-color:var(--accent)}
.c-028 .item{position:absolute;left:168px;transform:translateY(-50%);font-size:10.5px;color:var(--text-faint);white-space:nowrap}
.c-028 .item.is-live{color:var(--text);font-weight:500}
.c-028 .mark-svg{width:36px;height:36px;margin-bottom:24px}
.c-028 .title{font-family:var(--serif);font-size:50px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-028 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:30ch}`,
    body: `<span class="rail"></span>${RECENTS.map((r, i) => `<span class="node${r.live ? " is-live" : ""}" style="top:${16 + i * 17}%"></span><span class="item${r.live ? " is-live" : ""}" style="top:${16 + i * 17}%">${r.when} — ${r.name}</span>`).join("")}${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}`,
  },
  {
    id: "029", name: "Thirds", fam: "frame",
    css: `.c-029 .mock{display:flex;min-height:520px}
.c-029 .col{flex:1;padding:52px 34px;display:flex;flex-direction:column;justify-content:center}
.c-029 .col-a{align-items:center}
.c-029 .col-b{border-left:1px solid var(--border-subtle);border-right:1px solid var(--border-subtle)}
.c-029 .title{font-family:var(--serif);font-size:40px;font-weight:500;margin:0 0 8px;letter-spacing:-0.015em}
.c-029 .sub{margin:0 0 24px;color:var(--text-dim);font-size:12.5px}`,
    body: `<div class="col col-a">${mark(40)}</div><div class="col col-b"><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="col col-c"><div class="rows-wrap">${rowsList({ count: 4 })}</div></div>`,
  },
  {
    id: "030", name: "Ghost O", fam: "atmos",
    css: `.c-030 .mock{position:relative;min-height:520px;padding:60px 56px;display:flex;flex-direction:column;justify-content:center;overflow:hidden}
.c-030 .ghost{position:absolute;right:-50px;top:50%;transform:translateY(-50%);font-family:var(--serif);font-size:430px;line-height:1;color:var(--text);opacity:0.045;pointer-events:none;user-select:none}
.c-030 .mark-svg{width:36px;height:36px;margin-bottom:24px}
.c-030 .title{font-family:var(--serif);font-size:56px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-030 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:30ch}
.c-030 .rows{width:100%;max-width:320px;margin-top:36px}`,
    body: `<span class="ghost">O</span>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4 })}</div>`,
  },
  {
    id: "031", name: "Two-Up Ledger", fam: "table",
    css: `.c-031 .mock{min-height:520px;padding:52px 48px 36px;display:flex;flex-direction:column}
.c-031 .head{display:flex;align-items:center;gap:16px;margin-bottom:6px}
.c-031 .head .mark-svg{width:30px;height:30px}
.c-031 .title{font-family:var(--serif);font-size:34px;font-weight:500;margin:0;letter-spacing:-0.015em}
.c-031 .sub{margin:0 0 24px;color:var(--text-dim);font-size:12.5px}
.c-031 .ledger{margin-top:auto;border-top:1px solid var(--border-subtle);padding:12px 0 16px}
.c-031 .mono-list{columns:2;column-gap:44px}
.c-031 .mono-list li{break-inside:avoid}`,
    body: `<div class="head">${mark(30)}<h1 class="title">Vela</h1></div><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="ledger">${monoList()}</div>`,
  },
  {
    id: "032", name: "Plaque", fam: "frame",
    css: `.c-032 .mock{min-height:520px;display:grid;place-items:center;background:var(--bg);padding:48px}
.c-032 .plaque{background:var(--bg-panel);border:1px solid var(--border);border-radius:20px;box-shadow:var(--shadow-sm);padding:54px 60px;display:flex;flex-direction:column;align-items:center;text-align:center;max-width:430px}
.c-032 .mark-svg{width:38px;height:38px;margin-bottom:22px}
.c-032 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-032 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-032 .rows{width:100%;max-width:300px;margin-top:32px}`,
    body: `<div class="plaque">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4 })}</div></div>`,
  },
  {
    id: "033", name: "Tab Base", fam: "list",
    css: `.c-033 .mock{position:relative;min-height:520px;padding:56px 48px 0;display:flex;flex-direction:column}
.c-033 .mark-svg{width:34px;height:34px;margin-bottom:22px}
.c-033 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-033 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-033 .tabs{margin-top:auto;display:flex;gap:8px;align-items:flex-end}
.c-033 .tab{flex:1;background:var(--bg-inset);border:1px solid var(--border);border-bottom:0;border-radius:10px 10px 0 0;padding:12px 14px 16px;font-size:11px;color:var(--text-dim)}
.c-033 .tab .t-when{display:block;margin-top:3px;color:var(--text-faint);font-size:10px}
.c-033 .tab.is-live{background:var(--bg-panel);color:var(--text);font-weight:500;transform:translateY(-5px)}`,
    body: `${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="tabs">${RECENTS.map((r) => `<div class="tab${r.live ? " is-live" : ""}">${r.name}<span class="t-when">${r.project} · ${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "034", name: "Nightcap", fam: "split",
    css: `.c-034 .mock{display:flex;flex-direction:column;min-height:520px}
.c-034 .field{background:#352a20;color:#fbf7ec;padding:54px 52px 46px;display:flex;flex-direction:column}
.c-034 .field .mark-svg{width:36px;height:36px;margin-bottom:24px}
.c-034 .title{font-family:var(--serif);font-size:52px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-034 .sub{margin:0 0 28px;color:rgba(251,247,236,0.72);font-size:13px;max-width:30ch}
.c-034 .field .btn-primary{background:#fbf7ec;border-color:#fbf7ec;color:#352a20}
.c-034 .field .btn-ghost{border-color:rgba(251,247,236,0.4);color:#fbf7ec}
.c-034 .field .btn-ghost:hover{background:rgba(251,247,236,0.08)}
.c-034 .paper{flex:1;padding:30px 52px;display:flex;flex-direction:column;justify-content:center}`,
    body: `<div class="field">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder", "Open a file…")}</div><div class="paper"><div class="rows-wrap">${rowsList({})}</div></div>`,
  },
  {
    id: "035", name: "Dial", fam: "mark",
    css: `.c-035 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px 48px}
.c-035 .dial{position:relative;width:150px;height:150px;margin-bottom:30px;display:grid;place-items:center;border:1px dashed var(--border-strong);border-radius:50%}
.c-035 .dial::after{content:"";position:absolute;inset:12px;border:1px solid var(--border-subtle);border-radius:50%}
.c-035 .dial .mark-svg{width:54px;height:54px}
.c-035 .title{font-family:var(--serif);font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-035 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-035 .rows{width:100%;max-width:320px;margin-top:34px}`,
    body: `<div class="dial">${mark(54)}</div><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4 })}</div>`,
  },
  {
    id: "036", name: "Sidenotes", fam: "list",
    css: `.c-036 .mock{display:flex;min-height:520px;padding:56px 50px;gap:48px}
.c-036 .left{flex:1;display:flex;flex-direction:column;justify-content:center}
.c-036 .mark-svg{width:34px;height:34px;margin-bottom:22px}
.c-036 .title{font-family:var(--serif);font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-036 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:28ch}
.c-036 .right{width:270px;flex:none;display:flex;flex-direction:column;justify-content:center}
.c-036 .snote{display:flex;gap:12px;padding:12px 0;border-bottom:1px solid var(--border-subtle);font-size:12px}
.c-036 .snote:last-child{border-bottom:0}
.c-036 .sn-i{font-family:var(--serif);font-style:italic;font-size:15px;color:var(--accent);min-width:18px}
.c-036 .sn-name{display:block;font-weight:500}
.c-036 .sn-when{display:block;margin-top:2px;color:var(--text-faint);font-size:10.5px}`,
    body: `<div class="left">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="right">${RECENTS.map((r, i) => `<div class="snote"><span class="sn-i">${i + 1}.</span><span class="sn-body"><span class="sn-name">${r.name}</span><span class="sn-when">${r.project} · ${r.when}</span></span></div>`).join("")}</div>`,
  },
  {
    id: "037", name: "Inverse Field", fam: "atmos",
    css: `.c-037{--bg:#171310;--bg-panel:#241d16;--bg-inset:#1c1712;--bg-hover:rgba(255,255,255,0.05);--text:#efe7db;--text-dim:#b6a894;--text-faint:#8a7d69;--accent:#9eb4a1;--accent-hover:#b2c4b4;--accent-dim:rgba(158,180,161,0.16);--border:rgba(255,255,255,0.08);--border-subtle:rgba(255,255,255,0.05);--border-strong:rgba(255,255,255,0.16);--on-accent:#172019}
.c-037 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px 48px;background:var(--bg-panel)}
.c-037 .mark-svg{width:40px;height:40px;margin-bottom:24px}
.c-037 .title{font-family:var(--serif);font-size:52px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-037 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-037 .rows{width:100%;max-width:320px;margin-top:34px}`,
    body: `${mark(40)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4, ghost: true })}</div>`,
  },
  {
    id: "038", name: "Quiet Table", fam: "table",
    css: `.c-038 .mock{min-height:520px;padding:56px 54px 36px;display:flex;flex-direction:column}
.c-038 .mark-svg{width:30px;height:30px;margin-bottom:22px}
.c-038 .title{font-family:var(--serif);font-size:42px;font-weight:500;margin:0 0 8px;letter-spacing:-0.015em}
.c-038 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.c-038 .tbl{margin-top:auto}
.c-038 .th{display:flex;justify-content:space-between;padding:0 2px 10px;border-bottom:1px solid var(--border-strong);font-size:9.5px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint)}
.c-038 .tr{display:flex;align-items:baseline;justify-content:space-between;gap:20px;padding:15px 2px;border-bottom:1px solid var(--border-subtle);font-size:13px}
.c-038 .tr-name{display:flex;align-items:center;gap:9px;font-weight:500}
.c-038 .tr-dot{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.c-038 .tr.is-live .tr-dot{background:var(--accent)}
.c-038 .tr-when{font-size:11px;color:var(--text-faint)}`,
    body: `${mark(30)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="tbl"><div class="th"><span>Session</span><span>Updated</span></div>${RECENTS.map((r) => `<div class="tr${r.live ? " is-live" : ""}"><span class="tr-name"><span class="tr-dot"></span>${r.name}</span><span class="tr-when">${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "039", name: "Stacked Wordmark", fam: "type",
    css: `.c-039 .mock{min-height:520px;padding:56px;display:flex;flex-direction:column}
.c-039 .mark-svg{width:32px;height:32px;margin-bottom:26px}
.c-039 .stack{margin-top:auto}
.c-039 .t1,.c-039 .t2{display:block;font-family:var(--serif);font-weight:500;font-size:96px;line-height:0.82;letter-spacing:-0.03em;margin:0}
.c-039 .t2{font-style:italic;color:var(--accent);margin-left:64px}
.c-039 .row{display:flex;align-items:flex-end;gap:28px;margin-top:24px}
.c-039 .sub{margin:0;color:var(--text-dim);font-size:13px;max-width:34ch}
.c-039 .rows{margin-top:30px;border-top:1px solid var(--border-subtle);padding-top:8px}`,
    body: `${mark(32)}<div class="stack"><span class="t1">Or</span><span class="t2">bit</span><div class="row"><p class="sub">${SUB}</p>${cta("Open a folder")}</div></div><div class="rows-wrap">${rowsList({})}</div>`,
  },
  {
    id: "040", name: "Arch", fam: "mark",
    css: `.c-040 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;text-align:center;padding:56px 48px 44px}
.c-040 .arch{width:210px;height:230px;border:1px solid var(--border-strong);border-radius:105px 105px 0 0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:16px;margin-bottom:30px}
.c-040 .arch .mark-svg{width:48px;height:48px}
.c-040 .arch .vtag{font-size:9.5px;letter-spacing:0.26em;text-transform:uppercase;color:var(--text-faint)}
.c-040 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-040 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-040 .rows{width:100%;max-width:330px;margin-top:30px}`,
    body: `<div class="arch">${mark(48)}<span class="vtag">v0.5</span></div><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4 })}</div>`,
  },
  {
    id: "041", name: "Tri Columns", fam: "cards",
    css: `.c-041 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;text-align:center;padding:56px 52px 36px}
.c-041 .mark-svg{width:36px;height:36px;margin-bottom:22px}
.c-041 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-041 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-041 .cols{margin-top:auto;width:100%;border-top:1px solid var(--border-subtle);padding-top:16px;display:grid;grid-template-columns:repeat(3,1fr);gap:24px;text-align:left}
.c-041 .col-name{display:flex;align-items:center;gap:7px;font-size:12px;font-weight:500}
.c-041 .col-dot{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.c-041 .col.is-live .col-dot{background:var(--accent)}
.c-041 .col-meta{margin-top:3px;font-size:10.5px;color:var(--text-faint)}`,
    body: `${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="cols">${RECENTS.slice(0, 3).map((r) => `<div class="col${r.live ? " is-live" : ""}"><span class="col-name"><span class="col-dot"></span>${r.name}</span><span class="col-meta">${r.project} · ${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "042", name: "Double Rule", fam: "frame",
    css: `.c-042 .mock{min-height:520px;padding:64px 56px 48px;display:flex;flex-direction:column;justify-content:center}
.c-042 .frame{position:relative;padding:44px 8px;text-align:left}
.c-042 .frame::before,.c-042 .frame::after{content:"";position:absolute;left:0;right:0;height:4px;border-top:1px solid var(--border-strong);border-bottom:1px solid var(--border-subtle)}
.c-042 .frame::before{top:0}
.c-042 .frame::after{bottom:0}
.c-042 .mark-svg{width:34px;height:34px;margin-bottom:22px}
.c-042 .title{font-family:var(--serif);font-size:52px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-042 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px;max-width:36ch}
.c-042 .rows{margin-top:28px}`,
    body: `<div class="frame">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="rows-wrap">${rowsList({})}</div>`,
  },
  {
    id: "043", name: "Numbered Margin", fam: "list",
    css: `.c-043 .mock{min-height:520px;padding:52px 54px 32px;display:flex;flex-direction:column}
.c-043 .head{display:flex;align-items:center;gap:14px}
.c-043 .head .mark-svg{width:28px;height:28px}
.c-043 .head .title{font-family:var(--serif);font-size:30px;font-weight:500;margin:0;letter-spacing:-0.01em}
.c-043 .head .cta-row{margin-left:auto}
.c-043 .head .btn{padding:7px 14px;font-size:11.5px}
.c-043 .sub{margin:8px 0 0;color:var(--text-dim);font-size:12.5px}
.c-043 .list{margin-top:auto}
.c-043 .li{display:flex;align-items:center;gap:18px;padding:12px 0;border-bottom:1px solid var(--border-subtle)}
.c-043 .li:first-child{border-top:1px solid var(--border-subtle)}
.c-043 .num{font-family:var(--serif);font-style:italic;font-size:26px;line-height:1;color:var(--text);opacity:0.18;min-width:44px}
.c-043 .nm{font-size:13px;font-weight:500}
.c-043 .wh{margin-left:auto;font-size:11px;color:var(--text-faint)}`,
    body: `<div class="head">${mark(28)}<h1 class="title">Vela</h1>${cta("Open a folder")}</div><p class="sub">${SUB}</p><div class="list">${RECENTS.map((r, i) => `<div class="li"><span class="num">${String(i + 1).padStart(2, "0")}</span><span class="nm">${r.name}</span><span class="wh">${r.project} · ${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "044", name: "Mirror", fam: "split",
    css: `.c-044 .mock{display:flex;min-height:520px}
.c-044 .half-l{flex:1;background:var(--bg-inset);padding:56px 40px;display:flex;flex-direction:column;justify-content:center}
.c-044 .half-r{flex:1;padding:56px 48px;display:flex;flex-direction:column;justify-content:center;align-items:flex-end;text-align:right}
.c-044 .mark-svg{width:38px;height:38px;margin-bottom:24px}
.c-044 .title{font-family:var(--serif);font-size:52px;font-weight:500;margin:0 0 10px;letter-spacing:-0.02em}
.c-044 .sub{margin:0 0 28px;color:var(--text-dim);font-size:13px;max-width:28ch}`,
    body: `<div class="half-l"><div class="rows-wrap">${rowsList({})}</div></div><div class="half-r">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div>`,
  },
  {
    id: "045", name: "Eclipse", fam: "mark",
    css: `.c-045 .mock{position:relative;min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;overflow:hidden;padding:52px 48px}
.c-045 .eclipse{position:absolute;top:64px;right:104px;width:180px;height:180px;border-radius:50%;background:var(--accent);opacity:0.16;pointer-events:none}
.c-045 .eclipse::after{content:"";position:absolute;inset:0;transform:translate(-36px,-22px);border-radius:50%;background:var(--bg-panel)}
.c-045 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.c-045 .mark-svg{width:40px;height:40px;margin-bottom:22px}
.c-045 .title{font-family:var(--serif);font-size:54px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-045 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-045 .rows{width:100%;max-width:330px;margin-top:34px}`,
    body: `<span class="eclipse"></span><div class="inner">${mark(40)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4 })}</div></div>`,
  },
  {
    id: "046", name: "Column Rules", fam: "table",
    css: `.c-046 .mock{min-height:520px;padding:54px 50px 34px;display:flex;flex-direction:column}
.c-046 .mark-svg{width:28px;height:28px;margin-bottom:20px}
.c-046 .title{font-family:var(--serif);font-size:40px;font-weight:500;margin:0 0 8px;letter-spacing:-0.015em}
.c-046 .sub{margin:0 0 22px;color:var(--text-dim);font-size:12.5px}
.c-046 .news{margin-top:auto;border-top:1px solid var(--border-strong);padding-top:6px;display:grid;grid-template-columns:repeat(3,1fr)}
.c-046 .ncol{padding:14px 18px 8px;border-left:1px solid var(--border-subtle)}
.c-046 .ncol:first-child{border-left:0;padding-left:0}
.c-046 .ne-name{font-family:var(--serif);font-style:italic;font-size:14.5px}
.c-046 .ne-when{margin-top:3px;font-size:10px;color:var(--text-faint);letter-spacing:0.04em}`,
    body: `${mark(28)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="news">${[0, 1, 2].map((c) => `<div class="ncol">${RECENTS.filter((_, i) => i % 3 === c).map((r) => `<div class="ne"><div class="ne-name"${r.live ? ' style="color:var(--accent)"' : ""}>${r.name}</div><div class="ne-when">${r.project.toUpperCase()} — ${r.when}</div></div>`).join("")}</div>`).join("")}</div>`,
  },
  {
    id: "047", name: "Stepwell", fam: "stack",
    css: `.c-047 .mock{min-height:520px;padding:56px 54px 44px;display:flex;flex-direction:column}
.c-047 .mark-svg{width:32px;height:32px;margin-bottom:22px}
.c-047 .title{font-family:var(--serif);font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-047 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.c-047 .steps{margin-top:auto;display:flex;flex-direction:column;gap:8px}
.c-047 .step{display:flex;align-items:center;gap:10px;height:44px;border:1px solid var(--border);border-radius:10px;background:var(--bg-inset);padding:0 16px;font-size:11.5px;color:var(--text-dim)}
.c-047 .step:nth-child(1){width:100%}
.c-047 .step:nth-child(2){width:92%}
.c-047 .step:nth-child(3){width:84%}
.c-047 .step:nth-child(4){width:76%}
.c-047 .step:nth-child(5){width:68%}
.c-047 .st-dot{width:6px;height:6px;border-radius:50%;background:var(--border-strong);flex:none}
.c-047 .step.is-live{background:var(--bg-panel);border-color:var(--border-strong);color:var(--text);font-weight:500}
.c-047 .step.is-live .st-dot{background:var(--accent)}
.c-047 .st-when{margin-left:auto;font-size:10.5px;color:var(--text-faint)}`,
    body: `${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="steps">${RECENTS.map((r) => `<div class="step${r.live ? " is-live" : ""}"><span class="st-dot"></span>${r.name}<span class="st-when">${r.when}</span></div>`).join("")}</div>`,
  },
  {
    id: "048", name: "Vignette", fam: "atmos",
    css: `.c-048 .mock{position:relative;min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;overflow:hidden;padding:52px}
.c-048::before{content:"";position:absolute;inset:0;background:radial-gradient(ellipse at center,transparent 50%,color-mix(in srgb,var(--text) 7%,transparent));pointer-events:none}
.c-048 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.c-048 .mark-svg{width:38px;height:38px;margin-bottom:24px}
.c-048 .title{font-family:var(--serif);font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-048 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-048 .rows{width:100%;max-width:310px;margin-top:34px}`,
    body: `<div class="inner">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4, ghost: true })}</div></div>`,
  },
  {
    id: "049", name: "Kicker Bar", fam: "frame",
    css: `.c-049 .mock{position:relative;min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:64px 48px 48px}
.c-049 .bar{position:absolute;top:0;left:0;right:0;height:6px;background:linear-gradient(90deg,var(--accent),color-mix(in srgb,var(--accent) 35%,transparent))}
.c-049 .mark-svg{width:38px;height:38px;margin-bottom:24px}
.c-049 .title{font-family:var(--serif);font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.c-049 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.c-049 .rows{width:100%;max-width:330px;margin-top:34px}`,
    body: `<span class="bar"></span>${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="rows-wrap">${rowsList({ count: 4 })}</div>`,
  },
  {
    id: "050", name: "Colophon", fam: "type",
    css: `.c-050 .mock{min-height:520px;padding:56px 54px 30px;display:flex;flex-direction:column}
.c-050 .mark-svg{width:34px;height:34px;margin-bottom:24px}
.c-050 .title{font-family:var(--serif);font-size:76px;font-weight:500;line-height:0.95;letter-spacing:-0.025em;margin:0}
.c-050 .sub{margin:14px 0 0;color:var(--text-dim);font-size:13px;max-width:40ch}
.c-050 .colo{margin-top:auto;border-top:1px solid var(--border-subtle);padding-top:16px;display:flex;align-items:center;gap:22px}
.c-050 .colo .cta-row{margin:0}
.c-050 .sep{flex:1}
.c-050 .ri{display:flex;align-items:center;gap:6px;font-size:10.5px;color:var(--text-faint)}
.c-050 .ri-dot{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.c-050 .ri.is-live .ri-dot{background:var(--accent)}`,
    body: `${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p><div class="colo">${cta("Open a folder")}<span class="sep"></span>${RECENTS.map((r) => `<span class="ri${r.live ? " is-live" : ""}"><span class="ri-dot"></span>${r.name}</span>`).join("")}</div>`,
  },
]

// ---- shared css -------------------------------------------------------------

const BASE_CSS = `:root {
  color-scheme: light;
  --bg: #f4eee1;
  --bg-panel: #fbf7ec;
  --bg-inset: #eee5d4;
  --bg-hover: rgba(43,33,25,0.055);
  --text: #2b2119;
  --text-dim: #6b5f50;
  --text-faint: #948571;
  --accent: #617a68;
  --accent-hover: #4f6757;
  --accent-dim: rgba(97,122,104,0.15);
  --border: rgba(43,33,25,0.075);
  --border-strong: rgba(43,33,25,0.15);
  --border-subtle: rgba(43,33,25,0.05);
  --radius-full: 999px;
  --shadow-sm: 0 1px 2px rgba(67,48,33,0.08), inset 0 1px 0 rgba(255,255,255,0.45);
  --shadow-md: 0 2px 8px rgba(67,48,33,0.08), 0 16px 38px rgba(67,48,33,0.10);
  --grid: rgba(43,33,25,0.035);
  --serif: "Cormorant Garamond", "Iowan Old Style", Palatino, Georgia, serif;
  --on-accent: #fbf7ec;
}
:root[data-theme="dark"] {
  color-scheme: dark;
  --bg: #171412;
  --bg-panel: #262220;
  --bg-inset: #201d1b;
  --bg-hover: rgba(255,255,255,0.05);
  --text: #e8e3dd;
  --text-dim: #a8a29e;
  --text-faint: #8f8880;
  --accent: #9eb4a1;
  --accent-hover: #b2c4b4;
  --accent-dim: rgba(158,180,161,0.16);
  --border: rgba(255,255,255,0.05);
  --border-strong: rgba(255,255,255,0.10);
  --border-subtle: rgba(255,255,255,0.04);
  --shadow-sm: 0 1px 2px rgba(0,0,0,0.20);
  --shadow-md: 0 2px 8px rgba(0,0,0,0.10), 0 16px 38px rgba(0,0,0,0.22);
  --grid: rgba(255,255,255,0.025);
  --on-accent: #172019;
}
* { box-sizing: border-box; }
html { min-width: 320px; background: var(--bg); }
body { margin: 0; min-height: 100vh; background: var(--bg); color: var(--text); font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 13px; -webkit-font-smoothing: antialiased; text-rendering: optimizeLegibility; }
button, input, select { font: inherit; }
button, select { cursor: pointer; }
button:focus-visible, input:focus-visible, select:focus-visible { outline: 1px solid var(--accent); outline-offset: 2px; }
.preview-head { max-width: 720px; margin: 0 auto 16px; display: flex; justify-content: space-between; align-items: center; gap: 12px; color: var(--text-faint); font-size: 11px; }
.preview-head strong { color: var(--accent); letter-spacing: 0.06em; }
.pv-actions { display: flex; align-items: center; gap: 8px; }
.pv-btn, .preview-head a { color: var(--text-faint); text-decoration: none; border: 1px solid var(--border); background: transparent; border-radius: var(--radius-full); font-size: 11px; padding: 6px 12px; display: inline-flex; align-items: center; gap: 6px; }
.pv-btn:hover, .preview-head a:hover { color: var(--text); background: var(--bg-hover); }
.board { max-width: 720px; margin: 0 auto; }
.preview-note { max-width: 720px; margin: 12px auto 0; text-align: center; color: var(--text-faint); font-size: 10.5px; }
.mock { position: relative; background: var(--bg-panel); border: 1px solid var(--border); border-radius: 20px; overflow: hidden; min-height: 520px; }
.mark-svg { display: block; }
.title { font-family: var(--serif); }
.rows, .mono-list, .num-list { list-style: none; margin: 0; padding: 0; }
.row { display: flex; align-items: center; gap: 10px; padding: 9px 2px; border-bottom: 1px solid var(--border-subtle); }
.row:last-child { border-bottom: 0; }
.row-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--border-strong); flex: none; }
.row.is-live .row-dot { background: var(--accent); }
.row-name { font-size: 12.5px; font-weight: 500; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.row-meta { margin-left: auto; padding-left: 14px; font-size: 10.5px; color: var(--text-faint); white-space: nowrap; }
.rows-ghost { opacity: 0.55; }
.mono-list { font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; font-size: 11px; }
.mono-list li { display: flex; align-items: baseline; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border-subtle); }
.mono-list li:last-child { border-bottom: 0; }
.mono-i { color: var(--text-faint); }
.mono-name { color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.mono-lead { flex: 1; min-width: 20px; border-bottom: 1px dotted var(--border-strong); transform: translateY(-3px); }
.mono-when { color: var(--text-dim); white-space: nowrap; }
.num-list li { display: flex; align-items: baseline; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border-subtle); }
.num-list li:last-child { border-bottom: 0; }
.num-i { font-family: var(--serif); font-style: italic; font-size: 14.5px; color: var(--accent); min-width: 22px; }
.num-name { font-size: 12.5px; font-weight: 500; }
.num-when { margin-left: auto; font-size: 10.5px; color: var(--text-faint); white-space: nowrap; }
.cta-row { display: flex; align-items: center; gap: 10px; }
.btn { appearance: none; border: 1px solid var(--border-strong); background: transparent; color: var(--text); border-radius: var(--radius-full); padding: 9px 18px; font-size: 12.5px; font-weight: 500; }
.btn:hover { background: var(--bg-hover); }
.btn-primary { background: var(--accent); border-color: var(--accent); color: var(--on-accent); }
.btn-primary:hover { background: var(--accent-hover); border-color: var(--accent-hover); }`;

// ---- page shells ------------------------------------------------------------

const SPEC = {
  "001": "Centered monolith: mark, oversized serif title, one action, recents as a bottom ticker.",
  "002": "Sage side rail with rotated wordmark; hero and recents share the remaining canvas.",
  "003": "Mark and title locked into one horizontal unit; recents behind a hairline divider.",
  "004": "A giant ghost vela mark backs a centered lockup; recents fade to whispers.",
  "005": "Hairline cross divides the canvas into quadrants; content posts to three of them.",
  "006": "Serif italic session names read like margin notes beside the hero.",
  "007": "The mark sits in a thin circular aperture above the title.",
  "008": "Equal halves: hero left, recents right, one hairline between.",
  "009": "Oversized title over a short sage rule; recents anchor the bottom.",
  "010": "Directory of hollow dot rows; live sessions fill their dot with sage.",
  "011": "Hero compresses to the top; a dotted-leader mono ledger owns the base.",
  "012": "Wide gutter split; recents become two-line blocks on an inset field.",
  "013": "Content posts to the four corners; the middle stays empty.",
  "014": "A centered column between two full-height hairlines, like a nave.",
  "015": "An 88px title rests on a full-width baseline rule; recents run beneath.",
  "016": "Twin desks: paper left, inset right carrying a numbered serif list.",
  "017": "Double-ring halo encircles the bare mark; recents stay narrow.",
  "018": "A 10px sage spine with rotated label anchors the left edge.",
  "019": "Three stacked bands: bare mark, statement, recents ticker.",
  "020": "Fine graph-paper grid with corner ticks frames a centered lockup.",
  "021": "Compact hero over three hairline session cards in a row.",
  "022": "Marquee type: 110px title left, action and recents in a side column.",
  "023": "Sessions demote to numbered footnotes in two columns at the base.",
  "024": "Tilted vela ellipses sweep behind the centered lockup.",
  "025": "Recents recess into a raised inset panel beside the hero.",
  "026": "A slim live-session ticker pins the top edge with a pulsing dot.",
  "027": "Small-caps sage kicker leads a modest serif title; list on the right.",
  "028": "A meridian rail of node dots timestamps sessions down the left.",
  "029": "Two hairlines cut the canvas into thirds: mark, statement, recents.",
  "030": "A 430px ghost serif O watermarks the right edge behind left-set type.",
  "031": "The mono ledger splits into two balanced columns under a lockup head.",
  "032": "A single plaque floats on open paper; nothing else touches the field.",
  "033": "Sessions become folder tabs flush with the bottom edge; live raises.",
  "034": "A fixed warm-ink band carries the hero; recents stay on paper below.",
  "035": "A dashed dial ring surrounds the mark like a compass face.",
  "036": "Numbered serif sidenotes answer the hero from the right margin.",
  "037": "A fixed ink palette inverts this concept in both themes.",
  "038": "An airy two-column table with a caps header row; hero above.",
  "039": "The wordmark stacks Or / bit with an italic sage second line.",
  "040": "A round-topped arch frames the mark; content flows beneath it.",
  "041": "Sessions settle into three quiet text columns under a centered hero.",
  "042": "Double hairlines close above and below the hero like ledger rules.",
  "043": "Giant faint numerals hold the left margin of full-width session rows.",
  "044": "Mirrored reading order: recents left, right-aligned hero right.",
  "045": "A sage disc eclipses behind the lockup, masked by a paper circle.",
  "046": "Sessions set as newspaper columns divided by vertical hairlines.",
  "047": "Descending staggered bars step down like a well under the hero.",
  "048": "A soft radial vignette presses the edges; the lockup floats center.",
  "049": "A fading sage bar kisses the top edge above a classic centered column.",
  "050": "Everything funnels into a bottom colophon: action left, recents right.",
};

const page = (c) => `<!doctype html>
<html lang="en" data-theme="paper">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>${c.id} — ${c.name}</title>
<link rel="stylesheet" href="../landing-pages.css"/>
</head>
<body style="padding:28px 16px">
<div class="preview-head"><span><strong>${c.id}</strong> ${c.name} · ${c.fam}</span><span class="pv-actions"><button class="pv-btn" type="button" onclick="document.documentElement.dataset.theme=document.documentElement.dataset.theme==='dark'?'paper':'dark'">◐ Theme</button><a href="../index.html">← Gallery</a></span></div>
<div class="board"><div class="c-${c.id}"><div class="mock">${c.body}</div></div></div>
<p class="preview-note">${SPEC[c.id]}</p>
</body>
</html>
`;

function gallery() {
  const fams = [...new Set(CONCEPTS.map((c) => c.fam))].sort();
  const cards = CONCEPTS.map(
    (c) => `<section class="gal-card" data-id="${c.id}" data-name="${c.name.toLowerCase()}" data-fam="${c.fam}">
<header class="gal-card-head"><button class="gal-star" type="button" data-star="${c.id}" title="Shortlist">☆</button><span class="gal-id">${c.id}</span><span class="gal-name">${c.name}</span><span class="gal-tag">${c.fam}</span><a class="gal-open" href="concepts/${c.id}.html">Full →</a></header>
<div class="gal-stage"><div class="gal-zoom"><div class="c-${c.id}"><div class="mock">${c.body}</div></div></div></div>
<p class="gal-spec">${SPEC[c.id]}</p>
</section>`
  ).join("\n");
  return `<!doctype html>
<html lang="en" data-theme="paper">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>Vela — fifty calm fronts</title>
<link rel="stylesheet" href="./landing-pages.css"/>
<style>
.gal-shell { max-width: 1560px; margin: 0 auto; padding: 40px 28px 80px; }
.gal-head { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; flex-wrap: wrap; padding-bottom: 20px; border-bottom: 1px solid var(--border); }
.gal-kicker { color: var(--accent); font-size: 9.5px; font-weight: 600; letter-spacing: 0.13em; text-transform: uppercase; }
.gal-head h1 { margin: 6px 0 4px; font-family: var(--serif); font-size: clamp(26px, 2.4vw, 36px); font-weight: 500; letter-spacing: -0.03em; line-height: 1; }
.gal-head p { margin: 0; color: var(--text-dim); font-size: 12.5px; max-width: 66ch; }
.gal-controls { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.gal-search { display: flex; align-items: center; gap: 7px; min-height: 34px; padding: 0 12px; background: var(--bg-inset); border-radius: 12px; }
.gal-search input { width: 200px; border: 0; background: transparent; color: var(--text); font-size: 12px; outline: 0; }
.gal-search input::placeholder { color: var(--text-faint); }
.gal-select, .gal-chip { min-height: 34px; display: inline-flex; align-items: center; gap: 6px; padding: 0 12px; background: var(--bg-inset); color: var(--text-dim); border: 0; border-radius: 12px; font-size: 11.5px; }
.gal-chip.is-shake { animation: gal-shake 0.3s ease; }
@keyframes gal-shake { 25% { transform: translateX(-2px); } 75% { transform: translateX(2px); } }
.gal-grid { margin-top: 24px; display: grid; grid-template-columns: repeat(auto-fill, minmax(400px, 1fr)); gap: 18px; }
.gal-card { background: var(--bg-panel); border: 1px solid var(--border); border-radius: 16px; overflow: hidden; transition: box-shadow 0.15s ease, border-color 0.15s ease; }
.gal-card:hover { border-color: var(--border-strong); box-shadow: var(--shadow-md); }
.gal-card.is-picked { border-color: var(--accent); }
.gal-card.is-hidden { display: none; }
.gal-card-head { display: flex; align-items: center; gap: 8px; padding: 10px 14px; border-bottom: 1px solid var(--border-subtle); }
.gal-star { border: 0; background: transparent; color: var(--text-faint); font-size: 14px; padding: 2px 4px; line-height: 1; }
.gal-card.is-picked .gal-star { color: var(--accent); }
.gal-id { color: var(--accent); font-size: 11px; font-weight: 600; letter-spacing: 0.06em; }
.gal-name { font-weight: 500; font-size: 12.5px; }
.gal-tag { color: var(--text-faint); font-size: 10px; border: 1px solid var(--border); border-radius: var(--radius-full); padding: 2px 8px; }
.gal-open { margin-left: auto; color: var(--text-faint); text-decoration: none; font-size: 11px; }
.gal-open:hover { color: var(--text); }
.gal-stage { position: relative; overflow: hidden; background: var(--bg); }
.gal-zoom { width: 760px; transform-origin: 0 0; pointer-events: none; }
.gal-zoom .mock { border: 0; border-radius: 0; }
.gal-spec { margin: 0; padding: 9px 14px; color: var(--text-faint); font-size: 10.5px; border-top: 1px solid var(--border-subtle); }
.gal-empty { display: none; margin-top: 32px; text-align: center; color: var(--text-faint); font-size: 12px; }
.gal-empty.is-on { display: block; }
.gal-sets { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-top: 18px; }
.gal-sets span { color: var(--text-faint); font-size: 10px; letter-spacing: 0.12em; text-transform: uppercase; margin-right: 4px; }
.gal-sets a { color: var(--text-dim); text-decoration: none; font-size: 11.5px; border: 1px solid var(--border-strong); border-radius: var(--radius-full); padding: 7px 14px; }
.gal-sets a:hover { color: var(--on-accent); background: var(--accent); border-color: var(--accent); }
</style>
</head>
<body>
<div class="gal-shell">
<header class="gal-head">
<div><span class="gal-kicker">Vela · landing study</span><h1>Fifty calm fronts</h1><p>One distilled direction — bare mark, serif title, one line, one action, calm recents — composed fifty ways. Star up to four to shortlist; previews follow your theme.</p></div>
<div class="gal-controls">
<label class="gal-search"><input id="gal-q" type="search" placeholder="Search id, name, family…" autocomplete="off"/></label>
<select id="gal-fam" class="gal-select"><option value="">All families</option>${fams.map((f) => `<option value="${f}">${f}</option>`).join("")}</select>
<button id="gal-theme" class="gal-chip" type="button">◐ Theme</button>
<span id="gal-count" class="gal-chip">Shortlist 0/4</span>
</div>
</header>
<nav class="gal-sets"><span>Deep dives</span><a href="iterations/td.html">Twin Desk \u00b7 50 variations</a><a href="iterations/gp.html">Graph Paper \u00b7 50 variations</a><a href="iterations/pq.html">Plaque \u00b7 50 variations</a><a href="iterations/wm.html">Watermark \u00b7 50 variations</a></nav>
<main class="gal-grid" id="gal-grid">
${cards}
</main>
<p class="gal-empty" id="gal-empty">Nothing matches that filter.</p>
</div>
<script>
(function () {
  var KEY = "vela-landing-shortlist";
  var picked = [];
  try { picked = JSON.parse(localStorage.getItem(KEY) || "[]"); } catch (e) { picked = []; }
  var count = document.getElementById("gal-count");
  var q = document.getElementById("gal-q");
  var fam = document.getElementById("gal-fam");
  var empty = document.getElementById("gal-empty");
  var cards = Array.prototype.slice.call(document.querySelectorAll(".gal-card"));

  function save() { try { localStorage.setItem(KEY, JSON.stringify(picked)); } catch (e) {} }
  function paint() {
    cards.forEach(function (card) {
      var on = picked.indexOf(card.dataset.id) >= 0;
      card.classList.toggle("is-picked", on);
      card.querySelector(".gal-star").textContent = on ? "★" : "☆";
    });
    count.textContent = "Shortlist " + picked.length + "/4";
  }
  function filter() {
    var needle = q.value.trim().toLowerCase();
    var wantFam = fam.value;
    var shown = 0;
    cards.forEach(function (card) {
      var hay = card.dataset.id + " " + card.dataset.name + " " + card.dataset.fam;
      var hit = hay.indexOf(needle) >= 0 && (!wantFam || card.dataset.fam === wantFam);
      card.classList.toggle("is-hidden", !hit);
      if (hit) shown += 1;
    });
    empty.classList.toggle("is-on", shown === 0);
  }

  document.addEventListener("click", function (event) {
    var star = event.target.closest("[data-star]");
    if (!star) return;
    var id = star.dataset.star;
    var at = picked.indexOf(id);
    if (at >= 0) picked.splice(at, 1);
    else if (picked.length < 4) picked.push(id);
    else {
      count.classList.remove("is-shake");
      void count.offsetWidth;
      count.classList.add("is-shake");
      return;
    }
    save();
    paint();
  });
  q.addEventListener("input", filter);
  fam.addEventListener("change", filter);
  document.getElementById("gal-theme").addEventListener("click", function () {
    var root = document.documentElement;
    var next = root.dataset.theme === "dark" ? "paper" : "dark";
    root.dataset.theme = next;
    try { localStorage.setItem("vela-landing-theme", next); } catch (e) {}
  });
  try {
    var theme = localStorage.getItem("vela-landing-theme");
    if (theme) document.documentElement.dataset.theme = theme;
  } catch (e) {}

  function fit() {
    Array.prototype.forEach.call(document.querySelectorAll(".gal-stage"), function (stage) {
      var scale = stage.clientWidth / 760;
      var zoom = stage.firstElementChild;
      zoom.style.transform = "scale(" + scale + ")";
      stage.style.height = Math.ceil(520 * scale) + "px";
    });
  }
  window.addEventListener("resize", fit);

  paint();
  filter();
  fit();
})();
</script>
</body>
</html>
`;
}

// ---- emit -------------------------------------------------------------------

const knownIds = new Set(CONCEPTS.map((c) => c.id));
for (const entry of readdirSync(join(here, "concepts"))) {
  if (!knownIds.has(entry.replace(/\.html$/, ""))) unlinkSync(join(here, "concepts", entry));
}
for (const c of CONCEPTS) writeFileSync(join(here, "concepts", `${c.id}.html`), page(c));
writeFileSync(join(here, "landing-pages.css"), [BASE_CSS, ...CONCEPTS.map((c) => c.css)].join("\n"));
try { unlinkSync(join(here, "landing-pages.js")); } catch {}
writeFileSync(join(here, "index.html"), gallery());
console.log(`Emitted ${CONCEPTS.length} concepts, landing-pages.css, index.html`);
