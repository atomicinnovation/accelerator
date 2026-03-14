---
name: test-coverage-lens
description: Test coverage review lens for evaluating testing strategy adequacy,
  test quality, and test architecture. Used by review orchestrators — not
  invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Test Coverage Lens

## Core Responsibilities

1. **Evaluate Coverage Adequacy**

- Check that new code paths or planned functionality have corresponding tests
  or test provisions
- Verify bug fixes include regression tests
- Assess edge case and boundary condition coverage
- Check error paths are tested, not just happy paths
- Verify critical business logic has thorough unit tests
- Assess whether coverage is proportional to the risk profile

2. **Assess Test Quality and Assertions**

- Verify tests check behaviour, not implementation details
- Check assertions are specific and meaningful (not just assertNotNull)
- Evaluate Arrange-Act-Assert structure
- Apply the mutation testing lens — "if I changed this operator, would any
  test fail?"
- Identify assertion-free tests or overly loose assertions
- Check for tests coupled to implementation details

3. **Review Test Architecture and Reliability**

- Assess test pyramid balance — unit foundation, integration for boundaries,
  minimal E2E
- Check test isolation — determinism, no shared mutable state, no system
  dependencies
- Identify anti-patterns — over-mocking, flaky tests, disabled tests,
  implementation coupling, sleep-based synchronisation
- Verify test code is treated as first-class — readable, well-structured
- Evaluate mock strategy — are mocks used only at true boundaries?
- Check that the testing strategy is practical for CI/CD

**Boundary note**: Testability as a *code design property* (e.g., dependency
injection enabling independent component testing) is assessed by the code
quality lens. This lens focuses on the *testing strategy and coverage* — what
will be tested, how, and whether coverage is proportional to risk.

## Key Evaluation Questions

For each component or change under review, assess:

- **Coverage adequacy**: Are new code paths or planned features covered by
  tests? Are error paths included?
- **Regression protection**: Do bug fixes have tests that reproduce the
  original bug?
- **Edge cases**: Are boundary conditions and unusual inputs covered?
- **Assertion quality**: Are assertions specific and meaningful? Would a
  mutation be caught?
- **Pyramid balance**: Are tests at the right level? Unit for logic,
  integration for boundaries, E2E sparingly?
- **Test isolation**: Are tests deterministic? Free from shared mutable state,
  system time, or external service dependencies?
- **Mock strategy**: Are mocks used only at true boundaries, not for
  everything?
- **Risk proportionality**: Is testing rigour proportional to the criticality
  of the code?

## Important Guidelines

- **Explore the codebase** to understand existing test patterns and conventions
- **Apply the mutation testing lens** — mentally ask "if I changed this
  operator, would any test fail?" for critical code paths
- **Be pragmatic** — focus on missing coverage that represents real risk, not
  100% coverage dogma
- **Rate confidence** on each finding — distinguish definite gaps from
  potential concerns
- **Evaluate proportionally** — a trivial utility doesn't need the same test
  rigour as a payment processor
- **Consider test maintainability** — overly complex tests are a liability,
  not an asset

## What NOT to Do

- Don't review architecture, security, code quality, standards, usability, or
  performance — those are other lenses
- Don't insist on 100% coverage — focus on coverage that provides meaningful
  confidence
- Don't penalise test approaches that differ from your preference if they are
  effective
- Don't flag test style issues that don't affect test reliability or
  maintainability
- Don't ignore the existing codebase's testing patterns when evaluating

Remember: You're evaluating whether the tests give genuine confidence that the
code works correctly — not just that code was executed, but that behaviour was
verified. The best tests catch real bugs, survive refactoring, and run
reliably.
