---
type: plan-review
id: "2026-06-17-0114-fix-migration-0007-incomplete-mechanical-normalisation-review-1"
title: "Plan Review: Fix Migration 0007 Incomplete Mechanical Normalisation"
date: "2026-06-17T22:58:39+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-17-0114-fix-migration-0007-incomplete-mechanical-normalisation"
target: "plan:2026-06-17-0114-fix-migration-0007-incomplete-mechanical-normalisation"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, code-quality, test-coverage, architecture, safety, compatibility, standards]
review_number: 1
review_pass: 4
tags: [migrate, frontmatter, validator, "0007", awk, review]
last_updated: "2026-06-18T00:05:26+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Fix Migration 0007 Incomplete Mechanical Normalisation

**Verdict:** REVISE

This is a strong, RCA-grounded plan: it correctly frames the validator as the
contract and the mechanical passes as the implementation that must satisfy it,
drives transforms off the schema where the fact is already encoded, single-
sources the duplicated path classification, and decomposes into independently-
mergeable phases capped by a validator-clean-by-construction regression guard.
The reason for REVISE is concentrated, not pervasive: **one critical defect** in
the new `extra_default` helper would re-introduce the exact mid-migration abort
this work exists to fix, and a cluster of **major data-safety and test-fidelity
gaps** sit in the three new "fabricate/discard a value" transforms (pr_title
fold, ticket drop, verdict/lenses sentinels) and in the test plan's reliance on
manual rather than automated verification of those very behaviours. All are
addressable with targeted edits — the architecture is sound.

### Cross-Cutting Themes

- **Unguarded shell under `set -euo pipefail`** (flagged by: correctness, code-
  quality) — the `extra_default` pr_number pipeline (`grep -oE | head -1`) has no
  `|| true` guard, against every other `grep -oE` in the file. On a digit-less
  stem it aborts the whole migration mid-rewrite — the permanent-stall failure
  mode the plan is fixing, re-introduced via a new path. **This is the critical
  finding.**

- **Silent value fabrication / destruction without a persistent audit trail**
  (flagged by: safety, correctness, test-coverage) — three new transforms
  change data with no durable signal: `pr_title` is discarded when a differing
  `title:` exists (no breadcrumb); `ticket`/`ticket_id` is dropped on any value
  (no breadcrumb); `verdict: "unknown"`/`lenses: ["unknown"]` sentinels are
  injected and pass validation (the only warning is a first-run stderr line that
  never re-fires). The test plan demotes the one breadcrumb that *does* exist to
  manual verification.

- **`pr_title` fold arm needs hardening** (flagged by: correctness, safety) —
  the same arm both emits `title: ""` on an empty `pr_title` (an EMPTY-PLACEHOLDER
  re-violation) and silently discards a non-empty `pr_title` when a title exists
  (data loss). One arm, two distinct defects.

- **`*/docs/*` exclusion is too broad and changes shared behaviour** (flagged
  by: compatibility, safety, correctness) — adding `*/docs/*` to the *shared*
  `out_of_scope` silently stops every validator consumer (not just the migration)
  from validating any `meta/docs/` subtree, and the glob over-matches nested
  `*/docs/*` rather than top-level `meta/docs/`.

- **Idempotency of the new transforms is assumed, not asserted** (flagged by:
  test-coverage, compatibility) — the existing `=== Idempotency ===` block
  carries none of the new reproduction shapes, so "re-confirm it stays green" per
  phase proves nothing about the new arms; the cross-session 0001-then-0007 path
  is also unverified.

- **Single-sourcing is incomplete; positional schema coupling persists** (flagged
  by: architecture, compatibility, code-quality, standards) — the awk's
  `path_to_typed` remains a third hand-maintained path→type copy, the `cut -f4`/
  `-f6` reads hard-code column positions, and a dead `status_vocab_for_type`
  sibling is left beside the corrected `extras_for_type`.

### Tradeoff Analysis

- **Determinism of a frozen migration vs. reuse of an evolving contract**: Phase
  4 sources `frontmatter-emission-rules.sh` to avoid duplicating
  `FM_OPTIONAL_EXTRAS` (good DRY), but that file is documented as evolving. A
  shipped migration must reproduce its *historical* output forever; binding it to
  a moving constant risks a future edit silently changing what 0007 backfills.
  Recommendation: keep the reuse but confirm/freeze the optional-extras set 0007
  depends on (or snapshot it), and document the coupling direction.

- **Schema-driven (DRY) vs. positional fragility**: reading TSV columns by index
  is what makes the drop schema-driven, but `cut -fN` re-introduces the same
  positional coupling class that caused the original `extras_for_type` off-by-one.
  Recommendation: accept the column reads but pin them by header-name assertion
  in one place so a future reshape fails loudly.

### Findings

#### Critical

- 🔴 **Correctness / Code Quality**: Unguarded `grep -oE | head -1` in
  `extra_default` aborts the migration under `set -euo pipefail`
  **Location**: Phase 4, Section 1 (`extra_default`)
  The pr_number pipeline has no `|| true` guard; on a digit-less stem `grep`
  exits 1, `pipefail` propagates, and `set -e` aborts 0007 mid-rewrite — leaving
  the corpus partially mutated and 0007 unrecorded, the exact permanent-stall
  this plan fixes. Every existing `grep -oE` in the file (`:227,:285,:361,:397`)
  is guarded. The `| head -1` also risks SIGPIPE-under-pipefail (a hazard the
  validator explicitly documents avoiding).

#### Major

- 🟡 **Correctness**: `pr_number` derivation mis-parses date-prefixed stems
  **Location**: Phase 4, Section 1 (`extra_default` pr_number)
  `grep -oE '[0-9]+' | head -1` yields `2026` for a stem like
  `2026-06-17-pr-416-review` instead of `416`. It validates (presence-only) but
  ships silently-wrong data. Anchor to the pr-token or DIVERGE when ambiguous.

- 🟡 **Correctness**: `pr_title` fold can emit `title: ""` (EMPTY-PLACEHOLDER)
  **Location**: Phase 2, Section 2 (drop arm + fold)
  The fold has no empty-value guard; `pr_title: ""` produces `title: ""`, and the
  forbidden-drop `next` skips the omit-when-empty handler, so the file re-fails
  the gate. Guard with `&& !is_empty_val(val)`.

- 🟡 **Safety**: `pr_title` silently discarded when a differing `title:` exists
  **Location**: Phase 2, Section 2 (drop arm + fold)
  When `!has_title` is false the `pr_title` value is dropped with no breadcrumb;
  pr_title and title can legitimately differ, so real content is lost with no log
  signal. Emit a `0007-DIVERGE[discarded-key]` on a non-empty discard.

- 🟡 **Safety**: Unconditional `ticket`/`ticket_id` drop destroys data silently
  **Location**: Phase 3, Section 1 (awk drop arm)
  The drop inspects no value and logs nothing. A hand-added `ticket: "PROJ-1234"`
  (external tracker the plugin doesn't model) is destroyed with no signal. Emit a
  `0007-DIVERGE[dropped-legacy-key]` carrying key+value before `next`.

- 🟡 **Safety / Test Coverage**: Fabricated `verdict`/`lenses` sentinels are
  indistinguishable downstream and their only breadcrumb is unasserted
  **Location**: Phase 4 (sentinel backfill) + Testing Strategy
  `verdict` is an extra (no vocab check), so `"unknown"` reads as real review
  state to the visualiser/indexers; the `0007-DIVERGE[backfilled-extra]` line
  fires only on first run and is never asserted by an AC. Make the sentinel self-
  identifying/persistent AND add an automated AC asserting the breadcrumb.

- 🟡 **Test Coverage**: Fail-first TDD step is asserted nowhere
  **Location**: Implementation Approach (TDD discipline) + per-phase ACs
  Every AC checks only the green end-state; nothing verifies the fixture first
  failed for the targeted violation code, so a fixture can go green for the wrong
  reason. Add per-phase positive assertions on the transformed content (or a red-
  state violation-code check), not just validator-clean.

- 🟡 **Test Coverage**: Per-phase idempotency re-test is prose, not an AC
  **Location**: Migration Notes + per-phase ACs
  The existing `=== Idempotency ===` block carries none of the new shapes, so
  "re-confirm it stays green" proves nothing about the new (fold/sentinel/pr-ref)
  arms. Add a per-phase AC re-running on the just-migrated fixture and asserting
  an empty `meta/` diff.

- 🟡 **Test Coverage**: Phase 5 linkage coercion lacks negative/multi-element
  fixtures
  **Location**: Phase 5, Success Criteria
  Only single-token positives are covered. No fixture for a list value mixing
  `"PR #N"` with an already-typed ref, multiple PR tokens, or idempotency of an
  already-canonical `"pr:416"`. Add these to pin the regex against over-matching.

- 🟡 **Architecture**: Single-sourcing leaves the awk's `path_to_typed` as a
  third hand-maintained copy
  **Location**: Phase 1, Key Discoveries + Change 4
  `path_to_typed` classifies *referenced* paths (not the current file) so it
  cannot consume `-v type`; it keeps its own case-ladder and gains the
  `meta/prs/` arm by hand. The drift class is reduced from three copies to two,
  not eliminated. Document why, cross-reference, and add a fixture asserting
  `meta/prs/` linkage targets resolve to `pr-description`.

- 🟡 **Compatibility / Safety / Correctness**: `*/docs/*` out-of-scope is too
  broad and changes validator behaviour for all consumers
  **Location**: Phase 1, Section 1 (`out_of_scope`)
  Added to the *shared* helper, it silently stops every validator consumer from
  validating any `meta/docs/` file, and the glob matches any nested `*/docs/*`,
  not just top-level `meta/docs/`. Narrow to `*/meta/docs/*` (or scope the skip to
  the migration's enumeration), document the validator-wide change, and add a
  whole-corpus-mode validator AC.

- 🟡 **Code Quality**: `rewrite_file` `-v` threading is becoming a god-function
  **Location**: Implementation Approach + Phase 2/4 (`-v` lists)
  Already ~24 `-v` flags + ~14 presence locals; Phase 2 adds `forbidden`, Phase 4
  adds five `bf_*` plus per-extra absence/default computation, with one shell
  local + one awk param + one emit arm per extra (triplication inviting drift).
  Consider a single packed `backfill_extras` channel parsed by a generic emit
  loop keyed on extra name.

#### Minor

- 🔵 **Correctness**: `topic` backfill reuses the title without stripping
  embedded quotes (the `title_default` path uses `tr -d '"'` precisely to avoid
  malformed YAML / a visualiser panic). **Location**: Phase 4, Sections 1–2.
- 🔵 **Correctness**: Drop-arm placement relative to the omit-when-empty handler
  (`:301`) is left ambiguous; pin it explicitly. **Location**: Phase 2/3.
- 🔵 **Code Quality**: Dead `status_vocab_for_type` (`:53`) left beside the fixed
  `extras_for_type`; collapse the duplicate status helpers and drop the stale
  `# 5th col` comment. **Location**: Phase 4, Section 1. (Also flagged by
  standards.)
- 🔵 **Code Quality**: `is_forbidden` is a fourth copy of the split-and-scan
  membership idiom; the verdict/lenses DIVERGE emission is duplicated. Factor
  `in_list()` and `diverge_backfill()`. **Location**: Phase 2/4.
- 🔵 **Architecture / Compatibility**: Positional schema-column coupling
  (`cut -f4`/`-f6`) shared with the validator; pin columns by header name in one
  place. **Location**: Phase 2/4.
- 🔵 **Architecture**: Migration sources the *evolving* `frontmatter-emission-
  rules.sh` from a frozen migration — confirm/snapshot the optional-extras set.
  **Location**: Phase 4, Section 1.
- 🔵 **Architecture**: Env-overridable `SCHEMA_TSV`/`DOC_TYPE_INFERENCE` is a
  sound test seam (mirrors `FM_EMISSION_RULES`) but should be documented test-
  only. **Location**: Phase 1/2.
- 🔵 **Architecture**: Newly-typeable `pr-description` files now enter the corpus
  index and the interactive `migration_emit_transformations` loop — add a
  linkage-bearing PR-description fixture and confirm duplicate-id behaviour.
  **Location**: Phase 1.
- 🔵 **Test Coverage**: Validator-clean idiom (~9 lines) duplicated across 6+ new
  blocks; extract `assert_validates` / `assert_validator_violation`.
  **Location**: Testing Strategy.
- 🔵 **Test Coverage**: `meta/docs/` validator-side skip not isolated from the
  migration-side skip (whole-corpus vs file-list mode). **Location**: Phase 1.
- 🔵 **Test Coverage**: 0001→0007 integration asserts no-clobber but not combined
  idempotency on re-run. **Location**: Phase 3/6.
- 🔵 **Test Coverage**: `pr_title` fold doesn't test the closing-fence title-
  default interaction (exactly one `title:` across present/absent variants of a
  title-less file). **Location**: Phase 2.
- 🔵 **Compatibility**: Cross-session path (0001 already applied, only 0007 runs
  now) is unverified — the more common downstream case. **Location**: Phase 3/6.
- 🔵 **Compatibility**: Migration resolves helpers via `PLUGIN_ROOT`, validator
  via `SCRIPT_DIR`; note they must resolve to the same tree and the env overrides
  are test-only. **Location**: Phase 1/4.
- 🔵 **Safety**: Batch mutation has no all-or-nothing boundary on mid-run abort;
  signpost VCS-revert recovery in a Manual Verification step / abort log line.
  **Location**: Implementation Approach / Phase 6.
- 🔵 **Standards**: Confirm `doc-type-inference.sh` lints clean under
  `.shellcheckrc enable=all`; add a top-of-file directive in the
  `frontmatter-emission-rules.sh` style only if needed. **Location**: Phase 1.

#### Suggestions

- 🔵 **Code Quality**: Add micro-assertions on the pure helpers
  (`extra_default pr_number … → 430`, `forbidden_keys_for_type pr-review →
  'pr_title review_pass'`) so schema-reading logic is testable without a full
  corpus run. **Location**: Phase 6 / Testing Strategy.
- 🔵 **Standards**: Update the validator's top-of-file comment and the
  `out_of_scope` note (still says "specs/talks/global") to mention
  `doc-type-inference.sh` and the widened skip set. **Location**: Phase 1,
  Section 3.
- 🔵 **Standards**: Cite/align the `lenses: ["unknown"]` flow-list emission
  spelling with the `tags: []` precedent and committed templates. **Location**:
  Phase 4, Section 2.

### Strengths

- ✅ Correctly frames the validator as the contract and drives transforms off it
  (TSV col 6 forbidden keys, col 4 − `FM_OPTIONAL_EXTRAS` required extras) rather
  than re-encoding facts.
- ✅ Single-sources the two byte-identical bash copies of `infer_type_from_path`/
  `out_of_scope`, directly addressing the RCA's primary Contributing Factor.
- ✅ Accurate root-cause diagnosis: the `extras_for_type` `cut -f5→-f4` off-by-one
  on dead code, the schema column order, and the `pr:N` validation path are all
  verified correct against the source.
- ✅ The `normalize_pr_ref → normalize_bare` chaining is provably idempotent and
  non-overlapping; the new `meta/prs/` arm cannot shadow `reviews/prs`.
- ✅ Sentinel `verdict`/`lenses` values are correctly chosen to clear
  EMPTY-PLACEHOLDER and pass the validator (the *downstream* legibility of the
  sentinel is the remaining concern, not its validity).
- ✅ Genuinely independently-mergeable phases with self-complete fixtures, capped
  by a Phase 6 validator-clean-by-construction regression guard that encodes the
  RCA's recommended invariant.
- ✅ Schema-driven proof fixture (custom `SCHEMA_TSV` with a novel forbidden key)
  is a real test of the abstraction, not the value.
- ✅ Reasons explicitly and correctly about editing a shipped migration:
  already-applied corpora never re-run, and the blocked corpora never recorded
  0007 so they legitimately re-run the fixed version.
- ✅ Defaults keyed on extra-*name* naturally cover plan-review/work-item-review,
  avoiding per-type duplication.
- ✅ Strong convention discipline: kebab-case helper, correct `# shellcheck
  source=` directive, breadcrumb format and awk style consistent with the
  existing file, bash 3.2 floor honoured.

### Recommended Changes

1. **Guard the `extra_default` pr_number pipeline** (addresses: critical
   unguarded-grep). Add `|| true` (and reconsider `head -1` ordering / capture)
   so a digit-less stem can never abort the migration. Re-verify under
   `set -euo pipefail` with a digit-less pr fixture. *This single fix is the
   blocker; the plan cannot ship without it.*

2. **Harden the `pr_title` fold arm** (addresses: empty-fold EMPTY-PLACEHOLDER,
   silent pr_title discard). Add `&& !is_empty_val(val)`; emit a
   `0007-DIVERGE[discarded-key]` when a non-empty `pr_title` is dropped against a
   differing `title:`.

3. **Make destructive/fabricating transforms auditable** (addresses: ticket drop,
   verdict/lenses sentinels). Emit a breadcrumb on every non-empty
   `ticket`/`ticket_id` drop; make the sentinel self-identifying/persistent in the
   frontmatter; add automated ACs asserting both breadcrumbs.

4. **Fix pr_number derivation for date-prefixed stems** (addresses: mis-parse).
   Anchor to the pr-token or DIVERGE when no unambiguous number exists.

5. **Narrow and document the `meta/docs/` exclusion** (addresses: over-broad
   `*/docs/*`). Use `*/meta/docs/*`, note the validator-wide scope change in
   Migration Notes, and add a whole-corpus-mode validator AC.

6. **Add idempotency + fail-first ACs** (addresses: TDD-fidelity, idempotency-
   assumed). Per phase: assert the targeted transform on content (not just
   validator-clean) and re-run on the migrated fixture asserting an empty diff;
   extend Phase 5 with multi-element/negative/idempotency linkage fixtures.

7. **Acknowledge the residual drift surfaces** (addresses: third copy, column
   coupling, dead helper). Document `path_to_typed` as a retained third encoding
   with a cross-reference + alignment fixture; pin schema columns by header name;
   delete the dead `status_vocab_for_type`.

8. **Quote-strip the `topic` default and pin the drop-arm ordering** (addresses:
   malformed-YAML, placement ambiguity). Reuse the `tr -d '"'`/normalise path for
   the reused title; state the arm goes after the id/own-id handlers and before
   the omit-when-empty/linkage arms.

9. **(Optional, recommended) Reduce the `-v` threading wall** (addresses: god-
   function). A single packed `backfill_extras` channel parsed in awk would let
   Phase 4 add zero new `-v` flags and make future extras data-driven.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan is logically well-structured and traces each of the six
gaps to a concrete validator rule, with mostly-correct claims about awk arm
ordering, regex idempotency (the chained `normalize_pr_ref`/`normalize_bare`
interaction is sound), the schema TSV column indices, and the `extras_for_type`
off-by-one fix. The most serious correctness risk is an unguarded grep pipeline
in `extra_default` that will abort the migration under `set -euo pipefail` when a
stem has no digits, plus a `pr_number` derivation that mis-parses date-prefixed
stems and a `pr_title` fold that can emit an empty title placeholder. These are
subtle logic errors in the new transforms that the happy-path fixtures would not
catch.

**Strengths**:
- Correctly establishes that placing the new `*/prs/*` arm after `*/reviews/prs/*`
  preserves first-match discrimination; `^meta\/prs\/` cannot match
  `meta/reviews/prs/...` so no shadowing occurs.
- The `normalize_pr_ref → normalize_bare` chaining is provably idempotent and
  non-overlapping: rewritten `"pr:416"` contains no `#` and begins `"p` not `"4`.
- Sentinel `verdict`/`lenses` correctly satisfy the validator (non-empty; lenses
  is not a typed-linkage key for pr-review; neither has a vocab/value-shape check
  beyond presence).
- The `extras_for_type` off-by-one diagnosis is accurate (col 4 = extras, col 5 =
  status_vocab) and the function is genuinely dead code today, so the fix is safe.
- The `emitted_title` guard correctly prevents a duplicate `title:` after a
  `pr_title` fold.

**Findings**:
- 🔴 **critical / high** — Phase 4 `extra_default` (pr_number): unguarded
  `printf … | grep -oE '[0-9]+' | head -1` under `set -euo pipefail` aborts the
  migration mid-rewrite on a digit-less stem (every other `grep -oE` in the file
  is guarded; `head -1` adds a SIGPIPE-under-pipefail hazard). Re-introduces the
  partial-mutation/never-recorded stall this plan fixes. Guard with `|| true`.
- 🟡 **major / high** — Phase 4 `extra_default`: `grep -oE '[0-9]+' | head -1`
  yields the year for a date-prefixed stem (`2026-06-17-pr-416-review` → 2026 not
  416); validates but ships wrong data. Anchor to the pr-token or DIVERGE.
- 🟡 **major / medium** — Phase 2 fold arm: no empty-value guard → `pr_title: ""`
  emits `title: ""` (EMPTY-PLACEHOLDER), and the forbidden `next` skips the
  omit-when-empty handler. Add `&& !is_empty_val(val)`.
- 🔵 **minor / medium** — Phase 1 `out_of_scope`: `*/docs/*` matches any nested
  `/docs/` segment, broader than top-level `meta/docs/`. Anchor to `*/meta/docs/*`.
- 🔵 **minor / low** — Phase 4 `topic` backfill reuses the title without the
  `tr -d '"'` quote-stripping the `title_default` path applies → malformed YAML
  on a quote-bearing title.
- 🔵 **minor / medium** — Phase 2/3 drop-arm placement relative to the
  omit-when-empty handler (`:301`) is ambiguous; pin it explicitly.

### Code Quality

**Summary**: Unusually well-structured for a maintenance fix: each gap is closed
where the contract already encodes the fact, three duplicated copies of path
classification collapse to one sourced helper, and dead code (the
`extras_for_type` off-by-one) is repaired. The dominant code-quality risk is the
`rewrite_file` driver, already a god-function threading ~24 `-v` flags plus ~14
presence locals into awk; Phase 2/4 add `forbidden` plus five `bf_*` flags,
pushing toward an unmaintainable parameter wall. A concrete error-handling defect
also slips in: the proposed `extra_default` uses an unguarded `grep -oE | head -1`
under `set -euo pipefail`, contrary to the file's own `|| true` pattern.

**Strengths**:
- Collapses three near-identical copies of `infer_type_from_path`/`out_of_scope`
  into one sourced helper, eliminating the documented drift class at the root.
- Drives forbidden-key drop and required-extras backfill from the schema TSV and
  proves the schema-driven property with a custom-SCHEMA_TSV fixture.
- Repairs the `extras_for_type` `cut -f5→-f4` off-by-one and wires the dead helper
  into live use.
- Keys default providers on extra *name*, covering plan-review/work-item-review
  without duplication.
- Each phase ships independently green; Phase 6 binds the whole via a validator-
  clean-by-construction guard.

**Findings**:
- 🔴/🟡 **major (critical per correctness) / high** — Phase 4 `extra_default`:
  unguarded `grep -oE | head -1` under `pipefail` aborts the migration; matches
  the cross-cutting critical finding above.
- 🟡 **major / high** — `rewrite_file` `-v` threading god-function: ~24 flags
  growing to ~30 with one shell-local + awk-param + emit-arm per extra
  (triplication inviting drift). Consider a single packed `backfill_extras`
  channel with a generic emit loop.
- 🔵 **minor / high** — three status-vocab helpers (`:52-55`), only one live; the
  plan revives `extras_for_type` but leaves the dead `status_vocab_for_type` with
  its misleading `# 5th col` comment.
- 🔵 **minor / medium** — `is_forbidden` is a fourth membership-idiom copy; the
  verdict/lenses DIVERGE emission is duplicated. Factor `in_list()` /
  `diverge_backfill()`.
- 🔵 **suggestion / medium** — Phase 6 guard is end-to-end only; add direct micro-
  assertions on the pure helpers so schema-reading logic is unit-testable.

### Test Coverage

**Summary**: Unusually test-conscious — TDD discipline, per-shape fixtures, a
schema-driven proof, a 0001→0007 integration fixture, and a capstone combined-
corpus guard asserting validator-cleanliness at both the full-driver and
pre-harness levels. Coverage breadth is proportional to the blast radius. The
main gaps are in assertion *fidelity*: the fail-first step is unverifiable as
written, the DIVERGE breadcrumb and per-phase idempotency re-test are demoted to
manual checks despite being load-bearing, several reproduction shapes lack
fixtures, and ~9 lines of validator-clean boilerplate are duplicated with no
shared helper.

**Strengths**:
- Explicit per-phase TDD discipline — the right shape for a fix driven by
  validator violation codes.
- The custom-SCHEMA_TSV forbidden-key proof is genuinely mutation-resistant.
- The 0001→0007 integration tests the real two-migration sequence and asserts no
  clobber.
- Phase 6 asserts validator-cleanliness twice (full run + pre-harness guard),
  encoding the RCA's recommended property.
- Negative coverage exists for the highest-risk overwrite paths (existing
  topic/verdict never overwritten; pr_title fold both ways).

**Findings**:
- 🟡 **major / high** — Fail-first TDD step asserted nowhere; ACs check only the
  green end-state, so a fixture can pass for the wrong reason. Add content-level
  positive assertions / red-state violation-code checks.
- 🟡 **major / high** — Sentinel-backfill `0007-DIVERGE[backfilled-extra]`
  breadcrumb only manually verified, though it is the sole signal for injected
  `"unknown"` values and is assertable via the captured `RUN_OUT`.
- 🟡 **major / high** — Per-phase idempotency re-test is prose, not an AC; the
  existing Idempotency block carries none of the new shapes. Add a re-run/empty-
  diff AC per phase.
- 🟡 **major / medium** — Phase 5 linkage coercion lacks negative/multi-element/
  idempotency fixtures (list values, multiple PR refs, already-canonical `pr:N`).
- 🔵 **minor / high** — Validator-clean idiom duplicated across 6+ new blocks;
  extract `assert_validates` / `assert_validator_violation`.
- 🔵 **minor / medium** — `meta/docs/` validator-side skip not isolated from the
  migration-side skip (whole-corpus vs file-list mode).
- 🔵 **minor / medium** — 0001→0007 integration asserts no-clobber but not
  combined idempotency on re-run.
- 🔵 **minor / low** — pr_title fold doesn't test the closing-fence title-default
  interaction (exactly one `title:` across variants of a title-less file).

### Architecture

**Summary**: Structurally sound and well-reasoned — correctly identifies the
drift class (validator-as-contract vs. mechanical-passes-as-implementation),
chooses a schema-driven-where-encoded / single-sourced-where-duplicated strategy,
and reasons carefully about migration immutability, idempotency, and already-
migrated corpora. The central weakness is that the proposed `doc-type-inference.sh`
seam unifies only two of the three hand-maintained path-classification encodings —
the awk's `path_to_typed` remains an independent third copy. Secondary concerns
are positional schema-column coupling and a new dependency edge from a shipped
migration into the evolving emission-rules contract.

**Strengths**:
- Correctly frames validator-as-contract and drives transforms off it.
- Single-sources the two byte-identical bash copies, addressing the RCA's primary
  Contributing Factor.
- Reasons explicitly about migration immutability and already-migrated corpora.
- Genuinely independently-mergeable phases with a clean Phase 6 capstone.
- Phase 6 guard turns the contract relationship into an enforced property.
- Defaults keyed on extra-name for cohesion across review types.

**Findings**:
- 🟡 **major / high** — Phase 1: the seam leaves the awk's `path_to_typed` (which
  classifies *referenced* paths, so can't take `-v type`) as a third copy; drift
  reduced to two surfaces, not one. Document + cross-reference + alignment fixture.
- 🔵 **minor / medium** — Phase 2/4: positional `cut -f4`/`-f6` column coupling
  (the same class as the original off-by-one); pin columns by header name.
- 🔵 **minor / medium** — Phase 4: migration sources the evolving
  `frontmatter-emission-rules.sh` from a frozen migration; confirm/snapshot the
  optional-extras set to preserve historical determinism.
- 🔵 **minor / high** — Phase 1/2: env-overridable `SCHEMA_TSV`/
  `DOC_TYPE_INFERENCE` is a sound seam (mirrors `FM_EMISSION_RULES`) but should be
  documented test-only.
- 🔵 **minor / medium** — Phase 1: newly-typeable `pr-description` files enter the
  corpus index and interactive linkage loop (127 files downstream); add a
  linkage-bearing PR-description fixture and confirm duplicate-id behaviour.

### Safety

**Summary**: A dev-tool migration that mutates the user's `meta/` corpus in place
via `atomic_write`, gated by a dirty-tree refusal and recoverable through VCS — a
proportionate posture for the blast radius. The plan correctly preserves the
`set -euo pipefail` abort, the gate ordering, and per-file write atomicity, and
idempotency (`cmp -s`) holds across the new arms. The principal gaps are silent
data-loss/fabrication paths in three new transforms where the only breadcrumb is
a transient first-run stderr line that vanishes once committed.

**Strengths**:
- Explicitly preserves the dirty-tree refusal and the `set -euo pipefail`
  orchestration block as the recovery boundary.
- Idempotency treated as first-class (absence-gated backfill + `cmp -s` write).
- `atomic_write` gives per-file write atomicity.
- Never-overwrite-existing-extras applied consistently with an explicit AC.
- Editing shipped 0007 is safe for cleanly-migrated corpora (recorded applied →
  never re-run).

**Findings**:
- 🟡 **major / high** — Phase 2: `pr_title` silently discarded when a differing
  `title:` exists, no breadcrumb (unlike the Phase 4 sentinel). Emit
  `0007-DIVERGE[discarded-key]`.
- 🟡 **major / high** — Phase 3: unconditional `ticket`/`ticket_id` drop inspects
  no value and logs nothing; a hand-added external-tracker ref is destroyed
  silently. Emit `0007-DIVERGE[dropped-legacy-key]` with key+value.
- 🟡 **major / medium** — Phase 4: fabricated `verdict: "unknown"`/`lenses:
  ["unknown"]` pass validation (verdict is an extra, no vocab check) and read as
  real review state; the only warning is a one-shot stderr line. Make the sentinel
  self-identifying/persistent.
- 🔵 **minor / medium** — Phase 1: widening `out_of_scope` to `meta/docs/`
  permanently and silently excludes anything nested there; narrow the glob and
  fixture a byte-unchanged `meta/docs/` file.
- 🔵 **minor / medium** — batch mutation has no all-or-nothing boundary on
  mid-run abort; signpost VCS-revert recovery in a Manual Verification step / abort
  log line.

### Compatibility

**Summary**: Modifies a shipped migration (0007) and a shared validator that
multiple surfaces consume. The new env-overridable contracts use `:-` defaults so
they are backward compatible, and `pr:N` is correctly tolerated by both the
migration's `LINKAGE_REF_RE` and the validator's `FM_TYPED_REF_RE`. The main risks
are the validator's behavioural broadening (`out_of_scope` now skips `meta/docs/`
for every consumer) and the schema-TSV positional column coupling shared with the
validator.

**Strengths**:
- Env-overridable `SCHEMA_TSV`/`DOC_TYPE_INFERENCE` with `${VAR:-default}` keep
  default paths unchanged, matching the existing `FM_EMISSION_RULES` pattern.
- `pr:N` verified to validate consistently across both regexes and skipped from
  the DANGLING-REF lookup.
- `normalize_pr_ref` strips the `#` (forbidden by the id grammar) before emitting
  `pr:N`.
- Single-sourcing eliminates the byte-identical-copy drift class.
- The runner skips already-applied migrations and old 0007 fails before being
  recorded, so the in-place edit is the correct distribution mechanism.

**Findings**:
- 🟡 **major / high** — Phase 1: `*/docs/*` added to the *shared* `out_of_scope`
  silently stops every validator consumer from validating `meta/docs/` corpus-
  wide — a contract change beyond the migration. Document or scope to the
  migration's enumeration; narrow the glob.
- 🔵 **minor / high** — Phase 2/4: `cut -f6`/`cut -f4` hard-code positional
  columns shared with the validator's positional parse; pin by header name or add
  a column-order assertion.
- 🔵 **minor / medium** — Migration Notes: complementarity proven only for the
  same-session two-migration path; the common cross-session (0001 pre-applied,
  only 0007 now) path is unverified. Add that case.
- 🔵 **minor / medium** — Phase 1/4: migration resolves helpers via `PLUGIN_ROOT`,
  validator via `SCRIPT_DIR`; note they must resolve to the same tree and the env
  overrides are test-only.

### Standards

**Summary**: Unusually disciplined about conventions: correct kebab-case for the
new helper, single-sourcing through the established channels, consistent
`0007-DIVERGE[...]` breadcrumb format and awk style, and an accurate schema TSV
column-order. The proposed `# shellcheck source=` directive is correct given the
repo's `source-path=SCRIPTDIR` rc, and the new shell respects the bash 3.2 floor.
The observations are minor consistency nits rather than convention breaks.

**Strengths**:
- New `doc-type-inference.sh` follows shared-helper conventions (kebab-case,
  shebang, header comment, pure no-side-effect functions).
- The validator's `DOC_TYPE_INFERENCE=…` + `# shellcheck source=` mirrors the
  existing `FM_EMISSION_RULES` line and resolves under the repo rc.
- Schema TSV column-order documentation matches the header byte-for-byte; `cut`
  reads are column-accurate.
- Breadcrumb format and new awk helpers modelled on existing in-file style.
- All new shell honours the bash 3.2 floor.

**Findings**:
- 🔵 **minor / medium** — Phase 4: the buggy `extras_for_type` neighbours
  (`status_vocab_for_type` `:53`, plus the `# 5th col` comment) are left in the
  pre-fix muddle right beside the corrected line; collapse/clean them.
- 🔵 **minor / medium** — Phase 1: confirm `doc-type-inference.sh` lints clean
  under `.shellcheckrc enable=all`; add a top-of-file directive only if needed.
- 🔵 **suggestion / high** — Phase 1, Section 3: update the validator's top-of-file
  comment and the `out_of_scope` note ("specs/talks/global") to mention the new
  helper and widened skip set, or it drifts out of date.
- 🔵 **suggestion / low** — Phase 4, Section 2: cite/align the `lenses:
  ["unknown"]` flow-list spelling with the `tags: []` precedent and templates.

## Re-Review (Pass 2) — 2026-06-17T23:20:14+00:00

**Verdict:** REVISE

The pass-1 findings are substantially addressed — the critical unguarded-grep
abort and all the silent data-loss/fabrication paths are resolved, and the
test-fidelity, single-sourcing, and scope concerns landed well. **But the two
biggest revisions (the packed `backfill_extras` channel and the required-extras
backfill threading) introduced two new high-confidence defects that re-arm the
exact permanent-stall this plan exists to fix**, plus a cluster of new majors
around the packed-channel separator, the `pr_number` fallback, and cross-phase
fixture completeness. Net: most of pass 1 is closed, but the plan needs one more
iteration before it is implementation-ready.

### Previously Identified Issues

- 🔴 **Correctness/Code-Quality** — unguarded `grep` aborts under `pipefail` —
  **Resolved** (`|| true` guards + explicit "digit-less stem does not abort" AC;
  confirmed by correctness, code-quality, safety).
- 🟡 **Correctness** — `pr_number` mis-parses date-prefixed stems — **Partially
  resolved**: the pr-token case is fixed, but the leading-number *fallback* still
  grabs the year for a date-prefixed, pr-token-less stem (see new findings).
- 🟡 **Correctness** — `pr_title` empty-fold emits `title: ""` — **Resolved**
  (`!is_empty_val(val)` guard + empty-fold AC).
- 🟡 **Safety** — `pr_title` silently discarded — **Resolved**
  (`0007-DIVERGE[discarded-key]`, asserted).
- 🟡 **Safety** — unconditional `ticket`/`ticket_id` drop silent — **Resolved**
  (`0007-DIVERGE[dropped-legacy-key]` on non-empty, asserted).
- 🟡 **Safety/Test-Coverage** — sentinel fabrication unaudited — **Resolved**
  (neutral value retained per decision + asserted breadcrumb; residual minor on
  in-corpus durability, below).
- 🟡 **Test-Coverage** — fail-first not asserted — **Resolved** (Assertion
  Discipline + per-phase `assert_violation` red-steps; residual: red-step
  *mechanism* under-specified, below).
- 🟡 **Test-Coverage** — idempotency assumed — **Resolved** (per-phase empty-diff
  re-run ACs).
- 🟡 **Test-Coverage** — Phase 5 thin coverage — **Resolved** (list/multi-token/
  no-re-grab fixtures; residual minor: lowercase/`PR#N` spellings, below).
- 🟡 **Architecture** — single-sourcing incomplete — **Resolved** (third copy
  documented + alignment fixture; residual minor: fixture covers only the one new
  arm, below).
- 🟡 **Compatibility/Safety** — `*/docs/*` too broad — **Resolved** (narrowed to
  `*/meta/docs/*`, documented; acknowledged residual: still a validator-wide
  behaviour change for all consumers).
- 🟡 **Code-Quality** — `-v` threading wall — **Resolved** via the packed channel
  (which, however, introduced the separator fragility below).
- 🔵 Minors (dead `status_vocab_for_type`, validator header comment, `lenses`
  flow-list spelling, shellcheck directive, drop-arm placement, `topic`
  quote-strip, helper extraction) — **Resolved**.

### New Issues Introduced

- 🔴 **Correctness (new, high)** — *Empty-placeholder extras dropped but not
  backfilled.* The Phase 4 builder skips an extra when `fm_get` is non-empty, but
  `lenses: []` / `verdict: ""` return non-empty strings → skipped from backfill →
  then dropped by the awk omit-when-empty arm (`:301`) → `MISSING-EXTRA`. The
  presence check must use `is_empty_val` semantics (treat `""`/`[]` as absent).
  *Location: Phase 4, Change 1 (builder loop).*
- 🔴 **Correctness (new, high) / 🟡 Safety (new)** — *`topic` backfill derives
  from `title_default`, which is empty when the file already has a `title:`.* For
  a fenced research/note file that HAS a title but lacks `topic` (the common
  shape), `title_default` is `""` (migration `:387-390` only computes it when
  `has_title==0`), so `extra_default topic` returns empty and `topic` is never
  backfilled → `MISSING-EXTRA`, gate fails, partial-mutation stall persists. The
  Phase 4 AC "topic backfilled from title" cannot pass for an otherwise-complete
  (titled) fixture. Derive the topic default from the file's actual title
  (`fm_get title`/H1), not `title_default`. *Location: Phase 4, Change 1.*
- 🟡 **Correctness/Safety/Architecture/Compatibility/Code-Quality (new)** —
  *Packed `name=value;` channel silently truncates a value containing `;`.* The
  `topic` value is an arbitrary user H1; a `;`-bearing title splits mid-value
  (the fragment without `=` is dropped), emitting a truncated `topic:` that still
  validates — silent corruption with no breadcrumb, and a possible downstream YAML
  break. Use a separator that cannot appear in a single-line scalar (tab/RS under
  `LC_ALL=C`) or strip/breadcrumb `;` in the builder. *Location: Phase 4,
  Resolved decisions (Backfill threading) + Change 1/2.*
- 🟡 **Safety/Correctness (new)** — *`pr_number` leading-number fallback fabricates
  a year.* When no `pr` token is present, the fallback `grep -oE '^[0-9]+'` grabs
  the date component of a date-prefixed stem (e.g. `2026-06-17-summary` →
  `pr_number: 2026`), with no breadcrumb because a value WAS derived. Skip a
  `^\d{4}-\d{2}-\d{2}` prefix before the leading-number fallback so a date-only
  stem yields no `pr_number` + the DIVERGE breadcrumb. *Location: Phase 4,
  Resolved decisions (pr_number).*
- 🟡 **Test-Coverage/Architecture (new)** — *Phase 1 pr-description fixture cannot
  validate clean before Phase 4.* `pr-description` requires `pr_number` (TSV col
  4; not optional), backfilled only in Phase 4, so the Phase 1 "validates clean"
  AC fails in an isolated Phase-1 build. Author the fixture otherwise-complete
  (carry `pr_number`/provenance) per the plan's own rule. *Location: Phase 1,
  Success Criteria.*
- 🟡 **Test-Coverage (new)** — *Red-step `assert_violation` mechanism
  under-specified.* Running it via `run_0007` (which mutates then aborts) vs the
  standalone validator on the pre-seeded fixture yields different codes; specify
  the standalone-validator path once in the Assertion Discipline section.
  *Location: Implementation Approach, Assertion discipline.*
- 🟡 **Architecture (new)** — *Header assertion guards the migration but not the
  validator's parallel positional parse* (`validate-corpus-frontmatter.sh:56`).
  A reshape would still silently skew the validator. Put the guard in the shared
  layer and invoke it from both. *Location: Phase 2, Change 1.*
- 🟡 **Architecture (new)** — *Frozen-migration coupling to evolving
  `FM_OPTIONAL_EXTRAS` rests on an unenforced assumption.* Snapshot the
  optional-extra names 0007 depends on locally, or add a regression fixture that
  fails if the required-extra set for the touched types ever changes. *Location:
  Phase 4, Change 1.*
- 🟡 **Test-Coverage (new)** — *New-type fixtures (pr-review/pr-description) not
  verified to survive `precondition_prepass`* (duplicate post-rewrite-id REFUSE
  across the broadened namespace). Add a coexistence AC. *Location: Phase 2/4.*
- 🔵 **Standards (new, high)** — *Raw literal TAB embedded in the
  `assert_schema_columns` case pattern.* The codebase universally uses `$'\t'` /
  `-F'\t'`; a raw tab is invisible and editor-fragile. Reconstruct via `$'\t'`.
  *Location: Phase 2, Change 1.*
- 🔵 **Standards (new, high)** — *`grep -oiE` case-insensitive flag used nowhere
  else.* Stems are lowercase by convention; use plain `grep -oE` (widen the
  pattern explicitly if upper-case `PR-` must be tolerated). *Location: Phase 4,
  Change 1.*
- 🔵 **Correctness/Compatibility (new, minor)** — PR-ref coercion still misses
  lowercase `pr #N`, space-less `PR#N`, and `PR-N` (no `#`); the
  `assert_schema_columns` exact-match is stricter than the validator about
  additive schema columns; the awk `path_to_typed` alignment fixture covers only
  the one new arm. *Location: Phase 5 / Phase 2 / Phase 1.*
- 🔵 **Safety (new, minor)** — the abort-log VCS-revert recovery hint is narrated
  in Migration Notes but not assigned to a concrete code change at the migration's
  abort points. *Location: Migration Notes / Phase abort sites.*

### Assessment

The plan converged strongly on the pass-1 concerns, but two new **critical**
correctness bugs (empty-placeholder backfill skip; `topic` from an empty
`title_default`) both re-create the `MISSING-EXTRA` → gate-abort → partial-mutation
stall the plan exists to eliminate, and both would slip past the currently-drafted
"otherwise-complete" fixtures. These plus the packed-channel separator corruption
and the `pr_number` year-fallback are concrete, fixable defects in the new
material. One more iteration — fixing the two criticals and the packed-channel /
pr_number majors, then re-asserting with empty-placeholder and `;`-bearing-title
fixtures — should bring the plan to APPROVE.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-06-17T23:54:18+00:00

**Verdict:** REVISE (converging — refinements only; no criticals)

Both pass-2 criticals are **genuinely fixed** (verified against source): the
empty-placeholder presence check now treats `""`/`[]` as absent so they route to
backfill instead of being dropped, and `topic` now derives from the file's actual
title rather than the always-empty-when-titled `title_default`. The packed-channel
`;`-corruption and the `pr_number` year-fabrication are also resolved. What
remains is a tail of **edge-refinements** — three real majors (one test-feasibility
wording issue, two compatibility edge-cases) plus minors. The plan is close to
APPROVE; this pass found no blocker of the severity of the prior two rounds.

### Previously Identified Issues (pass 2)

- 🔴 Empty-placeholder dropped-not-backfilled — **Resolved** (`is_empty_val`
  presence check; verified `fm_get` returns literal `[]`/`""`).
- 🔴 `topic` from empty `title_default` — **Resolved** (derives from actual
  `title:`, `title_default` fallback).
- 🟡 Packed-channel `;` truncation — **Resolved** (US `\037` separator +
  value sanitisation + `;`-title fixture).
- 🟡 `pr_number` year fabrication — **Resolved** (date-strip fallback +
  micro-assertion); residual minor below on the post-strip leading-number path.
- 🟡 Phase 1 fixture can't validate pre-Phase-4 — **Resolved** (authored
  otherwise-complete with `pr_number`).
- 🟡 Red-step mechanism under-specified — **Resolved** (standalone validator on
  pre-seeded files); residual minor: pin file-list mode explicitly.
- 🟡 Header assertion one-sided — **Resolved** (shared, invoked by both); but see
  new compat major on its strictness.
- 🟡 Frozen-migration assumption unenforced — **Resolved** (contract-guard AC).
- 🟡 Prepass coexistence — **Resolved** (AC added); residual minor: assert the
  actual collision/non-collision, not just "reaches run_rewrite".
- 🔵 Standards (raw TAB, `grep -oiE`), alignment fixture, abort-log hint —
  **Resolved**.

### New / Residual Issues

- 🟡 **Test-Coverage (new, high)** — *Packed-channel parser probe is not feasible
  as specified.* The AC promises "an awk-level probe in the style of the frag.awk
  parity probe," but the parse/emit loop is **inline** in the closing-fence
  pattern-action block, not a callable function a `BEGIN{}` probe can invoke. Fix:
  extract it into an awk function (`emit_backfill_extras(packed)`) so the
  empty/single-record/`=`-in-value cases are probeable, or restate the AC as a
  full-corpus content assertion. *Phase 4 SC.*
- 🟡 **Compatibility/Architecture (new, medium)** — *Header assertion is stricter
  than the validator's surplus-tolerant `read`.* Invoking the exact-match
  `assert_schema_columns` inside the validator newly aborts any consumer that
  passes a column-**extended** `SCHEMA_TSV` (a config the validator's `read`
  previously tolerated). Fix: make it a **prefix** check on the columns the
  positional readers actually use, or document the narrowed validator-input
  contract in the validator header (not only the migration note). *Phase 2,
  Change 1.*
- 🟡 **Compatibility (new, medium)** — *US control byte through `awk -v` /
  `split("\037")` on an unpinned awk.* `awk` is not pinned in `mise.toml` (BSD awk
  on macOS, gawk/mawk on Linux); the codebase's existing US-channel idiom is
  parsed shell-side via `IFS`, never through awk `-v`/`split`. The path is sound
  in theory (raw bytes survive argv; `"\037"` is a POSIX octal escape) but
  unproven on BSD awk. Fix: run the parser-probe AC under the actual system awk on
  **both** macOS and Linux CI. *Phase 4 + Resolved decisions (Backfill
  threading).*
- 🔵 **Correctness (new, medium)** — `extra_default` pr-token grep
  `[Pp][Rr]-?[0-9]+` matches the `pr` inside a word like `expr-3`/`improve-2`.
  Anchor to a segment boundary: `(^|-)[Pp][Rr]-?[0-9]+`. *Phase 4.*
- 🔵 **Safety (new, medium)** — the `pr_number` **leading-number fallback** still
  silently adopts a non-PR leading number after the date-strip (e.g.
  `2026-06-17-0114-foo` → `0114`) with no breadcrumb. Better: only apply the
  fallback when the stem is **not** date-prefixed (else emit nothing + breadcrumb).
  *Phase 4.*
- 🔵 **Architecture/Standards/Code-Quality (new, medium; 3 lenses)** —
  `assert_schema_columns` (a TSV-column-contract guard) is housed in
  `doc-type-inference.sh` (path-classification helper). Move it to
  `frontmatter-emission-rules.sh`, which already self-declares as the cross-cutting
  schema-rules single source and is sourced by both surfaces. *Phase 2.*
- 🔵 **Standards (new, high)** — shell-side separator spelled `$'\037'` (octal)
  where the codebase's three existing US-channel sites use `$'\x1F'` (hex). Use
  `$'\x1F'` shell-side; keep the awk `"\037"` octal (awk has no `\x`). *Phase 4.*
- 🔵 **Code-Quality (new, medium)** — the `is_empty_val` placeholder predicate is
  now open-coded inline in shell, duplicating the awk's `is_empty_val` and the
  validator's EMPTY-PLACEHOLDER rule (a third copy). Extract a shell helper.
  *Phase 4.*
- 🔵 **Safety (new, low)** — `${dv//$US/}` silently strips a US byte with no
  breadcrumb (inconsistent with the plan's every-mutation-breadcrumbed discipline);
  document it as a parser-safety no-op or breadcrumb on change. *Phase 4.*
- 🔵 **Correctness/Safety (new, low)** — emit backfilled scalar extras via
  `fm_normalise_value` rather than hand-wrapping `": \"" bv "\""`, so a
  backslash/indicator-leading title is escaped consistently. *Phase 4.*
- 🔵 **Test-Coverage (new, minors)** — add fixtures for: a populated multi-element
  `lenses` left unchanged (no sentinel clobber); `pr_title` equal to an existing
  `title` (breadcrumb-firing boundary); the actual prepass collision/non-collision;
  and the table-driven alignment fixture asserting the full `doc-type:id` (incl.
  id-derivation), not only the type. *Phases 1/2/4/6.*
- 🔵 **Compatibility (new, minor)** — list-vs-scalar cardinality is hard-coded
  (`lenses` only) in the emit loop and not guarded by `assert_schema_columns`; note
  it and add a flow-list round-trip AC. *Phase 4.*
- ✅ **Correctness "major" (refuted)** — the claim that `normalize_pr_ref`
  corrupts an embedded `pr-NNN` inside a typed `"pr-review:…-pr-416-review"` ref is
  a **false positive**: the regex requires both delimiting quotes, so the
  quote-less embedded token cannot match. No change needed (a mixed-list fixture is
  cheap insurance and is being added anyway).

### Assessment

The plan has converged hard: criticals are gone, the two pass-2 regressions are
verified fixed, and the remaining items are refinements (a test-AC feasibility
wording, two awk/schema compatibility edge-cases on the new packed channel, and a
cluster of minors). None is a structural blocker. After one more light pass —
making the parser loop probeable, softening the header assertion to a prefix check
(or documenting the contract), proving the US channel on macOS+Linux CI, and the
pr-token/fallback/helper-placement minors — the plan should reach APPROVE.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 4) — 2026-06-18T00:05:26+00:00

**Verdict:** APPROVE

The plan is implementation-ready. **Six of seven lenses returned zero findings**
(correctness, code-quality, test-coverage, architecture, safety, compatibility —
each at explicit APPROVE-level confidence after tracing the latest snippets
against the source); standards returned **two trivial cosmetic nits**, both folded
in during this pass. Every pass-3 fix is verified present and correct.

### Previously Identified Issues (pass 3)

- 🟡 Parser-probe not feasible — **Resolved** (`emit_backfill_extras` extracted as
  a callable awk function; probe runs on macOS+Linux CI).
- 🟡 Header assertion too strict — **Resolved** (prefix check tolerating trailing
  extension; column-extension-accepted AC added).
- 🟡 US-byte awk portability — **Resolved** (cross-OS CI probe; octal `"\037"` is
  the portable awk spelling, shell `$'\x1F'`).
- 🔵 pr-token boundary (`expr-3`) — **Resolved** (`(^|-)` anchor + micro-assertion).
- 🔵 `pr_number` fallback fabrication — **Resolved** (date-prefixed stems skip the
  fallback + `missing-extra-no-default` breadcrumb).
- 🔵 `assert_schema_columns` placement — **Resolved** (relocated to
  `frontmatter-emission-rules.sh`; `doc-type-inference.sh` back to single
  responsibility).
- 🔵 `$'\037'` → `$'\x1F'` shell spelling — **Resolved**.
- 🔵 inline `is_empty_val` copy — **Resolved** (`fm_is_empty_val` extracted).
- 🔵 scalar emission — **Resolved** (`fm_normalise_value`).
- 🔵 fixtures (multi-element `lenses`, equal-value `pr_title`, real prepass
  collision, full `doc-type:id` alignment) — **Resolved**.
- ✅ `normalize_pr_ref` over-match — **Confirmed refuted** by the correctness lens
  (regex requires both delimiting quotes); mixed-list insurance fixture added.

### New Issues Introduced

- 🔵 **Standards (minor)** — awk separator spelled octal `"\037"` vs the
  codebase's one existing `"\x1f"` awk site. **Folded in**: added an inline
  `# octal US == shell $'\x1F'` cross-reference comment (octal is the more
  portable awk spelling, so kept).
- 🔵 **Standards (minor)** — the `expected=$'…'` schema-header literal exceeded the
  80-col floor. **Folded in**: split into two `expected=`/`expected+=` appends
  (bash 3.2-safe) that fit within 80 cols.

### Assessment

All four rounds' findings are resolved: pass 1's critical + 11 majors, pass 2's
two regressions, pass 3's compatibility/feasibility refinements, and pass 4's two
cosmetic nits. The lenses independently verified the logic (pr_number boundary
traces, idempotency, US-byte encoding parity, prefix-check correctness, breadcrumb
coverage, never-overwrite invariant) against the source. The plan is sound,
schema-driven, well-tested, and safe for an in-place corpus migration. **Ready for
`/implement-plan`.** Remaining risk is execution-time (the ACs landing green), not
plan design.

---
*Re-review generated by /accelerator:review-plan*
