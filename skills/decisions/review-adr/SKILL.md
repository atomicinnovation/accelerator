---
name: review-adr
description: Review an architecture decision record for quality and
  completeness, then accept, reject, or suggest revisions. Enforces ADR
  immutability — only proposed ADRs can be modified, accepted ADRs can only
  transition to superseded or deprecated. Use when a proposed ADR is ready for
  review, or when an accepted ADR needs to be deprecated.
argument-hint: "[@meta/decisions/ADR-NNNN.md] [--deprecate reason]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/*)
---

# Review Architecture Decision Record

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions meta/decisions`

You are tasked with reviewing ADRs for quality and managing their lifecycle
status transitions, enforcing the append-only immutability model.

## Initial Setup

When this command is invoked:

1. **Check if parameters were provided**:

- If an ADR path was provided, read it fully and proceed to review
- If `--deprecate` was specified with a path, proceed to deprecation workflow
- If `--deprecate` was specified without a path:
  - Scan the configured decisions directory for ADRs in `accepted` status
  - Present them for selection:
    ```
    I found the following accepted ADRs available for deprecation:

    1. `{decisions directory}/ADR-0001-use-jujutsu.md` — Use Jujutsu for Version
       Control
    2. `{decisions directory}/ADR-0002-filesystem-chaining.md` — Filesystem-Mediated
       Skill Chaining

    Which ADR would you like to deprecate? (enter number or path)
    ```
  - After selection, ask for the deprecation reason and proceed to deprecation
    workflow
- If no parameters provided:
  - Scan the configured decisions directory for ADRs in `proposed` status
  - Present them for selection:

```
I found the following proposed ADRs ready for review:

1. `{decisions directory}/ADR-0001-use-jujutsu.md` — Use Jujutsu for Version Control
2. `{decisions directory}/ADR-0003-append-only-lifecycle.md` — Append-Only ADR
   Lifecycle

Which ADR would you like to review? (enter number or path)

Tip: To deprecate an accepted ADR, use:
`/accelerator:review-adr --deprecate`
```

Wait for user selection.

## Mutability Rules

Before making ANY changes to an ADR, check its status using:

```
${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/adr-read-status.sh <adr-path>
```

Then apply these rules:

| Current Status | Content Editable? | Permitted Transitions                          |
|----------------|-------------------|------------------------------------------------|
| `proposed`     | Yes               | → `accepted`, → `rejected`                     |
| `accepted`     | No                | → `superseded` (via create-adr), → `deprecated`|
| `rejected`     | No                | None (terminal)                                |
| `superseded`   | No                | None (terminal)                                |
| `deprecated`   | No                | None (terminal)                                |

**CRITICAL**: If the ADR status is anything other than `proposed`, you MUST NOT
modify its content. You may only update the status field and related metadata
fields (`superseded_by`, `deprecated_reason`) for permitted transitions.

If the user asks to modify a non-proposed ADR's content, explain:

```
This ADR has status "[status]" and is immutable. Its content cannot be changed.

To revise this decision, create a new ADR that supersedes it:
`/accelerator:create-adr [new decision] --supersedes ADR-NNNN`
```

## Process Steps

### For Proposed ADRs — Quality Review

#### Step 1: Read and Analyse

1. Read the ADR fully
2. Check status is `proposed` (if not, apply mutability rules above)
3. Spawn agents for context (in parallel):
   - **documents-locator** to find related ADRs and source documents
   - **codebase-locator** to verify technical claims in the ADR

#### Step 2: Quality Review

Evaluate the ADR against these quality criteria:

**Context Completeness:**
- Are the forces and constraints clearly described?
- Is the context value-neutral (describing facts, not advocating)?
- Would a new team member understand WHY this decision matters?

**Decision Drivers:**
- Are the key drivers listed?
- Do they connect logically to the decision?

**Options Analysis:**
- Are the considered options genuinely viable? (No "Dummy Alternatives")
- Are there obvious options missing that should be considered?
- Is each option described clearly enough to understand?

**Decision Statement:**
- Is the decision clear and assertive? ("We will..." not "We might...")
- Does it follow from the context and drivers?
- Is it specific enough to be actionable?

**Consequences:**
- Are there both positive AND negative consequences? (No "Fairy Tale")
- Are consequences honest and realistic?
- Are neutral consequences included where relevant?

**Overall Quality:**
- Is the length appropriate for the complexity of the decision?
- Is it a decision record, not a blueprint or cookbook?
- Are references to related documents included?

#### Step 3: Present Review

```
## ADR Review: ADR-NNNN — [Title]

**Overall Assessment**: [Strong / Adequate / Needs Revision]

### Strengths:
- [What's done well]

### Suggestions:
- [Specific improvements, if any]

### Issues:
- [Problems that should be fixed before acceptance, if any]

---

**Actions available:**
1. **Accept** — Mark as accepted (ADR becomes immutable)
2. **Reject** — Mark as rejected with reason (ADR becomes immutable)
3. **Revise** — Make suggested improvements (stays proposed)

What would you like to do?
```

Wait for user decision.

#### Step 4: Execute Action

**If Accept:**
1. Update the ADR's frontmatter:
   - Change `status: proposed` to `status: accepted`
2. Update the in-body status line:
   - Change `**Status**: Proposed` to `**Status**: Accepted`
3. Confirm:
   ```
   ADR-NNNN has been accepted. It is now immutable — content can no longer be
   changed. If this decision needs to be revised in the future, create a new
   ADR that supersedes it.
   ```

**If Reject:**
1. Ask for a rejection reason
2. Update the ADR's frontmatter:
   - Change `status: proposed` to `status: rejected`
   - Add `rejected_reason: "[reason]"`
3. Update the in-body status line:
   - Change `**Status**: Proposed` to `**Status**: Rejected`
4. Confirm:
   ```
   ADR-NNNN has been rejected. Reason: [reason]
   ```

**If Revise:**
1. Make the suggested improvements to the ADR content
2. Present the updated ADR for another round of review
3. Return to Step 2

### For Accepted ADRs — Deprecation Only

If the ADR is in `accepted` status and `--deprecate` was specified:

1. Ask for or confirm the deprecation reason
2. Update the ADR's frontmatter:
   - Change `status: accepted` to `status: deprecated`
   - Add `deprecated_reason: "[reason]"`
3. Update the in-body status line:
   - Change `**Status**: Accepted` to `**Status**: Deprecated`
4. Confirm:
   ```
   ADR-NNNN has been deprecated. Reason: [reason]

   Note: The ADR content remains unchanged. If a replacement decision is
   needed, use `/accelerator:create-adr` to create a new ADR.
   ```

### For Terminal Status ADRs

If the ADR is in `rejected`, `superseded`, or `deprecated` status:

```
ADR-NNNN is in "[status]" status, which is terminal. No further changes are
permitted.

[If superseded]: This ADR was superseded by ADR-MMMM. See
`{decisions directory}/ADR-MMMM-description.md`.
[If deprecated]: Reason: [deprecated_reason from frontmatter]
[If rejected]: Reason: [rejected_reason from frontmatter]
```

## Important Notes

- **ALWAYS** check status before any modification — this is the core
  enforcement mechanism
- Never modify content of non-proposed ADRs, only status metadata
- Supersession is handled by `create-adr` with `--supersedes`, not by this
  skill
- Be constructive in reviews — suggest specific improvements, not vague
  criticism
- Quality review is a service, not a gate — the user decides whether to accept
- **Enforcement boundary**: Immutability is enforced at the skill level, not
  the filesystem level. Direct file edits outside of ADR skills bypass this
  protection. This is an accepted tradeoff — skill-level enforcement is
  simpler and sufficient for the intended workflow. If an ADR appears to have
  been modified outside the skills, note this to the user during review.
