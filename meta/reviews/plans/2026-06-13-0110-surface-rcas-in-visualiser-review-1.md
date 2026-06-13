---
type: plan-review
id: "2026-06-13-0110-surface-rcas-in-visualiser-review-1"
title: "Plan Review: Surface Root Cause Analyses in the Visualiser"
date: "2026-06-13T21:41:30+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-13-0110-surface-rcas-in-visualiser"
target: "plan:2026-06-13-0110-surface-rcas-in-visualiser"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, compatibility, documentation]
review_number: 1
review_pass: 3
tags: ["visualiser", "rca", "doc-types", "library", "operate"]
last_updated: "2026-06-13T23:16:21+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Surface Root Cause Analyses in the Visualiser

**Verdict:** REVISE

This is a meticulous, well-researched plan that correctly identifies the
compiler-enforced registry seam as its backbone, respects the load-bearing
frontend-before-server merge ordering, and documents the central "four
spellings" naming hazard up front — the architecture, correctness reasoning,
and standards conformance are all strong. It falls short of APPROVE on two
mechanical-but-real fronts: (1) the plan under-enumerates the hard-coded
doc-type-count assertions (`13`/`39`) that adding a 14th type breaks, so
following the listed edits literally will leave the plan's own `mise run
check` gate red; and (2) three work-item acceptance criteria (zero-count,
related-artifacts, search) are verified only manually despite cheap,
idiomatic unit-test seams already existing in the repo. Both are
addressable with small, targeted edits to the plan.

### Cross-Cutting Themes

- **Under-enumerated count assertions and comments** (flagged by:
  compatibility, correctness, standards, code-quality, documentation) — The
  plan treats the `13`→`14` bump as a short, line-referenced list, but the
  enumeration is incomplete. **Verified-live, build-breaking assertions the
  plan omits**: `server/tests/api_smoke.rs:102` (`len(), 13`),
  `server/src/docs.rs:376` (`types.len(), 13`),
  `frontend/.../glyph-showcase/GlyphShowcase.test.tsx:9` (`39`→`42`),
  `frontend/.../big-glyph-showcase/BigGlyphShowcase.test.tsx:9` (`13`→`14`),
  and the two explicit literals at `BigGlyph.test.tsx:69,81` (the plan only
  says "update any hard-coded count"). The plan also cites the
  `/api/types` assertion at `api_types.rs:13` when the real `assert_eq!` is
  at `:32` (line 13 is the test *name*). Separately, a cluster of stale
  count **comments** (`bigPalette.ts:16`, `Glyph.tsx:1,23,75`, `tokens.ts`,
  `global.css`, `router.ts`, the frontend README's `/glyph-showcase`
  description) is left untouched. The plan's "these fail to compile/pass
  until updated — TDD by construction" claim is sound in spirit but
  presented as complete when it is not.

- **Acceptance-criteria-to-test traceability** (flagged by: test-coverage) —
  Work-item ACs #2 (zero-count card), #5 (related-artifacts row), and #6
  (search) are pushed onto manual/E2E verification only, even though
  `api_related.rs`, `RelatedArtifacts.test.tsx`, `api_search.rs`, and
  `SearchResultsPanel.test.tsx` already provide trivial fixture seams. The
  work item's final AC explicitly requires only the structure/card-listing/
  E2E-navigation tests (which the plan *does* cover), so this is a
  thoroughness gap rather than a contract violation — but the regression
  guards are cheap and the behaviours are exactly the ones that only become
  possible once RCA joins the registry.

- **Short label "RCA" silently dropped** (flagged by: code-quality,
  standards) — Work-item ACs 2 and 7 name a short label "RCA", but no
  frontend short-label table exists (only `TIER_SHORT_LABELS` for
  templates) and the prototype's `short` is an icon-less fallback RCA never
  hits. The omission is therefore correct, but it is silent rather than
  recorded as N/A like the BigGlyph reword.

### Tradeoff Analysis

- **Test-coverage rigour vs scoping pragmatism**: The test-coverage lens
  wants automated guards for ACs #2/#5/#6; the work item only mandates a
  subset of tests and the plan satisfies that subset. Recommendation: add
  the two cheapest, highest-value guards (a server `api_related.rs` RCA case
  and an `api_search.rs` RCA case, both seeded from the fixture you are
  already adding) and accept the zero-count card as manual-or-unit at your
  discretion. This closes the regression-risk gap without over-investing.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Compatibility / Correctness / Standards**: Plan under-enumerates the
  doc-type-count assertions that adding a 14th type breaks
  **Location**: Phase 1 §7, Phase 2 §3 (test updates)
  Several live count-coupled assertions are not listed and will fail the
  plan's `mise run check` success gate: `api_smoke.rs:102`, `docs.rs:376`,
  `GlyphShowcase.test.tsx:9` (39→42), `BigGlyphShowcase.test.tsx:9`, and the
  explicit `13` literals at `BigGlyph.test.tsx:69,81`. The `api_types.rs`
  reference points at the test name (`:13`) not the assertion (`:32`).

- 🟡 **Test Coverage**: Related-artifacts AC (#5) has no named automated test
  **Location**: Phase 3 §1 / Testing Strategy
  AC #5 (a linking artifact shows the RCA in related artifacts, routing to
  the RCA detail page) is covered only by a loosely-specified E2E fixture
  plus manual verification, despite `api_related.rs` and
  `RelatedArtifacts.test.tsx` offering trivial seams. This behaviour only
  becomes possible after RCA joins the union + gains a singular label, so it
  warrants a fast regression guard.

- 🟡 **Test Coverage**: Search AC (#6) relies on auto-inclusion with no
  test that an RCA is returned/labelled as an RCA
  **Location**: Testing Strategy / Phase 2
  AC #6 is asserted only manually. The auto-inclusion preconditions
  (non-virtual, `config_path_key` mapped, non-`None` slug) are real and can
  regress; `api_search.rs` only exercises plans/decisions today and
  `SearchResultsPanel.test.tsx` is never named.

- 🟡 **Test Coverage**: Zero-count AC (#2) has no automated assertion for the
  Operate/RCA card
  **Location**: Desired End State / Phase 1 Manual Verification
  The seeded server fixture now writes a *non-zero* RCA, so the existing
  zero-count assertion does not cover RCA. No test asserts the Operate card
  renders at count 0 with a null latest when its directory is empty — the
  exact distinct branch AC #2 calls out.

#### Minor

- 🔵 **Architecture**: `in_lifecycle()` emits `true` for a type designed to
  be outside the lifecycle
  **Location**: Phase 2 §1 (lifecycle predicates)
  RCA falls through `in_lifecycle()`'s default-true branch, so the server
  emits `inLifecycle: true` for a peer category. Harmless today (the
  frontend pipeline drives off a hard-coded step list and the sidebar
  discards the flag), but the semantic flag becomes a lie. Make it return
  `false` explicitly, or pin the choice with a comment + a test asserting
  RCA's `inLifecycle` value.

- 🔵 **Architecture**: Two `DocTypeKey` registries are hand-synchronised with
  no cross-stack contract check
  **Location**: Current State Analysis / Implementation Approach
  Server enum and frontend union must agree on every wire token; only
  intra-side exhaustiveness is enforced. Consider a cheap follow-up contract
  test asserting `/api/types` keys equal `DOC_TYPE_KEYS` to convert the
  runtime-`undefined` risk into a CI failure.

- 🔵 **Code Quality**: `GlyphDocType` / `GLYPH_ONLY_DOC_TYPES` indirection
  becomes vacuous but is retained
  **Location**: Phase 1 §3
  With the array emptied, `GlyphDocType = DocTypeKey | never`. Add a one-line
  comment recording the empty list is intentional headroom, or note that
  collapsing `GlyphDocType` back to `DocTypeKey` is a deliberate non-goal.

- 🔵 **Code Quality / Correctness**: `resolved`/`monitoring` widen
  globally-shared status sets without a scoping note
  **Location**: Phase 1 §5
  `statusToVariant` is doc-type-agnostic, so the new entries colour *every*
  type's status column, not just RCAs. Add a note (and ideally a comment)
  that these are shared status verbs, and add them to the exhaustive
  `status-variant.test.ts` `it.each` table (currently unnamed in the plan)
  so a wrong-set placement fails a test.

- 🔵 **Code Quality / Standards**: Work-item short label "RCA" dropped without
  a recorded decision
  **Location**: Phase 1 §1 / "What We're NOT Doing"
  No short-label registry exists; the omission is correct but should be
  recorded as N/A alongside the BigGlyph reword so the AC checklist stays
  trustworthy.

- 🔵 **Test Coverage**: `api_types.rs` change is a bare length bump with no
  RCA property assertions
  **Location**: Phase 2 §3
  A length-only bump would still pass if RCA were emitted with a wrong
  `virtual` flag or a null `dirPath` (mis-mapped `config_path_key`). Add an
  assertion that the `root-cause-analyses` entry is `virtual:false` with a
  string `dirPath`.

- 🔵 **Test Coverage**: Two RCA fixtures risk drift
  **Location**: Phase 1 §7 / Phase 2 §3 / Phase 3 §1
  The inline `seeded_cfg` write and the committed E2E fixture encode the same
  frontmatter contract independently. Keep them byte-identical and note that
  they must be updated together.

- 🔵 **Compatibility**: `Glyph.test.tsx:205-215` glyph-only special case
  becomes contractually false
  **Location**: Phase 1 §3
  Once RCA joins `DOC_TYPE_KEYS` it is covered by the `describe.each` matrix,
  so the bespoke "not in DOC_TYPE_KEYS" block is stale and duplicative. Fold
  or rewrite it in Phase 1.

- 🔵 **Compatibility**: Directory-prefix classification depends on
  non-nesting of the two `research_*` roots
  **Location**: Phase 2 §1
  Classification is a `path.starts_with` over a `HashMap` with
  non-deterministic iteration. The default layout keeps
  `meta/research/codebase` and `meta/research/issues` disjoint, but a user
  override nesting one under the other could misclassify. Note the
  assumption; no code change needed for the default set.

- 🔵 **Compatibility**: E2E `start-server.mjs` `docPaths` has drifted from the
  launcher contract
  **Location**: Phase 3 §1
  The E2E launcher emits a 12-key `doc_paths` while `write-visualiser-config.sh`
  emits 13; until Phase 3 wiring lands, RCAs are server-active but
  E2E-invisible. Ensure the `start-server.mjs` edit lands with/before any E2E
  expecting RCAs, and note the two should track each other.

- 🔵 **Documentation / Standards / Code Quality**: Stale `13`/`12` count
  comments left across the frontend
  **Location**: Phase 1 §6
  `bigPalette.ts:16`, `Glyph.tsx:1,23,75`, `tokens.ts`, `global.css`,
  `router.ts`, and the frontend README's `/glyph-showcase` description all
  carry hard-coded counts the change invalidates. Prefer rewording them
  count-free (e.g. "one per `DocTypeKey`") to prevent recurrence.

#### Suggestions

- 🔵 **Architecture / Correctness**: Rename the `api_types.rs` test function
  away from "thirteen" so the contract's documented and asserted sizes stay
  coherent.

- 🔵 **Correctness**: Keep the Operate structure test to a single seeded RCA
  (as planned) so the `latest` mtime tie-break stays deterministic; if more
  are added, set distinct mtimes via `set_mtime_ms`.

- 🔵 **Correctness**: Confirm the `issue-research` status vocabulary is
  limited to single-token values; multi-word statuses would render an
  inconsistent facet label vs chip colour.

- 🔵 **Standards**: Confirm `EMPTY_TYPE_PLURALS` "root cause analyses"
  (unhyphenated, matching the display label) is intended — no change needed.

- 🔵 **Documentation**: The new "Operate" top-level category needs no
  human-facing doc update (no closed-set category inventory exists); the
  visualiser self-documents via its generic surfaces.

### Strengths

- ✅ Respects the load-bearing data-flow direction: making the frontend
  understand `root-cause-analyses` (dormant) before the server emits it
  prevents an undefined glyph/label/route lookup at runtime — a genuine
  architectural dependency, not just sequencing preference.
- ✅ Adds the type at the single designed seam (`PHASES` static) and consumes
  the pre-existing `research_issues` config key rather than introducing a
  parallel mechanism, keeping the contract footprint minimal.
- ✅ Leans on compiler-enforced exhaustiveness (Rust match arms +
  `Record<DocTypeKey, …>` tables) so per-side changes are atomic by
  construction.
- ✅ The "four spellings" table documents the central naming hazard up front
  — exactly the context the next maintainer needs.
- ✅ Surfaces, reasons about, and resolves the BigGlyph-placement divergence
  between the work-item ACs and the prototype toward the established pattern,
  rather than silently introducing a new per-type rendering pattern.
- ✅ The `in_lifecycle()`-by-default reasoning, the empty-`GLYPH_ONLY_DOC_TYPES`
  type-check, the status→variant normalisation, and the BigGlyph palette-key
  port were all verified correct against the code.
- ✅ Verified accurate: `config_contract.rs` genuinely needs no change —
  `research_issues` is already one of the 13 `doc_paths` keys.

### Recommended Changes

1. **Complete the count-assertion enumeration** (addresses: the count
   cross-cutting theme; the Compatibility/Correctness major). In Phase 1 §7
   and Phase 2 §3, explicitly list every `13`/`39` assertion to bump:
   `api_smoke.rs:102`, `docs.rs:376`, `GlyphShowcase.test.tsx:9` (→42),
   `BigGlyphShowcase.test.tsx:9` (→14), `BigGlyph.test.tsx:69,81` (→14), and
   fix the `api_types.rs` reference to `:32` (renaming the test). Add the
   `status-variant.test.ts` entries while you are there.

2. **Add cheap automated guards for ACs #5 and #6** (addresses: the
   related-artifacts and search majors). A server `api_related.rs` case
   seeding a linking artifact + asserting the RCA appears, and an
   `api_search.rs` case asserting a query returns `docType:
   "root-cause-analyses"` with a non-empty slug — both reuse the fixture you
   are already adding.

3. **Decide and record the zero-count coverage** (addresses: the zero-count
   major). Either add a `LibraryOverviewHub.test.tsx` Operate-at-count-0 case
   or note in the plan that AC #2 is intentionally manual-only.

4. **Record the short-label "RCA" decision as N/A** (addresses: the
   short-label minor) in the planning-decision section, mirroring the
   BigGlyph reword.

5. **Broaden the comment-update step to be count-free** (addresses: the stale
   comment minors) across `bigPalette.ts`, `Glyph.tsx`, `tokens.ts`,
   `global.css`, `router.ts`, and the frontend README.

6. **Pin `in_lifecycle()` intent** (addresses: the Architecture minor): return
   `false` for RCA explicitly, or comment + test the deliberate default-true.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan extends a well-established, compiler-enforced registry
pattern to add one doc type and one category. It is architecturally sound: it
adds the type at the single intended seam, leans on exhaustiveness checks, and
the phase/merge ordering correctly respects the server-driven data-flow
direction. Residual concerns: an internal semantic inconsistency in the
`in_lifecycle()` flag, and the absence of any cross-stack contract check
between the two hand-synchronised `DocTypeKey` registries.

**Strengths**:
- Correctly identifies and respects the server-driven data-flow direction
  (frontend dormant → server activates) to avoid runtime `undefined` lookups.
- Adds the type at the single designed extension seam and consumes the
  existing `research_issues` key.
- Leverages compiler-enforced exhaustiveness as the completeness guarantee.
- Preserves cohesion: classification, counts, listing, related-artifacts, and
  search are all generic — zero per-type branching added.
- Explicitly surfaces and resolves the BigGlyph-placement divergence.

**Findings**:
- _Minor (high)_ — Phase 2 §1: `in_lifecycle()` returns true-by-default for
  RCA, so the server emits `inLifecycle: true` for a type designed to be
  outside the lifecycle. Harmless today (frontend pipeline uses a hard-coded
  step list; sidebar discards the flag) but the flag becomes a lie. Return
  `false` explicitly or pin with comment + test.
- _Minor (medium)_ — Current State / Implementation Approach: server enum and
  frontend union are coupled only by convention; nothing enforces cross-side
  agreement. Consider a `/api/types` == `DOC_TYPE_KEYS` contract test.
- _Suggestion (high)_ — Phase 2 §3: `api_types.rs:13` is the test name, not
  the assertion (at `:32`); rename it away from "thirteen".

### Code Quality

**Summary**: A meticulous, well-scoped plan that leans on compiler-enforced
exhaustiveness so adding a doc type is forced to be atomic and complete. The
"four spellings" hazard is documented up front. Main residual concerns are
comment/abstraction debt: several hard-coded count comments and a now-vacuous
`GLYPH_ONLY_DOC_TYPES`/`GlyphDocType` indirection are not addressed.

**Strengths**:
- Leans on compiler-enforced exhaustiveness, framing each per-side change as
  inherently atomic and self-verifying.
- The "four spellings" table is documented up front with each token's lane.
- Changes confined to existing well-named seams; no new abstractions invented.
- BigGlyph hero added as a self-contained `BigGlyphDraw` mirroring siblings;
  port verified to use only existing palette keys.
- TDD framing and dormant-then-activate ordering reflect low-risk sequencing.

**Findings**:
- _Minor (high)_ — Phase 1 §6: hard-coded count comments left stale outside
  the touched files (`bigPalette.ts:16`, `tokens.ts:132,150`,
  `Glyph.constants.ts` "not a doc type today").
- _Minor (medium)_ — Phase 1 §3: `GlyphDocType` / `GLYPH_ONLY_DOC_TYPES`
  becomes vacuous (`DocTypeKey | never`) but is retained without explanation.
- _Minor (medium)_ — Phase 1 §5: RCA statuses widen globally-shared
  `GREEN`/`INDIGO` sets that apply to all doc types; blast radius undocumented.
- _Minor (medium)_ — Phase 1 §1: work-item short label "RCA" dropped without a
  recorded decision, weakening AC-to-code traceability.

### Test Coverage

**Summary**: Strong testing backbone — correctly leans on exhaustiveness,
applies TDD, and names the right server-structure, hub/listing, and E2E tests
for the headline ACs. But ACs #5 (related-artifacts) and #6 (search) have no
named automated test, and AC #2 (zero-count) and the new status mappings are
under-covered, despite cheap idiomatic seams existing.

**Strengths**:
- Exploits compile-time exhaustiveness as a genuine safety net.
- Applies TDD within each phase.
- Reuses existing seams (`seeded_cfg`, phase-id assertion, hub href pattern,
  BigGlyph dispatch guard, navigation template).
- Names the count-bearing assertions precisely for the headline ACs.

**Findings**:
- _Major (high)_ — Phase 3 §1: related-artifacts AC (#5) deferred to a vague
  E2E fixture; no `api_related.rs` or `RelatedArtifacts.test.tsx` case named.
- _Major (high)_ — Testing Strategy/Phase 2: search AC (#6) relies on
  auto-inclusion with no test that an RCA is returned/labelled as an RCA.
- _Major (high)_ — Phase 1 Manual Verification: zero-count AC (#2) has no
  automated assertion; the seeded fixture is now non-zero.
- _Minor (high)_ — Phase 1 §5: `resolved`/`monitoring` not added to the
  exhaustive `status-variant.test.ts` table; listing test checks attribute
  presence, not value.
- _Minor (medium)_ — Phase 2 §3: `api_types.rs` change is a bare length bump;
  no RCA `virtual:false`/`dirPath` property assertions.
- _Minor (medium)_ — Phase 1 §7 / Phase 3: two RCA fixtures risk drift; keep
  byte-identical.

### Correctness

**Summary**: Logically sound on every load-bearing claim verified against the
code: the merge ordering prevents undefined lookups, the `in_lifecycle()`
default-true reasoning is benign (gated by `participates_in_lifecycle`), the
empty `GLYPH_ONLY_DOC_TYPES` type-checks, the status→variant additions resolve
correctly, and the count/latest tie-break is deterministic. Main risks are
completeness gaps in the enumerated test-assertion edits.

**Strengths**:
- The frontend-before-server ordering is genuinely load-bearing and correctly
  justified (`build_structure` maps phases to per-type lookups).
- The `in_lifecycle()`-by-default reasoning verified correct: the linear
  pipeline is gated by `participates_in_lifecycle` (consumed only at
  `clusters.rs:226`).
- Classification-by-directory understood correctly; the four spellings stay in
  their lanes.
- Status→variant additions resolve correctly through `normaliseValue`.
- The RCA BigGlyph ports cleanly (palette keys sanctioned by the source-walk
  guard).

**Findings**:
- _Minor (high)_ — Phase 2 §3: `docs.rs:376` (`types.len(), 13`) not
  enumerated.
- _Minor (high)_ — Phase 1 §7: two hardcoded `13` literals at
  `BigGlyph.test.tsx:69,81`, not just a comment.
- _Minor (high)_ — Phase 2 §3: `api_types.rs` length assertion is at `:32`,
  not the cited `:13` (the test name).
- _Minor (medium)_ — Phase 2 §3: the Operate `latest` assertion depends on a
  non-deterministic mtime tie-break; keep to a single seeded RCA.
- _Suggestion (medium)_ — Phase 1 §5: server status-facet humanisation is a
  separate path from the chip variant; confirm single-token status vocabulary.

### Standards

**Summary**: Strongly aligned with the project's doc-type registration
conventions — the four-token triad is spelled consistently, and the
kebab-case wire token, sentence-case labels, numeric hue 310, and PascalCase
BigGlyph filename all match existing patterns. Main gaps: the work-item short
label "RCA" is silently dropped (defensible but undocumented), and the
count-comment update is scoped only to `BigGlyph.tsx`.

**Strengths**:
- The four-spelling naming triad is reproduced exactly as it lives in the code.
- Labels follow the established sentence-case convention precisely.
- New BigGlyph filename and draw-function shape match the icon-file convention.
- Numeric hue 310 placed in `DOC_TYPE_HUE` (the documented colour-identity
  source).
- Status-variant additions follow the existing set-membership pattern.

**Findings**:
- _Minor (high)_ — planning decision: the "short label RCA" AC has no frontend
  counterpart; the omission is correct but silent.
- _Minor (high)_ — Phase 1 §6: stale `13`/`thirteen` comments outside
  `BigGlyph.tsx` (`bigPalette.ts:16`, `BigGlyph.test.tsx:167`,
  `router.ts:160`, `Glyph.tsx:1,23,75`).
- _Suggestion (medium)_ — Phase 1 §3: update now-inaccurate "RCA is not a doc
  type" comments in `Glyph.tsx:23-25,75-77` and `tokens.ts:73-76`.
- _Suggestion (low)_ — Phase 1 §4: confirm `EMPTY_TYPE_PLURALS` capitalisation
  (no change needed).

### Compatibility

**Summary**: The core wire-contract change is additive (a 14th
`DocTypeKey`/phase), the safe direction, and the frontend-before-server
ordering correctly prevents an unknown-doc-type render gap because the SPA is
embedded in the server binary and they ship together. The serde round-trip is
sound, and the claim that `config_contract.rs` needs no change is verified
correct. Main gap: the plan undercounts the `13`-coupled assertions that will
break the build.

**Strengths**:
- Additive wire-contract change; nothing renamed or removed.
- Frontend-before-server ordering is the correct discipline; the release
  binary embeds the SPA so no durable mixed-version pairing occurs.
- serde kebab-case stability preserved.
- `config_contract.rs` genuinely unchanged — verified (`research_issues` is
  already the 6th of 13 keys).
- Directory-prefix classification safe for the default disjoint layout.

**Findings**:
- _Major (high)_ — Phase 2 §3: `/api/types` length is asserted in **two**
  server test files; the plan updates only `api_types.rs:32` and omits
  `api_smoke.rs:102`.
- _Major (high)_ — Phase 1 §7: `DOC_TYPE_KEYS` cardinality drives untouched
  showcase tests — `GlyphShowcase.test.tsx:9` (39→42),
  `BigGlyphShowcase.test.tsx:9` (→14), and the explicit literals in
  `BigGlyph.test.tsx:69,81`.
- _Minor (high)_ — Phase 1 §3: `Glyph.test.tsx:205-215` glyph-only special
  case becomes contractually false and duplicative.
- _Minor (medium)_ — Phase 2 §1: directory-prefix classification could
  misclassify under a nested `research_*` override (HashMap iteration order).
- _Minor (medium)_ — Phase 3 §1: `e2e/start-server.mjs` `docPaths` has drifted
  from the launcher's key set; ensure Phase 3 wiring tracks
  `write-visualiser-config.sh`.

### Documentation

**Summary**: Unusually thorough on code-level detail and correctly resolves
the AC/prototype documentation conflict in-text. Overlooks the human-facing
surface: several hard-coded count comments and the frontend README's dev-route
description go stale, and the plan updates only a subset of the count comments.
None blocking.

**Strengths**:
- Explicitly reconciles the work-item ACs 4/7 vs prototype contradiction in a
  dedicated decision section.
- The "four spellings" table is excellent reference documentation.
- Each phase carries self-contained verification checklists; migration/perf
  sections correctly document "none".
- Correctly leaves the open-ended README "Library" description untouched.

**Findings**:
- _Minor (high)_ — Phase 1 §6: stale `13`/`12` count comments across
  `Glyph.tsx`, `bigPalette.ts:16`, `BigGlyph.test.tsx:167`,
  `global.css:369,384`, `tokens.ts:59,133,151`.
- _Minor (high)_ — Phase 1: frontend README `/glyph-showcase` ("all 12
  doc-type Glyphs") and router comments carry fixed counts that go stale.
  _(Reviewer note: verified the showcase routes are still live on the target
  tree, retained as VR fixtures — so the count comments are stale but the
  manual-verification step referencing `/big-glyph-showcase` remains valid.)_
- _Minor (medium)_ — Manual Verification references to `/big-glyph-showcase`.
  _(Reviewer note: corrected — the route exists on this tree; no dead-route
  risk. The VR baselines for any `/dev` page are a separate concern.)_
- _Suggestion (medium)_ — Overview/Desired End State: the new "Operate"
  category needs no human-facing doc update; optionally add a one-line note if
  a closed-set lifecycle doc exists (it does not).

## Re-Review (Pass 2) — 2026-06-13T22:06:56+00:00

**Verdict:** COMMENT

The pass-1 edits resolved three of the four major findings outright and made the
fourth (related-artifacts) testable in principle. All seven lenses now lead with
strong-quality summaries. Two genuinely new majors surfaced — both about the
*new* test scaffolding the edits introduced, not the feature design — plus a
cluster of minor refinements. With 0 critical and 2 major (the documentation
"majors" the agents raised are down-weighted to minor here: they merely extend
the count-comment sweep the plan already commits to), the configured 3-major
REVISE threshold is not met. The plan is **acceptable as-is**; the items below
are improvements worth folding in before implementation.

### Previously Identified Issues

- 🟡 **Compatibility/Correctness/Standards**: count assertions under-enumerated —
  **Resolved.** Both lenses independently grep-verified the enumeration is now
  complete (`api_smoke.rs:102`, `docs.rs:249/296/376`, `api_types.rs:32`,
  `GlyphShowcase` 39→42, `BigGlyphShowcase` 13→14, `BigGlyph.test.tsx:69,81`); no
  remaining hard-coded 13/39 site is omitted, and `config.rs:460` /
  `config_contract.rs:46` are correctly left untouched.
- 🟡 **Test Coverage**: related-artifacts AC #5 had no test — **Partially
  resolved.** A server `api_related.rs` case is now named, but see the new major
  below: the proposed linkage form may not be one the server actually resolves.
- 🟡 **Test Coverage**: search AC #6 had no test — **Resolved.** `api_search.rs`
  + `SearchResultsPanel.test.tsx` cases added and verified feasible.
- 🟡 **Test Coverage**: zero-count AC #2 had no test — **Resolved.** A
  `LibraryOverviewHub.test.tsx` zero-count case is now specified.
- 🔵 **Architecture**: `in_lifecycle()` emitted a misleading `true` — **Resolved.**
  Now explicit `false` + comment + assertion; correctness re-verified the flag is
  discarded by the Sidebar (`void docTypes`) and the pipeline is governed by
  `participates_in_lifecycle`, so no surface drops RCA.
- 🔵 **Code Quality**: vacuous `GlyphDocType` — **Resolved** (documented as
  intentional headroom; retained deliberately).
- 🔵 **Code Quality/Standards**: short label "RCA" — **Resolved.** Recorded N/A;
  standards re-verified there is no doc-type short-label registry.
- 🔵 **Code Quality**: shared status-set blast radius — **Resolved** (note +
  `status-variant.test.ts` entries).
- 🔵 **Documentation/Standards/Code Quality**: stale count comments — **Partially
  resolved.** A broad sweep was added; see the new minor below — it omits a few
  comment sites and its enumerating grep is case-sensitive.
- 🔵 **Compatibility**: `start-server.mjs` docPaths drift — **Resolved** (noted,
  with a track-the-launcher comment).
- 🔵 **Correctness**: `latest` tie-break determinism — **Resolved** (single seeded
  RCA + `set_mtime_ms` fallback).
- 🔵 **Test Coverage**: bare `api_types.rs` length bump — **Resolved** (RCA
  `virtual:false`/`dirPath` property assertion added; see refinement below to
  also pin `count==1` and `inLifecycle==false`).

### New Issues Introduced

- 🟡 **Test Coverage** (high): The AC #5 `api_related.rs` test rests on an
  **unverified linkage channel**. The server's related resolution surfaces an
  entry only via a review `target:` or work-item cross-refs
  (`work_item_id:`/`parent:`/`related:` resolving to a work-item ID); the plan's
  suggested `relates_to: ["issue-research:…"]` form on a non-work-item, non-review
  artifact may resolve to nothing, making the proposed test assert against an
  empty list. This is also a latent *feature* question: confirm the
  related-artifacts resolver can surface an RCA at all. **Action**: pin the exact
  resolvable linkage (e.g. seed the inbound link from an artifact whose
  `related:`/`parent:` canonicalises to the RCA, or confirm RCA-targeted links are
  indexed); if none exists, state that AC #5's server half is the
  `RelatedArtifacts.test.tsx` synthetic-`IndexEntry` case + E2E, not a server
  resolution test.
- 🟡 **Test Coverage** (medium): Mutating the shared `seeded_cfg` (compiled into
  every server integration binary) is **under-guarded against perturbing existing
  suites** — e.g. `api_search.rs` cross-type match tests and `api_related.rs`'s
  `linked_count_equals_sum_of_related_lists`. **Action**: choose a provably
  non-colliding RCA title/slug, and add a success-criterion line that the *entire*
  `cargo test` server suite stays green after the `seeded_cfg` edit.
- 🔵 **Documentation/Standards** (minor; agents rated major): The count-comment
  sweep **omits** `template-tier.ts:30-33,62-63`, `global.css:119`
  ("Glyph-only doc type (not a server DocTypeKey)"), and
  `LibraryTemplatesIndex.test.tsx:93-97`, all of which assert RCA "is not a server
  doc type" and become false after Phase 2; and the enumerating grep
  (`grep -rEn 'thirteen|twelve|all 1[23]|glyph-only' src`) is **case-sensitive**
  so it misses capitalised "Glyph-only". **Action**: add those sites and make the
  grep `-i`.
- 🔵 **Compatibility** (minor): `api_smoke.rs`'s hand-rolled `doc_paths` config
  has drifted further from the launcher (single `research` key, missing several);
  bumping its length to 14 without adding `research_issues` means it reports RCA
  with a null `dirPath`. **Action**: add `("research_issues","research/issues")`
  to its config map when bumping the length.
- 🔵 **Compatibility** (minor): The `global.test.ts` WCAG-contrast suite iterates
  physical `DOC_TYPE_KEYS`, so it **auto-activates** for RCA in Phase 1 — confirm
  `#ab2c96` clears 3:1 against the light `--ac-bg` (it almost certainly does);
  worth a one-line note so a surprise red is anticipated.
- 🔵 **Correctness/Compatibility** (minor): Reconcile the Testing-Strategy claim
  with the Phase 2 detail — add wire-level `count==1` and `inLifecycle==false`
  assertions to the `api_types.rs` RCA block (currently only `virtual`/`dirPath`).
- 🔵 **Test Coverage** (minor): The `LibraryTypeView.test.tsx` listing case (and
  optionally the E2E) should assert the **specific** chip variant (resolved→green,
  monitoring→indigo), not mere chip presence, to be mutation-resistant.

### Assessment

The plan is in good shape and clears the bar for implementation. The two
remaining majors are both about the test scaffolding the pass-1 edits added: the
AC #5 linkage channel is the one worth resolving *before* coding (it touches
whether the related-artifacts behaviour is even constructible as described), and
the `seeded_cfg` blast-radius guard is a cheap insurance line. The minors are
mechanical sweep-completeness and assertion-tightening items. None are blocking;
folding in the AC #5 linkage confirmation and the `seeded_cfg` full-suite
success criterion would make the plan airtight.

## Re-Review (Pass 3) — 2026-06-13T23:16:21+00:00

**Verdict:** APPROVE

All pass-2 findings have been folded into the plan:

- 🟡 **AC #5 linkage channel** — Resolved. The plan now opens the
  related-artifacts work with a pre-flight to confirm which channel the server's
  resolver actually surfaces, seeds the linking fixture using that form, and makes
  the `api_related.rs` test contingent (falling back to the frontend
  `RelatedArtifacts.test.tsx` + E2E with a recorded gap if no inbound channel
  resolves to an RCA).
- 🟡 **`seeded_cfg` blast radius** — Resolved. Added a shared-fixture guard
  (non-colliding title/slug) and a success criterion that the entire `cargo test`
  server suite stays green after the edit.
- 🔵 **Count-comment sweep completeness** — Resolved. Added `template-tier.ts`,
  `global.css:119`, and `LibraryTemplatesIndex.test.tsx` sites; switched the
  enumerating grep to case-insensitive; added the per-table count caveat.
- 🔵 **`api_smoke.rs` config drift / `api_types` wire assertions / listing chip
  variant / `global.test.ts` WCAG auto-activation** — Resolved.
- 🔵 **E2E assertion depth** — Resolved. `navigation.spec.ts` now asserts the
  status badge (AC #3) and the related-artifacts link (AC #5), not just routing.

Three broader items (invert `in_lifecycle()` to an allow-list; a single shared
`doc_paths` source; a `/api/types` == `DOC_TYPE_KEYS` contract test) are recorded
as deferred follow-ups in the plan's *What We're NOT Doing* section rather than
expanded into this work.

### Assessment

The plan is sound, complete, and ready for implementation. No outstanding
blocking or major concerns remain. Marked **APPROVE**.
