---
type: "work-item"
id: "0200"
title: "Decide whether git log/diff belong in vcs guard's blocked subcommand set"
date: "2026-08-06T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "spike"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0169", "work-item:0198"]
derived_from: ["plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
tags: ["vcs", "hooks", "guard", "cli"]
last_updated: "2026-09-10T09:11:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-730"
---

# 0200: Decide whether git log/diff belong in vcs guard's blocked subcommand set

**Kind**: Spike
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

`vcs guard` (`vcs::guard::decide`, built in 0169) blocks `git log` and
`git diff` in a pure-jj repository, denying the call and suggesting
`jj log`/`jj diff` instead — reproducing `hooks/vcs-guard.sh`'s original
13-subcommand blocklist verbatim. Both are read-only, but so is `git status`,
so read-only-ness alone is not the property that singles them out. What
distinguishes `log`/`diff` is that they neither desynchronise the jj working
copy from git's index (unlike `git add`/`git commit`) nor duplicate a jj-only
capability (unlike `git branch`); the rest of the blocklist earns its place by
steering Claude Code away from a git-shaped mental model of the working copy.
0169 declined to change this blocklist membership — parity with the shell was
in scope, not a behavioural redesign — and named this item as the place to
make the call.

## Context

The guard is a steering aid, not an access-control boundary (see
`cli/vcs/src/guard.rs`'s own threat-model note, and
`cli/vcs-cli/src/guard.rs`'s mode composition — the deny-vs-warn axis the
guard already distinguishes): its purpose is nudging Claude Code toward jj-native
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

Throughout, the decision concerns the `diff` subcommand as a whole — the guard
blocks it regardless of flags — and `--stat` is only the invocation used to
illustrate how far git's default output diverges from `jj diff`.

## Requirements

- Decide, with a stated rationale, whether `log` and `diff` stay in
  `vcs::guard`'s `BLOCKED_SUBCOMMANDS` (`cli/vcs/src/guard.rs`), move to an
  "allowed, informational suggestion only" tier if one is introduced, or are
  dropped from the blocklist entirely.
- If the decision drops `log` and/or `diff` from the blocklist outright,
  update the guard decision table fixture
  (`hooks/test-fixtures/vcs-guard/decision-table.json`) and
  `cli/vcs-cli/tests/guard_decision_table.rs` to match, and record the
  change as a fifth declared departure from shell parity (0169 named four;
  this would be the first landed after the port itself). This fixture/test
  follow-through is in scope only because 0169 engineered the change to be
  trivial (removing at most two blocklist entries); a non-trivial change is
  spun out as a separate chore rather than landed under this spike. A move to
  an informational tier is not this branch — it is handled by the out-of-scope
  carve-out below.
- If the decision is to keep both blocked, close this item with that
  rationale recorded rather than leaving the question open indefinitely.
- Time-box: one `/conduct-spike` run. If the run ends without a firm
  decision, record the current leaning and what blocks it rather than
  extending the box.
- Introducing an "allowed, informational suggestion only" tier is out of
  scope here. If the decision points to needing it, record that outcome and
  spawn a separate work item to design it — do not build the tier in this
  spike.

## Acceptance Criteria

- [ ] If a firm decision is reached, it is recorded with rationale for `log`
      and for `diff` independently (they need not land the same way). The
      rationale cites the observed `git log`/`git diff --stat` vs `jj log`/`jj
      diff` output difference; cites the warn-vs-deny axis finding if the
      decision relied on it, or records that the axis was found not to bear on
      the decision; and addresses whether the chosen treatment leaves
      `skills/planning/validate-plan` broken in pure-jj repos and whether that
      cost is outweighed.
- [ ] If either subcommand is dropped from `BLOCKED_SUBCOMMANDS` outright:
      `vcs::guard::decide`'s tests, the decision-table fixture, and
      `guard_decision_table.rs` all reflect the new expected outcome; the
      change is recorded as the fifth declared departure from shell parity in
      0169's declared-departures ledger; and `mise run` (bare default task)
      exits 0 end-to-end.
- [ ] If neither changes: the rationale for keeping shell parity here is
      recorded on this item and it is closed without a code change.
- [ ] If the decision is to move `log` and/or `diff` to an
      informational-suggestion tier: the outcome is recorded and a separate
      work item to design that tier is created and linked from this item (per
      the out-of-scope note in Requirements), without building the tier here.
- [ ] If the `/conduct-spike` run ends without a firm decision: the current
      leaning and the specific blocker are recorded on this item, per the
      time-box requirement.

## Open Questions

- Does `git log` stay blocked, move to an informational-suggestion tier, or
  drop entirely — and does `git diff` land the same way, or differently?
  (`git diff`'s "changed relative to index" default diverges from `jj diff`
  more sharply than `git log` does from `jj log`.)
- Does nudging toward a jj-native read of the repo outweigh the cost of two
  exceptions in an otherwise-uniform "the guard blocks git VCS commands"
  rule?
- Does keeping `log`/`diff` blocked leave `skills/planning/validate-plan`
  broken in pure-jj repos, and if so, does the consistency benefit outweigh
  that user-facing cost? (0169 named unblocking validate-plan as the original
  motivation for reconsidering the blocklist.)

## Dependencies

- Relates to: work-item:0169 (built the blocklist this item reconsiders;
  done, not blocked on this item). An outright-drop outcome writes a fifth
  declared departure back to 0169's departures ledger, so this spike amends a
  closed item on that path.
- Relates to: work-item:0198 (reworked the `vcs status`/`vcs log` renderer
  and the `git diff --stat` output shape this decision reasons about; done).
- Downstream consumer: `skills/planning/validate-plan` — blocked in pure-jj
  repos because `log` and `diff` sit in the guard's blocked set (per 0169);
  dropping them would unblock it. This spike's outcome gates that capability,
  so validate-plan's pure-jj status is a consequence of whichever way the
  decision lands, and is resolved when this spike closes.
- Parent: epic 0136.
- If the decision spawns a downstream item — an "allowed, informational
  suggestion only" tier, or a separate chore for a non-trivial blocklist
  change — that item is gated by this spike; record it here as a Blocks entry
  and back-link from the new item so the coupling is captured on both sides.

## Assumptions

- The guard's threat-model note (it is a steering aid, not a security
  boundary) is settled and not itself up for reconsideration here — this
  item is scoped to blocklist membership, not to whether the guard should
  exist or be hardened.

## Technical Notes

- Compare real output of `git log` / `git diff --stat` against `jj log` /
  `jj diff` in a pure-jj repo, to gauge how git-shaped the unblocked read
  actually is.
- Check whether `vcs::guard` already exposes a warn-vs-deny axis (mode
  composition in `cli/vcs-cli/src/guard.rs`) an informational tier could
  reuse, or whether that tier is net-new machinery.
- The comparison assumes both `jj` and `git` binaries are present and that a
  genuinely pure-jj state is built with `git.colocate=false` (mise-pinned jj
  defaults to `colocate=true`), so the environment prerequisite is explicit
  for whoever conducts the run.

## Drafting Notes

- Time-box read as "one `/conduct-spike` run" (author's steer), not a
  wall-clock budget.
- Informational-tier design treated as out of scope, to be spun out as a
  separate work item if the decision requires it — narrows this spike to a
  decision plus the parity-fixture update if `log`/`diff` move.

## References

- `cli/vcs/src/guard.rs` — `BLOCKED_SUBCOMMANDS`, the threat-model note
- `cli/vcs-cli/src/guard.rs` — mode composition (deny vs warn)
- `hooks/test-fixtures/vcs-guard/decision-table.json`
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- `meta/work/0198-vcs-agnostic-status-log-renderer.md` — adjacent prior art
  on the `git diff --stat` / `jj diff` output shapes
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase
  10, "Not Doing" section
