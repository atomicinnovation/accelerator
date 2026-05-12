---
title: "Configuration system architecture"
type: adr-creation-task
status: done
---

# ADR Ticket: Configuration system architecture

## Summary

In the context of enabling userspace configuration with no built-in plugin
config mechanism, we decided for a hybrid strategy with two-tier config files
(`.claude/accelerator.md` for team, `.claude/accelerator.local.md` for
personal), the `` !`command` `` shell preprocessor as primary injection
mechanism, maximum two-level YAML nesting for bash parsability, and global-only
scope (no per-skill overrides), to achieve comprehensive configurability with
clean team/personal separation and reliable shell-based parsing, accepting
re-invocation for config changes, limited expressiveness, and no per-skill
fine-tuning.

## Context and Forces

- Claude Code has no built-in plugin configuration mechanism
- Skills are prompt text, not executable code — only three mechanisms for
  dynamic behaviour: `` !`command` `` preprocessor, natural language
  instructions, and hooks
- Teams need shared conventions (commit templates, review settings) that should
  be committed to VCS
- Individual developers need personal overrides (preferred agents, personal
  paths) that should not be committed
- Shell-based YAML parsing is inherently fragile for complex structures
- macOS ships with bash 3.2, constraining available shell features
- Per-skill configuration was considered but rejected as premature complexity

## Decision Drivers

- Deterministic configuration injection at skill load time
- Clean separation of team-shared vs personal configuration
- Reliable parsing with minimal dependencies (no external YAML parser)
- Simple mental model for users
- Comprehensive configurability without platform changes

## Considered Options

For config files:
1. **Single file** — One config file for everything. No team/personal
   separation.
2. **Two-tier** — `.claude/accelerator.md` (team, committed) +
   `.claude/accelerator.local.md` (personal, auto-gitignored by Claude Code).
   Last-writer-wins precedence.
3. **Environment variables** — Shell-native but poor discoverability and no
   VCS integration.

For injection mechanism:
1. **Natural language instructions** — Non-deterministic; the LLM interprets
   the instruction.
2. **Hooks** — Run at specific events. Too coarse for per-skill config.
3. **Shell preprocessor** — `` !`command` `` runs at skill load time. Deterministic,
   reliable, but config changes require re-invocation.

For schema constraints:
1. **Full YAML** — Maximum expressiveness. Unreliable in bash.
2. **Two-level nesting** — `section.key` with simple scalar values only.
   Reliable with awk-based extraction on macOS bash 3.2.
3. **Flat key-value** — Too restrictive for structured configuration.

## Decision

We will use a hybrid strategy: two-tier config files (team + personal with
last-writer-wins), the shell preprocessor as the primary injection mechanism,
and maximum two-level YAML nesting (`section.key` with simple scalars). Config
scope is global-only — all settings apply uniformly across all skills. No
external YAML parser dependency; parsing uses awk-based extraction.

## Consequences

### Positive
- Clean team/personal separation with natural VCS integration
- Deterministic config injection at skill load time
- Reliable parsing on macOS bash 3.2 with no external dependencies
- Simple mental model: one namespace, two files, two levels deep

### Negative
- Config changes require re-invoking the skill (not mid-conversation updates)
- No per-skill overrides — all config is global
- No multi-line block scalars or 3+ level nesting
- No sentinel mechanism to unset a team-level value from personal config
- Shell-based YAML parsing is inherently fragile compared to a proper parser

### Neutral
- The two-level constraint may need relaxing if future configuration needs
  grow, potentially requiring a migration to a more robust parser
- Global-only scope is an explicit simplicity decision, not a permanent
  constraint

## Source References

- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md` —
  Customisation strategy analysis and preprocessor selection
- `meta/plans/2026-03-23-config-infrastructure.md` — Two-tier config files,
  YAML nesting constraints, and global scope decision
