---
work_item_id: "0031"
title: "Consolidate Accelerator-Owned Files Under .accelerator/"
date: "2026-05-04T22:19:10+00:00"
author: Toby Clemson
type: story
status: done
priority: high
parent: ""
tags: [configuration, init, migration, paths, integrations, visualiser]
---

# 0031: Consolidate Accelerator-Owned Files Under .accelerator/

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an Accelerator user, I want all plugin-owned files to live under a single
`.accelerator/` tree so that the distinction between my project's content and
the plugin's config and state is clear, removing Accelerator from a repo is
trivial, and selectively gitignoring plugin state requires no knowledge of
`meta/` internals.

Accelerator-specific files are currently scattered across `.claude/` (config
files, per-skill context and instructions, custom lenses) and `meta/`
(migration state, integration state, ephemeral tmp files, template overrides).
Everything else under `meta/` is human-authored content that belongs to the
codebase regardless of which agent tooling is in use. Consolidating
Accelerator-owned files under a single `.accelerator/` root enforces that
separation and gives integration init skills full ownership of their own
state directories.

## Context

The current layout grew incrementally: config files followed the Anthropic
`plugin-dev` convention (`.claude/<plugin>.md`), migration state landed in
`meta/` alongside other artifacts, and the Jira integration added
`meta/integrations/jira/` under the same convention. With multiple integrations
now in flight, the cost of baking `meta/integrations/` further into skill prose
is rising faster than the cost of doing the reorganisation.

The Jira research document (2026-04-29) explicitly deferred this reorg as a
separate work item, and the original note (2026-04-29) laid out the proposed
directory structure and rationale in full. This story executes that structure,
extended to cover the full scope of Accelerator-owned userspace configuration
surfaces identified in ADR-0016, ADR-0017, ADR-0019, and ADR-0020. The backwards-compatibility
window the note proposed (read from old paths, log a deprecation warning, remove
after one minor version) was considered and rejected. Instead this story adopts
a hard cut (immediate flag day: old paths are removed, no fallback, migration
0003 is the sole recovery path).

Integration init skills (e.g. `init-jira`) take ownership of their own state
directories under `.accelerator/state/integrations/<tool>/`, including
gitignore rules. The `init` skill creates only the Accelerator core scaffold.
This story also establishes that ownership model as the forward convention for
all future integration init skills — not merely relocates existing files.

## Requirements

### Target directory structure

```
.accelerator/
  config.md           # was .claude/accelerator.md  (ADR-0016)
  config.local.md     # was .claude/accelerator.local.md  (ADR-0016, gitignored)
  skills/
    <name>/
      context.md      # was .claude/accelerator/skills/<name>/context.md  (ADR-0020)
      instructions.md # was .claude/accelerator/skills/<name>/instructions.md  (ADR-0020)
  lenses/
    <name>-lens/
      SKILL.md        # was .claude/accelerator/lenses/<name>-lens/SKILL.md  (ADR-0017)
  templates/          # was meta/templates/  (ADR-0017 tier-2 template resolution)
  state/
    migrations-applied    # was meta/.migrations-applied
    migrations-skipped    # was meta/.migrations-skipped
    integrations/         # not seeded by init — owned by integration init skills
      jira/               # created by init-jira, not by accelerator:init
        fields.json
        projects.json
        site.json         # gitignored (per-developer: accountId, displayName)
        .gitignore        # written by init-jira and by migration 0003
                          # ignores site.json, .refresh-meta.json, .lock/
        .gitkeep
  tmp/                    # was meta/tmp/  (ADR-0019)
    .gitignore            # inner pattern: *, !.gitkeep, !.gitignore
    .gitkeep
```

### `init` skill

- Creates the core `.accelerator/` scaffold on first run: top-level `.gitignore`
  (covering `config.local.md`), `state/`, `skills/`, `lenses/`, `templates/`,
  and `tmp/` with its inner `.gitignore` and `.gitkeep` files.
- Does **not** create `.accelerator/state/integrations/` or any subdirectory
  under it — those are owned by their respective integration init skills.
- Updates the project root `.gitignore` to cover the relocated config local
  file using the **anchored** form `.accelerator/config.local.md` (replacing
  the old `.claude/accelerator.local.md` rule). The anchored form prevents
  unrelated `config.local.md` files elsewhere in the repo from being silently
  ignored.
- No longer writes anything under `.claude/` or `meta/` as part of init.

### `init-jira` skill

- Creates `.accelerator/state/integrations/jira/` if it does not exist.
- Writes `.accelerator/state/integrations/jira/.gitignore` ignoring three
  entries: `site.json` (per-developer principal), `.refresh-meta.json`
  (refresh timestamp sidecar, not byte-idempotent), and `.lock/` (transient
  concurrency lock). Idempotent: each rule is appended only if not already
  present (per-rule `grep -qF` check before append); a `.gitignore` that
  already contains all three rules is left unchanged.
- Creates `.accelerator/state/integrations/jira/.gitkeep` if the directory
  would otherwise be empty.
- The previous root-level `.gitignore` rules at the project root for
  `<state>/.lock` and `<state>/.refresh-meta.json` (written by the legacy
  `init-jira` flow) are no longer added; the inner `.gitignore` covers them.
- Writes `site.json`, `fields.json`, and `projects.json` to this directory
  (same file set as the existing implementation — no new files introduced by
  this story), and reads the field catalogue from `fields.json` on subsequent
  invocations.
- Can be run on a fresh repo without `accelerator:init` having run first;
  creates `.accelerator/state/integrations/jira/` regardless. Emits a warning
  to stderr if `.accelerator/` itself is absent (suggesting the user run
  `accelerator:init`), but does not fail.

### `paths.tmp` default

- Updated from `meta/tmp` to `.accelerator/tmp` in `config-read-path.sh`.

### `integrations` path-config key

- A new key `integrations` added to `config-read-path.sh` with default
  `.accelerator/state/integrations`. Skills call
  `config-read-path.sh integrations .accelerator/state/integrations` rather
  than hardcoding any path, so updating the default in `config-read-path.sh` is
  the single point of change for any future path rename.

### Config scripts

All of the following updated to resolve from new paths:

- `scripts/config-common.sh` — `.claude/accelerator*.md` → `.accelerator/config*.md`
- `scripts/config-dump.sh` — same
- `scripts/config-read-skill-context.sh` — `.claude/accelerator/skills/` → `.accelerator/skills/`
- `scripts/config-read-skill-instructions.sh` — same
- `scripts/config-summary.sh` — skills and lenses discovery paths
- `scripts/config-read-review.sh` — `.claude/accelerator/lenses/*/SKILL.md` → `.accelerator/lenses/*/SKILL.md`
- `scripts/config-read-template.sh` — second-tier lookup from `meta/templates/` → `.accelerator/templates/`

### Visualiser

- `skills/visualisation/visualise/scripts/launch-server.sh:16` — default arg `meta/tmp` → `.accelerator/tmp`
- `skills/visualisation/visualise/scripts/stop-server.sh:13` — same
- `skills/visualisation/visualise/scripts/status-server.sh:13` — same
- `skills/visualisation/visualise/scripts/launch-server.sh:127` — error hint string updated from `.claude/accelerator.local.md` to `.accelerator/config.local.md`
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:48` — default arg `meta/templates` → `.accelerator/templates` (the other nine `abs_path` calls at lines 38-46 are deliberately unchanged: their targets remain in `meta/`)
- `skills/visualisation/visualise/SKILL.md:21` — default arg `meta/templates` → `.accelerator/templates`
- `skills/visualisation/visualise/SKILL.md:24` — default arg `meta/tmp` → `.accelerator/tmp`
- `skills/visualisation/visualise/SKILL.md:96-98` — user-facing docs updated to new config file paths
- `skills/visualisation/visualise/server/tests/fixtures/config.valid.json` — illustrative `meta/tmp` and `meta/templates` paths updated to reflect new defaults (tests pass either way; update is for accuracy)

### `configure/SKILL.md` and README

Updated throughout to reference new path locations for all config surfaces:
config files, per-skill customisation directories, custom lenses, templates,
and ephemeral tmp.

### Migration discoverability hook and driver clean-tree check

The SessionStart discoverability hook (`hooks/migrate-discoverability.sh`, per
ADR-0023) checks whether a repo is an Accelerator project by testing for the
presence of `.claude/accelerator.md` or content under `meta/`. The hook must be
updated to test for `.accelerator/` **in addition to** the existing sentinels
(three-clause OR: `.accelerator/`, `.claude/accelerator.md`, or `meta/`), so
that both pre-migration repos (which still need the migration warning to
prompt them to run `/accelerator:migrate`) and post-migration repos (for
future migrations 0004+) continue to trigger the hook.

The hook's state-file lookup must use a fallback chain: read
`.accelerator/state/migrations-applied` if `.accelerator/` exists, else fall
back to `meta/.migrations-applied`. This fallback is permanent — it is the
discoverability layer that lets un-migrated users learn about the migration
they need to run, and is not subject to the runtime-script hard-cut posture
that applies elsewhere in this work item.

The migration driver (`run-migrations.sh`) clean-tree check currently covers
`meta/` and `.claude/accelerator*.md`. It must also be extended to cover
`.accelerator/` so subsequent migrations protect the new paths.

Both the discoverability hook update and the clean-tree check extension are
prerequisites for migration 0003 — they must be reviewed and merged atomically
with the migration script, not merely in the same release cycle. Releasing
migration 0003 before these updates leaves migrated repos invisible to the
discoverability system and unprotected by the clean-tree guard.

### Migration `0003-relocate-accelerator-state.sh`

1. Refuses to run if the working tree has uncommitted changes to
   `.claude/accelerator*.md`, `meta/.migrations-*`, `meta/integrations/`, or
   `.accelerator/` (covers both the source paths being moved and the destination
   to prevent conflicts with partial previous runs).
2. Initialises `.accelerator/` scaffold before moving anything: creates
   top-level `.gitignore` (containing the anchored rule
   `.accelerator/config.local.md`), `state/.gitkeep`, `tmp/.gitignore`,
   `tmp/.gitkeep`, `skills/.gitkeep`, `lenses/.gitkeep`,
   `templates/.gitkeep`. Also updates the project root `.gitignore`:
   replaces any `.claude/accelerator.local.md` rule (anchored or unanchored
   form) with the anchored `.accelerator/config.local.md` rule; if no such
   rule exists, appends the new rule.
3. Moves each source path to its destination if the source exists:
   - `.claude/accelerator.md` → `.accelerator/config.md`
   - `.claude/accelerator.local.md` → `.accelerator/config.local.md`
   - `.claude/accelerator/skills/` → `.accelerator/skills/`
   - `.claude/accelerator/lenses/` → `.accelerator/lenses/`
   - `meta/templates/` → `.accelerator/templates/`
   - `meta/.migrations-applied` → `.accelerator/state/migrations-applied`
   - `meta/.migrations-skipped` → `.accelerator/state/migrations-skipped`
   - `meta/integrations/jira/` → `.accelerator/state/integrations/jira/`
   - `meta/tmp/` → `.accelerator/tmp/` (only if `paths.tmp` is at the default
     value; if overridden, the custom path is left untouched)
4. For `meta/integrations/jira/`, writes
   `.accelerator/state/integrations/jira/.gitignore` ignoring `site.json`,
   `.refresh-meta.json`, and `.lock/` as part of the move — bootstrapping
   the gitignore that `init-jira` would normally write. Also removes any
   pre-existing root-level `meta/integrations/jira/.lock` and
   `meta/integrations/jira/.refresh-meta.json` rules from the project root
   `.gitignore` (their replacements are now covered by the inner `.gitignore`).
5. Removes each source path that was successfully moved; sources that did not
   exist are not touched. All removals happen after all moves complete without
   error.
6. Hard cut (immediate flag day) — no fallback to old paths. Migration is the
   only recovery path.

## Acceptance Criteria

- [ ] Given a fresh project, when `accelerator:init` runs, then `.accelerator/`
  is created with the correct `.gitignore`, `state/.gitkeep`, `skills/.gitkeep`,
  `lenses/.gitkeep`, `templates/.gitkeep`, and `tmp/` with its inner
  `.gitignore` and `.gitkeep`; no Accelerator files are written to `.claude/`
  or `meta/`.

- [ ] Given a fresh project where `accelerator:init` has run but `init-jira`
  has not, then `.accelerator/state/integrations/` does not exist.

- [ ] Given `init-jira` runs on a fresh repo (with or without `accelerator:init`
  having run), then `.accelerator/state/integrations/jira/` is created with a
  `.gitignore` that ignores `site.json` and a `.gitkeep`; the skill completes
  successfully.

- [ ] Given `init-jira` runs on a repo that already has
  `.accelerator/state/integrations/jira/`, then the `.gitignore` continues to
  ignore `site.json`, `.refresh-meta.json`, and `.lock/`; `fields.json` and
  `projects.json` are refreshed from the live tenant; and re-running against
  an unchanged tenant produces no `git status` diff for `fields.json` or
  `projects.json`.

- [ ] Given `init-jira` completes successfully, then no `.lock/` directory
  remains under `.accelerator/state/integrations/jira/` (the lock is
  transient and removed on EXIT).

- [ ] Given the working tree has uncommitted changes to any of
  `.claude/accelerator*.md`, `meta/.migrations-*`, `meta/integrations/`, or
  `.accelerator/`, when migration `0003` is invoked, then it exits non-zero
  and no files are moved or removed.

- [ ] Given an existing project seeded with all of `.claude/accelerator.md`,
  `meta/.migrations-applied`, `meta/integrations/jira/fields.json`, and
  `meta/tmp/`, when migration `0003` is applied, then all files are present at
  their new `.accelerator/` locations and old paths are removed.

- [ ] Given migration `0003` moves `meta/integrations/jira/`, then the
  destination directory has a `.gitignore` ignoring `site.json` written by the
  migration.

- [ ] Given `paths.tmp` is at the default (`meta/tmp`), when migration `0003`
  is applied, then the directory and contents move to `.accelerator/tmp/` and
  the inner `.gitignore` is preserved.

- [ ] Given `paths.tmp` is explicitly overridden to a custom path, when
  migration `0003` is applied, then that custom path is left untouched.

- [ ] Given migration `0003` is applied and the repository is seeded with a
  skill context file at `.accelerator/skills/<name>/context.md`, a lens at
  `.accelerator/lenses/<name>-lens/SKILL.md`, and a template at
  `.accelerator/templates/<name>/`, when `config-read-skill-context.sh`,
  `config-read-skill-instructions.sh`, `config-read-review.sh`, and
  `config-read-template.sh` each run, then each exits 0 and the path it
  returns begins with `.accelerator/` (not `.claude/` or `meta/`).

- [ ] Given migration `0003` is applied and a valid `.accelerator/config.md`
  exists, when `config-common.sh`, `config-dump.sh`, and `config-summary.sh`
  run, then each exits 0 and no line whose value is a filesystem path (i.e.
  begins with `.` or `/`) contains a `.claude/` or `meta/` prefix.

- [ ] Given a project using custom lenses at `.claude/accelerator/lenses/`,
  when migration `0003` is applied, then `config-read-review.sh` discovers
  lenses at `.accelerator/lenses/` and the configure skill's documentation
  refers to that path.

- [ ] Given a project using per-skill context/instructions at
  `.claude/accelerator/skills/`, when migration `0003` is applied, then those
  files are injected correctly from `.accelerator/skills/`.

- [ ] Given a project with template overrides in `meta/templates/`, when
  migration `0003` is applied, then `config-read-template.sh` resolves those
  templates from `.accelerator/templates/` as the second tier.

- [ ] Given the `integrations` path-config key is added, when a skill calls
  `config-read-path.sh integrations`, then it returns
  `.accelerator/state/integrations` with no hardcoded path in skill prose.

- [ ] Given the Jira integration has been initialised and `site.json` was not
  previously committed to the repository, when migration `0003` is applied,
  then `fields.json` and `projects.json` are present under
  `.accelerator/state/integrations/jira/`; `site.json`, `.refresh-meta.json`,
  and `.lock/` are covered by the directory's inner `.gitignore`; and
  `config-read-path.sh integrations` returns `.accelerator/state/integrations`.

- [ ] Given migration `0003` is applied, when the migration skill runs again,
  then it reports `0003` as already applied (no double-move).

- [ ] Given migration `0003` is applied and no `paths.tmp` override is
  configured, when `launch-server.sh` is invoked with a valid configuration,
  then it exits 0 and the effective tmp path it uses is `.accelerator/tmp`.

- [ ] Given all migrations applied, then no Jira integration skill script
  (`init-jira`, `create-jira-issue`, `update-jira-issue`, `comment-jira-issue`,
  `attach-jira-issue`, `search-jira-issues`, `show-jira-issue`) contains a
  hardcoded reference to `meta/integrations/jira/` — each resolves the
  integration state path via `config-read-path.sh integrations`.

## Open Questions

None.

## Dependencies

- Blocked by: none (work can begin immediately)
- Release gate: migration 0003 must not be released until all seven Jira skill
  path updates (`init-jira`, `search-jira-issues`, `show-jira-issue`,
  `create-jira-issue`, `update-jira-issue`, `comment-jira-issue`,
  `attach-jira-issue`) and all config-script updates are merged — the hard cut
  makes this a breaking change, not a deprecation. Releasing the migration
  before these updates would cause immediate failures for every Jira skill user.
- Atomic delivery: the discoverability hook update (`hooks/migrate-discoverability.sh`)
  and migration driver clean-tree check extension (`run-migrations.sh`) must be
  reviewed and merged in the same commit or PR as migration 0003 — not merely
  in the same release cycle.
- Blocks: any future integration that would otherwise bake `meta/integrations/`
  into skill prose (e.g. Linear, Shortcut)
- Note: the `integrations` path-config key is functionally already wired up.
  `config-read-path.sh` is a 4-line pass-through that delegates any key to
  `config-read-value.sh paths.<key>` with the second arg as the default.
  Jira Phase 1 already calls `config-read-path.sh integrations meta/integrations`
  via `jira_state_dir()` in `skills/integrations/jira/scripts/jira-common.sh:62`.
  This work item only needs to (a) add `integrations` to the documented key list
  in the `config-read-path.sh` header comment, (b) update the documented default
  in `skills/config/configure/SKILL.md:402` from `meta/integrations` to
  `.accelerator/state/integrations`, and (c) change the runtime default arg in
  `jira-common.sh:62` to match.

## Assumptions

- `state/integrations/jira/site.json` is gitignored because it contains
  per-developer `accountId` and `displayName` from whoever ran `init-jira`;
  committing it would cause merge noise across team members. The other Jira
  state files (`fields.json`, `projects.json`) are team-shared and committed
  — consistent with the intent documented in the Jira research.
  `.refresh-meta.json` is gitignored because it embeds an ISO timestamp that
  is not byte-idempotent across refreshes; `.lock/` is gitignored because it
  is a transient concurrency lock removed on EXIT.
- Migration `0003` is the correct number; migrations `0001` and `0002` are
  confirmed to exist.
- `tmp/` uses the ADR-0019 inner-gitignore pattern (never a root-level entry
  for `tmp/`), because a root-level entry prevents git from descending into
  the directory and breaks `.gitkeep` persistence across fresh clones.
- Future Jira phases may add additional subdirectories under
  `.accelerator/state/integrations/jira/` (e.g. `issuetypes/`); this work
  item makes no provision for them beyond the parent directory itself
  existing. Any such additions are written by the relevant Jira skill at
  the time, not by `accelerator:init` or migration 0003.
- Pinned-path preservation per ADR-0023 is applied in this work item only to
  `paths.tmp`. `paths.templates` and `paths.integrations` are moved
  unconditionally by migration 0003; users who have explicitly pinned either
  to a `meta/<dir>` value will need to update their config post-migration.
  The simpler unconditional move is justified by the negligible
  explicit-override population for these two keys (both have been documented
  defaults for only weeks at the time this work item is drafted) and the
  cost of detecting "explicitly set" in pure bash 3.2.
- The other nine `meta/<dir>` defaults at
  `skills/visualisation/visualise/scripts/write-visualiser-config.sh:38-46`
  (`decisions`, `tickets`, `plans`, `research`, `review_plans`, `review_prs`,
  `validations`, `notes`, `prs`) are deliberately unchanged: their targets
  remain in `meta/` because they are project content, not Accelerator state.
- The `accelerator.md` → `config.md` rename is documented in a separate
  superseding ADR (forthcoming) that updates ADR-0016's userspace
  configuration model. The plugin-dev ecosystem-alignment rationale at
  ADR-0016:170-172 no longer applies once the file lives outside `.claude/`.
- The 2026-03-14 plugin-extraction work (`meta/research/2026-03-14-plugin-extraction.md`,
  `meta/plans/2026-03-14-plugin-extraction.md`) is confirmed irrelevant to this
  story as of 2026-05-05. That work addressed only the plugin's internal
  structure — moving skills and agents from `~/.claude/` to `${CLAUDE_PLUGIN_ROOT}/`
  — and placed no constraints on the `.accelerator/` root name or file
  placement in user repos. The userspace directory convention was established
  independently by ADR-0016 through ADR-0020.

## Technical Notes

Config scripts that need updating span:
`config-common.sh`, `config-dump.sh`, `config-read-skill-context.sh`,
`config-read-skill-instructions.sh`, `config-summary.sh`,
`config-read-review.sh`, `config-read-template.sh`, `config-read-path.sh`
(default for `tmp`; new key `integrations`).

Visualiser lifecycle scripts all already route through `config-read-path.sh` —
only the default arguments and documentation strings need updating, not the
resolution logic.

The pattern established here (each integration init skill owns and bootstraps
its own state directory) is intentionally forward-looking: future integrations
follow the same model, creating `.accelerator/state/integrations/<tool>/`
themselves rather than relying on `accelerator:init`.

## Drafting Notes

- Scope expanded beyond the original note to include `.claude/accelerator/skills/`
  and `.claude/accelerator/lenses/` (per ADR-0020 and ADR-0017 respectively)
  and `meta/templates/` (per ADR-0017 tier-2 template resolution).
- Hard cut (no backwards-compatibility fallback) confirmed in Step 1, with
  one exception: the SessionStart discoverability hook
  (`hooks/migrate-discoverability.sh`) reads its state file from a fallback
  chain (`.accelerator/state/migrations-applied` then
  `meta/.migrations-applied`) so that pre-migration repos still receive the
  warning telling them to run `/accelerator:migrate`. The hook is the only
  place this fallback applies; runtime scripts read the new path only.
- `init-jira` ownership of its state directory introduced in Step 3 as a
  deliberate separation-of-concerns decision: `accelerator:init` owns the core
  scaffold, integration init skills own their own subtrees.
- Visualiser changes are in-scope: the scripts already use `config-read-path.sh`
  correctly, but embed old defaults and documentation strings that would become
  misleading post-migration.
- Priority set to high based on user's stated preference to do this before more
  integrations bake the old paths into skill prose.

## References

- Source: `meta/notes/2026-04-29-accelerator-config-state-reorg.md`
- Research: `meta/research/2026-04-29-jira-cloud-integration-skills.md`
- Related: ADR-0016 (`meta/decisions/ADR-0016-userspace-configuration-model.md`)
- Related: ADR-0017 (`meta/decisions/ADR-0017-configuration-extension-points.md`)
- Related: ADR-0019 (`meta/decisions/ADR-0019-ephemeral-file-separation-via-paths-tmp.md`)
- Related: ADR-0020 (`meta/decisions/ADR-0020-per-skill-customisation-directory.md`)
- Related: ADR-0023 (`meta/decisions/ADR-0023-meta-directory-migration-framework.md`)
