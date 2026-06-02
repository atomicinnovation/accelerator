---
date: "2026-06-02T17:00:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, correctness, test-coverage, standards, compatibility, documentation]
review_pass: 3
status: complete
---

## Plan Review: Inline Code Styling In Meta Artifact Markdown

**Verdict:** REVISE

This is an exemplary, tightly-scoped plan: its numeric claims (the `1px`
EXCEPTIONS bump 5→7, the `0.1rem` removal, the `3px`/`5px` additions), its
specificity arithmetic ((0,1,4) vs (0,1,3)), its token-rename inventory (8 CSS
consumers + declaration + comment + mirror), and its readings of ADR-0036 and
ADR-0026 were all independently verified against the source and found accurate.
The production change itself is sound and low-risk. The reason for REVISE is not
the design but the **test specification**: the dark-mode and colour-resolution
assertions — the only coverage for acceptance criterion 5 and half of AC2 — are
written as prose comments rather than executable `expect()` calls, so as drafted
those criteria would be exercised by a test body with no assertions. Two major
gaps compound this: a prose-code selector that silently depends on fixture
append-order (undermining the "phases in any order" claim), and an ADR-0036
tween example that references the about-to-be-renamed token but is absent from
the rename site list.

### Cross-Cutting Themes

- **Token-naming convention erosion** (flagged by: Architecture, Code Quality,
  Standards, Documentation) — Renaming `--size-eyebrow` → `--size-micro` adds a
  second consumer to a token the `global.css` convention comment classifies as
  "semantic single-purpose". All four lenses independently noted that the
  comment must be *coherently rephrased*, not find-replaced, so the remaining
  single-purpose tokens (`--size-row`/`--size-subtitle`) and the tween example
  still read consistently. Standards additionally questions whether `micro` is
  the right name given ADR-0036's stated preference for numeric-ladder names.

- **Prose-code selector vs. fixture ordering** (flagged by: Test Coverage,
  Correctness) — `[class*="markdown"] :not(pre) > code` with `.first()` also
  matches `td > code`/`th > code`, so the prose assertion resolves correctly
  only because the prose paragraph is appended before the table. This directly
  contradicts the plan's "phases can be implemented in any order" claim.

- **AC5 var() floor ratchet** (flagged by: Architecture, Test Coverage) — The
  edits are net +1 `var()` reference; the `AC5_FLOOR <= observed` ratchet stays
  green, but the plan's guardrail inventory and Migration Notes omit it.

- **`1px` EXCEPTIONS reason text** (flagged by: Code Quality, Documentation) —
  The plan says to "extend the reason" without specifying the resulting text;
  both lenses want the enumerated, declaration-order reason spelled out to match
  the precision of the new `3px`/`5px` entries.

### Findings

#### Critical

- 🔴 **Test Coverage**: Dark-mode and colour assertions are sketched as comments, not written as `expect()` calls
  **Location**: Phase 1, Section 2 (real-cascade computed styles)
  The Phase 1 spec writes real assertions only for font-family, font-size,
  border width/style/radius, and padding. The light-theme background/border-colour
  comparison and the *entire* dark-mode test body are prose comments with no
  executable assertion — so AC5 (the only theme-token-resolution criterion) and
  the colour half of AC2 would be unverified, and the dark test would pass
  trivially. Resolve expected light/dark `--ac-bg-sunken`/`--ac-stroke-soft` to
  concrete rgb via the throwaway-element pattern and assert them in both themes,
  importing the token tables (as `chip-resolved-colours.spec.ts` does) rather
  than hardcoding hex.

#### Major

- 🟡 **Test Coverage + Correctness**: "Phases in any order" is unsafe — the prose-code `.first()` selector also matches table-cell code
  **Location**: Implementation Approach; Phase 1 §2 / Phase 2 §3 fixture appends
  `[class*="markdown"] :not(pre) > code` matches `td > code` too, so `.first()`
  returns the prose span only because the prose paragraph is appended before the
  table. If Phase 2's table fixture lands first, `.first()` resolves to an 11px
  cell and the 11.5px prose assertions fail — a red test caused by fixture order,
  not a defect. Tighten the locator (`[class*="markdown"] p > code`, or
  `:not(td):not(th) > code`) or drop the "any order" claim and require
  prose-before-table document order.

- 🟡 **Documentation**: ADR-0036 references `--size-eyebrow` in a tween example but the plan only updates `global.css`
  **Location**: Phase 2, Section 4 (token rename)
  ADR-0036 (line 66) uses the identical tween illustration the plan fixes in the
  `global.css` comment ("`--size-xxs-sm` 11.5 between `--size-xxs` 12 and
  `--size-eyebrow` 11"), but ADR-0036 is not in the rename site list. After the
  rename it would reference a name that exists nowhere in the codebase. ADRs here
  are immutable — add a short follow-up/amending note recording that 0094 renamed
  the token, rather than editing the example in place.

#### Minor

- 🔵 **Architecture + Code Quality + Standards + Documentation**: Convention-comment reclassification must be a coherent rewrite, not a find-replace
  **Location**: Phase 2 §4 / Desired End State (global.css:160-167 comment)
  Pulling `--size-micro` out of the "semantic single-purpose" sentence into its
  own "shared micro-text size" clause keeps the remaining
  `--size-row`/`--size-subtitle` grouping accurate and the tween-bound example
  coherent.

- 🔵 **Standards**: `--size-micro` name sits awkwardly against ADR-0036's numeric-ladder naming preference
  **Location**: Phase 2, Section 4
  ADR-0036 praises ladder names (`--size-3xs-lg`, `--size-xxs-sm`) over semantic
  prefixes. Consider `--size-xxs-lg` (11px just below 11.5px) or add a one-line
  justification for `--size-micro`.

- 🔵 **Architecture**: Phase 2's internal steps are ordered (rename before the td rule), so "any order" is cross-phase only
  **Location**: Implementation Approach — phase independence claim
  The td rule consumes `var(--size-micro)`, which only exists after the rename;
  scope the independence claim accordingly.

- 🔵 **Architecture + Test Coverage**: AC5 aggregate var() ratchet is unacknowledged
  **Location**: Testing Strategy / Migration Notes
  Net +1 var reference; the `AC5_FLOOR <= observed` invariant stays green, but add
  a one-line note (no `AC5_FLOOR` bump required) so the ratchet isn't a silent
  surprise.

- 🔵 **Code Quality + Documentation**: Specify the exact concatenated `1px` EXCEPTIONS reason text
  **Location**: Phase 1, Section 5 (EXCEPTIONS ledger)
  Enumerate all contributing roles (existing hairlines + inline-code pill border
  + vertical padding) in declaration order, matching the `3px`/`5px` precision.

- 🔵 **Correctness**: Note that the EXCEPTIONS literal count is exact-token, not substring, matching
  **Location**: Phase 1, Section 5
  The regex tokenises `11px`/`11.5px` whole and filters by `h === literal`, so
  `1px` cannot be miscounted from multi-digit literals. Worth a one-line note so
  the counts are understood to be insulated.

- 🔵 **Correctness**: Fenced-block 14px assertion relies on unverified `.hljs`/highlight-layer font-size inheritance
  **Location**: Phase 3, Section 1
  `pre code` has no direct font-size rule; the 14px comes from `.markdown pre`
  inheritance. Confirm the syntax-highlight stylesheet leaves `pre code`/`.hljs`
  font-size unset so the assertion provably reflects inheritance.

- 🔵 **Code Quality**: Playwright `[class*="markdown"]` selector couples tests to the CSS-modules hashing scheme
  **Location**: Phases 1-3 (inline-code-resolved-styles.spec.ts)
  Acceptable if it matches the sibling specs' established convention (it does);
  otherwise prefer a stable `data-*` hook.

- 🔵 **Code Quality**: Co-locate or cross-reference the split inline-code rules
  **Location**: Phase 1 §4 vs Phase 2 §5
  The base rule (57-60) and the td override (after 61-63) are separated by the
  table-layout rules; add a comment on the td rule referencing the base rule it
  out-specifies.

- 🔵 **Test Coverage**: AC1's "differs from prose font-family" contrast is not asserted
  **Location**: Phase 1, Section 2 (AC1)
  Only `toContain('Fira Code')` is asserted. Add a sibling check that a prose `p`
  does *not* resolve to Fira Code, making the differentiation explicit.

- 🔵 **Test Coverage**: Reuse the shared `setTheme` helper instead of inlining the dataset toggle
  **Location**: Phase 1 & Phase 2 spec setup
  `setTheme(page, 'dark')` already exists in the visual-regression lib; reuse it
  rather than re-implementing the toggle + `waitForFunction`.

- 🔵 **Test Coverage**: CSS-as-text guards don't verify the old declarations were removed
  **Location**: Phase 1, Section 1
  Add a negative check (e.g. the inline rule no longer contains `var(--size-xs)`)
  and scope assertions to the rule body via the existing `extractBlockBody`
  helper; the real-cascade check remains the authoritative backstop.

- 🔵 **Compatibility**: Land the rename as a single atomic commit (all 11 sites)
  **Location**: Phase 2, Section 4 / Migration Notes
  A partial rename leaves consuming routes referencing an undeclared property
  (font-size fallback regression). The `var()`-resolves test gates this; make the
  lockstep/commit-boundary requirement explicit.

- 🔵 **Documentation**: Consider a follow-up note to ADR-0026 for the new admitted irreducible literals
  **Location**: Phase 1 §5 / Phase 2
  `3px`/`5px` are new admitted instances of ADR-0026 §3 categories and the
  reclassification of `--size-micro` is a convention change; ensure the rationale
  is recoverable beyond this single plan.

### Strengths

- ✅ Reuses the existing 11px token (renamed) rather than introducing a
  duplicate, correctly avoiding token-scale proliferation.
- ✅ Expresses the fix through existing `.markdown …` descendant selectors
  rather than adding a `code` component override, matching the renderer's actual
  convention (only `pre` is overridden).
- ✅ Routes all irreducible literals (`1px`/`3px`/`5px`) through the ADR-0026
  EXCEPTIONS ledger with reasons phrased in the established idiom; the ledger
  arithmetic was verified exact against the live file.
- ✅ Correctly identifies and tests the specificity relationship ((0,1,4) beats
  (0,1,3)) with a discriminating td=11px / th=11.5px test pair.
- ✅ The token-rename blast radius is enumerated exhaustively and verified, and
  leans on the existing `var()`-resolves-to-declared-token test as a mechanical
  completeness guard; the unrelated `[data-slot="eyebrow"]`/`EyebrowLabel`/
  `EmptyState` collisions are correctly disambiguated.
- ✅ Independently confirmed contract-safe: the drift fixture
  `prototype-tokens.json` captures only `--code-*`/`--tk-*`/`--atomic-*`, and the
  augmented fixture route (`/library/plans/first-plan`) has no screenshot
  baseline.
- ✅ Correctly diagnoses that var()/cascade can't resolve in jsdom and routes
  every computed-style assertion to real Chromium while keeping fast structural
  guards in the vitest CSS-as-text layer — a well-balanced pyramid.

### Recommended Changes

1. **Write out the dark-mode and colour assertions as executable `expect()`
   calls** (addresses: Critical — dark-mode/colour assertions as comments).
   Resolve expected light/dark `--ac-bg-sunken`/`--ac-stroke-soft` to rgb via the
   throwaway-element pattern and assert `backgroundColor`/`borderTopColor` in both
   themes, importing token tables rather than hardcoding hex. This is the gating
   change for the verdict.

2. **Make the prose-code locator order-independent** (addresses: Major —
   selector vs fixture ordering). Use `[class*="markdown"] p > code` or
   `:not(td):not(th) > code`, *or* drop the "phases in any order" claim and state
   that fixture appends must preserve prose-before-table order.

3. **Add ADR-0036 to the rename footprint** (addresses: Major — ADR-0036 tween
   reference). Since ADRs are immutable, add a follow-up/amending note recording
   the `--size-eyebrow` → `--size-micro` rename rather than editing the example.

4. **Rewrite the `global.css` convention comment coherently** (addresses: the
   token-naming-erosion theme). Separate `--size-micro` into its own shared-size
   clause; keep the single-purpose sentence and the tween-bound example accurate.

5. **Tighten the test spec details** (addresses: AC1 contrast, `setTheme` reuse,
   CSS-as-text removal checks). Assert prose ≠ Fira Code; reuse `setTheme`; add a
   negative `var(--size-xs)` check scoped to the rule body.

6. **Spell out the remaining ledger/ratchet bookkeeping** (addresses: `1px`
   reason text, AC5 var ratchet, exact-token counting note). Specify the full
   `1px` reason string, note the AC5_FLOOR ratchet stays green, and note the
   counts are exact-token matches.

7. **Make the rename atomicity and naming choice explicit** (addresses:
   Compatibility atomic commit, Standards `--size-micro` name). Require a single
   lockstep commit and either pick a ladder-consistent name or justify `micro`.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Tightly-scoped CSS-only change plus a global token rename that
respects established conventions well — works through existing descendant
selectors, reuses a token rather than duplicating, and routes literals through
the EXCEPTIONS ledger. The rename's fan-out is the only broad change but is
enumerated and mechanically guarded. Phase independence is mostly sound with one
intra-phase ordering coupling and minor consistency observations.

**Findings**:
- 🔵 minor / high — *Convention comment reclassification* (Phase 2 rename): the
  11px token moves from the semantic single-purpose category to a general one,
  blurring the comment's taxonomy; rewrite the comment coherently.
- 🔵 minor / medium — *Phase independence overstated* (Implementation Approach):
  Phase 2's td rule consumes `--size-micro`, which exists only after the rename,
  so the rename must precede it within Phase 2; "any order" is cross-phase only.
- 🔵 minor / medium — *AC5 var() ratchet unacknowledged* (Testing Strategy): the
  `AC5_FLOOR <= observed` invariant stays green but is omitted from the guardrail
  inventory; add a one-line note.

### Code Quality

**Summary**: Complexity proportional to the task; no new abstractions. The
rename blast radius is enumerated and mechanically guarded, and the
DOM-attribute collisions are correctly disambiguated. Concerns are around the
durability of the rename's semantic intent and small readability/coupling risks
in test selectors, the EXCEPTIONS reason aggregation, and rule placement.

**Findings**:
- 🔵 minor / high — *Convention comment coherence* (Phase 2 §4): pull
  `--size-micro` out of the single-purpose sentence rather than annotating in
  place.
- 🔵 minor / medium — *`[class*="markdown"]` selector coupling* (spec): couples
  tests to CSS-modules hashing; acceptable if it matches sibling specs, else use
  a `data-*` hook.
- 🔵 minor / medium — *`1px` reason aggregation* (Phase 1 §5): keep the reason
  enumeration explicit and in declaration order so the count stays auditable.
- 🔵 minor / medium — *Split inline-code rules* (Phase 1 §4 vs Phase 2 §5): add a
  cross-reference comment on the td rule pointing to the base rule it
  out-specifies.

### Correctness

**Summary**: Numeric and logical claims are overwhelmingly accurate — the `1px`
bump 5→7, the `0.1rem` removal, the `3px`/`5px` additions, the specificity
arithmetic, and the 8-site rename all verified against source. Residual risks are
test-fragility/ordering, not arithmetic: the prose `.first()` selector depends on
fixture append-order, and a couple of cascade/regex assumptions warrant explicit
confirmation.

**Findings**:
- 🔵 minor / high — *Prose `.first()` depends on append order* (Phase 1/2/3
  fixtures): the selector also matches `td`/`th` code; make it order-independent.
  (Merged with Test Coverage's major.)
- 🔵 minor / medium — *EXCEPTIONS regex counting* (Phase 1 §5): note the counts
  are exact-token matches, insulated from multi-digit literals like `11px`.
- 🔵 minor / medium — *Fenced-block inheritance* (Phase 3 §2): the 14px assertion
  relies on `.markdown pre` inheritance; confirm the highlight layer leaves
  `pre code`/`.hljs` font-size unset.

### Test Coverage

**Summary**: Unusually rigorous for a CSS change — all six ACs mapped to named
tests, fast CSS-as-text guards split from real-cascade Playwright checks, correct
jsdom/Chromium reasoning. But the two most safety-critical assertions (AC5
dark-mode, AC2 colours) are prose comments rather than `expect()` calls, and the
"any order" claim is unsafe given the prose-code `.first()` selector also matches
table-cell code.

**Findings**:
- 🔴 critical / high — *Dark-mode/colour assertions are comments, not `expect()`*
  (Phase 1 §2): AC5 and the colour half of AC2 would be unverified; write the
  assertions out using the throwaway-element token-resolution pattern.
- 🟡 major / high — *"Any order" unsafe; prose `.first()` matches td code*
  (Implementation Approach / fixtures): tighten the locator or require
  prose-before-table order.
- 🔵 minor / high — *AC1 contrast not asserted* (Phase 1 §2): assert prose `p` ≠
  Fira Code, not just `code` = Fira Code.
- 🔵 minor / medium — *Inlined theme toggle* (spec setup): reuse the shared
  `setTheme` helper.
- 🔵 minor / medium — *AC5_FLOOR bump protocol skipped* (Phase 1 §5 / Migration
  Notes): note the floor stays satisfied (+1 var, non-decreasing).
- 🔵 minor / medium — *Substring guards don't verify removal* (Phase 1 §1): add a
  negative `var(--size-xs)` check scoped to the rule body via `extractBlockBody`.

### Standards

**Summary**: Rigorous, accurate engagement with documented conventions — ADR-0036
(categorical font-size ban) and ADR-0026 (§3 irreducible categories) readings are
correct, and the ledger arithmetic matches the file and the reverse-hygiene test.
Descendant selectors correctly follow the renderer convention. Main concern: the
`--size-micro` name sits awkwardly against ADR-0036's numeric-ladder preference.

**Findings**:
- 🔵 minor / medium — *`--size-micro` vs ladder naming* (Phase 2 §4): consider
  `--size-xxs-lg`, or justify the semantic prefix against ADR-0036.
- 🔵 minor / medium — *Single-purpose clause coherence* (Phase 2 §4 / Desired End
  State): rephrase the comment so the remaining single-purpose tokens and the
  tween example stay accurate.

### Compatibility

**Summary**: Sound. The rename touches a purely internal CSS custom property
consumed only at the enumerated sites, and is verifiably NOT pinned by any
external contract — `prototype-tokens.json` captures only
`--code-*`/`--tk-*`/`--atomic-*`, and the augmented fixture route has no
screenshot baseline. The only residual concern is rename atomicity, an internal
consistency risk already guarded by an existing vitest check.

**Findings**:
- 🔵 minor / high — *Atomic rename commit* (Phase 2 §4 / Migration Notes): land
  all 11 sites together; a partial rename causes font-size fallback regressions.
- 🔵 minor / medium — *Prototype drift / convention comment* (Phase 2): confirmed
  contract-safe; keep the convention-comment edit truthful about the shared
  classification.

### Documentation

**Summary**: Thorough documentation hygiene — the plan updates both
`--size-eyebrow` references in the `global.css` comment and reclassifies the
token. The principal gap is ADR-0036's identical tween example, which will drift
to a stale name. Neither ADR records the rename or the new EXCEPTIONS entries.

**Findings**:
- 🟡 major / high — *ADR-0036 tween example not updated* (Phase 2 §4): add an
  immutable-safe follow-up note recording the rename.
- 🔵 minor / medium — *No ADR follow-up for new EXCEPTIONS / reclassification*
  (Phase 1 §5 / Phase 2): ensure the rationale is recoverable beyond the plan.
- 🔵 minor / medium — *Vague `1px` reason* (Phase 1 §5): specify the exact
  concatenated reason text matching the `3px`/`5px` precision.
- 🔵 minor / low — *Tween-example coherence* (Phase 2 §4): phrase the bound by
  value (e.g. "the 11px tier") so it doesn't depend on `--size-micro`'s
  classification.

## Re-Review (Pass 2) — 2026-06-02

**Verdict:** COMMENT

All seven lenses were re-run against the revised plan. The critical and both
major findings are resolved, and no new critical or major issues were
introduced — so the plan is now acceptable for implementation. The dominant
remaining theme is a consequence of the inter-pass decision to name the renamed
token `--size-xxs-xs`: three lenses independently flagged that the `-xs` suffix
overloads the existing integer-tier `-xs` (`--size-xs` = 14px) and that `-sm`
(a 0.5px tween) vs `-xs` (a full 1px step) under the same `xxs` root gives no
ordering cue. This is a real but minor convention wrinkle, not a blocker — the
value (11px) is unchanged and all guard tests stay green; it is a naming-clarity
judgment the author has already weighed. Everything else is minor/suggestion
polish on an otherwise sound, well-tested plan.

### Previously Identified Issues

- 🔴 **Test Coverage**: Dark-mode/colour assertions as comments — **Resolved.**
  Phase 1 §2 now has executable `expect(s.backgroundColor).toBe(await
  resolveToken(page, '--ac-bg-sunken'))` / `borderTopColor` in a parametrized
  light/dark loop with `setTheme`. Correctness independently verified the
  throwaway-element comparison is serialisation-safe for the rgba
  `--ac-stroke-soft` token.
- 🟡 **Test Coverage + Correctness**: "Any order" / prose `.first()` matched
  table cells — **Resolved.** Selector is now `[class*="markdown"] p > code`
  (direct child), which a `<td><code>` can never satisfy; the "any order" claim
  is backed by explicit append-only + scoped-locator invariants.
- 🟡 **Documentation**: ADR-0036 dangling token reference — **Resolved (as a
  documented decision).** Migration Notes records "leave ADRs untouched" as an
  accepted cost, correctly grounded in the repo's ADR-immutability convention
  (corroborated against ADR-0031).
- 🔵 **Architecture/Code-Quality/Standards/Documentation**: Convention-comment
  taxonomy erosion — **Partially resolved / re-shaped.** The ladder rename + 3-
  step coherent rewrite is a clear improvement, but the *specific* name
  `--size-xxs-xs` re-opened the concern in a new form (suffix overloading; see
  New Issues).
- 🔵 **Architecture + Test Coverage**: AC5 var() ratchet — **Resolved.** Net +1
  reference and the 426→427 `AC5_FLOOR` bump are now explicit; Correctness
  confirmed the bump is protocol (non-blocking), not a failing test.
- 🔵 **Code Quality + Documentation**: `1px` reason text — **Resolved.** Now
  fully enumerated in declaration order.
- 🔵 **Correctness**: exact-token counting / fenced `.hljs` inheritance —
  **Resolved** (notes added).
- 🔵 **Test Coverage**: AC1 contrast, `setTheme` reuse, removal guard —
  **Resolved.** AC1 prose≠mono assertion, `setTheme` import, and the
  `extractBlockBody`-scoped negative `var(--size-xs)` guard are all present.
- 🔵 **Code Quality**: `[class*="markdown"]` coupling — **Addressed**, though the
  "established across sibling specs" justification is slightly overstated (the
  pattern appears in one precedent spec). See New Issues.

### New Issues Introduced

- 🔵 minor / high — **Standards**: `-xs` suffix on an integer (11px) value
  conflicts with the documented "suffix = sub-pixel tween" convention, and `-sm`
  (11.5) vs `-xs` (11) under the same `xxs` root carries no step-magnitude cue.
  Suggestion: give 11px a tier-style name, or have the rewritten comment define
  `-xs` precisely as a full integer sub-tier distinct from the `-sm`/`-lg`
  tweens. (Echoed by Architecture and Documentation as suggestions.)
- 🔵 minor / medium — **Test Coverage**: the per-theme colour assertions never
  cross-check light ≠ dark, so a token that accidentally became theme-invariant
  would pass both branches trivially. Add one `expect(darkBg).not.toBe(lightBg)`
  (or assert the literal dark `rgb(7, 11, 18)` / `rgba(255,255,255,0.04)`).
- 🔵 minor / medium — **Correctness**: the AC1 `proseFont).not.toContain('Fira
  Code')` assertion silently assumes the default (non-`[data-font="mono"]`)
  mode; assert that precondition or rely on the positive code-is-mono check.
- 🔵 minor / medium — **Test Coverage / Correctness**: the `th code` 11.5px and
  fenced `pre code` 14px guards rest on inheritance and on the GFM `<thead>`
  delimiter-row; add a comment/anchor so a future fixture or highlight-layer
  change can't silently void them.
- 🔵 suggestion / high — **Code Quality**: `resolveToken` is a third inline copy
  of the throwaway-element resolution pattern (also in `chip-resolved-colours`
  and `root-resolved-tokens`); consider extracting it into
  `lib/expected-colours.ts` beside `setTheme`.
- 🔵 suggestion — **Documentation**: convention-comment line citations are ~157-
  168 (not 160-167); consider an in-codebase breadcrumb (e.g. on the token
  declaration) reconnecting the stale ADR-0036 example to the new name.

### Assessment

The plan is in good shape and ready to implement. The headline correctness and
test-coverage risks from pass 1 are genuinely closed (verified against source,
not just asserted), the rename is mechanically guarded and contract-safe, and
the new spec code is clean. The only judgment call left open is the
`--size-xxs-xs` token name: it is functionally correct and contract-safe, but
three lenses agree the `-xs` suffix is a minor convention-clarity regression. If
the author is comfortable with that trade-off (and documents the suffix meaning
in the convention comment), no further revision is needed; otherwise a tier-
style name for the 11px value would fully satisfy the standards concern. The
remaining items are optional polish.

### Resolution (post-pass-2)

The author chose to resolve the naming concern at the root rather than locally:

- **0094 slimmed** — the token rename was dropped entirely. The Phase 2 td rule
  now consumes the existing `--size-eyebrow` (11px) by its current name, so
  Phase 2's 8-site rename, the convention-comment rewrite, and all naming churn
  are removed; 0094 is once again a pure CSS fix (the AC5 +1 var bump now stems
  from the td rule's `var(--size-eyebrow)`). All four selected polish items were
  folded in: light≠dark colour divergence cross-check, `th`/fenced guards
  anchored on intent/second properties, `resolveToken` extracted to
  `lib/expected-colours.ts`, and the AC1 prose-contrast assertion now guards the
  default-font-mode precondition.
- **Scale remap spun out** — the systemic naming inconsistency (three schemes
  across the sub-14px band, ~100 consumers) is captured as **work item 0099**
  (`meta/work/0099-remap-typography-size-scale-to-pure-numeric-tokens.md`),
  which will adopt a pure-numeric scheme (`--size-110` = 11px) and supersede
  ADR-0036 via its own ADR + plan. 0094 is decoupled and needs no rework when it
  lands.

This fully addresses the standards/architecture/documentation naming findings
(by removing the contested rename from 0094 and deferring naming to a coherent,
dedicated initiative) and the residual test-coverage/code-quality minors.

## Re-Review (Pass 3) — 2026-06-02

**Verdict:** APPROVE

A confirming pass was run against the slimmed plan across six lenses
(architecture, code-quality, correctness, test-coverage, standards,
documentation). Outcome: **0 critical, 0 major.** **Correctness returned zero
findings.** The Documentation lens raised one "major" (dangling review/research
cross-references) which was **verified to be a false positive** — both cited
files exist on disk and are jj-tracked
(`meta/reviews/plans/2026-06-02-0094-…-review-1.md`,
`meta/research/codebase/2026-06-02-0094-…md`); the agent's git-aware search
simply did not see them. Dismissing that, only minor/suggestion items remain.

### Residual minor/suggestion items (non-blocking)

- **Architecture** (minor): the `global.css` convention comment will misdescribe
  `--size-eyebrow` as "single-purpose" once the td rule adds a second consumer —
  *owned by the 0099 remap by design*.
- **Standards** (minor ×2): the second-consumer wart and consuming the
  `--size-xxs-sm` tween as a primary size — *both deferred to 0099*; suggestion
  that 0099's successor ADR carry forward ADR-0036's scale-extension policy.
- **Test Coverage** (minor ×2): the fenced `14px` assertion could derive from
  the token / assert the `pre`↔`pre code` inheritance relationship; add a
  loud-fail visibility check before the `th`/`td` `Promise.all`.
- **Code Quality** (minor ×2): the `resolveToken` extraction leaves the two
  pre-existing inline copies in place (partial DRY); the two-rule pill split
  could carry a back-reference comment on the base rule. Suggestion: a top-level
  `describe` grouping the spec.
- **Documentation** (minor ×2): the deferral rationale is now triplicated across
  Overview / Key Discoveries / Migration Notes (trim); label the References ADRs
  as "consulted, not modified".

### Verdict rationale

By the configured rubric this pass scores **COMMENT** (minor findings exist;
APPROVE is reserved for zero-finding plans). The author has exercised the
verdict override to **APPROVE**: the critical and both majors from earlier
passes are closed, Correctness is clean, the one new "major" was a false
positive, and every residual item is optional polish or explicitly owned by the
0099 follow-on. The plan is approved for implementation as-is; the listed minors
may be folded in opportunistically during implementation but are not blocking.
