---
id: "ADR-0030"
date: "2026-03-18T00:00:00+00:00"
author: Toby Clemson
status: accepted
tags: [adr, decisions, template, frontmatter]
type: adr
title: "ADR-0030: ADR template"
schema_version: 1
last_updated: "2026-03-18T00:00:00+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0028", "adr:ADR-0029", "adr:ADR-0031", "adr:ADR-0032", "codebase-research:2026-03-18-adr-support-strategy", "plan:2026-03-18-adr-skills", "work-item:0023"]
---

# ADR-0030: ADR template

**Date**: 2026-03-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

Each ADR needs a structure that captures enough context for a future
reader to understand why the decision was made, surfaces the alternatives
that were considered, and remains concise enough that authors actually
fill it in. Two well-known templates dominate the space: Nygard's original
(Context / Decision / Status / Consequences) and MADR (multiple variants
including a "long" form with Decision Drivers and Considered Options).
Both are proven; the question is which suits this project.

## Decision Drivers

- Surface decision drivers and alternatives so future readers can judge
  whether the decision still holds
- Stay concise enough for everyday decisions, not just large
  architectural pivots
- Machine-parseable metadata for tooling (status queries, supersession
  traversal, tag filters)
- Alignment with the common `meta/` frontmatter schema established by
  ADR-0028

## Considered Options

1. **Nygard original** — Context, Decision, Status, Consequences. Concise
   and widely understood. No structured place for alternatives or
   drivers, so they get buried in prose if recorded at all.
2. **MADR (full)** — Title, Context and Problem Statement, Decision
   Drivers, Considered Options with Pros/Cons each, Decision Outcome,
   Confirmation, Pros/Cons of the Decision, More Information.
   Comprehensive, but heavyweight for typical decisions. *Confirmation*
   and *More Information* in particular rarely add value at this
   project's scale, and per-option Pros/Cons duplicate work that the
   Consequences section already does for the chosen option.
3. **Hybrid Nygard + MADR** — Nygard's brevity (Context, Decision,
   Consequences) plus MADR's Decision Drivers and Considered Options.
   YAML frontmatter for machine-parseable metadata.

## Decision

ADRs use a hybrid template with these sections in order: Context,
Decision Drivers, Considered Options, Decision, Consequences (split into
Positive / Negative / Neutral), References. YAML frontmatter carries
`adr_id`, `date`, `author`, `status`, optional `supersedes` and
`superseded_by`, and `tags`.

This frontmatter is a partial conformance to the common `meta/` base
schema in ADR-0028. From the base, ADRs keep `date` and `status`. They
omit `type` (every file under `meta/decisions/` is type `adr`, so the
field is redundant) and `skill` (an ADR is a project-level artifact, not
the output of a single skill — `create-adr` and `extract-adrs` both
produce ADRs of the same shape). They add `adr_id` (the canonical
handle from ADR-0029), `author`, the supersession pair, and `tags`.

The in-body status block (Date / Status / Author) duplicates the
frontmatter because rendered markdown hides YAML frontmatter — without
the block, a reader of the rendered ADR would see no status, date, or
author.

The `create-adr` skill is the sole authority for the template;
`extract-adrs` follows the same shape by convention.

## Consequences

### Positive

- Decision Drivers and Considered Options give future readers a
  structured place to find context and alternatives
- The Positive / Negative / Neutral split forces honest accounting of
  trade-offs rather than pure justification
- YAML frontmatter makes status, tags, and supersession queryable by
  tooling
- Concise enough that authors fill the sections in rather than skipping
  them

### Negative

- Template authority lives in one skill (`create-adr`); other skills
  must track template changes by convention, not by shared code
- The in-body status block duplicates frontmatter — both must be kept in
  sync on every status transition

### Neutral

- The frontmatter is a partial conformance to the ADR-0028 base
  schema: `date` and `status` are kept, `type` and `skill` are
  intentionally omitted as redundant, and `adr_id`, `author`,
  supersession, and `tags` are added
- The template is the same regardless of whether the ADR was authored
  via `create-adr` or extracted by `extract-adrs`

## References

- `meta/decisions/ADR-0028-common-frontmatter-schema-for-meta-artifacts.md`
  — base frontmatter schema this template extends
- `meta/decisions/ADR-0029-sequential-adr-identifiers.md` — identifier
  scheme used in the `adr_id` field
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — lifecycle
  that operates on the `status` field
- `meta/decisions/ADR-0032-adr-skill-decomposition.md` — the three
  skills that produce ADRs with this template
- `meta/research/codebase/2026-03-18-adr-support-strategy.md` — template
  research
- `meta/plans/2026-03-18-adr-skills.md` — implementation plan
- `meta/work/0023-adr-system-design.md` — source work item
