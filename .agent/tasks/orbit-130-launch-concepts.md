# Orbit — 130 distinct launch concepts

Status: ACTIVE

## Goal

Extend the completed 30 local landing experiments with 100 additional unique,
agent-created compositions inspired by the experimental-art screenshots, in one
reviewable gallery with the same three-second launch behavior.

## Acceptance criteria

- [x] Original 30 preserved and checkpointed (`a0dd65b`).
- [ ] Exactly 100 additional independently composed concepts, numbered 31–130.
- [ ] Shared reservation registry and saved agent prompt prevent overlapping briefs.
- [ ] Every new concept records its source inspiration and visual differentiation.
- [ ] Combined gallery supports browsing originals and additions, individual preview,
      hold/replay, and three-second transition to an empty workspace.
- [ ] All artwork decodes; links, count, timing, and navigation verified in browser.
- [ ] Visual review across all 130; near duplicates revised before completion.
- [ ] Supported-toolchain canonical gate passed; owned changes checkpointed only.

## Relevant context

- `AGENTS.md`, `docs/agent-execution.md`, `docs/renderer.md` startup section.
- `design/experimental-art/experimental-landing-page/`: entire affected surface.
- Source screenshots stay outside the deliverable; app production code is untouched.

## Invariants / constraints

- Preserve unrelated staged and unstaged changes; concurrent app work is active.
- Root owns shared gallery/runtime/build integration and final commits.
- Three designers own disjoint expansion modules only (31–64, 65–97, 98–130).
- No recolors, new seeds, crop swaps, or relabeled templates count as new designs.
- Original 30's SVG artwork hashes must remain unchanged.
- Each concept needs a distinct dominant silhouette, layout and typography hierarchy.

## Phases

| Phase | Scope | Status | Validation | Commit |
|---|---|---|---|---|
| 1 | Original 30 concepts | complete | Browser 30 decodes; 10 checks; npm check | a0dd65b |
| 2 | Reference audit, unique reservations, agent prompts | complete | 29-image contact sheet; all 100 reservations reviewed | pending checkpoint |
| 3 | Three disjoint expansion modules and shared gallery integration | active | 100 assets; metadata; static validation | — |
| 4 | Cross-set review, repairs, final browser verification | pending | 130 decodes; visual review; canonical gate | — |

## Validation plan

- Validate manifest counts, reserved IDs, source paths, valid SVG/XML, local links,
  no identical artwork hashes, and original-art hash preservation.
- Review thumbnails/contact sheets and a representative full-size preview per family.
- Browser verification measures actual three-second timer after image decode/paint.
- Run `npm run check` with Node 22.23.2 before final commit.

## Decisions

- 2026-10-17 — User explicitly requested orchestration of agents for another 100.
  Reserve non-overlapping composition briefs, and assign different visual domains.
- 2026-10-17 — No production startup integration until a concept is chosen.
  Shared preview represents the requested launch behavior without selecting sessions.

## Open risks / blockers

- Subjective uniqueness requires visual comparison; file hashes alone prove no
  exact duplicates, not strong design difference.
- Design directory is ignored globally in this repo; stage only intended deliverables
  explicitly with `git add -f`, and commit with a scoped pathspec.

## Completion

- [ ] All acceptance criteria satisfied.
- [ ] Final browser and canonical validation passed.
- [ ] Final Git state reviewed; temporary ledger deleted in completion checkpoint.
