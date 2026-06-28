---
type: work-item
id: "0174"
title: "Retire Shell Tooling and CI Guards"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
parent: "work-item:0136"
blocked_by: ["work-item:0167", "work-item:0168", "work-item:0169", "work-item:0170", "work-item:0171", "work-item:0172", "work-item:0173"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [shell, tooling, ci, cleanup]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-195"
---

# 0174: Retire Shell Tooling and CI Guards

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As each script cluster's last shell file disappears, retire the build-system and CI
machinery that exists only to police shell — the bashisms linter, exec-bit
invariant, `SHELL_LIBRARIES`, shell-suite floors, shfmt/ShellCheck, `shell_sources()`,
and the `check-scripts` CI job — shrinking the shell surface toward the thin-wrapper
floor ADR-0048 targets.

## Context

A substantial build-system layer exists purely to guard the bash library: the
bashisms denylist (ADR-0049 floor), the exec-bit invariant + `SHELL_LIBRARIES`
frozenset, the shell-suite discovery + minimum-count floors, shfmt + ShellCheck, the
`.shellcheckrc` and `[*.sh]` editorconfig block, and the `check-scripts` CI job (a
release gate). Each becomes removable as the scripts it polices disappear. The
floors must be decremented in lockstep with suite retirement to avoid a green→red CI
gap. A thin slice of shell remains (the launcher bootstrap, the hook wrapper, the
Playwright executor) and stays under the bash-3.2 floor.

## Requirements

- As each cluster retires (in the subdomain stories), decrement the corresponding
  shell-suite floor in `tasks/test/integration.py` and shrink `SHELL_LIBRARIES` in
  `tasks/lint/scripts.py` in the same change — never leaving a floor expecting a
  removed suite.
- Once a checker has no remaining inputs, remove it: the bashisms linter
  (`scripts/lint-bashisms.sh` + its task), the exec-bit invariant guard, shfmt and
  ShellCheck tasks, the `.shellcheckrc`, and the `[*.sh]` editorconfig block.
- Remove `shell_sources()` (`tasks/shared/sources.py`) and the `check-scripts` CI
  job (`.github/workflows/main.yml`, including its release-gate dependency) once no
  policed shell remains.
- Keep the surviving thin shell (launcher bootstrap, hook wrapper, Playwright
  executor) bash-3.2-safe; decide whether a reduced bashisms check still guards
  them or whether the surviving set is small enough to review by hand.

## Acceptance Criteria

- [ ] Each suite-floor decrement and `SHELL_LIBRARIES` shrink lands in the same
      change that deletes the corresponding scripts; CI never goes green→red on a
      floor mismatch.
- [ ] Once their inputs are gone, the bashisms linter, exec-bit invariant, shfmt,
      ShellCheck, `.shellcheckrc`, and `[*.sh]` editorconfig block are removed and
      `mise run check` / bare `mise run` still pass.
- [ ] `shell_sources()` and the `check-scripts` CI job are removed; the release gate
      no longer depends on `check-scripts`.
- [ ] Any surviving thin shell is documented and held to bash 3.2 (ADR-0049).

## Open Questions

- Whether a minimal bashisms/shfmt check is retained for the surviving thin shell,
  or the residual set is small enough to drop automated checks entirely.

## Dependencies

- Blocked by: 0167, 0168, 0169, 0170, 0171, 0172, 0173 — retirement of each
  checker follows the disappearance of the scripts it polices.
- Parent: epic 0136.

## Assumptions

- A residual thin shell surface remains (bootstrap, hook wrapper, Playwright
  executor); full removal of all shell is not the goal (ADR-0048 thin-wrapper
  floor).

## Technical Notes

- Anchors: `scripts/lint-bashisms.sh`; `tasks/lint/scripts.py:18,86,100`
  (SHELL_LIBRARIES, bashisms, exec-bits); `tasks/test/integration.py:8-36` (floors);
  `tasks/format/scripts.py:9` (shfmt); `tasks/lint/scripts.py:70` (shellcheck);
  `tasks/shared/sources.py:60` (`shell_sources`); `.shellcheckrc`;
  `.editorconfig:36-39`; `.github/workflows/main.yml:99` (`check-scripts`).

## Drafting Notes

- Treated as the Phase 11 cross-cutting cleanup story; much of its work lands
  incrementally inside the subdomain stories (lockstep floor decrements), with the
  final checker/CI-job removals gated on all clusters retiring — hence the broad
  `blocked_by`.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0048, ADR-0049
- Prior research: `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
