---
type: "plan-review"
id: "2026-11-05-0338-linear-update-sends-only-given-fields-review-1"
title: "Plan Review: Linear Update Sends Only the Fields It Is Given Implementation Plan"
date: "2026-11-05T17:01:09+00:00"
author: "Phil Helm"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-05-0338-linear-update-sends-only-given-fields"
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "standards", "safety", "documentation"]
review_number: 1
review_pass: 2
tags: ["linear", "cli", "update"]
last_updated: "2026-11-05T17:26:38+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Plan Review: Linear Update Sends Only the Fields It Is Given Implementation Plan

**Verdict:** REVISE

The plan correctly diagnoses the defect and its core mechanism is sound: an
`Option`-filtered input, `--state` resolved from the catalogue before any
request, and one atomic `issueUpdate`. The tests are exact-input,
test-first and pin the zero-hit refusals. Three major concerns keep it from
approval. A second `IssueUpdateInput` builder (`edit_op`) departs from the
Jira client's single-`update_op` shape and the crate's `_op` naming. The
`--state` stdout change is filed under `Changed` where this CHANGELOG puts
parse-breaking changes under `Breaking`. The rewritten failure path has no
test. A cluster of smaller edge issues around failure semantics and the
skill text completes the picture.

### Cross-Cutting Themes

- **A second parallel `issueUpdate` builder instead of one rich update
  operation** (flagged by: architecture, standards, code-quality) — The
  Jira client has one `update_op(&UpdateFields)` that both the CLI and the
  port delegate to. The plan adds `edit_op` alongside `update_op` and
  `retitle_op`, and its "no `git diff` to `update_op`/`retitle_op`"
  criterion rules out the refactor step. The `contract_offline.rs` pins go
  through `tracker().update/retitle`, so they would still guard the wire
  input if those delegated through `IssueEdit`.
- **The `--state` output change is a breaking change** (flagged by:
  compatibility, documentation, standards) — The CHANGELOG's
  `[Unreleased]` section files stdout-shape changes under `### Breaking`
  with a "Scripts that parse … need updating" line.
- **`--state` failures now report *and exit* differently** (flagged by:
  correctness, compatibility, documentation, code-quality) — Moving from
  `for_surface` to `for_failure` changes exit codes in some edge cases (a
  non-JSON 401 goes from 11 to 20, a 400 with a rate-limit error from 34 to
  35, an oversized response from 16 to 21), not just the stderr text. That
  goes unrecorded, and no test pins it.
- **Step 4 of the skill still says "no write was made" on any non-zero
  exit** (flagged by: safety, correctness) — This is false for transport
  timeouts and 200-with-errors, which can have been applied. Now that one
  request carries several fields, the gap matters more.
- **The "an edit names at least one field" rule lives on the clap args,
  not the domain value** (flagged by: architecture, correctness) —
  `edit_op(&IssueEdit::default())` sends `{}` and reports success.
- **The `edit_op` rustdoc names its caller, not its contract** (flagged
  by: code-quality, documentation) — It should say that unnamed fields are
  left as they are, which is the property that distinguishes it from
  `update_op`.

### Tradeoff Analysis

- **Consolidation vs blast radius**: Architecture, standards and code
  quality want `update_op`/`retitle_op` to build an `IssueEdit` and share
  one operation, as Jira does. The plan deliberately leaves the port's
  operations untouched so that `work sync` cannot change. The port pins
  exercise `tracker().update/retitle` with exact `json!` equality, so a
  refactor stays guarded. Recommendation: consolidate in a refactor step
  after green, with the pins as the safety net, and relax the criterion to
  "port pins pass unedited". If you keep the separation, record why Linear
  deliberately differs from Jira and drop the `_op` suffix.
- **Faithful exit codes vs a single failure shape**: Converging `--state`
  failures onto `for_failure` simplifies `run_update` but changes some exit
  codes. Recommendation: accept the convergence, record it in the
  `Breaking` entry, and pin one `--state` wire failure in a test.

### Findings

#### Major

- 🟡 **Standards / Architecture / Code Quality**: `edit_op` adds a parallel
  `issueUpdate` builder that departs from the Jira single-`update_op`
  pattern and the `_op` convention
  **Location**: Phase 1, Section 3: The client operation (and Success
  Criteria `git diff` check)
  Every `*_op` in `linear-client` is the structured twin of a
  `RemoteTracker` method; `edit_op` has none and duplicates the
  refuse/build/`call_op(UPDATE)` sequence. Jira's port `update`/`retitle`
  delegate to one `update_op(&UpdateFields)`. The port pins go through
  `tracker()`, so they would still pass unedited after consolidation.
- 🟡 **Compatibility / Documentation**: The `--state` stdout change is
  filed under `Changed`, not `Breaking`
  **Location**: Phase 2, Section 5: CHANGELOG; Migration Notes
  The JSON envelope, including the echoed state name, goes away entirely.
  The `[Unreleased]` precedent files such changes under `### Breaking`
  with a "Scripts that parse … need updating" line, and the plan's own
  Migration Notes text never reaches the CHANGELOG.
- 🟡 **Test Coverage**: No flow test covers the rewritten `update` failure
  path
  **Location**: Phase 2, Section 1: Failing flow tests first
  Nothing drives `update` into a wire failure. Returning `SUCCESS` on
  `Err`, or misrouting the `--state` error, would pass every planned test.
  Installing `bad-request-400.json` and asserting exit 34 plus `linear
  update failed` needs no inventory bump.

#### Minor

- 🔵 **Correctness / Compatibility / Documentation**: Exit codes for
  `--state` wire failures change, not only the message
  **Location**: What We're NOT Doing; Phase 2, Section 3
  Moving from `for_surface` to `for_failure` remaps some outcomes: a
  non-JSON 401 goes from 11 to 20, a 400 rate-limit from 34 to 35, an
  oversized response from 16 to 21. An unsafe identifier is now also
  refused locally.
- 🔵 **Safety / Correctness**: Step 4 of the skill still claims "no write
  was made" on any non-zero exit
  **Location**: Phase 2, Section 4: The skill reads one success shape
  Transport timeouts and 200-with-errors may have been applied. Suggest
  `/show-linear-issue` to confirm instead.
- 🔵 **Correctness / Architecture**: An empty `IssueEdit` is accepted and
  sends `{}`
  **Location**: Phase 1, Section 2; Phase 2, Section 2
  The no-ops rule is a separate five-field check on `UpdateArgs` that can
  drift away from `IssueEdit`'s fields. Put `is_empty` on the domain value
  and refuse in `edit_op` (or guard on the edit).
- 🔵 **Code Quality / Documentation**: The `edit_op` rustdoc names its CLI
  caller rather than its contract
  **Location**: Phase 1, Section 3
  It should say that every field not named is left as it is, which is what
  distinguishes it from `update_op`.
- 🔵 **Code Quality**: The state-resolution `map(...).transpose()`
  expression is dense
  **Location**: Phase 2, Section 3
  Extract a `resolve_requested_state(...) -> Result<Option<String>,
  ExitCode>` helper, matching `resolve_body` and `client_or_report`.
- 🔵 **Code Quality**: Newly sent fields fail behind the undiagnosable
  `linear update failed`
  **Location**: Phase 2, Section 3; What We're NOT Doing
  `assigneeId` and `priority` are the fields most likely to be rejected,
  and the failure `detail` is dropped.
- 🔵 **Safety**: The preview doesn't flag that `--description ""` erases
  the body
  **Location**: Phase 2, Section 4 (Step 2 of the skill)
  Emptying a field is now a supported way to erase data, but an empty
  value is easy to miss in the confirmation preview.
- 🔵 **Test Coverage**: The unknown-state refusal test doesn't specify a
  catalogue
  **Location**: Phase 2, Section 1: refusals
  With no catalogue the test exercises `NotCatalogued` rather than
  `NotFound`. Seed explicitly, and optionally pin both branches.
- 🔵 **Test Coverage**: Two client-level checks are looser than the rest
  **Location**: Phase 1, Section 1
  The refused-identifier case should match `Outcome::RequestInvalid`, and
  the Phase 1 `sent_input` should assert `hits == 1`.
- 🔵 **Documentation**: `IssueEdit::state_id` takes a resolved UUID, and
  nothing says so
  **Location**: Phase 1, Section 2
  Prefer a type that can only come from `resolve_state`, or a field doc
  naming it.

#### Suggestions

- 🔵 **Architecture**: The edit concept is split across `edit.rs`,
  `client.rs` and `transition.rs` — pick one home.
- 🔵 **Code Quality**: Use `as_deref().map(Value::from)` rather than
  `clone().map(...)` in `IssueEdit::input`.
- 🔵 **Safety**: `updated` is reported without checking
  `issueUpdate.success`. Either check it or record that you deliberately
  don't.
- 🔵 **Safety**: Phase 3 should create the scratch issue explicitly and
  archive it afterwards.
- 🔵 **Test Coverage**: The client and flow suites duplicate the per-field
  matrix. Consider keeping the full matrix in the client suite only.
- 🔵 **Test Coverage**: Consider a repeatable live case in `contract.rs`
  instead of a one-off manual Phase 3.
- 🔵 **Compatibility**: Confirm in Phase 3 how Linear treats empty
  `assigneeId` and `title`.
- 🔵 **Standards**: `IssueEdit` (owned, in `edit.rs`) and Jira's
  `UpdateFields<'a>` (borrowed, in `mutation.rs`) differ. Align them or
  say why not.
- 🔵 **Standards**: Either adopt `UPDATE_NO_CATALOGUE`/`UPDATE_BAD_STATE`
  (113/114) for `update`, or record why it shares the transition codes.
- 🔵 **Documentation**: A test pins `--quiet` as a no-op while the skill
  still advertises it. Consider dropping it from `argument-hint`.

### Strengths

- ✅ Fixes the root cause: the CLI's partial-edit semantics are separated
  from the port's whole-item `update` semantics, which suit sync.
- ✅ `IssueEdit::input` filters `Option`s, so an absent field can never be
  sent while an empty string the caller typed always is, and `priority`
  goes out as a JSON integer.
- ✅ `--state` is resolved through the catalogue-only `resolve_state`
  before any I/O, and the refusals assert zero hits on `/graphql`.
- ✅ Folding `--state` into one `issueUpdate` turns a partial, silent
  outcome into an all-or-nothing write that is idempotent and safe to
  retry.
- ✅ The port pins in `contract_offline.rs` and `port.rs` stay unedited as
  the regression guard for `work sync`, `work update --push` and
  promotion.
- ✅ Exact `assert_eq!` against `json!` catches extra keys, missing keys,
  null-versus-absent and string-versus-integer, replacing the
  substring-only test that could not have caught the bug.
- ✅ The state-with-title test runs over both catalogue shapes, and reusing
  `issue-update-200.json` avoids an inventory bump.
- ✅ The phases can each merge on their own: Phase 1 adds unused API, and
  Phase 2 switches the caller together with the skill and CHANGELOG.
- ✅ `names_no_field` follows the `SearchArgs::filter_pairs` precedent and
  is a `const fn`. `edit.rs` and its test file mirror the crate's layout.
- ✅ Phase 3 is honest that no test double can prove what Linear accepts.

### Recommended Changes

1. **Consolidate onto one update operation over `IssueEdit`** (addresses:
   `edit_op` parallel builder; edit concept split; `IssueEdit`
   vs `UpdateFields`)
   Add a Phase 1 refactor step after green. Change `update_op` to take
   `&IssueEdit`, and have the port's `update` and `retitle` build an
   `IssueEdit` and delegate to it. Replace the `git diff` criterion with
   "`contract_offline.rs` and `port.rs` pass unedited". Alternatively,
   keep `edit_op`, but record why Linear deliberately differs from Jira
   and drop the `_op` suffix.
2. **Move the `--state` output entry to `### Breaking`** (addresses:
   `Changed` vs `Breaking`; exit-code change; stderr change)
   Say that the JSON and the echoed state name are gone. Say that
   `--state` failures now report and exit like any other `linear update`
   failure, and that an unsafe identifier is refused before sending. End
   with "Scripts that parse `linear update --state` output need
   updating." Bring the Migration Notes into line.
3. **Add a failure-path flow test** (addresses: no failure-path test;
   exit-code change)
   Use `bad-request-400.json` with `--state "In Progress" --title x` over
   a seeded catalogue, and assert exit 34, `linear update failed` and one
   hit.
4. **Put the emptiness rule on `IssueEdit`** (addresses: empty
   `IssueEdit`; no-ops rule on clap args)
   Add `IssueEdit::is_empty`, refuse an empty edit in the operation before
   sending, and test it in the client suite. Decide whether `run_update`
   guards on the edit or keeps `names_no_field`.
5. **Correct Step 4 of the skill** (addresses: "no write was made")
   On a non-zero exit, report the error and suggest
   `/show-linear-issue <IDENTIFIER>` to check whether the change landed.
   Optionally make the Step 2 preview call out a cleared description.
6. **Tighten the tests** (addresses: refusal catalogue; looser client
   checks)
   Seed the catalogue for the `Nowhere` refusal. Match `RequestInvalid`
   for the refused identifier. Assert `hits == 1` in the Phase 1 helper.
7. **Polish the code sketches** (addresses: rustdoc; dense expression;
   `as_deref`)
   Write a contract-focused doc for the operation, extract a
   `resolve_requested_state` helper, and use `as_deref()`.
8. **Phase 3 housekeeping** (addresses: scratch-issue lifecycle; empty
   strings)
   Create the scratch issue with `linear create`, archive it at the end,
   and optionally check how Linear handles `--assignee-id ""`.

## Per-Lens Results

### Architecture

**Summary**: The plan keeps the tracker port's contract stable by giving
the CLI its own operation over a domain value of optional fields, and it
resolves `--state` from the catalogue in the imperative shell before any
I/O. That is a sound way to contain a regression that came from sharing
the port adapter's whole-item update. The main weakness is that Linear now
has three parallel `IssueUpdateInput` builders (plus `transition`'s
document), whereas the Jira sibling crate already shows the cleaner shape:
one rich `update_op(&UpdateFields)` that the port's `update` and `retitle`
both delegate to. The plan's no-diff criterion rules out the refactor step
that would bring Linear into line.

**Strengths**:
- Separating the CLI's edit semantics from the port's whole-item `update`
  fixes the real cause.
- Catalogue-only `resolve_state` before the edit is built keeps a clean
  functional core / imperative shell split.
- `IssueEdit::input()` is a pure function that can be tested with no I/O.
- A single `issueUpdate` gives one success shape and stays idempotent.
- Each phase can merge on its own, and the port pins stay a guard
  throughout.

**Findings**:
- 🔵 minor (high) — *Phase 1 (Key Discoveries; Success Criteria `git
  diff`)* — **A third parallel `IssueUpdateInput` builder diverges from the
  Jira sibling's single-op design.** `jira-client/src/client.rs:639-663`
  delegates the port's `update`/`retitle` through `edit_summary_and_body`
  to one `update_op(&UpdateFields)`. The no-diff criterion rules out the
  TDD refactor step, which `contract_offline.rs` would keep safe. Either
  add an optional refactor step, or state why Linear differs.
- 🔵 minor (medium) — *Phase 1 §3* — **The `_op` suffix and the term
  "edit" blur the established vocabulary.** Every `*_op` is the twin of a
  port method; `edit_op` has none, and "edit" is a synonym for "update".
- 🔵 minor (medium) — *Phase 2 §2* — **The "an edit must name a field"
  rule lives on the clap args, not the domain value.** `edit_op(&IssueEdit::default())`
  sends `{"input": {}}`. Add `IssueEdit::is_empty()` and optionally refuse
  in `edit_op`.
- 🔵 suggestion (medium) — *Phase 1 §2–3* — **The edit concept is split
  across two modules.** Capability modules elsewhere own both their types
  and their `impl LinearClient` block. Pick one home, and consider moving
  `resolve_state` to a state-catalogue module.

### Code Quality

**Summary**: The plan is in good shape for maintainability. It adds a
small, well-named domain value and a sibling client operation instead of
overloading the port's `update_op`, and it simplifies `run_update` by
removing the branch and the `unwrap_or_default()` calls that caused the
bug. The concerns are the near-identical `issueUpdate` input builders the
plan forbids merging, a dense `--state` resolution expression, and a
generic failure message that now covers the fields most likely to be
rejected.

**Strengths**:
- `IssueEdit::input` follows the existing `SearchArgs::filter_pairs`
  idiom.
- `UpdateArgs::names_no_field` gives the no-ops guard a domain name.
- Removing the `transition` branch gives `run_update` a single linear
  path.
- `edit_op` reuses the private `refuse_identifier_op`, `call_op` and
  `UPDATE`.
- `IssueEdit::input` is `pub(crate)` and tested only through the request
  body.
- Scope decisions are stated explicitly.

**Findings**:
- 🔵 minor (medium) — *Phase 1 §3 / Success Criteria* — **Three parallel
  `issueUpdate` input builders where one could serve.** `IssueEdit` can
  express both older inputs exactly. Add a refactor step in which
  `update_op`/`retitle_op` delegate to the new operation, and replace the
  no-diff criterion with "pins pass unedited".
- 🔵 minor (medium) — *Phase 1 §3* — **The `edit_op` doc names its CLI
  caller rather than its domain distinction.** Suggested: "Edits only the
  fields `edit` names; Linear leaves every other field as it is."
- 🔵 minor (medium) — *Phase 2 §3* — **The state-resolution expression is
  dense.** Extract a `resolve_requested_state(client, name) ->
  Result<Option<String>, ExitCode>` helper, like `resolve_body`.
- 🔵 minor (medium) — *Phase 2 §3 / What We're NOT Doing* — **Newly sent
  fields fail behind an undiagnosable generic message.** `LinearFailure`
  has no `Display`, and its `detail` is dropped. Include the detail, or
  file a follow-up.
- 🔵 suggestion (low) — *Phase 1 §2* — **`IssueEdit::input` clones every
  present string unnecessarily.** Use `as_deref().map(Value::from)`.

### Test Coverage

**Summary**: Success-path coverage is strong and test-first. Every
acceptance criterion maps to a flow test that checks the exact input, the
hit count and stdout. A client suite pins the wire shape, and the port
pins guard the unchanged input. The main gap is the failure path: no test
drives `update` into a wire failure, even though the plan rewrites that
branch and says its behaviour changes for `--state`.

**Strengths**:
- Exact equality on `variables.input` catches extra keys, missing keys,
  null and type mistakes.
- The per-flag tests each guard one clause of `names_no_field`.
- The refusal tests assert zero hits, which proves the state is resolved
  before sending.
- The state test runs over both catalogue shapes.
- Port pins are kept unedited, with a `git diff` check.
- The substring-only test that could not have caught the bug is replaced.
- Reusing `issue-update-200.json` avoids an inventory bump.
- The plan is honest that the double cannot prove what Linear accepts.

**Findings**:
- 🟡 major (high) — *Phase 2 §1* — **No flow test covers the rewritten
  `update` failure path.** Add a `bad-request-400.json` test with
  `--state "In Progress" --title x` that asserts exit 34 and `linear
  update failed`. Optionally add a row to `flow_errors.rs`.
- 🔵 minor (medium) — *Phase 2 §1 refusals* — **The unknown-state refusal
  test doesn't say which catalogue it uses.** With no catalogue it takes
  the `NotCatalogued` branch. Seed explicitly, and optionally add a second
  row with no catalogue.
- 🔵 minor (high) — *Phase 1 §1* — **Two client-level checks are looser
  than the rest of the suite.** Match `Outcome::RequestInvalid` for the
  refused identifier, and assert `hits == 1` in `sent_input`.
- 🔵 suggestion (medium) — *Testing Strategy* — **The client and flow
  suites repeat the same per-field matrix.** Keep the full matrix in the
  client suite, and keep in the flow suite only the rows that need the
  binary.
- 🔵 suggestion (low) — *Phase 3* — **The live-API assumption could live
  in the existing `contract.rs` harness.** A repeatable case that calls
  the operation with all five fields and reads the issue back with
  `show_op`.

### Correctness

**Summary**: The core logic holds. Filtering by `Option` means absent
fields are never sent and empty strings always are. Resolving the state
before the edit is sent refuses a bad name before any request, and the
`run_update` sketch type-checks (the `transpose`, the partial moves and
the code mappings are all sound). The remaining concerns are at the edges:
an empty edit is accepted, `--state` exit codes change on some wire
failures, and Step 4's "no write was made" sentence remains.

**Strengths**:
- An absent field can never be sent, and an empty string is always sent.
- State refusals keep 122 and 123 and their stderr lines, proven by
  zero-hit assertions.
- `update_op` and `retitle_op` stay byte-for-byte unchanged.
- The no-ops guard runs before the client is built, so an edit sent from
  the CLI always holds at least one key.
- `issueUpdate` sets values rather than incrementing them, so retrying it
  is safe.
- `assert_eq!` on `Value` ignores key order.

**Findings**:
- 🔵 minor (medium) — *Phase 1 §2–3; Phase 2 §2* — **An empty `IssueEdit`
  is accepted and sends an empty `issueUpdate`.** Give `IssueEdit` an
  `is_empty` method and refuse an empty edit before sending (for example
  as `Outcome::RequestInvalid`), or guard on the edit itself.
- 🔵 minor (high) — *What We're NOT Doing; Phase 2 §3, §5* — **Wire-failure
  exit codes change for `--state` updates, not just the message.** A 400
  rate-limit goes from 34 to 35, and an oversized response from 16 to 21.
  Record this as a deliberate convergence.
- 🔵 suggestion (medium) — *Phase 2 §4* — **Step 4 still says no write was
  made on any non-zero exit.** `Transport` (21) and 200-with-errors (34)
  may have been applied. Suggest `/show-linear-issue` to check.

### Compatibility

**Summary**: Shared contracts stay stable: the port, `linear transition`
and the 122/123 refusal codes are all unchanged. The one intended break
for consumers, `--state` stdout moving from JSON to a text line, is
handled consistently with the skill change but filed under `Changed`
rather than `Breaking`. A smaller, unannounced change moves `--state`
wire failures from the surface-error classifier to the port-failure
classifier, which can change exit codes.

**Strengths**:
- The port's wire contract is protected by pins and a `git diff` check.
- `linear transition`'s JSON and its refusal codes are kept.
- The skill and the binary ship together in the same phase.
- Both catalogue shapes are tested.
- `linear-client` has no cargo-public-api snapshot to update.
- Phase 3 checks real Linear.

**Findings**:
- 🟡 major (high) — *Phase 2 §5; Migration Notes* — **The `--state` stdout
  change is filed under `Changed`, but this project lists such changes
  under `Breaking`.** Move it there, say the JSON (including the echoed
  state name) is gone, and add "Scripts that parse `linear update --state`
  output need updating; use `linear show` to read the resulting state."
- 🔵 minor (medium) — *What We're NOT Doing; Phase 2 §3* — **`--state`
  wire failures may now exit with different codes, not just a different
  message.** A non-JSON 401 goes from 11 to 20, and a 5xx with a
  classified `errors[]` from 34 to 20. Document it, or pin it with a
  test.
- 🔵 suggestion (medium) — *Desired End State; Phase 3* — **Empty strings
  for `assigneeId` (and `title`) are passed straight to Linear without
  confirming how it handles them.** Add a Phase 3 check that `--assignee-id
  ""` exits 34 and leaves the issue unchanged.

### Standards

**Summary**: The plan follows most local conventions: test files mirror
their modules, the `updated\t<KEY>` line matches Jira, and the module doc,
re-export, `const fn` and CHANGELOG style all match. The structural
problem is two operations over the same `issueUpdate` mutation, with
`edit_op` breaking the `_op` convention and ignoring the Jira client's
single-update pattern. The `--state` stdout change also belongs under
`Breaking`.

**Strengths**:
- `tests/edit.rs` mirrors `src/edit.rs`.
- Every success prints through `keywords::text_line`, as Jira does.
- Imports are one per line, the module has a `//!` doc, and the
  `pub mod`/`pub use` pattern is followed.
- `names_no_field` is a `pub const fn`, following `filter_pairs`.
- The CHANGELOG entries keep the bold-lead style.
- Test names read as behaviour sentences, and the scenario is reused.

**Findings**:
- 🟡 major (high) — *Phase 1 §3 (and Key Discoveries)* — **`edit_op`
  breaks the `_op` convention and the Jira client's single-update
  pattern.** Have `update_op` take `&IssueEdit`, and have the port's
  `update`/`retitle` build an `IssueEdit` and delegate to it. The pins in
  `contract_offline.rs` go through `tracker()`, so they still guard the
  wire input.
- 🔵 minor (medium) — *Phase 2 §5* — **The JSON-to-text change in stdout
  belongs under `### Breaking`.**
- 🔵 suggestion (medium) — *Phase 1 §2* — **`IssueEdit` differs in name
  and ownership from Jira's `UpdateFields<'a>`.** Align them, or justify
  the difference.
- 🔵 suggestion (low) — *What We're NOT Doing* — **`update` keeps reporting
  state refusals in the transition flow's codes.** Adopt 113/114 under
  `Breaking`, or record why `update` shares 122/123.

### Safety

**Summary**: Sound from a safety view. The plan fixes a real
silent-data-loss bug with one atomic `issueUpdate` carrying only the named
fields, checks state refusals before sending, and leaves the
sync/promotion input pinned and unchanged. What remains is small and in
the skill text: Step 4's "no write was made" claim, a preview that does
not flag an erased description, and success reported without checking
Linear's `success` flag.

**Strengths**:
- A single mutation turns partial silent outcomes into all-or-nothing
  writes.
- State resolution refuses before sending, proven by zero hits.
- Adding a sibling operation keeps the blast radius to `linear update`.
- `edit_op` applies the identifier refusal that `transition` skipped.
- `call_op` retries only where it is safe, and `issueUpdate` is
  idempotent.
- Phase 3 checks the fail-safe path against real Linear.
- Rollback means reverting one phase.

**Findings**:
- 🔵 minor (medium) — *Phase 2 §4* — **The skill still tells the user "no
  write was made" on any non-zero exit.** Reword it: refusals before
  sending mean no write was made, but on transport or server failures the
  user should check with `/show-linear-issue`.
- 🔵 minor (medium) — *Phase 2 §4* — **The preview doesn't flag that an
  empty `--description` erases the body.** Require the Step 2 preview to
  say "(cleared — the existing body will be erased)".
- 🔵 suggestion (low) — *Phase 1 §3* — **`updated` is reported without
  checking `issueUpdate.success`.** Treat a missing or false `success` as
  a `LinearFailure`, or record that it is deliberately unchecked.
- 🔵 suggestion (low) — *Phase 3* — **The live check doesn't say how the
  scratch issue is created or cleaned up.** Create it with `linear
  create` and archive it afterwards.

### Documentation

**Summary**: The plan covers the required documentation: Step 4 of the
skill, the CHANGELOG `Fixed` and `Changed` entries, and rustdoc that
follows crate conventions. The main gap is upgrade documentation: the
`--state` stdout change is filed under `Changed` with no note for
scripts, against the CHANGELOG's own `Breaking` precedent. Smaller gaps
are the unrecorded stderr change and a rustdoc that names its caller
rather than its contract.

**Strengths**:
- The skill and the binary change in the same phase.
- The plan correctly finds Step 4 as the only reader of the JSON, and
  Steps 1–3 stay accurate.
- The `edit.rs` module doc records a real invariant.
- `edit_op` carries a `# Errors` section.
- The stale `run_update` comment is removed along with its branch.
- The generated reference page is checked through `docs:check` rather
  than edited by hand.
- The `Fixed` entry names each user-visible symptom.

**Findings**:
- 🟡 major (medium) — *Phase 2 §5* — **The `--state` output format change
  is filed as `Changed`, not `Breaking`, with no note for scripts.** Move
  it, or end it with "Scripts that parse its JSON need updating to read
  the `updated` line."
- 🔵 minor (medium) — *Phase 2 §5* — **The stderr and identifier-refusal
  changes on the `--state` path are not in the CHANGELOG.**
- 🔵 minor (medium) — *Phase 1 §3* — **The `edit_op` rustdoc names its
  caller rather than its contract against `update_op`.** Suggested: "Sends
  only the fields `edit` names; every other field of the issue is left as
  it is."
- 🔵 minor (low) — *Phase 1 §2* — **`IssueEdit::state_id` takes a resolved
  UUID, but nothing says so or points to `resolve_state`.** Prefer a type
  that can only come from `resolve_state`, or a field doc. Widen the
  transition-scoped wording in `team_states.rs`.
- 🔵 suggestion (low) — *Phase 2 §1* — **A test now pins `--quiet` as a
  no-op while the skill still advertises the flag.** Drop it from the
  skill's `argument-hint`, or record a follow-up.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-05T17:26:38+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Standards / Architecture / Code Quality**: `edit_op` adds a parallel
  builder and breaks the `_op` convention — Resolved (one
  `update_op(&UpdateFields)`; the port's `update`/`retitle` delegate;
  `retitle_op` removed; borrowed `UpdateFields` matches Jira)
- 🟡 **Compatibility / Documentation**: `--state` stdout change under
  `Changed`, not `Breaking` — Resolved
- 🟡 **Test Coverage**: No flow test for the rewritten failure path —
  Resolved (`bad-request-400.json` test; checked to be red today on its
  stderr assertion, and one hit holds because a 400 is not retried)
- 🔵 **Correctness / Compatibility / Documentation**: `--state` exit codes
  change on wire failures — Resolved (recorded in `Breaking` and Migration
  Notes; the 401 example was checked: 11 before, 20 after)
- 🔵 **Safety / Correctness**: Step 4 "no write was made" — Resolved (I
  checked that 111/122/123 can only come from refusals before any request)
- 🔵 **Correctness / Architecture**: Empty update accepted — Resolved
- 🔵 **Code Quality / Documentation**: The rustdoc named its caller —
  Resolved
- 🔵 **Code Quality**: Dense state resolution — Resolved
  (`resolve_requested_state`)
- 🔵 **Code Quality**: Generic `linear update failed` — Accepted
  (deliberately out of scope)
- 🔵 **Safety**: The preview didn't flag an erased description — Resolved,
  but the new wording over-reaches (see below)
- 🔵 **Test Coverage**: The unknown-state test didn't specify a catalogue —
  Resolved (both resolver branches pinned)
- 🔵 **Test Coverage**: Looser client checks — Resolved
- 🔵 **Documentation**: `state_id` UUID undocumented — Resolved (field doc
  with an intra-doc link)
- 🔵 **Architecture**: Edit concept split across modules — Partially
  resolved (`update.rs` owns the value, document and operation; `transition`
  keeps its own `issueUpdate`, and `resolve_state` stays in
  `transition.rs`)
- 🔵 **Safety**: Phase 3 scratch-issue lifecycle — Resolved
- 🔵 **Compatibility**: Empty `assigneeId`/`title` unconfirmed — Partially
  resolved (Phase 3 records the empty-assignee result)
- 🔵 Suggestions on `issueUpdate.success`, exit codes 113/114, `--quiet`,
  matrix duplication and a live contract case — Accepted (skipped by
  choice)

### New Issues Introduced

- 🔵 **Correctness / Compatibility / Documentation**: Step 2 says any empty
  value clears its field. That is only known for `--description`: Linear
  refuses an empty title, `--state ""` exits 122 before any request, and
  the empty-assignee behaviour is unconfirmed. Limit the sentence to the
  description.
- 🔵 **Documentation**: The Step 2 insertion point ("after the paragraph
  listing the fields") would split "under:" from the blockquote heading it
  introduces. Insert after the blockquote instead.
- 🔵 **Documentation**: The `Breaking` entry's "an identifier the shared
  rule refuses" is internal jargon. Say it in user terms and give the exit
  code (34).
- 🔵 **Test Coverage**: The refused-identifier and empty-update client
  tests both expect `RequestInvalid` with zero hits. If the
  identifier case passes empty fields, dropping the identifier guard would
  go unnoticed. Give each test only the input that trips its own guard.
- 🔵 Suggestions:
  - **Code Quality**: The field list is repeated four times. Make
    `is_empty` private, as `*self == Self::default()`.
  - **Code Quality**: Add a flow-test helper for the eight success rows.
  - **Standards**: One rule has two names (`is_empty` /
    `names_no_field`).
  - **Standards**: Step 4 should name the `E_*` diagnostics alongside the
    exit numbers.
  - **Test Coverage**: Mark the Phase 2 tests that already pass today as
    pins.
  - **Test Coverage**: Assert `E_UPDATE_NO_OPS` in the no-ops test.
  - **Test Coverage**: Optionally pin the non-JSON 401 →
    `Outcome::Unexpected` change.
  - **Documentation**: Point a 122/123 refusal to `/init-linear` in
    Step 4.
  - **Documentation**: Widen the `team_states.rs` doc wording.
  - **Architecture**: Record that `transition` keeps its own
    `issueUpdate`.
  - **Architecture**: Move `refuse_identifier_op` beside
    `check_identifier`.
  - **Architecture**: Consider a `StateId` newtype.
  - **Safety**: Add a Phase 3 check that a rejected field leaves the other
    fields unapplied.

### Assessment

No critical or major findings remain, and the revisions introduced none.
Every lens confirmed that the Phase 1 consolidation keeps the port's wire
input byte-identical and that the change is guarded by exact pins. Both
phases still merge on their own. What remains is minor: mostly the
over-broad Step 2 sentence, which three lenses flagged, plus a misplaced
insertion point, CHANGELOG jargon and one test that could hide a dropped
guard. The plan is acceptable for implementation, and these four are quick
to fix first.
