---
type: adr
id: "ADR-0042"
title: "Reconciling Pre-Schema Status Values"
date: "2026-06-08T00:23:02+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
parent: "work-item:0070"
relates_to: ["adr:ADR-0033", "work-item:0070", "plan:2026-06-07-0070-meta-corpus-unified-schema-migration"]
tags: [migration, frontmatter, schema, status, vocabulary]
last_updated: "2026-06-08T09:33:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0042: Reconciling Pre-Schema Status Values

**Date**: 2026-06-08
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0033 gave each artifact type a deliberately tight `status` vocabulary
(recorded in `scripts/templates-schema.tsv`). The existing `meta/` corpus,
written before that vocabulary was unified, carries `status:` values that
predate it. The `0007` corpus migration's exhaustive type×vocab scan found ~56
nonconforming values:

- **plans** (vocab `draft|ready|in-progress|done`): `accepted` (21), `complete`
  (16), `approved` (7), `implemented` (6), `reviewed` (2), `revised` (1),
  `final` (1)
- **plan-review** (vocab `complete`): `accepted` (1)
- **design-gap** (vocab `draft`): `accepted` (1)
- **design-inventory** (vocab `draft`): `superseded` (2)

All other types already conform. The migration normalises base-field values that
predate the schema rather than leaving them for the validator to reject, so it
needs a recorded rule for what each legacy status becomes. This is a
schema-vocabulary decision in ADR-0033's domain, not a migration-mechanics detail.

## Decision Drivers

- The unified vocabularies are intentionally tight; sprawl erodes their value.
- The migration must be deterministic and produce a corpus that passes the
  unified-schema validator (story 0070, AC-1).
- Some legacy values are redundant synonyms for a terminal state; others name a
  genuinely distinct lifecycle state the tight vocab simply omitted.

## Considered Options

1. **Collapse synonyms to canonical; widen a vocab only for genuinely distinct
   states** — map redundant terminal values onto each type's canonical value, and
   add a vocab entry only where a value carries lifecycle meaning the vocab lacks.
2. **Widen every vocab to admit all observed legacy values** — no rewrites; every
   historical value becomes first-class.
3. **Leave status untouched and relax the validator** — exempt migrated artifacts
   from the `status_vocab` check.

## Decision

We will take **Option 1**. The migration applies a single-sourced legacy→canonical
map (`scripts/status-legacy-map.tsv`, `type⇥legacy⇥canonical`), with the invariant
that every map target is a literal in the matched type's `status_vocab`:

- **plan**: `accepted` / `complete` / `implemented` / `final` / `revised` →
  `done` (realised states); `approved` / `reviewed` → `ready` (signed-off but
  not yet realised — distinct from done).
- **plan-review**: `accepted` → `complete`.
- **design-gap**: widen `status_vocab` to admit `accepted` (an acknowledged gap
  is a genuine state); no rewrite.
- **design-inventory**: widen `status_vocab` to admit `superseded` (a genuine
  inventory-lifecycle state); no rewrite.

Any status value not covered by the map and not in the (possibly widened) vocab is
a migration error (`0007-DIVERGE[unmapped-status]`), not a silent pass-through.

## Consequences

### Positive

- The plan vocabulary stays tight; the corpus passes the unified-schema validator.
- The map is deterministic and single-sourced, so the migration and the validator
  cannot disagree on what a status may be.
- Genuinely distinct states (`design-gap accepted`, `design-inventory superseded`)
  are preserved rather than flattened.

### Negative

- Collapsing `accepted`/`implemented`/`complete`/`final`/`revised` to `done`
  loses the historical distinction between those realised plan states.
  (`approved`/`reviewed` are kept distinct from `done` by mapping to `ready`,
  preserving the signed-off-but-not-realised meaning.)
- A future legacy value not in the map fails the migration loudly and requires a
  map update (by design, but it is friction).

### Neutral

- Dynamically-aggregated visualiser facets lose the disappeared legacy plan-status
  options; this is the intended outcome, not a regression.
- The widenings (`design-gap` +`accepted`, `design-inventory` +`superseded`) edit
  two `templates-schema.tsv` rows.

## References

- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — the unified base
  schema and per-type vocabularies this reconciles against
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — the owning story
- `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md` — the
  migration plan that consumes this decision
- `scripts/templates-schema.tsv` — per-type `status_vocab` source
