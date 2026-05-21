---
work_item_id: "0070"
title: "Ship `meta/` Corpus Unified-Schema Migration"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: draft
priority: high
parent: "0057"
tags: [migration, frontmatter, schema, dogfood]
---

# 0070: Ship `meta/` Corpus Unified-Schema Migration

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Ship the numbered Accelerator migration that rewrites every existing artifact under `meta/` to the unified frontmatter schema and populates structured linkage frontmatter where free-form body sections allow confident inference. Dogfood the migration against this repo's own `meta/` corpus and fix any gaps surfaced.

## Context

Per 0057, frontmatter has evolved per-skill into an inconsistent state — field-name conflicts, shape inconsistencies, missing discriminators, absent `schema_version`. Producers will be brought into line by 0063 / 0064 / 0065 / 0066 / 0067, but existing artifacts under `meta/` need a corresponding rewrite. This story owns that rewrite.

The migration is numbered after the latest applied (current head determined at implementation time) and follows the existing migration-framework conventions, possibly extended by 0069 if interactive validation hooks are added.

## Requirements

- Author a new numbered migration under the migration framework that:
  - (Plan `work-item:` → `work_item_id:` and research/RCA
    `researcher:` → `author:` renames are **owned by migration 0006**,
    authored under story 0064. The visualiser server's transitional
    `work-item:` fallback ships with 0064 and **must be removed by
    this story** in the same release that closes out 0070 — by then
    every userspace repo will have run `/accelerator:migrate` at
    least once.)
  - (Work-item `type:` → `kind:` rename is **owned by migration
    0005**, authored under story 0063. This migration must not
    duplicate that rewrite — 0005 has already migrated the corpus by
    the time this migration runs in any repo.)
  - Adds the unified base fields (`type`, identity, `title`, `date`, `author`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`) with sensible defaults where missing.
  - Adds the provenance bundle (`revision`, `repository`) to code-state-anchored artifacts and removes `git_commit` / `branch`.
  - Adds per-artifact extras per 0057.
  - Parses `## Related Documents` / `## References` / `## Related Research` body sections and populates typed linkage frontmatter where confident.
  - Records `schema_version` per artifact type.
  - Surfaces uncertain inferences either interactively (if 0069 ships) or via a post-run report (otherwise).
- Decide treatment of existing hand-written notes under `meta/notes/` (skip, or add baseline frontmatter with conservative defaults; the epic's technical notes flag `author: <unknown>` as a possible default).
- Dogfood against this repo's own corpus, then fix any gaps surfaced.

## Acceptance Criteria

- [ ] The migration applies cleanly to this repo's `meta/` corpus, leaving every artifact conforming to the unified schema.
- [ ] Plan files already have `work_item_id` (quoted) and research/RCA files already have `author` — guaranteed by migration 0006 from story 0064. Work-item files already have `kind:` — guaranteed by migration 0005 from story 0063.
- [ ] The visualiser server's transitional `work-item:` fallback (introduced by 0064 in `frontmatter.rs:read_ref_keys`) has been removed, along with the test that pinned it.
- [ ] Code-state-anchored artifacts have `revision` + `repository`; no `git_commit` or `branch` remains.
- [ ] `schema_version` is set per artifact type.
- [ ] Typed linkage frontmatter is populated from body sections where the migration was confident; uncertain cases are surfaced either via interactive prompts or a post-run report.
- [ ] Existing `meta/notes/` files are either skipped or carry baseline frontmatter per the in-scope decision.
- [ ] Re-running the migration is a no-op against an already-migrated corpus (idempotency).

## Open Questions

- How are existing `meta/notes/` files treated (skip / add minimal frontmatter / require user confirmation per file)?
- For artifacts whose `last_updated` is being set for the first time, what value is used — `date` of the original artifact, or `now()` of the migration run?
- When a body-section reference is ambiguous between two artifact types (e.g. a number that could be a work-item or an ADR), what is the disambiguation rule?

## Dependencies

- Blocked by: 0060 (base schema), 0061 (linkage vocabulary), 0062 (migration-strategy ADR), 0063 (work-item `kind:` rename), 0064 (`work_item_id` / `author` canonicalisation), 0065 (templates updated), 0066 (review-skill inline frontmatter updated). Possibly blocked by 0068 (spike) and 0069 (interactive hooks) depending on chosen strategy.
- Blocks: future visualiser-graph epic (which consumes the structured linkages this migration writes).
- Related: 0057 (parent epic), 0056 (precedent for frontmatter-aware migration).

## Assumptions

- VCS revert remains the migration's safety net — no inverse migration is built.
- Producer-side updates (0063–0067) land before or with this migration so new artifacts created during/after migration are already unified.
- The `specs/` and `global/` directories are out of scope (deferred per 0057's assumptions).

## Technical Notes

- The migration is numbered after the latest applied at implementation time — do not hard-code the number in this work item.
- The body-section parser is shared with 0068's spike prototype where practical, or rebuilt with the spike's findings encoded.

## Drafting Notes

- Listed every preceding story as a blocker because the migration is the integration point for the whole epic. Some of those dependencies might be relaxable in practice (e.g. producer updates and migration could ship in the same commit), but the work item captures the conservative order.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0056, 0057, 0060, 0061, 0062, 0063, 0064, 0065, 0066, 0067, 0068, 0069
