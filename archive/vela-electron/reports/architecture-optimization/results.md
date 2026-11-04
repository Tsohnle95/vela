# Architecture campaign results

Production source checkpoint: `00de1b1`. Baseline: `a1c54a35c3856e56209110cf4db493754699ed00`.
Independent cumulative review approved the architectural outcome, with no
unresolved critical new regression identified. The reduction objective remains
unmet and the integration gaps below remain explicit.

The campaign established actual subsystem ownership, removed confirmed
redundancy, improved transcript projection, and installed permanent CI checks.
It **did not make the production codebase smaller overall**. Production LOC
increased by 747 (1.94%); the 30% target was not achieved. No functionality was
intentionally removed to obtain a lower count.

## Code measurements

| Metric | Before | After | Change |
|---|---:|---:|---:|
| Handwritten production LOC | 38,430 | 39,177 | +747 / +1.94% |
| Production source files | 152 | 193 | +41 |
| Modules above 800 code lines | 11 | 3 | −8 |
| Functions above 100 inclusive code lines | 37 | 47 | +10 |
| Complexity/nesting hotspots | 54 | 52 | −2 |
| Substantial exact duplicate functions/blocks | 0 / 0 | 0 / 0 | unchanged |
| Runtime import cycles | 0 | 0 | unchanged |
| Cycles including type-only imports | 1 | 0 | −1 |
| Export candidates without production consumers | 129 | 117 | −12 candidates |
| Build files | 720 | 720 | unchanged |
| Build logical bytes | 50,276,995 | 50,312,752 | +35,757 / +0.071% |

The frozen target is **26,901 LOC**. Final production is 12,276 lines above that
target. The generic analyzer's `production.targetLoc` is computed from each
measured reference itself; its final value, 27,423, is not the campaign target.
`final.json.campaignComparison` records the frozen baseline target explicitly.

Both inventories use the same nonblank, noncomment physical-line method and
production roots: main, preload, renderer source and shared. Tests, fixtures,
generated output and the same 16 content-pinned upstream styles are excluded.
Authored declarations, adapted OpenCode renderer code, dormant DeepSeek and the
static Linguist table remain included. No production implementation moved into
an excluded category. See [baseline.json](baseline.json) and [final.json](final.json)
for complete path, LOC, hash, dependency, function and build inventories.

Gross per-file decreases total **9,035 lines**, including relocated code. New
files and increases total **9,782 lines**, producing the net increase of 747.
These are inventory deltas, not a claim that 9,035 lines were eliminated.
Confirmed eliminations include shared normalization/credential parsing, repeated
failure DTOs, obsolete exports/styles, duplicate composer declarations and
session launch sequences. The last consolidation independently removed 23 net
production lines. Two unused direct dependencies (`react-markdown`, `remark-gfm`)
and their dependency chain removed 66 installed packages without changing retained
package versions. Build/verification code remains an auxiliary category.

Ownership interfaces, readonly contracts and integration commands offset those
savings. More inclusive oversized function entries reflect extracted hook
compositions; this campaign does not claim fewer large functions globally.
Each nested operation remains independently constrained, with precise reviewed
exceptions for cohesive inherited flows.

## Architecture and ownership

| Investigated module | Before LOC | After LOC | Result |
|---|---:|---:|---|
| Renderer Store | 3,841 | 1,172 | Private lifecycle, chat, execution, files/editor, explorer, inbox, request, progress, inventory and selection owners; facade and attributed IO/event coordination remain |
| OpenCode backend | 3,176 | 1,910 | Replay, recovery, observation and mutation owners; privileged SDK/context/session authority remains centralized |
| Main entry point | 1,372 | 759 | Trusted domain IPC registration, DevTools and update-window ownership |
| AgentPanel | 1,681 | 361 | Composer, controls/menu, usage and scrolling ownership |
| OpenCodeTimeline | 1,626 | 326 | Domain transformations and cohesive transcript presenters |
| App | 1,151 | 799 | Layout geometry and interaction lifecycles separated from application composition |
| Chat store | 1,090 | 595 | Record mechanics, projection and reconciliation boundaries |
| Provider usage | 835 | 680 | Credential discovery, Copilot policy/failure normalization and single JWT decoding |

These reductions within the original modules mostly represent extraction, not
net code deletion. Existing public IPC channels, preload implementation, shared
public types, runtime selection, `AGENTS.md` and `docs/agent-execution.md` are
unchanged from baseline. OpenCode V2 remains active and DeepSeek remains dormant
source. Canonical ownership and navigation are maintained in the existing docs;
this report is measurement evidence, not a replacement project brain.

Three oversized modules remain deliberately constrained:

- Store's 1,172 lines compose public context and cross-domain materialization,
  hydration, prompt, reconnect and attributed event ordering. Actual state
  authorities are private feature owners. Moving this remaining sequence into
  a large EventsHub would conceal its coupling.
- `opencode.ts` retains 1,910 lines under the protected single-SDK-owner policy.
  Service/session/context generations and SDK routing stay together; filesystem
  observation, durable recovery and transaction workflows have separate owners.
- The 1,908-line inline-code language table is cohesive static data. Compressing
  literals or moving them outside the measurement scope would manufacture savings.

The lifecycle owner constructs state and workflows at two existing composition
points in one module. Its private authority is opaque, even through reflected
TypeScript symbol keys. Consumers borrow readonly queries; legacy public UI
identities remain compatible. The Editor focus bridge preserves its original
callback cohort. No new effects, cleanup policy, service cache or public IPC
schema was introduced. Historical and current tests preserve captured saves,
late responses, event guards, stale acquisitions, concurrent reopen counts and
distinct busy-session closing behavior.

## Performance

Benchmark reports retain raw samples, workloads, variability and source-reference
metadata. Later comparisons additionally retain explicit source/runner hashes.
The command paths are recorded in each report. Use Node 22.23.2 and compare
identical workloads sequentially; these are operation measurements, not a claim
of faster application startup or Chromium painting.

| Workload | Before median | After median | Interpretation |
|---|---:|---:|---|
| Chat projection: 400 assistants, 32 KiB tool output, 500 tail deltas | 234.893 ms; repeat 241.050 | 7.030 ms; repeat 7.136 | Confirmed projection gain; unchanged retained content and 200,000 identity reuses |
| Timeline, ordinary 100 transformations | 300.272 ms; repeat 292.957 | 280.102 ms; repeat 283.401 | Inconclusive |
| Timeline, missing inputs | 71.909 ms; repeat 68.842 | 57.963 ms; repeat 59.100 | Lower measured descriptor cost on this fixture; limited to transformation work |
| Timeline, explicit inputs | 77.337 ms; repeat 74.379 | 63.260 ms; repeat 64.813 | Lower measured descriptor cost on this fixture; excludes React/paint |
| Workspace observation, unchanged events | 14.70 / 15.05 ms | 14.93 / 15.10 ms | Inconclusive |
| Durable mutation workload | 465.26 / 469.38 ms | 463.16 / 461.56 ms | Inconclusive; same file/recovery outcomes |

Renderer Store phase reports cover streaming, workspace/editor operations,
inbox, forms/permissions/preferences, progress and lifecycle acquisition. Their
timings are inconclusive. Final lifecycle comparisons retain all 72 samples:
201 consumer renders, 100 provider reads and identical per-event/final state in
every sample. Initial usage increases of 0.479/1.171 ms per 100 updates caused
additional comparisons; later −0.188/+0.048 ms did not reproduce the increase.
Todo medians were 0.140–0.269 ms higher per 100 updates; provider medians ranged
from 0.297 ms lower to 0.390 ms higher. Paired ranges overlap; no confirmed
meaningful regression or speedup is claimed.

Broad context consumers still receive unrelated updates. The campaign rejected
insufficiently justified selector/cache machinery, copy-on-write prototypes
that regressed, shared-watcher redesign and provider parallelization that would
change failure side effects. File benchmarks model trash with disposable unlink;
they do not establish Finder Trash behavior. Native/provider latency, OS startup,
mobile hardware, Chromium layout and paint remain outside these timing workloads.

## Verification and permanent safeguards

- Final lifecycle checkpoints: two supported `npm run check` passes, 1,326 tests
  in 156 passing files, four existing skipped tests in two files, node/web
  type checks, documentation checks, architecture checks and production build.
  The final root acceptance gate also passed under Node 22.23.2 after task cleanup.
  Sandboxed unit watcher tests emitted `EMFILE` warnings; the separate native
  platform gate passed outside that sandbox. Baseline was 989 tests in 123 files.
  No meaningful test was removed or weakened.
- Every implementation checkpoint has a targeted test/diff review and recovery
  commit. Lifecycle additionally passed 249 routed tests, 19 historical public
  cases, 19 permanent compiler boundary assertions and 24 callback/11 ordered
  effect equivalence checks. Its reviewer did not author the implementation.
- Full macOS `npm run test:platform` passed: launch/terminal tests, direct and
  Electron PTY, trusted IPC/navigation/popup/untrusted-sender checks, native
  watcher close/reopen and rename/move correlation. A shell-sandbox trust launch
  aborted; isolated trust and the full platform retry passed outside that shell
  sandbox without a source change.
- Bundled Sass worker generated disk CSS. Isolated installed OpenCode V2 service
  discovery, create/read/delete and `server.connected` SSE passed with disposable
  state, no credentials and no model prompt.
- Native actual compiled renderer/preload checks exercised layout, models/agents,
  streaming/replay, stop/late-event suppression, local/native queues, forms and
  approval, usage/compaction, file edit/save/diff, creation/rename and session
  close/reopen. The latest lifecycle dirty close used the actual mutation backend,
  verified UUID/generation, revision and original-content expectation, disk text
  and cleared unsaved state, then restored saved editor and transcript from History.
  Runtime/catalog responses in this disposable app were mocked.

Architecture checks run through `npm run check` and its existing GitHub Actions
workflow, without a new tooling dependency. They fail on budget growth, excessive
new function metrics, forbidden layer/authority imports, private-owner leaks,
new value/type cycles, substantial new exact duplication, malformed manifests,
recognized computed dependency-loader forms, missing canonical ownership routes and changed
excluded style pins. New files follow defaults; inherited exceptions have fixed
origins/rationales and downward ratchets. Three query-consumer allowances now
permit the public readonly renderer contract through type-only imports; this is
a deliberate boundary change, not a claim that every dependency rule was unchanged.

The exact-clone detector does not prove semantic equivalence or detect every
renamed/short clone. Static checks cannot defeat intentional opaque evaluation
or all assignment aliases. Aliased `require`, `require.call` and runtime-generated
loading are not dependency edges recognized by this syntax-based checker; no
new production use was found in review. These limits are documented; the checks make normal
architectural regression fail locally and in CI without a large framework.
Coverage percentage is not configured, so test counts are not a coverage claim.

Unverified surfaces: real paid-provider prompt/stop/steer, signed update
installation, native internal drag routing, RecoveryNotice interaction, the
real application quit/save dialog, Windows/Linux desktop behavior and a physical
mobile companion. The user's running installed app was not force-restarted.
Passing compilation or mocked UI checks does not establish those behaviors.

## Shortfall and remaining opportunities

The audit found bounded redundancy, not 11,529 safely removable baseline lines.
Further forced removal would consume distinct protocol/failure handling,
generation/concurrency protection, durable filesystem/recovery work, editor
conflict validation, UI capabilities, retained runtime source or cohesive data.
Extractions add explicit interfaces and ownership wiring; the final result is
larger despite genuine eliminations. The campaign therefore reports the shortfall
instead of claiming a defensible 30% reduction or hiding logic in dependencies,
JSON, generated files or excluded folders.

Further work needs separate characterization: broad context subscriptions,
same-root watcher sharing, timer/resource teardown, GUI/TUI scroll restoration,
update Return action, inbox projection dependencies and delete/save ordering.
See [follow-ups.md](follow-ups.md). Their pre-existing behavior was preserved;
this report does not claim those defects fixed. Design prototypes, dormant
adapters and legitimate test/public exports remain retained. Static candidate
lists alone are insufficient evidence to delete them.
