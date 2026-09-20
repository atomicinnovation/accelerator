---
type: "work-item"
id: "0286"
title: "Make Accelerator skills VCS-agnostic by eradicating direct git calls"
date: "2026-09-10T11:44:31+00:00"
author: "Toby Clemson"
producer: "conduct-spike"
status: "ready"
kind: "story"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0169", "work-item:0198", "work-item:0200"]
tags: ["vcs", "skills", "cli"]
last_updated: "2026-09-20T21:20:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0286: Make Accelerator skills VCS-agnostic by eradicating direct git calls

**Kind**: Story
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer running Accelerator skills in a pure-jj or colocated-jj
repository, I want every skill to express VCS operations in VCS-agnostic terms
so that each one behaves identically under git, colocated jj, and pure-jj. One
skill breaks in pure-jj repositories and two others name `git` where they
should defer to the session VCS: `validate-plan` gathers implementation
evidence with raw `git log`, `git diff`, and `git rev-parse`, all of which
break where there is no `.git`; `config/migrate` and `refine-work-item` each
name `git` in places that already prefer jj but are not fully backend-neutral.
Rewrite all three to be VCS-agnostic, expose the already-existing
backend-neutral repository-root capability as an `accelerator vcs root` command
for `validate-plan`'s repo-root lookup, and extend the SessionStart VCS Command
Reference with the diff-range and user-identity idioms the skills lean on.

## Context

Spawned by spike 0200, which decided to keep `log` and `diff` in the guard's
blocklist and established that the blocklist is not what fundamentally breaks
these skills: in a pure-jj checkout there is no `.git`, so raw `git` commands
fail with `fatal: not a git repository` regardless of the guard. The guard
compounds this by denying `git log`/`git diff` outright in pure-jj mode,
warning in colocated mode, and allowing them in git-only mode
(`cli/vcs-cli/src/guard.rs`).

The remedy is to stop issuing raw `git` from the skills. For the diff and the
status/log prose this is a phrasing change resolved by the session VCS: the
SessionStart VCS Command Reference already tells the model which backend
command to use per session (emitted by `accelerator vcs detect --descriptive`,
wired through `hooks/hooks.json`), with the guard as a backstop.
`research-issue` already follows this pattern — it speaks of "the session's VCS
log/diff command" and issues no raw git — so it is the template the other
skills should match.

Repository-root discovery is the exception. Spike 0200 rejected a broad
`accelerator vcs diff` subcommand, and that rejection stands for the diff. But
the rejection's premise — that no deterministic CLI is warranted — is weaker
than it read: the `vcs-adapters` crate already carries a backend-neutral
`repository_root` (and `jj_workspace_root`), used internally by
`accelerator vcs detect` to find the workspace boundary, and the deterministic
`accelerator vcs status`/`accelerator vcs log` commands already exist and are
injected via the `!` preprocessor by `skills/vcs/commit`. Exposing the existing
`repository_root` as a thin `accelerator vcs root` command therefore adds no
new logic, only a command-line surface, and gives `validate-plan` a
backend-neutral root without a raw `git rev-parse`. The broader model-driven
diff stays model-driven, phrased against the trunk divergence point.

A plugin-wide sweep found the full extent:

- `validate-plan` (lines 58, 59, 62) — `git log --oneline`,
  `git diff HEAD~N..HEAD`, and `git rev-parse --show-toplevel`. All break in
  pure-jj. `rev-parse` is not in the guard blocklist, so its failure is purely
  the absent `.git`, not a denial — a breakage the item's original framing
  missed.
- `config/migrate` (line 273) — user-facing prose offering
  `` `jj status`/`git status` ``. It does not execute and already leads with
  jj, but names git.
- `refine-work-item` (line 249) — a `git config user.name` fallback in the
  child-author resolution chain, after `jj config get user.name`. It reads
  global git config, so it works without a `.git` and never breaks; it is
  load-bearing for git-only repositories.
- `research-issue`, and all agents, templates, and hooks — clean.

## Requirements

- Expose the existing `vcs-adapters` `repository_root` capability as an
  `accelerator vcs root` subcommand that prints the checkout's repository root
  and resolves identically under git, colocated jj, and pure-jj. Route it
  through the launcher like the existing `vcs status`/`vcs log` commands so it
  is reachable under the general `Bash(accelerator ...)` permission.
- Rewrite `validate-plan`'s evidence-gathering so it issues no raw `git log`,
  `git diff`, or `git rev-parse`: recent commits and the implementation diff
  resolve to the session's VCS commands, and the step that runs the local
  checks from the repository root discovers that root via `accelerator vcs
  root` instead of `git rev-parse --show-toplevel`.
- Express `validate-plan`'s implementation diff as the cumulative change
  between the trunk divergence point and the working-copy commit, not a
  git-specific range — git renders this as `main...HEAD` (merge-base) and jj as
  a `fork_point(trunk())`-anchored revset, so the model reconstructs the
  correct anchor per backend from one documented idiom.
- Extend the SessionStart VCS Command Reference with two idioms so skills lean
  on one documented vocabulary rather than each phrasing it independently: a
  diff-range idiom (the trunk-divergence cumulative change, with the git and jj
  renderings) and a user-identity idiom (the session VCS user identity, with
  the git and jj renderings).
- Reword `config/migrate`'s dirty-path confirmation guidance to name the
  session's VCS status command rather than `git status` specifically, leaving
  no literal `git status`.
- Make `refine-work-item`'s child-author resolution backend-neutral without
  regressing git-only repositories: collapse the current `jj config get
  user.name`-then-`git config user.name` chain into a single session VCS user
  identity lookup driven by the reference idiom, which yields the jj identity in
  a jj session and the git identity in a git-only session, so the git-only
  identity source is preserved through the reference, not deleted.
- Leave `research-issue` unchanged; include it only as a verification target.

## Acceptance Criteria

- [ ] A sweep of `skills/` for `git <subcommand>` where the subcommand is one
      of `status`, `diff`, `add`, `commit`, `log`, `branch`, `checkout`,
      `switch`, `merge`, `rebase`, `reset`, `stash`, `show`, `rev-parse`, or
      `config` returns no command invocation, and a reviewer confirms every
      remaining VCS reference in the affected skills names the session's VCS
      command or the SessionStart VCS Command Reference rather than a backend.
- [ ] `accelerator vcs root` prints the checkout's repository root and exits 0
      under git-only, colocated jj, and pure-jj, each matching the checkout's
      actual root.
- [ ] The SessionStart VCS Command Reference emitted by `accelerator vcs detect
      --descriptive` carries both new idioms: a diff-range idiom rendering the
      trunk-divergence cumulative change as `main...HEAD` for git and the
      `fork_point(trunk())`-anchored revset for jj, and a user-identity idiom
      rendering `git config user.name` for git and `jj config get user.name`
      for jj.
- [ ] Given a fixture repository with at least one implementation commit ahead
      of trunk, `validate-plan` emits a non-empty recent-commit list and a
      non-empty implementation diff, with no `fatal: not a git repository`,
      under git-only, colocated jj, and pure-jj — pure-jj being the
      previously-broken case.
- [ ] `validate-plan`'s implementation diff equals the cumulative change
      between the trunk divergence point and the working-copy commit under
      git-only, colocated jj, and pure-jj, verified against a fixture carrying
      three implementation commits — git-only via `git diff main...HEAD`;
      colocated and pure-jj via `jj diff --from 'fork_point(trunk())' --to @`.
- [ ] `config/migrate`'s dirty-path confirmation guidance names the session VCS
      status command and contains no literal `git status`.
- [ ] `refine-work-item` resolves a child work item's author to the configured
      identity in a jj repository (fixture: `jj config user.name` set to a known
      value, no git identity → that value) and in a git-only repository
      (fixture: `git config user.name` set, no jj → that value), resolving the
      git-only case via the session VCS user identity idiom with no hard-coded
      `git config` call in the skill.
- [ ] `research-issue` gathers evidence successfully — a non-empty log and diff
      with no `fatal: not a git repository` — in a pure-jj repository
      (regression check; no code change expected).
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

- None. The three prior design questions — the diff-range anchor, repo-root
  discovery, and whether to extend the SessionStart VCS Command Reference —
  were resolved during review 1; see Drafting Notes for the decisions and their
  rationale.

## Assumptions

- `research-issue`'s existing session-VCS phrasing is precise enough for the
  model to act on reliably and needs no change. If it proves too vague in
  practice, scope expands to harden its wording.
- Extending the SessionStart VCS Command Reference with the diff-range and
  identity idioms will steer the model to the correct backend command
  reliably, with the guard as a backstop. If that steering proves unreliable,
  the same deterministic-CLI route taken for `vcs root` can be extended to the
  diff.
- `refine-work-item`'s git-config fallback exists to serve git-only
  repositories; preserving that behaviour through the reference's identity
  idiom — not merely deleting the git call — is in scope.

## Technical Notes

- `validate-plan` raw VCS usage: `git log --oneline -n 20` (line 58),
  `git diff HEAD~N..HEAD` (line 59), `git rev-parse --show-toplevel`
  (line 62), plus soft prose "through git" (line 48).
- `repository_root` already exists and is backend-neutral: `vcs-adapters`
  exposes `repository_root`/`jj_workspace_root`, exercised by
  `cli/vcs-adapters/tests/library.rs` and consumed by `accelerator vcs detect`.
  The `vcs root` command surfaces it — new argv parsing in
  `cli/vcs-cli/src/cli.rs` and a handler, no new resolution logic. Registering
  the subcommand follows the dispatched-sub-binary checklist in
  `tasks/README.md`.
- Deterministic-CLI precedent: `accelerator vcs status`
  (`jj status` / `git diff --cached --stat`) and `accelerator vcs log`
  (`jj log --limit 5` / `git log --oneline -5`) already exist in
  `cli/vcs-cli/src/cli.rs` and are `!`-injected by `skills/vcs/commit`.
- SessionStart VCS Command Reference: emitted by
  `accelerator vcs detect --descriptive` (the cheat-sheet flag in
  `cli/vcs-cli/src/cli.rs`), wired through `hooks/hooks.json`. This is where
  the diff-range and identity idioms are added, and where the git and jj
  identity commands live once `refine-work-item` stops naming them directly.
- Guard blocklist (`cli/vcs/src/guard.rs`): status, diff, add, commit, log,
  branch, checkout, switch, merge, rebase, reset, stash, show. `rev-parse` and
  `config` are absent, so their pure-jj behaviour is about `.git` presence, not
  guard denial.
- Guard behaviour by mode (`cli/vcs-cli/src/guard.rs`): git-only allows
  unconditionally without evaluating; colocated warns (command still runs);
  pure-jj denies. Suggested equivalents are raw jj (`jj diff`, `jj log`).
- Pattern to mirror: `research-issue` — "the session's VCS log/diff command",
  no raw git.
- allowed-tools: `validate-plan` and `research-issue` list only
  `Bash(accelerator ...)`, which already covers `accelerator vcs root`; no
  git/jj allowed-tools entry is required.
- Backend equivalents for reference: `jj log`, `jj diff` / `jj diff --from
  --to` / `jj diff -r <revset>`, `jj workspace root`, `jj config get
  user.name`.

## Dependencies

- Blocked by: none — spike 0200, the original blocker, is complete.
- Component coupling: the SessionStart VCS Command Reference is the load-bearing
  mechanism for the diff-range and identity phrasing; it is produced outside
  `skills/` by `accelerator vcs detect --descriptive`
  (`cli/vcs-cli`, `hooks/hooks.json`). The reference extension and the
  `accelerator vcs root` command must land before, or with, the skill rewrites
  that depend on them.
- Relates to: work-item:0198 (VCS-agnostic status/log renderer and the
  session-VCS precedent), work-item:0200 (the spike that decided keep-both and
  spawned this), work-item:0169 (built the guard and its blocklist).
- Parent: epic 0136.

## Drafting Notes

- Review 1 resolved the three prior Open Questions and expanded scope from
  prose-only back into the CLI:
  - Diff-range anchor: the cumulative change since the trunk divergence point
    (git `main...HEAD`; jj `fork_point(trunk())`), chosen over the plan's first
    commit (fragile to squash/reorder) and a named bookmark (needs consistent
    naming). The diff stays model-driven, backed by a new reference idiom.
  - Repo-root discovery: expose the existing `repository_root` as
    `accelerator vcs root` rather than making the checks step path-independent,
    since the backend-neutral capability already exists and only needs a CLI
    surface.
  - Reference extension: in scope for this item — add the diff-range and
    user-identity idioms so skills share one documented vocabulary.
- The `cli` tag is re-added: review 1 reversed the "no CLI touched" decision for
  repository-root discovery. The reversal is deliberately narrow — `vcs root`
  surfaces existing logic; the diff remains model-driven, so 0200's rejection of
  a broad `accelerator vcs diff` still stands.
- The bundling of the one real breakage fix (`validate-plan` under pure-jj) with
  the two never-breaking rewords (`config/migrate`, `refine-work-item`) is kept
  intentionally: the plugin-wide "no direct git token in `skills/`" invariant is
  the unit of value, now anchored by the `vcs root` command and the reference
  extension.
- `research-issue` is treated as already-migrated (verify-only) — it contains
  no raw git.
- `refine-work-item`'s change preserves git-only author resolution by moving the
  backend knowledge into the reference's identity idiom, not by deleting the
  git fallback; deleting it would regress identity resolution in git-only
  repositories.
- Surfaced a breakage the original item missed: `validate-plan`'s
  `git rev-parse --show-toplevel` (line 62) also fails in pure-jj.
- Corrected stale metadata: `research-issue` no longer carries raw git at lines
  63/66 (the original References were stale); `blocked_by: work-item:0200`
  cleared because 0200 is done.

## References

- `skills/planning/validate-plan/SKILL.md` — raw `git log`/`git diff`/`git
  rev-parse` at lines 58, 59, 62 (the evidence-gathering block to rewrite)
- `skills/research/research-issue/SKILL.md` — already session-VCS at lines 63,
  67; the pattern to mirror
- `skills/config/migrate/SKILL.md` — `git status` prose at line 273 to
  neutralise
- `skills/work/refine-work-item/SKILL.md` — `git config user.name` author
  fallback at line 249 to reframe onto the reference's identity idiom
- `skills/vcs/commit/SKILL.md` — the `!`-injection precedent for
  `vcs status`/`vcs log`, the pattern `vcs root` extends
- `cli/vcs-cli/src/cli.rs` — the `accelerator-vcs` command surface (`detect`,
  `status`, `log`, `guard`) that gains `root`, and the
  `detect --descriptive` cheat sheet to extend
- `cli/vcs-adapters` — the existing backend-neutral `repository_root`
  capability that `vcs root` surfaces
- `cli/vcs/src/guard.rs`, `cli/vcs-cli/src/guard.rs` — the guard, its
  blocklist, and per-mode deny/warn behaviour
- `hooks/hooks.json` — wires `vcs detect` into SessionStart, where the reference
  is emitted
- `meta/work/0200-decide-vcs-guard-log-diff-blocklist-membership.md` — the
  spike that decided keep-both and spawned this (done)
- `meta/work/0198-vcs-agnostic-status-log-renderer.md` — the VCS-agnostic
  renderer and session-VCS precedent (done)
- `meta/reviews/work/0286-eradicate-direct-git-calls-from-skills-review-1.md` —
  review 1, which resolved the design questions and expanded scope to `vcs root`
