---
date: "2026-05-03T16:30:00+01:00"
type: plan
skill: create-plan
work-item: ""
status: draft
---

# Update Visualiser for Work-Item Terminology Implementation Plan

## Overview

The visualiser was developed against the legacy "ticket" terminology and is
now broken in any project that has been migrated by `0001-rename-tickets-to-work`:
the launcher writes a `tickets` config key the plugin no longer recognises,
which silently resolves to the non-existent `meta/tickets/` directory, which
in turn empties the writable-roots whitelist so every kanban PATCH returns
`OnlyTicketsAreWritable` and every read-only library/lifecycle/kanban view
shows zero work-items.

This plan brings the visualiser into line with the new terminology, the
configurable filename ID pattern, and the multi-field frontmatter
cross-reference scheme. It ships in four phases and is built test-first
wherever the change shape allows.

## Current State Analysis

The full surface analysis is in
`meta/research/2026-05-03-update-visualiser-for-work-item-terminology.md`
(917 lines). In summary:

- **Launcher** (`skills/visualisation/visualise/scripts/write-visualiser-config.sh:39,89,105`)
  emits a `tickets` config key and the legacy `meta/tickets/` default. The
  plugin's `scripts/config-read-path.sh:7-19` only recognises `work` and
  `review_work` post-migration.
- **Server** (~50 sites across 12 files): a `DocTypeKey::Tickets` variant,
  `TicketStatus { Todo, InProgress, Done }` enum, `IndexEntry.ticket` field,
  `Completeness.has_ticket` flag, `OnlyTicketsAreWritable` error,
  digit-prefix-only slug derivation, `u32`-keyed
  `Indexer.ticket_by_number` map, `frontmatter::ticket_of` reading only the
  legacy `ticket:` key, and integration fixtures pinned to
  `tests/fixtures/meta/tickets/`.
- **Frontend** (~40 sites across ~20 files): `DocTypeKey` union with
  `'tickets'`, `TicketCard.tsx` with hardcoded `params={{ type: 'tickets' }}`,
  `WIKI_LINK_PATTERN = /\[\[(ADR|TICKET)-...]]/`, kanban toasts and
  aria-live strings using "ticket" copy, fixtures hardcoded to
  `meta/tickets/0001-...md` paths, and Playwright e2e specs pinned to
  legacy fixture filenames.
- **Live `meta/work/` content**: 30 files, all legacy schema (`type:
  adr-creation-task`, statuses `todo|done|proposed`, no `work_item_id:`).
  The migration script renamed paths and a config key only — per-file
  frontmatter was intentionally left alone. The visualiser must therefore
  tolerate **both** legacy-schema work-items (live in this and other
  pre-migration repos) **and** new-schema work-items that future skills
  produce.

### Key Discoveries

- **The wire-format flip is atomic.** Any rename that touches a `DocTypeKey`
  literal, an `IndexEntry` field name, or a `Completeness` flag must change
  on server and frontend in lockstep. There is no incremental bridge.
- **The launcher already orchestrates plugin scripts** via
  `config-read-path.sh`, so calling
  `skills/work/scripts/work-item-pattern.sh --compile-scan` to produce a
  scan regex slots into the existing pattern.
- **The Rust patcher's status logic is type-agnostic** at
  `skills/visualisation/visualise/server/src/patcher.rs:49` — it only
  edits the YAML `status:` line. The current `TicketStatus { Todo,
  InProgress, Done }` enum is the *type-level* gate; widening to
  configurable column keys requires turning the gate into a
  string-based check against config-supplied values.
- **The plan templates already migrated.** `templates/plan.md:4` is now
  `work-item: "{work-item reference, if any}"`; `templates/work-item.md:9`
  defines `parent:`. The visualiser's `frontmatter::ticket_of` is the
  only consumer reading the obsolete `ticket:` key.
- **Phase 12 (packaging/release) is complete** but its plan file's
  frontmatter still says `status: draft`. A small fix-up.
- **Stragglers from the 2026-04-26 audit are clean.**
  `agents/documents-locator.md`, `templates/pr-description.md`, the
  `test-lens-structure.sh`, `test-work-item-scripts.sh`, and other files
  flagged in `meta/research/2026-04-26-remaining-ticket-references-post-migration.md`
  no longer contain ticket references. No straggler bundle is needed.

## Desired End State

A freshly migrated project (any pre-migration repo plus a clean
`/accelerator:init` install) launches the visualiser successfully and:

1. The sidebar lists 11 doc types: `decisions`, `work-items`, `plans`,
   `research`, `plan-reviews`, `pr-reviews`, `work-item-reviews`,
   `validations`, `notes`, `prs`, `templates` — note `work-item-reviews`
   is net-new.
2. Work-items appear in the kanban with the configured column set
   (default = the seven `templates/work-item.md:7` statuses), drag-drop
   updates persist via PATCH, and unknown statuses still fall into
   "Other".
3. Wiki-links of the form `[[WORK-ITEM-NNNN]]` (default pattern) and
   `[[WORK-ITEM-PROJ-0042]]` (project-prefixed) resolve to the correct
   work-item; `[[ADR-NNNN]]` continues to resolve.
4. A configured `work.id_pattern: "{project}-{number:04d}"` with a
   default project code produces filenames the slug regex parses,
   the indexer keys on the full string ID, and lifecycle clusters
   build correctly.
5. Frontmatter cross-references aggregate values from `work-item:`,
   `parent:`, and `related:`; reverse cross-refs populate the
   "Related artifacts" aside in library view.
6. Both legacy-schema work-items (with `status: todo|done|proposed`
   and no `work_item_id:`) and new-schema work-items render
   gracefully.
7. Every Rust integration test, frontend Vitest, and Playwright e2e
   passes against the renamed fixture paths.

### Verification of the end state

- **Automated**: `make test` (server, frontend unit, e2e) and
  `make lint` from the visualise skill root all pass.
- **Manual**: spin up the visualiser against (a) this workspace's
  legacy `meta/work/` content, and (b) a fresh project initialised
  with `{project}-{number:04d}` and a custom four-column kanban
  config; both render correctly per the validation script in
  Phase 4.

## What We're NOT Doing

- **No frontmatter migration of live `meta/work/` files.** The
  visualiser must tolerate the legacy schema present in this and
  other pre-migration repos. Rewriting per-file frontmatter is a
  permanent tolerance contract (ADR-0025), not a deferred migration.
- **No new generic kanban support.** The kanban remains bound to
  the work-items doc type. Phase 3 makes the *column set*
  configurable, not the doc type.
- **No editing of `[[WORK-ITEM-...]]` content.** Only the read-side
  resolver changes; nothing in this plan inserts wiki-links into
  documents. Body-text `[[TICKET-NNNN]]` references found by the
  Step 1.7 grep are migrated by a one-shot script if the count is
  small (default behaviour); larger backlogs become dead-link debt
  recorded in ADR-0025.
- **No bare `[[NNNN]]` wiki-link form.** The literal `WORK-ITEM-`
  prefix stays mandatory (Q3 of the research).
- **No straggler cleanup of `agents/documents-locator.md`,
  `templates/pr-description.md`, etc.** Those have been cleaned
  separately since the 2026-04-26 audit; nothing remains.
- **No CSS/visual rework of kanban cards.** `TicketCard.tsx`
  becomes `WorkItemCard.tsx` with the same visual treatment.
- **No new config keys beyond `visualiser.kanban_columns`.** Phase
  2 reads existing `work.id_pattern` and `work.default_project_code`;
  Phase 3 introduces only `visualiser.kanban_columns`.
- **No `WorkItemStatus` newtype.** Phase 1 drops the enum entirely
  for plain `String` validated at the API boundary; no wrapper type
  is introduced. Validation source moves from hardcoded array to
  config slice in Phase 3.
- **No new visualiser-side README.** The visualise skill is
  documented via SKILL.md; config schema lives in
  `skills/config/configure/SKILL.md`. No new top-level README is
  created.

## Implementation Approach

The four phases are sequenced by dependency. Phase 1 unbreaks the
visualiser end-to-end on the default ID pattern (which is what every
consumer uses today). It takes **destination names and shapes** for
identifiers and types that Phase 2/3 will widen, so subsequent phases
only add behaviour rather than re-rename. Phases 2 and 3 layer
additive features. Phase 4 lands documentation, the two ADRs, and
the validation runs.

**Single-pass renames** (no two-step trajectories): Phase 1 renames
`parseTicketNumber` directly to `parseWorkItemId`, `ticket_by_number`
to `work_item_by_id`, `ticketByNumber` to `workItemById`,
`ticketNumberFromRelPath` to `workItemIdFromRelPath`. Phase 2 widens
the underlying types from numeric to string without renaming.

**No `WorkItemStatus` newtype**: Phase 1 drops the `TicketStatus`
enum entirely in favour of plain `String` validated at the API
boundary against a hardcoded seven-status default array. Phase 3
swaps the source of the validation array from hardcoded to
config-driven, with no type change.

**Frontend regex consumes server output**: Phase 2 has the frontend
consume the wiki-link inner pattern via the config endpoint rather
than maintaining a parallel hand-written regex.

**Fail-fast at boot**: Phase 1 adds validation that
`cfg.doc_paths.work` is present (rejects unmigrated repos with a
clear pointer at `/accelerator:migrate`); Phase 2 compiles the scan
regex via a fallible constructor with a clear error if the pattern
is invalid; Phase 3 rejects empty/malformed `kanban_columns`. Each
prevents a recurrence of the silent-degraded-state failure mode this
plan was created to fix.

### Test-driven discipline

The change shape determines the TDD style applied:

- **For new behaviour** (the `work-item-reviews` doctype, the
  pattern-aware slug regex, the configurable column rendering,
  the multi-field cross-ref aggregation): pure red→green→refactor.
  Add fixture, write failing test, implement minimum code, refactor,
  next test.
- **For mechanical renames** (`Tickets` → `WorkItems`,
  `IndexEntry.ticket` → `IndexEntry.workItemRefs`, fixture path
  moves): "test-first" doesn't apply — the compiler/test runner
  flags every site and we work the resulting fail-list to green.
  We use the test renames as the spec: rename the test's expected
  symbol/string first, run the suite, fix every failing assertion
  by renaming the production code in lockstep. The renames are
  atomic per commit boundary inside the Phase 1 PR but the tests
  drive the renames.
- **Establish a green baseline before each phase.** Run the full
  suite in this workspace before any code change; record the
  result; only proceed when green. After each in-phase commit,
  run the suite again. Phase 1 must end on a fully green suite
  before Phase 2 begins.

### PR strategy

- Phase 1: single PR (server + launcher + frontend together).
  Atomic wire-format flip is the binding constraint.
- Phase 2: single PR on top of Phase 1.
- Phase 3: single PR (or two, at reviewer discretion — kanban
  columns and cross-ref reading are independent).
- Phase 4: documentation/ADR PR.

---

## Phase 1: Coordinated rename — server, launcher, frontend (default pattern)

### Overview

Rename "ticket" → "work-item" across server, launcher, and frontend in
lockstep. Add the new `work-item-reviews` doctype. Take **destination
names** for renamed identifiers (`parse_work_item_id`,
`work_item_by_id`, etc.) so Phase 2 only adds pattern-aware behaviour
without a second rename pass. Take the **destination shape** for
`WorkItemStatus`: replace the closed enum with a plain `String`
validated at the API boundary against a hardcoded seven-status default
set (`draft | ready | in-progress | review | done | blocked |
abandoned`, matching `templates/work-item.md:7`). Phase 3 swaps the
hardcoded set for a config-driven one without touching the type. Do
**not** widen the slug regex or the wiki-link inner pattern in this
phase — those are Phase 2 work. The deliverable is a visualiser that
works end-to-end on the default `{number:04d}` ID pattern in a freshly
migrated project, including PATCH round-trip for any status in the
seven-state set (so legacy `proposed` and templated `draft|ready|
review|blocked|abandoned` files all work).

### Step 1.0 — Establish a green baseline

**Approach**: run the full test suite from a clean working copy and
confirm all green before any code change. If anything is red,
investigate and fix before proceeding.

```bash
cd skills/visualisation/visualise/server && cargo test --all-features
cd skills/visualisation/visualise/frontend && npm run test -- --run
cd skills/visualisation/visualise/frontend && npx playwright test
```

### Step 1.1 — Add the `work-item-reviews` doctype (TDD: red → green)

**Approach**: this is net-new behaviour, so write the failing tests
first, then implement.

#### 1.1a — Tests first

**File**: `skills/visualisation/visualise/server/src/docs.rs`

Add four failing tests to the existing `tests` module:

```rust
#[test]
fn work_item_reviews_serialises_to_kebab_case_wire_form() {
    let v = DocTypeKey::WorkItemReviews;
    assert_eq!(serde_json::to_string(&v).unwrap(), "\"work-item-reviews\"");
}

#[test]
fn work_item_reviews_uses_review_work_config_path_key() {
    assert_eq!(DocTypeKey::WorkItemReviews.config_path_key(), Some("review_work"));
}

#[test]
fn work_item_reviews_appears_in_all_and_in_lifecycle_only() {
    assert!(DocTypeKey::all().contains(&DocTypeKey::WorkItemReviews));
    assert!(DocTypeKey::WorkItemReviews.in_lifecycle());
    assert!(!DocTypeKey::WorkItemReviews.in_kanban());
    assert!(!DocTypeKey::WorkItemReviews.is_virtual());
}

#[test]
fn doc_type_key_all_returns_eleven_variants() {
    assert_eq!(DocTypeKey::all().len(), 11);
}
```

Update the existing `kebab_case_round_trip_covers_every_variant` and
`all_returns_every_variant_exactly_once` tests to expect 11 variants.

**File**: `skills/visualisation/visualise/server/src/slug.rs`

Add a failing test:

```rust
#[test]
fn work_item_reviews_strip_date_and_review_n_suffix() {
    let cases = &[
        ("2026-04-30-completeness-pass-review-1.md",
         Some("completeness-pass")),
        ("2026-05-02-foo-review-7.md", Some("foo")),
        ("2026-04-30-no-suffix.md", None),
    ];
    for (input, expected) in cases {
        let got = derive(DocTypeKey::WorkItemReviews, input);
        assert_eq!(got.as_deref(), *expected, "input={input}");
    }
}
```

Run `cargo test`; confirm the new tests fail with "no variant
`WorkItemReviews`".

#### 1.1b — Implementation (minimum to green)

**File**: `skills/visualisation/visualise/server/src/docs.rs`

```rust
pub enum DocTypeKey {
    // ... existing variants
    WorkItemReviews,
}
```

Update `all()` to include the new variant (length 11), `config_path_key`
returns `Some("review_work")`, `label()` returns `"Work-item reviews"`,
`in_kanban()` stays false.

**File**: `skills/visualisation/visualise/server/src/slug.rs`

Add the `WorkItemReviews` arm to `derive`:

```rust
DocTypeKey::PlanReviews | DocTypeKey::PrReviews | DocTypeKey::WorkItemReviews => {
    let without_date = strip_prefix_date(stem)?;
    strip_suffix_review_n(&without_date)
}
```

Run `cargo test`; confirm green.

### Step 1.2 — Wire up the `review_work` path in the launcher (TDD: pattern test in shell)

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`

#### 1.2a — Test first

**File**: `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`

This is the first shell test in the visualise skill's `scripts/`
directory. Follow the established plugin convention: source
`scripts/test-helpers.sh` and model the file on
`skills/work/scripts/test-work-item-pattern.sh`. Add it to whatever
top-level `make test` (or equivalent) target the visualise skill
exposes so it runs in CI.

Add tests covering:

1. Default config (no `paths.work` override) produces `config.json`
   with `doc_paths.work` resolving to `<project-root>/meta/work/` and
   `doc_paths.review_work` resolving to
   `<project-root>/meta/reviews/work/`.
2. Pre-migration project (`.claude/accelerator.md` carries
   `paths.tickets` but no `paths.work`): the launcher exits non-zero
   with a stderr message telling the operator to run
   `/accelerator:migrate`. **This is the test that locks down the
   "unmigrated project" failure mode** so we don't regress to silent
   broken-kanban after the rename.
3. Override (`paths.work: meta/items`) is reflected in the produced
   `doc_paths.work`.

Run; confirm fail.

#### 1.2b — Implementation

- Line 39: rename `TICKETS="$(abs_path tickets meta/tickets)"` →
  `WORK="$(abs_path work meta/work)"`.
- Add a new line: `REVIEW_WORK="$(abs_path review_work meta/reviews/work)"`.
- Add a pre-flight migration check: if the project's
  `.claude/accelerator.md` defines `paths.tickets` and does not
  define `paths.work`, the launcher exits non-zero with a clear
  message naming the migration: "this project predates the
  tickets→work-items rename. Run `/accelerator:migrate` to apply
  migration `0001-rename-tickets-to-work` before launching the
  visualiser." Reuse the existing config-read helper rather than
  hand-parsing YAML.
- Lines 89/105 jq invocation: replace `--arg tickets "$TICKETS"` /
  `tickets: $tickets` with `--arg work "$WORK"` / `work: $work`, and
  add `--arg review_work "$REVIEW_WORK"` / `review_work: $review_work`.

Run the shell test; confirm green.

### Step 1.3 — Update SKILL.md placeholder

**File**: `skills/visualisation/visualise/SKILL.md`

Line 22: replace
`**Tickets directory**: !\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets\``
with
`**Work directory**: !\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work\``

Add a new line:
`**Work reviews directory**: !\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_work meta/reviews/work\``

(Display-only; the launcher does its own resolution.)

### Step 1.4 — Server-side rename: `DocTypeKey::Tickets` → `DocTypeKey::WorkItems`

**Approach**: this is a mechanical rename. The compiler will list every
site. Run `cargo check` after each edit and use the failure list as
the work queue.

#### 1.4a — Update test expectations first

Rename in tests (these will fail to compile until the production code
is renamed in 1.4b — that's the desired TDD shape: tests describe the
target state):

- `docs.rs` test module: every `DocTypeKey::Tickets` → `DocTypeKey::WorkItems`,
  every `"tickets"` wire-form literal → `"work-items"`, every "Tickets"
  label literal → "Work items".
- `slug.rs` test module: rename `tickets_strip_numeric_prefix` →
  `work_items_strip_numeric_prefix`, every `DocTypeKey::Tickets` →
  `DocTypeKey::WorkItems`. (Behaviour and fixtures unchanged in this
  phase.)
- `patcher.rs` test module: drop tests that relied on serde-typed
  `TicketStatus`; replace with tests that pass a plain `&str` status
  through `apply` and assert the YAML body's `status:` line is
  rewritten verbatim. Status validation tests live alongside
  `api/docs.rs` handlers — see new tests below.
- `api/docs.rs` (or `tests/api_docs_patch.rs`): add tests covering all
  seven default statuses round-trip via PATCH, and a rejection test
  for an unknown status returning `ApiError::UnknownKanbanStatus`
  with the seven keys in `accepted_keys`. Add a legacy-`proposed`
  fixture and assert the round-trip succeeds.
- `frontmatter.rs` test module: add tests covering both `ticket:`
  (legacy) and `work-item:` (current) frontmatter keys producing a
  single-element vec; and the case where neither is present producing
  an empty vec.
- `clusters.rs` test module: every assertion on `Completeness.has_ticket`
  → `Completeness.has_work_item`; semantics unchanged (cluster-presence
  flag set when any entry has `DocTypeKey::WorkItems`).
- `server.rs` (or `tests/api_smoke.rs`): add a startup test that loads
  a config without a `work` key and asserts the server exits with a
  non-zero status and a stderr message naming the missing key.

#### 1.4b — Production code rename

**Files**:
- `server/src/docs.rs:8,23,38,53,70,115,153-160` — enum variant + all
  match arms; `config_path_key` returns `Some("work")`; `label`
  returns `"Work items"`; `in_kanban()` matches `WorkItems`.
- `server/src/slug.rs:11,35-42` — match arm + helper renamed:
  `strip_prefix_ticket_number` → `strip_prefix_work_item_id`. Body
  unchanged in this phase.
- `server/src/clusters.rs:11,69,98,110` — rename
  `Completeness.has_ticket` → `Completeness.has_work_item` (wire
  `hasWorkItem` via existing camelCase serde). This is a
  cluster-presence flag set by `derive_completeness` when any entry
  in the lifecycle cluster has `DocTypeKey::WorkItems` — parallel
  with `has_plan`, `has_decision`, `has_research`, `has_pr_review`,
  all of which are retained. **Do not** replace it with a per-entry
  `!workItemRefs.is_empty()` derivation: those are different
  concepts (presence-of-doc-in-cluster vs. presence-of-cross-ref).
  The match arm in `derive_completeness` and the canonical-rank arm
  rename in lockstep.
- `server/src/patcher.rs:5,14-20,22-30,49,119,272,296,304,326,341,347,352,362,371,378,386,404,416,429,437,452,462`
  — drop the `TicketStatus { Todo, InProgress, Done }` enum entirely.
  The patcher's `apply` takes a plain `&str` status (already validated
  at the API boundary) and writes it verbatim into the YAML `status:`
  line. Validation moves to the API layer (see `api/docs.rs` below).
- `server/src/frontmatter.rs:297-306` — replace `ticket_of` with the
  pure helper `read_ref_keys(fm) -> Vec<RawRef>` that lands in its
  destination shape now (no Phase-1-only `work_item_refs_of` shim).
  Phase 1 body reads `work-item:` (current) and `ticket:` (legacy)
  as single-element vec contributors; multi-key aggregation
  (`parent:`, `related:`) lands in Phase 3 by extending this same
  function. **Both-keys-present rule**: if a frontmatter contains
  both `work-item:` and `ticket:`, **`work-item:` wins** — the
  newer key is taken and the legacy key is silently ignored. This
  makes hand-edited transitional-state files index deterministically
  rather than depending on serde key ordering. Add a test:
  `read_ref_keys_with_both_legacy_and_current_keys_prefers_current`.
  Returns empty `Vec` when neither key is present.
- `server/src/indexer.rs:25,55,109,132-134,152,156,477-493,531-538,568,595,611-617,632-635`
  — rename `IndexEntry.ticket` → `IndexEntry.workItemRefs:
  Vec<String>`; take destination names directly:
  `ticket_by_number` → `work_item_by_id` (still `HashMap<u32,
  PathBuf>` in this phase keyed on the parsed number; Phase 2 changes
  the key type to `String` without renaming the field);
  `parse_ticket_number` → `parse_work_item_id` (returns `Option<u32>`
  in Phase 1, `Option<String>` in Phase 2 — same name);
  `ticket_number_from_entry` → `work_item_id_from_entry`. `build_entry`
  calls `read_ref_keys` (the pure parser landed in Phase 1; see
  `frontmatter.rs` below) and stores the resulting vec.
- `server/src/api/mod.rs:53,54,95,155` — rename
  `OnlyTicketsAreWritable` → `OnlyWorkItemsAreWritable`; literal string
  `"only work-items are writable"`. Add a new `ApiError` variant
  `UnknownKanbanStatus { accepted_keys: Vec<String> }` that serialises
  through the existing envelope (Phase 3 reuses this rather than
  inventing a bespoke 400 body shape; declared here in Phase 1 so the
  envelope is uniform from day one).
- `server/src/api/docs.rs:107-109,154-156` — `PatchFields { status:
  Option<String> }`; type guard compares against `DocTypeKey::WorkItems`.
  Extract a small pure helper `validate_kanban_status(status: &str,
  accepted: &[&str]) -> Result<(), ApiError>` (in `api/docs.rs` or
  a sibling `api/validation.rs`) so the handler stays thin and
  validation is unit-testable in isolation. The handler calls the
  helper with the **hardcoded seven-status default set** (`draft |
  ready | in-progress | review | done | blocked | abandoned`)
  before passing the validated string to `patcher::apply`.
  Validation is exact-match, case-sensitive, no whitespace
  trimming. On mismatch return `ApiError::UnknownKanbanStatus` with
  the seven keys in `accepted_keys`. Phase 3 swaps the slice
  passed to the helper from the hardcoded array to
  `cfg.kanban_columns.iter().map(|c| c.key.as_str())` without
  touching the helper or the handler shape.
- `server/src/server.rs:58-67` — writable-roots wiring reads
  `cfg.doc_paths.get("work")`. **This single change unblocks the
  visualiser end-to-end.** At `AppState::build` add fail-fast
  validation that **the config key is present**: `cfg.doc_paths.get("work")`
  returning None indicates a launcher-contract violation (rather
  than a fresh-install state) and the server exits non-zero with a
  precise message naming the key and pointing the operator at
  `/accelerator:migrate`. The same key-presence rule applies to
  `review_work`. **Tolerate a configured path that does not exist on
  disk** for both keys: a fresh repo with no `meta/work/` directory
  yet, or a workspace mid-recovery, should still launch — log a
  single startup info line and treat the directory as empty. The
  failure mode this plan was created to fix is silent missing-key,
  not missing-on-disk; conflating the two would refuse to launch in
  legitimate empty-state recovery scenarios.
- `server/src/sse_hub.rs:8-21` — variant labels emit `"work-items"`
  via the existing kebab-case serde.
- `server/src/file_driver.rs:594-616,724,762,791,825,866,879,918,952,997`
  — `seeded_write_driver` and test fixtures: rename `tickets`-keyed
  doc-paths entries to `work`; the directory literal in the helper
  becomes `work` once fixtures move (Step 1.5).

Run `cargo check`; iterate until clean. Run `cargo test`; confirm green.

### Step 1.5 — Move integration fixtures

**Approach**: rename fixtures atomically. This workspace uses jujutsu;
plain `mv` inside the active workspace is sufficient (jj snapshots
the working copy on its next operation). Run `jj st` before to
confirm a clean working copy and `jj st` after to confirm jj has
captured the rename.

```bash
cd skills/visualisation/visualise/server/tests/fixtures/meta
jj st  # confirm clean before moves
mv tickets work
cd work
mv 0001-first-ticket.md 0001-first-work-item.md
mv 0005-sse-test-ticket.md 0005-sse-test-work-item.md
cd -
jj st  # confirm jj sees the renames
```

Update fixture file content where the body text says "ticket" if any —
inspect the two files, replace where the meaning is "work-item", leave
generic English uses alone.

Update integration tests that hardcode the fixture path/filename:

- `server/tests/api_smoke.rs`, `api_docs.rs`, `api_docs_patch.rs`,
  `api_lifecycle.rs`, `api_related.rs`, `lifecycle_idle.rs`,
  `lifecycle_owner.rs`, `sse_e2e.rs` — search for `meta/tickets`,
  `0001-first-ticket`, `0005-sse-test-ticket`, replace.

Run `cargo test`; confirm green.

### Step 1.6 — Frontend rename: types, API helpers, components, tests

**Approach**: TypeScript will report every site. As with the server,
rename the test fixtures/expectations first, then chase the failures.

#### 1.6a — Update test expectations first

Rename in tests (red → green):

- `frontend/src/api/types.test.ts` (if any) — change `'tickets'` →
  `'work-items'`, `IndexEntry.ticket` → `IndexEntry.workItemRefs:
  []`, `Completeness.hasTicket` → `Completeness.hasWorkItem`
  (cluster-presence semantics unchanged). Add an assertion that
  fixture defaults satisfy `IndexEntry`'s new shape (e.g.
  `entry.type === 'work-items'`, `entry.workItemRefs` is `[]`,
  `entry.workItemId` is appropriate) so default-fixture drift surfaces
  as a single failure here rather than scattered across tests.
- `frontend/src/api/wiki-links.test.ts:25` and surrounds — keep the
  numeric inner pattern; just change the literal prefix from
  `[[TICKET-1]]` to `[[WORK-ITEM-1]]`. Pattern-aware widening lands
  in Phase 2.
- `frontend/src/api/use-doc-events.test.ts` — every `'tickets'` →
  `'work-items'`.
- `frontend/src/api/use-wiki-link-resolver.test.tsx` — same.
- `frontend/src/api/test-fixtures.ts:8,15` — defaults updated.
- `frontend/src/api/ticket.test.ts` → file rename to
  `work-item.test.ts`; same for `use-move-ticket.test.tsx` →
  `use-move-work-item.test.tsx`.
- `frontend/src/routes/kanban/KanbanBoard.test.tsx:57,65,74-99,159-204`
  — every `type: 'tickets'`, every `relPath: 'meta/tickets/...'`,
  every literal "ticket"/"tickets" in expected aria-live and toast
  strings.
- `frontend/src/routes/kanban/TicketCard.test.tsx` → file rename to
  `WorkItemCard.test.tsx`; every fixture path and assertion.
- `frontend/src/routes/kanban/announcements.test.ts:6-58` — every
  expected string literal "ticket {id}: {title}" → "work item {id}:
  {title}".

#### 1.6b — Production rename

- `frontend/src/api/types.ts:4-7,13-17,48,98,128,139,158-175`:
  `DocTypeKey` literal `'tickets'` → `'work-items'`; add
  `'work-item-reviews'`; `DOC_TYPE_KEYS` runtime array updated;
  `IndexEntry.ticket: string | null` → `IndexEntry.workItemRefs:
  string[]`; rename `Completeness.hasTicket` → `Completeness.hasWorkItem`
  (cluster-presence flag, matching server-side rename);
  `LIFECYCLE_PIPELINE_STEPS[0]` becomes
  `{ key: 'hasWorkItem', docType: 'work-items', label: 'Work item',
  placeholder: 'no work item yet' }` — same shape as sibling steps
  (`hasPlan`, `hasDecision`, etc.) so the rendering predicate stays
  uniform across the pipeline;
  `STATUS_COLUMNS` widens to the **seven-status default set**
  (`draft | ready | in-progress | review | done | blocked |
  abandoned`) so legacy `proposed` falls into "Other" but all seven
  template statuses round-trip through drag-drop in Phase 1. Phase 3
  replaces the hardcoded list with a server-driven one without
  changing the rendering shape.
- `frontend/src/api/fetch.ts:29-54`: rename `patchTicketFrontmatter` →
  `patchWorkItemFrontmatter`. URL path is generic, no change.
- File rename: `frontend/src/api/ticket.ts` → `work-item.ts`; rename
  exports `parseTicketNumber` → `parseWorkItemId` (destination name —
  returns string in Phase 2, but rename here so Phase 2 only widens
  the body), `groupTicketsByStatus` → `groupWorkItemsByStatus`. Inner
  regex unchanged in this phase.
- File rename: `frontend/src/api/use-move-ticket.ts` →
  `use-move-work-item.ts`; rename
  `MoveTicketVars` → `MoveWorkItemVars`,
  `MoveTicketContext` → `MoveWorkItemContext`,
  `useMoveTicket` → `useMoveWorkItem`; cache keys
  `queryKeys.docs('work-items')`.
- `frontend/src/api/wiki-links.ts:6,20,22,45,79-86,110`:
  - Line 20: `WIKI_LINK_PATTERN = /\[\[(ADR|WORK-ITEM)-(\d{1,6})\]\]/g`
    (still digits-only inner pattern; Phase 2 widens).
  - Line 22: `WikiLinkKind = 'ADR' | 'WORK-ITEM'`.
  - `ticketByNumber` → `workItemById` (destination name; Phase 2
    changes the underlying map key type from number to string
    without renaming).
- `frontend/src/components/MarkdownRenderer/wiki-link-plugin.ts:17,77`:
  Resolver type updated.
- `frontend/src/api/use-wiki-link-resolver.ts:19,36-39,41,45`:
  `queryKeys.docs('work-items')`, `fetchDocs('work-items')`.
- `frontend/src/api/use-doc-events.ts:30,74`: cache invalidation gate
  on `event.docType === 'work-items'`.
- `frontend/src/routes/kanban/KanbanBoard.tsx:11,12,15,21-22,28,33,37,54-55,152`:
  imports updated; "Other" column copy updated; toasts ("loading the
  work items", "This work item was updated by another editor", "The
  work item could not be saved"); `queryKeys.docs('work-items')`,
  `fetchDocs('work-items')`.
- File rename: `frontend/src/routes/kanban/TicketCard.tsx` →
  `WorkItemCard.tsx`. Component rename, props rename, CSS module
  rename (`TicketCard.module.css` → `WorkItemCard.module.css`).
  Critically, line 39's hardcoded link
  `params={{ type: 'tickets', fileSlug }}` becomes
  `params={{ type: 'work-items', fileSlug }}`.
- `frontend/src/routes/kanban/KanbanColumn.tsx:3,19,44,48`: import
  the renamed component; `workItemWord = count === 1 ? 'work item'
  : 'work items'`; `'No work items'` empty-state.
- `frontend/src/routes/kanban/announcements.ts:9,21-49`: rename
  `ticketNumberFromRelPath` → `workItemIdFromRelPath` (destination
  name); every aria-live string updated.
- `frontend/src/api/test-fixtures.ts:8,15`: default `type` and
  field names.
- `frontend/e2e/kanban.spec.ts:6-11,46,54,61,89,96,108`: literal
  paths `tests/fixtures/meta/tickets/` → `tests/fixtures/meta/work/`,
  filenames updated to `0001-first-work-item.md` and
  `0005-sse-test-work-item.md`.
- `frontend/e2e/kanban-conflict.spec.ts`, `frontend/e2e/wiki-links.spec.ts`,
  `frontend/e2e/navigation.spec.ts`: search-and-replace any path
  literals or wiki-link literals.

Run `npm run typecheck`; clean.
Run `npm run test -- --run`; clean.
Run `npx playwright test`; clean.

### Step 1.7 — Rename-completion verification

The Phase 12 plan-frontmatter `status: draft` → `status: complete`
fix-up (originally in this step) has moved to a standalone trivial
change tracked in Phase 4 Step 4.5; it does not belong inside the
work-item-terminology PR.

Re-grep the **whole repo** (not just the visualiser tree) for any
remaining `ticket`/`Ticket` references in comments, log messages,
error strings, CSS classes, body-text wiki-links, and operator
tooling. Triage each result individually rather than excluding
broad path patterns. Allow only specific, named exceptions
(historical CHANGELOG entries; sourcemaps regenerated by build).

```bash
# Lowercase / case-insensitive pass over the visualiser tree.
# Drop the .test. and /fixtures/ exclusions — leftover references
# inside test files are exactly what we need to catch.
grep -rni "ticket" skills/visualisation/visualise/ \
    | grep -v -E '(CHANGELOG|\.module\.css\.map$)' \
    || echo "clean (visualiser tree)"

# CamelCase residue across the visualiser tree (catches Ticket,
# TicketCard, Tickets, etc. that case-insensitive grep over UTF-8
# strings can miss in some shells).
grep -rn -E '\bTicket[A-Z]|\bTickets?\b' skills/visualisation/visualise/ \
    || echo "clean (CamelCase)"

# Body-text wiki-links across the whole repo: existing
# [[TICKET-NNNN]] references in plans/research/ADRs would silently
# stop resolving after Phase 1.
grep -rn '\[\[TICKET-' . \
    --include='*.md' --include='*.mdx' \
    --exclude-dir='.git' --exclude-dir='target' --exclude-dir='node_modules' \
    || echo "clean (no [[TICKET-...]] body-text refs)"

# Old error-literal: log-pattern matchers in operator tooling
# would silently stop alerting after the rename.
grep -rn 'only tickets are writable\|OnlyTicketsAreWritable' . \
    --exclude-dir='.git' --exclude-dir='target' --exclude-dir='node_modules' \
    || echo "clean (no stale error-literal matchers)"
```

For any residual `[[TICKET-NNNN]]` body-text wiki-links found by
the third grep, choose between (a) authoring a one-shot rename script
to migrate them and including it in this PR, or (b) accepting them as
dead-link debt and recording the call in the Phase 4 ADR. Default to
(a) if the count is small (<20).

### Success Criteria — Phase 1

#### Automated Verification:

- [ ] Server tests pass: `cd skills/visualisation/visualise/server && cargo test --all-features`
- [ ] Server clippy clean: `cd skills/visualisation/visualise/server && cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Frontend typecheck passes: `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [ ] Frontend unit tests pass: `cd skills/visualisation/visualise/frontend && npm run test -- --run`
- [ ] Playwright e2e passes: `cd skills/visualisation/visualise/frontend && npx playwright test`
- [ ] Shell tests pass: `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`
- [ ] Top-level visualise tests pass: `cd skills/visualisation/visualise && make test` (if a Makefile target exists; otherwise the above blocks)
- [ ] All four greps in Step 1.7 print "clean ..." (visualiser tree, CamelCase, body-text wiki-links, error-literal matchers)
- [ ] `GET /api/types` returns 11 entries including `work-items` and `work-item-reviews`
- [ ] PATCH a work-item to each of the seven default statuses (`draft|ready|in-progress|review|done|blocked|abandoned`) succeeds
- [ ] PATCH a work-item to an unknown status returns 400 with `ApiError::UnknownKanbanStatus` and `accepted_keys` listing the seven defaults
- [ ] Server fails fast at boot when launched against a config missing the `doc_paths.work` key (integration test); tolerates a configured path that does not exist on disk and serves the kanban as empty (integration test)
- [ ] `GET /api/docs/work-item-reviews` returns `200 []` when `meta/reviews/work/` does not exist on disk

#### Manual Verification:

- [ ] Launch the visualiser against this workspace (`/accelerator:visualise`); sidebar shows 11 doc types
- [ ] Kanban renders the 30 legacy `meta/work/` files; cards display ID and title; legacy `proposed` lands in "Other"
- [ ] Drag a card from "Ready" to "In progress"; status persists; refresh confirms
- [ ] Drag a card from "Other" (legacy `proposed`) to "Ready"; status persists (round-trip on legacy-schema content works because Phase 1 already accepts the seven-state set)
- [ ] `[[WORK-ITEM-0001]]` link in a plan resolves to the right file; `[[ADR-0023]]` still resolves
- [ ] `meta/reviews/work/` (empty or absent) appears as a tab and shows "no items" empty state
- [ ] Library view of `meta/work/0001-...md` renders frontmatter and body correctly
- [ ] Launch against a synthetic project lacking `paths.work` config — launcher exits with the migrate-pointer message; visualiser does not start in a broken state

---

## Phase 2: Configurable ID pattern support

### Overview

Make the visualiser pattern-aware. The launcher reads the configured
`work.id_pattern` and compiles it via
`skills/work/scripts/work-item-pattern.sh`; the server consumes the
compiled regex at boot and uses it for slug derivation, ID lookup,
and wiki-link resolution.

### Step 2.0 — Establish a green baseline (Phase 1 must end green)

Run the full test suite; confirm green before starting.

### Step 2.1 — Server: pattern-aware slug and indexer (TDD: red → green)

#### 2.1a — Tests first

**File**: `skills/visualisation/visualise/server/src/slug.rs`

Drive the test fixtures from regexes the **pattern compiler actually
produces** rather than hand-rolled assumptions. Add a small test helper
that shells out to `${PLUGIN_ROOT}/skills/work/scripts/work-item-pattern.sh
--compile-scan <pattern> [<project-code>]` once at suite startup and
returns the regex string. This both pins the compiler ↔ visualiser
contract and ensures the tests follow real-world output (which permits
mixed-case and digit-bearing project codes via `[A-Za-z][A-Za-z0-9]*`,
not the over-restrictive `[A-Z]+`).

Add failing tests for `derive_work_item_with_regex`:

```rust
#[test]
fn work_items_with_project_pattern_strip_full_id_prefix() {
    let pattern_re = compile_scan_via_cli("{project}-{number:04d}", Some("PROJ"));
    let cases = &[
        ("PROJ-0042-ship-the-thing.md", Some("ship-the-thing")),
        ("PROJ-1-short.md", Some("short")),
        ("PROJ-0042.md", None),
        ("malformed.md", None),
    ];
    for (input, expected) in cases {
        let got = derive_work_item_with_regex(&pattern_re, input);
        assert_eq!(got.as_deref(), *expected, "input={input}");
    }
}

#[test]
fn work_items_with_lowercase_or_digit_project_code() {
    // Real-world project codes contain digits and mixed case;
    // assert the compiler's regex accepts them, not just [A-Z]+.
    let pattern_re = compile_scan_via_cli("{project}-{number:04d}", Some("web2"));
    assert_eq!(
        derive_work_item_with_regex(&pattern_re, "web2-0042-foo.md").as_deref(),
        Some("foo")
    );
}

#[test]
fn work_items_default_numeric_pattern_still_works() {
    let pattern_re = compile_scan_via_cli("{number:04d}", None);
    assert_eq!(
        derive_work_item_with_regex(&pattern_re,
            "0001-three-layer-review-system-architecture.md").as_deref(),
        Some("three-layer-review-system-architecture")
    );
}

#[test]
fn invalid_id_pattern_fails_compilation_with_clear_message() {
    // The launcher invokes --compile-scan; this test verifies the
    // contract the launcher relies on: invalid pattern → non-zero
    // exit + clear stderr (no panic, no silent fallback).
    let result = compile_scan_cli_status("not-a-valid-pattern", None);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("E_PATTERN_") || stderr.contains("invalid"),
        "stderr should name the failure: {stderr}");
}
```

**File**: `skills/visualisation/visualise/server/src/indexer.rs` (test
module or new `tests/indexer_pattern.rs`)

Add failing tests for string-keyed lookup:

```rust
#[test]
fn indexer_keys_work_items_by_full_id_string() {
    // Seed a fixture with PROJ-0042-foo.md and assert
    // indexer.lookup_work_item("PROJ-0042") returns its path.
}

#[test]
fn indexer_falls_back_to_zero_padded_default_for_legacy_files() {
    // Seed 0001-foo.md; assert lookup_work_item("0001") returns it.
}
```

Run `cargo test`; confirm fail (no `derive_work_item_with_regex`,
no `lookup_work_item`).

#### 2.1b — Implementation

**File**: `skills/visualisation/visualise/server/src/config.rs`

Extend the schema. The compiled `Regex` is the runtime field; the
raw string is held only for diagnostics. Use a fallible constructor
so there is exactly one boot-time site that can fail to compile.

```rust
pub struct RawWorkItemConfig {
    pub scan_regex: String,
    pub id_pattern: String,
    pub default_project_code: Option<String>,
}

pub struct WorkItemConfig {
    pub scan_regex: regex::Regex,       // compiled at boot
    pub scan_regex_raw: String,         // diagnostics only
    pub id_pattern: String,
    pub default_project_code: Option<String>,
}

impl WorkItemConfig {
    pub fn from_raw(raw: RawWorkItemConfig) -> Result<Self, ConfigError> {
        let scan_regex = regex::Regex::new(&raw.scan_regex)
            .map_err(|e| ConfigError::InvalidScanRegex { source: e, raw: raw.scan_regex.clone() })?;
        Ok(Self { scan_regex, scan_regex_raw: raw.scan_regex, id_pattern: raw.id_pattern, default_project_code: raw.default_project_code })
    }
}
```

Compile the regex once at boot via `WorkItemConfig::from_raw`. On
failure, exit non-zero with a precise message naming the invalid
pattern (mirrors the fail-fast policy adopted in Phase 1 for
missing `doc_paths.work`). Downstream code accepts `&WorkItemConfig`
and never re-compiles. Add a test asserting `from_raw` returns
`Err(InvalidScanRegex)` on a malformed pattern.

**File**: `slug.rs`: add `derive_work_item_with_regex` taking a
`&regex::Regex`. Capture group 1 is the ID; the slug is
`stem[match_end..]` (after the trailing `-`). Update the existing
`derive` function's `WorkItems` arm to take the regex from
`Config`.

**File**: `indexer.rs`: change `Indexer.work_item_by_id` from
`HashMap<u32, PathBuf>` (Phase 1 keyed on parsed number) to
`HashMap<String, PathBuf>` (string key accommodates `PROJ-0042`). The
field name is unchanged — Phase 1 already took the destination name.
Update:

- `parse_work_item_id` body widens from returning `Option<u32>` to
  `Option<String>` (extracted via the configured regex). Name unchanged.
- `work_item_id_from_entry` body returns `Option<String>`. Name unchanged.
- `IndexEntry.workItemId: Option<String>` field added. **Field is
  filename-derived (via the configured regex), not frontmatter-derived
  — clarify in the doc-comment**. Three states: (1) regex matches →
  `Some(<full-string-id>)`; (2) regex doesn't match → `None`, file is
  excluded from `work_item_by_id` (but still indexed and renderable);
  (3) doc isn't a work-item type → field is conceptually N/A and
  emitted as `None` for non-work-item entries.
- All `:477-493,531-538` insertion sites: key on the
  `parse_work_item_id` result.
- **Mixed-pattern handling with fallback admission**: a workspace
  may legitimately have both bare-numeric files (`0001-foo.md`,
  legacy) and project-prefixed files (`PROJ-0001-bar.md`, current)
  under the same indexer — typical during a pattern-config
  rollout. The admission rule is two-pass:
  1. **Primary**: files whose stem matches the configured scan
     regex contribute to `work_item_by_id` keyed on the
     full-string ID extracted by the regex.
  2. **Fallback** (project-prefixed pattern only): files that do
     *not* match the configured regex but *do* match the
     bare-numeric form (`^\d+-`) are also admitted, keyed on the
     canonical `<DEFAULT_PROJECT_CODE>-<padded>` ID. This means
     `0042-foo.md` under a `{project}-{number:04d}` config with
     `default_project_code: "PROJ"` is admitted as
     `"PROJ-0042"` — matching the canonicalisation rule for
     bare-numeric cross-refs (case 2) so legacy `parent: 42`
     references resolve correctly.

  Files matching neither form (e.g. `README.md`) are still indexed
  (slug derived from the stem verbatim, less the date if
  applicable) but excluded from ID lookup. Add integration tests
  for: primary-only (default pattern, mixed bare-numeric files),
  fallback-admitted (project pattern, legacy bare-numeric file
  resolves to the canonical project-prefixed ID), and
  no-fallback-without-default-project-code (project pattern with
  no `default_project_code` configured — bare-numeric files are
  excluded from ID lookup but still indexed).

#### 2.1c — Acceptance test

Add a Rust integration test under `server/tests/` (e.g.
`api_work_item_pattern.rs`):

- Fixture: `tests/fixtures/meta/work-pattern/PROJ-0042-foo.md`,
  `PROJ-0007-bar.md`, with a `Config.work_item.scan_regex =
  "^(PROJ-\d+)-"`.
- Assertions:
  - `GET /api/docs/work-items` lists both files with their
    correct slug ("foo", "bar") and `workItemId` ("PROJ-0042",
    "PROJ-0007").
  - Lifecycle clusters group by slug correctly.

### Step 2.2 — Launcher: invoke `--compile-scan` (TDD: shell test)

**File**: `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`

Add a test: when the project's `.claude/accelerator.md` contains
`work.id_pattern: "{project}-{number:04d}"` and
`work.default_project_code: "PROJ"`, the produced `config.json`
contains `work_item.scan_regex` matching the compiler output and
`work_item.id_pattern: "{project}-{number:04d}"`. Run; confirm fail.

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`

Add reads:

```bash
ID_PATTERN="$(${PLUGIN_ROOT}/scripts/config-read-value.sh work.id_pattern '{number:04d}')"
PROJECT_CODE="$(${PLUGIN_ROOT}/scripts/config-read-value.sh work.default_project_code '')"
SCAN_REGEX="$(${PLUGIN_ROOT}/skills/work/scripts/work-item-pattern.sh \
    --compile-scan "$ID_PATTERN" "$PROJECT_CODE")"
```

Validate the pattern at launch time (the compiler exits non-zero on
invalid). Add to the jq invocation:

```bash
--arg scan_regex "$SCAN_REGEX" \
--arg id_pattern "$ID_PATTERN" \
--arg default_project_code "$PROJECT_CODE" \
# ...
work_item: { scan_regex: $scan_regex, id_pattern: $id_pattern,
             default_project_code: ($default_project_code | select(. != "")) }
```

Run shell test; confirm green.

### Step 2.3 — Frontend: pattern-aware wiki-link regex (TDD: red → green)

**Approach**: do not maintain a parallel hand-written regex on the
frontend. The pattern compiler already produces a scan regex of the
form `^PROJ-([0-9]+)-` (literal project value substituted in) under
project-prefixed patterns, and `^([0-9]+)-` under default. Expose
the *configured project code* and the *number-width regex fragment*
to the frontend via the existing config endpoint, and let the
frontend build a wiki-link regex from those two pieces. The frontend
does not receive the full scan regex (which is anchored at start of
filename and wraps a capture group different from what wiki-link
matching needs) — it receives the literal project code (or empty)
and reuses the digits portion verbatim.

This matches the compiler's *actual* grammar: a project code is a
single literal token (per `wip_validate_pattern` rule 5 in
`work-item-common.sh:133` — no hyphens permitted in project codes),
and the wiki-link inner pattern is therefore either `<PROJECT>-\d+`
or `\d+` depending on configuration. **Multi-segment project codes
like `ACME-CORE` are not supported by the compiler today and are out
of scope for this plan**; if such support is required later, that
work owns extending `wip_validate_pattern` rule 5 and adding a new
`--compile-wiki-link` subcommand.

#### 2.3a — Tests first

**File**: `frontend/src/api/wiki-links.test.ts`

Add failing tests that build the regex from server-supplied
config (mocked):

```ts
function buildPattern(projectCode: string | null): RegExp {
  // Matches the compiler's actual grammar: literal project token + digits,
  // or just digits when no project code is configured.
  const innerWorkItem = projectCode
    ? `${projectCode}-\\d+|\\d+`
    : `\\d+`;
  return new RegExp(`\\[\\[(ADR|WORK-ITEM)-(${innerWorkItem})\\]\\]`, 'g');
}

it('matches project-prefixed work-item ids under a project pattern', () => {
  const pattern = buildPattern('PROJ');
  const text = 'See [[WORK-ITEM-PROJ-0042]] for context';
  const matches = [...text.matchAll(pattern)];
  expect(matches).toHaveLength(1);
  expect(matches[0][2]).toBe('PROJ-0042');
});

it('falls back to bare numeric under a project pattern', () => {
  // Legacy bare-numeric refs in a workspace that has since switched
  // to a project-prefixed pattern still match the digits-only branch.
  const pattern = buildPattern('PROJ');
  const text = 'See [[WORK-ITEM-0007]] for legacy context';
  const matches = [...text.matchAll(pattern)];
  expect(matches).toHaveLength(1);
  expect(matches[0][2]).toBe('0007');
});

it('matches default-pattern work-item ids when no project code is configured', () => {
  const pattern = buildPattern(null);
  const text = 'See [[WORK-ITEM-0042]] and [[ADR-0023]]';
  const matches = [...text.matchAll(pattern)];
  expect(matches).toHaveLength(2);
});

it('does not match multi-segment project codes (out of scope)', () => {
  // Pinned negative test: the compiler grammar forbids hyphens in
  // project codes, so multi-segment ids like ACME-CORE-0042 are not
  // expected to resolve. If this changes, update the pattern compiler
  // first (rule 5) and the launcher second.
  const pattern = buildPattern('ACME');
  const text = 'See [[WORK-ITEM-ACME-CORE-0042]]';
  const matches = [...text.matchAll(pattern)];
  expect(matches).toHaveLength(0);
});

it('resolves a project-prefixed wiki-link via workItemById map', () => {
  // Build a map keyed on full-string ID; assert resolver returns the entry.
});
```

#### 2.3b — Implementation

**File**: `frontend/src/api/types.ts`

Add `IndexEntry.workItemId: string | null` (the server emits this in
Phase 2).

**File**: `frontend/src/api/wiki-links.ts`

Replace the hardcoded `WIKI_LINK_PATTERN` constant with a builder
that takes the server-supplied configured project code (or `null`
when no project code is configured):

```ts
export function buildWikiLinkPattern(projectCode: string | null): RegExp {
  const innerWorkItem = projectCode
    ? `${escapeRegExp(projectCode)}-\\d+|\\d+`
    : `\\d+`;
  return new RegExp(`\\[\\[(ADR|WORK-ITEM)-(${innerWorkItem})\\]\\]`, 'g');
}
```

(The `escapeRegExp` helper exists already or is added inline; the
project code is a literal token from config and may contain regex
metacharacters in a misconfigured edge case — escape defensively.)

The resolver and the markdown plugin take the compiled `RegExp` as
an argument rather than reading a module-level constant.

Replace `workItemByNumber: Map<number, IndexEntry>` with
`workItemById: Map<string, IndexEntry>` keyed on the full-string ID
verbatim from `IndexEntry.workItemId`.

**File**: `frontend/src/components/MarkdownRenderer/wiki-link-plugin.ts`

Resolver type: `(prefix: 'ADR' | 'WORK-ITEM', id: string) => ...`.
Plugin accepts the compiled pattern as a parameter rather than
importing a module-level constant.

**Server side**: The existing config endpoint (the one the
launcher's `config.json` flows through) gains a
`work_item.default_project_code` field carrying the configured value
(or omitted when unconfigured — the field is already in
`WorkItemConfig`; this just exposes it to the frontend). No new
endpoint needed; no `--compile-wiki-link` subcommand.

Run `npm run test -- --run`; confirm green.

### Step 2.4 — End-to-end Playwright test

**File**: `frontend/e2e/wiki-links.spec.ts`

Add a fixture under `frontend/e2e/fixtures/` with a project-pattern
work-item and a plan that links to it. Assert the wiki-link in the
rendered plan resolves and the click navigates to the work-item.

### Success Criteria — Phase 2

#### Automated Verification:

- [x] Server tests pass: `cargo test --all-features`
- [x] Frontend tests pass: `npm run test -- --run`
- [x] Playwright e2e passes including the new project-pattern spec
- [x] Pattern compiler validation rejects invalid `work.id_pattern` at launch with a clear error (asserted via integration test, not manual)
- [x] `WorkItemConfig::from_raw` returns `Err(InvalidScanRegex)` on a malformed pattern (unit test)
- [x] Mixed-pattern fixture (some files match the configured regex, some don't) indexes per the precedence rule (integration test)
- [x] Frontend regex builder consumes server-supplied `default_project_code` and matches both `[[WORK-ITEM-PROJ-0042]]` and `[[WORK-ITEM-0007]]` (legacy bare-numeric fallback) under a project-prefixed configuration; matches `[[WORK-ITEM-0042]]` only under the default pattern. Multi-segment project codes (e.g. `[[WORK-ITEM-ACME-CORE-0042]]`) are out of scope and explicitly do not match.

#### Manual Verification:

- [ ] In a fresh project configured with `work.id_pattern: "{project}-{number:04d}"` and `work.default_project_code: "PROJ"`, three seeded work-items (`PROJ-0001`, `PROJ-0002`, `PROJ-0007`) appear in the kanban
- [ ] Drag-drop status changes succeed across the seven default statuses
- [ ] `[[WORK-ITEM-PROJ-0042]]` resolves; `[[WORK-ITEM-PROJ-9999]]` (non-existent) shows the unresolved-link state
- [ ] Default-pattern work-items in this workspace still render after Phase 2 changes (regression check)

---

## Phase 3: Configurable kanban columns + multi-field cross-references

### Overview

Two independent additive features. **Neither requires a breaking type
change** — Phase 1 already took the destination shapes (`status:
String` validated against a hardcoded seven-status default; pure
`read_ref_keys` parser reading `ticket:` or `work-item:`). Phase 3
only swaps the validation source from the hardcoded defaults to
config and extends the parser to read `parent:` and `related:`.

1. The kanban column set becomes configurable per project, defaulting
   to the seven `templates/work-item.md:7` statuses (already the
   Phase 1 default). The validation source moves from the hardcoded
   constant in `api/docs.rs` to a `Vec<KanbanColumn>` injected via
   `Config`.
2. Frontmatter cross-references **extend** (not replace) the Phase 1
   single-key read: `read_ref_keys` now reads `work-item:`,
   `parent:`, and `related:`, with the indexer aggregating /
   deduplicating / canonicalising into a `Vec<String>` for storage
   in `IndexEntry.workItemRefs`. A reverse cross-ref index lights
   up the "Related artifacts" aside in library view.

### Step 3.0 — Establish a green baseline

### Step 3.1 — Configurable kanban columns (TDD: red → green)

#### 3.1a — Launcher read of `visualiser.kanban_columns`

**Approach**: reuse the existing config primitives. The plugin
already provides `scripts/config-read-value.sh` (awk-based scalar
read) and `scripts/config-common.sh:72-85`'s `config_parse_array`
helper (splits an inline YAML array of the form `[a, b, c]` by
string ops). Operators specify `visualiser.kanban_columns` as an
inline array — matching the inline-array convention already
established by other plugin config keys — and the launcher reads
it via `config-read-value.sh visualiser.kanban_columns '<defaults>'`
followed by `config_parse_array` to split the value into individual
column entries. Each entry is the column key (label defaults to a
title-cased version of the key, or operators may provide
`{key, label}` maps in a future extension; this plan ships
key-only). The launcher then composes the parsed array into
`config.json` via the existing `jq -nc --argjson columns ...`
invocation alongside `work_item.scan_regex` etc.

Block YAML form (multi-line lists with leading dashes) is **not
supported in this phase** — operators must use the inline-array
form. This matches the established plugin convention and avoids
introducing a new YAML-parsing pathway. Future phases may extend
the helper if block form is needed.

Default fallback: when the YAML omits `visualiser.kanban_columns`,
`config-read-value.sh` returns the configured default and the
launcher emits the seven template-status defaults. Empty list
(`visualiser.kanban_columns: []`) is detected by the launcher
(`config_parse_array` returns zero elements) and triggers a
non-zero exit with a clear error consistent with the server-side
reject-at-boot rule. Malformed inline-array syntax (e.g. unclosed
bracket) is caught by `config_parse_array` and produces the same
exit behaviour.

**File**: `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`

Extend the Phase 1 shell test with cases asserting that:

1. Missing `visualiser.kanban_columns` → `config.json` carries the
   seven defaults `[draft, ready, in-progress, review, done,
   blocked, abandoned]`.
2. Configured custom inline array (e.g.
   `visualiser.kanban_columns: [ready, in-progress, review, done]`)
   → `config.json` carries exactly that list in order.
3. Empty list (`visualiser.kanban_columns: []`) → launcher exits
   non-zero with a clear error (consistent with the server-side
   reject-at-boot rule).
4. Malformed inline-array syntax (e.g. `[ready, in-progress`) →
   launcher exits non-zero.

Run; fail. Implement using the helpers above. Run; green.

#### 3.1b — Server tests first

**File**: `server/src/config.rs` test module

```rust
#[test]
fn kanban_columns_default_to_seven_template_statuses() {
    let cfg = Config::default();
    assert_eq!(
        cfg.kanban_columns.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
        vec!["draft", "ready", "in-progress", "review", "done", "blocked", "abandoned"]
    );
}

#[test]
fn kanban_columns_read_from_config() {
    // Parse a config with custom columns; assert.
}

#[test]
fn kanban_columns_empty_list_rejected_at_boot() {
    // Config with `visualiser.kanban_columns: []` → ConfigError, not silent fallback.
}

#[test]
fn kanban_columns_missing_field_falls_back_to_defaults() {
    // Config without the key at all → seven defaults (matches Phase 1).
}

#[test]
fn kanban_columns_malformed_yaml_rejected_at_boot() {
    // Non-array, non-object shape → ConfigError surfacing the parse failure.
}
```

**File**: `server/src/api/docs.rs` test module (or `tests/api_docs_patch.rs`)

```rust
#[test]
fn patch_status_accepts_configured_column_key() {
    // Configure cols [ready, in-progress, done]; PATCH with status=ready; expect 200.
}

#[test]
fn patch_status_rejects_unconfigured_value_with_400_and_accepted_keys() {
    // PATCH with status=foo; expect 400; ApiError::UnknownKanbanStatus
    // body lists ["ready","in-progress","done"].
}

#[test]
fn patch_status_validation_is_case_sensitive_no_trim() {
    // PATCH with status="In Progress" (label-cased) or " ready " (whitespace)
    // returns 400. Documents the chosen normalisation contract.
}

#[test]
fn patch_status_seeds_in_other_then_moves_to_configured_column() {
    // Seed a fixture *on disk* (bypassing the API) with status=proposed
    // (legacy, falls into Other under the configured cols); PATCH to
    // status=ready; expect 200; refresh shows the card in the
    // configured column. This exercises the read-side fall-through
    // and the write-side admission of a configured target — it does
    // NOT round-trip back to proposed (see test below).
}

#[test]
fn patch_status_to_other_swimlane_value_rejected() {
    // PATCH with a value not in configured cols (e.g. status=proposed
    // when the configured set is [ready, in-progress, done]) returns
    // 400 with ApiError::UnknownKanbanStatus. Pins the boundary
    // contract: Other is read-only on the write path; cards leave it
    // by moving to a configured column and cannot be returned via
    // PATCH. Documented in ADR-0024 so the contract is durable.
    // Recovery for accidental drag-out: direct file edit, then VCS
    // revert (matches the project's broader 'destructive op safety
    // via VCS' convention).
}
```

#### 3.1c — Server implementation

- `config.rs`: add `KanbanColumn { key: String, label: String }` and
  `Config.kanban_columns: Vec<KanbanColumn>`. Validation rules:
  - missing `visualiser.kanban_columns` → fall back to the seven
    `templates/work-item.md:7` defaults (matches Phase 1 behaviour);
  - empty list (`visualiser.kanban_columns: []`) → reject at boot
    with a clear error (operator misconfiguration; not a fall-safe
    default);
  - malformed YAML → reject at boot via the existing config-load
    failure path.
- `patcher.rs`: **no change.** The patcher already takes a plain
  `&str` status (from Phase 1). Validation lives in `api/docs.rs`.
- `api/docs.rs`: change the validation source from the hardcoded
  seven-status default array to `cfg.kanban_columns.iter().map(|c|
  c.key.as_str())`. The `ApiError::UnknownKanbanStatus` variant
  declared in Phase 1 is reused unchanged; only the contents of
  `accepted_keys` change.
- New endpoint **`GET /api/kanban/config`** (dedicated, not an
  extension of `GET /api/types` — kanban config is a separate
  concern from doc-type metadata) returning
  `{ columns: KanbanColumn[] }`. Frontend hook `use-kanban-config.ts`
  aligns with the URL.

#### 3.1d — Frontend tests first

**File**: `frontend/src/routes/kanban/KanbanBoard.test.tsx`

```tsx
it('renders the configured column set from the server', async () => {
  // Mock GET /api/kanban/config returning four cols.
  // Render KanbanBoard; expect four <KanbanColumn>s.
});

it('falls back gracefully if config endpoint returns the seven defaults', async () => {
  // Mock seven cols; expect seven columns plus Other.
});
```

#### 3.1e — Frontend implementation

- `src/api/types.ts:158-175`: replace hardcoded `STATUS_COLUMNS`
  with an exported type `KanbanColumn { key: string; label: string }`.
- New `src/api/use-kanban-config.ts` querying
  `GET /api/kanban/config` via TanStack Query.
- `src/routes/kanban/KanbanBoard.tsx:189-196`: render
  `data.columns.map(...)`; "Other" still catches unknown statuses
  with a hover surface listing the configured keys.
- `src/routes/kanban/KanbanColumn.tsx`: count adapts.
- Drag-drop allowed targets: any configured column except Other.

### Step 3.2 — Multi-field cross-references (TDD: red → green)

**Semantics decided up front** (so tests pin them rather than
discover them):

- **Canonicalisation**: four cases, applied in order. The case-3 vs
  case-4 boundary is pinned by the predicate
  `^[A-Za-z][A-Za-z0-9]*-\d+$` (a single-token project code per
  `wip_validate_pattern` rule 5, followed by a hyphen and digits) —
  values matching the predicate go to case 3; everything else
  matching no earlier case goes to case 4.
  1. Bare numerics (input matches `^\d+$`) under default pattern
     (`{number:04d}`, no `{project}`): zero-pad via `format_string`.
     `42` → `"0042"`.
  2. Bare numerics under project-prefixed pattern with a
     configured `default_project_code: "PROJ"`: zero-pad and prefix.
     `42` → `"PROJ-0042"`.
  3. Already project-prefixed input (matches
     `^[A-Za-z][A-Za-z0-9]*-\d+$`, e.g. `"PROJ-0042"`) under any
     pattern: **pass through verbatim**. Under a project-prefixed
     pattern this is the canonical form. Under a default pattern
     the pass-through is preserved for forward compatibility but
     the cross-ref will not resolve unless the mixed-pattern
     fallback admission rule admits the target file (which today
     it does not, since the fallback is project-prefixed-only) —
     the silent drop happens at lookup time, not canonicalisation
     time.
  4. Anything else — non-numeric without a prefix (`"foo"`),
     digits-then-token (`"42-foo"`), token-then-empty (`"PROJ-"`),
     empty-then-digits (`"-0042"`), null, empty string, malformed —
     degrade to empty contribution. The function never panics on
     unparseable input.

  Reuse `wip_canonicalise_id` from
  `skills/work/scripts/work-item-common.sh:354` if its semantics
  match all four cases; otherwise port the logic to Rust and
  document the parity in ADR-0025.
- **Function decomposition**: `frontmatter::read_ref_keys` (landed
  in Phase 1) returns raw values verbatim (pure parser of YAML —
  no config awareness). The indexer composes: `read_ref_keys(fm)` →
  `dedup_refs(...)` → `canonicalise_refs(..., &cfg.work_item)` just
  before insertion into the reverse index. Tests in `frontmatter.rs`
  stay config-free.
- **Aggregation**: all three keys (`work-item:`, `parent:`,
  `related:`) contribute equally; no precedence between them. A doc
  with both `work-item: 0042` and `parent: 0007` produces a vec
  containing both. Self-reference (a doc whose own ID appears in any
  of its ref keys) is filtered out before the reverse index is built
  — a doc never appears in its own `referencedBy`.
- **Conflicting / malformed shapes**: `parent: null`, `related:
  null`, `related: ""` → empty contribution. `related: "0007"` (string
  scalar instead of array) → single-element contribution. `parent:
  [0001, 0002]` (array instead of scalar) → both contribute. YAML
  parse failures degrade to empty vec at the indexer call site (the
  function never panics on a single malformed file).
- **Reverse-index composition with ADR-0017**: the existing
  `reviews_by_target` map and the new work-item reverse-ref map both
  populate a single wire-level field, `IndexEntry.referencedBy:
  Vec<{ kind: 'plan-review' | 'work-item-ref', relPath: string }>`.
  In the in-memory indexer they remain separate maps to preserve
  ADR-0017's lock-ordering guarantees; they are merged into the
  emitted vec at `IndexEntry` construction time. Document the merge
  ordering and the lock-acquisition sequence in the new ADR-0025
  (Step 4.1).

#### 3.2a — Tests first

**File**: `server/src/frontmatter.rs` test module (config-free)

```rust
#[test]
fn read_ref_keys_reads_work_item_key() {
    let fm = parse(b"---\nwork-item: \"0042\"\n---\n").unwrap();
    assert_eq!(read_ref_keys(&fm), vec![RawRef::from("0042")]);
}

#[test]
fn read_ref_keys_reads_parent_and_related() {
    let fm = parse(b"---\nparent: 0007\nrelated: [0001, 0002]\n---\n").unwrap();
    let refs = read_ref_keys(&fm);
    assert_eq!(refs.len(), 3);
}

#[test]
fn read_ref_keys_handles_scalar_related_as_single_element() {
    let fm = parse(b"---\nrelated: \"0007\"\n---\n").unwrap();
    assert_eq!(read_ref_keys(&fm), vec![RawRef::from("0007")]);
}

#[test]
fn read_ref_keys_handles_array_parent_as_multi_element() {
    let fm = parse(b"---\nparent: [0001, 0002]\n---\n").unwrap();
    assert_eq!(read_ref_keys(&fm).len(), 2);
}

#[test]
fn read_ref_keys_handles_null_and_empty_string_as_empty() {
    for body in [
        b"---\nparent: null\n---\n" as &[u8],
        b"---\nrelated: null\n---\n",
        b"---\nrelated: \"\"\n---\n",
        b"---\n---\n",
    ] {
        let fm = parse(body).unwrap();
        assert!(read_ref_keys(&fm).is_empty(), "body={:?}", body);
    }
}

#[test]
fn read_ref_keys_handles_int_and_string_as_equivalent_raw() {
    let int_fm = parse(b"---\nparent: 42\n---\n").unwrap();
    let str_fm = parse(b"---\nparent: \"42\"\n---\n").unwrap();
    assert_eq!(read_ref_keys(&int_fm), read_ref_keys(&str_fm));
}
```

**File**: `server/src/indexer.rs` test module (config-aware)

```rust
#[test]
fn canonicalise_refs_pads_bare_numeric_under_default_pattern() {
    let cfg = WorkItemConfig::default();  // {number:04d}, no project
    let raw = vec![RawRef::from("42"), RawRef::from("0007")];
    assert_eq!(canonicalise_refs(raw, &cfg),
        vec!["0042".to_string(), "0007".to_string()]);
}

#[test]
fn canonicalise_refs_prefixes_default_project_under_project_pattern() {
    // pattern={project}-{number:04d}, default_project_code=PROJ
    // bare numeric 42 becomes PROJ-0042; already-prefixed PROJ-0007 stays.
}

#[test]
fn canonicalise_refs_dedups_after_canonicalisation() {
    // Heterogeneous input ("42", 42, "0042") under default pattern
    // collapses to a single "0042".
}

#[test]
fn canonicalise_refs_passes_prefixed_input_through_under_default_pattern() {
    // Cross-ref to a project-prefixed work-item from a workspace
    // currently configured with a default pattern: pass through
    // verbatim. Indexer lookup decides whether the target resolves.
    let cfg = WorkItemConfig::default();  // {number:04d}, no project
    let raw = vec![RawRef::from("PROJ-0042")];
    assert_eq!(canonicalise_refs(raw, &cfg), vec!["PROJ-0042".to_string()]);
}

#[test]
fn canonicalise_refs_drops_malformed_input() {
    // Non-numeric, non-prefixed, malformed values degrade to empty.
    let cfg = WorkItemConfig::default();
    let raw = vec![RawRef::from("not-a-valid-id"), RawRef::from("")];
    assert_eq!(canonicalise_refs(raw, &cfg), Vec::<String>::new());
}

#[test]
fn canonicalise_refs_case_3_vs_case_4_boundary() {
    // Pin the predicate `^[A-Za-z][A-Za-z0-9]*-\d+$` distinguishing
    // case 3 (pass-through) from case 4 (drop). Borderline inputs
    // are evaluated under the default pattern.
    let cfg = WorkItemConfig::default();
    let cases: &[(&str, Option<&str>)] = &[
        // case 3: token + hyphen + digits → pass through
        ("PROJ-0042",   Some("PROJ-0042")),
        ("FOO-1",       Some("FOO-1")),
        ("Web2-7",      Some("Web2-7")),  // mixed-case token with digit
        // case 4: drop
        ("42-foo",      None),  // digits then token (digit prefix not a project code)
        ("PROJ-",       None),  // token then empty
        ("-0042",       None),  // empty then digits
        ("PROJ--0042",  None),  // double hyphen (not a single token)
        ("PROJ-0042-x", None),  // trailing token after digits
        ("",            None),
    ];
    for (input, expected) in cases {
        let got = canonicalise_refs(vec![RawRef::from(*input)], &cfg);
        let got_first = got.first().map(String::as_str);
        assert_eq!(got_first, *expected, "input={input}");
    }
}

#[test]
fn aggregate_dedups_across_keys() {
    // work-item: 0042, parent: 0007, related: [0042, 0011]
    // → vec contains 0042, 0007, 0011 (3 entries).
}

#[test]
fn reverse_cross_ref_index_populates_referenced_by() {
    // Seed work-item 0001; seed plan with `work-item: 0001`;
    // assert IndexEntry for 0001 has referencedBy containing the plan path
    // with kind='work-item-ref'.
}

#[test]
fn reverse_cross_ref_excludes_self_reference() {
    // Work-item 0042 has parent: 0042 in its own frontmatter;
    // its referencedBy must NOT contain itself.
}

#[test]
fn reverse_cross_ref_handles_two_way_cycle() {
    // Work-item A.parent=B and Work-item B.parent=A;
    // both appear in each other's referencedBy exactly once
    // (no infinite loop, no duplication).
}

#[test]
fn reverse_cross_ref_to_unknown_id_is_silently_dropped() {
    // Plan references work-item 9999 which doesn't exist;
    // no panic; no entry created; no error logged at warn+ level.
    // Pin behaviour: silent drop (alternative would be to record
    // unresolved refs in IndexEntry.indexErrors — out of scope here).
}

#[test]
fn reverse_cross_ref_dedups_within_same_source_doc() {
    // Plan with work-item: 0001 AND parent: 0001 AND related: [0001]
    // produces one entry in 0001's referencedBy, not three.
}

#[test]
fn referenced_by_merges_plan_reviews_and_work_item_refs() {
    // A work-item that is the target of both a plan-review (per ADR-0017)
    // and a work-item-ref must show both kinds in its referencedBy.
}

#[test]
fn malformed_frontmatter_in_one_file_does_not_break_index_for_others() {
    // Seed two work-items, one with malformed YAML;
    // assert the malformed file is skipped (with a single warn log)
    // and the other indexes correctly.
}
```

#### 3.2b — Implementation

- `frontmatter.rs`: extend `read_ref_keys` (landed in Phase 1
  reading `work-item:` and legacy `ticket:`) to also read `parent:`
  and `related:` keys, aggregating their values into the same
  `Vec<RawRef>`. Body remains pure (no config dependency); shape
  and signature are unchanged from Phase 1.
- `indexer.rs`: add `canonicalise_refs(Vec<RawRef>, &WorkItemConfig)
  -> Vec<String>` that applies the configured pattern's format
  string and prefixes `default_project_code` when the pattern
  requires `{project}` and the input is bare-numeric. Build a
  reverse index keyed on canonical ID. At `IndexEntry` construction,
  merge `reviews_by_target` (kind='plan-review') and the work-item
  reverse-ref map (kind='work-item-ref') into a single
  `referencedBy` vec.
- Wrap `read_ref_keys` + canonicalise calls in a per-file
  try/catch boundary at the indexer call site so a single malformed
  file degrades to "this file contributes no refs" rather than
  crashing the whole indexer.

#### 3.2c — Frontend implementation

- `src/components/.../RelatedArtifacts.tsx` (verify exact path):
  render `workItemRefs` as a list rather than a single field.
  Preserve declared/inferred visual distinction.
- `src/routes/library/...`: surface `parent` work-items prominently
  for story/task work-items; surface child work-items on epics
  (these are inferred from `parent:` in the reverse index).

#### 3.2d — Tests

- New e2e: a fixture with a parent epic and three child stories;
  assert library view of the epic shows all three children; library
  view of a child shows the parent.

### Success Criteria — Phase 3

#### Automated Verification:

- [ ] Server tests pass: `cargo test --all-features`
- [ ] Frontend tests pass: `npm run test -- --run`
- [ ] Playwright e2e passes (including new cross-ref spec)
- [ ] Configuring `visualiser.kanban_columns: [ready, in-progress, review, done]` produces a four-column kanban (asserted by an e2e test)
- [ ] `PATCH /api/docs/.../frontmatter` with an unconfigured status returns 400 with `ApiError::UnknownKanbanStatus` envelope and `accepted_keys` in the body
- [ ] Empty `visualiser.kanban_columns: []` rejected at boot (config unit test)
- [ ] Missing `visualiser.kanban_columns` falls back to the seven defaults (config unit test)
- [ ] Self-referencing work-item does not appear in its own `referencedBy` (indexer test)
- [ ] Two-way cycle handled without duplication or infinite loop (indexer test)
- [ ] Heterogeneous YAML shapes for cross-ref keys (null / scalar-where-array / array-where-scalar / int-vs-string) all degrade to defined behaviour with no panic (frontmatter tests)
- [ ] Plan-review and work-item-ref both surface in `referencedBy` for a doc that's the target of both (composition test against ADR-0017 precedent)

#### Manual Verification:

- [ ] In a project with custom columns config, the kanban shows the configured set
- [ ] A work-item with `parent: 0001` and `related: [0007, 0009]` shows three cross-ref links in the library view's "Related artifacts" aside
- [ ] Work-item 0001 is reverse-linked from work-item with the parent reference
- [ ] Default (unconfigured) project shows the seven template statuses
- [ ] Changing `work.id_pattern` after first launch and reloading the browser without restarting the server shows partially-resolved wiki-links until restart (documents the contract operators must know about)

---

## Phase 4: Documentation, ADRs, and validation

### Overview

Documentation, two new ADRs, CHANGELOG, and a validation pass against
two fresh projects. Plus a small bookkeeping fix-up extracted from
Phase 1.

### Documentation deliverables checklist

A consolidated index of every documentation file this plan touches,
across all phases. Phase 4 success requires every box ticked.

- [ ] `skills/visualisation/visualise/SKILL.md:22` (Phase 1 Step 1.3) — path placeholders updated
- [ ] `meta/decisions/ADR-0024-visualiser-kanban-column-config.md` (Phase 4 Step 4.1) — new ADR
- [ ] `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md` (Phase 4 Step 4.1) — new ADR
- [ ] `skills/config/configure/SKILL.md` (Phase 4 Step 4.2) — Visualiser subsection added
- [ ] `skills/visualisation/visualise/SKILL.md` prose review (Phase 4 Step 4.2)
- [ ] `CHANGELOG.md` Unreleased section (Phase 4 Step 4.3)
- [ ] `meta/validations/2026-05-04-visualiser-work-item-migration-validation.md` (Phase 4 Step 4.4)
- [ ] `meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md` frontmatter (Phase 4 Step 4.5)

A `README.md` was previously listed; the visualise skill does not
have one, and creating one is out of scope for this plan. Visualiser
config schema docs live in `skills/config/configure/SKILL.md` (the
established home, following the v1.20.0 "Work Items" precedent),
linked from the ADR and CHANGELOG.

### Step 4.1 — ADRs (split into two)

The original plan bundled four decisions into a single ADR. This
violates the project's tightly-scoped ADR convention. Split into two
focused ADRs; the terminology rename for the visualiser folds into
ADR-0022 by reference (no new ADR needed), and the wiki-link prefix
is a corollary of ADR-0022 (also no new ADR needed).

**File**: `meta/decisions/ADR-0024-visualiser-kanban-column-config.md`

Single concern: the configurable kanban-column model. Cover:

- Why the column set is configurable per project (vs hardcoded).
- The `visualiser.kanban_columns` config schema (`{ key, label }[]`).
- The seven-status default matching `templates/work-item.md:7`.
- Boot-time validation rules: missing field → defaults; empty list →
  reject; malformed YAML → reject.
- **Boot-time pattern-immutability invariant**: which fields are
  boot-immutable (`scan_regex`, `default_project_code`, `doc_paths.work`)
  vs reload-safe (`kanban_columns`, in principle — though Phase 3
  reads it once per page-load via TanStack Query, not per-request).
  Pattern changes require a server restart; partial reload of pattern
  config produces stale `work_item_by_id` and `referencedBy` indexes.
  This is a deliberate floor that future contributors adding live
  reload must respect.
- API shape: dedicated `GET /api/kanban/config` endpoint returning
  `{ columns: KanbanColumn[] }`. PATCH validation via the
  `ApiError::UnknownKanbanStatus` envelope:
  ```json
  { "error": "unknown_kanban_status", "acceptedKeys": ["ready", "in-progress", "done"] }
  ```
  (camelCase wire form via existing serde rename_all). Frontend
  pattern-matches on `error == "unknown_kanban_status"` to render
  the toast.
- **Other-swimlane write-blocked contract**: any PATCH whose status
  is not in the configured column set returns 400, including PATCH
  *back to* a value currently in Other (e.g. legacy `proposed`).
  Drag-out-of-Other is one-way from the kanban UI; recovery for an
  accidental drag is direct file edit + VCS revert. This matches
  the project's "destructive op safety via VCS" convention rather
  than introducing UI-level undo. Document operator-facing.
- Out of scope: kanban for non-work-item doc types, per-column drag
  permissions, column reordering.

**File**: `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md`

Single concern: the multi-field cross-ref read and the reverse-index
shape. Cover:

- The `work-item:` / `parent:` / `related:` triple and the equal-
  weight aggregation rule (no precedence between keys).
- Canonicalisation rule: bare numerics formatted via the configured
  `format_string`; project-prefixed under project-prefixed patterns.
- Self-reference filter (a doc never appears in its own
  `referencedBy`).
- `IndexEntry.referencedBy: Vec<{ kind, relPath }>` as the wire
  shape, with `kind: 'plan-review' | 'work-item-ref'`.
- Composition with ADR-0017: separate in-memory maps preserve
  lock-ordering invariants; merged at IndexEntry construction.
- Permanent dual-schema tolerance for legacy work-items: missing
  ref keys → empty contribution; legacy `proposed` status falls into
  Other on the kanban; `IndexEntry.workItemId` is `None` for
  filenames that don't match the configured regex.
- Out of scope: editing wiki-link content; bare `[[NNNN]]` form;
  per-file frontmatter migration.

### Step 4.2 — SKILL.md updates

- `skills/visualisation/visualise/SKILL.md`: already updated in
  Phase 1 (path placeholders); prose review for any leftover
  "ticket" wording.
- `skills/config/configure/SKILL.md`: add a "Visualiser" subsection
  modelled on the existing "Work Items" v1.20.0 subsection,
  documenting the `visualiser.kanban_columns` schema (with the
  seven-status default and the boot-time validation rules) and
  pointing at ADR-0024 for rationale.

### Step 4.3 — CHANGELOG

**File**: `CHANGELOG.md` (Unreleased section)

The visualiser is still a prerelease feature; per project convention
the CHANGELOG documents user-visible surface area only, not
internal wire-format changes. Drop entries for `IndexEntry` shape,
`Completeness.hasTicket` rename, `OnlyWorkItemsAreWritable` error
literal, SSE `docType` value, and the `ApiError::UnknownKanbanStatus`
envelope — none of these are user-visible (they are consumed only by
the bundled frontend, which ships in lockstep).

Add entries covering only user-visible additions and the operator
behaviour around pre-migration projects:

- **Visualiser**: pre-migration repos must run `/accelerator:migrate`
  before launching the visualiser. The launcher now exits with a
  clear message pointing at the migrate command rather than
  starting in a broken state.
- **Visualiser**: new `work-item-reviews` doctype appears in the
  sidebar.
- **Visualiser**: pattern-aware work-item IDs (Phase 2). Projects
  configured with `work.id_pattern: "{project}-{number:04d}"` and
  `work.default_project_code` see project-prefixed IDs in the
  kanban, lifecycle, and library views; wiki-links of the form
  `[[WORK-ITEM-PROJ-0042]]` resolve.
- **Visualiser**: configurable kanban columns (Phase 3) via
  `visualiser.kanban_columns`. Defaults to the seven template
  statuses (`draft | ready | in-progress | review | done | blocked
  | abandoned`).
- **Visualiser**: multi-field cross-references (Phase 3). Library
  view's "Related artifacts" aside reads `work-item:`, `parent:`,
  and `related:` frontmatter keys.
- **Visualiser**: wiki-link prefix is now `[[WORK-ITEM-NNNN]]`
  (was `[[TICKET-NNNN]]` in the prerelease). Existing
  `[[TICKET-NNNN]]` references in user-authored docs are migrated
  by a one-shot script run as part of the rename PR (see Phase 1
  Step 1.7); larger backlogs are accepted as dead-link debt
  (recorded in ADR-0025).

Update the existing Unreleased section's references to
`meta/tickets/<file>` and `paths.tickets` to the post-migration
shapes (`meta/work/<file>` and `paths.work`). Specific lines to
rewrite: lines 11, 23, 32-33, 39-40 of the current
`CHANGELOG.md` (verify line numbers at edit time).

### Step 4.4 — Validation runs

**File**:
`meta/validations/2026-05-04-visualiser-work-item-migration-validation.md`

Use the same frontmatter shape as `meta/plans/*.md` (which this
plan itself uses as a model): `date`, `type` (`validation` here),
`skill`, `work-item`, `status`. `templates/validation.md` does not
currently carry frontmatter; this plan does **not** undertake to
update the template (out of scope). Each scenario produces a
script artefact (Playwright spec or shell script) plus its
captured output, not free-form prose.

#### Scenario A: default ID pattern, default columns

Convert to a Playwright spec
(`frontend/e2e/default-pattern.spec.ts`) plus a setup
shell script that seeds the temp project. Spec asserts:

1. Sidebar shows 11 doc types.
2. Five seeded work-items (one per non-trivial status) render in the
   correct kanban column.
3. Drag from `ready` to `in-progress` persists; refresh confirms.
4. PATCH against each of the seven default statuses returns 200.
5. Wiki-link `[[WORK-ITEM-0001]]` resolves to the right file;
   `[[ADR-0023]]` still resolves.
6. `meta/reviews/work/` (empty/absent) shows "no items".

The validation document records the spec name, the seed script, the
test run output, and pass/fail.

#### Scenario B: project ID pattern + custom columns

Playwright spec
(`frontend/e2e/project-pattern-custom-columns.spec.ts`)
configured against a temp project with
`work.id_pattern: "{project}-{number:04d}"`,
`work.default_project_code: "PROJ"`,
`visualiser.kanban_columns: [ready, in-progress, review, done]`.
Spec asserts:

1. Three seeded work-items (`PROJ-0001`, `PROJ-0002`, `PROJ-0007`)
   render in the kanban.
2. Kanban shows four columns (no Other lane needed when all seeded
   statuses are configured).
3. Wiki-links `[[WORK-ITEM-PROJ-0001]]` and
   `[[WORK-ITEM-PROJ-0007]]` resolve; `[[WORK-ITEM-PROJ-9999]]`
   shows the unresolved-link state.
4. PATCH with status outside the configured four returns 400 with
   `accepted_keys` listing the four.
5. PATCH with status `In Progress` (label-cased) returns 400 (case
   sensitivity contract).

#### Regression scenario

Playwright spec (`frontend/e2e/legacy-schema.spec.ts`)
running against a fixture copy of this workspace's 30 legacy
`meta/work/` files. Asserts:

1. All 30 files render in the kanban (legacy `proposed` → Other
   swimlane).
2. Drag a card from Other (proposed) to a configured column (e.g.
   `ready`) succeeds (PATCH 200, Phase 1 contract).
3. PATCH from a configured column to `proposed` returns 400 (Phase 3
   boundary contract under the configured-cols scenario).
4. Library view of any file with `type: adr-creation-task` and no
   `work_item_id:` renders without errors.

The validation document records each spec's run output verbatim.

### Step 4.5 — Phase 12 plan-frontmatter fix-up (extracted from Phase 1)

Single trivial edit, kept separate so the work-item-terminology PRs
remain focused on their concern:

- `meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md`:
  update frontmatter `status: draft` → `status: complete`. Land as a
  standalone commit (no PR needed if the project's contribution
  workflow allows direct trivial commits; otherwise as a one-line
  PR). Originally listed in Phase 1 Step 1.7.

### Success Criteria — Phase 4

#### Automated Verification:

- [ ] ADR file lints clean: `make lint` (if applicable)
- [ ] CHANGELOG entry is present and well-formatted
- [ ] All Phases 1–3 automated checks remain green

#### Manual Verification:

- [ ] Both validation scenarios above complete without errors
- [ ] Regression scenario confirms legacy-schema work-items render
- [ ] README/SKILL.md prose reads correctly and matches behaviour
- [ ] ADR is reviewed and merged

---

## Testing Strategy

### Unit tests

- `docs.rs`: enum membership, wire-format round-trip, `in_kanban` /
  `in_lifecycle` invariants for both renamed and net-new variants.
- `slug.rs`: pattern-aware regex (driven by the actual
  `work-item-pattern.sh --compile-scan` output, not hand-rolled)
  applied to default and project-prefix fixtures, including
  lowercase/digit-bearing project codes; non-md and malformed inputs
  return `None`; invalid pattern surfaces a clear error.
- `frontmatter.rs::read_ref_keys` (config-free pure parser): each
  key in isolation, all three combined, scalar-where-array, array-
  where-scalar, null, empty string, integer-vs-string equivalence,
  missing keys return empty vec.
- `indexer.rs::canonicalise_refs` (config-aware): bare-numeric
  padding under default pattern; project-prefix prefix application
  under project pattern; pre-prefixed pass-through; dedup after
  canonicalisation.
- `api/docs.rs` status validation: accepts the seven defaults
  (Phase 1) and the configured set (Phase 3); rejects unknown
  values with `ApiError::UnknownKanbanStatus`; case-sensitive
  exact-match (no trim).
- `indexer.rs`: pattern-aware ID extraction; reverse cross-ref
  index populates `referencedBy`; self-reference filter; two-way
  cycle handling; unknown-target silent drop; mixed bare-numeric +
  project-prefixed shape precedence; per-file try/catch boundary
  isolates malformed files; legacy-schema work-items index without
  `workItemId`.
- `clusters.rs`: `derive_completeness` sets `has_work_item: true`
  when any cluster entry has `DocTypeKey::WorkItems`, parallel to
  the existing `has_plan` / `has_decision` / `has_research` /
  `has_pr_review` flags. Wire-form `hasWorkItem` via existing
  camelCase serde.
- `config.rs`: `WorkItemConfig::from_raw` rejects invalid scan
  regexes; `kanban_columns` boot-time validation rules (missing →
  defaults; empty → reject; malformed → reject).
- `server.rs`: startup test — config without `doc_paths.work` exits
  non-zero with a precise message.
- Frontend: `wiki-links.ts` regex builder matches all documented
  forms (default, single-segment project, multi-segment project)
  via server-supplied inner pattern; `KanbanBoard` renders
  configured column count from `GET /api/kanban/config`;
  `WorkItemCard` links to `/library/work-items/<slug>`;
  `test-fixtures.ts` defaults satisfy the new `IndexEntry` shape.

### Integration tests (Rust `server/tests/`)

- `api_smoke.rs`: GET `/api/types` returns 11 types including
  `work-items` and `work-item-reviews`; GET
  `/api/docs/work-item-reviews` returns `200 []` when the directory
  does not exist on disk.
- `api_docs_patch.rs`: PATCH a work-item's status — all seven
  defaults accepted (Phase 1); unknown returns 400 with
  `accepted_keys`; legacy `proposed` round-trips out of Other to a
  configured column (Phase 1); PATCH-into-Other rejected (Phase 3
  boundary).
- `api_docs.rs`: list work-items under default and project patterns,
  including a fixture where some files match the configured regex
  and others don't (mixed-shape precedence).
- New `api_work_item_pattern.rs`: project-pattern fixtures end-to-end
  including a file in the work directory that doesn't match the
  pattern (assert defined behaviour).
- New `api_cross_refs.rs`: multi-field cross-ref aggregation; reverse
  index populates `referencedBy`; self-reference excluded; two-way
  cycle handled; plan-review and work-item-ref both surface in
  `referencedBy` for a doc that's the target of both.
- New `api_legacy_schema.rs`: a fixture under
  `tests/fixtures/meta/work-legacy/` mirroring this workspace's
  30-file shape (`type: adr-creation-task`, `status: proposed`, no
  `work_item_id:`, no `parent:`/`related:`); assert the file
  indexes, slugs, renders, and falls into Other; PATCH from Other to
  `ready` succeeds.

### Shell tests

- `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`:
  default config produces correct `doc_paths`; pre-migration config
  (`paths.tickets` set, `paths.work` absent) → launcher exits with
  migrate-pointer message; pattern-aware config flows through to
  `work_item.scan_regex` / `work_item.id_pattern`.
- Pattern compiler contract test exercising
  `work-item-pattern.sh --compile-scan` from the visualiser side
  (default, project-prefixed, invalid).

### E2E tests (Playwright, `frontend/e2e/`)

- `kanban.spec.ts`: drag-drop against renamed fixtures (Phase 1).
- `kanban-conflict.spec.ts`: ETag conflict path (no behaviour change).
- `wiki-links.spec.ts`: project-pattern wiki-link resolution via the
  server-supplied inner regex (Phase 2 addition).
- New `default-pattern.spec.ts` (Phase 4 Scenario A).
- New `project-pattern-custom-columns.spec.ts` (Phase 4
  Scenario B).
- New `legacy-schema.spec.ts` (Phase 4 Regression).
- New spec for parent/related cross-refs in library view.

### Manual testing

- Phase 4 manual verification checklists per phase. All Phase 4
  scenarios are implemented as Playwright specs (above) so manual
  testing is reduced to spinning up the visualiser and confirming
  the specs reflect observed reality.

## Migration Notes

This plan does **not** rewrite per-file frontmatter in existing
`meta/work/` directories. Live work-items keep their legacy schema
and the visualiser is required to render them gracefully — this is a
**permanent** dual-schema tolerance contract, not a transitional
concern (captured as a first-class decision in ADR-0025).

Per-file behaviour for legacy/missing frontmatter:

- Legacy `status: proposed` (and other values outside the configured
  set) → "Other" swimlane on the kanban (read path). The card is
  pinned in Other on the write path under Phase 3 (PATCH back to
  `proposed` returns 400 once columns are configured).
- Missing `work_item_id:` frontmatter does not affect indexing —
  `IndexEntry.workItemId` is **filename-derived via the configured
  scan regex**, not frontmatter-derived. Three states:
  - regex matches the filename stem → `Some("<full-string-id>")`;
  - regex doesn't match → `None`, file excluded from
    `work_item_by_id` but still indexed and renderable;
  - doc isn't a work-item type → `None`.
- Missing `work-item:` / `parent:` / `related:` (or any combination
  of YAML shapes — null, empty string, scalar where array expected,
  array where scalar expected) → empty contribution from that key.
  No cross-ref display, no error, never a panic. The indexer wraps
  these reads in a per-file boundary so a single malformed file
  cannot crash the whole index.
- Migration triggers under specific scenarios (the failure modes
  this plan was created to fix, transposed): launching against a
  config that defines `paths.tickets` but not `paths.work` →
  launcher exits non-zero with a `/accelerator:migrate` pointer
  (Phase 1 Step 1.2). Server boot against a config without
  `doc_paths.work` → exits with a precise message naming the
  missing key.
- Pattern changes after first launch require a visualiser restart
  (the scan regex is compiled once at boot; in-memory state is
  keyed by the old pattern). Document this in ADR-0024 / SKILL.md;
  the operator-facing failure mode is partially-resolved wiki-links
  until restart.

A separate work-items-frontmatter migration is not planned. If
future work decides to rewrite per-file frontmatter, that plan owns
its own scope; this plan's tolerance contract is permanent.

## Performance Considerations

- The reverse cross-ref index is built once at scan time alongside
  the existing forward index; no per-request work added.
- The pattern-aware regex is compiled once at boot from the launcher-
  supplied string. The regex is anchored and bounded; repeated
  filename matches add no measurable overhead on a 30-doc index.
- Configurable columns are read once per page load via TanStack
  Query and cached. No per-request server overhead.

## References

- Research: `meta/research/2026-05-03-update-visualiser-for-work-item-terminology.md`
- Related research:
  - `meta/research/2026-04-17-meta-visualiser-implementation-context.md` (visualiser design)
  - `meta/research/2026-04-25-rename-tickets-to-work-items.md` (full rename surface)
  - `meta/research/2026-04-26-remaining-ticket-references-post-migration.md` (post-migration cleanup; stragglers since cleaned)
  - `meta/research/2026-04-28-configurable-work-item-id-pattern.md` (pattern compiler)
- Decisions:
  - `meta/decisions/ADR-0022-work-item-terminology.md` (terminology rename — visualiser conformance is downstream)
  - `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
  - `meta/decisions/ADR-0017-configuration-extension-points.md` (plan-review reverse-link precedent for `referencedBy` composition)
  - `meta/decisions/ADR-0024-visualiser-kanban-column-config.md` (new — Phase 4)
  - `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md` (new — Phase 4)
- Plugin code (already migrated):
  - `scripts/config-read-path.sh:7-19` — recognised path keys
  - `skills/work/scripts/work-item-pattern.sh` — pattern compiler CLI
  - `skills/work/scripts/work-item-common.sh:212-449` — pattern helpers
  - `templates/work-item.md`, `templates/plan.md:4`
- Visualiser code (this plan modifies):
  - `skills/visualisation/visualise/scripts/write-visualiser-config.sh:39,89,105`
  - `skills/visualisation/visualise/SKILL.md:22`
  - `skills/visualisation/visualise/server/src/{docs,slug,clusters,patcher,frontmatter,indexer,server,sse_hub,file_driver}.rs`
  - `skills/visualisation/visualise/server/src/api/{mod,docs}.rs`
  - `skills/visualisation/visualise/server/tests/fixtures/meta/tickets/`
  - `skills/visualisation/visualise/frontend/src/api/{types,fetch,ticket,use-move-ticket,wiki-links,use-wiki-link-resolver,use-doc-events,test-fixtures}.ts`
  - `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/wiki-link-plugin.ts`
  - `skills/visualisation/visualise/frontend/src/routes/kanban/{KanbanBoard,TicketCard,KanbanColumn,announcements}.tsx`
  - `skills/visualisation/visualise/frontend/e2e/kanban.spec.ts`
- Phase 12 plan to mark complete:
  - `meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md`
