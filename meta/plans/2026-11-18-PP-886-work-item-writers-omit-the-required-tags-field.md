---
type: "plan"
id: "2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field"
title: "Work Item Writers Omit the Required Tags Field Implementation Plan"
date: "2026-11-18T18:37:06+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "done"
work_item_id: "work-item:PP-886"
parent: "work-item:PP-886"
derived_from: ["codebase-research:2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field"]
relates_to: ["work-item:PP-885"]
tags: ["work", "work-cli", "migrate", "frontmatter", "create-work-item", "extract-work-items", "sync-work-items"]
revision: "d0e5bf7340d9fd4798b94938b2bf2ce1dfce8f9f"
repository: "accelerator"
last_updated: "2026-11-19T01:41:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Work Item Writers Omit the Required Tags Field Implementation Plan

## Overview

`accelerator work create`, `work create-batch` and the sync importer leave
`tags` out of a new work item's frontmatter when there are no tags, so every
such item fails `accelerator corpus frontmatter validate` with
`MISSING-BASE-FIELD`. This plan makes the shared writer always emit `tags`
(as `[]` when empty). It then adds migration `0012` to backfill `tags: []`
into existing work items that lack it, changing nothing else in those files.
The migration leaves alone, and names, any item it can't edit safely.

## Current State Analysis

- The defect is one line: `push_sequence_if_non_empty(&mut fields, "tags",
  inputs.tags)` at `cli/work/src/create.rs:154`, inside
  `compose_frontmatter`. `tags` is treated like the omit-when-empty
  typed-linkage slots.
- All three writers reach it through `render_frontmatter`
  (`cli/work-cli/src/create.rs:239-247`):
  - `work create` and `create-batch` go through `CreationContext::content`
    (`cli/work-cli/src/create.rs:375-406`);
  - the sync importer goes through `ConfiguredLocalAuthor::author_from_remote`
    (`cli/work-cli/src/sync_author.rs:297-382`), which always passes
    `tags: &[]` (`sync_author.rs:355`).
- `tags` is already pushed directly before `last_updated`
  (`create.rs:154` → `156`), so emitting it every time puts it in template
  order with no reordering. An empty `FieldValue::Sequence` renders as flow
  `[]` through `serde_saphyr::FlowSeq` (`cli/document/src/value.rs:177-179`),
  but no test pins that literal.
- `BASE_FIELDS` (`cli/corpus/src/frontmatter_validation/schema.rs:330-345`)
  requires `tags`, and `templates/work-item.md:21` places `tags: []` outside
  the omit-when-empty block. ADR-0033 and ADR-0040 agree. The writer is the
  only side in error.
- The `compose_frontmatter` doc comment (`create.rs:74-83`) lists `tags`
  among the "non-empty-only" fields. The unit test
  `omits_empty_optional_fields` (`create.rs:292-314`) pins the omission.
  The CLI test `typed_linkage_and_tags_are_populated_and_omitted_when_empty`
  (`cli/work-cli/tests/cli_create.rs:122-155`) asserts only the populated
  case.
- Two SKILL.md passages repeat the misclassification:
  - `skills/work/create-work-item/SKILL.md:634-641`: "`work create` itself
    omits the corresponding frontmatter key when the flag is absent".
  - `skills/work/extract-work-items/SKILL.md:598-600`, which groups `tags`
    with the omit-when-empty fields.
- Tracker labels never reach the importer: neither `DiscoveredIssue`
  (`cli/work-adapters/src/sync/create.rs:39-44`) nor `RemoteIssue`
  (`cli/tracker/src/lib.rs:108-136`) has a labels field. The work item's
  sync criterion ("an issue carrying one or more tracker labels") is met by
  any pulled issue that ends up with `tags: []`. The test can't attach
  labels because the model has none.
- Migrations:
  - The registry (`cli/migrate/src/registry.rs:72-86`) ends at `0011`.
  - A migration must return `ApplyOutcome::Applied` for the ledger to record
    it (`cli/migrate/src/lifecycle.rs:104-119`).
  - The `migrate` crate may import only `std`, `kernel`, `crate` and
    `corpus` (`cli/pup.ron:604-619`).
  - `corpus::frontmatter_validation` exposes `parse_entries`, `is_present`
    and `declared_type` (`mod.rs:43-84, 148-150`). `parse_entries` scans
    column-0 `key:` lines through `str::lines()`. That strips `\r`, so CRLF
    files are read correctly, and flow, empty and block-style `tags` all
    register as present.
  - `migrations/text.rs` goes through `lines()`/`join("\n")` and drops
    `\r`, so it can't do a byte-preserving insertion.

## Desired End State

- Every work item written through `render_frontmatter` carries `tags`, as
  `[]` when there are none, on the line immediately before `last_updated`
  and after any omit-when-empty fields that are present.
- The omit-when-empty fields (`parent`, `blocks`, `blocked_by`,
  `derived_from`, `relates_to`, `source`, `external_id`) keep their
  present-only behaviour on all three paths.
- Both SKILL.md passages say that `tags` is always written.
- `/accelerator:migrate` offers migration
  `0012-backfill-work-item-tags`. It inserts exactly one `tags: []` line into
  every `type: "work-item"` file under `paths.work` whose frontmatter has no
  `tags` key, and leaves every other byte of every file unchanged. The
  exceptions are files it can't edit safely, which it leaves byte-identical
  and names with the fix each needs:
  - a file that is not UTF-8;
  - a file whose frontmatter doesn't parse as YAML;
  - a file whose `tags` key is spelled non-canonically;
  - a file where the insertion would change another field's value.
- It realigns the sync baselines of the items it repairs, so a synced item
  that was `Synced` before stays `Synced`.
- If a write or the realignment fails, it restores the items it had
  already repaired, so the failed run leaves the work directory as it
  found it. The exception is an item that changed after its repair, or
  that can't be written back. It names those, and the fix is to remove the
  one `tags: []` line it added.
- Once applied, it is not pending on a re-run.

Verify with `mise run` (the bare default task), which must exit 0, plus the
manual checks listed under each phase.

### Key Discoveries:

- The fix is one line in a pure domain function (`cli/work/src/create.rs:154`),
  so the `work` crate's unit test is the primary red/green site. The CLI,
  batch and sync tests then pin each path's observable output.
- `corpus::frontmatter_validation::pipeline::validate_text` with
  `corpus_adapters::YamlFrontmatter` is the in-process equivalent of
  `accelerator corpus frontmatter validate --file`. It is reachable from
  `work-cli` and `migrate-cli` tests, because both crates already depend on
  `corpus` and `corpus-adapters`. Precedent:
  `cli/corpus-adapters/tests/frontmatter_validation.rs`.
- `compose_config` (`cli/migrate-cli/src/main.rs:70`) fails before any
  migration runs when the config file is corrupt. `config_value("paths.work")`
  therefore resolves to the catalogue default `meta/work` when the key is
  unset, and the migration only has to refuse an empty or unsafe value
  itself (the m0005 pattern, `cli/migrate/src/migrations/m0005.rs:33-44,
  100-108`).
- Applied migrations are immutable history, so helpers are inlined per
  migration rather than shared (`m0010.rs:8-13`). m0012 still takes key
  presence from live `corpus` parsing, as m0007 does, because agreeing with
  the validator matters more than freezing behaviour. The flow, block and
  empty `tags` unit tests guard against upstream parser changes.
- `list_md_files` is recursive, sorted, and empty for a missing directory,
  so it reaches `drafts/` too.
- `ctx.read` is UTF-8-lossy (`cli/migrate-adapters/src/context.rs:180-184`).
  A file read back with U+FFFD can't be rewritten byte for byte, and the
  decoded text can't tell an undecodable file from a valid one holding
  U+FFFD. Phase 2 adds a strict `read_text` to the port for this.

## What We're NOT Doing

- Mapping tracker labels to `tags`. The importer keeps writing `tags: []`.
  That is a separate, not-yet-raised item.
- Reordering `tags` in files whose `tags` sits out of template order (for
  example after `work update --add-tag` appended it after `schema_version`).
  The migration only adds a missing key.
- Backfilling `tags` into other artefact types. Their templates already
  carry `tags`.
- Bumping `schema_version`. Adding `tags` brings items up to the existing v1
  shape (ADR-0033 lines 182-185 require a bump only for shape changes).
- Repairing work items that have no frontmatter fence, or whose fence is
  unterminated. Migration 0007 gives every legacy item frontmatter, and a
  broken fence is not this migration's to fix.
- Changing `templates/work-item.md`, the corpus schema, or the
  omit-when-empty classification in ADR-0040.
- Editing `docs-site/src/content/docs/reference/skills/**`, which is
  generated from SKILL.md and gitignored.
- Realigning the intended baseline held in a promotion record that is
  stalled under `pending-push/` (`cli/work/src/promotion.rs:56-61,
  125-128`). If 0012 repairs an item while its promotion is stuck in the
  `Retiring` stage, resuming the promotion writes a baseline from before
  the repair. The next sync then reports the item as locally modified and
  pushes unchanged content once. That costs one redundant push and loses
  no data, for a promotion interrupted at exactly that stage.

## Implementation Approach

Two phases, each of which can merge by itself:

1. Fix the writer test-first, pin each of the three paths' output, and
   correct the docs. From this point no new item lacks `tags`.
2. Add migration `0012` test-first: a pure, byte-preserving transform, an
   `apply` over the configured work directory, registration, and an
   end-to-end test. Existing items are repaired.

The two phases don't depend on each other's code. Phase 2's end-to-end
fixtures are hand-written. But Phase 2 must not be released before Phase
1. A migration that runs once can't repair items that an unfixed writer
creates after it.

---

## Phase 1: Writers Always Emit `tags`

### Overview

Write the failing tests for every writer path first, then make the one-line
change to `compose_frontmatter`, correct its doc comment, and fix both
SKILL.md passages.

### Changes Required:

#### 1. Red: the `compose_frontmatter` unit test

**File**: `cli/work/src/create.rs`
**Changes**:
- Rename `omits_empty_optional_fields` to
  `writes_empty_tags_and_omits_empty_linkage`.
- Insert `"tags"` between `"priority"` and `"last_updated"` in its expected
  key vector.
- Assert that the `tags` value is `FieldValue::Sequence(Vec::new())`.

```rust
#[test]
fn writes_empty_tags_and_omits_empty_linkage() {
    let fields = compose_frontmatter(&inputs(empty_linkage(), &[]));
    let keys: Vec<&str> =
        fields.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(
        keys,
        vec![
            "type",
            "id",
            "title",
            "date",
            "author",
            "producer",
            "status",
            "kind",
            "priority",
            "tags",
            "last_updated",
            "last_updated_by",
            "schema_version",
        ]
    );
    assert!(fields.contains(&(
        "tags".to_owned(),
        FieldValue::Sequence(Vec::new())
    )));
}
```

- Add `writes_every_field_the_schema_requires_when_all_else_is_empty`. It
  reads the required set from the `corpus` schema rather than restating
  it, so a required field added there later turns it red at the writer. It
  fails today with `["tags"]`.

```rust
#[test]
fn writes_every_field_the_schema_requires_when_all_else_is_empty() {
    let fields = compose_frontmatter(&inputs(empty_linkage(), &[]));
    let written: Vec<&str> =
        fields.iter().map(|(key, _)| key.as_str()).collect();
    let work_item = schema::row_for("work-item", "")
        .expect("a work-item schema row");
    let missing: Vec<&str> = schema::BASE_FIELDS
        .iter()
        .chain(work_item.required_extras)
        .copied()
        .filter(|key| !written.contains(key))
        .collect();
    assert_eq!(missing, Vec::<&str>::new());
}
```

#### 2. Red: `work create` CLI tests

The omit-when-empty checks share one set of helpers per test target. The
crate boundary between the integration tests and the in-crate unit tests
allows no fewer than two copies:
- `cli/work-cli/tests/common/mod.rs` for `cli_create.rs`;
- `cli/work-cli/src/test_support.rs` for the `create_batch` and
  `sync_author` test modules.

Both copies hold the same four items, so all three writer tests read the
same way:
- `OMIT_WHEN_EMPTY`: the seven omit-when-empty keys as `"<key>:"`
  prefixes. Each check is for a line that starts with the prefix, so
  `blocks:` never matches `blocked_by:`.
- `frontmatter_of(content)`: the text between the fences, built on
  `work_adapters::sync::digest::split_frontmatter_and_body` and failing
  loudly on a malformed fence, so the absence checks can never pass
  vacuously over an empty string.
- `omit_when_empty_lines(frontmatter)`: the frontmatter lines that start
  with an `OMIT_WHEN_EMPTY` prefix.
- `violations(content)`: `validate_text(content, &YamlFrontmatter)`.

```rust
use corpus::frontmatter_validation::pipeline::validate_text;
use corpus::frontmatter_validation::Violation;
use corpus_adapters::YamlFrontmatter;

pub const OMIT_WHEN_EMPTY: [&str; 7] = [
    "parent:",
    "blocks:",
    "blocked_by:",
    "derived_from:",
    "relates_to:",
    "source:",
    "external_id:",
];

pub fn frontmatter_of(content: &str) -> String {
    let (frontmatter, _) =
        work_adapters::sync::digest::split_frontmatter_and_body(content)
            .expect("a fenced frontmatter");
    frontmatter
}

pub fn omit_when_empty_lines(frontmatter: &str) -> Vec<&str> {
    frontmatter
        .lines()
        .filter(|line| {
            OMIT_WHEN_EMPTY.iter().any(|key| line.starts_with(key))
        })
        .collect()
}

pub fn violations(content: &str) -> Vec<Violation> {
    validate_text(content, &YamlFrontmatter)
}
```

**File**: `cli/work-cli/tests/cli_create.rs`
**Changes**:
- Add `mod common;`, and import the helpers from it.
- Rename `typed_linkage_and_tags_are_populated_and_omitted_when_empty` to
  `typed_linkage_is_populated_and_omitted_when_empty`. Its assertions stay
  as they are, so it is a characterisation pin that passes before and
  after the fix.
- Add these tests:
  - `an_untagged_item_is_written_with_empty_tags_before_last_updated`
    (criterion 1): `create "Example" story medium` with no flags. Assert:
    - the frontmatter contains `"tags: []\nlast_updated:"`;
    - no line starts with an `OMIT_WHEN_EMPTY` prefix;
    - `violations(&content)` is empty.
  - `tags_follow_the_linkage_fields_immediately_before_last_updated`
    (criterion 2): `--tag api --relates-to work-item:0001`. Assert:
    - the frontmatter contains `"relates_to: [\"work-item:0001\"]\n"` and
      `"tags: [\"api\"]\nlast_updated:"`;
    - the `relates_to` line's offset is lower than the `tags` line's.

    This is a characterisation pin. The current writer already places
    populated tags here, so it passes before and after the fix, guarding
    the populated path against regression.

```rust
mod common;

use common::frontmatter_of;
use common::omit_when_empty_lines;
use common::violations;

#[test]
fn an_untagged_item_is_written_with_empty_tags_before_last_updated(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), &["create", "Example", "story", "medium"])?;
    assert!(output.status.success(), "{output:?}");
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    let frontmatter = frontmatter_of(&content);

    assert!(frontmatter.contains("tags: []\nlast_updated:"), "{content}");
    assert_eq!(omit_when_empty_lines(&frontmatter), Vec::<&str>::new());
    assert_eq!(violations(&content), Vec::new());
    Ok(())
}
```

#### 3. Red: `create-batch` in-crate tests

**File**: `cli/work-cli/src/create_batch.rs` (test module)
**Changes**: add two tests built on `BatchRepo::numeric()`, which writes
local numeric IDs under `meta/work`, `push: false` and
`RecordingTracker::holding(Vec::new())`. Read each written file through
`items(&report)[ref].0` and `read(..)`, as
`push_declined_writes_every_item_as_a_draft_linked_to_parent_drafts` does.

- `an_untagged_entry_is_written_with_empty_tags_before_last_updated`
  (criterion 3). Manifest
  `[{"ref": "plain", "title": "Plain", "kind": "story", "priority": "medium"}]`.
  Assert:
  - the frontmatter contains `"tags: []\nlast_updated:"`;
  - `omit_when_empty_lines(&frontmatter)` is empty;
  - `violations(&content)` is empty.
- `a_tagged_entry_writes_its_tags_before_last_updated` (criterion 4). The
  same manifest plus `"tags": ["api"]`. Assert the frontmatter contains
  `"tags: [\"api\"]\nlast_updated:"`. Like criterion 2's test, this is a
  characterisation pin that passes before and after the fix.

Import `frontmatter_of`, `omit_when_empty_lines` and `violations` from
`crate::test_support`.

#### 4. Red: sync importer test

**File**: `cli/work-cli/src/sync_author.rs` (test module)
**Changes**: add
`a_pulled_issue_whose_labels_are_not_imported_gets_empty_tags_after_its_external_id`
(criterion 5), using `TrackerRepo::new()` and `repo.pull("ENG-42")`. Assert:
- the frontmatter contains `"external_id: \"ENG-42\"\n"` and
  `"tags: []\nlast_updated:"`;
- the `external_id` line comes before the `tags` line;
- `omit_when_empty_lines(&frontmatter)` equals
  `vec!["external_id: \"ENG-42\""]`, so `external_id` is the only
  omit-when-empty key present;
- `violations(&content)` is empty.

The helpers come from `crate::test_support`.

The test name records that the issue's labels are not imported, so a
later label-to-`tags` mapping meets a failing test that says why its
expectation was `[]`. The test sets no labels because `DiscoveredIssue`
has nowhere to carry them.

Run the four groups. Confirm that both `work` unit tests and each
`empty_tags` test fails because `tags` is absent rather than for a setup
error, and that the characterisation pins pass.

#### 5. Green: always emit `tags`

**File**: `cli/work/src/create.rs`
**Changes**: replace line 154 with an unconditional push.

```rust
fields.push((
    "tags".to_owned(),
    FieldValue::Sequence(inputs.tags.to_vec()),
));
```

#### 6. Refactor: correct the doc comment

**File**: `cli/work/src/create.rs:74-83`
**Changes**: replace the comment, which narrates every field group and
already drifted once, with only what the pushes and the ordered-key test
can't say: why `tags` differs from the fields beside it.

```rust
/// Builds the new item's field list, in template order.
///
/// `tags` is a required base field and is always written, as `[]` when
/// there are none.
```

#### 7. Correct the skill documentation

**File**: `skills/work/create-work-item/SKILL.md:634-641`
**Changes**: replace the paragraph so that it says (criterion 6):

> Include each bracketed flag only when that field has a value. `work
> create` always writes `tags`, as `[]` when no `--tag` is passed. The only
> keys it leaves out when empty are `parent`, `blocks`, `blocked_by`,
> `derived_from`, `relates_to`, `source` and `external_id`. Each flagged
> one is written only when its flag is passed, so no separate omission
> logic is needed here.
> `--tag`/`--block`/`--blocked-by`/`--derived-from`/`--relates-to` are each
> repeatable for multiple values.
> `external_id` has no flag; only the `--push` create in step 7 writes it.
> `aliases`, which records retired IDs, is added only when an item is
> rekeyed, never at creation.

`tests/unit/tasks/test_skill_frontmatter_population.py`
(`in_cli_delegated_omit_guidance`) reads this passage. Within 15 lines
after the `accelerator work create` fence it needs the bracket convention
explained ("only when") and one line naming `external_id` with "no flag".
Keep "`external_id` has no flag" on a single line when wrapping.

**File**: `skills/work/extract-work-items/SKILL.md:598-600`
**Changes**: take `tags` out of the omit-when-empty grouping.

> - `blocks`, `blocked_by`, `relates_to`, `derived_from` and `source`
>   follow the same omit-when-empty rules as the frontmatter in Step 4 h.
>   Leave `tags` out of an entry that has none: `create-batch` always
>   writes `tags`, as `[]` when empty. `body_file` is relative to the
>   manifest's directory.

**Files**: the Quality Guidelines in
`skills/work/create-work-item/SKILL.md:896-904` and
`skills/work/extract-work-items/SKILL.md:738-745`
**Changes**: both bullets say every template field "must be populated",
including `parent` ("empty string unless …") and `external_id` ("empty
unless set"). That contradicts the corrected passages, which leave those
keys out when empty. Reword the start of each bullet to:

> - Every written work item carries the required fields — …
>   `tags` (a YAML array, possibly empty), `last_updated`,
>   `last_updated_by`, and `schema_version: 1` — and the omit-when-empty
>   fields (the typed-linkage keys such as `parent`, plus `external_id`
>   and `aliases`) only when they have a value.

Drop the `parent (empty string unless …)` and `external_id (empty unless
set)` clauses from each list, and keep the rest of each bullet as it is.

Keep every passage within the 80-column wrap.

#### 8. Changelog

**File**: `CHANGELOG.md`, `[Unreleased]` → `### Fixed`
**Changes**: add an entry.

> - **New work items always carry `tags`.** `/create-work-item`,
>   `/extract-work-items` and `/sync-work-items` (through `work create`,
>   `work create-batch` and `work sync`) wrote no `tags` key for an item
>   without tags, so it failed `corpus frontmatter validate` with
>   `MISSING-BASE-FIELD`. They now write `tags: []`. Run
>   `/accelerator:migrate` (migration 0012) to repair items already written
>   without it.

#### 9. Re-run the evals the rewording reaches

**Files**: `skills/work/create-work-item/evals/benchmark.json`,
`skills/work/create-work-item/evals/benchmark.md`,
`skills/work/extract-work-items/evals/benchmark.json`
**Changes**: `tests/unit/tasks/test_work_skill_benchmarks.py` fails for a
work skill whose `SKILL.md` or `evals.json` has changed since its
benchmark's `skill_digest` was recorded, so §7 needs evals re-run. This
was found during implementation; the plan did not anticipate it.
- Re-run with_skill the evals whose runs pass through the reworded text:
  - `/create-work-item` 7 (draft frontmatter), 12 (a full local write)
    and 40 (both creates, with no tags or parent);
  - `/extract-work-items` 8 and 11 (manifest entries).
- Executors are barred from the evals and benchmark files and simulate
  every `accelerator` call. Separate graders judge each expectation from
  the outputs alone.
- Replace each re-run eval's with_skill run, record the new
  `skill_digest` on those runs and in `metadata`, and add a dated note to
  each benchmark (and to `/create-work-item`'s `benchmark.md`). Keep each
  file's existing JSON escaping, so unrelated lines don't churn.

All five passed in full.

### Success Criteria:

#### Automated Verification:

- [x] The `work` unit tests fail before the fix and pass after:
      `cargo nextest run --manifest-path cli/Cargo.toml -p work -E 'test(/writes_empty_tags_and_omits_empty_linkage|writes_every_field_the_schema_requires/)'`
- [x] The `empty_tags` tests fail before the fix and pass after:
      `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-work -E 'test(/empty_tags/)'`
- [x] The characterisation pins pass before and after the fix:
      `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-work -E 'test(/(^|::)(tags_follow_the_linkage|a_tagged_entry_writes|typed_linkage_is_populated)/)'`
      The patterns are anchored at the start of the test name, so
      `an_untagged_entry_…` (a red test) is not selected.
- [x] The skill population guard still accepts the reworded passage:
      `uv run pytest tests/unit/tasks/test_skill_frontmatter_population.py`
- [x] Both reworded skills' benchmarks record their current digest, and
      every `/create-work-item` eval passes in full:
      `uv run pytest tests/unit/tasks/test_work_skill_benchmarks.py`
- [x] The CLI workspace suite passes: `mise run test:unit:cli`
- [x] Format and lint pass: `mise run cli:check`
- [x] The `work` public API is unchanged: `mise run public-api:check`
- [x] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [x] In a scratch repository,
      `accelerator work create "Example" story medium` writes `tags: []` on
      the line before `last_updated`, and
      `accelerator corpus frontmatter validate --file <path>` exits 0.
- [x] Both SKILL.md passages, and both Quality Guidelines bullets, read
      correctly in context and make no claim that `tags` is omitted, or
      that `parent` or `external_id` is written empty.

**Implementation Note**: After completing this phase and all automated
verification passes, pause here for manual confirmation from the human that
the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Migration 0012 Backfills `tags`

### Overview

Add `cli/migrate/src/migrations/m0012.rs`. It holds a pure transform that
inserts `tags: []` into a work item lacking it without touching any other
byte, and an `apply` that runs the transform over every `.md` file under
`paths.work`. Give the migration port a strict UTF-8 read so that
non-UTF-8 files are left byte-identical. Register the migration and pin it
end to end.

### Changes Required:

#### 1. Red then green: the pure transform

**File**: `cli/migrate/src/migrations/mod.rs`
**Changes**: add `pub mod m0012;` first, so the unit tests below compile
and can be seen to fail. Registering the migration waits for §5.

Until `apply` exists, nothing outside the tests calls the transform, so the
crate's non-test build fails `dead_code` under `warnings = "deny"`. This
was found during implementation. The red and green runs before §3 select
the lib's test target only:
`cargo nextest run --manifest-path cli/Cargo.toml -p migrate --lib m0012`.

**File**: `cli/migrate/src/migrations/m0012.rs`
**Changes**:
- `with_empty_tags(content: &str) -> Option<String>` returns the repaired
  content, or `None` when the file isn't a fenced `type: "work-item"` whose
  frontmatter lacks a `tags` key.
- It walks `content.split_inclusive('\n')`, tracking byte offsets, so CRLF
  endings and a missing final newline survive untouched.
- Key presence and the document type come from `corpus` itself
  (`parse_entries`, `is_present`, `declared_type`), so the migration agrees
  with the validator about what counts as present.

Write these unit tests first, one per behaviour, each against a full
literal input and a full literal expected output:

| Test | Input | Expected |
|---|---|---|
| `inserts_tags_immediately_before_last_updated` | work item with `last_updated`, no `tags` | input with `tags: []\n` spliced in before the `last_updated` line |
| `a_repaired_item_validates` | the same item, complete apart from `tags` | `corpus::frontmatter_validation::validate_file` over the repaired frontmatter (the text between the fences) is empty; `migrate` has no `corpus-adapters` dependency, and the end-to-end test covers `validate_text` |
| `inserts_tags_as_the_last_frontmatter_line_without_last_updated` | no `tags`, no `last_updated` | `tags: []\n` immediately before the closing `---` |
| `last_updated_by_is_not_mistaken_for_last_updated` | `last_updated_by` but no `last_updated` | inserted before the closing fence, not before `last_updated_by` |
| `keeps_crlf_line_endings` | CRLF throughout | `tags: []\r\n` inserted, every other byte unchanged |
| `keeps_a_missing_final_newline` | body without a trailing newline | identical apart from the inserted line |
| `leaves_flow_tags_alone` | `tags: ["api"]` | `None` |
| `leaves_block_tags_alone` | `tags:\n  - api\n` | `None` |
| `leaves_empty_tags_alone` | `tags: []` | `None` |
| `ignores_a_tags_line_in_the_body` | no frontmatter `tags`, `tags: x` in the body | frontmatter gains `tags: []`, body unchanged |
| `leaves_other_document_types_alone` | `type: "plan"` without `tags` | `None` |
| `leaves_an_item_without_a_type_alone` | no `type:` key, no `tags` | `None` |
| `repairs_an_item_with_an_unquoted_work_item_type` | `type: work-item` (unquoted), no `tags` | `tags: []\n` inserted before `last_updated` |
| `leaves_unfenced_content_alone` | no opening `---` | `None` |
| `leaves_an_unterminated_fence_alone` | opening `---` with no closing fence | `None` |
| `a_repaired_item_needs_no_further_repair` | the output of the first test | `None` |

```rust
//! Backfills the required `tags` base field, as `tags: []`, into every work
//! item under `paths.work` whose frontmatter lacks it.
//!
//! The insertion walks byte offsets rather than using `migrations/text.rs`,
//! whose `lines()`/`join` drops `\r`, and its helpers are inline because an
//! applied migration is immutable history. Key presence and document type
//! deliberately come from the live `corpus::frontmatter_validation` parser,
//! as in m0007, so the migration repairs what `MISSING-BASE-FIELD` reports.
//! That parser only reads bare column-0 keys, so each repair is also checked
//! with a YAML parse: the original must not hold a `"tags":` key the scanner
//! can't see, and the result must read as the original plus `tags: []`, so
//! YAML readers never meet a duplicate key or a changed value. A failed
//! write or realignment restores the items already repaired, because once
//! an item holds `tags` a re-run could never see its original to realign
//! its sync baseline.

const FENCE: &str = "---";
const EMPTY_TAGS: &str = "tags: []";

struct Line<'a> {
    start: usize,
    text: &'a str,
}

impl<'a> Line<'a> {
    fn content(&self) -> &'a str {
        self.text.trim_end_matches('\n').trim_end_matches('\r')
    }

    fn ending(&self) -> &'a str {
        &self.text[self.content().len()..]
    }

    fn end(&self) -> usize {
        self.start + self.text.len()
    }

    fn holds_key(&self, key: &str) -> bool {
        self.content()
            .split_once(':')
            .is_some_and(|(found, _)| found == key)
    }
}

fn lines_with_offsets(content: &str) -> Vec<Line<'_>> {
    let mut start = 0;
    content
        .split_inclusive('\n')
        .map(|text| {
            let line = Line { start, text };
            start += text.len();
            line
        })
        .collect()
}

struct Frontmatter<'a> {
    opening: &'a Line<'a>,
    fields: &'a [Line<'a>],
    closing: &'a Line<'a>,
    entries: Vec<(String, String)>,
}

struct Insertion<'a> {
    at: usize,
    ending: &'a str,
}

fn fenced_frontmatter<'a>(
    content: &str,
    lines: &'a [Line<'a>],
) -> Option<Frontmatter<'a>> {
    let (opening, rest) = lines.split_first()?;
    if opening.content() != FENCE {
        return None;
    }
    let fence_at = rest.iter().position(|line| line.content() == FENCE)?;
    let (fields, from_closing) = rest.split_at(fence_at);
    let closing = from_closing.first()?;
    Some(Frontmatter {
        opening,
        fields,
        closing,
        entries: parse_entries(&content[opening.end()..closing.start]),
    })
}

impl<'a> Frontmatter<'a> {
    fn is_work_item_lacking_tags(&self) -> bool {
        declared_type(&self.entries) == Some("work-item")
            && !is_present(&self.entries, "tags")
    }

    fn tags_insertion(&self) -> Insertion<'a> {
        let preceding = iter::once(self.opening).chain(self.fields);
        let following = self.fields.iter().chain(iter::once(self.closing));
        let (before, anchor) = preceding
            .zip(following)
            .find(|(_, line)| line.holds_key("last_updated"))
            .unwrap_or((
                self.fields.last().unwrap_or(self.opening),
                self.closing,
            ));
        Insertion {
            at: anchor.start,
            ending: before.ending(),
        }
    }
}

fn with_empty_tags(content: &str) -> Option<String> {
    let lines = lines_with_offsets(content);
    let frontmatter = fenced_frontmatter(content, &lines)?;
    if !frontmatter.is_work_item_lacking_tags() {
        return None;
    }
    let Insertion { at, ending } = frontmatter.tags_insertion();
    Some(format!("{}{EMPTY_TAGS}{ending}{}", &content[..at], &content[at..]))
}
```

`tags_insertion` pairs every line from the opening fence to the closing
fence with the line before it, and inserts before the first line holding
`last_updated`, or else before the closing fence. The inserted line takes
the ending of the line before it. That line always has one, because
another line follows it, so a CRLF file gets `\r\n` and an LF file gets
`\n`. No index arithmetic is involved.

#### 2. Red then green: a strict UTF-8 read on the migration port

`ctx.read` decodes lossily, so writing its result back would replace every
invalid byte with U+FFFD. Scanning the decoded text for U+FFFD can't tell
an undecodable file from a valid one that holds the character (as m0008
can leave behind). So the port reports whether the bytes were UTF-8, and
m0012 never sees lossily decoded text.

**File**: `cli/migrate-adapters/tests/context.rs`
**Changes**: write these first, against `FileMigrationContext` in a
`TempDir`:

| Test | Asserts |
|---|---|
| `read_text_returns_utf8_content_unchanged` | a file holding `"a\r\nb\u{FFFD}"` reads as `Some(TextFile::Utf8(..))` with those exact characters |
| `read_text_reports_a_file_that_is_not_utf8` | a file holding the bytes `b"a\xFFb"` reads as `Some(TextFile::NotUtf8)` |
| `read_text_is_none_for_a_missing_file` | `None` |

All three tests fail until the adapter overrides `read_text`, because the
port's default is an error rather than `Ok(None)`.

**File**: `cli/migrate/src/ports.rs`
**Changes**:
- Add the domain type, and a port method beside `read`. Its default
  follows the convention of the other migration-specific capabilities
  (`parse_frontmatter`, `render_canonical`, `validate_frontmatter`): a
  context that hasn't implemented it fails loudly. An `Ok(None)` default
  would read as "no such file", and m0012 would then record itself as
  applied having repaired nothing.

```rust
/// A file's text, or the fact that its bytes are not UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFile {
    Utf8(String),
    NotUtf8,
}

/// `Ok(None)` when `path` does not exist. Unlike [`read`](Self::read),
/// never decodes lossily, so a migration can leave a file that is not
/// UTF-8 byte-identical.
///
/// # Errors
/// [`MigrationError`] when a present file cannot be read, or when this
/// context does not implement strict reads.
fn read_text(&self, _path: &Path) -> Result<Option<TextFile>, MigrationError> {
    Err(MigrationError::new("read_text is not implemented by this context"))
}
```

- Extend `read`'s doc with: "Decodes lossily, replacing invalid UTF-8
  with U+FFFD; use [`read_text`](Self::read_text) when the content may be
  written back."
- In the `parse_frontmatter` doc, remove the sentence "Only migrations 0001
  and 0008 call this", which becomes false.
- Generalise the realignment docs, which all describe m0008's whole-corpus
  re-render. m0012 adds a second caller, whose items are not
  content-identical: `tags: []` changes their digest.
  - `MigrationContext::realign_sync_baseline`:
    - remove "Only migration 0008 calls it";
    - reword "so a whole-corpus re-render does not spuriously reclassify
      content-identical items as locally modified" as "so a migration
      that rewrites work items does not reclassify items that were in sync
      before it as locally modified";
    - reword "each rewritten `meta/` file's pre-render bytes" as "each
      rewritten file's pre-migration bytes";
    - reword "after the files on disk have been re-rendered" as "after the
      files on disk have been rewritten".
  - `SyncBaselines` (`ports.rs:452-456`): "after a whole-corpus re-render"
    becomes "after a migration rewrites work items", and "each re-rendered
    `meta/` file's original content" becomes "each rewritten file's
    pre-migration content".
  - `cli/migrate-cli/src/sync_baselines.rs` module doc: "Migration 0008's
    sync-baseline realignment" becomes "Sync-baseline realignment after a
    migration rewrites work items".
  - `cli/work-adapters/src/sync/realign.rs`:
    - the module doc becomes "Realigning every integration's sync baseline
      after a migration has rewritten work items, so items that were in
      sync before it are not reclassified as locally modified";
    - in `realign_baselines` and `realigned_entry`, "re-render" becomes
      "migration" and "re-rendered file's" becomes "rewritten file's".
  - `cli/work-adapters/tests/sync_realign.rs` module doc: "after a
    whole-corpus re-render" becomes "after a migration rewrites work items",
    and "the re-rendered digest" becomes "the rewritten file's digest".

**File**: `cli/migrate-adapters/src/context.rs`
**Changes**: extract the bounded byte read that `read` already does into a
private `read_bytes`. Build `read` and `read_text` on it.

```rust
fn read_text(&self, path: &Path) -> Result<Option<TextFile>, MigrationError> {
    Ok(self.read_bytes(path)?.map(|bytes| {
        String::from_utf8(bytes).map_or(TextFile::NotUtf8, TextFile::Utf8)
    }))
}
```

#### 3. Red then green: `apply` over the work directory

**File**: `cli/migrate/src/migrations/m0012.rs`
**Changes**:
- `Migration0012`:
  - id `0012-backfill-work-item-tags`;
  - description "Add the required tags field, empty, to every work item
    missing it."
- `apply` resolves `paths.work` as m0005 does, inlined:
  - an empty value is an error naming `0012`;
  - a value that `is_dangerous_path` refuses is an error naming the value.
    Inline m0010's `is_dangerous_path` (`m0010.rs:98-108`), not m0005's: it
    also refuses `""` and `/./`.
- When the configured work directory doesn't exist, or is not a
  directory, warn and report a zero count, then return
  `ApplyOutcome::Applied` straight away, as m0005 and m0010 do. Listing a
  path that is a file would fail with `NotADirectory`.
- For each file from `ctx.list_md_files(&work_dir)`:
  - Read it with `ctx.read_text`.
  - A file that is not UTF-8 is left untouched and recorded. Its content
    can't be parsed, so whether it needs repair is unknown.
  - Otherwise compute `with_empty_tags`. When it offers a repair, check it
    with `ctx.parse_frontmatter` before writing. The line scanner reads
    only bare column-0 keys, so YAML can disagree with it. The file is left
    untouched and recorded when:
    - the original's frontmatter doesn't parse, or isn't a mapping. The
      parser's error is kept so the note can show it;
    - the original's YAML mapping already holds `tags`, spelled in a way
      the scanner can't see (`"tags":`, `'tags':`, `tags :`), where the
      insertion would add a duplicate key;
    - the repaired frontmatter doesn't read as the original's mapping plus
      `tags: []`. This is a defensive check: with the real parser, a
      column-0 `last_updated:` line in frontmatter that parses is a genuine
      key, so no known input reaches it. It guarantees that a disagreement
      between scanner and parser can only ever leave a file alone.
  - Otherwise record the repair (path, original and repaired content),
    then write it. Recording before writing matters because a write can
    return an error after its rename has landed (for example when the
    manifest append fails), and such a file must still be restored.
  - Only files offered a repair are parsed, so the YAML cost grows with
    the number of repairs.
- Realign the `/sync-work-items` baselines over the recorded originals with
  `ctx.realign_sync_baseline`, as m0008 does. `tags` is not in
  `work::normalise::IGNORE_KEYS`, so without this, every synced item the
  backfill touches would reach the next sync as `LocallyModified` (or as a
  `Conflict` under a legacy baseline whose remote has also moved). The
  sync importer always wrote items without `tags`, so the items repaired
  are mostly synced ones.
- If the backfill or the realignment fails, restore every recorded repair
  and return an error. Once an item holds `tags`, a re-run never sees its
  original again, so neither its repair nor its baseline could be
  finished later. Restoring lets the re-run start afresh. Each repair is
  restored only from a state the migration knows:
  - a file that still holds its repaired content gets its original back;
  - a file that already holds its original (its write never landed) is
    left as it is;
  - any other file changed after the repair, perhaps by a concurrent
    edit, so it is not overwritten. It is named, and the fix is to
    remove the added line.
- The error names the stage that failed, then gives one set of recovery
  steps, in the order to follow them:
  1. For any item it couldn't restore, remove the one `tags: []` line 0012
     added. The repair is exactly the original plus that line, so removing
     it brings back the original and keeps any later edit. Restoring from
     version control would also undo an earlier migration's uncommitted
     rewrite of the file, such as 0008's.
  2. Commit everything the run changed, including any `last-sync.json`,
     rather than discarding it, and don't push yet. The migrate pre-flight
     reports these as unowned changes, because realignment writes bypass
     the run manifest. Committing only `last-sync.json` would move the run
     base, so the next pre-flight would refuse the rest of the run's
     output. Discarding would be wrong: in a run that also applied 0008, it
     would throw away 0008's recorded realignment. Entries 0012 advanced
     before the failure already match the repair the re-run makes again,
     and realignment converges.
  3. Re-run `/accelerator:migrate` before syncing, then push.

  This holds for both stages. Even a failed backfill can leave
  `last-sync.json` changed, by an earlier migration in the same run.
- Report to stderr as m0005 and m0010 do, so every successful run leaves a
  record:
  - always, the count of repaired items under the configured directory
    and of realigned baselines, including when both are 0;
  - a note naming every file left untouched (relative to the root), with
    its reason and the fix it needs by hand:
    - not UTF-8: add `tags: []` if it is a work item lacking it;
    - frontmatter that doesn't parse as YAML, with the parser's error: fix
      the frontmatter, then add `tags: []`;
    - a non-canonical `tags` key: rewrite the key as `tags:`;
    - an insertion that would change another value: add `tags: []` by
      hand where it doesn't.
- `configured_work_dir(ctx) -> Result<String, MigrationError>` returns the
  validated relative value, so the path and the messages share it.
- Always return `ApplyOutcome::Applied` from a successful run. A
  repository with nothing to repair must still record the migration as
  applied, or it stays pending (criterion 10).

```rust
#[derive(Debug, PartialEq, Eq)]
enum Untouched {
    NotUtf8,
    UnparseableFrontmatter(MigrationError),
    NonCanonicalTagsKey,
    InsertionChangesMeaning,
}

struct Repair {
    file: PathBuf,
    original: String,
    repaired: String,
}

impl Repair {
    fn restore(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<(), MigrationError> {
        if self.holds(ctx, &self.original)? {
            return Ok(());
        }
        if !self.holds(ctx, &self.repaired)? {
            return Err(MigrationError::new("it changed after the repair"));
        }
        let written = ctx.write(&self.file, &self.original);
        if written.is_err() && self.holds(ctx, &self.original)? {
            return Ok(());
        }
        written
    }

    fn holds(
        &self,
        ctx: &dyn MigrationContext,
        content: &str,
    ) -> Result<bool, MigrationError> {
        Ok(matches!(
            ctx.read_text(&self.file)?,
            Some(TextFile::Utf8(current)) if current == content
        ))
    }
}

enum Failure {
    Backfilling(MigrationError),
    Realigning(MigrationError),
}

#[derive(Default)]
struct Backfill {
    repairs: Vec<Repair>,
    untouched: Vec<(PathBuf, Untouched)>,
}

impl Backfill {
    fn over(
        &mut self,
        ctx: &dyn MigrationContext,
        work_dir: &Path,
    ) -> Result<(), MigrationError> {
        for file in ctx.list_md_files(work_dir)? {
            let content = match ctx.read_text(&file)? {
                None => continue,
                Some(TextFile::NotUtf8) => {
                    self.untouched.push((file, Untouched::NotUtf8));
                    continue;
                }
                Some(TextFile::Utf8(content)) => content,
            };
            let Some(backfilled) = with_empty_tags(&content) else {
                continue;
            };
            if let Some(reason) = yaml_obstacle(ctx, &content, &backfilled) {
                self.untouched.push((file, reason));
                continue;
            }
            let written = ctx.write(&file, &backfilled);
            self.repairs.push(Repair {
                file,
                original: content,
                repaired: backfilled,
            });
            written?;
        }
        Ok(())
    }

    fn pre_migration(&self) -> Vec<(PathBuf, String)> {
        self.repairs
            .iter()
            .map(|repair| (repair.file.clone(), repair.original.clone()))
            .collect()
    }

    fn restore_after(
        &self,
        ctx: &dyn MigrationContext,
        failure: &Failure,
    ) -> MigrationError {
        let mut restored = 0usize;
        let mut by_hand = Vec::new();
        for repair in &self.repairs {
            match repair.restore(ctx) {
                Ok(()) => restored += 1,
                Err(cause) => by_hand.push((repair.file.as_path(), cause)),
            }
        }
        MigrationError::new(restoration_message(
            ctx.root(),
            failure,
            restored,
            &by_hand,
        ))
    }
}

fn yaml_obstacle(
    ctx: &dyn MigrationContext,
    original: &str,
    repaired: &str,
) -> Option<Untouched> {
    let before = match ctx.parse_frontmatter(original) {
        Ok(FrontmatterValue::Mapping(before)) => before,
        Ok(_) => {
            return Some(Untouched::UnparseableFrontmatter(
                MigrationError::new("the frontmatter is not a mapping"),
            ))
        }
        Err(cause) => return Some(Untouched::UnparseableFrontmatter(cause)),
    };
    if before.get("tags").is_some() {
        return Some(Untouched::NonCanonicalTagsKey);
    }
    match ctx.parse_frontmatter(repaired) {
        Ok(FrontmatterValue::Mapping(after))
            if is_original_plus_empty_tags(&before, &after) =>
        {
            None
        }
        _ => Some(Untouched::InsertionChangesMeaning),
    }
}

fn is_original_plus_empty_tags(before: &Mapping, after: &Mapping) -> bool {
    let empty_tags = FrontmatterValue::Sequence(Vec::new());
    after.entries().len() == before.entries().len() + 1
        && after.get("tags") == Some(&empty_tags)
        && before
            .entries()
            .iter()
            .all(|(key, value)| after.get(key) == Some(value))
}

impl Migration for Migration0012 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let configured = configured_work_dir(ctx)?;
        let work_dir = ctx.root().join(&configured);
        if !ctx.dir_exists(&work_dir) {
            eprintln!("Warning: 0012: no work directory at {configured}");
            eprintln!("{}", summary(&configured, 0, 0));
            return Ok(ApplyOutcome::Applied);
        }
        let mut backfill = Backfill::default();
        let realigned = backfill
            .over(ctx, &work_dir)
            .map_err(Failure::Backfilling)
            .and_then(|()| {
                ctx.realign_sync_baseline(&backfill.pre_migration())
                    .map_err(Failure::Realigning)
            })
            .map_err(|failure| backfill.restore_after(ctx, &failure))?;
        eprintln!(
            "{}",
            summary(&configured, backfill.repairs.len(), realigned)
        );
        if let Some(note) = untouched_note(ctx.root(), &backfill.untouched) {
            eprintln!("{note}");
        }
        Ok(ApplyOutcome::Applied)
    }
}
```

`summary` formats `0012: added tags to {repaired} work item(s) under
{configured}, {realigned} sync baseline(s) realigned`.
A restoring write can fail the same way the repair's did, after its
rename has landed. `restore` reads the file back, so such a file counts as
restored rather than being named for fixing by hand.

`restoration_message` builds its text in this order:
- the stage: `0012: adding tags failed: {cause}.` or `0012: realigning sync
  baselines failed: {cause}.`;
- `Left the {restored} work item(s) it had repaired as it found them.`,
  left out when `restored` is 0;
- when `by_hand` is not empty: `Before re-running, remove the tags: []
  line 0012 added from: {path} ({cause}), …`, with root-relative paths;
- always: `If the migrate pre-flight then reports changes from this run,
  including last-sync.json, commit them all rather than discarding them,
  and don't push yet; re-run /accelerator:migrate before syncing.`

Unit tests against an in-memory context double. Model it on m0011's
`Context` (`m0011.rs:192-297`), whose `files` map lets a test read back
what each file holds. Configure each scenario through per-test tables, as
m0008's double does with `rendering(original, rendered)`, rather than
through global flags or write-call numbers:
- a `work` field answering `config_value("paths.work")`, and `dir_exists`
  answering true for it unless a test says otherwise;
- `list_md_files` returning the held paths under `dir`;
- `read_text` answering from `files`, with a `not_utf8` set of paths it
  answers `TextFile::NotUtf8` for;
- `parse_frontmatter`, which by default builds a mapping from
  `parse_entries` over the frontmatter, reading `[]` as an empty sequence
  and anything else as a string scalar. `parsing(content, result)`
  overrides the answer for one exact content, so a test can make the
  original or the repaired text fail to parse, hold a hidden `tags`, or
  read with a changed value;
- `failing_write_of(path, content)`: that write returns an error and
  leaves `files` unchanged;
- `landing_write_of(path, content)`: that write updates `files` and then
  returns an error, as a failed manifest append does;
- `editing_on_failure(path, content)`: when a failing write fires, it also
  sets `path` to `content`, standing in for a concurrent edit;
- `fail_realign: bool`, making `realign_sync_baseline` return an error;
- `realigned: RefCell<Option<Vec<(PathBuf, String)>>>`, which stays `None`
  until `realign_sync_baseline` is called, so "never called" is
  observable;
- `failing_listing: bool`, making `list_md_files` return an error, for
  the listing half of `a_failure_before_any_repair_omits_the_left_count`;
- `writes: RefCell<Vec<PathBuf>>`, recording every write attempted, for
  the "nothing is written" assertions.

The last two were found during implementation; the list above did not
name them.

The tests of skipped files build `let mut backfill = Backfill::default();`,
call `backfill.over(&ctx, dir)?`, and assert on `backfill.untouched` and
`backfill.repairs`. `apply` sends `untouched` only to stderr.
Their repaired texts come from calling `with_empty_tags` on the original.

| Test | Asserts |
|---|---|
| `repairs_only_the_work_items_missing_tags` | of a tag-less item, a tagged item and a `README.md` without frontmatter, only the first is rewritten and the other two hold their original bytes |
| `reads_the_configured_work_directory` | with `paths.work = "docs/work"`, an item there is repaired and one under `meta/work` is not |
| `a_work_directory_needing_no_repair_still_applies` | `matches!(outcome, ApplyOutcome::Applied)`, nothing is written, and `realigned` is `Some(vec![])` |
| `a_missing_work_directory_applies_without_listing_or_realigning` | with `dir_exists` false, `apply` is `Applied`, nothing is written, and `realigned` is `None` |
| `refuses_an_empty_work_path` | `Err` whose message names `0012` |
| `dangerous_work_paths_are_refused` | `is_dangerous_path` holds for `""`, `.`, `..`, `/`, `/abs`, `a/..`, `../b`, `a/../b` and `a/./b`, and not for `meta/work` or `docs/work` |
| `refuses_a_work_path_outside_the_project` | `apply` is `Err`, naming the value, for `paths.work = "../elsewhere"` |
| `repairs_a_draft_in_a_nested_directory` | a tag-less `meta/work/drafts/draft-a1b2c3-x.md` is rewritten with `tags: []` |
| `leaves_a_file_that_is_not_utf8_untouched` | for a file in `not_utf8`, `untouched == [(path, Untouched::NotUtf8)]` and `repairs` is empty |
| `repairs_an_item_holding_a_literal_replacement_character` | a tag-less, valid UTF-8 item whose body holds U+FFFD is rewritten with `tags: []` |
| `leaves_an_item_whose_yaml_already_holds_tags_untouched` | with the original parsing to a mapping that holds `tags`, `untouched == [(path, Untouched::NonCanonicalTagsKey)]` and nothing is written |
| `leaves_an_item_whose_frontmatter_does_not_parse_untouched` | with the original parsing to `Err(cause)`, `untouched == [(path, Untouched::UnparseableFrontmatter(cause))]` and nothing is written |
| `leaves_an_item_whose_insertion_would_change_a_value_untouched` | with the repaired text parsing to the original's mapping plus `tags: []` but with one existing value changed (same entry count), the item is recorded as `InsertionChangesMeaning`, so the value comparison alone catches it |
| `leaves_an_item_whose_repair_lacks_empty_tags_untouched` | with the repaired text parsing to the original plus an extra entry other than `tags`, the item is recorded as `InsertionChangesMeaning` |
| `leaves_an_item_whose_repair_adds_more_than_tags_untouched` | with the repaired text parsing to the original plus `tags: []` plus one more entry, the item is recorded as `InsertionChangesMeaning`, so the entry-count check alone catches it |
| `leaves_an_item_whose_repair_reads_tags_as_non_empty_untouched` | with the repaired text parsing to the original plus `tags: ["x"]`, the item is recorded as `InsertionChangesMeaning`, so the exact empty-`tags` comparison catches it |
| `leaves_an_item_whose_repair_does_not_parse_untouched` | with the repaired text parsing to `Err`, the item is recorded as `InsertionChangesMeaning` |
| `parses_only_items_offered_a_repair` | a tagged item whose content is overridden to `Err` is not in `untouched` |
| `names_each_untouched_file_with_its_reason_in_the_note` | `untouched_note` lists each root-relative path with its reason and its fix by hand, including the parser's error for unparseable frontmatter, for all four reasons, and returns `None` for an empty list |
| `realigns_the_baselines_of_backfilled_items` | of a tag-less item and a tagged item, `realigned` is `Some` of exactly the tag-less item's path with its pre-migration bytes |
| `a_failed_write_restores_the_items_already_repaired` | three tag-less items with `failing_write_of` the second's repair: `apply` is `Err`; all three hold their original bytes; `realigned` is `None`; the message starts `0012: adding tags failed:`, says `Left the 2 work item(s)`, and has no `remove the tags: []` clause |
| `a_write_that_lands_then_fails_is_restored` | with `landing_write_of` the second's repair, the second item holds its original bytes after `apply` returns `Err` |
| `a_restoring_write_that_lands_then_fails_counts_as_restored` | with `failing_write_of` the second's repair and `landing_write_of` the first's restoring write, the first item holds its original bytes, and the message says `Left the 2 work item(s)` with no `remove the tags: []` clause |
| `a_failure_before_any_repair_omits_the_left_count` | with `failing_write_of` the only item's repair and that write never landing, the message says `Left the 1 work item(s)`; with a listing error before any repair, the message has no `Left the` clause |
| `a_failed_realignment_restores_every_repaired_item` | two tag-less items with `fail_realign: true`: `apply` is `Err`; both hold their original bytes; the message starts `0012: realigning sync baselines failed:`, says `Left the 2 work item(s)`, has no `remove the tags: []` clause, and says to commit all the run's changes, including `last-sync.json`, without pushing and re-run before syncing |
| `an_item_changed_after_its_repair_is_named_not_overwritten` | with `failing_write_of` the second's repair and `editing_on_failure` the first item to new content, the first item keeps that new content, and the message names it in the `remove the tags: []` clause, ahead of the commit-and-re-run step |
| `names_the_items_it_could_not_restore` | with `failing_write_of` both the second's repair and the first's restoring write (which never lands), the message names the first item's root-relative path and the write error in the `remove the tags: []` clause |

#### 4. Red: end-to-end tests

**File**: `cli/migrate-cli/tests/migration_0012.rs`
**Changes**: follow `migration_0010.rs`:
- an `already_applied_except_0012` ledger seed;
- running the `accelerator-migrate` binary in a `TempDir`;
- exact-byte comparisons.

**File**: `cli/migrate-cli/Cargo.toml`
**Changes**: add a dev-dependency on `store` with its `test-support`
feature, for `ReadOnlyDir`, so the test binary compiles.

Write these before registering the migration. Until §5, the binary doesn't
know 0012, so every test fails on the ledger or on unrepaired content.

Every fixture is written out in full. The shared `work_item(...)` helper
already carries `tags: []`, so it isn't used.

| Test | Asserts |
|---|---|
| `backfills_an_item_missing_tags` (criterion 7) | the file equals the original with `tags: []\n` inserted before `last_updated`; `validate_text(&repaired, &YamlFrontmatter)` is empty; the ledger contains `0012-backfill-work-item-tags`; a tag-less draft under `meta/work/drafts/` is repaired the same way; stderr contains `0012: added tags to 2 work item(s) under meta/work, 0 sync baseline(s) realigned` |
| `backfills_a_crlf_item_through_the_real_yaml_guard` | a tag-less work item with CRLF endings throughout equals its original with `tags: []\r\n` inserted before `last_updated`, so the real `parse_frontmatter` accepts CRLF on both the original and the repair |
| `warns_when_the_work_directory_is_absent` | with no `meta/work`, stderr contains `Warning: 0012: no work directory at meta/work` and `0012: added tags to 0 work item(s) under meta/work`, and the ledger contains `0012-backfill-work-item-tags` |
| `applies_when_the_work_path_is_a_file` | with `meta/work` a regular file, the run succeeds, stderr carries the same warning, the file is byte-identical, and the ledger contains `0012-backfill-work-item-tags`; without the early return, listing the file would fail with `NotADirectory` |
| `backfills_an_item_without_last_updated_at_the_end_of_its_frontmatter` (criterion 8) | `tags: []` is the last frontmatter line; the rest is byte-identical |
| `leaves_tagged_items_untouched_and_ignores_body_tags` (criterion 9) | the flow-`tags` and block-`tags` items are byte-identical; the body-`tags:` item gains `tags: []` in its frontmatter with its body unchanged |
| `a_backfilled_work_directory_is_unchanged_on_a_second_run` (criterion 10) | the first run repairs the fixture (its content equals the expected repaired bytes, and the ledger contains `0012-backfill-work-item-tags`); a second run then prints `No pending migrations.` and every file is byte-identical to its state after the first run |
| `a_backfilled_work_directory_is_byte_stable_on_re_application` | modelled on `migration_0010.rs`'s `a_stripped_corpus_is_byte_stable_on_re_application`, but first asserting that the first run repaired the fixture as above; then re-seed the ledger without 0012, run again, and every file is byte-identical to its state after the first run (the stragglers procedure under Migration Notes) |
| `a_synced_item_s_baseline_is_realigned` | a tag-less item with `id: "0001"` and `external_id`, its `linear/last-sync.json` entry seeded with `work_adapters::sync::digest::local` of the original (m0008's `seed_baseline` pattern): after the run, `baseline_local_hash(root, "0001")` equals `digest::local` of the repaired file, which differs from the original's, and stderr contains `0012: added tags to 1 work item(s) under meta/work, 1 sync baseline(s) realigned` |
| `names_and_leaves_alone_an_item_that_is_not_utf8` | a tag-less item whose body holds the raw byte `0xFF` (written with `fs::write` on bytes): after the run, its bytes are identical, stderr names its root-relative path, and the ledger contains `0012-backfill-work-item-tags` |
| `names_and_leaves_alone_an_item_with_a_quoted_tags_key` | a work item whose only tags key is `"tags": ["api"]`, read by the real YAML parser: after the run, its bytes are identical, stderr names its root-relative path and asks for the key to be rewritten as `tags:` |
| `names_and_leaves_alone_an_item_whose_frontmatter_does_not_parse` | a tag-less work item whose `title` is a double-quoted scalar continued on an under-indented second line, a realistic hand edit the real parser rejects: after the run, its bytes are identical, and stderr names it with the parser's error and asks for the frontmatter to be fixed |
| `a_failed_write_leaves_the_work_directory_as_it_found_it` | tag-less items at `meta/work/0001-a.md` and `meta/work/locked/0002-b.md`, with `locked/` sealed through `store::test_support::ReadOnlyDir` (0o555, so the read succeeds and only the write fails). The test returns early when the guard yields `None` (running as root with the skip variable set), and the guard restores 0o755 on drop. The run exits non-zero; both files hold their original bytes; stderr says `Left the 2 work item(s)`; the ledger lacks 0012. Once the guard is dropped, a re-run repairs both |

#### 5. Green: register the migration

- **`cli/migrate/src/registry.rs`**: import `Migration0012` and append
  `MigrationEntry::Mechanical(Box::new(Migration0012))` after the 0011
  entry. §4's tests now pass.
- **`cli/migrate/tests/fixtures/public-api.txt`**: regenerate with
  `mise run public-api:update`.
- **migrate-cli tests that hard-code the whole registry**: add
  `0012-backfill-work-item-tags` after `0011-ignore-machine-local-sync-state`
  wherever a seeded ledger or expected ledger lists every migration, and
  bump the expected counts.
  - `tests/common/mod.rs:33-39`: `MIGRATIONS_FROM_0007` becomes
    `[&str; 6]`.
  - `full_registry_e2e.rs:45-63, 61, 79`: `applied: 10` becomes
    `applied: 11`.
  - `full_registry_over_git.rs:116, 145-146, 180`: the count, plus
    `[0012-backfill-work-item-tags] running` and `… applied` lines. In
    `a_typed_ticket_reaches_the_unified_schema_byte_for_byte`, 0001 has
    moved the ticket into `meta/work` and 0007 has given it `tags: []`, so
    the exact stderr gains exactly one line between those two:
    `0012: added tags to 0 work item(s) under meta/work, 0 sync
    baseline(s) realigned`, with no warning. The halting test
    (`an_untyped_ticket_halts…`) stops at 0007 and is unchanged.
  - `no_pending_and_help.rs:28-38`, `dirty_tree_preflight.rs:401`,
    `skip_unskip_unapply.rs:140`, `run_lock.rs:25`, `migration_0009.rs:26`.
  - The seeded ledgers in `migration_0001.rs` to `migration_0008.rs`.
  - `migration_0011.rs`: add `0012-backfill-work-item-tags` to its ledger
    seed, renaming `LEDGER_THROUGH_0010` to `LEDGER_EXCEPT_0011`. Reword
    its module doc from "with the ledger pre-seeded through 0010 so 0011
    is the sole pending migration" to "with every other migration
    pre-seeded in the ledger, so 0011 is the sole pending migration".
  - Then run the migrate-cli suite and fix any remaining exact-output
    assertion (for example `tests/fixtures/*/stdout`) that the extra
    migration changes. A fixture whose ledger leaves 0012 pending now
    reports one more applied migration.

#### 6. Changelog

**File**: `CHANGELOG.md`, `[Unreleased]` → `### Migrations`
**Changes**: add an entry after the 0011 one.

> - **Run `/accelerator:migrate` (migration 0012) to add `tags: []` to work
>   items written without it.** It inserts one line, before `last_updated`
>   or at the end of the frontmatter, changes nothing else, and realigns
>   the sync baselines of the items it repairs. It leaves alone, and names
>   with the fix each needs, any file that is not UTF-8, whose frontmatter
>   doesn't parse, whose `tags` key is spelled unusually (such as
>   `"tags":`), or whose repair would change another value. Upgrade
>   everyone who writes work items before running it. To repair items
>   written without `tags` afterwards, run
>   `accelerator migrate --unapply 0012-backfill-work-item-tags`, then
>   `/accelerator:migrate`.

### Success Criteria:

#### Automated Verification:

- [x] The `m0012` unit tests fail before the code they drive exists and pass
      after:
      `cargo nextest run --manifest-path cli/Cargo.toml -p migrate m0012`
- [x] The `read_text` adapter tests fail before the method exists and pass
      after:
      `cargo nextest run --manifest-path cli/Cargo.toml -p migrate-adapters read_text`
- [x] The `migration_0012` end-to-end tests fail before registration and
      pass after:
      `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-migrate -E 'binary(migration_0012)'`
- [x] The whole migrate-cli suite passes after the registry updates:
      `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-migrate`
- [x] The CLI workspace suite passes: `mise run test:unit:cli`
- [x] The public-API snapshot matches: `mise run public-api:check`
- [x] The architecture rules hold (`migrate` imports only `std`, `kernel`,
      `crate`, `corpus`): `mise run test:integration:pup`
- [x] Format and lint pass: `mise run cli:check`
- [x] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [x] In a scratch repository holding a work item without `tags` (for
      example one written by the pre-fix `work create`),
      `/accelerator:migrate` previews `0012-backfill-work-item-tags`,
      applies it, and `jj diff` shows exactly one added `tags: []` line per
      repaired item.
- [x] Running `/accelerator:migrate` in this repository adds `0012` to the
      ledger and changes no file under `meta/work/`.
- [x] A second `/accelerator:migrate` reports no pending migrations.
- [x] In a scratch repository with pulled items that lack `tags`, a
      `/sync-work-items` preview after `/accelerator:migrate` reports the
      repaired items as `Synced`, with no `LocallyModified` or `Conflict`.

These ran against the debug `accelerator-migrate` and `accelerator-work`
built by `mise run`, because the installed plugin predates 0012. The
pre-fix items came from the installed `work create` and `work sync`:
- Scratch repository: of an untagged and a tagged item, only the untagged
  one gained `tags: []`, and it then validated.
- This repository's ledger stops at 0010, so a real run here would also
  apply 0011, which edits `.accelerator/.gitignore`. The second and third
  checks therefore ran on a scratch copy of `meta/` and `.accelerator/`
  with 0011 marked applied. 0012 changed none of the 384 work items, and
  a second run reported no pending migrations.
- Sync: PP-23 and PP-24 were pulled from Linear with `--pull-only
  --target`, so nothing was sent to Linear, and both lacked `tags`. After
  the migration both stayed `synced` under `--preview`. With the
  pre-migration `last-sync.json` put back, both read `locally-modified`,
  so the realignment is what keeps them in sync.

**Implementation Note**: After completing this phase and all automated
verification passes, pause here for manual confirmation from the human that
the manual testing was successful.

---

## Testing Strategy

### Unit Tests:

- `compose_frontmatter` emits `tags` as an empty sequence directly before
  `last_updated`, and still omits empty linkage.
- `compose_frontmatter` writes every field the `corpus` schema requires of a
  work item (`BASE_FIELDS` plus the work-item row's `required_extras`) when
  everything optional is empty.
- `with_empty_tags`:
  - inserts before `last_updated`, or before the closing fence when there is
    none;
  - doesn't mistake `last_updated_by` for `last_updated`;
  - keeps CRLF endings and a missing final newline;
  - treats flow, empty and block `tags` as present;
  - ignores body lines, other document types, untyped items, unfenced
    content and unterminated fences;
  - repairs an unquoted `type: work-item`;
  - is idempotent;
  - produces output that validates.
- `Migration0012::apply`:
  - resolves `paths.work` and refuses empty or unsafe values, with the full
    `is_dangerous_path` table;
  - reaches nested directories such as `drafts/`;
  - writes only repaired files;
  - leaves files that are not UTF-8 alone and names them, while still
    repairing a valid item that holds U+FFFD;
  - leaves alone and names an item whose YAML already holds a
    non-canonical `tags` key, whose frontmatter doesn't parse, or whose
    insertion would change another value, and parses only the items
    offered a repair;
  - realigns the sync baselines of what it wrote;
  - restores every repaired item when a write or the realignment fails,
    including a write that landed before failing;
  - never overwrites an item that changed after its repair, and names it
    and any item it couldn't write back;
  - counts a restoring write that lands and then fails as restored;
  - names the stage that failed, puts any by-hand fix before the
    commit-and-re-run step, and leaves out a zero restored count;
  - returns `Applied` straight away when there is no work directory, and
    `Applied` even when nothing changed.

### Integration Tests:

- `work create`: untagged, and tagged with `relates_to`. Checks the
  adjacency, the absence of empty linkage keys, and validation.
- `create-batch`: untagged and tagged manifest entries.
- The sync importer: a pulled issue carries `external_id`, then `tags: []`,
  then `last_updated`, and validates.
- `accelerator-migrate` end to end: backfill, no-`last_updated` placement,
  byte-identical tagged items, body `tags:` ignored, drafts repaired, a
  no-op re-run, byte stability on re-application, a synced item's baseline
  realigned with its count reported, a CRLF item repaired through the real
  YAML guard, non-UTF-8, quoted-`tags`-key and unparseable items left
  byte-identical and named, the repaired count reported, a missing work
  directory and a work path that is a file both warned about, and a failed
  write leaving the work directory as it was. The idempotence tests first
  check that the first run repaired the fixture, so they fail before
  registration.
- `FileMigrationContext::read_text`: UTF-8 content verbatim, non-UTF-8
  bytes reported as `NotUtf8`, and a missing file as `None`. All three
  fail first, against the port's "not implemented" default.

### Manual Testing Steps:

1. `accelerator work create "Example" story medium` in a scratch
   repository, then `accelerator corpus frontmatter validate --file <path>`.
   Expect exit 0.
2. Remove `tags: []` from that file by hand, run
   `accelerator migrate --unapply 0012-backfill-work-item-tags`, then run
   `/accelerator:migrate`. Expect the line to come back with no other diff.
3. Run `/accelerator:migrate` again. Expect "No pending migrations."

## Performance Considerations

The migration reads every `.md` file under the work directory once and
writes only the files it repairs. That is the same cost as earlier
work-directory migrations (m0005). `parse_entries` runs over the
frontmatter slice only. The YAML parses (the original and the repaired
content) run only for files offered a repair, so their cost grows with the
number of repairs, not with the size of the work directory.

## Migration Notes

- The ID `0012-backfill-work-item-tags` must sort after every registered ID,
  because the SessionStart lag check compares IDs as strings
  (`cli/migrate-cli/src/discoverability.rs:39-57`). No other revision
  touches an `m0012*` file today. If another in-flight branch claims `0012`
  before this lands, renumber this one to the next free ID, along with its
  module, ledger entries, test file and CHANGELOG line.
- The migration only inserts lines. Rolling back is by reverting the
  migration commit in VCS, as for every other migration.
- Stragglers: items written without `tags` after 0012 has run, for example
  by a teammate on an older plugin version, can be repaired by running
  `accelerator migrate --unapply 0012-backfill-work-item-tags` and then
  `/accelerator:migrate`. `--unapply` removes only that ID from the ledger,
  under the run lock. Don't edit the ledger by hand. The migration is
  idempotent, so a re-run only touches items still lacking `tags`.
- A failed run restores the items it had repaired, so a re-run starts
  afresh. Recover in this order:
  1. Remove the `tags: []` line from any item the error names. Don't
     restore it from version control, which would also undo an earlier
     migration's rewrite of the file.
  2. Commit everything the run changed, including any `last-sync.json`,
     rather than discarding it. Committing only part of it moves the run
     base, so the pre-flight refuses the rest. Discarding `last-sync.json`
     in a run that also applied 0008 would throw away 0008's recorded
     realignment. Entries 0012 already advanced converge on the re-run.
  3. Re-run `/accelerator:migrate` before syncing, and push only after it
     succeeds, so teammates never pull a half-realigned baseline.

## References

- Original work item: `meta/work/PP-886-work-item-writers-omit-the-required-tags-field.md`
- Work item review: `meta/reviews/work/PP-886-work-item-writers-omit-the-required-tags-field-review-1.md`
- Codebase research: `meta/research/codebase/2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field.md`
- Decisions: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`,
  `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`
- Migration precedents: `cli/migrate/src/migrations/m0005.rs:33-108`
  (work-directory resolution), `cli/migrate/src/migrations/m0011.rs:192-297`
  (context double), `cli/migrate-cli/tests/migration_0010.rs` (end-to-end
  pattern)
- Worked migration plan:
  `meta/plans/2026-10-11-PP-884-sync-pulls-of-tracked-work-items-fail-silently.md`
