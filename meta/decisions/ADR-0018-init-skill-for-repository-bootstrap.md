---
adr_id: ADR-0018
date: "2026-04-18T13:29:15+01:00"
author: Toby Clemson
status: accepted
tags: [configuration, plugin, skills, bootstrap, gitignore]
---

# ADR-0018: Init Skill for Repository Bootstrap

**Date**: 2026-04-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

As the plugin evolves, each new convention can introduce new
repository-level prerequisites: directories that must exist before first
use, entries that must appear in `.gitignore`, structure that must survive
fresh clones. Without a re-runnable mechanism that applies these
prerequisites in one place, responsibility scatters. Some prerequisites
end up enforced only as a side effect of users running a specific command,
so repositories whose owners never ran that command are silently left
without the protection. Others — directories named in documentation but
owned by no skill — are never created at all, so fresh clones lack the
structure their conventions promise.

Certain prerequisites resist simple handling. A directory that must
survive fresh clones while its contents remain uncommitted cannot be
expressed with a single root-level ignore entry, because ignoring the
directory prevents git from tracking the placeholder that keeps the
directory alive.

The plugin already establishes — through the `configure` skill — a
`disable-model-invocation: true` pattern for non-agent skills that
interact directly with the filesystem, hosted under the already-registered
`skills/config/` category. Consumer skills create their
working directories organically on demand, so directory existence cannot
reliably signal whether the plugin has been initialised against the
current plugin version.

## Decision Drivers

- **Repeatable, idempotent prerequisite application** — a single command
  that can be re-run safely after plugin upgrades or interrupted runs to
  bring any repository into alignment with the plugin's current
  requirements, with observable "did this" vs "already present" reporting
- **Unconditional protection of user-local artifacts** — safeguards like
  gitignoring local config must not depend on whether a user happened to
  run a specific adjacent command
- **Non-prerequisite for consumer skills** — skills must remain usable
  without running init, so init is a convenience rather than a gate

## Considered Options

1. **Status quo: on-demand creation in each skill** — each skill
   self-heals its own prerequisites via inline `mkdir -p` or gitignore
   writes. No central bootstrap, so prerequisites not owned by any
   specific skill (directories named in documentation, gitignore
   protections that span skills) either go unenforced or depend on users
   happening to run a specific command.
2. **Centralised init with dependent skills** — a dedicated init skill
   owns all repository-level prerequisites; consumer skills drop their
   inline self-healing and assume init has been run. Fully centralises
   directory-creation and gitignore logic, but makes init a prerequisite
   — skills break if a user invokes them on an uninitialised repository.
3. **Init as convenience, skills remain self-sufficient** — a dedicated
   init skill applies every prerequisite idempotently in one place, but
   consumer skills retain their inline `mkdir -p` so they continue to
   work when init has not been run. Provides a single home for evolving
   prerequisites and unconditional enforcement of user-local protections,
   at the cost of overlap between init and consumer skills for the
   prerequisites both touch.

## Decision

We will introduce `/accelerator:init` as a prompt-only,
`disable-model-invocation: true` skill under `skills/config/init/`,
following the `configure` pattern. The skill is invoked by users (not
auto-triggered) and idempotently applies every repository-level
prerequisite on each invocation.

On every run, init:

- Resolves every configured output directory through `config-read-path.sh`
  (honouring user overrides) and ensures it exists via `mkdir -p`
- Places a `.gitkeep` in every configured directory so that fresh clones
  preserve the expected structure regardless of whether any skill has
  yet produced output
- Seeds `meta/tmp/` with a self-contained inner `.gitignore` (`*`,
  `!.gitkeep`, `!.gitignore`) as the sole ignore mechanism — no
  root-level `meta/tmp/` entry, so the directory itself survives fresh
  clones
- Adds `.claude/accelerator.local.md` to the repository root
  `.gitignore` unconditionally, with whitespace-tolerant dedup against
  existing entries
- Reports "did this" vs "already present" per step and only declares
  success after every step completes

To support configurable `meta/tmp/` location, `tmp` is registered as an
additional path key recognised by `config-read-path.sh`, extending the
configurable path model established in ADR-0016. This is a documentation
change only — `config-read-path.sh` already delegates any key to
`config-read-value.sh`. Consumer skills retain their inline
directory-creation logic so they remain self-sufficient when init has
not been run.

## Consequences

### Positive

- Repository-level prerequisites have a single, re-runnable home that
  users invoke after upgrades to realign with the plugin's current
  requirements
- Local-config protection is applied unconditionally, no longer
  dependent on whether `configure` was run
- Fresh clones preserve the expected directory structure — directories
  documented in conventions exist where they are supposed to
- Idempotency makes repeated invocation the safe recovery path after
  partial or interrupted runs

### Negative

- Init writes to the repository root `.gitignore` — users invoking a
  prerequisite-applier may not expect it to modify tracked files
- Gitignore responsibility is now split between init and `configure`,
  so the same invariant is enforced in two places
- Consumer skills retain their inline directory-creation logic, so it
  is not centralised — evolving prerequisites that touch existing
  directories requires updates in more than one place

### Neutral

- Init is user-invoked rather than automatic; discoverability depends
  on surfacing the skill through other mechanisms (e.g., SessionStart
  hints handled by a companion ticket)
- Interrupted runs do not report success, so re-running is the
  expected recovery path

## References

- `meta/research/codebase/2026-03-28-initialise-skill-requirements.md` —
  Initialisation skill requirements, idempotency rules, taxonomy placement
- `meta/plans/2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md`
  — Implementation plan: skill spec, gitignore dedup rules, `tmp` as
  configurable path, retention of inline `mkdir -p`
- `meta/plans/2026-03-29-rename-initialise-to-init.md` — Rename from
  `initialise` to `init` for parity with Claude Code's `/init`
- `meta/decisions/ADR-0016-userspace-configuration-model.md` —
  Configurable path model that this ADR extends with the `tmp` key
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md`
  — Earlier decision establishing `meta/tmp/` as a shared location; init
  is the mechanism that ensures it exists
