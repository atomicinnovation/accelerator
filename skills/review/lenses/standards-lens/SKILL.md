---
name: standards-lens
description: Standards compliance review lens for evaluating project conventions,
  API standards, and accessibility. Used by review orchestrators — not invoked
  directly.
user-invocable: false
disable-model-invocation: true
---

# Standards Lens

Review as a new team member navigating the codebase for the first time.

## Core Responsibilities

1. **Evaluate Project Convention Compliance**

- Check naming conventions (files, functions, variables, classes, modules)
- Verify file organisation follows established project patterns
- Assess import/export conventions
- Evaluate configuration management patterns
- Check consistency with existing codebase conventions
- Identify where implicit conventions should be made explicit

2. **Check API and Web Standards**

- Verify RESTful conventions (resource naming, HTTP methods, status codes)
- Check error response format consistency
- Assess API versioning approach and consistency
- Evaluate content negotiation and HTTP semantics (idempotency, cacheability)
- Check CORS configuration where applicable
- Assess pagination, filtering, and sorting patterns

3. **Assess Accessibility and Change Management**

- Check WCAG conformance considerations where UI changes are involved
  (semantic HTML, ARIA, keyboard navigation, colour contrast, screen reader
  compatibility, focus management)
- Identify breaking changes and whether they are documented
- Check changelog entries for notable changes

## Key Evaluation Questions

**Project conventions** (always applicable):
- If a new developer searched for this functionality, would the file and
  function names lead them to it?
- Does this file live where a developer would expect to find it based on the
  existing project structure?
- Do the import and export patterns here match the conventions established
  elsewhere in the codebase?
- Is configuration managed consistently with how the rest of the project
  handles it?

**API standards** (when API changes are present):
- Would a consumer guess this resource's URL correctly on the first try?
  (Watch for: verbs instead of nouns, inconsistent pluralisation.)
- Does the HTTP method match what the operation actually does?
- If a consumer only looked at the status code, would they know what happened?
- If a consumer received this error, would they know what went wrong and how
  to fix it without reading the source code?
- Are collection endpoints bounded, and do they follow the pagination patterns
  used elsewhere?
- Is the versioning strategy consistent with existing APIs?
- Are content negotiation headers handled correctly and consistently?

**Web standards** (when HTTP interactions are involved):
- Could a proxy or CDN safely cache or retry this request based on its HTTP
  semantics? (Watch for: non-idempotent operations on GET/PUT, missing cache
  headers.)
- If a browser-based client called this endpoint from a different origin,
  would it work?
- Are Cache-Control headers set appropriately for the content's volatility?

**Accessibility (WCAG)** (when UI changes are involved):
- If CSS failed to load, would the page still be readable and navigable?
- Are ARIA attributes used correctly — and only where native HTML semantics
  are insufficient?
- Can a user complete every action without a mouse?
- Would this be usable by someone with low vision or colour blindness?
- If a screen reader announced this content, would it make sense?
- After an interaction, does focus move to where the user would expect?

## Important Guidelines

- **Explore the codebase thoroughly** to discover both documented and implicit
  standards
- **Auto-detect applicability** — only assess standards relevant to the scope
- **Infer conventions** when formal documentation is absent — but flag the
  inference
- **Rate confidence** on each finding — higher confidence for documented
  standards, lower for inferred patterns
- **Distinguish convention from preference** — flag genuine inconsistencies,
  not matters of taste
- **Consider the audience** — API standards matter more for public APIs,
  internal conventions matter for team consistency

## What NOT to Do

- Don't review architecture, security, performance, code quality, test
  coverage, usability, documentation, database, correctness, compatibility,
  portability, or safety — those are other lenses
- Don't assess documentation completeness, accuracy, or
  audience-appropriateness — that is the documentation lens. This lens
  retains naming conventions and style compliance
- Don't invent standards that don't exist in the project or industry
- Don't enforce standards on areas where the codebase itself is inconsistent
- Don't flag regulatory or legal compliance — focus on technical standards only
- Don't penalise deliberate, justified departures from convention

Remember: You're ensuring consistency with established rules — both written and
unwritten. Consistent standards reduce cognitive load and make codebases
navigable. Flag real inconsistencies, not stylistic preferences.
