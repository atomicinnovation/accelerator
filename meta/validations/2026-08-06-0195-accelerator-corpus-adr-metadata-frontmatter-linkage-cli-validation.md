---
type: plan-validation
id: "2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli-validation"
title: "Validation Report: accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI Implementation Plan"
date: "2026-08-08T14:43:51+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli"
tags: []
last_updated: "2026-08-08T14:43:51+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI Implementation Plan

### Implementation Status

All 5 phases are implemented and committed (`0cac8d986136` plan/research/review,
`bcb6c98eac8b` Phase 1, `2b34a557daa3` Phase 2, `613a06fa9f4a` Phase 3,
`6b09ed12db59` Phase 4, `c5d7716fda75` Phase 5, plus four later follow-up
commits addressing bugs surfaced after the item closed — see "Post-closure
fixes" below). Work item `0195` is already marked `done`; the plan's own
frontmatter still read `status: ready`, corrected to `done` by this report.

✓ Phase 1: Binary scaffold, sub-binary registration, `adr` noun group —
  fully implemented. `cli/corpus-cli/` exists (package `accelerator-corpus`),
  `adr next-number`/`adr read-status` work correctly against real files,
  `corpus::scan::{DirReader,FileReader}` + `corpus_adapters::fs::RealFs`
  present, sub-binary registered in `tasks/shared/paths.py`,
  `tasks/manifest.py`, `tasks/build.py`, `tests/integration/tasks/test_github.py`.
✓ Phase 2: `metadata derive` — fully implemented and wired into all 14 call
  sites (`create-adr`, `extract-adrs`, `conduct-spike`, `research-issue`,
  `research-codebase`, `create-note`, `create-work-item`, `review-work-item`,
  `extract-work-items`, `describe-pr`, `review-pr`, `create-plan`,
  `review-plan`, `validate-plan`), each with a matching `allowed-tools` grant.
✓ Phase 3: `linkage extract` — fully implemented, including the
  project-root-relative `table_from_config` design correction documented in
  the plan (verified: `table_from_config` joins the root at read time, not at
  table-build time). 0007 migration's `linkage-parser.sh` call site rewritten.
✓ Phase 4: `frontmatter validate` — fully implemented, including the 17th
  `DuplicateId` violation code, the `Checks`/`TargetOutcome::Skipped` design,
  the raw-text `parse_entries` correction (needed because a real YAML parser
  loses quote-syntax information `UNQUOTED-ID`/`BAD-LINKAGE-SHAPE` depend on),
  and the `--dir` doc-type-allowlist filtering fix. 0007 migration's
  `validate-corpus-frontmatter.sh` call sites rewritten with `log_warn`/
  VCS-revert guard messages matching every sibling guard in that script.
✓ Phase 5: Documentation and final acceptance sweep — `docs-site/src/content/docs/corpus.md`
  exists, registered in `docs-site/astro.config.mjs`'s sidebar, README.md's
  Concepts list carries a "Corpus CLI" entry (README.md:57-59).

### Automated Verification Results

✓ `cargo test --manifest-path cli/Cargo.toml -p accelerator-corpus -p corpus
  -p corpus-adapters` — all suites pass, 0 failed (re-run today; includes the
  unconditional whole-corpus `frontmatter validate` self-check).
✓ `mise run cli:check` — passes (clippy/rustfmt/lint across the workspace
  incl. `corpus-cli`).
✓ `mise run lint:dispatch-coherence:check` — passes.
✓ `mise run test:integration:config` — passes, 0 failed, floor at 16 with
  `_REQUIRED_CONFIG_SUITES` down to the single remaining entry
  (`scripts/test-skill-frontmatter-conformance.sh`). Run standalone twice
  today; both clean. (One run nested inside the full `mise run` aggregate hit
  a `rm: Directory not empty` cleanup race — this is the pre-existing,
  known-flaky-under-parallel-load behaviour of this suite, not a 0195
  regression; a solo re-run immediately after was clean.)
✓ `mise run test:integration:decisions` — passes with the decisions floor at 0.
✓ `uv run pytest tests/integration/tasks/test_github.py
  tests/unit/tasks/shared/test_dispatch_coherence.py
  tests/unit/tasks/test_signing.py tests/integration/tasks/test_release.py`
  — 154 passed.
✓ Repo-wide grep for executable invocations (`Bash(...)`, `allowed-tools`,
  shell call sites) of the five removed scripts — zero remain outside
  documentation/ADR/CHANGELOG prose, which AC2 explicitly excludes.
✓ Manual smoke test today: `accelerator-corpus adr next-number --count 2`,
  `accelerator-corpus linkage extract meta/work/0195-....md`,
  `accelerator-corpus frontmatter validate` (whole corpus, no arguments) —
  all ran correctly against this repo's real files; the frontmatter
  self-check reported 0 violations.

### Code Review Findings

#### Matches Plan:

- All five removed bash scripts (`adr-next-number.sh`, `adr-read-status.sh`,
  `test-adr-scripts.sh`, `artifact-derive-metadata.sh`, `linkage-parser.sh`,
  `test-linkage-parser.sh`, `validate-corpus-frontmatter.sh`,
  `test-validate-corpus-frontmatter.sh`) are confirmed absent from disk.
- All 8 skill call-site rewrites for `adr`/`metadata` verified with the
  grant-scoping the plan specified (wildcarded `corpus adr *` for
  multi-verb skills, exact-match single-verb grants elsewhere).
- Registration checklist: `DISPATCHED_SUBBINARIES` includes `"corpus"`,
  `_SUBBINARY_MANIFESTS` maps it to `cli/corpus-cli/Cargo.toml`,
  `_CLI_RELEASE_BINARIES` includes `"accelerator-corpus"`, and
  `test_github.py`'s upload-count assertion is the derived expression the
  plan specified, not a bumped literal.
- `ACCELERATOR_CORPUS_BIN` dev-override wiring present in
  `tasks/test/helpers.py`'s `accelerator_env()` and `tasks/build.py`'s
  `cli_dev` task, exactly as the plan's "Dev/test wiring gap closed" note
  describes.
- Em-dash (U+2014) `Display` separator pinned by its own character-level
  unit test (`the_separator_is_a_literal_em_dash`,
  `cli/corpus/src/frontmatter_validation/violation.rs:173`), independent of
  any golden-string comparison.
- The `schema::SCHEMA` `work-item` row's `typed_linkage_keys` includes
  `"source"` (the bug the plan's implementation notes describe finding and
  fixing via the schema-TSV parity test).
- `--dir` doc-type-allowlist filtering regression test present
  (`dir_walking_skips_files_outside_every_configured_doc_type_directory`,
  `cli/corpus-cli/tests/frontmatter_goldens.rs:107`).

#### Deviations from Plan (all previously documented in-plan as corrections):

- `table_from_config` stays project-root-relative rather than absolute
  (plan's own "Design correction found during implementation").
- `corpus::frontmatter_validation` validates raw frontmatter text via a
  ported `parse_entries`, not solely the parsed `Mapping`, to preserve
  quote-syntax information (plan's own documented correction).
- Full byte-for-byte parity against all 49 of the original bash validator's
  cases was not attempted 1:1; coverage is per-violation-code plus every
  explicitly flagged bash quirk instead — the plan states this was a
  deliberate, scoped trade-off given the `parse_entries` redesign, not an
  oversight.

#### Potential Issues:

- `templates/{plan,note,rca,codebase-research}.md` still contain placeholder
  comments reading `"{... from artifact-derive-metadata.sh}"` — the removed
  script's name lingers in these template files. This is a documentation
  mention, not an executable invocation, so it falls outside AC2's literal
  scope (which explicitly excludes "bare documentation/ADR mentions") and
  does not affect functionality — the skills that populate these templates
  already call `accelerator corpus metadata derive` correctly. Worth a small
  follow-up cleanup for accuracy, not a plan defect.

### Manual Testing Required:

The plan's own checklists already mark most manual verification items
complete `[x]` from implementation-time testing, with three items explicitly
left unexercised (`[ ]`) and reasons given in-plan:

1. Interactive skill invocation:
   - [ ] `/accelerator:create-adr`, `/accelerator:extract-adrs`,
     `/accelerator:review-adr` invoked interactively, confirming no
     permission prompt fires for the old script path (Phase 1 — requires an
     interactive Claude Code session with the locally-built binary on the
     dispatch path).
   - [ ] `/accelerator:create-plan`, `/accelerator:create-adr` invoked
     interactively, confirming the metadata block still renders correctly
     (Phase 2 — same constraint).
2. 0007 migration script:
   - [ ] The 0007 migration's own interactive test suite
     (`skills/config/migrate/scripts/test-migrate-0007.sh`) was not
     independently re-run by this validation pass (it is `INTERACTIVE: yes`
     and not wired into any `mise run test:*` task); the plan records it was
     run to completion during Phase 4 implementation (195 passed, 0 failed).

### Recommendations:

- Optionally refresh the `{... from artifact-derive-metadata.sh}` placeholder
  comments in `templates/plan.md`, `templates/note.md`, `templates/rca.md`,
  and `templates/codebase-research.md` to name the new
  `accelerator corpus metadata derive` command, for documentation accuracy.
- No blocking issues. The plan is fully implemented, its own success
  criteria are honestly and accurately marked, and every automated check
  re-run today is green.
