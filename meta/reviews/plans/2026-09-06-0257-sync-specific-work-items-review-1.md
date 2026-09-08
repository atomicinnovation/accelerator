---
type: "plan-review"
id: "2026-09-06-0257-sync-specific-work-items-review-1"
title: "Plan Review: Sync Specific Work Items"
date: "2026-09-06T19:57:55+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-09-06-0257-sync-specific-work-items"
target: "plan:2026-09-06-0257-sync-specific-work-items"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "safety", "security", "standards", "usability"]
review_number: 1
review_pass: 2
tags: ["sync", "cli", "work-sync"]
last_updated: "2026-09-06T20:36:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Sync Specific Work Items

**Verdict:** REVISE

The plan's core thesis is sound and well-argued: targeting is a set-construction
concern at the CLI boundary, the domain state machine is untouched, and a single
`ItemSelection` value binds the item slice to the discovery skip so the two
cannot desync. Eight lenses converge on that design being right — but one
critical safety gap (a targeted run advances the *global* baseline watermark,
risking silent data loss to non-targeted items on a later full sync), an
exit-code choice four lenses independently reject, and several implementation
details that would not compile or would fail the plan's own tests push this to
REVISE. None of the findings challenge the architecture; they harden a plan that
is already close.

### Cross-Cutting Themes

- **Exit-code taxonomy inconsistency** (flagged by: correctness, standards,
  usability, architecture) — Phase 3 maps every `--target` failure, including a
  no-match token and an out-of-directory path, to `USAGE` (2). But the
  authoritative taxonomy defines `RESOLVE_NOT_FOUND` (3) as exactly "an
  identifier resolved to no work item," and Phase 1 mints `RESOLVE_OUTSIDE_WORKDIR`
  (6) for the identical out-of-dir condition in `work resolve`. The same
  structural condition thus surfaces as 6 via `resolve` but 2 via `sync --target`,
  a well-formed-but-stale value is conflated with a syntax error, and the skill
  cannot branch on exit code to offer recovery.
- **Report-line rendering fragments and bypasses TSV sanitisation** (flagged by:
  code-quality, security, standards, architecture) — the `#\ttarget\tsuppressed`
  line is emitted by a raw `println!` inside `run_sync`, outside `render_report`
  (today the single owner of `#\t` records) and without the `single_line()`
  helper that every other variable-content field passes through. A token or
  local `id` carrying a tab or newline could forge a fabricated report record the
  skill then renders as a real outcome.
- **De-duplicate targets by resolved identity, not raw token** (flagged by:
  correctness, safety) — two distinct tokens (`0257` and its `external_id`
  `PP-787`) resolve to one item; token-string dedup does not collapse them, so
  the item enters the `Targeted` slice twice and flows duplicated into every
  apply loop — including a non-idempotent create-from-local.

### Tradeoff Analysis

- **Reusing `resolve::run` (exit-code-shaped `RunOutcome`) for set construction
  vs a direct `LocalItem` match**: reuse avoids a second resolver and gets Phase
  1's containment for free, but it round-trips through a `PathBuf` that must then
  be matched back onto a `LocalItem` by path — and the two sides canonicalise
  inconsistently (see the correctness finding). Reuse is defensible; the plan
  should pin the "resolved path always maps to a discovered item" invariant with
  a test and fix the canonicalisation mismatch.

### Findings

#### Critical

- 🔴 **Safety**: Targeted run advances the global baseline timestamp, corrupting
  the change-detection watermark for non-targeted items
  **Location**: Phase 2 (Success Criteria: "leaves the baseline entries for
  non-targeted items untouched")
  Every run ends in `finalise_run`, which unconditionally calls
  `set_timestamp(run_start_epoch)` (baseline_store.rs:109), advancing the
  *shared* watermark. Local-change detection short-circuits on `mtime <=
  baseline_timestamp` (classify.rs:70), so a non-targeted item edited before a
  targeted run has its mtime buried under the advanced watermark and is later
  classified `local=false` — silently never pushed, or overwritten remote-wins.

#### Major

- 🟡 **Correctness**: Unresolvable `--target` uses exit 2, contradicting the
  code-3 (and Phase 1's code-6) resolution-failure semantics
  **Location**: Phase 3 §3 vs Phase 1 §2
  The same condition yields exit 6 via `work resolve` but 2 via `sync --target`;
  a well-formed but unresolvable value is conflated with malformed flag usage.

- 🟡 **Correctness**: Canonicalised resolved path is matched against
  non-canonicalised `LocalItem.path`
  **Location**: Phase 3 §2 ("match path (canonicalised) to a LocalItem")
  `discover_items` stores `work_dir.join(filename)` uncanonicalised (sync.rs:100-127);
  under a symlinked root (macOS `/tmp`→`/private/tmp`, used by the integration
  tests) a valid path target is wrongly rejected as "not among the managed items."

- 🟡 **Correctness**: De-duplicate must key on the resolved item, not the token
  **Location**: Phase 3 §2 ("De-duplicate targets named more than once")
  Two distinct tokens can resolve to one `LocalItem`; token-string dedup lets it
  enter the `Targeted` slice twice, risking double plan/apply — including a
  non-idempotent remote create.

- 🟡 **Architecture**: Blanket item-slice narrowing also narrows the
  `corpus_carries` double-binding guard
  **Location**: Phase 2 §1 (routing every `request.items` read through `selection.items()`)
  `corpus_carries` (run.rs:831-836) is a second whole-corpus read — the guard that
  stops a recovered `Created` marker binding two files to one remote issue.
  Narrowing it lets an `external_id` on a non-targeted file become invisible to
  the guard during a targeted create-from-local recovery.

- 🟡 **Code Quality**: `run_sync` grows further despite an existing
  `too_many_lines` allow
  **Location**: Phase 3 §3
  The function already carries `#[allow(clippy::too_many_lines)]` (~200 lines);
  Phase 3 inlines the target-resolution match, suppression-print loop, selection
  construction, and a reorder. Extract the item-set construction into a helper.

- 🟡 **Code Quality**: Collect-all failure type is referenced but never modelled
  **Location**: Phase 3 §2
  The plan calls `failure.message()` and names conditions (`NoMatch`,
  `AmbiguousExternal`, …) but defines no type, inviting a stringly-typed
  `Vec<String>` against the DDD convention. Specify a `TargetResolutionFailure`
  enum mirroring `SelectionError`.

- 🟡 **Code Quality**: Report rendering fragments between `render_report` and an
  inline `run_sync` `println!`
  **Location**: Phase 3 §3 vs Phase 2 §3
  The report format is now authored in two places; the suppression tokens skip
  `single_line()`. Thread suppression notes into the report renderer.

- 🟡 **Usability**: Target failures collapse onto exit 2, contradicting the
  resolve taxonomy the same plan adds
  **Location**: Phase 3 §3 vs Phase 1 §2
  Exit 2 = "I mistyped the command" is violated by a valid flag with a stale id;
  the skill cannot distinguish a directional-conflict usage error from a
  target-resolution failure without brittle stderr matching.

- 🟡 **Usability**: A remote-id target that exists remotely but is untracked
  locally aborts with a misleading no-match message
  **Location**: Phase 3 §2 and "What We're NOT Doing"
  `--target PP-999` for a real-but-untracked remote issue reports "no match" for
  an id the user can see in their tracker, with no hint that a full sync imports it.

- 🟡 **Usability**: Target-abort messages don't guide the user to discover valid
  ids, unlike the resolve skill callers
  **Location**: Phase 3 §2 and Phase 4 §1
  The three resolve callers offer `/list-work-items` on not-found; the `--target`
  abort only names the offending token. Fat-fingering an id gives no recovery
  affordance.

- 🟡 **Standards**: Unresolvable `--target` uses `USAGE`=2 where
  `RESOLVE_NOT_FOUND`=3 is the taxonomy's exact match
  **Location**: Phase 3 §3
  Two codes carry overlapping meaning for the same failure — the drift the
  single-authoritative-taxonomy design exists to prevent. Code 3 is also missing
  from the sync `--help` and SKILL.md exit lists.

- 🟡 **Standards**: New code 6 not reflected in the taxonomy's band header "(0–5)"
  **Location**: Phase 1 §2
  The module doc groups "Process and selection codes (`0`–`5`)"; adding code 6
  without widening the header leaves the authoritative surface misrepresenting
  its own range.

- 🟡 **Test Coverage**: Per-item `ReportedItem` equality proof is not supported by
  the current types
  **Location**: Testing Strategy: Byte-identical comparison; Phase 2
  `ReportedItem` derives nothing and `ItemOutcome` wraps `ApplyError` with no
  `PartialEq`; only `action`/`state` are comparable. The headline "identical to a
  full sync" criterion has no compilable assertion. Add a derive or a projection
  helper.

- 🟡 **Test Coverage**: The target-suppressed stdout line has no automated
  coverage
  **Location**: Phase 3 §3 and Phase 4
  It is emitted only after a credentialed `registry.resolve`, unreachable by the
  bin-only subprocess suite and below the fake-tracker lib layer. Extract the
  formatting into a pure function and unit-test its TSV output.

#### Minor

- 🔵 **Correctness**: The narrowed-items `Vec` is borrowed but dropped at the end
  of its match arm
  **Location**: Phase 3 §3 (the `ItemSelection::Targeted(&resolved.items)` sketch)
  As written the borrow cannot outlive `resolved`; the code does not compile.
  Hoist the owned `Vec` into a binding that outlives `request`.

- 🔵 **Correctness**: The reorder must also relocate `work_dir`/`root` resolution
  ahead of the credential check
  **Location**: Phase 3 §3
  Today `registry.resolve` runs before `work_dir` is resolved; the sketch omits
  moving the resolution block that `discover_items` depends on.

- 🔵 **Correctness**: `canonical_external_key` takes `&ExternalId`, and
  `resolve::run` returns a `Result` the closure must absorb
  **Location**: Phase 3 §2
  Signature mismatches in the sketch: wrap the token in `ExternalId::new` for the
  index key, and define how a `kernel::Error` from `resolve::run` surfaces.

- 🔵 **Architecture**: `OutsideWorkDir` maps to two different exit codes across
  sibling commands
  **Location**: Phase 1 §2 vs Phase 3 §3
  Defensible if deliberate — but record why `sync` collapses all target failures
  (including `OutsideWorkDir`) to `USAGE`.

- 🔵 **Architecture**: Double-resolution — a binary-boundary type reused for set
  construction requires two path sources to agree
  **Location**: Phase 3 §2
  Pin the "resolved path maps to a discovered `LocalItem`" invariant with a test,
  or resolve directly against the already-built index.

- 🔵 **Architecture**: Run-level report-line emission split between `run_sync` and
  `render_report`
  **Location**: Phase 3 §3
  Thread suppression notes into `RunReport` so one renderer owns all `#\t` lines.

- 🔵 **Code Quality**: `Fn(&str) -> RunOutcome` closure hides `resolve::run`'s
  config-read error
  **Location**: Phase 3 §2
  State how the `kernel::Error` is handled before the pure closure, so a config
  fault does not masquerade as a per-token resolution failure.

- 🔵 **Code Quality**: Work-dir canonicalisation failure reported as "no work item
  at path"
  **Location**: Phase 1 §1
  An unreadable work directory (environment fault) and a missing item share one
  message; give the former a distinct diagnostic.

- 🔵 **Security**: The `#\ttarget\tsuppressed` line interpolates raw `token` and
  `local_id` without `single_line()`
  **Location**: Phase 3 §3
  A tab/newline in either field forges a fabricated report record (TSV injection);
  blast radius limited to CLI args and local frontmatter.

- 🔵 **Standards**: `RunOutcome` mapping doc comment not updated for the new
  variant
  **Location**: Phase 1 §1
  Add `OutsideWorkDir → 6` to the enum's inline mapping comment.

- 🔵 **Standards**: Suppression line's token field bypasses `single_line()`
  **Location**: Phase 3 §3
  Same TSV-integrity gap as the security finding, from the standards angle.

- 🔵 **Standards**: "Mirroring that skill's Exit-3 wording" assumes uniform Exit-3
  handling the callers don't share
  **Location**: Phase 1 §4
  `create-work-item` handles Exit 3 by *proceeding* (topic-string fallback), not
  stop-and-offer; mirroring it for an out-of-dir path is wrong. Specify Exit-6
  wording per skill.

- 🔵 **Usability**: Human rendering of the new discovery-skipped and
  target-suppressed lines is underspecified
  **Location**: Phase 4 §2
  The fixed Step 5 template has no slot for either; give exact human phrasing so
  the machine TSV tokens don't leak verbatim.

- 🔵 **Usability**: `--target` help text leans on the internal term `external_id`
  and gives no example tokens
  **Location**: Phase 3 §1 and Phase 4 §1
  Phrase around user vocabulary with examples: "a local id (0257), a remote
  tracker key / external_id (PP-787), or a file path."

- 🔵 **Test Coverage**: Several resolver edge cases the plan introduces have no
  tests
  **Location**: Phase 3 §2
  Duplicate `--target`, a resolved path not among managed items, and an empty
  `--target ''` token all lack coverage.

- 🔵 **Test Coverage**: Resolver-closure branch coverage is inconsistent between
  plan sections
  **Location**: Testing Strategy vs Phase 3
  The Testing Strategy lists six branches; the Phase 3 checklist enumerates four.
  Add unit tests for `OutsideWorkDir`-no-cascade and collect-all accumulation.

- 🔵 **Test Coverage**: Two success-criteria assertions are phrased against
  mechanisms the doubles don't expose per-item
  **Location**: Phase 2 Automated Verification
  Baseline is one file, not per-item; the tracker fake exposes `calls()`, not a
  search counter. Restate against the real mechanisms.

- 🔵 **Safety**: De-duplicate targets by resolved item identity, not raw token
  **Location**: Phase 3 §2
  (Reinforces the correctness finding from the double-apply angle; the id-keyed
  pending-push marker makes this defence-in-depth, not an open hole.)

- 🔵 **Safety**: Zero-writes abort assertion should also cover the baseline
  document, not just `meta/work`
  **Location**: Phase 3 §3 / Success Criteria
  The safety-critical mutable state on abort is the baseline and pending-push
  markers under the integrations root; assert those are byte-unchanged too.

#### Suggestions

- 🔵 **Security**: `OutsideWorkDir` message discloses the absolute work-directory
  path — consistent with existing `NotFound` messages, low impact for a local CLI.

- 🔵 **Security**: The `work_dir.canonicalize()` failure branch conflates an
  environment fault with a benign not-found; fail-closed, no bypass.

- 🔵 **Code Quality**: `resolve_targets` success value is used as an unnamed
  struct; name it (e.g. `ResolvedTargets { items, suppressed }`).

### Strengths

- ✅ `ItemSelection` binds the item slice and the discovery skip into one value,
  so a narrowed set can never be reconciled while discovery still runs — the
  invariant lives in the type, not in call-site discipline (all 8 lenses).
- ✅ Strong functional-core/imperative-shell separation: the precedence/cascade
  decision is extracted behind a pure `Fn(&str) -> RunOutcome` closure plus a
  pre-built index, unit-testable without composing a config.
- ✅ The domain state machine and every per-item behaviour are reused unchanged;
  targeting is confined to set construction at the CLI boundary.
- ✅ Phases 1 and 2 each merge alone as behaviour-preserving hardening (engine
  always receives `All`), keeping `main` green and shrinking each reviewable unit.
- ✅ Phase 1 path containment is fail-closed and sound: canonicalises both sides,
  uses component-wise `starts_with` (symlink- and prefix-confusion-safe), and
  requires containment before `is_file` (security lens).
- ✅ The unresolvable-target and out-of-dir aborts are pre-flight, before any
  tracker contact or write, with the credential-independence property tested
  explicitly (exits 74 not 2 on a valid target).
- ✅ Discovery suppression is well-covered: `SkippedTargeted` rendering is
  unit-tested, targeted-beats-push-only has a dedicated test, and the golden
  fixture (which uses `Ran`) is asserted unchanged.
- ✅ The `--target`/`targets` flag and the new report lines follow established
  conventions precisely (repeatable-flag model, `discovery` line family,
  `key=value` trailing fields).

### Recommended Changes

1. **Do not advance the global baseline timestamp on a narrowed run** (addresses:
   the critical safety finding). Either skip `set_timestamp` under `Targeted`, or
   track the watermark per-item so the mtime short-circuit only applies to
   reconciled items. Add the regression test the safety lens specifies: full sync
   → edit a non-targeted item → targeted sync of a different item → full sync must
   still detect the non-targeted local change. This is the blocking change.

2. **Reconcile the exit-code taxonomy** (addresses: the four exit-code findings).
   Map an unresolvable target to `RESOLVE_NOT_FOUND` (3) and an out-of-dir target
   to `RESOLVE_OUTSIDE_WORKDIR` (6), reserving `USAGE` (2) for genuine flag
   misuse; add codes 3 and 6 to the sync `--help` and SKILL.md exit lists; widen
   the `exit_codes.rs` band header to `(0–6)` and update the `RunOutcome` mapping
   doc comment. If collapsing to 2 is deliberate, record the rationale.

3. **Fix the path-match and de-dup correctness bugs** (addresses: the two
   correctness majors + the borrow/reorder minors). Canonicalise both sides
   before comparing resolved paths to `LocalItem.path` (or match on a canonical
   key); de-duplicate by resolved `LocalItem` id after resolution; hoist the owned
   narrowed `Vec` and the `work_dir`/`root` resolution above `registry.resolve`;
   correct the `canonical_external_key`/`ExternalId` and `Result` signatures.

4. **Preserve the `corpus_carries` guard over the full corpus** (addresses: the
   architecture major). Keep the whole discovered set available to the
   double-binding guard — carry both the full slice and the selection in
   `SyncRequest`, or build `corpus_carries` over the full set — and route only the
   reconciliation reads through `selection.items()`.

5. **Own the report line in one renderer with sanitisation** (addresses: the
   rendering/TSV theme). Thread suppression notes into `render_report` (or a
   sibling helper) and pass `token`/`local_id` through `single_line()`, so a
   single function owns every `#\t` record and applies one sanitisation control.

6. **Model the failure type and close the test gaps** (addresses: code-quality
   and test-coverage). Specify a `TargetResolutionFailure` enum with `message()`;
   add the `ReportedItem`-equality derive or projection; extract and unit-test the
   suppression-line formatter; cover duplicate/empty/resolved-but-unmanaged
   targets and the `OutsideWorkDir`-no-cascade branch.

7. **Sharpen the developer-facing surface** (addresses: usability + the Exit-6
   skill finding). Make the remote-id no-match message name the "untracked
   remotely — run a full sync" cause; offer `/list-work-items` on target abort;
   specify Exit-6 branch wording per skill (not a blanket "mirror Exit-3"); give
   `--target` help user-facing vocabulary with examples.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound in its core thesis — targeting as
set-construction at the CLI boundary, domain state machine untouched, the
narrowing-implies-discovery-suppression invariant encoded in one `ItemSelection`
value. The phase decomposition is genuinely incremental and open-closed. The main
weakness is the claim that discovery is the *single* non-trivial coupling that
inverts under narrowing — `corpus_carries` is a second whole-corpus read the
blanket routing instruction silently narrows.

**Strengths**:
- `ItemSelection` couples the item slice and the discovery gate into one value,
  so a narrowed set can never be reconciled while discovery still runs.
- Strong functional-core/imperative-shell separation via the pure resolver
  closure plus a pre-built index.
- The domain state machine and every per-item behaviour are reused unchanged.
- Phases 1 and 2 each merge alone with no user-visible change and leave main green.
- The discovery gate is extended additively (`SkippedTargeted`, targeted-beats-
  push-only) rather than by rewriting existing arms.

**Findings**:
- 🟡 major (medium): Blanket item-slice narrowing also narrows the
  `corpus_carries` double-binding guard (Phase 2 §1). `corpus_carries`
  (run.rs:831-836) scans `request.items` to stop a recovered `Created` marker
  binding two files to one remote issue; narrowing lets a non-targeted file's
  `external_id` become invisible during a targeted create-from-local recovery.
  Carry both the full slice and the selection, or build the guard over the full set.
- 🔵 minor (high): `OutsideWorkDir` maps to exit 6 via `resolve` but 2 via
  `sync --target` (Phase 1 §2 vs Phase 3 §3) — a taxonomy divergence; document the
  tradeoff if deliberate.
- 🔵 minor (medium): Double-resolution — `RunOutcome` (a binary exit-code type)
  reused for set construction requires `discover_items` and `resolve::run` to
  agree on canonicalisation/eligibility (Phase 3 §2). Pin the invariant with a test.
- 🔵 minor (medium): Run-level report-line emission split between `run_sync` and
  `render_report` (Phase 3 §3); thread suppression notes into `RunReport`.

### Correctness

**Summary**: Logically well-structured — one `ItemSelection` drives slice and
skip, the discovery gate gives `Targeted` precedence over `PushOnly`, and the
resolve-outcome cascade covers every variant. But the exit code for an
unresolvable target contradicts the taxonomy; path-to-`LocalItem` matching
compares canonicalised against non-canonicalised paths (mismatches under
symlinked temp dirs); de-dup is under-specified; and the sketched selection code
borrows a `Vec` dropped at the end of its match arm.

**Strengths**:
- `ItemSelection` makes narrowing and discovery-skip a single decision.
- The discovery gate tests `discovery_suppressed()` before the `PushOnly` branch,
  correctly giving `Targeted` precedence.
- The cascade enumerates all five `RunOutcome` variants with explicit
  cascade-vs-fail semantics.
- Collect-all accumulation and credential-independent abort ordering are correct.

**Findings**:
- 🔴 major (medium): Unresolvable `--target` uses exit 2, contradicting code-3
  (and Phase 1's code-6) resolution-failure semantics (Phase 3 §3 vs Phase 1 §2).
- 🔴 major (medium): Canonicalised resolved path matched against non-canonicalised
  `LocalItem.path` (Phase 3 §2); breaks under symlinked roots such as macOS `/tmp`
  used by the integration tests. Canonicalise both sides, or match on a canonical key.
- 🟡 major (medium): De-duplication must key on the resolved item, not the token
  (Phase 3 §2); two shapes naming one item otherwise apply it twice.
- 🔵 minor (medium): The narrowed-items `Vec` is borrowed but dropped at the end
  of its match arm (Phase 3 §3) — does not compile; hoist to an outer binding.
- 🔵 minor (medium): The reorder must also relocate `work_dir`/`root` resolution
  ahead of `registry.resolve` (Phase 3 §3), else credential-independence regresses.
- 🔵 minor (low): `canonical_external_key` takes `&ExternalId`, and `resolve::run`
  returns `Result<_, kernel::Error>` the closure must absorb (Phase 3 §2).

### Code Quality

**Summary**: Unusually rigorous for its size — isolates a genuine invariant into
one value, drives every phase test-first, and extracts a pure config-free
resolver closure. The maintainability risks concentrate in Phase 3: an already-
`too_many_lines` `run_sync` grows further, the collect-all failure type and the
`resolve_targets` return struct are referenced but never modelled, and report
rendering fragments across two sites.

**Strengths**:
- `ItemSelection` structurally prevents the narrowed-set desync.
- The pure `Fn(&str) -> RunOutcome` closure plus index is a strong testability move.
- Reordering target validation ahead of `registry.resolve` is a deliberate
  design-for-test choice.
- Domain naming is rich and intention-revealing.
- Phases sequenced so 1 and 2 each merge as behaviour-preserving hardening.

**Findings**:
- 🟡 major (high): `run_sync` grows despite the existing `too_many_lines` allow
  (Phase 3 §3); extract the item-set construction into a helper.
- 🟡 major (high): Collect-all failure type referenced but never modelled
  (Phase 3 §2); specify a `TargetResolutionFailure` enum with `message()`.
- 🟡 major (medium): Report rendering fragments between `render_report` and an
  inline `run_sync` `println!` (Phase 3 §3 vs Phase 2 §3); the suppression tokens
  also skip `single_line()`.
- 🔵 minor (medium): The `Fn(&str) -> RunOutcome` closure hides `resolve::run`'s
  config-read error (Phase 3 §2); state where the `kernel::Error` surfaces.
- 🔵 minor (medium): Work-dir canonicalisation failure reported as "no work item
  at path" (Phase 1 §1); give the environment fault a distinct message.
- 🔵 suggestion (medium): `resolve_targets` success value used as an unnamed
  struct (Phase 3 §3); name it `ResolvedTargets { items, suppressed }`.

### Test Coverage

**Summary**: Unusually test-literate — correctly places the "identical to a full
sync" proof in the fake-tracker lib layer, keeps credential-independent aborts at
the subprocess layer, unit-tests the discovery render arm, and preserves the
golden fixture. The central weakness is that the headline proof mechanism —
per-item `ReportedItem` equality — is not supported by the current types, and the
`#\ttarget\tsuppressed` line has no automated coverage.

**Strengths**:
- Correctly places the narrowing/identical-to-full proof in `sync_run.rs`.
- The credential-independent abort strategy is sound (exit 2 before the check;
  valid target reaches 74).
- Discovery suppression is well-covered; the golden fixture is asserted unchanged.
- The Spy write-recorder and RecordingTracker already support the assertions.
- Path-containment negatives include a plain out-of-dir file and a traversal case.

**Findings**:
- 🟡 major (high): Per-item `ReportedItem` equality proof is not supported by the
  current types (Testing Strategy; Phase 2); `ReportedItem`/`ItemOutcome` derive
  no `PartialEq`. Add a derive or a projection helper.
- 🟡 major (high): The target-suppressed stdout line has no automated coverage
  (Phase 3 §3, Phase 4); extract the formatting into a pure function and unit-test it.
- 🔵 minor (high): Duplicate `--target`, a resolved path not among managed items,
  and an empty `--target ''` token lack tests (Phase 3 §2).
- 🔵 minor (medium): Resolver-closure branch coverage inconsistent between plan
  sections (Testing Strategy vs Phase 3); add `OutsideWorkDir`-no-cascade and
  collect-all unit tests.
- 🔵 minor (medium): Two Phase 2 success-criteria assertions are phrased against
  mechanisms the doubles don't expose per-item (baseline is one file;
  `RecordingTracker` exposes `calls()`).

### Safety

**Summary**: The central mechanism — coupling the slice and discovery-skip into
one `ItemSelection` — is sound and closes the most obvious double-apply hazard.
The pre-flight, credential-independent aborts fail closed with zero writes. But
the plan overlooks that `finalise_run` advances the *global* baseline timestamp
on every run, including a targeted one; because change detection short-circuits on
`mtime <= baseline_timestamp`, a targeted run moves the watermark past
non-targeted items that were edited but not reconciled.

**Strengths**:
- `ItemSelection` structurally prevents the discovery desync (verified against
  `discover_untracked`, run.rs:415-436).
- The unresolvable-target and out-of-dir aborts are pre-flight with zero writes,
  tested explicitly.
- Phase 1 path containment fails closed; `resolve_targets` treats a
  resolved-but-unmanaged path as a failure.
- `finalise_run` only blanks named conflict ids and never prunes entries, so a
  targeted run leaves non-targeted baseline *entries* intact.

**Findings**:
- 🔴 critical (high): Targeted run advances the global baseline timestamp,
  corrupting the change-detection watermark for non-targeted items (Phase 2).
  `finalise_run` calls `set_timestamp(run_start_epoch)` (baseline_store.rs:109);
  `classify.rs:70` short-circuits on `mtime <= baseline_timestamp`. A non-targeted
  item edited before a targeted run is later classified `local=false` — silently
  never pushed, or overwritten remote-wins. Do not advance the global timestamp on
  a narrowed run, or track the watermark per-item; add the regression test.
- 🔵 minor (medium): De-duplicate targets by resolved item identity, not raw token
  (Phase 3 §2) — the id-keyed pending-push marker makes this defence-in-depth.
- 🔵 minor (medium): The zero-writes abort assertion should also cover the
  baseline document and integrations root, not just `meta/work` (Phase 3 §3).

### Security

**Summary**: Phase 1's path-containment hardening is soundly designed —
canonicalises both sides (resolving symlinks and `..`), uses component-wise
`starts_with`, and is strictly fail-closed. The `work_dir.canonicalize()`
NotFound path does not mask a bypass. Phase 3 reuses the hardened `resolve::run`
and adds a second containment layer, ordering validation before any tracker
contact. Residual concerns are minor: the new report line skips `single_line()`,
and error messages disclose absolute paths.

**Strengths**:
- Containment canonicalises both sides and uses component-wise `starts_with`
  (symlink-escape and prefix-confusion safe).
- Strictly fail-closed: `Resolved` requires candidate + root canonicalisation +
  containment + `is_file`.
- Phase 3 reuses the hardened resolver and layers a managed-item check.
- Target validation is ordered before `registry.resolve` — zero side effects on
  malicious input.
- The remote `external_id` field passes through `canonical_external_key`, which
  strips whitespace, neutralising TSV injection from remote data.

**Findings**:
- 🔵 minor (medium): The `#\ttarget\tsuppressed` line interpolates raw `token` and
  `local_id` without `single_line()` (Phase 3 §3); a tab/newline forges a
  fabricated report record. Blast radius limited to CLI args and local frontmatter.
- 🔵 suggestion (medium): The `OutsideWorkDir` message discloses the absolute
  work-directory path (Phase 1 §1) — consistent with existing `NotFound` messages,
  low impact for a local CLI.
- 🔵 suggestion (low): The `work_dir.canonicalize()` failure branch conflates an
  environment fault with a benign not-found (Phase 1 §1); fail-closed, no bypass.

### Standards

**Summary**: Broadly convention-faithful — the repeatable `--target`/`targets`
flag mirrors `--tag`/`tags`, the new discovery line parallels `skipped\tpush-only`
exactly, and the `key=value` fields follow the `found=N` precedent. The chief
concern is exit-code consistency: `USAGE`=2 for an unresolvable target where
`RESOLVE_NOT_FOUND`=3 is defined as precisely that, and code 6 introduced without
updating the band header and the `RunOutcome` mapping doc.

**Strengths**:
- The `--target` flag follows the repeatable-flag convention and doc-comment style
  precisely.
- The new discovery line is byte-consistent with the existing `discovery_line`
  family; suppression fields follow the `found=N` precedent.
- `ItemSelection` keeps the two derived facts co-located.
- Code 6 is placed in the semantically correct process band.

**Findings**:
- 🔴 major (medium): Unresolvable `--target` uses `USAGE`=2 where
  `RESOLVE_NOT_FOUND`=3 is the exact match (Phase 3 §3); code 3 is also missing
  from the sync `--help` and SKILL.md exit lists.
- 🟡 major (high): New code 6 not reflected in the taxonomy band header "(0–5)"
  (Phase 1 §2); widen to "(0–6)" and add the bullet.
- 🔵 minor (high): `RunOutcome` mapping doc comment not updated for the new variant
  (Phase 1 §1); add `OutsideWorkDir → 6`.
- 🔵 minor (medium): The suppression line's token field bypasses `single_line()`
  (Phase 3 §3).
- 🔵 minor (medium): "Mirroring that skill's Exit-3 wording" assumes uniform Exit-3
  handling the callers don't share (Phase 1 §4); `create-work-item` proceeds on
  Exit 3 rather than stopping. Specify per-skill wording.

### Usability

**Summary**: A well-scoped CLI addition following the codebase's repeatable-flag
convention, surfacing its two most surprising behaviours (discovery suppression,
local-id-wins) as explicit report lines rather than silent magic. The main DX
risks are around error taxonomy and actionability: target failures collapse onto a
generic usage code, a remote-id target not yet tracked locally aborts with a
"doesn't exist" message when it plainly does, and the abort paths lack the
`/list-work-items` affordance the sibling resolve callers offer.

**Strengths**:
- The repeatable `--target` flag matches the established `--tag`/`--block` convention.
- Collect-all abort names every offending target at once.
- Discovery suppression and local-id-wins are surfaced via explicit report lines.
- `--preview`/`--push-only` interactions are exercised in manual steps and pinned
  by a lib test.

**Findings**:
- 🟡 major (high): Target failures collapse onto exit 2, contradicting the resolve
  taxonomy the same plan adds (Phase 3 §3 vs Phase 1 §2); the skill cannot branch
  on exit code to give a targeted recovery message.
- 🟡 major (medium): A remote-id target that exists remotely but is untracked
  locally aborts with a misleading no-match message (Phase 3 §2); state the
  "untracked — run a full sync" cause at the point of failure.
- 🟡 major (medium): Target-abort messages don't guide the user to discover valid
  ids, unlike the resolve callers (Phase 3 §2, Phase 4 §1); offer `/list-work-items`.
- 🔵 minor (medium): Human rendering of the new discovery-skipped and
  target-suppressed lines is underspecified (Phase 4 §2); specify exact phrasing
  and template slots.
- 🔵 minor (low): The `--target` help text leans on the internal term `external_id`
  and gives no example tokens (Phase 3 §1, Phase 4 §1); use user-facing vocabulary
  with examples.

## Re-Review (Pass 2) — 2026-09-06

**Verdict:** APPROVE

All eight lenses re-ran against the revised plan. Every pass-1 finding is
resolved, including the critical data-loss hazard. The re-review surfaced a fresh
batch of issues in the pass-1 edits themselves — four majors and a set of
minors/suggestions — each of which was then addressed in a follow-up edit round
(noted per item below). Those follow-up edits were not independently re-reviewed;
one low-severity suggestion (a report note when two tokens collapse to one item)
is accepted rather than implemented.

### Previously Identified Issues (pass 1)

- 🔴 **Safety** — global baseline watermark data loss — Resolved. New Phase 2
  per-item `local_synced_at` watermark + Phase 3 selection-scoped advance +
  regression test.
- 🟡 **Correctness / Standards / Usability / Architecture** — exit 2 for target
  failures — Resolved. No-match → 3, out-of-dir → 6, misuse → 2; band header
  widened; help and SKILL exit lists updated.
- 🟡 **Architecture** — `corpus_carries` narrowed — Resolved. `SyncRequest.corpus`
  full-set field; guard reads it.
- 🟡 **Correctness** — canonicalised path vs `LocalItem.path` — Resolved. Both
  sides canonicalised; `Unmanaged` failure added.
- 🟡 **Correctness / Safety** — de-dup by token — Resolved. De-dup by resolved
  item id.
- 🟡 **Code Quality** — `run_sync` too long / collect-all type unmodelled /
  fragmented rendering — Resolved. `build_selection` helper;
  `TargetResolutionFailure` + `ResolvedTargets`; single-renderer with
  `single_line()`.
- 🟡 **Test Coverage** — `ReportedItem` no `PartialEq` / suppression line
  untested — Resolved. Projection helper; pure formatter + unit test.
- 🟡 **Usability** — remote no-match misleading / no `/list-work-items` — Resolved.
  Cause-naming message; skill offers `/list-work-items`.
- 🟡 **Standards** — code 6 not in band header — Resolved.
- 🔵 All pass-1 minors (borrow lifetime, `work_dir` reorder, signatures,
  `RunOutcome` doc, mirror-Exit-3, TSV injection, help vocabulary, rendering
  phrasing, test-mechanism phrasing) — Resolved.

### New Issues Introduced (found in pass 2, then addressed)

- 🟡 **Code Quality / Correctness** — `resolve_targets` vs `build_selection`
  name/shape inconsistency, `Scope` undefined, `suppressed` dropped — Addressed:
  one `build_selection` helper returning `SelectedTargets { scope, items,
  suppressed }`, wrapping the pure `resolve_targets`.
- 🟡 **Code Quality / Correctness** — work-dir-uncanonicalisable returned exit 3
  (which `create-work-item` treats as a topic string) — Addressed: routed to exit
  1 via the `Result` `Err` channel; `run` canonicalises the root once.
- 🟡 **Correctness** — empty `--target ''` yielded exit 3, not the specified 2 —
  Addressed: `Malformed` variant + pre-`classify_input` blank guard → exit 2.
- 🟡 **Usability** — exit 2 mislabelled "malformed invocation" (it also means
  ambiguous, which the sibling callers disambiguate) — Addressed: Phase 5 exit-2
  branch names ambiguity and offers disambiguation.
- 🟡 **Test Coverage** — watermark regression test wall-clock-coupled — Addressed:
  deterministic epochs/mtime via `filetime` with the stated ordering; harness
  gains a per-run clock and persistent baseline (Phase 3 §6).
- 🔵 **Architecture / Code Quality** — corpus/selection invariant unenforced —
  Addressed: `ItemSelection::All` made payload-free so `corpus` is the single
  source of truth.
- 🔵 **Safety** — `finalise_run` advanced the watermark for `Indeterminate`
  (read-failure) items; `unsynced_creates` not narrowed — Addressed: advance only
  on a definitive reconciled outcome; `unsynced_creates` reads `reconciled()`;
  read-failure and no-remote-create tests added.
- 🔵 **Architecture** — `finalise_run` coupled to the selection concept /
  `discover_untracked` rebind unstated — Addressed: selection-agnostic
  `finalise_run(blank, advance_ids, Option<u64>)`; `discover_untracked` reads
  `corpus`.
- 🔵 **Safety** — malformed per-item watermark tolerance — Addressed: non-integer
  reads back as the document `timestamp`; test added.
- 🔵 **Standards** — collect-all precedence direction — Addressed: `USAGE` (2) >
  `OUTSIDE` (6) > `NOT_FOUND` (3), documented.
- 🔵 **Usability / Standards** — `Sync` `--help` and SKILL Step 2 exit lists not
  updated for 3/6 — Addressed: explicit Changes items for both.
- 🔵 **Code Quality** — per-variant exit-code comments in the enum snippet
  (comment policy) — Addressed: removed; `exit_code()` is the sole mapping.
- 🔵 **Test Coverage** — "byte-unchanged" broke under the additive field; harness
  single-run; double-binding test mis-placed — Addressed: compare loaded-then-
  rendered form / seed the field; harness §6; double-binding test moved to
  `sync_create.rs`.
- 🔵 **Usability** (suggestion) — two tokens collapsing to one item give no
  feedback — Accepted, not implemented (low-value report-line surface).

### Assessment

The plan is in good shape and ready for implementation. The design thesis held up
under two review passes; the changes since pass 1 are the per-item watermark
(with its migration-free format change), the corpus/selection split, and a
consistent exit-code taxonomy — each now specified concretely enough to build
test-first. The only open item is one accepted low-severity suggestion. Because
the pass-2 corrective edits were not themselves re-reviewed, a light spot-check
during implementation of the watermark-advance gating (Phase 3 §4) and the
`build_selection`/`Scope` seam (Phase 4 §3) is worthwhile.
