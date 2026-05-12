---
date: "2026-03-22T21:09:57+0000"
researcher: Toby Clemson
git_commit: 662188f6f566501718337baf446305a48783f69c
branch: main
repository: accelerator
topic: "Skill customisation and override patterns for Claude Code plugins"
tags: [ research, plugin, skills, customisation, overrides, configuration, agents ]
status: complete
last_updated: "2026-03-22"
last_updated_by: Toby Clemson
---

# Research: Skill Customisation and Override Patterns for Claude Code Plugins

**Date**: 2026-03-22T21:09:57+0000
**Researcher**: Toby Clemson
**Git Commit**: 662188f6f566501718337baf446305a48783f69c
**Branch**: main
**Repository**: accelerator

## Research Question

How can the Accelerator plugin be updated so that users can override certain
aspects of the skills it contains -- such as agent names/definitions, rules,
suggestions, details, numeric limits, and tech-stack-specific context -- without
modifying the plugin source?

## Summary

Claude Code's plugin system has **no built-in mechanism for typed plugin
configuration or skill overrides**. However, Anthropic's official `plugin-dev`
toolkit documents a **convention-based pattern** using
`.claude/<plugin-name>.local.md` files with YAML frontmatter as the recommended
way for plugins to support user customisation. Combined with the skill
preprocessor's `` !`command` `` syntax (which can read configuration files at
skill load time), and the hooks system's ability to inject context via
`additionalContext` and `CLAUDE_ENV_FILE`, there are several viable strategies
for adding customisation to the Accelerator plugin.

The codebase analysis identified **40+ distinct customisation points** across
skills -- from agent names and numeric limits to document templates and verdict
rules -- none of which currently have any override mechanism.

The broader ecosystem (Cursor, Cline, Copilot, Continue.dev, Agent Zero, Goose)
shows a consistent pattern: **file-based configuration with hierarchical
discovery and convention-over-configuration**. No tool does structural merging
of markdown prompt sections; they all either replace entire files or concatenate
additive instructions.

## Detailed Findings

### 1. Claude Code Plugin System Capabilities

#### What the platform provides

| Mechanism               | Description                                                            | Available in skills?       |
|-------------------------|------------------------------------------------------------------------|----------------------------|
| `${CLAUDE_PLUGIN_ROOT}` | Absolute path to plugin installation dir                               | Yes (inline substitution)  |
| `${CLAUDE_PLUGIN_DATA}` | Persistent data dir surviving updates (`~/.claude/plugins/data/{id}/`) | Yes (inline substitution)  |
| `${CLAUDE_SKILL_DIR}`   | Directory containing the skill's SKILL.md                              | Yes (inline substitution)  |
| `$ARGUMENTS` / `$N`     | User arguments passed to the skill                                     | Yes                        |
| `` !`command` ``        | Shell preprocessor -- runs command, injects stdout into skill content  | Yes                        |
| `additionalContext`     | Hook output field injected into Claude's context                       | Via hooks only             |
| `CLAUDE_ENV_FILE`       | Env var persistence file (SessionStart hooks only)                     | Indirectly (via Bash tool) |
| `env` in settings.json  | Environment variables set at user/project level                        | Via Bash tool              |

#### What the platform does NOT provide

- No `configuration` or `parameters` schema in `plugin.json`
- No plugin-specific settings section in user/project settings.json
- No mechanism for a project to pass structured config to a plugin
- No skill inheritance or override mechanism
- No way to extend or modify an installed plugin's skills without editing cached
  files
- Plugin `settings.json` only supports the `agent` key

#### Known bugs relevant to plugin configuration

- **Issue #27145**: `CLAUDE_PLUGIN_ROOT` not set for SessionStart hooks
- **Issue #11927**: `env` from settings.json not reliably passed to plugin MCP
  servers
- **Issue #9354**: `CLAUDE_PLUGIN_ROOT` not expanded in command markdown files

### 2. The `.local.md` Convention (Official Pattern)

Anthropic's `plugin-dev` toolkit (shipped in `claude-plugins-official`)
documents this pattern:

**File**: `.claude/<plugin-name>.local.md` in the project directory
**Format**: YAML frontmatter for structured settings, markdown body for notes

```yaml
---
enabled: true
strict_mode: false
max_retries: 3
notification_level: info
---

# Plugin Configuration

This plugin is configured for standard validation mode.
```

**Key characteristics**:

- `.local.md` suffix is auto-gitignored by Claude Code
- Read by hooks via shell `sed`/`awk` parsing
- Read by skills via `` !`command` `` preprocessor or by Claude reading the file
- Changes require Claude Code restart for hooks to pick up
- The `plugin-dev` toolkit provides utility scripts (`parse-frontmatter.sh`,
  `validate-settings.sh`)
- This is **purely convention** -- each plugin must implement its own reading
  logic

**Sources**:

- [Plugin Settings SKILL.md](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/plugin-settings/SKILL.md)
- [Create Settings Command Example](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/plugin-settings/examples/create-settings-command.md)

### 3. Patterns from the Broader Ecosystem

#### 3a. File-based override by placement (Agent Zero / Goose)

Users place override files in a known directory. If the file exists, it replaces
the default. No merging -- first-match resolution.

- **Agent Zero**: `agents/<profile>/prompts/` overrides
  `agents/default/prompts/`.
  Each prompt file (e.g., `agent.system.main.role.md`) can be replaced entirely.
  Variables use `{{variable}}` syntax, replaced at runtime.
- **Goose**: Templates in `~/.config/goose/prompts/` override embedded defaults.
  Uses Jinja2 syntax. Deleting a local file reverts to the built-in default.

**Applicability**: High. This maps directly to how Accelerator skills work --
each skill is a self-contained markdown file that could be overridden by placing
a replacement in a project-level directory.

#### 3b. Hierarchical rule discovery (Cursor / Cline / Continue / Copilot)

Multiple tools use a directory of markdown rule files discovered by convention:

| Tool     | Discovery Location                       | Format                                                |
|----------|------------------------------------------|-------------------------------------------------------|
| Cursor   | `.cursor/rules/*.mdc`                    | Markdown with YAML frontmatter (globs for scoping)    |
| Cline    | `.clinerules/*.md`                       | Markdown, numeric prefix for ordering                 |
| Continue | `.continue/rules/*.md`                   | Markdown with frontmatter (globs, regex, alwaysApply) |
| Copilot  | `.github/instructions/*.instructions.md` | Markdown with path-scoping YAML                       |
| Roo Code | `.roo/rules-{mode}/*.md`                 | Markdown, mode-specific directories                   |

**Applicability**: Medium. This pattern is more about adding context/rules than
overriding specific skill behaviour. However, it could inform how users provide
tech-stack-specific context that skills incorporate.

#### 3c. Cascading configuration (Codex / Terraform / CSS)

Configuration defined at multiple levels with deterministic precedence:

- Codex: `~/.codex/config.toml` < `.codex/config.toml` (walking to CWD) < CLI
  flags
- Terraform: Module defaults < environment-specific inputs
- CSS: Cascade layers with specificity rules

**Applicability**: Medium. Useful for structured key-value settings (numeric
limits, agent names) but overkill for full skill override.

#### 3d. Template variable substitution (Jinja2 / Semantic Kernel / Bedrock)

Define placeholders in templates that are filled at runtime from configuration:

- Jinja2: `{{ variable }}` with block inheritance (`{% block %}`)
- Semantic Kernel: `PromptTemplateConfig` with multiple template format support
- Amazon Bedrock: `$variable$` placeholders with per-step override flags

**Applicability**: High for specific values (agent names, limits, paths) but
complex to implement within Claude Code's skill markdown format.

### 4. Current Customisation Points in Accelerator

The codebase analysis identified every potential customisation point. Here are
the most impactful categories:

#### 4a. Agent names (7 agents referenced across 10+ skills)

| Agent Name                | Referenced By                                                                      |
|---------------------------|------------------------------------------------------------------------------------|
| `reviewer`                | `review-pr`, `review-plan`                                                         |
| `codebase-locator`        | `create-plan`, `stress-test-plan`, `research-codebase`, `create-adr`, `review-adr` |
| `codebase-analyser`       | `create-plan`, `stress-test-plan`, `research-codebase`                             |
| `codebase-pattern-finder` | `create-plan`, `research-codebase`                                                 |
| `documents-locator`       | `create-plan`, `research-codebase`, `extract-adrs`, `create-adr`, `review-adr`     |
| `documents-analyser`      | `create-plan`, `research-codebase`, `extract-adrs`                                 |
| `web-search-researcher`   | `research-codebase`                                                                |

#### 4b. Numeric limits

| Value | Location                                             | Description            |
|-------|------------------------------------------------------|------------------------|
| 10    | `review-pr/SKILL.md:326-327`                         | Max inline PR comments |
| 6-8   | `review-pr/SKILL.md:167`, `review-plan/SKILL.md:122` | Target lens count      |
| 4     | `review-pr/SKILL.md:184`, `review-plan/SKILL.md:139` | Minimum lens count     |
| 3     | `review-pr/SKILL.md:309`                             | Dedup line proximity   |
| 5     | `scripts/vcs-log.sh:10,12`                           | Recent commits shown   |

#### 4c. Review lens catalogue (13 lenses, hardcoded in orchestrators)

The set of available lenses, their auto-detect criteria, the "core four"
concept,
and selection rules are all hardcoded in `review-pr/SKILL.md` and
`review-plan/SKILL.md`.

#### 4d. Document templates and file paths

Plan, research, ADR, and PR description templates are fully hardcoded within
their respective skills. Output paths (`meta/plans/`, `meta/research/codebase/`,
`meta/decisions/`, `meta/prs/`) are hardcoded conventions.

#### 4e. Verdict/decision rules

PR verdicts (APPROVE/COMMENT/REQUEST_CHANGES) and plan verdicts
(APPROVE/COMMENT/REVISE) have hardcoded thresholds based on finding severity.

#### 4f. Response style and conventions

Commit attribution rules, PR response tone, emoji usage, and review comment
structure are all hardcoded in their respective skills.

### 5. Viable Implementation Strategies

Based on the platform capabilities and ecosystem patterns, here are the viable
approaches ranked by feasibility and power:

#### Strategy A: `.local.md` Configuration + `!` Preprocessor (Recommended)

**How it works**: Users create `.claude/accelerator.local.md` (or
`.claude/accelerator.yml`) with YAML frontmatter containing overridable
settings.
Skills use the `` !`command` `` preprocessor to read specific values at load
time.
A setup skill (`/accelerator:configure`) helps users create and edit the config.

**Configuration file** (`.claude/accelerator.local.md`):

```yaml
---
# Agent overrides (swap agent implementations)
agents:
  reviewer: my-custom-reviewer
  codebase-locator: my-locator-agent

# Review settings
review:
  max_inline_comments: 15
  min_lenses: 3
  max_lenses: 10
  core_lenses: [ architecture, security, test-coverage, correctness ]
  disabled_lenses: [ portability, compatibility ]

# Commit settings
commit:
  include_co_author: false
  staging_policy: specific-files

# File path conventions
paths:
  plans: meta/plans
  research: meta/research/codebase
  decisions: docs/decisions
  pr_descriptions: meta/prs
---

# Accelerator Configuration

Additional context for skills to consider:
  - We use a monorepo with Bazel build system
  - Our API uses GraphQL, not REST
  - All database migrations must be backward-compatible
```

**Skill integration**: Each skill includes a preprocessor line that reads the
config:

```markdown
## Configuration

!`${CLAUDE_PLUGIN_ROOT}/scripts/read-config.sh review.max_inline_comments 10`
```

The `read-config.sh` script reads the YAML frontmatter, extracts the requested
key, and falls back to the default value if not set.

**Pros**:

- Follows Anthropic's official recommended pattern
- Works with current platform capabilities
- `.local.md` is auto-gitignored
- Markdown body provides free-form context injection
- Can be created interactively via a setup skill
- No changes needed to how Claude Code loads skills

**Cons**:

- Requires implementing YAML parsing in shell
- The `` !`command` `` preprocessor runs at skill load time, not at runtime
- Config changes require re-invoking the skill (not mid-conversation)
- Structured YAML parsing in bash is fragile for nested structures

#### Strategy B: Convention Directory with Partial Overrides

**How it works**: Users create files in `.claude/accelerator/` that override
specific aspects of skills. The plugin's SessionStart hook detects these files
and injects context about available overrides.

**Directory structure**:

```
.claude/accelerator/
  config.yml                  # Structured settings (limits, agent names, paths)
  context.md                  # Tech stack context injected into all skills
  lenses/
    custom-lens/SKILL.md      # User-defined custom lens
  templates/
    plan.md                   # Override plan template
    research.md               # Override research template
    adr.md                    # Override ADR template
  rules/
    review-rules.md           # Additional review guidelines
    commit-rules.md           # Additional commit conventions
```

**How overrides are detected**: The SessionStart hook scans for
`.claude/accelerator/` and injects a summary of available overrides:

```
Accelerator user overrides detected:
- Custom context: .claude/accelerator/context.md
- Config overrides: agent names, review limits
- Custom lens: my-domain-lens
- Template override: plan template
```

Skills then check for overrides before using defaults:

```markdown
## Plan Template

!
`if [ -f .claude/accelerator/templates/plan.md ]; then cat .claude/accelerator/templates/plan.md; else cat ${CLAUDE_PLUGIN_ROOT}/skills/planning/create-plan/templates/default-plan.md; fi`
```

**Pros**:

- Maximum flexibility -- users can override entire templates, add lenses, inject
  context
- Clean separation between config (YAML), context (markdown), and templates
- Lenses and templates are full files, not parsed fragments
- Aligns with Agent Zero's profile-based override model
- Supports both structured settings and free-form context

**Cons**:

- More complex to implement (SessionStart hook + per-skill override detection)
- Users need to understand the directory structure
- Template overrides are all-or-nothing (no partial section override)
- Requires extracting default templates from skills into separate files

#### Strategy C: Layered Context Injection via CLAUDE.md

**How it works**: Skills instruct Claude to look for project-level context in
CLAUDE.md or `.claude/rules/` files before applying defaults. No structural
changes to skills -- just added instructions.

**Example addition to each skill**:

```markdown
## Project-Specific Context

Before applying default rules, check for project-specific overrides:

1. Read `.claude/rules/accelerator-review.md` if it exists -- apply these rules
   in addition to the defaults below
2. Read `.claude/rules/accelerator-agents.md` if it exists -- use any agent name
   mappings specified there
3. Check CLAUDE.md for any accelerator-specific instructions

If no overrides are found, use the defaults below.
```

**Pros**:

- Simplest to implement -- just add instructions to existing skills
- Works with Claude Code's existing CLAUDE.md and rules infrastructure
- No new scripts or hooks needed
- Users familiar with CLAUDE.md already know how to use this

**Cons**:

- Relies on Claude interpreting instructions rather than deterministic config
- No validation -- Claude might misinterpret override instructions
- Agent name overrides can't work this way (the Agent tool needs exact names)
- No structured settings -- everything is natural language guidance
- Less reliable for numeric values and exact specifications

#### Strategy D: Hybrid (Recommended for Accelerator)

Combine Strategies A and B with elements of C:

**Structured settings** (`.claude/accelerator.local.md` or
`.claude/accelerator/config.yml`): For agent names, numeric limits, file paths,
and other machine-readable configuration. Read by scripts via `` !`command` ``.

**Context injection** (`.claude/accelerator/context.md` or markdown body of
`.local.md`): For tech-stack-specific guidance, coding conventions, domain
context. Injected into skills that benefit from project awareness (create-plan,
research-codebase, review-pr).

**Template overrides** (`.claude/accelerator/templates/`): For users who want to
completely replace document templates (plan, research, ADR formats). Full file
replacement with fallback to defaults.

**Custom lenses** (`.claude/accelerator/lenses/`): For users who want to add
domain-specific review lenses (e.g., `accessibility-lens`, `compliance-lens`).
Discovered by the review orchestrators alongside built-in lenses.

**Setup skill** (`/accelerator:configure`): Interactive configuration that walks
users through available settings and writes the config file.

### 6. Implementation Considerations

#### 6a. The `` !`command` `` preprocessor

This is the most powerful mechanism for reading configuration. It runs shell
commands before skill content reaches Claude, injecting the output inline.

```markdown
## User Configuration

!`${CLAUDE_PLUGIN_ROOT}/scripts/load-config.sh`
```

The `load-config.sh` script could:

1. Check for `.claude/accelerator.local.md` or `.claude/accelerator/config.yml`
2. Parse YAML frontmatter
3. Output a structured summary of active overrides
4. Fall back to defaults for any unset values

**Limitation**: The preprocessor runs at skill invocation time. It cannot
re-read configuration mid-conversation. This is acceptable for most settings.

#### 6b. Agent name overrides

Agent names are passed to the `Agent` tool's `subagent_type` parameter. For a
user to swap agent implementations, the skill text must contain the correct
agent
name at the point where it instructs Claude to spawn agents.

Two approaches:

1. **Preprocessor injection**:
   `` !`${CLAUDE_PLUGIN_ROOT}/scripts/agent-name.sh reviewer` ``
   outputs the configured agent name or falls back to `reviewer`
2. **Context instruction**: "If the user has configured a custom agent for the
   'reviewer' role in their accelerator config, use that agent name instead"

Approach 1 is more reliable since it produces deterministic output.

#### 6c. Context injection for tech-stack specificity

The most valuable customisation for most users. Skills like `create-plan`,
`review-pr`, and `research-codebase` would benefit from knowing:

- Build system (Make, Bazel, Gradle, etc.)
- Testing framework
- API style (REST, GraphQL, gRPC)
- Database technology
- Deployment target

This is best handled by the markdown body of the config file or a dedicated
`context.md` file, injected into skills via the preprocessor.

#### 6d. Custom lenses

The review orchestrators (review-pr, review-plan) hardcode the lens catalogue.
To support custom lenses:

1. Extract the lens catalogue into a discoverable format
2. At skill load time, scan `.claude/accelerator/lenses/` for additional lens
   directories containing SKILL.md files
3. Merge discovered lenses into the catalogue presented to the user

This requires the orchestrator skills to use `` !`command` `` to dynamically
build the lens table.

#### 6e. Template overrides

Skills embed document templates inline. To support overrides:

1. Extract default templates into separate files under
   `${CLAUDE_PLUGIN_ROOT}/templates/`
2. Use `` !`command` `` to check for user overrides first, falling back to
   defaults:
   ```markdown
   !`if [ -f .claude/accelerator/templates/plan.md ]; then cat .claude/accelerator/templates/plan.md; else cat ${CLAUDE_PLUGIN_ROOT}/templates/plan.md; fi`
   ```

### 7. Ecosystem Comparison Table

| Tool                      | Config Format         | Discovery                   | Override Model        | Structured Settings         |
|---------------------------|-----------------------|-----------------------------|-----------------------|-----------------------------|
| Claude Code (Accelerator) | `.local.md` + YAML    | `.claude/` directory        | Proposed              | Via YAML frontmatter        |
| Cursor                    | `.mdc`                | `.cursor/rules/`            | Additive              | YAML frontmatter (globs)    |
| Cline                     | `.md` / `.txt`        | `.clinerules/`              | Additive + toggle     | YAML frontmatter (paths)    |
| Continue.dev              | `.md` + `config.yaml` | `.continue/rules/`          | Additive + hub refs   | YAML (globs, regex)         |
| Copilot                   | `.instructions.md`    | `.github/instructions/`     | Additive              | YAML frontmatter (paths)    |
| Agent Zero                | `.md` per prompt      | `agents/<profile>/prompts/` | Full file replacement | `{{variable}}` substitution |
| Goose                     | `.md` (Jinja2)        | `~/.config/goose/prompts/`  | Full file replacement | Jinja2 variables            |
| Codex                     | `.toml`               | `.codex/` (dir walking)     | Key-value override    | TOML                        |
| Terraform                 | `.tf` variables       | Module directories          | Input override        | HCL variables               |

## Architecture Insights

### The fundamental constraint

Claude Code skills are **prompt text**, not executable code. They can't
programmatically read configuration, branch on values, or dynamically compose
themselves. The only mechanisms for dynamic behaviour are:

1. **`` !`command` `` preprocessor** -- runs shell commands and injects output
   at
   skill load time
2. **Natural language instructions** -- telling Claude to look for and act on
   configuration
3. **Hooks** -- injecting context at session or tool lifecycle events

This means any customisation system must work within these constraints. The
preprocessor is the most powerful and reliable mechanism.

### The "replace vs. extend" decision

Every override system must decide: does a user override **replace** the default
or **extend** it?

- **Replace**: Simpler, more predictable, but users must duplicate the full
  template to change one section
- **Extend/Merge**: More ergonomic for small changes, but requires a merge
  strategy and risks conflicts

The ecosystem consensus is clear: **replace entire files, extend with additive
rules**. No tool attempts structural merging of markdown sections. This is the
pragmatic choice for Accelerator too.

### Configuration surface prioritisation

Not all customisation points are equally valuable. Based on likely user needs:

**High value** (solve real user problems):

- Tech-stack context injection (every project is different)
- Agent name overrides (users may have custom agents)
- Custom review lenses (domain-specific quality concerns)
- Document template overrides (different team conventions)

**Medium value** (power user needs):

- Numeric limits (inline comment cap, lens count)
- File path conventions (some teams use `docs/` not `meta/`)
- Verdict threshold rules
- Lens selection criteria

**Low value** (rarely needed):

- Emoji customisation
- Severity/confidence level names
- Footer text
- Commit attribution rules (should remain non-negotiable)

## Code References

- Plugin manifest: `.claude-plugin/plugin.json:1-18`
- Hook configuration: `hooks/hooks.json:1-26`
- Review PR skill (lens catalogue, limits, verdict rules):
  `skills/github/review-pr/SKILL.md:48-63,167-184,326-334`
- Review plan skill (parallel structure):
  `skills/planning/review-plan/SKILL.md:42-57,122-139`
- Create plan skill (agent references, template):
  `skills/planning/create-plan/SKILL.md:64-69,209-310`
- Research codebase skill (agent references, template):
  `skills/research/research-codebase/SKILL.md:54-67,110-166`
- Commit skill (preprocessor usage, conventions):
  `skills/vcs/commit/SKILL.md:11-12,44-49`
- Reviewer agent definition: `agents/reviewer.md:1-49`
- VCS detection hook: `hooks/vcs-detect.sh` (referenced in hooks.json)
- PR review output format:
  `skills/review/output-formats/pr-review-output-format/SKILL.md`
- Plan review output format:
  `skills/review/output-formats/plan-review-output-format/SKILL.md`
- ADR skills (template, lifecycle, numbering):
  `skills/decisions/create-adr/SKILL.md`,
  `skills/decisions/extract-adrs/SKILL.md`,
  `skills/decisions/review-adr/SKILL.md`

## Related Research

- `meta/research/codebase/2026-03-14-plugin-extraction.md` -- Original plugin extraction
  research documenting plugin system capabilities and migration from
  `~/.claude/`

## Key External References

### Official Documentation

- [Create plugins - Claude Code Docs](https://code.claude.com/docs/en/plugins)
- [Plugins reference - Claude Code Docs](https://code.claude.com/docs/en/plugins-reference)
- [Extend Claude with skills](https://code.claude.com/docs/en/skills)
- [Hooks reference](https://code.claude.com/docs/en/hooks)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Memory (CLAUDE.md)](https://code.claude.com/docs/en/memory)

### Plugin Development

- [Plugin Settings SKILL.md (plugin-dev)](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/plugin-settings/SKILL.md)
- [Create Settings Command Example](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/plugin-settings/examples/create-settings-command.md)
- [plugin-dev toolkit](https://github.com/anthropics/claude-code/tree/main/plugins/plugin-dev)

### Ecosystem Patterns

- [Cursor Rules for AI](https://docs.cursor.com/context/rules-for-ai)
- [Cline Rules Overview](https://docs.cline.bot/features/cline-rules/overview)
- [Continue.dev Rules Deep Dive](https://docs.continue.dev/customize/deep-dives/rules)
- [GitHub Copilot Custom Instructions](https://docs.github.com/copilot/customizing-copilot/adding-custom-instructions-for-github-copilot)
- [Agent Zero Custom Profiles](https://deepwiki.com/agent0ai/agent-zero/17.2-custom-agent-profiles)
- [Goose Prompt Templates](https://block.github.io/goose/docs/guides/prompt-templates/)
- [Codex Advanced Configuration](https://developers.openai.com/codex/config-advanced)
- [Roo Code Custom Instructions](https://docs.roocode.com/features/custom-instructions)

### Known Issues

- [Issue #27145: CLAUDE_PLUGIN_ROOT not set for SessionStart hooks](https://github.com/anthropics/claude-code/issues/27145)
- [Issue #11927: env vars not passed to plugin MCPs](https://github.com/anthropics/claude-code/issues/11927)
- [Issue #9354: CLAUDE_PLUGIN_ROOT in command markdown](https://github.com/anthropics/claude-code/issues/9354)

### Community Resources

- [awesome-claude-code-plugins](https://github.com/ccplugins/awesome-claude-code-plugins)
- [claude-plugins.dev](https://claude-plugins.dev/)
- [Claude Code Extensibility Guide](https://happysathya.github.io/claude-code-extensibility-guide.html)
- [Claude Agent Skills Deep Dive](https://leehanchung.github.io/blogs/2025/10/26/claude-skills-deep-dive/)

## Open Questions

1. **Config file format**: Should the configuration be
   `.claude/accelerator.local.md`
   (following the official convention) or `.claude/accelerator/config.yml` (more
   structured, supports the override directory pattern)? Or both, with the
   directory approach being the "full" customisation and `.local.md` being the
   "quick" option?

2. **Override granularity for templates**: Should template overrides be
   all-or-nothing file replacement, or should we support section-level overrides
   (e.g., override just the plan's "Success Criteria" section while keeping the
   rest of the default template)? The ecosystem consensus is file-level
   replacement, but section-level would be more ergonomic.

3. **Custom lens discovery**: Should custom lenses be auto-discovered from the
   override directory and merged into the catalogue, or should users explicitly
   register them in the config file? Auto-discovery is more ergonomic but less
   explicit.

4. **Agent override scope**: Should agent name overrides apply globally (all
   skills use the overridden agent) or per-skill (override the agent only when
   used by `review-pr`)? Global is simpler; per-skill is more flexible.

5. **Backward compatibility**: When default templates or settings change in a
   plugin update, how should user overrides interact? Should there be versioning
   of the config schema?

6. **Non-gitignored config**: The `.local.md` convention auto-gitignores the
   file. But team-shared configuration (e.g., "our project uses Bazel") should
   be committed. Should there be both `.claude/accelerator.local.md` (personal)
   and `.claude/accelerator.md` (team-shared)?
