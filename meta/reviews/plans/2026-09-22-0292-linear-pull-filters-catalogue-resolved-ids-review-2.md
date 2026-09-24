---
type: "plan-review"
id: "2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-review-2"
title: "Plan Review: Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan"
date: "2026-09-23T22:58:54+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids"
relates_to: ["plan-review:2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-review-1"]
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "safety", "performance"]
review_number: 2
review_pass: 4
tags: ["linear", "pull-filters", "catalogue"]
last_updated: "2026-09-24T10:22:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan

**Verdict:** REVISE

The rewrite keeps the structural wins that review 1 converged on: a pure
pre-flight and completion core, `compose` over a drained `LowerableSearch`,
`NonEmpty`-backed refusals, and `TrackerError::Unconfigured` routed to
`DiscoveryUnconfigured`. It adds a converged catalogue with one strict writer
behind it. The new material (the converged document, the live fetch and the
self-heal) carries one internal contradiction that blocks implementation.
Coverage means "all four sections", but the live fetch retrieves only the
sections the configured filters need, so completion refuses every team it
just fetched. Finalisation also has no data path, and no specified seam, for
recording a newly imported team. The writer's merge and legacy rules are
stated in ways that contradict each other or lose data.

### Cross-Cutting Themes

- **Coverage vs needed-sections fetch** (flagged by: correctness,
  test-coverage) — `covers` requires a complete entry, while `search` fetches
  only the needed sections and then refuses an entry that is still incomplete
  as `TeamUnfetched`. The Phase 2 and Phase 3 red tests demand incompatible
  behaviour.
- **Finalisation cannot catalogue an imported team** (flagged by:
  architecture, correctness, test-coverage) — imported teams are known only
  by their key. `fetch_team_entries` takes ids, and dropping
  `enumerate_visible_entities` removes the only key→id step. There is also no
  production provider of `TeamEntryFetch` behind `&dyn RemoteTracker`.
- **The writer is not lossless** (flagged by: safety, compatibility,
  correctness, code-quality):
  - The typed `TeamEntry` round-trip drops unknown per-entry and per-record
    fields.
  - Malformed entries are dropped rather than refused.
  - "Replace whole entries" contradicts "sections not carried are kept".
  - `CatalogueDocument` keeps its known keys as `Value` beside the typed
    model.
- **The `teams` reconcile list cannot be rebuilt** (flagged by: safety,
  compatibility) — the "delete `catalogue.json` and re-run init" remedy, or a
  pre-0292 init, erases synced teams permanently. Tracked items are never
  re-imported, so those teams never return.
- **Base team and legacy derivation** (flagged by: correctness, safety,
  code-quality):
  - It is unspecified where `baseTeam` comes from on a sync write.
  - It is unspecified which of two same-id base entries wins.
  - The "base entry, else legacy" logic is repeated in five places.
- **`Unconfigured` classified as `Retryable`** (flagged by: code-quality,
  architecture) — the doc says "a retry will not clear it", yet `apply.rs`
  folds it into `FailureClass::Retryable`.
- **Cost and shape of the section fetch** (flagged by: performance,
  architecture, compatibility):
  - Non-synced scoped teams re-fetch whole sections on every pull.
  - The spike has no acceptance budget.
  - Nested connections risk Linear's 10,000-point complexity cap.
  - A batched ceiling is shared across all the teams in a batch.
- **Finalisation of an unreachable team** (flagged by: safety, correctness)
  — whether one inaccessible catalogued team blocks healing of every other
  team is unspecified.

### Tradeoff Analysis

- **Compatibility vs precision (assignee scope)**:
  - Compatibility wants a workspace-user fallback, so that assignees who
    aren't team members keep working.
  - Correctness and the work item want refusals that are precise per team.
  - Recommendation: keep membership scoping, and call it out as a break in
    the CHANGELOG `Migrations` entry, with a former-member test.
- **Performance vs uniformity (uncovered teams)**:
  - Performance wants value-targeted lookups for teams that are never
    written.
  - Architecture values one resolution path through `with_team_entries`.
  - Recommendation: set a request budget in the spike, and adopt
    value-targeted lookups only if the fallback or the member and project
    costs break it.
- **Typed model vs losslessness**:
  - Code-quality wants `teams` typed as `Vec<TeamEntry>`.
  - Safety and compatibility want unknown fields preserved.
  - Recommendation: do both. Use typed entries with a
    `#[serde(flatten)] extra` at entry and record level, and refuse when an
    entry cannot be parsed.

### Findings

#### Critical

- 🔴 **Correctness + Test Coverage**: Partial-section live fetch leaves teams
  uncovered, so completion refuses them as `TeamUnfetched`
  **Location**: Phase 3 §2 `complete_for_teams`, §3 `search` steps 1–4;
  Phase 2 §1 Coverage
  `covers` is true only for all four sections. `search` fetches only the
  needed sections, then refuses a team that is still uncovered after folding.
  Every filtered pull against today's catalogues would therefore refuse,
  which is the regression Phase 3 was merged to prevent.

#### Major

- 🟡 **Architecture + Correctness + Test Coverage**: Finalisation knows
  imported teams by key, but the fetch takes ids
  **Location**: Phase 3 §5 Finalisation
  Ids exist only for held entries, and entries are held only when a filter
  triggered a fetch. An unfiltered broadened pull, the 0229 growth case,
  cannot catalogue a new team. The finalisation tests all start from a
  pre-filled buffer, so none catches this.
- 🟡 **Architecture + Code Quality + Test Coverage**: Finalisation's seams
  are unspecified and partly untestable
  **Location**: Phase 3 §5 The backfill seam; `work-cli/src/sync.rs` tests
  - `TeamEntryFetch` is never defined, and has no production provider behind
    `&dyn RemoteTracker`.
  - The backfill `Arc` is threaded through two independent `Option`s.
  - `run_sync_hands_its_backfill_to_the_registry` cannot be written against
    the real `TrackerRegistry`.
  - The warning test cannot observe the exit code.
- 🟡 **Correctness + Test Coverage**: The writer's merge rules contradict
  each other
  **Location**: Phase 1 §3 `record_team_entries`; Desired End State
  "Writers"
  One rule says "replaces whole team entries"; another says "sections an
  entry does not carry are kept". Nothing defines how two same-id entries in
  one update combine, and finalisation depends on a section-level merge.
- 🟡 **Safety + Compatibility + Correctness + Code Quality**: The typed
  round-trip drops data the writer does not own
  **Location**: Phase 1 §1 `CatalogueDocument`, §3 writer
  Only top-level `extra` is preserved. Unknown entry and record fields, and
  malformed or blank-id entries, are silently dropped on rewrite.
  `CatalogueDocument` also stores its known keys as untyped `Value`.
- 🟡 **Correctness + Safety**: `baseTeam` and legacy derivation are undefined
  after the first write
  **Location**: Phase 1 §1 "Reading a legacy file"; Phase 3 §5; Migration
  Notes
  Nothing says where sync finalisation gets `baseTeam` from. If a legacy
  base and a `teams` entry share an id, nothing says which wins, so healing
  may never converge. A sync must never re-point an existing `baseTeam`.
- 🟡 **Safety + Compatibility**: Deleting the catalogue, or a pre-0292 init,
  permanently loses synced teams
  **Location**: Phase 1 §3 `for_cache` message; Phase 3 §6
  `E_SEARCH_CATALOGUE_DAMAGED`; Migration Notes
  Teams return to `teams` only when items are imported *in a run*. Items that
  are already tracked are never re-imported, so the lost teams stay
  indeterminate forever.
- 🟡 **Safety + Correctness**: Finalisation of a team that can no longer be
  fetched is unspecified
  **Location**: Phase 3 §5 Finalisation
  If this is all-or-nothing, one inaccessible team blocks healing everywhere
  and every apply sync warns.
- 🟡 **Correctness**: The zero-ids refusal is per family, but the work item
  requires a refusal per value, and forbids over-matching
  **Location**: Phase 2 §1 per-family rules; Phase 3 §2
  `label: [bug, typo]`, where `typo` is unknown, can silently drop `typo`. An
  archived and an active "Done" state, or labels with the same name, are
  unioned.
- 🟡 **Compatibility**: The `NoTeam`→77 remap cannot be expressed in the
  context-free `for_client`
  **Location**: Phase 3 §6, §3 `search_detailed`
  `NoTeam` is raised at client construction for every flow. Remapping it
  moves create, update and transition from 105 to 77.
- 🟡 **Compatibility**: Resolving assignees only among scoped-team members
  can break working configs
  **Location**: Phase 2 §1 Assignee; Migration Notes
  Today any workspace user matches. After the change, a non-member now causes
  an exit-74 refusal of the whole sync, and the CHANGELOG `Migrations` entry
  does not say so.
- 🟡 **Code Quality + Architecture**: `Unconfigured` is classed as
  `FailureClass::Retryable`, although its doc says a retry will not clear it
  **Location**: Phase 3 §1; Existing tests to update (`apply.rs`)
- 🟡 **Code Quality**: `ResolvedSearch` holds unresolved names, and three
  near-identical `Search` structs form a data clump
  **Location**: Implementation Approach; Phase 3 §2
- 🟡 **Code Quality**: Legacy catalogue reading is scattered across five
  sites, which makes the removal in 0294 error-prone
  **Location**: Phase 1 §1, §4; Phase 2 §2
- 🟡 **Performance + Architecture**: Non-synced scoped teams re-fetch whole
  sections on every pull, under a batch-shared ceiling
  **Location**: Phase 3 Overview; Performance Considerations; Phase 1 §2
  The cost scales with workspace size, not with the filter. Refusal on
  truncation depends on how many teams are batched together.
- 🟡 **Performance**: The spike has no acceptance budget, and the per-team
  fallback is unbounded
  **Location**: Prerequisite: team-section fetch spike
- 🟡 **Performance + Compatibility**: Nested-connection queries risk Linear's
  complexity cap and the 8 MiB response bound
  **Location**: Phase 1 §2 `paginate`
  `paginate` carries no page size, and nested `first:` sizing is unspecified.
- 🟡 **Test Coverage**: A single-key `Route::Sequence` couples the
  multi-query tests to request order and can hide extra requests
  **Location**: Phase 1, Phase 3 discovery, port, init and real-client tests
- 🟡 **Test Coverage**: `grow_linear_catalogue` has no tests today, so its
  Phase 1 rewire has no red test and there is nothing to "move"
  **Location**: Phase 1 §4; Phase 3 Existing tests
- 🟡 **Test Coverage**: No tests for per-team attribution, nested-connection
  paging, or the per-team fallback
  **Location**: Spike; Phase 1 discovery tests

#### Minor

- 🔵 **Correctness**: Workspace labels are outside coverage and cannot be
  folded in or held
  **Location**: Phase 3 live-fetch table; Phase 2 `with_team_entries`
- 🔵 **Correctness**: Pre-flight resolves against `baseTeam`, which may not
  be the team `linear.team_key` names
  **Location**: Phase 3 §2 `preflight`
- 🔵 **Safety + Test Coverage**: Pin that `fetch_all` pages every key
  `in_scope` knows, and pages the base team exactly once now that it is in
  `teams`
  **Location**: Phase 1 §1; Phase 2 §3
- 🔵 **Safety**: Lenient `append_line` can truncate an unreadable
  `.gitignore`
  **Location**: Phase 1 §3 `Filesystem::read`
- 🔵 **Architecture**: `ResolverSet` keeps two sources of team identity
  (`TeamResolver` and `TeamEntries`)
  **Location**: Phase 2 §1
- 🔵 **Architecture + Code Quality**: Linear-specific finalisation spreads
  further into the generic sync composition and into an already large
  `sync.rs`
  **Location**: Phase 3 §5
- 🔵 **Code Quality**: `NameResolver` returns a multi-id `Resolution` that
  its contract forbids
  **Location**: Phase 2 §1, §3
- 🔵 **Code Quality**: The partition, fetch, hold, fold, complete and compose
  sequence is duplicated across `search` and `search_detailed`
  **Location**: Phase 3 §3
- 🔵 **Code Quality + Architecture**: Pipeline stage is encoded in wire key
  strings (`label.name`)
  **Location**: Phase 3 §2
- 🔵 **Code Quality**: `Unresolved` uses a `&'static str` section and an
  `Option` tier that applies to assignees only
  **Location**: Phase 2 §1
- 🔵 **Code Quality**: Write-path arms and `E_PUSH_UNCONFIGURED` for a
  variant that write paths never produce
  **Location**: Phase 3 §1; Existing tests
- 🔵 **Performance**: The 200-page ceiling has no deadline, allowing ~800+
  requests before a refusal
  **Location**: Phase 1 §2
- 🔵 **Performance**: Per-team duplication of members and projects inflates
  the catalogue's size and parse cost
  **Location**: Phase 1 §1
- 🔵 **Compatibility**: Pre-0292 grow writes break id and key ordering, which
  undermines the "merges cleanly" claim
  **Location**: Migration Notes
- 🔵 **Compatibility**: The standalone `search` behaviour and exit-code
  changes are missing from the CHANGELOG
  **Location**: Phase 3 §3, §7
- 🔵 **Compatibility**: Removing the projections in 0294 has no version gate
  **Location**: What We're NOT Doing; Migration Notes
- 🔵 **Test Coverage**: The exit-74 refusal through the sync binary is not
  tested until Phase 4
  **Location**: Phase 3 tests
- 🔵 **Test Coverage**: Archived-entity boundaries in filter resolution are
  only partly covered
  **Location**: Phase 2 catalogue tests
- 🔵 **Test Coverage**: Tests deleted with `discover_team`, and the
  legacy-path `flow_transition` coverage, have no stated replacement
  **Location**: Phase 1; Phase 3 `seed_catalogue`

#### Suggestions

- 🔵 **Safety**: Healing also runs on `--push-only` and targeted apply runs.
  Limit it, or document it.
- 🔵 **Correctness**: Active-wins departs from the work item's criterion for
  ambiguity within a tier. Update the work item, and state which record wins
  when deduplicating by id.
- 🔵 **Performance**: Print one stderr line counting the teams and sections
  fetched live on each pull.
- 🔵 **Test Coverage**: Restate the `OnceCell` laziness test as behaviour,
  and make the catalogue ceiling injectable.
- 🔵 **Test Coverage**: Trace two acceptance criteria to named tests: a
  full-name match on one user beats a display-name match on another, and a
  value served only on a second page resolves.
- 🔵 **Architecture**: Add an engine test that `resolve_entities` passes
  filter pairs through byte-for-byte.

### Strengths

- ✅ `compose` accepts only a drained `LowerableSearch`, so an unresolved or
  dropped family is impossible by construction. The `_ => {}` silent drop is
  gone.
- ✅ `NonEmpty<T>` makes "zero ids" a construction-time invariant.
  `TeamEntries` answers both `for_family` and `covers`, so doubles cannot
  disagree.
- ✅ `preflight` and `complete_for_teams` are pure, and pre-flight on a
  base-only, complete scope refuses with zero requests.
- ✅ Refusals surface in `prepare_run` before any write, and
  `Unconfigured` → `DiscoveryUnconfigured` fixes today's "transient, retry"
  mislabel.
- ✅ There is one locked, atomic writer. It refuses damaged input instead of
  falling back to `{}`. Init is all-or-nothing. Preview is pinned
  byte-identical.
- ✅ The live fetch is held only after every pass succeeds. It is limited to
  the needed sections, and skipped when nothing needs it.
- ✅ Pre-0292 binaries keep reading the file. The `teams` entries still carry
  `key`/`id`, `fetch_all` deduplicates the base id, and the projections keep
  the state resolver and the `auth.rs` fallback alive.
- ✅ The port change is deliberate and fully enumerated: the snapshot, the
  frozen oracle, and every exhaustive match site.
- ✅ Every phase lists its red tests, plus the existing tests each change
  breaks.

### Recommended Changes

1. **Make coverage family-aware** (addresses: partial-section fetch
   refusal; workspace labels outside coverage)
   - Replace `covers(team)` with `covers(team, &SectionSet)`, and derive the
     `SectionSet` from the configured families. Treat workspace labels as
     part of the `Label` requirement.
   - Use that one predicate for partitioning, for the post-fold check, and
     for pre-flight's "complete" test.
   - Keep full completeness only for finalisation's heal set.
   - Add `a_team_folded_with_only_the_needed_sections_completes` to
     `completion.rs`.
   - Give the fold and the backfill a slot for workspace labels.
2. **Give finalisation team identity and a real fetch seam** (addresses:
   key vs id; unspecified seams; infeasible wiring test)
   - Either have `search` always hold the scoped `TeamRef`s, or keep a key→id
     step for imported keys that have no held entry.
   - Define `TeamEntryFetch` and its production provider, built by
     `ConfiguredTrackers` from the same credential context.
   - Pair the backfill and the fetch in one non-optional object passed to
     `run_sync`.
   - Test the wiring on `ConfiguredTrackers`.
   - Add `finalising_catalogues_an_imported_team_that_was_never_fetched`.
   - Inject a diagnostic writer for the warning test.
3. **Define one merge rule and make the writer lossless** (addresses: merge
   contradiction; typed round-trip loss; `Value` fields)
   - Merge section by section: a carried section replaces the stored one,
     and an uncarried one is kept.
   - Merge same-id entries in one update before applying them.
   - Give `TeamEntry` and the record types a `#[serde(flatten)] extra`.
   - Refuse (`Unparseable`) any entry that cannot be parsed.
   - Type `teams` and `labels`.
   - Add a section-kept test and a byte-exact golden test.
4. **Pin base-team and legacy semantics in one place** (addresses:
   `baseTeam` undefined; scattered legacy reads; pre-flight base mismatch)
   - Put `Catalogue::base_entry()` behind one parse function, and use it in
     `auth.rs`, the write-back and `TeamStates`.
   - A write sets `baseTeam` from `/team/id` only when it is absent, and
     never re-points it.
   - Prefer a `teams` entry with a matching id over the legacy derivation.
   - Pre-flight resolves against the team resolved from `team_key`.
   - Add a test that heals a legacy file and then reads it back as covered.
5. **Make the reconcile list recoverable** (addresses: delete remedy;
   pre-0292 init erasure)
   - Make the first remedy "restore the last good version from VCS".
   - Have finalisation derive synced teams from the key prefixes of every
     tracked item, not only this run's imports.
   - Correct the Migration Notes accordingly.
6. **Specify finalisation for teams that were not returned** (addresses:
   unreachable team)
   - Record the entries that were returned.
   - Name the missing teams in the warning.
   - Never write a partial entry.
7. **Refuse per value, and decide the multi-match rule** (addresses:
   zero-ids per family; archived boundaries)
   - Refuse every `NotFound` value.
   - Decide whether archived states and labels take part, and whether a
     second match within one team is ambiguous. Pin both in tests.
8. **Fix the search exit-code seam and the documentation of breaks**
   (addresses: `NoTeam` remap; assignee scope; search CHANGELOG; 0294 gate)
   - Add a distinct search-side refusal that maps to 77. Leave
     `NoTeam` → 105, and pin that create still exits 105.
   - Add CHANGELOG `Migrations` and `Changed` lines for assignee membership,
     standalone search scoping, and exit codes 89/77.
   - Tie 0294 to a minimum plugin version.
9. **Give the spike exit criteria** (addresses: no budget; nested
   complexity; uncovered-team refetch; batch-shared ceiling)
   - Set maximum requests per section for N = 50 and N = 200 teams, and a
     maximum complexity per page.
   - Pass an explicit `first:` per section, including nested ones, and assert
     it in tests.
   - Decide whether the ceiling applies per team or per batch.
   - Name the fallback plan, such as value-targeted lookups.
   - Add tests for attribution and nested paging.
10. **Harden the test harness and fill red-list gaps** (addresses:
    `Route::Sequence`; `grow_linear_catalogue` untested; exit-74 binary;
    deleted tests)
    - Add a GraphQL-aware route to `http-test-support`, and assert exact
      request counts per operation.
    - Add a Phase 1 red test for the grow rewire.
    - Add a Phase 3 binary test for an unknown label that exits 74.
    - List the retired `discover_team` tests, and keep
      `seed_legacy_catalogue`.
11. **Tidy the domain names and the error taxonomy** (addresses:
    `Unconfigured` classed as `Retryable`; `ResolvedSearch` naming;
    `NameResolver`; `Unresolved` strings; dead write arms)
    - Give `FailureClass` a name that fits the new meaning, or add a variant.
    - Rename the stages `ConfiguredSearch` → `ValidatedSearch` →
      `LowerableSearch`.
    - Give `NameResolver` a single-id result type.
    - Add a `CatalogueSection` enum.
    - Collapse the write-path arms into one helper.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. It has a pure core (preflight,
complete_for_teams, compose over a drained LowerableSearch), one strict
writer, and a single resolver abstraction over team entries, and it touches
the pinned port with only one new variant. The weak point is the apply-mode
self-heal. The plan does not say how `work-cli`, which holds only a
`&dyn RemoteTracker`, reaches a Linear team-entry fetch. It also removes the
key→id enumeration that finalisation needs for teams it imported but never
fetched.

**Strengths**:
- The filtering, pre-flight and completion core is pure, and compose accepts
  only a LowerableSearch.
- TeamEntries answers both for_family and covers, so the two cannot drift.
- There is one lossless, deterministic writer under the shared lock.
- Reads are forgiving and writes are strict, a split that matches the risk
  on each side.
- The buffered backfill and apply-only finalisation keep `search` free of
  filesystem side effects.
- The port change is minimal and explicitly scoped.
- The phase split is justified.

**Findings**:
- 🟡 major / high — **Finalisation has no specified production route to the
  team-entry fetch** (Phase 3 §5).
  - `TeamEntryFetch` is never defined.
  - Finalisation receives only `&dyn RemoteTracker`, and
    `ConfiguredTrackers::resolve` returns `Box<dyn RemoteTracker>`.
  - Fix: define the trait and its provider, and add a registry-level wiring
    test.
- 🟡 major / medium — **Finalisation knows imported teams only by key, but
  the fetch is keyed by id** (Phase 3 §5).
  - An unfiltered broadened pull holds nothing, so a new team has no id.
  - Fix: carry `TeamRef`s through the run, or keep key→id enumeration.
- 🔵 minor / medium — **TrackerError::Unconfigured adds a second
  classification axis** (Phase 3 §1).
  - `ScopeError` was deliberately kept out of `TrackerError`, and `apply.rs`
    classes the new variant as `Retryable`.
  - Fix: rewrite the doc as two axes, or record why a search-specific error
    type was rejected.
- 🔵 minor / medium — **Linear-specific catalogue maintenance spreads into
  the generic sync composition** (Phase 3 §5).
  - Fix: add a `LinearCatalogueMaintenance::finalise` in `linear-client`,
    held by `work-cli` as an opaque `RunFinaliser`.
- 🔵 minor / medium — **ResolverSet keeps two sources of team identity**
  (Phase 2 §1).
  - Fix: derive key resolution from `TeamEntries`.
- 🔵 minor / medium — **Batching many teams shares the page ceiling across
  teams** (Spike; Phase 1 §2).
  - A growing workspace can start refusing.
  - Fix: decide whether the ceiling applies per team or per batch, and chunk
    the `in` list.
- 🔵 suggestion / medium — **Validated state travels as a string-key suffix
  through the port's filter bag** (Phase 3 §2).
  - Fix: record the tradeoff, and add an engine test that the pairs pass
    through byte-for-byte.

### Code Quality

**Summary**: The domain is modelled carefully, with clear red-then-green
ordering throughout. The main risks are:
- misleading or overlapping type names;
- untyped `Value` sitting beside the typed entries;
- legacy reading scattered across modules;
- a hidden side effect threaded through the read port;
- a new error variant classified against its own doc.

**Strengths**:
- compose cannot fail, because it accepts only a LowerableSearch.
- `NonEmpty<T>` makes "zero ids" a construction-time invariant.
- A single `TeamEntries` answers both ids and coverage.
- complete_for_teams is pure.
- FilterKey is the sole owner of the filter strings.
- `Filesystem::read` distinguishes not-found from unreadable.
- A single writer replaces two lossy ones.

**Findings**:
- 🟡 major / high — **Unconfigured is classified as FailureClass::Retryable
  despite being documented as not clearable by retry** (Phase 3 §1; apply.rs
  arm).
  - A future maintainer could add auto-retry that loops forever.
  - Fix: rename the class after its meaning, or add a variant, and route
    through one helper.
- 🟡 major / high — **ResolvedSearch carries unresolved names; three
  near-identical Search structs** (Implementation Approach; Phase 3 §2).
  - Fix: rename the stages `ConfiguredSearch` → `ValidatedSearch` →
    `LowerableSearch`, and factor out the shared `team_ids`/`text`.
- 🟡 major / medium — **CatalogueDocument stores known sections as untyped
  Value beside the typed TeamEntry** (Phase 1 §1, §3).
  - Fix: type `teams` and `labels`, derive the projections at serialisation,
    and keep `Value` only for `extra`.
- 🟡 major / medium — **Legacy catalogue reading is spread across modules**
  (Phase 1 §1, §4; Phase 2 §2).
  - The sites are CatalogueDocument, `base_team_id`, TeamStates, `auth.rs`
    and `report_team_key_writeback`.
  - Fix: add one `Catalogue::base_entry()`, and share a parse function with
    an explicit strictness policy.
- 🟡 major / medium — **CatalogueBackfill adds a hidden side effect to a
  read port method, threaded through two Option layers** (Phase 3 §5).
  - Fix: add a `CatalogueHealing` object that pairs `hold` and `take`, and
    pass it to `run_sync` without an `Option`.
- 🔵 minor / medium — **NameResolver returns a multi-id Resolution its
  contract forbids** (Phase 2 §1, §3).
  - Fix: give it a single-id result type.
- 🔵 minor / medium — **The search step sequence is described twice**
  (Phase 3 §3).
  - Fix: extract a private
    `complete_scope(...) -> Result<LowerableSearch, CompletionFailure>`.
- 🔵 minor / medium — **Pipeline stage encoded in wire key strings
  (`label.name`)** (Phase 3 §2).
  - The spelling looks like a GraphQL path.
  - Fix: choose a clearly internal spelling, confined to `to_pairs` and
    `from_pairs`.
- 🔵 minor / medium — **Unresolved uses `&'static str` and an Option tier**
  (Phase 2 §1).
  - Fix: add a `CatalogueSection` enum, and consider splitting the ambiguity
    variants.
- 🔵 minor / low — **Exhaustive matches gain arms for a variant that write
  paths cannot produce** (Phase 3 §1).
  - Fix: route them through one shared helper, and drop
    `E_PUSH_UNCONFIGURED`.
- 🔵 suggestion / medium — **Five-step finalisation grows an already large
  sync.rs** (Phase 3 §5).
  - Fix: move it into its own module, or into a `linear-client` service.

### Test Coverage

**Summary**: The plan is test-first throughout, with tests at sensible
levels. The main risks are in the harnesses and the seams:
- the single-key `Route::Sequence` couples the tests to request order;
- finalisation is tested only from pre-filled buffers;
- one named test cannot be written;
- the coverage tests contradict the fetch-only-needed-sections tests.

**Strengths**:
- Every phase names its red tests and the existing tests each change breaks.
- The refusal paths are covered thoroughly.
- The zero-request guarantees use the `server.hits` pattern.
- The no-refetch regression test goes through the real writer.
- The hold-atomicity tests are pinned at the port.
- The Phase 4 binary test was checked as feasible.
- The `RecordingTracker` extension is safe for the existing suites.

**Findings**:
- 🟡 major / high — **A single-key Route::Sequence couples multi-query tests
  to request order and masks extra requests** (Phases 1 and 3).
  - The last response repeats forever, so the "nothing fetched" tests can
    pass vacuously.
  - Fix: add a GraphQL-aware route, and assert request counts per operation.
- 🟡 major / medium — **The coverage tests contradict the needed-sections
  fetch tests** (Phase 2 coverage vs Phase 3).
  - Fix: decide the rule, and pin
    `a_team_folded_with_only_the_needed_sections_completes`.
- 🟡 major / high — **Finalisation tests pre-fill the buffer, so the
  unfiltered 0229 growth path is untested** (Phase 3 `sync.rs` tests).
  - Fix: add `finalising_catalogues_an_imported_team_that_was_never_fetched`.
- 🟡 major / high — **grow_linear_catalogue has no tests today** (Phase 1
  §4; Phase 3).
  - The Phase 1 rewire has no red test, and there is nothing to move.
  - Fix: add a Phase 1 unit test in `sync.rs`.
- 🟡 major / high — **run_sync_hands_its_backfill_to_the_registry cannot be
  written, and the warning test cannot see the exit code** (Phase 3).
  - Fix: test the wiring on `ConfiguredTrackers`, inject a diagnostic
    writer, and check the exit code via `drive_sync`.
- 🟡 major / medium — **No tests for per-team attribution, nested
  pagination, or the fallback** (Spike; Phase 1).
  - Fix: add tests for shared nodes, nodes from unrequested teams, and
    nested connections past one page.
- 🔵 minor / high — **Section-level merge and deterministic output are
  unpinned** (Phase 1 cache tests).
  - Fix: add a section-kept test and a byte-exact golden test.
- 🔵 minor / medium — **Exit-74 rendering through the binary is untested
  until Phase 4** (Phase 3).
  - Fix: add `a_linear_unknown_label_filter_refuses_with_exit_74`.
- 🔵 minor / medium — **Archived-entity boundaries are only partly
  covered** (Phase 2).
  - Fix: add a single archived project, plus archived states and labels.
- 🔵 minor / medium — **Deleted tests and legacy-path binary tests have no
  replacement** (Phase 1; Phase 3 `seed_catalogue`).
  - Fix: list the retirements, decide the zero-states behaviour, and keep
    `seed_legacy_catalogue`.
- 🔵 minor / medium — **No test that the base team is paged once when it is
  also in `teams`** (Phase 1 §1).
  - Fix: add a `port.rs` case asserting two issue-search requests.
- 🔵 suggestion / medium — **Laziness test targets a mechanism; truncation
  test needs 201 round-trips**.
  - Fix: restate laziness as behaviour, and make the ceiling injectable.
- 🔵 suggestion / medium — **Two acceptance criteria are not traced to named
  tests**: a cross-user tier precedence match, and a value served only on a
  second page.

### Correctness

**Summary**: The domain model is mostly sound, but several rules contradict
each other in ways that would give wrong results:
- coverage requires all four sections, while the fetch retrieves only the
  needed ones, so fetched teams are refused;
- finalisation cannot map imported keys to ids;
- the merge, legacy-base and malformed-entry semantics are ambiguous.

**Strengths**:
- The pre-flight/completion split correctly avoids resolving against teams
  that are not yet known.
- Holding only after every pass succeeds keeps the buffer free of partial
  state.
- `Unconfigured` → `DiscoveryUnconfigured` fixes the transient mislabel.
- `LowerableSearch` makes lowering names impossible.
- `resolve_scope` runs before `resolve_entities`, and the base key is
  substituted only for `Keyed` with an empty `additional`.

**Findings**:
- 🔴 critical / high — **Partial-section live fetch leaves teams uncovered,
  so completion refuses them as TeamUnfetched** (Phase 3 §2, §3; Phase 2
  coverage).
  - Every filtered pull against today's catalogues would refuse.
  - Fix: make `covers` section- or family-aware, use it consistently, and
    keep full completeness only for healing.
- 🟡 major / high — **Finalisation cannot fetch newly imported teams: it has
  keys, but the fetch takes ids** (Phase 3 §5).
  - Fix: keep a key→id step, or carry `(key, id)` out of the run, and add a
    no-filter test.
- 🟡 major / high — **Contradictory merge semantics: whole-entry vs
  section-level** (Phase 1 §3; Desired End State).
  - Fix: use per-section replacement, merge same-id entries in an update,
    and rename the test.
- 🟡 major / medium — **Base-team pointer and legacy derivation undefined
  once the base is in `teams`** (Phase 1 §1; Phase 3 §5; Migration Notes).
  - Healing may never converge.
  - Fix: set `baseTeam` from `/team/id` on every write where it is absent,
    and prefer the matching `teams` entry.
- 🟡 major / medium — **Zero-ids refusal is per family, but the work item
  requires a refusal per value and no over-matching** (Phase 2 §1;
  Phase 3 §2).
  - Fix: refuse every `NotFound` value, and decide the multi-match and
    archived rules.
- 🔵 minor / medium — **Writer behaviour for malformed or unknown entry
  content is unspecified** (Phase 1 §3).
  - Fix: refuse, or preserve the raw content with `extra`.
- 🔵 minor / medium — **Workspace labels are outside coverage and cannot be
  folded in** (Phase 3; Phase 2 `with_team_entries`).
- 🔵 minor / medium — **Pre-flight's base entry may not be the team the
  scope searches** (Phase 3 §2).
  - Fix: resolve against the team id resolved from the key.
- 🔵 minor / medium — **The heal set can include teams the fetch no longer
  returns** (Phase 3 §5).
  - Fix: record the returned entries, and name the missing teams.
- 🔵 suggestion / medium — **Active-wins deviates from the work item's
  within-tier ambiguity rule**, and deduplication by id has no rule for
  which record wins.

### Compatibility

**Summary**: Pre-0292 binaries can still read the converged file, and the
port change is deliberate and enumerated. The main risks are:
- the `NoTeam`→77 remap through the context-free `for_client`;
- typed round-trips dropping unknown fields;
- assignee scoping breaking working configs;
- mixed-version and search changes that the notes understate.

**Strengths**:
- Old readers ignore the extra fields, and `fetch_all` deduplicates the
  base id.
- The projections keep the old state resolver and `auth.rs` working.
- The `Unconfigured` addition is deliberate and fully enumerated.
- Exit code 89 follows the `SEARCH_CAP_HIT` pattern.
- `project` lands last, with its mixed-version warning.

**Findings**:
- 🟡 major / high — **The NoTeam→77 remap for search cannot be expressed in
  the context-free for_client** (Phase 3 §6, §3).
  - `NoTeam` is raised at construction for every flow.
  - Fix: add a distinct search-side refusal mapped to 77, and pin that create
    stays at 105.
- 🟡 major / medium — **The typed entry round-trip drops unknown per-entry
  and per-record fields** (Phase 1 §1, §3).
  - Fix: add flattened `extra` maps, and test with unknown keys.
- 🟡 major / medium — **Assignee resolution narrowed to scoped-team members
  can break working configs** (Phase 2 §1; Migration Notes).
  - Fix: fall back to workspace users, or document the break with a
    former-member test.
- 🔵 minor / medium — **Pre-0292 grow writes break id and key ordering**
  (Migration Notes).
  - Old binaries append rather than sort, and deduplicate by key only.
  - Fix: serialise through a sorted `Value`, tolerate unsorted entries and
    duplicate ids, and soften the "merges cleanly" claim.
- 🔵 minor / high — **Teams erased by a pre-0292 init are not re-added
  "when items are imported again"** (Migration Notes).
  - Fix: derive synced teams from tracked items, or correct the notes.
- 🔵 minor / high — **Standalone search behaviour and exit-code changes are
  not in the CHANGELOG** (Phase 3 §3, §7).
  - The changes are label/assignee team scoping, 0→89, and 78→77.
- 🔵 minor / medium — **Removing the projections in 0294 has no version
  gate** (What We're NOT Doing; Migration Notes).
  - Fix: set a minimum plugin version, or a `schemaVersion`.
- 🔵 suggestion / low — **Nested-connection candidates may exceed Linear's
  complexity budget** (Spike).
  - Fix: record the complexity per page, and pin explicit `first:` sizes.

### Safety

**Summary**: The plan has strong safety habits: refusals come before any
write, zero ids are refused, init is all-or-nothing, preview writes nothing,
and the writer is strict. The remaining risks concern the committed
catalogue:
- the delete remedy loses the reconcile list;
- the typed round-trip drops data;
- behaviour for teams that cannot be fetched is unspecified;
- the source of `baseTeam` is unspecified.

**Strengths**:
- Refusals surface in `prepare_run`, before any pull, push or create.
- Refusing zero ids closes the path to silently widened discovery.
- Init is all-or-nothing, and holding only after every pass succeeds is
  pinned.
- Preview is pinned byte-identical, and search uses `NoBackfill`.
- `load_for_update` refuses damaged input, and the atomic write happens
  under the lock.
- A failed finalisation only warns, and `Unconfigured` keeps the baseline on
  write paths.
- The mixed-version hazards are called out.

**Findings**:
- 🟡 major / high — **The remedy "delete the file and re-run init" loses the
  synced-team reconcile list permanently** (Phase 1 §3; Phase 3 §6;
  Migration Notes).
  - Fix: restore from VCS first, or re-derive synced teams from tracked id
    prefixes.
- 🟡 major / medium — **The typed round-trip of `teams` can drop per-entry
  data** (Phase 1 §1, §3).
  - Fix: add an `extra` map per entry and record, refuse unparseable
    entries, and add tests.
- 🟡 major / medium — **Finalisation behaviour when a catalogued team can no
  longer be fetched is unspecified** (Phase 3 §5).
  - Fix: record the returned teams, name the missing ones, and never write a
    partial entry.
- 🔵 minor / medium — **The source of `baseTeam` for sync finalisation is
  unspecified** (Phase 3 §5; Migration Notes).
  - Fix: never change an existing `baseTeam`, and test with a credential
    team that differs.
- 🔵 minor / medium — **Pin the `in_scope`/`fetch_all` invariant across the
  team resolver rewrite** (Phase 2 §3).
  - Otherwise a live issue could be reported absent and unlinked.
- 🔵 minor / low — **The pre-0292 init mitigation relies on coordination and
  has no recovery step** (Migration Notes).
- 🔵 minor / low — **Lenient `append_line` can truncate an unreadable
  `.gitignore`** (Phase 1 §3).
  - Fix: propagate `Err` from `append_line`.
- 🔵 suggestion / low — **Healing also runs on `--push-only` and targeted
  apply runs** (Phase 3 §5).

### Performance

**Summary**: Local compute is handled well: the file is parsed once, indices
are lazy, each search is composed once, and base-only pulls make no
requests. The open risk is remote-request cost, which the plan defers to an
unrun spike with no budget:
- non-synced scoped teams re-fetch every pull;
- nested connections risk the complexity cap;
- the 200-page ceiling has no deadline.

**Strengths**:
- Base-only pre-flight makes zero requests.
- Resolution moves out of `fetch_page`.
- The live fetch is limited to the needed sections, and skipped when
  covered.
- Held entries are reused at finalisation, and no-refetch is pinned.
- The catalogue is read once, with lazy HashMap indices.
- Finalisation is one fetch plus one write, after paging.
- The `in` lists scale with the number of teams, not with catalogue size.

**Findings**:
- 🟡 major / high — **Non-synced teams in a broadened scope re-fetch whole
  sections on every pull** (Phase 3 Overview; Performance Considerations).
  - Fix: use value-targeted lookups for teams that are never written, or at
    least a stated budget.
- 🟡 major / medium — **Spike has no acceptance budget, and the per-team
  fallback is unbounded** (Spike).
  - Fix: set exit criteria per section for N = 50 and N = 200 teams.
- 🟡 major / medium — **Nested-connection queries may exceed Linear's
  complexity cap and the response bound** (Phase 1 §2).
  - Fix: pass the page size per section into `paginate`, and assert `first:`
    in tests.
- 🔵 minor / medium — **The 200-page ceiling allows a large request volume
  with no deadline** (Phase 1 §2).
  - Fix: apply `Deadline`, size the ceiling from the budget, and fail fast.
- 🔵 minor / medium — **Per-team duplication inflates catalogue size and
  parse cost** (Phase 1 §1).
  - Fix: measure it in the spike, and consider top-level records with
    per-team id lists.
- 🔵 suggestion / low — **Make the repeated-fetch cost observable**:
  print one stderr line per pull.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-24

**Verdict:** REVISE

The edits resolve 54 of the 62 prior findings, including the critical
coverage contradiction; all seven lenses agree. Three new majors reach the
REVISE threshold. One of them is a real correctness error introduced by the
edit that derives synced teams from the run report.

### Previously Identified Issues

- 🔴 **Correctness + Test Coverage**: Partial-section live fetch leaves
  teams uncovered — Resolved. `covers(team, &SectionSet)` drives the
  partition, the post-fold check and pre-flight.
- 🟡 **Architecture + Correctness + Test Coverage**: Finalisation knows
  imported teams by key, but the fetch takes ids — Resolved (catalogue, then
  held entry, then enumeration).
- 🟡 **Architecture + Code Quality + Test Coverage**: Finalisation's seams —
  Partially resolved. `TeamEntryFetch`, `CatalogueHealing` and `RunFinaliser`
  are defined. The registry wiring tests still assert `Arc` counts, and
  `drive_sync` discards the exit code.
- 🟡 **Correctness + Test Coverage**: Merge-rule contradiction — Resolved.
- 🟡 **Safety + Compatibility + Correctness + Code Quality**: Typed
  round-trip drops data — Resolved.
- 🟡 **Correctness + Safety**: `baseTeam` and legacy derivation — Resolved.
- 🟡 **Safety + Compatibility**: Delete or pre-0292 init loses synced teams —
  Partially resolved. The remedy text is fixed, but the re-derivation it
  relies on reads local ids (see the new major finding below).
- 🟡 **Safety + Correctness**: Unreachable team at finalisation — Resolved.
- 🟡 **Correctness**: Per-family zero-ids refusal and over-matching —
  Resolved.
- 🟡 **Compatibility**: `NoTeam`→77 remap — Resolved.
- 🟡 **Compatibility**: Assignee scoping breaks configs — Resolved
  (documented break).
- 🟡 **Code Quality + Architecture**: `Unconfigured` classed as `Retryable`
  — Resolved.
- 🟡 **Code Quality**: `ResolvedSearch` naming and data clump — Resolved.
- 🟡 **Code Quality**: Scattered legacy reads — Resolved.
- 🟡 **Performance + Architecture**: Non-synced teams re-fetch every pull —
  Resolved (bounded by the spike budget, with a named fallback).
- 🟡 **Performance**: Spike has no budget — Resolved.
- 🟡 **Performance + Compatibility**: Nested complexity cap — Resolved.
- 🟡 **Test Coverage**: Single-key `Route::Sequence` — Partially resolved.
  It does not reach the `linear-cli` scenario loader.
- 🟡 **Test Coverage**: `grow_linear_catalogue` untested — Resolved.
- 🟡 **Test Coverage**: Attribution and nested paging tests — Resolved.
- 🔵 **Performance**: 200-page ceiling without a deadline — Partially
  resolved. A deadline is added, but its scope and its exit mapping are
  unspecified, and the ceiling is not sized from the budget.
- 🔵 **Performance**: Live-fetch observability — Still present
  (suggestion, deliberately skipped).
- 🔵 All other prior minors and suggestions (architecture 5, code quality 6,
  test coverage 7, correctness 4, compatibility 4, safety 5) — Resolved.

### New Issues Introduced

Major:

- 🟡 **Safety + Correctness + Test Coverage**: Synced teams are derived from
  local work-item ids, not Linear keys.
  - For tracked items, `ReportedItem.planned.id` is the local frontmatter
    `id` (`plan.rs:129`, `work-cli/src/sync.rs:116`). Only
    `CreateFromRemote` entries carry the external id.
  - The re-derivation therefore finds nothing under numeric ids, and under
    `{project}-{number}` ids it reads the local project code as a team key.
    That code may never map, or may match an unrelated Linear team whose
    members' emails would then be committed.
  - "Whatever its action" also counts failed imports.
  - Nothing tests the derivation where it happens.
  - Fix: derive synced keys from the corpus items' `external_id`, plus
    applied `CreateFromRemote` ids, and pass them to the finaliser. Move the
    derivation into `linear-client` and test it with realistic local ids.
- 🟡 **Correctness**: A team with no labels or no projects risks being read
  as "not returned".
  - The plan does not say whether team presence comes from the identity
    lookup or from section nodes.
  - Fix: presence comes from the `{id, key, name}` lookup, and a present
    team with no nodes gets `Some([])`. Keep the non-empty guard for `states`
    only.
- 🟡 **Test Coverage**: The per-operation GraphQL route does not reach the
  `cli-test-support` scenario loader.
  - The `flow_init.rs` and `flow_search.rs` tests remain coupled to request
    order.
  - The unmatched-operation semantics, coexistence with plain routes, and
    multi-root keying are unspecified. A 599 response can make error-path
    tests pass without testing anything.

Minor:

- 🔵 **Architecture + Safety + Test Coverage**: Healing on enumeration or
  fetch failure, and keys that never map, are unspecified and untested.
  `HealOutcome` needs an `unmapped` bucket, and held entries should still be
  recorded.
- 🔵 **Architecture**: Healing reaches one remote through two handles
  (`&dyn RemoteTracker` for key→id, and a second client for the fetch).
  Consider `TeamEntryFetch::teams_by_key`.
- 🔵 **Safety**: The lenient `ensure_scaffold` clashes with the strict
  `append_line`. The scaffold would fail after the catalogue write succeeded.
- 🔵 **Safety**: One damaged entry blocks healing for every team, and the
  warning omits the `Unparseable` remedy.
- 🔵 **Correctness + Compatibility**: The precedence of the `Unconfigured`
  exit code against awaiting-human (4) is unspecified, and the `unconfigured`
  report token is undocumented.
- 🔵 **Correctness**: The merge rule does not cover `key` and `name`, entry
  and record `extra` on replaced sections, or which of two same-id entries
  wins a section.
- 🔵 **Compatibility**: The numeric type of `position` decides whether the
  round-trip is lossless; use `serde_json::Number`. Declare `sections` before
  `extra`.
- 🔵 **Code Quality**: `SingleResolution` reuses the multi-team `Unresolved`.
  `Fetched` and `FetchedEntries` are near-homonyms. `parse` returns
  `CatalogueGap` and so cannot name the entry.
- 🔵 **Code Quality + Test Coverage**: The registry wiring test asserts an
  `Arc` count. `drive_sync` discards the exit code, and the credential test
  cannot be observed.
- 🔵 **Test Coverage**: Finalisation is not tested for targeted runs or
  failed discovery. A port test for the conditional fetch of workspace labels
  is missing.
- 🔵 **Performance**:
  - The member and project budgets scale with record count, not team count.
  - The ceiling is not sized from the budget.
  - The deadline's scope is ambiguous and should map to `Retryable`.
  - An empty heal set has no pinned zero-cost path.
  - Complexity is not budgeted per pull against the hourly allowance.
- 🔵 **Code Quality** (suggestion): `WorkspaceLabels` is a catalogue-wide
  fact inside a per-team predicate.

### Assessment

The design is sound, and every structural concern from the first pass is
fixed. The remaining majors are all specification errors that are cheap to
correct:
- the synced-team derivation must read `external_id` from the corpus;
- team presence must come from the identity lookup;
- the harness change must extend to the scenario loader.

Fix those three and the healing failure modes, and the plan should reach
COMMENT or APPROVE on the next pass.

## Re-Review (Pass 3) — 2026-09-24

**Verdict:** COMMENT

All three pass-2 majors are resolved, and every lens agrees:
- synced teams now come only from Linear external ids;
- team presence comes from the identity lookup;
- the operation-keyed harness reaches the scenario loader.

One new major remains, below the REVISE threshold. The rest are minors at
the edges of the new rules. The plan is acceptable, but could be improved;
see the major finding below.

### Previously Identified Issues

- 🟡 **Safety + Correctness + Test Coverage**: Synced teams derived from
  local work-item ids — Resolved. `SyncedTeams::derive` accepts only
  `ExternalId`s from the corpus and applied imports, and is tested with
  realistic local ids.
- 🟡 **Correctness**: A team with no labels or projects read as "not
  returned" — Resolved (identity lookup; `Some([])` covers its section).
- 🟡 **Test Coverage**: Per-operation route does not reach the scenario
  loader — Resolved. First-root-field keying is now too coarse (see below).
- 🔵 **Architecture**: Two handles to one remote — Resolved
  (`teams_by_key`).
- 🔵 **Architecture + Safety + Test Coverage**: Healing failure modes and
  unmapped keys — Resolved.
- 🔵 **Safety**: `ensure_scaffold` vs `append_line` — Resolved.
- 🔵 **Safety**: A damaged entry blocks healing without the remedy —
  Resolved.
- 🔵 **Correctness + Compatibility**: `Unconfigured` exit precedence and
  report token — Resolved (71 > 4 > 74 > 70, documented).
- 🔵 **Correctness**: Merge-rule identity and extras — Resolved.
- 🔵 **Compatibility**: `position` type and flatten ordering — Resolved.
- 🔵 **Code Quality**: `SingleResolution`, fetch-type naming, parse error —
  Resolved.
- 🔵 **Code Quality + Test Coverage**: Registry wiring tests — Partially
  resolved. The test bypasses `resolve`, and the no-token test depends on
  the host environment.
- 🔵 **Test Coverage**: Discovery-status coverage and the workspace-labels
  port test — Resolved.
- 🔵 **Performance**: Record-scaled budgets, deadline scope, zero-cost
  empty heal, per-pull complexity — Resolved. The ceiling is only partly
  sized from the budget (see below).
- 🔵 **Performance**: Live-fetch observability — Still present (declined).
- 🔵 **Code Quality**: `WorkspaceLabels` in a per-team predicate — Still
  present (declined).

### New Issues Introduced

Major:

- 🟡 **Test Coverage + Compatibility**: First-root-field operation keys
  collide.
  - Three operations share the root field `teams`: enumeration, the
    identity lookup and `teams_by_key`.
  - Team labels and workspace labels share `issueLabels`.
  - Every existing query is anonymous, so there is no operation name to key
    on instead.
  - As a result, the whole-workspace real-client test is tied to request
    order again, and some per-operation counts cannot be told apart.
  - `Scenario::body_expectations` still returns plain keys.
  - Fix: give each discovery and catalogue query an operation name, key on
    that name, and fall back to the first root field only for anonymous
    queries.

Minor:

- 🔵 **Correctness + Safety + Compatibility + Architecture**: A prefix-derived
  team key can be wrong in four ways:
  - stale after a key rename;
  - reused by a different team;
  - left over from Jira;
  - pointing at a retired team, which `teams(filter)` omits without
    `includeArchived`.

  The result is either a permanent misleading "not visible to this
  credential" warning, or an unrelated team (and its members' emails) being
  catalogued. Fix: admit a derived key only when this run's reconcile read,
  or an applied import, confirmed an identifier under it on Linear;
  otherwise report it as `unconfirmed`. Pass `includeArchived` to both team
  lookups.
- 🔵 **Correctness**: Healing's partial-failure rules leave three gaps:
  - Does "complete" mean stored plus held sections?
  - An empty update still takes the lock and rewrites the file, so skip it.
  - Missing workspace labels never trigger a heal.
- 🔵 **Safety**: Keys dropped by a failed `teams_by_key` call have no
  bucket. Put them in the fetch-failure bucket, not in `unmapped`.
- 🔵 **Architecture + Code Quality**: The base key passed to
  `SyncedTeams::derive` comes from somewhere `work-cli` does not specify.
  Have `heal` add the base from `base_entry()` itself, and take a single
  id iterator.
- 🔵 **Code Quality + Test Coverage**: The lazily built fetch closure does
  not fit `heal`'s eager parameter, and there is no path for a failed build.
  Take a factory, or build eagerly, and test the diagnostic.
- 🔵 **Performance**:
  - The 60-page ceiling is sized from the team-scaled budget, but it also
    bounds record-scaled sections.
  - The shared deadline cannot span the separate `fetch_workspace_labels`
    method.
  - The identity and key lookups have no page size or pagination; Linear's
    default is 50.
- 🔵 **Test Coverage**:
  - The registry test bypasses `resolve`.
  - The no-token test depends on the host environment; inject an
    `Environment`.
  - There is no positive test for an applied import reaching the finaliser.
  - Plain-key recording under coexistence is unspecified, and
    `assert_no_unmatched` works only by convention.
  - There is no test for the `unconfigured` render token.

### Assessment

The plan is structurally sound and implementable. The remaining major is a
harness-keying detail: name the operations, which is cheap and should happen
before Phase 1 lands. The minors cluster on two themes:
- **Trusting identifier prefixes as team identity.** Confirm keys against
  this run's reconcile, and look up archived teams.
- **Specification gaps on healing's edges.** Empty writes, the source of
  the base key, workspace labels, and pagination of the lookups.

None changes the design. A short edit pass would bring this to APPROVE.

## Re-Review (Pass 4) — 2026-09-24

**Verdict:** COMMENT

Pass 3's major finding, the colliding operation keys, is resolved. Most of
its minors are resolved too: healing's edge cases, the source of the base
key, the fetch factory, per-section ceilings, the deadline scope, the
identity-lookup paging, and the registry and environment tests. Two new
majors come from the mechanism that confirms prefixes. That is below the
REVISE threshold, but both should be fixed before Phase 3.

### Previously Identified Issues

- 🟡 **Test Coverage + Compatibility**: First-root-field operation keys
  collide — Resolved (named operations, with the first root field only as
  a fallback).
- 🔵 **Correctness + Safety + Compatibility + Architecture**: A
  prefix-derived team key can be wrong — Partially resolved.
  - Renamed keys and archived teams are handled.
  - A buffer shortcut still skips confirmation.
  - A Jira `PROJ-12` resolves when Linear team `PROJ` has an issue 12.
  - The batch lookup's error behaviour, and the way `formerKeys` is
    inferred, open new holes (see the majors below).
- 🔵 **Correctness**: Healing edge cases — Resolved.
- 🔵 **Safety**: Dropped keys on a failed lookup — Resolved.
- 🔵 **Architecture + Code Quality**: Source of the base key; the lazy fetch
  — Resolved.
- 🔵 **Performance**: Per-section ceilings, deadline scope, lookup paging —
  Resolved.
- 🔵 **Test Coverage**: Applied-import test, `unconfigured` token, harness
  recording and the unmatched-operation guarantee — Resolved. The registry
  wiring test is only partly resolved.

### New Issues Introduced

Major:

- 🟡 **Compatibility + Correctness + Performance**: One unknown identifier
  fails the whole aliased `issue(id:)` batch.
  - `Query.issue` is non-null (`Issue!`), so "Entity not found" nulls
    `data` for the whole response. The repo pins this as HTTP 400 in
    `port.rs::a_404_shaped_read_failure_is_retryable_never_terminal`.
  - `classify::carries_errors` fails the whole call.
  - So `unconfirmed` never happens. Up to 49 genuine prefixes are reported
    as a fetch failure on every apply sync.
  - The mock returns a null per alias, so it hides this.
  - Fix: look up one identifier per request and treat "Entity not found" as
    unconfirmed; the count is small, because only unnamed prefixes need a
    lookup. Try the next identifier before marking a prefix unconfirmed.
    Script the mock with the real response: 400, `errors[]`, `data: null`.
- 🟡 **Correctness + Architecture**: Inferring `formerKeys` from one
  representative issue cannot tell a moved issue from a renamed team.
  - If ENG-3 has moved to OPS, ENG becomes a former key of OPS, and team
    ENG is never catalogued.
  - If a key is reused, the new holder is hidden behind the old team's
    former key.
  - `formerKeys` only ever grows, so nothing can undo a false alias.
  - The plan does not say whether `team_by_key` or `in_scope` consult
    `formerKeys`.
  - Fix: record a former key only when no team currently holds the prefix;
    let a current key win over a former one; state that `formerKeys` is
    used only by healing. Alternatively, drop `formerKeys`, and confirm
    teams from every tracked issue's current team rather than from
    prefixes.

Minor:

- 🔵 **Safety**: When a held entry's key matches a prefix, the heal skips
  confirmation (`healing_catalogues_a_synced_team_the_catalogue_does_not_name`).
  On a whole-workspace pull that could catalogue an unrelated team.
  Confirm every unnamed prefix, and use the buffer only for its sections.
- 🔵 **Correctness**: The guarantee that "a borrowed key never resolves" is
  false when the Linear team has that issue number. Reword it, pin it in
  the fixture, and accept the residual risk explicitly.
- 🔵 **Correctness**: Step 5's write condition does not cover an update that
  only adds a former key. Define the record set as the entries that change.
- 🔵 **Code Quality**: `formerKeys` appears in Phase 1's table and merge rule
  but has no Phase 1 test, and Phase 3 says the field is "gained" there.
  `FetchUnavailable` and its mapping from `SelectionError` are undefined.
  `teams_of_identifiers` lands in Phase 1 with no caller until Phase 3.
- 🔵 **Test Coverage**:
  - Dual recording can advance a plain sequence on requests it did not
    serve.
  - The `Arc::ptr_eq` wiring test cannot see `resolve`'s client. A
    crate-private endpoint override would allow a behavioural test.
  - `with_environment` with a `Box` default conflicts with `const fn new`.

### Assessment

The rest of the plan has converged. Every remaining finding comes from the
mechanism added in pass 3 to confirm prefixes as teams. The simplest
correct design is probably to drop `formerKeys` altogether, and derive
synced teams from each tracked identifier's current team: look up one
identifier per request, try the next on "not found", and confirm every
unnamed prefix. That removes both majors and three of the minors.

## Approval — 2026-09-24

**Verdict:** APPROVE

The reviewer approved the plan after the pass-4 fixes were applied, and set
it to `ready`. The fixes were:

- **Simpler prefix confirmation.**
  - `formerKeys` is dropped.
  - `team_of_identifier` makes one `TeamOfIdentifier` request per
    identifier. Linear's 400 "Entity not found" maps to `None`, and every
    other failure is an `Err`.
  - Each unnamed prefix tries up to three identifiers, moving on when an
    identifier is not found or its issue now belongs to another team.
  - A buffered team is never taken on key alone.
  - The collision with a foreign id is accepted and documented, with a test
    and a CHANGELOG Security note.
- **Healing records only what changes.** Healing's record set is the entries
  whose sections change, plus any fetched workspace labels.
- **Code quality.**
  - `FetchUnavailable { reason }` is defined in `linear-client`, and the
    finaliser maps `SelectionError` into it.
  - `team_of_identifier` lands in Phase 3 alongside its first caller.
  - `formerKeys` is removed from Phase 1.
- **Harness.** A route's sequence advances only on the requests that route
  served.
- **Registry.**
  - `with_environment` takes `&dyn Environment`, stored as an `Option`, so
    `new` stays `const`.
  - `with_linear_endpoint` enables a behavioural test: a client from
    `resolve` fills the buffer that healing reads.
- **Spike gating.** The spike now gates Phase 1 §2, not the plan's `ready`
  status.

No further lens pass was run after these fixes.

