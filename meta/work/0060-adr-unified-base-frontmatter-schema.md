---
id: "0060"
title: "ADR: Unified Base Frontmatter Schema"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: task
status: done
priority: high
parent: "work-item:0057"
tags: [adr, frontmatter, schema, accelerator-plugin]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
blocks: ["work-item:0057"]
relates_to: ["work-item:0021", "work-item:0022", "work-item:0023", "work-item:0057"]
external_id: PP-82
---

# 0060: ADR: Unified Base Frontmatter Schema

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Produce the source-of-truth ADR that defines the unified base frontmatter schema every Accelerator artifact type must carry, including the provenance bundle for code-state-anchored artifacts and the per-artifact-type extras vocabulary. This ADR is the foundation the rest of epic 0057 builds on.

## Context

Per 0057, frontmatter schemas evolved per-skill without a unifying contract, producing field-name inconsistencies (`work_item_id` vs `work-item`), shape inconsistencies (quoting, date formats), missing discriminators (only some artifacts carry `type:`), and absent fields like `schema_version`. A single source-of-truth document is needed so producers and consumers agree on the schema, and so future migrations can detect old-shape documents deterministically via `schema_version`.

Three prior ADR-creation tasks — 0021 (artifact persistence lifecycle), 0022 (artifact metadata and lifecycle), 0023 (ADR system design) — overlap with this work. They remain valid and are referenced by the new ADR as **supplemented**, not superseded.

## Requirements

- Decide and document the unified base frontmatter fields present on every artifact:
  - `type` (kebab-case artifact discriminator)
  - identity field — each artifact's **own** identity key is `id` (quoted YAML string; slug/path-derived where no natural ID exists). References to a **foreign** artifact are keyed by the referenced artifact's snake_case type plus `_id` (e.g. `work_item_id`, `adr_id`).
  - `title`, `date` (ISO UTC, quoted), `author`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version` (per-artifact-type integer)
- Decide and document the provenance bundle (`revision`, `repository`) for code-state-anchored artifacts (plans, codebase-research, issue-research/RCA, design-inventory, pr-description). Confirm `git_commit` and `branch` are removed.
- Document the per-artifact-type extras for each of the twelve artifact types plus `note`.
- Reference 0021/0022/0023 and explain how each is supplemented.
- Decide where the source-of-truth lives — ADR alone, or ADR plus a dedicated machine-readable schema file.

## Acceptance Criteria

- [ ] A new ADR exists under the configured ADR directory that defines the unified base frontmatter schema, the provenance bundle, the `schema_version` contract, and the per-artifact-type extras.
- [ ] The ADR explicitly references 0021/0022/0023 and acknowledges them as supplemented.
- [ ] The ADR records the decision on whether a separate machine-readable schema file accompanies it; if yes, that file is produced.
- [ ] Identity-value shape contract is documented as part of the ADR: an artifact's own identity key is `id`; references to a foreign artifact are keyed `<snake_case_type>_id`; all identity values (own and foreign) are quoted YAML strings.

## Open Questions

- Should the source-of-truth schema document live as an ADR only, as a dedicated schema file under `meta/` or `docs/`, or as both (ADR for decision, schema file for machine-readable reference)?
- Should this ADR land before producer-skill updates begin, or alongside them (each story carries its piece of the decision)?

## Dependencies

- Blocked by: none.
- Blocks: every producer-skill / template / migration story under 0057 — those consume this ADR's decisions.
- Related: 0021, 0022, 0023 (supplemented); 0057 (parent epic).

## Assumptions

- The new ADR supplements the prior ADR-creation tasks 0021/0022/0023 rather than superseding them.
- Vocabulary unification of `status` values and review verdict enums is out of scope — only field-name and shape unification is decided here.

## Technical Notes

- The typed linkage vocabulary is intentionally decided in a separate ADR (0061) so the two decisions can move independently if the user chooses to split them.
- Schema-file format (if produced): plain YAML or JSON Schema both work; the ADR should decide.

## Drafting Notes

- Treated this as a `task` rather than `story` because the deliverable is a documentation artifact (the ADR), matching the framing of 0021/0022/0023 which are also ADR-creation tasks.
- Set `parent` to "0057" since 0057 is the parent epic. If 0057 is preferred as `relates_to` rather than `parent`, the field is easily moved.
- Split the schema ADR (this) from the typed-linkage-vocabulary ADR (0061) per the epic's hint that the two are conceptually distinct.
- Identity-key convention corrected per user after this task was marked `done`: own identity = `id`, foreign references = `<snake_case_type>_id` (was `<type>_id` for own identity, e.g. `adr_id`). Because this task is already `done`, the produced ADR document under the decisions directory still records the old shape and must be revised to match; this work item's requirements/AC have been updated to reflect the corrected decision.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0021, 0022, 0023, 0057, 0061
