---
work_item_id: "0061"
title: "ADR: Typed Linkage Vocabulary"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
type: task
status: draft
priority: high
parent: "0057"
tags: [adr, frontmatter, knowledge-graph, accelerator-plugin]
---

# 0061: ADR: Typed Linkage Vocabulary

**Type**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Produce the ADR that decides the typed linkage vocabulary every Accelerator artifact uses to declare its relationships to other artifacts. The vocabulary is what turns the `meta/` corpus from disconnected silos into a navigable knowledge graph and is the precondition for the future visualiser-graph epic.

## Context

Per 0057, relationships between artifacts (a plan derived from research; a work-item that blocks another; an ADR superseding an earlier one; a design-gap referencing an inventory pair) currently live in free-text body sections like `## Related Research` rather than in structured frontmatter. The visualiser currently models the artifact lifecycle as a linear path because the corpus has no structured graph edges to traverse.

This ADR captures the decision; producer-skill updates (consuming the vocabulary) and corpus migration (populating links from existing body sections) happen in sibling stories.

## Requirements

- Define the typed linkage keys, their cardinality, and their semantics:
  - `parent` — single ref; hierarchical owner.
  - `supersedes` / `superseded_by` — list; replacement relationship (primary use: ADRs).
  - `blocks` / `blocked_by` — list; dependency direction.
  - `target` — single ref; what this artifact is *about* (reviews, validations).
  - `derived_from` — **list**; generative source (a plan can derive from multiple research docs).
  - `relates_to` — list; loose linkage.
  - `source` — single ref; external origin for extracted artifacts.
- Document which artifact-type pairs imply which relationship types so consumers can infer relationships not explicitly keyed.
- State the rule that referenced IDs follow the project's identity-value shape contract (quoted YAML strings).

## Acceptance Criteria

- [ ] A new ADR exists under the configured ADR directory that defines each typed linkage key, its cardinality, and its semantics.
- [ ] The ADR documents the rule for inferring relationship types from artifact-type pairs.
- [ ] `derived_from` is explicitly documented as a list to support multi-source generative provenance.
- [ ] The ADR cross-references 0060 (unified base schema) so producers know where each linkage key lives in the frontmatter.

## Open Questions

- Are there artifact-type pairs whose relationship semantics need explicit keys beyond the seven above (e.g. a dedicated `reviews:` instead of letting reviews use `target:`)?
- Should `relates_to` carry an optional qualifier (e.g. tag/role) or remain a flat list of IDs?

## Dependencies

- Blocked by: 0060 (the base schema ADR — defines where linkage keys live in the frontmatter).
- Blocks: producer-skill / template / migration stories under 0057.
- Related: 0057 (parent epic), 0060 (base schema), 0040 (visualiser context).

## Assumptions

- The future visualiser-graph epic will consume the vocabulary defined here; this ADR does not need to make rendering decisions.
- Cardinality decisions (single ref vs list) are fixed at the schema level; per-artifact-type overrides are not expected.

## Technical Notes

- `derived_from` as list (rather than single ref) is non-negotiable per 0057 — a plan can derive from multiple research docs.
- Bidirectional pairs (`supersedes`/`superseded_by`, `blocks`/`blocked_by`) require either dual-write at producer time or a single-direction-of-truth rule with a derivation step. The ADR should decide.

## Drafting Notes

- Kept this separate from the base-schema ADR (0060) per the epic's signal that the two decisions can move independently. If consolidating into one ADR is preferred, this work item can be folded into 0060.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0040, 0057, 0060
