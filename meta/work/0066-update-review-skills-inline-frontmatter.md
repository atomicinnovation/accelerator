---
work_item_id: "0066"
title: "Update Review Skills' Inline Frontmatter Generators to Unified Schema"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: draft
priority: medium
parent: "0057"
tags: [review-skills, frontmatter, schema]
---

# 0066: Update Review Skills' Inline Frontmatter Generators to Unified Schema

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Update the review and validation skills (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) so the frontmatter they emit inline in their SKILL.md prose matches the unified base schema and typed linkage vocabulary. These skills emit frontmatter without going through a template file, so they cannot be reached by the template-update story (0065).

## Context

Per 0057, four skills currently bake frontmatter field shapes directly into their SKILL.md prose rather than reading from a template under `templates/`: `review-plan`, `review-work-item`, `review-pr`, `validate-plan`. Their emitted frontmatter is therefore not updated by 0065 and needs a dedicated pass.

The epic's technical notes raise the option of extracting these into shared template files as a future simplification, but does not mandate it.

## Requirements

- Update `skills/.../review-plan/SKILL.md`, `skills/.../review-work-item/SKILL.md`, `skills/.../review-pr/SKILL.md`, and `skills/.../validate-plan/SKILL.md` so the inline frontmatter they describe matches the unified schema.
- Each emitted frontmatter must include the unified base fields: `type`, identity, `title`, `date`, `author`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`.
- Include per-artifact extras per 0057: `target` (the thing reviewed), `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass` where applicable.
- Decide (or defer) whether to extract the inline generators into shared template files — the epic explicitly leaves this optional.

## Acceptance Criteria

- [ ] Each of the four review/validation skills emits frontmatter matching the unified schema.
- [ ] The skills' SKILL.md prose explicitly documents each emitted field — no implicit fields.
- [ ] `target`, `reviewer`, `verdict`, and `lenses` are populated by the skills where applicable.
- [ ] Identity values in emitted frontmatter are quoted YAML strings.

## Open Questions

- Should the inline frontmatter be extracted into shared template files (the epic's optional simplification)? If so, where do those templates live?
- Verdict-enum alignment (`REVISE` vs `REQUEST_CHANGES`) is explicitly out of scope per the epic, but should this story flag a follow-up if the prose currently uses inconsistent enum values?

## Dependencies

- Blocked by: 0060 (base schema), 0061 (linkage vocabulary).
- Blocks: 0070 (corpus migration).
- Related: 0057 (parent epic), 0065 (template-based producer updates).

## Assumptions

- The four skills' inline frontmatter is the only producer of those four artifact-type bodies. If other surfaces emit the same frontmatter, this story's scope expands.
- Verdict-enum alignment stays explicitly out of scope per the epic.

## Technical Notes

- Per the epic's technical notes, extracting these into shared template files is left as an option for the implementing story — not mandated.

## Drafting Notes

- Set priority to `medium` rather than `high` because these skills can keep functioning with their current inline frontmatter until the migration runs; the high-priority dependencies are the ADRs and the corpus migration.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0061, 0065, 0070
