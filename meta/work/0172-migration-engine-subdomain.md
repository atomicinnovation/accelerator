---
type: work-item
id: "0172"
title: "Migration Engine Subdomain"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
parent: "work-item:0136"
relates_to: ["work-item:0180"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, migration-engine, concurrency]
last_updated: "2026-07-18T22:02:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-193"
---

# 0172: Migration Engine Subdomain

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Port the meta-directory migration engine into `accelerator-migrate` — the
highest-risk port — replacing the bash FIFO IPC, watchdog, and embedded awk JSON
parser with Rust, and `serde_json` removing the dual writer/reader escape hazard.

## Context

`skills/config/migrate/scripts/` is the most complex and stateful cluster:
`run-migrations.sh` (677), `interactive-lib.sh` (985) with two named FIFOs + literal
fds for bidirectional IPC (bash 3.2 has no `coproc`), a 30s watchdog escalating
SIGTERM→SIGKILL, JSON parsed twice by two independent escape implementations that
must agree, guarded resume keyed on jj `change_id` vs git `HEAD`, and 7 numbered
migrations. It sits last on the dependency spine and depends on the store/config
crates.

## Requirements

- Implement `accelerator-migrate` (`migrate run`, `migrate status`,
  `migrate discoverability`) over the shared `config`/`store` crates.
- Replace the FIFO/fd IPC and the 30s watchdog with a Rust concurrency model;
  replace the dual hand-rolled JSON escape (shell writer + awk reader) with
  `serde_json`, eliminating the escape-agreement hazard.
- Preserve the guarded-resume / staleness semantics (jj `change_id` vs git `HEAD`,
  fail-closed dirty-tree ownership check) and the interactive protocol's behaviour.
- Port the 7 numbered migrations and their fixtures; keep the legacy
  `.claude/accelerator.md` read path (`ACCELERATOR_MIGRATION_MODE`) working through
  the migration sequence.

## Acceptance Criteria

- [ ] `accelerator migrate run` applies pending migrations with the same
      guarded-resume and dirty-tree semantics, verified against the repointed
      `test-migrate*.sh` parity gates.
- [ ] The interactive protocol (prompt/response state machine) behaves equivalently,
      with the watchdog/timeout semantics preserved.
- [ ] JSON is composed/parsed once via `serde_json` (no dual escape implementation).
- [ ] The migrate shell scripts are removed and the migrate suite floor decremented
      in the same change.

## Open Questions

- Whether the interactive protocol's transport (was FIFOs + fds) maps to stdin/stdout
  framing or an internal channel in the Rust model — decided during implementation.

## Dependencies

- Blocked by: 0166 (shared `config`/`store` crates).
- Parent: epic 0136.
- Clean-cutover obligation from 0180 (`relates_to: 0180`): 0180 ports the
  canonical-order JSONL primitives and scopes bash↔Rust byte-parity out on the
  premise that no session-log file is written by both the bash primitives and the
  Rust port concurrently. As the one production JSONL caller, this engine must
  honour that premise — cut a given session-log file over from bash to Rust
  atomically, never interleaving a bash writer and a Rust writer against the same
  file. (This engine's own move to `serde_json` for compose/parse must also stay
  consistent with 0180's canonical field order and escaper for any records that
  outlive the cutover.) If an atomic cutover cannot be guaranteed, flag 0180 so
  its byte-parity scope-out is revisited.

## Assumptions

- The migration engine can be deferred to a late phase without blocking earlier
  clusters (its consumers are the config/store crates, already built).

## Technical Notes

- Source bash: `skills/config/migrate/scripts/run-migrations.sh`,
  `interactive-lib.sh`, `scripts/interactive-protocol.sh`, the 7 `migrations/*.sh`,
  and the large `test-migrate*.sh` suites + fixtures.
- This is the most subtle concurrency port; the repointed shell suites are the
  oracle.

## Drafting Notes

- Treated as the Phase 9 story; flagged highest-risk and sequenced last on the
  dependency spine per the research.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0047, ADR-0052, ADR-0053
- Prior research: `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
