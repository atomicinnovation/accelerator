---
type: "plan-validation"
id: "2026-10-13-0310-recovering-from-an-interrupted-conduct-run-validation"
title: "Validation Report: Recovering from an Interrupted Conduct Run Implementation Plan"
date: "2026-10-13T22:57:49+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-13-0310-recovering-from-an-interrupted-conduct-run"
tags: ["research", "conduct", "run-ledger", "outstanding", "recovery"]
last_updated: "2026-10-14T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Recovering from an Interrupted Conduct Run Implementation Plan

### Implementation Status

✓ Phase 1: Classify the replaced run's last batch - Fully implemented
✓ Phase 2: Emit `replaced_run` from `outstanding --start` - Fully implemented
✓ Phase 3: The interrupted-run report in `conduct` - Fully implemented
✓ Phase 4: The "unfinished run" recovery row - Fully implemented

### Automated Verification Results

✓ Full local CI: `mise run` exits 0 (430s); working copy unchanged afterwards,
  so no formatter or lint fix was pending
✓ `test:unit:cli`: 5362 passed, 2 skipped (includes `research` and
  `research-cli`, among them `topic_outstanding`)
✓ `public-api:check`, `pup:check`, `cli:check`, `docs:check` all run within the
  default task and passed
✓ `test:integration:skill-invocation`: 141 passed
✓ Structure tests (`tests/unit/tasks/test_research_structure.py`) passed within
  the default task

### Code Review Findings

#### Matches Plan:

- Every planned test name exists: 46 in `research` (outline, plan, evidence,
  `conduct::replaced_run`), 20 in `topic_outstanding.rs`, 17 structure tests;
  renamed tests carry their new names and `replaced_warning` is gone.
- `replaced_run.rs` production code matches the plan's listing verbatim apart
  from rustfmt wrapping.
- `claim_index` (`cli/research/src/topic/plan.rs:321-351`) orders the run's
  claim, a retained finding, a tree, a marker, then a fresh index.
- `run_outstanding` judges findings after `read_round_inputs`, quarantines the
  inputs, and feeds them to `RoundPlan::of`, `report` and `Observed::of`
  (`cli/research-cli/src/topic_command.rs:284-295`); the ledger write precedes
  `render()`.
- The replaced notice reaches stderr only; the corrupt notice reaches both
  `warnings` and stderr; plain, `--run` and corrupt-ledger plans never carry
  the key.
- The public API snapshot gains `ReplacedBatch`, `ReplacedRun` and `Leftover`
  only.
- `SKILL.md` carries the verbatim-print instruction, fixed opening sentence,
  four class headings, quarantine markers, rename-after-print paragraph,
  step 3 wait, **Unfinished run** summary section and the `| an unfinished run |`
  row; none of the forbidden "stopped" family appear.
- Docs and the `[Unreleased] ### Added` `CHANGELOG.md` entry are extended as
  planned; "safe to delete" is gone from both.

#### Deviations from Plan:

- `Planner` gains a private `tree_claims` helper
  (`cli/research/src/topic/plan.rs:146-151`), so `Planner::new` computes tree
  claims twice (once inside `file_claims`). No behavioural effect.
- `SKILL.md:311` says "under step 3's unique-suffix rule" and `:298-299` keys
  the "(quarantined)" marker on "`findings/{stem}.md` is a `quarantine` path";
  both are sharper wording than the plan's.
- The plain-plan guard in
  `quarantined_leftovers_keep_their_fresh_stems_for_the_rest_of_the_run`
  asserts `warnings == []`, stronger than the plan's "no unmatched entry for
  `Z?`" (improvement).

#### Potential Issues:

- Ownership needs an exact stem rebuild (`OutlineItem::pair_at`) while claims
  and `Finding::answers` compare normalised questions. An outline edit that
  keeps the normalised question but changes the ASCII slug (e.g. `A?` →
  full-width `Ａ?`) leaves the stem unowned; a valid finding there answers an
  asked pair, so `Judge::unowned_finding` classes it invalid and quarantines
  it. Contrived and untested; the cost is re-researching one pair.
- The doc comment on `claim_index` (`cli/research/src/topic/plan.rs:317-319`)
  restates the `or_else` chain; only its last sentence ("A tree holds research
  where a marker only reserves an index") is a why.
- The report prints "(quarantined)" before the rename is attempted. If a
  rename fails, the failure paragraph lists the path and stops `conduct`, so
  the user is corrected, but the printed marker is briefly untrue.
- The "root question from a quarantined root marker" case in
  `a_lineage_with_no_note_in_an_orphaned_tree_is_unwritten`
  (`cli/research/src/conduct/replaced_run.rs:692`) uses a hand-set root
  question, not one derived from a real marker.
- `docs-site/src/content/docs/research.md:472` runs to ~88 columns, over the
  80-column convention.

### Manual Testing Required:

The plan's manual criteria are already ticked. Re-run them only if the
potential issues above are addressed:

1. Interrupted-run walkthrough (plan's Testing Strategy, steps 1–8):
  - [ ] Final summary opens with the report verbatim, with no claim that the
        earlier run stopped
  - [ ] Listed findings are `.invalid` markers before the first spawn, and
        `finding_count` excludes them
  - [ ] A corrupt ledger shows only the corrupt-ledger warning
  - [ ] An empty-batch stale ledger shows the opening sentence alone

### Recommendations:

All five potential issues and the duplicate tree-claim computation were
addressed after this report, and `mise run` re-ran green (5366 CLI tests):

- Ownership falls back to the claimant whose profile ends the stem
  (`OutlineItem::pair_ending`), pinned by a domain test and by
  `a_finding_whose_question_was_reworded_to_the_same_normalised_form_is_kept`
  under real NFKC, which failed with the fallback disabled.
- The `claim_index` doc comment keeps only its why.
- The rename-failure paragraph states that an unrenamed path is not
  quarantined, whatever the report marks.
- `a_node_under_a_root_marker_naming_another_question_is_unwritten` builds the
  orphaned tree from a real `.1.md.invalid` marker.
- `docs-site/src/content/docs/research.md:472` is rewrapped to 80 columns.
- `Planner::new` computes tree claims once; `RoundInputs::file_claims` is gone.
