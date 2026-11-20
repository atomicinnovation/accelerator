---
type: "plan-review"
id: "2026-11-17-0351-sync-preview-hides-pending-push-refusals-review-2"
title: "Plan Review: Sync Preview Reports Pending-Push Refusals Implementation Plan"
date: "2026-11-17T14:48:34+00:00"
author: "Phil Helm"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-17-0351-sync-preview-hides-pending-push-refusals"
relates_to: ["plan-review:2026-11-17-0351-sync-preview-hides-pending-push-refusals-review-1"]
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "compatibility", "documentation"]
review_number: 2
review_pass: 1
tags: ["sync", "preview", "pending-push"]
last_updated: "2026-11-17T14:48:34+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Plan Review: Sync Preview Reports Pending-Push Refusals Implementation Plan

**Verdict:** COMMENT

The revision addressed every major finding from review 1, and most of its
minor findings. The engine tests now check the `Spy` for `pending-push/`
writes, there is a guard test for ordered creates and relinks, and the
work-file fixtures fail as planned. The marker is read once at the edge, and
`bare_create_decision` is a pure function. No lens raised a critical or major
finding this time. What remains is a set of minor gaps:

- Some public text claims more than the change delivers: the CLI help, the
  user guide, the changelog, and the 70-to-71 sentence.
- Several tests are weaker than the Testing Strategy says.
- The plan's reason for passing the whole request into the decision does not
  hold.
- A preview checks the corpus before identity settlement, which leaves a
  narrow parity gap.

The plan is acceptable as it stands. The minor findings below would improve
it.

### Relation to Review 1

All three of review 1's major findings are resolved:

- **Engine assertions could not detect a preview write: resolved.** The tests
  now check `fixture.spy.writes` and `fixture.spy.removals` and use
  `assert_nothing_sent`.
- **No test pinned ordered creates and relinks: resolved.**
  `a_preview_leaves_an_ordered_create_to_its_order` was added. A weaker form
  of the concern recurs: the guard's assertion is too loose (see Minor).
- **The no-title fixture did not fail: resolved.** The engine fixtures now use
  undecodable bytes (`ReadLocal`) and an unclosed fence (`ParseLocal`), the
  CLI fixture is an `id`-bearing unclosed fence, and the work item's
  acceptance criterion now says "cannot be parsed".

These minor findings and suggestions from review 1 are also resolved:

- the parity test now normalises paths;
- the CLI undecodable-file seeding was replaced;
- the whole-corpus 70-to-71 change is pinned by a test;
- red tests are split from guard tests;
- the unsafe-id and `Unknown` tracking cases are covered;
- the 0377 coordination is planned;
- `read_marker` became `MarkerReading::read` with the correct caller count;
- the marker is read at the edge;
- the decision keeps `Proceed`/`ReuseId` as `BareCreateDecision`;
- `pending_push_tree` now matches the shape of `meta_tree`;
- the second skill statement of preview coverage is aligned;
- `LocalCreate::is_bare()` was added;
- the method was renamed to `create_request`;
- there is a reusable-marker check at CLI level;
- the fixture order for the conflict test is set;
- the user guide is mentioned;
- the term "blocked" was replaced;
- Current State now describes `refused_orders` and `previewed_adoptions`
  accurately.

These recur in a new form:

- **The new guarantee covers every create.** The skill text is now scoped
  correctly, but the new CLI help sentence, user-guide sentence and changelog
  entry are not scoped.
- **The changelog omitted 70 to 71.** It is now included, but stated too
  broadly (`--pull-only` still exits 70), and it leaves out the unsafe-id
  exit. The Breaking-vs-Fixed choice was made, but in a "see Fixed" form that
  has no precedent in the changelog.
- **The digest was computed in two places.** `CreateFromLocalRequest::digest()`
  was added, but the digest is still passed through `settle_order` and
  `create_anew` as a parameter.
- **A full write-capable request for a read-only decision.** The tradeoff is
  now recorded, but its stated reason ("no ports, so it cannot write") is
  wrong, because the request carries the `LocalAuthor` port.
- **The CLI help added nothing.** A positive clause was added, but it
  overclaims and reads awkwardly.
- **The shape of `pending_push_tree`.** The shape is fixed, but its
  `String`-valued snapshot cannot see a non-UTF-8 marker.

### Cross-Cutting Themes

- **Public text still implies that ordered creates are covered** (flagged by:
  compatibility, documentation). The skill correctly says "every create
  without a `--resolve` order". The new CLI help sentence, the user-guide
  sentence and the changelog Fixed entry do not, so a reader can take a clean
  preview of an `adopt:` or `create` order as a guarantee.
- **The 70-to-71 sentence is too broad** (flagged by: documentation,
  correctness). The Fixed entry and Migration Notes say every whole-corpus
  preview that exited 70 for the discovery deferral now exits 71. Under
  `--pull-only` no create is evaluated, but discovery is still deferred, so
  the preview still exits 70.
- **The changelog split is unusual** (flagged by: documentation, standards,
  compatibility). The Breaking entry says only "see Fixed". Elsewhere in the
  changelog, Breaking entries carry the exit codes themselves, and Fixed
  entries are short. Compatibility adds that the pointer covers only "a
  refused create", although the unsafe id and unusable work files also change
  their exit codes.
- **The read-only guarantee is not enforced by the types** (flagged by:
  architecture, code-quality). `CreateFromLocalRequest` carries the
  write-capable `LocalAuthor`, and `previewed_outcome` takes the whole of
  `SyncPorts`. The plan's reason for its tradeoff therefore rests on a false
  premise.
- **"Bare" and the routing between bare and ordered creates** (flagged by:
  standards, code-quality, architecture). "Bare" is new vocabulary next to the
  existing "plain create". The run decides the routing with `request.order`
  and the preview with `create.is_bare()`, and `bare_create_decision` silently
  assumes it is never given an order.

### Tradeoff Analysis

- **Shared domain term vs codebase naming**: code-quality praises
  `BareCreateDecision` because it matches the work item's term, "bare
  create". Standards notes that "bare" appears nowhere in the code, which
  already says "plain create", and suggests a name built on "order" (for
  example `carries_no_order()`). Recommendation: pick one term and use it
  everywhere. Either keep "bare" and rename the `relink_order` doc's "plain
  create" to match, or adopt an "order"-based name in code, prose and skill
  text alike.
- **A narrow, port-free decision input vs one construction point**:
  architecture asks `LocalCreateInputs` to produce a port-free view for the
  decision. The plan prefers passing the whole request, so that
  `corpus_blocked_by` and `tracker_name` cannot drift. Both aims can be met:
  derive the narrow view from the single construction point. At minimum,
  correct the stated reason.
- **A dedicated gate module vs a minimal diff**: architecture suggests moving
  `MarkerReading`, `bare_create_decision` and `pending_push_refused` into a
  sibling module that mirrors `send_refusal.rs`. That costs a larger refactor
  in Phase 1, but gives 0379 one unit to extend.

### Findings

#### Minor

- 🔵 **Architecture / Code Quality**: The "no ports, so it cannot write"
  reason does not hold, and `previewed_outcome` takes all of `SyncPorts`
  **Location**: Implementation Approach; Phase 2, Section 3
  `CreateFromLocalRequest` carries `author: &dyn LocalAuthor`, which writes
  work files, and `previewed_outcome` receives the tracker and
  integration-state ports while using only `author` and `files.tracking`. The
  preview's read-only guarantee is enforced only by tests.
- 🔵 **Test Coverage**: The ordered-create guard's negative assertion is too
  loose
  **Location**: Phase 2, Section 1: `a_preview_leaves_an_ordered_create_to_its_order`
  The guard checks only that the row is not `Failed(PendingPushRefused)`, with
  well-formed work files. Running `gather` before the `is_bare()` check, or
  reporting `CorpusIncomplete` for an ordered create, would still pass. Assert
  `NotApplied` exactly, and add an ordered create and a relink over an
  unclosed fence.
- 🔵 **Test Coverage**: Most CLI preview tests never check the
  `pending-push/` snapshot
  **Location**: Phase 2, Section 2; Testing Strategy
  The Testing Strategy says each CLI test is checked against
  `pending_push_tree`. Only two tests do, and the six parity seedings,
  including the reusable `Created` marker, are not among them.
- 🔵 **Documentation / Correctness**: The 70-to-71 claim is too broad
  **Location**: Phase 3, Section 4: Changelog; Migration Notes
  A `--pull-only` preview, or one with a legacy marker on a draft id, still
  exits 70, because no create is evaluated while discovery is still deferred.
  Scope the sentence to previews that plan the create.
- 🔵 **Compatibility / Documentation**: The CLI help, the user guide and the
  changelog imply that ordered creates are covered
  **Location**: Phase 3, Sections 2–4
  The new help sentence ("A preview reports a create the run would refuse from
  its pending-push marker.") is unscoped and reads awkwardly, and so are the
  guide and changelog wording. Scope them to "a create without a `--resolve`
  order", as the skill is, or drop the help sentence.
- 🔵 **Architecture**: The shared preview/run gate goes in the applier module
  **Location**: Phase 1, Section 1
  `send_refusal.rs` is the existing precedent for a gate that both modes
  share. Putting `MarkerReading`, `bare_create_decision` and
  `pending_push_refused` in `apply.rs` widens that module's crate surface and
  gives `run.rs` a dependency on applier internals.
- 🔵 **Code Quality / Architecture**: `bare_create_decision` relies on an
  unchecked precondition, and the bare/ordered routing is decided in two
  places
  **Location**: Phase 1, Section 1; Phase 2, Section 3
  The decision ignores `request.order`. The run routes on `request.order` and
  the preview on `create.is_bare()`. A future caller, such as 0379, could
  pass an ordered request and get a bare-create answer.
- 🔵 **Code Quality**: The digest is both derived from the request and still
  passed as a parameter
  **Location**: Phase 1, Section 1
  The sample still passes `request.digest()` into `settle_order`, which hands
  it to `order_precondition` and `create_anew`. Choose one approach.
- 🔵 **Correctness**: The preview checks the corpus before identity
  settlement, but the run checks it after
  **Location**: Key Discoveries; Desired End State; Phase 2, Section 3
  In apply mode, draft promotions and retirements rewrite references, moved
  keys are followed, and the corpus is rediscovered before creates run. A
  body that references a promoted draft, or a marker that records a synced
  item's old key, can therefore preview differently from the run. Record this
  as a limitation and narrow the "exactly" wording.
- 🔵 **Test Coverage**: A `String`-valued snapshot skips a non-UTF-8 marker
  **Location**: Phase 2, Section 2: `pending_push_tree`
  Copying `meta_tree` drops files that `read_to_string` cannot decode. An
  unreadable-marker seeding with invalid UTF-8 would therefore be invisible
  to the "unchanged" check. Snapshot bytes, or require valid-UTF-8
  unreadable markers.
- 🔵 **Test Coverage**: No preview test covers a work file that cannot be
  read (retryable `Io`, exit 70)
  **Location**: Phase 2, Section 1; Phase 3, Section 4
  The changelog promises exit 70 for this case. Add the removed-file case to
  the unusable-create table, asserting `Retryable` and `ReadLocal`.
- 🔵 **Standards**: "Bare create" is new vocabulary that does not match the
  existing names
  **Location**: Phase 1, Sections 1 and 2
  The code already says "plain create" (`run.rs:1253`), and the neighbouring
  predicates (`relinks()`, `adopts()`) have no `is_` prefix. See Tradeoff
  Analysis.
- 🔵 **Standards**: The plan does not say what visibility `MarkerReading` gets
  **Location**: Phase 1, Section 1
  `MarkerReading` is a private enum today. State that it,
  `BareCreateDecision` and `bare_create_decision` are `pub(crate)`. Otherwise
  an implementer may make them `pub`, widening `apply`'s public surface, or
  hit the `private_interfaces` lint.
- 🔵 **Documentation / Standards**: The Breaking entry is a "see Fixed"
  pointer with no precedent in the changelog
  **Location**: Phase 3, Section 4
  Follow the split the changelog already uses: a full Breaking entry that
  carries the exit codes and the "Scripts that parse…" line, and a short
  Fixed entry.
- 🔵 **Documentation**: The new user-guide sentence separates "The refusal
  names one reason:" from the sentence it follows
  **Location**: Phase 3, Section 3
  Place the sentence after the reason table or the remedy paragraph.

#### Suggestions

- 🔵 **Code Quality**: Write `previewed_outcome` with `?` instead of a nested
  `and_then` closure
  **Location**: Phase 2, Section 3
- 🔵 **Test Coverage**: Scope the skill test's "both statements" assertion to
  the Step 2 and Step 5 sections
  **Location**: Phase 3, Section 1
- 🔵 **Test Coverage**: Make the precedence test assert the rows, not just the
  exit code
  **Location**: Phase 2, Section 2: `a_previewed_refusal_outranks_a_conflict`
- 🔵 **Standards**: Give the `SyncRequest` parameter one name, `request`, in
  both methods of `LocalCreateInputs`
  **Location**: Phase 1, Section 2
- 🔵 **Standards**: Name the new skill clause's failures by their row tokens,
  as the rest of the sentence does
  **Location**: Phase 3, Section 1
- 🔵 **Compatibility**: The exit-code list leaves out the unsafe-id case, and
  the Breaking pointer is narrower than the change
  **Location**: Phase 3, Section 4; Migration Notes
- 🔵 **Documentation**: The ordered-create caveat tells the agent about the
  limit but not what to tell the user
  **Location**: Phase 3, Section 1

### Strengths

- ✅ Preview–run parity holds by construction for bare creates. Both paths
  build the request through `LocalCreateInputs` and call one pure
  `bare_create_decision` over a `MarkerReading` read once at the edge.
- ✅ Phase 1 is a refactor with no behaviour change, protected by the existing
  `sync_create.rs` suite, which already exercises both `read_marker` callers
  and the digest-dependent reuse path.
- ✅ `LocalCreateInputs` replaces an anonymous `(String, String, String)`
  tuple, and `previewed_outcome` keeps the new logic out of the already
  over-long `execute`.
- ✅ The preview copies the `validate_pushes` pattern and reuses the existing
  rows, tokens and exit-code precedence. There is no CLI change and no new
  output vocabulary.
- ✅ Red tests and guard tests are separated. The engine tests mirror the run's
  refusal, distrust, deferral and unusable-file tables, and the engine write
  checks now go through the `Spy`.
- ✅ The `sync_settled.rs` test runs in both modes and adds the first
  apply-side coverage of `FollowedThisRun`.
- ✅ The CLI parity test uses fresh repositories and normalises the root, with
  absolute-value tests beside it.
- ✅ The gap for ordered creates and relinks is explicit, justified by the
  dossier-shape change, and tied to 0379. The skill text states the limit.
- ✅ All three copies of the workaround sentence and its content test are
  retired together, and both skill statements of preview coverage are
  updated.
- ✅ The exit-code change reaches script authors through the changelog and
  Migration Notes, and the 0377 coordination is a pre-merge check.
- ✅ The new items are crate-private, `work-adapters` is not pinned by
  `public-api:check`, and there are no comments in the proposed code.

### Recommended Changes

1. **Scope the public wording to creates without a `--resolve` order**
   (addresses: CLI help, user guide and changelog imply ordered creates are
   covered; the ordered-create caveat lacks an instruction)
   Reword or drop the CLI help sentence and update golden line 211 to match.
   Scope the guide and changelog sentences. In the skill, add "so a clean
   preview does not confirm it" to the ordered-create caveat.
2. **Rework the changelog and Migration Notes** (addresses: the 70-to-71
   claim is too broad; the Breaking "see Fixed" pointer; the unsafe-id case
   is missing)
   Move the exit codes (71, 75, 70, including the unsafe id) and the
   "Scripts that parse…" line into a full Breaking entry. Keep Fixed short.
   Scope the 70-to-71 sentence to previews that plan the create, or say "the
   preview exits as the run would".
3. **Tighten the tests** (addresses: the loose ordered-create guard; the
   `pending-push/` snapshot missing on most CLI tests; the non-UTF-8
   snapshot; the work file that cannot be read; the precedence test; the
   skill-test scoping)
   Assert `NotApplied` exactly for ordered rows, and add an unclosed-fence
   ordered create and relink. Snapshot `pending_push_tree` around every CLI
   preview. Snapshot bytes. Add the removed-file `Retryable`/`ReadLocal` case.
   Assert the rows in the conflict-precedence test. Check the skill's Step 2
   and Step 5 sections separately.
4. **Make the read-only boundary honest** (addresses: the "no ports"
   reason; `previewed_outcome` taking all of `SyncPorts`)
   Pass `previewed_outcome` only `author` and `tracking`. Either derive a
   port-free decision view from `LocalCreateInputs`, or correct the reason
   recorded for the tradeoff.
5. **Record the identity-settlement limitation** (addresses: the preview
   checks the corpus before identity settlement)
   Add it to "What We're NOT Doing" and narrow "exactly" in Desired End State.
6. **Settle the Phase 1 seam details** (addresses: the digest is both derived
   and passed; the unchecked precondition and two routing checks; the "bare"
   vocabulary; `MarkerReading` visibility; the gate's placement; parameter
   naming)
   Choose one way to supply the digest. State or assert that
   `bare_create_decision` is given no order. Pick one term ("bare" or an
   "order"-based name) and use it everywhere. State that the new items are
   `pub(crate)`. Optionally move the gate into a sibling module beside
   `send_refusal.rs`. Name the `SyncRequest` parameter `request` throughout.
7. **Move the user-guide sentence** after the reason table or the remedy
   paragraph (addresses: guide sentence placement).

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. It gives the run and the preview
one decision for a bare create. The marker is read once at the edge
(`MarkerReading::read`), and the mapping (`bare_create_decision`) is a pure
function, which puts the functional core / imperative shell split where
review 1 asked for it. The CLI and exit-code paths stay untouched because they
are already shared. The remaining concerns are about the boundary:

- The recorded reason for passing the whole request ("no ports, so it cannot
  write") does not hold, because the request carries the write-capable
  `LocalAuthor` port.
- The new shared gate lives in the applier module, not in a dedicated refusal
  module like `send_refusal.rs`.
- The preview and the run encode the bare/ordered routing in two different
  places.

**Strengths**:
- Preview–run parity holds by construction. Both paths call
  `bare_create_decision` and build the request through `LocalCreateInputs`, so
  `corpus_blocked_by`, `tracker_name` and the digest inputs cannot drift
  between modes.
- The marker is read once, at the edge, before the order branch, and the
  decision takes the reading as an argument. The decision is pure and can be
  unit-tested without a filesystem, and it matches how `settle_order` already
  receives its reading.
- Phase 1 is a behaviour-preserving refactor protected by the existing
  `sync_create.rs` suite. The behaviour change comes separately, in Phase 2,
  with red and guard tests.
- The preview follows the established `validate_pushes` pattern. Rendering and
  exit-code derivation need no new path.
- Hoisting the `carrier` closure is backed by checking that `SettledView` is
  built from `plan.key_changes` in both modes.
- The gap for ordered creates and relinks is acknowledged, scoped in the skill
  text, and tied to 0379, with a stated reason.

**Findings**:
- 🔵 minor (high): **The 'no ports, so it cannot write' reason does not hold
  for the request the decision receives**. Location: Implementation Approach;
  Phase 2, Section 3. `CreateFromLocalRequest` carries
  `author: &'a dyn LocalAuthor` (apply.rs:552), whose `link_external_id` and
  `author_from_remote` write work files. The preview fills it with the real
  `ports.author`, and `previewed_outcome` takes the whole of `&SyncPorts`. The
  read-only guarantee holds only by convention and tests. Suggestion: project
  a narrow, port-free view from `LocalCreateInputs` and pass `previewed_outcome`
  only `RepositoryTracking`, or correct the reason.
- 🔵 minor (medium): **The shared preview/run gate is placed in the applier
  module, not beside `send_refusal`**. Location: Phase 1, Section 1.
  `send_refusal.rs` is a dedicated module for a gate that both modes evaluate.
  Putting `MarkerReading`, `bare_create_decision` and `pending_push_refused` in
  `apply.rs` gives `run.rs` a dependency on applier internals and complicates
  the move to 0379. Suggestion: a sibling module such as
  `sync/create_refusal.rs`.
- 🔵 suggestion (medium): **The preview and the run encode the bare vs ordered
  routing separately**. Location: Phase 2, Section 3; Phase 1, Section 2. The
  preview uses `create.is_bare()`, and the run uses `create.relink_order()`
  then `request.order`. Suggestion: one routing method on `LocalCreate` (an
  enum `Bare | Ordered | Relink`) that both paths match on.

### Code Quality

**Summary**: The plan removes a real duplication risk by giving the run and
the preview one pure decision for a bare create. The tuple returned by
`create_inputs` becomes a named `LocalCreateInputs`, and the preview's new
logic sits in its own function rather than growing the over-long `execute`.
Most of review 1's points have been taken up. A few small problems are left:

- the digest is still passed through `settle_order` beside the new
  `request.digest()`;
- `bare_create_decision` relies on a precondition it never checks;
- the preview helper takes more context than it uses.

**Strengths**:
- There is one source of truth for the bare-create decision, called by both
  modes.
- Phase 1 is a pure refactor done before any behaviour change, with a manual
  check that no assertion changes.
- `LocalCreateInputs` replaces the anonymous tuple and the separate
  `CreateStatePaths` check with one named type, so both paths build
  `CreateFromLocalRequest` the same way.
- `previewed_outcome` stays out of `execute`, which already carries
  `#[allow(clippy::too_many_lines)]`.
- The Refuse arm is moved unchanged into `pending_push_refused`.
- Keeping `Proceed`/`ReuseId` on `BareCreateDecision` matches the
  `PushPrecondition` vocabulary and the work item's term, "bare create".
- The plan records the tradeoff of passing the wider request.

**Findings**:
- 🔵 minor (high): **The digest is both derived from the request and still
  passed as a parameter**. Location: Phase 1, Section 1. The
  `create_from_local_steps` sample still computes `request.digest()` and passes
  it to `settle_order`, which hands it to `order_precondition` and
  `create_anew`, and `create_anew` passes it on to `create_marking_progress`
  (apply.rs:1262-1332). Suggestion: pick one approach and update the sample.
- 🔵 minor (medium): **`bare_create_decision` relies on an unchecked
  precondition, and 'bare' is tested in two places**. Location: Phase 1,
  Section 1; Phase 2, Section 3. The decision ignores `request.order`. The run
  checks the order with `if let Some(order) = request.order`, and the preview
  with `create.is_bare()`. The `relink_order` doc calls the same case a "plain
  create". Suggestion: `debug_assert!(request.order.is_none())` or a one-line
  doc, define `is_bare()` as `self.planned.order().is_none()`, and unify the
  term.
- 🔵 suggestion (medium): **`previewed_outcome` would read more clearly using
  `?` than a nested `and_then` closure**. Location: Phase 2, Section 3.
  Extract `previewed_decision(...) -> Result<BareCreateDecision, ApplyError>`
  written as straight-line code.
- 🔵 suggestion (medium): **`previewed_outcome` takes all of `SyncPorts` but
  uses only two of them**. Location: Phase 2, Section 3. Pass `ports.author`
  and `ports.files.tracking` explicitly, in line with `validate_pushes` and
  `previewed_adoptions`.

### Test Coverage

**Summary**: The revised plan's test strategy is strong. Phase 2 is test-first,
with red tests kept apart from guard tests. The engine tests mirror the run's
refusal, distrust, deferral and unusable-file tables. Engine-level write
checks now go through the `Spy`, and the CLI parity test normalises the
repository root. The remaining gaps are small:

- The ordered-create guard uses a negative assertion that is too loose.
- The on-disk `pending-push/` snapshot is attached to only some of the CLI
  tests, and its `String`-based shape cannot see a non-UTF-8 marker.
- One exit path that the changelog advertises, a work file that cannot be
  read (exit 70), has no preview test.

**Strengths**:
- Red tests and guard tests are listed separately, and each red test must be
  seen failing first, with the failing run recorded in the PR.
- The engine preview tests mirror the run's own tables, so a mutant in the
  token, detail, class or operation would fail a test.
- Engine-level checks now look for writes and removals under `pending-push/`
  in `fixture.spy`, and `assert_nothing_sent` replaces "no tracker call".
- The distrust test loops over `Tracked` and `Unknown` and adds the
  non-tracker-key cause, and `a_preview_distrusts_a_committed_marker`
  exercises the real VCS tracking adapter.
- The `sync_settled.rs` test runs in both modes and adds apply-side coverage
  of `FollowedThisRun` that does not exist today.
- The CLI parity test builds fresh repositories, normalises the root and
  reads `would fail` as `failed`, with absolute-value tests beside it.
- The whole-corpus 70-to-71 change and the reusable-marker clean case are both
  pinned at CLI level.
- Phase 1 is protected by the existing suite, so a wrong
  `CreateFromLocalRequest::digest()` would fail the existing reuse tests.

**Findings**:
- 🔵 minor (high): **The ordered-create guard's negative assertion is too
  loose to pin "as it does today"**. Location: Phase 2, Section 1. If `gather`
  ran before the `is_bare()` check, or an ordered create reported
  `CorpusIncomplete` or `LocalUnparseable`, the test would still pass.
  Suggestion: assert `NotApplied` exactly, and add an ordered create and a
  relink over an unclosed-fence work file.
- 🔵 minor (high): **Most CLI preview tests never check the `pending-push/`
  snapshot, despite the Testing Strategy**. Location: Phase 2, Section 2;
  Testing Strategy. Only two tests assert `pending_push_tree`. The parity loop,
  the whole-corpus test, the committed-marker test and the conflict test do
  not. Suggestion: snapshot before and after every CLI preview.
- 🔵 minor (medium): **A `String`-valued snapshot silently skips a non-UTF-8
  marker**. Location: Phase 2, Section 2. Copying `meta_tree` keeps only files
  that `read_to_string` can decode. Suggestion: snapshot bytes, or require
  valid-UTF-8 unreadable markers.
- 🔵 minor (medium): **No preview test covers a work file that cannot be read
  (retryable `Io`, exit 70)**. Location: Phase 2, Section 1; Phase 3,
  Section 4. Suggestion: add the removed-file case, asserting `Retryable` and
  `ReadLocal`, mirroring `a_local_create_whose_file_went_missing_is_retryable`.
- 🔵 suggestion (medium): **"Both statements" in the skill test needs scoping
  to their sections**. Location: Phase 3, Section 1. The statements live in
  Step 2 and Step 5. Assert each within its own section.
- 🔵 suggestion (medium): **The precedence test asserts only the exit code**.
  Location: Phase 2, Section 2. Also assert the `0003` failed row and the
  conflict rows, or compare normalised output.

### Correctness

**Summary**: The core logic holds up. Extracting `bare_create_decision` and
`MarkerReading::read` keeps the run's branch structure: the refuse,
reuse-while-incomplete and proceed arms, the reading taken once before the
order branch, and the digest computed from the same borrowed fields. Gating
the preview on `is_bare()` matches the run's routing exactly. The parity claim
has one real gap. In apply mode the identity pass promotes drafts, retires
ids, rewrites references across the corpus and follows moved keys before the
engine replans, while the preview evaluates each bare create against the
corpus as it stood before any of that.

**Strengths**:
- `bare_create_decision` is port-free and built from the existing match, arm
  for arm, so the refusal detail, distrust cause and `CorpusIncomplete`
  blocker come out identical in both modes, and `ItemAttempt::finish` leaves
  them unchanged.
- A bare create's item never has an `external_id`, so the preview can safely
  skip `leftover_attempts`.
- The shared `LocalCreateInputs` keeps the run's failure order, with a
  work-file error before the unsafe-id `MarkerWrite`.
- The exit-code reasoning checks out: `PendingPushRefused` (71) outranks the
  retryable deferral (70) and UNRESOLVED (4), and `CorpusIncomplete` takes its
  class from the blocker.
- Leaving ordered creates and relinks unchanged avoids a dossier-shape change.

**Findings**:
- 🔵 minor (medium): **The preview checks the corpus before identity
  settlement, but the run checks it after**. Location: Key Discoveries /
  Desired End State (Phase 2). There are two cases:
  - (a) A bare create whose body references a draft promoted this run gets a
    different digest on the run, so a `Created` marker that is clean in the
    preview becomes `fingerprint-mismatch` on the run.
  - (b) A `Created` marker that records a moved synced item's old key is
    refused as `already-written` in the preview, while the run links it.

  Suggestion: record the limitation in "What We're NOT Doing" and narrow the
  "exactly" wording.
- 🔵 suggestion (high): **The 70-to-71 exit-code claim is broader than what
  changes**. Location: Phase 3: Changelog / Migration Notes. Under
  `--pull-only`, or with a legacy `Attempted` marker on a draft id, the
  preview still exits 70. Suggestion: scope the sentence.

### Standards

**Summary**: The plan follows the repository's conventions closely: test names
are sentences, the new helpers mirror their neighbours, the preview copies
`validate_pushes`, and verification uses the documented `mise` tasks and
nextest invocations. The gaps are small:

- the "bare create" vocabulary and `is_bare` do not match the existing names;
- the changelog's "see Fixed" Breaking pointer has no precedent;
- the plan does not say what visibility the moved `MarkerReading` gets.

**Strengths**:
- The new test names match the existing sentence style, and every helper they
  rely on exists today.
- `pending_push_tree` copies `meta_tree`'s shape and `#[must_use]`, and builds
  its path from `STATE_DIR`.
- `previewed_outcome` follows the `previewed_adoptions` naming.
- The golden is edited by hand, as the `cli_surface` test instructs.
- The skill content test is replaced in the same phase, and `TestRecoverySkill`
  is left alone.
- The Fixed entry follows the changelog's bold-lead format.

**Findings**:
- 🔵 minor (medium): **'Bare create' is new vocabulary that does not match
  existing names**. Location: Phase 1 (Changes 1 and 2). "Bare" appears
  nowhere in the code. The predicates on `LocalCreate` have no `is_` prefix,
  and the code already says "plain create" (run.rs:1253). Suggestion: use an
  "order"-based name, such as `carries_no_order()`, and the same term in the
  prose.
- 🔵 minor (medium): **The Breaking 'see Fixed' pointer has no precedent in
  CHANGELOG.md**. Location: Phase 3, Section 4. The closest precedent is a
  two-line Fixed entry (line 799) with a full Breaking entry (lines 119-125).
  Suggestion: follow that split.
- 🔵 minor (medium): **The plan does not say what visibility `MarkerReading`
  gets**. Location: Phase 1, Section 1. It is a private enum today
  (apply.rs:635). Suggestion: state that it, `BareCreateDecision` and
  `bare_create_decision` are `pub(crate)`, and use one-item-per-line imports.
- 🔵 suggestion (medium): **`SyncRequest` gets two parameter names within one
  impl**. Location: Phase 1, Section 2. `gather` takes `request` but
  `create_request` takes `sync`. Suggestion: use `request` in both.
- 🔵 suggestion (low): **The new skill sentence mixes backticked detail tokens
  with untagged prose**. Location: Phase 3, Section 1. Suggestion: name the
  create failures by their row tokens (`terminal`, `unparseable`,
  `retryable`).

### Compatibility

**Summary**: The plan changes the observable `work sync --preview` contract
without adding any new row shapes, tokens or exit codes. A preview over a bare
create the run would refuse now exits 71, 75 or 70 where it exited 0, and a
whole-corpus preview held back by the discovery deferral now exits 71 where it
exited 70. The plan follows the changelog convention and keeps the rendering,
precedence and help golden in step. Two compatibility risks remain, both
small: some contract text says more than the change delivers, and the list of
exit-code changes is incomplete.

**Strengths**:
- One function serves the preview and the run, so bare-create rows, error
  lines and exit codes cannot drift.
- No new output vocabulary is introduced, and the documented precedence holds.
- The breaking exit-code change is recorded the way earlier entries record
  theirs, and the move from retryable to never-retry is called out.
- Ordered creates and relinks are left alone, so the recovery skill's
  `--preview` step for an `adopt:<KEY>` order is unchanged.
- The golden and the skill content test are updated in the same PR.
- The new items are crate-private, and `work-adapters` is not pinned by
  `public-api:check`.
- The 0377 coordination is a pre-merge check, and the ordering risk largely
  enforces itself.
- The skill already reads the report whatever the exit code, and already maps
  71 to a refused pending push.

**Findings**:
- 🔵 minor (medium): **Public contract text implies coverage of ordered
  creates, which the preview still does not check**. Location: Phase 3 (CLI
  help, user guide, changelog). The skill is scoped, but the help sentence,
  the guide sentence and the Fixed entry read as covering every create, while
  a `create` or `adopt:<KEY>` order can still be refused locally. Suggestion:
  scope all three to "a create without a `--resolve` order".
- 🔵 suggestion (medium): **The list of preview exit-code changes omits the
  unsafe-id case, and the Breaking pointer is narrower than the change**.
  Location: Phase 3, Section 4; Migration Notes. A bare create whose `id`
  cannot name a marker now previews as `Io` on `MarkerWrite` (exit 70).
  Suggestion: add that case, or state the general rule.

### Documentation

**Summary**: The plan treats documentation as part of the change. It finds all
three copies of the workaround sentence and the content test, updates both
skill statements of preview coverage, and adds entries to the user guide and
the changelog, including a Breaking entry. The main gaps are in the wording:

- The 70-to-71 sentence is wrong for `--pull-only`.
- The Breaking entry is a "see Fixed" pointer.
- The new guide sentence lands between a lead-in and its table.
- The new CLI help sentence claims more than the change delivers.

**Strengths**:
- Every place that states the workaround is listed, and a repository-wide
  search finds no other copy outside `meta/`.
- Both skill coverage statements are updated, and the remaining limit for
  ordered creates is stated.
- The exit-code change reaches script authors through Breaking and Migration
  Notes, using the standard sentence.
- No explanatory comments, and the whole-request choice is explained in prose.
- The plan correctly notes that the golden must be edited by hand.

**Findings**:
- 🔵 minor (high): **The 70-to-71 claim is too broad: `--pull-only` previews
  still exit 70**. Location: Phase 3, Section 4; Migration Notes.
  `unsynced_creates` and the relink partition return no creates under
  `PullOnly`, but `untracked_to_import` (run.rs:2055-2069) still defers
  discovery. Suggestion: scope the sentence.
- 🔵 minor (medium): **The Breaking entry is a 'see Fixed' pointer, unlike the
  rest of the changelog**. Location: Phase 3, Section 4. Suggestion: put the
  exit codes and the scripts sentence in Breaking, and shorten Fixed.
- 🔵 minor (high): **The new guide sentence separates 'The refusal names one
  reason:' from the sentence it follows**. Location: Phase 3, Section 3
  (sync-work-items.mdx:189-191). Suggestion: place it after the reason table
  or the remedy paragraph.
- 🔵 minor (medium): **The new help sentence claims more than the change
  delivers and reads ambiguously**. Location: Phase 3, Section 2. Suggestion:
  delete it, or qualify it to match the skill, and update golden line 211.
- 🔵 suggestion (low): **The ordered-create caveat tells the agent about the
  limit but not what to tell the user**. Location: Phase 3, Section 1.
  Suggestion: add "so a clean preview does not confirm it".
