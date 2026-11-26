---
type: "plan-review"
id: "2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field-review-1"
title: "Plan Review: Work Item Writers Omit the Required Tags Field Implementation Plan"
date: "2026-11-18T18:53:33+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "compatibility", "documentation"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-11-18T23:47:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Work Item Writers Omit the Required Tags Field Implementation Plan

**Verdict:** REVISE

The plan is precise and well grounded. The writer fix sits in the one pure
domain function all three paths share, and the migration is split into a
pure, byte-preserving transform and a thin `apply`. The tests compare
literal inputs and outputs and run the validator in-process. One structural
gap stands out. Three lenses found it independently: migration 0012
rewrites the frontmatter of exactly the items that carry sync baselines,
but does not realign those baselines, which m0008 established as necessary.
Beyond that, the doc comment carries an inaccuracy, and the lossy-read
guard has no test proving it reports anything. Each fix is small and local
to the plan.

### Cross-Cutting Themes

- **Sync baselines are not realigned after the backfill** (flagged by:
  compatibility, safety, correctness). `tags` is not in
  `work::normalise::IGNORE_KEYS`. Most tag-less items came from the sync
  importer, so 0012 changes the frontmatter digest of nearly every synced
  item. On the next sync, split-recipe items show as `LocallyModified`.
  Legacy-recipe items whose remote body has moved become `Conflict`. m0008
  avoids this by calling `ctx.realign_sync_baseline(&pre_migration)`.
- **The U+FFFD guard rests on a heuristic that is neither tested nor
  stated** (flagged by: architecture, correctness, test-coverage,
  code-quality). `apply` infers a lossy decode by scanning for U+FFFD.
  Nothing tests that such files are actually reported. A valid file holding
  a literal U+FFFD, which m0008 may have left behind, is reported falsely
  and never repaired. The `unreadable` name also misdescribes the condition.
- **The one-shot migration and the writer fix must be released in order**
  (flagged by: architecture, compatibility). The phases are independent in
  code but not in system state. Once 0012 is recorded, any tag-less item
  written by an older or unfixed writer is never repaired.
- **The live `corpus` dependency conflicts with the "inline helpers"
  principle** (flagged by: architecture, code-quality). The plan cites
  immutable migration history but depends on live `parse_entries`. The
  choice is defensible, since it agrees with the validator, but the plan
  doesn't say so.
- **Omit-when-empty test helpers are duplicated** (flagged by:
  code-quality, test-coverage). `crate::test_support` already exists for the
  in-crate `create_batch` and `sync_author` tests.

### Tradeoff Analysis

- **Agreeing with the validator vs freezing behaviour**: taking presence
  from live `corpus` parsing keeps the migration and `MISSING-BASE-FIELD`
  in lockstep. Inlining a scanner would freeze behaviour for repositories
  that haven't applied 0012 yet. Recommendation: keep the corpus dependency
  (m0007 is a precedent) and add one sentence recording the deliberate
  choice.
- **Strict read at the port vs local heuristic**: a strict-read method on
  `MigrationContext` removes the U+FFFD false positive and lets later
  migrations reuse it, but it widens the port. Recommendation: either works.
  If the port stays as it is, name the false positive explicitly and word
  the note as "contains replacement characters" rather than "could not be
  read".

### Findings

#### Major

- 🟡 **Compatibility / Safety / Correctness**: Migration 0012 rewrites
  synced items without realigning their sync baselines
  **Location**: Phase 2, Section 2: `apply` over the work directory
  `tags` is not in `IGNORE_KEYS`, so inserting `tags: []` changes the local
  frontmatter digest and the legacy whole-file hash of every synced item it
  repairs. Split-recipe items become `LocallyModified`. Legacy items whose
  remote body has moved become `Conflict`, and choosing "local" there could
  push a stale body over a remote edit. m0008 collects `pre_migration` and
  calls `ctx.realign_sync_baseline`. 0012 should do the same.
- 🟡 **Documentation**: The rewritten doc comment keeps a false claim about
  when `external_id` is written
  **Location**: Phase 1, Change 6
  "`external_id` is `Some` only after a successful `create --push`" ignores
  the sync importer, which passes `Some(issue.external_id)`. The plan's own
  Change 4 test asserts this.
- 🟡 **Test Coverage**: Nothing tests that lossy-read items are reported, or
  that real invalid UTF-8 survives byte for byte
  **Location**: Phase 2, Sections 2 and 4
  Deleting `unreadable.push(file)`, or the `eprintln!` of the note, passes
  every planned test, and the real `from_utf8_lossy` adapter is never
  exercised. An end-to-end test with a raw `\xFF` byte would pin the
  CHANGELOG's promise.

#### Minor

- 🔵 **Correctness**: A write failure part-way leaves earlier repairs with
  stale baselines that a re-run cannot recover
  **Location**: Phase 2, Section 2
  With realignment added after the loop, a `?` on the Nth write skips the
  realignment for files 1 to N-1. On a re-run those files are already
  repaired, so their pre-migration bytes are never captured again.
- 🔵 **Correctness**: Valid UTF-8 files containing U+FFFD are treated as
  unreadable and never repaired
  **Location**: Phase 2, Section 2 (U+FFFD guard)
  m0008 may have written literal U+FFFD back into files. 0012 then reports
  those files falsely, every time, and 0012 is recorded as applied.
- 🔵 **Correctness**: Unusual spellings of the `tags` key get a duplicate
  `tags: []`
  **Location**: Phase 2, Section 1
  `"tags": [...]` or `tags : [...]` is invisible to `parse_entries`, so the
  migration adds a second key, which is invalid YAML for `document::parse`.
  Rare, and low confidence.
- 🔵 **Test Coverage**: Two of the "red" tests already pass before the fix
  **Location**: Phase 1, Success Criteria and Section 4
  The populated-tag tests for criteria 2 and 4 pass today. The fail-first
  filter `test(/empty_tags|tags_follow|tagged_entry/)` overstates the red
  step.
- 🔵 **Test Coverage**: Recursion into `drafts/` is relied on but untested
  **Location**: Phase 2, Sections 2 and 4
- 🔵 **Test Coverage**: The inlined `is_dangerous_path` copy is only partly
  exercised
  **Location**: Phase 2, Section 2
  m0005's precedent tests seven dangerous forms. The plan tests two.
- 🔵 **Test Coverage**: The type gate's boundaries (no `type`, unquoted
  `type`) are untested
  **Location**: Phase 2, Section 1
- 🔵 **Code Quality**: Omit-when-empty test helpers are duplicated despite
  `crate::test_support`
  **Location**: Phase 1, Sections 2 to 4
  The two `frontmatter_of` variants also differ, and `unwrap_or_default`
  would make the absence assertions pass vacuously.
- 🔵 **Code Quality**: `with_empty_tags` mixes fence location, the repair
  decision and splicing through dense index arithmetic
  **Location**: Phase 2, Section 1
- 🔵 **Code Quality**: Migration output is quieter than the m0005/m0010
  precedent and says nothing about a missing work directory
  **Location**: Phase 2, Section 2
- 🔵 **Architecture**: The one-shot migration must not ship ahead of the
  writer fix
  **Location**: Implementation Approach
- 🔵 **Architecture**: No structural guard ties the writer's always-emitted
  keys to corpus `BASE_FIELDS`
  **Location**: Phase 1
- 🔵 **Compatibility**: No test covers how the migration interacts with an
  item that has a sync baseline
  **Location**: Phase 2, Section 4 / Manual Verification
- 🔵 **Compatibility**: Teams on mixed plugin versions can keep creating
  tag-less items after 0012 is recorded
  **Location**: Migration Notes
- 🔵 **Documentation**: The new create-work-item passage contradicts itself
  about which keys are left out
  **Location**: Phase 1, Change 7
- 🔵 **Documentation**: m0012 has no module doc, unlike every existing
  migration
  **Location**: Phase 2, Section 1
- 🔵 **Documentation**: The CHANGELOG entries use an internal name and don't
  link the fix to its migration
  **Location**: Phase 1, Change 8; Phase 2, Change 5
- 🔵 **Documentation**: The Migrations entry describes only the
  `last_updated` placement
  **Location**: Phase 2, Change 5

#### Suggestions

- 🔵 **Architecture / Code Quality**: Record that the live corpus
  dependency is deliberate, and name the m0010 variant of
  `is_dangerous_path` as the one to inline
  **Location**: Key Discoveries / Phase 2, Section 1
- 🔵 **Architecture**: Note that the U+FFFD sniff is a deliberate local
  workaround with a known false positive, or add a strict read to the port
  **Location**: Phase 2, Section 2
- 🔵 **Code Quality**: Rename `unreadable` / `unreadable_note` to say what
  the condition is (lossily decoded)
  **Location**: Phase 2, Section 2
- 🔵 **Documentation**: Cut the `compose_frontmatter` doc comment to the
  non-obvious *why*
  **Location**: Phase 1, Change 6
- 🔵 **Test Coverage**: Add an end-to-end re-application test, modelled on
  m0010's byte-stability test
  **Location**: Phase 2, Section 4
- 🔵 **Test Coverage**: Make the sync test's name, or the plan's claim,
  match the unmodelled-labels caveat
  **Location**: Phase 1, Section 4

### Strengths

- ✅ The fix lands in the pure domain function `compose_frontmatter`, which
  `work create`, `create-batch` and the sync importer all reach through
  `render_frontmatter`. One change fixes all three paths.
- ✅ Migration 0012 is a pure core (`with_empty_tags`) inside an imperative
  shell (`apply`), so every byte-level edge case is unit-testable without
  filesystem setup.
- ✅ Presence and type come from `corpus::frontmatter_validation`, so the
  migration cannot disagree with `MISSING-BASE-FIELD`. This stays inside
  the `migrate` crate's pup boundary.
- ✅ The byte-offset splice keeps CRLF, a missing final newline and a
  closing fence at EOF, and the exact-key match avoids the
  `last_updated_by` trap.
- ✅ Returning `ApplyOutcome::Applied` unconditionally follows the
  lifecycle contract. The transform is idempotent, so an interrupted run
  can be retried as far as file content goes.
- ✅ Each writer path gets a boundary test that runs the real validator
  in-process, and the omit-when-empty checks match whole line prefixes
  within the frontmatter only.
- ✅ The plan lists the hard-coded registry and ledger tests, includes a
  catch-all step for exact-output assertions, and gives a renumbering
  procedure if another branch claims `0012`.
- ✅ The scope is tightly bounded: no label mapping, reordering,
  `schema_version` bump or template change, and generated docs-site pages
  are explicitly excluded.
- ✅ The stale doc comment and both SKILL.md passages are corrected in the
  same phase as the code.

### Recommended Changes

1. **Realign sync baselines in `apply`** (addresses: the sync-baseline
   finding; the write-failure finding; no test covering synced items)
   Collect `(file, original)` for each file written, and call
   `ctx.realign_sync_baseline(&pre_migration)` on both the success and
   error paths. One way is to gather the result of the write loop, realign,
   and then propagate. Report the realigned count in stderr. Add to the
   context double a recorder for realign calls and a `write` that can fail,
   plus unit tests for both. Add an end-to-end case that seeds a
   `last-sync.json` entry for a tag-less synced item and asserts it is
   realigned. Update the `realign_sync_baseline` port doc in
   `cli/migrate/src/ports.rs`, which says only 0008 calls it.
2. **Fix the `compose_frontmatter` doc comment** (addresses: the false
   `external_id` claim; the restates-the-code suggestion)
   Drop the `external_id` sentence, or reword it to cover both writers.
   Consider cutting the comment to what the code cannot say: fields are in
   template order, and `tags` is always written as a required base field.
3. **Pin the lossy-read behaviour end to end, and settle the false
   positive** (addresses: no test for lossy-read reporting; the
   valid-U+FFFD finding; the naming suggestion; the port-workaround
   suggestion)
   Add an end-to-end test that writes raw `\xFF` into a tag-less item and
   asserts the file is byte-identical, stderr names its path, and 0012 is
   recorded. Either add a strict read to the port, or rename to
   `lossily_decoded` and word the note as "contains replacement
   characters".
4. **State the release ordering and how to repair stragglers** (addresses:
   the one-shot migration finding; the mixed-versions finding)
   In Implementation Approach and Migration Notes, say that Phase 2 must
   ship with Phase 1 or after it. Add the straggler procedure (remove 0012
   from the ledger and re-run) to the Migration Notes, and possibly to the
   CHANGELOG.
5. **Correct the red-step claims** (addresses: the already-passing red
   tests)
   Mark the criteria 2 and 4 tests, and the renamed linkage test, as
   characterisation pins. Narrow the fail-first filter to the
   `empty_tags` tests and the renamed unit test.
6. **Widen the migration's unit and end-to-end coverage** (addresses:
   `drafts/` recursion; the partly exercised `is_dangerous_path`; the type
   gate; the re-application suggestion)
   Add a tag-less `meta/work/drafts/` item, the seven-form
   `is_dangerous_path` table, a no-`type` case and an unquoted-`type` case,
   and an end-to-end re-application test.
7. **Share the in-crate test helpers** (addresses: the duplicated helpers)
   Move `OMIT_WHEN_EMPTY`, `frontmatter_of` and
   `omitted_when_empty_present` into `crate::test_support` for the
   `create_batch` and `sync_author` tests. Have `frontmatter_of` fail loudly
   rather than default to empty.
8. **Tighten the documentation passages** (addresses: the contradictory
   create-work-item passage; the CHANGELOG wording; the Migrations entry
   placement; no m0012 module doc)
   Simplify the create-work-item passage. Use the user-facing skill names
   in both CHANGELOG entries, and point the Fixed entry at migration 0012.
   Mention the end-of-frontmatter placement. Add a short `//!` module doc to
   m0012 in the m0010 style.
9. **Smaller refinements** (addresses: the dense index arithmetic; the
   quiet output; the missing `BASE_FIELDS` guard; the corpus-dependency
   suggestion; the duplicate-key finding; the labels-caveat suggestion)
   Extract `fenced_frontmatter` / `insertion_point` steps. Always print a
   count line, and warn when the work directory is absent. Add a `work` unit
   test that the composed keys cover `BASE_FIELDS`. Record that the corpus
   dependency is deliberate, and name m0010's `is_dangerous_path`. Optionally
   guard against duplicate keys via `ctx.parse_frontmatter`. Align the sync
   test's name with the labels caveat.

## Per-Lens Results

### Architecture

**Summary**: The plan fixes the defect in one place: the pure domain
function `compose_frontmatter`, which all three writer paths share. The
migration splits cleanly into a pure, byte-preserving transform and a thin
`apply` shell, and stays inside the `migrate` crate's cargo-pup import
boundary. The main gaps are about recurrence and system state. Nothing
structural stops the writer's required-field set drifting from `BASE_FIELDS`
again. The claim that each phase can merge by itself ignores that a
migration runs once. The migration also takes on two tradeoffs it doesn't
name: it depends on live `corpus` parsing, and it works around a lossy read
port.

**Strengths**:
- The fix lands in `compose_frontmatter` (`cli/work/src/create.rs:154`),
  which all three paths reach through `render_frontmatter`.
- Migration 0012 is a pure core inside an imperative shell.
- Presence and type come from `corpus::frontmatter_validation`, inside
  `migrate_domain_imports_only_permitted`.
- Returning `Applied` even when nothing needs repair follows the lifecycle
  contract.
- The Phase 1 integration tests check the writer against the validator end
  to end.
- The scope is tightly bounded and explicitly justified.

**Findings**:
- **minor / high**: One-shot migration must not ship ahead of the writer
  fix (Implementation Approach). 0012 runs once and is then recorded in the
  ledger. Tag-less items written after it is applied, by an unfixed or
  older plugin, are never repaired. State the release ordering, and
  document removing 0012 from the ledger and re-running as the way to
  repair stragglers.
- **minor / high**: No structural guard ties the writer's always-emitted
  keys to corpus `BASE_FIELDS` (Phase 1). `assert_matches_template_schema`
  compares only key sets. Add a `work` unit test asserting that
  `compose_frontmatter` with empty linkage and tags emits every
  `BASE_FIELDS` key. Optionally name the omit-when-empty set as a domain
  constant.
- **suggestion / medium**: The live dependency on corpus parsing sits
  against the migration-immutability principle (Phase 2, Section 1). Record
  that it is deliberate, and that the flow, block and empty `tags` unit
  tests are the safety net.
- **suggestion / medium**: Domain code compensates for a lossy read port by
  sniffing for U+FFFD (Phase 2, Section 2). Note it as a deliberate
  workaround with a known false positive, or add a strict-read port method.

### Code Quality

**Summary**: The plan is proportionate and well designed for maintenance.
The fix is one line in a pure domain function, and the migration keeps a
pure transform apart from a thin I/O `apply`. The weak spots are small.
Test helpers are duplicated and inconsistent despite `test_support`. The
splice logic relies on dense index arithmetic. The stderr reporting is
quieter than the m0005/m0010 precedent.

**Strengths**:
- The fix is confined to `compose_frontmatter`, and the red tests come
  first.
- The pure `with_empty_tags` is split from the effectful `apply`.
- Key presence comes from `corpus` rather than an ad-hoc scanner.
- `apply` uses let-else guard clauses.
- The doc comment and both SKILL.md passages are corrected with the code.

**Findings**:
- **minor / high**: Omit-when-empty test helpers are duplicated despite an
  existing in-crate `test_support` module (Phase 1, Sections 2 to 4). Use
  one shared constant and helper. Have the sync test assert the result is
  exactly the `external_id` line. Make `frontmatter_of` fail loudly.
- **minor / medium**: `with_empty_tags` mixes fence location, the repair
  decision and splicing through dense index arithmetic (Phase 2, Section 1).
  `at` is used both as a closure parameter and as a binding. Extract
  `fenced_frontmatter`, `lacks_tags` and `insertion_point`.
- **minor / medium**: Migration output is quieter than the m0005/m0010
  precedent and says nothing about a missing work directory (Phase 2,
  Section 2). Warn via `ctx.dir_exists`, and always print a count line.
- **suggestion / medium**: `unreadable` misnames files that were read but
  lossily decoded (Phase 2, Section 2). Rename to `lossily_decoded`.
- **suggestion / medium**: The "inline helpers" rule is cited but relaxed
  without reconciling the two (Key Discoveries / Phase 2). Note the
  deliberate corpus dependency, and name m0010's stricter
  `is_dangerous_path` as the copy to inline.

### Test Coverage

**Summary**: The testing strategy is strong and tied to the acceptance
criteria: a red-first domain test, a boundary test per writer path
validated in-process, and a literal-in/literal-out transform suite covering
the subtle boundaries. The gaps are in the migration's data-safety and
reach. Nothing tests that lossy-read files are reported, and `drafts/`
recursion is unpinned. One success criterion also claims tests fail before
the fix when two already pass.

**Strengths**:
- The full ordered key vector is asserted in the pure domain test.
- Each of the three writer paths has a boundary test that runs the real
  validator.
- The omit-when-empty checks are scoped to the frontmatter and match whole
  prefixes.
- Every transform test compares a full literal input with a full literal
  output, across all the subtle boundaries, including idempotence.
- `a_work_directory_needing_no_repair_still_applies` pins the
  ledger-recording contract.
- The registry and ledger test updates are listed, with a catch-all step.
- The end-to-end tests follow the `migration_0010.rs` pattern with
  hand-written fixtures.

**Findings**:
- **major / high**: Nothing tests that lossy-read items are reported, or
  that real invalid UTF-8 survives byte for byte (Phase 2, Sections 2 and
  4). Add an end-to-end test that writes raw `\xFF` and asserts the file is
  byte-identical, its path appears in stderr, and 0012 is recorded.
- **minor / high**: Two "red" tests already pass before the fix (Phase 1,
  Success Criteria). Label them as characterisation pins, and narrow the
  fail-first filter.
- **minor / medium**: Recursion into `drafts/` is relied on but untested.
  Add a tag-less `meta/work/drafts/` fixture.
- **minor / high**: The inlined `is_dangerous_path` copy is only partly
  exercised. Mirror m0005's seven-form table plus the safe values.
- **minor / medium**: The type gate's boundaries are untested. Add a
  no-`type` case and an unquoted-`type` case.
- **suggestion / medium**: Share the frontmatter helpers through
  `crate::test_support`.
- **suggestion / medium**: The re-run end-to-end test exercises only the
  ledger gate. Add a re-application test modelled on m0010's.
- **suggestion / low**: The sync test doesn't record that it diverges from
  criterion 5's "tracker labels" premise. Rename it, or drop the claim.

### Correctness

**Summary**: The writer fix is right, and the byte-offset transform holds
up: CRLF, a missing final newline, a closing fence at EOF, and the
`last_updated_by` trap are all handled, and the type and presence gate
agrees with the validator. The main gap is a state invariant outside the
file being edited. Sync baselines are not realigned.

**Strengths**:
- Copying the preceding line's ending on `split_inclusive` byte offsets
  keeps every byte.
- The exact-key anchor match avoids `last_updated_by`, and leading
  whitespace excludes nested keys.
- Presence and type come from `corpus` directly.
- Always returning `Applied`, together with an idempotent transform, makes
  re-runs safe for file contents.
- The fence check rejects the same documents the validator rejects.

**Findings**:
- **major / high**: Migration 0012 rewrites synced items' frontmatter
  without realigning their sync baselines (Phase 2, Section 2). Legacy
  entries can become `Conflict`, and split entries `LocallyModified`.
  Collect the pre-migration content, call `realign_sync_baseline`, count
  the calls in the double, add an end-to-end case, and update the port doc.
- **minor / medium**: A write failure part-way leaves earlier repairs with
  stale baselines that a re-run cannot recover. Realign the written subset
  before propagating the error, and pin this with a double whose `write`
  fails.
- **minor / medium**: Valid UTF-8 files that contain U+FFFD are wrongly
  treated as unreadable and never repaired. Use a strict read, or reword
  the note.
- **minor / low**: Unusual spellings of the `tags` key get a duplicate
  `tags: []`, making the frontmatter invalid YAML. Check the result with
  `ctx.parse_frontmatter` before writing.

### Safety

**Summary**: The plan is careful about safety for a developer-tooling
migration. It only adds a line and preserves every other byte. It is gated
on `type: "work-item"`, refuses unsafe paths, skips lossy reads, writes
through bounded atomic writes, and records the ledger only on success. The
one real gap is that sync baselines are not realigned. Phase 1 has no
meaningful safety risk, because nothing maps `tags` to tracker labels.

**Strengths**:
- Byte-preserving insertion keeps the blast radius to one line per file.
- The `work-item` type gate bounds the rewrite even for a broad
  `paths.work`, and a missing or broken fence fails safe.
- Files read lossily are skipped and named, with the validator as a
  backstop.
- The unsafe-path refusal combines with `WriteBounds`.
- Idempotence plus ledger-on-success makes retries safe, and VCS revert
  gives a rollback path.
- The manual `jj diff` check and the no-change check on this repository
  are good real-world guards.

**Findings**:
- **major / medium**: Migration 0012 rewrites synced items without
  realigning sync baselines (Phase 2, Section 2). Legacy items whose remote
  body has moved become `Conflict`. In bidirectional mode, choosing "local"
  pushes a stale body over the remote edit. Realign as m0008 does, and add
  unit and end-to-end tests.

### Compatibility

**Summary**: The writer fix is additive and compatible. Readers treat an
absent `tags` and `tags: []` the same way, and the output matches the
template and schema. The migration fits the registry, ledger and
discoverability conventions and stays within the pup rules. The main gap is
that sync baselines are not realigned, which m0008 already does.

**Strengths**:
- The `compose_frontmatter` signature is unchanged and
  `public-api:check` is gated.
- `work list` and similar readers already treat a missing `tags` and
  `tags: []` alike.
- Presence comes from `corpus`, an allowed `migrate` dependency.
- Byte preservation plus the lossy-read skip avoid re-encoding files.
- String ordering of migration IDs is checked, and a renumbering procedure
  is given.
- No `schema_version` bump, correctly.

**Findings**:
- **major / high**: Migration 0012 rewrites synced work items without
  realigning their sync baselines (Phase 2, Section 2). Call
  `ctx.realign_sync_baseline(&pre_migration)` as m0008 does, update the
  port doc, and add unit, end-to-end and manual checks.
- **minor / medium**: No test covers how the migration interacts with an
  item that has a sync baseline or a pending promotion (Phase 2, Section 4).
  Add an end-to-end case with a seeded baseline, and a manual
  `/sync-work-items` step after migrating.
- **minor / medium**: Teams on mixed plugin versions can keep creating
  tag-less items after 0012 is recorded (Migration Notes). Advise upgrading
  first, and document the straggler procedure.

### Documentation

**Summary**: The plan handles documentation well. It corrects both SKILL.md
passages and the doc comment, adds CHANGELOG entries in the right
sections, and excludes the generated docs. The main accuracy problem is the
rewritten doc comment's `external_id` claim. The other gaps are small: no
module doc for m0012, a self-contradicting create-work-item passage, and
inconsistent naming in the CHANGELOG.

**Strengths**:
- Every stale documentation site is listed and corrected in the same phase
  as the code.
- Generated docs-site pages are explicitly excluded.
- The CHANGELOG entries follow the house style, and the Migrations entry
  states the one manual step left.
- The renumbering note covers the CHANGELOG line.
- The extract-work-items passage separates what the manifest author does
  from what `create-batch` does.

**Findings**:
- **major / high**: The rewritten doc comment keeps a false claim about
  when `external_id` is written (Phase 1, Change 6). The sync importer also
  passes `Some(external_id)`.
- **suggestion / medium**: The doc comment mostly restates the field order
  the code already shows. Cut it to the non-obvious *why*.
- **minor / medium**: The new create-work-item passage contradicts itself
  about which keys are left out (`aliases`, `external_id`, and a dangling
  "So no separate omission logic").
- **minor / high**: m0012 has no module doc, unlike every existing
  migration.
- **minor / medium**: The CHANGELOG entries use the internal name "sync
  importer", and the Fixed entry doesn't point at migration 0012.
- **minor / high**: The Migrations entry describes only the `last_updated`
  placement, not the end-of-frontmatter fallback.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-18T22:36:18+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Compatibility / Safety / Correctness**: Migration 0012 rewrites
  synced items without realigning their sync baselines — Resolved
- 🟡 **Documentation**: The rewritten doc comment keeps a false claim about
  when `external_id` is written — Resolved
- 🟡 **Test Coverage**: Nothing tests that lossy-read items are reported,
  or that real invalid UTF-8 survives byte for byte — Resolved, via the
  strict `read_text` port and a raw-`0xFF` end-to-end test
- 🔵 **Correctness**: A write failure part-way leaves earlier repairs with
  stale baselines — Resolved
- 🔵 **Correctness**: Valid UTF-8 files containing U+FFFD are treated as
  unreadable — Resolved
- 🔵 **Correctness**: Unusual spellings of the `tags` key get a duplicate
  `tags: []` — Resolved, via the YAML-parse guard
- 🔵 **Test Coverage**: Two of the "red" tests already pass before the
  fix — Resolved
- 🔵 **Test Coverage**: Recursion into `drafts/` is untested — Resolved
- 🔵 **Test Coverage**: The inlined `is_dangerous_path` copy is only
  partly exercised — Resolved
- 🔵 **Test Coverage**: The type gate's boundaries are untested — Resolved
- 🔵 **Code Quality**: Omit-when-empty test helpers are duplicated —
  Resolved
- 🔵 **Code Quality**: `with_empty_tags` mixes three jobs through dense
  index arithmetic — Partially resolved (`fenced[anchor - 1]` and
  `1 + position(..)` remain)
- 🔵 **Code Quality**: Migration output is quieter than the m0005/m0010
  precedent — Resolved
- 🔵 **Architecture**: The one-shot migration must not ship ahead of the
  writer fix — Resolved
- 🔵 **Architecture**: No structural guard ties the writer's keys to
  `BASE_FIELDS` — Resolved
- 🔵 **Compatibility**: No test covers the interaction with an item that
  has a sync baseline or a pending promotion — Partially resolved (the
  baseline is covered; in-flight promotion records are not)
- 🔵 **Compatibility**: Teams on mixed plugin versions keep creating
  tag-less items — Resolved
- 🔵 **Documentation**: The create-work-item passage contradicts itself —
  Partially resolved (its first sentence still clashes with the `tags`
  rule)
- 🔵 **Documentation**: m0012 has no module doc — Resolved
- 🔵 **Documentation**: The CHANGELOG entries use internal names and don't
  link the fix to its migration — Resolved
- 🔵 **Documentation**: The Migrations entry describes only the
  `last_updated` placement — Resolved
- 🔵 All six suggestions (deliberate corpus dependency, the lossy-port
  workaround, `unreadable` naming, cutting the doc comment, the
  re-application test, the labels caveat in the test name) — Resolved

### New Issues Introduced

- 🔵 **Architecture / Safety / Correctness / Code Quality**: The failure
  path of realignment itself is unhandled. If `realign_sync_baseline`
  fails after the writes have succeeded, the originals are gone and no
  re-run can realign. When both the write and the realignment fail, the
  realignment error is dropped. Safety adds that baseline files realigned
  on the failure path aren't in the run manifest, so the resume pre-flight
  blocks the re-run and points the user at reverting the baseline, which
  recreates the stale state. Safety suggests a simpler design: restore the
  files already written when a write fails, and skip realignment.
- 🔵 **Architecture / Code Quality / Test Coverage / Compatibility /
  Correctness**: `read_text` defaults to `Ok(None)`, so a context that
  doesn't override it turns 0012 into a silent, recorded no-op. The port's
  convention for migration-specific capabilities is a "not implemented"
  `Err`. The default also makes the adapter's missing-file test pass
  before the adapter exists.
- 🔵 **Code Quality / Compatibility / Test Coverage**: `TextFile` and
  `Untouched` lack `Debug`/`PartialEq` derives, which the planned
  `assert_eq!` tests need and which the public-API snapshot will freeze.
- 🔵 **Architecture / Documentation**: The other realignment docs (the
  `SyncBaselines` trait, the `migrate-cli/src/sync_baselines.rs` module
  doc, the `work-adapters/src/sync/realign.rs` docs) still describe an
  0008-only re-render. "Content-identical items" is inaccurate for 0012.
- 🔵 **Documentation / Safety**: The stragglers procedure and manual step 2
  hand-edit the ledger, when the locked `accelerator migrate --unapply
  <id>` exists.
- 🔵 **Documentation / Code Quality**: The `compose_frontmatter` doc
  comment lists `source` apart from the typed-linkage fields, and its last
  sentence restates the code.
- 🔵 **Documentation**: The Overview, Desired End State and Performance
  Considerations don't mention the files left untouched or the
  per-candidate YAML parse.
- 🔵 **Documentation** (suggestions): `read`'s doc doesn't say it is lossy,
  `TextFile` has no doc, and the Quality Guidelines in both SKILL.md files
  still say every template field must be populated.
- 🔵 **Test Coverage**: The characterisation-pin filter `tagged_entry` also
  matches the red test `an_untagged_entry_…`.
- 🔵 **Test Coverage**: Phase 2 adds `pub mod m0012;` and registration too
  late, so the unit and end-to-end red steps can't be observed.
- 🔵 **Test Coverage**: How a unit test observes "recorded as `Untouched`"
  is unspecified, and the `NotUtf8` unit test doesn't check the record.
- 🔵 **Test Coverage**: CRLF is never run through the real YAML guard end
  to end. (Suggestion: no test asserts the realigned count in stderr.)
- 🔵 **Compatibility** (low confidence): baselines held in stalled
  `pending-push/` promotion records aren't realigned.
- 🔵 **Correctness** (suggestion): a `paths.work` that names an existing
  file gets a false "does not exist" warning and then a hard error. An
  early return, as m0005 does, avoids this.
- 🔵 **Code Quality** (suggestions): the residual index arithmetic could
  pair each line with the one before it. The `violations` helper exists in
  only one copy.
- 🔵 **Safety** (suggestion, low confidence): check the *repaired* output
  with a YAML parse too, so an unforeseen splice hazard is caught.

### Assessment

All three major findings and all but three minor ones are resolved. None
of the lenses found anything critical or major in the revised plan, so the
verdict moves from REVISE to COMMENT: the plan can be implemented as it
stands. The one theme worth acting on is the failure path of realignment,
which four lenses raised. Safety's restore-on-write-failure alternative
would remove most of that complexity. Two cheap, widely flagged fixes are
the `read_text` default and the missing derives. The rest are polish on
documentation and tests.

## Re-Review (Pass 3) — 2026-11-18T23:08:37+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Architecture / Safety / Correctness / Code Quality**: The failure
  path of realignment itself is unhandled — Resolved. Restore-on-failure
  writes back every repaired item, and realignment no longer runs after a
  failed write.
- 🔵 **Architecture / Code Quality / Test Coverage / Compatibility /
  Correctness**: `read_text` defaults to `Ok(None)` — Resolved
- 🔵 **Code Quality / Compatibility / Test Coverage**: `TextFile` and
  `Untouched` lack derives — Resolved
- 🔵 **Architecture / Documentation**: The realignment docs describe an
  0008-only re-render — Partially resolved. One sentence of the
  `realign_sync_baseline` doc still says "after the files on disk have been
  re-rendered", and the `work-adapters/tests/sync_realign.rs` module doc
  isn't listed.
- 🔵 **Documentation / Safety**: The stragglers procedure hand-edits the
  ledger — Resolved
- 🔵 **Documentation / Code Quality**: The `compose_frontmatter` doc
  comment groups `source` separately — Resolved
- 🔵 **Documentation**: The summary sections don't mention the untouched
  files or the YAML parses — Resolved
- 🔵 **Documentation**: `read`'s lossy decoding, the `TextFile` doc, and the
  SKILL.md Quality Guidelines — Resolved (the guidelines' new wording has a
  taxonomy slip; see below)
- 🔵 **Documentation**: The create-work-item passage clashes with the
  `tags` rule — Resolved, though the new wording has smaller problems (see
  below)
- 🔵 **Test Coverage**: The characterisation-pin filter collides with a
  red test — Resolved
- 🔵 **Test Coverage**: Phase 2's red steps can't be observed —
  Partially resolved. Two idempotence tests still pass before
  registration.
- 🔵 **Test Coverage**: There is no stated way to observe a file being
  recorded as `Untouched` — Resolved
- 🔵 **Test Coverage**: CRLF isn't run through the real YAML guard, and the
  realigned count isn't asserted — Resolved
- 🔵 **Compatibility**: Stalled promotion records — Resolved, now listed as
  accepted scope
- 🔵 **Correctness**: A `paths.work` that names a file — Resolved in the
  code, but the early return has no test that fails without it
- 🔵 **Safety**: Check the repaired output with a YAML parse — Resolved in
  the code, but under-tested (see below)
- 🔵 **Code Quality**: Residual index arithmetic, and the `violations`
  helper in only one copy — Resolved

### New Issues Introduced

- 🔵 **Safety / Code Quality / Documentation / Architecture**: The advice to
  revert `last-sync.json` after a failure is wrong:
  - Safety: it's unnecessary, because realignment converges on a re-run.
    An entry advanced before the failure already matches the re-repaired
    file.
  - Architecture: in a multi-migration run it would discard 0008's
    realignment, which is already recorded as applied.
  - Code Quality: the message gives the advice even after a write failure,
    when no baseline was touched.

  Correctness notes that the pre-flight flags a partly written
  `last-sync.json` as an unowned change. The coherent advice is therefore
  to *keep* the file (commit it) and re-run `/accelerator:migrate` before
  any sync, and to give it only when the realignment failed.
- 🔵 **Safety / Correctness**: A write can return `Err` after its rename
  has landed, for example when the manifest append fails. Such a file is
  never in `originals`, so it is neither restored nor named. Record the
  original before writing. Safety adds that `restore_after` overwrites the
  file without checking that it still holds the repair.
- 🔵 **Code Quality / Documentation**: The restore message counts every
  item as restored, including those it then lists as "could not restore".
  The restore writes also run inside a `filter` predicate, and their errors
  are dropped.
- 🔵 **Code Quality**: `yaml_obstacle` discards the parser's error, so a
  "not implemented" `parse_frontmatter` becomes a per-file skip and a
  recorded run. Carry the cause in `UnparseableFrontmatter`.
- 🔵 **Correctness / Test Coverage**: The `InsertionChangesMeaning`
  end-to-end fixture, a multi-line quoted title, can't parse under
  granit-parser, so it exercises `UnparseableFrontmatter` instead. The
  module doc's example of the hazard can't occur in practice. In the unit
  tests, `shifts_meaning` only exercises the entry-count part of
  `is_original_plus_empty_tags`.
- 🔵 **Test Coverage**: The two idempotence end-to-end tests pass before
  registration, because they never check that the first run repaired
  anything.
- 🔵 **Test Coverage**: Nothing fails if the early return is removed. The
  realign recorder can't tell "never called" from "called with nothing",
  and no test has `paths.work` naming a file.
- 🔵 **Test Coverage**: The read-only-directory end-to-end test should use
  `store::test_support::ReadOnlyDir` (0o555, skipped as root, permissions
  restored on drop), and should assert the restored count.
- 🔵 **Documentation**: The create-work-item passage states the omission
  rule twice, and still says `work create` never writes `external_id`,
  which `--push` does. **Compatibility** (suggestion): this passage is read
  by `tests/unit/tasks/test_skill_frontmatter_population.py`, so the plan
  should name that test and the phrases it needs.
- 🔵 **Documentation**: The Quality Guidelines rewording calls `external_id`
  a typed-linkage key, and leaves out `aliases`.
- 🔵 Suggestions:
  - Documentation: the CHANGELOG Migrations entry has a broken list and is
    long, and the Fixed entry's "Run migration 0012" isn't a command.
  - Documentation: the plan gives two different line ranges for m0011's
    double.
  - Compatibility: state the exact `full_registry_over_git.rs` stderr (no
    warning), and fix the `migration_0011.rs` module doc.
  - Code Quality: replace the double's scenario flags with per-test
    parse and write tables.
  - Test Coverage: make the restore tests' assertions discriminate between
    failure kinds.

### Assessment

Every finding from pass 2 is now resolved or partly resolved, and none of
the lenses raised anything critical or major. The verdict stays COMMENT.
The new issues sit on the rarely hit failure path that pass 3 introduced,
in test precision, and in wording. The one that matters most is the
`last-sync.json` advice: the plan currently says to revert the file, and
in a multi-migration run that would discard 0008's realignment. Recording
each original before writing it is a one-line change that closes the
other restore gap. The plan can be implemented as it stands, provided
those two points are fixed first.

## Re-Review (Pass 4) — 2026-11-18T23:43:49+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Safety / Code Quality / Documentation / Architecture**: The advice
  to revert `last-sync.json` is wrong — Mostly resolved. The plan now says
  to keep and commit it, and only after a failed realignment. Correctness
  traced `realigned_entry` and confirmed this converges. What remains is
  listed under New Issues.
- 🔵 **Safety / Correctness**: A write that lands and then fails is never
  restored, and the restore overwrote without checking — Resolved. Each
  repair is recorded before its write, and `Repair::restore` acts only on
  states it recognises.
- 🔵 **Code Quality / Documentation**: The restore count was wrong, and the
  restore ran inside a `filter` — Resolved
- 🔵 **Code Quality**: `yaml_obstacle` discarded the parser's error —
  Resolved
- 🔵 **Correctness / Test Coverage**: The impossible
  `InsertionChangesMeaning` fixture — Resolved. It now tests unparseable
  frontmatter, and granit-parser was confirmed to reject it.
- 🔵 **Test Coverage**: The idempotence tests passed before registration,
  the early return was unpinned, and the read-only-directory guard was
  missing — Resolved
- 🔵 **Documentation**: The create-work-item passage, the Quality
  Guidelines taxonomy, the CHANGELOG, the m0011 range, and the remaining
  "re-render" wording — Resolved. Compatibility ran the population guard
  against the new passage and it passes, still meeting criterion 6.
- 🔵 **Compatibility**: The guard named, the exact stderr, the
  `migration_0011.rs` doc — Resolved (the module doc only partly; see
  below)
- 🔵 **Code Quality**: The test double's scenario flags — Resolved, now
  keyed tables

### New Issues Introduced

- 🔵 **Architecture / Correctness**: Committing *only* `last-sync.json`
  moves the run base, so in a multi-migration run the next pre-flight
  refuses again. The advice should be to commit the whole run's changes,
  as `STALE_RUN_EXPLANATION` already says.
- 🔵 **Safety**: A failed *backfill* in a run that also applied 0008 also
  leaves a dirty `last-sync.json`. The keep-and-commit advice is needed
  for both stages, not just a failed realignment. (Suggestion: don't push
  the intermediate commit until the re-run succeeds.)
- 🔵 **Safety / Documentation**: The advice for items left to restore by
  hand is wrong in two ways:
  - "Restore by hand from version control" could undo 0008's rewrite or a
    concurrent edit. Removing the one `tags: []` line is the safer fix.
  - The message lists those items *after* telling the user to re-run, so
    following it in order turns them into stragglers.
- 🔵 **Correctness**: A restoring write can hit the same manifest-append
  failure, so a restored file is reported as needing restoring by hand.
  Read the file back after a failed restoring write.
- 🔵 **Test Coverage**: The `store` test-support dev-dependency is added in
  §5, after §4's tests need it. Move it into §4.
- 🔵 **Test Coverage**: No test fails if the entry-count check or the exact
  empty-`tags` check is removed from `is_original_plus_empty_tags`.
  (Suggestion: assert the realignment message's count and the absence of
  "by hand".)
- 🔵 **Documentation**: `migration_0011.rs`'s module doc would still say
  "pre-seeded through 0010".
- 🔵 Suggestions:
  - Code Quality: the skipped-file tests need `let mut backfill`, not a
    temporary;
  - Code Quality / Documentation: name the stage in the backfill-failure
    message, and drop "Left the 0";
  - Documentation: list the seven keys plainly in the create-work-item
    passage.

### Assessment

Every pass-3 finding is resolved or mostly resolved, and Compatibility
found nothing new. None of the lenses raised anything critical or major.
The verdict stays COMMENT. Every new issue concerns the wording of the
recovery advice for a write or realignment that fails part-way, or the
precision of tests. The design has converged. The recovery advice is the
one area still producing findings each pass. The text that fits all of
them is: before re-running, remove the added `tags: []` line from any
named item; commit everything the run changed, including
`last-sync.json`, without pushing; then re-run before syncing.

## Approval — 2026-11-18T23:47:08+00:00

**Verdict:** APPROVE

Every pass-4 finding was addressed in the plan: the recovery advice was
rewritten as one ordered set of steps, restoring a file now reads it back
after a failed write, and the remaining test and documentation fixes were
made. The plan is approved for implementation.
