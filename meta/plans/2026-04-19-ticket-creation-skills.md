---
date: "2026-04-19T00:00:00+01:00"
type: plan
skill: create-plan
status: draft
---

# Ticket Creation Skills (Phase 2)

## Overview

Implement two new skills — `create-ticket` (interactive creation) and
`extract-tickets` (batch extraction from documents) — following the same
structural conventions as `create-adr` and `extract-adrs`. Both skills are
developed TDD-style: eval scenarios are defined first, then
`/skill-creator:skill-creator` is used to author and validate each SKILL.md.

## Current State Analysis

Phase 1 is complete. The following artifacts are confirmed in place:

- `templates/ticket.md` — ticket template with eight frontmatter fields
  (`ticket_id`, `date`, `author`, `type`, `status`, `priority`, `parent`,
  `tags`) and standard body sections
- `skills/tickets/scripts/ticket-next-number.sh` — sequential numbering with
  `--count N` support; exits non-zero if N=0 or on 9999 overflow
- `skills/tickets/scripts/ticket-read-status.sh` — delegates to `read-field`
  for the `status` key
- `skills/tickets/scripts/ticket-read-field.sh` — generic frontmatter field
  extraction
- `skills/tickets/scripts/test-ticket-scripts.sh` — 44-test bash harness
  covering all three scripts (uses `scripts/test-helpers.sh`)
- `.claude-plugin/plugin.json` — `"./skills/tickets/"` already present in the
  skills array; all subdirectories are picked up automatically

The `skills/tickets/` directory contains only the `scripts/` subdirectory. No
`SKILL.md` files exist yet.

## Desired End State

After this plan is complete:

- `skills/tickets/create-ticket/SKILL.md` exists and is callable as
  `/create-ticket`
- `skills/tickets/extract-tickets/SKILL.md` exists and is callable as
  `/extract-tickets`
- `bash skills/tickets/scripts/test-ticket-scripts.sh` exits 0 with all 44
  tests passing (no regressions in Phase 1 scripts)

### Key Discoveries

- `skills/decisions/create-adr/SKILL.md` is the direct structural model for
  `create-ticket`: parameter check → parallel context agents → type + questions
  → draft/review loop (with XXXX placeholder) → write (number assigned here)
- `skills/decisions/extract-adrs/SKILL.md` is the direct structural model for
  `extract-tickets`: document selection → parallel `documents-analyser` agents
  → candidate list → per-draft review → batch numbering via `--count N` →
  write all
- Both skills defer `ticket-next-number.sh` until the write step — the number
  is only allocated when a file is actually written, preventing gaps from
  abandoned sessions
- `extract-adrs` additionally allows `skills/research/research-codebase/scripts/*`
  for `research-metadata.sh`; the ticket skills do not need this
- The `"./skills/tickets/"` plugin.json entry scans subdirectories, so no
  further registration changes are needed when adding skill subdirectories
- The agent fallback list must use fully-qualified `accelerator:` namespace
  prefixes (e.g. `accelerator:reviewer`), matching all reference skills

## What We're NOT Doing

- Not implementing `list-tickets`, `update-ticket`, `review-ticket`,
  `stress-test-ticket`, or `refine-ticket` (Phases 3–6)
- Not creating review lenses or output formats (Phases 4–5)
- Not modifying the ticket template or any Phase 1 script
- Not calling `research-metadata.sh` — ticket creation does not need git
  commit metadata
- Not enforcing ticket type or hierarchy rules at creation time

## Implementation Approach

Each subphase follows TDD order:
1. Eval scenarios in this plan are the specification (written first)
2. Invoke `/skill-creator:skill-creator` and provide the specification + eval
   scenarios from this plan
3. Run the evals built into the skill-creator flow
4. Iterate on the SKILL.md until all evals pass
5. Run the Phase 1 regression suite before marking the subphase done

---

## Subphase 2.1: `create-ticket` Skill

### Overview

Author `skills/tickets/create-ticket/SKILL.md` — an interactive skill that
gathers codebase and document context, uses those findings to inform type
selection and clarifying questions, presents a draft for review, and writes
the final ticket to the configured tickets directory. Ticket numbering is
deferred to the write step so that no number is consumed if the session is
abandoned.

### Eval Scenarios

These scenarios are the "test" half of TDD. Define them before authoring the
SKILL.md and pass them as the eval specification to `/skill-creator`.

**Scenario 1 — Bare invocation prompts for topic**
Input: `/create-ticket` (no arguments)
Expected: Skill asks for a description or topic and waits. Does not write any
file.

**Scenario 2 — Vague topic prompts clarification in Step 0**
Input: `/create-ticket improve things`
Expected: Skill recognises the topic as too vague and asks at least one
clarifying question ("What specifically needs improving?", "What does done look
like?") before proceeding to business context questions or investigation.

**Scenario 3 — Business context questions asked before investigation**
Input: `/create-ticket add search to the docs index page`
Expected: Skill asks 3–5 open questions to understand business context (who is
affected, what success looks like, constraints, etc.) before spawning any
agents. It does not investigate or propose anything until the user answers.

**Scenario 4 — Investigation restricted to tickets directory**
Input: Topic with business context provided
Expected: Skill spawns `{documents locator agent}` scoped to `{tickets_dir}`
only. It does not search research documents, plans, or the codebase.

**Scenario 5 — Web search spawned for domain or business uncertainty**
Input: Topic involving a domain concept or business rule the model is uncertain
about (e.g., a regulated industry workflow, a competitor feature, an unfamiliar
business process)
Expected: Skill spawns `{web-search-researcher agent}` to gather context on the
uncertain aspect before proposing requirements. The proposal reflects findings
from the research.

**Scenario 6 — Model proposes type with rationale**
Input: Topic and business context provided
Expected: Skill recommends a ticket type with a one-sentence rationale rather
than presenting a bare list and asking the user to choose. The user can
validate, challenge, or change the suggestion.

**Scenario 7 — Model proposes acceptance criteria; challenges untestable ones**
Input: Topic and business context provided; user validates the proposal but
offers a vague acceptance criterion ("it should work correctly")
Expected: Skill proposes its own specific, testable acceptance criteria from
domain knowledge and research. When the user offers an untestable criterion,
the skill pushes back and asks what a passing test would look like before
accepting it into the draft.

**Scenario 8 — Bug type: model elicits reproduction details; Requirements
contains them**
Input: Topic that is a bug report; business context provided
Expected: Skill recognises the bug type, asks for or proposes reproduction
steps, expected behaviour, actual behaviour, and environment. The draft's
`Requirements` section contains this information — the section is not renamed.

**Scenario 9 — Near-duplicate ticket surfaced during investigation**
Input: Topic that matches an existing ticket in the tickets directory
Expected: Skill surfaces the similar ticket and asks whether to proceed with a
new ticket or exit. It does not silently create a duplicate.

**Scenario 10 — Draft presented with XXXX before any file is written**
Input: Proposal agreed by user
Expected: Skill shows the full draft using `XXXX` as the ticket number
placeholder and asks for explicit approval or revision. No file is written and
`ticket-next-number.sh` is not called.

**Scenario 11 — Approved draft written to correct location with all fields**
Input: User approves the draft
Expected: File written to `{tickets_dir}/NNNN-kebab-slug.md` where NNNN is the
output of `ticket-next-number.sh` called at write time. All eight frontmatter
fields (`ticket_id`, `date`, `author`, `type`, `status`, `priority`, `parent`,
`tags`) are populated; `status` is `draft` and `ticket_id` matches the NNNN
prefix of the filename. `Summary` and `Requirements` contain substantive content
with no unfilled `[...]` placeholder text.

**Scenario 12 — Ticket number not consumed when session is abandoned**
Input: Business context and proposal agreed; draft presented but user does not
approve (session ends without approval)
Expected: `ticket-next-number.sh` has not been called. A subsequent invocation
receives the same next number as would have been assigned to the abandoned draft.

### Changes Required

#### 1. Create `skills/tickets/create-ticket/SKILL.md`

**File**: `skills/tickets/create-ticket/SKILL.md`

Invoke `/skill-creator:skill-creator` with this specification:

```
Skill name: create-ticket
Category: tickets
Model after: skills/decisions/create-adr/SKILL.md (follow its structure exactly)

Frontmatter:
  name: create-ticket
  description: Interactively create a well-formed ticket. Use when capturing a
  feature, bug, task, spike, or epic as a structured ticket in meta/tickets/.
  argument-hint: "[topic or description]"
  disable-model-invocation: true
  allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)

Configuration preamble (bang-executed in this order):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh create-ticket`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

Agent fallback (if no Agent Names section above):
  accelerator:reviewer, accelerator:codebase-locator, accelerator:codebase-analyser,
  accelerator:codebase-pattern-finder, accelerator:documents-locator,
  accelerator:documents-analyser, accelerator:web-search-researcher

Path injection:
  **Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`

Template section (static, in skill body — same placement as ADR Template in create-adr):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh ticket`

Instructions injection (end of file):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh create-ticket`

Skill flow:

  Step 0 — Parameter check
    If a topic was provided as an argument: check if it is too vague (no
    clear deliverable or subject). Vague examples: "improve things", "fix
    the API", "add more features". Clear examples: "add full-text search to
    the docs index page", "fix login timeout after password reset". If vague,
    ask at least one clarifying question ("What specifically needs to change?",
    "What does done look like?") and wait for a more specific description
    before proceeding.
    If no argument: ask for a description and wait. Apply the same vagueness
    check to the user's response — if still vague, ask a clarifying question
    before proceeding to Step 1.

  Step 1 — Gather business context
    Ask 3–5 open questions to understand the business context before
    investigating. Tailor questions to the topic but cover: what problem this
    solves and who is affected; what success looks like from a user or business
    perspective; constraints, deadlines, or dependencies; impact and blocking
    status (for bugs); and whether there is anything the user is uncertain about
    or wants researched. Ask all relevant questions at once and wait for answers
    before proceeding to Step 2.

  Step 2 — Investigate
    Using the topic and business context, run investigation agents in parallel:
      - {documents locator agent}: search {tickets_dir} only — look for
        existing tickets with similar titles, descriptions, or scope. Do not
        search research, plans, or the codebase — tickets are about business
        requirements, not implementation details.
      - {web-search-researcher agent}: spawn when there is uncertainty or a
        need for richer context about any aspect of the topic — business rules,
        domain concepts, competitive landscape, industry standards, external
        technology, or anything the model lacks confidence on. Skip only when
        the topic is entirely self-contained and well-understood.
    Synthesise findings — model knowledge, research results, and prior tickets —
    as the foundation for Step 3.
    If a similar existing ticket is found: surface it and ask whether to proceed
    with a new ticket or exit (to update the existing one manually). Do not
    offer to modify it inline. Wait for the user's choice.

  Step 3 — Propose and refine
    Lead with a structured proposal — do not ask the user to generate
    requirements or acceptance criteria from scratch. Propose them and invite
    challenge. Include:
      - Recommended ticket type with a one-sentence rationale (read valid types
        from the ticket template's type field: story, epic, task, bug, spike)
      - Draft requirements drawn from business context and research
      - Draft acceptance criteria — specific and testable, preferring
        Given/When/Then for story/task; draw on domain knowledge and research
        to make these thorough
      - Assumptions made in the proposal, flagged explicitly
      - Open questions for the user to clarify before drafting
    Present as a structured proposal and wait for the user to validate, push
    back, or refine. Challenge vague or untestable responses — if an acceptance
    criterion is not measurable, ask what a passing test would look like. Do not
    accept weak criteria. Iterate until the proposal is well-specified and agreed.

  Step 4 — Draft ticket
    Load the ticket template. Draft a complete ticket from the agreed proposal.
    Use XXXX as the placeholder ticket number throughout (do NOT call
    ticket-next-number.sh at this step).
    Type-specific content placement:
      - bug reproduction steps, expected/actual behaviour → Requirements section
      - spike research questions, time-box, exit criteria → Requirements section
      - epic initial stories → Requirements section as a list
      Do not rename or add sections beyond those in the ticket template.
    Present the full draft. Continue to challenge during the review loop —
    flag untestable criteria or gaps from research that remain unaddressed.
    Iterate until the user explicitly approves. ticket-next-number.sh is never
    called during this loop.

  Step 5 — Write ticket
    Call ticket-next-number.sh to get the next NNNN. If the script exits
    non-zero (e.g., 9999 overflow), abort immediately and surface the error
    message verbatim — do not proceed.
    Resolve the target path: {tickets_dir}/NNNN-kebab-slug.md (slug is a
    meaningful kebab-case summary of the title).
    Check that the target path does not already exist. If it does, abort with
    a clear message: "Path {path} already exists — another session may have
    written a ticket concurrently. Please re-run /create-ticket."
    Create the tickets directory if it does not exist.
    Substitute XXXX with NNNN throughout the draft and write the file.
    Print a confirmation with the written file path.

Quality guidelines:
  - Never write a file without explicit user approval.
  - Never call ticket-next-number.sh before the user approves the draft.
  - The slug must be a meaningful kebab-case title, not raw input text.
  - The model must contribute its own knowledge and research to the proposal —
    not simply transcribe the user's answers.
  - Restrict {documents locator agent} to {tickets_dir} only. Do not search
    research, plans, or the codebase.
  - Spawn {web-search-researcher agent} whenever there is uncertainty about any
    aspect of the topic — business, domain, competitive, technical, or otherwise.
  - Acceptance criteria must be specific and testable; prefer Given/When/Then
    format. Challenge any criterion that is not measurable before accepting it.
  - Type-specific content belongs in the appropriate template sections; do not
    rename or add sections beyond those in the ticket template.
  - All eight frontmatter fields must be populated in the written file:
    ticket_id (matching NNNN), date, author, type, status (draft), priority
    (medium unless specified otherwise), parent, tags. No field may be left
    with unfilled placeholder text.
  - Summary and Requirements must have substantive content — no [bracketed
    placeholder text] in the final written file.
  - If ticket-next-number.sh exits non-zero, abort and surface the error.

Eval scenarios: [the 12 scenarios listed above in Subphase 2.1]
```

### Success Criteria

#### Automated Verification

- [x] Phase 1 regression suite passes: `bash skills/tickets/scripts/test-ticket-scripts.sh`
- [x] File exists: `skills/tickets/create-ticket/SKILL.md`
- [x] `grep "disable-model-invocation: true" skills/tickets/create-ticket/SKILL.md` matches
- [x] `grep "accelerator:reviewer" skills/tickets/create-ticket/SKILL.md` matches

#### Manual Verification (via `/skill-creator` evals)

- [ ] All 12 eval scenarios pass
- [ ] Bare `/create-ticket` invocation prompts for topic and waits
- [ ] Business context questions are asked before any agent is spawned
- [ ] Documents agent is restricted to `{tickets_dir}` — no codebase or
  research lookups
- [ ] Model proposes type, requirements, and acceptance criteria; pushes back
  on untestable criteria
- [ ] Written file has correct NNNN prefix, kebab-case slug, and all eight
  frontmatter fields populated
- [ ] No unfilled `[...]` placeholder text in a written ticket
- [ ] `ticket-next-number.sh` is not called during the draft/revision loop

---

## Subphase 2.2: `extract-tickets` Skill

### Overview

Author `skills/tickets/extract-tickets/SKILL.md` — a batch extraction skill
that reads source documents, identifies requirements and work items, presents
candidates for user selection, then enriches each selected candidate per-user-choice
(interactive enrichment with model knowledge + web research, similar to
`/create-ticket`, OR accept the source-derived skeleton as-is for a thin
draft). All approved tickets — enriched and thin — are written in one batch
using `ticket-next-number.sh --count N` called exactly once after all
approvals are collected.

The two-tier approval model exists because source documents typically tell
us *what* tickets should exist but rarely give the full business context,
testable acceptance criteria, dependencies, and assumptions a good ticket
needs. Enrichment is the path to that depth; thin drafts are an honest
escape hatch for items the user wants to refine later (or via a future
`/refine-ticket`).

### Eval Scenarios

**Scenario 1 — Bare invocation offers scan-all or specific files**
Input: `/extract-tickets` (no arguments)
Expected: Skill offers to scan all documents in configured directories OR
asks the user to specify files. Waits for choice before doing anything.

**Scenario 2 — Provided paths are read in full immediately**
Input: `/extract-tickets meta/research/codebase/2026-04-08-ticket-management-skills.md`
Expected: Skill reads the provided document completely and proceeds to
analysis without asking the user to supply it.

**Scenario 3 — One analysis agent spawned per document in parallel**
Input: Two document paths provided
Expected: Skill spawns one `{documents analyser agent}` per document, all in
parallel, before presenting findings.

**Scenario 4 — Candidates presented as a selectable numbered list**
Input: Any document with multiple requirements
Expected: Skill presents a numbered list of candidate tickets with brief
titles and one-line descriptions, each noting which source document it came
from. Waits for user to select (e.g., "1,3" or "all"). Does not create
tickets without selection confirmation.

**Scenario 5 — Each draft presented individually with approve/revise/skip;
approve-all-remaining bypasses further confirmation**
Input: User selects 3 items
Expected: All three drafts are pre-generated before the review loop begins.
Each draft is shown with XXXX as the placeholder number. User can approve,
revise, or skip each one individually. If the user selects "revise", the skill
accepts their revision instructions, updates the draft, re-presents it with the
same options, and waits for approval or skip before advancing. Selecting
"approve all remaining" at draft 1 marks drafts 2 and 3 approved without
showing them — `ticket-next-number.sh` is NOT called at this point and the
actual write happens in Step 4, not inline.

**Scenario 6 — ticket-next-number.sh --count N called exactly once after all
approvals**
Input: 3 items approved
Expected: `ticket-next-number.sh --count 3` is called once after all approvals
are collected. Sequential numbers are substituted into the approved drafts in
order. The script is NOT called once per ticket.

**Scenario 7 — Skipped items do not consume ticket numbers**
Input: User approves 2 items, skips 1
Expected: `ticket-next-number.sh --count 2` used; only 2 files written; no
gap in the number sequence.

**Scenario 8 — Source document reference in each written ticket**
Input: Extraction from a named document
Expected: Each written ticket's References section contains a link back to
the source document path. If an item appeared in multiple documents, all
source paths are listed.

**Scenario 9 — Type inference assigns correct type from content**
Input: Document containing a clear bug report with symptom description and
expected behaviour
Expected: The extracted candidate draft has `type: bug` and the Requirements
section is populated with the reproduction steps and expected/actual behaviour
from the source. The type is not defaulted to `story`.

**Scenario 10 — All items skipped results in clean exit, no files written**
Input: User selects candidates but skips all of them during draft review (N=0)
Expected: Skill prints "No tickets approved — nothing written." and exits
cleanly. `ticket-next-number.sh` is NOT called. No files are written.

**Scenario 11 — Zero candidates found exits cleanly**
Input: Document path whose content is structural/navigational only (table of
contents, meeting agenda with no actionable items)
Expected: Skill presents an empty candidate list, informs the user that no
actionable items were found, and exits cleanly without writing any files.

**Scenario 12 — Slug collision aborts before number allocation**
Input: User approves a draft whose kebab-case slug already exists in the
tickets directory (e.g. an existing `0042-add-search.md` and a new draft
that would also slug to `add-search`)
Expected: Skill detects the slug collision via a glob check
(`{tickets_dir}/[0-9][0-9][0-9][0-9]-{slug}.md`) before calling
`ticket-next-number.sh`, reports which slugs collide and which existing
files they match, and aborts. No number is allocated, no file is written.

**Scenario 13 — 9999 overflow surfaces verbatim, no files written**
Input: Approvals would push the next number past 9999 (e.g. existing
`9998-foo.md` plus three approvals)
Expected: `ticket-next-number.sh --count N` exits non-zero with the
"ticket number space exhausted" error. Skill aborts immediately, surfaces
the script's stderr verbatim, and writes no files — even though the script
emits partial numbers on stdout before exiting.

**Scenario 14 — Mid-batch write error reports allocated/written/missing**
Input: 3 approvals; second `Write` fails (e.g. permission error)
Expected: Skill reports which numbers were allocated (all N), which files
were written successfully, and which were not, so the user can manually
write the missing files using the pre-assigned numbers. Skill does not
silently retry, and does not call `ticket-next-number.sh` again.

**Scenario 15 — Deduplication preserves all source paths**
Input: Two documents both describe the same work item with similar wording
Expected: Skill presents one merged candidate. The candidate's source line
lists both document paths. The written ticket's `References` section
includes both source paths, one per line.

**Scenario 16 — "Approve all remaining" mid-loop, after some skips**
Input: 5 candidates selected. User approves draft 1, skips draft 2, then
chooses "approve all remaining" on draft 3
Expected: Drafts 3, 4, 5 are marked approved without being shown. Draft 2
stays skipped. N=4 (drafts 1,3,4,5). `ticket-next-number.sh` is not called
until Step 4.

**Scenario 17 — Iterative revise stays on the same draft**
Input: User chooses "revise" on draft 1 with instructions, sees the updated
draft, chooses "revise" again with further instructions, then "approve"
Expected: Skill re-presents draft 1 (same draft number) after each revision
and only advances to draft 2 once the user types "approve" or "skip".
"revise" never advances the loop.

**Scenario 18 — Type inference: spike**
Input: Source paragraph framed as an open question with a time-box
("Investigate over 2 days whether moving to gRPC reduces p99 latency by
20%; report findings.")
Expected: Extracted draft has `type: spike`. The Requirements section
contains the research question, the time-box, and the exit criterion.

**Scenario 19 — Type inference: epic**
Input: Source describing a broad multi-deliverable theme ("Modernise the
checkout funnel: address mobile UX, payment retries, and analytics
instrumentation.")
Expected: Extracted draft has `type: epic`. The Requirements section
contains the initial story decomposition as a list.

**Scenario 20 — Ambiguous content defaults to story**
Input: Source describing a vague piece of work without bug symptoms,
research questions, or epic-scale scope ("Improve the dashboard's empty
state.")
Expected: Extracted draft has `type: story`, not bug, task, spike, or
epic. The default is reached because none of the more specific type
heuristics match.

**Scenario 21 — All frontmatter fields populated, no placeholder text**
Input: Any approved draft written to disk
Expected: The written file's frontmatter contains every field defined in
the ticket template (`ticket_id` matching the assigned NNNN, `date`,
`author`, `type`, `status: draft`, `priority`, `parent`, `tags`). No
field contains placeholder text like `[author]`, `NNNN`, or
`YYYY-MM-DD…`. `parent` is an empty string unless the source establishes
a parent. `tags` is a YAML array, possibly empty.

**Scenario 22 — Structural content is not extracted**
Input: Document containing actionable requirements interleaved with table
of contents entries, navigation headings, and an agenda block listing
"Next steps: discuss" with no concrete outcome
Expected: Candidate list contains only the actionable items; the TOC
entries, navigation headings, and the bare agenda line are not surfaced
as candidate tickets.

**Scenario 23 — Non-existent file path is reported, not silently skipped**
Input: `/extract-tickets meta/research/codebase/2026-04-08-real.md meta/research/codebase/does-not-exist.md`
Expected: Skill reports that `meta/research/codebase/does-not-exist.md` does not
exist and asks the user to correct or remove the path. It does not
silently proceed with only the first file, and does not produce an empty
candidate list with no explanation.

**Scenario 24 — Numbers assigned in presentation order, not approval order**
Input: 5 candidates presented in order [A, B, C, D, E]. User responds in
the loop with: A=approve, B=skip, C=revise then approve, D=skip,
E=approve
Expected: Approved set is {A, C, E}. After `ticket-next-number.sh --count 3`
returns (e.g.) `0042 0043 0044`, A receives 0042, C receives 0043, E
receives 0044. The numbers map to the original presentation order, not
the order in which the user finished approving.

**Scenario 25 — Faithfulness: inferred extensions flagged as assumptions**
Input: Source describes a feature outline but does not state acceptance
criteria
Expected: Draft includes proposed acceptance criteria derived from the
outline, but the draft explicitly flags them as assumptions (e.g. an
"Assumptions" note or an inline marker) so the user can validate them
during the review loop. Skill does not silently add criteria as if they
were stated in the source.

**Scenario 26 — Enrich path: Q&A + research → enriched draft**
Input: User selects "enrich" on a candidate whose source content is thin
on business context (e.g. "add CSV export to the report page" with no
detail on who needs it or what fields are required)
Expected: Skill asks 1–3 focused business-context questions tailored to
this candidate, spawns `{web-search-researcher agent}` if uncertainty
warrants it, then re-presents an updated draft containing material
absent from the source — testable acceptance criteria (preferring
Given/When/Then), dependencies, and an explicit `Assumptions` block
listing anything still inferred. Skill iterates until the user
explicitly approves; a vague acceptance criterion ("it should work
correctly") is challenged before being accepted.

**Scenario 27 — Accept-as-is: thin draft carries non-enrichment note**
Input: User selects "accept as-is" on a candidate
Expected: Final draft is the source-derived skeleton from 3.1 with no
further questions asked. The `Assumptions` section contains a note that
the ticket was extracted without interactive enrichment and that
acceptance criteria, dependencies, and type may need refinement before
promoting from `draft` to `ready`. The candidate is marked approved
(thin) and added to the batch.

**Scenario 28 — Mid-loop mode switch: enrich some, then accept rest as-is**
Input: 5 candidates. User enriches candidates 1 and 2 (full Q&A +
research), then chooses "accept remaining as-is" on candidate 3
Expected: Candidates 3, 4, 5 are written as thin drafts with the
non-enrichment `Assumptions` note. No further questions are asked.
Enriched and thin drafts coexist in the same batch; the directory
ends with two enriched and three thin tickets, each numbered in
presentation order. `ticket-next-number.sh --count 5` is called once.

**Scenario 29 — Skip during enrichment honours interrupt immediately**
Input: User chooses "enrich" on a candidate, answers the questions,
then partway through reviewing the proposed draft chooses "skip"
Expected: Skill exits enrichment for this candidate immediately,
excludes it from the batch, and advances to the next candidate. The
candidate does not become approved by virtue of having been partially
enriched, and `ticket-next-number.sh` is not called.

**Scenario 30 — Numbering presentation-order across enriched + thin mix**
Input: 5 candidates presented as [A, B, C, D, E]. User enriches A and
D, accepts B and C as-is, skips E. Enrichment of D takes longer than
A's, so A is approved first chronologically and D last
Expected: Approved set is {A, B, C, D} in presentation order. After
`ticket-next-number.sh --count 4` returns (e.g.) `0042 0043 0044 0045`,
A receives 0042, B receives 0043, C receives 0044, D receives 0045.
Numbers map to original presentation order regardless of which were
enriched vs accepted, or in which order they were approved.

### Changes Required

#### 1. Create `skills/tickets/extract-tickets/SKILL.md`

**File**: `skills/tickets/extract-tickets/SKILL.md`

Invoke `/skill-creator:skill-creator` with this specification:

```
Skill name: extract-tickets
Category: tickets
Model after: skills/decisions/extract-adrs/SKILL.md (follow its structure
exactly, adapting for tickets instead of ADRs)

Frontmatter:
  name: extract-tickets
  description: Extract tickets from existing documents (specs, PRDs, research,
  meeting notes). Use when requirements exist in documents and need to be
  captured as structured tickets in meta/tickets/.
  argument-hint: "[document paths...] or leave empty to scan all"
  disable-model-invocation: true
  allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)

Configuration preamble (bang-executed in this order):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh extract-tickets`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

Agent fallback (if no Agent Names section above):
  accelerator:reviewer, accelerator:codebase-locator, accelerator:codebase-analyser,
  accelerator:codebase-pattern-finder, accelerator:documents-locator,
  accelerator:documents-analyser, accelerator:web-search-researcher

Path injections:
  **Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`
  **Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
  **Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`

Template section (static, in skill body — same placement as in create-adr):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh ticket`

Instructions injection (end of file):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh extract-tickets`

Skill flow:

  Step 1 — Identify source documents
    If document paths were provided as arguments: read them fully.
    If no paths provided: ask whether to scan all configured directories
    (research + plans) or specify files manually. If scan-all, spawn
    {documents locator agent} to discover candidates; present the list
    and wait for the user to select which documents to analyse.

  Step 2 — Analyse documents
    Spawn one {documents analyser agent} per selected document, all in
    parallel. Each agent looks for: requirements, acceptance criteria,
    user stories, feature descriptions, bug reports, and work items.
    Collect findings. Deduplicate items that appear in multiple documents —
    for each deduplicated item, record all source documents it came from.
    If no actionable items are found across all documents: inform the user
    ("No actionable items found in the provided documents") and exit cleanly.
    Otherwise: present the full candidate list as a numbered list. Each entry
    shows the brief title, one-line description, and the source document(s)
    it came from. Wait for user to select items ("1,3,5", "all", or "none").
    If the user selects "none": exit cleanly without writing any files.

  Step 3 — Enrich and approve (per candidate)
    Do NOT pre-generate drafts for the entire batch — enrichment can change
    a draft significantly, so generation happens per candidate inside this
    loop. For each selected candidate, in original presented order:

      3.1 Build the source-derived skeleton:
        Use XXXX as the placeholder ticket number. Read valid ticket types
        from the ticket template's frontmatter and infer the type:
          - clear bug reports with symptoms → bug
          - open-ended investigation with questions → spike
          - broad multi-deliverable themes → epic
          - specific deliverables → story
          - one-off tasks → task
          Default to story for ambiguous items.
        Type-specific content placement:
          - bug: reproduction, expected/actual → Requirements
          - spike: research questions, time-box, exit criteria → Requirements
          - epic: initial story decomposition → Requirements as a list
          - Do not rename or add sections beyond the ticket template.
        Source-derived content stays faithful to what the source says;
        anything inferred (type, implied criteria) goes in an Assumptions
        block flagged as such. References lists all source documents.

      3.2 Present the skeleton with four options:
        enrich / accept as-is / skip / accept remaining as-is
        Wait for the user's choice.

      3.3 enrich (interactive):
        Ask 1–3 focused business-context questions tailored to this
        candidate (fewer than create-ticket's 3–5 because the source
        provides some context). Cover whichever are not already clear:
        pain point + who's affected, desired outcome, constraints/deps,
        bug impact/blocker, anything to research.
        Spawn {web-search-researcher agent} when there is uncertainty
        about any aspect — domain, business, competitive, technical.
        Skip web research only when the candidate is self-contained.
        Update the draft from source + answers + model knowledge + research.
        Re-present as a structured proposal with: type + rationale,
        requirements, specific testable acceptance criteria
        (Given/When/Then for story/task), dependencies, Assumptions block,
        open questions.
        Challenge weak/untestable acceptance criteria — when not measurable,
        ask what a passing test would look like and reformulate together.
        Iterate until the user explicitly approves. Mark approved (enriched).
        Honour mid-loop interrupts: skip / accept as-is.

      3.4 accept as-is (thin):
        Take the 3.1 skeleton as the final draft. Append an Assumptions
        note: "Extracted from source documents without interactive
        enrichment. Acceptance criteria, dependencies, and type may need
        refinement before promoting from draft to ready." Mark approved
        (thin).

      3.5 skip: exclude from batch. Skipped candidates never become
        approved later, even via accept-remaining-as-is.

      3.6 accept remaining as-is: mark every remaining unreviewed candidate
        approved (thin) using the same skeleton + Assumptions note as 3.4.
        Already-skipped stay skipped. Jump to Step 4.

    ticket-next-number.sh is NOT called at any point in Step 3, regardless
    of which option the user picks. Writing happens exclusively in Step 4.

  Step 4 — Write tickets
    Count approved (non-skipped) items: N.
    If N is 0: print "No tickets approved — nothing written." and exit cleanly.
    Do not call ticket-next-number.sh.
    Otherwise:
    1. Compute all N target slugs from the approved draft titles.
    2. Create the tickets directory if it does not exist.
    3. Verify that none of the N target paths already exist in the tickets
       directory. If any collide, report which paths exist, abort without
       calling ticket-next-number.sh, and ask the user to resolve the
       collision before re-running.
    4. Call ticket-next-number.sh --count N exactly once. If the script exits
       non-zero (e.g., 9999 overflow), abort immediately and surface the error
       message verbatim — do not write any files.
    5. Substitute the N sequential numbers into approved drafts in order (first
       approved gets the first number, etc.).
    6. Write all N ticket files. Each ticket's References section must include
       all source documents the item was extracted from.
    If a write error occurs mid-batch: report which numbers were allocated,
    which files were written successfully, and which were not — so the user can
    manually write the missing files with their pre-assigned numbers.
    Print a summary table: number | title | file path.

Quality guidelines:
  - Never call ticket-next-number.sh before all approvals are collected.
  - Never call ticket-next-number.sh when N=0.
  - If ticket-next-number.sh exits non-zero, abort immediately and surface
    the script's error output verbatim.
  - Every written ticket MUST include all source document paths in its
    References section. For deduplicated items, list all contributing sources.
  - Do not extract structural/navigational content (table of contents entries,
    section headings with no requirements content) as candidate tickets.
  - Ticket type inference must use types read from the ticket template
    frontmatter, not a hardcoded list. Default to story for ambiguous items.
  - All eight frontmatter fields must be populated in every written ticket.
  - "Accept remaining as-is" only marks unreviewed candidates as approved
    (thin) — it does not resurrect skipped candidates, and writing always
    happens exclusively in Step 4, after the single ticket-next-number.sh call.
  - Source-derived content stays faithful to the source. Extensions
    (proposed acceptance criteria, inferred type, suggested dependencies,
    business-context inferences) are model/research contributions and must
    be flagged in the draft's Assumptions block so the user can validate
    or challenge them. Do not silently invent requirements.
  - Enrichment (3.3) is per candidate. Do not pre-generate enriched drafts
    for the whole batch — enrichment depends on user answers and research,
    both of which differ per candidate.
  - Web research is a first-class step inside enrichment — spawn it
    whenever there is uncertainty about any aspect of the candidate.
  - Thin and enriched drafts coexist in the same tickets directory. Thin
    drafts must carry the non-enrichment Assumptions note so they can be
    identified for follow-up before promotion from draft to ready.
  - Acceptance criteria in enriched drafts must be specific and testable
    (Given/When/Then for story/task). Challenge any non-measurable
    criterion before accepting it.

Eval scenarios: [the 30 scenarios listed above in Subphase 2.2]
```

### Success Criteria

#### Automated Verification

- [x] Phase 1 regression suite passes: `bash skills/tickets/scripts/test-ticket-scripts.sh`
- [x] File exists: `skills/tickets/extract-tickets/SKILL.md`
- [x] `grep "disable-model-invocation: true" skills/tickets/extract-tickets/SKILL.md` matches
- [x] `grep "accelerator:reviewer" skills/tickets/extract-tickets/SKILL.md` matches

#### Manual Verification (via `/skill-creator` evals)

- [ ] All 30 eval scenarios pass
- [ ] Batch numbering uses `--count N` not N individual calls
- [ ] Source document reference appears in each written ticket
- [ ] Skipped items do not consume ticket numbers
- [ ] All-skip (N=0) exits cleanly without calling `ticket-next-number.sh`
- [ ] Slug collision aborts before number allocation; 9999 overflow
  surfaces verbatim and writes nothing
- [ ] Numbers map to approved drafts in original presented order, across
  enriched and thin drafts alike
- [ ] Inferred extensions (acceptance criteria, type) are flagged as
  assumptions in the draft, not silently added
- [ ] Non-existent input paths are reported, not silently skipped
- [ ] Enrichment loop asks 1–3 focused questions, spawns
  `{web-search-researcher agent}` on uncertainty, and challenges
  untestable acceptance criteria before accepting them
- [ ] Thin (accept-as-is) drafts carry a non-enrichment `Assumptions`
  note identifying them as needing refinement before promotion
- [ ] Enriched and thin drafts coexist in the same batch and same
  tickets directory
- [ ] Written tickets contain all eight frontmatter fields (`ticket_id`, `date`,
  `author`, `type`, `status`, `priority`, `parent`, `tags`) with no unfilled
  placeholder text

---

## Subphase 2.3: Integration Verification

### Overview

Confirm both skills integrate cleanly with the Phase 1 foundation, load
correctly via the plugin, and are callable without errors.

### Changes Required

None — verification only.

### Verification Steps

1. Run Phase 1 regression suite: `bash skills/tickets/scripts/test-ticket-scripts.sh`
2. Confirm `allowed-tools` is inline (no block scalar) in both SKILL.md files
3. Confirm both SKILL.md files contain `disable-model-invocation: true`
4. Confirm agent fallback block uses `accelerator:` prefix in both files
5. Confirm plugin registration already covers both — `"./skills/tickets/"` in
   `plugin.json` scans all subdirectories, so no further changes are needed
6. Invoke each skill bare in a Claude Code session and confirm it prompts
   correctly without errors

### Success Criteria

#### Automated Verification

- [x] `bash skills/tickets/scripts/test-ticket-scripts.sh` exits 0, "All tests passed!"
- [x] `grep -r "disable-model-invocation: true" skills/tickets/` returns two matching files
- [x] `grep -r "allowed-tools" skills/tickets/` shows only inline `config-*` and `tickets/scripts/*` patterns (no block scalars)
- [x] `grep "accelerator:reviewer" skills/tickets/create-ticket/SKILL.md` matches
- [x] `grep "accelerator:reviewer" skills/tickets/extract-tickets/SKILL.md` matches

#### Manual Verification

- [ ] `/create-ticket` is available in a Claude Code session and prompts for a
  topic when invoked with no arguments
- [ ] `/extract-tickets` is available and offers scan-all or specific files
  when invoked with no arguments
- [ ] No unresolved `{agent placeholder}` tokens visible in either skill

---

## Testing Strategy

### TDD Order Per Subphase

1. Eval scenarios in this plan are the specification — written before
   implementation
2. `/skill-creator:skill-creator` authors the SKILL.md from the specification
3. Built-in evals run against the authored SKILL.md
4. Iterate on the SKILL.md until all evals pass
5. Run `bash skills/tickets/scripts/test-ticket-scripts.sh` as regression guard

### What Cannot Be Automated

SKILL.md files are markdown prompts, not executable code. The skill-creator
eval runner is the verification mechanism. There is no CI-runnable test for
SKILL.md prompt correctness. The `allowed-tools` boundary (whether the skill
actually avoids calling scripts outside its declared scope) must be verified
manually during the integration step by running each skill through a happy-path
flow and confirming no "tool not permitted" errors occur.

## References

- Research: `meta/research/codebase/2026-04-08-ticket-management-skills.md` (§2, §3,
  §5.1, §5.2, §6, §10)
- Pattern for interactive creation: `skills/decisions/create-adr/SKILL.md`
- Pattern for batch extraction: `skills/decisions/extract-adrs/SKILL.md`
- Numbering script: `skills/tickets/scripts/ticket-next-number.sh`
- Phase 1 regression suite: `skills/tickets/scripts/test-ticket-scripts.sh`
