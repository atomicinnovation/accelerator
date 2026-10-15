---
type: "plan-validation"
id: "2026-10-10-0296-sync-round-trip-defects-validation"
title: "Validation Report: Sync Round-Trip Defects Implementation Plan"
date: "2026-10-15T08:29:53+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
parent: "plan:2026-10-10-0296-sync-round-trip-defects"
target: "plan:2026-10-10-0296-sync-round-trip-defects"
tags: ["sync", "linear", "jira", "baseline", "equivalence", "adf"]
last_updated: "2026-10-15T08:29:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Sync Round-Trip Defects Implementation Plan

Validated at revision `cb47e2f7` (PR #159, branch
`0296-fix-sync-round-trip-defects`) against work item 0296 and the plan's
"Amendments to 0296" section.

### Implementation Status

✓ Phase 0: Fixture Capture - Fully implemented
✓ Phase 1: Projected Title Split - Implemented; one regression test deferred to 0300
✓ Phase 2: `adf` Crate Extraction - Fully implemented
⚠️ Phase 3: ADF Converter Extension - Implemented; table-cell hard break renders a broken table (see issues)
✓ Phase 4: Jira Pulls Render Markdown - Fully implemented
✓ Phase 5: Minimal Rich Body Model - Fully implemented
✓ Phase 6: Body Equivalence - Fully implemented
✓ Phase 7: Digest Recipe 1 - Fully implemented
⚠️ Phase 8: Equivalence-Aware Classification - Implemented; four planned tests not written
✓ Phase 9: Dossier Presentation - Fully implemented
✓ Phase 10: Promotion Adopts Equivalence - Fully implemented
✓ Phase 11: Merge-Time Verification - Preview recorded in `meta/prs/159-description.md` with the 32-item table

### Automated Verification Results

✓ `mise run` exits 0 (2026-10-15, 1,499 s; working copy unchanged afterwards)
✓ `test:unit:cli`: 5,660 run, 5,660 passed, 2 skipped
✓ All 47 test functions named in the plan's traceability table exist
✗ Not written, as the plan records: `a_pulled_item_syncs_again_as_synced_with_no_update` (deferred to 0300), `a_create_whose_read_back_failed_syncs_as_synced_and_gains_a_remote_hash`, `jira_remote_paragraph_text_differences_are_not_equivalent`, `a_baselined_jira_item_with_a_rejected_local_body_and_no_edits_is_synced`, `an_unbaselined_jira_item_with_a_rejected_local_body_is_an_unresolved_conflict`

Not run in this validation: `test:integration:pup`, `test:integration:deny`,
`test:integration:tracker-contract` (credentialed) and the manual sandbox
checks; the plan records each as passed.

### Code Review Findings

#### Matches Plan:

- Recovery table: all six rows behave as specified through `recover` in
  `cli/work/src/sync/classify.rs`; no entry is created for a not-equivalent
  missing baseline.
- `--preview` returns before `finalise_baseline`, so `last-sync.json` is never
  written; preview and apply share `prepare_run`.
- Legacy entries classify under the legacy recipe and are rewritten under
  recipe 1 when settled; remote-absent, indeterminate, dirty and
  direction-filtered entries keep the legacy recipe.
- A stamp-only change classifies `Synced` with hashes untouched.
- Every pull path writes the stripped, rendered description
  (`run.rs:1064`, `apply.rs:376-382`); `split_projected`
  (`cli/remote-projection/src/lib.rs:135-143`) strips by position with tests
  for an empty description and a first line equal to the title.
- Equivalence in `cli/corpus/src/body/canonical.rs` covers every listed
  normalisation; fenced code, ordered start numbers and table alignment are
  compared.
- `pulldown-cmark` is pinned `=0.13.4`, `default-features = false`, used only
  by `document`, absent from public-API pins; `golden_digests.rs` ties the
  `Cargo.lock` version to the recipe history.
- Dossiers omit equivalent sections, carry no title line, and render
  `title: <local> → <remote>` only on a difference; `sync-work-items` documents
  it.
- Promotion's `agrees_but_for_ids` compares titles exactly and bodies by
  equivalence.

#### Deviations from Plan:

- 0296 has not been amended per the plan's "Amendments to 0296" section:
  the synced-fields update rule, `local_fields_hash`/`remote_fields_hash`,
  `work diff` adopting section equivalence, promotion equivalence, the
  unrenderable-ADF refusal and 0335's precondition are absent; Required
  behaviour still carries the superseded "sends an update only when the local
  side changed and the local title differs" wording.
- `keeps_local_hash` (`cli/work-adapters/src/sync/run.rs:608-621`) keeps
  `local_hash` on a legacy conflict over unrenderable Jira ADF. The plan
  approves this; the amended final criterion says an unresolved conflict
  always blanks it. One document needs reconciling.
- Recipe-1 entries still store a legacy `remote_hash` over the full projected
  body, title line included (`baseline_digester.rs:62`). This is the plan's
  dual-recipe design pending 0335, but contradicts 0296's description of the
  new-recipe `remote_hash` and "each digest strips the projected title line".
- Jira rejected-body and paragraph-difference criteria rest on golden-digest
  pairs and `body_comparison.rs` unit tests rather than the run-level tests
  the traceability table names; the `sync_equivalence.rs` scenarios the plan
  cites as cover contain only one rejected-body case
  (`a_previewed_retitle_validates_only_the_title`).

#### Potential Issues:

- A Jira table-cell `hardBreak` renders as `"  \n"`
  (`cli/adf/src/render.rs:328-331`, reached from `cell()` at `:287-292`),
  splitting the row. Strict render does not refuse it, so a pull writes a
  broken table and a later push sends it back. No fixture covers it; this
  breaks the crate's own read-back promise.
- Code text containing a backtick is wrapped in a single backtick pair, and a
  code block containing a ```` ``` ```` line, or an empty one, is fenced with
  a fixed three-backtick fence (`render.rs:189-199`, `:410-411`). Both pass
  strict render and may not read back identically. Not reproduced.
- `local_title_and_body` (`run.rs:393-400`) defaults a failed frontmatter
  split to an empty title and body. An unbaselined item with malformed
  frontmatter classifies as a conflict; `--resolve id=local` then sends
  `title: ""` and `body: ""`, which `send_refusal` does not stop. Predates the
  branch, but the recovery path makes it more reachable.
- `reconstruct_pulled_content` (`run.rs:402-410`) has the same default and
  writes `---\n\n---` over unparseable frontmatter; `ItemApplier::pull` writes
  before digesting, so a digest failure leaves the file written with no
  baseline. Predates the branch.
- One unreadable item fails the whole run (`plan.rs:120-125` →
  `RunError::Internal`) rather than that item.
- A planned pull over unrenderable Jira ADF previews as not applied but fails
  on apply (`run.rs:770-804` validates sends only).
- A remote-only retitle is absorbed into the baseline without being reported;
  accepted until 0290 per 0296's out-of-scope note.
- 0184's pending pull would write Linear's damaged code-span table cells
  locally; a full bidirectional sync waits on its remote repair (0320).

### Manual Testing Required:

1. Jira:
  - [ ] Pull an issue whose table cell holds a line break; confirm the table
        survives (expected to fail today)
  - [ ] Pull an issue whose code span contains a backtick
2. Corpus:
  - [ ] Re-run `work sync --preview` after rebasing on `main` (branch is at
        `1.24.0-pre.81`, `main` at `pre.82`); confirm `last-sync.json` stays
        byte-identical and only expected items differ

### Recommendations:

- Fix the table-cell hard break (render as `<br>`-free text or refuse in
  strict mode) with a failing fixture first; add backtick-in-code fixtures.
- Make `local_title_and_body` and `reconstruct_pulled_content` fail the item
  rather than defaulting, or have `send_refusal` refuse an empty payload.
- Apply the plan's amendments to 0296, and reconcile the legacy-unrenderable
  `local_hash` retention between plan and work item.
- Write the three Jira rejected-body/paragraph run-level tests and the failed
  read-back test, or strike them from 0296's criteria explicitly.
- Track `a_pulled_item_syncs_again_as_synced_with_no_update` against 0300.
