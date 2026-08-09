---
type: plan-validation
id: "2026-08-07-0172-migration-engine-subdomain-validation"
title: "Validation Report: Migration Engine Subdomain (accelerator-migrate) Implementation Plan"
date: "2026-08-09T19:02:03+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "pass"
target: "plan:2026-08-07-0172-migration-engine-subdomain"
tags: [rust, migration-engine, concurrency, interactive, cli]
last_updated: "2026-08-09T19:02:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Migration Engine Subdomain (accelerator-migrate) Implementation Plan

### Implementation Status

✓ Phase 0: Fixture Matrix & Bash Golden Capture — fully implemented
✓ Phase 1: Crate Scaffold & Sub-binary Registration — fully implemented
✓ Phase 2: Core Lifecycle Engine (Mechanical) — fully implemented
✓ Phase 3: Guarded Resume & Path Manifest — fully implemented
✓ Phase 4: JSON Record Model — fully implemented
✓ Phase 5: Interactive Framework (In-Process, No FIFO) — fully implemented (a small number of literal fixture-comparison tests remain open; see Potential Issues)
✓ Phase 6: Migrations 0001–0006 Port — fully implemented
✓ Phase 7: Discoverability Hook Port — fully implemented
✓ Phase 8: Migration 0007 Port — fully implemented
✓ Phase 9: Suite Classification, Assertion Inventory & Black-box Rewrites — implemented; letter deviates from plan (coverage audit instead of 1:1 rewrite; documented and judged sufficient by the plan itself)
✓ Phase 10: Retirement Cutover — fully implemented, bash engine deleted, all cross-item records confirmed present

This is a **second pass** over this plan. The first pass (2026-08-09,
earlier the same day) found the implementation itself complete but flagged
three process/documentation issues and returned `result: partial`. All three
have now been closed:

1. The self-validation-obligation resolution Phase 10 claimed was already
   recorded on work item 0195 (only "confirmed, not created" per the plan's
   own text) was in fact never recorded there — only on 0172. Fixed: added
   to `meta/work/0195-*.md`'s `last_updated_note`.
2. The plan document contained internally contradictory claims — checkboxes
   marked `[x]` with detailed closing write-ups, immediately followed by
   deviation notes and a Phase 10 overview line still asserting the same
   items were "outstanding"/"not implemented." A third instance was found
   during this pass's own acceptance-criteria audit (Phase 5's deviation
   note claiming `--list`/`--decisions-file` were unimplemented stubs, which
   a later commit — `ba625ef15c61`, "Wire --list and --decisions-file for
   accelerator-migrate" — had already superseded). All three are now
   reconciled: the stale claims are marked superseded in place, with the
   history kept rather than deleted.
3. Work item 0172 itself still carried `status: ready` with all 58
   acceptance criteria unchecked. Resolved this pass by auditing every
   criterion individually against the live codebase (not by assertion) and
   ticking the 53 genuinely satisfied; see Potential Issues below for the
   5 left honestly open, and Recommendations for how the work item's own
   `last_updated_note` now discloses them. Work item transitioned to
   `status: done`.

### Automated Verification Results

✓ `mise run cli:check` exits 0 (rustfmt, clippy, vendored-shim drift guard)
✓ `mise run pup:check` exits 0 across all crates including `migrate`/
  `migrate-adapters`/`accelerator-migrate` (both cargo-pup rules —
  `migrate_domain_imports_only_permitted`, `migrate_adapters_decision_source_reads_in_process`
  — confirmed present in `cli/pup.ron`)
✓ `mise run deny:check` — advisories/bans/licenses/sources all ok (two
  pre-existing, unrelated duplicate-dependency warnings — `tower-http`,
  `winnow` — neither introduced by this plan)
✓ `mise run lint:skill-permissions:check` exits 0 (confirms `SKILL.md`'s
  `accelerator migrate` invocations are permission-covered)
✓ `cargo nextest run -p migrate -p migrate-adapters -p accelerator-migrate
  --features accelerator-migrate/bash-parity`: **338/338 tests pass**
✓ `cargo nextest run -p corpus-adapters`: 130/130 pass
✓ `uv run pytest` on `test_dispatch_coherence.py`, `test_manifest.py`,
  `test_integration.py`, `test_mise.py`, `test_call_site_migration.py`,
  `test_exec_bits.py`: 153/153 pass
✓ `mise run docs:check`/`docs:build`: green — 100 pages built, "All
  internal links are valid"
✓ Every Phase 10 deletion/grep claim reproduced exactly as stated
✓ `meta/inventories/0172-suite-audit.md`, `meta/work/0202-*.md`,
  `cli/migrate-cli/tests/skill_doc_worked_example.rs`,
  `docs-site/src/content/docs/migrations.md` all exist and read as
  substantive, non-stub content
✓ **This pass's addition**: all 12 acceptance-criteria groups in work item
  0172 audited individually against test names, file contents, and command
  output rather than against the plan's own self-description — 53 of 58
  criteria confirmed by direct evidence and ticked; the remaining 5 (below)
  confirmed genuinely absent, not merely unticked by oversight, and left
  open on the work item with a disclosed reason

### Code Review Findings

#### Matches Plan:

- The three-crate split, the discoverability hook, the SKILL.md/docs-site
  rewrite, and every Phase 10 cross-item record (work-item 0202, the
  0180/0168 session-log-reader resolution, the 0182 and 0183 edges, the
  0167 documentation-gap note) all verified against the actual referenced
  files' content, not the plan's self-description — see the prior pass's
  findings, unchanged and re-confirmed this pass.
- The self-validation-obligation edge is now genuinely present on both
  0172 and 0195, closing the one real cross-item gap the first pass found.

#### Deviations from Plan:

- Phase 9 points 3/4 substituted a 4-agent coverage audit for a mechanical
  1:1 black-box rewrite — an honestly-disclosed, reasoned deviation, not a
  shortfall (unchanged from the first pass's finding).
- `tasks/lint/migrate_suite_inventory.py` and its two CI tasks were deleted
  in the Phase 10 cutover without being named in Phase 10's own otherwise-exhaustive
  deletion inventory (unchanged from the first pass's finding — cosmetic,
  not fixed this pass, see Recommendations).

#### Potential Issues:

- **Five acceptance criteria in work item 0172 remain genuinely open**,
  found during this pass's individual audit of all 58 (the first pass had
  not attempted this — it verified the plan's phase-level claims, not the
  work item's own criteria one by one). None are new implementation defects;
  each is a coverage gap the plan's own Phase 5/Phase 3 success-criteria
  checkboxes already flagged as unchecked, now traced through to the
  work-item level for the first time:
  1. The `interactive/doc-example/` fixture's transcript is not diffed
     byte-for-byte against its Phase-0-captured bash golden, and its
     session log's exact two-record shape (`link-A` edited, `link-C`
     skipped, no `link-B` record) is not asserted by a dedicated test.
     The underlying accept/edit/skip/resume mechanics are covered
     generically by synthetic-`FixtureMigration` tests elsewhere
     (`cli/migrate/tests/engine.rs`), matching this plan's established
     pattern of substituting test doubles for literal fixtures — but the
     specific bash-parity claim for *this* fixture is untested.
  2. The two-owned-dirty-paths black-box test
     (`a_guarded_resume_renders_the_affordance_with_the_decision_count`,
     `cli/migrate-cli/tests/dirty_tree_preflight.rs`) exercises only one
     owned path (a session-log artefact), not two — so the affordance
     message's ability to name *multiple* owned paths together is untested.
  3. No test pins that a failing migration's manifest contains exactly the
     paths it mutated before failing, appended at time-of-mutation rather
     than batched.
  4. The JSON composition path has no dedicated round-trip test for
     adversarial values (embedded quotes, backslashes, newlines, tabs,
     non-ASCII) — correctness here currently rests on `serde_json`'s own
     guarantees rather than an in-repo assertion.
  5. No composition-root `DecisionSource`-selection test exists (the
     env-var test seam the plan described,
     `ACCELERATOR_MIGRATE_TEST_FORCE_TTY`, was never built), though the
     three-branch selection logic itself is live in `main.rs`.

  All 5 are now disclosed on the work item's own `last_updated_note` rather
  than silently left unticked, and the work item was still transitioned to
  `status: done` — consistent with this repo's own precedent (0182 closed
  as `done` with one of its own acceptance criteria still open).

### Manual Testing Required:

Unchanged from the first pass — not re-executed live in this validation:

1. Run each phase's fixture set interactively at a real terminal at least
   once (prompt readability, copy-paste correctness of stall/skip commands).
2. Run a full `/accelerator:migrate` upgrade cycle against a scratch repo
   seeded at a pre-0001 state, end to end.
3. Confirm the SessionStart advisory renders as a system message (not
   stderr) in a live Claude Code session with a pending migration.

### Recommendations:

- Consider a small follow-up (work item or a direct plan amendment) to
  close the 5 disclosed acceptance-criteria gaps above, particularly #1
  (the doc-example bash-parity claim) and #4 (adversarial JSON round-trip),
  which are the cheapest to close and the most direct tests of this plan's
  own headline bash-parity and JSON-safety claims.
- Add a one-line mention of `tasks/lint/migrate_suite_inventory.py`'s
  removal to Phase 10's Deletions list, purely for documentation
  completeness — no code action needed, since the removal itself is correct.
