---
type: "plan-review"
id: "2026-11-24-PP-994-generic-set-model-indexing-and-api-review-1"
title: "Plan Review: Generic Set Model: Indexing and API Implementation Plan"
date: "2026-11-24T09:53:43+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-24-PP-994-generic-set-model-indexing-and-api"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "performance", "standards", "security"]
review_number: 1
review_pass: 4
tags: ["visualiser", "set-model"]
last_updated: "2026-11-24T10:57:45+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Generic Set Model: Indexing and API Implementation Plan

**Verdict:** REVISE

The plan models sets and declared doc types well: typed rule enums, a
pure `SetShape::classify`, characterisation-pinned refactor phases, and a
careful PP-996 ordering table. Its weaknesses cluster around four places:
the overlap refusal's placement in a CLI-wide resolver, the opt-out
treatment of members in the shared index, watcher behaviour (concurrent
rescans, widened admission, a live activity feed the server roll-up does
not reach), and the per-set serving boundary on `/api/docs`. Several
tests as written could not fail.

### Cross-Cutting Themes

- **Overlap refusal reaches every `doc_type_dirs` caller** (flagged by:
  architecture, compatibility, correctness, security) — the migrate
  adapter calls it with `unwrap_or_default()`, so an overlapping config
  would make migrations silently run over nothing; other CLIs would start
  failing hard.
- **Members share the flat index and every consumer opts out** (flagged
  by: architecture, code-quality) — `!is_member()` filters plus a
  `slug: None` sentinel; a forgotten call site silently counts members.
- **Duplicate set enumeration paths** (flagged by: architecture,
  code-quality, performance, correctness) — `list` and `list_sets` walk
  the same trees; member `setId` depended on root-first ordering.
- **Watcher widening** (flagged by: performance, security, correctness) —
  every non-dot path under a set root triggered a full rescan; dot
  markdown slipped through the first clause; concurrent tasks could
  double-emit.
- **Text-scanning boundary tests are bypassable** (flagged by:
  architecture, code-quality, test-coverage).
- **Double percent-decoding of `setId`** (flagged by: compatibility,
  correctness, security).

### Tradeoff Analysis

- **Compatibility vs simplicity of the overlap guard**: one check in the
  shared resolver is simplest, but changes behaviour for every CLI and
  silently for migrations. Escalated.

### Findings

#### Major

- 🟡 **Architecture / Compatibility / Correctness**: Overlap refusal in
  `doc_type_dirs` breaks CLI-wide consumers, silently in migrations
  **Location**: Phase 2: The refusal
  `migrate-adapters/src/context.rs` swallows the error; `corpus-cli` and
  `work-cli` also call the resolver.
- 🟡 **Architecture / Code Quality**: Members share the flat index; every
  consumer must filter them out, and `slug: None` carries hidden meaning
  **Location**: Phase 7: Membership on records / Consumer rules
- 🟡 **Compatibility**: Live frontend activity feed builds rows from raw
  SSE events, bypassing the server roll-up
  **Location**: Phase 10: Activity roll-up
- 🟡 **Compatibility**: Member search rows reuse the set's slug,
  colliding with the frontend's `(docType, slug)` row key
  **Location**: Phase 7: Consumer rules — `api/search.rs`
- 🟡 **Correctness**: Concurrent per-path rescans break the "exactly one
  event" guarantee
  **Location**: Phase 10: Watcher
- 🟡 **Correctness**: Removal ledger and case-only pairing computed in
  different places
  **Location**: Phase 10: Removal ledger
- 🟡 **Performance**: Widened watcher admission causes one full rescan
  per unslotted file event
  **Location**: Phase 10: Watcher
- 🟡 **Security**: Set-root restriction on `/api/docs` bypassable unless
  decided on the canonical path
  **Location**: Phase 8: Document endpoint
- 🟡 **Security**: Recursive set walk and `FileDriver::stat` bypass the
  canonicalise-and-allow boundary
  **Location**: Phase 8: Enumeration
- 🟡 **Code Quality**: Cluster recompute triplication only partly
  consolidated
  **Location**: Phase 7: Consumer rules (Clusters)
- 🟡 **Code Quality**: `SetMembership.place` allows invalid states
  **Location**: Phase 7 / Phase 8
- 🟡 **Architecture**: Boundary tests miss closed-world type knowledge
  (`TypedRef`) and the `corpus-cli` exception
  **Location**: Phase 4: Boundary test
- 🟡 **Test Coverage**: Cluster-key rules, anchoring, listing rules and
  library order not characterised
  **Location**: Phase 4: Characterisation tests
- 🟡 **Test Coverage**: Single-path refresh of a member untested
  **Location**: Phase 8: Index model
- 🟡 **Test Coverage**: Exact-key and inferred-item exclusion tests may
  never fail
  **Location**: Phase 7: Failing tests first
- 🟡 **Test Coverage**: Fabricated records never checked against real
  enumeration; no real-member related test
  **Location**: Phase 7
- 🟡 **Standards**: Member API `path` departs from the wire meaning of
  `path`/`relPath`
  **Location**: Phase 1: Member API specification
- 🟡 **Standards**: `corpus::DocType` collides with the server's
  `doc_type_view::DocType`
  **Location**: Phase 3: The declaration

#### Minor

- 🔵 **Correctness**: Expected overlap pair for `research_codebase` is
  not the first pair found (`research_issues` comes first)
  **Location**: Phase 2: Failing tests first
- 🔵 **Correctness**: Set latest time did not include assets or asset
  removals — **Location**: Phase 7 / Phase 8
- 🔵 **Correctness**: `should_rescan`'s first clause admits dot markdown
  under set roots — **Location**: Phase 10: Watcher
- 🔵 **Correctness / Architecture**: `root_actions` unbounded —
  **Location**: Phase 10: Activity roll-up
- 🔵 **Correctness / Architecture**: Invalid slot patterns and duplicate
  wire tokens not rejected at declaration time — **Location**: Phase 3 /
  Phase 6
- 🔵 **Correctness**: Members' `setId` depends on root-first ordering —
  **Location**: Phase 8
- 🔵 **Compatibility / Correctness**: Double percent-decoding of `setId`
  — **Location**: Phase 9: Handler
- 🔵 **Compatibility**: Asset `doc-changed` events widen the SSE
  contract — **Location**: Phase 10: Watcher
- 🔵 **Security**: Overlap refusal compares strings, so symlink aliases
  evade it — **Location**: Phase 2
- 🔵 **Performance**: Per-row root-slug lookup in search; compute latest
  time at rescan; avoid two walks — **Location**: Phases 7–8
- 🔵 **Code Quality**: `FLAT_DEFAULTS` weakens completeness; wide flat
  `DocType`; generic `ConfigError::Invalid`; tuple-with-bool set
  context; anonymous `link_keys_of` tuple — **Location**: Phases 2–10
- 🔵 **Architecture**: Topic-research shape duplicated in `corpus`
  without a contract test — **Location**: Phase 6
- 🔵 **Test Coverage**: Member API tie-break, decoding and order not
  pinned; overlap order/normalisation; harness split; ledger tests;
  scanner self-tests — **Location**: Phases 2, 9, 10
- 🔵 **Standards**: `--filter` expressions select no tests; CHANGELOG
  steps missing; public-api snapshot steps; spec frontmatter validation
  — **Location**: Phases 1–9

#### Suggestions

- 🔵 Unencoded member paths need an encoding contract (security);
  watcher coalescing (security, performance); member API scans
  (performance); `ptr::eq` fast path (performance); hedges to settle
  (code-quality); set declaration in one place and `None` variant names
  (code-quality, architecture); terminology drift, comment-inviting
  wording, route syntax (standards); in-memory state lifecycle
  (architecture); small untested changes (test-coverage).

### Strengths

- ✅ Per-variant `match` arms become named domain enums, not strings.
- ✅ Phases 3–5 are pure refactors pinned by a literal characterisation
  table, and the plan is honest about the XL size and proposes a split.
- ✅ `SetShape::classify` is a pure, isolated domain function; extension
  case rules are pinned in both directions.
- ✅ The member API reserves `/derived` and `sources` without registering
  them, and the PP-996 table resolves either landing order.
- ✅ Slot patterns reject `..`, `**` and leading `/`, and `setId` never
  reaches the filesystem.
- ✅ Assets are stat'ed, never read; the activity roll-up stays O(50).
- ✅ Criteria are cited next to the tests that prove them.

### Recommended Changes

1. **Decide where the overlap refusal applies** (escalated).
2. **Decide the index's root/member interface** (escalated).
3. **Decide how the live activity feed rolls up** (escalated).
4. Applied in place: canonical-path serving decision and symlink-safe
   walk; atomic `rescan_with_diff`; slot-aware watcher admission;
   single-walk `list_sets` with root-first `SetContext`; one pairing
   function feeding both emit and ledger; typed `SetDocument`/`SetContext`
   and `LinkKeys`; committed `recompute_clusters`; catalogue validation;
   `DocTypeView` rename; `setRelativePath` in the member API;
   strengthened tests; filter, CHANGELOG and public-api steps.

## Per-Lens Results

### Architecture

**Summary**: Structurally ambitious and mostly sound; weakest in the
overlap guard's reach, opt-out member filtering, and a syntactic-only
boundary.

**Findings**:
- major/high — Phase 2: overlap refusal reaches more consumers; migrate
  swallows it.
- major/medium — Phase 7: members share the flat index; opt-out filtering.
- major/medium — Phase 4/10: boundary tests miss `TypedRef` and
  work-item helpers; allow-list omits `corpus-cli/src/main.rs`.
- minor/medium — Phase 8: enumeration split between `list` and
  `list_sets`.
- minor/medium — Phase 5/3: injected types bypass config resolution;
  catalogue has no uniqueness invariant.
- minor/medium — Phase 6: topic-research shape duplicated without a
  contract test.
- minor/low — Phase 6/ADR-0078: set declaration in two places.
- suggestion/low — Phase 10: in-memory state lifecycle unstated.

### Code Quality

**Summary**: Good domain modelling; risks in members sharing the index,
partial cluster consolidation, invalid-state types and deferred hedges.

**Findings**:
- major/high — Phase 7: root-vs-member rule spread across consumers.
- major/medium — Phase 7: `slug: None` carries hidden meaning.
- major/high — Phase 7: cluster recompute only partly consolidated.
- major/medium — Phases 6–8: `SetMembership.place` allows invalid states.
- minor/medium — Phase 5: `FLAT_DEFAULTS` weakens completeness.
- minor/medium — Phases 3–4: `DocType` is a wide flat record.
- minor/medium — Phases 4/10: two fragile text scanners.
- minor/medium — Phase 8: two enumeration paths.
- minor/high — Phase 2: generic `ConfigError::Invalid`.
- minor/medium — Phase 10: set context as a tuple with a bool.
- minor/medium — Phase 7: `link_keys_of` tuple plus snapshot lookup.
- suggestion/medium — Phases 5/7/8: "if X, otherwise Y" hedges.
- suggestion/low — ADR-0078/Phase 6: set declaration described
  inconsistently; `None` variant names.

### Test Coverage

**Summary**: Test-first with criteria mapped; weakest where risk is
highest.

**Findings**:
- major/high — Phase 4: cluster, anchor, listing and library order
  uncharacterised.
- major/high — Phase 8: single-path member refresh untested.
- major/medium — Phase 7: exclusion tests may never fail.
- major/medium — Phase 7: fabricated records unchecked; no real-member
  related tests.
- minor/high — Phase 9: tie-break, decoding and order unpinned.
- minor/medium — Phase 2: iteration order and normalisation.
- minor/medium — Phase 10: watcher tests in a harness with no router.
- minor/medium — Phase 10: ledger clearing, pairing and clock untested.
- minor/medium — Phases 4/10: boundary scanners bypassable.
- suggestion/medium — small behaviour changes untested.

### Correctness

**Summary**: Core logic holds; several guarantees depend on assumptions
the code does not meet.

**Findings**:
- major/high — Phase 10: concurrent per-path rescans double-emit.
- major/high — Phase 2: overlap refusal empties migrations' directory set.
- minor/high — Phase 2: expected overlap pair is `research_issues`.
- major/medium — Phase 10: ledger and pairing computed in different places.
- minor/medium — Phases 7–8: latest time omits assets.
- minor/high — Phase 10: `should_rescan` admits dot markdown.
- minor/medium — Phase 10: `root_actions` unbounded.
- minor/medium — Phases 3/6: invalid patterns and duplicate tokens.
- minor/medium — Phase 8: member `setId` depends on root order.
- suggestion/low — Phase 9: double percent-decoding.

### Compatibility

**Summary**: Wire changes are mostly additive; risks in the shared
resolver and frontend consumers assuming one path per document.

**Findings**:
- major/high — Phase 2: overlap refusal breaks CLI-wide consumers.
- major/high — Phase 10: live `ActivityFeed` builds rows from SSE.
- major/high — Phase 7: search row key collision.
- minor/medium — Phases 1/9: double percent-decoding.
- minor/medium — Phase 10: asset `doc-changed` events widen the contract.

### Performance

**Summary**: Bounded algorithmic work; real cost is in widened watcher
admission.

**Findings**:
- major/high — Phase 10: one full rescan per unslotted file event.
- minor/medium — Phase 7: per-row root-slug lookup in search.
- minor/medium — Phases 7/10: compute latest time at rescan.
- minor/medium — Phase 8: two walks and serial stats.
- suggestion/low — Phase 9: member API lookups scan maps.
- suggestion/low — Phase 3: string equality and hashing.

### Standards

**Summary**: Follows conventions; naming collisions and verification
commands need fixing.

**Findings**:
- major/high — Phase 1: member API `path` naming.
- major/high — Phase 3: `DocType` name collision.
- minor/high — Phases 3/9: `--filter` expressions select no tests.
- minor/medium — Phases 2/8/9: CHANGELOG steps.
- minor/medium — Phases 3/4/6: public-api snapshot steps.
- minor/medium — Phase 1: spec frontmatter validation.
- suggestion/medium — terminology drift.
- suggestion/low — comment-inviting wording in Phase 5.
- suggestion/low — route syntax.

### Security

**Summary**: Mostly read-only and local; the per-set serving and
enumeration boundaries need canonical-path and symlink rules.

**Findings**:
- major/medium — Phase 8: `/api/docs` set-root decision on lexical path.
- major/medium — Phase 8: walk and `stat` bypass containment.
- minor/medium — Phase 2: symlink aliases evade the overlap check.
- suggestion/medium — Phase 1: encoding contract for member paths.
- suggestion/low — Phase 10: watcher admits every non-dot path.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-24

**Verdict:** REVISE

The user decided the three escalations before this pass: the overlap
refusal stays in the shared resolver and the migrate adapter propagates
it (E1, option a); the indexer gets root-only default accessors with
explicit member accessors (E2, option b); `ActivityFeed` refetches
`/api/activity` on each event (E3, option b).

### Previously Identified Issues

- 🟡 **Architecture/Compatibility/Correctness**: Overlap refusal reaches every `doc_type_dirs` caller — Resolved (callers, tests, CHANGELOG and Migration Notes)
- 🟡 **Architecture/Code Quality**: Members share the flat index — Resolved
- 🟡 **Compatibility**: Live activity feed bypasses the roll-up — Resolved
- 🟡 **Compatibility**: Search row key collision — Resolved
- 🟡 **Correctness**: Concurrent rescans double-emit — Partially resolved (emit ordering and non-diffing rescans remained)
- 🟡 **Correctness**: Ledger and pairing computed apart — Resolved
- 🟡 **Performance**: Widened watcher admission — Partially resolved (per-path rescans remained)
- 🟡 **Security**: `/api/docs` lexical set-root decision — Resolved
- 🟡 **Security**: Walk and stat bypass containment — Resolved
- 🟡 **Code Quality**: Cluster recompute triplication — Resolved
- 🟡 **Code Quality**: `SetMembership.place` invalid states — Resolved
- 🟡 **Architecture**: Boundary tests miss `TypedRef` — Resolved (recorded exception)
- 🟡 **Test Coverage**: Phase 4 characterisation gaps — Resolved
- 🟡 **Test Coverage**: Single-path member refresh untested — Resolved
- 🟡 **Test Coverage**: Exclusion tests may never fail — Resolved
- 🟡 **Test Coverage**: Fabricated records unchecked — Partially resolved (no seam into map construction)
- 🟡 **Standards**: Member API `path` naming — Resolved
- 🟡 **Standards**: `DocType` name collision — Resolved
- 🔵 Minor and suggestion findings from pass 1 — Resolved, apart from the deliberately deferred topic-research contract test and the unrestructured `DocType` record

### New Issues Introduced

- 🟡 **Compatibility**: Resolving doc-type directories once at context construction would change what frozen migrations see after `m0004` rewrites `paths.*` — fixed: the port becomes fallible, resolution stays live, and the runner resolves once up front
- 🟡 **Correctness**: The refetch-only feed drops in-app writes, because the PATCH handler bypasses the ring buffer — fixed: the PATCH event goes through `emit`
- 🟡 **Correctness**: Full rescans outside `rescan_with_diff` absorb changes silently — fixed: `rescan_with_diff` is the only full-rescan entry point
- 🟡 **Code Quality**: `classify` on the unparsed static shape — fixed: `SetMatcher` built and held by the catalogue
- 🟡 **Test Coverage**: No seam for fabricated members into map construction — fixed: `IndexMaps::build` shared by `rescan` and `Indexer::from_records`
- 🔵 Diff snapshots must include members; ledger/latest/emit ordering; emit order across rescans; `list_sets` on the `FileDriver` trait; `recompute_clusters` reading a private iterator; `library_order` hedge (now `LibraryPlacement`); remaining hedges; `source_scan` home in `cli-test-support` and Phase 10's duplicate guard folded into Phase 4's; `FLAT_DEFAULTS` behind a feature; shared `first_overlap`; `SetPlacement` value; uncanonicalisable roots fail closed; set-level debounce and empty-diff skip; client fan-out note; migrate-cli and launcher refusal tests; root-`id` refresh test; scanner guards; package names in the nextest filter; `config` public-api step; `from_wire_token` — all fixed in place

### Assessment

There were five new major findings, so the verdict stays REVISE. None
needed escalating, and all are fixed in the plan. A third pass checks
the fixes.

## Re-Review (Pass 3) — 2026-11-24

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Compatibility**: Resolving doc-type dirs once changed frozen migrations — Resolved
- 🟡 **Correctness**: Refetch-only feed dropped in-app writes — Resolved
- 🟡 **Correctness**: Full rescans outside `rescan_with_diff` — Resolved
- 🟡 **Code Quality**: `classify` on the unparsed shape — Resolved
- 🟡 **Test Coverage**: No seam into map construction — Resolved
- 🔵 Pass-2 minor findings (diff snapshots, ordering, `list_sets` on the trait, `recompute_clusters` access, `LibraryPlacement`, hedges, `source_scan` home, `FLAT_DEFAULTS`, `first_overlap`, `SetPlacement`, fail-closed roots, debounce, fan-out, refusal tests, root refresh, scanner guards, filters, snapshots, `from_wire_token`) — Resolved
- 🔒 **Security** — no findings this pass

### New Issues Introduced

- 🟡 **Architecture**: Emission inside `rescan_with_diff` coupled the indexer to delivery, and start-up became a second entry point — fixed: an injected `ChangeSink` owns recompute, suppression and emission, and the constructor's initial build is the one non-emitting exception
- 🟡 **Architecture**: Ownership of an in-app write's event was split — fixed: the sink owns it through `WrittenDocument`, and the PATCH handler emits nothing
- 🟡 **Correctness**: The self-write suppression filter was dropped — fixed: the sink applies `should_suppress` and always emits the written document once
- 🟡 **Correctness**: Events left before the cluster recompute — fixed: the sink recomputes and backfills before emitting
- 🟡 **Correctness**: Tests wrote set members through a work-item-only endpoint — fixed: member and root changes are driven by external writes; concurrent tests pair an external member edit with a work-item PATCH
- 🟡 **Standards**: A second OS-watcher test was added to a lane that runs exactly one test — fixed: the case-only rename is folded into the lane's single test
- 🔵 `linkage_table` and `m0007` missing from the fallible-port list; mid-run overlap recording; double rescan on in-app writes; refusal sample versus `first_overlap`; stale `shape.classify`; `may_contain_members` query; `SetPlacement` in Phase 7; matcher ownership through `Arc<DocTypeCatalogue>`; single `IndexMaps::build` with the ledger; corpus `test-support` feature; public-api steps in Phases 5 and 10; missing-root overlap, ordering, debounce, trust-flip and start-up tests; driver using the ancestor-resolved root — all fixed in place

### Assessment

There were six new major findings, all fixed in the plan and none
needing escalation. A fourth pass checks them.

## Re-Review (Pass 4) — 2026-11-24

**Verdict:** COMMENT

Seven lenses re-ran (security was clean in pass 3).

### Previously Identified Issues

- 🟡 **Architecture**: Indexer depended on delivery; start-up conflict — Resolved
- 🟡 **Architecture**: Split ownership of an in-app write's event — Resolved
- 🟡 **Correctness**: Self-write suppression dropped — Resolved
- 🟡 **Correctness**: Events left before the recompute — Resolved
- 🟡 **Correctness**: Tests wrote members through a work-item-only endpoint — Resolved
- 🟡 **Standards**: Second test in a single-test lane — Resolved
- 🔵 Pass-3 minor findings — Resolved, apart from two code-quality suggestions kept by choice (two `FileDriver` listing methods; asset events carrying `SetDocumentRole::Member`)

### New Issues Introduced

- 🟡 **Correctness**: Emitting the written document unconditionally could double-emit once the 5 s suppression window lapsed — fixed by simplifying: only flat work items are writable, so the PATCH handler keeps today's refresh-then-emit order through `emit`, and the `WrittenDocument` mechanism is removed
- 🟡 **Architecture**: Storing the `ChangeSink` on the `Indexer` made a cycle that today's construction order cannot satisfy — fixed: the sink is passed per call to `rescan_with_diff` and `refresh_one`
- 🔵 Public `rescan()` could bypass the diff — fixed: it becomes the private, non-swapping `build_records`
- 🔵 `corpus_index` becomes fallible with `linkage_table`; `--list` refuses through `run_list`, and the CHANGELOG says so; the resolve sits before the no-pending return, with a test
- 🔵 Member inbound linkage stays on the precomputed `work_item_refs_by_target` map
- 🔵 `refresh_one` stays the plain resync; `first_overlap` is typed on `&Path`; the placeholder comment is removed from the `SetMatcher` snippet
- 🔵 Tests: the mid-run stub test moves to `migrate/tests/lifecycle.rs`; the PATCH race test is made deterministic; the cluster-freshness test uses a cluster-moving edit; a `TestSink` wraps the production sink; the case-variant test skips on case-sensitive volumes

### Assessment

There were fewer than three major findings and no critical ones, so the
verdict is COMMENT. Both majors and every minor finding are fixed in
place, and there are no outstanding escalations. The plan is ready for
implementation.
