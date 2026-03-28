---
adr_id: ADR-0020
date: "2026-04-18T14:24:07+01:00"
author: Toby Clemson
status: accepted
tags: [configuration, plugin, skills, bash, preprocessor, customisation]
---

# ADR-0020: Per-Skill Customisation Directory

**Date**: 2026-04-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0016 chose a global-only YAML configuration scope as an explicit
simplicity decision and explicitly noted that per-skill context and
instructions would be handled by a separate directory-based mechanism.
This ADR defines that mechanism.

Skills are prompt text loaded via the `!` shell preprocessor. The plugin
already uses this preprocessor to inject global context
(`config-read-context.sh`) and has an established convention-based
precedent in the custom lenses directory (`.claude/accelerator/lenses/*/SKILL.md`
— ADR-0017), which users already understand.

Users want distinct injection sites for "what the skill should know"
(background context) versus "how the skill should behave" (instructions).
A single catch-all file conflates these concerns. The preprocessor runs
at skill load time, so both injection points must be resolved at that
moment.

Skills are added or renamed as the plugin evolves. Any validation of
user-supplied directory names must adapt automatically to avoid brittle
hard-coded lists.

## Decision Drivers

- **Convention-based discovery** — mirrors the existing directory
  convention from ADR-0017; no explicit registration required
- **Distinct injection sites** for context vs. instructions — different
  concerns, different positions in the prompt
- **Zero-config opt-in** — dropping a file in the right directory is
  sufficient
- **Automatic adaptation** when skills are added or renamed — no
  hand-maintained lists
- **User-friendly diagnostics** for typos without breaking sessions
- **No per-skill parameter overrides** — avoid premature complexity in
  the override-resolution model

## Considered Options

**Customisation surface:**
1. **Per-skill YAML parameter overrides** — rejected by ADR-0016 as
   premature complexity in the override-resolution model
2. **Full skill replacement (override SKILL.md)** — too invasive; breaks
   on plugin upgrades
3. **Convention-based directory with fixed filenames** — chosen; mirrors
   ADR-0017 custom-lenses pattern

**Filename contract:**
1. **Single combined file** — conflates context with behavioural
   directives
2. **Two fixed filenames (`context.md`, `instructions.md`)** — clear
   mental model; distinct injection points
3. **Arbitrary filenames with an index** — flexible but no longer
   convention-based

**Injection order:**
1. **Both prepended near the top** — instructions lose precedence over
   built-in skill instructions
2. **Context near top (after global context), instructions at the very
   end** — instructions appended last effectively override earlier
   directives

**Reader-script structure:**
1. **Single multi-purpose script** — awkward; one injection site must
   accept both types
2. **Two dedicated scripts (`config-read-skill-context.sh`,
   `config-read-skill-instructions.sh`)** — matches the two injection
   sites 1:1; consistent with existing `config-*.sh` pattern

**Directory-name validation:**
1. **Hard-fail on unknown names** — brittle when skills are added or
   renamed
2. **Silent ignore** — hides typos
3. **Advisory stderr warning listing valid names; session continues** —
   user-friendly without breaking sessions

**`KNOWN_SKILLS` derivation:**
1. **Hand-maintained allow-list** — drifts from the skill catalogue
2. **Dynamic scan of `SKILL.md` frontmatter `name:` fields** — single
   source of truth; zero maintenance

## Decision

We will introduce per-skill userspace customisation at
`.claude/accelerator/skills/<skill-name>/` with:

- **Two fixed filenames**: `context.md` injected immediately after the
  global context line, and `instructions.md` appended at the very end of
  the skill prompt. Both are optional; either may be absent.
- **Two dedicated reader scripts** —
  `config-read-skill-context.sh <skill-name>` and
  `config-read-skill-instructions.sh <skill-name>` — invoked via the `!`
  preprocessor from each skill's `SKILL.md`. Empty or whitespace-only
  files produce no output, matching `config-read-context.sh` behaviour.
- **`KNOWN_SKILLS`** derived dynamically by scanning `name:` frontmatter
  across plugin skill directories, excluding `configure`. Directory names
  not in `KNOWN_SKILLS` produce an advisory stderr warning listing the
  valid names; the session continues normally.
- **`configure` explicitly excluded** — no preprocessor lines in its
  `SKILL.md`, excluded from `KNOWN_SKILLS`.
- **Detected customisations surfaced** in the SessionStart hook's
  `additionalContext` and in `/accelerator:configure view`, enumerated as
  `<skill> (<context | instructions | context + instructions>)`.
- **Files contain raw markdown only** — no YAML frontmatter, no template
  substitution; reader scripts own the section-header wrapper.

## Consequences

### Positive

- **Convention-based discovery** — zero explicit registration; dropping
  a file suffices
- **Clear mental model** — "context = knowledge", "instructions =
  behaviour"
- **Per-skill instructions take precedence** — appended last, they
  effectively override earlier directives when conflicts arise
- **`KNOWN_SKILLS` auto-adapts** when skills are added or renamed
- **Advisory warnings give actionable feedback** without breaking sessions
- **`configure` remains authoritative and unaffected** by user-injected
  instructions
- **Mirrors the custom-lenses convention** (ADR-0017), reducing learning
  cost

### Negative

- **Rigid two-file contract** — no support for multiple context or
  instruction fragments per skill
- **Directory names are stringly-typed** and must match skill names
  exactly
- **Every non-`configure` skill's `SKILL.md` grows two preprocessor
  lines**
- **Unknown-name directories produce no prompt-level effect** — only a
  stderr warning that some users may miss

### Neutral

- **Reader scripts add ~5–10ms each** per skill invocation
- **The `configure` exclusion creates asymmetry** — 13 of 14
  user-facing skills support the mechanism
- **`context.local.md` / `instructions.local.md`** personal override
  files are not introduced in this ADR; the convention-based directory
  naturally accommodates them in a future change

## References

- `meta/decisions/ADR-0016-userspace-configuration-model.md` — prior
  decision that deferred this mechanism
- `meta/decisions/ADR-0017-configuration-extension-points.md` — custom
  lenses convention this ADR mirrors
- `meta/plans/2026-03-28-per-skill-userspace-customisation.md` — full
  implementation plan
- `meta/research/2026-03-22-skill-customisation-and-override-patterns.md` —
  foundational research
- `meta/research/2026-03-27-skill-customisation-implementation-status.md` —
  gap analysis identifying the deferred per-skill mechanism
