---
title: "Configuration extension points: templates, agents, and custom lenses"
type: adr-creation-task
status: done
---

# ADR Ticket: Configuration extension points: templates, agents, and custom lenses

## Summary

In the context of allowing users to customise templates, agent assignments, and
review lenses, we decided for file-level template replacement with three-tier
resolution (explicit config path, templates directory, plugin default), a dual
agent name resolution strategy (deterministic inline scripts for critical
`subagent_type`, labeled variable definitions for prose references), and
auto-discovery of custom lenses from `.claude/accelerator/lenses/*/SKILL.md`,
to achieve ergonomic extensibility where dropping a file suffices, accepting
full-template duplication for partial changes, semi-deterministic LLM
interpretation for most agent references, and structural validation overhead.

## Context and Forces

- Users need to customise document templates (plans, research, ADRs,
  validations) for their team's conventions
- No tool in the ecosystem attempts structural merging of markdown sections —
  file-level replacement is the consensus approach
- Agent names are referenced in two ways: as `subagent_type` parameters (must
  be exact strings) and in prose instructions (interpreted by the LLM)
- Inline script calls for every agent reference would add unacceptable
  preprocessor latency
- Users may want domain-specific review lenses (e.g., accessibility, regulatory
  compliance) beyond the built-in 13
- Explicit registration of custom lenses (in a config file) is less ergonomic
  than auto-discovery

## Decision Drivers

- Ergonomic extension: minimal friction to customise
- Reliability: critical parameters must be deterministic
- Performance: preprocessor latency must be acceptable
- Alignment with ecosystem patterns (file replacement, not section merging)
- Custom lenses should integrate seamlessly with built-in lenses

## Considered Options

For templates:
1. **Section-level merging** — Override individual sections. No ecosystem
   precedent; complex to implement.
2. **File-level replacement** — Three-tier resolution: explicit config path →
   `.claude/accelerator/templates/` directory → plugin default. User duplicates
   full template to change one section.

For agent names:
1. **All inline scripts** — Deterministic everywhere. Too slow (6+ script calls
   per skill load).
2. **All labeled variables** — Single script call outputs named definitions. LLM
   interprets. Semi-deterministic.
3. **Dual strategy** — Inline scripts for critical `subagent_type` parameters;
   labeled variable definitions for prose references. Balances reliability and
   latency.

For custom lenses:
1. **Config file registration** — List custom lenses in config. Explicit but
   requires manual registration.
2. **Auto-discovery** — Scan `.claude/accelerator/lenses/*/SKILL.md` with
   frontmatter validation (`name` required, `auto_detect` optional), collision
   checking against built-in lenses, and integration into a unified lens
   catalogue with absolute paths.

## Decision

We will use file-level template replacement with three-tier resolution. Agent
names use a dual strategy: inline `config-read-agent-name.sh` for
`subagent_type` parameters (deterministic) and labeled variable definitions
from `config-read-agents.sh` for prose references (semi-deterministic). Custom
lenses are auto-discovered from `.claude/accelerator/lenses/*/SKILL.md` with
frontmatter validation and collision checking.

## Consequences

### Positive
- Templates are fully customisable with familiar file-replacement pattern
- Critical agent parameters are deterministic
- Custom lenses require only dropping a SKILL.md file — no registration needed
- Collision checking prevents silent conflicts with built-in lenses

### Negative
- Changing one template section requires duplicating the entire template
- 6 of 7 agent references rely on LLM interpretation of variable definitions
- Auto-discovery requires structural validation to prevent silent failures
- Custom lenses must follow the same structural invariants as built-in lenses

### Neutral
- The dual agent name strategy reveals a fundamental tension in Claude Code
  plugin development between determinism and performance
- Template three-tier resolution means the most specific config wins

## Source References

- `meta/plans/2026-03-23-template-and-path-customisation.md` — Template
  replacement design and three-tier resolution
- `meta/plans/2026-03-27-remaining-configuration-features.md` — Dual agent
  name resolution strategy
- `meta/plans/2026-03-23-context-and-agent-customisation.md` — Agent
  customisation design
- `meta/plans/2026-03-23-review-system-customisation.md` — Custom lens
  auto-discovery design
