---
date: 2026-02-22T19:05:56+0000
researcher: Toby Clemson
git_commit: N/A
branch: N/A
repository: ~/.claude (Claude Code configuration)
topic: "Refactoring agents, commands, and skills for optimal Claude Code configuration"
tags: [ research, skills, agents, commands, refactoring, review-pr, review-plan ]
status: complete
last_updated: 2026-02-22
last_updated_by: Toby Clemson
last_updated_note: "Revised to path-passing pattern; added skills vs agents capability boundaries"
---

# Research: Skills, Agents, and Commands Refactoring

**Date**: 2026-02-22T19:05:56+0000
**Researcher**: Toby Clemson
**Repository**: ~/.claude (Claude Code configuration)

## Research Question

This codebase contains Claude Code configuration with 18 agents and 8
commands but no skills. What refactorings would make best use of Claude Code
features? Specifically: should some agents become skills? Could there be a
generic reviewer agent parameterised by lens-specific skills?

## Summary

Skills are the evolution of commands in Claude Code and provide several
capabilities that commands lack: supporting files (references, scripts,
templates), auto-discovery, argument hints, dynamic context injection via
shell preprocessing, and progressive disclosure. All 8 commands should
migrate to skills. The 12 review agents can be collapsed to 1 generic
`reviewer` agent + 6 lens skills + 2 output format references. The
orchestrator passes skill file paths to the generic agent at spawn time;
the agent reads them itself. This preserves main conversation context
(the orchestrator never reads lens content) while keeping each agent's
context self-contained. Skills and agents are complementary — commands
are deprecated in favour of skills, but agents are actively enhanced and
remain necessary for parallel spawning, custom system prompts, and
advanced execution controls.

## Key Concepts: Skills vs Agents vs Commands

### Skills

- Defined as `SKILL.md` in a directory under `.claude/skills/<name>/`
- Can include supporting files: `references/`, `scripts/`, `examples/`,
  `assets/`
- Can be auto-discovered by Claude based on description matching
- Can be invoked by users via `/skill-name` or by Claude automatically
- Run inline in the main conversation by default
- Can fork into an isolated subagent via `context: fork` and `agent: <type>`
- Support `$ARGUMENTS` for parameter passing
- Support `!`command`` for dynamic shell context injection
- Follow the open Agent Skills standard (agentskills.io)
- Support `allowed-tools` to restrict tool access when skill is active
- Support `model` to override the model used
- Support `hooks` scoped to skill lifecycle (PreToolUse, PostToolUse, Stop)

### Agents (Subagents)

- Defined as `.md` files in `.claude/agents/`
- The agent body IS the system prompt (skills' body becomes the task prompt)
- Run in isolated context windows with their own tool permissions
- Can run in parallel as background tasks (`background: true`)
- Return summarised results to the main conversation
- Can preload skills via `skills:` frontmatter field
- Cannot spawn other subagents (no nesting)
- Support capabilities that skills lack:
  - `disallowedTools` (denylist approach, vs skills' allowlist-only)
  - `permissionMode` (override permission checking behaviour)
  - `maxTurns` (limit agentic turns)
  - `mcpServers` (control MCP server access)
  - `memory` (persistent cross-session learning)
  - `isolation: worktree` (git worktree isolation)
  - Spawning via `Task` tool for parallel/background execution

### Commands (Legacy)

- Defined as single `.md` files in `.claude/commands/`
- Always user-invoked (no auto-discovery)
- No supporting files (single file only)
- Merged into the skills system as of Claude Code v2.1.3
- Continue to work but skills are the recommended approach

### Key Architectural Relationship

- Skills can fork into agents: `context: fork` + `agent: <agent-name>`
- Agents can preload skills: `skills: [skill-a, skill-b]` in frontmatter
- Commands are a subset of skills (same frontmatter, fewer capabilities)
- **Critical distinction**: With `context: fork`, a skill's body becomes the
  task prompt sent to the specified agent type. A standalone agent's body IS
  the system prompt. This means agents provide custom behavioural framing
  that forked skills cannot replicate.

## Detailed Findings

### Current Inventory

| Category           | Count | Items                                                                                                                      |
|--------------------|-------|----------------------------------------------------------------------------------------------------------------------------|
| Commands           | 8     | commit, create-plan, describe-pr, implement-plan, research-codebase, review-plan, review-pr, validate-plan                 |
| Utility agents     | 6     | codebase-locator, codebase-analyser, codebase-pattern-finder, documents-locator, documents-analyser, web-search-researcher |
| PR review agents   | 6     | pr-{architecture,security,test-coverage,code-quality,standards,usability}-reviewer                                         |
| Plan review agents | 6     | plan-{architecture,security,test-coverage,code-quality,standards,usability}-reviewer                                       |
| Skills             | 0     | (none)                                                                                                                     |

### Analysis: What Should Become Skills

#### Commands (All 8 should migrate)

Every command should migrate to a skill. The benefits are:

1. **Supporting files**: Commands like `research-codebase` reference a
   metadata script (`~/.claude/scripts/research-metadata.sh`) and a research
   document template. As a skill, these can be bundled as `scripts/` and
   `references/`.

2. **Progressive disclosure**: Skill descriptions (~100 tokens) are always
   in context; full content loads only when invoked. Commands load their
   full content immediately. For large orchestration commands like
   `review-pr` and `review-plan`, this saves context budget.

3. **Dynamic context injection**: Skills support `!`command`` to inject
   shell output before Claude processes the skill. This benefits:
  - `commit`: inject `!`git diff --cached`` and `!`git log --oneline -5``
  - `describe-pr`: inject `!`gh pr diff`` and `!`gh pr view``
  - `review-pr`: inject `!`gh pr view --json number,title,body``

4. **Argument hints**: Skills support `argument-hint` in frontmatter,
   improving autocomplete UX:
  - `review-pr`: `argument-hint: [PR number or URL]`
  - `review-plan`: `argument-hint: [path to plan file]`
  - `implement-plan`: `argument-hint: [path to plan file]`

5. **Future-proofing**: Skills are the recommended approach; commands are
   legacy.

#### Review Agents (Stay as agents, but with shared skills)

The 12 review agents are correctly agents because:

- They need isolated context windows (each explores the codebase
  independently)
- They run in parallel as background tasks
- They return summarised JSON results
- They need specific tool restrictions (Read, Grep, Glob, LS only)

However, they have significant duplication. Across all 12 agents, these
sections are near-identical:

- Output Format JSON schema structure (field reference, severity levels)
- Severity Emoji Prefixes (3 lines, identical across all 12)
- Finding/Comment Body Format template (identical structure)
- 3 Important Guidelines (self-contained findings, emoji prefixes,
  JSON-only output)
- Parts of "What NOT to Do"

This duplication was recently painful during the review-plan alignment
effort, which required updating the output format across all 6 plan agents.

**Recommendation**: Create shared convention skills that review agents
preload via the `skills:` frontmatter field. This factors out the
duplicated content while keeping each agent focused on its domain-specific
responsibilities.

#### Utility Agents (Stay as agents)

The 6 utility agents (codebase-locator, codebase-analyser, etc.) are
correctly agents. They need isolated context windows, run in parallel, and
are spawned by the research-codebase command or other orchestrators. No
change needed.

### Analysis: Why Agents Are Retained (Not Replaced by Skills)

Since skills gained `context: fork` and `allowed-tools` in Claude Code 2.1,
a natural question is whether agents are now redundant. **They are not.**
Commands were explicitly merged into skills and deprecated; agents were not
-- they are actively being enhanced with new features (`isolation: worktree`,
`background: true`, `memory`, agent-scoped hooks, `maxTurns`).

The key reasons agents are retained in this configuration:

1. **System prompt vs task prompt**: An agent's body IS the system prompt,
   giving it persistent behavioural framing. A forked skill's body becomes
   the task prompt -- it tells the agent what to do, not how to behave. The
   generic `reviewer` agent needs a system prompt that establishes its role
   as a reviewer following injected lens instructions. This framing cannot
   come from a skill alone.

2. **Parallel spawning via Task tool**: The review orchestrators spawn
   multiple reviewers in parallel using the Task tool, which requires an
   agent type. Skills cannot be spawned as parallel background tasks by
   another skill or command.

3. **Agent-only capabilities in use**:
  - Tool restrictions via `tools:` frontmatter (used by all review and
    utility agents to limit to Read, Grep, Glob, LS)
  - Background parallel execution (review agents run concurrently)
  - Isolated context windows (each reviewer explores independently)

4. **Capabilities that may be needed in future**:
  - `maxTurns` to bound reviewer agent exploration
  - `memory` for persistent learning across review sessions
  - `disallowedTools` for denylist-based restrictions
  - `mcpServers` for controlled external access

Skills with `context: fork` are useful when a user-invocable workflow needs
isolated execution (e.g., a deep research skill), but they are not a
replacement for agents that are spawned programmatically by orchestrators.
The two systems are complementary: skills define knowledge and workflows;
agents define execution environments and behavioural roles.

### Analysis: The "Generic Reviewer + Lens Skills" Pattern

The intuition about "a reviewer agent that takes a skill to use during the
review" is achievable via **path passing at spawn time**.

The orchestrator passes file paths to the generic `reviewer` agent, which
reads the lens skill and output format itself. This avoids consuming main
conversation context with lens content (which would happen if the
orchestrator read and injected the content):

```
For each selected lens:
  1. Spawn the generic `reviewer` agent with a prompt containing paths:

     "Review the PR at [path].

     ## Your Lens
     Read the lens skill at: ~/.claude/skills/<lens>-lens/SKILL.md

     ## Output Format
     Read the output format at: ~/.claude/skills/pr-review-output-format/SKILL.md"
```

The generic `reviewer` agent's system prompt instructs it to always start
by reading the lens skill and output format files at the provided paths
before beginning its review.

This achieves:

- **12 agents → 1 agent + 6 lens skills + 2 output format references**
- Adding a new lens = 1 new skill file (not 2 agent files)
- Lens knowledge defined once, used for both PR and plan review
- Format changes are a single-file edit
- No dependency on `skills:` preloading behaviour
- **Minimal main context usage** — orchestrator passes paths, not content

**What goes where**:

| Content                                          | Location                  | Rationale                   |
|--------------------------------------------------|---------------------------|-----------------------------|
| Domain expertise (what the lens cares about)     | Lens skill                | Shared between PR and plan  |
| Output format (JSON schema, fields, body format) | Output format reference   | Differs between PR and plan |
| Context-specific framing (read the diff/plan)    | Orchestrator prompt       | Differs per invocation      |
| Tool access and isolation settings               | Generic agent frontmatter | Same for all lenses         |

**Lens skill content** captures the domain expertise at a level that
applies to both PR and plan review:

- Core Responsibilities (what this lens evaluates)
- Key evaluation questions
- Domain-specific guidelines
- What NOT to review (other lenses)

**Orchestrator task prompt** provides context-specific framing and paths:

- For PR: "Evaluate the architectural impact of the changes in this diff..."
- For Plan: "Evaluate the architectural soundness of this proposed
  design..."
- Analysis strategy adapted to PR vs plan context
- Paths to the lens skill and output format files

**Assessment**: This pattern is practical and achievable. The orchestrator
remains simple (it passes paths, not content), and the agent handles its
own context loading. The reduction from 12 agents to 1 agent + 6 skills
is significant for maintainability.

**Path passing vs content injection tradeoffs**:

| Approach            | Main context cost | Orchestrator complexity | Agent reliability |
|---------------------|-------------------|-------------------------|-------------------|
| Content injection   | ~800-1000 lines (reads all lens + format files) | Higher (reads files, composes prompts) | High (content guaranteed in prompt) |
| Path passing        | ~12 lines (just path strings) | Lower (passes paths only) | High (agent reads files; one extra tool call per agent) |

Path passing is preferred because the main conversation context is the
scarce resource — each lens read in the orchestrator consumes context that
could be used for user interaction. The agent's context is isolated and
disposable.

### Recommended New Skills

#### Lens Skills (6, not user-invocable)

Each captures domain expertise for one review lens:

| Skill                        | Domain                                                           |
|------------------------------|------------------------------------------------------------------|
| `skills/architecture-lens/`  | Structural integrity, coupling, cohesion, evolutionary fitness   |
| `skills/security-lens/`      | STRIDE analysis, OWASP coverage, threat modelling                |
| `skills/test-coverage-lens/` | Test strategy, pyramid balance, edge cases, isolation            |
| `skills/code-quality-lens/`  | Design principles, testability, error handling, complexity       |
| `skills/standards-lens/`     | Project conventions, API standards, accessibility, documentation |
| `skills/usability-lens/`     | Developer experience, API ergonomics, configuration, migration   |

#### Output Format References (2, not user-invocable)

| Skill                               | Content                                                                                                   |
|-------------------------------------|-----------------------------------------------------------------------------------------------------------|
| `skills/pr-review-output-format/`   | PR JSON schema (comments + general_findings with path/line), field reference, emoji prefixes, body format |
| `skills/plan-review-output-format/` | Plan JSON schema (findings with location), field reference, emoji prefixes, body format                   |

### Plan Lifecycle Shared References

The 4 plan lifecycle commands (create-plan, review-plan, implement-plan,
validate-plan) could share references:

- Plan document template/structure
- Plan conventions (phase format, success criteria format)
- Plan metadata patterns

As skills, they can each reference shared files in a common location or
each skill can maintain its own references.

## Recommendations

### Tier 1: Migrate Commands to Skills (High impact, low effort)

Convert all 8 commands from `commands/*.md` to `skills/*/SKILL.md`. This
is a mostly mechanical migration:

1. Create directory: `skills/<command-name>/`
2. Move content to `SKILL.md` with updated frontmatter
3. Add `argument-hint` where applicable
4. Add `disable-model-invocation: true` for all (these are user-initiated
   workflows)
5. Factor large reference sections into `references/`
6. Add dynamic context injection where beneficial

Specific per-command notes:

| Command             | Key Skill Benefits                                                       |
|---------------------|--------------------------------------------------------------------------|
| `commit`            | `!`git diff --cached``, `!`git log --oneline -5`` injection              |
| `create-plan`       | `references/plan-template.md`, `argument-hint: [ticket or description]`  |
| `describe-pr`       | `!`gh pr diff``, `!`gh pr view`` injection                               |
| `implement-plan`    | `argument-hint: [path to plan file]`                                     |
| `research-codebase` | Bundle `scripts/research-metadata.sh`, `references/research-template.md` |
| `review-plan`       | `argument-hint: [path to plan file]`, `references/output-format.md`      |
| `review-pr`         | `argument-hint: [PR number or URL]`, `references/output-format.md`       |
| `validate-plan`     | `argument-hint: [path to plan file]`                                     |

### Tier 2: Generic Reviewer + Lens Skills (High impact, medium effort)

Collapse 12 review agents into 1 generic agent + 6 lens skills + 2 output
format references, using path passing at spawn time:

1. Create 6 lens skills in `skills/<lens>-lens/SKILL.md`
  - `user-invocable: false`
  - Core Responsibilities and evaluation questions for each lens
  - Lens-specific guidelines and "What NOT to Do"
  - Derive content from existing PR/plan agent pairs, taking the union of
    domain expertise
2. Create 2 output format references:
  - `skills/pr-review-output-format/SKILL.md` -- PR JSON schema, field
    reference, emoji prefixes, body format, inline comment rules
  - `skills/plan-review-output-format/SKILL.md` -- Plan JSON schema,
    field reference, emoji prefixes, body format, location format
3. Create 1 generic `reviewer` agent in `agents/reviewer.md`
  - Tools: Read, Grep, Glob, LS
  - System prompt establishing the reviewer role and instructing the agent
    to always start by reading the lens skill and output format files at
    the paths provided in the task prompt
4. Update `review-pr` and `review-plan` orchestrators to:
  - Pass the lens skill path and output format path to each agent
  - Include context-specific framing (PR diff location or plan file path)
  - Spawn the generic `reviewer` agent with paths, not content
5. Delete the 12 lens-specific agent files

**Benefits**:

- 12 agents → 1 agent + 6 skills + 2 format references (9 files, down
  from 12, but each much smaller and with no duplication)
- Adding a new lens = 1 new skill file (not 2 agent files)
- Lens knowledge defined once, works for both PR and plan review
- Output format changes are a single-file edit per review type
- Minimal main context usage (orchestrator passes paths, not content)
- Simpler orchestrator (no file reads or prompt composition)

**Risks**:

- Agent must reliably read skill files before starting review (mitigated
  by clear system prompt instructions)
- The transition requires careful testing of all 12 agent equivalents

### Tier 3: Plan Lifecycle Skill Family (Low impact, low effort)

Create shared references for the plan lifecycle skills:

1. `skills/create-plan/references/plan-conventions.md` -- shared plan
   structure conventions
2. Other plan lifecycle skills reference the same conventions
3. This ensures consistency across create, review, implement, validate

## Architecture Insights

### Progressive Disclosure Budget

Skills descriptions share a context budget of ~2% of the context window
(fallback: 16,000 characters). With 8 skills having descriptions of ~50
characters each, the total is ~400 characters -- well within budget. Even
adding 3 non-user-invocable convention skills stays within limits.

### Naming Conventions

Skills use the same name as their directory. To maintain compatibility with
existing `/command-name` invocations, skill directories should match current
command names (e.g., `skills/review-pr/SKILL.md` creates `/review-pr`).

### Migration Path

Commands and skills can coexist. If both exist with the same name, the
skill takes precedence. This allows incremental migration: create the skill,
verify it works, then remove the command.

## Open Questions

1. **Agent skill preloading verification**: Does `skills:` in agent
   frontmatter actually inject the skill's full content into the agent's
   context? This is no longer critical for Tier 2 (which uses prompt
   injection instead) but would be useful to understand for future patterns.

2. **Context budget with many skills**: With 8 user-invocable skills + 8
   non-user-invocable skills (6 lenses + 2 formats), will the description
   budget cause any to be excluded? Likely not (total ~1200 chars vs 16,000
   budget) but worth verifying. Non-user-invocable skills with
   `disable-model-invocation: true` may not consume description budget at
   all since they're never auto-discovered.

3. **Dynamic context injection timing**: For `review-pr`, the `!`gh pr
   diff`` injection happens before the skill content reaches Claude. But the
   orchestration logic saves the diff to a temp file for agents to read. Need
   to decide whether to use injection for the orchestrator's initial context
   or keep the temp-file approach for agent access.

4. **Skill priority and conflicts**: With skills at `~/.claude/skills/`
   (personal) and potentially project-level `.claude/skills/`, need to
   understand priority behaviour for this configuration which is personal,
   not project-scoped.

5. **Agent file-reading reliability**: The path-passing pattern depends on
   the agent reliably reading the lens skill and output format files before
   starting its review. The generic `reviewer` agent's system prompt should
   make this unambiguous. Worth verifying with a test run that the agent
   consistently reads both files.

6. **Lens skill granularity**: The current PR and plan agents have slightly
   different analysis strategies (PR agents examine diffs, plan agents
   evaluate designs). The lens skill should capture the shared domain
   expertise while the orchestrator provides context-specific analysis
   framing. Need to validate this split works in practice with a test run.

## References

- [Claude Code Skills Documentation](https://code.claude.com/docs/en/skills)
- [Claude Code Subagents Documentation](https://code.claude.com/docs/en/sub-agents)
- [Agent Skills Open Standard](https://agentskills.io)
- [Agent Skills Specification](https://agentskills.io/specification)
- [Anthropic Engineering: Agent Skills](https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills)
- [Inside Claude Code Skills](https://mikhail.io/2025/10/claude-code-skills/)
- [Claude Agent Skills Deep Dive](https://leehanchung.github.io/blogs/2025/10/26/claude-skills-deep-dive/)
- Review alignment research:
  `meta/research/2026-02-22-review-plan-pr-alignment.md`
- Current agents: `agents/*.md` (18 files)
- Current commands: `commands/*.md` (8 files)
- Installed plugin skills (for reference patterns):
  `plugins/marketplaces/claude-plugins-official/plugins/plugin-dev/skills/`
