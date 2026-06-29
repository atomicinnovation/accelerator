---
name: conduct-spike
description: Interactively conduct a time-boxed spike — collaboratively reduce
  uncertainty through discussion mixed with agent-driven research (and small
  throwaway prototypes where a question is empirical), then record the outcome
  on the spike's work item. Use when a work item or brief poses open questions
  that must be resolved before planning or implementation can proceed with
  confidence.
argument-hint: "[path to spike work item or brief, or work item number]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)
---

# Conduct Spike

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh conduct-spike`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:web-search-researcher, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work`

You are tasked with conducting a **spike**: a time-boxed, uncertainty-reducing
investigation. The spike is mostly conceptual, but a question is often best
answered by building a small throwaway prototype or test case. Your job is to
take an open, ambiguous brief and **iteratively reduce its uncertainty through
collaborative discussion** — backed by agent-driven research and, where needed,
small experiments — until the team can proceed with confidence, then record the
outcome.

This is **not** an interview and **not** a report you generate alone. It is a
**collaborative discussion**: you surface evidence, propose interpretations, and
the human steers — together you converge on answers. Lean on the research agents
to do the legwork so the conversation stays high-level.

## Initial Response

When this command is invoked:

1. **If a brief reference was provided**, resolve it in this order:

   - A **path** (e.g. `meta/work/0003-skill-evaluation-framework-selection.md`,
     or any arbitrary brief document) — read it.
   - A bare **work item number** (e.g. `0003` or `3`) — resolve against the
     work items directory shown above.
   - Otherwise, treat the provided text as an **inline brief** for the spike.
   - If a path or number was given but no file exists there, report
     "No spike brief at <path>" and stop — do not guess.

   Then read it FULLY (no limit/offset). If it is a work item with a non-empty
   `parent`, read the parent too. Proceed to the process below.

2. **If no reference was provided**, respond with:

   ```
   I'll run a spike with you — an interactive, time-boxed investigation to
   reduce the uncertainty until we can proceed with confidence.

   Point me at the spike: a path to the work item or brief, or a work item
   number. You can also paste the brief directly.

   Example: `/conduct-spike meta/work/0003-skill-evaluation-framework-selection.md`
   Or by number: `/conduct-spike 3`
   ```

   Then wait.

## Principles

Hold these throughout — they define the character of a spike:

- **Reduce uncertainty, don't just gather information.** Every move should
  shrink a specific unknown that is currently blocking a decision. Track what is
  still uncertain explicitly and watch it converge.
- **Cover all bases before concluding.** A spike ends with confidence, which
  means actively hunting for the blind spot — the option not considered, the
  assumption not verified, the question not yet asked — not just answering the
  questions already written down.
- **Discussion over interrogation.** Bring evidence and a proposed reading of
  it; invite the human to push back, reframe, or redirect. Resolve one thread to
  its conclusion before opening the next.
- **Self-answer through research; reserve the human for judgment.** If a
  question can be answered by an agent (the web, the codebase, our own
  documents) or by a quick experiment, answer it that way. Spend the human's
  attention on intent, priorities, trade-offs, and risk appetite.
- **Evidence is cited.** Findings carry their source — a URL, a `file:line`, a
  prototype result. Unsupported assertions are flagged as assumptions, not
  facts.
- **Respect the time-box.** Spikes are bounded (the brief usually states the
  box, e.g. "2 days"). Prioritise the highest-leverage unknowns; if the box
  forces unresolved questions, that is itself a finding to record.

## The Spike Process

### Step 1: Read the brief and absorb the frame

Read the brief (and parent) fully, then extract — for your own grounding, not as
a wall of text to the user:

- **The core questions / decision(s)** the spike must resolve.
- **The acceptance criteria** — what must be true for the spike to be "done".
- **The time-box** and any constraints.
- **Where the outcome should go.** Default is to append it to this work item
  (see Step 6). If the brief names a different or additional destination — an
  ADR to feed, a doc to write, a downstream work item to update — note that and
  honour it.

Also scan our own prior thinking early: spawn the **{documents locator agent}**
to find related research, notes, plans, or decisions in the meta directories, so
the spike builds on what we already know instead of rediscovering it.

### Step 2: Frame the uncertainty (collaborative checkpoint)

Distil the brief into an explicit **uncertainty register** — the set of open
questions the spike must close. For each, capture:

- **The question / unknown**, in plain language.
- **Why it matters** — which decision or acceptance criterion it gates.
- **Current confidence** — what we believe today and how firmly.
- **How we'd reduce it** — web research, codebase investigation, prior-art in
  meta, a small prototype/test, or a judgment call by the human.

Present this register to the user as the opening move and **collaborate to
sharpen it**: Have we framed the right questions? Is anything missing? What's
the riskiest unknown? Agree the starting order (highest leverage first) before
diving in. This first exchange is where you confirm you're solving the right
problem.

### Step 3: Reduce uncertainty iteratively (the core loop)

Work the register down, one thread at a time:

1. **Pick** the highest-leverage open question.
2. **Choose the instrument** that best reduces it (see "Choosing how to reduce a
   question" below) — often more than one in parallel.
3. **Investigate.** Spawn agents (concurrently when they're independent) and/or
   build a small prototype or test case. Give each agent a specific, focused,
   read-only brief — the agents know how to search; tell them *what* you need,
   not *how* to look.
4. **Bring the evidence into the discussion.** Summarise what was learned, cite
   sources, and propose what it implies. Surface tensions and surprises. Let the
   user react, decide, or redirect — this is the collaborative heart of the
   spike, not a status report.
5. **Update the register.** Mark questions resolved, adjust confidences, and
   **add any new unknowns** the findings surfaced (they often do). A spike that
   only ever shrinks its question list and never grows it probably isn't probing
   deeply enough.
6. **Repeat** until the register's open, decision-gating questions are closed or
   consciously deferred.

Run this as a genuine back-and-forth: short loops, visible evidence, the human
in the loop on every interpretation that affects the decision.

### Step 4: Cover all bases (blind-spot sweep)

Before converging, deliberately look for what you might have missed — this is
what turns "we answered the questions" into "we can proceed with confidence":

- **Unconsidered options.** Did we evaluate the real alternatives, or anchor on
  the first? Consider a fresh **{web search researcher agent}** pass framed to
  find approaches we haven't named.
- **Unverified assumptions.** Walk the brief's stated and implied assumptions —
  verify each against the codebase, the docs, or a quick test rather than
  trusting it.
- **Unused research modality.** Did we lean on one source and skip another that
  would see different failure modes (web vs. our own codebase vs. our own
  documents vs. an experiment)?
- **Untested empirical claims.** Anything asserted as "X works / behaves like Y"
  that we haven't actually run — prototype it.
- **Acceptance-criteria coverage.** Walk each criterion in the brief and confirm
  the spike now answers it.

Surface what the sweep turns up and feed anything material back into Step 3.

### Step 5: Converge on a decision

When uncertainty is acceptably low, synthesise with the user:

- The **recommendation / decision** (a choice, or a small set with a clear
  default), in the brief's own terms.
- The **rationale**, grounded in the evidence gathered (with links / `file:line`
  / prototype results).
- **What was surveyed / explored** — the options and angles considered, so the
  decision is auditable, not just asserted.
- **Residual risks and open questions** — what remains uncertain, and why it's
  acceptable to proceed anyway (or what would trigger revisiting).
- **Implications** — what this unblocks and any follow-on work it implies.

Use the `AskUserQuestion` tool to confirm the synthesis with the user before
recording it, with two options:

1. **Yes, record this synthesis** — write the outcome to the spike document
2. **No, revise first** — adjust the synthesis before recording

### Step 6: Record the outcome

**Default: append the outcome to the work item** (the brief overrides this only
if it explicitly names another destination — honour additions like feeding an
ADR or updating a downstream item *as well*).

- Use `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` to obtain the
  `Current Date/Time (UTC):` value and the resolved author. Run the bare path
  **directly** as an executable; never prefix it with `bash`/`sh`/`env` (a
  wrapper prefix escapes the skill's `allowed-tools` permission and forces an
  unnecessary prompt).
- With the Edit tool, add (or update) outcome sections in the work item body.
  Match the section name the brief asks for — e.g. if the acceptance criteria
  call for a **Recommendation** section, write exactly that. A typical shape:

  - `## Spike Outcome` — date, time spent vs. box, and a one-line verdict.
  - `## Recommendation` (or `## Findings` + `## Recommendation`) — the decision
    and rationale, the options surveyed, and the evidence/links.
  - `## Residual Risks & Open Questions` — what remains, and the trigger to
    revisit.

- Update **only** the frontmatter fields `last_updated` (to the UTC value above)
  and `last_updated_by` (to the resolved author). **Do not change `status`,
  `priority`, or other lifecycle frontmatter** — those transitions belong to
  `/update-work-item`. Likewise leave the body `**Status**:` / `**Priority**:`
  labels alone.
- If an Edit target can't be matched (the file differs from what you read),
  abort that specific edit with a clear diagnostic and continue with the rest.

If the spike worked from an **inline brief** with no work item, write the
outcome to a research document in the configured research directory instead, and
tell the user where it went.

### Step 7: Present and hand off

- Confirm where the outcome was recorded.
- Give a concise summary: the decision, the key evidence, and the residual
  risks.
- Note the natural next step (e.g. "this work item is ready to move to `ready`
  via `/update-work-item`", or "this feeds ADR …"), but don't take lifecycle
  actions yourself.

## Choosing how to reduce a question

Match the instrument to the unknown — and run independent investigations in
parallel:

- **External / conceptual unknowns** (how does this technology work, what are
  the options, what's current best practice, what are the trade-offs) → the
  **{web search researcher agent}**. This is the workhorse of a conceptual
  spike. Instruct it to return **links** with its findings, and carry those
  links into the outcome.
- **"Where does X live / does our code already do Y?"** → the **{codebase
  locator agent}** to find it, then the **{codebase analyser agent}** to
  understand how it actually works.
- **"Is there an existing pattern we'd follow?"** → the **{codebase pattern
  finder agent}** for concrete examples.
- **"Have we already thought about this?"** → the **{documents locator agent}**
  to discover relevant meta documents, then the **{documents analyser agent}**
  to extract the substance from the most relevant ones.
- **Empirical / "does it actually behave this way?" unknowns** → build a small
  prototype or test case (see below). Don't speculate when you can measure.
- **Intent / priority / trade-off / risk-appetite questions** → ask the human.
  These are the questions agents can't answer.

## Building prototypes and test cases

When a question is empirical — does this API behave as documented, does this
approach compile, what's the rough performance, does this integration actually
work — get ground truth instead of reasoning in the abstract:

- Keep it **minimal and throwaway**: the smallest thing that answers the
  question. Build it in the scratchpad directory (or a clearly-marked spike
  scratch location), not in the production tree, unless the brief says the
  prototype is a deliverable.
- **Capture the result** — the command run, the output, the measurement — as
  evidence for the outcome.
- **Discard the code, keep the learning.** The artifact of a spike is the
  resolved uncertainty and the recorded decision, not the prototype.
- If a prototype reveals the real cost or risk is different from what the brief
  assumed, that's a primary finding — bring it back into the discussion.

## Important notes

- **The conversation is the deliverable's source — the record is its
  destination.** Don't silently research and dump a conclusion; reduce
  uncertainty *with* the user, then record what you concluded together.
- **Never conclude with placeholder values** in the recorded outcome, and never
  skip the recording step — an unrecorded spike is an unfinished spike.
- **Parallelise** independent agent investigations to keep the loop tight and
  the main context lean; keep yourself focused on synthesis, not deep file
  reading.
- **Prefer live evidence over our own historical documents** when they
  disagree; treat prior meta documents as context, not ground truth.
- **Read fully**: always read the brief and any directly referenced files with
  no limit/offset before spawning agents.
- **Edit conservatively** when recording: add the outcome and touch only
  `last_updated` / `last_updated_by`; leave every other field to its owning
  skill.
- **A deferred question is a finding.** If the time-box closes with unknowns
  open, record them explicitly with their risk — don't let them vanish.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh conduct-spike`
