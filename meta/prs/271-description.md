---
type: "pr-description"
id: "271"
title: "[PP-994] Index directory doc types as sets and serve each set whole"
date: "2026-11-28T09:53:51+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-994"
parent: "work-item:PP-994"
relates_to: ["work-item:PP-996", "work-item:PP-1007", "work-item:PP-998", "work-item:PP-134"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/271"
pr_number: 271
tags: ["visualiser", "server", "corpus", "config", "set-model", "topic-research", "design-inventories", "doc-types"]
revision: "f04ade476a26635f537314e6f303596d4ed04613"
repository: "accelerator"
last_updated: "2026-11-28T09:53:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-994] Index directory doc types as sets and serve each set whole

## Summary

Topic research and design inventories store each entry as a directory: a
root document (`manifest.md`, `inventory.md`) with member documents and
assets beside it. Before this PR the visualiser indexed only the root, so a
finding, a report or a screenshot was invisible to search, live updates and
the API. The visualiser now indexes each such directory as a **set**: its
root plus exactly the members and assets its doc type's declared **shape**
names. The index's default views still hold one entry per set, a new member
API serves a set whole, and change events and the activity feed follow
members.

To get there without a `match` arm per doc type, doc types become
**declared data** in a catalogue. The two set types' shapes are declared by
the domains that write them, in new declarations crates, and one assembly
builds the catalogue every composition root takes. The corpus resolver
reads sets through the same shapes, and overlapping doc roots are now
refused, because a set root inside another doc root would be indexed twice.

## Changes

- **Decision records.** ADR-0077 (the generic set model), ADR-0078 (doc
  types as declared data), ADR-0079 (doc roots may not overlap) and
  ADR-0080 (domain declarations crates) are accepted. The member API is
  specified in `meta/specs/2026-11-24-set-member-api.md`.
- **Breaking: overlapping doc roots are refused.** `config` refuses a
  configuration in which one `paths.*` doc root equals or contains another,
  naming both keys. `config paths --doc-types`, `config path` for a doc
  type's key, and the `corpus`, `work` and `research` commands refuse it;
  other keys still resolve and `config set` still writes, so the overlap
  can be fixed. `migrate` and `migrate --list` stop before applying or
  listing anything, and a migration that writes such a configuration is
  not recorded as applied. The visualiser repeats the check after following
  symlinks.
- **Doc types as declared data (`corpus`).** `DocType` declares a type's
  identities, facts and rules: slug, cluster key, completeness mark,
  linkage id, lifecycle rank, library placement, facets and listing.
  `DocTypeKey` is a `Copy` handle to one. `DocTypeCatalogue` refuses two
  declarations sharing a wire token, linkage name or config key, and
  refuses a second virtual type. Corpus consumers read the declared rules.
  Characterisation tests pin every built-in fact and rule, and a boundary
  test fails if production code names a built-in doc type outside its
  declarations.
- **Set model (`corpus::set`).** `SetShape` names a root and member slots
  (`docs/*.md`, `screenshots/*.png`, …), each holding documents or assets.
  `SetMatcher` places a path in a shape, refusing dot-prefixed names and
  comparing extensions case-insensitively. `Presentation` says whether a
  set opens as a set page or its entry point.
  `pair_case_only_renames` pairs a removed and an added path that differ
  only in case, in the set-relative path or the set directory.
- **Declarations crates.** `research-declarations` declares the topic
  research set and `design-declarations` the design inventory set, beside
  their layout constants. The `declarations` assembly pairs each set type
  with its set declaration, and the corpus CLI, research CLI, migrate CLI
  and visualiser server take the catalogue from it. The crate-dependency
  lint gains the declarations and declarations-assembly roles and four
  rules, with matching pup rules and public-API snapshots.
- **Breaking: the corpus resolver reads sets through their shape.** For a
  set type, a slug resolves only a non-dot directory holding the set's
  root, and a path resolves to its set only when it is the set directory,
  its root, a member or a directory that may hold members. Anything else
  exits 3. `accelerator research topic` resolves sets the same way. The
  `inventory-design` and `research-topic` skills say so.
- **Visualiser: the catalogue is injected.** `AppState` takes the
  assembled catalogue, and server code branches on declared facts only.
  Its writable roots are the doc roots of every `in_kanban` doc type, and
  a refused frontmatter write now reads "only kanban items are writable".
- **Visualiser: members are indexed.** `FileDriver::list_sets` walks each
  set through its shape. Records carry their set membership, and the
  index's default views (listings, counts, facets, clusters, lifecycle)
  hold roots only. Search, related items and the latest-change time read
  members by name. An unreadable root leaves its own set out, and an
  unreadable member leaves out only itself.
- **Visualiser: the document endpoint.** `/api/docs/{path}` serves a set's
  root and member documents and returns 404 for anything else in a set
  directory, for a file in a rootless directory, and for any directory,
  which used to fail with 500.
- **Visualiser: the member API.** `GET /api/sets/{type}/{setId}` returns
  the set's summary, its root and every member in set-relative path order,
  read from one snapshot of the index. Documents carry frontmatter and
  etag but no body; assets carry size and modification time. When two
  sets share an `id`, each takes its directory's name as its `setId`, the
  `id` the library already links it by. `/api/types` carries each set
  type's shape and presentation.
- **Visualiser: live updates.** The watcher admits a path by its set's
  shape and debounces on the set directory, so a burst inside one set
  costs one rescan. Each rescan computes one diff, emits one event per
  changed member or asset, and reads a set renamed into place, or any
  case-only rename, as one change. A set's changes share one activity row,
  for its root, and the feed now shows the rows the server recorded. A
  removed member dates its set from when it was seen gone.
- **Frontend.** The API types and normalisation carry `setId` and
  `setRelativePath` on entries and search rows, the set view on doc types,
  and the member API's response.
- **Fixtures.** The flat codebase research fixtures moved to
  `meta/research/codebase/`, so the API smoke test and the e2e server run
  with no path override. Every topic manifest and inventory fixture has a
  quoted `id`.

## Context

- Work item: `meta/work/PP-994-generic-set-model-indexing-and-api.md`
  (all 43 acceptance criteria ticked; status `in-progress`)
- Research:
  `meta/research/codebase/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`
- Plan: `meta/plans/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`
- Plan review:
  `meta/reviews/plans/2026-11-24-PP-994-generic-set-model-indexing-and-api-review-1.md`
- Validation:
  `meta/validations/2026-11-24-PP-994-generic-set-model-indexing-and-api-validation.md`
  (result: pass). Every issue it found is fixed, each test first; the
  plan's "Validation follow-up" records them and the author's decisions.
- Decisions: ADR-0077, ADR-0078, ADR-0079, ADR-0080; spec
  `meta/specs/2026-11-24-set-member-api.md`.
- Stacked on #265 (PP-996), which is stacked on #262, #261 and #268.
- Follow-up: PP-1007 moves whole doc type declarations to their owners,
  including the 23 places that still name the set types as strings.
  PP-998, PP-134, PP-1000, PP-1001 and PP-1002 build on the set model.

## Testing

- [x] `mise run` exits 0 end to end (938 s) at `360d57b8`: 2783 frontend
      unit tests, 474 and 478 visualiser unit tests, 208 visualiser
      integration tests, 8644 cli tests and 361 e2e tests. The commits
      after it change only `meta/` records and wrap one CHANGELOG line.
- [x] Every acceptance criterion was checked against the test that shows
      it. Eight were only partly tested (8, 20, 21, 23, 27, 32, 34, 35),
      and new tests now cover them in full; none found a bug. A test
      whose first version passed was also broken on purpose to see it
      fail.
- [x] The real-OS watcher lane (`mise run
      test:integration:visualiser-os-watcher`) passes on macOS: a write, a
      directory renamed into place, and a case-only rename that gives
      exactly one `edited` event.
- [ ] The same lane on Linux, where it runs in the ordinary unit lane: CI
      on this PR.
- [x] Manual checks, recorded per phase in the plan: overlap refusals and
      their fixes in a scratch repo; `corpus resolve` against this
      repository's six real inventories; `/api/types`, `/api/docs` and
      `/api/sets` against this repository; indexing time with every real
      inventory; and live activity rows under `mise run dev:up` for a new
      topic set, an edited finding, an inventory copied in and its root
      deleted.
- [x] Edge cases covered by tests:
  - dot-prefixed files and directories, rootless directories and files no
    slot names, in the index, search, events and `/api/docs`
  - malformed and absent frontmatter on roots and members
  - a percent-encoded `setId` decoded exactly once
  - sets sharing an `id`, reached by their directory names
  - a root that escapes the doc roots, and a member too large to read
  - a set directory or screenshot renamed only in case
  - a set doc root that stops being a symlink after start-up
  - a set directory outside the project, served as an absolute path
  - a declared set type only the tests know, end to end

## Notes for Reviewers

- **Three breaking changes** are in the CHANGELOG's `### Breaking`:
  overlapping doc roots are refused, `migrate` refuses them before
  applying anything, and `corpus resolve` resolves a set only through its
  shape. A repository with `research_codebase: meta/research` must move it,
  for example to `meta/research/codebase`.
- **Two API responses change.** Sets that share an `id` are reached by
  their directory names, not by the shared `id`; the spec says so. A
  refused frontmatter write reads "only kanban items are writable".
- **A suggested reading order:** the ADRs and the spec;
  `cli/corpus/src/doc_type/` and `cli/corpus/src/set/`; `cli/declarations/`; then the server's
  `indexer.rs` (records, `build_records`, `IndexDiff`), `watcher.rs`
  (admission and debounce), `api/sets.rs` and `activity_feed.rs`.
- **Strings still name the set types outside the server.** The boundary
  test scans the whole `cli/` workspace for `DocTypeKey` constants, but
  for identities written as strings it scans the visualiser server only.
  Eight files in `config`, `corpus`, `launcher` and `research` still name
  topic research or design inventories as strings, mostly in tables that
  list every doc type. PP-1007 moves them.
- **A case-sensitive volume pairs two different files.** Deleting `a.png`
  and creating `A.png` in one rescan reads as one rename there. The plan
  accepts this.
- **Changes beyond the story**, each test-first in its own commit:
  - Lifecycle order and related items no longer depend on hash-map or
    arrival order when slugs collide.
  - Lifecycle rows are keyed by their cluster's lowest member path, which
    removes React's duplicate-key warning.
  - The GitHub client uses the bundled Mozilla trust roots, because
    reading the macOS certificate store timed out under the test lane's
    load.
- **The PR is large:** 119 commits and 199 files, about 20,600 lines added.
  The plan's twelve phases map onto the commits in order.
