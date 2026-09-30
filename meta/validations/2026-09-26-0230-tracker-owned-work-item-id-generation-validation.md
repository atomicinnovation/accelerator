---
type: "plan-validation"
id: "2026-09-26-0230-tracker-owned-work-item-id-generation-validation"
title: "Validation Report: Tracker-Owned Work Item ID Generation Implementation Plan"
date: "2026-09-29T08:59:11+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
parent: "work-item:0230"
target: "plan:2026-09-26-0230-tracker-owned-work-item-id-generation"
tags: ["sync", "tracker", "id-generation", "drafts", "promotion"]
last_updated: "2026-09-30T00:24:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Tracker-Owned Work Item ID Generation Implementation Plan

Every phase's code changes are in place. The full local CI mirror passes
apart from one visualiser end-to-end test that fails only under full load
(see Automated Verification Results).
The result is `partial` for two reasons. First, the tracker contract suite
has not run, because it needs live Jira and Linear tenants. Second, eleven
manual checks are still open, most of which also need a live tenant. The
code review's findings (P1–P9), their leftovers and the plan-bookkeeping
gaps are all fixed.

### Implementation Status

✓ Phase 0: Superseding ADR — implemented (ADR-0070 `accepted`, ADR-0044
`superseded`)
✓ Phase 1: Not-sent failures are retryable — implemented
✓ Phase 2: Draft-aware discovery, `aliases`, and identity resolution —
implemented
✓ Phase 3: ID retirement — implemented
✓ Phase 4: Following tracker-side key changes — implemented (P1 fixed
since the first pass)
✓ Phase 5: The `{tracker}` token and drafts — implemented
⚠️ Phase 6: Online creation and the promotion service — implemented, but
three planned tests are absent (see Deviations)
✓ Phase 7: Promotion through sync and `work promote` — implemented
✓ Phase 8: `work create-batch` and skill integration — implemented

Acceptance-criteria traceability: all 76 test names in the plan's table
exist. 69 are Rust `fn`s. The other 7 are four skill evals in
`skills/work/*/evals/evals.json` and three labels inside parameterised Rust
tests.

### Automated Verification Results

⚠️ `mise run` (the full default task), with every fix applied, passes every
lane but `test:e2e:visualiser`. That covers 4,055 CLI unit tests (1 skipped)
and every integration, docs and tasks lane. In the last two full runs, one
visualiser test failed, `aside-row-resolved-colours.spec.ts` ("inferred
rows surface one row per other non-virtual doc type"), with
`page.goto: net::ERR_ABORTED` on both attempts. Run alone, the lane passes
all 355 tests, twice. Earlier full runs in this validation passed with the
same visualiser code, which depends on no `work` crate. It is not proven to
fail the same way on the parent commit under full load. Before the fixes,
`mise run` exited 0 on `6b815050`, over 4,025 CLI unit tests.
✓ Every per-phase `cargo test`, `public-api:check`, `pup:check`,
`test:integration:conformance` and `test:integration:skill-invocation`
command is covered by the full run.
✗ `mise run test:integration:tracker-contract` (Phases 1 and 4) was not run.
It needs `ACCELERATOR_JIRA_*` and Linear tenant credentials. Run locally
without them, `cargo test -p jira-client --test contract` fails with `NoSite`.

### Code Review Findings

#### Matches Plan:

- **Phase 1.** Only a connect failure on the first attempt counts as
  `NotSent`, in both transports
  (`cli/jira-client/src/transport.rs:335`,
  `cli/linear-client/src/transport.rs:301`). `REJECTED = 75` ranks
  `71 > 1 > 4 > 75 > 74 > 70`, and the frozen oracles are updated.
- **Phase 2.** `DraftId`, `ItemIdentity`, `resolve_identity`, `holder_of`
  and `linker_of` exist. The `WorkItemFiles` port and its filesystem
  adapter exist, and the pup allowance for `kernel::Error` has a probe pair.
- **Phase 3.** Planning is pure and resumable. The applier saves recovery
  copies, takes sorted per-file locks under the retirement lock, verifies
  the planned bytes, then takes its steps in order and rolls back in
  reverse. It restores a path only while that path still holds what the
  retirement wrote. `RESTORE-PENDING` is downgraded on completion. Fault
  injection covers all five store operations.
- **Phase 4.** The identity pass runs in this order: reconcile, one gather,
  decide, `check_ceilings`, apply. The engine then runs over the
  re-discovered corpus with the budget that remains.
- **Phase 5.** `validate_id_pattern` handles braces, so `{{tracker}}` is a
  literal. Minting uses the Crockford alphabet, redraws all-digit and
  colliding draws, and gives up after 16 draws.
- **Phase 6.** Promotion loops on `next_step` under the retirement lock,
  re-reading the record each time. An `Attempted` record resumes as
  `EarlierAttemptUnconfirmed`, never as a second create.
- **Phase 7.** `--no-promote`, `work promote --adopt|--create`, and the
  `not-promoted` reason keywords all match the keyword table.
- **Phase 8.** `parents_first` and `cyclic_members` live in `work`.
  `create-batch` has its manifest, a journal that prunes after 30 days, and
  the skill, docs and configure changes.

#### Deviations from Plan:

- **Unwritten Phase 6 tests.** Three planned tests were never written, and
  existing tests cover each; the plan's implementation notes now say which.
  `two_clean_rollbacks_…` is covered by
  `a_retirement_failure_rolls_back_and_leaves_a_promotion_record_holding_the_key`.
  `a_promotion_killed_mid_rewrites_…` and `a_create_killed_after_sending_…`
  are covered by
  `a_promotion_killed_at_each_stage_boundary_finishes_on_the_next_promote`,
  which kills at every store operation. Separately,
  `rerunning_a_create_sends_no_second_create_at_any_record_stage` covers
  `Attempted` and `Created`, but not `RemoteKept` or `Retiring`.
- **Phase 7 success criteria (fixed).** The command ran `--test
  sync_create` only, while the promotion tests live in `sync_settled`. It
  now runs both.
- **Shared hierarchy walk (fixed).** `work list`'s private `child_index` is
  replaced by `work::hierarchy::children_of`, as Key Discoveries planned for
  0291. It returns child positions, so two items sharing an ID stay two
  children.
- **Documented deviations.** The following are recorded in the plan's
  implementation notes and are equivalent or improvements:
  - `SettlementPorts` in place of extending `SyncPorts`;
  - `IdentityRow` in place of new `Action` variants;
  - `keys_held_by_promotions` in place of extending `followed_keys`;
  - byte comparison in place of digests during retirement;
  - `BadIdentifier` → `RequestInvalid` in Jira;
  - the journal at `.accelerator/state/batch-journal/journal.json`.

#### Potential Issues:

Ranked most severe first. A failing test reproduced every code finding
before its fix, except P4, a skill-text correction confirmed by reading the
code. All are fixed.

1. **P1: the identity pass plans over the corpus from before
   reconciliation** (confirmed). `settle_identities` finishes interrupted
   retirements, then `detect_identity_changes` gathers over
   `request.reconciled()`, which was discovered before that
   (`cli/work-adapters/src/sync/identity_settlement.rs:388-398`, `:611`).
   After resuming `PP-760 → ENG-42`, the same run still sees `PP-760`. A
   tracker that follows moves then yields a second key change for an item
   that no longer exists, which is refused or a no-op and uses up pull
   budget. `a_key_change_retirement_killed_after_writing_to_resumes`
   misses this because its tracker models no move.
   **Fixed:** `run_settled` now calls `resume_interrupted` first. When a
   resumption lands, it re-discovers the corpus and re-scopes the request
   (following a targeted run's items through the rename) before
   `settle_identities` detects key changes.
   `a_resumed_key_change_is_not_followed_again_from_the_pre_resumption_corpus`
   and `a_targeted_run_follows_its_target_through_a_resumed_retirement`
   cover it.
2. **P2: titles are read back without YAML unescaping.** `read_field_raw`
   strips only one surrounding quote (`cli/work/src/show.rs:9-26`).
   Promotion then uses the result for the create title and
   `content_digest`, while the rerun guard digests the raw `args.title`
   (`cli/work-cli/src/create.rs:983`). A title containing `"` or `\` could
   defeat the guard, sending a second create, and could put escape
   characters in the remote title. **Fixed:** promotion and
   `draft_content_digest` now read `title` and `kind` with a real YAML
   parse, through `work_adapters::create_request_fields::read`.
   `a_title_the_frontmatter_escapes_reaches_the_tracker_as_written` and
   `a_rerun_whose_title_the_frontmatter_escapes_is_e_draft_exists` cover
   it. Sync's create-from-local and push paths
   (`cli/work-adapters/src/sync/run.rs`) and `work list`
   (`cli/work-cli/src/list.rs`) had the same defect, and now read `title`
   as YAML too, through
   `work_adapters::frontmatter_strings::FrontmatterStrings`. Three tests
   cover them:
   `a_created_title_the_frontmatter_escapes_reaches_the_tracker_as_written`,
   `a_pushed_title_the_frontmatter_escapes_reaches_the_tracker_as_written`
   and `scan_reads_a_title_the_frontmatter_escapes_as_written`. If a file's
   frontmatter is not valid YAML, `work list` still shows its raw title,
   while sync treats the file as malformed.
3. **P3: `--adopt` compares keys case-sensitively** (confirmed).
   `key != named` on `ExternalId` is an exact string comparison
   (`cli/work/src/promotion.rs:297`). `--adopt pp-900` against a recorded
   `PP-900` therefore stops as `adopt-conflicts-with-recorded-key`,
   although resolution elsewhere ignores case. **Fixed:** the comparison
   now ignores case, and
   `adopting_the_recorded_key_in_another_case_resumes_onto_it` covers it.
4. **P4: the extract skill contradicted the CLI on empty paths**
   (fixed). The skill said an empty path appears only on legacy
   `created-unwritten`, but `create-batch` also prints one for `pending`
   from a legacy slug marker and for a journalled `created-unwritten`.
   `skills/work/extract-work-items/SKILL.md` now names both cases and gives
   `pending` with no draft a working recovery (`/sync-work-items`). It also
   says that a rerun changes nothing only with `--push`.
5. **P5: the batch journal matched by title across batches** (fixed). A
   later, unrelated batch with an entry of the same title was claimed and
   never created. Each journal entry now records a fingerprint of its
   manifest's refs and titles, and the title fallback claims only entries
   from the same batch. Covered by
   `an_unrelated_batch_sharing_a_title_creates_its_own_item` and
   `a_title_another_batch_recorded_is_not_claimed`. An entry claimed by
   title now moves to the new digest at once, so the superseded digest no
   longer lingers
   (`a_title_claim_moves_the_entry_to_the_reworded_digest`,
   `a_title_claimed_entry_can_be_forgotten`).
6. **P6: a stopped pass lost the applied count** (fixed). `settle_identities`
   now fails with a `SettlementFailure` carrying how many changes had
   landed. Covered by
   `a_pass_stopped_mid_way_counts_the_changes_that_landed_before_it`.
   The failure also carries the identity and promotion rows the pass
   reported, and `work sync` prints them before the note
   (`a_stopped_run_prints_the_identity_changes_that_landed_before_its_note`).
7. **P7: the rerun guard ran before the create lock** (fixed). The
   tracker-keyed create checks for a pending create while holding the
   create lock. Covered by
   `a_draft_written_while_a_create_waits_for_the_lock_is_e_draft_exists`.
8. **P8: failures were swallowed silently** (fixed).
   - `work resolve` reports an unlistable corpus as `E_RESOLVE_UNLISTABLE`
     (exit 1), and `sync --target` fails on it instead of treating the
     token as a remote candidate.
   - A legacy `create --push` fails when the corpus cannot be listed,
     rather than reusing a `created` marker's key.
   - `work sync` and `work promote` warn about each unreadable promotion
     record by path.

   Covered by
   `an_unlistable_corpus_is_reported_rather_than_matched_by_filename`,
   `a_token_resolved_over_an_unlistable_corpus_fails_as_an_error`,
   `a_created_marker_over_an_unlistable_corpus_fails_rather_than_reusing_its_key`
   and `each_unreadable_promotion_record_is_named_in_a_warning`.
9. **P9: minor** (fixed).
   - `link_external_id` holds the file's write lock
     (`a_link_waits_for_an_edit_holding_the_file_lock_and_keeps_it`).
   - Following a moved key holds the retirement lock
     (`following_an_external_id_holds_the_retirement_lock`).
   - An unknown create outcome stays `possible-duplicate` even when its
     failure detail cannot be written
     (`an_unknown_create_outcome_stays_unknown_when_its_failure_cannot_be_noted`).
   - `work promote` refuses a non-`{tracker}` pattern with
     `E_PROMOTE_NOT_TRACKER_OWNED`; before the fix it promoted the draft.
     It names both claimants of a conflicting draft ID with
     `E_PROMOTE_CONFLICTING`.
   - A retirement locks and verifies files before taking recovery copies,
     so a file edited after planning leaves no directory behind
     (`a_file_changed_after_the_snapshot_leaves_no_recovery_directory`).

   A fresh retirement whose recovery copies fail to write removes the
   directory it prepared
   (`a_recovery_copy_that_fails_to_write_leaves_no_recovery_directory`,
   driven by a fault-injecting recovery store added to the test harness).

### Manual Testing Required:

1. Live tracker (a scratch Linear team and a scratch Jira project):
   - [ ] `mise run test:integration:tracker-contract` passes.
   - [ ] `work create "T" task low --push` under `{tracker}` writes
     `meta/work/<KEY>-t.md`, the remote H1 shows `<KEY>`, and `work sync
     --preview` reports `synced`, for Linear and for Jira.
   - [ ] A cross-team or cross-project move: `work sync --preview` reports
     `key-changed PP-x->ENG-y`, and the applied run updates `external_id`,
     for Linear and for Jira.
   - [ ] With two drafts, one referenced from a plan, `work sync` promotes
     both and rewrites the plan, and `work resolve <old-draft-id>`
     returns the promoted path. With networking off, both drafts are left
     untouched and the run exits 70.
2. Skills in a scratch `{tracker}` repo:
   - [ ] `/accelerator:create-work-item`: declining offers **No, save as
     draft**; accepting yields a tracker-keyed item.
   - [ ] `/accelerator:extract-work-items` over an epic with two children
     makes one push offer. Accepting links the children's `parent` to the
     epic key. Declining yields three linked drafts, which
     `/accelerator:sync-work-items` promotes, rewriting the children's
     `parent`.
3. Local:
   - [ ] Ctrl-C a scratch retirement mid-rewrite; the next retirement
     completes it.
   - [ ] ADR-0070's decision matches 0230's Requirements section.

### Recommendations:

- Run the tracker contract suite and the live manual checks, then
  re-validate. The plan stays `in-progress` until then.
- Find out why `aside-row-resolved-colours.spec.ts` fails under full load,
  or confirm it fails the same way on the parent commit.
