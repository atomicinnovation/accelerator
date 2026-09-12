---
type: "plan-review"
id: "2026-09-13-0278-complete-codebase-research-rename-review-1"
title: "Plan Review: Complete the Codebase-Research Rename Implementation Plan"
date: "2026-09-13T07:38:55+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-13-0278-complete-codebase-research-rename"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "documentation", "standards"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-14T13:25:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Complete the Codebase-Research Rename Implementation Plan

**Verdict:** REVISE

The plan rests on a genuinely sound spine — the five-namespace boundary table
correctly separates the moving codebase-research doc type from load-bearing
umbrella namespaces and preserves the parser-coupled `## Related Research`
heading and cross-layer `hasResearch` field, and the phase decomposition with
TDD ordering is well-judged. The rename mechanics (Tiers 0–4) are accurate and
well-referenced. The concentration of findings is on the one substantive,
destructive piece: `m0010` is under-specified and under-tested relative to the
`m0006` precedent it claims to mirror, its test plan misses independent branches
and the anchoring invariant, the destructive-migration safety guards from
`m0006` are dropped, and the "either order is safe" framing for Phases 3–4 is
wrong for data consistency. Eight major findings clear the REVISE threshold of
three; none is structural enough to invalidate the approach.

### Cross-Cutting Themes

- **`m0010` is under-specified and under-tested** (flagged by: Test Coverage,
  Code Quality, Correctness, Safety, Standards) — the migration's transform is
  not designed as an isolated pure function, has no unit-level tests, misses
  independent title-only / H1-only / post-`## ` anchoring cases, drops `m0006`'s
  dangerous-path guard and operator diagnostics, and carries a policy-violating
  what-comment in its `apply()` sketch. Five lenses converge on the same module.
- **Completeness gaps in a plan whose whole purpose is completeness** (flagged
  by: Documentation, Test Coverage) — a doc-type enumeration is missed inside a
  file the plan already edits (`visualiser.md:22`), the Phase 4 §5 ledger table
  understates the per-file edits (inline seeds + a byte-for-byte golden
  assertion), an api_smoke.rs coverage claim is factually wrong, and Phase 1
  omits the `docs:generate` note its sibling phases include.
- **Destructive-migration operational safety** (flagged by: Safety,
  Architecture, Correctness) — a 177-file in-place rewrite has no dry-run or
  mandated diff review, an unbounded blast radius if `paths.research_codebase`
  is misconfigured, and a partial-failure/resume path relied on but neither
  stated nor tested.

### Tradeoff Analysis

- **Robustness (mirror `m0006`) vs simplicity (fit the verified corpus)**: the
  Correctness lens verified the corpus is 100% double-quoted titles
  (177/177), so `m0006`'s single-quote/unquoted/normalise branches would ship
  exercised by no test. Either keep the general quote-aware machinery for
  authorship consistency **and** add unit tests for every branch, or scope the
  transform to the double-quoted shape and drop the dead branches (YAGNI). Do
  one or the other — the current plan keeps the machinery untested, the worst of
  both. Recommendation: keep the general helpers (consistent migration
  authorship) and add the branch tests, since a one-shot data migration is
  exactly where latent branch bugs are unrecoverable.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Test Coverage, Code Quality**: m0010 transform has no pure-function or
  unit-level tests; quote-variant and near-miss branches ship untested
  **Location**: Phase 4, items 1 & 4
  The cited precedent `m0006` extracts a pure `transform` and carries ~20
  in-module unit tests (quote normalisation, embedded-quote escaping, unquoted
  values); m0010 proposes two coarse binary-spawning tests over a transform
  inlined in `apply()`. Breaking the quote handling or matching `Researchers:` /
  `## Research:` would fail no assertion.

- 🟡 **Test Coverage**: m0010 apply tests omit independent title-only / H1-only
  cases and the post-`## ` anchoring guard
  **Location**: Phase 4, item 4 / Testing Strategy
  The proposed test puts both a prefixed frontmatter title and a prefixed body
  H1 in one document. Frontmatter and body are independent branches, so a
  coupled implementation (strips H1 only when the title also matched) or an
  anchoring bug (rewrites a `# Research:` after the first `## `) passes the
  single combined test yet corrupts real docs. `m0006` explicitly tests the
  analogous `..._before_first_h2_only` case.

- 🟡 **Test Coverage**: Phase 4 §5 ledger table understates the per-file edits
  **Location**: Phase 4, item 5
  "Add `0010-…` to each `already_applied*`/seed list" is too terse:
  `migration_0002.rs` writes the ledger inline in six tests, `migration_0006.rs`
  in three, and `migration_0001.rs`'s `matches_the_golden_byte_for_byte` asserts
  the resulting ledger byte-for-byte — an assertion, not a seed. Following the
  guidance literally risks count/golden mismatches that are hard to trace.

- 🟡 **Architecture**: "Either order" pairing of the one-shot m0010 with a
  still-emitting template risks a permanently mixed corpus
  **Location**: Implementation Approach (phase-dependency note) / Phases 3–4
  The plan blesses shipping Phase 4 before Phase 3. `m0010` is applied-once; if
  it runs before the template stops emitting the prefix, the template keeps
  minting `Research: `-prefixed docs the migration will never revisit, leaving a
  self-perpetuating split corpus with no remedy short of a further migration.
  True for CI-green, false for end-state data consistency.

- 🟡 **Safety**: No dry-run/preview or mandated diff review before the 177-file
  destructive rewrite
  **Location**: Phase 4 Manual Verification / Migration Notes
  Post-apply verification is a frontmatter grep and a second-run no-op; no step
  directs the operator to inspect the full `jj diff` before committing, and the
  plan actively normalises the dirty tree ("that is the migration working"). A
  subtly-wrong transform silently rewrites up to 177 files and the framing
  invites committing without review — turning a reversible mistake into a
  committed one.

- 🟡 **Safety, Code Quality**: m0010 drops m0006's dangerous-path guard and
  operator diagnostics; the preflight dirty-scan scope is fixed
  **Location**: Phase 4, item 1
  The sketch resolves `paths.research_codebase` but omits `m0006`'s
  `is_dangerous_path` refusal (empty/`/`/absolute/`../`) and its
  rewrite-count/missing-dir `eprintln!` diagnostics. The dirty-tree preflight
  scans a hardcoded `meta/`/`.accelerator/` set, so a corpus configured
  elsewhere would be rewritten with no dirty-tree protection at all — an
  unbounded, unprotected blast radius on misconfiguration.

- 🟡 **Documentation**: Phase 5 §4 edits `visualiser.md:13` but misses the
  equivalent doc-type enumeration nine lines later
  **Location**: Phase 5, item 4
  `visualiser.md:22` ("a rendered-Markdown reader over every document type in
  `meta/` — plans, research, ADRs, …") uses "research" as a doc type exactly as
  line 13 does. The same page will read "codebase research" at line 13 and bare
  "research" at line 22 for the identical concept — an inconsistency the edit
  itself introduces, which `docs:check` will not catch.

- 🟡 **Code Quality, Standards**: The `apply()` sketch carries a what-comment
  that would violate the comments policy if it survives
  **Location**: Phase 4, item 1
  `// resolve, guard dir_exists, walk list_md_files, rewrite+write` narrates the
  steps the code performs. CLAUDE.md mandates a very low tolerance for comments
  and to actively strip them from plans; `m0006` self-documents through named
  helpers instead. Implementers following the template verbatim would seed a
  policy-violating comment in shipped code.

#### Minor

- 🔵 **Correctness**: Body-H1 anchoring is positional, not fence-aware
  **Location**: Phase 4, item 1 / Migration Notes
  The plan claims the rule "never touch[es] `#`-comment lines inside code
  fences," but `m0006`'s `extract_pre_h2`/`saw_first_h2` mechanism only tracks
  the first `## ` — any column-0 `# Research: X` before it (a pre-`## ` fenced
  block, or the template's own `# typed-linkage slots` frontmatter comment)
  would be rewritten. Add an `!in_frontmatter` guard and place the fence test in
  the pre-`## ` region so it exercises the real mechanism.

- 🔵 **Correctness**: No content pre-gate — reconstruction could spuriously
  rewrite prefix-free files
  **Location**: Phase 4, item 1
  "Write only when changed" does not carry over `m0006`'s content pre-gate. If
  m0010 reconstructs via `lines().join("\n")` before comparing, that normalises
  line endings and can differ from the original for a prefix-free file, breaking
  the "leave non-matching titles byte-identical" guarantee. Add an early return
  for files whose title and pre-`## ` H1 lack the prefix.

- 🔵 **Test Coverage**: api_smoke.rs asserts template count, not name — the
  Phase 1 §4 claim is wrong
  **Location**: Phase 1, item 4
  api_smoke computes `expected_templates` by counting `.md` files at runtime and
  asserts only the count; it references no bare `research` name. Renaming one
  fixture keeps the count, so there is nothing to update and `--test api_smoke`
  does not name-verify the rename. Name-level coverage lives only in items 3 & 5.

- 🔵 **Test Coverage**: No direct m0010 test for "missing corpus dir still
  records Applied"
  **Location**: Phase 4, item 4 / Testing Strategy
  `full_registry_e2e`'s `applied: 9` and migration_0008's second-run guard both
  depend on m0010 returning `Applied` even when the corpus dir is absent, but
  m0010's own tests only use a populated repo. If m0010 no-op-pended on a missing
  dir, its own tests pass while the aggregate tests fail with opaque count
  mismatches. The existing suite pins this per migration.

- 🔵 **Test Coverage**: The idempotency test must bypass the ledger gate
  **Location**: Phase 4, item 4
  A second `accelerator migrate` run will not re-execute m0010 (the ledger
  records it applied), so a naive two-run test verifies the gate, not the
  transform. Model it on `m0006`'s byte-stability test: run, drop 0010 from the
  ledger, re-run, assert byte-identical.

- 🔵 **Test Coverage**: Display-copy and "stop emitting prefix" changes have no
  automated test
  **Location**: Phase 2 item 3 / Phase 3
  Phase 2 §3 (empty-state plural) names no test, and Phase 3 (template + skill
  stop emitting the prefix) is verified only manually. A regression reintroducing
  the `Research: ` prefix — the most visible leak the plan targets — would pass
  `mise run check`. Consider a lightweight test asserting the template's
  title/H1 carry no prefix.

- 🔵 **Architecture**: Test ledgers mirror registry membership rather than
  deriving it
  **Location**: Phase 4, item 5
  ~13 files hardcode the full ledger and aggregate counts, so every future
  migration is O(n) shotgun surgery where a missed seed silently mis-isolates a
  test. Out of scope to fix here; worth capturing as a follow-up (derive seeds
  from `registry()` via an all-ids-except-target helper) and refreshing stale
  doc comments (`full_registry_e2e.rs:1` "7-migration registry").

- 🔵 **Architecture**: Overlapping edits qualify the "independently mergeable"
  claim
  **Location**: Implementation Approach; Phase 1 §1 & Phase 5 §3
  `configure/SKILL.md` is edited in Phase 1 (877, 930) and Phase 5 (100, 884);
  `types.ts` display copy pairs with its test across Phase 2. The phases are
  CI-independent but not conflict-free on parallel branches. Clarify that
  independence is scoped to CI-green with sequential application.

- 🔵 **Architecture, Safety**: The single-pass 177-file rewrite stresses the
  partial-failure/resume path, which is relied on but untested
  **Location**: Phase 4, item 1 / Migration Notes
  A mid-walk `ctx.write` failure leaves files 1..N-1 rewritten, the migration
  pending, and the tree dirty — which the preflight then refuses on retry.
  Recovery genuinely rests on atomic per-file writes + manifest resume +
  idempotency, but the plan neither states nor tests it. Note the recovery path
  and add an apply-over-partially-migrated-corpus test.

- 🔵 **Code Quality**: Record why m0006's helpers are duplicated inline
  **Location**: Phase 4, item 1
  Inline duplication of `is_double_quoted`/`semantic_inner`/`extract_pre_h2` is
  defensible (migrations are frozen; `text.rs` is public-API-tracked) but the
  plan states the decision without the reasoning. A future DRY pass could
  "fix" it by touching frozen migrations. Add one sentence recording the
  rationale.

- 🔵 **Standards**: 80-column and table-alignment drift in the docs-site edits
  **Location**: Phase 5, item 4
  Inserting "codebase " pushes `configuration.md:70` past 80 columns, and
  widening the `visualiser.md:13` table cell overruns its dash padding. No
  markdown formatter guards this, so `check`/`docs:check` stay green while the
  docs drift from the documented convention. Rewrap the paragraph and re-pad the
  table row and separator.

- 🔵 **Documentation**: Phase 1 omits the `docs:generate` mirror-regeneration
  note its sibling phases include
  **Location**: Phase 1
  Phase 1 edits `configure/SKILL.md` (a mirrored skill) but, unlike Phases 3 and
  5 — which edit the same file — omits the `mise run docs:generate` note and
  success criterion. Mirrors are gitignored, so nothing signals the staleness on
  an independent Phase 1 merge.

- 🔵 **Documentation**: Desired End State grep contradicts the Migration Notes
  **Location**: Desired End State / Phase 4 Migration Notes
  Desired End State says `grep -rn 'Research: '` over `meta/research/codebase/`
  "returns nothing after migration," but the Migration Notes say three embedded
  template examples are deliberately never touched, so the broad grep still
  matches them. Use the narrower `grep -rn 'title: "Research: '` (and an anchored
  `'^# Research: '` for the H1).

- 🔵 **Documentation**: `research-issue/SKILL.md:23` keeps a bare "Research
  directory" with no note
  **Location**: Phase 5, item 1
  After renaming the five `research_codebase` labels, one anomalous "Research
  directory" (resolving `research_issues`) remains, out of scope per the
  sibling-corpora boundary. Add a "What We're NOT Doing" line so the residual
  reads as a deliberate deferral, not a miss.

#### Suggestions

- 🔵 **Correctness**: State that block-scalar / continuation titles are
  intentionally skipped, and lock it with a test
  **Location**: Phase 4, item 1 / Migration Notes
  The transform silently skips non-single-line title shapes (block scalar,
  continuation). Zero exist today, so this is defensive-only, but the intended
  behaviour is unstated. State the scope and add a test asserting a
  non-matching-shape title is byte-identical.

- 🔵 **Safety**: Keep the m0010 apply and corpus rewrite in a single labelled
  commit
  **Location**: Migration Notes
  m0010 strips the title of its own driving research document, so the original
  titles survive only in VCS history. Keep the apply + rewrite in one reviewable
  commit and confirm the provenance doc's history is preserved, so the
  before/after is auditable in one place.

- 🔵 **Documentation**: State the criterion separating moved from deferred
  doc-type enumerations
  **Location**: Overview / Phase 5
  The plan moves two docs-site enumerations while deferring semantically similar
  narrative lists (`getting-started.mdx`, `index.mdx`, `visualiser.md:22`)
  without a stated rule. One sentence ("typed reader/template enumerations move;
  phase-spine and generic narrative lists stay") makes the deferral auditable.

### Strengths

- ✅ The five-namespace boundary table is the architectural core and is drawn
  correctly — it separates the moving codebase-research doc type from the
  umbrella `meta/research/` dir, `skills/research/` namespace, sibling corpora,
  and generic agents, and it protects the parser-coupled `## Related Research`
  heading and the cross-layer `hasResearch` field from a blind find-replace.
- ✅ Phase decomposition is excellent for review — five phases, each
  independently mergeable and each leaving `mise run check` green, with an
  explicit dependency graph and TDD ordering called out where it matters.
- ✅ `m0010` is a clean open-closed registry extension mirroring `m0006`, with
  inline module-private helpers that respect migration immutability; the id,
  module filename, struct name, and registration steps all follow the
  `m0001`–`m0009` convention and the library-crate discipline.
- ✅ Stem-vs-label form discipline (`codebase-research` for keys/template names,
  "Codebase research"/"codebase research" for display copy) is applied correctly
  per surface, and Phase 1 anticipates the alphabetical re-sort of the
  `templates.rs` seed assertion.
- ✅ Idempotency is treated as a first-class property; the inherited safety
  infrastructure (write-only-when-changed, atomic per-file writes, manifest
  resume, dirty-tree preflight) is genuinely strong.
- ✅ Verified against reality: the Correctness lens confirmed 177/177
  double-quoted titles and one column-0 H1 per file, and that `## Related
  Research` cannot be reached by the strip; the Test Coverage lens confirmed the
  `full_registry_e2e` count change (8→9) and the `list_and_decisions_file.rs`
  no-change claim.
- ✅ Proactively de-stales the `migrations/mod.rs` "0001-0006" module doc
  comment rather than re-introducing a range that keeps going stale.

### Recommended Changes

1. **Redesign the m0010 test strategy** (addresses: no unit-level tests;
   omitted title-only/H1-only/anchoring cases; missing-dir Applied; idempotency
   gate). Extract a pure `transform(&str) -> String`; co-locate unit tests over
   quote variants and near-miss prefixes; add independent frontmatter-only,
   body-H1-only, and post-`## ` anchoring cases; add a missing-corpus-dir case
   asserting `Applied`; make the idempotency test drop 0010 from the ledger
   before re-running.
2. **Resolve the Phase 3/4 ordering** (addresses: "either order" mixed-corpus
   risk). Make Phase 3 → Phase 4 a hard ordering (or same-release) and remove
   the "independently mergeable in either order" latitude; the existing P3-first
   numbering already satisfies this.
3. **Port m0006's destructive-migration guards** (addresses: dropped
   dangerous-path guard + diagnostics; no diff review). Add `is_dangerous_path`
   refusal and rewrite-count/missing-dir diagnostics; add a mandated `jj diff`
   review-before-commit step to Manual Verification; warn against
   `ACCELERATOR_MIGRATE_FORCE` for m0010.
4. **Correct and complete the Phase 4 §5 ledger guidance** (addresses:
   understated ledger edits). Note the inline per-test seeds and
   `migration_0001`'s byte-for-byte golden assertion explicitly; optionally
   extract the repeated full-ledger string into a shared constant.
5. **Fix documentation completeness and accuracy** (addresses: missed
   `visualiser.md:22`; Phase 1 docs:generate omission; contradictory grep;
   wrong api_smoke claim). Add `visualiser.md:22` to Phase 5 §4; add the
   `docs:generate` note to Phase 1; narrow the Desired End State grep to
   `title: "Research: '`; correct the api_smoke.rs claim to count-based.
6. **Harden the transform correctness** (addresses: positional anchoring; no
   content pre-gate; block-scalar scope). Describe the guard as positional, add
   an `!in_frontmatter` guard, add a content pre-gate to avoid spurious
   rewrites, and state that block-scalar/continuation titles are intentionally
   skipped (with a locking test).
7. **Remove the what-comment and record the inline-duplication rationale**
   (addresses: policy-violating comment; undocumented duplication). Mark the
   sketch comment illustrative-only and require named helpers; add one sentence
   on why `m0006`'s helpers are duplicated inline.
8. **Minor polish** (addresses: 80-column drift; residual issue-research label;
   deferral criterion; single-commit auditability; parallel-merge caveat).
   Apply the remaining minor and suggestion fixes as convenient.

## Per-Lens Results

### Architecture

**Summary**: The plan rests on a genuinely sound architectural spine — the
five-namespace boundary table correctly separates the moving codebase-research
doc type from the umbrella namespaces and deliberately preserves the
parser-coupled `## Related Research` heading and cross-layer `hasResearch`
field, so it respects the system's real coupling points rather than doing a
blind find-replace. The migration is a clean open-closed extension of the
registry, mirrors `m0006`, and keeps its helpers frozen/inline in line with
migration-immutability. The main weakness is the claim that Phase 3 and Phase 4
are safe to merge in either order — that holds for CI-green but not for data
consistency.

**Strengths**:
- The five-namespace boundary table distinguishes the moving doc type from
  load-bearing umbrella namespaces, avoiding a destructive blind substitution.
- It explicitly protects existing coupling seams (`## Related Research` parser
  binding; the cross-layer `hasResearch` field + public-API snapshot).
- `m0010` is a correct open-closed extension — new module appended to
  `registry()` plus a mod declaration, no modification to frozen migrations —
  selecting files via `paths.research_codebase`, which enforces the boundary.
- Keeping helpers inline/module-private (mirroring `m0006` rather than extending
  `text.rs`) is right for a migration system whose history must stay frozen.
- Correctly recognises registering a tenth migration as cross-cutting and
  updates ~13 ledger-seeded tests, counts, and the public-API snapshot in
  lockstep.
- Naming aligns with the domain; the migration is idempotent-by-construction
  with a test-first approach.

**Findings**:
- (major, medium) "Either order" pairing of a one-shot migration with a
  still-emitting template risks a permanently inconsistent corpus — Implementation
  Approach / Phase 3–4. m0010 is applied-once; if it lands before Phase 3 the
  template keeps minting prefixed docs the migration never revisits. Treat P3→P4
  as a hard ordering or same-release.
- (minor, medium) Registry membership is mirrored, not derived, across ~13 test
  ledgers — each migration is O(n) shotgun surgery — Phase 4 §5. Capture as a
  follow-up: derive seeds from `registry()`; refresh stale doc comments.
- (minor, medium) Overlapping edits to the same artifacts qualify the
  "independently mergeable" claim — Implementation Approach; Phase 1 §1 / Phase 5
  §3. Clarify independence is scoped to CI-green with sequential application.
- (minor, low) The largest corpus-wide body rewrite to date stresses the
  partial-failure recovery path — Phase 4 §1 / Migration Notes. Confirm and note
  that the per-run manifest restores partially-written files on error.

### Code Quality

**Summary**: The plan is well-decomposed into five independently-mergeable
phases with explicit test-first ordering, precise location references, and a
sound grounding in the `m0006` precedent. The main concerns are around m0010: a
step-listing what-comment sits in the `apply()` sketch (contravening the
comments policy), the transform is not designed as an isolated pure function for
unit-level testability, and the acknowledged inline duplication of `m0006`'s
helpers would benefit from a recorded rationale.

**Strengths**:
- Phase decomposition is excellent for maintainability and review, with an
  explicit dependency graph.
- TDD ordering is called out per phase, matching the red-green-refactor mandate.
- The migration id and struct name are descriptive and follow the established
  convention, expressing intent through domain language.
- Good maintainability catch: rewriting the stale `mod.rs` module doc comment
  rather than re-introducing a range that keeps going stale.
- Idempotency is treated as a first-class design property.

**Findings**:
- (major, high) Step-listing what-comment in the m0010 `apply()` sketch violates
  the comments policy — Phase 4 §1. Remove it; express steps as named helper
  calls as `m0006` does.
- (major, medium) Rewrite logic should be an isolated pure function for
  unit-level testability — Phase 4 §1 & §4. `m0006` extracts a pure `transform`
  with ~20 unit tests; m0010 inlines it with only two integration tests.
- (minor, medium) Record why `m0006`'s helpers are duplicated inline rather than
  shared — Phase 4 §1. Prevents a future DRY pass touching frozen migrations.
- (minor, medium) m0010 sketch omits `m0006`'s operator diagnostics and path
  guard — Phase 4 §1. Mirror the `eprintln!` summary and dangerous-path guard.
- (suggestion, low) Full quote-aware machinery may add untested paths given a
  uniformly double-quoted corpus — Phase 4 §1. Scope down or add branch tests.

### Test Coverage

**Summary**: The plan is unusually test-aware for a rename: it correctly
enumerates the ~13 ledger-seeded migrate-cli tests, its headline count change
(`full_registry_e2e` line 59 `applied: 8` → `applied: 9`) is verified correct,
and its claim that `list_and_decisions_file.rs` needs no change checks out. The
main weaknesses are in the new m0010 test design — the two proposed black-box
tests omit the independent frontmatter-title-only / body-H1-only cases, the
pre-first-`## ` anchoring guard, and single/unquoted-title handling, with no
unit-level tests despite the cited precedent. Two plan claims also mislead:
Phase 4 §5 understates per-file ledger edits, and Phase 1 §4 overstates
api_smoke.rs coverage.

**Strengths**:
- Cross-cutting ledger impact correctly scoped; the `full_registry_e2e` count is
  real and correctly slated to become `applied: 9`, consistent with `m0006`
  returning `Applied` even when the target dir is absent.
- The `list_and_decisions_file.rs` no-change claim is verified accurate.
- Correctly flags the sensitive spots (migration_0008's second-run guard;
  isolation of the migration-under-test) and models m0010 on `m0006`.
- TDD "change the test first" is stated for the two places it matters most.

**Findings**:
- (major, high) m0010 tests omit independent title-only / H1-only cases and the
  H1 anchoring guard — Phase 4 §4 / Testing Strategy.
- (major, high) No unit-level tests for m0010's transform; quote-variant
  branches untested — Phase 4 §1 & §4.
- (major, high) Ledger table understates per-file edits: scattered inline seeds
  and `migration_0001`'s byte-for-byte golden ledger assertion — Phase 4 §5.
- (minor, high) api_smoke.rs does not assert the template name — the rename is
  not name-verified there; the plan's claim is wrong — Phase 1 §4.
- (minor, high) No direct m0010 test for the "missing corpus dir still records
  Applied" property — Phase 4 §4 / Testing Strategy.
- (minor, medium) Idempotency test must bypass the ledger gate to test the
  transform, not the gate — Phase 4 §4.
- (minor, medium) Display-copy and "stop emitting prefix" changes have no
  automated test — Phase 2 §3 / Phase 3.

### Correctness

**Summary**: The m0010 transform logic is sound for the verified corpus — all
177 frontmatter titles are double-quoted `title: "Research: …"` (zero
single-quoted, unquoted, or block-scalar) and exactly 176 column-0
`# Research:` H1s, so the core apply/idempotency behaviour matches reality, and
`paths.research_codebase` correctly excludes the sibling corpora. The remaining
risks are edge/robustness rather than wrong-output-on-real-data: the body-H1
anchoring is positional rather than fence-aware, byte-stability of prefix-free
files depends on an implicit content pre-gate the plan does not spell out, and
non-double-quoted/block-scalar shapes are silently skipped without an explicit
intent or locking test.

**Strengths**:
- Scope boundary is correct: selecting via `paths.research_codebase` naturally
  excludes `research_topics`/`research_issues` (siblings, not nested).
- `## Related Research` (line 58) is provably safe from the strip — it matches
  neither the frontmatter `title:` rule nor the column-0 `# Research: ` H1 rule.
- Verified corpus uniformity backs the plan's claim (177 double-quoted titles, no
  stray column-0 lookalikes; the one file with a second `title: "Research: "`
  has it inside a body `yaml` fence, protected by the in-frontmatter guard).
- Idempotency is sound for the double-quoted case; `m0006` is a well-matched
  precedent.
- Phase 1 correctly anticipates the alphabetical re-sort of the seed assertion.

**Findings**:
- (minor, high) Body-H1 anchoring is positional (pre-first-`## `), not
  fence-aware; add an `!in_frontmatter` guard and place the fence test in the
  pre-`## ` region — Phase 4 §1 / Migration Notes.
- (minor, medium) No content pre-gate — line-ending reconstruction could
  spuriously rewrite prefix-free files, breaking the byte-identical guarantee —
  Phase 4 §1.
- (suggestion, medium) Non-double-quoted / block-scalar title shapes silently
  skipped; state intent and lock with a test — Phase 4 §1 / Migration Notes.

### Safety

**Summary**: The plan adds m0010, a destructive in-place rewrite of ~177
version-controlled corpus files, and inherits genuinely strong protective
infrastructure: idempotent-by-construction transforms, write-only-when-changed,
atomic per-file writes (temp+rename) with manifest-tracked resume, and a
dirty-tree preflight. The principal residual risks are the absence of any
dry-run/preview or mandated diff-review before committing a 177-file rewrite
(the plan actively normalises a post-migration dirty tree), and a blast-radius
gap when `paths.research_codebase` is misconfigured — the apply sketch omits the
precedent's dangerous-path guard, and the preflight's dirty-scan scope is
hardcoded and does not follow a corpus configured elsewhere. Version control is
the sole recovery net, proportionate for a git-tracked docs corpus provided the
operator is steered to inspect the diff.

**Strengths**:
- Idempotency is a first-class, tested safety property; re-running converges.
- Write-only-when-changed keeps the post-run diff limited to affected files.
- The dirty-tree preflight is a real safety net for the default configuration.
- Per-file atomic writes + run manifest make partial-failure recoverable and
  enable a guarded resume rather than a foreign-dirt refusal.
- `apply()` marks applied only on success, so a mid-walk failure leaves the
  migration pending and re-runnable.
- Body-H1 rewrite is anchored and prefix-matched, bounding per-file blast radius.
- The plan openly acknowledges it rewrites its own provenance and that a dirty
  tree afterwards is expected.

**Findings**:
- (major, medium) No dry-run/preview or mandated diff review before a 177-file
  destructive rewrite — Phase 4 Manual Verification / Migration Notes.
- (major, medium) Blast radius when `paths.research_codebase` is misconfigured —
  dangerous-path guard omitted, preflight scope fixed — Phase 4 §1.
- (minor, medium) Partial-failure/resume recovery relied upon but not asserted —
  Phase 4 Testing Strategy.
- (suggestion, high) Migration self-modifies its own provenance; audit trail
  lives only in VCS — keep the apply + rewrite in a single labelled commit —
  Migration Notes.

### Documentation

**Summary**: A well-scoped documentation-terminology rename with a clear
boundary table separating the codebase-research doc type (moves) from
umbrella/verb usage (stays), applying stem form vs label form thoughtfully per
surface. Verified file:line references and the five "Research directory" label
edits are accurate and complete for `research_codebase` resolvers. The main gaps
are internal inconsistencies the plan itself introduces: a doc-type enumeration
missed within a file it edits (`visualiser.md`), an inconsistent
mirror-regeneration instruction across phases, and a verification-grep claim
that contradicts the plan's own Migration Notes.

**Strengths**:
- The five-namespace boundary table is excellent documentation, preventing a
  destructive blind find-replace.
- Per-surface form discipline (stem vs label) is applied correctly and
  consistently.
- Coverage of the five `research_codebase` labels and the three template
  cross-references is complete and verified.
- `docs:generate` (SKILL edits) and `docs:check` (hand-written docs-site) are
  both correctly invoked in Phase 5; `docs:generate` is present in Phase 3.
- Sampled file:line references are accurate, consistent with the plan's
  line-number correction note.

**Findings**:
- (major, high) Phase 5 §4 edits `visualiser.md:13` but misses the equivalent
  doc-type enumeration at `visualiser.md:22` — Phase 5 §4.
- (minor, medium) Phase 1 edits `configure/SKILL.md` but omits the
  `docs:generate` mirror-regeneration note Phases 3 & 5 include — Phase 1.
- (minor, medium) Desired End State grep contradicts the Migration Notes —
  Desired End State / Phase 4 Migration Notes.
- (minor, low) `research-issue/SKILL.md:23` keeps a bare "Research directory"
  (issue research) with no deferral note — Phase 5 §1.
- (suggestion, low) No stated criterion separating moved from deferred doc-type
  enumerations — Overview / Phase 5.

### Standards

**Summary**: Unusually convention-aware for a naming cleanup: the m0010 id,
module filename, and struct all follow the `m0001`–`m0009` pattern; the
registration steps match both the `m0006` precedent and the library-crate
discipline; and the stem-vs-label distinction is applied correctly phase by
phase. Two concerns remain, both minor: an illustrative what-comment in the
sketched `apply()` that would violate the near-zero comment tolerance if it
survived, and 80-column/table-alignment drift in the docs-site edits that no
formatter guards.

**Strengths**:
- The m0010 id `0010-strip-research-title-prefix`, `m0010.rs`, and
  `Migration0010` mirror the mechanical-migration precedents (not the m0007
  interactive directory shape).
- Registration steps are complete and match precedent; correctly scoped as a
  library-crate change, so no sub-binary checklist applies.
- Stem-vs-label distinction applied per project convention throughout.
- Phase 1 preserves the alphabetical-ordering assertion at `templates.rs:446`.
- Phase 4 §2 proactively de-stales the `mod.rs` doc comment.

**Findings**:
- (minor, high) What-comment in the m0010 `apply()` sketch — Phase 4 §1. Require
  named helpers with no inline what-comments.
- (minor, medium) 80-column / table-alignment drift in the docs-site edits
  (`configuration.md:70`, `visualiser.md:13`) — Phase 5 §4. Rewrap and re-pad.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-13

**Verdict:** COMMENT

All eight major findings from the initial review are resolved. The re-review
surfaced two new major findings — both terminology/comment-consistency
regressions the revisions themselves left or exposed — plus a set of new minor
edge-case and completeness findings. Two majors is below the REVISE threshold of
three, so the plan is acceptable as-is; but both majors touch the plan's core
deliverable (terminology consistency) and are small in-plan fixes worth closing
before implementation.

### Previously Identified Issues

Majors — all resolved:

- 🟡 **Test Coverage, Code Quality**: No pure-function/unit tests for m0010 —
  Resolved (pure `transform` + co-located unit tests + integration matrix).
- 🟡 **Test Coverage**: Apply test omits title-only/H1-only + anchoring guard —
  Resolved.
- 🟡 **Test Coverage**: Ledger table understates edits — Resolved (inline seeds
  + byte-for-byte golden now called out; verified against the real tests).
- 🟡 **Architecture**: "Either order" P3/P4 mixed-corpus risk — Resolved (hard
  P3→P4 ordering; residual minor to harden beyond numbering).
- 🟡 **Safety**: No dry-run/diff review — Resolved (mandated `jj diff` + FORCE
  caveat).
- 🟡 **Safety, Code Quality**: Dropped dangerous-path guard + diagnostics —
  Resolved (guard + diagnostics ported; residual minor on the note's wording).
- 🟡 **Documentation**: Missed `visualiser.md:22` — Resolved (verified correct).
- 🟡 **Code Quality, Standards**: What-comment in the `apply()` sketch — Resolved
  (removed; named-helpers instruction added).

Minors / suggestions:

- Resolved: CR-1 positional / `!in_frontmatter` (praised as a genuine
  improvement), CR-2 content pre-gate, CR-3 block-scalar scope, TC-4 api_smoke
  claim, TC-5 missing-dir Applied, TC-6 idempotency ledger-drop, AR-3 overlapping
  edits, CQ-3 duplication rationale, ST-2 80-col note, DOC-2 Phase 1
  `docs:generate`, DOC-3 grep, DOC-4 issue-research label, SF-4 single commit.
- Partially resolved: TC-7 display-copy guard (Phase 3 guard added but
  under-located; empty-state plural still soft); AR-4/SF-3 partial-failure
  recovery (note added, but the jj resume gate may not fire — see new issues).
- Still deferred by design: AR-2 ledger-mirror derivation (out-of-scope
  follow-up).
- Escalated: DOC-5 moved-vs-deferred criterion — the added criterion does not
  actually separate moved from left lines, re-raised as a new major.

### New Issues Introduced

- 🟡 **Standards**: Phase 1 §2 renames the `research` key in `template-tier.ts`
  but leaves two comments (the lines 24–29 doc comment and the lines 88–91 inline
  example) describing the removed `research`-stem mechanism — the same
  stale-comment class the plan proactively fixes in `mod.rs`.
- 🟡 **Documentation**: The moved-vs-deferred criterion is not applied
  consistently — bare "research" doc-type lists remain in higher-traffic pages
  (`index.mdx:45`, `getting-started.mdx:43/148`, `meta-directory.mdx:116` incl. an
  intra-page clash with its own line 24, `skills/adrs.md:25`,
  `skills/work-items.md:33`); the deferral inventory names only two of them.
- 🔵 **Architecture**: Harden the P3→P4 constraint beyond phase numbering — land
  P3 and P4 in one commit, or add an explicit merge-gate note.
- 🔵 **Safety**: The path-guard note overstates what it bounds — a wrong-but-
  in-repo `research_codebase` passes `is_dangerous_path`; the content pre-gate and
  VCS reversibility are the real backstop for that case.
- 🔵 **Safety**: The manifest-resume claim may not fire under jj — the
  working-copy revision shifts once partial edits snapshot, flipping the preflight
  to ForeignDirt; document a manual clean-and-rerun fallback or test it (FORCE is
  forbidden for m0010).
- 🔵 **Code Quality / Correctness**: The content pre-gate predicate must be
  derived from the same scan as the rewrite, or the "touch this file" decision and
  the "which lines to strip" decision can drift.
- 🔵 **Correctness**: A pre-`## ` fenced `# Research:` lookalike is unprotected by
  positional anchoring; add a test, or state the pre-`## ` region is fence-free by
  template convention.
- 🔵 **Standards**: `full_registry_e2e.rs:1`'s "7-migration registry" comment is
  stale and in a file the plan already edits — fold the fix in rather than defer.
- 🔵 **Standards**: `configure/SKILL.md:877`'s filename-example code block loses
  its comment-column alignment after the rename.
- 🔵 **Test Coverage**: `skill_doc_worked_example.rs` seeds the ledger but is
  absent from the Phase 4 §5 audit table (stays green, but the enumeration claims
  to be exhaustive).
- 🔵 **Test Coverage**: The Phase 3 regression guard's host test module is
  under-specified.
- 🔵 **Documentation**: The configure template-key list stays incomplete (6 of
  13) on the very line the plan edits.
- 🔵 **Documentation**: State the intended end-state name for the deferred
  `research-issue` "Research directory" label so the asymmetry reads as
  intentional.
- Suggestions: name `transform` for its domain (`strip_research_title_prefix`);
  make the empty-state-plural assertion unconditional; add a per-migration
  second-run no-pending case; carry m0006's `refuses` guard for unquoted `#`/`"`
  titles; pin double-prefix idempotency; pull the registry-derived ledger helper
  forward now; add an automated over-reach guard alongside the grep.

### Assessment

The plan is materially stronger: the destructive migration is now well-specified,
thoroughly tested, and safety-guarded, and every initial major is closed. The two
remaining majors are consistency regressions — the `template-tier.ts` stale
comments and the incomplete docs-site terminology sweep — that touch the exact
quality the rename exists to deliver, and both are small in-plan fixes. Closing
them, plus the cheap new minors (the `full_registry_e2e.rs:1` comment, the
`skill_doc_worked_example.rs` audit row, an unconditional empty-state assertion),
would reach APPROVE; the remaining low-impact edge cases can be accepted or
addressed at the implementer's discretion.

## Re-Review (Pass 3) — 2026-09-13

**Verdict:** COMMENT

Focused pass over the three lenses whose findings the Pass-2 edits addressed
(Standards, Documentation, Test Coverage). Standards is now clean. Test Coverage's
two prior findings are resolved (plus two low-impact ledger-accounting nits,
folded in). Documentation confirmed the moved-vs-deferred criterion now
distinguishes correctly, but verified against source that the Pass-2 sweep
extension over-reached — three of the added targets name the umbrella corpus or
generic source, not the doc type, and two cited paths were gitignored generated
mirrors. Those three majors were introduced by the Pass-2 fix and are corrected
in this pass by narrowing §4 to the verified doc-type enumerations and recording
the rest as deliberately left. No majors remain.

### Previously Identified Issues (Pass 2 majors)

- 🟡 **Standards**: template-tier.ts stale `research`-stem comments — Resolved
  (Phase 1 §2 instruction verified accurate against the source comments).
- 🟡 **Documentation**: moved-vs-deferred criterion not distinguishing —
  Resolved at the criterion level; the §4 target list it drove over-reached (see
  New Issues).

Pass-2 cheap minors folded in:

- 🟢 **Standards**: `full_registry_e2e.rs:1` "7-migration registry" comment
  refresh — Resolved (now in the §5 table row, verified stale).
- 🟢 **Test Coverage**: `skill_doc_worked_example.rs` audit row — Resolved
  (verified it stays green with 0010 pending).
- 🟢 **Test Coverage**: empty-state plural unconditional assertion — Resolved
  (`EMPTY_TYPE_PLURALS["codebase-research"]` and `empty-descriptions.test.ts`
  verified as described).

### New Issues Introduced (by the Pass-2 sweep extension) — corrected this pass

- 🟡 **Documentation**: §4 targeted gitignored mirror paths
  (`reference/skills/*.md`); the committed sources are `skills/adrs.md` /
  `skills/work-items.md`. Corrected — those targets removed; a "no docs-site edit
  into `reference/skills/`" note added.
- 🟡 **Documentation**: `skills/adrs.md:25` and `skills/work-items.md:33` are
  umbrella/generic-source references (extract-* mine the whole corpus and
  specs/PRDs), not the doc type — renaming would misdescribe them. Corrected —
  recorded on the explicit leave-list.
- 🟡 **Documentation**: `getting-started.mdx:43` is the umbrella `meta/research/`
  tree `/init` scaffolds, not a doc-type list (only `:148` qualifies). Corrected
  — scoped to `:148`, `:43` left.
- 🔵 **Documentation**: `meta-directory.mdx:116` also covers issue-research
  lifecycle. Corrected — added a check-first caveat before narrowing.
- 🔵 **Documentation**: comma-separated lists in `case-study.md` / `philosophy.md`.
  Corrected — added to the explicit leave-list as artefact-trail prose.
- 🔵 **Test Coverage**: `discoverability_hook.rs` is a third no-change file, and
  the `skill_doc_worked_example.rs` `--list` test stays green because mechanical
  migrations do not surface in `--list` (not because it is "substring-based").
  Corrected in §5.

### Assessment

Every major from all three passes is now resolved: the destructive migration is
well-specified and tested, and the docs-site sweep is scoped to the verified
doc-type enumerations with the umbrella and generic-source lines explicitly left.
The remaining open items are the low-impact edge-case minors carried from Pass 2
in the four lenses not re-run this pass (correctness edge cases, the path-guard
note wording, the ledger-derivation follow-up), each acceptable at the
implementer's discretion. The plan is ready to implement; APPROVE once
comfortable with those accepted residuals.

**Post-pass follow-up (2026-09-14):** a final tidy-up pass closed most of the
accepted residuals in the plan:

- **Safety** — jj resume-gate wording: Migration Notes now document the
  sanctioned discard-and-rerun (`jj restore` / `git restore`, then re-run on a
  clean tree, relying on idempotency) instead of an in-place manifest resume, and
  note the preflight's run-id/revision gate and that FORCE is not the recovery
  path.
- **Safety** — path-guard note reworded: `is_dangerous_path` only rejects
  escaping values; a wrong-but-in-repo corpus is bounded by the content pre-gate
  and VCS reversibility, not the guard.
- **Architecture** — P3→P4 hardened beyond numbering: P4 must not merge/apply
  before P3, or land both in one commit.
- **Correctness** — carried `m0006`'s `refuses` guard for unsafe unquoted titles
  (with a test), stated the pre-`## ` fence-free assumption (with a test), and
  pinned pre-gate/rewrite predicate parity.
- **Code Quality** — the transform is now named for its domain
  (`strip_research_title_prefix`).

Remaining open (deliberately deferred): the Architecture ledger-derivation
follow-up (derive test seeds from `registry()`), and the low-impact correctness
double-prefix idempotency note. The plan is ready to implement.

## Approval (2026-09-14)

**Verdict: APPROVE.** Every major finding across all three review passes is
resolved and the substantive residuals are closed; the two remaining items are
deliberately-deferred, low-impact follow-ups the author has accepted. The plan is
approved for implementation and its `status` is set to `ready`.
