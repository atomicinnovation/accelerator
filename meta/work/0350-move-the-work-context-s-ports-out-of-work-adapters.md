---
type: "work-item"
id: "0350"
title: "Move the Work Context's Ports out of work-adapters"
date: "2026-10-24T11:26:33+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
relates_to: ["work-item:0299", "work-item:0302"]
external_id: "PP-925"
tags: ["architecture"]
last_updated: "2026-10-24T11:26:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0350: Move the Work Context's Ports out of work-adapters

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`work-adapters` defines 10 port traits that ADR-0053 says belong in the
`work` domain crate, alongside the use-case orchestration that consumes
them. Moving them restores the inward dependency rule for the work
context and gives the orchestration a deliberate home rather than a
default one.

## Context

ADR-0053 states that ports "are defined as traits in the core; both
driving and driven ports live with the domain/application, not with the
adapters". ADR-0069's layer table names only domain, adapter and
composition-root crates, so orchestration that needs more than `work`'s
pup allow-list (`std`, `kernel::Error`, `corpus`, `tracker`) has drifted
into `work-adapters`, and its ports went with it. `rekey/run.rs`, for
example, needs `vcs::RepositoryProbe`, another context's port that
ADR-0069 expects to arrive through a work-owned port injected at the
composition root.

The identity run (`IdentityRun`, `RunPlanStatus`, `begin_identity_run`,
`begin_rekey_run`) was moved into `work::identity_run` under PR 168 as the
first instance; its filesystem lock stays in `work-adapters`.

Remaining traits in `work-adapters`:

| Trait | Module |
|---|---|
| `RetirementRecords` | `retirement_records.rs` |
| `CreationStore`, `NewItem` | `creation.rs` |
| `PromotionRecords` | `promotion_records.rs` |
| `LocalAuthor` | `sync/create.rs` |
| `CorpusDiscovery` | `sync/settled_run.rs` |
| `WorkingCopyStatus` | `sync/fetch.rs` |
| `RunPlanStore` | `rekey/run_plan_store.rs` |
| `RekeyProgress`, `KeyRefresh` | `rekey/run.rs` |

## Requirements

- Every port trait the work context's orchestration depends on is
  defined in the `work` crate.
- Orchestration that needs another context's capability (e.g. the
  repository probe) receives it through a port owned by `work`, bound at
  the `work-cli` composition root.
- `work-adapters` holds only implementations of `work`'s ports.
- A `pup.ron` rule enforces that `work-adapters` defines no public
  traits, so the drift cannot recur silently.
- Decide where use-case orchestration lives: inside `work` (as
  ADR-0053's "domain + application core" implies) or in a new
  application-layer crate, and record the choice by amending or
  superseding ADR-0069.

## Acceptance Criteria

- [ ] Given the CLI workspace, when listing `pub trait` definitions in
      `cli/work-adapters/src`, then there are none.
- [ ] Given the moved ports, when `mise run pup:check` runs, then it
      passes with `work`'s allow-list unchanged, or with any widening
      justified in the ADR change.
- [ ] Given `pup.ron`, when a public trait is added to `work-adapters`,
      then `pup:check` fails.
- [ ] Given the change, when `mise run` runs, then it exits 0, with
      `work/tests/fixtures/public-api.txt` regenerated to include the
      moved ports.
- [ ] Given ADR-0069, when it is read after this work, then it names
      where use-case orchestration lives.

## Open Questions

- Does orchestration belong in `work` itself, or in a separate
  application crate between domain and adapters?
- Can cargo-pup express "no public traits in this crate", or does that
  rule need a different check?

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- The orchestration moves with its ports; moving only the traits would
  leave `work-adapters` consuming ports it does not implement, which
  inverts the problem rather than fixing it.

## Technical Notes

- `work::identity_run` is the template: pure decision logic plus port
  traits in `work`, the concrete lock in `work-adapters`.
- Each move adds to `work`'s pinned public API; regenerate with
  `mise run public-api:update`.

## Drafting Notes

- Scoped to the work context only. Other `*-adapters` crates have not been
  audited for the same drift; that would be a separate item.
- The pup enforcement requirement is my addition; drop it if a
  convention is preferred over a rule.
- Kind is `task` rather than `story`: no user-visible behaviour changes.

## References

- ADR-0053: `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
- ADR-0069: `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- Related: 0299 (crate dependencies brought into line with ADR-0069)
- Related: 0302 (introduced the identity run and re-key ports)
