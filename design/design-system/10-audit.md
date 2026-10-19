# 10 — Audit & History

> **Sources:** `redesign/rebrand.md:1`, `redesign/logo.html:1`, `resources/icon.svg:1`, `src/renderer/src/components/VelaMark.tsx:3`, `design/README.md:1`, `_foundation.scss:1`, `package.json:25`

This note records where the **current shipped Paper system** inherits from, diverges from, or deliberately ignores older explorations. It should be kept current whenever a colour or name changes — otherwise the repo will hold two contradictory palettes under one logo.

---

## 1. What is authoritative today

- **Name — `Vela`** (`package.json:5`) — `vela`, `OneAGI/vela`. The welcome header, titlebar, and docs all say Vela.
- **Default theme — `paper`** (`_foundation.scss:72` + `src/renderer/src/theme.tsx:10` → persisted as `vela.theme`, default fallback `"paper"`). The OS dark variant is the *alternate* (`"original"`).
- **Accent — sage** `original #9eb4a1 · paper #617a68` (`_foundation.scss:41 + :109`). All hover/active washes, pill fills, focus rings, activity pulses, and the `VelaMark` star are sage.
- **Mark — `VelaMark.tsx:3`** — a four-point star and smaller sparkle using the active appearance tokens.

Because these are the *experienced* truth, **this folder treats sage as the only accent**. Any document that references `#c25f3c / #e8875f` is historic (see §3).

---

## 2. What's in `resources/`

| File | Content | Line | Status |
|---|---|---|---|
| `icon.svg:1` | Dark blue 1024 squircle with a luminous four-point star and smaller sparkle | `2–35` | **Aligned** — the OS icon now uses the Vela star mark on its own dark blue palette. |
| `icon.icns`, `icon.png` | Derived from `icon.svg` (`npm run generate-icons`) | — | Aligned — generated from the current star icon |

> The Vela star mark and OS icon are separate compositions. Update the in-app
> and lockup marks together; regenerate `icon.icns` and `icon.png` whenever
> `resources/icon.svg` changes.

---

## 3. What's in `redesign/`

| Path | Content | Palette | Relation to ship |
|---|---|---|---|
| `rebrand.md:1` | `100 name directions` + `top 10 shortlist (Patchbay, Harness, Roundhouse, Helm …)` + explicit thesis *"The GUI is the constant. The agent is the plugin."* | none (names) | **Historic but live intel.** The naming thesis is the same one Vela executes ("runtime adapters"). Names like `Harness`/`Roundhouse`/`Helm` are still being considered for a public re-rename — the logo families in `design/logos` can be read against them. |
| `logo.html:1` | `100 tile marks` (dark bg `#171310` + burnt-clay `--accent: #e8875f` + `radial accent 7%`). | Burnt-clay `#e8875f / #c25f3c` | **Closed exploration.** Dark-ground, ember-brightwards, deliberately warm-industrial. The shipped Paper system intentionally moved **away** from this — cool-sage replacing burnt-clay — because the paper surface lets sage read warm on its own. Carry the tile explorations as mood, not as spec. |
| `app-redesign-*.html` ×5, `landing-design-*.html` ×5, `landing.html`, `animation2.html` under `redesign/output/` | Early layout/marketing mock boards (contain ragged experimental palettes including clay oranges). | mixed | Non-authoritative. Keep for reference, do not sync tokens to them. |

*If a new logo family or palette emerges from the logos gallery, add a row here and note whether it became canonical or remained a kitchen exploration.*

---

## 4. What's in `design/` (current kitchen)

| Path | Content | Notes |
|---|---|---|
| `index.html:1` + `session-panels.css` / `.js` | `100 session-panel concepts` constrained to the current file-panel grammar — `system UI, cream/beige/sage, 400–525, 28–40px rows, transparent lists` | **Current, owned.** Its constraint line (`index.html:46` / `README.md:5`) is exactly the list-row contract of `04-navigation-and-lists.md`. Shortlist IDs `001–100` are the only sanctioned refinement queue for session-panel work. |
| `README.md:1` | 10 IA families × 10 density treatments | See above. |
| `logos/` (empty at time of this audit) | Reserved for `300 calm logo directions` sub-agent | Each logo must be individually searchable, on the sage/paper palette — not ember. Use `01-tokens.md` §1.4. |

---

## 5. Open inconsistencies & how to close them

| # | Symptom | Risk | Path to close |
|---|---|---|---|
| 1 | ~~Old clay icon vs sage interface~~ **Closed** — replaced with Vela's star icon on a dark blue ground | ~~Dock icon did not match the interface~~ Resolved | Generate `.icns` and `.png` from `resources/icon.svg` with `npm run generate-icons` |
| 2 | `design/session-panels.css:1` hard-codes `--accent #617a68` which **is** correct — good. No action. | — | — |
| 3 | `src/renderer/src/theme.tsx:2` `ThemeId = "original" | "paper"` — naming implies "original" is the base. | Minor. Renaming to `"dark"` / `"light"` would retire historic debt (`git mv` + migrate localStorage key). Coordinate with docs. |
| 4 | No design-system presence check in `scripts/check-docs.mjs` (it inventories IPC / window.opencode contract etc. per `AGENTS.md:Docs maintenance`) | Design drift is currently unchecked in CI | Add an optional `docs:check:design-system` that diffs `_foundation.scss` token list → `01-tokens.md` table and warns if a token was added without a doc row. Keep it advisory, not blocking. |

---

## 6. Intended vs accidental drift — lore snippet

The **Vela → Vela** move was a *runtime adapter* story — `rebrand.md:2` opens *"Today Vela is welded to one power source: opencode2."* — but the rebrand did not yet commit to a new name or a new icon; the shortlist spans socket stories (`Junction`, `Manifold`, `Conduit`), harness stories (`Harness`, `Byssus`), and home stories (`Hearth`, `Roundhouse`).

The design-language move happened in parallel and was intentional: when the Paper theme became the default (`ThemeProvider: default "paper"` in `theme.tsx:16`), the palette migrated from *"ember on ink"* (dark `#171310` + `#e8875f`) to *"sage on parchment"* (warm `#fbf7ec` + `#617a68`) so the default surface felt like paper rather than cardboard. The dark `original` theme preserved a cooled version of the same sage (`#9eb4a1`) rather than keeping the ember (`#c25f3c`) — proof that the ember→sage was a palette decision, not just a light-mode tweak.

The rebrand replaced the old icon with a Vela star and sparkle on a dark blue
ground; the in-app mark remains appearance-aware (see #1).

---

## 7. Checklist when shipping a new token / component / theme

1. Add token to **both** blocks (`:root:1` and `:root[data-theme="paper"]:72`) in `_foundation.scss`.
2. Add to `01-tokens.md` matrix with both hex values, section references, and usage.
3. Add/extend a slide in `02–09` as applicable (new component needs an anatomy tree; new token needs a "how to add" note).
4. If it is a chat/ IO token re-alias it in `_opencode-chat.scss:1` `--v2-*`.
5. Add the variant matrix row in `design/session-panels.css` if the component participates in the 100-directions gallery (optional but encouraged).
6. When removing a token, delete its rows in `01-tokens.md` and note the removal here with date + commit.
