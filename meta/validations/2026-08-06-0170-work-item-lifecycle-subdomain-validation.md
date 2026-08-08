---
type: plan-validation
id: "2026-08-06-0170-work-item-lifecycle-subdomain-validation"
title: "Validation Report: Work-Item Lifecycle Subdomain Implementation Plan"
date: "2026-08-07T23:26:37+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-08-06-0170-work-item-lifecycle-subdomain"
tags: []
last_updated: "2026-08-07T23:26:37+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Work-Item Lifecycle Subdomain Implementation Plan

### Implementation Status

All 9 phases are implemented and committed (13 commits, `5494d465de06` through
`9f9fd88d7e76`, plus two later unrelated commits at HEAD):

✓ Phase 1: Capture Characterization Fixtures — fully implemented. All 8
  golden fixture files present under `skills/work/scripts/test-fixtures/`,
  including the richer per-case-directory format for `section-diff`,
  `project-remote`, and `normalise`.
✓ Phase 2: Complete the Pattern-DSL Port — fully implemented
  (`compile_format_string`, `pattern_max_number`, `parse_full_id`,
  `canonicalise_id`, legacy-ID helpers on `WorkItemIdScheme`).
✓ Phase 3: Core `work` Domain Logic — fully implemented (`resolve`,
  `next_number`, `section_diff`, `tags`, `normalise`, `template_hints`,
  `own_identity`, `show::read_field_raw`), all regex-free per the
  `work_domain_imports_only_permitted` pup rule.
✓ Phase 4: `work resolve` + sub-binary registration — fully implemented;
  `accelerator-work` registered in `DISPATCHED_SUBBINARIES`, manifest, CI
  release binaries, `.gitignore`, and the dispatch-coherence guard (bound
  via the `work template-hints` static calls, pulled forward from Phase 8
  as the plan's own implementation-time note describes).
✓ Phase 5: `work show` — fully implemented.
✓ Phase 6: `work diff` — fully implemented, including the `diff`-shellout
  adapter, capped-wait timeout handling, and `project_remote` projection
  (relocated to `work-adapters` as documented).
✓ Phase 7: `work create` — fully implemented, including the mkdir-lock
  collision guard, template-schema drift guard, and `NNNN`-placeholder
  substitution contract.
✓ Phase 8: `work update`, `template-hints`, `canonicalise-id`,
  `next-number` — fully implemented, including list-field validation
  (`ScalarSetOnListField`), the lost-update lock guard, and the full
  8-command CLI-surface snapshot golden.
✓ Phase 9: Skill Repoint and Shell Deletion — fully implemented; all
  documented implementation-time corrections (migration script's
  self-contained format compiler, `list-work-items` post-filter,
  `corpus-adapters` test rewiring, schema-conformance CLI-delegation
  recognition, review-1 frontmatter fix) verified present.

### Automated Verification Results

✓ `mise run cli:check` (rustfmt, clippy, vendored-shim drift) passes
✓ `mise run pup:check` (cargo-pup architecture rules) passes — confirms
  `work` imports only `std`/`kernel::Error`/`corpus`/`crate`, and
  `work_adapters::filesystem` never imports `std::process`
✓ `cargo test -p work -p work-adapters -p corpus -p corpus-adapters --locked`
  — 250 passed, 0 failed
✓ `cargo test -p accelerator-work --locked` — 61 passed, 0 failed
✓ `cargo test -p accelerator-work -p work-adapters --locked --features
  bash-parity` — 78 passed, 0 failed (real `diff` binary parity suite)
✓ `mise run lint:dispatch-coherence:check` passes — `work` token
  correctly bound, not exempted
✓ `mise run lint:skill-permissions:check` passes
✓ `mise run lint:scripts:check` (shellcheck, exec-bits, bashisms) passes
✓ `mise run test:integration:work` passes — 108 + 6 + 21 assertions
  observed across suites, `_EXPECTED_WORK_SUITES` confirmed at the
  decremented floor of 5 (`tasks/test/integration.py:49`)
✓ Script deletion set matches the plan exactly: the 8 named scripts plus
  `work-item-common.sh` and `test-work-item-pattern.sh` are gone; the 3
  deferred scripts (`work-item-file-dirty.sh`, `work-item-project-remote.sh`,
  `work-item-normalise.sh`) remain, as designed
✓ `grep -rn "work-item-\(common\|next-number\|pattern\|read-field\|read-status\|resolve-id\|section-diff\|update-tags\|template-field-hints\)\.sh" skills/`
  — zero matches in live skill logic; remaining matches are inert
  historical text in `evals/benchmark.json` fixtures and `test-fixtures/*.golden`
  header comments (see Potential Issues)
✓ CLI-surface snapshot golden (`work-cli/tests/fixtures/cli_surface.golden`)
  lists all 8 subcommands (`resolve`, `template-hints`, `show`, `diff`,
  `create`, `update`, `canonicalise-id`, `next-number`) with real `--help`
  text, confirming the frozen-CLI-contract claim in Desired End State
✓ `CHANGELOG.md` carries the documented `## [Unreleased]` → `### Added`
  entry for the new command family
✗ `mise run` (bare, full local CI mirror) — fails at HEAD, but **not**
  because of this plan: the failure is
  `test_bootstrap_exports_the_one_plugin_root_the_launcher_reads`
  (`tests/unit/tasks/test_bootstrap_coverage.py`), caused by the
  **later, unrelated** commit `98c17ab3f233` ("Consolidate
  ACCELERATOR_PLUGIN_ROOT reads into config-adapters"), which touches only
  `launcher/src/main.rs`, `launcher/.../cache_root.rs`, and
  `visualiser/server/src/main.rs` — none of which are part of 0170's
  scope. Every 0170-scoped check above (`cli:check`, `pup:check`, all
  `work`/`work-adapters`/`accelerator-work` test suites,
  `test:integration:work`, dispatch-coherence, skill-permissions,
  scripts lint) is green in isolation.

### Code Review Findings

#### Matches Plan:

- All 5 user-facing commands (`create`, `show`, `resolve`, `diff`,
  `update`) and all 3 utility subcommands (`template-hints`,
  `canonicalise-id`, `next-number`) implemented exactly as specified,
  including the declared behavioural quirks (naive comma-split tag
  re-parse, un-stripped `IGNORE_KEYS` in `diff`'s frontmatter section,
  hash-free section comparison).
- Skill repoints verified file-by-file (`update-work-item`,
  `list-work-items`, `create-work-item`, `extract-work-items`,
  `refine-work-item`, `review-work-item`, `sync-work-items`) — each calls
  the correct `accelerator work <verb>` invocations, none reference a
  deleted script, and each carries the
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)` `allowed-tools`
  grant except one noted deviation below.
- `sync-work-items`' exact two-repoint/three-deferred split (diff → Phase
  6, next-number → Phase 8, the other three call sites left untouched)
  matches the plan precisely, including line numbers.
- The migration script's deliberate non-repoint to the not-yet-released
  `accelerator-work` binary (self-contained
  `compile_legacy_rename_format` instead) is present and matches the
  documented reasoning.
- `cli/corpus-adapters/tests/work_item_pattern_parity.rs` is deleted; the
  one test in `parity.rs` calls `corpus_adapters::compile_scan_regex`
  directly rather than shelling to the deleted bash script.
- Review-1's `parent`/`relates_to` frontmatter keys are correctly omitted
  (not empty-valued), matching ADR-0040.

#### Deviations from Plan:

- `skills/work/stress-test-work-item/SKILL.md` does not carry the
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)` grant the plan's
  Phase 9 item 1 says to add to it. In practice this skill never invoked
  any of the 8 deleted scripts either, so the grant was never load-bearing
  — a stale item in the plan's file list rather than a functional gap.
- `work-item-template-field-hints.golden` uses a flat, `\n`-encoded
  single-line format rather than the richer per-case-directory format the
  plan's Phase 1 prose said this golden would need ("template content").
  This is a reasonable simplification (the case references the real
  template via `accelerator config template work-item` rather than
  embedding template content), not a parity gap — all rows are
  cross-checked against the real template per the file's own header.

#### Potential Issues:

- `mise run` at current HEAD fails on an unrelated pre-existing issue from
  a later commit outside 0170's scope (see Automated Verification). This
  should be tracked and fixed independently before `mise run` can be
  reported green in general, but it does not indicate any defect in this
  plan's implementation.
- References to the deleted scripts by name remain in
  `skills/work/*/evals/benchmark.json` (grading-criteria text describing
  historical skill behaviour) and in the Phase-1 golden files' own header
  comments — outside `meta/` as the AC's grep check technically requires,
  but inert (JSON prose / comments, not live shell-outs) and arguably
  correct to keep as historical record.

### Manual Testing Required:

The plan itself marks several manual-verification items as intentionally
not yet performed (interactive skill runs this session could not drive).
These remain outstanding and are not required for the automated
acceptance criteria to be considered met, but should be done before
relying on the repointed skills in anger:

1. Skill end-to-end runs:
   - [ ] Drive `create-work-item` end to end with no integration
         configured, confirming Step 5's write goes through `accelerator
         work create` and the integration-configured path is unaffected
         (Phase 7).
   - [ ] Drive `update-work-item` end to end on a scratch work item,
         confirming the id-immutability error surfaces the CLI's message
         (Phase 8).
   - [ ] Drive `update-work-item`, `list-work-items`, `create-work-item`,
         and `extract-work-items` against a scratch work item, confirming
         no residual shell-out to a deleted script (Phase 9).
   - [ ] Confirm `sync-work-items`' dirty-check guard
         (`work-item-file-dirty.sh`) still functions unmodified against a
         deliberately-dirtied local file (Phase 9).

2. Unrelated follow-up:
   - [ ] Fix `test_bootstrap_exports_the_one_plugin_root_the_launcher_reads`
         (introduced by commit `98c17ab3f233`, outside this plan's scope)
         so `mise run` is green again at HEAD.

### Recommendations:

- Add the `accelerator work *` `allowed-tools` grant to
  `stress-test-work-item/SKILL.md` for consistency with the plan's stated
  intent, or explicitly note in the plan that this skill needed no
  repoint (cosmetic, non-blocking).
- Track the `test_bootstrap_exports_the_one_plugin_root_the_launcher_reads`
  failure separately — it blocks a clean `mise run` for any future work
  until fixed, independent of 0170.
- Complete the outstanding manual skill-run verification items above
  before the repointed skills are considered fully validated in
  production use.
- All 8 acceptance criteria on `meta/work/0170-work-item-subdomain-and-sync-engine.md`
  are met and have been marked complete on the work item.
