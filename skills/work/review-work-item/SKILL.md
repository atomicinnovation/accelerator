---
name: review-work-item
description: Review a work item through multiple quality lenses and
  collaboratively iterate based on findings. Use when the user wants to
  evaluate a work item before implementation or escalation.
argument-hint: "[path to work item file]"
allowed-tools:
   - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
   - Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---

# Review Work Item

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh review-work-item`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-review.sh work-item`

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work`
**Work item reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_work`

## Work Item Review Template

The template below defines the frontmatter and body structure that every
work item review must carry. Read it now — use it to guide what information
you record in Steps 3-4 and what shape you persist in Step 4.8.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh work-item-review`

You are tasked with reviewing a work item through quality lenses and then
collaboratively iterating the work item based on findings.

## Initial Response

When this command is invoked:

1. **Check if a work item path or ID was provided**: invoke the
   resolver:

   ```
   ${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-resolve-id.sh <argument>
   ```

   The resolver respects `work.id_pattern` and accepts paths, full IDs
   (`PROJ-0042`), and bare numbers.

   - **Exit 0**: stdout is the absolute path. Read the work item
     immediately and FULLY, then begin the review process.
   - **Exit 1**: unrecognised input. Print the resolver's error and
     offer to run `/list-work-items`.
   - **Exit 2**: ambiguous match. The resolver lists candidates with
     source-category tags. Ask the user to disambiguate by re-running
     with a full ID or path.
   - **Exit 3**: no match. Print the resolver's error and offer to run
     `/list-work-items`.
   - If optional focus arguments were provided (e.g., "focus on testability"),
     note them for lens selection.

2. **If no work item path or number provided**, respond with:

```
I'll help you review a work item. Please provide:
1. The path to the work item file (e.g., `{work_dir}/0042-my-work-item.md`)
2. (Optional) A work item number shorthand (e.g., `/review-work-item 42`)
3. (Optional) Focus areas to emphasise (e.g., "focus on testability")

Tip: Use `/list-work-items` to find the work item you want to review.
```

Then wait for the user's input.

## Available Review Lenses

| Lens             | Lens Skill          | Focus                                                                   |
|------------------|---------------------|-------------------------------------------------------------------------|
| **Clarity**      | `clarity-lens`      | Unambiguous referents, internal consistency, jargon handling            |
| **Completeness** | `completeness-lens` | Section presence, content density, kind-appropriate content             |
| **Dependency**   | `dependency-lens`   | Implied couplings not captured — blockers, consumers, external systems  |
| **Scope**        | `scope-lens`        | Right-sized, single coherent unit of work; decomposition; orthogonality |
| **Testability**  | `testability-lens`  | Measurable criteria, verifiable outcomes, verification framing          |

> Note: completeness flags an *absent* Dependencies section; dependency flags
> an *empty or underspecified* section whose contents fail to name every
> coupling the work item implies.

## Process Steps

### Step 1: Read and Understand the Work Item

1. **Read the work item file FULLY** — never use limit/offset
2. **Parse the frontmatter** to note `kind` (bug, story, spike, epic, etc.)
   and `status`
3. **Read any documents referenced in the References section** — these provide
   context the lenses may need; do not read source code
4. **Check for existing reviews**: Glob for review documents matching
   `{work_reviews_dir}/{work-item-stem}-review-*.md`. If any are found:
   - Read the most recent review document (highest review number)
   - Note the previous verdict, review pass count, and key findings
   - Inform the user: "I found {N} previous review(s) of this work item. The
     most recent (review {N}, verdict: {verdict}) will be used as context."
   - The agents do NOT receive the previous review — they review the work item
     fresh. But the aggregation step (Step 4) should reference the previous
     review when composing cross-cutting themes and the assessment.
   - If the prior review file exists but cannot be parsed (malformed
     frontmatter), warn the user and proceed as if no prior review exists.

   The new review creates a **new file** with the next review number (e.g.,
   `-review-2.md`). Previous review files are never modified.

### Step 2: Select Review Lenses

By default, run every lens registered in `BUILTIN_WORK_ITEM_LENSES` unless the
user has provided focus arguments or config restricts the selection. The five
work item lenses cover orthogonal concerns, so there is no relevance-based
auto-selection.

**If the user provided focus arguments:**

- Map the focus areas to the corresponding lenses
- Include any additional lenses that are clearly relevant
- Briefly explain which lenses you're running

**If no focus arguments were provided:**

Run all built-in work item lenses unless:
- A lens is listed in `disabled_lenses` — remove it from the active set
- The user's configured `core_lenses` has filtered this to a subset (see below)

When `core_lenses` is set in config, apply it as the *minimum required set*;
add any remaining non-disabled lenses up to `max_lenses`. This means users
who previously pinned `core_lenses` to the Phase 4 work item lenses
(`completeness`, `testability`, `clarity`) will also receive `scope` and
`dependency` on upgrade, unless they add those names to `disabled_lenses` or
set `max_lenses` to their subset size.

Present the selection briefly — enumerate the chosen lenses with a one-line
focus each — then wait for confirmation before spawning reviewers. The
confirmation gate is preserved even though the default always selects every
lens; the gate is useful when focus args or config have narrowed the set.

Example (default path, no focus args, no `core_lenses` restriction):

```
I'll review this work item through all work item lenses (clarity, completeness,
dependency, scope, testability). Shall I proceed?
```

Wait for confirmation before spawning reviewers.

### Step 3: Spawn Review Agents

For each selected lens, spawn the {reviewer agent} agent with a prompt that
includes the paths to the lens skill and output format files. Do NOT read these
files yourself — the agent reads them in its own context.

Compose each agent's prompt following this template:

```
You are reviewing a work item through the [lens name] lens.

## Context

The work item is at [path]. Read it fully.
Also read any source documents listed in the work item's References section.

## Analysis Strategy

1. Read your lens skill and output format files (see paths below)
2. Read the work item file fully
3. Read referenced documents from the work item's References section if present
4. Evaluate the work item through your lens, applying each key question
5. Reference specific work item sections in your findings using the `location`
   field (e.g., "Acceptance Criteria", "Requirements", "Frontmatter: kind")

IMPORTANT: Do not evaluate the codebase — work item content (and any documents
it explicitly references) is the sole artefact under review. Do not run
codebase exploration agents or read source files unless the work item's
References section explicitly links to them.

## Lens

Read the lens skill at the path listed in the Lens Catalogue table in the
review configuration above. If no review configuration is present, use:
${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md

## Output Format

Read the output format at:
${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/work-item-review-output-format/SKILL.md

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
   REVISE verdict when `work_item_revise_severity` is `major` or higher.

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
   - If `work_item_revise_severity` is `none`, skip the severity-based REVISE
     rule (major count rule still applies independently)
   - If any findings at or above the work item revise severity
     ({work item revise severity}) exist → suggest `REVISE`
   - If {work item revise major count} or more `"major"` findings exist
     → suggest `REVISE`
   - If fewer major findings than the threshold, or only minor/suggestion
     → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`

   Verdict meanings:
   - `APPROVE` — work item is ready for implementation
   - `REVISE` — work item needs changes before implementation
   - `COMMENT` — observations only, work item is acceptable as-is

   When presenting a `COMMENT` verdict with major findings, note: "Work item is
   acceptable but could be improved — see major findings below."

6. **Identify cross-cutting themes**: Look for findings that appear across
   multiple lenses — issues flagged by 2+ agents reinforce each other and
   should be highlighted in the summary.

7. **Compose the review summary**:

   ```markdown
   ## Work Item Review: [Work item Title]

   **Verdict:** [APPROVE | REVISE | COMMENT]

   [Combined assessment: synthesise each agent's summary into 2-3 sentences
   covering the overall quality of the work item across all lenses]

   ### Cross-Cutting Themes
   [Issues that multiple lenses identified — these deserve the most attention]
   - **[Theme]** (flagged by: [lenses]) — [description]

   ### Findings

   #### Critical
   - 🔴 **[Lens]**: [title]
     **Location**: [work item section]
     [First 1-2 sentences of body as summary]

   #### Major
   - 🟡 **[Lens]**: [title]
     **Location**: [work item section]
     [First 1-2 sentences of body as summary]

   #### Minor
   - 🔵 **[Lens]**: [title]
     **Location**: [work item section]
     [First 1-2 sentences of body as summary]

   #### Suggestions
   - 🔵 **[Lens]**: [title]
     **Location**: [work item section]
     [First 1-2 sentences of body as summary]

   ### Strengths
   - ✅ [Aggregated and deduplicated strengths from all agents]

   ### Recommended Changes
   [Ordered list of specific, actionable changes to the work item, prioritised by
   impact. Each should reference the finding(s) it addresses.]

   1. **[Change description]** (addresses: [finding titles])
      [Specific guidance on what to modify in the work item]

   ---
   *Review generated by /review-work-item*
   ```

8. **Write the review artifact** to `{work_reviews_dir}/`:

   Derive the review filename using the work item stem and the next available
   review number. The work item stem is the basename of the work item path without
   the `.md` extension. For example, if the work item is
   `{work_dir}/0042-improve-search.md` and no prior reviews exist,
   the review filename is
   `{work_reviews_dir}/0042-improve-search-review-1.md`.

   To determine the next review number:
   ```bash
   mkdir -p {work_reviews_dir}
   # Glob for existing reviews of this work item
   ls {work_reviews_dir}/{work-item-stem}-review-*.md 2>/dev/null
   # Extract the highest number, increment by 1. If none exist, use 1.
   ```

   Extract the work item's stable 4-digit identifier from its filename using
   `${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh id {path}`
   (or parse the 4-digit prefix from the filename directly).

   Before writing the work item review file, capture metadata and substitute
   the unified base fields and per-type extras into the template's
   frontmatter block:

   1. Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
      to obtain `Current Date/Time (UTC):`.
   2. **Substitute** every field below with the indicated value:
      - `type:` ← `work-item-review`
      - `id:` ← the review filename stem (without `.md`), always quoted
        as a YAML string
      - `title:` ← `Work Item Review: {work item title}`
      - `date:` ← the `Current Date/Time (UTC):` value
      - `author:` ← the author value resolved per `create-work-item/SKILL.md:578-580`
      - `producer:` ← `review-work-item`
      - `status:` ← `complete`
      - `last_updated:` ← the same `Current Date/Time (UTC):` value
      - `last_updated_by:` ← the same value resolved for `author`
      - `schema_version:` ← `1` (bare integer, not quoted)
      - `target:` ← `"work-item:<4-digit-id>"` (e.g. `"work-item:0042"`),
        typed-linkage key per ADR-0034, always emitted as a single
        quoted YAML string in `"doc-type:id"` form
      - `work_item_id:` ← the same 4-digit identifier as the `target`
        payload's id portion (transitional alias — see Migration Notes;
        the visualiser's `read_ref_keys` consumes this scalar today)
      - `reviewer:` ← the reviewer value resolved per `create-work-item/SKILL.md:578-580`
      - `verdict:` ← the verdict from Step 4.5 (`APPROVE | REVISE | COMMENT`)
      - `lenses:` ← the list of work-item lens names used
      - `review_number:` ← `N` (the next available review number from the
        glob above)
      - `review_pass:` ← `1` (initial-write pass count; re-reviews bump
        per the Step 7 flow)
   3. Write the file with the substituted frontmatter block, followed by
      the review summary composed in Step 4.7 and the per-lens results as
      a final section:

   ```markdown
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

   The `target:` field stores the work item's stable 4-digit identifier
   as a typed-linkage key (e.g. `"work-item:0042"`) per ADR-0034,
   providing resilience against work item renames. A `work_item_id:`
   field is also emitted as a transitional alias carrying the same
   4-digit identifier — the visualiser's `read_ref_keys` consumes it
   as the primary work-item cross-reference key today. Both fields
   encode the same edge; the duplication is bounded by the visualiser
   consumer update.

### Step 5: Present the Review

Present the composed review summary from Step 4.7 to the user.

After presenting, offer the user control before proceeding to iteration:

```
The review is complete. Verdict: [verdict]

Would you like to:
1. Proceed to address findings? (I'll help edit the work item)
2. Change the verdict? (currently: [verdict])
3. Discuss any specific findings in more detail?
4. Re-run specific lenses with adjusted focus?
```

### Step 6: Collaborative Work Item Iteration

After presenting the review:

1. **Discuss findings with the user**:
   - Ask which recommendations they want to address
   - Clarify any findings that need more context

2. **Edit the work item based on agreed changes**:
   - Use the Edit tool to modify the work item file directly
   - Make targeted edits to the relevant work item sections (Summary, Context,
     Requirements, Acceptance Criteria, etc.)
   - Do NOT modify the `status` field — that is a separate workflow decision
   - Preserve the work item's existing frontmatter and section structure

3. **Summarise changes made**:
   ```
   I've made the following changes to the work item:
   - [Change 1] — addressing [finding]
   - [Change 2] — addressing [finding]
   - [Skipped] — [finding discussed and decided not to address, with reason]
   ```

### Step 7: Offer Re-Review

After edits are complete:

```
The work item has been updated. Would you like me to run another review pass to
verify the changes address the findings? This will re-run the relevant lenses
to check for any remaining issues.
```

If the user accepts:

- Re-run **only the lenses that had findings** in the previous pass
- Use the same spawn pattern and JSON extraction strategy from Steps 3-4
- Compare previous findings against new findings to determine resolution status
- Present a shorter, delta-focused review:
  ```
  ## Re-Review: [Work item Title]

  **Verdict:** [APPROVE | REVISE | COMMENT]

  ### Previously Identified Issues
  - [emoji] **[Lens]**: [title] — Resolved / Partially resolved / Still present

  ### New Issues Introduced
  - [emoji] **[Lens]**: [title] — [brief description]

  ### Assessment
  [Whether the work item is now ready for implementation or needs further iteration]
  ```

After composing the re-review summary, **update the review artifact**
as a single write operation:

1. Read the full content of the existing review document at
   `{work_reviews_dir}/{work-item-stem}-review-{N}.md`
2. If the existing review file's frontmatter cannot be parsed (malformed
   YAML or missing `---` delimiters), warn the user and write a fresh
   `-review-{N+1}.md` file instead of appending in place
3. In memory, update exactly four frontmatter fields — `verdict`,
   `review_pass`, `last_updated`, and `last_updated_by` — preserving
   all other fields and body content verbatim. The `date` field retains
   the original-review timestamp; only `last_updated` advances on
   re-review. (`last_updated_by` may match `reviewer` if the
   re-reviewer is the same person, but is computed independently.)

   **Pre-0066-artifact handling**: when the re-reviewed artifact lacks
   `last_updated:` and/or `last_updated_by:` (it was written pre-0066),
   insert those fields rather than treating their absence as
   malformed-frontmatter. Only an unparseable YAML block or missing
   `---` delimiters triggers the fresh-`-review-{N+1}.md` fallback.
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
{Whether the work item is now ready for implementation or needs further iteration}
```

If the user declines or the re-review shows all clear, the review is complete.

## Important Guidelines

1. **Read the work item fully** before doing anything else

2. **Spawn agents in parallel** — the work item lenses are independent and
   should run concurrently for efficiency

3. **Synthesise, don't concatenate** — your value is in compiling a balanced
   view across lenses, identifying themes, and prioritising actionable
   recommendations

4. **Do not modify the work item's `status` field** — a REVISE verdict does not
   automatically change the work item's status; that transition belongs to a
   separate workflow decision by the team

5. **Do not run codebase exploration agents** — the reviewer agents stay
   inside the work item and any documents it explicitly references; source code
   is out of scope for work item review

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
  `{work_reviews_dir}/` so the review is visible to the team
- Don't modify the work item's `status` field during review
- Don't run codebase exploration agents or read source files
- Don't skip the lens selection step — always confirm with the user
- Don't present raw agent output — always aggregate and curate
- Don't make work item edits without user agreement
- Don't post findings as individual items for positive feedback —
  strengths go in the summary only

## Relationship to Other Commands

Work item review sits in the work item lifecycle between authoring and implementation:

1. `/create-work-item` or `/extract-work-items` — Author or capture the work item
2. `/list-work-items` — Discover work items available for review
3. `/review-work-item` — Review and iterate work item quality (this command)
4. `/update-work-item` — Apply status transitions after review decisions
5. `/create-plan` — Create an implementation plan from an approved work item

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh review-work-item`
