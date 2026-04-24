---
name: review-ticket
description: Review a ticket through multiple ticket-quality lenses and
  collaboratively iterate based on findings. Use when the user wants to
  evaluate a ticket before implementation or escalation.
argument-hint: "[path to ticket file]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/ticket-read-*)
---

# Review Ticket

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh review-ticket`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-review.sh ticket`

**Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`
**Ticket reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_tickets meta/reviews/tickets`

You are tasked with reviewing a ticket through quality lenses and then
collaboratively iterating the ticket based on findings.

## Initial Response

When this command is invoked:

1. **Check if a ticket path or number was provided**:

   - **Path-like** (contains `/` or ends in `.md`): treat as a file path. If
     the file does not exist, print `"No ticket at <path>."` and offer to run
     `/list-tickets` to find a valid path.
   - **Numeric**: treat as a ticket number. Zero-pad to 4 digits, then glob
     `{tickets directory}/NNNN-*.md`.
     - Zero matches: print `"No ticket numbered NNNN found in {tickets
       directory}."` and offer to run `/list-tickets`.
     - One match: use it.
     - Multiple matches: list them and ask the user to select.
   - If a path or number was provided and resolves correctly, read the ticket
     immediately and FULLY, then begin the review process.
   - If optional focus arguments were provided (e.g., "focus on testability"),
     note them for lens selection.

2. **If no ticket path or number provided**, respond with:

```
I'll help you review a ticket. Please provide:
1. The path to the ticket file (e.g., `{tickets directory}/0042-my-ticket.md`)
2. (Optional) A ticket number shorthand (e.g., `/review-ticket 42`)
3. (Optional) Focus areas to emphasise (e.g., "focus on testability")

Tip: Use `/list-tickets` to find the ticket you want to review.
```

Then wait for the user's input.

## Available Review Lenses

| Lens             | Lens Skill          | Focus                                                                   |
|------------------|---------------------|-------------------------------------------------------------------------|
| **Clarity**      | `clarity-lens`      | Unambiguous referents, internal consistency, jargon handling            |
| **Completeness** | `completeness-lens` | Section presence, content density, type-appropriate content             |
| **Dependency**   | `dependency-lens`   | Implied couplings not captured — blockers, consumers, external systems  |
| **Scope**        | `scope-lens`        | Right-sized, single coherent unit of work; decomposition; orthogonality |
| **Testability**  | `testability-lens`  | Measurable criteria, verifiable outcomes, verification framing          |

> Note: completeness flags an *absent* Dependencies section; dependency flags
> an *empty or underspecified* section whose contents fail to name every
> coupling the ticket implies.

## Process Steps

### Step 1: Read and Understand the Ticket

1. **Read the ticket file FULLY** — never use limit/offset
2. **Parse the frontmatter** to note `type` (bug, story, spike, epic, etc.)
   and `status`
3. **Read any documents referenced in the References section** — these provide
   context the lenses may need; do not read source code
4. **Check for existing reviews**: Glob for review documents matching
   `{ticket reviews directory}/{ticket-stem}-review-*.md`. If any are found:
   - Read the most recent review document (highest review number)
   - Note the previous verdict, review pass count, and key findings
   - Inform the user: "I found {N} previous review(s) of this ticket. The
     most recent (review {N}, verdict: {verdict}) will be used as context."
   - The agents do NOT receive the previous review — they review the ticket
     fresh. But the aggregation step (Step 4) should reference the previous
     review when composing cross-cutting themes and the assessment.
   - If the prior review file exists but cannot be parsed (malformed
     frontmatter), warn the user and proceed as if no prior review exists.

   The new review creates a **new file** with the next review number (e.g.,
   `-review-2.md`). Previous review files are never modified.

### Step 2: Select Review Lenses

By default, run every lens registered in `BUILTIN_TICKET_LENSES` unless the
user has provided focus arguments or config restricts the selection. The five
ticket lenses cover orthogonal concerns, so there is no relevance-based
auto-selection.

**If the user provided focus arguments:**

- Map the focus areas to the corresponding lenses
- Include any additional lenses that are clearly relevant
- Briefly explain which lenses you're running

**If no focus arguments were provided:**

Run all built-in ticket lenses unless:
- A lens is listed in `disabled_lenses` — remove it from the active set
- The user's configured `core_lenses` has filtered this to a subset (see below)

When `core_lenses` is set in config, apply it as the *minimum required set*;
add any remaining non-disabled lenses up to `max_lenses`. This means users
who previously pinned `core_lenses` to the Phase 4 ticket lenses
(`completeness`, `testability`, `clarity`) will also receive `scope` and
`dependency` on upgrade, unless they add those names to `disabled_lenses` or
set `max_lenses` to their subset size.

Present the selection briefly — enumerate the chosen lenses with a one-line
focus each — then wait for confirmation before spawning reviewers. The
confirmation gate is preserved even though the default always selects every
lens; the gate is useful when focus args or config have narrowed the set.

Example (default path, no focus args, no `core_lenses` restriction):

```
I'll review this ticket through all ticket lenses (clarity, completeness,
dependency, scope, testability). Shall I proceed?
```

Wait for confirmation before spawning reviewers.

### Step 3: Spawn Review Agents

For each selected lens, spawn the {reviewer agent} agent with a prompt that
includes the paths to the lens skill and output format files. Do NOT read these
files yourself — the agent reads them in its own context.

Compose each agent's prompt following this template:

```
You are reviewing a ticket through the [lens name] lens.

## Context

The ticket is at [path]. Read it fully.
Also read any source documents listed in the ticket's References section.

## Analysis Strategy

1. Read your lens skill and output format files (see paths below)
2. Read the ticket file fully
3. Read referenced documents from the ticket's References section if present
4. Evaluate the ticket through your lens, applying each key question
5. Reference specific ticket sections in your findings using the `location`
   field (e.g., "Acceptance Criteria", "Requirements", "Frontmatter: type")

IMPORTANT: Do not evaluate the codebase — ticket content (and any documents
it explicitly references) is the sole artefact under review. Do not run
codebase exploration agents or read source files unless the ticket's
References section explicitly links to them.

## Lens

Read the lens skill at the path listed in the Lens Catalogue table in the
review configuration above. If no review configuration is present, use:
${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md

## Output Format

Read the output format at:
${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/ticket-review-output-format/SKILL.md

IMPORTANT: Return your analysis as a single JSON code block. Do not include
prose outside the JSON block.
```

Spawn all selected agents **in parallel** using the Task tool with
`subagent_type: "!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agent-name.sh reviewer`"`.

**IMPORTANT**: Wait for ALL review agents to complete before proceeding.

**Handling malformed agent output**:

If an agent's response is not a clean JSON block, apply this extraction
strategy:

1. Look for a JSON code block fenced with triple backticks (optionally with
   a `json` language tag)
2. If found, extract and parse the content within the fences
3. If the extracted JSON is valid, use it normally
4. If no JSON code block is found, or the JSON within it is invalid, apply
   the fallback: treat the agent's entire output as a single finding with
   `"suggestion"` severity (marked `synthetic: true`), attributed to that
   agent's lens

   Note: `"suggestion"` severity is used here (not `"major"` as in
   `review-plan`) so a single flaky agent cannot deterministically force a
   REVISE verdict when `ticket_revise_severity` is `major` or higher.

When falling back, warn the user that the agent's output could not be parsed
and present the raw agent output so the user can see what the agent found.
Include remediation guidance: "Try re-running with a narrower lens selection,
or file a bug with the raw output above."

### Step 4: Aggregate and Curate Findings

Once all reviews are complete:

1. **Parse agent outputs**: Extract the JSON block from each agent's response
   (see the extraction strategy in Step 3). Collect the `summary`, `strengths`,
   and `findings` arrays from each.

2. **Aggregate across agents**:
   - Combine all `findings` arrays into a single list
   - Combine all `strengths` arrays into a single list
   - Collect all `summary` strings

3. **Deduplicate findings**: Where multiple agents flag the same section with
   similar concerns, consider merging — but only when the findings address the
   same underlying concern from different lens perspectives. When in doubt,
   keep findings separate.

   When merging:
   - Combine the bodies, attributing each part to its lens
   - Use the highest severity among the merged findings
   - Use the highest confidence among the merged findings

4. **Prioritise findings**:
   - Sort by severity: critical > major > minor > suggestion
   - Within the same severity, sort by confidence: high > medium > low

5. **Determine suggested verdict**:

   If review configuration provides verdict overrides above, apply those
   thresholds instead of the defaults below:
   - If `ticket_revise_severity` is `none`, skip the severity-based REVISE
     rule (major count rule still applies independently)
   - If any findings at or above the ticket revise severity
     ({ticket revise severity}) exist → suggest `REVISE`
   - If {ticket revise major count} or more `"major"` findings exist
     → suggest `REVISE`
   - If fewer major findings than the threshold, or only minor/suggestion
     → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`

   Verdict meanings:
   - `APPROVE` — ticket is ready for implementation
   - `REVISE` — ticket needs changes before implementation
   - `COMMENT` — observations only, ticket is acceptable as-is

   When presenting a `COMMENT` verdict with major findings, note: "Ticket is
   acceptable but could be improved — see major findings below."

6. **Identify cross-cutting themes**: Look for findings that appear across
   multiple lenses — issues flagged by 2+ agents reinforce each other and
   should be highlighted in the summary.

7. **Compose the review summary**:

   ```markdown
   ## Ticket Review: [Ticket Title]

   **Verdict:** [APPROVE | REVISE | COMMENT]

   [Combined assessment: synthesise each agent's summary into 2-3 sentences
   covering the overall quality of the ticket across all lenses]

   ### Cross-Cutting Themes
   [Issues that multiple lenses identified — these deserve the most attention]
   - **[Theme]** (flagged by: [lenses]) — [description]

   ### Findings

   #### Critical
   - 🔴 **[Lens]**: [title]
     **Location**: [ticket section]
     [First 1-2 sentences of body as summary]

   #### Major
   - 🟡 **[Lens]**: [title]
     **Location**: [ticket section]
     [First 1-2 sentences of body as summary]

   #### Minor
   - 🔵 **[Lens]**: [title]
     **Location**: [ticket section]
     [First 1-2 sentences of body as summary]

   #### Suggestions
   - 🔵 **[Lens]**: [title]
     **Location**: [ticket section]
     [First 1-2 sentences of body as summary]

   ### Strengths
   - ✅ [Aggregated and deduplicated strengths from all agents]

   ### Recommended Changes
   [Ordered list of specific, actionable changes to the ticket, prioritised by
   impact. Each should reference the finding(s) it addresses.]

   1. **[Change description]** (addresses: [finding titles])
      [Specific guidance on what to modify in the ticket]

   ---
   *Review generated by /review-ticket*
   ```

8. **Write the review artifact** to `{ticket reviews directory}/`:

   Derive the review filename using the ticket stem and the next available
   review number. The ticket stem is the basename of the ticket path without
   the `.md` extension. For example, if the ticket is
   `{tickets directory}/0042-improve-search.md` and no prior reviews exist,
   the review filename is
   `{ticket reviews directory}/0042-improve-search-review-1.md`.

   To determine the next review number:
   ```bash
   mkdir -p {ticket reviews directory}
   # Glob for existing reviews of this ticket
   ls {ticket reviews directory}/{ticket-stem}-review-*.md 2>/dev/null
   # Extract the highest number, increment by 1. If none exist, use 1.
   ```

   Extract the ticket's stable 4-digit identifier from its filename using
   `${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/ticket-read-field.sh {path} number`
   (or parse the 4-digit prefix from the filename directly).

   Write the review document with YAML frontmatter followed by the review
   summary composed in Step 4.7. Include the per-lens results as a final
   section:

   ```markdown
   ---
   date: "{ISO timestamp}"
   type: ticket-review
   skill: review-ticket
   target: "{tickets directory}/{ticket-stem}.md"
   ticket_id: "{4-digit number, e.g. 0042}"
   review_number: {N}
   verdict: {APPROVE | REVISE | COMMENT}
   lenses: [{list of lenses used}]
   review_pass: 1
   status: complete
   ---

   {The full review summary from Step 4.7}

   ## Per-Lens Results

   ### {Lens 1 Name}

   **Summary**: {agent summary}

   **Strengths**:
   {agent strengths}

   **Findings**:
   {agent findings — each with severity, confidence, location, and body}

   ### {Lens 2 Name}

   ...
   ```

   The `ticket_id` field stores the ticket's stable 4-digit identifier,
   providing resilience against ticket renames. `target` remains as the path
   used at review time.

### Step 5: Present the Review

Present the composed review summary from Step 4.7 to the user.

After presenting, offer the user control before proceeding to iteration:

```
The review is complete. Verdict: [verdict]

Would you like to:
1. Proceed to address findings? (I'll help edit the ticket)
2. Change the verdict? (currently: [verdict])
3. Discuss any specific findings in more detail?
4. Re-run specific lenses with adjusted focus?
```

### Step 6: Collaborative Ticket Iteration

After presenting the review:

1. **Discuss findings with the user**:
   - Ask which recommendations they want to address
   - Clarify any findings that need more context

2. **Edit the ticket based on agreed changes**:
   - Use the Edit tool to modify the ticket file directly
   - Make targeted edits to the relevant ticket sections (Summary, Context,
     Requirements, Acceptance Criteria, etc.)
   - Do NOT modify the `status` field — that is a separate workflow decision
   - Preserve the ticket's existing frontmatter and section structure

3. **Summarise changes made**:
   ```
   I've made the following changes to the ticket:
   - [Change 1] — addressing [finding]
   - [Change 2] — addressing [finding]
   - [Skipped] — [finding discussed and decided not to address, with reason]
   ```

### Step 7: Offer Re-Review

After edits are complete:

```
The ticket has been updated. Would you like me to run another review pass to
verify the changes address the findings? This will re-run the relevant lenses
to check for any remaining issues.
```

If the user accepts:

- Re-run **only the lenses that had findings** in the previous pass
- Use the same spawn pattern and JSON extraction strategy from Steps 3-4
- Compare previous findings against new findings to determine resolution status
- Present a shorter, delta-focused review:
  ```
  ## Re-Review: [Ticket Title]

  **Verdict:** [APPROVE | REVISE | COMMENT]

  ### Previously Identified Issues
  - [emoji] **[Lens]**: [title] — Resolved / Partially resolved / Still present

  ### New Issues Introduced
  - [emoji] **[Lens]**: [title] — [brief description]

  ### Assessment
  [Whether the ticket is now ready for implementation or needs further iteration]
  ```

After composing the re-review summary, **update the review artifact**
as a single write operation:

1. Read the full content of the existing review document at
   `{ticket reviews directory}/{ticket-stem}-review-{N}.md`
2. If the existing review file's frontmatter cannot be parsed (malformed),
   warn the user and write a fresh `-review-{N+1}.md` file instead of
   appending in place
3. In memory, update exactly three frontmatter fields — `verdict`,
   `review_pass`, and `date` — preserving all other fields and body
   content verbatim
4. Append the re-review section at the end of the content
5. Write the complete modified content back to the same file in one
   operation

The document reads chronologically: initial review, per-lens results,
then re-review sections in order. The frontmatter always reflects the
latest verdict and pass count:

```markdown

## Re-Review (Pass {N}) — {date}

**Verdict:** {verdict}

### Previously Identified Issues
- {emoji} **{Lens}**: {title} — {Resolved | Partially resolved | Still present}

### New Issues Introduced
- {emoji} **{Lens}**: {title} — {brief description}

### Assessment
{Whether the ticket is now ready for implementation or needs further iteration}
```

If the user declines or the re-review shows all clear, the review is complete.

## Important Guidelines

1. **Read the ticket fully** before doing anything else

2. **Spawn agents in parallel** — the ticket lenses are independent and
   should run concurrently for efficiency

3. **Synthesise, don't concatenate** — your value is in compiling a balanced
   view across lenses, identifying themes, and prioritising actionable
   recommendations

4. **Do not modify the ticket's `status` field** — a REVISE verdict does not
   automatically change the ticket's status; that transition belongs to a
   separate workflow decision by the team

5. **Do not run codebase exploration agents** — the reviewer agents stay
   inside the ticket and any documents it explicitly references; source code
   is out of scope for ticket review

6. **Be balanced** — highlight strengths alongside concerns

7. **Prioritise by impact** — structural issues that would block implementation
   matter more than surface-level polish

8. **Handle malformed agent output gracefully** — use the `suggestion` severity
   fallback (not `major`) so a single flaky agent does not force a REVISE
   verdict

9. **Use emoji severity prefixes consistently** — 🔴 critical, 🟡 major,
   🔵 minor/suggestion, ✅ strengths. **IMPORTANT**: Use the actual Unicode
   emoji characters (🔴 🟡 🔵 ✅), NOT text shortcodes.

## What NOT to Do

- Don't skip writing the review artifact — always persist to
  `{ticket reviews directory}/` so the review is visible to the team
- Don't modify the ticket's `status` field during review
- Don't run codebase exploration agents or read source files
- Don't skip the lens selection step — always confirm with the user
- Don't present raw agent output — always aggregate and curate
- Don't make ticket edits without user agreement
- Don't post findings as individual items for positive feedback —
  strengths go in the summary only

## Relationship to Other Commands

Ticket review sits in the ticket lifecycle between authoring and implementation:

1. `/create-ticket` or `/extract-tickets` — Author or capture the ticket
2. `/list-tickets` — Discover tickets available for review
3. `/review-ticket` — Review and iterate ticket quality (this command)
4. `/update-ticket` — Apply status transitions after review decisions
5. `/create-plan` — Create an implementation plan from an approved ticket

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh review-ticket`
