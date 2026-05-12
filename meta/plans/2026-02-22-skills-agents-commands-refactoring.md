# Skills, Agents, and Commands Refactoring

## Overview

Refactor the Claude Code configuration to make best use of the skills system:
migrate all 8 commands to skills and collapse 12 review agents into 1 generic
agent + 6 lens skills + 2 output format references using path passing at
spawn time.

## Current State

- 8 commands in `commands/` (single-file, legacy format)
- 12 review agents (6 PR + 6 plan) with significant duplication
- 6 utility agents (correctly agents, no change needed)
- 0 skills

## Desired End State

- 0 commands (all migrated to skills)
- 1 generic `reviewer` agent (replaces 12 review agents)
- 6 utility agents (unchanged)
- 16 skills:
  - 8 user-invocable (migrated from commands)
  - 6 lens skills (not user-invocable)
  - 2 output format references (not user-invocable)

**Verification**: Run `/review-pr` and `/review-plan` against real targets
and confirm:
- All lenses produce valid JSON output
- The orchestrator correctly reads lens skills and composes prompts
- Review quality is comparable to the current agent-based approach
- Collaborative iteration still works for plan review

## What We're NOT Doing

- Changing the utility agents (codebase-locator, etc.) -- they're correctly
  agents
- Changing the review orchestration logic (steps, verdict, deduplication) --
  only how agents are spawned
- Adding new lenses or changing lens-specific domain expertise
- Creating a plugin -- all skills stay in `~/.claude/skills/`

## Implementation Approach

Four phases, executed sequentially:

1. **Phase 1**: Migrate commands to skills
2. **Phase 2**: Create review lens skills, output format references, and
   generic reviewer agent
3. **Phase 3**: Update review orchestrators to use path passing
4. **Phase 4**: Clean up old files and verify

---

## Phase 1: Migrate Commands to Skills

### Overview

Convert all 8 commands from `commands/*.md` to `skills/*/SKILL.md`. This is
mostly mechanical: move content, update frontmatter, add skill-specific
features where beneficial.

### Changes Required

For each of the 8 commands, create a skill directory and SKILL.md file:

| Command | Skill Directory | Key Additions |
|---------|----------------|---------------|
| `commit` | `skills/commit/` | `argument-hint`, dynamic context injection |
| `create-plan` | `skills/create-plan/` | `argument-hint` |
| `describe-pr` | `skills/describe-pr/` | `argument-hint` |
| `implement-plan` | `skills/implement-plan/` | `argument-hint` |
| `research-codebase` | `skills/research-codebase/` | `scripts/`, `references/` |
| `review-plan` | `skills/review-plan/` | `argument-hint` |
| `review-pr` | `skills/review-pr/` | `argument-hint` |
| `validate-plan` | `skills/validate-plan/` | `argument-hint` |

#### Migration Pattern (applies to all 8)

For each command:

1. Create directory: `skills/<command-name>/`
2. Create `SKILL.md` with updated frontmatter:

   ```yaml
   ---
   name: <command-name>
   description: <derive from command content -- what it does and when to use>
   argument-hint: <where applicable>
   disable-model-invocation: true
   ---
   ```

3. Move the command's markdown content into the SKILL.md body
4. Replace `$ARGUMENTS` usage: if the command references arguments via the
   `ARGUMENTS:` appendage pattern, consider using `$ARGUMENTS` inline
5. For commands that reference external files (scripts, templates), consider
   bundling them as `scripts/` or `references/` subdirectories

**IMPORTANT**: Do NOT delete the old command files during this phase. Both
commands and skills can coexist, with skills taking precedence. The old
command files serve as a fallback during testing and will be deleted in
Phase 4 after full verification. Similarly, the old review agent files in
`agents/` must remain until Phase 4 -- the migrated `review-pr` and
`review-plan` skills will reference them until Phase 3 updates the spawning
pattern.

#### Command-Specific Notes

**commit**:
- Add `argument-hint: [optional message or flags]`
- Add dynamic context at the top of the skill body:
  ```
  ## Current State
  - Staged changes: !`git diff --cached --stat`
  - Recent commits: !`git log --oneline -5`
  ```
- This gives the skill immediate context without Claude needing to run
  commands first

**create-plan**:
- Add `argument-hint: [ticket reference or description]`
- Consider extracting the plan template into
  `references/plan-template.md` if the command contains one

**describe-pr**:
- Add `argument-hint: [PR number or URL]`
- Note: Dynamic context injection using `!`command`` is not practical here
  because `$ARGUMENTS` passes the full argument string and may not resolve
  cleanly as a PR identifier in shell commands. Keep the current approach
  where Claude runs `gh pr view` and `gh pr diff` interactively as part of
  the skill's process steps.

**implement-plan**:
- Add `argument-hint: [path to plan file]`

**research-codebase**:
- Add `argument-hint: [research question]`
- Move `~/.claude/scripts/research-metadata.sh` to
  `skills/research-codebase/scripts/research-metadata.sh` and update the
  script path reference in the skill body accordingly
- Consider extracting the research document template into
  `references/research-template.md`

**review-plan**:
- Add `argument-hint: [path to plan file]`
- Content will be significantly updated in Phase 3

**review-pr**:
- Add `argument-hint: [PR number or URL]`
- Content will be significantly updated in Phase 3

**validate-plan**:
- Add `argument-hint: [path to plan file]`

### Success Criteria

#### Automated Verification:

- [x] All 8 `skills/*/SKILL.md` files exist
- [x] Each has valid YAML frontmatter with `name` and `description`
- [x] Each has `disable-model-invocation: true`

#### Manual Verification:

- [ ] Run `/commit`, `/create-plan`, `/describe-pr`, `/implement-plan`,
  `/research-codebase`, `/validate-plan` and verify they work as before
- [ ] Verify dynamic context injection works for `commit`

---

## Phase 2: Create Review Lens Skills and Generic Agent

### Overview

Create 6 lens skills that capture domain expertise, 2 output format
references, and 1 generic reviewer agent. This phase creates the new
infrastructure without changing the orchestrators (Phase 3 wires them up).

### Changes Required

#### 1. Create 6 Lens Skills

Each lens skill captures the domain expertise shared between the PR and plan
versions of that lens. The content is derived from the union of the current
PR and plan agent pairs, abstracting away PR-specific (diff, line numbers)
and plan-specific (plan sections, location) framing.

**File pattern**: `skills/<lens>-lens/SKILL.md`

For each lens, the SKILL.md should contain:

```yaml
---
name: <lens>-lens
description: <lens> review lens for evaluating <domain>. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---
```

Followed by:
- **Core Responsibilities**: 3 numbered areas of focus (derived from the
  shared expertise in the PR and plan agent pairs)
- **Key Evaluation Questions**: The sub-lens questions to apply to each
  component or change (from Step 3/4 of current agents)
- **Important Guidelines**: Lens-specific guidelines (not output format
  guidelines -- those go in the output format reference)
- **What NOT to Do**: Lens-specific restrictions (don't review other lenses)
- **Closing principle**: The "Remember:" paragraph

**Lens-specific content sources**:

| Lens | PR Agent Source | Plan Agent Source | Core Domain |
|------|----------------|-------------------|-------------|
| `architecture-lens` | `pr-architecture-reviewer.md` | `plan-architecture-reviewer.md` | Structural integrity, coupling, cohesion, evolutionary fitness, dependency direction |
| `security-lens` | `pr-security-reviewer.md` | `plan-security-reviewer.md` | STRIDE analysis, OWASP coverage, trust boundaries, secrets management, defence in depth |
| `test-coverage-lens` | `pr-test-coverage-reviewer.md` | `plan-test-coverage-reviewer.md` | Test strategy adequacy, test quality, test architecture, pyramid balance |
| `code-quality-lens` | `pr-code-quality-reviewer.md` | `plan-code-quality-reviewer.md` | Design principles, testability, error handling, observability, complexity |
| `standards-lens` | `pr-standards-reviewer.md` | `plan-standards-reviewer.md` | Project conventions, API standards, accessibility, documentation |
| `usability-lens` | `pr-usability-reviewer.md` | `plan-usability-reviewer.md` | Developer experience, API ergonomics, configuration, error experience, migration |

**Deriving lens content**: For each lens, read both the PR and plan agent
files. Extract:
1. Core Responsibilities -- take the union; most are identical or nearly so.
   Where they differ (e.g., PR says "changes" and plan says "proposed
   design"), generalise to cover both (e.g., "the code or design under
   review").
2. Sub-lens evaluation questions -- from the "Evaluate through each
   sub-lens" step. These are the same between PR and plan.
3. Guidelines -- take lens-specific guidelines (not output format
   guidelines like "anchor to diff lines" or "output only JSON").
4. What NOT to Do -- keep "don't review other lenses" and lens-specific
   items. Drop format-specific items.

**Merge principle**: Include all unique evaluation questions from both the
PR and plan variants. Where questions are conceptually equivalent but use
different framing (e.g., "changes" vs "proposed design"), use the more
general framing that covers both. Where a question is unique to one variant
(e.g., "Scalability & resilience: What happens under 10x load?" in the plan
agent only), include it -- the orchestrator's analysis strategy section
provides the context-specific framing, so the lens can be inclusive. Do not
drop domain-specific evaluation questions just because they appear in only
one variant.

#### 2. Create 2 Output Format References

**File**: `skills/pr-review-output-format/SKILL.md`

```yaml
---
name: pr-review-output-format
description: Output format specification for PR review agents. Defines the
  JSON schema, field reference, severity emoji prefixes, and comment body
  format for PR reviews.
user-invocable: false
disable-model-invocation: true
---
```

Content derived from the current `pr-architecture-reviewer.md` Output Format
section (representative of all 6 PR agents since they share the same
format). Includes:
- JSON schema with `comments` and `general_findings` arrays
- Field reference (path, line, end_line, side, severity, confidence, etc.)
- Multi-line comment API mapping note
- Severity emoji prefixes
- Comment body format template and example
- Diff anchoring guidelines
- "Output only the JSON block" instruction

**File**: `skills/plan-review-output-format/SKILL.md`

```yaml
---
name: plan-review-output-format
description: Output format specification for plan review agents. Defines the
  JSON schema, field reference, severity emoji prefixes, and finding body
  format for plan reviews.
user-invocable: false
disable-model-invocation: true
---
```

Content derived from the current `plan-architecture-reviewer.md` Output
Format section. Includes:
- JSON schema with `findings` array
- Field reference (location, severity, confidence, etc.)
- Severity emoji prefixes
- Finding body format template and example
- "Output only the JSON block" instruction

#### 3. Create Generic Reviewer Agent

**File**: `agents/reviewer.md`

```yaml
---
name: reviewer
description: Generic review agent that evaluates code or plans through a
  specific quality lens. Spawned by review orchestrators with lens-specific
  instructions and output format injected at spawn time.
tools: Read, Grep, Glob, LS
---
```

Body content establishes the reviewer's behavioural conventions as system
prompt invariants. These conventions are shared across ALL lenses and provide
a reliable backstop regardless of the task prompt's composition:

```markdown
You are a specialist reviewer. Your task instructions provide a review lens,
analysis strategy, and output format specification. Your job is to read those
materials, explore the codebase, and produce a structured JSON review.

## How You Work

1. **Read your instructions first**: Your task prompt contains paths to a
   lens skill file and an output format file. Read BOTH files before doing
   anything else. These contain your domain expertise and output
   specification.
2. **Follow the Analysis Strategy** provided in your task instructions
3. **Apply the domain expertise** from the lens skill to evaluate what
   you're reviewing
4. **Explore the codebase** using your available tools (Read, Grep, Glob,
   LS) to gather context relevant to your lens
5. **Return your analysis** as structured JSON following the output format
   specification

## Behavioural Conventions

These apply regardless of lens or review type:

- **Output only a JSON code block** — do not include additional prose,
  narrative analysis, or markdown outside the JSON code fence. The
  orchestrator parses your output as JSON.
- **Use severity emoji prefixes** — start each finding body with 🔴
  (critical), 🟡 (major), or 🔵 (minor/suggestion) followed by the lens
  name in bold
- **Make each finding body self-contained** — it will be presented
  alongside findings from other lenses without surrounding context.
  Include enough context for the finding to be understood on its own.
  Structure as: emoji + **Lens** + issue description + **Impact** +
  **Suggestion**
- **Rate confidence** on each finding — distinguish verified concerns
  (high) from potential issues (medium) and speculative observations (low)
- **Take time to ultrathink** about the implications of what you're
  reviewing
- **Be pragmatic** — focus on issues that matter, not theoretical
  perfection
- **Don't review outside your lens** — other lenses cover other concerns
```

### Success Criteria

#### Automated Verification:

- [x] All 6 `skills/*-lens/SKILL.md` files exist with valid frontmatter
- [x] Both `skills/pr-review-output-format/SKILL.md` and
  `skills/plan-review-output-format/SKILL.md` exist
- [x] `agents/reviewer.md` exists with valid frontmatter
- [x] Each lens skill contains Core Responsibilities and Key Evaluation
  Questions
- [x] Each lens skill has `user-invocable: false` and
  `disable-model-invocation: true`

#### Manual Verification:

- [ ] Spot-check 2 lens skills to confirm they capture the shared domain
  expertise from both the PR and plan agent pairs
- [ ] Verify the output format references contain the complete JSON schema,
  field reference, and body format
- [ ] Verify no PR-specific (diff, line numbers) or plan-specific (plan
  sections) framing leaked into the lens skills

---

## Phase 3: Update Review Orchestrators

### Overview

Update the `review-pr` and `review-plan` skills to spawn the generic
`reviewer` agent with path references to lens skills and output format files.
The orchestrator passes paths only -- the agent reads the files itself in its
own isolated context. This keeps the orchestrator's main context small
(~12 lines per lens vs ~800-1000 for content injection).

### Changes Required

#### 1. Update `review-pr` Skill

**File**: `skills/review-pr/SKILL.md` (created in Phase 1)

Replace Step 3 (Spawn Review Agents) with a path-passing pattern. The
orchestrator passes file paths to the generic reviewer agent -- it does NOT
read the lens skill or output format files itself. The agent reads them in
its own isolated context.

````markdown
### Step 3: Spawn Review Agents

For each selected lens, spawn the generic `reviewer` agent with a prompt
that includes paths to the lens skill and output format files. Do NOT read
these files yourself -- the agent reads them in its own context.

Compose each agent's prompt following this template:

```
You are reviewing pull request changes through the [lens name] lens.

## Context

The PR artefacts are in the temp directory at [path]:
- `diff.patch` — the full diff
- `changed-files.txt` — list of changed file paths
- `pr-description.md` — PR description
- `commits.txt` — commit messages

PR number: [number]

## Analysis Strategy

1. Read your lens skill and output format files (see paths below)
2. Read `diff.patch` and `changed-files.txt` from the temp directory
3. Read `pr-description.md` and `commits.txt` for intent context
4. Explore the codebase to understand the architectural landscape around
   the changes
5. Evaluate the changes through your lens, applying each key question
6. Identify beyond-the-diff impact — trace how changes affect consumers
7. Anchor findings to precise diff line numbers (lines must be within
   diff hunks)

## Lens

Read the lens skill at: ~/.claude/skills/[lens]-lens/SKILL.md

## Output Format

Read the output format at: ~/.claude/skills/pr-review-output-format/SKILL.md

IMPORTANT: Return your analysis as a single JSON code block. Do not include
prose outside the JSON block.
```

Spawn all selected agents **in parallel** using the Task tool with
`subagent_type: "reviewer"`.

**IMPORTANT**: Wait for ALL review agents to complete before proceeding.

**Handling malformed agent output**: [keep existing malformed output
handling from current command]
````

#### 2. Update `review-plan` Skill

**File**: `skills/review-plan/SKILL.md` (created in Phase 1)

Replace Step 3 (Spawn Review Agents) with the same path-passing pattern,
using plan-specific framing:

````markdown
### Step 3: Spawn Review Agents

For each selected lens, spawn the generic `reviewer` agent with a prompt
that includes paths to the lens skill and output format files. Do NOT read
these files yourself -- the agent reads them in its own context.

Compose each agent's prompt following this template:

```
You are reviewing an implementation plan through the [lens name] lens.

## Context

The implementation plan is at [path]. Read it fully.
Also read any files the plan references for additional context.

## Analysis Strategy

1. Read your lens skill and output format files (see paths below)
2. Read the implementation plan file fully
3. Identify the scope and complexity of the proposed changes
4. Explore the codebase to understand existing patterns and context
5. Evaluate the plan through your lens, applying each key question
6. Reference specific plan sections in your findings using the `location`
   field (e.g., "Phase 2: API Endpoints", "Testing Strategy section")

## Lens

Read the lens skill at: ~/.claude/skills/[lens]-lens/SKILL.md

## Output Format

Read the output format at: ~/.claude/skills/plan-review-output-format/SKILL.md

IMPORTANT: Return your analysis as a single JSON code block. Do not include
prose outside the JSON block.
```

Spawn all selected agents **in parallel** using the Task tool with
`subagent_type: "reviewer"`.

**IMPORTANT**: Wait for ALL review agents to complete before proceeding.

**Handling malformed agent output**: [keep existing malformed output
handling from current skill]
````

#### 3. Update Lens Table in Both Orchestrators

Update the "Available Review Lenses" table in both `review-pr` and
`review-plan` to reference lens skills instead of named agents:

```markdown
| Lens               | Lens Skill                    | Focus                                                                 |
|--------------------|-------------------------------|-----------------------------------------------------------------------|
| **Architecture**   | `architecture-lens`           | Modularity, coupling, scalability, evolutionary design, tradeoffs     |
| **Security**       | `security-lens`               | Threats, missing protections, STRIDE analysis, OWASP coverage         |
| **Test Coverage**  | `test-coverage-lens`          | Testing strategy, test pyramid, edge cases, isolation, risk coverage (PR review only) |
| **Code Quality**   | `code-quality-lens`           | Design principles, testability, error handling, complexity management |
| **Standards**      | `standards-lens`              | Project conventions, API standards, accessibility, documentation      |
| **Usability**      | `usability-lens`              | Developer experience, API ergonomics, configuration, migration paths  |
```

### Success Criteria

#### Automated Verification:

- [x] `skills/review-pr/SKILL.md` contains the path-passing pattern
- [x] `skills/review-plan/SKILL.md` contains the path-passing pattern
- [x] Both reference `subagent_type: "reviewer"` (not named lens agents)
- [x] Both reference `skills/<lens>-lens/SKILL.md` for lens content
- [x] Both reference their respective output format skill

#### Manual Verification:

- [ ] Run `/review-pr` against a real PR and verify:
  - All selected lenses produce valid JSON with correct schema
  - Inline comments reference valid diff lines
  - The review summary uses emoji prefixes and verdict logic
  - Quality is comparable to the previous agent-based approach
- [ ] Run `/review-plan` against a real plan and verify:
  - All selected lenses produce valid JSON with correct schema
  - Findings reference plan sections correctly
  - Collaborative iteration (Steps 6-7) still works
  - Quality is comparable to the previous agent-based approach

---

## Phase 4: Clean Up and Verify

### Overview

Delete the old command files and review agent files. Verify everything works
end-to-end.

### Changes Required

#### 1. Delete Old Command Files

Delete all 8 files in `commands/`:
- `commands/commit.md`
- `commands/create-plan.md`
- `commands/describe-pr.md`
- `commands/implement-plan.md`
- `commands/research-codebase.md`
- `commands/review-plan.md`
- `commands/review-pr.md`
- `commands/validate-plan.md`

#### 2. Delete Old Review Agent Files

Delete all 12 review agent files:
- `agents/pr-architecture-reviewer.md`
- `agents/pr-security-reviewer.md`
- `agents/pr-test-coverage-reviewer.md`
- `agents/pr-code-quality-reviewer.md`
- `agents/pr-standards-reviewer.md`
- `agents/pr-usability-reviewer.md`
- `agents/plan-architecture-reviewer.md`
- `agents/plan-security-reviewer.md`
- `agents/plan-test-coverage-reviewer.md`
- `agents/plan-code-quality-reviewer.md`
- `agents/plan-standards-reviewer.md`
- `agents/plan-usability-reviewer.md`

#### 3. Update References

Search for any references to the old agent names or command paths in:
- `CLAUDE.md` (if it exists)
- Memory files
- Research documents
- Plan documents

Update references to point to the new skills and agent.

### Success Criteria

#### Automated Verification:

- [x] No files remain in `commands/`
- [x] No `pr-*-reviewer.md` or `plan-*-reviewer.md` files remain in
  `agents/`
- [x] `agents/reviewer.md` exists
- [x] All 16 `skills/*/SKILL.md` files exist

#### Manual Verification:

- [ ] Run `/review-pr` end-to-end against a real PR
- [ ] Run `/review-plan` end-to-end against a real plan
- [ ] Run `/commit`, `/describe-pr`, `/research-codebase` to verify
  migrated commands work
- [ ] Verify no broken references in memory or research documents

---

## Testing Strategy

### Incremental Testing

Each phase can be tested independently:

1. **Phase 1**: After migration, run each `/command` and verify it works.
   Commands and skills coexist (skill takes precedence), so old commands
   can be kept as fallback during testing.

2. **Phase 2**: After creating lens skills and the reviewer agent, test by
   manually spawning the reviewer agent with a composed prompt to verify
   it produces valid JSON output.

3. **Phase 3**: After updating orchestrators, run `/review-pr` and
   `/review-plan` against real targets. Compare output quality with
   previous runs.

4. **Phase 4**: After cleanup, run all skills to verify nothing is broken.

### Key Test Cases

1. **PR review with all 6 lenses**: Verify each lens produces valid JSON
   via the generic reviewer agent
2. **Plan review with all 6 lenses**: Same verification
3. **PR review with partial lens selection**: Verify skipped lenses are
   handled correctly
4. **Plan review with collaborative iteration**: Verify Steps 6-7 still
   work after the orchestrator changes
5. **Malformed agent output**: Verify the fallback handling still works
6. **Dynamic context injection**: Verify `!`command`` works in the commit
   skill

## References

- Research: `meta/research/codebase/2026-02-22-skills-agents-commands-refactoring.md`
- Review alignment research:
  `meta/research/codebase/2026-02-22-review-plan-pr-alignment.md`
- PR review agents design:
  `meta/research/codebase/2026-02-22-pr-review-agents-design.md`
- Current agents: `agents/*.md` (18 files)
- Current commands: `commands/*.md` (8 files)
- Claude Code skills docs: https://code.claude.com/docs/en/skills
- Claude Code subagents docs: https://code.claude.com/docs/en/sub-agents
