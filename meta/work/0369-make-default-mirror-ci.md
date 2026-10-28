---
type: "work-item"
id: "0369"
title: "Make default mirror CI"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
relates_to: ["work-item:0361"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "ci"]
last_updated: "2026-10-28T16:18:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0369: Make default mirror CI

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The team decided that a bare `mise run` should mirror CI as far as possible.
Checking that decision turned up several CI checks the `default` task never
reaches, a formatting step that rewrites files where CI fails, and a
`CLAUDE.md` description that does not match what `default` runs. This item
closes those gaps, or records why a gap stays.

## Context

These CI jobs run tasks that `default` does not reach:

| CI job runs | `default` today |
|---|---|
| `lint:workflows:check` (actionlint) in `check-build-system`, via `build-system:check` | Not reached: `default` depends on `lint:check`, which omits it |
| `test:integration:pup` in `check-architecture` | Not reached: not in the `test:integration` roll-up |
| `test:integration:measure` in `check-measure-harness` | Not reached; it needs network egress |
| `test:e2e:visualiser:docker` in `test-visual-regression` | Not reached; it needs Docker |
| `format:*:check`, which fails on drift | `format:*:fix`, which rewrites files and passes |

The formatting row is a deliberate local convenience. It means `default` can
pass where CI fails, until the reformatted files are committed.

The research also recorded these related problems:
- **Docs mismatch:** `default` reaches `format:fix` but not `lint:fix`,
  although the root `CLAUDE.md` says it applies "all formatters and safe lint
  fixes".
- **Ordering hazard:** `format:fix` runs in parallel with lint and test tasks
  that read the same files.
- **Duplicate formatting:** `format:server:fix` and `format:cli:fix` both
  format the whole workspace, because `--all` through a member manifest
  covers every member.

Closing these gaps adds work to a local run. The research did not measure how
much.

## Requirements

1. For each CI job's task that `default` does not reach, either add it to
   `default`, or document in `tasks/README.md` why it stays out (for example,
   Docker or network egress).
2. Decide whether `default` should check formatting, as CI does, or keep
   fixing it, and document the choice.
3. Either wire `lint:fix` into `default` or correct the root `CLAUDE.md`, so
   the description matches what `default` runs.
4. Make every task that reads source wait for `format:fix` to finish.
5. Format the workspace once per `default` run.

## Acceptance Criteria

- [ ] Given each CI job in `.github/workflows/main.yml`, when its mise task
      is compared with `default`'s dependency closure, then the task is
      either reachable from `default` or listed in `tasks/README.md` with a
      reason.
- [ ] Given a `default` run, when it completes, then `lint:workflows:check`
      has run.
- [ ] Given the root `CLAUDE.md`, when it describes `default`, then the
      description matches the formatting and lint-fix steps `default`
      actually runs.
- [ ] Given a `default` run, when its task timeline is inspected, then no
      lint or test task started before `format:fix` finished.
- [ ] Given a `default` run, when it completes, then rustfmt has formatted
      the workspace once, not twice.

## Open Questions

- Should `default` check formatting and fail, as CI does, or fix it and pass?
- For lanes that need Docker or network egress, should `default` include
  them and skip cleanly when the prerequisite is missing, or exclude them
  with a documented reason?
- How much extra local time is acceptable for the lanes added?

## Dependencies

- Blocked by: none
- Blocks: none
- Related: 0361 (this item adds work to a local run, while 0361 removes it)

## Assumptions

- "Mirror CI as far as possible" allows documented exceptions for lanes
  whose prerequisites a contributor's machine may not have.

## Technical Notes

- `default`'s dependency list: `mise.toml:757-759`.
- `test:integration:pup` needs the nightly lane, which `tasks/README.md`
  ("The Rust nightly lane") says is isolated to `check-architecture`.
  Bringing it into `default` must not break that isolation guard in
  `tests/unit/tasks/test_workflows.py`.

## Drafting Notes

- Kept this item outside epic 0361, linked through `relates_to`, because it
  adds work rather than removing it. Agreed in conversation.
- Folded in two of the minor observations, the `format:fix` ordering hazard
  and duplicate workspace formatting, because both concern the shape of
  `default`'s task graph. The other minor observations were dropped.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
