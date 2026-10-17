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

- A hand-run re-offer with `--run R --withhold-profile web` and no
  `--spawned` displaces a pending batch's web researcher spawns.
  `PendingBatch::replace` (`cli/research/src/conduct/memory.rs:39-51`)
  adds them to `attempted`, so they then appear in `unfinished`. `conduct`
  always passes `--spawned {batch}`, so only hand runs reach this, and no
  test covers it.
- A level without a stored budget silently gets 1, and demand is not
  re-checked (`search_budget.rs:91-93`). This happens when a later plan
  passes a larger `--depth`, when a level sized away reopens (for example,
  note `1` is quarantined after a re-run sized only levels 2–3), or when
  web pairs are added to the outline mid-run. The plan accepts this by
  design, but no test pins it.
- `--withhold-profile` accepts any non-empty name, including a misspelling
  such as `wbe`. `withheld` echoes the name back, so step 2's check that
  the expected profile is named would not catch a misspelling.
- The ledger codec parses level keys with `str::parse::<u32>`, so `"01"`
  and `"+1"` both decode as level 1 and silently collapse into one entry
  (`cli/research-adapters/src/conduct/ledger.rs:203`). The stored `demand`
  is trusted rather than checked against the levels. Only a hand-edited
  ledger can produce either.
- Halted capacity-refused spawns are recorded with the
  concurrent-subagent-limit reason, so Failures shows that row's "re-run
  once other subagents have finished". The Halt section gives the correct
  `/clear` recovery. This follows the plan as written.

### Manual Testing Required:

The plan records every manual check as done, including the headless halt
run in phase 6. This validation did not repeat them. Remaining
pre-merge check:

1. Integration:
  - [ ] Re-check that 0307 and 0309 are still not `done` at merge time.
        If either has landed, apply phase 4 step 8 or phase 5 step 4.

### Recommendations:

- Either refuse `--withhold-profile` while a pending batch holds that
  profile's researcher spawns and `--spawned` is absent, or keep withheld
  spawns out of `displaced`. Add a CLI test for the case.
- Consider refusing a `--withhold-profile` name that no outline pair uses,
  so a misspelling fails loudly.
- Optionally tighten the codec to reject non-canonical level keys, and
  assert the notice order in
  `a_start_that_cannot_plan_still_reports_its_concurrency_notices`.
