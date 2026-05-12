---
date: "2026-03-14T22:18:33+0000"
researcher: Toby Clemson
git_commit: N/A
branch: N/A
repository: .claude (user config directory)
topic: "Extracting skills and agents into a Claude Code plugin"
tags: [ research, plugin, skills, agents, extraction, accelerator ]
status: complete
last_updated: "2026-03-14"
last_updated_by: Toby Clemson
---

# Research: Extracting Skills and Agents into a Claude Code Plugin

**Date**: 2026-03-14T22:18:33+0000
**Researcher**: Toby Clemson
**Repository**: ~/.claude (user config directory)
**Target**: ~/Code/organisations/atomic/company/accelerator

## Research Question

How should the skills and agents in `~/.claude/` be extracted into a Claude Code
plugin at `~/Code/organisations/atomic/company/accelerator`?

## Summary

The `~/.claude/` directory contains **7 custom agents** and **19 skills**
(9 user-invocable, 7 review lenses, 2 output format specs, plus 1 helper
script). Claude Code has a mature, officially supported plugin system that maps
directly to these components. The extraction is straightforward: agents and
skills can be moved into a plugin directory with minimal modification, primarily
needing path references updated from `~/.claude/skills/` to
`${CLAUDE_PLUGIN_ROOT}/skills/`.

## Current State: ~/.claude/ Contents

### Agents (7 files in `~/.claude/agents/`)

| Agent                     | Tools                                                | Purpose                                           |
|---------------------------|------------------------------------------------------|---------------------------------------------------|
| `codebase-locator`        | Grep, Glob, LS                                       | Find WHERE files/components live                  |
| `codebase-analyser`       | Read, Grep, Glob, LS                                 | Analyse HOW code works                            |
| `codebase-pattern-finder` | Grep, Glob, Read, LS                                 | Find similar implementations                      |
| `documents-locator`       | Grep, Glob, LS                                       | Discover documents in `meta/`                     |
| `documents-analyser`      | Read, Grep, Glob, LS                                 | Extract insights from documents                   |
| `web-search-researcher`   | WebSearch, WebFetch, TodoWrite, Read, Grep, Glob, LS | Web research                                      |
| `reviewer`                | Read, Grep, Glob, LS                                 | Generic review agent parameterised by lens skills |

### Skills (19 SKILL.md files in `~/.claude/skills/`)

**User-invocable workflow skills (9):**

- `commit` — atomic git commits
- `create-plan` — interactive implementation planning
- `implement-plan` — execute approved plans phase-by-phase
- `validate-plan` — post-implementation verification
- `describe-pr` — generate PR descriptions from template
- `review-pr` — multi-lens PR review (orchestrator)
- `review-plan` — multi-lens plan review (orchestrator)
- `research-codebase` — parallel research with synthesis
- `respond-to-pr` — work through PR review feedback

**Review lenses (7, non-user-invocable):**

- `architecture-lens`, `security-lens`, `performance-lens`,
  `test-coverage-lens`, `code-quality-lens`, `standards-lens`, `usability-lens`

**Output format specifications (2, non-user-invocable):**

- `pr-review-output-format` — JSON schema for PR review comments
- `plan-review-output-format` — JSON schema for plan review findings

**Supporting script (1):**

- `research-codebase/scripts/research-metadata.sh`

### Key Dependencies Between Components

```
research-codebase ──spawns──> codebase-locator, codebase-analyser,
                              codebase-pattern-finder, documents-locator,
                              documents-analyser, web-search-researcher

review-pr ──spawns──> reviewer (×N, one per lens)
  └── each reviewer reads: [lens]-lens/SKILL.md + pr-review-output-format/SKILL.md

review-plan ──spawns──> reviewer (×N, one per lens)
  └── each reviewer reads: [lens]-lens/SKILL.md + plan-review-output-format/SKILL.md

respond-to-pr ──uses patterns from──> commit

create-plan ──spawns──> codebase-locator, codebase-analyser,
                        codebase-pattern-finder, documents-locator,
                        documents-analyser
```

### Path References Requiring Update

Skills reference other skills and scripts using absolute paths like:

- `~/.claude/skills/[lens]-lens/SKILL.md` (in review-pr, review-plan)
- `~/.claude/skills/pr-review-output-format/SKILL.md` (in review-pr)
- `~/.claude/skills/plan-review-output-format/SKILL.md` (in review-plan)
- `~/.claude/skills/research-codebase/scripts/research-metadata.sh` (in
  research-codebase)

These must become `${CLAUDE_PLUGIN_ROOT}/skills/...` paths.

## Plugin System: Key Facts

### Directory Structure

A Claude Code plugin is a directory with this layout:

```
plugin-name/
├── .claude-plugin/
│   └── plugin.json          # Manifest (name is the only required field)
├── agents/                  # Agent definitions (.md with YAML frontmatter)
├── skills/                  # Skill directories (each with SKILL.md)
├── hooks/                   # Optional hook configurations
├── .mcp.json                # Optional MCP server definitions
├── scripts/                 # Optional utility scripts
└── README.md
```

### Key Mechanisms

1. **Convention over configuration**: Components auto-discovered from standard
   directories. Manifest only needs `name`.
2. **`${CLAUDE_PLUGIN_ROOT}`**: Environment variable resolving to plugin root at
   runtime. Available in hook commands, MCP server args, scripts, and markdown
   content.
3. **Skill namespacing**: Skills in plugin `foo` are invoked as
   `/foo:skill-name`.
4. **Progressive loading**: Skill metadata always in context → SKILL.md loaded
   on
   trigger → bundled resources loaded on demand.
5. **Agent frontmatter**: Supports `name`, `description`, `tools`, plus optional
   `model` and `color`.

### Installation

```bash
# Development/testing (no install needed)
claude --plugin-dir ./my-plugin

# Install for user scope
claude plugin install <path-or-url> --scope user

# Reload after changes (inside TUI)
/reload-plugins
```

### Manifest Format (plugin.json)

```json
{
  "name": "accelerator",
  "version": "1.0.0",
  "description": "Development acceleration toolkit with multi-lens code review, implementation planning, and codebase research",
  "author": {
    "name": "Toby Clemson",
    "email": "tobyclemson@gmail.com"
  }
}
```

## Proposed Plugin Structure

```
~/Code/organisations/atomic/company/accelerator/
├── .claude-plugin/
│   └── plugin.json
├── agents/
│   ├── codebase-locator.md
│   ├── codebase-analyser.md
│   ├── codebase-pattern-finder.md
│   ├── documents-locator.md
│   ├── documents-analyser.md
│   ├── web-search-researcher.md
│   └── reviewer.md
├── skills/
│   ├── commit/
│   │   └── SKILL.md
│   ├── create-plan/
│   │   └── SKILL.md
│   ├── implement-plan/
│   │   └── SKILL.md
│   ├── validate-plan/
│   │   └── SKILL.md
│   ├── describe-pr/
│   │   └── SKILL.md
│   ├── review-pr/
│   │   └── SKILL.md
│   ├── review-plan/
│   │   └── SKILL.md
│   ├── research-codebase/
│   │   ├── SKILL.md
│   │   └── scripts/
│   │       └── research-metadata.sh
│   ├── respond-to-pr/
│   │   └── SKILL.md
│   ├── architecture-lens/
│   │   └── SKILL.md
│   ├── security-lens/
│   │   └── SKILL.md
│   ├── performance-lens/
│   │   └── SKILL.md
│   ├── test-coverage-lens/
│   │   └── SKILL.md
│   ├── code-quality-lens/
│   │   └── SKILL.md
│   ├── standards-lens/
│   │   └── SKILL.md
│   ├── usability-lens/
│   │   └── SKILL.md
│   ├── pr-review-output-format/
│   │   └── SKILL.md
│   └── plan-review-output-format/
│       └── SKILL.md
├── LICENSE
└── README.md
```

## Migration Considerations

### 1. Path Reference Updates

All `~/.claude/skills/` references in skill content must become
`${CLAUDE_PLUGIN_ROOT}/skills/`. Affected files:

- `review-pr/SKILL.md` — references lens skills and pr-review-output-format
- `review-plan/SKILL.md` — references lens skills and plan-review-output-format
- `research-codebase/SKILL.md` — references research-metadata.sh script

### 2. Skill Namespacing

Once in a plugin named `accelerator`, user-invocable skills will be invoked as:

- `/accelerator:commit` (instead of `/commit`)
- `/accelerator:review-pr` (instead of `/review-pr`)
- etc.

This is a significant UX change. Consider choosing a short plugin name to
minimise typing overhead.

### 3. Agent Discovery

Agents in `~/.claude/agents/` are currently available globally. Plugin agents
will also be globally available once the plugin is installed, but they will be
namespaced in the agent list display.

### 4. Settings Not Migrated

The `settings.json` file contains user-specific configuration (API keys,
permissions, MCP servers) that should NOT be part of the plugin. These remain in
`~/.claude/settings.json`.

### 5. Memory System Not Migrated

The memory system (`projects/*/memory/`) is user and project-specific and stays
in `~/.claude/`.

### 6. Meta Directory Convention

Skills reference `meta/` directories for output (plans, research, PR
descriptions). This convention assumes `meta/` exists in the project working
directory, not the plugin. No change needed — the skills already operate on the
current working directory.

### 7. Script Permissions

`research-metadata.sh` must remain executable. Ensure `chmod +x` after copying.

### 8. Cleanup After Extraction

Once the plugin is installed and verified, remove the original files from
`~/.claude/agents/` and `~/.claude/skills/` to avoid conflicts/duplication.

## Plan Options

### Option A: Direct Extraction (Recommended)

1. Create plugin directory structure at target path
2. Copy all 7 agents and 19 skills verbatim
3. Create minimal `plugin.json` manifest
4. Find-and-replace `~/.claude/skills/` → `${CLAUDE_PLUGIN_ROOT}/skills/` in
   all skill files
5. Find-and-replace `~/.claude/` → `${CLAUDE_PLUGIN_ROOT}/` for script paths
6. Initialize git repo
7. Test with
   `claude --plugin-dir ~/Code/organisations/atomic/company/accelerator`
8. Install with `claude plugin install`
9. Remove originals from `~/.claude/`

**Pros**: Simple, preserves all existing behavior, minimal risk.
**Cons**: No opportunity to restructure or improve.

### Option B: Extract with Restructuring

Same as Option A but also:

- Group related skills into logical subdirectories (review/, planning/, git/)
- Add a top-level CLAUDE.md to the plugin with usage documentation
- Add version to plugin.json
- Consider shorter aliases for frequently-used skills

**Pros**: Better organization for sharing.
**Cons**: More work, introduces potential for breakage in skill
cross-references.

### Option C: Extract as Marketplace Plugin

Same as Option A or B but additionally:

- Add `marketplace.json` for potential distribution
- Add LICENSE file
- Add comprehensive README.md
- Follow anthropic marketplace conventions for metadata

**Pros**: Ready for distribution if desired.
**Cons**: Additional overhead if distribution isn't a near-term goal.

## Recommendation

Start with **Option A** — direct extraction is lowest risk and can be enhanced
later. The plugin system supports iterative development: test with
`--plugin-dir`, use `/reload-plugins` for rapid iteration, and install when
stable. Restructuring (Option B) can happen as a follow-up.

## References

- [Plugins reference (complete spec)](https://code.claude.com/docs/en/plugins-reference)
- [Create plugins (tutorial)](https://code.claude.com/docs/en/plugins)
- [Discover and install plugins](https://code.claude.com/docs/en/discover-plugins)
- [Plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces)
- [anthropics/claude-plugins-official](https://github.com/anthropics/claude-plugins-official)
- [anthropics/claude-code/plugins/](https://github.com/anthropics/claude-code/tree/main/plugins)
- Local reference:
  `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/plugin-dev/skills/plugin-structure/SKILL.md`
- Local reference:
  `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/plugin-dev/skills/plugin-structure/references/manifest-reference.md`

## Open Questions

1. **Plugin name**: Should it be `accelerator` (matches repo) or something
   shorter like `accel` or `acc` to reduce typing for skill invocation?
2. **Distribution scope**: Is this plugin intended for personal use, team use
   within Atomic, or wider distribution?
3. **Existing plugins conflict**: The `~/.claude/plugins/marketplaces/` already
   contains plugins like `code-review` and `pr-review-toolkit` with overlapping
   functionality. Should those be disabled when this plugin is active?
