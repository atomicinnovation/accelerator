# README Restructure Implementation Plan

## Overview

Restructure the Accelerator plugin's README to lead with its philosophy and
primary development loop, making the `meta/` directory pattern and context
management rationale first-class concepts rather than afterthoughts.

## Current State Analysis

The current README (`README.md`, 99 lines) is a flat feature catalogue:

- Logo + one-line description (lines 1-10)
- Installation (lines 12-27)
- Skills table (lines 29-45)
- Review lenses table (lines 47-67)
- Planning workflow list (lines 69-76)
- Agents table (lines 78-88)
- Meta directory — 3 lines, describes it as plugin-internal (lines 90-94)
- License (lines 96-98)

### Key Discoveries:

- The planning workflow (lines 69-76) is the only hint of the development loop,
  but it's buried as a subsection and omits `research-codebase`
- The `meta/` directory description (lines 90-94) actively misleads: it says
  "New plugin development documentation goes here" rather than explaining it as
  a core user-facing feature
- Skills are listed alphabetically rather than grouped by purpose
- No mention of context management philosophy or why the phased approach exists
- The agents table lists agents without explaining their role in the workflow

## Desired End State

The README should:

1. Open with the plugin's philosophy: phased development workflow with
   filesystem-based state to keep context small and focused
2. Present the primary development loop (research -> plan -> implement) as the
   organising principle
3. Explain the `meta/` directory pattern as a core feature, not an
   implementation detail
4. Group skills by their role: primary loop, complementary planning, PR workflow
5. Explain agents as the mechanism for context-efficient research
6. Preserve all existing content (installation, lenses, license)

### Verification:

- All 9 user-invocable skills are mentioned with their `/accelerator:` prefix
- All 7 review lenses are listed
- All 7 agents are listed
- Installation instructions are preserved
- License reference is preserved
- The development loop is clearly presented as: research-codebase -> create-plan
  -> implement-plan
- The `meta/` directory structure is documented with its subdirectories
- The context management rationale is explained

## What We're NOT Doing

- Adding new skills or agents
- Changing any skill or agent behaviour
- Adding badges, CI status, or other README decorations
- Writing a tutorial or getting-started guide
- Documenting the research findings in detail (that's what the research doc is
  for)

## Implementation Approach

Single-phase rewrite of `README.md`. The new structure will be:

1. Logo + tagline (preserved)
2. **Philosophy** — why the plugin exists and how it approaches development
3. **The Development Loop** — research -> plan -> implement with explanation
4. **The `meta/` Directory** — filesystem as persistent state, subdirectory
   purposes
5. **Skills Reference** — grouped by role (primary loop, complementary planning,
   PR workflow)
6. **Review System** — lenses and how they're used
7. **Agents** — what they are and their role in context isolation
8. **Installation** (moved down — philosophy first, mechanics second)
9. **License**

## Phase 1: Rewrite README

### Overview

Replace the current README with a restructured version that leads with
philosophy and uses the development loop as its organising principle.

### Changes Required:

#### 1. README.md

**File**: `README.md`
**Changes**: Full rewrite preserving all factual content but restructuring
around the development loop and `meta/` directory philosophy.

**New structure:**

```markdown
<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/accelerator_logo_dark_bg.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/accelerator_logo_light_bg.png">
    <img alt="Accelerator" src="assets/accelerator_logo_light_bg.png" width="342px">
  </picture>
</p>

A Claude Code plugin for structured, context-efficient software development.
[Jump to installation](#installation) if you're ready to get started.

## Philosophy

Accelerator structures development into discrete phases — research, plan,
implement — where each phase runs with minimal context and communicates with
the next through the filesystem. This design is intentional: by writing
research findings, plans, and other artifacts to disk rather than holding them
in the conversation, each step stays focused and avoids the quality degradation
that comes with large, cluttered context windows.

The result is a development workflow where:

- Each phase has a clear purpose and bounded scope
- The filesystem (specifically the `meta/` directory) serves as persistent
  shared memory between phases and sessions
- Subagents handle exploratory work in isolation, returning only summaries to
  the main context
- Human review happens at the highest-leverage points (research quality and
  plan quality) before implementation begins

For example, a research phase might read 50 files across a codebase, but only
a structured summary is written to disk and passed to the planning phase —
keeping the planner focused and accurate.

## The Development Loop

The primary workflow is a three-phase loop:

```
research-codebase  →  create-plan  →  implement-plan
       ↓                   ↓                 ↓
  meta/research/codebase/      meta/plans/     checked-off plan
```

1. **Research** (`/accelerator:research-codebase "how does auth work?"`):
   Investigate the codebase using parallel subagents. Produces a structured
   research document in `meta/research/codebase/` with findings, file references, and
   architectural context.

2. **Plan** (`/accelerator:create-plan ENG-1234`): Build a phased
   implementation plan informed by research. Produces a plan document in
   `meta/plans/` with specific file changes, success criteria, and testing
   strategy. The plan is reviewed by the developer before proceeding.

3. **Implement** (`/accelerator:implement-plan @meta/plans/plan.md`): Execute
   the plan phase by phase, checking off success criteria as each phase
   completes. The plan file serves as both instructions and progress tracker.

Two complementary skills support this loop:
- `/accelerator:review-plan @meta/plans/plan.md` — Review a plan through
  multiple quality lenses before implementation
- `/accelerator:validate-plan @meta/plans/plan.md` — Verify after
  implementation that the code matches the plan

## The `meta/` Directory

Every project using Accelerator gets a `meta/` directory that serves as
persistent state for the development workflow. Each skill reads from and writes
to predictable paths within it (directories are created on first use by their
respective skills):

| Directory | Purpose | Written by |
|-----------|---------|------------|
| `research/` | Research findings with YAML frontmatter | `research-codebase` |
| `plans/` | Implementation plans with phased changes | `create-plan` |
| `prs/` | PR descriptions | `describe-pr` |
| `templates/` | Reusable templates (e.g., PR descriptions) | manual |
| `tmp/` | Ephemeral working data (e.g., review artifacts) | `review-pr` |

This approach means:
- No skill assumes access to another skill's conversation history
- Work survives session boundaries and context compaction
- Plans can be resumed after interruption (implement-plan picks up from the
  first unchecked item)
- Artifacts are structured and machine-parseable (YAML frontmatter, JSON
  schemas)

## PR Workflow Skills

Alongside the development loop, Accelerator provides skills for team workflows
around pull requests:

| Skill | Usage | Description |
|-------|-------|-------------|
| **commit** | `/accelerator:commit` | Create well-structured, atomic git commits |
| **describe-pr** | `/accelerator:describe-pr 123` | Generate comprehensive PR descriptions following repo templates |
| **review-pr** | `/accelerator:review-pr 123` | Review a PR through multiple quality lenses with inline comments |
| **respond-to-pr** | `/accelerator:respond-to-pr 123` | Address PR review feedback interactively with code changes |

## Review System

The `review-pr` and `review-plan` skills use a multi-lens review system. Each
lens is a specialised subagent that evaluates changes through a specific quality
perspective:

| Lens | Focus |
|------|-------|
| **Architecture** | Modularity, coupling, dependency direction, structural drift |
| **Code Quality** | Complexity, design principles, error handling, code smells |
| **Performance** | Algorithmic efficiency, resource usage, concurrency, caching |
| **Security** | OWASP Top 10, input validation, auth/authz, secrets, data flows |
| **Standards** | Project conventions, API standards, naming, documentation |
| **Test Coverage** | Coverage adequacy, assertion quality, test pyramid, anti-patterns |
| **Usability** | Developer experience, API ergonomics, configuration, migration paths |

Lenses are automatically selected based on scope, or you can specify focus
areas:

```

/accelerator:review-pr 123 focus on security and architecture

```

## Agents

Accelerator uses specialised subagents to keep the main context lean. Each
agent runs in its own context window with restricted tools, returning only a
focused summary to the parent:

| Agent | Role | Tools |
|-------|------|-------|
| **codebase-locator** | Finds files and components by description | Grep, Glob, LS |
| **codebase-analyser** | Analyses implementation details of specific components | Read, Grep, Glob, LS |
| **codebase-pattern-finder** | Finds similar implementations and usage examples | Read, Grep, Glob, LS |
| **documents-locator** | Discovers relevant documents in `meta/` | Grep, Glob, LS |
| **documents-analyser** | Extracts insights from meta documents | Read, Grep, Glob, LS |
| **reviewer** | Evaluates code/plans through a specific quality lens | Read, Grep, Glob, LS |
| **web-search-researcher** | Researches external documentation and resources | WebSearch, WebFetch, Read, Grep, Glob, LS |

The separation between locators (find, no Read) and analysers (understand, with
Read) is deliberate: it prevents any single agent from needing to both search
broadly and read deeply, keeping each agent's context bounded.

## Installation

Add the marketplace and install the plugin:

```bash
/plugin marketplace add atomicinnovation/accelerator
/plugin install accelerator@atomic-innovation
```

### Development

To load from a local checkout:

```bash
claude --plugin-dir /path/to/accelerator
```

## License

MIT — see [LICENSE](LICENSE).

```

### Success Criteria:

#### Automated Verification:

- [x] `README.md` exists and is valid markdown
- [x] All 9 user-invocable skills are mentioned: `grep -c '/accelerator:' README.md` returns at least 9
- [x] All 7 agents are listed: `grep -c '^\| \*\*codebase-locator\|^\| \*\*codebase-analyser\|^\| \*\*codebase-pattern-finder\|^\| \*\*documents-locator\|^\| \*\*documents-analyser\|^\| \*\*reviewer\|^\| \*\*web-search-researcher' README.md` returns 7
- [x] All 7 review lenses are listed: `grep -c '^\| \*\*Architecture\|^\| \*\*Code Quality\|^\| \*\*Performance\|^\| \*\*Security\|^\| \*\*Standards\|^\| \*\*Test Coverage\|^\| \*\*Usability' README.md` returns 7
- [x] Installation instructions present: `grep -c 'plugin marketplace add\|plugin install\|plugin-dir' README.md` returns 3
- [x] License reference present: `grep -c 'LICENSE' README.md` returns at least 1
- [x] `meta/` directory table present with at least 5 rows: `grep -c 'research/\|plans/\|prs/\|templates/\|tmp/' README.md` returns 5

#### Manual Verification:

- [ ] README reads naturally with philosophy first, mechanics second
- [ ] The development loop diagram renders correctly in GitHub markdown
- [ ] The `meta/` directory is presented as a user-facing feature, not an
      implementation detail
- [ ] Skills are grouped by role (primary loop, complementary, PR workflow)
      rather than alphabetically
- [ ] The context management rationale is clear without being preachy
- [ ] No existing content has been lost

## Testing Strategy

This is a documentation-only change with no runtime code. Verification is
covered entirely by the automated and manual checks in the Phase 1 success
criteria. No unit, integration, or runtime testing is applicable.

## References

- Current README: `README.md`
- Context management research: `meta/research/codebase/2026-03-15-context-management-approaches.md`
