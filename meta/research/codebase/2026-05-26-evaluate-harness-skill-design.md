---
date: 2026-05-26 11:58:22 CEST
author: accelerator
git_commit: b06116de8046a132fca7ae091de5ec2390d090f5
branch: main
repository: accelerator
topic: "Evaluate Harness Skill — Design Research"
tags: [research, skill-design, harness-evaluation, claude-md, agents-md, config]
status: complete
last_updated: 2026-05-26
last_updated_by: accelerator
last_updated_note: "Revised design: individual lens sub-agents (consistent with review-pr pattern); dimensions renamed to lenses"
---

# Research: Evaluate Harness Skill — Design Research

**Date**: 2026-05-26 11:58:22 CEST
**Author**: accelerator
**Git Commit**: b06116de8046a132fca7ae091de5ec2390d090f5
**Branch**: main
**Repository**: accelerator

## Research Question

How should a new `evaluate-harness` skill be designed to analyse a user's
coding harness configuration (CLAUDE.md or AGENTS.md), give per-aspect
quantitative scores (0–10), and produce prioritised improvement
recommendations?

## Summary

The skill should be placed at `skills/config/evaluate-harness/` and follows
the same orchestrator + individual lens sub-agent pattern as `review-pr`. The
orchestrator detects the harness file, selects applicable lenses, spawns them
in parallel via the reviewer agent, aggregates their JSON results, and writes
a structured evaluation artifact to `meta/harness-evaluations/`.

Each lens is its own `SKILL.md` under
`skills/config/evaluate-harness/lenses/` and can be invoked standalone.
Lenses split into two kinds:

**Live lenses** (empirical): Read the relevant command from the harness, run
it against clean code, introduce a deliberate targeted failure, run again, and
score the quality of both outputs. If no command is documented → score 0, no
execution. Require broad `Bash` access including `git restore` for mutation
cleanup.

**Static lenses** (document analysis): Score the harness document directly —
no command execution, no `Bash` needed beyond reading files.

The skill requires one new path key (`paths.harness_evaluations` →
`meta/harness-evaluations`) in `config-defaults.sh`.

## Detailed Findings

### 1. Where "Harness" Lives in This Project

The term *harness* appears in two overlapping senses in this codebase:

**Sense A — CLAUDE.md / AGENTS.md** (what the new skill evaluates):
The file that governs how a coding agent approaches a project — coding
conventions, test runners, workflow rules, architectural constraints. This is
what a developer writes once and updates over time. Research document
`2026-03-15-context-management-approaches.md` is the canonical best-practice
reference for this sense.

**Sense B — Loop harness** (from the harness-engineering talk):
The `while :; do cat PROMPT.md | claude-code; done` pattern (the "Ralph loop")
where the conversation itself is disposable and the artifacts are permanent.
This sense is more advanced and is out of scope for the new skill.

The skill targets **Sense A**: evaluating and improving the CLAUDE.md /
AGENTS.md file.

### 2. Evaluation Lenses

> **To be defined by the plan.** Individual lenses, their scoring rubrics, and
> the live vs. static split will be worked out lens-by-lens during planning.

### 2a. Lens Evaluation Guideline Format

Each lens SKILL.md contains a structured set of evaluation guidelines that the
lens agent uses when scoring. Guidelines are split into two top-level sections
— **GOOD** and **BAD** — so the lens evaluates from both directions: what
signals raise the score and what signals lower it. Each section is subdivided
by urgency using RFC-style keywords:

- **MUST** — the criterion has the highest impact on score; presence (GOOD) or
  absence (BAD) of this alone can dominate the lens result
- **SHOULD** — meaningful impact; not a dealbreaker but a clear improvement or
  concern
- **COULD** — minor refinement; worth noting in recommendations but does not
  materially change the score

Each individual guideline point is structured with two fields:

- **WHAT**: a concrete, observable description of the signal — what to look
  for in the harness or in the command output
- **WHY**: the reason this signal matters — the downstream impact on agent
  effectiveness, context quality, or verification capability

The WHY field is critical: it allows the skill to generate specific,
well-reasoned suggestions rather than bare observations. When the lens
identifies a BAD MUST violation, it can surface the WHY directly as the
rationale in the recommendation.

**Example guideline block (illustrative — not a final lens):**

```markdown
## GOOD

### MUST
- **WHAT**: A test command is explicitly documented with the exact invocation
  (e.g. `npm test`, `pytest`, `go test ./...`)
  **WHY**: Without a known command the agent cannot run tests autonomously,
  breaking the verification loop entirely

### SHOULD
- **WHAT**: A single-test invocation pattern is documented alongside the
  full-suite command
  **WHY**: Allows the agent to re-run only the affected test after a fix,
  dramatically reducing iteration time

### COULD
- **WHAT**: A watch-mode command is documented
  **WHY**: Useful for long-running sessions but not required for the core
  verification loop

## BAD

### MUST
- **WHAT**: No test command is present anywhere in the harness
  **WHY**: The agent has no way to verify correctness; it must ask the
  developer, interrupting the autonomous loop

### SHOULD
- **WHAT**: The test command is documented but produces more than ~50 lines of
  output on a passing run
  **WHY**: Noisy passing output makes it hard for the agent to confirm the
  green state confidently, increasing the risk of misreading results

### COULD
- **WHAT**: The test command does not include a flag to suppress colour codes
  or progress spinners
  **WHY**: ANSI escape sequences in captured output can obscure the signal;
  minor but worth noting
```

This format decouples the evaluation criteria from the scoring rubric —
the lens agent reads the guidelines, applies them to what it observes, and
derives a score. The plan will define the full guideline sets for each lens.

### 3. Overall Score Calculation

> **To be defined by the plan.** Weights, formula, and interpretation bands
> will be determined once all lenses are agreed.

### 4. Mutation Strategy

> **To be defined by the plan.** The safe mutation/restore approach for live
> lenses will be specified during planning.

### 5. Harness File Detection Logic

> **To be defined by the plan.** Detection order (CLAUDE.md, AGENTS.md, global
> config, etc.) and missing-file behaviour will be specified during planning.

### 6. Skill Architecture

`evaluate-harness` is an **orchestrator** that spawns individual lens
sub-agents in parallel, exactly as `review-pr` does. Each lens is its own
`SKILL.md` and can be invoked standalone.

**Directory layout:**
```
skills/config/evaluate-harness/
├── SKILL.md                          ← orchestrator
└── lenses/
    ├── <lens-name>-lens/
    │   └── SKILL.md                  ← individual lens (one per lens)
    └── ...
```

**Pattern to follow**: `skills/github/review-pr/SKILL.md` for the
orchestrator; `skills/review/lenses/` for the individual lens SKILL.md
structure.

**Orchestrator frontmatter** — the orchestrator only needs config scripts and
the artifact helper; Bash execution of user commands happens inside lens
sub-agents:
```yaml
---
name: evaluate-harness
description: Evaluate the project's coding harness (CLAUDE.md / AGENTS.md)
  through parallel lens sub-agents. Each lens scores a specific aspect 0–10
  and returns structured JSON. Produces an aggregated scorecard with
  prioritised recommendations. Use when the user wants to assess or improve
  their AI-coding configuration.
argument-hint: "[optional: focus on specific lens]"
disable-model-invocation: true
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)
---
```

**Live lens frontmatter** — live lenses need broad Bash access to run the
user's commands and restore mutated files:
```yaml
---
name: <lens-name>-lens
user-invocable: false
disable-model-invocation: true
allowed-tools:
  - Bash(git status *)
  - Bash(git restore *)
  - Bash(*)
---
```

**Static lens frontmatter** — static lenses need no Bash:
```yaml
---
name: <lens-name>-lens
user-invocable: false
disable-model-invocation: true
---
```

**`!` directives at the top of the orchestrator**:
```
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh evaluate-harness`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
**Harness evaluations directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh harness_evaluations`
```

This requires adding to `config-defaults.sh`:
- Path key: `"paths.harness_evaluations"` → default `"meta/harness-evaluations"`

### 7. Skill Process Steps

**Step 1 — Detect and read the harness file(s)**
- Look for `CLAUDE.md`, `AGENTS.md`, `~/.claude/CLAUDE.md`, `.accelerator/config.md`
- Read all found files fully (no limit/offset) in the main orchestrator context
- Note approximate instruction count and presence of child-directory CLAUDE.md files

**Step 2 — Parse commands and select lenses**
- Extract documented commands (test, lint, type-check, etc.) from the harness
- Determine which live lenses are applicable (command present) vs. score 0
  (command absent)
- All static lenses are always applicable
- Present lens selection to the user before spawning (mirroring review-pr)

**Step 3 — Spawn all lens agents in parallel**
- Spawn one reviewer agent per selected lens via the Task tool
- Pass the harness file content and any extracted commands directly in each
  agent's prompt (sub-agents have no implicit context sharing)
- Wait for ALL lens agents to complete before proceeding

**Step 4 — Aggregate results**
- Parse the JSON block from each lens agent's response
- Apply the fallback strategy for malformed output (treat as a single major
  finding, as review-pr does)
- Compute the overall weighted score
- Identify cross-cutting themes (issues flagged by multiple lenses)

**Step 5 — Generate recommendations**
- Produce at most 5 prioritised, concrete, actionable recommendations
- Rank by ROI: live lens gaps first (they directly block the verification
  loop), then static lens improvements

**Step 6 — Gather metadata and write evaluation artifact**
- Run `artifact-derive-metadata.sh`
- Write to `{harness evaluations directory}/YYYY-MM-DD-harness-evaluation-N.md`
  (N increments; glob for existing files to find next N)

**Step 7 — Present results and offer actions**
- Show the per-lens scorecard
- Show overall score with interpretation band
- Show the top recommendations
- Offer to apply specific recommendations (edit CLAUDE.md) or re-run
  individual lenses with adjusted focus

### 8. Output Format

> **To be defined by the plan.** The evaluation artifact template, scorecard
> layout, and per-lens result structure will be specified during planning.

### 9. Skill Category and Location

Place the skill under `skills/config/` alongside other config-related skills
(`browser-executor`, `configure`, `init`, `migrate`, `paths`).

**Final path**: `skills/config/evaluate-harness/SKILL.md`

**Evals**: Create `skills/config/evaluate-harness/evals/` with at least:
- A fixture for a well-configured harness with good tooling output (expected
  overall ≥8.0)
- A fixture for a harness missing all commands (expected overall ≤2.0)
- A fixture for a harness with commands documented but a noisy/unhelpful
  toolchain (expected overall ~4–5 despite having commands)

The third fixture is the most important — it validates that the skill correctly
distinguishes *documented* from *effective*.

### 10. Required Infrastructure Changes

One addition to `scripts/config-defaults.sh`:

```bash
# In PATH_KEYS array, add:
"paths.harness_evaluations"

# In PATH_DEFAULTS array, add (same index):
"meta/harness-evaluations"
```

No changes needed to any other script — `config-read-path.sh` will
automatically resolve `harness_evaluations` once the default is registered.

## Code References

- `skills/config/configure/SKILL.md` — sibling skill for placement context
- `skills/config/init/SKILL.md` — another config-category sibling
- `skills/github/review-pr/SKILL.md` — orchestrator pattern to model
- `skills/review/lenses/` — individual lens SKILL.md structure to model
- `scripts/config-defaults.sh:27-43` — PATH_KEYS and PATH_DEFAULTS arrays
- `scripts/config-read-path.sh` — path resolution used in `!` directives
- `scripts/artifact-derive-metadata.sh` — metadata for artifact frontmatter
- `meta/research/codebase/2026-03-15-context-management-approaches.md` — canonical CLAUDE.md best-practice reference used as the scoring basis

## Architecture Insights

**Why empirical evaluation, not static analysis**: A harness that documents
`npm test` but whose test output is a wall of noisy logs scores the same as a
harness with tight, actionable output under a static approach — both just
"have a test command". The empirical approach distinguishes them correctly.
This is analogous to how the review-pr skill distinguishes between a PR that
passes CI and one that is actually correct — presence of the mechanism is
not the same as quality of the mechanism.

**Why individual lens sub-agents**: Consistency with the established lens
pattern in this repo (`review-pr`, `review-plan`, `review-work-item`) takes
priority over the marginal efficiency gain of running everything in the main
context. Each lens is then independently invocable, testable with its own
evals, and composable into other future skills. The orchestrator stays clean
and the Bash access boundary is correctly scoped — only live lens agents need
broad Bash; static lens agents need none.

**Why `git restore` over file-content save/restore**: `git restore` is
atomic and guaranteed — it does not depend on the skill having saved the
original content correctly. The pre-flight `git status` check ensures the
file is in a known state before mutation. Writing back saved content
introduces a risk of writing stale content if the skill re-reads the file
between save and restore.

**Why a new path key**: Harness evaluations are a distinct artifact type from
codebase research. Mixing them under `meta/research/codebase/` would pollute a
directory that developers use to find investigative research documents. A
dedicated `meta/harness-evaluations/` directory makes the artifact type
discoverable and separable.

**Scoring rationale**: Live lenses use averaged sub-scores (pass quality +
failure quality) to capture both directions of feedback. A tool that is noisy
on success but clear on failure scores mid-range, not full marks — the agent
needs both to operate autonomously.

## Historical Context

- `meta/research/codebase/2026-03-15-context-management-approaches.md` —
  Primary source for CLAUDE.md best practices and impact multiplier hierarchy
- `meta/decisions/ADR-0001-context-isolation-principles.md` — establishes the
  ~150 instruction / ~120k token ceilings as architectural decisions
- `meta/talks/2026-05-22-context-and-harness-engineering.md` — harness
  engineering framing; defines the "harness" concept used in the skill name
- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md` —
  covers CLAUDE.md layered injection as a customisation strategy
- `meta/research/codebase/2026-05-07-research-issue-documentation-gaps.md` —
  Documents the full set of files that must be updated when shipping a new
  skill: `README.md` (meta directory table + narrative), `CHANGELOG.md` (new
  entry under `## [Unreleased] > ### Added`), and `scripts/test-config.sh`
  (skill arrays). `plugin.json` requires no change — category directories are
  auto-discovered. The plan for `evaluate-harness` must include all of these.

## Open Questions

- Should the skill also evaluate `~/.claude/settings.json` / hooks
  configuration as a separate optional dimension? Hooks (PreCompact, etc.) are
  a high-ROI technique per the research but are distinct from CLAUDE.md content.
- Should the skill offer to auto-apply the top recommendations (edit CLAUDE.md),
  or remain read-only by default? Given that CLAUDE.md is a critical file with
  outsized downstream impact, read-only-by-default / opt-in-edit (as `review-pr`
  does) is the safer pattern.
- What is the right evaluation cadence recommendation? Monthly review of the
  harness seems appropriate — toolchain outputs change as projects evolve.
- Should the failure injection be configurable? Some teams may prefer the
  mutation to target a specific file or test; a `.accelerator/skills/evaluate-
  harness/context.md` could specify this via the existing per-skill context
  mechanism.
