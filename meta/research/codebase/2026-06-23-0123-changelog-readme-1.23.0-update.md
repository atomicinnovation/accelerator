---
type: codebase-research
id: "2026-06-23-0123-changelog-readme-1.23.0-update"
title: "Research: User-facing CHANGELOG and README update for 1.23.0"
date: "2026-06-23T12:35:21+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0123"
parent: "work-item:0123"
relates_to: ["codebase-research:2026-06-17-readme-changelog-1.22.0-refresh"]
topic: "User-facing CHANGELOG and README update for 1.23.0"
tags: [research, codebase, changelog, readme, release, migrations, sync-work-items, worktree]
revision: "5fdc6873db5d9cc4e3e97bd6fb95073bfd20f91d"
repository: "accelerator"
last_updated: "2026-06-23T12:35:21+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: User-facing CHANGELOG and README update for 1.23.0

**Date**: 2026-06-23 12:35 UTC
**Author**: Toby Clemson
**Git Commit**: 5fdc6873db5d9cc4e3e97bd6fb95073bfd20f91d
**Branch**: workspace `build-system` (main @ 2846b9a12a0c)
**Repository**: accelerator

## Research Question

For the 1.23.0 release (work item 0123), determine the **user-facing** change
set between the 1.22.0 release point and current `main`, so we can:

1. Record consumer-relevant changes under the CHANGELOG's existing
   `## [Unreleased]` section (Keep a Changelog grouping), **without** promoting
   to a `## [1.23.0]` heading or adding a date.
2. Add an **upgrade callout** for the fixes to the existing migration 0007 (no
   new migration this cycle).
3. Refresh the README feature/skill catalogue for skills and capabilities added
   or materially changed this cycle.

Developer-of-the-plugin changes (CI, build-system, tests, lint/tooling,
internal refactors, planning/dogfooding artifacts) are explicitly excluded.

## Summary

The 1.22.0 release commit is `082e202f072a "Bump version to 1.22.0 [skip ci]"`.
**101 non-version-bump commits** separate it from `main` (current
`1.23.0-pre.12`). As in the 1.22.0 cycle, the overwhelming majority are
internal — work-item status churn, Linear sync of the dogfooded backlog,
CI/test resilience, executable-bit auditing, pyrefly/SIGPIPE/stat-mtime build
fixes, and the planning artifacts behind the features — and **must not** appear
in a user-facing CHANGELOG.

The genuinely user-facing surface this cycle is small and tightly clustered
around **work-item sync** and **the migration framework**:

- **`/accelerator:sync-work-items` (new skill)** — batch reconciliation of
  `meta/work/` against the configured remote tracker (Jira or Linear):
  bidirectional / push-only / pull-only, `--preview`, five sync states against a
  `last-sync.json` baseline, conflict resolution UX, offers to push never-synced
  items and pull untracked remote issues. **Not yet in the CHANGELOG.**
- **`/accelerator:list-work-items` — sync-state column** — a colour-coded Sync
  column / labels rendered when an integration is configured. **Not yet in the
  CHANGELOG.**
- **`/accelerator:migrate` — agent decisions bridge** (`--list`,
  `--decisions-file`, validated dry-apply). **Already drafted** in the current
  `[Unreleased]` Added block.
- **`/accelerator:migrate` — strict argument handling** (unknown flags/positionals
  fail; `--help` to stdout). **Already drafted** in the current `[Unreleased]`
  Changed block.
- **Migration 0007 fixes** — the original 0007 stalled mid-run on real corpora
  (rewrote files, then its own validator rejected them, aborting before
  recording completion). This cycle closed every normalisation gap so 0007 now
  runs to completion. **This is the upgrade-callout target; not yet in the
  CHANGELOG.**
- **Git linked-worktree fix** — `find_repo_root` / `vcs_mode` tested for a
  `.git` *directory*; in a linked worktree `.git` is a *file*, so detection
  failed. Symptoms: `/accelerator:visualise` aborting with an empty error, and
  work-item sync treating every file as dirty. **Fixed; not yet in the
  CHANGELOG.**
- *(Borderline / optional)* a **structured stall** replacing opaque
  interactive-migration aborts (companion to the agent-decisions bridge), and
  **resume-safe partial migration failure** handling.

The current `[Unreleased]` block documents **only** the two `/accelerator:migrate`
items. Everything else above is missing. The README has **no home** for
`sync-work-items`, the new sync-state column, migration 0007, the new migrate
flags, or git-worktree detection — all are absent.

## Detailed Findings

### CHANGELOG — current state

`CHANGELOG.md:3-26` — the `[Unreleased]` section currently contains exactly two
entries:

- **Added**: `/accelerator:migrate` agent decisions bridge (`--list` dry-emit of
  pending interactive transforms; `ACCELERATOR_MIGRATE_DECISIONS_FILE` /
  `--decisions-file` validated up front by a no-mutation dry-apply; SKILL.md
  invoker contract). (`CHANGELOG.md:5-19`)
- **Changed**: `/accelerator:migrate` strict argument handling (unknown flag *or*
  positional exits non-zero; `--help`/`-h` print to stdout). (`CHANGELOG.md:21-26`)

`CHANGELOG.md:28-145` is the released `## [1.22.0] - 2026-06-17` entry. It opens
with the model upgrade callout blockquote (`CHANGELOG.md:30-38`) to copy for the
0007 callout, uses `Added` / `Changed` / `Migrations` subsections, and already
shipped the **Remote-tracker sync *ergonomics*** (create-work-item push-on-accept,
create-jira-issue work-item-file mode, list-work-items sync *label*) at
`CHANGELOG.md:93-97` — distinct from this cycle's batch **sync-work-items** skill.

### CHANGELOG — what to add to `[Unreleased]`

Proposed additions (Keep a Changelog grouping), written for consumers:

**Added**

- **`/accelerator:sync-work-items`** — reconcile local work items in `meta/work/`
  with your configured remote tracker (Jira or Linear). Bidirectional by
  default, plus `--push-only`, `--pull-only`, a non-destructive `--preview`,
  `--all` to bypass project scope, and pass-through tracker filter flags
  (`--label`, `--assignee`, `--state`, …). Detects five sync states (synced,
  unsynced, locally modified, remotely modified, conflict) against a
  `last-sync.json` baseline; resolves conflicts via a section-grouped diff and an
  explicit typed prompt (`remote`/`local`/`skip` — never `y/n`, so a reflexive
  Enter cannot discard local edits); offers to push never-synced items and pull
  untracked remote issues; never overwrites dirty local files; crash-safe and
  idempotent across interrupted runs.
- **`/accelerator:list-work-items` — Sync column.** When an integration is
  configured, the listing gains a colour-coded Sync column / per-item labels
  (`🟢 synced`, `⚪ unsynced`, `🔵 locally modified`, `🟣 remotely modified`,
  `🔴 conflict`). With no `last-sync.json` baseline it shows presence-only
  (synced/unsynced); with a baseline, tracked items upgrade to the three
  change-detected states. Output is unchanged when no integration is set.

**Changed / Fixed — migration framework (vague framing, per direction)**

> **Decision (Toby, 2026-06-23):** keep the migration entries *high-level* —
> describe **migration bug fixes and resilience**, not the flag-level mechanics.
> The current `[Unreleased]` migrate bullets (`CHANGELOG.md:5-26`) are too
> detailed (`--list`, `--decisions-file`, dry-apply, strict-arg specifics) and
> should be **collapsed** into a terse resilience/bug-fix line rather than
> expanded.

Recommended single consolidated entry covering all this cycle's migration work
(the agent-decisions bridge, structured stall, strict-arg handling, resume-safe
partial-failure, and the 0007 normalisation completeness fixes), e.g.:

- **Migration framework — more robust and resilient.** Interactive migrations can
  now be completed reliably in non-interactive/automated contexts, partially
  applied migrations resume cleanly after an interruption, and several
  frontmatter-normalisation bugs were fixed so migrations run to completion on
  more corpora. (See the upgrade note for migration 0007.)

This replaces — does not supplement — the two detailed migrate bullets currently
drafted at `CHANGELOG.md:5-26`.

**Fixed**

- **Skills and hooks now work inside git linked worktrees.** Repository-root and
  VCS-mode detection tested for a `.git` *directory*, but in a git linked
  worktree (e.g. a Conductor workspace) `.git` is a file, not a directory. As a
  result `/accelerator:visualise` (and its stop/status variants) could fail with
  an empty error message, and work-item sync could treat every work-item file as
  having uncommitted changes. Detection now recognises both forms, so
  worktree-based sessions behave exactly like plain checkouts.

**Migrations** (upgrade callout + no new migration)

Model on the 1.22.0 blockquote (`CHANGELOG.md:30-38`). No new migration ships
this release, but **migration 0007's behaviour changed** — significant enough to
alert upgraders:

> **No new migration this release, but migration 0007 was fixed.** If a previous
> `/accelerator:migrate` run stalled on migration 0007, **re-run it** — it now
> runs to completion. The original 0007 left several frontmatter-normalisation
> cases incomplete, so on real corpora it would rewrite files and then fail its
> own validation gate without recording completion, repeating identically on
> re-run. 0007 now: types PR-description files under `meta/prs/`; drops
> schema-forbidden keys (folding `pr_title` into `title` when absent); strips
> obsolete `ticket` / `ticket_id` keys; backfills missing required fields
> (derived where possible, otherwise stamped `unknown`); normalises PR links
> like `"PR #416"` to `"pr:416"`; and scopes itself to your configured `paths.*`
> directories (skipping freeform directories like `meta/docs/`). All non-trivial
> coercions are logged as `0007-DIVERGE[...]` breadcrumbs; VCS revert remains the
> recovery path.

**Excluded as non-user-facing** (do not add): Linear sync of the dogfooded
backlog and all work-item status churn; "Add task to update documentation",
backlog extraction/notes; config-driven corpus-validation *internals* and
validator guard tests; the executable-bit invariant guard + sourced-library
manifest; pyrefly node_modules race fix; SIGPIPE-safe `grep -q`, dual-platform
`stat` mtime, api_smoke / e2e test resilience; the 1.21/1.22 release
announcement; and every research/plan/review/validation artifact.

### CHANGELOG — sequencing caveat (acceptance criterion 2)

The work item forbids a `## [1.23.0]` heading or release date — leave promotion
to the release process. Keep all additions under `## [Unreleased]`
(`CHANGELOG.md:3`). Do **not** touch version-coherence files (`plugin.json`,
`Cargo.toml`, `checksums.json`) — acceptance criterion 6.

### `/accelerator:sync-work-items` (detail)

- Skill: `skills/work/sync-work-items/SKILL.md:1` (frontmatter `:2-7`). Name
  `sync-work-items`; argument-hint
  `[--push-only|--pull-only] [--preview] [--all] [filter-flags…]`.
- Registered via the directory entry `./skills/work/` at
  `.claude-plugin/plugin.json:21` — **no per-skill registration line needed**.
- Supporting scripts under `skills/work/scripts/`: `work-item-sync-decide.sh`
  (mode/decision table), `work-item-sync-classify.sh` (five-state change
  detection), `work-item-sync-baseline.sh` (`last-sync.json` store),
  `work-item-sync-label.sh` (glyph/label vocabulary), `work-item-sync-apply.sh`,
  `work-item-fetch-remote.sh`, `work-item-create-remote.sh`,
  `work-item-project-remote.sh`, `work-item-section-diff.sh`,
  `work-item-file-dirty.sh`.
- Five states (`work-item-sync-classify.sh:22-46`): `synced`, `unsynced`,
  `locally-modified`, `remotely-modified`, `conflict` (+ internal
  `remote-absent` / `indeterminate`). "Synced" iff a non-empty `external_id` is
  present; local `id` is never pushed. (Aligns with the
  external_id-as-remote-key convention, ADR-0044.)
- Change detection: local mtime pre-filter against baseline `timestamp`, then
  authoritative SHA-256 of normalised content vs per-item `local_hash` /
  `remote_hash`; normalisation ignores whitespace and remote-managed fields so a
  reformat/touch isn't flagged (`work-item-sync-classify.sh:30-46`).
- Baseline `last-sync.json` lives at
  `<paths.integrations>/<work.integration>/last-sync.json` (e.g.
  `.accelerator/state/integrations/jira/last-sync.json`); atomic write,
  baseline-last so interrupted runs are resumable
  (`work-item-sync-baseline.sh:4-32`; `SKILL.md:189-191`).
- Safety: pull-overwrite and untracked-pull blast-radius gates at threshold 25
  (`SKILL.md:171-181,288-299`); dirty files skipped (`skip-dirty`,
  `SKILL.md:156-158`); terminal API errors (code 71) never auto-retried.
- **No new config keys** — reuses `work.integration`, `work.default_project_code`,
  `paths.integrations` (all already documented). Supported trackers: Jira and
  Linear (full adapters in `work-item-fetch-remote.sh:250-289`); `trello` /
  `github-issues` report "not available" (exit 72).
- `/accelerator:list-work-items` changes gated on `work.integration` being set
  (`skills/work/list-work-items/SKILL.md:27-33`); Sync column appended at
  `:360-363`, labels from `work-item-sync-label.sh:54-67`.

### Migration 0007 fixes (detail) — upgrade-callout basis

The original 0007 mutated ~147 files on a real corpus, then its own validator
(`scripts/validate-corpus-frontmatter.sh`) rejected ~136 and the script aborted
before recording completion — a **permanent stall** (idempotent re-run repeats
identically). RCA:
`meta/research/issues/2026-06-17-migration-0007-incomplete-mechanical-normalisation.md:26-205`
(197 violations across `INVALID-TYPE`, `FORBIDDEN-OWN-ID 'pr_title'`,
`OBSOLETE-LEGACY-KEY 'ticket'`, `MISSING-EXTRA`, `BAD-LINKAGE-SHAPE`).

What 0007 now does that it did not before:

1. **Types `meta/prs/` PR-description files** via the config-driven doc-type
   table — previously skipped (empty `type:`) then rejected as `INVALID-TYPE`.
   (`scripts/doc-type-inference.sh`; `0007-frontmatter-rewrite.awk:81-98,323-326`)
2. **Drops schema-forbidden own-id keys**, folding `pr_title` → `title` when
   absent (else logged as `0007-DIVERGE[discarded-key]`); also drops
   `review_pass`. (`0007-frontmatter-rewrite.awk:357-372`;
   `0007-unify-meta-corpus-frontmatter.sh:108-114`)
3. **Drops obsolete `ticket` / `ticket_id`** on any type (non-empty value logged
   `0007-DIVERGE[dropped-legacy-key]`). (`0007-frontmatter-rewrite.awk:335-342`)
4. **Backfills required type-extras** on already-fenced files — `topic` from
   title, `pr_number` from a genuine `pr-`/`PR-` stem segment,
   `review_number`/`sequence` → 1, etc.
   (`0007-unify-meta-corpus-frontmatter.sh:188-230`)
5. **`unknown` backfill sentinel** for underivable required extras (the exact
   stall fix) — validator taught that `unknown` is present/non-empty.
   (`0007-unify-meta-corpus-frontmatter.sh:514-524`;
   `scripts/test-validate-corpus-frontmatter.sh:139-162`)
6. **Coerces non-canonical PR references** (`"PR #416"`, `"PR-416"`, `"#416"`,
   …) → `"pr:416"`. (`0007-frontmatter-rewrite.awk:158-168`)
7. **Config-driven path classification + allowlist**, single-sourced between
   migration and validator; fail-closed; honours configured `paths.*`; skips
   untypeable freeform dirs like `meta/docs/`. (`scripts/doc-type-inference.sh`,
   `doc-type-table.sh`, `config-read-doc-type-paths.sh`)
8. A combined-corpus capstone regression guard asserting the mechanical passes
   alone leave the corpus validator-clean. (`test-migrate-0007.sh:1703-1929`)

Consumer framing: **no new migration; 0007 just completes correctly now** — see
the callout above. Bundled migrations remain **0001–0007** (no 0008).

### Git linked-worktree fix (detail)

- `scripts/vcs-common.sh`: `find_repo_root()` (`:8-18`, walk-up test now `:11`
  uses `-e`) and `vcs_mode()` (`:27-36`, `:29`/`:31` use `-e`). Three `-d` → `-e`
  changes; in a linked worktree `.git` is a file with a `gitdir:` pointer.
- Two symptoms: (a) `find_repo_root` returned 1 under `set -euo pipefail` →
  silent abort with empty stderr — hit by `/accelerator:visualise` via
  `launch-server.sh:13` (and stop/status); (b) `vcs_mode` returned `none` → the
  fail-safe-to-DIRTY branch in `work-item-file-dirty.sh:101-104` marked **every**
  work-item file dirty, breaking `/sync-work-items`.
- Blast radius: `find_repo_root` is the documented single source of truth,
  sourced by ~30 scripts (VCS hooks, config layer, visualiser, Jira/Linear
  init/auth, work-item/ADR numbering). Every Conductor-based session was
  affected.
- Regression tests: `hooks/test-vcs-detect.sh:444-480`,
  `skills/work/scripts/test-work-item-scripts.sh:1590-1595`.
- Scope note: convergence onto the 0058 probe layer (`classify_checkout`) is
  deferred to work item 0125 (this is the minimal `-d`→`-e` fix).

### README — required updates

The README (`README.md`, 878 lines) has **no home** for any of this cycle's
user-facing items. Insertion points:

1. **`sync-work-items` — Work Item Management** (`README.md:319`). Add a row to
   the skill table (`:346-352`, currently create/extract/list/update/review) and
   ideally a node in the workflow diagram (`:335-344`). The skill is in the
   `work-item` family but its remote-reconciliation behaviour is also thematically
   at home in **Remote Work Item Management (Jira & Linear)** (`:361`), whose
   intro already frames `external_id` as the synced signal (`:369-370`) — a
   cross-reference there is warranted. No existing section describes *batch*
   sync/reconciliation today.
2. **`list-work-items` Sync column.** Update the `list-work-items` row
   description (`README.md:350`) to mention the sync-state column.
3. **Migrations** (`README.md:142-170`). **Decision (Toby, 2026-06-23):** the
   README should **not** mention specific migrations or the new migrate flags —
   leave it at "a `/accelerator:migrate` command exists." No migration-0007
   detail, no `--list` / `--decisions-file` / agent-decisions documentation here.
   The existing high-level Migrations section needs **no change** for this cycle.
4. **Git worktree detection — VCS Detection** (`README.md:172-189`). Speaks only
   of `.jj/`/`.git/` in "the working directory"; **no mention of worktrees**.
   Optionally note linked-worktree support now that it works.
5. Already accurate (spot-checked, no change): Linear section (`:454-501`), Jira
   section (`:375-452`), Getting Started (`:19`), `work.integration` allowed
   values blurb (`:326-333`), `create-note` (shipped 1.22.0).

Scope per work item: documentation edits only; no version-coherence bump, no
release/tag/publish.

## Code References

- `CHANGELOG.md:3-26` — current `[Unreleased]` (only the two migrate items).
- `CHANGELOG.md:28-38` — 1.22.0 heading + upgrade-callout blockquote to model.
- `CHANGELOG.md:93-97` — 1.22.0 sync *ergonomics* (distinct from this cycle).
- `skills/work/sync-work-items/SKILL.md:1-7` — new skill frontmatter.
- `skills/work/scripts/work-item-sync-classify.sh:22-46` — five sync states.
- `skills/work/scripts/work-item-sync-label.sh:54-67` — sync labels.
- `skills/work/list-work-items/SKILL.md:27-33,360-363` — Sync column gating.
- `.claude-plugin/plugin.json:21` — `./skills/work/` directory registration.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh` — the
  fixed migration (`:108-114,188-230,514-524`).
- `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:81-98,158-168,335-372`.
- `scripts/doc-type-inference.sh`, `scripts/doc-type-table.sh`,
  `scripts/config-read-doc-type-paths.sh` — config-driven classification.
- `scripts/vcs-common.sh:8-18,27-36` — `find_repo_root` / `vcs_mode` worktree fix.
- `skills/work/scripts/work-item-file-dirty.sh:45,101-104` — dirty fail-safe.
- `README.md:142-170` (Migrations), `:172-189` (VCS Detection), `:319-359`
  (Work Item Management), `:361-501` (Remote Work Item Management).

## Architecture Insights

- **Sync model.** `sync-work-items` formalises the `external_id`-presence sync
  signal (ADR-0044) into a full reconciliation engine: a per-item baseline
  (`last-sync.json`) + content hashing yields the five-state model, and the
  same classifier feeds both the sync apply path and the `list-work-items` Sync
  column — one source of truth, two surfaces. It is the natural completion of the
  1.22.0 "sync ergonomics" groundwork (push-on-accept / single-issue create).
- **Migration robustness.** This cycle's migration work is all about making the
  framework survive non-interactive and partial-failure conditions: the 0007
  normalisation completeness fixes (no more self-inflicted validator stall), the
  agent-decisions bridge + structured stall (interactive migrations satisfiable
  without a human), and resume-safe partial-failure handling (0119). The
  classifier/allowlist was single-sourced between migration and validator to kill
  a drift class. None of this is a new migration — the **upgrade obligation is
  unchanged** (still "run `/accelerator:migrate`"), only the outcome improved.
- **Worktree correctness.** The `-d`→`-e` fix is one line of intent (a worktree's
  `.git` is a file) with a ~30-script blast radius because `find_repo_root` is the
  single root-detection primitive — a good example of why the README's VCS
  Detection section understates the surface.

## Historical Context

Provenance for the user-facing items (per `documents-locator`):

- **sync-work-items**: `meta/work/0051-sync-work-items-skill.md`;
  `meta/research/codebase/2026-06-18-0051-sync-work-items-skill.md`;
  `meta/plans/2026-06-18-0051-sync-work-items-skill.md`;
  `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md`. Core
  integration touch-points: 0047 (`…2026-06-15-0047-core-skills-sync-integration`).
- **Migration 0007 fixes**: `meta/work/0114-…`, `meta/work/0118-…`;
  `meta/research/issues/2026-06-17-migration-0007-incomplete-mechanical-normalisation.md`;
  `meta/plans/2026-06-17-0114-…`, `meta/plans/2026-06-18-0114-config-driven-corpus-validation-scope.md`,
  `meta/plans/2026-06-20-0118-…`.
- **Migrate agent-decisions bridge / structured stall**: `meta/work/0115-…`,
  `0116-…`, `0117-…`, `0120-…`;
  `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`;
  `meta/decisions/ADR-0037-…`, `ADR-0023-meta-directory-migration-framework.md`.
- **Git worktree fix**: `meta/work/0124-find-repo-root-fails-in-git-worktrees.md`;
  `meta/research/codebase/2026-06-22-0124-…`; `meta/plans/2026-06-22-0124-…`;
  antecedent 0058 (workspace/worktree boundary). 0125 (probe-layer convergence)
  is a deferred follow-up, **not** in this cycle's user-facing surface.
- **Resume-safe partial migration**: `meta/work/0119-…` and its plan/validation.
- Prior precedent for this whole exercise:
  `meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md`.

## Related Research

- `meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md` — the
  1.22.0 CHANGELOG/README refresh (same task, prior cycle; structural model).

## Decisions (resolved 2026-06-23)

1. **Migrate-entry consolidation — RESOLVED.** Be vague: describe migration **bug
   fixes and resilience**, not flag-level mechanics. Collapse the two detailed
   migrate bullets currently in `[Unreleased]` (`CHANGELOG.md:5-26`) into one
   terse resilience/bug-fix line (drafted above), plus the 0007 upgrade callout.
2. **Structured stall + resume-safe partial failure — RESOLVED.** Folded into the
   single high-level migration-resilience line; not itemised.
3. **README depth for migrate flags — RESOLVED.** README mentions only that a
   `/accelerator:migrate` command exists; no specific migrations, no flags. The
   Migrations section needs no change this cycle.

## Open Questions

1. **`work.integration` overstatement.** As in 1.22.0, the allowed-values list
   includes `trello` / `github-issues` which still have no skills; `sync-work-items`
   reports "not available" for them. Worth one clause noting only jira/linear are
   implemented (if not already added in the 1.22.0 pass).
</content>
</invoke>
