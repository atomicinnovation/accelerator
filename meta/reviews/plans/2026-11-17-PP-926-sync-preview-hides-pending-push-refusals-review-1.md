---
type: "plan-review"
id: "2026-11-17-0351-sync-preview-hides-pending-push-refusals-review-1"
title: "Plan Review: Sync Preview Hides Pending-Push Refusals Implementation Plan"
date: "2026-11-17T14:29:55+00:00"
author: "Phil Helm"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-17-0351-sync-preview-hides-pending-push-refusals"
reviewer: "Phil Helm"
verdict: "REVISE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "compatibility", "documentation"]
review_number: 1
review_pass: 1
tags: ["sync", "preview", "pending-push"]
last_updated: "2026-11-17T14:29:55+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Plan Review: Sync Preview Hides Pending-Push Refusals Implementation Plan

**Verdict:** REVISE

The design is sound. Every lens agreed that moving the bare-create decision
into one function that both the run and the preview call is the right fix,
and that it closes the preview–run drift by construction. Phase 1 is a true
refactor behind the existing suite, and the CLI needs no change because
rendering and exit-code derivation are already shared. The plan needs
revising for its test specifications rather than its design. Three tests
cannot work as written: the engine byte check cannot see a write, the "no
title" fixture does not fail, and the order guard has no test. The skill
guarantee and the changelog also claim more, or less, than the change
delivers.

### Cross-Cutting Themes

- **Ordered creates are outside the seam, but the new guarantee covers every
  create** (flagged by: architecture, correctness, documentation). The skill
  sentence says "every create carries the pending-push refusal the run would
  meet", but only bare creates are checked. `order_precondition` calls
  `push_precondition` (push_precondition.rs:206) and can refuse an ordered
  create from local state (`AlreadyWritten`, `KeyCarried`,
  `adopt_awaits_no_incomplete_corpus`). The Current State claim that
  `refused_orders` and `previewed_adoptions` already cover ordered creates is
  inaccurate. The workaround being removed also covered ordered `create` rows,
  because they render as `create-from-local`.
- **The changelog understates the exit-code change** (flagged by:
  documentation, standards, compatibility). The entry describes only 0→71.
  It leaves out 70→71 for a whole-corpus preview, and 0→75 or 0→70 for an
  unreadable work file or a blocked reuse. It doesn't use the changelog's
  "Scripts that parse `work sync` exit codes need updating" formula, and it
  doesn't record why Fixed was chosen over Breaking.
- **The CLI parity test compares absolute paths from two temp repositories**
  (flagged by: test-coverage, correctness). Refusal details end
  `(marker <absolute path>)`, so a literal comparison fails even when preview
  and run agree.
- **The work-file failure fixtures don't fail as planned** (flagged by:
  correctness, test-coverage). A missing title reads as empty
  (`a_missing_field_reads_empty`). At CLI level, an undecodable file is an
  unidentified inventory entry and never becomes a create. Use an unclosed
  frontmatter fence (`ParseLocal`) and undecodable bytes (`ReadLocal`) at
  engine level, and an `id`-bearing unclosed fence at CLI level.
- **`CreateDecision` renames the `PushPrecondition` vocabulary** (flagged by:
  code-quality, standards). `Proceed`/`ReuseId` become `Create`/`Reuse`, and
  the type name drops "bare".
- **`read_marker` has two callers, not three, and its free-function form
  clashes with `pending_push::read_marker`** (flagged by: code-quality,
  architecture, correctness).
- **"Bare create" is never named in code** (flagged by: code-quality,
  architecture). It appears only as `create.planned.order().is_some()` in the
  preview and `request.order` in the run. Suggest `LocalCreate::is_bare()`.
- **The decision takes a wider request than it reads** (flagged by:
  code-quality, architecture). The preview has to pass `ports.author`,
  `attempted_at` and paths the decision never touches.

### Tradeoff Analysis

- **A pure decision vs an encapsulated read**: architecture wants
  `bare_create_decision` to take a `&MarkerReading`, read once at the edge
  before branching. That keeps the mapping testable without a filesystem and
  consistent with `settle_order`. The plan's shape hides the read inside the
  decision, which makes it simpler for callers. Recommendation: read at the
  edge. It also fixes the read-ownership split between the bare and ordered
  branches.
- **A narrow decision input vs one request construction**: a narrow view of
  the request makes the read-only intent explicit. Building the full
  `CreateFromLocalRequest` through `LocalCreateInputs` in both paths
  guarantees that `corpus_blocked_by` and `tracker_name` stay in step.
  Recommendation: narrow the input to what the decision reads, and derive it
  from `LocalCreateInputs` so that the single construction point remains.

### Findings

#### Major

- 🟡 **Test Coverage / Correctness**: Engine-level assertions can't detect a
  preview write, and "no tracker call" can't hold
  **Location**: Phase 2, Section 1: Engine-level tests
  The engine harness routes `integration_state` writes through the in-memory
  `Spy`, so an on-disk byte comparison can't catch a preview that writes a
  marker. Whole-corpus preparation legitimately calls the tracker (search,
  `locate`, `fetch::gather`), so "no tracker call" fails for reasons unrelated
  to the behaviour under test.
- 🟡 **Test Coverage**: No test pins that ordered creates and relinks keep
  their current preview
  **Location**: Phase 2, Section 3: The preview branch
  None of the existing preview order tests seeds a marker, so deleting the
  `order().is_some()` guard would make no test fail. The main regression risk
  is a refusal wrongly reported for an item that a `--resolve` order settles.
- 🟡 **Correctness / Test Coverage**: A work file with no frontmatter title
  does not fail the run's create
  **Location**: Phase 2, Section 1: `a_preview_reports_an_unreadable_create_the_run_would`
  `create_request_fields::read` reads a missing title as an empty string, so
  the test can't go green without changing the run. That would break the
  parity the work item exists to create. The work item's acceptance criterion
  carries the same wording.

#### Minor

- 🔵 **Architecture / Correctness / Documentation**: The new `--preview`
  guarantee covers every create, but ordered creates are still not checked
  **Location**: Phase 3, Section 1; Current State Analysis; What We're NOT Doing
  Narrow the sentence to creates without a `--resolve` order, and correct the
  Current State bullet about `refused_orders` and `previewed_adoptions`. The
  documentation lens also asks for the clause to name the incomplete-corpus
  deferral and the work-file read failure, which are not pending-push
  refusals.
- 🔵 **Documentation / Standards / Compatibility**: The changelog omits the
  70→71 change and the other new preview codes
  **Location**: Phase 3, Section 3: Changelog; Migration Notes
  Name 71, 75 and 70 by cause, add the whole-corpus 70→71 shift, close with
  the standard "Scripts that parse `work sync` exit codes need updating", and
  decide between Breaking and Fixed on purpose.
- 🔵 **Test Coverage / Correctness**: The parity comparison across two
  tempdirs will differ on absolute paths
  **Location**: Phase 2, Section 2: `a_preview_and_the_run_report_the_same_create_failure`
  Normalise each repository root to a placeholder before comparing, and say
  whether "rows" means every stdout line or only the item rows.
- 🔵 **Correctness**: At CLI level an undecodable work file never becomes a
  create
  **Location**: Phase 2, Section 2: parity test seedings
  `inventory::identify` classes it as unidentified, so the seeding matches
  before and after. Seed an `id`-bearing file with an unclosed fence.
- 🔵 **Test Coverage**: The whole-corpus 70→71 exit change is not pinned by a
  test
  **Location**: Migration Notes / Phase 2, Section 2
  Add a whole-corpus preview test with an `Attempted` marker. It should assert
  exit 71, the failed row, and the unchanged `#\tdiscovery\tskipped\tpending-push`
  record.
- 🔵 **Test Coverage**: "Each new test fails first" cannot hold for the
  characterisation tests
  **Location**: Phase 2: Success Criteria
  The reusable-marker and no-marker tests pass today. Split them into red
  tests and guard tests.
- 🔵 **Test Coverage**: Two newly visible preview failures have no test
  **Location**: Phase 2, Section 1
  These are the unsafe id (`MarkerWrite`) and the distrust case for
  `FileTracking::Unknown`. Loop the distrust test over `Tracked` and
  `Unknown`, as the apply test does.
- 🔵 **Compatibility**: The plan omits the 0377 stdout row contract
  coordination
  **Location**: What We're NOT Doing
  State the rule that whichever of 0377 and 0351 lands second updates the
  other's expectations, and add a pre-merge check.
- 🔵 **Code Quality**: The digest is passed beside the request it comes from
  and computed in two places
  **Location**: Phase 1, Sections 1–2
  Add `CreateFromLocalRequest::digest()` and drop the parameter and
  `LocalCreateInputs::digest`.
- 🔵 **Code Quality / Architecture**: The preview builds a full write-capable
  request to make a read-only decision
  **Location**: Phase 2, Section 3; Phase 1, Section 2
  Narrow the decision's input, or record the tradeoff.
- 🔵 **Code Quality / Correctness / Architecture**: The free `read_marker`
  takes three request fields as separate arguments, shares a name with
  `pending_push::read_marker`, and has two callers, not three
  **Location**: Phase 1, Section 1
  Consider `MarkerReading::read(request, tracking)`, and name the two real
  call sites (apply.rs:1143, 1168).
- 🔵 **Architecture**: The extracted decision both reads the marker and
  decides
  **Location**: Phase 1, Section 1
  Read at the edge and pass `&MarkerReading`; see Tradeoff Analysis.
- 🔵 **Code Quality / Standards**: `CreateDecision` drifts from the
  `PushPrecondition` vocabulary
  **Location**: Phase 1, Section 1
  Keep `Proceed` and `ReuseId`, and consider naming the type
  `BareCreateDecision`.
- 🔵 **Standards**: `pending_push_tree` has a different shape from the
  `meta_tree` helper beside it
  **Location**: Phase 2, Section 2: test_support.rs
  Match its key and value types and `#[must_use]`, or share a snapshot
  helper. Build the path from `STATE_DIR`.
- 🔵 **Documentation**: The CLI help only drops the caveat and adds nothing
  **Location**: Phase 3, Section 2
  Add a positive clause and update the golden, or reword Desired End State.
- 🔵 **Documentation**: A second statement of what a preview covers is left
  unchanged
  **Location**: Phase 3, Section 1
  SKILL.md lines 760–766 also describe `--preview` coverage.

#### Suggestions

- 🔵 **Code Quality / Architecture**: Name "bare create" as
  `LocalCreate::is_bare()`
  **Location**: Phase 2, Section 3
- 🔵 **Code Quality**: Clarify `LocalCreateInputs::request` and align its
  arguments with `gather`
  **Location**: Phase 1, Section 2
- 🔵 **Test Coverage**: The reusable-marker acceptance criterion has no
  CLI-level check
  **Location**: Phase 2, Section 2
- 🔵 **Test Coverage**: Fixture order matters when combining
  `conflicts_with_dossiers` with a marker
  **Location**: Phase 2, Section 2: `a_previewed_refusal_outranks_a_conflict`
- 🔵 **Compatibility**: Consider whether the exit-code shift belongs under
  Breaking
  **Location**: Phase 3, Section 3
- 🔵 **Documentation**: The user guide still says only a sync reveals the
  refusal
  **Location**: Phase 3
- 🔵 **Documentation**: The manual check says "blocked", which is not the
  skill's term
  **Location**: Phase 3: Manual Verification

### Strengths

- ✅ One decision function shared by the run and the preview, so parity holds
  by construction rather than through a mirrored copy.
- ✅ `LocalCreateInputs` centralises how the request is built, which closes a
  second drift risk and replaces an anonymous `(String, String, String)`
  tuple.
- ✅ Phase 1 is a pure refactor protected by the existing `sync_create.rs`
  suite, and it keeps the run's order of checks and its
  nothing-written-before-the-decision rule.
- ✅ `previewed_outcome` copies the `validate_pushes` pattern and stays out of
  the already-long `execute`.
- ✅ No CLI change is needed: the preview's `Failed` outcome flows through the
  shared rendering and exit-code derivation, and the 70→71 reasoning holds.
- ✅ Bare creates can't reach the run's leftover-marker removal, so skipping
  that step in the preview does not split the modes.
- ✅ The engine tests mirror the run's refusal and deferral tables, and the
  `sync_settled.rs` test covers the hoisted carrier closure in both modes.
- ✅ All three copies of the workaround sentence are found and retired
  together, and the skill test is changed first.
- ✅ The new items are `pub(crate)`, the launcher pins the binary to the
  plugin version, and the `work list` parity test is unaffected.
- ✅ No comments in the proposed code, and names follow existing conventions
  (`previewed_*`, prose-style test names).

### Recommended Changes

1. **Fix the engine-test assertions** (addresses: Engine-level assertions
   can't detect a preview write)
   Assert that `fixture.spy.writes` and `fixture.spy.removals` hold nothing
   under `pending-push/`, and use `assert_nothing_sent` (no write calls)
   instead of "no tracker call". Keep on-disk byte checks for
   `sync_settled.rs` and the CLI tests.
2. **Add order-guard tests** (addresses: No test pins that ordered creates and
   relinks keep their current preview)
   Seed `Attempted` and mismatched `Created` markers under `adopt:<KEY>`,
   `create` and relink orders in preview. Assert that the row is not a
   pending-push refusal, that it matches the run, and that the relink dossier
   shape is unchanged.
3. **Replace the work-file fixtures** (addresses: no frontmatter title, CLI
   undecodable file)
   At engine level, use undecodable bytes (`ReadLocal`) and an unclosed fence
   (`ParseLocal`), asserting class and operation. At CLI level, use an
   `id`-bearing unclosed fence. Correct the work item's acceptance criterion
   ("has no frontmatter title" becomes "whose frontmatter cannot be parsed").
4. **Scope the skill guarantee and correct Current State** (addresses: the new
   `--preview` guarantee covers every create; the second statement left
   unchanged)
   Say "every create without a `--resolve` order" and name the deferral and
   work-file failures. Fix the claim about `refused_orders` and
   `previewed_adoptions`, record the ordered-create gap as residual scope for
   0379, and align SKILL.md lines 760–766.
5. **Rewrite the changelog and Migration Notes** (addresses: the changelog
   omits the 70→71 change; Breaking vs Fixed)
   List 71, 75 and 70 by cause, add the whole-corpus 70→71 shift, use the
   standard closing sentence, and either add a Breaking pointer or record why
   Fixed is the right section.
6. **Harden the CLI tests** (addresses: parity paths, whole-corpus 70→71,
   reusable marker at CLI, fixture order)
   Normalise the repository root, add a whole-corpus `Attempted` test, loop
   the no-marker test over a reusable `Created` marker, and seed `0003` after
   `conflicts_with_dossiers`.
7. **Split the red tests from the guard tests in the success criteria**
   (addresses: "Each new test fails first" cannot hold)
8. **Reshape the Phase 1 seam** (addresses: decision reads and decides, wide
   request, digest duplication, `read_marker` naming and caller count,
   `CreateDecision` vocabulary, bare create unnamed)
   Read the `MarkerReading` at the edge, add `CreateFromLocalRequest::digest()`,
   keep `Proceed` and `ReuseId` on a `BareCreateDecision`, add
   `LocalCreate::is_bare()`, and name the two real `read_marker` callers.
9. **Smaller documentation and coordination edits**: note the 0377 rule and a
   pre-merge check; add a positive `--preview` clause to the CLI help and
   golden, or reword Desired End State; replace "blocked" with the skill's
   terms; mention the docs-site guide; reshape `pending_push_tree` to match
   `meta_tree`; add the unsafe-id and `Unknown` tracking cases.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. It removes the drift between
preview and run by moving the bare-create decision and its input gathering
into one seam that both paths call. It follows the existing
`validate_pushes` pattern for reporting a previewed failure, and leaves the
CLI untouched because rendering and exit-code derivation are already shared.
The remaining concerns are about where the seam is drawn. The extracted
decision both reads the marker and decides, and it covers only the bare path,
so the ordered-create path keeps a decision of its own that only the run can
reach. The new skill guarantee also promises a little more than the seam
delivers.

**Strengths**:
- A single decision function that both the run and the preview call, so
  preview–run parity holds by construction.
- `LocalCreateInputs::gather` / `request` centralise how a
  `CreateFromLocalRequest` is built.
- Phase 1 is a pure refactor protected by the existing `sync_create.rs`
  suite.
- Dependency direction is unchanged, with no new cross-module edges or CLI
  changes.
- The coupling to 0379 is named, and the reason for leaving the relink
  preview alone is good.
- Hoisting the `carrier` closure is backed by checking that `SettledView` is
  built in both modes (identity_settlement.rs:784-790).

**Findings**:
- 🔵 minor (medium): **Extracted decision both reads the marker and decides,
  splitting who reads it**. Location: Phase 1, Section 1.
  `bare_create_decision` calls `read_marker` internally, while the ordered
  branch reads the marker itself and passes it to `settle_order`. The
  decision is therefore not pure, and it can't be unit-tested without a
  filesystem. There are two `read_marker` call sites (apply.rs:1143, 1168),
  not three. Suggestion: take `&MarkerReading`, read once before branching,
  and correct the caller count.
- 🔵 minor (medium): **Shared seam covers only bare creates, but the new
  preview guarantee covers every create**. Location: What We're NOT Doing /
  Phase 3, Section 1. `order_precondition` calls `push_precondition`
  (push_precondition.rs:206). `refused_orders` covers only orders without a
  create (run.rs:1383-1405), and `previewed_adoptions` never checks
  `KeyCarried`/`AlreadyWritten` (run.rs:1315-1341). Suggestion: scope the
  sentence to bare creates and record the gap for 0379, or draw the seam so
  the ordered path can reuse it.
- 🔵 suggestion (medium): **The bare-create test is written inline instead of
  as a `LocalCreate` method**. Location: Phase 2, Section 3. Add
  `LocalCreate::is_bare()` beside `adopts()`/`relinks()`.
- 🔵 suggestion (low): **Preview must build a full applier request to ask a
  read-only question**. Location: Phase 2, Section 3. State the tradeoff, or
  narrow the decision's input to a view that `LocalCreateInputs` produces.

### Code Quality

**Summary**: The plan fixes a real duplication risk with one source of truth
for the bare-create decision. Phase 1 is a pure refactor and replaces an
anonymous tuple with a named type. The remaining problems are small: the
decision depends on a wider request than it reads, the digest is computed in
two places, "bare create" is implied rather than named, and some new names
clash with or stutter against existing domain vocabulary.

**Strengths**:
- `bare_create_decision` is the only place that maps `push_precondition`
  to refuse, defer or proceed.
- Phase 1 has no behaviour change and is protected by the existing tests.
- `LocalCreateInputs` replaces `(String, String, String)` and owns the state
  paths.
- `previewed_outcome` is extracted out of `execute` and copies
  `validate_pushes`.
- No explanatory comments in the proposed snippets.

**Findings**:
- 🔵 minor (high): **The digest is passed beside the request it comes from
  and computed in two places**. Location: Phase 1, Sections 1–2. Add
  `CreateFromLocalRequest::digest()`, and drop the parameter and
  `LocalCreateInputs::digest`.
- 🔵 minor (medium): **The preview builds a full write-capable request just
  to make a read-only decision**. Location: Phase 2, Section 3. The types do
  not enforce "the preview never touches state". Narrow the input, or record
  the tradeoff.
- 🔵 minor (high): **The free `read_marker` takes three request fields as
  separate arguments and shares a name with `pending_push::read_marker`**.
  Location: Phase 1, Section 1. Use `MarkerReading::read(request, tracking)`
  and correct the caller list (two call sites).
- 🔵 minor (medium): **'Bare create' is a domain concept, but it is only
  implied by inline order checks**. Location: Phase 2, Section 3. Add
  `LocalCreate::is_bare()`.
- 🔵 suggestion (medium): **`CreateDecision` renames the variants of
  `PushPrecondition` and repeats its own name**. Location: Phase 1,
  Section 1. Keep `Proceed` and `ReuseId`.
- 🔵 suggestion (medium): **The name `request` is used for two kinds of
  request, and the two methods take different parts of the create**.
  Location: Phase 1, Section 2. Rename the method to `create_request`, and
  have `gather` take `&LocalCreate`.

### Test Coverage

**Summary**: The testing strategy is strong overall. Phase 2 is driven by
tests written first, the engine tests mirror the run's refusal and deferral
tables, and the CLI parity tests guard against drift. The main gaps are an
engine byte-identity assertion that cannot fail (writes go through the
in-memory Spy) and no test for the order guard. There are smaller fixture and
specification issues: parity paths, the "no title" case, and a success
criterion that some characterisation tests cannot meet.

**Strengths**:
- Phase 1 is protected by a suite that already exercises every
  `read_marker` caller.
- The engine preview tests mirror the run's tables, which kills mutants in
  the reason, detail, class and distrust cause.
- The `sync_settled.rs` test runs in both modes and tests the hoisted
  carrier closure.
- The CLI parity test is paired with absolute-value tests.
- The CLI tests use real files, and `pending_push_tree` checks byte identity
  on disk.
- The Python skill test is updated before the skill edit, and the golden is
  updated too.

**Findings**:
- 🟡 major (high): **Engine-level 'marker bytes identical' check cannot
  detect a preview write**. Location: Phase 2, Section 1. Writes go to the
  `Spy`, not the disk, and "no tracker call" fails on legitimate reads.
  Assert that the Spy has no `pending-push/` writes or removals, and use
  `assert_nothing_sent`.
- 🟡 major (high): **No test pins that ordered creates and relinks keep
  their current preview**. Location: Phase 2, Section 3. Add preview tests
  with markers under `adopt:`, `create` and relink orders.
- 🔵 minor (high): **Parity comparison across two tempdirs will differ on
  absolute paths**. Location: Phase 2, Section 2. Normalise the repository
  root, and say which lines are compared.
- 🔵 minor (medium): **The whole-corpus exit change from 70 to 71 is not
  pinned by a test**. Location: Migration Notes / Phase 2, Section 2. Add a
  whole-corpus `Attempted` test.
- 🔵 minor (medium): **'No frontmatter title' fixture does not produce
  LocalUnparseable**. Location: Phase 2, Section 1. Use the apply table's
  two fixtures, and assert class and operation.
- 🔵 minor (high): **'Each new test fails first' cannot hold for the
  characterisation tests**. Location: Phase 2: Success Criteria. Split them
  into red tests and guard tests.
- 🔵 minor (medium): **Two newly visible preview failures have no test**.
  Location: Phase 2, Section 1. These are the unsafe id (`MarkerWrite`) and
  the non-`InvalidData` read; also loop distrust over `Tracked` and
  `Unknown`.
- 🔵 suggestion (medium): **Reusable-marker acceptance criterion has no
  CLI-level check**. Location: Phase 2, Section 2. Loop the no-marker test
  over a reusable `Created` marker.
- 🔵 suggestion (low): **Fixture order matters when combining
  conflicts_with_dossiers with a marker**. Location: Phase 2, Section 2.
  Seed `0003` after the helper returns, and build the apply copy separately.

### Correctness

**Summary**: The core design is sound, and the data flow holds up when
traced. The two paths share `prepare_run`, bare creates never reach the
leftover-marker removal, `carrier_of` reads a fixed corpus plus a
`SettledView` built in both modes, and `ItemAttempt::finish` passes refusals
through unchanged. The weak points are in the test specifications and the
new skill text.

**Strengths**:
- The same function runs in both modes and builds the errors exactly as the
  run does.
- `create.planned.order().is_some()` divides creates exactly as the run's
  dispatch does.
- Bare creates can't reach `leftover_attempts` (run.rs:2413).
- Phase 1 keeps the run's order of checks and its no-write-before-decision
  rule.
- The 70→71 exit reasoning holds under the precedence.

**Findings**:
- 🟡 major (high): **A work file with no frontmatter title does not fail the
  run's create**. Location: Phase 2, Section 1. Use an unclosed fence
  (`ParseLocal`) and undecodable bytes (`ReadLocal`), and fix the work item
  acceptance criterion and the changelog wording.
- 🔵 minor (high): **At CLI level an undecodable work file never becomes a
  create**. Location: Phase 2, Section 2. Seed an `id`-bearing file with an
  unclosed fence.
- 🔵 minor (high): **The parity test compares error lines that contain
  absolute paths from two different temp repositories**. Location: Phase 2,
  Section 2. Replace the root with a placeholder.
- 🔵 minor (medium): **'No tracker call' cannot hold for whole-corpus
  previews**. Location: Phase 2, Section 1. Assert no write calls, or use a
  targeted selection.
- 🔵 minor (high): **The new --preview guarantee covers every create, but
  ordered creates are still not checked**. Location: Phase 3, Section 1;
  Current State Analysis. Narrow the sentence and correct the Current State
  bullet.
- 🔵 suggestion (high): **read_marker has two callers, not three**.
  Location: Phase 1, Section 1.

### Standards

**Summary**: The plan follows the repository's conventions closely: helper
placement, `pub(crate)` visibility, prose test names and no comments. The
findings are small consistency gaps: `CreateDecision`'s variant names, the
shape of `pending_push_tree`, and the changelog's exit-code formula.

**Strengths**:
- New free functions follow the apply.rs helper pattern, and `pub(crate)`
  keeps them off the public API.
- `previewed_outcome` follows `previewed_adoptions` naming, and
  `LocalCreateInputs` sits beside `CreateStatePaths`.
- Test names follow the prose-sentence style.
- No explanatory comments.
- The CLI golden is edited deliberately, and the changelog entry uses the
  bold-lead style.
- Verification commands use the real package names and mise entry points.

**Findings**:
- 🔵 minor (medium): **The changelog entry omits the 70-to-71 exit change and
  departs from the changelog's exit-code formula**. Location: Phase 3,
  Section 3. Add the case, use "Scripts that parse `work sync` exit codes
  need updating", and decide Breaking vs Fixed on purpose.
- 🔵 minor (medium): **CreateDecision's variants drift from the
  PushPrecondition vocabulary they refine**. Location: Phase 1, Section 1.
  Keep `ReuseId`, and consider `BareCreateDecision`.
- 🔵 minor (medium): **pending_push_tree has a different shape from the
  meta_tree snapshot helper beside it**. Location: Phase 2, Section 2. Share
  a helper or match its shape, and build paths from `STATE_DIR`.

### Compatibility

**Summary**: The plan leaves the CLI contract intact. The new rows and
`would fail` lines match a format consumers already parse, and every new Rust
item is crate-private. The visible change is to the preview's exit codes and
rows, but the changelog and Migration Notes understate which codes a preview
can now return, and the plan omits the 0377 coordination.

**Strengths**:
- No change to CLI rendering or exit-code derivation.
- The launcher pins the binary to the plugin version, so the skill and the
  engine ship together.
- The new Rust surface is `pub(crate)`, and no public-API golden is touched.
- The parity tests pin the consumer-facing contract.
- The help text change is paired with its golden.
- The `work list` vs `sync --preview` parity test compares only
  `planned.state()` and stays compatible.

**Findings**:
- 🔵 minor (high): **Changelog and Migration Notes understate the preview's
  new exit codes**. Location: Phase 3, Section 3; Migration Notes. The new
  codes are 75 (unparseable file, or a reuse blocked by content) and 70
  (I/O failure, or a reuse blocked by the environment), plus the
  whole-corpus 70→71.
- 🔵 suggestion (low): **Consider whether the exit-code shift belongs under
  Breaking**. Location: Phase 3, Section 3.
- 🔵 minor (medium): **Plan omits the 0377 stdout row contract
  coordination**. Location: What We're NOT Doing. Add the rule and a
  pre-merge check.

### Documentation

**Summary**: The plan finds all three copies of the workaround sentence and
retires them with the skill's content test and a changelog entry. Gaps
remain. The new guarantee sentence is narrower than what the preview will
report. The changelog leaves out the 70→71 change. The CLI help only loses a
sentence. A second skill paragraph about preview coverage is left unchanged.

**Strengths**:
- All three copies of the sentence are listed, with no fourth found.
- The skill's test is changed before the skill itself.
- The changelog entry follows house style and sits beside its precedent.
- No explanatory comments in the proposed Rust.
- The recovery skill's pending-push section is correctly left alone.

**Findings**:
- 🔵 minor (high): **New --preview guarantee names only the pending-push
  refusal**. Location: Phase 3, Section 1. It omits the deferral and the
  work-file failures, and "every create" includes ordered creates.
- 🔵 minor (medium): **Changelog entry omits the 70→71 change and may belong
  under Breaking**. Location: Phase 3, Section 3. Replace "see that code"
  with "see a non-zero code".
- 🔵 minor (medium): **The CLI help only drops the caveat and adds
  nothing**. Location: Phase 3, Section 2.
- 🔵 minor (medium): **A second statement of what a preview covers is left
  unchanged**. Location: Phase 3, Section 1 (SKILL.md 760–766).
- 🔵 suggestion (low): **The user guide still says only a sync reveals the
  refusal**. Location: Phase 3
  (`docs-site/src/content/docs/guides/sync-work-items.mdx`).
- 🔵 suggestion (medium): **Manual check says 'blocked', which is not the
  skill's term**. Location: Phase 3: Manual Verification.
