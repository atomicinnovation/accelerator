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
last_updated: "2026-09-29T10:56:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Tracker-Owned Work Item ID Generation Implementation Plan

Every phase's code changes are in place, and the full local CI mirror passes.
The result is `partial` for two reasons. First, the tracker contract suite
has not run, because it needs live Jira and Linear tenants. Second, eleven
manual checks are still open, most of which also need a live tenant. The
code review found three defects worth fixing before merge (P1–P3), and all
three are now fixed. The lower-severity findings (P4–P9) are still open.

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

✓ `mise run` (the full default task) exits 0 with the P1–P3 fixes applied.
That covers 4,032 CLI unit tests (1 skipped, 1 reported leaky) plus every
integration, e2e, docs and tasks lane. Before the fixes it also exited 0 on
`6b815050`, over 4,025 CLI unit tests.
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

- **Missing Phase 6 tests.** Three planned tests do not exist, and no
  implementation note names them:
  - `two_clean_rollbacks_rewind_the_record_and_the_next_promotion_creates_no_new_issue`
  - `a_promotion_killed_mid_rewrites_before_to_resumes_without_a_new_issue`
    (arguably covered by the per-operation kill test)
  - `a_create_killed_after_sending_leaves_a_draft_that_promotion_refuses_as_a_possible_duplicate`

  `rerunning_a_create_sends_no_second_create_at_any_record_stage` covers
  `Attempted` and `Created`, but not `RemoteKept` or `Retiring`
  (`cli/work-cli/src/create.rs:2119`).
- **Stale success criteria (Phase 7).** The sync promotion tests live in
  `work-adapters/tests/sync_settled.rs`, as a note records. The phase's
  success-criteria command `--test sync_create` therefore does not run
  them.
- **Shared hierarchy walk (Phase 8).** `child_index` stays private in
  `cli/work-cli/src/list.rs:359`. Key Discoveries planned to move it into
  `work` for 0291.
- **Documented deviations.** The following are recorded in the plan's
  implementation notes and are equivalent or improvements:
  - `SettlementPorts` in place of extending `SyncPorts`;
  - `IdentityRow` in place of new `Action` variants;
  - `keys_held_by_promotions` in place of extending `followed_keys`;
  - byte comparison in place of digests during retirement;
  - `BadIdentifier` → `RequestInvalid` in Jira;
  - the journal at `.accelerator/state/batch-journal/journal.json`.

#### Potential Issues:

Ranked most severe first. Failing tests reproduced P1–P3, and all three
are fixed. I confirmed P4 by reading the code. P5–P9 come from the review
agents and are plausible, but I have not reproduced them.

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
   it. Still open elsewhere: sync's create-from-local
   (`cli/work-adapters/src/sync/run.rs:373`, `:686`) and `work list`
   (`cli/work-cli/src/list.rs:89`) still read `title` through
   `read_field_raw`.
3. **P3: `--adopt` compares keys case-sensitively** (confirmed).
   `key != named` on `ExternalId` is an exact string comparison
   (`cli/work/src/promotion.rs:297`). `--adopt pp-900` against a recorded
   `PP-900` therefore stops as `adopt-conflicts-with-recorded-key`,
   although resolution elsewhere ignores case. **Fixed:** the comparison
   now ignores case, and
   `adopting_the_recorded_key_in_another_case_resumes_onto_it` covers it.
4. **P4: the extract skill contradicts the CLI on empty paths**
   (confirmed). `skills/work/extract-work-items/SKILL.md:640` says an
   empty path appears only on legacy `created-unwritten`. `create-batch`
   also prints one for `pending` from a legacy slug marker and for a
   journalled `created-unwritten` under `{tracker}`
   (`cli/work-cli/src/create_batch.rs:328-339`, `:434-439`).
5. **P5: the batch journal matches by title across batches.** A later,
   unrelated batch within 30 days whose entry shares a title, such as "Add
   tests", is claimed and never created
   (`cli/work-cli/src/batch_journal.rs:101-103`).
6. **P6: a mid-pass stop loses the applied count.** `run_settled` maps any
   `settle_identities` error to `failed(0)`
   (`cli/work-adapters/src/sync/settled_run.rs:115`). A `RetirementIncomplete`
   after earlier promotions or key changes landed therefore drops both the
   `identity-applied-before-refusal` note and those rows.
7. **P7: the rerun guard is racy.** `pending_create` runs before the create
   lock is taken (`cli/work-cli/src/create.rs:1259` vs `:598`), so two
   concurrent identical `create --push` runs can both send.
8. **P8: several failures are swallowed silently.**
   - `resolve_with` uses `.files().unwrap_or_default()`
     (`cli/work-cli/src/resolve.rs:127`).
   - `corpus_carries_external_id` treats a listing failure as "not
     carried" (`cli/work-cli/src/create.rs:322`).
   - `reconcile_promotions` and `keys_held_by_promotions` `.flatten()` away
     unreadable records, where the plan requires a warning
     (`identity_settlement.rs:291`, `:365`).
9. **P9: minor.**
   - `link_external_id` read-modify-writes without a lock
     (`cli/work-cli/src/sync_author.rs:222`).
   - An unknown create outcome whose record re-save fails reports
     `local-save` (exit 0) instead of `loud-terminal`
     (`cli/work-adapters/src/promotion.rs:539`).
   - `promote.rs:174` hardcodes `IdOwnership::Tracker` without checking
     `id_pattern`.
   - A `Conflicting` resolution surfaces as `E_PROMOTE_NOT_A_DRAFT`
     (`cli/work-cli/src/promote.rs:133`).
   - Recovery directories left by a failure before the lock step are never
     swept (`cli/corpus-adapters/src/recovery.rs:181`).

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

- Move sync's create-from-local and `work list` onto the YAML title
  reader too, the remaining sites of P2's defect.
- Correct the empty-path sentence in `extract-work-items/SKILL.md` (P4),
  and scope the journal's title fallback to one manifest (P5).
- Write the three missing Phase 6 tests, or record in the plan why the
  per-operation kill test covers them.
- Point Phase 7's success-criteria command at `--test sync_settled`.
- Run the tracker contract suite and the live manual checks, then
  re-validate. The plan stays `in-progress` until then.
