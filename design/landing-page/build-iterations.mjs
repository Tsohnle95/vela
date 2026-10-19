// Generates 4×50 landing iterations distilled from shortlisted 016 Twin Desk,
// 020 Graph Paper, 032 Plaque, 004 Watermark. Emits iterations/*.html
// galleries plus iterations.css. Run after build-concepts.mjs.

import { writeFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));

// ---- shared builders --------------------------------------------------------

const mark = (size = 32) => `<svg class="mark-svg" viewBox="0 0 96 96" width="${size}" height="${size}" aria-hidden="true"><g fill="none" stroke-linecap="round"><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(24 48 48)" stroke="#9eb4a1" stroke-width="4.5"/><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(-24 48 48)" stroke="#617a68" stroke-width="5"/><circle cx="48" cy="48" r="9" fill="#46584b"/><circle cx="62.1" cy="28.3" r="6" fill="#9eb4a1"/></g></svg>`;

const SUB = "Every agent. One surface.";

const RECENTS = [
  { name: "Refine sessions panel", project: "vela", when: "just now", live: true },
  { name: "Provider usage cleanup", project: "vela", when: "18m ago", live: true },
  { name: "Update runtime docs", project: "vela", when: "yesterday" },
  { name: "Polish daily notes", project: "atlas-notes", when: "2h ago" },
  { name: "Improve quick search", project: "atlas-notes", when: "Tue" },
];

const WS = [
  { name: "vela", count: 4, desc: "Shell, adapters, streams", live: true },
  { name: "atlas-notes", count: 2, desc: "Daily notes pipeline" },
  { name: "quiet-web", count: 2, desc: "Site refresh" },
];

const CHEV = `<svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true"><path d="m4.5 6 3.5 3.5 3.5-3.5" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

const rowsList = ({ count = 5, ghost = false } = {}) =>
  `<ul class="rows${ghost ? " rows-ghost" : ""}">` +
  RECENTS.slice(0, count)
    .map(
      (r) => `<li class="row${r.live ? " is-live" : ""}"><span class="row-dot"></span><span class="row-name">${r.name}</span><span class="row-meta">${r.when}</span><span class="go">${CHEV}</span></li>`
    )
    .join("") +
  `</ul>`;

const wsRows = () =>
  `<ul class="ws">` +
  WS.map(
    (w) => `<li class="ws-row${w.live ? " is-live" : ""}"><span class="ws-dot"></span><span class="ws-main"><span class="ws-name">${w.name}</span><span class="ws-desc">${w.desc}</span></span><span class="ws-count">${w.count}</span><span class="go">${CHEV}</span></li>`
  ).join("") +
  `</ul>`;

const wsTiles = () =>
  `<div class="wtiles">` +
  WS.map(
    (w) => `<div class="wtile${w.live ? " is-live" : ""}"><span class="wtile-name">${w.name}</span><span class="wtile-meta">${w.count} sessions · ${w.desc}</span></div>`
  ).join("") +
  `</div>`;

const seg = (...labels) =>
  `<div class="seg">` + labels.map((l, i) => `<span${i === 0 ? ' class="on"' : ""}>${l}</span>`).join("") + `</div>`;

const sel = (label) => `<button class="sel" type="button" tabindex="-1"><span>${label}</span>${CHEV}</button>`;

const chips = (on = 1) =>
  `<div class="chips">` + ["All", "vela", "atlas-notes", "quiet-web"].map((c, i) => `<span class="chip${i === on ? " on" : ""}">${c}</span>`).join("") + `</div>`;

const stats = () => `<div class="jump"><button class="jmp" type="button" tabindex="-1"><span class="jmp-t"><b>Recent sessions</b><span>12 this week · 2 live</span></span>${CHEV}</button><button class="jmp" type="button" tabindex="-1"><span class="jmp-t"><b>Workspaces</b><span>3 attached</span></span>${CHEV}</button></div>`;

const kick = (t) => `<p class="kick">${t}</p>`;

const cta = (primary = "Open a folder", ghost = null) =>
  `<div class="cta-row"><button class="btn btn-primary" type="button" tabindex="-1">${primary}</button>${ghost ? `<button class="btn btn-ghost" type="button" tabindex="-1">${ghost}</button>` : ""}</div>`;

// ---- registry ---------------------------------------------------------------

const SETS = {};
const set = (code, title, blurb) => { SETS[code] = { code, title, blurb, items: [] }; };
const V = (code, id, name, css, body) => { SETS[code].items.push({ id, name, css, body }); };

set("td", "Twin Desk", "The right desk becomes a switcher: sessions, workspaces, or something stranger.");

// ---- twin desk --------------------------------------------------------------

V("td", "01", "Tab Deck", `.td-01 .mock{display:flex}
.td-01 .left{flex:1;padding:56px 46px;display:flex;flex-direction:column;justify-content:center}
.td-01 .left .mark-svg{width:38px;height:38px;margin-bottom:22px}
.td-01 .left .title{font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-01 .left .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.td-01 .desk{width:300px;background:var(--bg-inset);padding:20px;display:flex;flex-direction:column}`,
`<div class="left">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="desk">${seg("Sessions", "Workspaces")}<div class="rows-wrap">${rowsList({ count: 4 })}</div></div>`);

V("td", "02", "Dropdown Head", `.td-02 .mock{display:flex}
.td-02 .left{flex:1;padding:56px 46px;display:flex;flex-direction:column;justify-content:center}
.td-02 .left .mark-svg{width:38px;height:38px;margin-bottom:22px}
.td-02 .left .title{font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-02 .left .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.td-02 .desk{width:300px;border-left:1px solid var(--border-subtle);padding:24px 22px;display:flex;flex-direction:column;gap:14px}`,
`<div class="left">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${sel("Workspaces")}${wsRows()}${chips(0)}</div>`);

V("td", "03", "Open Drawer", `.td-03 .mock{position:relative;padding:56px 46px}
.td-03 .hero{max-width:330px;display:flex;flex-direction:column;justify-content:center;min-height:100%}
.td-03 .mark-svg{width:38px;height:38px;margin-bottom:22px}
.td-03 .title{font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-03 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.td-03 .drawer{position:absolute;top:44px;bottom:44px;right:-14px;width:310px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;box-shadow:var(--shadow-md);padding:18px;display:flex;flex-direction:column}
.td-03 .handle{width:44px;height:4px;border-radius:999px;background:var(--border-strong);margin:0 auto 12px}`,
`<div class="hero">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="drawer"><span class="handle"></span>${seg("Sessions", "Workspaces")}${wsRows()}</div>`);

V("td", "04", "Rail Switch", `.td-04 .mock{display:flex}
.td-04 .rail{width:56px;background:var(--accent);display:flex;flex-direction:column;align-items:center;gap:18px;padding:24px 0}
.td-04 .rail i{width:22px;height:22px;border-radius:7px;background:rgba(251,247,236,0.28)}
.td-04 .rail i.on{background:#fbf7ec}
.td-04 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-04 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-04 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-04 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-04 .desk{width:270px;border-left:1px solid var(--border-subtle);padding:22px}`,
`<div class="rail">${mark(26)}<i class="on"></i><i></i><i></i></div><div class="left"><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${kick("Workspaces")}${wsRows()}</div>`);

V("td", "05", "Stacked Decks", `.td-05 .mock{display:flex}
.td-05 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-05 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-05 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-05 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-05 .stack{width:290px;padding:36px 26px 26px 0;display:flex;flex-direction:column;gap:-10px}
.td-05 .deck{background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;padding:14px 16px;box-shadow:var(--shadow-sm)}
.td-05 .deck + .deck{margin-top:-8px}
.td-05 .deck.back{background:var(--bg-inset);opacity:0.75}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="stack"><div class="deck">${kick("Sessions")}${rowsList({ count: 3 })}</div><div class="deck back">${kick("Workspaces")}<div class="wtiles">${WS.map((w) => `<div class="wtile"><span class="wtile-name">${w.name}</span></div>`).join("")}</div></div></div>`);

V("td", "06", "Underline Tabs", `.td-06 .mock{display:flex}
.td-06 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center;border-right:1px solid var(--border-subtle)}
.td-06 .mark-svg{width:38px;height:38px;margin-bottom:22px}
.td-06 .title{font-size:50px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-06 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.td-06 .desk{width:290px;padding:26px 22px}
.td-06 .utabs{display:flex;gap:18px;border-bottom:1px solid var(--border-subtle);margin-bottom:6px}
.td-06 .utabs span{padding:8px 2px;font-size:12px;color:var(--text-faint)}
.td-06 .utabs .on{color:var(--text);font-weight:500;box-shadow:inset 0 -2px 0 var(--accent)}`,
`<div class="left">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="utabs"><span class="on">Sessions</span><span>Workspaces <b>3</b></span></div>${rowsList({ count: 4 })}</div>`);

V("td", "07", "Pill Switch", `.td-07 .mock{display:flex}
.td-07 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-07 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-07 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-07 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-07 .desk{width:280px;background:var(--bg-inset);padding:24px 20px;display:flex;flex-direction:column;align-items:center;text-align:center}
.td-07 .big{font-family:var(--serif);font-size:64px;font-weight:500;line-height:1;margin:14px 0 4px}
.td-07 .cap{font-size:9.5px;letter-spacing:0.16em;text-transform:uppercase;color:var(--text-faint);margin-bottom:16px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${seg("Sessions", "Workspaces")}<span class="big">3</span><span class="cap">Workspaces attached</span>${wsRows()}</div>`);

V("td", "08", "Drawer Handle", `.td-08 .mock{position:relative;padding:56px 46px 74px}
.td-08 .hero{max-width:340px}
.td-08 .mark-svg{width:38px;height:38px;margin-bottom:22px}
.td-08 .title{font-size:52px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-08 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.td-08 .shade{position:absolute;left:0;right:0;bottom:0;height:64px;border-top:1px solid var(--border-subtle);background:var(--bg-inset);display:flex;align-items:center;justify-content:center;gap:10px;font-size:11px;color:var(--text-faint)}
.td-08 .grab{width:56px;height:5px;border-radius:999px;background:var(--border-strong)}`,
`<div class="hero">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="shade"><span class="grab"></span><span>3 workspaces · pull to browse</span></div>`);

V("td", "09", "Side Index", `.td-09 .mock{display:flex}
.td-09 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-09 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-09 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-09 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-09 .desk{width:300px;border-left:1px solid var(--border-subtle);display:flex}
.td-09 .idx{width:34px;border-left:1px solid var(--border-subtle);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;font-size:9px;color:var(--text-faint)}
.td-09 .idx .on{color:var(--accent);font-weight:700}
.td-09 .pane{flex:1;padding:22px 18px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="pane">${kick("S · W · R")}${wsRows()}</div><div class="idx"><span>S</span><span class="on">W</span><span>R</span></div></div>`);

V("td", "10", "Split Desk", `.td-10 .mock{display:flex}
.td-10 .left{flex:1;padding:54px 42px;display:flex;flex-direction:column;justify-content:center}
.td-10 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-10 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-10 .sub{margin:0 0 22px;color:var(--text-dim);font-size:13px}
.td-10 .desk{width:300px;border-left:1px solid var(--border-subtle);display:flex;flex-direction:column}
.td-10 .half{flex:1;padding:16px 20px}
.td-10 .half.b{border-top:1px solid var(--border-subtle);background:var(--bg-inset)}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="half">${kick("Live sessions")}${rowsList({ count: 2 })}</div><div class="half b">${kick("Workspaces")}${wsRows()}</div></div>`);

V("td", "11", "Card Carousel", `.td-11 .mock{display:flex;flex-direction:column}
.td-11 .head{display:flex;align-items:center;gap:14px;padding:40px 46px 8px}
.td-11 .head .mark-svg{width:30px;height:30px}
.td-11 .head .title{font-size:30px;font-weight:500;margin:0;letter-spacing:-0.015em}
.td-11 .head .cta-row{margin-left:auto}
.td-11 .strip{margin-top:auto;padding:26px 46px 34px;background:var(--bg-inset);display:flex;gap:14px;overflow:hidden}
.td-11 .wcard{min-width:200px;background:var(--bg-panel);border:1px solid var(--border);border-radius:14px;padding:16px}
.td-11 .wcard.is-live{border-color:var(--accent)}
.td-11 .dots{display:flex;gap:5px;margin-top:14px}
.td-11 .dots i{width:5px;height:5px;border-radius:50%;background:var(--border-strong)}
.td-11 .dots i.on{background:var(--accent);width:14px;border-radius:999px}`,
`<div class="head">${mark(30)}<h1 class="title">Vela</h1>${cta("Open a folder")}</div><div class="strip">${WS.concat([{ name: "new-workspace", count: 0, desc: "Drop a folder", live: false }]).map((w) => `<div class="wcard${w.live ? " is-live" : ""}"><span class="wtile-name">${w.name}</span><span class="wtile-meta">${w.count} sessions · ${w.desc}</span></div>`).join("")}</div>`);

V("td", "12", "Accordion", `.td-12 .mock{display:flex}
.td-12 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-12 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-12 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-12 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-12 .desk{width:310px;border-left:1px solid var(--border-subtle);padding:20px}
.td-12 .acc{border:1px solid var(--border);border-radius:12px;overflow:hidden}
.td-12 .acc-head{display:flex;align-items:center;gap:9px;padding:11px 13px;font-size:12px;font-weight:500;background:var(--bg)}
.td-12 .acc-head .n{margin-left:auto;color:var(--text-faint);font-size:10.5px}
.td-12 .acc-body{border-top:1px solid var(--border-subtle);background:var(--bg-panel);padding:4px 13px 8px 30px}
.td-12 .acc + .acc{margin-top:8px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="acc"><div class="acc-head">${CHEV}${WS[0].name}<span class="n">${WS[0].count}</span></div><div class="acc-body">${rowsList({ count: 2 })}</div></div>${WS.slice(1).map((w) => `<div class="acc"><div class="acc-head">${CHEV}${w.name}<span class="n">${w.count}</span></div></div>`).join("")}</div>`);

V("td", "13", "Tree Desk", `.td-13 .mock{display:flex}
.td-13 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-13 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-13 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-13 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-13 .desk{width:300px;border-left:1px solid var(--border-subtle);padding:20px;font-size:12px}
.td-13 .tree-row{display:flex;align-items:center;gap:8px;padding:8px 6px;border-radius:8px}
.td-13 .tree-row.ws{font-weight:500}
.td-13 .tree-row.ses{padding-left:28px;color:var(--text-dim)}
.td-13 .tree-row .cv{color:var(--text-faint);display:inline-flex}
.td-13 .tree-row .live{width:6px;height:6px;border-radius:50%;background:var(--accent);margin-left:auto}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${WS.map((w) => `<div class="tree-row ws"><span class="cv">${CHEV}</span>${w.name}</div>${w.name === "vela" ? RECENTS.slice(0, 2).map((r) => `<div class="tree-row ses"><span>${r.name}</span>${r.live ? '<span class="live"></span>' : ""}</div>`).join("") : ""}`).join("")}</div>`);

V("td", "14", "Chip Cloud", `.td-14 .mock{padding:56px 46px;display:flex;flex-direction:column}
.td-14 .hero{display:flex;align-items:flex-end;gap:28px}
.td-14 .hero-l{flex:1}
.td-14 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.td-14 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.td-14 .sub{margin:0;color:var(--text-dim);font-size:13px}
.td-14 .list{margin-top:26px;border-top:1px solid var(--border-subtle);padding-top:14px}`,
`<div class="hero"><div class="hero-l">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p></div>${cta("Open a folder")}</div>${chips(2)}<div class="list">${rowsList({ count: 4 })}</div>`);

V("td", "15", "Two-Up Toggle", `.td-15 .mock{display:flex}
.td-15 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-15 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-15 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-15 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-15 .desk{width:300px;background:var(--bg-inset);padding:20px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${seg("Sessions", "Workspaces")}${wsTiles()}</div>`);

V("td", "16", "Ledger Switch", `.td-16 .mock{display:flex}
.td-16 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center;border-right:1px solid var(--border-subtle)}
.td-16 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-16 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-16 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-16 .desk{width:300px;padding:24px 22px;font-family:ui-monospace,"SF Mono",Menlo,monospace;font-size:11px}
.td-16 .ltabs{display:flex;gap:14px;margin-bottom:10px;font-size:10px;letter-spacing:0.12em}
.td-16 .ltabs span{color:var(--text-faint)}
.td-16 .ltabs .on{color:var(--accent);border-bottom:1px solid var(--accent);padding-bottom:2px}
.td-16 .lrow{display:flex;gap:8px;padding:7px 0;border-bottom:1px dotted var(--border-subtle)}
.td-16 .lrow b{font-weight:500;color:var(--text)}
.td-16 .lrow span{margin-left:auto;color:var(--text-faint)}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="ltabs"><span class="on">SESS</span><span>WKSP</span></div>${[["01", "Refine sessions panel", "now"], ["02", "Provider usage cleanup", "18m"], ["03", "Update runtime docs", "1d"], ["04", "Polish daily notes", "2h"]].map(([i, n, w]) => `<div class="lrow">${i} <b>${n}</b><span>${w}</span></div>`).join("")}</div>`);

V("td", "17", "Corner Fold", `.td-17 .mock{position:relative;padding:56px 46px}
.td-17 .hero{max-width:340px}
.td-17 .mark-svg{width:38px;height:38px;margin-bottom:22px}
.td-17 .title{font-size:52px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-17 .sub{margin:0 0 26px;color:var(--text-dim);font-size:13px}
.td-17 .fold{position:absolute;top:0;right:0;width:190px;height:190px;background:var(--bg-inset);border-left:1px solid var(--border);border-bottom:1px solid var(--border);border-radius:0 0 0 100%;padding:26px 20px 20px 34px}
.td-17 .fold .tri{position:absolute;top:0;right:0;width:56px;height:56px;background:var(--bg-panel);border-left:1px solid var(--border-strong);border-bottom:1px solid var(--border-strong);border-radius:0 0 0 100%}`,
`<div class="hero">${mark(38)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="fold"><span class="tri"></span>${kick("Workspaces")}${WS.map((w) => `<div class="fold-ws">${w.name} · ${w.count}</div>`).join("")}</div>`);

V("td", "18", "Slide Tabs", `.td-18 .mock{display:flex}
.td-18 .vtabs{width:44px;border-right:1px solid var(--border-subtle);display:flex;flex-direction:column;align-items:center;padding:18px 0;gap:14px}
.td-18 .vtabs span{writing-mode:vertical-rl;font-size:9.5px;letter-spacing:0.2em;text-transform:uppercase;color:var(--text-faint);padding:8px 3px;border-radius:999px}
.td-18 .vtabs .on{background:var(--accent);color:var(--on-accent)}
.td-18 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-18 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-18 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-18 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-18 .desk{width:290px;background:var(--bg-inset);padding:22px 20px}`,
`<div class="vtabs"><span class="on">Sessions</span><span>Workspaces</span><span>Pulse</span></div><div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${rowsList({ count: 4 })}</div>`);

V("td", "19", "Dial Switch", `.td-19 .mock{display:flex}
.td-19 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-19 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-19 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-19 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-19 .desk{width:300px;border-left:1px solid var(--border-subtle);padding:22px;display:flex;flex-direction:column;align-items:center}
.td-19 .dial{position:relative;width:92px;height:92px;border:1px dashed var(--border-strong);border-radius:50%;display:grid;place-items:center;margin-bottom:14px}
.td-19 .dial::after{content:"";position:absolute;top:6px;left:50%;width:5px;height:5px;border-radius:50%;background:var(--accent)}
.td-19 .dial b{font-family:var(--serif);font-size:26px;font-weight:500}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="dial"><b>W</b></div>${kick("Workspaces · 3 attached")}${wsRows()}</div>`);

V("td", "20", "Book Spine", `.td-20 .mock{display:flex}
.td-20 .spine{width:40px;background:var(--accent);display:flex;align-items:center;justify-content:center}
.td-20 .spine span{writing-mode:vertical-rl;color:#fbf7ec;font-size:10px;letter-spacing:0.3em;text-transform:uppercase}
.td-20 .page{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-20 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-20 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-20 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-20 .leaf{width:290px;border-left:1px solid var(--border-subtle);box-shadow:inset 12px 0 18px -14px rgba(67,48,33,0.25);padding:24px 22px}`,
`<div class="spine"><span>Vela</span></div><div class="page">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="leaf">${seg("Contents", "Index")}${wsRows()}</div>`);

V("td", "21", "Hover Peek", `.td-21 .mock{display:flex}
.td-21 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-21 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-21 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-21 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-21 .desk{width:290px;border-left:1px solid var(--border-subtle);padding:20px;display:flex;flex-direction:column}
.td-21 .peek{margin-top:auto;padding:10px 12px;border:1px dashed var(--border-strong);border-radius:10px;font-size:11px;color:var(--text-faint);display:flex;align-items:center;gap:8px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="rows-wrap">${rowsList({ count: 3 })}</div><div class="peek">${CHEV} Workspaces desk · 3 more below</div></div>`);

V("td", "22", "Top Bar Switch", `.td-22 .mock{display:flex;flex-direction:column}
.td-22 .bar{display:flex;align-items:center;gap:16px;padding:14px 22px;border-bottom:1px solid var(--border-subtle)}
.td-22 .bar .mark-svg{width:22px;height:22px}
.td-22 .bar .brand{font-family:var(--serif);font-size:18px;font-weight:500}
.td-22 .bar .seg{margin-left:auto}
.td-22 .main{flex:1;display:flex}
.td-22 .hero{flex:1;padding:44px 46px;display:flex;flex-direction:column;justify-content:center}
.td-22 .title{font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-22 .sub{margin:0 0 22px;color:var(--text-dim);font-size:13px}
.td-22 .side{width:260px;border-left:1px solid var(--border-subtle);padding:20px}`,
`<div class="bar">${mark(22)}<span class="brand">Vela</span>${seg("Sessions", "Workspaces", "Pulse")}</div><div class="main"><div class="hero"><h1 class="title">Every agent. One surface.</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="side">${kick("Workspaces")}${wsRows()}</div></div>`);

V("td", "23", "Ghost Panel", `.td-23 .mock{position:relative;padding:56px 46px}
.td-23 .hero{max-width:320px}
.td-23 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-23 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-23 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-23 .float{position:absolute;right:60px;top:70px;width:280px;background:color-mix(in srgb,var(--bg-panel) 82%,transparent);backdrop-filter:blur(2px);border:1px solid var(--border-strong);border-radius:16px;box-shadow:var(--shadow-md);padding:16px;transform:rotate(-1.2deg)}
.td-23 .float .grip{display:flex;gap:4px;margin-bottom:10px}
.td-23 .float .grip i{width:4px;height:4px;border-radius:50%;background:var(--border-strong)}`,
`<div class="hero">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="float"><div class="grip"><i></i><i></i><i></i></div>${sel("Workspaces · all")}${wsRows()}</div>`);

V("td", "24", "Ladder", `.td-24 .mock{position:relative;padding:56px 210px 56px 46px}
.td-24 .hero{max-width:330px}
.td-24 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-24 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-24 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-24 .rung{position:absolute;right:46px;height:52px;border:1px solid var(--border);border-radius:10px;background:var(--bg-panel);display:flex;align-items:center;gap:10px;padding:0 16px;font-size:12px;font-weight:500}
.td-24 .rung small{margin-left:auto;color:var(--text-faint);font-weight:400}
.td-24 .r1{top:64px;width:230px}
.td-24 .r2{top:130px;width:200px}
.td-24 .r3{top:196px;width:170px}
.td-24 .rung.is-live{border-color:var(--accent)}`,
`<div class="hero">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="rung r1 is-live">vela<small>4</small></div><div class="rung r2">atlas-notes<small>2</small></div><div class="rung r3">quiet-web<small>2</small></div>`);

V("td", "25", "Vela Map", `.td-25 .mock{display:flex}
.td-25 .left{flex:1;position:relative;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-25 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.td-25 .title{font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-25 .sub{margin:0 0 22px;color:var(--text-dim);font-size:13px;max-width:26ch}
.td-25 .map{position:absolute;right:26px;bottom:26px;width:190px;height:190px}
.td-25 .map circle.o{fill:none;stroke:var(--border-strong)}
.td-25 .map circle.w{fill:var(--bg-panel);stroke:var(--accent)}
.td-25 .desk{width:240px;border-left:1px solid var(--border-subtle);padding:20px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<svg class="map" viewBox="0 0 100 100"><circle class="o" cx="50" cy="50" r="38"/><circle class="o" cx="50" cy="50" r="22"/><circle cx="50" cy="50" r="7" fill="#46584b"/><circle class="w" cx="81" cy="35" r="6"/><circle class="w" cx="33" cy="76" r="6"/><circle cx="29" cy="29" r="4" fill="var(--accent)"/></svg></div><div class="desk">${kick("In vela")}${wsRows()}</div>`);

V("td", "26", "File Cabinet", `.td-26 .mock{display:flex}
.td-26 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-26 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-26 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-26 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-26 .cab{width:280px;border-left:1px solid var(--border-subtle);display:flex;flex-direction:column;justify-content:center;padding:0 22px;gap:12px}
.td-26 .drawer-f{height:74px;border:1px solid var(--border-strong);border-radius:10px;background:var(--bg-inset);display:flex;align-items:center;padding:0 16px;gap:10px;font-size:12px;font-weight:500}
.td-26 .handle{width:34px;height:6px;border-radius:999px;background:var(--border-strong)}
.td-26 .drawer-f.is-open{background:var(--bg-panel);border-color:var(--accent);flex-direction:column;align-items:flex-start;justify-content:center;gap:8px;height:110px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="cab"><div class="drawer-f is-open"><span>Sessions — vela</span><div class="rows-wrap" style="width:100%">${rowsList({ count: 2 })}</div></div><div class="drawer-f"><span class="handle"></span>Workspaces<span style="margin-left:auto;color:var(--text-faint)">3</span></div></div>`);

V("td", "27", "Spring Stack", `.td-27 .mock{display:flex}
.td-27 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-27 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-27 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-27 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-27 .spring{width:300px;position:relative;padding:30px 0}
.td-27 .card{position:relative;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:16px;box-shadow:var(--shadow-sm)}
.td-27 .card.b1{transform:translate(14px,-26px) rotate(2deg);opacity:0.65;z-index:0}
.td-27 .card.b2{transform:translate(-12px,-14px) rotate(-1.6deg);opacity:0.85;z-index:1}
.td-27 .card.top{z-index:2}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="spring"><div class="card top">${seg("Sessions", "Workspaces")}${rowsList({ count: 3 })}</div><div class="card b1"></div><div class="card b2">${kick("peek")}</div></div>`);

V("td", "28", "Console Switch", `.td-28 .mock{display:flex}
.td-28 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-28 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-28 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-28 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-28 .term{width:320px;background:var(--bg-inset);border-left:1px solid var(--border-subtle);padding:22px;font-family:ui-monospace,"SF Mono",Menlo,monospace;font-size:11px}
.td-28 .prompt{color:var(--text-dim);margin-bottom:10px}
.td-28 .prompt b{color:var(--text);font-weight:500}
.td-28 .caret{display:inline-block;width:7px;height:12px;background:var(--accent);vertical-align:-2px}
.td-28 .pop{border:1px solid var(--border-strong);border-radius:10px;background:var(--bg-panel);margin-top:10px;overflow:hidden}
.td-28 .pop-h{padding:6px 12px;font-size:9px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);border-bottom:1px solid var(--border-subtle)}
.td-28 .pop-i{display:flex;gap:8px;padding:7px 12px}
.td-28 .pop-i.on{background:var(--accent-dim)}
.td-28 .pop-i span{margin-left:auto;color:var(--text-faint)}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="term"><div class="prompt">$ vela use <b>w</b><span class="caret"></span></div><div class="pop"><div class="pop-h">workspaces</div><div class="pop-i on">vela<span>4</span></div><div class="pop-i">atlas-notes<span>2</span></div><div class="pop-i">quiet-web<span>2</span></div></div></div>`);

V("td", "29", "Curtain Reveal", `.td-29 .mock{display:flex}
.td-29 .stage{flex:1;position:relative;padding:56px 300px 56px 44px}
.td-29 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-29 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-29 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-29 .curtain{position:absolute;top:0;bottom:0;right:0;width:250px;background:repeating-linear-gradient(90deg,var(--bg-inset) 0 26px,color-mix(in srgb,var(--text) 4%,transparent) 26px 28px);border-left:1px solid var(--border);transform-origin:right;}
.td-29 .curtain .edge{position:absolute;left:-1px;top:0;bottom:0;width:8px;background:var(--accent);opacity:0.5}`,
`<div class="stage">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}<div class="curtain"><span class="edge"></span></div></div>`);

V("td", "30", "Magnet Tabs", `.td-30 .mock{display:flex}
.td-30 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-30 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-30 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-30 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-30 .desk{width:290px;border-left:1px solid var(--border-subtle);padding:0 18px 20px}
.td-30 .mags{display:flex;gap:6px;padding:14px 0}
.td-30 .mag{padding:7px 13px;border-radius:9px 9px 0 0;border:1px solid var(--border);border-bottom:0;background:var(--bg-inset);font-size:11px;color:var(--text-dim)}
.td-30 .mag.on{background:var(--bg-panel);border-color:var(--border-strong);color:var(--text);font-weight:500;transform:translateY(-4px);box-shadow:var(--shadow-sm)}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="mags"><span class="mag on">Sessions</span><span class="mag">Workspaces</span><span class="mag">All</span></div>${rowsList({ count: 4 })}</div>`);

V("td", "31", "Breadcrumb Desk", `.td-31 .mock{display:flex}
.td-31 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center;border-right:1px solid var(--border-subtle)}
.td-31 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-31 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-31 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-31 .desk{width:300px;padding:20px 22px;background:var(--bg-inset)}
.td-31 .crumbs{display:flex;align-items:center;gap:6px;font-size:11px;color:var(--text-faint);margin-bottom:12px}
.td-31 .crumbs b{color:var(--text);font-weight:500}
.td-31 .crumbs .sep{opacity:0.5}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="crumbs">Vela <span class="sep">/</span> ${sel("quiet-web")}</div>${rowsList({ count: 3 })}${stats()}</div>`);

V("td", "32", "Counter Tabs", `.td-32 .mock{display:flex}
.td-32 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-32 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-32 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-32 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-32 .desk{width:290px;border-left:1px solid var(--border-subtle);padding:22px 20px}
.td-32 .ctabs{display:flex;gap:8px;margin-bottom:14px}
.td-32 .ctab{flex:1;text-align:center;padding:10px 6px;border:1px solid var(--border);border-radius:12px;font-size:10.5px;color:var(--text-faint)}
.td-32 .ctab b{display:block;font-family:var(--serif);font-size:22px;font-weight:500;color:var(--text);margin-bottom:2px}
.td-32 .ctab.on{border-color:var(--accent);background:var(--accent-dim)}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk"><div class="ctabs"><span class="ctab on"><b>5</b>sessions</span><span class="ctab"><b>3</b>workspaces</span></div>${rowsList({ count: 4 })}</div>`);

V("td", "33", "Zipper", `.td-33 .mock{display:flex}
.td-33 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-33 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-33 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-33 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-33 .tear{width:0;border-left:2px dashed var(--border-strong)}
.td-33 .stub{width:150px;background:var(--bg-inset);padding:56px 16px;display:flex;flex-direction:column;gap:10px}
.td-33 .stub div{font-size:11px;color:var(--text-dim);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="tear"></div><div class="stub">${kick("next desk")}<div>▸ workspaces</div><div>▸ pulse</div><div>▸ archive</div></div>`);

V("td", "34", "Slot Machine", `.td-34 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:48px}
.td-34 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.td-34 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.td-34 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-34 .reels{display:flex;gap:10px;margin-bottom:24px}
.td-34 .reel{width:150px;height:120px;border:1px solid var(--border-strong);border-radius:12px;background:var(--bg-inset);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:4px}
.td-34 .reel b{font-size:14px;font-weight:500}
.td-34 .reel span{font-size:10px;color:var(--text-faint)}
.td-34 .lever{width:4px;height:34px;background:var(--border-strong);border-radius:999px;margin:0 auto;position:relative}
.td-34 .lever::after{content:"";position:absolute;top:-8px;left:-6px;width:16px;height:16px;border-radius:50%;background:var(--accent)}`,
`${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p><div class="reels"><div class="reel"><b>vela</b><span>workspace</span></div><div class="reel"><b>Refine sessions</b><span>session</span></div><div class="reel"><b>Open</b><span>action</span></div></div><div class="lever"></div>${cta("Pull")}`);

V("td", "35", "Compass Drawer", `.td-35 .mock{position:relative;padding:56px 46px}
.td-35 .hero{max-width:330px}
.td-35 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-35 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-35 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-35 .comp{position:absolute;top:40px;right:40px;width:74px;height:74px;border:1px solid var(--border-strong);border-radius:50%;display:grid;place-items:center;background:var(--bg-panel);box-shadow:var(--shadow-sm)}
.td-35 .comp::before{content:"";width:2px;height:30px;background:linear-gradient(var(--accent) 50%,var(--border-strong) 50%)}
.td-35 .quad{position:absolute;right:40px;top:130px;width:230px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;box-shadow:var(--shadow-md);padding:14px 16px}`,
`<div class="hero">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="comp"></div><div class="quad">${kick("NE · workspaces")}${wsRows()}</div>`);

V("td", "36", "Sticky Swap", `.td-36 .mock{display:flex}
.td-36 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-36 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-36 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-36 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-36 .board{width:300px;background:var(--bg-inset);padding:20px;display:flex;flex-wrap:wrap;gap:12px;align-content:flex-start}
.td-36 .note{width:calc(50% - 6px);aspect-ratio:1;background:var(--bg-panel);border:1px solid var(--border);border-radius:4px 12px 4px 4px;padding:10px;font-size:10.5px;line-height:1.35;box-shadow:var(--shadow-sm)}
.td-36 .note:nth-child(odd){transform:rotate(-1.4deg)}
.td-36 .note:nth-child(even){transform:rotate(1.1deg)}
.td-36 .note b{display:block;font-size:11.5px;margin-bottom:3px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="board"><div class="note"><b>vela</b>4 sessions · live</div><div class="note"><b>Refine sessions</b>panel polish</div><div class="note"><b>atlas-notes</b>2 sessions</div><div class="note"><b>Daily notes</b>pipeline</div><div class="note"><b>quiet-web</b>site refresh</div><div class="note" style="border-style:dashed;color:var(--text-faint)">+ pin a note</div></div>`);

V("td", "37", "Time Scrub", `.td-37 .mock{display:flex;flex-direction:column}
.td-37 .main{flex:1;display:flex}
.td-37 .hero{flex:1;padding:48px 46px;display:flex;flex-direction:column;justify-content:center}
.td-37 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.td-37 .title{font-size:44px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-37 .sub{margin:0 0 20px;color:var(--text-dim);font-size:13px}
.td-37 .side{width:270px;border-left:1px solid var(--border-subtle);padding:20px}
.td-37 .scrub{border-top:1px solid var(--border-subtle);padding:16px 46px 20px;background:var(--bg-inset)}
.td-37 .track{position:relative;height:4px;border-radius:999px;background:var(--border-strong);margin:12px 0 8px}
.td-37 .thumb{position:absolute;top:50%;left:72%;width:14px;height:14px;border-radius:50%;background:var(--accent);transform:translate(-50%,-50%);border:3px solid var(--bg-panel);box-shadow:var(--shadow-sm)}
.td-37 .days{display:flex;justify-content:space-between;font-size:9.5px;color:var(--text-faint);letter-spacing:0.06em}`,
`<div class="main"><div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="side">${kick("Thursday")}${rowsList({ count: 3 })}</div></div><div class="scrub"><div class="days"><span>Mon</span><span>Tue</span><span>Wed</span><span>Thu</span><span>Fri</span></div><div class="track"><span class="thumb"></span></div></div>`);

V("td", "38", "Kanban Mini", `.td-38 .mock{display:flex}
.td-38 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-38 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-38 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-38 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-38 .desk{width:320px;border-left:1px solid var(--border-subtle);padding:18px;display:flex;flex-direction:column}
.td-38 .cols{display:flex;gap:10px;flex:1}
.td-38 .col{flex:1;background:var(--bg-inset);border-radius:10px;padding:10px}
.td-38 .col-h{font-size:9.5px;letter-spacing:0.12em;text-transform:uppercase;color:var(--text-faint);margin-bottom:8px}
.td-38 .kcard{background:var(--bg-panel);border:1px solid var(--border);border-radius:8px;padding:8px 10px;font-size:11px;margin-bottom:8px}
.td-38 .kcard i{display:block;font-style:normal;color:var(--text-faint);font-size:9.5px;margin-top:2px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${seg("By state", "By workspace")}<div class="cols"><div class="col"><div class="col-h">Live</div><div class="kcard">Refine sessions<i>vela</i></div><div class="kcard">Usage cleanup<i>vela</i></div></div><div class="col"><div class="col-h">Today</div><div class="kcard">Daily notes<i>atlas</i></div><div class="kcard">Quick search<i>atlas</i></div></div></div></div>`);

V("td", "39", "Command Palette", `.td-39 .mock{position:relative;padding:56px 46px}
.td-39 .hero{max-width:320px}
.td-39 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-39 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-39 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-39 .pal{position:absolute;right:54px;top:58px;width:300px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;box-shadow:var(--shadow-md);overflow:hidden}
.td-39 .pin{display:flex;align-items:center;gap:8px;padding:11px 14px;border-bottom:1px solid var(--border-subtle);font-size:12px;color:var(--text-dim)}
.td-39 .pin .cc{margin-left:auto;font-size:9.5px;border:1px solid var(--border-strong);border-radius:5px;padding:1px 5px;color:var(--text-faint)}
.td-39 .grp{padding:6px 14px;font-size:9px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);background:var(--bg)}
.td-39 .pi{display:flex;gap:8px;padding:7px 14px;font-size:11.5px}
.td-39 .pi.on{background:var(--accent-dim)}
.td-39 .pi span{margin-left:auto;color:var(--text-faint);font-size:10px}`,
`<div class="hero">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="pal"><div class="pin">Jump to… <span class="cc">⌘K</span></div><div class="grp">workspaces</div><div class="pi on">vela<span>4</span></div><div class="pi">atlas-notes<span>2</span></div><div class="grp">sessions</div><div class="pi">Polish daily notes<span>2h</span></div></div>`);

V("td", "40", "Badge Dock", `.td-40 .mock{display:flex;flex-direction:column}
.td-40 .hero{flex:1;padding:52px 46px 30px;display:flex;flex-direction:column;justify-content:center}
.td-40 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-40 .title{font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-40 .sub{margin:0 0 22px;color:var(--text-dim);font-size:13px}
.td-40 .dock{border-top:1px solid var(--border-subtle);background:var(--bg-inset);padding:14px 46px 18px;display:flex;align-items:flex-end;gap:14px}
.td-40 .di{width:52px;height:52px;border-radius:14px;background:var(--bg-panel);border:1px solid var(--border-strong);display:grid;place-items:center;font-size:10px;color:var(--text-faint);position:relative}
.td-40 .di.big{width:66px;height:66px;border-color:var(--accent);transform:translateY(-10px);color:var(--text)}
.td-40 .bubble{position:absolute;top:-30px;left:50%;transform:translateX(-50%);background:var(--accent);color:var(--on-accent);font-size:9.5px;padding:3px 8px;border-radius:999px;white-space:nowrap}`,
`<div class="hero">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="dock"><div class="di big">vela<span class="bubble">4 sessions</span></div><div class="di">atlas</div><div class="di">web</div><div class="di" style="border-style:dashed">+</div></div>`);

V("td", "41", "Origami Fold", `.td-41 .mock{display:flex}
.td-41 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-41 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-41 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-41 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-41 .fan{width:300px;display:flex;height:100%}
.td-41 .pleat{flex:1;border-left:1px solid var(--border);display:flex;align-items:flex-end;padding:18px 8px;background:linear-gradient(105deg,var(--bg-panel) 0 60%,var(--bg-inset) 60%)}
.td-41 .pleat:first-child{border-left:0}
.td-41 .pleat.on{background:var(--bg-panel);box-shadow:inset 3px 0 0 var(--accent)}
.td-41 .pleat span{writing-mode:vertical-rl;font-size:9.5px;letter-spacing:0.18em;text-transform:uppercase;color:var(--text-faint)}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="fan"><div class="pleat on"><span>Sessions</span></div><div class="pleat"><span>Workspaces</span></div><div class="pleat"><span>Pulse</span></div><div class="pleat"><span>Archive</span></div></div>`);

V("td", "42", "Radio List", `.td-42 .mock{display:flex}
.td-42 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-42 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-42 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-42 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-42 .desk{width:280px;border-left:1px solid var(--border-subtle);padding:24px 22px}
.td-42 .radio{display:flex;align-items:center;gap:10px;padding:10px 4px;font-size:12.5px;border-bottom:1px solid var(--border-subtle)}
.td-42 .radio:last-of-type{border-bottom:0}
.td-42 .dot{width:14px;height:14px;border-radius:50%;border:1.5px solid var(--border-strong);display:grid;place-items:center}
.td-42 .radio.on .dot::after{content:"";width:7px;height:7px;border-radius:50%;background:var(--accent)}
.td-42 .radio small{margin-left:auto;color:var(--text-faint);font-size:10px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="desk">${kick("Show me")}<div class="radio on"><span class="dot"></span>Sessions<small>5</small></div><div class="radio"><span class="dot"></span>Workspaces<small>3</small></div><div class="radio"><span class="dot"></span>Everything<small>8</small></div><div class="rows-wrap" style="margin-top:14px">${rowsList({ count: 2 })}</div></div>`);

V("td", "43", "Window Shade", `.td-43 .mock{position:relative;padding:56px 46px}
.td-43 .hero{max-width:330px}
.td-43 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-43 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-43 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-43 .roller{position:absolute;top:0;right:36px;bottom:120px;width:240px;border:1px solid var(--border);border-top:0;border-radius:0 0 12px 12px;background:var(--bg-inset);box-shadow:var(--shadow-md)}
.td-43 .roller .bar{position:absolute;top:-14px;left:-8px;right:-8px;height:12px;border-radius:999px;background:var(--accent)}
.td-43 .cord{position:absolute;right:8px;top:6px;width:2px;height:120px;background:var(--border-strong)}
.td-43 .cord::after{content:"";position:absolute;bottom:-10px;left:-4px;width:10px;height:14px;border-radius:3px;background:var(--accent)}`,
`<div class="hero">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="roller"><span class="bar"></span><div style="padding:18px">${kick("workspaces")}<div class="wtiles">${WS.map((w) => `<div class="wtile"><span class="wtile-name">${w.name}</span></div>`).join("")}</div></div><span class="cord"></span></div>`);

V("td", "44", "Split Flap", `.td-44 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:48px;text-align:center}
.td-44 .mark-svg{width:32px;height:32px;margin-bottom:16px}
.td-44 .title{font-size:40px;font-weight:500;margin:0 0 4px;letter-spacing:-0.02em}
.td-44 .sub{margin:0 0 24px;color:var(--text-dim);font-size:12.5px}
.td-44 .board{display:flex;flex-direction:column;gap:6px;width:420px}
.td-44 .flaprow{display:flex;gap:6px;justify-content:center}
.td-44 .flap{flex:1;height:34px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:6px;display:flex;align-items:center;justify-content:center;font-family:ui-monospace,"SF Mono",Menlo,monospace;font-size:11px;position:relative}
.td-44 .flap::after{content:"";position:absolute;left:0;right:0;top:50%;height:1px;background:color-mix(in srgb,var(--text) 14%,transparent)}
.td-44 .flap.on{background:var(--accent);color:var(--on-accent);border-color:var(--accent)}`,
`${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p><div class="board"><div class="flaprow"><span class="flap">SURFACE</span><span class="flap on">WORKSPACES</span><span class="flap">3 FOUND</span></div><div class="flaprow"><span class="flap">vela</span><span class="flap">atlas-notes</span><span class="flap">quiet-web</span></div></div>${cta("Enter", "Open a folder")}`);

V("td", "45", "Grid Lens", `.td-45 .mock{position:relative;padding:56px 46px}
.td-45 .hero{max-width:300px}
.td-45 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.td-45 .title{font-size:44px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-45 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}
.td-45 .field{position:absolute;right:0;top:0;bottom:0;width:330px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:34px 34px;display:grid;grid-template-columns:repeat(auto-fill,34px);grid-auto-rows:34px;align-content:end;justify-content:end;padding:20px;gap:0}
.td-45 .cell{border-radius:6px;display:grid;place-items:center;font-size:8px;color:var(--text-faint)}
.td-45 .cell.hot{outline:2px solid var(--accent);outline-offset:-2px;background:var(--accent-dim);color:var(--text);font-weight:600;font-size:9px}`,
`<div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="field">${["vela·4", "", "", "", "", "atlas·2", "", "web·2", "", ""].map((v, i) => `<span class="cell${i === 0 || i === 5 || i === 7 ? " hot" : ""}">${v}</span>`).join("")}</div>`);

V("td", "46", "Ticket Stub", `.td-46 .mock{display:flex;align-items:center;padding:48px;gap:0}
.td-46 .ticket{flex:1;display:flex;border:1px solid var(--border-strong);border-radius:16px;background:var(--bg-panel);overflow:hidden;box-shadow:var(--shadow-sm)}
.td-46 .main-s{flex:1;padding:34px 36px}
.td-46 .mark-svg{width:32px;height:32px;margin-bottom:16px}
.td-46 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.td-46 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}
.td-46 .perf{width:0;border-left:2px dashed var(--border-strong);position:relative}
.td-46 .perf::before,.td-46 .perf::after{content:"";position:absolute;left:-11px;width:20px;height:20px;border-radius:50%;background:var(--bg)}
.td-46 .perf::before{top:-10px}
.td-46 .perf::after{bottom:-10px}
.td-46 .stub-s{width:190px;background:var(--bg-inset);padding:26px 20px;display:flex;flex-direction:column;gap:10px}`,
`<div class="ticket"><div class="main-s">${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="perf"></div><div class="stub-s">${kick("admit: workspaces")}${wsRows()}${stats()}</div></div>`);

V("td", "47", "Layer Cake", `.td-47 .mock{display:flex;flex-direction:column;align-items:center;padding:44px 48px 36px;text-align:center}
.td-47 .mark-svg{width:30px;height:30px;margin-bottom:14px}
.td-47 .title{font-size:40px;font-weight:500;margin:0 0 4px;letter-spacing:-0.02em}
.td-47 .sub{margin:0 0 22px;color:var(--text-dim);font-size:12.5px}
.td-47 .layers{display:flex;flex-direction:column;gap:10px;width:380px;margin-bottom:22px}
.td-47 .layer{border:1px solid var(--border-strong);border-radius:12px;padding:12px 16px;background:var(--bg-panel);display:flex;align-items:center;gap:12px;font-size:12px;font-weight:500}
.td-47 .layer:nth-child(2){transform:scale(0.94);opacity:0.8;background:var(--bg-inset)}
.td-47 .layer:nth-child(3){transform:scale(0.88);opacity:0.6;background:var(--bg)}
.td-47 .layer .tag{margin-left:auto;font-size:9.5px;color:var(--text-faint);letter-spacing:0.1em;text-transform:uppercase}`,
`${mark(30)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p><div class="layers"><div class="layer">${mark(16)}Sessions layer<span class="tag">front</span></div><div class="layer">Workspaces<span class="tag">mid</span></div><div class="layer">Archive<span class="tag">deep</span></div></div>${cta("Open a folder")}`);

V("td", "48", "Anchor Pins", `.td-48 .mock{display:flex}
.td-48 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center;border-right:1px solid var(--border-subtle)}
.td-48 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-48 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-48 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-48 .board{width:300px;padding:24px 20px;display:grid;grid-template-columns:1fr 1fr;gap:14px;background:var(--bg-inset)}
.td-48 .pinned{position:relative;background:var(--bg-panel);border:1px solid var(--border);border-radius:10px;padding:14px 12px 10px;font-size:11px;box-shadow:var(--shadow-sm)}
.td-48 .pinned::before{content:"";position:absolute;top:-5px;left:50%;transform:translateX(-50%);width:9px;height:9px;border-radius:50%;background:var(--accent);box-shadow:0 1px 2px rgba(67,48,33,0.4)}
.td-48 .pinned b{display:block;font-size:12px;margin-bottom:2px}
.td-48 .pinned i{font-style:normal;color:var(--text-faint);font-size:9.5px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="board">${[["vela", "4 sessions · live"], ["Refine sessions", "just now"], ["atlas-notes", "2 sessions"], ["Daily notes", "2h ago"]].map(([b, i]) => `<div class="pinned"><b>${b}</b><i>${i}</i></div>`).join("")}</div>`);

V("td", "49", "Metronome", `.td-49 .mock{display:flex}
.td-49 .left{flex:1;padding:56px 44px;display:flex;flex-direction:column;justify-content:center}
.td-49 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.td-49 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.td-49 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}
.td-49 .metro{width:280px;border-left:1px solid var(--border-subtle);padding:24px 22px;display:flex;flex-direction:column;align-items:center}
.td-49 .arm{width:3px;height:90px;background:linear-gradient(to top,var(--border-strong) 60%,var(--accent));border-radius:999px;transform:rotate(18deg);transform-origin:bottom;margin-bottom:4px}
.td-49 .pivot{width:44px;height:10px;border-radius:999px;background:var(--border-strong);margin-bottom:18px}
.td-49 .scale{display:flex;justify-content:space-between;width:100%;font-size:9px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint);border-top:1px solid var(--border-subtle);padding-top:10px}`,
`<div class="left">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="metro"><span class="arm"></span><span class="pivot"></span><div class="scale"><span>sess</span><span>wksp</span><span>pulse</span></div>${kick("swinging toward workspaces")}${wsRows()}</div>`);

V("td", "50", "Portal Pair", `.td-50 .mock{display:flex;align-items:stretch;justify-content:center;gap:26px;padding:52px 48px}
.td-50 .intro{position:absolute;top:44px;left:48px}
.td-50 .intro .mark-svg{width:30px;height:30px;margin-bottom:12px}
.td-50 .intro .title{font-size:30px;font-weight:500;margin:0;letter-spacing:-0.015em}
.td-50 .portal{width:170px;border:1px solid var(--border-strong);border-radius:999px 999px 14px 14px;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:0 14px 18px;position:relative;margin-top:70px}
.td-50 .portal .lbl{font-size:11px;font-weight:500;margin-bottom:8px}
.td-50 .portal .cnt{font-size:10px;color:var(--text-faint)}
.td-50 .portal.lit{background:var(--accent-dim);border-color:var(--accent)}
.td-50 .portal.lit::before{content:"→";position:absolute;top:-34px;font-size:18px;color:var(--accent)}`,
`<div class="intro">${mark(30)}<h1 class="title">Choose your door</h1></div><div class="portal lit"><span class="lbl">Sessions</span><span class="cnt">5 open</span>${cta("Enter")}</div><div class="portal"><span class="lbl">Workspaces</span><span class="cnt">3 attached</span></div>`);

// ---- graph paper ------------------------------------------------------------

set("gp", "Graph Paper", "Grids, plotters, and drafting instruments around the same calm lockup — now carrying workspaces and pulse.");

V("gp", "01", "Blue Print", `.gp-01 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(color-mix(in srgb,var(--text) 11%,transparent) 1px,transparent 1px),linear-gradient(90deg,color-mix(in srgb,var(--text) 11%,transparent) 1px,transparent 1px);background-size:18px 18px}
.gp-01 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.gp-01 .title{font-size:46px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.gp-01 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}`,
`${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "02", "Crosshair", `.gp-02 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:44px 44px}
.gp-02::before,.gp-02::after{content:"";position:absolute;background:var(--border-strong)}
.gp-02::before{left:0;right:0;top:50%;height:1px}
.gp-02::after{top:0;bottom:0;left:50%;width:1px}
.gp-02 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-02 .coord{position:absolute;font-family:ui-monospace,Menlo,monospace;font-size:9px;color:var(--text-faint)}
.gp-02 .c1{top:12px;left:14px}
.gp-02 .c2{bottom:12px;right:14px}
.gp-02 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.gp-02 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-02 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="coord c1">x:00 y:00</span><span class="coord c2">x:06 y:06</span><div class="inner">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "03", "Margin Ruled", `.gp-03 .mock{padding:56px 48px 56px 92px;background-image:repeating-linear-gradient(to bottom,transparent 0 27px,var(--border-subtle) 27px 28px)}
.gp-03::before{content:"";position:absolute;top:0;bottom:0;left:64px;width:1px;background:var(--accent);opacity:0.4}
.gp-03 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.gp-03 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.gp-03 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}`,
`<div class="note-l" style="position:absolute;left:14px;top:58px;writing-mode:vertical-rl;font-size:9px;letter-spacing:0.3em;text-transform:uppercase;color:var(--text-faint)">vela</div>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${chips(0)}`);

V("gp", "04", "Dot Matrix", `.gp-04 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:radial-gradient(var(--border-strong) 1px,transparent 1.5px);background-size:22px 22px}
.gp-04 .panel{background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:34px 44px;display:flex;flex-direction:column;align-items:center}
.gp-04 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-04 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-04 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<div class="panel">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "05", "Iso Corners", `.gp-05 .mock{padding:56px 48px;display:flex;flex-direction:column;align-items:flex-start;justify-content:center}
.gp-05 .iso{position:absolute;width:150px;height:150px;background:repeating-linear-gradient(45deg,transparent 0 9px,var(--border-subtle) 9px 10px),repeating-linear-gradient(-45deg,transparent 0 9px,var(--border-subtle) 9px 10px)}
.gp-05 .a{top:0;left:0;border-right:1px solid var(--border-subtle);border-bottom:1px solid var(--border-subtle)}
.gp-05 .b{bottom:0;right:0;border-left:1px solid var(--border-subtle);border-top:1px solid var(--border-subtle)}
.gp-05 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.gp-05 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.gp-05 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px;max-width:30ch}`,
`<span class="iso a"></span><span class="iso b"></span>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "06", "Plotter", `.gp-06 .mock{display:flex;flex-direction:column;align-items:center;justify-content:flex-end;text-align:center;padding:52px 48px 40px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:40px 40px}
.gp-06 .plot{position:absolute;inset:auto 0 170px 0;height:120px;opacity:0.9}
.gp-06 .inner{position:relative;z-index:1;display:flex;flex-direction:column;align-items:center;margin-top:auto}
.gp-06 .mark-svg{width:32px;height:32px;margin-bottom:14px}
.gp-06 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-06 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="plot" viewBox="0 0 400 100" preserveAspectRatio="none"><polyline points="0,80 60,55 120,66 180,30 240,44 300,18 360,26 400,10" fill="none" stroke="var(--accent)" stroke-width="2"/><polyline points="0,80 60,55 120,66 180,30 240,44 300,18 360,26 400,10 400,100 0,100" fill="var(--accent-dim)" stroke="none"/></svg><div class="inner">${mark(32)}<h1 class="title">Vela</h1><p class="sub">Twelve sessions this week.</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "07", "Constellation", `.gp-07 .mock{min-height:520px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-07 .sky{position:absolute;inset:0}
.gp-07 .sky line{stroke:var(--border-strong)}
.gp-07 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-07 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.gp-07 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-07 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="sky" viewBox="0 0 400 300" preserveAspectRatio="xMidYMid slice"><line x1="60" y1="60" x2="140" y2="110"/><line x1="140" y1="110" x2="90" y2="200"/><line x1="330" y1="70" x2="270" y2="140"/><line x1="270" y1="140" x2="340" y2="230"/><circle cx="60" cy="60" r="3" fill="var(--accent)"/><circle cx="140" cy="110" r="2.5" fill="var(--border-strong)"/><circle cx="90" cy="200" r="2" fill="var(--border-strong)"/><circle cx="330" cy="70" r="3" fill="var(--accent)"/><circle cx="270" cy="140" r="2.5" fill="var(--border-strong)"/><circle cx="340" cy="230" r="2" fill="var(--border-strong)"/></svg><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "08", "Axis Ticks", `.gp-08 .mock{padding:64px 60px;display:flex;flex-direction:column;align-items:flex-start;justify-content:center}
.gp-08::before{content:"";position:absolute;top:0;left:0;right:0;bottom:0;border-right:1px solid var(--border-strong);border-bottom:1px solid var(--border-strong);pointer-events:none;background:linear-gradient(90deg,transparent calc(100% - 8px),var(--border-subtle) 0),linear-gradient(to bottom,transparent calc(100% - 8px),var(--border-subtle) 0)}
.gp-08 .num{position:absolute;font-family:ui-monospace,Menlo,monospace;font-size:9px;color:var(--text-faint)}
.gp-08 .n1{top:8px;right:8px}
.gp-08 .n2{bottom:8px;right:8px}
.gp-08 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.gp-08 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.gp-08 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}`,
`<span class="num n1">520</span><span class="num n2">760</span>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "09", "Section Flags", `.gp-09 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:56px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:56px 56px}
.gp-09 .flag{position:absolute;display:flex;flex-direction:column;align-items:flex-start;gap:2px;font-size:9px;color:var(--text-faint);letter-spacing:0.1em}
.gp-09 .flag i{width:1px;height:26px;background:var(--border-strong);order:2;margin-top:2px}
.gp-09 .flag b{font-weight:600;color:var(--accent);letter-spacing:0.2em}
.gp-09 .f1{top:26px;left:34px}
.gp-09 .f2{bottom:26px;right:34px;align-items:flex-end}
.gp-09 .f2 i{order:1;margin-top:0;margin-bottom:2px}
.gp-09 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.gp-09 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-09 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<div class="flag f1"><b>VELA</b><i></i></div><div class="flag f2"><i></i><b>SHEET 01</b></div>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "10", "Contour", `.gp-10 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-10 .contours{position:absolute;inset:0}
.gp-10 .contours ellipse{fill:none;stroke:var(--border-subtle);stroke-width:1.5}
.gp-10 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-10 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.gp-10 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-10 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="contours" viewBox="0 0 400 280" preserveAspectRatio="xMidYMid slice"><ellipse cx="210" cy="150" rx="190" ry="120"/><ellipse cx="215" cy="145" rx="150" ry="92"/><ellipse cx="220" cy="140" rx="110" ry="66"/><ellipse cx="225" cy="135" rx="72" ry="42"/></svg><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "11", "Pixel Field", `.gp-11 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--grid) 1px,transparent 1px),linear-gradient(90deg,var(--grid) 1px,transparent 1px);background-size:26px 26px}
.gp-11 .px{position:absolute;width:12px;height:12px;border-radius:3px}
.gp-11 .p1{top:70px;left:88px;background:var(--accent);opacity:0.75}
.gp-11 .p2{top:130px;right:120px;background:var(--border-strong)}
.gp-11 .p3{bottom:96px;left:150px;background:var(--border-strong)}
.gp-11 .p4{bottom:170px;right:80px;background:var(--accent);opacity:0.35}
.gp-11 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.gp-11 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-11 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="px p1"></span><span class="px p2"></span><span class="px p3"></span><span class="px p4"></span>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "12", "Circuit Trace", `.gp-12 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-12 .traces{position:absolute;inset:0}
.gp-12 .traces path{fill:none;stroke:var(--border-strong)}
.gp-12 .traces circle{fill:var(--bg-panel);stroke:var(--accent);stroke-width:1.5}
.gp-12 .pad{position:absolute;inset:0;margin:auto;width:74px;height:74px;border:1px solid var(--accent);border-radius:16px;display:grid;place-items:center;background:var(--bg-panel)}
.gp-12 .pad .mark-svg{width:44px;height:44px}
.gp-12 .stack{position:absolute;left:0;right:0;bottom:54px;display:flex;flex-direction:column;align-items:center;gap:12px}`,
`<svg class="traces" viewBox="0 0 400 280" preserveAspectRatio="none"><path d="M0 40 H140 V110 H163"/><path d="M400 60 H300 V120 H237"/><path d="M0 240 H110 V170 H163"/><path d="M400 220 H320 V160 H237"/></svg><div class="pad">${mark(44)}</div><div class="stack"><h1 class="title" style="font-size:40px;font-weight:500;margin:0;letter-spacing:-0.02em">Vela</h1><p style="margin:0 0 14px;color:var(--text-dim);font-size:12.5px">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "13", "Crop Marks", `.gp-13 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:56px}
.gp-13 .crop{position:absolute;width:20px;height:20px;border-color:var(--border-strong)!important}
.gp-13 .tl{top:24px;left:24px;border-top:1px solid;border-left:1px solid}
.gp-13 .tr{top:24px;right:24px;border-top:1px solid;border-right:1px solid}
.gp-13 .bl{bottom:24px;left:24px;border-bottom:1px solid;border-left:1px solid}
.gp-13 .br{bottom:24px;right:24px;border-bottom:1px solid;border-right:1px solid}
.gp-13 .reg{position:absolute;top:24px;left:50%;transform:translateX(-50%);width:14px;height:14px;border:1px solid var(--border-strong);border-radius:50%}
.gp-13 .plate{border:1px solid var(--border-strong);border-radius:4px;padding:38px 52px;text-align:center;background:var(--bg-panel)}
.gp-13 .mark-svg{width:32px;height:32px;margin-bottom:14px}
.gp-13 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-13 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="crop tl"></span><span class="crop tr"></span><span class="crop bl"></span><span class="crop br"></span><span class="reg"></span><div class="plate">${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "14", "Blueprint Stamp", `.gp-14 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:36px 36px}
.gp-14 .stamp{position:absolute;top:44px;right:48px;transform:rotate(6deg);border:2px solid var(--accent);color:var(--accent);border-radius:6px;padding:6px 12px;font-family:ui-monospace,Menlo,monospace;font-size:10px;letter-spacing:0.18em;opacity:0.85}
.gp-14 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.gp-14 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-14 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<div class="stamp">APPROVED · v0.5</div>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "15", "Coordinate Readout", `.gp-15 .mock{padding:56px 48px;display:flex;flex-direction:column;justify-content:center;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:30px 30px}
.gp-15 .readout{position:absolute;bottom:16px;left:20px;font-family:ui-monospace,Menlo,monospace;font-size:9.5px;color:var(--text-faint);letter-spacing:0.08em}
.gp-15 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.gp-15 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.gp-15 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}`,
`<div class="readout">cursor 048·096 &nbsp; zoom 1.0×</div>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "16", "Ruler Origin", `.gp-16 .mock{padding:76px 60px 56px 84px}
.gp-16 .ruler-x{position:absolute;top:34px;left:0;right:0;height:16px;background:repeating-linear-gradient(90deg,var(--border-strong) 0 1px,transparent 1px 20px);border-bottom:1px solid var(--border-subtle)}
.gp-16 .ruler-y{position:absolute;left:34px;top:0;bottom:0;width:16px;background:repeating-linear-gradient(to bottom,var(--border-strong) 0 1px,transparent 1px 20px);border-right:1px solid var(--border-subtle)}
.gp-16 .origin{position:absolute;top:34px;left:34px;width:17px;height:17px;border-right:1px solid var(--border-subtle);border-bottom:1px solid var(--border-subtle)}
.gp-16 .zero{position:absolute;top:52px;left:40px;font-family:ui-monospace,Menlo,monospace;font-size:8.5px;color:var(--text-faint)}
.gp-16 .mark-svg{width:36px;height:36px;margin-bottom:20px}
.gp-16 .title{font-size:48px;font-weight:500;margin:0 0 8px;letter-spacing:-0.02em}
.gp-16 .sub{margin:0 0 24px;color:var(--text-dim);font-size:13px}`,
`<span class="ruler-x"></span><span class="ruler-y"></span><span class="origin"></span><span class="zero">0,0</span>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "17", "Radial Collapse", `.gp-17 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:44px 44px}
.gp-17::before{content:"";position:absolute;left:50%;top:50%;width:300px;height:300px;transform:translate(-50%,-50%);border-radius:50%;background-color:var(--bg-panel);background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:14px 14px;border:1px solid var(--border-strong)}
.gp-17 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-17 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-17 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-17 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="inner">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "18", "Hatch Quads", `.gp-18 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:56px}
.gp-18 .hq{position:absolute;width:110px;height:110px;background:repeating-linear-gradient(45deg,transparent 0 6px,var(--border-subtle) 6px 7px)}
.gp-18 .q1{top:0;left:0;border-bottom:1px solid var(--border-subtle);border-right:1px solid var(--border-subtle)}
.gp-18 .q2{top:0;right:0;border-bottom:1px solid var(--border-subtle);border-left:1px solid var(--border-subtle);background:repeating-linear-gradient(-45deg,transparent 0 6px,var(--border-subtle) 6px 7px)}
.gp-18 .q3{bottom:0;left:0;border-top:1px solid var(--border-subtle);border-right:1px solid var(--border-subtle)}
.gp-18 .q4{bottom:0;right:0;border-top:1px solid var(--border-subtle);border-left:1px solid var(--border-subtle);background:repeating-linear-gradient(-45deg,transparent 0 6px,var(--border-subtle) 6px 7px)}
.gp-18 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.gp-18 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-18 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="hq q1"></span><span class="hq q2"></span><span class="hq q3"></span><span class="hq q4"></span>${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "19", "Perforation Plate", `.gp-19 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:52px;background-image:radial-gradient(var(--border-subtle) 1.5px,transparent 1.5px);background-size:16px 16px}
.gp-19 .ticket{background:var(--bg-panel);border:2px dashed var(--border-strong);border-radius:14px;padding:36px 48px;text-align:center}
.gp-19 .mark-svg{width:32px;height:32px;margin-bottom:14px}
.gp-19 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-19 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="ticket">${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "20", "Waveform", `.gp-20 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:48px 48px}
.gp-20 .wave{position:absolute;left:0;right:0;top:56%;height:60px}
.gp-20 .wave path{fill:none;stroke:var(--accent);stroke-width:1.5}
.gp-20 .above{position:absolute;left:0;right:0;top:calc(56% - 208px);display:flex;flex-direction:column;align-items:center}
.gp-20 .below{position:absolute;left:0;right:0;top:calc(56% + 62px);display:flex;flex-direction:column;align-items:center;gap:14px}`,
`<svg class="wave" viewBox="0 0 400 60" preserveAspectRatio="none"><path d="M0 30 Q25 0 50 30 T100 30 T150 30 T200 30 T250 30 T300 30 T350 30 T400 30"/></svg><div class="above">${mark(32)}<h1 class="title" style="font-size:44px;font-weight:500;margin:12px 0 6px;letter-spacing:-0.02em">Vela</h1><p style="margin:0;color:var(--text-dim);font-size:12.5px">${SUB}</p></div><div class="below">${cta("Open a folder")}${stats()}</div>`);

V("gp", "21", "Node Net", `.gp-21 .mock{padding:56px 48px;background-image:radial-gradient(var(--border-strong) 1px,transparent 1.5px);background-size:26px 26px;display:flex;flex-direction:column;justify-content:center}
.gp-21 .cellbox{position:absolute;display:grid;place-items:center;width:78px;height:78px;background:var(--bg-panel);border:1px solid var(--accent);border-radius:12px;font-size:10px;font-weight:600;color:var(--text);box-shadow:var(--shadow-sm)}
.gp-21 .cb1{top:70px;right:80px}
.gp-21 .cb2{bottom:120px;right:190px}
.gp-21 .cb3{bottom:60px;right:70px}
.gp-21 .cellbox small{color:var(--text-faint);font-weight:400;margin-top:2px}
.gp-21 .hero{max-width:300px}
.gp-21 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.gp-21 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-21 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="cellbox cb1">vela<small>4 live</small></div><div class="cellbox cb2">atlas-notes<small>2</small></div><div class="cellbox cb3">quiet-web<small>2</small></div>`);

V("gp", "22", "Elevation", `.gp-22 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-22 .terr{position:absolute;right:-40px;bottom:-40px;width:260px;height:260px}
.gp-22 .terr i{position:absolute;border:1px solid var(--border-strong);border-radius:14px}
.gp-22 .t1{inset:0}
.gp-22 .t2{inset:36px}
.gp-22 .t3{inset:72px;background:var(--bg-inset)}
.gp-22 .flaglet{position:absolute;top:26px;right:196px;font-size:9px;letter-spacing:0.16em;color:var(--text-faint);text-transform:uppercase}
.gp-22 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-22 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-22 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="terr"><i class="t1"></i><i class="t2"></i><i class="t3"></i></div><span class="flaglet">▲ elevation 3</span>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "23", "Scanline", `.gp-23 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:38px 38px}
.gp-23::before{content:"";position:absolute;left:0;right:0;height:44px;background:linear-gradient(to bottom,transparent,var(--accent-dim));animation:gp23-sweep 4.5s linear infinite}
@keyframes gp23-sweep{from{top:-48px}to{top:100%}}
.gp-23 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-23 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-23 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-23 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="inner">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "24", "Marginalia", `.gp-24 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:56px;background-image:linear-gradient(var(--grid) 1px,transparent 1px),linear-gradient(90deg,var(--grid) 1px,transparent 1px);background-size:34px 34px}
.gp-24 .hand{position:absolute;font-family:var(--serif);font-style:italic;color:var(--text-faint);font-size:14px}
.gp-24 .h1a{top:64px;left:56px;transform:rotate(-4deg)}
.gp-24 .h2a{bottom:70px;right:56px;transform:rotate(3deg)}
.gp-24 .dash{position:absolute;height:1px;background:none;border-top:1px dashed var(--border-strong)}
.gp-24 .d1{top:86px;left:150px;width:70px}
.gp-24 .d2{bottom:96px;right:170px;width:70px}
.gp-24 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-24 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-24 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="hand h1a">start here →</span><span class="dash d1"></span><span class="hand h2a">← calm lives here</span><span class="dash d2"></span>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "25", "Cross-Stitch", `.gp-25 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:64px 52px}
.gp-25 .band{position:absolute;left:0;right:0;height:26px;color:var(--border-strong);font-size:11px;letter-spacing:6px;line-height:26px;white-space:nowrap;overflow:hidden;user-select:none}
.gp-25 .bt{top:10px}
.gp-25 .bb{bottom:10px}
.gp-25 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-25 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-25 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="band bt">${"✕ ".repeat(40)}</div><div class="band bb">${"✕ ".repeat(40)}</div>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "26", "Radar Sweep", `.gp-26 .mock{padding:56px 48px;display:flex;flex-direction:column;justify-content:center}
.gp-26 .radar{position:absolute;top:-90px;left:-90px;width:280px;height:280px}
.gp-26 .radar circle{fill:none;stroke:var(--border-strong)}
.gp-26 .blip{position:absolute;top:104px;left:112px;width:8px;height:8px;border-radius:50%;background:var(--accent)}
.gp-26 .blip::after{content:"";position:absolute;inset:-5px;border:1px solid var(--accent);border-radius:50%;animation:gp26-ping 2.2s ease-out infinite}
@keyframes gp26-ping{from{transform:scale(0.4);opacity:1}to{transform:scale(1.8);opacity:0}}
.gp-26 .hero{max-width:290px;margin-left:auto;text-align:right;display:flex;flex-direction:column;align-items:flex-end}
.gp-26 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-26 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-26 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="radar" viewBox="0 0 100 100"><circle cx="0" cy="0" r="40"/><circle cx="0" cy="0" r="70"/><circle cx="0" cy="0" r="98"/><line x1="0" y1="0" x2="70" y2="70" stroke="var(--accent)" stroke-width="1"/></svg><span class="blip"></span><div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "27", "Title Block", `.gp-27 .mock{padding:26px}
.gp-27 .frame{position:relative;height:100%;border:1px solid var(--border-strong);outline:1px solid var(--border-subtle);outline-offset:4px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:40px}
.gp-27 .tb{position:absolute;right:-1px;bottom:-1px;border:1px solid var(--border-strong);display:grid;grid-template-columns:auto auto;font-family:ui-monospace,Menlo,monospace;font-size:8.5px;color:var(--text-faint);background:var(--bg-panel)}
.gp-27 .tb span{padding:4px 10px;border-top:1px solid var(--border-subtle)}
.gp-27 .tb span:nth-child(odd){border-right:1px solid var(--border-subtle);background:var(--bg-inset)}
.gp-27 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-27 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-27 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="frame">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}<div class="tb"><span>PROJECT</span><span>VELA</span><span>SHEET</span><span>01 / 50</span></div></div>`);

V("gp", "28", "Iso Shelf", `.gp-28 .mock{display:flex;flex-direction:column;align-items:center;justify-content:flex-start;text-align:center;padding:60px 52px 0}
.gp-28 .shelfline{position:absolute;left:15%;right:15%;top:308px;height:1px;background:var(--border-strong)}
.gp-28 .cube{position:absolute;top:244px;width:56px;height:56px}
.gp-28 .c1{left:22%}.gp-28 .c2{left:38%}.gp-28 .c3{left:54%}.gp-28 .c4{left:70%}
.gp-28 .mark-svg{width:34px;height:34px;margin-bottom:14px}
.gp-28 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-28 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12.5px}
.gp-28 .under{margin-top:auto;padding-bottom:10px;display:flex;flex-direction:column;align-items:center;gap:14px}`,
`${mark(34)}<h1 class="title">Vela</h1><p class="sub">Three workspaces on the bench.</p><span class="shelfline"></span><svg class="cube c1" viewBox="0 0 56 56"><path d="M28 4 52 18 28 32 4 18Z" fill="var(--accent)" opacity=".85"/><path d="M4 18v20l24 14V32Z" fill="var(--accent)" opacity=".45"/><path d="M52 18v20L28 52V32Z" fill="var(--accent)" opacity=".65"/></svg><svg class="cube c2" viewBox="0 0 56 56"><path d="M28 4 52 18 28 32 4 18Z" fill="var(--border-strong)"/><path d="M4 18v20l24 14V32Z" fill="var(--border-subtle)"/><path d="M52 18v20L28 52V32Z" fill="var(--bg-inset)"/></svg><svg class="cube c3" viewBox="0 0 56 56"><path d="M28 4 52 18 28 32 4 18Z" fill="var(--border-strong)"/><path d="M4 18v20l24 14V32Z" fill="var(--border-subtle)"/><path d="M52 18v20L28 52V32Z" fill="var(--bg-inset)"/></svg><svg class="cube c4" viewBox="0 0 56 56"><path d="M28 4 52 18 28 32 4 18Z" fill="var(--accent)" opacity=".85"/><path d="M4 18v20l24 14V32Z" fill="var(--accent)" opacity=".45"/><path d="M52 18v20L28 52V32Z" fill="var(--accent)" opacity=".65"/></svg><div class="under">${cta("Open a folder")}${stats()}</div>`);

V("gp", "29", "Star Chart", `.gp-29 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-29 .chart{position:absolute;inset:0}
.gp-29 .chart path{fill:none;stroke:var(--border-subtle)}
.gp-29 .chart circle.s{fill:var(--border-strong)}
.gp-29 .flare{position:absolute;top:96px;left:50%;transform:translateX(-50%)}
.gp-29 .inner{position:relative;margin-top:120px;display:flex;flex-direction:column;align-items:center}
.gp-29 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-29 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="chart" viewBox="0 0 400 300" preserveAspectRatio="xMidYMid slice"><path d="M-20 80 Q200 20 420 90"/><path d="M-20 130 Q200 70 420 140"/><circle class="s" cx="80" cy="52" r="2"/><circle class="s" cx="310" cy="60" r="2.5"/><circle class="s" cx="150" cy="105" r="1.6"/><circle class="s" cx="250" cy="95" r="1.6"/><circle cx="200" cy="96" r="7" fill="var(--accent)"/></svg><div class="inner"><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "30", "Punch Card", `.gp-30 .mock{display:flex;padding:56px 0 56px 64px}
.gp-30 .punches{position:absolute;left:26px;top:40px;bottom:40px;width:16px;background-image:radial-gradient(circle,var(--bg) 3px,var(--border-strong) 3px 4px,transparent 4.5px);background-size:16px 26px;border-left:2px solid var(--border-strong);border-right:2px solid var(--border-strong)}
.gp-30 .content{display:flex;flex-direction:column;justify-content:center;padding-right:48px}
.gp-30 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.gp-30 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-30 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<div class="punches"></div><div class="content">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${chips(0)}</div>`);

V("gp", "31", "Halftone", `.gp-31 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-31::before{content:"";position:absolute;top:40px;right:40px;width:220px;height:160px;background-image:radial-gradient(var(--border-strong) 2.2px,transparent 2.6px);background-size:12px 12px;-webkit-mask-image:linear-gradient(115deg,transparent 30%,black 75%);mask-image:linear-gradient(115deg,transparent 30%,black 75%)}
.gp-31::after{content:"";position:absolute;bottom:36px;left:36px;width:180px;height:130px;background-image:radial-gradient(var(--accent) 2px,transparent 2.4px);background-size:10px 10px;opacity:.5;-webkit-mask-image:linear-gradient(295deg,transparent 30%,black 75%);mask-image:linear-gradient(295deg,transparent 30%,black 75%)}
.gp-31 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-31 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-31 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "32", "Measure Tape", `.gp-32 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px 52px 96px}
.gp-32 .tape{position:absolute;left:0;right:0;bottom:34px;height:34px;border-top:1px solid var(--border-strong);border-bottom:1px solid var(--border-strong);background:repeating-linear-gradient(90deg,var(--border-strong) 0 1px,transparent 1px 12px),repeating-linear-gradient(90deg,var(--border-strong) 0 1px,transparent 1px 60px);background-blend-mode:normal}
.gp-32 .tape-num{position:absolute;bottom:44px;left:0;right:0;display:flex;justify-content:space-around;font-family:ui-monospace,Menlo,monospace;font-size:8.5px;color:var(--text-faint)}
.gp-32 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-32 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-32 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="tape-num"><span>10</span><span>20</span><span>30</span><span>40</span><span>50</span></div><div class="tape"></div>${mark(34)}<h1 class="title">Vela</h1><p class="sub">Nothing hidden.</p>${cta("Open a folder")}${stats()}`);

V("gp", "33", "Drafting Table", `.gp-33 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:56px}
.gp-33 .tsquare{position:absolute;left:60px;right:60px;top:110px;height:1px;background:var(--border-strong)}
.gp-33 .tsquare::before{content:"";position:absolute;top:0;left:0;width:1px;height:300px;background:var(--border-strong)}
.gp-33 .tri{position:absolute;right:64px;top:130px;width:120px;height:104px;border:1px solid var(--border-strong);clip-path:polygon(0 100%,100% 100%,100% 0);opacity:.7}
.gp-33 .sheet{position:relative;background:var(--bg-panel);border:1px solid var(--border-strong);padding:34px 44px;text-align:center;margin-top:40px}
.gp-33 .mark-svg{width:32px;height:32px;margin-bottom:12px}
.gp-33 .title{font-size:40px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-33 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<span class="tsquare"></span><span class="tri"></span><div class="sheet">${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "34", "Signal Bars", `.gp-34 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:42px 42px}
.gp-34 .sig{position:absolute;top:48px;right:52px;display:flex;align-items:flex-end;gap:4px;height:44px}
.gp-34 .sig i{width:8px;border-radius:3px 3px 0 0;background:var(--border-strong)}
.gp-34 .sig i:nth-child(1){height:30%}
.gp-34 .sig i:nth-child(2){height:52%}
.gp-34 .sig i:nth-child(3){height:74%}
.gp-34 .sig i.on{height:100%;background:var(--accent)}
.gp-34 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-34 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-34 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="sig"><i></i><i></i><i class="on"></i><i class="on"></i></div>${mark(34)}<h1 class="title">Vela</h1><p class="sub">Two agents broadcasting.</p>${cta("Open a folder")}${stats()}`);

V("gp", "35", "Maze Border", `.gp-35 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:64px}
.gp-35::before{content:"";position:absolute;inset:22px;border:1px solid var(--border-strong);border-radius:6px;
-webkit-mask:linear-gradient(black,black) top/100% 8px no-repeat,linear-gradient(black,black) bottom/100% 8px no-repeat,linear-gradient(black,black) left/8px 60% no-repeat,linear-gradient(black,black) right/8px 40% no-repeat;mask:linear-gradient(black,black) top/100% 8px no-repeat,linear-gradient(black,black) bottom/100% 8px no-repeat,linear-gradient(black,black) left/8px 60% no-repeat,linear-gradient(black,black) right/8px 40% no-repeat}
.gp-35 .arrow{position:absolute;right:34px;top:38%;color:var(--accent);font-size:14px}
.gp-35 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-35 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-35 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="arrow">➜</span>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "36", "Topo Lines", `.gp-36 .mock{display:flex;flex-direction:column;align-items:center;justify-content:flex-start;text-align:center;padding:70px 52px 0}
.gp-36 .topo{position:absolute;left:0;right:0;bottom:0;height:200px}
.gp-36 .topo path{fill:none;stroke:var(--border-strong)}
.gp-36 .peak{position:absolute;left:50%;bottom:132px;transform:translateX(-50%);display:flex;flex-direction:column;align-items:center;gap:3px;font-size:9px;letter-spacing:0.14em;color:var(--text-faint)}
.gp-36 .peak i{width:1px;height:22px;background:var(--accent)}
.gp-36 .head{position:relative;z-index:1;display:flex;flex-direction:column;align-items:center;margin-bottom:150px}
.gp-36 .mark-svg{width:34px;height:34px;margin-bottom:14px}
.gp-36 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-36 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="topo" viewBox="0 0 400 100" preserveAspectRatio="none"><path d="M0 70 Q60 40 120 66 T240 60 T400 72"/><path d="M0 82 Q70 56 140 78 T280 74 T400 84"/><path d="M0 94 Q80 72 160 90 T320 88 T400 94"/></svg><div class="head">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="peak"><i></i>summit</div>`);

V("gp", "37", "Chladni", `.gp-37 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-37 .fig{position:absolute;inset:0}
.gp-37 .fig path{fill:none;stroke:var(--accent);opacity:.35}
.gp-37 .fig circle{fill:none;stroke:var(--border-strong)}
.gp-37 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-37 .mark-svg{width:36px;height:36px;margin-bottom:16px}
.gp-37 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-37 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="fig" viewBox="0 0 200 200"><circle cx="100" cy="100" r="86"/><path d="M100 20 C160 60 160 140 100 180 M100 20 C40 60 40 140 100 180 M20 100 C60 40 140 40 180 100 M20 100 C60 160 140 160 180 100"/></svg><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "38", "Ledger Cells", `.gp-38 .mock{padding:56px 52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:34px 34px;display:flex;flex-direction:column;justify-content:center}
.gp-38 .rangechip{position:absolute;top:44px;right:52px;font-family:ui-monospace,Menlo,monospace;font-size:9.5px;color:var(--accent);border:1px solid var(--accent);border-radius:5px;padding:2px 7px}
.gp-38 .sel{position:absolute;left:190px;top:214px;width:138px;height:102px;background:var(--accent-dim);outline:1.5px solid var(--accent);border-radius:2px}
.gp-38 .colhead{position:absolute;left:190px;top:192px;display:flex;width:138px;justify-content:space-around;font-size:8.5px;color:var(--text-faint);font-family:ui-monospace,Menlo,monospace}
.gp-38 .rowhead{position:absolute;left:168px;top:216px;display:flex;flex-direction:column;height:100px;justify-content:space-between;font-size:8.5px;color:var(--text-faint);font-family:ui-monospace,Menlo,monospace}
.gp-38 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.gp-38 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-38 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px;max-width:24ch}`,
`<div class="rangechip">workspaces!A1:C3</div><div class="sel"></div><div class="colhead"><span>A</span><span>B</span><span>C</span></div><div class="rowhead"><span>1</span><span>2</span><span>3</span></div>${mark(36)}<h1 class="title">Vela</h1><p class="sub">Three workspaces selected.</p>${cta("Open a folder")}`);

V("gp", "39", "Pin Board", `.gp-39 .mock{padding:56px 48px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:40px 40px}
.gp-39 .pinlab{position:absolute;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:6px;padding:6px 10px;font-size:10.5px;box-shadow:var(--shadow-sm)}
.gp-39 .pinlab::before{content:"";position:absolute;top:-4px;left:8px;width:0;height:0;border-left:5px solid transparent;border-right:5px solid transparent;border-top:6px solid var(--accent)}
.gp-39 .pl1{top:64px;right:64px}
.gp-39 .pl2{bottom:120px;right:170px}
.gp-39 .pl3{bottom:56px;right:60px}
.gp-39 .hero{max-width:290px}
.gp-39 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-39 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-39 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="pinlab pl1">📌 vela · 4</div><div class="pinlab pl2">atlas-notes · 2</div><div class="pinlab pl3">quiet-web · 2</div>`);

V("gp", "40", "Wind Rose", `.gp-40 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-40 .rose{position:absolute;bottom:26px;left:26px;opacity:.8}
.gp-40 .rose line{stroke:var(--border-strong)}
.gp-40 .rose text{fill:var(--text-faint);font-size:7px;font-family:ui-monospace,Menlo,monospace}
.gp-40 .needle{position:absolute;bottom:96px;left:96px;color:var(--accent);font-size:12px}
.gp-40 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-40 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-40 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="rose" width="110" height="110" viewBox="0 0 100 100"><line x1="50" y1="4" x2="50" y2="96"/><line x1="4" y1="50" x2="96" y2="50"/><circle cx="50" cy="50" r="30" fill="none" stroke="var(--border-subtle)"/><text x="47" y="12">N</text><text x="88" y="53">E</text><text x="47" y="97">S</text><text x="8" y="53">W</text></svg><span class="needle">↗</span>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "41", "Barcode Edge", `.gp-41 .mock{display:flex;align-items:center;padding:56px 90px 56px 52px}
.gp-41::after{content:"";position:absolute;top:0;bottom:0;right:34px;width:26px;background:repeating-linear-gradient(90deg,var(--text) 0 2px,transparent 2px 5px,var(--text) 5px 6px,transparent 6px 11px,var(--text) 11px 15px,transparent 15px 19px);opacity:.55}
.gp-41 .code{position:absolute;right:22px;bottom:44px;writing-mode:vertical-rl;font-family:ui-monospace,Menlo,monospace;font-size:8px;letter-spacing:.35em;color:var(--text-faint)}
.gp-41 .hero{max-width:300px}
.gp-41 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-41 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-41 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="code">VELA-0001</div>`);

V("gp", "42", "Tessellate", `.gp-42 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:52px}
.gp-42 .band{position:absolute;left:0;right:0;height:64px;background:conic-gradient(from 45deg at 50% 50%,var(--bg-inset) 25%,transparent 0 50%,var(--bg-inset) 0 75%,transparent 0) 0 0/32px 32px;opacity:.6}
.gp-42 .bt{top:70px}
.gp-42 .bb{bottom:70px}
.gp-42 .plate{position:relative;z-index:1;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:32px 44px;text-align:center;box-shadow:var(--shadow-md)}
.gp-42 .mark-svg{width:32px;height:32px;margin-bottom:12px}
.gp-42 .title{font-size:40px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-42 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<div class="band bt"></div><div class="plate">${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div><div class="band bb"></div>`);

V("gp", "43", "Oscilloscope", `.gp-43 .mock{display:flex;align-items:center;justify-content:center;padding:48px}
.gp-43 .scope{position:relative;width:430px;height:300px;border:1px solid var(--border-strong);border-radius:18px;background:var(--bg);overflow:hidden}
.gp-43 .scope svg.grid{position:absolute;inset:0}
.gp-43 .lissa{position:absolute;inset:0}
.gp-43 .lissa path{fill:none;stroke:var(--accent);stroke-width:1.5;opacity:.7}
.gp-43 .card{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:22px 30px;text-align:center;box-shadow:var(--shadow-md)}
.gp-43 .mark-svg{width:28px;height:28px;margin-bottom:10px}
.gp-43 .title{font-size:34px;font-weight:500;margin:0 0 4px;letter-spacing:-0.02em}
.gp-43 .sub{margin:0 0 14px;color:var(--text-dim);font-size:11.5px}`,
`<div class="scope"><svg class="grid" width="100%" height="100%"><defs><pattern id="sc" width="30" height="30" patternUnits="userSpaceOnUse"><path d="M30 0H0V30" fill="none" stroke="var(--border-subtle)"/></pattern></defs><rect width="100%" height="100%" fill="url(#sc)"/></svg><svg class="lissa" viewBox="0 0 430 300" preserveAspectRatio="none"><path d="M215 30 C330 30 390 90 330 150 C270 210 160 270 100 210 C40 150 100 30 215 30 Z"/></svg><div class="card">${mark(28)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div></div>`);

V("gp", "44", "Fractal Corners", `.gp-44 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:56px}
.gp-44 .fr{position:absolute;border:1px solid var(--border-strong)}
.gp-44 .tl1{top:18px;left:18px;width:64px;height:64px}
.gp-44 .tl2{top:18px;left:18px;width:32px;height:32px;border-color:var(--accent)}
.gp-44 .br1{bottom:18px;right:18px;width:64px;height:64px}
.gp-44 .br2{bottom:18px;right:18px;width:32px;height:32px;border-color:var(--accent)}
.gp-44 .tr1{top:18px;right:18px;width:44px;height:44px}
.gp-44 .bl1{bottom:18px;left:18px;width:44px;height:44px}
.gp-44 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-44 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-44 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="fr tl1"></span><span class="fr tl2"></span><span class="fr tr1"></span><span class="fr bl1"></span><span class="fr br1"></span><span class="fr br2"></span>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "45", "String Art", `.gp-45 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.gp-45 .strings{position:absolute;inset:0}
.gp-45 .strings line{stroke:var(--accent);opacity:.16}
.gp-45 .strings circle{fill:none;stroke:var(--border-strong)}
.gp-45 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-45 .mark-svg{width:36px;height:36px;margin-bottom:16px}
.gp-45 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-45 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="strings" viewBox="0 0 300 300"><circle cx="150" cy="150" r="120"/>${Array.from({ length: 24 }, (_, i) => { const a1 = (i / 24) * Math.PI * 2, a2 = ((i * 7) / 24) * Math.PI * 2; const x1 = 150 + 120 * Math.cos(a1), y1 = 150 + 120 * Math.sin(a1), x2 = 150 + 120 * Math.cos(a2), y2 = 150 + 120 * Math.sin(a2); return `<line x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}"/>`; }).join("")}</svg><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "46", "Callouts", `.gp-46 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:64px 90px}
.gp-46 .callout{position:absolute;font-size:9.5px;letter-spacing:0.12em;text-transform:uppercase;color:var(--text-faint)}
.gp-46 .callout::before{content:"";position:absolute;top:50%;width:44px;border-top:1px solid var(--border-strong)}
.gp-46 .co1{top:150px;right:14px}
.gp-46 .co1::before{right:100%}
.gp-46 .co2{bottom:170px;left:14px;text-align:right}
.gp-46 .co2::before{left:100%}
.gp-46 .mark-svg{width:34px;height:34px;margin-bottom:14px}
.gp-46 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-46 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="callout co1">← bare mark</div><div class="callout co2">one action →</div>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

V("gp", "47", "Dot Pitch", `.gp-47 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:repeating-linear-gradient(90deg,var(--border-subtle) 0 1px,transparent 1px 4px)}
.gp-47 .glow{position:absolute;left:50%;top:50%;width:340px;height:340px;transform:translate(-50%,-50%);border-radius:50%;background:radial-gradient(circle,var(--bg-panel) 30%,transparent 70%)}
.gp-47 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.gp-47 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-47 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-47 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="glow"></div><div class="inner">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "48", "Survey Plat", `.gp-48 .mock{padding:56px 52px}
.gp-48 .plot{position:absolute;inset:44px 44px 44px 44px;border:1.5px dashed var(--border-strong);border-radius:10px}
.gp-48 .plot::after{content:"";position:absolute;inset:14px;border:1px solid var(--border-subtle);border-radius:6px}
.gp-48 .lot{position:absolute;top:34px;right:56px;font-family:ui-monospace,Menlo,monospace;font-size:9px;color:var(--accent);border:1px solid var(--accent);border-radius:999px;padding:3px 9px;background:var(--bg-panel)}
.gp-48 .north{position:absolute;top:52px;left:56px;font-size:9px;letter-spacing:0.2em;color:var(--text-faint)}
.gp-48 .hero{position:relative;max-width:300px;margin-top:40px}
.gp-48 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-48 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-48 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="plot"></div><div class="lot">LOT 1 · VELA</div><div class="north">↑ N</div><div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("gp", "49", "Weave Band", `.gp-49 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:52px}
.gp-49 .weave{position:absolute;left:0;right:0;height:44px;display:flex;gap:6px;overflow:hidden}
.gp-49 .weave i{flex:none;width:34px;border-radius:8px}
.gp-49 .wt{top:64px}
.gp-49 .wb{bottom:64px}
.gp-49 .wt i:nth-child(odd){background:var(--bg-inset);transform:translateY(-6px)}
.gp-49 .wt i:nth-child(even){background:var(--accent-dim);transform:translateY(6px)}
.gp-49 .wb i:nth-child(odd){background:var(--accent-dim);transform:translateY(-6px)}
.gp-49 .wb i:nth-child(even){background:var(--bg-inset);transform:translateY(6px)}
.gp-49 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.gp-49 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-49 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="weave wt">${"<i></i>".repeat(24)}</div>${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}<div class="weave wb">${"<i></i>".repeat(24)}</div>`);

V("gp", "50", "Night Lab", `.gp-50{--bg:#141110;--bg-panel:#221d1a;--bg-inset:#1b1715;--bg-hover:rgba(255,255,255,0.05);--text:#efe7db;--text-dim:#b6a894;--text-faint:#8a7d69;--accent:#9eb4a1;--accent-hover:#b2c4b4;--accent-dim:rgba(158,180,161,0.16);--border:rgba(255,255,255,0.08);--border-subtle:rgba(255,255,255,0.05);--border-strong:rgba(255,255,255,0.16);--on-accent:#172019}
.gp-50 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(color-mix(in srgb,var(--accent) 9%,transparent) 1px,transparent 1px),linear-gradient(90deg,color-mix(in srgb,var(--accent) 9%,transparent) 1px,transparent 1px);background-size:34px 34px;background-color:var(--bg-panel)}
.gp-50 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.gp-50 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.gp-50 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}`);

// ---- plaque -----------------------------------------------------------------

set("pq", "Plaque", "A small panel before the main app — gates, docks, and thresholds that never take the whole surface.");

V("pq", "01", "Step One", `.pq-01 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-01 .plaque{background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:18px;padding:36px 44px;text-align:center;box-shadow:var(--shadow-md)}
.pq-01 .mark-svg{width:30px;height:30px;margin-bottom:14px}
.pq-01 .title{font-size:34px;font-weight:500;margin:0 0 4px}
.pq-01 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12px}
.pq-01 .steps{display:flex;gap:5px;justify-content:center;margin-top:16px}
.pq-01 .steps i{width:6px;height:6px;border-radius:50%;background:var(--border-strong)}
.pq-01 .steps i.on{background:var(--accent);width:16px;border-radius:999px}`,
`<div class="plaque">${mark(30)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Continue")}${stats()}<div class="steps"><i class="on"></i><i></i><i></i></div></div>`);

V("pq", "02", "Keyhole", `.pq-02 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-02 .plaque{position:relative;width:300px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:18px;padding:56px 36px 32px;text-align:center;box-shadow:var(--shadow-md)}
.pq-02 .keyhole{position:absolute;top:-26px;left:50%;transform:translateX(-50%);width:52px;height:52px;border-radius:50%;background:var(--accent);display:grid;place-items:center;box-shadow:var(--shadow-sm)}
.pq-02 .keyhole::after{content:"";width:10px;height:10px;border-radius:50%;border:3px solid var(--on-accent)}
.pq-02 .title{font-size:32px;font-weight:500;margin:0 0 4px}
.pq-02 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12px}`,
`<div class="plaque"><span class="keyhole"></span><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a workspace")}${wsRows()}</div>`);

V("pq", "03", "Door Mat", `.pq-03 .mock{position:relative;display:grid;place-items:center;background:var(--bg)}
.pq-03 .door{position:absolute;left:50%;top:60px;transform:translateX(-50%);width:300px;height:330px;border:1.5px solid var(--border-strong);border-bottom:0;border-radius:150px 150px 0 0}
.pq-03 .door::after{content:"";position:absolute;right:26px;top:170px;width:7px;height:7px;border-radius:50%;background:var(--accent)}
.pq-03 .mat{position:relative;margin-top:250px;width:240px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;padding:20px 24px;text-align:center;box-shadow:var(--shadow-sm)}
.pq-03 .title{font-size:28px;font-weight:500;margin:0 0 2px}
.pq-03 .sub{margin:0 0 14px;color:var(--text-dim);font-size:11.5px}`,
`<div class="door"></div><div class="mat"><h1 class="title">Welcome in</h1><p class="sub">3 workspaces · 2 live</p>${cta("Step inside")}</div>`);

V("pq", "04", "Lift Buttons", `.pq-04 .mock{display:flex;align-items:center;justify-content:center;gap:26px;background:var(--bg);padding:48px}
.pq-04 .panel-btns{display:flex;flex-direction:column;gap:10px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:14px;padding:14px}
.pq-04 .lb{width:38px;height:38px;border-radius:50%;border:1.5px solid var(--border-strong);display:grid;place-items:center;font-size:13px;color:var(--text-faint)}
.pq-04 .lb.on{background:var(--accent);border-color:var(--accent);color:var(--on-accent);box-shadow:0 0 0 4px var(--accent-dim)}
.pq-04 .plaque{width:280px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:18px;padding:30px 32px;text-align:center;box-shadow:var(--shadow-md)}
.pq-04 .title{font-size:30px;font-weight:500;margin:0 0 4px}
.pq-04 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<div class="panel-btns"><span class="lb on">▲</span><span class="lb">▼</span></div><div class="plaque">${kick("floor 1 · landing")}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${wsRows()}</div>`);

V("pq", "05", "Badge Tap", `.pq-05 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-05 .reader{position:relative;width:340px;height:120px;border:1px solid var(--border-strong);border-radius:14px;background:repeating-linear-gradient(90deg,var(--bg-inset) 0 14px,var(--bg-panel) 14px 28px);display:flex;align-items:center;justify-content:flex-end;padding-right:18px}
.pq-05 .led{position:absolute;top:14px;right:14px;width:8px;height:8px;border-radius:50%;background:var(--accent);animation:pq05-blink 1.6s ease infinite}
@keyframes pq05-blink{50%{opacity:.25}}
.pq-05 .badge{position:absolute;left:-34px;top:50%;transform:translateY(-50%) rotate(-8deg);width:150px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;padding:16px 14px 12px;text-align:center;box-shadow:var(--shadow-md)}
.pq-05 .badge .hole{width:26px;height:8px;border-radius:999px;border:1.5px solid var(--border-strong);margin:0 auto 10px}
.pq-05 .badge b{font-size:13px;font-weight:500}
.pq-05 .badge span{display:block;font-size:9.5px;color:var(--text-faint);margin-top:2px}`,
`<div class="reader"><span class="led"></span><div class="badge"><span class="hole"></span><b>VELA</b><span>tap to enter · 3 desks</span></div></div>`);

V("pq", "06", "Airlock", `.pq-06 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-06 .ring{position:relative;width:230px;height:230px;border:1px solid var(--border-strong);border-radius:50%;display:grid;place-items:center}
.pq-06 .ring::before{content:"";position:absolute;inset:16px;border:1px dashed var(--border-strong);border-radius:50%}
.pq-06 .core{width:150px;height:150px;border-radius:50%;background:var(--bg-panel);border:1px solid var(--border-strong);box-shadow:var(--shadow-md);display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:16px}
.pq-06 .hatch{position:absolute;inset:-1px;border-radius:50%;border:1px solid transparent;background:repeating-conic-gradient(var(--border-subtle) 0 4deg,transparent 4deg 8deg);-webkit-mask:radial-gradient(circle,transparent 62%,black 63%);mask:radial-gradient(circle,transparent 62%,black 63%)}
.pq-06 .title{font-size:26px;font-weight:500;margin:0 0 2px}
.pq-06 .sub{margin:0 0 12px;color:var(--text-dim);font-size:11px}`,
`<div class="ring"><span class="hatch"></span><div class="core">${mark(26)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Enter")}</div></div>`);

V("pq", "07", "Concierge", `.pq-07 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-07 .deskcard{width:330px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;overflow:hidden;box-shadow:var(--shadow-md)}
.pq-07 .dc-head{display:flex;align-items:center;gap:12px;padding:16px 18px;border-bottom:1px solid var(--border-subtle)}
.pq-07 .bell{width:30px;height:18px;border-radius:15px 15px 0 0;background:var(--accent);position:relative}
.pq-07 .bell::after{content:"";position:absolute;left:50%;bottom:-4px;transform:translateX(-50%);width:8px;height:3px;border-radius:2px;background:var(--accent-hover)}
.pq-07 .dc-head b{font-family:var(--serif);font-size:19px;font-weight:500}
.pq-07 .dc-head span{margin-left:auto;font-size:9.5px;color:var(--text-faint);letter-spacing:0.1em;text-transform:uppercase}
.pq-07 .dc-body{padding:12px 18px 16px}`,
`<div class="deskcard"><div class="dc-head"><span class="bell"></span><b>Vela</b><span>at your service</span></div><div class="dc-body">${wsRows()}${cta("New session")}</div></div>`);

V("pq", "08", "Ticket Gate", `.pq-08 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-08 .gate{display:flex;flex-direction:column;align-items:center;gap:14px}
.pq-08 .slot{width:260px;height:14px;border-radius:999px;background:var(--bg-inset);border:1px solid var(--border-strong);position:relative;overflow:visible}
.pq-08 .stub{position:absolute;left:50%;top:-58px;transform:translateX(-50%) rotate(-3deg);width:190px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:10px;padding:12px 16px;box-shadow:var(--shadow-md)}
.pq-08 .stub b{font-size:13px;font-weight:500}
.pq-08 .stub span{display:block;font-size:9.5px;color:var(--text-faint);margin-top:2px}
.pq-08 .post{width:8px;height:64px;background:var(--border-strong);border-radius:0 0 4px 4px}`,
`<div class="gate"><div class="slot"><div class="stub"><b>ADMIT ONE — vela</b><span>4 sessions queued</span></div></div><div class="post"></div>${cta("Insert & enter")}</div>`);

V("pq", "09", "Podium", `.pq-09 .mock{display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:56px 48px 44px;background:var(--bg)}
.pq-09 .riser{display:flex;flex-direction:column;align-items:center}
.pq-09 .r2,.pq-09 .r1{background:var(--bg-inset);border:1px solid var(--border-subtle);border-bottom:0;border-radius:12px 12px 0 0}
.pq-09 .r2{width:320px;height:26px}
.pq-09 .r1{width:270px;height:22px}
.pq-09 .plaque{width:220px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px 14px 0 0;padding:22px 20px 26px;text-align:center;box-shadow:var(--shadow-md)}
.pq-09 .mark-svg{width:26px;height:26px;margin-bottom:10px}
.pq-09 .title{font-size:24px;font-weight:500;margin:0 0 2px}
.pq-09 .engrave{font-size:9px;letter-spacing:0.18em;text-transform:uppercase;color:var(--text-faint);margin-top:8px}`,
`<div class="riser"><div class="plaque">${mark(26)}<h1 class="title">Vela</h1>${cta("Open")}</div><div class="r1"></div><div class="r2"><p style="margin:6px 0 0;font-size:9px;letter-spacing:0.16em;color:var(--text-faint);text-transform:uppercase;text-align:center">12 sessions · 3 workspaces · 2 live</p></div></div>`);

V("pq", "10", "Portico", `.pq-10 .mock{display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:64px 48px 54px;background:var(--bg)}
.pq-10 .lintel{width:300px;height:10px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:6px 6px 0 0}
.pq-10 .cols{width:300px;display:flex;justify-content:space-between;padding:0 26px}
.pq-10 .col{width:14px;height:74px;background:repeating-linear-gradient(90deg,var(--bg-inset) 0 4px,var(--bg-panel) 4px 8px);border:1px solid var(--border-strong);border-top:0}
.pq-10 .step{width:360px;height:8px;border:1px solid var(--border-subtle);border-bottom:0;background:var(--bg-inset);border-radius:4px 4px 0 0}
.pq-10 .cella{position:absolute;bottom:138px;left:50%;transform:translateX(-50%);width:250px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;padding:18px 22px;text-align:center;box-shadow:var(--shadow-md)}`,
`<div class="cella">${mark(24)}<h1 class="title" style="font-size:24px;font-weight:500;margin:6px 0 2px">Vela</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">3 workspaces await.</p>${cta("Enter")}</div><div class="lintel"></div><div class="cols"><span class="col"></span><span class="col"></span></div><div class="step"></div>`);

V("pq", "11", "Sluice Gate", `.pq-11 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-11 .wall{position:relative;width:340px;height:210px;border:1px solid var(--border-strong);border-radius:14px;background:var(--bg-inset);display:flex;align-items:center;justify-content:center;overflow:hidden}
.pq-11 .opening{width:120px;height:96px;border-radius:10px;background:linear-gradient(to top,var(--accent-dim),transparent 70%),var(--bg-panel);border:1px solid var(--border-strong);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:8px}
.pq-11 .bars{position:absolute;top:0;left:50%;transform:translateX(-50%);width:120px;height:56px;background:repeating-linear-gradient(90deg,var(--border-strong) 0 3px,transparent 3px 12px);border-radius:0 0 8px 8px}
.pq-11 .title{font-size:20px;font-weight:500;margin:0}
.pq-11 .enter{margin-top:2px;font-size:10px;color:var(--text-faint);letter-spacing:0.12em;text-transform:uppercase}`,
`<div class="wall"><div class="bars"></div><div class="opening"><span class="title">Vela</span><span class="enter">light on ↓</span></div></div>`);

V("pq", "12", "Vault Dial", `.pq-12 .mock{display:flex;align-items:center;justify-content:center;gap:30px;background:var(--bg);padding:48px}
.pq-12 .dial{position:relative;width:130px;height:130px;border-radius:50%;border:1.5px solid var(--border-strong);background:var(--bg-panel);box-shadow:var(--shadow-md);display:grid;place-items:center}
.pq-12 .dial::before{content:"";position:absolute;inset:10px;border-radius:50%;background:repeating-conic-gradient(var(--border-subtle) 0 3deg,transparent 3deg 12deg)}
.pq-12 .spoke{position:absolute;top:12px;left:50%;transform-origin:50% 53px;transform:rotate(40deg);width:3px;height:34px;border-radius:2px;background:var(--accent)}
.pq-12 .notch{position:absolute;top:-7px;left:50%;transform:translateX(-50%);width:4px;height:14px;border-radius:2px;background:var(--accent)}
.pq-12 .plaque{width:250px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:26px 28px;box-shadow:var(--shadow-md)}
.pq-12 .title{font-size:28px;font-weight:500;margin:0 0 4px}
.pq-12 .sub{margin:0 0 14px;color:var(--text-dim);font-size:11.5px}`,
`<div class="dial"><span class="spoke"></span><span class="notch"></span></div><div class="plaque">${kick("turn to open")}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open the vault")}${wsRows()}</div>`);

V("pq", "13", "Post Box", `.pq-13 .mock{display:flex;align-items:flex-end;justify-content:center;padding:64px 48px 56px;background:var(--bg)}
.pq-13 .box{position:relative;width:180px;height:200px;background:var(--accent);border-radius:14px 14px 8px 8px;display:flex;flex-direction:column;align-items:center;padding-top:26px;box-shadow:var(--shadow-md)}
.pq-13 .slot{width:96px;height:8px;border-radius:999px;background:#352a20;opacity:.55}
.pq-13 .flag{position:absolute;top:18px;right:-16px;width:12px;height:34px;background:#fbf7ec;border-radius:3px;transform:rotate(8deg)}
.pq-13 .flag::after{content:"";position:absolute;top:-8px;left:-2px;width:26px;height:14px;background:#fbf7ec;border-radius:3px}
.pq-13 .brand{margin-top:auto;margin-bottom:20px;color:#fbf7ec;font-family:var(--serif);font-size:21px}
.pq-13 .count{position:absolute;bottom:-30px;left:50%;transform:translateX(-50%);white-space:nowrap;font-size:10px;color:var(--text-faint);letter-spacing:0.1em;text-transform:uppercase}`,
`<div class="box"><span class="flag"></span><span class="slot"></span><span class="brand">Vela</span><span class="count">collections hourly · 2 live now</span></div>`);

V("pq", "14", "Lift Lobby", `.pq-14 .mock{display:flex;flex-direction:column;align-items:center;padding:44px 48px 0;background:var(--bg)}
.pq-14 .indic{display:flex;align-items:center;gap:10px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:12px;padding:8px 16px;font-family:ui-monospace,Menlo,monospace;font-size:12px;color:var(--text-faint)}
.pq-14 .indic b{color:var(--accent);font-weight:600}
.pq-14 .doors{margin-top:18px;display:flex;width:320px;height:250px;border:1px solid var(--border-strong);border-bottom:0;border-radius:12px 12px 0 0;overflow:hidden}
.pq-14 .leaf{flex:1;background:linear-gradient(var(--bg-panel),var(--bg-panel)) padding-box,repeating-linear-gradient(90deg,var(--border-subtle) 0 2px,transparent 2px 26px);border:1px solid var(--border-subtle)}
.pq-14 .leaf.l{border-right-width:6px;border-right-color:var(--accent)}
.pq-14 .summon{width:100%;background:var(--bg-inset);border-top:1px solid var(--border-subtle);display:flex;justify-content:center;padding:12px}`,
`<div class="indic">GND <b>▲</b> 1 <b>▲</b> 2</div><div class="doors"><div class="leaf l"></div><div class="leaf"></div></div><div class="summon">${cta("Summon Vela")}</div>`);

V("pq", "15", "Kiosk", `.pq-15 .mock{display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:56px 48px 40px;background:var(--bg)}
.pq-15 .screen{width:290px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:22px;transform:perspective(600px) rotateX(4deg);box-shadow:var(--shadow-md)}
.pq-15 .stand{width:12px;height:44px;background:var(--border-strong);border-radius:0 0 6px 6px}
.pq-15 .base{width:90px;height:10px;background:var(--border-strong);border-radius:999px}
.pq-15 .touchring{display:inline-flex;width:12px;height:12px;border:2px solid var(--accent);border-radius:50%;margin-left:6px;vertical-align:-2px}`,
`<div class="screen">${mark(26)}<h1 class="title" style="font-size:24px;font-weight:500;margin:8px 0 2px">Vela kiosk<span class="touchring"></span></h1><p style="margin:0 0 12px;font-size:11px;color:var(--text-dim)">Touch to begin · ${WS.length} workspaces attached</p>${wsRows()}</div><div class="stand"></div><div class="base"></div>`);

V("pq", "16", "Checkpoint", `.pq-16 .mock{position:relative;display:grid;place-items:center;background:var(--bg);overflow:hidden}
.pq-16 .lane{position:absolute;inset:0;background:repeating-linear-gradient(90deg,transparent 0 34px,var(--border-subtle) 34px 36px);}
.pq-16 .stopline{position:absolute;left:0;right:0;top:56%;height:10px;background:repeating-linear-gradient(90deg,var(--accent) 0 18px,transparent 18px 34px);opacity:.75}
.pq-16 .sign{position:relative;z-index:1;width:280px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:24px 26px;text-align:center;box-shadow:var(--shadow-md);margin-bottom:70px}
.pq-16 .sign .mark-svg{width:26px;height:26px;margin-bottom:10px}
.pq-16 .title{font-size:24px;font-weight:500;margin:0 0 2px}
.pq-16 .sub{margin:0 0 14px;color:var(--text-dim);font-size:11.5px}`,
`<div class="lane"></div><div class="stopline"></div><div class="sign">${mark(26)}<h1 class="title">Checkpoint</h1><p class="sub">2 agents cleared · queue empty</p>${cta("Enter")}</div>`);

V("pq", "17", "Footbridge", `.pq-17 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:center;background:var(--bg)}
.pq-17 .moat{position:absolute;left:60px;right:60px;bottom:110px;height:44px;border:1px solid var(--border-subtle);border-radius:10px;background-image:repeating-radial-gradient(circle at 12px 22px,var(--border-subtle) 0 1.5px,transparent 1.5px 22px)}
.pq-17 .plank{position:absolute;left:50%;bottom:118px;transform:translateX(-50%) rotate(-4deg);width:190px;height:12px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:6px;box-shadow:var(--shadow-sm)}
.pq-17 .far{position:absolute;bottom:160px;left:50%;transform:translateX(-50%);z-index:1;width:230px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:18px 22px;text-align:center;box-shadow:var(--shadow-md)}`,
`<div class="moat"></div><div class="plank"></div><div class="far">${mark(24)}<h1 class="title" style="font-size:23px;font-weight:500;margin:6px 0 2px">Cross into vela</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">3 workspaces across</p>${cta("Cross")}</div>`);

V("pq", "18", "Bell Curve", `.pq-18 .mock{position:relative;display:flex;flex-direction:column;align-items:center;padding:64px 48px 0;background:var(--bg)}
.pq-18 .curve{position:absolute;left:0;right:0;bottom:0;height:190px}
.pq-18 .curve path{fill:var(--bg-inset);stroke:var(--border-strong)}
.pq-18 .peakcard{position:relative;z-index:1;margin-bottom:132px;width:250px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:20px 24px;text-align:center;box-shadow:var(--shadow-md)}
.pq-18 .ticklbl{position:absolute;bottom:8px;left:0;right:0;display:flex;justify-content:space-around;font-size:8.5px;color:var(--text-faint);letter-spacing:0.1em}`,
`<svg class="curve" viewBox="0 0 400 100" preserveAspectRatio="none"><path d="M0 100 Q100 100 160 55 T200 8 Q205 4 210 8 T240 55 Q300 100 400 100 Z"/></svg><div class="peakcard">${mark(24)}<h1 class="title" style="font-size:24px;font-weight:500;margin:8px 0 2px">Vela</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">2 live sessions</p>${cta("Open a folder")}</div><div class="ticklbl"><span>rush</span><span>drift</span><span>peak</span><span>drift</span><span>rush</span></div>`);

V("pq", "19", "Monolith Mini", `.pq-19 .mock{position:relative;display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-19 .slab{position:relative;width:150px;height:210px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:10px;box-shadow:14px 18px 0 -2px var(--bg-inset),var(--shadow-md);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;text-align:center;padding:18px}
.pq-19 .title{font-size:24px;font-weight:500;margin:0}
.pq-19 .sub{margin:0;font-size:10.5px;color:var(--text-dim)}`,
`<div class="slab">${mark(26)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open")}</div>`);

V("pq", "20", "Lantern", `.pq-20 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-20 .halo{position:relative;width:250px;height:250px;border-radius:50%;background:radial-gradient(circle,var(--accent-dim),transparent 65%);display:grid;place-items:center}
.pq-20 .lamp{width:190px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:22px;text-align:center;box-shadow:var(--shadow-md)}
.pq-20 .chain{position:absolute;top:-64px;left:50%;width:2px;height:60px;background:var(--border-strong)}
.pq-20 .chain::after{content:"";position:absolute;top:-10px;left:-4px;width:10px;height:10px;border-radius:50%;background:var(--border-strong)}
.pq-20 .title{font-size:25px;font-weight:500;margin:0 0 2px}
.pq-20 .sub{margin:0 0 12px;font-size:11px;color:var(--text-dim)}`,
`<div class="halo"><span class="chain"></span><div class="lamp">${mark(24)}<h1 class="title">Vela</h1><p class="sub">2 live inside</p>${cta("Resume")}</div></div>`);

V("pq", "21", "Intercom", `.pq-21 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-21 .panel{width:230px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:18px;padding:26px 24px;text-align:center;box-shadow:var(--shadow-md)}
.pq-21 .grille{width:120px;height:44px;margin:0 auto 16px;background-image:radial-gradient(var(--border-strong) 1.6px,transparent 1.8px);background-size:12px 11px;border:1px solid var(--border-subtle);border-radius:8px}
.pq-21 .btn-round{width:56px;height:56px;border-radius:50%;background:var(--accent);color:var(--on-accent);display:grid;place-items:center;margin:14px auto 0;font-size:10.5px;font-weight:600;letter-spacing:0.08em}
.pq-21 .title{font-size:22px;font-weight:500;margin:0}
.pq-21 .sub{margin:6px 0 0;font-size:10.5px;color:var(--text-faint)}`,
`<div class="panel"><div class="grille"></div><h1 class="title">Vela reception</h1><p class="sub">Any hour, any workspace</p><div class="btn-round">PRESS</div></div>`);

V("pq", "22", "Turnstile Count", `.pq-22 .mock{display:flex;align-items:center;justify-content:center;gap:34px;background:var(--bg);padding:48px}
.pq-22 .rotor{position:relative;width:140px;height:140px}
.pq-22 .arm{position:absolute;left:50%;top:50%;width:56px;height:10px;border-radius:6px;background:var(--bg-inset);border:1px solid var(--border-strong);transform-origin:0 50%}
.pq-22 .a1{transform:rotate(10deg)}.pq-22 .a2{transform:rotate(130deg)}.pq-22 .a3{transform:rotate(250deg)}
.pq-22 .hub{position:absolute;left:50%;top:50%;width:22px;height:22px;margin:-11px;border-radius:50%;background:var(--accent)}
.pq-22 .plaque{width:240px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:24px 26px;box-shadow:var(--shadow-md)}
.pq-22 .big{font-family:var(--serif);font-size:44px;font-weight:500;line-height:1}
.pq-22 .cap{font-size:9.5px;letter-spacing:0.16em;text-transform:uppercase;color:var(--text-faint);margin:4px 0 14px}`,
`<div class="rotor"><span class="arm a1"></span><span class="arm a2"></span><span class="arm a3"></span><span class="hub"></span></div><div class="plaque"><span class="big">12</span><p class="cap">sessions through today</p>${cta("Enter")}${chips(0)}</div>`);

V("pq", "23", "Weigh Station", `.pq-23 .mock{display:flex;flex-direction:column;align-items:center;padding:52px 48px 0;background:var(--bg)}
.pq-23 .beam{position:relative;width:280px;height:2px;background:var(--border-strong);transform:rotate(-2deg);margin-bottom:34px}
.pq-23 .pan{position:absolute;top:8px;width:104px;height:64px;border:1px solid var(--border-strong);border-radius:0 0 12px 12px;background:var(--bg-panel);display:flex;flex-direction:column;align-items:center;justify-content:center;font-size:10.5px}
.pq-23 .pan.l{left:-14px}
.pq-23 .pan.r{right:-14px}
.pq-23 .fulcrum{width:16px;height:26px;background:var(--border-strong);clip-path:polygon(50% 0,100% 100%,0 100%)}
.pq-23 .base{width:120px;height:8px;border-radius:999px;background:var(--bg-inset);border:1px solid var(--border-subtle);margin-top:2px}
.pq-23 .verdict{margin-top:26px;text-align:center}`,
`<div class="beam"><div class="pan l"><b style="font-size:13px">3</b>workspaces</div><div class="pan r"><b style="font-size:13px">5</b>sessions live</div></div><div class="fulcrum"></div><div class="base"></div><div class="verdict">${mark(24)}<h1 class="title" style="font-size:24px;font-weight:500;margin:8px 0 2px">Balanced workload</h1>${cta("Open both")}</div>`);

V("pq", "24", "Toll Booth", `.pq-24 .mock{display:flex;align-items:center;justify-content:center;gap:0;background:var(--bg);padding:48px}
.pq-24 .booth{width:120px;height:190px;border:1px solid var(--border-strong);border-radius:12px 12px 0 0;background:repeating-linear-gradient(45deg,var(--accent) 0 12px,var(--bg-panel) 12px 24px);position:relative}
.pq-24 .window{position:absolute;top:26px;left:50%;transform:translateX(-50%);width:70px;height:44px;background:var(--bg);border:1px solid var(--border-strong);border-radius:8px;display:grid;place-items:center;font-family:var(--serif);font-size:15px}
.pq-24 .bar{width:190px;height:9px;border-radius:999px;background:repeating-linear-gradient(90deg,var(--accent) 0 16px,#fbf7ec 16px 32px);border:1px solid var(--border-strong);transform-origin:left center;transform:rotate(-14deg) translate(10px,-64px)}
.pq-24 .pass{margin-top:120px;margin-left:-40px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:10px;padding:12px 16px;font-size:10.5px;box-shadow:var(--shadow-sm)}`,
`<div class="booth"><span class="window">O</span></div><span class="bar"></span><div class="pass"><b>E-ZPASS VELA</b><br>3 workspaces · unlimited calm</div>`);

V("pq", "25", "Cat Flap", `.pq-25 .mock{position:relative;display:grid;place-items:center;background:var(--bg)}
.pq-25 .bigdoor{position:absolute;left:50%;top:44px;transform:translateX(-50%);width:320px;height:400px;border:1.5px solid var(--border-strong);border-radius:14px 14px 0 0;background:var(--bg-panel)}
.pq-25 .flap{position:absolute;left:50%;bottom:96px;transform:translateX(-50%);width:110px;height:86px;border:1.5px solid var(--accent);border-radius:55px 55px 8px 8px;background:linear-gradient(to top,var(--accent-dim),transparent 60%)}
.pq-25 .flap::after{content:"";position:absolute;top:10px;left:50%;transform:translateX(-50%);width:5px;height:5px;border-radius:50%;background:var(--accent)}
.pq-25 .invite{position:relative;z-index:1;margin-top:300px;text-align:center}`,
`<div class="bigdoor"></div><div class="flap"></div><div class="invite">${cta("Slip through")}</div>`);

V("pq", "26", "Service Bell", `.pq-26 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-26 .counter{display:flex;flex-direction:column;align-items:center;gap:16px}
.pq-26 .dome{position:relative;width:110px;height:58px;background:var(--accent);border-radius:55px 55px 0 0;box-shadow:var(--shadow-sm)}
.pq-26 .dome::before{content:"";position:absolute;top:-9px;left:50%;transform:translateX(-50%);width:12px;height:12px;border-radius:50%;background:var(--accent-hover)}
.pq-26 .dome::after{content:"DING";position:absolute;top:16px;left:50%;transform:translateX(-50%);color:var(--on-accent);font-size:11px;font-weight:700;letter-spacing:0.14em}
.pq-26 .ctr{width:220px;height:10px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:999px}
.pq-26 .note{text-align:center}`,
`<div class="counter"><span class="dome"></span><span class="ctr"></span><div class="note"><h1 class="title" style="font-size:24px;font-weight:500;margin:0 0 2px">Front desk, vela</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">Workspaces appear here</p>${cta("Ring")}</div></div>`);

V("pq", "27", "Meter Dial", `.pq-27 .mock{display:flex;align-items:center;justify-content:center;gap:30px;background:var(--bg);padding:48px}
.pq-27 .meter{width:120px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:14px 14px 8px 8px;padding:16px;text-align:center}
.pq-27 .face{width:84px;height:84px;margin:0 auto 10px;border-radius:50%;border:1.5px solid var(--border-strong);background:conic-gradient(var(--accent) 0 270deg,var(--bg-panel) 270deg 360deg);display:grid;place-items:center}
.pq-27 .face b{width:56px;height:56px;border-radius:50%;background:var(--bg-panel);display:grid;place-items:center;font-family:var(--serif);font-size:19px}
.pq-27 .coin{width:14px;height:34px;background:var(--border-strong);border-radius:4px;margin:0 auto}
.pq-27 .plaque{width:240px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:24px 26px;box-shadow:var(--shadow-md)}`,
`<div class="meter"><div class="face"><b>¾</b></div><span class="coin"></span><p style="margin:8px 0 0;font-size:9px;color:var(--text-faint);letter-spacing:0.1em">CAPACITY</p></div><div class="plaque">${kick("meter fed · plenty of time")}<h1 class="title" style="font-size:25px;font-weight:500;margin:0 0 4px">Vela</h1><p style="margin:0 0 12px;font-size:11.5px;color:var(--text-dim)">3 of 4 workspace slots humming.</p>${cta("Open")}</div>`);

V("pq", "28", "Cycle LEDs", `.pq-28 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-28 .air{width:300px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:26px 28px;text-align:center;box-shadow:var(--shadow-md)}
.pq-28 .ledrow{display:flex;justify-content:center;gap:12px;margin-bottom:16px}
.pq-28 .led{width:11px;height:11px;border-radius:50%;background:var(--border-strong)}
.pq-28 .led.on{background:var(--accent);box-shadow:0 0 0 3px var(--accent-dim)}
.pq-28 .led.warn{background:#c9a24a}
.pq-28 .seq{font-family:ui-monospace,Menlo,monospace;font-size:9.5px;color:var(--text-faint);margin-top:12px}`,
`<div class="air"><div class="ledrow"><span class="led on"></span><span class="led on"></span><span class="led warn"></span><span class="led"></span></div>${mark(24)}<h1 class="title" style="font-size:24px;font-weight:500;margin:10px 0 2px">Cycling in</h1><p style="margin:0 0 12px;font-size:11px;color:var(--text-dim)">Almost ready</p>${cta("Open a folder")}<p class="seq">[ok] adapters [ok] streams [..] agents</p></div>`);

V("pq", "29", "Reception Queue", `.pq-29 .mock{display:flex;flex-direction:column;align-items:center;padding:56px 48px 0;background:var(--bg)}
.pq-29 .deskfront{width:320px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:14px 14px 0 0;padding:18px 22px 22px;display:flex;align-items:center;gap:12px}
.pq-29 .plate{background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:8px;padding:8px 14px;display:flex;align-items:center;gap:8px}
.pq-29 .plate b{font-family:var(--serif);font-size:16px;font-weight:500}
.pq-29 .queue{margin-top:22px;display:flex;gap:10px}
.pq-29 .person{width:34px;height:34px;border-radius:50%;border:1px solid var(--border-strong);background:var(--bg-panel);display:grid;place-items:center;font-size:10px;color:var(--text-faint)}
.pq-29 .person.live{border-color:var(--accent);color:var(--accent)}`,
`<div class="deskfront"><div class="plate">${mark(18)}<b>Vela</b></div><span style="font-size:10px;color:var(--text-faint)">next please</span></div><div class="queue"><span class="person live">◉</span><span class="person">2</span><span class="person">3</span><span class="person">+</span></div><p style="margin:14px 0 18px;font-size:11px;color:var(--text-dim)">Two ahead of you · none waiting long</p>${cta("Join quietly")}`);

V("pq", "30", "Trap Door", `.pq-30 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-30 .hatch{position:relative;width:230px;height:150px;border:1.5px solid var(--border-strong);border-radius:12px;background:linear-gradient(#171310,#241d16);transform:perspective(400px) rotateX(38deg);box-shadow:var(--shadow-md)}
.pq-30 .rungs{position:absolute;left:50%;top:14px;bottom:14px;width:70px;transform:translateX(-50%);background:repeating-linear-gradient(to bottom,transparent 0 14px,color-mix(in srgb,#efe7db 45%,transparent) 14px 17px)}
.pq-30 .label{position:absolute;bottom:-34px;left:50%;transform:translateX(-50%);white-space:nowrap;font-size:10px;color:var(--text-faint);letter-spacing:0.12em;text-transform:uppercase}`,
`<div class="hatch"><span class="rungs"></span><span class="label">down to vela</span></div>`);

V("pq", "31", "Sky Bridge", `.pq-31 .mock{position:relative;display:flex;align-items:flex-end;justify-content:center;gap:70px;padding:56px 48px 0;background:var(--bg)}
.pq-31 .tower{width:110px;height:150px;background:var(--bg-inset);border:1px solid var(--border-strong);border-bottom:0;border-radius:8px 8px 0 0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:8px;font-size:10px;color:var(--text-faint)}
.pq-31 .bridge{position:absolute;bottom:96px;left:50%;transform:translateX(-50%);width:150px;height:26px;border:1px solid var(--border-strong);border-radius:8px;background:var(--bg-panel);display:grid;place-items:center;font-size:9px;letter-spacing:0.14em;color:var(--accent);text-transform:uppercase}
.pq-31 .t1 b,.pq-31 .t2 b{font-size:13px;color:var(--text)}`,
`<div class="tower t1"><b>You</b>warm side</div><div class="bridge">vela link ●</div><div class="tower t2"><b>Vela</b>calm side<br>3 desks</div>`);

V("pq", "32", "Ferry Slip", `.pq-32 .mock{position:relative;display:flex;flex-direction:column;align-items:center;padding:56px 48px 0;background:var(--bg)}
.pq-32 .water{position:absolute;left:0;right:0;bottom:0;height:80px;background-image:repeating-radial-gradient(circle at 16px 40px,var(--border-subtle) 0 1.5px,transparent 1.5px 26px);border-top:1px solid var(--border-subtle)}
.pq-32 .pilings{position:relative;z-index:1;display:flex;gap:26px;margin-top:120px}
.pq-32 .pil{width:12px;height:66px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:4px}
.pq-32 .gang{position:relative;z-index:1;width:200px;height:10px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:5px;margin-bottom:-6px}
.pq-32 .ticket{position:relative;z-index:1;margin-bottom:26px;text-align:center}`,
`<div class="ticket">${mark(24)}<h1 class="title" style="font-size:23px;font-weight:500;margin:6px 0 2px">Next ferry: vela</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">Departs when you are</p>${cta("Board")}</div><div class="gang"></div><div class="pilings"><span class="pil"></span><span class="pil"></span><span class="pil"></span></div><div class="water"></div>`);

V("pq", "33", "Funnel", `.pq-33 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-33 .funnel{position:relative;display:flex;flex-direction:column;align-items:center;gap:10px;margin-bottom:22px}
.pq-33 .fl{height:1px;background:var(--border-strong);border-radius:999px}
.pq-33 .f1{width:240px}.pq-33 .f2{width:190px}.pq-33 .f3{width:140px}.pq-33 .f4{width:90px}
.pq-33 .dotr{width:10px;height:10px;border-radius:50%;background:var(--accent);animation:pq33-fall 2.4s ease-in infinite}
@keyframes pq33-fall{0%{transform:translateY(-90px);opacity:0}25%{opacity:1}80%{opacity:1}100%{transform:translateY(0);opacity:0}}
.pq-33 .mouth{width:76px;height:12px;border:1.5px solid var(--accent);border-radius:8px;background:var(--bg-panel)}`, 
`<div class="funnel"><span class="dotr"></span><span class="fl f1"></span><span class="fl f2"></span><span class="fl f3"></span><span class="fl f4"></span><div class="mouth"></div></div>${kick("drop a folder anywhere")}${cta("Let it land")}`);

V("pq", "34", "Launch Rail", `.pq-34 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:56px 48px 40px;background:var(--bg)}
.pq-34 .rail{position:relative;width:300px;height:10px;border-radius:999px;background:var(--bg-inset);border:1px solid var(--border-strong);margin-top:26px}
.pq-34 .car{position:absolute;top:-52px;left:50%;transform:translateX(-50%);width:170px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;padding:14px 16px;text-align:center;box-shadow:var(--shadow-md)}
.pq-34 .wheels{position:absolute;bottom:-7px;left:50%;transform:translateX(-50%);display:flex;gap:22px}
.pq-34 .wheels i{width:12px;height:12px;border-radius:50%;background:var(--border-strong)}
.pq-34 .ignite{position:absolute;right:-34px;top:50%;transform:translateY(-50%);display:flex;gap:3px}
.pq-34 .ignite i{width:4px;height:14px;border-radius:2px;background:var(--accent);opacity:.85}`,
`<div class="rail"><div class="car">${mark(22)}<h1 class="title" style="font-size:20px;font-weight:500;margin:6px 0 2px">Vela</h1><p style="margin:0;font-size:10px;color:var(--text-faint)">cleared for launch</p><span class="wheels"><i></i><i></i></span><span class="ignite"><i></i><i></i><i></i></span></div></div>${kick("Ready when you are")}${cta("Ignite calmly")}`);

V("pq", "35", "Beacon Sweep", `.pq-35 .mock{position:relative;display:grid;place-items:center;background:var(--bg);overflow:hidden}
.pq-35 .cone{position:absolute;top:70px;left:50%;width:0;height:0;border-left:150px solid transparent;border-right:150px solid transparent;border-top:190px solid var(--accent-dim);transform-origin:top center;animation:pq35-sweep 7s ease-in-out infinite alternate}
@keyframes pq35-sweep{from{transform:translateX(-50%) rotate(-24deg)}to{transform:translateX(-50%) rotate(24deg)}}
.pq-35 .lamp{position:absolute;top:44px;left:50%;transform:translateX(-50%);width:26px;height:26px;border-radius:50%;background:var(--accent);box-shadow:0 0 0 6px var(--accent-dim)}
.pq-35 .plaque{position:relative;z-index:1;margin-top:210px;width:260px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:20px 24px;text-align:center;box-shadow:var(--shadow-md)}`,
`<span class="cone"></span><span class="lamp"></span><div class="plaque">${mark(24)}<h1 class="title" style="font-size:23px;font-weight:500;margin:6px 0 2px">Scanning now</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">2 found · both quiet</p>${cta("Open Vela")}</div>`);

V("pq", "36", "Drawbridge", `.pq-36 .mock{position:relative;display:flex;flex-direction:column;align-items:center;padding:52px 48px 0;background:var(--bg)}
.pq-36 .keep{width:250px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:20px 24px;text-align:center;box-shadow:var(--shadow-md)}
.pq-36 .chains{display:flex;justify-content:space-between;width:210px;height:26px}
.pq-36 .chains i{width:2px;background:repeating-linear-gradient(to bottom,var(--border-strong) 0 4px,transparent 4px 7px)}
.pq-36 .chains i:first-child{transform:rotate(14deg)}
.pq-36 .chains i:last-child{transform:rotate(-14deg)}
.pq-36 .deck{width:190px;height:12px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:6px}
.pq-36 .moat{width:280px;height:26px;border:1px solid var(--border-subtle);border-top:0;border-radius:0 0 12px 12px;background-image:repeating-radial-gradient(circle at 12px 13px,var(--border-subtle) 0 1.5px,transparent 1.5px 20px)}`,
`<div class="keep">${mark(24)}<h1 class="title" style="font-size:23px;font-weight:500;margin:6px 0 2px">Welcome back</h1><p style="margin:0;font-size:11px;color:var(--text-dim)">Drawbridge lowered · cross freely</p></div><div class="chains"><i></i><i></i></div><div class="deck"></div><div class="moat"></div>`);

V("pq", "37", "Revolving Door", `.pq-37 .mock{display:flex;align-items:center;justify-content:center;gap:36px;background:var(--bg);padding:48px}
.pq-37 .rev{position:relative;width:170px;height:170px;border:1.5px solid var(--border-strong);border-radius:50%;background:var(--bg-inset)}
.pq-37 .wing{position:absolute;left:50%;top:50%;width:82px;height:12px;margin:-6px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:6px;transform-origin:0 50%}
.pq-37 .w1{transform:rotate(15deg)}.pq-37 .w2{transform:rotate(135deg)}.pq-37 .w3{transform:rotate(255deg)}
.pq-37 .hub{position:absolute;left:50%;top:50%;width:16px;height:16px;margin:-8px;border-radius:50%;background:var(--accent)}
.pq-37 .lit{position:absolute;right:-6px;top:-6px;width:14px;height:14px;border-radius:50%;background:var(--accent);box-shadow:0 0 0 4px var(--accent-dim)}
.pq-37 .side{max-width:230px}`,
`<div class="rev"><span class="wing w1"></span><span class="wing w2"></span><span class="wing w3"></span><span class="hub"></span><span class="lit"></span></div><div class="side">${kick("one compartment spins free")}<h1 class="title" style="font-size:26px;font-weight:500;margin:0 0 4px">Step into Vela</h1><p style="margin:0 0 12px;font-size:11.5px;color:var(--text-dim)">One click to enter.</p>${cta("Step in")}</div>`);

V("pq", "38", "Escalator", `.pq-38 .mock{position:relative;display:flex;align-items:center;justify-content:center;background:var(--bg);padding:48px}
.pq-38 .stairs{position:absolute;right:70px;bottom:70px;width:190px;height:190px;transform:skewY(-16deg);display:grid;grid-template-rows:repeat(6,1fr);gap:4px}
.pq-38 .st{background:var(--bg-inset);border:1px solid var(--border-subtle);border-radius:4px}
.pq-38 .st:nth-child(2n){background:var(--bg-panel)}
.pq-38 .move{position:absolute;right:70px;bottom:70px;width:190px;height:190px;overflow:hidden;transform:skewY(-16deg)}
.pq-38 .dashline{position:absolute;left:-40%;right:-40%;height:2px;background:repeating-linear-gradient(90deg,var(--accent) 0 10px,transparent 10px 20px);animation:pq38-up 3s linear infinite}
@keyframes pq38-up{from{top:90%}to{top:-10%}}
.pq-38 .copy{max-width:240px;margin-right:auto}`,
`<div class="stairs"><span class="st"></span><span class="st"></span><span class="st"></span><span class="st"></span><span class="st"></span><span class="st"></span></div><div class="move"><span class="dashline"></span></div><div class="copy">${kick("standing lane stays left")}<h1 class="title" style="font-size:27px;font-weight:500;margin:0 0 4px">Ride up to Vela</h1><p style="margin:0 0 12px;font-size:11.5px;color:var(--text-dim)">Always this smooth.</p>${cta("Board upward")}</div>`);

V("pq", "39", "Turnstile Arms", `.pq-39 .mock{display:flex;align-items:center;justify-content:center;gap:34px;background:var(--bg);padding:48px}
.pq-39 .twisty{position:relative;width:130px;height:170px}
.pq-39 .post{position:absolute;left:50%;top:20px;bottom:0;width:12px;transform:translateX(-50%);background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:6px}
.pq-39 .arm{position:absolute;left:50%;width:88px;height:9px;border-radius:5px;background:var(--accent);opacity:.85;transform-origin:0 50%}
.pq-39 .r1{top:44px;transform:rotate(18deg)}
.pq-39 .r2{top:78px;transform:rotate(160deg)}
.pq-39 .r3{top:112px;transform:rotate(300deg)}
.pq-39 .footmarks{position:absolute;bottom:-6px;left:8px;display:flex;gap:8px}
.pq-39 .footmarks i{width:14px;height:5px;border-radius:999px;background:var(--border-subtle)}
.pq-39 .plaque{width:240px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:24px 26px;box-shadow:var(--shadow-md)}`,
`<div class="twisty"><span class="post"></span><span class="arm r1"></span><span class="arm r2"></span><span class="arm r3"></span><span class="footmarks"><i></i><i></i><i></i></span></div><div class="plaque">${kick("push · it yields")}<h1 class="title" style="font-size:25px;font-weight:500;margin:0 0 4px">Vela admits you</h1><p style="margin:0 0 12px;font-size:11.5px;color:var(--text-dim)">No waiting</p>${cta("Walk in")}</div>`);

V("pq", "40", "Security Desk", `.pq-40 .mock{position:relative;display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-40 .monitor{width:330px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;overflow:hidden;box-shadow:var(--shadow-md)}
.pq-40 .mhead{display:flex;align-items:center;gap:8px;padding:10px 14px;border-bottom:1px solid var(--border-subtle);font-size:10px;color:var(--text-faint);letter-spacing:0.12em;text-transform:uppercase}
.pq-40 .cam{margin-left:auto;width:8px;height:8px;border-radius:50%;background:var(--accent)}
.pq-40 .scanline{position:absolute;left:0;right:0;height:2px;background:var(--accent);opacity:.6;animation:pq40-scan 3.4s linear infinite}
@keyframes pq40-scan{from{top:20%}to{top:95%}}
.pq-40 .guests{padding:12px 14px}`,
`<div class="monitor"><div class="mhead">visitor log · vela <span class="cam"></span></div><span class="scanline"></span><div class="guests">${wsRows()}${cta("Sign in")}</div></div>`);

V("pq", "41", "Loading Dock", `.pq-41 .mock{display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:56px 48px 0;background:var(--bg)}
.pq-41 .dock{width:300px;height:230px;border:1.5px solid var(--border-strong);border-bottom:0;border-radius:14px 14px 0 0;background:var(--bg-inset);position:relative;overflow:hidden}
.pq-41 .slats{position:absolute;inset:0;background:repeating-linear-gradient(to bottom,var(--bg-panel) 0 22px,var(--border-subtle) 22px 24px);transform:translateY(-64px)}
.pq-41 .glowgap{position:absolute;left:0;right:0;bottom:0;height:64px;background:linear-gradient(to bottom,transparent,color-mix(in srgb,var(--accent) 26%,transparent))}
.pq-41 .bumper{display:flex;gap:26px;margin-top:0}
.pq-41 .bump{width:44px;height:14px;background:var(--border-strong);border-radius:0 0 8px 8px}`,
`<div class="dock"><div class="slats"></div><div class="glowgap"><p style="margin:18px 0 0;text-align:center;font-size:11px;color:var(--text);">Door half-open · warm light below</p></div></div><div class="bumper"><span class="bump"></span><span class="bump"></span></div><div style="padding:16px 0 22px">${cta("Roll on through")}</div>`);

V("pq", "42", "Cable Car", `.pq-42 .mock{position:relative;display:flex;flex-direction:column;align-items:center;padding:44px 48px 0;background:var(--bg)}
.pq-42 .cable{width:100%;height:2px;background:var(--border-strong);position:relative}
.pq-42 .car{position:absolute;top:2px;left:50%;transform:translateX(-50%);display:flex;flex-direction:column;align-items:center}
.pq-42 .hanger{width:3px;height:26px;background:var(--border-strong)}
.pq-42 .cab{width:170px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;padding:14px 16px;text-align:center;box-shadow:var(--shadow-md)}
.pq-42 .pylons{display:flex;justify-content:space-between;width:100%;margin-top:120px}
.pq-42 .pylon{width:10px;height:90px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:4px}
.pq-42 .ground{width:100%;height:8px;border-radius:999px;background:var(--bg-inset);border:1px solid var(--border-subtle);margin-bottom:20px}`,
`<div class="cable"><div class="car"><span class="hanger"></span><div class="cab">${mark(20)}<h1 class="title" style="font-size:19px;font-weight:500;margin:6px 0 2px">Over to vela</h1><p style="margin:0;font-size:10px;color:var(--text-faint)">smooth · scenic · silent</p></div></div></div><div class="pylons"><span class="pylon"></span><span class="pylon"></span></div><div class="ground"></div>${cta("Ride over")}`);

V("pq", "43", "Porthole Rise", `.pq-43 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-43 .port{position:relative;width:250px;height:250px;border:2px solid var(--border-strong);border-radius:50%;background:linear-gradient(#1c1712,#241d16);overflow:hidden;box-shadow:inset 0 0 0 10px var(--bg-panel),var(--shadow-md)}
.pq-43 .rise{position:absolute;left:50%;bottom:-70px;transform:translateX(-50%);width:120px;height:120px;border-radius:50%;background:var(--accent);opacity:.9;animation:pq43-rise 9s ease-in-out infinite alternate}
@keyframes pq43-rise{from{bottom:-90px}to{bottom:-30px}}
.pq-43 .rivets{position:absolute;inset:0}
.pq-43 .rivets i{position:absolute;width:6px;height:6px;border-radius:50%;background:var(--border-strong)}
.pq-43 .rv1{top:16px;left:50%;transform:translateX(-50%)}.pq-43 .rv2{bottom:16px;left:50%;transform:translateX(-50%)}.pq-43 .rv3{left:16px;top:50%;transform:translateY(-50%)}.pq-43 .rv4{right:16px;top:50%;transform:translateY(-50%)}
.pq-43 .under{margin-top:20px;text-align:center}`,
`<div class="port"><span class="rise"></span><span class="rivets"><i class="rv1"></i><i class="rv2"></i><i class="rv3"></i><i class="rv4"></i></span></div><div class="under">${kick("a calm world rises")}${cta("Through the glass")}</div>`);

V("pq", "44", "Luggage Tag", `.pq-44 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-44 .case{position:relative;width:300px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:18px;padding:30px 32px;text-align:center;box-shadow:var(--shadow-md)}
.pq-44 .case::before,.pq-44 .case::after{content:"";position:absolute;top:14px;bottom:14px;width:1px;background:var(--border-subtle)}
.pq-44 .case::before{left:84px}
.pq-44 .case::after{right:84px}
.pq-44 .strap{position:absolute;left:-10px;right:-10px;top:50%;height:26px;transform:translateY(-50%);background:var(--bg-inset);border:1px solid var(--border-subtle)}
.pq-44 .tag{position:absolute;top:26px;right:-34px;width:92px;background:var(--bg);border:1px solid var(--border-strong);border-radius:8px;padding:10px 12px;font-size:9.5px;color:var(--text-dim);transform:rotate(6deg);box-shadow:var(--shadow-sm)}
.pq-44 .tag b{display:block;color:var(--accent);font-size:10.5px;letter-spacing:0.08em}
.pq-44 .loop{position:absolute;top:-8px;left:50%;width:26px;height:12px;border:2px solid var(--border-strong);border-radius:999px;transform:translateX(-50%)}`,
`<div class="case"><span class="strap"></span>${mark(28)}<h1 class="title" style="font-size:26px;font-weight:500;margin:10px 0 2px">Checked in to Vela</h1><p style="margin:0 0 12px;font-size:11.5px;color:var(--text-dim)">3 workspaces inside</p>${cta("Open")}<div class="tag"><span class="loop"></span><b>VELA / MAIN</b>3 workspaces · 2 live</div></div>`);

V("pq", "45", "Gate Change", `.pq-45 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-45 .board{width:320px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:14px;padding:18px 20px;font-family:ui-monospace,"SF Mono",Menlo,monospace}
.pq-45 .brow{display:flex;gap:8px;margin-bottom:8px}
.pq-45 .flip{flex:1;height:36px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:6px;display:flex;align-items:center;justify-content:center;font-size:11px;position:relative}
.pq-45 .flip::after{content:"";position:absolute;left:0;right:0;top:50%;height:1px;background:color-mix(in srgb,var(--text) 14%,transparent)}
.pq-45 .flip.hot{background:var(--accent);color:var(--on-accent);border-color:var(--accent)}
.pq-45 .legend{margin-top:10px;font-size:9px;letter-spacing:0.16em;color:var(--text-faint);text-transform:uppercase;text-align:center}`,
`<div class="board"><div class="brow"><span class="flip">DEST</span><span class="flip hot">VELA</span><span class="flip">CALM</span></div><div class="brow"><span class="flip">GATE</span><span class="flip hot">OPEN</span><span class="flip">NOW</span></div><div class="legend">boarding now · 3 workspaces</div></div>`);

V("pq", "46", "Palm Scan", `.pq-46 .mock{display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-46 .pad{position:relative;width:210px;height:250px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:18px;display:grid;place-items:center;box-shadow:var(--shadow-md);overflow:hidden}
.pq-46 .palm{width:110px;height:150px;border:1.5px solid var(--accent);border-radius:46% 54% 40% 40%/60% 60% 34% 34%;position:relative;opacity:.9}
.pq-46 .palm::before,.pq-46 .palm::after{content:"";position:absolute;top:-26px;width:12px;height:44px;border:1.5px solid var(--accent);border-bottom:0;border-radius:8px 8px 0 0}
.pq-46 .palm::before{left:18px}
.pq-46 .palm::after{left:44px;height:36px}
.pq-46 .scan{position:absolute;left:0;right:0;height:3px;background:var(--accent);box-shadow:0 0 12px var(--accent);animation:pq46-scan 2.8s ease-in-out infinite}
@keyframes pq46-scan{0%,100%{top:16%}50%{top:84%}}
.pq-46 .ok{margin-top:14px;font-size:10px;letter-spacing:0.18em;color:var(--accent);text-transform:uppercase}`,
`<div class="pad"><div class="palm"></div><span class="scan"></span></div><p class="ok">signed in</p>`);

V("pq", "47", "Coin Slot", `.pq-47 .mock{display:flex;align-items:center;justify-content:center;gap:26px;background:var(--bg);padding:48px}
.pq-47 .coin{width:74px;height:74px;border-radius:50%;background:conic-gradient(from 210deg,var(--accent),var(--accent-hover));display:grid;place-items:center;color:var(--on-accent);font-family:var(--serif);font-size:24px;box-shadow:var(--shadow-sm)}
.pq-47 .machine{position:relative;width:230px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:26px 24px 20px;text-align:center;box-shadow:var(--shadow-md)}
.pq-47 .slot{width:86px;height:10px;border-radius:999px;background:var(--bg);border:1.5px solid var(--border-strong);margin:0 auto 14px;position:relative}
.pq-47 .slot::after{content:"";position:absolute;right:-30px;top:-8px;width:22px;height:22px;border-radius:50%;background:var(--accent);opacity:.25}
.pq-47 .title{font-size:23px;font-weight:500;margin:0 0 2px}
.pq-47 .sub{margin:0 0 12px;font-size:11px;color:var(--text-dim)}`,
`<div class="coin">¢</div><div class="machine"><div class="slot"></div><h1 class="title">Open Vela</h1><p class="sub">3 workspaces attached</p>${cta("Open")}</div>`);

V("pq", "48", "Signal Lamps", `.pq-48 .mock{display:flex;flex-direction:column;align-items:center;padding:56px 48px 0;background:var(--bg)}
.pq-48 .lamps{display:flex;gap:16px;margin-bottom:20px}
.pq-48 .lamp{width:30px;height:30px;border-radius:50%;border:1.5px solid var(--border-strong);background:var(--bg-inset)}
.pq-48 .lamp.go{background:var(--accent);border-color:var(--accent);box-shadow:0 0 0 5px var(--accent-dim)}
.pq-48 .bridge{width:240px;height:14px;border:1px solid var(--border-strong);border-radius:8px;background:var(--bg-panel);box-shadow:var(--shadow-sm)}
.pq-48 .trestle{display:flex;gap:18px;margin-top:0}
.pq-48 .leg{width:8px;height:44px;background:var(--bg-inset);border:1px solid var(--border-subtle);border-top:0}
.pq-48 .msg{margin-top:22px;text-align:center}`,
`<div class="lamps"><span class="lamp go"></span><span class="lamp"></span></div><div class="bridge"></div><div class="trestle"><span class="leg"></span><span class="leg"></span><span class="leg"></span></div><div class="msg">${mark(22)}<h1 class="title" style="font-size:22px;font-weight:500;margin:8px 0 2px">Clear to enter</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">Bridge clear · vela holding</p>${cta("Cross over")}</div>`);

V("pq", "49", "Space Dock", `.pq-49 .mock{position:relative;display:grid;place-items:center;background:var(--bg);padding:48px}
.pq-49 .clamp{position:absolute;top:50%;width:56px;height:20px;background:var(--bg-inset);border:1px solid var(--border-strong);border-radius:6px}
.pq-49 .cl-l{left:calc(50% - 150px);transform:translateY(-50%)}
.pq-49 .cl-r{right:calc(50% - 150px);transform:translateY(-50%)}
.pq-49 .umbilical{position:absolute;top:calc(50% - 60px);left:50%;transform:translateX(-50%);width:2px;height:52px;background:var(--border-strong)}
.pq-49 .capsule{position:relative;width:190px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:90px 90px 14px 14px;padding:30px 24px 22px;text-align:center;box-shadow:var(--shadow-md)}
.pq-49 .win{position:absolute;top:16px;left:50%;transform:translateX(-50%);width:34px;height:16px;border-radius:999px;background:var(--accent-dim);border:1px solid var(--accent)}`,
`<span class="clamp cl-l"></span><span class="clamp cl-r"></span><span class="umbilical"></span><div class="capsule"><span class="win"></span>${mark(24)}<h1 class="title" style="font-size:23px;font-weight:500;margin:12px 0 2px">Docked with Vela</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">Hard seal · all systems gentle</p>${cta("Open the hatch")}</div>`);

V("pq", "50", "Threshold Light", `.pq-50 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:64px 48px 0;background:var(--bg);overflow:hidden}
.pq-50 .wedge{position:absolute;top:0;left:50%;transform:translateX(-50%);width:0;height:0;border-left:190px solid transparent;border-right:190px solid transparent;border-top:230px solid color-mix(in srgb,var(--accent) 16%,transparent);filter:blur(1px)}
.pq-50 .beamline{position:absolute;top:0;left:50%;width:2px;height:210px;background:linear-gradient(var(--accent),transparent)}
.pq-50 .mat{position:relative;z-index:1;width:230px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px 12px 0 0;padding:20px 24px 26px;text-align:center;box-shadow:var(--shadow-md)}
.pq-50 .thresh{width:280px;height:10px;background:var(--bg-inset);border:1px solid var(--border-strong);border-bottom:0;border-radius:6px 6px 0 0}`,
`<span class="wedge"></span><span class="beamline"></span><div class="mat">${mark(24)}<h1 class="title" style="font-size:23px;font-weight:500;margin:8px 0 2px">Welcome in</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">3 workspaces ready</p>${cta("Step in")}</div><div class="thresh"></div>`);

// ---- watermark --------------------------------------------------------------

set("wm", "Watermark", "The ghost mark grows a supporting cast: workspaces, pulse, and quiet data behind the veil.");

V("wm", "01", "Double Exposure", `.wm-01 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-01 .ghost{position:absolute;left:50%;top:50%;transform:translate(-58%,-54%);opacity:.055;pointer-events:none}
.wm-01 .ghost2{position:absolute;left:50%;top:50%;transform:translate(-42%,-46%);opacity:.05;pointer-events:none}
.wm-01 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-01 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-01 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-01 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghost">${mark(300)}</span><span class="ghost2">${mark(300)}</span><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "02", "Corner Ghost", `.wm-02 .mock{padding:56px 48px;display:flex;flex-direction:column;justify-content:center}
.wm-02 .ghost{position:absolute;right:-70px;bottom:-70px;opacity:.06;pointer-events:none}
.wm-02 .hero{max-width:320px}
.wm-02 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.wm-02 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-02 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}
.wm-02 .ledgerline{margin-top:22px;border-top:1px solid var(--border-subtle);padding-top:12px;display:flex;gap:20px;font-size:10.5px;color:var(--text-faint)}`,
`<div class="ghost">${mark(260)}</div><div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="ledgerline"><span>vela · 4</span><span>atlas-notes · 2</span><span>quiet-web · 2</span><span>2 live</span></div>`);

V("wm", "03", "Ghost Ring", `.wm-03 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-03 .ring{position:absolute;left:50%;top:48%;transform:translate(-50%,-50%) rotate(-16deg);width:430px;height:190px;border:44px solid var(--accent);opacity:.07;border-radius:50%;pointer-events:none}
.wm-03 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-03 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-03 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-03 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ring"></span><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "04", "Fade Steps", `.wm-04 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-04 .g{position:absolute;pointer-events:none}
.wm-04 .g1{left:24px;top:30px;opacity:.09;transform:scale(.5);transform-origin:top left}
.wm-04 .g2{left:120px;top:110px;opacity:.05;transform:scale(.7);transform-origin:top left}
.wm-04 .g3{left:250px;top:220px;opacity:.03;transform:scale(.95);transform-origin:top left}
.wm-04 .inner{position:relative;z-index:1;display:flex;flex-direction:column;align-items:center;margin-top:60px}
.wm-04 .title{font-size:44px;font-weight:500;margin:14px 0 6px;letter-spacing:-0.02em}
.wm-04 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="g g1">${mark(120)}</span><span class="g g2">${mark(120)}</span><span class="g g3">${mark(120)}</span><div class="inner"><span style="display:block">${mark(30)}</span><h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "05", "Half Sunk", `.wm-05 .mock{display:flex;flex-direction:column;justify-content:flex-start;padding:56px 48px 0}
.wm-05 .ghost{position:absolute;left:50%;top:150px;transform:translateX(-50%);opacity:.07;pointer-events:none}
.wm-05::after{content:"";position:absolute;left:0;right:0;top:300px;height:1px;background:var(--border-strong)}
.wm-05 .hero{position:relative;z-index:1;text-align:center;display:flex;flex-direction:column;align-items:center}
.wm-05 .title{font-size:46px;font-weight:500;margin:8px 0 6px;letter-spacing:-0.02em}
.wm-05 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghost">${mark(340)}</span><div class="hero">${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "06", "Mark Column", `.wm-06 .mock{display:flex;padding:56px 0 56px 56px}
.wm-06 .col{position:absolute;left:26px;top:40px;bottom:40px;width:64px;display:flex;flex-direction:column;justify-content:space-between;opacity:.08;pointer-events:none}
.wm-06 .content{margin-left:80px;max-width:300px;display:flex;flex-direction:column;justify-content:center}
.wm-06 .mark-svg{width:34px;height:34px;margin-bottom:18px}
.wm-06 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-06 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<div class="col">${mark(64)}${mark(64)}${mark(64)}</div><div class="content">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${wsRows()}</div>`);

V("wm", "07", "Vellum Sheet", `.wm-07 .mock{display:grid;place-items:center;background:var(--bg)}
.wm-07 .underneath{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);opacity:.12;pointer-events:none}
.wm-07 .vellum{position:relative;width:330px;background:color-mix(in srgb,var(--bg-panel) 72%,transparent);border:1px solid var(--border-strong);border-radius:16px;padding:28px 32px;text-align:center;box-shadow:var(--shadow-md)}
.wm-07 .title{font-size:30px;font-weight:500;margin:10px 0 4px}
.wm-07 .sub{margin:0 0 16px;color:var(--text-dim);font-size:11.5px}`,
`<span class="underneath">${mark(240)}</span><div class="vellum">${kick("tracing paper over the machine")}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Peek under")}${stats()}</div>`);

V("wm", "08", "Outline Only", `.wm-08 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-08 .ghost path,.wm-08 .ghost circle{stroke-width:6}
.wm-08 .ghost{position:absolute;inset:0;display:grid;place-items:center;opacity:.06;pointer-events:none}
.wm-08 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-08 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-08 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-08 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghost">${mark(360)}</span><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${wsRows()}</div>`);

V("wm", "09", "Motion Trail", `.wm-09 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-09 .trail{position:absolute;left:50%;top:44%;transform:translate(-50%,-50%);pointer-events:none}
.wm-09 .t1{opacity:.10}.wm-09 .t2{opacity:.06;transform:translate(-16px,10px)}.wm-09 .t3{opacity:.03;transform:translate(-32px,20px)}
.wm-09 .inner{position:relative;display:flex;flex-direction:column;align-items:center;margin-top:40px}
.wm-09 .title{font-size:44px;font-weight:500;margin:12px 0 6px;letter-spacing:-0.02em}
.wm-09 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="trail t1">${mark(120)}</span><span class="trail t2">${mark(120)}</span><span class="trail t3">${mark(120)}</span><div class="inner"><span>${mark(30)}</span><h1 class="title">Vela</h1><p class="sub">Two agents live now.</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "10", "Figures on Ghost", `.wm-10 .mock{display:flex;align-items:center;padding:56px 52px}
.wm-10 .ghost{position:absolute;right:30px;top:50%;transform:translateY(-50%);opacity:.07;pointer-events:none}
.wm-10 .figures{position:relative;z-index:1;width:210px;margin-left:auto}
.wm-10 .fig{display:flex;align-items:baseline;gap:8px;padding:10px 0;border-bottom:1px solid var(--border-subtle)}
.wm-10 .fig b{font-family:var(--serif);font-size:30px;font-weight:500;line-height:1}
.wm-10 .fig span{font-size:9.5px;letter-spacing:0.12em;text-transform:uppercase;color:var(--text-faint)}
.wm-10 .left{max-width:270px;margin-right:auto}`,
`<span class="ghost">${mark(280)}</span><div class="left">${mark(34)}<h1 class="title" style="font-size:44px;font-weight:500;margin:16px 0 6px;letter-spacing:-0.02em">Vela</h1><p style="margin:0 0 18px;font-size:12.5px;color:var(--text-dim)">${SUB}</p>${cta("Open a folder")}</div><div class="figures"><div class="fig"><b>12</b><span>sessions</span></div><div class="fig"><b>3</b><span>workspaces</span></div><div class="fig"><b>2</b><span>agents live</span></div></div>`);

V("wm", "11", "Split Dissolve", `.wm-11 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-11 .ghostwrap{position:absolute;inset:0;display:grid;place-items:center;pointer-events:none;-webkit-mask-image:linear-gradient(100deg,black 35%,transparent 75%);mask-image:linear-gradient(100deg,black 35%,transparent 75%)}
.wm-11 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-11 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-11 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-11 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghostwrap">${mark(330)}</span><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${chips(0)}</div>`);

V("wm", "12", "Ghost Atlas", `.wm-12 .mock{padding:52px 48px;background-image:linear-gradient(var(--grid) 1px,transparent 1px),linear-gradient(90deg,var(--grid) 1px,transparent 1px);background-size:40px 40px}
.wm-12 .atlas{position:absolute;inset:0;pointer-events:none}
.wm-12 .city{position:absolute;transform:translate(-50%,-50%);display:flex;flex-direction:column;align-items:center;gap:4px;font-size:8.5px;color:var(--text-faint);letter-spacing:0.08em}
.wm-12 .city svg{opacity:.5}
.wm-12 .hero{position:relative;max-width:280px;margin-top:60px}
.wm-12 .title{font-size:44px;font-weight:500;margin:14px 0 6px;letter-spacing:-0.02em}
.wm-12 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="atlas"><span class="city" style="left:22%;top:26%">vela<br>${mark(34)}</span><span class="city" style="left:66%;top:38%">atlas-notes<br>${mark(28)}</span><span class="city" style="left:44%;top:74%">quiet-web<br>${mark(30)}</span></div><div class="hero"><h1 class="title">Your three workspaces</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "13", "Echo Type", `.wm-13 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:52px}
.wm-13 .echo{position:absolute;left:50%;top:44%;transform:translate(-50%,-50%);font-family:var(--serif);font-size:200px;font-weight:500;letter-spacing:-0.04em;color:var(--text);opacity:.045;white-space:nowrap;pointer-events:none}
.wm-13 .inner{position:relative;text-align:center;display:flex;flex-direction:column;align-items:center;margin-top:90px}
.wm-13 .mark-svg{width:32px;height:32px;margin-bottom:14px}
.wm-13 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-13 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="echo">Vela Vela</span><div class="inner">${mark(32)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "14", "Prismatic", `.wm-14 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-14 .layer{position:absolute;left:50%;top:47%;pointer-events:none}
.wm-14 .l1{transform:translate(-62%,-58%);color:#617a68;opacity:.14}
.wm-14 .l2{transform:translate(-50%,-50%);color:#49708f;opacity:.10}
.wm-14 .l3{transform:translate(-38%,-42%);color:#aa624f;opacity:.08}
.wm-14 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-14 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-14 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-14 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="layer l1" width="240" height="240" viewBox="0 0 96 96">${""}<ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(24 48 48)" fill="none" stroke="currentColor" stroke-width="4"/><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(-24 48 48)" fill="none" stroke="currentColor" stroke-width="4"/></svg><svg class="layer l2" width="240" height="240" viewBox="0 0 96 96"><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(24 48 48)" fill="none" stroke="currentColor" stroke-width="4"/><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(-24 48 48)" fill="none" stroke="currentColor" stroke-width="4"/></svg><svg class="layer l3" width="240" height="240" viewBox="0 0 96 96"><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(24 48 48)" fill="none" stroke="currentColor" stroke-width="4"/><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(-24 48 48)" fill="none" stroke="currentColor" stroke-width="4"/></svg><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "15", "Ghost Ribbon", `.wm-15 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-15 .ribbon{position:absolute;left:-30px;right:-30px;top:120px;height:130px;opacity:.07;pointer-events:none}
.wm-15 .ribbon path{fill:none;stroke:var(--accent);stroke-width:40;stroke-linecap:round}
.wm-15 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-15 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-15 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-15 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="ribbon" viewBox="0 0 400 130" preserveAspectRatio="none"><path d="M-20 90 C80 10 180 140 260 60 S420 30 440 70"/></svg><div class="inner">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "16", "Knockout Ink", `.wm-16{--bg:#241d16;--bg-panel:#2c241d;--bg-inset:#332a21;--text:#efe7db;--text-dim:#b6a894;--text-faint:#8a7d69;--accent:#9eb4a1;--accent-hover:#b2c4b4;--accent-dim:rgba(158,180,161,0.16);--border:rgba(255,255,255,0.08);--border-subtle:rgba(255,255,255,0.05);--border-strong:rgba(255,255,255,0.16);--on-accent:#172019}
.wm-16 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background:var(--bg-panel)}
.wm-16 .knock{position:absolute;left:50%;top:42%;transform:translate(-50%,-50%);opacity:.5;pointer-events:none;filter:brightness(0) saturate(0)}
.wm-16 .inner{position:relative;display:flex;flex-direction:column;align-items:center;margin-top:60px}
.wm-16 .title{font-size:44px;font-weight:500;margin:12px 0 6px;letter-spacing:-0.02em}
.wm-16 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="knock">${mark(280)}</span><div class="inner"><span>${mark(30)}</span><h1 class="title">Vela after dark</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "17", "Arc Queue", `.wm-17 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-17 .ghost{position:absolute;inset:0;display:grid;place-items:center;opacity:.06;pointer-events:none}
.wm-17 .arcrow{position:relative;display:flex;gap:14px;margin-top:26px}
.wm-17 .arc{width:118px;padding:10px 12px;border:1px solid var(--border-subtle);border-radius:10px;background:var(--bg-panel);text-align:left;font-size:10.5px}
.wm-17 .arc b{display:block;font-weight:500}
.wm-17 .arc span{color:var(--text-faint);font-size:9.5px}
.wm-17 .arc:nth-child(1){transform:rotate(-4deg) translateY(8px)}
.wm-17 .arc:nth-child(2){transform:translateY(-4px)}
.wm-17 .arc:nth-child(3){transform:rotate(4deg) translateY(8px)}`,
`<span class="ghost">${mark(320)}</span><div style="position:relative">${mark(34)}<h1 class="title" style="font-size:42px;font-weight:500;margin:14px 0 6px;letter-spacing:-0.02em">Vela</h1><p style="margin:0 0 6px;color:var(--text-dim);font-size:12.5px">${SUB}</p>${cta("Open a folder")}<div class="arcrow">${RECENTS.slice(0, 3).map((r) => `<div class="arc"><b>${r.name}</b><span>${r.when}</span></div>`).join("")}</div></div>`);

V("wm", "18", "Depth Planes", `.wm-18 .mock{display:grid;place-items:center;background:var(--bg)}
.wm-18 .stack{position:relative;width:360px;height:400px}
.wm-18 .plane{position:absolute;border-radius:16px}
.wm-18 .back{inset:0;border:1px solid var(--border-subtle);background:var(--bg-panel);display:grid;place-items:center;overflow:hidden}
.wm-18 .back svg{opacity:.08;transform:scale(1.6)}
.wm-18 .mid{inset:34px;border:1px solid var(--border-strong);background:color-mix(in srgb,var(--bg-panel) 88%,transparent);display:flex;flex-direction:column;justify-content:flex-end;padding:18px 20px}
.wm-18 .front{inset:78px 78px auto auto;width:170px;background:var(--bg-panel);border:1px solid var(--border-strong);box-shadow:var(--shadow-md);border-radius:14px;padding:16px;text-align:center}
.wm-18 .mid .cap{font-size:9.5px;letter-spacing:0.14em;text-transform:uppercase;color:var(--text-faint)}`,
`<div class="stack"><div class="plane back">${mark(200)}<span></span></div><div class="plane mid"><span class="cap">behind the glass</span></div><div class="plane front">${mark(24)}<h1 class="title" style="font-size:21px;font-weight:500;margin:8px 0 2px">Vela</h1><p style="margin:0 0 8px;font-size:9.5px;color:var(--text-faint)">3 planes deep</p>${cta("Focus front")}</div></div>`);

V("wm", "19", "Long Exposure", `.wm-19 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-19 .blurset{position:absolute;left:50%;top:46%;transform:translate(-50%,-50%);filter:blur(2px);opacity:.10;pointer-events:none}
.wm-19 .sk1{transform:translateX(-14px) skewX(-6deg)}
.wm-19 .sk2{transform:translateX(10px) skewX(-3deg);opacity:.6}
.wm-19 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-19 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-19 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-19 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="blurset"><svg class="sk1" width="280" height="280" viewBox="0 0 96 96"><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(24 48 48)" fill="none" stroke="#617a68" stroke-width="4"/></svg><svg class="sk2" width="280" height="280" viewBox="0 0 96 96" style="position:absolute;inset:0"><ellipse cx="48" cy="48" rx="36" ry="15" transform="rotate(-24 48 48)" fill="none" stroke="#617a68" stroke-width="4"/></svg></span><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "20", "Compass Needle", `.wm-20 .mock{padding:56px 48px}
.wm-20 .ghost{position:absolute;left:40px;top:40px;bottom:40px;display:flex;align-items:center;opacity:.07;pointer-events:none}
.wm-20 .needle{position:absolute;left:150px;top:60px;bottom:60px;width:1px;background:repeating-linear-gradient(to bottom,var(--border-strong) 0 6px,transparent 6px 12px)}
.wm-20 .needle::before{content:"N";position:absolute;top:-16px;left:-4px;font-size:9px;color:var(--text-faint)}
.wm-20 .content{margin-left:170px;max-width:280px;display:flex;flex-direction:column;justify-content:center;height:100%}
.wm-20 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-20 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-20 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<span class="ghost">${mark(230)}</span><span class="needle"></span><div class="content">${mark(34)}<h1 class="title">Where you left off</h1><p class="sub">${SUB}</p>${cta("Choose a direction")}${wsRows()}</div>`);

V("wm", "21", "Clipped Corner", `.wm-21 .mock{padding:56px 48px;display:flex;flex-direction:column;justify-content:center}
.wm-21 .clipghost{position:absolute;top:-60px;right:-60px;width:240px;height:240px;border-radius:50%;display:grid;place-items:center;background:var(--accent);opacity:.08;pointer-events:none}
.wm-21 .clipghost svg{transform:scale(.55)}
.wm-21 .hero{max-width:300px}
.wm-21 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-21 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-21 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="clipghost">${mark(300)}</div><div class="hero">${mark(34)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "22", "Mark Family", `.wm-22 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-22 .big{position:absolute;left:-70px;bottom:-70px;opacity:.06;pointer-events:none}
.wm-22 .sats{position:absolute;right:60px;top:60px;display:flex;gap:16px;opacity:.12;pointer-events:none}
.wm-22 .inner{position:relative;display:flex;flex-direction:column;align-items:center;margin-left:120px}
.wm-22 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-22 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-22 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="big">${mark(300)}</span><span class="sats">${mark(54)}${mark(40)}${mark(30)}</span><div class="inner">${mark(36)}<h1 class="title">Vela &amp; kin</h1><p class="sub">One mark, many workspaces.</p>${cta("Open a folder")}${wsRows()}</div>`);

V("wm", "23", "Waterline", `.wm-23 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:0 48px 40px}
.wm-23::before{content:"";position:absolute;left:0;right:0;top:52%;height:1px;background:var(--border-strong)}
.wm-23 .above{position:absolute;top:calc(52% - 190px);left:0;right:0;display:flex;flex-direction:column;align-items:center;gap:12px}
.wm-23 .reflect{position:absolute;top:calc(52% + 14px);transform:scaleY(-1);opacity:.05;pointer-events:none}
.wm-23 .panel{position:relative;z-index:1;display:flex;gap:18px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;padding:10px 14px;font-size:10px;color:var(--text-dim)}`,
`<div class="above">${mark(150)}<h1 class="title" style="font-size:38px;font-weight:500;margin:0;letter-spacing:-0.02em">Where things settled</h1></div><span class="reflect">${mark(150)}</span><div class="panel"><span>◉ 2 live</span><span>3 workspaces</span><span>depth: calm</span></div>`);

V("wm", "24", "Construction Dashed", `.wm-24 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-24 .cons{position:absolute;inset:0;pointer-events:none;opacity:.5}
.wm-24 .cons ellipse,.wm-24 .cons line{fill:none;stroke:var(--border-strong);stroke-dasharray:4 5}
.wm-24 .cross{fill:var(--accent)}
.wm-24 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-24 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-24 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-24 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="cons" viewBox="0 0 400 300" preserveAspectRatio="xMidYMid slice"><ellipse cx="200" cy="150" rx="170" ry="70" transform="rotate(-14 200 150)"/><ellipse cx="200" cy="150" rx="170" ry="70" transform="rotate(14 200 150)"/><line x1="200" y1="30" x2="200" y2="270"/><circle class="cross" cx="252" cy="98" r="3"/></svg><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "25", "Dot Mosaic Mark", `.wm-25 .mock{display:flex;align-items:center;padding:56px 48px;gap:40px}
.wm-25 .mosaic{position:absolute;right:40px;top:50%;transform:translateY(-50%);display:grid;grid-template-columns:repeat(6,14px);gap:7px;opacity:.5;pointer-events:none}
.wm-25 .mosaic i{width:14px;height:14px;border-radius:50%;background:var(--border-strong)}
.wm-25 .mosaic i.on{background:var(--accent)}
.wm-25 .hero{max-width:280px;z-index:1}
.wm-25 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-25 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-25 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<div class="mosaic">${Array.from({ length: 36 }, (_, i) => `<i class="${[0, 7, 8, 14, 15, 21, 27, 28].includes(i) ? "on" : ""}"></i>`).join("")}</div><div class="hero">${mark(34)}<h1 class="title">Vela in dots</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "26", "Ink Bleed", `.wm-26 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-26 .bleed{position:absolute;left:50%;top:44%;transform:translate(-50%,-50%);width:300px;height:300px;border-radius:50%;background:radial-gradient(circle,color-mix(in srgb,var(--accent) 22%,transparent),transparent 65%);filter:blur(14px);pointer-events:none}
.wm-26 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-26 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-26 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-26 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="bleed"></span><div class="inner">${mark(36)}<h1 class="title">Focus, by default</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "27", "Entering Marks", `.wm-27 .mock{padding:56px 48px;display:flex;align-items:flex-end;justify-content:space-between}
.wm-27 .stair{position:absolute;inset:0;pointer-events:none}
.wm-27 .stepm{position:absolute;opacity:.07}
.wm-27 .content{position:relative;z-index:1;max-width:290px;margin-left:auto;text-align:right;display:flex;flex-direction:column;align-items:flex-end}
.wm-27 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-27 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-27 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<div class="stair"><span class="stepm" style="left:30px;bottom:40px;transform:scale(.5);transform-origin:bottom left">${mark(160)}</span><span class="stepm" style="left:150px;bottom:120px;transform:scale(.7);transform-origin:bottom left">${mark(160)}</span><span class="stepm" style="left:290px;bottom:220px;opacity:.1;transform:scale(.9);transform-origin:bottom left">${mark(160)}</span></div><div class="content">${mark(34)}<h1 class="title">Steps to stillness</h1><p class="sub">${SUB}</p>${cta("Keep climbing")}${stats()}</div>`);

V("wm", "28", "Embossed Seal", `.wm-28 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-28 .seal{position:absolute;left:50%;top:45%;transform:translate(-50%,-50%);width:250px;height:250px;border-radius:50%;border:1.5px dashed var(--border-strong);display:grid;place-items:center;opacity:.55;pointer-events:none}
.wm-28 .seal::after{content:"";position:absolute;inset:14px;border-radius:50%;border:1px solid var(--border-strong)}
.wm-28 .seal svg{opacity:.14}
.wm-28 .inner{position:relative;margin-top:120px;display:flex;flex-direction:column;align-items:center}
.wm-28 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-28 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="seal">${mark(120)}</div><div class="inner"><h1 class="title">Sealed, not sealed off</h1><p class="sub">${SUB}</p>${cta("Open all")}${stats()}</div>`);

V("wm", "29", "Gridlock Intersect", `.wm-29 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;background-image:linear-gradient(var(--border-subtle) 1px,transparent 1px),linear-gradient(90deg,var(--border-subtle) 1px,transparent 1px);background-size:34px 34px}
.wm-29 .ghost{position:absolute;inset:0;display:grid;place-items:center;opacity:.09;pointer-events:none}
.wm-29 .nodespot{position:absolute;width:9px;height:9px;border-radius:50%;background:var(--accent)}
.wm-29 .n1{left:116px;top:150px}.wm-29 .n2{left:218px;top:184px}.wm-29 .n3{left:320px;top:116px}
.wm-29 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-29 .title{font-size:44px;font-weight:500;margin:14px 0 6px;letter-spacing:-0.02em}
.wm-29 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghost">${mark(300)}</span><span class="nodespot n1"></span><span class="nodespot n2"></span><span class="nodespot n3"></span><div class="inner"><span>${mark(30)}</span><h1 class="title">Intersections, kept few</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "30", "Gemini Twins", `.wm-30 .mock{display:flex;align-items:center;justify-content:space-around;padding:56px 40px}
.wm-30 .twin{position:relative;width:200px;height:380px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center}
.wm-30 .halo{position:absolute;inset:0;display:grid;place-items:center;opacity:.06;pointer-events:none}
.wm-30 .link{width:60px;height:1px;background:var(--border-strong);position:relative}
.wm-30 .link::after{content:"";position:absolute;left:50%;top:-3px;width:7px;height:7px;border-radius:50%;background:var(--accent);transform:translateX(-50%)}
.wm-30 .title{font-size:30px;font-weight:500;margin:10px 0 4px;letter-spacing:-0.02em}
.wm-30 .side{font-size:10.5px;color:var(--text-faint);max-width:120px}`,
`<div class="twin"><span class="halo">${mark(190)}</span>${mark(40)}<h1 class="title">Sessions</h1><p class="side">what is happening now</p></div><span class="link"></span><div class="twin"><span class="halo">${mark(190)}</span>${wsTiles()}<h1 class="title">Workspaces</h1><p class="side">where it happens</p></div>`);

V("wm", "31", "Horizon Sun", `.wm-31 .mock{display:flex;flex-direction:column;align-items:center;padding:64px 48px 0}
.wm-31 .sun{opacity:.08;pointer-events:none;margin-bottom:14px}
.wm-31 .title{font-size:38px;font-weight:500;margin:0 0 4px;letter-spacing:-0.02em}
.wm-31 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12px}
.wm-31 .horizon{width:100%;height:1px;background:var(--border-strong);margin-top:auto}
.wm-31 .ground{width:100%;height:58px;background-color:var(--bg-inset);background-image:repeating-linear-gradient(90deg,var(--border-subtle) 0 1px,transparent 1px 26px);display:flex;align-items:center;justify-content:center;gap:22px;font-size:10.5px;color:var(--text-dim)}
.wm-31 .ground .live{color:var(--accent);font-weight:600}`,
`${mark(150)}<h1 class="title">Good morning</h1><p class="sub">Two sessions kept warm overnight.</p>${cta("Resume")}<div class="horizon"></div><div class="ground"><span class="live">\u25c9 vela \u00b7 live</span><span>atlas-notes \u00b7 idle</span><span>quiet-web \u00b7 idle</span></div>`);

V("wm", "32", "Tally Census", `.wm-32 .mock{padding:56px 48px;display:flex;flex-direction:column;justify-content:center}
.wm-32 .ghost{position:absolute;right:-40px;top:-40px;opacity:.06;pointer-events:none}
.wm-32 .tallies{position:absolute;left:44px;bottom:44px;display:flex;gap:18px;opacity:.5}
.wm-32 .gate5{display:flex;gap:3px;align-items:center;position:relative}
.wm-32 .gate5 i{width:1.5px;height:22px;background:var(--border-strong);display:inline-block}
.wm-32 .gate5 s{position:absolute;width:26px;height:1.5px;background:var(--accent);transform:rotate(-24deg)}
.wm-32 .hero{max-width:310px}
.wm-32 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-32 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-32 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghost">${mark(260)}</span><div class="tallies">${[0,1,2].map(()=>`<span class="gate5"><i></i><i></i><i></i><i></i><s></s></span>`).join("")}</div><div class="hero">${mark(34)}<h1 class="title">The week so far</h1><p class="sub">Fifteen sessions this week across three workspaces.</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "33", "Paths Only", `.wm-33 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-33 .paths{position:absolute;inset:0;pointer-events:none}
.wm-33 .paths ellipse{fill:none;stroke:var(--accent);opacity:.16;stroke-width:1.5}
.wm-33 .moon{fill:var(--accent)}
.wm-33 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-33 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-33 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-33 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<svg class="paths" viewBox="0 0 400 300" preserveAspectRatio="xMidYMid slice"><ellipse cx="200" cy="150" rx="175" ry="66" transform="rotate(-14 200 150)"/><ellipse cx="200" cy="150" rx="120" ry="44" transform="rotate(-14 200 150)"/><circle class="moon" cx="322" cy="102" r="5"/></svg><div class="inner">${mark(36)}<h1 class="title">Every session, one click away</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${wsRows()}</div>`);

V("wm", "34", "Stamp Roll", `.wm-34 .mock{display:grid;place-items:center;background-image:radial-gradient(var(--border-strong) 1.2px,transparent 1.4px);background-size:14px 14px}
.wm-34 .stamp{position:relative;width:300px;background:var(--bg-panel);border:2px dashed var(--border-strong);border-radius:4px;padding:30px 34px;text-align:center}
.wm-34 .perf{position:absolute;inset:-7px;background-image:radial-gradient(circle,var(--bg) 3.5px,transparent 4px);pointer-events:none}
.wm-34 .mark-svg{opacity:.9;width:30px;height:30px;margin-bottom:12px}
.wm-34 .title{font-size:28px;font-weight:500;margin:0 0 2px}
.wm-34 .sub{margin:0 0 14px;color:var(--text-dim);font-size:11.5px}`,
`<div class="stamp"><span class="perf"></span>${mark(30)}<h1 class="title">First-class calm</h1><p class="sub">${SUB}</p>${cta("New session")}${stats()}</div>`);

V("wm", "35", "Veil Wash", `.wm-35 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-35 .veil{position:absolute;inset:0;background:linear-gradient(115deg,transparent 30%,color-mix(in srgb,var(--accent) 14%,transparent) 55%,transparent 80%);pointer-events:none}
.wm-35 .veilghost{position:absolute;inset:0;display:grid;place-items:center;opacity:.07;pointer-events:none}
.wm-35 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-35 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-35 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-35 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="veilghost">${mark(320)}</span><span class="veil"></span><div class="inner">${mark(36)}<h1 class="title">Recent activity</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "36", "Fossil Imprint", `.wm-36 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-36 .imprint{position:absolute;left:50%;top:45%;transform:translate(-50%,-50%);opacity:.5;pointer-events:none;filter:drop-shadow(2px 3px 0 var(--bg)) drop-shadow(0 0 1px var(--border))}
.wm-36 .imprint svg{opacity:.16;filter:none}
.wm-36 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-36 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-36 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-36 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="imprint">${mark(300)}</span><div class="inner">${mark(36)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open everything")}${wsRows()}</div>`);

V("wm", "37", "Dot Nebula", `.wm-37 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-37 .neb{position:absolute;inset:0;pointer-events:none}
.wm-37 .neb i{position:absolute;border-radius:50%;background:var(--border-strong)}
.wm-37 .neb i.a{background:var(--accent)}
.wm-37 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-37 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-37 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-37 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<div class="neb">${[[15,20,3],[25,60,2],[40,35,4,"a"],[55,75,2],[62,25,3],[70,55,4,"a"],[80,40,2],[85,70,3],[33,82,3],[12,55,2]].map(([x,y,s,k])=>`<i class="${k||""}" style="left:${x}%;top:${y}%;width:${s}px;height:${s}px"></i>`).join("")}</div><div class="inner">${mark(36)}<h1 class="title">Jump into a workspace</h1><p class="sub">${SUB}</p>${cta("Browse")}${stats()}</div>`);

V("wm", "38", "Marquee Letters", `.wm-38 .mock{display:flex;flex-direction:column;justify-content:center;padding:52px 40px}
.wm-38 .marq{position:absolute;left:0;right:0;top:46%;transform:translateY(-50%);display:flex;justify-content:center;gap:8vw;font-family:var(--serif);font-size:150px;font-weight:500;line-height:1;color:var(--text);opacity:.04;white-space:nowrap;pointer-events:none}
.wm-38 .inner{position:relative;text-align:center;margin-top:60px}
.wm-38 .title{font-size:40px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-38 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="marq"><span>O</span><span>R</span><span>B</span><span>I</span><span>T</span></div><div class="inner">${mark(30)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "39", "Echo Frames", `.wm-39 .mock{display:grid;place-items:center;padding:48px}
.wm-39 .framestack{position:absolute;inset:0;pointer-events:none}
.wm-39 .fr{position:absolute;border:1px solid var(--border-strong);border-radius:22px;opacity:.5}
.wm-39 .f1{inset:34px}
.wm-39 .f2{inset:58px;opacity:.3}
.wm-39 .f3{inset:82px;opacity:.16}
.wm-39 .corecard{position:relative;z-index:1;width:300px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:16px;padding:26px 30px;text-align:center;box-shadow:var(--shadow-md)}
.wm-39 .title{font-size:27px;font-weight:500;margin:10px 0 2px}
.wm-39 .sub{margin:0 0 14px;color:var(--text-dim);font-size:11.5px}`,
`<div class="framestack"><span class="fr f1"></span><span class="fr f2"></span><span class="fr f3"></span></div><div class="corecard">${mark(28)}<h1 class="title">Vela</h1><p class="sub">${SUB}</p>${cta("Enter innermost")}${wsRows()}</div>`);

V("wm", "40", "Tide Pools", `.wm-40 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px;overflow:hidden}
.wm-40 .pool{position:absolute;border-radius:50%;pointer-events:none}
.wm-40 .p1{width:220px;height:220px;left:-60px;bottom:-70px;background:var(--accent);opacity:.10}
.wm-40 .p2{width:150px;height:150px;left:90px;bottom:-40px;background:var(--accent);opacity:.07}
.wm-40 .p3{width:110px;height:110px;left:230px;bottom:-30px;background:var(--border-strong);opacity:.10}
.wm-40 .inner{position:relative;display:flex;flex-direction:column;align-items:center;margin-bottom:60px}
.wm-40 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-40 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-40 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<span class="pool p1"></span><span class="pool p2"></span><span class="pool p3"></span><div class="inner">${mark(34)}<h1 class="title">Pick a session</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "41", "Circuit Fade", `.wm-41 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-41 .circ{position:absolute;inset:0;pointer-events:none}
.wm-41 .circ path{fill:none;stroke:var(--accent);opacity:.18}
.wm-41 .circ circle{fill:var(--bg-panel);stroke:var(--accent)}
.wm-41 .ghostcore{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);opacity:.06;pointer-events:none}
.wm-41 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-41 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-41 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-41 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghostcore">${mark(240)}</span><svg class="circ" viewBox="0 0 400 300" preserveAspectRatio="none"><path d="M200 150 L200 60 L280 60"/><path d="M200 150 L120 150 L120 230"/><path d="M200 150 L320 150 L320 90"/><path d="M200 150 L80 80"/><circle cx="280" cy="60" r="5"/><circle cx="120" cy="230" r="5"/><circle cx="320" cy="90" r="5"/><circle cx="80" cy="80" r="5"/></svg><div class="inner">${mark(36)}<h1 class="title">Back to work</h1><p class="sub">${SUB}</p>${cta("Trace a session")}${wsRows()}</div>`);

V("wm", "42", "Weather Vane", `.wm-42 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:0 48px 40px}
.wm-42 .pole{position:absolute;left:50%;bottom:110px;width:1.5px;height:230px;background:var(--border-strong)}
.wm-42 .vane{position:absolute;left:50%;bottom:318px;transform:translateX(-50%);opacity:.9}
.wm-42 .dirs{position:absolute;left:50%;bottom:250px;transform:translateX(-50%);display:flex;gap:34px;font-size:9px;color:var(--text-faint);letter-spacing:0.1em}
.wm-42 .card{position:relative;z-index:1;width:280px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:14px;padding:20px 24px;text-align:center;box-shadow:var(--shadow-md)}`,
`<span class="pole"></span><span class="vane">${mark(56)}</span><span class="dirs"><span>NW drift</span><span>CALM E</span></span><div class="card">${kick("2 live sessions")}<h1 class="title" style="font-size:23px;font-weight:500;margin:0 0 2px">Any direction works</h1><p style="margin:0 0 10px;font-size:11px;color:var(--text-dim)">Both desks are one click away.</p>${cta("Pick any")}</div>`);

V("wm", "43", "Ledger Texture", `.wm-43 .mock{display:flex;align-items:center;padding:56px 48px}
.wm-43 .texture{position:absolute;right:30px;top:40px;bottom:40px;width:200px;overflow:hidden;opacity:.10;pointer-events:none;font-family:ui-monospace,Menlo,monospace;font-size:10px;line-height:1.9;color:var(--text);white-space:pre}
.wm-43 .hero{max-width:290px;z-index:1}
.wm-43 .mark-svg{width:34px;height:34px;margin-bottom:16px}
.wm-43 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-43 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<div class="texture">${Array.from({length:14},(_,r)=>Array.from({length:8},(_,c)=>String((r*8+c)%97).padStart(2,"0")).join(" ")).join("\n")}</div><div class="hero">${mark(34)}<h1 class="title">This week</h1><p class="sub">${SUB}</p>${cta("Read the sky")}${stats()}</div>`);

V("wm", "44", "Aurora Bands", `.wm-44 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-44 .aur{position:absolute;inset:0;pointer-events:none;overflow:hidden}
.wm-44 .band{position:absolute;top:-40px;bottom:-40px;width:70px;transform:rotate(14deg);background:linear-gradient(to bottom,transparent,var(--accent-dim),transparent)}
.wm-44 .b1{left:18%}.wm-44 .b2{left:38%;opacity:.7}.wm-44 .b3{left:58%;opacity:.5}.wm-44 .b4{left:78%;opacity:.35}
.wm-44 .ghost{position:absolute;inset:0;display:grid;place-items:center;opacity:.06;pointer-events:none}
.wm-44 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-44 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-44 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-44 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ghost">${mark(300)}</span><div class="aur"><span class="band b1"></span><span class="band b2"></span><span class="band b3"></span><span class="band b4"></span></div><div class="inner">${mark(36)}<h1 class="title">Resume a session</h1><p class="sub">${SUB}</p>${cta("Open a folder")}${stats()}</div>`);

V("wm", "45", "Stampede Horizon", `.wm-45 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-45 .herd{position:absolute;left:0;right:0;bottom:70px;height:120px;pointer-events:none}
.wm-45 .herd svg{position:absolute;opacity:.08}
.wm-45 .inner{position:relative;display:flex;flex-direction:column;align-items:center;margin-bottom:80px}
.wm-45 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-45 .title{font-size:44px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-45 .sub{margin:0 0 18px;color:var(--text-dim);font-size:12.5px}`,
`<div class="herd">${[[0,70,90],[110,84,70],[210,76,80],[320,90,56],[420,64,100],[520,82,66]].map(([x,b,w])=>`<span style="position:absolute;left:${x}px;bottom:${b - 60}px">${mark(w)}</span>`).join("")}</div><div class="inner">${mark(36)}<h1 class="title">Today's sessions</h1><p class="sub">${SUB}</p>${cta("Open today")}${stats()}</div>`);

V("wm", "46", "Stitch Outline", `.wm-46 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-46 .stitch{position:absolute;inset:0;display:grid;place-items:center;pointer-events:none}
.wm-46 .stitch *{stroke-dasharray:6 7!important;opacity:.22}
.wm-46 .inner{position:relative;display:flex;flex-direction:column;align-items:center}
.wm-46 .mark-svg{width:36px;height:36px;margin-bottom:18px}
.wm-46 .title{font-size:46px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-46 .sub{margin:0 0 20px;color:var(--text-dim);font-size:12.5px}
.wm-46 .hem{margin-top:22px;border-top:1px dashed var(--border-strong);padding-top:10px;width:100%;max-width:300px;display:flex;justify-content:center;gap:16px;font-size:9.5px;color:var(--text-faint);letter-spacing:0.1em;text-transform:uppercase}`,
`<span class="stitch">${mark(300)}</span><div class="inner">${mark(36)}<h1 class="title">Hand-stitched software</h1><p class="sub">${SUB}</p>${cta("Open a folder")}</div><div class="hem"><span>seam: sage</span><span>tension: calm</span><span>3 panels</span></div>`);

V("wm", "47", "Eclipse Knockout", `.wm-47 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-47 .disc{position:absolute;left:50%;top:45%;transform:translate(-50%,-50%);width:260px;height:260px;border-radius:50%;background:var(--accent);opacity:.14}
.wm-47 .disc svg{transform:scale(1.15);mix-blend-mode:normal}
.wm-47 .cutout{position:absolute;left:50%;top:45%;transform:translate(-50%,-50%);width:260px;height:260px;border-radius:50%;display:grid;place-items:center;pointer-events:none}
.wm-47 .cutout svg{filter:brightness(0) opacity(.5)}
.wm-47 .cutout{background:var(--bg-panel);border-radius:50%;}
.wm-47 .ring{position:absolute;left:50%;top:45%;transform:translate(-50%,-50%);width:284px;height:284px;border-radius:50%;border:1px solid var(--border-subtle)}
.wm-47 .inner{position:relative;margin-top:170px;display:flex;flex-direction:column;align-items:center}
.wm-47 .title{font-size:40px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-47 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12px}`,
`<span class="disc">${mark(260)}</span><span class="ring"></span><span class="cutout">${mark(210)}</span><div class="inner"><h1 class="title">One moment of focus</h1><p class="sub">${SUB}</p>${cta("Reveal")}${stats()}</div>`);

V("wm", "48", "Sonar Rings", `.wm-48 .mock{display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:52px}
.wm-48 .ping{position:absolute;left:50%;top:44%;transform:translate(-50%,-50%);width:80px;height:80px;border-radius:50%;border:1.5px solid var(--accent);animation:wmping 3s ease-out infinite}
.wm-48 .p2{animation-delay:1s}.wm-48 .p3{animation-delay:2s}
@keyframes wmping{from{width:80px;height:80px;opacity:.5}to{width:420px;height:420px;opacity:0}}
.wm-48 .coredot{position:absolute;left:50%;top:44%;transform:translate(-50%,-50%);width:10px;height:10px;border-radius:50%;background:var(--accent)}
.wm-48 .inner{position:relative;margin-top:150px;display:flex;flex-direction:column;align-items:center}
.wm-48 .title{font-size:42px;font-weight:500;margin:0 0 6px;letter-spacing:-0.02em}
.wm-48 .sub{margin:0 0 16px;color:var(--text-dim);font-size:12.5px}`,
`<span class="ping"></span><span class="ping p2"></span><span class="ping p3"></span><span class="coredot"></span><div class="inner">${mark(28)}<h1 class="title">Found 2 sessions</h1><p class="sub">Both are recent work.</p>${cta("Bring them up")}${stats()}</div>`);

V("wm", "49", "Archive Fan", `.wm-49 .mock{display:grid;place-items:center;background:var(--bg)}
.wm-49 .fan{position:relative;width:340px;height:330px}
.wm-49 .sheet{position:absolute;left:50%;top:50%;width:280px;height:190px;background:var(--bg-panel);border:1px solid var(--border-strong);border-radius:12px;display:grid;place-items:center;overflow:hidden}
.wm-49 .sheet svg{opacity:.08;transform:scale(1.4)}
.wm-49 .s1{transform:translate(-50%,-50%) rotate(-10deg)}
.wm-49 .s2{transform:translate(-50%,-50%) rotate(-3deg)}
.wm-49 .top{transform:translate(-50%,-50%);z-index:2;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:18px}
.wm-49 .title{font-size:23px;font-weight:500;margin:8px 0 2px}
.wm-49 .sub{font-size:10px;color:var(--text-dim)}`,
`<div class="fan"><div class="sheet s1">${mark(140)}</div><div class="sheet s2">${mark(140)}</div><div class="sheet top">${mark(120)}<span></span><h1 class="title">Archive, alive</h1><p class="sub">${SUB}</p>${cta("Fan it open")}</div></div>`);

V("wm", "50", "Zenith Rays", `.wm-50 .mock{position:relative;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;padding:0 48px 44px;overflow:hidden}
.wm-50 .zenith{position:absolute;top:-90px;left:50%;transform:translateX(-50%);opacity:.08;pointer-events:none}
.wm-50 .ray{position:absolute;top:-20px;left:50%;width:1px;height:190px;background:linear-gradient(var(--accent),transparent);transform-origin:top center}
.wm-50 .r1{transform:rotate(-24deg)}.wm-50 .r2{transform:rotate(-8deg)}.wm-50 .r3{transform:rotate(8deg)}.wm-50 .r4{transform:rotate(24deg)}
.wm-50 .content{position:relative;z-index:1;text-align:center;margin-top:150px}`,
`<span class="zenith">${mark(260)}</span><span class="ray r1"></span><span class="ray r2"></span><span class="ray r3"></span><span class="ray r4"></span><div class="content">${mark(28)}<h1 class="title" style="font-size:36px;font-weight:500;margin:10px 0 4px;letter-spacing:-0.02em">Start where you stopped</h1><p style="margin:0 0 14px;font-size:12px;color:var(--text-dim)">No shadows to argue with.</p>${cta("Step into the light")}</div>`);

// ---- shared css -------------------------------------------------------------

const IT_BASE = `:root {
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
.mock { position: relative; background: var(--bg-panel); border: 1px solid var(--border); border-radius: 20px; overflow: hidden; min-height: 520px; }
.mark-svg { display: block; }
.title { font-family: var(--serif); }
.kick { font-size: 9.5px; letter-spacing: 0.14em; text-transform: uppercase; color: var(--text-faint); margin: 0 0 12px; }
.rows, .ws { list-style: none; margin: 0; padding: 0; }
.row, .ws-row { display: flex; align-items: center; gap: 10px; padding: 9px 2px; border-bottom: 1px solid var(--border-subtle); }
.row:last-child, .ws-row:last-child { border-bottom: 0; }
.row-dot, .ws-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--border-strong); flex: none; }
.row.is-live .row-dot, .ws-row.is-live .ws-dot { background: var(--accent); }
.row-name { font-size: 12px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.row-meta { margin-left: auto; padding-left: 10px; font-size: 10px; color: var(--text-faint); white-space: nowrap; }
.ws-name { display: block; font-size: 12px; font-weight: 500; }
.ws-desc { display: block; margin-top: 2px; font-size: 9.5px; color: var(--text-faint); }
.ws-count { margin-left: auto; font-size: 10px; color: var(--text-faint); border: 1px solid var(--border); border-radius: var(--radius-full); padding: 2px 8px; }
.seg { display: inline-flex; gap: 3px; background: var(--bg-inset); border-radius: var(--radius-full); padding: 3px; align-self: flex-start; }
.seg span { padding: 6px 14px; border-radius: var(--radius-full); font-size: 11.5px; color: var(--text-dim); }
.seg span.on { background: var(--bg-panel); color: var(--text); font-weight: 500; box-shadow: var(--shadow-sm); }
.sel { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: 10px; padding: 9px 12px; font-size: 12px; font-weight: 500; color: var(--text); }
.sel svg { color: var(--text-faint); }
.chips { display: flex; gap: 7px; flex-wrap: wrap; margin-top: 12px; }
.chip { font-size: 10.5px; padding: 4px 11px; border: 1px solid var(--border-subtle); border-radius: var(--radius-full); color: var(--text-dim); }
.chip.on { background: var(--accent); border-color: var(--accent); color: var(--on-accent); }
.jump { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 18px; width: 100%; }
.jmp { flex: 1; min-width: 150px; display: flex; align-items: center; gap: 10px; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: 12px; padding: 12px 14px; text-align: left; box-shadow: var(--shadow-sm); }
.jmp:hover { background: var(--bg-hover); }
.jmp svg { margin-left: auto; color: var(--text-faint); flex: none; }
.jmp-t b { display: block; font-size: 12.5px; font-weight: 500; }
.jmp-t span { display: block; margin-top: 2px; font-size: 10px; color: var(--text-faint); }
.go { display: inline-flex; margin-left: 8px; color: var(--text-faint); flex: none; }
.ws-main { min-width: 0; }
.wtiles { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-top: 4px; }
.wtile { text-align: left; background: var(--bg-panel); border: 1px solid var(--border); border-radius: 12px; padding: 12px 14px; min-width: 0; }
.wtile.is-live { border-color: var(--accent); }
.wtile-name { display: block; font-size: 12px; font-weight: 500; }
.wtile-meta { display: block; margin-top: 3px; font-size: 9.5px; color: var(--text-faint); }
.cta-row { display: flex; align-items: center; gap: 10px; }
.btn { appearance: none; border: 1px solid var(--border-strong); background: transparent; color: var(--text); border-radius: var(--radius-full); padding: 9px 18px; font-size: 12.5px; font-weight: 500; }
.btn:hover { background: var(--bg-hover); }
.btn-primary { background: var(--accent); border-color: var(--accent); color: var(--on-accent); }
.btn-primary:hover { background: var(--accent-hover); border-color: var(--accent-hover); }`;

// ---- gallery shell ----------------------------------------------------------

const CHROME = `
.it-shell { max-width: 1560px; margin: 0 auto; padding: 40px 28px 80px; }
.it-head { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; flex-wrap: wrap; padding-bottom: 20px; border-bottom: 1px solid var(--border); }
.it-kicker { color: var(--accent); font-size: 9.5px; font-weight: 600; letter-spacing: 0.13em; text-transform: uppercase; }
.it-head h1 { margin: 6px 0 4px; font-family: var(--serif); font-size: clamp(26px, 2.4vw, 36px); font-weight: 500; letter-spacing: -0.03em; line-height: 1; }
.it-head h1 em { font-style: italic; font-weight: 400; color: var(--accent); }
.it-head p { margin: 0; color: var(--text-dim); font-size: 12.5px; max-width: 66ch; }
.it-controls { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.it-search { display: flex; align-items: center; gap: 7px; min-height: 34px; padding: 0 12px; background: var(--bg-inset); border-radius: 12px; }
.it-search input { width: 200px; border: 0; background: transparent; color: var(--text); font-size: 12px; outline: 0; }
.it-search input::placeholder { color: var(--text-faint); }
.it-chip { min-height: 34px; display: inline-flex; align-items: center; gap: 6px; padding: 0 12px; background: var(--bg-inset); color: var(--text-dim); border: 0; border-radius: 12px; font-size: 11.5px; cursor: pointer; }
.it-back { color: var(--text-faint); text-decoration: none; font-size: 11.5px; border: 1px solid var(--border); border-radius: 999px; padding: 8px 14px; }
.it-back:hover { color: var(--text); background: var(--bg-hover); }
.it-grid { margin-top: 24px; display: grid; grid-template-columns: repeat(auto-fill, minmax(400px, 1fr)); gap: 18px; }
.it-card { background: var(--bg-panel); border: 1px solid var(--border); border-radius: 16px; overflow: hidden; transition: box-shadow 0.15s ease, border-color 0.15s ease; }
.it-card:hover { border-color: var(--border-strong); box-shadow: var(--shadow-md); }
.it-card.is-picked { border-color: var(--accent); }
.it-card.is-hidden { display: none; }
.it-card-head { display: flex; align-items: center; gap: 8px; padding: 10px 14px; border-bottom: 1px solid var(--border-subtle); }
.it-star { border: 0; background: transparent; color: var(--text-faint); font-size: 14px; padding: 2px 4px; line-height: 1; cursor: pointer; }
.it-card.is-picked .it-star { color: var(--accent); }
.it-id { color: var(--accent); font-size: 11px; font-weight: 600; letter-spacing: 0.06em; }
.it-name { font-weight: 500; font-size: 12.5px; }
.it-stage { position: relative; overflow: hidden; background: var(--bg); }
.it-zoom { width: 760px; transform-origin: 0 0; pointer-events: none; }
.it-zoom .mock { border: 0; border-radius: 0; }
.it-empty { display: none; margin-top: 32px; text-align: center; color: var(--text-faint); font-size: 12px; }
.it-empty.is-on { display: block; }
`;

const SCRIPT = `<script>
(function () {
  var KEY = "vela-landing-shortlist";
  var picked = [];
  try { picked = JSON.parse(localStorage.getItem(KEY) || "[]"); } catch (e) { picked = []; }
  var count = document.getElementById("it-count");
  var q = document.getElementById("it-q");
  var empty = document.getElementById("it-empty");
  var cards = Array.prototype.slice.call(document.querySelectorAll(".it-card"));
  function save() { try { localStorage.setItem(KEY, JSON.stringify(picked)); } catch (e) {} }
  function paint() {
    cards.forEach(function (card) {
      var on = picked.indexOf(card.dataset.id) >= 0;
      card.classList.toggle("is-picked", on);
      card.querySelector(".it-star").textContent = on ? "\\u2605" : "\\u2606";
    });
    count.textContent = "Shortlist " + picked.length + "/4";
  }
  function filter() {
    var needle = q.value.trim().toLowerCase();
    var shown = 0;
    cards.forEach(function (card) {
      var hit = (card.dataset.id + " " + card.dataset.name).toLowerCase().indexOf(needle) >= 0;
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
    else return;
    save();
    paint();
  });
  q.addEventListener("input", filter);
  document.getElementById("it-theme").addEventListener("click", function () {
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
    Array.prototype.forEach.call(document.querySelectorAll(".it-stage"), function (stage) {
      var scale = stage.clientWidth / 760;
      stage.firstElementChild.style.transform = "scale(" + scale + ")";
      stage.style.height = Math.ceil(520 * scale) + "px";
    });
  }
  window.addEventListener("resize", fit);
  paint();
  filter();
  fit();
})();
</script>`;

function shell(code) {
  const s = SETS[code];
  const upper = code.toUpperCase();
  const cards = s.items.map(
    (v) => `<section class="it-card" data-id="${code}-${v.id}" data-name="${v.name.toLowerCase()}">
<header class="it-card-head"><button class="it-star" type="button" data-star="${code}-${v.id}" title="Shortlist">\u2606</button><span class="it-id">${upper}\u00b7${v.id}</span><span class="it-name">${v.name}</span></header>
<div class="it-stage"><div class="it-zoom"><div class="${code}-${v.id}"><div class="mock">${v.body}</div></div></div></div>
</section>`
  ).join("\n");
  return `<!doctype html>
<html lang="en" data-theme="paper">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>Vela — ${s.title}, 50 variations</title>
<link rel="stylesheet" href="./iterations.css"/>
<style>${CHROME}</style>
</head>
<body>
<div class="it-shell">
<header class="it-head">
<div><span class="it-kicker">Vela · landing iterations</span><h1>${s.title} <em>\u00b7 50 variations</em></h1><p>${s.blurb}</p></div>
<div class="it-controls">
<a class="it-back" href="../index.html">\u2190 All directions</a>
<label class="it-search"><input id="it-q" type="search" placeholder="Search ${upper}-01… names…" autocomplete="off"/></label>
<button id="it-theme" class="it-chip" type="button">\u25d0 Theme</button>
<span id="it-count" class="it-chip">Shortlist 0/4</span>
</div>
</header>
<main class="it-grid" id="it-grid">
${cards}
</main>
<p class="it-empty" id="it-empty">Nothing matches that filter.</p>
</div>
${SCRIPT}
</body>
</html>
`;
}

// ---- emit -------------------------------------------------------------------

mkdirSync(join(here, "iterations"), { recursive: true });
writeFileSync(join(here, "iterations", "iterations.css"), [IT_BASE, ...Object.values(SETS).flatMap((s) => s.items.map((v) => v.css))].join("\n"));
for (const code of Object.keys(SETS)) writeFileSync(join(here, "iterations", `${code}.html`), shell(code));
console.log(Object.entries(SETS).map(([code, s]) => `${code}: ${s.items.length} iterations (${code}.html)`).join(", "));


