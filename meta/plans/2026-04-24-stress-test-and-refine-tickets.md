---
date: "2026-04-24T00:00:00+01:00"
type: plan
skill: create-plan
ticket: ""
status: complete
---

# Stress-Test and Refine Tickets — Phase 6 (Interactive Quality)

## Overview

Implement the two interactive-quality ticket skills for the Accelerator
plugin: `stress-test-ticket` (adversarial examination, modelled on
`stress-test-plan`) and `refine-ticket` (a novel menu-driven skill for
decomposition, enrichment, sharpening, sizing, and linking). Both skills
are authored TDD-style — approach evals are defined in this plan as the
specification, then `/skill-creator:skill-creator` is invoked to author
and iterate each SKILL.md until all evals pass. Evaluations and
benchmarks are persisted alongside each SKILL.md as long-term regression
evidence.

## Current State Analysis

Phases 1–5 of the ticket management initiative are complete:

- `templates/ticket.md` — nine-field frontmatter (`ticket_id`, `title`,
  `date`, `author`, `type`, `status`, `priority`, `parent`, `tags`) and
  fixed body sections (`Summary`, `Context`, `Requirements`,
  `Acceptance Criteria`, `Open Questions`, `Dependencies`, `Assumptions`,
  `Technical Notes`, `Drafting Notes`, `References`)
- `skills/tickets/scripts/` — `ticket-next-number.sh`,
  `ticket-read-status.sh`, `ticket-read-field.sh`,
  `ticket-update-tags.sh`, `ticket-template-field-hints.sh`, covered by
  `test-ticket-scripts.sh`
- `skills/tickets/{create-ticket,extract-tickets,list-tickets,
  update-ticket,review-ticket}/SKILL.md` — all five shipped, each with
  committed `evals/evals.json` and `evals/benchmark.json`
- `skills/review/lenses/{completeness,testability,clarity,scope,
  dependency}-lens/SKILL.md` — all five ticket lenses shipped
- `skills/review/output-formats/ticket-review-output-format/SKILL.md`
- `scripts/config-read-review.sh` — `ticket` mode shipped in Phase 4
- `tasks/test.py` (`mise run test`) runs: `test-config.sh`,
  `test-adr-scripts.sh`, `test-ticket-scripts.sh`,
  `test-lens-structure.sh`, `test-boundary-evals.sh`

The `skills/tickets/` directory contains the five shipped skill
subdirectories plus `scripts/`. No `stress-test-ticket/` or
`refine-ticket/` subdirectory exists yet.

The `stress-test-plan` precedent is complete:

- `skills/planning/stress-test-plan/SKILL.md` — 230-line skill, the
  direct structural model for `stress-test-ticket`. Uses
  `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` (no
  script dependencies), conversational depth-first interrogation, and
  optional targeted edits via the `Edit` tool

The `/skill-creator:skill-creator` workflow is established — Phases 2–5
all follow it: evals are written first as the specification, the skill
is invoked with the spec, it iterates the SKILL.md until all evals pass,
and `evals.json` / `benchmark.json` are committed alongside the skill.

### Key Discoveries

- `stress-test-plan/SKILL.md` is the only skill in the plugin that uses
  `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` with no
  category scripts — it relies entirely on `Read`, `Edit`, and optional
  `Agent` spawning. `stress-test-ticket` will follow the same pattern.
- `create-ticket/SKILL.md` defers `ticket-next-number.sh` to the write
  step to avoid consuming numbers on abandoned sessions. `refine-ticket`
  cannot defer: decomposition produces multiple committed children, so
  numbering is consumed eagerly per child at write time. This is the
  same pattern as existing content that is persisted mid-flow.
- `list-tickets/SKILL.md` renders hierarchy (parent → indented children)
  by reading each ticket's `parent` field, but does not currently pin
  the exact indentation characters (§"Hierarchy Rendering" only says
  children "appear indented beneath their parent"). For `refine-ticket`
  to legitimately match `/list-tickets`' output, `list-tickets` must
  first commit to a literal format. This plan therefore includes
  Subphase 6.0 — a small pinning change to `list-tickets` — before
  refine-ticket references the format. Rendering logic is specified
  inline in both SKILL.md files (rather than extracted into a shared
  script) because we want to revisit extraction only once a third
  consumer appears; the duplication cost is acknowledged explicitly.
- `update-ticket/SKILL.md` canonicalises the `parent` field to a
  zero-padded four-digit string (e.g. `1` → `"0001"`). `refine-ticket`
  must apply the same canonicalisation when populating `parent` on child
  tickets so hierarchy rendering works consistently.
- Child ticket type is inferred from parent type: epic → story, story →
  task. Bug and spike decomposition is not expected (these don't usually
  decompose); the skill can still offer decomposition for them but
  should challenge whether it is appropriate.
- `create-ticket` Step 2 offers an option to link the new ticket as a
  child of an existing one. This shows how parent linking is expressed
  in prose. `refine-ticket` will inject the same pattern during
  decompose, but write N children at once.
- Lens names use singular form: `completeness-lens`, `testability-lens`,
  `clarity-lens`, `scope-lens`, `dependency-lens` (not `dependencies-lens`).
  Relevant when `refine-ticket`'s link operation references lens
  terminology.
- `stress-test-plan` explicitly positions itself after `/review-plan` in
  the lifecycle ("complementary — review gives broad coverage,
  stress-test goes deep"). `stress-test-ticket` should adopt the same
  positioning relative to `/review-ticket`.
- No new config keys, paths, or template changes are required. All
  existing infrastructure covers both skills.
- `tasks/test.py` does not currently test skill content — only scripts
  and lens structure. Skill content is covered by `evals/benchmark.json`
  at authoring time; this plan does not add CI-level eval running (that
  is a separate, future concern).

## Desired End State

After this plan is complete:

1. `skills/tickets/stress-test-ticket/SKILL.md` exists and is callable
   as `/stress-test-ticket`. It conducts a depth-first, adversarial,
   one-question-at-a-time conversation about a ticket, produces a
   structured findings summary, and optionally applies targeted edits.
2. `skills/tickets/stress-test-ticket/evals/evals.json` and
   `benchmark.json` are committed, with all evals at 100% pass rate in
   the committed benchmark.
3. `skills/tickets/refine-ticket/SKILL.md` exists and is callable as
   `/refine-ticket`. It reads the target ticket, spawns codebase agents
   in parallel, presents a menu of five operations (decompose, enrich,
   sharpen, size, link), applies the selected operations interactively,
   and displays the updated hierarchy after decomposition.
4. `skills/tickets/refine-ticket/evals/evals.json` and `benchmark.json`
   are committed, with all evals at 100% pass rate.
5. `mise run test` passes with no regressions in any Phase 1–5 script
   or lens-structure check.
6. No changes are made to `templates/ticket.md`, any
   `skills/tickets/scripts/*.sh`, any Phase 4–5 lens, or any
   configuration script.

## What We're NOT Doing

- Not introducing any new shell script in `skills/tickets/scripts/`.
  Decomposition reuses `ticket-next-number.sh`; field reads reuse
  `ticket-read-field.sh` and `ticket-template-field-hints.sh`;
  hierarchy rendering is inline prose in `refine-ticket/SKILL.md`
  matching the pattern established by `list-tickets`.
- Not adding any new frontmatter field to `templates/ticket.md`
  (including no `size`, `estimate`, `complexity`, or `refined_at`
  fields). Sizing output appends to `Technical Notes` or the body — not
  the frontmatter.
- Not adding any new config key (no `refine_*`, no `stress_test_*`).
- Not adding full CI-level eval running (i.e., no re-running of the
  LLM-graded eval harness in CI). Evals are run at authoring time by
  `/skill-creator:skill-creator` and their `benchmark.json` is the
  committed regression artifact. Subphase 6C DOES add a lightweight
  structural validation of every `evals.json` + `benchmark.json` pair
  to `mise run test` — this catches stale/truncated benchmarks and
  missing scenarios, but does not re-score the evals.
- Not automatically invoking `/review-ticket` at the end of
  `refine-ticket`. The skill offers to invoke it but does not call it
  itself.
- Not rolling back child tickets if the user abandons a `refine-ticket`
  session mid-way. Children already written stay on disk. Numbering is
  eagerly consumed for children, mirroring how committed tickets
  behave elsewhere.
- Not changing `stress-test-plan/SKILL.md` or any other Phase 1–5
  artifact. `stress-test-ticket` is a sibling, not a refactor.
- Not restricting `refine-ticket` to specific ticket types. It can
  operate on any ticket; the decompose operation adapts child type based
  on parent type (epic→story, story→task, other→story by default).
- Not modifying the target ticket's `tags` field. Tag changes are
  `/update-ticket`'s concern. Children of decompose inherit tags
  verbatim from the parent as part of their initial frontmatter.
- Not offering a mid-flow "delete child" path. Once a child has been
  written to disk during decompose, removing it is the user's
  responsibility via `jj restore` or a follow-up `/update-ticket`.
- Not supporting concurrent `/refine-ticket` invocations.
  `ticket-next-number.sh` has no file locking, so two sessions
  running decompose in parallel can allocate overlapping numbers.
  The skill's filename-collision check immediately before each
  child write catches the trivial case; users with two terminals
  should not run decompose in both simultaneously.
- Not providing an automated rollback. The skill's "no rollback"
  posture is safe because the workspace is under jj/git — users can
  `jj restore <file>` any child written, or `jj undo` to revert the
  most recent change. The skill surfaces these commands in its abort
  ledger (Scenario 12).

## Implementation Approach

Each subphase follows the same TDD loop established in Phases 2–5:

1. Approach evals in this plan are the specification (written first)
2. Invoke `/skill-creator:skill-creator` with the specification + evals
3. `/skill-creator:skill-creator` runs evals against the SKILL.md draft
4. Iterate SKILL.md prose until all evals pass
5. Commit `evals/evals.json` and `evals/benchmark.json` alongside the
   SKILL.md
6. Run `mise run test` before marking the subphase done

Subphase 6.0 (`list-tickets` hierarchy format pin) ships first so
`refine-ticket` has a literal format to reference. Subphase 6A
(`stress-test-ticket`) ships next because it is simpler (one
interaction pattern, no child creation, no mandatory agents) and
establishes the conversational-skill eval pattern for Phase 6. Subphase
6B (`refine-ticket`) builds on that familiarity with a richer
menu-driven flow.

Brace-wrapped tokens like `{codebase locator agent}` that appear in the
embedded SKILL.md specifications below are resolved at skill-load time
via `config-read-agents.sh`; angle-bracket placeholders like
`<rationale>` are literal spec placeholders the skill-creator fills in.

The `argument-hint` value `[ticket number or path]` used by both new
skills is a deliberate convention extension — sibling skills currently
use four different forms (`[path to ticket file]`, `[topic or
description]`, `[filter description]`, `[ticket-ref] [field-op...]`).
The new dual-shape phrasing matches the actual accepted argument set
(numeric shorthand resolved against the tickets directory, OR an
explicit path) and is intended to be propagated to the other ticket
skills in a follow-up convergence pass — that propagation is OUT of
scope for this plan.

---

## Subphase 6.0: `list-tickets` Hierarchy Format Pin

### Overview

Update `skills/tickets/list-tickets/SKILL.md` §"Hierarchy Rendering" to
commit to a literal, reproducible tree rendering so that
`refine-ticket`'s post-decompose tree display can reference a single
authoritative format. The current prose only says "appear indented
beneath their parent" — which is not verifiable from output.

### Changes Required

#### 1. Pin the hierarchy tree format in `list-tickets`

**File**: `skills/tickets/list-tickets/SKILL.md`

In §"Hierarchy Rendering", replace the second bullet ("Tickets whose
`parent` points to a ticket in the current result set appear indented
beneath their parent.") with a literal rendering specification. The
new bullet text reads:

> Tickets whose `parent` points to a ticket in the current result
> set are rendered as children. Each parent→children group prints
> as a tree using Unicode box-drawing characters. Children use
> `├── ` for all entries except the last in the group, which uses
> `└── `. Indent two spaces per depth level. Example:

The new bullet is followed immediately by a fenced code block
showing the canonical rendering (the LITERAL output users will
see). Wrap the fence in HTML comment markers so the cross-SKILL.md
drift check (`scripts/test-hierarchy-format.sh`) can extract it
deterministically:

```
<!-- canonical-tree-fence -->
NNNN — parent title (type: <type>, status: <status>)
  ├── NNNN — child 1 title (type: <type>, status: <status>)
  ├── NNNN — child 2 title (type: <type>, status: <status>)
  └── NNNN — last child title (type: <type>, status: <status>)
<!-- /canonical-tree-fence -->
```

Use the SAME placeholder content (`NNNN — parent title` etc.) in
both `list-tickets/SKILL.md` and `refine-ticket/SKILL.md` Step 5,
so the drift check compares like-for-like rather than literal
example IDs. Concrete-ID examples (e.g. `0042 — User Auth Rework`)
belong only in eval `expected_output` strings (Scenario 13), not
in the SKILL.md files themselves.

After the code block, append a second prose sentence:

> No ASCII fallback is attempted; terminals without Unicode
> support will render mojibake. Users on such terminals can
> re-display the hierarchy via /list-tickets.

This structure (prose bullet → fenced example → prose caveat)
mirrors how `refine-ticket/SKILL.md` Step 5 and `evals.json`
Scenario 13 present the same tree, eliminating any literal-vs-
illustrative ambiguity.

### Success Criteria

#### Automated Verification:
- [x] `grep -F '├── ' skills/tickets/list-tickets/SKILL.md` returns a match
- [x] `grep -F '└── ' skills/tickets/list-tickets/SKILL.md` returns a match
- [x] The phrase "Indent two spaces per depth level" appears verbatim
  in `skills/tickets/list-tickets/SKILL.md`
- [x] The phrase "Unicode box-drawing characters" appears verbatim
  in `skills/tickets/list-tickets/SKILL.md`
- [x] HTML markers `<!-- canonical-tree-fence -->` and
  `<!-- /canonical-tree-fence -->` both appear in
  `skills/tickets/list-tickets/SKILL.md`
- [ ] `test-hierarchy-format.sh` (added in Subphase 6C §3) passes —
  asserts byte-equality between the marker-bracketed fence in
  `list-tickets/SKILL.md` and the marker-bracketed fence in
  `refine-ticket/SKILL.md` Step 5
- [x] `mise run test` passes

#### Manual Verification:
- [ ] `/list-tickets hierarchy` on a fixture with at least one epic
  containing multiple children renders the documented tree

---

## Subphase 6A: `stress-test-ticket` Skill

### Overview

Author `skills/tickets/stress-test-ticket/SKILL.md` — an interactive,
adversarial skill that grills the user about a ticket's scope, assumptions,
acceptance criteria, edge cases, and dependencies. Modelled on
`stress-test-plan/SKILL.md`. Optional codebase agents for verifying
technical assumptions against the code. Produces a structured findings
summary; optionally applies targeted edits to the ticket via `Edit`.

### Eval Scenarios

These scenarios are the TDD specification. Define them before authoring
the SKILL.md and pass them to `/skill-creator:skill-creator`.

**Scenario index** (stress-test-ticket has 15 evals; cross-references
in the SKILL.md flow resolve here):
- Invocation surface: 1, 2, 14
- Conversational discipline: 3, 4
- Interrogation quality: 5, 6, 7, 8, 9
- Termination and summary: 10, 11
- Edit behaviour: 12, 13, 15

**Scenario 1 — Bare invocation prompts for ticket identifier**
Input: `/stress-test-ticket` (no arguments)
Expected: Skill asks for a ticket identifier — accepting either a
path (e.g. `meta/tickets/0042-user-auth.md`) or a bare ticket number
(e.g. `0042` or `42`, resolved against the configured tickets
directory). Prompt also suggests running `/list-tickets` to discover
available tickets. Skill waits; does not read or edit any file; does
not start questioning.

**Scenario 2 — Ticket path resolution reads file fully before questioning**
Input: `/stress-test-ticket @meta/tickets/NNNN-fixture.md`
Expected: Skill reads the entire ticket file before asking any question.
The first question references specific content from the ticket (an
acceptance criterion, a specific requirement, a named assumption).

**Scenario 3 — One question at a time, no bulleted lists of concerns**
Input: Ticket provided with multiple weak acceptance criteria. Prompt
follows the create-ticket multi-turn pattern:
`[Step 1: invoke /stress-test-ticket @files/weak-acs/ticket.md]`
Expected: Skill's single response contains exactly one question (or a
tightly related cluster on the same point) — NOT a bulleted list
enumerating all issues, and NOT multiple unrelated questions. The
response ends in a question awaiting user input (no "then I'll ask X
next" pre-commitment).

**Scenario 4 — Depth-first: follows the thread to a conclusion**
Input: Prompt simulates the first exchange inline —
`[Step 1: invoke /stress-test-ticket @files/weak-acs/ticket.md]`
`[Skill asked: "What does 'gracefully' mean in AC2?"]`
`[User responds: "I think it should log and keep running."]`
Expected: Skill's next response asks a follow-up on the SAME branch
(e.g. "What should be logged, and who reads that log?") rather than
switching to a new topic (scope, dependencies, edge cases). The
branch is only abandoned once the user has given a concrete answer.

**Scenario 5 — Challenges vague acceptance criteria**
Input: Ticket with an acceptance criterion like "it should handle errors
gracefully"
Expected: Skill challenges it concretely — asks what "gracefully" means,
what a failing path looks like, what a passing test would assert.

**Scenario 6 — Probes over-scoped tickets**
Input: Story with five independent acceptance criteria covering
unrelated concerns
Expected: Skill surfaces the scope concern and asks whether the ticket
should be decomposed or whether some criteria belong in separate tickets.

**Scenario 7 — Probes edge cases beyond the happy path**
Input: Ticket with happy-path-only acceptance criteria
Expected: Skill asks about specific edge cases relevant to the ticket's
content (empty data, concurrent access, partial failures, malformed
input, timeouts — whichever apply). The edge cases are specific to the
ticket, not a generic checklist.

**Scenario 8 — Verifies technical assumptions against the codebase**
Input: Ticket that states a technical fact ("service X already
supports Y"). Prompt includes:
`SIMULATE {codebase analyser agent} returning: "service X has no Y
method; the nearest capability is Z in src/services/x.ts:120". Do not
actually spawn the agent.`
Expected: Skill's first response references the simulated agent
finding (e.g. "the analyser reports X does not support Y — did you
mean to scope this to Z?") rather than asking the user the factual
question verbatim. Questions that require human judgment (intent,
priority, trade-off) may still be routed to the user in follow-ups.

**Scenario 9 — Probes empty Dependencies section**
Input: Ticket with blank Dependencies but non-trivial Requirements that
imply prerequisites
Expected: Skill asks what must exist before this work, what this work
blocks, and whether any schema or data changes are preconditions.

**Scenario 10 — Stops when all major branches are resolved**
Input: Prompt simulates a compressed conversation history in which
the user has given concrete, confirmed positions on scope, acceptance
criteria, edge cases, dependencies, and assumptions. Prompt ends with:
`[User responds: "Yes, that's confirmed."]`
Expected: Skill's next response is the three-section findings summary
(see Scenario 11) — NOT another question. The summary includes issues,
confirmed decisions, and accepted risks drawn from the simulated
history. If the simulated history leaves a genuine ambiguity
unresolved, the skill asks about that ambiguity instead; tested by a
variant fixture `all-but-deps-resolved` where Dependencies remains
ambiguous and the expected response is a question about Dependencies.

**Scenario 11 — Findings summary is structured**
Input: Conversation concluded
Expected: Skill emits a summary with three sections: `**Issues to fix**`
(concrete changes needed), `**Decisions confirmed**` (affirmed by user),
`**Risks accepted**` (acknowledged and accepted by user). Asks whether
to update the ticket.

**Scenario 12 — Optional targeted edit on user approval**
Input: User approves updating the ticket
Expected: Skill uses the `Edit` tool to apply targeted modifications to
the relevant body sections only: `Acceptance Criteria`, `Dependencies`,
`Assumptions`, `Technical Notes`. The skill NEVER modifies any
frontmatter field (`ticket_id`, `title`, `date`, `author`, `type`,
`status`, `priority`, `parent`, `tags`) nor the body `**Type**:`,
`**Status**:`, `**Priority**:`, or `**Author**:` labels — any such
transition is `/update-ticket`'s concern. Does NOT rewrite unaffected
sections. If an Edit target string cannot be matched (e.g. the section
contents differ from what the skill read), the skill aborts that edit
with a diagnostic and continues with the remaining approved edits.

**Scenario 13 — No-redesign guard**
Input: Conversation surfaces a concern that suggests a different
architectural approach might be better
Expected: Skill flags the concern as an Issue for the user to decide —
does NOT propose a rewritten ticket with a different architecture, does
NOT rewrite `Requirements` to reflect an alternative approach.

**Scenario 14 — Missing or invalid ticket path**
Input: `/stress-test-ticket @meta/tickets/9999-does-not-exist.md`
Expected: Skill reports a clear error ("No ticket file at
meta/tickets/9999-does-not-exist.md") and exits without reading any
other file or starting questioning.

**Scenario 15 — Edit targets only owned sections and body labels are
preserved**
Input: Ticket with substantive Requirements and Summary content;
conversation leads the user to approve an edit to Acceptance Criteria
only
Expected: After the Edit, the full ticket is emitted and the eval
diffs the produced file against the checked-in
`expected-target.md`. The diff must show changes ONLY in Acceptance
Criteria; all nine frontmatter fields, the body
`**Type**:`/`**Status**:`/`**Priority**:`/`**Author**:` labels, the
Summary section, and the Requirements section must be byte-for-byte
unchanged.

### Changes Required

#### 1. Create `skills/tickets/stress-test-ticket/SKILL.md`

**File**: `skills/tickets/stress-test-ticket/SKILL.md`

Invoke `/skill-creator:skill-creator` with this specification:

```
Skill name: stress-test-ticket
Category: tickets
Model after: skills/planning/stress-test-plan/SKILL.md
  (follow its structure exactly; adapt wording from "plan" to "ticket"
  and narrow the interrogation areas to ticket-quality concerns)

Frontmatter:
  name: stress-test-ticket
  description: Interactively stress-test a ticket by grilling the user
    on scope, assumptions, acceptance criteria, edge cases, and
    dependencies to surface issues, gaps, and flawed assumptions before
    implementation is planned.
  argument-hint: "[ticket number or path]"
  disable-model-invocation: true
  allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)

Configuration preamble (bang-executed in this order):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh stress-test-ticket`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

Agent fallback (if no Agent Names section above):
  accelerator:reviewer, accelerator:codebase-locator,
  accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
  accelerator:documents-locator, accelerator:documents-analyser,
  accelerator:web-search-researcher

Path injection:
  **Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`

Template section: intentionally omitted. Matching `stress-test-plan`,
this skill reasons over the ticket file directly and does not need
a template block injected. Ticket section names are known from
`templates/ticket.md` at skill-author time.

Instructions injection (end of file):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh stress-test-ticket`

Layout: the three preamble bang-executions appear immediately after
the closing `---` of the frontmatter and BEFORE the `# Stress-Test
Ticket` H1, with no intervening heading — matching the layout of
every sibling ticket skill and `stress-test-plan`.

Skill flow:

  Initial Response
    If a ticket path was provided: read the ticket file FULLY. Read any
    parent ticket referenced via the parent field. Optionally spawn
    codebase agents ({codebase locator agent}, {codebase analyser agent})
    if the ticket makes specific technical claims that the agents can
    verify — but do not spawn them reflexively. Wait for any spawned
    agents to complete before the first question.
    If no ticket path: ask for a path and show an example. Wait.

  The Stress-Testing Process
    Walk the decision tree depth-first. Ask ONE question (or a tightly
    related cluster) at a time. Build on the previous answer. Follow
    each thread to its conclusion before switching branches.
    Self-answer from the codebase when the question is about technical
    reality (spawn an agent if needed) — reserve user-facing questions
    for intent, priority, trade-off, and scope decisions.
    Be adversarial but constructive: challenge vague language, probe
    edge cases, question scope, surface contradictions between
    sections, test completeness.

  What to Stress-Test
    Work through these areas as the conversation leads to them — do NOT
    treat as a checklist:
      - Assumptions: what is assumed about the system that might be
        wrong? verify against codebase
      - Acceptance criteria: testable? specific? measurable? cover
        failure paths?
      - Scope: too big? too small? decomposable? over-scoped for first
        delivery?
      - Edge cases: empty data, concurrent access, partial failures,
        malformed input, timeouts, large datasets
      - Dependencies: what must exist first, what this blocks, schema
        or data prerequisites
      - Non-functional: performance, security, accessibility,
        observability
      - Definition of done: clear completion criteria, verifiable
      - Consistency: sections agree with each other; Requirements match
        Acceptance Criteria match Summary

  When to Stop
    Stop when all major branches are explored, no realistic failure
    scenario remains unaddressed, and the user has confirmed positions
    on all identified ambiguities. Do NOT stop prematurely when genuine
    issues remain — flag them and ask to resolve.

  Capturing Changes
    On conclusion, emit a structured summary with three sections:
      **Issues to fix** — concrete changes needed
      **Decisions confirmed** — affirmed by user
      **Risks accepted** — acknowledged and accepted
    Ask whether to update the ticket.
    If the user approves updates: use the Edit tool to apply targeted
    modifications to the body sections Acceptance Criteria,
    Dependencies, Assumptions, or Technical Notes ONLY. Never modify
    any frontmatter field (ticket_id, title, date, author, type,
    status, priority, parent, tags) nor the body **Type**/**Status**/
    **Priority**/**Author** labels — those transitions are
    /update-ticket's concern. Do NOT rewrite sections beyond what was
    agreed. If an Edit target cannot be matched, abort that edit
    with a diagnostic and continue with the remaining agreed edits.
    After editing, summarise the changes made.

  Important Guidelines
    - Conversation, not report — value is in the back-and-forth
    - Don't redesign — surface concerns for user decision; do not propose
      an alternative architecture or rewrite Requirements
    - Verify against reality — spawn agents to check technical claims
    - Depth over breadth — thoroughly stress-test riskiest parts
    - Respect confirmed decisions — don't circle back
    - Edit conservatively — minimum changes needed

  Relationship to Other Commands
    Position in the ticket lifecycle:
      /create-ticket or /extract-tickets — create the ticket
      /refine-ticket — decompose and enrich
      /review-ticket — automated multi-lens quality review
      /stress-test-ticket — interactive adversarial examination (this)
      /create-plan — plan implementation from an approved ticket
    /review-ticket and /stress-test-ticket are complementary: review
    gives broad coverage through lenses, stress-test goes deep through
    conversation.
```

Iterate with `/skill-creator:skill-creator` until all 15 evals pass.
Commit `evals/evals.json`, `evals/benchmark.json`, and `SKILL.md` in
the same change.

#### 2. Create eval fixtures

**Directory**: `skills/tickets/stress-test-ticket/evals/files/`

For each scenario that requires a concrete ticket fixture (Scenarios
2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15), create a `ticket.md` file
under `skills/tickets/stress-test-ticket/evals/files/<scenario-name>/`
that exhibits the condition under test. For Scenario 15 additionally
check in `expected-target.md` with the post-edit byte-level expected
state so the eval can diff against it. Follow the fixture pattern
used by `completeness-lens/evals/files/*/ticket.md` — the fixture is
a complete ticket in the template format, crafted to trigger the
specific behaviour the eval verifies.

**Expected-fixture naming convention** (used by Phase 6 onward, to
be propagated to earlier skills' evals as a follow-up):
- `expected-target.md` — the post-state of the ticket the skill
  operated on (single-target operations: stress-test edit, refine-
  ticket non-decompose ops)
- `expected-parent.md` — the post-state of the parent ticket after
  decompose (refine-ticket only; the parent IS the target, but
  `expected-parent.md` reads more clearly given the children fanout)
- `expected-child-N.md` — the post-state of the Nth child written
  by decompose (refine-ticket only)

Scenarios that simulate agent results (8) and scenarios that
simulate compressed conversation history (4, 10) encode the
simulation inline in the eval's prompt field rather than in a
fixture file — following the pattern used by create-ticket evals 9
and 14.

### Success Criteria

#### Automated Verification:

- [x] Skill directory created:
  `test -d skills/tickets/stress-test-ticket`
- [x] SKILL.md exists:
  `test -f skills/tickets/stress-test-ticket/SKILL.md`
- [x] Evals file exists:
  `test -f skills/tickets/stress-test-ticket/evals/evals.json`
- [x] Benchmark file exists:
  `test -f skills/tickets/stress-test-ticket/evals/benchmark.json`
- [x] All 15 evals pass 100% in `benchmark.json` (verified by reading
  `run_summary.with_skill.pass_rate.mean == 1.0`)
- [x] Phase 1–5 regression suite passes: `mise run test`
- [x] Skill frontmatter matches the specification:
  `grep -E '^(name|description|argument-hint|disable-model-invocation|allowed-tools):' skills/tickets/stress-test-ticket/SKILL.md` matches the spec field-for-field
- [x] Frontmatter field ORDER is correct — the grep above emits the
  five keys in the exact order name, description, argument-hint,
  disable-model-invocation, allowed-tools
- [x] Preamble present: grep confirms each of the three
  `config-read-*.sh` bang-executions is present in the SKILL.md

#### Manual Verification:

- [ ] Run `/stress-test-ticket` with no arguments in a fresh session —
  verify it asks for a path without reading or editing anything
- [ ] Run `/stress-test-ticket @meta/tickets/NNNN-*.md` on a real
  ticket — verify the conversation is depth-first, one question at a
  time, and references specific ticket content
- [ ] During the conversation, introduce a vague user answer and
  verify the skill follows up on the same branch (not a new topic)
- [ ] On conclusion, verify the findings summary has the three
  sections and that approving an edit produces a targeted `Edit` to
  the ticket (not a full rewrite)
- [ ] Verify the skill does NOT modify `ticket_id`, `date`, `status`,
  `priority`, `parent`, or `tags` when applying edits

---

## Subphase 6B: `refine-ticket` Skill

### Overview

Author `skills/tickets/refine-ticket/SKILL.md` — a novel menu-driven
skill that decomposes tickets into children, enriches them with
codebase context, sharpens their acceptance criteria, appends
size indicators, and links them to related tickets or dependencies.
Mandatory codebase agents (`{codebase locator agent}` +
`{codebase analyser agent}`) run in parallel at the start to inform
every operation. Child tickets are written eagerly (numbering consumed
per child); parent tickets are updated to link to their children.
Hierarchy is displayed after decomposition so the user can verify the
new structure.

### Eval Scenarios

**Scenario index** (refine-ticket has 36 evals across 25 numbered
scenario blocks — Scenario 5a expands to 8 sub-scenario evals
and Scenario 24 to 2; the 36 total counts each sub-scenario as a
distinct eval. Cross-references in Step 4 / Important Guidelines /
fixture block resolve here):
- Invocation surface: 1, 2, 3, 4, 17, 18
- Decompose: 5, 5a (eight sub-scenarios: -approve, -edit, -drop,
  -add, -regenerate, -cancel, -unknown, -legend), 6, 7, 12, 13,
  19, 20, 23
- Enrich: 8, 14, 22
- Sharpen: 9, 21
- Size: 10, 10a
- Link: 11, 11a, 11b, 11c
- Multi-operation order: 16
- Edit-failure recovery: 24a, 24b
- Post-refinement offer: 15

**Scenario 1 — Bare invocation prompts for ticket identifier**
Input: `/refine-ticket` (no arguments)
Expected: Skill asks for a ticket identifier — accepting either a
path (e.g. `meta/tickets/0042-user-auth.md`) or a bare ticket number
(e.g. `0042` or `42`, resolved against the configured tickets
directory). Prompt also suggests running `/list-tickets` to discover
available tickets. Skill waits; does not read or edit any file; does
not spawn agents.

**Scenario 2 — Ticket path resolution reads fully before acting**
Input: Path provided to a story ticket
Expected: Skill reads the entire ticket. If the ticket has a non-empty
`parent` field, it also reads the parent ticket. The ticket template is
injected at skill-load time via `config-read-template.sh ticket`; the
skill uses that injected content to know valid types, statuses, and
priorities (it does not invoke the template script at runtime).

**Scenario 3 — Codebase agents spawned in parallel; findings appear
in the menu preview**
Input: Path provided. Prompt includes:
`SIMULATE {codebase locator agent} returning: "src/auth/session.ts,
src/auth/tokens.ts, src/middleware/auth.ts are relevant."
SIMULATE {codebase analyser agent} returning: "session.ts uses an
in-memory map; tokens.ts signs JWTs; middleware reads cookies."
Both agents should appear in a single parallel tool-use turn. Do
not actually spawn the agents.`
Expected: Skill's response contains a single tool-use block with
BOTH Agent invocations (demonstrating parallelism as an observable
artefact), and the menu presented to the user includes previews that
reference the simulated findings (e.g. "decompose — Requirements
cover auth surfaces across 3 files", "enrich — 3 relevant files
identified"). Sequential agent spawning (two separate tool-use
turns) fails the scenario.

**Scenario 4 — Menu presents five operations**
Input: Agent results received
Expected: Skill presents the five operations with a one-line
description of each: **decompose**, **enrich**, **sharpen**, **size**,
**link**. User can pick one, multiple, or "all relevant".

**Scenario 5 — Decompose epic → child stories**
Input: Epic ticket selected for decomposition
Expected: Skill proposes 2–5 candidate child stories, each with a
draft title and one-line Summary derived from the epic's Requirements.
User iterates using the approval grammar (see Scenario 5a). Before any
write, skill warns: "This will allocate N ticket numbers and write N
files; aborting mid-write leaves partial state. Proceed? (y/n)". On
approval, skill calls `ticket-next-number.sh --count N` once,
writes each child with `type: story`, `status: draft`,
`parent: <parent_id>` (canonicalised to zero-padded four-digit
string), then appends a new `### Child tickets` subsection to the
END of the parent's `Requirements` section containing the child link
list (`- NNNN — title`, one per line, in write order). The existing
Requirements prose is never modified; the Edit targets only the
Requirements-section terminator. On completion, the skill prints a
ledger of which NNNN were written (see Scenario 12).

**Scenario 5a (group) — Decompose approval grammar**

The grammar is verified by eight focused sub-scenarios, each using
the same Scenario 5 fixture (an epic with a non-empty Requirements
section) and a single-turn prompt that simulates the proposal-
already-presented state. Each sub-scenario asserts the skill's NEXT
output for one verb. None of the 5a sub-scenarios actually write
files; the proposal-state assertions are pinned in the prompt
itself or in `expected-proposal-state.md` snippets where useful.

**Scenario 5a-approve — `approve all` proceeds to the write step**
Input prompt simulates: candidate proposal of 3 children presented;
user types `approve all`.
Expected: Skill emits the pre-write warning ("This will allocate 3
ticket numbers…") and waits for y/n confirmation. Synonyms `yes`
and `lgtm` are tested by re-running this scenario with each
synonym in turn (one fixture, three prompt variants).

**Scenario 5a-edit — `edit N: <new title>` updates the proposal**
Input prompt simulates: 3-child proposal with titles A/B/C; user
types `edit 2: New Title`.
Expected: Skill re-shows the updated proposal where child 2's title
is exactly `New Title`; children 1 and 3 are unchanged. Skill does
NOT proceed to write. Skill restates the grammar legend below the
updated proposal.

**Scenario 5a-drop — `drop N` removes a child**
Input prompt simulates: 3-child proposal A/B/C; user types `drop 2`.
Expected: Skill re-shows a 2-child proposal containing only A and
C, renumbered as 1 and 2. Skill does NOT proceed to write.

**Scenario 5a-add — `add: <title>` appends a child**
Input prompt simulates: 3-child proposal A/B/C; user types
`add: New Child`.
Expected: Skill re-shows a 4-child proposal with A/B/C/`New Child`
numbered 1–4. Skill does NOT proceed to write.

**Scenario 5a-regenerate — `regenerate` produces a fresh proposal**
Input prompt simulates: 3-child proposal A/B/C; user types
`regenerate`.
Expected: Skill discards the current proposal and produces a new
set of candidate children grounded in the same Requirements but
with different titles or scope. The new proposal contains 2–5
children (per the decompose rules). Skill does NOT proceed to
write.

**Scenario 5a-cancel — `cancel` exits decompose without writing**
Input prompt simulates: 3-child proposal presented; user types
`cancel` (also tested with synonym `abort`).
Expected: Skill exits decompose with a clear "decompose cancelled —
no children written" message and returns to the menu. No ticket
numbers are allocated. No files are written.

**Scenario 5a-unknown — Unrecognised input re-shows proposal and
grammar**
Input prompt simulates: 3-child proposal presented; user types
`looks fine to me` (or any free-form text not matching a verb).
Expected: Skill prints "unrecognised command" plus the grammar
legend and re-shows the unchanged proposal. Skill does NOT proceed
to write and does NOT silently treat the input as approval.

**Scenario 5a-legend — Initial proposal includes a grammar legend**
Input: Skill has just generated the first candidate proposal.
Expected: The proposal output includes a one-line legend
immediately under the numbered child list:
`Commands: approve all | edit N: <title> | drop N | add: <title> | regenerate | cancel`
The legend appears on the FIRST proposal turn (not only after an
unrecognised command).

**Scenario 6 — Decompose story → child tasks**
Input: Story ticket selected for decomposition
Expected: Same flow as Scenario 5 but children have `type: task` and
child count is 2–4. The parent story's `Requirements` gains the same
`### Child tickets` subsection — never a separate `Tasks` subsection
(keeping the structural convention identical across parent types).

**Scenario 7 — All nine frontmatter fields populated on children**
Input: Decompose completed
Expected: Each child ticket has all nine frontmatter fields, derived
per these rules (identical to create-ticket's Quality Guidelines where
applicable):
- `ticket_id` — from `ticket-next-number.sh --count N`, zero-padded
  four-digit string
- `title` — per-child proposal; matches the body H1 exactly
- `date` — current UTC timestamp via
  `date -u +%Y-%m-%dT%H:%M:%S+00:00` (not `Z` suffix)
- `author` — EXTENDS create-ticket's chain by first checking the
  parent ticket's `author` field; if unset, falls through to
  create-ticket's standard chain (configured author via
  `config-read-context.sh`'s `author` field if present, else
  `jj config get user.name` falling back to `git config user.name`,
  else ask the user once and apply to every child in this session).
  See `skills/tickets/create-ticket/SKILL.md` §"Quality Guidelines"
  for the source chain.
- `type` — derived from parent type: `epic → story`, `story → task`,
  `bug`/`spike` → ask the user to confirm before proceeding (see the
  bug/spike challenge eval), any other parent type → `story` by
  default, with a one-line notice so the user can override
- `status` — literal `draft`
- `priority` — inherit from parent; if parent has none, ask the user
  once and apply the answer to every child written in this session
- `parent` — the target ticket's `ticket_id`, canonicalised to a
  zero-padded four-digit string
- `tags` — a verbatim copy of the parent's `tags` array (empty array
  if the parent has none)

Filename matches `NNNN-kebab-slug.md` where NNNN equals `ticket_id`
and the slug is derived from the child's title via the same slugify
rule create-ticket uses.

**Environment-dependent fields**: `date` and `author` cannot be
pinned byte-level in `expected-child-N.md` because they vary per
test invocation and per machine. The eval splits assertions:
- The seven deterministic fields (`ticket_id`, `title`, `type`,
  `status`, `priority`, `parent`, `tags`) plus the body content
  are pinned byte-level in `expected-child-N.md`.
- `date` is asserted via regex
  `^date: "\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00"$` and a
  recency check (within ±60s of test invocation).
- `author` is asserted via the chain rule: in eval-harness
  contexts, the parent's `author` field is the first match (since
  fixtures pin parent author); the assertion is that the child's
  `author` equals the parent's `author` byte-for-byte.

**Scenario 8 — Enrich adds Technical Notes from agent results**
Input: Ticket selected for enrichment
Expected: Skill proposes content for the `Technical Notes` section
with specific `path:line` references drawn from the codebase agent
results (e.g., `src/auth/session.ts:42`). User approves; `Edit` tool
applies the change to `Technical Notes`. Skill does NOT modify
`Requirements`, `Acceptance Criteria`, `Summary`, or any frontmatter
field during enrich.

**Scenario 9 — Sharpen challenges vague acceptance criteria**
Input: Ticket with "it should be fast" as an acceptance criterion;
user selects sharpen
Expected: Skill proposes specific, measurable rewrites (e.g.,
"p95 latency under 200ms for the endpoint under the default benchmark
dataset"). Iterates with the user until the criterion is testable.
`Edit` tool applies the approved rewrite to `Acceptance Criteria`.
Skill does NOT modify criteria that are already testable.

**Scenario 10 — Size appends a size indicator as the first line of
Technical Notes, not as a frontmatter key**
Input: Story selected for size
Expected: Skill proposes a t-shirt size (XS, S, M, L, XL) with a
rationale referencing specific files or subsystems from the agent
results (e.g., "M — touches auth and session store; ~4 files"). The
size output is written as a single line (`**Size**: M — rationale`)
placed as the FIRST line of the `Technical Notes` section, separated
from any following content by a blank line. The line is never added
as a frontmatter key.

**Scenario 10a — Size re-run requires diff + second confirmation**
Input: Ticket that already has `**Size**: M — touches auth and
session store` at the top of Technical Notes; user selects size
again and the agents now suggest L.
Expected: Skill reads the existing `**Size**:` line, proposes the
new value + rationale, shows a unified diff (existing line struck,
new line added), and asks for explicit y/n confirmation before
invoking Edit. On `y`, the skill uses Edit to replace the existing
line in place (not append a second line). On `n`, the skill makes
no Edit and exits the size operation. If the proposed value AND
rationale equal the existing line byte-for-byte (whitespace-
ignored), the skill reports "size unchanged — M" and makes no Edit
without prompting.

**Scenario 11 — Link populates Dependencies from related tickets**
Input: Ticket with empty `Dependencies`; user selects link
Expected: Skill searches the tickets directory for related tickets
using the following rule: if the directory contains 30 or fewer
`NNNN-*.md` files, read them directly (via a `Glob` + batched `Read`
pattern); otherwise spawn `{documents locator agent}` scoped to the
tickets directory. Skill proposes `Blocked by:` and/or `Blocks:`
entries referencing real ticket numbers. User approves; `Edit` tool
populates `Dependencies`. Skill does NOT invent non-existent ticket
numbers; if no related tickets are found it prints "no related
tickets found — link skipped" and leaves the section unchanged.

**Scenario 11b — Link with >30 tickets spawns documents-locator
agent**
Input: Tickets directory containing 35 stub `NNNN-*.md` files
(checked-in fixture). User selects link on a target ticket. Prompt
includes `SIMULATE {documents locator agent} returning: "0031
appears to be a blocker for this work; 0029 references the same
subsystem". Do not actually spawn the agent.`
Expected: Skill uses a single `Glob` tool invocation to enumerate
`NNNN-*.md` files in the tickets directory (consistent with Step
4e's "via Glob" wording and refine-ticket's allowed-tools, which
do not permit a generic Bash invocation against the tickets
directory), observes the count exceeds 30, and spawns the
{documents locator agent} as a single tool-use call. The eval
asserts: (a) a `Glob` tool-use appears, (b) a `documents-locator-
agent` Agent invocation appears, and (c) NO batched Read of the
35 ticket files appears. The proposed link entries reference 0031
and 0029 from the simulated agent result.

**Scenario 11c — Link with exactly 30 tickets reads directly (no
agent)**
Input: Tickets directory containing exactly 30 stub `NNNN-*.md`
files (checked-in fixture). User selects link on a target ticket.
Expected: Skill uses a single `Glob` invocation to enumerate
files, observes the count is ≤30, and reads them directly via
batched Read. The eval asserts: (a) a `Glob` tool-use appears,
(b) batched Read calls appear (one per matched file or grouped),
and (c) NO `documents-locator-agent` Agent invocation appears.
This scenario brackets the threshold from below; together with
11b it pins the boundary at 30.

**Scenario 11a — Link re-run offers replace/append/skip**
Input: Ticket with non-empty `Dependencies`; user selects link
Expected: Skill reads existing Dependencies entries and asks
"replace (overwrites existing entries) / append (add new entries
after existing) / skip?". Replace requires a unified diff and a
second y/n confirmation before Edit. Append writes only net-new
entries (skipping duplicates of existing ones). Skip makes no Edit.

**Scenario 12 — Numbering consumed per child; not deferred; ledger
emitted**
Input: Decompose approved with three children
Expected: Before writing, the skill prints a warning: "This will
allocate 3 ticket numbers and write 3 files; aborting mid-write
leaves partial state — use `jj restore <file>` to discard any
children written." and waits for explicit y/n confirmation.
On confirmation, `ticket-next-number.sh --count 3` is called exactly
once, producing three consecutive numbers. The skill re-checks that
each computed `NNNN-slug.md` filename does not already exist
immediately before writing it and aborts with a clear collision
diagnostic if it does. All three child files are written. On
completion, the skill prints a ledger: one `Wrote NNNN — title` line
per child, followed by "Allocated 3 numbers, wrote 3 files." On
abort (Ctrl-C or collision), the ledger shows allocated vs. written
counts and lists any written child filenames. Children already
written remain on disk — there is no rollback. The parent's
`### Child tickets` subsection is updated only once all child writes
succeed; if the parent Edit fails, the children are left on disk and
the skill prints the ticket numbers so the user can manually link or
`jj restore` them.

**Scenario 13 — Parent hierarchy displayed after decompose, matches
pinned /list-tickets format**
Input: Decompose completed with parent 0042 "User Auth Rework" (epic)
and three children 0043/0044/0045.
Expected: Skill displays the updated hierarchy verbatim as:
```
0042 — User Auth Rework (type: epic)
  ├── 0043 — Session Store (type: story, status: draft)
  ├── 0044 — Token Signing (type: story, status: draft)
  └── 0045 — Middleware Refactor (type: story, status: draft)
```
The eval's `expected_output` pins this exact string (including the
Unicode box-drawing characters). The format matches the literal
rendering pinned in `/list-tickets` §"Hierarchy Rendering" by
Subphase 6.0 — any drift from that format fails the scenario.

**Scenario 14 — Idempotent enrich: no blind duplication; destructive
replace requires diff + second confirmation**
Input: Ticket that already has substantive Technical Notes content;
user selects enrich
Expected: Skill reads existing Technical Notes, proposes net-new
content grounded in agent results, then asks "replace (deletes
existing Technical Notes), append (add after existing content), or
skip?". If the user picks `replace`, the skill must show a unified
diff of the change (old struck, new added) and require an explicit
second `y` confirmation before invoking Edit. If the user picks
`append`, no diff is needed — the new content is added after any
existing content, preserving the LEADING `**Size**:` line at the
top of Technical Notes if present (per Scenario 10's first-line
invariant). If `skip`, no Edit is performed. The skill never
blindly appends a duplicate block and never replaces existing content
without the two-step confirmation.

**Scenario 15 — Offers review after refinement, does not invoke it**
Input: Any refinement operation completed
Expected: Skill offers "Would you like to run `/review-ticket` on
this ticket now?" (or on each child after decompose). Does NOT
invoke the review skill itself — only surfaces the offer. The skill
exits after the offer regardless of the user's response.

**Scenario 16 — Multi-operation selection runs in canonical order**
Input: User selects decompose, enrich, and size from the menu in
the order (size, decompose, enrich)
Expected: Regardless of selection order, operations execute in the
canonical order decompose → enrich → sharpen → size → link. The
hierarchy display (Step 5) appears immediately after decompose and
before enrich runs. Size is applied last among the selected
operations and its `**Size**:` line appears as the FIRST line of
Technical Notes in the final state (above the content enrich added).

**Scenario 17 — Missing or invalid ticket path**
Input: `/refine-ticket @meta/tickets/9999-does-not-exist.md`
Expected: Skill reports a clear error ("No ticket file at
meta/tickets/9999-does-not-exist.md — run `/list-tickets` to see
available tickets") and exits without reading any other file,
spawning any agent, or writing anything.

**Scenario 18 — Malformed ticket frontmatter**
Input: Path to a ticket whose YAML frontmatter is missing the
closing `---` (or has a syntax error)
Expected: Skill reports "Could not parse frontmatter in
<path> — the file may be corrupted. Re-open it and check that the
YAML frontmatter is bracketed by two `---` lines and contains all
nine required fields, or run `/update-ticket <path>` which surfaces
the same diagnostic with field-level detail." and exits without
editing the file or spawning agents. Error phrasing matches
/update-ticket's convention for the same condition.

**Scenario 19 — Decompose on bug/spike challenges the user**
Input: Bug ticket selected for decomposition
Expected: Skill asks "bug/spike tickets don't typically decompose —
are you sure? (y/n)" and only proceeds to propose children on
explicit `y`. On `n`, the skill exits decompose and returns to the
menu.

**Scenario 20 — Decompose on a parent that already has children**
Input: Parent ticket whose Requirements already contains a
`### Child tickets` subsection; user selects decompose
Expected: Skill detects the existing subsection and offers "append
(add new children to the existing list) / skip (do not decompose
further) / cancel". Never replaces the existing list silently. On
`append`, new children are added to the end of the existing link
list in write order.

**Scenario 21 — Sharpen on a ticket where every AC is already
testable**
Input: Ticket with fully specified, measurable acceptance criteria;
user selects sharpen
Expected: Skill reports "all acceptance criteria already testable —
nothing to sharpen" and makes no Edit.

**Scenario 22 — Enrich when codebase agents return nothing concrete**
Input: Ticket referencing a subsystem that the (simulated) agents
return no relevant files for. Prompt includes:
`SIMULATE {codebase locator agent} returning: "no relevant files
found". SIMULATE {codebase analyser agent} returning: "no components
identified".`
Expected: Skill reports "no enrichment could be grounded in code —
skipping enrich" and makes no Edit. Does not invent or hallucinate
`path:line` references to satisfy the operation.

**Scenario 23 — User-driven cancel after partial writes leaves a
consistent ledger**
Input: Multi-step prompt simulating decompose approved with 3
children, then user cancels mid-flow. Encoded as:
`[Step 1: user invokes /refine-ticket on epic 0042]`
`[Step 2: agents simulated to return relevant files]`
`[Step 3: skill proposes 3 children A, B, C]`
`[Step 4: user types "approve all"]`
`[Step 5: skill emits pre-write warning; user types "y"]`
`[Step 6: skill writes child 1 (0043) and child 2 (0044)]`
`[Step 7: BEFORE writing child 3, user types "cancel" or sends Ctrl-C]`
Expected: Skill stops the write loop, does NOT write child 3, does
NOT update the parent's `### Child tickets` subsection (the parent
Edit is the final step and only runs after all child writes succeed).
Skill prints the ledger:
```
Allocated 3 numbers (0043, 0044, 0045); wrote 2 files:
  Wrote 0043 — Session Store
  Wrote 0044 — Token Signing
Cancelled before writing 0045.
Parent 0042 was NOT updated. To recover:
  jj restore meta/tickets/0043-session-store.md  # discard child 1
  jj restore meta/tickets/0044-token-signing.md  # discard child 2
or run /update-ticket on 0042 to add child links manually.
```
The eval pins the ledger format byte-level in `expected_output`. The
test does not depend on simulating Write-tool failure — it relies
only on the skill respecting a user-driven cancel between writes,
which is a pattern the eval harness already supports (matching how
the create-ticket evals model user input mid-flow).

Note: a fully orthogonal "Write-tool fails after partial writes"
case would also be valuable but requires eval-harness primitives
that are not currently established. Capturing it as future work
rather than asserting it here.

**Scenario 24 — Edit-target mismatch aborts that edit, continues
others (covers both `not found` and `not unique` failure modes)**

Two sub-scenarios verify both failure modes the skill must handle
uniformly:

**Scenario 24a — `not found` (target string absent)**
Input: Multi-step prompt simulating an enrich + sharpen selection
on a target ticket whose Acceptance Criteria section has been
changed between the skill's Read step and Edit step. Encoded
via two physical fixtures: `ticket-before-edit.md` (what the skill
Reads) and `ticket-during-edit.md` (what is actually on disk when
Edit runs); the harness swaps the file between Read and Edit.
`[Step 1: skill reads ticket-before-edit.md; AC1 is "AC1: foo"]`
`[Step 2: skill proposes enrich Technical Notes; user approves]`
`[Step 3: skill proposes sharpen "AC1: foo" → "AC1: foo measurably under X ms"; user approves]`
`[Step 4: harness swaps file to ticket-during-edit.md whose AC1 is "AC1: foo (updated)"]`
Expected: Skill's enrich Edit on Technical Notes succeeds (its
target string was unaffected). Skill's sharpen Edit on Acceptance
Criteria fails the exact-match check (`old_string` not found); the
skill prints the diagnostic `Edit target for "Acceptance Criteria"
did not match — the section may have changed since this session
began. Sharpen aborted; other operations preserved.` and continues
to any remaining operations. The eval asserts the tool-use trace
shows Edit was attempted with the original `old_string` (verifying
the skill genuinely tried and failed rather than skipped).

**Scenario 24b — `not unique` (target string ambiguous)**
Input: Decompose selected on a parent whose Requirements section
contains a code fence quoting the heading `## Acceptance Criteria`
verbatim (e.g., a template snippet pasted into Requirements as
documentation). Fixture: `parent-with-duplicate-heading.md`.
Expected: Skill performs the pre-Edit uniqueness count, observes
that the constructed `old_string` matches more than once, and
aborts the parent update with the diagnostic `Could not locate a
unique '## Acceptance Criteria' anchor in <path> (matches found:
2). Parent not updated. Children NNNN, NNNN, NNNN remain on disk;
add their links manually or run \`jj restore <parent-path>\` and
re-run decompose.` Children written before the parent Edit attempt
remain on disk per the no-rollback policy. The eval asserts the
tool-use trace shows NO Edit was attempted on the parent (the
uniqueness check happened first), and the ledger lists the orphan
children's filenames.

The eval diffs the final state against `expected-target.md` (24a)
or `expected-parent-not-updated.md` plus `expected-child-N.md`
files (24b) and asserts the diagnostic appears in the output for
each.

### Changes Required

#### 1. Create `skills/tickets/refine-ticket/SKILL.md`

**File**: `skills/tickets/refine-ticket/SKILL.md`

Invoke `/skill-creator:skill-creator` with this specification:

```
Skill name: refine-ticket
Category: tickets
Model after: structural composition of
  - skills/tickets/create-ticket/SKILL.md (agent spawning, template
    injection, author resolution, frontmatter population)
  - skills/tickets/update-ticket/SKILL.md (parent canonicalisation,
    targeted Edit usage on existing tickets)
  - skills/tickets/list-tickets/SKILL.md (hierarchy rendering inline)

Frontmatter:
  name: refine-ticket
  description: Interactively refine a ticket by decomposing it into
    children, enriching it with codebase context, sharpening its
    acceptance criteria, sizing it, or linking it to dependencies.
    Use after a ticket has been drafted and before planning begins.
  argument-hint: "[ticket number or path]"
  disable-model-invocation: true
  allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)

Configuration preamble (bang-executed in this order):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh refine-ticket`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

Agent fallback (if no Agent Names section above):
  accelerator:reviewer, accelerator:codebase-locator,
  accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
  accelerator:documents-locator, accelerator:documents-analyser,
  accelerator:web-search-researcher

Path injection:
  **Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`

Template section (rendered inline via bang-execution at skill-load
time, matching create-ticket/update-ticket — place under a
`## Ticket Template` H2 immediately after path injection):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh ticket`

Instructions injection (end of file):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh refine-ticket`

Layout: the three preamble bang-executions appear immediately after
the closing `---` of the frontmatter and BEFORE the `# Refine Ticket`
H1, with no intervening heading — matching the layout of every
sibling ticket skill.

Skill flow:

  Step 0 — Parameter check
    If no ticket path: ask for one; show an example; wait.
    If ticket path: proceed to Step 1.

  Step 1 — Read target and context
    Read the target ticket fully. If parent field is non-empty, read
    the parent ticket too. Read the template to know valid types,
    statuses, priorities.

  Step 2 — Analyse (mandatory parallel agents)
    Spawn in parallel:
      - {codebase locator agent}: find files relevant to the ticket's
        Requirements and Summary
      - {codebase analyser agent}: analyse how the relevant components
        currently work
    Wait for both before presenting the menu.

  Step 3 — Present refinement menu
    Present the five operations with one-line descriptions:
      1. decompose — split into child tickets (epic→stories, story→
         tasks)
      2. enrich — add Technical Notes from codebase analysis
      3. sharpen — tighten vague acceptance criteria
      4. size — append a t-shirt size indicator with rationale
      5. link — populate Dependencies from related tickets
    Reference agent findings in the menu prose where relevant. For
    each operation, show a one-line preview based on agent results
    so the user can pick informed — e.g. "decompose — Requirements
    suggest 3 child stories (R1, R2, R3)"; "sharpen — 2 vague
    acceptance criteria detected (AC3, AC5)"; "size — estimate M
    based on files in auth/ and session/"; "link — 1 potential
    blocker found (0031)"; "enrich — 4 relevant files identified".
    Operations that have nothing to do should be marked as such
    (e.g. "sharpen — all criteria already testable").
    User selects one or more. Regardless of selection order, the
    skill always executes in this canonical order: decompose →
    enrich → sharpen → size → link. This ordering guarantees that
    enrich's Technical Notes content is in place before size's
    `**Size**:` line is appended, and that decompose's children
    exist before any link operation references them.

  Step 4 — Execute operations (per user selection)

    4a. decompose
      Propose 2–5 (story decomposition) or 2–4 (task decomposition)
      candidate children with draft titles and one-line Summaries from
      the Requirements section. If the target ticket is of type `bug`
      or `spike`, first ask "bug/spike tickets don't typically
      decompose — are you sure? (y/n)" and proceed only on explicit
      confirmation.
      Iterate using the approval grammar (see Scenarios 5a-approve
      / -edit / -drop / -add / -regenerate / -cancel / -unknown /
      -legend). Each candidate proposal MUST include a one-line
      grammar legend immediately under the numbered child list:
      `Commands: approve all | edit N: <title> | drop N | add: <title> | regenerate | cancel`
      Iteration terminates on `approve all` / `yes` / `lgtm`; the
      `cancel` / `abort` verbs exit decompose without writing;
      unrecognised input prints "unrecognised command", restates
      the legend, and re-shows the unchanged proposal.
      Before writing: warn "This will allocate N ticket numbers and
      write N files; aborting mid-write leaves partial state — use
      `jj restore <file>` to discard any children written. Proceed?
      (y/n)". Wait for explicit confirmation.
      On approval:
        - Call ticket-next-number.sh --count N (single call, N numbers)
        - For each child, write NNNN-kebab-slug.md with all nine
          frontmatter fields populated per the rules in Scenario 7
          (ticket_id from the script; title; date via `date -u
          +%Y-%m-%dT%H:%M:%S+00:00`; author extends create-ticket's
          chain (parent's author → config → jj/git identity → ask once);
          type via parent-derivation with bug/spike confirmation;
          status: draft; priority inherited or asked once per
          session; parent canonicalised four-digit; tags copied
          verbatim from parent).
          Body: Summary (from proposal), Context (linking to
          parent), Requirements (per child — minimal but substantive;
          no `[bracketed placeholder]` text), Acceptance Criteria
          (per child — minimal but substantive; sharpen can tighten
          these later), Dependencies (blank), other sections per
          template.
        - Append a new `### Child tickets` subsection to the END of
          the parent's `Requirements` section containing the child
          link list (`- NNNN — title`, one per line, in write order).
          Edit anchor (uniqueness-tolerant): construct a multi-line
          `old_string` that captures the LAST line of the parent's
          Requirements section followed by the blank-line transition
          and the `## Acceptance Criteria` H2. Concretely:
            1. Read the parent file.
            2. Locate `\n## Requirements\n` and `\n## Acceptance Criteria\n`.
            3. Extract the last non-empty line of Requirements (the
               line immediately before the blank-line transition to
               Acceptance Criteria); call this `<req_tail>`.
            4. Edit `old_string` = `<req_tail>\n\n## Acceptance Criteria\n`
               Edit `new_string` = `<req_tail>\n\n### Child tickets\n\n- NNNN — title\n- NNNN — title\n…\n\n## Acceptance Criteria\n`
          This anchor is unique whenever the parent has at least one
          line of Requirements content followed by the standard H2
          transition. Pre-Edit, count occurrences of the constructed
          `old_string` in the parent file; if not exactly 1, abort
          with the diagnostic below rather than invoking Edit (avoids
          Edit's `not unique` surface, which would otherwise leak
          past the pinned diagnostic).
          Abort diagnostic (covers all failure modes — missing H2,
          duplicate match, empty Requirements):
          `Could not locate a unique '## Acceptance Criteria' anchor
          in <path> (matches found: <N>). Parent not updated.
          Children NNNN, NNNN, NNNN remain on disk; add their links
          manually or run \`jj restore <parent-path>\` and re-run
          decompose.`
          Edge case: if Requirements is empty (no `<req_tail>`), use
          `## Requirements\n\n## Acceptance Criteria\n` as the
          two-headings-with-blank-between anchor and replace with
          `## Requirements\n\n### Child tickets\n\n- NNNN — title\n…\n\n## Acceptance Criteria\n`.
        - Re-running decompose on a parent that already has a
          `### Child tickets` subsection detects this and offers
          "append (add new children to the existing list) / skip
          (do not touch the subsection) / cancel". Never replace the
          existing list silently. For the append path, Edit anchor
          (uniqueness-tolerant): construct a multi-line `old_string`
          that captures the LAST `- NNNN — title` line of the
          existing `### Child tickets` subsection together with the
          blank-line transition and the next H2. Concretely:
            1. Read the parent file.
            2. Locate the `### Child tickets` subsection.
            3. Extract its last `- NNNN — title` line; call this
               `<last_link>`.
            4. Edit `old_string` = `<last_link>\n\n## Acceptance Criteria\n`
               Edit `new_string` = `<last_link>\n- NNNN — title\n- NNNN — title\n…\n\n## Acceptance Criteria\n`
          Pre-Edit, count occurrences of the constructed
          `old_string` in the parent file; if not exactly 1, abort
          with the same diagnostic shape as the initial-decompose
          case.
        - Print a completion ledger: "Wrote NNNN — title", one per
          child, followed by the count of numbers allocated vs.
          written (equal on success; lower on aborted runs).
      Display the updated hierarchy (see Step 5).

    4b. enrich
      Read the target's existing Technical Notes. Propose Technical
      Notes content grounded in agent results (include specific
      `path:line` references). If the codebase agents returned
      nothing concrete, tell the user "no enrichment could be
      grounded in code — skipping enrich" and make no Edit.
      If non-trivial Technical Notes content already exists, ask
      whether to replace, append, or skip (Scenario 14); replace
      requires a diff preview and a second y/n confirmation.
      Iterate until the user approves or skips.
      On approval: Edit the Technical Notes section of the target
      ticket only. Preserve any leading `**Size**:` line placed by
      a prior size operation (never overwrite it during enrich).
      Do not modify other sections.

    4c. sharpen
      Read the target's Acceptance Criteria. Identify criteria that
      are vague or untestable. If every criterion is already
      testable, report "all acceptance criteria already testable —
      nothing to sharpen" and make no Edit. Otherwise, for each
      vague criterion propose a specific, measurable rewrite and
      iterate with the user. Skip criteria that are already
      testable.
      On approval: Edit the Acceptance Criteria section with the
      rewrites. Preserve criteria that were not sharpened.

    4d. size
      Read the target's Technical Notes. If a `**Size**:` line
      already exists on the first line, capture its current value.
      Propose a t-shirt size (XS, S, M, L, XL) with a rationale
      referencing specific files or subsystems from agent results.
      Iterate.
      On approval:
        - If no `**Size**:` line exists: insert
          `**Size**: <value> — <rationale>` as the FIRST line of
          Technical Notes, followed by a blank line separating it
          from any existing content
        - If a `**Size**:` line exists and the proposed value AND
          rationale match the existing line byte-for-byte
          (excluding leading/trailing whitespace): report "size
          unchanged" and make no Edit
        - If a `**Size**:` line exists with a different value or a
          different rationale: show a unified diff of the
          proposed change and require an explicit second y/n
          confirmation before invoking Edit (matching the
          destructive-path policy used by enrich-replace and
          link-replace). This protects user-authored rationale
          prose from silent overwrite. On confirmation, replace
          the line in place via Edit.
      Do NOT add a frontmatter key.

    4e. link
      Count `NNNN-*.md` files in the tickets directory. If the count
      is ≤ 30, read them directly via Glob + batched Read. Otherwise
      spawn {documents locator agent} scoped to the tickets
      directory. Propose `Blocked by:` and/or `Blocks:` entries in
      Dependencies, referencing only real ticket numbers (verify
      each proposed number exists before including it). If none
      found, print "no related tickets found — link skipped" and
      make no Edit.
      If Dependencies already has content (see Scenario 11a), ask
      replace / append / skip. Replace requires a diff preview and
      a second y/n confirmation. Append writes only net-new entries,
      skipping duplicates.
      On approval: Edit the Dependencies section.

  Step 5 — Display hierarchy (immediately after decompose writes, if
  decompose was part of the selection AND at least one child was
  actually written; skipped otherwise — including cancel via
  Scenario 5a-cancel, bug/spike `n` via Scenario 19, and parent-Edit
  failure via the decompose abort path)
    Render the parent → children tree using the literal format pinned
    in /list-tickets (Subphase 6.0) — ASCII box-drawing, two-space
    indent per depth level, `├── ` for all children except the last
    and `└── ` for the last. The canonical fence below MUST appear
    verbatim in this step's prose so `scripts/test-hierarchy-format.sh`
    can verify byte-equality with the same fence in
    `list-tickets/SKILL.md`:

    <!-- canonical-tree-fence -->
    NNNN — parent title (type: <type>, status: <status>)
      ├── NNNN — child 1 title (type: <type>, status: <status>)
      ├── NNNN — child 2 title (type: <type>, status: <status>)
      └── NNNN — last child title (type: <type>, status: <status>)
    <!-- /canonical-tree-fence -->

    Concrete IDs (e.g. `0042 — User Auth Rework`) appear only in
    eval `expected_output` strings (Scenario 13), never in this
    SKILL.md.

    Step 5 runs inline after decompose completes (before enrich,
    sharpen, size, link operate on the parent), so subsequent edits
    to the parent are visible in the final state. The hierarchy is
    not re-rendered after later operations.

  Step 6 — Offer review (runs once after the entire operation set
  completes)
    Offer: "Would you like to run /review-ticket on this ticket now?"
    If decompose was in the selection, also offer to run review on
    each child. Do NOT invoke /review-ticket automatically — only
    offer. The skill exits after this offer regardless of the
    user's response.

Important Guidelines:
  - Mandatory codebase agents — always spawn at Step 2
  - Canonical operation order is fixed regardless of selection order:
    decompose → enrich → sharpen → size → link
  - Each operation modifies only its owned sections:
    decompose writes new files and appends a `### Child tickets`
      subsection to the parent's Requirements (never touching
      existing Requirements prose);
    enrich owns Technical Notes (prose content);
    sharpen owns Acceptance Criteria;
    size owns a single `**Size**: <value> — <rationale>` line which
      always lives as the FIRST line of Technical Notes (replace in
      place on re-run);
    link owns Dependencies
  - Never modify any frontmatter field of the target ticket
    (ticket_id, title, date, author, type, status, priority, parent,
    tags) — those transitions are /update-ticket's concern. Children
    of decompose are the exception: they are new tickets getting
    their full initial frontmatter.
  - Destructive paths (replace mode for enrich; replace mode for
    link if offered; clobbering an existing `**Size**:` line) must
    show a unified diff and require a second y/n confirmation before
    Edit is invoked
  - Edit failure path: if an Edit's target string cannot be matched
    (file changed between read and edit, section content differs),
    abort that specific edit with a clear diagnostic and continue
    with the remaining approved edits. For decompose specifically,
    write all children first and update the parent last; if the
    parent Edit fails, children stay on disk and the ticket numbers
    are printed so the user can manually link or `jj restore` them.
  - Numbering is consumed eagerly on decompose with no rollback; the
    skill assumes a single-session invocation. Concurrent refine
    sessions are not supported (ticket-next-number.sh has no
    locking); the filename-collision check immediately before each
    write is the safety net.
  - Canonicalise parent field to zero-padded four-digit string
    (same as /update-ticket)
  - Idempotent on re-run: every operation checks existing content
    before proposing additions (see operation-specific scenarios)

Relationship to Other Commands:
  /create-ticket or /extract-tickets — create the ticket
  /refine-ticket — this skill
  /review-ticket — quality review (offered by this skill)
  /stress-test-ticket — adversarial examination
  /update-ticket — status/metadata transitions (not this skill's
  concern)
```

Iterate with `/skill-creator:skill-creator` until all 36 evals pass
(including the eight 5a sub-scenarios as separate eval entries, plus
Scenarios 11b and 24 added below).
Commit `evals/evals.json`, `evals/benchmark.json`, and `SKILL.md` in
the same change.

#### 2. Create eval fixtures

**Directory**: `skills/tickets/refine-ticket/evals/files/`

Create fixture directories per scenario. Each directory contains
the concrete ticket(s) and, where verification needs byte-level
assertion, the expected post-condition files:

- Scenarios 2, 4, 8, 9, 10, 10a, 11, 11a, 14, 18, 20, 21, 22 —
  a single `ticket.md` in the shape the eval requires (existing
  Technical Notes for Scenarios 14 and 10a, vague acceptance
  criteria for Scenario 9, empty Dependencies for Scenario 11,
  non-empty Dependencies for Scenario 11a, malformed frontmatter
  for Scenario 18, ticket with existing `### Child tickets`
  subsection for Scenario 20, fully-testable ACs for Scenario 21,
  subsystem that agents won't match for Scenario 22, etc.)
- Scenarios 5, 6, 12, 13, 16, 19, 20, 23 — a parent `ticket.md`
  plus checked-in expected child files (`expected-child-1.md`,
  `expected-child-2.md`, …) and an `expected-parent.md` showing
  the post-decompose parent state with the `### Child tickets`
  subsection inserted between Requirements and Acceptance Criteria.
  The eval asserts byte-level equality between the produced files
  and the expected files. Scenario 13 additionally pins the literal
  expected tree string in the eval's `expected_output` field
  (including the Unicode box-drawing characters). For Scenario 16,
  the `expected-parent.md` covers the FINAL state after the canonical
  multi-op pipeline runs (decompose subsection + enrich Technical
  Notes + size line as first line) so the operation-ordering
  invariant is byte-level pinned. For Scenario 20, the
  `expected-parent.md` shows the existing `### Child tickets`
  subsection extended with newly-appended entries below the prior
  last-line entry. Scenarios 5a sub-scenarios (see below) reuse
  the Scenario 5 fixture and verify the proposal-iteration UX
  rather than the post-write state, so they do not need their own
  expected files.
- Scenario 3 — a ticket whose Requirements reference real codebase
  concepts; prompt includes SIMULATE blocks for both agents so the
  eval is deterministic
- Scenarios 1, 7, 15, 17 — no ticket fixture needed (Scenario 1 is
  bare invocation; Scenario 17 is a non-existent path; Scenarios 7
  and 15 are post-conditions verified across Scenarios 5/6/12/16
  fixtures and their expected child files)

All fixture paths referenced in `evals.json` are absolute paths
rooted at `${CLAUDE_PLUGIN_ROOT}`. Scenarios that simulate agent
results (3, 22) or simulate Write-tool interruption (23) encode
the simulation inline in the prompt field following the
create-ticket evals 9 and 14 pattern.

For Scenario 7 specifically, do NOT emit a single bundled "all nine
fields populated" assertion. Split the verification across the
decompose scenarios (5, 6, 16) using per-field byte-level checks
against the checked-in `expected-child-N.md` files — each field's
derivation rule (per Scenario 7) gets its own attribute on each
child file, catching per-field regressions cleanly.

### Success Criteria

#### Automated Verification:

- [ ] Skill directory created:
  `test -d skills/tickets/refine-ticket`
- [ ] SKILL.md exists:
  `test -f skills/tickets/refine-ticket/SKILL.md`
- [ ] Evals file exists:
  `test -f skills/tickets/refine-ticket/evals/evals.json`
- [ ] Benchmark file exists:
  `test -f skills/tickets/refine-ticket/evals/benchmark.json`
- [ ] All 36 evals pass 100% in `benchmark.json`
  (`run_summary.with_skill.pass_rate.mean == 1.0`)
- [ ] Phase 1–5 regression suite passes: `mise run test`
- [ ] Skill frontmatter matches specification:
  `grep -E '^(name|description|argument-hint|disable-model-invocation|allowed-tools):' skills/tickets/refine-ticket/SKILL.md` matches the spec field-for-field
- [ ] Frontmatter field ORDER is correct — the grep above emits the
  five keys in the exact order name, description, argument-hint,
  disable-model-invocation, allowed-tools
- [ ] Seven-agent fallback paragraph present: grep confirms all seven
  `accelerator:*` agent names appear in the fallback block
- [ ] Preamble present: grep confirms the three `config-read-*.sh`
  bang-executions and `config-read-template.sh ticket`
- [ ] Allowed-tools includes both `scripts/config-*` and
  `skills/tickets/scripts/*` patterns (grep-verifiable)
- [ ] No new script added to `skills/tickets/scripts/`:
  `ls skills/tickets/scripts/*.sh | wc -l` matches Phase 5 count
- [ ] No change to `templates/ticket.md`:
  `git diff --exit-code HEAD -- templates/ticket.md` (or jj equivalent)

#### Manual Verification:

- [ ] Run `/refine-ticket` with no arguments — verify it prompts for
  a path and does nothing else
- [ ] Run `/refine-ticket @meta/tickets/NNNN-*.md` on a real epic —
  select decompose, verify 2–5 children proposed, approve, verify
  all children written with all nine frontmatter fields, parent
  updated with child links, hierarchy tree displayed
- [ ] On a real story with a vague acceptance criterion, select
  sharpen — verify the criterion is rewritten in place via `Edit`
  and other criteria unchanged
- [ ] On a real ticket, select enrich — verify Technical Notes
  contains specific `path:line` references from the codebase; verify
  Requirements and Acceptance Criteria are unchanged
- [ ] On a ticket with existing Technical Notes, select enrich —
  verify the skill asks about replace/append/skip (no blind
  duplication)
- [ ] On a ticket with empty Dependencies, select link — verify
  proposed entries reference only real ticket numbers
- [ ] After any operation, verify the skill offers to run
  `/review-ticket` but does NOT invoke it automatically
- [ ] Verify decomposition consumes ticket numbers eagerly — after
  decompose, `ticket-next-number.sh` (called directly for a diagnostic
  check) returns a number strictly greater than the highest child
  number just written

---

## Subphase 6C: Structural Eval Validation in `mise run test`

### Overview

Add a lightweight structural check to `tasks/test.py` that validates
the shape of every `evals/evals.json` + `evals/benchmark.json` pair
shipped across the plugin. This does NOT run evals — it guards against
stale or truncated benchmark files, which are the most common silent
regression for skills whose evals are not re-run under CI.

### Changes Required

#### 1. Add `test-evals-structure.sh`

**File**: `scripts/test-evals-structure.sh` — placed alongside the
existing `scripts/test-config.sh` to match the convention used by
`test-adr-scripts.sh`, `test-lens-structure.sh`, and
`test-boundary-evals.sh` (locate them via `ls scripts/test-*.sh`
to confirm before implementing).

For each `skills/**/evals/evals.json` file:

- Assert a sibling `benchmark.json` exists
- Parse both; assert every scenario `name` present in `evals.json`
  also appears in `benchmark.json`
- Assert `benchmark.json.run_summary.with_skill.pass_rate.mean`
  equals `1.0` (the committed-benchmark invariant)
- Exit non-zero with a clear diagnostic on any failure

**Precondition**: before wiring this script into `mise run test`,
run it once across every existing committed benchmark (the five
Phase 2–3 ticket skills and five Phase 4–5 lenses) to verify the
`pass_rate.mean == 1.0` invariant currently holds. If any benchmark
is below 1.0 due to JSON serialisation noise (e.g. `0.999...`) or a
deliberately-failing scenario, either regenerate the benchmark or
soften the invariant (e.g. `>= 0.9999`) and document the choice in
the script comments. Do not commit the wiring change until this is
green.

#### 2. Add self-test fixtures and meta-test

**Directory**: `scripts/test-evals-structure-fixtures/`

Check in fixture pairs that the script must classify correctly:

- `valid-pair/evals.json` + `valid-pair/benchmark.json` — both
  well-formed, `pass_rate.mean == 1.0`, scenario names match.
  Script must exit 0.
- `missing-benchmark/evals.json` (no benchmark.json sibling).
  Script must exit non-zero with a clear "missing benchmark.json"
  diagnostic.
- `scenario-name-mismatch/evals.json` listing scenarios `a, b, c`
  + `benchmark.json` listing only `a, b`. Script must exit
  non-zero naming the missing scenario.
- `low-pass-rate/evals.json` + `benchmark.json` with
  `pass_rate.mean: 0.83`. Script must exit non-zero naming the
  observed pass rate.
- `malformed-json/evals.json` containing invalid JSON. Script
  must exit non-zero with a parse-error diagnostic.

**File**: `scripts/test-evals-structure-self.sh`

For each fixture directory above, invoke
`test-evals-structure.sh --fixture-root <dir>` (or an equivalent
override mechanism) and assert exit code matches expectation.
Additionally, drive `test-hierarchy-format.sh` against its three
fixtures (`matched-fences`, `mismatched-fences`, `missing-marker`
under `scripts/test-hierarchy-format-fixtures/`) and assert exit
codes. Exit non-zero on any mismatch.

#### 3. Add `test-hierarchy-format.sh` (cross-SKILL.md drift check)

**File**: `scripts/test-hierarchy-format.sh`

Extract the canonical hierarchy tree fence from
`skills/tickets/list-tickets/SKILL.md` (the example added in
Subphase 6.0) and from `skills/tickets/refine-ticket/SKILL.md`
(Step 5). Diff the two; exit non-zero with a clear diagnostic if
they differ.

Both files use HTML-comment markers `<!-- canonical-tree-fence -->`
and `<!-- /canonical-tree-fence -->` to bracket the canonical
example. Extraction is therefore a deterministic awk/sed pattern:
for each file, capture the lines strictly between the two markers.
Compare the two extracted blocks byte-for-byte; on mismatch print
both blocks and the diff, then exit non-zero. On either marker
missing or extracted block empty, exit non-zero with a "marker
missing or empty extraction" diagnostic — never treat this as
"no diff".

This is a one-line invariant that catches any future drift
between the two SKILL.md files' tree examples without relying on
manual smoke or eval re-running.

**Self-tests**: in addition to running against the live SKILL.md
files, the meta-test script (§2) drives `test-hierarchy-format.sh`
against three fixtures:
- `matched-fences/` — two synthetic markdown files with identical
  marker-bracketed fences. Script must exit 0.
- `mismatched-fences/` — two synthetic markdown files with
  marker-bracketed fences that differ by one character. Script
  must exit non-zero with the diff in output.
- `missing-marker/` — one of the two files lacks the closing
  marker. Script must exit non-zero with the marker-missing
  diagnostic.

#### 4. Wire all three into `tasks/test.py`

Add `test-evals-structure-self.sh` (runs first; verifies the
validator works), `test-evals-structure.sh` (runs second; the
real check across committed benchmarks), and
`test-hierarchy-format.sh` (runs third; the cross-SKILL.md drift
check) to the list of scripts invoked by `mise run test`,
alongside `test-config.sh`, `test-adr-scripts.sh`,
`test-ticket-scripts.sh`, `test-lens-structure.sh`,
`test-boundary-evals.sh`.

### Success Criteria

#### Automated Verification:
- [ ] `test -x scripts/test-evals-structure.sh`
- [ ] `test -x scripts/test-evals-structure-self.sh`
- [ ] `test -x scripts/test-hierarchy-format.sh`
- [ ] `test -d scripts/test-evals-structure-fixtures`
- [ ] `test -d scripts/test-hierarchy-format-fixtures`
- [ ] All five evals-structure fixture directories present:
  `for d in valid-pair missing-benchmark scenario-name-mismatch low-pass-rate malformed-json; do test -d scripts/test-evals-structure-fixtures/$d || echo MISSING $d; done`
- [ ] All three hierarchy-format fixture directories present:
  `for d in matched-fences mismatched-fences missing-marker; do test -d scripts/test-hierarchy-format-fixtures/$d || echo MISSING $d; done`
- [ ] `mise run test` invokes all three new scripts
- [ ] `test-evals-structure-self.sh` passes (validator behaves
  correctly on every fixture)
- [ ] `test-evals-structure.sh` passes for every existing skill with
  an `evals/` directory (precondition verified separately before
  wiring; see §1)
- [ ] `test-hierarchy-format.sh` passes (list-tickets and
  refine-ticket tree fences match byte-for-byte)

---

## Testing Strategy

### Eval Coverage (per skill)

Evals are the TDD specification. They are written first in this plan,
then passed to `/skill-creator:skill-creator` with the skill
specification. `/skill-creator:skill-creator` runs each eval against
the evolving SKILL.md draft, iterates prose until all evals pass, and
records outcomes in `benchmark.json`.

Coverage targets:

- **`stress-test-ticket`**: 15 evals covering invocation surface (1–2,
  14), conversational discipline (3–4), interrogation quality (5–9),
  termination and summary (10–11), edit behaviour (12–13, 15)
- **`refine-ticket`**: 36 evals across 25 scenario blocks covering
  invocation surface (1–3, 17, 18), decompose (5, 5a-{approve, edit,
  drop, add, regenerate, cancel, unknown, legend}, 6, 7, 12, 13, 19,
  20, 23), enrich (8, 14, 22), sharpen (9, 21), size (10, 10a),
  link (11, 11a, 11b, 11c), multi-operation order (16), Edit-failure
  recovery (24a, 24b), menu (4), post-refinement offer (15)

### Regression Guarantee

Both subphases must leave these unchanged:

- `templates/ticket.md`
- All files under `skills/tickets/scripts/`
- All files under `skills/review/lenses/`
- All files under `skills/review/output-formats/`
- All files under `skills/tickets/{create-ticket,extract-tickets,update-ticket,review-ticket}/` (`list-tickets/SKILL.md` is modified by Subphase 6.0 only; the modification is confined to §"Hierarchy Rendering" and must not touch any other section)
- `scripts/config-read-review.sh`
- `tasks/test.py` (modified by Subphase 6C only, to add the new
  `test-evals-structure.sh` invocation; existing invocations must
  remain unchanged)

Verification: run `mise run test` at the end of each subphase. All
pre-existing suites (`test-config.sh`, `test-adr-scripts.sh`,
`test-ticket-scripts.sh`, `test-lens-structure.sh`,
`test-boundary-evals.sh`) must pass with no changes to their expected
counts or fixtures. After Subphase 6C, the new
`test-evals-structure.sh` is added to the suite and must also pass.

### Manual Integration Smoke (end of Phase 6)

After both subphases are complete, run an end-to-end smoke sequence
on a fresh ticket:

1. `/create-ticket` — draft a new epic
2. `/review-ticket <path>` — verify lenses report on the draft
3. `/stress-test-ticket <path>` — interactive examination, edit
   where appropriate
4. `/refine-ticket <path>` — decompose → enrich → sharpen on the
   children
5. `/list-tickets hierarchy under <epic-id>` — verify the tree
   renders correctly
6. `/review-ticket <child-path>` — verify a refined child passes
   review

This exercises the full interactive-quality loop against the Phase 4
review infrastructure.

## References

- Research document:
  `meta/research/2026-04-08-ticket-management-skills.md`
  (section `#### Phase 6: Interactive Quality`)
- Phase 2 plan (create-ticket + extract-tickets TDD pattern):
  `meta/plans/2026-04-19-ticket-creation-skills.md`
- Phase 3 plan (list-tickets + update-ticket):
  `meta/plans/2026-04-21-list-and-update-tickets.md`
- Phase 4 plan (review-ticket + core lenses — establishes the TDD
  flow this plan follows):
  `meta/plans/2026-04-22-ticket-review-core.md`
- Phase 5 plan (extended lenses):
  `meta/plans/2026-04-24-ticket-review-extended-lenses.md`
- `stress-test-ticket` model:
  `skills/planning/stress-test-plan/SKILL.md`
- `refine-ticket` composition sources:
  `skills/tickets/create-ticket/SKILL.md` (agent spawning, frontmatter
  population),
  `skills/tickets/update-ticket/SKILL.md` (parent canonicalisation,
  targeted Edit),
  `skills/tickets/list-tickets/SKILL.md` (inline hierarchy rendering)
- Ticket numbering script:
  `skills/tickets/scripts/ticket-next-number.sh` (supports
  `--count N` batch allocation for decompose)
- Eval fixture pattern:
  `skills/tickets/create-ticket/evals/files/`
- Benchmark pass-rate convention:
  `skills/review/lenses/clarity-lens/evals/benchmark.json`
  (`run_summary.with_skill.pass_rate.mean`)
