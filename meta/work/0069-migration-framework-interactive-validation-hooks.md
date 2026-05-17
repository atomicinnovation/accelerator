---
work_item_id: "0069"
title: "Extend Migration Framework with Interactive Validation Hooks"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: "0057"
tags: [migration, framework, accelerator-plugin]
---

# 0069: Extend Migration Framework with Interactive Validation Hooks

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Extend the Accelerator meta-directory migration framework (`skills/config/migrate/`) with optional interactive validation hooks so migrations can prompt the user to confirm low-confidence transformations. Conditional on 0062's migration-strategy ADR deciding interactive hooks are wanted; conditional on 0068's spike showing accuracy is too low for purely deterministic migration.

## Context

The migration framework today is purely mechanical per ADR-0023: no prompts, no dry-run, VCS-as-rollback. The unified-schema migration's body-section inference pass may not meet a deterministic confidence bar, in which case the framework needs an extension point for interactive validation.

This story is conditional on:
- 0062 (migration-strategy ADR) deciding interactive hooks are the chosen strategy.
- 0068 (spike) finding inference accuracy below the confidence threshold.

If either outcome is "deterministic + post-run report", this story is closed without implementation.

## Requirements

- Extend the migration framework with an optional hook that a migration can declare to request interactive validation for low-confidence transformations.
- Define the hook's contract: prompt structure, accept/edit/skip semantics, idempotency on re-run (resumability).
- Preserve the existing purely-mechanical path: migrations that don't declare the hook continue to run with no prompts.
- Update the migration framework's documentation to describe the new optional contract and its relationship to ADR-0023.

## Acceptance Criteria

- [ ] Migrations can declare an interactive-validation hook; migrations that don't declare it run identically to today.
- [ ] When invoked, the hook presents low-confidence transformations one at a time with accept / edit / skip controls.
- [ ] A re-run after partial completion resumes from the last unprocessed transformation rather than re-prompting confirmed ones.
- [ ] The framework's documentation describes the new contract and amends or references ADR-0023.
- [ ] Tests cover at least: mechanical-only migration unaffected; interactive migration with all-accept, mixed accept/edit/skip, and partial-run-then-resume.

## Open Questions

- Resumability mechanics: is state stored in the migration framework, or in the migration itself (e.g. checkpoint file under `meta/`)?
- What happens if the user invokes a different command mid-migration? Is the migration's state corrupted, or does the framework guard against this?
- Are interactive hooks single-purpose (linkage-inference) or general-purpose (any low-confidence step)?

## Dependencies

- Blocked by: 0062 (migration-strategy ADR — decides whether this story is even built), 0068 (spike — informs whether interactive hooks are needed).
- Blocks: 0070 (corpus migration consumes this extension if it's built).
- Related: 0057 (parent epic), 0023 (mechanical-contract ADR being amended).

## Assumptions

- The hook is opt-in per migration; the framework's default path stays mechanical.
- VCS revert remains the safety net even with interactive hooks — partial-run state must be recoverable from VCS without manual surgery.

## Technical Notes

- The framework's purely-mechanical posture is documented by ADR-0023. This story explicitly amends that contract for the optional interactive case.
- Resumability via a checkpoint file (e.g. `meta/.migrate-state.json`) is one option; transactional VCS commits per accepted transformation is another. The implementer decides.

## Drafting Notes

- Marked this conditional — it may not be built if 0062 / 0068 outcomes favour deterministic-with-report. Captured here so the option is visible during planning.

Extracted from source documents without interactive enrichment. Acceptance criteria, dependencies, and type may need refinement before promoting from `draft` to `ready`.

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Related: 0023, 0057, 0062, 0068, 0070
