---
id: "ADR-0034"
date: "2026-05-20T10:13:04+00:00"
author: Toby Clemson
status: accepted
tags: [frontmatter, schema, linkages, knowledge-graph, meta-directory]
type: adr
title: "ADR-0034: Typed linkage vocabulary for meta/ artifacts"
schema_version: 1
last_updated: "2026-05-20T10:13:04+00:00"
last_updated_by: Toby Clemson
relates_to: ["work-item:0060", "adr:ADR-0033", "work-item:0061", "work-item:0057", "work-item:0040"]
---

# ADR-0034: Typed linkage vocabulary for meta/ artifacts

**Date**: 2026-05-20
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0033 unified the base frontmatter schema across every artifact the Accelerator plugin produces — identity (`id`), provenance, and shape contracts — and explicitly deferred the typed cross-linkage vocabulary to a sibling ADR. This is that ADR.

Today the relationships between artifacts (a plan derived from research; a work item that blocks another; an ADR superseding an earlier one; a review targeting a plan) live in free-text body sections such as `## Related Research`, `## Dependencies`, or `## References`. With no structured edges to traverse, the visualiser presents the corpus as a linear pipeline rather than the graph it actually is, and producer skills cannot reliably navigate between related artifacts without parsing prose.

ADR-0033 also establishes that accepted ADRs are immutable. That constraint shapes how bidirectional relationships can be expressed: the older side of a `supersedes`/`superseded_by` pair cannot be mutated when its successor is accepted, so dual-writing both ends is infeasible for ADRs and consumers must be able to derive the inverse direction from whichever side is present.

One adjacent question is inherited from ADR-0033: whether design-gap's `current_inventory` / `target_inventory` keys remain type-specific or are folded into the generic vocabulary with a role qualifier.

## Decision Drivers

- A small, predictable keyword vocabulary that every artifact type can carry — consumers should not need per-type linkage parsing
- Compatibility with ADR-0033's immutability rule for accepted ADRs
- A reference value shape that is stable across renames and composes with ADR-0033's unified `id` field
- Rich edge semantics for the future visualiser-graph epic without inflating the keyword set
- Support for fan-in generative provenance — a plan must be able to derive from multiple research docs (inherited constraint from epic 0057)

## Considered Options

1. **Flat keyword vocabulary with type-pair semantics computed at read time** — Small set of generic keys (`parent`, `supersedes`, `blocks`, `target`, `derived_from`, `relates_to`, `source`). Edge meaning emerges from the (source-type, key, target-type) triple, looked up in a table this ADR publishes.
2. **Type-specific linkage keys per relationship** — e.g. `reviews:` (review→subject), `implements:` (plan→work-item), `extracted_from:` (work-item→note). Sharper semantics out of the box; vocabulary grows linearly with relationship types and couples consumers to the catalogue.
3. **Mandatory dual-write for bidirectional pairs** — Both sides must carry the link; integrity maintained by producers. Infeasible for accepted ADRs (immutability) without an exception layer.
4. **Path-only references** — Every link is a project-relative file path. Simple but brittle to renames; does not compose with ADR-0033's unified `id`.

## Decision

We will adopt option 1 — a flat, generic keyword vocabulary where either side of a bidirectional pair is sufficient to express the edge, and edge meaning is resolved via a published (source-type, key, target-type) table.

### Linkage keys

| Key             | Cardinality | Semantic                                                  |
|-----------------|-------------|-----------------------------------------------------------|
| `parent`        | single ref  | Hierarchical owner. Corpus-wide — any artifact type may carry it. |
| `supersedes`    | list        | This artifact replaces the referenced one(s). Primary use: ADRs. |
| `superseded_by` | single ref  | Inverse of `supersedes`. May be written on mutable artifacts; derivable from `supersedes` when absent. |
| `blocks`        | list        | This artifact's completion is a prerequisite for the referenced one(s). |
| `blocked_by`    | list        | Inverse of `blocks`. May be written on mutable artifacts; derivable from `blocks` when absent. |
| `target`        | single ref  | What this artifact is *about*. Open-domain — any artifact type may carry it. Primary use: reviews and validations. |
| `derived_from`  | list        | Generative source(s). A plan can derive from multiple research docs; this is the fan-in key. |
| `relates_to`    | list        | Loose linkage where no stronger key fits. Flat — no qualifier in v1. |
| `source`        | single ref  | External or non-meta origin for extracted artifacts (e.g., the notes file a work item was lifted from). |

### Single key sufficient for bidirectional pairs

For `supersedes` / `superseded_by` and `blocks` / `blocked_by`, writing either side is sufficient to express the relationship — consumers MUST be able to derive the inverse direction by traversing the corpus, without requiring both keys to be present.

- `supersedes` is the canonical side to write when possible. For ADRs, ADR-0033's immutability rule means the older artifact cannot be updated to carry `superseded_by`, so the newer artifact's `supersedes` is the only writable channel and must be the one consumers traverse.
- `blocks` and `blocked_by` may both be written on mutable artifacts (e.g. work items). Producers are not required to keep both in sync; consumers MUST tolerate either side being absent and derive what they need from whichever is present.

The rule is sufficiency, not exclusivity: one well-formed side establishes the edge, and consumers stitch the inverse direction themselves. Producers writing both sides is allowed but not required.

### Reference value shape

Two reference forms are supported:

1. **`doc-type:id`** — canonical, e.g. `"plan:0042"`, `"adr:ADR-0033"`, `"work-item:0061"`. Composes ADR-0033's artifact-type discriminator with its unified `id`. Stable across file renames.
2. **Project-root-relative path** — e.g. `"meta/notes/2026-01-15-pipeline-incident.md"`, `"src/auth/session.ts"`. Used when the target is not a meta artifact with a stable `id`, or when the link is to a specific code file.

The whole reference is a single quoted YAML string in both forms, per ADR-0033's identity-value contract — `"plan:0042"`, never `plan:"0042"`. Lists of references are YAML arrays of such strings.

Emission and acceptance rules apply uniformly to every linkage key:

- **Producers** emit `doc-type:id` when the target is a meta artifact with an `id`, or a project-root-relative path when the target lies outside `meta/` (e.g. a code file, an external doc) or otherwise lacks an `id`.
- **Consumers** MUST accept both forms on every key.

Producers SHOULD prefer `doc-type:id` whenever the target is a meta artifact, so the graph is traversable without filesystem lookups and survives file renames. Path-form references are allowed on every key so non-meta targets remain expressible as structured linkages rather than body prose; the uniform rule replaces an earlier per-key split that distinguished "structural" keys (id-only) from "semantic" keys (both forms).

### Type-pair semantic table

Edge meaning is the triple of (source-type, key, target-type), not the key alone. Consumers — the visualiser-graph epic in particular — render labels by lookup in this table. The keyword vocabulary stays small; semantics scale via the table.

At least one row per canonical linkage key is listed; the table is illustrative and extensible by future ADRs as new artifact pairings become load-bearing. Inverse keys (`superseded_by`, `blocked_by`) are derived from their canonical sides per the bidirectional-pair rule above and are not listed separately.

| Source type        | Key            | Target type        | Edge label                                  |
|--------------------|----------------|--------------------|---------------------------------------------|
| work-item          | `parent`       | work-item          | child story under epic                      |
| plan               | `parent`       | work-item          | plan owned by work item                     |
| adr                | `supersedes`   | adr                | replaces previous decision                  |
| work-item          | `blocks`       | work-item          | blocking dependency                         |
| plan-review        | `target`       | plan               | review subject                              |
| work-item-review   | `target`       | work-item          | review subject                              |
| plan-validation    | `target`       | plan               | validation subject                          |
| plan               | `derived_from` | codebase-research  | plan informed by research                   |
| plan               | `derived_from` | issue-research     | plan informed by RCA                        |
| work-item          | `derived_from` | note               | work item extracted from a note             |
| work-item          | `derived_from` | work-item          | refinement / decomposition output           |
| adr                | `relates_to`   | adr                | loose conceptual link                       |
| design-gap         | `relates_to`   | design-inventory   | loose linkage (qualified pairs use the type-specific keys below) |
| work-item          | `source`       | note               | extraction origin                           |

### Design-gap inventory keys

`current_inventory` / `target_inventory` remain type-specific keys on design-gap, not folded into the generic vocabulary. Rationale: these keys carry a role distinction (which inventory is the baseline, which is the goal) that has no equivalent in the flat vocabulary. Expressing the same semantics generically would require either two new top-level keys (defeating the point) or a per-link qualifier object — and we have no other use for per-link qualifiers in v1. This decision is consistent with the position ADR-0033 recorded under design-gap's per-type extras; this ADR confirms it as a corollary of keeping the generic vocabulary flat.

### Open questions resolved

- **Additional keys (e.g. dedicated `reviews:` instead of letting reviews use `target:`)?** — Deferred. `target` covers the review→subject relationship adequately for v1, and the type-pair table renders the "review" semantic from the triple. Affected consumers (review skills, `validate-plan`, the future visualiser-graph epic) can revisit if rendering ever needs a sharper key.
- **Qualifier on `relates_to`?** — Deferred. v1 is a flat list of references. Adding a qualifier shape later is a minor migration; adding it speculatively now would pull in per-link object syntax that no other key needs.

### Out of scope

- **Producer skill updates and template changes** — Tracked under parent epic 0057 as separate sibling stories that consume this ADR.
- **Corpus migration** — Backfilling linkages from existing body sections is a sibling story under 0057, not this ADR.
- **Rendering decisions** — How the visualiser draws the graph, what node/edge styling applies — owned by the future visualiser-graph epic.

## Consequences

### Positive

- The corpus becomes a navigable typed graph at the schema layer: consumers traverse seven well-known keys instead of parsing prose.
- Edge semantics scale via the type-pair table without inflating the vocabulary — adding a new artifact-type pairing is a table-row addition, not a new keyword.
- Sufficiency (not exclusivity) of either bidirectional side respects ADR-0033's immutability rule for accepted ADRs while still letting mutable artifacts carry both directions if producers find it convenient.
- `doc-type:id` references are stable across file renames and compose with ADR-0033's unified `id` field; the graph is traversable without filesystem lookups.
- The visualiser-graph epic is unblocked at the schema layer.

### Negative

- A meaningful backfill burden — existing artifacts encode relationships in body sections that must be migrated to frontmatter linkages. Tracked under 0057.
- Consumers must implement inverse-direction traversal because the inverse key may be absent on either side. The cost is borne by tooling, not by producers.
- The type-pair table is publishing surface that grows as new artifact pairings emerge; keeping it current is an ongoing obligation on this ADR (or a successor).

### Neutral

- `parent` is positioned as a corpus-wide vocabulary key; ADR-0033 retains the by-name listing on work items. The two ADRs are consistent — ADR-0033 names the field on the artifact type that uses it most heavily; this ADR owns the semantic.
- `target` is open-domain — any artifact type may carry it. The reviews/validations use is primary, not exhaustive.
- The reference shape allows both `doc-type:id` and project-relative paths; producers SHOULD prefer the former for meta-to-meta links.

## References

- `meta/work/0061-adr-typed-linkage-vocabulary.md` — Source task
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` — Parent epic
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — Defines `id`, identity-value shape, immutability rule; explicitly defers the typed-linkage vocabulary to this ADR
- `meta/work/0060-adr-unified-base-frontmatter-schema.md` — Sibling task that produced ADR-0033
- `meta/work/0040-visualiser-meta-content-rendering.md` — Visualiser context; the future visualiser-graph epic will consume this ADR
