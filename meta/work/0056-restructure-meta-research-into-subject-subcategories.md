---
work_item_id: "0056"
title: "Restructure meta/research/ into Subject Subcategories"
date: "2026-05-11T16:39:44+00:00"
author: Toby Clemson
type: story
status: ready
priority: high
parent: ""
tags: [meta, paths, migration, research, configuration]
---

# 0056: Restructure meta/research/ into Subject Subcategories

**Type**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As an Accelerator plugin author, I want `meta/research/` subcategorised by
subject — codebase, issues, design-inventories, design-gaps — so that
research artifacts of different shapes have predictable homes, the
subcategory pattern is established for the forthcoming idea/concept
research skill to extend without re-litigating structure, and
design-convergence outputs (the `design-inventories/` and `design-gaps/`
artifacts produced by the design-convergence workflow) are reunified
under the research umbrella where they conceptually belong.

## Context

`meta/research/` is currently a flat directory holding ~36 single-document
artifacts that mix codebase analyses, issue research, strategy notes, and
historical design work. `meta/design-inventories/` and `meta/design-gaps/`
sit at the top of `meta/` as a deliberate, temporary deferral from the
design-convergence workflow rollout — they were kept out of `meta/research/`
because the flat layout would have been the worst of both worlds.

The forcing function is a forthcoming idea/concept research skill that will
produce multi-document research per topic. Shipping it into the current flat
layout would either further overload `meta/research/`'s mixed bag or commit
to a structural decision under time pressure. This work resolves the
structural question first so the skill can land cleanly.

The note at `meta/notes/2026-05-02-research-directory-subcategory-restructure.md`
captures the longer history and the original deferral reasoning. Work items
`0030` (centralise path defaults) and `0052` (documents-locator config-driven
paths) have already put the plumbing in place for path keys to be added,
renamed, and consumed by agents via `accelerator:paths`.

## Requirements

### Directory restructure

- Create `meta/research/codebase/`, `meta/research/issues/`,
  `meta/research/design-inventories/`, and `meta/research/design-gaps/`.
- Move existing flat `meta/research/*.md` wholesale into
  `meta/research/codebase/`.
- Move `meta/design-inventories/*` into `meta/research/design-inventories/`.
- Move `meta/design-gaps/*` into `meta/research/design-gaps/`.
- Remove the now-empty top-level `meta/design-inventories/` and
  `meta/design-gaps/`.
- Preserve VCS rename history on all moves (use the `_move_if_pending`
  helper pattern from
  `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:32-64`,
  not delete + add — jj's rename tracking relies on the same operation
  primitives as git's content-similarity detection).

### Per-category file-vs-directory shape policy

- `codebase/` — single file per investigation.
- `issues/` — single file per investigation.
- `design-inventories/` — directory per investigation (existing pattern with
  screenshots).
- `design-gaps/` — single file per investigation.

### Configuration changes

- Rename `paths.research` → `paths.research_codebase` (preserving any
  user-customised value).
- Rename `paths.design_inventories` → `paths.research_design_inventories`.
- Rename `paths.design_gaps` → `paths.research_design_gaps`.
- Add `paths.research_issues`.
- Update `scripts/config-defaults.sh` and any configure templates to reflect
  the new key set.

### Migration via `accelerator:migrate`

- Add a migration that performs:
  - Filesystem moves for all three directory groups (flat research, design
    inventories, design gaps) with VCS rename history preserved.
  - Removal of obsolete top-level `meta/design-inventories/` and
    `meta/design-gaps/`.
  - Config key rewrites for the three renamed keys, preserving any
    user-customised values (non-interactive rewrite (no prompt) with a
    per-key one-line stdout notification).
  - Addition of the new key `paths.research_issues` with its default
    value.
  - Rewriting of inbound `meta/research/*.md`, `meta/design-inventories/*`,
    and `meta/design-gaps/*` references inside `meta/**/*.md` files to point
    at the new subcategory paths.
- The migration must be idempotent and inherit the framework's
  dirty-working-tree guard.

### Skill updates

- `research-codebase` writes output to `paths.research_codebase`.
- `research-issue` writes output to `paths.research_issues`.
- `accelerator:paths` surfaces all four new keys via its preloaded path
  discovery.

### Agent update

- `documents-locator` replaces its inline directory map with output sourced
  from `accelerator:paths`.
- `documents-locator` output format surfaces subcategories (e.g.
  "Research (codebase)", "Research (issues)") instead of a flat "Research"
  group.

### Documentation

- README's `meta/` directory table updated to list the new subcategories
  with their purposes.
- `accelerator:configure` paths-table help updated to list the new key set.

### Out of scope

- Subcategorising any other `meta/` directory (`notes/`, `work/`, `plans/`,
  `specs/`, etc.).
- Splitting top-level `research/` and `analysis/`.
- Introducing a `meta/strategy/` home for external research.
- Building the forthcoming idea/concept research skill. The
  `paths.research_ideas` key and `meta/research/ideas/` directory are
  also out of scope and will land with that skill's own work item — this
  work establishes the subcategory pattern so the skill can extend it
  without re-litigating structure.
- Per-investigation directory promotion (a file growing into a directory
  mid-life).

## Acceptance Criteria

- [ ] Given a fresh repo, when I run `accelerator:configure`, then I see
  `paths.research_codebase`, `paths.research_issues`,
  `paths.research_design_inventories`, and `paths.research_design_gaps`,
  and no `paths.research`, `paths.design_inventories`, or
  `paths.design_gaps`.
- [ ] Given a userspace repo on the legacy layout, when I run
  `accelerator:migrate`, then all flat-laid `meta/research/*.md` files move
  to `meta/research/codebase/`, `meta/design-inventories/*` moves to
  `meta/research/design-inventories/`, `meta/design-gaps/*` moves to
  `meta/research/design-gaps/`, and the obsolete top-level directories are
  removed.
- [ ] Given a userspace repo where `paths.research`,
  `paths.design_inventories`, or `paths.design_gaps` is set in
  `.accelerator/config.md` (whether at the legacy default value or an
  override), when migration runs, then the legacy key is removed and the
  new key is written with the same value (verbatim preservation), and
  the migration emits one line per renamed key to stdout in the form
  `Renamed paths.<old> → paths.<new> (value preserved: <path>)`.
- [ ] Given a userspace repo, when migration runs, then its inbound-link
  scan corpus is the union of markdown files inside every path surfaced
  by `accelerator:paths` (which may itself be user-customised) rather
  than a hardcoded `meta/**/*.md` glob.
- [ ] Given a markdown file inside the scan corpus containing references
  to the configured legacy paths (`paths.research`,
  `paths.design_inventories`, `paths.design_gaps` — each resolved from
  the user's config via `scripts/config-read-path.sh`, which may differ
  from the defaults), when migration runs, then every matching reference
  is rewritten to the corresponding configured new subcategory path
  (`paths.research_codebase`, `paths.research_design_inventories`,
  `paths.research_design_gaps` — likewise config-resolved), and
  non-matching content in the same file is byte-identical to the
  pre-migration state.
- [ ] Given a fixture corpus shipped alongside this migration (in a
  `fixtures/` directory sibling to the migration script) that contains,
  for each of the three renamed keys, at least one matching example and
  one non-matching example of every reference shape — markdown links
  `[label](path)`, frontmatter scalar values, frontmatter list entries
  in both inline and multi-line YAML forms, and prose path mentions —
  when migration runs, then each matching reference is rewritten to the
  corresponding new key's path and each non-matching string (paths under
  unrelated keys, path-like substrings that aren't real path references)
  is byte-identical to its pre-migration state.
- [ ] Given `accelerator:migrate` is re-run after a successful migration,
  when it executes, then it exits with code 0, prints no per-migration
  preview line for this migration (the migration is recorded as applied
  in `.accelerator/state/migrations-applied`), and makes no filesystem
  or config changes.
- [ ] Given a dirty working tree (uncommitted changes under `meta/` or
  `.accelerator/`), when `accelerator:migrate` is invoked, then it exits
  non-zero, prints a message naming the dirty paths, and makes no
  filesystem or config changes.
- [ ] Given the `documents-locator` agent is invoked in a migrated repo,
  when it reports research artifacts, then the output groups research
  results under subcategory labels matching the keys surfaced by
  `accelerator:paths` (e.g. "Research (codebase)", "Research (issues)",
  "Research (design-inventories)", "Research (design-gaps)") rather
  than a flat "Research" group.
- [ ] Given the `documents-locator` agent definition is inspected, then
  it contains no inline directory map hardcoding `meta/research/` or
  similar paths, and its frontmatter declares `skills: [accelerator:paths]`
  for path discovery.
- [ ] Given `research-codebase` runs in a migrated repo, when it writes an
  artifact, then the artifact lands in `meta/research/codebase/`.
- [ ] Given `research-issue` runs in a migrated repo, when it writes an
  artifact, then the artifact lands in `meta/research/issues/`.
- [ ] Given a developer reads the README's `meta/` table, when they look for
  research artifacts, then all four subcategories are listed with their
  purposes.
- [ ] Given a fresh repo, when I run `accelerator:configure`, then
  `paths.research_codebase` defaults to `meta/research/codebase`,
  `paths.research_issues` to `meta/research/issues`,
  `paths.research_design_inventories` to `meta/research/design-inventories`,
  and `paths.research_design_gaps` to `meta/research/design-gaps`.
- [ ] Given the migration has run, when `jj log --follow <new-path>` (or
  `git log --follow`) is invoked on any migrated file, then the history
  shows the file's pre-migration commits, confirming move-semantics
  preservation rather than delete+add.
- [ ] Given `accelerator:paths` is invoked in a migrated repo, when its
  output is rendered, then the **Configured Paths** block lists all four
  new keys (`research_codebase`, `research_issues`,
  `research_design_inventories`, `research_design_gaps`) and contains no
  entries for the legacy `research`, `design_inventories`, or
  `design_gaps` keys.
- [ ] Given `accelerator:configure help` is invoked in a migrated repo,
  when the paths-table reference is rendered, then it lists all four new
  keys with their default values and one-line descriptions, and contains
  no rows for the legacy keys.
- [ ] Given the plugin commit that ships this work, when its `meta/`
  tree is inspected, then `meta/research/codebase/`,
  `meta/research/design-inventories/`, and `meta/research/design-gaps/`
  contain the previously-flat or previously-top-level content, and no
  legacy top-level `meta/design-inventories/` or `meta/design-gaps/`
  directories remain.
- [ ] Given `scripts/research-metadata.sh` is inspected after the work
  lands, then it contains no hardcoded `meta/research` literal that
  would bypass the configured path resolution.
- [ ] Given a developer reads the README, when they look for references
  to `meta/research/`, `meta/design-inventories/`, or `meta/design-gaps/`,
  then all narrative prose, tables, and ASCII diagrams reference the new
  subcategory paths.
- [ ] Given a userspace repo where none of `paths.research`,
  `paths.design_inventories`, or `paths.design_gaps` is set in
  `.accelerator/config.md` (relying on defaults), when migration runs,
  then no rename notification is printed for absent keys, and the new
  keys (`paths.research_codebase`, `paths.research_design_inventories`,
  `paths.research_design_gaps`) resolve to their new default values via
  `config-defaults.sh`.

## Open Questions

- None at this time.

## Dependencies

- Blocked by: none.
- Blocks:
  - Forthcoming idea/concept research skill (work item TBD — when drafted,
    its `Blocked by` should reference 0056).
- Builds on (code, not work items):
  - `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
    (config-key rewrite pattern)
  - `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh`
    (three-phase inbound-link rewriter pattern)
  - `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh`
    (idempotent filesystem-move helper pattern)
- Ordering: The in-tree plugin restructure (directory moves, config-key
  rename/additions, skill/agent updates, migration code) must all land
  in a single atomic commit. The plugin's in-tree restructure is a
  prerequisite — not a parallel track — to the userspace migration
  shipping, because the migration is modelled on the new plugin layout.
- Related: 0021, 0022 (artifact persistence/metadata — conceptually
  adjacent, not technically blocking).

## Assumptions

- The existing `accelerator:migrate` framework can express filesystem
  moves, config-key renames, and config-resolved inbound-link rewriting.
  If a new primitive proves necessary during implementation, it ships
  as part of this work item rather than as a separate prerequisite.
- A previously-customised `paths.research`, `paths.design_inventories`, or
  `paths.design_gaps` value in userspace config maps cleanly to the renamed
  key, preserving the user's path. Silent rewrite with notification was
  chosen over interactive prompt or fail-loud.
- VCS rename detection (jj/git) will preserve history across the moves
  provided the migration uses move semantics rather than delete + add.

## Technical Notes

**Size**: L — ~38 file moves in `meta/research/` plus design-inventories
and design-gaps moves; in-tree plugin application plus the userspace
migration; lockstep edits to 9+ files (`config-defaults.sh`, `init.sh`,
`config-read-all-paths.sh`, `paths/SKILL.md`, `research-codebase/SKILL.md`,
`research-issue/SKILL.md`, `documents-locator.md`, `configure/SKILL.md`,
`README.md`) plus the shared `scripts/research-metadata.sh`; a brand-new
migration combining all three patterns from existing migrations 0001
(config-key rewrite), 0002 (three-phase inbound-link rewrite), and 0003
(idempotent filesystem moves) — no existing migration combines them —
with the inbound-link rewriter further generalised to config-resolve
both scan corpus and old/new path values.

- `accelerator:migrate` already guards against dirty working trees and
  prints per-migration previews — leverage these.
- Inbound-link rewriting must derive both its scan corpus and the
  old/new path values from the config layer rather than hardcoding
  `meta/...` literals: scan markdown files inside every path surfaced
  by `accelerator:paths` (which may itself be user-customised), and for
  each file rewrite references matching the user's resolved
  `paths.research` / `paths.design_inventories` / `paths.design_gaps`
  values to the resolved `paths.research_codebase` /
  `paths.research_design_inventories` / `paths.research_design_gaps`
  values. Rewriting covers markdown links `[label](path)`, frontmatter
  scalars and list entries, and prose path mentions.
- `research-codebase` and `research-issue` currently resolve output paths
  via the centralised path defaults from work item 0030. Both skills'
  `SKILL.md` and any path-resolution scripts need updating in lockstep with
  the config key renames.
- `documents-locator` currently has an inline directory map per work item
  0052; that map should be replaced with output sourced from
  `accelerator:paths`.
- The plugin repo's own `meta/` is reorganised by hand in the same
  commit that adds the migration script — the script itself is only
  executed by users via `accelerator:migrate` against their userspace
  repos. The plugin's in-tree layout is the source the userspace
  migration is modelled on, so it must reach its target shape in the
  same atomic commit that ships the migration.
- `scripts/config-defaults.sh:26-62` defines `paths.*` via index-paired
  `PATH_KEYS` and `PATH_DEFAULTS` arrays — additions and renames must keep
  both arrays in lockstep. The three renames are in-place edits to
  `PATH_KEYS` entries (with default values shifted to point at the new
  subdirectories); the one addition (`paths.research_issues`) is an
  appended row in both arrays.
- A parallel `DIR_KEYS`/`DIR_DEFAULTS` array pair lives in
  `skills/config/init/scripts/init.sh:18-31` as a known deferred duplicate
  (flagged in 0030's Open Questions). It must be updated in lockstep with
  `config-defaults.sh` or fresh-init repos will scaffold the wrong
  directories.
- `scripts/config-read-all-paths.sh:14` carries
  `EXCLUDED_KEYS=(tmp templates integrations design_inventories design_gaps)`.
  Once the design keys are renamed to `research_design_*`, they must also
  be removed from this exclusion list so the `accelerator:paths` block
  surfaces them.
- `skills/config/paths/SKILL.md:27-37` carries a hand-written "Path
  legend" that is *not* derived from `PATH_KEYS`. It needs hand-editing
  to drop the legacy `research` entry and add the four new ones with
  one-line descriptions.
- Three exemplar migrations cover the patterns 0056's migration must
  combine: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
  (config-key rewrite via sed handling both nested-YAML and flat-dotted
  forms, lines 111-134); `0002-rename-work-items-with-project-prefix.sh`
  (inbound-link rewriter over `meta/**/*.md` covering frontmatter
  scalars/lists, markdown links, and prose, lines 113-313); and
  `0003-relocate-accelerator-state.sh` (idempotent `_move_if_pending`
  filesystem-move helper, lines 32-64). No existing migration combines
  all three patterns; 0056's migration will be the most complex single
  migration shipped so far.
- The migrate framework explicitly forbids per-migration dry-run
  behaviour (`skills/config/migrate/SKILL.md:41`: "MUST NOT honour any
  `DRY_RUN` env var"); recovery is VCS revert.
- `documents-locator` already preloads `accelerator:paths` (per 0052,
  frontmatter `skills: [accelerator:paths]` at line 10-11). What remains
  in scope for 0056 is the inline key-to-document-type legend at
  `agents/documents-locator.md:33-41` (overlaps with the paths-skill
  legend), the hardcoded output group names at lines 71-96, and the
  "Historic intent" purpose hints at lines 110-117 that reference
  `research` as a single key.
- `accelerator:configure` paths reference is hand-written markdown with
  two mirrored blocks: the table at `skills/config/configure/SKILL.md:386-402`
  and the example YAML at lines 406-422. Both must be edited.
- `research-codebase` resolves its output path via a single bang call at
  `skills/research/research-codebase/SKILL.md:23` (`config-read-path.sh research`);
  `research-issue` does the same at line 22. Each is a one-line edit to
  swap the bare key name. The two skills share `scripts/research-metadata.sh`
  — that script should be inspected for any hardcoded `meta/research`
  literal that bypasses the bang call.
- README `meta/` table at `README.md:84-95` lists `research/`,
  `design-inventories/`, and `design-gaps/` rows that need replacing.
  Additionally, narrative prose at lines 42, 47, 64, 409, and 413
  references `meta/research/` literally — including the ASCII flow
  diagram at lines 408-415 — and must be revised in lockstep.

## Drafting Notes

- Type chosen as `story` rather than `epic` because the parts must ship
  together — partial migration leaves userspace repos in a broken state.
- Per-category file-vs-directory policy decided at proposal time:
  `codebase/`, `issues/`, and `design-gaps/` are single-file;
  `design-inventories/` is directory-per-investigation.
- Path keys mirror directory names exactly: plural where the directory is
  plural (`paths.research_issues` → `meta/research/issues/`), naturally
  singular where the directory is naturally singular
  (`paths.research_codebase` → `meta/research/codebase/`).
- External-research / strategy directory placed in Out of Scope — deferred
  to a future `meta/strategy/` home, surfaced by the eventual idea/concept
  skill or a separate forcing function.
- Forthcoming idea/concept research skill is *not* in scope; its
  directory and path key (`paths.research_ideas`,
  `meta/research/ideas/`) will land with its own work item, extending
  the subcategory pattern established here.
- `documents-locator` updates routed via `accelerator:paths` rather than
  inline directory-map edits, matching the direction established by 0052.
- Existing flat `meta/research/*.md` files are dropped wholesale into
  `meta/research/codebase/` rather than hand-categorised, accepting some
  legacy strategy/idea notes will live under `codebase/` until anyone cares
  to re-categorise.

## References

- Source note: `meta/notes/2026-05-02-research-directory-subcategory-restructure.md`
- `meta/research/2026-05-02-design-convergence-workflow.md` — §9.8 discusses
  why nested layout was deferred at the time.
- `meta/work/0030-centralise-path-defaults.md` — path defaults centralisation
  this work builds on.
- `meta/work/0052-make-documents-locator-paths-config-driven.md` —
  documents-locator config-driven paths this work builds on.
- `meta/work/0027-ephemeral-file-separation-via-paths-tmp.md` — pattern
  reference for adding new `paths.*` keys.
- `agents/documents-locator.md` — agent whose directory map and output
  format must change.
- `skills/research/research-codebase/SKILL.md` — skill whose output path
  must change.
- `skills/research/research-issue/SKILL.md` — skill whose output path must
  change.
- `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md` —
  related concern about agent path-awareness.
