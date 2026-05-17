---
work_item_id: "0064"
title: "Canonicalise `work_item_id` and `author` Field Names"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
type: story
status: draft
priority: high
parent: "0057"
tags: [refactor, frontmatter, schema]
---

# 0064: Canonicalise `work_item_id` and `author` Field Names

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Replace the hyphenated `work-item:` field on plan frontmatter with the canonical `work_item_id:`, and replace `researcher:` on research and RCA frontmatter with the canonical `author:`. The renames eliminate per-skill field-name inconsistencies and prepare producers for the unified-schema migration.

## Context

Per 0057, two field-name conflicts have accumulated:

- The same concept (a reference to a work-item) is spelled `work_item_id` in work-item and work-item-review frontmatter but `work-item` (hyphenated) in plan frontmatter.
- The same role (the person authoring an artifact) is spelled `author` on most artifacts but `researcher` on codebase-research and issue-research frontmatter.

Both are pure renames at the schema level. The corpus migration (0070) handles existing files; this story handles producers and consumers.

## Requirements

- Rename plan frontmatter's `work-item:` → `work_item_id:` in `templates/plan.md` and every plan-producing skill.
- Rename research and RCA frontmatter's `researcher:` → `author:` in `templates/codebase-research.md`, `templates/issue-research.md` (or equivalents), and every producing skill.
- Update any helper scripts, agent prompts, or downstream consumers that read these fields.
- Cross-reference 0060 (unified base schema) so the rename aligns with the canonical schema decision.

## Acceptance Criteria

- [ ] No template, skill, or helper references `work-item:` (hyphenated) as a work-item reference field — only `work_item_id:`.
- [ ] No template, skill, or helper references `researcher:` — only `author:`.
- [ ] The unified-schema migration (0070) — separately tracked — handles existing files carrying the old field names.
- [ ] All `work_item_id:` values remain quoted YAML strings per the identity-value shape contract.

## Open Questions

- Are there userspace template overrides that carry the old field names and need a migration path?
- Does any downstream tooling (e.g. visualiser, external sync) currently key off the old field names and need a coordinated update?

## Dependencies

- Blocked by: 0060 (base schema ADR — confirms the canonical field names).
- Blocks: 0065 (template-wide updates), 0070 (corpus migration applies these renames).
- Related: 0057 (parent epic), 0063 (the other coordinated rename — work-item `type:` → `kind:`).

## Assumptions

- The two renames are independent of each other and can ship together or separately. Bundled here for review-cost economy.
- No downstream tooling outside the plugin currently reads the old field names — the migration's blast radius is the plugin and `meta/`.

## Technical Notes

- The shape contract for `work_item_id` (quoted YAML string) is already documented in `skills/config/configure/SKILL.md`. This story extends consistent use across the plan template and any inline frontmatter generators.

## Drafting Notes

- Bundled the two renames into one story because they are pure mechanical renames with the same risk profile. If you'd rather split them — e.g. because `work_item_id` and `author` touch different reviewer surfaces — splitting is low-cost.
- Treated the migration of existing files as out of scope here (it's in 0070), matching the pattern used for 0063.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0063, 0065, 0070
