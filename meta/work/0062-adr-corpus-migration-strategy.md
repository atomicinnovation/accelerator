---
work_item_id: "0062"
title: "ADR: Corpus Migration Strategy"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
type: task
status: draft
priority: high
parent: "0057"
tags: [adr, migration, frontmatter, accelerator-plugin]
---

# 0062: ADR: Corpus Migration Strategy

**Type**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Produce the ADR that decides how the unified-schema migration treats high-confidence vs uncertain transformations and whether the migration framework gains interactive validation hooks for this case. The decision drives the shape of the actual corpus-migration story (0070) and the framework-extension story (0069).

## Context

The migration framework today (`skills/config/migrate/`) is purely mechanical: no prompts, no dry-run, VCS-as-rollback per ADR-0023. The unified-schema migration in epic 0057 has two transformation classes — deterministic field renames / shape normalisation, and best-effort parsing of free-form "Related documents" body sections into typed linkage frontmatter. The second class has unknown inference accuracy.

Extending the framework with interactive validation hooks is a deliberate departure from the mechanical contract; this ADR decides whether the departure is taken or whether ambiguous inferences are surfaced via a post-run report instead.

## Requirements

- Decide whether the migration framework gains optional interactive validation hooks for low-confidence transformations.
- Define the contract those hooks expose (if added): when prompted, what data shown, accept/edit/skip semantics.
- Decide the alternative if hooks are not added: post-run report shape and where it lands.
- Specify the criterion the migration uses to classify a transformation as low- vs high-confidence.
- Reference ADR-0023 and acknowledge the contract change (if any).

## Acceptance Criteria

- [ ] A new ADR exists that decides interactive vs post-run-report strategy for the unified-schema migration.
- [ ] If interactive hooks are chosen, the contract is documented (prompt structure, accept/edit/skip semantics, idempotency on rerun).
- [ ] If post-run report is chosen, the report's location, structure, and consumption workflow are documented.
- [ ] The ADR cross-references ADR-0023 (mechanical contract) and explains the change explicitly.

## Open Questions

- Does the decision depend on the outcome of the spike (0068) that prototypes inference accuracy, or can the strategy be decided before the spike runs?
- Should the interactive vs report decision be one-time or per-migration?

## Dependencies

- Blocked by: possibly 0068 (spike on inference accuracy) — depends on whether the ADR is decided before or after the spike runs.
- Blocks: 0069 (framework extension, conditional on this ADR's outcome), 0070 (corpus migration shipping).
- Related: 0057 (parent epic), 0060, 0061 (schema and linkage ADRs the migration applies), 0023 (current mechanical contract).

## Assumptions

- VCS revert remains the migration safety net regardless of which strategy is chosen — no inverse migration is built.
- The decision applies specifically to the unified-schema migration; the general framework contract may or may not change permanently depending on ADR outcome.

## Technical Notes

- The migration framework's purely-mechanical posture is documented by ADR-0023. Any change here amends that contract.
- A hybrid is possible: deterministic transformations run mechanically; the linkage-inference pass produces a side-channel report or an interactive sub-flow.

## Drafting Notes

- The epic's open question 3 ("should there be an explicit spike before committing to interactive-vs-non-interactive design?") shapes the dependency between this ADR (0062) and the spike (0068). Treated 0068 as a possible blocker, but kept open since the ADR could also be drafted up front with a placeholder for the spike's findings.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0023, 0057, 0060, 0061, 0068, 0069, 0070
