---
adr_id: ADR-0004
date: "2026-03-29T09:06:01+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, lenses, scaling, iso-25010]
---

# ADR-0004: Lens Catalogue Expansion with Bounded Selection and Core Set

**Date**: 2026-03-29
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system uses a three-layer architecture (ADR-0002) where an
orchestrator selects specialist lenses, each executed by a reviewer agent, with
lenses designed around PBR principles and structural invariants (ADR-0003).

A gap analysis against ISO 25010 quality characteristics revealed that the
original seven lenses left significant coverage gaps in compatibility,
portability, safety, and functional suitability (correctness). Documentation and
database concerns were embedded as secondary responsibilities within other lenses
rather than receiving dedicated focus.

Expanding the catalogue to close these gaps introduces a cost-relevance tension:
running every lens on every review would increase execution cost with diminishing
returns, since many lenses are irrelevant to a given change. A
documentation-only change gains nothing from a database lens; a backend data
migration gains nothing from a usability lens. Noise from irrelevant findings
also degrades reviewer trust.

The parallel execution model means that adding lenses has near-zero marginal
latency cost — total review time equals the slowest lens, not the sum. However,
each lens consumes API tokens and produces findings that must be aggregated, so
unbounded selection still carries cost and noise penalties.

Some evaluation questions within a lens are only relevant when specific code
patterns are present (e.g., database queries, concurrency, API changes). Without
a mechanism to scope these, reviewers waste effort on inapplicable criteria.

## Decision Drivers

- **Comprehensive quality coverage**: the original seven lenses leave gaps
  against ISO 25010, particularly in compatibility, portability, safety, and
  correctness
- **Cost efficiency**: each lens consumes API tokens; running irrelevant lenses
  wastes budget without improving review quality
- **Noise reduction**: findings from inapplicable lenses or evaluation questions
  erode reviewer trust and make reviews harder to act on
- **Consistent baseline quality**: some concerns (architecture, code quality,
  test coverage, correctness) are relevant to nearly every non-trivial change
  and should not be skipped by selection heuristics
- **Scalability of the selection mechanism**: the approach must accommodate
  future lens additions without reworking the selection logic each time

## Considered Options

1. **Run all lenses on every review** — Expand the catalogue and execute every
   lens regardless of the change's characteristics. Maximises coverage but
   ignores relevance, increasing cost linearly with catalogue size and
   generating noise from inapplicable findings.

2. **Manual lens selection** — Users specify which lenses to run per review.
   Offers full control but places the burden on the user, requires lens
   familiarity, and leads to inconsistent coverage when users forget or misjudge
   relevance.

3. **Orchestrator auto-selection with a cap and core set** — The orchestrator
   selects 4-8 lenses per review based on observable characteristics of the
   change. A "core four" set (Architecture, Code Quality, Test Coverage,
   Correctness) is always included, setting the floor at four. Within each
   lens, conditional applicability sub-groups skip evaluation questions that
   don't apply to the change.

## Decision

We will expand the lens catalogue to close ISO 25010 coverage gaps, adding six
new lenses: documentation, database, correctness, compatibility, portability,
and safety. Three of these absorb concerns previously embedded in existing
lenses — documentation from standards, database from performance, compatibility
from usability — following the concern ownership transfer protocol established
in ADR-0003.

To manage the cost and noise implications of a larger catalogue, the
orchestrator will select 4-8 lenses per review based on observable
characteristics of the change (e.g., file types touched, patterns present in
the diff). Four lenses — Architecture, Code Quality, Test Coverage, and
Correctness — form a "core four" that the orchestrator always includes,
providing a consistent quality baseline; these four are never skipped, setting
the effective minimum at four. For changes with broader scope, additional
domain-specific lenses whose auto-detect criteria match the change fill the
remaining slots, with most reviews landing in the 6-8 range. If more than eight
lenses pass auto-detection, the orchestrator ranks by relevance and drops the
least applicable.

Within each lens, evaluation questions are organised into conditional
applicability sub-groups. Each sub-group has an observable-characteristic
condition (e.g., "when the change touches database queries") that signals when
its questions are relevant. Every lens retains at least one "always applicable"
group to ensure it produces findings regardless of the specific change pattern.

## Consequences

### Positive

- Closes ISO 25010 coverage gaps without requiring every lens to run on every
  review
- The core four ensures a consistent quality baseline across all non-trivial
  changes
- Conditional applicability sub-groups reduce noise from inapplicable evaluation
  questions within selected lenses
- Future lens additions don't increase per-review cost linearly — the cap
  absorbs growth
- Concern ownership transfers give documentation, database, and compatibility
  concerns dedicated focus rather than secondary treatment

### Negative

- Orchestrator selection logic becomes more complex, requiring relevance
  heuristics and ranking
- Boundary list maintenance scales linearly — each new lens must be added to
  every existing lens's "What NOT to Do" section
- The selection cap could occasionally exclude a relevant lens, missing findings
  that an uncapped run would catch
- Conditions in sub-groups must describe observable code characteristics, adding
  structural complexity to lens files

### Neutral

- The 4-8 range is a tunable parameter that can be adjusted based on experience
  with cost and coverage
- Each lens defines its own auto-detect criteria, distributing selection logic
  rather than centralising it
- The inline comment cap (~10 per review) remains unchanged; more lenses doesn't
  mean more inline noise

## References

- `meta/research/2026-02-22-review-lens-gap-analysis.md` — ISO 25010 gap
  analysis identifying coverage holes in the original seven lenses
- `meta/research/2026-03-15-review-lens-optimal-structure.md` — Research into
  optimal lens structure, conditional sub-groups, and catalogue sizing
- `meta/plans/2026-03-15-new-review-lenses.md` — Implementation plan for the
  six new lenses and selection cap
- `meta/plans/2026-03-15-review-lens-improvements.md` — Conditional
  applicability sub-groups added to existing lenses
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  architecture this decision builds on
- `meta/decisions/ADR-0003-pbr-lens-design-with-structural-invariants.md` —
  Structural invariants and concern ownership transfer protocol
