---
type: "work-item"
id: "0286"
title: "Make Accelerator skills VCS-agnostic by eradicating direct git calls"
date: "2026-09-10T11:44:31+00:00"
author: "Toby Clemson"
producer: "conduct-spike"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0169", "work-item:0198", "work-item:0200"]
tags: ["vcs", "skills"]
last_updated: "2026-09-20T20:34:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0286: Make Accelerator skills VCS-agnostic by eradicating direct git calls

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Several Accelerator skills issue raw `git` commands that fail in pure-jj
repositories, and others name `git` where they should defer to the session
VCS. `validate-plan` gathers implementation evidence with raw `git log`,
`git diff`, and `git rev-parse`, all of which break under pure-jj;
`config/migrate` and `refine-work-item` each name `git` in places that
already prefer jj but are not fully backend-neutral. Rewrite all three to
express VCS operations in general terms that resolve to the session's VCS
commands — no new CLI — so every skill works identically under git,
colocated jj, and pure-jj.

## Context

Spawned by spike 0200, which decided to keep `log` and `diff` in the guard's
blocklist and established that the blocklist is not what fundamentally breaks
these skills: in a pure-jj checkout there is no `.git`, so raw `git` commands
fail with `fatal: not a git repository` regardless of the guard. The guard
compounds this by denying `git log`/`git diff` outright in pure-jj mode,
warning in colocated mode, and allowing them in git-only mode
(`cli/vcs-cli/src/guard.rs`).

The remedy is to stop issuing raw `git` from the skills, not to add tooling.
The guard's own suggested equivalents are raw `jj` commands (`jj diff`,
`jj log` — `cli/vcs/src/guard.rs`), and the SessionStart VCS Command
Reference already tells the model which backend command to use per session.
`research-issue` already follows this pattern — it speaks of "the session's
VCS log/diff command" and issues no raw git — so it is the template the other
skills should match. An earlier proposal to add an `accelerator vcs diff`
subcommand was rejected: its only real advantage is `!`-preprocessor
injection at invocation time (as `skills/vcs/commit` uses for `vcs
status`/`vcs log`), and none of the affected skills gather VCS data that way —
they all do it at runtime, model-driven, where the session reference plus the
guard backstop already suffice.

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

- Rewrite `validate-plan`'s evidence-gathering (recent commits, implementation
  diff, and repo-root discovery for the checks step) in general VCS terms that
  resolve to the session's VCS commands; issue no raw `git log`, `git diff`,
  or `git rev-parse`.
- Express the implementation diff by intent — the cumulative change across the
  implementation commits — rather than a git-specific range, since git's
  `HEAD~N..HEAD` and jj's revset/`--from --to` syntax diverge and the model
  must reconstruct the correct range per backend.
- Replace `validate-plan`'s `git rev-parse --show-toplevel` with a
  VCS-agnostic means of locating the repository root, or make the checks
  invocation path-independent so no root lookup is needed.
- Reword `config/migrate`'s dirty-path confirmation guidance to name the
  session's VCS status command rather than `git status` specifically.
- Make `refine-work-item`'s child-author resolution backend-neutral without
  regressing git-only repositories — the current `git config user.name` leg is
  the identity source when jj is absent, so it must be reframed as "the session
  VCS user identity", not deleted.
- Leave `research-issue` unchanged; include it only as a verification target.

## Acceptance Criteria

- [ ] A sweep of `skills/` for git VCS subcommands (`git log`/`git diff`/`git
      status`/`git rev-parse`/`git config`, …) returns no command invocation;
      every VCS operation is phrased in session-relative terms.
- [ ] `validate-plan` gathers implementation evidence successfully under
      git-only, colocated jj, and pure-jj, with pure-jj the previously-broken
      case, and its implementation diff renders the intended cumulative change
      under both git and jj.
- [ ] `refine-work-item` resolves a child work item's author correctly in both
      a jj repository and a git-only repository — the git-identity fallback
      still works via the session VCS, not a hard-coded `git config` call.
- [ ] `research-issue` gathers evidence successfully in a pure-jj repository
      (regression check; no code change expected).
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

- How should `validate-plan` phrase the implementation-diff range so the model
  reconstructs it reliably in jj? git uses `HEAD~N..HEAD`; jj needs a revset or
  `--from/--to` anchored on a base. What defines that base — the trunk
  divergence point, a bookmark, or the plan's first commit?
- Should repo-root discovery use the session VCS root command (e.g. `jj
  workspace root` / `git rev-parse --show-toplevel`, expressed generally), or
  should the checks step be made path-independent so no root lookup is needed?
- Should the SessionStart VCS Command Reference be extended with a diff-range
  and repo-root idiom, so skills lean on one documented vocabulary rather than
  each phrasing it independently?

## Assumptions

- `research-issue`'s existing session-VCS phrasing is precise enough for the
  model to act on reliably and needs no change. If it proves too vague in
  practice, scope expands to harden its wording.
- The SessionStart VCS Command Reference reliably steers the model to the
  correct backend command per session, with the guard as a backstop. If that
  steering proves unreliable, the rejected deterministic-CLI approach (an
  `accelerator vcs diff`) may warrant revisiting.
- `refine-work-item`'s git-config fallback exists to serve git-only
  repositories; preserving that behaviour — not merely deleting the git call —
  is in scope.

## Technical Notes

- `validate-plan` raw VCS usage: `git log --oneline -n 20` (line 58),
  `git diff HEAD~N..HEAD` (line 59), `git rev-parse --show-toplevel`
  (line 62), plus soft prose "through git" (line 48).
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
  `Bash(accelerator ...)`. research-issue already runs session VCS commands
  under the general Bash permission, so no git/jj allowed-tools entry is
  required.
- Backend equivalents for reference: `jj log`, `jj diff` / `jj diff --from
  --to` / `jj diff -r <revset>`, `jj workspace root`, `jj config get
  user.name`.

## Dependencies

- Blocked by: none — spike 0200, the original blocker, is complete.
- Relates to: work-item:0198 (VCS-agnostic status/log renderer and the
  session-VCS precedent), work-item:0200 (the spike that decided keep-both and
  spawned this), work-item:0169 (built the guard and its blocklist).
- Parent: epic 0136.

## Drafting Notes

- Reframed from "add an `accelerator vcs diff` subcommand" to a prose-only fix
  across skills, per the decision that a new CLI is not warranted: the affected
  skills gather VCS data at runtime, model-driven, not via `!`-injection, so
  the session VCS Command Reference plus the guard backstop already provide the
  backend-neutral behaviour the CLI would have added.
- Expanded scope from validate-plan/research-issue to all skills after a
  plugin-wide sweep, to eradicate every direct git token in `skills/`, not only
  the breaking calls.
- `research-issue` is treated as already-migrated (verify-only) — it contains
  no raw git.
- `refine-work-item`'s change is a rephrasing that preserves git-only author
  resolution, not a deletion of the git fallback; deleting it would regress
  identity resolution in git-only repositories.
- Surfaced a breakage the original item missed: `validate-plan`'s `git
  rev-parse --show-toplevel` (line 62) also fails in pure-jj.
- Corrected stale metadata: `research-issue` no longer carries raw git at lines
  63/66 (the original References were stale); `blocked_by: work-item:0200`
  cleared because 0200 is done.
- Dropped the `cli` tag, since the CLI is no longer touched.

## References

- `skills/planning/validate-plan/SKILL.md` — raw `git log`/`git diff`/`git
  rev-parse` at lines 58, 59, 62 (the evidence-gathering block to rewrite)
- `skills/research/research-issue/SKILL.md` — already session-VCS at lines 63,
  67; the pattern to mirror
- `skills/config/migrate/SKILL.md` — `git status` prose at line 273 to
  neutralise
- `skills/work/refine-work-item/SKILL.md` — `git config user.name` author
  fallback at line 249 to reframe
- `skills/vcs/commit/SKILL.md` — the `!`-injection precedent (why the CLI
  exists for injection but not for these skills)
- `cli/vcs/src/guard.rs`, `cli/vcs-cli/src/guard.rs` — the guard, its
  blocklist, and per-mode deny/warn behaviour
- `meta/work/0200-decide-vcs-guard-log-diff-blocklist-membership.md` — the
  spike that decided keep-both and spawned this (done)
- `meta/work/0198-vcs-agnostic-status-log-renderer.md` — the VCS-agnostic
  renderer and session-VCS precedent (done)
