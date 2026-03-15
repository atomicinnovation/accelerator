---
date: 2026-03-15 14:39:41 GMT
researcher: accelerator
git_commit: a0ffa8d28db952004a8ae84556e433e49415f48c
branch: main
repository: accelerator
topic: "Context Management Approaches for Claude Code"
tags: [ research, context-engineering, claude-code, subagents, compaction, filesystem-state ]
status: complete
last_updated: 2026-03-15
last_updated_by: accelerator
---

# Research: Context Management Approaches for Claude Code

**Date**: 2026-03-15 14:39:41 GMT
**Researcher**: accelerator
**Git Commit**: a0ffa8d28db952004a8ae84556e433e49415f48c
**Branch**: main
**Repository**: accelerator

## Research Question

What are the best approaches to context management for Claude Code, to maximise
agent effectiveness and overall workflow quality while reducing the likelihood
of
requiring context compaction?

## Summary

Context management for Claude Code revolves around a single constraint: the
200k-token context window degrades in quality as it fills, with performance
notably declining above ~120k tokens. The state of the art combines six key
strategies: (1) structured project configuration via CLAUDE.md, (2) progressive
disclosure through skills and just-in-time loading, (3) subagent isolation for
exploratory work, (4) filesystem-based persistent state, (5) proactive
compaction management, and (6) disciplined session hygiene. The most effective
workflows use a phased approach (research -> plan -> implement), with each phase
running in fresh context and using the filesystem as the communication channel
between phases.

## Detailed Findings

### 1. The Impact Multiplier Hierarchy

From the AI That Works podcast (Episode 17, Dex Horthy & Vaibhav), human effort
should be concentrated at the highest-leverage points in the pipeline:

| Level                           | Bad Input Produces       | Human Review Priority    |
|---------------------------------|--------------------------|--------------------------|
| CLAUDE.md / system instructions | 100k+ bad lines of code  | Highest                  |
| Prompts                         | Thousands of bad lines   | High                     |
| Research                        | 1000+ bad lines of code  | High                     |
| Plans                           | 10-100 bad lines of code | Medium                   |
| Implementation                  | Individual bad lines     | Lowest (agent does this) |

The key insight: **focus human effort on CLAUDE.md, prompts, and research
quality**. These have outsized impact on everything downstream.

### 2. CLAUDE.md Configuration

**What to include:**

- Bash commands Claude cannot guess
- Code style rules that differ from defaults
- Testing instructions and preferred runners
- Repo etiquette (branch naming, PR conventions)
- Architectural decisions specific to the project
- Developer environment quirks

**What to exclude:**

- Anything Claude can figure out from code
- Standard language conventions
- Detailed API docs (link instead)
- Information that changes frequently
- File-by-file codebase descriptions

**Sizing guidelines:** Frontier models follow roughly 150-200 instructions
before compliance drops. Claude Code's system prompt consumes about 50 of those
slots. Target **under 150 instructions** in CLAUDE.md.

**Key technique - use pointers, not copies:** Do not paste code snippets into
CLAUDE.md. Use `file:line` references pointing to the authoritative source.

**Hierarchical placement:**

- `~/.claude/CLAUDE.md` - applies to all sessions globally
- `./CLAUDE.md` - project root, shared with team via git
- Child directories - pulled in on-demand when Claude works with files there

### 3. Progressive Disclosure via Skills

Skills are the primary mechanism for on-demand context loading without bloating
every session.

**Three levels of progressive disclosure:**

1. **Metadata loading (~100 tokens)**: Claude scans skill frontmatter to
   identify relevant matches
2. **Full instructions (<5k tokens)**: When Claude determines a skill applies,
   it reads the full SKILL.md
3. **Bundled resources (variable)**: Additional files within the skill directory
   are loaded only when Claude navigates to them

**Design principles:**

- Keep main SKILL.md under 500 lines
- Include concrete input/output examples
- For skills with side effects, set `disable-model-invocation: true` to prevent
  automatic loading
- Scripts bundled with skills never load into context; only their output
  consumes tokens

### 4. Subagent Context Isolation

Subagents solve the fundamental problem: **exploration consumes context**.

**How isolation works:**

- Each subagent runs in its own fresh context window with a custom system prompt
  and restricted tool access
- Intermediate tool calls and results stay inside the subagent
- Only the final summary message (typically 1,000-2,000 tokens) returns to the
  parent
- The parent's context stays bounded regardless of how much work the subagent
  does

**When to use subagents:**
| Use Case | Why |
|----------|-----|
| Codebase research / investigation | Reading many files would fill parent
context |
| Test running and log analysis | Verbose output stays isolated |
| Documentation fetching | Web content stays in subagent |
| Code review after implementation | Fresh context means no bias toward own
code |
| Parallel exploration of alternatives | Each approach gets clean context |

**The two-phase search pattern:**

1. First, spawn locator agents to find relevant files (narrow tools: Grep, Glob,
   LS only)
2. Then, spawn analyser agents on the most promising findings (add Read tool)

This prevents any single agent from needing to both search the entire codebase
and deeply understand individual files.

**Critical detail:** The only channel from parent to subagent is the Agent
tool's prompt string. Include any file paths, error messages, or decisions the
subagent needs directly in that prompt. There is no implicit context sharing.

### 5. Filesystem as Persistent State

The filesystem serves as external memory that persists across sessions and
survives compaction. This is a critical technique for long-horizon work.

**Core patterns:**

- **Planning documents**: Write plans to files before implementation. These can
  be referenced in future sessions without re-explaining context.
- **Progress tracking**: Maintain running documents that track decisions,
  completed steps, and remaining work. Claude can update these throughout a
  session and future sessions can read them to rebuild context.
- **Research documents**: Write research findings to structured markdown with
  YAML frontmatter. These become the input for downstream planning and
  implementation.

**Spec-driven workflow:** Have Claude research a problem, write findings to a
file, then start a fresh session to plan from the research. Start another fresh
session to implement from the plan. Each session has clean context focused
entirely on its phase.

**The `meta/` directory pattern:**

```
meta/
  research/    # Research findings with YAML frontmatter
  plans/       # Implementation plans with phased changes
  prs/         # PR descriptions
  templates/   # Reusable templates (PR descriptions, etc.)
  tickets/     # Ticket documentation
  decisions/   # Architectural decisions
  tmp/         # Ephemeral working data (review artifacts, etc.)
```

Each skill writes to predictable paths. No skill assumes access to another
skill's conversation history. The filesystem is the sole communication channel
between workflow stages.

### 6. Compaction Management

**Understanding compaction:** When context approaches limits, Claude summarises
conversation history, preserving key decisions and file states while discarding
redundant tool outputs. Auto-compact triggers at approximately 75% utilisation.

**Techniques to avoid or control compaction:**

1. **Proactive manual compaction with focus instructions:**
   ```
   /compact Focus on the API changes and test failures
   ```
   This controls what is preserved rather than leaving it to auto-compact.

2. **PreCompact hooks for automatic context preservation:**
   ```json
   {
     "hooks": {
       "PreCompact": [{
         "matcher": "auto",
         "hooks": [{
           "type": "command",
           "command": "/path/to/backup-context.sh",
           "async": true
         }]
       }]
     }
   }
   ```

3. **Session segmentation:** Break work into phases that each fit within a
   single session. Commit frequently. Start new sessions for new phases.

4. **`/clear` between unrelated tasks:** The simplest and most effective
   technique.

5. **Reduce MCP overhead:** Run `/mcp` to check per-server token costs. Disable
   unused MCP servers.

6. **Use `.claudeignore`:** Exclude `node_modules/`, `dist/`, `build/` and
   similar directories.

7. **Use `/btw` for side questions:** The answer appears in a dismissible
   overlay and never enters conversation history.

### 7. The Three-Phase Workflow

From the AI That Works podcast and validated by production experience:

**Phase 1 - Research:** Understand the problem and system with a dedicated
agent. Produce a research document with specific file paths, line numbers, and
root cause analysis. Each research attempt gets a FRESH context window.

**Phase 2 - Planning:** Build a step-by-step outline of changes. Review as a
human BEFORE implementation. Include "What We're NOT Doing" sections. Include
success criteria and testing strategy.

**Phase 3 - Implementation:** Execute the plan, test as you go. Be ready for
surprises.

Each phase gets fresh context. The filesystem (research docs, plan files) is the
communication channel between phases.

**The parallel plan racing technique:** Create TWO plans - one with research,
one without. Implement both in parallel (e.g., using git worktrees). Sometimes
the "no research" plan is better than overthinking. Let implementations race.

### 8. Context Rot: Four Forms of Degradation

Anthropic identifies four forms of context degradation:

| Type                    | Description                                | Mitigation                                            |
|-------------------------|--------------------------------------------|-------------------------------------------------------|
| **Context Poisoning**   | Incorrect or outdated information          | Regular CLAUDE.md review; use pointers not copies     |
| **Context Distraction** | Irrelevant information consuming attention | `/clear` between tasks; subagent isolation            |
| **Context Confusion**   | Similar information mixed together         | Structured sections with XML tags or markdown headers |
| **Context Clash**       | Contradictory information                  | Single source of truth; prune duplicates              |

### 9. Anti-Patterns

- **Stuffing everything into context** until hitting 200k tokens - the agent
  gets confused
- **Using `/compact` blindly** - it is "designed to work 'okay' for every use
  case, which means it's GUARANTEED to be suboptimal for your use case"
- **Not starting fresh context windows** - stale/bad research from a previous
  window can poison subsequent attempts
- **Working on a stale branch** - can cause research to return incorrect
  findings
- **Reading files into the main agent** that should be delegated to subagents
- **Overloading CLAUDE.md** beyond ~150 instructions

### 10. Combined Strategy: Priority Order

The highest-impact combination of techniques, ordered by return on effort:

1. **Give Claude verification** (tests, screenshots, expected outputs) - highest
   leverage
2. **Use `/clear` aggressively** between unrelated tasks - zero effort,
   immediate benefit
3. **Keep CLAUDE.md concise** (under 150 instructions) - audit and prune
   regularly
4. **Use subagents for exploration** - keeps main context clean
5. **Write plans and specs to files** - persistent state that survives sessions
6. **Use skills instead of CLAUDE.md** for domain-specific knowledge - loads on
   demand
7. **Configure PreCompact hooks** - preserves critical context during compaction
8. **Use Plan Mode** for exploration - halves token consumption
9. **Break work into session-sized phases** - commit between phases, start fresh
10. **Disable unused MCP servers** - free context consumed by tool definitions

## Architecture Insights

### How the Accelerator Plugin Implements These Principles

The Accelerator plugin embodies these context management principles through its
architecture:

**Filesystem as shared memory:** All durable state lives in `meta/`. No skill
assumes access to another skill's conversation history. Plans, research
documents, PR descriptions, and temporary review data are all written to
predictable filesystem paths.

**Agents have bounded, specialised contexts:** Each agent is designed for a
narrow task with restricted tools. The separation between locators (find) and
analysers (understand) prevents any single agent from needing to hold both a
broad search space and deep file content.

**Orchestrators synthesise, workers investigate:** The main agent context stays
lean by delegating investigation to subagents. Only files directly mentioned by
the user are read into the main context.

**Structured workflows replace ad-hoc exploration:** Each skill defines a
numbered, sequential process. This prevents unbounded exploration and keeps each
step's context requirements predictable.

**Artifacts have explicit schemas:** Research documents have YAML frontmatter.
Review outputs have JSON schemas. Plans have template structures. This ensures
artifacts are machine-parseable and consistently structured.

**The planning lifecycle as a context chain:**

1. `research-codebase` writes `meta/research/...`
2. `create-plan` reads research, writes `meta/plans/...`
3. `review-plan` reads plan, spawns reviewers, edits plan
4. `implement-plan` reads plan, executes phases, updates checkboxes
5. `validate-plan` reads plan and git history, compares planned vs actual

Each stage uses the filesystem as the sole communication channel. No stage needs
to know what happened in a previous conversation.

## Code References

- `skills/research/research-codebase/SKILL.md` - Research skill with subagent
  orchestration
- `skills/planning/create-plan/SKILL.md` - Plan creation with subagent research
- `skills/planning/implement-plan/SKILL.md` - Plan execution with checkpoint
  resumption
- `skills/planning/validate-plan/SKILL.md` - Post-implementation verification
- `skills/git/review-pr/SKILL.md` - Multi-lens PR review with temp directory
  pattern
- `agents/codebase-locator.md` - Locator agent (no Read tool - context boundary)
- `agents/codebase-analyser.md` - Analyser agent (has Read tool)
- `agents/reviewer.md` - Generic parameterised review agent

## References

### AI That Works Podcast (Episode 17)

- [Video Recording (1h27m)](https://www.youtube.com/watch?v=42AzKZRNhsk)
- [GitHub Repository](https://github.com/ai-that-works/ai-that-works/tree/main/2025-08-05-advanced-context-engineering-for-coding-agents)
- [Agents as Spec Compilers (Dex Horthy)](https://x.com/dexhorthy/status/1946586571865800724)
- [How not to use SubAgents (Dex Horthy)](https://x.com/dexhorthy/status/1950288431122436597)

### Anthropic Official

- [Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Equipping Agents for the Real World with Agent Skills](https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills)
- [Claude Code Best Practices](https://code.claude.com/docs/en/best-practices)
- [Extend Claude with Skills](https://code.claude.com/docs/en/skills)
- [Create Custom Subagents](https://code.claude.com/docs/en/sub-agents)
- [Hooks Reference](https://code.claude.com/docs/en/hooks)

### Community & Industry

- [Context Engineering for Coding Agents - Martin Fowler](https://martinfowler.com/articles/exploring-gen-ai/context-engineering-coding-agents.html)
- [Background Coding Agents: Context Engineering - Spotify Engineering](https://engineering.atspotify.com/2025/11/context-engineering-background-coding-agents-part-2)
- [Context Management with Subagents - RichSnapp](https://www.richsnapp.com/article/2025/10-05-context-management-with-subagents-in-claude-code)
- [How Claude Code Got Better by Protecting More Context](https://hyperdev.matsuoka.com/p/how-claude-code-got-better-by-protecting)
- [Context Management Optimization - SFEIR Institute](https://institute.sfeir.com/en/claude-code/claude-code-context-management/optimization/)
- [Writing a Good CLAUDE.md - HumanLayer](https://www.humanlayer.dev/blog/writing-a-good-claude-md)
- [Stop Bloating Your CLAUDE.md: Progressive Disclosure](https://alexop.dev/posts/stop-bloating-your-claude-md-progressive-disclosure-ai-coding-tools/)
- [Context Engineering Workflow - Alabe Duarte](https://alabeduarte.com/context-engineering-with-claude-code-my-evolving-workflow/)
- [Claude Code Compaction - Steve Kinney](https://stevekinney.com/courses/ai-development/claude-code-compaction)
- [What to Do When Claude Code Starts Compacting](https://www.duanlightfoot.com/posts/what-to-do-when-claude-code-starts-compacting/)
- [Claude Code Context Handoff - GitHub](https://github.com/who96/claude-code-context-handoff)
- [Building Persistent Memory for AI Agents - DEV Community](https://dev.to/oblivionlabz/building-persistent-memory-for-ai-agents-a-4-layer-file-based-architecture-4pip)

## Open Questions

- What is the optimal size for subagent return summaries? Current practice is
  1,000-2,000 tokens but there may be a more principled threshold.
- How do PreCompact hooks interact with the new 75% auto-compact threshold?
  Is there a way to configure the threshold?
- What is the cost/benefit of the parallel plan racing technique for typical
  development tasks vs. only for ambiguous research situations?
- How should context management strategies adapt as model context windows grow
  beyond 200k? Do the same principles apply at 1M tokens?
