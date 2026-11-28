---
type: "plan-validation"
id: "2026-11-24-PP-994-generic-set-model-indexing-and-api-validation"
title: "Validation Report: Generic Set Model: Indexing and API Implementation Plan"
date: "2026-11-27T09:48:59+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-24-PP-994-generic-set-model-indexing-and-api"
tags: ["visualiser", "server", "corpus", "set-model"]
last_updated: "2026-11-28T09:50:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Generic Set Model: Indexing and API

### Implementation Status

✓ Phase 1: Decision records - Fully implemented
✓ Phase 2: Refuse overlapping doc roots; fixtures on default paths - Fully implemented
✓ Phase 3: Doc types as declared data — identity and facts - Fully implemented
✓ Phase 4: Doc types as declared data — rules - Fully implemented
✓ Phase 5: The server takes an injected catalogue - Fully implemented
✓ Phase 6: Declarations crates and the assembled catalogue - Fully implemented
✓ Phase 7: Set declarations on the catalogue - Fully implemented
✓ Phase 8: The corpus resolver reads sets through their shape - Fully implemented
✓ Phase 9: Index consumers tell roots from members - Fully implemented
✓ Phase 10: Enumerate and index set members - Fully implemented
✓ Phase 11: The member API - Fully implemented
⚠️ Phase 12: Live update, activity roll-up and acceptance - Implemented; the
Linux run of the real-OS watcher lane is pending CI

### Automated Verification Results

✓ Full local CI mirror: `mise run` exited 0 in 730 s, and left the working
copy unchanged (no formatter or lint-fix edits).
✓ All four ADRs pass `accelerator corpus frontmatter validate`.
✓ `grep -n '^status: "accepted"'` over ADR-0077..0080 prints four lines.
✓ `rg -n "research_codebase: meta/research" cli/visualiser` prints nothing.
✓ Every test in `cli/visualiser/server/tests/api_sets.rs` is named `set_*`
(15 tests), so `--filter set_` selects them all.
⚠️ `mise run test:integration:visualiser-os-watcher` on Linux: not run. No
PP-994 pull request exists yet, so CI has not run it. The plan leaves this
box unticked.

### Code Review Findings

#### Matches Plan:

- ADR-0077, ADR-0078, ADR-0079 and ADR-0080 are accepted, and the member API
  spec is at `meta/specs/2026-11-24-set-member-api.md`.
- `cli/config/src/paths.rs:87-179` refuses overlapping doc roots.
  `ConfigError::OverlappingDocRoots` is a refusal. The visualiser repeats
  the check on canonical paths (`file_driver.rs:153-171`, `server.rs:86`).
- `DocType`, `DocTypeKey` and `DocTypeCatalogue` replace the closed enum.
  Characterisation tests pin the built-in facts and rules. The boundary
  test `doc_type_declarations_are_single_source.rs` guards the single
  source.
- Server production code branches on declared facts. Every
  `DocTypeKey::<NAME>` in `cli/visualiser/server/src` is inside a
  `#[cfg(test)]` module.
- `research-declarations`, `design-declarations` and the `declarations`
  assembly exist. The crate-dependency lint, pup rules and public-API
  pins cover them. Every composition root takes the assembled catalogue.
- Owners declare the two set declarations. The assembly pairs them, and
  `/api/types` carries each shape and presentation.
- The corpus resolver reads sets through `SetMatcher`, drops dot-prefixed
  and rootless directories, and has goldens.
- Index records carry set membership. `listed()` keeps the default view to
  roots. Search, related and clusters read members by name.
- `/api/sets/:type/:set_id` conforms to the spec. `setId` is compared only
  with indexed ids, so there is no path-traversal risk.
- The watcher debounces per set directory. One rescan produces one diff.
  Case-only renames pair within a set. The activity feed rolls members up
  into one row per set, and the removal ledger is in place.

#### Deviations from Plan:

Most deviations are recorded in the phases' implementation notes. These are
not recorded:

- The migrate-adapters overlap tests are in
  `cli/migrate-adapters/tests/context.rs:180,203`, not inline.
- The canonical overlap re-check lives in `file_driver.rs`, called from
  `server.rs`, not from `compose.rs`.
- `related.rs` reaches members through `declared_outbound` and the
  members-inclusive refs map, not `documents_including_members()`.
- `api/search.rs:65` calls `is_member()` directly.
- `SetContext` gained `root_rel_path` (`indexer.rs:208`) for Phase 12.
- The real-OS case-only rename step accepted any number of `Edited`
  events. The success criterion asks for exactly one. **Fixed**
  (`vxtwqqsk`): the step now expects exactly one.
- Phase 4's note that "nothing imports from `built_in`" was stale.
  `built_in` is now a `pub mod` that the assembly reads. **Corrected** in
  the plan.

The plan's "Validation follow-up" notes now record every deviation above.

#### Potential Issues:

1. **Sets that share an `id` are not all reachable through the member
   API.** `publish_document_ids` gives each such root its directory name as
   its library `id` (`indexer.rs:2405-2416`). `set_root` matches only
   `set_id` (`indexer.rs:1521-1535`). The library can show the second set,
   but `/api/sets` serves only the lexically first one. Member search rows
   link to the first set too. This follows the spec's "lexically first"
   rule, but the library and the member API use different keys.
   **Fixed** (`ntztvypt`): Each such set takes its directory name as its set
   `id`. The spec and CHANGELOG are updated.
2. **One bad set root can fail the whole rescan.** `set_directories_in`
   follows a symlinked root file without an allowed-roots check
   (`file_driver.rs:393`). A `PathEscape` from `read` then returns early
   from `build_records` (`indexer.rs:1023`). A member over
   `MAX_DOC_BYTES` fails the rescan the same way. This is plausible from
   the code, but not reproduced.
   **Fixed** (`wlrpqyno`): An unreadable root leaves out its own set, and an
   unreadable member leaves out only itself.
3. A file directly under a set doc root (for example `topics/stray.md`) is
   admitted as a set directory (`watcher.rs:96`, `within.is_empty()`). It
   causes a rescan that produces no event.
   **Fixed** (`vswnmumm`): Such a file causes no rescan.
4. A case-only rename of a set directory (`S/` to `s/`) is not paired. It
   gives a delete and a create for every member. No test covers it.
   **Fixed** (`ynxozkqk`): Every document and asset pairs as one rename.
5. `Indexer::lies_in_a_set_doc_root` checks only resolved roots
   (`indexer.rs:1195-1201`). The driver also checks configured roots. They
   can disagree for a root created after startup.
   **Fixed** (`pwwrszkz`): The indexer asks the file driver.
6. `set.directory` stays absolute when a doc root lies outside the project
   root (`indexer.rs:2346-2348`). The spec says it is project-relative.
   **Fixed** (`zovkopup`): The code matched `relPath`. The spec now says so, and
   a test pins it.
7. `api/sets.rs:103-124` takes separate read locks for the root and the
   members. A rescan between them can mix two snapshots.
   **Fixed** (`luxxwxqs`): `Indexer::set` reads the set from one snapshot.
8. `server.rs:93-98` picks the writable root by the `"work"` path key. It
   is a named doc type in production code, and probably predates this work.
   **Fixed** (`kxoqwusq`, `xkmxsokn`): Every `in_kanban` doc root is writable.
   The refusal is now `OnlyKanbanItemsAreWritable`, "only kanban items are
   writable".
9. The library response treats the first virtual type as templates
   (`api/library.rs:154-158`). That is a convention, not a declared fact.
   **Fixed** (`kyoptuqk`): The catalogue refuses a second virtual type and names
   its one virtual type.

Comment-policy findings, all **fixed** (`lrptzwzn`):

- `indexer.rs:861-866` names `rescan` as a consumer. That method no longer
  exists.
- `watcher.rs:33-34` and `:39-41`, `indexer.rs:650-651`,
  `file_driver.rs:140-141` and `frontend/src/api/types.ts:180-181` restate
  the code.
- `cli/corpus/src/doc_type/mod.rs:39-41,88-90` hold `//` comments in tests.
- `paths.rs:152-163` has two `# Errors` sections in one doc comment.

Test-name finding, **fixed** (`lrptzwzn`):

- `set_documents_and_assets_interleave_in_set_relative_path_order`
  (`api_sets.rs:471`) held no assets. It is now
  `set_members_come_in_set_relative_path_order`.

### Manual Testing Required:

The plan records every manual check as done. These remain open:

1. Linux:
  - [ ] `mise run test:integration:visualiser-os-watcher` passes in CI once
        the PP-994 pull request is raised.

2. Duplicate set ids:
  - [x] Two topic sets with the same frontmatter `id`: `/api/sets` must
        reach both. Decided 2026-11-27, and fixed.

### Follow-up, 2026-11-27

All issues above are fixed, each test first, in 19 commits,
`ntztvypt`..`nyyozwsu`:

- 11 commits fix the issues and the comment findings.
- `xkmxsokn` renames the refusal to `OnlyKanbanItemsAreWritable`,
  "only kanban items are writable", with a CHANGELOG entry.
- `slrqonxl`, `nvyulxts` and `svxlxwos` add tests for the eight
  acceptance criteria that were only partly tested: 8, 20, 21, 23, 27,
  32, 34 and 35. None of them found a bug.
- `nvyulxts` adds a scanner function for string literals. The boundary
  test now flags a set type's identity written as a string in the
  visualiser server.
- `kuuotntr` hands PP-1007 the 23 places in `config`, `corpus`,
  `launcher` and `research` that still name the set types as strings.
- The other commits update the plan, this report and the work item.

The author decided four questions: how shared set ids resolve, the scope
of the fixes, the refusal's wording, and the scope of criterion 21's
string scan. The plan's "Validation follow-up" records each one.

The full `mise run` exited 0 on the final code in 938 s and left the
working copy unchanged. The real-OS watcher lane passed on macOS. The
Linux run still waits for CI.

PP-994 is `in-progress`, with all 43 acceptance criteria ticked.

### Recommendations:

- Raise the pull request, and tick the Linux criterion when CI passes.
- Set PP-994 to `done` when the pull request merges.
