# Task memory

Temporary working memory for multi-phase agent work. This directory is not
durable architecture documentation and not a discussion archive.

## When to use it

- **PATCH**: no task file.
- **FEATURE**: normally keep the plan in context. Create a task file only when
  the work spans sessions, agents, commits, or a risky handoff.
- **PROJECT**: create a task file before implementation.
- **BATCH**: create one only when recovery or coordination justifies it.

A task file is warranted when losing the current context would force another
agent to repeat expensive investigation or risk an unsafe partial state.

## Lifecycle

`DRAFT` → `REVIEWED` → `ACTIVE` → `COMPLETE` → delete

1. Draft the goal, acceptance criteria, preserved behavior, and affected
   surfaces.
2. Review the plan for unsupported assumptions, missing contracts/tests,
   sequencing, scope, and unverifiable goals.
3. Record resolved decisions and the current checkpoint, then move to `ACTIVE`.
4. Update phases and validation evidence as work proceeds.
5. Run cumulative validation, move genuine remaining work to the backlog, and
   delete the task file. Git owns completed history.

## Rules

- Start from [`_TEMPLATE.md`](_TEMPLATE.md).
- Name files by outcome, for example `multi-panel-session-routing.md`.
- Link canonical docs and source instead of copying architecture prose.
- Record verified facts separately from assumptions.
- Never store secrets, credentials, personal transcripts, or raw provider data.
- Do not paste conversation transcripts. Keep only decisions, evidence paths,
  commands, and checkpoint IDs.
- A fresh agent must be able to recover active work from root/scoped
  instructions, the task file, canonical docs, source/tests, and Git.
- Do not leave completed task files behind.
