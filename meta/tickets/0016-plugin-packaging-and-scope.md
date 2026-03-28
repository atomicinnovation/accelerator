---
title: "Plugin packaging and scope boundary"
type: adr-creation-task
status: todo
---

# ADR Ticket: Plugin packaging and scope boundary

## Summary

In the context of skills and agents living in `~/.claude/` as personal
configuration, we decided for extraction into a standalone Claude Code plugin
named "accelerator" using `${CLAUDE_PLUGIN_ROOT}` for all internal references,
with a clear boundary where settings, keybindings, memory, and MCP configs
remain in `~/.claude/`, to achieve separation of reusable tooling from
user-specific configuration with location-independent resolution, accepting the
namespace prefix and a dependency on Claude Code's variable resolution.

## Context and Forces

- 7 agents and 19 skills were personal configuration in `~/.claude/`
- This prevented portability, version control, and distribution
- Claude Code's plugin system provides a mechanism for packaging and sharing
  skills and agents
- After extraction, skills change from `/skill-name` to
  `/accelerator:skill-name`
- Internal cross-references (lens paths, script paths, output format paths)
  must work regardless of install location
- Some configuration is inherently personal (settings, keybindings, memory) and
  should not be distributed with the plugin
- MCP server configs are environment-specific and belong to the user

## Decision Drivers

- Portability: the plugin must work when installed by any user
- Version control: the plugin should live in its own repository
- Distribution: others should be able to install and use the plugin
- Clean boundary: shareable tooling vs personal configuration
- Location independence: internal references must resolve regardless of install
  path

## Considered Options

1. **Stay in `~/.claude/`** — No extraction. Simple but not portable or
   shareable.
2. **Full extraction** — Move everything including settings and memory to the
   plugin. Mixes personal config with shareable tooling.
3. **Selective extraction with `${CLAUDE_PLUGIN_ROOT}`** — Extract agents and
   skills to a plugin; use `${CLAUDE_PLUGIN_ROOT}` for all internal references;
   keep settings, keybindings, memory, and MCP configs in `~/.claude/`.

## Decision

We will extract all agents and skills into a standalone Claude Code plugin
named "accelerator". All internal cross-references use
`${CLAUDE_PLUGIN_ROOT}` for location-independent resolution. Settings,
keybindings, the memory system, and MCP server configs remain in `~/.claude/`
as user-specific configuration. The plugin name "accelerator" was chosen for
clarity and discoverability, accepting the verbosity of
`/accelerator:skill-name`.

## Consequences

### Positive
- Plugin is portable, version-controlled, and distributable
- Clean boundary between shareable tooling and personal configuration
- `${CLAUDE_PLUGIN_ROOT}` enables location-independent resolution
- Plugin name is descriptive and discoverable

### Negative
- All skill invocations change from `/skill-name` to `/accelerator:skill-name`
- Dependency on Claude Code's `${CLAUDE_PLUGIN_ROOT}` variable resolution
- Open risk: `${CLAUDE_PLUGIN_ROOT}` may not resolve inside subagent prompt
  strings (fallback: pre-resolve to absolute paths)
- Users must configure settings separately from the plugin

### Neutral
- The plugin name can be changed later via a single-field change in
  `plugin.json`, though this affects user habits and documentation

## Source References

- `meta/research/2026-03-14-plugin-extraction.md` — Extraction analysis,
  scope boundary, and `${CLAUDE_PLUGIN_ROOT}` decision
- `meta/plans/2026-03-14-plugin-extraction.md` — Implementation plan with
  phased extraction approach
