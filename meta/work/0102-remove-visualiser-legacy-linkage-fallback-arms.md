---
type: work-item
id: "0102"
title: "Remove Visualiser-Server Legacy Linkage Fallback Arms (Follow-on Contract)"
date: "2026-06-09T10:30:00+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0057"
blocked_by: ["work-item:0070"]
relates_to: ["adr:ADR-0034", "adr:ADR-0033"]
tags: [migration, visualiser, frontmatter, cleanup, contract]
last_updated: "2026-06-09T10:30:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0102: Remove Visualiser-Server Legacy Linkage Fallback Arms

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Story 0070 shipped the unified-schema corpus migration and **expanded** the
visualiser server's reader to accept both legacy and unified linkage shapes,
**deprecating** (not removing) the transitional fallback arms so a userspace
repo that has not yet run `/accelerator:migrate` is not broken. This story is
the **contract** half of that expand/migrate/contract split: once every
consuming repo has migrated, remove the now-deprecated fallback arms and their
pinning tests.

## Context

0070 deliberately deferred arm removal because migrate-on-use is *advisory*, not
enforced (`migrate/SKILL.md`: skills do not gate on pending migrations), so a
dormant repo could upgrade the visualiser before applying the migration and
silently lose cross-references. The expanded reader (shipped under 0070, Phase
5a) accepts both shapes and emits a `tracing::warn!` deprecation whenever a
legacy arm resolves, so the arms are safe to retain until the migration has
propagated everywhere. This story removes them once that condition holds.

The `parent:`-orphaning risk is already closed: 0070's migration mechanically
derives `parent:` from the foreign `work_item_id:`, so the canonical clustering
key is populated corpus-wide and the follow-on inherits a corpus where the
canonical side is present.

## Requirements

- Remove the three retained-and-deprecated fallback arms in the visualiser
  server (re-derive the exact line refs against the then-current source):
  - the `work-item:` arm in `frontmatter.rs:read_ref_keys` (and its
    `tracing::warn!`);
  - the legacy `work_item_id:` branch in `cluster_key.rs:parent_or_legacy_id`,
    and rename `parent_or_legacy_id` (the `parent:` branch is the only remaining
    resolution path);
  - the filename fallback in `indexer.rs` work-item identity (and its
    `tracing::warn!`); `id:` becomes primary with `work_item_id:` retained only
    if still needed.
- Delete the pinning tests that assert the *removed* legacy behaviour, and
  retarget the survivors to assert canonical resolution. Note when re-deriving:
  - the `cluster_key.rs` tests near the retained-plan block assert *retained*
    plan `work_item_id`/path-shape parent resolution — **not** the legacy
    work-item arm — and stay green;
  - the genuine work-item-review legacy/typed tests to update are
    `work_item_review_target_path_resolves_to_work_item_id` and
    `…_typed_work_item_target_short_circuits`;
  - the deprecation-warning tests added under 0070 (the per-arm `capture_logs`
    assertions) are deleted with their arms.
- Add an **observable migration-completion gate** for arm removal: a
  corpus-wide grep returning zero surviving legacy `work-item:` / `work_item_id:`
  own-identity shapes (rather than a manual "everyone has migrated" judgement).

## Acceptance Criteria

- [ ] No `work-item:` fallback remains in `frontmatter.rs:read_ref_keys`; no
      `parent_or_legacy_id` path remains in `cluster_key.rs`; no filename
      fallback remains in `indexer.rs` (a grep for `parent_or_legacy_id` and the
      legacy keys returns nothing) — the 0070 AC-12/AC-13 deferred here.
- [ ] `mise run test:unit:visualiser` passes (both feature modes) with the
      legacy pinning tests deleted and the survivors retargeted to canonical
      resolution.
- [ ] The migration-completion gate is in place and passes against the
      reference corpus (zero surviving legacy own-identity shapes).

## Dependencies

- Blocked by: 0070 (ships the migration + the reader-expand/deprecate this
  story contracts). Removal must not precede every consuming userspace repo
  having run `/accelerator:migrate` at least once — the migration-completion
  gate is the observable proxy for that condition.
- Related: 0057 (parent epic), ADR-0034 (typed linkage vocabulary), ADR-0033
  (identity contract).

## Drafting Notes

- Raised as the explicit follow-on deliverable of 0070's revised
  expand/migrate/contract sequencing (0070 plan, Phase 5 §5), so the
  deprecate-then-contract sequence has an owner and cannot ossify in the
  expanded-but-never-contracted state.
