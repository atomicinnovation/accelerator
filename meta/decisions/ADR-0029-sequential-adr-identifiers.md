---
adr_id: ADR-0029
date: "2026-03-18T00:00:00+00:00"
author: Toby Clemson
status: accepted
tags: [adr, decisions, naming, identifiers]
---

# ADR-0029: Sequential ADR identifiers

**Date**: 2026-03-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADRs need stable identifiers so cross-references and supersession chains
survive re-ordering. The accelerator's `meta/` directory already uses
date-prefixed naming (`YYYY-MM-DD-description.md`) for research and plans,
but those artifacts are read in chronological order and rarely cite each
other by name. ADRs are different: one ADR may explicitly supersede
another, and reviewers, tooling, and prose all need a stable handle to
refer to a specific decision.

## Decision Drivers

- Stable cross-references that survive re-ordering, rename, or amendment
- Unambiguous ordering between decisions
- Support for supersession chains linked by identifier
- Trade-off between consistency with the existing `meta/` naming style and
  the stability ADRs require

## Considered Options

1. **Date-prefixed `YYYY-MM-DD-description.md`** — Consistent with the
   existing research and plan filenames. Dates do not unambiguously order
   decisions made on the same day, and any re-dating (e.g. revising a
   draft) invalidates every existing reference to the file.
2. **Sequential `ADR-NNNN-description.md`** — Each ADR receives a permanent
   zero-padded number at creation. References by identifier remain valid
   regardless of when the file was created or amended. Date is recorded in
   frontmatter rather than the filename.
3. **Hybrid `ADR-NNNN-YYYY-MM-DD-description.md`** — Combines the sequential
   identifier with a date prefix. Adds the date's visibility at the cost of
   making the filename longer and duplicating information already captured
   in frontmatter; the date in the filename becomes wrong if the decision
   is revised before acceptance.

## Decision

ADRs use `ADR-NNNN-description.md` with zero-padded sequential numbering.
The number is allocated at creation and never reused. The `description`
slug is descriptive metadata; the canonical handle is `ADR-NNNN`. Date is
recorded in the frontmatter `date` field and the in-body status block, not
in the filename.

## Consequences

### Positive

- Cross-references by `ADR-NNNN` remain stable across re-orderings,
  renames, and amendments
- Supersession chains link by identifier without depending on date
- Zero padding keeps lexicographic order matching numeric order

### Negative

- `meta/` now contains two naming styles — date-prefixed for research and
  plans, sequential for decisions
- Numbers are never reused, so rejected or abandoned ADRs leave gaps in
  the numbering

### Neutral

- The `description` slug in the filename is descriptive only; tooling and
  prose should reference an ADR by its `ADR-NNNN` identifier
- Allocation of the next number is mechanical; ADR authoring does not need
  to track the previous high-water mark

## References

- `meta/decisions/ADR-0030-adr-template.md` — template structure used
  inside each ADR file
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — lifecycle
  that operates on these identifiers
- `meta/decisions/ADR-0032-adr-skill-decomposition.md` — skills that
  create and manage these files
- `meta/research/codebase/2026-03-18-adr-support-strategy.md` — ADR
  strategy research covering numbering, templates, lifecycle, and
  enforcement
- `meta/plans/2026-03-18-adr-skills.md` — implementation plan for the
  three ADR skills
- `meta/work/0023-adr-system-design.md` — source work item
