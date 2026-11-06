---
type: "plan-validation"
id: "2026-11-05-0338-linear-update-sends-only-given-fields-validation"
title: "Validation Report: Linear Update Sends Only the Fields It Is Given Implementation Plan"
date: "2026-11-06T12:44:06+00:00"
author: "Phil Helm"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-11-05-0338-linear-update-sends-only-given-fields"
tags: ["linear", "cli", "update"]
last_updated: "2026-11-06T12:44:06+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Validation Report: Linear Update Sends Only the Fields It Is Given

Validated at `14adeae0`.

### Implementation Status

✓ Phase 1: One `update_op` Over `UpdateFields` - Fully implemented
✓ Phase 2: `linear update` Sends Only the Flags Given - Fully implemented
✓ Phase 3: Merge-Time Check Against Real Linear - Run on 2026-11-05 against
PP-938, every check passed

### Automated Verification Results

✓ `test:unit:cli`: 7062 passed, 2 skipped, covering `linear-client`'s
`update`, `contract_offline` and `port` suites and `linear-cli`'s
`flow_update`, `flow_transition`, `flow_errors`, `stderr_diagnostics`,
`keyword_surface`, `cli_surface`, `scenario_inventory` and `exit_codes_parity`
✓ Format, lint and types: `format:fix` changed nothing; every `lint:*` and
`types:*` task passes, including `lint:integration-skills:check`
✓ `deny:check`, `pup:check`, `public-api:check`, `notices:check`
✓ `docs:build` and `docs:generate:check`, so the generated
`update-linear-issue` reference page matches the skill
✓ Every other test suite in the default task, including `test:unit:tasks`
(4669 passed), the visualiser unit and e2e suites, and the integration suites
✗ `mise run` exits 1 on two date-gated checks that fail on `main` too:

- `docs:audit:check`: the `GHSA-ch52-4w7c-c8xp` ignore in
  `docs-site/audit-ignores.toml` passed its review-by date, 2026-11-03.
- `test:integration:deny`
  (`test_no_ignore_entry_is_past_its_review_by_date`): the `RUSTSEC-2026-0118`
  and `RUSTSEC-2026-0119` cargo-deny ignores passed the same review-by date.

Neither touches a file this branch changes. Both were found with
`mise run --continue-on-error`; no other task failed.

### Code Review Findings

#### Matches Plan:

- `UpdateFields` and `update_op` live in `cli/linear-client/src/update.rs`
  with the `UPDATE` document, as specified; `retitle_op` is gone, and
  `call_op` and `refuse_identifier_op` are `pub(crate)`.
- The port's `update` and `retitle` delegate to `update_op`, and the new exact
  pin `update_with_a_title_sends_the_title_and_description` sits beside the
  existing pins, none of which was edited.
- `update_op` refuses an empty `UpdateFields` and a refused identifier before
  any request.
- `run_update` takes `&UpdateArgs`, guards with `names_no_field`, resolves
  `--state` through `resolve_requested_state` before any request, and builds
  `UpdateFields` from exactly the flags given with `as_deref()`.
- Every success prints `updated\t<KEY>`; every failure prints
  `linear update failed` with the port-failure exit code.
- `flow_update.rs` covers every row of the plan's table, the three refusals
  (122 with and without a catalogue, 123 for an ambiguous name), and the
  failed `--state` update (exit 34, one request).
- The skill's Step 2 flags a cleared field and Step 4 no longer mentions a JSON
  `outcome`; refusals are named by keyword, not exit code.
- `CHANGELOG.md` `[Unreleased]` carries the `Breaking` and `Fixed` entries as
  worded in the plan.

#### Deviations from Plan:

- `update_op`'s rustdoc is trimmed to "`update`, surfacing the structured
  discriminant." (`14adeae0`), dropping the plan's restatement of the module
  doc and its list of callers.
- Two flow tests beyond the plan back the CHANGELOG's `Breaking` claims
  (`91d7fa12`): a `--state` update to a refused identifier sends nothing, and
  a failed `--state` update exits as a plain update would against a bare 401.
  An improvement.
- Phase 3 ran over the committed catalogue rather than a refreshed one, so
  that no tracked file changed. Recorded in the plan.

#### Potential Issues:

- Linear refuses `--title ""` and `--assignee-id ""` with exit 34 and the
  generic `linear update failed`, and an out-of-range `--priority` will fail
  the same way. Accepted as out of scope in the work item.
- The 123 ambiguous-state refusal is proven offline only: the live catalogue
  has no ambiguous active state name.

### Manual Testing Required:

1. Skill, end to end against a scratch Linear issue:
   - [ ] `/update-linear-issue <KEY> --title "…"` previews and confirms a
         title-only change, then `/show-linear-issue <KEY>` shows the
         description survived
   - [ ] `/update-linear-issue <KEY> --description ""` previews the
         description as cleared

2. Done here:
   - [x] `UpdateFields`'s field names match the domain, and `update.rs`
         carries only its module doc, the `state_id` field doc and
         `update_op`'s rustdoc
   - [x] The skill's text no longer mentions a JSON `outcome`

### Recommendations:

- Re-check `GHSA-ch52-4w7c-c8xp`, `RUSTSEC-2026-0118` and `RUSTSEC-2026-0119`
  upstream and remove or re-date each ignore, in a change separate from this
  branch, so that bare `mise run` exits 0 again.
- Run the two skill checks above before merging, then mark the plan done.
