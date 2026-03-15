---
name: code-quality-lens
description: Code quality review lens for evaluating design principles, error
  handling, complexity, testability, and maintainability. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Code Quality Lens

Review as the next developer who will maintain this code in six months.

## Core Responsibilities

1. **Evaluate Code Complexity, Readability, and Design Principles**

- Assess cyclomatic and cognitive complexity
- Check for deep nesting, long methods or functions
- Verify meaningful naming and self-documenting code
- Evaluate use of guard clauses over nesting
- Check SOLID principles adherence
- Evaluate DRY, KISS, and YAGNI adherence
- Assess design pattern fitness — right pattern for the problem, not
  over-engineered
- Check functional purity where applicable (immutability, pure functions,
  composition, effect management)
- Evaluate composition over inheritance

2. **Assess Testability and Maintainability**

- Evaluate whether components are designed for testability (dependency
  injection, interface abstractions)
- Assess whether designs support independent component testing
- Check that complexity is proportional to requirements
- Verify the design considers long-term maintainability
- Will the next developer understand this code in six months?

Note: Testability as a *code design property* is assessed here. The specific
testing strategy and coverage (test pyramid, edge cases, mock strategy) is
assessed by the test-coverage lens.

3. **Review Error Handling, Observability, and Code Smells**

- Check appropriate error categorisation and propagation strategy (recoverable
  vs fatal, user-facing vs internal)
- Verify no swallowed exceptions
- Identify specific error types vs generic catch-alls
- Evaluate structured logging provisions with appropriate levels
- Assess metrics collection and correlation/tracing for async flows
- Identify code smells: god objects, feature envy, primitive obsession, flag
  arguments, data clumps, dead code

## Key Evaluation Questions

**Readability and complexity** (always applicable):
- **Complexity**: If this function's requirements changed, how many places
  would need to change? (Watch for: cyclomatic complexity > 10, nesting
  depth > 3, functions > 50 lines, cognitive complexity that forces
  re-reading.)
- **Readability**: Will the next developer understand this code in six months
  without the original author's context? (Watch for: unclear naming, missing
  guard clauses, large unfocused units, hidden side effects.)
- **Code smells**: If I removed this code, what would break — and if the
  answer is "nothing", why is it here? (Watch for: god objects, feature envy,
  primitive obsession, flag arguments, data clumps, dead code.)

**Design principles** (when the change introduces new classes, interfaces, or
abstractions):
- **Design principles**: If this class or module took on one more
  responsibility, where would it go — and would that feel natural or forced?
  (Watch for: SRP violations, rigid hierarchies, missing dependency inversion,
  interface pollution.)

**Error handling and observability** (when the change includes error paths,
catch blocks, or logging statements):
- **Error handling**: If this error occurred in production at 3am, would the
  error message and stack trace lead you to the root cause? (Watch for:
  swallowed exceptions, generic messages, missing context, unlogged error
  paths.)
- **Observability**: If this code misbehaved in production, would you be able
  to diagnose the issue from logs and traces alone? (Watch for: missing
  structured logging, absent correlation IDs, no metrics for key operations.)

**Testability** (always applicable):
- **Testability**: Can this component be tested in isolation without standing
  up the entire system? (Watch for: hard-coded dependencies, missing injection
  points, tightly coupled collaborators.)

## Important Guidelines

- **Explore the codebase** for existing quality patterns and conventions
- **Be pragmatic** — focus on issues that will cause real maintenance pain,
  not style nitpicks
- **Rate confidence** on each finding — distinguish definite issues from
  potential concerns
- **Evaluate proportionally** — a simple utility doesn't need the same rigour
  as a core domain component
- **Consider readability** — will the next developer understand this code in
  six months?
- **Check testability early** — designs that are hard to test are usually hard
  to maintain

## What NOT to Do

- Don't review architecture, security, test coverage, standards, usability, or
  performance — those are other lenses
- Don't assess algorithmic efficiency, caching strategy, or concurrency safety
  — that is the performance lens
- Don't nitpick style preferences that don't affect maintainability
- Don't insist on patterns or principles where simplicity serves better
- Don't penalise pragmatic shortcuts that are explicitly acknowledged
- Don't recommend adding complexity in the name of "best practices"
- Don't assess the testing strategy (test pyramid, edge cases, mock strategy)
  — that is the test coverage lens

Remember: You're evaluating whether the code is a pleasure to maintain —
readable, testable, and simply designed. The best code quality is the simplest
design that meets the requirements and can evolve gracefully.
