---
type: work-item
id: "0193"
title: "Decide whether git log/diff belong in vcs guard's blocked subcommand set"
date: "2026-08-06T00:00:00+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: spike
priority: low
parent: "work-item:0136"
relates_to: ["work-item:0169"]
derived_from: ["plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
tags: [vcs, hooks, guard, cli]
last_updated: "2026-08-06T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0193: Decide whether git log/diff belong in vcs guard's blocked subcommand set

**Kind**: Spike
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

`vcs guard` (`vcs::guard::decide`, built in 0169) blocks `git log` and
`git diff` in a pure-jj repository, denying the call and suggesting
`jj log`/`jj diff` instead — reproducing `hooks/vcs-guard.sh`'s original
13-subcommand blocklist verbatim. Both are read-only commands with no
jj-workspace side effects, unlike `git status`/`git add`/`git commit`, whose
blocking exists to steer Claude Code away from a git-shaped mental model of
the working copy. 0169 declined to change this blocklist membership — parity
with the shell was in scope, not a behavioural redesign — and named this
item as the place to make the call.

## Context

The guard is a steering aid, not an access-control boundary (see
`cli/vcs/src/guard.rs`'s own threat-model note, and `vcs-cli/src/guard.rs`'s
mode composition): its purpose is nudging Claude Code toward jj-native
commands, not preventing any particular git invocation a determined caller
could still reach another way. Against that purpose, `git log`/`git diff`
are a different case from the other eleven blocked subcommands: they cannot
desynchronise the jj working copy from git's index the way `git add`/
`git commit`/`git checkout`/`git reset`/`git stash` can, and they do not
duplicate a jj-only capability the way `git branch` does (jj has no
branches, only bookmarks). A plausible case exists for demoting them to
"allowed, no suggestion" or "allowed with an informational note" rather
than "blocked."

The counter-case: consistency. A user who has internalised "the guard blocks
git VCS commands" now has to remember two of the thirteen are exceptions,
and `git log`/`git diff --stat` genuinely do produce different output than
`jj log`/`jj diff` (different default formatting, different defaults for
what "changed" means relative to the working copy), so even a read-only
command steers toward a git-shaped read of the repository if left unblocked.

## Requirements

- Decide, with a stated rationale, whether `log` and `diff` stay in
  `vcs::guard`'s `BLOCKED_SUBCOMMANDS` (`cli/vcs/src/guard.rs`), move to an
  "allowed, informational suggestion only" tier if one is introduced, or are
  dropped from the blocklist entirely.
- If the decision changes behaviour, update the guard decision table fixture
  (`hooks/test-fixtures/vcs-guard/decision-table.json`) and
  `cli/vcs-cli/tests/guard_decision_table.rs` to match, and record the
  change as a fifth declared departure from shell parity (0169 named four;
  this would be the first landed after the port itself).
- If the decision is to keep both blocked, close this item with that
  rationale recorded rather than leaving the question open indefinitely.

## Acceptance Criteria

- [ ] A decision is recorded, with rationale, for `log` and for `diff`
      independently (they need not land the same way).
- [ ] If either subcommand's treatment changes: `vcs::guard::decide`'s tests,
      the decision-table fixture, and `guard_decision_table.rs` all reflect
      the new expected outcome; `mise run` (bare default task) exits 0
      end-to-end.
- [ ] If neither changes: the rationale for keeping shell parity here is
      recorded on this item and it is closed without a code change.

## Dependencies

- Relates to: work-item:0169 (built the blocklist this item reconsiders;
  done, not blocked on this item).
- Parent: epic 0136.

## Assumptions

- The guard's threat-model note (it is a steering aid, not a security
  boundary) is settled and not itself up for reconsideration here — this
  item is scoped to blocklist membership, not to whether the guard should
  exist or be hardened.

## References

- `cli/vcs/src/guard.rs` — `BLOCKED_SUBCOMMANDS`, the threat-model note
- `cli/vcs-cli/src/guard.rs` — mode composition (deny vs warn)
- `hooks/test-fixtures/vcs-guard/decision-table.json`
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase
  10, "Not Doing" section
