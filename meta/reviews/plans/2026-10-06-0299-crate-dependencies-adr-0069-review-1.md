---
type: "plan-review"
id: "2026-10-06-0299-crate-dependencies-adr-0069-review-1"
title: "Plan Review: Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069 Implementation Plan"
date: "2026-10-06T12:58:04+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-06-0299-crate-dependencies-adr-0069"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "compatibility", "performance", "standards"]
review_number: 1
review_pass: 2
tags: ["cli", "architecture", "dependencies", "refactor", "build-system"]
last_updated: "2026-10-06T14:06:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069 Implementation Plan

**Verdict:** REVISE

The plan is well sequenced: a characterisation safety net before any refactor,
a fixture-driven lint that ratchets a known-violations set down to `== []`, and
ports that land before their consumers switch. It falls down on several claimed
equivalences: m0008's guard over `classify` aborts on every frontmatter-less
document (critical, verified against `corpus-adapters/src/document.rs:36-40`),
m0001 and `frontmatter_text` change semantics, and `LockError` and
`collaboration::OriginRemote` drop error branches. The safety net and baseline
have determinism gaps around the launcher's release fetch, and most `cargo test
-p` gates use directory names rather than package names.

### Cross-Cutting Themes

- **Frontmatter port does not preserve `document::parse` semantics** (flagged
  by: correctness, test-coverage, compatibility, code-quality) — `classify`
  returns `Absent` for unfenced input where `parse` returns `{}`, folds
  non-mapping roots into `Malformed`, and drops error detail. m0008, m0001 and
  `frontmatter_text` all inherit the mismatch, and the Phase 2 fixtures contain
  none of the diverging inputs. The stated fallback (editing `classify`) would
  change the visualiser's `frontmatter_state` field.
- **Release-fetch nondeterminism in the summary path** (flagged by:
  correctness, test-coverage, compatibility, performance) — neither the
  characterisation harness nor `measure:summary-latency` pins
  `ACCELERATOR_RELEASE_BASE_URL` or `ACCELERATOR_VCS_BIN`, so the "without
  `accelerator-vcs`" goldens and the before latency figures depend on GitHub
  reachability. Phase 7 also changes more than the one golden it names.
- **Error channels dropped by new ports** (flagged by: correctness,
  code-quality, compatibility, test-coverage) — `collaboration::OriginRemote`
  returns `Option` where today's trait returns `Result<Option<_>, _>`, and
  `LockError` loses `NotWritable` and has no specified `Display`.
- **`MigrationContext::frontmatter()` cannot take an erroring default body**
  (flagged by: code-quality, architecture, correctness) — it returns
  `&dyn FrontmatterParser`, not a `Result`, and breaks the operation-style
  pattern of `ports.rs`.
- **Single-implementation and lifetime-constrained seams** (flagged by:
  architecture, code-quality) — `CanonicalRenderer` adds an adapter-internal
  indirection, and `&'static dyn RepositoryTracking` forbids stateful
  trackers.
- **Phase 10 extension seam unspecified** (flagged by: code-quality,
  test-coverage, architecture) — neither the representation of an unknown
  field in `PullConfig` nor a public table-taking entry point is named, and the
  launcher dump is not driven by the injected table.

### Tradeoff Analysis

- **Catalogue in platform `config` vs per-tracker ownership**: Architecture
  notes `TRACKER_BLOCKS` puts product data in a platform domain and keeps
  parallel filter lists in `tracker-support`. This is the cost of the mandated
  approach; record it as an accepted consequence and derive `FilterSchema` from
  the catalogue to remove the duplication.
- **Fault isolation vs in-process simplicity**: Phase 7 removes the child
  process that contained gix/jj-lib panics and hangs. The no-deadline choice is
  recorded; the lost panic containment is not. A `catch_unwind` boundary folding
  to `Unknown` restores fail-closed behaviour at low cost.

### Findings

#### Critical

- 🔴 **Correctness**: m0008's value-change guard over `classify()` aborts on every document without frontmatter
  **Location**: Phase 9, Section 1: `migrate` domain (m0008)
  `classify` returns `Absent` for an unfenced original and `Parsed({})` for its
  rendering, so the guard fails on every frontmatter-less meta/ file. It also
  collapses distinct non-mapping roots to equal `Malformed` values.

#### Major

- 🟡 **Correctness + Compatibility + Test Coverage**: m0001's malformed gate diverges for non-mapping roots; fixtures miss it; fallback breaks other `classify` consumers
  **Location**: Phase 9 §1 (m0001); Phase 2 `test_migrations.py`
  `document::parse` accepts sequence and scalar roots that `classify` calls
  `Malformed`. No Phase 2 fixture contains one, and editing `classify` would
  reclassify documents in the visualiser API.
- 🟡 **Correctness + Code Quality**: one `frontmatter_text` serves two consumers with different split semantics, and loses diagnostic detail
  **Location**: Phase 8 §1; Phase 9 §1 (m0008)
  The pipeline gates on `Parsed` and swallows split errors; m0008 needs raw
  split, `Unterminated` propagation and empty-string-for-unfenced.
- 🟡 **Correctness + Code Quality + Test Coverage**: `collaboration::OriginRemote` drops the probe-failure branch
  **Location**: Phase 11 §2
  Today's trait returns `Result<Option<String>, kernel::Error>` and its error
  path is tested at `collaboration/src/lib.rs:218`. No characterisation case
  covers origin resolution.
- 🟡 **Correctness + Compatibility**: `LockError` loses `NotWritable`; `Display` text unspecified
  **Location**: Phase 4 §2
  Callers print the error directly, so permission-denied and timeout text
  would change unpinned.
- 🟡 **Correctness + Test Coverage + Compatibility**: "without `accelerator-vcs`" goldens depend on the network; Phase 7 changes more than one golden
  **Location**: Phase 2 §1–2; Phase 7 §4; Implementation Approach
  `ACCELERATOR_RELEASE_BASE_URL` is not pinned. Tracked and untracked, git and
  jj — at least four goldens — change when the note disappears.
- 🟡 **Correctness + Performance + Compatibility**: before summary-latency figures time a network fetch, not dispatched tracking
  **Location**: Phase 1 §2
  Cold runs fetch or time out (up to 5 s); the 10× gate can never trip.
- 🟡 **Correctness**: success-criteria commands use directory names, not package names
  **Location**: Phases 4–11 Success Criteria; Phase 6 Manual; Phase 3 declarations test
  `corpus-cli` is `accelerator-corpus`, `work-cli` is `accelerator-work`,
  the visualiser is `accelerator-visualiser`, and so on. Cargo rejects the
  unknown specs.
- 🟡 **Code Quality + Architecture + Correctness**: `MigrationContext::frontmatter()` cannot have a `MigrationError` default body
  **Location**: Phase 9 §1
  Expose `Result`-returning operations instead of a collaborator accessor.
- 🟡 **Code Quality + Test Coverage + Architecture**: Phase 10 unknown-field representation, table injection seam and dump coverage unspecified
  **Location**: Phase 10 §2, §3, §4
  The extension test cannot reach the launcher dump, and private
  `&[TrackerBlock]` APIs are not reachable from `tests/`.
- 🟡 **Architecture**: in-process tracking removes fault isolation with no panic boundary
  **Location**: Phase 7
  A gix/jj-lib panic now aborts the whole SessionStart summary.
- 🟡 **Architecture**: work-sync baseline logic moves into the `migrate-cli` composition root
  **Location**: Phase 9 §4
  About 120 lines of work policy belong in `work-adapters`, with a thin
  delegating `WorkSyncBaselines` in the root.
- 🟡 **Test Coverage**: lock-contention cases block for the 300 s default ceiling
  **Location**: Phase 2 (`test_work_commands.py`, `test_tracker_caches.py`, `test_research_commands.py`)
  Roughly an hour across git and jj. Pin reclaim via a dead-PID sentinel and
  leave timeout text to crate tests.
- 🟡 **Performance**: fixture repositories cannot reveal the unbounded in-process cost
  **Location**: Performance Considerations; Phase 1 §2
  Add an informational run against a realistically sized repository.
- 🟡 **Standards**: `mise run public-api:fix` does not exist
  **Location**: Implementation Approach
  The task is `public-api:update`.
- 🟡 **Standards**: `vcs::RepositoryFacts` trait clashes with `corpus::RepositoryFacts` struct
  **Location**: Phase 5 §1
  Name it in line with `vcs`'s `*Probe` convention.

#### Minor

- 🔵 **Architecture + Code Quality**: `CanonicalRenderer` is a single-implementation seam inside one adapter crate
  **Location**: Phase 9 §3
- 🔵 **Architecture + Code Quality**: `&'static dyn RepositoryTracking` restricts the port to stateless constants
  **Location**: Phase 6 §1
- 🔵 **Architecture**: tracker block knowledge split across `config` and `tracker-support` with duplicated filter lists
  **Location**: Phase 10
- 🔵 **Architecture**: expected-declarations table is a second registration point the checklists omit
  **Location**: Phase 3 §3; Phase 12 §2
- 🔵 **Correctness**: the known-violation set cannot be "exactly 0299's table"
  **Location**: Phase 3 §3
  `work` → `config` is rule 3; `migrate-adapters` → `work-adapters` may
  double-report under rules 2 and 4.
- 🔵 **Correctness + Test Coverage**: validation order omits parse-stage errors; single-fault fixtures cannot pin order
  **Location**: Phase 10 §1; Phase 2 dump/tracker-block cases
- 🔵 **Correctness**: privatising `file_tracking`/`repository_roots` breaks `vcs-adapters/tests/{file_tracking,roots}.rs`; wrong caller named
  **Location**: Phase 5 §2; Phase 7 §2
- 🔵 **Correctness**: `read_records` missing-file, empty-line and error-text semantics unspecified
  **Location**: Phase 9 §3
- 🔵 **Correctness**: end-state lists `verify` as a launcher dependency (it is not) and a dev-only `work-adapters` → `tracker-support` edge
  **Location**: Desired End State
- 🔵 **Code Quality**: `RepositoryFacts` mixes questions and shadows an inherent method name
  **Location**: Phase 5 §1–2
- 🔵 **Code Quality**: two error taxonomies for one structural failure risk duplicated text
  **Location**: Phase 10 §1–2
- 🔵 **Code Quality**: stringly-typed block lookup and implicit noun pairing
  **Location**: Phase 10 §1
- 🔵 **Compatibility**: in-process gix/jj-lib could write into the SessionStart hook output
  **Location**: Phase 7 §1
- 🔵 **Test Coverage**: nothing mechanically shows goldens unchanged
  **Location**: Phase 2 harness; every phase's Success Criteria
- 🔵 **Test Coverage**: lock error mapping and `holder_pid` not pinned
  **Location**: Phase 4 §2
- 🔵 **Test Coverage**: port contract tests cover tracking only, not `RepositoryFacts` or nested roots
  **Location**: Phase 5 §2
- 🔵 **Test Coverage**: gix feature guard does not cover the launcher's unified graph
  **Location**: Phase 7 §1
- 🔵 **Test Coverage**: stub-driven pipeline tests risk drifting from `YamlFrontmatter`
  **Location**: Phase 8 §3
- 🔵 **Performance**: colocated jj, the most expensive tracking path, is not measured
  **Location**: Phase 1 §2
- 🔵 **Performance**: fixture tracking state and "cold" definition unspecified
  **Location**: Phase 1 §2
- 🔵 **Standards**: checklist-count update misses other "thirteen" references and the bold-verb style
  **Location**: Phase 12 §2
- 🔵 **Standards**: `collaboration::OriginRemote` shares `vcs::OriginRemote`'s name
  **Location**: Phase 11 §2
- 🔵 **Standards**: characterisation harness duplicates `accelerator_env` and `ceiling_directories`
  **Location**: Phase 2 §1
- 🔵 **Standards**: `LockError` shape does not follow `store::WriteError` conventions
  **Location**: Phase 4 §2

#### Suggestions

- 🔵 **Performance**: bound characterisation suite cost with template repositories
  **Location**: Phase 2
- 🔵 **Performance**: report p90 ratio beside the median-only 10× gate
  **Location**: Phase 13 §3
- 🔵 **Code Quality**: share a `_timed_run` helper rather than copy `subprocess_measurement_runner`
  **Location**: Phase 1 §2
- 🔵 **Test Coverage + Correctness**: cover SCC boundaries (self-loops, three-context cycles) and commit the Phase 12 injection check as a test
  **Location**: Phase 3 §2–3; Phase 12

### Strengths

- ✅ Characterisation suite over built binaries in git and jj lands before any refactor, decoupled from every Rust API the refactor reshapes.
- ✅ The known-violations test ratchets phase by phase to `== []`, turning progress into a checked assertion rather than a growing exception list.
- ✅ The lint has an explicit domain model (`Role`, `Kind`, `Crate`, `Context`, `Edge`, `Finding`), synthetic-metadata fixtures, and reuses `workspace_packages()` from `tasks/shared/`.
- ✅ `config-adapters` depends on `vcs`, never `vcs-adapters`, keeping gix and jj-lib out of the visualiser's closure.
- ✅ Narrow consumer ports (`RepoFactsProbe`, `VcsIdentityProbe`, `WorkingCopy`) are kept and re-implemented over the new `vcs` port.
- ✅ Lock and `TEMP_PREFIX` move to technical libraries; `holder_pid` removes a duplicated sentinel reader; `TEMP_PREFIX` is pinned to `.tmp-`.
- ✅ Dead code goes: `FetchBudget`, `FileSessionLogRewriter`, `work` → `config`.
- ✅ Removing `vcs tracking` is safe because the launcher pins sub-binaries to its own `CARGO_PKG_VERSION`.
- ✅ Measurement tasks follow the `MeasurementRunner` Protocol pattern, and warm dispatch is measured alongside summary latency.
- ✅ The `audit` rewrite keeps the `Known(tracking)` arm exactly.

### Recommended Changes

1. **Replace the `FrontmatterState`-based m0008 guard and m0001 gate with a lossless port operation** (addresses: m0008 guard aborts; m0001 diverges; `frontmatter_text` semantics; `MigrationContext::frontmatter()` default body)
   Either move the parse–render–compare guard behind `render_canonical`
   (implemented over `document::Yaml` in `migrate-adapters`) or add a
   `parse_value`/well-formedness operation with `document::parse`'s exact
   semantics. Split `frontmatter_text` into named operations with explicit
   contracts. Expose them on `MigrationContext` as `Result`-returning methods
   with erroring defaults. Keep `classify` unchanged and state that the
   visualiser's `frontmatter_state` must not change.
2. **Extend Phase 2 fixtures to cover every divergence** (addresses: m0001 fixtures; m0008; validation order; origin remote; lock not-writable)
   Add frontmatter-less, sequence, scalar, null and invalid-YAML roots for
   m0001 and m0008; multi-fault tracker blocks; `work sync --preview` with an
   invalid pull block; a `collaboration-cli` origin case; a read-only lock
   parent; a migrate run-lock case.
3. **Pin the release fetch in the harness and the measure task** (addresses: network-dependent goldens; before-latency baseline)
   Point `ACCELERATOR_RELEASE_BASE_URL` at a refusing loopback server; set
   `ACCELERATOR_VCS_BIN` to the locally built binary for the before latency
   run and seed warm caches from it; assert each run took the intended path.
   Name every golden Phase 7 changes and reconcile the Implementation Approach
   wording.
4. **Keep error channels on new types** (addresses: `OriginRemote`; `LockError`)
   `collaboration`'s port returns `Result<Option<String>, kernel::Error>` under
   a distinct name; `LockError` gains `NotWritable`, mirrors `WriteError`'s
   derives and `#[non_exhaustive]`, and reproduces `StoreError`'s text.
5. **Fix verification commands and end-state facts** (addresses: package names; `public-api:fix`; `verify`; known-violation set)
   Use package names in every `-p`; use `public-api:update`; drop `verify`
   and the dev-only edge from the end state; pin the computed finding list.
6. **Make lock-contention cases fast** (addresses: 300 s ceiling)
   Use a dead-PID sentinel at binary level; leave timeout text to crate tests.
7. **Add a panic boundary for in-process tracking** (addresses: fault isolation; hook output)
   Fold an unwinding panic to `FileTracking::Unknown`, test it with a
   panicking stub, and assert hook stdout is one JSON line with empty stderr.
8. **Specify the Phase 10 extension seam** (addresses: unknown-field representation; dump coverage; duplicated filter lists)
   Name the public `*_with(blocks, ..)` entry points, the field representation,
   a dump core over `&[TrackerBlock]` with a launcher test, and derive
   `FilterSchema` from the catalogue.
9. **Move `WorkSyncBaselines`' body into `work-adapters`** (addresses: composition root absorbing policy)
10. **Tidy smaller seams** (addresses: `CanonicalRenderer`; `&'static`; `RepositoryFacts` naming; `vcs-adapters` tests; `read_records`; checklist references; declaration-table registration)

## Per-Lens Results

### Architecture

**Summary**: A well-sequenced application of ADR-0069's strict injection,
with correct placement of most moves. Structural concerns: lost process-level
fault isolation in Phase 7, `migrate-cli` absorbing work-sync policy, tracker
block knowledge split across two crates, and a few interfaces that resist
evolution.

**Strengths**:
- Sequencing follows the dependency direction it enforces; the lint gates only in Phase 12.
- Known-violations ratchet makes the refactor a series of measurable steps.
- `config-adapters` → `vcs`, never `vcs-adapters`, keeps the visualiser closure clean.
- Narrow consumer-side ports are preserved.
- Lock and `TEMP_PREFIX` go to technical libraries; `store` keeps its own error.
- Lint has an explicit domain model and reuses `workspace_packages()`.

**Findings**:
- 🟡 major / high — **Phase 7** — In-process tracking removes the launcher's fault isolation from gix/jj-lib with no panic boundary. `vcs-cli/src/report.rs` already uses `catch_unwind`; mirror it at the adapter edge folding to `Unknown`.
- 🟡 major / medium — **Phase 9 §4** — Work-sync baseline logic (`context.rs:297-413`) moves into the composition root. Put it in `work-adapters` and delegate thinly.
- 🔵 minor / medium — **Phase 10** — Tracker block knowledge split across `config` and `tracker-support`; `JIRA_FILTERS`/`LINEAR_FILTERS` duplicated. Derive from the catalogue and record the ownership split.
- 🔵 minor / high — **Phase 9 §1** — `frontmatter()` accessor breaks the default-body capability pattern.
- 🔵 minor / medium — **Phase 9 §3** — `CanonicalRenderer` is a one-implementation seam.
- 🔵 minor / medium — **Phase 6 §1** — `&'static dyn RepositoryTracking` restricts substitutability.
- 🔵 minor / medium — **Phase 3 §3 / Phase 12 §2** — The expected-declarations table is a second registration point the checklists omit.
- 🔵 minor / low — **Phase 10 §4** — The launcher dump has no stated seam for the injected block table.

### Correctness

**Summary**: Carefully sequenced, port shapes mostly match live code, but
several claimed equivalences are not equivalent, and the verification commands
and golden expectations have defects.

**Strengths**:
- Known-violation shrinkage matches what the lint would report per phase.
- `RepositoryFacts::working_copy_state` matches `InProcessProbe`'s signature.
- The `audit` rewrite keeps the `Known(tracking)` arm exactly.
- `&'static` with a unit struct compiles via rvalue static promotion.
- The plan flags the m0001 risk rather than assuming equivalence.

**Findings**:
- 🔴 critical / high — **Phase 9 §1 (m0008)** — Guard over `classify()` aborts on every frontmatter-less document (`Absent` vs `Parsed({})`) and cannot distinguish non-mapping roots. Move the guard behind the render port or add a lossless `parse_value`.
- 🟡 major / high — **Phase 9 §1 (m0001)** — Non-mapping roots newly refused; fallback breaks other `classify` consumers. Add a well-formedness operation; add fixtures.
- 🟡 major / medium — **Phase 8 §1** — `frontmatter_text` must serve pipeline (Parsed-gated) and m0008 (raw split) semantics. Specify or split.
- 🟡 major / high — **Phase 11 §2** — `OriginRemote` signature drops `Result`.
- 🟡 major / high — **Phase 4 §2** — `LockError` loses `NotWritable`; `Display` unspecified.
- 🟡 major / high — **Phase 2 §2, Phase 7 §4** — "Without vcs" goldens are network-dependent; at least four goldens change in Phase 7, not one.
- 🟡 major / medium — **Phase 1 §2** — Before latency measures a fetch; gate cannot trip. Use `ACCELERATOR_VCS_BIN` and a loopback release URL.
- 🟡 major / high — **Phases 4–11 Success Criteria** — `-p` uses directory names; package names are `accelerator-corpus`, `accelerator-work`, `accelerator-design`, `accelerator-research`, `accelerator-collaboration`, `accelerator-migrate`, `accelerator-vcs`, `accelerator-visualiser`. The declarations test must key by package name.
- 🔵 minor / high — **Phase 3 §3** — Known-violation set cannot be "exactly 0299's table"; pin the computed list and decide the rule 2/4 double report.
- 🔵 minor / high — **Phase 9 §1** — `frontmatter()` cannot return `MigrationError` by default.
- 🔵 minor / medium — **Phase 10 §1** — Validation order omits `NotAMapping`/`SubBlockNotAMapping` and `max_pages.<sub>` unknown collection.
- 🔵 minor / high — **Phase 5 §2, Phase 7 §2** — Privatising free functions breaks `vcs-adapters/tests/{file_tracking,roots}.rs`; `design-cli/src/executor.rs:380` uses `facts`, not these.
- 🔵 minor / medium — **Phase 9 §3** — `read_records` missing-file, empty-line and error-text semantics unspecified.
- 🔵 minor / high — **Desired End State** — Launcher has no `verify` dependency; `work-adapters` → `tracker-support` is dev-only.
- 🔵 suggestion / low — **Phase 3 §2** — Drop intra-context edges before SCC; only components of two or more contexts are findings.

### Test Coverage

**Summary**: A strong safety net with a ratchet and test-first discipline, but
the Phase 2 fixtures miss the m0001 divergence Phase 9 relies on, lock cases
are impractically slow, the extension test cannot reach the dump, and the
origin-remote reroute is uncovered.

**Strengths**:
- Characterisation suite committed first, decoupled from Rust APIs.
- Synthetic-metadata lint fixtures per acceptance bullet.
- Known-violations ratchet.
- The single intended behaviour change is confined to a reviewed golden diff.
- `Unchecked` tests deleted with their behaviour and replaced test-first.
- Measurement scheduling unit-tested with fake runners.
- `FieldKind::Filters.accepted` pinned against `FilterSchema`.

**Findings**:
- 🟡 major / high — **Phase 2 / Phase 9** — m0001 fixtures miss sequence, scalar and null roots.
- 🟡 major / high — **Phase 2** — Lock-contention cases block for the 300 s default ceiling; use dead-PID reclaim at binary level.
- 🟡 major / high — **Phase 2 / Phase 7** — "Without vcs" golden depends on an unpinned release fetch.
- 🟡 major / high — **Phase 10 §4** — Extension test cannot exercise the launcher dump.
- 🟡 major / medium — **Phase 11 §2** — No case covers origin-remote reroute; error-path test may be lost.
- 🔵 minor / high — **Phase 2 harness** — Nothing shows goldens unchanged; add `git diff --exit-code` on goldens, fail on missing golden, refuse `UPDATE_GOLDEN` under CI.
- 🔵 minor / medium — **Phase 4 §2** — Lock error mapping and `holder_pid` cases not pinned.
- 🔵 minor / medium — **Phase 2 / Phase 10 §2** — Single-fault fixtures cannot pin order; `work sync` with an invalid pull block uncovered.
- 🔵 minor / medium — **Phase 5 §2** — Contract tests cover tracking only.
- 🔵 minor / medium — **Phase 7 §1** — `test_vcs_library_graph.py` checks gix features only for `-p vcs-adapters`; add `-p accelerator` and list `tests/integration/deny`.
- 🔵 minor / medium — **Phase 8 §3** — Add `YamlFrontmatter` and `PatternCanonicaliser` contract tests.
- 🔵 suggestion / medium — **Phase 3 §3, Phase 12** — Cover SCC boundaries and commit the injection check as a test.

### Code Quality

**Summary**: Well sequenced for maintainability; most seams proportionate. Five
design details need tightening: `OriginRemote` error channel,
`MigrationContext::frontmatter()`, `CanonicalRenderer`, the Phase 10 extension
representation, and lost m0008 diagnostics plus a `'static` constraint.

**Strengths**:
- Shrinking known-violations test.
- Rich lint domain types with pure functions.
- `holder_pid` removes duplication; `LockError` follows `WriteError`'s shape.
- Dead code removed rather than carried.
- Catalogue replaces hand-maintained `NOUN_OWNERS` and dump field lists.
- Measurement follows the `MeasurementRunner` pattern.

**Findings**:
- 🟡 major / high — **Phase 11 §2** — `OriginRemote` drops the error channel.
- 🟡 major / high — **Phase 9 §1** — `frontmatter()` cannot have an erroring default; expose operations.
- 🟡 major / medium — **Phase 9 §1** — m0008 loses parser error detail; carry a reason in `Malformed` or return `Result`.
- 🟡 major / medium — **Phase 10 §2, §4** — Unknown-field representation and public table-taking entry points unspecified.
- 🔵 minor / medium — **Phase 9 §3** — `CanonicalRenderer` is YAGNI.
- 🔵 minor / medium — **Phase 6 §1** — `&'static` leaks a lifetime constraint; let `TrackedConfigFile` own its port.
- 🔵 minor / medium — **Phase 5 §1–2** — `RepositoryFacts` mixes questions; inherent and trait `working_copy_state` share a name.
- 🔵 minor / medium — **Phase 10 §1–2** — Wrap structural errors as `Structure(BlockError)` to avoid duplicated text.
- 🔵 minor / low — **Phase 10 §1** — Use a `BlockName` enum; test the one-`EntityList`-one-`ScopeFlag` invariant; state `label`'s purpose.
- 🔵 suggestion / medium — **Phase 1 §2** — Extract `_timed_run`; consider a separate measure module.

### Compatibility

**Summary**: The main cross-version risk is handled: the launcher pins
sub-binaries to its own version. Remaining risks: `LockError` text,
m0001/`classify` and the visualiser API, and network-dependent goldens and
baselines.

**Strengths**:
- Migration Notes backed by `expected_version: env!("CARGO_PKG_VERSION")`.
- `TEMP_PREFIX` pinned to `.tmp-`.
- Re-exports keep visualiser and research import paths.
- Visualiser closure stays free of gix/jj-lib.
- `PullConfigError`/`PushConfigError` kept for callers.

**Findings**:
- 🟡 major / high — **Phase 4 §2** — `LockError` drops `NotWritable`; user-visible text unspecified.
- 🟡 major / medium — **Phase 9 §1** — m0001 switch changes refusals; fallback changes the visualiser's `frontmatter_state`.
- 🟡 major / medium — **Phase 2** — "Without vcs" golden depends on the network.
- 🔵 minor / medium — **Phase 1 §2** — Before cold latency includes a network fetch.
- 🔵 minor / low — **Phase 7 §1** — In-process gix/jj-lib output could corrupt the hook's single-line JSON.

### Performance

**Summary**: Structural changes have negligible cost; Phase 7's in-process
gix/jj-lib with no deadline is the one that matters. Committed measurement and
a gate are right, but the baseline may not time the same path and fixtures are
too small to show the cost.

**Strengths**:
- Committed, repeatable before/after measurement on one host with warm-ups and p90.
- Warm dispatch measured, covering every PreToolUse hook.
- In-process tracking removes a spawn, pipe and possible fetch.
- Port dispatch overhead is negligible.
- Interim measurement skipped deliberately.

**Findings**:
- 🟡 major / medium — **Phase 1 §2** — Before figures may not time the tracking path; pin it and assert each run's output.
- 🟡 major / medium — **Performance Considerations; Phase 1 §2** — Tiny fixtures cannot reveal unbounded cost; add a realistic-size informational run.
- 🔵 minor / high — **Phase 1 §2** — Colocated jj not measured.
- 🔵 minor / medium — **Phase 1 §2** — Fixture tracking state and "cold" definition unspecified.
- 🔵 suggestion / low — **Phase 2** — Template repositories and a shared loopback server to bound suite time.
- 🔵 suggestion / low — **Phase 13 §3** — Report p90 ratio alongside the median gate.

### Standards

**Summary**: Mostly follows conventions for task names, lint wiring and
checklist counts. Issues: non-existent `public-api:fix`, two name clashes,
missed "thirteen" references, harness helper placement and `LockError`'s shape.

**Strengths**:
- `measure:*` naming and reuse of `summarise`/`MeasurementRunner`.
- Lint registered as `lint:crate-dependencies:check` in both roll-ups; `_CLI_CHECK_GATES` gains the missing config-test-support gate.
- `UPDATE_GOLDEN=1` matches existing suites.
- `workspace_packages()` moved to `tasks/shared/`.
- Checklist count updated across README, test and both `CLAUDE.md` files.
- Kebab-case declaration values.

**Findings**:
- 🟡 major / high — **Implementation Approach** — `public-api:fix` does not exist; use `public-api:update`.
- 🟡 major / high — **Phase 5 §1** — `vcs::RepositoryFacts` trait clashes with `corpus::RepositoryFacts` struct and sits beside `RepoFacts`.
- 🔵 minor / medium — **Phase 11 §2** — `collaboration::OriginRemote` shares `vcs::OriginRemote`'s name.
- 🔵 minor / high — **Phase 12 §2** — Misses `test_the_checklist_has_thirteen_points`, the line 118 comment and `tasks/public_api.py:78`; bold only the leading verb.
- 🔵 minor / medium — **Phase 2 §1** — Reuse `accelerator_env` and `ceiling_directories`; place shared harness in `tests/integration/support/`.
- 🔵 minor / medium — **Phase 4 §2** — `LockError` should mirror `WriteError` (`#[non_exhaustive]`, derives, hand-written `Display`); update the crate doc's dependency line.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-06T13:54:00+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔴 **Correctness**: m0008's value-change guard over `classify()` aborts on every document without frontmatter — Resolved
- 🟡 **Correctness + Compatibility + Test Coverage**: m0001 diverges for non-mapping roots; fixtures miss it — Resolved
- 🟡 **Correctness + Code Quality**: one `frontmatter_text` serves two split semantics; m0008 loses detail — Resolved
- 🟡 **Correctness + Code Quality + Test Coverage**: `collaboration::OriginRemote` drops the error branch — Resolved
- 🟡 **Correctness + Compatibility**: `LockError` loses `NotWritable` — Resolved
- 🟡 **Correctness + Test Coverage + Compatibility**: network-dependent goldens; Phase 7 golden count — Resolved
- 🟡 **Correctness + Performance + Compatibility**: before latency times a fetch — Resolved
- 🟡 **Correctness**: `-p` uses directory names — Resolved
- 🟡 **Code Quality + Architecture + Correctness**: `MigrationContext::frontmatter()` default body — Resolved
- 🟡 **Code Quality + Test Coverage + Architecture**: Phase 10 extension seam unspecified — Resolved
- 🟡 **Architecture**: in-process tracking has no panic boundary — Resolved
- 🟡 **Architecture**: work-sync policy in the composition root — Resolved
- 🟡 **Test Coverage**: lock cases wait 300 s — Resolved
- 🟡 **Performance**: fixtures cannot reveal unbounded cost — Resolved
- 🟡 **Standards**: `public-api:fix` does not exist — Resolved
- 🟡 **Standards**: `RepositoryFacts` name clash — Resolved
- 🔵 **Test Coverage**: nothing shows goldens unchanged — Partially resolved (only Phase 7 lists the `jj diff` check)
- 🔵 **Correctness**: validation order — Partially resolved (wording diverges from first-fault semantics)
- 🔵 **Correctness**: `read_records` semantics — Partially resolved (error-text recovery through `StoreError` unstated)
- 🔵 **Test Coverage**: SCC boundaries and committed injection check — Partially resolved (injection check still manual)
- 🔵 All other pass-1 minor findings — Resolved
- 🔵 Pass-1 suggestions (template repositories, p90 ratio, `_timed_run`) — Still present by the author's choice

### New Issues Introduced

- 🔵 **Correctness**: `Capabilities.walker: &dyn CorpusWalker` cannot drive the pipeline, which also needs `FileReader` (Phase 9 §3)
- 🔵 **Correctness**: a session-log factory holding `&dyn RecordStore`/`&dyn Clock` cannot return `Box<dyn SessionLog>` (`'static`) (Phase 9 §3)
- 🔵 **Correctness**: the moved realign body builds `RealFs`/`FileCorpusStore` per baseline directory; moving it as is adds `work-adapters` → `corpus-adapters` (rule 4), and a single repo-rooted store breaks absolute `paths.integrations` (Phase 9 §4)
- 🔵 **Correctness**: m0008's YAML-dependent unit tests become stub checks once `migrate` drops `document` (Phase 9 §1)
- 🔵 **Correctness**: `holder_pid` legacy-`owner` answer unstated; may change migrate's refusal text (Phase 4 §2)
- 🔵 **Correctness + Test Coverage**: the git/jj parity grep conflicts with single-VCS cases and multi-parameter ids (Phase 2)
- 🔵 **Compatibility**: removing `TrackerBlock.label` leaves no source for "Jira"/"Linear" in the wrong-noun hint (Phase 10 §1)
- 🔵 **Test Coverage**: Phase 7's refused-release count is a test assertion, not golden content, so Phase 7 edits test code (Phase 2 / Phase 7)
- 🔵 **Test Coverage**: `WorkSyncBaselines` error mapping has no named test (Phase 9 §4)
- 🔵 **Code Quality**: `validate_against` takes a table and a block with implicit membership and mismatched lifetimes (Phase 10 §1)
- 🔵 **Code Quality**: m0008's guard reaches YAML through two routes (injected parser vs direct `document`) (Phase 9 §3)
- 🔵 **Architecture**: `RepositoryProbe` duplicates `vcs`'s fine-grained ports instead of composing them once in the domain (Phase 5 §1)
- 🔵 **Architecture**: the panic fold copies `report.rs`'s boundary at a different layer and without its `warn!` diagnostics (Phase 7 §2)
- 🔵 **Standards**: table-injection seams use `_in`/`_against`/`_with` where the codebase uses `_with` (Phase 10)
- 🔵 **Standards**: `RepositoryProbe::user_name`/`facts` reuse names `InProcessProbe` already carries (Phase 5)
- 🔵 **Performance**: the large-repository figure is ungated, and its repository state is not pinned between before and after (Phase 1 / Phase 13)
- 🔵 Suggestions: library-checklist bullet style (Standards); committed end-to-end injection test (Test Coverage)

### Assessment

The one critical and fifteen major findings are all resolved, and every lens
reports no major or critical issue, so the verdict moves to COMMENT. The plan
is implementable. The remaining minor findings fall into two groups.
Correctness's Phase 9 interface details (`Capabilities` reader, factory
lifetime, realign injection) would block compilation or reintroduce a rule 4
edge if left as written. They are worth a short edit before Phase 9 starts.
The rest are naming and wording refinements an implementer can settle in
passing.

## Verdict Change — 2026-10-06T14:06:36+00:00

**Verdict:** APPROVE

After pass 2, the remaining minor findings were addressed in the plan:
Phase 9 interface details (reader injection, factory lifetime, per-directory
baseline writers, error text), the restored `TrackerBlock.label` with the
`TrackerCatalogue` value type, first-fault validation order, the
`RepositoryProbe` blanket impl, the `PanicFold` decorator in
`vcs-adapters`, golden-borne release-fetch counts, per-phase golden checks
and the pinned, gated large-repository run. The author approved the plan
without a third pass. Five suggestions remain open by choice.
