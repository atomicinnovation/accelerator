---
type: work-item
id: "0194"
title: "Tracker Crate and Remote Sync Engine"
date: "2026-08-05T18:18:52+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0170"]
blocks: ["work-item:0170", "work-item:0171"]
tags: [rust, work-items, sync, tracker]
last_updated: "2026-08-05T19:37:49+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0194: Tracker Crate and Remote Sync Engine

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the shared `tracker` crate (the `RemoteTracker` port and sync state
machine, in pure domain terms) and the `accelerator work sync` command that
orchestrates it against the local work-item store, so both 0170's `create`/
`update --push` flows and 0171's per-provider client adapters have a stable
port to build against, and the sync flow can be unit-tested against a fake
`RemoteTracker`.

## Context

`skills/work/scripts/work-item-sync-*.sh` implements a transactional state
machine (classify → decide → apply → baseline → label) that orchestrates the
remote tracker. This story split off from 0170 (the work-item lifecycle
subdomain — see that item's Drafting Notes) on 2026-08-05 following a work
item review that found the two efforts independently deliverable: the
`tracker` crate depends only on the already-built shared `corpus`/`config`/
`store` crates (0166), not on 0170's own CRUD implementation, whereas 0170's
`create`/`update --push` flows and 0171's client adapters both need this
crate's `RemoteTracker` port to exist first. The `RemoteTracker` port and
the sync state machine live in their own `tracker` crate;
`accelerator-work` links the per-provider client adapters (0171) in-process
at its composition root and fakes the port in tests.

## Requirements

- Implement the `tracker` crate: the `RemoteTracker` port (issue/transition/
  sync verdict vocabulary) and the sync state machine in pure domain terms,
  with no provider-specific or HTTP types in its public API.
- Implement `accelerator work sync [--push-only|--pull-only]` in
  `accelerator-work`, orchestrating the tracker crate's classify → decide →
  apply → baseline → label pipeline against the local work-item store (via
  the shared `corpus`/`store` crates from 0166) and the active provider
  client, wired at the work binary's composition root per `work.integration`
  (the config key selecting the active remote tracker provider, e.g. `jira`
  or `linear`).
- Preserve the JSONL/atomic-write semantics for baseline writes (via
  `store`).
- Close the coverage gap: characterize-then-port the sync-side previously
  untested scripts — `work-item-sync-baseline.sh`, `work-item-sync-classify.sh`,
  `work-item-sync-decide.sh`, `work-item-sync-label.sh` (none has a dedicated
  `test-work-item-*.sh` suite today; `work-item-sync-apply.sh` already does
  and is out of scope here).
- Partition tests so the default `cargo test`/`cargo nextest run` invocation
  exercises the parity/characterization suite only (no live network calls);
  gate the contract/integration suite exercising real remote calls behind a
  separate, explicitly-tagged cargo-nextest filter excluded from that
  default invocation.
- Remove the migrated `work-item-sync-*.sh` scripts, `work-item-fetch-remote.sh`,
  and their `test-*.sh` suites, and decrement the work suite floor in the
  same change.

## Acceptance Criteria

- [ ] Given a local item and a fake `RemoteTracker` record, when
      `accelerator work sync` runs its classify → decide → apply → baseline →
      label pipeline, then the five-state classification (unsynced /
      local-ahead / remote-ahead / in-sync / conflict) matches the bash
      `work-item-sync-classify.sh` parity fixtures, and the per-item write
      sequence honours the "side effect first, baseline write last"
      resumability contract (the remote/local side effect for a given item is
      written before that item's baseline record, so a baseline write can be
      treated as confirmation the side effect already completed) — verified
      by a test harness that captures write-call order via a fake store and
      asserts the baseline write occurs strictly after the corresponding side
      effect for every classified state, and by simulating a crash after the
      side effect but before the baseline write and asserting a re-run
      produces the same terminal state as an uninterrupted run.
- [ ] Given `accelerator work sync --push-only` or `--pull-only`, then the
      decision table (the classification-state × sync-mode matrix the
      `decide` stage consults to choose push, pull, or no-op per item)
      forbids writes in the disallowed direction, matching
      `work-item-sync-decide.sh`'s forbidden-write cells.
- [ ] Given each of the four previously-untested sync scripts
      (`sync-baseline`, `sync-classify`, `sync-decide`, `sync-label`), a
      characterization test captures its pre-port behaviour — covering each
      documented flag/argument combination and at least one error path —
      before the Rust port replaces it.
- [ ] The `sync` parity suite (`accelerator work sync` against the
      repointed `skills/work/scripts/test-work-item-sync-*.sh` gates and the
      classify/decide fixtures) passes with no live network calls; remote
      calls are exercised only by a separate, explicitly-tagged contract/
      integration suite (gated behind a cargo-nextest filter excluded from
      the default `cargo test`/`cargo nextest run` invocation).
- [ ] The `tracker` crate's public API contains no provider-specific or HTTP
      types (verified by its dependency graph carrying no `reqwest` or
      provider-crate types in public signatures) — 0171's clients implement
      the port instead.
- [ ] The migrated `work-item-sync-*.sh` scripts, `work-item-fetch-remote.sh`,
      and their `test-*.sh` suites are removed and the work suite floor is
      decremented in the same change.

## Open Questions

- Whether `work-item-sync-decide.sh`'s internal-function boundary (kept as
  a private function inside `sync` per 0170's Technical Notes, not a
  separate subcommand) holds once the `tracker` crate scaffolding starts —
  a bash-era boundary may turn out to matter for a reason not visible from
  the script's header comment alone. Carried over from 0170's Drafting
  Notes as the sync-specific half of that judgment call.

## Dependencies

- No remaining blockers: 0166 (shared crates) and 0187 (generalises the
  sub-binary registration surface) are both done as of 2026-08-05. The
  `tracker` crate's own build only needs these two, not 0170's CRUD
  implementation.
- Blocks: 0170 (the work-item lifecycle subdomain) — `create`/`update
  --push` call through the `RemoteTracker` port this story defines, so
  0170 cannot implement `--push` (though it can implement the rest of its
  CRUD surface) until this crate's port is stable. This blocking milestone
  is narrower than this item's full Acceptance Criteria gate: 0170 only
  needs the `RemoteTracker` trait signature to compile, not the finished
  `accelerator work sync` command, its characterization-test suites, or the
  legacy-script removal — those can land after 0170 unblocks.
- Blocks: 0171 (Jira and Linear Integrations) — its client adapters
  implement the `RemoteTracker` port this story defines; 0171 cannot
  complete its `impl RemoteTracker` work until this crate's port is
  stable. As with 0170, only the port signature is the actual blocking
  milestone, not this item's full acceptance gate.
- Parent: epic 0136.

## Assumptions

- `reqwest` in the work binary (via the client adapters 0171 provides) is
  acceptable — it is already workspace-wide via the launcher, resolved by
  the work↔integrations coupling decision (research doc
  `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`,
  Open Question 2).
- The active provider client is faked at the `RemoteTracker` port boundary
  for this story's own unit tests; real Jira/Linear wiring is exercised
  once 0171 lands, via the separate contract/integration suite.

## Technical Notes

- Source bash: `skills/work/scripts/work-item-sync-*.sh` and
  `work-item-fetch-remote.sh` (the sync engine's read counterpart to
  `work-item-create-remote.sh`; `work-item-sync-apply.sh` calls it directly
  — it is not a 0170 lifecycle concern despite appearing in that item's
  original source-bash list before the split. It already has a dedicated
  `test-work-item-fetch-remote.sh` suite, so it needs no new
  characterization test, only porting and removal alongside the other
  sync-stage scripts).
- The sync flow depends on config + JSONL/store + the integration clients
  (0171); this story only requires the port to exist, not a concrete
  provider implementation.
- `work sync [--push-only|--pull-only]` runs the classify → decide → apply →
  baseline → label pipeline as one command; the current plan is for the five
  stages to stay internal functions, not separate subcommands, unit-tested
  directly at the function level — pending confirmation once the `tracker`
  crate scaffolding starts (see Open Questions).
- Because the reversed sequencing means this story, not 0170, may be the
  first to scaffold/extend the `accelerator-work` binary and its composition
  root, registration follows the same checklist 0187 adds at
  `tasks/README.md#registering-a-dispatched-sub-binary` that 0170's
  Technical Notes also references.

## Drafting Notes

- Split from 0170 on 2026-08-05 following work item review 1
  (`meta/reviews/work/0170-work-item-subdomain-and-sync-engine-review-1.md`),
  which found the lifecycle CRUD and sync engine independently deliverable.
  This item carries the `tracker` crate and `sync` command; 0170 keeps the
  CRUD lifecycle ops. Because 0170's `--push` flows and 0171's clients both
  build against this crate's port, this item precedes both rather than
  depending on either — the reverse of the sequencing first assumed during
  the split discussion.
- The four-script untested-scripts list here was verified against the
  actual `skills/work/scripts/` inventory at split time: scripts with a
  dedicated `test-work-item-<name>.sh` file (`sync-apply`) are excluded;
  the remaining sync-stage scripts without one are listed above.
- Revised following work item review 1
  (`meta/reviews/work/0194-tracker-crate-and-remote-sync-engine-review-1.md`):
  added a concrete verification mechanism for AC1's resumability contract,
  clarified that Dependencies' blocking milestone is the port signature
  compiling rather than this item's full Acceptance Criteria gate, added a
  Technical Notes cross-reference to the 0187 registration checklist, added
  Requirements bullets mirroring AC4's test-partitioning and AC6's
  removal/floor-decrement scope, and glossed several previously-undefined
  terms (`work.integration`, the decision table, the Q2 reference).

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Split from: `meta/work/0170-work-item-subdomain-and-sync-engine.md`
- ADRs: ADR-0045, ADR-0052, ADR-0053
