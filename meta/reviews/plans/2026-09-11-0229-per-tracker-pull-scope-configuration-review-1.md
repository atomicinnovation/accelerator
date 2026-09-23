---
type: "plan-review"
id: "2026-09-11-0229-per-tracker-pull-scope-configuration-review-1"
title: "Plan Review: Per-Tracker Pull Scope Configuration"
date: "2026-09-12T00:04:25+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-11-0229-per-tracker-pull-scope-configuration"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "compatibility", "security", "usability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-09-18T10:49:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Per-Tracker Pull Scope Configuration

**Verdict:** REVISE

The plan is architecturally well-grounded and genuinely test-driven: it finds real
seams (the dormant `SearchScope.filters`, the per-key `get` that yields whole-block
replacement for free), keeps the tracker port entity-neutral, slices eight
independently-mergeable phases in a defensible order, and its central safety claim
("no sync path deletes a work-item file today") was verified true against the
decision table. But two defects break stated behaviour as written — the configured
`max_items` is dead on arrival behind the `--max-pulls` clap default, and Phase 4's
headline truncation→abort is not implementable on either keyed path because neither
distinguishes truncation from a transient failure — and a dense band of `major`
findings (adapter-lowering duplication, a config-validation gap on the sync path, a
non-total ordering comparator, and a `max_items` domain/implementation mismatch)
need resolving before implementation. Fixable with targeted plan edits, not a
rewrite.

### Cross-Cutting Themes

- **`--max-pulls` clap default shadows configured `max_items`** (flagged by:
  compatibility, correctness, usability) — `--max-pulls` is `#[arg(default_value_t =
  25)]`, so clap always supplies `25` and cannot tell "unset" from "passed 25". As
  written the config ceiling can never take effect and the "`max_items: 3` refuses a
  5-issue discovery whereas the default 25 completes" acceptance criterion cannot
  pass. High confidence across three lenses; a small, well-understood fix
  (`Option<usize>`, resolve in the layer below clap).
- **No truncation-vs-transient signal on the keyed reconcile paths** (flagged by:
  correctness, safety, test-coverage, architecture) — Phase 4 promises a clean split
  (truncation → abort, transient → retry, absent → noop, out-of-scope-indeterminate →
  proceed), but Jira's `fetch_chunk` returns one stringly-typed `Err(String)` for both
  cap-hit and transient failure, Linear's `page_all` collapses cap-hit/deadline/wire
  failure into one `truncated: Option`, `FetchOutcome` carries no completeness flag,
  and the named test seam (`RecordingTracker::truncating`) produces indeterminate ids
  indistinguishable from a genuine out-of-scope indeterminate — and collides with the
  existing `an_indeterminate_items_watermark_is_left_unadvanced` test. The abort is
  neither correctly targetable nor testable without a new typed signal the plan does
  not specify.
- **Scope-lowering re-implemented twice with no shared abstraction** (flagged by:
  code-quality, architecture) — the enumerate-visible-set / confirm-membership /
  abort-vs-transient / emit-`IN`-list policy is built independently in `jira-client`
  and `linear-client` across Phases 5–7. The safety-relevant categorisation (unseen
  entity → abort, network blip → transient) is exactly the part two hand-maintained
  copies will drift on.
- **Config validation does not gate the sync read path** (flagged by: security,
  usability, correctness) — validation is wired only into `configure`/`dump`; the sync
  path uses the pure `parse` (which "does not itself validate acceptance") and lowers
  straight through. Security frames it as JQL key injection from a hand-edited/PR'd
  team config reaching raw JQL; usability as unrecognised/wrong-tracker keys silently
  dropped; correctness as a malformed non-mapping personal block shadowing a valid team
  block. Same root: the trusted-input boundary is enforced at one entry point only.
- **`max_items` domain/implementation mismatch** (flagged by: correctness,
  architecture) — `max_items` is documented as bounding "the discovered-issue count",
  but it defaults `max_pulls`, enforced against `plan.pull_count() + untracked.len()`
  (tracked pull updates *plus* discovered). A configured `max_items: 3` can refuse when
  only one item was newly discovered. Whether the count is pre- or post-dedup is also
  unspecified.
- **Comparator is not a total order** (flagged by: correctness, test-coverage) —
  `ap.cmp(&bp).then(an.cmp(&bn))` returns `Equal` for distinct ids sharing prefix and
  sequence (e.g. zero-padded `PP-02` vs `PP-2`, which survive dedup), with no raw-id
  tie-break; no-digit and integer-overflow cases are undefined. Violates the "total,
  deterministic" guarantee.

### Tradeoff Analysis

- **Fail-loud vs availability (Phase 4 keyed read)**: the plan deliberately trades
  availability for fail-loud, and that is a reasonable direction. But safety and
  architecture agree the plan under-describes the blast radius: the keyed read runs in
  the shared `fetch::gather`/`prepare_run` path feeding *both* pull and push, so one
  truncated chunk aborts the whole run — including queued pushes — which crosses the
  "strictly pull-side" boundary; and because it pages the tracked corpus at the fixed
  default, a corpus that outgrows `max_pages` fails on *every* subsequent sync, a
  persistent stall for unattended runs. Recommendation: keep the fail-loud intent, but
  scope the abort to the pull/discovery decision, name `pull.<tracker>.max_pages`
  (and the page count) at the point of failure, and consider a `--push-only` bypass.
- **Whole-workspace power vs least-privilege (`all_* + unlimited`)**: security, safety,
  and architecture each note that `all_projects: true` + `max_items: unlimited` +
  `max_pages: unlimited`, settable from *shared* team config, removes every flood-guard
  — unbounded enumeration into memory plus mass local-file creation, with only the
  transport deadline as backstop. Recovery is cheap (VCS-tracked files), so this is a
  documented-footgun concern, not a blocker. Recommendation: call the combination out
  explicitly and consider a preview/confirmation nudge or a soft cap.

### Findings

#### Critical

- 🔴 **Compatibility / Correctness / Usability**: `--max-pulls` clap default (25)
  permanently shadows the new `pull.<tracker>.max_items` config default
  **Location**: Phase 3 §3 (Source the reconcile bound from config)
  `--max-pulls` is `#[arg(long, default_value_t = 25)]` and wired straight through as
  `max_pulls: args.max_pulls`, so clap always supplies a concrete `usize` — "unset" is
  indistinguishable from "passed 25". As written the config ceiling is dead code and
  the `max_items: 3` acceptance criterion cannot pass. (Agents rated this `major` at
  high/medium confidence across three lenses; elevated here because it renders a stated
  AC unimplementable.) Fix: make the flag `Option<usize>` with no clap default and
  resolve `None` → config `max_items` → `Bounded(25)` in the resolution layer; state
  the arg change explicitly in Phase 3.

- 🔴 **Correctness / Safety / Test Coverage / Architecture**: Phase 4's keyed-read
  truncation→abort is neither correctly targetable nor testable — no
  truncation-vs-transient signal exists
  **Location**: Phase 4 §2 (Keyed reconcile reads)
  Jira `fetch_chunk` returns one `Err(String)` for both the page-cap hit and every
  transient failure, folded into `indeterminate`; Linear `page_all` collapses
  cap-hit/deadline/wire failure into a single `truncated: Option` and separately routes
  an out-of-scope id into `indeterminate`. `FetchOutcome` has no completeness flag, so
  a naive abort fires on transient blips (converting graceful degradation into a total
  sync outage) and on legitimate out-of-scope indeterminates, and the `RecordingTracker::truncating`
  seam cannot express the distinction — it collides with the existing
  `an_indeterminate_items_watermark_is_left_unadvanced` test. (Agents rated `major`
  high-confidence across four lenses; elevated because Phase 4's headline behaviour is
  not implementable as specified.) Fix: add a plan step introducing a typed truncation
  signal on `fetch_chunk`/`page_all` (and likely a `FetchOutcome`/port change,
  budgeted for a `tracker` snapshot regen), preserve the out-of-scope-indeterminate
  non-aborting branch, and name a distinct `RecordingTracker` seam plus the update to
  the colliding test.

#### Major

- 🟡 **Code Quality / Architecture**: multi-scope resolution policy duplicated across
  both adapters with no shared home
  **Location**: Phases 6–7 §2 (Adapters emit enumerated `IN` list with sync-time
  resolution)
  Extract the entity-neutral parts (visible-set resolution, base∪additional membership,
  enumerated-list construction, abort-vs-transient classification) into one shared unit
  the two adapters call, leaving only JQL/`IssueFilter` emission tracker-specific.

- 🟡 **Security**: JQL key injection — configure-time key allow-list does not gate the
  sync read path
  **Location**: Phase 1 §3 / Phase 5 §3
  Jira interpolates the filter field name raw into JQL (`jql.rs:213-215,232`); a
  key like `label) OR project = SECRET OR (label` from a hand-edited or PR-injected
  team config bypasses `configure` validation and reaches raw JQL. Run the same
  `validate` on the sync read path before lowering, or allow-list field names in the
  Jira lowering defensively.

- 🟡 **Security**: new `project IN (...)` list not stated to reuse `quote()` escaping
  **Location**: Phase 6 §2 / Phase 7 §2
  The base clause is `project = <quoted>`, but the new multi-value `IN` list has no
  stated escaping; `all_*` entities come straight from remote enumeration (no
  allow-list). State that every entity passes through `quote()` and add a fixture
  golden for an entity containing a quote/paren.

- 🟡 **Correctness / Architecture**: `max_items` bound onto `max_pulls`, which counts
  tracked pulls plus discovered — not the "discovered-issue count"
  **Location**: Phase 3 §3 / Desired End State
  `max_pulls` is enforced against `plan.pull_count() + untracked.len()`. Either restate
  `max_items` as bounding total pull writes, or add a discovery-count-only check at
  `untracked.len()` distinct from the write bound; specify pre-/post-dedup counting.

- 🟡 **Correctness / Test Coverage**: ordering comparator is not a total order over
  distinct identifiers
  **Location**: Phase 8 §2 (Total ordering comparator)
  Tie-break on the raw id after `(prefix, sequence)`; define no-digit and
  overflow behaviour (saturating/String fallback); ensure the ordering agrees with the
  `canonical_external_key` dedup canonicalisation. Add degenerate-id test cases.

- 🟡 **Architecture / Safety**: Phase 4 abort blast radius spans the whole run
  (blocks pushes) and becomes a persistent block past `max_pages`
  **Location**: Phase 4 §2 (⚠️ availability tradeoff)
  The keyed read feeds both pull and push planning inside `prepare_run`; a truncated
  chunk aborts queued pushes too, crossing the "pull-only" boundary, and recurs on
  every sync once the corpus outgrows `max_pages`. Scope the abort narrowly, name the
  release valve at the failure point, consider a `--push-only` bypass.

- 🟡 **Code Quality**: `SearchScope` accretes four overlapping scope fields with
  invariants enforced only at the construction site
  **Location**: Phase 6 §1 / Phase 7 §1
  `project` / `all_projects` / `additional` / `filters` leave illegal combinations
  representable. Consider a sum type (`Keyed { base, additional } | WholeWorkspace`)
  or at least a single guarded constructor.

- 🟡 **Code Quality**: `PullConfig` parser/validator placed in the pure `work` domain
  crate, coupling it to `config::Value` and a homeless `FilterSchema`
  **Location**: Phase 1 §2 / Phase 2 §§1–2
  `work` today has no `config` dependency and is documented as pure decision logic; the
  `FilterSchema` *type* has no declared owner (only the const instances live in the
  clients, risking an inverted domain→client dependency). Define `FilterSchema` in a
  low shared crate (e.g. `tracker`) and reconsider whether the config-bridging parse
  belongs in `work-cli`/`work-adapters`.

- 🟡 **Test Coverage**: OR-within-a-key grouping pinned above and below the mapping,
  but not through it
  **Location**: Phase 5 (Automated Verification)
  All three named tests sit off the `filters → Family/Search` grouping seam; the exact
  bug being fixed (one value per family → two AND'd clauses) would pass every one. Add
  a client-level contract test asserting the emitted JQL/GraphQL carries `label IN
  ('a','b')` / `labels: { name: { in: [...] } }`.

- 🟡 **Test Coverage**: `max_pages` config-sourcing and the `Ceiling::Unlimited`
  transport branch are unreachable through the page-less fake
  **Location**: Phase 3 (Automated Verification)
  `RecordingTracker` models no pagination. Split the criteria by layer: a
  client/transport test that `Unlimited` never truncates a beyond-default page set, and
  a wiring test that a configured `pull.max_pages` actually reaches the constructed
  `TransportConfig` (not `::default()`).

- 🟡 **Usability**: validation does not reject unknown top-level or wrong-tracker block
  keys (typo footgun)
  **Location**: Phase 1 §2 / Phase 2 §2
  `max_item:`, `additional_project:`, or `additional_teams` under active Jira parse to
  defaults and are silently dropped — the quiet misconfiguration the fail-loud
  philosophy exists to prevent. Reject unrecognised and wrong-tracker keys at
  `configure`, naming the offending key and the accepted set.

- 🟡 **Usability**: whole-block replacement footgun is not surfaced in `dump`
  **Location**: Phase 1 §4
  The flat `Key | Value | Source` dump shows only the effective result; a personal
  block silently dropping team `filters` leaves no trace. Emit an explicit annotation
  when a personal `pull` block shadows a present team block.

- 🟡 **Usability**: rejection error-message actionability underspecified versus the
  `bad_integration` precedent
  **Location**: Phase 2 §2
  The codebase already names the bad value, the allowed set, the file, and a fix
  command. Require each `pull`-block branch to follow that shape rather than leaving it
  to implementation.

#### Minor

- 🔵 **Architecture**: `Ceiling` type placement adds an engine→transport production
  dependency and is left unresolved (Phase 3 §1) — prefer `tracker` or a dedicated
  crate over `tracker-support`.
- 🔵 **Compatibility**: `Ceiling` must derive `Copy/Eq/PartialEq/Clone/Debug` or it
  silently drops `TransportConfig`'s pinned `Copy`/`Eq` (Phase 3 §1).
- 🔵 **Compatibility**: non-`#[non_exhaustive]` `SearchScope`/`TransportConfig` struct
  literals fan out to ~10 and ~15 sites the plan under-scopes (Phases 3, 6) — enumerate
  the constructor sites (both `resolve_scope` returns, test seeds).
- 🔵 **Compatibility**: Linear `in`-operator support on `team.id`/`labels.name`/`state.id`
  is assumed, not verified against the live schema (Phases 5–7) — golden fixtures can't
  catch a wire-format mismatch; add a contract check.
- 🔵 **Correctness**: whether `max_items` counts pre- or post-dedup is unspecified
  (Phase 8 §1 / Phase 3 §3).
- 🔵 **Correctness**: `parse` totality claimed "over shape" but only the mapping case is
  specified; a malformed scalar personal block shadows a valid team mapping (Phase 1
  §§2–3) — define and test the non-mapping case as a structural rejection.
- 🔵 **Correctness**: `max_pages` pre-empts `max_items`, so "`all_*` bounded by
  `max_items`" is only reachable when `max_pages: unlimited` (Phase 7 criteria) —
  document the interaction.
- 🔵 **Correctness**: `Unlimited` requires rewriting the four `1..=cap` paging loops as
  unbounded, and deadline expiry is folded into the `truncated` signal — Phase 4
  guidance would misattribute a transient timeout to `max_pages` (Phase 3 §1 / Phase 4
  §1).
- 🔵 **Safety**: `all_* + unlimited` disables every flood-guard for unattended bulk
  create (Phase 7 / Phase 3) — consider a preview nudge or soft cap.
- 🔵 **Safety**: grow-on-pull catalogue write has unstated atomicity relative to the
  item imports (Phase 6 §3) — specify write ordering so an interrupted pull can't leave
  the index inconsistent.
- 🔵 **Security**: `all_* + unlimited` is a maximally-broad, semi-trusted-config-controlled
  scope lever (Phase 7 / Phase 3) — consider a hard safety ceiling or personal-config
  opt-in.
- 🔵 **Security**: committed-catalogue growth under `all_*` can commit workspace metadata
  for unrelated projects into VCS (Phase 6 §3) — grow only for entities actually
  imported; note in Migration Notes.
- 🔵 **Code Quality**: ceiling default magic numbers (20 / 25) scatter across
  construction sites (Phase 3 §§2–3) — keep each in one named constant.
- 🔵 **Code Quality**: ceiling tokens validated raw in Phase 2 then re-parsed to
  `Ceiling` in Phase 3 — two interpretations of one lexeme; parse once into `Ceiling`.
- 🔵 **Code Quality**: `split_prefix_sequence` duplicates the existing `{ key, number }`
  identifier parse (Phase 8 §2) — reuse or thinly wrap it.
- 🔵 **Test Coverage**: `Mapping`-variant match arms (`as_string_sequence`,
  `render_value`) and per-tracker noun normalisation are under-specified (Phase 1) —
  name explicit tests for both.
- 🔵 **Test Coverage**: ceiling-token validation classes collapsed into one "bad
  ceiling" case (Phase 2) — enumerate accept-`0`, accept-`unlimited`, reject
  negative/float/non-numeric.
- 🔵 **Test Coverage**: the `all_*`-clears-base-`project` construction invariant is not
  pinned by a named test (Phase 7 §1).
- 🔵 **Usability**: `max_items: 0` means refuse-all, the opposite of the common
  `0`-means-unlimited convention (Phase 3) — contrast `0` and `unlimited` in docs and
  the resolved `dump`.
- 🔵 **Usability**: truncation guidance names bare `max_pages` without the config path
  or the release valve (Phase 4 §1) — name `pull.<tracker>.max_pages`, the file, and
  that `unlimited`/higher lifts the cap.

#### Suggestions

- 🔵 **Architecture**: extract a shared scope-lowering abstraction to prevent
  cross-tracker semantic drift as the model grows (Phases 5–7). (Overlaps the
  duplication `major`; recorded once above.)
- 🔵 **Architecture**: note the transport deadline as the residual guard for the
  fully-`unlimited` `all_*` case and confirm O(n) in-memory accumulation is acceptable
  for the largest intended workspaces (Phase 7 / Performance Considerations).
- 🔵 **Code Quality**: `FilterSchema.required` is a speculative, unread slot — drop it
  until a tracker needs it, or exercise it with a rejection branch (Phase 2 §1).
- 🔵 **Usability**: render an unset-but-available `pull.<tracker>` placeholder in `dump`
  so the feature is discoverable before it is configured (Phase 1 §4).

### Strengths

- ✅ Phasing is genuinely vertical and dependency-ordered — Phase 1 lands the
  config-read foundation, ceilings/fail-loud precede scope-broadening, dedup/ordering
  land last; each phase independently mergeable and CI-green.
- ✅ The central safety claim is verified true: both `Indeterminate` and `RemoteAbsent`
  decide `Action::Noop` (`decide.rs:92-94`), `gather` only marks `Absent` on a
  provably-complete retrieval, and the only `remove_file` calls delete pending-push
  marker sidecars — so Phase 4 changes abort-vs-proceed, not deletion safety, and the
  zero-write / hold-all-watermarks guarantee is structurally sound.
- ✅ The `Value::Mapping` impact analysis is accurate — `Value` is `#[non_exhaustive]`,
  cross-crate matches are compiler-forced to a wildcard arm, and the three named
  in-crate sites (`project`, `as_string_sequence`, `render_value`) are exactly the ones
  that change; a purely additive, forward-compatible broadening.
- ✅ Config-file backward compatibility is explicitly and correctly preserved — absent
  block → base-only discovery, unset ceilings → `Bounded(25)/Bounded(20)`, no
  migration, lazy additive index growth.
- ✅ Whole-block replacement reuses the existing per-key `get` resolution (verified at
  `service.rs:476-492`) with no new merge logic; ordering is kept a reconciliation-local
  free comparator without bolting `Ord` onto the `ExternalId` port.
- ✅ Idiomatic, mutation-resistant test instincts — differential ceiling tests
  (configured value vs built-in default), the row-coverage-guarded fixture goldens, the
  black-box config `Fixture` with `--fail-safe` non-degradation twins mirroring the
  `work.integration` precedent, and named regression guards.
- ✅ Modelling ceilings as a named `Ceiling { Bounded, Unlimited }` value object (over an
  ambiguous `0`) and choosing tracker-native config vocabulary over an entity-neutral
  port are sound domain-modelling calls.

### Recommended Changes

1. **Make `--max-pulls` `Option<usize>` and resolve the ceiling below clap**
   (addresses: the `--max-pulls`-shadows-`max_items` Critical). No clap default;
   `None` → config `max_items` → `Bounded(25)`. Call the arg change out in Phase 3 and
   confirm the `max_items: 3` AC now exercises the config value.

2. **Add a truncation-vs-transient signal to the keyed reconcile paths before wiring
   the Phase 4 abort** (addresses: the Phase 4 signal-gap Critical). Introduce a typed
   cap-hit signal on `fetch_chunk`/`page_all` distinct from `TrackerError`, keep the
   out-of-scope-indeterminate branch non-aborting, budget any `FetchOutcome`/`tracker`
   snapshot change, and specify a new `RecordingTracker` seam plus the update to the
   colliding `an_indeterminate_items_watermark_is_left_unadvanced` test. Add a criterion
   proving a transient keyed-read failure still degrades.

3. **Run structural validation (and key allow-listing) on the sync read path, not only
   `configure`** (addresses: JQL key injection, unknown/wrong-tracker keys, malformed
   non-mapping block). Reject unrecognised top-level and wrong-tracker keys; define
   `parse` for `Scalar`/`Sequence` as a structural rejection.

4. **Scope the Phase 4 abort to the pull/discovery decision and make the release valve
   discoverable** (addresses: blast radius blocks pushes / persistent block). Confirm
   the run-wide scope is intended for bidirectional runs or gate on pull-affecting
   reads; name `pull.<tracker>.max_pages`, the page count, and the file at the failure
   point; consider a `--push-only` bypass.

5. **Reconcile `max_items` naming with the enforced count** (addresses: the
   domain/implementation mismatch). Either restate it as bounding total pull writes, or
   add a discovery-count-only check; specify pre-/post-dedup semantics and cover the
   duplicate-collapsing case.

6. **Make the ordering comparator total and defined at the edges** (addresses: the
   non-total-order `major`). Raw-id tie-break after `(prefix, sequence)`; define
   no-digit and overflow behaviour; align with the dedup canonicalisation; add
   degenerate-id cases.

7. **Extract the shared scope-lowering / entity-resolution helper** (addresses: adapter
   duplication). One entity-neutral unit for visible-set resolution, membership,
   enumerated-list construction, and abort-vs-transient classification; adapters supply
   only their enumeration call and string emission.

8. **Resolve the placement questions and derive-set** (addresses: `Ceiling` placement,
   `FilterSchema` home, `Ceiling` derives). Choose a neutral home for `Ceiling` (avoid
   the engine→`tracker-support` production edge), define `FilterSchema` in a low shared
   crate, and derive `Copy/Eq/PartialEq/Clone/Debug` on `Ceiling`.

9. **Raise error-message and dump discoverability to the `bad_integration` bar**
   (addresses: error actionability, whole-block-replacement footgun, `0`-vs-`unlimited`).
   Name value/accepted-set/file in every branch; annotate a shadowed team block in
   `dump`; contrast `0` (refuse-all) and `unlimited`.

10. **Verify Linear `in`-operator schema support and enumerate the struct-literal
    fan-out** (addresses: assumed protocol support, under-scoped mechanical changes).
    Add a Linear contract check for `in` on the targeted fields; list the `SearchScope`
    / `TransportConfig` constructor sites in Phases 3 and 6.

## Per-Lens Results

### Architecture

**Summary**: Architecturally well-grounded — real seams, entity-neutral port,
dependency-ordered vertical phases. Principal risk is Phase 6 placing live-enumeration
I/O in the discovery path without reconciling the abort-vs-transient distinction with
the pinned port's `ScopeError` (pure, hard) and `TrackerError` (soft, transient)
channels; secondary concerns are the Phase 4 keyed-read abort's run-wide blast radius
(crossing the pull-only boundary) and unresolved `Ceiling` placement / `max_items`
folding.

**Strengths**:
- Vertical, dependency-ordered phasing; each independently mergeable and CI-green.
- Reuses established engine predicates (per-key `get` for whole-block replacement,
  `canonical_external_key` for dedup) rather than inventing parallel ones.
- Ordering kept reconciliation-local; port surface stays minimal.
- `Value::Mapping` broadening additive and cross-crate-safe under `#[non_exhaustive]`,
  verified against source.
- Preserves entity-neutral port while exposing tracker-native config vocabulary; the
  major tradeoffs are explicitly acknowledged.

**Findings**:
- 🟡 (major, medium) Live-enumeration resolution has no clean abort-vs-transient error
  channel on the pinned port (Phase 6 §2). `resolve_scope` returns `ScopeError` (pure,
  hard config fault); `search` returns `TrackerError` whose read-applicable variant is
  soft/`Retryable` mapped to `DiscoveryStatus::Failed`. A membership miss needs a hard
  abort, a transient a retry — neither channel fits, and both are public-API-pinned yet
  Phase 6 budgets only the additive `additional` field.
- 🟡 (major, medium) Keyed-read fail-loud abort escalates a pull concern into a
  sync-wide abort that also blocks pushes (Phase 4 §2). The keyed read runs in the
  shared `fetch::gather` path feeding both pull and push planning inside `prepare_run`;
  one truncated chunk aborts queued pushes, crossing the "pull-only" boundary.
- 🔵 (minor, medium) `Ceiling` placement in `tracker-support` introduces a new
  production dependency from the reconcile engine onto the transport-bounds crate
  (Phase 3 §1); prefer `tracker` or a dedicated crate.
- 🔵 (minor, medium) `max_items` (discovered issues) folded onto `max_pulls`
  (`pull_count() + untracked.len()`), so the ceiling counts more than its name
  (Phase 3 §3).
- 🔵 (suggestion, medium) Scope-lowering re-implemented twice per capability with no
  shared abstraction (Phases 5–7).
- 🔵 (suggestion, low) `all_* + unlimited` leaves only the transport deadline guarding
  unbounded in-memory accumulation (Phase 7 / Performance Considerations).

### Code Quality

**Summary**: Well-structured for maintainability — pure/total parser separated from a
reusable validator, ceilings as a named type, whole-block replacement reusing per-key
resolution. Main risks are structural: `SearchScope` accreting overlapping fields
enforced only at the construction site, the resolution policy duplicated across both
adapters, and the parser/validator placed in the pure `work` crate depending on
`config::Value` and a homeless `FilterSchema`. Smaller DRY/YAGNI concerns round it out.

**Strengths**:
- Explicit, principled parse/validate separation (pure/total parse; standalone
  reusable `validate`).
- Ceilings modelled as `Ceiling { Bounded, Unlimited }` over an overloaded `0` — escapes
  primitive obsession.
- Whole-block replacement reuses existing `get` with no new merge logic.
- `#[non_exhaustive]` `Value` makes the `Mapping` variant a safe additive change.
- Ordering kept a free reconciliation-local comparator, not `Ord` on the port type.
- Distinct `PullConfigError` variants per rejection branch.
- Vertically sliced, TDD-structured phases naming concrete existing harnesses.

**Findings**:
- 🟡 (major, medium) `SearchScope` accretes four overlapping scope fields with invariants
  enforced only at the construction site (Phase 6 §1 / Phase 7 §1); illegal combinations
  representable. Consider a sum type or a single guarded constructor.
- 🟡 (major, medium) Multi-scope resolution policy duplicated across both adapters,
  risking drift on the safety-relevant abort-vs-transient categorisation (Phases 6–7 §2).
- 🟡 (major, medium) Parser/validator in the pure `work` crate couples it to `config` and
  to a `FilterSchema` type with no declared home (Phase 1 §2 / Phase 2 §§1–2).
- 🔵 (minor, medium) Ceiling default magic numbers (20 / 25) scattered across
  construction sites (Phase 3 §§2–3).
- 🔵 (minor, medium) Ceiling token validated raw then re-parsed to `Ceiling` — two
  interpretations of one lexeme (Phase 1 §2 / Phase 2 §2 / Phase 3).
- 🔵 (minor, medium) `split_prefix_sequence` duplicates existing identifier prefix/number
  parsing (Phase 8 §2).
- 🔵 (suggestion, low) `FilterSchema.required` slot is speculative — unused and unchecked
  (Phase 2 §1).

### Test Coverage

**Summary**: Strongly test-driven, reusing the right harnesses (RecordingTracker seams,
fixture goldens, black-box `Fixture` with `--fail-safe` twins), and its differential
ceiling tests give excellent mutation resistance. But several phases name tests at the
wrong layer or against a fake seam that cannot express the behaviour being changed:
Phase 4's keyed-read abort conflates truncation with genuine-indeterminate and collides
with an existing test; Phase 5's OR-within-a-key grouping is pinned above and below the
mapping but not through it; Phase 3's `max_pages`/`unlimited` transport behaviour is
unreachable through the page-less fake. Edge-case coverage is thin in places.

**Strengths**:
- Differential ceiling tests pin the ACs' intent (configured value vs built-in default).
- Idiomatic reuse of RecordingTracker's `Call::Search` recording, row-coverage-guarded
  goldens, and the black-box config `Fixture` with a `--fail-safe` twin.
- Explicit regression guards named (existing incomplete-discovery refusal, unkeyed-scope
  guard).
- Phase 2 enumerates a test per rejection branch plus the accepting case.

**Findings**:
- 🔴-in-body (major, high) Keyed-read abort relies on a fake seam that cannot distinguish
  truncation from genuine-indeterminate (`FetchOutcome` has no completeness flag) and
  collides with `an_indeterminate_items_watermark_is_left_unadvanced` (Phase 4).
- 🟡 (major, high) OR-within-a-key grouping pinned above and below the mapping but not
  through it; the exact bug being fixed would pass every named test (Phase 5).
- 🟡 (major, medium) `max_pages` config-sourcing and the `Unlimited` transport path are
  unreachable through the page-less fake (Phase 3).
- 🔵 (minor, medium) Comparator has only one 3-element case; degenerate identifiers
  untested (Phase 8).
- 🔵 (minor, medium) `Mapping`-variant match arms and per-tracker noun normalisation
  under-specified (Phase 1).
- 🔵 (minor, medium) Ceiling-token validation classes collapsed into one "bad ceiling"
  case (Phase 2).
- 🔵 (minor, medium) The `all_*`-clears-base construction rule is not pinned by a named
  test (Phase 7 §1).

### Correctness

**Summary**: Logically well-structured; several load-bearing claims check out — the
two-ceiling mutual exclusivity (`DiscoveryIncomplete` short-circuits at run.rs:804
before the count at run.rs:882) and whole-block replacement from `ConfigService::get`.
But three high-confidence defects: the truncation→hard-abort state machine cannot
distinguish truncation from transient on either keyed path; `max_items` binds to
`max_pulls` (tracked plus discovered, not "discovered count"); and the configured
`max_items` is dead behind the `--max-pulls` clap default. The comparator is also not a
total order and leaves no-digit / overflow undefined.

**Strengths**:
- Mutual-exclusivity claim verified (run.rs:804 propagated via `?` before run.rs:882).
- Whole-block replacement verified via `ConfigService::get` (service.rs:476-492).
- Dedup reuses `canonical_external_key`, the engine's single identifier-equality
  definition.
- `max_items` boundary reuses the existing strictly-greater refusal — no off-by-one.

**Findings**:
- 🔴-in-body (major, high) Keyed reconcile paths conflate truncation with transient
  failure; the clean three-way split is not implementable as coded (Phase 4 §2).
- 🔴-in-body (major, high) `max_items` maps onto `max_pulls`, which counts tracked pulls
  plus discovered — not the "discovered-issue count" (Phase 3 §3 / Desired End State).
- 🔴-in-body (major, high) `--max-pulls` carries a clap default of 25, so the configured
  `max_items` can never take effect (Phase 3 §3).
- 🟡 (major, medium) `split_prefix_sequence` comparator is not a total order over distinct
  ids and leaves no-digit / overflow undefined (Phase 8 §2).
- 🔵 (minor, medium) Whether `max_items` counts pre- or post-dedup is unspecified
  (Phase 8 §1 / Phase 3 §3).
- 🔵 (minor, medium) `parse` totality claimed "over shape" but only the mapping case is
  specified; a malformed scalar personal block shadows the team block (Phase 1 §§2–3).
- 🔵 (minor, medium) `max_pages` pre-empts `max_items`, so "`all_*` bounded by
  `max_items`" is only reachable when `max_pages: unlimited` (Phase 7).
- 🔵 (minor, low) `Unlimited` requires restructuring the bounded `1..=cap` paging loops;
  deadline expiry is misattributed to `max_pages` (Phase 3 §1 / Phase 4 §1).

### Safety

**Summary**: The core safety claim holds — verified that both `RemoteAbsent` and
`Indeterminate` map to `Action::Noop`, `gather` only marks `Absent` on a
provably-complete retrieval, and the only `remove_file` calls delete pending-push marker
sidecars, so Phase 4 changes abort-vs-proceed, not deletion safety; zero-write /
hold-all-watermarks is structurally sound. The two real concerns are that the Jira
client cannot distinguish a truncated keyed read from a transient failure (a naive
Phase 4 converts every transient blip into a whole-sync abort), and that Phase 4
deliberately widens one truncated read's blast radius to the whole sync — a persistent
block for any corpus that outgrows `max_pages`.

**Strengths**:
- "No path deletes a local file today" verified against `decide.rs:92-94` and the
  `remove_file` call sites.
- Zero-writes / all-or-nothing structurally sound (keyed read in `prepare_run`, run() `?`
  at run.rs:939 before apply / finalise_baseline).
- Watermark behaviour on abort strictly safer than the degrade path it replaces
  (finalise_baseline never reached, all watermarks held).
- Wider `all_*`/`additional_*` discovery is additive — only NEW-file authoring, dedup
  only shrinks the create set, bounded by the pre-write refusal.
- Sync-time resolution aborting on an unseen entity is a fail-safe default.

**Findings**:
- 🟡 (major, high) Jira keyed read cannot distinguish truncation from transient failure —
  abort promotion risks aborting on transient blips; Phase 4 tests only the
  truncation-aborts case (Phase 4 §2). Linear already separates the two, so only the Jira
  error type needs the split.
- 🟡 (major, medium) Abort widens one truncated read's blast radius to the whole sync — a
  persistent, self-perpetuating block past `max_pages`, including unattended scheduled
  syncs (Phase 4 ⚠️).
- 🔵 (minor, medium) `all_* + unlimited` disables every flood-guard for unattended bulk
  create (Phase 7 / Phase 3); recovery is cheap (VCS), so low severity — consider a
  preview nudge or soft cap.
- 🔵 (minor, low) Grow-on-pull catalogue write has unstated atomicity relative to the
  import (Phase 6 §3).

### Compatibility

**Summary**: Soundly designed for backward compatibility — absent-block base-only
discovery, unset-ceiling 25/20 defaults, single-value wire forms all preserved, and
pinned-crate changes additive with snapshot regen at the right phases. The
`Value::Mapping` impact analysis is accurate (verified `#[non_exhaustive]`, wildcard
arms, the three in-crate sites). The one material gap is the `--max-pulls` clap default,
which makes the config-sourced `max_items` default unreachable as written.

**Strengths**:
- `Value::Mapping` genuinely additive; impact analysis verified in the visualiser and
  launcher consumers (all fall through to empty/default).
- Config-file backward compatibility explicitly preserved; no migration; lazy additive
  index growth.
- Outbound wire format stable for the existing single-value case (Linear `eq`, Jira
  `IN (x)`), multi-value purely additive.
- Snapshots regenerated at exactly the phases touching pinned surfaces; all consumers
  in-tree, so surface changes are caught at compile time.

**Findings**:
- 🔴-in-body (major, high) `--max-pulls` default sentinel permanently shadows the new
  `pull.max_items` config default (Phase 3 §3).
- 🔵 (minor, medium) `Ceiling` must derive `Copy/Eq` to keep `TransportConfig`'s pinned
  surface intact (Phase 3 §1).
- 🔵 (minor, medium) Non-`#[non_exhaustive]` structs force literal-constructor updates the
  plan under-scopes (~10 `SearchScope`, ~15 `TransportConfig` sites) (Phases 3, 6).
- 🔵 (minor, low) Linear `in`-operator protocol support is assumed, not verified against
  the live schema (Phases 5–7).

### Security

**Summary**: Mixed injection posture. Jira filter VALUES are safely escaped through
`quote()` and Linear lowers to a structured `IssueFilter` passed as a GraphQL `$filter`
variable, so filter values are not a string-injection surface. The real gaps are Jira
filter KEYS raw-interpolated into JQL with acceptance gated only at `configure` (the
sync read path parses without validating), the new `project IN (...)` clause not stated
to reuse `quote()`, and `all_* + unlimited` as a deliberately unbounded whole-workspace
lever controlled by semi-trusted team config.

**Strengths**:
- Jira filter values escaped through `quote()`; Linear lowers to a serde_json
  `IssueFilter` GraphQL variable — filter values are not a query-injection surface.
- Phase 2's per-tracker key allow-list closes the raw-key gap at the `configure`
  boundary.
- `all_*` emits an explicit enumerated `IN` list (not an unbounded empty filter),
  preserving 0220's flood-guards; configure-time validation is fail-loud and
  non-degradable.
- Sync-time entity resolution aborts on a crafted/non-existent name rather than silently
  broadening.

**Findings**:
- 🔴-in-body (major, high) JQL key injection: configure-time schema validation does not
  gate the sync read path (Phase 1 §3 / Phase 5 §3). A hand-edited or PR-injected team
  config reaches raw JQL key interpolation unguarded.
- 🟡 (major, medium) New `project IN (...)` clause is not stated to reuse `quote()`
  escaping; `all_*` entities come straight from remote enumeration (Phase 6 §2 / Phase 7
  §2).
- 🔵 (minor, medium) `all_* + unlimited` is a maximally-broad, semi-trusted-config-controlled
  scope lever (Phase 7 / Phase 3).
- 🔵 (minor, low) Committed catalogue growth under `all_*` can widen workspace metadata in
  version control (Phase 6 §3).

### Usability

**Summary**: A genuinely configurable surface with strong DX instincts — fail-loud
`configure`-time validation, an observable `dump` deliverable, `unlimited` over `0`,
per-tracker vocabulary. But real gaps: the `--max-pulls` default threatens to silently
defeat `max_items`; unknown top-level keys (typos) are silently ignored despite the
fail-loud ethos; the whole-block replacement footgun is not surfaced in `dump`; and
error-message actionability is left underspecified against the codebase's own
`bad_integration` precedent.

**Strengths**:
- Validation fires at `configure` (fail-loud, non-degradable), following the
  `work.integration` precedent.
- Phase 1 surfaces the resolved block in `config dump` — an inspectable deliverable from
  the first phase.
- `unlimited` sentinel over `0` avoids the ambiguous-zero trap.
- Per-tracker vocabulary is self-documenting for the common single-tracker case.
- CLI-beats-config precedence for `--max-pulls` is the conventional direction.

**Findings**:
- 🟡 (major, medium) `--max-pulls` default (25) can silently defeat a configured
  `max_items`; the same ceiling is spelled three ways with no `--max-pages` flag
  (Phase 3 §3).
- 🟡 (major, medium) Unknown top-level pull-block keys are silently ignored (typo
  footgun) (Phase 1 §2 / Phase 2 §2).
- 🟡 (major, medium) Whole-block replacement footgun is not clearly surfaced in `dump`
  (Phase 1 §4).
- 🟡 (major, medium) Error-message actionability underspecified versus the
  `bad_integration` precedent (Phase 2 §2).
- 🟡 (major, medium) Wrong-tracker vocabulary is silently ignored rather than guided
  (Implementation Approach / Phase 1 §2).
- 🔵 (minor, medium) `max_items: 0` means refuse-all, the opposite of a common "unlimited"
  convention (Phase 3).
- 🔵 (minor, medium) Truncation guidance names bare `max_pages` without the config path to
  raise it (Phase 4 §1).
- 🔵 (suggestion, medium) An unconfigured `pull` block is invisible in `dump`, hurting
  discoverability (Phase 1 §4).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-13

**Verdict:** REVISE

All eight lenses were re-run against the revised plan. Both criticals and every pass-1
major are resolved or downgraded — the structural problems are gone. But deeper
verification of the now-more-detailed plan surfaced a focused second layer of ~18 major
findings, concentrated in Phase 4's fail-loud plumbing, the scope-abort error channel,
test specification, and one concrete boundary bug (`max_pages: 0`). No criticals. The
plan is materially stronger and on firm structural ground; it needs one more iteration on
the items below, not a rewrite.

### Previously Identified Issues

- 🔴 **Compatibility/Correctness/Usability**: `--max-pulls` shadows `max_items` —
  **Resolved** (`Option<usize>` presence-based override; snapshot schedule verified).
- 🔴 **Correctness/Safety/Test/Architecture**: Phase 4 keyed truncation signal —
  **Partially resolved** — signal split accepted, but the `gather → prepare_run`
  hard-abort plumbing is under-specified (risk: cap-hit merged into the soft
  `read_failure` path, abort never fires); test seam still unnamed.
- 🟡 **Security**: validation skips sync path (JQL key injection) — **Resolved**
  (non-degradable validate on the read path, verified against `jql.rs`).
- 🟡 **Code Quality/Architecture**: adapter lowering duplicated — **Resolved** (shared
  resolver single-sources the abort-vs-transient classifier).
- 🟡 **Architecture**: no abort-vs-transient channel — **Resolved** as designed, but the
  chosen `TrackerError` variant introduced new closed-enum/bifurcation majors (below).
- 🟡 **Architecture/Safety**: Phase 4 blast radius — **Partially resolved** —
  acknowledged and justified, but still over-broad (aborts create-from-local, which
  carries no `external_id` and cannot be made unsafe by a truncated read) and
  upgrade-breaking.
- 🟡 **Correctness/Architecture**: `max_items` counts more than its name — **Partially
  resolved** — wording clarified; downgraded to a minor (reconcile against the AC's
  "discovered count" framing).
- 🟡 **Correctness/Test**: comparator not a total order — **Resolved** (raw-id tie-break
  gives a provable total order; degenerate-id suite added).
- 🟡 **Security**: `project IN (...)` not `quote()`d — **Resolved** (values + entities
  escaped; Linear structured variable; break-out test).
- 🟡 **Code Quality**: `SearchScope` accretion — **Resolved** (`EntityScope` sum type;
  illegal states unrepresentable).
- 🟡 **Code Quality**: parser placement / `FilterSchema` home — **Partially resolved** —
  `FilterSchema`→`tracker` done; the `Value`→`PullConfig` parse-bridge home crate is
  still unidentified (neither `work` nor `work-adapters` depends on `config`).
- 🟡 **Test**: OR-within-a-key not pinned through the mapping — **Resolved** (contract
  test), but a new Jira golden field-name-mapping gap surfaced.
- 🟡 **Test**: `max_pages`/`Unlimited` untested through the fake — **Resolved**
  (layer-split criteria).
- 🟡 **Usability**: whole-block replacement invisible in `dump` — **Resolved** for the
  annotation, but the *test* is still weaker than the AC's field-drop scenario (major,
  below).
- 🟡 **Usability**: weak rejection messages — **Partially resolved** — new validation
  errors follow `bad_integration`, but existing runtime messages and the hardcoded team
  file path are still gaps (below).

### New Issues Introduced

- 🔴 **Correctness** (high): `max_pages: 0` yields a silent complete-empty result — the
  `1..=cap` loops make `cap=0` an empty range (no cap-hit fires), and the keyed read then
  marks live items `absent`. The "`0` = refuse-all" symmetry documented for `max_items`
  does not hold for a page count; reject `max_pages: 0` at validation.
- 🟡 **Correctness/Compatibility/Test** (Phase 4 cluster): the cap-hit abort needs a
  distinct hard-abort field on `GatheredFacts` (not the soft `read_failure` path); the
  exit-code/behaviour flip is upgrade-breaking with default `max_pages: 20` (release
  notes + preserve the `truncated` JSON field); the Jira standalone-search abort collides
  with the existing `search_echoes_the_envelope_and_audits_the_jql` test (can't go green).
- 🟡 **Architecture/Compatibility**: the scope-abort `TrackerError` variant expands a
  deliberately *closed* enum (compile-break across `for_tracker_error`, `apply.rs`,
  `classify.rs`, tests) and needs an exit-code-taxonomy slot; it also bifurcates the
  "unresolvable entity" concept from the pre-flight `ScopeError`. Reconsider routing the
  miss through the existing `ScopeError`/exit-74 channel.
- 🟡 **Architecture** (high): `max_pages` now governs two concerns (discovery pagination
  cap *and* the keyed-read completeness bound) that scale differently and can't be tuned
  independently — consider separate keys.
- 🟡 **Safety**: `all_* + unlimited` — the soft nudge is not a real safeguard (doesn't
  stop the unbounded apply loop; confirmation undefined non-interactively). Wants a
  genuine apply-time gate that fails safe without a TTY.
- 🟡 **Usability**: runtime messages (`RunError::Refused`, `DiscoveryIncomplete`) still
  cite CLI flags, not the config key that now sets the bound; and "name the config file"
  inherits `bad_integration`'s hardcoded *team* path, misdirecting when a personal block
  shadows.
- 🟡 **Test** (7): whole-block test weaker than the AC field-drop; config-sourced
  `max_items: 0` untested; no `all_*` empty-enumeration test; dedup test uses exact
  duplicates (won't pin the canonical fold); Jira golden `state`/`label` vs harness
  `status`/`labels` mapping; no `RecordingTracker` seam for the non-retryable scope-abort.
- 🔵 Minor/suggestion: catalogue serialization forward-compat (keep `/team`, add a new
  key); `tracker-support → tracker` dependency edge unacknowledged; deadline-driven
  keyed-read incompleteness still degrades silently (Overview wording overstated);
  saturating digit-run parse; sink-level JQL identifier guard (defence-in-depth).

### Assessment

The revision did its job on the findings it targeted: the two criticals are gone, the
`EntityScope` sum type and shared resolver retired whole classes of the original
concerns, and injection is closed at the boundary. What remains is a tighter, more
advanced set of issues — several of them *created* by the fixes (the closed-enum
expansion, the `max_pages` dual-role, the `max_pages: 0` boundary, the parse-bridge
home) and several test criteria that still under-pin the behaviour the plan now
describes. The verdict stays **REVISE**, but the distance to APPROVE is a focused
iteration — most impactfully: reject `max_pages: 0`; specify the Phase 4 abort plumbing
and its upgrade/skill impact; settle the scope-abort error channel (likely reuse
`ScopeError`/exit-74); decide the `max_pages` split; make the `all_*` guard a real gate;
and strengthen the named tests (AC field-drop, config `max_items: 0`, empty enumeration,
canonical dedup fold, Jira field-name mapping).

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-09-14

**Verdict:** REVISE

Every pass-2 finding is resolved or dissolved — verified against the codebase (the
zero-write/watermark plumbing, `EntityScope`, the exit-74 reuse, the injection defence in
depth all check out). Pass 3 then surfaced ~19 new majors, **zero criticals** — and while
most are second/third-order follow-ons from the pass-2 fixes, two are genuine functional
defects (the ordering comparator's parser mismatch and the Linear additional-team reconcile
gap). Several were verified against source (`pup.ron`, the shipped skills, the Linear
paging numbers, the local keyless `id_pattern`).
The plan is structurally sound; what remains is a focused finishing round plus a handful
of design decisions the fixes re-opened. This is the point of diminishing returns: a
pass 4 would surface a pass-4 tail.

### Previously Identified Issues (pass 2)

- 🔴 `--max-pulls` shadows `max_items` — **Resolved**.
- 🔴 Phase 4 truncation signal / plumbing — **Resolved** (distinct `GatheredFacts`
  hard-abort field, verified zero-write before planning).
- 🟡 Scope-abort error channel (closed enum / exit taxonomy / bifurcation / seam) —
  **Resolved** by reusing `RunError::DiscoveryUnconfigured` (exit-74).
- 🟡 `max_pages` dual-role — **Documented** (but see the new safety finding — the split
  may now be a prerequisite, not future work).
- 🟡 `all_* + unlimited` real gate — **Resolved** (fail-safe gate + `--allow-unbounded`).
- 🟡 Parse-bridge home — **Resolved in placement but the chosen edge is invalid** (see
  new `pup.ron` finding).
- 🟡 Runtime messages / config level — **Resolved**.
- 🟡 Test specification (7 items) — **Mostly resolved** (strong new controls); a few
  pinning gaps remain (below).
- 🟡 Injection on the sync path / `IN`-list escaping — **Resolved** (triple-layer defence
  verified); filter-*value* escaping still untested (below).

### New Issues Introduced

- 🔴 **Architecture** (high, verified): `work → config` is *forbidden* by
  `cli/pup.ron`'s `work_domain_imports_only_permitted` allow-list — my Rec 6 B is invalid
  as written. Either plan the `pup.ron` rule change (relaxing domain purity) or site the
  parser in a composition crate; do not describe it as a mere "confirm".
- 🟡 **Architecture**: the shared resolver "co-located with `Ceiling`" lands logic in the
  `tracker` crate, documented as a logic-free seam — site the resolver in
  `work-adapters/src/sync` instead; keep only the value types in `tracker`.
- 🔴 **Compatibility** (high): the Phase 4 exit-code flips break shipped skills not in the
  change set — `search-jira-issues` maps any non-zero to a credential error,
  `search-linear-issues` treats `truncated` as normal exit-0, `sync-work-items` documents
  exit 4 + `--push-only`. Add the skills to Phase 4 and assign dedicated, documented
  cap-hit exit codes.
- 🟡 **Compatibility**: Jira standalone `search` has no internal `max_pages` loop — its
  `nextPageToken` is deliberate user paging, so "exit non-zero on incomplete page" would
  break the documented `--page-token` workflow. Distinguish a cursor from an internal
  cap-hit.
- 🟡 **Safety** (verified): the Linear keyed read pages the *whole team* (~250×20 ≈ 5000),
  so organic team growth past ~5000 cap-hits every keyed read → run-wide abort at the
  shared `max_pages: 20`. Decoupling the keyed-read page budget from discovery is now a
  safety prerequisite, not the deferred "future story".
- 🟡 **Safety** (verified): pull-time catalogue growth must use the existing
  advisory-locked cache writers (`with_lock` + version stamp), not a bare `AtomicWrite`
  (lost-update window; Jira `.cache-version` desync).
- 🟡 **Security**: filter-*value* escaping leans entirely on the pre-existing `quote()`
  (first production use) with no adversarial test — add value goldens (interior `'`, `)`,
  ` OR `, trailing `\`) and confirm `quote()`'s backslash handling.
- 🟡 **Security**: committed-metadata disclosure under `all_*` is guarded only by a
  warning — a bounded `all_*` still commits entity metadata ungated.
- 🟡 **Code Quality**: `validate(&PullConfig, &FilterSchema)` can't carry the config
  level or tracker nouns its new error/hint requirements need — keep `PullConfigError`
  data-only and decorate in the launcher.
- 🟡 **Code Quality**: Phase 4 threads several parallel completeness signals — consolidate
  into one richer `FetchOutcome` outcome enum (`Complete | CappedAtPages | Deadline |
  Transient`) with the decisions derived in one match.
- 🔴 **Correctness** (high, verified): the Phase 8 comparator reuses
  `parse_full_id`/`WorkItemIdScheme`, which is parameterised by the *local* `work.id_pattern`
  — this repo's default is keyless numeric (`{number:04d}`), so it returns `NoMatch` for
  every hyphenated *remote* id, collapsing all ids to `(empty, 0)` and ordering `PP-10`
  before `PP-2` — failing the story's own AC. Split remote ids with a generic
  prefix/sequence decomposition independent of the local pattern. (This reopens the pass-1
  "reuse the existing parser" minor, which was wrong.)
- 🟡 **Correctness** (verified): the Linear keyed reconcile read pages only the *base*
  team and `in_scope` returns true only for the base prefix, so items imported from
  `additional_*`/`all_*` teams become permanently `indeterminate` on every later sync —
  never reconciled again. Broaden `page_all`/`in_scope` to every team the pull spans, with
  a regression test. (Jira is unaffected — project-agnostic `key IN (...)`.)
- 🟡 **Correctness**: the untracked-pull `DiscoveryIncomplete` path folds cap-hit and
  deadline into one `Discovery.complete` boolean, yet my Rec 7 relabelled its message to
  blame `max_pages` — a deadline cutoff would mis-blame the page cap. Carry a cap-vs-deadline
  cause on `Discovery`, or word the message without asserting the cap.
- 🟡 **Test Coverage** (×4): the Linear multi-value golden is inexpressible by the current
  harness (commit to the extension or the contract test); the Jira config-key→field
  mapping isn't pinned (golden drives post-mapping); the deadline soft-signal has no test;
  the interactive-confirmation gate branch has no harness (needs a confirmation port).
- 🟡 **Usability**: `--preview`'s relationship to the `all_*` gate is unspecified (exempt
  it — writes are the hazard).
- 🔵 Minors: catalogue reverse-compat (new binary reading a legacy `/team` file); gate
  keys on `all_*` not the unbounded-write condition (`additional_* + unlimited` also
  floods); non-TTY confirmation must default to refuse on EOF; `max_pages`-in-`pull`
  namespace governs a push-affecting read; typed (not string-matched) cap-hit-vs-transient;
  extend the sink guard to the `IS EMPTY` sites; near-miss-typo did-you-mean;
  `Mapping`-on-scalar-path assert-unreachable; `prepare_run` god-function accretion; the
  `FetchOutcome` completeness signal must be tri-state (complete / cap-hit / transient), not
  a boolean; same-key `IN` grouping needs deterministic order (BTreeMap, not HashMap) or the
  goldens are flaky; the `max_pages: 0` guard depends on validate running before
  `TransportConfig` construction; entity enumeration completeness is assumed (a truncated
  enumeration must be transient, not proof of absence); align the work item's `max_items`
  wording with "pull-direction writes".

### Assessment

Pass 3 earned its keep — it caught real, verified problems (`pup.ron` forbids the chosen
`work → config` edge; the shipped skills misread the new exit codes; the Linear keyed read
is an organic-growth cliff; the catalogue growth bypasses the established lock; the Jira
cursor conflation). Notably, most new majors are follow-ons from the pass-2 fixes, and the
sharpest reopen two earlier decisions — Rec 6 (parse-bridge home, now blocked by an
enforced rule) and Rec 4 (single `max_pages`, now a safety concern on the keyed path). The
verdict stays **REVISE**, but the trajectory is clearly converging: structural soundness is
established and verified. The tail is mostly finishing detail and a few design calls, with
two exceptions the correctness lens caught — genuine functional defects that must be fixed:
the ordering comparator reuses the local keyless id-parser (fails the AC), and Linear
`additional_*`/`all_*` imports can never be reconciled again (single-team keyed read).
Recommended: one targeted round on the verified-sharp items — the two correctness defects
(comparator parser, Linear multi-team reconcile), the `pup.ron`/parse-home decision, the
resolver's home, dedicated exit codes + skill updates, the Jira cursor split, the
keyed-read page-budget decoupling, the catalogue lock, the filter-value test — then **stop
the review loop** and let TDD implementation resolve the remaining specification details —
rather than run a pass 4.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 4) — 2026-09-15

**Verdict:** REVISE

All eight lenses re-ran (all eight accounted for before synthesis this time). Pass 4
surfaced ~15 new majors, **zero criticals** — but, contrary to the pass-3 prediction that
a pass 4 would find only a finer tail, several are **substantive and verified against
source**, and a meaningful fraction are **defects the targeted fix round itself
introduced**. This is the key signal: the review-and-fix cycle is no longer purely
converging — each substantial edit (the `max_pages` restructure especially) creates new
interaction bugs and inconsistencies. The recommendation to **stop the review loop and move
to implementation** is now stronger, not weaker.

### Previously Identified (pass 3) — status

All pass-3 sharp items were addressed in the targeted round and are **resolved** at the
plan level (comparator parser, Linear multi-team reconcile, `pup.ron`, keyed-read budget via
`max_pages` restructure, exit codes + skills, Jira cursor / tri-state, resolver home,
catalogue lock, filter-value test) — but pass 4 found that several fixes were incomplete or
introduced new problems (below).

### New Issues Introduced (pass 4)

**Verified, substantive (design gaps, not polish):**
- 🔴 **Architecture** (verified): Linear `list_teams` enumeration is a *single unpaginated
  query* — `all_teams` silently under-scopes and a valid `additional_teams` past the first
  page falsely aborts. Silent truncation at the enumeration layer the feature depends on.
- 🔴 **Correctness** (high, verified): Linear `page_all` is shared by discovery `search` and
  the keyed reconcile read, so it cannot read one `TransportConfig` field and honour both
  the `discovery` and `keyed_read` caps — the per-operation design is unsatisfiable unless
  each loop takes its `Ceiling` as a call argument. *(Fixed in the plan text this pass.)*
- 🟡 **Correctness**: the tri-state has no stated precedence when a chunked (Jira 50-id) or
  multi-team (Linear) read cap-hits *and* transient-fails at once — cap-hit must dominate,
  and the per-sub-read → single-signal fold must be specified, or a silent degrade returns.
- 🟡 **Correctness**: the untracked-pull discovery deadline behaviour is described as both
  "degrade" and "abort" — must state it still hard-aborts on both cap-hit and deadline.
- 🟡 **Architecture** (verified): the resolver's enumeration seam is undefined — the engine
  holds only `Box<dyn RemoteTracker>` (no enumeration method) and `work-adapters` has no
  production dep on the clients; may need a pinned-`tracker` API change not budgeted.
- 🟡 **Safety** (verified): the catalogue advisory lock (my Targeted 8) doesn't serialise the
  read-merge — the writers lock only the write of a pre-formed shape, so concurrent pulls
  still clobber; the whole read-merge-write must be inside one `with_lock`.
- 🟡 **Safety / Security**: the unbounded-write gate keys on `all_*` only —
  `additional_* + unlimited` floods ungated; key it on the hazard (unbounded write +
  broadened scope).
- 🟡 **Compatibility** (high, verified): the standalone-search cap-hit code belongs in the
  *per-binary* `jira-cli`/`linear-cli` taxonomies (off the reserved 70–74 band), not
  `work-cli`. *(Fixed this pass.)*
- 🟡 **Compatibility**: the Jira search pagination contract shift (single-page + manual token
  → internal pagination) is under-specified, and Jira has **no** `truncated` field today, so
  the "preserved `truncated`" claim was Linear-only. *(Field claim fixed this pass.)*
- 🟡 **Security**: the injection control is verified only against encoder *output*, not
  Jira's *parse* — `quote()`'s doubling-vs-backslash strategy must be grounded in Atlassian's
  JQL grammar with a contract check, not just an emitted-string golden.
- 🟡 **Test Coverage**: `max_items` post-dedup / discovered-create counting is asserted in
  prose but pinned by no named test.
- 🟡 **Usability**: the ceiling error enumeration ("non-negative integer or unlimited")
  contradicted the `max_pages: 0` rejection *(fixed this pass)*; `--allow-unbounded` isn't
  threaded into the `sync-work-items` skill and the interactive confirmation should be
  skill-driven, not binary-TTY.
- 🟡 **Code Quality**: the completeness/failure channels accrete in `prepare_run` — centralise
  the classification in one named unit.

**Self-introduced inconsistencies (all fixed this pass):** Migration Notes said "unchanged
default `max_pages: 20`" after Targeted 4 raised it to 50; the ceiling error enumeration
contradicted the `0` rejection; `PageCaps.general` disagreed with the `default` block key;
the `page_all` single-field cap; the standalone-search exit-code location; the Jira
`truncated`-field claim.

### Assessment

Pass 4 was worth running — it caught a genuine functional gap (Linear's unpaginated
enumeration) and several verified design/consistency defects, six of them introduced by the
very fixes of the prior round, which I corrected here. But that is exactly the warning sign:
we have reached the depth where fixing surfaces new problems roughly as fast as it closes
them, and the plan is now very large. The core remains sound and thoroughly reasoned; the
residue is interaction detail (which cap each loop reads, how sub-signals fold, which crate
an exit code lives in) that the compiler and TDD will force to a resolution far more
reliably than further prose review. **Strong recommendation: stop the review loop here and
implement.** The remaining pass-4 items above are recorded for the implementer. Verdict
stays **REVISE** only in the formal sense; the plan is implementation-ready with these notes
in hand.

---
*Re-review generated by /accelerator:review-plan*

## Approval — 2026-09-18

**Verdict:** APPROVE

After the post-pass-4 fix round (the substantive pass-4 majors applied to the plan — Linear
enumeration pagination + the `enumerate_visible_entities` port method, the cap-hit-dominant
tri-state fold, discovery abort-on-both, the lock-scoped catalogue merge, the hazard-keyed
skill-driven unbounded gate, `quote()` grounded in the JQL grammar, the `max_items`
post-dedup test, and the `prepare_run` classification step), the reviewer approved the plan
for implementation. The residual items recorded above are finishing detail deferred to TDD
implementation, not blockers. This supersedes the formal pass-4 REVISE verdict; the
frontmatter reflects APPROVE.

---
*Approved via /accelerator:review-plan*
