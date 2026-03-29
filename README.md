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
  meta/research/      meta/plans/     checked-off plan
```

1. **Research** (`/accelerator:research-codebase "how does auth work?"`):
   Investigate the codebase using parallel subagents. Produces a structured
   research document in `meta/research/` with findings, file references, and
   architectural context.

2. **Plan** (`/accelerator:create-plan ENG-1234`): Build a phased
   implementation plan informed by research. Produces a plan document in
   `meta/plans/` with specific file changes, success criteria, and testing
   strategy. The plan is reviewed by the developer before proceeding.

3. **Implement** (`/accelerator:implement-plan @meta/plans/plan.md`): Execute
   the plan phase by phase, checking off success criteria as each phase
   completes. The plan file serves as both instructions and progress tracker.

Three complementary skills support this loop:

- `/accelerator:review-plan @meta/plans/plan.md` — Review a plan through
  multiple quality lenses before implementation
- `/accelerator:stress-test-plan @meta/plans/plan.md` — Interactively
  stress-test a plan through adversarial questioning to find issues,
  inconsistencies, and gaps
- `/accelerator:validate-plan @meta/plans/plan.md` — Verify after
  implementation that the code matches the plan

## The `meta/` Directory

Every project using Accelerator gets a `meta/` directory (by default) that
serves as persistent state for the development workflow. Each skill reads from
and writes to predictable paths within it. Run `/accelerator:init` to
create all directories up front, or let skills create them on first use.
These paths can be overridden via the `paths` configuration section:

| Directory      | Purpose                                         | Written by                                 |
|----------------|-------------------------------------------------|--------------------------------------------|
| `research/`    | Research findings with YAML frontmatter         | `research-codebase`                        |
| `plans/`       | Implementation plans with phased changes        | `create-plan`                              |
| `decisions/`   | Architecture decision records (ADRs)            | `create-adr`, `extract-adrs`, `review-adr` |
| `reviews/`     | Review summaries and per-lens results           | `review-pr`, `review-plan`                 |
| `validations/` | Plan validation reports                         | `validate-plan`                            |
| `prs/`         | PR descriptions                                 | `describe-pr`                              |
| `templates/`   | Reusable templates (e.g., PR descriptions)      | manual                                     |
| `tickets/`     | Ticket files referenced by planning             | manual                                     |
| `notes/`       | Notes and working documents                     | manual                                     |
| `tmp/`         | Ephemeral working data (e.g., review artifacts) | `review-pr`                                |

This approach means:

- No skill assumes access to another skill's conversation history
- Work survives session boundaries and context compaction
- Plans can be resumed after interruption (implement-plan picks up from the
  first unchecked item)
- Artifacts are structured and machine-parseable (YAML frontmatter, JSON
  schemas)

## VCS Detection

Accelerator automatically detects whether a repository uses git or
[jujutsu (jj)](https://github.com/jj-vcs/jj) and adapts its behaviour
accordingly. A `SessionStart` hook inspects the working directory for `.jj/` and
`.git/` directories, injecting VCS-specific context (command references and
conventions) into the session. A complementary `PreToolUse` guard warns when raw
git commands are used in a jujutsu repository.

This means all VCS-aware skills — `commit`, `respond-to-pr`, and ad-hoc
interactions — use the correct CLI commands without manual configuration. The
detection covers three modes:

| Mode               | Detected when      | VCS commands used |
|--------------------|--------------------|-------------------|
| **git**            | `.git/` only       | `git`             |
| **jj (colocated)** | `.jj/` and `.git/` | `jj`              |
| **jj (pure)**      | `.jj/` only        | `jj`              |

## Configuration

Accelerator supports project-specific configuration through markdown files with
YAML frontmatter. Configuration allows you to provide project context, customise
agent behaviour, and tune review settings.

### Config Files

| File                           | Scope                   | Purpose                             |
|--------------------------------|-------------------------|-------------------------------------|
| `.claude/accelerator.md`       | Team-shared (committed) | Shared project context and settings |
| `.claude/accelerator.local.md` | Personal (gitignored)   | Personal overrides and preferences  |

Local settings override team settings for the same key. Markdown bodies from
both files are concatenated (team context first, then personal).

### File Format

Both files use YAML frontmatter for structured settings and a markdown body for
free-form project context:

```yaml
---
agents:
  reviewer: my-custom-reviewer
review:
  disabled_lenses: [ portability, compatibility ]
  max_inline_comments: 15
  pr_request_changes_severity: major
---

## Project Context

  This is a Ruby on Rails application using PostgreSQL and Redis.

### Conventions
- Follow StandardRB for code style
- Use service objects for business logic
- All API endpoints require authentication

### Build & Test
- `bundle exec rspec` to run tests
- `bin/dev` to start the development server
```

The YAML frontmatter supports `agents` (override which agents skills spawn),
`review` (customise review behaviour), `paths` (override where skills write
output documents), and `templates` (point to custom document templates). See
`/accelerator:configure help` for the full key reference.

### Getting Started

Run `/accelerator:configure` to create or view your configuration. The skill
walks you through gathering project context and writes the config file for you.

### How It Works

- A `SessionStart` hook detects config files and injects a summary into the
  session context
- Skills read project context at invocation time via the `!` preprocessor
- Config changes take effect on the next skill invocation (no session restart
  needed for skills); the SessionStart summary updates on session restart

### Custom Review Lenses

You can add custom review lenses alongside the 13 built-in ones. Place them in
`.claude/accelerator/lenses/` following the `[name]-lens/SKILL.md` convention.
Custom lenses are auto-discovered and included in the lens catalogue. See
`/accelerator:configure help` for details and a minimal template.

### Per-Skill Customisation

Beyond global context, you can provide context or instructions targeted at
individual skills by placing files in
`.claude/accelerator/skills/<skill-name>/`:

```
.claude/accelerator/skills/
  review-pr/
    context.md          # Context specific to PR review
    instructions.md     # Additional instructions for PR review
  create-plan/
    instructions.md     # Additional instructions for plan creation
```

- **`context.md`** — Injected after global project context. Use for information
  only one skill needs (e.g., review criteria for `review-pr`, architecture
  context for `create-plan`).
- **`instructions.md`** — Appended to the end of the skill's prompt. Use to add
  steps, enforce conventions, or modify output format. Instructions at the end
  of the prompt typically take precedence when they conflict with earlier
  instructions.

Both files are optional. Directory names must match the skill name exactly (the
part after `/accelerator:`). The SessionStart hook warns about unrecognised
directory names. See `/accelerator:configure help` for the full reference.

## Architecture Decision Records

ADR skills capture architectural decisions that emerge from research and
planning:

```
research-codebase → create-plan → implement-plan
       ↓                ↓
  meta/research/    meta/plans/
       ↓                ↓
  extract-adrs ←────────┘
       ↓
  meta/decisions/
       ↓
  review-adr → accepted ADRs inform future research & planning
```

| Skill            | Usage                                      | Description                                                |
|------------------|--------------------------------------------|------------------------------------------------------------|
| **create-adr**   | `/accelerator:create-adr [topic]`          | Interactively create an ADR with context gathering         |
| **extract-adrs** | `/accelerator:extract-adrs [doc paths...]` | Extract decisions from existing meta documents into ADRs   |
| **review-adr**   | `/accelerator:review-adr [path to ADR]`    | Review proposed ADRs; accept, reject, or suggest revisions |

ADRs follow an append-only lifecycle: once accepted, an ADR's content becomes
immutable. To revise a decision, create a new ADR that supersedes the original.

## VCS and PR Workflow Skills

Alongside the development loop, Accelerator provides skills for version control
and team workflows around pull requests:

| Skill             | Usage                            | Description                                                              |
|-------------------|----------------------------------|--------------------------------------------------------------------------|
| **commit**        | `/accelerator:commit`            | Create well-structured, atomic commits (works with both git and jujutsu) |
| **describe-pr**   | `/accelerator:describe-pr 123`   | Generate comprehensive PR descriptions from a configurable template      |
| **review-pr**     | `/accelerator:review-pr 123`     | Review a PR through multiple quality lenses with inline comments         |
| **respond-to-pr** | `/accelerator:respond-to-pr 123` | Address PR review feedback interactively with code changes               |

## Review System

The `review-pr` and `review-plan` skills use a multi-lens review system. Each
lens is a specialised subagent that evaluates changes through a specific quality
perspective:

| Lens              | Focus                                                                |
|-------------------|----------------------------------------------------------------------|
| **Architecture**  | Modularity, coupling, dependency direction, structural drift         |
| **Code Quality**  | Complexity, design principles, error handling, code smells           |
| **Compatibility** | API contracts, cross-platform, protocol compliance, deps             |
| **Correctness**   | Logical validity, boundary conditions, state management, concurrency |
| **Database**      | Migration safety, schema design, query correctness, integrity        |
| **Documentation** | Documentation completeness, accuracy, audience fit                   |
| **Performance**   | Algorithmic efficiency, resource usage, concurrency, caching         |
| **Portability**   | Environment independence, deployment flexibility, vendor lock        |
| **Safety**        | Data loss prevention, operational safety, protective mechanisms      |
| **Security**      | OWASP Top 10, input validation, auth/authz, secrets, data flows      |
| **Standards**     | Project conventions, API standards, naming, accessibility            |
| **Test Coverage** | Coverage adequacy, assertion quality, test pyramid, anti-patterns    |
| **Usability**     | Developer experience, API ergonomics, configuration, migration paths |

Lenses are automatically selected based on scope, or you can specify focus
areas:

```
/accelerator:review-pr 123 focus on security and architecture
```

## Agents

Accelerator uses specialised subagents to keep the main context lean. Each
agent runs in its own context window with restricted tools, returning only a
focused summary to the parent:

| Agent                       | Role                                                   | Tools                                     |
|-----------------------------|--------------------------------------------------------|-------------------------------------------|
| **codebase-locator**        | Finds files and components by description              | Grep, Glob, LS                            |
| **codebase-analyser**       | Analyses implementation details of specific components | Read, Grep, Glob, LS                      |
| **codebase-pattern-finder** | Finds similar implementations and usage examples       | Read, Grep, Glob, LS                      |
| **documents-locator**       | Discovers relevant documents in configured directories | Grep, Glob, LS                            |
| **documents-analyser**      | Extracts insights from meta documents                  | Read, Grep, Glob, LS                      |
| **reviewer**                | Evaluates code/plans through a specific quality lens   | Read, Grep, Glob, LS                      |
| **web-search-researcher**   | Researches external documentation and resources        | WebSearch, WebFetch, Read, Grep, Glob, LS |

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
