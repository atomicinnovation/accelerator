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

| File                           | Scope       | Git        | Purpose                             |
|--------------------------------|-------------|------------|-------------------------------------|
| `.claude/accelerator.md`       | Team-shared | Committed  | Shared project context and settings |
| `.claude/accelerator.local.md` | Personal    | Gitignored | Personal overrides and preferences  |

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
5. Mention that additional customisation is available: "You can also
   customise review behaviour (lens selection, verdict thresholds, inline
   comment limits), output paths (where skills write documents), and
   document templates (plan, ADR, research, validation formats). Run
   `/accelerator:configure help` for the full key reference — you can add
   `review:`, `paths:`, and `templates:` sections to the frontmatter later."
6. Write the config file with a markdown body containing the gathered context
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

### review

Customise review behaviour for `/accelerator:review-pr` and
`/accelerator:review-plan`. Config keys use underscores (e.g.,
`max_inline_comments`). Lens names within array values use their original
hyphenated form (e.g., `code-quality`, `test-coverage`):

Shared settings (apply to both `review-pr` and `review-plan`):

| Key | Default | Description |
|-----|---------|-------------|
| `min_lenses` | `4` | Minimum lenses to run |
| `max_lenses` | `8` | Maximum lenses to run |
| `core_lenses` | `[architecture, code-quality, test-coverage, correctness]` | Lenses considered "core four" |
| `disabled_lenses` | `[]` | Lenses to never use |

PR review only (`review-pr`):

| Key | Default | Description |
|-----|---------|-------------|
| `max_inline_comments` | `10` | Max inline comments |
| `dedup_proximity` | `3` | Line proximity for merging findings |
| `pr_request_changes_severity` | `critical` | Min severity for REQUEST_CHANGES (`critical`, `major`, or `none`) |

Plan review only (`review-plan`):

| Key | Default | Description |
|-----|---------|-------------|
| `plan_revise_severity` | `critical` | Min severity for REVISE (`critical`, `major`, or `none`) |
| `plan_revise_major_count` | `3` | Major findings count to trigger REVISE |

Example configuration:

\```yaml
---
review:
  min_lenses: 3
  max_lenses: 10
  core_lenses: [architecture, security, test-coverage, correctness]
  disabled_lenses: [portability, compatibility]
  max_inline_comments: 15
  dedup_proximity: 5
  pr_request_changes_severity: major
  plan_revise_severity: critical
  plan_revise_major_count: 2
---
\```

Note: YAML comments (`#`) are not supported by the config parser. Do not
add inline comments to config values.

#### Custom Lenses

Create custom review lenses in `.claude/accelerator/lenses/`:

\```
.claude/accelerator/lenses/
  compliance-lens/
    SKILL.md           # Follow the same structure as built-in lenses
  accessibility-lens/
    SKILL.md
\```

Custom lenses are auto-discovered and added to the available lens catalogue.
They must have YAML frontmatter with a `name` field and follow the same
SKILL.md structure as built-in lenses. Custom lenses that provide an
`auto_detect` field participate in auto-detect selection like built-in
lenses. Those without `auto_detect` are always included. Minimal template:

\```markdown
---
name: compliance
description: Evaluates regulatory and policy compliance
auto_detect: Relevant when changes touch regulatory, compliance, or policy-related code
---

# Compliance Lens

## Core Responsibilities
- [What this lens evaluates]

## Key Questions
1. [Questions the reviewer should ask through this lens]

## Boundary
- [What is NOT in scope for this lens]
\```

See any lens in the plugin's `skills/review/lenses/` directory for full
examples of the expected structure.

### paths

Override where skills write output documents. Paths are relative to the
project root (absolute paths are also supported):

| Key | Default | Description |
|-----|---------|-------------|
| `plans` | `meta/plans` | Implementation plans |
| `research` | `meta/research` | Research documents |
| `decisions` | `meta/decisions` | Architecture decision records |
| `prs` | `meta/prs` | PR descriptions |
| `validations` | `meta/validations` | Plan validation reports |
| `review_plans` | `meta/reviews/plans` | Plan review artifacts |
| `review_prs` | `meta/reviews/prs` | PR review working directories |
| `templates` | `meta/templates` | User-provided templates (e.g., PR description) |
| `tickets` | `meta/tickets` | Ticket files referenced by create-plan |
| `notes` | `meta/notes` | Notes directory |

Example configuration:

\```yaml
---
paths:
  plans: docs/plans
  research: docs/research
  decisions: docs/adrs
  prs: docs/prs
  validations: docs/validations
  review_plans: docs/reviews/plans
  review_prs: docs/reviews/prs
  templates: docs/templates
  tickets: docs/tickets
  notes: docs/notes
---
\```

Note: YAML comments (`#`) are not supported by the config parser. Do not
add inline comments to config values.

### templates

Override document templates by placing custom template files in the
templates directory (`paths.templates`, defaults to `meta/templates/`):

\```
meta/templates/
  plan.md            # Custom plan template
  research.md        # Custom research template
  adr.md             # Custom ADR template
  validation.md      # Custom validation template
  pr-description.md  # PR description template (used by describe-pr)
\```

All templates — both skill structure templates (plan, ADR, research,
validation) and user content templates (PR description) — live in the same
directory. Override `paths.templates` to move them all:

\```yaml
---
paths:
  templates: docs/templates
---
\```

For advanced use cases, you can also point individual templates to specific
file paths using the `templates` config section:

\```yaml
---
templates:
  plan: docs/templates/our-plan-format.md
  adr: docs/templates/our-adr-format.md
  pr-description: docs/templates/our-pr-template.md
---
\```

Resolution order: `templates.<name>` config path (if set) → templates
directory (`paths.templates`) → plugin default. Use the plugin's
`templates/` directory as a starting point for customisation.

**Note on cross-references**: Default templates contain hardcoded references
to other skills' output paths (e.g., the plan template references
`meta/research/` in its References section). If you override output paths
(e.g., `paths.research: docs/research`), you should also provide custom
templates with updated cross-references.

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
