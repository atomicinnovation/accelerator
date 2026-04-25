---
adr_id: ADR-0023
date: "2026-04-25T22:30:00+01:00"
author: Toby Clemson
status: accepted
tags: [migration, meta-directory, configuration, upgrade]
---

# ADR-0023: Meta-directory migration framework

**Date**: 2026-04-25
**Status**: Accepted
**Author**: Toby Clemson

## Context

Pre-release breaking changes to the plugin's meta-directory layout or
configuration key names have historically been communicated as manual
instructions in `CHANGELOG.md` (e.g. v1.8.0's `rm -rf` ephemeral-file move;
v1.9.0's `initialise` → `init` rename). As the plugin gains more users with
customised configurations (pinned `paths.*` values, review thresholds, etc.),
manual instructions become error-prone: users with non-default config must
translate the instructions for their setup, and there is no automated check
that the migration was applied correctly.

The `tickets` → `work` rename (ADR-0022) is the first change that:
1. Affects both the storage directory layout and configuration key names
2. Has a non-trivial "pinned path" case: a user who set `paths.tickets:
   meta/custom-tix` must have the config KEY rewritten but the DIRECTORY
   preserved
3. Requires a discoverability mechanism so users know they need to migrate at
   all

A lightweight, shell-based migration framework fits the plugin's existing
conventions (all infrastructure is shell; no Python/Node runtime required) and
has a well-understood execution model.

## Decision Drivers

- Users with pinned configuration values must not have their intent clobbered —
  only plugin-level expectations (key names, defaults) are rewritten
- Migrations must be idempotent so re-runs are safe
- Discoverability: users should learn they need to migrate without having to
  read the CHANGELOG proactively
- The safety model must be compatible with the plugin's "destructive actions are
  guarded, not prevented" philosophy (see `init` skill precedent)
- The framework must not require a runtime beyond POSIX shell

## Considered Options

**Migration mechanism:**
1. **Manual CHANGELOG instructions only** — rejected: error-prone for users
   with pinned config; no automated correctness check
2. **`/accelerator:migrate` skill with ordered shell scripts** — chosen: fits
   shell conventions; scripts are independently testable; ordered by filename
   prefix; idempotent by design
3. **Python/JS migration runner** — rejected: introduces a runtime dependency
   out of step with the plugin's shell conventions
4. **Schema-version field in `meta/.config`** — rejected: larger surface than
   needed; requires a new config file format; migrations are infrequent

**State tracking:**
1. **Newline-delimited applied-IDs file (`meta/.migrations-applied`)** — chosen:
   human-readable; one entry per line; trivially diffable in VCS; unknown IDs
   from downgrades are preserved verbatim
2. **Timestamp-based journal** — rejected: more complex to parse; migration
   idempotency depends on content, not time

**Dry-run model:**
1. **`--dry-run` flag (the original research proposal)** — rejected: requires
   migration scripts to branch on a flag; doubles the code path; increases the
   maintenance surface
2. **Clean-tree pre-flight + preview** — chosen: the driver verifies no
   uncommitted changes in `meta/` or `.claude/accelerator*.md` before mutating;
   it prints a one-line preview per pending migration before applying; this
   provides the same user protection as dry-run without an extra flag, and
   matches the `init` skill's report-then-act idiom
3. **Always-destructive (no guard)** — rejected: too risky for files users may
   have uncommitted changes in

**Rollback support:**
1. **Per-migration undo scripts** — rejected: maintenance multiplier; VCS revert
   is always available and is the right tool for "undo migration"
2. **VCS revert only** — chosen: the clean-tree pre-flight ensures there is a
   clean VCS state to revert to; no additional rollback mechanism needed

**Discoverability:**
1. **CHANGELOG only** — rejected: passive; users upgrade the plugin and do not
   notice the migration requirement until something breaks
2. **SessionStart hook comparing applied vs. available migration IDs** — chosen:
   fires automatically at the start of each Claude Code session; emits a
   one-line warning to stderr when the repo's `meta/.migrations-applied` lags
   the plugin's bundled migrations; self-adjusting as new migrations land

## Decision

Introduce `/accelerator:migrate` with the following design:

**Registry**: ordered shell scripts under
`skills/config/migrate/migrations/[0-9][0-9][0-9][0-9]-*.sh`, discovered by
sorted glob. Each script:
- Begins with `#!/usr/bin/env bash` then `# DESCRIPTION: <short description>`
  on line 2 (consumed by the driver for preview output)
- Receives `PROJECT_ROOT` as an environment variable
- Must be independently idempotent (self-detect no-op conditions and exit 0)
- Must use atomic write patterns (temp-file-then-rename) for file mutations

**State file**: `meta/.migrations-applied` — newline-delimited migration IDs.
Unknown IDs (e.g. from a plugin downgrade) are preserved verbatim and warned
about; they are never deleted.

**Driver (`skills/config/migrate/scripts/run-migrations.sh`)**:
1. Verify clean working tree (uncommitted changes in `meta/` or
   `.claude/accelerator*.md` abort the run). `ACCELERATOR_MIGRATE_FORCE=1`
   bypasses this check for advanced users.
2. Read state file; glob bundled migrations; filter applied; identify pending.
3. Print one-line preview per pending migration.
4. Apply each pending migration in order; on success append its ID to the state
   file (atomic temp-file-then-rename). On failure: abort, no partial state
   write, exit 1.
5. Print end-of-run summary table.

**Pinned-path preservation**: migrations rewrite plugin-level expectations
(key names, defaults), NOT user intent. A user-pinned config value (e.g.
`paths.tickets: meta/custom-tix`) has its KEY rewritten (`paths.work:
meta/custom-tix`) and its directory left where the user put it. Directory
renames apply only when the resolved path matches the plugin default.

**Collision guard**: if both the old default directory and the new default
directory exist, the migration aborts with a clear error naming both paths and
pointing at the resolution procedure. Neither directory is touched.

**Discoverability hook** (`hooks/migrate-discoverability.sh`): compares the
highest applied migration ID against the highest available bundled migration ID
at SessionStart. Emits a one-line warning to stderr when the repo is behind,
pointing at `/accelerator:migrate`. Fires only for repos that are clearly using
Accelerator (`meta/` or `.claude/accelerator.md` exists). Always exits 0.

**First migration**: `0001-rename-tickets-to-work.sh` — renames `meta/tickets/`
→ `meta/work/` (default path only), rewrites `ticket_id:` → `work_item_id:`
in work-item frontmatter, and rewrites five config keys
(`paths.tickets`/`paths.review_tickets`/`review.ticket_revise_severity`/
`review.ticket_revise_major_count`/`review.min_lenses_ticket`) to their new
names.

## Consequences

### Positive

- Future restructures land as small, independently testable migration scripts
  rather than CHANGELOG instructions
- Users with pinned config values are handled correctly automatically, not by
  manual translation
- The `meta/.migrations-applied` state file provides a clear audit trail
  visible in VCS history
- The SessionStart hook eliminates the most common failure mode (user upgrades
  plugin, doesn't notice they need to migrate, encounters mysterious breakage)
- Per-migration idempotency makes re-runs safe at every step, including after
  partial failures

### Negative

- Migration scripts must self-detect no-op conditions (belt) even though the
  state file already filters applied migrations (suspenders); this doubles the
  idempotency surface but is a maintenance requirement for each new migration
- The framework has no rollback support; users who need to undo a migration must
  use VCS revert; this is intentional but requires VCS to be present
- Without a runtime beyond shell, structural YAML parsing is limited to
  constrained line-anchored patterns; complex nested config rewrites may require
  workarounds

### Neutral

- `ACCELERATOR_MIGRATE_FORCE=1` bypass is an escape hatch for advanced users
  (CI environments, no-VCS repos); it is opt-in and undocumented in the main
  help text, reducing the chance of accidental misuse

## References

- `meta/research/2026-04-25-rename-tickets-to-work-items.md` — migration
  framework design exploration, pinned-path semantics, dry-run alternatives
- `meta/decisions/ADR-0022-work-item-terminology.md` — the rename that
  necessitates the first migration
- `skills/config/migrate/SKILL.md` — skill body documenting the operational
  contract for users
- `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh` — first
  migration implementing the tickets→work rename
- `hooks/migrate-discoverability.sh` — SessionStart hook for migration lag
  detection
- `meta/research/2026-03-28-initialise-skill-requirements.md` — `init` skill's
  report-then-act idiom (precedent for preview-before-apply)
