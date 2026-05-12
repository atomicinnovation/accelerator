# Review Lens Improvements Implementation Plan

## Overview

Improve all 7 existing review lens skills to align with the optimal structure
identified in
`meta/research/codebase/2026-03-15-review-lens-optimal-structure.md`. Three
categories of improvement apply across lenses: adding perspective preambles
(PBR principle), strengthening evaluation questions to be generative rather than
checklist-style, and adding conditional applicability sub-groups to evaluation
questions.

## Current State Analysis

All 7 lenses follow a consistent template but lack three elements identified by
the research as optimal:

1. **No perspective preambles** — None of the 7 lenses state who the reviewer is
   reviewing as. The PBR research shows that adopting a stakeholder perspective
   improves review focus and defect detection.

2. **Inconsistent generative questioning** — Some lenses use generative "what
   happens when..." questions (architecture, security, test-coverage) while
   others use passive checklist-style questions (code-quality, standards,
   performance, usability).

3. **Limited conditional applicability** — Only the standards lens groups
   questions by when they apply (e.g., "when API changes are present"). Other
   lenses present flat question lists, which can produce irrelevant findings.

### Per-Lens Assessment

| Lens          | Perspective | Generative Qs | Conditional Groups     | Overall Improvement Needed |
|---------------|-------------|---------------|------------------------|----------------------------|
| Architecture  | Missing     | Good          | Needs improvement      | Light                      |
| Security      | Missing     | Excellent     | Already has sub-groups | Light                      |
| Performance   | Missing     | Moderate      | Needs improvement      | Moderate                   |
| Code Quality  | Missing     | Weak          | Needs improvement      | Significant                |
| Standards     | Missing     | Weak          | Already excellent      | Moderate                   |
| Test Coverage | Missing     | Good          | Needs improvement      | Light-moderate             |
| Usability     | Missing     | Moderate      | Needs improvement      | Moderate                   |

### Key Discoveries

- Each lens follows an identical section structure: Core Responsibilities, Key
  Evaluation Questions, Important Guidelines, What NOT to Do, Remember statement
  (`skills/review/lenses/*/SKILL.md`)
- The standards lens is the best example of conditional applicability — it
  groups
  questions under "Project conventions (always applicable)", "API standards (
  when
  API changes are present)", "Accessibility (WCAG) (when UI changes are
  involved)", etc. (`skills/review/lenses/standards-lens/SKILL.md:45-79`)
- The security lens is the best example of generative questioning — STRIDE
  categories naturally produce "Can an attacker..." questions
  (`skills/review/lenses/security-lens/SKILL.md:46-53`)
- The test-coverage lens's "mutation testing lens" is a strong generative
  pattern: "if I changed this operator, would any test fail?"
  (`skills/review/lenses/test-coverage-lens/SKILL.md:29-30`)
- Architecture lens already uses good generative phrasing: "What would change if
  a dependency changed?", "What happens under 10x load?"
  (`skills/review/lenses/architecture-lens/SKILL.md:59-75`)

## Desired End State

After implementation, all 7 lenses will:

- Have a one-sentence perspective preamble after the `# <Name> Lens` title,
  stating who the reviewer is reviewing as
- Use generative evaluation questions that drive production of derivative
  artifacts (failure scenarios, attack vectors, mutation cases) rather than
  passive checklist scanning
- Group evaluation questions by conditional applicability where the lens covers
  multiple sub-domains that aren't always relevant

### Verification

- All 7 lens skills contain a perspective preamble sentence
- Key Evaluation Questions sections use predominantly generative phrasing
  ("What happens when...", "Can...", "If I changed...", "Would...")
- Lenses with multiple sub-domains group questions with applicability conditions
- No changes to Core Responsibilities, Important Guidelines, What NOT to Do, or
  Remember statements (these are already well-structured)
- No changes to frontmatter, output formats, orchestrators, or the reviewer
  agent

## What We're NOT Doing

- Not changing Core Responsibilities sections — these are well-structured
- Not changing Important Guidelines or What NOT to Do sections — these are
  already effective
- Not changing Remember statements — these serve their purpose well
- Not changing frontmatter, output formats, orchestrators, or agent definitions
- Not adding new lenses — this plan is about improving existing ones
- Not changing the overall section structure of any lens
- Not rewriting lenses from scratch — making targeted improvements to two
  sections (preamble and Key Evaluation Questions)

## Implementation Approach

Work lens-by-lens across three phases: first add perspective preambles to all
lenses, then strengthen generative questions, then add conditional applicability
sub-groups.

**Each phase is interactive**: before making changes to any lens, present the
proposed changes and rationale to the user, ask focused questions about the
specific wording and approach, and only proceed after agreement. This ensures
each lens improvement reflects the user's vision rather than assumptions.

The interactive pattern for each lens within each phase is:

1. **Present**: Show the current state and proposed change for this lens
2. **Ask**: Ask specific questions about wording, scope, or approach
3. **Agree**: Get explicit agreement before editing
4. **Apply**: Make the change
5. **Confirm**: Show the result and move to the next lens

---

## Phase 1: Add Perspective Preambles ✅

### Overview

Add a one-sentence perspective preamble to all 7 lenses, placed immediately
after the `# <Name> Lens` title and before `## Core Responsibilities`. The
preamble states who the reviewer is reviewing as, helping the AI reviewer
inhabit a specific stakeholder role.

### Approach

For each lens, present a proposed preamble based on the research's perspective
mapping and ask the user for feedback before applying.

### Proposed Preambles

These are starting proposals — each will be discussed with the user before
applying:

| Lens          | Proposed Perspective                                                                                  |
|---------------|-------------------------------------------------------------------------------------------------------|
| Architecture  | "Review as a systems architect evaluating whether the structure will sustain the system's evolution." |
| Security      | "Review as an attacker probing for ways to compromise the system."                                    |
| Performance   | "Review as a capacity planner identifying where the system will bottleneck under load."               |
| Code Quality  | "Review as the next developer who will maintain this code in six months."                             |
| Standards     | "Review as a new team member navigating the codebase for the first time."                             |
| Test Coverage | "Review as a mutation tester trying to break the code without any test catching it."                  |
| Usability     | "Review as a developer using this API or interface for the first time."                               |

### Changes Required

For each of the 7 lenses, the change is identical in form:

**Files**: All `skills/review/lenses/*/SKILL.md`

**Change**: Insert a single paragraph after the `# <Name> Lens` heading and
before `## Core Responsibilities`.

Example for architecture lens:

```markdown
# Architecture Lens

Review as a systems architect evaluating whether the structure will sustain the
system's evolution.

## Core Responsibilities
```

### Interactive Process

For each lens:

1. Present the proposed preamble
2. Ask: "Does this perspective capture the right reviewer role for this lens?
   Would you phrase it differently?"
3. Apply the agreed wording
4. Move to the next lens

Process all 7 lenses in order: architecture, security, performance,
code-quality, standards, test-coverage, usability.

### Success Criteria

#### Automated Verification

- All 7 `SKILL.md` files contain a paragraph between the `# <Name> Lens`
  heading and `## Core Responsibilities`
- Each preamble's perspective is consistent with the lens's Remember statement
  and does not contradict Important Guidelines or What NOT to Do
- No other sections have been modified

---

## Phase 2: Strengthen Generative Evaluation Questions ✅

### Overview

Rephrase checklist-style evaluation questions into generative form for lenses
that need it. Generative questions drive the reviewer to produce derivative
artifacts (failure scenarios, bottleneck analysis, mutation cases) rather than
passively scanning against a checklist.

### Approach

For each lens that needs improvement, present the current questions alongside
proposed generative rewrites. Discuss each lens individually with the user
before applying changes.

**Transformation principle**: Convert "Check X" / "Assess Y" / "Evaluate Z"
patterns into "What happens when..." / "Can..." / "If I changed..." / "Would..."
interrogatives that force the reviewer to reason about consequences.

**Preservation rules** (apply to every rewrite):

1. **Keep the `**Bold dimension**:` prefix pattern** — this anchors the AI
   reviewer's systematic evaluation and maintains scannability across lenses
2. **Retain all specific evaluation dimensions** from the original question,
   either in the generative framing or as parenthetical sub-items. Example:
   `**Complexity**: If this function's requirements changed, how many places
   would need to change? (Watch for: cyclomatic complexity > 10, nesting
   depth > 3, functions > 50 lines.)`
3. **Keep questions concise** — one generative interrogative per dimension,
   not multi-sentence narratives
4. **Preserve approximate question count** — each lens should retain roughly
   the same number of evaluation questions after rewriting

**Industry framework convention**: Well-known framework questions (e.g., OWASP
Top 10, STRIDE) may retain their established phrasing where it is widely
recognised by practitioners, even if it is more checklist-style.

### Per-Lens Assessment and Changes

#### Architecture Lens — Light Touch

**File**: `skills/review/lenses/architecture-lens/SKILL.md`

Current questions are already predominantly generative. Minor improvements
possible:

- "Could a module be replaced independently?" — already generative
- "What would change if a dependency changed?" — already generative
- "What happens under 10x load?" — already generative

**Proposed**: Review with user whether any questions need minor rephrasing. This
lens may need no changes in this phase. Candidate rewrites if changes are
warranted:

- "Could a module be replaced independently?" → "**Modularity**: If this module
  needed to be replaced, what else would break?"
- "What would change if a dependency changed?" → "**Coupling**: If a dependency
  released a breaking change, how many files would need to change?"

#### Security Lens — No Changes Needed

**File**: `skills/review/lenses/security-lens/SKILL.md`

STRIDE categories are inherently generative ("Can an attacker impersonate...",
"Can data be modified without detection..."). OWASP section is more
checklist-style but is a well-known framework that reviewers expect in its
standard form.

**Proposed**: No changes. Confirm with user.

#### Performance Lens — Moderate Improvement

**File**: `skills/review/lenses/performance-lens/SKILL.md`

Current questions mix checklist and generative styles. Examples of checklist
patterns that could be strengthened:

- "Are data structures chosen for the access patterns used?" → more generative
  form might be: "What access pattern does this data structure optimise for, and
  does that match how it's actually used?"
- "Are connections, handles, and pools managed correctly?" → "What happens to
  this resource if the operation fails halfway through? Will it be released?"
- "Is caching applied where it would reduce load?" → "Which operations are
  repeated with the same inputs? What would the cost/benefit be of caching
  them?"

**Proposed**: Present each question with a proposed rewrite and discuss.

#### Code Quality Lens — Significant Improvement

**File**: `skills/review/lenses/code-quality-lens/SKILL.md`

Most questions are metric/checklist-style:

- "Cyclomatic complexity, cognitive complexity, nesting depth, method/function
  length" — these are measurements, not generative questions
- "Meaningful naming, small focused units, minimal side effects, guard clauses"
  — these are properties to check, not scenarios to explore

**Proposed**: Rewrite questions to be generative. For example:

- "**Readability**: Will the next developer understand this code in six months
  without the original author's context? (Watch for: cognitive complexity,
  nesting depth > 3, unclear naming, missing guard clauses.)"
- "**Coupling**: If this function's requirements changed, how many other places
  would need to change with it? (Watch for: SRP violations, feature envy, data
  clumps, shotgun surgery.)"
- "**Error handling**: If this error occurred in production at 3am, would the
  error message and stack trace lead you to the root cause? (Watch for: swallowed
  exceptions, generic messages, missing context, unlogged error paths.)"

Present full set of proposed rewrites for discussion.

#### Standards Lens — Moderate Improvement

**File**: `skills/review/lenses/standards-lens/SKILL.md`

Questions are entirely checklist-style ("Naming conventions", "Resource naming",
"Semantic HTML structure"). The conditional applicability grouping is excellent
but the questions within each group could be more generative.

**Proposed**: Rewrite questions within existing groups. For example:

- "Naming conventions (files, functions, variables, classes, modules)" → "If a
  new developer searched for this functionality, would the file and function
  names lead them to it?"
- "Error response format and consistency" → "If a consumer received this error,
  would they know what went wrong and how to fix it without reading the source
  code?"

Present proposed rewrites for discussion.

#### Test Coverage Lens — Light Improvement

**File**: `skills/review/lenses/test-coverage-lens/SKILL.md`

Already has the excellent "mutation testing lens" pattern. Some questions could
be strengthened:

- "Are new code paths or planned features covered by tests?" → "**Coverage
  adequacy**: If I introduced a subtle bug in this code path, which specific
  test would catch it?"
- "Are tests deterministic? Free from shared mutable state?" → "**Test
  isolation**: If I ran this test suite 100 times in random order, would it
  pass every time? (Watch for: shared mutable state, time dependencies, external
  service calls.)"

**Proposed**: Present proposed rewrites for discussion, preserving the existing
mutation testing framing and following the bold-prefix format.

#### Usability Lens — Moderate Improvement

**File**: `skills/review/lenses/usability-lens/SKILL.md`

Mixed generative and checklist patterns. Some questions are already good ("Does
anything behave in an unexpected way?"). Others could be strengthened:

- "Do similar operations work the same way?" → "If a developer learned how to
  do operation A, could they guess how to do operation B without reading docs?"
- "Is the API surface as small as it can be?" → "Which parts of this API could
  be removed without losing the ability to accomplish any use case?"

**Proposed**: Present proposed rewrites for discussion.

### Interactive Process

For each lens that needs changes:

1. Present the current Key Evaluation Questions section
2. Show proposed generative rewrites alongside the originals
3. Ask: "Do these rewrites capture the same intent with better generative
   framing? Would you adjust any of them?"
4. Apply agreed changes
5. Move to the next lens

Process in order of change magnitude: security (confirm no changes),
architecture (confirm light/no changes), test-coverage, usability, performance,
standards, code-quality.

### Success Criteria

#### Automated Verification

- All modified `SKILL.md` files still contain a `## Key Evaluation Questions`
  section
- No changes to Core Responsibilities, Important Guidelines, What NOT to Do, or
  Remember statements
- Questions use predominantly interrogative phrasing ("What...", "If...",
  "Would...", "Can...", "Which...") — at least 75% of questions per lens
- All questions use the `**Bold dimension**:` prefix pattern
- Each lens retains approximately the same number of evaluation questions as
  before
- All specific evaluation dimensions from original questions are preserved
  (either in the generative framing or as parenthetical sub-items)

---

## Phase 3: Add Conditional Applicability Sub-Groups ✅

### Overview

Group evaluation questions by when they apply, so reviewers skip irrelevant
sections. The standards lens already demonstrates this pattern well with groups
like "API standards (when API changes are present)" and "Accessibility (WCAG)
(when UI changes are involved)".

### Approach

For each lens that would benefit, propose a grouping structure based on the
sub-domains within the lens's evaluation questions. Discuss with the user before
applying.

Not all lenses need this — it's most valuable for lenses that cover multiple
distinct sub-domains where some are frequently irrelevant.

**Applicability design principles**:

1. **Conditions must describe observable characteristics** of the code or plan,
   not intent. Use "when the change introduces new classes or interfaces" rather
   than "when refactoring is involved."
2. **Every lens must have at least one "always applicable" group** — this
   ensures a baseline set of questions is always evaluated regardless of change
   type.
3. **When in doubt, include** — the reviewer should err toward evaluating a
   sub-group rather than skipping it. The parenthetical conditions are hints,
   not hard filters.
4. **Follow the standards lens formatting convention** — use
   `**Category name** (condition):` followed by a bulleted list, matching the
   pattern in `skills/review/lenses/standards-lens/SKILL.md`.

### Per-Lens Assessment

#### Architecture Lens — Light Improvement

**File**: `skills/review/lenses/architecture-lens/SKILL.md`

Currently has flat questions covering modularity, coupling, scalability,
resilience, evolutionary design, functional core/imperative shell, and domain
alignment.

**Proposed grouping**:

- **Structural integrity** (always applicable): modularity, coupling, domain
  alignment
- **Resilience** (when external dependencies or failure modes are present):
  retry, degradation, timeouts, idempotency
- **Scalability** (when the change affects data volume, request rate, or
  resource consumption): 10x load, horizontal scaling, bottlenecks
- **Evolutionary fitness** (always applicable): evolutionary design, functional
  core/imperative shell

Note: These sub-groups deliberately use different granularity than the Core
Responsibilities groups. Core Responsibilities define *what* to evaluate;
sub-groups here define *when* each set of questions applies. Discuss with user
whether this distinction is clear or whether the groups need adjustment to
avoid redundancy.

#### Security Lens — No Changes Needed

**File**: `skills/review/lenses/security-lens/SKILL.md`

Already has excellent sub-groups: STRIDE categories, OWASP Top 10, and
full-stack security layers. No improvement needed.

**Proposed**: Confirm with user.

#### Performance Lens — Moderate Improvement

**File**: `skills/review/lenses/performance-lens/SKILL.md`

Currently has flat questions covering algorithmic complexity, data structures,
hot paths, database, resources, I/O, concurrency, and caching.

**Proposed grouping**:

- **Algorithmic efficiency** (always applicable): complexity, data structures,
  hot paths
- **Database and query performance** (when the change includes database queries
  or schema changes): N+1, indexes, batching, result set bounds
- **Resource and I/O efficiency** (when the change opens connections, makes
  network calls, or handles file I/O): connections, pools, streaming, payload
  size
- **Concurrency safety** (when the change uses threads, async/await, or shared
  mutable state): shared state, locks, async/await, thread pools
- **Caching** (when the change involves repeated lookups or high-frequency
  access patterns): cache strategy, invalidation, TTLs

Present to user for discussion.

#### Code Quality Lens — Moderate Improvement

**File**: `skills/review/lenses/code-quality-lens/SKILL.md`

Currently has flat questions covering complexity, clean code, design principles,
code smells, readability, error handling, observability, and testability.

**Proposed grouping**:

- **Readability and complexity** (always applicable): complexity, naming,
  readability, code smells
- **Design principles** (when the change introduces new classes, interfaces, or
  abstractions): SRP, OCP, DI, ISP, composition
- **Error handling and observability** (when the change includes error paths,
  catch blocks, or logging statements): error categorisation, propagation,
  logging, tracing
- **Testability** (always applicable): dependency injection, independent
  testing, proportional complexity

Present to user for discussion.

#### Standards Lens — No Changes Needed

**File**: `skills/review/lenses/standards-lens/SKILL.md`

Already has the best conditional applicability grouping in the codebase. No
improvement needed.

**Proposed**: Confirm with user.

#### Test Coverage Lens — Light Improvement

**File**: `skills/review/lenses/test-coverage-lens/SKILL.md`

Currently has flat questions covering coverage, regression, edge cases,
assertions, pyramid, isolation, mocks, and risk proportionality.

**Proposed grouping**:

- **Coverage adequacy** (always applicable): code path coverage, regression
  tests, edge cases, risk proportionality
- **Test quality and assertions** (always applicable): assertion specificity,
  mutation testing lens, implementation coupling
- **Test architecture** (when test infrastructure or patterns are involved):
  pyramid balance, isolation, mock strategy

Present to user for discussion.

#### Usability Lens — Light Improvement

**File**: `skills/review/lenses/usability-lens/SKILL.md`

Currently has flat questions covering consistency, minimality, discoverability,
composability, least surprise, errors, configuration, and migration.

**Proposed grouping**:

- **API ergonomics** (always applicable): consistency, minimality,
  discoverability, composability, least surprise, error messages, defaults
- **Migration and compatibility** (when breaking changes or version transitions
  are present): breaking changes, migration paths, deprecation, configuration
  complexity

Present to user for discussion.

### Interactive Process

For each lens that needs changes:

1. Present the current flat question list
2. Show proposed conditional grouping with applicability conditions
3. Ask: "Does this grouping make sense? Should any questions move between
   groups? Should any groups be merged or split?"
4. Apply agreed changes
5. Move to the next lens

Process in order: security (confirm no changes), standards (confirm no changes),
architecture, test-coverage, usability, performance, code-quality.

### Success Criteria

#### Automated Verification

- Lenses with sub-groups use bold sub-headings with parenthetical applicability
  conditions, following the `**Category name** (condition):` format
- Every lens has at least one sub-group marked "(always applicable)"
- All applicability conditions describe observable code/plan characteristics,
  not intent
- All original evaluation questions are preserved (regrouped, not removed)
- No changes to Core Responsibilities, Important Guidelines, What NOT to Do, or
  Remember statements

---

## Testing Strategy

### Phase 0: Baseline Capture (Before Any Changes)

Before starting Phase 1, capture baseline review outputs:

1. Choose one representative PR and one representative plan as test artefacts
2. Run `/review-pr` on the chosen PR and save the full output
3. Run `/review-plan` on the chosen plan and save the full output
4. For each lens, document the categories of defects its current questions are
   intended to detect (e.g., code quality: cyclomatic complexity, naming,
   nesting depth, code smells, design principles, error handling, observability,
   testability)

Save these baselines for comparison after all phases complete.

### Per-Phase Validation Checkpoints

After each phase, run a lightweight validation before proceeding:

1. **After Phase 1**: Run `/review-pr` on the baseline PR with all 7 lenses.
   Spot-check that outputs are coherent and preambles are reflected in tone.
2. **After Phase 2**: Run `/review-pr` on the baseline PR with all 7 lenses.
   Verify that findings still cover the defect categories documented in Phase 0.
3. **After Phase 3**: Run `/review-pr` on the baseline PR with all 7 lenses.
   Verify that conditional groups reduce irrelevant findings without creating
   gaps.

If a checkpoint reveals a regression, address it before proceeding to the next
phase.

### Final Verification

After all phases:

1. Read each of the 7 lens files and verify they follow the optimal structure
2. Run `/review-pr` on the baseline PR and compare against Phase 0 output:
   - **Actionable findings**: count should be comparable or higher
   - **Scenario specificity**: findings should reference specific failure
     scenarios, attack vectors, or mutation cases rather than generic concerns
   - **Defect category coverage**: all categories documented in Phase 0 should
     still be represented in findings
   - **Irrelevance reduction**: fewer findings about sub-domains not applicable
     to the PR under review
3. Run `/review-plan` on the baseline plan and apply the same comparison
   criteria

## References

- Research: `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md`
- Gap analysis: `meta/research/codebase/2026-02-22-review-lens-gap-analysis.md`
- Performance lens plan:
  `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md`
- PR review orchestrator: `skills/git/review-pr/SKILL.md`
- Plan review orchestrator: `skills/planning/review-plan/SKILL.md`
- Generic reviewer agent: `agents/reviewer.md`
