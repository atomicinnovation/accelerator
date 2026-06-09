---
date: "2026-06-02T00:45:15+00:00"
type: plan-review
producer: review-plan
target: "plan:2026-06-01-lifecycle-clustering-composite-key"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, compatibility, safety, performance]
review_pass: 2
status: complete
id: "2026-06-01-lifecycle-clustering-composite-key-review-1"
title: "2026-06-01-lifecycle-clustering-composite-key-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-06-02T00:45:15+00:00"
last_updated_by: Toby Clemson
---

## Plan Review: Lifecycle Clustering Composite Key Implementation Plan

**Verdict:** REVISE

The plan's phased architecture, TDD-first ordering, and reuse of existing
secondary indexes are sound, and the bug-fix story is well-motivated by the
research it cites. However, the reviews surface one critical downstream
inconsistency (the inferred-cluster lookup in `related.rs` silently breaks
when the new cluster representative slug diverges from member entries' slugs)
and several major issues that span lenses: the back-compat shim's
"empty-snapshots-means-slug-only" contract is implicit and easy to
mis-trigger, target-resolution logic is duplicated across three modules,
`parse_typed_ref` drops the existing path-validation defence and misclassifies
`plan:<path>` hybrid values, the `PROJ-NNNN-` slug-strip claim is not
implemented, and Phase 3 silently widens `/api/related/*` for three doc types
with no explicit announcement or fixture update plan.

### Cross-Cutting Themes

- **Back-compat shim with empty maps is implicit and risky** (flagged by:
  architecture, code-quality, test-coverage, safety, correctness) — Keeping
  `compute_clusters_with_backfill` as a slug-only shim that calls the new
  typed variant with empty maps + dummy root only works because
  `entry_for_test` produces null frontmatter and `work_item_id: None`. Any
  production caller missed during the migration or any future test that
  populates `work_item_id` will silently produce wrong clusters with no
  failure. Recommended fix: remove the shim (update existing slug-only tests
  to pass empty maps explicitly) or rename it to make the slug-only
  assumption named, not implicit.
- **Depth-limit truncation is silent** (flagged by: architecture,
  test-coverage, safety, correctness, performance) — `MAX_DEPTH = 4` is a
  bare integer; cycles and over-depth chains both produce indistinguishable
  `None` results with no log. The cycle test alone cannot pin both ends of
  the bound (a mutation `MAX_DEPTH = 3` would still pass it). Recommended
  fix: split the boundary test (one chain at exactly `MAX_DEPTH - 1`
  resolves; one cycle or `MAX_DEPTH + 1` returns `None`), bump the limit to
  6–8 (essentially free), and emit a `tracing::warn!` when the limit is
  hit.
- **Identity-extraction logic is duplicated across three modules** (flagged
  by: architecture, code-quality) — `target_path_from_entry` (Phase 3),
  `cluster_key::walk` (Phase 4), `parent_or_legacy_id` /
  `id_from_value` / `canonicalise_one`, and an inline path-shape branch all
  re-parse the same `target:` / `work_item_id:` / `parent:` value space.
  Future vocabulary additions require updating each. Recommended fix: have
  `cluster_key::walk` call `target_path_from_entry` for review/validation
  branches and recurse on the resulting `IndexEntry`, reserving direct
  `parse_typed_ref` use for the `WorkItem` short-circuit. Consolidate the
  three id-from-value helpers behind a single typed parser.
- **`parse_typed_ref` is too loose to be the contract surface** (flagged by:
  code-quality, correctness, compatibility, test-coverage) — As specified,
  it accepts empty IDs (`work-item:` → `Some(WorkItem(""))`), misclassifies
  `plan:<path>` as a Plan ID with no path fallback, and drops the
  `..`/absolute/NUL/backslash rejection that `normalize_target_key`
  currently enforces. Recommended fix: reject empty suffixes, fall through
  to `TypedRef::Path` when a typed-prefixed value contains `/` or ends in
  `.md`, and either return validated paths or document that callers must
  apply `normalize_target_key`.
- **Project-prefixed work-item IDs (`PROJ-NNNN-`) are claimed in prose but
  not implemented** (flagged by: correctness, test-coverage, code-quality)
  — The Phase 1 overview promises stripping `NNNN-` *or* `PROJ-NNNN-` after
  the date, but `strip_optional_work_item_id_prefix` only matches
  digit-only heads. The Phase 4 resolver tests use only the default numeric
  pattern and never exercise project-prefixed workspaces, so the entire
  prefix-pattern path is unverified. Recommended fix: either drop the
  `PROJ-NNNN-` claim, or plumb `&WorkItemConfig` into the slug helper and
  add tests for project-prefixed shapes across `slug` and `cluster_key`.
- **Phase 3 contract change is framed as a side-benefit** (flagged by:
  architecture, compatibility, safety) — Generalising
  `target_path_from_entry` populates `reviews_by_target` for three
  additional doc types, which changes `/api/related/{plan-path}`
  `declaredInbound`/`declaredOutbound` arrays for those types from "always
  empty" to "potentially populated". Frontend test fixtures and the
  manual-verification's "byte-identical capture" expectation no longer
  hold. Recommended fix: name this as an explicit wire-shape behaviour
  change in Phase 3 success criteria, enumerate frontend fixtures that
  need refreshing, and add Phase 3 verification of the now-populated
  declared-inbound rows.
- **Cluster URL contract changes silently with no transition path**
  (flagged by: architecture, compatibility, safety) — Cluster URLs change
  for every ID-prefixed cluster on the next deploy; bookmarks, in-corpus
  cross-references, and active-session tabs all break. The plan's framing
  ("clusters with old bookmarked URLs simply 404, exactly as they do
  today") is incorrect — today those URLs resolve. Recommended fix: add a
  thin alias in `/api/lifecycle/:slug` (and the SPA route) that, on miss,
  matches `IndexEntry.slug` and redirects to the canonical cluster, for at
  least one release.

### Tradeoff Analysis

- **Shim convenience vs implicit-contract risk**: keeping the old
  single-arg shim minimises diff churn but encodes a behavioural switch
  inside an unmarked function. The reviews uniformly prefer explicitness
  (remove or gate the shim). Tradeoff: ~5 extra one-line test edits vs
  removing a foot-gun that five lenses independently flag.
- **Performance vs allocation cleanliness**: the plan's perf section
  understates allocation cost (a second deep-clone of all entries when
  building `entries_by_path`, repeated `Vec`/`HashSet` allocation inside
  `canonicalise_one`). At ~200 entries the absolute cost is sub-millisecond
  and not a blocker — but the plan's "no extra I/O, ~3 KB" claim should be
  corrected to acknowledge it. Cheap to fix; cheap to leave alone.
- **Depth-limit safety vs visited-set complexity**: a visited-set is
  strictly more correct (separates cycle detection from chain-length
  bounding) but bigger than the depth-int. Given the dev-tool context, the
  simpler workaround — bump `MAX_DEPTH` to 6–8 and add a warn-log — is
  sufficient.

### Findings

#### Critical

- 🔴 **Architecture**: `inferred_cluster` lookup in `related.rs` silently
  breaks when `cluster.slug` diverges from `entry.slug`
  **Location**: Phase 4: Composite cluster-key resolver — cluster
  representative slug picker; cross-impact on related.rs
  `related::resolve_related` (server/src/related.rs:27-42) finds an
  entry's inferred-cluster siblings by `clusters.iter().find(|c| &c.slug
  == slug)` keyed on `entry.slug`. After Phase 4, the cluster's slug is
  the work-item's slug while plan/research/review entries retain their
  own (now-tightened) slugs. Any cluster bucketed by `cluster_key` whose
  representative slug differs from a member's `entry.slug` will produce
  an empty inferred-cluster for that member, breaking the invariant that
  `/api/lifecycle` and `/api/related/*` agree.

#### Major

- 🟡 **Architecture / Code Quality / Safety / Correctness**: Back-compat
  shim with empty snapshot maps is a latent foot-gun
  **Location**: Phase 4 §2 — `compute_clusters_with_backfill` shim
  The plan keeps the single-arg `compute_clusters_with_backfill` as a
  shim that calls the typed variant with empty maps and a dummy root.
  This works only because `entry_for_test` produces entries with null
  frontmatter and `work_item_id: None`. If a production call site is
  missed during the Phase 4 migration, or a future test populates
  `work_item_id`, the cluster bucket key changes silently with no
  failure.

- 🟡 **Architecture / Code Quality**: Duplicated target-resolution logic
  across three modules
  **Location**: Phase 3 (`target_path_from_entry`) and Phase 4
  (`cluster_key::walk`, `parent_or_legacy_id`, `id_from_value`,
  `canonicalise_one`)
  Both `target_path_from_entry` and `cluster_key::walk` independently
  parse `target:` via `parse_typed_ref` and dispatch on the variants.
  Adding a new vocabulary form requires updating both — a recurring
  class of drift bug.

- 🟡 **Architecture / Compatibility / Safety**: Phase 3 changes the
  `/api/related/{plan}` declared-inbound contract without an explicit
  announcement
  **Location**: Phase 3: Generalise target resolution
  `reviews_by_target` now covers WorkItemReviews, PrReviews, and
  Validations. `/api/related/{plan-path}` `declaredInbound` arrays gain
  members of new types. Phase 2's "byte-identical wire capture"
  manual-verification step is invalidated for those types, and frontend
  fixtures that capture the array shapes may need refreshing.

- 🟡 **Code Quality / Correctness**: `parse_typed_ref` drops path
  validation and misclassifies `plan:<path>` hybrid shapes
  **Location**: Phase 2 (`typed_ref.rs::parse_typed_ref`) and Phase 3
  (`target_path_from_entry` rewrite)
  The parser returns `TypedRef::Path(PathBuf::from(s))` unvalidated, and
  Phase 3 resolves it as `project_root.join(p)` — bypassing the existing
  `normalize_target_key` defence against `..`, absolute paths, NUL, and
  backslash. Additionally, `plan:meta/plans/...md` returns
  `TypedRef::Plan(meta/plans/...md)` with no path fallback, silently
  dropping resolution.

- 🟡 **Code Quality**: Six-argument resolver signature with a "shim with
  empty maps" pattern
  **Location**: Phase 4 §2 — `compute_clusters_with_backfill_typed`
  The new function takes six positional args (entries,
  entries_by_path, work_item_by_id, plans_by_id, project_root,
  work_item_cfg) including three same-typed HashMaps that are easy to
  swap. A `ClusterContext` (or `&Indexer`-shaped view) would name the
  data-clump and make the shim's empty-state contract explicit.

- 🟡 **Code Quality**: Three overlapping id-extraction helpers in
  `cluster_key.rs`
  **Location**: Phase 4 — `parent_or_legacy_id`, `id_from_value`,
  `canonicalise_one`
  Four ways to extract an ID from a string (including the inline
  `Path::new(raw).file_stem()` + `format!("{stem}.md")` hack) — a smell
  that grows. The legacy-path-shape branch already overlaps with
  `parse_typed_ref`'s `TypedRef::Path` handling.

- 🟡 **Correctness**: Digit-only descriptor heads are mis-stripped as
  work-item IDs
  **Location**: Phase 1 — `strip_optional_work_item_id_prefix`
  The helper strips any leading run of ASCII digits followed by `-`,
  with no width guard. Filenames like `2026-04-17-100-day-plan.md`
  would have head `100` stripped, producing `day-plan` instead of
  `100-day-plan`. Constrain to the configured ID width or reuse
  `cfg.extract_id` so only true ID prefixes match.

- 🟡 **Correctness / Test Coverage**: `PROJ-NNNN-` strip is claimed but
  not implemented
  **Location**: Phase 1 overview vs `strip_optional_work_item_id_prefix`
  body
  The Phase 1 Overview promises stripping `NNNN-` or `PROJ-NNNN-` after
  the date. The helper only accepts digit-only heads. Project-prefixed
  workspaces' entries silently retain the prefix and never converge
  with the work-item's slug. Either drop the claim or plumb
  `WorkItemConfig` into the slug helper.

- 🟡 **Compatibility / Safety**: Cluster URL contract change with no
  redirect leaves prior links broken
  **Location**: Migration Notes §URL stability + Phase 4 representative
  slug picker
  Every cluster whose representative previously came from an
  ID-prefixed filename gets a new URL on deploy. Bookmarks, in-corpus
  cross-references, and active tabs all 404 silently. The Migration
  Notes' framing that "they 404, exactly as today" is wrong — today
  those URLs resolve. Add a one-release alias (`/lifecycle/<old>` →
  `/lifecycle/<new>`) by looking up `IndexEntry.slug`.

- 🟡 **Test Coverage**: Phase 4 integration tests are sketched, not
  specified
  **Location**: Phase 4 §3 — `clusters.rs` integration tests block
  Six of the seven tests are comment stubs; the `run_typed(&[...],
  /* fixtures */)` helper isn't specified. The most important fixture
  decision — how the resolver's snapshot maps are populated for a given
  entry list — is left unwritten, inviting tests that pass by
  coincidence.

- 🟡 **Test Coverage**: Depth-limit / cycle test conflates two concerns
  **Location**: Phase 4 cluster_key.rs tests (case 13)
  Only the cyclic case is tested. A mutation `MAX_DEPTH = 3` (or `= 2`)
  would still pass the cycle test. Split into (a) a chain of depth
  `MAX_DEPTH - 1` that resolves, asserting the correct work-item id,
  (b) a chain or cycle exceeding the limit that returns `None`.

- 🟡 **Test Coverage**: No test for project-prefixed `WorkItemConfig`
  canonicalisation
  **Location**: Phase 4 cluster_key.rs tests
  Every prescribed test uses bare numeric ids (`"0040"`/`"0042"`) and
  default config. Project-prefixed workspaces would silently produce
  wrong cluster keys with no failure. Add cases for `parent: "42"`
  under both numeric and project-prefixed patterns.

- 🟡 **Test Coverage**: No integration test exercises the motivating
  corpus shapes
  **Location**: Phase 3 / Phase 4 — overall
  Existing fixtures already include
  `meta/reviews/work/2026-05-26-ac2-coverage-review-1.md` and
  `meta/work/0099-ac2-coverage.md` (the corpus pattern this plan
  targets). No `tests/api_lifecycle.rs` test asserts the
  `/api/lifecycle` payload actually clusters them post-fix. A wiring
  bug between backfill and the indexer call site would pass every unit
  test and still ship the motivating bug.

#### Minor

- 🔵 **Architecture**: `MAX_DEPTH = 4` silently masks pathological
  linkage rather than reporting it. Emit a `tracing::warn!` at the
  limit branch identifying the entry, or use a visited-set so the
  depth limit can be raised safely.

- 🔵 **Architecture**: Orphan-by-design types (Notes, DesignGaps,
  Decisions, DesignInventories) still merge via slug fallback,
  creating asymmetric clustering semantics. Document the policy or
  gate the slug fallback to types that participate in the lifecycle
  pipeline.

- 🔵 **Architecture**: `cluster_key` module straddles the
  indexer/clusters seam; identity-extraction is split between
  `cluster_key.rs` and `indexer.rs`'s existing
  `plan_id_from_entry`/`work_item_id_from_entry`/`adr_id_from_entry`.
  Either promote both into a shared `identity.rs` or move the resolver
  into `indexer.rs`.

- 🔵 **Code Quality**: `strip_optional_work_item_id_prefix` is named for
  the work-item domain but enforces only "leading digits then hyphen".
  Either rename to `strip_optional_leading_numeric_segment` or accept
  `WorkItemConfig` and validate.

- 🔵 **Code Quality**: `parse_typed_ref`'s path-detection heuristic
  ("contains `/` or ends with `.md`") is implicit; make the
  known-prefix list explicit so a future colliding prefix can't
  silently break.

- 🔵 **Code Quality**: Two large `match DocTypeKey` arms in
  `cluster_key::walk` and `target_path_from_entry` duplicate the
  doc-type vocabulary; consider promoting per-variant behaviour onto
  `DocTypeKey` (`fn carries_target(self) -> bool`).

- 🔵 **Code Quality**: Snapshot-coherence requirement for
  `entries_by_path` (must align with the `entries` slice) is unstated;
  build it inside `compute_clusters_with_backfill_typed` from the
  borrowed slice or document the coherence invariant.

- 🔵 **Correctness**: Cluster `slug: String` picker is undefined when
  the chosen work-item entry has `slug: None`. Specify the fallback
  chain (WorkItems slug → any entry's slug → cluster_key string).

- 🔵 **Correctness**: Path-shape `work_item_id:` only resolves when
  `extract_id` accepts the stem; strict project-prefixed workspaces
  with no `default_project_code` silently fail.

- 🔵 **Correctness**: `-review-N` interaction with internal `-review-`
  tokens in the no-date WorkItemReview shape isn't pinned by a test
  case targeting a descriptor that ends in `-review`.

- 🔵 **Correctness**: `id_from_value("work-item:", cfg)` silently
  returns `None`; consider a `tracing::debug!` when a typed-prefix
  value canonicalises to empty so migration tooling can grep.

- 🔵 **Correctness**: `read_ref_keys` refactor must match *only*
  `TypedRef::WorkItem(id)` after folding to `parse_typed_ref` — pin
  this in the plan and add regression cases for `target: "adr:0034"`
  and `target: "pr:42"`.

- 🔵 **Compatibility**: React Query cache keyed on changing cluster
  representative slug retains stale entries across deploys; an alias
  layer would dissolve this, or invalidate
  `lifecycleClusterPrefix` on SSE doc-changed.

- 🔵 **Compatibility**: `clusterKey` ships on every serialised
  `IndexEntry` — appears in `/api/docs`, `/api/related`, and
  `/api/lifecycle`. The plan only documents the lifecycle endpoint;
  add a wire-shape test for one of the other endpoints to lock the
  contract.

- 🔵 **Compatibility**: `parse_typed_ref` returns
  `Some(WorkItem(""))` / `Some(Plan(""))` for prefix-only inputs;
  reject empty suffixes.

- 🔵 **Test Coverage**: Phase 1 PROJ-prefixed strip behaviour is
  unpinned — either add an explicit case or note it's out of scope.

- 🔵 **Test Coverage**: `parse_typed_ref` tests miss the empty-id
  forms (`"work-item:"`, `"plan:"`) and the bare-slug fallback
  (`"foo"` → `None`).

- 🔵 **Test Coverage**: Phase 3 has no test for the routing decision
  that `work-item:NNNN` / `adr:NNNN` / `pr:N` targets must return
  `None` from `target_path_from_entry` (resolved by the cluster-key
  resolver instead).

- 🔵 **Test Coverage**: No test pins the shim's empty-snapshot
  equivalence to the typed variant; a one-line equivalence assertion
  would lock the contract.

- 🔵 **Test Coverage**: Representative slug + cluster_key picker has
  no negative test (no-work-item branch, two-work-items branch).

- 🔵 **Test Coverage**: Phase 5 debug-tag tests cover labels but not
  the label-selection logic; extract a pure `clusterViaLabel(entry,
  cluster)` and table-test it directly.

- 🔵 **Test Coverage**: Phase 1 has no automated cluster-collapse
  test — only manual verification. Add one `clusters.rs` test
  asserting two slug shapes from the research now bucket together.

- 🔵 **Safety**: Active dev-server sessions will 404 on the
  moment-of-deploy URL change; confirm `LifecycleClusterView`'s 404
  affordance is friendly during Phase 4 manual verification.

- 🔵 **Safety**: Resolver snapshot is taken outside the lock; add a
  concurrency-parity test for `apply_cluster_key_backfill` mirroring
  the existing
  `refresh_one_target_migration_is_atomic_under_single_writer_lock`.

- 🔵 **Performance**: `entries_by_path` snapshot requires a second
  full deep-clone; pass a `HashMap<PathBuf, &IndexEntry>` over the
  borrowed slice instead.

- 🔵 **Performance**: `canonicalise_one` allocates Vec + HashSet per
  call inside the walk; extract a single-string canonicaliser.

- 🔵 **Performance**: Watcher does a full rescan on every fs event;
  worth flagging in the Performance Considerations section so the
  framing isn't "O(N·depth) extra" when it's actually layered on a
  full rescan.

#### Suggestions

- 🔵 **Architecture**: The Migration Notes claim that old URLs "404
  exactly as today" is misleading — today they resolve. Revise the
  prose to acknowledge previously-resolving URLs will start 404-ing,
  or add the alias proposed above.

- 🔵 **Code Quality**: Frontend `clusterVia` label logic — expose the
  walked-chain reason from the server (`clusterKeyReason:
  'parent-work-item' | 'target-plan-parent' | 'target-work-item' |
  'slug-fallback'`) rather than re-deriving the reason client-side, or
  put the derivation in a single pure function with table-driven
  tests.

- 🔵 **Compatibility**: Note in the plan that a repo-wide audit
  confirms `compute_clusters_with_backfill` has no external callers
  (saves the next reader the grep).

- 🔵 **Performance**: Two-pass bucketing in
  `compute_clusters_with_backfill_typed` does two `PathBuf` HashMap
  lookups per entry; fold into a single pass or thread the cluster_key
  result through.

- 🔵 **Performance**: `MAX_DEPTH = 4` is essentially free to bump to
  6–8; pre-emptively widen during the messy migration window.

- 🔵 **Performance**: Wire-shape growth is negligible — the plan's
  ~6 KB estimate is accurate. No action.

### Strengths

- ✅ Five-phase decomposition with each phase independently shippable
  matches the codebase's incremental refactor style and keeps blast
  radius small.
- ✅ Test-first ordering with concrete, table-driven test bodies
  mirrors the existing slug.rs / clusters.rs style; new test names
  (e.g. `dated_types_strip_optional_work_item_id_after_date`) are
  self-documenting.
- ✅ Phase 2's `parse_typed_ref` is a clear DRY win, removing ad-hoc
  `strip_prefix` parsing that's currently duplicated between
  `frontmatter::read_ref_keys` and `target_path_from_entry`.
- ✅ Cluster_key is layered alongside slug rather than replacing it,
  preserving per-file URL identity (`IndexEntry.slug`) and the
  open-closed principle: legacy filename-only entries continue to
  cluster via the slug fallback.
- ✅ Reuses existing infrastructure (`work_item_by_id`, `plans_by_id`,
  `canonicalise_refs`) instead of introducing a parallel identity
  scheme; functional-core / imperative-shell separation is respected.
- ✅ Phase 1 explicitly tests empty-descriptive-tail boundaries
  (`2026-05-31-0040-.md` and `2026-05-31-0040.md` → None) and the
  internal `-review-` preservation case.
- ✅ Concurrency model is inherited, not invented:
  `apply_cluster_key_backfill` is documented to mirror
  `apply_completeness_backfill`, which is already serialised against
  rescan via `rescan_lock`.
- ✅ Bounded recursion (MAX_DEPTH) plus an explicit cycle-detection
  test addresses the obvious unbounded-recursion smell up front.
- ✅ Wire-shape change is strictly additive: `clusterKey: string |
  null` via the existing `#[serde(rename_all = "camelCase")]`. Frontend
  parses via plain TypeScript casts (no zod), so older clients ignore
  the new field cleanly.
- ✅ Plan explicitly enumerates what it is NOT doing (no corpus
  rewrite, no graph-rendering, no strategy switch, no clippy
  enforcement) — clear YAGNI discipline.
- ✅ URL-breakage tradeoff is consciously made and documented (Migration
  Notes), not a silent regression — even if the framing of today's
  behaviour is imprecise.
- ✅ Performance Considerations section is quantitative (O(N) with
  bounded depth, ~3 KB memory, no extra invocations) — even if it
  understates allocation cost.

### Recommended Changes

1. **Update `related::resolve_related` to look up inferred-cluster
   siblings via `cluster_key` with fallback to slug equality** (addresses:
   critical inferred_cluster finding)
   Add this to Phase 4's automated and manual success criteria. The
   `/api/related` ↔ `/api/lifecycle` consistency invariant is part of the
   same change wave that introduces the divergence.

2. **Remove the back-compat shim or gate it explicitly** (addresses:
   shim cross-cutting theme)
   Either delete `compute_clusters_with_backfill` and update existing
   slug-only tests to pass empty maps explicitly to the typed variant,
   or rename it `compute_clusters_with_backfill_slug_only` and gate
   behind `#[cfg(test)]`. Update all three production call sites
   (`watcher.rs:154`, `api/docs.rs:243`, `server.rs:91`) and the
   indexer's main clustering call site unconditionally.

3. **Tighten `parse_typed_ref` and consolidate target resolution**
   (addresses: parse_typed_ref + duplication themes)
   - Reject empty suffixes (`work-item:` → `None`)
   - Fall through to `TypedRef::Path` when a typed-prefixed value
     contains `/` or ends in `.md`
   - Preserve `normalize_target_key` path validation — either inside
     the parser or document explicitly that callers must apply it
   - Have `cluster_key::walk` delegate to `target_path_from_entry`
     plus an `entries_by_path` lookup rather than re-implementing the
     dispatch

4. **Fix or drop the `PROJ-NNNN-` slug-strip claim** (addresses:
   project-prefix theme)
   Either plumb `&WorkItemConfig` into `slug::derive` (the
   `WorkItems` branch already does) and extend
   `strip_optional_work_item_id_prefix` to accept a project-prefixed
   shape, or remove the `PROJ-NNNN-` line from Phase 1's Overview.
   Add Phase 1 + Phase 4 test cases for project-prefixed workspaces
   either way.

5. **Constrain `strip_optional_work_item_id_prefix` to the configured
   ID width** (addresses: digit-only mis-strip correctness finding)
   Use `cfg.extract_id` or a width-aware regex so filenames like
   `2026-04-17-100-day-plan.md` don't lose their first descriptor
   token.

6. **Document Phase 3 as an explicit wire-shape behaviour change**
   (addresses: Phase 3 contract-change theme)
   Add a "Wire contract change" subsection to Phase 3 enumerating
   that `/api/related/{plan-path}.declaredInbound` may include
   WorkItemReviews / Validations / PrReviews, list the frontend
   fixtures that need refreshing, and add a Phase 3 manual-verification
   step that confirms the new declared-inbound rows are correctly
   rendered.

7. **Add a one-release URL alias** (addresses: URL stability theme)
   In `/api/lifecycle/:slug` (and the SPA router), on cluster miss
   look up the slug against `IndexEntry.slug` values and 301/200 to
   the canonical cluster. Note the deprecation in a UI banner or
   `Deprecation` header. Remove after epic-0057 migration completes.

8. **Specify Phase 4 integration tests fully and add a boundary
   test for `MAX_DEPTH`** (addresses: test coverage themes)
   - Spell out the `run_typed` helper signature (derive snapshots
     from the entry list automatically)
   - Give each integration test full assertion bodies (cluster count,
     slug, cluster_key, member types, per-entry cluster_key backfill)
   - Split the depth-limit test: one chain at exactly `MAX_DEPTH - 1`
     resolves, one chain or cycle at `MAX_DEPTH + 1` returns `None`
   - Add at least one `tests/api_lifecycle.rs` integration test that
     uses the existing `meta/reviews/work/` fixture to assert
     end-to-end clustering

9. **Add observability for the depth limit and silent canonicalisation
   failures** (addresses: depth-limit and migration-debuggability
   themes)
   Emit `tracing::warn!` when `MAX_DEPTH` is hit (with entry path),
   and `tracing::debug!` when an empty typed-prefix value is
   encountered. Bump `MAX_DEPTH` to 6–8 pre-emptively (free).

10. **Specify the cluster representative slug picker's fallback for
    `slug: None`** (addresses: minor correctness finding)
    Document and test: WorkItems slug → any entry's slug →
    cluster_key string itself. Required for any LifecycleCluster.slug
    to remain `String` rather than `Option<String>`.

11. **Update the Performance Considerations section to be honest about
    allocations** (addresses: performance findings)
    Note the `entries_by_path` snapshot cost, the
    `canonicalise_one`-per-walk-hop allocations, and the fact that
    the watcher does a full rescan per fs event. Optional: switch to
    borrowed snapshots and a single-string canonicaliser.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured, follows the established
functional-core pattern, and respects existing module boundaries (slug,
frontmatter, indexer, clusters, related). The phased shipping discipline
and the use of existing secondary indexes (work_item_by_id, plans_by_id)
limits architectural blast radius. However, the design introduces two
parallel notions of cluster identity (cluster_key and slug) without
re-aligning downstream consumers — most notably related.rs::resolve_related
still matches inferred-cluster siblings via `c.slug == entry.slug`, which
silently breaks under the new cluster.slug picker. The cluster_key
resolver also reaches into the indexer's primary entries map from a new
module and duplicates parsing already centralised in
target_path_from_entry.

**Findings**: 1 critical (`inferred_cluster` lookup breaks), 3 major
(shim foot-gun, duplicated target resolution, Phase 3 reviews_by_target
contract change), 4 minor (MAX_DEPTH silent truncation, orphan-by-design
slug fallback, module placement, URL stability framing).

### Code Quality

**Summary**: The plan introduces sensible new modules (`typed_ref`,
`cluster_key`) with focused responsibilities and a clear phasing strategy
that keeps each change independently shippable. However, the prescribed
`parse_typed_ref` silently drops critical security validation (the
`..`/absolute/NUL/backslash rejection in `normalize_target_key`), the
`compute_clusters_with_backfill_typed` design grows a 6-arg signature
with a leaky 'shim with empty maps' pattern, and the `cluster_key`
module duplicates concerns (`id_from_value` vs `canonicalise_one`,
ad-hoc path-shape parsing) in ways that will hurt maintainability.

**Findings**: 3 major (parse_typed_ref drops path validation, 6-arg
signature, three overlapping id helpers), 3 minor (helper naming,
path-detection heuristic, DocTypeKey duplication, snapshot coherence),
1 suggestion (frontend label derivation).

### Test Coverage

**Summary**: The plan is heavily test-first and prescribes focused unit
tests at each module boundary, with explicit before/after assertions and
a strong regression-preservation posture. Coverage is broadly
proportional to risk. However, several edge cases that the plan itself
identifies as failure modes are not concretely covered by the prescribed
tests, the Phase 4 integration tests are sketched as fragments rather
than full bodies, and there is no end-to-end integration test that uses
the existing `tests/fixtures/meta/reviews/work/` corpus to prove the
lifecycle pipeline actually collapses on the wire.

**Findings**: 4 major (sketched integration tests, depth-limit test
conflation, missing project-prefix tests, no end-to-end integration
test on motivating corpus), 6 minor (PROJ-prefix unpinned, parser
boundary cases, work-item:NNNN routing test, shim equivalence test,
representative slug negative test, debug-tag derivation), 1 low
(Phase 1 cluster-collapse test).

### Correctness

**Summary**: The plan's overall logic is sound and the phased approach
reduces risk well, but several concrete correctness issues exist in the
slug helper, the typed-ref parser, and the cluster-key resolver. The
most consequential are: (1) the new
`strip_optional_work_item_id_prefix` will mis-strip legitimate
descriptor heads that happen to be all-digit (e.g. a four-digit token),
(2) the prose claim about stripping `PROJ-NNNN-` is not implemented by
the helper as written, and (3) `parse_typed_ref` misclassifies
path-shaped values that carry a `plan:` prefix as Plan IDs with no path
fallback, which silently drops resolution.

**Findings**: 3 major (digit-head mis-strip, PROJ-NNNN- not
implemented, plan:<path> misclassified), 6 minor (cluster slug picker
with None, path-shape extract_id strictness, MAX_DEPTH silent
truncation, -review-N internal edge case, shim correctness coupling
to test defaults, empty-id silent failure, read_ref_keys post-refactor
contract).

### Compatibility

**Summary**: The plan is dominated by additive, internal-consumer-only
changes — a new `clusterKey` JSON field, a new resolver, and refinements
to slug derivation — and the frontend parses `/api/lifecycle` via plain
TypeScript casts with no zod/runtime schema, so older clients tolerate
the new field cleanly. The main compatibility concerns are (1) the
deliberate URL-contract change at `/lifecycle/<slug>` where existing
bookmarks 404 with no redirect or alias, (2) a generalisation of
`target_path_from_entry` that changes `/api/related/*` payloads, and
(3) React Query cache entries keyed on the changing cluster
representative slug, which will leave stale entries across deploys.

**Findings**: 2 major (URL contract change, Phase 3 related payload
shape change), 3 minor (React Query stale cache, clusterKey on every
IndexEntry, parser empty-id), 1 suggestion (no external callers audit).

### Safety

**Summary**: From a safety lens this plan is well-scoped: the
visualiser is a read-only dev-time tool, the change is in-memory only,
and the existing rescan_lock + single-writer-lock invariant the plan
inherits from apply_completeness_backfill is a sound pattern to mirror.
The blast radius of failure is bounded — at worst a developer sees
wrong clusters until they `git restore` the crate. The remaining safety
concerns are operational papercuts (silent depth truncation, silent
shim fallback, transient inconsistency across phased rollout) rather
than data-loss risks.

**Findings**: 5 minor (silent depth truncation, shim silent fallback,
Phase 3→4 transient inconsistency, active session 404, resolver
snapshot race).

### Performance

**Summary**: At the stated scale (~200 entries, low-frequency cluster
recomputes triggered by file saves), the plan's performance profile is
dominated by an existing pattern — `compute_clusters_with_backfill`
already runs after every rescan/refresh_one and already deep-clones
every entry. The new typed walk adds bounded O(N·depth) work with
depth ≤ 4 plus per-walk-hop allocations, which is negligible at corpus
sizes ≤ ~500. The main genuine concern is that the plan understates
allocation cost: the resolver requires a new `HashMap<PathBuf,
IndexEntry>` snapshot (~200 deep clones of rich frontmatter
`serde_json::Value` blobs) and calls `canonicalise_one` (Vec + HashSet
allocation) per typed reference.

**Findings**: 3 minor (second deep-clone, canonicalise_one allocations,
watcher full-rescan framing), 3 suggestions (two-pass bucketing,
MAX_DEPTH bump, wire-shape growth confirmation).

## Re-Review (Pass 2) — 2026-06-02T00:45:15+00:00

**Verdict:** APPROVE

The first round of edits resolved every prior critical and major
finding (related.rs cluster_key lookup, shim removal, target-resolution
delegation, Phase 3 wire-shape callout, URL stability honesty, depth
limit bump, fully-specified tests, project-prefix support). The
re-review surfaced **two new criticals** and **three new majors** that
were inadvertently introduced by the edits — all factually verified
against the live codebase (`extract_id`'s trailing-hyphen requirement,
`normalize_target_key`'s argument order, the resolver signature
contradicting `ClusterContext`'s borrowed-entry map, non-existent test
helpers, an e2e fixture lacking the required `target:` frontmatter).
A second patch round addressed all of them. The plan is now sound.

### Previously Identified Issues

- 🔴 **Architecture** (Pass 1): `inferred_cluster` lookup in related.rs
  silently breaks — **Resolved** via the explicit `cluster_key`-first
  lookup in `related::resolve_related` with slug fallback.
- 🟡 **Architecture / Code Quality / Safety / Correctness** (Pass 1):
  Back-compat shim foot-gun — **Resolved**; shim removed, all three
  production call sites updated, `ClusterContext` introduced.
- 🟡 **Architecture / Code Quality** (Pass 1): Duplicated target-resolution
  logic — **Resolved**; `cluster_key::walk` delegates to
  `target_path_from_entry` for review/validation branches.
- 🟡 **Architecture / Compatibility / Safety** (Pass 1): Phase 3
  declared-inbound contract change unannounced — **Resolved**; "Wire
  contract change" subsection enumerates affected endpoints and four
  frontend fixture files.
- 🟡 **Code Quality / Correctness** (Pass 1): `parse_typed_ref` drops
  path validation and misclassifies `plan:<path>` — **Resolved**;
  parser rejects empty suffixes, falls through to `TypedRef::Path`
  for hybrid shapes, and `target_path_from_entry` threads through
  `normalize_target_key`.
- 🟡 **Correctness / Test Coverage** (Pass 1): `PROJ-NNNN-` claim
  unimplemented — **Resolved** via `is_canonical_id_token` and
  project-prefixed test cases.
- 🟡 **Correctness** (Pass 1): Digit-only descriptor head mis-stripped
  — **Resolved** via the width-aware `is_canonical_id_token` predicate
  (not the prior broken `extract_id` probe).
- 🟡 **Compatibility / Safety** (Pass 1): URL contract change framing
  — **Resolved**; Migration Notes explicitly call out that
  previously-resolving URLs will start 404-ing.
- 🟡 **Test Coverage** (Pass 1): All four flagged gaps (sketched
  integration tests, depth-limit conflation, missing project-prefix
  tests, no motivating-corpus integration test) — **Resolved**;
  test bodies fully specified, depth split into 3 cases (positive,
  cycle, upper bound), project-prefix tests added, e2e test added.
- 🔵 All Pass 1 minor findings — substantially addressed (slug picker
  fallback chain specified, parser empty-id tests added, snapshot
  coherence via `ClusterContext::from_entries`, perf section honesty,
  representative slug negative test, etc.)

### New Issues Introduced (by Pass 1 edits)

- 🔴 **Correctness** (Pass 2): `strip_optional_work_item_id_prefix`
  probed `extract_id(&format!("{head}.md"))` but the regex requires a
  trailing `-`, so the helper strips nothing — **Fixed in Pass 2** by
  adding `WorkItemConfig::is_canonical_id_token` (width-aware via
  `id_pattern`'s `{number:0Nd}` specifier) and routing the helper
  through it.
- 🔴 **Correctness** (Pass 2): `normalize_target_key(project_root,
  raw_str)` had the args swapped; real signature is
  `normalize_target_key(raw, project_root)` — **Fixed in Pass 2** by
  swapping the call + adding a positive resolution test
  (`path_target_resolves_against_supplied_project_root`) that pins
  the order behaviourally.
- 🟡 **Architecture / Code Quality / Correctness / Performance**
  (Pass 2): Resolver signature took `&HashMap<PathBuf, IndexEntry>`
  (owned) while `ClusterContext` held `HashMap<PathBuf,
  &'a IndexEntry>` (borrowed) — would have not compiled or forced a
  re-clone — **Fixed in Pass 2** by making the resolver signature
  consistently borrowed with explicit `'a` lifetime and a deref
  comment on the recursion site.
- 🟡 **Test Coverage / Correctness** (Pass 2): Tests called
  `WorkItemConfig::default()`, `with_pattern_for_test`, and
  `entry_for_test_with_filename` which don't exist — **Fixed in Pass 2**
  by adding a "Test-support helpers required by Phases 1 + 4"
  subsection prescribing each helper.
- 🟡 **Test Coverage** (Pass 2): The e2e test referenced helpers
  (`test_fixture_root`, `start_test_server`) that don't exist AND
  asserted against the existing `ac2-coverage-review-1.md` fixture
  which lacks the `target:` frontmatter — **Fixed in Pass 2** by
  rewriting the test against `tempfile::tempdir()` + inline
  `std::fs::write` of a seeded fixture, matching the codebase's
  existing test conventions.
- 🔵 **Code Quality** (Pass 2): `ClusterContext::empty()` used a
  brittle `OnceLock`+static-lifetime trick — **Fixed in Pass 2** by
  replacing with `EmptyClusterFixture` (caller-owned storage,
  conventional Rust borrow-shaped-context idiom).
- 🔵 **Compatibility** (Pass 2): `/api/related/{path}.inferredCluster`
  membership change introduced by Phase 4's related.rs update was
  not enumerated in the Phase 3 Wire-contract-change callout —
  **Fixed in Pass 2** by adding a "Phase 4 wire-change note" listing
  the three frontend test files that assert on `inferredCluster`.
- 🔵 Other Pass 2 minor findings (DocTypeKey vocabulary duplication
  in match arms, orphan-type slug fallback collision, wire-shape
  test placeholders, Phase 5 debug-tag fixture sketches,
  concurrency-parity test under-specified, React-Query cache
  staleness, MAX_DEPTH warn-log message wording) — **Not addressed
  in Pass 2; deferred** as residual non-blocking refinements.

### Assessment

The plan is in good shape to ship to the implementer. The two
critical Pass 2 issues — both factual signature mismatches against
the live codebase — were exactly the class of subtle bug that a fresh
re-review is best placed to catch; they would have failed at the
compile or unit-test step rather than shipping silently, but the
implementer would have wasted time reverse-engineering the intent.
With Pass 2 in place every prescribed code snippet is consistent
with the real APIs (`extract_id`'s regex requirements, the actual
`normalize_target_key` signature, the existing test conventions in
`tests/api_lifecycle.rs`, and the `default_numeric()` constructor),
and the new test-helper subsection makes the test prerequisites
explicit.

Residual minor concerns are deliberate trade-offs or polish items
that a follow-up can address without disturbing the architecture:

- Orphan-by-design types can still collision-merge via the slug
  fallback (rare in practice; documented behaviour).
- DocTypeKey vocabulary lists in `target_path_from_entry` and
  `cluster_key::walk` remain duplicated (a `DocTypeKey::carries_target`
  helper would consolidate; small refactor).
- The MAX_DEPTH warn-log message could be tightened to mention the
  slug-less drop case (impossible in today's vocabulary).
- React-Query cache staleness across deploy is not addressed (a
  hard-refresh note in Migration Notes or a query-key version bump
  would suffice).
- Phase 5 wire-shape and debug-tag tests still carry placeholder
  fixture sketches — fine as-is for Phase 5's optional scope.

Recommend approval. Ship to implementation.

### Pass 3 — Residual fixes (2026-06-02)

All seven residual minor concerns listed above were patched in a
follow-up edit round. Verdict remains APPROVE.

- ✅ **Orphan-by-design slug collision** — Gated. Added
  `DocTypeKey::participates_in_lifecycle()` predicate; the bucketing
  pass routes orphan types (Notes, Decisions, DesignGaps,
  DesignInventories) to per-path orphan buckets (`__orphan__::<path>`)
  so they cannot accidentally slug-merge. New regression test
  `orphan_types_with_colliding_slugs_do_not_merge` pins the gate;
  counterpart `lifecycle_type_with_no_linkage_still_slug_merges_with_work_item`
  pins that lifecycle types preserve the legacy fallback.
- ✅ **DocTypeKey vocabulary duplication** — Consolidated. Added
  `DocTypeKey::carries_target_frontmatter()` predicate;
  `target_path_from_entry` dispatches through it. `cluster_key::walk`
  keeps an explicit variant list for compiler-enforced exhaustiveness
  and a new alignment test
  (`cluster_key_target_arm_matches_carries_target_predicate`) locks
  the two against drift.
- ✅ **MAX_DEPTH warn-log message** — Tightened to include
  `entry_slug` and message text covering the slug-less drop case
  (`"... fall back to slug bucket if a slug is present, otherwise be
  excluded from clustering"`).
- ✅ **React-Query cache staleness** — Migration Notes paragraph
  added; Phase 5 bumps cluster query keys to a `'v2'` segment
  (`['lifecycle-clusters', 'v2']`, `['lifecycle-cluster', 'v2',
  slug]`) so post-deploy clients miss the old cache.
- ✅ **Phase 5 wire-shape tests** — Placeholders replaced with
  concrete setup via `run_clusters` for both positive (`Some("0042")`)
  and negative (`None` → JSON `null`) cases; defensive assertion
  added that the field key remains present when the value is null.
- ✅ **Phase 5 debug-tag tests** — Label-selection extracted as a
  pure function `clusterViaLabel(entry, cluster)`; table-driven
  branch coverage lives in `cluster-via-label.test.ts` with 8
  fixture cases. The TSX test narrows to a single wiring smoke test.
- ✅ **Phase 4 concurrency-parity test** — Spelled out with full
  body mirroring `refresh_one_target_migration_is_atomic_under_single_writer_lock`,
  including the new `PostClusterKeyUpdateHook` barrier and a
  reader/writer rendezvous proving torn reads are impossible.
- ✅ **Case 17 (parent with no matching work_item_by_id)** — Spelled
  out with a full test body that uses an empty `work_item_by_id`
  map, pinning that `cluster_key` is a logical id not a path-lookup
  result. A defensive `contains_key` implementation would fail this
  test.
- ✅ **Write-path consolidation** — Bucketing no longer clones and
  mutates `entry.cluster_key` per bucket. Buckets hold `&IndexEntry`
  refs; the cluster builder reads `cluster_key` from
  `cluster_key_by_path` at build time. `apply_cluster_key_backfill`
  is the sole writer to `IndexEntry.cluster_key`, eliminating the
  prior dual-write-path drift risk.

The plan is ready to hand to the implementer with no known
outstanding issues.
