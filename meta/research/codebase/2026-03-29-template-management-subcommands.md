---
date: "2026-03-29T14:24:21+01:00"
researcher: Toby Clemson
git_commit: 508ca24b973d8c742e52e829c557f0c62f81076d
branch: main
repository: accelerator
topic: "Template management subcommands for the configure skill"
tags: [ research, codebase, templates, configure, configuration, customisation ]
status: complete
last_updated: 2026-03-29
last_updated_by: Toby Clemson
---

# Research: Template Management Subcommands for the Configure Skill

**Date**: 2026-03-29T14:24:21+01:00
**Researcher**: Toby Clemson
**Git Commit**: 508ca24b973d8c742e52e829c557f0c62f81076d
**Branch**: main
**Repository**: accelerator

## Research Question

The plugin's configuration system allows users to override built-in templates
for plans, research documents, ADRs, etc. However, this requires manual
creation of template files in the correct location. How should we add
subcommands to the `configure` skill for template management — specifically
`show` (display a template by key) and `eject` (copy a built-in template to
the user's templates directory for customisation)? What other template
subcommands would be useful?

## Summary

The existing template resolution infrastructure (`config-read-template.sh`)
already implements a clean three-tier fallback: per-template config path →
templates directory → plugin default. Adding template management subcommands
to the configure skill is straightforward because:

1. The built-in templates are standalone files in `templates/` (already
   extracted from inline skill content).
2. The resolution logic is well-encapsulated in shell scripts that can be
   reused or adapted.
3. The configure skill already dispatches on subcommand arguments
   (`view`/`create`/`help`), making it natural to add `templates`-prefixed
   subcommands.

The recommended subcommands are:

| Subcommand              | Purpose                                                                        |
|-------------------------|--------------------------------------------------------------------------------|
| `templates list`        | List all available template keys with their current resolution source          |
| `templates show <key>`  | Display the effective template content for a given key                         |
| `templates eject <key>` | Copy the built-in template to the user's templates directory for customisation |
| `templates diff <key>`  | Show differences between built-in and user's customised template               |
| `templates reset <key>` | Remove a user's customised template, reverting to the built-in default         |

## Detailed Findings

### Current Template System Architecture

#### Built-in Templates

Five template files ship with the plugin in `templates/`:

| Key              | File                          | Used By             |
|------------------|-------------------------------|---------------------|
| `plan`           | `templates/plan.md`           | `create-plan`       |
| `research`       | `templates/research.md`       | `research-codebase` |
| `adr`            | `templates/adr.md`            | `create-adr`        |
| `validation`     | `templates/validation.md`     | `validate-plan`     |
| `pr-description` | `templates/pr-description.md` | `describe-pr`       |

#### Resolution Mechanism

`scripts/config-read-template.sh` resolves templates using a three-tier
fallback (`config-read-template.sh:51-99`):

1. **Tier 1 — Config-specified path** (lines 53-66): Reads
   `templates.<name>` from config. If found, resolves relative to project
   root. Falls through with warning if file doesn't exist.

2. **Tier 2 — Templates directory** (lines 68-76): Reads
   `paths.templates` (default: `meta/templates`), looks for
   `<dir>/<name>.md`.

3. **Tier 3 — Plugin default** (lines 78-83): Falls back to
   `<plugin_root>/templates/<name>.md`.

The output is wrapped in markdown code fences by `_output_template()` (lines
36-49) so the LLM treats it as a template structure to follow.

#### Skills Consume Templates via Preprocessor

Skills embed template resolution in their SKILL.md files using the `!`
preprocessor directive:

```
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh adr`
```

This is resolved at prompt-expansion time, before the skill prompt reaches
the LLM.

#### Config Infrastructure

The relevant scripts for template management:

| Script                            | Role                                                           |
|-----------------------------------|----------------------------------------------------------------|
| `scripts/config-read-template.sh` | Resolves and outputs a template by name                        |
| `scripts/config-read-value.sh`    | Generic YAML frontmatter value reader                          |
| `scripts/config-read-path.sh`     | Reads `paths.*` config (thin wrapper)                          |
| `scripts/config-dump.sh`          | Dumps all config keys with effective values and sources        |
| `scripts/config-common.sh`        | Shared library: project root detection, frontmatter extraction |

`config-dump.sh` already enumerates template config keys
(`config-dump.sh:209-224`) and shows whether each has a configured override
or uses the default.

### Configure Skill Structure

The configure skill (`skills/config/configure/SKILL.md`) dispatches on a
single argument:

- `view` (or no argument with existing config) — displays configuration
- `create` (or no argument with no config) — interactive config creation
- `help` — displays full configuration reference

The skill uses `disable-model-invocation: true` in its frontmatter, meaning
the SKILL.md content itself is the response — it is not processed by the LLM
as a prompt. This is important: the skill's output is pure markdown
instructions that Claude follows.

#### Subcommand Pattern

The existing subcommands are dispatched via the "Based on the argument or
user intent" section. Each subcommand is documented with a `### heading` and
its behaviour is described in markdown. Adding new subcommands follows the
same pattern — add a new `###` heading under the "Available Actions" section.

### Proposed Template Subcommands

#### `templates list`

**Purpose**: Show all available template keys, where each currently resolves
from, and whether it has been customised.

**Implementation approach**: This is a pure-prompt subcommand — no new
scripts needed. The skill instructs Claude to:

1. Enumerate the known template keys: `plan`, `research`, `adr`,
   `validation`, `pr-description`.
2. For each key, determine the resolution source by checking:
  - Whether `templates.<key>` is set in config (read via
    `config-read-value.sh`)
  - Whether a file exists at `<paths.templates>/<key>.md`
  - Otherwise it resolves to the plugin default
3. Display as a table:

```
| Key            | Source         | Path                           |
|----------------|----------------|--------------------------------|
| plan           | plugin default | <plugin>/templates/plan.md     |
| research       | plugin default | <plugin>/templates/research.md |
| adr            | user override  | meta/templates/adr.md          |
| pr-description | config path    | custom/templates/pr.md         |
```

**Alternative**: Create a script `scripts/config-list-templates.sh` that
outputs this table directly. This would be more reliable than instructing the
LLM to probe each tier. The script could iterate over template files in
`<plugin_root>/templates/` (to discover all keys), then run the resolution
logic for each. This is the recommended approach.

#### `templates show <key>`

**Purpose**: Display the effective template content for a given key,
including the built-in default even if no override exists.

**Implementation approach**: Use the existing
`config-read-template.sh` script but output the raw content (without
code fence wrapping) for human readability. Options:

1. **Reuse existing script**: Call `config-read-template.sh <key>` and strip
   the code fences. Simple but slightly awkward (stripping fences that were
   just added).

2. **Add a `--raw` flag**: Modify `config-read-template.sh` to accept a
   `--raw` flag that skips the code fence wrapping. The skill would call
   `config-read-template.sh --raw <key>`.

3. **New script `config-show-template.sh`**: A thin wrapper that does the
   same three-tier resolution but outputs the raw file content plus metadata
   (source tier, file path).

Option 2 is recommended — it reuses the existing resolution logic with
minimal change.

**Alternatively**, the skill could simply instruct Claude to use the Read
tool to read the resolved template file. The skill prompt would describe the
resolution logic and tell Claude which file to read. This avoids any script
changes but is less reliable.

A hybrid approach: the skill calls a script that outputs both the source
path and the content, e.g.:

```
Source: plugin default (<plugin>/templates/plan.md)
---
<template content>
```

This gives Claude both the metadata and content to present to the user.

#### `templates eject <key>`

**Purpose**: Copy the built-in (plugin default) template to the user's
templates directory so they can customise it.

**Implementation approach**: This is the most impactful subcommand. The skill
instructs Claude to:

1. Determine the target directory: read `paths.templates` config (default:
   `meta/templates`).
2. Check if the template already exists at that location. If so, warn the
   user and ask for confirmation before overwriting.
3. Read the built-in template from `<plugin_root>/templates/<key>.md`.
4. Write it to `<templates_dir>/<key>.md`.
5. Inform the user that the template is now customisable at the ejected
   path.

**Script approach**: A script `scripts/config-eject-template.sh` could
handle the file copy and safety checks, outputting the result for Claude to
relay. However, since Claude already has the Write tool and the skill prompt
has `disable-model-invocation: true` set to false (i.e., Claude processes
this as instructions), the skill can simply instruct Claude to:

- Read the plugin default template (Read tool on
  `<CLAUDE_PLUGIN_ROOT>/templates/<key>.md`)
- Create the target directory if needed (Bash: `mkdir -p`)
- Write the content to the target location (Write tool)

This keeps the implementation simple — no new scripts, just prompt
instructions. The key benefit is that Claude can handle the confirmation
dialogue naturally ("This template already exists at... Do you want to
overwrite it?").

**Important consideration**: The configure skill currently has
`disable-model-invocation: true`, meaning it outputs its markdown directly
without LLM processing. For the `templates eject` subcommand to work (which
requires tool use — reading files, writing files, creating directories), the
skill would either need to:

a. Remove `disable-model-invocation: true` so the LLM processes the
instructions (affects all subcommands), or
b. Have the `templates` subcommands in a separate skill that doesn't have
this flag, or
c. Use a script-based approach where a shell script handles the eject
operation and the skill outputs the result directly.

Option (c) is the most consistent with the existing architecture — other
template operations are script-based. A `config-eject-template.sh` script
that:

- Accepts a template key
- Resolves the target directory from config
- Checks for existing file and exits with a status code (0 = ejected,
  1 = already exists, 2 = error)
- Copies the plugin default to the target directory
- Outputs status information

The skill prompt would then use a `!` preprocessor directive or instruct
Claude to run the script via Bash.

**However**, the `disable-model-invocation: true` flag means the skill
content IS the response — it's not interpreted as instructions for Claude.
This means the configure skill currently outputs its help text directly.
Looking more carefully at how the skill works: when the user runs
`/accelerator:configure help`, the SKILL.md content is expanded (preprocessor
directives run) and then displayed to Claude, who follows the instructions.

Actually, re-examining: `disable-model-invocation: true` means the skill's
content is returned without calling the model. So the configure skill outputs
markdown text directly. But the existing `view` and `create` subcommands
describe interactive workflows (e.g., "Ask whether they want to create a
team config or personal config"). This means the flag must not prevent
Claude from processing these instructions — rather, it likely means the
skill is loaded directly into context without a separate model call, and
Claude in the main conversation follows the instructions.

Given this, all subcommands can include instructions for Claude to use tools
(Read, Write, Bash) as part of the workflow. No architectural change needed.

#### `templates diff <key>`

**Purpose**: Show differences between the plugin's built-in template and the
user's customised version.

**Implementation approach**: Instruct Claude to:

1. Resolve the effective template (user's version) and the plugin default.
2. If no user override exists, inform the user ("No customised template
   found for '<key>' — using plugin default").
3. If a user override exists, run `diff` between the two files and present
   the result.

A script `config-diff-template.sh` could handle this cleanly:

- Accept a template key
- Find the user's template (Tier 1 or Tier 2 resolution)
- Find the plugin default (Tier 3)
- If no user template, output a message and exit
- Otherwise, output `diff -u <default> <user>` results

#### `templates reset <key>`

**Purpose**: Remove a user's customised template to revert to the built-in
default.

**Implementation approach**: Instruct Claude to:

1. Find where the user's template override lives (Tier 1 config path or
   Tier 2 templates directory).
2. If no override exists, inform the user.
3. If an override exists, confirm with the user before deleting.
4. If the override is via `templates.<key>` config, advise the user to also
   remove that config entry.

### Implementation Strategy

#### Approach A: Pure Prompt Instructions (No New Scripts)

Add the template subcommand descriptions to the configure skill's SKILL.md
as new `###` sections under "Available Actions". Each section describes what
Claude should do using its available tools (Read, Write, Bash).

**Pros**:

- No new scripts to maintain
- Natural conversational interaction (confirmations, error handling)
- Consistent with the configure skill's existing pattern

**Cons**:

- Relies on the LLM correctly following resolution logic
- Potentially fragile — the three-tier resolution logic is nuanced
- Template key enumeration must be hardcoded in the prompt (or the LLM
  must list files in `templates/`)

#### Approach B: Script-Backed Operations

Create new scripts that encapsulate the template management operations:

| Script                     | Subcommand        | Purpose                                      |
|----------------------------|-------------------|----------------------------------------------|
| `config-list-templates.sh` | `templates list`  | Enumerate templates with resolution status   |
| `config-show-template.sh`  | `templates show`  | Output template content with source metadata |
| `config-eject-template.sh` | `templates eject` | Copy plugin default to user templates dir    |
| `config-diff-template.sh`  | `templates diff`  | Diff user template against plugin default    |

The skill prompt instructs Claude to run these scripts via Bash and present
the results.

**Pros**:

- Reliable — resolution logic is in shell, not LLM interpretation
- Testable — scripts can be added to `test-config.sh`
- Consistent with existing script-per-operation architecture

**Cons**:

- More files to maintain
- `eject` needs to handle the confirmation flow (either script-side with
  flags or by splitting into check + execute)

#### Approach C: Hybrid (Recommended)

Use scripts for operations that need reliable resolution logic (`list`,
`show`, `diff`) and prompt instructions for operations that benefit from
interactivity (`eject`, `reset`).

For `eject` and `reset`, the skill prompt can instruct Claude to:

1. Use a script to determine the current state (does override exist? where
   is the plugin default?)
2. Present the situation to the user and ask for confirmation
3. Perform the file operation using tools (Write/Bash)

This gives the best of both worlds: reliable resolution via scripts and
natural interaction via the LLM.

### Subcommand Argument Design

The configure skill currently accepts a single argument
(`argument-hint: "[view | create | help]"`). Template subcommands need two
levels: the action and the template key.

Options for the invocation syntax:

1. **Nested subcommands**: `/accelerator:configure templates list`,
   `/accelerator:configure templates show plan`
  - Clean namespace, extensible
  -
  `argument-hint: "[view | create | help | templates [list | show | eject | diff | reset] [key]]"`

2. **Flat subcommands**: `/accelerator:configure template-list`,
   `/accelerator:configure template-show plan`
  - Simpler parsing but clutters the top-level namespace
  - Less extensible if more template operations are added

3. **Separate skill**: `/accelerator:templates list`,
   `/accelerator:templates show plan`
  - Clean separation of concerns
  - A new skill under `skills/config/templates/SKILL.md`
  - Can have its own `argument-hint` and settings

Option 1 (nested under configure) is the most natural extension. Option 3
(separate skill) is cleaner if template management grows complex but adds
another skill to discover. For now, option 1 is recommended, with the option
to extract to a separate skill later if warranted.

### Error Cases to Handle

- Unknown template key → list available keys
- `eject` when template already exists → warn and confirm
- `eject` when templates directory doesn't exist → create it
- `show` with config path that doesn't exist → show warning, fall through
  to show the effective template
- `reset` when no override exists → inform user
- `reset` when override is via `templates.<key>` config → advise removing
  the config entry too
- `diff` when no override exists → inform user (nothing to diff)

### Impact on `disable-model-invocation`

The configure skill has `disable-model-invocation: true`. Reviewing the
skill content, the `view` subcommand instructs Claude to read files and
display formatted output, and the `create` subcommand instructs Claude to
ask questions and write files. This means `disable-model-invocation: true`
does NOT prevent Claude from using tools — it likely only affects how the
skill is loaded (direct injection vs. separate model call).

The template subcommands that require tool use (eject, reset) will work
correctly within this framework.

## Code References

- `scripts/config-read-template.sh` — Template resolution script (3-tier
  fallback)
- `scripts/config-read-value.sh` — Generic config value reader
- `scripts/config-read-path.sh` — Path config reader (thin wrapper)
- `scripts/config-dump.sh:209-224` — Template config key enumeration
- `scripts/config-common.sh` — Shared config utilities
- `skills/config/configure/SKILL.md` — Configure skill (subcommand dispatch)
- `templates/plan.md` — Built-in plan template
- `templates/research.md` — Built-in research template
- `templates/adr.md` — Built-in ADR template
- `templates/validation.md` — Built-in validation template
- `templates/pr-description.md` — Built-in PR description template

## Architecture Insights

1. **Script-per-operation pattern**: The codebase consistently encapsulates
   each config operation in a dedicated script (`config-read-*.sh`). New
   template management operations should follow this pattern.

2. **Three-tier resolution is reusable**: The resolution logic in
   `config-read-template.sh` can be adapted for management scripts. The
   key insight is that `list` and `show` need to know WHICH tier resolved
   (not just the content), so scripts may need to output source metadata
   alongside content.

3. **Preprocessor as the skill-template bridge**: Skills consume templates
   via `!` preprocessor directives at prompt-expansion time. Template
   management subcommands operate on the same files but for human
   inspection/editing, not LLM consumption. This is a clean separation.

4. **`config-dump.sh` as precedent**: The dump script already iterates over
   template keys and shows their resolution status. A `config-list-templates.sh`
   script would be a refined version of this for user-facing output.

## Historical Context

- `meta/plans/2026-03-23-template-and-path-customisation.md` — The original
  plan that implemented the template override system. Phase 2 extracted
  templates from inline skill content; Phase 3 created
  `config-read-template.sh`; Phase 5 updated the configure skill's `help`
  text. The plan explicitly noted "Use the plugin's `templates/` directory
  as a starting point for customisation" but didn't provide tooling to
  facilitate this.

- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md` —
  Earlier research into customisation patterns that informed the template
  override design.

## Recommendations

### Minimum Viable Enhancement

Add three subcommands to the configure skill:

1. **`templates list`** — backed by a new `config-list-templates.sh` script
   that outputs a table of template keys, their resolution source (plugin
   default / user override / config path), and the resolved file path.

2. **`templates show <key>`** — backed by a `--raw` flag added to
   `config-read-template.sh` (or a new `config-show-template.sh`) that
   outputs the raw template content with source information.

3. **`templates eject <key>`** — backed by a new
   `config-eject-template.sh` script that copies the plugin default to the
   user's templates directory. The skill prompt handles the confirmation
   dialogue if the file already exists.

### Full Enhancement

Add all five subcommands (`list`, `show`, `eject`, `diff`, `reset`) as
described in the detailed findings. The `diff` and `reset` subcommands are
lower priority but complete the template management lifecycle.

### Implementation Order

1. `config-list-templates.sh` script + `templates list` subcommand
2. `config-show-template.sh` script (or `--raw` flag) + `templates show`
   subcommand
3. `config-eject-template.sh` script + `templates eject` subcommand
4. `config-diff-template.sh` script + `templates diff` subcommand
5. `templates reset` subcommand (prompt-only, no new script needed)
6. Update `argument-hint` in configure skill frontmatter
7. Update `help` subcommand with template management documentation
8. Add tests to `test-config.sh`

## Open Questions

1. **Separate skill vs. nested subcommand?** Should template management be a
   separate `/accelerator:templates` skill or nested under
   `/accelerator:configure templates`? Nesting is simpler; a separate skill
   is cleaner if the feature set grows.

2. **`eject` all?** Should `templates eject` support an `--all` flag to eject
   all templates at once? Useful for teams that want to customise everything.

3. **Template versioning**: When the plugin updates a built-in template,
   ejected templates won't get the update. Should `templates diff` or
   `templates list` highlight when an ejected template differs from the
   current plugin default? This could help users know when to re-eject and
   merge.

4. **`templates edit <key>`?** Should there be an `edit` subcommand that
   ejects (if not already done) and then opens the file for editing? This
   could be a combined eject + inform workflow.
