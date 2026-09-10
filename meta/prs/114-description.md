---
type: "pr-description"
id: "114"
title: "[0200] Decide vcs guard keeps git log/diff blocked; spawn VCS-agnostic follow-up"
date: "2026-09-10T17:29:57+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0200"
parent: "work-item:0200"
relates_to: ["work-item:0286"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/114"
pr_number: 114
tags: ["vcs", "guard", "spike"]
revision: "4de586c3beceb4021ee9fe717a892781efd09bb5"
repository: "accelerator"
last_updated: "2026-09-10T17:29:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0200] Decide vcs guard keeps git log/diff blocked; spawn VCS-agnostic follow-up

> Stacked on #112 (`[0184]`). Base is the `0184-mark-work-item-done` branch;
> GitHub retargets this PR to `main` automatically once #112 merges.

## Summary

Records the outcome of spike 0200: keep `git log` and `git diff` in the vcs
guard's blocked subcommand set, and close the spike with no code change. Adds
the spike's first-review artefact, transitions 0200 to done, and spawns
follow-up 0286 for the real remedy the spike identified.

## Changes

- Record the 0200 spike outcome on the work item — a keep-both decision with
  per-subcommand rationale, the mode-split findings, and residual risks — and
  transition it draft → ready → done.
- Add the spike's first-review document
  (`meta/reviews/work/0200-…-review-1.md`).
- Spawn work item 0286, cross-linked to 0200 via a Blocks/blocked_by pair, to
  make `validate-plan` and `research-issue` VCS-agnostic through a new
  `accelerator vcs diff` subcommand.

## Context

Spike 0200 asked whether `log`/`diff` should stay in `BLOCKED_SUBCOMMANDS`
(`cli/vcs/src/guard.rs`). The decision is keep-both, resting on one empirical
fact: in a pure-jj repo (`--no-colocate`) there is no `.git` working tree, so
raw `git log`/`git diff` fail with `fatal: not a git repository` regardless of
the guard. The guard's deny converts that into a helpful `jj` redirect;
dropping the entries would surface the cryptic git failure instead and would
not unblock `validate-plan`. The guard's warn-vs-deny outcome is keyed on repo
mode (pure-jj denies, colocated warns), not on the subcommand — which is why
`validate-plan` is broken only in pure-jj repos, and why the fix belongs in the
skills, not the blocklist. That fix is 0286.

Implements work item 0200 (`meta/work/0200-…`); spawns 0286 (`meta/work/0286-…`).

## Testing

- [x] Frontmatter validated on both work items and the review doc
      (`accelerator corpus frontmatter validate` exits 0).
- [x] Docs-only change — no source, build, or test files touched, so no code
      verification applies.

## Notes for Reviewers

- This is a decision record plus a spawned follow-up; there is no code change,
  and deliberately so (Acceptance Criterion 3 of the spike).
- Two stale references in the original 0200 brief are corrected in the recorded
  outcome: the guard has no in-code "threat-model note", and the decision-table
  fixture now lives at `cli/vcs-test-support/fixtures/vcs-guard/`.
- 0200 carries `external_id: PP-730`; its done status reaches the remote tracker
  only on the next work-item sync, not through this PR.
