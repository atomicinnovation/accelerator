---
type: work-item
id: "0170"
title: "Work-Item Subdomain and Sync Engine"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0171"]
tags: [rust, work-items, sync, tracker]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-191"
---

# 0170: Work-Item Subdomain and Sync Engine

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the `accelerator-work` subdomain — work-item lifecycle operations plus the
remote sync engine — and the shared `tracker` crate (the `RemoteTracker` port and
sync state machine), wiring the active integration client in-process at the work
binary's composition root (resolved Q2).

## Context

`skills/work/scripts/` (22 prod scripts) covers create/fetch/update/sync/normalise/
next-number/section-diff/read-field. Sync is a transactional state machine
(classify → decide → apply → baseline → label) that orchestrates the remote tracker.
Resolved Q2: the `RemoteTracker` port and the sync state machine live in their own
`tracker` crate, and `accelerator-work` links the per-provider client adapters
(0171) in-process and fakes the port in tests. ~14 `work-item-*` scripts have no
dedicated test suite — a coverage gap to close.

## Requirements

- Implement `accelerator-work` over the shared `corpus`/`config`/`store` crates:
  lifecycle ops (create, read-field, update, update-tags, next-number, normalise,
  section-diff) and the sync flow (classify/decide/apply/baseline/label).
- Implement the `tracker` crate: the `RemoteTracker` port (issue/transition/sync
  verdict vocabulary) and the sync state machine in pure domain terms; the work
  binary wires the active provider per `work.integration` at its composition root,
  faking the port in unit tests.
- Preserve the `external_id`-as-remote-key convention and the JSONL/atomic-write
  semantics (via `store`).
- Close the coverage gap: characterize-then-port the ~14 untested `work-item-*`
  scripts (resolved Q7).

## Acceptance Criteria

- [ ] `accelerator work …` reproduces the lifecycle and sync behaviours, verified
      against the repointed `skills/work/scripts/test-*.sh` parity gates where they
      exist, and against characterization tests for the previously untested scripts.
- [ ] The sync state machine is unit-tested in-process against a fake
      `RemoteTracker`, with no live network in unit tests.
- [ ] The `tracker` port carries no provider-specific or HTTP types; provider
      clients (0171) implement it.
- [ ] The migrated `work-item-*` scripts are removed and the work suite floor
      decremented in the same change.

## Open Questions

- Exact `accelerator work` subcommand vocabulary (which shell scripts collapse into
  one subcommand with flags vs stay distinct) — decided during implementation per
  the Q7 interface-redesign principle.

## Dependencies

- Blocked by: 0166 (shared crates).
- Relates to: 0171 (provides the `jira-client`/`linear-client` adapters the sync
  engine wires in).
- Parent: epic 0136.

## Assumptions

- `reqwest` in the work binary (via the client adapters) is acceptable — it is
  already workspace-wide via the launcher (resolved Q2).

## Technical Notes

- Source bash: `skills/work/scripts/work-item-common.sh`, `work-item-sync-*.sh`,
  `work-item-create-remote.sh`, `work-item-fetch-remote.sh`,
  `work-item-next-number.sh`, `work-item-pattern.sh`, etc.
- The sync flow depends on config + JSONL/store + the integration clients.

## Drafting Notes

- Treated as the Phase 7 story; introduces the `tracker` crate that 0171's clients
  implement, so the two are tightly related but separable.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0052, ADR-0053
