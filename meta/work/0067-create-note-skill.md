---
work_item_id: "0067"
title: "Create `create-note` Skill"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: draft
priority: medium
parent: "0057"
tags: [skills, notes, accelerator-plugin]
---

# 0067: Create `create-note` Skill

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Introduce a `create-note` skill at `skills/notes/create-note/SKILL.md` that produces files under `meta/notes/` conforming to the unified frontmatter schema. Notes are short-form observations or strategy snippets that don't fit the research / plan / ADR mould, and they currently have no creator skill.

## Context

Per 0057, `meta/notes/` exists as an artifact category but has no creator skill and no frontmatter convention. Files there today are hand-written and free-form. The unified-schema work makes notes a first-class artifact type with the unified base schema plus a `topic` field and provenance bundle.

## Requirements

- Create the skill directory `skills/notes/create-note/` with a `SKILL.md`.
- The skill writes a new file under `meta/notes/` conforming to the unified schema: base fields plus `topic` and provenance bundle.
- The skill interactively elicits the note's topic, body content, and any optional tags.
- The skill follows the plugin's prevailing skill conventions (allowed-tools frontmatter, conversational prompt flow, deterministic output path naming).

## Acceptance Criteria

- [ ] `skills/notes/create-note/SKILL.md` exists and is discoverable via the skill registry.
- [ ] Running the skill produces a new file under `meta/notes/` with the unified base frontmatter, `topic`, and provenance bundle populated.
- [ ] The skill follows existing skill-creation conventions (see `create-work-item` for prevailing pattern).
- [ ] The skill is named and described in a way that the skill router can trigger it on intent like "capture a note", "jot this down".

## Open Questions

- What is the file-naming convention for notes (date-prefixed slug? sequential number?). The unified-schema work decides identity field shape but not necessarily file-naming.
- Should notes carry a `parent` linkage to a parent work-item or plan they relate to, or is `relates_to` enough?
- Is there a corresponding `list-notes` / `show-note` skill needed for parity with the work and research surfaces? (Possibly a follow-up.)

## Dependencies

- Blocked by: 0060 (base schema), 0061 (linkage vocabulary), 0065 (template for `note` — produces `templates/note.md` that this skill reads).
- Blocks: 0070 (corpus migration's treatment of `meta/notes/` may interact with this skill's conventions).
- Related: 0057 (parent epic).

## Assumptions

- The skill consumes `templates/note.md` produced by 0065 rather than baking the frontmatter shape inline (matches the established convention for non-review skills).
- `meta/notes/` is the canonical location for notes per the plugin's path configuration.

## Technical Notes

- The migration story (0070) must decide how to treat existing hand-written notes (skip, or add baseline frontmatter with conservative defaults). That decision interacts with this skill's conventions.

## Drafting Notes

- Treated this as a `story` rather than `task` because it creates a new user-facing skill with interactive prompt flow, not just a documentation artifact.
- Priority `medium` because the skill is additive — nothing breaks if it ships after the schema work.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0061, 0065, 0070
