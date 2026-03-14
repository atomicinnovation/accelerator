---
date: 2026-02-22T13:59:16+00:00
researcher: Toby Clemson
git_commit: n/a
branch: n/a
repository: claude-config
topic: "Designing PR Review Agents for Multi-Lens Pull Request Analysis"
tags: [research, agents, pr-review, architecture, security, test-coverage, code-quality, standards, usability]
status: complete
last_updated: 2026-02-22
last_updated_by: Toby Clemson
---

# Research: Designing PR Review Agents for Multi-Lens Pull Request Analysis

**Date**: 2026-02-22T13:59:16+00:00
**Researcher**: Toby Clemson
**Git Commit**: n/a (claude config directory, not a git repo)
**Branch**: n/a
**Repository**: claude-config

## Research Question

How should we design six PR review agents (architecture, security, test coverage,
code quality, standards compliance, usability) that analyse pull requests through
distinct quality lenses, modelled on the existing plan review agent pattern?

## Summary

This research synthesises findings from four parallel investigations: (1) the
structural patterns of existing plan review agents, (2) web research on
AI-powered PR review tools and their architectures, (3) per-lens review criteria
from professional code review practices, and (4) deep research on test coverage
review specifically (the one lens without an existing plan reviewer counterpart).

The key findings are:

1. **The existing plan review agent template is highly uniform** and provides a
   clear structural pattern to follow. All five plan reviewers share identical
   YAML frontmatter format, section structure (Core Responsibilities, Analysis
   Strategy, Output Format, Important Guidelines, What NOT to Do), severity
   tiers, confidence ratings, and scoping conventions.

2. **PR review differs from plan review in important ways**: PR reviewers examine
   actual code diffs rather than specification documents, need to reference
   specific file:line locations in the diff, must consider the codebase context
   beyond the diff, and evaluate implementation quality rather than design intent.

3. **Industry tools (CodeRabbit, Qodo, Sourcery, Greptile) converge on similar
   patterns**: severity-tiered findings, inline code references, structured
   output with domain-specific assessment sections, and explicit lens separation.

4. **Test coverage is a new lens** not covered by the existing plan reviewers.
   Research from Google's engineering practices, Martin Fowler's test pyramid,
   mutation testing literature, and anti-pattern catalogues provides rich criteria
   for this lens.

5. **Each lens has distinct review criteria when applied to code vs. plans**:
   architecture review in code focuses on coupling metrics and dependency
   direction violations; security review focuses on OWASP Top 10 in actual code;
   code quality focuses on complexity metrics and code smells; standards focuses
   on naming conventions and API consistency; usability focuses on API ergonomics
   and error message quality.

## Detailed Findings

### 1. Existing Agent Structural Patterns

All five plan review agents follow an identical template:

**YAML Frontmatter:**
```yaml
---
name: pr-[LENS]-reviewer
description: Reviews pull requests through a [LENS] lens. Use this agent when...
tools: Read, Grep, Glob, LS
---
```

**Body sections (in order):**
1. Opening paragraph: "You are a specialist at evaluating [TARGET] from a [LENS]
   perspective."
2. `## Core Responsibilities` - exactly 3 numbered items with bold titles and
   4 dash-bullet sub-items
3. `## Analysis Strategy` - numbered steps starting with "Read and understand",
   then "Explore existing codebase", then domain-specific evaluation steps
4. `## Output Format` - fenced code block with severity-tiered findings
5. `## Important Guidelines` - ~7 bold-titled bullet points
6. `## What NOT to Do` - exactly 5 bullets, first always excludes other lenses
7. Closing "Remember:" paragraph

**Finding structure (universal across all plan reviewers):**
```markdown
- **[Finding title]** (confidence: high/medium/low)
  **Location in [target]**: [Section or line reference]
  **[Domain field]**: [Value]
  **Impact**: [Why it matters]
  **Suggestion**: [Concrete remedy]
```

**Key pattern: "ultrathink"** - appears in Step 2 (codebase exploration) of 4/5
plan reviewers. Always phrased "Take time to ultrathink about..."

**Key pattern: lens scoping** - "What NOT to Do" bullet 1 always reads "Don't
review [other lenses] -- those are other lenses"

### 2. Key Differences: PR Review vs. Plan Review

| Dimension | Plan Review | PR Review |
|-----------|-------------|-----------|
| Input | Markdown specification document | Git diff + changed files |
| Reference points | "Location in plan" (section reference) | File:line in diff or codebase |
| Context | Plan's stated intent and design | Actual code behaviour and side effects |
| Assessment target | Design quality and completeness | Implementation quality and correctness |
| Codebase exploration | To validate plan against existing patterns | To understand context beyond the diff |
| Success criteria | "Will this plan lead to good code?" | "Is this code good?" |

### 3. Industry PR Review Tool Patterns

**Tools researched:** CodeRabbit, Qodo/PR-Agent, Sourcery, GitHub Copilot Code
Review, Greptile, Google Conductor, Microsoft's internal tool.

**Common output structure across tools:**
1. PR summary (what changed, intent, scope)
2. Walkthrough/review guide with ordered changes
3. Inline findings attached to specific diff lines
4. Severity/effort indicators
5. Actionable suggestions with concrete code fixes

**Review lenses used by major tools:**
- Sourcery: general quality, security, complexity, documentation, testing, custom
- Qodo: code feedback, security, test coverage, effort estimation, ticket
  compliance
- Google Conductor: code review, test-suite validation, guideline enforcement,
  security, compliance

**Best practices identified:**
- Limit findings per review (Qodo defaults to max 3 to reduce noise)
- Use separate prompts per lens rather than one omnibus prompt
- Rate confidence on each finding
- Focus on high-impact, likely issues not theoretical edge cases
- Ground security findings in real CVEs, not theoretical threats
- Include concrete code suggestions, not just problem descriptions

### 4. Per-Lens Review Criteria for Code

#### Architecture
- **Module boundaries**: coupling metrics, cohesion assessment, circular
  dependency detection
- **Dependency direction**: violations of the dependency rule (inner layers
  importing from outer layers)
- **Component design**: single responsibility, appropriate abstraction level,
  design pattern fitness
- **System impact**: how the change affects the broader system architecture
- **Functional core / imperative shell**: separation of business logic from
  side effects

#### Security
- **OWASP Top 10**: injection, broken access control, cryptographic failures,
  SSRF, security misconfiguration
- **Input validation**: server-side validation, allowlist-based, parameterised
  queries
- **Authentication/authorisation**: at every access point, default-deny,
  re-authentication for sensitive ops
- **Secrets management**: no hardcoded secrets, proper .gitignore, secrets
  scanning
- **Information disclosure**: error messages, logging, debug output
- **Data flow analysis**: tracing user input from entry to storage/output

#### Test Coverage
- **Coverage adequacy**: new code paths have tests, bug fixes include regression
  tests, edge cases covered
- **Test quality**: behaviour validation vs. code path exercising, assertion
  quality, mutation testing perspective
- **Test pyramid balance**: unit tests as foundation, integration for boundaries,
  minimal E2E
- **Test isolation**: determinism, no shared mutable state, no system
  dependencies
- **Anti-patterns**: assertion-free tests, implementation coupling, over-mocking,
  flaky tests, disabled tests

#### Code Quality
- **Complexity**: cyclomatic complexity (>10 flag), cognitive complexity (>15
  flag), deep nesting
- **Clean code**: SOLID principles, DRY, KISS, YAGNI, meaningful naming
- **Code smells**: god objects, feature envy, primitive obsession, long methods,
  flag arguments
- **Readability**: comments explain "why" not "what", self-documenting code,
  guard clauses over nesting
- **Error handling**: appropriate categorisation, propagation strategy, no
  swallowed exceptions

#### Standards Compliance
- **Coding conventions**: naming patterns, file organisation, import conventions
  consistent with codebase
- **API design**: RESTful conventions, HTTP semantics, consistent error format,
  versioning
- **Automated standards**: linter compliance, formatter compliance, type safety
- **Documentation**: public interfaces documented, breaking changes noted,
  migration guides for consumers

#### Usability (Developer Experience)
- **API ergonomics**: consistency, minimality, discoverability, least surprise
- **Error messages**: actionable (what/why/how-to-fix), structured format,
  contextual
- **Configuration**: sensible defaults, zero-config for common cases, validation
  at startup
- **Migration**: breaking changes identified, deprecation strategy, incremental
  upgrade path
- **Documentation**: getting started path, API reference, examples, migration
  guides

### 5. Test Coverage Lens: Deep Findings

This is the one lens without an existing plan reviewer counterpart. Key insights:

**Beyond line coverage:**
- Line coverage alone is "deeply insufficient" -- code can have 100% coverage
  with zero meaningful assertions
- The mutation testing perspective: "If I changed this `<` to `<=`, would any
  test fail?"
- Nine metrics beyond coverage: code complexity, mutation score, flaky test
  analytics, execution time, defect detection rate, fault isolation, maintenance
  effort

**Test pyramid in PR review:**
- Is the test at the right level? Business logic = unit, API contracts =
  integration, critical flows = E2E
- Is the distribution inverted? Many E2E/few unit tests = red flag
- Is there unnecessary duplication across levels?

**Red flags to detect:**
- Assertion-free tests (execute code but never verify outcomes)
- Tests tightly coupled to implementation (mocking internal methods)
- Non-deterministic elements (system time, random values, external services)
- Bug-fix PRs without regression tests
- Disabled/skipped tests alongside changes
- Over-mocking (testing mock configuration, not behaviour)

**13 anti-patterns from Codepipes:**
1. Unit tests without integration tests
2. Integration tests without unit tests
3. Wrong kind of tests for the application
4. Testing trivial code, neglecting critical logic
5. Testing internal implementation
6. Excessive attention to coverage numbers
7. Flaky or slow tests
8. Manual test execution
9. Test code as second-class citizen
10. Not converting production bugs to tests
11. Treating TDD as religion
12. Writing tests without reading documentation
13. Giving testing a bad reputation

## Agent Design Recommendations

### Naming Convention

Following the `plan-[lens]-reviewer` pattern:
- `pr-architecture-reviewer`
- `pr-security-reviewer`
- `pr-test-coverage-reviewer`
- `pr-code-quality-reviewer`
- `pr-standards-reviewer`
- `pr-usability-reviewer`

### Structural Template

Each agent should follow the plan reviewer template with these adaptations:

1. **Opening paragraph**: "You are a specialist at reviewing pull requests from a
   [LENS] perspective."

2. **Core Responsibilities**: 3 numbered items, adapted from plan reviewer
   equivalents but focused on code rather than plans

3. **Analysis Strategy**:
   - Step 1: Read and understand the PR (diff, commit messages, PR description)
   - Step 2: Explore the codebase for context (existing patterns, affected
     components)
   - Step 3+: Domain-specific evaluation (adapted from plan reviewer steps but
     applied to actual code)

4. **Output Format**: Same severity tiers (Critical/Major/Minor/Suggestions) with
   `Location in PR` replacing `Location in plan` (using file:line references)

5. **Important Guidelines**: Adapted to PR context (reference diffs, consider
   beyond-the-diff impact)

6. **What NOT to Do**: Same lens-scoping pattern, now excluding the other 5 PR
   lenses

7. **Closing "Remember:" paragraph**: Adapted to code review context

### Key Adaptations from Plan to PR Review

- **Location references**: Use `file.ext:line` instead of "Section X of the plan"
- **Codebase context**: Emphasise looking beyond the diff to understand impact
- **Concrete suggestions**: Include actual code snippets in suggestions where
  appropriate
- **Pragmatism**: Focus on high-impact issues; limit noise; distinguish blocking
  from non-blocking findings
- **Tools**: Same toolset (`Read, Grep, Glob, LS`) since agents need to read the
  diff and explore the codebase

### Per-Agent Design Notes

**pr-architecture-reviewer:**
- Adapt from plan-architecture-reviewer
- Focus on: coupling/cohesion in actual code, dependency direction violations,
  component boundary integrity, architectural drift from established patterns
- Output sections: Architectural Impact Assessment, Dependency Analysis,
  Codebase Consistency, Strengths

**pr-security-reviewer:**
- Adapt from plan-security-reviewer
- Focus on: OWASP Top 10 in code, input validation, auth/authz at every access
  point, secrets in diff, information disclosure via error handling/logging
- Output sections: Threat Assessment, OWASP Coverage, Data Flow Analysis,
  Strengths

**pr-test-coverage-reviewer:**
- New lens -- no plan reviewer counterpart
- Focus on: coverage adequacy, test quality (assertions, behaviour validation),
  test pyramid balance, isolation/determinism, anti-patterns
- Output sections: Coverage Assessment, Test Quality Assessment, Test
  Architecture Assessment, Anti-Pattern Detection, Strengths

**pr-code-quality-reviewer:**
- Adapt from plan-code-quality-reviewer
- Focus on: complexity metrics, clean code principles in actual code, code smells,
  readability, error handling patterns
- Output sections: Complexity Assessment, Design Principle Assessment, Error
  Handling Assessment, Codebase Consistency, Strengths

**pr-standards-reviewer:**
- Adapt from plan-standards-reviewer
- Focus on: naming conventions, file organisation, API design consistency, linter
  compliance, documentation provisions
- Output sections: Convention Consistency, API Standards Assessment,
  Documentation Provisions, Strengths

**pr-usability-reviewer:**
- Adapt from plan-usability-reviewer
- Focus on: API ergonomics in implementations, error message quality,
  configuration surface area, migration/compatibility impact
- Output sections: API Ergonomics Assessment, Error Experience Assessment,
  Configuration Assessment, Migration & Compatibility Assessment, Strengths

## Web Research Sources

### AI PR Review Tools
- [CodeRabbit](https://www.coderabbit.ai/) - AI PR reviewer with 8-stage pipeline
- [Qodo/PR-Agent](https://github.com/qodo-ai/pr-agent) - Open-source PR review with /describe, /review, /improve
- [Sourcery](https://www.sourcery.ai/) - Multiple specialised reviewers
- [Greptile](https://www.greptile.com) - Graph-based codebase-aware review
- [Google Conductor](https://developers.googleblog.com/conductor-update-introducing-automated-reviews/) - Post-implementation review
- [GitHub Copilot Code Review](https://docs.github.com/en/copilot/concepts/agents/code-review)
- [awesome-reviewers](https://github.com/baz-scm/awesome-reviewers) - 8000+ review prompts

### Per-Lens Code Review Criteria
- [Google Engineering Practices: What to Look For](https://google.github.io/eng-practices/review/reviewer/looking-for.html)
- [OWASP Secure Code Review Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secure_Code_Review_Cheat_Sheet.html)
- [Pragmatic Engineer: Good Code Reviews, Better Code Reviews](https://blog.pragmaticengineer.com/good-code-reviews-better-code-reviews/)
- [Axolo: Code Review Security Checklist](https://axolo.co/blog/p/code-review-security-checklist)
- [Nerdify: 8 Pillars of Code Review](https://getnerdify.com/blog/code-review-checklist/)

### Test Coverage Deep Dive
- [Martin Fowler: The Practical Test Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html)
- [Codepipes: Software Testing Anti-patterns](https://blog.codepipes.com/testing/software-testing-antipatterns.html)
- [Codecov: Measuring Test Suite Effectiveness](https://about.codecov.io/blog/measuring-the-effectiveness-of-test-suites-beyond-code-coverage-metrics/)
- [Google: Practical Mutation Testing at Scale](https://homes.cs.washington.edu/~rjust/publ/practical_mutation_testing_tse_2021.pdf)
- [Stack Overflow Blog: Coverage Paradox](https://stackoverflow.blog/2025/12/22/making-your-code-base-better-will-make-your-code-coverage-worse/)
- [Microsoft .NET: Unit Testing Best Practices](https://learn.microsoft.com/en-us/dotnet/core/testing/unit-testing-best-practices)
- [Diffblue: Better Unit Test Assertions](https://www.diffblue.com/resources/how-to-write-better-unit-test-assertions/)

### Prompt Engineering for Code Review
- [CrashOverride: Prompting LLM Security Reviews](https://crashoverride.com/blog/prompting-llm-security-reviews)
- [Graphite: Effective Prompt Engineering for AI Code Reviews](https://graphite.com/guides/effective-prompt-engineering-ai-code-reviews)
- [Addy Osmani: Code Review in the Age of AI](https://addyo.substack.com/p/code-review-in-the-age-of-ai)

## Code References

- `/Users/tobyclemson/.claude/agents/plan-architecture-reviewer.md` - Architecture lens template
- `/Users/tobyclemson/.claude/agents/plan-security-reviewer.md` - Security lens template
- `/Users/tobyclemson/.claude/agents/plan-code-quality-reviewer.md` - Code quality lens template
- `/Users/tobyclemson/.claude/agents/plan-standards-reviewer.md` - Standards lens template
- `/Users/tobyclemson/.claude/agents/plan-usability-reviewer.md` - Usability lens template
- `/Users/tobyclemson/.claude/agents/codebase-analyser.md` - General-purpose agent pattern
- `/Users/tobyclemson/.claude/commands/review-plan.md` - Orchestrating command for plan review
- `/Users/tobyclemson/.claude/commands/describe-pr.md` - Existing PR interaction patterns

## Architecture Insights

The existing agent system has a clear layered architecture:

1. **Agent layer**: Individual specialist agents with focused responsibilities
   and read-only codebase access
2. **Command layer**: Orchestrating commands (`/review-plan`, `/describe-pr`)
   that spawn agents in parallel and synthesise findings
3. **Convention layer**: Shared output formats, severity tiers, confidence
   ratings, and scoping patterns that enable synthesis

The PR review agents should follow this same architecture. A new `/review-pr`
command would be needed to orchestrate the six PR review agents, similar to how
`/review-plan` orchestrates the five plan review agents.

## Decisions

1. **PR diff delivery to agents**: The orchestrating command (`/review-pr`)
   fetches the diff once and writes it to a randomised temp directory using
   `mktemp -d /tmp/pr-review-{number}-XXXXXXXX`. This produces paths like
   `/tmp/pr-review-123-a1b2c3d4/`. The directory contains:
   - `diff.patch` - full PR diff
   - `changed-files.txt` - list of changed file paths
   - `pr-description.md` - the PR description/body
   - `commits.txt` - commit message headlines

   The temp directory path is passed to each agent in their prompt. Agents use
   `Read` to access the diff and changed files list, then use `Read`, `Grep`,
   `Glob` to explore the actual codebase for context beyond the diff.

2. **Hybrid diff access strategy**: Agents receive both the full diff
   (`diff.patch`) and the changed files list (`changed-files.txt`) in the temp
   directory. The changed files list includes per-file change stats and acts as
   a table of contents. Agents use their judgement: for small PRs, read the full
   diff; for large PRs, use the changed files list to prioritise reading files
   most relevant to their lens directly from the codebase via `Read`. Agent
   prompts include guidance like "for PRs with many changed files, prioritise
   reading files most relevant to your lens rather than consuming the full diff."

3. **Create a `/review-pr` orchestrating command**: Following the pattern of
   `/review-plan`, a new `/review-pr` command will orchestrate lens selection,
   parallel agent spawning, finding synthesis, and compiled review presentation.
   It will use the existing commands (`/review-plan`, `/describe-pr`) as
   inspiration for structure and conventions.

## Open Questions

4. **Hybrid location references**: Findings use file line numbers as the primary
   reference (consistent with existing agent conventions) and annotate whether
   the line was changed in the PR or is existing code. Format:
   - `**Location**: \`src/auth/handler.ts:42\` (changed in PR)`
   - `**Location**: \`src/auth/handler.ts:15\` (existing code, not changed)`

   This gives navigability, supports beyond-the-diff findings (e.g., a security
   reviewer flagging that existing auth is bypassed by new code paths), and makes
   it clear whether the finding relates to new or pre-existing code.

5. **Single test coverage lens**: Test adequacy and test quality remain as one
   combined lens (`pr-test-coverage-reviewer`) rather than being split. This
   maintains six-agent symmetry and can be revisited if the lens proves too
   broad in practice.
