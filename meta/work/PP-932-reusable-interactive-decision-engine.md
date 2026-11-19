---
type: "work-item"
id: "PP-932"
title: "Reusable Interactive Decision Engine"
date: "2026-10-15T18:23:05+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:PP-898"
relates_to: ["work-item:PP-732", "work-item:PP-879"]
tags: ["migration-engine", "interactive", "rust"]
last_updated: "2026-10-15T18:23:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-932"
aliases: ["0344"]
---
# PP-932: Reusable Interactive Decision Engine

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer building a command that needs a human or agent to decide
ambiguous cases, I want `accelerator migrate`'s interactive decision engine
available as its own crate, so that commands such as `work rekey` can surface
candidates with context and take decisions back through the same TTY,
decisions-file, stall and resume contract.

## Context

`migrate` already runs a candidate → predicate → decision → record-then-apply
pipeline (`cli/migrate/src/engine.rs`): candidates route to `Mechanical` or
`Prompt`, a `DecisionSource` answers from a TTY or a decisions file, and every
decision is appended to a session log before it applies, so a stalled run
resumes and replays what was decided. The `migrate` skill drives it as list,
decide, write a decisions file, resume.

The engine is coupled to migration concepts: `InteractiveMigration` requires
`MigrationMeta` and takes `MigrationContext` in every callback, and session
logs, stall messages and decisions-file paths are keyed by migration id. The
adapters assume at most one interactive migration. A candidate carries only a
`display` string, `proposed`, and `path:anchor` — no surrounding source — and
`--list` omits `display`.

work-item:PP-933 (bare-number review) needs this engine to ask whether a number
in prose names a work item, which cannot be decided without the sentence
around it.

## Requirements

- The decision core — candidates, predicate outcomes, decisions, decision
  sources, the session log, decisions-file parsing and validation, stall
  reporting, and record-then-apply — lives in a crate that depends on neither
  `migrate` nor `MigrationContext`.
- `migrate` and `work` both consume it.
- A candidate carries a window of source lines around its anchor, shown in
  the TTY prompt and in the listing output.
- Session-log and decisions-file locations are keyed by consumer and run, so
  more than one consumer can be interactive in one repository.
- `migrate`'s observable behaviour is otherwise unchanged: decisions-file
  format, stall message, listing columns, and resume.

**Out of scope**

- New decision verbs beyond `accept`, `skip` and `edit`.
- Changes to the `migrate` skill's invoker contract beyond showing context.

## Acceptance Criteria

- [ ] Given the existing `migrate` interactive test suite, when it runs
      against the extracted engine, then every test passes unchanged.
- [ ] Given a candidate anchored at line 40 of a file, when the candidates
      are listed, then the output includes the lines around line 40.
- [ ] Given two consumers with pending decisions in one repository, when each
      stalls and resumes, then each reads and writes only its own session log
      and decisions file.
- [ ] Given the extracted crate, when its dependencies are checked, then it
      depends on neither `migrate` nor any `migrate` adapter crate.

## Open Questions

- Should the `migrate` skill's invoker contract present the new context to
  the agent now, or once work-item:PP-732's successor ADRs settle the ports?

## Dependencies

- Blocked by: none.
- Blocks: work-item:PP-933 — Bare-Number Review During Alias Normalisation.
- Related: work-item:PP-732 — its successor ADRs document `DecisionSource`,
  `SessionLog` and `MigrationContext`, which this extraction reshapes.
- Related: work-item:PP-879 — the `work rekey` command whose alias
  normalisation becomes the second consumer.

## Assumptions

- A fixed window of lines around the anchor is enough context to decide a
  prose candidate; richer context (the enclosing section) is not needed.

## Technical Notes

- Core types and ports: `Transformation`, `PredicateOutcome`, `Decision`,
  `InteractiveMigration` (`cli/migrate/src/interactive.rs`); `DecisionSource`,
  `SessionLog`, `SessionLogFactory` (`cli/migrate/src/ports.rs`);
  `run_interactive`, `pending_transformations`, `record_then_apply`
  (`cli/migrate/src/engine.rs`); `cli/migrate/src/decisions_file.rs`.
- The single-consumer assumption lives in `cli/migrate-cli/src/main.rs`
  (one session-log path) and the shared cursor in
  `cli/migrate-adapters/src/decisions_file_decision_source.rs`.
- The TTY prompt text lives in
  `cli/migrate-adapters/src/tty_decision_source.rs`; listing renders in
  `cli/migrate-cli/src/render.rs`.
- Registering a new library crate follows the library-crate checklist in
  `tasks/README.md`.

## Drafting Notes

- Parented under work-item:PP-898 because the engine is migrate-owned; the
  extraction serves `work` but changes `migrate`'s crates.
- Context is a window of source lines rather than a per-consumer formatter,
  to keep the candidate model consumer-agnostic.

## References

- Source: work-item:PP-879 — Work Item Re-Key Command (stress test)
- Related: work-item:PP-137 — Make Interactive Migrations Satisfiable Under
  Agent Invocation
- Related: work-item:PP-732 — Supersede Migration-Engine ADRs with Rust-Native
  Successors
