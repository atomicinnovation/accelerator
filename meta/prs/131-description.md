---
type: "pr-description"
id: "131"
title: "[0286] Remove direct git calls from skills"
date: "2026-09-22T18:33:33+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0286"
parent: "work-item:0286"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/131"
pr_number: 131
tags: ["vcs", "skills", "cli"]
revision: "c11606d3c40753534a73bf93a53c9ef856b6cad0"
repository: "accelerator"
last_updated: "2026-09-22T18:33:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0286] Remove direct git calls from skills

## Summary

Makes every skill express VCS operations in backend-neutral terms, so each one
behaves the same under git, colocated jj, and pure jj. `validate-plan` broke
outright in pure-jj checkouts (raw `git log`, `git diff`, `git rev-parse` fail
with `fatal: not a git repository` when there is no `.git`), and
`config/migrate` and `refine-work-item` named backend commands where they
should defer to the session VCS. Two new lint guards keep it that way.

## Changes

**CLI — `accelerator vcs root`** (`cli/vcs-cli/`)

- New `root` subcommand prints the working-copy root, wrapping the existing
  `RepoRoot::discover`. In a jj secondary workspace it returns the
  **workspace** root, not the shared main repository. Outside a repository it
  exits non-zero, naming the path it searched from.
- Unit tests over a stubbed probe, plus `root_goldens.rs` against real git-only,
  colocated, pure-jj, and jj-secondary-workspace checkouts
  (`bash-parity` feature).

**SessionStart VCS Command Reference** (`cli/vcs-cli/src/detect.rs`)

- Adds two idioms to both the jj and git reference blocks:
  - Diff range — `jj diff --from 'fork_point(trunk() | @)' --to @` /
    `git diff <trunk>...HEAD` for the cumulative change since the trunk
    divergence point, with the `trunk()`→`root()` fallback caveat and the
    `git symbolic-ref` hint for resolving `<trunk>`.
  - User identity — `jj config get user.name`, falling back to
    `git config user.name` when the jj identity is unset / `git config
    user.name`.
- Render tests extended; the three `vcs-detect` golden fixtures updated.

**Skills**

- `validate-plan` — replaces the raw-git evidence block with prose deferring to
  the session's VCS log command and diff-range idiom. It re-derives the
  reference via `accelerator vcs detect --descriptive` if compaction dropped it,
  and runs checks via `root="$(accelerator vcs root)" && cd "$root" && make
  check test`. The two new commands are added to `allowed-tools`.
- `config/migrate` — the `jj status`/`git status` reference becomes "the
  session's VCS status command".
- `refine-work-item` — collapses the `jj config` → `git config` author legs into
  one step deferring to the identity idiom. All three partial-write recovery
  sites say "delete the newly-written child file(s)" instead of `jj restore`.
  The eval expectations are updated to match.

**Guards** (`tasks/lint/`, wired into `build-system:check` and `lint:check`)

- `lint:git-tokens:check` — fails on any `git <subcommand>` from a blocklist in
  `skills/**/SKILL.md`. Scoped to `SKILL.md` only, so eval fixtures that narrate
  git commands are out of scope. Git-only by design, so `update-work-item`'s
  retained `jj restore` does not trip it.
- `lint:skill-cli-refs:check` — every `accelerator vcs <sub>` a skill names must
  be a real subcommand. `VCS_SUBCOMMANDS` is pinned against the clap `Command`
  enum by a cross-language test. It also render-locks the diff-range and
  identity idiom anchors in both `detect.rs` reference consts.
- `tasks/__init__.py` — the collection registrations lose their trailing
  per-line comments and fit on one line each.

**Meta**

- Work item 0286 (reframed from the narrower
  `0286-vcs-agnostic-validate-plan-research-issue-diff`, which is removed), its
  research, review, plan, plan review, and validation.

## Context

- Work item: `meta/work/0286-eradicate-direct-git-calls-from-skills.md`
  (parent 0136; spawned by spike 0200, which kept `log`/`diff` in the guard
  blocklist and located the real failure in the skills' raw git calls).
- Plan: `meta/plans/2026-09-20-0286-eradicate-direct-git-calls-from-skills.md`
- Validation:
  `meta/validations/2026-09-20-0286-eradicate-direct-git-calls-from-skills-validation.md`

## Testing

- [x] `mise run check` (read-only CI mirror) — exit 0
- [x] `mise run lint:git-tokens:check` and `mise run lint:skill-cli-refs:check`
  against the real tree — exit 0
- [x] `pytest` over `test_git_tokens.py`, `test_skill_cli_refs.py`,
  `test_mise.py` — 66 passed
- [x] `cargo test -p accelerator-vcs --features bash-parity` — all suites pass,
  including the 4 `root_goldens` and the secondary-workspace discriminating case
- [ ] Full bare `mise run` (adds `docs:check`, full test suite, in-place
  formatting) — not run for this description
- [ ] Release-time attended gates from the validation, which cover model-driven
  prose that no unit test exercises:
  - `validate-plan` on a pure-jj fixture yields a non-empty log and diff. The
    diff matches the cumulative change since divergence across all three
    topologies, including one where trunk has advanced past the fork point.
  - `refine-work-item` resolves the author correctly in jj-only, git-only, and
    colocated (jj identity unset) fixtures. Its eval suite passes via the
    harness.
  - `research-issue` still gathers log and diff on pure jj.

## Notes for Reviewers

- ⚠️ The diff-range and colocated identity fallback have **no CLI backstop**:
  skills name the idioms in prose and the model picks the command. This follows
  the plan's decision not to add a `vcs diff` subcommand. The
  `skill-cli-refs` render-lock only guarantees the idioms stay in `detect.rs`,
  not that skills use them correctly. Regressions surface only through the
  attended gates above.
- `vcs root` uses `RepoRoot::discover`, not `repository_root`, on purpose.
  `validate-plan` must run checks in the checkout it was invoked from, which in
  a jj secondary workspace is not the main repo.
- The `detect.rs` module doc now states that the reference consts are a
  steering contract that skills depend on by name. Removing an idiom there breaks
  skills silently, apart from the anchors the render-lock covers.
- The guard blocklist in `cli/vcs-cli/src/guard.rs` is unchanged.
