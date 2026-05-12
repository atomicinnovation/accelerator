---
title: "Init skill for repo bootstrap"
type: adr-creation-task
status: done
---

# ADR Ticket: Init skill for repo bootstrap

## Summary

In the context of a plugin with no bootstrap mechanism, where directories are
created on demand by individual skills via inline `mkdir -p` and some documented
directories (notably `meta/tmp/`) are never created at all, we decided for a
dedicated `/accelerator:init` skill — prompt-only with `disable-model-invocation`,
located under `skills/config/` alongside `configure` — that idempotently creates
every configured output directory via `config-read-path.sh`, seeds `meta/tmp/`
with a `.gitkeep` and a self-contained inner `.gitignore`, unconditionally adds
`.claude/accelerator.local.md` to the consumer's root `.gitignore`, and is safe
to re-run via whitespace-tolerant dedup checks, to achieve a smooth, predictable
first-run experience and reliable protection of user-local config, accepting
overlap with `configure`'s gitignore handling and retention of defensive
`mkdir -p` in consumer skills so they still work when init has not been run.

## Context and Forces

- The plugin had no initialisation mechanism; directories were created on demand
  by individual skills, and some (notably `meta/tmp/`) were documented in the
  README but never created by any skill
- `.claude/accelerator.local.md` was only gitignored opportunistically by the
  `configure` skill's `create` action — users who never ran `create` left local
  config unprotected
- `meta/tmp/` needs to survive fresh clones (so ephemeral-file consumers have
  somewhere to write) while its contents must never be committed — root-level
  gitignore entries prevent git descending into the directory at all, breaking
  `.gitkeep` persistence
- The `configure` skill has already established a prompt-only,
  `disable-model-invocation: true` pattern for non-agent skills that interact
  directly with the filesystem
- Consumer skills (e.g., `review-pr`) create `tmp/` organically on demand, so
  directory existence cannot reliably signal whether the plugin has been
  initialised
- Skills should remain usable without running init, so init is a convenience
  rather than a prerequisite

## Decision Drivers

- Smooth, predictable first-run experience with no "confusing missing-directory
  errors"
- Reliable gitignoring of `.claude/accelerator.local.md` regardless of whether
  `configure` is ever run
- Safe repeated execution (idempotent by design)
- Consistency with existing skill-taxonomy conventions (non-agent skills under
  `skills/config/`, `disable-model-invocation: true`, no manifest changes)
- Retain robustness when init has not been run — skills must still self-heal

## Considered Options

For bootstrap mechanism:
1. **Status quo (on-demand creation in each skill)** — fragile; no protection
   for documented-but-missing dirs like `meta/tmp/`; local config gitignore
   unreliable.
2. **Dedicated init skill** — centralises directory creation and gitignore
   management; provides a single entry point for repo setup.

For skill taxonomy:
1. **New top-level category** — requires plugin.json manifest changes.
2. **Under `skills/config/` alongside `configure`** — `./skills/config/` is
   already registered; natural home for infrastructure skills.

For `meta/tmp/` gitignoring strategy:
1. **Root-level `meta/tmp/` in `.gitignore`** — breaks `.gitkeep` persistence
   because git never descends into the directory.
2. **Self-contained inner `.gitignore`** — `*`, `!.gitkeep`, `!.gitignore`
   keeps structure in fresh clones and ignores contents.

For `.claude/accelerator.local.md` gitignoring:
1. **Leave it to `configure create`** — fragile, requires user action.
2. **Add unconditionally in init** — robust even if `configure` was never run;
   harmless if the file doesn't exist yet.

For `paths.tmp` configurability:
1. **Hardcode relative to project root** — simpler but inconsistent with the
   other ten `paths.*` keys.
2. **Register as the 11th configurable path key** — consistent with all other
   output paths; resolvable via `config-read-path.sh` with a `meta/tmp` default.

For init's effect on consumer skills:
1. **Remove inline `mkdir -p` from consumers and rely on init** — skills break
   if init was never run.
2. **Keep inline `mkdir -p` in consumers** — init becomes a convenience
   rather than a prerequisite.

## Decision

We will introduce `/accelerator:init` under `skills/config/init/SKILL.md` —
a prompt-only skill with `disable-model-invocation: true`, following the
`configure` pattern — that, on every invocation:

- Resolves every configured output directory through `config-read-path.sh` and
  creates it with `mkdir -p` (honouring any user overrides)
- Seeds `meta/tmp/` with a `.gitkeep` and a self-contained inner `.gitignore`
  (`*`, `!.gitkeep`, `!.gitignore`) as the sole ignore mechanism — no
  root-level `meta/tmp/` entry
- Adds `.claude/accelerator.local.md` to the consumer's root `.gitignore`
  unconditionally, with whitespace-tolerant dedup against existing entries
- Applies selective `.gitkeep` only to manually-populated directories
  (`tmp/`, `templates/`, `work/`, `notes/`) — skill-output directories get
  artifacts on first use
- Reports "did this" vs "already present" and only emits "Initialisation
  complete" after all steps succeed

`tmp` is registered as the 11th configurable path key (documentation change
only — `config-read-path.sh` already delegates any key to
`config-read-value.sh`). Consumer skills retain their inline `mkdir -p` so
they remain self-sufficient when init has not been run.

## Consequences

### Positive
- Single entry point for repo setup; first skill invocation is smooth
- `meta/tmp/` finally exists and is correctly gitignored in fresh clones
- Local config gitignore is reliable regardless of `configure` usage
- Skill taxonomy remains clean (no plugin.json changes)
- Idempotent by design — safe to re-run after upgrades or partial runs
- Works with or without user overrides; `init` can run before or after
  `configure`

### Negative
- Overlap with `configure`'s gitignore handling creates two places that
  enforce the same invariant
- `init` mutates the consumer's root `.gitignore` — may surprise users
- Selective `.gitkeep` policy introduces asymmetry between manually-populated
  and skill-output directories
- Retaining inline `mkdir -p` in consumers means directory-creation logic is
  not fully centralised

### Neutral
- `init` is invoked by users, not auto-triggered — discoverability of the
  skill is a separate concern (handled by a SessionStart sentinel hint in a
  companion ticket)
- Interrupted runs do not report success, so re-running is the expected
  recovery path

## Source References

- `meta/research/codebase/2026-03-28-initialise-skill-requirements.md` — init skill
  requirements, idempotency rules, `.gitkeep` policy, skill-taxonomy placement
- `meta/plans/2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md`
  — init-skill spec, gitignore dedup rules, `tmp` as configurable path,
  retention of inline `mkdir -p`
- `meta/plans/2026-03-29-rename-initialise-to-init.md` — rename from
  `initialise` to `init` for parity with Claude Code's `/init`
