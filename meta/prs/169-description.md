---
type: "pr-description"
id: "169"
title: "[0300] Land sync pulls into tracked work items and report every failure"
date: "2026-10-24T18:45:07+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0300"
parent: "work-item:0300"
relates_to: ["work-item:0296", "work-item:0257", "work-item:0285", "work-item:0351", "work-item:0352", "work-item:0353", "work-item:0354"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/169"
pr_number: 169
tags: ["sync", "work-cli", "work-adapters", "store", "sync-work-items"]
revision: "1e77846312c7773d6af56c9ff81738182bc93f30"
repository: "accelerator"
last_updated: "2026-11-03T12:59:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0300] Land sync pulls into tracked work items and report every failure

## Summary

`accelerator work sync` wrote pulled work-item files through a store rooted at
the integration-state directory, so containment refused every pull into a
tracked item. Since 0296 the refusal showed up as a `local-write-refused` row,
but the pull never landed and the cause never reached stderr. Every other local
I/O failure exited 71, whether or not anything had landed, and unreadable work
files dropped out of the run unseen. This PR makes those pulls land through a
guarded pull path and two bounded stores. It classes each failure by whether
anything landed, and every failure now leaves a visible record. It also stops
`--target` and `--preview` runs deleting the dossiers of conflicts they never
settled. A second round of commits answers the first review: every sync writer
now goes through a validated root, refusals and failures after a landing are
classed `terminal`, and the cross-crate `unreachable!` invariants are carried
in types.

## Changes

**Store containment (`cli/store`, `cli/corpus-adapters`)**

- A `..` through a directory that does not exist yet is refused as
  `UnsafePath` on every store read and write, where it could escape the root.
- A permission refusal on a write or lockdir keeps the OS message
  (`NotWritable { detail }`). A rename that published but whose directory sync
  failed is `NotDurable`.

**The pull path (`cli/work-adapters/src/sync/apply.rs`, `attempt.rs`)**

- A pull takes the item's `ForFile` lock, then reads the file and checks it
  still holds what the run classified (or the pull's target).
- It keeps a recovery copy of edits the last sync did not capture, then hashes
  the new content.
- It re-reads the file and requires exactly the content the copy decision
  read, then writes it and sets the baseline. Nothing is written over a file
  that changed mid-run.
- An untracked file skips the copy only when its baseline matches under both
  digest recipes (`UnchangedSinceSync::BothRecipes`). Recipe 1 alone ignores
  formatting and markup the dialect drops on push.
- Recovery copies live under `.accelerator/state/sync-recovery/`, reported as
  `#\tkept-edits` records. Each sync reviews and sweeps the settled copies,
  and `--discard-kept-edits <id>` removes an item's copies.
- An item whose classification fails becomes its own `retryable` row instead
  of aborting the run with exit 1.

**Bounded stores (`cli/work-cli/src/bounded_roots.rs`, `sync/stores.rs`,
`sync_author.rs`)**

- Sync writes through a work-item store and an integration-state store; each
  refuses the other's root, `..` and non-plain path components.
- Every other writer in the run builds its store from a validated root with
  `within_project`: retirement and promotion records, and the local author's
  link/unlink writes, authored files and promotion-record cleanup. A root
  swapped for an outbound symlink after validation is refused.
- `ValidatedRoots::check` refuses these with exit 74 before any tracker call:
  - a root symlinked out of the repository;
  - overlapping roots, or a root containing the state root (the refusal names
    the resolved paths);
  - a symlinked state root, recovery directory or tracker state directory;
  - an outside `paths.work` or `paths.integrations` set anywhere but an
    untracked `.accelerator/config.local.md`.
- Pending-push markers are written and removed only through the
  integration-state store. `config::provenance` reports which file set a path.

**Failure classes and reporting (`cli/work-adapters/src/sync/report.rs`,
`cli/work-cli/src/sync.rs`)**

- An `Io` failure is `retryable` (70) when nothing for the item had landed and
  `terminal` (71) when something may have. `ItemAttempt::finish` applies the
  same rule to every failure, so a refusal or tracker error after a landing is
  `terminal` too; a create that clears its own `Attempted` marker rolls back
  to the progress before it.
- A local author's write that landed but could not be made durable is
  reported as possibly landed (`AuthorError::NotDurable`).
- A permission-refused write is `retryable`. `local-write-refused` keeps only
  refusals a retry would repeat.
- A ninth class, `order-refused` (75), covers a refused pending-push order.
- Every failure prints `error: <id-or-path-or-dash>: <operation> failed:
  <cause>` to stderr, including these run-level ones:
  - a failed baseline finalise;
  - a failed tracker read;
  - a failed discovery search.
- The exit-code precedence is unchanged.
- `work::inventory` reports every Markdown file sync cannot use as
  `#\tunidentified`. Unreadable or dangling files fail a whole-corpus run
  with 70; undecodable or malformed files, including VCS conflict markers,
  fail it with 75. A file that is not a work item is informational, and a
  targeted run only warns.
- `PlannedAction` fields are private, and an action carries at most one
  `Instruction::{Order, Resolution}`.
- Pending pushes are settled by `--resolve <id>=adopt:<KEY>` or
  `--resolve <id>=create`. A marker that cannot be read, is tracked, or names
  a foreign key is refused as `marker-unreadable`. A relink `create` settles
  its order before it clears the item's link.
- `OrderPrecondition<C>` carries the key and carrier it decided on, so
  `settle_order` no longer looks them up again behind `unreachable!`.

**Dossiers scoped to the run (`cli/work-adapters/src/sync/dossiers.rs`)**

- `ConflictStatus` and `DossierAuthority` retire a dossier only when its item
  is settled or out of conflict in this apply run. On a full run whose corpus
  is complete, a dossier whose id no work item uses is also retired.
- A preview retires nothing, and dossiers are built after apply.
- Retirement never settles a recovery copy it cannot read, and
  `--discard-kept-edits` leaves a recovery directory version control may
  track. A kept copy's folders are fsynced before the overwrite.
- A resolution that failed part-way keeps its dossier and reports it as
  `#\tstale-dossier`.

**Migration and init (`cli/migrate`, `cli/config-adapters`)**

- Migration `0011` and `accelerator config init` ignore the recovery
  directories and each integration's `pending-push/`, `conflicts/` and
  `adopted/`. An integrations root anywhere under `.accelerator/state` other
  than the default gets its own root rules.

**Skills and docs**

- `sync-work-items` is rewritten around reading the run's output:
  - `retryable`/`terminal` rows count as failures whatever the exit code;
  - advice is keyed on the `error:` line's operation;
  - every marker or `external_id` change needs explicit confirmation;
  - kept copies are relayed;
  - a run with failures is never reported complete;
  - `#\tsummary\tsynced\t<N>` is reported as `in sync:` (0353), and the
    duplicate `noop\tunsynced` row beside a create joins no group, an interim
    rule until 0352 lands.
- The skill-structure test reads its four vocabularies from
  `cli/work-adapters/tests/fixtures/sync-vocabulary.golden`, which a Rust test
  renders from `ALL` on `SyncOperation`, `FailureClass` and `RefusalReason`.
  Every operation must appear in the retryable-advice table unless it is
  run-level.
- A new non-invocable `sync-work-items-recovery` skill holds the terminal,
  order-refusal and pending-push recovery tables.
- `create-work-item` and `extract-work-items` relay a refused sync as
  `local-save`; `configure` documents the outside-root rule for both keys.
- The skill and `work sync --help` no longer promise that a preview shows a
  create's pending-push refusal (0351); the skill checks for a marker
  instead.
- The user guide and `CHANGELOG.md` cover the new records, classes and exit
  codes.

**Integration with re-keying (#168)**

The branch is rebased onto main after #168, so sync now runs under its
identity-run lock.

- `work sync` holds the lock for its whole run and refuses with
  `E_REKEY_IN_PROGRESS` or `E_REKEY_PLAN_UNREADABLE` (exit 4), or
  `E_IDENTITY_RUN_BUSY` (exit 1).
- The lock is taken after the id scheme, `--discard-kept-edits` ids and roots
  are validated and before any work item is read, so a refused root or a
  usage error still writes nothing.
- `IdentityWorkspace` keeps #168's split into `CorpusWorkspace` and gains
  `for_sync` and `for_identity_change`, which open it under checked roots.
- `sync-work-items` relays the re-key refusals at the end of Step 2.

**Structure (`cli/work-cli/src/sync.rs`, `cli/work-adapters/src/sync/`)**

- `run_sync_with` reads prepare → plan → assemble → report: `prepare_sync`
  returns `Result<PreparedSync, Refusal>`, each `Refusal` carrying its lines
  and exit code, and the `too_many_lines` allow is gone.
- Smaller consolidations: `From<&ApplyError> for RunFailure`, one adopt-order
  preamble, the VCS-status warning as a `RunFailure`, an exhaustive
  `discard_note`, one `UNCLASSIFIABLE_CLASS`, and tracked adoption records
  kept distrusted through a key change.

**Unrelated changes needed for a green `mise run`**

- The visualiser's watcher and SSE tests feed scripted events through an
  injected watcher (`server::run_with`), because a loaded macOS `fseventsd`
  delayed OS notifications past the test budgets. The real OS watcher test
  runs on every non-macOS leg, and on macOS in the opt-in
  `test:integration:visualiser-os-watcher` lane.
- `tokio` is bumped to 1.53.3 for RUSTSEC-2026-0386, and `pulldown-cmark` to
  0.13.6 for RUSTSEC-2026-0433. Digest recipe 1 records 0.13.6: its diff
  touches only wikilinks, which no crate enables.
- `docs-site` pins zod exactly to 4.6.7 through an override for
  GHSA-mrhx-c7xc-m3w5; `undici` 8.12.3 already came from main.
- `test:unit:cli` announces when permission tests are skipped.

## Context

- Work item: `meta/work/0300-sync-pulls-of-tracked-work-items-fail-silently.md`
- Plan: `meta/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`
- Plan reviews: `meta/reviews/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently-review-{1,2}.md`
- Validation: `meta/validations/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently-validation.md`
- PR review: `meta/reviews/prs/169-review-1.md`; every inline thread is
  answered and resolved on GitHub.
- Builds on 0296 (#159, merged): body equivalence, `ItemSnapshot`, and the
  `ApplyError` refusal variants.
- Defects found in the manual checks are filed as 0351–0354. 0353 and 0354
  are fixed here and marked done; 0352 has its interim skill rule; 0351 is
  open, with the docs qualified.

## Testing

- [x] `mise run` (full local CI mirror) exits 0 at the branch tip, rebased
  onto main after #168. It covers format, lint, types, every test suite,
  `public-api:check` and `docs:check`.
- [x] Store containment, including each `..` shape and permission detail:
  `cli/store` unit tests, plus one pin per consumer crate for a
  misconfigured `paths.*` through a missing directory.
- [x] Pull safeguards and the kept-copy lifecycle:
  `work-adapters/tests/sync_pull_safeguards.rs` and `kept_edits.rs`.
- [x] Classification by landing, totality and the tracker mapping:
  `tests/sync_apply.rs` (`classification_is_total`,
  `tracker_failures_keep_their_named_class`).
- [x] End-to-end through `run_sync_with` with real stores rooted as in
  production and a captured console (`work-cli/src/sync.rs`), covering:
  - the pull regression and the read-only work directory;
  - unreadable, malformed and dangling work files;
  - every root refusal, including the symlinked tracker directory;
  - the unreadable pending-push marker;
  - the dossier criteria, the relink one included.
- [x] Skill content contract: `tests/unit/tasks/test_sync_work_items_skill_structure.py`,
  against the CLI-rendered vocabulary golden.
- [x] Review fixes, each test-first: the relink order, the untracked
  copy-skip, case-folded overlap, failures after a landing, `NotDurable`
  author writes, the swapped work directory, tracked records and folders,
  and m0011's state rule.
- [x] Manual checks against Linear, run in one scratch session and recorded as
  done in the plan:
  - a pull into a tracked item;
  - a read-only work directory (exit 70);
  - a two-conflict `--target` run;
  - driving `/sync-work-items` through every failure case.
- [ ] The fixes made after validation, and the review fixes, have tests only
  and were not re-run against Linear: the unreadable marker, the symlinked
  tracker directory, the `show`/`discover` error lines, dangling links, and
  every change since the first review.
- [ ] The kept-copy folder fsync is not observable from a test.

## Notes for Reviewers

- ⚠️ Breaking exit codes for scripts that call `work sync`:
  - A local read failure now exits 70, not 71, and so does a
    permission-refused pull, not 75.
  - An unreadable work file now exits 70; an undecodable or malformed one
    exits 75. Before, both exited 0.
  - A team-config `paths.work` or `paths.integrations` outside the
    repository now refuses the run with 74.
  - A failure after something for the item landed now exits 71 whatever its
    class, for example a relink `create` that fails after its unlink.
  - An unrecognised or duplicate `--resolve` order now exits 2.
- ⚠️ Run `/accelerator:migrate` (migration `0011`) to ignore machine-local sync
  state. Settle any tracked pending-push markers first with `--resolve`.
- Validation turned up four known gaps, deliberately left as they are:
  - a recovery copy that fails part-way leaves its directory behind (the pull
    still refuses, so no data is lost);
  - a tab or newline in a work-file path garbles its copy record;
  - some directory-creation permission errors surface as `Io`, not
    `NotWritable` (the class is still `retryable`);
  - a file that turns non-UTF-8 between planning and the pull exits 70, not
    75;
  - an untracked file with only whitespace edits is still overwritten without
    a copy, because the legacy digest trims each line and no byte-exact digest
    is stored;
  - a preview does not read a create's pending-push marker (0351).
- Review findings left as they are: the `sync.rs`/`run.rs`/`apply.rs` splits
  and report-module cycles, `RootedStore` into `corpus-adapters`, raw
  `std::fs` reads, moving classification from the skill into the CLI, the
  `KeptEdits` split and the `RunReport` double build.
- Two state-root gaps from #168's commands are left for a follow-up:
  - `work rekey` opens its workspace without `ValidatedStateRoots::check`, so
    a symlinked `.accelerator/state` is written through rather than refused;
  - `work promote` takes the identity-run lock before
    `for_identity_change` checks the state root, so the lock is written
    through a symlinked state root before the refusal.
- `.accelerator/state/integrations/linear/last-sync.json` changes because the
  0351–0354 work items were synced to Linear (PP-926 to PP-929).
- Most of the line count is `meta/` (plan, reviews, research, validation) and
  tests. The production change sits mostly in `work-adapters/src/sync/`
  (`apply.rs`, `attempt.rs`, `run.rs`, `report.rs`, `kept_edits.rs`,
  `dossiers.rs`) and `work-cli/src/` (`sync.rs`, `bounded_roots.rs`,
  `sync/stores.rs`), plus `work/src/inventory.rs` and `store/src/lib.rs`.
- GitHub cannot render this PR's diff (over 20,000 lines); review it locally
  or per commit.
