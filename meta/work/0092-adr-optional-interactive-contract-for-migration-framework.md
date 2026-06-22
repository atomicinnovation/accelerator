---
id: "0092"
title: "ADR: Optional Interactive Contract for the Migration Framework"
date: "2026-05-26T10:00:00+00:00"
author: Toby Clemson
kind: task
status: done
priority: high
parent: "work-item:0057"
tags: [adr, migration, framework, accelerator-plugin]
type: work-item
schema_version: 1
last_updated: "2026-05-26T10:00:00+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0068", "codebase-research:2026-05-24-0068-related-documents-inference-accuracy"]
blocks: ["work-item:0062", "work-item:0069"]
relates_to: ["adr:ADR-0023", "adr:ADR-0030", "work-item:0023", "work-item:0030", "work-item:0057", "codebase-research:2026-05-24-0068-related-documents-inference-accuracy"]
external_id: PP-114
---

# 0092: ADR: Optional Interactive Contract for the Migration Framework

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Produce the ADR that amends ADR-0023's mechanical-by-default migration framework with a permanent, opt-in interactive contract any future migration may adopt. The ADR specifies the framework-level contract primitives — trigger-predicate shape, runner-surfaced display elements, accept / edit / skip behavioural semantics, and resumability mechanism — without prescribing how a particular migration parameterises them. Spike 0068's verdict (interactive hooks for low-confidence linkage inferences in the unified-schema migration) is the motivating consumer; this ADR generalises the contract so subsequent migrations can adopt it without re-amending ADR-0023.

## Context

ADR-0023 specifies the migration framework as purely mechanical: no prompts, no dry-run, VCS-as-rollback. Spike 0068 measured the unified-schema migration's body-section inference accuracy at 84% correct / 11.3% wrong / 4.7% uncertain — exceeding the deterministic-acceptable threshold of ≤5% wrong-rate. The spike's verdict (consumed by 0062) is interactive validation hooks; the residual wrong-rate under a cheap-fix counterfactual (~5.3%, resolving `template-path`, `prose-keyword-false-match`, and `sibling-as-deriv`) is still above threshold, so the verdict is robust to plausible parser improvements.

That verdict raises two distinct questions:

1. **What does the framework offer?** A permanent, opt-in interactive contract is the natural shape, but it amends ADR-0023's "no prompts" guarantee for any migration that adopts it. The amendment scope is broad (future migrations may opt in) rather than narrow (one-off exception for the corpus migration) — i.e. the framework permanently supports interactive hooks as an opt-in capability, while the mechanical no-prompt path remains the default for migrations that do not opt in.
2. **How does this migration apply it?** Confidence-band design, hybrid-vs-uniform routing, parser-fix ownership, vocabulary-gap resolutions — these are linkage-application decisions specific to the unified-schema migration.

This ADR addresses (1). The application-specific decisions live in 0062. Implementation of the framework contract is 0069.

## Requirements

- Amend ADR-0023's mechanical-by-default contract to admit a permanent optional interactive contract: any migration may declare a hook to request interactive validation for low-confidence transformations; migrations that do not declare the hook run identically to today. Identify the specific ADR-0023 clause(s) being amended (e.g. the no-prompts language) and state the replacement / addition.
- Specify the framework-level contract primitives a migration must declare to use the interactive path:
  - **Trigger predicate** — the shape (e.g. a boolean function over named transformation fields including a confidence value) by which a migration declares which transformations fire the prompt.
  - **Runner-surfaced display elements** — what the framework runner shows to the user when the prompt fires (e.g. the proposed transformation, its source line/section, the trigger predicate's value); migrations parameterise the specifics.
  - **User-control semantics** — accept / edit / skip controls, each defined by (i) the resulting frontmatter (or other artifact) mutation and (ii) the effect on the session log / resume state.
  - **Resumability mechanism** — the persistence artefact (file path and format) written by the migration runner, and the re-entry semantics by which a subsequent invocation resumes from it.
- Preserve the mechanical path as the framework's default: only migrations that declare the hook are affected by the amendment.

## Acceptance Criteria

- [ ] The ADR amends ADR-0023's mechanical-by-default contract by quoting or naming by section heading at least one specific clause (e.g. the no-prompts language) and stating the replacement / addition.
- [ ] The amendment is broad: any future migration may adopt the optional interactive contract without re-amending ADR-0023. The ADR explicitly preserves the mechanical-by-default path — migrations that do not declare the hook are unaffected.
- [ ] The ADR specifies the framework-level **trigger predicate** shape (e.g. a boolean function over named transformation fields including a confidence value) that migrations must declare to fire the prompt.
- [ ] The ADR enumerates the **runner-surfaced display elements** the framework presents when the prompt fires.
- [ ] The ADR defines **accept / edit / skip** controls by stating, for each, (i) the resulting frontmatter / artifact mutation and (ii) the effect on the session log / resume state.
- [ ] The ADR names the **resumability** persistence artefact (file path and format) written by the migration runner and the re-entry semantics by which a subsequent invocation resumes from it.
- [ ] The ADR conforms to the ADR-0030 template: required body sections (Context, Decision Drivers, Considered Options, Decision, Consequences with Positive / Negative / Neutral subsections, References) are present and non-empty; required frontmatter fields (`adr_id`, `date`, `author`, `status`, `tags`) are populated; supersession / amendment cross-references (`supersedes` / `superseded_by` / `derived_from` / equivalent) are added where applicable.

## Open Questions

- None — all decision points the ADR must resolve are captured in Requirements / Acceptance Criteria.

## Dependencies

- Blocked by: 0068 (spike whose verdict motivates the amendment — captured in `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`; status done). No scheduling impact.
- Blocks: 0062 (linkage-application ADR — adopts and parameterises the contract defined here), 0069 (framework-extension implementation — implements the contract in the runner).
- Related: 0023 (mechanical-contract ADR being amended; per the corpus's accepted-ADR immutability convention, ADR-0023 itself is not mutated — the amendment is recorded in the new ADR's text, and readers discover it via the supersession / amendment cross-reference declared by the new ADR), ADR-0030 (ADR template authority), 0057 (parent epic).

## Assumptions

- VCS revert remains the migration safety net regardless of whether a migration adopts the optional interactive contract — no inverse migration is built (consistent with ADR-0023).
- The framework contract is broad enough to cover linkage validation (the spike's motivating case) and any other low-confidence transformation class a future migration might surface — the ADR does not pre-commit to linkage-specific shapes.

## Technical Notes

- The hook contract specified here is the framework's API; 0062 parameterises it for the unified-schema migration (which confidence band fires; which linkage data is surfaced; what the edit operation mutates); 0069 implements it in the migration runner.
- Spike 0068's failure-pattern catalogue informs but does not constrain the contract's exact shape — the framework primitives should be general enough to accommodate parser improvements without re-amending ADR-0023.

## Drafting Notes

- Split from 0062 during pass-3 review (2026-05-26): the broad ADR-0023 amendment and the framework-level hook contract primitives belong at the framework abstraction level, while the linkage-specific application (confidence bands, hybrid-vs-uniform, parser routing, vocab gaps) stays in 0062. The split gives a three-layer chain — framework contract (this ADR) → migration application (0062) → runner implementation (0069).
- The contract primitives mirror AC 2 of pre-split 0062: trigger criterion, what is shown, accept/edit/skip, resumability. They are framed as framework-level shapes here, not migration-specific decisions.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Research: `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
- Related: 0023, ADR-0030, 0057, 0062, 0068, 0069
