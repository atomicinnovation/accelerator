---
type: "pr-review"
id: "246-review-1"
title: "[PP-886] Always write tags on new work items and backfill missing ones"
date: "2026-11-19T16:47:35+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:246"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "compatibility", "documentation", "standards"]
review_number: 1
pr_number: 246
tags: []
last_updated: "2026-11-19T16:47:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #246 - [PP-886] Always write tags on new work items and backfill missing ones

**Verdict:** COMMENT

The writer fix is small and lands in the right place: one unconditional push
in `compose_frontmatter`, held to the corpus schema by a test that reads the
required set from the schema. Migration 0012 is careful. Its byte-offset
insertion is checked against a real YAML parse, rollback only ever writes back
from a known state, and the correctness lens traced every path without finding
a defect. Two gaps are worth fixing before merge:

- Baseline realignment silently skips Shortcut integrations, which the head now
  ships.
- An out-of-repository `paths.work`, which the product supports, makes the
  migration fail hard on every run.

Beyond those, the rollback covers work items but not baselines, and the test
suite has several minor gaps.

> **Review scope.** GitHub can't render this PR's diff (about 39k lines across
> 290 files). The head contains `main`'s Shortcut merge (#215) and a newer #234
> history that the base branch `0351-preview-pending-push-refusals` on origin
> lacks. This review covers only the 24 PP-886 commits
> (`f8671d6a0e79..bf87e4340c62`, 58 files).

### Cross-Cutting Themes

- **Rollback restores work items but not sync baselines** (flagged by:
  architecture, safety, code-quality, correctness). `realign_baselines` writes
  `last-sync.json` one entry at a time. If realignment fails partway, some
  baselines already point at the repaired digests while `restore_after` puts
  the items back to their original bytes. Correctness shows this converges if
  0012 is re-run. Safety notes that a persistent cause leaves it inconsistent,
  with `--skip` the only escape. Related points: `Repair::restore` reports
  "already original" and "restored" as the same `Ok(())`, which inflates the
  "left N item(s)" count, and the rollback runs inside a `map_err` closure. The
  migration also brings in its own recovery model next to 0008's "revert the
  commit".
- **Out-of-repository `paths.work` blocks every migrate run** (flagged by:
  compatibility, safety). `configured_work_dir` returns a hard error where
  m0010 warns and skips. `bounded_roots.rs` and the 0011 CHANGELOG support an
  outside root set in `config.local.md`.
- **Knowledge copied by hand into tests** (flagged by: code-quality,
  test-coverage). The full migration ledger is copied into about 30 test sites,
  and `already_applied_except_0012` adds another. The 0010 suite now also runs
  0011 and 0012. The writer-test helpers, including a hand-kept
  `OMIT_WHEN_EMPTY` list, are copied word for word between `src/test_support.rs`
  and `tests/common/mod.rs`.
- **Repairing stragglers goes through `--unapply`** (flagged by:
  architecture, compatibility, documentation). This turns a one-time migration
  into a repair tool people run repeatedly, and reorders the committed ledger
  once 0013 exists. No user-facing doc describes `--unapply`.

### Tradeoff Analysis

- **Recovery that converges vs recovery that is atomic**: correctness shows a
  partial realignment followed by a restore converges on re-run. Safety and
  architecture point out that this depends on the operator following the
  `RECOVERY` text and on the cause being transient. Recommendation: snapshot
  each integration's `last-sync.json` before 0012 realigns and write the
  snapshots back on `Failure::Realigning`. It is cheap and makes "leaves the
  tree as it found it" true for baselines too.
- **Module doc as rationale vs restated mechanics**: documentation praises the
  m0012 module doc for recording non-obvious reasons. Standards says lines 9-12
  restate what `yaml_obstacle` and the `Untouched` variants already say by
  name. Recommendation: keep the why (the scanner reads only bare column-0
  keys) and drop the restated conditions.
- **One immutable file vs a directory module**: code-quality suggests splitting
  m0012's ~470 production lines into an `m0012/` directory, following m0007.
  That doesn't conflict with treating migrations as immutable history, so it
  is a low-cost improvement if the module is touched again before merge.

### Strengths

- ✅ The writer fix is made once at the composition point all three writers
  share (`compose_frontmatter`). `writes_every_field_the_schema_requires_when_all_else_is_empty`
  derives the required set from `BASE_FIELDS` plus `required_extras`, so a
  future required field fails at the writer.
- ✅ `yaml_obstacle` turns any disagreement between the column-0 scanner and
  the YAML parser into "leave the file byte-identical and name it". The insertion
  always lands on a line boundary with a real line ending, so CRLF files and
  files without a final newline survive.
- ✅ `Repair::restore` only writes back from a known state. Recording before
  writing correctly covers a rename that lands and then errors.
- ✅ `TextFile::{Utf8, NotUtf8}` makes strict versus lossy decoding a type,
  not a convention. The erroring default stops a partial context from
  passing for a missing file.
- ✅ `realigned_entry` only advances baselines whose digest matched the
  pre-migration bytes, so genuine local edits keep their pending push.
- ✅ The tests are thorough. A purpose-built fake models writes that never
  land, writes that land and then fail, and edits made after a repair. End-to-end
  tests run CRLF through the real YAML guard, use a sealed directory to force a
  rollback, and check idempotence and byte stability. Each of the three writer
  paths has its own test.
- ✅ Doc comments that would have gone stale ("Only migration 0008 calls
  this") are removed, and the realignment docs are made general.
- ✅ `tags` never enters a tracker push payload or synced-field digest, so
  emitting `tags: []` cannot clear remote labels. Skipping a `schema_version`
  bump is correct.

### General Findings

- 🔵 **Code Quality**: m0012.rs combines five concerns (path validation,
  byte-level insertion, the YAML guard, rollback and its messages, untouched
  reporting) in a single 1,400-line file. m0007's directory module is a
  precedent for splitting it.
- 🔵 **Architecture**: repairing stragglers routes through `--unapply` and a
  re-run. If more required-field backfills come along, consider a corpus-side
  fixer (for example a `--fix` mode for `MISSING-BASE-FIELD`).
- 🔵 **Compatibility**: teams on mixed plugin versions have to unapply and
  re-apply by hand. At minimum, mention `corpus frontmatter validate` as the
  way stragglers surface.
- 🔵 **Standards**: the `MigrationContext` convention note (attached to
  `VersionControl` at ports.rs:66-69) still says capability methods "default to
  the empty/absent case", with no exception for `read_text`.
- 🔵 **Test Coverage**: the write that lands and then fails is shown only
  against the fake. No end-to-end test makes the real manifest append fail.
- 🔵 **Test Coverage**: `InsertionChangesMeaning` is never triggered with the
  real YAML parser. A multi-line flow mapping with a column-0 `last_updated:`
  inside it would trigger it.

### Additional Findings

- 🔵 `cli/migrate/src/migrations/m0012.rs:58` — The rollback runs inside a
  `map_err` closure (code-quality)
- 🔵 `cli/migrate/src/migrations/m0012.rs:59` — A third recovery model sits
  alongside 0008's VCS revert (architecture)
- 🔵 `cli/migrate/src/ports.rs:195` — The lossy `read` still looks like the
  default, and its loud-failure default is untested (architecture,
  test-coverage)
- 🔵 `cli/migrate-cli/tests/migration_0010.rs:2` — The 0010 suite also runs
  0011 and 0012 on every fixture (test-coverage)
- 🔵 `cli/work/src/create.rs:76` — The `compose_frontmatter` doc singles out
  `tags`, narrating the change (standards)
- 🔵 `cli/migrate/src/migrations/m0012.rs:9` — The module doc restates the YAML
  guard's mechanics (standards)
- 🔵 `meta/prs/246-description.md:166` — The description says "14
  `migration_0012` end-to-end tests"; the file has 13 (documentation)
- 🔵 `cli/migrate/src/migrations/m0012.rs:226` — In the recovery text, "don't
  push yet" is ambiguous between `git push` and a tracker sync (safety)
- 🔵 `cli/migrate/src/migrations/m0012.rs:289` — The record-before-write
  ordering isn't expressed by a name (code-quality)
- 🔵 `cli/migrate/src/migrations/m0012.rs:1277` — A test literal is 83
  columns, over the 80 limit (standards)
- 🔵 `cli/migrate/src/migrations/m0012.rs:213` — `Failure::stage()` and
  `RECOVERY` name less than they hold (standards)
- 🔵 `cli/migrate/src/ports.rs:87` — `TextFile` is inserted between
  `VersionControl` and `TrackedState` (standards)
- 🔵 `cli/migrate/src/migrations/m0012.rs:443` — The offset-zip in
  `tags_insertion` takes several reads to follow (code-quality)
- 🔵 `skills/work/create-work-item/SKILL.md:636` — The "only keys it leaves
  out" list is contradicted by the `aliases` sentence that follows
  (documentation)
- 🔵 `CHANGELOG.md:1239` — The CHANGELOG points users to `migrate --unapply`,
  which no user-facing doc describes (documentation)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/migrate-cli/src/sync_baselines.rs:1-2` — 0012's baseline realignment skips Shortcut integrations
**Severity**: major | **Confidence**: high | **Lens**: safety

🟡 **Safety**

This module now serves any migration that rewrites work items, but
`body_rules` still maps only `jira` and `linear`. Its test asserts that `"no
repository holds Shortcut baselines older than the migration"`. That was true
for 0008, but it isn't for 0012: the head ships Shortcut sync (#215), and
Shortcut pulls went through the same `render_frontmatter`, which omitted
`tags` before this fix. `realign_baselines` skips any directory with no
dialect (`cli/work-adapters/src/sync/realign.rs:82-84`). As a result,
`integrations/shortcut/last-sync.json` is never realigned, and every tag-less
Shortcut item that 0012 repairs reads as `LocallyModified` on the next sync.

**Impact**: Shortcut users on prerelease builds get a redundant push of every
repaired item. Where the remote has also changed, they get a spurious conflict
instead of a clean pull. This is the misclassification the PR says
realignment prevents.

**Suggestion**: Map `"shortcut"` to its body dialect in `body_rules`, update
the test and its message, and add a 0012 end-to-end case with a Shortcut
baseline.

*(The lens rated this minor. It was raised to major after `body_rules` and
the skip in `realign_baselines` were checked in the code.)*

---

### `cli/migrate/src/migrations/m0012.rs:87-91` — 0012 hard-fails when `paths.work` is outside the repository
**Severity**: major | **Confidence**: medium | **Lens**: compatibility, safety

🟡 **Compatibility / Safety**

`configured_work_dir` returns an `Err` for any absolute or `../` `paths.work`.
That configuration is supported: `work-cli/src/bounded_roots.rs` accepts an
outside root set in an untracked `config.local.md`, and the 0011 CHANGELOG
tells users to move it there. Writers have been producing tag-less items in
that directory. On upgrade, `/accelerator:migrate` stops at 0012 with
`refusing dangerous paths.work value`. No later migration runs, the
SessionStart reminder keeps firing, and the error doesn't mention `--skip`.
m0010 handles the same check by warning and skipping (`m0010.rs:88-93`).

**Impact**: A supported configuration makes every migrate run fail.

**Suggestion**: Warn and return `Applied` as m0010 does. Name the directory and
the manual fix (add `tags: []` to each work item that lacks it). Add a test for
an absolute `paths.work` that asserts the run continues.

---

### `cli/migrate/src/migrations/m0012.rs:59-66` — A partial realignment isn't rolled back when the items are
**Severity**: minor | **Confidence**: high | **Lens**: safety, architecture

🔵 **Safety / Architecture**

`BaselineStore::set` rewrites `last-sync.json` once per entry, and
`realign_baselines` walks integrations one at a time. A failure partway
through leaves the earlier entries pointing at the *repaired* digests, while
`restore_after` puts the items back to their original bytes. Those items read
as `LocallyModified` until a later 0012 run succeeds. A re-run does converge,
because `realigned_entry` skips the entries already advanced. But if the cause
persists (for example one unwritable baseline), every retry ends in the same
state, and `--skip` leaves it there for good. The `RECOVERY` text also builds
the adapter's `last-sync.json` file name into the `migrate` domain crate.

**Impact**: Repaired items can sit with mismatched baselines indefinitely,
causing redundant pushes or spurious conflicts.

**Suggestion**: Snapshot each integration's `last-sync.json` just before
0012's realignment and write the snapshots back on `Failure::Realigning`.
Alternatively, make realignment all-or-nothing behind the `SyncBaselines` port.
Either way, a failed 0012 then leaves items and baselines as it found them, and
the message can say "sync baselines" rather than naming the file.

---

### `cli/migrate/src/migrations/m0012.rs:179-194` — `Repair::restore` merges distinct outcomes and can lose the write error
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, correctness, test-coverage

🔵 **Code Quality**: `restore` returns the same `Ok(())` for three different
cases: the file never changed (its write didn't land), it was restored, or it
was restored by a write that then reported failure. It also returns the same
`MigrationError` for "changed after the repair" and for real I/O failures. As
a result, `restore_after` counts a write that never landed as one of the "Left
the N work item(s) it had repaired". Consider returning an enum such as
`Restoration { AlreadyOriginal, Restored, ChangedSinceRepair, Failed(..) }`.

🔵 **Correctness**: when the restoring write fails, the `?` on
`self.holds(ctx, &self.original)` returns the follow-up *read* error if that
read also fails. The original write error is dropped, so the reason shown for
an unrestored item names the wrong problem. Only use the re-check to upgrade
the result to success:
`if written.is_err() && self.holds(ctx, &self.original).unwrap_or(false)`.

🔵 **Test Coverage**: no test covers `read_text` returning `Err` during
restore, or a repaired file deleted before restore (`Ok(None)`).

---

### `cli/migrate/src/migrations/m0012.rs:336-340` — Frontmatter that isn't a mapping: misleading message, and the branch is untested
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, test-coverage

🔵 **Code Quality**: frontmatter that parses but isn't a mapping is reported
through `Untouched::UnparseableFrontmatter` with a synthetic
`MigrationError`. The user sees "frontmatter does not parse as YAML (the
frontmatter is not a mapping)", which contradicts itself. Add an
`Untouched::FrontmatterNotAMapping` variant with its own fix, or reword the
message to cover both cases.

🔵 **Test Coverage**: no test reaches this arm, because every `parsing(...)`
stub returns a `Mapping` or an `Err`. Changing it to `return None` would
pass. Add a unit test with
`.parsing(&item, Ok(FrontmatterValue::Sequence(vec![])))` that asserts the
variant and that nothing was written.

---

### `cli/migrate/src/migrations/m0012.rs:1133-1152` — A test's name contradicts half of what it asserts
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, test-coverage

🔵 **Code Quality / Test Coverage**

`a_failure_before_any_repair_omits_the_left_count` runs two scenarios. The
unlanded-write case asserts the message *contains* `"Left the 1 work
item(s)"`. Only the failed-listing case omits the count. The first scenario is
the clearest unit-level check on record-before-write, and the name hides it,
so someone could "fix" the assertion and weaken that guard. Split it, for
example into `a_write_that_never_landed_is_still_counted_as_left_as_found` and
`a_listing_failure_omits_the_left_count`. Better still, fix the counting (see
the `Repair::restore` comment).

---

### `meta/prs/246-description.md:33-34` — "This diff shows only the 0333 work" is inaccurate
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

GitHub can't render #246's diff at all: it is about 39k lines across 290
files. The head contains `main`'s Shortcut merge (#215) and a newer #234
history that the base branch on origin lacks. The description also contradicts
itself, since a later section says #234's renamed documents "appear in this
diff". Either update the base branch so the claim holds, or say what the GitHub
diff contains and name the focused range (`f8671d6a0e79..bf87e4340c62`).
Use PP-886 rather than 0333 throughout.

---

### `cli/migrate/src/migrations/m0012.rs:834-842` — The empty-path test passes for any error that mentions 0012
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`refuses_an_empty_work_path` asserts only that the message contains `"0012"`.
`is_dangerous_path("")` is also true, so the test would still pass if the
explicit `configured.is_empty()` check were deleted, as it would for any
other 0012 failure. Assert
`message.contains("config path returned empty for 'work'")`. Consider also
covering `config_value` returning `Ok(None)`.

---

### `cli/migrate-cli/tests/migration_0012.rs:94-110` — Yet another hand-copied full migration ledger
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, test-coverage

🔵 **Code Quality / Test Coverage**

`already_applied_except_0012` hard-codes all eleven earlier IDs. This PR also
had to append 0012 to about 30 literal ledgers across the migrate-cli suites.
A copy that's missed doesn't fail: the newer migration just runs inside an
unrelated suite, which is already happening in the 0010 suite. Add
`mark_all_migrations_applied_except(root, id)` to `tests/common/mod.rs`, built
from `migrate::registry::registry()` IDs, and use it here. Migrating the other
suites can be follow-up work.

---

### `cli/work-cli/tests/common/mod.rs:42-68` — Writer-test helpers copied word for word, with a hand-kept omit-when-empty list
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, test-coverage

🔵 **Code Quality / Test Coverage**

`OMIT_WHEN_EMPTY`, `frontmatter_of`, `omit_when_empty_lines` and `violations`
are identical in `src/test_support.rs` and here. `OMIT_WHEN_EMPTY` also
restates which keys `compose_frontmatter` omits when empty. If a key is added
to one copy but not the other, the `== []` assertions keep passing without
checking it. Include one copy from the other with `#[path = ...] mod`, as
`keyword_surface.rs` does, or move the helpers into a small test-support crate.
Ideally, derive the key list from the production definition.

---

## Per-Lens Results

### Architecture

**Summary**: The architecture is sound. The writer fix sits at the one point
every writer passes through. A test derived from the schema holds the writer to
the corpus, and 0012 keeps a pure core behind the migration port while reusing
the existing realignment. The concerns: 0012 brings in its own in-process
rollback, a different recovery model from 0008's revert. The rollback doesn't
cover baselines that a failed realignment already advanced. And the port's
lossy read still looks like the default.

**Strengths**: The one-point writer fix. The writer test derived from the
schema. A clean dependency direction, with YAML behind the port and pure
scanner functions. A pure decision core that can be fully tested with a fake.
Reuse of the `realign_sync_baseline` port, with its docs widened. `read_text`'s
erroring default follows the convention for optional capabilities.
`TextFile` as a domain type.

**Comments**:
- `m0012.rs:59-66` (minor, medium): 0012 brings in its own rollback, a
  third recovery model alongside 0008's VCS revert. Move it into a shared write
  session on `MigrationContext`, or document why a revert is not enough (for
  example, drafts aren't tracked).
- `m0012.rs:226-229` (minor, medium): the rollback restores work items but
  not baselines a failed realignment already advanced, and the `migrate`
  domain message names the adapter's `last-sync.json`.
- `ports.rs:195-202` (suggestion, medium): the lossy `read` still looks like
  the default and strict `read_text` is the opt-in. Consider renaming it
  `read_lossy`.

**General Findings**:
- (suggestion) Repairing stragglers routes through `--unapply` and a re-run.
  Consider a corpus-side fixer if more backfills follow.

### Code Quality

**Summary**: The names are rich and the writer fix is one line. The risk
concentrates in m0012.rs: `restore` merges outcomes and inflates the count, a
test name contradicts its assertion, the rollback is hidden in `map_err`, and
five concerns share one file. The test helpers and ledgers are copied by hand.

**Strengths**: `TextFile` makes the decoding strategy explicit. The domain
vocabulary is rich (`Backfill`, `Repair`, `Untouched::*`), and each
`Untouched` reason carries its own fix. The writer fix is at the single
composition point. `is_original_plus_empty_tags` is readable. The fake context
makes the failure paths testable.

**Comments**:
- `m0012.rs:179-194` (minor, high): `Repair::restore` returns the same
  `Ok(())` for several outcomes, and the `?` can replace the write error.
- `m0012.rs:1134-1152` (minor, high): a test name contradicts its assertion.
- `m0012.rs:58-66` (minor, medium): the file restoration is hidden inside a
  `map_err`.
- `m0012.rs:336-340` (minor, high): frontmatter that isn't a mapping is
  reported as "does not parse as YAML".
- `m0012.rs:289-295` (suggestion, medium): the record-before-write ordering
  isn't expressed by the code.
- `m0012.rs:443-456` (suggestion, low): the offset-zip in `tags_insertion`
  takes several reads to follow.
- `work-cli/tests/common/mod.rs:42-68` (minor, high): helpers are copied
  word for word between `test_support.rs` and `common/mod.rs`.
- `migration_0012.rs:94-110` (minor, high): the full ledger is copied by hand
  again.

**General Findings**:
- (suggestion) m0012.rs combines five concerns in 1,400 lines. Follow the
  m0007 directory precedent.

### Test Coverage

**Summary**: Coverage is strong and in proportion to the risk. Every restore
state, the record-before-write ordering, CRLF, a missing final newline, no
`last_updated`, `drafts/` and idempotence are covered. Gaps: one branch of the
YAML guard, one loose assertion, a test name that misleads, and the ledger
copies.

**Strengths**: The fake models writes that never land, writes that land and
fail, and edits after a repair. Each guard refusal has a specific test.
Byte-level insertion is checked, including CRLF through the real guard. A real
U+FFFD is told apart from a non-UTF-8 file. The writer test is driven by the
schema. Each of the three writer paths is tested. The summary counts are
asymmetric, so swapped arguments would be caught. The sealed-directory test is
deterministic and checks recovery.

**Comments**:
- `m0012.rs:1133-1152` (minor, high): one test holds two scenarios with
  opposite expectations.
- `m0012.rs:336-340` (minor, high): the branch for frontmatter that isn't a
  mapping is untested.
- `m0012.rs:834-842` (minor, high): the empty-path test passes for any 0012
  error.
- `migration_0012.rs:94-110` (minor, high): another ledger copied by hand.
  Add `mark_all_migrations_applied_except`.
- `migration_0010.rs:2-3` (minor, medium): the 0010 suite now also runs 0011
  and 0012.
- `m0012.rs:183-193` (suggestion, medium): the restore path when the re-read
  fails or the file is gone is untested.
- `work-cli/tests/common/mod.rs:42-68` (suggestion, medium): duplicated
  helpers with a hand-kept key list.
- `ports.rs:195-202` (suggestion, medium): the loud-failure default of
  `read_text` is untested.

**General Findings**:
- (suggestion) A write that lands and then fails is shown only against the
  fake.
- (suggestion) `InsertionChangesMeaning` is never triggered with the real YAML
  parser.

### Correctness

**Summary**: No correctness defects. The insertion always lands on a line
boundary with a real line ending. The fence and key scanning match
`corpus::frontmatter_validation` and `document::fence_offsets`. The YAML guard
reliably turns disagreement into "leave alone". Restore, record-before-write
and partial-realignment recovery all converge. One minor issue: which error is
reported on a rare restore path.

**Strengths**: `tags_insertion` keeps line endings and a missing final
newline. The fence rules match the validator's split, including CRLF and a BOM.
`holds_key` needs an exact column-0 key. `yaml_obstacle` covers quoted keys,
split constructs and tagged YAML. Restore only writes from a known state.
Recovery from a failed realignment converges on re-run. The `read_text`
adapter maps NotFound, NotUtf8 and other errors correctly. `compose_frontmatter`
order matches the template, and no remaining writer drops `tags`.

**Comments**:
- `m0012.rs:189-193` (minor, medium): the re-check during restore can replace
  the write error with a read error.

**General Findings**: none.

### Safety

**Summary**: Careful with user data. The insertion is checked against YAML,
and any file the migration can't repair safely is left byte-identical and
named. Rollback writes only from a known state, and realignment never masks
genuine edits. Gaps: Shortcut baselines are skipped, a partial realignment
isn't rolled back, and an out-of-repository `paths.work` fails every run.

**Strengths**: A guard that fails safe. A rollback that only writes from a
known state. Record-before-write closes the gap where a write half-lands.
`read_text` refuses lossy decoding. `realigned_entry` keeps genuine local
edits. The `--unapply` route only touches the ledger and re-runs only the
idempotent 0012.

**Comments**:
- `sync_baselines.rs:1-2` (minor → raised to major on verification,
  medium): 0012's realignment skips Shortcut integrations.
- `m0012.rs:59-66` (minor, high): a partial realignment isn't rolled back
  when the items are.
- `m0012.rs:226-229` (suggestion, medium): "don't push yet" in the recovery
  text is ambiguous.
- `m0012.rs:87-91` (minor, medium): an out-of-repository `paths.work` fails
  the migration on every run.

**General Findings**: none.

### Compatibility

**Summary**: Mostly safe. `read_text` is a purely additive, defaulted trait
method. `tags: []` brings items up to the existing v1 shape. Tags never enter a
tracker push payload, and realignment keeps synced items `Synced`. The one
real gap: an out-of-repository `paths.work` makes 0012 fail hard.

**Strengths**: A defaulted trait method with the public-api fixture updated.
Tags are absent from push payloads and digests. Realignment reuses the 0008
mechanism, and diverged entries keep their baselines. The ledger only warns on
unknown IDs, so older collaborators are tolerated. Consumers treat `tags: []`
the same as a missing key. Skipping the `schema_version` bump is correct.

**Comments**:
- `m0012.rs:87-91` (major, medium): 0012 hard-fails for users whose
  `paths.work` is outside the repository.

**General Findings**:
- (minor) Repairing items for mixed-version teams means editing the committed
  ledger by hand.

### Documentation

**Summary**: The shipped docs match the code. The SKILL.md passages, Quality
Guidelines bullets and CHANGELOG entries are accurate, and the realignment
docs were made general correctly. The docs-site page lists no individual
migrations. Remaining issues: the PR description's scope statement and one test
count, an "only" in create-work-item that the next lines contradict, and
`--unapply` has no user-facing docs.

**Strengths**: The `compose_frontmatter` doc was cut to a one-line why. Stale
caller claims were removed. The `read_text` doc says what the code cannot. The
m0012 module doc records non-obvious reasons. The CHANGELOG Migrations entry is
actionable. The Quality Guidelines bullets separate required fields from
omit-when-empty ones. Each untouched-file note gives a concrete fix.

**Comments**:
- `meta/prs/246-description.md:33-34` (minor, high): the scope statement is
  inaccurate.
- `meta/prs/246-description.md:166` (minor, medium): the description says
  14 end-to-end tests; the file has 13.
- `skills/work/create-work-item/SKILL.md:636-644` (suggestion, medium): the
  "only keys" list is contradicted by the `aliases` sentence.
- `CHANGELOG.md:1239-1242` (suggestion, medium): `--unapply` is not
  documented anywhere a user would look.

**General Findings**: none.

### Standards

**Summary**: Close fit with the project's conventions. Message prefixes,
warning and note formats, the summary line, the inline `is_dangerous_path`,
and the registry, ledger and public-API updates all match m0005, m0010 and
m0011. Stale comments were removed. Small issues: a doc comment that narrates
the change, a module doc that restates code, one line over 80 columns, and a
couple of names that say less than they hold.

**Strengths**: Consistent with the conventions of earlier migrations. Comments
that would go stale were removed. The renamed test helpers say what they seed.
Domain vocabulary throughout, with no inline `//` comments. Registration is
complete.

**Comments**:
- `cli/work/src/create.rs:76-77` (minor, medium): the `compose_frontmatter`
  doc singles out `tags`.
- `m0012.rs:9-12` (minor, medium): the module doc restates the guard's
  mechanics.
- `m0012.rs:1277` (suggestion, medium): a test literal exceeds 80 columns (it
  is 83).
- `m0012.rs:213-229` (suggestion, low): `Failure::stage()` and `RECOVERY`
  name less than they hold.
- `ports.rs:87-91` (suggestion, low): `TextFile` is inserted between
  `VersionControl` and `TrackedState`.

**General Findings**:
- (suggestion) The convention note on `MigrationContext` defaults (attached to
  `VersionControl`) needs an exception for `read_text`.
