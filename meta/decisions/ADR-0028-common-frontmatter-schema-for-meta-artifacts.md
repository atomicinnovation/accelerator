---
id: "ADR-0028"
date: "2026-03-22T00:00:00+00:00"
author: Toby Clemson
status: accepted
tags: [artifacts, metadata, frontmatter, lifecycle, meta-directory]
type: adr
title: "ADR-0028: Common frontmatter schema for meta/ artifacts"
schema_version: 1
last_updated: "2026-03-22T00:00:00+00:00"
last_updated_by: Toby Clemson
relates_to: ["plan:2026-03-22-validation-crossref-frontmatter", "codebase-research:2026-03-18-meta-management-strategy", "adr:ADR-0027", "adr:ADR-0001"]
---

# ADR-0028: Common frontmatter schema for meta/ artifacts

**Date**: 2026-03-22
**Status**: Accepted
**Author**: Toby Clemson

## Context

Skills under `meta/` had grown into a multi-artifact ecosystem — research
documents, plans, reviews, validations, PR descriptions — but only
`research-codebase` produced machine-parseable metadata. Plans, PR
descriptions, and the newly-persisted review artifacts were free-form
markdown.

This left several capabilities out of reach:

- No reliable way for tooling to query artifact status, filter by type,
  or trace lifecycle history without parsing prose.
- Skills produce artifacts consumed by other skills (e.g.,
  `respond-to-pr` triaging feedback that `review-pr` had already
  analysed) but there was no standard way to discover and read those
  artifacts.
- Plan lifecycle had no automated transitions. Plans stayed in their
  initial state even after `validate-plan` confirmed successful
  implementation; closure was a manual edit.

ADR-0027 had just established that structured skill outputs persist to
`meta/`. The next question was *what shape* those persisted artifacts
take so the rest of the system can rely on them.

## Decision Drivers

- Uniform machine-parseability across every artifact type
- Enable inter-skill communication via filesystem artifacts as a shared
  data contract
- Reduce manual status tracking through automated lifecycle transitions
- Keep type-specific metadata expressible without fragmenting the schema

## Considered Options

1. **No frontmatter convention** — Each skill defines its own metadata
   format. Tooling cannot rely on any common shape; the status quo
   prior to this ADR.
2. **Per-type schemas with no shared base** — Every artifact type
   defines its own complete frontmatter independently. Type-specific
   needs are well served, but cross-cutting queries ("show me all
   artifacts produced by skill X" or "find all artifacts still in
   `draft`") require type-specific parsers for each artifact kind.
3. **Common base schema, type-specific extensions** — Shared base
   fields (`date`, `type`, `skill`, `status`) on every artifact,
   extended with type-specific fields per artifact type. Downstream
   skills can reliably parse any artifact for identity/status without
   knowing its type.
4. **External metadata store** — A registry or JSON index file outside
   the artifacts themselves. Adds a synchronisation problem (artifact
   and index can drift) and duplicates what inline YAML frontmatter
   already provides.

## Decision

We will adopt a common base YAML frontmatter schema across all meta/
artifacts:

| Field    | Purpose                            |
|----------|------------------------------------|
| `date`   | ISO timestamp of artifact creation |
| `type`   | Artifact type identifier           |
| `skill`  | Skill that produced the artifact   |
| `status` | Lifecycle status of the artifact   |

Each artifact type extends this base with type-specific fields
(`work-item` on plans, `target` and `result` on validations,
`pr_number` and `pr_title` on PR descriptions, and so on).

The `status` field is shared in name and presence only — its value
vocabulary is per-type (plans use `draft`/`complete`, ADRs use
`proposed`/`accepted`/`rejected`/`superseded`/`deprecated`,
validations use `complete`, and so on). The base schema fixes that
every artifact has a status, not what its lifecycle looks like.

This schema becomes the data contract for two patterns:

- **Cross-referencing**: downstream skills load prior artifacts to
  enrich their work. `respond-to-pr` reads the latest numbered review
  artifact at `meta/reviews/prs/{number}-review-{N}.md` (highest `N`)
  to inform triage with severity ratings, confidence levels, and lens
  context.
- **Automated lifecycle transitions**: when `validate-plan` produces a
  passing validation, it updates the plan's frontmatter `status` to
  `complete`, closing the lifecycle without a manual edit.

The existing `research-codebase` frontmatter predates this convention
and uses a different field set. Field-name casing across artifacts is
also presently inconsistent (kebab-case `work-item` alongside
snake-case `pr_number`, `review_pass`, `adr_id`, `last_updated`).
Retrofitting `research-codebase` and normalising casing are both
deferred.

## Consequences

### Positive

- Uniform machine-parseability across every artifact type
- Inter-skill communication via filesystem artifacts is now a first-class
  pattern with a stable data contract
- Automated lifecycle closure eliminates one source of manual drift
- Future tooling (status queries, dashboards, filters) can depend on a
  consistent metadata shape

### Negative

- Coupling between `validate-plan` and plan artifacts: validation now
  mutates the plan file, not just emits a report
- Existing artifacts without frontmatter need to be retrofitted to gain
  the benefits, and that migration is not done in this change
- Downstream skills must degrade gracefully when a referenced artifact
  is absent (e.g., `respond-to-pr` on a PR with no prior review)

### Neutral

- `research-codebase` keeps its older frontmatter field set; alignment
  is deferred to future work
- Field-name casing remains mixed (kebab alongside snake) across
  artifact types; normalisation is deferred
- Automated lifecycle transitions are one-directional (validation pass
  → `complete`); other transitions like `draft` → `ready` remain manual
- The base schema fixes field presence and naming, not value
  vocabulary — each artifact type owns the value set for its `status`
  and `type` fields

## References

- `meta/plans/2026-03-22-validation-crossref-frontmatter.md` —
  Frontmatter convention, cross-referencing design, and lifecycle
  transition logic
- `meta/research/codebase/2026-03-18-meta-management-strategy.md` —
  Filesystem-as-shared-memory principle
- `meta/decisions/ADR-0027-persist-structured-skill-outputs-to-meta.md` —
  Establishes that structured outputs persist to `meta/`; this ADR
  defines the shape they take
- `meta/decisions/ADR-0001-context-isolation-principles.md` —
  Clear-context-phase foundation that motivates filesystem handoff
