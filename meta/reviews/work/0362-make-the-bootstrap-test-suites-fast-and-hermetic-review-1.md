---
type: "work-item-review"
id: "0362-make-the-bootstrap-test-suites-fast-and-hermetic-review-1"
title: "Work Item Review: Make the bootstrap test suites fast and hermetic"
date: "2026-10-28T21:34:11+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0362"
work_item_id: "0362"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: ["build-system", "performance", "testing", "ci"]
last_updated: "2026-10-29T12:54:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Make the bootstrap test suites fast and hermetic

**Verdict:** REVISE

The item is well grounded. Every claim rests on a measured figure, the
Requirements name exact files and settings, and the out-of-scope list
defends the epic's no-weakening rule. It falls short in three places:
- how it couples to its siblings: 0368's baseline, 0366's in-pytest builds
  and the epic's same-test-count gate;
- two acceptance criteria that cannot fail on their key claim: the CI
  saving, and that the working-tree launcher is the one dispatched;
- one shifting term, "non-release key", which makes Requirements 2 and 3
  read as contradictory.

The four major findings, all independent of each other, trip the
major-count threshold.

### Cross-Cutting Themes

- **The build-wiring choice is left open and collides with 0366**
  (flagged by: completeness, scope, dependency). The "pick one" between
  `build_launcher()` and the `build:cli:dev` edge sits in Technical Notes
  while Open Questions says "None". 0366 Requirement 2 removes exactly the
  in-pytest cargo builds that one of the options would add.
- **The Dependencies section understates the item's real couplings**
  (flagged by: completeness, dependency). It says "Blocked by: none", but
  the item is likely the epic's first speed change and so must wait for
  0368's baseline. It also overlaps 0366 and changes a quantity the epic's
  closure gate checks. `relates_to` omits 0368.
- **The CI criterion is record-only and reads ambiguously** (flagged by:
  testability, clarity). The headline benefit, CI's critical path, has no
  criterion that can fail. The text of the third criterion doesn't say
  whether 300 s is binding.
- **The new negative test breaks the epic's same-test-count criterion**
  (flagged by: scope, dependency). 0361 allows only the visualiser lib
  exception.
- **The speed ratios are sensitive to 0366 landing first** (flagged by:
  dependency, testability). Lighter debug info would shrink the launcher
  the shim hashes, which shifts both the shim ratio and the suite ratio.

### Findings

#### Major

- 🟡 **Dependency**: Ordering against 0368's `measure:build` baseline is not captured
  **Location**: Dependencies
  Requirement 1 says to ship A1 first and on its own, which makes 0362 very
  likely the epic's first speed change. 0361 and 0368 both require a
  `measure:build` baseline to be recorded before the first child lands, but
  0362 says "Blocked by: none".
- 🟡 **Testability + Clarity**: The CI criterion passes whatever the measured duration is
  **Location**: Acceptance Criteria (third criterion)
  The criterion is met once the duration is "measured and recorded",
  whether that is 1,266 s or 200 s. The Drafting Notes say this is
  deliberate, but the item's priority rests on the CI saving and no
  criterion fails if it doesn't appear. Read alone, "against a target of
  under 300 s" also reads as a pass/fail threshold.
- 🟡 **Testability**: No observable shows the smoke test used the working-tree launcher
  **Location**: Acceptance Criteria (fourth criterion)
  A pass looks the same whether the bootstrap ran the local launcher, a
  cached one or the published one. Nothing the test asserts would catch a
  regression back to a non-local launcher.
- 🟡 **Clarity**: "Non-release key" shifts meaning once a per-run keypair is introduced
  **Location**: Requirements 2–3; Acceptance Criteria (fifth criterion)
  The Context ties "release key" to `keys/accelerator-release.pub`. Under
  that reading the per-run key in Requirement 2 is itself a non-release key,
  so Requirement 2's passing path and Requirement 3's refusal contradict
  each other. The item never says the per-run public key is installed as
  the temporary plugin root's trusted key.

#### Minor

- 🔵 **Completeness + Scope + Dependency**: The build-wiring choice is unresolved and overlaps 0366
  **Location**: Technical Notes; Open Questions
  "Pick one" between `build_launcher()` and `build:cli:dev` is an open
  decision recorded as a note. One option adds the in-pytest cargo build
  that 0366 Requirement 2 removes. The `accelerator-vcs` build prerequisite
  is also unnamed, though the chosen option must keep it.
- 🔵 **Scope + Dependency**: The new negative test conflicts with the epic's same-test-count criterion
  **Location**: Requirements (3)
  0361 requires each suite to run "the same number of tests as before".
  Requirement 3 adds one hooks test, and neither item records the
  exception.
- 🔵 **Scope**: Bundles two independently deliverable changes
  **Location**: Requirements
  A1 and A7 share no code, and the criteria split cleanly between them
  (criteria 1–3 and 6 against 4–5). If A1 ships alone, the item can't show
  it as partly done, and nothing says which PR carries the CI measurement.
- 🔵 **Clarity + Scope**: The Summary misdescribes the item's scope
  **Location**: Summary
  The Summary says the suites that run `bin/accelerator` spend most of their
  time in the shim, which is true only for skill-invocation. It never names
  the suites or mentions the negative case (Requirement 3).
- 🔵 **Clarity**: The Context blames the missing `ACCELERATOR_BIN` for the download, which Technical Notes contradict
  **Location**: Context
  The Context names `ACCELERATOR_BIN` as part of the cause. Technical Notes
  say `bin/accelerator` never reads it.
- 🔵 **Clarity**: "Its `assert_hermetic` precondition" has three possible referents
  **Location**: Technical Notes
  "Its" could mean the stubbed downloader, `make_installation` or
  `run_bootstrap`.
- 🔵 **Clarity**: "The original 0.2 s target" refers to a figure stated nowhere else
  **Location**: Drafting Notes
  Neither the item nor the research states a 0.2 s target, so the 5× bar's
  justification can't be checked.
- 🔵 **Clarity**: `minisign-verify` names both the dependency crate and the shim
  **Location**: Summary
  The Summary calls the verifier "a debug-built `minisign-verify` shim", but
  `minisign-verify` is the dependency that the override targets. The shim
  goes by five other names across the item.
- 🔵 **Testability**: The "no network access" precondition has no defined set-up and is not checked on CI
  **Location**: Acceptance Criteria (fourth criterion)
  Hermeticity is checked once, by hand. A silent fallback to GitHub would
  still pass on any networked runner.
- 🔵 **Testability**: The smoke test's own speed-up is never checked
  **Location**: Acceptance Criteria
  A hermetic smoke test that rebuilds the launcher inside pytest on every
  run would meet every criterion.
- 🔵 **Completeness**: The Dependencies section is sparse compared with the coordination discussed elsewhere
  **Location**: Dependencies
  Sibling coordination is scattered across Technical Notes and References,
  not stated in Dependencies.
- 🔵 **Testability**: Empty stdout proves the launcher did not run only if a successful run prints something
  **Location**: Acceptance Criteria (fifth criterion)
  The item doesn't say what a successful `vcs detect --fail-safe` prints in
  a plain non-repo directory.

#### Suggestions

- 🔵 **Dependency + Testability**: Pin the launcher, and the order against 0366, for the speed ratios
  **Location**: Acceptance Criteria (first and second criteria)
  If 0366's debug-info trial lands first, the launcher shrinks. That
  changes both ratios for reasons unrelated to A1.
- 🔵 **Testability**: Nothing keeps the speed-up in place after merge
  **Location**: Technical Notes
  Resolving the expected `cli/Cargo.toml` conflict with 0366 could silently
  drop the override.
- 🔵 **Completeness**: `relates_to` omits 0368
  **Location**: Frontmatter: `relates_to`
  References lists 0368 as Related, but the structured field does not.
- 🔵 **Dependency**: The build prerequisite for the local `accelerator-vcs` is not named
  **Location**: Technical Notes
  The "pick one" must not drop the edge that builds the binary that
  `ACCELERATOR_VCS_BIN` points to.
- 🔵 **Clarity**: The A1/A7 labels and "dev-launcher override" are used without definition
  **Location**: Requirements; Drafting Notes
  A reader without the research open can't decode them.
- 🔵 **Clarity**: The third out-of-scope bullet has an unclear "the suite" and a compressed justification
  **Location**: Requirements: Out of scope
  It is unclear which suite is meant, and the "either" clause compresses
  the Context's argument.

### Strengths

- ✅ The Context quantifies the problem with measured before/after figures:
  1.15 s vs 0.06 s per verification, 163 s vs 18 s per suite, and 1,266 s
  on CI.
- ✅ The Requirements are concrete enough to start on: the exact
  `Cargo.toml` table and setting, and each part of the stubbed-downloader
  installation.
- ✅ The out-of-scope list justifies each exclusion. It keeps per-call
  re-verification under the epic's no-weakening rule and leaves the
  remaining dev-profile work to 0366.
- ✅ The local speed criteria are same-host ratios with deliberate headroom
  (5× against ~19×, 4× against 9×), so host noise won't flip the result.
- ✅ The negative case swaps an unverifiable "nothing is mocked out" claim
  for three concrete observables. The notes explain why the exit status is
  not asserted under `--fail-safe`.
- ✅ The last criterion turns the assumption that only speed changes into a
  check, by naming two existing trust-chain tests.
- ✅ The item removes a live dependency on GitHub releases and cites
  release-time re-verification (0165) as the reason this loses no
  confidence.
- ✅ The Drafting Notes record the reasoning behind every non-obvious
  choice: the ratio bars, dropping `ACCELERATOR_BIN`, dropping the
  published fetch.

### Recommended Changes

1. **Record the 0368 baseline ordering** (addresses: Ordering against
   0368's baseline; relates_to omits 0368; Dependencies section sparse)
   Add "Blocked by: 0368 (warm no-op `measure:build` baseline recorded
   before A1 merges)" to Dependencies, add `work-item:0368` to `relates_to`,
   and add a "Related" line naming 0366 and 0361's test-count criterion.
2. **Give the CI criterion a bar that can fail** (addresses: CI criterion
   passes whatever the duration is)
   Keep the 300 s target as the aim, and add a floor, for example "at least
   3× faster than the 1,266 s baseline". Alternatively, say what a miss
   triggers, for example a follow-up item with the per-case median. State in
   the criterion itself whether 300 s is binding, and which PR carries the
   measurement if A1 ships alone.
3. **Make the dispatched launcher observable** (addresses: No observable
   for the working-tree launcher; "no network access" precondition)
   Restate the fourth criterion as an assertion the test makes on every run.
   For example: the stub served exactly one request, for the launcher asset;
   no request went anywhere else; and the cached launcher has the same
   digest as the working-tree build.
4. **Define the trusted key relative to the installation** (addresses:
   "Non-release key" shifts meaning)
   In Requirement 2, say that the per-run public key is installed as the
   temporary plugin root's trusted release key. In Requirement 3 and the
   fifth criterion, define "non-release key" as any key other than that
   installed one.
5. **Resolve the build-wiring choice with 0366 in mind** (addresses:
   build-wiring choice unresolved; `accelerator-vcs` prerequisite)
   Prefer keeping the `build:cli:dev` edge, which fits 0366's direction and
   adds no cargo build inside pytest. Name the task that supplies
   `accelerator-vcs`. Either way, Open Questions or Requirement 2 should
   carry the decision.
6. **Reconcile the extra test with 0361** (addresses: negative test
   conflicts with the same-test-count criterion)
   Note the exception here and update 0361's criterion, for example to "no
   fewer tests than before, plus deliberate additions".
7. **Tighten the Summary and Context** (addresses: Summary misdescribes
   scope; Context blames `ACCELERATOR_BIN`; `minisign-verify` naming)
   Name the suites and which change targets which, mention the negative
   case, drop the `ACCELERATOR_BIN` cause, and use one name for the shim.
8. **Add the smoke test's own speed bar and the smaller precision fixes**
   (addresses: smoke-test speed-up unchecked; empty stdout; launcher pinning;
   "Its" referent; 0.2 s target; A1/A7 labels; out-of-scope wording)
   Add a same-host ratio for the smoke test (53–66 s today). State the
   positive case's expected stdout. Pin both shim timings to one launcher.
   Then fix the referent, the stale 0.2 s figure, the undefined labels and
   the third out-of-scope bullet.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is mostly precise. Its claims rest on measured
figures, the Requirements name exact files and settings, and the Acceptance
Criteria name specific tasks, tests and observable signals. The main risk
is the term "non-release key". Once Requirement 2 introduces a per-run
keypair, its meaning shifts, and read literally, Requirements 2 and 3
contradict each other. Smaller problems: the Context gives a cause
involving `ACCELERATOR_BIN` that the Technical Notes contradict, the
Summary refers to "the suites that run `bin/accelerator`" without naming
them and overstates the claim about them, and a few references point at
nothing clear ("Its", "the original 0.2 s target", AC3's "against a
target").

**Strengths**:
- The Context states the problem with concrete before/after numbers, so
  the intended outcome is easy to read.
- The out-of-scope list gives each exclusion and its reason, which
  prevents misreading "fast" as permission to weaken checks.
- The criteria name exact tasks, test functions and observable signals;
  the notes explain why exit status is not asserted under `--fail-safe`.
- The boundary with 0366 is explicit, including how Cargo combines a
  named-package override with a wildcard one.
- The Drafting Notes explain the 4×/5× thresholds and why the
  `ACCELERATOR_BIN` route was dropped.

**Findings**:
- 🟡 major (medium) — Requirements 2–3 and Acceptance Criteria: "Non-release
  key" shifts meaning once a per-run keypair is introduced. Elsewhere,
  "release key" means `keys/accelerator-release.pub`, so the per-run key is
  itself a non-release key, and Requirement 2's passing path contradicts
  Requirement 3's refusal. Say in Requirement 2 whether the per-run public
  key is installed as the plugin root's trusted key, then define the term
  relative to it.
- 🔵 minor (high) — Context: the Context says the test "sets
  `ACCELERATOR_VCS_BIN` but not `ACCELERATOR_BIN` … So the bootstrap
  `curl`s", yet Technical Notes say `bin/accelerator` does not read
  `ACCELERATOR_BIN`. Restate the cause as an empty cache and an unstubbed
  downloader.
- 🔵 minor (high) — Summary: "the suites that run `bin/accelerator`" are
  never listed, and "most of their time" in the shim holds only for
  skill-invocation. Speed and hermeticity target different suites. Name
  them.
- 🔵 minor (high) — Acceptance Criteria: in AC3, "against a target of under
  300 s" can be read as a threshold or as a reference. Only the Drafting
  Notes clarify it; state it in the criterion.
- 🔵 minor (high) — Technical Notes: "Its `assert_hermetic` precondition"
  could refer to the stubbed downloader, `make_installation` or
  `run_bootstrap`. Name the helper.
- 🔵 minor (high) — Drafting Notes: "the original 0.2 s target" appears
  nowhere else in the item or in the research. Explain or remove it.
- 🔵 minor (medium) — Summary: `minisign-verify` names both the dependency
  crate and the shim, and the shim goes by five other names. Use one name
  for the binary and keep `minisign-verify` for the crate.
- 🔵 suggestion (high) — Requirements: the A1/A7 labels and "dev-launcher
  override" are never defined. Gloss them on first use.
- 🔵 suggestion (medium) — Requirements: Out of scope: in the third bullet,
  "the suite" is unclear and the "either" justification is compressed. Name
  the suite and point back to the Context.

### Completeness

**Summary**: A well-populated task. Every expected section is present and
has real content, and `kind: task` is correct. The remaining gaps concern
where information sits. Dependencies says nothing, although the item
discusses coordination elsewhere. Open Questions says "None" while
Technical Notes leave a choice open. `relates_to` is incomplete.

**Strengths**:
- The Context explains the forces behind the work with measured figures,
  and why the published fetch adds no release confidence.
- The Requirements are specific: the exact `Cargo.toml` table, and the
  installation's parts.
- The out-of-scope list says what is excluded and why.
- Every requirement maps to at least one criterion.
- The Assumptions are hedged with a fallback (the `templates` argument).
- The Drafting Notes record the reasoning behind non-obvious choices.
- The frontmatter is complete.

**Findings**:
- 🔵 minor (medium) — Dependencies: it says only "none / none", although
  Technical Notes, References and `relates_to` describe sibling
  coordination. Bring that coordination into the section, or state that
  there are no ordering constraints.
- 🔵 minor (medium) — Open Questions: it says "None", but Technical Notes
  leave the `build_launcher()` vs `build:cli:dev` choice open. Resolve it,
  move it to Open Questions, or state that it is delegated to the
  implementer.
- 🔵 suggestion (medium) — Frontmatter: `relates_to` omits 0368, which
  References lists as Related.

### Dependency

**Summary**: The item handles its external coupling well. It removes the
GitHub dependency, explains why that is safe (0165), and names the textual
`Cargo.toml` conflict with 0366. Its Dependencies section ("none / none")
misses couplings that the epic and its siblings show. The main one is
ordering against 0368's baseline. It also overlaps 0366 on in-pytest and
`build:cli:dev` builds.

**Strengths**:
- It names the shared `cli/Cargo.toml` table with 0366, explains how Cargo
  combines the overrides, and limits the conflict to text.
- It removes the live external dependency on published releases and
  records why that is safe.
- It states the ordering inside the item explicitly: A1 can ship first.
- It records the target-lock constraint between `build_launcher()` and
  `build:cli:dev` as an explicit decision.

**Findings**:
- 🟡 major (high) — Dependencies: ordering against 0368's `measure:build`
  baseline is not captured. A1 ships first, and 0361 and 0368 require the
  baseline before the first child lands. Add "Blocked by: 0368" for A1, and
  add 0368 to `relates_to`.
- 🔵 minor (medium) — Technical Notes: the overlap with 0366 Requirements 1
  and 2 on in-pytest builds and the `build:cli:dev` edge is not captured.
  Prefer the `build:cli:dev` edge, or set an order, and ask for the
  matching note in 0366.
- 🔵 minor (medium) — Requirements: the new negative test conflicts with
  0361's same-test-count criterion. Record the exception and update the
  epic.
- 🔵 suggestion (low) — Technical Notes: the task that builds the local
  `accelerator-vcs` for the hooks suite is not named. The "pick one" must
  keep it.
- 🔵 suggestion (low) — Acceptance Criteria: if 0366's
  `line-tables-only` trial lands first, the launcher shrinks and the 4× bar
  may be out of reach. Note the assumption, or measure A1 before the trial
  is adopted.

### Scope

**Summary**: A small, well-bounded task with a justified out-of-scope list
and a fitting kind. The main scope signal is that it bundles A1, a one-line
profile override, with A7, the smoke-test rework plus a negative case. The
two can be delivered independently, and the item admits as much by telling
A1 to ship alone. The boundary with 0366 over in-pytest builds is loose,
and the Summary omits the negative case.

**Strengths**:
- The out-of-scope list is explicit and justified against the epic and
  against 0366.
- Every requirement serves one theme, the suites that run `bin/accelerator`.
- `task` fits the size: one XS change plus one S change.
- The item orders its own delivery, which lowers the risk of bundling.
- A7 is narrowed to the stubbed-downloader route, with reasoning that stays
  inside the epic's limits.

**Findings**:
- 🔵 minor (high) — Requirements: bundles two independently deliverable
  changes. If A1 ships alone, the item can't show it as partly done, and
  nothing says which PR carries the CI measurement. Split A1 out, or state
  that the item stays in progress and that AC3 belongs to A1's PR.
- 🔵 minor (medium) — Technical Notes: the boundary with 0366 over cargo
  builds inside pytest is not drawn. Choosing `build_launcher()` adds what
  0366 removes. Settle the choice with 0366's direction in mind.
- 🔵 suggestion (medium) — Summary: it omits the negative-case requirement,
  and the epic says the test count won't change. Mention the case in the
  Summary and note the exception on 0361.

### Testability

**Summary**: Mostly testable. The local speed criteria are same-host ratios
with explicit baselines and headroom, the negative case replaces an
unverifiable claim with concrete observables, and the last criterion turns
an assumption into named regression tests. There are two main gaps. The CI
criterion passes whatever duration is measured, and the hermetic criterion
names nothing that shows the working-tree launcher was used or that the
network was avoided. Smaller gaps: the smoke test's own speed-up is never
checked, and some observables could be sharper.

**Strengths**:
- Same-host ratios with stated baselines and deliberate headroom.
- The last criterion names two existing trust-chain tests that must still
  pass, confirming that cache-hit verification is not weakened.
- The negative case has three concrete observables, and the notes steer
  assertions away from the `--fail-safe` exit status.
- Given/When/Then framing with exact invocations makes the procedure
  reproducible.

**Findings**:
- 🟡 major (high) — Acceptance Criteria: the CI criterion passes whatever
  the measured duration is. Add a bar that can fail, for example ≥3× faster
  than 1,266 s, or say what a miss triggers. Name the PR that carries the
  record if A1 ships alone.
- 🟡 major (medium) — Acceptance Criteria: no observable shows the smoke
  test used the working-tree launcher. Assert that the stub served exactly
  one request, for the launcher asset, and that the cached digest matches
  the working-tree build.
- 🔵 minor (medium) — Acceptance Criteria: the "no network access"
  precondition has no defined set-up and is not checked on CI. Restate it
  as a per-run assertion that every request goes to the stub, or name a
  reproducible way to block the network.
- 🔵 minor (medium) — Acceptance Criteria: the smoke test's speed-up is
  never checked. Add a same-host ratio against 53–66 s.
- 🔵 minor (low) — Acceptance Criteria: empty stdout proves that the
  launcher did not run only if a successful run prints something. State the
  positive case's expected stdout, or assert that the refused launcher is
  not cached.
- 🔵 suggestion (medium) — Acceptance Criteria: the shim comparison does
  not fix which launcher is verified. Use the same signed launcher for both,
  timed as the median of five runs.
- 🔵 suggestion (low) — Technical Notes: nothing keeps the speed-up in
  place after merge. Add a generous per-verification time budget, or name
  0368's `measure:build` as the regression check.

## Re-Review (Pass 2) — 2026-10-29T01:57:30+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency**: Ordering against 0368's `measure:build` baseline is not captured — Resolved
- 🟡 **Testability + Clarity**: The CI criterion passes whatever the measured duration is — Resolved
- 🟡 **Testability**: No observable shows the smoke test used the working-tree launcher — Resolved
- 🟡 **Clarity**: "Non-release key" shifts meaning once a per-run keypair is introduced — Resolved
- 🔵 **Completeness + Scope + Dependency**: The build-wiring choice is unresolved and overlaps 0366 — Partially resolved (the choice is made, but the order against 0366 is unset and "adds none" reads ambiguously)
- 🔵 **Scope + Dependency**: The new negative test conflicts with the epic's same-test-count criterion — Resolved (0361 reworded)
- 🔵 **Scope**: Bundles two independently deliverable changes — Still present (scope now rates it major)
- 🔵 **Clarity + Scope**: The Summary misdescribes the item's scope — Partially resolved (the suites are still not listed, and "installation" now names the actor ambiguously)
- 🔵 **Clarity**: The Context blames the missing `ACCELERATOR_BIN` for the download — Resolved
- 🔵 **Clarity**: "Its `assert_hermetic` precondition" has three possible referents — Resolved
- 🔵 **Clarity**: "The original 0.2 s target" refers to a figure stated nowhere else — Resolved
- 🔵 **Clarity**: `minisign-verify` names both the dependency crate and the shim — Resolved
- 🔵 **Testability**: The "no network access" precondition has no defined set-up — Partially resolved (hermeticity now rests on `run_bootstrap`, but in the Given rather than the Then)
- 🔵 **Testability**: The smoke test's own speed-up is never checked — Resolved, but the new bar introduced a major issue (below)
- 🔵 **Completeness**: The Dependencies section is sparse — Resolved
- 🔵 **Testability**: Empty stdout proves the launcher did not run only if a successful run prints something — Resolved, but the fix introduced the "stub" ambiguity (below)
- 🔵 **Dependency + Testability**: Pin the launcher for the speed ratios — Resolved
- 🔵 **Testability**: Nothing keeps the speed-up in place after merge — Not addressed (by decision: such a guard would test implementation, and CI duration exposes a regression)
- 🔵 **Completeness**: `relates_to` omits 0368 — Resolved (through `blocked_by`)
- 🔵 **Dependency**: The build prerequisite for `accelerator-vcs` is not named — Resolved
- 🔵 **Clarity**: The A1/A7 labels and "dev-launcher override" are undefined — Resolved
- 🔵 **Clarity**: The third out-of-scope bullet is unclear — Resolved

### New Issues Introduced

- 🟡 **Testability + Completeness**: The smoke-test 5× bar rests on a baseline dominated by the network and on a target nobody measured — 42 s of the 53–66 s is a GitHub download, so the "before" figure can't be reproduced on the same host, and no time after the change was ever measured.
- 🟡 **Clarity**: "The stub" names both the stubbed downloader and the stub launcher — read the first way, "the stub never runs" in the negative case asserts the opposite of what is meant.
- 🔵 **Completeness + Scope + Clarity**: The narrower committed-key check is recorded only in Drafting Notes — it sits awkwardly beside the Context's "adds no release confidence" claim and the epic's "keep every gate" rule.
- 🔵 **Dependency**: `blocked_by` covers all of 0368 — only its harness and baseline gate this item, and 0368 is medium priority while 0362 is high.
- 🔵 **Dependency**: The order against 0366 is not stated — 0366 may reshape the `build:cli:*:dev` edges, `build_shim()` and the launcher's size, and 0366 has no link back.
- 🔵 **Dependency**: The failing RUSTSEC-2026-0414 check is not recorded as something that could hold up the merge.
- 🔵 **Testability**: The speed criteria do not require the measured run to succeed.
- 🔵 **Testability**: The local before/after measurements have no stated method or record.
- 🔵 **Clarity**: Requirement 3 and the negative-case criterion describe the served launcher differently.
- 🔵 **Clarity**: It is unclear whether the negative case shares the smoke test's installation and download log.
- 🔵 **Clarity**: "Installation" means both the test harness and the bootstrap behaviour under test.
- 🔵 **Clarity**: "This item adds none" drops its noun and seems to contradict the `build_shim()` note.
- 🔵 **Scope**: The fallback in Assumptions could extend A7 into shared test fixtures without limit.
- 🔵 **Dependency**: Reliance on 0165's release-time re-verification is not recorded as a dependency.
- 🔵 **Testability**: The expected stderr for the refused launcher is not pinned.
- 🔵 **Testability**: The CI bar doesn't say which run counts.
- 🔵 **Clarity**: The title's "bootstrap test suites" are never listed.

### Assessment

All four pass-1 majors are resolved. Three majors remain:
- **Smoke-test speed bar** (new, from change 8a). Fix it by dropping the bar: the criterion that the stub's download log holds exactly one request already proves the download is gone.
- **"The stub"** (new, from change 8a). Fix it by giving the stub downloader and the stub launcher separate names.
- **A1/A7 bundling** (now rated major). This is a judgement call, settled either by splitting A7 into its own child of 0361 or by recording why the two stay together.

Most of the new minor findings are precision fixes in the negative-case wording, the success conditions of the speed criteria, and the 0366 and 0368 edges.

## Re-Review (Pass 3) — 2026-10-29T09:19:52+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Testability + Completeness**: The smoke-test 5× bar rests on a baseline dominated by the network — Resolved (bar dropped, with the reason recorded)
- 🟡 **Clarity**: "The stub" names both the stubbed downloader and the stub launcher — Resolved
- 🟡 **Scope**: A1 and A7 are bundled — Resolved by decision (kept together, with the reason recorded; a narrower edge finding remains below)
- 🔵 **Completeness + Scope + Clarity**: The narrower committed-key check is recorded only in Drafting Notes — Resolved (Out of scope and a 0361 exception); the framing in Out of scope is still flagged below
- 🔵 **Dependency**: `blocked_by` covers all of 0368 — Partially resolved (the prose names Requirements 1–2 and 0368 is now high priority; the frontmatter edge still reads as covering the whole item)
- 🔵 **Dependency**: The order against 0366 is not stated — Resolved (0362 blocks 0366, recorded on both sides)
- 🔵 **Dependency**: The failing RUSTSEC-2026-0414 check is not recorded — Resolved (the fix is duplicated into this branch, and `deny:check` passes)
- 🔵 **Testability**: The speed criteria do not require the run to succeed — Resolved
- 🔵 **Testability**: The local before/after measurements have no method or record — Partially resolved (medians are named, but not how many runs or for which criteria)
- 🔵 **Testability**: Hermeticity is a Given precondition — Resolved
- 🔵 **Clarity**: Requirement 3 and the negative-case criterion describe the launcher differently — Resolved
- 🔵 **Clarity**: It is unclear whether the negative case shares the smoke test's installation — Resolved
- 🔵 **Clarity**: "Installation" means both the harness and the bootstrap's behaviour — Resolved
- 🔵 **Clarity**: "This item adds none" — Resolved
- 🔵 **Scope**: The fallback in Assumptions is open-ended — Resolved (replaced by a checked Technical Note)
- 🔵 **Dependency**: Reliance on 0165 is not recorded — Resolved
- 🔵 **Testability**: The expected stderr is not pinned — Resolved
- 🔵 **Testability**: The CI bar doesn't say which run counts — Resolved
- 🔵 **Clarity**: The title's suites are never listed — Resolved

### New Issues Introduced

- 🔵 **Scope + Dependency**: The typed edges are coarser than the couplings they record — `blocks: 0366` makes 0366 wait for A7 when every stated reason belongs to A1, and `blocked_by: 0368` reads as a gate on starting work when it only gates the merge.
- 🔵 **Dependency**: The shim exemption rests on a premise 0366 may change — if 0366 brings the debug shim under a mise task, all three bootstrap suites must switch to that task in place of `build_shim()`.
- 🔵 **Completeness**: The premises behind the CI bar are not listed in Assumptions, and nothing says what happens to A1's merge if the bar is missed.
- 🔵 **Testability**: The smoke-test criterion doesn't require the existing dispatch assertions to survive.
- 🔵 **Testability**: Hermeticity is checked only through stand-in signals — nothing checks that the launcher itself stays off the network.
- 🔵 **Testability**: "The stub launcher never runs" has no positive control showing the record is written when the stub does run.
- 🔵 **Clarity**: Requirement 1 says separate shipping happens only if needed, while the Drafting Notes treat it as settled.
- 🔵 **Clarity**: The Out of scope list frames a deliberate removal as an exclusion, next to its own "no weakening" rationale.
- 🔵 **Clarity**: "The smoke test" refers to today's test in one Technical Note and the rewritten test in the next.
- 🔵 **Clarity**: A1 and A7 are used in Context before they are defined, and Requirement 3 carries no tag.
- 🔵 **Clarity**: Requirement 3 refers to "the fixture's stub launcher" and "the per-run attacker key" without introducing either.
- 🔵 **Clarity + Testability**: The median instruction doesn't say how many runs, or which criteria it covers.
- 🔵 **Completeness + Dependency**: `relates_to` omits 0182 (suggestion).
- 🔵 **Completeness**: Nothing explains why the entrypoint suite has no speed bar (suggestion).
- 🔵 **Testability**: The test that proves release-time re-verification is not named (suggestion).

### Assessment

No critical or major findings remain, so the item is ready for planning. The remaining findings are minor wording and edge-precision fixes. Two are worth doing before planning: the positive control for the stub launcher's run record, and scoping the 0366 block to Requirement 1. Each stops something from looking verified when it isn't.

## Re-Review (Pass 4) — 2026-10-29T10:35:32+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Scope + Dependency**: The typed edges are coarser than the couplings — Partially resolved (0368's gate is now described as merge-only, and 0366 waits for the whole item by decision; the `blocked_by` frontmatter still doesn't say when to remove it)
- 🔵 **Dependency**: The shim exemption rests on a premise 0366 may change — Resolved (mirrored in 0366)
- 🔵 **Completeness**: The CI premises are not listed, and the effect of a miss on merging is undefined — Resolved, but the closure rule introduced the major below
- 🔵 **Testability**: The existing dispatch assertions may not survive — Resolved; a weaker form remains (below)
- 🔵 **Testability**: Hermeticity is checked only through stand-in signals — Resolved (the launcher's `release_base_url()` honours the same `.invalid` URL)
- 🔵 **Testability**: "Never runs" has no positive control — Resolved by decision (cites `test_happy_path_forwards_args_and_exit_code`); testability still prefers a control inside the hooks suite
- 🔵 **Clarity**: Requirement 1's separate shipping is described as conditional — Resolved
- 🔵 **Clarity**: Out of scope framed the removal as an exclusion — Resolved; Context still overstates (below)
- 🔵 **Clarity**: "The smoke test" meant both today's test and the rewritten one — Resolved
- 🔵 **Clarity**: A1/A7 were used before they were defined — Resolved
- 🔵 **Clarity**: Requirement 3's fixture terms had no antecedent — Resolved
- 🔵 **Clarity + Testability**: The median instruction gave no count or scope — Resolved in Technical Notes; the criteria still don't state the statistic (below)
- 🔵 **Completeness + Dependency**: `relates_to` omitted 0182 — Resolved
- 🔵 **Completeness**: Nothing explained the missing entrypoint speed bar — Resolved
- 🔵 **Testability**: The release re-verification test was not named — Resolved

### New Issues Introduced

- 🟡 **Clarity + Scope + Dependency + Testability + Completeness**: If the CI bar is missed, "done" can no longer be decided — the CI criterion is tied to one run that can't be repeated, so "stays open" means 0362 can never close, and 0366, which waits for the whole item, stalls with it.
- 🔵 **Testability + Completeness**: The measurement statistic, the warm-up and the duty to record figures sit in Technical Notes rather than in the first two criteria.
- 🔵 **Testability**: Exit 0 with empty stdout would pass even if the launcher never dispatched.
- 🔵 **Clarity**: "The working-tree debug build" in the byte-identity assertion could mean the live `cli/target/debug` path or the claimed copy that was served.
- 🔵 **Clarity**: "Built like the smoke test's" leaves it unclear which parts of the installation the negative case shares.
- 🔵 **Clarity**: Context's "adds no confidence" overstates what the item loses, given the moved key check.
- 🔵 **Dependency**: The `blocked_by` edge doesn't say when to remove it.
- 🔵 **Scope**: A1 and A7 are still bundled — already decided in review; no change.
- 🔵 **Scope**: The negative case doesn't say what it adds beyond the entrypoint suite's refusal test (suggestion).
- 🔵 **Testability**: State that `assert_hermetic` plus the download log *is* the hermeticity check (suggestion).
- 🔵 **Clarity**: "This change" in the first two criteria should name Requirement 1 (suggestion).
- 🔵 **Clarity**: "Boundary prohibition" and "the host's launcher asset" are undefined (suggestion).

### Assessment

One major remains, raised by all five lenses. It came from how the CI-miss decision was worded, and a closure rule fixes it. Everything else is precision work. The item is acceptable for planning once the CI-miss closure is settled.

### Resolution after Pass 4

Every pass-4 finding was addressed in 0362, and no further pass was run:
- **CI miss:** the CI criterion moves to a follow-up item, and 0362 closes on its other criteria.
- **Smoke test:** it now asserts the `SessionStart` hook JSON as evidence that the launcher dispatched.
- **Measurement:** the medians are stated inside the first two criteria.
- **Wording:** the byte-identity target, the negative case's set-up, the Context claim, the `blocked_by` removal and the boundary-prohibition gloss were all made precise.
- **Positive control:** it stays as decided in pass 3, citing `test_happy_path_forwards_args_and_exit_code`.

The reviewer changed the verdict to APPROVE on 2026-10-29T12:54:26+00:00.
