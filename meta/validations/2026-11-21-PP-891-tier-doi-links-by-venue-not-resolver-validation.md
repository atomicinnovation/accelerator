---
type: "plan-validation"
id: "2026-11-21-PP-891-tier-doi-links-by-venue-not-resolver-validation"
title: "Validation Report: Tier DOI Links by Venue, Not by the doi.org Resolver Implementation Plan"
date: "2026-11-23T00:40:43+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-21-PP-891-tier-doi-links-by-venue-not-resolver"
tags: ["research", "research-cli", "queue", "crossref", "web-profile"]
last_updated: "2026-11-23T16:06:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Tier DOI Links by Venue, Not by the doi.org Resolver

The first pass was made at `e7211df7` ("Delete PP-891's baseline research
sets"), and its result was `partial`. Its issues were fixed in the working
copy above that commit, and the fixes are recorded in the plan's
"Validation Follow-ups" section. This report gives the state after those
fixes.

### Implementation Status

✓ Phase 1: Baseline Run and Work-Item Corrections - Fully implemented
✓ Phase 2: arXiv Queue Characterisation - Fully implemented
✓ Phase 3: Generalise the Queue Engine - Fully implemented
✓ Phase 4: `fetch doi metadata` on the Anonymous Pool - Fully implemented
✓ Phase 5: `crossref.mailto` and the Polite Pool - Fully implemented
✓ Phase 6: OpenAlex Search/Lookup Parity Test - Fully implemented
✓ Phase 7: Web Profile and Finding Outputter Text - Fully implemented
✓ Phase 8: Closing Manual Run - Fully implemented (recorded in the work
item's Closing Run section)

### Automated Verification Results

✓ The bare `mise run` exits 0. All 8479 CLI tests pass, and none needed a
  retry. Line coverage is 94.42%, above the 92% floor.
✓ All 556 tests in `uv run pytest tests/unit/tasks/test_research_structure.py`
  pass.
✓ The `public-api` snapshot diff adds only `UnreadableWork` and the new
  signature of `CrossrefDecoder::work`.
✓ `mise run cli:check` passes: rustfmt and clippy are clean.
✓ No test body outside `support/` names a state path. The `rg` check from
  phase 3 prints nothing.
✓ The work item and the plan both pass
  `accelerator corpus frontmatter validate`.

First pass, for the record: the bare `mise run` exited 1 on
`each_pool_keeps_to_its_own_limits_when_both_are_busy`. The polite pool's
last start came 1.014 s after its first, against a 1 s limit, because one
hand-off took 76 ms.

### Code Review Findings

#### Matches Plan:

- `Service` and `CrossrefPool` carry the planned pacing:
  - arXiv: 3 s after the previous finish;
  - Crossref public: 210 ms after the previous start, 1 slot;
  - Crossref polite: 102 ms after the previous start, 3 slots.
- `PacingState` follows the planned rules:
  - `confirm` never reads `next_free`;
  - `Deferred` requires both `not_before > now` and `not_before > send_at`;
  - `release` rolls back only the latest reservation;
  - `trust` applies each of its repairs at the planned boundaries.
- The file gate reads the wall clock only once it holds `pacing.lock`, and
  never sleeps while holding it. Companion locks behave as planned.
- Foreign ticket directories are listed read-only. `serving`, `admission`
  and `ports` never import `sources::fetch`.
- These match the plan:
  - Crossref classification and normalisation;
  - the canonical DOI binding;
  - the `crossref_work` request;
  - `ContactAddress::parse`, the `open_value` ladder and `OpenKey`;
  - the two warning texts.
- The skill and docs text covers every phase 7 criterion, and each one has
  a structure pin.

#### Deviations from Plan:

Departures recorded in each phase's Outcome or in "Validation Follow-ups"
are not repeated here.

- The web profile's Waiting text drops the plan's "Unlike the arXiv
  profile…" clause. The meaning is unchanged.
- The rate tests launch all N processes together, not "over 5 concurrent
  process slots". This is stricter than the plan.
- Some test and help-text lines exceed 80 columns, but all of them are
  string literals that rustfmt cannot wrap. The `cli.rs` help lines are
  over 80 only because of their `\"` escapes, and they render within 80.

#### Potential Issues:

- The web profile states the run-step order the work item requires, with
  the resolver `WebFetch` after `web metadata` and before `doi metadata`,
  but gives no reason for it. In phase 8 the researcher ran `doi metadata`
  first. The order does not affect the tier. Either give a reason or relax
  the criterion; that choice belongs to the item's author.
- Two values in `main.rs:384-407` stay consistent only by convention, not
  by type: the pool the adapters are built with, and the `Service` passed
  to rendering.
- After a `trust` clamp moves `next_free`, `release` can no longer roll
  back a refused reservation. The pool loses at most one interval of
  capacity, which the plan accepts.
- The liveness tests pass on a retry when the machine is under load. A
  `FLAKY` line in nextest's output is the early warning that the per-gap
  budget is wearing thin.

### Manual Testing Required:

1. Done and recorded in the plan and the work item:
   - [x] the phase 1 baseline;
   - [x] the phase 8 closing run;
   - [x] live `doi metadata` calls on both pools.
   - [x] `/accelerator:configure help` shows the `crossref` section,
     with this checkout loaded through `claude --plugin-dir`. The
     installed `1.24.0-pre.112` lacks it until this branch is released.

### Recommendations:

- Tick the work item's 53 automated acceptance criteria against this report
  and move its `status` on. Done: every criterion is ticked, and the item
  is `in-progress` until PR #258 merges.
- Decide whether the web profile's step order needs a stated reason, or
  whether the criterion should be relaxed. Still open.

### After the PR #258 Review

Rechecked at `a58a1203`, after the review response. The plan's "PR Review
Follow-ups" section lists the changes. They go beyond the plan, and the
work item's requirements and criteria were amended to match: a tied-DOI
rule, `E_DOI_TICKET_LIVE` handled as `unavailable`, and a diagnostic when a
DOI ticket changes pool. Three criteria were added for them.

✓ The bare `mise run` exits 0: 8482 of 8482 CLI tests pass, 2 skipped.
✓ `/accelerator:configure help` shows the `crossref` section.

Status of the potential issues above:

- **Step order.** Open, as above.
- **Pool and `Service` kept consistent by convention.** Resolved:
  `DoiAdapters.service` comes from the `ServiceAdapters` bundle built for
  the pool.
- **`release` after a `trust` clamp.** Unchanged, and accepted by the plan.
- **Liveness retries.** Unchanged. No `FLAKY` line appeared in this run.

The phase 8 closing run predates the review response, so no live run has
exercised the tied-DOI rule or the new `E_DOI_TICKET_LIVE` handling. The
result stays `pass`.
