---
work_item_id: "0066"
title: "Move Review/Validation Skills' Frontmatter into Templates on Unified Schema"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: draft
priority: medium
parent: "0057"
tags: [review-skills, frontmatter, schema]
---

# 0066: Move Review/Validation Skills' Frontmatter into Templates on Unified Schema

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move the frontmatter that the review and validation skills (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) currently bake inline in their SKILL.md prose **into template files under `templates/`**, conforming to the unified base schema and typed linkage vocabulary, and rewire each skill to read its frontmatter from the template rather than emitting it inline. This brings these four producers onto the same template-based footing as every other producer (0065), so a single future schema change touches only template files.

## Context

Per 0057, four skills currently bake frontmatter field shapes directly into their SKILL.md prose rather than reading from a template under `templates/`: `review-plan`, `review-work-item`, `review-pr`, `validate-plan`. Their emitted frontmatter is therefore not updated by 0065 and needs a dedicated pass.

The epic's technical notes raised extracting these into shared template files as an *optional* simplification. **That option is now a decision**: this story moves the inline frontmatter into templates rather than merely rewriting the inline prose. Three of these artifact types have no template file today (`plan-review`, `work-item-review`, `pr-review`) and are created here; `plan-validation` already has a (body-only) `templates/validation.md`, to which 0065 adds the unified frontmatter block — this story rewires `validate-plan` to read it.

## Requirements

- Create template files under `templates/` for the three review artifact types that lack one — `plan-review`, `work-item-review`, `pr-review` — each emitting the unified base schema. (`plan-validation` reuses `templates/validation.md`, whose frontmatter block is added by 0065.)
- Each template must emit the unified base fields: `type`, `id` (own identity), `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version` (value `1` per ADR-0033). Identity values are quoted YAML strings; foreign references use `<snake_case_type>_id`.
- Apply per-artifact extras per ADR-0033: the review types carry `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass` where applicable; `plan-validation` carries `result`. Relationship-named keys (`target` — the thing reviewed/validated) come from 0061's linkage vocabulary, not the extras list.
- Rewire `review-plan`, `review-work-item`, `review-pr`, and `validate-plan` to read their frontmatter from the corresponding template rather than baking field shapes into SKILL.md prose, and to populate the field values (including `producer`, `schema_version`, `last_updated`, `last_updated_by`, `target`, `reviewer`, `verdict`, `lenses`).
- For `validate-plan` specifically: read frontmatter from `templates/validation.md` (populated by 0065) instead of the inline block currently in its SKILL.md.

## Acceptance Criteria

- [ ] Template files exist under `templates/` for `plan-review`, `work-item-review`, and `pr-review`, each emitting the unified base fields including `producer` and `schema_version: 1`.
- [ ] All four skills (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) read their frontmatter from a template file and no longer bake frontmatter field shapes into SKILL.md prose.
- [ ] Artifacts produced by the four skills carry the unified base fields plus applicable extras (`target`, `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`; `result` for plan-validation), with populated, non-placeholder values.
- [ ] Identity values in emitted frontmatter are quoted YAML strings.

## Open Questions

- Verdict-enum alignment (`REVISE` vs `REQUEST_CHANGES`) is explicitly out of scope per the epic, but should this story flag a follow-up if the prose currently uses inconsistent enum values?

## Dependencies

- Blocked by: 0060 (base schema), 0061 (linkage vocabulary), 0065 (adds the unified frontmatter block to `templates/validation.md`, which this story rewires `validate-plan` to read).
- Blocks: 0070 (corpus migration).
- Related: 0057 (parent epic), 0065 (template-based producer updates — owns the template *files*; this story owns the skill-side rewiring and the three new review templates).

## Assumptions

- The four skills' inline frontmatter is the only producer of those four artifact-type bodies. If other surfaces emit the same frontmatter, this story's scope expands.
- Verdict-enum alignment stays explicitly out of scope per the epic.

## Technical Notes

- The epic left template extraction optional; this story now mandates it (per user decision). Moving the frontmatter into templates means a future schema change touches only `templates/` files, not skill prose — the same maintenance property every other producer already has after 0065.
- `templates/validation.md` is today a body-only report template that `validate-plan` reads for the report structure while emitting frontmatter inline. 0065 adds the frontmatter block to that file; this story changes `validate-plan` to read the frontmatter from it too. The two stories therefore touch the same file from different angles (0065: template content; 0066: the skill that reads it) — hence the 0065→0066 ordering.

## Drafting Notes

- Set priority to `medium` rather than `high` because these skills can keep functioning with their current inline frontmatter until the migration runs; the high-priority dependencies are the ADRs and the corpus migration.
- Scope changed per user decision from "rewrite inline frontmatter in prose" to "move frontmatter into template files and rewire skills to read them". This resolves the prior open question (extract into templates? — yes, under `templates/`) and adds creation of the three missing review templates plus a 0065 dependency for `validation.md`.

Originally extracted from source documents without interactive enrichment; refined during 0065's review when the `validation.md` boundary surfaced.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0061, 0065, 0070
