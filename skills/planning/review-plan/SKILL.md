---
name: review-plan
description: Review an implementation plan through multiple quality lenses and
  collaboratively iterate based on findings. Use when the user wants to evaluate
  a plan before implementation.
argument-hint: "[path to plan file]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

# Review Plan

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh review-plan`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-review.sh plan`

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
**Plan reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_plans meta/reviews/plans`

You are tasked with reviewing an implementation plan through multiple quality
lenses and then collaboratively iterating the plan based on findings.

## Initial Response

When this command is invoked:

1. **Check if a plan path was provided**:

- If a plan path was provided, read it immediately and FULLY
- If optional focus arguments were provided (e.g., "security and architecture"),
  note them for lens selection
- Begin the review process

2. **If no plan path provided**, respond with:

```
I'll help you review an implementation plan. Please provide:
1. The path to the plan file (e.g., `{plans directory}/2025-01-08-ENG-1478-feature.md`)
2. (Optional) Focus areas to emphasise (e.g., "focus on security and architecture")

Tip: You can invoke this command with arguments:
  `/review-plan {plans directory}/2025-01-08-feature.md`
  `/review-plan {plans directory}/2025-01-08-feature.md focus on security and architecture`
```

Then wait for the user's input.

## Available Review Lenses

| Lens               | Lens Skill                    | Focus                                                                 |
|--------------------|-------------------------------|-----------------------------------------------------------------------|
| **Architecture**   | `architecture-lens`           | Modularity, coupling, scalability, evolutionary design, tradeoffs     |
| **Security**       | `security-lens`               | Threats, missing protections, STRIDE analysis, OWASP coverage         |
| **Test Coverage**  | `test-coverage-lens`          | Testing strategy, test pyramid, edge cases, isolation, risk coverage  |
| **Code Quality**   | `code-quality-lens`           | Design principles, testability, error handling, complexity management |
| **Standards**      | `standards-lens`              | Project conventions, API standards, accessibility                     |
| **Usability**      | `usability-lens`              | Developer experience, API ergonomics, configuration, onboarding       |
| **Performance**    | `performance-lens`            | Algorithmic efficiency, resource usage, concurrency, caching          |
| **Documentation**  | `documentation-lens`          | Documentation completeness, accuracy, audience fit                    |
| **Database**       | `database-lens`               | Migration safety, schema design, query correctness, integrity         |
| **Correctness**    | `correctness-lens`            | Logical validity, boundary conditions, state management, concurrency  |
| **Compatibility**  | `compatibility-lens`          | API contracts, cross-platform, protocol compliance, deps              |
| **Portability**    | `portability-lens`            | Environment independence, deployment flexibility, vendor lock         |
| **Safety**         | `safety-lens`                 | Data loss prevention, operational safety, protective mechanisms       |

## Process Steps

### Step 1: Read and Understand the Plan

1. **Read the plan file FULLY** — never use limit/offset
2. **Read any files the plan references** — tickets, related research, key
   source files mentioned
3. **Understand the plan's scope**:
  - What technologies and layers does it touch?
  - Does it involve APIs, UI, infrastructure, data models?
  - What's the complexity and risk profile?
  - Who are the consumers — other developers, services, end users?

4. **Check for existing reviews**: Glob for review documents matching
   `{plan reviews directory}/{plan-stem}-review-*.md`. If any are found:
   - Read the most recent review document (highest review number) to
     understand what was previously reviewed
   - Note the previous verdict, review pass count, and key findings
   - Inform the user: "I found {N} previous review(s) of this plan. The
     most recent (review {N}, verdict: {verdict}) will be used as context."
   - The agents do NOT receive the previous review — they review the plan
     fresh. But the aggregation step (Step 4) should reference the previous
     review when composing cross-cutting themes and the assessment:
     specifically, note which findings from the previous review recur in
     the new review and which appear to have been addressed by plan changes.
   - If the prior review file exists but cannot be parsed (e.g., malformed
     frontmatter from a partial write), warn the user and proceed as if no
     prior review exists.

   The new review creates a **new file** with the next review number (e.g.,
   `-review-2.md`). Previous review files are never modified or deleted —
   the full review history is preserved on disk.

### Step 2: Select Review Lenses

Determine which lenses are relevant based on the plan's scope and any user-
provided focus arguments.

**If the user provided focus arguments:**

- Map the focus areas to the corresponding lenses
- Include any additional lenses that are clearly relevant to the plan's scope
- Briefly explain which lenses you're running and why

**If no focus arguments were provided, auto-detect relevance:**

Take time to think carefully about which lenses apply based on:

- **Architecture** — relevant for most plans; skip only for trivial, single-file
  changes
- **Code Quality** — relevant for most plans; skip only for documentation-only
  or configuration-only changes
- **Test Coverage** — relevant for most plans; skip only for documentation-only,
  configuration-only, or infrastructure-only changes with no code
- **Security** — relevant when the plan involves: authentication/authorisation,
  user input handling, data storage, external integrations, API endpoints,
  secrets/credentials, network boundaries
- **Standards** — relevant when the plan involves: API changes, UI changes,
  new file/module creation, changes to public interfaces
- **Usability** — relevant when the plan involves: public APIs, CLI interfaces,
  configuration surfaces, breaking changes, migration paths, developer-facing
  libraries
- **Performance** — relevant when the plan involves: data processing pipelines,
  high-throughput APIs, concurrent processing resource efficiency, caching
  strategy, or algorithm-heavy logic. Skip for documentation-only,
  configuration-only, or trivial changes.
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

**Lens selection cap:** Select the most relevant lenses for the change under
review. If review configuration is provided above, use the configured
`min_lenses` and `max_lenses` values. Otherwise, use the defaults: 
**{min lenses} to {max lenses}** lenses. Apply these prioritisation rules:

Apply this lens selection pipeline in order:

1. **Start with all available lenses**: the 13 built-in lenses plus any
   custom lenses listed in the review configuration above.
2. **Remove disabled lenses**: if review configuration specifies
   `disabled_lenses`, remove those from the available set. They are never
   selected regardless of auto-detect criteria.
3. **Mark core lenses**: if review configuration specifies `core_lenses`,
   use that list. Otherwise, the core lenses are Architecture, Code Quality,
   Test Coverage, and Correctness. Core lenses are included unless the change
   is clearly outside their scope.
4. **Auto-detect remaining lenses**: use the criteria below (for built-in
   lenses) and the auto-detect criteria from review configuration (for custom
   lenses) to identify which non-core lenses are relevant to the change.
   Custom lenses that provide auto-detect criteria participate in selection
   like any other non-core lens. Custom lenses without auto-detect criteria
   (marked "always include" in the configuration) are always selected. Custom
   lenses use absolute paths instead of the `${CLAUDE_PLUGIN_ROOT}` lens
   path template.
5. **Apply focus arguments**: if the user provided focus areas, prioritise
   the corresponding lenses and fill remaining slots with auto-detected ones.
6. **Cap at `max_lenses`**: if more lenses than the configured maximum pass
   selection, rank by relevance and drop the least relevant. Prefer lenses
   whose core responsibilities directly overlap with the change's concerns.
7. **Enforce `min_lenses` floor**: never run fewer than `min_lenses` unless
   the change is trivially scoped.

When presenting the lens selection, clearly indicate which lenses are
selected and which are skipped, with a brief reason for each skip.

Present your lens selection to the user before proceeding:

```
Based on the plan's scope, I'll review through these lenses:
- Architecture: [reason]
- Security: [reason — or "Skipping: no security-sensitive changes identified"]
- Test Coverage: [reason]
- Code Quality: [reason]
- Standards: [reason — or "Skipping: ..."]
- Usability: [reason — or "Skipping: ..."]
- Performance: [reason — or "Skipping: no performance-sensitive changes identified"]
- Documentation: [reason — or "Skipping: ..."]
- Database: [reason — or "Skipping: no database changes identified"]
- Correctness: [reason]
- Compatibility: [reason — or "Skipping: ..."]
- Portability: [reason — or "Skipping: ..."]
- Safety: [reason — or "Skipping: ..."]

Shall I proceed, or would you like to adjust the selection?
```

Wait for confirmation before spawning reviewers.

### Step 3: Spawn Review Agents

For each selected lens, spawn the {reviewer agent} agent with a prompt
that includes paths to the lens skill and output format files. Do NOT read
these files yourself — the agent reads them in its own context.

Compose each agent's prompt following this template:

```
You are reviewing an implementation plan through the [lens name] lens.

## Context

The implementation plan is at [path]. Read it fully.
Also read any files the plan references for additional context.

## Analysis Strategy

1. Read your lens skill and output format files (see paths below)
2. Read the implementation plan file fully
3. Identify the scope and complexity of the proposed changes
4. Explore the codebase to understand existing patterns and context
5. Evaluate the plan through your lens, applying each key question
6. Reference specific plan sections in your findings using the `location`
   field (e.g., "Phase 2: API Endpoints", "Testing Strategy section")

## Lens

Read the lens skill at the path listed in the Lens Catalogue table in the
review configuration above. If no review configuration is present, use:
${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md

## Output Format

Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/plan-review-output-format/SKILL.md

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
   the agent's lens name and `"major"` severity, and include it in the
   review summary

When falling back, warn the user that the agent's output could not be parsed
and present the raw agent output in a collapsed form so the user can see what
the agent actually found.

### Step 4: Aggregate and Curate Findings

Once all reviews are complete:

1. **Parse agent outputs**: Extract the JSON block from each agent's response
   (see the extraction strategy in Step 3). Collect the `summary`, `strengths`,
   and `findings` arrays from each.

2. **Aggregate across agents**:
   - Combine all `findings` arrays into a single list
   - Combine all `strengths` arrays into a single list
   - Collect all `summary` strings

3. **Deduplicate findings**: Where multiple agents flag overlapping plan
   sections with similar concerns, consider merging — but only when the
   findings address the same underlying concern from different lens
   perspectives. Location proximity alone is not sufficient; the findings must
   be semantically related.

   When merging:
   - Combine the bodies, attributing each part to its lens
   - Use the highest severity among the merged findings
   - Use the highest confidence among the merged findings
   - Note all contributing lenses in the title

   When in doubt, keep findings separate — distinct findings are easier to
   address individually than a merged finding covering multiple concerns.

4. **Prioritise findings**:
   - Sort by severity: critical > major > minor > suggestion
   - Within the same severity, sort by confidence: high > medium > low

5. **Determine suggested verdict**:

   If review configuration provides verdict overrides above, apply those
   thresholds instead of the defaults below:
   - If `plan_revise_severity` is `none`, skip the severity-based REVISE
     rule (major count rule still applies independently)
   - If any findings at or above the plan revise severity ({plan revise severity})
     exist → suggest `REVISE`
   - If {plan revise major count} or more "major" findings exist
     → suggest `REVISE`
   - If fewer major findings than the threshold, or only minor/suggestion
     → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`

   Verdict meanings:
   - `APPROVE` — plan is sound and ready for implementation
   - `REVISE` — plan needs changes before implementation
   - `COMMENT` — observations only, plan is acceptable as-is

   When presenting a `COMMENT` verdict with major findings, note: "Plan is
   acceptable but could be improved — see major findings below."

6. **Identify cross-cutting themes**: Look for findings that appear across
   multiple lenses — issues flagged by 2+ agents reinforce each other and
   should be highlighted in the summary. Also identify tradeoffs where
   different lenses conflict (e.g., security wants more validation, usability
   wants less friction).

7. **Compose the review summary**:

   ```markdown
   ## Plan Review: [Plan Name]

   **Verdict:** [APPROVE | REVISE | COMMENT]

   [Combined assessment: take each agent's summary and synthesise into 2-3
   sentences covering the overall quality of the plan across all lenses]

   ### Cross-Cutting Themes
   [Issues that multiple lenses identified — these deserve the most attention]
   - **[Theme]** (flagged by: [lenses]) — [description]

   ### Tradeoff Analysis
   [Where different lenses disagree, present both perspectives]
   - **[Quality A] vs [Quality B]**: [description and recommendation]

   [Omit either section if there are no cross-cutting themes or tradeoffs]

   ### Findings

   #### Critical
   - 🔴 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   #### Major
   - 🟡 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   #### Minor
   - 🔵 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   #### Suggestions
   - 🔵 **[Lens]**: [title]
     **Location**: [plan section]
     [First 1-2 sentences of body as summary]

   ### Strengths
   - ✅ [Aggregated and deduplicated strengths from all agents]

   ### Recommended Changes
   [Ordered list of specific, actionable changes to the plan, prioritised by
   impact. Each should reference the finding(s) it addresses.]

   1. **[Change description]** (addresses: [finding titles])
      [Specific guidance on what to modify in the plan]

   ---
   *Review generated by /review-plan*
   ```

8. **Write the review artifact** to `{plan reviews directory}/`:

   Derive the review filename using the plan stem and the next available
   review number. The plan stem is the basename of the plan path without
   the `.md` extension. For example, if the plan is
   `{plans directory}/2026-03-22-improve-error-handling.md` and no prior reviews
   exist, the review filename is
   `{plan reviews directory}/2026-03-22-improve-error-handling-review-1.md`.

   To determine the next review number:
   ```bash
   mkdir -p {plan reviews directory}
   # Glob for existing reviews of this plan
   ls {plan reviews directory}/{plan-stem}-review-*.md 2>/dev/null
   # Extract the highest number, increment by 1. If none exist, use 1.
   ```

   Write the review document with YAML frontmatter followed by the review
   summary composed in Step 4.7. Include the per-lens results as a final
   section:

   ```markdown
   ---
   date: "{ISO timestamp}"
   type: plan-review
   skill: review-plan
   target: "{plans directory}/{plan-stem}.md"
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

   The per-lens results section contains the full content from each agent's
   JSON output, converted to readable markdown. This preserves the complete
   analysis for future reference while keeping it human-readable.

### Step 5: Present the Review

Present the composed review summary from Step 4.7 to the user.

After presenting, offer the user control before proceeding to iteration:

```
The review is complete. Verdict: [verdict]

Would you like to:
1. Proceed to address findings? (I'll help edit the plan)
2. Change the verdict? (currently: [verdict])
3. Discuss any specific findings in more detail?
4. Re-run specific lenses with adjusted focus?
```

### Step 6: Collaborative Plan Iteration

After presenting the review:

1. **Discuss findings with the user**:
  - Ask which recommendations they want to address
  - Discuss any tradeoffs where they need to make a judgment call
  - Clarify any findings that need more context

2. **Edit the plan based on agreed changes**:
  - Use the Edit tool to modify the plan file directly
  - Make changes incrementally — one finding at a time or in logical groups
  - Preserve the plan's existing structure and conventions
  - Don't rewrite sections unnecessarily — make targeted edits

3. **Summarise changes made**:
   ```
   I've made the following changes to the plan:
   - [Change 1] — addressing [finding]
   - [Change 2] — addressing [finding]
   - [Skipped] — [finding you discussed and decided not to address, with reason]
   ```

### Step 7: Offer Re-Review

After edits are complete:

```
The plan has been updated. Would you like me to run another review pass to
verify the changes address the findings? This will re-run the relevant lenses
to check for any remaining issues or new concerns introduced by the edits.
```

If the user accepts:

- Re-run **only the lenses that had findings** in the previous pass
- Use the same spawn pattern and JSON extraction strategy from Steps 3-4
- Compare previous findings against new findings (by `title` + `lens`) to
  determine resolution status
- Focus the review on whether findings were addressed and whether edits
  introduced new issues
- Present a shorter, delta-focused review:
  ```
  ## Re-Review: [Plan Name]

  **Verdict:** [APPROVE | REVISE | COMMENT]

  ### Previously Identified Issues
  - [emoji] **[Lens]**: [title] — Resolved / Partially resolved / Still present
  - ...

  ### New Issues Introduced
  - [emoji] **[Lens]**: [title] — [brief description]

  ### Assessment
  [Whether the plan is now in good shape or needs further iteration]
  ```

After composing the re-review summary, **update the review artifact**
as a single write operation:

1. Read the full content of the existing review document at
   `{plan reviews directory}/{plan-stem}-review-{N}.md`
2. In memory, update exactly three frontmatter fields — `verdict`,
   `review_pass`, and `date` — preserving all other fields and body
   content verbatim
3. Append the re-review section at the end of the content (after the
   Per-Lens Results section)
4. Write the complete modified content back to the same file in one
   operation

The frontmatter's `verdict` and `review_pass` fields reflect the
latest re-review state (not the initial review state), so readers can
check the current status without scrolling:

   ```markdown

   ## Re-Review (Pass {N}) — {date}

   **Verdict:** {verdict}

   ### Previously Identified Issues
   - {emoji} **{Lens}**: {title} — {Resolved | Partially resolved | Still present}
   - ...

   ### New Issues Introduced
   - {emoji} **{Lens}**: {title} — {brief description}

   ### Assessment
   {Whether the plan is now in good shape or needs further iteration}
   ```

The document reads chronologically: initial review, per-lens results,
then re-review sections in order. The frontmatter always reflects the
latest verdict and pass count.

If the user declines or the re-review shows all clear, the review is complete.

## Important Guidelines

1. **Read the plan fully** before doing anything else — you need complete
   context to select lenses and brief the agents properly

2. **Spawn agents in parallel** — the review lenses are independent and should
   run concurrently for efficiency

3. **Synthesise, don't concatenate** — your value is in compiling a balanced
   view across lenses, identifying themes and tradeoffs, and prioritising
   actionable recommendations. Don't just paste seven reports together.

4. **Be balanced** — highlight strengths alongside concerns. A plan that makes
   good architectural decisions but has security gaps should get credit for
   both.

5. **Prioritise by impact** — structural issues that are hard to fix later
   matter more than surface-level concerns. A critical finding from one lens
   outweighs minor findings from all seven.

6. **Respect tradeoffs** — when lenses conflict, present both sides and let the
   user decide. Don't privilege one quality attribute over another without
   justification.

7. **Edit conservatively** — when modifying the plan, make the minimum changes
   needed to address findings. Don't restructure or rewrite beyond what's
   required.

8. **Track what was addressed** — when presenting changes, clearly map them
   back to findings so nothing falls through the cracks.

9. **Handle malformed agent output gracefully** — if an agent doesn't return
   valid JSON, extract what you can and present the raw output to the user
   rather than silently dropping findings

10. **Keep positive feedback in the summary** — strengths go in the review
    summary, not as individual findings. Findings are exclusively for
    actionable concerns.

11. **Use emoji severity prefixes consistently** — 🔴 critical, 🟡 major,
    🔵 minor/suggestion, ✅ strengths. **IMPORTANT**: Use the actual Unicode
    emoji characters (🔴 🟡 🔵 ✅), NOT text shortcodes like `:red_circle:`,
    `:yellow_circle:`, `:blue_circle:`, or `:white_check_mark:`. Shortcodes
    are not rendered in markdown and will appear as literal text.

## What NOT to Do

- Don't skip writing the review artifact — always persist to
  {plan reviews directory}/ so the review is visible to the team
- Don't skip the lens selection step — always confirm with the user which
  lenses will run
- Don't present raw agent output — always aggregate and curate into the
  structured format
- Don't make plan edits without user agreement
- Don't force all findings to be addressed — some may be intentionally
  accepted tradeoffs
- Don't run lenses that clearly aren't relevant — a documentation plan doesn't
  need a security review
- Don't post findings as individual items for positive feedback — strengths go
  in the summary only
- Don't skip the verdict — always include a suggested verdict based on finding
  severity

## Relationship to Other Commands

The review sits in the plan lifecycle between creation and implementation:

1. `/create-plan` — Create the implementation plan interactively
2. `/review-plan` — Review and iterate the plan quality (this command)
3. `/implement-plan` — Execute the approved plan
4. `/validate-plan` — Verify implementation matches the plan

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh review-plan`
