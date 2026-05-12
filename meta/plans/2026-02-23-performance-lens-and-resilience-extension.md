# Performance Lens & Resilience Extension Implementation Plan

## Overview

Add a new **performance lens** to the review system and extend the existing
**architecture lens** with resilience/reliability coverage. The performance lens
closes the most significant gap identified by the review lens gap analysis — no
existing lens covers code-level performance concerns (algorithmic complexity,
resource efficiency, database queries, caching, concurrency, I/O). The
architecture lens extension adds systematic resilience evaluation (retry
strategies, circuit breakers, graceful degradation, timeouts) that is currently
only superficially covered.

## Current State Analysis

The review system has 6 lenses:

| Lens | Skill | Core Domain |
|------|-------|-------------|
| Architecture | `architecture-lens` | Modularity, coupling, evolutionary fitness |
| Security | `security-lens` | OWASP, STRIDE, auth, secrets, data flows |
| Test Coverage | `test-coverage-lens` | Coverage, assertions, pyramid, anti-patterns |
| Code Quality | `code-quality-lens` | Complexity, design principles, error handling |
| Standards | `standards-lens` | Conventions, API standards, accessibility, docs |
| Usability | `usability-lens` | DX, API ergonomics, configuration, migration |

Each lens is a skill at `~/.claude/skills/<name>-lens/SKILL.md`. Both
`review-pr` and `review-plan` share the same lenses. The generic `reviewer`
agent reads the lens skill at spawn time.

### Key Discoveries:

- Each lens follows a consistent structure: frontmatter, Core Responsibilities
  (3 numbered groups), Key Evaluation Questions, Important Guidelines, What NOT
  to Do (`architecture-lens/SKILL.md:1-85`)
- Every lens's "What NOT to Do" lists the other 5 lenses explicitly
  (e.g., `architecture-lens/SKILL.md:73-74`)
- Review orchestrators have an Available Review Lenses table and auto-detect
  relevance criteria in Step 2 (`review-pr/SKILL.md:47-55`, `review-pr/SKILL.md:113-128`)
- Output format skills list lens identifiers as examples
  (`pr-review-output-format/SKILL.md:50-51`, `plan-review-output-format/SKILL.md:39-40`)
- Architecture lens currently has a brief "Scalability & resilience" question
  (`architecture-lens/SKILL.md:48-49`) but no systematic resilience evaluation

## Desired End State

After implementation:

- The review system has **7 lenses** (6 existing + performance)
- The architecture lens has **4 Core Responsibilities** (3 existing + resilience)
- Both `review-pr` and `review-plan` include Performance in their lens tables
  and auto-detection logic
- All 7 lens skills correctly reference the other 6 in their "What NOT to Do"
- Output format skills list `"performance"` alongside existing identifiers

### Verification:

- All 7 lens skills exist and follow the established structure
- `review-pr/SKILL.md` and `review-plan/SKILL.md` list 7 lenses in their tables
- Every lens's "What NOT to Do" lists exactly 6 other lenses
- The performance lens clearly delineates boundaries with architecture
  (system-level scalability stays in architecture, code-level efficiency is
  performance) and code-quality (observability infrastructure stays in
  code-quality, performance metrics/what-to-measure is performance)
- The architecture lens's new resilience section covers retry strategies, circuit
  breakers, graceful degradation, timeout handling, idempotency, error recovery,
  and health checks

## What We're NOT Doing

- Not creating a standalone resilience lens — extending architecture instead
- Not changing the reviewer agent or output format schemas — they're
  lens-agnostic
- Not changing the spawn pattern (stays as `subagent_type: "reviewer"`)
- Not modifying any review process logic (aggregation, deduplication, etc.)
- Not adding functionality/correctness as a lens — it's a cross-cutting concern

## Implementation Approach

Work phase-by-phase, starting with the new lens skill (the largest new content),
then extending architecture, then wiring changes into orchestrators and support
files. Each phase is independently verifiable.

---

## Phase 1: Create Performance Lens Skill

### Overview

Create the new performance lens skill following the established pattern.

### Changes Required:

#### 1. Performance Lens Skill

**File**: `~/.claude/skills/performance-lens/SKILL.md` (new file)
**Changes**: Create with full lens content

```markdown
---
name: performance-lens
description: Performance review lens for evaluating algorithmic efficiency,
  resource usage, and concurrency safety. Used by review orchestrators — not
  invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Performance Lens

## Core Responsibilities

1. **Evaluate Algorithmic Efficiency and Data Structure Selection**

- Assess time and space complexity of algorithms and data structures
- Identify unnecessary iteration (nested loops, redundant passes, repeated
  lookups)
- Check data structure fitness — maps vs lists for lookups, sets vs arrays for
  membership, appropriate use of indexes
- Evaluate sorting and searching strategies for dataset sizes involved
- Identify opportunities to reduce work (early returns, short-circuiting,
  memoisation)

2. **Assess Resource Efficiency and I/O Performance**

- Check for memory allocation patterns in hot paths (object creation in loops,
  unbounded caches, string concatenation in loops)
- Evaluate connection and resource pool management (database connections, HTTP
  clients, file handles)
- Identify N+1 query patterns and missing batch operations
- Assess query efficiency (missing indexes, unbounded result sets, missing
  pagination)
- Evaluate I/O patterns (lazy vs eager loading, streaming vs buffering, payload
  size, compression)
- Check for unnecessary network round-trips and missing connection reuse

3. **Review Concurrency Safety and Caching Strategy**

- Identify race conditions, data races, and shared mutable state
- Assess lock granularity and contention potential
- Check for deadlock risk in lock ordering
- Evaluate async/await correctness (missing awaits, unnecessary serialisation
  of independent operations)
- Assess caching strategy — what to cache, invalidation approach, TTL
  appropriateness
- Check for cache stampede / thundering herd potential
- Evaluate thread pool and worker pool sizing

**Boundary note**: System-level scalability (how the architecture handles 10x
load, horizontal scaling, component failure) is assessed by the architecture
lens. Resilience patterns (retry strategies, circuit breakers, timeout policies)
are also assessed by the architecture lens — this lens focuses on whether the
*implementation* of these patterns is efficient, not whether the strategy itself
is appropriate. This lens focuses on *code-level performance* — whether
individual components, algorithms, and data paths are efficient. Observability
infrastructure (structured logging, metrics collection, tracing) is assessed
by the code quality lens. This lens may note *what to measure* for performance
but does not assess the observability design itself.

## Key Evaluation Questions

For each component or change under review, assess:

- **Algorithmic complexity**: What is the time/space complexity? Is it
  appropriate for the expected data sizes? Are there O(n²) patterns that could
  be O(n) or O(n log n)?
- **Data structure fitness**: Are data structures chosen for the access patterns
  used? Would a different structure reduce complexity?
- **Hot path efficiency**: Is unnecessary work being done in frequently executed
  code paths? Are there allocations, lookups, or computations that could be
  hoisted or cached?
- **Database performance**: Are queries efficient? Are there N+1 patterns? Are
  result sets bounded? Are batch operations used where appropriate?
- **Resource management**: Are connections, handles, and pools managed correctly?
  Are resources released promptly? Are there potential leaks?
- **I/O efficiency**: Are network calls batched where possible? Is payload size
  appropriate? Are streaming or pagination patterns used for large data sets?
- **Concurrency safety**: Is shared mutable state protected? Are locks
  appropriately scoped? Are async operations correctly awaited?
- **Caching**: Is caching applied where it would reduce load? Is the
  invalidation strategy sound? Are TTLs appropriate?

## Important Guidelines

- **Explore the codebase** for existing performance patterns and conventions
- **Be pragmatic** — focus on performance issues that will matter at expected
  scale, not micro-optimisations
- **Rate confidence** on each finding — distinguish measured bottlenecks from
  potential concerns
- **Consider the data scale** — an O(n²) loop over 5 items is fine; over
  50,000 items it's critical
- **Check for existing optimisations** — understand whether seemingly
  inefficient code has already been profiled and is adequate
- **Assess proportionally** — a background job doesn't need the same
  optimisation scrutiny as a hot API endpoint
- **Suggest measurement** — when uncertain, recommend profiling rather than
  speculative optimisation

## What NOT to Do

- Don't review architecture, security, test coverage, code quality, standards,
  or usability — those are other lenses
- Don't recommend premature optimisation — only flag issues proportional to
  expected scale and frequency
- Don't micro-optimise — focus on algorithmic and structural improvements, not
  shaving nanoseconds
- Don't assess system-level scalability (horizontal scaling, load balancing,
  component failure) — that is the architecture lens
- Don't assess observability infrastructure (logging, metrics, tracing design)
  — that is the code quality lens
- Don't penalise code that has been profiled and shown to be adequate

Remember: You're evaluating whether code will perform well under expected load
— efficient algorithms, appropriate resource management, safe concurrency, and
effective caching. The best performance work targets the right bottleneck with
the simplest fix.
```

### Success Criteria:

#### Automated Verification:

- [x] File exists: `ls ~/.claude/skills/performance-lens/SKILL.md`
- [x] Frontmatter contains `name: performance-lens`
- [x] Frontmatter contains `user-invocable: false`
- [x] File contains exactly 3 Core Responsibilities sections
- [x] File contains Key Evaluation Questions section
- [x] File contains Important Guidelines section
- [x] File contains What NOT to Do section
- [x] What NOT to Do lists all 6 other lenses

---

## Phase 2: Extend Architecture Lens with Resilience

### Overview

Add a 4th Core Responsibility to the architecture lens covering resilience and
fault tolerance. Add corresponding evaluation questions.

### Changes Required:

#### 1. Architecture Lens Resilience Extension

**File**: `~/.claude/skills/architecture-lens/SKILL.md`
**Changes**: Add 4th Core Responsibility after the existing 3rd, expand the Key
Evaluation Questions section

After the existing 3rd responsibility (`architecture-lens/SKILL.md:28-36`), add:

```markdown
4. **Evaluate Resilience and Fault Tolerance**

- Assess retry strategies and backoff policies for external dependencies
- Check for circuit breaker patterns where cascading failure is a risk
- Evaluate graceful degradation — does the system provide partial service when
  components fail?
- Verify timeout handling — are timeouts set and propagated appropriately?
- Check idempotency guarantees for operations that may be retried
- Assess error recovery and compensation strategies
- Evaluate health check and readiness probe design
- Check for single points of failure that lack failover mechanisms

Note: Code-level performance (algorithmic complexity, N+1 queries, caching
efficiency, resource management) is assessed by the performance lens. This
responsibility focuses on whether the *architectural strategy* for resilience
is appropriate, not whether the implementation is efficient.
```

In the Key Evaluation Questions section (`architecture-lens/SKILL.md:38-55`),
replace the existing "Scalability & resilience" bullet with two separate bullets:

Replace:
```markdown
- **Scalability & resilience**: What happens under 10x load? What happens when
  a component fails?
```

With:
```markdown
- **Scalability**: What happens under 10x load? Can the architecture scale
  horizontally? Are there bottleneck components?
- **Resilience**: What happens when a dependency fails? Are retry and backoff
  strategies appropriate? Is there graceful degradation? Are timeouts set and
  propagated? Are operations idempotent where they need to be?
```

### Success Criteria:

#### Automated Verification:

- [x] File contains 4 Core Responsibilities sections
- [x] File contains "Evaluate Resilience and Fault Tolerance" heading
- [x] Key Evaluation Questions contains separate Scalability and Resilience
  bullets
- [x] File still contains all original content (structural integrity, coupling,
  consistency sections unchanged)

---

## Phase 3: Wire Performance Lens into Review Orchestrators

### Overview

Update both `review-pr` and `review-plan` to include the performance lens in
their Available Review Lenses tables and auto-detect relevance criteria.

### Changes Required:

#### 1. Update review-pr Skill

**File**: `~/.claude/skills/review-pr/SKILL.md`

**Change 1**: Add Performance row to the Available Review Lenses table
(`review-pr/SKILL.md:48-55`).

After the Usability row, add:

```markdown
| **Performance**  | `performance-lens`            | Algorithmic efficiency, resource usage, concurrency, caching   |
```

**Change 2**: Add Performance auto-detect criteria in Step 2
(`review-pr/SKILL.md:113-128`).

After the Usability criteria, add:

```markdown
- **Performance** — relevant when changes involve: data processing, database
  queries, API endpoints handling load, algorithm-heavy code, concurrency
  primitives, caching logic, or hot code paths. Skip for documentation-only,
  configuration-only, or simple UI changes.
```

**Change 3**: Update the lens selection preview template
(`review-pr/SKILL.md:130-142`).

Add a Performance line to the preview template:

```markdown
- Performance: [reason — or "Skipping: no performance-sensitive changes identified"]
```

**Change 4**: Update hardcoded lens count references in Important Guidelines.

Replace "six" with "seven" at two locations:
- `review-pr/SKILL.md:438`: "Don't just paste six reports together" →
  "Don't just paste seven reports together"
- `review-pr/SKILL.md:446`: "outweighs minor findings from all six" →
  "outweighs minor findings from all seven"

#### 2. Update review-plan Skill

**File**: `~/.claude/skills/review-plan/SKILL.md`

**Change 1**: Add Performance row to the Available Review Lenses table
(`review-plan/SKILL.md:42-49`).

After the Usability row, add:

```markdown
| **Performance**  | `performance-lens`            | Algorithmic efficiency, resource usage, concurrency, caching  |
```

**Change 2**: Add Performance auto-detect criteria in Step 2
(`review-plan/SKILL.md:75-92`).

After the Usability criteria, add:

```markdown
- **Performance** — relevant when the plan involves: data processing pipelines,
  database interactions, high-throughput APIs, concurrent processing, caching
  strategy, or algorithm-heavy logic. Skip for documentation-only,
  configuration-only, or trivial changes.
```

**Change 3**: Update the lens selection preview template
(`review-plan/SKILL.md:96-106`).

Add a Performance line to the preview template:

```markdown
- Performance: [reason — or "Skipping: no performance-sensitive changes identified"]
```

**Change 4**: Update hardcoded lens count references in Important Guidelines.

Replace "five" with "seven" at two locations (correcting the pre-existing
inconsistency where the count was already wrong):
- `review-plan/SKILL.md:365`: "Don't just paste five reports together" →
  "Don't just paste seven reports together"
- `review-plan/SKILL.md:373`: "outweighs minor findings from all five" →
  "outweighs minor findings from all seven"

### Success Criteria:

#### Automated Verification:

- [x] `review-pr/SKILL.md` contains "Performance" in the Available Review Lenses
  table
- [x] `review-pr/SKILL.md` contains `performance-lens` in the table
- [x] `review-pr/SKILL.md` contains Performance auto-detect criteria
- [x] `review-pr/SKILL.md` contains Performance in the preview template
- [x] `review-pr/SKILL.md` Important Guidelines references "seven" lenses
- [x] `review-plan/SKILL.md` contains "Performance" in the Available Review
  Lenses table
- [x] `review-plan/SKILL.md` contains `performance-lens` in the table
- [x] `review-plan/SKILL.md` contains Performance auto-detect criteria
- [x] `review-plan/SKILL.md` contains Performance in the preview template
- [x] `review-plan/SKILL.md` Important Guidelines references "seven" lenses

---

## Phase 4: Update Output Format Skills

### Overview

Add `"performance"` to the lens identifier examples in both output format skills.

### Changes Required:

#### 1. PR Review Output Format

**File**: `~/.claude/skills/pr-review-output-format/SKILL.md`

**Change**: Update the lens identifier examples in the Field Reference
(`pr-review-output-format/SKILL.md:50-51`).

Replace:
```markdown
- **lens**: Agent lens identifier (e.g., `"architecture"`, `"security"`,
  `"test-coverage"`, `"code-quality"`, `"standards"`, `"usability"`)
```

With:
```markdown
- **lens**: Agent lens identifier (e.g., `"architecture"`, `"security"`,
  `"test-coverage"`, `"code-quality"`, `"standards"`, `"usability"`,
  `"performance"`)
```

#### 2. Plan Review Output Format

**File**: `~/.claude/skills/plan-review-output-format/SKILL.md`

**Change**: Update the lens identifier examples in the Field Reference
(`plan-review-output-format/SKILL.md:39-40`).

Replace:
```markdown
- **lens**: Agent lens identifier (e.g., `"architecture"`, `"security"`,
  `"test-coverage"`, `"code-quality"`, `"standards"`, `"usability"`)
```

With:
```markdown
- **lens**: Agent lens identifier (e.g., `"architecture"`, `"security"`,
  `"test-coverage"`, `"code-quality"`, `"standards"`, `"usability"`,
  `"performance"`)
```

### Success Criteria:

#### Automated Verification:

- [x] `pr-review-output-format/SKILL.md` contains `"performance"` in the lens
  identifier list
- [x] `plan-review-output-format/SKILL.md` contains `"performance"` in the lens
  identifier list

---

## Phase 5: Update Existing Lens Boundary Statements

### Overview

Update all 6 existing lens skills' "What NOT to Do" sections to include
"performance" in their list of other lenses.

### Changes Required:

#### 1. Architecture Lens

**File**: `~/.claude/skills/architecture-lens/SKILL.md`

Replace (`architecture-lens/SKILL.md:73-74`):
```markdown
- Don't review security, test coverage, code quality, standards, or usability
  — those are other lenses
```

With:
```markdown
- Don't review security, test coverage, code quality, standards, usability, or
  performance — those are other lenses
- Don't assess code-level performance (algorithmic complexity, N+1 queries,
  resource efficiency) — that is the performance lens
```

#### 2. Security Lens

**File**: `~/.claude/skills/security-lens/SKILL.md`

Replace (`security-lens/SKILL.md:88-89`):
```markdown
- Don't review architecture, test coverage, code quality, standards, or
  usability — those are other lenses
```

With:
```markdown
- Don't review architecture, test coverage, code quality, standards, usability,
  or performance — those are other lenses
- Security-motivated DoS evaluation (e.g., "Can this endpoint be overwhelmed by
  a malicious actor?") stays in this lens. General performance efficiency (e.g.,
  "Is this algorithm O(n²) when it could be O(n)?") is the performance lens.
```

#### 3. Test Coverage Lens

**File**: `~/.claude/skills/test-coverage-lens/SKILL.md`

Replace (`test-coverage-lens/SKILL.md:87-88`):
```markdown
- Don't review architecture, security, code quality, standards, or usability
  — those are other lenses
```

With:
```markdown
- Don't review architecture, security, code quality, standards, usability, or
  performance — those are other lenses
```

#### 4. Code Quality Lens

**File**: `~/.claude/skills/code-quality-lens/SKILL.md`

Replace (`code-quality-lens/SKILL.md:87-88`):
```markdown
- Don't review architecture, security, test coverage, standards, or usability
  — those are other lenses
```

With:
```markdown
- Don't review architecture, security, test coverage, standards, usability, or
  performance — those are other lenses
- Don't assess algorithmic efficiency, caching strategy, or concurrency safety
  — that is the performance lens
```

#### 5. Standards Lens

**File**: `~/.claude/skills/standards-lens/SKILL.md`

Replace (`standards-lens/SKILL.md:96-97`):
```markdown
- Don't review architecture, security, test coverage, code quality, or
  usability — those are other lenses
```

With:
```markdown
- Don't review architecture, security, test coverage, code quality, usability,
  or performance — those are other lenses
```

#### 6. Usability Lens

**File**: `~/.claude/skills/usability-lens/SKILL.md`

Replace (`usability-lens/SKILL.md:80-81`):
```markdown
- Don't review architecture, security, test coverage, code quality, or
  standards — those are other lenses
```

With:
```markdown
- Don't review architecture, security, test coverage, code quality, standards,
  or performance — those are other lenses
```

### Success Criteria:

#### Automated Verification:

- [x] All 6 existing lens skills mention "performance" in their What NOT to Do
  section
- [x] Architecture lens has specific boundary note about code-level performance
- [x] Security lens has specific boundary note about DoS vs general performance
- [x] Code quality lens has specific boundary note about algorithmic efficiency

---

## Testing Strategy

### Manual Verification:

1. Run `/review-pr` on a PR that involves database queries and algorithm-heavy
   code — verify the performance lens is auto-selected
2. Run `/review-pr` on a documentation-only PR — verify the performance lens
   is skipped
3. Run `/review-plan` on a plan involving data processing — verify the
   performance lens is auto-selected
4. Run `/review-pr` with a focus argument like "focus on performance" — verify
   the performance lens is included
5. Verify that the performance lens reviewer produces valid JSON matching the
   output format schema
6. Verify that the architecture lens now covers resilience concerns (retry
   strategies, circuit breakers, timeouts) in its review output

## Performance Considerations

Adding a 7th lens adds one more parallel agent task during reviews. Since all
lenses run concurrently, the impact is marginal — the review takes as long as
the slowest lens, not the sum of all lenses. The additional context from the
performance lens skill (~100 lines) is read by each agent independently and
does not affect the orchestrator's context window.

## References

- Gap analysis research: `meta/research/codebase/2026-02-22-review-lens-gap-analysis.md`
- Existing lens pattern: `~/.claude/skills/architecture-lens/SKILL.md`
- PR review orchestrator: `~/.claude/skills/review-pr/SKILL.md`
- Plan review orchestrator: `~/.claude/skills/review-plan/SKILL.md`
- Generic reviewer agent: `~/.claude/agents/reviewer.md`
- PR output format: `~/.claude/skills/pr-review-output-format/SKILL.md`
- Plan output format: `~/.claude/skills/plan-review-output-format/SKILL.md`
