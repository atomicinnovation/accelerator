---
type: "plan-validation"
id: "2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field-validation"
title: "Validation Report: Work Item Writers Omit the Required Tags Field Implementation Plan"
date: "2026-11-19T09:24:58+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field"
tags: ["work", "work-cli", "migrate", "frontmatter"]
last_updated: "2026-11-19T09:24:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Work Item Writers Omit the Required Tags Field

### Implementation Status

✓ Phase 1: Writers Always Emit `tags` - Fully implemented
✓ Phase 2: Migration 0012 Backfills `tags` - Fully implemented

Every success-criterion checkbox in the plan is ticked, and the code
bears each one out.

### Automated Verification Results

✓ `mise run` (bare default task): every task except `test:unit:cli`
  completed green on the first run, including `check`, `pup:check`,
  `public-api:check`, `deny:check`, `docs:build`, every lint, the Python
  suites, and the visualiser unit, integration and e2e suites.
✗ → ✓ `test:unit:cli` failed on the first run, but neither failure came
  from this change:
  - `design-adapters::spawn_properties::the_child_s_streams_are_redirected_to_the_bootstrap_log`
    failed after 3,686 of 7,808 tests and cancelled the rest. It is a
    timing flake: `await_content` returns as soon as the log has any
    content, before stderr has reached it. It failed 1 of 10 isolated
    runs. This branch does not touch `design-adapters`.
  - A second run failed five `rekey` tests with `No space left on device`.
    The disk had 781 MiB free; `cli/target` alone is 65 GiB.
✓ `mise run test:unit:cli` re-run once space had freed: 7,808 run,
  7,808 passed, 2 skipped.
✓ The `accelerator-migrate` suite: 204 run, 204 passed. Run against
  `llvm-cov-target` without `LLVM_PROFILE_FILE` set, the instrumented
  binary drops a `.profraw` file into the test's tree, and
  `migration_0004`'s `read_tree` then fails reading it as UTF-8. That came
  from how I invoked the tests; the code is not at fault.
✓ Targeted crates (`work`, `accelerator-work`, `migrate`,
  `migrate-adapters`, `work-adapters`, `corpus`, `corpus-adapters`): all
  passed.

The full default task was not re-run end to end in one go after the
flake. Its green status is established from the first run's other tasks
plus the clean re-run of `test:unit:cli`.

### Code Review Findings

#### Matches Plan:

- `compose_frontmatter` (`cli/work/src/create.rs:148-151`) pushes `tags`
  unconditionally, directly before `last_updated`. The doc comment is cut
  down to the one non-obvious fact.
- `writes_empty_tags_and_omits_empty_linkage` and
  `writes_every_field_the_schema_requires_when_all_else_is_empty` are as
  specified. The latter reads `BASE_FIELDS` and the work-item row's
  `required_extras`, so it will catch a required field added there later.
- The CLI (`cli_create.rs`), batch (`create_batch.rs`) and sync
  (`sync_author.rs`) tests all pin adjacency, omit-when-empty absence,
  and validation via `validate_text`. The helpers live in both
  `tests/common/mod.rs` and `src/test_support.rs` as planned, and the
  former's `#![allow(dead_code)]` covers the twelve other test binaries
  that include it.
- Both SKILL.md passages and both Quality Guidelines bullets are
  reworded as specified, within 80 columns. Benchmarks for the reworded
  evals are re-recorded.
- `TextFile`, the port's `read_text` (failing loudly by default) and the
  adapter's `read_bytes`/`read_text` split match the plan. The three
  adapter tests are present.
- `m0012.rs` follows the plan's design closely: byte-offset insertion via
  `split_inclusive`, presence and type taken from live `corpus` parsing,
  the YAML guard (`yaml_obstacle`, `is_original_plus_empty_tags`),
  record-before-write, the three-way restore, the stage-naming error
  message, and the stderr summary and untouched note.
- All 16 transform tests, all 27 `apply` tests and all 14 end-to-end
  tests named in the plan's tables exist.
- Registration, the public-API snapshot, every hard-coded ledger in the
  `migrate-cli` tests, the realignment doc rewording across `ports.rs`,
  `sync_baselines.rs`, `realign.rs` and `sync_realign.rs`, and both
  CHANGELOG entries are in place.
- Every one of the 390 work items in this repository carries a `tags` key.

#### Deviations from Plan:

- The `TextFile` doc comment the plan specified was dropped in the
  comment-trimming commit, in line with the repository's comment policy
  (improvement).
- `writes_every_field_the_schema_requires_…` uses
  `let … else { unreachable!(…) }` instead of `.expect(…)`, presumably to
  satisfy the crate's clippy settings (neutral).
- Three items found during implementation are recorded in the plan
  itself: the eval re-runs (Phase 1 §9), the `--lib` test target needed
  while the transform was dead code, and two more fields on the m0012
  test double (`failing_listing`, `writes`).

#### Potential Issues:

- None in the delivered code. The edge cases I checked (CRLF, a missing
  final newline, `last_updated_by` as a near-miss key, indented
  look-alike keys inside block scalars, quoted keys, an empty
  frontmatter, an unterminated fence, a byte-order mark) are each either
  tested or handled consistently with the validator. A failed
  realignment that advanced some baselines before failing converges on
  re-run, because `realigned_entry` only moves entries that still record
  the original digest.
- Two test-harness fragilities outside this change surfaced while
  validating:
  - the flaky `design-adapters` stream-redirection test;
  - `migration_0004`'s `read_tree` assumes every file in the tree is
    UTF-8.

### Manual Testing Required:

The plan records all four Phase 2 manual checks and both Phase 1 manual
checks as done, with notes on how each ran. Nothing more is needed
before merge. If re-confirming:

1. Writer:
  - [ ] `accelerator work create "Example" story medium` in a scratch
        repository writes `tags: []` immediately before `last_updated`,
        and `accelerator corpus frontmatter validate --file <path>`
        exits 0.
2. Migration:
  - [ ] Remove that line, run
        `accelerator migrate --unapply 0012-backfill-work-item-tags`, then
        `/accelerator:migrate`. The line comes back with no other diff.
  - [ ] A second `/accelerator:migrate` reports no pending migrations.

### Recommendations:

- Move work item 0333 from `ready` to `done` (or the next status in your
  workflow); it still reads `ready`.
- Raise a work item for the flaky
  `the_child_s_streams_are_redirected_to_the_bootstrap_log`. It should
  wait until both markers are present rather than for the first content.
- Optionally, make `migration_0004`'s `read_tree` compare bytes, so it
  doesn't depend on every file in the tree being UTF-8.
- Free disk space before the next full `mise run`. `cli/target` is
  65 GiB.
