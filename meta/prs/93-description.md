---
type: "pr-description"
id: "93"
title: "VCS-agnostic status/log renderer"
date: "2026-08-31T23:39:35+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0198"
parent: "work-item:0198"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/93"
pr_number: 93
tags: ["rust", "vcs", "cli", "gix", "jj-lib", "status", "log"]
revision: "7e82b14f01df214dec11d76643f8e630361edca8"
repository: "accelerator"
last_updated: "2026-08-31T23:39:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# VCS-agnostic status/log renderer

## Summary

Replaces the last two `vcs` subcommands that shelled out to a child process —
`status` and `log` — with in-process renderers over `gix` (git) and `jj-lib`
(jj), emitting the single backend-neutral format fixed by ADR-0066. Both
backends flip together in one atomic behavioural change, `vcs_adapters::subprocess`
is deleted, and a zero-spawn assertion proves neither subcommand launches a
child. This closes work item 0198 and removes the last process spawn from the
VCS facts/orientation path used by `/commit`.

## Changes

- **Neutral model + pure renderer** (`cli/vcs/src/status.rs`, `log.rs`) — value
  types (`ChangeType`, `FileChange`, `StatusReport`, `LogEntry`, `LogReport`)
  and pure `render` functions that own the sort, the empty-state literals, and
  the conflict summary, so neither adapter re-implements formatting.
- **`VcsReporter` port** (`cli/vcs/src/lib.rs`) returning `kernel::Error` via the
  existing `From` seam — a fallible port whose `Err` arm drives the fallback and
  the AC6 diagnostic, without introducing a new public error type.
- **Git adapter** (`cli/vcs-adapters/src/library/status_log.rs`) — branch from
  `head_name()`, change-type and conflict from one `Item::summary()` read, a
  first-parent 12-hex `rev_walk` capped at five. Pure `classify`/`resolve`
  functions map status items to `FileChange`s and dedup same-path collisions by
  commit-accuracy (staged type wins; conflict overrides).
- **jj adapter** — snapshot-and-tree-diff extracted into a shared
  `library/snapshot.rs` (`working_copy_diff`), reused by `dirty_paths`; conflict
  read separately from `MergedTree::conflicts()` and unioned in, because a
  merge conflict appears in no tree-diff entry; a first-parent change-id walk for
  log, root-excluded, capped at five.
- **Never-fail boundary + AC6** (`cli/vcs-cli/src/report.rs`, `main.rs`) — a
  `catch_unwind` boundary over the port folds `Err` and cleanly-unwinding panics
  to `(status|log unavailable)`, warn-logging the `gix`/`jj-lib` token on an
  `adapter =` field; `kernel::logging::init()` is now called (gated on
  `ACCELERATOR_LOG`) so the diagnostic is actually delivered.
- **Subprocess deleted, zero-spawn widened** (`cli/vcs-adapters/src/subprocess.rs`
  removed; `cli/pup.ron`) — a crate-wide `vcs_adapters_is_zero_spawn` rule
  forbids `std::process` anywhere in the crate; the zero-spawn lane is extended to
  render `status`/`log` under absolute-path `git`/`jj` shadowing, including an
  adversarial hostile-config state.
- **Parity harness + goldens** (`cli/vcs-cli/tests/status_log_parity.rs`, the
  `vcs-status-log` fixtures) — git-vs-jj shape parity with an unmasked control,
  plus regenerated ADR-format goldens covering conflict, rename, delete, unborn,
  sha256-fallback, and jj bookmarks.
- **Prompt-injection hardening** (`skills/vcs/commit/SKILL.md`) — the injected
  status/log block is now framed in a `<repository-vcs-context>` delimiter with
  an untrusted-data warning, since status widens the repo-controlled surface
  reaching the `/commit` LLM prompt.

## Context

- Work item: `meta/work/0198-vcs-agnostic-status-log-renderer.md`
- Plan: `meta/plans/2026-08-31-0198-vcs-agnostic-status-log-renderer.md` (now `done`)
- Validation: `meta/validations/2026-08-31-0198-vcs-agnostic-status-log-renderer-validation.md`
- Format decision: `meta/decisions/ADR-0066-vcs-agnostic-status-log-output-format.md`

## Testing

- [x] Full cli workspace (`--all-features`): 2783 passed, 1 skipped
- [x] `vcs` renderer unit tests: 52 passed
- [x] Architecture rules + probe pairs: `pup:check`, `test:integration:pup` (65)
- [x] Feature graph + licence closure: `test:integration:deny` (111), `deny:check`
- [x] Public-API baseline gate: `public-api:check`
- [x] Local zero-spawn incl. adversarial-config state: `test:integration:zero-spawn` (3)
- [x] jj-settings guard + `cli:check` (rustfmt + clippy) + `build-system:check`
- [x] Never-fail fault injection: `Err`, panic (`catch_unwind`), malformed `ACCELERATOR_LOG`, AC6 `D2`-forced-failure token
- [ ] Strong-form zero-spawn (`check-zero-spawn` CI job, Linux) — cannot run under macOS SIP; gated on CI

## Notes for Reviewers

- **Accepted fault-isolation regression.** Moving in-process removes the
  subprocess 10-second cap and child-crash containment. `catch_unwind` restores
  never-fail for cleanly-unwinding panics, but a wall-clock hang or unbounded
  read is not caught — recovery is Ctrl-C. Priced for single-shot `/commit`
  callers; a future hang complaint reopens it.
- **Status now honours global git config.** `gix::open` reads `core.excludesFile`,
  `status.showUntrackedFiles`, and `core.ignorecase`, so untracked enumeration
  matches real `git status`. The old subprocess scrub forced these off — that was
  the anomaly. Pinned by the extended `scrub.rs` characterisation test.
- **Conflict is read differently per backend** — git counts the `Summary::Conflict`
  status item in `N`; jj unions `MergedTree::conflicts()` in, so `conflict-jj`
  renders `1 changed, 1 conflicted` with no other change lines. Worth a look in
  `status_log.rs`.
- The one remaining unchecked box is the Linux-only strong zero-spawn CI lane;
  everything else is green locally.
