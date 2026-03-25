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
for the markdown body — this is the primary value of config files in the
current version. Structured settings will be added in future versions.

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
4. Write the config file with a markdown body containing the gathered context.
   Include a minimal YAML frontmatter section with a comment noting that
   structured settings will be available in future versions.

### `help`

Display the configuration reference:
```
## Accelerator Configuration Reference

### File Format

Both config files use YAML frontmatter with a markdown body:

\```yaml
---
# Structured settings (YAML) — settings will be added in future versions.
# For now, the frontmatter section can be left empty or omitted.
---

# Free-form project context (markdown)
Additional context that skills will consider when making decisions.
\```

### Structured Settings

Structured configuration settings (for customising agents, review behaviour,
output paths, etc.) will be added in future versions of the plugin. When
available, they will use YAML frontmatter with max 2-level nesting:

\```yaml
---
section:
  key: value
---
\```

### Project Context

The markdown body of your config file is injected into skills that benefit
from project awareness. This is the primary configuration mechanism in the
current version. Use it for:
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
