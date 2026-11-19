---
type: "codebase-research"
id: "2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field"
title: "Work Item Writers Omit the Required Tags Field"
date: "2026-11-18T18:05:26+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0333"
parent: "work-item:PP-886"
topic: "Work Item Writers Omit the Required Tags Field"
tags: ["research", "codebase", "work", "work-cli", "migrate", "corpus", "frontmatter"]
revision: "bc6a2d29ee89cce2e19ebc3a9a4ae74848055d91"
repository: "accelerator"
last_updated: "2026-11-18T18:05:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Work Item Writers Omit the Required Tags Field

**Date**: 2026-11-18T18:05:26+00:00
**Author**: Toby Clemson
**Git Commit**: bc6a2d29ee89cce2e19ebc3a9a4ae74848055d91
**Branch**: none (jj working copy on top of `0281-corpus-ask-and-report`)
**Repository**: accelerator

## Research Question

For work item 0333: where and why do `accelerator work create`,
`work create-batch` and the sync importer drop `tags` from a new work item's
frontmatter, what has to change to always write it, and how should a
byte-preserving backfill migration be built and registered?

## Summary

- **One line causes the defect, and all three paths share it.**
  `compose_frontmatter` emits `tags` through `push_sequence_if_non_empty`
  (`cli/work/src/create.rs:154`). Its only production caller is
  `render_frontmatter` (`cli/work-cli/src/create.rs:239-247`), and that has
  two production callers:
  - `CreationContext::content`, which serves `work create` and
    `create-batch`;
  - `author_from_remote` in `sync_author.rs`, which always passes `tags: &[]`.
- **The fix needs no reordering.** `tags` is already pushed directly before
  `last_updated` (lines 154 → 156). The template order is
  `external_id` → `aliases` → `tags` → `last_updated`, and since `aliases` is
  never emitted, the output already matches it. An empty
  `FieldValue::Sequence` renders as flow `[]` through `serde_saphyr::FlowSeq`,
  but no test pins that literal yet.
- **The decisions agree with the work item.**
  - ADR-0033 makes `tags` a mandatory base field that may be empty.
  - ADR-0040 explicitly keeps `tags` out of the omit-when-empty set.
  - The misclassification began in the 0170 plan.
  - The validator agrees as well: `tags` is in `BASE_FIELDS`, and it is the
    only key exempt from `EMPTY-PLACEHOLDER`.
- **Existing tests:** exactly one breaks. The `compose_frontmatter` unit test
  `omits_empty_optional_fields` asserts the exact key list without `tags`. One
  CLI test name goes stale. No create-batch or sync test currently asserts
  frontmatter shape.
- **The migration** would be `0012-…` in `cli/migrate/src/migrations/m0012.rs`,
  a `Mechanical` entry appended to the registry. Building it involves:
  - Detecting presence with `corpus::frontmatter_validation::parse_entries` +
    `is_present`, over the fenced frontmatter only. This counts flow, empty
    and block-style `tags` as present.
  - Inserting with a byte-offset splice over `split_inclusive('\n')`, not
    `lines()`/`join`, which drop `\r`.
  - Registering it in `mod.rs`, `registry.rs`, the public-API fixture,
    `CHANGELOG.md`, and roughly 20 migrate-cli test ledgers that list every
    migration ID.
- **Doc fixes:**
  - The work item's scope covers the `create-work-item` SKILL.md and the
    `compose_frontmatter` doc comment.
  - Not in its scope, but also wrong: `extract-work-items` SKILL.md:598-600
    groups `tags` with the omit-when-empty fields.

## Detailed Findings

### The writer: `compose_frontmatter`

- Signature: `compose_frontmatter(inputs: &CreateInputs<'_>) ->
  Vec<(String, FieldValue)>` (`cli/work/src/create.rs:84-87`).
- Emission order:
  1. fixed scalars `type`…`priority` (88-122);
  2. `parent` if `Some` (124-127);
  3. `blocks`, `blocked_by`, `derived_from`, `relates_to` via
     `push_sequence_if_non_empty` (128-143);
  4. `source` if `Some` (144-147);
  5. `external_id` if `Some` (148-153);
  6. **`tags` via `push_sequence_if_non_empty` (154)**;
  7. `last_updated` (156-159), `last_updated_by` (160-163),
     `schema_version: 1` (164).
- `push_sequence_if_non_empty` (169-177) is the only sequence helper. No
  always-emit sibling exists.
- The fix is either `fields.push(("tags".to_owned(),
  FieldValue::Sequence(inputs.tags.to_vec())))` or a small,
  intention-revealing helper.
- The doc comment at 74-83 says "the six typed-linkage slots (present-only
  for `parent`/`source`, non-empty-only for
  `blocks`/`blocked_by`/`derived_from`/`relates_to`/`tags`)". The six slots
  are counted correctly; `tags` is the extra entry that must move out of the
  non-empty-only list.
- `CreateInputs.tags: &'a [String]` (line 29). `CreateInputs`,
  `TypedLinkage` and `compose_frontmatter` are public API
  (`cli/work/tests/fixtures/public-api.txt:115`). A private helper would not
  change that fixture.
- `KNOWN_FRONTMATTER_KEYS` (50-72) already includes `tags` (line 68).
  - The template drift guard `assert_matches_template_schema` (223-252)
    compares key sets, not emission. That is why it never caught the omission.
  - `aliases` is listed there but `compose_frontmatter` never emits it.

### Rendering: `render_frontmatter` and the document crate

- `render_frontmatter` (`cli/work-cli/src/create.rs:239-247`) adds no fields.
  It appends each composed field into a `document::Mapping`, whose `push`
  keeps order (`cli/document/src/value.rs:49-51`), then calls
  `document::render(None, …)`.
- `FieldValue::Sequence` → `Yaml::Sequence` (`field_to_yaml`, 157-168) →
  `serde_saphyr::FlowSeq` (`cli/document/src/value.rs:177-179`). Items are
  `DoubleQuoted`, giving `tags: ["api"]`.
- For an empty `Vec`, serde-saphyr 0.0.29's flow branch writes `[` … `]`
  (`ser/serializer/compound.rs`), so the expected output is `tags: []`.
- No workspace test asserts the literal `tags: []` from this emitter. The new
  tests should pin it.

### The three callers

- **`work create`**:
  - `--tag` is `Vec<String>` (`cli/work-cli/src/cli.rs:376-378`), mapped in
    `create_args_from_cli` (`cli/work-cli/src/main.rs:222-243`).
  - `CreationContext::content` (`cli/work-cli/src/create.rs:375-406`) builds
    `CreateInputs { tags: &args.tags, … }` (395) and calls
    `render_frontmatter` (403).
  - Every creation strategy goes through `content`: local, local+push,
    draft, and tracker-keyed.
- **`work create-batch --manifest`**:
  - `ManifestEntry.tags` (`cli/work-cli/src/batch_manifest.rs:34`) is parsed
    by `EntryFields::list("tags")` (256-271, 338). An absent or null field
    gives an empty Vec.
  - `create_args` copies the tags across (64).
  - `Batch::create` (`cli/work-cli/src/create_batch.rs:346-428`) goes through
    the same `CreationContext::content`.
- **Sync importer**:
  - `ConfiguredLocalAuthor::author_from_remote`
    (`cli/work-cli/src/sync_author.rs:297-382`) builds `CreateInputs` at
    341-360, with `tags: &[]` (355) and `external_id: Some(…)` (359).
  - It calls `crate::create::render_frontmatter` (361-362), then writes
    `{frontmatter}\n{description}` (368) through `exclusive_write`.
  - The imported item's defaults are kind `task`, priority `medium`, status
    `ready`, producer `sync-work-items` (57-60).
  - Because `external_id` is emitted before `tags`, the sync criterion's
    order (`external_id`, then `tags` immediately before `last_updated`)
    follows from the fix.

### Other writers that interact with a missing `tags` key (unchanged by the fix)

- **`work update --add-tag`** works through `apply_tags`
  (`cli/work-cli/src/update.rs:121-137`), `work::tags::mutate_tags`
  (`cli/work/src/tags.rs:94-129`) and `Mapping::set`
  (`cli/document/src/value.rs:57-63`).
  - `Mapping::set` appends a missing key at the end, so today `--add-tag` on
    a create-written file places `tags` after `schema_version`.
  - Removing the last tag keeps `tags: []`.
  - Block-style `tags` is rejected (`cli_update.rs:157-171`).
- **Sync `link_external_id`** (`sync_author.rs:233-286`) also appends
  `external_id` at the end when it is missing.
- **Sync pull** (`cli/work-adapters/src/sync/run.rs:390-414`) keeps
  frontmatter text verbatim.
- **Promotion and rekey retirement** (`cli/work/src/retirement.rs:843-872`)
  edit specific lines.
- None of these add or remove `tags`. Files that went through `--add-tag`
  before the fix already carry `tags`, possibly out of template order, and
  the migration must leave them alone.

### Validation and schema

- `BASE_FIELDS` (`cli/corpus/src/frontmatter_validation/schema.rs:330-345`) is
  `type, id, title, date, author, tags, last_updated, last_updated_by,
  schema_version`.
- How a missing key is reported:
  - `check_base_fields` (`mod.rs:257-266`) uses the key-only `is_present`
    (`mod.rs:82-84`) and emits `Violation::MissingBaseField`.
  - The code is `MISSING-BASE-FIELD` (`violation.rs:97`) and the message is
    "required base field '{field}' absent" (`violation.rs:157-158`).
- `tags: []` is safe: `WHOLLY_EXEMPT_KEYS = ["tags"]` exempts it from
  `EMPTY-PLACEHOLDER` (`mod.rs:386-404`).
- The work-item schema row (`schema.rs:28-54`) has `optional_extras:
  ["external_id", "aliases"]` and typed linkage `parent, blocks, blocked_by,
  derived_from, relates_to, source`.
- `templates/work-item.md`:
  - line 11 opens the omit-when-empty comment block (12-18);
  - `external_id` (19) and `aliases` (20) each carry their own "omit" note;
  - `tags: []` (21) has no omit note and sits directly before `last_updated`
    (22).

### Tests: what breaks, and patterns to reuse

- **Breaks:** `omits_empty_optional_fields` (`cli/work/src/create.rs:292-314`).
  It asserts the exact key vector without `"tags"`, so `"tags"` must be
  inserted before `"last_updated"`, and the name should change.
- **Goes stale but still passes:**
  `typed_linkage_and_tags_are_populated_and_omitted_when_empty`
  (`cli/work-cli/tests/cli_create.rs:122-155`) only checks the populated
  `tags` case.
- **Assertion style in `cli_create.rs`:** `content.contains(...)` on the
  whole file, with no snapshots and no adjacency checks.
  - Proving "`tags` is on the line immediately before `last_updated`" needs
    a line-pair assertion, e.g. searching for the substring
    `"tags: []\nlast_updated:"`.
  - Helpers: `scratch_repo()` (13-34), `run()` (65-71) and the baseline
    `creates_a_work_item_with_the_template_schema` (73-93).
- **create-batch:**
  - The CLI tests (`cli/work-cli/tests/cli_create_batch.rs`) never read
    frontmatter.
  - The in-crate tests in `create_batch.rs` (~600 onwards) read files with
    `read(path)` / `items(&report)`, and read a field with `parent_of`
    (867-872), which uses `split_frontmatter_and_body` + `read_field_raw`.
    A `tags_of` analogue fits.
- **Sync importer:** tests are in `sync_author.rs:397-949`. The fixtures are
  `TrackerRepo::new()` and `repo.pull("ENG-42")`.
  `a_pulled_issue_takes_its_external_id_as_id_under_tracker` (621-635) is the
  natural test to extend.
  - The `DiscoveredIssue` fixture would need to carry tracker labels to match
    the acceptance criterion "an issue carrying one or more tracker labels".
    Whether `DiscoveredIssue` has a labels field at all needs checking (see
    Open Questions).
- **"Validation exits 0" in tests:**
  - The `accelerator-corpus` binary isn't reachable from work-cli tests,
    because `CARGO_BIN_EXE_*` is defined only for the crate's own binaries.
  - The library is reachable: `corpus` and `corpus-adapters` are already
    dependencies of `accelerator-work` (`cli/work-cli/Cargo.toml:36-37`).
  - Pattern to copy, from `cli/corpus-adapters/tests/frontmatter_validation.rs:5-12, 88-91`:
    `assert_eq!(validate_text(&content, &YamlFrontmatter), Vec::new())`.
  - For CLI exit-code style, see `cli/corpus-cli/tests/frontmatter_goldens.rs`.

### Skill documentation

- **`skills/work/create-work-item/SKILL.md:634-641`** is the sentence to
  rewrite: "`work create` itself omits the corresponding frontmatter key when
  the flag is absent, matching the template's own omit-when-empty
  convention…". Lines 896-907 already describe `tags` as "a YAML array,
  possibly empty".
- **`skills/work/extract-work-items/SKILL.md:598-600`** says "`tags`,
  `blocks`, `blocked_by`, `relates_to`, `derived_from` and `source` follow
  the same omit-when-empty rules…". Leaving `tags` out of a manifest is still
  fine after the fix, but this wording repeats the misconception. It is not
  in 0333's scope.
- **`skills/work/sync-work-items/SKILL.md`** does not mention `tags`.
- **`docs-site/src/content/docs/reference/skills/**`** copies these
  sentences, but is gitignored and generated from SKILL.md. Don't edit it.

### Migration framework

- **Shape:**
  - `MigrationMeta` (`id`, `description`) plus `Migration::apply(&self, ctx:
    &dyn MigrationContext) -> Result<ApplyOutcome, MigrationError>`
    (`cli/migrate/src/registry.rs:23-50`).
  - `ApplyOutcome` is `Applied` or `NoOpPending`.
  - The registry is a hand-ordered `vec!` (`registry.rs:72-86`), with
    modules declared in `cli/migrate/src/migrations/mod.rs:5-16`.
- **Ledger:**
  - Files: `.accelerator/state/migrations-applied` and `migrations-skipped`.
  - `pending = registry − applied − skipped` (`cli/migrate/src/ledger.rs:54-67`).
  - `Applied` appends the ID (`cli/migrate/src/lifecycle.rs:104-116`).
    `NoOpPending` leaves the migration pending (117-119).
  - **So the new migration must return `Applied` even when no file
    changed.** This repository's own `meta/work/` is expected to be exactly
    that case. Returning `NoOpPending` would break the "not listed as
    pending on re-run" criterion.
- **Ordering and IDs:**
  - Migrations run in registry order.
  - The SessionStart lag check compares IDs as strings
    (`cli/migrate-cli/src/discoverability.rs:39-57`), so the ID must sort
    last: `0012-…`.
  - No jj revision in the repository touches an `m0012*` file. Work item
    0317, which would add a guard against duplicate IDs, is still a draft,
    so nothing would catch a colliding `0012` on another branch.
- **Runner:** `run_default` (`cli/migrate-cli/src/main.rs:253-343`):
  1. preflight: lock, dirty-tree classification over `meta/`, `.claude/accelerator`
     and `.accelerator/` (`cli/migrate/src/preflight.rs:17, 73-108`);
  2. preview banner (`render.rs:145-152`);
  3. `run_pending`.
  `ctx.write` is the only mutation path. It writes atomically and records
  the path in the run manifest (`cli/migrate-adapters/src/context.rs:128-141`).
- **Crate boundary:** `migrate` may import only `std`, `kernel::Error`,
  `crate` and `corpus` (`cli/pup.ron:604-619`). It cannot use
  `document::fence::fence_offsets`. m0007's private `fence.rs`/`merge.rs` are
  out of reach too.

#### m0007 precedent, and why not to copy it

- **Fence-less backfill:** `m0007/backfill.rs:56-115` synthesises a whole new
  frontmatter block, with `tags: []` at line 101. It regenerates the file
  rather than inserting into it.
- **Strict-fenced rewrite:** `m0007/rewrite.rs` pushes `tags: []` just before
  the closing fence (643-645), driven by `presence(content, "tags")`
  (145-147, 244).
  - `presence` requires a **non-empty** value, so a block-style `tags:`
    counts as absent. The new migration must not reuse that predicate.
  - `apply_rules` (490-495) is the correct precedent for treating `tags` as
    always kept: "every remaining key except `tags`".

#### Resolving and iterating the work directory

- **Option A, the m0005 pattern** (`m0005.rs:33-57`):
  `ctx.config_value("paths.work")` plus a duplicated `is_dangerous_path`
  check, `ctx.dir_exists`, `ctx.list_md_files`. It errors on an empty
  configured path.
- **Option B, the m0008 pattern** (`m0008.rs:90-108`): `ctx.doc_type_dirs()`
  filtered to `work-item`.
  - The paths are already root-joined and safety-checked
    (`cli/config/src/paths.rs:32-69`).
  - But the adapter falls back to an empty vec on a config error
    (`context.rs:115`), which would turn a broken config into a silent no-op.
- `list_md_files` is recursive and sorted, and returns empty for a missing
  directory. So it includes `drafts/` (where tracker-mode `create-batch`
  writes) and any non-work-item `.md` file under the work directory.
- `ctx.read` uses `String::from_utf8_lossy` (`context.rs:180-184`). m0008
  guards against writing back U+FFFD (`m0008.rs:166-168`).

#### Byte-preserving insertion recipe

1. Require line 0 to be exactly `---` (allowing `---\r`). Skip files without
   a fence: m0007 should have given every legacy item frontmatter.
2. Walk `content.split_inclusive('\n')`, adding up byte offsets, until the
   next exact `---` line, the closing fence. Body lines are never scanned, so
   a body `tags:` is ignored.
3. Detect: `is_present(&parse_entries(frontmatter), "tags")`.
   - `parse_entries` (`cli/corpus/src/frontmatter_validation/mod.rs:43-65`)
     scans column-0 `key:` lines only. Flow, empty and block-style
     (`tags:` followed by indented items) all register `tags`.
4. Anchor on the first column-0 line whose key is exactly `last_updated`.
   `last_updated:` as a prefix does not match `last_updated_by:`. If there is
   no such line, anchor on the closing fence's start offset ("last
   frontmatter line").
5. Splice `tags: []` plus the anchor line's own ending (`\r\n` or `\n`) at
   the anchor offset. Every other byte is unchanged, including CRLF files and
   a missing final newline.
6. Write only when the content changed, and return `Applied`.

- **Why not the existing text helpers:**
  - `migrations/text.rs` (`map_lines`, `ensure_lines` and the rest) goes
    through `lines()`/`join("\n")`, which drops `\r`.
  - `m0007/fence.rs:31-36` miscounts offsets on CRLF.
  - Convention (`m0010.rs:8-13`): migrations inline their helpers rather than
    sharing them through `text.rs`, because applied migrations are immutable
    history and `text.rs` items are public API.

#### Registration checklist

- `cli/migrate/src/migrations/mod.rs`: `pub mod m0012;`.
- `cli/migrate/src/registry.rs`: the `use`, plus
  `MigrationEntry::Mechanical(Box::new(Migration0012))` appended after the
  0011 entry.
- `cli/migrate/tests/fixtures/public-api.txt`: regenerate with
  `mise run public-api:update`.
- `CHANGELOG.md` `[Unreleased]` → `### Migrations`: "Run
  `/accelerator:migrate` (migration 0012) to …" (precedent at ~1210-1215).
- migrate-cli tests that hard-code the full ledger or the expected counts and
  logs need `0012-…` added or their counts bumped:
  - `tests/common/mod.rs:33-39` (`MIGRATIONS_FROM_0007`);
  - `full_registry_e2e.rs:45-63, 61, 79` (`applied: 10; pending (no-op): 1.`);
  - `full_registry_over_git.rs:116, 121-150, 170-181`;
  - `no_pending_and_help.rs:28-38`;
  - `dirty_tree_preflight.rs:392-401`;
  - `skip_unskip_unapply.rs:131-140`;
  - `run_lock.rs:16-25`;
  - `migration_0009.rs:17-26`;
  - the ledger seeds in `migration_0001`–`0008`, `0011`.
- `skills/config/migrate/SKILL.md` needs no change, since it doesn't list
  mechanical migrations.

#### Testing patterns

- **Unit tests** go inline in `mNNNN.rs`:
  - pure-function transform tests, including idempotency (`m0010.rs:270-413`);
  - `apply()` against an in-memory context double (`m0011.rs:210-297`'s
    builder `Context::new().holding(rel, content)` and a `writes` counter).
  - Every double must implement `doc_type_dirs`, `corpus_index` (a `NoIndex`
    stub) and `write`.
- **End-to-end tests** go in `cli/migrate-cli/tests/migration_NNNN.rs`. They
  seed the ledger so only the target is pending, run the binary, compare
  exact bytes and check the ledger. `migration_0010.rs:176-203` shows the
  re-run test.
  - Shared helpers live in `tests/common/mod.rs`. Its `work_item(...)` fixture
    (93-101) already carries `tags: []`, so tag-less fixtures must be written
    out explicitly.

## Code References

- `cli/work/src/create.rs:74-83` - doc comment wrongly listing `tags` as non-empty-only
- `cli/work/src/create.rs:154` - the defect: `push_sequence_if_non_empty(&mut fields, "tags", inputs.tags)`
- `cli/work/src/create.rs:169-177` - `push_sequence_if_non_empty`
- `cli/work/src/create.rs:292-314` - `omits_empty_optional_fields`, the unit test that will break
- `cli/work-cli/src/create.rs:239-247` - `render_frontmatter`
- `cli/work-cli/src/create.rs:375-406` - `CreationContext::content` (`work create` and `create-batch`)
- `cli/work-cli/src/batch_manifest.rs:34, 64, 256-271, 338` - manifest tags parsing
- `cli/work-cli/src/sync_author.rs:341-362` - sync importer `CreateInputs` with `tags: &[]`
- `cli/work-cli/tests/cli_create.rs:122-155` - test whose name goes stale; empty case untested
- `cli/document/src/value.rs:177-179` - sequences render as `FlowSeq`
- `cli/corpus/src/frontmatter_validation/schema.rs:330-345` - `BASE_FIELDS`
- `cli/corpus/src/frontmatter_validation/mod.rs:43-65, 82-84` - `parse_entries`, `is_present`
- `cli/corpus/src/frontmatter_validation/mod.rs:386-404` - `tags` exempt from `EMPTY-PLACEHOLDER`
- `templates/work-item.md:11-24` - omit-when-empty block, then `tags: []` before `last_updated`
- `skills/work/create-work-item/SKILL.md:634-641` - sentence to correct
- `skills/work/extract-work-items/SKILL.md:598-600` - same misconception (outside 0333's scope)
- `cli/migrate/src/registry.rs:23-50, 72-86` - migration traits and registry
- `cli/migrate/src/lifecycle.rs:104-123` - `Applied` vs `NoOpPending` ledger handling
- `cli/migrate/src/migrations/m0005.rs:33-57` - work-directory iteration via `paths.work`
- `cli/migrate/src/migrations/m0008.rs:90-108, 166-168` - `doc_type_dirs` iteration; lossy-UTF-8 guard
- `cli/migrate/src/migrations/m0007/backfill.rs:101` - existing `tags: []` emission (whole-block synthesis)
- `cli/migrate/src/migrations/m0007/rewrite.rs:145-147, 490-495, 643-645` - presence predicate (unsuitable), tags-kept rule
- `cli/migrate/src/migrations/m0011.rs:210-327` - newest unit-test double pattern
- `cli/migrate-cli/tests/migration_0010.rs:22-73, 176-203` - end-to-end pattern and re-run idempotency
- `cli/migrate-cli/src/discoverability.rs:39-57` - lexicographic highest-ID check
- `cli/pup.ron:604-619` - `migrate` crate import allow-list

## Architecture Insights

- **One composition point.**
  - All new-item frontmatter flows through the pure, domain-level
    `compose_frontmatter` in the `work` crate.
  - The CLI layer (`render_frontmatter`) is only a serialisation adapter.
  - The fix belongs in the domain function, and the `work` crate's unit
    tests are the primary red/green site. The CLI, batch and sync tests then
    pin each path's observable output.
- **Policy is enforced by guidance, not tests.** ADR-0040 notes
  (lines 170-177) that the omit-when-empty classification has no test over
  generated output. The key-set-only template drift guard is why this
  slipped through. Tests asserting that each writer's output passes
  `validate_text` would close that gap for work items.
- **Migrations are immutable history.** Helpers are inlined per migration;
  `migrate` depends only on `corpus` and `kernel`; side effects go through the
  `MigrationContext` port. Idempotency comes from two sides: the ledger, and
  each migration detecting for itself when there is nothing to do.
- **Presence vs non-emptiness.**
  - The corpus validator and the new migration care about key *presence*
    (`is_present`).
  - m0007's rewrite cared about non-empty values (`presence`).
  - Mixing the two is exactly the class of error behind this bug.

## Historical Context

- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` (~line 128):
  `tags` is a mandatory base field, an "array of strings, possibly empty". It
  sets no ordering; the ordering comes from the template.
  - Lines 182-185 require a `schema_version` bump for shape changes.
    Backfilling `tags` brings items up to the existing v1 shape, so no bump
    is warranted. The plan should state this.
- `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`:
  - lines 99 and 117: `tags: []` stays when empty;
  - lines 130-137 list the omit-when-empty set;
  - lines 141-147: reclassifying needs a new ADR. 0333 doesn't reclassify,
    so it is a bug fix, not a schema change.
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md`:
  - **Still applies:** self-detected idempotency, atomic writes, no partial
    ledger writes, no rollback beyond VCS.
  - **Stale:** its shell-script mechanism. Use the m0011 example in
    `meta/plans/2026-10-11-PP-884-sync-pulls-of-tracked-work-items-fail-silently.md`
    as the worked example instead. Its gotcha at ~1441-1442 says every
    migrate-cli suite seeding the ledger needed the new ID.
- `meta/plans/2026-08-06-PP-191-work-item-lifecycle-subdomain.md:1520-1525, 1701, 1714-1717`:
  - This is where `tags` was grouped with the omit-when-empty fields. It
    misread the template, and no rationale is recorded.
  - It also built the misclassification into the tests.
- `meta/reviews/work/PP-886-work-item-writers-omit-the-required-tags-field-review-1.md`:
  - Pass 1 brought the backfill migration into scope.
  - Pass 2 added byte preservation, the edge cases, migration ordering and
    numbering.
  - Final verdict: APPROVE.
- `meta/prs/161-description.md:36`: where the defect was first recorded. 0332
  and 0333 themselves needed `tags: []` added by hand.
- `meta/work/PP-903-guard-migration-registry-order-and-id-uniqueness.md`: still
  a draft, so duplicate or misordered IDs are not guarded.

## Related Research

- `meta/research/codebase/2026-10-29-PP-905-explicit-work-item-id-on-accelerator-work-create.md` - `work create`, `compose_frontmatter`, `sync_author`
- `meta/research/codebase/2026-08-06-PP-191-work-item-lifecycle-subdomain.md` - origin of the work CLI and `compose_frontmatter`
- `meta/research/codebase/2026-08-06-PP-193-migration-engine-implementation-research.md` - the Rust migration engine
- `meta/research/codebase/2026-06-20-PP-140-reconcile-0007-backfill-sentinel-with-validator.md` - m0007 backfill against the validator
- `meta/research/codebase/2026-10-11-PP-884-sync-pulls-of-tracked-work-items-fail-silently.md` - context for m0011

## Open Questions

- **`aliases` vs "the only keys left out when empty".**
  - The template marks `aliases` "omit when empty", and the work-item schema
    lists it as an optional extra.
  - `work create` never writes `aliases`, so the SKILL.md wording the
    acceptance criterion requires (naming the seven fields as "the only keys
    left out when empty") is true of `work create`'s output. It is not true
    of the template as a whole.
  - Either scope the sentence to what `work create` writes, or mention that
    `aliases` is never written by creation.
- **Sync criterion: "an issue carrying one or more tracker labels".**
  - The importer ignores labels: it hard-codes `tags: &[]`.
  - It still needs checking whether `DiscoveredIssue` carries labels at all,
    so that a test fixture can set them.
  - If it doesn't, the criterion is met trivially, and the test can only
    document that labels are not mapped.
- **Which files does the migration touch?**
  - `list_md_files` is recursive, so it covers `drafts/` and any
    non-work-item `.md` under the work directory (e.g. a README).
  - Should the migration act only on files whose frontmatter declares
    `type: "work-item"`, or on every fenced `.md` file?
  - The work item says "every work item in the work directory", which
    suggests restricting by `type`.
- **Work-directory resolution.**
  - `doc_type_dirs()` (m0008) is safe and config-driven, but silently empty
    on a config error.
  - `config_value("paths.work")` (m0005) fails loudly but duplicates
    path-safety checks.
  - The plan should pick one deliberately.
- **Should the `extract-work-items` SKILL.md:598-600 wording** be corrected
  in this change or in a separate item? It is the same misconception, but it
  falls outside 0333's stated scope.
- **Migration numbering:** `0012` is free as of this revision. Any other
  in-flight item adding a migration must agree the number.
