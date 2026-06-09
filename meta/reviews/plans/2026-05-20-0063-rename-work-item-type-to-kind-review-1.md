---
date: "2026-05-21T00:44:14+00:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-20-0063-rename-work-item-type-to-kind"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, safety, standards, compatibility]
review_pass: 4
status: complete
id: "2026-05-20-0063-rename-work-item-type-to-kind-review-1"
title: "2026-05-20-0063-rename-work-item-type-to-kind-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-21T00:44:14+00:00"
last_updated_by: Toby Clemson
---

## Plan Review: Rename work-item `type:` → `kind:`

**Verdict:** REVISE

The plan is structurally sound — sequencing, framework respect, idempotence
shape, and TDD discipline are all in good order — but it ships on a load-bearing
factual error: the claim that the eval corpus carries zero `^**Type**:` body
labels is contradicted by the codebase (≈104–105 fixture files carry them).
That error cascades through Phase 6's rewrite mechanism, the AC verification
greps, and the stress-test/create-work-item grader strings that reference
`**Type**:` literally, producing a plan that would pass its own AC oracle while
leaving the fixture corpus structurally inconsistent and at least one grader
suite asserting against vanished evidence. Secondary clusters of concern are
the two-step `atomic_write` sequence in the migration body, Phase 6's bypass
of the `atomic_write` primitive, and Phase 3's over-broad line enumeration
into tests that aren't actually about the kind field.

### Cross-Cutting Themes

- **Phase 6 body-label coverage gap** (flagged by: architecture, correctness)
  — The "Current State Analysis" and Phase 6 both assert zero `^**Type**:`
  body labels in eval fixtures; verification shows ≈104–105 files carry them.
  Phase 6 omits the body-label rewrite branch on that basis, and AC6/AC8 greps
  don't cover body labels outside `meta/work/`. Net effect: the plan would
  ship "green" while leaving 100+ stale `**Type**:` labels and at least one
  grader suite (stress-test-work-item) asserting against impossible evidence.

- **Migration uses two sequential `atomic_write` calls per file** (flagged by:
  code-quality, safety, correctness, standards, architecture, compatibility)
  — Six lenses independently flagged the same shape. Frontmatter and body
  rewrites run as separate `atomic_write` invocations, leaving a brief
  inconsistent on-disk state between writes and diverging from migration 0001's
  single-pass model. Idempotent re-run recovers, but the seam is real.

- **Phase 6 uses `> tmp && mv` instead of `atomic_write`** (flagged by: safety,
  standards) — Plan's Key Discoveries section calls `atomic_write` the
  preferred primitive, then Phase 6 doesn't use it. No EXIT trap for tmp-file
  cleanup on interruption either.

- **Phase 3 line enumeration overreaches into unrelated tests** (flagged by:
  code-quality, test-coverage) — Lines 690, 805/809/812/818/821, 1029 are
  literal-dot tests, field-name-agnostic reader tests, and a custom-template
  test exercising arbitrary user vocabularies. Blanket `type` → `kind` sweep
  erodes the original test rationale.

- **AC verification has a body-label blind spot** (flagged by: architecture,
  correctness) — AC8 checks `^\*\*Type\*\*:` only under `meta/work/`; nothing
  in the AC oracle catches stale body labels in fixtures or producer SKILL.md
  examples.

- **Eval-suite baseline capture is informally specified** (flagged by:
  test-coverage, safety, architecture) — "Save summary line(s) to a scratch
  file" with no path, no schema, no command. Phase 7's AC10 gate depends on
  it but can't be re-derived if lost.

- **`.cardType` CSS class deferral creates persistent naming inconsistency**
  (flagged by: architecture, code-quality, standards) — After rename, the
  visualiser has `kind`/`kindLabel` mapped through `styles.cardType`. Either
  bundle the rename (one CSS-module key + one JSX site) or pin a follow-up
  work-item.

### Tradeoff Analysis

- **Strict producer-corpus coupling (intra-repo) vs. upgrade window
  (downstream)**: The plan correctly bundles producer/migration/corpus inside
  this repo so no consumer sees a stale shape mid-PR. For downstream user
  repos, however, the same atomicity does NOT hold — plugin upgrade installs
  producer code immediately, but the corpus only gets migrated on the user's
  next `/accelerator:migrate`. Compatibility lens recommends a transitional
  `frontmatter['kind'] ?? frontmatter['type']` fallback or a dirty-tree
  diagnostic; safety/architecture/code-quality lenses prefer the cleaner
  one-shot rename. The user's stated preference is that VCS revert is the
  recovery path — but the upgrade-migrate gap is not a recovery problem, it's
  a UX one. Worth a deliberate decision rather than an implicit deferral.

- **Aggressive body-label rewriting (catches all forms) vs. precise
  rewriting (avoids prose mangling)**: The plan's body rewrite is
  unconditional on the regex, intentionally catching example/quoted lines.
  This is desirable inside the plugin's controlled corpus but risky in user
  repos where stories might legitimately use `**Type**:` for unrelated
  purposes. Either restrict to kind-vocabulary values, or document the
  breaking nature explicitly in changelog.

### Findings

#### Critical

- 🔴 **Architecture**: Phase 6 eval-fixture rewrite skips body labels despite 105 fixtures carrying them
  **Location**: Phase 6: Eval fixtures + grader strings — "Rewrite mechanism" section (lines 724-739)
  The plan asserts zero `^**Type**:` body labels in the eval corpus and omits the body-rewrite branch in Phase 6 on that basis. Direct inspection finds 105 fixture files carrying the body label, including every scenario-11b/c stub and every stress-test fixture. AC6/AC8 don't grep for body labels outside `meta/work/`, so the omission ships invisibly.

- 🔴 **Correctness**: Plan's "zero `**Type**:` body labels in eval corpus" premise is false — 104 fixture files carry the label
  **Location**: Current State Analysis (line 52) and Phase 6 §1 (lines 736-739)
  Same factual error from the correctness lens. Plan states twice that the eval corpus carries zero body labels; verification finds 104+ files. Phase 6's rewrite leaves them stale; ACs don't catch it.

- 🔴 **Correctness**: stress-test-work-item grader JSON references `**Type**:` body labels but is not in the Phase 6 edit set
  **Location**: Phase 6 §2 (Scope-lens grader strings, lines 742-759)
  `skills/work/stress-test-work-item/evals/{evals,benchmark}.json` contain grader strings asserting `**Type**:` body labels are preserved by the skill under test (evals.json:167; benchmark.json:185, 297-299), plus `create-work-item/evals/benchmark.json:552`. After Phase 2 + Phase 6 (with body labels rewritten), these graders assert against vanished evidence — eval failures in Phase 7 are likely.

#### Major

- 🟡 **Architecture**: AC verification greps do not cover the full producer/fixture surface for body labels
  **Location**: Phase 7: Final verification — "Verification commands"
  AC8 grep for `^**Type**:` is scoped to `meta/work/` only. No AC grep checks body labels across eval fixtures or producer SKILL.md examples. Combined with the Phase 6 omission, the verification net has a structural blind spot.

- 🟡 **Architecture**: Test harness setup helper omits the no-VCS escape used by the established 0004 pattern
  **Location**: Phase 1, Section 2: Test harness extensions (lines 219-234)
  Existing 0004 tests use `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1`. The plan's `setup_0005_repo` relies on implicit `vcs=""` branch behaviour. Inconsistent with prior art, fragile to future harness evolution.

- 🟡 **Code Quality**: Migration 0005 issues two `atomic_write`s per file instead of composing one final state
  **Location**: Phase 1, Section 3: Migration 0005 (migration body)
  Two sequential `atomic_write` calls (frontmatter, then body label) where 0001/0002 compose a single per-file write. Doubles temp-file churn, opens an interrupted-state window, diverges from canonical shape.

- 🟡 **Code Quality**: Idempotence shape diverges from canonical 0001 pattern
  **Location**: Phase 1, Section 3: Migration 0005 (idempotence pattern)
  Plan's "Manual Verification" claims the shape matches 0001 (outer guard + dual-presence sub-check). It doesn't — proposed 0005 uses two independent boolean flags + three branches. Inspection will contradict the success criterion.

- 🟡 **Code Quality**: Phase 3 test-edit list overreaches into unrelated test fixtures
  **Location**: Phase 3, Section 1: helper script test updates
  Lines 690, 805/809/812/818/821, 1029 are `sub.type` literal-dot tests and field-name-agnostic reader tests. Blanket sweep risks weakening dot-handling coverage and the "arbitrary field name" regression test.

- 🟡 **Test Coverage**: Phase 3 line enumeration sweeps in tests that don't test the kind field
  **Location**: Phase 3: Helper script — TDD (lines 432-438)
  Same concern from the test-coverage lens. Line 1029 ("User-overridden template with custom values") tests that field-hints accepts user-defined vocabularies; rewriting that fixture removes the coverage.

- 🟡 **Test Coverage**: Migration test fixtures omit empty-directory and missing-config edges
  **Location**: Phase 1: Migration 0005 — fixture scenarios (lines 188-210)
  No fixture for: work_dir-absent (early-exit guard), empty work_dir (zero `.md` files), or body-line false-positive (literal `type: story` inside a fenced code block). Migration's body-line false-positive behaviour is unspecified by the test suite.

- 🟡 **Test Coverage**: Eval-suite baseline oracle is too loose to be actionable
  **Location**: Phase 7: Eval-suite baseline comparison (AC10, lines 836-846)
  "Compare summary pass rates line-by-line" with an escape hatch ("attributed to flakiness") makes AC10 unfalsifiable. No specification of runs averaged, seed/temperature, or per-suite vs aggregate.

- 🟡 **Correctness**: Divergent-value partial-prior-run silently drops data
  **Location**: Phase 1 §3 Migration 0005 (lines 287-289)
  `grep -v '^type:'` unconditionally drops `type:` when both keys present, even if values diverge (e.g. `type: bug` + `kind: story`). The partial-prior-run fixture uses matching values, so this branch is never exercised.

- 🟡 **Correctness**: Silent exit 0 when `paths.work` resolves to a missing directory can mask misconfiguration
  **Location**: Phase 1 §3 Migration 0005 (line 277)
  Typo in `paths.work` → migration silent no-op, driver records 0005 as applied, producer code ships immediately, corpus never migrated. User gets no diagnostic.

- 🟡 **Correctness**: AC verification greps cannot detect stale `**Type**:` body labels in eval fixtures
  **Location**: Phase 7 Verification commands and AC8
  AC8 body-label check is `meta/work/` only. Combined with Phase 6 omission, failure mode invisible to oracle.

- 🟡 **Safety**: Phase 6 bash bypasses `atomic_write` and leaves no cleanup trap on interruption
  **Location**: Phase 6: Mass rewrite of fixture work-items
  Uses `> "$file.tmp" && mv "$file.tmp" "$file"` directly. Interruption leaves `.md.tmp` orphans; no EXIT trap. Diverges from `atomic_write` used by every migration.

- 🟡 **Safety**: Two sequential `atomic_write`s leave an inconsistent intermediate state on interruption
  **Location**: Phase 1, Section 3: Migration 0005 script body
  Same shape concern from the safety lens. Interrupted between frontmatter and body rewrite, file has `kind:` frontmatter but `**Type**:` body label — a state no reader expects.

- 🟡 **Standards**: Body-label rewrite does not follow the canonical dual-presence idempotence pattern
  **Location**: Phase 1, Section 3: Migration 0005 — body-rewrite branch (lines 282-296)
  Frontmatter branch checks for `^kind:` to handle dual-presence; body branch has no analogue for `^**Kind**:`. No fixture covers a file with both body labels.

- 🟡 **Standards**: Phase 6 rewrite uses `mv` instead of `atomic_write`, inconsistent with the migration it mirrors
  **Location**: Phase 6, Section 1: Mass rewrite of fixture work-items (lines 724-735)
  Plan's own Key Discoveries says `atomic_write` is preferred. Phase 6 ignores that.

- 🟡 **Compatibility**: User-repo gap between plugin upgrade and next `/accelerator:migrate`
  **Location**: Migration Notes / downstream user-repo impact
  Window between plugin upgrade (producer references `kind:`) and migration run (corpus still `type:`). Kanban cards lose kind chip; LLM-driven flows read stale schema. Dirty-tree guard extends the gap indefinitely if user has uncommitted `meta/` work.

- 🟡 **Compatibility**: Body-label rewrite unconditional on context can mangle unrelated prose in user repos
  **Location**: Phase 1, Section 3: Migration 0005 — unconditional body-label rewrite
  Any `^**Type**:` line in any work-item body gets rewritten. User repos using `**Type**:` for unrelated purposes (TypeScript discussions, HTTP content types) silently break.

- 🟡 **Compatibility**: Story 0070 still declares the `type:` → `kind:` rewrite as a requirement; no mechanism enforces removal
  **Location**: Dependencies section / Story 0070 coordination
  Story 0070 still lists "Renames work-item `type:` → `kind:`" as a requirement. No automated guard; reliance on human discipline across stories that may land months apart.

#### Minor

- 🔵 **Architecture**: User-repo rollout coupling is asymmetric — producer rewrite ships with migration, but user repos receive them in separate updates
- 🔵 **Architecture**: Migration 0005 reads file twice and rewrites it potentially twice — different shape from 0001
- 🔵 **Architecture**: Leaving `.cardType` CSS class out of scope creates a small but persistent naming-coherence debt
- 🔵 **Code Quality**: Two grep invocations per file when one suffices
- 🔵 **Code Quality**: Variable rename leaves `styles.cardType` orphaned from naming convention
- 🔵 **Code Quality**: Inline find/grep/sed sweep in Phase 6 duplicates the migration logic
- 🔵 **Code Quality**: Migration sources `atomic-common.sh` but only uses it conditionally (no comment block explaining the three branches)
- 🔵 **Test Coverage**: Idempotency assertion uses two different mechanisms with no chosen path — should commit to `tree_hash` per the 0002 precedent
- 🔵 **Test Coverage**: Pre-flight baseline capture described but not made automated or reproducible
- 🔵 **Test Coverage**: WorkItemCard test for missing-kind branch updates the title but not the assertion shape (still only `queryByText(/undefined/)`)
- 🔵 **Test Coverage**: JSON-grader oracle scoped only to scope-lens — could miss drift in other lenses
- 🔵 **Test Coverage**: No automated cross-skill integration test that `create-work-item` end-to-end still works
- 🔵 **Correctness**: Migration body-label regex can match within code fences and YAML literal blocks in work-item bodies
- 🔵 **Correctness**: AC2 grep does not catch legacy `type: adr-creation-task` references
- 🔵 **Correctness**: Two `atomic_write` calls per file double the FS churn but are functionally correct
- 🔵 **Correctness**: No fixture for `paths.work` set but pointing to a non-existent directory
- 🔵 **Safety**: Empty work_dir resolution could expand `find` scope to whole repo
- 🔵 **Safety**: Eval baseline capture informally specified and could be skipped
- 🔵 **Safety**: `set -euo pipefail` halts mid-corpus on any per-file failure with no partial-state report
- 🔵 **Safety**: Plan does not require committing Phase 1 before Phase 2 even though dirty-tree guard would not catch it
- 🔵 **Standards**: `setup_0005_repo` scenario-dispatcher diverges from `setup_old_repo` per-scenario convention
- 🔵 **Standards**: Fixture config-file convention is ambiguous — `.claude/accelerator.md` OR `.accelerator/config.md`
- 🔵 **Standards**: Table header rewrite changes column width without confirming alignment is preserved
- 🔵 **Compatibility**: Breaking schema change ships without a `schema_version` field
- 🔵 **Compatibility**: Custom userspace scripts that read `type:` from work-items will break silently
- 🔵 **Compatibility**: Two separate `atomic_write` calls per file create a brief inconsistent on-disk state

#### Suggestions

- 🔵 **Architecture**: Eval-suite baseline capture is described informally — no scripted artefact preserved
- 🔵 **Safety**: Phase 6 has no dry-run rehearsal against fixture data before touching the corpus
- 🔵 **Standards**: `.cardType` CSS class deferral creates a documented naming inconsistency

### Strengths

- ✅ Sequencing inside the atomic delivery is structurally correct — migration authored first, applied to corpus before any producer reads `kind:`, all in one PR.
- ✅ Migration 0005 honours all established framework contracts: `PROJECT_ROOT` via `config_project_root`, `paths.work` via `config-read-path.sh`, `atomic_write` for rewrites, no direct state-file mutation.
- ✅ Test fixtures use the established `test-fixtures/NNNN/<scenario>/` structure and `ONLY_NNNN_DIR` isolation pattern.
- ✅ Frontmatter-branch idempotence pattern mirrors `0001-rename-tickets-to-work.sh:53-69` (outer key guard + dual-presence sub-check).
- ✅ Explicit out-of-scope items (`.cardType` CSS class, agent prompts, `type: work-item-review`, grader-infrastructure `"type":` JSON) are acknowledged tradeoffs with stated reasons.
- ✅ Five-scenario fixture matrix covers most migration branches; idempotency is explicitly tested at multiple levels (Phase 1 test, Phase 2 re-run, Phase 7 final re-run).
- ✅ AC2 field-value pattern with `[^.\w]` boundary and kind-vocabulary alternation correctly excludes `entry.type` / `params.type` / TypeScript `type` keywords without explicit `-v` steps.
- ✅ Phase independence after Phase 2 is called out explicitly, reducing the cost of localised rework.
- ✅ Visualiser Rust backend is correctly identified as field-name-agnostic; no Rust changes needed.
- ✅ TDD-fail-then-pass cadence in Phases 1/3/4 produces evidence that tests actually exercise the new contract.
- ✅ Strong convention awareness on SKILL.md touch sites — surgical line-by-line itemisation with explicit DO-NOT-TOUCH carve-outs for `type: work-item-review`, TanStack `params.type`, and `.cardType` CSS class.

### Recommended Changes

Ordered by impact. The first three address the load-bearing factual error and
are required for the plan to ship correctly. The middle group consolidates the
migration's two-step rewrite into one. The remainder tighten verification,
test coverage, and documented compatibility.

1. **Re-inventory `^**Type**:` body labels across the eval corpus and add the
   body rewrite to Phase 6** (addresses: Phase 6 critical findings from
   architecture + correctness; the body-label-blind AC findings)
   Run `rg -c '^\*\*Type\*\*:' skills/**/evals/files/` and correct the plan's
   "Current State Analysis" and Phase 6 §1 text. Extend the Phase 6
   rewrite loop with `sed 's/^\*\*Type\*\*:/**Kind**:/'` on every fixture
   file. Add a Phase 6 / Phase 7 AC: `rg -n '^\*\*Type\*\*:' skills/work
   skills/review/lenses templates/ meta/work/` returns zero hits.

2. **Update the stress-test-work-item and create-work-item grader strings**
   (addresses: stress-test grader critical finding)
   Update every `**Type**` reference in `skills/work/*/evals/{evals,benchmark}.json`
   to `**Kind**`, plus `create-work-item/evals/benchmark.json:552`. Add an AC:
   `rg -n '\*\*Type\*\*' skills/work/*/evals/` returns zero hits.

3. **Consolidate the two `atomic_write` calls into one per file**
   (addresses: six lens findings on the dual-write seam)
   Replace the migration body's two-pass shape with a single
   `sed -e 's/^type:/kind:/' -e 's/^\*\*Type\*\*:/**Kind**:/' "$file" | atomic_write "$file"`
   when both rewrites apply. Keep the dual-presence (`type:` AND `kind:`) branch
   for partial-prior-run cleanup but collapse the body-label branch into the
   main rewrite. Update the "Manual Verification" claim that the shape matches
   0001 to reflect the actual structure.

4. **Use `atomic_write` in Phase 6 mass-rewrite** (addresses: safety + standards
   findings on Phase 6 bypass)
   Source `scripts/atomic-common.sh` in the Phase 6 bash and replace
   `> "$file.tmp" && mv "$file.tmp" "$file"` with `| atomic_write "$file"`.

5. **Carve out the `sub.type` literal-dot tests and the custom-template test
   from Phase 3's line enumeration** (addresses: code-quality + test-coverage
   overreach findings)
   Explicitly exclude lines 690, 805, 809, 812, 818, 821 (literal-dot
   `sub.type` regression tests for `work-item-read-field.sh`) and line 1029
   (custom-vocabulary template test) from the rewrite list. Document why each
   group is exempt.

6. **Add a body-label dual-presence sub-check to migration 0005**
   (addresses: standards finding on idempotence asymmetry)
   Mirror the frontmatter dual-presence guard for body labels: if both
   `^**Type**:` and `^**Kind**:` exist, drop the stale `**Type**:` line. Add
   a `partial-prior-run-body-label` fixture scenario.

7. **Add a divergent-value handling policy to the partial-prior-run branch**
   (addresses: correctness finding on silent data loss)
   Either fail loudly when `type:` and `kind:` values disagree, or document
   the "kind wins" policy in the migration header. Add a fixture covering
   divergent values.

8. **Add a non-empty guard on `work_dir_rel` and a misconfigured-paths.work
   diagnostic** (addresses: correctness + safety findings on silent
   misconfiguration)
   After resolving `work_dir_rel`, assert it is non-empty. If `paths.work` is
   set but the directory is missing, emit a stderr warning before exiting 0.
   Add a `paths-override-missing/` fixture.

9. **Tighten the Phase 2 / AC10 eval-suite baseline contract**
   (addresses: test-coverage + safety + architecture findings on informal
   baseline)
   Specify the baseline artefact path (e.g. `meta/scratch/0063-eval-baseline.txt`,
   gitignored), the exact commands to capture it, and the diff command Phase
   7 uses to compare. Pin a tolerance band for LLM-judge noise.

10. **Decide on the upgrade-migrate gap UX** (addresses: compatibility finding
    on user-repo gap)
    Either add a transitional `frontmatter['kind'] ?? frontmatter['type']`
    fallback in `WorkItemCard.tsx` for one release, or add a startup-time
    diagnostic when the visualiser detects unmigrated work-items. Document
    the choice in Migration Notes.

11. **Update story 0070's requirements to remove the duplicated rewrite step**
    (addresses: compatibility finding on enforcement gap)
    One-line edit to `meta/work/0070-...md` removing "Renames work-item
    `type:` → `kind:`" and pointing at migration 0005 as the owner. Acceptable
    scope creep for atomicity.

12. **Add the missing fixture scenarios** (addresses: test-coverage finding on
    edge-case fixtures)
    `empty-work-dir/` (directory exists, zero `.md` files), `body-line-false-
    positive/` (literal `type: story` in a code fence), and the divergent-
    value and paths-override-missing fixtures noted above.

13. **Decide on `.cardType` CSS class** (addresses: architecture + code-quality
    + standards minor findings)
    Either bundle the `.cardType` → `.cardKind` rename into Phase 4 (one
    CSS-module key + one JSX call site) or pin a follow-up work-item ID in
    "What We're NOT Doing".

14. **Strengthen the WorkItemCard missing-kind test assertion** (addresses:
    test-coverage minor finding)
    While editing the test in Phase 4, add an assertion that the kind chip
    element is absent from the DOM, not just that `/undefined/` text is
    absent.

15. **Use no-VCS env var in the migration test setup** (addresses: architecture
    minor finding)
    Mirror the 0004 pattern: invoke the test driver with
    `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` and optionally add a parallel
    dirty-tree refusal test.

16. **Standardise on `tree_hash` for the Phase 1 idempotency test** (addresses:
    test-coverage minor finding)
    Per the 0002 precedent in `test-migrate.sh:611-617`, drop the "or" hedge
    and use `tree_hash $REPO/meta` for the byte-identical assertion.

17. **Add a comment block above the migration's rewrite loop** (addresses:
    code-quality minor finding)
    Two-line comment explaining the three branches (rename, partial-prior-run
    cleanup, body-label only) and referencing
    `0001-rename-tickets-to-work.sh:53-69`.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Plan well-structured around a sound atomic-delivery strategy with
correct sequencing and framework respect, but contains a load-bearing
data-shape claim (zero `**Type**:` body labels in eval fixtures) that is
contradicted by the codebase. Consequences cascade through Phase 6's rewrite
mechanism and Phase 7 verification, leaving residual structural inconsistency
between producer and fixture corpora.

**Strengths**: Sequencing structurally correct (migration → corpus → producers
in one PR); framework contracts honoured; test fixture structure follows
0004's pattern; idempotence pattern mirrors 0001 precedent; out-of-scope items
acknowledged with rationale; phase independence after Phase 2 honest about
loose coupling.

**Findings**: 1 critical (Phase 6 body-label omission), 2 major (AC body-label
blind spot, no-VCS env-var omission), 3 minor (upgrade-migrate asymmetry,
two-write file shape, `.cardType` debt), 1 suggestion (informal eval baseline).

### Code Quality

**Summary**: Generally well-structured plan with minimal targeted edits and
direct reference to canonical migration model, but the 0005 migration body
diverges from the established idempotence pattern in three small but
meaningful ways and the helper-script tests update list overreaches into
unrelated `READ_FIELD`/`sub.type` fixture lines.

**Strengths**: Direct reference to canonical migration 0001:53-69; tight
pre/post-edit grep oracles; explicit DO-NOT-TOUCH guidance reduces scope
creep; phase independence acknowledged.

**Findings**: 0 critical, 3 major (two atomic_writes per file, idempotence
shape divergence, Phase 3 overreach), 4 minor (two grep invocations,
`styles.cardType` orphan, inline find duplicates logic, missing comment
block).

### Test Coverage

**Summary**: TDD-by-phase structure is solid for Phases 1/3/4, with the
migration fixture set covering most branches. Coverage gaps around
malformed-frontmatter/empty-corpus edges, incorrect line enumeration for
test-work-item-scripts.sh updates, and unstructured "compare summary pass
rates" Phase 7 oracle.

**Strengths**: Six explicit migration branch tests mirroring established
patterns; TDD fail-then-pass cadence with explicit assertion steps;
verification of kind-absent rendering branch preserved; layered Phase 1
success criteria (test exit + bash -n syntax + preview discovery); whole-repo
AC greps in Phase 7.

**Findings**: 0 critical, 3 major (Phase 3 line overreach, missing
empty-dir/malformed fixtures, loose eval oracle), 5 minor (idempotency
mechanism hedge, informal baseline capture, weak missing-kind assertion,
JSON-grader scope, no end-to-end integration test).

### Correctness

**Summary**: Mostly sound but contains a critical false premise (zero
`**Type**:` body labels in fixtures contradicted by 104 files), divergent
partial-prior-run silent data loss, stress-test grader strings missing from
Phase 6, and silent exit on misconfigured `paths.work`.

**Strengths**: Idempotence mirrors canonical 0001; five-scenario fixture
matrix covers most branches; body-label-only trace correct; sequential
`atomic_write` calls race-free; AC2 boundary pattern correctly excludes
unrelated `type:` forms; strict producer/consumer sequencing.

**Findings**: 2 critical (false body-label premise, stress-test graders
missing), 3 major (divergent partial-prior-run, silent exit, AC body-label
blind spot), 4 minor (regex match in code fences, legacy adr-creation-task
not in AC, two-write FS churn, missing paths.work-missing fixture).

### Safety

**Summary**: Operates within existing framework safety primitives but has
three notable gaps: Phase 6 bypasses `atomic_write`, the migration's dual
write leaves brief inconsistent intermediate state, and work_dir resolution
lacks a non-empty guard. All risks have viable VCS-revert recovery paths.

**Strengths**: Relies on framework's dirty-tree guard and state-file append;
uses `atomic_write` in the migration; idempotency explicitly tested and
verified at runtime; eval-suite baseline captured before destructive change;
VCS-backed recovery viable in jj-colocated repo; dual-presence guard handles
partial-prior-run; phase independence reduces blast radius.

**Findings**: 0 critical, 2 major (Phase 6 bypasses atomic_write, dual-write
inconsistency), 4 minor (empty work_dir → whole-repo blast, informal
baseline, pipefail mid-corpus halt, Phase 1 not committed before Phase 2),
1 suggestion (no Phase 6 rehearsal).

### Standards

**Summary**: Adheres to most established conventions but several
inconsistencies surface: migration 0005's body-rewrite block doesn't follow
the framework's idempotence guard convention; Phase 6's mass-rewrite bypasses
`atomic_write`; test-harness extension introduces a divergent setup-helper
naming pattern.

**Strengths**: Migration shebang/DESCRIPTION/set-flags/source-order/PROJECT_ROOT
fallback all match canonical 0001/0002/0004 model; preserves
artifact-type-discriminator references; surgical SKILL.md line-itemisation
with DO-NOT-TOUCH carve-outs; TDD ordering consistent with executable-test
surfaces; Phase 2 verification commands match AC greps verbatim.

**Findings**: 0 critical, 2 major (body-rewrite idempotence pattern, Phase 6
mv vs atomic_write), 3 minor (setup-helper dispatcher convention, fixture
config-file ambiguity, table-header width), 1 suggestion (`.cardType`
deferral inconsistency).

### Compatibility

**Summary**: Executes a breaking rename. Within plugin repo correctly bundles
producer/consumer/corpus migration in single atomic delivery, and migration
framework's numeric ordering ensures correct application in fresh user repos.
However, downstream user-repo impact is understated: kind-display gap between
plugin upgrade and next migrate, unconditional body-label rewrite can mangle
unrelated prose, story 0070's overlapping requirement is enforced only by
human discipline.

**Strengths**: Sequencing inside atomic delivery correct; migration follows
framework conventions; visualiser Rust backend wire-compatible
(`serde_json::Value`); test matrix covers compatibility surface including
`paths.work` override; false-positive `type:` references identified and
excluded.

**Findings**: 0 critical, 3 major (upgrade-migrate gap, unconditional body
rewrite, story 0070 enforcement), 3 minor (no schema_version, userspace
scripts break silently, two atomic_writes intermediate state).

---

## Re-Review (Pass 2) — 2026-05-20

**Verdict:** REVISE

The revision was substantial and surgical: all 3 critical findings from pass 1
are resolved, every cross-cutting theme is addressed, and most major findings
are gone. However, the new migration body uses `eval "$pipeline"` to dispatch
a string-built shell pipeline — **four independent lenses flagged this as a
major concern**, and a related correctness issue (`set -euo pipefail` +
`grep -v` aborting the migration on a degenerate file) is also new. Both
collapse to one focused change: replace the string-and-eval pattern with
explicit sequential `atomic_write` passes following the 0001 model. With that
fix, the plan is in shape to approve.

### Previously Identified Issues

#### Critical (3) — all resolved

- ✅ **Architecture**: Phase 6 eval-fixture body-label omission — **Resolved**. Phase 6 §1 now uses the same dual-presence pipeline as 0005; AC adds body-label sweep across `skills/`, `templates/`, `meta/work/`.
- ✅ **Correctness**: False "zero `**Type**:`" premise — **Resolved**. Current State Analysis corrected to 104; Phase 6 rewrite covers body labels.
- ✅ **Correctness**: stress-test/create-work-item grader strings — **Resolved**. New Phase 6 §3 enumerates the nine `**Type**` grader occurrences with file/line refs; AC checks `rg '\*\*Type\*\*' skills/work/*/evals/*.json`.

#### Major (≈18) — 16 resolved, 2 partially resolved

- ✅ **Architecture**: AC body-label blind spot — **Resolved** (whole-repo body-label sweep added).
- ✅ **Architecture**: No-VCS escape in test harness — **Resolved** (`ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` documented).
- ✅ **Code Quality**: Two `atomic_write`s per file — **Resolved** (single composed pipeline).
- 🟡 **Code Quality**: Idempotence shape divergence from 0001 — **Partially resolved**. The plan now claims fidelity to 0001 via header comment, but the actual shape (string-built pipeline + eval) is structurally further from 0001, not closer. See new finding below.
- ✅ **Code Quality**: Phase 3 overreach into unrelated tests — **Resolved** (carve-outs documented for lines 690, 805/809/812/818/821, 1029).
- ✅ **Test Coverage**: Phase 3 line enumeration — **Resolved** (same carve-out).
- ✅ **Test Coverage**: Missing empty-dir/malformed fixtures — **Resolved** (`empty-work-dir`, `paths-override-missing` added; body-line false-positive deliberately documented as accepted behaviour).
- ✅ **Test Coverage**: Loose eval-suite oracle — **Resolved** (±3% tolerance + no PASS→FAIL flip + per-fixture diff).
- ✅ **Correctness**: Divergent partial-prior-run silent data loss — **Resolved** ("kind wins" policy documented; `partial-prior-run-divergent` fixture added).
- 🟡 **Correctness**: Silent exit on missing `paths.work` — **Partially resolved**. Explicit-override case now warns; default `meta/work` missing case is still silent. See new finding below.
- ✅ **Correctness**: AC body-label blind spot — **Resolved**.
- ✅ **Safety**: Phase 6 bypasses `atomic_write` — **Resolved**.
- ✅ **Safety**: Two `atomic_write`s inconsistent state — **Resolved**.
- ✅ **Standards**: Body-label idempotence pattern — **Resolved** (dual-presence handling now symmetric).
- ✅ **Standards**: Phase 6 `mv` vs `atomic_write` — **Resolved**.
- ✅ **Compatibility**: Upgrade-migrate gap — **Resolved** (documented in Migration Notes, CHANGELOG checklist).
- ✅ **Compatibility**: Unconditional body-label rewrite mangling — **Resolved** (called out in breaking-change notes).
- ✅ **Compatibility**: Story 0070 enforcement — **Resolved** (0070 updated in-place to carve out the rename).

#### Minor (≈26) — most resolved; remainder either resolved or deliberately deferred

- ✅ Most minor findings addressed (CSS class rename, comment block, `tree_hash`, scripted baseline, strengthened missing-kind assertion, repo-wide grader grep, `set -euo pipefail` summary line, Phase 1 commit-first requirement, fixture config-file standardisation, legacy-kind sanity check, paths-override-missing fixture).
- ⏸️ **Compatibility**: No `schema_version` field — **Still present, deliberately deferred to story 0070** per the user's "accept and document" decision. Documented as deferred in the re-review.
- ⏸️ **Test Coverage**: No automated cross-skill integration test — **Still present, deferred**. Manual verification step 3 is the gate.
- ⏸️ **Safety**: No Phase 6 dry-run rehearsal — **Still present, deliberately deferred** per the project's "VCS revert is the recovery path" preference.

### New Issues Introduced

- 🟡 **Code Quality / Safety / Standards / Correctness**: `eval "$pipeline"` over a find-discovered filename is a code smell, a safety hazard, and divergent from every existing migration. The pipeline is built as a string (`pipeline="cat \"\$file\"" ; pipeline+=" | grep -v ..."`) then dispatched via `eval`. Four lenses independently flagged this. Issues:
  - **Safety**: A filename containing `"`, `` ` ``, or `$(...)` would be interpreted as shell code when expanded under `eval`. The fixture corpus is safe; user repos are not — `paths.work` content is user-controlled.
  - **Standards**: A repo-wide search (`rg '\beval\b' scripts/ skills/config/migrate/`) finds zero precedent. Migrations 0001/0002/0004 all use direct command pipelines.
  - **Code Quality**: The double-parse of `\"\$file\"` is harder to reason about than direct quoted expansion; `eval` invites less-careful copies in future migrations.
  - **Correctness**: When `eval` is paired with `set -euo pipefail` (see next finding), the failure modes compound.

- 🟡 **Correctness**: `set -euo pipefail` + `grep -v` in the eval'd pipeline can abort the migration on a degenerate file. `grep -v '^type:'` returns exit code 1 when **every** input line matches (or input is empty after upstream filtering), and `pipefail` propagates that failure. A pathological work-item file (single `^type:` line plus delimiter, or an upstream filter that empties the stream) would halt the migration mid-corpus. The same construct ships into Phase 6's 137-file fixture sweep.

- 🔵 **Architecture / Correctness**: Default `meta/work` missing case is still silent. The warning condition is `if [ "$work_dir_rel" != "meta/work" ]`. A user with no config and a fresh checkout, or with explicit `paths.work: meta/work` (matching the default) on a deleted directory, gets no diagnostic. Either drop the gate (warn unconditionally) or distinguish "default fallback" from "explicit value" via the resolver's signal rather than a string comparison.

- 🔵 **Architecture**: Divergent-value reconciliation has no observable signal. When `type: bug` + `kind: story` are both present, the migration drops `type:` silently. Documented in header comment + test name, but a real divergence almost certainly indicates upstream data corruption — suppressing the signal architecturally prevents users from noticing. Emit a single stderr line on divergence detection.

- 🔵 **Correctness**: Phase 6 mass-rewrite has no `set -euo pipefail`, no `rewrote` counter, and no per-file failure summary — inconsistent with the migration's Phase 1 pattern.

- 🔵 **Correctness**: Divergent **body-label** values (e.g. `**Type**: Bug` + `**Kind**: Story`) are not fixtured. The pipeline handles them identically to the frontmatter divergent case, but the policy isn't tested for the body channel.

- 🔵 **Standards**: Missing-`paths.work` warning uses a raw `echo ... >&2` rather than the `log_warn` helper used by migration 0004 (via `scripts/log-common.sh`).

- 🔵 **Standards**: DESCRIPTION header line for 0005 is longer than the 0001-0004 convention (single short clause). The detailed policy block can stay in the body comment; the DESCRIPTION should match the preview-banner shape.

- 🔵 **Test Coverage**: `partial-prior-run-divergent` and `body-label-only` test assertions are described in prose but not codified as concrete `assert_*` calls. The divergent test in particular needs an explicit value assertion (`kind: story` survives, not `kind: bug`) to actually exercise the "kind wins" policy.

- 🔵 **Code Quality**: Migration now does four grep invocations per file (one per flag); 0001 uses two. More invocations than before, not fewer.

- 🔵 **Compatibility**: Visualiser dev server needs restart after `/accelerator:migrate` runs — call out in the breaking-change notes alongside the dirty-tree guard.

### Assessment

The plan is **close to approval** — every load-bearing concern from pass 1
(false fixture premise, missing grader updates, fixture coverage gaps, ACs
blind to body labels, two-write inconsistent intermediate state, Phase 6
atomicity bypass, broken story 0070 dependency) is resolved. The remaining
concern is concentrated on a single new code construct (`eval "$pipeline"`)
that four independent lenses surfaced.

**Recommended path to approve:** Replace the string-built-pipeline +
`eval` with explicit sequential `atomic_write` passes mirroring the 0001
shape — one pass for the frontmatter, one pass for the body label, each
gated by its own dual-presence check. Two `atomic_write` calls per file
is acceptable here (each is atomic on its own; no concurrent reader; the
inconsistent-intermediate-state concern is bounded by the same idempotency
guards that already cover interrupted runs). This both eliminates the
eval safety/code-quality concerns and brings the migration into closer
textual alignment with the canonical 0001 model the plan cites. The
pipefail+`grep -v` abort risk is automatically resolved by the same change
(no shared pipeline, each stage's exit is observable).

Other minor concerns (default-missing warning, divergent-value stderr,
Phase 6 set-e + counter, divergent body-label fixture, `log_warn` helper,
DESCRIPTION line shortening, codified assertions, dev-server-restart note)
are inexpensive cleanups that can be folded into the same revision.

---

## Re-Review (Pass 3) — 2026-05-20

**Verdict:** COMMENT — plan is acceptable as-is; remaining items are
optional cleanups

Pass 3 confirms the pass-2 fixes landed cleanly. All four major findings
from pass 2 (eval-pipeline as code smell, eval over filenames as safety
hazard, eval diverging from migration conventions, pipefail+`grep -v`
abort risk) are resolved by the switch to two sequential `atomic_write`
passes. **Two lenses (safety, standards) returned zero findings.** No
new criticals or majors anywhere.

### Previously Identified Issues

#### Major (pass 2) — all resolved

- ✅ **Code Quality**: eval pipeline construction — **Resolved**. Replaced with two sequential `atomic_write` passes mirroring 0001:53-69 line-by-line.
- ✅ **Safety**: eval over find-discovered filenames — **Resolved**. No more eval; `$file` is now a normal quoted parameter expansion throughout.
- ✅ **Standards**: string-built pipeline diverging from existing migrations — **Resolved**. New shape matches 0001 precedent + uses `atomic_write` from 0002-0004 convention + uses `log_warn`/`log_die` from 0004 convention.
- ✅ **Correctness**: `set -euo pipefail` + `grep -v` abort on degenerate file — **Resolved**. Each `grep -v ... | atomic_write` is gated by an outer guard + inner dual-presence check, guaranteeing the inverse-grep output is non-empty.

#### Minor (pass 2) — most resolved; remainder consciously deferred

- ✅ Default-missing `meta/work` warning, divergent-value stderr signal, divergent body-label fixture, Phase 6 `set -euo pipefail` + counter, Phase 6 dry-run rehearsal note, raw-echo → `log_warn`, DESCRIPTION line shortening, codified test assertions, dev-server-restart note — **all resolved**.
- ⏸️ **Architecture / Code Quality**: Phase 6 logic duplication — still present but acceptable. Both copies updated in lockstep; pass 3 reaffirms this is workable.
- ⏸️ **Architecture**: `--summary` flag unverified in baseline-capture script — still present; deferred.
- ⏸️ **Correctness**: Body-label regex matches inside code fences / YAML literals — still present, documented as accepted behaviour ("rewrite is unconditional on the regex").
- ⏸️ **Compatibility**: No `schema_version` field — deferred to story 0070 per user decision.

### New Issues Introduced

All new findings are **minor / suggestion only**. Grouped by whether the
team would benefit from folding them in or whether they can ship as-is:

#### Worth folding in (low cost, real value)

- 🔵 **Correctness** (high confidence): Body-label divergence is silently dropped without a `log_warn`, asymmetric with frontmatter divergence. Pass 1 emits `divergent type/kind in $file — kept kind=…, dropped type=…`; Pass 2 (body label) does not. The `partial-prior-run-body-label-divergent` fixture asserts the silent drop but doesn't assert any stderr signal. **Mirror the divergence-detection block from Pass 1 into Pass 2** and add a stderr assertion to the test.
- 🔵 **Correctness** (medium confidence): `work_dir_rel` is checked for empty string but not for `.`, `/`, `..`, or absolute paths. A user setting `paths.work: .` would expand `find` scope to the entire repo. **Add `case "$work_dir_rel" in .|..|/|/*) log_die …;; esac`** after the empty-string guard.
- 🔵 **Test Coverage** (medium confidence): The matching-values `partial-prior-run` test should add `assert_not_contains "$stderr" 'divergent type/kind'` — locks in the silent-on-match contract and catches the mutation "always warn on partial prior run".
- 🔵 **Test Coverage** (medium confidence): `empty-work-dir` `tree_hash` assertion is trivially satisfied because there are no files. Tighten to `find docs/work -type f | wc -l` equals 0 AND assert `0005: rewrote 0 file(s) under docs/work` appears in stdout.

#### Acceptable to defer

- 🔵 **Correctness**: Divergence comparison captures unstripped trailing content — a `type: story    # legacy` + `kind: story` produces a false-positive divergence warning. Real-world impact is low (production corpus is bare YAML); harden the trim if needed.
- 🔵 **Code Quality**: `touched` flag readability — rename to `file_changed` or add a one-line comment explaining "count once per file, not per pass". Nit.
- 🔵 **Architecture**: Phase 6 duplicates 0005's logic — acknowledged. Optional refactor to a shared helper script if maintenance burden grows.
- 🔵 **Test Coverage**: Phase 6 mass-rewrite is not test-gated (the same code lives in the migration's test suite). Pre-flight grep asserting no fixture file has both `^type:` and `^kind:` would close the gap cheaply.
- 🔵 **Correctness**: Pass 2 drops every `^**Type**:` line in dual-presence cases (multiple stale labels handled identically) — documented behaviour, no fixture covers the multi-label case.

### Assessment

The plan is **ready to implement**. All structural and shape-fidelity
concerns from passes 1 and 2 have been resolved. Pass 3's remaining
findings are minor — the most valuable cluster is the four "worth
folding in" items above, all of which are surgical edits (one log_warn
addition, one case-statement guard, two added test assertions). Verdict
is COMMENT rather than APPROVE because these are still genuine
improvements, but none are blocking — the plan can ship as-is and the
items can be addressed in a follow-up commit or during implementation.

**Lenses returning zero findings**: safety, standards.

---

## Re-Review (Pass 4) — 2026-05-21

**Verdict:** APPROVE

The four "worth folding in" items from pass 3 have been addressed:

1. ✅ **Body-label divergence symmetry** — Pass 2 now extracts and compares `**Type**:` / `**Kind**:` values via `grep -m 1` + `sed`, emits a `log_warn` line on divergence (`divergent **Type**/**Kind** body label in $file — kept Kind=…, dropped Type=…`).
2. ✅ **`work_dir_rel` pathological-value guard** — Added a `case "$work_dir_rel" in .|..|/|/*|*/..|../*|*/../*) log_die …;; esac` block immediately after the empty-string guard. Refuses `.`, `..`, `/`, absolute paths, and parent-traversal escapes.
3. ✅ **Negative stderr assertions** — Added to both `partial-prior-run` (frontmatter matching values) and `partial-prior-run-body-label` (body-label matching values) tests. The body-label-divergent test gets a positive stderr assertion.
4. ✅ **Empty-work-dir assertion tightened** — Replaced trivial `tree_hash byte-identical` with `find docs/work -type f | wc -l` equals 0 AND stdout contains `0005: rewrote 0 file(s) under docs/work`.

No new findings introduced; the plan is approved as-is and ready for
implementation.
