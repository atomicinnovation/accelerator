---
date: "2026-05-23T18:30:00Z"
type: plan-review
producer: review-plan
target: "plan:2026-05-23-0075-typography-size-scale-consumption"
review_number: 1
verdict: APPROVE
lenses: [architecture, correctness, code-quality, test-coverage, standards, documentation, compatibility, safety]
review_pass: 5
status: complete
id: "2026-05-23-0075-typography-size-scale-consumption-review-1"
title: "2026-05-23-0075-typography-size-scale-consumption-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-23T18:30:00Z"
last_updated_by: Toby Clemson
---

## Plan Review: 0075 Typography Size-Scale Consumption Reconciliation

**Verdict:** REVISE

The plan is unusually well-organised — TDD-first ordering, explicit per-phase decrement-vs-delete reasoning, layered enforcement (vitest category-level ban + AC2 ripgrep sweeps + Playwright computed-style spec), and clear cross-references to ADR-0026. However, sixteen major findings across all eight lenses converge on a small number of structural issues that should be resolved before implementation kicks off: regex semantics in the new vitest enforcement diverge between layers and have leading-dot / global.css coverage gaps; the Playwright spec defers unbounded route/selector discovery work to Phase 1; the EXCEPTIONS decrement bookkeeping is error-prone and one count appears arithmetically inconsistent; in-place ADR amendment contradicts ADR-0031's documented immutability convention; and the deliberate-drift case understates accessibility/visual magnitude for inline code inside headings. None of these are critical, but together they justify a revision pass.

### Cross-Cutting Themes

- **Regex divergence + coverage gaps** (flagged by: correctness, code-quality, standards, test-coverage, compatibility, safety) — The Phase 1.4 vitest regex pair and the AC2 ripgrep sweeps use subtly different patterns (`\b` vs `\s` boundary); leading-dot decimals (`.5px`) may slip the test; global.css is not covered because `cssGlobals` globs `*.global.css` only; the shorthand regex can false-positive on unit-bearing line-heights. Multiple lenses recommend aligning the test and AC2 regex verbatim, tightening the boundary, and either widening `allCss` to include `global.css` or correcting the Phase 1.4 coverage commentary.
- **tokens.ts insertion order contradicts convention** (flagged by: correctness, code-quality, standards) — Phase 1.2 instructs "alphabetical-by-key" but the existing `TYPOGRAPHY_TOKENS` object is px-descending, matching `global.css`. Inserting alphabetically would create two divergent orderings of the same set.
- **Playwright spec placeholders + scope** (flagged by: code-quality, test-coverage, safety) — Phase 1.5 ships 10+ unresolved route/selector placeholders. Several components (ActivityFeed, EmptyState, interactive menus inside SortPill/FilterPill) may have no deterministic mounting route without scaffolding work the plan does not budget. The spec is also brittle to legitimate future token-value tweaks.
- **EXCEPTIONS decrement bookkeeping** (flagged by: correctness, code-quality, documentation, safety) — Six decrement operations are specified, but explicit `rg` cross-check is given for only one (Phase 6.3). FilterPill `10px` count math appears inconsistent: the existing reason names three padding uses, but Phase 6.2 also migrates a 10px font-size on `.badge` — either the EXCEPTIONS reason is misleading or the current count is wrong. The plan also trusts existing EXCEPTIONS counts without a Phase 1 baseline verification.
- **Work item ID 0075 baked into durable artefacts** (flagged by: architecture, standards, documentation) — ADR-0026 is amended in-place (which contradicts ADR-0031's documented immutability convention for `accepted` ADRs), and the canonical comment in `global.css` plus the §2/§3 amendment text reference "0075" inline. These references couple long-lived artefacts to ephemeral work-item context.
- **Inline-code drift + rem→px accessibility regression** (flagged by: compatibility, test-coverage) — The plan describes `0.88em` → `14px` for inline `<code>` as a single "drift" but in headings it's a ~10px reduction (H1: ~24.6 → 14) and ~5px reduction (H2: ~19.4 → 14). The H1 `1.75rem` → fixed-px migration also silently removes user-controllable root font-size scaling for that heading. Neither change is covered by automated regression beyond a single representative selector.
- **TDD red-suite duration without dev workflow** (flagged by: test-coverage, safety) — The new vitest assertion stays red between Phase 1 and Phase 8. There's no documented mechanism to distinguish expected red from new regressions across the multi-phase window.
- **Token naming convention split** (flagged by: architecture, documentation) — Five new tokens mix numeric (`--size-4xs/3xs`) and semantic (`--size-eyebrow/row/subtitle`) conventions; chip tokens are reused outside chip components without rename. Rationale lives in the work item only — no in-code signal for future readers.

### Tradeoff Analysis

- **Single-PR atomicity vs. blast-radius safety** — The plan firmly commits to one PR. The work item's Drafting Notes name an epic-split contingency, but it is not reflected in the plan and there is no documented rollback path for a subtle post-merge regression. The safety lens recommends adding the contingency trigger and rollback strategy in-plan; the plan's atomicity stance is otherwise sound for review coherence.
- **In-place ADR amendment vs. ADR-0031 immutability** — Standards lens flags the amendment-in-place as contradicting ADR-0031's accepted-immutability rule; architecture lens flags the loss of the decision-trail. The plan should either acknowledge the deliberate bypass (with rationale) or supersede ADR-0026 with a new ADR for the typography rule, leaving the spacing band intact.
- **Categorical ban vs. emergency escape hatch** — Architecture/safety lenses both note the new vitest ban deliberately bypasses EXCEPTIONS. This is correct for a rule-level enforcement, but the plan should document a structured escape valve (`FONT_SIZE_LITERAL_EXCEPTIONS` array with ADR-backed entries) so future emergencies don't lead to commenting out the test.
- **Fixed-px tokens vs. rem-based accessibility scaling** — Compatibility lens flags that anchoring `--size-*` to px (and migrating `1.75rem` → fixed px) removes user-controllable root-font-size scaling. The plan should either acknowledge this as a deliberate project-wide stance in ADR-0026 or keep critical rem-based tokens (e.g. headings) as rem inside the token definition.

### Findings

#### Major

- 🟡 **Architecture**: Mixed numeric/semantic naming fragments the scale's vocabulary
  **Location**: Phase 1.1 (Extend the `--size-*` scale) and work item Decisions
  The extended scale interleaves numeric-tier (`--size-4xs/3xs`) with semantic-purpose tokens (`--size-eyebrow/row/subtitle`) inside one px-ordered block. The "single-purpose consumer" justification will not survive the first 11px consumer that isn't an eyebrow.

- 🟡 **Architecture**: Reusing `--size-chip` / `--size-chip-md` outside chip components encodes a semantic lie
  **Location**: What We're NOT Doing (line 104-106)
  The token name still says "chip" but consumers include Sidebar headings, FilterPill menu headers, LibraryTypeView rows, EmptyState eyebrows, etc. Defers a rename that gets more expensive with each new consumer. Consider renaming to numeric tiers in this PR.

- 🟡 **Correctness**: `FONT_*_RE` does not actually cover `global.css`
  **Location**: Phase 1.4 — vitest enforcement coverage claim
  `cssGlobals = import.meta.glob('../**/*.global.css', …)` picks up `*.global.css` files but NOT `styles/global.css` itself. The Phase 1.4 commentary that the test covers globals is incorrect. Either extend `cssGlobals` to import `../styles/global.css?raw` or correct the Desired End State claim.

- 🟡 **Code Quality**: `FONT_SHORTHAND_RE` has word-boundary edge cases and noisy failure output
  **Location**: Phase 1.4
  `\b[.0-9]+` may miss leading-dot decimals like `.5px`; `[.0-9]+` can match a lone `.`; the matched substring shown on failure is the whole shorthand suffix, not just the offending literal. Tighten the regex and consider a two-step extract.

- 🟡 **Code Quality / Standards**: `tokens.ts` insertion order contradicts file convention
  **Location**: Phase 1.2
  Plan says "alphabetical-by-key", but existing `TYPOGRAPHY_TOKENS` is px-descending (mirroring global.css). Replace with "slotted by px ordering, mirroring the Phase 1.1 CSS block" and enumerate insertion positions.

- 🟡 **Code Quality**: FilterPill `10px` decrement math appears inconsistent
  **Location**: Phase 6.3
  Current entry says `count: 3, reason: 'trigger + badge + menu-header horizontal padding from design'` (3 paddings). Phase 6.2 also migrates `.badge { font-size: 10px }`. Either the existing reason mislabels the badge font-size as padding, or the actual file has 4 occurrences and the existing hygiene test is failing. Verify before merging the plan.

- 🟡 **Code Quality / Test Coverage**: Playwright placeholders defer unbounded discovery to implementation
  **Location**: Phase 1.5
  12 cases contain `<route-…>` and `<scoped-…>` placeholders. Several components likely require interaction (open menu) or fixture state (markdown content, ActivityFeed SSE) to render their target selectors. Resolve concretely (route + selector + render-precondition) before Phase 1 closes.

- 🟡 **Test Coverage**: TDD red-suite stays red across most of the migration without workflow guidance
  **Location**: Phase 1 + Phases 2–7
  AC2/0075 describe block fails Phase 1 by design and clears phase-by-phase. No mechanism to distinguish expected red from new regressions. Either reorder (test last), gate behind a flag, or specify per-phase allow-lists.

- 🟡 **Test Coverage**: No automated regression coverage for the documented deliberate-drift case
  **Location**: Testing Strategy + Phase 2
  Inline `<code>` inside headings is the principal behaviour change; AC7 tests inline code in body context only (where the value is unchanged). Add a Playwright case asserting computed font-size of inline `<code>` *inside* an H2.

- 🟡 **Standards**: In-place ADR amendment contradicts ADR-0031 immutability
  **Location**: Phase 1.3 / Migration Notes
  ADR-0031 explicitly forbids edits on non-`proposed` ADRs. The plan amends `accepted` ADR-0026 in-place. Either acknowledge the deliberate bypass with rationale, supersede ADR-0026 with a new ADR, or extend ADR-0031 first to permit narrowly-scoped amendments.

- 🟡 **Documentation**: Proposed ADR §2 text states the rule but omits rationale
  **Location**: Phase 1.3
  The replacement paragraph removes the worked-example table from §2 without substitute reasoning. Add one sentence explaining *why* the tolerance band was abandoned (the scale was widened to remove genuinely off-grid values).

- 🟡 **Documentation**: Canonical comment + ADR text references "0075" by ID, coupling code to ephemeral context
  **Location**: Phase 1.1 / AC6a + Phase 1.3 ADR §2/§3
  Long-term readers will see a number without context. AC6a's wording is locked, so flag this in plan/Migration Notes; consider whether the work-item attribution can live solely in the References block.

- 🟡 **Documentation**: Decrementing EXCEPTIONS counts erodes per-occurrence justification
  **Location**: Phases 4.3 / 6.3 / 7.1-7.3
  Rewritten reasons preserve count but lose which uses remain justified. A future contributor adding another `22px` literal cannot tell whether the existing entry already covers their case. Phase 8.1 audit should require each remaining reason to enumerate concrete uses (e.g. `card padding-block at .card`).

- 🟡 **Compatibility**: Inline `<code>` drift magnitude understated for in-heading occurrences
  **Location**: Phase 2 / Decisions
  Inside H1 (28px), inline code drops from ~24.6 → 14px (~10px). Inside H2 (22px), ~19.4 → 14px (~5px). The plan should enumerate per-heading-level deltas and capture before/after screenshots in both H1 and H2 (not just H2).

- 🟡 **Compatibility**: H1 `1.75rem` → fixed `28px` removes user-controllable root-font-size scaling
  **Location**: Phase 2.1
  Migrating from rem to fixed-px tokens silently removes accessibility scaling for users with custom browser font-size preferences. Either acknowledge this in the ADR amendment as a deliberate project-wide stance, or consider keeping critical heading tokens as rem internally.

- 🟡 **Safety**: Contingency epic-split path from work item not reflected in plan
  **Location**: Implementation Approach / Phase 8
  Work item names an epic-split fallback for unreviewable PRs, but the plan provides no trigger criterion or pre-split state. Add a "Contingency: epic-split" subsection naming the trigger and first child-story scope.

- 🟡 **Safety**: Cross-check grep specified for only 1 of 6 decrement operations
  **Location**: Phases 4, 6, 7
  Phase 6.3 alone asks for an `rg` count cross-check; the other five decrements eyeball against existing reason strings. Add explicit `rg -n '<literal>' <file>` steps to each decrement operation with expected post-migration count.

- 🟡 **Safety**: Plan trusts existing EXCEPTIONS counts without baseline verification
  **Location**: Current State Analysis
  If an existing `count` is stale (pre-migration miscount), Phase N decrements produce a red hygiene test with no diagnostic guidance. Add a Phase 1 step: run vitest on `main` first and confirm `EXCEPTIONS hygiene` passes.

- 🟡 **Safety**: No automated CI hook for the AC2 ripgrep sweeps
  **Location**: Desired End State / Phase 8.2
  The three sweeps are manual; only the vitest test enforces continuously. Either wire the sweeps into CI as a fail-on-match check, or annotate the vitest test as the authoritative AC2 implementation so future regex edits stay aligned.

#### Minor

- 🔵 **Architecture**: Retaining unused heading tokens as "defensive scaffolding" weakens scale discoverability — either remove them or add a "reserved" comment per token.
- 🔵 **Architecture**: Category-level ban regex couples to CSS source syntax, not parsed semantics — won't catch indirection through `--my-size: 12px; font-size: var(--my-size)`. Document the scope limitation.
- 🔵 **Architecture**: ADR amendment-in-place loses rationale trail — add a dated "Amendments" subsection or supersede instead.
- 🔵 **Correctness / Code Quality**: Existing `--size-*` block is not strictly descending (chip 10.5 followed by chip-md 11.5) — the proposed reorder splits the chip pair; either keep the pair grouped or call out the intentional reorder.
- 🔵 **Correctness**: `FONT_SHORTHAND_RE` would flag a unit-bearing line-height inside `font:` (`font: 400 var(--size-xxs)/1.5rem …`) even when font-size already uses a token — no current site, but worth tightening or documenting.
- 🔵 **Code Quality**: Two `it()` per CSS file doubles test surface — consider one `it` per file asserting both regex match-arrays are empty.
- 🔵 **Code Quality**: EXCEPTIONS edits across 7 phases create review-coupling — consider a Phase 8 sub-step that re-derives all `reason` fields from observed state.
- 🔵 **Test Coverage**: Per-selector Playwright coverage is thin (12 selectors vs ~37 sites) — add at least one selector per new token.
- 🔵 **Test Coverage / Standards**: Vitest regex differs from AC2 third sweep (`\b` vs `\s`) — align verbatim.
- 🔵 **Test Coverage**: Playwright spec doesn't pin theme — sufficient because type scale is theme-invariant, but document the omission.
- 🔵 **Test Coverage**: Plan doesn't verify spec is wired into the visual-regression project run — add `npx playwright test --list --project=visual-regression | grep typography-resolved-sizes`.
- 🔵 **Standards**: Prose-only §2 replacement breaks tabular pattern used by Spacing subsection — keep the structural shape or note the asymmetry.
- 🔵 **Standards**: Canonical comment style diverges from established short-heading convention in global.css — flag in plan; consider revising AC6a wording.
- 🔵 **Standards**: ADR References "amends" verb introduces new lifecycle vocabulary — use established phrasing ("source work item") or transition to supersession.
- 🔵 **Documentation**: Five new tokens have no inline rationale; mixed naming convention undocumented in code — add a section comment explaining numeric vs semantic split.
- 🔵 **Documentation**: ADR References entry assumes work item path stability — strip inline "(Rule introduced by 0075)" from amended text.
- 🔵 **Documentation**: Phase 8.3 PR description "Phases" subsection underspecified — either drop or specify minimum content.
- 🔵 **Documentation**: ADR §3 footer line uses "0075" as sole anchor — tighten to "see §2's Typography rule".
- 🔵 **Compatibility**: `font:` shorthand expansion does not reset `font-style/variant/stretch` as the shorthand did — add explicit `normal` resets or document why omitted.
- 🔵 **Compatibility**: Phase 7.2 EXCEPTIONS reason says "card title" but selector is `.cardLabel` — fix during the same audit pass.
- 🔵 **Compatibility**: Font-shorthand regex requires whitespace before numeric size — leading-dot decimals slip through; add edge-case fixtures.
- 🔵 **Safety**: No documented procedure for legitimate emergency font-size literal — codify a `FONT_SIZE_LITERAL_EXCEPTIONS` escape valve in ADR-0026 or beside the test block.
- 🔵 **Safety**: Playwright spec asserts hardcoded px values brittle to token re-tuning — either re-derive expected values from `TYPOGRAPHY_TOKENS` or document the maintenance contract.
- 🔵 **Safety**: Single-PR atomicity has no documented rollback path for post-merge visual regression — add a "Rollback strategy" subsection.

#### Suggestions

- 🔵 **Correctness**: Phase 4.2 line-height token mapping — anchor that `--lh-normal` resolves to `1.5` matching the literal being replaced.
- 🔵 **Code Quality**: Shorthand expansion produces verbose CSS without an in-file comment naming the convention — add a one-line comment.

### Strengths

- ✅ Category-level vitest ban replaces a per-occurrence EXCEPTIONS regime with a structural invariant — strengthens authoring/enforcement separation.
- ✅ Three-sweep AC2 verification (modules / non-global CSS / `font:` shorthand) provides layered safety against drift.
- ✅ TDD-first sequencing in Phase 1: tokens + ADR + (red) enforcement test land before consumer migration begins.
- ✅ Phase decomposition is internally independent (Phases 2–7 touch only their own cluster) with single-atomic-PR landing.
- ✅ Shorthand expansion in Sidebar/ActivityFeed closes the structural loophole where font-size literals could hide inside compound declarations.
- ✅ Per-EXCEPTIONS-entry decision is documented explicitly as "delete vs decrement" with reason-rewriting instructions.
- ✅ All EXCEPTIONS decrement counts that *were* spot-verified against live migration.test.ts and CSS sources match (EmptyState 12px/22px/14px, Sidebar 10px, LibraryTypeView 12px) — the FilterPill 10px case is the lone arithmetic concern.
- ✅ Playwright spec deliberately picks the two value-transition cases (`1.75rem` → `28px`, `0.88em` → `14px`) where regression risk is highest, and pins viewport for determinism.
- ✅ Phase 1.2 (`tokens.ts` registration) preempts the cross-cutting failure mode of new `var(--*)` references resolving against an unupdated declared-tokens set.
- ✅ Single source of truth sharpened: ADR-0026 + canonical comment + AC2 grep + vitest assertion form a redundant enforcement triangle.
- ✅ Each phase carries Automated and Manual Verification sections with concrete `rg` commands and expected counts.
- ✅ Phase 8 functions as a reconciliation gate (final grep proof + reason re-audit + PR description deliverable).

### Recommended Changes

Ordered by impact. Each entry references the findings it addresses.

1. **Align and tighten the Phase 1.4 regex pair** (addresses: `FONT_*_RE doesn't cover global.css`, `FONT_SHORTHAND_RE word-boundary`, `Vitest regex differs from AC2`, `Font-shorthand regex requires whitespace`, `Shorthand regex flags unit-bearing line-height`)
   - Make the vitest regex and the AC2 third sweep identical strings (mirrored in both Phase 1.4 and Phase 8.2).
   - Tighten the boundary to handle leading-dot decimals (`(?<![\w.-])(\d+(?:\.\d+)?|\.\d+)`).
   - Either widen `cssGlobals` to include `styles/global.css?raw` or correct the Phase 1.4 coverage commentary explicitly.
   - Add a unit-test fixture exercising edge cases (`.875rem`, leading-dot decimals, double whitespace, `font:` with unit-bearing line-height).
   - Surface only the literal on failure (two-step extract or named capture), not the whole shorthand suffix.

2. **Resolve Phase 1.5 Playwright placeholders before Phase 1 closes** (addresses: `Playwright placeholders defer unbounded discovery`, `Playwright placeholders unverified routing assumptions`, `Plan doesn't verify spec is wired into project run`)
   - Replace each `<route-…>` / `<scoped-…>` placeholder with a concrete route + selector + render precondition (e.g. `await page.click(...)` for menus).
   - For each case lacking a mounting route (likely ActivityFeed, EmptyState, possibly FilterPill/SortPill open-menu states), name the smallest action: add `data-testid`, reuse existing showcase route, or scaffold fixture state.
   - Add a Phase 1 verification: `npx playwright test --list --project=visual-regression | grep typography-resolved-sizes` should list all 12 case names.

3. **Fix Phase 1.2 tokens.ts insertion guidance + Phase 1.1 token-block ordering note** (addresses: `tokens.ts insertion order contradicts convention`, `Existing block not strictly descending`)
   - Replace "alphabetical-by-key" with "slotted by px ordering, mirroring the Phase 1.1 CSS block".
   - Enumerate the five insertion positions (e.g. `size-subtitle` after `size-xs`, `size-row` after `size-xxs`, etc.).
   - Add a one-line note in Phase 1.1 acknowledging the chip/chip-md reorder (10.5 → 11.5 inversion is being corrected) so a reviewer doesn't read it as accidental drift.

4. **Verify FilterPill `10px` count baseline and reconcile EXCEPTIONS** (addresses: `FilterPill 10px decrement math inconsistent`, `Plan trusts existing EXCEPTIONS counts`)
   - Before any phase 6 work, run `rg -n '10px' src/components/FilterPill/FilterPill.module.css` and reconcile against the EXCEPTIONS entry.
   - Add a Phase 1 baseline step: run vitest on `main` and confirm `EXCEPTIONS hygiene` passes for every literal touched by Phases 4/6/7.
   - Add explicit `rg -n '<literal>' <file>` cross-check + expected post-migration count to each decrement operation in Phases 4.3, 7.1, 7.2, 7.3 (matching Phase 6.3's pattern).

5. **Address ADR-0031 immutability conflict** (addresses: `In-place ADR amendment contradicts ADR-0031`, `ADR amendment-in-place loses rationale trail`)
   - Decide: (a) acknowledge the deliberate bypass with rationale in plan + PR description; (b) supersede ADR-0026 with a new ADR scoped to the consume-tokens rule; or (c) extend ADR-0031 to permit narrowly-scoped amendments first.
   - If staying with (a), at minimum add a dated "Amendments" subsection to ADR-0026 naming work item 0075 and summarising what changed.

6. **Add accessibility/visual-magnitude documentation for the deliberate drift** (addresses: `Inline code drift magnitude understated`, `H1 rem→fixed-px removes user font-size scaling`, `No automated regression coverage for deliberate-drift case`)
   - In Phase 2 Overview and ADR-0026 Consequences amendment, enumerate per-heading-level computed deltas (H1: ~24.6 → 14; H2: ~19.4 → 14; H3: ~14.1 → 14).
   - Capture before/after screenshots in both H1 and H2 (not just H2).
   - Add a Playwright case asserting computed `font-size` of inline `<code>` *inside* an H2 equals `14px`.
   - Acknowledge in the ADR §2 amendment that fixed-px tokens are a deliberate project-wide stance (or revisit keeping critical heading tokens as rem internally).

7. **Add rationale to ADR §2 replacement text + remove inline 0075 references where possible** (addresses: `ADR §2 text omits rationale`, `Canonical comment + ADR text references 0075 by ID`, `ADR References "amends" vocabulary`, `ADR §3 footer line uses 0075 anchor`)
   - Add one sentence to §2 explaining why the tolerance band was abandoned for typography (scale widened to remove off-grid values).
   - Strip "(Rule introduced by work item 0075.)" from the §2 paragraph — let the new References entry carry the link.
   - Replace `see §2's Typography rule introduced by 0075` with `see §2's Typography rule`.
   - Use established References phrasing ("source work item") instead of "amends".

8. **Resolve Phase 1 → Phase 8 red-suite workflow** (addresses: `TDD red-suite stays red across migration`)
   - Pick one of: (i) reorder so the new test lands in Phase 8 (red-then-green pattern preserved at the file level by Phases 2–7's existing AC4 entries); (ii) gate behind `MIGRATION_IN_PROGRESS` flag and remove in Phase 8; (iii) replace "N fewer red cases" with concrete allow-lists of expected-failing case names per phase.

9. **Address mixed naming convention in code** (addresses: `Mixed numeric/semantic naming fragments scale`, `Reusing --size-chip outside chip components`, `Five new tokens have no inline rationale`)
   - Either commit to a single naming axis (recommended: rename chip → numeric tier in this PR), or add a section comment in `global.css` documenting the numeric-vs-semantic split rule with a worked example.
   - Document the convention in ADR-0026 §2 alongside the rule.

10. **Add safety scaffolding around single-PR delivery + emergency escape hatch** (addresses: `Contingency epic-split not in plan`, `Single-PR atomicity lacks rollback`, `No emergency escape hatch`, `No CI hook for AC2 sweeps`)
    - Add a brief "Contingency: epic-split" subsection naming trigger criteria and the first child-story's scope.
    - Add a "Rollback strategy" subsection covering partial-revert vs. full-revert decision.
    - Document a `FONT_SIZE_LITERAL_EXCEPTIONS` escape valve in ADR-0026 §2 or beside the test block.
    - Either wire AC2 sweeps into CI or annotate the vitest test as the authoritative AC2 implementation.

11. **Improve EXCEPTIONS reason hygiene during decrements** (addresses: `Decrementing EXCEPTIONS counts erodes justification`, `EXCEPTIONS edits across phases create coupling`)
    - When rewriting reasons during decrements, enumerate concrete remaining uses (e.g. `card padding-block at .card`, not just "card padding").
    - Expand Phase 8.1 to require each remaining EXCEPTIONS entry's reason to be specific enough to identify the literal's exact use site.

12. **Smaller standards/documentation fixes** (addresses: minor findings)
    - Phase 7.2: change "card title" to `.cardLabel` in EXCEPTIONS reason.
    - Phase 1.4: consider collapsing two `it()` per file into one.
    - Phase 4.2/5.1: add explicit `font-style: normal; font-variant: normal; font-stretch: normal;` to shorthand expansions (or document why omitted).
    - Phase 1.1: anchor `--lh-normal` resolves to `1.5` next to the line-height token mapping note.
    - Phase 8.3: either drop the "Phases" PR-description subsection or specify minimum content.
    - Phase 1.3: keep §2 tabular shape (or note asymmetry).

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound in adopting a category-level ban over per-occurrence EXCEPTIONS, which strengthens the boundary between authoring and enforcement and turns a tolerance-band convention into a checkable invariant. However, the mixed numeric/semantic naming convention for the new tokens, the reuse of `--size-chip*` outside chip components without renaming, and the retention of unused heading tokens introduce small but durable couplings between the token vocabulary and component semantics that will weaken the scale's evolutionary fitness.

**Strengths**: Category-level vitest ban; single source of truth sharpened by ADR/comment/grep triangle; TDD ordering; internally-independent phase decomposition; shorthand expansion closes structural loophole.

**Findings**:
- 🟡 (major, high) Mixed numeric/semantic naming fragments the scale's vocabulary — Phase 1.1 / Decisions.
- 🟡 (major, high) Reusing `--size-chip` / `--size-chip-md` outside chip components encodes a semantic lie — What We're NOT Doing.
- 🔵 (minor, medium) Retaining unused heading tokens as "defensive scaffolding" weakens the scale's discoverability.
- 🔵 (minor, high) Category-level ban regex couples to declaration syntax, not semantic intent — won't catch `--my-size: 12px; font-size: var(--my-size);` indirection.
- 🔵 (minor, medium) ADR amendment-in-place loses the rationale trail for the convention shift.

### Correctness

**Summary**: The plan's correctness story is largely sound — EXCEPTIONS decrement counts have been verified against live migration.test.ts and CSS sources and they match; tokens.ts entries align with the existing object shape; the new vitest regex correctly distinguishes literal numeric font sizes from var(--*) references. Two correctness gaps: (1) the FONT_*_RE will not actually be evaluated against `global.css` because `cssGlobals` globs `*.global.css` only, and (2) the proposed `--size-*` block ordering silently corrects a pre-existing chip/chip-md inversion.

**Strengths**: All decrement counts verified; tokens.ts shape correct; regex correctly handles `var(...)` exclusion and `0.88em` matching; new block strictly descending.

**Findings**:
- 🟡 (major, high) FONT_*_RE does not actually cover global.css (vite glob only picks up *.global.css).
- 🔵 (minor, high) Existing `--size-*` block is not strictly descending; the proposed reorder fixes a pre-existing inversion.
- 🔵 (minor, medium) FONT_SHORTHAND_RE — line-height edge case (`/1.5rem` would be flagged).
- 🔵 (suggestion, high) Token entries must be added in the typed object before the `} as const` boundary; "alphabetical" contradicts the existing px-descending convention.
- 🔵 (suggestion, medium) Phase 4.2 line-height token mapping should anchor `--lh-normal = 1.5`.

### Code Quality

**Summary**: Plan is unusually well-organised: TDD-first ordering, clear per-phase scope, explicit decrement-vs-delete reasoning. But several code-quality concerns affect maintainability: regex patterns have subtle gaps (leading-dot decimals), tokens.ts insertion contradicts file convention, the existing `--size-*` block is not strictly descending, Phase 1.5's Playwright placeholders defer too much discovery to implementation, EXCEPTIONS bookkeeping across 7 phases is error-prone, and one count appears arithmetically inconsistent.

**Strengths**: TDD-first Phase 1; explicit Automated/Manual Verification per phase; delete-vs-decrement documented; shorthand expansion justified; Phase 1.2 preempts cross-cutting failure; Phase 8 reconciliation gate.

**Findings**:
- 🟡 (major, high) FONT_SHORTHAND_RE has word-boundary edge cases and may double-fire / produce noisy failure output.
- 🟡 (major, high) `tokens.ts` insertion order contradicts file convention and plan's own ordering rule.
- 🔵 (minor, high) Existing block is not strictly descending — chip/chip-md pair is inverted; proposed reorder splits the pair.
- 🟡 (major, medium) FilterPill 10px count math appears inconsistent with the current EXCEPTIONS entry.
- 🟡 (major, high) Placeholder routes/selectors defer unbounded discovery to implementation.
- 🔵 (minor, high) Two `it()` per CSS file doubles the test surface for a single conceptual check.
- 🔵 (minor, medium) EXCEPTIONS edits in every phase create review-coupling and silent-conflict risk.
- 🔵 (suggestion, medium) Expansion produces verbose CSS without a clear precedent or comment.

### Test Coverage

**Summary**: The testing strategy is structurally solid (category-level vitest ban + Playwright computed-size spec + 3 AC2 sweeps), but the Playwright spec ships with unresolved route/selector placeholders whose feasibility is asserted but not verified, the plan offers no baseline/screenshot regression coverage for the documented deliberate drift, and Phase 1's red-suite TDD posture is under-specified for the long stretch between Phase 1 and Phase 8.

**Strengths**: Category-level enforcement bypasses EXCEPTIONS; 3-sweep AC2 layered net; both literal and shorthand forms covered; value-transition cases deliberately picked; viewport pinned; prototype-tokens.fixture.test.ts verified safe.

**Findings**:
- 🟡 (major, high) Playwright spec ships with unresolved placeholders and an unverified routing assumption.
- 🟡 (major, high) Vitest suite stays red across most of the migration without a documented dev workflow.
- 🟡 (major, medium) No automated regression coverage for the documented deliberate-drift case (inline code in headings).
- 🔵 (minor, high) Per-selector Playwright coverage is thin relative to migration surface (12 selectors vs ~37 sites).
- 🔵 (minor, high) Shorthand regex requires a leading whitespace boundary that may miss legitimate forms — and differs from AC2 third sweep.
- 🔵 (minor, medium) Playwright spec pins viewport but not theme.
- 🔵 (minor, high) Plan never asserts AC7 spec is wired into the visual-regression project run.

### Standards

**Summary**: The plan generally aligns with established codebase conventions (Playwright pattern reuse, AC verification, EXCEPTIONS decrement semantics, CSS comment placement), but has two material standards conflicts: (1) in-place amendment of an `accepted` ADR contradicts ADR-0031's immutability rule, and (2) the proposed insertion ordering for new entries in `tokens.ts` contradicts the existing px-descending ordering.

**Strengths**: AC2 verification mirrors work item; Playwright pattern reuse; correct per-entry EXCEPTIONS semantics; comment placement convention preserved; ADR-0026 §5 precedent for inline ADR references.

**Findings**:
- 🟡 (major, high) In-place ADR amendment contradicts ADR-0031 immutability convention.
- 🟡 (major, high) Proposed alphabetical insertion order for tokens.ts contradicts existing px-descending convention.
- 🔵 (minor, medium) Prose-only §2 replacement breaks tabular pattern used by surrounding subsections.
- 🔵 (minor, medium) Canonical comment style diverges from established short-heading convention.
- 🔵 (minor, medium) Naming a work item as 'amending' an ADR introduces unestablished vocabulary.
- 🔵 (minor, high) FONT_SHORTHAND_RE inconsistent with AC2 sweep (`\b` vs `\s`).

### Documentation

**Summary**: Generally well-documented with clear cross-references, but several documentation choices create longevity risks: the canonical comment and ADR amendment both reference work item 0075 by ID (coupling to ephemeral context), the proposed §2 text states the rule without rationale, and the EXCEPTIONS-reason decrement-without-deletion pattern erodes information about which font-size use cases were originally justified.

**Strengths**: Phase 1.3 specifies exact loci with line numbers; AC6a wording locked to work item; Phase 8.3 enumerates 4 PR description components; deliberate-drift screenshot capture chained to Phase 2; References enumerated; ADR amendment includes back-reference.

**Findings**:
- 🟡 (major, high) Proposed ADR §2 text states the rule but omits rationale for the convention change.
- 🟡 (major, high) Canonical comment references work item 0075 by ID, coupling code documentation to ephemeral context.
- 🟡 (major, high) Decrementing EXCEPTIONS counts without preserving original justification erodes documentation.
- 🔵 (minor, high) New tokens have no inline rationale; mixed naming convention is undocumented in code.
- 🔵 (minor, medium) ADR's new References entry assumes work item path stability.
- 🔵 (minor, medium) PR description structure underspecifies the 'Phases' summary.
- 🔵 (minor, medium) ADR §3 replacement footer line uses '0075' as the sole anchor.

### Compatibility

**Summary**: Generally sound — Playwright viewport pinned, `--lh-normal` confirmed = 1.5, regex won't false-positive on token declarations. But three real risks are insufficiently acknowledged: the deliberate-drift on inline `<code>` is materially larger inside headings than the prose suggests; the H1 `1.75rem` → fixed `28px` migration removes user-controllable root-font-size scaling (accessibility regression); and the `font:` shorthand → four-property expansion changes reset semantics.

**Strengths**: Viewport pinned; three-sweep ripgrep verification; vitest ban bypasses EXCEPTIONS deliberately; `--lh-normal` exact; line-height `1` left without token by design.

**Findings**:
- 🟡 (major, high) Inline `<code>` drift magnitude understated for in-heading occurrences.
- 🟡 (major, high) H1 `1.75rem` → fixed `28px` removes user-controllable root-font-size scaling.
- 🔵 (minor, medium) Shorthand expansion does not reset font-style/variant/stretch as the original shorthand did.
- 🔵 (minor, high) Phase 7.2 EXCEPTIONS reason references "card title" but selector is `.cardLabel`.
- 🔵 (minor, medium) Font-shorthand enforcement regex requires whitespace before numeric size; may miss leading-dot decimals.

### Safety

**Summary**: The plan ships a 37-site CSS migration plus ADR amendment plus harness changes as a single atomic PR, with the new vitest guard intentionally red between Phase 1 and Phase 8. Safety provisions for blast-radius containment are weak: the contingency epic-split path documented in the work item is not surfaced in the plan, cross-check greps are specified for only one of six decrement operations, and the post-merge rollback path for a subtle visual regression is unaddressed.

**Strengths**: Phase 8.2 final AC2 sweeps; TDD-first sequencing locks target state; Phase 4.2 instructs locate-by-content for line-shifted edits; per-phase manual verification of affected components.

**Findings**:
- 🟡 (major, high) Contingency epic-split path from work item not reflected in plan.
- 🟡 (major, high) Cross-check grep specified for only one of six decrement operations.
- 🟡 (major, high) Plan trusts existing EXCEPTIONS counts with no safety net for pre-existing miscounts.
- 🟡 (major, medium) No automated CI hook ensures AC2 ripgrep sweeps run on future PRs.
- 🔵 (minor, high) No documented procedure for legitimate emergency font-size literal.
- 🔵 (minor, high) Playwright spec asserts hardcoded px values brittle to future token re-tuning.
- 🔵 (minor, medium) Single-PR atomicity has no documented rollback path for post-merge visual regression.

---

## Re-Review (Pass 2) — 2026-05-23T16:10:00Z

**Verdict:** REVISE

The structural revisions resolve the bulk of Pass-1 findings cleanly — 13 of 16 majors and most minors are addressed. Two especially strong wins: the ADR-0035 supersession path is properly grounded in ADR-0031, and the cross-check ripgrep steps + Phase 1.5 baseline verification jointly close the count-drift hazard. **However, the edits introduce a handful of new major issues that would cause the plan to fail on first execution as written**: the chip rename's consumer coverage is incomplete (Chip/FrontmatterTable/MarkdownRenderer line 46 + 2 test files unaccounted for), the Phase 8.1 vitest snippet has missing imports and ESM `__dirname` problems, the regex's `^` boundary lacks the `m` flag despite the documented intent, and follow-up work item 0091 referenced repeatedly in the plan and ADR-0035 does not exist on disk. A focused third-pass revision can fix these mechanically.

### Previously Identified Issues

#### Architecture
- 🟡 **Mixed numeric/semantic naming** — Resolved (naming convention now documented inline in global.css).
- 🟡 **Reusing `--size-chip` outside chip components** — Resolved (renamed atomically to `--size-3xs-lg` / `--size-xxs-sm`).
- 🔵 Retaining unused heading tokens — Partially resolved (justification thin; reader still cannot tell consumed-vs-scaffolding by inspection).
- 🔵 Category-level ban regex couples to syntax — Partially resolved (fixture tests + `font-size:` anchor address most concerns; CSS-syntax-coupling remains by design).
- 🔵 ADR amendment-in-place — Resolved (ADR-0035 created; partial supersession via `superseded_by` linkage).

#### Correctness
- 🟡 **FONT_*_RE doesn't cover global.css** — Resolved via `allCssWithRoot` construct.
- 🔵 Existing block not strictly descending — Resolved (chip/chip-md inversion corrected with explicit note).
- 🔵 FONT_SHORTHAND_RE line-height edge case — Partially resolved (regex still matches unit-bearing line-heights; no fixture).
- 🔵 tokens.ts entry placement — Resolved (now px-descending).
- 🔵 Phase 4.2 line-height anchor — Resolved (anchored to `--lh-normal: 1.5` with inline comment).

#### Code Quality
- 🟡 **FONT_SHORTHAND_RE edge cases + failure UX** — Resolved (regex tightened; structured object expectation).
- 🟡 **tokens.ts insertion order** — Resolved.
- 🔵 chip/chip-md inversion — Resolved.
- 🟡 **FilterPill 10px count math** — Resolved (branching procedure with stop-and-reconcile gate).
- 🟡 **Playwright placeholders defer discovery** — Resolved (procedure tightened; `setup` callback; resolution recorded before Phase 1 closes).
- 🔵 Two `it()` per file — Resolved.
- 🔵 EXCEPTIONS edits across phases — Partially resolved (cross-checks help, but still touches 6 phases).
- 🔵 Shorthand expansion verbosity — Resolved.

#### Test Coverage
- 🟡 **Placeholders + routing assumption** — Resolved.
- 🟡 **Red-suite duration without workflow** — Resolved (vitest moved to Phase 8; suite green throughout).
- 🟡 **No coverage for deliberate-drift case** — Resolved (H2 inline-code case added; H1 still only via screenshots).
- 🔵 Per-selector coverage thin — Partially resolved (12 → 13 cases; finer FilterPill coverage still optional).
- 🔵 Vitest regex differs from AC2 — Resolved (divergence documented as deliberate strict upper bound).
- 🔵 Theme not pinned — Resolved (rationale documented).
- 🔵 Spec wiring not asserted — Resolved (`--list | grep` check added).

#### Standards
- 🟡 **In-place ADR amendment vs ADR-0031** — Resolved.
- 🟡 **tokens.ts ordering** — Resolved.
- 🔵 §2 tabular pattern — Resolved (single pointer line preserves §2's structural shape).
- 🔵 Canonical comment style — Resolved (multi-line block matches existing precedents).
- 🔵 "amends" vocabulary — Resolved.
- 🔵 FONT_SHORTHAND_RE vs AC2 — Resolved.

#### Documentation
- 🟡 **ADR §2 omits rationale** — Resolved (ADR-0035 has explicit Rationale section).
- 🟡 **Code references 0075 by ID** — Partially resolved (global.css comment fixed; ADR-0035 body still embeds "0075" in Context/Rationale/Consequences/References).
- 🟡 **EXCEPTIONS reason erosion** — Resolved (concrete remaining-use enumeration required in Phases 4/6/7 + Phase 8.3 audit).
- 🔵 New tokens have no inline rationale — Resolved (naming-convention comment block).
- 🔵 References path stability — Still present (ADR-0035 References embed mutable paths).
- 🔵 PR Phases underspecified — Resolved (minimum content per phase specified).
- 🔵 §3 footer 0075 anchor — Resolved (routed via ADR-0035).

#### Compatibility
- 🟡 **Inline code drift magnitude understated** — Resolved (per-heading deltas enumerated).
- 🟡 **H1 rem→px accessibility regression** — Resolved with caveat (px-anchored stance documented in ADR-0035 + 0091 follow-up — but 0091 doesn't exist on disk; see new findings).
- 🔵 font-style reset missing — Resolved (explicit `normal` resets added).
- 🔵 `.cardLabel` naming — Resolved.
- 🔵 Font-shorthand regex misses `.5px` — Resolved (leading-dot handling added).

#### Safety
- 🟡 **Contingency epic-split not in plan** — Resolved.
- 🟡 **Cross-check greps missing** — Resolved (all six decrements covered).
- 🟡 **Trust existing EXCEPTIONS counts** — Resolved (Phase 1.5 baseline verification).
- 🟡 **No CI hook for AC2 sweeps** — Partially resolved (CI wiring documented but left optional).
- 🔵 No emergency escape valve — Resolved (FONT_SIZE_LITERAL_EXCEPTIONS in ADR-0035).
- 🔵 Hardcoded px brittleness — Resolved (maintenance contract documented).
- 🔵 Single-PR rollback path — Partially resolved (default/surgical postures named; operational specifics thin).

### New Issues Introduced

The Pass-2 edits introduced these net-new concerns. The five **major** ones below would each block clean first-execution of the plan.

#### Major

- 🟡 **Correctness**: **`FONT_SHORTHAND_RE` `^` boundary requires `m` flag**
  **Location**: Phase 8.1 regex.
  The regex `(?:^|[\s/])` claims to catch the no-space `font:12px/1 …` case, but without the `m` flag `^` only matches the start of the input string (not line starts). A `font:12px/1 …` declaration anywhere except line 1 silently passes. The fixture for `font:12px/1 sans` would either fail (proving the regex broken) or pass only when the fixture string starts at position 0 of the test input. **Fix**: add the `m` flag, or replace `^` with a more general boundary like `[\s/:]`.

- 🟡 **Correctness**: **Phase 8.1 snippet uses `readFileSync`/`resolve`/`__dirname` without imports or ESM handling**
  **Location**: Phase 8.1 — `allCssWithRoot` construct.
  The snippet introduces `readFileSync(resolve(__dirname, 'global.css'), 'utf8')` but the existing `migration.test.ts` does not import these from `node:fs` / `node:path`. Vitest runs tests in ESM mode where `__dirname` is undefined. As written, the test file will fail to compile or throw `ReferenceError: __dirname is not defined`. **Fix**: add explicit imports + `fileURLToPath(import.meta.url)` derivation of `__dirname`, OR sidestep entirely via `import.meta.glob('../styles/global.css', { eager: true, query: '?raw', import: 'default' })`.

- 🟡 **Compatibility**: **Chip rename does not enumerate non-migrated consumer files**
  **Location**: Phase 1.1 — chip rename consumer coverage.
  A grep across the frontend source surfaces four consumer CSS files and two test files that the plan does not touch in any phase: `Chip.module.css:9,19`, `FrontmatterTable.module.css:13`, `MarkdownRenderer.module.css:46` (separate from the line 9/53 sites Phase 2 migrates), `Chip.test.tsx:67-68,114` (regex-matches the literal token name), `FrontmatterTable.test.tsx:215-216` (matches `--size-chip-md`). Once Phase 1.1 deletes `--size-chip` from `global.css`, these consumers become invalid `var()` references → visual regressions, plus the two tests break the vitest suite during Phase 1 (contradicting the plan's green-throughout invariant). **Fix**: add to Phase 1.1 explicit edits for all 6 sites + an automated verification: `rg -n 'size-chip' skills/visualisation/visualise/frontend/src` returns 0 matches.

- 🟡 **Compatibility / Documentation**: **Work item 0091 referenced repeatedly but does not exist on disk**
  **Location**: What We're NOT Doing; ADR-0035 Consequences; Migration Notes; References.
  The plan cites "follow-up work item 0091 (typography rem-vs-px stance review) created alongside this plan" in multiple places, including ADR-0035's Consequences section as the durable tracker for the accessibility regression. `meta/work/0091-typography-rem-vs-px-stance.md` does not exist. ADR-0035 ships with a dangling reference. **Fix**: create `meta/work/0091-…` as part of this PR (or as a precursor commit) with at least Context, AC placeholder, and `status: backlog` frontmatter.

- 🟡 **Documentation**: **Plan acknowledges but does not actionably plan the work-item 0075 update**
  **Location**: Plan §1.1 AC6a-wording paragraph; absent Phase/Success-Criteria step.
  The plan changes AC6a's canonical comment from `ADR-0026 (as amended by 0075)` to `ADR-0035`, and the supersession path obsoletes the work item's AC5 ("ADR-0026 is amended in place"). The plan notes inline that the work item needs updating but has no explicit Phase/sub-step or Success Criteria checkbox calling it out. The PR will ship with the work item's ACs misaligned against the implementation. **Fix**: add Phase 1.6 ("Update work item 0075") enumerating concrete edits: AC5 reframed for supersession-by-new-ADR; AC5a/b/c reframed against ADR-0035; AC6a wording updated; Decisions §"chip tokens are reused" reframed for the rename; baseline assumption phrasing reviewed.

- 🟡 **Code Quality**: **Phase 1 is now a multi-deliverable mega-phase**
  **Location**: Phase 1 structure.
  Phase 1 now bundles: global.css token-block extension + chip rename + comment split + tokens.ts updates + ADR-0035 authoring (with Context/Decision/Rationale/Consequences/Escape-valve/References) + ADR-0026 partial-supersession surgery (frontmatter + Status + §2 + §3 + Consequences) + Playwright scaffold + 13-case enumeration + baseline verification. If any one sub-step needs rework, the entire phase blocks. **Fix**: split into Phase 1a (ADR-0035 + ADR-0026 update — documentation only), Phase 1b (tokens + chip rename + comment), Phase 1c (Playwright + baseline). The contingency-epic split mechanism is already documented; consider applying it pre-emptively at phase level.

#### Minor / Suggestion

- 🔵 **Architecture**: "partial supersession" status model is implicit, not codified in ADR-0031. Consider a follow-up ADR proposing a `partially_superseded` status or `superseded_sections` field.
- 🔵 **Architecture**: `-sm` / `-lg` suffix grammar opens unbounded sub-pixel-tween growth. Add a single-sentence stop rule to ADR-0035 §Decision.
- 🔵 **Architecture**: AC2 sweeps + vitest test are parallel implementations of the same rule. Prefer the CI-script wiring option (8.2) to mechanically enforce alignment.
- 🔵 **Architecture**: `FONT_SIZE_LITERAL_EXCEPTIONS` escape valve has no sunset rule. Add one (e.g. entries reference a work item with target removal date).
- 🔵 **Correctness (suggestion)**: FilterPill `observed == 3` branch reasoning is misleading — say the EXCEPTIONS reason mis-attributes the `.badge` font-size as padding, not "the badge font-size site never existed".
- 🔵 **Code Quality**: Multi-line global.css comment block is verbose (11 lines for a 19-line token block). Consider keeping the AC6a one-liner in CSS and moving the naming-convention paragraph into ADR-0035's Decision.
- 🔵 **Test Coverage**: Moving the vitest ban to Phase 8 removes the per-phase TDD red→green signal. Per-phase Automated Verification relies on rg + Playwright + existing hygiene; acceptable but worth flagging.
- 🔵 **Test Coverage**: Per-FilterPill Playwright coverage is single-selector; a wrong-token migration (e.g. `.menuHeader` → `--size-3xs` instead of `--size-3xs-lg`) would pass vitest + sweeps but slip past Playwright.
- 🔵 **Standards**: ADR-0035 outlined sections (Context/Decision/Rationale/Consequences/Escape valve/References) diverge from ADR-0030 template (Context/Decision Drivers/Considered Options/Decision/Consequences/References). Reframe or fold sections.
- 🔵 **Standards**: ADR-0034 unambiguously defines `superseded_by` / `supersedes` and the value shape (`"adr:ADR-0035"`). Plan's hedge ("verify the exact field name when editing") and bracketed bare-identifier example contradict ADR-0034. Tighten to `supersedes: ["adr:ADR-0026"]` on ADR-0035.
- 🔵 **Standards (suggestion)**: `FONT_SIZE_LITERAL_EXCEPTIONS` name diverges from existing `EXCEPTIONS` convention — either unify or document the deliberate split inside ADR-0035 Decision.
- 🔵 **Documentation**: ADR-0035 body still embeds "0075" in Context / Rationale / Consequences / References. Reword to descriptive paraphrases; confine "0075" to the References list.
- 🔵 **Documentation**: AC6a "verbatim" wording is ambiguous against the multi-line block (the sentence is reformatted with leading ` * ` and a `.` instead of `*/`). Rewrite AC6a in the work item to specify "lead sentence of a comment block above the --size-* tokens".
- 🔵 **Compatibility**: `FONT_SIZE_LITERAL_EXCEPTIONS` is declared in ADR-0035 but Phase 8.1's test code doesn't actually consume it — surgical-revert path in Phase 8.4 depends on a contract that isn't implemented. Either implement the empty array with subtract-before-expect, or downgrade 8.4 to "full PR revert only".
- 🔵 **Compatibility**: FONT_SHORTHAND_RE boundary `(^|[\s/])` doesn't include `:` — `font:.5px/1 …` (no space, leading dot) is a pathological edge case the test misses but AC2 catches. Add `:` to the alternation or add a fixture.
- 🔵 **Safety**: Phase 1.5 baseline says "From `main` (clean working copy)" but the implementer is in a jj workspace with in-flight work. Reword to "create a fresh jj workspace at main" with the project's VCS conventions.
- 🔵 **Safety**: Phase 6.3 FilterPill reconciliation lacks abort criteria for the case where observed count is neither 3 nor 4. Add a step 5: "stop and re-scope the phase if observed count diverges from both expected branches".
- 🔵 **Safety**: Phase 8.4 rollback strategy lacks operational specifics (who decides default vs surgical; what regression-detection triggers a revert; concrete command sequence).

### Assessment

The plan is substantially stronger than its Pass-1 form. The Pass-2 revisions did exactly what they set out to do: ADR-0031 conformance, chip rename, regex tightening, baseline verification, EXCEPTIONS reason hygiene, accessibility documentation, contingency + rollback scaffolding. **However**, in addressing the structural concerns, the edits introduced new concrete issues — most importantly, the chip rename's consumer coverage gap and the Phase 8.1 vitest snippet's import + ESM issues, both of which would cause first-execution failures. A third pass focused on the five new majors above (chip consumers, regex/imports, Phase 1 split or scope acknowledgement, missing 0091, work-item update step) is the right next step. The minors are largely polish that can be batched with the same revision.

Recommended Pass-3 scope: roughly half a day of mechanical edits. After that, the plan should be ready to mark `status: ready` and begin implementation.

---

## Re-Review (Pass 3) — 2026-05-23T17:00:00Z

**Verdict:** REVISE

**Scope caveat**: 5 of 8 lens agents (test-coverage, standards, documentation, compatibility, safety) hit a usage-limit error before completing. This Pass-3 review records only the substantive findings returned by **architecture, correctness, and code-quality**. Re-running the missing 5 when credits reset is recommended before transitioning the plan to `status: ready`.

### Previously Identified Issues (covered by completed lenses)

#### Architecture
- 🔵 (Pass-2) Partial-supersession status model — **Resolved**: Pass-3 drops the coinage and uses scope-limited textual identification + `superseded_by` typed linkage.
- 🔵 (Pass-2) `-sm`/`-lg` suffix grammar unbounded — **Resolved**: ADR-0035 scale-extension policy added (design-review gate + "not infinitely extensible" statement).
- 🔵 (Pass-2) AC2/vitest dual-source-of-truth — **Resolved**: authoritative-impl comment now mandatory (Phase 8.2).
- 🔵 (Pass-2) `FONT_SIZE_LITERAL_EXCEPTIONS` sunset clause — **Resolved**: 12-week sunset added to ADR-0035.

#### Correctness
- 🟡 (Pass-2) `FONT_SHORTHAND_RE` requires `m` flag — **Resolved on the flag but introduces a new critical bug** (see below): `gm` flag added, but the boundary class still cannot reach the colon in `font:` and unit-bearing line-heights are still matched.
- 🟡 (Pass-2) Phase 8.1 snippet imports + `__dirname` — **Resolved**: `import.meta.glob` substituted for `readFileSync`.
- 🔵 (Pass-2) FilterPill observed==3 reasoning misleading — **Resolved**: rewritten to correctly identify the EXCEPTIONS reason as mis-attributing `.badge` font-size as padding.
- 🔵 (Pass-2) Chip-rename consumer enumeration — **Verified complete**: all 9 cited file:line references match the working tree byte-for-byte.

#### Code Quality
- 🟡 (Pass-2) Phase 1 mega-phase — **Resolved**: scope-acknowledgement paragraph + sub-PR-commit guidance + pre-emptive epic-split applicability.
- 🔵 (Pass-2) Multi-line global.css comment over-prescriptive — **Resolved**: comment now carries load-bearing naming-convention content.
- 🔵 (Pass-2) EXCEPTIONS scatter across 6 phases — **Accepted as designed**: per-phase rg cross-checks + Phase 8.3 audit compensate.
- 🔵 (Pass-2) Shorthand expansion verbosity — **Resolved**: in-file rationale paragraph in Phase 4.2 cross-referenced by Phase 5.

### New Issues Introduced

#### Critical

- 🔴 **Correctness**: **`FONT_SHORTHAND_RE` fails its own positive fixtures and matches its own negative fixture**
  **Location**: Phase 8.1 — regex + fixture list.
  Tracing `/font:\s*[^;]*(?:^|[\s/:])(?:\d+(?:\.\d+)?|\.\d+)(px|rem|em)\b/gm` against the Pass-3 fixtures:
  - **`font:12px/1 sans;`** (positive fixture) — does NOT match. The boundary class can only fire on whitespace, `/`, or `:` *after* the literal `font:` prefix. With no whitespace and `/` followed by `1 sans` (no unit), the regex finds no `[\s/:]` followed by a digit-unit pair.
  - **`font:.5px/1 sans;`** (positive fixture) — same failure mode.
  - **`font: 400 var(--size-xxs)/1.5rem var(--ac-font-body);`** (negative fixture) — DOES match: the `/` before `1.5rem` is in the boundary class, `1.5rem` matches digit-unit. The plan's "intentional over-match" comment 8 lines below contradicts the negative fixture.
  **Impact**: Phase 8.1 would either ship with a regex that fails its own test suite (blocking PR) or ship with a regex that misses no-space shorthand reintroductions (silent rule erosion). The plan's stated contract is incoherent.
  **Fix options**:
  - **(a)** Reclassify `font: …/1.5rem …` as a positive fixture and update the over-match note to say the over-match IS asserted; tighten the no-space fixtures with a lookbehind (`(?<=[\s/:])` instead of `(?:^|[\s/:])`) so the `:` in `font:` is reachable.
  - **(b)** Rewrite the regex to position-match only the font-size slot (parse `font:` then optional style/variant/weight tokens then mandatory size), trading regex complexity for fixture coherence.
  - **(c)** Drop the no-space shorthand fixtures, documenting that prettier/dprint normalises to space-separated form so the case never occurs in practice.

#### Major

(No new majors from the 3 lenses that completed.)

#### Minor

- 🔵 **Architecture**: `FONT_SIZE_LITERAL_EXCEPTIONS` 12-week sunset lacks a date anchor in the entry shape — sunset cannot be mechanically enforced. Either add an `added_at` field + assertion, or document the sunset as review-cadence (not CI-enforced).
- 🔵 **Architecture**: `-sm`/`-lg` stop rule is judgement-based (design-review gate) rather than mechanical. Consider a vitest cardinality bound on `--size-*` count to give the policy teeth.
- 🔵 **Architecture**: Pre-built escape valve (empty array + subtract loop) lowers the bar for adding the first exception. Consider deferring the implementation until first use, or document the pre-shipped posture as deliberate.
- 🔵 **Correctness**: `allCssWithRoot` `Object.fromEntries(...map(...))` construct collapses correctly but is indirect. If the glob ever finds zero files, global.css silently drops out of test scope. Simplify to `Object.values(globalCssModules)[0] ?? ''` with an explicit zero-file assertion.
- 🔵 **Code Quality**: Phase 1.6 brings Phase 1 sub-step count to six; consider landing 1.6 first so subsequent commits reference the updated work-item ACs.
- 🔵 **Code Quality**: Plan length (~1600 lines) is on the long side. Consider consolidating the ADR-supersession rationale (currently repeated in Overview, Current State Analysis, Desired End State, Phase 1.3) into a single subsection.
- 🔵 **Code Quality**: `FONT_SIZE_LITERAL_EXCEPTIONS` filter matches by literal string only — two same-literal sites in one file cannot be exempted independently. Consider adding a `count` field matching the existing `EXCEPTIONS` per-occurrence contract.

### Untested Lenses (Pass-3)

The following lenses did not complete; their Pass-2 concerns remain unverified for Pass-3:

- **Test Coverage**: viability of `import.meta.glob` for global.css; fixture-list comprehensiveness; line-5 mid-file fixture exercising the `m` flag; escape-valve loop correctness.
- **Standards**: ADR-0035 outline conformance to ADR-0030 template; `superseded_by` typed-linkage value shape correctness; `FONT_SIZE_LITERAL_EXCEPTIONS` naming divergence from `EXCEPTIONS`; embedding "scale extension policy" inside Decision vs Consequences.
- **Documentation**: ADR-0035 body still embedding "0075" in narrative; Phase 1.6 enumeration completeness; AC6a verbatim wording resolution.
- **Compatibility**: chip-rename consumer enumeration completeness (independent verification via `rg`); 0091 work-item existence and resolution; escape-valve contract implementation; FONT_SHORTHAND_RE colon-adjacent edge case.
- **Safety**: CI authoritative-impl comment mandate; Phase 1.5 jj workspace step; FilterPill abort criteria; rollback decision rubric; `jj backout` command correctness; sunset clause anchor.

These should be re-run when usage credits reset.

### Assessment

The Pass-3 edits address the 6 new majors from Pass-2 cleanly at the structural level — chip consumers enumerated and verified, work item 0091 created, ADR-0035 restructured to template, work-item update step added, rollback rubric concrete. The architecture and code-quality concerns from Pass-2 are fully closed.

**However**, Pass-3's regex edit introduces a critical correctness bug: the `FONT_SHORTHAND_RE` boundary cannot reach the `:` in `font:` (so no-space shorthand is missed) while simultaneously matching unit-bearing line-heights (so the negative fixture would fail). The plan ships with a fixture suite that contradicts the regex's actual behaviour.

**Recommended Pass-4 scope**: a single focused edit to resolve the regex contradiction (one of the three fix options above). Plus the optional minor polish items. The 5 untested lenses should re-run after the regex fix lands so the verification is complete before `status: ready`.

---

## Re-Review (Pass 4) — 2026-05-23T17:30:00Z

**Verdict:** REVISE

A full 8-lens re-review verified Pass-3's structural fixes (chip rename, ADR-0035, work-item update, rollback rubric) all hold up. However, three lenses (correctness, compatibility, test-coverage) independently flagged a critical regression in Pass-3's regex fix: `FONT_SHORTHAND_RE`'s negative lookbehind only excluded the digit immediately following `/`, but the lazy `[^;]*?` could advance past the `1` of `1.5rem` and start matching at the `5` where the preceding char was `.` not `/` — so the previously contradictory negative fixture `font: 400 var(--size-xxs)/1.5rem var(--ac-font-body);` still false-positive-matched on `5rem`.

Additional Pass-4 findings:
- Escape-valve subtraction logic was a no-op (`fontSizeHits` contained full match strings; `exemptForFile` contained bare literals — never equal).
- Regex would match `--ac-font-size-base: 12px;` and similar custom-property names containing `font-size:` as a substring.
- Surgical-revert command sequence (`jj diff | jj split`) is not actually executable.
- Default-revert sequence may double-describe the backout commit.
- ADR-0035 Consequences still embeds work-item ID 0091 in narrative (same ephemeral-ID anti-pattern that was fixed for 0075).
- AC5c wording inconsistency (`ADR-0035 / 0075` vs `ADR-0035`).
- Phase 1 Manual Verification hedge contradicts firm ADR-0034 commitment elsewhere.
- `FONT_SIZE_LITERAL_EXCEPTIONS` naming split from `EXCEPTIONS` lacks justification in ADR-0035 Decision.
- Per-FilterPill single-selector Playwright coverage gap not documented.

## Pass-5 Edits Applied — 2026-05-23T18:00:00Z

Pass-5 was a comprehensive edit batch:

1. **Regex (FONT_SHORTHAND_RE)** rewritten to `/(?<![\w-])font:[^;/]*?(\d+(?:\.\d+)?|\.\d+)(px|rem|em)\b/g`. The `[^;/]*?` structurally bounds the search to the pre-`/` portion of the shorthand so the engine cannot reach into the line-height slot at all (fixing the Pass-3/Pass-4 contradiction).
2. **Regex (FONT_SIZE_LITERAL_RE)** gained `(?<![\w-])` property-position lookbehind to reject custom-property declarations whose name contains `font-size:` as a substring.
3. **Capture groups** changed to plain `(...)` so `m[1] + m[2]` = bare `size+unit` (the natural exemption-key shape).
4. **Escape-valve subtraction** rewritten with per-occurrence `count` semantics matching the existing `EXCEPTIONS` shape. Shared `consumed` Map across both category subtractions so a single `count: 1` entry exempts ONE occurrence total (not one per category).
5. **CSS comments stripped** before regex matching (one-line `replace(/\/\*[\s\S]*?\*\//g, '')`) so documentation comments referencing literals don't false-positive.
6. **Zero-file glob assertion** added at module load — fires loudly if `import.meta.glob('../styles/global.css', ...)` ever resolves to anything other than exactly one file.
7. **Phase 1.7 added** scaffolding the 0091 stub (file already exists on disk; this documents the deliverable in-plan).
8. **AC5c wording aligned** to match the ADR-0026 update text verbatim.
9. **"Partial supersession" sweep** in plan narration → "scope-limited supersession".
10. **Phase 1 Manual Verification hedge tightened** to specify `superseded_by: "adr:ADR-0035"` (single ref, quoted string per ADR-0034).
11. **FONT_SIZE_LITERAL_EXCEPTIONS naming split justified** inside ADR-0035 §Decision (`EXCEPTIONS` is a per-occurrence admission ledger; `FONT_SIZE_LITERAL_EXCEPTIONS` is a near-empty escape valve; merging would lose the policy distinction).
12. **0091 narrative reference removed from ADR-0035 Consequences** (kept in References).
13. **Surgical revert command sequence rewritten** with `jj file show -r ... > <file>` + `jj commit` (executable, not `jj diff | jj split`).
14. **Default revert sequence** captures `BACKOUT_ID` from `jj log` and uses `jj describe -r $BACKOUT_ID` to avoid double-describing.
15. **1-week vs 12-week sunset distinction** documented — surgical-revert exceptions get 1 week; general ADR-0035 escape-valve entries get 12 weeks.
16. **Per-FilterPill Playwright coverage gap** documented explicitly in Testing Strategy with the risk-bounding mechanisms named.
17. **New negative fixtures added**: `--my-font-size: 12px;` (proper lookbehind exerciser), `font: 14pxsans;` (boundary check), `/* migrated font-size: 12px ... */` (comment stripping check).

## Pass-5 Focused Verification — 2026-05-23T18:30:00Z

Two lenses ran for verification: **correctness** and **test-coverage**.

### Correctness verification

**Verdict:** All 15 fixtures traced correctly. The Pass-5 regex is structurally sound.

Manual trace of the previously-broken case `font: 400 var(--size-xxs)/1.5rem var(--ac-font-body);`: `font:` matches at position 0; lazy `[^;/]*?` can only consume up to but not including `/`; the only digit chunk before `/` is `400` (no unit follows); engine cannot reach `1.5rem`; no match. ✓

Custom-property contexts: `--my-font-size: 12px;` — char before `font-size:` is `-`, lookbehind rejects. ✓

Escape-valve subtraction traced through four scenarios (zero/partial/full/unused exemption budgets) — all behave correctly.

**Minor findings (do not block):**
- Uppercase units (`14PX`) silently allowed — accepted as codebase-convention-enforced elsewhere.
- Comments inside `font:` shorthand (`font: /* comment */ 14px sans;`) cause silent false negative — extremely unusual CSS pattern, accepted as known limitation shared with AC2 sweeps.
- Exception keys `0.5px` vs `.5px` distinct — implementer writes the literal as it appears in source.

### Test-coverage verification

**Verdict:** Pass-5 escape-valve logic + capture groups + property-position lookbehind verified. Cross-category bleed bug **found and fixed in-Pass-5** (shared `consumed` Map).

**Minor remaining findings (do not block):**
- Escape-valve subtraction logic not yet unit-tested via synthetic fixtures (currently exercised only when an exemption is actually used). Would benefit from a sibling describe with 4 traced scenarios. Optional uplift.
- No declared-vs-observed hygiene check on `FONT_SIZE_LITERAL_EXCEPTIONS` (unused entries silently accumulate). Existing `EXCEPTIONS` has such a check; the new array does not. Acceptable for an escape valve that is expected to remain near-empty.
- Zero-file glob assertion is at module-load scope rather than inside a named `it()`. Vitest reports a load error rather than a named test failure. Functionally correct, slightly noisier in CI summaries.

### Assessment

The plan is now substantially complete and implementation-ready. The critical correctness bug from Pass-3/Pass-4 is resolved with high confidence (manual trace + verification lens agreement). Remaining findings are minor polish — useful uplifts for future iterations but not blockers for a first implementation pass.

**Recommended next step**: transition the plan to `status: ready` and begin Phase 1 implementation. The minor findings above can be addressed as part of implementation (e.g. the unit-tested subtraction logic naturally lands when the first exemption is added; the comment-stripping behaviour will be verified by the first PR that ships a literal-mentioning comment).
