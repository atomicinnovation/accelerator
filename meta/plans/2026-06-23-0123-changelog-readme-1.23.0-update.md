---
type: plan
id: "2026-06-23-0123-changelog-readme-1.23.0-update"
title: "User-Facing CHANGELOG and README Update for 1.23.0 Implementation Plan"
date: "2026-06-23T13:07:21+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0123"
parent: "work-item:0123"
derived_from: ["codebase-research:2026-06-23-0123-changelog-readme-1.23.0-update"]
relates_to: ["codebase-research:2026-06-17-readme-changelog-1.22.0-refresh"]
tags: [documentation, release, changelog, readme]
revision: "52a48331b22f8208e029fb5dfa59aa86ed1b4bf4"
repository: "accelerator"
last_updated: "2026-06-23T16:14:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# User-Facing CHANGELOG and README Update for 1.23.0 Implementation Plan

## Overview

Curate the user-facing CHANGELOG and README for the 1.23.0 release. Record the
consumer-relevant changes since 1.22.0 under the CHANGELOG's existing
`## [Unreleased]` section (Keep a Changelog conventions), add an upgrade callout
for the fixes to migration 0007, and refresh the README skill catalogue to cover
the new and materially-changed skills. This is a documentation-only change that
gates the 1.23.0 release.

The change set has already been derived and pre-decided in the research at
`meta/research/codebase/2026-06-23-0123-changelog-readme-1.23.0-update.md`
(including the exact prose for the CHANGELOG entries and the 0007 callout). This
plan turns that research into two independently-mergeable doc edits.

## Current State Analysis

- **CHANGELOG `[Unreleased]`** (`CHANGELOG.md:3-26`) currently contains **only**
  two detailed `/accelerator:migrate` bullets — an `Added` block for the agent
  decisions bridge (`:5-19`) and a `Changed` block for strict argument handling
  (`:21-26`). Per the resolved research decision these are too low-level and must
  be **collapsed** into one terse resilience line, not supplemented.
- The released **1.22.0 entry** (`CHANGELOG.md:28-145`) opens with an upgrade
  callout blockquote (`:30-38`) — the model for the 0007 callout — and uses
  `Added` / `Changed` / `Migrations` subsections. Its sync *ergonomics* entry
  (`:93-97`) is distinct from this cycle's batch `sync-work-items` skill.
- **README Work Item Management** (`README.md:319-359`): the skill table
  (`:346-352`) lists exactly create/extract/list/update/review — **no
  `sync-work-items` row**; the workflow diagram (`:335-344`) has no sync node;
  the `list-work-items` row description (`:350`) does not mention a Sync column.
- **README Remote Work Item Management** (`README.md:361-371`): intro frames
  `external_id` as the synced signal but does not mention batch reconciliation.
- **README VCS Detection** (`README.md:172-189`): describes only `.jj/` / `.git/`
  *directories*; no mention of linked worktrees.
- **README Migrations** (`README.md:142-170`): high-level, mentions only that
  `/accelerator:migrate` exists — **no change needed** (resolved decision).
- **No markdown/prose linter** exists in the task tree (`mise tasks`); the
  read-only CI mirror is `mise run check`. Prose edits pass it trivially, so the
  meaningful gate is that **no version-coherence files** (`plugin.json`,
  `Cargo.toml`, `checksums.json`) are touched.

### Key Discoveries:

- `sync-work-items` is already registered via the `./skills/work/` directory
  entry (`.claude-plugin/plugin.json:21`) — **no per-skill registration line is
  needed** and none should be added.
- The `sync-work-items` skill exists and is current
  (`skills/work/sync-work-items/SKILL.md:1-10`); argument-hint is
  `[--push-only|--pull-only] [--preview] [--all] [filter-flags…]`.
- Research **Open Question #1** (the `work.integration` overstatement) is
  **already resolved** in the current README — `README.md:326-329` already states
  only jira/linear ship skills and trello/github-issues are reserved. **No action.**
- The exact CHANGELOG prose (sync-work-items / list-work-items Added entries,
  the consolidated migration line, the worktree Fixed entry, and the 0007
  callout) is drafted verbatim in the research (`:112-186`) and is adopted below.

## Desired End State

- `CHANGELOG.md` `[Unreleased]` documents every user-facing change in the
  1.22.0→`main` range — `sync-work-items`, the `list-work-items` Sync column, the
  worktree fix, and a single consolidated migration-resilience line — plus a
  `Migrations` upgrade callout for the 0007 fixes. The two detailed migrate
  bullets are **gone**, replaced by the consolidated line. No `## [1.23.0]`
  heading and no release date are present.
- `README.md` skill catalogue reflects `sync-work-items` (table row + workflow
  node + cross-reference) and the `list-work-items` Sync column, and VCS
  Detection notes linked-worktree support.
- `plugin.json`, `Cargo.toml`, and `checksums.json` are **unmodified**.
- `mise run check` exits 0.

Verify by inspecting the diff (`jj diff`) — only `CHANGELOG.md` and `README.md`
should appear — and re-reading the `[Unreleased]` section against the 1.22.0
entry's grouping conventions.

## What We're NOT Doing

- **No** `## [1.23.0]` heading and **no** release date — left to the release
  process (acceptance criterion 2).
- **No** version-coherence bump and **no** edits to `plugin.json`, `Cargo.toml`,
  or `checksums.json` (acceptance criterion 6).
- **No** release/tag/publish action.
- **No** CHANGELOG entries for developer-internal changes (CI, build-system,
  tests, lint/tooling, internal refactors, Linear sync of the dogfooded backlog,
  planning/dogfooding artifacts) — explicitly excluded (research `:188-194`).
- **No** README Migrations-section edit and **no** documentation of specific
  migrations or the new migrate flags in the README (resolved decision,
  research `:411-413`).
- **No** flag-level detail in the CHANGELOG migration entry — kept high-level
  (resolved decision, research `:138-143`).
- **No** per-skill registration line for `sync-work-items` (already covered by
  the directory entry).

## Implementation Approach

Two cohesive, independently-mergeable doc edits, applied test-first in the only
sense applicable to prose: each phase's acceptance assertions are written as
checks run *after* the edit (there are no unit tests for prose). Phase 1 touches
`CHANGELOG.md` only; Phase 2 touches `README.md` only. Either can land first and
leave the repo green, satisfying the independent-integratability requirement. All
prose is adopted verbatim from the research draft so no new authoring decisions
arise during implementation.

## Phase 1: CHANGELOG `[Unreleased]` refresh

### Overview

Replace the two detailed migrate bullets with the full consumer-facing change
set under `## [Unreleased]`, in Keep a Changelog grouping, and add the 0007
upgrade callout. All entries stay under `## [Unreleased]`; no version heading or
date is introduced.

### Changes Required:

#### 1. Replace the `[Unreleased]` body

**File**: `CHANGELOG.md`
**Changes**: Replace the current `### Added` + `### Changed` blocks
(`CHANGELOG.md:5-26`) with the structure below. Subsection order follows the
Keep a Changelog canonical ordering (`Added` → `Changed` → `Fixed`) plus this
project's convention of placing the custom `Migrations` subsection last (as in
the 1.22.0 entry, which uses `Added` → `Changed` → `Migrations` and has no
`Fixed` section). The factual content is taken from the prose paragraphs within
research `:112-186` (the dated Decision blockquote at research `:138-143` and the
"Recommended …, e.g." recommendation lead-ins are editorial scaffolding and are
**not** copied into the CHANGELOG). Layout deviates from the research draft in
two deliberate, fact-preserving ways for reader scannability: the
`sync-work-items` entry is broken from one paragraph into sub-bullets, and the
0007 callout's internal-mechanics enumeration is moved out of the blockquote into
a following paragraph so the blockquote leads with the upgrade action.

```markdown
## [Unreleased]

### Added

- **`/accelerator:sync-work-items`** — reconcile local work items in `meta/work/`
  with your configured remote tracker (Jira or Linear).
  - Bidirectional by default, plus `--push-only`, `--pull-only`, a
    non-destructive `--preview`, `--all` (drops project scope on project-scoped
    trackers — Jira; Linear is single-team), and pass-through tracker filter
    flags (`--label`, `--assignee`, `--state`, …).
  - Detects five per-item sync states against a `last-sync.json` baseline:
    **synced**, **unsynced**, **locally modified** / **remotely modified**
    (changed on one side since the last sync), and **conflict** (changed on
    both).
  - Resolves conflicts via a section-grouped diff and an explicit typed prompt
    (`remote`/`local`/`skip` — never `y/n`, so a reflexive Enter cannot discard
    local edits); offers to push never-synced items and pull untracked remote
    issues.
  - Never overwrites dirty local files; crash-safe and idempotent across
    interrupted runs.
- **`/accelerator:list-work-items` — richer Sync column.** The Sync column
  (introduced in 1.22.0) now distinguishes five colour-coded states against a
  `last-sync.json` baseline (`🟢 synced`, `⚪ unsynced`, `🔵 locally modified`,
  `🟣 remotely modified`, `🔴 conflict`). With no baseline it shows presence-only
  (synced/unsynced); once a baseline exists, tracked items upgrade to the three
  change-detected states. Output is unchanged when no integration is set.

### Changed

- **Migration framework — more robust and resilient.** Interactive migrations can
  now be completed reliably in non-interactive/automated contexts, partially
  applied migrations resume cleanly after an interruption, and several
  frontmatter-normalisation bugs were fixed so migrations run to completion on
  more corpora. (See the upgrade note for migration 0007.)

### Fixed

- **Skills and hooks now work inside git linked worktrees.** Repository-root and
  VCS-mode detection tested for a `.git` *directory*, but in a git linked
  worktree (e.g. a Conductor workspace) `.git` is a file, not a directory.
  Because repository-root detection underpins many skills, the symptoms were
  session-wide: `/accelerator:visualise` (and its stop/status variants) could
  fail with an empty error message, work-item sync could treat every work-item
  file as having uncommitted changes, and other repository-root-dependent skills
  could misbehave. Detection now recognises both forms, so worktree-based
  sessions behave exactly like plain checkouts.

### Migrations

> **No new migration this release, but migration 0007 was fixed.** If a previous
> `/accelerator:migrate` run stalled on migration 0007, **re-run it** — it now
> runs to completion. Previously it could rewrite files and then fail its own
> validation gate without recording completion, so it repeated identically on
> re-run.

What 0007 now does (for the curious — no action needed): types PR-description
files under `meta/prs/`; drops schema-forbidden keys (folding `pr_title` into
`title` when absent); strips obsolete `ticket` / `ticket_id` keys; backfills
missing required fields (derived where possible, otherwise stamped `unknown`);
normalises PR links like `"PR #416"` to `"pr:416"`; and scopes itself to your
configured `paths.*` directories (skipping freeform directories like
`meta/docs/`). All non-trivial coercions are logged as `0007-DIVERGE[...]`
breadcrumbs; VCS revert remains the recovery path.
```

**Note**: the consolidated *Migration framework* bullet **replaces** — does not
supplement — the two detailed migrate bullets currently at `CHANGELOG.md:5-26`.

### Success Criteria:

#### Automated Verification:

- [x] Read-only CI mirror passes: `mise run check`
- [x] No version-coherence file changed (path-scoped, so unrelated
      working-copy state cannot mask it): `jj diff --stat
      .claude-plugin/plugin.json
      skills/visualisation/visualise/server/Cargo.toml
      skills/visualisation/visualise/bin/checksums.json` prints nothing
- [x] This phase's file did change: `jj diff --stat CHANGELOG.md` is non-empty
- [x] No `## [1.23.0]` heading or release date was added:
      `grep -n "## \[1.23.0\]" CHANGELOG.md` returns nothing

#### Manual Verification:

- [x] All new entries sit under `## [Unreleased]`, above the `## [1.22.0]`
      heading
- [x] The two detailed `/accelerator:migrate` bullets are gone, replaced by the
      single consolidated *Migration framework* line
- [x] Grouping (`Added` / `Changed` / `Fixed` / `Migrations`) matches the 1.22.0
      entry's Keep a Changelog conventions (acceptance criterion 5)
- [x] The 0007 callout reads as an upgrade alert modelled on the 1.22.0
      blockquote (acceptance criterion 3)
- [x] No developer-internal change appears (acceptance criterion 1)

---

## Phase 2: README skill catalogue refresh

### Overview

Surface `sync-work-items` and the `list-work-items` Sync column in the README
catalogue, cross-reference batch reconciliation from the Remote Work Item
Management intro, and note linked-worktree support in VCS Detection. The
Migrations section is intentionally untouched.

### Changes Required:

#### 1. Work Item Management — table + workflow diagram

**File**: `README.md`
**Changes**:

- Add a `sync-work-items` row to the skill table (`README.md:346-352`), after
  the `update-work-item` row (or grouped with the remote-facing skills — final
  placement at author's discretion during the edit, keeping column alignment):

```markdown
| **sync-work-items**    | `/accelerator:sync-work-items [--push-only\|--pull-only] [--preview] [--all] [filter-flags…]` | Reconcile local work items with the configured remote tracker (Jira or Linear), detecting per-item sync state and resolving conflicts |
```

  (The escaped `\|` is required so the table parser does not read it as a column
  delimiter; verify in rendered preview that it displays as `|`. The Usage matches
  the skill's argument-hint at `SKILL.md:7`.)

- Update the `list-work-items` row description (`README.md:350`) — replace the
  cell's Description verbatim with (consistent with the CHANGELOG framing):
  "List and filter work items by status, type, priority, tag, parent, or title;
  shows a colour-coded Sync column when a remote integration is configured".

- Add a `sync-work-items` node to the workflow diagram (`README.md:335-344`)
  **additively** — keep the existing `existing docs` source row, the
  `extract-work-items` inflow, and the `list-work-items` sub-tree (its
  `review-work-item` and `create-plan` branches) intact. `list-work-items` is
  re-parented under a new `├──`/`└──` sibling structure off `meta/work/` so a
  sibling `sync-work-items` branch can be added alongside it, with a *labelled*
  bidirectional edge to the remote tracker. Replace the current diagram block
  with exactly this (verified for monospace column alignment):

```
existing docs (specs, PRDs, notes)
       │
       ├── extract-work-items ──┐
       │                     ↓
       create-work-item ──→  meta/work/  ←── update-work-item
                              │
                              ├── list-work-items ──┬──→  review-work-item → meta/reviews/work/
                              │                     └──→  create-plan → implement-plan
                              └── sync-work-items ⇄ remote tracker (Jira/Linear)
```

  The `⇄` reads as bidirectional reconciliation and `sync-work-items` sits on its
  own line (not coupled to the `list-work-items` row), so the data flow is legible
  without decoding an unlabelled arrow.

#### 2. Remote Work Item Management — cross-reference

**File**: `README.md`
**Changes**: In the Remote Work Item Management intro (`README.md:361-371`), add
a sentence noting that `/accelerator:sync-work-items` performs batch
reconciliation of `meta/work/` against the tracker (building on the
`external_id`-as-synced-signal framing already present at `:368-370`).

#### 3. VCS Detection — linked-worktree note

**File**: `README.md`
**Changes**: In VCS Detection (`README.md:172-189`), add a sentence noting that
detection also recognises git **linked worktrees** (where `.git` is a file, not
a directory), so worktree-based sessions are detected like plain checkouts.

### Success Criteria:

#### Automated Verification:

- [x] Read-only CI mirror passes: `mise run check`
- [x] No version-coherence file changed (path-scoped): `jj diff --stat
      .claude-plugin/plugin.json
      skills/visualisation/visualise/server/Cargo.toml
      skills/visualisation/visualise/bin/checksums.json` prints nothing
- [x] This phase's file did change: `jj diff --stat README.md` is non-empty
- [x] The new table row keeps the markdown table well-formed (renders as a
      table in **preview** — `mise run check` does not parse markdown; the
      escaped pipe `\|` must display as `|`, not literally as `\|`)

#### Manual Verification:

- [x] `sync-work-items` appears in the Work Item Management table and the
      workflow diagram (acceptance criterion 4)
- [x] The `list-work-items` row mentions the Sync column (acceptance criterion 4)
- [x] The Remote Work Item Management intro cross-references batch sync
- [x] VCS Detection notes linked-worktree support
- [x] The Migrations section is unchanged
- [x] No mention of specific migrations or new migrate flags was added to the
      README (resolved decision)

---

## Testing Strategy

### Unit Tests:

- None applicable — this is prose-only. There is no markdown/prose linter in the
  task tree, and no test asserts CHANGELOG/README content.

### Integration Tests:

- `mise run check` (the read-only CI mirror) must stay green after each phase;
  for prose edits this is effectively a guard that nothing structural broke.

### Manual Testing Steps:

1. After Phase 1, render `CHANGELOG.md` and confirm the `[Unreleased]` section
   matches the 1.22.0 entry's grouping and contains all four new items + callout.
2. After Phase 2, render `README.md` and confirm the table/diagram/cross-ref/VCS
   edits and that the table still renders correctly.
3. Run `jj diff --stat` and confirm the documentation edits are confined to
   `CHANGELOG.md` and `README.md` and that no version-coherence file
   (`.claude-plugin/plugin.json`,
   `skills/visualisation/visualise/server/Cargo.toml`,
   `skills/visualisation/visualise/bin/checksums.json`) appears. Note: in a jj
   workspace the working copy may already carry unrelated changes, so judge by
   *which* files the doc edits touched, not by the total changed-file count.

## Performance Considerations

None — documentation-only change.

## Migration Notes

No code or data migration. The change *describes* a migration-framework fix
(migration 0007) but ships no new migration and changes no migration code.

## References

- Original work item: `meta/work/0123-changelog-readme-1-23-0-update.md`
- Driving research: `meta/research/codebase/2026-06-23-0123-changelog-readme-1.23.0-update.md`
- Prior-cycle precedent: `meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md`
- CHANGELOG model entry: `CHANGELOG.md:28-145` (1.22.0, incl. callout `:30-38`)
- README insertion points: `README.md:172-189` (VCS Detection),
  `README.md:319-359` (Work Item Management), `README.md:361-371` (Remote intro)
- New skill: `skills/work/sync-work-items/SKILL.md:1-10`
