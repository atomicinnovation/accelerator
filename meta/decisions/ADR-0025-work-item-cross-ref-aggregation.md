---
adr_id: ADR-0025
date: "2026-05-05T00:00:00+01:00"
author: Toby Clemson
status: proposed
tags: [visualiser, cross-references, indexer]
---

# ADR-0025: Multi-field Work-item Cross-reference Aggregation

**Date**: 2026-05-05
**Status**: Proposed
**Author**: Toby Clemson

## Context

Work-item documents are interrelated: plans reference the work item they
implement (`work-item:`), child stories declare their parent epic
(`parent:`), and related work is cross-referenced (`related:`). The
visualiser needs to surface these relationships in both directions — from a
work item to the documents that reference it, and from a document to the
work items it mentions.

ADR-0017 established the precedent for the review-to-plan relationship:
a separate in-memory reverse index keyed by target path, populated at scan
time. This ADR extends that pattern to cover multi-field work-item
cross-references and generalises the `referencedBy` composition in the
`GET /api/related/{path}` handler.

## Decision Drivers

- `work-item:`, `parent:`, and `related:` all carry the same semantic weight:
  they are cross-references to work items. Treating them as separate
  aggregation problems would duplicate logic and diverge over time.
- Frontmatter is user-authored YAML; each field may be absent, a scalar,
  an array, null, or an integer (YAML auto-detects types). The reader must
  degrade gracefully for every combination.
- Canonicalisation must be config-aware: a bare `42` means `"0042"` under
  the default pattern and `"PROJ-0042"` under a project-prefixed pattern.
- A work item must never appear in its own `referencedBy` (self-reference
  via `parent: 0001` on work item 0001 is silently dropped).
- Two-way cycles (`A.parent=B`, `B.parent=A`) must be handled without
  infinite loops or duplicate entries.
- Lock-ordering invariants established by ADR-0017 must be preserved; the
  new index must not introduce deadlock risk.

## Considered Options

1. **Single aggregated field** — merge all three keys into one
   `work_item_refs` vec in `read_ref_keys`; canonicalise and index together.
2. **Per-field separate indexes** — maintain three separate reverse indexes
   (`by_work_item`, `by_parent`, `by_related`), merge at query time.
3. **Embed refs in IndexEntry wire format** — store the resolved cross-refs
   directly in each `IndexEntry` rather than in a secondary index.

## Decision

We will use **Option 1**: aggregate all three fields into a single
`work_item_refs` vec at parse time, then build one reverse index
(`work_item_refs_by_target`) keyed on canonical work-item ID.

### Field aggregation rules

`frontmatter::read_ref_keys` reads, in order:
1. `work-item:` wins over `ticket:` for the primary reference (scalar only;
   `ticket:` is the legacy form, kept for backwards compatibility).
2. `parent:` — scalar or array; all values collected.
3. `related:` — scalar or array; all values collected.

Each value is extracted as a raw string regardless of its YAML type: integer
`7`, string `"7"`, and string `"0007"` all yield the same raw ref `"7"`,
`"7"`, `"0007"` respectively.

### Canonicalisation

`canonicalise_refs(raw: Vec<String>, cfg: &WorkItemConfig) -> Vec<String>`:

1. **Bare numeric** (matches `^\d+$`) under default pattern: zero-pad to the
   configured width. `"7"` → `"0007"` under `{number:04d}`.
2. **Bare numeric** under project-prefixed pattern with `default_project_code`:
   zero-pad and prefix. `"7"` → `"PROJ-0007"` under `{project}-{number:04d}`
   with `default_project_code=PROJ`.
3. **Already project-prefixed** (matches `^[A-Za-z][A-Za-z0-9]*-\d+$`):
   pass through verbatim. `"PROJ-0042"` → `"PROJ-0042"`.
4. **Everything else** (non-numeric without a project prefix, malformed,
   empty): silently dropped. The function never panics.

Deduplication is applied after canonicalisation so `work-item: 0007` and
`parent: 7` under a `{number:04d}` pattern produce one entry, not two.

### Self-reference filter

When building the reverse index, any ref whose canonical ID equals the
source document's own `work_item_id` is skipped. This prevents a work item
from appearing in its own `referencedBy`. Two-way cycles are safe because
the filter is per-document: `A.parent=B` does not prevent `B` from appearing
in `A`'s `referencedBy`, nor `A` from appearing in `B`'s.

### Reverse index lifecycle

`work_item_refs_by_target: Arc<RwLock<HashMap<String, BTreeSet<PathBuf>>>>`
is populated at initial scan and maintained incrementally on file events
(`refresh_one`, `remove`). Lock acquisition follows the existing invariant:
entries → adr → work_item → reviews_by_target → work_item_refs_by_target,
preventing deadlock.

### Composition with plan-review reverse index (ADR-0017)

`GET /api/related/{path}` builds `declared_inbound` by merging:
1. `reviews_by_target(entry.path)` — plan-reviews whose `target:` resolves
   to this document (ADR-0017 path-keyed index).
2. `work_item_refs_by_id(entry.work_item_id)` — documents whose
   `work-item:`/`parent:`/`related:` canonicalises to this work item's ID.

Deduplication by path prevents an entry appearing twice when both conditions
are true. The two indexes remain separate in memory — merging is a query-time
operation, preserving the lock-ordering invariant and keeping each index
independently testable.

### Permanent dual-schema tolerance

Legacy work-item files without `work-item:`, `parent:`, or `related:` make
no contribution to the reverse index and are not affected by this change.
Files with `status: proposed` (outside any configured column set) remain
visible in the "Other" swimlane; their `IndexEntry.workItemId` is derived
from the filename, not from frontmatter, so ID resolution is unaffected by
legacy frontmatter schema. This is a permanent tolerance contract — no
per-file migration is planned.

## Consequences

### Positive

- All three cross-reference fields are visible in the "Related artifacts"
  aside without additional per-field UI work.
- A single canonical-ID-keyed index is simpler to maintain and query than
  three path-keyed indexes.
- Self-reference and two-way cycles are handled without caller awareness.
- The composition with ADR-0017's review index is additive: existing
  plan-review `referencedBy` behaviour is unchanged.

### Negative

- Cross-refs targeting a canonical ID that doesn't exist in `work_item_by_id`
  are silently dropped at query time (the index entry exists, but
  `entries.get(path)` returns None). This is preferable to surfacing broken
  refs but means stale refs are invisible.
- Pattern changes after boot leave the canonical IDs in the index stale
  (e.g. `"PROJ-0042"` if the pattern is later changed to numeric-only).
  A restart is required; documented as a known operator concern.

### Neutral

- Bare wiki-link cross-refs (`[[NNNN]]`) are not aggregated by this feature;
  they require a separate parser extension.
- The `work_item_refs` field on `IndexEntry` stores raw strings (pre-
  canonicalisation) so the wire format is stable across pattern changes.

## References

- `meta/decisions/ADR-0017-configuration-extension-points.md` — plan-review
  reverse-index precedent
- `skills/visualisation/visualise/server/src/frontmatter.rs` — `read_ref_keys`
- `skills/visualisation/visualise/server/src/indexer.rs` — `canonicalise_refs`,
  `work_item_refs_by_target`, `work_item_refs_by_id`, `declared_outbound`
- `skills/visualisation/visualise/server/src/api/related.rs` — `declared_inbound` composition
- `meta/decisions/ADR-0024-visualiser-kanban-column-config.md` — companion ADR
