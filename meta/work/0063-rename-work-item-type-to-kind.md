---
work_item_id: "0063"
title: "Rename work-item `type:` Field to `kind:`"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
type: story
status: draft
priority: high
parent: "0057"
tags: [refactor, work-item, schema, breaking-change]
---

# 0063: Rename work-item `type:` Field to `kind:`

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Rename the work-item frontmatter field `type:` (which currently holds the semantic kind — `story | epic | task | bug | spike`) to `kind:` across templates, helpers, agent prompts, and every consumer in the codebase. This frees the `type:` slot for the artifact-type discriminator the unified schema introduces. Per epic 0057, this rename is coordinated in a single story because of its disruption surface.

## Context

The unified-schema work (epic 0057) needs every artifact type to carry a uniform `type:` field naming its artifact-type discriminator (`work-item`, `plan`, `adr`, etc.). Work-items currently overload `type:` with their semantic kind, blocking the discriminator from being applied uniformly. Renaming to `kind:` resolves the collision once and for all rather than working around it indefinitely.

The rename is the single most disruptive change in epic 0057 — it touches templates, scripts, every work-skill SKILL.md, agent prompts, and every existing work-item file. Per the epic's technical notes, all of these are coordinated in this single story.

## Requirements

- Update `templates/work-item.md` to use `kind:` in place of `type:`, with the same accepted vocabulary (`story | epic | task | bug | spike`).
- Update every skill under `skills/work/*/SKILL.md` that references the field.
- Update helpers under `skills/work/scripts/` — at minimum `work-item-read-field.sh` and `work-item-resolve-id.sh` — that read or write the field.
- Update agent prompts under `agents/` (or wherever agents are configured) that read or write the field.
- Migrate every existing work-item file under `meta/work/` — handled by the unified-schema migration (0070) rather than this story directly.

## Acceptance Criteria

- [ ] `templates/work-item.md` uses `kind:` for the semantic kind.
- [ ] No SKILL.md, helper script, or agent prompt references the work-item `type:` field as the semantic kind any more — `grep -r "type:" skills/work` returns no semantic-kind references.
- [ ] `work-item-read-field.sh` and `work-item-resolve-id.sh` accept `kind` as the field name where the semantic kind is queried.
- [ ] All work-skills' SKILL.md files describe `kind:` in their templates and instructions.
- [ ] The unified-schema migration (0070) — separately tracked — handles every existing work-item file's rename.

## Open Questions

- Are there userspace overrides (per project) that reference the work-item `type:` field and need to be migrated? If so, how are they reached from this story?
- Should the `kind:` rename ship as its own migration step, or is it bundled into the unified-schema migration (0070)?

## Dependencies

- Blocked by: 0060 (base schema ADR — decides the unified `type:` discriminator vocabulary) for full alignment; could land independently if the field rename is decoupled from the discriminator addition.
- Blocks: 0065 (template updates), 0070 (corpus migration must apply this rename to existing files).
- Related: 0057 (parent epic).

## Assumptions

- The semantic-kind vocabulary itself does not change — only the field name does.
- Userspace customisations of work-skills follow the same field-name conventions as the plugin defaults.

## Technical Notes

- The technical-notes section of 0057 enumerates the affected surface: `templates/work-item.md`, every `skills/work/*/SKILL.md`, `skills/work/scripts/work-item-read-field.sh`, `skills/work/scripts/work-item-resolve-id.sh`, agent prompts, and every work-item in `meta/work/`.
- This story does not migrate existing work-item files — that's the migration story (0070). This story prepares producers and consumers so the migration's rewrite is consumed correctly.

## Drafting Notes

- Kept this as a single story per the epic's explicit "Coordinate this rename in a single story" instruction.
- Treated the migration of existing files as out of scope here — bundled into 0070 — to avoid the rename story owning both producer changes and a corpus rewrite.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0065, 0070
