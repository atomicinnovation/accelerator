---
date: "2026-05-23T15:00:00+01:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0075-typography-size-scale-consumption.md"
work_item_id: "0075"
review_number: 2
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: Typography Size-Scale Consumption Reconciliation

**Verdict:** COMMENT

The 2026-05-23 rewrite is implementation-ready. The expanded scope (35
outliers, 5 new tokens, `migration.test.ts` retirement, ADR-0026
amendment) is internally consistent across Summary, Context,
Requirements, Acceptance Criteria, and Decisions; the Playwright
verification mechanism and grep enforcement are concrete and executable.
No critical or major findings. The minor findings cluster around two
themes: the Acceptance Criteria list is unnumbered, which causes
ambiguous in-text references (clarity, testability), and the
single-PR-series contract sits in tension with the decomposition
fallback described in Drafting Notes (clarity, scope).

### Cross-Cutting Themes

- **AC checklist is unnumbered** (flagged by: clarity, testability) —
  in-text references like "AC2 grep" cannot be resolved to a specific
  checkbox; several individual AC clauses bundle multiple sub-criteria
  (AC6 has three sub-bullets, AC5 has two sub-clauses) that would be
  easier to verify if split.
- **Single-PR-series contract vs decomposition fallback** (flagged by:
  clarity, scope) — Requirements promises "no half-migrated intermediate
  state" while Drafting Notes pre-specifies a per-cluster decomposition
  path. The reader cannot tell whether the contract is one PR, an
  atomic stack, or a sequence where AC2 grep may fail until the final
  PR lands.

### Findings

#### Minor

- 🔵 **Clarity**: "AC2 grep" reference is ambiguous because Acceptance Criteria are unnumbered
  **Location**: Assumptions / Decisions
  The grep sweeps actually live in the third checkbox; the second
  checkbox is the conceptual "no orphan literals" statement. A reader
  cannot resolve "AC2" without counting and guessing.

- 🔵 **Clarity**: "Single PR series" is in tension with "no half-migrated intermediate state"
  **Location**: Requirements / Drafting Notes
  Implementers cannot pick a delivery strategy the author would accept
  without asking; reviewers cannot tell whether a partial-coverage PR
  violates the requirement.

- 🔵 **Dependency**: 0090 may be a downstream consumer rather than merely Related
  **Location**: Dependencies
  If 0090 depends on the enforcement pattern, canonical rule wording,
  and `migration.test.ts` retirement strategy this work item
  establishes, that is a directional coupling closer to Blocks than
  Related.

- 🔵 **Scope**: Story sizing sits at the upper bound given expanded surface
  **Location**: Requirements / Drafting Notes
  35-outlier migration across 9 files + tokens + enforcement test + ADR
  amendment is at the upper edge of what is typically a single story.
  The pre-specified decomposition lives in Drafting Notes rather than
  as a planned structure.

- 🔵 **Testability**: AC2 "no orphan literals" is subsumed by AC3 grep but stated as a separate, less-precise criterion
  **Location**: Acceptance Criteria
  AC2 is effectively a prose restatement of AC3+AC1 with no
  independent verification procedure.

- 🔵 **Testability**: AC7 (PR description + stylesheet comment wording) lacks a defined acceptance threshold
  **Location**: Acceptance Criteria
  No defined wording, minimum content, or check beyond a reviewer's
  subjective judgement that the text "states" the rule.

- 🔵 **Testability**: AC5 (`migration.test.ts` EXCEPTIONS update) lacks a mechanical check
  **Location**: Acceptance Criteria
  The "updated reason strings" clause has no defined target wording —
  reviewer must judge whether the new reason adequately drops the
  "font-size from design" framing.

- 🔵 **Testability**: AC6 (ADR-0026 amendment) verifiable only by inspection; three sub-bullets could be a checklist
  **Location**: Acceptance Criteria
  AC6 lists three things the ADR amendment must document but does not
  specify whether each must be explicitly addressed or whether a single
  sentence covering all three is acceptable.

#### Suggestions

- 🔵 **Clarity**: "Current-app", "prototype", and "deliberate-drift" rely on prior shared vocabulary
  **Location**: Context / Decisions
  A reader joining the project today will need to ask what "the
  prototype" is and what a "deliberate-drift" screenshot is before
  scoping the work.

- 🔵 **Scope**: ADR-0026 amendment is a separable concern bundled with the migration
  **Location**: Requirements (ADR-0026 amendment)
  Bundling the ADR with the code change is defensible — it does mean a
  CSS-focused implementer needs ADR-edit context. Keep as-is unless
  team prefers ADRs to land separately.

- 🔵 **Testability**: AC8 Playwright spec verifies 10 of ~35 outliers; coverage rationale is informal
  **Location**: Acceptance Criteria
  The choice of which 10 selectors are "representative" is not defined
  by an explicit rule — a future maintainer cannot reproduce the
  selection criterion if they need to extend it.

### Strengths

- ✅ Outlier inventory in Context is explicit and exhaustive (35
  outliers across 9 files), with each value mapped to an existing or
  proposed token; no ambiguity about scope.
- ✅ Five new tokens are named identically in Requirements, Acceptance
  Criteria, and Assumptions with consistent px values; mixed
  semantic/numeric naming is explained in Decisions.
- ✅ The single deliberate behavioural change (inline `<code>`
  `0.88em` → `--size-xs` `14px`) is called out separately in
  Decisions, AC, and Assumptions — no hidden visual deltas.
- ✅ AC3 provides three concrete ripgrep commands with exact patterns
  and a defined pass condition — fully automated.
- ✅ AC8 enumerates ten specific Playwright selectors with exact
  expected computed px values, naming the test file path and viewport
  source.
- ✅ Frontmatter is complete; all standard story sections present and
  substantively populated.
- ✅ Dependencies cleanly separates Blocks (0076), Related (0090), and
  tooling/convention couplings (Playwright harness, ADR-0026).
- ✅ Drafting Notes explicitly defends the story-vs-epic sizing choice
  and pre-specifies a sensible decomposition fallback.
- ✅ Scope boundaries are explicitly stated — current-app only,
  font-size only, no per-component carve-outs.

### Recommended Changes

1. **Number the Acceptance Criteria (AC1…AC8) explicitly and update
   in-text references** (addresses: "'AC2 grep' reference is
   ambiguous", "AC2 'no orphan literals' is subsumed by AC3", "AC6
   verifiable only by inspection; three sub-bullets could be a
   checklist", "AC5 lacks a mechanical check")
   Prefix each checkbox with `AC1.`…`AC8.` so in-text references
   (Assumptions, Decisions) resolve unambiguously. Then either drop AC2
   in favour of AC3, or rephrase AC2 with its own procedure. Split AC6
   into three sub-checkboxes (one per ADR amendment bullet) and AC5
   into two (the deletion check and the reason-string check) so each
   has its own pass/fail.

2. **Reconcile the "single PR series" contract with the decomposition
   fallback** (addresses: "'Single PR series' is in tension with 'no
   half-migrated intermediate state'", "Story sizing sits at the upper
   bound given expanded surface")
   Pick one delivery shape and describe it precisely in Requirements:
   either (a) "single PR — must merge atomically", (b) "stacked PR
   series that merges as a unit (intermediate PRs may not merge to
   main alone)", or (c) "a sequence of PRs where AC3 grep may fail
   until the final PR lands; each PR carries a 'WIP — typography
   migration N/N' tag". Move the decomposition fallback out of
   Drafting Notes into a Delivery Plan subsection so it's the
   planned-of-record path rather than a contingency.

3. **Promote 0090 to Blocks (or document why Related is intentional)**
   (addresses: "0090 may be a downstream consumer rather than merely
   Related")
   Either move 0090 to a Blocks entry with a one-line note that the
   coupling is pattern-reuse, or add a sentence under the 0090 Related
   entry clarifying that 0075 must land first so 0090 can adopt the
   established pattern verbatim.

4. **Specify the canonical comment wording or grep target for AC7**
   (addresses: "AC7 lacks a defined acceptance threshold")
   Either inline the exact one-line comment to add to `global.css`
   ("/* font-size consumers: use these tokens; see ADR-0026 §2 */"),
   or require the comment to contain the substring `ADR-0026` so the
   AC has a grep-able pass condition. Same treatment for the PR
   description.

5. **Specify the AC5 "reason string" template** (addresses: "AC5 lacks
   a mechanical check")
   Either supply the expected reason template (e.g. `reason: 'padding
   literal — not a font-size consumer'`), or require that no remaining
   EXCEPTIONS entry's `reason` field contains the substring `font-size`
   — that's a grep-able pass condition.

6. **(Suggestion) Add a Playwright selection rule in AC8** (addresses:
   "AC8 verifies 10 of ~35 outliers; coverage rationale is informal")
   Add a one-line statement: "Spec covers one selector per outlier file
   group plus the deliberate-drift case (inline code) and the
   relative-unit case (`MarkdownRenderer` H1)."

7. **(Suggestion) Gloss "current-app", "prototype", and
   "deliberate-drift" on first use** (addresses: "'Current-app',
   'prototype', and 'deliberate-drift' rely on prior shared
   vocabulary")
   One sentence in Context defining each term, e.g. "*current app*:
   the production frontend under `skills/visualisation/visualise/frontend/src/`;
   *prototype*: the investigative snapshot at
   `meta/research/design-inventories/.../prototype-standalone.html`;
   *deliberate-drift screenshot*: a screenshot captured to document an
   intentional visual change."

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates a coherent intent with strong
internal consistency between Summary, Context, Requirements, Acceptance
Criteria, Decisions, and Assumptions. The outlier inventory, token
additions, and Playwright assertions cross-reference cleanly. A few
clarity issues remain: the term "AC2 grep" used in Assumptions and
Decisions appears to refer to the grep sweep in AC3 (the checklist is
unnumbered), the phrase "single PR series" is in tension with "no
half-migrated intermediate state" and the possibility of follow-up PRs
noted in Drafting Notes, and a couple of domain phrases
("deliberate-drift", "current-app") would benefit from a one-line gloss
on first use.

**Strengths**:
- Outlier inventory in Context is explicit and exhaustive.
- The MarkdownRenderer inline-code change is called out separately as a
  deliberate visual delta — the only intentional behavioural change is
  unambiguously flagged.
- Five new tokens are named identically across Requirements, AC, and
  Assumptions with consistent px values.
- Disambiguators are applied where they matter (`EmptyState (library
  route)`, `font:` shorthand line numbers).

**Findings**:
- 🔵 **minor / high** — "'AC2 grep' reference is ambiguous because
  Acceptance Criteria are unnumbered"
- 🔵 **minor / medium** — "'Single PR series' is in tension with 'no
  half-migrated intermediate state'"
- 🔵 **suggestion / medium** — "'Current-app', 'prototype', and
  'deliberate-drift' rely on prior shared vocabulary"

### Completeness

**Summary**: The work item is thoroughly populated across all expected
sections for a story: Summary, Context, Requirements, Acceptance
Criteria, Decisions, Dependencies, Assumptions, Technical Notes,
Drafting Notes, and References. Frontmatter is complete with recognised
values, and the story-type expectations (motivating context, specific
testable criteria) are amply met. No structural or density gaps warrant
a finding.

**Strengths**:
- Frontmatter is complete with recognised values.
- Summary provides a clear single-paragraph statement of intent.
- Context is dense and motivates the work with a concrete 35-outlier
  inventory.
- Acceptance Criteria contains seven specific, enumerable criteria.
- Decisions section explicitly captures non-obvious choices.
- All standard sections substantively populated; References lists
  source research, the ADR to amend, and related work items.

**Findings**: (none)

### Dependency

**Summary**: The work item has strong dependency capture: upstream
blocker 0033 is named, downstream consumer 0076 is named as Blocks, the
ADR-0026 amendment is captured as a convention coupling that must land
in the same PR series, and the Playwright/`migration.test.ts` tooling
dependencies are explicit. The only potential gap is the relationship
to 0090, which is listed as Related but described as depending on
patterns this work item establishes.

**Strengths**:
- Upstream blocker 0033 explicitly named with rationale.
- Downstream consumer 0076 captured as Blocks with cross-reference.
- ADR-0026 captured as a Convention coupling rather than left implicit.
- Tooling dependencies (Playwright harness with named precedent specs,
  `migration.test.ts`) explicitly named with paths.
- Sibling work items 0073, 0074, 0077 correctly characterised as
  contextual rather than dependencies.

**Findings**:
- 🔵 **minor / medium** — "0090 may be a downstream consumer rather
  than merely Related"

### Scope

**Summary**: The work item describes a coherent unit of work organised
around a single rule (consume-tokens-everywhere for font-size) with all
sub-tasks — token additions, outlier migrations, test exceptions
retirement, and ADR amendment — serving that one rule. The story type
is defensible given the single guiding rule and single-PR-series
intent, though the 35-outlier surface area and four distinct artefact
classes (CSS, tokens, migration test, ADR) push the upper bound of what
a single story should carry. The author has anticipated this and
pre-described a decomposition fallback, which substantially mitigates
the sizing risk.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria all describe the same
  scope.
- Scope boundaries are explicitly stated — current-app only, font-size
  only, no per-component carve-outs.
- The ADR amendment and harness retirement are not independent concerns
  but direct consequences of adopting the single rule.
- Drafting Notes explicitly defends the story-vs-epic sizing choice and
  pre-specifies a sensible decomposition strategy.

**Findings**:
- 🔵 **minor / medium** — "Story sizing sits at the upper bound given
  expanded surface"
- 🔵 **suggestion / low** — "ADR-0026 amendment is a separable concern
  bundled with the migration"

### Testability

**Summary**: The Acceptance Criteria are unusually well-specified for
testability: three precise ripgrep sweeps provide automated
enforcement, a Playwright spec with named selectors and exact px
expected values is enumerated, and token additions are listed with
literal values. A few criteria reuse unbounded language ("every
outlier", "no orphan literals") that is well-supported by the explicit
inventory but could be tightened, and one criterion (PR description /
stylesheet comment) is verifiable only by manual inspection without
defined wording.

**Strengths**:
- AC3 provides three concrete ripgrep commands with exact patterns and
  a defined pass condition.
- AC8 enumerates ten specific selectors with exact expected computed
  px values.
- AC1 lists exact token names and px values to add.
- Decisions section explicitly resolves the one deliberate behavioural
  change with the post-migration expected value.
- Context bounds the otherwise-unbounded "every outlier" phrasing in
  AC4 with the explicit inventory.

**Findings**:
- 🔵 **minor / medium** — "AC2 'no orphan literals' is subsumed by AC3
  grep but stated as a separate, less-precise criterion"
- 🔵 **minor / high** — "AC7 (PR description + stylesheet comment
  wording) lacks a defined acceptance threshold"
- 🔵 **minor / medium** — "AC5 (`migration.test.ts` EXCEPTIONS update)
  lacks a mechanical check"
- 🔵 **minor / medium** — "AC6 (ADR-0026 amendment) verifiable only by
  inspection; three sub-bullets could be a checklist"
- 🔵 **suggestion / medium** — "AC8 Playwright spec verifies 10 of ~35
  outliers; coverage rationale is informal"

## Re-Review (Pass 2) — 2026-05-23

**Verdict:** COMMENT — implementation-ready; residual findings are
polish-grade minors with no critical/major findings.

The work item addressed every Pass 1 finding that had an actionable
recommendation. Four additional quick fixes were applied after Pass 2
re-review to resolve the more mechanical new findings introduced by the
pass-1 rewrites (AC5 branching, AC5c line-number reference, "likely
`--size-xs`" hedge, Sidebar shorthand line-number fragility). The
remaining new minors are subjective trade-offs (AC3 enumeration vs
reliance on AC2 grep, AC7 route specificity, deliberate-drift screenshot
ACs) that the team may treat as known minor debt.

### Previously Identified Issues

- 🔵 **Clarity**: "AC2 grep" reference ambiguous — **Resolved** (ACs
  numbered AC1–AC7 with explicit prefixes; in-text references resolve
  unambiguously to AC2 = ripgrep sweeps)
- 🔵 **Clarity**: "Single PR series" tension — **Resolved**
  (Requirements now says "single atomic PR"; decomposition reframed in
  Drafting Notes as escalation contingency)
- 🔵 **Clarity**: Vocabulary glosses — **Resolved** (Context opens with
  "Terms used below" subsection defining current app, prototype, and
  deliberate-drift screenshot)
- 🔵 **Dependency**: 0090 may be Blocks — **Resolved** (promoted from
  Related to Blocks with pattern-reuse rationale)
- 🔵 **Scope**: Story sizing at upper bound — **Still present**
  (re-review flags it again as minor; author's contingency framing
  accepted as appropriate mitigation)
- 🔵 **Scope**: ADR bundling — **Still present** (suggestion-grade; no
  change intended per the original decision to bundle)
- 🔵 **Testability**: AC2 subsumed by AC3 — **Resolved** (old AC2
  dropped)
- 🔵 **Testability**: AC7 PR-description wording threshold —
  **Resolved** (AC6a specifies verbatim global.css comment; AC6b
  requires ADR-0026 reference, making both grep-able)
- 🔵 **Testability**: AC5 EXCEPTIONS reason-string check —
  **Resolved** (AC4b is a grep-able pass condition)
- 🔵 **Testability**: AC6 ADR sub-bullets could be checklist —
  **Resolved** (split into AC5a/AC5b/AC5c)
- 🔵 **Testability**: AC8 selector criterion informal — **Resolved**
  (selection rule added: one per outlier file group + the two
  value-transition cases)

### New Issues Introduced and Their Resolution

Resolved by post-pass-2 quick fixes:
- 🔵 **Clarity / Testability**: AC5 "or is superseded by a new ADR"
  branching — **Resolved** (AC5 now commits to "amended in place")
- 🔵 **Testability**: AC5c line-number reference (line 287–290) —
  **Resolved** (locator switched to content; "line numbers shift as
  part of the same amendment" rationale noted)
- 🔵 **Clarity**: Requirements "likely `--size-xs`" hedge — **Resolved**
  (now commits firmly to `var(--size-xs)`)
- 🔵 **Clarity**: Sidebar `font:` shorthand line-number fragility —
  **Resolved** (selector names added: `.searchInput`, `.kbd`, `.link`;
  noted that AC2 grep is the authoritative locator)

Accepted as minor debt (not addressed):
- 🔵 **Clarity**: AC4b "no `font-size` substring" — hyphenation/case
  unspecified. Acceptable because the harness file is small and a
  reviewer can sanity-check manually.
- 🔵 **Clarity**: Assumption "no per-component carve-outs" reads as
  directive. Pre-existing wording; semantically equivalent to a
  requirement repeat.
- 🔵 **Dependency**: `prototype-tokens.fixture.test.ts` named in
  Assumptions but absent from Dependencies. Acceptable because the
  Assumptions already asserts no impact and the fixture audit was done
  during scoping.
- 🔵 **Dependency (suggestion)**: Contingency epic ordering implicit.
  Acceptable because the contingency is a fallback, not a planned
  path.
- 🔵 **Testability**: AC3 enumeration depends on Context prose.
  Acceptable because AC2's three zero-match sweeps are the authoritative
  mechanical pass condition; AC3 is a human-readable companion.
- 🔵 **Testability**: AC4a deletion scope can't be recomputed
  post-merge. Acceptable because AC4b's grep ("no `font-size` in
  reasons") provides the post-merge mechanical check.
- 🔵 **Testability**: AC7 route per selector not specified.
  Acceptable; spec author will pick the obvious mount route per
  selector and document choice in the spec header.
- 🔵 **Testability**: Deliberate-drift screenshot AC missing. Accepted
  as minor debt — Decisions documents the trade-off; reviewers will
  prompt for the screenshot if it's missing.

### Assessment

The work item is ready for implementation. The remaining minor findings
are quality-of-life refinements that can be addressed opportunistically
during the PR series or accepted as known debt. No critical or major
issues remain; the canonical rule, scale-widening, harness retirement,
ADR amendment plan, and Playwright regression spec are all
sufficiently specified for a planner and implementer to proceed.
