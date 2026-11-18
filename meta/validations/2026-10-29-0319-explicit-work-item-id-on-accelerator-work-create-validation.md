---
type: "plan-validation"
id: "2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create-validation"
title: "Validation Report: Explicit Work Item ID on accelerator work create Implementation Plan"
date: "2026-11-02T17:52:48+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create"
tags: ["work", "cli", "allocation", "create-work-item", "corpus", "visualiser"]
last_updated: "2026-11-02T17:52:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Explicit Work Item ID on accelerator work create Implementation Plan

All eleven phases are implemented as planned, and every test the plan and
its traceability table name exists (88 of 88). `mise run` exits 0 end to
end.

A first validation pass at `40d92de1` came back `partial`. It found a
RustSec advisory that failed `deny:check`, a wording bug in
`E_PATTERN_OVERFLOW`, and a handful of smaller gaps. The fixes listed under
"Fixed during validation" resolve all of them.

### Implementation Status

✓ Phase 1: Tracker-key grammar in `tracker` - Fully implemented
✓ Phase 2: A general identity port in `corpus` - Fully implemented
✓ Phase 3: One ID-pattern grammar in `work` - Fully implemented
✓ Phase 4: The ID scheme moves into `work` - Fully implemented
✓ Phase 5: The last consumers move, and `corpus` loses work-item identity - Fully implemented
✓ Phase 6: `RequestedId` domain model - Fully implemented
✓ Phase 7: `work create --id` - Fully implemented
✓ Phase 8: `--id` checks on `work create --dry-run` - Fully implemented
✓ Phase 9: The `work id` verb group - Fully implemented
✓ Phase 10: `work id check` - Fully implemented
✓ Phase 11: `/create-work-item --id` - Fully implemented

### Automated Verification Results

✓ `mise run` (default): exit 0. It covers build, `format:fix` (no further
  changes), `lint:check`, `types:check`, `deny:check` (advisories, bans,
  licences and sources all ok), `pup:check`, `public-api:check`,
  `notices:check`, `docs:check` and `test`.
✓ `cargo nextest run -p work next_number`: 18 of 18 passed.
✓ `cargo nextest run -p corpus -p accelerator-visualiser`: 746 of 746 passed.

The per-phase `cargo nextest`/`cargo test` filters in the plan were not
re-run one by one. The `test` lane covers all of them.

### Code Review Findings

#### Matches Plan:

- `corpus` and `corpus-adapters` hold no work-item identity. The scanner,
  pattern compiler and canonicaliser are deleted, `corpus-adapters` drops
  `regex`, and `tasks/lint/corpus_identity.py` guards the boundary.
- `work::id_pattern` is the one grammar: 255-byte bound, width cap 19,
  restricted literal alphabet, `escaped` helper. `WorkItemIdScheme`
  implements `corpus::IdConvention`, and `LegacyNumericIds` implements
  `corpus::LinkageIds`.
- `work_adapters::config` provides `read_scheme`, `require_key` and
  `resolve_scheme`, all reporting through `SchemeError`. The taken rule
  goes through `holder_of_under`/`linker_of_under`, used only by retirement
  and re-key.
- Re-key reads IDs written with literal pattern text. Run-plan targets are
  read through `read_exact`, and retirement refuses an ID that is not one
  path component. `migrate::ports::WorkItemIdentity` is wired in
  `migrate-cli`.
- Refusal order for `work create --id`:
  - The order is `locate_work_items`, then checks 1–4, then
    `create_dir_all`, then the `Requested` arm under the create lock: the
    pending-push check (only with `--push`), then `refuse_if_taken`.
  - No write or tracker request happens before any refusal.
- With `--id`, `--dry-run` runs the real create's checks in the real
  create's order and reads `integration_state` once. A plain dry-run is
  untouched.
- `work id {canonicalise,next,check}` is in place. The retired flat verbs
  exit 2 with a pointer, and no old verb name remains in the shipped skills,
  agents, templates, hooks or docs.
- `/create-work-item`:
  - It settles `--id` through `work id check` and confirms a correction.
  - It passes `--id` on the push-gate dry-run and the real create.
  - It sends `E_ID_*` refusals to recovery instead of saving locally.
- Evals 39–72 pass with the skill (with-skill mean 0.9956). The amendment to
  work item 0319 matches the plan.

#### Fixed during validation:

- **`pulldown-cmark` 0.13.6** for RUSTSEC-2026-0433:
  - This repeats trunk's `rtmpnvtpsywn` change hunk for hunk: the pin, the
    lock, 0.13.6 recorded against digest recipe 1, and the notices.
  - The upstream diff changes only the wikilink closing path. Neither
    `markdown` nor `document` enables wikilinks.
- **`E_PATTERN_OVERFLOW`** uses the at-the-cap wording only when an item
  holds the cap (`cli/work/src/next_number.rs`):
  - A count that crosses the cap from below, or an empty directory, gets the
    neutral `number space exhausted (highest=…, cap=…)` form again.
  - Two new tests pin this and were red before the fix.
  - The changelog entry is narrowed to match.
- **`.MD` work items** now get a slug and an extension-free title:
  - `corpus::slug` strips the markdown extension ignoring case.
  - The visualiser's `build_entry` does the same.
  - `work_items_keep_their_slug_under_an_upper_case_extension` and the
    extended `the_visualiser_indexes_an_upper_case_md_item` were red first.
- **Changelog** now records that `--project ""` gives `E_WORK_KEY_INVALID`
  where it gave `E_PATTERN_MISSING_KEY`.
- **Comments:**
  - The stale comment in `clusters.rs` is removed.
  - The `corpus/Cargo.toml` comment no longer mentions a scan regex.
  - The restating doc comment on `id::retired` is removed.
- **Line width:** the over-width lines are rewrapped in
  `cli/tracker/tests/structure.rs`, `create-work-item/SKILL.md` and the 0319
  work item.

#### Deviations from Plan:

- `migrate-cli` depends on `work-adapters` as well as `work`, so it can
  render `SchemeError::Pattern`. This is acceptable for a composition root.
- The Phase 2 scanner doubles collapse into one shared `FourDigitIds` (an
  improvement). Phase 3's parity test compares the empty key's error text
  only.
- `work id check` reads config errors before the re-key guard, the same
  order as `work id next`. The plan as built notes this.

#### Potential Issues:

- The `identity_key` fallback through `keyed_digits` always joins with `-`
  (`cli/work/src/id_scheme.rs:292-294`). Under `{key}_{number:04d}`, a
  reference written `PP-42` therefore joins as `PP-0042`. This is outside
  the plan's specified rows.
- Under `{tracker}` without `--id`, `E_PATTERN_KEY_UNUSED` still follows
  `create_dir_all`. That is the old behaviour and outside the `--id` promise.
- `--dry-run --id` without `--push` fails when `paths.integrations` cannot
  be read, while a plain dry-run succeeds. The plan intends this.
- The branch trails trunk by 71 changes. Trunk's identical 0.13.6 bump
  merges cleanly.

### Manual Testing Required:

The plan's manual checks are all ticked, with run notes recorded. These were
not re-run here:

1. `/create-work-item --id`:
  - [ ] `/create-work-item --id 400 <topic>` in a scratch repository
        proposes `0400` and writes `0400-<slug>.md` after confirmation.
  - [ ] With an integration configured and `0400` taken, the push gate
        relays `E_ID_TAKEN` and offers another ID or automatic allocation.
2. Overflow wording:
  - [ ] With `9997-x.md` held, `accelerator work id next --count 3` prints
        `9998` and `9999`, then the neutral `E_PATTERN_OVERFLOW`.

### Recommendations:

- Rebase onto trunk before merge, or merge trunk in, to pick up its 71
  changes. The duplicated `pulldown-cmark` bump folds away.
- Optionally:
  - Pin `work id check`'s config-error-before-re-key-guard order with a
    test.
  - Decide whether `identity_key`'s fallback should honour the pattern's
    separator.

Not checked: red-first ordering within the original commits, the bodies of
most tests, the frontend, and a blind re-grade of the eval runs. Eval 26's
with-skill failure (0.67) is recorded in `benchmark.md` and predates `--id`.
