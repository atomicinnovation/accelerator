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

# Extract Tickets from Meta Documents

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh extract-tickets`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`

## Ticket Template

The template below defines the sections and frontmatter fields that every
ticket must contain. Read it now — the valid ticket types live in the `type`
field (not a hardcoded list elsewhere in this skill), and every written file
must populate every frontmatter field.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh ticket`

You are tasked with identifying requirements, work items, and actionable
tasks within existing meta documents and helping the user capture them as
formal tickets. Source documents typically tell you *what* tickets should
exist but rarely give the full business context, testable acceptance
criteria, dependencies, and assumptions a good ticket needs. The model,
the user, and web research fill those gaps.

Extraction therefore proceeds in two layers:

- **Source-derived content stays faithful.** Anything you draw from the
  source documents must reflect what they actually say — do not silently
  invent requirements that are not there.
- **Business-context gaps are surfaced.** The `Assumptions`, `Open Questions`
  and `Drafting Notes` sections serve different purposes when present, and all 
  matter:
  - **Assumptions** are interpretations you made that affect the ticket's
    meaning. Flag one only when using the wrong interpretation would lead
    someone to build something different. Example: *"Interpreted 'users' as
    end users rather than internal staff — if wrong, scope changes."*
  - **Open Questions** are genuine unknowns the source raises but leaves
    unanswered that a reader or implementer needs resolved before work can
    proceed. Example: *"What does 'better results' mean — improved relevance, 
    faster delivery, or both?"*
  - **Drafting Notes** capture interpretations you made while filling out
    the ticket — business-context calls, scope decisions, or technical
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
*enriching* the ticket interactively (with model knowledge, web research,
and a few focused questions, similar to `/create-ticket`) or *accepting
the source-derived skeleton as-is* and refining later. The enrichment
loop is per candidate, not pre-generated, so the work the model invests
matches the depth the user wants for each ticket.

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

     Which documents should I scan for tickets? (enter numbers, "all", or
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

Which items would you like to create tickets for? (enter numbers, "all",
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

- Infer the ticket type from its content using types read from the
  ticket template's `type` field:
  - clear bug reports with symptoms and expected/actual behaviour → `bug`
  - open-ended investigations with specific questions → `spike`
  - broad multi-deliverable themes → `epic`
  - specific single deliverables → `story`
  - one-off technical or operational tasks → `task`
  - Default to `story` for items where the type is genuinely ambiguous.
- Draft a complete ticket from the source content alone, using `XXXX`
  as the placeholder ticket number.
- Type-specific content placement:
  - bug: reproduction steps, expected/actual behaviour → `Requirements` section
  - spike: research questions, time-box, exit criteria → `Requirements` section
  - epic: initial story decomposition → `Requirements` section as a list
  - Do not rename or add sections beyond those in the ticket template.
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

[ticket content with XXXX placeholder, including Drafting Notes section]

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

Treat this as a focused, per-candidate version of the `/create-ticket`
flow, seeded with the skeleton above:

1. **Ask 1–3 focused business-context questions** tailored to this
   candidate. Fewer than `/create-ticket`'s 3–5 because the source
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
following note **verbatim** (so future tooling like `/refine-ticket`
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

#### 3.7 No `ticket-next-number.sh` calls in Step 3

`ticket-next-number.sh` is not called at any point during Step 3,
regardless of which option the user picks. Writing happens exclusively
in Step 4 after all approvals — enriched and thin — are collected.

### Step 4: Write Tickets

1. **Count approved (non-skipped) items: N.**

2. **If N is 0**: print "No tickets approved — nothing written." and exit
   cleanly. Do NOT call `ticket-next-number.sh`.

3. **Otherwise**:

   a. **Compute target slugs** — for each approved draft, derive a meaningful
      kebab-case slug from its title.

   b. **Create the tickets directory** if it does not exist.

   c. **Verify all N target slugs are free** before allocating any numbers.
      Numbers are not yet known, so check by slug pattern: for each approved
      draft's slug, confirm that no file matching
      `{tickets_dir}/[0-9][0-9][0-9][0-9]-{slug}.md` already exists. If any
      slug collides, report which slugs collide and which existing files
      they match, abort without calling `ticket-next-number.sh`, and ask
      the user to resolve the collision (rename or remove) before re-running.

   d. **Call `ticket-next-number.sh --count N` exactly once**:
      ```
      ${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/ticket-next-number.sh --count N
      ```
      If the script exits non-zero (e.g., 9999 overflow), abort immediately
      and surface the error message verbatim — do not write any files. The
      script may emit partial numbers on stdout before exiting non-zero;
      ignore those numbers and write nothing.

   e. **Substitute sequential numbers** into approved drafts in their
      original presented order — the first approved draft (lowest position
      in the candidate list) receives the first number, and so on. Do not
      reorder by approval timestamp or by user selection order.

   f. **Write all N ticket files**. Each ticket's `References` section must
      include all source document paths the item was extracted from. For
      deduplicated items that appeared in multiple documents, list every
      contributing source under `References`, one per line.

   g. If a write error occurs mid-batch: report which numbers were allocated,
      which files were written successfully, and which were not — so the user
      can manually write the missing files with their pre-assigned numbers.
      Do not retry writes silently and do not call `ticket-next-number.sh`
      again to re-allocate; the original allocation stands. The user needs
      to know the exact state.

4. **Print a summary table**:

```
Created the following tickets:
| Number | Title | File |
|--------|-------|------|
| 0001   | [title] | `{tickets_dir}/0001-slug.md` |
| 0002   | [title] | `{tickets_dir}/0002-slug.md` |
...
```

## Quality Guidelines

- Never call `ticket-next-number.sh` before all approvals are collected.
  The number space is shared and finite; consuming numbers for drafts the
  user might still skip creates gaps that are impossible to clean up later.
- Never call `ticket-next-number.sh` when N=0. An all-skipped session must
  exit cleanly with no side effects.
- If `ticket-next-number.sh` exits non-zero, abort immediately and surface
  the script's error output verbatim — even if it emitted some numbers on
  stdout before failing, treat the entire batch as failed.
- Verify all target slugs are free BEFORE calling `ticket-next-number.sh` —
  collision checks happen before number allocation, by slug pattern, since
  numbers are not yet known.
- Numbers are assigned to approved drafts in their original presented
  order, not in approval timestamp order. This makes outputs deterministic
  and matches the order the user reviewed.
- Every written ticket MUST include all source document paths in its
  `References` section. For deduplicated items that appeared in multiple
  documents, list every contributing source.
- Do not extract structural or navigational content (table of contents
  entries, section headings with no requirements content, agenda items
  with no actionable outcome) as candidate tickets. If a heading just
  organises content rather than describing work, skip it.
- Ticket type inference must use types read from the ticket template
  frontmatter (loaded at the top of this skill), not a hardcoded list.
  Default to `story` for items where the type is genuinely ambiguous.
- All frontmatter fields defined in the ticket template must be populated
  in every written ticket — `ticket_id` matching the assigned NNNN, `title`
  matching the ticket's title, `date`, `author`, `type`, `status` (draft),
  `priority` (medium unless the source implies otherwise), `parent` (empty
  string unless the source establishes a parent), and `tags` (a YAML
  array, possibly empty). No field may contain unfilled placeholder text
  like `[author]` or `NNNN`. The body H1 format is `# NNNN: <title>` —
  kept in sync with the frontmatter `title:` field.
- `date` must use the ticket template's `YYYY-MM-DDTHH:MM:SS+00:00`
  format in UTC (e.g. obtained via `date -u +%Y-%m-%dT%H:%M:%S+00:00`).
- `author` is sourced in this order: configuration if present, then the
  source document's author/owner field if named, then the current git/jj
  user identity, then — only if all of those fail — ask the user once
  before writing the batch. Never write `[author]` or any placeholder.
- "Accept remaining as-is" only marks unreviewed candidates as approved
  (thin) — it does not resurrect skipped candidates, and writing still
  happens exclusively in Step 4 after the single `ticket-next-number.sh`
  call.
- Source-derived content stays faithful to what the source documents say.
  Do not silently invent requirements. When you make an interpretation while
  filling out the ticket — about scope, stakeholders, terminology, or implied
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
  tickets directory. Thin drafts must carry the verbatim `Drafting Notes`
  entry recording non-enrichment so a future `/refine-ticket` invocation (or
  manual review) can identify them as needing follow-up before promotion
  from `draft` to `ready`.
- Acceptance criteria in enriched drafts must be specific and testable;
  prefer Given/When/Then for story/task. Challenge any criterion that
  is not measurable before accepting it into the draft.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh extract-tickets`
