---
type: "plan-validation"
id: "2026-10-10-0296-sync-round-trip-defects-validation"
title: "Validation Report: Sync Round-Trip Defects Implementation Plan"
date: "2026-10-15T08:29:53+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-10-10-0296-sync-round-trip-defects"
target: "plan:2026-10-10-0296-sync-round-trip-defects"
tags: ["sync", "linear", "jira", "baseline", "equivalence", "adf"]
last_updated: "2026-10-15T10:14:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Sync Round-Trip Defects Implementation Plan

First validated at `cb47e2f7` with result `partial`; revalidated after the
fixes listed under Resolved Findings. PR #159, branch
`0296-fix-sync-round-trip-defects`, against work item 0296 as amended.

### Implementation Status

✓ Phase 0: Fixture Capture - Fully implemented
✓ Phase 1: Projected Title Split - Fully implemented
✓ Phase 2: `adf` Crate Extraction - Fully implemented
✓ Phase 3: ADF Converter Extension - Fully implemented; strict render now refuses the three bodies that did not read back
✓ Phase 4: Jira Pulls Render Markdown - Fully implemented
✓ Phase 5: Minimal Rich Body Model - Fully implemented
✓ Phase 6: Body Equivalence - Fully implemented
✓ Phase 7: Digest Recipe 1 - Fully implemented
✓ Phase 8: Equivalence-Aware Classification - Fully implemented; previously unwritten tests now written
✓ Phase 9: Dossier Presentation - Fully implemented
✓ Phase 10: Promotion Adopts Equivalence - Fully implemented
✓ Phase 11: Merge-Time Verification - Preview recorded in `meta/prs/159-description.md`

### Automated Verification Results

✓ `test:unit:cli`: 5,675 run, 5,675 passed, 2 skipped
✓ `test:unit:frontend`: 2,615 passed
✓ `lint:cli:check`, `public-api:check` (`tracker` pin updated for `BodyDialect::rejects`) and every other `mise run` task
✓ `mise run test:e2e:visualiser` alone: 355 passed
⚠️ `mise run`: exit 0 on the run after the first fixes; the last two runs failed only `kanban-drag-overlay.spec.ts` ("Escape-cancelling a drag clears the SSE gate…"), a visualiser flake under full-suite load that the plan records in Phases 2, 3 and 10. No change on this branch since validation touches the visualiser.
✓ All acceptance criteria in the plan's traceability table name an existing test

Not run in this validation: `test:integration:pup`, `test:integration:deny`,
`test:integration:tracker-contract` (credentialed) and a fresh
`work sync --preview` against Linear.

### Code Review Findings

#### Matches Plan:

- Recovery table rows, `--preview` never writing `last-sync.json`, legacy
  entries migrating to recipe 1 as they settle, and stamp-only changes leaving
  hashes alone.
- No pull path writes the projected title line; `split_projected` strips by
  position.
- Equivalence covers every listed normalisation; `pulldown-cmark` is pinned
  `=0.13.4` and tied to the recipe history.
- Dossiers list only differing sections and show `title: <local> → <remote>`.
- Promotion compares titles exactly and bodies by equivalence.

#### Resolved Findings:

- Strict ADF render refuses a table-cell hard break, code text holding a
  backtick, and a code block holding a fence line, so a pull can no longer
  write a body that reads back differently.
- An item whose local frontmatter does not parse now fails: `=local` sends
  nothing (it sent `Update { title: "", body: "" }`) and `=remote` leaves the
  file untouched; its dossier reads `reason: local-unreadable`.
- A local body Jira's converter rejects is never equivalent, through the new
  `BodyDialect::rejects`.
- `--preview` fails a planned pull the run would fail before writing.
- Previously unwritten tests added: Jira paragraph difference, rejected-body
  sync with and without a baseline, failed `create --push` read-back, and
  pull-then-sync for both trackers.
- 0296 now carries the plan's amendments, the recipe-1 entry keys, the
  retained legacy hashes until 0335, and the legacy-unrenderable exception to
  blanking a conflict's local hash.

#### Deviations from Plan:

- The end-to-end `work-cli` variant of
  `a_pulled_item_syncs_again_as_synced_with_no_update` still waits on 0300;
  the `work-adapters` variant covers the criterion for both trackers.

#### Potential Issues:

- One item whose digest cannot be computed aborts the whole run
  (`plan` → `RunError::Internal`). Unchanged from trunk, by design.
- A remote-only retitle is absorbed into the baseline unreported; accepted
  until 0290.
- 0184's pending pull would write Linear's damaged code-span cells locally;
  waits on 0320.

### Manual Testing Required:

1. Corpus:
  - [ ] Rebase on `main` and re-run `work sync --preview`; confirm
        `last-sync.json` stays byte-identical and only 0184 differs
2. Jira:
  - [ ] Pull an issue whose table cell holds a line break; confirm it fails
        as `unrenderable` and writes nothing

### Recommendations:

- Stabilise or quarantine `kanban-drag-overlay.spec.ts`'s Escape-cancel test;
  it fails `mise run` under load independently of this work.
- Write the `work-cli` pull-then-sync test once 0300 lands.
