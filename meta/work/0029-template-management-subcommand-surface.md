---
title: "Template management subcommand surface"
type: adr-creation-task
status: done
---

# ADR Ticket: Template management subcommand surface

## Summary

In the context of ADR-0017's three-tier template resolution (explicit config
path → userspace templates directory → plugin default) providing *how*
templates resolve but offering no tooling for users to discover, inspect, or
manage overrides, we decided for a fixed five-action vocabulary —
`list`/`show`/`eject`/`diff`/`reset` — nested under the existing `configure`
skill as `/accelerator:configure templates <action>`, implemented via a
hybrid architecture where shell scripts own deterministic resolution and the
skill layer owns LLM-mediated confirmation for destructive actions, with
structured exit codes (0/1/2) as the script↔skill contract and a new
`config-show-template.sh` providing raw (un-fenced) output alongside the
existing `config-read-template.sh`, to achieve a complete
extract-modify-compare-restore lifecycle on a single coherent configuration
surface, accepting a two-level argument grammar in `configure`, duplication of
entire templates to change one section, and no first-class template-drift or
versioning detection.

## Context and Forces

- ADR-0017 decided file-level template replacement with three-tier resolution
  but left no tooling on top of the mechanism — users must manually discover
  template locations, copy files, diff them, and delete overrides
- `configure` already dispatches subcommands via prose H3 headings
  (`view`, `create`, `help`) — a natural pattern to extend
- `config-read-template.sh` wraps output in code fences for LLM preprocessor
  injection, which is wrong for human-facing display
- Destructive actions (overwriting an existing override, deleting one) need
  user confirmation, but the LLM is better at natural-language confirmation
  flows than shell scripts
- Tier-1 overrides (via `templates.<key>` config path) live outside the
  templates directory and may be user-authored — silently deleting them on
  `reset` would surprise users
- Out-of-project override paths (e.g., absolute paths to shared-team
  templates) are a plausible real scenario — reset should refuse to delete
  them without explicit re-confirmation
- Template versioning/staleness detection is out of scope per the research's
  explicit rejection ("user decided not needed")

## Decision Drivers

- Users need a complete lifecycle: discover → inspect → customise → compare →
  revert
- Deterministic resolution for non-destructive operations
- Safe destructive actions via natural-language confirmation
- Clean separation of mechanism (scripts) and policy (skill layer)
- Reuse of the three-tier resolution logic — no duplication
- Single configuration surface; avoid proliferating top-level skills

## Considered Options

For the management vocabulary:
1. **Minimum viable set (`list`/`show`/`eject` only)** — omits compare and
   revert flows.
2. **Full five-action set (`list`/`show`/`eject`/`diff`/`reset`)** —
   complete lifecycle.
3. **Additional `templates edit`** — rejected: eject + manual edit suffices.
4. **Template versioning / staleness detection** — rejected per research.

For invocation syntax:
1. **Nested subcommands (`configure templates <action>`)** — clean
   namespace; two-level argument dispatch in `configure`.
2. **Flat subcommands (`configure template-list`, `configure template-show`)**
   — clutters top-level; less extensible.
3. **Separate `/accelerator:templates` skill** — requires new manifest entry
   and fragmentation of the config surface.

For implementation architecture:
1. **Pure prompt instructions (no new scripts)** — LLM re-implements
   three-tier resolution; fragile.
2. **Fully script-backed** — natural confirmation flows harder in shell.
3. **Hybrid: scripts for `list`/`show`/`eject`/`diff`, prompt-driven for
   `reset` destructive flow** — deterministic resolution plus natural
   confirmation.

For raw-vs-fenced template output:
1. **Strip code fences from `config-read-template.sh` output in the skill** —
   awkward ("strip fences that were just added").
2. **Add a `--raw` flag to `config-read-template.sh`** — single script
   serves two audiences.
3. **Separate `config-show-template.sh`** — single-purpose scripts; shared
   resolution helper extracted into `config-common.sh`.

For the eject confirmation flow:
1. **Script-side `--force` only** — confirmation hard to express in shell.
2. **Exit-code contract (0 = ejected, 1 = already exists, 2 = error) with
   skill-layer confirmation** — clean handshake; skill orchestrates a
   preview-then-act flow.

For reset when the override is a Tier-1 config path:
1. **Silently delete the referenced file** — dangerous for user-authored
   paths, especially out-of-project.
2. **Edit the config file automatically** — surface area too large.
3. **Skill layer removes the `templates.<key>` entry via Edit tool, with
   explicit team-vs-local precedence rules** — conservative; warns on team
   config and on out-of-project paths.

For template-key enumeration:
1. **Hardcode the key list in the skill prompt** — drifts.
2. **Iterate over `<plugin_root>/templates/`** — self-updating;
   `templates/` becomes the canonical key registry.

## Decision

We will add template management to the `configure` skill as a nested
subcommand group:

- **Vocabulary**: exactly five actions — `list` (enumerate all template keys
  and their resolved source), `show` (display a template with source
  metadata), `eject` (copy the plugin default into the userspace templates
  directory for customisation), `diff` (compare user override against plugin
  default), `reset` (revert a user override to the plugin default). No
  `edit` action, no versioning
- **Invocation**: `/accelerator:configure templates <action> [key]`,
  following the existing prose-H3 dispatch pattern in `configure`
- **Architecture**: hybrid — scripts (`config-list-templates.sh`,
  `config-show-template.sh`, `config-eject-template.sh`,
  `config-diff-template.sh`, `config-reset-template.sh`) own deterministic
  resolution; the skill layer owns LLM-mediated confirmation for
  destructive actions
- **Exit-code contract**: `0 = success`, `1 = error`, `2 = destructive
  action requires confirmation` (target exists / override exists). The
  skill layer interprets codes and runs scripts in two phases (report,
  then act with `--confirm`/`--force`)
- **Raw output**: a new `config-show-template.sh` outputs raw content plus
  a `Source: <label> (<path>)` metadata header. The existing
  `config-read-template.sh` keeps fenced output for LLM preprocessor
  consumption. Shared resolution logic is extracted into
  `config_resolve_template()` in `config-common.sh`
- **Key enumeration**: scripts iterate over `<plugin_root>/templates/*` to
  derive the authoritative key set — `templates/` is the canonical registry
- **Reset semantics**:
  - For Tier-2 overrides (files in the userspace templates directory): the
    script deletes the file after confirmation
  - For Tier-1 overrides (`templates.<key>` config path): the script
    deletes the referenced file and emits a "remove config entry" note;
    the skill layer then removes the `templates.<key>` entry via the Edit
    tool, following precedence rules (local only → local; team only →
    team; both same value → both; both differ → local only and warn)
  - Out-of-project override paths trigger an explicit re-confirmation

## Consequences

### Positive
- Users have a complete extract-modify-compare-restore lifecycle on a
  single coherent configuration surface
- Deterministic resolution for non-destructive operations; LLM-mediated
  confirmation for destructive ones — each mechanism plays to its strength
- Shared `config_resolve_template()` helper eliminates duplication between
  `read` and `show` scripts
- `templates/` directory becomes the self-updating canonical key registry —
  adding a template requires no script changes
- Exit-code contract (0/1/2) establishes a reusable mechanism↔policy pattern
  that can be reapplied to future destructive management commands

### Negative
- `configure`'s argument grammar now has two levels (`templates <action>`);
  `argument-hint` grows longer
- To change a single section of a template, users still duplicate the whole
  file (inherited constraint from ADR-0017)
- No first-class template-drift detection — when the plugin default evolves,
  ejected copies silently fall behind
- Reset can mutate team config (`accelerator.md`) — requires careful
  team-vs-local precedence rules; a mistake could silently affect other
  team members
- The reset flow now spans both the script and the skill layer; drift
  between them could re-introduce bugs
- Two scripts resolve templates (`config-read-template.sh` for fenced,
  `config-show-template.sh` for raw) — mitigated by the shared helper but
  still a maintenance surface

### Neutral
- Separate `config-show-template.sh` vs. a `--raw` flag on the existing
  script is a tradeoff between single-purpose scripts and minimal surface;
  either choice is defensible
- Safety checks (out-of-project warning, team-vs-local precedence) depend
  on LLM compliance with the skill-layer instructions — same class of
  dependency as the rest of the skill system
- The option to extract template management into a standalone
  `/accelerator:templates` skill remains open for the future

## Source References

- `meta/research/2026-03-29-template-management-subcommands.md` — subcommand
  vocabulary analysis, nesting-vs-flat-vs-separate-skill tradeoff, hybrid
  architecture exploration, raw-vs-fenced output options, enumeration
  strategy
- `meta/plans/2026-03-29-template-management-subcommands.md` — full spec:
  five-action set, `configure` nesting, exit-code contract, Tier-1 reset
  rules, out-of-project safety, key enumeration via `templates/` scan
- `meta/decisions/ADR-0017-configuration-extension-points.md` — the
  three-tier resolution this ticket builds management tooling on top of
