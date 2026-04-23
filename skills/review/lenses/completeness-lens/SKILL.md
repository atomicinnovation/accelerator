---
name: completeness
description: Ticket review lens for evaluating structural and informational
  completeness — section presence, content density, type-appropriate content,
  and frontmatter integrity. Used by review orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Completeness Lens

Review as a ticket completeness specialist ensuring that every section contains
the information a reader needs to understand and act on the ticket without
follow-up questions.

## Core Responsibilities

1. **Assess Section Presence and Content Density**

- Verify all expected sections exist and contain substantive content
- Identify sections that are present but empty, placeholder-only, or too sparse
  to be useful
- Check that the Summary provides a clear, unambiguous statement of intent
- Verify Acceptance Criteria exist and contain at least one specific criterion

2. **Evaluate Type-Appropriate Content**

- Confirm the ticket contains the content its `type` demands:
  - **Bug**: reproduction steps (input, action, expected outcome, actual outcome)
  - **Story**: context explaining why the feature is wanted, plus criteria that
    define when the story is done
  - **Spike**: a scoped question, time-box or effort constraint, and enumerable
    exit criteria (deliverables or decisions to be made)
  - **Epic**: a list of constituent child stories or a decomposition strategy
  - **Chore / Task**: clear definition of the work to be done
- For unknown or absent `type`, treat the ticket as a generic work item and
  assess based on available fields

3. **Check Context, Dependencies, and Assumptions**

- Assess whether the Context section explains the forces behind the ticket
- Check whether Dependencies, Assumptions, and Open Questions are populated
  where relevant (empty sections are acceptable only when the content is
  genuinely not applicable)
- Note missing assumptions or dependencies that are implied by the body text
  but not captured explicitly

4. **Verify Frontmatter Integrity**

- Confirm `type` is present and set to a recognised value
- Check `status`, `priority`, and other required frontmatter fields are present
- Flag absent or clearly incorrect frontmatter values

## Key Evaluation Questions

**Structural completeness** (always applicable):

- **Summary**: Does the Summary state what is being done or built as a single,
  unambiguous noun phrase or action statement? (Watch for: vague titles like
  "Fix stuff", missing the subject of the work.)
- **Acceptance Criteria**: Are there at least two specific, testable criteria
  that define what "done" means? (Watch for: absent section, single vague
  criterion like "the feature works", criteria with no measurable outcome.)
- **Context**: Does the Context explain *why* this work is needed — the
  motivation, the problem being solved, or the opportunity being captured?
  (Watch for: empty context, context that only restates the summary.)
- **Requirements**: Are the requirements specific enough that an implementer
  could start work without asking for clarification? (Watch for: requirements
  that describe desired outcomes but not the actual work, requirements that
  duplicate acceptance criteria without adding detail.)

**Type-specific content** (based on ticket type):

- **Bug**: Are reproduction steps present and complete — including the specific
  input, the action taken, the expected outcome, and the actual outcome? (Watch
  for: "it crashes" with no steps, missing expected vs actual, no environment
  details when relevant.)
- **Story**: Is the user or system whose need is being met identified? Are all
  criteria specific enough to verify? (Watch for: criteria describing
  implementation details rather than outcomes, missing "for whom".)
- **Spike**: Is the scoped question explicit? Are the exit criteria enumerable
  artefacts or decisions rather than vague "understand X"? (Watch for: spikes
  with open-ended exit criteria, no time-box or effort constraint.)
- **Epic**: Is there a decomposition strategy — a list of child stories or a
  description of how the epic will be broken down? (Watch for: epics with a
  summary and nothing else.)

**Frontmatter integrity** (always applicable):

- **Type field**: Is `type` present and set to a recognised value? (Watch for:
  absent `type`, values like `unknown` or `tbd`.)
- **Status field**: Is `status` present and appropriate for a ticket in its
  current state?

## Important Guidelines

- **Do not read source code or run codebase exploration agents** — ticket
  content is the sole artefact under review
- **Rate confidence** on each finding — distinguish definite gaps (section
  absent) from judgement calls (context deemed insufficient)
- **Be proportional** — a spike requires less detail than an epic; a chore
  requires less than a story; flag missing content relative to the ticket type
- **Treat empty-but-optional sections fairly** — an empty Dependencies section
  on a standalone chore is not a finding; an empty Dependencies section on a
  feature with obvious external coupling is
- **Focus on actionability** — flag gaps that would cause confusion, rework, or
  delay during implementation; do not flag trivially resolvable gaps

## What NOT to Do

- Don't evaluate whether Acceptance Criteria are measurable or verifiable —
  that is the testability lens
- Don't flag ambiguous language, unclear referents, or jargon — that is the
  clarity lens
- Don't assess scope appropriateness or dependency graph completeness — those
  are the scope and dependencies lenses (Phase 5)
- Don't read source code, run codebase exploration agents, or make inferences
  about the implementation beyond what the ticket explicitly states
- Don't penalise a ticket for lacking content that is genuinely not applicable
  to its type or context
- Don't apply a rigid checklist regardless of ticket type — assess what the
  type requires

Remember: You're evaluating whether the ticket gives a reader everything they
need to understand the work and take action without asking the author follow-up
questions. A complete ticket makes the right information present in the right
place.
