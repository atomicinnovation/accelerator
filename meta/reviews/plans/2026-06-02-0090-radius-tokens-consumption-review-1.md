---
date: "2026-06-03T00:00:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-06-02-0090-radius-tokens-consumption.md"
review_number: 1
verdict: APPROVE
lenses: [correctness, test-coverage, architecture, code-quality, documentation, standards, usability]
review_pass: 2
status: complete
---

## Plan Review: Radius Tokens Consumption (0090)

**Verdict:** REVISE

The plan is unusually disciplined for a mechanical token migration: it correctly
treats the always-on Vitest harness as the test oracle, respects the four-touch
migration contract (CSS + `global.css` + `tokens.ts` + `migration.test.ts`),
faithfully mirrors the proven 0075 migration-first/enforcement-last phasing, and
documents its authorised AC deviations in a dedicated section. It is close to
implementable. What pushes it to REVISE is a single concrete frontmatter bug
flagged independently by three lenses, a clutch of test-soundness gaps that would
let a wrong token value ship undetected, and the fact that the plan's central
deliberate decision — the px-encoded ladder — is asserted rather than argued and
carries real, under-mitigated costs that four lenses surfaced. None of these are
hard to fix; most are precision/documentation edits to the plan text.

### Cross-Cutting Themes

- **ADR-0026 `superseded_by` is single-cardinality and already occupied** (flagged
  by: Architecture, Documentation, Standards — all high confidence). Phase 1 §2
  says "add reciprocal frontmatter `superseded_by: "adr:ADR-0039"`", but ADR-0026
  already carries `superseded_by: "adr:ADR-0036"` from the 0075 amendment, and
  ADR-0034 defines `superseded_by` as a **single ref**. Following the instruction
  literally either silently overwrites the ADR-0036 edge or writes a
  non-conformant list. The unanimous recommendation: rely on the canonical
  writable side — ADR-0039's `supersedes: ["adr:ADR-0026"]` already makes the edge
  derivable — and leave ADR-0026's existing `superseded_by` untouched. This is the
  single clearest defect in the plan.

- **The px-encoded naming policy is the plan's biggest decision and its
  rationale/guardrails are thin** (flagged by: Documentation, Usability,
  Architecture, Standards). It contradicts the work item's AC1/AC8 (authorised),
  but: the ADR-0039 rationale only *states* the policy rather than arguing it from
  the `3px`/`--radius-block` tensions the research surfaced (Documentation); the
  numeric suffix *collides* with the sibling `--sp-N` spacing scale where the
  suffix is an ordinal index, not a px value — `--sp-1`=4px but `--radius-12`=12px
  (Usability, high); px-encoding lowers the friction against minting off-scale
  tokens like `--radius-5`/`--radius-7` (Usability); names couple identity to
  value, so a future value retune forces a rename cascade (Architecture); and the
  "semantic vs measurement" vocabulary is described inconsistently across the plan
  (Standards). The decision can stand — it is the author's to make — but ADR-0039
  and the `global.css` comment should *argue* it, call out the `--sp-N` collision,
  and state an explicit closed-set extension policy.

- **Test soundness for "zero visual change" has gaps** (flagged by: Test
  Coverage, with Correctness corroboration). The plan's only behavioural guarantee
  is the Playwright spec, and for the three `50%` selectors it pins a box-derived
  px string that cannot distinguish `--radius-full`(50%) from a coincidentally
  equal px token; EmptyState `.card` (and every renamed `--radius-12`/`-lg` site)
  has no computed-value assertion, so a typo like `--radius-12: 1px` would pass
  every automated check; and several selectors depend on route/setup mounting whose
  reachability is asserted but unverified.

- **The in-test gate regex deliberately admits vendor-prefixed literals** (flagged
  by: Correctness, Code Quality, Standards). The `(?<![\w-])` lookbehind that
  whitelists `-webkit-border-radius: var(--radius-2)` also silently admits
  `-webkit-border-radius: 6px`, diverging from the AC3 sweeps the gate claims to
  approximate. No such declaration exists today, so this is latent — but it should
  be a recorded decision, not an unexplained boundary.

### Tradeoff Analysis

- **Self-documenting value lookup (Usability +) vs scale integrity & cross-family
  consistency (Usability −, Architecture −).** Px-encoding means a developer types
  `var(--radius-6)` for a measured 6px with no t-shirt translation — genuinely
  lower friction. The cost is a fourth naming model in a token system that already
  has ordinal (`--sp-N`) and t-shirt (`--size-sm`, old `--radius-sm`) families, and
  weaker structural friction against scale sprawl. Recommendation: keep the
  decision (author-authorised) but mitigate via ADR-0039 (argue it, name the
  `--sp-N` collision, state a closed-set/sign-off extension rule) rather than
  reverting to semantic names.

- **AC3 grep strictness (forces `0`/`50%` tokenisation) vs naming clarity.**
  Tokenising `0` as `--radius-0` keeps the AC3 `[.0-9]` sweep strict (good), but
  `--radius-0` reads as "smallest px step" rather than "no rounding"; Usability
  suggests `--radius-none`. This trades a slightly muddier name for a stricter,
  simpler gate — a reasonable call, worth stating explicitly.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Architecture / Documentation / Standards**: ADR-0026 `superseded_by`
  single-cardinality collision (merged, high confidence)
  **Location**: Phase 1, §2 (Amend ADR-0026 §3 — frontmatter)
  ADR-0026 already holds `superseded_by: "adr:ADR-0036"`; ADR-0034 defines the
  field as single-ref. "Add `superseded_by: "adr:ADR-0039"`" would overwrite the
  ADR-0036 edge or produce a non-conformant list. Rely on ADR-0039's
  `supersedes: ["adr:ADR-0026"]` (canonical writable side) and leave ADR-0026's
  `superseded_by` untouched.

- 🟡 **Correctness**: FilterPill `:224` `3px` row is pure-radius — must be
  deleted, not decremented (high)
  **Location**: Phase 5, §2 (EXCEPTIONS cleanup)
  All three `3px` occurrences in FilterPill (`:84`, `:160`, `:174`) are
  `border-radius`; after migration the observed `3px` count is 0. "Decrement"
  leaves `declared > observed` and fails the hygiene gate, breaking Phase 5's
  green-suite guarantee. Reclassify as "delete (pure radius)" like `:69`/`:78`.

- 🟡 **Test Coverage**: The three `50%` selectors assert a box-px value, not that
  the token resolves to `50%` (high)
  **Location**: Phase 3 (Playwright spec — `50%` selectors)
  Pinning `~4px`/`~5px`/`~2.5px` cannot distinguish `--radius-full`(50%) from a
  px token producing the same used value, and is fragile to box-size change. Add a
  box-dimension assertion, or lean on a unit assertion that
  `RADIUS_TOKENS['radius-full'] === '50%'` and treat the Playwright px as a smoke
  check.

- 🟡 **Test Coverage**: No automated check asserts a token's *resolved value*;
  EmptyState `.card` (12px) and all renamed sites can ship a wrong value
  undetected (medium)
  **Location**: Phase 3 / Phase 6 (EmptyState excluded; token values untested)
  AC3 proves no literal remains; the var-resolution test proves the *name* exists.
  Neither proves `--radius-12: 12px`. A cheap fix closes this for every new token:
  unit-assert `RADIUS_TOKENS` values (`'radius-12' === '12px'`, `'radius-0' === '0'`,
  `'radius-full' === '50%'`, etc.), independent of route reachability.

- 🟡 **Test Coverage**: Interaction-/route-gated selectors (RelatedArtifacts
  `.badge`, FilterPill options, Sidebar search) are asserted to mount but not
  verified (medium)
  **Location**: Phase 3 (route/setup mounting)
  The existing aside-row spec reaches the `0099`/`ADR-0099` fixture's work-items
  only as a *cluster sibling*, not a direct `.badge` mount. Add explicit
  `waitFor()` on each target in `setup` and confirm `.badge` renders at the chosen
  route during the Phase 3 baseline before pinning expected values.

- 🟡 **Documentation**: px-encoded naming rationale is asserted, not argued, in
  the ADR that must justify a work-item deviation (high)
  **Location**: Phase 1, §1 (ADR-0039 naming-policy clause); Authorised Deviations #1
  ADR-0039 is the durable record reviewers will consult; its Decision
  Drivers / Considered Options should capture the use-case-naming failure
  (`3px` spans six unrelated surfaces; `--radius-block` mis-describes 5/7 sites),
  mirroring how ADR-0036 argued numeric names against t-shirt prefixes.

- 🟡 **Usability**: Numeric-suffix radius tokens collide with the ordinal-index
  meaning of `--sp-N` spacing tokens (high)
  **Location**: Phase 2 (scale rename); Authorised Deviations #1
  `--sp-1`=4px, `--sp-2`=8px (suffix = step index), but `--radius-2`=2px,
  `--radius-12`=12px (suffix = px). Two numeric-suffix families in one file with
  opposite meanings is a least-surprise violation. If px-encoding is kept, call
  the collision out explicitly in ADR-0039 and the `global.css` comment.

- 🟡 **Usability**: Px-encoded names lower the friction against minting off-scale
  tokens (`--radius-5`, `--radius-7`) and eroding the scale (medium)
  **Location**: Authorised Deviations #1; ADR-0039 "Scale extension policy"
  The only stated guardrail is prose ("extend the ladder"). Add an explicit
  closed-set policy (new steps need recorded rationale + sign-off) and consider an
  allow-list assertion over declared `--radius-*` names so adding a step is a
  visible, reviewed change.

#### Minor

- 🔵 **Correctness / Test Coverage / Code Quality**: Sidebar `:109` `4px` is a
  deferred conditional describing a literal that does not exist (merged)
  **Location**: Phase 5, §2 ("Verify :109 Sidebar 4px")
  There is no `border-radius: 4px` in Sidebar (scrollbar-thumb radii are already
  `var(--radius-sm)`; the `4px` occurrences are non-radius). The reason string is
  stale. Resolve the branch before Phase 5: state definitively that `:109` is
  non-radius, leave count at 4, optionally fix the stale reason. Also confirm the
  Phase 7 gate runs over the same files as the AC3 sweep so any stray `4px` radius
  is caught automatically, not just by manual grep.

- 🔵 **Correctness / Code Quality / Standards**: `BORDER_RADIUS_LITERAL_RE`
  lookbehind silently admits vendor-prefixed literals (merged, medium)
  **Location**: Phase 7, §1 (gate regex)
  `-webkit-border-radius: 6px` escapes the gate while AC3's plain grep would too —
  neither layer catches it. Either anchor the match to allow an optional
  `-webkit-`/`-moz-` prefix while still requiring `[.0-9]`, or document the
  exclusion as an accepted limitation in the gate comment.

- 🔵 **Correctness**: AC5_FLOOR bump is framed as "read the failure", but a
  stale-low floor does not fail (high)
  **Location**: Phase 7, §3
  Both ratchet assertions (`observed >= AC5_FLOOR`, `AC5_FLOOR <= observed`) stay
  green while the floor is low. Reframe as a discipline step: set AC5_FLOOR to the
  observed count printed in the passing test name, then append the bump comment.

- 🔵 **Correctness / Test Coverage**: Playwright spec omits the `.codeblock pre`
  (`0` reset) selector, contradicting its own checklist (medium)
  **Location**: Phase 3 (CASES vs manual-verification checkbox)
  Add `.codeblock pre` (expected `'0px'`) to CASES, or explicitly carve it out
  with a stated rationale like EmptyState.

- 🔵 **Test Coverage**: Gate fixtures omit a bare-`0` custom-property definition
  and logical-property longhands (medium)
  **Location**: Phase 7, §2 (regex fixtures)
  Add negatives `--radius-0: 0;` / `--my-radius: 0;` and decide explicitly whether
  `border-start-start-radius` (logical) is in scope.

- 🔵 **Test Coverage**: `50%`/`0` migrations have no per-phase hygiene driver
  (medium)
  **Location**: Testing Strategy / Phase 4
  They carry no EXCEPTIONS entry, so a forgotten migration turns nothing red until
  Phase 7. Acknowledge they are guarded only by the enforcement-last gate (and the
  Phase 3 spec).

- 🔵 **Code Quality**: New gate duplicates comment-stripping instead of reusing
  the existing `stripComments` helper (high)
  **Location**: Phase 7, §1
  Hoist/reference the existing helper so the two literal-ban gates share one
  definition.

- 🔵 **Code Quality**: EXCEPTIONS `reason` prose is unvalidated and will drift
  during decrement work (high)
  **Location**: Phases 5-6
  Make the AC5 backstop mechanical: a test asserting no `kind: 'irreducible'`
  reason contains `border-radius`/`radius` after migration, rather than trusting
  manual rewrites.

- 🔵 **Architecture**: Px-encoded names couple token identity to value, weakening
  evolvability (medium)
  **Location**: Overview / Authorised Deviations #1
  Note in ADR-0039 Consequences that value changes are handled by adding a new
  ladder step and re-pointing consumers, not mutating a token's value in place.

- 🔵 **Architecture**: In-test gate regex vs the three AC3 sweeps are parallel
  definitions of one invariant (medium)
  **Location**: Phase 7
  Add a one-line assertion in the gate header that the in-test regex is the
  authoritative superset of the sweeps, and exercise the divergence cases
  (`0`, `50%`, all four corners) in fixtures.

- 🔵 **Documentation**: Proposed `global.css` comment is thinner than the
  typography precedent and omits the naming convention (medium)
  **Location**: Phase 2, §1
  Extend it to name the convention (px-encoded measurements; `--radius-pill`/
  `--radius-full` semantic), mirroring the self-introducing `--size-*` block.

- 🔵 **Documentation**: PR-description requirement doesn't pin the
  px-omission/`0`/`50%` rationale (medium)
  **Location**: Phase 8, §1
  State that the ladder enumerates only consumed values and why `0`/`50%` deviate,
  so the model is self-explaining from the PR alone.

- 🔵 **Standards**: ADR-0039 claims to supersede 0033's t-shirt naming without a
  linkage (and cites a bare work-item number) (medium)
  **Location**: Phase 1, §1
  Reword to reference the ADR-0026 convention (already in `supersedes`) and keep
  work-item IDs in References only.

- 🔵 **Standards**: Semantic-vs-scale token vocabulary is described inconsistently
  across the plan (medium)
  **Location**: Overview / Authorised Deviations #1 / Phase 1
  Fix one canonical two-bucket vocabulary and use it verbatim in ADR-0039, the
  `global.css` comment, and the PR description.

- 🔵 **Standards**: 0075's multi-line self-introducing comment + ordering
  precedent isn't explicitly carried over (medium)
  **Location**: Phase 2, §1
  Either match the multi-line comment or state the terser form is deliberate; pin
  the intended sort direction.

- 🔵 **Usability**: Gate failure gives no actionable guidance to a developer who
  writes a literal (medium)
  **Location**: Phase 7 (`expect(hits).toEqual([])`)
  Add a custom failure message teaching the rule ("must use `var(--radius-*)` per
  ADR-0039; off-scale values need a new token, not a literal").

- 🔵 **Usability**: `--radius-0` and `--radius-full` straddle both conventions
  confusingly (medium)
  **Location**: Authorised Deviations #2-3; Phase 2
  Consider `--radius-none` for the reset and group measurement vs intent tokens
  visually in the `global.css` comment.

- 🔵 **Documentation**: Reason-rewrite instruction lacks a target wording /
  consistency gate in Phases 4 & 6 (low)
  **Location**: Phases 4-6 EXCEPTIONS cleanup
  Add a worked example of a rewritten reason and extend the AC5 "no reason
  references border-radius" grep to Phases 4/6 (currently listed only for Phase 5).

- 🔵 **Usability**: Whole-scale rename breaks existing `sm/md/lg` muscle memory
  with no aliasing period (low)
  **Location**: Overview (43-site rename)
  If the rename proceeds, ADR-0039 should justify why radius alone diverges from
  the t-shirt convention typography retains.

#### Suggestions

- 🔵 **Architecture**: EmptyState AC3-only backstop is acceptable as-is given the
  exact `12px`→`--radius-12` match; fold it into the spec later if a cheap
  empty-listing mount appears.
- 🔵 **Code Quality**: Pin the Phase 2 rename to the full `var(--radius-sm)` form
  (parens included) and treat the `rg 'radius-(sm|md|lg)'` zero-match check as the
  authoritative completeness gate.

### Strengths

- ✅ Correctly identifies the enforcement model as a Vitest admission ledger (not
  CI grep) and threads every phase through the harness's interlocking gates
  (var-resolution, `observed === declared` hygiene, AC5_FLOOR ratchet, 0038 pill
  allow-list), keeping the test oracle authoritative.
- ✅ Migration-first / enforcement-last phasing keeps the suite green and every
  commit independently committable; the literal-ban gate lands only after all
  literals are migrated.
- ✅ The literal-migration map enumerates exactly the 25 inventory sites; an
  independent sweep returns precisely that set (no missed longhand-corner or
  bare-`0` declarations).
- ✅ "Drive final counts off the hygiene-test failure messages, do not
  hand-compute" is the right call for the mixed `(file, literal)` rows — a
  self-correcting procedure that removes a class of transcription errors.
- ✅ Routing `50%` to a distinct `--radius-full` (not `--radius-pill`) is correct:
  it preserves box-dependent computed values and keeps the three non-allow-listed
  files clean against the 0038 pill gate.
- ✅ ADR-0039 scoping (supersedes ADR-0026 §3 radius row only; leaves the 1px/2px
  border-width row and incidental prose intact) respects ADR immutability and
  mirrors the ADR-0036 partial-supersession precedent.
- ✅ The single-PR-with-epic-split contingency is bounded by a concrete,
  precedent-derived threshold (>~1500 non-test lines; 0075 shipped 35/9).
- ✅ Authorised Deviations section records every departure from the work item and
  frames them as the contract, so the validation step won't read them as failures.

### Recommended Changes

1. **Fix the ADR-0026 `superseded_by` instruction** (addresses: superseded_by
   cardinality collision). In Phase 1 §2, replace "add reciprocal frontmatter
   `superseded_by: "adr:ADR-0039"`" with: leave ADR-0026's existing
   `superseded_by: "adr:ADR-0036"` untouched and rely on ADR-0039's
   `supersedes: ["adr:ADR-0026"]` (the canonical, derivable side per ADR-0034).
   Add a success criterion verifying the ADR-0036 linkage is not lost.

2. **Reclassify FilterPill `:224` as a deletion** (addresses: FilterPill pure-radius
   row). In Phase 5 §2 change the `:224` action from "decrement" to "delete (pure
   radius)", matching `:69`/`:78`.

3. **Add direct token-value unit assertions** (addresses: EmptyState/token-value
   untested; weak `50%` assertion). Add a small unit test asserting every
   `RADIUS_TOKENS` value (`'radius-0' === '0'`, `'radius-12' === '12px'`,
   `'radius-full' === '50%'`, …). This closes the EmptyState gap without a new
   fixture and gives the `50%` case a value-level guarantee independent of the
   box-px Playwright check.

4. **Harden the Playwright spec** (addresses: `50%` assertion; route mounting;
   missing `.codeblock pre`). Add box-dimension assertions (or a documented
   reliance on #3) for the three `50%` selectors; add `waitFor()` on each
   interaction-gated target in `setup` and verify RelatedArtifacts `.badge`
   actually renders at the chosen route during baseline; add `.codeblock pre`
   (`'0px'`) to CASES or carve it out explicitly.

5. **Argue the px-encoded policy in ADR-0039, not just state it** (addresses:
   rationale asserted-not-argued; `--sp-N` collision; off-scale invitation;
   value-coupling; vocabulary inconsistency; reset naming). In ADR-0039: (a)
   capture the `3px`/`--radius-block` use-case-naming failure as the justification;
   (b) explicitly name the `--sp-N` ordinal collision; (c) state a closed-set
   extension policy (new steps need recorded rationale/sign-off); (d) note value
   changes add a step rather than mutating in place; (e) fix one canonical
   "measurement vs shape-intent" vocabulary and reuse it verbatim in the
   `global.css` comment and PR description; (f) reconsider `--radius-none` vs
   `--radius-0` for the reset.

6. **Resolve the `:109` Sidebar `4px` conditional before implementation**
   (addresses: deferred conditional). State definitively that `:109` is non-radius,
   leave count at 4, and remove the if/else from the execution steps.

7. **Record the vendor-prefix gate boundary** (addresses: lookbehind admits
   prefixed literals). Either widen the regex to flag prefixed literals or add a
   gate comment documenting the exclusion as an accepted limitation, and note the
   intentional divergence from the AC3 sweeps.

8. **Make EXCEPTIONS reason hygiene mechanical and reframe the AC5_FLOOR bump**
   (addresses: reason drift; AC5_FLOOR "read the failure"). Add a test asserting no
   irreducible `reason` references `border-radius`/`radius` post-migration; reframe
   the AC5_FLOOR step as "set to the observed count in the passing test name" with
   the bump comment. Also reuse `stripComments` in the new gate rather than
   inlining it.

9. **Strengthen the `global.css` comment, gate failure message, and PR
   rationale** (addresses: thin comment; cryptic gate failure; PR rationale).
   Expand the `global.css` comment to the self-introducing style; give the gate
   assertion a teaching failure message; have the PR rationale explain the
   occurrence-driven ladder and the `0`/`50%` deviations.

---
*Review generated by /review-plan*

## Per-Lens Results

### Correctness

**Summary**: Logically sound in its core mechanics — the literal-migration map
matches all 25 inventory sites, the proposed `BORDER_RADIUS_LITERAL_RE` correctly
matches `0`/`50%`/px while excluding `var()` and `-webkit-` prefixes, and the
phasing keeps the harness green. Substantive risks are concentrated in Phase 5's
EXCEPTIONS bookkeeping: one "decrement" row is actually pure-radius, and another
targets a `4px` radius literal that does not exist. The "drive counts off the
hygiene failure messages" procedure backstops most of these.

**Strengths**:
- Literal-migration map enumerates exactly the 25 inventory sites (independently
  verified).
- Proposed regex correctly flags `0`/`50%`/px while excluding `var(--radius-*)`;
  the `(?<![\w-])` lookbehind distinguishes `border-radius` from `--radius-*` defs
  and `-webkit-border-radius`.
- Routing `50%` to a new `--radius-full` storing `50%` preserves box-dependent
  computed values and keeps non-allow-listed files clean against the 0038 gate.
- Migration-first/enforcement-last leaves var-resolution and hygiene gates green at
  each committable phase.

**Findings**:
- 🟡 major (high) — FilterPill `:224` `3px` is pure-radius (all three sites are
  `border-radius`); "decrement" leaves `declared > observed` and fails hygiene.
  Reclassify as delete.
- 🔵 minor (high) — `:109` Sidebar `4px` conditional describes a literal that does
  not exist; scrollbar radii are already `var(--radius-sm)`, the `4px` occurrences
  are non-radius. State `:109` is non-radius, leave count 4.
- 🔵 minor (high) — AC5_FLOOR bump won't produce a failure to read; both ratchet
  assertions stay green while the floor is low. Reframe as a discipline step.
- 🔵 minor (medium) — `(?<![\w-])` lookbehind lets vendor-prefixed literals escape
  the gate; latent (none exist today) but a standing gap.
- 🔵 minor (medium) — Phase 3 never lists the `.codeblock pre` (`0` reset) selector,
  contradicting its own "every non-EmptyState selector has one assertion" checkbox.

### Test Coverage

**Summary**: Inherits a strong, well-understood test oracle (var-resolution,
hygiene, ratchet, pill allow-list) that genuinely drives the work red→green, and
the new gate + fixtures mirror the proven 0075 precedent. Coverage is
risk-proportional. But the Playwright spec — the only behavioural guard for
computed radii — has two structural soundness gaps (the `50%` values are pinned to
box-derived px with no tie to the token; several selectors depend on
unverified route/setup mounting), and the deliberate EmptyState exclusion leaves a
`12px` migration backstopped only by a literal-absence grep.

**Strengths**:
- The always-on harness is correctly identified as the oracle; var-resolution
  proves rename completeness, hygiene forces same-commit decrements.
- "Drive counts off hygiene failures, don't hand-compute" turns the harness into a
  self-correcting oracle for the trickiest part.
- Phase 7 gate + fixtures follow the established FONT_SIZE pattern; gate is
  explicitly verified to fail on an inserted `7px`.
- TDD baseline-capture ordering is sound.
- AC5_FLOOR bump and RADIUS_TOKENS lockstep edits are explicitly sequenced.

**Findings**:
- 🔴/major (high) — the three `50%` selectors assert a box-px string that cannot
  distinguish `--radius-full`(50%) from an equal px value, and is fragile to box
  change.
- 🟡 major (medium) — EmptyState `.card` (and all renamed sites) have no
  computed-value assertion; a wrong token value would pass AC3 + var-resolution.
  Add `RADIUS_TOKENS` value unit assertions.
- 🟡 major (medium) — RelatedArtifacts `.badge` and interaction-gated selectors
  mount in non-trivial ways the plan asserts but doesn't verify; add `waitFor()` and
  confirm `.badge` renders at baseline.
- 🔵 minor (medium) — gate fixtures omit a bare-`0` custom-property definition and
  logical-property longhands.
- 🔵 minor (high) — `:109` `4px` branch: hygiene and AC3 can disagree; ensure the
  Phase 7 gate runs over the same files so leftover radius is caught automatically.
- 🔵 minor (medium) — `50%`/`0` migrations have no per-phase hygiene driver;
  detection deferred to enforcement-last.

### Architecture

**Summary**: Structurally sound and well-aligned with the enforcement
architecture: treats the Vitest harness as the oracle, respects the four-touch
migration contract, follows 0075 phasing with a green suite at every commit. The
most consequential decision — the px-encoded ladder — trades domain-intent naming
for self-documenting value-coupled names; defensible and acknowledged but couples
identity to value. The supersession structure has one unaddressed schema collision
(ADR-0026's single-cardinality `superseded_by`).

**Strengths**:
- Correctly identifies the admission-ledger model and threads every gate.
- Migration-first/enforcement-last keeps every commit independently committable.
- ADR-0039 supersession is tightly scoped to the §3 radius row.
- `50%`→`--radius-full` is a correct boundary decision preserving the pill
  invariant.
- Epic-split contingency is bounded by a concrete threshold.

**Findings**:
- 🔴/major (high) — ADR-0026 already has single-cardinality
  `superseded_by: ADR-0036`; adding ADR-0039 collides with ADR-0034's single-ref
  rule. Rely on ADR-0039's `supersedes`.
- 🔵 minor (medium) — px-encoded names couple identity to value; a value retune
  forces a rename cascade. Note the value-mutation policy in ADR-0039 Consequences.
- 🔵 minor (medium) — in-test regex and the three AC3 sweeps are parallel
  definitions; assert the regex is the authoritative superset.
- 🔵 suggestion (low) — EmptyState AC3-only backstop acceptable given the exact
  value match; fold into the spec later if cheap.

### Code Quality

**Summary**: Unusually disciplined for a mechanical migration: leans on the
always-on harness, stays green throughout, and defers count computation to the
harness's own failure messages. The main risks are in the test-harness changes —
unvalidated EXCEPTIONS `reason` prose that will drift, a near-verbatim copy of the
comment-stripping logic instead of reusing the helper, and a gate regex whose
edge-case behaviour rests on a small fixture set.

**Strengths**:
- Migration-first/enforcement-last keeps diffs reviewable and bisectable.
- Resists hand-computing counts; drives off hygiene failures.
- Rename completeness machine-checked by the var-resolution test.
- Px-encoding removes the AC8 use-case-naming tension.
- `50%`→`--radius-full` avoids semantic overload of `--radius-pill`.

**Findings**:
- 🔵 minor (high) — new gate inlines comment-stripping instead of reusing
  `stripComments`.
- 🔵 minor (high) — EXCEPTIONS `reason` prose is unvalidated and will drift; make
  the AC5 backstop mechanical.
- 🔵 minor (medium) — gate regex silently ignores vendor-prefixed radius; document
  the lookbehind's purpose and the boundary.
- 🔵 suggestion (medium) — resolve the `:109` conditional before implementation.
- 🔵 suggestion (low) — pin the rename to the full `var(--radius-sm)` form.

### Documentation

**Summary**: Documentation-heavy and largely well-structured: pairs ADR-0039 with
the ADR-0026 §3 amendment, mirrors the ADR-0036 template, and enumerates authorised
deviations so rationale is traceable. The main gaps: the load-bearing px-encoded
rationale is asserted rather than argued, the proposed `global.css` comment is
thinner than the typography precedent, and the ADR-0026 single-value `superseded_by`
mechanic isn't addressed.

**Strengths**:
- Authorised Deviations section records decision provenance.
- ADR-0039 modelled section-by-section on ADR-0036.
- Reciprocal supersession intent documented and scoped to the radius row.
- Phase 7 gate comments explain *why*, not just *what*.

**Findings**:
- 🔴/major (high) — px-encoded rationale asserted, not argued, in the ADR that must
  justify the work-item deviation.
- 🟡 major (high) — ADR-0026 already carries single-value `superseded_by`; "add"
  won't produce valid reciprocal linkage. Convert/avoid per ADR-0034.
- 🔵 minor (medium) — `global.css` comment thinner than the `--size-*` precedent;
  omits the naming convention.
- 🔵 minor (medium) — PR description doesn't pin the px-omission/`0`/`50%`
  rationale.
- 🔵 minor (low) — reason-rewrite instruction lacks a target wording / consistency
  gate in Phases 4 & 6.

### Standards

**Summary**: Strongly aligned with conventions: faithfully mirrors 0075 (phasing,
ADR pairing, EXCEPTIONS retirement, Vitest-not-grep gate, `*-resolved-*` spec),
allocates ADR-0039 per the sequential rule, and documents AC deviations. The most
significant concern is the ADR-0026 reciprocal-linkage instruction colliding with
ADR-0034's single-ref cardinality; secondary concerns are an unlinked "supersedes
0033" claim and an inconsistent semantic-vs-scale vocabulary.

**Strengths**:
- Faithfully follows the 0075 precedent across phasing, ADR pairing, EXCEPTIONS
  retirement, and the Vitest describe-block gate.
- Correctly allocates ADR-0039; conventional filename/slug.
- Documents the AC1/AC8 deviation in a dedicated section.
- AC4 Vitest-block choice is a legitimate precedent-follow (no ripgrep step exists).
- Models ADR-0039 on ADR-0036; keeps work-item IDs in References.

**Findings**:
- 🔴/major (high) — `superseded_by: ADR-0039` conflicts with single-ref cardinality
  already occupied by ADR-0036.
- 🔵 minor (medium) — ADR-0039 claims to supersede 0033's t-shirt naming without a
  linkage; cites a bare work-item number.
- 🔵 minor (medium) — semantic-vs-scale token taxonomy described inconsistently;
  fix one canonical vocabulary.
- 🔵 minor (medium) — 0075's multi-line comment + ordering precedent not explicitly
  carried over.
- 🔵 minor (low) — confirm vendor-prefixed literals are intentionally out of scope.

### Usability

**Summary**: The px-encoded ladder is reasonably discoverable for picking a value
but introduces a fourth, conflicting naming model into a system that already uses
ordinal indices (`--sp-1`=4px) and t-shirt sizes. The same numeric-suffix shape now
means "px value" for radius but "ordinal step" for spacing — a direct least-surprise
collision. The mixed model (numeric measurements + semantic `--radius-pill`/`-full`)
is defensible, but the plan under-specifies contributor-facing guardrails and the
gate's failure experience.

**Strengths**:
- Self-documenting value lookup (type `var(--radius-6)` for measured 6px).
- The two shape-intent tokens being semantic is a coherent split.
- ADR-0039 gives a single discoverable place for the rule.
- Keeps `--radius-pill` named (999px is a mechanism, not an intent).

**Findings**:
- 🔴/major (high) — numeric-suffix radius collides with the ordinal-index meaning of
  `--sp-N`.
- 🟡 major (medium) — px-encoded names invite off-scale token invention; add a
  closed-set policy / allow-list assertion.
- 🔵 minor (medium) — gate failure gives no actionable guidance; add a teaching
  message.
- 🔵 minor (medium) — `--radius-0`/`--radius-full` straddle conventions; consider
  `--radius-none` and visual grouping.
- 🔵 minor (low) — whole-scale rename breaks existing `sm/md/lg` muscle memory with
  no aliasing period.

## Re-Review (Pass 2) — 2026-06-03

**Verdict:** APPROVE

Re-ran all 7 lenses against the revised plan. **20 of 25 prior findings resolved,
5 partially resolved (mitigated/acknowledged), none still-present.** The edits
introduced a handful of new findings — one major (flagged by two lenses) and
several minor — all of which were then addressed in a follow-up edit pass. After
those fixes the plan is sound and ready for implementation.

Note: the most consequential issue this cycle — that the plan directed an
in-place content edit to the `accepted` ADR-0026, violating ADR-0031 immutability
— was **not** caught by the original 7-lens review; it was raised by the author.
The plan was reworked to leave ADR-0026 untouched and record the supersession
solely on the new (proposed) ADR-0039 via `supersedes` (Authorised Deviation 6),
and a follow-up note was filed for the pre-existing 0075 in-place typography edit
(`meta/notes/2026-06-03-adr-0026-body-edited-in-place-breaks-immutability.md`).

### Previously Identified Issues

**Resolved (20):**
- 🟡 **Architecture/Documentation/Standards**: ADR-0026 `superseded_by` cardinality
  collision — Resolved (plan now writes no `superseded_by`; ADR-0039's list-valued
  `supersedes` carries the edge; ADR-0034 derivability cited).
- 🟡 **Correctness**: FilterPill `:224` pure-radius — Resolved (now "delete the row").
- 🟡 **Documentation**: px-encoded rationale asserted not argued — Resolved (ADR-0039
  Decision Drivers now argue it from the `3px`/`--radius-block` failures).
- 🟡 **Usability**: `--sp-N` ordinal collision — Resolved (named in ADR-0039, the
  `global.css` comment, and the PR description).
- 🟡 **Usability**: px-encoding invites off-scale tokens — Resolved (closed-set
  extension policy + gate teaching message + global.css note).
- 🔵 **Test Coverage**: route/setup mounting unverified — Resolved (explicit
  `waitFor()` per gated case + RelatedArtifacts `.badge` mount pre-check).
- 🔵 **Correctness**: `:109` Sidebar `4px` non-existent literal — Resolved (declared
  non-radius, count left at 4).
- 🔵 **Correctness**: AC5_FLOOR "read the failure" — Resolved (reframed as a
  discipline step; stale-low floor doesn't fail).
- 🔵 **Correctness/TestCov**: `.codeblock pre` (`0` reset) selector — Resolved
  (added to Phase 3 CASES).
- 🔵 **Code Quality**: stripComments duplication — Resolved (reuse + hoist).
- 🔵 **Code Quality**: EXCEPTIONS reason drift — Resolved (mechanical Phase 7 §4
  assertion).
- 🔵 **Test Coverage**: gate fixtures (bare-`0` def / logical longhands) — Resolved.
- 🔵 **Documentation**: global.css comment thin — Resolved (self-introducing comment).
- 🔵 **Documentation**: PR-description rationale unpinned — Resolved (four clauses
  enumerated).
- 🔵 **Documentation**: reason-rewrite no target wording — Resolved (worked example +
  assertion).
- 🔵 **Standards**: ADR-0039 "supersedes 0033" unlinked / bare work-item ID — Resolved
  (reworded to ADR-0026; IDs in References only).
- 🔵 **Standards**: vocabulary inconsistency — Resolved (one canonical two-bucket
  vocabulary used verbatim across surfaces).
- 🔵 **Standards**: global.css comment/ordering precedent — Resolved.
- 🔵 **Standards/CodeQuality/Correctness**: vendor-prefix scope — Resolved (documented
  + fixture).
- 🔵 **Usability**: gate failure message not actionable — Resolved (teaching message).

**Partially resolved / mitigated (5):**
- 🔵 **Architecture**: in-test regex vs AC3 sweeps parallel definitions — Partial
  (authority relationship now explicit; two artifacts still exist, the accepted
  0075-precedent design).
- 🔵 **Architecture/Documentation (suggestion)**: EmptyState `.card` AC3-only — Resolved
  via the existing `global.test.ts` parity guarding `--radius-12`'s value.
- 🔵 **Usability**: `--radius-0`/`--radius-full` straddle conventions — Resolved by the
  two-bucket vocabulary (author kept `--radius-0`).
- 🔵 **Usability**: whole-scale rename muscle memory — Partial (cost acknowledged in
  ADR-0039 Consequences; hard cut retained, low-value to alias for an internal set).
- 🔵 **Correctness**: vendor-prefix lookbehind — Partial→Resolved (fixture corrected to
  a prefixed *literal* that exercises the limitation).

### New Issues Introduced (all addressed)

- 🟡 **Test Coverage (major) / Correctness**: the Phase 2 §5 token-value assertion
  compared `tokens.ts` to a hardcoded table, so it could not catch a `global.css`
  drift (and was redundant with the existing `global.test.ts` parity suite).
  **Fixed**: Phase 2 §5 rescoped to confirm the existing `tokens.ts ↔ global.css`
  parity suite (which reads `global.css`) covers the new tokens; the redundant
  self-referential assertion is explicitly not added.
- 🔵 **Correctness**: vendor-prefix negative fixture used a `var()` value (proved
  nothing). **Fixed**: changed to a prefixed literal asserted not-matched.
- 🔵 **Correctness**: stale `AC5_FLOOR` baseline (426 vs actual 427). **Fixed** in
  both places.
- 🔵 **Code Quality**: gate comment conflated `*.global.css` with the root
  `global.css`. **Fixed**: comment reworded.
- 🔵 **Test Coverage**: reason-hygiene assertion matched only `border-radius`, not the
  bare `radius` phrasing the reasons use. **Fixed**: regex broadened to
  `/border-radius|\bradius\b/i`.
- 🔵 **Architecture/Documentation**: ADR-0026 now the target of two `supersedes` edges
  without attribution. **Fixed**: ADR-0039 scope clause now names both edges and the
  section each retires, and frames radius as a genuine replacement (vs ADR-0035's
  `relates_to` supplement).
- 🔵 **Standards**: ADR-0030-vs-ADR-0036 "template" ambiguity. **Fixed**: clarified
  ADR-0030 = structural template, ADR-0036 = worked exemplar.
- 🔵 **Usability (suggestion)**: sparse non-contiguous ladder reduces step
  discoverability. **Noted, not actioned** — mitigated by the gate teaching message
  and the ADR's "enumerates only consumed values" statement.
- 🔵 **Standards (suggestion)**: follow-up-note path referenced before creation —
  **moot**: the note was created this session and ships with the PR.

### Assessment

The plan is now in good shape and ready for implementation. The major
test-coverage regression introduced by the first fix pass has been corrected, the
ADR-immutability rework is internally consistent across all sections and the
follow-up note, and the remaining open items are low-value suggestions (vocabulary
duplication across surfaces, listing available steps in the gate message) that are
safe to leave to implementation discretion.

---
*Re-review generated by /review-plan*
