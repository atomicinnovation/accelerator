---
type: work-item
id: "0199"
title: "Retire scripts/vcs-common.sh's residual shell callers and hooks/launcher-link-refresh.sh"
date: "2026-08-06T00:00:00+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
parent: "work-item:0136"
relates_to: ["work-item:0169", "work-item:0125", "work-item:0172"]
derived_from: ["plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
tags: [rust, migration, vcs, shell, cli]
last_updated: "2026-08-06T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0199: Retire scripts/vcs-common.sh's residual shell callers and hooks/launcher-link-refresh.sh

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

0169 built `accelerator vcs detect|status|log|guard` on the library-backed VCS
adapter and retired the two shell hooks that used to call
`scripts/vcs-common.sh`'s `classify_checkout`/`find_repo_root`/`vcs_mode` for
the SessionStart and PreToolUse hooks. It deliberately left two things
untouched: `find_repo_root`/`vcs_mode` themselves, which still have roughly
twenty other shell callers across `scripts/` and `skills/`; and
`hooks/launcher-link-refresh.sh`, a SessionStart hook unrelated to VCS
detection that has not been ported to Rust at all. This item owns both —
bundled because they are the two named pieces of shell surface 0169's own
"What We're NOT Doing" section explicitly deferred, not because they are
technically related to each other.

## Context

`scripts/vcs-common.sh`'s `classify_checkout` (the seven-arm taxonomy) is now
dead weight for the two consumers 0169 replaced, but `find_repo_root` and
`vcs_mode` — the two-function subset 0125 originally scoped to converge —
remain load-bearing for their other callers. 0125's 2026-08-06 amendment
records that this item is their designated successor: 0125 itself is not
closed by 0169, and its remaining scope (the ~20 callers, `vcs_mode`'s
`-d`/`.git`-as-file blind spot on any of them) is what this item inherits.

`hooks/launcher-link-refresh.sh` is a separate concern: it keeps
`${CLAUDE_PLUGIN_DATA}/bin/accelerator` pointing at the current
installation's launcher across upgrades. It has never depended on
`scripts/vcs-common.sh`. Whether it should move into the CLI (matching the
epic-0136 direction every other hook has taken) or stay a self-contained
shell script (it already self-locates by absolute path rather than reading
`CLAUDE_PLUGIN_ROOT`, unlike its siblings) is an open question this item
should resolve rather than assume.

## Requirements

- Inventory every remaining caller of `find_repo_root` and `vcs_mode` across
  `scripts/` and `skills/`, and decide, per caller, whether it should move
  onto the library-backed adapter (via a new or existing `accelerator vcs`
  subcommand) or stay shell — not every caller necessarily needs the
  `.git`-as-file correction or the performance win.
- Decide whether `classify_checkout` (now unused by any surviving `hooks/`
  consumer) should be deleted from `scripts/vcs-common.sh`, or kept for the
  callers this item's inventory finds, if any.
- Decide whether `hooks/launcher-link-refresh.sh` ports onto the CLI
  (matching the shape 0169 established: a shared `kernel::hooks` envelope,
  dispatched via `hooks.json`) or stays a standalone shell script, and
  implement whichever is decided.
- If `find_repo_root`/`vcs_mode` callers move, repoint
  `scripts/test-vcs-common.sh` (0169's split-off in-process test suite)
  accordingly, or retire it if nothing remains to test.

## Acceptance Criteria

- [ ] The caller inventory is recorded, with a migrate/keep decision per
      caller.
- [ ] Every caller decided to migrate now calls the Rust adapter (directly or
      through a subcommand), with parity fixtures matching 0169's own
      pattern (masked goldens, quote-aware/behaviour-preserving departures
      declared explicitly where taken).
- [ ] `classify_checkout`'s fate (deleted or retained-with-named-callers) is
      explicit, not left ambiguous.
- [ ] `hooks/launcher-link-refresh.sh`'s fate (ported or explicitly kept
      shell, with a reason) is explicit.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- Blocked by: work-item:0169 (done) — establishes the library-backed adapter
  pattern this item's migrated callers would follow.
- Relates to: work-item:0125 (the item this residue was originally scoped
  under; not closed, this item inherits its remaining surface).
- Parent: epic 0136.

## Assumptions

- Not every remaining `find_repo_root`/`vcs_mode` caller necessarily
  benefits from migration — some may be low-frequency enough, or run in
  contexts (e.g. already inside a hook that has no Rust binary to shell
  out to) where the shell function is simpler than adding a new
  `accelerator vcs` subcommand just to serve it. The inventory step should
  make this judgement per caller, not assume uniform migration.

## References

- `scripts/vcs-common.sh`
- `hooks/launcher-link-refresh.sh`
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- `meta/work/0125-converge-vcs-detection-on-probe-layer.md` — 2026-08-06
  amendment records this item as its successor
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase
  10, "Not Doing" section
