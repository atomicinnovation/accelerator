---
date: "2026-05-23T12:42:01+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0075-typography-size-scale-consumption.md"
work_item_id: "0075"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: Typography Size-Scale Consumption Reconciliation

**Verdict:** REVISE

The work item is well-structured, tightly scoped, and substantively populated
— all five lenses confirm it is a single coherent unit of work with clear
boundaries, named outliers, and explicit out-of-scope carve-outs. However,
testability flags two major issues: AC5's visual-regression criterion has no
defined verification procedure or tolerance, and AC1 risks tautology because
no acceptance criterion directly verifies that the Requirement-mandated new
tokens (`--size-eyebrow`, `--size-subtitle`) actually exist. These warrant
revision before implementation; the remaining minor findings cluster around
the same areas and should be addressed in the same pass.

### Cross-Cutting Themes

- **AC5 visual regression is underspecified** (flagged by: clarity,
  dependency, testability) — the criterion's "modulo deliberate
  normalisations" clause is open-ended, the verification method is undefined,
  and no supporting tooling is named in Dependencies.
- **Markdown-body Open Question gates AC5** (flagged by: clarity,
  testability) — until the `--size-sm` vs `14.5px` decision is resolved,
  AC5's baseline comparison set is ambiguous.
- **Scale-widening verification is missing** (flagged by: dependency,
  testability) — Requirements name new tokens (`--size-eyebrow`,
  `--size-subtitle`) but no AC enforces their presence; downstream consumers
  of those tokens are also generically named in Blocks rather than enumerated.

### Findings

#### Major

- 🟡 **Testability**: Visual regression criterion lacks verification procedure and tolerance
  **Location**: Acceptance Criteria (AC5)
  AC5 does not specify how "rendered `font-size` values match" is verified —
  no comparison tool, baseline capture procedure, viewports, or tolerance is
  defined. A reviewer can claim AC5 is met by any plausible procedure,
  undermining the regression guarantee.

- 🟡 **Testability**: AC1 is restated by AC2 and risks tautology without an enumeration
  **Location**: Acceptance Criteria (AC1)
  Once AC2 passes, AC1 is trivially true. AC1 alone does not require that
  the Requirement-mandated new tokens (`--size-eyebrow`, `--size-subtitle`)
  are actually present — the scale-widening step has no direct pass/fail
  check.

#### Minor

- 🔵 **Clarity**: 'Rule 1' and 'Rule 2' shorthand introduced without prior enumeration
  **Location**: Drafting Notes / Requirements
  The Requirements section references "Rule 1 (consume tokens everywhere)"
  and Drafting Notes mentions "Rule 2 (drop the scale)", but no enumerated
  list of rules appears in the work item or the referenced gap analysis
  under those labels.

- 🔵 **Clarity**: Visual-regression criterion's 'modulo' clause is conditional and underspecified
  **Location**: Acceptance Criteria (AC5)
  The "modulo any deliberate normalisations called out in the PR" clause
  delegates the allowed-changes rule to the PR description rather than
  fixing it in the AC, making in-scope vs scope-creep judgements unstable.

- 🔵 **Clarity**: Actor for 'the migration' and 'documentation' steps is implicit
  **Location**: Requirements
  The "add a short code comment to the tokens stylesheet" obligation is
  appended to a documentation requirement rather than carrying its own
  framing, making it easy to miss.

- 🔵 **Clarity**: `rg` command in AC uses a glob the reader must interpret
  **Location**: Acceptance Criteria (AC2)
  The glob `app/components/**/*.module.css` depends on shell expansion
  behaviour; reviewers may get inconsistent results depending on how `rg` is
  invoked.

- 🔵 **Dependency**: Blocks entry is generic rather than naming concrete consumers
  **Location**: Dependencies
  The generic "any downstream component-redesign work" entry hides whether
  sibling items 0073, 0074, 0076, 0077 have hard dependencies on 0075's
  output.

- 🔵 **Dependency**: Radius-tokens follow-up surfaced but not captured as a dependency relationship
  **Location**: Assumptions
  The radius follow-up is mentioned in Assumptions and Drafting Notes but
  no work item ID is referenced and no `Related:` entry captures the
  linkage.

- 🔵 **Dependency**: Visual regression check implies a tooling/process dependency that is not captured
  **Location**: Acceptance Criteria (AC5)
  AC5 implies a screenshot-diff or manual verification process exists, but
  no tooling, fixture, or supporting work item is named in Dependencies.

- 🔵 **Testability**: Glob pattern coverage is unverified for nested component directories
  **Location**: Acceptance Criteria (AC2)
  AC2 assumes all relevant CSS lives under `app/components/**/*.module.css`;
  global stylesheets or non-module CSS would pass AC2 while violating
  Requirement 1.

- 🔵 **Testability**: "Every component listed in Context" is verifiable but Context list may be non-exhaustive
  **Location**: Acceptance Criteria (AC3)
  Context names three outliers but Summary says "every hard-coded outlier"
  and Drafting Notes says "≈3–5 components" — the Context list's
  exhaustiveness is not committed to.

- 🔵 **Testability**: Markdown-body normalisation decision affects AC5 pass/fail
  **Location**: Open Questions
  The Open Question directly controls whether AC5's "deliberate
  normalisations" clause is exercised; AC5 is conditionally testable until
  the decision is resolved.

#### Suggestions

- 🔵 **Completeness**: Summary doubles as Context restatement
  **Location**: Summary
  The Summary blends what the work item does with how, trending long while
  Context re-presents much of the same framing.

- 🔵 **Scope**: Documentation requirement is a minor adjunct to the core migration
  **Location**: Requirements
  The PR-description and tokens-stylesheet comment requirement is correctly
  co-located with the migration; no action required unless the team prefers
  documentation to live in a separate design-system reference.

### Strengths

- ✅ Canonical rule is named explicitly ("Rule 1: consume tokens
  everywhere") and consistently referenced across Requirements, Acceptance
  Criteria, and Drafting Notes.
- ✅ Outlier locations are named with explicit file paths and px/rem values,
  so "the outliers" has a single unambiguous referent.
- ✅ All standard story sections are present and substantively populated;
  frontmatter is complete and well-formed.
- ✅ Drafting Notes capture rule-selection rationale and scope decisions,
  reducing follow-up clarification.
- ✅ Upstream blocker on 0033 is named explicitly with rationale; Drafting
  Notes explain why no further blockers exist.
- ✅ Scope is single-purpose: every requirement supports the canonical rule;
  out-of-scope carve-outs (prototype, radius outliers) prevent scope creep.
- ✅ AC2 specifies an exact, executable verification command — unambiguous
  pass/fail.
- ✅ Requirements provide concrete px values and target token names,
  removing intent ambiguity for verifiers.

### Recommended Changes

1. **Specify the AC5 verification procedure and tolerance** (addresses:
   "Visual regression criterion lacks verification procedure and tolerance",
   "Visual-regression criterion's 'modulo' clause is conditional and
   underspecified", "Visual regression check implies a tooling/process
   dependency that is not captured")
   Rephrase AC5 to specify the method (e.g. computed `font-size` via
   `getComputedStyle` snapshots, or DevTools inspection), enumerate the
   selectors to be checked, and either resolve the markdown-body Open
   Question now or name the allowed normalisation(s) explicitly rather than
   delegating to the PR description. Add the verification tooling to
   Dependencies or Technical Notes.

2. **Add an explicit AC for the scale-widening tokens** (addresses: "AC1 is
   restated by AC2 and risks tautology without an enumeration")
   Add an AC such as: "`--size-eyebrow` (11px) and `--size-subtitle` (13px)
   are present in the tokens stylesheet, slotted into the scale by px
   ordering." This gives the widening Requirement its own pass/fail check
   independent of consumption.

3. **Resolve the markdown-body Open Question** (addresses: "Markdown-body
   normalisation decision affects AC5 pass/fail", "Visual-regression
   criterion's 'modulo' clause…")
   Settle on `--size-sm` per the recommendation (or commit to a new token)
   and remove the conditional from AC5.

4. **Tighten the AC2 grep scope or broaden it to match Requirement 1**
   (addresses: "Glob pattern coverage is unverified for nested component
   directories", "`rg` command in AC uses a glob the reader must
   interpret")
   Either narrow Requirement 1 to match the AC2 glob exactly, or rewrite
   AC2 using `rg --glob '**/*.module.css' …` (or an additional sweep of
   `app/**/*.css` with explicit exclusions) so the verification covers
   every place a `font-size` literal could hide.

5. **Commit Context to being the exhaustive outlier inventory, or drop
   AC3's qualifier** (addresses: "'Every component listed in Context' is
   verifiable but Context list may be non-exhaustive")
   Either state explicitly in Context that the three listed outliers are
   the complete set, or rely solely on AC2's grep as the completeness gate
   and drop the "and every component listed in Context" qualifier from
   AC3.

6. **Replace the generic Blocks entry with concrete couplings** (addresses:
   "Blocks entry is generic rather than naming concrete consumers",
   "Radius-tokens follow-up surfaced but not captured as a dependency
   relationship")
   Enumerate which (if any) of 0073/0074/0076/0077 actually consume the
   new tokens or rely on the uniform-consumption invariant, and either
   file the radius-tokens follow-up as a sibling work item to reference
   under `Related:` or note that it is intentionally not yet captured.

7. **Drop the 'Rule 1' / 'Rule 2' numbering or introduce it in Context**
   (addresses: "'Rule 1' and 'Rule 2' shorthand introduced without prior
   enumeration")
   Either refer to the options by descriptive name only, or add a one-line
   preamble in Context enumerating the two candidate rules so the
   numbering has a referent.

8. **(Optional) Split the documentation requirement into two bullets**
   (addresses: "Actor for 'the migration' and 'documentation' steps is
   implicit")
   One bullet for the PR description rule statement, one for the tokens
   stylesheet comment.

9. **(Optional) Tighten the Summary** (addresses: "Summary doubles as
   Context restatement")
   Trim to a single-sentence noun phrase and let Context carry the
   migration mechanics.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear with well-defined scope,
explicit references, and concrete component names. It reads cleanly on first
pass, with most pronouns resolving unambiguously and the canonical rule named
consistently. A few residual ambiguities concern undefined 'Rule 1' / 'Rule
2' shorthand, an unexplained 'modulo' clause, and minor inconsistency between
the Open Questions recommendation and how the Acceptance Criteria treat the
markdown-body nudge.

**Strengths**:
- Canonical rule is named explicitly ('Rule 1: consume tokens everywhere')
  and consistently referenced across Requirements, Acceptance Criteria, and
  Drafting Notes.
- Outlier locations are named with explicit file paths and px/rem values, so
  'the outliers' has a single unambiguous referent.
- Scope boundaries are stated positively (current-app component CSS) and
  negatively (prototype out of scope, radius out of scope).
- Open Questions names a specific recommendation rather than leaving the
  question fully open.

**Findings**:
- 🔵 **minor / high** — "'Rule 1' and 'Rule 2' shorthand introduced without
  prior enumeration" (Drafting Notes): The Requirements section references
  Rule 1 / Rule 2 without any enumerated list of rules appearing in the work
  item or the referenced gap analysis under those labels. Suggestion: drop
  the numbering or introduce it in Context.
- 🔵 **minor / high** — "Visual-regression criterion's 'modulo' clause is
  conditional and underspecified" (Acceptance Criteria, AC5): The clause
  pre-authorises "deliberate normalisations" without bounding which are
  allowed. Suggestion: resolve the markdown-body Open Question and list the
  allowed normalisations explicitly, or constrain the clause to a named
  list.
- 🔵 **minor / medium** — "Actor for 'the migration' and 'documentation'
  steps is implicit" (Requirements): The tokens-stylesheet comment
  obligation is appended to a documentation requirement and easy to miss.
  Suggestion: split into two bullets, or accept the implicit actor.
- 🔵 **minor / medium** — "`rg` command in AC uses a glob the reader must
  interpret" (Acceptance Criteria, AC2): The glob depends on shell expansion
  behaviour. Suggestion: rewrite using `rg --glob` or specify the shell
  context.

### Completeness

**Summary**: The work item is well-structured and substantively populated
across all expected sections for a story. Frontmatter is complete with a
recognised type and status. No critical or major completeness gaps were
identified.

**Strengths**:
- All standard story sections present and substantively populated.
- Frontmatter is complete and well-formed.
- Context explains motivation and enumerates concrete outliers.
- Requirements are specific and actionable.
- Drafting Notes capture rule-selection rationale and scope decisions.
- Acceptance Criteria provide multiple distinct checks including an
  executable ripgrep command.

**Findings**:
- 🔵 **suggestion / low** — "Summary doubles as Context restatement"
  (Summary): The Summary blends what and how while Context re-presents the
  same framing. Suggestion: tighten Summary to a single-sentence noun
  phrase.

### Dependency

**Summary**: The work item captures its primary upstream blocker (0033) and
a generic downstream category, and its scope is well-bounded within the
current-app surface. However, the 'Blocks' entry is generic rather than
naming the concrete sibling items already cited (0073, 0074, 0076, 0077),
and a candidate radius-tokens follow-up is surfaced in Assumptions but not
captured as a Blocks/Related relationship.

**Strengths**:
- Upstream blocker on 0033 named explicitly with clear rationale.
- Drafting Notes explain why no further blockers exist.
- Assumptions explicitly carve radius outliers out of scope.
- Related items (0033, 0073, 0074, 0076, 0077) listed in References.

**Findings**:
- 🔵 **minor / high** — "Blocks entry is generic rather than naming
  concrete consumers" (Dependencies): The generic Blocks entry hides
  whether sibling work items have hard dependencies. Suggestion: enumerate
  concrete couplings or state explicitly that none have hard dependencies.
- 🔵 **minor / medium** — "Radius-tokens follow-up surfaced but not
  captured as a dependency relationship" (Assumptions): No work item ID is
  referenced and no Blocks/Related entry captures the linkage. Suggestion:
  file the follow-up and link it, or note explicitly that it is not yet
  tracked.
- 🔵 **minor / medium** — "Visual regression check implies a
  tooling/process dependency that is not captured" (Acceptance Criteria,
  AC5): No verification tooling, fixture, or supporting work item is named.
  Suggestion: name the verification mechanism in Dependencies or Technical
  Notes.

### Scope

**Summary**: The work item is a well-scoped, atomic story focused on a
single concern. The Summary, Requirements, and Acceptance Criteria describe
the same scope, and explicit out-of-scope carve-outs reinforce the boundary.
Sizing as a story is appropriate.

**Strengths**:
- Single unified purpose across all sections.
- Explicit out-of-scope statements sharpen the boundary.
- Drafting Notes justify type (story over epic) by citing bounded surface.
- Summary, Requirements, and Acceptance Criteria are consistent.
- Scale-widening work is appropriately bundled with the consumption
  migration.

**Findings**:
- 🔵 **suggestion / medium** — "Documentation requirement is a minor
  adjunct to the core migration" (Requirements): The documentation
  deliverable is tightly coupled to the rule being introduced. Suggestion:
  leave as-is unless the team prefers separate design-system docs.

### Testability

**Summary**: The Acceptance Criteria are largely concrete and verifiable,
with one criterion expressed as an executable ripgrep command and others
tied to specific files and tokens. However, AC1 contains a tautology-prone
phrasing, AC5's visual-regression criterion lacks a defined comparison
procedure or tolerance, and the AC set does not verify the requirement to
widen the scale with specifically-named tokens.

**Strengths**:
- AC2 specifies an exact, executable verification command.
- AC3 names specific files and a clearly bounded scope.
- AC4 prescribes two concrete documentation artefacts.
- Requirements provide concrete px values which make intent unambiguous.

**Findings**:
- 🟡 **major / high** — "Visual regression criterion lacks verification
  procedure and tolerance" (AC5): No comparison tool, baseline capture
  procedure, viewports, or tolerance is defined. Suggestion: rephrase to
  specify method (e.g. `getComputedStyle` snapshots) and enumerate
  selectors.
- 🟡 **major / high** — "AC1 is restated by AC2 and risks tautology
  without an enumeration" (AC1): Scale-widening step has no direct AC.
  Suggestion: add an explicit AC requiring `--size-eyebrow` (11px) and
  `--size-subtitle` (13px) in the tokens stylesheet.
- 🔵 **minor / medium** — "Glob pattern coverage is unverified for nested
  component directories" (AC2): Non-module CSS or global stylesheets would
  pass AC2 while violating Requirement 1. Suggestion: tighten Requirement 1
  to match AC2 or broaden AC2.
- 🔵 **minor / medium** — "'Every component listed in Context' is
  verifiable but Context list may be non-exhaustive" (AC3): Summary and
  Drafting Notes hint at more outliers than Context lists. Suggestion:
  commit Context to exhaustiveness or drop the qualifier from AC3.
- 🔵 **minor / medium** — "Markdown-body normalisation decision affects
  AC5 pass/fail" (Open Questions): AC5's baseline comparison set is
  ambiguous until the Open Question resolves. Suggestion: resolve before
  work starts.

## Re-Review (Pass 2) — 2026-05-23

**Verdict:** APPROVE (overridden from COMMENT by reviewer judgement — remaining findings are suggestion-grade and acceptable as minor debt)

The work item is implementation-ready. Both major findings from the
initial review are resolved, and ten of the eleven previously-identified
minor findings are fully addressed (one is partially addressed — the
Playwright tooling dependency is now named in Technical Notes but not in
Dependencies, which one lens still flags as a suggestion). Re-review
surfaced no critical or major issues; the remaining findings are all
suggestion-grade quality improvements.

### Previously Identified Issues

- 🟡 **Testability**: AC5 visual regression lacks verification procedure/tolerance — **Resolved** (now specifies Playwright `getComputedStyle` per enumerated selector; one residual viewport-spec suggestion logged below)
- 🟡 **Testability**: AC1 tautology / scale-widening missing AC — **Resolved** (new AC1 explicitly requires `--size-eyebrow` and `--size-subtitle` in the tokens stylesheet, slotted by px ordering)
- 🔵 **Clarity**: "Rule 1" / "Rule 2" shorthand without enumeration — **Resolved** (numbering dropped; rule referred to by descriptive name)
- 🔵 **Clarity**: AC5 "modulo" clause underspecified — **Resolved** (Decisions section bounds the markdown-body normalisation as the sole permitted exception)
- 🔵 **Clarity**: Tokens-stylesheet comment buried in doc requirement — **Resolved** (documentation requirement split into two bullets)
- 🔵 **Clarity**: `rg` glob depends on shell expansion — **Resolved** (rewritten to use `rg --glob` flag)
- 🔵 **Dependency**: Generic `Blocks` entry — **Resolved** (now `Blocks: 0076` with cross-reference to 0076's fallback sequence)
- 🔵 **Dependency**: Radius follow-up not tracked — **Resolved** (0090 filed as a minimal stub and listed under `Related:`)
- 🔵 **Dependency**: Visual regression tooling not captured — **Partially resolved** (Playwright harness named in Technical Notes; one lens still suggests surfacing it under Dependencies)
- 🔵 **Testability**: AC2 glob coverage incomplete — **Resolved** (second `rg` sweep covers `app/**/*.css` with tokens-stylesheet exclusion)
- 🔵 **Testability**: Context outlier list may be non-exhaustive — **Resolved** (Assumptions explicitly commits Context to the exhaustive inventory; AC4 qualifier dropped)
- 🔵 **Testability**: Markdown-body Open Question gates AC5 — **Resolved** (Decisions section settles markdown body on `--size-sm`; Open Questions section removed)
- 🔵 **Completeness**: Summary overlaps with Context — **Still present** (intentionally skipped as optional)
- 🔵 **Scope**: Documentation requirement minor adjunct — **Not applicable** (no change required per original finding)

### New Issues Introduced

All new findings are suggestion-grade; none block implementation.

- 🔵 **Clarity** (suggestion): "current-app CSS" is not explicitly linked to the `app/` directory the AC grep paths target. Suggestion: add one sentence in Context or Technical Notes binding the term to the `app/` tree.
- 🔵 **Clarity** (suggestion): Requirements name "global stylesheets" but no concrete global stylesheet file is identified beyond the excluded tokens stylesheet. Suggestion: name the file(s) in Technical Notes or state that the second `rg` sweep is forward-looking insurance.
- 🔵 **Clarity** (suggestion): AC4 parenthetical cross-reference to Assumptions adds a navigation hop. Suggestion: drop the parenthetical or inline the relevant clause.
- 🔵 **Dependency** (suggestion): Playwright harness named in Technical Notes but not in Dependencies. Suggestion: add a one-line Dependencies entry so the AC tooling reliance is visible alongside 0033.
- 🔵 **Scope** (suggestion): Tension between the single-PR-series constraint and the "exhaustive outlier inventory" assumption — if the grep sweep surfaces a surprise outlier requiring a new token, the unit-of-delivery boundary blurs. Suggestion: add a note on how newly discovered outliers are absorbed vs deferred.
- 🔵 **Scope** (suggestion): Playwright regression check is a small adjacent concern. No change required.
- 🔵 **Testability** (minor): Playwright snapshot viewport/route not fully specified. `MarkdownRenderer` H1 currently uses `1.75rem` which resolves against root font-size — two engineers on different routes could record different baselines. Suggestion: name the exact route(s) and viewport, or state the assertion is `computedStyle.fontSize === '<expected-px>'` independent of viewport.
- 🔵 **Testability** (minor): "States the canonical rule" criterion is a prose check with no minimum content. Suggestion: inline the required wording or enumerate key points (rule + rationale + scope).
- 🔵 **Testability** (minor): `rg` patterns match `font-size:\s*[0-9]` only — would miss leading-dot literals (`.875rem`), `font` shorthand with embedded sizes, or `calc()`/`clamp()` with literals. Suggestion: broaden the regex (e.g. `font(-size)?:\s*[.0-9]`) or add an explicit assumption that those forms are not in use.

### Assessment

The work item is ready for implementation. The remaining suggestions are
discretionary refinements that the implementer can address opportunistically
during the PR series or that the team may choose to leave as known minor
debt. None of them gate the consume-tokens-everywhere migration or change
the merge-time verification posture.
