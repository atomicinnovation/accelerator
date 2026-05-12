# PR Review Agents and Orchestrating Command Implementation Plan

## Overview

Implement six PR review agents and one orchestrating command (`/review-pr`) that
analyse pull requests through distinct quality lenses: architecture, security,
test coverage, code quality, standards compliance, and usability. The agents
follow the structural template established by the existing plan review agents,
adapted for reviewing actual code diffs rather than specification documents.

## Current State Analysis

The `~/.claude/agents/` directory contains five plan review agents that follow a
highly uniform template:

- `plan-architecture-reviewer.md`
- `plan-security-reviewer.md`
- `plan-code-quality-reviewer.md`
- `plan-standards-reviewer.md`
- `plan-usability-reviewer.md`

The `~/.claude/commands/` directory contains an orchestrating command
`review-plan.md` that selects lenses, spawns agents in parallel, synthesises
findings, and presents a compiled review.

No PR review agents or PR review orchestrating command exist yet.

### Key Discoveries:

- All five plan reviewers share identical structure: YAML frontmatter (`name`,
  `description`, `tools`), Core Responsibilities (3 items), Analysis Strategy
  (numbered steps), Output Format (severity-tiered findings with confidence),
  Important Guidelines (~7 bullets), What NOT to Do (5 bullets with lens
  scoping), closing Remember paragraph
- The `review-plan.md` command handles lens selection with user confirmation,
  parallel agent spawning, cross-lens synthesis with deduplication, severity-
  prioritised presentation, and collaborative plan iteration
- The `describe-pr.md` command demonstrates PR identification via `gh` CLI and
  diff retrieval patterns
- Tools for all plan review agents: `Read, Grep, Glob, LS`

## Desired End State

Seven new files exist:

1. `~/.claude/agents/pr-architecture-reviewer.md`
2. `~/.claude/agents/pr-security-reviewer.md`
3. `~/.claude/agents/pr-code-quality-reviewer.md`
4. `~/.claude/agents/pr-standards-reviewer.md`
5. `~/.claude/agents/pr-usability-reviewer.md`
6. `~/.claude/agents/pr-test-coverage-reviewer.md`
7. `~/.claude/commands/review-pr.md`

Each agent follows the established template and is usable both standalone (via
Task tool) and through the orchestrating command. The `/review-pr` command
successfully identifies a PR, fetches the diff to a temp directory, selects
relevant lenses, spawns agents in parallel, and synthesises findings.

### Verification:

- All seven files exist with correct naming
- Agent files follow the structural template (frontmatter, all required
  sections, correct tools)
- Each agent's "What NOT to Do" correctly excludes the other five PR lenses
- The orchestrating command references all six agents correctly
- The temp directory strategy uses `mktemp -d /tmp/pr-review-{number}-XXXXXXXX`
- Location references use the hybrid format: `file:line` with
  `(changed in PR)` / `(existing code, not changed)` annotation

## What We're NOT Doing

- Not creating a plan-test-coverage-reviewer (that's a separate concern)
- Not modifying existing plan review agents
- Not modifying the existing `/review-plan` command
- Not building CI/CD integration or GitHub Actions
- Not adding automated testing for the agents themselves
- Not creating a `/validate-pr` or `/improve-pr` command

## Implementation Approach

Adapt the five existing plan review agents to their PR counterparts by
transforming the lens-specific criteria from plan evaluation to code evaluation,
while preserving the structural template exactly. Create the test coverage agent
from scratch using the same template and the deep research findings. Finally,
create the orchestrating command by adapting the `/review-plan` command pattern
for PR workflows.

---

## Phase 1: PR Review Agents (Adapted from Plan Reviewers)

### Overview

Create five PR review agents by adapting their plan reviewer counterparts. Each
agent transforms the lens-specific criteria from evaluating plans to evaluating
actual code, while preserving the structural template.

### Changes Required:

#### 1. `pr-architecture-reviewer.md`

**File**: `~/.claude/agents/pr-architecture-reviewer.md`
**Adapted from**: `~/.claude/agents/plan-architecture-reviewer.md`
**Changes**: New file, adapted from plan counterpart

Structure:

```markdown
---
name: pr-architecture-reviewer
description: Reviews pull requests through an architecture lens. Use this agent
  when you want to evaluate whether code changes maintain sound architectural
  foundations -- modularity, coupling, dependency direction, and consistency with
  established patterns. It identifies structural drift and beyond-the-diff
  impact.
tools: Read, Grep, Glob, LS
---
```

**Core Responsibilities** (3 items):
1. **Evaluate Structural Impact of Changes** - module boundary integrity,
   separation of concerns in changed code, interface changes between modules,
   single points of failure introduced
2. **Analyse Coupling, Cohesion, and Dependencies** - dependency direction
   violations, circular dependencies introduced, data flow across trust
   boundaries, cohesion within modified modules
3. **Assess Architectural Consistency and Drift** - consistency with established
   patterns, justified vs unjustified divergence, evolutionary fitness of
   changes, functional core / imperative shell separation

**Analysis Strategy** (4 steps):
1. Read and understand the PR (read diff.patch and changed-files.txt from temp
   dir, understand scope and intent)
2. Explore the existing codebase (use Grep/Glob to find existing architectural
   patterns, read files adjacent to changed files for context; include
   "ultrathink" bullet about whether changes are consistent with or diverge from
   established patterns)
3. Evaluate architecture through each sub-lens (modularity, coupling & cohesion,
   system impact, evolutionary design, functional core / imperative shell, domain
   alignment)
4. Identify beyond-the-diff impact (trace how changed interfaces affect
   consumers, identify ripple effects, note implicit architectural decisions)

**Output Format**:
```
## Architecture Review: PR #{number}

### Summary
[2-3 sentence architectural assessment]

### Findings
#### Critical / Major / Minor / Suggestions
- **[Finding title]** (confidence: high/medium/low)
  **Location**: `file:line` (changed in PR | existing code, not changed)
  **Issue**: [architectural problem]
  **Impact**: [why this matters structurally]
  **Suggestion**: [concrete alternative]

### Architectural Impact Assessment
- **Module boundaries**: [Assessment]
- **Dependency direction**: [Assessment]
- **System-wide impact**: [Assessment]
- **Evolutionary fitness**: [Assessment]

### Codebase Consistency
- **Consistent with**: [patterns followed]
- **Diverges from**: [patterns broken, with justification assessment]

### Strengths
- [Architectural decisions the PR gets right]
```

**Important Guidelines** (~7 bullets): Always read the diff and changed files
list first; explore the codebase for context beyond the diff; be specific with
file:line references; assess tradeoffs fairly; consider beyond-the-diff impact;
rate confidence on each finding; think in terms of architectural forces.

**What NOT to Do** (5 bullets):
1. Don't review security, test coverage, code quality, standards, or usability
   -- those are other lenses
2. Don't suggest complete redesigns -- work within the PR's constraints
3. Don't penalise tradeoffs that are explicitly acknowledged
4. Don't assume your preferred architecture is the only valid one
5. Don't ignore the existing codebase context when evaluating changes

**Closing**: "Remember: You're evaluating whether the PR's changes maintain or
improve the system's architectural integrity -- modularity, appropriate coupling,
and evolutionary fitness."

#### 2. `pr-security-reviewer.md`

**File**: `~/.claude/agents/pr-security-reviewer.md`
**Adapted from**: `~/.claude/agents/plan-security-reviewer.md`
**Changes**: New file, adapted from plan counterpart

Structure:

```markdown
---
name: pr-security-reviewer
description: Reviews pull requests through a security lens. Use this agent when
  you want to identify vulnerabilities, missing protections, and security gaps
  in code changes. It covers OWASP Top 10, input validation, auth/authz, secrets
  exposure, and information disclosure at a pragmatic depth.
tools: Read, Grep, Glob, LS
---
```

**Core Responsibilities** (3 items):
1. **Identify Vulnerabilities in Changed Code** - OWASP Top 10 in actual code
   (injection, broken access control, cryptographic failures, SSRF, security
   misconfiguration), input validation completeness, output encoding
2. **Evaluate Authentication and Authorisation** - auth checks at every access
   point, default-deny policies, horizontal/vertical privilege escalation,
   session management, re-authentication for sensitive operations
3. **Detect Secrets and Information Disclosure** - hardcoded secrets/credentials
   in diff, sensitive data in error messages/logs, debug output exposure, data
   flow from user input to storage/output

**Analysis Strategy** (5 steps):
1. Read and understand the PR (read diff and changed files, identify entry
   points, data stores, external integrations, user-facing surfaces)
2. Explore the existing security posture (find existing auth/authz patterns,
   validation implementations, security middleware, secrets management)
3. Trace data flows in changed code (follow user input from entry to output/
   storage, identify trust boundary crossings, note sensitive data paths)
4. Apply OWASP Top 10 to changed code (injection, broken access control,
   cryptographic failures, security misconfiguration, SSRF, etc.)
5. Check for secrets and information disclosure (scan diff for hardcoded
   secrets, check error handling for stack trace exposure, review logging for
   sensitive data)

**Output Format**:
```
## Security Review: PR #{number}

### Summary
[2-3 sentence security assessment]

### Threat Assessment
- **Attack surface changes**: [new entry points, external integrations]
- **Sensitive data flows**: [PII, credentials, tokens in changed code]
- **Trust boundary crossings**: [where trusted meets untrusted]

### Findings
#### Critical / Major / Minor / Suggestions
- **[Finding title]** (confidence: high/medium/low)
  **Location**: `file:line` (changed in PR | existing code, not changed)
  **OWASP category**: [if applicable]
  **Vulnerability**: [what could go wrong and how]
  **Impact**: [consequences of exploitation]
  **Suggestion**: [concrete mitigation with code snippet if appropriate]

### OWASP Top 10 Coverage
- [Category]: [Relevant / Not relevant / Addressed / Gap identified]

### Data Flow Analysis
- [Assessment of user input handling through the changed code]

### Strengths
- [Security decisions the PR gets right]
```

**Important Guidelines**: Always read the diff fully first; explore the codebase
for existing security patterns; be pragmatic -- focus on high-impact likely
threats; rate confidence on each finding; consider the full stack; check for
defence in depth; assess secrets handling.

**What NOT to Do**:
1. Don't review architecture, test coverage, code quality, standards, or
   usability -- those are other lenses
2. Don't flag theoretical threats with negligible real-world likelihood
3. Don't assume the worst about every decision -- assess proportionally
4. Don't recommend security theatre -- controls should provide real protection
5. Don't ignore the existing codebase's security patterns when evaluating changes

**Closing**: "Remember: You're identifying where the code changes leave the door
open to real threats. Good security review is pragmatic -- it catches what's
most likely to cause harm."

#### 3. `pr-code-quality-reviewer.md`

**File**: `~/.claude/agents/pr-code-quality-reviewer.md`
**Adapted from**: `~/.claude/agents/plan-code-quality-reviewer.md`
**Changes**: New file, adapted from plan counterpart

Structure:

```markdown
---
name: pr-code-quality-reviewer
description: Reviews pull requests through a code quality lens. Use this agent
  when you want to evaluate whether code changes are clean, maintainable, and
  readable -- checking complexity, design principles, error handling, and code
  smells. Pragmatic, not pedantic.
tools: Read, Grep, Glob, LS
---
```

**Core Responsibilities** (3 items):
1. **Evaluate Code Complexity and Readability** - cyclomatic/cognitive
   complexity of changed code, deep nesting, long methods/functions, meaningful
   naming, self-documenting code, guard clauses over nesting
2. **Assess Design Principle Adherence** - SOLID in changed code, DRY/KISS/YAGNI,
   design pattern fitness (right pattern, not over-engineered), functional
   purity where applicable, composition over inheritance
3. **Review Error Handling and Code Smells** - appropriate error categorisation,
   propagation strategy, no swallowed exceptions, code smells (god objects,
   feature envy, primitive obsession, flag arguments, dead code)

**Analysis Strategy** (4 steps):
1. Read and understand the PR (read diff and changed files, identify scope and
   complexity of changes)
2. Explore existing quality patterns (find existing code quality patterns, read
   adjacent code for conventions; include "ultrathink" about whether changes
   follow or improve upon patterns)
3. Evaluate code quality through each sub-lens (complexity metrics, clean code
   principles, code smells, readability, error handling)
4. Assess maintainability (will the next developer understand this in six
   months? are changes testable? is complexity proportional to requirements?)

**Output Format**:
```
## Code Quality Review: PR #{number}

### Summary
[2-3 sentence quality assessment]

### Findings
#### Critical / Major / Minor / Suggestions
- **[Finding title]** (confidence: high/medium/low)
  **Location**: `file:line` (changed in PR | existing code, not changed)
  **Issue**: [quality concern]
  **Impact**: [why this will cause maintenance pain]
  **Suggestion**: [concrete improvement with code snippet if appropriate]

### Complexity Assessment
- **Cyclomatic complexity**: [Assessment of changed code]
- **Cognitive complexity**: [Assessment of changed code]
- **Nesting depth**: [Assessment]
- **Method/function length**: [Assessment]

### Design Principle Assessment
- **Single Responsibility**: [Assessment]
- **Open-Closed**: [Assessment]
- **Dependency Inversion**: [Assessment]
- **Functional Purity**: [Assessment]
- **Pattern Fitness**: [Are patterns appropriate and not over-engineered?]

### Error Handling Assessment
- **Error categorisation**: [Assessment]
- **Error propagation**: [Assessment]
- **Exception handling**: [Assessment]

### Codebase Consistency
- **Follows existing patterns**: [What the PR gets right]
- **Departs from conventions**: [Where and whether departure is justified]

### Strengths
- [Quality decisions the PR gets right]
```

**Important Guidelines**: Always read the diff fully first; explore the codebase
for existing quality patterns; be pragmatic -- focus on issues causing real
maintenance pain; rate confidence; evaluate proportionally (simple utility vs
core domain); consider readability for the next developer; check testability.

**What NOT to Do**:
1. Don't review architecture, security, test coverage, standards, or usability
   -- those are other lenses
2. Don't nitpick style preferences that don't affect maintainability
3. Don't insist on patterns where simplicity serves better
4. Don't penalise pragmatic shortcuts that are explicitly acknowledged
5. Don't recommend adding complexity in the name of "best practices"

**Closing**: "Remember: You're evaluating whether the code changes are a pleasure
to maintain -- readable, well-structured, and simply designed. The best code
quality is the simplest design that meets the requirements."

#### 4. `pr-standards-reviewer.md`

**File**: `~/.claude/agents/pr-standards-reviewer.md`
**Adapted from**: `~/.claude/agents/plan-standards-reviewer.md`
**Changes**: New file, adapted from plan counterpart

Structure:

```markdown
---
name: pr-standards-reviewer
description: Reviews pull requests through a standards compliance lens. Use this
  agent when you want to check whether code changes align with project
  conventions, API design standards, naming patterns, and documentation
  practices. It auto-detects which standards apply and infers conventions from
  the codebase.
tools: Read, Grep, Glob, LS
---
```

**Core Responsibilities** (3 items):
1. **Evaluate Project Convention Compliance** - naming conventions (files,
   functions, variables, classes), file organisation patterns, import/export
   conventions, configuration management patterns, consistency with existing
   codebase
2. **Check API and Interface Standards** - RESTful conventions (resource naming,
   HTTP methods, status codes), error response format consistency, API versioning,
   content negotiation, HTTP semantics (idempotency, cacheability)
3. **Assess Documentation and Change Management** - public interface
   documentation, breaking change identification, migration guides for consumers,
   changelog entries, ADRs for significant decisions

**Analysis Strategy** (4 steps):
1. Read and understand the PR (read diff and changed files, categorise: API
   changes, UI changes, backend logic, infrastructure, data model)
2. Discover applicable standards (find linting configs, style guides, convention
   documentation; read existing similar code to infer implicit conventions;
   include "ultrathink" about implicit conventions)
3. Evaluate against each applicable standard (project conventions, API standards
   if applicable, web standards if applicable, accessibility if applicable,
   documentation standards)
4. Distinguish convention from preference (flag genuine inconsistencies, not
   matters of taste; higher confidence for documented standards, lower for
   inferred patterns)

**Output Format**:
```
## Standards Compliance Review: PR #{number}

### Summary
[2-3 sentence compliance assessment]

### Applicable Standards
- [List of standard categories that apply to this PR and why]

### Findings
#### Critical / Major / Minor / Suggestions
- **[Finding title]** (confidence: high/medium/low)
  **Location**: `file:line` (changed in PR | existing code, not changed)
  **Standard**: [which standard is violated]
  **Issue**: [how the code deviates]
  **Impact**: [consequences of non-compliance]
  **Suggestion**: [how to align with the standard]

### Convention Consistency
- **Consistent with codebase**: [Conventions the PR follows correctly]
- **Inconsistent with codebase**: [Where the PR breaks established patterns]
- **Undocumented conventions noted**: [Implicit patterns discovered]

### API Standards Assessment (if applicable)
- **REST conventions**: [Assessment]
- **HTTP semantics**: [Assessment]
- **Error responses**: [Assessment]
- **Versioning**: [Assessment]

### Documentation Provisions
- **Public interfaces documented**: [Assessment]
- **Breaking changes noted**: [Assessment]
- **Migration guides**: [Assessment]

### Strengths
- [Standards decisions the PR gets right]
```

**Important Guidelines**: Always read the diff fully first; explore the codebase
thoroughly to discover both documented and implicit standards; auto-detect
applicability; infer conventions when formal documentation is absent but flag the
inference; rate confidence; distinguish convention from preference; consider the
audience.

**What NOT to Do**:
1. Don't review architecture, security, test coverage, code quality, or
   usability -- those are other lenses
2. Don't invent standards that don't exist in the project or industry
3. Don't enforce standards on areas where the codebase itself is inconsistent
4. Don't flag regulatory or legal compliance -- focus on technical standards only
5. Don't penalise deliberate, justified departures from convention

**Closing**: "Remember: You're ensuring the code changes play by the established
rules -- both written and unwritten. Consistent standards reduce cognitive load
and make codebases navigable."

#### 5. `pr-usability-reviewer.md`

**File**: `~/.claude/agents/pr-usability-reviewer.md`
**Adapted from**: `~/.claude/agents/plan-usability-reviewer.md`
**Changes**: New file, adapted from plan counterpart

Structure:

```markdown
---
name: pr-usability-reviewer
description: Reviews pull requests through a usability lens focused on developer
  experience. Use this agent when you want to evaluate DX concerns in code
  changes -- API ergonomics, error message quality, configuration complexity,
  migration paths, and documentation for public interfaces. It balances
  convenience with safety.
tools: Read, Grep, Glob, LS
---
```

**Core Responsibilities** (3 items):
1. **Evaluate Developer Experience of Changes** - API ergonomics (consistency,
   minimality, discoverability, least surprise), time to first success, sensible
   defaults, progressive disclosure
2. **Assess Error and Configuration Experience** - error message actionability
   (what/why/how-to-fix), structured error format, graceful degradation,
   configuration complexity proportional to needs, validation at startup
3. **Review Migration and Backward Compatibility** - breaking changes identified,
   migration path clarity, deprecation strategy, incremental upgrade support,
   versioning communication

**Analysis Strategy** (5 steps):
1. Read and understand the PR (read diff and changed files, identify all
   interfaces exposed: APIs, CLIs, configuration surfaces, libraries)
2. Explore existing DX patterns (find existing API patterns, error handling,
   configuration approaches; include "ultrathink" about whether changes improve,
   maintain, or degrade DX)
3. Evaluate API ergonomics (consistency, minimality, discoverability,
   composability, least surprise)
4. Assess error and configuration experience (actionable errors, structured
   format, sensible defaults, validation)
5. Evaluate migration and compatibility (breaking changes identified, migration
   paths defined, deprecation strategy)

**Output Format**:
```
## Usability Review: PR #{number}

### Summary
[2-3 sentence DX assessment]

### Findings
#### Critical / Major / Minor / Suggestions
- **[Finding title]** (confidence: high/medium/low)
  **Location**: `file:line` (changed in PR | existing code, not changed)
  **Issue**: [usability problem]
  **Impact**: [how this affects developers using the system]
  **Suggestion**: [concrete improvement]

### API Ergonomics Assessment
- **Consistency**: [Assessment]
- **Minimality**: [Assessment]
- **Discoverability**: [Assessment]
- **Least surprise**: [Assessment]

### Error Experience Assessment
- **Actionability**: [Do errors tell you what/why/how-to-fix?]
- **Contextuality**: [Do errors include relevant context?]
- **Graceful degradation**: [Does the system fail partially?]

### Configuration Assessment
- **Defaults**: [Are they sensible and secure?]
- **Complexity**: [Proportional to needs?]
- **Error detection**: [Are misconfigurations caught early?]

### Migration & Compatibility Assessment
- **Breaking changes identified**: [Yes/No/Partially]
- **Migration paths defined**: [Yes/No/Partially]
- **Deprecation strategy**: [Assessment]

### Strengths
- [DX decisions the PR gets right]
```

**Important Guidelines**: Always read the diff fully first; explore the codebase
for existing DX patterns; think like a consumer; rate confidence; balance
convenience and safety; focus on DX not end-user UX unless explicitly involved;
evaluate documentation only for public APIs.

**What NOT to Do**:
1. Don't review architecture, security, test coverage, code quality, or
   standards -- those are other lenses
2. Don't evaluate end-user UX unless the PR explicitly involves UI changes
3. Don't insist on documentation for every internal interface
4. Don't prioritise convenience over safety -- flag the tradeoff, don't decide it
5. Don't assume your DX preferences are universal -- assess against common
   patterns

**Closing**: "Remember: You're evaluating whether the code changes create
interfaces that developers will enjoy using -- intuitive, forgiving of mistakes,
and smooth to upgrade."

### Success Criteria:

#### Automated Verification:

- [x] All five files exist: `ls ~/.claude/agents/pr-{architecture,security,code-quality,standards,usability}-reviewer.md`
- [x] Each file has correct YAML frontmatter with `name`, `description`, `tools` fields
- [x] Each file's `tools` field is `Read, Grep, Glob, LS`

#### Manual Verification:

- [x] Each agent's Core Responsibilities has exactly 3 items adapted for code review
- [x] Each agent's Analysis Strategy Step 1 references reading diff.patch and changed-files.txt
- [x] Each agent's Analysis Strategy Step 2 includes "ultrathink" (except security which distributes reasoning across STRIDE/OWASP steps)
- [x] Each agent's Output Format uses `**Location**: \`file:line\` (changed in PR | existing code, not changed)` format
- [x] Each agent's What NOT to Do bullet 1 excludes the other 5 PR lenses by name
- [x] Each agent has a closing "Remember:" paragraph
- [x] Descriptions follow the two-sentence pattern from plan reviewers

---

## Phase 2: PR Test Coverage Agent (New)

### Overview

Create the test coverage review agent from scratch. This is the one lens without
a plan reviewer counterpart, so it draws entirely from the research findings on
test quality, test pyramid, mutation testing, and anti-patterns.

### Changes Required:

#### 1. `pr-test-coverage-reviewer.md`

**File**: `~/.claude/agents/pr-test-coverage-reviewer.md`
**Adapted from**: N/A (net-new, informed by research)
**Changes**: New file

Structure:

```markdown
---
name: pr-test-coverage-reviewer
description: Reviews pull requests through a test coverage and quality lens. Use
  this agent when you want to evaluate whether code changes have adequate,
  well-structured tests -- checking coverage adequacy, assertion quality, test
  pyramid balance, isolation, and common anti-patterns.
tools: Read, Grep, Glob, LS
---
```

**Core Responsibilities** (3 items):
1. **Evaluate Coverage Adequacy** - new code paths have corresponding tests, bug
   fixes include regression tests, edge cases and boundary conditions covered,
   error paths tested not just happy paths, critical business logic has thorough
   unit tests
2. **Assess Test Quality and Assertions** - tests verify behaviour not
   implementation details, assertions are specific and meaningful (not just
   assertNotNull), tests follow Arrange-Act-Assert, mutation testing perspective
   ("if I changed this operator, would any test fail?"), no assertion-free tests
3. **Review Test Architecture and Reliability** - test pyramid balance (unit
   foundation, integration for boundaries, minimal E2E), test isolation
   (determinism, no shared mutable state, no system dependencies), no
   anti-patterns (over-mocking, flaky tests, disabled tests, implementation
   coupling), test code treated as first-class (readable, well-structured)

**Analysis Strategy** (5 steps):
1. Read and understand the PR (read diff and changed files, identify which files
   are production code and which are test code, note the ratio)
2. Explore existing test patterns (find existing test files adjacent to changed
   code, read test conventions: naming, structure, frameworks, helper patterns;
   include "ultrathink" about whether the PR's tests follow or improve upon
   established testing patterns)
3. Evaluate coverage adequacy (for each production code change, check if
   corresponding tests exist; for bug fixes, check for regression tests; identify
   untested code paths, edge cases, error conditions)
4. Assess test quality (examine assertions: are they specific and meaningful?
   apply the mutation testing lens: "if I changed this operator, would a test
   fail?"; check for assertion-free tests, overly loose assertions, tests
   coupled to implementation details)
5. Review test architecture and reliability (assess pyramid level appropriateness;
   check for determinism issues: system time, randomness, external services,
   shared state; flag anti-patterns: over-mocking, disabled tests, test logic,
   sleep-based synchronisation)

**Output Format**:
```
## Test Coverage Review: PR #{number}

### Summary
[2-3 sentence test coverage and quality assessment]

### Findings
#### Critical / Major / Minor / Suggestions
- **[Finding title]** (confidence: high/medium/low)
  **Location**: `file:line` (changed in PR | existing code, not changed)
  **Issue**: [test coverage or quality concern]
  **Impact**: [why this matters for confidence in the code]
  **Suggestion**: [concrete improvement, e.g., specific test to add]

### Coverage Assessment
- **New code paths tested**: [Assessment]
- **Bug fix regression tests**: [Assessment, if applicable]
- **Edge cases and boundaries**: [Assessment]
- **Error path coverage**: [Assessment]

### Test Quality Assessment
- **Assertion quality**: [Are assertions specific and meaningful?]
- **Behaviour vs implementation testing**: [Are tests testing contracts?]
- **Mutation resilience**: [Would tests catch subtle bugs?]
- **Test naming and structure**: [Are tests readable and well-organised?]

### Test Architecture Assessment
- **Pyramid balance**: [Are tests at the right level?]
- **Test isolation**: [Are tests deterministic and independent?]
- **Mock strategy**: [Are mocks used only at true boundaries?]
- **Test performance**: [Are tests fast enough for CI?]

### Anti-Pattern Detection
- **Assertion-free tests**: [Found / Not found]
- **Implementation coupling**: [Found / Not found]
- **Over-mocking**: [Found / Not found]
- **Flaky test indicators**: [Found / Not found]
- **Disabled/skipped tests**: [Found / Not found]

### Strengths
- [Testing decisions the PR gets right]
```

**Important Guidelines** (~7 bullets):
- **Always read the diff fully first** -- identify production code vs test code
  changes
- **Explore the codebase** to understand existing test patterns and conventions
- **Apply the mutation testing lens** -- mentally ask "if I changed this
  operator, would any test fail?" for critical code paths
- **Be pragmatic** -- focus on missing coverage that represents real risk, not
  100% coverage dogma
- **Rate confidence** on each finding -- distinguish definite gaps from potential
  concerns
- **Evaluate proportionally** -- a trivial utility doesn't need the same test
  rigour as a payment processor
- **Consider test maintainability** -- overly complex tests are a liability, not
  an asset

**What NOT to Do** (5 bullets):
1. Don't review architecture, security, code quality, standards, or usability
   -- those are other lenses
2. Don't insist on 100% coverage -- focus on coverage that provides meaningful
   confidence
3. Don't penalise test approaches that differ from your preference if they are
   effective
4. Don't flag test style issues that don't affect test reliability or
   maintainability
5. Don't ignore the existing codebase's testing patterns when evaluating the PR

**Closing**: "Remember: You're evaluating whether the tests give genuine
confidence that the code works correctly -- not just that code was executed, but
that behaviour was verified. The best tests catch real bugs, survive refactoring,
and run reliably."

### Success Criteria:

#### Automated Verification:

- [x] File exists: `ls ~/.claude/agents/pr-test-coverage-reviewer.md`
- [x] File has correct YAML frontmatter with `name`, `description`, `tools` fields
- [x] `tools` field is `Read, Grep, Glob, LS`

#### Manual Verification:

- [x] Core Responsibilities covers coverage adequacy, test quality, and test
  architecture (3 items)
- [x] Analysis Strategy includes mutation testing perspective in Step 4
- [x] Analysis Strategy Step 2 includes "ultrathink" bullet
- [x] Output Format includes Anti-Pattern Detection section
- [x] Output Format uses hybrid location reference format
- [x] What NOT to Do bullet 1 excludes the other 5 PR lenses
- [x] Guidelines include "apply the mutation testing lens" bullet
- [x] Closing "Remember:" paragraph is present

---

## Phase 3: Review PR Orchestrating Command

### Overview

Create the `/review-pr` command that orchestrates the six PR review agents. It
handles PR identification, diff fetching, temp directory creation, lens
selection, parallel agent spawning, finding synthesis, and compiled review
presentation. Modelled on `/review-plan` with adaptations for PR workflow.

### Changes Required:

#### 1. `review-pr.md`

**File**: `~/.claude/commands/review-pr.md`
**Modelled on**: `~/.claude/commands/review-plan.md`
**Changes**: New file

Structure:

```markdown
# Review PR

You are tasked with reviewing a pull request through multiple quality lenses and
then presenting a compiled analysis of the code changes.
```

**Initial Response** (follows review-plan pattern):
1. Check if a PR number or URL was provided as argument
2. If provided, identify the PR immediately
3. If not provided, respond with instructions:
   ```
   I'll help you review a pull request. Please provide:
   1. The PR number or URL (or I'll check the current branch)
   2. (Optional) Focus areas to emphasise (e.g., "focus on security and
      architecture")

   Tip: You can invoke this command with arguments:
     `/review-pr 123`
     `/review-pr 123 focus on security and test coverage`
   ```
4. If no argument, check if current branch has a PR:
   `gh pr view --json number,url,title,state 2>/dev/null`

**Step 1: Identify and Fetch the PR**

1. Get PR metadata:
   `gh pr view {number} --json number,url,title,state,baseRefName,headRefName`
2. Create temp directory:
   ```bash
   REVIEW_DIR=$(mktemp -d "/tmp/pr-review-${PR_NUMBER}-XXXXXXXX")
   ```
3. Fetch diff, changed files, PR description, and commit context:
   ```bash
   gh pr diff {number} > "$REVIEW_DIR/diff.patch"
   gh pr diff {number} --name-only > "$REVIEW_DIR/changed-files.txt"
   gh pr view {number} --json body --jq '.body' > "$REVIEW_DIR/pr-description.md"
   gh pr view {number} --json commits --jq '.commits[].messageHeadline' > "$REVIEW_DIR/commits.txt"
   ```
4. Read the diff, changed files list, PR description, and commits to understand
   scope and intent

**Error handling**: If any `gh` command fails, handle these cases:
- **`gh` not installed or not authenticated**: Inform the user that the `gh`
  CLI is required and suggest running `gh auth login` to authenticate.
- **No default remote repository**: Instruct the user to run
  `gh repo set-default` and select the appropriate repository (mirrors the
  pattern in `/describe-pr`).
- **Invalid PR number or PR not found**: Inform the user that the PR could not
  be found and suggest checking the number. If on a branch with no PR, list
  open PRs with `gh pr list --limit 10` and ask the user to select one.
- **Empty diff**: If `diff.patch` is empty (e.g., a draft PR with no changes),
  inform the user and ask whether to proceed with a review of the PR
  description and commits only.

**Step 2: Select Review Lenses**

Determine which lenses are relevant based on the PR's scope and any user-
provided focus arguments. Follow the same pattern as review-plan:

If the user provided focus arguments, map them to lenses and include any
additionally relevant ones. If not, auto-detect:

- **Architecture** -- relevant for most PRs; skip only for trivial single-file
  changes
- **Security** -- relevant when changes involve: user input handling, auth/authz,
  data storage, external integrations, API endpoints, secrets/config
- **Test Coverage** -- relevant for most PRs; skip only for documentation-only
  or configuration-only changes
- **Code Quality** -- relevant for most PRs; skip only for documentation-only
  changes
- **Standards** -- relevant when changes involve: API changes, new files/modules,
  public interfaces, naming-heavy changes
- **Usability** -- relevant when changes involve: public APIs, CLI interfaces,
  configuration surfaces, breaking changes, developer-facing libraries

Present lens selection to the user and wait for confirmation before proceeding.

**Step 3: Spawn Review Agents**

Spawn all selected review agents in parallel using the Task tool. Each agent
receives:
- The temp directory path containing:
  - `diff.patch` -- full PR diff
  - `changed-files.txt` -- list of changed file paths
  - `pr-description.md` -- the PR description/body
  - `commits.txt` -- commit message headlines
- The PR number and metadata for context
- Instructions to read the PR context, explore the codebase, and return
  structured analysis
- Guidance on the hybrid diff access strategy: "For PRs with many changed files,
  prioritise reading files most relevant to your lens rather than consuming the
  full diff."

Example spawn pattern:
```
Task 1 (pr-architecture-reviewer):
  "Review PR #{number} for architectural concerns. The PR context is at
  {temp_dir}/ -- read diff.patch, changed-files.txt, pr-description.md, and
  commits.txt. Explore the codebase for context and return your structured
  analysis."

Task 2 (pr-security-reviewer):
  "Review PR #{number} for security vulnerabilities. The PR context is at
  {temp_dir}/ -- read diff.patch, changed-files.txt, pr-description.md, and
  commits.txt. ..."

[... additional lenses as selected ...]
```

Wait for ALL review agents to complete before proceeding.

**Step 4: Compile and Synthesise Findings**

Once all reviews are complete (follows review-plan synthesis pattern):

1. Collect all findings across lenses, categorised by severity
2. Identify cross-cutting themes (findings from multiple lenses)
3. Assess tradeoffs where lenses conflict
4. Deduplicate findings flagged by multiple lenses
5. Prioritise by impact (structural > surface-level, multi-lens > single-lens)

**Step 5: Present the Review**

Present compiled review in this format:

```markdown
## PR Review: #{number} - {title}

### Overall Assessment

[2-3 sentences summarising the PR's quality across all lenses. Highlight
strongest aspects and most significant concerns.]

### Cross-Cutting Themes

[Issues that multiple lenses identified]

- **[Theme]** (flagged by: architecture, security)
  [Description and why multiple lenses care about this]

### Findings by Severity

#### Critical

- **[Finding title]** (lenses: [which lenses flagged this])
  **Location**: `file:line` (changed in PR | existing code, not changed)
  **Issue**: [What's wrong]
  **Impact**: [Why it matters]
  **Recommendation**: [What to change]

#### Major
#### Minor
#### Suggestions

### Tradeoff Analysis

[Where different lenses disagree]

### Strengths

[What the PR gets right -- aggregated across all lenses]

### Recommended Changes

[Ordered list of specific, actionable changes, prioritised by impact]

1. **[Change description]** (addresses: [finding titles])
   [Specific guidance]
```

**Step 6: Offer Follow-Up Options**

After presenting the review:

```
The review is complete. Would you like to:
1. Discuss any specific findings in more detail?
2. Re-run specific lenses with adjusted focus?
3. Post a summary as a PR comment? (`gh pr comment {number} --body "..."`)
```

**Important Guidelines** (adapted from review-plan):
1. Read the diff before doing anything else
2. Spawn agents in parallel
3. Synthesise, don't concatenate
4. Be balanced -- highlight strengths alongside concerns
5. Prioritise by impact
6. Respect tradeoffs between lenses
7. Clean up temp directory only at session end

**What NOT to Do** (adapted from review-plan):
- Don't write review findings to a separate file -- all output goes to the
  conversation
- Don't skip the lens selection step
- Don't present raw agent output -- always synthesise
- Don't run lenses that clearly aren't relevant
- Don't modify any code -- this is a read-only review

**Relationship to Other Commands**:

The PR review sits in the development lifecycle alongside other commands:

1. `/create-plan` -- Create the implementation plan
2. `/review-plan` -- Review and iterate the plan quality
3. `/implement-plan` -- Execute the approved plan
4. `/validate-plan` -- Verify implementation matches the plan
5. `/describe-pr` -- Generate PR description
6. `/review-pr` -- Review the PR through quality lenses (this command)

### Success Criteria:

#### Automated Verification:

- [x] File exists: `ls ~/.claude/commands/review-pr.md`

#### Manual Verification:

- [x] Command handles PR identification (argument, current branch, user prompt)
- [x] Temp directory uses `mktemp -d /tmp/pr-review-{number}-XXXXXXXX`
- [x] Diff fetched with `gh pr diff` and changed files with `--name-only`
- [x] Lens selection step with user confirmation before spawning agents
- [x] All six PR review agents referenced correctly
- [x] Synthesis follows the review-plan pattern (cross-cutting themes,
  deduplication, severity prioritisation, tradeoff analysis)
- [x] Compiled output format includes Overall Assessment, Cross-Cutting Themes,
  Findings by Severity, Tradeoff Analysis, Strengths, Recommended Changes
- [x] Follow-up options offered after review
- [x] Relationship to Other Commands section includes full command lifecycle

---

## Testing Strategy

Since these are configuration files (Markdown agent definitions and command
definitions), testing is manual verification against the structural template.

### Verification Checklist:

1. **Template compliance**: Each agent follows the 7-section structure
2. **Lens scoping**: Each agent's What NOT to Do correctly names the other 5
   PR lenses
3. **Cross-reference**: The orchestrating command references all 6 agents
4. **Temp directory**: The command uses the agreed randomised temp directory
   pattern
5. **Location format**: All agents use the hybrid location reference format
6. **Consistency**: All agents use the same severity tiers, confidence ratings,
   and finding structure

### Smoke Test:

After implementation, run `/review-pr` on an actual PR to verify end-to-end
flow: PR identification, diff fetching, lens selection, agent spawning, and
finding synthesis.

## References

- Research document: `~/.claude/meta/research/codebase/2026-02-22-pr-review-agents-design.md`
- Plan review agents: `~/.claude/agents/plan-{architecture,security,code-quality,standards,usability}-reviewer.md`
- Review plan command: `~/.claude/commands/review-plan.md`
- Describe PR command: `~/.claude/commands/describe-pr.md`
