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
blocks: ["work-item:0057"]
relates_to: ["adr:ADR-0034", "adr:ADR-0033"]
tags: [migration, visualiser, frontmatter, cleanup, contract]
last_updated: "2026-06-15T21:14:04+00:00"
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
the **contract** half of that expand/migrate/contract split — *contract* here
being the pattern's third phase (the refactoring step that removes the now-dead
compatibility paths), not an interface agreement. Once every consuming repo has
migrated, it removes those fallback arms and their pinning tests — both the arms
0070 deprecated (`work-item:`/`work_item_id:`) and the older `ticket:` arm, which
predates 0070, carries no deprecation warn, and is gated instead on migration
`0001` — so that visualiser-server maintainers read and reason about a single
canonical linkage resolution path.

## Context

0070 deliberately deferred arm removal because migrate-on-use is *advisory*, not
enforced (`migrate/SKILL.md`: skills do not gate on pending migrations), so a
dormant repo could upgrade the visualiser before applying the migration and
silently lose cross-references. The expanded reader (shipped under 0070, Phase
5a) accepts both shapes and emits a `tracing::warn!` deprecation whenever a
legacy arm resolves, so the arms are safe to retain until the migration has
propagated everywhere. This story removes them once the migration has propagated
to every consuming repo — a condition observed via the migration-completion gate
(see Dependencies).

The `parent:`-orphaning risk is already closed: 0070's migration mechanically
derives `parent:` from the foreign `work_item_id:`, so the canonical clustering
key is populated corpus-wide and the follow-on inherits a corpus where the
canonical side is present.

## Requirements

> Note on `work_item_id:` — the token denotes several distinct roles in this
> story: the **own-identity** shape (a document declaring its own legacy id —
> the target of the gate clause, the `cluster_key.rs:parent_or_legacy_id` branch,
> and the `indexer.rs` identity branch) versus a **foreign** reference
> (deliberately left in place by 0070). Every mention below refers to the
> own-identity role unless stated otherwise.

- Remove the four retained legacy fallback arms in the visualiser server
  (re-derive the exact line refs against the then-current source):
  - the `work-item:` arm in `frontmatter.rs:read_ref_keys` (and its
    `tracing::warn!`);
  - the `ticket:` arm in the same `frontmatter.rs:read_ref_keys` chain — the
    oldest legacy shape, migrated out by migration `0001` (the
    `ticket`→`work-item` rename), predating the 0070 unified-schema work. It
    carries **no** deprecation warn, so removal rests on migration `0001` having
    propagated, not on 0070's gate (see the gate requirement below);
  - the legacy `work_item_id:` branch in `cluster_key.rs:parent_or_legacy_id`,
    and rename `parent_or_legacy_id` (the `parent:` branch is the only remaining
    resolution path);
  - the filename fallback in `indexer.rs` work-item identity (and its
    `tracing::warn!`); `id:` becomes primary. Decision rule for `work_item_id:`:
    the implementer retains it only if a non-legacy reader still consumes it
    after the arms are removed (checked against the then-current reader);
    otherwise it is removed. Default to removal when no such consumer remains.
- **Do not** touch the typed `target:` arm in the same
  `frontmatter.rs:read_ref_keys` chain. `target:` is the current first-class
  ADR-0034 typed-linkage key — the destination the legacy arms migrate toward,
  not a peer in the legacy set — and is independently load-bearing via
  `target_path_from_entry` (`indexer.rs`) for plan-review / work-item-review /
  validation linkage. It stays.
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
- Add an **observable migration-completion gate** for arm removal: a single
  recursive grep over this repo's `meta/` returning zero surviving legacy
  `work-item:` / `work_item_id:` own-identity shapes (rather than a manual
  "everyone has migrated" judgement). Because `ticket:` is now in scope, the gate
  needs a **second clause** asserting zero surviving `ticket:` / `ticket_id:`
  shapes — noting this clause rests on migration `0001` having propagated, a
  distinct (older) guarantee from the 0070 migration the rest of the gate covers.
  This reference-corpus grep is the **accepted, definitive proxy** for "every
  consuming repo has migrated"; arm removal does *not* additionally require a
  release-gate or deprecation-window signal (resolved — see Open Questions).

## Acceptance Criteria

- [ ] No legacy resolution path remains in the visualiser server: no
      `work-item:` or `ticket:` fallback in `frontmatter.rs:read_ref_keys`, no
      `parent_or_legacy_id` path in `cluster_key.rs`, and no filename fallback in
      `indexer.rs` (the `parent_or_legacy_id` and filename-fallback removals 0070
      deferred). Because this story also
      *renames* `parent_or_legacy_id`, the controlling condition is the **absence
      of any legacy resolution path**, not the survival of a particular symbol
      name — verified by a grep over the server source
      (`skills/visualisation/visualise/server/src/`) for `parent_or_legacy_id`,
      `work-item:`, `ticket:`, the legacy `work_item_id:` own-identity branch,
      and the filename fallback, all returning nothing. The typed `target:` arm
      is retained.
- [ ] The migration-completion gate is in place and passes. It is a **single**
      recursive grep over this repo's `meta/` (the reference corpus) with two
      clauses: (a) zero surviving `work-item:`/`work_item_id:` own-identity
      shapes, and (b) zero surviving `ticket:`/`ticket_id:` shapes. Both clauses
      pass. The pattern matches a document's *own legacy identity declaration*
      and must **not** match canonical typed-linkage *references*, which
      legitimately carry the `work-item:` token; anchor to the frontmatter key at
      line start to make the distinction mechanical:
      - MATCH (legacy own-identity — must be zero): `^\s*work_item_id:` /
        `^\s*ticket_id:` (e.g. `work_item_id: "0042"`).
      - NO MATCH (canonical typed reference — retained): `parent:
        "work-item:0057"`, `target: "work-item:0102"`, `relates_to:
        ["work-item:0070"]`.
      This grep is the accepted, definitive proxy for "every consuming repo has
      migrated" — no separate release-gate signal is required (see
      Requirements/Dependencies/Open Questions).
- [ ] `mise run test:unit:visualiser` passes in both server feature modes
      (`embed-dist`, the default, and `dev-frontend`) with the legacy pinning
      tests deleted and the survivors retargeted to canonical resolution.

## Open Questions

_None outstanding._

**Resolved — what the migration-completion gate proves.** The gate is a
corpus-wide grep over *this* repo's `meta/` returning zero surviving legacy
own-identity shapes. Migrate-on-use is advisory across *external/userspace*
repos, which this repo's corpus cannot observe, so the reference-corpus grep is
the **accepted, definitive proxy** for "every consuming repo has migrated": arm
removal does *not* additionally require a release-gate or deprecation-window
signal. The gate spans *two* migrations — 0070 (`work-item:`/`work_item_id:`)
and the older 0001 (`ticket:`/`ticket_id:`) — so the proxy must hold for both
(see Dependencies).

## Dependencies

- Blocked by: 0070 (ships the migration + the reader-expand/deprecate this
  story contracts). Removal of the `work-item:`/`work_item_id:` arms must not
  precede every consuming userspace repo having run `/accelerator:migrate` at
  least once — the migration-completion gate (clause a) is the observable proxy
  for that condition.
- Second precondition (for the `ticket:`/`ticket_id:` arm): migration `0001`
  (`ticket`→`work-item`) having propagated to every consuming repo. This is a
  distinct, older guarantee than 0070's, observed by the gate's second clause
  (b). It is not reflected in `blocked_by` (no work-item blocker carries it), but
  it gates the `ticket:` removal exactly as 0070 gates the `work-item:` removal.
- Blocks: 0057 (parent epic) closure. This is the contract step 0070 deferred;
  the epic cannot close in the expanded-but-never-contracted state, so 0057's
  completion is gated on this story landing.
- Related: ADR-0034 (typed linkage vocabulary), ADR-0033 (identity contract).

## Technical Notes

Verified against current source (`skills/visualisation/visualise/server/src/`)
as of revision `20e5760`. Re-confirm before editing, but these are the
confirmed coordinates rather than cold re-derivation:

- **`work-item:` arm** — `frontmatter.rs:350-364` (the `work-item` branch inside
  `read_ref_keys`, function at `:321`); its `tracing::warn!` is at `:357-362`.
- **`ticket:` arm** — `frontmatter.rs:365-368`, in the same `read_ref_keys`
  chain. Oldest legacy shape (docstring `:317` "older legacy fallback"); carries
  **no** deprecation warn, so there is no per-arm `capture_logs` test to delete
  for it. Its lineage is the `ticket`→`work-item` BREAKING rename recorded in
  `CHANGELOG.md` (migration `0001`). In scope for removal.
- **`target:` arm (typed) — KEEP** — `frontmatter.rs:369-380`. Current ADR-0034
  typed-linkage key (`parse_typed_ref` in `typed_ref.rs`); also consumed
  independently by `target_path_from_entry` (`indexer.rs:974-998`) for
  review/validation linkage. Not legacy; must not be removed.
- **`parent_or_legacy_id`** — `cluster_key.rs:128`; `parent:` branch
  `:132-137`, legacy `work_item_id:` branch `:138-155`, deprecation warn
  `:148-152`. Caveat: this file imports the macro as bare `warn!`, not
  `tracing::warn!`.
- **`indexer.rs` filename fallback** — identity chain in `build_entry` at
  `:1382-1400`; `id:` primary `:1382`, legacy `work_item_id:` warn `:1384-1390`,
  filename-fallback `tracing::warn!` `:1391-1397`. **Do not remove** the
  separate shape-validation `tracing::warn!` at `:1372-1377` — it is not a
  legacy-arm warn.
- **Pinning/deprecation tests to delete with their arms** (all
  `capture_logs`-based): `frontmatter.rs:573`, `cluster_key.rs:294`,
  `indexer.rs:3467`, and `indexer.rs:3486`.
- **Survivors to retarget** to canonical resolution: `cluster_key.rs:403`
  (`work_item_review_target_path_resolves_to_work_item_id`) and `:433`
  (`work_item_review_typed_work_item_target_short_circuits`).
- **Retained-plan block** — has no literal label in source; it is the run of
  plan-resolution tests at `cluster_key.rs:278`, `:320`, and `:336`, which stay
  green.

## Drafting Notes

- Raised as the explicit follow-on deliverable of 0070's revised
  expand/migrate/contract sequencing (0070 plan, Phase 5 §5), so the
  deprecate-then-contract sequence has an owner and cannot ossify in the
  expanded-but-never-contracted state.
- Technical Notes line refs were verified against the current source during a
  pre-implementation enrichment pass; all named functions and tests exist
  unrenamed. Two naming caveats (the bare `warn!` in `cluster_key.rs`; the
  unrelated shape-validation warn in `indexer.rs`) were surfaced into Technical
  Notes so the implementer does not conflate them.
- Scope of the `frontmatter.rs:read_ref_keys` chain was resolved during
  enrichment: the `work-item:` and `ticket:` arms are both removed (`ticket:` is
  the oldest legacy shape, from migration `0001`), while the typed `target:` arm
  is deliberately retained as the current ADR-0034 key the others migrate
  toward. Including `ticket:` was an explicit owner decision; it widens the
  gate to a second `ticket:`/`ticket_id:` clause resting on migration `0001`.

## References

- Related work items: 0070 (done — ships the migration and the reader
  expand/deprecate this story contracts), 0057 (in-progress — parent epic),
  0064 (done — introduced canonical `work_item_id:` and the visualiser
  `work-item:` reads that these arms trace back to).
- ADR-0034 (typed linkage vocabulary), ADR-0033 (identity contract).
- Migration `0001` (`ticket`→`work-item` BREAKING rename, recorded in
  `CHANGELOG.md`) — the lineage the `ticket:` arm removal rests on.
