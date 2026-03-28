---
title: "Lens catalogue scaling: 7 to 13 with selection cap"
type: adr-creation-task
status: todo
---

# ADR Ticket: Lens catalogue scaling: 7 to 13 with selection cap

## Summary

In the context of expanding coverage to close ISO 25010 gaps, we decided for
growing to 13 lenses with a 6-8 selection cap per review, a "core four"
always-consider set (Architecture, Code Quality, Test Coverage, Correctness),
and conditional applicability sub-groups within evaluation questions, to achieve
comprehensive coverage without wasteful execution or noise from irrelevant
findings, accepting orchestrator selection complexity and structural complexity
in lens files.

## Context and Forces

- The initial 7 lenses leave gaps against ISO 25010 quality characteristics
  (documentation, database, correctness, compatibility, portability, safety)
- Running all 13 lenses on every review would double cost with diminishing
  returns — many lenses are irrelevant to a given change
- Some evaluation questions within a lens are only relevant when specific code
  patterns are present (e.g., database queries, concurrency)
- A "core four" set provides a baseline quality gate regardless of change type
- Noise from irrelevant findings degrades reviewer trust and usefulness

## Decision Drivers

- Comprehensive coverage of ISO 25010 quality characteristics
- Cost efficiency: don't run irrelevant lenses
- Noise reduction: don't generate findings for inapplicable concerns
- Consistent baseline quality: some lenses should always be considered
- Practical scalability of the review system

## Considered Options

1. **Run all lenses always** — Maximum coverage but wasteful and noisy
2. **Manual lens selection** — User picks lenses per review. Flexible but
   burdensome and inconsistent.
3. **Auto-detect with selection cap** — Orchestrator selects 6-8 lenses based
   on relevance criteria, with a "core four" always-consider set. Within lenses,
   conditional sub-groups skip inapplicable evaluation questions.

## Decision

We will expand to 13 lenses with a 6-8 selection cap per review. The
orchestrator auto-detects which lenses are relevant based on the change's
characteristics. Architecture, Code Quality, Test Coverage, and Correctness are
always considered ("core four"). Within each lens, evaluation questions are
organized into conditional applicability sub-groups with observable-
characteristic conditions (e.g., "if the change touches database queries") and
at least one "always applicable" group per lens.

## Consequences

### Positive
- Comprehensive coverage without running irrelevant lenses
- The "core four" ensures consistent baseline quality
- Conditional sub-groups reduce noise from inapplicable evaluation questions
- Adding future lenses doesn't increase per-review cost linearly

### Negative
- Orchestrator selection logic adds complexity
- Conditions must describe observable code characteristics (not intent)
- The cap could occasionally exclude a relevant lens
- Structural complexity in lens files increases

### Neutral
- The 6-8 cap is tunable based on experience
- Each lens's auto-detect criteria are defined within the lens itself

## Source References

- `meta/plans/2026-03-15-new-review-lenses.md` — Lens expansion plan and
  selection cap design
- `meta/plans/2026-03-15-review-lens-improvements.md` — Conditional
  applicability sub-groups
- `meta/research/2026-03-15-review-lens-optimal-structure.md` — Structural
  invariants supporting conditional groups
