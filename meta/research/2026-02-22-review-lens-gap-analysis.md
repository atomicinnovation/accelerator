---
date: "2026-02-22T23:01:34+00:00"
researcher: Toby Clemson
topic: "Gap analysis of review lenses used by review-pr and review-plan skills"
tags: [research, review-lenses, review-pr, review-plan, gap-analysis]
status: complete
last_updated: "2026-02-22"
last_updated_by: Toby Clemson
---

# Research: Review Lens Gap Analysis

**Date**: 2026-02-22T23:01:34+00:00
**Researcher**: Toby Clemson

## Research Question

Are there gaps in the set of review lenses used by `review-pr` and `review-plan`,
and what additional lenses should be added?

## Summary

The current review system uses 6 lenses: architecture, security, test-coverage,
code-quality, standards, and usability. These provide strong coverage across most
review dimensions recommended by industry frameworks (Google engineering
practices, OWASP, comprehensive code review checklists). The most significant gap
is **performance** — a dimension that is universally recommended by code review
frameworks but is not covered by any existing lens. A secondary gap exists around
**resilience/reliability**, though this is partially addressed by the
architecture lens. Other commonly cited dimensions (documentation, observability,
concurrency) are adequately covered by existing lenses.

## Detailed Findings

### Current Lens Inventory

The review system has 6 lenses, each defined as a skill under
`~/.claude/skills/<name>-lens/SKILL.md`:

| Lens | Skill Directory | Core Domain |
|------|----------------|-------------|
| Architecture | `architecture-lens` | Modularity, coupling, dependency direction, evolutionary fitness, scalability |
| Security | `security-lens` | OWASP Top 10, STRIDE analysis, auth/authz, secrets, data flows, GDPR/CCPA |
| Test Coverage | `test-coverage-lens` | Coverage adequacy, assertion quality, test pyramid, anti-patterns |
| Code Quality | `code-quality-lens` | Complexity, design principles, error handling, observability, code smells |
| Standards | `standards-lens` | Project conventions, API standards, naming, accessibility (WCAG), documentation |
| Usability | `usability-lens` | Developer experience, API ergonomics, configuration, migration paths |

Both `review-pr` and `review-plan` use the same 6 lenses. Each lens skill is
context-agnostic — the review type (PR vs plan) is determined by the output
format skill and the orchestrator's prompt, not the lens itself.

### Industry-Recommended Review Dimensions

#### Google Engineering Practices

Google's "What to look for in a code review" document identifies these
dimensions:

1. **Design** — Do interactions make sense? Do changes integrate well?
2. **Functionality** — Does code achieve intent? Edge cases handled?
3. **Complexity** — Can it be understood quickly? Over-engineering?
4. **Tests** — Appropriate, valid, will fail when code breaks?
5. **Naming** — Sufficiently descriptive?
6. **Comments** — Explain why, not what?
7. **Style** — Adherence to style guides?
8. **Consistency** — Alignment with existing patterns?
9. **Documentation** — READMEs, guides updated?

Source: [Google Engineering Practices](https://google.github.io/eng-practices/review/reviewer/looking-for.html)

#### Comprehensive Code Review Checklists

Aggregated from multiple industry sources, the commonly cited dimensions are:

1. Functionality & Correctness
2. Readability & Maintainability
3. **Performance & Efficiency** (significant gap)
4. Testing
5. Security
6. Design & Architecture
7. Documentation

Sources:
- [Swimm Code Review Checklist](https://swimm.io/learn/code-reviews/ultimate-10-step-code-review-checklist)
- [GetDX Code Review Checklist](https://getdx.com/blog/code-review-checklist/)
- [Axify Code Review Checklist](https://axify.io/blog/code-review-checklist)

#### Design Document / Plan Review Checklists

From architectural design review frameworks:

1. Clean Code & Code Style
2. Security
3. **Performance** (gap)
4. **Logging and Tracing** (partially covered by code-quality)
5. **Concurrency** (gap — no dedicated coverage)
6. Error Handling
7. Maintainability & Testability
8. Domain/Business
9. Architecture
10. **Scalability** (partially covered by architecture)
11. **Reliability & Resiliency** (partially covered by architecture)
12. Design Patterns

Sources:
- [Medium: Code and System Design Review Checklist](https://medium.com/@azomshahriar05/code-and-system-design-review-checklist-aade2bb1d8dc)
- [Smartsheet Design Review Templates](https://www.smartsheet.com/content/design-review-checklist-templates)
- [TOGAF Architecture Review Checklist](https://www.opengroup.org/architecture/togaf7-doc/arch/p4/comp/clists/syseng.htm)

#### Emerging Dimensions

- **Data Privacy / AI Governance** — EU AI Act (August 2026), GDPR enforcement
  acceleration, AI governance responsibilities. Currently partially covered by
  the security lens.
- **Cost Optimization / Sustainability** — Green FinOps, cloud resource
  efficiency. Niche concern, probably too specialized for a general review
  framework.

### Gap Analysis

#### Coverage Map

| Industry Dimension | Current Lens Coverage | Gap Status |
|---|---|---|
| Design / Architecture | Architecture lens | Covered |
| Functionality / Correctness | Not explicitly covered (implicit) | Minor gap — see below |
| Complexity / Readability | Code Quality lens | Covered |
| Testing | Test Coverage lens | Covered |
| Security | Security lens | Covered |
| Naming / Style / Consistency | Standards lens | Covered |
| Documentation | Standards lens | Covered |
| Developer Experience / Migration | Usability lens | Covered |
| Accessibility (WCAG) | Standards lens | Covered |
| Error Handling | Code Quality lens | Covered |
| Observability / Logging | Code Quality lens | Covered |
| **Performance / Efficiency** | **Not covered** | **Primary gap** |
| **Concurrency / Thread Safety** | **Not covered** | **Secondary gap** |
| **Resilience / Reliability** | Architecture lens (partial) | **Secondary gap** |
| Data Privacy / Compliance | Security lens (partial) | Adequate |
| Cost / Sustainability | Not covered | Not needed |
| Functionality / Correctness | Not covered | See assessment below |

### Recommended New Lens: Performance

**Priority: High — this is the most significant gap.**

Performance is universally cited as a primary code review dimension by Google,
OWASP, and every comprehensive checklist surveyed. No existing lens adequately
covers performance concerns at the code level.

**What the architecture lens covers** (and what it doesn't):
- Architecture covers scalability at the system level ("What happens under 10x
  load?") and failure modes
- Architecture does NOT cover code-level performance: algorithmic complexity,
  resource usage, database query efficiency, caching patterns, I/O optimization

**Proposed scope for a performance lens:**

1. **Algorithmic Complexity**
   - Time and space complexity of algorithms and data structures
   - Unnecessary iteration (nested loops, redundant passes)
   - Appropriate data structure selection (map vs list, set vs array)

2. **Resource Efficiency**
   - Memory allocations and leaks (object creation in hot paths, unbounded
     caches)
   - Connection and resource pool management
   - File handle and descriptor management

3. **Database and Query Performance**
   - N+1 query patterns
   - Missing indexes or inefficient queries
   - Batch vs individual operations
   - Query result set size and pagination

4. **Caching Strategy**
   - What to cache, where, and for how long
   - Cache invalidation strategy
   - Cache stampede / thundering herd prevention

5. **I/O and Network Efficiency**
   - Payload size and compression
   - Lazy loading vs eager loading
   - Batching and streaming
   - Connection reuse

6. **Concurrency and Thread Safety**
   - Race conditions and data races
   - Deadlock potential
   - Lock granularity and contention
   - Thread pool sizing
   - Async/await correctness

7. **Hot Path Optimization**
   - Identifying critical paths that execute frequently
   - Unnecessary work in request handling paths
   - Startup time impact

**Auto-detect relevance criteria:**
- **For PRs**: Relevant when changes involve data processing, database queries,
  API endpoints handling load, algorithm-heavy code, concurrency primitives,
  caching, or hot paths. Skip for documentation-only, configuration-only, or
  simple UI changes.
- **For Plans**: Relevant when the plan involves data processing pipelines,
  database interactions, high-throughput APIs, concurrent processing, or
  caching strategy. Skip for documentation-only or trivial changes.

### Assessment of Secondary Gaps

#### Resilience / Reliability (Moderate Priority)

The architecture lens partially covers this with questions like "What happens
when a component fails?" and "scalability & resilience." However, a dedicated
resilience perspective would systematically evaluate:

- Retry strategies and backoff policies
- Circuit breaker patterns
- Graceful degradation
- Timeout handling (both setting and propagating)
- Idempotency guarantees
- Error recovery and compensation
- Failover mechanisms
- Health check design

**Recommendation**: Rather than a full new lens, consider extending the
architecture lens with a "Resilience Evaluation" section under Core
Responsibilities. This keeps the lens count manageable while covering the gap.
If resilience reviews become a frequent need, it could be promoted to a
standalone lens later.

#### Functionality / Correctness (Low Priority)

Google's engineering practices lists "Functionality" as a primary review
dimension — does the code do what the author intended? This is a meta-concern
rather than a specialized lens. Every reviewer implicitly checks correctness.
Making it an explicit lens could lead to overlap with test-coverage (edge cases)
and code-quality (error handling).

**Recommendation**: Do not add a standalone lens. Correctness is a cross-cutting
concern best left implicit. If needed, add a brief reminder to the reviewer
agent's behavioural conventions.

## Steps to Add a New Lens

Based on the existing codebase patterns, adding a lens requires changes in 4
locations:

### 1. Create the Lens Skill

Create `~/.claude/skills/<lens-name>-lens/SKILL.md` following the established
pattern:

```yaml
---
name: <lens-name>-lens
description: <Lens name> review lens for evaluating <domain>. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---
```

Followed by markdown sections:
- `# <Lens Name> Lens`
- `## Core Responsibilities` (3 numbered responsibility groups)
- `## Key Evaluation Questions` (bulleted list per dimension)
- `## Important Guidelines` (pragmatic review guidance)
- `## What NOT to Do` (boundary setting with other lenses)

Every lens skill explicitly lists which other lenses exist and states "Don't
review [X, Y, Z] — those are other lenses" to prevent overlap.

### 2. Update review-pr Skill

Edit `~/.claude/skills/review-pr/SKILL.md`:

- Add the lens to the **Available Review Lenses** table (line ~48)
- Add auto-detect relevance criteria in **Step 2** (line ~113)

### 3. Update review-plan Skill

Edit `~/.claude/skills/review-plan/SKILL.md`:

- Add the lens to the **Available Review Lenses** table (line ~42)
- Add auto-detect relevance criteria in **Step 2** (line ~76)

### 4. Update Output Format Skills

Edit both output format skills to add the new lens identifier:

- `~/.claude/skills/pr-review-output-format/SKILL.md` — add to the lens
  identifier examples in the Field Reference (line ~50)
- `~/.claude/skills/plan-review-output-format/SKILL.md` — add to the lens
  identifier examples in the Field Reference (line ~39)

### 5. Update Existing Lens "What NOT to Do" Sections

Each existing lens lists "Don't review [other lenses]" in its What NOT to Do
section. Update all 6 existing lens skills to include the new lens in their
exclusion list. Files to update:

- `~/.claude/skills/architecture-lens/SKILL.md`
- `~/.claude/skills/security-lens/SKILL.md`
- `~/.claude/skills/test-coverage-lens/SKILL.md`
- `~/.claude/skills/code-quality-lens/SKILL.md`
- `~/.claude/skills/standards-lens/SKILL.md`
- `~/.claude/skills/usability-lens/SKILL.md`

### 6. Verify Agent Type Registration

The system has agent types like `pr-code-quality-reviewer` and
`plan-architecture-reviewer` that appear to be auto-derived from the reviewer
agent combined with lens skills. After adding a new lens, verify that
corresponding `pr-<lens>-reviewer` and `plan-<lens>-reviewer` agent types
become available. If they require manual registration, add them to whatever
configuration defines these types.

### Boundary Considerations When Adding Performance Lens

Some performance concerns currently touch other lenses. To avoid overlap:

- **Architecture lens** currently covers "scalability & resilience" and "What
  happens under 10x load?" — the architecture lens should retain system-level
  scalability concerns while the performance lens handles code-level efficiency
- **Code Quality lens** covers "observability" including "metrics collection"
  — performance metrics (latency histograms, throughput counters) should be
  mentioned in the performance lens as "what to measure" but the code-quality
  lens retains ownership of observability infrastructure
- **Test Coverage lens** covers "test reliability" — performance test strategy
  (load testing, benchmarks) could be mentioned in the performance lens as
  guidance, but test-coverage retains ownership of test strategy

## Architecture Insights

The review system's architecture is well-designed for extensibility:

- **Lens-agnostic reviewer agent**: The single `reviewer.md` agent reads its
  lens skill at runtime. Adding a lens doesn't require a new agent definition.
- **Shared output formats**: The `pr-review-output-format` and
  `plan-review-output-format` skills define output schemas that are
  lens-agnostic. No format changes needed for a new lens.
- **Orchestrator-driven selection**: The review-pr and review-plan skills
  handle lens selection and auto-detection. A new lens only needs its relevance
  criteria added to the selection logic.
- **Parallel execution**: All lenses run as concurrent agent tasks. Adding a
  7th lens adds marginal latency (one more parallel task) but no serial
  overhead.

The cost of adding a new lens is low: one new skill file plus minor edits to
4-8 existing files.

## Code References

- `~/.claude/skills/review-pr/SKILL.md` — PR review orchestrator
- `~/.claude/skills/review-plan/SKILL.md` — Plan review orchestrator
- `~/.claude/agents/reviewer.md` — Generic reviewer agent
- `~/.claude/skills/architecture-lens/SKILL.md` — Architecture lens
- `~/.claude/skills/security-lens/SKILL.md` — Security lens
- `~/.claude/skills/test-coverage-lens/SKILL.md` — Test coverage lens
- `~/.claude/skills/code-quality-lens/SKILL.md` — Code quality lens
- `~/.claude/skills/standards-lens/SKILL.md` — Standards lens
- `~/.claude/skills/usability-lens/SKILL.md` — Usability lens
- `~/.claude/skills/pr-review-output-format/SKILL.md` — PR output format
- `~/.claude/skills/plan-review-output-format/SKILL.md` — Plan output format

## Open Questions

1. **How are `pr-*-reviewer` and `plan-*-reviewer` agent types registered?**
   These appear in the system's available agent types but no corresponding
   definition files were found in `~/.claude/agents/`. Understanding their
   registration mechanism is needed to ensure a new lens's agent types are
   correctly created.

2. **Should resilience be a standalone lens or an extension?** If resilience
   reviews are frequently needed, a standalone lens provides clearer separation.
   If they're occasional, extending the architecture lens avoids lens
   proliferation.

3. **Performance lens boundary with architecture**: The line between "system
   scalability" (architecture) and "code-level performance" (proposed
   performance lens) needs to be clearly drawn to avoid reviewer confusion and
   overlapping findings.
