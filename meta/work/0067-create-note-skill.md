---
id: "0067"
title: "Create `create-note` Skill"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
producer: extract-work-items
status: done
kind: story
priority: medium
parent: "work-item:0057"
derived_from: ["work-item:0057"]
relates_to: ["work-item:0060", "work-item:0061", "work-item:0065", "work-item:0070", "work-item:0057", "adr:ADR-0034", "adr:ADR-0033"]
tags: [skills, notes, accelerator-plugin]
last_updated: "2026-06-06T08:57:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
blocked_by: ["work-item:0060", "work-item:0061", "work-item:0065"]
blocks: ["work-item:0070"]
external_id: PP-89
---

# 0067: Create `create-note` Skill

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Introduce a `create-note` skill at `skills/notes/create-note/SKILL.md` that produces files under `meta/notes/` conforming to the unified frontmatter schema, together with the `templates/note.md` template the skill consumes. Notes are short-form observations or strategy snippets that don't fit the research / plan / ADR mould, and they currently have neither a creator skill nor a template. The intended user is a plugin author or contributor who wants to capture such an observation in the moment, without the ceremony of a research doc, plan, or ADR.

## Context

Per 0057, `meta/notes/` exists as an artifact category but has no creator skill and no frontmatter convention. Files there today are hand-written and free-form. The unified-schema work makes notes a first-class artifact type with the unified base schema plus a `topic` field and provenance bundle. The note template was originally slated for the template-update story (0065) but was moved here so the template and the skill that consumes it ship together.

## Requirements

- Create `templates/note.md` emitting this authoritative field set: the unified base fields (per ADR-0033 / 0060), the note-specific extras `topic` and the provenance bundle (`revision`, `repository`), and the omit-when-empty typed-linkage slots (per ADR-0034), with own identity keyed `id` (quoted), `schema_version` (bare integer), and a per-type valid-status comment. These shapes match the conventions applied to the templates in 0065 (now shipped); that conformance is a non-normative cross-check — the enumerated field set here is the pass/fail source.
- Create the skill directory `skills/notes/create-note/` with a `SKILL.md`.
- The skill writes a new file under `meta/notes/` by consuming `templates/note.md`, producing frontmatter conforming to the unified schema: base fields plus `topic` and the provenance bundle.
- Generate the output filename as `meta/notes/YYYY-MM-DD-<topic-slug>.md` — a date-prefixed kebab slug, matching the existing `meta/notes/` corpus and the `research-codebase` convention. Do **not** introduce a sequential-number allocator for notes.
- The note template carries the standard omit-when-empty typed-linkage slots. When the user names a related work-item or plan, the skill records it under `relates_to` by default, and under `parent` only when the user confirms that artifact *owns* the note (a hierarchical owner per ADR-0034); the skill does not infer ownership unprompted.
- A note never writes `source` or `derived_from` itself. Those keys belong to the extraction-origin direction (per ADR-0034) and are written by the *other* artifact when it is extracted from a note — e.g. a work item carrying `source` / `derived_from` back to the note.
- The skill interactively elicits the note's topic, body content, and any optional tags.
- The skill follows the plugin's prevailing skill conventions (allowed-tools frontmatter, conversational prompt flow, deterministic output path naming).

## Acceptance Criteria

- [ ] `templates/note.md` exists and emits the unified base fields, `topic`, the provenance bundle (`revision`, `repository`), and the omit-when-empty typed-linkage slots (per ADR-0034), with own identity keyed `id` (quoted), `schema_version` (bare integer), and a per-type valid-status comment. This enumerated field set is the authoritative checklist; conformance to the 0065 templates is a non-normative cross-check, not the pass/fail source.
- [ ] `skills/notes/create-note/SKILL.md` exists with well-formed `name` / `description` frontmatter and appears in the available-skills listing alongside sibling skills (the same enumeration surface that lists `create-work-item`), offered as `create-note`.
- [ ] Running the skill produces a new file under `meta/notes/`, generated from `templates/note.md`, with the unified base frontmatter, `topic`, and provenance bundle populated.
- [ ] Running the skill prompts for the note's topic, body content, and optional tags, and the resulting note's body and `tags` frontmatter reflect the values the user supplied — an implementation that hard-codes or skips these inputs fails this criterion.
- [ ] Running the skill writes the file to `meta/notes/YYYY-MM-DD-<topic-slug>.md`, where the date is the creation date and the slug is a kebab-case summary of the topic — matching the existing notes corpus.
- [ ] When the user names a related work-item or plan, the generated note records it under `relates_to` by default — and under `parent` only when the user confirms that artifact owns the note — using the typed-linkage `doc-type:id` ref shape per ADR-0034; unused linkage slots are omitted, not emitted empty.
- [ ] The generated note never emits `source` or `derived_from` (neither populated nor empty), even when the user names a related artifact — those keys are owned by the extracting artifact per ADR-0034.
- [ ] The skill's `SKILL.md` carries `allowed-tools` frontmatter, uses a conversational elicitation prompt flow, and derives the output path deterministically (no random suffix, no free-form path entry) — matching the prevailing pattern in `create-work-item`.
- [ ] The skill's `name` / `description` frontmatter contain the note-capture intent keywords — at minimum "note" and one of "capture" / "jot".
- [ ] Each of the test phrases "capture a note", "jot this down", and "make a note of this" routes to `create-note` on a single dispatch when evaluated against the full enabled skill set.

## Dependencies

- Blocked by: 0060 (base schema), 0061 (linkage vocabulary). This story now produces `templates/note.md` itself, so it no longer depends on 0065 for the note template.
- Blocks: 0070 (corpus migration's treatment of `meta/notes/` may interact with this skill's conventions).
- Related: 0057 (parent epic), 0065 (sibling template-update story, now **done** — the convention precedent `templates/note.md` must mirror, not merely a sibling), `create-work-item` (prevailing skill-convention precedent the skill must follow — `allowed-tools` frontmatter, conversational flow, deterministic path naming).
- Convention couplings (no work-item edge): the `notes` → `meta/notes` path key in the config/paths skill is an already-wired prerequisite for the skill's deterministic output-path naming; and the inbound `source` / `derived_from` back-references to notes are owned by extracting skills (e.g. `extract-work-items`) per ADR-0034, not by this skill.

## Assumptions

- The skill consumes `templates/note.md` — produced by this story — rather than baking the frontmatter shape inline (matches the established convention for non-review skills).
- `meta/notes/` is the canonical location for notes per the plugin's path configuration (the `notes` → `meta/notes` key is already wired in the config/paths skill, so no new path config is required).
- Notes use date-prefixed filenames (`YYYY-MM-DD-<topic-slug>.md`) rather than a sequential ID — matching every existing note and avoiding a notes allocator script. *(Resolves former open question on file-naming.)*
- Note→artifact linkage uses `relates_to` by default and `parent` only for true ownership, per the corpus-wide `parent` semantics in ADR-0034. *(Resolves former open question on linkage.)*

## Technical Notes

- The migration story (0070) must decide how to treat existing hand-written notes (skip, or add baseline frontmatter with conservative defaults). That decision interacts with this skill's conventions.
- `list-notes` / `show-note` are explicitly **out of scope** for this story, deferred as a possible follow-up. Across native meta surfaces only work items have a `list-*` skill and none have a `show-*` skill, so notes shipping create-only is consistent with research, ADRs, and plans. *(Resolves former open question on list/show parity.)*

## Drafting Notes

- Treated this as a `story` rather than `task` because it creates a new user-facing skill with interactive prompt flow, not just a documentation artifact.
- Priority `medium` because the skill is additive — nothing breaks if it ships after the schema work.
- `templates/note.md` was moved into this story from 0065 per user decision, so the note template and the skill that consumes it ship together rather than the template landing ahead of its only consumer.
- Resolved all three original open questions during enrichment: file-naming (date-prefixed slug, grounded in the existing corpus + the `research-codebase` convention + ADR-0034's own example reference), note linkage (`relates_to` default / `parent` for ownership, per ADR-0034), and list/show parity (deferred follow-up).
- `producer` recorded as `extract-work-items` to reflect the file's true extraction origin, even though this enrichment pass ran through `create-work-item`. By policy `producer` names the originating skill and is not expected to reflect later enrichment passes, so the frontmatter value and the enrichment history are not in conflict.
- 0057 is represented as both `parent` (epic ownership) and `derived_from` (extraction source), since the source spec names it in both roles.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0060, 0061, 0065, 0070
- Decisions: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` (unified base schema + note `topic` / provenance extras), `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` (`parent` / `relates_to` / `source` semantics)

## Schema Reference

The note template emits the unified base schema plus the `topic` per-type
extra and the `revision`/`repository` provenance bundle per ADR-0033, and
the `parent`/`relates_to` typed-linkage slots per ADR-0034, emitted
omit-when-empty per ADR-0040. Authoritative source: ADR-0033 and ADR-0034,
with omit-when-empty emission per ADR-0040; on any discrepancy the ADRs win
and this table should be re-synced. This table is mirror data validated by
`test-template-frontmatter.sh`, but its cross-check compares only the
template-filename cell — the `type` / `schema_version` / provenance / extras
columns are unverified by any test and must be checked by hand against the
template and the ADRs during manual verification. The note template
deliberately carries no `work_item_id` foreign-reference slot (unlike the
`codebase-research.md` / `rca.md` exemplars): ADR-0033 lists no foreign
reference for the note type — notes link via `parent`/`relates_to` — so the
omission is intentional, not an oversight.

| Template file | Artifact `type` | `schema_version` | Provenance bundle? | Per-type extras (beyond base) |
|---|---|---|---|---|
| `note.md` | `note` | 1 | yes | `topic` |
