---
type: work-item
id: "0093"
title: "Extend Templates With Typed-Linkage Slots"
date: "2026-05-31T12:13:53+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: story
priority: medium
parent: "0057"
external_id: ""
tags: [templates, frontmatter, schema, linkage]
last_updated: "2026-05-31T12:13:53+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0093: Extend Templates With Typed-Linkage Slots

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Extend every artifact template under `templates/` with empty optional slots for the typed-linkage keys defined in **ADR-0034** that the artifact type can legitimately carry, so newly-created artifacts can express cross-artifact links at draft time and downstream consumers (the corpus migration in 0070, the future visualiser-graph epic) read the keys from a stable, documented template surface rather than inserting them ad hoc.

## Context

ADR-0034 (produced by 0061) defines the typed-linkage vocabulary: `parent`, `supersedes`/`superseded_by`, `blocks`/`blocked_by`, `target`, `derived_from`, `relates_to`, `source`, plus design-gap's type-specific `current_inventory`/`target_inventory`. The ADR also pins a type-pair semantic table enumerating which linkage edges each artifact type can legitimately participate in (e.g. `plan derived_from codebase-research`, `work-item source note`, `work-item blocks work-item`).

Story 0065 ("Update All Artifact Templates to Unified Schema") only carried forward linkage keys that were *already* present on legacy templates plus a small handful added incidentally during the rewrite:

- `parent` on work-item & plan (already present)
- `supersedes` on adr (reshape from single scalar to a list)
- `target` on validation (added new)
- `current_inventory` / `target_inventory` on design-gap (already present)

Its requirements §35 explicitly notes that the broader linkage vocabulary "is applied per that [ADR-0034] vocabulary, not this list" — but no story owns adding the remaining ADR-0034 keys to the templates. Story 0066 only adds linkage keys to the new review templates it creates (specifically `target`). Story 0070 (corpus migration) infers links from prose and writes them to existing artifacts but is not responsible for amending templates.

The 0057 epic's AC #114 ("Typed linkage keys (`parent`, `supersedes`/`superseded_by`, `blocks`/`blocked_by`, `target`, `derived_from`, `relates_to`, `source`) are documented and used where they apply") therefore hangs without a clear template-side owner. This story closes that gap.

## Requirements

- Audit each in-scope template against ADR-0034's type-pair semantic table and add an empty optional slot for every linkage key the type can legitimately carry. Slots are emitted present-but-empty: `<key>: ""` for single-ref keys and `<key>: []` for list-cardinality keys, with an inline comment naming the typed-linkage shape (e.g. `# typed-linkage list: ["work-item:NNNN", ...] or []`).
- Per the type-pair table, expected additions are at minimum:
  - **work-item**: `blocks: []`, `blocked_by: []`, `derived_from: []`, `relates_to: []`, `source: ""` (already has `parent`).
  - **plan**: `blocks: []`, `blocked_by: []`, `derived_from: []`, `relates_to: []` (already has `parent`).
  - **adr**: `superseded_by: ""`, `relates_to: []`, `parent: ""` (already has `supersedes`).
  - **codebase-research**, **rca**: `parent: ""`, `relates_to: []`.
  - **pr-description**: `relates_to: []`.
  - **design-inventory**, **design-gap**: `relates_to: []`. (design-gap keeps its type-specific `current_inventory`/`target_inventory`.)
  - **validation**: already has `target`; add `relates_to: []` if applicable.
  The implementer reconciles this list against ADR-0034's type-pair table at implementation time and surfaces any divergence.
- Update each consuming SKILL.md so the canonical Populate-frontmatter step lists every new slot alongside the existing fields, with guidance on when the producer should fill the slot versus leave it empty. Empty slots are the default — producers fill them only when the link is explicit at draft time.
- Extend `scripts/templates-schema.tsv` (the machine-readable mirror used by `scripts/test-template-frontmatter.sh`) with a new column or extras-list entries for the typed-linkage keys per template, so the template-shape test asserts each slot is present in the right templates.
- Extend `scripts/test-template-frontmatter.sh` to assert (a) every expected linkage slot is present on the right template, (b) value shape matches the key's cardinality (single-ref keys default to `""`, list keys default to `[]`), and (c) the inline comment names the typed-linkage form.
- No corpus migration. Existing artifacts under `meta/` keep their current shape; populating the new slots on legacy artifacts is 0070's territory (it already infers links from prose).
- No new ADR. ADR-0034 is the authority; this story implements its template-side projection.

## Acceptance Criteria

- [ ] Every in-scope template (the nine frontmatter-bearing templates updated by 0065 plus the three review templates created by 0066) carries the typed-linkage slots its row in ADR-0034's type-pair table justifies, present-but-empty.
- [ ] `scripts/test-template-frontmatter.sh` passes with new per-template assertions for each linkage slot's presence, default value shape, and comment form.
- [ ] Each consuming SKILL.md's Populate-frontmatter step names every new slot, with a one-line note on when to fill it (typically: "leave empty unless the link is explicit at draft time").
- [ ] No template emits a linkage key that ADR-0034's type-pair table does not justify for that source type.
- [ ] `templates/design-gap.md` retains its type-specific `current_inventory` / `target_inventory` keys verbatim; they are not folded into the generic vocabulary (per ADR-0034 §Design-gap inventory keys).

## Open Questions

- Should `parent` be added universally (per ADR-0034: "Corpus-wide — any artifact type may carry it") or only on types the type-pair table explicitly lists? The plan should pick one and apply it uniformly, with the rationale recorded.
- Inverse keys (`superseded_by`, `blocked_by`): ADR-0034 §"Single key sufficient for bidirectional pairs" says writing either side is sufficient. Should the template expose both inverse slots (and let producers pick), or only the canonical side (`supersedes`, `blocks`)? Default recommendation: expose both, with a comment noting producers SHOULD prefer the canonical side.
- Should this story land before or after 0066? 0066 creates the three review templates; if 0093 lands first, 0066 inherits the slot convention. If 0066 lands first, 0093 has to amend the review templates too. Recommendation: 0093 lands after 0066 so it can sweep the full template surface in one pass.

## Dependencies

- Blocked by: 0061 (ADR-0034 typed-linkage vocabulary), 0065 (unified-schema templates — this story extends them), 0066 (creates the three review templates this story also extends — see Open Questions).
- Blocks: 0070 (corpus migration's link-inference can target stable template slots rather than inserting undocumented keys).
- Related: 0057 (parent epic — closes AC #114's template-side gap).

## Assumptions

- ADR-0034's type-pair table is the authoritative list of which linkage edges each artifact type can carry. New edges added to the ADR in future trigger a template-side update, but this story takes the table as currently written.
- Consumers (the visualiser, 0070's link inference, future render layers) read linkage keys from the unified base + linkage slots, not from per-type per-key heuristics. Keys absent from a template's frontmatter are treated as absent edges, not missing data.
- Producers populate slots only when the link is explicit at draft time; speculative or inferred links remain in body prose and are picked up by 0070's inference pass.

## Technical Notes

- The template-shape test (`scripts/test-template-frontmatter.sh`) already loads a per-template TSV (`scripts/templates-schema.tsv`) with extras assertions. The cleanest extension is to add an optional seventh column for typed-linkage keys (space-separated) and assert each is present, value-shaped per cardinality, and comment-formed correctly. Cardinality lookup can come from a small in-script map keyed by linkage-key name.
- The canonical Populate-frontmatter snippet documented in 0065's plan (Implementation Approach §Canonical persistence-step prose snippet) is the right place to slot the linkage-key guidance. Each per-skill phase should extend its snippet instance with the new bullets, keeping the snippet's shape uniform.

## Drafting Notes

- Drafted as a follow-up after 0065's pass-3 review: the typed-linkage application surface was found to be split inconsistently across 0061, 0065, 0066, and 0070, with no story owning the empty-slot template extension. The 0057 epic's AC #114 is the ambiguous line ("documented and used *where they apply*") — this story interprets "used" as "templates expose empty slots so producers and migration can fill them".
- Priority is `medium` rather than `high` because newly-created artifacts can still link via body prose today and 0070's inference will pick those up; the cost of waiting is reader convenience and visualiser-graph readability, not a hard blocker.
- Sized as a story rather than a task because the per-template audit against ADR-0034's type-pair table and the test-extension work are non-trivial; a spike could precede if the type-pair coverage turns out to be larger than expected.

## References

- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` (closes AC #114's template-side gap)
- Authoritative linkage ADR: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- Predecessor: `meta/work/0065-update-artifact-templates-to-unified-schema.md` (added the unified base; this story adds the linkage slots)
- Related: `meta/work/0066-update-review-skills-inline-frontmatter.md` (creates the three review templates this story also extends)
- Related: `meta/work/0070-...` (corpus migration; consumes the new slots as targets for link inference)
