# New Review Lenses Implementation Plan

## Overview

Add 6 new review lenses to the accelerator plugin's multi-lens review system:
documentation, database, correctness, compatibility, portability, and safety.
This expands coverage from 7 to 13 lenses, closing gaps identified against
ISO 25010, the Qodo specialist-agent pattern, and project-specific needs
(technical writing quality, DBA review). Additionally, update existing lenses
to reflect boundary changes where the new lenses take ownership of concerns
previously handled by other lenses. To manage review cost, both orchestrators
are updated to constrain lens selection to the 6-8 most relevant lenses per
review rather than running all 13.

## Current State Analysis

The review system has 7 lenses under
`skills/review/lenses/<name>-lens/SKILL.md`:

| Lens          | Core Domain                                                    |
|---------------|----------------------------------------------------------------|
| Architecture  | Modularity, coupling, resilience, evolutionary fitness         |
| Security      | OWASP, STRIDE, auth/authz, secrets, data flows                |
| Performance   | Algorithmic efficiency, resource usage, concurrency, caching   |
| Code Quality  | Complexity, design principles, error handling, observability   |
| Standards     | Project conventions, API standards, naming, **documentation**  |
| Test Coverage | Coverage adequacy, assertion quality, test pyramid             |
| Usability     | Developer experience, API ergonomics, **breaking changes**     |

The bold items indicate concerns that will transfer to new lenses:
- **Documentation** ownership transfers from standards to the new
  documentation lens
- **Database and query performance** transfers from performance to the new
  database lens
- **Breaking changes / backward compatibility** transfers from usability to
  the new compatibility lens

Each lens follows the optimal structure defined in
`meta/research/codebase/2026-03-15-review-lens-optimal-structure.md`:

- YAML frontmatter (`name`, `description`, `user-invocable: false`,
  `disable-model-invocation: true`)
- Perspective preamble
- Core Responsibilities (3-4 numbered groups)
- Key Evaluation Questions (with conditional applicability sub-groups)
- Important Guidelines
- What NOT to Do
- Closing Remember statement

Two orchestrators consume lenses:
- `skills/git/review-pr/SKILL.md` — PR reviews
- `skills/planning/review-plan/SKILL.md` — plan reviews

Two output formats define JSON schemas:
- `skills/review/output-formats/pr-review-output-format/SKILL.md`
- `skills/review/output-formats/plan-review-output-format/SKILL.md`

### Key Discoveries

- Each lens is a passive skill (`user-invocable: false`,
  `disable-model-invocation: true`) read by the generic `reviewer` agent at
  spawn time (`agents/reviewer.md`)
- Lens selection auto-detect criteria are defined in the orchestrators
  (`skills/git/review-pr/SKILL.md:120-137`,
  `skills/planning/review-plan/SKILL.md:80-97`)
- The performance lens has a "Database and query performance" conditional
  group (`skills/review/lenses/performance-lens/SKILL.md:76-81`) that will
  transfer to the database lens
- The standards lens covers documentation as a sub-concern
  (`skills/review/lenses/standards-lens/SKILL.md`) that will transfer to the
  documentation lens
- The usability lens covers breaking changes and migration paths that will
  partially transfer to the compatibility lens
- Output format files list lens identifiers as examples
  (`skills/review/output-formats/pr-review-output-format/SKILL.md:50-52`,
  `skills/review/output-formats/plan-review-output-format/SKILL.md:39-41`)
- All existing lenses list other lenses in their "What NOT to Do" section

## Desired End State

After implementation:

- 13 lens skills exist under `skills/review/lenses/`, each following the
  optimal structure
- Both orchestrators (`review-pr`, `review-plan`) list all 13 lenses with
  auto-detect relevance criteria
- Both orchestrators enforce a 6-8 lens cap per review, selecting the most
  relevant lenses for the change under review
- Both output format files list all 13 lens identifiers
- All 13 lenses cross-reference each other in their "What NOT to Do" sections
- Existing lenses have updated boundary notes where ownership has transferred
- The performance lens no longer has a "Database and query performance"
  conditional group (transferred to database lens)
- The standards lens no longer covers documentation quality (transferred to
  documentation lens)
- The usability lens no longer covers backward-compatibility concerns
  (transferred to compatibility lens)

### Verification

- 13 directories exist under `skills/review/lenses/`, each containing a
  `SKILL.md`
- Each lens has: YAML frontmatter, perspective preamble, Core
  Responsibilities (3-4 groups), Key Evaluation Questions (with conditional
  sub-groups), Important Guidelines, What NOT to Do, Remember statement
- Both orchestrators list 13 lenses in their Available Review Lenses table
- Both orchestrators have auto-detect criteria for all 13 lenses
- Both orchestrators enforce a 6-8 lens cap, selecting the most relevant
  lenses and skipping the rest
- Both output formats list all 13 lens identifiers
- Every lens's "What NOT to Do" lists all 12 other lenses
- No two lenses claim ownership of the same concern without a boundary note
  clarifying the distinction

## What We're NOT Doing

- Not changing the reviewer agent (`agents/reviewer.md`) — it is lens-agnostic
- Not changing the output format schemas — they are lens-agnostic
- Not changing the orchestrator workflow (steps 1, 3-7) — only updating the
  lens tables, auto-detect criteria, and lens cap constraint in step 2
- Not rewriting existing lenses from scratch — making targeted boundary
  updates only
- Not changing the `~10 inline comment` cap or severity model
- Not implementing feedback loops or attribution tracking
- Not changing the parallel execution model — all lenses still run
  concurrently

## Implementation Approach

Work in 8 phases:

- **Phases 1-6**: Create each new lens skill file, one per phase
- **Phase 7**: Update orchestrators, output formats, and existing lens
  boundary statements
- **Phase 8**: Verification pass

Each lens follows the same template. Phases 1-6 are independent and could be
implemented in parallel. Phase 7 depends on all 6 lenses being created. Phase
8 depends on phase 7.

---

## Phase 1: Create Documentation Lens

### Overview

Create a documentation lens that reviews as a technical writer, taking
ownership of documentation quality concerns previously handled by the
standards lens.

### Changes Required

#### 1. Create Lens Skill File

**File**: `skills/review/lenses/documentation-lens/SKILL.md`

```markdown
---
name: documentation-lens
description: Documentation review lens for evaluating documentation
  completeness, accuracy, and audience-appropriateness. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Documentation Lens

Review as a technical writer ensuring that documentation enables self-service
understanding for every audience.

## Core Responsibilities

1. **Evaluate API and Interface Documentation Completeness**

- Assess whether public APIs have complete documentation (parameters, return
  values, error codes, usage examples)
- Check that function/method signatures are documented with purpose, inputs,
  outputs, and side effects where non-obvious
- Verify error responses are documented with codes, messages, and remediation
  guidance
- Evaluate whether type definitions and data models are documented
- Check for working code examples that demonstrate common use cases

2. **Assess Developer-Facing Documentation Quality**

- Evaluate README completeness (purpose, quick start, prerequisites,
  installation, configuration, contribution guide)
- Check for architectural documentation (system overview, component
  relationships, data flow)
- Assess changelog and migration guide maintenance for breaking changes
- Verify that getting-started guides are accurate and follow a logical
  progression
- Check that decision records exist for non-obvious architectural choices

3. **Review Inline Documentation and Code Comments**

- Assess whether comments explain "why" rather than "what"
- Check that complex algorithms or non-obvious logic have explanatory
  comments
- Verify that TODO/FIXME/HACK comments include context (who, when, why,
  ticket reference)
- Identify misleading or outdated comments that contradict the code
- Evaluate whether the code is sufficiently self-documenting to minimise
  comment need

4. **Evaluate Documentation Consistency and Audience Fit**

- Check consistency between code behaviour and documentation claims
- Assess whether documentation is appropriate for its audience (end-user
  docs vs developer docs vs operator docs)
- Verify consistent terminology, formatting, and style across documentation
- Check that links and cross-references are valid and point to current
  content
- Evaluate whether documentation is discoverable (sensible file locations,
  table of contents, search-friendly titles)

**Boundary note**: Naming conventions and code style compliance are assessed
by the standards lens. This lens focuses on whether documentation *content* is
complete, accurate, and useful — not whether it follows formatting rules.

## Key Evaluation Questions

**Documentation completeness** (always applicable):

- **API documentation**: If a developer needed to call this API without
  reading the source code, would the documentation alone be sufficient?
  (Watch for: missing parameter descriptions, undocumented error codes,
  no usage examples, missing authentication requirements.)
- **README currency**: If a new team member cloned this repository today,
  could they get a working development environment from the README alone?
  (Watch for: outdated setup instructions, missing prerequisites, broken
  commands, assumed knowledge.)
- **Change documentation**: If a consumer upgraded to this version, would
  the changelog and migration guide tell them everything they need to know?
  (Watch for: undocumented breaking changes, missing migration steps, vague
  changelog entries like "bug fixes".)

**Inline documentation** (when the change includes non-trivial logic or
algorithms):

- **Comment accuracy**: Do the comments still describe what the code actually
  does, or have they drifted? (Watch for: comments that describe previous
  behaviour, comments that contradict the code, outdated TODO references.)
- **Explanatory depth**: For the most complex function in this change, could
  a new developer understand *why* it works this way from the comments
  alone? (Watch for: uncommented edge cases, unexplained magic numbers,
  missing rationale for non-obvious approaches.)

**Documentation consistency** (when the change affects documented interfaces
or behaviour):

- **Behaviour-documentation alignment**: If I tested every claim in the
  documentation against the actual code, which claims would fail? (Watch
  for: documented defaults that don't match code, documented error codes
  that aren't thrown, documented parameters that are ignored.)
- **Audience appropriateness**: Would the intended reader of this
  documentation understand it without asking a colleague? (Watch for:
  jargon without definition, assumed familiarity with internal systems,
  missing context for external consumers.)

## Important Guidelines

- **Explore the codebase** for existing documentation patterns and
  conventions
- **Be pragmatic** — focus on documentation gaps that would block or confuse
  real users, not on perfecting every sentence
- **Rate confidence** on each finding — distinguish definite documentation
  errors (code contradicts docs) from improvement suggestions
- **Assess proportionally** — internal utilities need less documentation than
  public APIs
- **Check for existing docs** — sometimes documentation exists in a
  different location (wiki, external site, parent README)
- **Prioritise accuracy over completeness** — incorrect documentation is
  worse than missing documentation

## What NOT to Do

- Don't review architecture, security, performance, code quality, standards,
  test coverage, usability, database, correctness, compatibility,
  portability, or safety — those are other lenses
- Don't enforce specific documentation formatting or style — that is the
  standards lens
- Don't assess whether naming is descriptive — that is the standards lens
- Don't rewrite documentation yourself — identify what's missing or wrong
- Don't require documentation for self-evident code (simple getters, obvious
  one-liners)
- Don't insist on comments when the code is already self-documenting

Remember: You're evaluating whether documentation empowers its readers to
succeed without asking the author for help. The best documentation answers
the questions someone will actually have.
```

### Success Criteria

#### Automated Verification

- [ ] File exists at `skills/review/lenses/documentation-lens/SKILL.md`
- [ ] File contains YAML frontmatter with `name: documentation-lens`,
  `user-invocable: false`, `disable-model-invocation: true`
- [ ] File contains all 6 required sections: perspective preamble, Core
  Responsibilities, Key Evaluation Questions, Important Guidelines, What NOT
  to Do, Remember statement
- [ ] Core Responsibilities has 4 numbered groups
- [ ] Key Evaluation Questions has conditional applicability sub-groups
- [ ] What NOT to Do lists all 12 other lenses

---

## Phase 2: Create Database Lens

### Overview

Create a database lens that reviews as a DBA, taking ownership of all
database concerns including query performance previously handled by the
performance lens.

### Changes Required

#### 1. Create Lens Skill File

**File**: `skills/review/lenses/database-lens/SKILL.md`

```markdown
---
name: database-lens
description: Database review lens for evaluating migration safety, schema
  design, query correctness, and data integrity. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Database Lens

Review as a database administrator ensuring data integrity, migration safety,
and query fitness for purpose.

## Core Responsibilities

1. **Evaluate Migration Safety and Rollback Strategy**

- Assess whether migrations can be rolled back without data loss
- Check for locking implications on large tables (ALTER TABLE on millions of
  rows, full table scans during migration)
- Verify zero-downtime compatibility (can the migration run while the
  application serves traffic?)
- Evaluate data backfill strategies for correctness and performance
- Check for idempotency — can the migration be safely re-run?
- Assess whether migration order dependencies are explicit

2. **Assess Schema Design and Data Integrity**

- Evaluate normalisation level — is it appropriate for the access patterns?
- Check for appropriate constraints (NOT NULL, UNIQUE, CHECK, foreign keys)
- Assess index strategy — are indexes aligned with query patterns?
- Verify referential integrity across related tables
- Evaluate column type appropriateness (varchar length, numeric precision,
  timestamp timezone handling)
- Check for appropriate default values and nullable columns

3. **Review Query Correctness and Fitness**

- Assess query correctness (JOIN conditions, WHERE clauses, GROUP BY, NULL
  handling)
- Identify N+1 query patterns and missing batch operations
- Check for unbounded result sets (missing LIMIT/pagination)
- Evaluate query plans for missing indexes or full table scans
- Assess whether queries use parameterised inputs (not string concatenation)
- Check for appropriate use of transactions and isolation levels

4. **Evaluate Connection and Transaction Management**

- Assess connection pool configuration and sizing
- Check for proper transaction scoping (not holding transactions open during
  I/O or user interaction)
- Evaluate deadlock potential from transaction ordering
- Verify that connections are released in all code paths (including error
  paths)
- Check for appropriate use of read replicas vs primary for read/write
  separation

**Boundary note**: Algorithmic efficiency and general resource management are
assessed by the performance lens. This lens focuses specifically on
*database-layer* concerns — schema, queries, migrations, and data integrity.
SQL injection and data exposure are assessed by the security lens. This lens
focuses on whether queries are *correct and fit for purpose*, not whether they
are *safe from attack*.

## Key Evaluation Questions

**Migration safety** (when the change includes database migrations or schema
changes):

- **Rollback safety**: If this migration failed halfway through in
  production, what data would be lost or corrupted, and can the migration be
  reversed? (Watch for: destructive column drops without backup, data type
  changes that lose precision, missing down migrations.)
- **Locking impact**: If this migration runs on a table with 10 million rows
  during peak traffic, what locks will it acquire and for how long? (Watch
  for: ALTER TABLE adding NOT NULL columns, full table rewrites, missing
  concurrent index creation.)
- **Zero-downtime compatibility**: Can the old application version and the
  new application version both run against this schema simultaneously?
  (Watch for: column renames without aliases, removed columns still
  referenced, constraint additions that reject existing data.)

**Schema design** (when the change introduces or modifies database tables):

- **Constraint completeness**: What invalid data could be inserted into this
  table that the schema doesn't prevent? (Watch for: missing NOT NULL on
  required fields, missing UNIQUE constraints, missing CHECK constraints,
  absent foreign keys.)
- **Index fitness**: For each query that will hit this table, is there an
  index that supports it without a full table scan? (Watch for: missing
  indexes on foreign keys, missing composite indexes for multi-column
  queries, over-indexing that slows writes.)

**Query correctness** (when the change includes database queries):

- **NULL handling**: What happens to this query's results if any of the
  joined or filtered columns contain NULL? (Watch for: NULL in NOT IN
  subqueries, NULL equality comparisons, NULL in aggregate functions,
  missing COALESCE.)
- **Result set bounds**: What happens when this query returns 1 million rows
  instead of 10? (Watch for: missing pagination, unbounded IN clauses,
  missing LIMIT, eager loading of relationships.)
- **N+1 patterns**: Is this query executed inside a loop where a single
  batch query would suffice? (Watch for: ORM lazy loading in loops, missing
  JOIN or subquery, individual lookups for related records.)

**Transaction management** (when the change uses explicit transactions or
modifies data):

- **Transaction scope**: What is the longest this transaction could be held
  open, and what other operations would it block? (Watch for: transactions
  spanning HTTP calls, user interactions inside transactions, long-running
  computations under lock.)

## Important Guidelines

- **Explore the codebase** for existing database patterns, ORM conventions,
  and migration practices
- **Be pragmatic** — focus on data integrity risks and migration hazards that
  could cause production incidents, not theoretical schema perfection
- **Rate confidence** on each finding — distinguish definite data integrity
  risks from schema improvement suggestions
- **Consider the data scale** — a missing index on a 100-row lookup table is
  irrelevant; on a 100-million-row table it's critical
- **Check existing migrations** — understand the migration history and
  conventions before flagging patterns
- **Assess migration risk proportionally** — a new table with no existing
  data has lower migration risk than altering a high-traffic table

## What NOT to Do

- Don't review architecture, security, performance, code quality, standards,
  test coverage, usability, documentation, correctness, compatibility,
  portability, or safety — those are other lenses
- Don't assess SQL injection or data exposure — that is the security lens
- Don't assess general algorithmic efficiency — that is the performance lens
- Don't review ORM code quality or design patterns — that is the code
  quality lens
- Don't assess whether database tests exist — that is the test coverage lens
- Don't recommend schema changes for theoretical normalisation purity —
  denormalisation is appropriate when access patterns demand it
- Don't flag missing indexes without considering write overhead

Remember: You're evaluating whether the database layer will maintain data
integrity under all conditions — safe migrations, sound schema design,
correct queries, and disciplined transaction management. The best database
work prevents the 3am incident where data is silently corrupted.
```

### Success Criteria

#### Automated Verification

- [ ] File exists at `skills/review/lenses/database-lens/SKILL.md`
- [ ] File contains YAML frontmatter with `name: database-lens`,
  `user-invocable: false`, `disable-model-invocation: true`
- [ ] File contains all 6 required sections
- [ ] Core Responsibilities has 4 numbered groups
- [ ] Key Evaluation Questions has conditional applicability sub-groups
- [ ] What NOT to Do lists all 12 other lenses

---

## Phase 3: Create Correctness Lens

### Overview

Create a correctness lens that reviews as a formal verifier, focusing on
logical invariants, boundary conditions, and state validity.

### Changes Required

#### 1. Create Lens Skill File

**File**: `skills/review/lenses/correctness-lens/SKILL.md`

```markdown
---
name: correctness-lens
description: Correctness review lens for evaluating logical validity, boundary
  conditions, invariant preservation, and state management. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Correctness Lens

Review as a formal verifier checking whether the code's logic is sound under
all valid inputs and state transitions.

## Core Responsibilities

1. **Evaluate Logical Correctness and Invariant Preservation**

- Verify that conditional logic covers all cases (no missing branches,
  correct boolean expressions)
- Check arithmetic operations for overflow, underflow, division by zero, and
  precision loss
- Assess whether loop invariants hold (correct initialisation, termination
  conditions, progress guarantees)
- Verify that preconditions and postconditions are maintained across function
  boundaries
- Identify logic errors in complex expressions (De Morgan violations,
  operator precedence, short-circuit evaluation assumptions)

2. **Assess Boundary Conditions and Edge Cases**

- Check behaviour at boundaries: empty collections, zero values, maximum
  values, negative values, null/undefined
- Assess off-by-one errors in loops, array indexing, pagination, and range
  operations
- Verify handling of unicode, special characters, and locale-sensitive
  operations
- Evaluate behaviour when optional/nullable values are absent
- Check for integer overflow in size calculations, counter increments, and
  timestamp arithmetic

3. **Review State Management and Transition Validity**

- Verify that state machines have valid transitions and no unreachable or
  dead states
- Check that state mutations are atomic where required (no partial updates
  visible to other components)
- Assess initialisation completeness — can any code path use uninitialised
  or partially initialised state?
- Verify that cleanup/teardown logic runs in all code paths (including error
  paths)
- Identify time-of-check-to-time-of-use (TOCTOU) vulnerabilities in
  business logic

**Boundary note**: Concurrency safety (race conditions, deadlocks, lock
contention) is assessed by the performance lens. This lens focuses on
*logical* correctness — whether the code produces correct results assuming
single-threaded execution. Error handling patterns and observability are
assessed by the code quality lens. This lens focuses on whether the *logic*
is correct, not whether errors are well-structured or well-logged. Test
strategy and coverage are assessed by the test coverage lens. This lens
focuses on whether the *code itself* is correct, not whether tests would
catch incorrectness.

## Key Evaluation Questions

**Logical validity** (always applicable):

- **Branch completeness**: For each conditional in this change, what input
  would take the path that the author likely didn't consider? (Watch for:
  missing else branches, uncovered enum/switch cases, boolean expressions
  that don't cover the full domain.)
- **Arithmetic safety**: What happens to this calculation when the input is
  zero, negative, or the maximum representable value? (Watch for: division
  by zero, integer overflow, floating-point precision loss, unsigned
  underflow.)
- **Invariant preservation**: What invariant does this function assume on
  entry, and does every code path preserve it on exit? (Watch for:
  preconditions not checked, postconditions violated in error paths,
  partially-applied mutations.)

**Boundary conditions** (always applicable):

- **Edge case handling**: What happens when this function receives an empty
  collection, a single element, or a collection of maximum size? (Watch
  for: off-by-one in loops, empty array dereference, pagination at
  boundaries, first/last element special cases.)
- **Null/undefined propagation**: If any value in this data flow is
  null or absent, where does it first cause an error, and is that the right
  place? (Watch for: null pointer dereferences, undefined property access,
  missing null checks before operations.)

**State management** (when the change involves stateful components, workflows,
or lifecycle management):

- **State transition validity**: If I drew a state diagram for this
  component, are there any transitions that would leave the system in an
  inconsistent state? (Watch for: missing transitions, unreachable states,
  concurrent state mutations, partial updates without rollback.)
- **Initialisation completeness**: What happens if this component is used
  before its initialisation completes? (Watch for: uninitialised fields
  accessed in early lifecycle methods, missing null guards on lazy
  properties, constructor side effects.)

## Important Guidelines

- **Explore the codebase** for existing correctness patterns and defensive
  coding conventions
- **Be pragmatic** — focus on logic errors that would produce wrong results
  in production, not theoretical edge cases that can't occur given the
  domain
- **Rate confidence** on each finding — distinguish provable logic errors
  from possible edge cases
- **Consider domain constraints** — if the domain guarantees positive
  integers, don't flag missing negative-number handling
- **Trace data flow** — follow values from input to output to identify where
  assumptions break down
- **Check both happy and error paths** — logic errors in error handling code
  are often more dangerous than those in the happy path

## What NOT to Do

- Don't review architecture, security, performance, code quality, standards,
  test coverage, usability, documentation, database, compatibility,
  portability, or safety — those are other lenses
- Don't assess code style or readability — that is the code quality lens
- Don't assess whether tests cover the edge cases you identify — that is the
  test coverage lens
- Don't assess concurrency safety (race conditions, deadlocks) — that is
  the performance lens
- Don't assess SQL correctness or query logic — that is the database lens
- Don't flag theoretical edge cases that the domain prevents — verify domain
  constraints before flagging
- Don't recommend defensive coding where the type system already provides
  guarantees

Remember: You're evaluating whether the code produces correct results for
every valid input and state combination. The best correctness review finds
the subtle logic error that would pass every test except the one nobody
thought to write.
```

### Success Criteria

#### Automated Verification

- [ ] File exists at `skills/review/lenses/correctness-lens/SKILL.md`
- [ ] File contains YAML frontmatter with `name: correctness-lens`,
  `user-invocable: false`, `disable-model-invocation: true`
- [ ] File contains all 6 required sections
- [ ] Core Responsibilities has 3 numbered groups
- [ ] Key Evaluation Questions has conditional applicability sub-groups
- [ ] What NOT to Do lists all 12 other lenses

---

## Phase 4: Create Compatibility Lens

### Overview

Create a compatibility lens that reviews as an integration engineer, taking
ownership of backward-compatibility concerns previously handled by the
usability lens.

### Changes Required

#### 1. Create Lens Skill File

**File**: `skills/review/lenses/compatibility-lens/SKILL.md`

```markdown
---
name: compatibility-lens
description: Compatibility review lens for evaluating API contract stability,
  cross-platform support, protocol compliance, and dependency management. Used
  by review orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Compatibility Lens

Review as an integration engineer ensuring the system works correctly with its
consumers, dependencies, and target environments.

## Core Responsibilities

1. **Evaluate API Contract Compatibility**

- Assess backward compatibility of API changes (additions are safe, removals
  and renames are breaking)
- Check forward compatibility considerations (can older clients handle new
  response fields gracefully?)
- Verify that versioning strategy is followed consistently
- Evaluate serialisation format stability (JSON field names, enum values,
  date formats)
- Check that deprecation policies are followed (deprecation notices before
  removal, migration period)

2. **Assess Cross-Platform and Cross-Environment Compatibility**

- Check for browser compatibility issues (feature availability, polyfills,
  CSS compatibility)
- Assess OS-level compatibility (file paths, line endings, process signals,
  filesystem case sensitivity)
- Evaluate Node.js/runtime version compatibility for language features used
- Check for locale and timezone handling that assumes a specific environment
- Verify that character encoding is handled consistently (UTF-8, BOM
  handling)

3. **Review Protocol Compliance and Interoperability**

- Assess HTTP standard compliance (status codes, content types, headers,
  caching directives)
- Check for correct use of content negotiation and media types
- Evaluate WebSocket, gRPC, or other protocol compliance
- Verify that authentication protocol implementation follows spec (OAuth2,
  OIDC, JWT)
- Check for standards-compliant error response formats (RFC 7807/9457
  Problem Details)

4. **Evaluate Dependency Compatibility**

- Assess whether dependency version constraints are appropriate (not too
  tight, not too loose)
- Check for known incompatibilities between dependency versions
- Evaluate peer dependency satisfaction
- Identify transitive dependency conflicts
- Check that dependency upgrades don't introduce breaking changes to the
  project

**Boundary note**: Developer experience of APIs (ergonomics, discoverability,
least surprise) is assessed by the usability lens. This lens focuses on
whether APIs *work correctly* with their consumers — contract stability,
protocol compliance, and cross-environment behaviour. Security implications
of protocol misuse (e.g., missing CORS, insecure cookies) are assessed by the
security lens.

## Key Evaluation Questions

**API contract stability** (when the change modifies public APIs, response
schemas, or serialisation formats):

- **Backward compatibility**: If an existing consumer made the same API call
  after this change, would they get an error or unexpected result? (Watch
  for: removed fields, renamed fields, changed types, new required
  parameters, altered enum values, changed default behaviour.)
- **Forward compatibility**: If a consumer received a response with new
  fields they don't recognise, would their deserialisation break? (Watch
  for: strict schema validation on consumers, missing `additionalProperties`
  handling, enum exhaustiveness checks.)
- **Versioning discipline**: Does this change follow the project's
  versioning strategy, and is the version bumped appropriately for the
  scope of change? (Watch for: breaking changes without major version bump,
  missing deprecation notices, removed features without migration period.)

**Cross-platform compatibility** (when the change includes platform-specific
code, file operations, or environment assumptions):

- **Environment assumptions**: What would happen if this code ran on a
  different OS, runtime version, or locale than the developer's machine?
  (Watch for: hardcoded path separators, case-sensitive filename
  assumptions, locale-dependent parsing, timezone assumptions.)

**Protocol compliance** (when the change involves HTTP handlers, API
endpoints, or inter-service communication):

- **Standard compliance**: Would a generic HTTP client (not your custom
  client) interact with this endpoint correctly based on the response codes,
  headers, and content types returned? (Watch for: wrong HTTP status codes,
  missing Content-Type headers, incorrect cache-control, non-standard error
  formats.)

**Dependency management** (when the change adds, removes, or updates
dependencies):

- **Version safety**: If all dependencies resolved to their latest allowed
  version within the specified constraints, would the build still pass?
  (Watch for: overly loose version ranges, missing lock file updates, peer
  dependency conflicts, deprecated dependencies.)

## Important Guidelines

- **Explore the codebase** for existing compatibility patterns, versioning
  conventions, and platform support targets
- **Be pragmatic** — focus on compatibility issues that would break real
  consumers, not theoretical interoperability with unused platforms
- **Rate confidence** on each finding — distinguish definite breaking changes
  from potential compatibility risks
- **Consider the consumer base** — an internal API with one consumer has
  different compatibility requirements than a public API
- **Check for compatibility tests** — the codebase may already have contract
  tests or cross-platform CI
- **Assess the change scope** — additive API changes are generally safe;
  focus scrutiny on modifications and removals

## What NOT to Do

- Don't review architecture, security, performance, code quality, standards,
  test coverage, usability, documentation, database, correctness,
  portability, or safety — those are other lenses
- Don't assess API ergonomics or developer experience — that is the usability
  lens
- Don't assess security implications of protocols — that is the security lens
- Don't assess whether the API is well-documented — that is the
  documentation lens
- Don't flag theoretical compatibility issues with platforms the project
  doesn't target
- Don't insist on backward compatibility when the change is explicitly a
  breaking version bump

Remember: You're evaluating whether the system will continue to work
correctly with everything it connects to — consumers, platforms, protocols,
and dependencies. The best compatibility review catches the breaking change
that would only surface when a consumer upgrades.
```

### Success Criteria

#### Automated Verification

- [ ] File exists at `skills/review/lenses/compatibility-lens/SKILL.md`
- [ ] File contains YAML frontmatter with `name: compatibility-lens`,
  `user-invocable: false`, `disable-model-invocation: true`
- [ ] File contains all 6 required sections
- [ ] Core Responsibilities has 4 numbered groups
- [ ] Key Evaluation Questions has conditional applicability sub-groups
- [ ] What NOT to Do lists all 12 other lenses

---

## Phase 5: Create Portability Lens

### Overview

Create a portability lens that reviews as a platform engineer, evaluating
environment, deployment, and code portability.

### Changes Required

#### 1. Create Lens Skill File

**File**: `skills/review/lenses/portability-lens/SKILL.md`

```markdown
---
name: portability-lens
description: Portability review lens for evaluating environment independence,
  deployment flexibility, and vendor lock-in avoidance. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Portability Lens

Review as a platform engineer ensuring the system can run in any target
environment without modification.

## Core Responsibilities

1. **Evaluate Environment Portability**

- Assess whether the application runs correctly across target operating
  systems (Linux, macOS, Windows)
- Check for hardcoded environment assumptions (paths, environment variables,
  available system tools)
- Verify that configuration is externalised (not baked into build artifacts)
- Evaluate runtime version requirements and compatibility ranges
- Check for locale, timezone, and character encoding assumptions

2. **Assess Deployment Portability**

- Evaluate containerisation quality (Dockerfile best practices, image size,
  multi-stage builds)
- Check infrastructure-as-code for provider abstraction (Terraform modules,
  Pulumi components)
- Assess whether deployment scripts work across target environments
- Verify that health checks, readiness probes, and graceful shutdown are
  implemented portably
- Check for hardcoded deployment-environment assumptions (specific
  hostnames, IP ranges, account IDs)

3. **Review Code Portability and Vendor Independence**

- Identify vendor-specific API usage that could be abstracted behind an
  interface
- Assess cloud provider lock-in (AWS-specific, GCP-specific, Azure-specific
  services without abstraction)
- Check for database engine-specific SQL or features without a portability
  layer
- Evaluate whether third-party service integrations are behind interfaces
  that allow substitution
- Assess whether the codebase could migrate to a different hosting provider
  with reasonable effort

**Boundary note**: Cross-platform runtime compatibility (browser versions,
Node.js versions) is assessed by the compatibility lens. This lens focuses on
*deployment and operational portability* — whether the system can be deployed
and run in different environments and on different providers. Infrastructure
security (network policies, IAM) is assessed by the security lens.

## Key Evaluation Questions

**Environment portability** (always applicable):

- **Environment coupling**: If I deployed this to a completely fresh
  environment with only the documented prerequisites, what would fail?
  (Watch for: undocumented system dependencies, hardcoded paths,
  assumptions about available tools, missing environment variable
  documentation.)
- **Configuration externalisation**: What configuration is baked into the
  build artifact versus injected at runtime? (Watch for: hardcoded
  connection strings, embedded API keys, build-time feature flags that
  should be runtime flags.)

**Deployment portability** (when the change involves infrastructure,
deployment configuration, or containerisation):

- **Container quality**: If this container image needed to run on a
  different orchestrator (Kubernetes, ECS, Nomad), what would need to
  change? (Watch for: orchestrator-specific health check patterns,
  hardcoded port assignments, missing graceful shutdown, oversized images.)
- **Infrastructure abstraction**: If the organisation decided to switch
  cloud providers, which parts of this infrastructure code would need
  rewriting? (Watch for: provider-specific resource types without
  abstraction, hardcoded region or account references, proprietary service
  usage without fallback.)

**Vendor independence** (when the change introduces or deepens integration
with external services or cloud providers):

- **Vendor lock-in depth**: How deeply does this change couple the
  application to a specific vendor's API, and is there an interface boundary
  that would allow substitution? (Watch for: direct SDK calls scattered
  throughout business logic, vendor-specific data formats without a
  translation layer, proprietary features without open-standard
  alternatives.)

## Important Guidelines

- **Explore the codebase** for existing portability patterns, abstraction
  layers, and infrastructure conventions
- **Be pragmatic** — focus on portability risks that affect the project's
  actual deployment targets, not theoretical environments
- **Rate confidence** on each finding — distinguish definite portability
  blockers from improvement suggestions
- **Consider the project's portability requirements** — a single-cloud
  project may intentionally use provider-specific features
- **Assess lock-in proportionally** — using a managed database is
  appropriate lock-in; using a proprietary API without abstraction in core
  business logic is concerning
- **Check for existing abstraction layers** — the codebase may already wrap
  vendor-specific code behind interfaces

## What NOT to Do

- Don't review architecture, security, performance, code quality, standards,
  test coverage, usability, documentation, database, correctness,
  compatibility, or safety — those are other lenses
- Don't assess runtime version compatibility — that is the compatibility lens
- Don't assess infrastructure security — that is the security lens
- Don't assess deployment pipeline quality (CI/CD) — that is outside the
  review scope
- Don't penalise intentional vendor usage that is appropriate for the
  project's constraints
- Don't insist on abstraction layers for services the project will never
  migrate away from

Remember: You're evaluating whether the system could be picked up and
deployed elsewhere without a rewrite. The best portability review identifies
the vendor coupling that would become a six-month migration project if the
business needs changed.
```

### Success Criteria

#### Automated Verification

- [ ] File exists at `skills/review/lenses/portability-lens/SKILL.md`
- [ ] File contains YAML frontmatter with `name: portability-lens`,
  `user-invocable: false`, `disable-model-invocation: true`
- [ ] File contains all 6 required sections
- [ ] Core Responsibilities has 3 numbered groups
- [ ] Key Evaluation Questions has conditional applicability sub-groups
- [ ] What NOT to Do lists all 12 other lenses

---

## Phase 6: Create Safety Lens

### Overview

Create a safety lens that reviews as a safety engineer, focusing on data
safety and operational safety — preventing accidental harm as distinct from
security (which covers malicious harm).

### Changes Required

#### 1. Create Lens Skill File

**File**: `skills/review/lenses/safety-lens/SKILL.md`

```markdown
---
name: safety-lens
description: Safety review lens for evaluating data loss prevention,
  operational safety, and protective mechanisms against accidental harm. Used
  by review orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Safety Lens

Review as a safety engineer ensuring the system prevents accidental harm to
data and operations.

## Core Responsibilities

1. **Evaluate Data Safety**

- Assess whether operations that destroy or modify data have appropriate
  safeguards (confirmation, soft delete, backups)
- Check for data loss risks in migration, cleanup, and batch operations
- Verify that cascading deletes are intentional and bounded
- Evaluate backup and recovery provisions for critical data operations
- Check for data corruption risks from concurrent modifications or partial
  writes
- Assess whether audit trails exist for irreversible data operations

2. **Assess Operational Safety**

- Check for safeguards against accidental deployment to production (feature
  flags, canary releases, rollback mechanisms)
- Evaluate blast radius of failures — does a single component failure
  cascade to full system outage?
- Assess graceful degradation — does the system continue to serve critical
  functions when non-critical components fail?
- Check for resource exhaustion protections (memory limits, disk space
  monitoring, queue depth limits)
- Verify that dangerous operations require elevated permissions or
  confirmation
- Evaluate whether rate limiting and circuit breakers prevent runaway
  processes

3. **Review Protective Mechanisms and Recovery Paths**

- Assess whether destructive operations have undo or recovery mechanisms
- Check for kill switches and emergency stop capabilities
- Verify that monitoring and alerting cover critical failure modes
- Evaluate whether the system fails safe (denying access, stopping
  processing) rather than failing open
- Check for timeout enforcement on all external calls and long-running
  operations
- Assess whether automated processes have deadman switches or watchdog
  timers

**Boundary note**: Security (malicious actors, authentication, authorisation,
injection attacks) is assessed by the security lens. This lens focuses on
*accidental* harm — data loss from bugs, outages from configuration errors,
cascading failures from missing safeguards. Resilience patterns (retry
strategies, circuit breakers) are assessed by the architecture lens for
*architectural fitness*. This lens assesses whether those patterns *prevent
harm to users and data* in practice.

## Key Evaluation Questions

**Data safety** (always applicable):

- **Destructive operation safeguards**: If this data-modifying operation
  were accidentally triggered with wrong parameters, what is the worst-case
  data loss, and is there a recovery path? (Watch for: hard deletes without
  soft-delete option, missing confirmation for bulk operations, cascading
  deletes without bounds, no backup before destructive migration.)
- **Data corruption prevention**: If this write operation failed halfway
  through, would the data be left in an inconsistent state? (Watch for:
  non-atomic multi-step writes, missing transactions, partial updates
  visible to readers, no integrity checks after write.)

**Operational safety** (when the change affects deployment, configuration, or
system behaviour):

- **Blast radius containment**: If this component failed completely right
  now, which other components would be affected and would the system
  continue serving its most critical function? (Watch for: single points
  of failure, missing circuit breakers, synchronous dependencies on
  non-critical services, missing fallbacks.)
- **Runaway process prevention**: If this automated process received 100x
  the expected input, would it consume all available resources? (Watch for:
  unbounded loops, missing rate limits, no memory caps, queue consumers
  without backpressure.)

**Protective mechanisms** (when the change involves critical operations,
automated processes, or infrastructure):

- **Fail-safe defaults**: If the configuration for this feature were
  missing or corrupted, would the system fail safely (denying, stopping)
  or fail dangerously (allowing, proceeding)? (Watch for: missing default
  values that default to permissive behaviour, disabled safety checks when
  config is absent, no validation of critical configuration on startup.)
- **Recovery capability**: If this operation caused an incident, how long
  would it take to recover — minutes, hours, or days? (Watch for: no
  rollback mechanism, missing backups, irreversible state changes, no
  kill switch for automated processes.)

## Important Guidelines

- **Explore the codebase** for existing safety patterns, circuit breakers,
  and protective mechanisms
- **Be pragmatic** — focus on safety risks proportional to the blast radius
  and probability of occurrence
- **Rate confidence** on each finding — distinguish definite safety hazards
  from precautionary suggestions
- **Consider the criticality of the data and service** — a development tool
  has different safety requirements than a financial system
- **Think about the 3am scenario** — what happens when this fails with
  nobody watching?
- **Assess recovery time** — fast recovery reduces the impact of any failure

## What NOT to Do

- Don't review architecture, security, performance, code quality, standards,
  test coverage, usability, documentation, database, correctness,
  compatibility, or portability — those are other lenses
- Don't assess malicious attack vectors — that is the security lens
- Don't assess architectural resilience patterns for fitness — that is the
  architecture lens
- Don't assess migration correctness (schema design, query logic) — that is
  the database lens
- Don't penalise systems that appropriately trade safety for performance in
  non-critical paths
- Don't insist on safety mechanisms for operations that are easily reversible
- Don't conflate safety with security — a feature can be secure against
  attackers but unsafe against accidental misuse

Remember: You're evaluating whether the system protects users and data from
accidental harm — the misconfigured deployment, the runaway batch job, the
cascading failure at 3am. The best safety review ensures that when things go
wrong, the damage is contained and recovery is fast.
```

### Success Criteria

#### Automated Verification

- [ ] File exists at `skills/review/lenses/safety-lens/SKILL.md`
- [ ] File contains YAML frontmatter with `name: safety-lens`,
  `user-invocable: false`, `disable-model-invocation: true`
- [ ] File contains all 6 required sections
- [ ] Core Responsibilities has 3 numbered groups
- [ ] Key Evaluation Questions has conditional applicability sub-groups
- [ ] What NOT to Do lists all 12 other lenses

---

## Phase 7: Update Orchestrators, Output Formats, and Existing Lenses

### Overview

Integrate all 6 new lenses into the review system by updating the
orchestrators, output formats, and existing lens boundary statements.

### Changes Required

#### 1. Update review-pr Orchestrator

**File**: `skills/git/review-pr/SKILL.md`

**Change 1**: Add 6 rows to the Available Review Lenses table (after line 57):

```markdown
| **Documentation** | `documentation-lens`          | Documentation completeness, accuracy, audience fit             |
| **Database**      | `database-lens`               | Migration safety, schema design, query correctness, integrity  |
| **Correctness**   | `correctness-lens`            | Logical validity, boundary conditions, state management        |
| **Compatibility** | `compatibility-lens`          | API contracts, cross-platform, protocol compliance, deps       |
| **Portability**   | `portability-lens`            | Environment independence, deployment flexibility, vendor lock  |
| **Safety**        | `safety-lens`                 | Data loss prevention, operational safety, protective mechanisms|
```

**Change 2**: Add auto-detect relevance criteria to Step 2 (after line 137):

```markdown
- **Documentation** — relevant when changes involve: public APIs, README
  files, configuration surfaces, new features that need documentation,
  breaking changes requiring migration guides. Skip for internal refactoring
  with no interface changes.
- **Database** — relevant when changes involve: database migrations, schema
  changes, new queries, ORM model changes, transaction logic, connection
  pool configuration. Skip for changes with no database interaction.
- **Correctness** — relevant for most PRs; skip only for documentation-only,
  configuration-only, or simple renaming changes.
- **Compatibility** — relevant when changes involve: public API
  modifications, dependency updates, serialisation format changes,
  cross-platform code, protocol implementations. Skip for internal-only
  changes with no external consumers.
- **Portability** — relevant when changes involve: infrastructure
  configuration, deployment scripts, containerisation, cloud provider
  integrations, environment-specific code paths. Skip for application logic
  with no environment dependencies.
- **Safety** — relevant when changes involve: data deletion or modification
  operations, deployment configuration, automated batch processes,
  infrastructure changes, feature flags, or critical system components.
  Skip for read-only features, documentation, or UI-only changes.
```

**Change 3**: Add a lens selection cap instruction to Step 2, immediately
after the auto-detect relevance criteria and before the lens selection
display:

```markdown
**Lens selection cap:** With 13 available lenses, running all of them for
every review would be wasteful. Select the **6 to 8 most relevant lenses**
for the change under review. Apply these prioritisation rules:

1. **Always consider the core four**: Architecture, Code Quality, Test
   Coverage, and Correctness are relevant for most non-trivial changes.
   Include them unless the change is clearly outside their scope (e.g.,
   documentation-only).
2. **Add domain-specific lenses based on the change**: Use the auto-detect
   criteria above to identify which of the remaining lenses are relevant.
3. **If more than 8 lenses pass auto-detect**, rank by relevance to the
   specific change and drop the least relevant until you reach 6-8. Prefer
   lenses whose core responsibilities directly overlap with the change's
   primary concerns.
4. **If the user provided focus arguments**, prioritise the requested lenses
   and fill remaining slots (up to 8) with the most relevant auto-detected
   lenses.
5. **Never run fewer than 4 lenses** unless the change is trivially scoped
   (e.g., a typo fix).

When presenting the lens selection, clearly indicate which lenses are
selected and which are skipped, with a brief reason for each skip.
```

**Change 4**: Add the 6 new lenses to the lens selection display (after
line 149):

```markdown
- Documentation: [reason — or "Skipping: ..."]
- Database: [reason — or "Skipping: no database changes identified"]
- Correctness: [reason]
- Compatibility: [reason — or "Skipping: ..."]
- Portability: [reason — or "Skipping: ..."]
- Safety: [reason — or "Skipping: ..."]
```

#### 2. Update review-plan Orchestrator

**File**: `skills/planning/review-plan/SKILL.md`

Apply the same 4 changes as review-pr:

**Change 1**: Add 6 rows to the Available Review Lenses table (after
line 51).

**Change 2**: Add auto-detect relevance criteria to Step 2 (after line 97):

```markdown
- **Documentation** — relevant when the plan involves: new public APIs, new
  user-facing features, configuration changes, breaking changes, or new
  system components that will need documentation.
- **Database** — relevant when the plan involves: database schema changes,
  new tables, migrations, query-heavy features, or changes to data access
  patterns.
- **Correctness** — relevant for most plans; skip only for
  documentation-only or trivial configuration changes.
- **Compatibility** — relevant when the plan involves: public API changes,
  dependency updates, protocol changes, cross-platform considerations, or
  versioning decisions.
- **Portability** — relevant when the plan involves: infrastructure changes,
  deployment modifications, new cloud service integrations, or
  environment-specific logic.
- **Safety** — relevant when the plan involves: data migration, deletion
  logic, deployment changes, automated processes, or changes to critical
  system paths.
```

**Change 3**: Add the same lens selection cap instruction as review-pr
(identical text — the cap applies equally to plan reviews).

**Change 4**: Add the 6 new lenses to the lens selection display.

#### 3. Update Output Format Files

**File**: `skills/review/output-formats/pr-review-output-format/SKILL.md`

**Change**: Update the lens identifier examples in the Field Reference
(line 50-52) to include all 13:

```markdown
- **lens**: Agent lens identifier (e.g., `"architecture"`, `"security"`,
  `"test-coverage"`, `"code-quality"`, `"standards"`, `"usability"`,
  `"performance"`, `"documentation"`, `"database"`, `"correctness"`,
  `"compatibility"`, `"portability"`, `"safety"`)
```

**File**: `skills/review/output-formats/plan-review-output-format/SKILL.md`

**Change**: Same update to lens identifier examples (line 39-41).

#### 4. Update Existing Lens Boundary Statements

For each of the 7 existing lenses, update the "What NOT to Do" section to
list all 12 other lenses:

**File**: `skills/review/lenses/architecture-lens/SKILL.md`

**Change**: Update "Don't review..." line to include all 12 other lenses:

```markdown
- Don't review security, performance, code quality, standards, test
  coverage, usability, documentation, database, correctness, compatibility,
  portability, or safety — those are other lenses
```

Apply the same pattern to all 7 existing lens files:

- `skills/review/lenses/architecture-lens/SKILL.md`
- `skills/review/lenses/security-lens/SKILL.md`
- `skills/review/lenses/performance-lens/SKILL.md`
- `skills/review/lenses/code-quality-lens/SKILL.md`
- `skills/review/lenses/standards-lens/SKILL.md`
- `skills/review/lenses/test-coverage-lens/SKILL.md`
- `skills/review/lenses/usability-lens/SKILL.md`

#### 5. Transfer Ownership from Existing Lenses

**File**: `skills/review/lenses/performance-lens/SKILL.md`

**Change**: Remove the "Database and query performance" conditional group
from Key Evaluation Questions (lines 76-81) and add a boundary note:

```markdown
**Boundary note** (addition): Database query performance, N+1 patterns,
index fitness, and migration locking are assessed by the database lens. This
lens retains algorithmic efficiency and general resource management.
```

Remove from Key Evaluation Questions:

```markdown
**Database and query performance** (when the change includes database queries
or schema changes):
- **Database performance**: What happens to this query when the table has 10
  million rows? (Watch for: N+1 patterns, missing indexes, unbounded result
  sets, missing batch operations.)
```

**File**: `skills/review/lenses/standards-lens/SKILL.md`

**Change**: Remove documentation quality concerns from Core Responsibilities
and Key Evaluation Questions. Add boundary note:

```markdown
**Boundary note** (addition): Documentation completeness, accuracy, and
audience-appropriateness are assessed by the documentation lens. This lens
retains naming conventions and style compliance.
```

**File**: `skills/review/lenses/usability-lens/SKILL.md`

**Change**: Remove backward-compatibility and breaking-change concerns from
Core Responsibilities and Key Evaluation Questions. Add boundary note:

```markdown
**Boundary note** (addition): API contract compatibility, backward/forward
compatibility, and versioning discipline are assessed by the compatibility
lens. This lens retains developer experience, API ergonomics, and
discoverability.
```

**File**: `skills/review/lenses/security-lens/SKILL.md`

**Change**: Add boundary note distinguishing from safety:

```markdown
**Boundary note** (addition): Accidental harm (data loss from bugs,
operational outages, cascading failures) is assessed by the safety lens.
This lens focuses on *malicious* threats — attackers, injection, privilege
escalation.
```

**File**: `skills/review/lenses/code-quality-lens/SKILL.md`

**Change**: Add boundary note distinguishing from correctness:

```markdown
**Boundary note** (addition): Logical correctness (invariant preservation,
boundary conditions, state validity) is assessed by the correctness lens.
This lens focuses on *maintainability* — readability, design principles,
error handling patterns.
```

**File**: `skills/review/lenses/test-coverage-lens/SKILL.md`

**Change**: Add boundary note distinguishing from correctness:

```markdown
**Boundary note** (addition): Logical correctness of the code under review
is assessed by the correctness lens. This lens focuses on whether *tests
exist and are effective* at catching defects, not whether the code itself is
correct.
```

### Success Criteria

#### Automated Verification

- [ ] Both orchestrators list 13 lenses in their Available Review Lenses
  table
- [ ] Both orchestrators have auto-detect criteria for all 13 lenses
- [ ] Both orchestrators include a lens selection cap instruction (6-8 lenses)
- [ ] Both output format files list all 13 lens identifiers
- [ ] All 13 lens files list 12 other lenses in their "What NOT to Do"
- [ ] Performance lens no longer has "Database and query performance" group
- [ ] Standards lens no longer covers documentation quality
- [ ] Usability lens no longer covers backward-compatibility
- [ ] Security lens has boundary note distinguishing from safety
- [ ] Code quality lens has boundary note distinguishing from correctness
- [ ] Test coverage lens has boundary note distinguishing from correctness

---

## Phase 8: Verification Pass

### Overview

Read all 13 lens files and both orchestrators to verify structural
consistency and completeness.

### Changes Required

No file changes — this is a read-only verification phase.

### Verification Checklist

- [ ] All 13 lens directories exist under `skills/review/lenses/`
- [ ] Each lens SKILL.md has valid YAML frontmatter with correct `name`
- [ ] Each lens has a perspective preamble after the title
- [ ] Each lens has 3-4 Core Responsibility groups
- [ ] Each lens has Key Evaluation Questions with conditional sub-groups
- [ ] Each lens has at least one "(always applicable)" question group
- [ ] Each lens has Important Guidelines starting with "Explore the codebase"
  and including "Rate confidence"
- [ ] Each lens's "What NOT to Do" lists all 12 other lenses by name
- [ ] Each lens has a closing "Remember:" statement
- [ ] No two lenses claim the same concern without boundary notes
- [ ] Both orchestrators list all 13 lenses with auto-detect criteria
- [ ] Both orchestrators include lens selection cap (6-8 lenses per review)
- [ ] Both output formats list all 13 lens identifiers
- [ ] Boundary changes are applied correctly:
  - Performance lens defers database concerns to database lens
  - Standards lens defers documentation concerns to documentation lens
  - Usability lens defers compatibility concerns to compatibility lens
  - Security lens distinguishes from safety lens
  - Code quality lens distinguishes from correctness lens
  - Test coverage lens distinguishes from correctness lens

### Success Criteria

#### Automated Verification

- [ ] `ls skills/review/lenses/` shows 13 directories
- [ ] `grep -l "user-invocable: false" skills/review/lenses/*/SKILL.md`
  returns 13 files

#### Manual Verification

- [ ] Run `/review-pr` on a representative PR and verify:
  - Relevant new lenses are offered in the selection step
  - Selection is capped at 6-8 lenses (not all 13)
  - Skipped lenses have clear reasons
- [ ] Run `/review-plan` on a representative plan and verify same constraints
- [ ] Spot-check that new lens reviewers produce coherent, non-overlapping
  findings

---

## Testing Strategy

### Per-Phase Validation

After each of Phases 1-6, verify the new lens file:
- Has all required sections
- Follows the template from
  `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md`
- Has no overlapping concerns with existing lenses (check boundary notes)

After Phase 7, verify integration:
- Both orchestrators are syntactically correct
- All 13 lens identifiers appear in output formats
- All boundary updates are consistent across lenses

### Final Validation (Phase 8)

1. Read all 13 lens files and verify structural consistency
2. Run `/review-pr` on a representative PR:
   - Verify new lenses appear in selection
   - Verify findings from new lenses are non-overlapping with existing
   - Verify boundary changes don't create coverage gaps
3. Run `/review-plan` on a representative plan:
   - Same checks as PR review

## References

- Optimal lens structure:
  `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md`
- Original gap analysis:
  `meta/research/codebase/2026-02-22-review-lens-gap-analysis.md`
- Context management research:
  `meta/research/codebase/2026-03-15-context-management-approaches.md`
- Performance lens plan:
  `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md`
- Lens improvement plan:
  `meta/plans/2026-03-15-review-lens-improvements.md`
- PR review orchestrator: `skills/git/review-pr/SKILL.md`
- Plan review orchestrator: `skills/planning/review-plan/SKILL.md`
- Generic reviewer agent: `agents/reviewer.md`
- PR review output format:
  `skills/review/output-formats/pr-review-output-format/SKILL.md`
- Plan review output format:
  `skills/review/output-formats/plan-review-output-format/SKILL.md`
