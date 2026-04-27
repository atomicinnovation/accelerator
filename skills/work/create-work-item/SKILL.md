---
name: create-work-item
description: Interactively create a well-formed work item. Use when capturing a
  feature, bug, task, spike, or epic as a structured work item in meta/work/.
argument-hint: "[topic or existing work item path/number]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---

# Create Work Item

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh create-work-item`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work`

## Work Item Template

The template below defines the sections and frontmatter fields that every
work item must contain. Read it now — use it to guide what information you gather
in Step 1 and what structure you produce in Steps 3–4.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh work-item`

You are tasked with guiding the user through creating a well-formed work item —
a structured document capturing a feature, bug, task, spike, or epic for
tracking and implementation. This is a collaborative, challenging conversation:
the model contributes its own knowledge and research alongside the user's input
rather than simply transcribing what the user says. The goal is a work item that
is well-reasoned and well-specified — one that would stand on its own without
needing the author present to explain it.

The work item template has three sections that look similar but capture
different things — populate each deliberately:

- **Assumptions** are interpretations you made that affect what gets built.
  Flag one only when using the wrong interpretation would lead someone to
  build something different. Example: *"Treating 'users' as end users
  rather than internal staff — if wrong, scope changes."*
- **Open Questions** are genuine unknowns a reader or implementer needs
  resolved before work can proceed. Example: *"What does 'better results'
  mean — improved relevance, faster delivery, or both?"*
- **Drafting Notes** capture interpretations you made while filling out the
  work item — business-context calls, scope decisions, or technical choices
  that someone should review if they turn out to be wrong. Actively
  populate this section. If you inferred who the stakeholders are, what a
  vague term means, where the scope boundary sits, or which technical
  approach is implied, write it down. Routine field selections (type,
  priority, tags) don't need an entry unless the choice reflects a
  substantive scope or meaning interpretation a reviewer should be aware
  of.

## Step 0: Parameter Check

When this command is invoked:

1. **If an argument was provided**:

   First, **try to resolve the argument as a reference to an existing work
   item**. The discriminator order is path-like → numeric.

   - **Path-like** — argument contains `/` or ends in `.md`: treat as a file
     path. Resolve relative to the user's current working directory if
     relative, or use the argument verbatim if absolute.
     - File exists and resolves: continue to frontmatter validation below.
     - File does not exist: print
       `"No work item at <path> — interpreting as topic string. If that's
       wrong, abort and re-run with a different argument (or
       /list-work-items to find a valid path)."` and proceed to
       topic-string handling below.

   - **Numeric** — argument matches `^[0-9]+$`: zero-pad to 4 digits (or
     use as-is if already ≥4) and glob `{work_dir}/NNNN-*.md`. The glob
     is case-sensitive and does not recurse.
     - One match: continue to frontmatter validation below.
     - Multiple matches: list them as numbered options and ask the user to
       select by number or specify the full path.
     - Zero matches: print `"No work item numbered NNNN found in {work_dir}
       — interpreting as topic string. If that's wrong, abort and re-run
       with a different argument (or /list-work-items to find a valid
       number)."` and proceed to topic-string handling below.

   **Frontmatter validation** (only reached after a file was successfully
   resolved). Run:

   ```
   bash ${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh work_item_id <path>
   ```

   If it exits non-zero (frontmatter missing or unclosed), print:

   ```
   Could not parse frontmatter in <path> — the file may be corrupted.
   Re-open it and check that the YAML frontmatter is bracketed by two
   `---` lines and contains all nine required fields.
   ```

   and exit without spawning agents or writing any file. (The file clearly
   resolved, so the user intended a path — no fallback applies.)

   If validation passes, read the file fully (frontmatter and body) and
   cache the identity fields in conversation state:
   `work_item_id`, `date`, `author`, `status`, `title`, `type`,
   `priority`, `parent`, `tags`. For any missing optional field, use the
   template default — do not abort. Set the conversation into
   **enrich-existing mode** with `existing_work_item_path` cached, and
   skip directly to Step 1 in that mode — do not run the vagueness check.

   **Topic-string handling**: if no resolution succeeded (discriminators
   did not trigger, or they triggered with a fallback warning), check
   whether the argument is too vague (no clear deliverable or subject).
   - Vague examples: "improve things", "fix the API", "add more features"
   - Clear examples: "add full-text search to the docs index page",
     "fix login timeout after password reset"
   - If vague, ask at least one clarifying question and wait for a more
     specific description before proceeding to Step 1.
   - If clear, proceed directly to Step 1.

2. **If no argument was provided**, respond with:

```
I'll help you create a well-specified work item through a collaborative conversation.

To get started, describe what you want to achieve — what's the problem or goal?
For example: "add full-text search to the docs index page", or "users can't log in after resetting their password"

I'll ask a few questions to understand the problem space, do some research, then
work with you to shape a thoroughly reasoned work item.
```

Then wait for the user's input. Apply the same vagueness check to their
response — if still vague, ask a clarifying question before proceeding to
Step 1.

## Step 1: Gather Business Context

Once the topic is clear, ask 3–5 open questions to understand the business
context before investigating. Consult the work item template sections above to
understand what the work item will need — use them to guide which questions are
worth asking. Tailor the questions to the topic, but cover:

- What pain point or problem does this address, and who experiences it?
- What is the desired outcome — what changes for people once this is done?
- Does the user have a solution or approach in mind? If so, surface it and
  ask whether it's a constraint or a preference to be explored further.
- Are there any constraints, deadlines, or dependencies worth knowing?
- For bug topics: what is the impact, and is it a blocker?
- Is there anything you are uncertain about, or that I should research?

Ask all relevant questions at once rather than one at a time. Wait for the
user's answers before proceeding to Step 2.

### In enrich-existing mode

Do not ask the broad discovery questions above. Instead:

1. Identify which body sections of the existing file are **substantive** (real
   content beyond `[bracketed placeholder]` text) and which are **gaps** (empty,
   placeholder-only, or missing entirely). Tag each gap with **exactly one** of
   the literal tokens:
   - `empty` — section is absent or contains no content
   - `placeholder-only` — section contains only template `[...]` blocks
   - `instructional-prose` — section contains the template's instructional
     prose carried over verbatim (e.g. "Describe the business value of this
     work item here…")
   - `partial` — section contains one or more substantive sentences alongside
     residual placeholders

   Use the literal token; do not paraphrase. The eval grader pattern-matches
   on these exact strings.

2. Present the gap analysis briefly:

   ```
   I've read the existing work item (<resolved path>). Here's what looks
   complete and what still needs work:

   Complete: [section list, or "none"]
   Gaps:
     - <Section> (<tag>)
     - ...

   I'll ask targeted questions about the gaps.
   ```

3. Ask only questions that address the identified gaps. Do not re-ask
   questions whose answers are already substantively present in the file.

4. If the existing content is rich enough that no obvious gaps remain,
   briefly confirm this and ask what the user wants to add or improve, then
   proceed to Step 2.

## Step 2: Investigate

Using the topic and business context from Step 1, run investigation agents
in parallel:

1. **Spawn {documents locator agent}** to search `{work_dir}` only —
   look for existing work items with similar titles, descriptions, or scope.
   Do not search research documents, plans, or other directories — work items
   capture business requirements, not implementation details.

2. **Spawn {web-search-researcher agent}** when there is uncertainty or a need
   for richer context about any aspect of the topic — business rules, domain
   concepts, competitive landscape, industry standards, external technology, or
   anything the model lacks confidence on. Skip this agent only when the topic
   is entirely self-contained and well-understood from the user's description.
   When in doubt, prefer to spawn research — over-asking is cheaper than
   producing a vague or poorly-grounded proposal.

Run both agents in parallel where both are warranted.

Once agents complete, synthesise findings — what the model knows from training,
what research turned up, what prior work items exist — as the foundation for Step 3.

**If a similar existing work item is found**: Check its status.
- If the status is `done`, `abandoned`, or `superseded` — mention the work item
  briefly and note it is already closed, then continue creating the new one.
- Otherwise — surface the work item and offer the user numbered options, for
  example:
  1. Proceed with a new work item (if the scope genuinely differs)
  2. Exit and update the existing work item instead (use `/update-work-item` once
     available; for now, update it manually)
  3. Continue creating a new work item linked to the existing one as a parent

  Adapt the options to what makes sense given the work item's type and status.
  Do not silently continue or modify the existing work item inline. Wait for
  the user's choice before proceeding.

**In enrich-existing mode**: exclude the resolved input file from the
similarity scan. The {documents locator agent} search of `{work_dir}` will
find the file being enriched; do not surface it as a "potential duplicate" of
itself. Other near-duplicates discovered are handled per the unchanged rules
above.

## Step 3: Propose and Refine

Using the business context and investigation synthesis, the model leads with
a structured proposal. Do not ask the user to generate requirements or
acceptance criteria from scratch — propose them and invite challenge.

1. **Recommend a work item type** with a brief rationale. Valid types come from
   the work item template's `type` field (loaded at the top of this skill) — do
   not hardcode the list. Default to `story` when the type is genuinely
   ambiguous.
2. **Draft requirements** drawn from the business context and research.
3. **Draft acceptance criteria** — specific and testable; prefer Given/When/Then
   format for story/task. Draw on domain knowledge and research to make these
   thorough, not just what the user explicitly mentioned.
4. **Surface interpretations and gaps** using the right section:
   - Put scope-changing interpretations you made in `Assumptions` and invite
     the user to confirm or correct them.
   - Put genuine unknowns that block progress in `Open Questions`.
   - Put every meaningful drafting-time interpretation (who stakeholders are,
     what vague terms mean, scope boundaries, implied technical approach) in
     `Drafting Notes` so a reviewer can challenge them.

Present as a structured proposal:

```
Based on what you've told me and my research, here's what I think this work item needs:

**Suggested type**: [type]
**Rationale**: [one sentence]

**Requirements I'd suggest**:
- [requirement]

**Out of scope** (explicitly not captured here):
- [item]

**Acceptance criteria I'd suggest**:
- Given/When/Then...

**Assumptions I've made** (scope-changing calls — confirm or correct):
- [assumption]

**Drafting Notes** (interpretations a reviewer should see):
- [interpretation — e.g. who stakeholders are, what a vague term means,
  scope boundary, implied technical approach]

**Open questions** (unknowns to resolve before work begins):
- [anything the user should clarify before drafting]
```

Wait for the user to validate, push back, or refine. Challenge vague or
untestable responses — if an acceptance criterion is not measurable (e.g.,
"it works correctly"), first ask a clarifying question to understand the
intent ("what would a passing test actually look like here?"). Only after
understanding the intent should you help reformulate it into something
testable. Do not accept vague criteria into the draft. Iterate until the
proposal is well-specified and agreed.

### In enrich-existing mode

Do not lead with a from-scratch proposal. Instead present a section-by-section
review and augmentation:

```
Here's how the existing work item reads against my research, with proposed
additions for the gaps:

**[Section name]** — [complete | needs improvement: <reason> | missing]
[existing content excerpt or note that it is missing]
[proposed addition or replacement, when applicable]

[repeat per section]

**Title**: [keep / propose new title with rationale]
**Type**: [keep existing <type> / propose change to <type> with rationale]
**Priority**: [keep / propose change with rationale]
**Parent**: [keep / propose change]
**Tags**: [keep / propose additions]
**Status**: [keep <cached> — say so if you'd like to transition it]
```

Apply the canonical Identity Field Rules (see Quality Guidelines): propose
only the fields marked Proposable; never propose changes to immutable fields;
surface a status transition only if the user makes a direct explicit request
for one (the listing offers the affordance but never proposes a change
unsolicited).

The refinement loop in this step (challenging untestable criteria, vague
requirements, etc.) applies equally to existing and proposed content.

## Step 4: Draft Work Item

1. **Draft a complete work item** from the agreed proposal using the template
   structure loaded at the top of this skill. Use `XXXX` as the placeholder
   work item number throughout. Do NOT call `work-item-next-number.sh` at this step.

2. **Type-specific content placement**:
   - story/epic: open the `Summary` section with a user story statement —
     "As a [role], I want [goal], so that [benefit]." — before the
     descriptive sentences
   - bug reproduction steps, expected/actual behaviour → `Requirements` section
   - spike research questions, time-box, exit criteria → `Requirements` section
   - epic initial stories → `Requirements` section as a list
   - Do not rename or add sections beyond those defined in the work item template

3. **Populate the `References` section** with any external material that
   informed the work item — research artefacts surfaced by the web researcher,
   related work items found in Step 2, design docs, or source specs. Leave it
   empty if nothing external was consulted; do not invent references.

4. **Populate the `Drafting Notes` section** with the interpretations
   carried over from the agreed proposal. If there were none (the user
   confirmed every call), the section may be omitted or left as an empty
   list — but do not leave placeholder bracketed text in it.

5. **Present the full draft** to the user:

```
Here's my draft work item. Please review and let me know if you'd like any
changes before I write it to disk:

[draft content]
```

6. **Continue to challenge** during the review loop — flag untestable criteria,
   vague requirements, or gaps surfaced by research that remain unaddressed.
   Iterate until the user explicitly approves. **`work-item-next-number.sh` is
   never called during this loop.**

### In enrich-existing mode

1. Produce a complete updated draft incorporating the existing content plus the
   additions approved in Step 3.
2. Apply the canonical Identity Field Rules (see Quality Guidelines) when
   filling frontmatter — the rules are the single source of truth for which
   fields are immutable, preserved, or proposable.
3. Apply the H1 sync rule (see Quality Guidelines).
4. Apply the Script avoidance rule (see Quality Guidelines).
5. Present the full updated draft, framed as an update preview:

   ```
   Here's the updated work item. The Step 5 confirmation will name the file
   path and ask for explicit approval before any write:

   [draft content]
   ```

6. Continue to challenge during review — apply the same rules as the
   normal-path Step 4. Iterate the draft until the user is happy with it. Do
   **not** treat a "looks good" mid-iteration as approval to write — that
   approval is gated by Step 5's single confirmation.

## Step 5: Write Work Item

1. **Call `work-item-next-number.sh`** to get the next number NNNN:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-next-number.sh
```

If the script exits non-zero (e.g., 9999 overflow), abort immediately and
surface the error message verbatim — do not proceed.

2. **Resolve the target path**: `{work_dir}/NNNN-kebab-slug.md`
   where the slug is a meaningful kebab-case summary of the title (not raw
   input text).

3. **Check that the target path does not already exist**. If it does, abort:

```
Path {path} already exists — another session may have written a work item
concurrently. Please re-run /create-work-item.
```

4. **Create the work items directory** if it does not exist.

5. **Substitute `XXXX` with `NNNN`** throughout the draft and write the file.

6. **Print a confirmation**:

```
Work item created: `{work_dir}/NNNN-kebab-slug.md`
```

### In enrich-existing mode

1. Do **not** call `work-item-next-number.sh`. The target path is the resolved
   `existing_work_item_path` cached in Step 0.

2. **At-write identity-swap check** (best-effort guard): immediately before
   the confirmation prompt, re-read the target file's `work_item_id` from
   disk via:

   ```
   bash ${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh work_item_id <existing_work_item_path>
   ```

   - If the script exits non-zero (file gone or frontmatter unparseable since
     Step 0), abort with:
     `"Error: <path> is no longer present or its frontmatter is unparseable.
     Your proposed draft is below — copy it before re-running
     /create-work-item: <draft>"`
   - If the on-disk `work_item_id` differs from the value cached in Step 0,
     abort with:
     `"Error: <path> changed identity since Step 0 (was <cached>, now
     <current>). Your proposed draft is below — copy it before re-running
     /create-work-item to refresh: <draft>"`

3. **Single confirmation gate**: present the path and a per-section change
   summary, then require explicit y/n approval before any write:

   ```
   I'm about to overwrite <existing_work_item_path>.

   Sections changed:
     Added:    [...]
     Modified: [...]

   Frontmatter changed:
     <field>: <cached> → <new>     # repeated per modified field
     (status: <cached> unchanged)  # always show status explicitly

   Sections preserved verbatim: <count> (<terse list or "none">)
   Frontmatter fields preserved verbatim: work_item_id, date, author
     [+ any unmodified proposable fields]

   Proceed? (y/n)
   ```

4. **Confirmation interpretation** (fail-safe):
   - Exactly `y` or `Y` (after trimming whitespace): proceed to step 5.
   - Exactly `n` or `N`: go to step 8.
   - Anything else (empty input, "yes", "go ahead", "looks good but also...",
     paragraph of feedback): treat as `n`. Print:
     `"Did not recognise <response> as y/n — staying in review. What change
     would you like before I overwrite?"` and go to step 8.

5. **On `y` — substitute cached immutable fields, then write**: immediately
   before invoking the Write tool, re-read the cached immutable identity fields
   (`work_item_id`, `date`, `author`) from conversation state and overwrite the
   corresponding frontmatter lines in the draft text with those exact values,
   even if they appear unchanged. This textual substitution defends against
   drift during Step 4 iteration.

6. Write the file to `existing_work_item_path`. The path-existence guard from
   the normal flow does not apply — overwrite is the intended behaviour.

7. Print:

   ```
   Work item updated: <existing_work_item_path>
   ```

8. **On `n` or unrecognised**: stay in Step 4 / Step 5 review and iterate.
   Do not re-run the identity-swap check until the next `y`; do not write.

## Quality Guidelines

- Never write a file without explicit user approval.
- Never call `work-item-next-number.sh` before the user approves the draft.
- The slug must be a meaningful kebab-case title, not raw input text.
- Work item type must come from the work item template's `type` field (loaded at
  the top of this skill), not a hardcoded list. Default to `story` when the
  type is genuinely ambiguous.
- All frontmatter fields defined in the work item template must be populated
  in every written work item: `work_item_id` matching the assigned NNNN, `title`
  matching the user-approved title, `date`, `author`, `type`, `status`
  (draft), `priority` (medium unless the user specified otherwise),
  `parent` (empty string unless a parent was established), and `tags`
  (a YAML array, possibly empty). No field may contain unfilled
  placeholder text like `[author]` or `NNNN`. The body H1 format is
  `# NNNN: <title>` — kept in sync with the frontmatter `title:` field.

**Identity Field Rules** (apply in enrich-existing mode):

- **Immutable** — `work_item_id`, `date`, `author`. Cached from the source
  file in Step 0; never proposed for change; the model substitutes the cached
  values back into the draft frontmatter at write time (Step 5 step 5) as a
  defence against drift during Step 4 iteration. This is a textual
  substitution the model performs — the eval suite verifies the written file's
  values match the cached values, which is the strongest guarantee the
  grader-mediated harness can provide.
- **Preserved unless explicitly changed** — `status`. Defaults to the cached
  value. May only change if the user makes an explicit, direct request during
  the conversation (e.g. "set status to in-progress"). The model must not
  propose a status change unsolicited; if the user makes only an oblique
  reference (e.g. "this is now in flight"), the model asks a clarifying
  question rather than infer the transition. A proposed transition is shown
  explicitly (e.g. "draft → in-progress") and requires confirmation in Step 3
  before acceptance. The `status: draft` default applies only to
  newly-created work items, not to enrichment.
- **Proposable in Step 3** — `title`, `type`, `priority`, `parent`, `tags`.
  Default to the cached values; can be replaced after explicit user agreement
  in Step 3's augmentation review.

**H1 sync** (enrich-existing mode): the body H1 is `# <work_item_id>: <title>`,
using the cached immutable `work_item_id` (4-digit, never `XXXX`) and the
title as confirmed or replaced in Step 3.

**Script avoidance** (enrich-existing mode): `work-item-next-number.sh` is
never called. The number is already cached. The path-existence guard in Step 5
does not apply — overwrite is the intended behaviour once the at-write
identity-swap check passes.

- `date` must use the work item template's `YYYY-MM-DDTHH:MM:SS+00:00` format
  in UTC (e.g. obtained via `date -u +%Y-%m-%dT%H:%M:%S+00:00`).
- `author` is sourced in this order: configuration if present, then the
  current git/jj user identity, then — only if both fail — ask the user
  once before writing the file. Never write `[author]` or any placeholder.
- `Summary` and `Requirements` must have substantive content — no
  `[bracketed placeholder text]` in the final written file. The same
  applies to every other body section that is populated: if a section
  would only contain a bracketed placeholder, remove the placeholder and
  leave the section empty (or omit optional sections entirely).
- Acceptance criteria must be specific and testable; prefer Given/When/Then
  format. Challenge any criterion that is not measurable before accepting it.
- Populate `Drafting Notes` with every meaningful interpretation made while
  filling out the work item — who stakeholders are, what a vague term means,
  where scope boundaries sit, which technical approach is implied. Use
  `Assumptions` for scope-changing calls the user should confirm. Use
  `Open Questions` for genuine unknowns that block progress. These three
  sections serve different purposes; don't collapse them.
- Populate `References` with any external material that informed the
  work item — research artefacts, related work items, design docs, source specs.
  Leave it empty if nothing external was consulted; do not invent
  references.
- The model must contribute its own knowledge and research to the proposal —
  not simply transcribe the user's answers. Bring domain expertise, surfaced
  research, and reasoned suggestions to every work item.
- Restrict `{documents locator agent}` to `{work_dir}` only. Do not search
  research documents, plans, or the codebase — work items are about business
  requirements, not implementation details.
- Spawn `{web-search-researcher agent}` whenever there is uncertainty about any
  aspect of the topic — business rules, domain concepts, competitive landscape,
  industry standards, external technology, or otherwise. When in doubt,
  prefer to spawn research — over-asking is cheaper than producing a
  vague or poorly-grounded proposal.
- If `work-item-next-number.sh` exits non-zero, abort and surface the error
  message verbatim.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh create-work-item`
