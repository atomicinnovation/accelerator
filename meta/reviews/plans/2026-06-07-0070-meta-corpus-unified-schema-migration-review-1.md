---
type: plan-review
id: "2026-06-07-0070-meta-corpus-unified-schema-migration-review-1"
title: "Plan Review: Ship meta/ Corpus Unified-Schema Migration"
date: "2026-06-07T09:35:49+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-07-0070-meta-corpus-unified-schema-migration"
parent: "plan:2026-06-07-0070-meta-corpus-unified-schema-migration"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: [architecture, code-quality, test-coverage, correctness, safety, database, compatibility, portability]
review_number: 1
review_pass: 6
tags: [migration, frontmatter, schema, interactive, visualiser, linkage, review]
last_updated: "2026-06-08T00:12:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Ship `meta/` Corpus Unified-Schema Migration

**Verdict:** REVISE

The plan is genuinely strong on strategy: the phase decomposition respects the
dependency graph, the validator and linkage parser are correctly carved out as
standalone tested components ahead of the migration that consumes them, the
indivisible-XL justification is explicit rather than hand-waved, and it inherits
a proven per-mutation safety substrate (atomic temp-then-rename, layered
idempotency, write-ahead-log invariant, clean-tree pre-flight). But it carries
**ten critical findings** that would cause the dogfood to fail or silently
corrupt the corpus as written: the `type:`-from-location inference is wrong for
several real corpus locations (design-gaps/design-inventories, the
subdirectory-split reviews, the one already-fenced note), the
mechanical-pass-before-`harness_run` ordering creates an unhandled partial-apply
half-state, no referential-integrity check exists for emitted linkages, the
Phase 5 `indexer.rs` removal would break the identity of the very work-items the
migration produces, the migrate-on-use guarantee underpinning the fallback
removal is not actually enforced, the binding acceptance gate is a
non-reproducible manual measurement, and the spike-mandated `\b` regexes are
non-portable on the macOS toolchain. None are unfixable, but several are
factual errors about the corpus or the consuming code that must be corrected
before implementation.

### Cross-Cutting Themes

- **`type:` discriminator inference from directory layout is under-specified and
  factually wrong in places** (flagged by: correctness, architecture) — the rule
  omits the `design-gaps/`/`design-inventories/` research subdirs, mis-frames
  reviews as filename-classified when they are subdirectory-split
  (`reviews/plans/` vs `reviews/work/`), and ignores files that already carry an
  explicit `type:`. The validator reads `type:` from frontmatter post-migration
  and so cannot catch a *wrong* inferred type, only a missing one.

- **The mechanical pass running to completion before `harness_run` is the single
  riskiest design decision** (flagged by: safety, database, correctness) — an
  interactive abort leaves the whole corpus rewritten with the ledger unrecorded
  and no whole-or-nothing guarantee; partial-apply convergence on re-run over a
  half-written corpus is unproven; and any stdout the mechanical pass emits is
  parsed by the runner as interactive protocol frames before `READY`.

- **"Single-sourced from `templates-schema.tsv`" overstates reality** (flagged
  by: architecture, code-quality, database) — the TSV encodes per-type tabular
  facts (extras, status vocab, code_state_anchored, forbidden own-id) but *not*
  the base-field set, the quoted-`id:` rule, `schema_version: 1`, omit-when-empty,
  or the `"doc-type:id"` value shape. Those rules end up duplicated across the
  new validator, the existing template-contract test, and the migration's awk —
  the classic schema-drift surface the plan claims to avoid.

- **Phase 5's fallback removal can break corpora it was supposed to be safe for**
  (flagged by: compatibility, safety) — removing the `indexer.rs` filename-id
  fallback strips the only working identity source for migrated work-items
  (which carry `id:`, not the dead `work_item_id:` the code still reads); the
  migrate-on-use contract that justifies same-release removal is advisory, not
  enforced; and `cluster_key.rs` clustering now depends on `parent:` being fully
  populated where it previously fell back to the retained foreign `work_item_id:`.

- **The skip-as-terminal semantics weaken the dogfood gate** (flagged by:
  test-coverage, correctness, database, compatibility) — a skipped ambiguous
  reference reaches `APPLIED_CONFIRM` with no mutation, so AC-9's terminal-state
  gate is satisfiable with zero linkage written, the apply path is never
  exercised against real data, and clustering can be orphaned by skipped edges.

- **Idempotency's "three planes" conflate ledger-skip with verified no-op**
  (flagged by: correctness, database, test-coverage) — a dogfood re-run via
  `/accelerator:migrate` skips 0007 at the ledger entirely; the `cmp -s` and
  resume planes are only exercised by the fixture test that re-invokes the script
  directly. "No changes" on the dogfood re-run is true for the wrong reason.

### Tradeoff Analysis

- **Cohesion vs framework constraint (Phase 3 monolith)**: Code Quality flags the
  single migration script holding awk rewrite + backfill + interactive hooks as a
  god-object; Architecture agrees the cohesion is poor but attributes it to a real
  framework constraint (one migration = one ledger ID = one dogfood gate). These
  reconcile: keep the single *entry point* for ledger semantics, but factor the
  three concerns into separately-sourced helpers/functions so packaging-as-one-file
  doesn't become one-blob internal coupling.

- **Same-release Rust removal: correctness vs fail-soft**: Safety notes the
  removal eliminates graceful degradation before the migration's reliability is
  proven beyond one corpus; Compatibility confirms the removal is *correct* (the
  fallback is unsafe once legacy keys are gone) but unsafe to ship without
  enforcing migrate-on-use. The recommendation is not to split Phase 5 but to add
  a deliberately-un-migrated-corpus test proving the fallback-removed code fails
  *loudly and recoverably*, and to gate or warn on the 0007 ledger entry.

### Findings

#### Critical

- 🔴 **Correctness**: `type:`-from-location omits design-gaps/design-inventories research subdirs and mis-handles already-typed files
  **Location**: Current State Analysis / Phase 3 §1 (corpus walker)
  The rule maps "research subdirs → codebase-research/issue-research" but the live
  corpus also has `meta/research/design-gaps/` and `meta/research/design-inventories/`,
  whose files already carry explicit `type:` (with `git_commit`/`branch` to drop).
  These get the wrong type or hit an unhandled branch, failing AC-1's clean-validation gate.

- 🔴 **Correctness**: reviews are classified by subdirectory, not filename
  **Location**: Current State Analysis ("`meta/reviews/` by filename→`*-review`")
  Reviews live in `meta/reviews/plans/` (→`plan-review`) and `meta/reviews/work/`
  (→`work-item-review`); filenames carry only `-review-N` and cannot distinguish
  the two. A filename discriminator mis-types ~131 review files, producing wrong
  per-type extras.

- 🔴 **Correctness**: the lone partial note already has a fence and escapes the no-fence backfill gate
  **Location**: Phase 3 §3 (backfill for the 32 no-fence files)
  `meta/notes/2026-04-17-security-lens-owasp-ai-top-10.md` already has a `---`
  block (with `status: draft`). Keyed on "no leading `---` fence", it is never
  backfilled — it misses `type: note`/note extras and keeps out-of-vocab
  `status: draft`, a guaranteed validator failure. Route notes by location, not
  by absence-of-fence. (The "14 notes have no fence" count is off by one.)

- 🔴 **Safety**: interactive abort after the mechanical pass leaves the whole corpus rewritten with the ledger unrecorded
  **Location**: Phase 3 §1 ("runs the awk rewrite + backfill … before `harness_run`")
  The bulk mutation completes before the interactive phase; a `FAIL` exits
  non-zero and the runner does not record 0007, leaving migrated frontmatter on
  disk with the ledger saying "pending" and no integrity check forcing the
  required `jj/git revert`. Add a post-run self-validation gate and make revert an
  explicit checklist step.

- 🔴 **Database**: no referential-integrity check that emitted typed linkages point at existing artifacts
  **Location**: Phase 2 (parser) / Phase 3 §4 (`migration_apply_decision`)
  The migration writes `"doc-type:id"` foreign references inferred from prose, but
  nothing verifies the target file exists — the validator checks only the *form*,
  not resolution. The migration can mechanically write dangling references,
  silently corrupting the artifact graph the downstream visualiser epic depends on.

- 🔴 **Database**: partial-apply recovery (non-zero exit mid-corpus) is unaddressed
  **Location**: Phase 3 §1 / Migration Notes (no_op_pending)
  If the awk pass mutates N files then exits non-zero, the corpus is half-rewritten
  while the ledger shows pending. A re-run is only safe if the mechanical pass is
  independently *convergent* over a half-written corpus — asserted nowhere. Prove
  convergence with an interrupt-then-re-run fixture, and document revert-before-rerun.

- 🔴 **Compatibility**: removing the `indexer.rs` filename-id fallback breaks identity of the work-items the migration just produced
  **Location**: Phase 5 §3
  Post-migration work-items carry `id:`, not `work_item_id:`, but `indexer.rs:1216-1233`
  reads only `work_item_id` then falls back to filename. Dropping the filename
  fallback leaves migrated work-items with *no* identity source, collapsing the
  work-item graph. Retarget the frontmatter read to the unified `id:` key before
  removing the fallback.

- 🔴 **Compatibility**: the migrate-on-use guarantee underpinning same-release fallback removal is not enforced
  **Location**: Migration Notes (cross-repo coupling) / Phase 5 Overview
  `migrate/SKILL.md` shows migration is manually-invoked and advisory ("Skills do
  not gate themselves on pending migrations"). A dormant repo can upgrade the
  visualiser without running 0007, getting silently broken cross-references. Gate
  the read path on the 0007 ledger entry, keep the fallback one release longer
  behind a deprecation warning, or document the hard upgrade requirement.

- 🔴 **Test Coverage**: the binding acceptance gate is a manual, non-reproducible measurement with no automated correctness check behind the mechanical path
  **Location**: Phase 4 §2 (resolved-band wrong-rate, AC-8) / Manual Verification
  The ≤5% wrong-rate over a ≥150 sample is hand-classified, with no fixed seed and
  no automated test that resolved references emit the *right target* (Phase 2
  fixtures check only the band). A parser regression flipping correct→wrong on the
  highest-volume, unaudited path would pass every automated test.

- 🔴 **Portability**: the spike-mandated `\b` word-boundary regexes are not portable across BSD awk / BSD grep
  **Location**: Phase 2 §1 (the three spike fixes)
  `\bblocks?\b` / `\bsibling\b` "must work identically everywhere", but BSD awk and
  BSD `grep -E` (macOS defaults) do not honour `\b` — only PCRE does. The same
  pattern classifies references differently per platform, changing which apply
  mechanically. Emulate boundaries with explicit character classes
  (`(^|[^[:alnum:]])…`) as `lint-bashisms.sh` already does.

#### Major

- 🟡 **Correctness**: the mechanical pass's stdout will be parsed as interactive protocol frames on the FIFO
  **Location**: Phase 3 §1
  For an interactive migration the runner parses every stdout line as a TSV frame;
  the first legitimate frame is `READY` inside `harness_run`. 0006's reused walker
  echoes progress to stdout — those bytes arrive before `READY` and corrupt the
  handshake. Redirect all pre-`harness_run` output to stderr; add a fixture
  asserting no non-frame bytes precede `READY`.

- 🟡 **Correctness**: awk `MALFORMED`-on-no-fence vs the deliberate no-fence backfill needs an explicit per-file gate
  **Location**: Phase 3 §2 / §3
  0006 emits `MALFORMED` when a target key appears in a fence-less file; the
  migration deliberately backfills 32 fence-less files. Without a mutually
  exclusive per-file dispatch, a no-fence file containing a matched token trips
  `MALFORMED` (failing AC-1's zero-`MALFORMED` gate) while also being backfilled.

- 🟡 **Correctness / Database / Test Coverage**: the ledger plane makes a dogfood re-run skip 0007 entirely, not re-run it as a verified no-op
  **Location**: Desired End State / Phase 4 ("immediate re-run reports no changes")
  On first success the runner records 0007 applied; a re-run never executes it, so
  `cmp -s`/resume idempotency is unexercised on the dogfood. State that genuine
  three-plane no-op is proven by the fixture test that re-invokes the script
  directly (ledger bypassed); keep the dogfood re-run as the ledger-skip check.

- 🟡 **Safety**: `REFUSE`/`MALFORMED` diagnostics do not abort, yielding silent per-file partial migration
  **Location**: Phase 3 §2 / Desired End State ("zero REFUSE/MALFORMED")
  Following 0006, a `REFUSE` prints the line unchanged and continues; the runner
  treats stderr diagnostics as non-fatal. A refused key keeps its legacy form
  while the rest of the file is rewritten, and "zero REFUSE/MALFORMED" is required
  but not *enforced*. Count occurrences and exit non-zero (or emit `no_op_pending`),
  or gate Phase 4 on a programmatic grep for zero `0007-REFUSE`/`0007-MALFORMED`.

- 🟡 **Safety**: same-release Rust removal eliminates graceful degradation before reliability is proven beyond one corpus
  **Location**: Phase 5 ordering rationale / Migration Notes
  Not a request to split Phase 5, but add a test exercising the fallback-removed
  code against a *deliberately un-migrated* fixture corpus to confirm it fails
  loudly and recoverably, and assert the degradation mode is acceptable.

- 🟡 **Database**: 0007 depends on 0005/0006 being *applied*, but `sort -z` only guarantees they sort first
  **Location**: Migration Notes (runtime ordering)
  A repo with a hand-edited/lost ledger could have 0007 pending while 0005/0006
  are unapplied; the runner then runs all three in order, but 0007's awk assumes
  the precondition is already satisfied. Make preconditions defensive — REFUSE
  (not silently mis-handle) a file with unquoted foreign `work_item_id:` or a
  work-item missing `kind:`.

- 🟡 **Database**: canonical-side-only linkage writes leave the reverse relation unmaterialised with no integrity guarantee
  **Location**: Phase 3 §4 / Desired End State
  The TSV lists `blocked_by` as a first-class work-item linkage key; the plan
  doesn't say what happens to a pre-existing reverse-side value. State the rule
  (drop, or verify it agrees with the canonical edge) and add a validator
  consistency check — or confirm no artifact carries an explicit reverse side today.

- 🟡 **Test Coverage**: AC-7's terminal-state gate is satisfiable by skips, so the dogfood does not prove the apply path
  **Location**: Phase 3 §5 / Phase 4 §3
  Every ambiguous reference could be skipped, AC-9 stays green, and zero linkage is
  written — the apply/edit code never runs against real data. Add a Phase 4
  verification that the session log contains some `accepted`/`edited` outcomes, or
  require driving a representative subset to an applied terminal.

- 🟡 **Test Coverage**: new shell suites are wired into the wrong test task and the migrate suite-count guard is not updated
  **Location**: Phase 1 §3 / Phase 2 / Testing Strategy
  The plan says add tests to `tasks/test/unit.py`, but shell `test-*.sh` suites are
  auto-discovered by the *integration* tasks (`test:integration:config` for
  `scripts/`, `test:integration:migrate` for `skills/config/migrate/`), and
  `integration.py` hard-asserts `_EXPECTED_MIGRATE_SUITES = 3` — adding a migrate
  suite without bumping it fails the build. Place suites under the globbed
  subtree, ensure the exec bit, and update the constant.

- 🟡 **Test Coverage**: idempotency no-op is asserted only on a fixture corpus, not proven against the real dogfood corpus
  **Location**: Phase 3 §5 / Phase 4
  The irregular real files (32 no-fence, the `status: draft` note, ADR-0033's
  `adr_id:`) are where non-idempotency hides, yet real-corpus proof is one manual
  eyeball. Make the Phase 4 re-run an automated assertion of an empty VCS diff,
  and add fixtures mirroring the known-irregular files.

- 🟡 **Architecture**: the validator single-sources from `templates-schema.tsv`, but Phase 5 mutates a TSV row it depends on
  **Location**: Phase 1 §1 / Phase 5 §4
  The `work-item-review` row carries `work_item_id` as an extra; Phase 5 drops it.
  The Phase 1 validator and Phase 4 dogfood depend on the row while Phase 5 edits
  it. State that extras are validated as optional (mutation-neutral), or sequence
  the TSV edit so validator and dogfood see one stable schema.

- 🟡 **Architecture / Code Quality / Database**: schema rules are split across the TSV, the validator, the contract test, and the awk
  **Location**: Phase 1 §1 ("single-sourced from `templates-schema.tsv`")
  Base-field set, quoted-`id:`, `schema_version: 1`, omit-when-empty, and the
  `"doc-type:id"` shape have no TSV column and end up duplicated. Reframe the
  single-source claim (TSV = tabular per-type facts only) and factor the
  cross-cutting rules into one shared helper consumed by both the validator and
  the template-contract test.

- 🟡 **Architecture**: intra-migration ordering between the awk rewrite and the no-fence backfill is unspecified but load-bearing
  **Location**: Phase 3 §2 / §3
  Three writers touch the same files; the order between the awk pass and the
  backfill is not fixed. Specify the explicit sequence (backfill → awk normalise →
  `harness_run`) and whether backfilled files re-enter the awk pass.

- 🟡 **Code Quality**: Phase 3 migration is a god-object with no proposed internal structure
  **Location**: Phase 3 (whole) / Implementation Approach
  Indivisibility justifies shipping as one migration, not organising one file.
  Factor the rewrite, backfill, and hook bodies into separately-sourced helpers
  with tight contracts.

- 🟡 **Code Quality**: writing typed linkage into existing YAML frontmatter is under-specified versus the body-append precedent
  **Location**: Phase 3 §4 / §2
  The 0099 example appends to the body; here the hook must merge a key into the
  just-rewritten frontmatter block, respecting omit-when-empty/canonical-side/key
  ordering, and stay byte-identical to the rewrite's output for idempotency.
  Specify that linkage insertion reuses the same fence-aware machinery as the
  deterministic rewrite.

- 🟡 **Compatibility**: dropping the `cluster_key` `work_item_id` branch assumes `parent:` is fully populated, but foreign `work_item_id:` is retained
  **Location**: Phase 5 §2 / What We're NOT Doing
  Clustering now depends entirely on the migration having populated `parent:` —
  and that population partly routes through the ambiguous hook where a skip writes
  nothing. Add a Phase 4/5 check that every plan/research/pr-description with a
  foreign `work_item_id:` also has `parent:` after migration.

- 🟡 **Compatibility**: validating and emitting `pr:` references against an incomplete published vocabulary risks forward-compat breakage
  **Location**: What We're NOT Doing / Phase 2
  If the pending ADR lands with a different `pr:` spelling/shape, ~487 migrated
  files carry values the then-current vocabulary rejects. Pin the exact `pr:`
  spelling now, and have the validator treat `pr:` as a known-tolerated literal
  rather than accepting any `pr:`-prefixed string.

- 🟡 **Portability**: net-new per-file VCS author resolution does not pin jj/git templates or version floors
  **Location**: Phase 3 §3 (author from VCS history)
  No existing helper resolves a *per-file historical* author; jj (`--template`)
  and git (`--format`/`--follow`) diverge sharply. Specify the exact template/format
  strings and minimum versions, route through one helper, and fall cleanly to
  `Unknown` on unsupported flags; test the jj and git branches independently.

- 🟡 **Portability**: filename/date parsing may invoke non-portable `date` flags (GNU `-d` vs BSD `-j`)
  **Location**: Phase 3 §3 (18 pre-convention plans — date from filename)
  Treat the filename date as an opaque `YYYY-MM-DD` string copied verbatim (no
  `date(1)` round-trip), or normalise via string slicing, as
  `artifact-derive-metadata.sh` already does (emit-only, never parse).

#### Minor

- 🔵 **Architecture**: the single migration script carries three responsibilities; cohesion is framework-forced, not chosen (Phase 3) — keep concerns as separate internal functions.
- 🔵 **Architecture**: tolerated-but-unmodelled `pr:` references leave a vocabulary gap the parser hard-codes around (What We're NOT Doing) — centralise the carve-out in one allowlist.
- 🔵 **Architecture**: `type:`-from-location couples the migration to directory layout; reconcile the inferred type distribution against known corpus counts in the gap-fix log.
- 🔵 **Code Quality**: prose linkage parsing in bash regex is a maintainability hotspot — keep the ADR-0034 tuple table as explicit data, name each spike-fix rule.
- 🔵 **Code Quality**: `note` is `code_state_anchored=yes` in the TSV but omitted from the prose enumeration of anchored types — derive the flag from the TSV column in all three consumers.
- 🔵 **Code Quality**: YAML `id`-value quoting/escaping is asserted but not designed — explicitly reuse 0006's `normalise_value`/`refuses`.
- 🔵 **Code Quality**: the validator-test wiring target is vague — extend the existing `templates` aggregator rather than ad-hoc wiring.
- 🔵 **Correctness**: pre-convention plan filenames are date-only stems — specify the exact `date:` form the backfill emits and confirm the validator accepts it.
- 🔵 **Correctness**: internal inconsistency — research Open Question 6 calls the ~18 non-note no-fence files "likely skip"; the plan backfills them as full plans. Verify by inspection that all 18 are genuinely plans.
- 🔵 **Safety**: the `Unknown` author fallback bakes irreversible attribution loss — distinguish lookup-failure from genuine-absence with a counted diagnostic.
- 🔵 **Safety**: the dogfood mutates this repo's live corpus as the test subject — capture a tagged pre-migration VCS point and record the wrong-rate sample for auditability.
- 🔵 **Database**: type-count discrepancy — the TSV has 13 type rows, the plan repeatedly says "14"; derive the fixture-coverage assertion from the TSV row count.
- 🔵 **Database**: `last_updated` seeding must be a pure function of on-disk fields (never wall-clock) — add a run-twice byte-identical fixture, especially for derived-date backfill files.
- 🔵 **Database**: skipped ambiguous references leave a slot unpopulated with no residual marker — confirm the loss is intentional and record skips in the gap-fix log.
- 🔵 **Test Coverage**: AC-2, AC-10, AC-16-on-real-corpus lack explicit verification steps — add a precondition fixture, a no-placeholder emission assertion, and a date-seeding check on awkward shapes.
- 🔵 **Test Coverage**: `pr:` tolerance and `"Source:"`-line prose disambiguation lack dedicated parser fixtures.
- 🔵 **Test Coverage**: Phase 5 tests that "lose their premise" are deferred to a review judgement — retarget at least one to assert canonical `target:`/`id` resolution post-removal.
- 🔵 **Compatibility**: `templates-schema.tsv` has no `schema_version` column — the validator pins `1` as a constant; add a column or a TODO/test that fails on a non-1 producer version.
- 🔵 **Compatibility**: the `work-item-review` TSV `extras` edit must be same-commit with the template-line removal, or the contract test breaks.
- 🔵 **Compatibility**: VCS author resolution and clean-tree pre-flight assume jj/git availability and locale-stable output — force `LANG=C` and add a git-only-repo fixture.
- 🔵 **Portability**: interactive display relies on `base64 -d` (historically `-D` on BSD) — verify on the lowest supported macOS.
- 🔵 **Portability**: regex-heavy awk/grep over a non-ASCII corpus is locale-sensitive — export `LC_ALL=C`/`LANG=C` for the text-processing pipeline.
- 🔵 **Portability**: the two new scripts must be explicitly subjected to `lint-bashisms.sh` and the bash-3.2 replay gate — add as a success criterion.

### Strengths

- ✅ Phase ordering respects the dependency graph: the validator (Phase 1) and
  linkage parser (Phase 2) land as independently-tested standalone artifacts
  before the migration consumes them, with the Rust removal correctly sequenced
  last.
- ✅ The indivisible-XL justification for Phase 3 is explicit and well-reasoned —
  one migration = one ledger entry, so a partially-built migration recorded as
  applied would block later additions from re-running.
- ✅ The cross-repo coupling forcing the Rust removal into the same release is
  surfaced as an acknowledged tradeoff, not left implicit.
- ✅ Strong per-mutation safety substrate inherited from the framework: atomic
  temp-then-rename on every write, a `.lockdir`-mutexed JSONL session log, the
  write-ahead-log invariant (RECORDED persisted before APPLY), and a clean-tree
  pre-flight scoped to exactly the directories the migration touches.
- ✅ Idempotency is layered across three independent planes (ledger, per-file
  `cmp -s`, interactive resume keyed on `(artifact_path, source_anchor)`).
- ✅ The validator and linkage parser are designed as sourceable libraries with
  CLI entry points, reusable beyond this one-shot migration.
- ✅ The fixture taxonomy is explicit and risk-targeted (per-type good,
  per-failure-mode, band-classification, known-ambiguous, one-per-spike-fix), and
  author resolution is correctly tested on both branches.
- ✅ Conscious portability discipline: anchored to the bash-3.2 floor, reusing
  0006's parallel-array dedup, the mkdir mutex, the two-FIFO transport, and
  jj-then-git auto-detection.

### Recommended Changes

1. **Rewrite the `type:`-from-location inference to match the real corpus**
   (addresses: design-gaps/design-inventories omission, reviews-by-subdir,
   already-typed files). Enumerate every research subdir
   (`codebase/`, `issues/`, `design-gaps/`, `design-inventories/`), map reviews by
   subdirectory (`reviews/plans/`→plan-review, `reviews/work/`→work-item-review),
   and prefer an existing `type:` over location inference. Reconcile the inferred
   distribution against known corpus counts in the gap-fix log.

2. **Route notes to the note shape by location, not by absence-of-fence**
   (addresses: the lone fenced note). The one note with a partial `---` block must
   be brought to the full `note` shape with `status: draft → captured`. Correct
   the "14 notes have no fence" count.

3. **Specify the mechanical-pass / interactive-phase contract**
   (addresses: abort half-state, FIFO stdout, MALFORMED-vs-backfill, partial-apply
   convergence). State the explicit pass order (backfill → awk → `harness_run`);
   redirect all pre-`harness_run` output to stderr; make no-fence files go to
   backfill *only* (excluded from the awk legacy-key pass); add a post-run
   self-validation gate; prove mechanical convergence with an interrupt-then-re-run
   fixture; document revert-before-rerun on a failed interactive session.

4. **Add referential-integrity validation for emitted linkages**
   (addresses: dangling references). Resolve every `"doc-type:id"` against the
   corpus index in the validator (tolerating only the explicit `pr:` exception),
   and state the rule for pre-existing reverse-side (`blocked_by`) keys.

5. **Fix Phase 5 so it does not break migrated corpora**
   (addresses: indexer.rs identity, migrate-on-use, cluster_key parent dependency).
   Retarget the `indexer.rs` work-item read to the unified `id:` key before
   removing the filename fallback; gate or warn the visualiser read path on the
   0007 ledger entry (or keep the fallback one release longer behind a deprecation
   warning); add a Phase 4/5 check that every artifact with a foreign
   `work_item_id:` also has `parent:`; add a test against a deliberately
   un-migrated corpus proving loud, recoverable failure.

6. **Make the resolved-band correctness gate automated and reproducible**
   (addresses: manual binding gate). Add a golden set of resolved-band references
   with hand-verified *target* values asserted in the Phase 2 parser test, and pin
   the ≥150-sample procedure (fixed seed, recorded stratification, checked-in
   classification script).

7. **Replace `\b` regexes with portable character-class boundaries**
   (addresses: BSD awk/grep). Use `(^|[^[:alnum:]])…([^[:alnum:]]|$)` and add a
   macOS-toolchain fixture for the hyphen-boundary ("code-block") case. Pin LANG=C
   for the text-processing pipeline.

8. **Correct the test wiring**
   (addresses: wrong mise task, suite-count guard). Place the new shell suites
   under the integration-globbed subtrees, set the exec bit, and bump
   `_EXPECTED_MIGRATE_SUITES`.

9. **Reframe the schema single-sourcing and idempotency claims**
   (addresses: partial single-sourcing, ledger-skip vs no-op). State that the TSV
   provides per-type tabular facts only and factor cross-cutting rules into one
   shared helper; clarify that the genuine three-plane no-op is proven by the
   direct-invocation fixture, the dogfood re-run being the ledger-skip check.

10. **Defensive preconditions and provenance** (addresses: 0005/0006-applied
    assumption, Unknown attribution, VCS portability). REFUSE on violated
    preconditions; emit a counted diagnostic when the `Unknown` author fallback
    fires; pin the jj/git author-resolution templates and version floors; treat
    filename dates as opaque strings.

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound in its big moves — phase decomposition
respects dependency direction, the validator and parser are correctly carved out
as standalone reusable components, and the two hardest couplings (indivisible
Phase 3, same-release Rust removal) are explicitly justified. The main structural
risks are the single migration script carrying three responsibilities whose
internal ordering and shared-file writes are under-specified, and treating
`templates-schema.tsv` as the single schema source when the validator must layer
non-TSV rules on top and Phase 5 mutates a row the validator depends on.

**Strengths**: Phase ordering respects the dependency graph; explicit
indivisible-XL justification; acknowledged cross-repo coupling; three-plane
layered idempotency; validator/parser designed as reusable sourceable libraries.

**Findings**:
- 🟡 (medium) Validator single-sources from the TSV but a row it depends on is mutated in Phase 5 — Phase 1 §1 / Phase 5 §4.
- 🟡 (medium) Omit-when-empty and location-inferred type are validator rules with no TSV representation — Phase 1 §1.
- 🟡 (medium) Intra-migration ordering between awk rewrite and no-fence backfill is unspecified but load-bearing — Phase 3 §2/§3.
- 🔵 (high) Single migration script carries three responsibilities; cohesion is framework-forced — Phase 3.
- 🔵 (medium) Tolerated-but-unmodelled `pr:` references leave a vocabulary gap the parser hard-codes around — What We're NOT Doing.
- 🔵 (low) `type`-from-location couples the migration to directory layout — Phase 3 §1.

### Code Quality

**Summary**: Reuses the proven 0006 awk-state-machine precedent soundly and
decomposes the two heaviest components into independently tested standalone
artifacts. The chief risks are concentrated in Phase 3: a single script holding a
multi-rule awk rewrite, a backfill state machine, a sourced parser, and the
interactive hooks is a god-object with no proposed internal modularity; schema
interpretation is duplicated between the validator and the contract test; and
writing typed linkage into existing YAML frontmatter is under-specified versus
the body-append precedent.

**Strengths**: Extracting validator and parser as standalone fixture-tested
components; reusing 0006's state machine and conventions; consistent
error-diagnostic conventions and layered idempotency; explicit reuse of 0006's
path-safety helpers.

**Findings**:
- 🟡 (high) Phase 3 migration is a god-object with no proposed internal structure — Phase 3.
- 🟡 (high) Schema interpretation duplicated between the new validator and the template-contract test — Phase 1 §1 vs test-template-frontmatter.sh.
- 🟡 (medium) Writing typed linkage into existing YAML frontmatter under-specified versus the body-append precedent — Phase 3 §4/§2.
- 🔵 (medium) YAML id-value quoting/escaping asserted but not designed; 0006 has the reusable primitive — Phase 3 §2.
- 🔵 (medium) Prose linkage parsing in bash regex is a maintainability hotspot — Phase 2 §1.
- 🔵 (medium) `note` is code_state_anchored=yes in the TSV but omitted from the prose enumeration — Phase 3 §3 / Phase 1.
- 🔵 (low) Validator-test wiring target is vague; an aggregator task already exists — Phase 1 §3.

### Test Coverage

**Summary**: Unusually test-forward — components land before consumers, the
fixture taxonomy is named and risk-targeted, and most ACs map to verification
steps. The most serious gaps: the binding resolved-band ≤5% wrong-rate gate is a
manual, non-reproducible measurement with no automated correctness check behind
the mechanical path; AC-7's terminal-state gate is satisfiable by skips so the
dogfood never proves the apply path; the new shell suites are wired into the
wrong test task and the migrate suite-count guard is not updated.

**Strengths**: Components-before-consumer sequencing; explicit risk-targeted
fixture taxonomy with one-per-spike-fix; golden-output tests plus a
pre-migration-corpus sanity check; both author-resolution branches; idempotency
across three planes; reliance on an established scripted-decision harness.

**Findings**:
- 🔴 (high) The binding gate is a manual, non-reproducible measurement with no automated correctness check behind the mechanical path — Phase 4 §2 / Manual Verification.
- 🟡 (high) AC-7's terminal-state gate is satisfiable by skips, so the dogfood does not prove the apply path — Phase 3 §5 / Phase 4 §3.
- 🟡 (high) New shell suites wired into the wrong test task; migrate suite-count guard not updated — Phase 1 §3 / Phase 2 / Testing Strategy.
- 🟡 (medium) Idempotency no-op asserted only on a fixture corpus, not the real dogfood corpus — Phase 3 §5 / Phase 4.
- 🔵 (high) AC-2, AC-10, AC-16-on-real-corpus lack explicit verification steps — Success Criteria.
- 🔵 (medium) `pr:` tolerance and Source:-line disambiguation lack dedicated fixtures — Phase 2 §1.
- 🔵 (medium) Phase 5 surviving tests deferred to a review judgement rather than given a defined post-removal assertion — Phase 5.

### Correctness

**Summary**: Logically careful and leans correctly on 0006's proven machinery,
the layered idempotency model, and the 0069 contract. But several boundary
conditions are misstated or unhandled: the `type:`-from-location rule omits real
research subdirs and mis-frames the subdirectory-split reviews; the lone partial
note already has a fence and escapes the no-fence backfill gate; the
mechanical-pass-before-`harness_run` ordering risks writing non-protocol bytes
onto the FIFO; and the "three idempotency planes" conflate ledger-skip with a
verified no-op.

**Strengths**: Correct cmp -s idempotency reasoning (date-seeded, only-when-absent);
correct resolved/ambiguous band rule and routing; correct own-vs-foreign identity
handling; recognises skip-still-reaches-APPLIED_CONFIRM.

**Findings**:
- 🔴 (high) `type`-from-location omits design-gaps/design-inventories and mis-handles already-typed files — Current State / Phase 3 §1.
- 🔴 (high) Reviews are classified by subdirectory, not filename — Current State Analysis.
- 🔴 (high) The lone partial note already has a fence and falls through the no-fence backfill gate — Phase 3 §3.
- 🟡 (medium) Mechanical-pass stdout will be parsed as interactive protocol frames on the FIFO — Phase 3 §1.
- 🟡 (high) Ledger plane makes a re-run skip 0007 entirely, not re-run it as a verified no-op — Desired End State / Phase 4.
- 🟡 (medium) awk MALFORMED-on-no-fence vs deliberate no-fence backfill need an explicit per-file gate — Phase 3 §2/§3.
- 🔵 (medium) Pre-convention plan filenames are date-only stems; `date`/`id` inference must handle the shape — Phase 3 §3.
- 🔵 (low) Internal inconsistency: research says "likely skip" the ~18 non-note no-fence files; plan backfills them as plans — What We're NOT Doing.

### Safety

**Summary**: Inherits a genuinely strong per-mutation safety substrate (atomic
writes, .lockdir mutex, write-ahead-log invariant, layered idempotency, scoped
clean-tree pre-flight); with VCS revert as the sanctioned recovery path the blast
radius is bounded. Three gaps stand out for a 487-file in-place rewrite: the
unrecorded mechanical pass completes before the interactive phase so an abort
leaves a half-state with no whole-or-nothing check; REFUSE/MALFORMED do not fail
the run, yielding silent per-file partial migration; and the irreversible
same-release Rust removal eliminates graceful degradation before reliability is
proven beyond one corpus.

**Strengths**: Atomic temp-then-rename everywhere; write-ahead-log invariant;
three-plane idempotency; tightly scoped pre-flight with a resume/discard message;
runner aborts and does not record the ledger on non-zero exit.

**Findings**:
- 🔴 (high) Interactive abort after the mechanical pass leaves the corpus rewritten but the ledger unrecorded, with no whole-or-nothing guarantee — Phase 3 §1.
- 🟡 (high) REFUSE/MALFORMED diagnostics do not abort, yielding silent per-file partial migration — Phase 3 §2 / Desired End State.
- 🟡 (medium) Irreversible same-release coupling removes graceful degradation before reliability is proven — Phase 5 / Migration Notes.
- 🔵 (high) The `Unknown` author fallback bakes irreversible attribution loss — Phase 3 §3.
- 🔵 (medium) The dogfood mutates this repo's live production corpus as the test subject — Phase 4.

### Database

**Summary**: As a schema migration over the file corpus, the plan is disciplined:
layered idempotency, single-sourced per-type rules, the 0006 transform precedent,
and forward-only with VCS revert. The most serious database-layer risks are
referential integrity of the emitted typed linkages (no existence check) and a
partial-apply recovery hazard. Secondary: the TSV does not encode every rule the
validator needs, a 13-vs-14 type-count discrepancy, and the unbounded assumption
that 0005/0006 were *applied* rather than merely present.

**Strengths**: Sound, explicitly reasoned three-plane idempotency; schema
single-sourced from the TSV with a drift guard; correct forward-only-with-revert
call; numbering verified against the ledger; atomic writes plus write-ahead-log
session log.

**Findings**:
- 🔴 (high) No referential-integrity check that emitted typed linkages point at existing artifacts — Phase 2 / Phase 3 §4.
- 🔴 (medium) Partial-apply recovery: a non-zero exit mid-corpus or ledger-recording of an incomplete 0007 blocks correction — Phase 3 §1 / Migration Notes.
- 🟡 (high) templates-schema.tsv does not encode every rule the validator claims to derive from it — Phase 1 §1.
- 🟡 (high) 0007 depends on 0005/0006 being applied, but the runner only guarantees they sort first — Migration Notes.
- 🟡 (medium) Canonical-side-only linkage writes leave the reverse relation unmaterialised with no integrity guarantee — Phase 3 §4.
- 🔵 (high) Type-count discrepancy: the TSV defines 13 type rows, the plan says 14 — Phase 1 Success Criteria.
- 🔵 (medium) `last_updated` seeding interacts with the cmp -s gate; verify stability across re-runs — Phase 3 §2.
- 🔵 (medium) Skipped ambiguous references leave a slot unpopulated with no integrity record — Phase 3 §4.

### Compatibility

**Summary**: The central risk is the same-release coupling between the corpus
migration and the Rust fallback removal, justified by a migrate-on-use contract
the codebase does not enforce (migration is advisory). More concretely, removing
the `indexer.rs` filename-id fallback strips the only working identity source for
migrated work-items, breaking the work-item graph for exactly the corpora the
migration produces. Migration ordering, protocol versioning, and the `pr:`
carve-out are smaller but real.

**Strengths**: Migration ordering enforced by lexical ledger replay; three-plane
idempotency makes re-runs safe; session-log schema_version validated on resume;
single-sourced wire-protocol escape contract; Phase 5 ordered last with canonical
read paths retained.

**Findings**:
- 🔴 (high) Removing the indexer.rs filename-id fallback breaks identity of the work-items the migration just produced — Phase 5 §3.
- 🔴 (high) The migrate-on-use guarantee underpinning same-release fallback removal is not enforced — Migration Notes / Phase 5.
- 🟡 (medium) Dropping the cluster_key work_item_id branch assumes parent: is fully populated, but foreign work_item_id is retained — Phase 5 §2.
- 🟡 (medium) Validating/emitting pr: references against an incomplete vocabulary risks forward/backward incompatibility — What We're NOT Doing / Phase 2.
- 🔵 (medium) templates-schema.tsv has no schema_version column; the validator pins 1 as a constant — Phase 1.
- 🔵 (medium) The work-item-review TSV extras edit must be same-commit with the alias removal — Phase 5 §4.
- 🔵 (low) VCS author resolution and clean-tree pre-flight assume jj/git availability and locale-stable output — Phase 3 §3.

### Portability

**Summary**: Extends a codebase with strong, conscious portability discipline
(enforced bash-3.2 floor, mkdir mutex, two-FIFO transport, jj/git auto-detection,
atomic same-dir rename), and the new components inherit it by modelling on 0006.
But three load-bearing decisions are unspecified, the most serious being that the
spike-mandated `\b` word-boundary regexes are non-portable across BSD awk and BSD
grep (the macOS defaults) — they would classify references differently per
platform. The net-new per-file VCS author resolution and any filename-date
parsing also reach for jj/git/date surfaces whose flag differences the plan never
pins down.

**Strengths**: Anchored to the bash-3.2 floor with lint-bashisms.sh as a gate;
reuses portable primitives (atomic rename, mkdir mutex, two FIFOs); jj-then-git
auto-detection with Unknown fallback; models 0007 on 0006's portable walker.

**Findings**:
- 🔴 (high) `\b` word-boundary regexes are not portable across BSD awk / BSD grep — Phase 2 §1.
- 🟡 (high) Net-new per-file historical author resolution does not pin jj/git templates or version floors — Phase 3 §3.
- 🟡 (medium) Filename/date parsing may invoke non-portable `date` flags (GNU `-d` vs BSD `-j`) — Phase 3 §3.
- 🔵 (medium) Interactive display relies on `base64 -d`, non-portable on older BSD — Phase 2/3.
- 🔵 (medium) Regex-heavy awk/grep over the corpus is locale-sensitive without LANG pinning — Phase 1 §1 / Phase 3 §2.
- 🔵 (low) The two new scripts must be explicitly subjected to the bash-3.2 lint and replay gate — Implementation Approach / Phase 3.

---

## Re-Review (Pass 2) — 2026-06-07T21:03:28+00:00

**Verdict:** REVISE

All 8 lenses were re-run against the revised plan. **Every one of the 10 original
critical findings is resolved** — multiple lenses verified the fixes against the
code (the FIFO `READY`-frame ordering, `indexer.rs` work-item identity, the
advisory `migrate/SKILL.md` gating, the `_EXPECTED_MIGRATE_SUITES` guard, the
strict `/^---$/` fence behaviour on the partial note). The verdict stays **REVISE**
only because the deeper pass surfaced a fresh layer of **major** findings (0
critical, ~10 major, ~25 minor) — second-order issues exposed once the criticals
were cleared, several of them concrete and quick to fix (notably the un-stripped
`work_item_id:` alias on existing review artifacts and the backfill date format).
The plan is now structurally sound; this is a normal second iteration, not a
re-litigation of the first.

### Previously Identified Issues

**Criticals (all resolved):**
- 🔴→✅ **Correctness**: `type:`-from-location omitting design-gaps/design-inventories + already-typed files — **Resolved** (exhaustive 12-location map; inference gated on absence; design-gap/inventory `type:` no longer overridden).
- 🔴→✅ **Correctness**: reviews classified by filename not subdir — **Resolved** (subdir map `plans`/`work`/`prs`).
- 🔴→✅ **Correctness**: partial-fence note escapes the no-fence backfill — **Resolved** (notes routed by location; partial fence + `status: draft→captured` handled).
- 🔴→✅ **Safety**: interactive abort leaves corpus rewritten, ledger unrecorded — **Resolved** (REFUSE/MALFORMED fatal, post-pass self-validation, named pre-migration VCS point, revert-before-rerun) — see new finding on pre-pass timing.
- 🔴→✅ **Database**: no referential-integrity check on emitted linkages — **Resolved** (validator resolves every `"doc-type:id"`; `pr:` tolerated literal).
- 🔴→✅ **Database**: partial-apply recovery unaddressed — **Resolved** (interrupt-then-rerun convergence fixture + revert docs) — see new finding on pre-pass ordering.
- 🔴→✅ **Compatibility**: `indexer.rs` removal breaks migrated work-item identity — **Resolved** (unified `id:` read path added this release; arms retained) — see new minor on `normalise_id`.
- 🔴→✅ **Compatibility**: migrate-on-use not enforced — **Resolved** (expand/migrate/contract split; arm removal deferred to follow-on).
- 🔴→✅ **Test Coverage**: binding gate manual/non-reproducible — **Resolved** (resolved-band golden-target fixtures + fixed-seed reproducible sample).
- 🔴→✅ **Portability**: `\b` regexes non-portable on BSD — **Resolved** (POSIX character-class boundaries mandated; macOS hyphen fixture).

**Majors (representative — resolved):** schema single-sourcing reframed; test wiring → integration subtree; ledger-skip vs no-op clarified; REFUSE abort; canonical reverse-side reconciliation rule added; AC-7 skip apply-path requirement; defensive 0005/0006 preconditions; VCS author template pinning; opaque dates; LANG=C. All **Resolved or substantially addressed**.

### New Issues Introduced

These are newly surfaced (mostly second-order); none is a regression of a prior fix.

- 🟡 **Code Quality**: `0006`'s `normalise_value`/`refuses` are awk-*internal* functions, not sourceable primitives — the "reuse, don't hand-roll" instruction is unachievable as written without an extraction mechanism (a shared `awk -f` include, or an accepted copy + a byte-for-byte equivalence fixture).
- 🟡 **Correctness**: backfilled `date:`/`last_updated:` are bare `YYYY-MM-DD`, but `templates/note.md`/`plan.md` and the corpus use ISO timestamps — the validator's `last_updated` check may reject them and the "shape-consistent with create-note" claim breaks. Pin both forms and extend the validator checks.
- 🟡 **Compatibility**: the `work-item-review` `work_item_id:` alias is dropped from the schema row this release, but the migration's awk rules don't **strip** it from the ~48 existing review artifacts — they'd fail the closed-set validator on the dogfood. Add an explicit strip rule + fixture.
- 🟡 **Database/Safety**: the fatal precondition REFUSE can only fire in the awk pass (step 2), *after* the backfill pass (step 1) has already written files — a fatal abort leaves a partially-mutated tree. Make the precondition a **whole-corpus pre-pass** that REFUSEs before any write.
- 🟡 **Safety/Correctness/Database**: the post-pass self-validation runs *before* `harness_run` populates linkage, so the apply path's writes aren't caught in-run; and referential integrity is a whole-corpus property the touched-file/mid-flight index can't evaluate cleanly. Run a final self-validation after `harness_run`; scope the in-run gate to structural checks; resolve the index against the same identity rule (`id:` → legacy → filename).
- 🟡 **Test Coverage/Architecture/Compatibility**: the deferred follow-on's precondition ("every foreign-`work_item_id:` artifact also has `parent:`") is unsatisfiable as-is — skipped ambiguous refs leave `parent:` unpopulated. Either the follow-on needs a mechanical `work_item_id:`→`parent:` backfill, or the contract gate must be "`parent:` present OR foreign `work_item_id:` retained" (the legacy branch stays the safety net for skips). Establish/test `parent:` population in *this* release.
- 🟡 **Test Coverage**: the `_EXPECTED_MIGRATE_SUITES` bump is inconsistent with the stated packaging (fixtures + extending an existing suite add no new discoverable `test-*.sh`) — clarify whether a standalone suite file is added (bump) or not (don't).
- 🔵 **Minor (notable)**: `id:` read path must route through `normalise_id` like the legacy path (Correctness); stale "removed in the release that closes 0070" doc-comments on the retained arms (Compatibility); per-arm deprecation-warning tests + capture mechanism (Test Coverage); shared-helper single-source needs an automated guard, not just manual (Test Coverage); the boundary idiom should match `lint-bashisms.sh`'s underscore-inclusive class verbatim (Portability); the reused harness's `base64 -d` is GNU-only on older macOS (Portability); negative fixture for the in-run self-validation actually aborting (Test Coverage); create the follow-on work item as a tracked deliverable so deprecate-then-never-contract can't ossify (Architecture).

### Assessment

The revision did its job: the plan went from ten critical, ship-blocking defects to
zero, with the fixes verified against the codebase rather than asserted. It is now
in good structural shape and close to implementation-ready. The remaining majors
are localised and concrete — the most important being the **un-stripped
`work_item_id:` alias** (a guaranteed dogfood validator failure), the **backfill
date/`last_updated` format**, the **`normalise_value` extraction mechanism**, the
**pre-pass precondition ordering**, and the **`parent:`-population/follow-on
contract**. A second revision pass addressing these (and the high-value minors)
would bring the plan to APPROVE; none requires re-architecting what was just
fixed.

---

## Re-Review (Pass 3) — 2026-06-07T23:16:16+00:00

**Verdict:** REVISE

All 8 lenses re-run against the twice-revised plan. **All pass-2 findings are
resolved** and verified against the code/corpus (the `_EXPECTED_MIGRATE_SUITES`
discovery semantics, the `read_ref_keys` else-if precedence, `target_path_from_entry`
index behaviour, the existing tracing-capture precedent). But the pass-3 deep dive
surfaced **4 new critical findings — all within the corpus-wide
frontmatter-linkage-normalisation scope that was *added* in pass 2's revision** —
plus ~13 majors. These are not regressions; they are genuine, corpus-verified
defects in newly-introduced mechanical scope, and one of them is a **real latent
data bug in the live corpus** the review process flushed out. Verdict REVISE on
the critical-severity rule.

**Meta-note on convergence:** each pass has resolved its predecessor's criticals;
pass-3's criticals exist because pass-2 introduced substantial new scope
(normalising ~160 legacy frontmatter linkage values) to fix a pass-2 major. That
scope is correct to include, but it needs the tighter specification below before
the plan is implementation-ready. The non-normalisation parts of the plan are now
solid.

### Previously Identified Issues (pass-2 majors — all resolved)

- 🟡→✅ **Code Quality**: `normalise_value`/`refuses` extraction — **Resolved** (shared `frontmatter-awk.inc`, equivalence fixture; agent confirmed the awk `-f` mechanism is realistic).
- 🟡→✅ **Correctness**: backfill date format — **Resolved** (ISO via string-concat midnight suffix, no `date(1)`).
- 🟡→✅ **Compatibility**: `work_item_id:` alias not stripped — **Resolved** (migration strips it from the ~49 artifacts, coordinated with schema-row + template edits; agent confirmed all three agree).
- 🟡→✅ **Database/Safety**: fatal REFUSE after backfill → partial tree — **Resolved** (step-0 read-only precondition pre-pass, zero mutations).
- 🟡→✅ **Safety/Correctness/Database**: self-validation timing — **Resolved** (two-stage: structural post-mechanical, full+referential post-`harness_run`) — see new finding on the `harness_run`-is-last-line ambiguity.
- 🟡→✅ **Test Coverage**: `_EXPECTED_MIGRATE_SUITES` packaging — **Resolved** (standalone `test-migrate-0007.sh` + bump 3→4; agent verified consistent with `run_shell_suites`).
- 🟡→✅ **Test/Arch/Compat**: follow-on `parent:` precondition — **Resolved** (mechanical `work_item_id:`→`parent:` derivation populates it this release; follow-on a tracked deliverable).
- 🔵→✅ Minors: `id:` via `normalise_id`, stale doc-comments, per-arm deprecation tests, underscore-class idiom, reverse-side rule, validator invocation contract — all addressed.

### New Issues Introduced (in the pass-2 normalisation scope)

**Critical (corpus-verified):**
- 🔴 **Correctness**: the **path→typed id-extraction rule is ambiguous**. The example strips `meta/work/0030-foo.md`→`work-item:0030` (bare number), but ~83 plan-review `target:` values point at **dated-stem plans whose `id:` IS the full stem** (`2026-05-13-0055-sidebar-…`). A bare-number rule yields `plan:0055`, which no plan's `id:` matches → ~83 dangling refs fail the referential-integrity gate. **Fix:** id-extraction must be a function of the *target* doc-type (work-item/ADR → number; plan/research/review/validation/note → full stem).
- 🔴 **Correctness**: **parent-from-foreign-`work_item_id:` only handles bare numbers**, but the live foreign key exists in three shapes — bare (`"0079"`), **path-shape** (`"meta/work/0072-….md"`), and **already-typed** (`"work-item:0101"`). Naive concat produces `work-item:meta/work/0072-….md` / `work-item:work-item:0101` (dangling). **Fix:** normalise the foreign value to its bare id first, across all three shapes.
- 🔴 **Database**: **duplicate primary key** — `meta/work/0032-per-test-server-configuration-for-e2e-tests.md` carries `work_item_id: "0031"` (verified: both 0031 and 0032 files hold `"0031"`). The own-id→`id:` rewrite mints a second `id: "0031"`, colliding in the referential-integrity index. **A real latent corpus bug.** **Fix:** step-0 precondition REFUSE on own-id≠filename-id (or duplicate post-rewrite `id:`); correct the stray value before running.
- 🔴 **Database**: **parent-derivation propagates the 0032→0031 skew** into a wrong-but-resolvable `parent:` edge that *no gate catches* (it resolves to a real, wrong work-item). **Fix:** existence-check the derived target; DIVERGE on zero/multi-resolution.

**Major (representative):**
- 🟡 **Code Quality/Correctness**: bare-number **frontmatter** linkage routes "to the hook", but `migration_emit_transformations` only runs the parser over **body sections** — no plumbing feeds a frontmatter bare-number to the interactive emit. Specify the side-channel, or convert deterministic key+type pairings (e.g. `parent:` on a work-item) mechanically and only route genuinely multi-candidate values.
- 🟡 **Architecture/Safety**: `harness_run`-is-last-line (§4) contradicts stage-2-validation-after-`harness_run` (§2) — if literally last, the referential-integrity gate never runs. State `harness_run` is *not* last and stage-2 must exit non-zero to block the ledger.
- 🟡 **Architecture**: schema-row alias drop (Phase 5) and corpus alias-strip (Phase 4) are in *independently-mergeable* phases — constrain the merge order or have the validator tolerate a residual legacy alias as a known-legacy (not unknown-key) violation.
- 🟡 **Architecture/Code Quality**: two shared frontmatter helpers (`frontmatter-awk.inc` producer + emission-rules helper validator) need an explicit boundary + a producer-through-validator cross-check fixture; and the location map is now duplicated into awk (pass via `-v`/data file, don't hand-copy).
- 🟡 **Compatibility**: `target_path_from_entry` returns `None` for `TypedRef::WorkItem`, so typing a review's `target:` drops the `reviews_by_target` reverse edge — it survives only via a different index (`work_item_refs_by_id`) that depends on the Phase 5 §1 `id:` path. Make the conversion edge-preserving or document+test the index substitution.
- 🟡 **Compatibility**: reader-expand (`id:` path) must ship *before or with* `0007`, never after — state the ordering in Migration Notes.
- 🟡 **Correctness**: parent-derivation idempotency/precedence vs a pre-existing `parent:` (some plans already have it; some have an empty placeholder) — replace empty placeholder, no-op on agreement, DIVERGE on disagreement.
- 🟡 **Correctness**: validator must accept **both** the `Z` and `+00:00` ISO forms (the untouched corpus carries both); add a `Z`-suffix good fixture.
- 🟡 **Safety**: unconditional alias-strip on a review whose alias *disagrees* with `target:` silently loses the value — compare-then-strip, DIVERGE on disagreement.
- 🟡 **Test Coverage**: `test:integration:config` has no suite-count guard, but Phase 1/2 add two suites there via exec-bit discovery — add a `_EXPECTED_CONFIG_SUITES` guard or justify its absence.
- 🟡 **Test Coverage**: the single-source guard must preserve `test-template-frontmatter.sh`'s hardcoded self-test counts (`-eq 6`/`-eq 9`).

**Minor (notable):** alias-strip must gate on "`target:` is typed" (state predicate), not "converted this run", to catch already-typed-target reviews; reverse-side canonical-existence check must normalise both sides' id shape (fires on the ADR-0026/0036/0039 dual-supersede, and mutates an *immutable* ADR — make conscious); body-region path-shape/foreign-id occurrences (in code/example blocks, some pointing at fictional files) must not be normalised — add a fence-region fixture; per-arm tracing capture needs thread-local/synchronous guarantees; `.inc` extension departs from the `.awk` convention + pin trailing-newline/BEGIN-ownership on the fragment; `base64 -d` flagged manually only.

### Assessment

The plan's spine is now sound — base-field rewrite, backfill, interactive
contract, idempotency, safety net, the expand/migrate/contract visualiser
sequencing, and test wiring are all in good shape and verified against the code.
The remaining work is concentrated entirely in the **frontmatter-linkage
normalisation** added in pass 2: its id-extraction, multi-shape foreign-id
handling, primary-key-collision guard, and frontmatter-bare-number routing each
need a tighter spec, and the review surfaced a genuine corpus data bug (the
0032/0031 own-id collision) that must be corrected regardless. These are concrete
and bounded — a fourth revision targeting the normalisation rules (plus the
`harness_run` ordering and the `target_path_from_entry` edge) should reach
APPROVE. Equally defensible: split the frontmatter-linkage normalisation into its
own follow-up story so 0070 ships the (already-solid) base-field + backfill +
body-section-linkage migration now, and the trickier frontmatter-linkage
normalisation lands separately with room to specify it fully.

---

## Re-Review (Pass 4) — 2026-06-07T23:37:20+00:00

**Verdict:** REVISE

All 8 lenses re-run. **All 4 pass-3 criticals are resolved** and verified against
the code/corpus (the per-target-doc-type id extraction against real plan/review
`id:` values; the 0032/0031 own-id collision — whose H1 *also* reads "0031",
confirming a wholesale-wrong file; the `harness_run`/ledger gate via
`wait_status`; `target_path_from_entry`). Pass 4 found **1 new critical + ~12
majors**. The new critical was fixed in-session (see below); it was again a
corpus-verified data issue the review flushed out.

**Convergence trend:** criticals per pass have gone 10 → 0(+10 majors) → 4 → 1.
Each pass resolves its predecessor's criticals; the residue is increasingly
fine-grained implementation specification, and a disproportionate share of every
post-pass-1 finding traces to the frontmatter-linkage **normalisation** scope
folded in at pass 2. The plan's architecture, approach, safety model, and
non-normalisation mechanics are now thoroughly vetted and sound.

### Previously Identified Issues (pass-3 criticals — all resolved)

- 🔴→✅ **Correctness**: path→typed id-extraction ambiguity — **Resolved** (extraction keyed off target doc-type; verified against real plan-review targets and the two 0030-prefixed plans).
- 🔴→✅ **Correctness**: parent-from-foreign-id only handled bare numbers — **Resolved** (normalises all three live shapes; verified idempotent on the real `work-item:0101` case).
- 🔴→✅ **Database**: duplicate primary key (0032/0031) — **Resolved** (step-0 REFUSE; confirmed 0032 is the sole own-id≠filename mismatch, and its H1 also wrongly reads 0031).
- 🔴→✅ **Database**: parent-derivation propagating the skew — **Resolved** (existence-checked, DIVERGE+skip on zero/multi-resolution).
- Plus pass-3 majors (`harness_run` ordering, schema/corpus merge order, two-helper boundary, reader-expand ordering, ISO `Z`/`+00:00`) — all **Resolved**.

### New Issues (pass 4)

**Critical (fixed in-session):**
- 🔴→✅ **Database**: **71 fenced files carry a date-only `last_updated:`** the awk left untouched (it seeded only when absent), which the ISO-requiring validator would reject — failing the AC-1 gate corpus-wide. **Fixed**: the awk now *normalises* an existing date-only `date:`/`last_updated:` to full ISO (string-concat midnight; already-ISO is a no-op), byte-stable, fixture-pinned.

**Major (not yet applied — see Assessment):**
- 🟡 **Correctness/Safety**: the step-0 duplicate-id pre-pass runs *before* backfill, so the 31 backfill-derived full-stem ids escape its uniqueness check — fold backfill-predicted ids into the step-0 scan.
- 🟡 **Database**: `id:` derivation is unspecified for fenced research/plan files that carry *no* legacy own-id key and no `id:` — state it derives from the filename stem (quoted).
- 🟡 **Code Quality**: the side-channel is read *inside* the command-substituted `migration_emit_transformations`, so any stray stdout (parser diagnostics, the side-channel read) corrupts the TX frame stream — require all non-`TX` output to stderr and extend the hygiene fixture.
- 🟡 **Code Quality**: side-channel file lifecycle (path, format, truncate-on-create, interrupt/re-run) is unspecified.
- 🟡 **Code Quality**: 0007 concentrates five heavyweight concerns in one file — source concern-specific helpers (author-resolution, backfill, linkage-emit) while keeping one ledger unit.
- 🟡 **Architecture**: Phase 5 is no longer independently mergeable — §1 (reader-expand) must precede Phase 3 while §4 (schema drop) must follow Phase 4. Split into Phase 5a (before/with 0007) and 5b (after Phase 4).
- 🟡 **Safety/Architecture**: the runner's 30s post-DONE watchdog can SIGKILL a legitimate whole-corpus stage-2 validation — run stage-2 outside the watchdog window or pin the time budget.
- 🟡 **Compatibility**: extending `target_path_from_entry` to resolve `TypedRef::WorkItem` is a signature change rippling to ~5 call sites (the plan names only 2); enumerate them and the `work_item_cfg`/`canonicalise_one_id` threading.
- 🟡 **Compatibility**: `refresh_one` on a renamed work-item orphans its inbound review edges until rescan — add a test or document rescan-required.
- 🟡 **Test Coverage**: the producer→validator cross-check and the reverse-side (ADR-0026) reconciliation are described in prose but absent from the enumerated fixture list/success criteria — add named fixtures.
- 🟡 **Test Coverage**: side-channel→`PROMPT` belongs in the interactive-harness suite (which drives the FIFO), not the golden suite; pin the `_EXPECTED_CONFIG_SUITES` baseline value.

**Minor (notable):** step-2 value-quoting REFUSE still leaves a partial tree (recoverable via revert — document the asymmetry vs step-0's zero-mutation guarantee); reverse-side cross-file write is non-atomic (write canonical-on-target before dropping reverse); side-channel anchor namespace vs body-section anchors (key-space collision risk); per-arm tracing capture thread-locality; `awk -v` map-delimiter must avoid escape-prone bytes; functions-only fragment should not carry an `awk -f` shebang; stale "closes story 0070" comment lives in `frontmatter.rs` not `cluster_key.rs`; follow-on pinning-test line refs need re-deriving.

### Assessment

The plan is now **architecturally sound and exhaustively vetted** — four review
passes, every critical resolved and code/corpus-verified, with two genuine latent
corpus bugs (the 0032/0031 id collision and the 71 date-only `last_updated:`
files) flushed out along the way. The single pass-4 critical is fixed. The
remaining ~12 majors are **implementation-specification detail** — call-site
enumeration, side-channel lifecycle, fixture enumeration, phase-merge granularity
— the kind of thing test-driven implementation surfaces and resolves naturally,
and several are localised to the normalisation scope folded in at pass 2.

**Recommendation:** stop the review→fix→re-review loop here. Continuing to pass 5
would very likely repeat the pattern (resolve these majors, surface finer ones).
The higher-leverage paths now are: (a) **accept the plan as approved-with-notes**
and let TDD implementation drive out the residual detail (the test fixtures the
plan already mandates will catch them); or (b) **split the frontmatter-linkage
normalisation into its own follow-up story** — it has been the dominant source of
findings in every pass since it was introduced, and isolating it would let 0070's
vetted spine ship while the normalisation is specified at leisure. Either is
defensible; further in-place revision has diminishing returns.

---

## Re-Review (Pass 5) — 2026-06-07T23:56:59+00:00

**Verdict:** REVISE

All 8 lenses re-run. **All pass-4 fixes verified resolved** — and verified deeply
against source: several lenses (code-quality, architecture, safety, test-coverage,
portability) independently concluded the plan is implementation-ready *in their
domain*, confirming the call-site enumeration, lock order, `with_default` tracing
precedent, `run_shell_suites` discovery, and the `target_path_from_entry` None
contract. Pass 5 found **1 new critical + ~5 majors**. The critical is the
**third consecutive pass surfacing the same systemic class** — see below.

### Previously Identified Issues (pass-4 critical + majors — all resolved)

- 🔴→✅ **Database**: 71 date-only `last_updated:` files — **Resolved** (awk normalises date-only→ISO; verified 72 total, 71 in-scope).
- 🟡→✅ Majors: step-0 backfill-id union, `id:`-from-stem, side-channel lifecycle/stdout-hygiene, Phase 5a/5b split, `target_path_from_entry` call-site enumeration + canonicalisation, watchdog budget, producer→validator & reverse-side fixtures, `_EXPECTED_CONFIG_SUITES` — all **Resolved** and source-verified.

### New Issues (pass 5)

**Critical:**
- 🔴 **Database**: **present-but-out-of-vocab `status:` values fail the validator corpus-wide.** ~54 of ~88 plans carry `status:` outside the plan vocab (`draft|ready|in-progress|done`): `accepted` (21), `complete` (16), `approved` (7), `implemented` (6), `reviewed`/`revised`/`final`; design-inventories carry `superseded`. The awk seeds `status` only when *absent* and never normalises a present out-of-vocab value, so the validator rejects them — the **same bug class as pass-4's date-only `last_updated:`**, now in `status:`. **This needs a semantic decision** (expand the vocab via ADR, or a deterministic legacy→canonical status map), not just a one-line fix.

**Major:**
- 🟡 **Correctness**: a `last_updated` *seeded* from a date-only `date:` escapes the date-only→ISO normaliser (the seed synthesises a value the normalise rule never re-reads) — normalise first, then seed.
- 🟡 **Correctness**: the step-0 predicted-id union says "31 fence-less files" but backfill covers 32 (the partial-fence note is backfilled by location) — the partial-fence note's predicted id is omitted from the duplicate check (off-by-one).
- 🟡 **Correctness**: the `frontmatter:<key>` side-channel anchor can't disambiguate two ambiguous bare numbers in a *list-valued* key (`relates_to`/`blocks`) — same resume key → a dropped prompt on resume. Qualify the anchor with value/index.
- 🟡 **Compatibility**: an existing pinning test (`typed_work_item_target_returns_none_resolved_by_cluster_key_resolver`) asserts the *old* `None` behaviour the Phase 5a edit inverts — it must be retargeted, not "kept green"; and the signature change also breaks ~10 test call sites the enumeration omits.
- 🟡 **Database**: the seed-vs-normalise question also applies to a fenced date-only `date:` (confirm the normaliser covers `date:`, not `last_updated:` only).

**Minor/suggestion:** date-normalise must preserve surrounding quote bytes; body-region unterminated-quote `date:` fixture; 0007 internal decomposition into sourced concern-helpers (latitude); the shared frontmatter-edit helper (§2↔§4) is named only by description; `[author-lookup-failed]` tag lives in a different sub-section from its producer; location-map `-v` handoff agreement fixture; stage-2/watchdog has no automated timing guard; `base64 -d` BSD caveat (reused harness); BSD-awk parity fixtures should SKIP-loudly on gawk-only CI; Implementation-Approach "independently mergeable" header should forward-reference the partial order.

### Assessment — a systemic pattern, and a recommendation

**The headline:** passes 4 and 5 each found exactly one critical, and they are the
**same class** — *the corpus accumulated base-field values that predate the unified
schema (date-only timestamps; legacy status vocabularies), and the migration's
"seed/infer only when absent" design never normalises a present-but-nonconforming
value, so each surfaces as a corpus-wide validator failure.* A pass 6 would likely
find the next instance (some other field). This is not a defect in any single rule
— it is a **missing general principle**.

**Recommendation — stop iterating field-by-field; instead do one of:**

1. **Add a general normalisation principle to Phase 3 §2** — "for *every* base
   field, a present value that does not conform to the unified schema is either
   deterministically normalised to a conforming value or REFUSE/DIVERGE'd; no
   present nonconforming value is silently left for the validator to reject" —
   and enumerate the known nonconforming fields (status, dates) with their maps.
   This requires **a semantic decision on the status vocabulary** (the legacy
   plan statuses `accepted`/`complete`/`implemented`/`approved`/… do not map
   cleanly onto `draft|ready|in-progress|done`): either expand the plan/
   design-inventory `status_vocab` (an ADR-level schema change) or define an
   explicit legacy→canonical mapping.

2. **Split the frontmatter normalisation scope out** (the option weighed at
   pass 3): ship 0070's vetted base-field + backfill + body-linkage + visualiser-
   expand spine; let the corpus-value normalisation (linkage shapes, status vocab,
   date forms) land as a dedicated story specified against this systemic principle.

3. **Accept-with-notes**: the architecture is exhaustively vetted; treat the
   recorded findings as an implementation checklist and let TDD drive them out,
   accepting that the status-vocab decision must be made during implementation.

Continued in-place pass-by-pass revision has clearly hit diminishing returns: the
spine is sound and source-verified, and the residue is a single well-understood
systemic class plus implementation detail.

---

## Re-Review (Pass 6, final targeted) — 2026-06-08T00:12:04+00:00

**Verdict:** COMMENT (was REVISE; the one critical was fixed in-session and the
systemic class is now closed by construction)

The targeted final pass re-ran the five lenses touching the pass-5 changes
(database, correctness, code-quality, test-coverage, compatibility). The pass-5
fixes all verified sound, and compatibility confirmed (against the visualiser
source) that collapsing plan status→`done` breaks no reader/kanban consumer. Pass 6
found **1 critical** — exactly the systemic prediction: the status map covered
`plan`+`design-inventory` but `design-gap` and `plan-review` *also* carried
out-of-vocab `status: accepted`. **Fixed in-session by making the enumeration
exhaustive**: a full corpus scan of all 13 types against their `status_vocab`
confirmed only those two values were missing (`plan-review accepted→complete`;
`design-gap accepted` → widen vocab), and **all other types conform**. The
systemic "present-but-nonconforming base-field value" class is now closed *by
construction* (exhaustive scan + general principle + in-run self-validation
backstop), not by iteration — a pass 7 on this class would find nothing.

### Previously Identified Issues (pass-5 critical + majors — resolved)

- 🔴→✅ **Database**: out-of-vocab `status:` — **Resolved** via the general normalisation principle + an exhaustive, single-sourced legacy→canonical map (now covering plan, plan-review, design-gap, design-inventory, note; all other types verified conforming).
- 🟡→✅ Majors: normalise-before-seed; step-0 union covers all 32 backfilled files; list-valued side-channel anchor (now list-index-qualified, duplicate-safe); the `typed_work_item_target` test inverted + ~10 test call sites; date: coverage — all **Resolved**.

### New Issues (pass 6)

- 🔴→✅ **Database**: incomplete status enumeration (design-gap/plan-review `accepted`) — **Resolved in-session** (exhaustive scan; both now mapped; the map's catch-all `[unmapped-status]` should fire on nothing in this corpus).
- 🔵 Minor residuals (folded in-session where cheap): general-principle catch-all now has `[nonconforming-base-field]`/`[unmapped-status]` sub-reason tags and an explicit self-validation-backstop cross-reference; the status map's home is now named concretely (`scripts/status-legacy-map.tsv`) with a "every target ∈ status_vocab" invariant; quoting is clarified as enforced on `id:` only (unquoted `author:`/`title:` stay conforming); the list-valued side-channel anchor uses list-index (duplicate-safe).
- 🔵 Minor (not yet applied — implementation latitude): `_EXPECTED_CONFIG_SUITES` guard-fires checkbox; `update_reviews_by_target` covers both `:992`+`:993`; the design-gap `accepted` mapping (widen-vocab vs →draft) is flagged in the plan for the author's confirmation; the optional ADR note that disappearing legacy plan facet options are an expected (non-regression) consequence.

### Assessment — the plan is implementation-ready

Six adversarial passes have driven the plan from ten ship-blocking criticals to a
state where the only residuals are minor/latitude. The decisive outcome of this
pass is that the recurring critical class — *present base-field values that
predate the unified schema* — is now addressed **systemically**: a full
type×vocab scan enumerated every nonconforming value (statuses on plan/plan-review/
design-gap/design-inventory; date-only timestamps on 71 files; the lone draft
note), a general principle plus in-run self-validation backstop guards the
unenumerated case, and three latent corpus bugs the review flushed out (the
0032/0031 id collision, the 71 date-only files, the out-of-vocab statuses) are
each handled or REFUSE-guarded. The architecture, safety model, idempotency,
visualiser expand/migrate/contract sequencing, test strategy, and portability were
all source-verified across passes and concluded ready in their domains.

**Verdict COMMENT, not APPROVE**, only because two items legitimately remain the
author's calls rather than review defects: (1) confirm the `design-gap accepted`
mapping (widen-vocab, as written, vs →`draft`); (2) author the prerequisite
status-vocab ADR. Neither is a flaw in the plan; both are decisions the plan
correctly surfaces. With those confirmed, this is an APPROVE.

**Recommendation: stop here and proceed to implementation** (flip the plan to a
ready state once the two author-calls are settled). Further review passes have no
remaining critical/major surface to find on this plan.

---
*Re-review generated by /accelerator:review-plan*
