---
work_item_id: "0030"
title: "Centralise PATH and TEMPLATE config arrays to scripts/config-defaults.sh"
date: "2026-04-26T00:00:00+00:00"
author: Toby Clemson
type: task
status: ready
priority: low
parent: ""
tags: [config, refactoring]
---

# 0030: Centralise PATH and TEMPLATE config arrays to scripts/config-defaults.sh

**Type**: Task
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

The `PATH_KEYS`, `PATH_DEFAULTS`, and `TEMPLATE_KEYS` arrays are defined inline
in `scripts/config-dump.sh`, with a further 13 consumer bash scripts (and ~25
SKILL.md exec blocks) that reference the same key names with hardcoded default
strings. Extracting the array definitions into a single sourced file makes the
next default rename a one-line edit at the definition site rather than a
grep-and-replace across the consumer surface.

## Context

During the `0001-rename-tickets-to-work` migration (tracked in
`meta/plans/2026-04-25-rename-tickets-to-work-items.md`), `paths.tickets` /
`paths.review_tickets` had to be updated in `scripts/config-dump.sh`,
`scripts/config-read-path.sh`, multiple SKILL.md files, and several helper
scripts. A single canonical defaults file would have reduced the definition-side
edit to one location.

This task centralises only the array **definitions**. Consumer scripts that
embed `<key> <default>` pairs at call sites are out of scope (see Assumptions).

## Requirements

- Extract `PATH_KEYS`, `PATH_DEFAULTS`, and `TEMPLATE_KEYS` arrays from
  `scripts/config-dump.sh` into a new file `scripts/config-defaults.sh`.
- Source the new file from `scripts/config-common.sh` near the existing
  `source "$SCRIPT_DIR/vcs-common.sh"` line, so the arrays are transitively
  available to `config-dump.sh` (and any future caller of `config-common.sh`)
  without further per-file edits. Use the existing `$SCRIPT_DIR` self-resolution
  pattern: `source "$SCRIPT_DIR/config-defaults.sh"`. Do not use
  `${CLAUDE_PLUGIN_ROOT}` — it is not reliably set under `mise run test` or
  direct CLI invocation, and the existing config scripts don't depend on it.
- Ensure no change in observable behaviour: the config-dump and path-resolution
  test suites continue to pass.

## Acceptance Criteria

- Given `scripts/config-defaults.sh` is created, when the file is inspected,
  then it defines `PATH_KEYS`, `PATH_DEFAULTS`, and `TEMPLATE_KEYS` arrays with
  the same entries (in the same order) as the pre-migration definitions in
  `scripts/config-dump.sh:175-219`. (`TEMPLATE_DEFAULTS` is out of scope; see
  Assumptions for rationale.)
- Given the migration is complete, when
  `grep -rn --include='*.sh' 'PATH_KEYS=\|PATH_DEFAULTS=\|TEMPLATE_KEYS='`
  is run from the repo root excluding `workspaces/` (which contains jj
  workspace checkouts, not source files), then the only matching file is
  `scripts/config-defaults.sh`. The single definition site uses bare `=`
  assignment form — see Technical Notes. `declare -a` forms are not present.
- Given the migration is complete, when `mise run test:integration:config` is
  executed, then the config-dump tests
  (`scripts/test-config.sh:2426-2555`) and path-resolution tests
  (`scripts/test-config.sh:2606-2761`) pass. (Note: config-init tests in
  `skills/config/init/scripts/test-init.sh` are not currently picked up by
  any `mise run test` task — that's a pre-existing harness gap unrelated to
  this work item, and `init.sh` is not modified here.)

## Open Questions

- Should `DIR_KEYS`/`DIR_DEFAULTS` in `skills/config/init/scripts/init.sh` also
  source `config-defaults.sh`, or remain independent? These arrays use a
  different key shape (bare keys, not `paths.`-prefixed) and include keys that
  `PATH_KEYS` does not (`design_inventories`, `design_gaps`) — see Technical
  Notes. `init.sh` also does not source `config-common.sh`, so making the
  shared arrays available there would require an additional source line plus a
  shape adapter. This decision is deferred to a follow-on task (work item to
  be created); resolving it is not required before 0030 closes.

## Dependencies

- Blocked by: none
- Blocks: 0052 (make-documents-locator-paths-config-driven depends on
  `config-defaults.sh` existing)
- Triggers follow-up in 0052: once this task lands, the path-key enumeration in
  0052 should be updated to source `config-defaults.sh` rather than maintaining
  its own list.

## Assumptions

- The single array-definition site (`scripts/config-dump.sh`) is a bash script
  that already sources `config-common.sh`, so adding a `source` of a sibling
  defaults file from `config-common.sh` is the convention-fit insertion point.
- The 13 bash consumer scripts and ~25 SKILL.md exec blocks that reference path
  keys with inline `<key> <default>` strings are **not modified** by this
  migration. Their correctness is exercised by `scripts/test-config.sh`
  (lines 2606-2761 cover all path-key resolutions; lines 3039-3194 verify
  SKILL.md `config-read-path.sh` references).
- `TEMPLATE_KEYS` will be co-located in `scripts/config-defaults.sh` alongside
  the path arrays. `TEMPLATE_DEFAULTS` does not exist as an array — template
  fallback uses a three-tier function lookup (`config_resolve_template()` in
  `scripts/config-common.sh:188-227`), so there is nothing to extract.
- The `workspaces/` directory contains jj workspace checkouts (confirm with
  `jj workspace list`), not duplicate source. Files inside `workspaces/*/` are
  parallel working copies of the same repository at potentially different
  revisions, and are excluded from the migration scope.

## Technical Notes

**Array definition site** (the one file to migrate):
- `scripts/config-dump.sh:175-187` — `PATH_KEYS` (11 entries: `paths.plans`,
  `paths.research`, `paths.decisions`, `paths.prs`, `paths.validations`,
  `paths.review_plans`, `paths.review_prs`, `paths.review_work`,
  `paths.templates`, `paths.work`, `paths.notes`)
- `scripts/config-dump.sh:189-201` — `PATH_DEFAULTS` (paired by index:
  `meta/plans`, `meta/research`, `meta/decisions`, `meta/prs`,
  `meta/validations`, `meta/reviews/plans`, `meta/reviews/prs`,
  `meta/reviews/work`, `.accelerator/templates`, `meta/work`, `meta/notes`)
- `scripts/config-dump.sh:212-219` — `TEMPLATE_KEYS` (6 entries:
  `templates.plan`, `templates.research`, `templates.adr`,
  `templates.validation`, `templates.pr-description`, `templates.work-item`)

All three definitions use the bare `=` array-literal form (no `declare -a`,
no associative arrays).

**Recommended sourcing pattern**: add
`source "$SCRIPT_DIR/config-defaults.sh"` to `scripts/config-common.sh` near
line 8 (after the existing `source "$SCRIPT_DIR/vcs-common.sh"`). Because
`config-dump.sh:12` already does `source "$SCRIPT_DIR/config-common.sh"`, the
arrays propagate transitively without further edits to `config-dump.sh` beyond
deleting the now-redundant inline definitions.

**`TEMPLATE_DEFAULTS` does not exist**: template resolution uses
`config_resolve_template()` in `scripts/config-common.sh:188-227`; there is
no defaults array for template keys — their fallback is a three-tier
config/user/plugin lookup, not a simple parallel array.

**`scripts/config-read-path.sh:7-21`**: comment-only enumeration; no
executable array definitions. All call-site defaults are inline string
arguments.

**`skills/config/init/scripts/init.sh:18-29`**: defines independent
`DIR_KEYS`/`DIR_DEFAULTS` arrays using a bare-key vocabulary (`plans`, not
`paths.plans`) and including `design_inventories` / `design_gaps` keys that
are absent from `PATH_KEYS`. `init.sh` does not source `config-common.sh`.
This is out of scope for this task — see Open Questions for the deferred
decision.

## Drafting Notes

- Scope expanded from original draft to include `TEMPLATE_KEYS` in the same
  centralisation — this was an open question resolved during enrichment.
  `TEMPLATE_DEFAULTS` was also proposed but confirmed not to exist as an
  array; template fallback uses `config_resolve_template()` instead.
- An earlier draft and the four review passes framed the migration as
  "4 parallel `config-dump.sh` copies" across the root and three sub-workspaces
  — this was a misreading of the `workspaces/` directory, which contains jj
  workspace **checkouts** (parallel working copies of the same repository),
  not duplicate source files. Corrected by research dated 2026-05-08:
  `meta/research/2026-05-08-0030-centralise-path-defaults-implementation.md`.
- The "11 consumer scripts" / "15 sites in total" figure in earlier drafts was
  also stale. Re-counted at 13 bash consumers (+ ~25 SKILL.md exec blocks);
  consumer-site refactoring remains out of scope per Assumptions.

## References

- Source: `meta/plans/2026-04-25-rename-tickets-to-work-items.md`
- Implementation research:
  `meta/research/2026-05-08-0030-centralise-path-defaults-implementation.md`
- Related: `meta/decisions/ADR-0023-meta-directory-migration-framework.md` —
  migration framework that established the `config-dump.sh` structure being
  consolidated here.
