---
type: plan-review
id: "2026-06-11-0085-humanise-detail-page-h1-review-1"
title: "Plan Review: Humanise Detail-Page H1 Across All Doc Kinds"
date: "2026-06-11T13:33:11+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-11-0085-humanise-detail-page-h1"
target: "plan:2026-06-11-0085-humanise-detail-page-h1"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, correctness, test-coverage, standards, documentation]
review_number: 1
review_pass: 2
tags: [backend, detail-page, frontmatter, humanise-slug]
last_updated: "2026-06-11T16:46:05+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Humanise Detail-Page H1 Across All Doc Kinds

**Verdict:** COMMENT

This is a tightly-scoped, architecturally sound, genuinely test-first plan: a
pure, config-independent `humanise_slug` helper placed in the already-public
`slug.rs`, wired into a single cascade layer of `title_from`, with a
table-driven AC2 suite and a concrete-literal oracle that give the change real
value-level bite. The placement decision is well-reasoned and avoids two real
architectural smells. The plan is acceptable as-is, but two **major** test-coverage
gaps are worth closing before implementation — both concern unpinned title
edge cases that can still produce a blank or stray-space `<h1>`, which would
quietly defeat the plan's own stated invariant.

Note: one agent raised a **critical** finding claiming the `mise run server:check`,
`format:server:check`, and `lint:server:check` tasks do not exist. I verified
directly against `mise tasks` — all four referenced tasks (including
`test:unit:visualiser`) exist exactly as named; the guardrails landed in the
most recent commit (`0bd003bfa`). That finding was a stale-tree artifact and has
been **dropped**.

### Cross-Cutting Themes

- **Unpinned title/segment edge cases produce blank or stray-space H1s**
  (flagged by: test-coverage, correctness) — A whitespace-only `title: "   "`
  is *not* caught by the `!s.is_empty()` guard and renders verbatim; and
  multi/leading/trailing hyphens (`"foo--bar"`, `"0042-"`) produce
  double/leading/trailing spaces via `split('-')` + `join(" ")`. Neither is
  fixtured, and both can defeat the plan's stated invariant that "no detail
  page renders an unhumanised stem as its `<h1>`."
- **Deferred `humanise_status` duplication is recorded in the plan, not in the
  code** (flagged by: architecture, code-quality, documentation) — Three lenses
  independently observed that `title_case_segment` is byte-for-byte identical to
  the existing `api::library::humanise_status`, and that nothing in the source
  cross-references the twins. A one-line comment on `title_case_segment` would
  make the deferred-unification decision discoverable from the code.
- **The 13-kind invariant loop is largely tautological** (flagged by:
  code-quality, test-coverage) — Because `title_from` is kind-agnostic, the
  per-iteration `assert_eq!(t, humanise_slug(stem))` compares the production
  output against the same helper it calls. Only the `assert_ne!` and the
  post-loop `"Test Fixture"` literal oracle carry real mutation-catching value.
  The plan already frames this as a forward-looking tripwire, which is fair.
- **Some load-bearing behaviour lives only in prose/tests, not the helper
  doc-comment** (flagged by: documentation, correctness, architecture) — The
  accepted `HHMMSS-` time-token quirk and the verbatim-stem-for-nested-kinds
  coupling are documented in the plan or a test comment but not where a
  maintainer reading `humanise_slug` would look.

### Tradeoff Analysis

No genuine inter-lens conflicts surfaced — the lenses are aligned. The only
latent decision is product-facing, not a quality tradeoff: whether a stray
6-digit timestamp (`123456 Architecture`) in a rare design-inventory fallback
H1 is acceptable. The plan pins it as accepted behaviour; confirm that is a
conscious product call.

### Findings

#### Critical

_None. (One critical finding from the Standards lens was raised but verified
false — the referenced mise tasks exist — and has been dropped.)_

#### Major

- 🟡 **Test Coverage**: Whitespace-only `frontmatter.title` is untested and
  behaves differently from empty
  **Location**: Phase 2, Section 4 (empty-title fixture, Open Q5)
  The `frontmatter.rs:294` guard is `!s.is_empty()`, so `title: "   "` returns
  `"   "` verbatim as the `<h1>` rather than falling through to the humanised
  stem. The plan fixtures the empty case but not the whitespace-only case,
  leaving a path that can still render a blank H1 — and pins neither behaviour.

- 🟡 **Test Coverage**: Title-caser degenerate inputs (empty segments from
  multi/edge hyphens) are unspecified and untested
  **Location**: Phase 1, Section 4 (humanise_slug AC2 table)
  `split('-')` on `"foo--bar"`, `"-foo"`, or `"0042-"` yields empty segments
  that `join(" ")` renders as double/leading/trailing spaces. These are
  realistic tool/hand-authored stems feeding a visible H1, and no fixture pins
  the result. The Correctness lens independently traced `"0042-"` → `"0042 "`
  (trailing space) as a latent defect.

#### Minor

- 🔵 **Correctness**: `"0042-"` / `"foo--bar"` produce stray spaces
  **Location**: Phase 1, Section 3 (humanise_slug / title_case_segment)
  Trailing-dash and double-hyphen stems produce trailing/doubled spaces; add
  fixtures and consider `.filter(|s| !s.is_empty())` before the join if
  undesirable. (Reinforces the major above.)

- 🔵 **Correctness**: `title_case_segment` "emit verbatim" framing is slightly
  misleading
  **Location**: Phase 1, Section 3
  The code unconditionally applies `first.to_uppercase()`; it works for digits
  only because digits have no uppercase mapping. A non-ASCII first char (e.g.
  `ß`→`SS`) would expand. Reword the comment; add a Unicode fixture if in scope.

- 🔵 **Correctness**: `HHMMSS-` quirk desirability not explicitly confirmed
  **Location**: Phase 1, Section 4 (HHMMSS row)
  `2026-05-21-123456-architecture` → `123456 Architecture` is correct against
  the code; confirm a stray timestamp token in the rare fallback H1 is a
  conscious product decision, not an artefact of reusing the date stripper.

- 🔵 **Architecture**: Bare-date guard correctness hinges on branch ordering +
  shared date predicate
  **Location**: Phase 1, Change 2; Key Discoveries (bare-date trap)
  The contract (date-with-tail → bare-date → id-strip) is encoded only as
  control flow. Retain the bare-date and both mixed-prefix fixtures permanently
  as the regression guard for the ordering.

- 🔵 **Architecture / Code Quality / Documentation**: `humanise_status`
  duplication not cross-referenced in code
  **Location**: Phase 1, Section 3; What We're NOT Doing
  `title_case_segment` is identical to `api::library::humanise_status`; add a
  one-line comment linking them so the deferred unification is discoverable.
  (Note: the Current State Analysis claim that they "share no structural logic"
  understates this — they are the same function.)

- 🔵 **Architecture**: Helper correctness for nested kinds is coupled to indexer
  stem derivation
  **Location**: Current State Analysis (call site); Desired End State
  For design-inventory kinds the stem is the parent directory name, not
  `inventory.md`. A one-line doc-comment note that the input is the slug-source
  stem (which may be a directory name) would make the `HHMMSS-` fixture's
  rationale self-documenting.

- 🔵 **Code Quality**: Two co-located date-length predicates (`>= 10` vs `< 11`)
  read as redundant
  **Location**: Phase 1, Change 2
  After extracting `is_iso_date_prefix` (`>= 10`), `strip_prefix_date_str` keeps
  its `< 11` guard. Add a comment distinguishing "date shape" from "date +
  trailing separator" so a maintainer doesn't "fix" one and break the
  bare-date-vs-dated-tail distinction.

- 🔵 **Code Quality / Test Coverage**: 13-kind loop's per-iteration `assert_eq`
  is tautological
  **Location**: Phase 2, Section 3 (all-doc-kinds invariant test)
  Consider dropping the `assert_eq!(t, humanise_slug(stem))` (keeping the
  `assert_ne!` and the literal oracle) so the test's coverage value matches its
  apparent scope. Acceptable as-is given the documented forward-looking intent.

- 🔵 **Standards**: New `is_iso_date_prefix` uses a different digit-check idiom
  than the code it refactors
  **Location**: Phase 1, Change 2
  `b[0..4].iter().all(u8::is_ascii_digit)` (method ref) vs the existing closure
  form in `strip_prefix_date_str`. Both clippy-clean; pick one form for file
  uniformity.

- 🔵 **Standards**: `_str` borrowed-variant suffix is being introduced, not
  consistently pre-existing
  **Location**: Phase 1, Change 1
  Only `strip_prefix_date_str` precedes it; `strip_optional_work_item_id_prefix`
  returns `&str` with no `_str` suffix. Proceed with `_str` (better precedent);
  optionally note the lone exception.

- 🔵 **Test Coverage**: Unicode / very-long-segment inputs to the title-caser
  are untested
  **Location**: Phase 1, Section 4; Testing Strategy
  The AC2 table is entirely ASCII. Either add one Unicode fixture or document
  the ASCII-only assumption in the helper doc-comment.

#### Suggestions

- 🔵 **Documentation**: Accepted `HHMMSS-` quirk documented only in a test
  comment
  **Location**: Phase 1, Changes #3 doc comment vs #4 test
  Add one clause to the `humanise_slug` doc comment naming the time-token case
  as accepted, so the rationale lives next to the code.

- 🔵 **Documentation**: Empty-title test comment re-embeds a volatile line number
  **Location**: Phase 2, Section 4 (empty-title fixture comment)
  The comment `// ... (line ~294 guard)` contradicts the plan's own
  volatile-line-number caveat. Name the guard by behaviour
  (`!s.is_empty()`) instead.

### Strengths

- ✅ Module placement (`slug.rs` over `api/library.rs`) is reasoned through
  dependency direction — avoids an upward `frontmatter → api` dependency and
  avoids promoting the private `api::library` module; verified against `lib.rs`.
- ✅ `humanise_slug` is a pure, side-effect-free `&str → String` function,
  co-located with the strippers it reuses — textbook functional-core separation
  and a clean unit-test boundary; the single call site already passes the stem
  verbatim so no wiring change is needed.
- ✅ Genuinely test-first: the AC2 table is written and watched failing first,
  and the bare-date fixture is what forces the correct guard structure over the
  naive composition. All nine oracle rows were traced by hand and match the
  proposed implementation exactly.
- ✅ The concrete-literal oracle (`"0042-test-fixture"` → `"Test Fixture"`) is a
  real defence against a shared bug in the oracle and the production path.
- ✅ The borrowed-stripper refactor is behaviour-preserving, keeping the
  `derive()` path byte-for-byte unchanged while avoiding an allocation.
- ✅ `is_iso_date_prefix` byte-slicing is panic-safe (all indices guarded by
  `len >= 10`; slicing `&[u8]` never hits a UTF-8 boundary panic).
- ✅ Tradeoffs are explicit rather than hidden: deferred `humanise.rs` module,
  declined idiom unification, kind-agnostic `title_from` — the right
  open/closed calls for a single new use site.
- ✅ AC5 is fully satisfied: each cascade layer carries a one-line comment
  naming source and position; the `humanise_slug` doc comment is accurate and
  complete against verified behaviour.

### Recommended Changes

1. **Pin the whitespace-only `frontmatter.title` behaviour** (addresses:
   "Whitespace-only frontmatter.title is untested")
   Decide whether `title: "   "` should fall through to the humanised stem. If
   yes, change the guard to `!s.trim().is_empty()` and assert the fall-through;
   if no, add a fixture asserting it returns `"   "` verbatim. Either way, pin
   it so the divergence from the empty case is deliberate.

2. **Add fixtures for empty-segment stems** (addresses: "Title-caser degenerate
   inputs", "`0042-` / `foo--bar` produce stray spaces")
   Add AC2 rows for at least `"0042-"` (trailing dash) and `"foo--bar"`
   (consecutive hyphens), pinning the chosen output. If stray/doubled spaces are
   undesirable, add `.filter(|s| !s.is_empty())` before the `join(" ")`.

3. **Cross-reference the `humanise_status` twin in code** (addresses:
   "`humanise_status` duplication not cross-referenced")
   Add a one-line comment on `title_case_segment` noting it mirrors
   `api::library::humanise_status` and that unification is deferred until a
   third humaniser appears. Correct the Current State Analysis claim that they
   "share no structural logic" — they are identical.

4. **Move the `HHMMSS-` quirk rationale into the helper doc-comment** (addresses:
   "Accepted HHMMSS- quirk documented only in a test comment", "HHMMSS quirk
   desirability not confirmed")
   Add a clause to the `humanise_slug` doc comment naming the accepted
   time-token behaviour, and confirm in the plan it is a conscious product call.

5. **Clarify the two date-length predicates and unify the digit-check idiom**
   (addresses: "Two co-located date-length predicates", "different digit-check
   idiom")
   Add a comment distinguishing `is_iso_date_prefix`'s `>= 10` (shape) from
   `strip_prefix_date_str`'s `< 11` (shape + trailing separator), and adopt one
   `all(...)` spelling across the file.

6. **(Optional) Trim the tautological loop assertion and reword the volatile
   line-number comment** (addresses: "13-kind loop tautological", "test comment
   re-embeds a line number")
   Drop `assert_eq!(t, humanise_slug(stem))` from the loop (keep `assert_ne!`
   and the literal oracle), and name the empty-title guard by behaviour
   (`!s.is_empty()`) rather than `line ~294`.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: A tightly scoped, architecturally sound change: a new
config-independent pure helper (`humanise_slug`) placed in the already-public
`slug.rs`, wired into a single cascade layer of `title_from`. The placement
decision is well-reasoned and explicitly avoids two genuine architectural
smells (promoting a private `api::library` module and introducing an upward
`frontmatter → api` dependency). The functional-core/imperative-shell
separation is exemplary, and the plan correctly identifies the change as a
defensive fallback layer rather than a primary path, with tradeoffs (deferred
module extraction, accepted idiom duplication) explicitly acknowledged.

**Strengths**:
- Module placement decision (slug.rs over api/library.rs) reasoned through
  dependency direction; verified against lib.rs (both crate-root `pub mod`).
- `humanise_slug` is a pure function co-located with the strippers it reuses,
  giving textbook functional-core separation and a clean unit-test boundary —
  no integration wiring needed (indexer.rs:1308 passes the stem verbatim).
- Tradeoffs explicitly acknowledged: deferred `humanise.rs`, declined idiom
  unification, kind-agnostic `title_from`.
- Borrowed-stripper refactor preserves byte-for-byte behaviour on `derive()`.
- Open/closed with respect to doc kinds; the all-kinds invariant test
  future-proofs against a later per-kind refactor.

**Findings**:
- 🔵 minor (medium): Bare-date guard depends on a subtle ordering invariant
  (`strip_prefix_date_str` → `is_iso_date_prefix` → id strip) encoded only as
  control flow. A future reorder or `len >= 11` change could silently
  reintroduce the year-eating bug. Retain the bare-date + mixed-prefix fixtures
  permanently as the regression guard.
  **Location**: Phase 1, Change 2; Key Discoveries (bare-date trap)
- 🔵 minor (medium): `humanise_status` and `title_case_segment` will duplicate
  the per-segment capitalise idiom across two modules with no cross-reference;
  cohesion of "display humanisation" is split. Add a one-line cross-reference
  comment.
  **Location**: What We're NOT Doing; Current State Analysis
- 🔵 minor (high): Helper correctness for the design-inventory kind is coupled
  to an upstream indexer transformation (parent-dir substitution for nested
  manifests). No code change needed; a doc-comment note that the input may be a
  directory name would make the `HHMMSS-` fixture self-documenting.
  **Location**: Desired End State; Current State Analysis (call site)

### Code Quality

**Summary**: A small, well-structured plan that swaps one cascade-layer return
for a new pure helper, with clear naming, thorough rustdoc, and table-driven
tests. The design favours simplicity appropriately: pure functions, no new
abstractions, and explicit deferral of premature extraction. The only concerns
are minor — a knowingly-deferred 4-line duplication and two near-identical
date-length predicates the refactor leaves side-by-side.

**Strengths**:
- `humanise_slug` is pure and trivially unit-testable in isolation.
- KISS/YAGNI applied deliberately (no `humanise.rs` module, no `DocTypeKey`
  param) and documented in "What We're NOT Doing".
- Thorough rustdoc plus one inline comment per cascade layer.
- Behaviour-preserving borrowed-stripper refactor isolates risk.
- Meaningful, intention-revealing naming; flat guard-clause style.

**Findings**:
- 🔵 minor (high): `title_case_segment` is a byte-for-byte duplicate of
  `humanise_status` with no cross-reference linking the two copies across
  modules. Add a one-line comment noting the mirror and deferred unification.
  **Location**: Phase 1, Change 3
- 🔵 minor (medium): Two co-located length predicates (`>= 10` in
  `is_iso_date_prefix` vs `< 11` in `strip_prefix_date_str`) can read as
  redundant/contradictory; a maintainer could "fix" one and break the
  bare-date distinction. Add a clarifying comment.
  **Location**: Phase 1, Change 2
- 🔵 minor (low): The 13-kind loop's per-iteration `assert_eq!(t,
  humanise_slug(stem))` is tautological today; only `assert_ne!` and the
  literal oracle have bite. Consider dropping the tautology. Acceptable given
  the documented intent.
  **Location**: Phase 2, Change 3

### Correctness

**Summary**: The plan is logically sound. All nine rows of
`humanise_slug_covers_ac2_cases` were traced by hand through the proposed
`strip_humanise_prefix` and `title_case_segment` against the actual current
behaviour of the strippers, and every asserted output is exactly what the code
produces. The bare-ISO-date trap is correctly defended, mixed-prefix single-pass
cases resolve correctly, empty-string is handled, and `is_iso_date_prefix`
byte-slicing is panic-safe. A few minor edge cases are worth flagging but none
are blocking.

**Strengths**:
- Bare-date guard ordering verified correct: `2026-05-21` → `2026 05 21`, not
  the naive `05 21`.
- Both mixed-prefix rows trace exactly (`2026-05-21-0042-foo` → `0042 Foo`;
  `0042-2026-05-21-foo` → `2026 05 21 Foo`).
- `is_iso_date_prefix` byte-slicing is panic-safe (indices guarded by
  `len >= 10`; `&[u8]` slicing never hits a char-boundary panic).
- Empty-string handled correctly (`""` → `""`).
- Borrowed-stripper refactor is genuinely behaviour-preserving.
- Empty-`title:` fall-through fixture is valid against the parser + guard.

**Findings**:
- 🔵 minor (medium): No oracle row for bare id (`"0042"`) or trailing-dash
  (`"0042-"` → `"0042 "`, trailing space). Empty trailing segments produce
  stray spaces in the H1. Add rows for `"0042-"` and `"foo--bar"`; filter empty
  segments if undesirable.
  **Location**: Phase 1, Section 3
- 🔵 minor (medium): The "emit verbatim" doc framing is misleading —
  `first.to_uppercase()` is applied unconditionally and works for digits only
  because they have no uppercase. A non-ASCII first char (`ß`→`SS`) would
  expand. Reword; add a non-ASCII row if in scope.
  **Location**: Phase 1, Section 3
- 🔵 minor (low): The `HHMMSS-` row is correct against the code, but surfacing a
  raw 6-digit timestamp in the H1 is debatable. Confirm it's a conscious
  product decision.
  **Location**: Phase 1, Section 4

### Test Coverage

**Summary**: The plan is genuinely test-first and the AC2 table plus the
concrete-literal oracle give the helper real value-level bite, with sensible
coverage of the simple-split, prefix-strip, mixed-prefix, single-segment,
bare-date, and empty-string cases. However the AC2 table is not exhaustive: the
title-caser's degenerate inputs (empty segments from consecutive/leading/
trailing hyphens, and Unicode first chars) are unspecified and untested, and the
cascade's whitespace-only `frontmatter.title` branch is left unpinned despite
the task calling it out. The 13-kind loop is, by the plan's own admission, a
tautological pass.

**Strengths**:
- Genuinely test-first; the bare-date fixture forces the correct guard.
- The concrete-literal oracle defends against a shared oracle/prod bug.
- Empty-string and bare-date degenerate cases are fixtured, not just asserted in
  prose; the stale stem-fallback test is correctly updated.
- Coverage is proportional to risk; correctly declines integration/E2E.

**Findings**:
- 🟡 major (high): Whitespace-only `title: "   "` is not caught by `!s.is_empty()`
  and renders verbatim as the H1 — a blank-H1 path the plan's invariant claims
  to prevent. Neither behaviour is pinned. Add a fixture (and decide whether to
  switch the guard to `!s.trim().is_empty()`).
  **Location**: Phase 2, Section 4 (Open Q5)
- 🟡 major (high): Empty-segment cases (`"foo--bar"`, `"-foo"`, `"foo-"`) produce
  double/leading/trailing spaces via `split('-')`+`join(" ")`; no fixture pins
  the result, and these are realistic stems feeding a visible H1. Add fixtures
  for at least one consecutive- and one trailing-hyphen stem.
  **Location**: Phase 1, Section 4
- 🔵 minor (medium): The 13-kind loop's `assert_eq!(t, humanise_slug(stem))` is
  tautological today; only `assert_ne!` and the literal oracle catch mutations.
  Consider dropping the tautology so coverage matches apparent scope.
  **Location**: Key Discoveries / Phase 2, Section 3
- 🔵 minor (low): Unicode and very-long-segment inputs to the title-caser are
  untested (`to_uppercase()` is multi-char-yielding). Add a Unicode fixture or
  document the ASCII-only assumption.
  **Location**: Phase 1, Section 4 / Testing Strategy

### Standards

**Summary**: The plan is strongly aligned with established Rust conventions in
`slug.rs`: `humanise_slug` is correctly a `pub fn` co-located with the
config-independent strippers, the `_str` borrowed-variant suffix is idiomatic,
and `title_case_segment` is a structural match of the existing `humanise_status`
precedent (clippy-clean by construction). Inline-comment, visibility, and naming
conventions are respected.

> **Reviewer note:** This lens additionally raised a *critical* finding claiming
> the `mise run server:check`, `format:server:check`, and `lint:server:check`
> tasks do not exist in `mise.toml`. This was **verified false** during
> aggregation — `mise tasks` lists all four referenced tasks (`server:check`,
> `format:server:check`, `lint:server:check`, `test:unit:visualiser`) exactly as
> named; the rustfmt/clippy guardrails landed in commit `0bd003bfa`. The agent
> reviewed against a stale tree. The critical finding has been **dropped** and is
> recorded here only for traceability.

**Strengths**:
- `humanise_slug` as a plain `pub fn` matches existing `slug.rs` visibility
  convention (`pub fn derive`, …); avoids promoting the private `api::library`.
- `_str` borrowed-variant suffix is idiomatic; the owned→borrowed delegation
  keeps `derive()` byte-identical.
- `title_case_segment` mirrors the proven `humanise_status` idiom, inheriting
  clippy `-D warnings` compliance.
- `DocTypeKey::all()` confirmed to return exactly 13 variants.
- TDD ordering, inline cascade comments, and the `use crate::slug::…` import
  follow established patterns.

**Findings**:
- 🔴 critical (high) — **DROPPED, verified false**: Claimed `server:check` /
  `format:server:check` / `lint:server:check` tasks do not exist. They do (see
  reviewer note above).
  **Location**: Desired End State; both phases' Automated Verification
- 🔵 minor (medium): `is_iso_date_prefix` uses `all(u8::is_ascii_digit)`
  (method-ref) while `strip_prefix_date_str` uses the closure form; both
  clippy-clean but inconsistent. Pick one form.
  **Location**: Phase 1, Change 2
- 🔵 minor (medium): The `_str` suffix convention is being established here, not
  consistently pre-existing (`strip_optional_work_item_id_prefix` returns `&str`
  without it). Proceed with `_str`; optionally note the exception.
  **Location**: Phase 1, Change 1

### Documentation

**Summary**: Unusually documentation-conscious for an internal utility: the
`humanise_slug` doc comment is accurate against the real stripper behaviour, the
AC5 inline cascade comments correctly name each layer's source and position, and
the bare-date guard comment matches the code. The main gaps are that two pieces
of load-bearing behaviour (the accepted `HHMMSS-` quirk and the deliberate
non-unification with the identical `humanise_status`) live only in the plan or a
test comment rather than the helper itself, and one test comment re-embeds a
volatile line number.

**Strengths**:
- The `humanise_slug` doc comment is accurate and complete (every clause traced
  against the real strippers).
- AC5 fully satisfied — each cascade layer names source + position; layer 3 also
  states the why.
- The bare-date guard comment accurately describes the non-obvious control flow.
- Doc-comment style matches existing `slug.rs`/`frontmatter.rs` conventions.
- The untested empty-`title:` branch is explicitly recorded with a fixture and
  explanatory comment.

**Findings**:
- 🔵 minor (high): The accepted `HHMMSS-` quirk is documented only in a test
  comment, not the helper doc comment where a maintainer would look. Add a
  clause to the `humanise_slug` doc comment.
  **Location**: Phase 1, Changes #3 vs #4
- 🔵 minor (high): The deferred unification with the identical `humanise_status`
  is documented in the plan but not the code; nothing points the twins at each
  other. (The Current State Analysis claim they "share no structural logic"
  understates this — they are the same function.) Add a cross-reference comment.
  **Location**: Phase 1, Change 3; What We're NOT Doing
- 🔵 suggestion (medium): The empty-title test comment re-embeds `line ~294`,
  contradicting the plan's own volatile-line-number caveat. Name the guard by
  behaviour instead.
  **Location**: Phase 2, Change 4

---

## Re-Review (Pass 2) — 2026-06-11T15:29:23+00:00

**Verdict:** APPROVE

Re-ran all six lenses against the revised plan. Both prior **major** findings
are **resolved**, and the documentation/comment fixes (recs 3–6) all landed and
verify against source. The revision introduced **one new major** — a
test-validity defect in the whitespace-only `title:` fixture — which was
**fixed during this pass** by driving `title_from` with a directly-constructed
`Parsed` state instead of round-tripping a panic-prone quoted scalar through
`parse()`. No outstanding majors remain; the residual findings are minor
observations and two small judgment calls (guard an empty-tail date stem; trim
non-blank titles on return).

### Previously Identified Issues

- 🟡 **Test Coverage**: Whitespace-only `frontmatter.title` untested —
  **Resolved** (guard tightened to `!s.trim().is_empty()`; fixture added — see
  new finding below for the follow-on fix to the fixture's validity).
- 🟡 **Test Coverage**: Title-caser empty-segment inputs untested —
  **Resolved** (`.filter(|s| !s.is_empty())` added; AC2 rows `"foo--bar"` →
  `"Foo Bar"` and `"0042-"` → `"0042"` genuinely exercise the filter, traced by
  the correctness lens).
- 🔵 **Architecture/Code-Quality/Documentation**: `humanise_status` twin not
  cross-referenced — **Resolved** (cross-reference doc comment added on
  `title_case_segment`; Current State Analysis claim corrected).
- 🔵 **Documentation/Correctness**: `HHMMSS-` quirk + nested-kind stem not on
  the helper — **Resolved** (doc comment expanded).
- 🔵 **Code-Quality/Standards**: two date-length predicates / digit idiom —
  **Resolved** (clarifying prose added; predicate switched to the closure
  idiom; shape-vs-separator division documented).
- 🔵 **Code-Quality/Test-Coverage**: tautological loop assertion —
  **Resolved** (`assert_eq!(t, humanise_slug(stem))` dropped; unused import
  removed; intent comment retained).
- 🔵 **Documentation**: volatile line number in test comment — **Resolved**
  (comment now names the `!s.trim().is_empty()` guard by behaviour).

### New Issues Introduced

- 🟡 **Test Coverage** (high): The whitespace-only `title: "   "` fixture
  routed through `parse()` on a quoted whitespace-only scalar, which trips
  libyml's trailing-whitespace panic and is caught as `FrontmatterState::
  Malformed` (per `parse()` at frontmatter.rs:146-156 and the
  `malformed_when_quoted_scalar_has_trailing_whitespace` test). The cascade
  would then fall through to layer 3 regardless of the guard, so the assertion
  passed for the wrong reason — green even if `trim()` were reverted.
  **Status: FIXED this pass** — the fixture now builds `FrontmatterState::
  Parsed` directly and calls `title_from` at the boundary, with no `parse()`
  dependency, so the `!s.trim().is_empty()` guard is genuinely the code under
  test. (Verified the `Parsed(BTreeMap<String, serde_json::Value>)` shape
  against frontmatter.rs:170-197.)

### Residual Minor Findings / Open Judgment Calls

- 🔵 **Correctness** (high): A date stem with an empty descriptive tail
  (`"2026-05-21-"`) strips to `""` and humanises to an empty string — a blank
  `<h1>`, the very thing the change exists to prevent. **Status: FIXED this
  pass** — `strip_humanise_prefix` now skips an empty date-strip tail and falls
  through to the bare-date guard (`"2026-05-21-"` → `"2026 05 21"`); AC2 row
  added.
- 🔵 **Correctness/Standards** (medium): The layer-1 guard tested `s.trim()`
  but returned `s.to_string()` (untrimmed), so a padded non-blank title
  (`"  Foo  "`) rendered with surrounding whitespace, unlike layer 2. **Status:
  FIXED this pass** — layer 1 now returns `s.trim().to_string()`, normalising
  all three cascade layers uniformly.
- 🔵 **Code-Quality/Architecture/Documentation** (low–medium): the
  `title_case_segment` ↔ `humanise_status` cross-reference is one-directional;
  a back-reference on `humanise_status` would make the twin discoverable from
  the api side (accepted as-is given the clean two-file diff).
- 🔵 **Standards** (low): per-segment capitalisation title-cases short words
  (`Vs`); consistent with the existing `humanise_status` precedent — no change.
- 🔵 **Documentation** (low): References section still cites volatile line
  ranges the plan elsewhere disclaims; prefer symbol-anchored references.
- 🔵 **Test Coverage** (suggestion): the 13-kind loop cannot detect a per-kind
  regression while `title_from` is kind-agnostic; comment already records this
  as load-bearing only once the signature takes a `DocTypeKey`.

### Assessment

**APPROVED.** The plan is ready for implementation. Both original majors are
closed with real mutation-catching coverage, the introduced test-validity
major has been fixed, and the two on-theme correctness improvements (empty-tail
date guard; trim-on-return) have both been applied this pass. The documentation
is accurate against source. No critical findings; no outstanding majors; the
only remaining items are low-priority observations explicitly accepted as-is.
