---
type: "adr"
id: "ADR-0067"
title: "Kind-Discriminated Corpus Schema and Templates"
date: "2026-09-09T12:36:29+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:0277"
relates_to: ["adr:ADR-0068", "adr:ADR-0033"]
tags: ["corpus", "frontmatter", "templates", "schema", "topic-research"]
last_updated: "2026-09-09T14:45:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0067: Kind-Discriminated Corpus Schema and Templates

**Date**: 2026-09-09
**Status**: Accepted
**Author**: Toby Clemson

## Context

The corpus frontmatter validator (`cli/corpus/src/frontmatter_validation/schema.rs`)
and the template surface key on the artifact `type` alone: one `SchemaRow` per
type, one plugin-default template per type, kept 1:1 by tests. The Topic Research
Skillset (0121) introduces a doc type, `topic-research`, whose shapes —
`manifest`, `brief`, `outline`, `finding`, `synthesis`, `report` — must share one
umbrella `type` so the visualiser registers a single library entry, yet each shape
carries different required frontmatter and a different base-`status` vocabulary.
A single flat `topic-research` row can enforce only the common fields and the
union of statuses, and `row_for`'s first-match plus the
`every_row_matches_templates_schema_tsv` test forbid expressing per-shape rules as
several same-type rows.

## Decision Drivers

- Enforce per-shape required fields and status under a single umbrella `type`.
- Keep the plugin's template safety-net covering every shape, not one.
- A general mechanism, not one special-cased for topic-research.
- Align the discriminator with the existing work-item `kind` field.
- Pave the way for per-work-item-kind templates.

## Considered Options

1. **Single flat `topic-research` row** — one row, one template. Only one shape's
   template is shape-validated (the rest become orphans), and the union
   `status_vocab` accepts a wrong per-shape status (e.g. a `draft` finding).
2. **A distinct `type` per shape** — clean per-type validation, but breaks the
   single umbrella doc type the visualiser and indexer require (0121) and
   multiplies the registration cost.
3. **A `(type, kind)` composite lookup with a type-level fallback** — extends the
   schema and template resolution to key on the pair.

## Decision

We will discriminate the corpus frontmatter schema and template resolution by a
`(type, kind)` pair, renaming the as-yet-unimplemented topic-research
discriminator field `research_kind` → `kind` (the field name work items already
use).

- `SchemaRow` and `templates-schema.tsv` gain a `kind` column. `row_for(type,
  kind)` selects the kind-specific row and falls back to the type-default row
  `(type, "")` when none exists.
- `validate_file` reads a document's `kind` and resolves the composite key. The
  existing 13 types become `(type, "")` rows and resolve exactly as before.
- Templates resolve `<type>-<kind>` when present, else `<type>` (e.g.
  `topic-research-finding`, else `topic-research`).

This is deliberately general: it later admits per-work-item-kind rows and
templates (`work-item` + `story`/`epic`/…, `work-item-<kind>` → `work-item`).

## Consequences

### Positive

- Per-shape required fields and status enforced under one umbrella type.
- Every shape's template is shape-validated; no orphan templates.
- General and forward-looking — work-item-kind templates need no further model
  change.
- The discriminator name is consistent with work items.

### Negative

- Touches the shared validation engine (`row_for` signature and callers), the TSV
  format (new column; field-count self-check `7`→`8`), and the `cargo-public-api`
  snapshot — paths all 13 existing types run through.
- Each new `(type, kind)` row's template filename must also appear in the
  `## Schema Reference` tables of work items 0065/0066/0067, or the cross-check
  fails.
- More rows and templates to maintain per multi-shape type.

### Neutral

- Existing types are unaffected functionally (they resolve via `(type, "")`).
- First implemented by work item 0277.

## References

- `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md` — resolving research
- `meta/work/0121-topic-research-skillset.md` — artifact contract + umbrella doc type
- `meta/work/0277-single-round-web-research-engine.md` — first implementer
- `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md` — sibling decision
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — the unified schema this extends; defines `kind` as a work-item extra
