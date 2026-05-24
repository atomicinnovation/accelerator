---
work_item_id: "0062"
title: "ADR: Interactive Validation for Corpus Migration"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: task
status: ready
priority: high
parent: "0057"
tags: [adr, migration, frontmatter, accelerator-plugin]
---

# 0062: ADR: Interactive Validation for Corpus Migration

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Produce the ADR that decides how the unified-schema migration applies interactive validation to its low-confidence linkage inferences and resolves the spike-identified vocabulary-policy gaps in the typed-linkage vocabulary.

Spike 0068 has resolved the deterministic-vs-interactive question: verdict interactive hooks, wrong-rate 11.3% vs the 5% threshold, with the cheap-fix counterfactual — resolving `template-path`, `prose-keyword-false-match`, and `sibling-as-deriv` — still over threshold at ~5.3%. The framework-level optional interactive contract is the subject of sibling ADR-task 0092, which this ADR adopts.

Decisions this ADR makes:

- The confidence-band design (two-band vs three-band) used to choose which transformations fire the framework's interactive hook.
- Whether the hook contract applies in a **hybrid** shape (interactive only on low-confidence transformations; deterministic for high-confidence) or a **uniform** shape (interactive on every inference).
- The ownership of the spike's cheap-fix parser recommendations (this ADR, 0069, 0070, or out-of-scope).
- The in-place resolution of spike-identified vocabulary gaps (the literal `"Source:"` prose on plan artifacts; the "broader workstream" linkage type).

The decisions drive the shape of the framework-extension implementation (0069) and the corpus-migration shipping story (0070).

## Context

The migration framework today (`skills/config/migrate/`) is purely mechanical per ADR-0023. The unified-schema migration in epic 0057 has two transformation classes — deterministic field renames / shape normalisation, and best-effort parsing of free-form body sections (`## References`, `## Dependencies`, `## Related Research`, `## Historical Context`, `## Source References`) into typed linkage frontmatter. Spike 0068 measured the second class's accuracy at 84% correct / 11.3% wrong / 4.7% uncertain on a stratified sample of 150 inferences, exceeding the pre-committed deterministic-acceptable threshold of ≤5% wrong-rate. The spike's verdict is interactive validation hooks; the residual wrong-rate under a cheap-fix counterfactual — resolving `template-path`, `prose-keyword-false-match`, and `sibling-as-deriv` — is still above threshold at ~5.3%, so the verdict is robust to plausible parser improvements.

Sibling ADR-task 0092 owns the framework-level optional interactive contract — the amendment to ADR-0023 and the framework-level shapes of trigger predicate, runner-surfaced display elements, accept/edit/skip semantics, and resumability mechanism. This ADR adopts that contract and parameterises it for the unified-schema migration's linkage validation, and resolves the linkage-vocabulary gaps the spike surfaced. Spike 0068's calibration data shows the parser's current high vs medium confidence bands are statistically indistinct (88% vs 90% accuracy), which is the input to the confidence-band-design decision below.

## Requirements

- Decide the confidence-band design — two-band (resolved-and-typed vs ambiguous) or three-band with a sharpened high-band gate. Spike 0068's calibration data shows the current high/medium distinction is not load-bearing.
- Decide the hook contract's **application shape**: **hybrid** (interactive only on low-confidence transformations; deterministic for high-confidence — the spike's implicit recommendation) or **uniform** (interactive on every inference). The framework contract (0092) admits both; this migration must pick.
- Adopt the framework-level interactive contract defined by sibling ADR 0092 and parameterise it for this migration: name the trigger predicate (the confidence band or named boolean predicate over the inferred linkage's fields), the linkage-specific display elements the runner surfaces, the accept/edit/skip mutation targets (frontmatter fields / parse state), and the migration's resumability persistence artefact (path and format).
- Decide where the spike's cheap-fix parser recommendations (`template-path` blocklist, `\bblocks?\b` regex tightening, `\bsibling\b` hint) are owned — this ADR for policy, 0069 for contract, 0070 for implementation, or out-of-scope.
- Resolve the spike-identified vocabulary gaps in-place. (a) Canonical reading of the literal `"Source:"` prose on plan artifacts: plan documents commonly write lines like `- Source: meta/work/0057-...md` to identify their originating work-item; the vocab-canonical type for plan→work-item per ADR-0034 is `target`, while the author intent reads closer to `source` — pick one of the vocab types {`target`, `source`, `derived_from`} or record as a documented exception, and justify. (b) Treatment of the "broader workstream" linkage gap (a link that names a multi-ticket workstream rather than a single artifact — e.g. a plan that references "the unified-schema initiative" as a whole rather than a specific work item): resolve as a new vocab type, reuse of an existing type, or documented limitation. If documented limitation: state the unsupported input shape, the rationale for not handling it, and whether a follow-up ticket is created. (c) When the resolution introduces or reuses a vocabulary term, record the term's name, definition, and which artifact-type pairs it applies to.

## Acceptance Criteria

- [ ] The ADR adopts interactive validation hooks as the strategy, citing spike 0068's wrong-rate (11.3% vs 5% threshold) and the cheap-fix counterfactual (~5.3%) as binding rationale.
- [ ] The ADR adopts the framework-level interactive contract from sibling ADR 0092 and parameterises it for this migration:
  - [ ] **Trigger criterion**: names exactly one confidence band, or a named predicate expressed as a boolean function over the inferred linkage's fields, that fires the prompt.
  - [ ] **What is shown**: enumerates the linkage-specific display elements the runner surfaces (e.g. inferred linkage type and target, source line / section, confidence band).
  - [ ] **User controls**: each of accept / edit / skip is defined by stating (i) the resulting frontmatter mutation and (ii) the effect on the session log / resume state.
  - [ ] **Resumability**: names the persistence artefact (file path and format) written by the migration runner for this migration, and the re-entry semantics by which a subsequent invocation resumes from it.
- [ ] The ADR explicitly adopts or rejects the **hybrid** application shape (interactive only on low-confidence; deterministic for high-confidence) versus the **uniform** shape (interactive on every inference) and states the alternative considered.
- [ ] The ADR decides the confidence-band design (two-band vs three-band). The rationale explicitly cites the 88% / 90% high-vs-medium accuracy figures from spike 0068's calibration data and states whether they support collapsing to two bands or sharpening the high-band gate.
- [ ] The ADR decides where the spike's cheap-fix parser recommendations are owned — this ADR (as policy), 0069 (as framework contract), 0070 (as implementation), or out-of-scope — so neither 0069 nor 0070 is left ambiguous.
- [ ] The ADR resolves the spike-identified vocabulary gaps in-place. The canonical interpretation of the literal `"Source:"` prose on plan artifacts is chosen from the vocab types {`target`, `source`, `derived_from`} (or recorded as a documented exception) and justified. The "broader workstream" linkage is treated as a new vocab type, existing type reuse, or documented limitation; if documented limitation, the unsupported input shape, the rationale, and any follow-up ticket are stated. When the resolution introduces or reuses a vocabulary term, the ADR records the term's name, definition, and which artifact-type pairs it applies to. The ADR includes a paragraph explaining why the vocab-gap decisions are co-located with the linkage-application ADR rather than separated into a sibling vocabulary-amendment ADR derived from 0061 / ADR-0034.
- [ ] The ADR conforms to the ADR-0030 template: required body sections (Context, Decision Drivers, Considered Options, Decision, Consequences with Positive / Negative / Neutral subsections, References) are present and non-empty; required frontmatter fields (`adr_id`, `date`, `author`, `status`, `tags`) are populated; supersession / amendment cross-references (`supersedes` / `superseded_by` / `derived_from` / equivalent) are added where applicable.

## Open Questions

- None — all decision points the ADR must resolve are captured in Requirements / Acceptance Criteria.

## Dependencies

- Blocked by: 0060 (foundational unified-base-schema decision — the interactive-hook contract and vocab-gap resolutions are defined against this base schema; status done), 0061 / ADR-0034 (typed-linkage vocabulary — this ADR supplements it via the vocab-gap resolutions and needs the typed-linkage vocabulary settled; status done), 0068 (spike whose recommendation this ADR consumes — verdict captured in `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`; status done), 0092 (framework-level optional interactive contract — this ADR adopts and parameterises that contract; status ready). The first three upstreams are accepted; 0092 must land before 0062 can finalise its parameterisation.
- Blocks: 0069 (interactive-hooks framework extension — confirmed in-scope by the spike, not moot; the parser-fix routing decision may materially shift 0069's scope), 0070 (corpus migration shipping; the parser-fix routing decision may materially shift 0070's scope). Acceptance of this ADR resolves epic 0057's Open Question 3 (deterministic-vs-interactive migration design).
- Related: ADR-0030 (ADR template authority), 0057 (parent epic — see Blocks for OQ-resolution coupling). If this ADR introduces or reuses a typed-linkage vocabulary term, the term lives in this ADR's text; per the corpus's accepted-ADR immutability convention, ADR-0034 itself is not mutated — readers discover the supplementation via the cross-reference this ADR declares (mirroring how sibling ADR 0092 handles its ADR-0023 amendment).

## Assumptions

- VCS revert remains the migration safety net regardless of the hook contract — no inverse migration is built (consistent with ADR-0023).
- Spike 0068's rubric-based recommendation is binding for this ADR; the ADR does not re-litigate the 5% / 15% thresholds.
- Spike 0068's failure-pattern catalogue informs but does not constrain the production parser's exact design — that is 0070's concern, not this ADR's.
- The framework-level interactive contract (0092) is settled before this ADR finalises its parameterisation.

## Technical Notes

- Spike 0068's research write-up (`meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`) is the primary input to this ADR; the verdict, failure-pattern catalogue, and band-calibration observations should be cited directly in the ADR's Context.
- The interactive-hooks decision is committed (see AC 1); the **hybrid** (interactive only on low-confidence) vs **uniform** (interactive on every inference) application shape is the remaining choice — AC 3 forces it to be made explicitly.
- Sibling ADR 0092 carries the broad ADR-0023 amendment and the framework-level contract primitives; this ADR adopts and parameterises that contract for the unified-schema migration. The framework primitives' shapes (trigger predicate, display elements, control semantics, resumability) are 0092's; this ADR fills in the migration-specific values.

## Drafting Notes

- Split during pass-3 review (2026-05-26): the broad ADR-0023 amendment and the framework-level hook contract primitives were extracted into sibling ADR-task 0092. This ADR now owns the linkage-application decisions (confidence bands, hybrid-vs-uniform, parser-fix routing, vocab-gap resolutions) — i.e. the linkage-related decisions all sit together here.
- Vocabulary-policy decisions (literal `"Source:"` prose on plans, "broader workstream" gap) kept co-located here per author direction: they are linkage-vocabulary decisions and stay with the other linkage-related decisions rather than being routed to a separate vocabulary-amendment ADR derived from 0061 / ADR-0034. The vocab-gap resolution acceptance criterion requires the ADR itself to record the co-location rationale.
- Cheap-fix parser work routed as a decision the ADR makes about where the work lives, rather than as work this ticket performs.
- The vocab type `source` (a candidate resolution for the `"Source:"`-prose gap) is distinct from the literal `"Source:"` prose label being interpreted on plan artifacts; this work item backticks vocab types and quotes prose forms to keep them apart.
- Title narrowed from "Corpus Migration Strategy" to "Interactive Validation for Corpus Migration" to reflect spike 0068's verdict; the work-item title now commits to the chosen direction. If this work item's reviewer believes the title should stay neutral, flag it as a review finding so the title can be reverted before drafting begins.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Research: `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
- Related: ADR-0030, 0057, 0060, 0061 / ADR-0034, 0068, 0069, 0070, 0092
