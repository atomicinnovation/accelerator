---
name: create-adr
description: Interactively create an architecture decision record (ADR). Use
  when the user wants to document an architectural decision, technology choice,
  or significant design decision. Guides through context gathering, options
  analysis, and consequence documentation.
argument-hint: "[topic or description] [--supersedes ADR-NNNN]"
disable-model-invocation: true
---

# Create Architecture Decision Record

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

You are tasked with guiding the user through creating an architecture decision
record (ADR) — a concise document capturing a significant architectural
decision, its context, the options considered, and the consequences.

## Initial Setup

When this command is invoked:

1. **Check if parameters were provided**:

- If a topic/description was provided, proceed directly to context gathering
- If `--supersedes ADR-NNNN` was provided, note the supersession target
- If no parameters provided, respond with:

```
I'll help you create an architecture decision record. Please provide:
1. The topic or decision to document (e.g., "use PostgreSQL for user data")
2. Any relevant context or constraints

You can also specify if this supersedes an existing ADR:
`/accelerator:create-adr use Redis for caching --supersedes ADR-0003`
```

Then wait for the user's input.

## Process Steps

### Step 1: Determine ADR Number

1. Run the companion script to get the next ADR number:

```
${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/adr-next-number.sh
```

2. If `--supersedes ADR-NNNN` was specified:
   - Find the target ADR file by matching `meta/decisions/ADR-NNNN-*.md`
   - Verify exactly one file matches the glob pattern (error if zero or
     multiple matches)
   - Read the target ADR's status using the companion script:
     ```
     ${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/adr-read-status.sh <resolved-path>
     ```
   - Verify the target ADR is in `accepted` status (only accepted ADRs can be
     superseded). This is an early-fail check to avoid wasted effort — the
     status will be re-verified before writing in Step 4.
   - If not `accepted`, inform the user and ask how to proceed

### Step 2: Gather Context

1. **Spawn agents to gather relevant context** (in parallel):

- Use **documents-locator** to find related research, plans, and existing ADRs
  in `meta/`
- Use **codebase-locator** to find relevant code related to the decision topic

2. **Read any directly mentioned files** fully

3. **Present gathered context and ask clarifying questions**:

```
Based on my research, here's what I found relevant to this decision:

**Related documents:**
- [list of relevant meta documents]

**Related code:**
- [list of relevant code areas]

**Existing ADRs on related topics:**
- [list of related ADRs, if any]

Before I draft the ADR, I'd like to understand:
1. What forces or constraints are driving this decision?
2. What alternatives have you considered?
3. Are there specific tradeoffs you want to highlight?
```

If context gathering finds nothing relevant (e.g., first ADR in a new project),
skip the context sections and present:

```
No existing documents or ADRs found related to this topic. I'll draft the ADR
based on the information you provide.

Before I draft, I'd like to understand:
1. What forces or constraints are driving this decision?
2. What alternatives have you considered?
3. Are there specific tradeoffs you want to highlight?
```

Wait for user input before proceeding.

### Step 3: Draft the ADR

1. **Gather metadata** by running:

```
${CLAUDE_PLUGIN_ROOT}/skills/research/research-codebase/scripts/research-metadata.sh
```

2. **Draft the ADR** using the template below and present it to the user for
   review:

```
Here's my draft ADR. Please review and let me know if you'd like any changes
before I write it to disk:

[draft content]
```

Wait for user approval or revision requests before writing.

3. **Iterate** on the draft based on user feedback. Only proceed to writing
   when the user approves.

### Step 4: Write the ADR

1. Create the `meta/decisions/` directory if it doesn't exist

2. Write the ADR to:
   `meta/decisions/ADR-NNNN-description.md`
   where NNNN is the number from Step 1 and description is a kebab-case summary

3. If this supersedes an existing ADR:
   - Read the superseded ADR's current status to confirm it's `accepted`
   - Update ONLY the superseded ADR's frontmatter:
     - Change `status: accepted` to `status: superseded`
     - Add `superseded_by: ADR-MMMM` (where MMMM is the new ADR number)
   - Do NOT modify any other content in the superseded ADR

4. Present the result:

```
ADR created: `meta/decisions/ADR-NNNN-description.md`
Status: proposed

[If supersession]: Updated ADR-XXXX status to "superseded"

Next steps:
- Review and refine while in "proposed" status
- When ready, use `/accelerator:review-adr` to accept or reject
```

## ADR Template

Use this exact template structure when generating ADRs:

```markdown
---
adr_id: ADR-NNNN
date: "YYYY-MM-DDTHH:MM:SS+00:00"
author: Author Name
status: proposed
supersedes: ADR-NNNN     # only include if this ADR replaces another
tags: [tag1, tag2]
---

# ADR-NNNN: Title as Short Noun Phrase

**Date**: YYYY-MM-DD
**Status**: Proposed
**Author**: Author Name

## Context

[Forces at play — technological, political, social, project-specific.
Value-neutral language describing facts, not advocating.]

## Decision Drivers

- [Driver 1]
- [Driver 2]

## Considered Options

1. **Option A** — Brief description
2. **Option B** — Brief description
3. **Option C** — Brief description

## Decision

[The chosen option and why, stated in active voice: "We will..."]

## Consequences

### Positive

- [Consequence 1]

### Negative

- [Consequence 1]

### Neutral

- [Consequence 1]

## References

- `meta/research/YYYY-MM-DD-topic.md` — Related research
- `meta/decisions/ADR-NNNN.md` — Related/superseded ADR
```

## Quality Guidelines

When drafting ADRs, follow these principles:

- **Concise**: One to two pages maximum. Match length to problem complexity.
- **Assertive**: Use active voice ("We will...", "We chose...")
- **Balanced**: Include genuine pros AND cons. Avoid the "Fairy Tale" pattern
  (only pros, no cons)
- **Honest options**: Only include options that were genuinely considered. Avoid
  "Dummy Alternatives" (non-viable options to make preferred choice look good)
- **Focused**: Each ADR captures ONE decision. If you find multiple decisions,
  suggest creating separate ADRs.
- **Context-rich**: Explain WHY, not just WHAT. Future readers need to
  understand the forces at play.

## Anti-Patterns to Avoid

- **Fairy Tale**: Only listing positive consequences
- **Dummy Alternative**: Including obviously non-viable options
- **Mega-ADR**: Multi-page documents crammed with implementation detail
- **Blueprint in Disguise**: Reads like a cookbook, not a decision journal
- **Missing context**: Decision without the forces that drove it

## Important Notes

- New ADRs ALWAYS start with status `proposed`
- ADR numbers are NEVER reused — always increment from the highest existing
- File naming is `ADR-NNNN-description.md` (e.g., `ADR-0001-use-jujutsu.md`)
- Only modify existing ADRs to update status fields during supersession
- Cross-reference related documents in the References section
- Use `documents-locator` and `codebase-locator` agents for context, not deep
  file reads in the main context
- **Dual status fields**: The template includes status in both YAML frontmatter
  (`status: proposed`) and the body (`**Status**: Proposed`). The frontmatter
  is the authoritative source of truth — `adr-read-status.sh` reads only
  frontmatter. The body line is for human readability. When updating status,
  ALWAYS update both locations.
- Before writing a new ADR file, verify the target path does not already exist
  to prevent accidental overwrites from concurrent invocations
