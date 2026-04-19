---
name: create-ticket
description: Interactively create a well-formed ticket. Use when capturing a
  feature, bug, task, spike, or epic as a structured ticket in meta/tickets/.
argument-hint: "[topic or description]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)
---

# Create Ticket

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh create-ticket`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`

## Ticket Template

The template below defines the sections and frontmatter fields that every
ticket must contain. Read it now — use it to guide what information you gather
in Step 1 and what structure you produce in Steps 3–4.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh ticket`

You are tasked with guiding the user through creating a well-formed ticket —
a structured document capturing a feature, bug, task, spike, or epic for
tracking and implementation. This is a collaborative, challenging conversation:
the model contributes its own knowledge and research alongside the user's input
rather than simply transcribing what the user says. The goal is a ticket that
is well-reasoned and well-specified — one that would stand on its own without
needing the author present to explain it.

## Step 0: Parameter Check

When this command is invoked:

1. **If a topic was provided as an argument**: Check whether it is too vague
   (no clear deliverable or subject).
   - Vague examples: "improve things", "fix the API", "add more features"
   - Clear examples: "add full-text search to the docs index page",
     "fix login timeout after password reset"
   - If vague, ask at least one clarifying question ("What specifically needs
     to change?", "What does done look like?") and wait for a more specific
     description before proceeding to Step 1.

2. **If no argument was provided**, respond with:

```
I'll help you create a well-specified ticket through a collaborative conversation.

To get started, describe what you want to achieve — what's the problem or goal?
For example: "add full-text search to the docs index page", or "users can't log in after resetting their password"

I'll ask a few questions to understand the problem space, do some research, then
work with you to shape a thoroughly reasoned ticket.
```

Then wait for the user's input. Apply the same vagueness check to their
response — if still vague, ask a clarifying question before proceeding to
Step 1.

## Step 1: Gather Business Context

Once the topic is clear, ask 3–5 open questions to understand the business
context before investigating. Consult the ticket template sections above to
understand what the ticket will need — use them to guide which questions are
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

1. **Spawn {documents locator agent}** to search `{tickets_dir}` only —
   look for existing tickets with similar titles, descriptions, or scope.
   Do not search research documents, plans, or other directories — tickets
   capture business requirements, not implementation details.

2. **Spawn {web-search-researcher agent}** when there is uncertainty or a need
   for richer context about any aspect of the topic — business rules, domain
   concepts, competitive landscape, industry standards, external technology, or
   anything the model lacks confidence on. Skip this agent only when the topic
   is entirely self-contained and well-understood from the user's description.

Run both agents in parallel where both are warranted.

Once agents complete, synthesise findings — what the model knows from training,
what research turned up, what prior tickets exist — as the foundation for Step 3.

**If a similar existing ticket is found**: Check its status.
- If the status is `done`, `abandoned`, or `superseded` — mention the ticket
  briefly and note it is already closed, then continue creating the new one.
- Otherwise — surface the ticket and offer the user numbered options, for
  example:
  1. Proceed with a new ticket (if the scope genuinely differs)
  2. Exit and update the existing ticket instead (use `/update-ticket` once
     available; for now, update it manually)
  3. Continue creating a new ticket linked to the existing one as a parent

  Adapt the options to what makes sense given the ticket's type and status.
  Do not silently continue or modify the existing ticket inline. Wait for
  the user's choice before proceeding.

## Step 3: Propose and Refine

Using the business context and investigation synthesis, the model leads with
a structured proposal. Do not ask the user to generate requirements or
acceptance criteria from scratch — propose them and invite challenge.

1. **Recommend a ticket type** with a brief rationale, reading valid types from
   the ticket template's `type` field (story, epic, task, bug, spike).
2. **Draft requirements** drawn from the business context and research.
3. **Draft acceptance criteria** — specific and testable; prefer Given/When/Then
   format for story/task. Draw on domain knowledge and research to make these
   thorough, not just what the user explicitly mentioned.
4. **Flag assumptions** made in the proposal and questions that remain open.

Present as a structured proposal:

```
Based on what you've told me and my research, here's what I think this ticket needs:

**Suggested type**: [type]
**Rationale**: [one sentence]

**Requirements I'd suggest**:
- [requirement]

**Out of scope** (explicitly not captured here):
- [item]

**Acceptance criteria I'd suggest**:
- Given/When/Then...

**Assumptions I've made**:
- [assumption] — is this correct?

**Open questions**:
- [anything the user should clarify before drafting]
```

Wait for the user to validate, push back, or refine. Challenge vague or
untestable responses — if an acceptance criterion is not measurable (e.g.,
"it works correctly"), first ask a clarifying question to understand the
intent ("what would a passing test actually look like here?"). Only after
understanding the intent should you help reformulate it into something
testable. Do not accept weak criteria into the draft without first
understanding what the user means. Iterate until the proposal is
well-specified and agreed.

## Step 4: Draft Ticket

1. **Draft a complete ticket** from the agreed proposal using the template
   structure loaded at the top of this skill. Use `XXXX` as the placeholder
   ticket number throughout. Do NOT call `ticket-next-number.sh` at this step.

2. **Type-specific content placement**:
   - story/epic: open the `Summary` section with a user story statement —
     "As a [role], I want [goal], so that [benefit]." — before the
     descriptive sentences
   - bug reproduction steps, expected/actual behaviour → `Requirements` section
   - spike research questions, time-box, exit criteria → `Requirements` section
   - epic initial stories → `Requirements` section as a list
   - Do not rename or add sections beyond those defined in the ticket template

3. **Present the full draft** to the user:

```
Here's my draft ticket. Please review and let me know if you'd like any
changes before I write it to disk:

[draft content]
```

4. **Continue to challenge** during the review loop — flag untestable criteria,
   vague requirements, or gaps surfaced by research that remain unaddressed.
   Iterate until the user explicitly approves. **`ticket-next-number.sh` is
   never called during this loop.**

## Step 5: Write Ticket

1. **Call `ticket-next-number.sh`** to get the next number NNNN:

```
${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/ticket-next-number.sh
```

If the script exits non-zero (e.g., 9999 overflow), abort immediately and
surface the error message verbatim — do not proceed.

2. **Resolve the target path**: `{tickets_dir}/NNNN-kebab-slug.md`
   where the slug is a meaningful kebab-case summary of the title (not raw
   input text).

3. **Check that the target path does not already exist**. If it does, abort:

```
Path {path} already exists — another session may have written a ticket
concurrently. Please re-run /create-ticket.
```

4. **Create the tickets directory** if it does not exist.

5. **Substitute `XXXX` with `NNNN`** throughout the draft and write the file.

6. **Print a confirmation**:

```
Ticket created: `{tickets_dir}/NNNN-kebab-slug.md`
```

## Quality Guidelines

- Never write a file without explicit user approval.
- Never call `ticket-next-number.sh` before the user approves the draft.
- The slug must be a meaningful kebab-case title, not raw input text.
- All eight frontmatter fields must be populated in the written file:
  `ticket_id` (matching NNNN), `date`, `author`, `type`, `status` (draft),
  `priority` (medium unless specified otherwise), `parent`, `tags`. No field
  may contain unfilled placeholder text.
- `Summary` and `Requirements` must have substantive content — no `[bracketed
  placeholder text]` in the final written file.
- Acceptance criteria must be specific and testable; prefer Given/When/Then
  format. Challenge any criterion that is not measurable before accepting it.
- The model must contribute its own knowledge and research to the proposal —
  not simply transcribe the user's answers. Bring domain expertise, surfaced
  research, and reasoned suggestions to every ticket.
- Restrict `{documents locator agent}` to `{tickets_dir}` only. Do not search
  research documents, plans, or the codebase — tickets are about business
  requirements, not implementation details.
- Spawn `{web-search-researcher agent}` whenever there is uncertainty about any
  aspect of the topic — business rules, domain concepts, competitive landscape,
  industry standards, external technology, or otherwise.
- If `ticket-next-number.sh` exits non-zero, abort and surface the error
  message verbatim.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh create-ticket`
