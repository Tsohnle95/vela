# Agent execution protocol

This is the single owner of how agents plan, implement, validate, document, and
recover work in this repository. Root [`AGENTS.md`](../AGENTS.md) owns routing
and safety; domain documents own technical truth.

**Governing principle:** use the minimum process that keeps expected repair cost
acceptably low. Planning is valuable when it reduces uncertainty, coordination
cost, or likely rework — not as ceremony.

## Core loop

```text
Understand → route context → classify → plan as needed → implement
→ targeted validation → review → canonical validation
→ update durable truth → checkpoint
```

1. **Understand** the requested behavior and the current system before editing.
   Inspect Git status and the relevant source/tests, not the whole repository.
2. **Route** with root/scoped instructions, then load only the task-relevant
   domain owner. Expand to more files only when evidence requires it.
3. **Classify** the work; choose PATCH, FEATURE, PROJECT, or BATCH.
4. **Plan** only as much as the uncertainty and blast radius justify.
5. **Implement** the smallest coherent change. Prefer self-explanatory code;
   comments preserve non-obvious invariants and rationale.
6. **Validate** the behavior and affected contracts, then review the diff.
7. **Run the canonical gate** when the change is complete or the risk warrants
   it. Report anything that could not run.
8. **Update durable truth** only where it changed.
9. **Checkpoint** with a logical commit only when the user has authorized
   commits; otherwise leave a clean, reviewable working tree.

## Context hierarchy

```text
AGENTS.md
  → applicable scoped instructions (archive/AGENTS.md under archive/)
  → task-relevant domain owner (docs/architecture/*.md)
  → specific symbols, contracts, and tests
  → expand only when evidence requires it
```

Do not repeatedly reopen established facts, preload every document, ingest the
archive or parity evidence wholesale, or crawl generated/dependency/build
content. Under `archive/`, follow `archive/AGENTS.md`; the nested
`archive/vela-electron/AGENTS.md` is historical reference only.

## Modes

### PATCH

Use for localized, low-coupling, well-understood work with clear acceptance and
straightforward verification.

1. Inspect the relevant source/docs/tests.
2. Make the smallest coherent change.
3. Run focused validation.
4. Review the diff for unintended edits.
5. Run the canonical gate when practical.
6. Update durable docs only if truth changed.
7. Commit only when authorized.

No persistent planning file.

### FEATURE

Use for coordinated work across multiple files/components while the architecture
remains understood.

Keep an in-context plan covering acceptance, affected components/contracts,
sequence, validation, and significant uncertainty. Implement in small testable
slices and reassess after meaningful progress. Use persistent task memory only
when sessions, agents, commits, handoffs, or recovery genuinely justify it.

### PROJECT

Use for architecture, shared contracts, migrations, persistence, security/trust,
concurrency, recovery, major integrations, substantial uncertainty, or tightly
coupled phases.

Before coding:

1. Create a temporary `.agent/tasks/<outcome>.md` from the template.
2. Investigate current behavior and record verified facts.
3. Define acceptance, preserved behavior, invariants, affected surfaces, phases,
   and validation.
4. Adversarially review the plan for unsupported assumptions, missing
   contracts/tests, compatibility, security, state, race/recovery risks,
   duplication, sequencing, excessive scope, and unverifiable goals.
5. Revise the file with resolved decisions and concise rationale; move it to
   `REVIEWED`/`ACTIVE`.

Implement one phase at a time. Validate and review each phase; commit
independently sound checkpoints only when authorized. Maintain useful
continuation state and checkpoint IDs. At completion, run cumulative/integration
validation, review the full diff and commit series, update affected canonical
docs, move genuine remaining work to the backlog, and delete the task file.

### BATCH

Use when one request contains multiple independently completable outcomes.

Triage the whole batch once for shared causes, dependencies, conflicts, and
overlapping ownership. Cluster or order the work, then classify each unit
independently as PATCH, FEATURE, or PROJECT. Execute logical units separately.
Use durable batch memory only when recovery or handoff warrants it.

BATCH is orchestration, not a complexity level.

## Classification

Classify using:

- ambiguity of the requested behavior;
- blast radius and number of contracts/consumers;
- coupling across crates or state owners;
- architectural novelty;
- cost of a wrong assumption/failure;
- testability and observability.

Begin with a lighter mode only when an incorrect assumption can be detected and
corrected safely. Escalate when the work reveals hidden contracts or state.

## Replanning triggers

Replan when:

- an assumption or contract proves false;
- scope expands materially;
- an unexpected dependency or consumer appears;
- two repairs fail for the same cause;
- unrelated refactors become prerequisites;
- a new durable contract, migration, or security boundary emerges;
- the correctness invariant becomes unclear;
- context is dominated by obsolete investigation.

Preserve verified work, summarize new evidence, adjust the mode/plan, and
continue from a known-good state. Do not restart investigation that is still
valid.

## Delegation

Parallelize independent discovery, investigation, and review aggressively;
parallelize implementation conservatively.

- Do not concurrently modify coupled contracts or shared state without stable
  agreed boundaries.
- Give each subagent a bounded question, relevant paths, and the expected
  evidence format.
- Treat subagent output as evidence, not automatically accepted truth.
- The coordinating agent owns integrated architecture, sequencing, synthesis,
  and final validation.
- Delegation is optional; use it when it reduces elapsed time or uncertainty
  without adding coordination cost.

## Temporary task memory

Lifecycle: `DRAFT → REVIEWED → ACTIVE → COMPLETE → DELETE`.

A useful task file contains:

- goal and verifiable acceptance, including behavior that must remain unchanged;
- links to canonical docs, source, and tests;
- invariants, constraints, and affected surfaces;
- phases, status, validation, and checkpoint IDs;
- resolved decisions with concise reasons;
- current risks/blockers and completion criteria.

Do not copy canonical architecture prose or preserve discussion transcripts.
See [`.agent/tasks/README.md`](../.agent/tasks/README.md) and
[`_TEMPLATE.md`](../.agent/tasks/_TEMPLATE.md).

PATCH never requires a task file. FEATURE normally stays in context. PROJECT
uses one. BATCH uses one only when warranted. A completed task file is deleted;
Git owns the history.

## Validation

Validate according to risk and acceptance:

| Change | Required evidence |
|---|---|
| Local logic/state | focused unit tests; affected integration path |
| Shared contract/API/IPC/event | producer and consumer tests; type/parse checks; affected flow |
| Persistence/security/concurrency/recovery | invariants, failure paths, restart/recovery checks |
| UI behavior/appearance | rendered/behavioral tests plus direct interaction/appearance validation where required |
| Platform/external integration | automated checks plus actual platform/integration exercise |
| Documentation/process | link/path check and cold-start routing audit |

The canonical gate is:

```sh
bash scripts/verify.sh
bash scripts/verify.sh --release   # CI/pre-package
```

It runs documentation links, Clippy with warnings denied, the workspace tests,
and an app build. `--check-format` adds rustfmt verification to the full gate;
it is opt-in because the imported tree has pre-existing rustfmt drift. Do not
reformat unrelated files inside a behavior change. The gate is deterministic and
non-interactive, but it proves only what it exercises.

The canonical gate does not establish:

- live visual parity;
- VoiceOver/platform focus behavior;
- real provider OAuth/key refresh/usage;
- WKWebView permission/isolation/download lifecycle;
- real IME/candidate behavior;
- full TUI/PTY appearance;
- packaging, signing, notarization, install, or update;
- mobile/shared-session behavior.

Exercise those directly when access exists. Otherwise report them as
unverified; never convert “tests pass” into “feature accepted”.

### Target-specific gates

- Core/domain: `cargo test -p vela-core`.
- Adapter: `cargo test -p vela-opencode`.
- Engine/integration: `cargo test -p vela-engine --test engine_flow`.
- Terminal: `cargo test -p vela-terminal --test pty_flow`.
- UI: `cargo test -p vela-ui`.
- Performance: `cargo test -p vela-core --release --test perf -- --ignored --nocapture`.
- Docs: `python3 scripts/check-docs.py`.

Known intermittent tests must be rerun isolated before the full gate. Never
replace a failing evidence file with a passing filename; record the failure and
the correction separately.

## Git and checkpoints

- Inspect `git status` before editing; preserve pre-existing changes.
- Never reset, clean, stash, overwrite, or discard unrelated work.
- Commit only when the user has explicitly authorized it.
- Use logical checkpoints: one independently sound change set, with related
  canonical docs where practical.
- Never commit secrets, credentials, personal data, generated debris, or
  unrelated user edits.
- Before a checkpoint, review the full diff, not just the files you remember
  editing.
- A commit is not acceptance; it is a recoverable state.

## Documentation evolution

Normal development updates only the canonical truth that changed:

| Change | Update |
|---|---|
| Architecture/ownership/boundary | `docs/architecture.md` or the relevant domain owner |
| Contract/API/event/schema | canonical contract owner + affected docs |
| Setup/toolchain/verification | `docs/development.md`, `scripts/verify.sh`, CI |
| Routing/safety | root or scoped `AGENTS.md` |
| Feature status/evidence | parity matrix/checkpoint |
| Pure refactor with unchanged durable truth | no documentation churn |

Do not maintain duplicate inventories. Prefer machine-readable source/config
and links over copied lists. Mechanical checks prove surface alignment; tests
prove behavior; architectural review proves semantic correctness.

When architecture evolves, reconsider document boundaries, scoped instructions,
routing, walkthroughs, and drift checks. Split/merge/retire documents only when
the software change justifies it.

## Recovery and handoff

A fresh agent should recover active work from:

1. root/scoped instructions;
2. the active task file (if any);
3. relevant canonical docs;
4. current source/tests;
5. meaningful Git checkpoints and the last validation evidence.

On interruption, leave the working tree in a coherent state, update the task
file with what is verified versus assumed, and record the next concrete step.
Do not preserve a conversation transcript or rely on hidden context.

## Reporting

When finishing, state:

- what changed and why;
- what was validated, with commands/results;
- what remains unverified and why;
- affected docs/contracts;
- any follow-up or residual risk.

Keep the report concise. Do not output private reasoning or investigation
transcripts.
