---
work_item_id: "0068"
title: "Spike: Evaluate `Related Documents` Body-Section Inference Accuracy"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: spike
status: draft
priority: medium
parent: "0057"
tags: [spike, migration, frontmatter]
---

# 0068: Spike: Evaluate `Related Documents` Body-Section Inference Accuracy

**Kind**: Spike
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Prototype the parser that extracts typed linkages from free-form body sections (`## Related Documents`, `## References`, `## Related Research`) across the current `meta/` corpus and measure its accuracy. The outcome informs whether the unified-schema migration (0070) needs interactive validation hooks (0069) or can rely on deterministic best-effort inference plus a post-run report.

## Context

Per 0057, relationships between artifacts (a plan derived from research; a work-item that blocks another) currently live in free-text body sections rather than structured frontmatter. The unified-schema migration must parse those sections and populate typed linkage frontmatter where confident.

The epic's open question 3 asks whether to commit to an interactive-vs-non-interactive migration design before this accuracy is known. This spike answers that question: if inference confidence is high across the dogfood corpus, the framework stays mechanical; if low, the framework gains interactive validation hooks (0069).

## Requirements

- Prototype a parser that reads body sections plausibly containing artifact references and produces candidate typed linkage entries.
- Run the prototype against this repo's own `meta/` corpus.
- Manually classify each inferred linkage as correct, plausibly-correct-but-uncertain, or wrong, and report counts and rates.
- Identify the patterns the parser struggles with (e.g. references with no path, references with verbose prose around them, multi-target sentences).
- Recommend interactive-validation-hooks vs deterministic-with-report based on the findings.

## Acceptance Criteria

- [ ] A working prototype parser exists (location and language at implementer's discretion — does not need to ship as production code).
- [ ] Accuracy is measured against the current `meta/` corpus with explicit counts (e.g. "84 confident inferences, 11 uncertain, 4 wrong").
- [ ] Failure patterns are catalogued.
- [ ] A recommendation is written for 0062 (migration-strategy ADR): interactive hooks vs deterministic + report.
- [ ] Time-box: 1–2 days of effort. Beyond that, capture findings as-is and conclude the spike.

## Open Questions

- Does the spike output need to land as a research artifact under `meta/research/codebase/`, or is a comment under this work item sufficient?
- Should the spike's prototype evolve into the actual migration parser, or be discarded once findings are recorded?

## Dependencies

- Blocked by: none (can run against the corpus as it exists today).
- Blocks: 0062 (migration-strategy ADR — uses the spike's recommendation), and possibly 0069 / 0070 depending on outcome.
- Related: 0057 (parent epic).

## Assumptions

- The corpus is sufficiently representative of the long-term mix of artifacts that accuracy measured here predicts accuracy on future corpora reasonably well.
- Manual classification of ~100 inferences is feasible within the time-box.

## Technical Notes

- The prototype can be a throwaway script (Python, Bash, or anything else). Quality of output measurement matters more than quality of code.
- Consider scoring inferences with a confidence band so the threshold for "high-confidence enough to skip interactive validation" can be calibrated.

## Drafting Notes

- Treated this as a `spike` per the epic's open question 3 framing. Time-box set conservatively at 1–2 days.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0057, 0062, 0069, 0070
