---
title: "Ephemeral file separation via paths.tmp"
type: adr-creation-task
status: done
---

# ADR Ticket: Ephemeral file separation via paths.tmp

## Summary

In the context of the `review-pr` skill co-locating transient working files
(`diff.patch`, `changed-files.txt`, `review-payload.json`, etc.) with persistent
review artifacts under `meta/reviews/prs/`, creating accidental-commit risk and
making gitignoring structurally awkward, we decided for a dedicated ephemeral
location — `{tmp}/pr-review-{number}/` resolved via a new `paths.tmp`
configurable key (default `meta/tmp`) — with contents protected by a
self-contained inner `.gitignore` (`*`, `!.gitkeep`, `!.gitignore`) rather than
a root-level entry, to achieve clean separation of transient vs persistent data
and reliable gitignoring that still survives fresh clones, accepting a
coordinated migration across multiple sections of `review-pr`'s SKILL.md, stale
directories remaining at the old location after upgrade, and a slightly
non-obvious placement of the ignore logic inside the directory itself.

## Context and Forces

- `review-pr` currently writes both ephemeral working files and the persistent
  `{number}-review-{N}.md` artifact under `meta/reviews/prs/pr-review-{number}/`,
  making accidental commits of transient data easy
- Gitignoring a pattern like `pr-review-*/` at the root is fragile: it risks
  excluding persistent artifacts and does not generalise to other skills that
  want ephemeral locations
- The README already documents `meta/tmp/` as written by `review-pr`, but the
  skill does not currently write there — documentation and behaviour diverge
- The existing config system exposes ten `paths.*` keys for relocatable output
  directories, but has no entry for ephemeral/temp data — `tmp` is the obvious
  eleventh key
- A root-level `meta/tmp/` gitignore entry would prevent git from descending
  into the directory at all, breaking `.gitkeep` persistence — the directory
  would not survive fresh clones
- ADR-0008 specifies a shared temp-directory pattern for PR diff delivery but
  uses a literal `/tmp` root; it does not address configurable tmp paths

## Decision Drivers

- Clean separation between transient and persistent data
- Reliable gitignoring that does not depend on fragile root-level patterns
- Consistency with the existing `paths.*` configurable-path convention
- Directory must persist across fresh clones so ephemeral consumers have
  somewhere to write
- Honour user overrides (e.g., relocating tmp to `.tmp` or outside `meta/`)

## Considered Options

For ephemeral-file location:
1. **Keep under `{pr reviews directory}/pr-review-{number}/`** — status quo;
   accidental-commit risk and fragile gitignoring.
2. **Move to `{tmp}/pr-review-{number}/`** — cleanly separates transient from
   persistent; aligns with README; enables wholesale gitignoring of tmp
   contents.

For `tmp`-path configurability:
1. **Hardcode `meta/tmp` in skills** — inconsistent with the other ten
   `paths.*` keys; not overridable.
2. **Register `tmp` as the 11th configurable path key** — resolvable via
   `config-read-path.sh tmp meta/tmp`; documentation-only change since the
   script already delegates any key generically.

For gitignoring mechanism:
1. **Add `meta/tmp/` to the consumer's root `.gitignore`** — prevents git
   descending into the directory; breaks `.gitkeep` persistence; directory
   does not survive fresh clones.
2. **Self-contained inner `.gitignore` (`*`, `!.gitkeep`, `!.gitignore`)** —
   directory persists; contents ignored; ignore logic lives with the
   directory it governs.

For relationship to ADR-0008:
1. **Amend ADR-0008** to make the tmp root configurable.
2. **Add this as a follow-up ADR** that extends ADR-0008's literal-`/tmp`
   design with a configurable alternative for plugin-produced ephemeral files.

## Decision

We will:

- Register `tmp` as the 11th configurable path key (default `meta/tmp`),
  resolved via `config-read-path.sh tmp meta/tmp` — a documentation-only
  change since `config-read-path.sh` already delegates any key to
  `config-read-value.sh` generically
- Migrate `review-pr`'s ephemeral files from
  `{pr reviews directory}/pr-review-{number}/` to
  `{tmp}/pr-review-{number}/`, updating every section of `review-pr`'s
  SKILL.md that references the old path
- Protect tmp contents via a self-contained inner `.gitignore` (`*`,
  `!.gitkeep`, `!.gitignore`) seeded by the `init` skill — no root-level
  `meta/tmp/` entry
- Add this as a follow-up to ADR-0008 rather than amending it; ADR-0008's
  literal-`/tmp` design remains in place for its specific PR diff delivery
  pattern while plugin-produced ephemeral files now use `paths.tmp`

## Consequences

### Positive
- Transient and persistent data are cleanly separated
- `{tmp}/` contents are ignored wholesale via the inner `.gitignore`; no
  fragile root-level patterns
- README and behaviour converge — `meta/tmp/` is finally the documented
  ephemeral location
- `paths.tmp` is user-overridable like every other configurable path
- Other skills gain a conventional ephemeral location (`{tmp directory}/...`)
  without inventing their own

### Negative
- Existing `pr-review-*/` directories from previous sessions remain stale and
  un-gitignored at the old location until consumers clean them up manually
- Migration requires coordinated edits across multiple sections of
  `review-pr`'s SKILL.md; drift between sections reintroduces bugs
- Ignore logic inside the directory is less discoverable than a root-level
  entry (mitigated by documentation in the `init` skill)

### Neutral
- `paths.tmp` resolution is documentation-only — no code change to
  `config-read-path.sh`
- The relationship with ADR-0008 is additive: ADR-0008's literal `/tmp`
  pattern remains, and this decision introduces a parallel configurable
  `paths.tmp` for plugin-produced ephemeral files

## Source References

- `meta/research/2026-04-07-pr-review-tmp-directory-usage.md` — evidence for
  the hardcoded-tmp bug and analysis of the conflict with ADR-0008
- `meta/plans/2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md`
  — migration strategy, inner-`.gitignore` mechanism, `tmp` as the 11th
  configurable path key, stale-directory migration notes
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md` —
  prior decision this ADR extends
