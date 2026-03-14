# Review-Plan Alignment with Review-PR Patterns

## Overview

Upgrade the `review-plan` command and its agents to be consistent with the
newer `review-pr` patterns: structured JSON agent output, emoji severity
prefixes, structured aggregation pipeline, and malformed output handling. Also
add a `plan-test-coverage-reviewer` agent (bringing plan review to 6 lenses)
and adjust `plan-code-quality-reviewer` to remove the testing strategy overlap.

## Current State Analysis

The `review-plan` command (5 lenses) and `review-pr` command (6 lenses) share
the same multi-lens architecture but diverge in agent output format (markdown
vs JSON), command synthesis (loose narrative vs structured pipeline), and lens
coverage (no test coverage for plans).

### Key Discoveries:

- All 5 plan agents output free-form markdown; all 6 PR agents output JSON
- `plan-code-quality-reviewer` currently owns testing strategy (Core
  Responsibility #2, Analysis Step 4, Output Format section) which should move
  to the new test coverage agent
- The plan command lacks malformed output handling, emoji prefixes, explicit
  deduplication rules, and verdict logic
- The collaborative iteration workflow (Steps 6-7) is unique to plan review
  and should be preserved

## Desired End State

After this plan is complete:

1. All 6 plan agents output structured JSON with `lens`, `summary`,
   `strengths`, and `findings` fields
2. A new `plan-test-coverage-reviewer` agent reviews testing strategy in plans
3. `plan-code-quality-reviewer` no longer overlaps with test coverage
4. The `review-plan` command has a structured aggregation pipeline, emoji
   prefixes, malformed output handling, and verdict logic
5. The collaborative plan iteration workflow (Steps 6-7) is preserved

**Verification**: Run `/review-plan` against a real plan and confirm:
- All 6 agents return valid JSON
- The command aggregates, deduplicates, and presents findings with emojis
- The verdict reflects finding severity
- Collaborative iteration still works after the review presentation

## What We're NOT Doing

- Adding `path`/`line`/`side`/`end_line` fields to plan agent findings — plans
  have no diff hunks. We use a `location` field instead.
- Splitting plan findings into `comments` vs `general_findings` — a single
  `findings` array suffices since there's no inline-vs-summary distinction
- Adding an "Anchor Findings to Diff Locations" step — plan agents reference
  plan sections, not diff lines
- Adding GitHub API posting — plan reviews are consumed in-conversation
- Adding Multi-Line Comment API Mapping — PR-specific concern
- Changing plan-specific Core Responsibilities or Analysis Steps — they should
  remain focused on proposed design, not actual code
- Using `REQUEST_CHANGES` as a verdict — we use `REVISE` instead because plan
  reviews result in plan edits rather than GitHub change requests. We also
  escalate to `REVISE` on 3+ major findings (the PR command only escalates on
  critical) because plan edits are low-cost and multiple major concerns
  collectively indicate a plan that needs rework before implementation.
- Preserving lens-specific Output Format sub-assessment sections (e.g., Threat
  Model, OWASP Coverage, Convention Consistency, Design Principle Assessment) —
  these are replaced by the generic JSON schema for aggregation consistency. The
  analytical depth is preserved via the Core Responsibilities and Analysis Steps
  (which remain unchanged) and agents should reflect these dimensions in their
  `summary` field.

## Implementation Approach

Four phases, executed sequentially:

1. **Phase 1**: Create the new `plan-test-coverage-reviewer` agent
2. **Phase 2**: Update `plan-code-quality-reviewer` to remove testing strategy
   overlap and convert to JSON output
3. **Phase 3**: Convert the remaining 4 plan agents to JSON output
4. **Phase 4**: Update the `review-plan` command to consume JSON, add Test
   Coverage lens, and align orchestration patterns

---

## Phase 1: Create `plan-test-coverage-reviewer` Agent

### Overview

Create a new agent that reviews implementation plans through a testing strategy
lens — evaluating whether the plan includes adequate testing provisions, an
appropriate test pyramid, edge case identification, and a sound test
architecture approach.

**Note**: The alignment research initially concluded "Test Coverage makes sense
as PR-only" because it focused on reviewing actual test *code* in PRs. However,
evaluating testing *strategy* in plans is a distinct and valuable lens: it
catches missing test provisions, inadequate edge case coverage, and poor test
architecture *before* code is written, when changes are cheapest. This is
analogous to how the security lens reviews security *design* in plans rather
than security *code* in PRs.

### Changes Required

#### 1. New Agent File

**File**: `agents/plan-test-coverage-reviewer.md`

Create this file with the following complete content:

````markdown
---
name: plan-test-coverage-reviewer
description: Reviews implementation plans through a test coverage and quality lens. Use this agent when you want to evaluate whether a plan includes adequate testing provisions — test pyramid balance, edge case identification, test isolation strategy, and coverage proportional to risk.
tools: Read, Grep, Glob, LS
---

You are a specialist at evaluating implementation plans from a test coverage
and quality perspective. Your job is to assess whether the plan includes
adequate, well-structured testing provisions — test pyramid balance, edge case
identification, test isolation strategy, mock approach, and coverage
proportional to the risk profile of the changes.

## Core Responsibilities

1. **Evaluate Testing Strategy Adequacy**

- Check that the plan includes a testing strategy for new functionality
- Verify bug fixes include regression test provisions
- Assess edge case and boundary condition identification
- Check that error paths and failure scenarios are covered in the test plan
- Verify critical business logic has thorough unit test provisions

2. **Assess Test Quality Planning**

- Verify the plan specifies behaviour-oriented testing (not implementation)
- Check that the plan considers assertion quality and specificity
- Apply the mutation testing lens — would the planned tests catch subtle bugs?
- Identify areas where the testing approach risks assertion-free tests or
  implementation coupling

3. **Review Test Architecture Planning**

- Assess test pyramid balance — unit foundation, integration for boundaries,
  minimal E2E
- Check test isolation strategy — determinism, no shared mutable state
- Evaluate mock/stub strategy — are mocks planned for true boundaries only?
- Verify test code is treated as first-class in the plan's design

**Boundary note**: Testability as a *code design property* (e.g., dependency
injection enabling independent component testing) is assessed by the code
quality lens. This lens focuses on the *testing strategy and coverage
provisions* in the plan — what will be tested, how, and whether coverage is
proportional to risk.

## Analysis Strategy

### Step 1: Read and Understand the Plan

- Read the implementation plan file fully
- Identify the scope and complexity of the proposed changes
- Note any existing testing strategy or success criteria sections
- Determine the risk profile of the changes

### Step 2: Explore Existing Test Patterns

- Use Grep and Glob to find existing test files relevant to the planned changes
- Read test conventions: naming, structure, frameworks, helper patterns
- Identify the project's testing approach and tools
- Take time to ultrathink about whether the plan's testing provisions follow
  or improve upon established testing patterns

### Step 3: Evaluate Testing Strategy Completeness

- For each planned feature or change, check if testing provisions exist
- For bug fixes, check for regression test provisions
- Identify untested scenarios, edge cases, and error conditions
- Assess whether testing coverage is proportional to the risk profile

### Step 4: Assess Test Quality Approach

- Does the plan specify testing behaviour rather than implementation details?
- Are assertions likely to be specific and meaningful?
- Apply the mutation testing lens: would the planned tests catch operator
  changes in critical code paths?
- Identify risks of assertion-free tests or implementation-coupled tests

### Step 5: Review Test Architecture Planning

- Assess pyramid level appropriateness for each type of test planned
- Check for determinism considerations in the testing approach
- Evaluate mock strategy — are mocks planned only at true boundaries?
- Check that the testing strategy is practical for CI/CD

## Output Format

Return your analysis as a JSON code block. Do not include any text before or
after the JSON block — the orchestrator will parse this output directly.

```json
{
  "lens": "test-coverage",
  "summary": "2-3 sentence Test Coverage assessment of the plan.",
  "strengths": [
    "Positive observation about what the plan gets right from a test coverage perspective"
  ],
  "findings": [
    {
      "severity": "critical",
      "confidence": "high",
      "lens": "test-coverage",
      "location": "Phase 2: API Endpoints — Success Criteria",
      "title": "Brief finding title",
      "body": "🔴 **Test Coverage**\n\n[Issue description — 1-2 sentences with enough context to understand standalone].\n\n**Impact**: [Why this matters — 1 sentence].\n\n**Suggestion**: [Concrete fix — 1-2 sentences]."
    }
  ]
}
```

### Field Reference

- **lens**: Agent lens identifier — `"test-coverage"`
- **summary**: 2-3 sentence assessment from the test coverage perspective.
  Reflect the key dimensions from your Core Responsibilities — testing
  strategy adequacy, test quality approach, and test architecture planning.
  This is where holistic assessment lives, beyond individual findings.
- **strengths**: Positive observations (fed into the review summary — never
  posted as individual findings)
- **findings**: All findings, each referencing a location in the plan
  - **severity**: One of `"critical"`, `"major"`, `"minor"`, `"suggestion"`
  - **confidence**: One of `"high"`, `"medium"`, `"low"`
  - **lens**: The lens identifier (same value as the top-level `lens` field).
    Included on each finding so the orchestrator can attribute findings after
    merging outputs from multiple agents.
  - **location**: Human-readable reference to the plan section where the
    finding is most relevant (e.g., "Phase 2: API Endpoints",
    "Testing Strategy section", "Success Criteria for Phase 1")
  - **title**: Brief title for the finding (used in the summary index)
  - **body**: Self-contained finding body. See "Finding Body Format" below.

### Severity Emoji Prefixes

Use these at the start of each finding `body`:
- `🔴` for `"critical"` severity
- `🟡` for `"major"` severity
- `🔵` for `"minor"` and `"suggestion"` severity

### Finding Body Format

Each finding `body` should follow this structure:

```
[emoji] **[Lens Name]**

[Issue description — 1-2 sentences, standalone context].

**Impact**: [Why this matters].

**Suggestion**: [Concrete fix].
```

Example:

```
🟡 **Test Coverage**

The plan proposes integration tests for the new API endpoints but includes no
unit tests for the validation logic. The validation rules are complex with
multiple edge cases.

**Impact**: Without unit tests, subtle validation bugs will only surface in
slower integration tests, or not at all.

**Suggestion**: Add unit test provisions for the validation module, covering
boundary conditions and invalid input combinations.
```

## Important Guidelines

- **Always read the plan fully** before assessing test coverage
- **Explore the codebase** to understand existing test patterns and conventions
- **Apply the mutation testing lens** — mentally ask "if I changed this
  operator, would the planned tests catch it?" for critical code paths
- **Be pragmatic** — focus on missing coverage that represents real risk, not
  100% coverage dogma
- **Rate confidence** on each finding — distinguish definite gaps from
  potential concerns
- **Evaluate proportionally** — a trivial utility doesn't need the same test
  rigour as a payment processor
- **Consider test maintainability** — overly complex test plans are a liability
- **Make each finding body self-contained** — it will be presented alongside
  findings from other lenses without surrounding context
- **Use severity emoji prefixes** — start each finding body with 🔴 (critical),
  🟡 (major), or 🔵 (minor/suggestion) followed by the lens name in bold
- **Output only the JSON block** — do not include additional prose, narrative
  analysis, or markdown outside the JSON code fence. The orchestrator parses
  your output as JSON.

## What NOT to Do

- Don't review architecture, security, code quality, standards, or usability
  — those are other lenses
- Don't insist on 100% coverage — focus on coverage that provides meaningful
  confidence
- Don't penalise test approaches that differ from your preference if they are
  effective
- Don't flag test style issues that don't affect test reliability or
  maintainability
- Don't ignore the existing codebase's testing patterns when evaluating the plan

Remember: You're evaluating whether the plan's testing provisions will give
genuine confidence that the implemented code works correctly. The best testing
strategy catches real bugs, survives refactoring, and runs reliably in CI.
````

### Success Criteria

#### Automated Verification:

- [x] `agents/plan-test-coverage-reviewer.md` exists and parses as valid markdown
- [x] The file contains a JSON code block in its Output Format section
- [x] The file contains `"lens": "test-coverage"` in the JSON schema

#### Manual Verification:

- [x] The agent's Core Responsibilities cover testing strategy, test quality
  planning, and test architecture — not actual test code review
- [x] The Output Format uses the plan-adapted JSON schema (with `location`
  field, single `findings` array, no `path`/`line`/`side`)

---

## Phase 2: Update `plan-code-quality-reviewer` Agent

### Overview

Remove testing strategy content from the code quality agent (now handled by the
new test coverage agent) and convert the output format to structured JSON.

### Changes Required

**File**: `agents/plan-code-quality-reviewer.md`

#### 1. Update Frontmatter Description

**Location**: Replace the `description` field in the YAML frontmatter.

**Current**:
```markdown
description: Reviews implementation plans through a code quality lens. Use this agent when you want to evaluate whether a plan sets up for clean, maintainable, testable code — checking design principles, error handling, observability, testing strategy, and complexity management. Pragmatic, not pedantic.
```

**Replace with**:
```markdown
description: Reviews implementation plans through a code quality lens. Use this agent when you want to evaluate whether a plan sets up for clean, maintainable, testable code — checking design principles, error handling, observability, testability, and complexity management. Pragmatic, not pedantic.
```

#### 2. Update Intro Paragraph

**Location**: Replace "testing approach" in the intro paragraph (line 11 of
the current file).

**Current**:
```markdown
Your job is to assess whether the plan sets up for clean,
maintainable, testable, and readable code — checking adherence to design
principles, error handling strategy, observability, testing approach, and
complexity management.
```

**Replace with**:
```markdown
Your job is to assess whether the plan sets up for clean,
maintainable, testable, and readable code — checking adherence to design
principles, error handling strategy, observability, testability, and
complexity management.
```

#### 3. Update Core Responsibility #2

**Location**: Replace Core Responsibility #2 entirely.

**Current**:
```markdown
2. **Assess Testability and Testing Strategy**

- Evaluate whether components are designed for testability (dependency injection,
  interface abstractions)
- Check test pyramid balance (unit, integration, end-to-end proportions)
- Assess edge case identification and coverage strategy
- Review mock/stub strategy and test isolation approach
```

**Replace with**:
```markdown
2. **Assess Testability and Maintainability**

- Evaluate whether components are designed for testability (dependency injection,
  interface abstractions)
- Assess whether designs support independent component testing
- Check that complexity is proportional to requirements
- Verify the plan considers long-term maintainability

Note: Testability as a *code design property* is assessed here. The specific
testing strategy and coverage plan (test pyramid, edge cases, mock strategy)
is assessed by the test-coverage lens.
```

#### 4. Replace Analysis Step 4

**Location**: Replace Step 4 entirely.

**Current**:
```markdown
### Step 4: Evaluate Testing Strategy in Detail

- Assess test pyramid balance — are unit tests the foundation?
- Check whether the plan identifies key edge cases
- Evaluate test isolation — can components be tested independently?
- Review mock strategy — are mocks used at boundaries, not everywhere?
- Verify integration test coverage for component interactions
- Check that the testing strategy matches the risk profile of the changes
```

**Replace with**:
```markdown
### Step 4: Assess Maintainability and Testability

- Will the proposed design be maintainable by the next developer?
- Are components designed for independent testability?
- Is complexity proportional to the requirements?
- Are there opportunities for simplification without losing functionality?
```

#### 5. Replace Output Format Section

**Location**: Replace the entire "## Output Format" section (from the heading
through the closing ``` of the markdown code block).

**Replace with**:

````markdown
## Output Format

Return your analysis as a JSON code block. Do not include any text before or
after the JSON block — the orchestrator will parse this output directly.

```json
{
  "lens": "code-quality",
  "summary": "2-3 sentence Code Quality assessment of the plan.",
  "strengths": [
    "Positive observation about what the plan gets right from a code quality perspective"
  ],
  "findings": [
    {
      "severity": "critical",
      "confidence": "high",
      "lens": "code-quality",
      "location": "Phase 1: Data Model — Component Design",
      "title": "Brief finding title",
      "body": "🔴 **Code Quality**\n\n[Issue description — 1-2 sentences with enough context to understand standalone].\n\n**Impact**: [Why this matters — 1 sentence].\n\n**Suggestion**: [Concrete fix — 1-2 sentences]."
    }
  ]
}
```

### Field Reference

- **lens**: Agent lens identifier — `"code-quality"`
- **summary**: 2-3 sentence assessment from the code quality perspective.
  Reflect the key dimensions from your Core Responsibilities — design
  principle adherence, testability and maintainability, error handling, and
  codebase consistency. This is where holistic assessment lives, beyond
  individual findings.
- **strengths**: Positive observations (fed into the review summary — never
  posted as individual findings)
- **findings**: All findings, each referencing a location in the plan
  - **severity**: One of `"critical"`, `"major"`, `"minor"`, `"suggestion"`
  - **confidence**: One of `"high"`, `"medium"`, `"low"`
  - **lens**: The lens identifier (same value as the top-level `lens` field).
    Included on each finding so the orchestrator can attribute findings after
    merging outputs from multiple agents.
  - **location**: Human-readable reference to the plan section where the
    finding is most relevant (e.g., "Phase 2: API Endpoints",
    "Implementation Approach", "Phase 1: Data Model")
  - **title**: Brief title for the finding (used in the summary index)
  - **body**: Self-contained finding body. See "Finding Body Format" below.

### Severity Emoji Prefixes

Use these at the start of each finding `body`:
- `🔴` for `"critical"` severity
- `🟡` for `"major"` severity
- `🔵` for `"minor"` and `"suggestion"` severity

### Finding Body Format

Each finding `body` should follow this structure:

```
[emoji] **[Lens Name]**

[Issue description — 1-2 sentences, standalone context].

**Impact**: [Why this matters].

**Suggestion**: [Concrete fix].
```

Example:

```
🟡 **Code Quality**

The proposed handler function combines request validation, business logic, and
database access in a single method. This creates high cyclomatic complexity and
makes unit testing difficult.

**Impact**: The combined responsibilities will make the handler hard to test and
maintain as requirements evolve.

**Suggestion**: Separate validation, business logic, and data access into
distinct functions that can be tested independently.
```
````

#### 6. Update Important Guidelines

**Location**: Add these bullets after the existing guidelines:

```markdown
- **Make each finding body self-contained** — it will be presented alongside
  findings from other lenses without surrounding context
- **Use severity emoji prefixes** — start each finding body with 🔴 (critical),
  🟡 (major), or 🔵 (minor/suggestion) followed by the lens name in bold
- **Output only the JSON block** — do not include additional prose, narrative
  analysis, or markdown outside the JSON code fence. The orchestrator parses
  your output as JSON.
```

#### 7. Update What NOT to Do

**Location**: Update the first bullet to include test coverage in the list of
other lenses, and add the JSON output item.

**Current first bullet**:
```markdown
- Don't review architecture, security, or usability — those are other lenses
```

**Replace with**:
```markdown
- Don't review architecture, security, test coverage, standards, or usability
  — those are other lenses
```

**Add after the last existing bullet**:
```markdown
- Don't assess the testing strategy (test pyramid, edge cases, mock strategy)
  — that is the test coverage lens
```

### Success Criteria

#### Automated Verification:

- [x] `agents/plan-code-quality-reviewer.md` contains `"lens": "code-quality"`
  in a JSON code block
- [x] The file does NOT contain "test pyramid" or "mock strategy"
  (Note: these terms appear only in the boundary cross-reference note directing
  readers to the test-coverage lens, not as owned analysis)
- [x] The file does NOT contain "Evaluate Testing Strategy in Detail"
- [x] The frontmatter description contains "testability" not "testing strategy"
- [x] The intro paragraph contains "testability" not "testing approach"

#### Manual Verification:

- [x] Core Responsibility #2 focuses on testability-as-design-quality, not
  testing strategy
- [x] Core Responsibility #2 includes boundary note distinguishing from test
  coverage lens
- [x] Step 4 focuses on maintainability and testability, not test pyramid or
  mock strategy

---

## Phase 3: Update Remaining 4 Plan Agents to JSON Output

### Overview

Convert `plan-architecture-reviewer`, `plan-security-reviewer`,
`plan-standards-reviewer`, and `plan-usability-reviewer` to the same JSON
output format.

### Changes Required

The following changes apply to all 4 agent files:

- `agents/plan-architecture-reviewer.md`
- `agents/plan-security-reviewer.md`
- `agents/plan-standards-reviewer.md`
- `agents/plan-usability-reviewer.md`

#### 1. Replace Output Format Section

**Location**: Replace the entire "## Output Format" section (from the heading
through the closing ``` of the markdown code block) in each file.

**Replace with** (substituting `[Lens Name]` and `[lens-name]` per the table
below):

````markdown
## Output Format

Return your analysis as a JSON code block. Do not include any text before or
after the JSON block — the orchestrator will parse this output directly.

```json
{
  "lens": "[lens-name]",
  "summary": "2-3 sentence [Lens Name] assessment of the plan.",
  "strengths": [
    "Positive observation about what the plan gets right from a [lens] perspective"
  ],
  "findings": [
    {
      "severity": "critical",
      "confidence": "high",
      "lens": "[lens-name]",
      "location": "Phase 2, Section 3: Database Migration",
      "title": "Brief finding title",
      "body": "🔴 **[Lens Name]**\n\n[Issue description — 1-2 sentences with enough context to understand standalone].\n\n**Impact**: [Why this matters — 1 sentence].\n\n**Suggestion**: [Concrete fix — 1-2 sentences]."
    }
  ]
}
```

### Field Reference

- **lens**: Agent lens identifier — `"[lens-name]"`
- **summary**: 2-3 sentence assessment from the [lens] perspective. Reflect
  the key dimensions from your Core Responsibilities — e.g., for architecture:
  modularity, coupling, evolutionary fitness; for security: STRIDE coverage,
  OWASP alignment, supply chain risk; for standards: convention consistency,
  API standards alignment, documentation provisions; for usability: API
  ergonomics, configuration complexity, migration paths.
  This is where holistic assessment lives, beyond individual findings.
- **strengths**: Positive observations (fed into the review summary — never
  posted as individual findings)
- **findings**: All findings, each referencing a location in the plan
  - **severity**: One of `"critical"`, `"major"`, `"minor"`, `"suggestion"`
  - **confidence**: One of `"high"`, `"medium"`, `"low"`
  - **lens**: The lens identifier (same value as the top-level `lens` field).
    Included on each finding so the orchestrator can attribute findings after
    merging outputs from multiple agents.
  - **location**: Human-readable reference to the plan section where the
    finding is most relevant (e.g., "Phase 2: API Endpoints",
    "Implementation Approach", "Phase 1: Data Model")
  - **title**: Brief title for the finding (used in the summary index)
  - **body**: Self-contained finding body. See "Finding Body Format" below.

### Severity Emoji Prefixes

Use these at the start of each finding `body`:
- `🔴` for `"critical"` severity
- `🟡` for `"major"` severity
- `🔵` for `"minor"` and `"suggestion"` severity

### Finding Body Format

Each finding `body` should follow this structure:

```
[emoji] **[Lens Name]**

[Issue description — 1-2 sentences, standalone context].

**Impact**: [Why this matters].

**Suggestion**: [Concrete fix].
```
````

Each agent's `[Lens Name]` and `[lens-name]` values, plus a lens-specific
example to append after the Finding Body Format section:

| Agent File | `[Lens Name]` | `[lens-name]` |
|---|---|---|
| `plan-architecture-reviewer.md` | Architecture | architecture |
| `plan-security-reviewer.md` | Security | security |
| `plan-standards-reviewer.md` | Standards | standards |
| `plan-usability-reviewer.md` | Usability | usability |

**Lens-specific examples** (append after the Finding Body Format section in
each agent):

**Architecture** (insert after the Finding Body Format section in
`plan-architecture-reviewer.md`):

````markdown
Example:

```
🔴 **Architecture**

The plan proposes a direct dependency from the API layer to the database schema
with no service abstraction. This couples the presentation layer to the data
model.

**Impact**: Database schema changes will ripple into the API layer, breaking
the dependency rule.

**Suggestion**: Introduce a service layer to mediate between the API handlers
and the data access layer.
```
````

**Security** (insert after the Finding Body Format section in
`plan-security-reviewer.md`):

````markdown
Example:

```
🔴 **Security**

The plan proposes storing API tokens in a configuration file without encryption.
No rotation strategy is mentioned.

**Impact**: Token compromise would grant persistent access with no revocation
mechanism.

**Suggestion**: Store tokens in a secrets manager (e.g., AWS Secrets Manager)
and include a rotation strategy in the plan.
```
````

**Standards** (insert after the Finding Body Format section in
`plan-standards-reviewer.md`):

````markdown
Example:

```
🔵 **Standards**

The proposed API endpoint uses `POST /api/users/search` which deviates from the
established convention of using query parameters on `GET /api/users`.

**Impact**: Inconsistent API design increases cognitive load for consumers.

**Suggestion**: Use `GET /api/users?query=...` to match existing search
patterns in the codebase.
```
````

**Usability** (insert after the Finding Body Format section in
`plan-usability-reviewer.md`):

````markdown
Example:

```
🟡 **Usability**

The plan requires 12 configuration fields with no defaults. Most deployments
will use the same values for 10 of these fields.

**Impact**: Unnecessary configuration burden for the common case; risk of
misconfiguration.

**Suggestion**: Provide sensible defaults for the 10 common fields and require
only the 2 deployment-specific values.
```
````

#### 2. Update Important Guidelines

**Location**: In each agent's "## Important Guidelines" section, add these
bullets after the existing guidelines:

```markdown
- **Make each finding body self-contained** — it will be presented alongside
  findings from other lenses without surrounding context
- **Use severity emoji prefixes** — start each finding body with 🔴 (critical),
  🟡 (major), or 🔵 (minor/suggestion) followed by the lens name in bold
- **Output only the JSON block** — do not include additional prose, narrative
  analysis, or markdown outside the JSON code fence. The orchestrator parses
  your output as JSON.
```

#### 3. Update What NOT to Do

**Location**: In each agent's "## What NOT to Do" section, update the first
bullet to include "test coverage" in the list of other lenses (if not already
present).

Replace the first bullet in each agent's What NOT to Do section with the
exact text below (each agent excludes the other 5 lenses):

**`plan-architecture-reviewer.md`**:
```markdown
- Don't review security, test coverage, code quality, standards, or usability
  — those are other lenses
```

**`plan-security-reviewer.md`**:
```markdown
- Don't review architecture, test coverage, code quality, standards, or
  usability — those are other lenses
```

**`plan-standards-reviewer.md`**:
```markdown
- Don't review architecture, security, test coverage, code quality, or
  usability — those are other lenses
```

**`plan-usability-reviewer.md`**:
```markdown
- Don't review architecture, security, test coverage, code quality, or
  standards — those are other lenses
```

### Agent-Specific Notes

Beyond the shared template above, each agent needs its lens-specific values
substituted. No other agent-specific structural changes are needed — the Core
Responsibilities, Analysis Strategy steps, closing "Remember:" paragraph, and
YAML frontmatter `description` fields all remain unchanged for each of these
4 agents. (The code quality agent's frontmatter is updated separately in
Phase 2.)

### Success Criteria

#### Automated Verification:

- [x] All 4 agent files contain a JSON code block in their Output Format section
- [x] Each agent file contains the correct `"lens"` value in the JSON schema

#### Manual Verification:

- [x] Spot-check 2 agent files to confirm the lens name is correctly
  substituted in the Output Format section
- [x] Confirm each agent's Core Responsibilities and Analysis Steps are
  unchanged

---

## Phase 4: Update `review-plan` Command

### Overview

Update the `review-plan` command to add the Test Coverage lens, consume the new
JSON agent output, and align the orchestration pipeline with the `review-pr`
patterns — while preserving the collaborative iteration workflow.

### Changes Required

**File**: `commands/review-plan.md`

#### 1. Update Available Review Lenses Table

**Location**: Replace the lens table in "## Available Review Lenses".

**Current**:
```markdown
| Lens             | Agent                        | Focus                                                                 |
|------------------|------------------------------|-----------------------------------------------------------------------|
| **Architecture** | `plan-architecture-reviewer` | Modularity, coupling, scalability, evolutionary design, tradeoffs     |
| **Code Quality** | `plan-code-quality-reviewer` | Design principles, testability, error handling, complexity management |
| **Security**     | `plan-security-reviewer`     | Threats, missing protections, STRIDE analysis, OWASP coverage         |
| **Standards**    | `plan-standards-reviewer`    | Project conventions, API standards, accessibility, documentation      |
| **Usability**    | `plan-usability-reviewer`    | Developer experience, API ergonomics, configuration, migration paths  |
```

**Replace with**:
```markdown
| Lens               | Agent                          | Focus                                                                 |
|--------------------|--------------------------------|-----------------------------------------------------------------------|
| **Architecture**   | `plan-architecture-reviewer`   | Modularity, coupling, scalability, evolutionary design, tradeoffs     |
| **Security**       | `plan-security-reviewer`       | Threats, missing protections, STRIDE analysis, OWASP coverage         |
| **Test Coverage**  | `plan-test-coverage-reviewer`  | Testing strategy, test pyramid, edge cases, isolation, risk coverage  |
| **Code Quality**   | `plan-code-quality-reviewer`   | Design principles, testability, error handling, complexity management |
| **Standards**      | `plan-standards-reviewer`      | Project conventions, API standards, accessibility, documentation      |
| **Usability**      | `plan-usability-reviewer`      | Developer experience, API ergonomics, configuration, migration paths  |
```

#### 2. Update Lens Selection Criteria

**Location**: In "### Step 2: Select Review Lenses", add Test Coverage to the
auto-detect criteria after the existing Code Quality entry.

**Add**:
```markdown
- **Test Coverage** — relevant for most plans; skip only for documentation-only,
  configuration-only, or infrastructure-only changes with no code
```

**Update the lens selection presentation** to include Test Coverage:
```markdown
```
Based on the plan's scope, I'll review through these lenses:
- Architecture: [reason]
- Security: [reason — or "Skipping: no security-sensitive changes identified"]
- Test Coverage: [reason]
- Code Quality: [reason]
- Standards: [reason — or "Skipping: ..."]
- Usability: [reason — or "Skipping: ..."]

Shall I proceed, or would you like to adjust the selection?
```
```

#### 3. Replace Step 3: Spawn Review Agents

**Location**: Replace the entire "### Step 3: Spawn Review Agents" section.

**Replace with**:

````markdown
### Step 3: Spawn Review Agents

Spawn all selected review agents **in parallel** using the Task tool. Each
agent receives:

- The full path to the plan file
- Instructions to read the plan fully and explore the codebase for context
- Instructions to return structured JSON output (not prose)

Example spawn pattern:

```
Task 1 (plan-architecture-reviewer):
  "Review the implementation plan at [path]. Read it fully, explore the
  codebase for architectural context.

  IMPORTANT: Return your analysis as a single JSON code block following your
  Output Format specification. Do not include prose outside the JSON block."

Task 2 (plan-security-reviewer):
  "Review the implementation plan at [path]. Read it fully, explore the
  codebase for security context.

  IMPORTANT: Return your analysis as a single JSON code block following your
  Output Format specification. Do not include prose outside the JSON block."

[Same pattern for all selected agents]
```

**IMPORTANT**: Wait for ALL review agents to complete before proceeding.

**Handling malformed agent output**:

If an agent's response is not a clean JSON block, apply this extraction
strategy:

1. Look for a JSON code block fenced with triple backticks (optionally with
   a `json` language tag)
2. If found, extract and parse the content within the fences
3. If the extracted JSON is valid, use it normally
4. If no JSON code block is found, or the JSON within it is invalid, apply
   the fallback: treat the agent's entire output as a single finding with
   the agent's lens name and `"major"` severity, and include it in the
   review summary

When falling back, warn the user that the agent's output could not be parsed
and present the raw agent output in a collapsed form so the user can see what
the agent actually found.
````

#### 4. Replace Step 4: Compile and Synthesise Findings

**Location**: Replace the entire "### Step 4: Compile and Synthesise Findings"
section.

**Replace with**:

````markdown
### Step 4: Aggregate and Curate Findings

Once all reviews are complete:

1. **Parse agent outputs**: Extract the JSON block from each agent's response
   (see the extraction strategy in Step 3). Collect the `summary`, `strengths`,
   and `findings` arrays from each.

2. **Aggregate across agents**:
   - Combine all `findings` arrays into a single list
   - Combine all `strengths` arrays into a single list
   - Collect all `summary` strings

3. **Deduplicate findings**: Where multiple agents flag overlapping plan
   sections with similar concerns, consider merging — but only when the
   findings address the same underlying concern from different lens
   perspectives. Location proximity alone is not sufficient; the findings must
   be semantically related.

   When merging:
   - Combine the bodies, attributing each part to its lens
   - Use the highest severity among the merged findings
   - Use the highest confidence among the merged findings
   - Note all contributing lenses in the title

   When in doubt, keep findings separate — distinct findings are easier to
   address individually than a merged finding covering multiple concerns.

4. **Prioritise findings**:
   - Sort by severity: critical > major > minor > suggestion
   - Within the same severity, sort by confidence: high > medium > low

5. **Determine suggested verdict**:
   - If any `"critical"` severity findings exist → suggest `REVISE`
   - If 3 or more `"major"` severity findings exist → suggest `REVISE`
   - If 1-2 `"major"` findings or only minor/suggestion → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`

   Verdict meanings:
   - `APPROVE` — plan is sound and ready for implementation
   - `REVISE` — plan needs changes before implementation
   - `COMMENT` — observations only, plan is acceptable as-is

   When presenting a `COMMENT` verdict with major findings, note: "Plan is
   acceptable but could be improved — see major findings below."

6. **Identify cross-cutting themes**: Look for findings that appear across
   multiple lenses — issues flagged by 2+ agents reinforce each other and
   should be highlighted in the summary. Also identify tradeoffs where
   different lenses conflict (e.g., security wants more validation, usability
   wants less friction).

7. **Compose the review summary**:

   ```markdown
   ## Plan Review: [Plan Name]

   **Verdict:** [APPROVE | REVISE | COMMENT]

   [Combined assessment: take each agent's summary and synthesise into 2-3
   sentences covering the overall quality of the plan across all lenses]

   ### Cross-Cutting Themes
   [Issues that multiple lenses identified — these deserve the most attention]
   - **[Theme]** (flagged by: [lenses]) — [description]

   ### Tradeoff Analysis
   [Where different lenses disagree, present both perspectives]
   - **[Quality A] vs [Quality B]**: [description and recommendation]

   [Omit either section if there are no cross-cutting themes or tradeoffs]

   ### Findings

   #### Critical
   - 🔴 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   #### Major
   - 🟡 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   #### Minor
   - 🔵 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   #### Suggestions
   - 🔵 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   ### Strengths
   - ✅ [Aggregated and deduplicated strengths from all agents]

   ### Recommended Changes
   [Ordered list of specific, actionable changes to the plan, prioritised by
   impact. Each should reference the finding(s) it addresses.]

   1. **[Change description]** (addresses: [finding titles])
      [Specific guidance on what to modify in the plan]

   ---
   *Review generated by /review-plan*
   ```
````

#### 5. Replace Step 5: Present the Review

**Location**: Replace the entire "### Step 5: Present the Review" section.

**Replace with**:

````markdown
### Step 5: Present the Review

Present the composed review summary from Step 4.7 to the user.

After presenting, offer the user control before proceeding to iteration:

```
The review is complete. Verdict: [verdict]

Would you like to:
1. Proceed to address findings? (I'll help edit the plan)
2. Change the verdict? (currently: [verdict])
3. Discuss any specific findings in more detail?
4. Re-run specific lenses with adjusted focus?
```
````

#### 6. Update Steps 6-7: Preserve Collaborative Iteration

**Location**: Steps 6 (Collaborative Plan Iteration) and 7 (Offer Re-Review)
remain largely unchanged. Make one update to Step 7's re-review to leverage the
new structured format.

**In Step 7**, add a note that re-review runs use the same spawn pattern and
JSON extraction strategy from Steps 3-4. The orchestrator should compare
previous findings against new findings (by `title` + `lens`) to determine
resolution status.

Also update the re-review presentation to use the structured format:

**Current**:
```markdown
  ```
  ## Re-Review: [Plan Name]

  ### Previously Identified Issues
  - [Finding]: Resolved / Partially resolved / Still present
  - [Finding]: ...

  ### New Issues Introduced
  - [Any new findings from the edits]

  ### Assessment
  [Whether the plan is now in good shape or needs further iteration]
  ```
```

**Replace with**:
```markdown
  ```
  ## Re-Review: [Plan Name]

  **Verdict:** [APPROVE | REVISE | COMMENT]

  ### Previously Identified Issues
  - [emoji] **[Lens]**: [title] — Resolved / Partially resolved / Still present
  - ...

  ### New Issues Introduced
  - [emoji] **[Lens]**: [title] — [brief description]

  ### Assessment
  [Whether the plan is now in good shape or needs further iteration]
  ```
```

#### 7. Update Important Guidelines

**Location**: Add after the existing guidelines:

```markdown
9. **Handle malformed agent output gracefully** — if an agent doesn't return
   valid JSON, extract what you can and present the raw output to the user
   rather than silently dropping findings

10. **Keep positive feedback in the summary** — strengths go in the review
    summary, not as individual findings. Findings are exclusively for
    actionable concerns.

11. **Use emoji severity prefixes consistently** — 🔴 critical, 🟡 major,
    🔵 minor/suggestion, ✅ strengths
```

#### 8. Update What NOT to Do

**Location**: Replace the "## What NOT to Do" section with a merged list.

**Replace with**:
```markdown
## What NOT to Do

- Don't write review findings to a separate file — all output goes to the
  conversation and then into plan edits
- Don't skip the lens selection step — always confirm with the user which
  lenses will run
- Don't present raw agent output — always aggregate and curate into the
  structured format
- Don't make plan edits without user agreement
- Don't force all findings to be addressed — some may be intentionally
  accepted tradeoffs
- Don't run lenses that clearly aren't relevant — a documentation plan doesn't
  need a security review
- Don't post findings as individual items for positive feedback — strengths go
  in the summary only
- Don't skip the verdict — always include a suggested verdict based on finding
  severity
```

### Success Criteria

#### Automated Verification:

- [x] `commands/review-plan.md` parses as valid markdown
- [x] The file contains `plan-test-coverage-reviewer` in the lens table
- [x] The file contains "Handling malformed agent output" section
- [x] The file contains verdict logic (APPROVE, REVISE, COMMENT)
- [x] The file preserves Steps 6-7 (collaborative iteration and re-review)

#### Manual Verification:

- [ ] Run `/review-plan` against a real plan with at least 2 phases
- [ ] Confirm agents return valid JSON with `findings`, `strengths`, and
  `summary` fields
- [ ] Confirm the command presents a structured review with emoji prefixes
- [ ] Confirm the verdict reflects finding severity
- [ ] Confirm collaborative iteration (discuss → edit → re-review) still works
- [ ] Confirm the re-review tracks resolved vs new issues

---

## Testing Strategy

### Manual Testing Steps

1. **Agent output validation**: Run a single agent (e.g.,
   `plan-test-coverage-reviewer`) against a known plan and verify the JSON:
   - Contains valid JSON with `lens`, `summary`, `strengths`, `findings`
   - Finding bodies follow the emoji + lens + issue + impact + suggestion format
   - Locations reference plan sections correctly

2. **Code quality / test coverage separation**: Run both
   `plan-code-quality-reviewer` and `plan-test-coverage-reviewer` against the
   same plan. Verify:
   - Code quality findings focus on design principles, complexity, error handling
   - Test coverage findings focus on testing strategy, pyramid, edge cases
   - No overlap in findings between the two lenses

3. **End-to-end review flow**: Run `/review-plan` against a real plan:
   - Verify lens selection includes Test Coverage
   - Verify all agents run in parallel and return JSON
   - Verify the review summary uses emoji prefixes
   - Verify the verdict is suggested based on finding severity
   - Verify collaborative iteration works (edit plan, re-review)

4. **Malformed output handling**: Verify that if an agent returns prose instead
   of JSON, the command gracefully falls back and shows the raw output

5. **Edge cases**:
   - Plan with no findings (APPROVE verdict, strengths-only summary)
   - Plan with only critical findings (REVISE verdict)
   - Agent returning malformed JSON (graceful fallback)
   - Two agents flagging the same plan section with unrelated issues (should
     NOT merge)
   - Two agents flagging the same plan section with the same concern (should
     merge)

## References

- Alignment research: `meta/research/2026-02-22-review-plan-pr-alignment.md`
- Original PR agent design: `meta/research/2026-02-22-pr-review-agents-design.md`
- PR inline comments plan: `meta/plans/2026-02-22-pr-review-inline-comments.md`
- Current plan agents: `agents/plan-architecture-reviewer.md`,
  `agents/plan-code-quality-reviewer.md`, `agents/plan-security-reviewer.md`,
  `agents/plan-standards-reviewer.md`, `agents/plan-usability-reviewer.md`
- Current command: `commands/review-plan.md`
- PR agents (reference for JSON format): `agents/pr-architecture-reviewer.md`
  (template for all 6)
