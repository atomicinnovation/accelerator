---
title: "Per-skill userspace customisation directory"
type: adr-creation-task
status: done
---

# ADR Ticket: Per-skill userspace customisation directory

## Summary

In the context of ADR-0016 deliberately deferring per-skill customisation to a
separate directory-based mechanism (keeping the YAML config global-only for
simplicity), we decided for a convention-based directory at
`.claude/accelerator/skills/<skill-name>/` containing two fixed filenames —
`context.md` injected immediately after global context and `instructions.md`
appended at the very end of the skill prompt — backed by two dedicated reader
scripts (`config-read-skill-context.sh`, `config-read-skill-instructions.sh`),
with empty/whitespace-only files producing no output, a dynamically-derived
`KNOWN_SKILLS` list validating directory names with advisory (non-blocking)
stderr warnings, and the `configure` skill explicitly excluded, to achieve
per-skill customisation that mirrors the existing custom-lenses precedent
without introducing per-skill YAML overrides, accepting a rigid two-file
contract, a stringly-typed directory-name convention, and no first-class
team/personal split analogous to `accelerator.md`/`accelerator.local.md`.

## Context and Forces

- ADR-0016 chose global-only YAML config scope as an explicit simplicity
  decision and explicitly noted that per-skill context/instructions would be
  handled by a separate directory-based mechanism — this ticket fulfills that
  deferred mechanism
- The plugin already has a working convention-based precedent in the custom
  lenses directory (`.claude/accelerator/lenses/*/SKILL.md`), which users and
  tooling already understand
- Users want distinct injection sites for "what the skill should know" versus
  "how the skill should behave" — a single catch-all file conflates these
- Per-skill *parameter* overrides (e.g., different `review.max_lenses` per
  skill) are explicitly out of scope per ADR-0016
- The `configure` skill itself manages configuration and should not be
  subject to user-injected instructions that could interfere with its
  operation
- Hard-failing on unknown directory names is brittle as skills are added or
  renamed; silent ignore hides real mistakes

## Decision Drivers

- Consistency with the existing custom-lenses convention
- Distinct injection sites for context vs. instructions
- Zero-config opt-in — dropping a file suffices
- Safe, automatic adaptation when skills are added or renamed
- User-friendly diagnostics for typos without breaking sessions
- Avoid premature complexity from per-skill parameter overrides

## Considered Options

For the customisation surface:
1. **Per-skill YAML parameter overrides** — rejected by ADR-0016 as premature
   complexity in the override-resolution model.
2. **Full skill replacement (override SKILL.md)** — too invasive; breaks
   plugin upgrades.
3. **Convention-based directory with fixed filenames** — chosen; mirrors
   custom lenses.

For filename contract:
1. **Single combined file** — conflates context with behavioural directives.
2. **Two fixed filenames (`context.md`, `instructions.md`)** — clear mental
   model; distinct injection points; no support for multiple fragments per
   skill.
3. **Arbitrary filenames with an index** — more flexible but no longer
   convention-based.

For injection order:
1. **Both prepended near the top** — instructions lose precedence.
2. **Context near top (after global context), instructions at the very end**
   — instructions are appended last so they effectively override.

For reader-script structure:
1. **Single multi-purpose script** — one injection site would have to accept
   both types; awkward.
2. **Two dedicated scripts** — matches the two injection sites 1:1;
   single-responsibility; consistent with existing `config-*.sh` pattern.

For directory-name validation:
1. **Hard-fail on unknown names** — brittle; breaks when skills are added or
   renamed.
2. **Silent ignore** — hides typos.
3. **Advisory stderr warning listing valid names; still report in summary**
   — user-friendly diagnostics without breaking sessions.

For `KNOWN_SKILLS` derivation:
1. **Hand-maintained allow-list** — drifts from the skill catalogue.
2. **Dynamic scan of `skills/*/SKILL.md` and `skills/*/*/SKILL.md`
   frontmatter** — single source of truth; zero maintenance.

For the team/personal model:
1. **Dual-file convention (`context.md`/`context.local.md` per skill)** —
   mirrors `accelerator.md`/`accelerator.local.md` but doubles the surface.
2. **Single file, committed by default, personal opt-out via `.gitignore`**
   — mirrors lens-sharing convention; simpler.

## Decision

We will introduce per-skill userspace customisation at
`.claude/accelerator/skills/<skill-name>/` with:

- Two fixed filenames: `context.md` (injected immediately after the global
  context line) and `instructions.md` (appended at the very end of the skill
  prompt). Both are optional; either may be absent
- Two dedicated reader scripts —
  `config-read-skill-context.sh <skill-name>` and
  `config-read-skill-instructions.sh <skill-name>` — invoked via the `!`
  preprocessor from each skill's SKILL.md. Empty or whitespace-only files
  produce no output (matching `config-read-context.sh` behaviour)
- `KNOWN_SKILLS` derived dynamically from plugin skill directories
  (scanning `name:` frontmatter), excluding `configure`. Directory names
  not in `KNOWN_SKILLS` produce an advisory stderr warning listing the
  valid names, but files are still reported in the session-start summary
- `configure` is explicitly excluded from per-skill customisation — no
  preprocessor lines in its SKILL.md, excluded from `KNOWN_SKILLS`
- Detected customisations are surfaced at both the SessionStart hook's
  `additionalContext` and the `/accelerator:configure view` output,
  enumerated as `<skill> (<context | instructions | context + instructions>)`
- Files contain raw markdown only — no YAML frontmatter, no template
  substitution; reader scripts own the section-header wrapper
- Sharing model: per-skill files are team-shared by default (committed).
  Personal per-skill preferences are opt-out via `.gitignore`

## Consequences

### Positive
- Convention-based discovery; zero explicit registration
- Clear mental model: "context = knowledge", "instructions = behaviour"
- Per-skill instructions appear last and typically take precedence if they
  conflict with earlier instructions
- `KNOWN_SKILLS` auto-adapts when skills are added or renamed
- Advisory warnings give users actionable feedback without breaking sessions
- Configure skill remains authoritative and unaffected
- Mirrors the existing custom-lenses convention, reducing learning cost

### Negative
- Rigid two-file contract — no support for multiple context or instruction
  fragments per skill
- Directory names are stringly-typed and must match skill names exactly
- No first-class personal/team split; users must use `.gitignore` to separate
- Every skill's SKILL.md grows two preprocessor lines
- Unknown-name directories produce no prompt-level effect, only warnings —
  some users may miss the stderr output

### Neutral
- Reader scripts add ~5–10ms each per skill invocation
- The `configure` exclusion is documented but creates asymmetry — 13 of 14
  user-facing skills support the mechanism
- Surfacing customisations in both hook context and `configure view` couples
  the summary script to two consumers

## Source References

- `meta/plans/2026-03-28-per-skill-userspace-customisation.md` — full plan
  covering directory layout, reader scripts, injection positions, dynamic
  `KNOWN_SKILLS` derivation, advisory warning behaviour, configure
  exclusion, sharing model
- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md` —
  foundational research informing the convention-based approach
- `meta/research/codebase/2026-03-27-skill-customisation-implementation-status.md` —
  gap analysis identifying the deferred per-skill mechanism
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — prior
  decision that explicitly deferred this mechanism
