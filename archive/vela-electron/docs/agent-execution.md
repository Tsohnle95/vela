# Agent Execution Protocol

> **Document role:** canonical owner for how coding agents size, plan, execute,
> review, delegate, validate, checkpoint, and recover work in this repository.
> `AGENTS.md` contains only the lightweight routing rules that are always loaded.

The goal is not maximum planning. The goal is the **minimum process that keeps
expected repair cost acceptably low**. Spend planning tokens when they reduce
likely rework; avoid ceremony when a direct, verifiable change is safer and
cheaper.

## Core loop

```text
understand → route context → classify → plan only as needed → implement
→ targeted validate → review → canonical validate → document durable truth
→ commit
```

At every stage, keep the active problem small. Prefer recovering stable facts
from the project brain, tests, task file, and Git rather than carrying an entire
multi-hour conversation in working context.

## Execution modes

### PATCH

Use when the change is localized, acceptance criteria are clear, architecture
is unchanged, coupling is low, and correctness is straightforward to verify.

Process:

1. Inspect the relevant doc section, implementation, and nearby tests.
2. Make the smallest coherent change.
3. Run the narrowest meaningful validation first.
4. Inspect the diff for unintended scope.
5. Run `npm run check` before committing.
6. Update durable docs only if durable truth changed.
7. Commit the logical unit.

Do **not** create a planning file for PATCH work.

### FEATURE

Use when several files/components must coordinate, the current behavior needs
investigation, or implementation steps depend on each other but the overall
architecture is known.

Before editing, make a concise in-context plan containing only:

- intended outcome / acceptance criteria;
- relevant components/contracts;
- implementation sequence;
- validation strategy;
- known risk or uncertainty.

Implement in small, testable slices. Re-evaluate after each meaningful slice.
A persistent task file is optional and should be used only when the feature is
likely to span multiple sessions, models, agents, or commits.

### PROJECT

Use for architectural changes, shared contracts, migrations, persistence,
security/trust boundaries, concurrency/recovery behavior, major integrations,
or work whose correctness depends on several coupled phases.

Before implementation:

1. Create `.agent/tasks/<descriptive-task>.md` from `_TEMPLATE.md`.
2. Investigate current behavior from the minimum relevant docs, source, and
   tests. Record facts, not copied prose.
3. Draft acceptance criteria, invariants, affected surfaces, phases, and
   validation.
4. Review the plan adversarially before coding. Look for incorrect assumptions,
   hidden coupling, missing tests, unnecessary scope, compatibility risks,
   duplicated existing functionality, bad sequencing, and unverifiable goals.
5. Revise the file to contain the **resolved decisions**, not a transcript of
   reviewer/coder discussion.

During implementation:

1. Work one phase at a time.
2. Keep each phase independently understandable and preferably independently
   sound.
3. Use targeted validation while editing.
4. Review the phase diff and architectural assumptions.
5. Run the required gate for that checkpoint (`npm run check`, plus any
   task-specific platform/manual checks).
6. Commit the completed phase.
7. Update the task ledger with status and commit id before proceeding.

After the final phase:

1. Run integration/full validation.
2. Review the cumulative diff and commit series for accidental scope.
3. Update only canonical durable docs whose truth changed.
4. Move remaining future work to the normal backlog/issues.
5. Mark the task complete, then delete the temporary task file in the completion
   commit. Git retains the task history; the live project brain stays clean.

### BATCH

Use whenever one request contains multiple independently completable outcomes.
Batch is an orchestration mode, not a complexity level: each item may itself be
a PATCH, FEATURE, or PROJECT.

Before implementation:

1. Triage the whole request once.
2. Identify shared root causes, dependencies, conflicts, and items that touch
   the same subsystem.
3. Cluster or order the work.
4. Keep only the current logical unit active in implementation context.

For a small batch, an in-context checklist is enough. For a batch that spans
multiple contexts/commits, use a temporary task ledger with one row per logical
unit and its mode/status/commit.

Do not implement 10–15 unrelated changes as one giant unbroken turn merely
because they arrived in one prompt.

## Complexity signals

Classification should be fast and mostly internal. Do not emit scoring tables
unless they help the task. Consider:

- **Ambiguity:** is "done" precisely knowable?
- **Blast radius:** how many subsystems/contracts may change?
- **Coupling:** must those areas evolve together?
- **Architectural novelty:** is a new pattern/boundary introduced?
- **Failure cost:** is a mistake cosmetic, recoverable, data-affecting, or
  security/concurrency sensitive?
- **Testability:** can the result be verified cheaply and objectively?

When uncertain between two modes, start with the lighter mode only if the
escalation rules below can safely catch a mistake early.


## Implementation decisions and causal verification

### Before choosing an implementation

Use the investigation depth appropriate to the selected execution mode.

1. Establish the requested outcome and the relevant current behavior.
2. Separate verified facts from assumptions and unresolved questions.
3. Identify the existing owner of the behavior and the contracts that must
   remain valid.
4. Consider materially different implementation approaches when they exist.
   Prefer the approach that satisfies the outcome with the least unnecessary
   complexity, duplication, coupling, and regression risk.
5. Identify the evidence needed to verify the selected approach before
   making the change.

Do not manufacture alternative designs when one established implementation
path is clearly appropriate. Do not replace existing abstractions merely
because another approach is possible.

For consequential decisions, record the selected approach and its rationale
in the existing FEATURE plan or PROJECT task file. PATCH work does not need
a separate design record.

### Debugging and fault isolation

For a reported malfunction:

1. Establish the expected behavior and reproduce the failure when feasible.
   Capture the relevant error, failing test, log, or observable behavior.
2. Trace the actual failing execution path through the relevant components.
   Distinguish the observed failure from hypotheses about its cause.
3. When the cause is uncertain, perform the smallest useful diagnostic
   experiment that distinguishes between plausible explanations.
4. Implement a correction supported by the evidence. Preserve existing
   behavior outside the intended scope.
5. Add or update a regression test when a reliable automated test can
   reproduce the defect.
6. Repeat the original failure scenario after the fix. Run the relevant
   automated and integration checks before reporting completion.

When the original failure cannot be reproduced, explicitly identify the
remaining uncertainty. Do not claim that a suspected cause has been proven
or that an unexercised behavior is fixed.

Avoid speculative fixes and repeated modifications to the same area without
new diagnostic evidence. Use the existing replan/escalation rules when the
current approach is not converging.

### Verification strategy

Choose verification from the actual acceptance criteria and the affected
execution path, not merely from the availability of existing tests.

Prefer automated regression checks for repeatable behavior and direct
integration or platform checks for behavior that unit tests cannot exercise.

A successful compile, passing unrelated tests, or absence of logged errors
does not independently prove that the requested behavior works.

Report which acceptance criteria were directly verified, which were only
partially exercised, and which could not be verified in the available
environment.

## Replan / escalation triggers

Stop patching and revise the mental model or task plan when any of these occurs:

- an assumed architecture, API, or invariant is false;
- the implementation spreads materially beyond the expected subsystem;
- two repair attempts fail for the same underlying problem;
- tests reveal an unexpected cross-component dependency;
- unrelated refactors are becoming prerequisites;
- the implementation requires a new durable contract or migration not in scope;
- current edits can no longer be explained in terms of a clear invariant;
- accumulated context is dominated by dead approaches, stale logs, or obsolete
  assumptions.

Escalation does not require starting over. Preserve verified work, summarize the
new evidence, revise the plan, and continue from a known-good checkpoint.

## De-escalation / anti-ceremony rule

Do not create a specification, architecture proposal, persistent plan,
subagent hierarchy, or task decomposition when direct execution is sufficient.

A plan is valuable only if it reduces uncertainty, coordination cost, or repair
risk. Delete or compress obsolete planning detail as soon as the resolved
choice is clear.

## Context management

Treat context as a scarce working set, not a knowledge archive.

- Load `AGENTS.md`, then only task-relevant docs/source/tests.
- Prefer symbol/section reads over whole-file ingestion.
- Keep task files compact: current facts, decisions, phases, state, validation,
  and open risks.
- Do not copy permanent architecture prose into a task file; link to it.
- Convert investigation/review discussion into decisions instead of preserving
  the entire debate.
- After a phase commit, retain only the facts required for the next phase; Git
  owns the completed implementation details.

A fresh agent should normally be able to resume large work from:

```text
AGENTS.md
+ active .agent/tasks/<task>.md
+ relevant canonical module doc(s)
+ current source/tests
+ recent phase commits
```

## Delegation and subagents

**Parallelize discovery aggressively; parallelize implementation
conservatively.**

Good parallel work:

- map separate subsystems;
- locate tests and contracts;
- independently audit assumptions;
- investigate unrelated batch items;
- review a completed plan or diff.

Risky parallel work:

- multiple agents changing coupled interfaces simultaneously;
- one agent building consumers of an API another agent is still designing;
- several agents editing shared state/contract files without a stable boundary.

The coordinating agent owns the integrated architecture, phase ordering, and
final validation. Subagent output is evidence to synthesize, not automatically
accepted truth.

## Plan review protocol

For PROJECT work, review the plan before coding. A useful review explicitly
checks:

- Are acceptance criteria complete and verifiable?
- Are current architectural assumptions supported by source/tests?
- Is an existing abstraction being bypassed or duplicated?
- Are all affected contracts and tests represented?
- Are compatibility, persistence, security, race, or recovery concerns present
  where applicable?
- Is any phase too large to validate or revert independently?
- Is sequencing correct?
- Is scope larger than necessary?

After review, edit the plan itself. Prefer:

```text
Decision: RuntimeManager remains the only adapter-selection boundary.
Reason: renderer behavior already depends on normalized manifests; a second
routing path would duplicate ownership.
```

over a preserved comment thread between agents.

## Validation depth

Verification should scale with risk and with what the acceptance criteria
actually require.

- Cosmetic/local UI: targeted build/type/component check; when appearance or
  interaction is part of acceptance, verify that behavior directly.
- State/logic bug: targeted unit/component test plus affected integration path.
- Contract/event/IPC change: producer + consumer tests/fixtures and typecheck.
- Persistence/recovery/security/concurrency change: focused invariant tests,
  failure-path tests, and restart/recovery behavior where relevant.
- Platform/runtime/integration behavior: targeted automated checks plus the
  narrowest relevant platform, CI, integration, or manual smoke.
- Project completion: cumulative integration validation plus `npm run check`
  and any additional checks required by the acceptance criteria or
  `docs/operations.md`.

The canonical gate is necessary but not sufficient for behavior it does not
exercise. If a required acceptance surface cannot be exercised in the current
environment, report it explicitly as unverified rather than inferring it from
passing unrelated tests.

## Git as execution memory

Git checkpoints are part of the workflow, not cleanup at the end.

Prefer:

```text
plan/review
→ phase 1 → validate → commit
→ phase 2 → validate → commit
→ phase 3 → validate → commit
→ integration review → final docs/cleanup commit
```

over one huge commit after dozens of files have changed.

Never rewrite or clean away pre-existing user work to manufacture a clean
starting point. The task file records phase intent/status; Git records what was
actually completed.

## Temporary task files

Use `.agent/tasks/` only for active work that benefits from durable task memory.
The permanent files in that directory are `README.md` and `_TEMPLATE.md`.

An active task file should contain:

- goal;
- acceptance criteria;
- relevant canonical docs / implementation areas;
- invariants and constraints;
- affected surfaces;
- phase ledger with status and commit ids;
- validation plan/results;
- resolved decisions;
- open risks/blockers.

It should not contain copied source, giant logs, chain-of-thought, or a full
conversation history.

Lifecycle:

```text
DRAFT → REVIEWED → ACTIVE → COMPLETE → DELETE
```

Task files may be committed while active so another session/machine/agent can
recover them. Delete the completed task file after durable truths have moved to
their canonical docs and follow-up work has moved to the backlog. The deleted
file remains recoverable in Git history without polluting future contexts.
