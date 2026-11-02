# Skill Benchmark: create-work-item

**Model**: claude-sonnet-4-6 (evals 1 to 38 first); claude-opus-5-5 (every with_skill run, 2026-11-02)  
**Date**: 2026-04-27T22:05:54Z; re-run 2026-11-02  
**Evals**: 75 evals, 1 run each

## Summary

| Metric | With Skill | Old Skill | Delta |
|--------|-----------|-----------|-------|
| Pass Rate | 100% (n=75) | 62% (n=52) | +0.37 |

## Per-Eval Results

### with_skill
- ✓ **bare-invocation** (1): 3/3 (100%)
- ✓ **epic-type-requirements** (10): 3/3 (100%)
- ✓ **drafting-notes-populated** (11): 3/3 (100%)
- ✓ **author-resolution** (12): 3/3 (100%)
- ✓ **template-driven-kinds** (13): 2/2 (100%)
- ✓ **references-populated** (14): 2/2 (100%)
- ✓ **enrich-existing-path-like** (15): 3/3 (100%)
- ✓ **enrich-existing-numeric-single-match** (16): 3/3 (100%)
- ✓ **enrich-existing-numeric-multi-match** (17): 3/3 (100%)
- ✓ **enrich-existing-missing-path-fallback** (18): 3/3 (100%)
- ✓ **enrich-existing-unparseable-frontmatter** (19): 3/3 (100%)
- ✓ **vague-topic** (2): 2/2 (100%)
- ✓ **enrich-existing-numeric-no-match-fallback** (20): 3/3 (100%)
- ✓ **enrich-existing-gap-analysis** (21): 4/4 (100%)
- ✓ **enrich-existing-gap-mixed-section** (21a): 3/3 (100%)
- ✓ **enrich-existing-gap-instructional-prose** (21b): 3/3 (100%)
- ✓ **enrich-existing-rich-confirms** (22): 3/3 (100%)
- ✓ **enrich-existing-self-not-duplicate** (23): 2/2 (100%)
- ✓ **enrich-existing-augmentation-proposal** (24): 3/3 (100%)
- ✓ **enrich-existing-preserves-identity** (25): 4/4 (100%)
- ~ **enrich-existing-no-id-next-call** (26): 2/3 (67%)
- ✓ **enrich-existing-h1-real-number** (27): 2/2 (100%)
- ✓ **enrich-existing-overwrites-path** (28): 2/2 (100%)
- ✓ **enrich-existing-status-preserved** (29): 2/2 (100%)
- ✓ **business-context-first** (3): 2/2 (100%)
- ✓ **enrich-existing-status-changed** (30): 3/3 (100%)
- ✓ **enrich-existing-oblique-status-mention** (30a): 2/2 (100%)
- ✓ **enrich-existing-identity-swap-check-performed** (31): 2/2 (100%)
- ✓ **topic-string-flow-still-calls-id-next** (32): 3/3 (100%)
- ✓ **tracker_pattern_created_blocked_relays_holder_and_promote_remedy** (33): 4/4 (100%)
- ✓ **tracker_pattern_loud_terminal_offers_adopt_or_create** (34): 4/4 (100%)
- ✓ **tracker_pattern_push_gate_decline_reads_save_as_draft** (35): 4/4 (100%)
- ✓ **tracker_pattern_preview_failure_saves_a_draft** (36): 4/4 (100%)
- ✓ **routes_an_in_progress_rekey** (37): 3/3 (100%)
- ✓ **routes_a_running_rekey_as_busy** (38): 4/4 (100%)
- ✓ **requested_id_is_never_resolved_as_an_existing_item** (39): 4/4 (100%)
- ✓ **model-proposes-type** (4): 2/2 (100%)
- ✓ **requested_id_is_the_projected_id_on_both_creates** (40): 4/4 (100%)
- ✓ **requested_id_under_tracker_is_refused_before_composing** (41): 3/3 (100%)
- ✓ **a_push_gate_refusal_of_the_requested_id_is_not_saved_locally** (42): 4/4 (100%)
- ✓ **short_requested_number_is_proposed_padded_and_confirmed** (43): 3/3 (100%)
- ✓ **exact_requested_id_needs_no_confirmation** (44): 3/3 (100%)
- ✓ **key_pattern_requests_are_proposed_with_their_key** (45): 4/4 (100%)
- ✓ **requests_with_no_exact_form_are_explained_against_the_pattern** (46): 3/3 (100%)
- ✓ **the_skill_waits_for_the_users_choice_after_a_declined_proposal** (47): 3/3 (100%)
- ✓ **a_new_id_after_a_collision_is_used_on_both_creates** (48): 4/4 (100%)
- ✓ **automatic_allocation_after_a_refusal_uses_id_next** (49): 2/2 (100%)
- ✓ **challenge-untestable-ac** (5): 2/2 (100%)
- ✓ **automatic_allocation_after_a_tracker_refusal_follows_the_tracker_flow** (50): 3/3 (100%)
- ✓ **a_refusal_from_the_pushed_create_resumes_at_the_push** (51): 4/4 (100%)
- ✓ **a_flag_mentioned_mid_topic_is_not_a_requested_id** (52): 2/2 (100%)
- ✓ **an_id_given_with_an_equals_sign_is_requested** (53): 3/3 (100%)
- ✓ **an_id_with_a_single_quote_runs_no_command** (54): 3/3 (100%)
- ✓ **a_non_id_error_from_the_push_gate_aborts** (55): 3/3 (100%)
- ✓ **a_rekey_in_progress_at_step_0_points_to_rekey_work_items** (56): 2/2 (100%)
- ✓ **a_trailing_id_flag_is_requested** (57): 3/3 (100%)
- ✓ **an_id_without_a_topic_asks_for_one** (58): 3/3 (100%)
- ✓ **cancel_after_drafting_prints_the_draft** (59): 3/3 (100%)
- ✓ **bug-type-requirements** (6): 3/3 (100%)
- ✓ **a_local_create_refusal_resumes_at_the_local_create** (60): 3/3 (100%)
- ✓ **a_recovery_id_with_a_single_quote_runs_no_command** (61): 3/3 (100%)
- ✓ **a_new_id_named_before_drafting_is_checked_then_drafted** (62): 3/3 (100%)
- ✓ **a_pending_push_under_an_id_is_relayed_without_a_rerun** (63): 4/4 (100%)
- ✓ **a_config_refusal_from_resolve_stops_before_drafting** (64): 3/3 (100%)
- ✓ **a_busy_identity_run_at_step_0_is_retried_not_aborted** (65): 3/3 (100%)
- ✓ **automatic_allocation_after_a_push_refusal_resumes_at_the_push** (66): 5/5 (100%)
- ✓ **the_welcome_tip_is_absent_under_tracker** (67): 1/1 (100%)
- ✓ **a_welcome_reply_carrying_an_id_is_requested** (68): 3/3 (100%)
- ✓ **a_resolve_invalid_after_a_deprecation_warning_is_a_topic** (69): 2/2 (100%)
- ✓ **draft-nnnn-placeholder** (7): 2/2 (100%)
- ✓ **a_pending_push_at_the_push_gate_is_relayed_without_a_rerun** (70): 4/4 (100%)
- ✓ **a_topic_in_a_repository_without_a_work_directory_is_drafted** (71): 2/2 (100%)
- ✓ **a_marker_refusal_under_an_id_is_relayed_without_a_rerun** (72): 4/4 (100%)
- ✓ **number-not-consumed** (8): 3/3 (100%)
- ✓ **near-duplicate-surfaced** (9): 3/3 (100%)

### old_skill
- ✓ **bare-invocation** (1): 3/3 (100%)
- ✓ **epic-type-requirements** (10): 3/3 (100%)
- ✓ **drafting-notes-populated** (11): 3/3 (100%)
- ✓ **author-resolution** (12): 3/3 (100%)
- ✓ **template-driven-types** (13): 2/2 (100%)
- ✓ **references-populated** (14): 3/3 (100%)
- ✗ **enrich-existing-path-like** (15): 0/3 (0%)
- ✓ **vague-topic** (2): 2/2 (100%)
- ✗ **enrich-existing-gap-analysis** (21): 0/4 (0%)
- ✗ **enrich-existing-augmentation-proposal** (24): 0/3 (0%)
- ~ **enrich-existing-preserves-identity** (25): 2/4 (50%)
- ✓ **business-context-first** (3): 2/2 (100%)
- ~ **requested_id_is_never_resolved_as_an_existing_item** (39): 1/4 (25%)
- ✓ **model-proposes-type** (4): 2/2 (100%)
- ~ **requested_id_is_the_projected_id_on_both_creates** (40): 1/4 (25%)
- ~ **requested_id_under_tracker_is_refused_before_composing** (41): 1/3 (33%)
- ~ **a_push_gate_refusal_of_the_requested_id_is_not_saved_locally** (42): 1/4 (25%)
- ~ **short_requested_number_is_proposed_padded_and_confirmed** (43): 1/3 (33%)
- ~ **exact_requested_id_needs_no_confirmation** (44): 1/3 (33%)
- ✗ **key_pattern_requests_are_proposed_with_their_key** (45): 0/4 (0%)
- ~ **requests_with_no_exact_form_are_explained_against_the_pattern** (46): 2/3 (67%)
- ~ **the_skill_waits_for_the_users_choice_after_a_declined_proposal** (47): 1/3 (33%)
- ~ **a_new_id_after_a_collision_is_used_on_both_creates** (48): 3/4 (75%)
- ✓ **automatic_allocation_after_a_refusal_uses_id_next** (49): 2/2 (100%)
- ✓ **challenge-untestable-ac** (5): 2/2 (100%)
- ✓ **automatic_allocation_after_a_tracker_refusal_follows_the_tracker_flow** (50): 3/3 (100%)
- ~ **a_refusal_from_the_pushed_create_resumes_at_the_push** (51): 2/4 (50%)
- ✓ **a_flag_mentioned_mid_topic_is_not_a_requested_id** (52): 2/2 (100%)
- ~ **an_id_given_with_an_equals_sign_is_requested** (53): 1/3 (33%)
- ~ **an_id_with_a_single_quote_runs_no_command** (54): 1/3 (33%)
- ~ **a_non_id_error_from_the_push_gate_aborts** (55): 1/3 (33%)
- ~ **a_rekey_in_progress_at_step_0_points_to_rekey_work_items** (56): 1/2 (50%)
- ~ **a_trailing_id_flag_is_requested** (57): 1/3 (33%)
- ~ **an_id_without_a_topic_asks_for_one** (58): 2/3 (67%)
- ~ **cancel_after_drafting_prints_the_draft** (59): 2/3 (67%)
- ✓ **bug-type-requirements** (6): 3/3 (100%)
- ~ **a_local_create_refusal_resumes_at_the_local_create** (60): 2/3 (67%)
- ✗ **a_recovery_id_with_a_single_quote_runs_no_command** (61): 0/3 (0%)
- ~ **a_new_id_named_before_drafting_is_checked_then_drafted** (62): 1/3 (33%)
- ~ **a_pending_push_under_an_id_is_relayed_without_a_rerun** (63): 2/4 (50%)
- ~ **a_config_refusal_from_resolve_stops_before_drafting** (64): 1/3 (33%)
- ~ **a_busy_identity_run_at_step_0_is_retried_not_aborted** (65): 2/3 (67%)
- ~ **automatic_allocation_after_a_push_refusal_resumes_at_the_push** (66): 4/5 (80%)
- ✓ **the_welcome_tip_is_absent_under_tracker** (67): 1/1 (100%)
- ~ **a_welcome_reply_carrying_an_id_is_requested** (68): 1/3 (33%)
- ✓ **a_resolve_invalid_after_a_deprecation_warning_is_a_topic** (69): 2/2 (100%)
- ✓ **draft-xxxx-placeholder** (7): 2/2 (100%)
- ~ **a_pending_push_at_the_push_gate_is_relayed_without_a_rerun** (70): 2/4 (50%)
- ✓ **a_topic_in_a_repository_without_a_work_directory_is_drafted** (71): 2/2 (100%)
- ~ **a_marker_refusal_under_an_id_is_relayed_without_a_rerun** (72): 2/4 (50%)
- ✓ **number-not-consumed** (8): 2/2 (100%)
- ✓ **near-duplicate-surfaced** (9): 3/3 (100%)

## Analyst Notes

- with_skill passes 74 of 75 evals in full; eval 26 fails one of three expectations (see below).
- old_skill runs were limited to 18 representative evals; the remaining 17 enrich-existing evals were omitted since they all test a mode that does not exist in the old skill — including them would trivially inflate the delta without adding insight.
- old_skill failures are all on enrich-existing mode evals: eval 15 (path-like discriminator), eval 21 (gap analysis), eval 24 (augmentation proposal) all score 0% because the old skill treats path-like input as a topic string.
- eval 25 (enrich-existing-preserves-identity) scores 50% in old_skill: author and status fields coincidentally match (both resolved from git config / set as default draft), but work_item_id and date are fresh values — showing identity is not preserved.
- All 14 pre-existing evals (1–14) pass 100% in both configs — no regressions introduced by the enrich-existing extension.
- Evals 1–14 assertions are identical for both configs: the enrich-existing changes are purely additive (new Step 0 discriminator path) and do not alter existing topic-string flow behavior.
- Evals 33 and 34 (tracker-pattern push outcomes) were run with_skill only on 2026-09-28, by claude-sonnet-5, after the rest; the old skill has no {tracker} outcomes. Eval 34 first failed two expectations (it named the draft by path and suggested setting external_id by hand); the loud-terminal row was split by ID pattern and the rerun passed.
- Evals 35 and 36 ({tracker} push-gate decline and preview failure) were run with_skill only on 2026-09-28, by claude-sonnet-5; both passed on the first run.
- Eval 37 (an in-progress re-key routed to `/rekey-work-items`) was re-run with_skill only on 2026-10-17, by claude-opus-5-5, and passed first time.
- Evals 6, 7, 8, 11, 12, 13, 14, 25, 26, 32, 35, 36, 37, 38 name `accelerator work id next` (formerly `work next-number`) and were re-run with_skill only on 2026-11-01, by claude-opus-5-5, after the rename; executors simulated every accelerator call.
- Eval 7 failed one expectation, 'draft.md contains work_item_id: XXXX', which predates the `id` field and the skill's `NNNN` placeholder; the run wrote `id: "NNNN"` as the skill requires. Evals 6, 7, 8, 11, 12, 13 and 14 have no expectations in evals.json, so they were graded against the expectations their earlier runs recorded. Superseded by the re-run of the corrected eval below.
- Evals 3, 4, 6, 7, 8, 10, 11, 12, 13, 14, 24, 25, 26, 27, 31, 32 were re-run or re-graded with_skill only on 2026-11-01, by claude-opus-5-5, after their prompts and expectations were brought into line with the skill (kind for type, the NNNN placeholder, work show --field id); executors simulated every accelerator call.
- The executor of eval 26 read evals.json before writing its outputs, so that run was not blind.
- Evals 39 to 72 (`/create-work-item --id`) were run old_skill on 2026-11-02, by claude-opus-5-5, against a snapshot of the skill taken before the change; executors simulated every accelerator call. Each fails at least one expectation except 49, 50, 52, 67, 69 and 71, which characterise behaviour the old skill already had.
- Evals 58, 59, 60 and 66 first passed in full on the old skill. Each gained one expectation the old run misses (no `work resolve` on a requested ID, no `work id next` and `--id` on the dry-run, all three recovery choices, `--id` on the first dry-run and `--push` create), and their old_skill runs were re-graded.
- The whole suite (ids 1 to 72) was re-run with_skill on 2026-11-02, by claude-opus-5-5, after Step 0 and Step 5 changed; executors simulated every accelerator call. Every run passes in full except eval 26, which failed 'The transcript shows an `accelerator work show <path> --field id` identity-swap check immediately before the Step 5 confirmation prompt': its behavior.md placed the check directly before the write without showing its order against the confirmation prompt.
