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
