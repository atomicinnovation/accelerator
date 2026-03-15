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

| Directory    | Purpose                                         | Written by          |
|--------------|-------------------------------------------------|---------------------|
| `research/`  | Research findings with YAML frontmatter         | `research-codebase` |
| `plans/`     | Implementation plans with phased changes        | `create-plan`       |
| `prs/`       | PR descriptions                                 | `describe-pr`       |
| `templates/` | Reusable templates (e.g., PR descriptions)      | manual              |
| `tmp/`       | Ephemeral working data (e.g., review artifacts) | `review-pr`         |

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

| Skill             | Usage                            | Description                                                      |
|-------------------|----------------------------------|------------------------------------------------------------------|
| **commit**        | `/accelerator:commit`            | Create well-structured, atomic git commits                       |
| **describe-pr**   | `/accelerator:describe-pr 123`   | Generate comprehensive PR descriptions following repo templates  |
| **review-pr**     | `/accelerator:review-pr 123`     | Review a PR through multiple quality lenses with inline comments |
| **respond-to-pr** | `/accelerator:respond-to-pr 123` | Address PR review feedback interactively with code changes       |

## Review System

The `review-pr` and `review-plan` skills use a multi-lens review system. Each
lens is a specialised subagent that evaluates changes through a specific quality
perspective:

| Lens              | Focus                                                                |
|-------------------|----------------------------------------------------------------------|
| **Architecture**  | Modularity, coupling, dependency direction, structural drift         |
| **Code Quality**  | Complexity, design principles, error handling, code smells           |
| **Performance**   | Algorithmic efficiency, resource usage, concurrency, caching         |
| **Security**      | OWASP Top 10, input validation, auth/authz, secrets, data flows      |
| **Standards**     | Project conventions, API standards, naming, documentation            |
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
| **documents-locator**       | Discovers relevant documents in `meta/`                | Grep, Glob, LS                            |
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
