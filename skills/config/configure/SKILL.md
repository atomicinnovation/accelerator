---
name: configure
description: "View, create, or edit Accelerator plugin configuration. Manage document templates."
argument-hint: "[view | create | help | templates ...]"
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

### Per-Skill Customisations
[For each directory under .claude/accelerator/skills/ that contains
non-empty context.md or instructions.md, list:]

- `<skill-name>`:
  - context.md: [present / not found]
  - instructions.md: [present / not found]

[If no per-skill customisation directories exist:]

No per-skill customisations found. See `/accelerator:configure help`
for details on per-skill context and instructions.
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
   comment limits), output paths (where skills write documents), document
   templates (plan, ADR, research, validation formats), and per-skill
   context and instructions (`.claude/accelerator/skills/<skill-name>/`).
   Run `/accelerator:configure help` for the full key reference."
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

| Config Key                | Default Role                                               |
|---------------------------|------------------------------------------------------------|
| `reviewer`                | Reviews plans, PRs, and work items using configured lenses |
| `codebase-locator`        | Finds relevant source files for a given task               |
| `codebase-analyser`       | Analyses implementation details of components              |
| `codebase-pattern-finder` | Finds similar implementations and usage examples           |
| `documents-locator`       | Discovers relevant documents in meta/ directory            |
| `documents-analyser`      | Deep-dives on research topics in documents                 |
| `web-search-researcher`   | Researches topics via web search                           |

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

Customise review behaviour for `/accelerator:review-pr`,
`/accelerator:review-plan`, and `/accelerator:review-work-item`. Config keys use
underscores (e.g., `max_inline_comments`). Lens names within array values use
their original hyphenated form (e.g., `code-quality`, `test-coverage`):

Shared settings (apply to `review-pr`, `review-plan`, and `review-work-item`):

| Key               | Default                                                    | Description                   |
|-------------------|------------------------------------------------------------|-------------------------------|
| `min_lenses`      | `4` (3 for work item)                                      | Minimum lenses to run         |
| `max_lenses`      | `8`                                                        | Maximum lenses to run         |
| `core_lenses`     | `[architecture, code-quality, test-coverage, correctness]` | Lenses considered "core four" |
| `disabled_lenses` | `[]`                                                       | Lenses to never use           |

PR review only (`review-pr`):

| Key                           | Default    | Description                                                       |
|-------------------------------|------------|-------------------------------------------------------------------|
| `max_inline_comments`         | `10`       | Max inline comments                                               |
| `dedup_proximity`             | `3`        | Line proximity for merging findings                               |
| `pr_request_changes_severity` | `critical` | Min severity for REQUEST_CHANGES (`critical`, `major`, or `none`) |

Plan review only (`review-plan`):

| Key                       | Default    | Description                                              |
|---------------------------|------------|----------------------------------------------------------|
| `plan_revise_severity`    | `critical` | Min severity for REVISE (`critical`, `major`, or `none`) |
| `plan_revise_major_count` | `3`        | Major findings count to trigger REVISE                   |

Work item review only (`review-work-item`):

| Key                            | Default    | Description                                              |
|--------------------------------|------------|----------------------------------------------------------|
| `work_item_revise_severity`    | `critical` | Min severity for REVISE (`critical`, `major`, or `none`) |
| `work_item_revise_major_count` | `2`        | Major findings count to trigger REVISE                   |

Work items are smaller artifacts than plans, so `work_item_revise_major_count`
defaults to `2` (not `3`): a lower threshold produces equivalent signal density.

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
  work_item_revise_severity: major
  work_item_revise_major_count: 3
---
\```

Note: YAML comments (`#`) are not supported by the config parser. Do not
add inline comments to config values.

#### Per-Review-Type Lenses

Built-in lenses are partitioned by review type: the 13 code-review lenses
(`architecture`, `code-quality`, etc.) are used by `review-pr` and
`review-plan`; work-item-specific lenses (`completeness`, `testability`,
`clarity`) are used by `review-work-item`. Each command sees only its own lenses
in the Lens Catalogue.

`core_lenses` and `disabled_lenses` entries are cross-mode: they are validated
against the union of all built-in and custom lens names, so naming a PR lens in
`core_lenses` does not produce a warning when running `review-work-item`. Entries
not applicable to the active mode are silently filtered out, with an
informational note in the `## Review Configuration` block so you have an audit
trail. Entries that are not valid in any mode still produce an "unrecognised
lens" warning.

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

**Optional fields** — by default a custom lens appears in all review modes
(`pr`, `plan`, and `work-item`). To restrict it to specific modes, add an
`applies_to` field:

\```markdown
---
name: compliance
description: Evaluates regulatory and policy compliance
auto_detect: Relevant when changes touch regulatory, compliance, or policy-related code
# no applies_to — applies to all modes: pr, plan, and work-item
---
\```

\```markdown
---
name: work-item-style
description: Evaluates work-item-specific style conventions
applies_to: [work-item]   # work item reviews only
---
\```

Accepted values: `pr`, `plan`, `work-item`. The field accepts a YAML flow array
(`[pr, plan]`) or a bare scalar (`pr`). Omitting it is equivalent to all modes.
The `applies_to` field is only for custom lenses — built-in lenses are
partitioned via script arrays, not frontmatter.

### Per-Skill Customisation

Provide context or additional instructions for specific skills by placing
files in `.claude/accelerator/skills/<skill-name>/`:

\```
.claude/accelerator/skills/
  create-plan/
    context.md          # Context specific to plan creation
    instructions.md     # Additional instructions for plan creation
  review-pr/
    context.md          # Context specific to PR review
    instructions.md     # Additional instructions for PR review
  review-work-item/
    context.md          # Context specific to work item review
    instructions.md     # Additional instructions for work item review
  commit/
    instructions.md     # Additional instructions for commits
\```

**`context.md`** — Skill-specific context injected after global project
context. Use this for information that is only relevant to a particular
skill. For example, review-pr might need to know about specific review
criteria, while create-plan might need architecture context.

**`instructions.md`** — Additional instructions appended to the skill's
prompt. Use this to customise skill behaviour: add extra steps, enforce
conventions, or modify output format.

Both files are optional. If neither exists for a skill, it behaves as
before. Files are read at skill invocation time. Do not add YAML
frontmatter to these files — their entire content is injected as-is.

**When to use which**: Use **global context** (`.claude/accelerator.md`)
for information all skills should know. Use **skill context**
(`context.md`) for information only one skill needs. Use **skill
instructions** (`instructions.md`) to change how a skill behaves — add
steps, enforce formats, or modify output. Per-skill context and
instructions supplement global context (both are visible to the skill);
per-skill instructions appear at the end of the prompt and will typically
take precedence if they conflict with earlier instructions.

**Shared vs personal**: Per-skill files are typically committed to the
repository as team-shared customisations. For personal per-skill
preferences, add the relevant directories to `.gitignore`.

**Troubleshooting**: Directory names must match a known skill name exactly.
The directory name matches the skill name after `/accelerator:` — for
example, `/accelerator:review-pr` uses `review-pr/`,
`/accelerator:create-plan` uses `create-plan/`. Run
`/accelerator:configure view` to see all available skill names and any
active per-skill customisations. The SessionStart hook output also lists
detected per-skill customisations and warns about unrecognised directory
names. To temporarily disable a customisation, rename the file (e.g.,
`context.md.disabled`).

Note: The `configure` skill is not customisable via this mechanism as it
manages configuration itself.

Example `context.md` for review-pr:

\```markdown
## Review Focus Areas

Our team particularly cares about:
- API backward compatibility (we have external consumers)
- Database migration safety (zero-downtime deploys required)
- Test coverage for error paths (we've had incidents from untested error handling)
\```

Example `instructions.md` for create-plan:

\```markdown
- Always include a "Security Considerations" section in plans
- Reference our threat model at docs/security/threat-model.md
- Plans touching the payments service require a rollback strategy
\```

### paths

Override where skills write output documents. Paths are relative to the
project root (absolute paths are also supported):

| Key            | Default              | Description                                    |
|----------------|----------------------|------------------------------------------------|
| `plans`        | `meta/plans`         | Implementation plans                           |
| `research`     | `meta/research`      | Research documents                             |
| `decisions`    | `meta/decisions`     | Architecture decision records                  |
| `prs`          | `meta/prs`           | PR descriptions                                |
| `validations`  | `meta/validations`   | Plan validation reports                        |
| `review_plans` | `meta/reviews/plans` | Plan review artifacts                          |
| `review_prs`   | `meta/reviews/prs`   | PR review working directories                  |
| `review_work`  | `meta/reviews/work`  | Work item review artifacts                     |
| `templates`    | `meta/templates`     | User-provided templates (e.g., PR description) |
| `work`         | `meta/work`          | Work item files referenced by create-plan      |
| `notes`        | `meta/notes`         | Notes directory                                |
| `tmp`          | `meta/tmp`           | Ephemeral working data (gitignored)            |

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
  review_work: docs/reviews/work
  templates: docs/templates
  work: docs/work
  notes: docs/notes
  tmp: docs/tmp
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
  work-item.md       # Custom work-item template
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

#### Template Management Commands

Use `/accelerator:configure templates <action>` to manage templates:

| Command                 | Description |
|-------------------------|--------------------------------------------------------|
| `templates list`        | List all templates with resolution source and path     |
| `templates show <key>`  | Display the effective template content                 |
| `templates eject <key>` | Copy plugin default to your templates directory        |
| `templates eject --all` | Eject all templates at once                            |
| `templates diff <key>`  | Show differences between your template and the default |
| `templates reset <key>` | Remove your customisation, revert to plugin default    |

Available template keys: `plan`, `research`, `adr`, `validation`, `pr-description`, `work-item`.

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

### `templates`

When the user's argument starts with `templates`, dispatch based on the
action that follows. The `CLAUDE_PLUGIN_ROOT` environment variable points
to the plugin installation directory where scripts are located.

#### `templates list`

Run the list script and display its output:

```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-list-template.sh"
```

Present the table output to the user.

#### `templates show <key>`

Run the show script with the template key:

```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-show-template.sh" <key>
```

Present the source metadata and template content to the user. If the user
doesn't specify a key, ask which template they'd like to see, or suggest
running `templates list` first.

#### `templates eject <key>` or `templates eject --all`

**Before ejecting**, run with `--dry-run` to preview what will happen:

```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" --dry-run <key|--all>
```

Present the dry-run output to the user. If any templates already exist
(exit code 2), ask whether they want to overwrite. If the user confirms
overwriting, run a second dry-run with `--force` to show the full preview
(including which files will be overwritten):

```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" --dry-run --force <key|--all>
```

Present this preview, then run the actual eject with `--force`:

```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" --force <key|--all>
```

If no templates already exist (exit code 0 from the initial dry-run),
proceed directly with the eject (no `--force` needed):

```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" <key|--all>
```

If the user says `eject --all` or `eject all`, pass `--all` to the script.

After successful ejection, inform the user:
- Which file(s) were created and where
- That they can now edit the template(s) at the ejected path
- That the customised template will be used by the relevant skill on next
  invocation

#### `templates diff <key>`

Run the diff script:

```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-diff-template.sh" <key>
```

Present the diff output to the user. If exit code is 2, no customisation
exists — relay the "using plugin default" message.

#### `templates reset <key>`

This action removes a user's customised template to revert to the plugin
default. Reset operates on a **single template at a time** — if the user
requests resetting all templates, process them one-by-one with individual
confirmations.

1. Determine the template key. If not provided, ask the user.
2. Run the reset script without `--confirm` to check for an override:
   ```bash
   bash "$CLAUDE_PLUGIN_ROOT/scripts/config-reset-template.sh" <key>
   ```
3. If exit code 2: tell the user "No customised template found for '<key>'
   — already using plugin default."
4. If exit code 0: present the override information to the user and ask for
   confirmation. Show the file path and note about config entry if present.
   If the output includes "Warning: This file is outside the project
   directory", explicitly highlight this to the user and ask them to
   confirm they want to delete a file outside the project root.
5. On confirmation, run with `--confirm`:
   ```bash
   bash "$CLAUDE_PLUGIN_ROOT/scripts/config-reset-template.sh" --confirm <key>
   ```
6. Inform the user that the template was reset. If the script output
   includes a note about removing a config entry (i.e., the override was
   a config path / Tier 1), also remove the `templates.<key>` entry from
   the config using the Edit tool. Check both `.claude/accelerator.md`
   (team) and `.claude/accelerator.local.md` (local) for the entry:
   - If the entry exists in **local only**: remove it from local.
   - If the entry exists in **team only**: remove it from team.
   - If the entry exists in **both with the same value**: remove from both.
   - If the entry exists in **both with different values**: remove from
     local only (team config may affect other team members). Inform the
     user that the team config still has a `templates.<key>` entry and
     they should coordinate with their team if it should also be removed.

## Important Notes

- Config changes take effect on the next skill invocation (no restart needed
  for skills using the `!` preprocessor)
- The SessionStart hook summary requires a session restart to update
- `.local.md` files should be gitignored — the `create` action will help
  with this
- Team config should contain only project-relevant context, not personal
  preferences
