---
name: configure
description: View, create, or edit Accelerator plugin configuration. Use when the
  user wants to customise how Accelerator skills behave in their project.
argument-hint: "[view | create | help]"
disable-model-invocation: true
---

# Configure Accelerator

You help users manage their Accelerator plugin configuration.

## Configuration Files

Accelerator reads configuration from two files in the project's `.claude/`
directory:

| File | Scope | Git | Purpose |
|------|-------|-----|---------|
| `.claude/accelerator.md` | Team-shared | Committed | Shared project context and settings |
| `.claude/accelerator.local.md` | Personal | Gitignored | Personal overrides and preferences |

Both files use YAML frontmatter for structured settings and a markdown body for
free-form project context. Local settings override team settings for the same
key.

## Available Actions

When invoked:

1. **Check current configuration state**:
   - Check if `.claude/accelerator.md` exists
   - Check if `.claude/accelerator.local.md` exists
   - If either exists, read and display current settings
   - **If a config file already exists, always show its current contents and ask
     the user to confirm before overwriting. Never silently replace an existing
     config file.**

2. **Based on the argument or user intent**:

### `view` (or no argument with existing config)

Display the current configuration:
```
## Current Accelerator Configuration

### Team Config (.claude/accelerator.md)
[Display frontmatter settings as a formatted table]
[Display markdown body if present]

### Personal Config (.claude/accelerator.local.md)
[Display frontmatter settings as a formatted table]
[Display markdown body if present]

### Effective Settings
[Show merged settings with source attribution]
```

### `create` (or no argument with no existing config)

Help the user create a configuration file. Focus on gathering project context
for the markdown body — this is the highest-value feature.

1. Ask whether they want to create a team config (shared) or personal config
   (local), or both
2. If creating a personal config, check whether `.claude/accelerator.local.md`
   is in `.gitignore` (or `.claude/.gitignore`). If not, offer to add it to
   the repo root `.gitignore`.
3. Ask about their project context — frame questions around "What should
   Accelerator skills know about your project?":
   - What tech stack do they use? (languages, frameworks, build system)
   - Any specific conventions or standards?
   - Any domain-specific context that should inform skills?
   - Build and test commands?
4. Optionally ask about agent overrides: "Would you also like to configure
   custom agent overrides? (This is an advanced feature — most users can skip
   this.)" If yes, explain the available agents and their roles, then gather
   override mappings.
5. Write the config file with a markdown body containing the gathered context
   and YAML frontmatter containing any agent overrides (or empty frontmatter
   if none).

### `help`

Display the configuration reference:
```
## Accelerator Configuration Reference

### File Format

Both config files use YAML frontmatter with a markdown body:

\```yaml
---
agents:
  reviewer: my-custom-reviewer
---

# Free-form project context (markdown)
Additional context that skills will consider when making decisions.
\```

### agents

Override which agents are used when skills spawn sub-agents. Config keys
use the same hyphenated names as the agents themselves:

Available agents and their roles:

| Config Key | Default Role |
|---|---|
| `reviewer` | Reviews plans and PRs using configured lenses |
| `codebase-locator` | Finds relevant source files for a given task |
| `codebase-analyser` | Analyses implementation details of components |
| `codebase-pattern-finder` | Finds similar implementations and usage examples |
| `documents-locator` | Discovers relevant documents in meta/ directory |
| `documents-analyser` | Deep-dives on research topics in documents |
| `web-search-researcher` | Researches topics via web search |

\```yaml
---
agents:
  reviewer: my-custom-reviewer
  codebase-locator: my-locator
  codebase-analyser: my-analyser
  codebase-pattern-finder: my-pattern-finder
  documents-locator: my-doc-locator
  documents-analyser: my-doc-analyser
  web-search-researcher: my-web-researcher
---
\```

Only list agents you want to override. Unlisted agents use their defaults.
Unrecognised keys produce a warning to stderr and are ignored. Override
values can be any agent name — the plugin does not validate values since
the override may reference a user-defined agent outside the plugin.

### Project Context

The markdown body is injected into skills as project-specific guidance:

\```markdown
---
agents:
  reviewer: my-custom-reviewer
---

# Project Context

## Tech Stack
- Language: TypeScript with strict mode
- Framework: Next.js 14 with App Router
- Database: PostgreSQL via Prisma ORM
- Testing: Vitest for unit tests, Playwright for E2E

## Conventions
- All API routes use GraphQL (no REST endpoints)
- Database migrations must be backward-compatible
- Feature flags managed via LaunchDarkly

## Build & Test
- Build: `npm run build`
- Test: `npm run test`
- Lint: `npm run lint`
- Full check: `npm run check`
\```

Use the project context for:
- Tech stack description
- Coding conventions
- Domain-specific terminology
- Build and test commands
- Architecture notes

### Parser Constraints

The configuration parser supports simple scalar YAML values. The following
are not currently supported in frontmatter values:
- Multi-line YAML values (block scalars `|` / `>`)
- YAML comments (the `#` character in values is included as-is)
- Nesting deeper than 2 levels
```

## Important Notes

- Config changes take effect on the next skill invocation (no restart needed
  for skills using the `!` preprocessor)
- The SessionStart hook summary requires a session restart to update
- `.local.md` files should be gitignored — the `create` action will help
  with this
- Team config should contain only project-relevant context, not personal
  preferences
