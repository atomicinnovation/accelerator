---
adr_id: ADR-0021
date: "2026-04-18T15:33:05+00:00"
author: Toby Clemson
status: accepted
tags: [configuration, templates, configure-skill]
---

# ADR-0021: Template management subcommands

**Date**: 2026-04-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0017 introduced a three-tier template resolution hierarchy (explicit
`templates.<key>` config path → userspace templates directory → plugin
default) but left no tooling above the mechanism. Users who want to inspect,
customise, compare, or revert templates must manually discover template
locations, copy files, diff them, and delete overrides. The `configure` skill
already dispatches subcommands via H3 headings (`view`, `create`, `help`),
providing a natural extension point. The existing `config-read-template.sh`
wraps output in code fences for LLM preprocessor consumption, which makes it
unsuitable for human-facing display. Destructive operations — overwriting an
existing override on `eject`, or deleting one on `reset` — need user
confirmation; natural-language confirmation flows are better expressed in the
skill layer than in shell scripts. Tier-1 overrides (via `templates.<key>`)
may point to user-authored files, including paths outside the repository, so
silent deletion on `reset` would be surprising and potentially destructive.

## Decision Drivers

- Users need a complete lifecycle on a single coherent surface: discover →
  inspect → customise → compare → revert
- Deterministic resolution for non-destructive operations; natural-language
  confirmation for destructive ones — each mechanism plays to its strength
- Resolution logic must not be duplicated across scripts
- Destructive actions (eject over existing override, reset) must be safe by
  default
- Template management should extend the existing `configure` skill rather than
  fragment the configuration surface across multiple skills

## Considered Options

**Management vocabulary:**
1. **Minimum viable set (`list`/`show`/`eject` only)** — omits compare and
   revert flows
2. **Full five-action set (`list`/`show`/`eject`/`diff`/`reset`)** —
   complete lifecycle
3. **Add `edit` action** — rejected; eject + manual edit suffices

**Invocation syntax:**
1. **Nested subcommands (`configure templates <action>`)** — clean namespace;
   extends existing two-level dispatch
2. **Flat subcommands (`configure template-list`, `configure template-show`)**
   — clutters top-level; less extensible
3. **Separate `/accelerator:templates` skill** — fragments the configuration
   surface; requires new manifest entry

**Implementation architecture:**
1. **Pure prompt instructions** — LLM re-implements three-tier resolution;
   fragile
2. **Fully script-backed** — natural confirmation flows hard to express in
   shell
3. **Hybrid: scripts for resolution, skill layer for destructive
   confirmation** — deterministic where possible, LLM-mediated where
   appropriate

**Destructive-action signalling (hybrid architecture):**
1. **Skill layer always confirms, no script signal** — skill calls scripts
   blindly and asks for confirmation on every destructive invocation; over-
   prompts for cases where confirmation is unnecessary
2. **Exit code 2 = "confirmation required"** — script determines whether
   confirmation is needed and signals via exit code; skill layer interprets
   and drives a two-phase flow with `--confirm`/`--force`
3. **`--dry-run` flag** — skill passes `--dry-run` first to preview the
   action, then re-invokes to execute; doubles the script invocation cost
   and complicates script logic

**Raw template output:**
1. **Strip fences from `config-read-template.sh` output in the skill** —
   awkward inversion
2. **`--raw` flag on `config-read-template.sh`** — single script serves two
   audiences
3. **Separate `config-show-template.sh` with shared resolution helper** —
   single-purpose scripts; duplication mitigated by extracting
   `config_resolve_template()` into `config-common.sh`

## Decision

We will add template management to the `configure` skill as a nested
subcommand group:

- **Vocabulary**: five actions — `list` (enumerate all template keys and
  their resolved source), `show` (display a template's content with source
  metadata), `eject` (copy the plugin default into the userspace templates
  directory for customisation), `diff` (compare user override against plugin
  default), `reset` (revert a user override to the plugin default). No `edit`
  action; no versioning.
- **Invocation**: `/accelerator:configure templates <action> [key]`,
  following the existing H3-heading dispatch pattern in `configure`
- **Architecture**: hybrid — five scripts (`config-list-template.sh`,
  `config-show-template.sh`, `config-eject-template.sh`,
  `config-diff-template.sh`, `config-reset-template.sh`) own deterministic
  resolution; the skill layer owns LLM-mediated confirmation for destructive
  actions
- **Exit-code contract**: `0 = success`, `1 = error`, `2 = destructive action
  requires confirmation`. The skill layer interprets codes and drives a
  two-phase flow: report, then act with `--confirm`/`--force`
- **Raw output**: `config-show-template.sh` outputs raw content plus a
  `Source: <label> (<path>)` header. `config-read-template.sh` retains fenced
  output for LLM preprocessor consumption. Shared resolution logic lives in
  `config_resolve_template()` in `config-common.sh`
- **Key enumeration**: scripts iterate over `<plugin_root>/templates/*` — the
  templates directory is the canonical key registry
- **Reset semantics**: for Tier-2 overrides (userspace templates directory),
  the script deletes the file after confirmation; for Tier-1 overrides
  (`templates.<key>` config path), the script deletes the referenced file and
  the skill layer removes the config entry, following team-vs-local precedence
  rules; out-of-project paths trigger explicit re-confirmation

## Consequences

### Positive

- Users have a complete extract-modify-compare-restore lifecycle on a single
  coherent configuration surface
- Deterministic resolution for non-destructive operations; LLM-mediated
  confirmation for destructive ones — each mechanism plays to its strength
- `config_resolve_template()` in `config-common.sh` eliminates resolution
  logic duplication across scripts
- The `templates/` directory becomes the self-updating canonical key registry
  — adding a template requires no script changes
- The exit-code contract (0/1/2) establishes a reusable mechanism↔policy
  pattern applicable to future destructive management commands

### Negative

- `configure`'s argument grammar now has two levels (`templates <action>`);
  the argument hint grows longer
- To change one section of a template, users must still duplicate the whole
  file (inherited constraint from ADR-0017)
- No first-class template-drift detection — when the plugin default evolves,
  ejected copies silently fall behind
- Reset can mutate team config (`accelerator.md`); careful team-vs-local
  precedence rules required — a mistake could silently affect other team
  members
- The reset flow spans both script and skill layers; drift between them could
  reintroduce bugs

### Neutral

- Separate `config-show-template.sh` vs. a `--raw` flag on
  `config-read-template.sh` is a tradeoff between single-purpose scripts and
  minimal surface; either is defensible
- Safety checks (out-of-project warning, team-vs-local precedence) depend on
  LLM compliance with skill-layer instructions — the same class of dependency
  as the rest of the skill system

## References

- `meta/research/2026-03-29-template-management-subcommands.md` — subcommand
  vocabulary analysis, nesting vs. flat vs. separate-skill tradeoff, hybrid
  architecture exploration, raw vs. fenced output options, enumeration strategy
- `meta/plans/2026-03-29-template-management-subcommands.md` — full
  implementation spec: five-action set, `configure` nesting, exit-code
  contract, Tier-1 reset rules, out-of-project safety, key enumeration via
  `templates/` scan
- `meta/decisions/ADR-0017-configuration-extension-points.md` — three-tier
  template resolution this ticket builds management tooling on top of
