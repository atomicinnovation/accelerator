---
date: "2026-04-08T01:21:36+01:00"
researcher: Toby Clemson
git_commit: 508ca24b973d8c742e52e829c557f0c62f81076d
branch: ticket-management
repository: accelerator
topic: "Product management and requirements gathering skills for the Accelerator plugin"
tags: [ research, tickets, product-management, skills, requirements, lifecycle ]
status: complete
last_updated: "2026-04-08"
last_updated_by: Toby Clemson
---

# Research: Product Management and Requirements Gathering Skills

**Date**: 2026-04-08T01:21:36+01:00
**Researcher**: Toby Clemson
**Git Commit**: 508ca24b973d8c742e52e829c557f0c62f81076d
**Branch**: ticket-management
**Repository**: accelerator

## Research Question

How should the Accelerator plugin be extended with product management and
requirements gathering skills that manage tickets in `meta/tickets/`, following
all existing conventions for skill structure, configuration, review, templates,
agent delegation, and filesystem persistence?

## Summary

The Accelerator plugin has well-established conventions across five skill
categories (planning, decisions, review, github, research) that can be directly
applied to a new `tickets/` skill category. The existing `decisions/` category
(create-adr, extract-adrs, review-adr) provides the closest structural parallel
— a lifecycle-managed artifact with numbered files, interactive creation,
batch extraction from source documents, and quality review. The planning
category adds patterns for stress-testing, multi-lens review, and
implementation. Together, these precedents inform a comprehensive ticket
management skill set.

This document proposes **7 skills** organised in a `skills/tickets/` category,
with a dedicated ticket template, companion scripts for numbering, and review
lenses tailored to ticket quality.

## Detailed Findings

### 1. Existing Conventions That Must Be Followed

Every skill in the plugin follows a consistent structural pattern that the new
ticket skills must replicate exactly.

#### 1.1 SKILL.md Frontmatter

All skills use this frontmatter structure:

```yaml
---
name: skill-name
description: One-line description of what the skill does and when to use it.
argument-hint: "[arguments description]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)
---
```

- `name`: kebab-case, unique across the plugin
- `description`: includes usage guidance ("Use when...")
- `argument-hint`: shows expected arguments
- `disable-model-invocation: true`: present on all skills
- `allowed-tools`: explicitly lists permitted Bash patterns; config scripts
  are always included

Reference: `skills/decisions/create-adr/SKILL.md:1-9`,
`skills/planning/stress-test-plan/SKILL.md:1-9`

#### 1.2 Configuration Injection Preamble

Every skill begins its body with the same preprocessor calls:

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh <skill-name>`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```

Followed by the agent names fallback block and path resolution:

```markdown
If no "Agent Names" section appears above, use these defaults: reviewer,
codebase-locator, codebase-analyser, codebase-pattern-finder,
documents-locator, documents-analyser, web-search-researcher.

**Tickets directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`
```

Reference: Every SKILL.md in the plugin follows this pattern.

#### 1.3 Plugin Registration

Skills are registered in `plugin.json` by directory:

```json
"skills": [
"./skills/vcs/",
"./skills/github/",
"./skills/planning/",
"./skills/research/",
"./skills/decisions/",
"./skills/review/lenses/",
"./skills/review/output-formats/",
"./skills/config/"
]
```

A new entry `"./skills/tickets/"` must be added.

Reference: `.claude-plugin/plugin.json:9-18`

#### 1.4 Path Configuration

The `tickets` path is already registered in the configuration system:

| Key       | Default        | Description                            |
|-----------|----------------|----------------------------------------|
| `tickets` | `meta/tickets` | Ticket files referenced by create-plan |

Reference: `skills/config/configure/SKILL.md:339`,
`scripts/config-read-path.sh:18`

#### 1.5 Template System

Templates live in `templates/` with three-tier resolution:

1. Explicit config path (`templates.ticket`)
2. User templates directory (`paths.templates`)
3. Plugin default (`templates/ticket.md`)

A new `ticket.md` template must be created. The configure skill's template
management commands must be updated to include `ticket` as an available key.

Reference: `scripts/config-read-template.sh`,
`skills/config/configure/SKILL.md:369-406`

#### 1.6 Companion Scripts

The decisions category uses companion scripts in `skills/decisions/scripts/`:

- `adr-next-number.sh` — Sequential number assignment
- `adr-read-status.sh` — Status extraction from frontmatter

The tickets category should follow the same pattern with:

- `ticket-next-number.sh` — Sequential ticket number assignment
- `ticket-read-status.sh` — Status extraction from frontmatter
- `ticket-read-field.sh` — Generic field extraction (type, priority, etc.)

Reference: `skills/decisions/scripts/adr-next-number.sh`

#### 1.7 Artifact Frontmatter Convention

All artifacts use YAML frontmatter with common base fields (`date`, `type`,
`skill`, `status`) extended per artifact type:

```yaml
---
date: "2026-04-08T01:21:36+01:00"
type: ticket
skill: create-ticket
status: draft
---
```

Reference: `meta/tickets/0022-artifact-metadata-and-lifecycle.md`

#### 1.8 Per-Skill Customisation

Each new skill automatically gains customisation support via:

- `.claude/accelerator/skills/<skill-name>/context.md`
- `.claude/accelerator/skills/<skill-name>/instructions.md`

This requires no implementation — the `config-read-skill-context.sh` and
`config-read-skill-instructions.sh` scripts handle it, and every skill already
includes the injection calls.

Reference: `scripts/config-read-skill-context.sh`,
`scripts/config-read-skill-instructions.sh`

### 2. Existing Ticket Format Analysis

The current `meta/tickets/` directory contains 25 tickets, all following the
same `adr-creation-task` type. They use this consistent structure:

```yaml
---
title: "Short descriptive title"
type: adr-creation-task
status: todo
---

# ADR Ticket: Title

  ## Summary
  [ Y-statement format: "In the context of X, facing Y, we decided for Z
to achieve Q, accepting D." ]

  ## Context and Forces
  [ Bullet list of forces at play ]

  ## Decision Drivers
  [ Bullet list of key drivers ]

  ## Considered Options
  [ Numbered list with brief descriptions ]

  ## Decision
  [ Clear statement of what was decided ]

  ## Consequences
  ### Positive
  ### Negative
  ### Neutral

  ## Source References
  [ Links to source documents ]
```

This format is specific to `adr-creation-task` tickets. A general-purpose
ticket template needs to support broader use cases: features, bugs, technical
debt, spikes, etc.

Reference: `meta/tickets/0001-three-layer-review-system-architecture.md` through
`meta/tickets/0025-configuration-extension-points.md`

### 3. Parallel With the Decisions Category

The decisions category (create-adr, extract-adrs, review-adr) provides the
closest structural precedent for the tickets category:

| Decisions Skill | Tickets Parallel     | Shared Pattern                                                       |
|-----------------|----------------------|----------------------------------------------------------------------|
| `create-adr`    | `create-ticket`      | Interactive creation with context gathering and clarifying questions |
| `extract-adrs`  | `extract-tickets`    | Batch extraction from source documents with user selection           |
| `review-adr`    | `review-ticket`      | Quality review with structured criteria and status transitions       |
| —               | `stress-test-ticket` | Interactive adversarial examination (from `stress-test-plan`)        |
| —               | `refine-ticket`      | Iterative decomposition and improvement                              |
| —               | `list-tickets`       | Discovery and status overview                                        |
| —               | `update-ticket`      | Status transitions and field updates                                 |

#### Key differences from the decisions model:

1. **Numbering**: ADRs use `ADR-NNNN-description.md`. Tickets should use
   `NNNN-description.md` (simpler, as in the existing tickets).

2. **Lifecycle**: ADRs have an append-only lifecycle (proposed → accepted →
   immutable). Tickets need a richer, mutable lifecycle:
   `draft → ready → in-progress → done → cancelled`.

3. **Mutability**: Unlike ADRs, tickets should remain editable throughout
   their lifecycle. Only `done` and `cancelled` are terminal states where
   content changes are discouraged (but not enforced).

4. **Types**: ADRs are a single artifact type. Tickets need multiple types:
   `epic`, `story`, `task`, `bug`, `spike`.

5. **Hierarchy**: ADRs are flat. Tickets need parent-child relationships
   (epics contain stories, stories contain tasks).

6. **Extraction source**: `extract-adrs` extracts from research and plan
   documents. `extract-tickets` should extract from specifications, PRDs,
   meeting notes, research documents, and any document containing requirements.

### 4. Review System Integration

The review system uses a three-layer architecture:

1. **Agent layer**: Specialist reviewer agents produce structured JSON findings
2. **Orchestrator layer**: Skills coordinate, aggregate, and present results
3. **Convention layer**: Shared severity tiers, confidence ratings, output
   schemas

For ticket review, two integration patterns are available:

#### Option A: Reuse the existing review infrastructure

Create ticket-specific review lenses (e.g., `completeness-lens`,
`testability-lens`, `scope-lens`) and a `ticket-review-output-format`. The
`review-ticket` skill would follow the same orchestrator pattern as
`review-plan`, spawning reviewer agents with ticket-specific lenses.

**Pros**: Maximum code reuse, consistent review experience, benefits from
future review system improvements.

**Cons**: Tickets are shorter and simpler than plans/PRs — the full multi-lens
machinery may be heavyweight.

#### Option B: Lightweight inline review

The `review-ticket` skill performs the review directly (no sub-agents), using
a structured checklist of quality criteria. Similar to how `review-adr`
evaluates against quality criteria without spawning reviewer agents.

**Pros**: Simpler, faster, lower token cost for short artifacts.

**Cons**: Doesn't scale if tickets become complex; inconsistent with the
review pattern for plans/PRs.

#### Recommendation: Hybrid approach

Use the **multi-lens review system** (Option A) for `review-ticket`, as this
maintains consistency and allows the same lens infrastructure to evaluate
tickets. But design ticket-specific lenses that are lighter-weight than
code-review lenses — focused on requirements quality rather than code quality.

For `stress-test-ticket`, use the **interactive pattern** from
`stress-test-plan` (Option B) — this is inherently conversational and does
not benefit from parallel agent execution.

### 5. Proposed Skill Set

Based on the analysis, the following 7 skills are recommended for the
`skills/tickets/` category:

#### 5.1 `/extract-tickets` — Extract tickets from documents

**Parallel**: `extract-adrs`

Extract epic-level or story-level tickets from existing documents
(specifications, PRDs, research documents, meeting notes). The skill reads
source documents, identifies requirements and work items, and presents them
for user selection before creating ticket files.

**Key design decisions**:

- Source documents can be any file — not limited to `meta/` directory documents
- Extraction granularity is user-selectable: epic-level (broad themes) or
  story-level (specific deliverables)
- Each extracted ticket gets a draft status and links back to the source
  document
- Batch creation with user selection (like `extract-adrs`)
- Uses `documents-analyser` agents for parallel document scanning

#### 5.2 `/create-ticket` — Interactive ticket creation

**Parallel**: `create-adr`

Interactively walk through creating a well-formed ticket. The skill challenges
the user's understanding, asks probing questions about scope, acceptance
criteria, and edge cases, and produces a ticket that meets quality standards.

**Key design decisions**:

- Socratic approach: challenges vague requirements, asks "what does done
  look like?", probes for missing edge cases
- Gathers context from the codebase using `codebase-locator` and
  `documents-locator` agents
- Supports creating epics, stories, tasks, bugs, and spikes
- For epics: also prompts for initial story decomposition
- For stories: asks about acceptance criteria in Given/When/Then format
- For bugs: asks for reproduction steps, expected vs actual behaviour
- For spikes: asks for time-box, research questions, and exit criteria
- Draft → user review → write to disk cycle

#### 5.3 `/review-ticket` — Multi-lens ticket review

**Parallel**: `review-plan`, `review-adr`

Review a ticket through multiple quality lenses focused on requirements
quality. Uses the existing reviewer agent and review infrastructure with
ticket-specific lenses and output format.

**Proposed ticket review lenses** (new, ticket-specific):

| Lens             | Focus                                                                                             |
|------------------|---------------------------------------------------------------------------------------------------|
| **Completeness** | Are all necessary sections filled? Acceptance criteria present? Dependencies identified?          |
| **Testability**  | Can acceptance criteria be verified? Are they specific and measurable?                            |
| **Scope**        | Is the ticket appropriately sized? Could it be decomposed further? Is it too broad or too narrow? |
| **Clarity**      | Is the language unambiguous? Would a developer understand what to build?                          |
| **Dependencies** | Are external dependencies, prerequisites, and blockers identified?                                |

These are lighter-weight than code-review lenses — focused on requirements
quality rather than implementation quality. They should follow the same
SKILL.md structure as existing lenses (`user-invocable: false`,
`disable-model-invocation: true`, Core Responsibilities, Key Evaluation
Questions, What NOT to Do).

**Output format**: A new `ticket-review-output-format` similar to
`plan-review-output-format`, with `location` referencing ticket sections
rather than plan phases.

**Verdict semantics**: `APPROVE` (ticket is ready for implementation),
`REVISE` (ticket needs changes), `COMMENT` (observations only).

#### 5.4 `/stress-test-ticket` — Interactive adversarial examination

**Parallel**: `stress-test-plan`

Interactively stress-test a ticket by challenging the user on requirements,
scope, edge cases, and assumptions. This is a conversational skill — no
sub-agents, just depth-first adversarial questioning.

**Key design decisions**:

- Follows the `stress-test-plan` pattern exactly: one question at a time,
  depth-first, adversarial but constructive
- Areas to probe:
  - **Scope**: Is this too big? Too small? Can it be delivered incrementally?
  - **Acceptance criteria**: Are they testable? Measurable? Complete?
  - **Edge cases**: What happens with empty data, concurrent access, error
    conditions?
  - **Dependencies**: What needs to be done first? What does this block?
  - **Assumptions**: What is being assumed about the current system?
  - **User impact**: Who is affected? How will they know it works?
  - **Non-functional requirements**: Performance? Security? Accessibility?
  - **Definition of done**: Is it clear when this ticket is complete?
- Captures findings and optionally updates the ticket with agreed changes

#### 5.5 `/refine-ticket` — Decompose and improve tickets

**No direct parallel** (new interaction pattern)

Interactively refine a ticket by decomposing it into smaller tickets,
improving acceptance criteria, adding technical context from the codebase,
or splitting an over-scoped ticket. This is the "grooming" skill.

**Key design decisions**:

- Reads the ticket and spawns `codebase-locator`/`codebase-analyser` agents
  to understand the technical landscape
- Supports several refinement operations:
  - **Decompose**: Split an epic into stories, or a story into tasks
  - **Enrich**: Add technical context, file references, and implementation
    hints from codebase analysis
  - **Sharpen**: Improve vague acceptance criteria into specific, testable
    criteria
  - **Estimate**: Add complexity indicators based on codebase analysis
  - **Link**: Connect related tickets, identify dependencies
- Creates child tickets using the same numbering system
- Updates the parent ticket with links to children

#### 5.6 `/list-tickets` — Discover and filter tickets

**No direct parallel** (new utility skill)

List and filter tickets from the configured tickets directory. Provides an
overview of ticket status, types, and relationships.

**Key design decisions**:

- Reads all ticket files and parses frontmatter
- Supports filtering by: status, type, priority, parent, assignee, tags
- Presents in a structured table with key metadata
- Shows hierarchy (epic → stories → tasks) when viewing epics
- Lightweight — no sub-agents, just filesystem reading and formatting
- Useful as a starting point for other ticket skills ("which ticket should
  I review?")

#### 5.7 `/update-ticket` — Status transitions and field updates

**Parallel**: `review-adr` (status transitions)

Update ticket metadata: status transitions, priority changes, assignment,
and field modifications. Enforces valid transitions and maintains consistency.

**Key design decisions**:

- Valid status transitions:
  - `draft` → `ready`, `cancelled`
  - `ready` → `in-progress`, `draft`, `cancelled`
  - `in-progress` → `done`, `ready`, `cancelled`
  - `done` → (terminal, discourage reopening but don't enforce)
  - `cancelled` → `draft` (allow reactivation)
- Supports batch updates: "mark all stories under epic X as ready"
- Updates frontmatter fields without modifying content
- Logs transitions for audit trail (optional `history` section)

### 6. Ticket Template Design

The ticket template should support all ticket types while keeping the
structure simple. Based on the existing artifact patterns:

```markdown
---
ticket_id: NNNN
date: "YYYY-MM-DDTHH:MM:SS+00:00"
author: Author Name
type: story
status: draft
priority: medium
parent: NNNN
tags: [tag1, tag2]
---

# NNNN: Title as Short Noun Phrase

**Type**: Story | Epic | Task | Bug | Spike
**Status**: Draft
**Priority**: High | Medium | Low
**Author**: Author Name

## Summary

[1-3 sentence description of what this ticket is about and why it matters]

## Context

[Background information, forces at play, relevant constraints.
Link to source documents if extracted.]

## Requirements

[For stories/tasks: specific requirements to be met]
[For epics: high-level goals and themes]
[For bugs: reproduction steps, expected vs actual behaviour]
[For spikes: research questions and time-box]

## Acceptance Criteria

- [ ] [Criterion 1 — specific, testable, measurable]
- [ ] [Criterion 2]
- [ ] [Criterion 3]

[For stories, prefer Given/When/Then format where applicable:

- Given [precondition], when [action], then [expected result]]

## Dependencies

- Blocked by: [ticket references or external dependencies]
- Blocks: [tickets that depend on this one]

## Technical Notes

[Optional: implementation hints, relevant code references,
architectural considerations discovered during refinement]

## References

- Source: `path/to/source-document.md`
- Related: NNNN, NNNN
- Research: `meta/research/YYYY-MM-DD-topic.md`
```

**Type-specific sections**:

For **epics**, add after Requirements:

```markdown
## Stories

- NNNN — [Story title]
- NNNN — [Story title]
```

For **bugs**, replace Requirements with:

```markdown
## Reproduction Steps

1. [Step 1]
2. [Step 2]

**Expected behaviour**: [What should happen]
**Actual behaviour**: [What actually happens]
**Environment**: [Where this occurs]
```

For **spikes**, replace Acceptance Criteria with:

```markdown
## Research Questions

1. [Question 1]
2. [Question 2]

**Time-box**: [Duration]
**Exit criteria**: [What constitutes a successful spike]
```

### 7. Directory Structure

```
skills/tickets/
  scripts/
    ticket-next-number.sh
    ticket-read-status.sh
    ticket-read-field.sh
  create-ticket/
    SKILL.md
  extract-tickets/
    SKILL.md
  review-ticket/
    SKILL.md
  stress-test-ticket/
    SKILL.md
  refine-ticket/
    SKILL.md
  list-tickets/
    SKILL.md
  update-ticket/
    SKILL.md

skills/review/
  output-formats/
    ticket-review-output-format/
      SKILL.md
  lenses/
    completeness-lens/
      SKILL.md
    testability-lens/
      SKILL.md
    scope-lens/
      SKILL.md
    clarity-lens/
      SKILL.md
    dependencies-lens/
      SKILL.md

templates/
  ticket.md
```

### 8. Configuration Integration

#### 8.1 Paths

The `tickets` path key already exists in the configuration system. No changes
needed for basic path support.

Additional paths may be needed for ticket reviews:

```yaml
paths:
  review_tickets: meta/reviews/tickets
```

This would require:

- Adding `review_tickets` to `config-read-path.sh` comments
- Adding it to the `init` skill's directory list
- Adding it to the `configure` skill's path reference

#### 8.2 Review Configuration

If using the multi-lens review infrastructure, ticket review settings should
be added to the review configuration:

```yaml
review:
  ticket_revise_severity: major
  ticket_revise_major_count: 2
```

#### 8.3 Templates

Add `ticket` to the available template keys. This requires:

- Creating `templates/ticket.md` in the plugin
- Updating `config-read-template.sh` validation (if it validates template names)
- Updating `configure` skill template management docs to list `ticket`

### 9. Lifecycle and Workflow Integration

The ticket skills extend the existing development loop:

```
extract-tickets ─→ create-ticket ─→ refine-ticket ─→ review-ticket
        ↓                ↓                ↓                ↓
  meta/tickets/    meta/tickets/    meta/tickets/    meta/reviews/tickets/
        ↓                ↓                ↓
  stress-test-ticket     │          list-tickets
                         ↓
                   create-plan ─→ implement-plan ─→ validate-plan
                         ↓               ↓                ↓
                    meta/plans/    checked-off plan   meta/validations/
```

The ticket lifecycle feeds into the existing planning lifecycle:

1. Extract or create tickets from requirements documents
2. Refine tickets through decomposition and enrichment
3. Review tickets for quality and completeness
4. Stress-test tickets for gaps and assumptions
5. Create implementation plans from approved tickets
6. Implement, validate, and deliver

The `create-plan` skill already accepts ticket references — it reads from
the configured tickets directory. The new skills fill the gap between
"requirements exist somewhere" and "a well-formed ticket is ready for
planning."

### 10. Implementation Phasing

The implementation is split into 6 phases, each suitable for a focused plan.
Phases are ordered by dependency: each phase builds on the prior phases but
is independently valuable once complete.

#### Phase 1: Foundation and Configuration

**Deliverables**:

- `templates/ticket.md` — Ticket template with type-specific sections
- `skills/tickets/scripts/ticket-next-number.sh` — Sequential numbering
- `skills/tickets/scripts/ticket-read-status.sh` — Status extraction
- `skills/tickets/scripts/ticket-read-field.sh` — Generic field extraction
- `.claude-plugin/plugin.json` — Add `"./skills/tickets/"` entry
- `scripts/config-read-path.sh` — Add `review_tickets` to comments
- `skills/config/init/SKILL.md` — Add `review_tickets` directory
- `skills/config/configure/SKILL.md` — Add `ticket` template key, add
  `review_tickets` path to reference, update template management docs

**Why first**: Every subsequent phase depends on the template, scripts, and
plugin registration. No user-facing skills yet, but the infrastructure is
testable in isolation.

**Dependencies**: None.

#### Phase 2: Ticket Creation

**Deliverables**:

- `skills/tickets/create-ticket/SKILL.md` — Interactive ticket creation
- `skills/tickets/extract-tickets/SKILL.md` — Batch extraction from documents

**Why second**: These are the highest-value skills — they produce the tickets
that all other skills operate on. `create-ticket` covers the manual case;
`extract-tickets` covers bootstrapping from existing specifications and PRDs.

**Dependencies**: Phase 1 (template, scripts, plugin registration).

#### Phase 3: Ticket Management

**Deliverables**:

- `skills/tickets/list-tickets/SKILL.md` — Discovery and filtering
- `skills/tickets/update-ticket/SKILL.md` — Status transitions and field
  updates

**Why third**: `list-tickets` is a utility that later skills benefit from
(selecting tickets for review, viewing hierarchy). `update-ticket` closes
the basic lifecycle loop — tickets can be created, listed, and transitioned
through statuses.

**Dependencies**: Phase 1 (template, scripts). Phase 2 is not strictly
required but provides tickets to manage.

#### Phase 4: Ticket Review (Core)

**Deliverables**:

- `skills/review/output-formats/ticket-review-output-format/SKILL.md`
- `skills/review/lenses/completeness-lens/SKILL.md`
- `skills/review/lenses/testability-lens/SKILL.md`
- `skills/review/lenses/clarity-lens/SKILL.md`
- `skills/tickets/review-ticket/SKILL.md` — Review orchestrator

**Why fourth**: The review system requires tickets to exist (Phase 2) and
benefits from `list-tickets` for ticket selection (Phase 3). The three core
lenses (completeness, testability, clarity) are universally applicable to all
ticket types and provide immediate value as a quality gate.

**Dependencies**: Phase 1 (template, scripts), Phase 2 (tickets to review).

#### Phase 5: Ticket Review (Extended Lenses)

**Deliverables**:

- `skills/review/lenses/scope-lens/SKILL.md`
- `skills/review/lenses/dependencies-lens/SKILL.md`

**Why fifth**: These lenses add depth to the review system but are more
situational — scope matters most for stories/epics, dependencies for complex
work with prerequisites. The review orchestrator from Phase 4 already handles
lens selection, so these are picked up automatically once added.

**Dependencies**: Phase 4 (review infrastructure and orchestrator).

#### Phase 6: Interactive Quality

**Deliverables**:

- `skills/tickets/stress-test-ticket/SKILL.md` — Adversarial examination
- `skills/tickets/refine-ticket/SKILL.md` — Decomposition and enrichment

**Why last**: These are high-value but independent of the review system.
`stress-test-ticket` is a standalone interactive skill (no sub-agents).
`refine-ticket` is the most complex skill — it creates child tickets, updates
parent tickets, and spawns codebase agents for enrichment. Both benefit from
the full ticket infrastructure being in place.

**Dependencies**: Phase 1 (template, scripts), Phase 2 (tickets to work with).
Phase 3 is useful for `refine-ticket` (viewing hierarchy after decomposition).

#### Phase Summary

| Phase | Focus               | Skills                            | Lenses                             | Est. Complexity                                |
|-------|---------------------|-----------------------------------|------------------------------------|------------------------------------------------|
| 1     | Foundation          | —                                 | —                                  | Medium (template, 3 scripts, config updates)   |
| 2     | Creation            | create-ticket, extract-tickets    | —                                  | High (2 complex interactive skills)            |
| 3     | Management          | list-tickets, update-ticket       | —                                  | Low-Medium (utility skills)                    |
| 4     | Review (Core)       | review-ticket                     | completeness, testability, clarity | High (orchestrator + 3 lenses + output format) |
| 5     | Review (Extended)   | —                                 | scope, dependencies                | Low (2 additional lenses)                      |
| 6     | Interactive Quality | stress-test-ticket, refine-ticket | —                                  | High (adversarial + decomposition)             |

Phases 3 and 6 have no hard dependency on each other and could be reordered
or developed in parallel. Phases 4 and 5 are strictly sequential. Phase 2
must precede phases 4-6 (tickets must exist before they can be reviewed,
stress-tested, or refined).

## Code References

- `skills/decisions/create-adr/SKILL.md` — Primary pattern for interactive
  artifact creation with context gathering
- `skills/decisions/extract-adrs/SKILL.md` — Primary pattern for batch
  extraction from source documents
- `skills/decisions/review-adr/SKILL.md` — Pattern for quality review with
  status transitions
- `skills/planning/stress-test-plan/SKILL.md` — Pattern for interactive
  adversarial examination
- `skills/planning/review-plan/SKILL.md` — Pattern for multi-lens review
  orchestration
- `skills/planning/create-plan/SKILL.md` — Shows how tickets are already
  consumed by the planning workflow
- `skills/review/lenses/architecture-lens/SKILL.md` — Pattern for lens
  SKILL.md structure
- `skills/review/output-formats/plan-review-output-format/SKILL.md` — Pattern
  for review output format specification
- `skills/decisions/scripts/adr-next-number.sh` — Pattern for sequential
  numbering script
- `.claude-plugin/plugin.json` — Plugin registration
- `scripts/config-read-path.sh` — Path configuration (tickets already
  registered)
- `templates/adr.md` — Template structure pattern
- `meta/tickets/0001-three-layer-review-system-architecture.md` — Existing
  ticket format example

## Architecture Insights

### Pattern: Lifecycle-Managed Filesystem Artifacts

The plugin's core pattern is: skills create structured markdown artifacts with
YAML frontmatter in predictable `meta/` directory locations. Each artifact type
has a lifecycle (draft → active → complete; proposed → accepted → superseded),
status transitions are enforced at the skill level, and artifacts communicate
between skills via the filesystem. Tickets fit naturally into this pattern.

### Pattern: Three-Skill Decomposition Per Artifact Type

The decisions category established the pattern of decomposing artifact
management into three skills aligned with distinct user intents: create
(interactive generation), extract (batch mining from existing documents), and
review (quality evaluation with status transitions). The tickets category
extends this to seven skills because tickets have a richer lifecycle — they
need refinement, status management, and discovery/filtering that ADRs don't.

### Pattern: Interactive vs Automated Quality Assessment

The plugin distinguishes between automated multi-lens review (broad, parallel,
structured output) and interactive stress-testing (deep, sequential,
conversational). Both patterns should apply to tickets: `/review-ticket` for
automated quality gates, `/stress-test-ticket` for deep adversarial probing.

### Pattern: Configuration Extensibility

Every path, template, agent name, and review setting is configurable. New
skills inherit this automatically through the preprocessor scripts. The
ticket skills should add `review_tickets` to the paths configuration and
`ticket` to the templates configuration, but otherwise require no
configuration infrastructure work.

## Historical Context

- The `meta/tickets/` directory already exists and is used by `create-plan`
  as a source for planning context
- The existing 25 tickets all use the `adr-creation-task` type, which is a
  narrow format specific to ADR extraction work
- Ticket 0021 (artifact persistence lifecycle) established the principle
  that every skill producing structured output must write to `meta/`
- Ticket 0022 (artifact metadata and lifecycle) established the common base
  YAML frontmatter schema
- Ticket 0024 (configuration system architecture) established the config
  infrastructure that new skills leverage automatically
- ADR-0001 (context isolation principles) establishes that the filesystem
  is the sole inter-phase communication channel — tickets are a natural
  extension of this principle

## Resolved Questions

1. **Ticket numbering format**: Use plain `NNNN-description.md` as the default
   (consistent with the 25 existing tickets). A future enhancement will add
   configurable filename patterns — e.g.,
   `{project-code}-{ticket-number}-{description}.md`
   — to support project/team code prefixes (`XXX-NNNN-description.md`) needed
   for eventual sync with external trackers (Jira, Linear, Trello). This
   configurability is **out of scope** for the initial implementation.

2. **Review lens count**: All 5 ticket-specific lenses (Completeness,
   Testability, Scope, Clarity, Dependencies) will be included in the design.
   Implementation will be phased — core lenses first, with the remaining lenses
   in subsequent plans — but all 5 are in scope for the overall feature.

3. **Ticket review persistence**: Persist to `meta/reviews/tickets/` following
   the same pattern as plan and PR reviews — separate numbered documents with
   YAML frontmatter, per-lens results, and appendable re-review history. This
   maintains consistency across all review artifacts and provides a full audit
   trail. Requires adding `review_tickets` to the paths configuration.

4. **Epic-story hierarchy enforcement**: No enforcement. All ticket types are
   standalone by default. The `parent` field is optional — when present, skills
   like `list-tickets` and `refine-ticket` display hierarchy, but nothing
   requires it. This follows the plugin's philosophy of simplicity and optional
   structure.

5. **Integration with external trackers**: `meta/tickets/` is the sole source
   of truth for now. Sync with external systems (Jira, Linear, Trello, GitHub
   Issues) is a future enhancement — the configurable filename pattern (resolved
   question 1) lays groundwork for it. **Out of scope** for the initial
   implementation.

6. **Ticket-to-plan automation**: **Out of scope** for now. Automatically
   updating a ticket's status when `create-plan` references it is desirable
   but introduces coupling that should be designed carefully once the ticket
   lifecycle is proven. A lighter future alternative: `create-plan` adds a
   `plan` cross-reference field to the ticket frontmatter without changing
   status.

7. **Existing ticket migration**: Coexist. The 25 existing `adr-creation-task`
   tickets remain untouched. New tickets start from 0026 — the
   `ticket-next-number.sh` script finds the highest existing `NNNN` and
   increments, so continuity is automatic. `list-tickets` displays them with
   their existing `adr-creation-task` type, distinguishing them from new
   tickets.
