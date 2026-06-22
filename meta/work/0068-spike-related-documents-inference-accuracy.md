---
id: "0068"
title: "Spike: Evaluate `Related Documents` Body-Section Inference Accuracy"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: spike
status: done
priority: medium
parent: "work-item:0057"
tags: [spike, migration, frontmatter]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
relates_to: ["work-item:0057", "codebase-research:2026-05-24-0068-related-documents-inference-accuracy"]
external_id: PP-90
---

# 0068: Spike: Evaluate `Related Documents` Body-Section Inference Accuracy

**Kind**: Spike
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Prototype the parser that extracts typed linkages from free-form body sections (`## Related Documents`, `## References`, `## Related Research`) across the current `meta/` corpus and measure its accuracy. The outcome informs whether the unified-schema migration (sibling work item 0070) needs interactive validation hooks (sibling work item 0069) or can rely on deterministic best-effort inference plus a post-run report.

## Context

Per 0057, relationships between artifacts (a plan derived from research; a work-item that blocks another) currently live in free-text body sections rather than structured frontmatter. The unified-schema migration must parse those sections and populate typed linkage frontmatter where confident.

The epic's open question 3 asks whether to commit to an interactive-vs-non-interactive migration design before this accuracy is known. This spike answers that question: if inference confidence is high across the dogfood corpus, the framework stays mechanical; if low, the framework gains interactive validation hooks (sibling work item 0069). The downstream migration-strategy ADR work item (0062) will cite this spike's findings as evidence for its decision.

## Requirements

- Prototype a parser that reads body sections plausibly containing artifact references and produces candidate typed linkage entries. Each candidate carries a parser-emitted **confidence band** (e.g. high/medium/low).
- Run the prototype against this repo's own `meta/` corpus.
- The spike implementer manually classifies each inferred linkage as **correct**, **uncertain**, or **wrong** (single-rater classification is acceptable for this time-box), and reports counts and rates. The manual-verdict classification is distinct from the parser-emitted confidence band — both are recorded per inference so accuracy can be reported per band.
- Identify the patterns the parser struggles with (e.g. references with no path, references with verbose prose around them, multi-target sentences).
- Recommend interactive-validation-hooks vs deterministic-with-report based on the findings, applying the decision rubric in Acceptance Criteria.

## Acceptance Criteria

- [ ] A working prototype parser exists (location and language at implementer's discretion — does not need to ship as production code) that runs end-to-end against the `meta/` corpus and emits at least one candidate linkage record per qualifying body section. Each record includes source path, target reference, inferred linkage type, and a parser-emitted confidence band.
- [ ] The parser run covers at least 100 inferences corpus-wide. If fewer than 100 candidates exist, the run covers every qualifying body section in `meta/` and the research artifact notes the corpus-exhaustion.
- [ ] Findings are written as a research artifact under `meta/research/codebase/`, containing:
  - explicit accuracy counts using the three manual-verdict labels (e.g. "84 correct, 11 uncertain, 4 wrong")
  - a per-confidence-band accuracy breakdown (so the recommendation rubric can be calibrated against the band)
  - a failure-pattern catalogue in which every **wrong** or **uncertain** inference is attributed to a named pattern
  - a recommendation for work item 0062 (migration-strategy ADR creation): interactive hooks vs deterministic + report
- [ ] The recommendation follows this rubric, committed in advance and **binding for this spike's verdict**: recommend **deterministic + report** if **wrong-rate ≤ 5%** AND **uncertain-rate ≤ 15%** on a sample of ≥ 100 inferences; otherwise recommend **interactive hooks**. The spike implementer must not change the thresholds after observing the counts; if the findings show the rubric is itself flawed (e.g. a borderline outcome where small label-judgement shifts would flip the verdict), the artifact records that observation and proposes alternative thresholds for a follow-on re-run rather than rewriting the rubric in place.
- [ ] Spike work item closes with a `Findings` section summarising the outcome and linking to the research artifact.
- [ ] Time-box: 1–2 days of effort. Beyond that, capture findings as-is and conclude the spike.

## Open Questions

_None outstanding. Earlier questions on findings-location and prototype-fate are resolved (see Drafting Notes)._

## Dependencies

- Blocked by: none (can run against the corpus as it exists today).
- Blocks:
  - **work item 0062** (migration-strategy ADR creation) — cites the spike's recommendation as evidence.
  - **work item 0070** (unified-schema migration) — consumes the spike's recommendation in either branch (deterministic + report or interactive hooks).
- Conditionally blocks:
  - **work item 0069** (migration-framework interactive validation hooks) — only blocked if the spike recommends interactive hooks; if the recommendation is deterministic + report, 0069 is moot.
- Related: 0057 (parent epic).

## Assumptions

- The corpus is sufficiently representative of the long-term mix of artifacts that accuracy measured here predicts accuracy on future corpora reasonably well.
- Manual classification of ~100 inferences is feasible within the time-box.

## Technical Notes

- The prototype is **throwaway**: it is discarded once findings are recorded. Any production migration parser is built separately under a follow-on story to epic 0057. Quality of measurement matters more than quality of code; language is at the implementer's discretion (Python, Bash, or anything else).
- Confidence-band scoring is a hard requirement (see Acceptance Criteria) — the per-band accuracy breakdown is what lets the recommendation rubric's thresholds be calibrated against parser output rather than chosen arbitrarily.

## Drafting Notes

- Treated this as a `spike` per the epic's open question 3 framing. Time-box set conservatively at 1–2 days.
- Resolved earlier Open Question #1: findings land as a research artifact under `meta/research/codebase/`, with the spike work item closing via a `Findings` link. Rationale: separates durable findings from work-item lifecycle, and lets work item 0062 (migration-strategy ADR creation) cite evidence directly rather than reaching into a closed work item.
- Resolved earlier Open Question on prototype fate: the prototype is throwaway; any production migration parser is a separate follow-on story under epic 0057. Rationale: preserves the spike's measurement focus and 1–2 day time-box; avoids the scope creep that would result from blending accuracy measurement with production-parser construction.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## Findings

Prototype parser run against the full `meta/` corpus on 2026-05-24 produced **1,231 candidate typed linkages** across the five qualifying section types (`## References`, `## Related Research`, `## Dependencies`, `## Historical Context`, `## Source References`). A stratified random sample of 150 candidates (50 per `high`/`medium`/`low` confidence band) was manually classified.

- **Total: 84.0% correct / 11.3% wrong / 4.7% uncertain**
- Per-band: high 88% correct, medium 90% correct, low 74% correct
- Top failure patterns: `template-path` (7), `source-note-vs-relates` (4), `plan-target-ambiguous` (3), `plan-source-vs-target` (2)

**Rubric application:** wrong-rate 11.3% **fails** the ≤5% threshold (uncertain-rate 4.7% passes). Per the pre-committed binding rubric, **recommend interactive hooks**.

The recommendation is robust to plausible parser improvements: even resolving every "cheap to fix" failure pattern only brings the wrong-rate to ~5.3%, still over threshold.

Full findings, parser design, per-band breakdown, failure-pattern catalogue with cheap-fix counterfactual, and rubric-calibration observation:

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`

**Downstream impact:**
- Work item 0062 (migration-strategy ADR creation) should cite this finding and decide on the interactive-hooks branch.
- Work item 0069 (migration-framework interactive validation hooks) is **no longer conditional** — this spike's recommendation activates it.
- Work item 0070 (unified-schema migration) consumes the recommendation regardless of branch.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Findings: `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
- Related: 0057, 0062, 0069, 0070
