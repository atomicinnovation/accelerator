# Internals

## The `meta/` Directory

Every project using Accelerator gets a `meta/` directory (by default) that
serves as persistent state for the development workflow. Each skill reads from
and writes to predictable paths within it. Run `/accelerator:init` to
create all directories up front, or let skills create them on first use.
These paths can be overridden via the `paths` configuration section:

`research/` is itself subdivided into four subcategories — codebase
research, issue/RCA research, design inventories, and design gaps:

| Directory                       | Purpose                                                        | Written by                                                   |
|---------------------------------|----------------------------------------------------------------|--------------------------------------------------------------|
| `research/`                     | (parent — see subcategories below)                             | —                                                            |
| `  ├─ codebase/`                | Codebase research findings with YAML frontmatter               | `research-codebase`                                          |
| `  ├─ issues/`                  | Issue / RCA research findings                                  | `research-issue`                                             |
| `  ├─ design-inventories/`      | Per-source design inventory snapshots (markdown + screenshots) | `inventory-design`                                           |
| `  └─ design-gaps/`             | Design-gap analysis artifacts                                  | `analyse-design-gaps`                                        |
| `plans/`                        | Implementation plans with phased changes                       | `create-plan`                                                |
| `decisions/`                    | Architecture decision records (ADRs)                           | `create-adr`, `extract-adrs`, `review-adr`                   |
| `reviews/`                      | Review summaries and per-lens results                          | `review-pr`, `review-plan`                                   |
| `validations/`                  | Plan validation reports                                        | `validate-plan`                                              |
| `prs/`                          | PR descriptions                                                | `describe-pr`                                                |
| `work/`                         | Work item files referenced by planning                         | `create-work-item`, `extract-work-items`, `update-work-item` |
| `notes/`                        | Notes and working documents                                    | `create-note`                                                |

This approach means:

- No skill assumes access to another skill's conversation history
- Work survives session boundaries and context compaction
- Plans can be resumed after interruption (implement-plan picks up from the
  first unchecked item)
- Artifacts are structured and machine-parseable (YAML frontmatter, JSON
  schemas)

## Agents

Accelerator uses specialised subagents to keep the main context lean. Each
agent runs in its own context window with restricted tools, returning only a
focused summary to the parent:

| Agent                       | Role                                                              | Tools                                                                                                                                                                                                                                               |
|-----------------------------|-------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **codebase-locator**        | Finds files and components by description                         | Grep, Glob, LS                                                                                                                                                                                                                                      |
| **codebase-analyser**       | Analyses implementation details of specific components            | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **codebase-pattern-finder** | Finds similar implementations and usage examples                  | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **documents-locator**       | Discovers relevant documents in configured directories            | Grep, Glob, LS                                                                                                                                                                                                                                      |
| **documents-analyser**      | Extracts insights from meta documents                             | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **reviewer**                | Evaluates code/plans through a specific quality lens              | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **web-search-researcher**   | Researches external documentation and resources                   | WebSearch, WebFetch, Read, Grep, Glob, LS                                                                                                                                                                                                           |
| **browser-locator**         | Locates routes/screens/components in a running app via Playwright | `Bash(run.sh navigate)`, `Bash(run.sh snapshot)`                   |
| **browser-analyser**        | Analyses screens, captures state and screenshots via Playwright   | `Bash(run.sh navigate\|snapshot\|screenshot\|evaluate\|click\|type\|wait_for)` |

The separation between locators (find, no Read) and analysers (understand, with
Read) is deliberate: it prevents any single agent from needing to both search
broadly and read deeply, keeping each agent's context bounded.

`browser-*` agents drive Playwright through the skill-shipped executor
(`run.sh`), a Bash wrapper around a Node.js TCP daemon that runs Chromium.
No MCP server is required. See `skills/design/inventory-design/PROTOCOL.md`
for the executor wire protocol.
