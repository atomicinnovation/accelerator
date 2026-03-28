---
adr_id: ADR-0019
date: "2026-04-18T13:46:13+00:00"
author: Toby Clemson
status: accepted
tags: [configuration, paths, gitignore, ephemeral, review-pr]
---

# ADR-0019: Ephemeral File Separation via paths.tmp

**Date**: 2026-04-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

Skills in the plugin produce two kinds of files: persistent artifacts that
belong under version control (review reports, plans, research notes, ADRs)
and ephemeral working files (downloaded diffs, changed-file lists, assembled
JSON payloads) that exist only for the duration of a single invocation. The
`review-pr` skill is the first to produce a meaningful volume of both, and
sets the pattern other skills will follow.

Without a dedicated ephemeral location, the natural default is to write
working files alongside the persistent artifact they help produce. This
co-locates data with different lifetimes, making accidental commits easy and
leaving no clean gitignore boundary between them.

The configuration system already exposes `paths.*` keys for all persistent
output directories, each resolved via `config-read-path.sh` with a built-in
default. The wrapper delegates any key generically to `config-read-value.sh`,
so new keys cost nothing structurally. There is, however, no `paths.*` entry
for ephemeral data, and no established convention for where skills should
write it.

Gitignoring an ephemeral directory at the root of `.gitignore` prevents git
from descending into it at all. That breaks any `.gitkeep` seed placed there
to ensure the directory survives fresh clones, and makes any inner ignore
rules unreachable. An inner `.gitignore` that ignores everything except
itself and `.gitkeep` keeps the directory present and the rules local to the
thing they govern, at the cost of being less discoverable than a root-level
entry.

## Decision Drivers

- Clean separation between transient and persistent data, so each can have
  appropriate lifetime and version-control treatment
- A reliable gitignoring mechanism that does not depend on fragile patterns
  matching artifact filenames
- Ephemeral directories must survive fresh clones, so consumers always have
  somewhere to write
- Consistency with the existing `paths.*` configurable-path convention, so
  users can relocate the ephemeral root like any other output directory
- A reusable convention other skills can adopt without each inventing its
  own ephemeral location

## Considered Options

1. **Leave ephemeral files co-located with persistent artifacts** — Status
   quo. Working files sit under the persistent output directory; gitignoring
   relies on filename patterns at the root. Simple, but keeps the
   accidental-commit risk and offers no convention for other skills.

2. **Hardcode a shared `meta/tmp/` directory across skills, root-ignored** —
   Dedicated ephemeral location at a fixed path, with `meta/tmp/` added to
   the root `.gitignore`. Cleanly separates lifetimes, but the directory
   cannot carry a `.gitkeep` (git will not descend into it), so it does not
   survive fresh clones; and the path is not relocatable.

3. **Register `paths.tmp` as a configurable key, protected by an inner
   self-ignoring `.gitignore`** — Add `tmp` as an eleventh `paths.*` key
   (default `meta/tmp`), resolved the same way as every other path. Protect
   its contents with a `.gitignore` file *inside* the directory, using the
   pattern `*`, `!.gitkeep`, `!.gitignore`. The directory persists across
   clones, its contents are ignored wholesale, ignore rules live with the
   thing they govern, and the location is user-overridable.

## Decision

We will adopt option 3: register `paths.tmp` as an eleventh configurable
path key with a default of `meta/tmp`, and protect its contents with a
self-contained inner `.gitignore` using the pattern `*`, `!.gitkeep`,
`!.gitignore`.

Resolution follows the existing convention: skills read the path via
`config-read-path.sh tmp meta/tmp`, which delegates generically through
`config-read-value.sh`. Registering the key therefore requires no new
resolution code; it is a matter of documenting the key alongside the
existing ten and updating `config-dump.sh` to enumerate it.

Skills producing ephemeral files will write them under
`{paths.tmp}/{skill-specific-subdirectory}/`, leaving persistent output
directories for version-controlled artifacts only. The `init` skill will
seed `{paths.tmp}/.gitkeep` and `{paths.tmp}/.gitignore` at repository
bootstrap so the directory exists and ignores its own contents on fresh
clones.

## Consequences

### Positive

- Transient and persistent data are cleanly separated: each can be reasoned
  about, relocated, and gitignored independently.
- Ephemeral contents are ignored wholesale via the inner `.gitignore`, with
  no reliance on filename-pattern matches at the root of the tree.
- The `paths.tmp` key is user-overridable like every other configurable
  path, so users who prefer `.tmp/` or a location outside `meta/` can
  relocate it without skill changes.
- Other skills gain a conventional ephemeral location and do not need to
  invent their own.
- Ignore rules live with the directory they govern, so relocating
  `paths.tmp` carries its own gitignore behaviour automatically.

### Negative

- Ignore logic inside the directory is less immediately discoverable than a
  root-level `.gitignore` entry. New contributors may not realise where the
  rule lives until they encounter it.
- Every consuming skill must be written to respect the convention; a skill
  that forgets to use `paths.tmp` reintroduces the co-location problem
  silently.

### Neutral

- Registering `paths.tmp` is a small change — a new entry in
  `config-dump.sh` and documentation updates — since `config-read-path.sh`
  already resolves any key generically.
- The `init` skill acquires responsibility for seeding both `.gitkeep` and
  the inner `.gitignore` at bootstrap; without `init` having run, skills
  writing to `paths.tmp` will create the directory but not its ignore
  rules.

## References

- `meta/tickets/0027-ephemeral-file-separation-via-paths-tmp.md` — Ticket
  driving this ADR
- `meta/research/2026-04-07-pr-review-tmp-directory-usage.md` — Research
  identifying the co-location problem and the conflict between root-level
  gitignore patterns and `.gitkeep` persistence
- `meta/plans/2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md`
  — Implementation plan covering the `paths.tmp` registration, inner-`.gitignore`
  mechanism, and migration of `review-pr`'s ephemeral files
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md` —
  Prior decision on temp-directory usage for inter-skill PR diff handoff
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — Establishes
  the `paths.*` configurable-path convention this ADR extends
- `meta/decisions/ADR-0018-init-skill-for-repository-bootstrap.md` —
  Establishes the `init` skill that seeds `.gitkeep` and inner-`.gitignore`
  files for `paths.tmp`
