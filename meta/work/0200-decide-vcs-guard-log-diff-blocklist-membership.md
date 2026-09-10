---
type: "work-item"
id: "0200"
title: "Decide whether git log/diff belong in vcs guard's blocked subcommand set"
date: "2026-08-06T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "spike"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0169", "work-item:0198"]
derived_from: ["plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
tags: ["vcs", "hooks", "guard", "cli"]
last_updated: "2026-09-10T11:44:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-730"
---

# 0200: Decide whether git log/diff belong in vcs guard's blocked subcommand set

**Kind**: Spike
**Status**: Done
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

## Spike Outcome

**2026-09-10 — one `/conduct-spike` run, within the time-box. Verdict: keep
both `log` and `diff` in the guard's blocked set; close without a code change
(Acceptance Criterion 3).**

`log` and `diff` stay in `BLOCKED_SUBCOMMANDS` (`cli/vcs/src/guard.rs:19-22`).
No behaviour changes, so nothing is appended to 0169's "Declared behavioural
changes" ledger, no informational-suggestion tier is built, the decision-table
fixture and `guard_decision_table.rs` are untouched, and `mise run` is not
re-run because no code moved. `validate-plan`'s pure-jj brokenness is real but
is neither caused by nor fixed by this blocklist; a VCS-agnostic follow-up
(0286) owns it — see Dependencies.

## Findings

The brief's "read-only, therefore demotable" framing inverts once the two repo
modes the guard actually sees are separated. The operative axis is repository
mode, not read-only-ness — `git status` is read-only too, yet nobody proposes
dropping it.

The guard's warn-vs-deny outcome is keyed on mode, not on subcommand
(`cli/vcs-cli/src/guard.rs:71-73`): `Mode::Jj` denies, `Mode::JjColocated`
warns, `Mode::Git` never evaluates the command. `GuardDecision` itself is
binary — `Allow` / `Block` (`cli/vcs/src/guard.rs:9-17`); the deny-vs-warn
choice is made afterwards, per mode. This is the finding that splits the
picture:

| Mode | mise default? | Guard today | Can raw `git log`/`diff` run? |
|---|---|---|---|
| Pure-jj (`--no-colocate`) | No | deny | No — no `.git`; `fatal: not a git repository` |
| Colocated (`--colocate`) | Yes | warn (non-blocking) | Yes, but diverges from jj |

The mise-pinned jj defaults to `colocate=true`, so real repos are
predominantly colocated (`jj git init --help`; `CLAUDE.md`).

Empirical evidence, from throwaway probe repos (jj 0.43.0, git 2.55.0):

- Pure-jj (`jj git init --no-colocate`): no `.git` directory exists; the git
  backing store is a bare repo hidden at `.jj/repo/store/git`, unreachable from
  the working tree. `git log` exits 128 (`fatal: not a git repository`);
  `git diff` exits 129. Git cannot serve either command, guard or no guard.
- Colocated (`jj git init --colocate`): `git log --oneline` shows a *different
  commit set* than `jj log` — it omits the working-copy commit jj models as a
  real commit (one commit vs two in the probe). `git diff` is index-relative
  (unstaged changes); `jj diff` is parent-relative (the whole working-copy
  commit). The `--stat` summary lines nearly coincide, but the patch bodies
  differ entirely (git unified vs jj's `Modified regular file` line-numbered
  form).

Per-subcommand rationale — both land the same way:

- `log` — In pure-jj the deny replaces a cryptic `fatal: not a git repository`
  with `Use jj instead of git log. Equivalent: jj log`, which is strictly
  better. In colocated the git-shaped read is genuinely misleading (a different
  commit set), and the guard's response there is a non-blocking warn, the right
  severity. Keep.
- `diff` — Same pure-jj argument. In colocated the index-vs-parent semantics
  diverge more sharply than `log` does (the staging model git carries and jj
  does not), which strengthens the case for keeping `diff` blocked rather than
  weakening it. The `--stat` shape looking similar is incidental; the base of
  comparison is what differs. Keep.

The `validate-plan` cost is illusory. `validate-plan/SKILL.md:58-59` runs raw
`git log --oneline -n 20` and `git diff HEAD~N..HEAD` in a Bash block (not the
`accelerator vcs` abstraction — and there is no `vcs diff` subcommand). It is
denied only in pure-jj repos. Dropping `log`/`diff` from the blocklist would
not fix it there: the command would then run and hit `fatal: not a git
repository`, a worse failure than today's redirect. In colocated repos the warn
is non-blocking, so validate-plan already works. The blocklist is therefore not
what breaks validate-plan, and changing it fixes nothing.
`research-issue/SKILL.md:63,66` shares the same raw-git dependency and the same
consequence.

Two corrections to the brief, neither of which changes the decision:

- The "threat-model note" the brief cites in `cli/vcs/src/guard.rs` (Assumptions
  and References) does not exist in the code — a grep for steering, security, or
  boundary language finds nothing there. The guard's steering-not-security
  character is real, evidenced by the mode-keyed warn/deny design, but the
  `file:line` citation is stale.
- The decision-table fixture has moved to
  `cli/vcs-test-support/fixtures/vcs-guard/decision-table.json` (138 rows); the
  brief's `hooks/test-fixtures/…` path is stale. Moot here, since keep-both
  touches no fixture.

## Recommendation

Keep both `log` and `diff` in `BLOCKED_SUBCOMMANDS`, unchanged. Close this item
under Acceptance Criterion 3: shell parity retained, no code change.

The real remedy for `validate-plan` (and `research-issue`) is orthogonal — make
them VCS-agnostic by adding an `accelerator vcs diff` subcommand and repointing
the skills onto `vcs log`/`vcs diff`, exactly as `skills/vcs/commit/SKILL.md`
already uses `vcs status`/`vcs log`. That is spun out as work item 0286
(recorded under Dependencies as a Blocks entry), not built here.

## Residual Risks & Open Questions

- `validate-plan` and `research-issue` stay broken in pure-jj repos until 0286
  lands. This is not a regression — it is the pre-existing state 0169 declined
  to change — and dropping the blocklist would make it worse, not better.
- Revisit the blocklist only if a future `vcs diff` implementation still needed
  a raw `git` invocation to reach jj's backing store. It will not:
  `vcs status`/`vcs log` already shell the real binaries under a controlled
  environment (0198), and `vcs diff` would do the same.
- The colocated warn on `log`/`diff` is a deliberate nudge, not a bug: the
  commit-set and index-vs-parent divergences above are exactly the git-shaped
  read it steers away from.

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
- Blocks: work-item:0286 (make `validate-plan`/`research-issue` VCS-agnostic
  via a new `accelerator vcs diff` subcommand and a repoint of the skills) —
  spawned by this spike's outcome; it is the real remedy for the pure-jj
  brokenness that keeping `log`/`diff` blocked leaves in place. Back-linked
  from 0286.
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
