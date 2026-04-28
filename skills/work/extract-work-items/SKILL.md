---
name: extract-work-items
description: Extract work items in batch from existing documents (specs, PRDs,
  research, plans, meeting notes, design docs). Use whenever the user wants
  to capture, pull out, or convert requirements, work items, user stories,
  bug reports, or actionable tasks from existing files into structured
  work items in meta/work/ — even if they don't say "extract" explicitly.
argument-hint: "[document paths...] or leave empty to scan all"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---

# Extract Work Items from Meta Documents

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh extract-work-items`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`

## Work Item Template

The template below defines the sections and frontmatter fields that every
work item must contain. Read it now — the valid work item types live in the `type`
field (not a hardcoded list elsewhere in this skill), and every written file
must populate every frontmatter field.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh work-item`

You are tasked with identifying requirements, work items, and actionable
tasks within existing meta documents and helping the user capture them as
formal work items. Source documents typically tell you *what* work items should
exist but rarely give the full business context, testable acceptance
criteria, dependencies, and assumptions a good work item needs. The model,
the user, and web research fill those gaps.

Extraction therefore proceeds in two layers:

- **Source-derived content stays faithful.** Anything you draw from the
  source documents must reflect what they actually say — do not silently
  invent requirements that are not there.
- **Business-context gaps are surfaced.** The `Assumptions`, `Open Questions`
  and `Drafting Notes` sections serve different purposes when present, and all 
  matter:
  - **Assumptions** are interpretations you made that affect the work item's
    meaning. Flag one only when using the wrong interpretation would lead
    someone to build something different. Example: *"Interpreted 'users' as
    end users rather than internal staff — if wrong, scope changes."*
  - **Open Questions** are genuine unknowns the source raises but leaves
    unanswered that a reader or implementer needs resolved before work can
    proceed. Example: *"What does 'better results' mean — improved relevance, 
    faster delivery, or both?"*
  - **Drafting Notes** capture interpretations you made while filling out
    the work item — business-context calls, scope decisions, or technical
    choices that someone should review if they turn out to be wrong.
    Actively populate this section. If you inferred who the stakeholders
    are, what a vague term means, what the scope boundary is, or which
    technical approach the source implies, write it down.
    Examples: *"Treated this as a spike because no acceptance criteria are 
    defined — if implementation is already expected, type and scope both 
    change."* Routine field selections (type, priority, tags) don't need an 
    entry unless the choice reflects a substantive scope or meaning 
    interpretation that a reviewer should be aware of.

For each selected candidate, you offer the user the choice between
*enriching* the work item interactively (with model knowledge, web research,
and a few focused questions, similar to `/create-work-item`) or *accepting
the source-derived skeleton as-is* and refining later. The enrichment
loop is per candidate, not pre-generated, so the work the model invests
matches the depth the user wants for each work item.

## Initial Setup

When this command is invoked:

1. **Check if parameters were provided**:

- If one or more file paths were provided, note them as the target documents
- If no parameters provided, ask conversationally and helpfully which
  documents to work with. Give the user enough context to respond easily:
  briefly explain what kinds of sources work (specific files, planning docs,
  meeting notes, research, specs), mention that you can also scan all
  documents in the configured directories, and invite them to direct.
  Helpful examples matter more here than brevity — aim for something
  welcoming and informative rather than a single terse question.

Wait for user input.

## Process Steps

### Step 1: Identify Source Documents

1. If specific files were provided, read them FULLY. Before reading,
   verify each path exists. If any path does not exist, report which
   paths are missing and ask the user to correct or remove them — do not
   silently skip them or proceed with an empty set.
2. If scanning all meta documents:
   - Spawn a **{documents locator agent}** agent to find all documents in the
     configured research and plans directories (shown above)
   - Present the discovered documents and let the user select which to scan:
     ```
     I found the following documents:

     **Research:**
     - `{research directory}/2026-04-08-topic.md` — Topic research
     - ...

     **Plans:**
     - `{plans directory}/2026-04-19-feature.md` — Feature plan
     - ...

     Which documents should I scan for work items? (enter numbers, "all", or
     specific paths)
     ```
   - Wait for user selection.

### Step 2: Analyse Documents for Work Items

1. **Spawn {documents analyser agent} agents** (one per document, in parallel)
   with instructions to identify requirements and actionable work items.
   Look for:
   - Explicit requirements ("The system must...", "Users need to...",
     "We need to implement...")
   - User stories ("As a..., I want..., so that...")
   - Feature descriptions and acceptance criteria
   - Bug reports with symptoms and expected behaviour
   - Open-ended investigations or unknowns requiring research
   - Multi-deliverable themes that span several stories
   - One-off tasks (migrations, infrastructure work, documentation)

2. **Wait for all agents to complete.**

3. **Deduplicate**: Where the same work item appears across multiple
   documents, merge the entries and record all source documents it came from.

4. **Present discovered candidates** as a numbered list:

```
I found the following actionable items across the scanned documents:

1. **[Short title]** — [one-line description]
   Source: `{research directory}/2026-04-08-topic.md`

2. **[Short title]** — [one-line description]
   Source: `{plans directory}/2026-04-19-feature.md`, `{research directory}/2026-04-08-topic.md`

3. ...

Which items would you like to create work items for? (enter numbers, "all",
or "none")
```

If no actionable items are found across all documents, inform the user:
"No actionable items found in the provided documents." and exit cleanly.

Wait for user selection. If the user selects "none", exit cleanly without
writing any files.

### Step 3: Enrich and Approve (Per Candidate)

For each selected candidate, in original presented order, build a
source-derived skeleton, present it, and let the user choose how much
enrichment to invest. Do NOT pre-generate drafts for the entire batch
in advance — enrichment can change a draft significantly, so generation
happens per candidate inside this loop.

#### 3.1 Build the source-derived skeleton

For the current candidate:

- Infer the work item type from its content using types read from the
  work item template's `type` field:
  - clear bug reports with symptoms and expected/actual behaviour → `bug`
  - open-ended investigations with specific questions → `spike`
  - broad multi-deliverable themes → `epic`
  - specific single deliverables → `story`
  - one-off technical or operational tasks → `task`
  - Default to `story` for items where the type is genuinely ambiguous.
- Draft a complete work item from the source content alone, using `XXXX`
  as the placeholder work item number.
- Type-specific content placement:
  - bug: reproduction steps, expected/actual behaviour → `Requirements` section
  - spike: research questions, time-box, exit criteria → `Requirements` section
  - epic: initial story decomposition → `Requirements` section as a list
  - Do not rename or add sections beyond those in the work item template.
- Surface business-context gaps using the right section: put your
  interpretation in `Assumptions` (when you made a call and the wrong call
  changes what gets built). Put unanswered questions in `Open Questions` when 
  you genuinely cannot tell from the source. Populate `Open Questions` with 
  anything that would materially change scope, approach, or acceptance 
  criteria if resolved differently. Add a `Drafting Notes` entry for every 
  meaningful interpretation you made (scope boundaries, who
  stakeholders are, what vague terms mean, which technical approach is implied).
- Include all source documents for this item in the `References` section.

#### 3.2 Present the skeleton with options

```
Candidate #N of M: [title]
Type (proposed): [type]
Source: [paths]

[work item content with XXXX placeholder, including Drafting Notes section]

How would you like to proceed?
  1. enrich              — interactive Q&A, web research where useful, then approve
  2. accept as-is        — write this skeleton as a thin draft for later refinement
  3. skip                — exclude from this batch
  4. accept remaining as-is — fast-path every remaining candidate as a thin draft
```

Wait for the user's choice. The four options behave as follows.

If the user types something other than the four options — for example
`revise <instructions>` or free-form revision text — treat it as
`enrich` seeded with those instructions: enter the enrichment loop in
3.3 and use the supplied text as the user's first round of revision
guidance, skipping the question phase if the instructions are already
substantive.

#### 3.3 enrich — interactive enrichment

Treat this as a focused, per-candidate version of the `/create-work-item`
flow, seeded with the skeleton above:

1. **Ask 1–3 focused business-context questions** tailored to this
   candidate. Fewer than `/create-work-item`'s 3–5 because the source
   already provides some context. Cover whichever of the following are
   not already clear from the source:
   - What pain point or problem does this address, and who experiences it?
   - What is the desired outcome — what changes for people once this is done?
   - Are there constraints, deadlines, or dependencies worth knowing?
   - For bugs: what is the impact, and is it a blocker?
   - Anything you are uncertain about, or that should be researched?

2. **Spawn `{web-search-researcher agent}`** when there is uncertainty
   about any aspect of this candidate the model lacks confidence on —
   business rules, domain concepts, competitive landscape, industry
   standards, external technology. Skip only when the candidate is
   self-contained and well-understood from the source plus user answers.
   When in doubt, prefer to spawn research — over-asking is cheaper
   than producing a vague enriched draft.

3. **Update the draft** combining source content, user answers, model
   knowledge, and research findings. Re-present it as a structured
   proposal that:
   - Confirms or revises the type
   - Lists requirements drawn from source + enrichment
   - Proposes specific, testable acceptance criteria — prefer
     Given/When/Then for story/task; draw on domain knowledge and
     research to make them thorough
   - Lists dependencies (blocking and blocked) where known
   - Keeps a `Drafting Notes` section for any interpretations still
     unresolved so the user can challenge them
   - Lists remaining open questions in the `Open Questions` section

   Then offer next steps with numbers:

   ```
   How would you like to proceed?
     1. approve    — accept this draft and move on to the next candidate
     2. revise <instructions> — provide revision instructions and I'll update the draft
     3. skip       — discard this candidate entirely
     4. accept as-is — discard enrichment and use the original source-only skeleton
   ```

4. **Iterate on the same candidate.** Challenge weak or untestable
   acceptance criteria — when a criterion is not measurable, ask what a
   passing test would look like and reformulate together. Do not accept
   vague criteria into the draft. The candidate position (e.g. "Candidate
   #N of M") does NOT advance during iteration: re-present the updated
   draft for the same candidate and accept further free-form revision
   instructions, looping as many times as needed. Only `approve` (or one
   of the interrupts below) advances to candidate N+1. Once the user
   explicitly approves, mark the candidate **approved (enriched)** and
   advance to the next candidate.

The user may switch out of enrichment at any point by saying:

- `skip` — exclude this candidate entirely. Discard any partial
  enrichment work — questions answered, drafts re-proposed — and never
  approve it later, even if `accept remaining as-is` is invoked on a
  subsequent candidate.
- `accept as-is` — replace the in-flight enriched draft with the
  original 3.1 source-derived skeleton (NOT the partially enriched
  state) and apply the 3.4 thin-draft assumptions note. The user is
  saying "stop enriching this one; take the source-only version".

Honour these interrupts immediately on the turn they are received.

#### 3.4 accept as-is — thin draft

Take the source-derived skeleton from 3.1 as the final draft for this
candidate. Append (or extend) the `Drafting Notes` section with the
following note **verbatim** (so future tooling like `/refine-work-item`
and human reviewers can detect thin drafts deterministically):

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and type may need refinement before
> promoting from `draft` to `ready`.

If the candidate already has source-derived drafting notes, keep them
and add the verbatim note as a separate paragraph beneath.
Keep any `Open Questions` from the 3.1 skeleton — these are genuine
business unknowns that need resolution regardless of whether enrichment
happened. Mark the candidate **approved (thin)** and advance to the next.

#### 3.5 skip

Exclude this candidate from the batch and advance to the next. Skipped
candidates never become approved later, even if a subsequent
`accept remaining as-is` is used.

#### 3.6 accept remaining as-is — fast-path

Mark every remaining *unreviewed* candidate as **approved (thin)**,
applying the same skeleton + assumptions note as 3.4. Do not ask
further questions. Already-skipped candidates stay skipped. Jump to
Step 4.

#### 3.7 No `work-item-next-number.sh` calls in Step 3

`work-item-next-number.sh` is not called at any point during Step 3,
regardless of which option the user picks. Writing happens exclusively
in Step 4 after all approvals — enriched and thin — are collected.

### Step 4: Write Work Items

1. **Count approved (non-skipped) items: N.**

2. **If N is 0**: print "No work items approved — nothing written." and exit
   cleanly. Do NOT call `work-item-next-number.sh`.

3. **Otherwise**:

   a. **Compute target slugs** — for each approved draft, derive a meaningful
      kebab-case slug from its title.

   b. **Read configuration**:
      ```
      PATTERN=$(${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh work.id_pattern "{number:04d}")
      DEFAULT_PROJECT=$(${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh work.default_project_code "")
      ```

   c. **Suggest projected IDs**: if `PATTERN` contains `{project}`, the
      default project for each row is `DEFAULT_PROJECT` (warn and require
      user amendment if `DEFAULT_PROJECT` is empty). If `PATTERN` lacks
      `{project}`, no project column is shown.

      Compute *display-only* projected IDs by calling, per distinct
      project code:
      ```
      ${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-next-number.sh --project <code> --count <count-for-that-project>
      ```
      These calls do not commit numbers; the same call is re-issued after
      every amendment to keep the table accurate.

   d. **Present an amendment table**:

      ```
      | # | Slug       | Project | Projected ID |
      | 1 | add-foo    | PROJ    | PROJ-0001    |
      | 2 | fix-bar    | PROJ    | PROJ-0002    |
      | 3 | update-baz | PROJ    | PROJ-0003    |

      Amend any rows? (`<rows> <PROJECT>` to set, `<rows> -` to revert to
      default, `?` for help, `q` to cancel, blank to confirm.)
      ```

      When `PATTERN` lacks `{project}`, omit the `Project` column and
      render only `| # | Slug | Projected ID |`. The amendment prompt is
      not shown in that case — proceed directly to confirmation.

      **Amendment grammar** (canonical — same wording in every state):

      - `<rows>`: one row number (`2`) or comma-separated list (`2,3,7`).
        Whitespace around commas is permitted (`2, 3, 7`) and trimmed.
      - `<PROJECT>`: a project code matching `[A-Za-z][A-Za-z0-9]*`.
      - `<rows> -`: revert the named rows to the default project code
        (or to "no project" when no default is set).
      - `?`: re-display the amendment grammar reference plus the
        unchanged table; no state change.
      - `q`: cancel the entire flow with no files written and no numbers
        allocated.
      - Blank input: confirms the current table state.

      **Validation**: out-of-range row numbers re-prompt with
      `error: row N — out of range (valid: 1-M)` without applying any
      other amendments in the same input. Invalid project codes
      re-prompt with `error: row N — project value "<value>" must
      match [A-Za-z][A-Za-z0-9]*` and discard the entire input
      (no partial application). Unrecognised commands re-prompt with
      `error: unrecognised input. Type ? for help.` On any rejection
      the table reverts to its last valid state.

      After every accepted amendment, recompute projected IDs by
      re-issuing the per-project allocator calls (display only).

   e. **Project-aware slug-collision check** before any allocation. For
      each row, glob:
      - When the pattern has `{project}`:
        `{work_dir}/<project>-*-<slug>.md` for the row's project — a
        same-slug file under the *same* project is a real collision.
      - Always (legacy fallback):
        `{work_dir}/[0-9][0-9][0-9][0-9]-<slug>.md` — a same-slug legacy
        file shadows the new file regardless of project.

      Within the same batch, two amendments to the same project with
      the same slug are also a collision. Same slug under different
      projects (`PROJ-0001-add-foo.md` and `OTHER-0001-add-foo.md`) is
      legitimate and not a collision.

      If any collision is detected, report which slugs collide and
      which existing files they match, abort without calling the
      allocator, and ask the user to resolve the collision before
      re-running.

   f. **Allocate per distinct project code**, in original presentation
      order:
      ```
      ${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-next-number.sh --project <code> --count <count>
      ```
      One call per distinct project code; `--project` is omitted when
      the pattern lacks `{project}`. If any allocator call exits
      non-zero, abort immediately and surface the error verbatim — do
      not write any files. The whole batch fails atomically.

   g. **Substitute the allocated full IDs** into approved drafts in
      their original presented order. Within a single project, the
      first row in presentation order takes the first allocated
      number; multiple projects each preserve their own ordering. The
      `work_item_id` frontmatter is **always quoted** (`"PROJ-0001"`).

   h. **Write all N work item files**. Each work item's `References`
      section must include all source document paths the item was
      extracted from. For deduplicated items that appeared in multiple
      documents, list every contributing source under `References`,
      one per line.

   i. If a write error occurs mid-batch: report which numbers were
      allocated, which files were written successfully, and which were
      not — so the user can manually write the missing files with
      their pre-assigned IDs. Do not retry writes silently and do not
      call the allocator again to re-allocate; the original allocation
      stands. The user needs to know the exact state.

4. **Print a summary table**:

```
Created the following work items:
| ID         | Title   | File                                 |
|------------|---------|--------------------------------------|
| PROJ-0001  | [title] | `{work_dir}/PROJ-0001-slug.md`       |
| OTHER-0001 | [title] | `{work_dir}/OTHER-0001-slug.md`      |
...
```

Under the default `{number:04d}` pattern the ID column shows
`0001`, `0002`, etc., and no project amendment table appears.

## Quality Guidelines

- Never call `work-item-next-number.sh` before all approvals are collected.
  The number space is shared and finite; consuming numbers for drafts the
  user might still skip creates gaps that are impossible to clean up later.
- Never call `work-item-next-number.sh` when N=0. An all-skipped session must
  exit cleanly with no side effects.
- If `work-item-next-number.sh` exits non-zero, abort immediately and surface
  the script's error output verbatim — even if it emitted some numbers on
  stdout before failing, treat the entire batch as failed.
- Verify all target slugs are free BEFORE calling `work-item-next-number.sh` —
  collision checks happen before number allocation, by slug pattern, since
  numbers are not yet known. Under a `{project}` pattern the collision
  check is **project-aware**: the same slug under two different project
  codes (`PROJ-0001-add-foo.md` and `OTHER-0001-add-foo.md`) is
  legitimate. Same-slug legacy `NNNN-{slug}.md` files always count as a
  collision.
- Under a `{project}` pattern, the amendment table prompts the user
  to assign or override project codes per row before allocation. The
  display-only projected IDs are recomputed after every amendment.
  No numbers are committed until the user confirms with blank input.
- Numbers are assigned to approved drafts in their original presented
  order, not in approval timestamp order. This makes outputs deterministic
  and matches the order the user reviewed.
- Every written work item MUST include all source document paths in its
  `References` section. For deduplicated items that appeared in multiple
  documents, list every contributing source.
- Do not extract structural or navigational content (table of contents
  entries, section headings with no requirements content, agenda items
  with no actionable outcome) as candidate work items. If a heading just
  organises content rather than describing work, skip it.
- Work item type inference must use types read from the work item template
  frontmatter (loaded at the top of this skill), not a hardcoded list.
  Default to `story` for items where the type is genuinely ambiguous.
- All frontmatter fields defined in the work item template must be populated
  in every written work item — `work_item_id` matching the assigned NNNN, `title`
  matching the work item's title, `date`, `author`, `type`, `status` (draft),
  `priority` (medium unless the source implies otherwise), `parent` (empty
  string unless the source establishes a parent), and `tags` (a YAML
  array, possibly empty). No field may contain unfilled placeholder text
  like `[author]` or `NNNN`. The body H1 format is `# NNNN: <title>` —
  kept in sync with the frontmatter `title:` field.
- `date` must use the work item template's `YYYY-MM-DDTHH:MM:SS+00:00`
  format in UTC (e.g. obtained via `date -u +%Y-%m-%dT%H:%M:%S+00:00`).
- `author` is sourced in this order: configuration if present, then the
  source document's author/owner field if named, then the current git/jj
  user identity, then — only if all of those fail — ask the user once
  before writing the batch. Never write `[author]` or any placeholder.
- "Accept remaining as-is" only marks unreviewed candidates as approved
  (thin) — it does not resurrect skipped candidates, and writing still
  happens exclusively in Step 4 after the single `work-item-next-number.sh`
  call.
- Source-derived content stays faithful to what the source documents say.
  Do not silently invent requirements. When you make an interpretation while
  filling out the work item — about scope, stakeholders, terminology, or implied
  approach — capture it in `Drafting Notes`. Use `Open Questions` for genuine
  unknowns the source leaves unanswered. A Drafting Note is worth writing
  whenever the wrong interpretation would send someone in a meaningfully
  different direction.
- The enrichment loop (3.3) is per candidate. Do not pre-generate enriched
  drafts for the whole batch — enrichment depends on the user's answers
  and the model's research, both of which differ per candidate. Build the
  source-derived skeleton (3.1), present it (3.2), then enrich one
  candidate at a time.
- Web research (`{web-search-researcher agent}`) is a first-class step
  inside enrichment. Spawn it whenever there is uncertainty about
  domain, business, competitive, or technical aspects of the candidate.
  Skip only when the candidate is self-contained and well-understood.
- Thin drafts (accepted as-is) and enriched drafts coexist in the same
  work items directory. Thin drafts must carry the verbatim `Drafting Notes`
  entry recording non-enrichment so a future `/refine-work-item` invocation (or
  manual review) can identify them as needing follow-up before promotion
  from `draft` to `ready`.
- Acceptance criteria in enriched drafts must be specific and testable;
  prefer Given/When/Then for story/task. Challenge any criterion that
  is not measurable before accepting it into the draft.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh extract-work-items`
