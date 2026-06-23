---
type: work-item
id: "0218"
title: "Bound cache-root growth"
date: "2026-08-17T20:36:50+00:00"
author: Toby Clemson
producer: implement-plan
status: draft
kind: task
priority: low
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0189", "work-item:0164"]
tags: [cli, launcher, performance, bootstrap]
last_updated: "2026-08-17T20:36:50+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-747
---

# 0218: Bound cache-root growth

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Bound the launcher's cache-root growth. `cache::find` scans that directory on
**every** warm dispatch, and the module header declares it needs no eviction — so
the scan term grows without limit in accumulated versions and staged shims.

## Context

Measured in work item 0189's closing session
(`meta/measurements/warm-dispatch-3.json`): `cache::find` costs **0.036 ms**
against a cache root holding **21 entries and 47.8 MB**. Small today, and
unbounded over a long-lived plugin root: every prerelease leaves a launcher, its
`.minisig` and a sub-binary behind, and the directory is never pruned.

The growth is not only a scan cost. The same directory is the launcher's
exec target and, for the measurement harness, an integrity witness — its entry
set is compared before and after a session — so unbounded accumulation makes that
witness progressively noisier too.

⚠️ Raised deliberately rather than left implied: 0189 measured the term and then
closed, so without this item a measured, growing term on the hook path would have
no owner.

## Requirements

- Decide and implement a bound: a retention policy, or eviction on successful
  store.
- Never evict before a verified replacement exists. `fetch_and_verify` already
  renames a verified successor over a corrupt entry rather than removing first;
  eviction must preserve that property.
- Leave concurrent first-use safe: the lock directory and the `.tmp-` staging
  namespace are in the same directory, and a sweep must not race a store.
- Size the policy from real data — the entry count and total size 0189 records —
  rather than from a round number.

## Acceptance Criteria

- [ ] The cache root's entry count is bounded by a stated policy, with the policy
      and its rationale recorded.
- [ ] A dispatch concurrent with an eviction is shown safe by a named test,
      including that no in-use entry is removed.
- [ ] No entry is evicted before a verified replacement exists.
- [ ] `cache::find`'s cost is re-measured after the change via `mise run
      measure:warm-dispatch`, and the before/after recorded.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- **Relates to** 0189, which measured the term and the directory's state, and
  0164, which established the fetch-verify-cache resolver.
- **Parent**: epic 0136.

## Assumptions

- Retaining one prior version is enough for rollback. If the release process
  needs more, the policy widens rather than the item changing shape.

## References

- `cli/launcher/src/launch/outbound/resolve/cache.rs:1-6`, `:51-73` — the
  never-evicted root, its scan, and the `.tmp-` namespace
- `bin/accelerator:201` — the cache root derivation
- `meta/measurements/warm-dispatch-3.json` — 21 entries, 47.8 MB, 0.036 ms
