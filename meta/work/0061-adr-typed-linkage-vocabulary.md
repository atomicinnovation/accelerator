---
id: "0061"
title: "ADR: Typed Linkage Vocabulary"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: task
status: done
priority: high
parent: "work-item:0057"
tags: [adr, frontmatter, knowledge-graph, accelerator-plugin]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0033", "work-item:0057", "work-item:0060", "work-item:0033", "work-item:0040"]
external_id: PP-83
---

# 0061: ADR: Typed Linkage Vocabulary

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Produce the ADR that decides the typed linkage vocabulary every Accelerator artifact uses to declare its relationships to other artifacts. The vocabulary is what turns the `meta/` corpus from disconnected silos into a navigable knowledge graph and is the precondition for the future visualiser-graph epic.

## Context

Per 0057, relationships between artifacts (a plan derived from research; a work-item that blocks another; an ADR superseding an earlier one; a design-gap referencing an inventory pair) currently live in free-text body sections like `## Related Research` rather than in structured frontmatter. With no structured graph edges to traverse, the visualiser today presents the corpus as a linear pipeline rather than the graph it actually is.

ADR-0033 (accepted) establishes the unified base frontmatter schema — including the `id` field that linkage references key into and the rule that ADRs are immutable once accepted. ADR-0033 explicitly defers the typed-linkage vocabulary to this ADR. It also leaves one adjacent decision to this ADR: whether design-gap's existing `current_inventory` / `target_inventory` keys remain type-specific or are folded into the generic vocabulary.

This ADR captures the decision; producer-skill updates (consuming the vocabulary) and corpus migration (populating links from existing body sections) happen in sibling stories.

## Requirements

- Define the typed linkage keys, their cardinality, and their semantics:
  - `parent` — single ref; hierarchical owner. Defined here as a corpus-wide linkage key; any artifact type may carry it. ADR-0033's base schema lists `parent` as a frontmatter field by name on work items; this ADR owns the vocabulary semantic.
  - `supersedes` / `superseded_by` — list; replacement relationship (primary use: ADRs).
  - `blocks` / `blocked_by` — list; dependency direction.
  - `target` — single ref; what this artifact is *about*. Open-domain — any artifact type may carry `target` to point at its subject. Primary use: reviews and validations.
  - `derived_from` — **list**; generative source (a plan can derive from multiple research docs).
  - `relates_to` — list; loose linkage.
  - `source` — single ref; external origin for extracted artifacts.
- Adopt **single-direction-of-truth** for bidirectional pairs (`supersedes`/`superseded_by`, `blocks`/`blocked_by`): only the "newer" or "owning" side carries the explicit key; the inverse is derived at read time. ADR-0033's immutability rule forces this for `supersedes` (the older ADR cannot be mutated) and uniformity carries it across the other pairs.
- Document the **type-pair semantic table**: edge meaning emerges from the (source-type, key, target-type) triple, not the key alone. For example, `work-item derived_from note` and `work-item derived_from work-item` are different relationships that share a generic key. The ADR tabulates these so consumers can render richer semantics without an expanded vocabulary.
- Define the **reference value shape**: either `doc-type:id` (e.g. `"plan:0042"`) or project-root-relative path (e.g. `"meta/plans/0042-foo.md"`). The whole reference is a single quoted YAML string per ADR-0033's identity-value shape contract, regardless of which form is chosen. The ADR specifies, for each linkage key, which reference shape(s) producers should emit and which consumers must accept.
- Decide the disposition of design-gap's `current_inventory` / `target_inventory` keys: keep as type-specific keys (as ADR-0033 currently lists them) or fold into the generic vocabulary with a qualifier mechanism — for example, each link entry carries a role tag such as `{ref: "design-inventory:2026-04-01-foo", role: "current_inventory"}`. The ADR records its choice and the rationale.

## Acceptance Criteria

- [ ] The new ADR (the deliverable of this task) exists under the configured ADR directory and defines each of the following keys, with cardinality and semantics: `parent`, `supersedes` / `superseded_by`, `blocks` / `blocked_by`, `target`, `derived_from`, `relates_to`, `source`.
- [ ] The ADR documents the (source-type, key, target-type) → semantic-label table, with at least one row per linkage key.
- [ ] The ADR adopts single-direction-of-truth for bidirectional pairs and specifies how inverses are derived at read time.
- [ ] The ADR decides the reference value shape (`doc-type:id`, project-relative path, or both) and specifies, for each linkage key, which reference shape(s) producers should emit and which consumers must accept.
- [ ] The ADR decides the disposition of design-gap's `current_inventory` / `target_inventory` keys and records the rationale.
- [ ] `derived_from` is explicitly documented as a list to support multi-source generative provenance.
- [ ] The ADR cross-references ADR-0033 so producers know where each linkage key lives in the frontmatter and how reference values are quoted.
- [ ] The ADR explicitly resolves or defers each open question listed in this work item, with rationale recorded.

## Open Questions

- Are there artifact-type pairs whose relationship semantics need explicit keys beyond those listed above (e.g. a dedicated `reviews:` instead of letting reviews use `target:`)? Affected consumers if such a key is added: review skills (`review-plan`, `review-work-item`, `review-pr`), `validate-plan`, and the future visualiser-graph epic.
- Should `relates_to` carry an optional qualifier (e.g. tag/role) or remain a flat list of IDs?

## Dependencies

- Builds on: ADR-0033 (accepted base schema — defines the `id` field, identity-value shape, and ADR immutability rule).
- Blocks: producer-skill / template / migration stories under 0057; the future visualiser-graph epic (work item to be created — this ADR is its precondition). Backfill with explicit work-item IDs once those sibling stories are created.
- Related: 0057 (parent epic), 0060 (sibling task that produced ADR-0033), 0040 (visualiser context).

## Assumptions

- The future visualiser-graph epic will consume the vocabulary defined here; this ADR does not need to make rendering decisions.
- Cardinality decisions (single ref vs list) are fixed at the schema level; per-artifact-type overrides are not expected.

## Technical Notes

- `derived_from` as list (rather than single ref) is non-negotiable per 0057 — a plan can derive from multiple research docs.
- ADR-0033's immutability rule for accepted ADRs is what forces single-direction-of-truth for `supersedes` — the older ADR cannot be mutated to add `superseded_by`. The decision then propagates to `blocks`/`blocked_by` for uniformity, even though work items remain mutable.
- Type-pair-aware semantics are computed at consumer time: render layers traverse the corpus, lift the (source-type, key, target-type) triple per edge, and look up the semantic label in the ADR's table. No additional frontmatter is required for this beyond the linkage keys themselves.

## Drafting Notes

- Single-direction-of-truth for bidirectional pairs was selected over dual-write because ADR-0033's immutability rule makes dual-write infeasible for `supersedes`/`superseded_by`. Uniformity carries the choice across the other pairs even though work items would technically allow dual-write.
- The "infer relationships from artifact-type pairs" requirement was reframed during enrichment from generic inference into the explicit (source-type, key, target-type) → semantic-label table. This is the mechanism that lets the keyword vocabulary stay small while consumers render rich edge semantics.
- Reference value shape (`doc-type:id` vs project-relative path) is left for the ADR to choose between; this work item does not pre-commit to one. The quoting form, however, is pinned: the whole reference is a single quoted YAML string per ADR-0033's identity-value contract (e.g. `"plan:0042"`, not `plan:"0042"`).
- `parent` is positioned as a corpus-wide linkage key rather than a work-item-specific frontmatter field — the ADR owns the vocabulary semantic while ADR-0033 retains the by-name listing on work items.
- `target` is open-domain rather than restricted to reviews/validations; the latter are recorded as its primary use.
- The design-gap `current_inventory`/`target_inventory` decision was inherited from ADR-0033, which marked it as type-specific but explicitly invited this ADR to revisit if the generic vocabulary can express the same semantics with a qualifier.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — Unified base schema (accepted); defines `id`, identity-value shape, and the deferral of typed linkages to this ADR
- Related: 0040, 0057, 0060
