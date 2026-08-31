---
type: "plan-review"
id: "2026-08-30-0220-untracked-remote-discovery-on-linear-review-1"
title: "Plan Review: Untracked-Remote Discovery on Linear"
date: "2026-08-30T17:48:07+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-08-30-0220-untracked-remote-discovery-on-linear"
target: "plan:2026-08-30-0220-untracked-remote-discovery-on-linear"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "usability"]
review_number: 1
review_pass: 3
tags: ["sync", "linear", "tracker", "discovery"]
last_updated: "2026-08-30T20:02:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Untracked-Remote Discovery on Linear

**Verdict:** REVISE

The plan is well-grounded — its `TeamResolver` port mirrors the established
`StateResolver` precedent, its `DiscoveryStatus` enum makes the previously silent
skip observable, and its two-phase merge-order claims hold under the real code.
It carries three confirmed defects that must be fixed before implementation: a
production `LinearClient::new` caller it never wires, a `render_report` change
that drops the `#\tsummary` line on empty runs, and a Phase 2 Red step that
cannot be written because the test double it targets has no search-failure seam.
Layered over these, five of six lenses independently flagged the same design
error: permanent config faults are labelled transient/retryable when an
`UNCONFIGURED` exit code already exists for exactly this case.

### Cross-Cutting Themes

- **Config faults mislabelled as transient** (flagged by: architecture,
  code-quality, usability, correctness) — an unset or unresolvable team key is
  returned as `TrackerError::Retryable` and then folded into the `RETRYABLE` (70)
  exit arm. Neither retries to success. `exit_codes::UNCONFIGURED` (74) already
  exists and is documented as "the tracker is wired but its configuration is
  missing … nothing was sent — fix the config"; verified in use at
  `sync.rs:381`. This is the single most-reinforced issue in the review.
- **`render_report` ordering drops the summary on empty runs** (flagged by:
  correctness, usability) — the plan pushes the discovery line before
  `lines.sort()` (`sync.rs:200`), which makes `lines.is_empty()` false, so the
  `if synced_count > 0 || lines.is_empty()` guard (`sync.rs:201`) stops emitting
  `#\tsummary\tsynced\t0`. Confirmed against the code — a real regression to the
  rendered output.
- **`LinearClient::new` caller set is under-enumerated** (flagged by:
  compatibility) — the plan wires only `from_config` and the test support file.
  A production caller, `build_with_override` at `cli/linear-cli/src/context.rs:182`,
  constructs `LinearClient::new` directly and is confirmed present. Following the
  plan's `FixedTeam` guidance there would resolve nothing and refuse every key.

### Tradeoff Analysis

- **Error-taxonomy purity vs the fixed `search` contract**: the port `search`
  contract forbids `Terminal` (a read never mutates), so a permanent config fault
  has no honest variant and is forced through `Retryable`. Architecture suggests
  evolving the read taxonomy; that is larger than 0220. The pragmatic resolution
  is to keep `Retryable` at the port but map it to `UNCONFIGURED` at the exit
  boundary, so the process signal is correct even while the type is a best-fit.

### Findings

#### Critical

None.

#### Major

- 🟡 **Compatibility**: Omitted production `LinearClient::new` caller
  **Location**: Phase 1, Change 5
  `build_with_override` (`cli/linear-cli/src/context.rs:182`) constructs
  `LinearClient::new` directly, not via `from_config`. Confirmed. Change 3 wires
  the resolver only into `from_config`, so this override/loopback discovery path
  would either fail to compile or, if given a `FixedTeam` per the plan's test
  guidance, silently refuse every key.

- 🟡 **Usability / Architecture / Code-Quality / Correctness**: Config faults
  routed to `RETRYABLE` when `UNCONFIGURED` (74) fits
  **Location**: Phase 2, Change 4; Phase 1, Change 4
  `Skipped(NoScopeKey)` and the resolution-failure `Failed` are folded into the
  `RETRYABLE` arm. `exit_codes::UNCONFIGURED` (74) exists and means exactly "fix
  the config, nothing was sent". Automation reading exit 70 is told to retry a
  fault only a config edit resolves.

- 🟡 **Test Coverage**: No `failing_search` seam blocks Phase 2 Red step 4
  **Location**: Phase 2, Test-Driven Steps, step 4
  `RecordingTracker` exposes `failing_update/create/preview/show` and
  `discovering`, but no search-failure seam (confirmed). The step asserting a
  search error yields `Failed` with `read_failure == None` cannot be written
  without adding one, and the plan's Changes touch only `run.rs`/`sync.rs`.

- 🟡 **Test Coverage**: The AC-8 end-to-end regression is narrative-only
  **Location**: Testing Strategy → Integration Tests; Phase 1 Red step 1
  Phase 1 Red step 1 is labelled "the AC-8 regression test" but only checks the
  client's UUID substitution, not the sync path producing a `create-from-remote`
  pull. The genuine end-to-end regression (AC-1/AC-4/AC-8) appears only in prose
  and in no phase's automated Success Criteria.

- 🟡 **Compatibility**: Stale exit-code documentation in the sync skill
  **Location**: Phase 2, Change 4
  `skills/work/sync-work-items/SKILL.md` documents exit 70 as "a read failed".
  Broadening 70 to cover a no-scope-key skip makes that doc wrong. No phase
  updates it. (Resolves toward the `UNCONFIGURED` remap above.)

- 🟡 **Compatibility**: Jira's no-project run newly exits non-zero
  **Location**: Phase 2, Change 2
  The `NoScopeKey` skip branch applies to both trackers; a Jira bidirectional run
  with no project configured exits 0 today and non-zero after. The plan frames
  the required-key/non-clean-exit decision as Linear-only. Confirm this is
  intended, or scope it to key-requiring trackers.

- 🟡 **Usability**: `Failed { detail }` double-wraps the `TrackerError` Display
  **Location**: Phase 2, Change 2 & 3
  `error.to_string()` on a `Retryable` renders as "tracker call failed with no
  remote change: E_SEARCH_…", so the failure line is triple-labelled and claims a
  remote change that never happened. A real network detail may carry tabs or
  newlines, breaking the single-line TSV. Render the inner detail and normalise
  whitespace.

#### Minor

- 🔵 **Correctness / Usability**: Discovery line placement is non-deterministic
  **Location**: Phase 2, Change 3
  Pushing before the sort sorts the `#\tdiscovery` line to the top, above item
  rows, not adjacent to the summary the wording implies — and drops the summary
  on empty runs (see theme). Push after the summary-guard block, or drop the
  `|| lines.is_empty()` guard.

- 🔵 **Architecture / Code-Quality**: Single-team identity now read from the
  catalogue in three places
  **Location**: Phase 1, Changes 2-3
  `CatalogueTeam` re-derives `/team/key` and `/team/id` that `team_key` and
  `credentials().team_id` already hold at construction. Consider building the
  resolver from the in-hand pair rather than a third disk reader.

- 🔵 **Code-Quality**: `pointer_string` and the `RunReport` builder extractions
  are left conditional
  **Location**: Phase 1 step 6; Phase 2 step 7
  Both are well-motivated (a third catalogue reader; a sixth field added to two
  duplicated `RunReport` literals). Commit to them rather than deferring to a
  maybe-refactor.

- 🔵 **Architecture / Code-Quality**: Two no-key guards, the inner effectively
  unreachable
  **Location**: Phase 2 Change 2 vs Phase 1 Change 4
  The gate short-circuits to `Skipped(NoScopeKey)` before `search`, so
  `E_SEARCH_NO_TEAM` is unreachable in the wired flow (`all_projects` is
  hardcoded `false`). Keep it as port defence, but state that it is secondary.

- 🔵 **Architecture**: Jira's `E_JQL_NO_PROJECT` guard becomes unreachable in the
  discovery path
  **Location**: What We're NOT Doing; Phase 2 Change 2
  AC-6 is phrased as a JQL refusal, but the gate now yields a generic
  `no-scope-key` skip before `search`. Reconcile AC-6's wording and keep a direct
  unit test on `compose`'s `E_JQL_NO_PROJECT`.

- 🔵 **Test Coverage**: AC-6 (Jira no-project) and AC-2 (bounding) have weak
  named coverage
  **Location**: Phase 2 Change 2; Phase 1 Red step 1
  No step names the Jira no-project test; AC-2's "no request enumerates the wider
  workspace" is asserted only positively (presence of the team constraint), so a
  second unbounded page would pass.

- 🔵 **Correctness**: Stated trim invariant is inaccurate
  **Location**: Assumptions; Phase 1 Change 2
  `CatalogueTeam::resolve` trims both sides; `in_scope` compares untrimmed
  (`prefix == key.as_str()`, confirmed). A whitespace-bearing catalogue key would
  make discovery and fetch-classification disagree. Correct the assumption text
  or align the comparisons.

- 🔵 **Usability**: `E_SEARCH_UNKNOWN_TEAM` states the symptom without a remedy
  **Location**: Phase 1 Change 4
  Its sibling names the fix ("set work.default_project_code"); this one does not.
  Add a next step (check the key matches the catalogue team key, or refresh
  `linear/catalogue.json`).

#### Suggestions

- 🔵 **Code-Quality**: `FixedTeam(BTreeMap)` over-models the single-team domain
  **Location**: Phase 1 Change 1
  The double expresses multi-team resolution the catalogue never has. Model it as
  a single `(key, id)` pair, or note the map is a deliberate `FixedStates` mirror
  holding one entry.

- 🔵 **Architecture**: `Failed { detail: String }` flattens the structured error
  **Location**: Phase 2 Change 1
  Carrying the `TrackerError` (or a small enum) would let the renderer and exit
  logic branch on kind from one source, rather than parallel `matches!` arms.

- 🔵 **Usability**: `ran\tfound=N` diverges from the bare-value `summary\t{count}`
  **Location**: Phase 2 Change 3
  Consider `#\tdiscovery\tran\t{found}` to mirror the summary line, or accept the
  label deliberately.

### Strengths

- ✅ `TeamResolver` faithfully mirrors the `StateResolver` port — trait +
  `FixedTeam` double + catalogue-backed impl injected as `Box<dyn …>` and built
  in `from_config` — keeping provider law in the client crate.
- ✅ The boolean gate plus empty-`else` becomes a single match producing both the
  untracked set and an explicit `DiscoveryStatus`, killing the skip-vs-empty
  ambiguity at its source; `Ran { found: 0 }` is now visibly distinct from a skip.
- ✅ Both two-phase merge-order claims verified against the code: Phase 1 alone
  folds a resolution failure into `read_failure` for a non-clean exit; Phase 2
  alone reports the raw-key trap honestly as `ran found=0`.
- ✅ Routing discovery-search errors off `read_failure` is confirmed non-lossy —
  its only work-cli consumer is the exit-code check, and fetch-originated values
  are untouched.
- ✅ The de Morgan rewrite of the discovery gate is logically equivalent to the
  original; no discovery case is silently dropped or newly enabled.
- ✅ `linear-client` is public-api-exempt (adapter), so the new `pub` items and
  the changed `new` signature need no snapshot update; the `tracker` port
  signature and the `issue-filter.txt` fixture are correctly untouched.
- ✅ Nearly every cited test idiom exists and is accurately described
  (`scoped()`, `discovering`, `client_for`/`TEAM_KEY`/`TEAM_ID`, `with_catalogue`,
  `server.bodies`, the filter-family coverage guard).

### Recommended Changes

1. **Wire the resolver into every `LinearClient::new` caller** (addresses:
   omitted production caller, further test sites) — enumerate the five confirmed
   sites: `support/client.rs` (×2), `contract.rs`, `sync_run_real_client.rs`, and
   `linear-cli/src/context.rs:182`. Inject `CatalogueTeam::load(integrations_root)`
   at `build_with_override` (mirroring `from_config`), and state the resolver each
   test site carries.

2. **Map config faults to `UNCONFIGURED` (74)** (addresses: exit-code theme,
   stale skill doc) — route `Skipped(NoScopeKey)` and the resolution-failure
   `Failed` to `exit_codes::UNCONFIGURED`, reserving `RETRYABLE` for transient
   search failures. Then update `skills/work/sync-work-items/SKILL.md`'s exit-code
   list and add the `#\tdiscovery` line to its report documentation.

3. **Fix the `render_report` ordering and summary guard** (addresses: summary
   drop, non-deterministic placement) — push the discovery line after the
   summary-guard block (or drop `|| lines.is_empty()` and emit the summary
   unconditionally), so the `#\tsummary` line survives empty runs and the
   `[items] / discovery / summary` order is deterministic.

4. **Add a `failing_search` seam and promote the AC-8 regression to a Red step**
   (addresses: both test-coverage majors) — add `failing_search(TrackerError)` to
   `RecordingTracker` as an explicit Phase 2 change, and add a red-first
   end-to-end test driving `run()` over a mocked Linear with a seeded untracked
   issue, asserting a `create-from-remote` row and `DiscoveryStatus::Ran
   { found >= 1 }`, wired into the phase's automated Success Criteria.

5. **Render the inner error detail, not the wrapped Display** (addresses: Failed
   line double-wrap) — put the `E_SEARCH_…` detail into the `failed` line and
   normalise tabs/newlines so it stays one TSV record.

6. **Confirm or scope the Jira exit-code change** (addresses: Jira non-zero exit)
   — state explicitly whether an unscoped Jira bidirectional run should newly exit
   non-zero, and add a Red step pinning `Skipped(NoScopeKey)` + no `Call::Search`
   for Jira, plus a direct `E_JQL_NO_PROJECT` unit guard.

7. **Commit the deferred extractions and correct the trim invariant** (addresses:
   conditional refactors, inaccurate assumption, minor smells) — commit to
   `pointer_string` and a `RunReport::from_prepared` builder; correct the
   Assumptions text on `in_scope` trimming; add a remedy to `E_SEARCH_UNKNOWN_TEAM`.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally well-grounded — mirrors the `StateResolver` port,
centralises the discovery decision into one match, and introduces rich domain
vocabulary. The dominant structural risk is the error model forcing a permanent
misconfiguration through `Retryable`; secondary concerns are catalogue cohesion
(single-team identity read in three places), Jira's deep guard becoming
unreachable, and the config-skip/transient-failure collapse in the exit arm.

**Strengths**:
- `TeamResolver` mirrors the `StateResolver` precedent, preserving the
  pure-port/adapter separation.
- The discovery decision is refactored into a single match producing both the set
  and an explicit `DiscoveryStatus`.
- `DiscoveryStatus::{Ran, Skipped, Failed}` is threaded end-to-end.
- Dropping the capability method and the credential-team fallback is explicitly
  reasoned and justified against the sole production caller.
- Resolution logic is pure and injectable via `FixedTeam`.

**Findings**:
- (major, medium) Permanent key-misconfiguration modelled as a `Retryable` error;
  the taxonomy cannot express a permanent read failure. Suggest a follow-up to add
  a config-fault/terminal-for-reads discriminant.
- (minor, medium) Single-team identity resolved from the catalogue in three places
  (`CatalogueTeam`, `team_key`, `credentials().team_id`); consider building the
  resolver from the already-resolved pair.
- (minor, medium) Jira's `E_JQL_NO_PROJECT` guard becomes unreachable in the
  discovery path; reconcile AC-6 wording and keep a direct `compose` unit test.
- (minor, medium) `NoScopeKey` (permanent) folded into the same `RETRYABLE` arm as
  transient failures; consider a distinct non-clean class.
- (suggestion, low) `Failed { detail: String }` flattens the structured
  `TrackerError` at the report boundary.

### Correctness

**Summary**: Core logic is sound — the rewritten gate is a correct de Morgan
transformation of the original, both merge-order claims hold under the real code,
and routing search errors off `read_failure` is non-lossy. Remaining concerns are
edge-case interactions and two under-specified semantics.

**Strengths**:
- The replacement gate is logically equivalent to the original boolean gate.
- Both merge-order claims verified against the code.
- Moving discovery-search errors off `read_failure` is verified non-lossy.

**Findings**:
- (minor, medium) The discovery line pushed before `lines.sort()` makes
  `lines.is_empty()` false, so an empty run no longer emits `#\tsummary\tsynced\t0`.
  Push after the summary guard or drop `|| lines.is_empty()`.
- (minor, medium) `Skipped(NoScopeKey)` and `Failed` folded into `RETRYABLE`;
  neither is transient. Map to a terminal-but-non-clean code or document.
- (minor, low) The stated trim invariant is inaccurate: `resolve` trims, `in_scope`
  compares untrimmed; a whitespace-bearing key would diverge.

### Code Quality

**Summary**: Well-structured, models the outcome as an explicit domain type, and
mirrors the `StateResolver` port. Concerns: a third near-duplicate catalogue
reader, a test double that over-models the single-team domain, the
retryable/exit-code categorisation, and two well-motivated refactors left soft.

**Strengths**:
- Replaces the silent empty-`else`/empty-`Vec` path with an explicit
  `DiscoveryStatus`, making the ambiguity unrepresentable.
- `TeamResolver` is consistent with an established, testable pattern.
- The search rewrite removes the two-kinds-of-value-in-one-field smell.
- No code comments introduced, consistent with the repo's low-comment convention.
- TDD steps are specified red-first with concrete assertions.

**Findings**:
- (minor, high) `CatalogueTeam::load` is the third near-identical catalogue reader;
  the `pointer_string` extraction is left conditional despite two uses. Commit to a
  shared helper.
- (minor, medium) `FixedTeam(BTreeMap)` models a multi-team map the domain never
  has; model the single-team shape or note the deliberate mirror.
- (minor, medium) Missing/unresolvable key surfaced as `Retryable` and folded into
  `RETRYABLE`; the observability signal points away from the config fix.
- (minor, medium) Two no-key guards (gate + `search`); the inner is effectively
  unreachable in the wired flow — state it is defensive.
- (suggestion, medium) A sixth field added to two duplicated `RunReport` literals;
  commit to a `from_prepared` builder now.

### Test Coverage

**Summary**: Unusually well-grounded in existing idioms — nearly every cited seam
exists and is accurately described, and the Phase 1 port-search Red steps are
genuine red-first tests. Two gaps undercut it: the Phase 2 read_failure Red step
cannot be written (no search-failure seam), and the headline end-to-end regression
is narrative-only. AC-6 also has no named test.

**Strengths**:
- Phase 1 Red steps 1-3 are genuine red-first tests; the port `search` has no
  direct coverage today and the error arms return `Ok` currently.
- Cited idioms are accurate (`scoped()`, `discovering`, `client_for` constants,
  `with_catalogue`, `server.bodies`, the filter-family guard).
- Phase 2 gives `DiscoveryStatus` thorough per-branch coverage plus render/exit
  assertions.

**Findings**:
- (major, high) No `failing_search` seam on `RecordingTracker`, blocking the
  read_failure Red step; add `failing_search(TrackerError)` as an explicit change.
- (major, medium) AC-8/AC-1/AC-4 end-to-end regression is narrative-only, not a Red
  step, and absent from Success Criteria; promote it.
- (minor, medium) AC-6 (Jira no-project) has no named test and the gate rewrite
  changes its outcome untested.
- (minor, low) AC-2's bounding guarantee is asserted only positively; a second
  unbounded page would pass.

### Compatibility

**Summary**: Two deliberate contract changes on the Linear discovery path and a
Phase 2 exit-code change that turns previously-clean no-scope runs non-zero. The
`Retryable`-vs-`Terminal` contract is honoured and `linear-client` is public-api
exempt, but the plan under-enumerates the `new` callers (omitting a production
composition root) and leaves the sync skill's exit-code docs stale.

**Strengths**:
- `linear-client` is public-api exempt, so the new items and changed signature
  need no snapshot update; the plan correctly omits one.
- The `tracker` crate's `search` signature/doc stay unchanged.
- Both refusal errors return `Retryable`, honouring the port contract.
- The `issue-filter.txt` fixture is unaffected — resolution sits upstream of
  `compose`.

**Findings**:
- (major, high) `build_with_override` (`cli/linear-cli/src/context.rs:182`)
  constructs `LinearClient::new` directly and is not wired; a `FixedTeam` there
  would refuse every key. Inject `CatalogueTeam::load`.
- (major, high) Exit 70 is documented narrowly in
  `skills/work/sync-work-items/SKILL.md:112`; the plan updates no skill doc.
- (major, medium) The `NoScopeKey` non-clean exit applies to Jira too — an
  unscoped Jira bidirectional run now exits non-zero. Confirm or scope it.
- (minor, medium) Two further `new` sites (`contract.rs:146`,
  `sync_run_real_client.rs:179`) get no resolver guidance; enumerate all five.

### Usability

**Summary**: The central goal — distinguishing an invisible skip from an empty
search — is well served by the four `#\tdiscovery` lines and `Ran { found=0 }`.
The weak spots are the error-exit semantics (a config fault mapped to `RETRYABLE`
when `UNCONFIGURED` (74) exists) and the failure line double-wrapping the
`TrackerError` Display.

**Strengths**:
- The `Ran/Skipped/Failed` split directly kills the invisible failure.
- The lines reuse the established `#\t<category>\t<key>\t<value>` sentinel shape.
- `E_SEARCH_NO_TEAM` names the exact config field, repeated in the report line.

**Findings**:
- (major, high) `Skipped(NoScopeKey)`/`Failed` mapped to `RETRYABLE`; the
  `UNCONFIGURED` (74) code matches these cases exactly. Remap.
- (major, high) `Failed { detail: error.to_string() }` double-wraps the `Retryable`
  Display into a misleading, triple-labelled line; a real detail with tabs/newlines
  breaks the TSV. Render the inner detail and normalise whitespace.
- (minor, medium) `E_SEARCH_UNKNOWN_TEAM` states the symptom without a remedy,
  unlike its sibling.
- (minor, medium) `lines.sort()` does not order discovery vs summary as the plan
  claims; a pre-sort push drops the summary on empty runs and lands the line above
  item rows. Insert explicitly.
- (suggestion, low) `ran\tfound=N` diverges from the bare-value `summary\t{count}`
  in the same column.

## Re-Review (Pass 2) — 2026-08-30T19:18:15+00:00

**Verdict:** REVISE

All six lenses re-ran against the revised plan. Every Pass 1 finding is resolved
and confirmed against the code. The re-review surfaced one convergent new major —
introduced by the Pass 1 exit-code fix — that four lenses flagged independently,
plus a test-coverage major and assorted minors. The plan was then revised again
(the pre-flight redesign below) to address them; that redesign has not itself been
re-reviewed.

### Previously Identified Issues

- ✅ **Compatibility**: Omitted production caller `context.rs:182` — Resolved (grep confirms all five sites enumerated).
- ✅ **Test Coverage**: `failing_search` seam missing — Resolved (mirrors `failing_show`, implementable).
- ✅ **Test Coverage**: AC-8 regression narrative-only — Resolved (now a Red step; rationale further corrected this pass).
- ✅ **Usability**: `Failed` double-wraps Display — Resolved (inner detail + `single_line`).
- ✅ **Correctness/Usability**: render ordering / summary drop — Resolved (correctness confirms the reorder is correct).
- ✅ **Compatibility**: stale SKILL.md exit-code doc — Resolved (Change 6; broadened this pass to both doc sites).
- ✅ All Pass 1 minors (E_SEARCH remedy, AC-6 test, AC-2 negative, `pointer_string`, `RunReport` builder, `FixedTeam` note, trim invariant) — Resolved.

### New Issues Introduced

- 🔴 **Usability / Architecture / Correctness / Compatibility**: exit 74 for the no-key skip breaks its "nothing was sent" invariant — a bidirectional no-key run pushes before exiting 74; 74 is a pre-run-abort-only code. Convergent, high/medium confidence. **Addressed** by the pre-flight redesign: config faults now refuse in `prepare_run` before the push (`RunError::DiscoveryUnconfigured` → 74, nothing sent); transient failures → 70.
- 🟡 **Test Coverage**: the AC-8 e2e "issue never surfaces" rationale is invalid against a `MockServer` (which does not evaluate the team filter). **Addressed** — the load-bearing red assertion is now the captured-body UUID, and the `execute` harness is noted to need a scope/direction parameter.
- 🔵 **Code Quality**: shared gate message said "team key" (Linear-specific) for a Jira run. **Addressed** — each tracker's `resolve_scope` now owns its own message (Jira: `E_JQL_NO_PROJECT`).
- 🔵 **Code Quality**: `pointer_string` empty-string contract unspecified. **Addressed** — mirrors `catalogue_field`'s non-empty filter.
- 🔵 **Compatibility**: unscoped Jira exit `0 → 74` is a contract change — flag in release notes (still applies; now via the pre-flight refusal).

### Assessment

The redesign the user chose — validate the discovery scope pre-flight via a new
`resolve_scope` port method and refuse an invalid one before any push — resolves
the convergent exit-code major at its root rather than papering over it: 74 is now
honest ("nothing was sent"), 70 is reserved for genuine transient failures, and no
discriminant or new exit code is needed. It expands the plan (a public-api-pinned
port method, a new `RunError` variant, five `resolve_scope` impls) and reverses the
plan's former "no hard-abort" stance. A confirming review pass over the revised
Phases 1-2 is warranted before implementation, focused on the new port method and
the pre-flight control flow.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-08-30T20:02:07+00:00

**Verdict:** APPROVE

A confirming pass over the pre-flight redesign ran all six lenses. The redesign's
core is endorsed across lenses (clean `resolve_scope`/`search` split, honest 74
before apply, no exit-code double-count). The pass surfaced one design major
converged by two lenses, three high-confidence contract/test majors, and several
minors — all now addressed in-plan. One item is an explicitly accepted product
tradeoff rather than a fix (below).

### Previously Identified Issues (Pass 2)

- ✅ All Pass 2 findings resolved and confirmed against the code (the exit-74
  invariant, render ordering, `Failed` double-wrap, caller enumeration, seams).

### New Issues Introduced (Pass 3) — resolution

- 🔴 **Compatibility/Correctness**: dropping Linear `search`'s credential fallback turned a `None` scope into an unbounded workspace **flood** (not "empty"). **Fixed** — `search` keeps a defensive `None` refusal (`E_SEARCH_UNRESOLVED_SCOPE`), mirroring Jira's deep guard; the rationale is corrected.
- 🔴 **Compatibility**: the public-api regen command was fabricated (`cargo test --test public_api`). **Fixed** — now `mise run public-api:update` / `:check`.
- 🔴 **Test Coverage**: a sixth `RemoteTracker` impl (`MarkerObservingTracker`) was omitted; a required method breaks compilation. **Fixed** — all six impls enumerated in Change 4.
- 🔴 **Test Coverage**: the five existing `sync_run_real_client` tests break under the unconditional pre-flight refusal (`execute` panics). **Fixed** — Change 8 migrates the harness (a `Result` return + `scope`/`direction` params) and every caller.
- 🔴 **Architecture / Code-Quality**: the config-vs-transient fault class was carried by call-site position, risking a `for_tracker_error` misroute. **Fixed** — a dedicated `ScopeError` type carries the config fault; the `resolve_scope` map no longer destructures `TrackerError`.
- 🟡 **Usability**: the refusal never surfaced the `--push-only` escape hatch, and `E_SEARCH_*` sentinels leaked into the operator message. **Fixed** — messages name `--push-only`; the render arm gets a "refused:" framing.
- 🟡 **Compatibility**: the unscoped-Jira `0→74` shift and 74's broadened meaning needed capture. **Fixed** — a Migration Notes release-note line, cross-referencing 0146; the exit_codes.rs band header and both SKILL.md sites updated.
- 🔵 Minors — `pointer_string` empty filter, `TrackerError::into_detail` accessor, resolve_scope-vs-write-bound precedence, fail-fast ordering vs `fetch::gather`, AC-7 observability split — all noted or applied.

### Accepted Tradeoff (not a fix)

- 🟡 **Usability/Correctness (high confidence)**: the pre-flight hard-abort refuses the *whole* run — blocking the tracked-item push/pull for a discovery-only misconfiguration; pull-only is also blocked. The user chose this deliberately over the soft-degrade alternative, accepting that a keyless bidirectional/pull-only run refuses (with `--push-only` as the escape hatch) in exchange for a clean 74/70 taxonomy. Documented in "What We're NOT Doing" and Migration Notes.

### Assessment

The design has converged and is implementation-ready. Every finding across three
passes is resolved except the whole-run-abort behaviour, which is a deliberate,
now-documented product decision rather than a defect. The Pass 3 fixes are targeted
and code-verified (public-api tasks, the six impls, the flood mechanism, the write
bound all checked against the tree) but were not themselves put through a fourth
lens pass; a final skim of the `resolve_scope`/`ScopeError` port change during
implementation is the only residual diligence.

---
*Re-review generated by /accelerator:review-plan*
