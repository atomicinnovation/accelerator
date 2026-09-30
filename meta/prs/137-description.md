---
type: "pr-description"
id: "137"
title: "[0230] Let the tracker own work item IDs, with local drafts promoted on sync"
date: "2026-09-30T00:28:58+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0230"
parent: "work-item:0230"
relates_to: ["work-item:0302", "work-item:0296", "work-item:0297", "work-item:0227", "work-item:0291", "work-item:0146"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/137"
pr_number: 137
tags: []
revision: "8025407ec5f8024c669f89c01e434a0dba9d65dc"
repository: "accelerator"
last_updated: "2026-10-04T22:39:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0230] Let the tracker own work item IDs, with local drafts promoted on sync

## Summary

Today every synced work item carries two identifiers: a locally minted `id`
(`0230`) and the tracker's `external_id` (`PP-760`). People translate between
them by hand. This PR adds an opt-in `work.id_pattern: "{tracker}"`, under
which the Jira or Linear key becomes the item's `id`. Anything the tracker has
not yet confirmed is saved as a `draft-xxxxxx` item, and sync promotes it
later. ADR-0070 records the decision and supersedes ADR-0044.

## Changes

- **`{tracker}` pattern.** `{tracker}` must be the only token in
  `work.id_pattern` and needs `work.integration` set to `jira` or `linear`.
  `{{tracker}}` is a literal. The pure validator in
  `cli/corpus/src/work_item_id.rs` is what 0227's `config validate` will call.
- **Draft-first create.** `work create --push` writes a draft, then promotes it
  onto the issue the tracker creates. The result is `meta/work/<KEY>-<slug>.md`,
  with `id`, filename, H1 and `external_id` all set to the key.
  - A sync baseline is now recorded on create under every pattern. That fixes
    0296 Defect 1.
  - New outcomes: `created-unwritten` (71), `created-blocked` (4),
    `retirement-incomplete` (71) and `rejected` (75).
  - `E_PUSH_PENDING` and `E_DRAFT_EXISTS` (exit 4) stop a rerun from creating a
    second issue.
- **Drafts.** Drafts live at `meta/work/drafts/draft-xxxxxx-<slug>.md`. The
  suffix is 6 Crockford base32 characters; a colliding draw is redrawn, up to
  16 draws. Drafts come from `work create` without `--push`, from `local-save`,
  and from a tracker-error `loud-terminal`. They can be resolved and are valid
  typed-link targets.
- **Promotion on sync.** `work sync` now promotes drafts by default, and
  `--no-promote` skips that. A draft that stays local is reported as
  `not-promoted` with a reason, for example `tracker-unreachable`,
  `remote-may-exist`, `possible-duplicate`, `id-taken` or `rejected`.
- **`work promote <DRAFT_ID> [--adopt <KEY> | --create]`.** Promotes one draft,
  with explicit recoveries: `--adopt` binds to an existing issue after checking
  it exists, and `--create` accepts the risk of a duplicate.
- **`work create-batch --manifest <json> [--push]`.** Creates parents before
  children and links each child's `parent` to the ID its parent took. A cycle
  exits 2 and writes nothing. Pushed batches are journalled under
  `.accelerator/state/batch-journal/`. `extract-work-items` now makes a single
  push offer through this command.
- **Identity pass in sync.** Before the engine plans, sync follows key changes
  made on the tracker: `key-changed`, `not-found`, `resumed`. Under every
  pattern it updates `external_id`. Under `{tracker}`, when the old key is the
  item's `id`, it also retires that `id`.
- **Transactional ID retirement.** Retiring an ID renames the file, adds the old
  ID to the new `aliases` field, and rewrites references across `meta/`. It is
  lock-guarded, resumable, and backed by recovery copies.
  - Typed links are always rewritten.
  - Prose is rewritten only for distinctive IDs (`draft-…` or `<KEY>-<n>`),
    never for bare numbers.
- **Resolution.** `work resolve` matches `id`, `aliases` and `external_id`,
  ignoring case. A token that hits different items through different fields
  fails with `E_RESOLVE_AMBIGUOUS` (exit 2).
- **Tracker clients.** Jira and Linear gain `locate()` (`Found` or `NotFound`)
  and fill in `RemoteIssue.key`. They classify errors more precisely:
  - A connect or DNS failure on the first attempt is `NotSent`, so retryable
    (70).
  - A request the client refuses as invalid is `Rejected`, exit 75, and is
    never retried.
- **Skills and docs.** Updated `create-work-item` (new "No, save as draft"
  option), `extract-work-items`, `sync-work-items`, `list-work-items`,
  `refine-work-item`, `configure` and their evals. The docs site's sync guide,
  configuration cookbook and work-items reference are updated to match.
  `work list` shows drafts as `🟠 draft`.

## Context

- Work item: `meta/work/0230-tracker-owned-work-item-id-generation.md`
- Decision: `meta/decisions/ADR-0070-tracker-owned-work-item-identity.md`
  (supersedes ADR-0044)
- Research:
  `meta/research/codebase/2026-09-26-0230-tracker-owned-work-item-id-generation.md`
- Plan: `meta/plans/2026-09-26-0230-tracker-owned-work-item-id-generation.md`
- Validation:
  `meta/validations/2026-09-26-0230-tracker-owned-work-item-id-generation-validation.md`
- Also on this branch: re-keying split out as 0302, the sync round-trip bugs
  as 0296, and qualified prose references as 0297. 0146, 0227 and 0291 are
  realigned to match.

## Testing

- [x] `mise run check` exits 0. That covers format, lint and types for all four
  components.
- [x] Full `mise run` exits 0 after the rebase onto `main`. That includes
  4,921 CLI unit tests (1 skipped), 2,927 tasks unit tests, 2,611 frontend
  unit tests, 355 visualiser e2e tests, the visualiser unit and integration
  lanes, the integration, conformance, skill-invocation and docs lanes, and
  `public-api:check` and `pup:check`.
  - One earlier `mise run test` failed `api_smoke` with
    `server never became reachable`. It passed alone and on the full run; it
    matches the server-readiness flake noted in earlier validations.
- [x] `mise run test:integration:pup` (92 tests). The default task does not run
  it.
- [x] Every one of 0230's 76 acceptance-criteria tests exists: 69 Rust tests, 4
  skill evals, and 3 cases inside parameterised tests.
- [x] Nine defects found in validation (P1–P9) are fixed, each with a test.
  They include the identity pass planning over the corpus as it was before
  resumption, YAML-escaped titles defeating the rerun guard, `--adopt` comparing
  keys case-sensitively, and the batch journal claiming entries across batches.
- [ ] `mise run test:integration:tracker-contract`. Not run, because it needs
  live Jira and Linear credentials.
- [ ] Live tenant checks: a `{tracker}` create, a cross-team move reported as
  `key-changed`, and draft promotion, including an offline run that leaves
  drafts untouched and exits 70.
- [ ] Skill checks: `create-work-item` "No, save as draft", then
  `extract-work-items` over an epic with two children (accepted and declined),
  then `sync-work-items` promotion.
- [ ] Ctrl-C during a scratch retirement, with the next retirement resuming it.
- [ ] ADR-0070's decision matches 0230's Requirements section.

## Notes for Reviewers

- ⚠️ **Outcome changes under legacy patterns.**
  - `work create --push` now prints `created-unwritten\t<key>` (71) where it
    used to print `loud-terminal` with a key.
  - A rejected request now exits 75 as `rejected` instead of 71.
  - Scripts that parse these outcomes need updating.
- ⚠️ **Downgrade.** An older binary's `outstanding` aborts on a promotion record
  written at a later stage. Finish promotions, or run `work sync`, before
  downgrading.
- **No data migration.** Legacy IDs stay as they are. Pushing an unsynced legacy
  item sets only `external_id`. Moving existing IDs to a new pattern is 0302.
- **Where to focus.**
  - The promotion state machine: `next_step` in `cli/work/src/promotion.rs`,
    and the service in `cli/work-adapters/src/promotion.rs`.
  - The retirement applier and recovery: `cli/work-adapters/src/retirement.rs`
    and `cli/corpus-adapters/src/recovery.rs`.
  - The rules for rewriting prose references in `cli/corpus/src/references.rs`.
  - The identity pass and its ceilings in
    `cli/work-adapters/src/sync/identity_settlement.rs`.
- **Known gaps** (recorded in the validation report):
  - 3 planned Phase 6 tests were not written. The report names the existing
    tests that cover them.
  - `rerunning_a_create_sends_no_second_create_at_any_record_stage` does not
    exercise the `RemoteKept` or `Retiring` stages.
- **Since the last revision.** Rebased onto `main` after #136.
  - The docs-site audit-exceptions change is dropped. `main`'s
    `docs-site/audit-ignores.toml` ignores the same advisory, and its check
    also fails once a review-by date passes.
  - This branch's `RemoveFile` port is folded into `main`'s `FileRemove`,
    under which an already-absent file counts as removed. Retirement still
    verifies every file it touches, under per-file locks, before its first
    step.
  - The re-key work item is now 0302, because `main` took 0295 and 0299.
- ❓ The plan stays `in-progress` until the manual checks above are done.
  Decide whether to merge before or after them.
