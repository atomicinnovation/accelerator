---
type: "plan-review"
id: "2026-09-08-0285-targeted-pull-of-remote-only-work-items-review-1"
title: "Plan Review: Targeted Pull of Remote-Only Work Items and Resolution Normalisation"
date: "2026-09-08T20:57:03+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-08-0285-targeted-pull-of-remote-only-work-items"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "architecture", "code-quality", "test-coverage", "compatibility", "safety", "usability", "documentation"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-08T21:50:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Targeted Pull of Remote-Only Work Items and Resolution Normalisation

**Verdict:** REVISE

The plan is unusually well-reasoned: it preserves the credential-free
resolution invariant by keeping the remote probe a separate imperative-shell
pre-flight, reuses `create_from_remote`/`author_from_remote` for allocation
parity by construction, extends `ItemSelection::Targeted` as a capability rather
than a behaviour switch, and correctly grounds absence on `fetch_all`'s provable
partition instead of `show`. Two structural issues nonetheless block approval:
the `SkippedTargeted` → `TargetedPull { imported: 0 }` swap falsifies Phase 2's
"no CLI-visible change" claim and cannot compile without touching the renderer
it defers, and the four-way exit-code precedence spans two mechanisms whose tie
between `absent` (3) and `indeterminate` (70) is unresolved. Alongside these,
the test strategy targets a harness that provably cannot stub the tracker, and
the skill-rendering change set omits several stale duplicate locations.

### Cross-Cutting Themes

- **`SkippedTargeted` → `TargetedPull { imported: 0 }` breaks Phase 2's
  independence** (flagged by: architecture, code-quality, correctness,
  compatibility, documentation) — Phase 2 maps every `Targeted` run to
  `TargetedPull { imported: pull_ids.len() }`. With the CLI passing an empty
  slice, an existing reconcile-only targeted run now emits
  `TargetedPull { imported: 0 }` instead of `#\tdiscovery\tskipped\ttargeted`.
  `DiscoveryStatus` is matched exhaustively by `discovery_line`, so the phase
  cannot compile without updating the renderer — contradicting "changes no
  CLI-visible behaviour and is mergeable on its own", and stranding
  `SkippedTargeted` as dead code with a now-false doc comment.

- **Exit-code precedence and all-offenders accumulation split across two sites**
  (flagged by: correctness, architecture, code-quality, compatibility) — Today
  `TargetResolutionFailure::exit_code` + `highest_precedence_code` are the single
  owners. Phase 3 adds a second precedence path in `run_sync` for the `fetch_all`
  outcome. `highest_precedence_code` ranks both `RESOLVE_NOT_FOUND` (3) and
  `RETRYABLE` (70) at the same weight, so a batch carrying both `absent` and
  `indeterminate` could exit 70 (retry forever) instead of the intended 3, by
  iteration order.

- **The `fetch_all` gate is untestable as placed** (flagged by: test-coverage,
  code-quality) — The gate sits inline in the already-`too_many_lines` `run_sync`,
  after `registry.resolve`. The proposed integration tests target the bin-only
  subprocess harness in `cli_sync_targets.rs`, which cannot inject a stub
  tracker, so `--target PP-999` fails at the credential check before reaching any
  pull, mixed-abort, or preview behaviour the plan wants to assert.

- **Existing tests encode the behaviour the plan reverses** (flagged by:
  test-coverage) — Phase 1 inverts `a_local_id_wins_and_records_the_remote_match_as_suppressed`
  and Phase 3 inverts `a_no_match_target_exits_three_before_the_credential_check`,
  but the plan only "adds" a collision test and "narrows" exit 3. The red step is
  understated: these tests must be rewritten/deleted or the suite asserts stale
  behaviour (or won't compile once `Suppressed` is gone).

- **Skill-rendering change set is incomplete** (flagged by: documentation,
  usability) — Exit codes are documented in `SKILL.md` in several further places
  the change set omits: the Step 1 precedence line (must become `2 > 6 > 3 > 70`),
  the Step 2 target-abort summary (lines 153-156), and the suppressed-note
  lead-in sentence (281-283). Phase 1 and Phase 3 also both cite line 84 for
  different edits.

### Tradeoff Analysis

- **Fail-safe refuse-all vs the AC's "at most N created"**: The reused engine
  path refuses the whole run with zero writes when `pulls > max_pulls`
  (`RunError::Refused`), but work item AC #9 reads "at most N are created and the
  remainder are reported as bounded", implying partial creation. Refuse-all is
  the safer choice (no partial blast radius, consistent with 0257).
  Recommendation: adopt refuse-all and reword AC #9 so no future change
  reinterprets it as a per-item budget.

- **One exit-code owner vs the resolution-pure/probe-separate split**: The
  architecture and code-quality lenses want a single `exit_code` + precedence
  function; the plan's design deliberately evaluates local failures before
  credentials and remote failures after. These are reconcilable: keep one
  ranking function (extend it with `Absent`/`Indeterminate` variants) invoked at
  two sequenced moments, rather than duplicating the precedence logic inline.

### Findings

#### Critical

- None.

#### Major

- 🟡 **Architecture / Code Quality / Correctness / Compatibility**: `TargetedPull { imported: 0 }` replaces `SkippedTargeted` for all targeted runs, contradicting "no CLI-visible change"
  **Location**: Phase 2 §2 (Inject the pull ids at the discovery branch)
  Every `Targeted` run now emits `TargetedPull { imported: pull_ids.len() }`. With Phase 2's empty slice, a reconcile-only targeted run emits `imported: 0` rather than `skipped targeted`, changing the report TSV that `discovery_line` and `SKILL.md` consume. The enum is matched exhaustively, so the phase cannot compile without the renderer change it defers to Phase 3, and `SkippedTargeted` becomes dead code with a false doc comment.

- 🟡 **Correctness / Compatibility**: Combined `absent` + `indeterminate` `fetch_all` outcome has an unresolved precedence tie
  **Location**: Phase 3 §2 (`fetch_all` gate); Implementation Approach
  A single `fetch_all` batch can return both a non-empty `absent` and a non-empty `indeterminate`. The gate lists `absent → 3` and `indeterminate → 70` as separate arms without stating which wins, and `highest_precedence_code` ranks both at the same weight, so `max_by_key` breaks the tie by iteration order — a batch could exit 70 (retryable, re-run forever) instead of 3.

- 🟡 **Correctness**: Duplicate remote candidates canonicalising to one remote issue create two local files
  **Location**: Phase 3 §1 (Resolution collects remote candidates)
  Two no-local-match tokens for the same underlying issue (e.g. `--target PP-999 --target pp-999`) are each collected without canonical de-dup. `create_from_remote` runs once per candidate and is not wrapped by the `corpus_carries` double-binding guard, so a canonically-duplicated target authors two local files bound to one remote issue — the exact double-binding the work item's Technical Notes require the guard to prevent.

- 🟡 **Compatibility**: `--push-only --target <remote-only>` would perform a pull
  **Location**: Phase 2 §2 (match ordering)
  The `Targeted` arm precedes the `PushOnly` guard, so injected `pull_ids` become the `untracked` set even on a push-only run. Once Phase 3 populates `pull_ids` from `fetch_all` without gating on direction, a push-only targeted run authors a new local file — violating the direction contract that push-only never pulls.

- 🟡 **Code Quality / Architecture / Compatibility**: Exit-code precedence and all-offenders accumulation split across two sites
  **Location**: Phase 3 §2 (`fetch_all` gate); Implementation Approach
  The precedence `2 > 6 > 3 > 70` and the "name every offender" property, each single-sourced today, are split: remote-absent tokens accumulate in a second accumulator inside `run_sync`, and precedence is enforced partly by `highest_precedence_code` and partly by control-flow sequencing. A future change must be made consistently in two implicitly-related places.

- 🟡 **Test Coverage**: `fetch_all` gate placed where no test can reach it
  **Location**: Testing Strategy (Integration Tests); Phase 3 §2
  The subprocess harness is bin-only and cannot stub the tracker, and the gate sits after `registry.resolve`, so the planned remote-only pull, mixed-batch abort, and preview tests are unreachable without live credentials. The highest-risk new behaviour would be manual-only.

- 🟡 **Test Coverage**: Plan inverts existing tests but only describes "adding"
  **Location**: Phase 1 §1-2; Phase 3 §1
  `a_local_id_wins_and_records_the_remote_match_as_suppressed` and `a_no_match_target_exits_three_before_the_credential_check` assert the behaviour this plan reverses. The plan says "add a collision test" and "narrow exit 3" but never lists these (plus `the_suppression_line_...` and every `render_report(&report, &[])` call site) as rewrites/deletions — the TDD red step is understated.

- 🟡 **Documentation / Usability**: Skill-rendering change set omits stale duplicate locations
  **Location**: Phase 1 §3; Phase 3 §4
  Exit codes are documented again at the Step 1 precedence line (100-101, still `2 > 6 > 3`), the Step 2 target-abort summary (153-156, no exit-70 case), and the suppressed-note lead-in (281-283, orphaned once the bullet is deleted). Phase 1 and Phase 3 also cite overlapping edits on line 84.

- 🟡 **Usability**: "imported N remote-only item(s)" misleads under `--preview`
  **Location**: Phase 2 §2 / Phase 3 §4 (`TargetedPull` rendering)
  The rendered line and field name use past-tense `imported`, but under `--preview` nothing is written. The existing discovery line uses neutral `found=N` to stay preview-safe; an operator running `--preview --target PP-999` would read "imported 1" and believe a write occurred.

#### Minor

- 🔵 **Architecture / Code Quality**: `discovery_suppressed()` name and `ItemSelection` doc become misleading once `Targeted` imports
  **Location**: Phase 2 §2 injection point
  After the change, discovery is not suppressed but substituted by a by-id pull; the predicate named for suppression becomes the hook that performs imports, and the doc comment ("a narrowed set can never re-import") no longer holds. Rename (e.g. `discovery_search_suppressed`) and correct the doc.

- 🔵 **Correctness**: `imported` count reflects attempted, not successfully applied, pulls
  **Location**: Phase 2 §2 / Phase 3 §4
  `TargetedPull { imported: pull_ids.len() }` is computed before the apply loop; if a `create_from_remote` fails mid-run the item reports `Failed` yet the summary still says `imported N`, disagreeing with the per-item rows.

- 🔵 **Correctness / Safety / Test Coverage**: `--max-pulls` "at most N created" (AC) vs refuse-all (engine)
  **Location**: Phase 2 Success Criteria; work item AC #9
  The engine refuses the whole run with zero writes when exceeded; AC #9 implies partial creation up to N. The planned test asserts refuse-all, so it would not confirm the AC as written. Reconcile the wording (see Tradeoff Analysis).

- 🔵 **Correctness / Safety / Test Coverage**: `found`-then-`show`-fails path unspecified and untested
  **Location**: Desired End State (resolution table); Phase 3 gate
  Presence is established by `fetch_all`, but the body is fetched by a later `show` inside `create_from_remote`. `show` cannot report absence, so an issue deleted in the window fails indefinitely as a per-item error. "Present remotely → pull-create" is not an absolute guarantee; the path needs an engine test and a documented expectation.

- 🔵 **Safety**: "All-or-nothing" holds at the gate, not across the apply loop
  **Location**: Implementation Approach; Phase 3 §2
  The pre-flight gate aborts before any write, but the create loop applies each `create_from_remote` sequentially and continues past a failure, so a mid-batch failure leaves earlier files written (recoverable, idempotent on re-run). Scope the wording to the gate and add a test proving a clean recoverable state.

- 🔵 **Safety / Usability**: A typo'd remote-only target is no longer diagnosable offline
  **Location**: Phase 3 §1-2; Desired End State exit table
  Under 0257 a no-local-match token exited 3 credential-free; now it defers to the gate. Offline, a bare typo surfaces as exit 70 (retryable) or exit 74 (unconfigured) rather than exit 3, and the skill's exit-3 recovery text ("offer /list-work-items") no longer fully fits. Note the shift and update the recovery guidance.

- 🔵 **Usability**: Collision message should label each file's role
  **Location**: Phase 1 §1 (Collision failure variant)
  Naming two paths plus "pick one" leaves the operator guessing why the token is ambiguous. The message should say the token is A's local id and B's `external_id`, so the operator can choose the right path to re-run with.

- 🔵 **Documentation**: Phase 1 prose rewrite should state the own-`external_id` reconcile case
  **Location**: Phase 1 §3 (`--target` prose)
  The rewrite addresses only the collision case; it should also positively state that a token equal to a single file's own `external_id` reconciles silently (the "reconcile A, no note" row), the common now-changed path.

- 🔵 **Documentation**: Resolution table omits exit 6 and the ambiguous exit-2 cases
  **Location**: Desired End State (lines 99-107)
  The table reads as an exhaustive per-token spec but omits outside-workdir (6) and malformed/ambiguous (2), which appear in the `2 > 6 > 3 > 70` precedence. Add the rows or annotate the table as covering only the outcomes this item changes.

#### Suggestions

- 🔵 **Architecture**: Targeted pull bypasses `discover_untracked`'s corpus dedup
  **Location**: Implementation Approach / Phase 2 injection
  Full sync filters discovered ids against the corpus by `canonical_external_key`; the targeted pull injects `pull_ids` directly, skipping that filter and relying on a cross-module invariant. Defend it at the injection point by filtering `pull_ids` against the corpus's canonical `external_id` set (reusing the same predicate).

- 🔵 **Code Quality**: Plan snippet comments risk verbatim transcription
  **Location**: Phase 2 §2 / Phase 3 §2 (illustrative snippets)
  `// existing discover_untracked path, unchanged` and the four-line partition legend would violate the project's very-low comment tolerance if carried into production. Treat them as plan-only annotation and express the distinctions through named arms/functions.

- 🔵 **Usability / Documentation**: Emitted TSV shape for `TargetedPull` is not pinned
  **Location**: Phase 3 §4 (skill rendering)
  The plan gives the human phrasing but not the exact `#\tdiscovery\t…` token the skill's Step 5 table must match. If the table cannot match it, the raw TSV leaks. Pin the emitted shape and add the matching translation entry.

- 🔵 **Test Coverage**: "No remote call" assertion needs a recording stub
  **Location**: Testing Strategy (Unit Tests)
  Proving a locally-resolvable token makes no remote call requires a spy stub counting `fetch_all` invocations; the plan states the assertion without the infrastructure. Specify a recording stub and assert a zero count.

### Strengths

- ✅ Preserves the deliberate credential-independence invariant from 0257 by
  keeping `resolve_targets` filesystem-pure and moving the remote probe into a
  separate pre-flight in `run_sync` after credentials resolve — a clean
  functional-core / imperative-shell split.
- ✅ Reuses `create_from_remote` → `author_from_remote`, achieving id, filename,
  and baseline allocation parity with a discovery import by construction rather
  than a parallel scheme, and folds into the existing pull accounting, preview,
  and apply loops.
- ✅ Correctly grounds absence on `fetch_all`'s provable `absent` partition
  rather than `show` (which cannot distinguish a deleted issue from a transient
  fault), and maps `indeterminate`/transport failure to retryable exit 70.
- ✅ Extends `ItemSelection::Targeted` from a tuple to a struct variant carrying
  `pull_ids` — an open-closed extension that adds capability without rewriting
  the selection model.
- ✅ Preserves the untargeted `All` path structurally (the discover branch,
  `reconciled()` over the whole corpus, and the document-watermark advance gated
  on `ItemSelection::All`), so the byte-identical full-sync requirement holds by
  construction.
- ✅ New local files are authored through an exclusive `LocalAuthor` write, so an
  id/filename collision surfaces as an error rather than silently clobbering an
  existing file; the `--max-pulls` bound refuses the whole run before any write
  in both preview and apply; and re-run idempotence is sound via the baseline
  `local_synced_at` written at creation.
- ✅ Deleting the whole `Suppressed` machinery (rather than repurposing it) and
  renaming `suppressed_remote` to return the colliding file are genuine
  readability/DDD improvements.
- ✅ Phased delivery with genuinely separable concerns, each with named automated
  checks paired to manual verification, and an explicit mixed-failure precedence
  contract.

### Recommended Changes

1. **Resolve the `SkippedTargeted`/`TargetedPull` swap explicitly** (addresses:
   the swap theme, "imported under preview", dead-doc findings) — Either retain
   `SkippedTargeted` for the `imported == 0` case and emit `TargetedPull` only
   when a pull occurs, or move the `discovery_line` + skill rendering into Phase 2
   and drop the "no CLI-visible behaviour change" claim. Make the rendered line
   preview-aware ("would import N" under `--preview`) and rename the field if it
   overstates applied work.

2. **Pin the exit-code precedence in one ranking function** (addresses: the
   precedence-split theme, the absent/indeterminate tie) — Model `absent` and
   `indeterminate` as `TargetResolutionFailure`-style variants carrying their own
   `exit_code`, so `absent` (3) provably dominates `indeterminate` (70) within one
   `fetch_all` batch and the `2 > 6 > 3 > 70` chain lives in one place, even
   though evaluated at two sequenced moments. Add a test for a batch carrying both.

3. **De-duplicate and guard the pull candidates** (addresses: double-binding) —
   De-dup `remote_candidates` by `canonical_external_key` before the gate, and
   filter confirmed `pull_ids` against the corpus's canonical `external_id` set at
   the injection point, mirroring the double-binding guard the work item requires.

4. **Gate the pull on a pull-capable direction** (addresses: push-only pulls) —
   Collect/inject `pull_ids` only for a direction that pulls, so
   `--push-only --target <remote-only>` is a no-op or usage error, not a create.

5. **Relocate and extract the test seam** (addresses: gate untestability,
   inverted existing tests) — Extract the candidate partition/precedence into a
   pure function over a `FetchOutcome`, drive the wiring via `run_sync` with a
   stub `TrackerRegistry`, and reserve the subprocess harness for
   credential-independent assertions. List the existing tests to invert/delete as
   the Phase 1/Phase 3 red step, and add cases for `found`-then-`show`-fails and a
   mixed absent+indeterminate batch.

6. **Complete the skill-rendering change set** (addresses: skill doc theme) — Add
   the Step 1 precedence line (100-101), the Step 2 target-abort summary
   (153-156), and the suppressed-note lead-in (281-283) to the change set;
   correct the overlapping Phase 1/Phase 3 citation on line 84; state the
   own-`external_id` reconcile case in the `--target` prose; and pin the emitted
   `TargetedPull` TSV shape with a matching Step 5 translation entry.

7. **Reconcile `--max-pulls` semantics** (addresses: refuse-all vs AC) — Adopt
   refuse-the-whole-run (fail-safe, zero writes) and reword AC #9 and the success
   criteria so no future reviewer reinterprets it as a partial-create budget.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan is logically well-structured and correctly identifies the
key invariants it must preserve (filesystem-pure resolution before credentials,
all-or-nothing target resolution, provable absence via `fetch_all` rather than
`show`, and reuse of the `create_from_remote` import path). The main correctness
gaps are under-specified branch behaviour when a single `fetch_all` outcome
carries more than one partition class, an unaddressed duplicate-candidate path
that can double-bind one remote issue to two local files, and a Phase 2
discovery-status change that contradicts its own "no CLI-visible change" claim.
None are fatal, but each would produce a wrong result on an input the acceptance
criteria imply.

**Strengths**:
- Correctly grounds absence on `fetch_all`'s provable `absent` partition rather
  than `show` (whose Retryable error cannot distinguish a deleted issue from a
  transient fault), citing the verified tracker contract at lib.rs:413-454.
- Preserves the all-or-nothing zero-write property by keeping local-resolvable
  failures aborting in `build_selection` before any tracker contact.
- Reuses `create_from_remote` so id/filename/baseline allocation is
  byte-identical to a discovery import.
- Correctly reasons that the ordinary-synced-item arm already reconciles
  silently, so a re-run of a pulled item resolves through the local external_id
  index with no re-pull.

**Findings**:
- 🟡 major (medium) — **Combined absent + indeterminate `fetch_all` outcome has
  under-specified precedence and ties in the rank function** (Phase 3 §2). A
  single `fetch_all` call can return non-empty `absent` AND `indeterminate`
  simultaneously. The gate lists them as separate arms without stating which
  wins, and `highest_precedence_code` ranks both `RESOLVE_NOT_FOUND` (3) and
  `RETRYABLE` (70) at the same `else => 1` weight (sync.rs:566-577), so
  `max_by_key` breaks the tie by iteration order. A mixed batch could exit 70
  instead of 3, or be order-dependent. Specify absent-first precedence and extend
  the `rank` closure to rank `RESOLVE_NOT_FOUND` above `RETRYABLE`.
- 🔴 major (medium) — **Duplicate remote candidates that canonicalise to one
  remote issue create two local files** (Phase 3 §1). Two distinct tokens with no
  local match but the same underlying remote issue are each collected without
  canonical de-dup. `resolve_targets` de-dupes matched local items by id
  (sync.rs:543-548) but adds no equivalent for candidates, and `create_from_remote`
  is not guarded by `corpus_carries` (that guard only wraps `apply_local_create`,
  run.rs:914-919). The work item explicitly requires the double-binding guard to
  extend to a newly-created targeted item. De-dup by `canonical_external_key`
  before the gate and confirm absence from `request.corpus` before authoring.
- 🟡 major (high) — **Replacing `SkippedTargeted` with `TargetedPull{imported:0}`
  changes reporting for existing reconcile-only targeted runs** (Phase 2 §2).
  With Phase 2's empty `pull_ids`, an existing targeted reconcile now emits
  `TargetedPull { imported: 0 }` instead of `SkippedTargeted`, changing the
  rendered discovery line (sync.rs:181-196) and breaking existing assertions that
  expect `#\tdiscovery\tskipped\ttargeted`. The phase is not the behaviour-neutral,
  independently-mergeable change it claims. Keep `SkippedTargeted` for the empty
  case, or acknowledge and update the affected assertions in Phase 2.
- 🔵 minor (medium) — **Refuse-all `--max-pulls` may not satisfy the "at most N
  created" AC** (Phase 2 / work item AC #9). The reused engine path refuses the
  whole run before any write (`RunError::Refused`, run.rs:824-833) — zero, not up
  to N. Confirm the intended semantics; if refuse-all is correct, reword the AC.
- 🔵 minor (medium) — **`imported` count reflects attempted, not applied, pulls**
  (Phase 2 §2 / Phase 3 §4). Computed in `prepare_run` before the apply loop; a
  mid-run `create_from_remote` failure reports the item `Failed` yet the count
  still says imported N. Name the field to reflect the attempted count or derive
  it from applied outcomes.
- 🔵 suggestion (low) — **A `fetch_all`-confirmed "found" target can still fail at
  `create_from_remote`'s `show`** (Desired End State / Phase 3 gate). Presence is
  established by `fetch_all`; the body is fetched by a later `show` (apply.rs:270).
  A deleted-after-confirmation issue fails indefinitely as a per-item error. Note
  that a confirmed-found target may still fail at `show` and is reported as a
  per-item retryable failure.

### Architecture

**Summary**: The plan is structurally sound and shows strong architectural
discipline: it preserves the credential-free resolution invariant by keeping the
remote probe a separate imperative-shell pre-flight, reuses the
create-from-remote/author-from-remote path for allocation parity, and extends
`ItemSelection::Targeted` as a capability (carried `pull_ids`) rather than a
behaviour switch. The main concerns are (1) the report/discovery-status contract
changing in a way that contradicts Phase 2's "no CLI-visible change" claim, and
(2) exit-code precedence and the load-bearing "no-double-import" invariant now
being split across two decision sites, weakening single-source-of-truth cohesion.

**Strengths**:
- Preserves the deliberate credential-independence invariant from 0257 — a clean
  functional-core / imperative-shell separation.
- Reuses `create_from_remote` → `author_from_remote`, achieving allocation parity
  by construction and folding into existing accounting, preview, and apply loops.
- Extends `ItemSelection::Targeted` from a tuple to a struct variant — an
  open-closed extension that keeps `reconciled() ⊆ corpus` intact.
- Phased delivery with genuinely separable concerns and an explicit precedence
  contract for mixed-failure exit codes.

**Findings**:
- 🟡 major (medium) — **`TargetedPull` discovery status replaces `SkippedTargeted`
  for all targeted runs, contradicting the "no CLI-visible change" claim** (Phase
  2 §2). Because the CLI passes an empty slice, an existing targeted-local run
  emits `TargetedPull { imported: 0 }` instead of `#\tdiscovery\tskipped\ttargeted`
  — a change to the engine→CLI→skill report contract. Either retain
  `SkippedTargeted` for the empty case, or move the discovery-line and skill
  rendering into Phase 2 and drop the claim.
- 🔵 minor (medium) — **Exit-code precedence and all-or-nothing failure
  accumulation split across two sites** (Phase 3 §1-2). Phase 3 introduces a
  second, parallel failure-collection-and-precedence path in `run_sync`,
  reproducing "name every offender, all-or-nothing" outside the resolution
  accumulator. Model remote outcomes as additional `TargetResolutionFailure`-style
  variants so one authoritative `exit_code` + precedence function remains.
- 🔵 minor (medium) — **`discovery_suppressed()` name and `ItemSelection`
  invariant no longer match behaviour once `Targeted` imports** (Phase 2 injection
  point). The predicate named "suppressed" becomes the gate that performs imports;
  the documented "a narrowed set can never re-import" no longer holds. Update the
  doc and consider `discovery_search_suppressed()`.
- 🔵 suggestion (medium) — **Targeted pull bypasses `discover_untracked`'s corpus
  dedup, relying on an implicit cross-module invariant** (Implementation Approach
  / Phase 2). Full sync filters discovered ids by `canonical_external_key`; the
  targeted pull injects `pull_ids` directly, skipping that filter. Defend the
  invariant at the injection point by reusing the same predicate.

### Code Quality

**Summary**: The plan is unusually well-reasoned and expressed in rich domain
language, with careful attention to reusing existing paths and phasing the change
so each step is independently mergeable. The main maintainability risks are a
now-stale `DiscoveryStatus` variant the plan never retires, a
`discovery_suppressed()` abstraction whose name and doc become misleading once
`Targeted` carries pull ids, and the fragmentation of two previously-single-source
invariants (exit-code precedence and all-offenders accumulation) across two code
sites. A couple of testability and comment-hygiene points round out the concerns.

**Strengths**:
- Renaming `suppressed_remote` to a collision-detection name that returns the
  colliding file is a genuine readability/DDD improvement.
- Deleting the whole `Suppressed` machinery rather than repurposing it is the
  right call — it removes a concept the domain no longer has.
- Keeps resolution filesystem-pure and isolates the remote probe as a separate
  pre-flight, preserving credential-independence.
- Threads `pull_ids` through the existing `ItemSelection::Targeted` lever rather
  than a parallel discovery path.

**Findings**:
- 🔴 major (high) — **Stale `SkippedTargeted` variant plus an unnoticed report-line
  change** (Phase 2 §2). After the change `SkippedTargeted` (its production site,
  its `discovery_line` arm at sync.rs:189, its doc comment) is unreachable dead
  code with a factually-wrong comment, and a fully-local targeted run emits
  "targeted pull imported 0" — a visible report change contradicting the
  mergeable-in-isolation claim. Decide explicitly: remove `SkippedTargeted` (and
  its arm and tests) in Phase 2, or keep it for `imported == 0` and branch.
- 🟡 major (medium) — **Two single-sourced invariants split by Phase 3**
  (Implementation Approach / Phase 3 §2). Exit-code precedence
  (`exit_code` + `highest_precedence_code`) and "one run names all offenders" (the
  single `failures` accumulator) are both split — remote-absent tokens accumulate
  in a second accumulator, and precedence is enforced partly by control-flow.
  Fold remote outcomes into the same vocabulary/accumulator (e.g. `Absent`,
  `Indeterminate` variants).
- 🔵 minor (medium) — **`discovery_suppressed()` misnomer** (Phase 2 §1 / Phase 3
  §2). Discovery is not suppressed, it is substituted by a targeted by-id pull;
  the plan even injects pull ids "at the `discovery_suppressed()` branch",
  entrenching the misnomer. Rename to reflect substitution and update the docs.
- 🔵 minor (medium) — **`fetch_all` gate placed inline in an already-oversized
  `run_sync`** (Phase 3 §2 / Testing Strategy). The partition/print/precedence
  logic is inline in a `#[allow(clippy::too_many_lines)]` function reachable only
  through a full `TrackerRegistry`, making the intended unit assertions hard to
  reach. Extract a pure function over a `FetchOutcome`.
- 🔵 suggestion (low) — **Plan snippets carry explanatory comments** (Phase 2 §2 /
  Phase 3 §2). `// existing discover_untracked path, unchanged` and the partition
  legend risk verbatim transcription into production, which the project forbids.
  Treat them as plan-only annotation.

### Test Coverage

**Summary**: The plan has a phase-by-phase testing structure with named automated
checks and clear happy/edge partitions, and Phase 2's engine-level coverage is
genuinely strong because it reuses the existing create-from-remote path. However,
it does not reckon with how Phase 3 breaks and inverts existing tests,
mis-locates its integration tests in a bin-only subprocess harness that provably
cannot stub the tracker, and leaves the full `run_sync` wiring seam and a real
two-tier-read error path without automated coverage.

**Strengths**:
- Each phase pairs named automated verification with manual verification, and
  Phase 2 is structured to be fully test-covered at the engine level.
- The Testing Strategy enumerates meaningful partitions of the new remote gate,
  not only the happy path.
- Calls out a negative-behaviour assertion ("a locally-resolvable token makes no
  remote call") and asserts all-or-nothing/zero-write invariants.
- Reusing `author_from_remote`/`create_from_remote` legitimately shrinks the
  surface a targeted-pull test must re-verify.

**Findings**:
- 🔴 major (high) — **Phase 3 inverts an existing subprocess test the plan never
  updates** (Phase 3 §1 / Testing Strategy).
  `a_no_match_target_exits_three_before_the_credential_check`
  (cli_sync_targets.rs:85) asserts exit 3 before the credential check; under
  Phase 3 a no-local-match token passes resolution and reaches `registry.resolve`
  (exit 74 with no credentials), and the `fetch_all` exit-3 now sits behind the
  credential check. Rewrite the test and file header, and add an in-crate
  `run_sync` test for the post-credential `absent` → exit-3 path.
- 🟡 major (high) — **Integration tests placed in a harness that cannot stub the
  tracker** (Testing Strategy). The harness is bin-only, and the gate lives after
  `registry.resolve`, so `--target PP-999` fails at the credential check, never
  reaching the stubbed pull/mixed-abort/preview behaviour. Relocate to in-crate
  tests calling `run_sync` with a stub `TrackerRegistry`.
- 🟡 major (high) — **Existing suppressed-remote tests encode the reversed
  behaviour; plan only "adds"** (Phase 1 §1-2).
  `a_local_id_wins_and_records_the_remote_match_as_suppressed` (sync.rs:906)
  asserts the opposite of the new exit-2 collision, and
  `the_suppression_line_collapses_record_breaking_whitespace` plus every
  `render_report(&report, &[])` call site depend on deleted machinery. List the
  exact tests to invert/delete as the red step.
- 🟡 major (medium) — **No testable seam for the `fetch_all` gate** (Phase 3 §2 /
  Success Criteria). The partition, precedence, and id-threading are inline in the
  oversized `run_sync`; the planned checks presuppose a way to drive the gate the
  plan never provides. Extract a pure function over `FetchOutcome` and cover the
  wiring with a stub-registry `run_sync` test.
- 🔵 minor (medium) — **Untested error path: `fetch_all` "found" then `show` fails
  during create** (Phase 2/3 create path). A two-tier read; an issue deleted
  between reads yields `Failed` (Retryable → exit 70). Add an engine test where
  the stub returns `found` from `fetch_all` but `Retryable` from `show`.
- 🔵 minor (medium) — **No coverage for a single batch yielding both `absent` and
  `indeterminate`** (Desired End State / Phase 3). Precedence `3 > 70` is asserted
  in prose but not by a test. Add a case where the stub carries both.
- 🔵 minor (medium) — **Planned `--max-pulls` test asserts refuse-all, but the AC
  describes "at most N created"** (Phase 2 / work item AC). Reconcile the AC with
  the refuse-all engine behaviour and assert whichever is intended.
- 🔵 suggestion (low) — **The "no remote call" assertion needs a recording stub**
  (Testing Strategy). Specify a stub that records `fetch_all` calls and assert the
  count is zero for a fully local-resolvable run.

### Compatibility

**Summary**: The plan is contract-aware in the right places: it routes the
existence check through `fetch_all` (provable `absent`) rather than `show`, and
preserves the untargeted `All` path structurally so the full-sync report and
write set stay byte-identical. Two contract seams are underspecified: the
discovery report line for existing no-pull targeted runs changes shape (breaking
the skill's `SkippedTargeted` renderer) and undermines the Phase 2 "no CLI-visible
change" claim, and injecting `pull_ids` at a branch that precedes the push-only
guard would make `--push-only --target <remote-only>` perform a pull. The
exit-code precedence also spans two mechanisms whose composition is not pinned.

**Strengths**:
- Honours the `RemoteTracker` trait contract: `fetch_all` for the existence gate
  because `show` cannot distinguish absence (lib.rs:413-425), mapping
  `indeterminate`/whole-call `Err` to retryable 70 (lib.rs:213-249, 439-454).
- The untargeted `All` path is preserved structurally — the `All` arm falls
  through to the unchanged discover branch, `reconciled()` returns the whole
  corpus, and the document-watermark advance still gates on `ItemSelection::All`.
- Reusing `create_from_remote` (apply.rs:270) preserves allocation parity.

**Findings**:
- 🔴 major (high) — **`TargetedPull` replaces `SkippedTargeted`; a non-`non_exhaustive`
  public enum is matched exhaustively by `discovery_line`** (Phase 2). Phase 2
  cannot compile without also updating the renderer (sync.rs:181-196), and the
  `SkippedTargeted` renderer in SKILL.md:285 would no longer match any run —
  contradicting the phase-independence guarantee. Retain `SkippedTargeted` for
  `imported == 0` or move the rendering update into Phase 2.
- 🟡 major (medium) — **Match ordering: push-only + targeted remote-only would
  pull** (Phase 2, match ordering). The `Targeted` arm precedes the `PushOnly`
  guard, so `pull_ids` drive `create_from_remote` even on a push-only run. Gate
  the candidate collection/injection on a pull-capable direction.
- 🔵 minor (medium) — **Four-way precedence spans two disconnected mechanisms**
  (Phase 3 §2 / Implementation Approach). The accumulator/`highest_precedence_code`
  runs before credentials and ranks only 2 and 6; the `fetch_all` gate producing 3
  and 70 runs after. How `absent` (3) reliably dominates `indeterminate` (70) in
  one batch is asserted, not mechanised. Check `absent` before `indeterminate` and
  state the two tiers are composable, adding 70 to the skill's precedence table.

### Safety

**Summary**: This plan extends targeted sync to create new local files from
remote data, and handles the core safety concerns well: it gates existence on
`fetch_all`'s provable `absent` partition (never on `show`), maps the
indeterminate case to a retryable exit, folds pull-creates into the existing
pre-write `--max-pulls` bound that refuses the whole run before any write, and
keeps local resolution failures credential-free ahead of the tracker call. The
main residual risks are language rather than mechanism: the plan repeatedly
claims "all-or-nothing" for a mixed batch, but that holds only at the pre-flight
gate — the per-id create loop is not transactional. The remaining observations
are minor and low-blast-radius given the target is version-controlled Markdown.

**Strengths**:
- The existence gate uses `fetch_all`, whose `absent` partition is provably
  complete, and routes `indeterminate`/transport failure to exit 70 — avoiding
  the "infer absence from a truncated fetch" hazard (lib.rs:221-247).
- Pull-creates fold into `pulls = plan.pull_count() + untracked.len()`
  (run.rs:822-833), which refuses the whole run with zero writes before apply, in
  both preview and apply modes.
- The `--preview` path returns before the apply block (run.rs:883-908), emitting
  report rows without calling `create_from_remote` — no `show`, no writes.
- New files are authored through an exclusive `LocalAuthor` write (apply.rs:263-289),
  so a collision errors rather than clobbering.
- Local failures are resolved credential-free before the tracker call
  (precedence 2 > 6 > 3 > 70), preserving the 0257 credential-independence
  invariant.
- Idempotence is sound: `create_from_remote` writes the baseline with
  `local_synced_at = run_start_epoch`, and `advance_document` stays false for
  targeted runs (run.rs:232), so a second run reconciles via the ordinary arm.

**Findings**:
- 🔴 minor (high) — **"All-or-nothing" holds at the gate, not across the apply
  loop** (Implementation Approach / Phase 3 §2). Once the `found` set enters the
  engine, the create loop (run.rs:929-940) applies each `create_from_remote`
  sequentially and continues past a failure, so a mid-batch failure leaves earlier
  files committed (recoverable, idempotent on re-run). Scope the wording to the
  gate and add a test proving a cleanly recoverable state.
- 🔵 minor (medium) — **Refuse-whole-run bound is safer than the AC's "at most N
  created" — reconcile them** (Phase 2 / AC #9). Reuses `RunError::Refused`
  (run.rs:824), refusing the entire run with zero writes; AC #9 implies partial
  creation. State the refuse-all bound explicitly and annotate AC #9.
- 🔵 minor (medium) — **TOCTOU gap between `fetch_all` found and
  `create_from_remote` show** (Phase 3 §2-3). An id proven `found` is re-fetched
  by `show` (apply.rs:275), which can never report absence; a deleted-in-the-window
  issue returns Retryable indefinitely. Confirm and test that this surfaces as a
  non-zero exit with an explicit per-id failure line and never advances a
  watermark.
- 🔵 minor (medium) — **A typo'd remote-only target is no longer diagnosable
  offline** (Phase 3 §1 / Desired End State). A bare typo now requires a reachable
  tracker to classify; offline it surfaces as exit 70 rather than exit 3. Note the
  change in the skill and steer the exit-70 message toward both "unreachable" and
  "check the key".

### Usability

**Summary**: The plan is unusually disciplined about operator experience: it
fixes the collision message to name both files, preserves `--preview` zero-write
semantics, keeps the all-or-nothing batch abort, and explicitly demands no raw
TSV leak in manual verification. The main gaps are documentation-surface
consistency in the skill (a second exit-code block is left un-updated), a
report-line word ("imported") that misleads under `--preview`, and an unremarked
shift in the operator's mental model for exit 3, which now depends on tracker
reachability.

**Strengths**:
- The collision error names both files and how to disambiguate, turning a silent
  local-wins note into an actionable exit-2 error.
- Exit 70 (RETRYABLE) is reused for the indeterminate remote read, consistent
  with its existing meaning, and added to the skill's abort table.
- The `--preview` affordance is preserved end-to-end: a would-create with zero
  writes that still counts against `--max-pulls`.
- Manual verification explicitly checks human phrasing with no raw TSV leaking,
  and the all-or-nothing batch abort names every absent token.

**Findings**:
- 🟡 major (medium) — **Second exit-code block in the skill (Step 2, lines
  144-157) is left un-updated** (Phase 1 §3 / Phase 3 §4). Step 2 (153-156)
  describes exit 3 as "no match", omits the new exit-70 case, and is not in any
  change item — contradicting the Step 1 block. Add it to the Phase 3 skill
  changes.
- 🟡 major (medium) — **"imported N remote-only item(s)" misleads under
  `--preview`** (Phase 2 §2 / Phase 3 §4). Past-tense `imported` / field
  `imported`, but under `--preview` nothing is written; the existing line uses
  neutral `found=N`. Make the phrasing preview-aware.
- 🔵 minor (medium) — **Collision message should identify which file is the
  id-match vs the external_id-match** (Phase 1 §1). Two bare paths plus "pick one"
  leaves the operator guessing. Label each file's role.
- 🔵 minor (medium) — **Exit 3 for a mistyped remote key now requires working
  credentials — unremarked mental-model shift** (Implementation Approach / Phase 3
  §2). With missing credentials the operator gets exit 74, and the skill's exit-3
  guidance no longer fully fits. Update the recovery text.
- 🔵 suggestion (low) — **Wire (TSV) format of the new `TargetedPull` status is
  not specified for the skill to translate** (Phase 3 §4). If the Step 5
  translation table cannot match the emitted token, raw TSV leaks. Pin the emitted
  shape and add the matching entry.

### Documentation

**Summary**: The plan documents the new resolution model clearly through a
deterministic five-case table in Desired End State, and gives concrete SKILL.md
line anchors for each rewrite. However, the exit-code documentation is not fully
reconciled across the skill's several duplicate locations: the Step 1 precedence
summary and the Step 2 target-abort summary are left stale, the suppressed-remote
lead-in sentence is orphaned, and the now-superseded "skipped targeted" discovery
rendering is never removed. Several Phase 1/Phase 3 line citations are also
imprecise and overlap on the same sentence.

**Strengths**:
- The Desired End State resolution table (99-107) documents the new per-token
  behaviour deterministically across all five outcomes.
- Each SKILL.md rewrite is anchored to concrete line ranges.
- Phase 1's Manual Verification explicitly checks for no dangling suppressed-note
  reference.
- Exit-code precedence and internal assignments are stated consistently across
  Overview, Desired End State, and both phases.

**Findings**:
- 🟡 major (high) — **Exit 70 not added to the Step 1 precedence (100-101) and
  Step 2 (153-156) blocks** (Phase 3 §4). Step 1 reads "2 > 6 > 3" and must become
  "2 > 6 > 3 > 70"; Step 2 lists only "3, 6, or 2" with no exit-70 case. Neither
  is in the change set.
- 🟡 major (high) — **Suppressed-remote lead-in sentence (281-283) orphaned**
  (Phase 1 §3). The lead-in ("...or a suppressed-remote note, render them...")
  references a rendering deleted at 287-289 but is not in the change set. Add it so
  the sentence is rewritten when the bullet is removed.
- 🟡 major (high) — **Overlapping line citations Phase 1 vs Phase 3** (Phase 1 §3
  vs Phase 3 §4). Phase 1 cites 81-83 but the "suppressed" sentence spans 82-84;
  Phase 3 separately claims 84-86. The two phases misattribute overlapping edits
  on line 84. Correct the citations or note Phase 3 must re-base after Phase 1.
- 🟡 major (medium) — **`TargetedPull` vs "skipped targeted" dead documentation**
  (Phase 2 §2 / Phase 3 §4). The Step 5 bullet at 285-286 ("Discovery skipped:
  targeted run over N item(s)") and the Step 2 narrative at line 138 are left
  untouched. Clarify whether "skipped targeted" survives for `imported == 0`; list
  the updates either way.
- 🔵 minor (medium) — **Phase 1 prose rewrite addresses only the collision case**
  (Phase 1 §3). It should also state that a token equal to a single file's own
  `external_id` reconciles silently.
- 🔵 minor (low) — **Resolution table omits exit-6 and ambiguous exit-2 cases**
  (Desired End State, 99-107). The table reads as exhaustive but omits codes that
  appear in the precedence chain. Add the rows or annotate the table's scope.

## Re-Review (Pass 2) — 2026-09-08

**Verdict:** REVISE

All eight lenses ran again against the revised plan. Every finding from the
initial review is resolved — the structural blockers are gone. The verdict stays
REVISE only because the edits introduced a cluster of implementation-precision
findings, none structural: a count field that carries the wrong quantity, a code
sketch that does not type-check, a deferred renderer arm that breaks Phase 2's
compile, a guard placed one layer from where it protects, and test gaps for
newly-added branches. Several are naturally resolved during TDD; four are
design-level and worth a focused second edit.

### Previously Identified Issues

- 🟡 **Architecture/Code Quality/Correctness/Compatibility**: `SkippedTargeted` →
  `TargetedPull{0}` swap — **Resolved**. `TargetedPull` is now emitted only when
  `pull_ids` is non-empty; an empty slice keeps `SkippedTargeted`, so a
  reconcile-only run is byte-identical.
- 🟡 **Correctness/Architecture/Code Quality/Compatibility**: Exit-code precedence
  split + absent/indeterminate tie — **Resolved**. `Absent`/`Indeterminate` are
  now `TargetResolutionFailure` variants through one `rank` function; `rank` is
  extended so 3 beats 70.
- 🟡 **Correctness**: Duplicate candidates double-bind — **Resolved** (dedup by
  `canonical_external_key` + corpus filter), though the filter placement drew a
  new finding (see below).
- 🟡 **Compatibility**: `--push-only` + remote-only pulls — **Resolved**. Now an
  exit-2 usage error before any tracker call.
- 🟡 **Test Coverage**: gate untestable / existing tests inverted / no seam —
  **Resolved**. The subprocess harness is scoped to credential-independent
  assertions; the gate is a pure `partition_candidates`; existing tests are
  listed for inversion. (Test coverage confirms the in-crate seam is genuinely
  supported by existing `RecordingTracker`/`RecordingAuthor`/`Spy`.)
- 🟡 **Documentation/Usability**: incomplete skill change set — **Resolved**. All
  exit-code surfaces, the orphaned lead-in, the citations, and the pinned TSV
  shape are now enumerated (residual line-reference precision noted below).
- 🔵 **Safety**: "all-or-nothing" scoping, TOCTOU, refuse-all — **Resolved** and
  explicitly documented; `--max-pulls` AC reworded in the work item.
- 🔵 Minor/suggestion items (discovery_suppressed rename, preview wording, exit-3
  offline shift, own-`external_id` prose) — **Resolved**.

### New Issues Introduced

#### Major

- 🟡 **Count is attempted, not applied** (code-quality, architecture,
  compatibility, usability, correctness, test-coverage, documentation) — The most
  cross-cutting new issue. `DiscoveryStatus::TargetedPull { imported }` carries
  `pull_ids.len()` (attempted) and drives the TSV `#\tdiscovery\ttargeted-pull\t<N>`,
  while the human line is told to derive N from applied outcomes. On a
  partial-failure run the two diverge and overstate success; under `--preview`
  there are no applied outcomes at all. The field name `imported` compounds it.
  Fix: single-source the count (carry the applied count, or drop N from the TSV),
  rename the field to `attempted`, and state the skill's N source per mode.
- 🟡 **`partition_candidates` does not type-check** (code-quality, compatibility,
  correctness) — `FetchOutcome::found` is `Vec<(ExternalId, RemoteTimestamp)>`,
  not `Vec<ExternalId>`; the sketch returns it directly, and maps bare ids into
  `Absent(String)`/`Indeterminate(String)` which must carry a user-facing message
  per the `message()` invariant. Fix: project the id out of the pair and build a
  message per token.
- 🟡 **Phase 2 will not compile** (correctness) — Adding
  `DiscoveryStatus::TargetedPull` makes `discovery_line`'s exhaustive match
  non-exhaustive, yet `mise run cli:check` is a Phase 2 criterion and the renderer
  arm is deferred to Phase 3. Fix: add the `discovery_line` arm in Phase 2 (a
  provisional TSV line Phase 3 finalises).
- 🟡 **Double-binding filter placed at the CLI caller, not the engine boundary**
  (architecture; untested per test-coverage) — The filter sits in work-cli, but
  the engine trusts `pull_ids` verbatim at the injection branch beside its own
  `corpus_carries` closure. A second caller could reintroduce double-authoring.
  Fix: apply the corpus filter inside the engine so `Targeted { pull_ids }` is
  safe by construction, and cover it with a test.
- 🟡 **Whole-call `fetch_all` `Err` → exit 70 misclassifies** (compatibility;
  safety concurs) — Per the tracker contract, `fetch_all` errors only on a
  pre-flight fault (unresolvable credentials, an unembeddable id) — permanently
  unfixable by retry. Mapping to RETRYABLE (70) tells the operator to re-run when
  re-running cannot help. Fix: route an unembeddable-id/credential fault to a
  usage/unconfigured code, or document why it is deliberately retryable.
- 🟡 **Test gaps for new behaviour** (test-coverage) — No test for the AC7
  all-success mixed batch (local + path + remote-only in one run, asserting the
  write set is exactly the targeted union), the double-binding filter, or the
  partial-failure count.
- 🟡 **Skill line references and the phase-boundary sentence split are inaccurate**
  (documentation) — The suppressed-note and discovery-suppression sentences do not
  sit at the cited lines, and the 84/85 split cuts one sentence in half. Fix:
  reference by sentence content, not line number, and assign each whole sentence
  to one phase.
- 🟡 **Step 2 discovery enumeration not updated** (documentation) — The skill's
  Step 2 lists the discovery states (ran/skipped-push-only/skipped-targeted/failed)
  but the change set never adds the new `targeted-pull` state to it.

#### Minor

- 🔵 **`ItemSelection::Targeted` has three test construction sites** beyond the CLI
  (compatibility, high) — `sync_run.rs:342,480` and `sync_create.rs:275` use the
  tuple form and must migrate in Phase 2, or Phase 2 will not build. Consider
  whether `cargo-public-api` needs regenerating.
- 🔵 **Push-only and collision messages under-specified** (usability) — The
  push-only error should state the contradiction and remedy; the collision message
  should carry actual file paths (not ids) for the path-based recovery.
- 🔵 **`--preview` change-class list missing targeted-pull** (usability) — Step 1's
  enumeration of previewable actions does not add the new targeted pull-create.
- 🔵 **exit-70 recovery text and write-semantics** (documentation, usability) — The
  new exit-70 skill entry omits recovery guidance, and a targeted exit-70 (aborts
  before any write) is not distinguished from the engine-level exit-70 (may follow
  partial writes).
- 🔵 **Rewritten collision test keeps the old name** (code-quality) — rename
  `a_local_id_wins_..._as_suppressed` to describe the new exit-2 behaviour.
- 🔵 **Confirmed target silently dropped by the corpus filter** (correctness) — if
  the defensive filter ever fires, the named target is neither pulled nor
  reconciled with no signal; route it to reconcile or emit a diagnostic.
- 🔵 **Baseline-write partial state untested** (safety) — a create whose baseline
  write fails after the file is authored is a distinct partial state; add a
  recovery test.
- 🔵 **AC2 allocation parity has no comparison test** (test-coverage) — assert a
  discovery import and a targeted pull of the same stub issue author identical
  id/filename/baseline.

#### Not a defect

- ⚪ **Desired End State precedence note** (flagged by documentation as still
  `2 > 6 > 3`) — false positive. Plan line 99 already reads `2 > 6 > 3 > 70`; the
  `2 > 6 > 3` the lens saw is the SKILL.md copy the plan (line 445) correctly
  instructs changing.

### Assessment

The plan is now structurally sound: the resolution model, the phase decomposition,
the exit-code single-sourcing, and the test seam are all sensible, and every
original finding is resolved. The new findings are precision issues — four
design-level ones worth fixing in the plan text (single-source the count; move the
double-binding filter into the engine; correct the `fetch_all` `Err` mapping; add
the Phase 2 `discovery_line` arm and enumerate the enum construction sites), and a
tail of mechanical ones (pseudocode types, skill line references, test-name and
Step 2 enumeration) that a careful implementer resolves during TDD. A focused
second edit on the design-level four would carry this to APPROVE.

### Verdict Update (2026-09-08) — APPROVE

A second edit round applied **every** Pass-2 finding to the plan, not only the
design-level four: the count is single-sourced on an `attempted` field with the
discovery line reporting "requested/would import N" (never "imported"); the
double-binding filter moved into the engine at the injection branch; the
`fetch_all` whole-call `Err` is split by fault class (exit 2 / 74 / 70); the
Phase 2 `discovery_line` arm and the three enum construction sites are enumerated;
`partition_candidates` type-checks; the `PushOnlyRemoteOnly` variant is named; the
skill edits are anchored by sentence with the Step 2 enumeration, exit-70 recovery
text, and `--preview` class added; and tests were added for the engine filter,
AC7, AC2 parity, the partial-failure count, the baseline-write state, and the
`Err`-class. The one documentation "major" was a false positive.

This verdict was set by the reviewer after applying those fixes; it was **not**
produced by a third automated lens pass. The plan is approved and marked ready
for implementation.

---
*Re-review generated by /accelerator:review-plan*
