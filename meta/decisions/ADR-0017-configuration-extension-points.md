---
adr_id: ADR-0017
date: "2026-04-18T09:00:21+01:00"
author: Toby Clemson
status: accepted
tags: [configuration, templates, agents, lenses, plugin]
---

# ADR-0017: Configuration Extension Points for Templates, Agents, and Custom Lenses

**Date**: 2026-04-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0016 established the foundation: two-tier config files, shell
preprocessor injection, two-level YAML nesting, global scope. That answers
*how* values flow into skills, but not *what* surfaces users can extend or
how each surface is resolved.

Three extension surfaces emerged from real user need:

1. **Document templates** — the plugin ships defaults for ADRs, plans,
   research, validation reports, and PR descriptions. Teams want these bent
   to local conventions (extra sections, different frontmatter).

2. **Agent assignments** — skills reference agents in two structurally
   distinct ways: as `subagent_type` parameters in tool calls (must be exact
   strings, or the spawn fails) and in prose instructions (interpreted by
   Claude). The preprocessor can deterministically inject either, but at
   different latency costs.

3. **Custom review lenses** — the plugin ships 13 built-in lenses. Users
   want domain-specific lenses (accessibility, regulatory compliance)
   integrated alongside built-ins.

Each surface has its own constraint set:

- **Templates**: no plugin in the Claude Code ecosystem attempts structural
  section-merging of markdown. File-level replacement is the de-facto
  pattern.
- **Agents**: inline `` !`config-read-agent-name.sh` `` calls are
  deterministic but each one runs a subprocess at skill load. A skill with
  6+ agent references becomes noticeably slow if every reference takes that
  path.
- **Lenses**: lens skills have a regular structure (`SKILL.md` with
  frontmatter). Auto-discovery is more ergonomic than registration, but is
  only safe if validation and collision-checking against built-ins prevent
  silent failure.

## Decision Drivers

- **Ergonomic extension** — the friction to customise should be minimal;
  ideally dropping a file in a known directory suffices.
- **Reliability for critical parameters** — `subagent_type` values are
  exact-string contracts with the platform; misinterpretation causes spawn
  failures, not soft degradation.
- **Acceptable preprocessor latency** — the `` !`command` `` mechanism
  runs subprocesses synchronously at skill load. Each call adds latency
  users feel.
- **Alignment with ecosystem patterns** — file-level replacement is the
  de-facto pattern for template customisation; deviating from it raises
  learning cost without obvious benefit.
- **Seamless integration of custom and built-in components** — custom
  lenses should not feel second-class; they should appear in the same
  catalogue and follow the same structural invariants as built-ins.
- **Fail-loud on misconfiguration** — silent failures (a custom lens
  silently ignored because of a typo, or a template path that resolves to
  nothing) erode trust faster than verbose errors.

## Considered Options

This ADR bundles three coupled extension-point choices. Options are grouped
by surface; the Decision section picks one from each.

### Templates

1. **Section-level merging** — Override individual sections of a template
   while inheriting the rest. No precedent in the Claude Code ecosystem;
   would require a custom merge engine and a way to address sections stably
   across plugin upgrades.
2. **File-level replacement, three-tier resolution** — At skill load,
   `config-read-template.sh` resolves the template path in order: explicit
   override in `accelerator.md` (`templates.<name>: <path>`),
   `meta/templates/<name>.md` in the project, then the plugin's bundled
   default. The first match wins. To change one section, the user
   duplicates the entire template.

### Agent name resolution

1. **All inline scripts** — Every agent reference (both `subagent_type`
   parameters and prose mentions) is wrapped in
   `` !`config-read-agent-name.sh <role>` ``. Fully deterministic. A typical
   skill has 6+ agent references; running that many subprocesses at every
   skill load is unacceptably slow.
2. **All labeled variables** — A single `config-read-agents.sh` call emits
   a labeled override table once per skill load, and the LLM consults it
   whenever it spawns an agent. One subprocess, but every reference
   (including `subagent_type`) goes through Claude's interpretation.
3. **Dual strategy** — Inline `config-read-agent-name.sh` *only* for
   `subagent_type` parameters (where exact strings are non-negotiable). One
   `config-read-agents.sh` table per skill for prose references. Reserves
   the deterministic path for the cases that need it; pays for it only
   where needed.

### Custom lens registration

1. **Explicit registration** — Users list custom lenses in `accelerator.md`
   (e.g., `review.custom_lenses: [.claude/accelerator/lenses/accessibility-lens]`).
   Discoverable from config alone; no filesystem scanning. Requires a
   separate registration step beyond dropping the lens in.
2. **Auto-discovery** — `config-read-review.sh` scans
   `.claude/accelerator/lenses/*/SKILL.md`, validates frontmatter (`name`
   required, `auto_detect` optional), checks for collisions with the 13
   built-in names, and merges discovered lenses into a unified catalogue
   with absolute paths. Dropping a `SKILL.md` directory in suffices.

## Decision

We adopt one option per surface:

**Templates**: File-level replacement with three-tier resolution.
`config-read-template.sh` resolves each template at skill load time by
checking, in order: (1) an explicit path in `accelerator.md` under
`templates.<name>`, (2) `meta/templates/<name>.md` in the project, (3) the
plugin's bundled default in `<plugin_root>/templates/`. The first match
wins. Users accept full-template duplication as the cost of section-level
changes.

**Agent names**: Dual strategy. `subagent_type` tool-call parameters use
inline `` !`config-read-agent-name.sh <role>` `` — deterministic, because a
wrong string is a spawn failure. Prose references to agents (in skill
instructions) are served by a single `config-read-agents.sh` call per skill
that emits a labeled override table ("Agent Names" block); Claude consults
this table when generating prose that references agents. The deterministic
path is reserved for the cases that cannot tolerate LLM interpretation.

**Custom lenses**: Auto-discovery from `.claude/accelerator/lenses/*/SKILL.md`.
`config-read-review.sh` scans the directory, validates each `SKILL.md`
frontmatter (`name` required, `auto_detect` optional), rejects any custom
lens whose `name` collides with a built-in, and merges discovered lenses
into a unified catalogue with absolute paths so downstream skills cannot
tell built-in from custom. No explicit registration step; dropping the lens
directory in suffices.

## Consequences

### Positive

- **Ergonomic extension across all three surfaces.** Templates and lenses
  are customised by dropping a file in a conventional location — no
  explicit registration. Agent overrides are a single key in
  `accelerator.md`.
- **Deterministic `subagent_type` values.** The one place where a wrong
  string is a hard spawn failure has the one fully-deterministic path.
  Overrides reach the tool call exactly as configured.
- **Single-subprocess agent override table.** Prose references are served
  by one preprocessor call per skill rather than one per reference, keeping
  skill-load latency bounded.
- **Custom lenses are first-class.** Discovered lenses share a catalogue
  with built-ins, follow the same structural invariants, and are
  indistinguishable to downstream selection logic — users do not hit a
  "custom vs built-in" cliff.
- **Loud failure on lens misconfiguration.** Missing `name`, invalid
  frontmatter, or a collision with a built-in name produces a `>&2` warning
  and the lens is skipped — users see the failure rather than silently
  losing a lens.
- **Consistency with the ecosystem.** File-level template replacement
  matches the pattern users already know from other Claude Code plugins and
  from the broader Unix tradition of layered config resolution.

### Negative

- **Changing one template section requires duplicating the whole template.**
  There is no partial-override path; a team that wants to add a single
  heading carries the full file forward across plugin upgrades and must
  manually re-merge upstream changes.
- **Most agent references rely on LLM interpretation.** The `subagent_type`
  path is deterministic, but the 6+ prose references per skill depend on
  Claude correctly consulting the override table. A subtle enough reference
  ("the pattern finder") may still be interpreted incorrectly.
- **Auto-discovery requires structural validation.** Every discovered
  `SKILL.md` must be parsed for frontmatter, have its `name` extracted, and
  be collision-checked. The validation code is non-trivial and is a new
  failure surface — a bug in the validator can silently exclude valid
  lenses.
- **Custom lenses inherit all built-in structural invariants.** Users
  writing a lens must conform to the same `SKILL.md` shape as built-ins
  (frontmatter fields, body structure expected by the reviewer agent). A
  future `/accelerator:create-lens` skill could scaffold the structure and
  close this gap.

### Neutral

- **The dual agent-name strategy is a direct reflection of a platform
  tension.** Claude Code's plugin model offers either deterministic shell
  injection (at subprocess cost) or LLM interpretation (fast but soft). The
  dual strategy is the least-bad reconciliation, not a clean resolution; if
  the platform later offers a third option, this choice should be
  revisited.
- **Three-tier template resolution means the most specific config wins.**
  Config > userspace directory > plugin default. The ordering is
  conventional but not the only defensible choice (e.g., one could argue
  the plugin default should always win for consistency).

## References

- `meta/decisions/ADR-0016-userspace-configuration-model.md` — The
  foundational config model this ADR extends
- `meta/plans/2026-03-23-template-and-path-customisation.md` — Template
  replacement and three-tier resolution design
- `meta/plans/2026-03-23-context-and-agent-customisation.md` — Agent
  customisation design
- `meta/plans/2026-03-23-review-system-customisation.md` — Review system
  and lens override design
- `meta/plans/2026-03-27-remaining-configuration-features.md` — Dual
  agent-name strategy and custom-lens auto-discovery
- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md` —
  Ecosystem survey informing file-level replacement
- `meta/research/codebase/2026-04-07-bare-agent-name-references.md` — Evidence for
  the `subagent_type` determinism constraint
