---
type: "plan-validation"
id: "2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion-validation"
title: "Validation Report: Budget Web Searches and Halt on Exhaustion Implementation Plan"
date: "2026-10-17T13:07:05+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion"
tags: ["research", "deep-research", "conduct", "web-profile", "research-cli", "run-ledger"]
last_updated: "2026-10-17T13:07:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Budget Web Searches and Halt on Exhaustion Implementation Plan

### Implementation Status

✓ Phase 1: Worst-Case Tree and Budget Sizing - Fully implemented
✓ Phase 2: Search Cap from the Environment - Fully implemented
✓ Phase 3: Budgets in the Run Ledger and the Plan - Fully implemented
✓ Phase 4: Budget Delivery to Researchers and Level Notes - Fully implemented
✓ Phase 5: Quota Exhausted Outcome - Fully implemented
✓ Phase 6: Halting - Fully implemented

Every test named in the plan's tables exists and asserts what its row
states. The 0307 and 0309 conditional steps were correctly skipped: both
work items are still `draft` at revision `360c0570`.

### Automated Verification Results

✓ Full local CI mirror: `mise run` exits 0, and the working copy is
  unchanged afterwards (no formatter drift)
✓ ADR validates: `accelerator corpus frontmatter validate --file
  meta/decisions/ADR-0073-*.md` (status `accepted`)

`mise run` covers every per-phase automated criterion: the `cargo test`
runs, `cli:check`, `public-api:check`, `build-system:check` and the
structure and coupling pytest suites. An earlier `mise run` failed in
`test:e2e:visualiser` with `Cannot find module '../../../package.json'`.
It overlapped a second `mise run` that was reinstalling the frontend, and
the branch touches no visualiser or frontend file. The second run, alone,
passed.

### Code Review Findings

#### Matches Plan:

- `Depth::requested` bounds requested depth at `DEEPEST = 10`, while
  `Depth::new` stays unbounded for stamped depths
  (`cli/research/src/topic/tree.rs:24-35`). The `depth: 12` row pins this.
- `capped_follow_ups` is the only place the cap is applied
  (`tree.rs:419-427`). The worst-case count inserts into a clone of
  `known`, so it records no trims (`tree.rs:367-380`).
- Sizing follows the planned pipeline: empty tree, then the floor, then a
  binary search over `[1, 4·available + 4]`, then a deepest-first top-up
  (`cli/research/src/conduct/search_budget.rs:67-196`). No overflow or
  off-by-one was found, and the enumerated property test covers fit,
  floor, shape and maximality.
- The harness parse moved to `harness_setting.rs` unchanged. The search
  cap names its variable only in the files the coupling guard admits.
- `run_outstanding` keeps the planned notice order and now prints notices
  ahead of a planning error (`cli/research-cli/src/topic_command.rs:296-300`).
- The ledger stores `search_budgets` and `withheld` under the
  `owed_quarantine` precedent: written only when present, an absent key
  decodes as the default, and a malformed value is corrupt.
- Withholding is one filter after the `attempted` split
  (`cli/research/src/conduct/window.rs:98-100`): it spares compositions,
  frees slots, and keeps attempted withheld spawns in `unfinished`.
- The skill defines halting failures once (`research-topic/SKILL.md:622-634`),
  and steps 2 and 4–7 carry the halt as planned.

#### Deviations from Plan:

- Step 5 had no explicit rule that the halting bullet overrides the
  no-file bullet. Fixed after validation: the no-file bullet now points a
  halting reason on to the halt, the halting bullet states its precedence,
  and `test_a_halting_reason_overrides_the_no_file_bullet` pins both.
- Step 6 lacks the planned sentence "The ledger never offers a withheld
  spawn, so the loop ends when the plan offers nothing else". The generic
  loop-end rule stands in for it (`SKILL.md:510-523`).
- `a_start_that_cannot_plan_still_reports_its_concurrency_notices`
  (`cli/research-cli/tests/topic_outstanding.rs:2700-2707`) asserts that
  both messages appear, not that they appear in order. The code does emit
  them in order.
- `SearchBudgets::from_parts` is `const fn` (no effect).

#### Potential Issues:

Each issue was investigated after validation. Two were fixed; three stand,
for the reasons given.

- Fixed: a hand-run re-offer with `--run R --withhold-profile web` and no
  `--spawned` moved a pending batch's web researcher spawns into
  `attempted`, because recording treats a spawn dropped from the offer as
  one that wrote. The window now names the spawns it held back, and
  recording leaves them unattempted. Pinned in the domain ledger tests and
  by `withholding_a_re_offered_batch_leaves_its_spawns_unattempted`.
- Fixed: the ledger codec read level keys with a plain integer parse, so
  `"01"` and `"+1"` both decoded as level 1 and could silently merge. A
  key must now be the form the writer prints, or the ledger is corrupt.
- Not fixed: a level without a stored budget gets 1, and demand is not
  re-checked. `conduct` holds depth and the outline fixed for a run, so
  only a hand run with a larger `--depth`, or a level reopened by a
  quarantined note, reaches it, and a budget of 1 errs towards the cap.
  The plan accepts this, and `a_level_without_a_budget_is_budgeted_one`
  pins it.
- Not fixed: `--withhold-profile` accepts a misspelt name. `conduct` only
  ever passes `web`, from the Halting failures table, so only a hand run
  can misspell it, and `withheld` echoes the name back to that user.
- Not fixed: halted capacity-refused spawns carry the
  concurrent-subagent-limit reason, as AC 30–31 require. The reason is
  accurate, and the Halt section above Failures gives the `/clear`
  recovery. The stored `demand` is still trusted, since the ledger does
  not store the node counts it could be checked against.

### Manual Testing Required:

The plan records every manual check as done, including the headless halt
run in phase 6. This validation did not repeat them. Remaining
pre-merge check:

1. Integration:
  - [ ] Re-check that 0307 and 0309 are still not `done` at merge time.
        If either has landed, apply phase 4 step 8 or phase 5 step 4.

### Recommendations:

- Optionally assert the notice order in
  `a_start_that_cannot_plan_still_reports_its_concurrency_notices`.
