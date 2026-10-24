---
type: "pr-description"
id: "168"
title: "[0302] Add a re-key command to renumber work items onto a changed ID pattern"
date: "2026-10-18T08:17:14+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0302"
parent: "work-item:0302"
relates_to: ["work-item:0146", "work-item:0230", "work-item:0343", "work-item:0344", "work-item:0345", "work-item:0346"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/168"
pr_number: 168
tags: []
revision: "d018d67fd103dd192f7ef53cb0f865f18b4b87eb"
repository: "accelerator"
last_updated: "2026-10-22T03:21:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0302] Add a re-key command to renumber work items onto a changed ID pattern

## Summary

Changing `work.id_pattern` (`{number:04d}` → `{key}-{number:04d}`, or →
`{tracker}`) leaves existing items on their old shape. 0230 kept them valid
on purpose and left translating them to an explicit command. This PR adds
that command, `accelerator work rekey`. It moves every item onto the current
pattern in one run by building on 0230's ID retirement: the file is renamed,
the old `id` is kept in `aliases`, and references are rewritten across every
corpus root. A persisted **run plan** ties a preview to its apply, lets an
interrupted run finish when run again, and can be discarded once a VCS revert
has undone the run. `/rekey-work-items` wraps the command: it previews, asks,
then applies or discards.

## Changes

- **`work rekey [--preview | --discard] [--per-item-reads]`.** What each
  item becomes depends on the pattern:
  - Under `{key}`/`{number}`: an ID from an earlier numeric pattern keeps its
    sequence number, re-rendered, and keeps its own project key when it has
    one. Tracker keys and `draft-` IDs take fresh numbers on `work.key`, in
    `date` order then old `id` byte-wise, starting above the highest number
    any `id` or alias in the target shape uses.
  - Under `{tracker}`: a synced item takes its `external_id`, refreshed from
    the tracker first (bulk `fetch_all`, then `locate` for every key not
    found). An unsynced item becomes a draft in `meta/work/drafts/`.
  - The run is refused as a whole on a collision with another item's `id` or
    alias, an unfinished sync retirement, a `pending_push` marker, an
    unclassifiable `id`, or exhausted number space.
- **Run plan.** It lives in `.accelerator/state/rekey/run-plan.json`, which
  VCS ignores, and is either proposed or in progress.
  - `--preview` records a proposed plan and prints `old → new` lines. The
    next apply re-keys exactly that mapping, or refuses with
    `E_REKEY_PREVIEW_STALE` if the identity fingerprint has changed.
  - An apply saves the plan in progress before its first write, and records
    each path's digest or absence before writing it.
  - `--discard` removes an in-progress plan only once every write-set path
    matches its recorded start state. Otherwise it lists the paths to
    restore, each beside its base revision. Forcing past a refusal sets the
    plan aside instead of deleting it.
  - A newer-schema or malformed plan refuses as `E_REKEY_PLAN_UNREADABLE`.
- **Identity-run lock.** `.accelerator/state/locks/identity-run.lockdir` is
  taken first by `work sync`, `promote`, `create`, `create-batch`,
  `next-number`, `update --push` and `rekey`, and released when the command
  returns. A waiter says which command it is waiting for and gives up after
  60 s with `E_IDENTITY_RUN_BUSY`. While a plan is in progress, every other
  holder refuses with `E_REKEY_IN_PROGRESS`. The global lock order is
  documented in `cli/corpus/src/lock.rs`. Retiring and allocating use cases
  take `&IdentityRunGuard`, so the compiler stops a caller skipping the lock.
- **Shared dirty-tree check.** The scoped dirty-path observation moves from
  `migrate` into `vcs::guarded_working_copy`, and each caller picks its
  probe-failure policy: rekey treats an unprobeable copy as dirty, `migrate`
  keeps treating it as clean. Both now find the repository through
  `RepositoryProbe::facts_at`, and `WorkingCopyState::seen_from` scopes dirty
  paths to the config root. This fixes a gap where a `.accelerator/` below
  the VCS root saw no dirty paths at all.
- **`ACCELERATOR_FORCE`.** It replaces `ACCELERATOR_MIGRATE_FORCE` and
  accepts `1` or `true`, ignoring case. A forced run lists on stderr what it
  skipped. `migrate` still honours the old name, printing a deprecation
  notice; 0346 removes it.
- **Retirement extensions.** Retirement can now land in `drafts/`, strips
  the new `id` from `aliases`, and keeps a synced item's tracker link. Path
  spellings are rewritten as whole tokens only, so `0041-first.md` no longer
  matches inside `ACC-0041-first.md`. A `before_apply` hook hands rekey each
  plan before it is written.
- **`fetch_all` contract.** Adapters report an issue only under its current
  key, so a moved issue always reaches `locate`. A new contract property pins
  this, and the Jira and Linear offline contracts run it.
- **Lock hygiene.** `store::lock::clear_abandoned` removes a per-file
  lockdir whose holder is dead. Rekey calls it over the write set once the
  entries are done, so a resumed run no longer leaves a stale `*.lockdir`
  for `git add -A` to pick up.
- **Skills and docs.** The new `/rekey-work-items` skill and its evals.
  `sync-work-items`, `create-work-item`, `extract-work-items` and
  `refine-work-item` now route `E_REKEY_IN_PROGRESS` to it. The docs site's
  sync guide, work-items reference, which-skill, migrations and configuration
  cookbook are updated, along with the CHANGELOG `Added`/`Changed` entries.

## Context

- Work item: `meta/work/0302-work-item-re-key-command.md` (parent 0146,
  builds on 0230)
- Research:
  `meta/research/codebase/2026-10-15-0302-work-item-re-key-command.md`
- Plan: `meta/plans/2026-10-15-0302-work-item-re-key-command.md` (8 phases,
  each with an "as built" section)
- Reviews: `meta/reviews/work/0302-work-item-re-key-command-review-1.md`,
  `meta/reviews/plans/2026-10-15-0302-work-item-re-key-command-review-1.md`
- Validation:
  `meta/validations/2026-10-15-0302-work-item-re-key-command-validation.md`
  (pass; every issue found resolved test-first)
- Split out on this branch: 0343 (alias normalisation), 0344 (reusable
  interactive decision engine), 0345 (bare-number review) and 0346 (removing
  `ACCELERATOR_MIGRATE_FORCE`).

## Testing

- [x] `mise run` exits 0 at `a36755b6`, after the rebase onto `main`, and
  its `fix` stage left the working copy unchanged. That covers every
  `lint:*`, `types:*`, `public-api:check`, `pup:check`, `deny:check` and
  `docs:*` lane.
  - `test:unit:cli`: 6,276 run, 6,276 passed, 2 skipped.
  - `test:integration:skill-invocation`: 143 passed, including the new
    `rekey-work-items` evals.
  - `test:unit:frontend`: 2,615 passed; `test:e2e:visualiser`: 355 passed;
    `test:unit:tasks`: 3,573 passed.
- [x] Each phase was built test-first. The validation's test-name audit found
  305 of the plan's 311 named tests verbatim, and accounts for the other 6.
- [x] Manual checks in scratch jj and git repositories with `.accelerator/`
  in `pkg/` below the VCS root (2026-10-18). Each check is listed in the
  validation report:
  - a dirty `pkg/meta/notes/links.md` refuses both `work rekey` (exit 4) and
    `migrate` (exit 1), naming the path;
  - a clean apply renames, aliases and rewrites links, and a second run has
    nothing to re-key;
  - a 402-item run killed with `SIGTERM` resumes to 402 distinct IDs, with
    no run plan or lockdir left;
  - `--discard` after a kill refuses until the paths are restored, then
    succeeds;
  - preview, then a committed `aliases` edit, refuses with
    `E_REKEY_PREVIEW_STALE`;
  - a second concurrent `work rekey` prints its waiting line.
- [x] Phase 7's moved-issue check against Linear, its offline check behind a
  refusing proxy, and the live tracker-contract lane against scratch Jira
  and Linear, all run on 2026-10-17.
- [ ] The live tracker-contract lane, the Linear moved-issue check and the
  manual checks above were not re-run after the final fixes or the rebase
  onto `main`. The fixes touch the dirty-tree check, collisions, allocation
  overflow, lock clearing and path rendering, not the tracker reads. The
  rebase adapted the branch to `main`'s optional-title `RemoteTracker`, its
  split baseline entries and its boxed `IntendedBaseline`, and the offline
  contracts cover those.

## Notes for Reviewers

- **Behaviour change for `migrate`.** In a repository whose `.accelerator/`
  sits below the VCS root, `migrate` used to probe nothing and run over a
  dirty tree. It now refuses there unless `ACCELERATOR_FORCE=1` is set.
- **Upgrade every session before re-keying.** An older binary's `work sync`
  does not take the identity-run lock, and its pulls can recreate a retired
  file. The CHANGELOG says so. Rekey also holds the `Create` lock for the
  whole run, so an older `work create` waits instead of racing.
- **Rekey holds the retirement lock for the whole run.** This departs on
  purpose from 0230's "never a whole pass". Each entry still re-walks the
  corpus through `finish_retirement`; batching that walk is out of scope.
- **Exit codes.** Every rekey refusal exits 4 with an `E_REKEY_*` code, which
  widens the 4 band to "stopped pending a human". Rekey does not reuse
  retirement's `retirement-failed`/`retirement-incomplete`, which are frozen
  at 71 with a remote-mutation meaning. The table is in the plan's "Exit codes
  and keywords" section and in `cli/work-cli/src/exit_codes.rs`.
- **Known limits.** A half-run that was committed and then lost its run plan
  is detected only while an alias or recovery copy remains. The `{tracker}`
  refresh has no ceiling on tracker calls, so a 429 exits 70.
- **Where to start.** The diff is ~22.2k lines over 162 files, about a third
  of them `meta/` and tests. The pure model is in `cli/work/src/rekey/`, the
  run-plan state machine in `cli/work-adapters/src/rekey/run.rs`, and the CLI
  wiring and rendering in `cli/work-cli/src/rekey.rs`.
