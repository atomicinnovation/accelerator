---
type: plan-validation
id: "2026-06-17-0114-fix-migration-0007-incomplete-mechanical-normalisation-validation"
title: "Validation Report: Fix Migration 0007 Incomplete Mechanical Normalisation"
date: "2026-06-18T11:40:04+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-06-17-0114-fix-migration-0007-incomplete-mechanical-normalisation"
tags: [migrate, frontmatter, validator, unified-schema, "0007", awk]
last_updated: "2026-06-18T11:40:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Fix Migration 0007 Incomplete Mechanical Normalisation

### Implementation Status

✓ Phase 1: Single-source path classification + `meta/prs/` + `meta/docs/` — Fully implemented
✓ Phase 2: Schema-driven forbidden own-id key drop + `pr_title`→`title` fold — Fully implemented
✓ Phase 3: Unconditional `ticket`/`ticket_id` drop — Fully implemented
✓ Phase 4: Required-extras backfill (`topic`/`pr_number`/`review_number`/`verdict`/`lenses`) — Fully implemented
✓ Phase 5: Non-canonical linkage shape coercion (`PR #N` → `pr:N`) — Fully implemented
✓ Phase 6: Integration capstone — combined fixture corpus, validator-clean by construction — Fully implemented

All six gaps identified in the RCA are closed. Every automated success-criterion
checkbox across the six phases is marked complete and verified against the code.

### Automated Verification Results

✓ Migrate suite passes: `bash skills/config/migrate/scripts/test-migrate-0007.sh`
  — **155 passed, 0 skipped, 0 failed** (matches the plan's claimed count exactly)
✓ Shell checks pass: `mise run scripts:check` — shfmt (format), ShellCheck
  (`enable=all`), and the bash-3.2 bashisms linter all green
✓ Working tree clean (`jj status`) — implementation fully committed across six
  atomic commits (`slmowtlxxnqy`..`uuknrtskyzts`)

The full `mise run` (bare default) end-to-end run was performed during
implementation (Phase 6 AC) and recorded green except for a pre-existing,
unrelated visualiser dev-server timing flake
(`test_dev_integration.py::test_status_exit_codes`) that passes in isolation.
This work is bounded to the `scripts` component, whose targeted check is green.

### Code Review Findings

#### Matches Plan:

- **Phase 1**: `scripts/doc-type-inference.sh` created as the single source for
  `infer_type_from_path` + `out_of_scope`, with the new `*/prs/* → pr-description`
  arm (ordered after `*/reviews/prs/*`) and the `*/meta/docs/*` skip arm. Both the
  migration (`0007-…sh`) and the validator (`validate-corpus-frontmatter.sh`)
  source it via an env-overridable seam (`DOC_TYPE_INFERENCE`) and the local
  byte-identical copies are deleted. The awk `path_to_typed` gains the matching
  `^meta\/prs\/ → pr-description` arm with the documented cross-reference comment.
  The awk now drops an empty `type:` value so the closing-fence backfill emits the
  inferred type.
- **Phase 2**: `SCHEMA_TSV` made overridable; `forbidden_keys_for_type()` reads
  TSV col 6; `fm_assert_schema_columns()` added to `frontmatter-emission-rules.sh`
  (prefix-match, CRLF-tolerant, `$'\t'`-built) and invoked by both the migration
  pre-harness block and the validator. The awk gains `is_forbidden()`,
  `emitted_title` state, the drop/fold arm at the pinned position (before
  omit-when-empty), and the closing-fence `!emitted_title` guard.
- **Phase 3**: Unconditional `ticket`/`ticket_id` awk drop arm with the
  non-empty `0007-DIVERGE[dropped-legacy-key]` breadcrumb.
- **Phase 4**: `extras_for_type()` off-by-one fixed (`cut -f5`→`-f4`); dead
  `status_vocab_for_type()` removed; `FM_OPTIONAL_EXTRAS` reused (not duplicated);
  `fm_is_empty_val()`, `extra_default()` (with PR-anchored/date-guarded `pr_number`
  derivation), the packed US-separator (`$'\x1F'`) backfill channel, and the
  callable awk `emit_backfill_extras()` (octal `\037` split, list/scalar
  cardinality, sentinel breadcrumbs) all present and matching. Fence-less `topic`
  backfill widened to `note`/`codebase-research`/`issue-research` with title
  quote-stripping.
- **Phase 5**: `normalize_pr_ref()` added and chained innermost
  (`normalize_bare(normalize_pr_ref(normalize_paths(val)))`), matching all the
  documented spelling variants with the `[Pp][Rr][ -]?#?|#` pattern.
- **Phase 6**: Combined-corpus capstone block, mechanical-passes-only regression
  guard, prepass-coexistence and namespace-collision ACs all present and passing.

#### Deviations from Plan:

- **Breadcrumb capture mechanism (documented, corrected in-flight).** The plan
  assumed `run_0007` captures migration stderr (`2>&1`); the implementation
  discovered the runner sandboxes migration stderr to a per-migration log it
  deletes on success, so DIVERGE breadcrumb assertions route through a
  `run_0007_direct` helper instead. The plan's own AC text was updated to record
  this correction. This is an improvement (correct mechanism), not a gap.
- **Test-only sourcing seam added.** An `ACCELERATOR_0007_NO_RUN` sentinel
  (guarded `return`/`exit 0`) was added so the suite can unit-test the pure shell
  helpers without triggering orchestration. A sensible, well-commented addition
  beyond the plan's explicit text, consistent with its assertion discipline.

#### Potential Issues:

- **Sentinel `verdict`/`lenses` values** (`"unknown"`) are a deliberate, plan-
  documented accepted limitation: a committed sentinel will not re-warn on
  subsequent runs. The asserted `0007-DIVERGE[backfilled-extra]` breadcrumb is the
  audit trail. Operators must manually follow up sentinel-backfilled reviews.
- **Residual three-copy path→type encoding** (two bash copies collapsed to one;
  the awk's `path_to_typed` remains a third, in-runtime copy with a distinct
  input). This is explicitly documented with cross-reference comments and pinned
  by a table-driven fixture, so drift fails loudly. Accepted by design.

### Manual Testing Required:

1. Real-world downstream unblock:
  - [ ] Re-run `/accelerator:migrate` against the originally-failing downstream
        corpus; confirm 0007 completes (reaches the interactive harness; records
        as applied).
  - [ ] Confirm the `INVALID-TYPE` / `FORBIDDEN-OWN-ID` / `OBSOLETE-LEGACY-KEY` /
        `BAD-LINKAGE-SHAPE` buckets are gone for `meta/prs/` + affected files.
  - [ ] Confirm `meta/docs/` files are byte-unchanged.

2. Legibility spot-checks:
  - [ ] Sentinel-backfilled reviews and folded titles read sensibly.
  - [ ] `0007-DIVERGE[backfilled-extra]` log lines name the right files.

3. Recovery rehearsal:
  - [ ] Deliberately abort 0007 mid-run; confirm a single VCS revert of `meta/`
        recovers the tree (the new abort log lines now signpost this).

### Recommendations:

- Proceed to PR. The implementation is complete, atomically committed, and the
  automated gate is green.
- The manual verification items are inherently manual (real downstream corpus,
  legibility, recovery rehearsal) and do not block merge given the comprehensive
  fixture coverage; schedule them as a post-merge real-world confirmation.
- No code changes are recommended — the implementation matches the plan with two
  improvements (corrected breadcrumb-capture mechanism, test-only helper seam).
