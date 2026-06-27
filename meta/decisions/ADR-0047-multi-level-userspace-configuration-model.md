---
id: "ADR-0047"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: proposed
tags: [architecture, configuration, cli, skills, foundations]
type: adr
title: "ADR-0047: Multi-Level Userspace Configuration Model"
schema_version: 1
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0045", "adr:ADR-0046"]
supersedes: ["adr:ADR-0016", "adr:ADR-0017"]
---

# ADR-0047: Multi-Level Userspace Configuration Model

**Date**: 2026-06-27
**Status**: Proposed
**Author**: Toby Clemson

## Context

Claude Code provides no built-in mechanism for plugin configuration — no
settings schema in `plugin.json`, no plugin section in user/project
`settings.json`, and no structured channel from a project to a plugin.
Anthropic's `plugin-dev` toolkit documents only a convention: a
`.claude/<plugin-name>.local.md` file with YAML frontmatter that each plugin
must read itself. Accelerator therefore defines and implements its own
configuration model.

This repo's configuration model was established across two accepted decisions
that this ADR now supersedes: a two-tier team/personal file model (ADR-0016) and
extension points for templates, agents, and custom lenses (ADR-0017). Two further
decisions — the per-skill customisation directory (ADR-0020) and template-
management subcommands (ADR-0021) — extend that model and remain in force. The
files were later consolidated out of `.claude/accelerator*.md` into a dedicated
`.accelerator/` directory (`config.md` / `config.local.md`); ADR-0016 already
records that its file paths were superseded in part and notes that "a full
superseding ADR is forthcoming." **This is that superseder**: it carries the
decisions of ADR-0016 and ADR-0017 forward into a CLI-native model.

The model needs to support both team-shared conventions (templates, path
conventions, agent assignments) and personal developer overrides (preferred
agents, local paths). Team settings belong in version control; personal settings
must never be committed.

Today we read configuration with a bash, line-by-line/awk frontmatter parser,
constrained by the macOS bash 3.2 floor (ADR-0049). That constraint forces two
design compromises that exist *only* because of the parser, not because the
domain wanted them: a two-level `section.key` nesting cap, and a ban on any
external YAML dependency. ADR-0045 changes the calculus: it establishes a
compiled CLI that owns deterministic procedural logic, and parsing configuration
is exactly that kind of work. A native YAML reader in the CLI removes the bash
constraints entirely, while ADR-0046's zero-setup static-binary distribution
means the reader ships with no runtime dependency to install.

## Decision Drivers

- Clean separation of team-shared vs personal configuration, with natural VCS
  integration (team committed, personal gitignored).
- Deterministic configuration resolution — values must reach skills exactly as
  written, not via the model's interpretation of natural language.
- Reliability and testability — resolution logic should be unit-testable in the
  CLI core, not encoded in fragile shell parsing.
- No runtime dependencies — must work within the zero-setup static binary
  (ADR-0046); no `yq`, Python, or external YAML parser required.
- A simple, discoverable mental model — one namespace, predictable precedence,
  known file locations.
- Alignment with Anthropic's documented `.local.md` plugin convention.
- Carry forward the extension surfaces already decided (template/agent extension
  points, per-skill customisation, template management) without re-architecting.
- Consistency with the skills-vs-CLI division (ADR-0045): config parsing is
  deterministic work the CLI owns, not skill-prose work.

## Considered Options

This ADR bundles four coupled choices, plus the extension surfaces it carries
forward from ADR-0017. Options are grouped by axis; the Decision picks one from
each.

### Config file scheme

1. **Single file** — one config file for everything. Simpler, but no
   team/personal separation.
2. **Two-tier files in `.claude/`** — `.claude/accelerator.md` +
   `.claude/accelerator.local.md` (Anthropic's literal `.local.md` convention;
   the original ADR-0016 layout). Scatters plugin-owned files across the shared
   `.claude/` directory.
3. **Two-tier files in a dedicated `.accelerator/` directory** —
   `.accelerator/config.md` (team, committed) + `.accelerator/config.local.md`
   (personal, gitignored), each with YAML frontmatter and last-writer-wins
   precedence with personal last, housed alongside the other files the plugin
   owns (templates, per-skill customisation, state). Keeps Anthropic's
   `.local.md` spirit while consolidating everything plugin-owned under one root.
4. **Environment variables** — shell-native, no parsing. Poor discoverability,
   no VCS integration, no room for free-form project context.

### Configuration reader

1. **Bash/awk frontmatter parser** — the status quo. Fragile, bound by the bash
   3.2 floor, and precisely the untestable deterministic-in-prose pattern
   ADR-0045 moves away from.
2. **External YAML tool (`yq` / Python)** — robust parsing, but requires a
   runtime dependency, breaking the zero-setup distribution story (ADR-0046).
3. **CLI-native YAML parsing in the hexagonal core** — the compiled CLI parses
   the frontmatter natively and exposes resolution through a command (e.g.
   `accelerator config get`). Deterministic, dependency-free, and unit-testable.

### Frontmatter expressiveness

1. **Two-level `section.key` cap** — ADR-0016's constraint, imposed solely to
   keep bash parsing reliable. Unnecessary once a real parser reads the file.
2. **Arbitrary YAML structure** — lists, nesting, and richer schemas, parsed
   natively by the CLI. No artificial depth limit.

### Scope and extension model

1. **Global-only structured settings** — uniform keys across all skills, nothing
   more. Cannot express the per-skill context, template, and agent extensions the
   model already commits to.
2. **Global structured settings plus the established extension model** — global
   two-tier value resolution, a free-form context channel from the file bodies,
   plus the per-skill customisation directory (ADR-0020), template/agent extension
   points (ADR-0017), and template-management subcommands (ADR-0021), retained as
   the target model.

## Decision

We supersede ADR-0016 and ADR-0017 and adopt a multi-level userspace
configuration model, choosing one option per axis and carrying the superseded
decisions forward:

**Config file scheme** (carries ADR-0016 forward): A dedicated `.accelerator/`
directory is the consolidated root for everything the plugin owns. Within it,
`config.md` holds team-shared, committed configuration and `config.local.md`
holds personal overrides, gitignored by the `init` skill. Precedence is
last-writer-wins per key, personal last: a value in `config.local.md` overrides
the same key in `config.md`. The markdown bodies of both files are concatenated
(team first, personal second) as a free-form project-context channel. Other
plugin-owned files — templates, per-skill customisation directories, and local
state — live under the same `.accelerator/` root.

**Configuration reader** (the change from ADR-0016's bash/awk parser): The
compiled CLI is the native reader. It parses the YAML frontmatter itself,
resolves precedence in the hexagonal core, and exposes the result through a
command (e.g. `accelerator config get` / `set`). This is the deterministic work
the CLI owns under ADR-0045. Skills inject values at load time via the `!`
preprocessor invoking the CLI; a SessionStart hook injects a configuration
summary into session context. Natural-language interpretation is not used to
carry config values. This preserves ADR-0016's injection mechanism (preprocessor
+ SessionStart summary) while replacing its bash/awk reader.

**Frontmatter expressiveness** (relaxes ADR-0016's cap): Arbitrary YAML
structure. Because the CLI parses natively, we drop the two-level `section.key`
cap — frontmatter may use lists, nesting, and richer schemas as the configuration
catalogue grows.

**Scope and extension model** (carries ADR-0017/0020/0021 forward): We retain the
full extension model. The foundational two-tier value resolution is proved
end-to-end by the `configure` skill and CLI. The extension surfaces — template
and agent extension points with their resolution rules (ADR-0017: file-level
template replacement with three-tier resolution; the dual agent-name strategy of
inline `config-read-agent-name.sh` for exact `subagent_type` values plus a single
per-skill agent-table call for prose references; custom-lens auto-discovery with
collision checking), the per-skill `context.md`/`instructions.md` directories
(ADR-0020), and template-management subcommands (ADR-0021) — are part of the model
this ADR commits to. Their detailed internal designs are recorded in those ADRs
(ADR-0020/0021 remain in force; ADR-0017's extension-point decisions are carried
here); this ADR commits to the model's shape and moves resolution into the CLI,
not to re-deciding each extension's internals.

We chose two-tier files with a CLI-native reader because it is the only
combination that keeps configuration deterministic, dependency-free, and
unit-testable while preserving the proven team/personal ergonomics. The bash/awk
reader was rejected as the very fragility ADR-0045 exists to eliminate; an
external YAML tool was rejected for breaking zero-setup distribution; the
two-level nesting cap was rejected as an artefact of a constraint the CLI removes.

## Consequences

### Positive

- Clean team/personal separation with natural VCS integration: `init`-style setup
  gitignores `.accelerator/config.local.md`; the team file is a normal committed
  file.
- All plugin-owned files live under a single discoverable `.accelerator/` root,
  keeping the project's `.claude/` directory uncluttered.
- Deterministic configuration injection — values reach skills via CLI stdout
  exactly as configured, not via model interpretation.
- Resolution logic is unit-testable in the CLI core, free of token cost, model
  variance, and the fragility of shell-based YAML parsing.
- No runtime dependency — the reader ships inside the zero-setup static binary
  (ADR-0046); no `yq` or Python to install.
- Arbitrary frontmatter structure — lists, nesting, and richer schemas — with no
  artificial depth cap.
- Aligns with the skills-vs-CLI division (ADR-0045) and Anthropic's documented
  `.local.md` convention.
- Supersedes ADR-0016 and ADR-0017 with a single coherent model that carries
  their decisions forward, satisfying ADR-0016's note that a full superseding ADR
  was forthcoming, and retaining the extension surfaces (ADR-0017/0020/0021).

### Negative

- Configuration resolution now depends on the CLI being present and
  version-coherent with the plugin — config is coupled to the binary
  build/distribution pipeline.
- Migrating the existing bash/awk config reader to the CLI is substantial work,
  and the model must reach feature parity with the current extension surfaces
  before the shell reader can be retired.
- Config changes take effect only on the next skill invocation: the preprocessor
  runs at skill load time, not mid-conversation (inherited from the injection
  mechanism).
- Last-writer-wins offers no sentinel to *unset* a team value from personal config
  — a developer wanting "use the built-in default" must set a concrete value
  (inherited from ADR-0016's model).

### Neutral

- Injection is via the `!` preprocessor plus a SessionStart summary hook, governed
  by ADR-0045 rather than re-decided here.
- The two tiers are both project-scoped (team committed, personal project-local);
  a machine-global (`$HOME`) configuration level is not introduced and can be
  added later if a concrete need arises.
- Extension-surface internals continue to be governed by ADR-0020 (per-skill
  directories) and ADR-0021 (template-management subcommands), which are not
  superseded by this ADR.

## References

- **Ported from luminosity** — original decision (lum ADR-0003):
  https://github.com/atomicinnovation/luminosity/blob/main/meta/decisions/ADR-0003-multi-level-userspace-configuration-model.md
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — Superseded; its
  two-tier team/personal model and injection mechanism are carried forward here.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — Superseded; its
  template/agent/lens extension points are carried forward here.
- `meta/decisions/ADR-0020-per-skill-customisation-directory.md` — Extends the
  model (per-skill customisation); remains in force.
- `meta/decisions/ADR-0021-template-management-subcommands.md` —
  Template-management subcommands; remains in force.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — The CLI owns
  this deterministic parsing work.
- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md` — Why no
  runtime YAML dependency is acceptable.
- `meta/decisions/ADR-0049-bash-3.2-compatibility-floor.md` — The floor that
  forced the bash parser's design compromises this model removes.
</content>
