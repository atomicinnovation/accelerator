---
date: "2026-05-06T16:05:54+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0033-design-token-system.md"
work_item_id: "0033"
review_number: 1
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: Design Token System

**Verdict:** REVISE

The work item is structurally sound — every section is populated, the foundational role is clearly motivated, and the Acceptance Criteria for hex literals (AC3) and the screenshot review (AC5 after sharpening) are unusually concrete. The blockers are not gaps but inconsistencies: the token enumeration ("full `--ac-*` palette") is under-specified relative to both the inventory's ~24 tokens and the nine listed in Requirements; the px/rem grep (AC4) is simultaneously too narrow (only `*.module.css`, missing `wiki-links.global.css`) and too aggressive (will fail on irreducible 1–3px borders); and the dark-theme override block is described as in scope (Requirements: "theme-swapping shadows") and out of scope (Assumptions: "theme swap delivered by 0034") in the same document. The story also bundles four token families and a 17-module migration, which the gap analysis itself describes as two phases.

### Cross-Cutting Themes

- **Token enumeration ambiguity** (flagged by: clarity, testability) — "the full `--ac-*` palette" is asserted without naming which tokens count; Requirements list 9 while the inventory shows ~24.
- **Dark-theme override scope contradiction** (flagged by: clarity, testability) — Requirements call for "theme-swapping shadows" while Assumptions defer theme swap to 0034.
- **AC4 px/rem grep is broken in three independent ways** (flagged by: testability) — wrong glob, missing test exclusions, unreachable zero-match bar against 1px borders.
- **Raleway silently dropped** (flagged by: clarity, completeness, testability) — gap analysis lists four families; Requirements/AC list three with no rationale.
- **Test-fixture handling left as either/or** (flagged by: clarity, completeness, dependency, testability) — Technical Notes acknowledges but doesn't resolve the test-file policy; AC3 resolves it implicitly via grep flags, AC4 does not.
- **Scope/type mismatch** (flagged by: scope, completeness) — type=story but content is epic-shaped (four token families plus sweeping consumer migration).

### Findings

#### Major

- 🟡 **Clarity**: Dark-theme override scope is contradictory
  **Location**: Requirements / Assumptions
  Requirements call for "five elevation tokens covering both static and theme-swapping shadows" while Assumptions says "theme swap (light/dark) is delivered separately by 0034." The reader cannot tell whether the `[data-theme="dark"]` override block lands in this story or in 0034.

- 🟡 **Clarity**: "Every hex value listed in the inventory" is under-specified vs. AC3
  **Location**: Requirements (hex migration bullet) / Acceptance Criteria #1
  Requirements introduce sixteen hex examples with "e.g." while Technical Notes cites 168 literals across 21 files; AC3's grep then asserts zero matches. Similarly AC1 demands "the full `--ac-*` colour palette" while Requirements enumerate only nine of the inventory's ~24 `--ac-*` tokens.

- 🟡 **Clarity**: Summary restricts replacement to component CSS modules; Requirements/Technical Notes broaden it
  **Location**: Summary vs. Requirements / Technical Notes
  Summary says "replace inline values throughout component CSS modules" but Technical Notes flags `wiki-links.global.css` as in scope and AC3's grep covers all of `src/`. A reader who stops at Summary will under-scope.

- 🟡 **Dependency**: Google Fonts third-party dependency not captured
  **Location**: Dependencies
  Requirements mandate loading `Sora`, `Inter`, `Fira Code` via Google Fonts `<link>` tags but Dependencies records no external coupling. CSP, privacy, and availability implications are invisible to ops/security review.

- 🟡 **Scope**: Single story bundles four token families plus a sweeping migration across 17+ modules
  **Location**: Summary / Requirements / Drafting Notes
  The gap analysis describes this as two phases ("introduction" then "per-component sweep"). Bundling them as one story creates a long-lived branch, makes partial rollback impossible, and risks all-or-nothing acceptance. Drafting Notes already flags the four-way split as an alternative.

- 🟡 **Testability**: AC4 px/rem grep will fail on irreducible literals
  **Location**: Acceptance Criteria — AC4
  The spacing scale starts at `--sp-1: 4px`, so 1–3px borders, hairlines, and outline widths have no token equivalent. As written, "zero matches" is unreachable; a correctly-migrated PR would be marked failing.

- 🟡 **Testability**: AC4 glob excludes the global stylesheet that Requirements mandate migrating
  **Location**: Acceptance Criteria — AC4 vs. Technical Notes
  AC4 restricts to `-g '*.module.css'` while Technical Notes calls out `src/styles/wiki-links.global.css` as in scope. AC3 covers it via `--type css`; AC4 does not. Px/rem violations in `wiki-links.global.css` pass AC4 silently.

- 🟡 **Testability**: AC4 does not exclude test fixtures
  **Location**: Acceptance Criteria — AC4 vs. Technical Notes
  AC3 mirrors Technical Notes' "exclude tests" policy with `-g '!**/*.test.ts'` and `-g '!**/*.test.tsx'`; AC4 has no equivalent. Once AC4's glob is widened (per the previous finding), test fixtures with intentional px/rem will fail it.

- 🟡 **Testability**: "Full" palette not enumerated, leaving the token set under-specified
  **Location**: Acceptance Criteria — AC1
  Inventory lists ~24 `--ac-*` colour tokens; Requirements enumerate nine. Two implementations defining different subsets could both claim AC1 met. Downstream stories may discover missing tokens late.

- 🟡 **Testability**: Visual-parity check is subjective with no defined viewport, threshold, or reviewer
  **Location**: Acceptance Criteria — AC5
  AC5 names routes but not viewport dimensions, reviewer, tolerance threshold, or what "clean equivalent" means concretely. Two reviewers can reach opposite verdicts on the same screenshot pair.

#### Minor

- 🔵 **Clarity**: Typography family count silently reduced from four to three
  **Location**: Requirements (typography bullet)
  Gap analysis lists `Sora`, `Inter`, `Fira Code`, `Raleway`. Work item drops Raleway with no rationale. Inventory's Crawl Notes flag Raleway as "possibly aspirational, not referenced by any `--ac-*` token" — so the drop may be intentional, but the work item gives no signal.

- 🔵 **Clarity**: Passive voice obscures actor in screenshot-comparison criterion
  **Location**: Acceptance Criteria #5
  Three actions (taking screenshots, reviewing them, judging "no clean equivalent", listing exceptions) are all passive with no named actor.

- 🔵 **Clarity**: Test-fixture migration choice left as either/or in Technical Notes
  **Location**: Technical Notes
  Technical Notes describes the test-fixture problem as "exclude tests, or migrate explicitly" — but AC3 has already chosen "exclude". The disjunction in Technical Notes confuses readers.

- 🔵 **Completeness**: Open Questions may understate unresolved decisions
  **Location**: Open Questions
  Only the colour-mapping question is listed; test-fixture handling, story-splitting, and Raleway are surfaced in Drafting/Technical Notes but not as open decisions.

- 🔵 **Completeness**: Font-family list diverges from gap analysis without explanation
  **Location**: Requirements
  (Same as clarity finding above; reinforces the cross-cutting theme.)

- 🔵 **Completeness**: Test-fixture exclusion is acknowledged but inconsistently encoded in the criteria
  **Location**: Acceptance Criteria
  AC3 excludes test files; AC4 has no test-file exclusion. The inconsistency makes "done" ambiguous.

- 🔵 **Dependency**: Blocks list enumerates IDs without naming downstream work
  **Location**: Dependencies
  `Blocks: 0034, 0035, …, 0042` is bare; planning has to cross-reference those work items to interpret the coupling map.

- 🔵 **Dependency**: Test fixtures coupling not captured in Dependencies
  **Location**: Technical Notes
  `global.test.ts` parity invariant and the hex-bearing test files are surfaced in Technical Notes but not as Dependencies entries.

- 🔵 **Scope**: Web-font loading is a separable concern from token authoring
  **Location**: Requirements / Technical Notes
  Font-loading regressions (Google Fonts blocked, preconnect misconfigured) would force rollback of the entire token authoring change despite being independent of token definitions.

- 🔵 **Scope**: Type 'story' is plausibly undersized given the scope described
  **Location**: Frontmatter: type
  391 literals across 21+ files plus authoring four token families plus blocking nine downstream stories suggests epic.

- 🔵 **Testability**: Web-font AC drops `Raleway` without acknowledgement; no concrete load-success check
  **Location**: Acceptance Criteria — AC2 vs. Requirements
  AC2 says fonts "load" but does not specify how loading is verified.

- 🔵 **Testability**: No criterion verifies that component CSS modules actually consume the new tokens
  **Location**: Acceptance Criteria
  ACs only verify the *absence* of literals (AC3, AC4) and the *presence* of definitions (AC1) — not that `var(--*)` references actually appear in modules. A migration that deletes literals without adding token references could pass.

- 🔵 **Testability**: Dark-theme overrides not covered by any AC despite being part of the token spec
  **Location**: Acceptance Criteria — AC1 / Assumptions
  Inventory documents complete `[data-theme="dark"]` overrides; no AC asserts they're present, and Assumptions is ambiguous about whether the toggle UI or the dark token values themselves ship in 0034.

#### Suggestions

- 🔵 **Clarity**: AC4 regex doesn't address `0px` / `0rem` literals
  **Location**: Acceptance Criteria #4
  `0px`/`0rem` literals are ubiquitous in CSS resets and have no token equivalent; the criterion is silent on whether they count.

- 🔵 **Dependency**: Source inventory existence is an implicit prerequisite
  **Location**: Dependencies
  The target inventory is referenced as the canonical source for token values but not framed as a dependency.

### Strengths

- ✅ Domain-specific token names (`--ac-*`, `--sp-N`, `--radius-*`) are introduced with concrete values inline, removing any need for the reader to consult external glossaries.
- ✅ Cross-references to source documents (gap analysis, inventories, screenshots) are explicit and pathed, anchoring every claim.
- ✅ All standard story sections are present and substantively populated; frontmatter is complete and well-formed.
- ✅ Context explains the foundational/load-bearing motivation thoroughly, citing the gap analysis "Suggested Sequencing".
- ✅ AC3 and AC5 specify exact ripgrep commands and a route-by-route screenshot review protocol — reproducible and verifiable.
- ✅ Drafting Notes explicitly flag the four-way split as an alternative, demonstrating scope-awareness rather than accidental conflation.
- ✅ Theme-swap concern is correctly deferred to 0034 (a clean scope boundary even though it contradicts the "theme-swapping shadows" wording).
- ✅ Technical Notes are exceptionally detailed (file:line references, scope counts, test parity assertions, font loading wiring) — an implementer can start without follow-up questions.
- ✅ Numeric scales are stated identically across Context, Requirements, and Acceptance Criteria.

### Recommended Changes

1. **Resolve dark-theme override scope** (addresses: clarity #1, testability #5)
   Edit Requirements or Assumptions to state explicitly whether the `[data-theme="dark"]` override block is authored in 0033 or 0034. Reconcile "theme-swapping shadows" wording with the deferral. If dark token values land here and only the toggle UI ships in 0034, say so.

2. **Enumerate "the full `--ac-*` palette"** (addresses: clarity #2, testability #4)
   Replace AC1's "full `--ac-*` colour palette" with an explicit checklist of the ~24 tokens from the inventory (or a pointer to the inventory's token table as the canonical list). Mirror in Requirements so the two stay aligned.

3. **Fix AC4 grep on three axes** (addresses: testability #1, #2, #3)
   - Widen the glob to `*.css` (or list `wiki-links.global.css` explicitly) so the global stylesheet is exercised.
   - Add `-g '!**/*.test.ts'` and `-g '!**/*.test.tsx'` exclusions to mirror AC3.
   - Restate as "every spacing or radius value with a clean token equivalent has been replaced; remaining literals are listed in the PR description with a one-line justification" — adopting AC5's escape-hatch pattern so the criterion is reachable.

4. **Capture Google Fonts external dependency** (addresses: dependency #1)
   Add a `Depends on (external)` line to Dependencies naming Google Fonts (or note that fonts will be self-hosted to remove the coupling). Decision to be reached during refinement.

5. **Resolve Raleway** (addresses: clarity #4, completeness #2, testability #6)
   Either add Raleway back to Requirements/AC2 or add a one-line note (in Drafting Notes or as an inline Requirements annotation) confirming it's intentionally excluded per the inventory's "possibly aspirational" Crawl Note.

6. **Decide story vs epic** (addresses: scope #1, #3, completeness)
   Either reclassify as `type: epic` and decompose into at least two children (token authoring; consumer migration) — matching the gap analysis's two-phase framing — or commit to the single-story bundling and document the rollback strategy explicitly.

7. **Broaden Summary scope** (addresses: clarity #3)
   Update Summary to say "component CSS modules and global stylesheets" so its scope matches Requirements/Technical Notes/AC.

8. **Add a positive-coverage AC** (addresses: testability #7)
   Add an AC that asserts component modules actually reference the new tokens (e.g. minimum `var(--sp-` / `var(--radius-` / `var(--size-` / `var(--ac-` reference counts), so deletion-only migrations cannot pass.

9. **Tighten AC5 visual-parity** (addresses: testability #5, clarity #5)
   Specify viewport dimensions (e.g. 1440×900), reviewer (implementer captures, reviewer accepts), tolerance (pixel-diff or ΔE threshold), and a concrete definition of "clean equivalent" (e.g. ±1px on the spacing scale, ΔE < 5 on the colour palette).

10. **Promote Drafting/Technical decisions into Open Questions** (addresses: completeness #1, dependency #3)
    Move test-fixture handling, story-splitting, and Raleway from drafting/technical asides into Open Questions so they are gated before the work item is promoted to `ready`.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally well-structured and uses domain vocabulary consistently, but contains several material clarity gaps: an unresolved scope ambiguity around the dark-theme override block, an undefined-quantifier problem in 'every hex value listed' vs. the 168 actual literals, a Summary/Requirements scope mismatch around CSS modules vs. global stylesheets, and a passive-voice acceptance criterion that doesn't name the actor. The typography family count silently drops from four (in source) to three without explanation.

**Strengths**:
- Domain-specific token names introduced with concrete values inline.
- Cross-references to source documents are explicit and pathed.
- Drafting Notes explicitly flags the splitting decision.
- Pronouns and subject identity are consistent throughout.
- Numeric scales stated identically across Context, Requirements, and AC.

**Findings**: 3 major (dark-theme contradiction, "every hex" under-specification, Summary/Requirements scope mismatch); 3 minor (Raleway omission, passive voice in AC5, test-fixture either/or); 1 suggestion (0px/0rem boundary).

### Completeness

**Summary**: Structurally complete for a story: every expected section is present and substantively populated. Frontmatter well-formed. The main concern is the very large scope packed into a single 'story' — but that is a scope-lens concern rather than a completeness gap.

**Strengths**:
- All standard story sections present and substantively populated.
- Frontmatter complete and well-formed.
- Context explains motivation thoroughly.
- AC contains five specific bullets including concrete grep commands.
- Dependencies enumerates downstream blocked work items.
- Technical Notes exceptionally detailed.

**Findings**: 0 major; 2 minor (Open Questions understates unresolved decisions; font-family list diverges from gap analysis); 1 minor (test-fixture exclusion inconsistently encoded).

### Dependency

**Summary**: The work item captures the most important coupling (foundational role blocking 0034–0042) clearly, but introduces a third-party coupling on Google Fonts that is not captured, and the Blocks list enumerates IDs without describing downstream work.

**Strengths**:
- Dependencies explicit about no upstream blockers and explains why.
- All nine downstream consumers (0034–0042) enumerated in Blocks.
- Assumptions explicitly hands off theme-swap to 0034.

**Findings**: 1 major (Google Fonts external dependency missing); 2 minor (bare IDs in Blocks, test-fixture coupling not in Dependencies); 1 suggestion (source inventory implicit prerequisite).

### Scope

**Summary**: The work item describes a coherent foundational pass and treats it as a single 'story'. The bundling is internally justified and theme-swap correctly deferred, but the unit is large — four token families plus migration across 17+ modules — and the gap analysis itself describes this as two phases. Story type may be undersized; epic with children is a credible alternative the author already raises.

**Strengths**:
- Scope boundaries stated explicitly (theme swap deferred to 0034).
- Drafting Notes openly acknowledges bundling decision.
- Dependencies make foundational role clear.
- Assumptions locks scope to "visually neutral", preventing redesign creep.

**Findings**: 1 major (single story bundles four families plus sweeping migration); 2 minor (font loading separable, type 'story' plausibly undersized).

### Testability

**Summary**: The work item provides two strong fully-automated checks (AC3 hex grep, AC4 px/rem grep) and a route-named visual review (AC5), but the px/rem grep is broken three ways, AC1's "full" palette isn't enumerated, and AC5's visual parity has no defined viewport, threshold, or reviewer. As written, a tester cannot make a clean pass/fail call.

**Strengths**:
- AC3 and AC4 specify exact ripgrep commands with explicit globs and exclusions.
- AC5 names the specific routes whose screenshots must be re-taken.
- Requirements enumerate the literal hex values that must be migrated.
- AC5's escape-hatch (PR-listed exceptions) turns an unbounded mapping requirement into a verifiable one.

**Findings**: 5 major (AC4 unreachable on 1px borders; AC4 glob excludes wiki-links.global.css; AC4 missing test exclusions; AC1 "full" palette unenumerated; AC5 subjective); 3 minor (Raleway dropped without acknowledgement, no positive-coverage AC, dark-theme overrides not covered).

## Re-Review (Pass 2) — 2026-05-06T16:05:54+00:00

**Verdict:** REVISE

Most pass-1 blockers are resolved or substantially addressed: dark-theme scope, Google Fonts dependency, Summary breadth, Raleway, AC4 glob/test/escape-hatch, story-vs-epic decision, positive-coverage AC, and AC6 tolerances are all in place. The remaining majors are about precision-tightening rather than fundamental gaps: AC1 still defers token enumeration to the inventory without listing tokens locally, AC2 mixes a manual DevTools observation with a "at least one reference" floor, AC6's numeric tolerances aren't tied to a measurement tool, and Requirements bullet 1's typography-only framing reads as inconsistent with the broader "everything ships in tokens.ts and global.css" implication of AC1. The work item is implementable as-is; the question is how much of the remaining tightening is worth doing now versus deferring to the implementation PR.

### Previously Identified Issues

#### Major (pass 1)
- 🟢 **Clarity**: Dark-theme override scope contradiction — **Resolved**. Requirements bullet 4 ("Both layers ship in this story") and Assumptions reconcile cleanly.
- 🟡 **Clarity**: "Every hex value listed" / "full --ac-* palette" under-specified — **Partially resolved**. Now points at the inventory table; testability still flags this as needing local enumeration to be reviewer-mechanical.
- 🟢 **Clarity**: Summary restricts to component CSS modules — **Resolved**. Summary now says "component CSS modules and global stylesheets".
- 🟢 **Dependency**: Google Fonts third-party dependency — **Resolved**. Captured under "Depends on (external)" with CSP/runtime caveats.
- 🟢 **Scope**: Single story bundles four token families — **Resolved by decision**. User chose to keep as one story; Drafting Notes records the decision and reaffirmation. (Scope lens downgraded the residual concern to minor.)
- 🟢 **Testability**: AC4 will fail on irreducible 1–3px borders — **Resolved**. Escape-hatch in place.
- 🟢 **Testability**: AC4 glob excludes `wiki-links.global.css` — **Resolved**. Glob widened to `*.css` with explicit token-file exclusions.
- 🟢 **Testability**: AC4 missing test exclusions — **Resolved**. `*.test.ts` and `*.test.tsx` exclusions added.
- 🟡 **Testability**: AC1 "full" palette unenumerated — **Partially resolved**. Now references the inventory table explicitly, but testability flags this as still allowing reviewer disagreement on which inventory rows count.
- 🟡 **Testability**: AC6 (was AC5) visual-parity subjective — **Partially resolved**. ΔE / ±px / 5%-region tolerances added, but the measurement tool / sampling method is unspecified.

#### Minor / Suggestions (pass 1)
- 🟢 **Clarity**: Raleway silently dropped — **Resolved**. Requirements explicitly notes the deliberate exclusion with rationale.
- 🟢 **Clarity**: Passive voice in AC6 — **Resolved**. AC6 now names the implementer and the PR reviewer.
- 🟢 **Clarity**: Test-fixture either/or in Technical Notes — **Resolved**. Bullet rewritten to state the policy.
- 🟢 **Completeness**: Open Questions understated — **Resolved**. Explicitly closed with pointers to where each prior question was answered.
- 🟢 **Dependency**: Bare IDs in Blocks — **Resolved**. 0034 annotated as immediate successor; 0035–0042 grouped as component-level redesigns.
- 🟢 **Testability**: No positive-coverage AC — **Resolved**. AC5 added (≥300 `var(--*)` references).
- 🟢 **Testability**: Dark-theme overrides not covered by any AC — **Resolved**. AC1 explicitly requires the dark override layer.
- 🟢 **Suggestion**: Source inventory implicit prerequisite — **Resolved**. Now captured as "Depends on (artefact)".

### New Issues Introduced (Pass 2)

#### Major
- 🟡 **Clarity**: Requirements bullet 1 is internally inconsistent — its surface scope is typography-only ("Add the three-family typography stack ... to `src/styles/global.css` and `src/styles/tokens.ts`") but bullet 4 separately routes colour tokens to the same files and AC1 expects every token in tokens.ts. A reader cannot tell from bullet 1 alone whether colour, spacing, radius, and shadow tokens also land in `tokens.ts`. **Suggestion**: Add an umbrella sentence stating that every token enumerated below ships in both files with parity asserted.
- 🟡 **Clarity**: "Three-family" vs "four-family" mismatch with the source gap analysis — Context inherits the four-family description silently; only Requirements bullet 1 + Drafting Notes resolve the discrepancy. **Suggestion**: Add one sentence to Context naming the three families and noting Raleway is excluded.
- 🟡 **Testability**: AC1 defers the canonical token list to the inventory without local enumeration — verifiers can disagree about whether `--atomic-*`, `--fg-*`/`--bg-*` aliases, and per-theme shadow rows are in scope, and inventory revisions can retroactively change AC outcome. **Suggestion**: Inline a token table in the work item, or explicitly enumerate which inventory subsections are in scope.
- 🟡 **Testability**: AC2 mixes a manual DevTools observation with a "at least one reference" clause that's trivially satisfied. **Suggestion**: Replace with an automated `<link>`-tag assertion and name where each family must be applied (e.g. Sora on h1, Inter on body, Fira Code on code surfaces).
- 🟡 **Testability**: AC6's numeric tolerances (ΔE < 5, ±2px, ±1px, 5%-region) lack a specified measurement tool, sampling method, or formula choice. **Suggestion**: Either name the tool/configuration (e.g. Pixelmatch with threshold 0.1, CIEDE2000) or downgrade the tolerances to reviewer-judgement language.

#### Minor
- 🔵 **Clarity**: "PR description" referenced as a load-bearing artefact across AC4/AC6 + Requirements without defined sections.
- 🔵 **Clarity**: Requirements bullet 4's "complete `--ac-*` colour palette" overlaps with bullet 3's elevation tokens (per-theme shadows are in the colour table in the inventory).
- 🔵 **Clarity**: AC4 grep glob `'*.css'` is technically inclusive of `*.module.css` but ambiguous to a reader unfamiliar with rg semantics.
- 🔵 **Clarity**: Context's "fourteen hex values" vs Technical Notes' "168 hex literals" are different metrics but presented without that distinction.
- 🔵 **Clarity**: Open Questions references "the four-way split alternative documented in Drafting Notes" — Drafting Notes mentions the bundling but doesn't label a discrete alternative.
- 🔵 **Completeness**: Story does not explicitly name the "for whom" beneficiary (suggestion, low confidence).
- 🔵 **Dependency**: CSP allowlist update for fonts.googleapis.com is implied as ops/security action but not captured as a `Blocked by` entry.
- 🔵 **Dependency**: Ordering among the 0035–0042 Blocks entries is flattened ("all depend on the token layer") despite the gap analysis implying richer sequencing.
- 🔵 **Scope**: Authoring + migration phases are still bundled (downgraded from major; user accepted bundling).
- 🔵 **Testability**: AC4's "Irreducible-literal exceptions" list is forward-defined in the PR description with no upper bound — could swallow arbitrary literals.
- 🔵 **Testability**: AC5's 300-reference threshold has no derivation; could allow concentrated-in-one-file gaming.
- 🔵 **Testability**: AC6's screenshot route list doesn't pin to specific filenames in the inventory directory.
- 🔵 **Testability**: Requirements bullet 6 (font-family/font-size migration) has no dedicated AC; AC3/AC4/AC5 don't catch inline `font-family: system-ui` declarations.
- 🔵 **Suggestion**: Google Fonts privacy posture (GDPR/DPA) not flagged alongside CSP.

### Assessment

The work item moved from **REVISE (10 major / 13 minor / 2 suggestions)** to **REVISE (5 major / 14 minor / 1 suggestion)**. All ten pass-1 majors are either resolved or partially resolved; no pass-1 finding regressed. The five new majors are precision-tightening concerns (token enumeration locality, manual-vs-automated verification, measurement tool naming, internal Requirements consistency) rather than blockers — the work item describes a buildable story end-to-end, and a competent implementer could ship it as-is.

Per the configured threshold (`work_item_revise_major_count: 2`), the verdict remains REVISE. Pragmatic options:

1. **Accept as-is**: most remaining majors are reviewer-disagreement risks, not implementation blockers. The team may judge them tolerable for a foundational story whose canonical reference (the inventory) is stable.
2. **Tighten further**: address the two highest-leverage residuals — Requirements bullet 1's umbrella framing (clarity, ~5min edit) and AC1's local token enumeration (testability, ~15min edit). Those two cuts the major count to 3 and the verdict still reads REVISE, but the residual majors then are arguably stylistic.
3. **Lower the verdict bar**: this is one of those work items where a strict major-count threshold over-fires. Manually overriding to APPROVE / COMMENT is defensible if the team is comfortable.
