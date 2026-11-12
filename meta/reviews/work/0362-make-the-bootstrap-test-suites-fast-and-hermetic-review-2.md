---
type: "work-item-review"
id: "0362-make-the-bootstrap-test-suites-fast-and-hermetic-review-2"
title: "Work Item Review: Make the bootstrap test suites fast and hermetic"
date: "2026-11-03T19:26:49+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0362"
relates_to: ["work-item-review:0362-make-the-bootstrap-test-suites-fast-and-hermetic-review-1"]
work_item_id: "0362"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 2
review_pass: 3
tags: ["build-system", "performance", "testing", "ci"]
last_updated: "2026-11-10T12:23:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Make the bootstrap test suites fast and hermetic

**Verdict:** COMMENT

All five lenses rate the item ready for planning, and none of them raises
a critical or major finding. The edits made since review 1 (marking the
0368 gate as discharged, the CI-miss hand-off, the `SessionStart` dispatch
evidence) hold up. What remains is precision work:
- one frontmatter link is missing again;
- the CI criterion relies on a waiver that it doesn't state;
- some vocabulary is never defined;
- a few sentences in Dependencies and the Summary are loose.

### Cross-Cutting Themes

- **`relates_to` omits 0368** (flagged by: completeness, dependency,
  clarity). Review 1 counted this as fixed through `blocked_by`. That edge
  went when the 0368 gate was discharged, so the frontmatter no longer
  links to 0368 anywhere.
- **The CI criterion and its hand-off read differently in different
  places** (flagged by: testability, clarity, dependency). Criterion 3
  reads as a hard bar, while Technical Notes waive it into a follow-up
  item. The Summary also states the CI saving as fact, although the
  Assumptions say CI was never profiled.
- **The shared set-up in Requirement 3 is unclear** (flagged by: clarity,
  dependency). "Its own installation" and "shares the smoke test's…" pull
  against each other, and nothing says whether Requirement 3 lands with
  Requirement 2.

### Findings

#### Minor

- 🔵 **Testability**: CI speed criterion has a waiver that the criterion itself does not state
  **Location**: Acceptance Criteria
  Criterion 3 reads as a hard bar of under 422 s. Technical Notes turn a
  miss into a hand-off to a follow-up item, so two verifiers could reach
  opposite verdicts on the same run.
- 🔵 **Completeness + Dependency + Clarity**: Frontmatter `relates_to` omits 0368
  **Location**: Frontmatter: `relates_to`
  References and Dependencies both lean on 0368's baseline, but no
  frontmatter relationship field names it.
- 🔵 **Clarity**: The core actors are never told apart
  **Location**: Summary
  The item never glosses bootstrap, launcher, shim, stub launcher or stub
  downloader. Requirements 2–3 switch between them within single
  sentences.
- 🔵 **Clarity**: Requirement 3 says the negative case has its own installation but also "shares" the smoke test's set-up
  **Location**: Requirements
  "Shares" can be read as reusing the same fixture instance. The
  requirement also uses `launcher_source`, `secret="attacker"` and "the
  unverified log" before any of them is introduced.
- 🔵 **Clarity**: Summary asserts the CI cost as fact while Assumptions say CI was not profiled
  **Location**: Summary
  "Removes the largest single cost … on CI" is stronger than the hedged
  Assumptions and the lowered 3× bar support.
- 🔵 **Clarity**: The "Blocked by: none" bullet mentions 0368 without saying why it mattered
  **Location**: Dependencies
  The bullet never says why this item's merge waited for the baseline:
  so that 0361's before/after comparison isn't skewed by this saving.
- 🔵 **Clarity**: The "It" in the 0366 bullet's last sentence is ambiguous
  **Location**: Dependencies
  "It also blocks 0370" follows a sentence whose subject is "Both items",
  so "It" can be read as 0366.
- 🔵 **Testability**: "Stub launcher never runs" can pass vacuously
  **Location**: Acceptance Criteria
  Its positive control sits in the entrypoint suite, which uses a
  different set-up. Review 1 kept this by decision; testability still
  prefers a control on the same plumbing.
- 🔵 **Testability**: The workspace-boundary assertion names an open-ended set
  **Location**: Acceptance Criteria
  "`do not edit files in` and its siblings" doesn't list the siblings, so
  it's unclear how many strings the test must check.
- 🔵 **Dependency**: The effect on 0361's closing warm no-op comparison is not recorded
  **Location**: Dependencies
  Both requirements change what 0361's closing comparison measures, and
  re-measuring at commits before this item still needs GitHub.
- 🔵 **Scope**: A1 and A7 are bundled
  **Location**: Requirements
  The two can be delivered on their own. Review 1 already decided to keep
  them together, and the Drafting Notes record why.

#### Suggestions

- 🔵 **Testability**: The negative case could assert exit 0 and the content of the unverified log
  **Location**: Acceptance Criteria
  Asserting the exit status guards the `--fail-safe` hook contract, and
  checking the log entry ties it to this refusal.
- 🔵 **Dependency**: The CI criterion depends on GitHub-hosted runner performance
  **Location**: Acceptance Criteria
  Name the runner as an external coupling, and say whether a rerun is
  allowed before the hand-off.
- 🔵 **Dependency**: The order between Requirements 2 and 3 is not stated
  **Location**: Requirements
  Say whether they land in one pull request.
- 🔵 **Scope**: The negative refusal case goes beyond the research's A7 row
  **Location**: Requirements
  Note in Context why Requirement 3 adds coverage.
- 🔵 **Clarity**: "Workspace" means two different things, and "warm" and "full run" are undefined
  **Location**: Acceptance Criteria
  It means a warm build cache in criterion 2 and a secondary VCS
  workspace in criterion 4.
- 🔵 **Clarity**: The purpose of the boundary assertion and the stub-launcher aside is unexplained
  **Location**: Acceptance Criteria
  Say what each one rules out.

### Strengths

- ✅ Every timing figure says where it was measured: the research
  workstation or the CI ubuntu runner, and alone or in a full run.
- ✅ The speed bars are same-host median ratios with a fixed run count, a
  warm-up rule and deliberate headroom (5× against ~19×, 4× against 9×).
- ✅ The hermetic criterion rests on things the test can observe:
  `assert_hermetic`, a download log limited to two assets, a
  byte-identical cache, and `SessionStart` JSON that only
  `accelerator-vcs` emits.
- ✅ The negative case shows that both assets were served, so the refusal
  can only come from verification. The notes explain why it doesn't
  assert the exit status under `--fail-safe`.
- ✅ The downstream blocks (0366, 0370) appear on both sides, with the
  reasons for each. The reliance on 0165's re-verification names the test
  that guards it.
- ✅ The out-of-scope list justifies every exclusion, including where the
  moved key check now lives.
- ✅ The CI-miss contingency reads the same in Dependencies and Technical
  Notes, so the item cannot stall indefinitely.

### Recommended Changes

1. **State the CI hand-off inside criterion 3** (addresses: CI waiver not
   stated; Summary asserts the CI cost; runner coupling)
   For example: "either under 422 s, or the measured duration is recorded
   and a follow-up item owning the CI criterion is linked". Hedge the
   Summary's CI claim to "and is expected to on CI".
2. **Add `work-item:0368` to `relates_to`** (addresses: `relates_to` omits
   0368; "Blocked by" bullet)
   In the same bullet, add one clause saying why the baseline had to exist
   before this item merged.
3. **Disambiguate Requirement 3** (addresses: shared set-up; order between
   Requirements 2 and 3)
   Say that it sets up the same components again rather than reusing the
   smoke test's fixture. Name `.accelerator-unverified.log` and
   `test_non_release_key_signature_is_refused` where they first appear,
   and state whether Requirements 2 and 3 land together.
4. **Gloss the actors once, early** (addresses: actors never told apart;
   two meanings of "workspace")
   Bootstrap = `bin/accelerator`, launcher = the fetched Rust binary, shim
   = `accelerator-verify`, stub downloader, stub launcher. Call the second
   sense a "secondary VCS workspace".
5. **Small fixes to Dependencies and the criteria** (addresses: "It"
   referent; open-ended prohibitions; 0361 consumer; exit status and log
   content)
   Replace "It" with "This item". List the boundary prohibition strings or
   check the structure instead. Name 0361's closing comparison as a
   consumer. Optionally assert exit 0 and the content of the log entry.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The item is precise and well cross-referenced. Its remaining
clarity risks are local: the core actors are never told apart,
Requirement 3 is dense and uses fixture vocabulary before introducing it,
and a few sentences have unclear referents or state as fact what the
Assumptions hedge.

**Strengths**:
- A1 and A7 are tied explicitly to the research table and to specific
  Requirements.
- Every timing figure says where it was measured.
- Context, Out of scope and Drafting Notes explain the removal of the
  published-release fetch the same way.
- The Technical Notes explain the choice of assertions in the negative
  case.
- Dependencies and Technical Notes describe the CI-miss contingency the
  same way.

**Findings**:
- 🔵 minor (medium), Summary: the core actors (bootstrap, launcher, shim,
  stub launcher, stub downloader) are never told apart. Gloss each one
  early.
- 🔵 minor (medium), Requirements: Requirement 3 says the negative case
  has "its own installation" and also "shares" the smoke test's set-up,
  and it uses fixture vocabulary before introducing it. Say that it sets
  up the same components again, and name the refusal test and the log
  file.
- 🔵 minor (medium), Summary: it asserts the CI cost as fact while the
  Assumptions say CI was not profiled. Hedge the CI part.
- 🔵 minor (medium), Dependencies: "Blocked by: none" mentions 0368
  without saying why the baseline mattered. Add the reason in one clause.
- 🔵 minor (high), Dependencies: the "It" in "It also blocks 0370" is
  ambiguous. Replace it with "This item".
- 🔵 suggestion (medium), Acceptance Criteria: "workspace" means two
  different things, and "warm" and "full run" are undefined.
- 🔵 suggestion (low), Acceptance Criteria: the purpose of the
  workspace-boundary assertion and of the stub-launcher aside is
  unexplained.

### Completeness

**Summary**: A very complete task-kind item. Every section is present and
has real content, and the frontmatter is valid. The one gap is that the
frontmatter relationship fields don't match References and Dependencies.

**Strengths**:
- The Context gives measured baselines and explains why dropping the
  published fetch loses no confidence.
- The Requirements are specific enough to start on.
- The out-of-scope list gives a reason for each exclusion.
- The six criteria cover speed, hermeticity, refusal and regression.
- Dependencies, Assumptions and Open Questions are all filled in on
  purpose, and the Assumptions say what was not measured.
- The frontmatter is complete, and the item traces back to research rows
  A1 and A7.

**Findings**:
- 🔵 minor (medium), Frontmatter: `relates_to`: 0368 appears in the prose
  but in none of the frontmatter relationship fields. Add
  `work-item:0368`.

### Dependency

**Summary**: Dependencies are mapped unusually well. Both downstream
blocks appear on both sides, and the discharged 0368 gate, the reliance
on 0165, the CI hand-off and the `Cargo.toml` conflict are all spelled
out. What remains is minor: the effect on 0361's closing comparison, the
CI runner as a coupling, metadata drift, and the order between
Requirements 2 and 3.

**Strengths**:
- The blocks on 0366 and 0370 appear in the frontmatter and the body,
  with reasons.
- Requirement 1 ships first and alone, and 0366 waits for the whole item.
- The 0368 baseline is shown as discharged, with the path of the
  measurement record.
- The key check that moves names its successor and the test that guards
  it.
- The CI-miss branch is spelled out.
- The `Cargo.toml` overlap with 0366 is named, along with how Cargo
  resolves it.

**Findings**:
- 🔵 minor (medium), Dependencies: the effect on 0361's closing warm no-op
  comparison is not recorded. Name it as a consumer, and note that
  re-measuring at commits before this item depends on GitHub.
- 🔵 suggestion (medium), Acceptance Criteria: the CI criterion depends on
  the performance of the GitHub-hosted runner. Name it, and say whether a
  rerun is allowed.
- 🔵 suggestion (high), Frontmatter: `relates_to`: 0368 is omitted.
- 🔵 suggestion (low), Requirements: the order between Requirements 2 and
  3 is not stated.

### Scope

**Summary**: The item bundles A1 (a one-line profile override) with A7 (a
hermetic smoke test plus a refusal case). They share a theme and the
boundaries are clear, but the two can be delivered on their own.

**Strengths**:
- The out-of-scope boundaries are explicit and justified.
- The overlap with 0366 is stated precisely.
- The CI criterion has a defined escape hatch.
- The Summary, Requirements and Acceptance Criteria describe the same
  scope.
- "Task" fits the size.

**Findings**:
- 🔵 minor (medium), Requirements: A1 and A7 can be delivered on their
  own but share one item. Review 1 decided to keep them together. Either
  split them into sibling tasks or keep the recorded reason.
- 🔵 suggestion (low), Requirements: the negative refusal case goes beyond
  the research's A7 row. Note why in Context.

### Testability

**Summary**: Very testable. Almost every criterion names its
precondition, action and threshold. The remaining gaps: the CI waiver is
not stated in the criterion, the "never runs" control sits in another
suite, and "and its siblings" is open-ended.

**Strengths**:
- The bars are same-host median ratios with a run count, a warm-up rule
  and headroom.
- The smoke-test criterion turns "hermetic" into facts the test can check.
- The negative case shows that the download happened, so the refusal can
  only come from verification.
- Criteria cover the assumption that only speed changes.
- The Drafting Notes explain why some suites have no speed bar.

**Findings**:
- 🔵 minor (high), Acceptance Criteria: the CI speed criterion has a
  waiver that the criterion doesn't state. Put both outcomes in the
  criterion.
- 🔵 minor (medium), Acceptance Criteria: "stub launcher never runs" can
  pass vacuously without a control in the same test.
- 🔵 minor (medium), Acceptance Criteria: the workspace-boundary assertion
  names an open-ended set. List the strings or check the structure.
- 🔵 suggestion (medium), Acceptance Criteria: the negative case could
  also assert exit 0 and the content of the unverified log.

## Re-Review (Pass 2) — 2026-11-09T22:09:14+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Testability**: CI speed criterion has a waiver that the criterion
  itself does not state — Resolved. Criterion 3 now names both outcomes.
- 🔵 **Completeness + Dependency + Clarity**: Frontmatter `relates_to`
  omits 0368 — Resolved.
- 🔵 **Clarity**: The core actors are never told apart — Resolved. Context
  opens with a glossary.
- 🔵 **Clarity**: Requirement 3 says the negative case has its own
  installation but also "shares" the smoke test's set-up — Resolved.
- 🔵 **Clarity**: Summary asserts the CI cost as fact — Resolved. It is
  now "expected" on CI.
- 🔵 **Clarity**: The "Blocked by: none" bullet mentions 0368 without
  saying why it mattered — Resolved.
- 🔵 **Clarity**: The "It" in the 0366 bullet's last sentence is
  ambiguous — Resolved. 0370 has its own bullet.
- 🔵 **Testability**: "Stub launcher never runs" can pass vacuously —
  Resolved by decision (review 1); not re-raised.
- 🔵 **Testability**: The workspace-boundary assertion names an open-ended
  set — Partially resolved. It now checks one fixed header, but see the
  new finding below.
- 🔵 **Dependency**: The effect on 0361's closing warm no-op comparison is
  not recorded — Resolved.
- 🔵 **Scope**: A1 and A7 are bundled — Still present, by decision.
- 🔵 **Testability** (suggestion): Assert exit 0 and the log content —
  Resolved.
- 🔵 **Dependency** (suggestion): CI criterion depends on GitHub-hosted
  runner performance — Partially resolved. Criterion 3 names the runner
  and rules out reruns; Dependencies does not list it.
- 🔵 **Dependency** (suggestion): Order between Requirements 2 and 3 —
  Resolved.
- 🔵 **Scope** (suggestion): Negative case goes beyond the A7 row —
  Resolved. Scope now counts this as a strength.
- 🔵 **Clarity** (suggestion): Two meanings of "workspace"; "warm" and
  "full run" undefined — Resolved.
- 🔵 **Clarity** (suggestion): Purpose of the boundary assertion and
  stub-launcher aside unexplained — Resolved.

### New Issues Introduced

- 🔵 **Testability** (minor): The boundary-header assertion only bites
  inside a secondary VCS workspace. On CI or a main checkout it passes
  even if `vcs detect` resolved the surrounding repository. Add a
  positive assertion that holds on every host.
- 🔵 **Testability** (minor): Criterion 3 is now met by either outcome,
  so this item never falsifies the CI claim. Require the follow-up item
  to carry the 3× bar and 1,266 s baseline as its own criterion.
- 🔵 **Clarity** (minor): Requirement 3 explains `launcher_source` and
  `secret`, but Requirement 2's smoke test is the one that sets
  `launcher_source`. Say which test sets which option, and that both
  exist today.
- 🔵 **Clarity** (minor): "claimed copy", "warm no-op baseline" and "the
  committed release shim" are used without definition.
- 🔵 **Testability** (suggestion): The negative case does not check that
  the refused launcher is absent from the cache afterwards.
- 🔵 **Clarity** (suggestion): "the epic" and "the main workflow" are
  unnamed; Technical Notes refer to criteria by position in an
  unnumbered list.

### Assessment

Every recommended change from pass 1 landed, and no lens raises a
critical or major finding. The item is ready for planning. The new
findings are precision fixes; the two worth taking before planning are
the boundary assertion, which can pass without testing anything, and
making the CI follow-up carry the bar.

## Re-Review (Pass 3) — 2026-11-09T22:49:54+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Testability**: Boundary-header assertion vacuous outside a
  secondary workspace — Resolved. The criterion now pins the start of the
  git-mode reference, which holds on every host.
- 🔵 **Testability**: CI criterion met by either outcome — Partially
  resolved. The follow-up must now carry the bar as its own criterion,
  but the criterion still cannot fail here (see below).
- 🔵 **Clarity**: Unclear which test sets which `make_installation`
  option — Resolved.
- 🔵 **Clarity**: "claimed copy", "warm no-op baseline" and "committed
  release shim" undefined — Resolved. All three are in the glossary.
- 🔵 **Testability** (suggestion): Negative case does not check the cache
  after a refusal — Resolved.
- 🔵 **Clarity** (suggestion): "the epic", "the main workflow" and
  positional criterion references — Resolved. Criteria are now named.
- 🔵 **Dependency** (minor, pass 2): GitHub-hosted runner not named in
  Dependencies — Resolved.
- 🔵 **Scope**: A1 and A7 bundled — Still present, by decision.

### New Issues Introduced

- 🔵 **Clarity** (minor): The Technical Note on a CI miss says the item
  "can close, which clears its block on 0366", contradicting the Drafting
  Notes, which keep it open until Requirements 2–3 land. Introduced in
  pass 2.
- 🔵 **Clarity** (minor): The refusal case calls `vcs detect
  --fail-safe`, not the hook's full command, without saying whether that
  is deliberate.
- 🔵 **Testability** (minor): The refusal case cannot tell a
  signature mismatch from a broken shim. Add a mismatch-specific
  assertion, or show the same installation accepts a release-signed
  launcher.
- 🔵 **Testability** (minor): The CI criterion still always passes; say
  plainly that this item verifies the CI saving only conditionally, and
  require the follow-up to be created as ready.
- 🔵 **Scope** (minor): Say in Dependencies which requirements 0370
  actually waits on.
- 🔵 **Testability** (suggestions): Name the source of the CI duration
  figure; require the whole entrypoint suite to pass; note why the
  full-run saving has no criterion.
- 🔵 **Clarity** (suggestions): Reword "the debug launcher's size, which
  the speed ratios measure against"; reconcile the smoke test's 53–66 s
  with the research's 18–58 s.
- 🔵 **Dependency** (suggestions): The 0361-consumer and 0165-reliance
  relationships appear only in prose; say whether 0363 or 0367 touch the
  shared test fixtures.

### Assessment

Every pass-2 finding is resolved, apart from the CI criterion, which is
partly resolved by design. The item remains ready for planning. Of the
new findings, the closing-condition contradiction is a real error and
should be fixed; the refusal-cause assertion is the most valuable
testing improvement. The rest are polish, and each pass is now finding
issues in text added by the previous pass.

### Changes After Pass 3 (not re-reviewed)

Applied 2026-11-10T12:22:08+00:00, by decision, without a further review pass:

- The Technical Note on a CI miss no longer says the item can close; it
  closes, and clears its blocks on 0366 and 0370, only once Requirements
  2–3 have landed.
- The refusal case runs the hook's full command, and ends with a control
  in the same test: re-signing the served stub launcher with the release
  key makes it run, so the refusal is shown to come from the attacker key.
- The Summary says this item confirms the CI saving only if it meets the
  bar, and the follow-up must exist at status ready before the
  Requirement 1 pull request merges.
- Dependencies says 0370 waits for all three requirements, since
  Requirements 1 and 2 each change the full-run duration.

The pass-3 suggestions (CI duration source, whole entrypoint suite,
full-run criterion, launcher-size wording, smoke-test timing, prose-only
relationships, shared fixtures) are left as recorded observations.

## Final Verdict — 2026-11-10T12:23:12+00:00

**Verdict:** APPROVE

Approved by the reviewer after the post-pass-3 changes. The work item is
ready for implementation; the remaining pass-3 suggestions stay as
recorded observations.
