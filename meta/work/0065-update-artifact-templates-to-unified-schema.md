---
work_item_id: "0065"
title: "Update All Artifact Templates to Unified Schema"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: draft
priority: high
parent: "0057"
tags: [templates, frontmatter, schema]
---

# 0065: Update All Artifact Templates to Unified Schema

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Rewrite every artifact template under `templates/` so it emits the unified base frontmatter schema decided by 0060 and the typed linkage vocabulary decided by 0061. This is the producer-side equivalent of the corpus migration: new artifacts created from these templates are born unified.

## Context

Per 0057, the Accelerator plugin produces twelve distinct artifact types (work-items, plans, plan-validations, plan-reviews, work-item-reviews, pr-reviews, pr-descriptions, ADRs, codebase-research, issue-research, design-inventories, design-gaps) plus the new `note` type. Each currently has its own template with idiosyncratic field names and shapes. With the schema and linkage ADRs decided, the templates must be brought into line.

## Requirements

- Update every template file under `templates/` to emit the unified base fields: `type`, identity, `title`, `date` (quoted ISO UTC), `author`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`.
- Add the provenance bundle (`revision`, `repository`) to code-state-anchored templates: plan, codebase-research, issue-research, design-inventory, pr-description. Remove `git_commit` and `branch` where present.
- Apply per-artifact-type extras per 0057's "Per-artifact extras" section: e.g. `kind` on work-item, `target` on plan and reviews, `pr_url` / `pr_number` on pr-description, `adr_id` / `supersedes` / `decision_makers` on ADR, `current_inventory` / `target_inventory` on design-gap.
- Ensure all identity values are quoted YAML strings (e.g. `adr_id: "0042"`).

## Acceptance Criteria

- [ ] Every template under `templates/` emits the unified base fields.
- [ ] Templates for code-state-anchored artifacts emit the provenance bundle and no longer emit `git_commit` or `branch`.
- [ ] Each template's per-artifact extras match the lists in 0057's "Per-artifact extras" section.
- [ ] All identity-value fields in templates are quoted (e.g. `adr_id: "NNNN"`, not bare integers).
- [ ] Templates include `schema_version` with the value decided in 0060 for that artifact type.

## Open Questions

- Are there templates that live outside `templates/` (e.g. inline in SKILL.md prose) that this story should also cover, or is that strictly the inline-frontmatter-generators story (0066)?
- For artifact types whose status vocabulary is unaffected (per the epic's out-of-scope decision on vocabulary unification), does the template still need a comment explaining the per-type vocabulary?

## Dependencies

- Blocked by: 0060 (base schema), 0061 (linkage vocabulary), 0063 (work-item `kind:` rename — touches `templates/work-item.md`), 0064 (`work_item_id` / `author` canonicalisation — touches plan/research templates).
- Blocks: 0070 (corpus migration), 0067 (note-creator skill needs the new note template).
- Related: 0057 (parent epic), 0066 (inline frontmatter generators in review skills).

## Assumptions

- The `templates/` directory is the single source of truth for templates that are not baked inline into SKILL.md files. Inline-only generators are 0066's scope.
- A new `templates/note.md` is part of this story so 0067's `create-note` skill has a template to consume.

## Technical Notes

- `last_updated` is only refreshed by skills that touch the artifact — manual editor edits don't automatically update it. Templates can document this contract in a comment.
- The identity-value shape contract (quoted strings) already applies to `work_item_id` per `skills/config/configure/SKILL.md`; 0060 extends it to `adr_id` and any new identity fields.

## Drafting Notes

- Treated the new `templates/note.md` as in-scope here so 0067 (create-note skill) has its template ready. If you'd rather pair note-template + note-skill into 0067, this story drops `note.md`.
- Kept inline frontmatter generators (review-plan, review-work-item, review-pr, validate-plan) explicitly out of scope — they're 0066's responsibility.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0061, 0063, 0064, 0066, 0067, 0070
