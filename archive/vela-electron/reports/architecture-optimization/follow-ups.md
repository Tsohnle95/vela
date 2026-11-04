# Separately observed opportunities

These are outside this semantics-preserving campaign's implementation scope.
They are not acceptance claims or silently bundled behavior changes.

- **GUI/TUI mode changes lose a reader's mid-history position.** Native QA
  scrolled a 25-turn fixture into the middle, switched GUI → TUI → GUI, and
  returned at the beginning. Independent paired tests loaded the historical
  `AgentPanel` from `0cee93e` and current source: both replaced the scroll DOM,
  changed scrollTop from 600 to 0, and retained all 25 turns. The unchanged
  scroll restoration effect depends on workspace memory identity, while the
  parent component stays mounted during mode changes. A future behavior fix
  should characterize mode transition restoration separately from whole-panel
  remount, bottom-follow, reader intent and incoming streamed content.
- **Broad renderer context remains a propagation cost.** Per-session projection
  reuse reduces derived work but `usePanel` still consumes the broad Store
  context. A selector migration requires render-count characterization across
  unrelated sessions and explicit ownership of optimistic/busy/permission state.
- **Panel projection's inbox dependency needs a separate correctness review.**
  Discovery observed an omitted inbox dependency in the existing memo. It was
  not corrected during extraction.
- **Renderer teardown timer cleanup needs characterization.** Existing tree,
  toast and compaction timers are not uniformly cancelled; stale workspace
  checks mitigate writes but do not establish complete teardown cleanup.
- **Recovery test teardown can race a background purge.** A Requests checkpoint
  gate passed all assertions but reported `ENOENT` after `multi-context` removed
  its temporary workspace. Existing activation fires recovery purge without
  awaiting it; purge later resolves the recovery root outside its current-context
  guard. Backend and test code were unchanged by Requests. A future repair should
  characterize resource teardown and cancellation before changing production
  failure handling. The isolated 65-test and complete 1,287-test retries passed;
  that does not establish the race is absent.
- **Layout test teardown can outlive a toast timer.** The same gate reported
  `window is not defined` when an existing four-second toast timer fired after
  jsdom teardown. `App.layout` lacks a saved-workspaces bridge mock, producing
  that toast; its unchanged teardown does not clear the timer. Keep fixture
  completeness and real application timer ownership as separate investigations.
- **Sharing filesystem observation needs workload evidence.** Separate contexts
  intentionally own baselines and watcher lifetime; reducing repeated same-root
  reads requires reference-count/lifecycle measurement before changing ownership.
- **Provider concurrency is not behavior-neutral.** Existing malformed OpenAI
  credentials reject before subsequent provider HTTP calls begin. Parallelization
  would change failure side effects. Keep sequencing unless a separate behavior
  change deliberately defines normalization and continuation policy.

- **Release-update Return action needs a separate behavior fix.** Native Electron
  37.10.3 QA displayed the expected failed-update popup, but its Return button did
  not close it. Independent old/current probes found that the encoded `data:`
  page ignores `location.hash` assignment and emits no in-page navigation event.
  Both historical `545a170` and extracted source use this same action. The handler
  also reads Electron's fourth argument (`frameProcessId`) as the main-frame flag,
  although the actual flag is third; old and current handlers behave identically.
  A future fix should characterize an actual data-page button activation and
  main/subframe event gating. The architectural extraction preserves this existing
  behavior; unit fixtures now use Electron's actual five-argument event shape.

- **Session deletion starts before dirty-close persistence settles.** Historical
  and current public lifecycle fixtures produce the same `delete → save → close`
  bridge trace. The existing delete action starts close, then deletes the runtime
  session while the save queue is pending. A separate behavior fix should define
  save-failure and cancellation policy before serializing those operations; this
  extraction deliberately preserves the existing interaction.

Design experiments, mockups, dormant adapters, public/test-only exports and
prototype plugins remain intentionally preserved without an established
deletion policy. Static unused-consumer findings alone do not authorize removal.
