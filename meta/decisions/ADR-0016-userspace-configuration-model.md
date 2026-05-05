---
adr_id: ADR-0016
date: "2026-04-17T17:18:53+01:00"
author: Toby Clemson
status: accepted
tags: [configuration, plugin, skills, bash, preprocessor]
---

# ADR-0016: Userspace Configuration Model

**Date**: 2026-04-17
**Status**: Active — file paths superseded in part by
`meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md`
(`.accelerator/config*.md` replaces `.claude/accelerator*.md`). A
full superseding ADR is forthcoming.
**Author**: Toby Clemson

## Context

Claude Code provides no built-in mechanism for plugin configuration — no
settings schema in `plugin.json`, no plugin-specific sections in user/project
`settings.json`, and no way for a project to pass structured config to a
plugin. Anthropic's `plugin-dev` toolkit documents a convention-based pattern
using `.claude/<plugin-name>.local.md` with YAML frontmatter, but each plugin
must implement its own reading logic.

Skills are prompt text, not executable code. Only three mechanisms exist for
injecting dynamic behaviour at skill invocation: the `` !`command` `` shell
preprocessor (runs at load time, injects stdout inline), natural-language
instructions to Claude (non-deterministic), and SessionStart hooks (coarse —
session-level, not per-skill).

The plugin needs to support both team-shared conventions (commit templates,
review lens sets, file path conventions) and personal developer overrides
(preferred agents, personal paths). Team settings belong in version control;
personal settings must not be committed.

Shell-based YAML parsing is inherently fragile for complex structures. macOS
ships with bash 3.2, which lacks associative arrays and several newer
features. No external YAML parser (`yq`, `python -c yaml`) can be required as
a dependency.

The plugin already had a working pattern in
`skills/decisions/scripts/adr-read-status.sh` using line-by-line
`while IFS= read -r` parsing of YAML frontmatter — a precedent for
shell-based YAML extraction.

## Decision Drivers

- Deterministic configuration injection — settings must reach skills
  reliably, not via Claude's interpretation of natural-language instructions.
- Clean separation of team-shared vs personal configuration — with natural
  VCS integration (team committed, personal gitignored).
- Comprehensive configurability without platform changes — must work with
  current Claude Code capabilities; no waiting on upstream plugin
  configuration support.
- Minimal external dependencies — no requiring users to install `yq`,
  Python, or other YAML parsers; must run on macOS bash 3.2 out of the box.
- Simple mental model for users — one namespace, discoverable file
  locations, predictable precedence.
- Alignment with Anthropic's documented plugin conventions — follow the
  `plugin-dev` toolkit pattern where it makes sense, to benefit from future
  tooling and user familiarity.

## Considered Options

This ADR bundles four coupled architectural choices. Options are grouped by
axis; the Decision section picks one from each.

### Config file scheme

1. **Single file** — One config file for everything. Simpler, but no
   team/personal separation.
2. **Two-tier** — `.claude/accelerator.md` (team, committed) +
   `.claude/accelerator.local.md` (personal, gitignored via the plugin's
   `init` skill). Last-writer-wins precedence. Follows Anthropic's
   `plugin-dev` convention for `.local.md` personal overrides.
3. **Environment variables** — Shell-native, no parsing needed. But poor
   discoverability, no VCS integration, and no room for free-form project
   context.
4. **Convention directory** — `.claude/accelerator/config.yml` plus
   companion files for templates, lenses, context. More flexible but higher
   learning curve and fragmented file layout.

### Injection mechanism

1. **Natural language instructions** — Tell Claude to read config files and
   act on them. Simplest to implement but non-deterministic; the LLM
   interprets the instruction and may misapply it, especially for exact
   values like agent names.
2. **SessionStart hooks** — Inject config via `additionalContext` at session
   start. Reliable but session-scoped — can't update mid-conversation and
   coarse for per-skill needs. Note: hooks and preprocessor are not mutually
   exclusive — the final design uses SessionStart hooks for the config
   summary and preprocessor for value injection.
3. **Shell preprocessor (`` !`command` ``)** — Runs at skill load time,
   injects script stdout inline. Deterministic, works with current platform,
   but config changes require re-invoking the skill.

### YAML nesting depth

1. **Full YAML** — Maximum expressiveness (lists, block scalars, deep
   nesting). Unreliable in bash without an external parser.
2. **Two-level nesting** — `section.key` with simple scalar values only.
   Reliably parseable with awk on macOS bash 3.2, no external dependencies.
3. **Flat key-value** — No nesting at all. Trivially parseable but forces
   long key names (`review_max_inline_comments`) and prevents sensible
   grouping.

### Scope

1. **Global-only** — All settings apply uniformly across all skills. Simple
   mental model, no override resolution complexity.
2. **Per-skill overrides** — Skills can individually override global
   settings. More flexible but adds precedence rules and a surface-area
   explosion as the config grows.
3. **Two namespaces, no override resolution** — A global structured-settings
   channel for uniform config (agent names, numeric limits, path overrides)
   plus a separate per-skill directory mechanism for free-form context and
   instructions. No precedence rules are needed because the two channels
   target different concerns. This is the combined strategy across
   mechanisms; this ADR decides only the structured-settings portion, with
   the per-skill directory mechanism documented separately.

## Decision

We will use a hybrid architecture combining one option from each axis:

**Config file scheme**: Two-tier — `.claude/accelerator.md` for team-shared,
committed configuration, and `.claude/accelerator.local.md` for personal
overrides gitignored by the plugin's `init` skill. Precedence is
last-writer-wins per key: a value
set in `accelerator.local.md` overrides the same key in `accelerator.md`.
Markdown bodies from both files are concatenated (team context first,
personal context second) for the free-form project context channel.

**Injection mechanism**: The shell preprocessor (`` !`command` ``) is the
primary mechanism for injecting config values into skills at load time. The
SessionStart hook is used only to inject a summary of active configuration
into the session context — it does not carry config values into skills.
Natural-language instructions are not used for config injection.

**YAML nesting depth**: Maximum two levels — `section.key` with simple
scalar values only. No block scalars (`|` / `>`), no deeply nested
structures, no complex types. Parsing uses awk-based extraction following
the pattern established in `skills/decisions/scripts/adr-read-status.sh`.
No external YAML parser is required.

**Scope**: Global only. All configuration keys apply uniformly across all
skills. There is no mechanism for a skill to override a global setting, and
no per-skill config files. This is an explicit simplicity decision to avoid
premature complexity in the override resolution model. This constraint can
be revisited if configuration needs grow beyond what global-only scope can
express cleanly.

## Consequences

### Positive

- **Clean team/personal separation with natural VCS integration.**
  The plugin's `init` skill adds `.claude/accelerator.local.md` to the
  project's `.gitignore`; `.md` is a normal committed file. No manual
  `.gitignore` management required after initialisation.
- **Deterministic config injection.** Values reach skills via shell stdout,
  not Claude's interpretation — agent names, numeric limits, and path
  values arrive exactly as configured.
- **Works on macOS bash 3.2 with no external dependencies.** No `yq`, no
  Python, no npm install — users can adopt configuration without touching
  their toolchain.
- **Simple mental model.** One namespace (`accelerator`), two files (team +
  local), two levels of nesting. The surface is small enough to hold in
  one's head.
- **Aligns with Anthropic's documented plugin convention.** Benefits from
  future tooling (`plugin-dev` helpers) and user familiarity with the
  `.local.md` pattern.
- **Incremental adoption path.** Skills can opt into reading config keys
  one at a time; unconfigured keys fall back to hardcoded defaults, so
  partial rollout does not break existing behaviour.
- **Configuration behaviour is unit-testable.** Awk-based parsing is
  deterministic and covered by `scripts/test-config.sh`, unlike an
  LLM-interpretation approach where correctness depends on Claude's
  behaviour on the day.

### Negative

- **Config changes require re-invoking the skill.** The preprocessor runs
  at skill load time, not mid-conversation — editing config during an
  active skill invocation has no effect until the skill is invoked again.
- **No per-skill scoping of structured YAML settings.** Numeric limits,
  agent names, and path overrides in the frontmatter apply globally across
  all skills. Per-skill context and instructions (the thing users actually
  want per-skill) are handled through a separate directory-based mechanism
  and are not in scope for the YAML config system. This ADR's "global-only"
  scope refers only to the structured-settings channel.
- **No multi-line values or deep nesting.** Block scalars (`|` / `>`) and
  3+ level structures are rejected by the parser. This precludes some
  ergonomic patterns.
- **No sentinel for unsetting a team value from personal config.** If the
  team config sets `review.max_lenses: 8` and a developer wants "use the
  built-in default" rather than a different number, there is no way to
  express that — last-writer-wins means they must pick a concrete value.
- **Shell-based YAML parsing is inherently fragile.** Edge cases (values
  containing colons or hashes, unclosed frontmatter, Windows line endings)
  have been handled individually, but the parser is not a conformant YAML
  implementation and may surprise users who expect full YAML semantics.

### Neutral

- **The two-level nesting cap is a soft constraint.** If future
  configuration needs genuinely require deeper structures, the parser can
  be upgraded — possibly to `yq` as an optional dependency with a bash
  fallback. This ADR is not a permanent lock-in.
- **Global-only scope is an explicit simplicity choice.** It may need
  relaxing as the config catalogue grows; the decision to revisit is
  deferred until a concrete use case forces it.

## References

- `meta/research/2026-03-22-skill-customisation-and-override-patterns.md` —
  Customisation strategy analysis and preprocessor selection
- `meta/plans/2026-03-23-config-infrastructure.md` — Two-tier config files,
  YAML nesting constraints, and global scope decision
