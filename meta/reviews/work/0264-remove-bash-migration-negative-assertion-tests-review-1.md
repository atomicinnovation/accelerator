---
type: "work-item-review"
id: "0264-remove-bash-migration-negative-assertion-tests-review-1"
title: "Work Item Review: Remove Bash-Migration Negative-Assertion Tests"
date: "2026-09-08T19:35:53+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0264"
work_item_id: "0264"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-08T21:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Remove Bash-Migration Negative-Assertion Tests

**Verdict:** REVISE

This is a tightly-bounded, well-drafted cleanup task: a path-level removal
inventory, explicit keep/remove/out-of-scope boundaries, and dependency posture
recorded in both directions with rationale. Two structural issues hold it back
from approval — the Summary under-states the scope (it also deletes a
production lint module, not just tests), and the preservation guarantee in AC4
has no verification procedure a green suite can satisfy. Both are fixable with
targeted edits; nothing about the removal itself is contested beyond the
ADR-0048 guardrail decision the author has already flagged for reviewer
sign-off.

### Cross-Cutting Themes

- **Removing the active ADR-0048 call-site guardrail** (flagged by: scope,
  clarity, completeness) — the work bundles genuinely dead absence-tests with
  the removal of `call_site_migration.py`, an *active* anti-regression guard.
  Scope flags the coupling of a contestable policy decision to an
  uncontroversial cleanup; clarity flags that "config cluster" and "ADR-0048"
  appear undefined and unlinked; completeness flags that the decision is buried
  in Drafting Notes with no Open Questions surfacing it for sign-off.
- **Summary/Requirements scope mismatch** (flagged by: clarity, scope) — the
  Summary and title frame the work as removing "tests", but Requirements and
  Technical Notes also delete a lint module and its wiring.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Summary scope ('tests') narrower than Requirements (also a
  lint module)
  **Location**: Summary
  The Summary and title frame the work exclusively as removing
  'negative-assertion tests', but Requirements and Technical Notes also remove
  the production lint module `tasks/lint/call_site_migration.py` and its task
  wiring — a guard, not a test. A reader stopping at the Summary would not
  expect a lint module and its wiring to be deleted.

- 🟡 **Testability**: "Every positive bash-parity/golden test still exists" is
  not conclusively verifiable
  **Location**: Acceptance Criteria
  AC4 requires that every positive `bash-parity`/golden test "still exist and
  pass", but a passing suite cannot detect an accidentally deleted positive
  test — a removed test simply stops running without failing. The positive test
  set is only partially and wildcard-enumerated in Technical Notes.

#### Minor

- 🔵 **Dependency**: Ordering dependency on sibling retirement items asserted
  collectively, not itemised
  **Location**: Context
  Context states these tests 'landed alongside the retirement work in 0174,
  0211, 0212, 0245, and 0269', but Dependencies records 'Blocked by: none' on a
  collective 'key parts complete' assumption without tying the individual
  completion status of each sibling — particularly the highest-numbered 0269 —
  to the removal.

- 🔵 **Clarity**: 'config-cluster' / 'bash config cluster' used without
  definition
  **Location**: Requirements
  'call-site config-cluster guard' (Requirements) and 'the bash config cluster'
  (Drafting Notes) name a concept never defined or linked. A reader who did not
  live through the migration cannot tell what a 'config cluster' is or why the
  guard protected against regressing to it.

- 🔵 **Testability**: AC1 bundles multiple removals behind a broad "suite
  passes" outcome
  **Location**: Acceptance Criteria
  AC1 collapses every named removal into a single criterion whose observable
  outcome is "the named files no longer exist and `mise run check` plus the test
  suite pass". If the suite fails, the criterion does not localise which removal
  broke, and the circular Given/Then adds no independent verification.

#### Suggestions

- 🔵 **Scope**: Removing an active ADR-0048 guardrail is bundled with
  dead-scaffolding removal
  **Location**: Requirements
  The four Rust tests and the call-site test assert absence of unreachable
  behaviour (dead scaffolding), whereas `call_site_migration.py` is an *active*
  ADR-0048 guardrail. A reviewer who wants to keep the guard cannot approve the
  safe dead-test removals without also signing off on retiring a live guard.

- 🔵 **Completeness**: ADR-0048 removal decision buried in Drafting Notes with
  no Open Questions section
  **Location**: Drafting Notes
  Removing the ADR-0048 guardrail is 'per explicit decision' and 'a reviewer who
  still wants that guard should say so before this lands', but there is no Open
  Questions section surfacing this as a decision awaiting sign-off where a
  reader scanning for outstanding questions would look.

- 🔵 **Clarity**: 'key parts' of the migration is an unqualified vague term
  **Location**: Assumptions
  The load-bearing claim — that asserted-absent behaviour is no longer
  reachable — hinges on 'the key parts of the shell-to-Rust migration are
  complete', without stating which parts are 'key' or whether non-key parts
  remain incomplete.

- 🔵 **Clarity**: ADR-0048 referenced without a link
  **Location**: Drafting Notes
  The Drafting Notes cite 'ADR-0048' as the source of the guardrail being
  removed, but provide no link or title, so a reader must go hunting to
  understand the decision context.

### Strengths

- ✅ Requirements and Technical Notes give a concrete, path-level removal
  inventory — specific files, a named single test, and exact wiring line
  references — so an implementer could begin without follow-up questions.
- ✅ The four-item count in Requirements reconciles precisely with the
  three-files-plus-one-test breakdown in Technical Notes, and pronouns/referents
  resolve unambiguously throughout.
- ✅ Explicit out-of-scope list (SURVIVING_SHELL_SOURCES invariant, bash-3.2
  floor lint, pup rules, positive parity/golden tests, bare-invocation lint,
  bash-parity feature) makes the boundary unambiguous and guards against
  over-removal.
- ✅ Dependencies records both directions (Blocked by / Blocks) with reasoning,
  and Assumptions names the real upstream prerequisite and marks it confirmed.
- ✅ The retained `bash-parity` cargo feature coupling is called out explicitly,
  so the removal does not silently break dependent positive tests.
- ✅ Acceptance Criteria pair removals with explicit preservation checks, and
  the 'task' kind fits: a bounded deletion owned end-to-end by one team.

### Recommended Changes

1. **Broaden the Summary to cover the lint-module removal** (addresses: Summary
   scope narrower than Requirements; Summary/Requirements scope mismatch theme)
   State that the work removes both the leftover absence tests and the
   associated migration-only lint guard (`call_site_migration.py`) with its
   wiring, so every section describes the same scope.

2. **Make AC4's preservation guarantee verifiable** (addresses: "every positive
   test still exists" not conclusively verifiable) Provide an enumerated or
   count-based reference list of positive tests that must survive, and phrase
   the criterion so a verifier compares the post-change test inventory against
   that list rather than relying on a green suite alone.

3. **Decouple or gate the ADR-0048 guardrail removal** (addresses: active
   guardrail bundled with dead-scaffolding removal; decision buried in Drafting
   Notes) Consider splitting the `call_site_migration.py` removal into its own
   task, or add a distinct acceptance criterion gated on explicit reviewer
   sign-off, and surface the decision in an Open Questions section so the safe
   dead-test deletions can proceed independently.

4. **Split AC1 into per-path non-existence checks** (addresses: AC1 bundles
   multiple removals) List each removed path as its own tick-box, separate from
   the suite-green gate, so a failure localises to the specific removal.

5. **Sharpen the reachability assumption and define migration terms**
   (addresses: 'key parts' vague; 'config cluster' undefined; ADR-0048
   unlinked) Replace 'key parts' with the concrete completion the assumption
   depends on (e.g. the shell surface reduced to the two SURVIVING_SHELL_SOURCES
   files), add a one-line gloss for 'config cluster', and link ADR-0048.

6. **Tie the ordering dependency to the specific sibling items** (addresses:
   sibling retirement completion asserted collectively) In Dependencies, note
   the completion state of the items whose tests are being removed — in
   particular confirm 0269 has landed — rather than relying on a blanket
   assumption.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely precise: referents resolve cleanly,
section counts reconcile (three whole Rust files plus one named test equals the
stated four), and the removal inventory in Technical Notes matches the
Requirements. The main clarity concern is a scope mismatch between the Summary
(which frames the work as removing 'tests') and the Requirements/Technical Notes
(which also remove a production lint module and its task wiring). A few undefined
domain terms ('config cluster', ADR-0048) and one vague qualifier ('key parts')
are minor.

**Strengths**:
- Pronouns and referents resolve unambiguously throughout — 'they', 'it', and
  'them' consistently point back to the negative-assertion tests without
  competing referents.
- The removal set is enumerated concretely with exact file paths and line
  numbers, and the four-item count in Requirements reconciles precisely with the
  three-files-plus-one-test breakdown in Technical Notes.
- Out-of-scope items (SURVIVING_SHELL_SOURCES invariant, bash-3.2 floor,
  positive parity/golden tests, bare-invocation lint) are named explicitly,
  removing ambiguity about what is preserved.

**Findings**:
- 🟡 **major** (confidence: medium) — _Summary_: Summary scope ('tests')
  narrower than Requirements (also a lint module). The Summary and title frame
  the work exclusively as removing 'negative-assertion tests', but the
  Requirements and Technical Notes also remove the production lint module
  `tasks/lint/call_site_migration.py` and its task wiring in `tasks/__init__.py`
  and `tasks/lint/__init__.py` — a guard, not a test. Broaden the Summary to
  state that the work removes both the leftover absence tests and the associated
  migration-only lint guard with its wiring.
- 🔵 **minor** (confidence: medium) — _Requirements_: 'config-cluster' / 'bash
  config cluster' used without definition. The phrase names a concept never
  defined or linked within the work item; a reader who did not live through the
  migration cannot tell what a 'config cluster' is. Add a one-line gloss or a
  link (e.g. to ADR-0048).
- 🔵 **suggestion** (confidence: medium) — _Assumptions_: 'key parts' of the
  migration is an unqualified vague term. The load-bearing reachability claim
  hinges on 'the key parts ... are complete' without stating which parts are
  key. State concretely what completion the assumption depends on.
- 🔵 **suggestion** (confidence: low) — _Drafting Notes_: ADR-0048 referenced
  without a link. Add the ADR title or a path/link alongside 'ADR-0048'.

### Completeness

**Summary**: This task work item is structurally and informationally complete:
it carries all expected sections with substantive, concrete content, and its
frontmatter is fully populated with a recognised kind, status, and priority. For
a cleanup task, the definition of work is unusually precise — a file-by-file
removal inventory with explicit keep/remove/out-of-scope boundaries. No critical
or major completeness gaps were found.

**Strengths**:
- Requirements and Technical Notes give a concrete, path-level removal inventory
  so an implementer could begin without follow-up questions.
- Acceptance Criteria contains five specific criteria that each map to a distinct
  removal or preservation outcome.
- Context clearly explains the motivation rather than restating the summary.
- Frontmatter is fully populated and valid, plus parent linkage to epic 0136.
- Scope boundaries are made explicit in both Requirements and Drafting Notes.

**Findings**:
- 🔵 **suggestion** (confidence: low) — _Drafting Notes_: ADR-0048 removal
  decision buried in Drafting Notes with no Open Questions section. The decision
  that gates part of the removal — 'a reviewer who still wants that guard should
  say so before this lands' — is not surfaced where a reader scanning for
  outstanding questions would look. Optionally lift this into an Open Questions
  section.

### Dependency

**Summary**: The work item captures its dependency posture well: it explicitly
states 'Blocked by: none' with a rationale, 'Blocks: none known', and backs the
not-gated claim with an Assumptions entry. The couplings this cleanup implies are
largely internal and named explicitly. The only interpretive gap is that the
removal is a cleanup after five named sibling retirement items whose individual
completion status is asserted collectively rather than itemised.

**Strengths**:
- Dependencies section records both directions with reasoning rather than
  leaving them blank.
- Assumptions names the real upstream prerequisite and marks it confirmed.
- The retained bash-parity cargo feature coupling is explicitly called out.

**Findings**:
- 🔵 **minor** (confidence: medium) — _Context_: Ordering dependency on sibling
  retirement items asserted collectively, not itemised. Dependencies records
  'Blocked by: none' on a collective 'key parts complete' assumption, but the
  individual completion status of each of the five related items — particularly
  0269 — is not tied to the removal. Note the completion state of the specific
  items whose tests are being removed.

### Scope

**Summary**: This is a well-scoped, atomic 'task' that removes migration-era
absence-assertion tests as a single coherent cleanup unit, with explicit in-scope
and out-of-scope boundaries. The Summary, Requirements, Acceptance Criteria, and
Technical Notes all describe the same removal set, and the task is appropriately
sized. The one scope nuance is that the removal bundles dead absence-assertion
tests with an active ADR-0048 anti-regression guardrail.

**Strengths**:
- Requirements, Acceptance Criteria, and Technical Notes describe an identical,
  concrete removal inventory — no drift between sections.
- Explicit out-of-scope list makes the boundary unambiguous.
- The 'task' kind is a good fit: a bounded deletion owned end-to-end by a single
  team with no cross-service orchestration.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Requirements_: Removing an active
  ADR-0048 guardrail is bundled with dead-scaffolding removal. The four Rust
  tests assert absence of unreachable behaviour, whereas
  `tasks/lint/call_site_migration.py` is an active ADR-0048 guardrail. Removing a
  live guard is a policy decision that could be deliberated independently.
  Consider its own task or a separate acceptance criterion gated on reviewer
  sign-off.

### Testability

**Summary**: For a cleanup task, the acceptance criteria are unusually concrete:
named files, a named single test, and a named module are checked for
non-existence, and reference-removal is checked at specific call sites — all of
which admit a definitive pass/fail via file listing, grep, and a passing suite.
The main weakness is the preservation guarantee (AC4), whose 'every positive test
still exists' claim cannot be conclusively verified by a green suite alone.

**Strengths**:
- Removal targets are enumerated with exact paths and a specific test name, so a
  verifier can conclusively confirm each item is gone.
- AC2 gives a directly verifiable check that a grep can settle.
- Criteria pair removals with explicit preservation checks.
- Kind-appropriate framing: verification grounded in file existence plus checks
  and suite passing.

**Findings**:
- 🟡 **major** (confidence: medium) — _Acceptance Criteria_: "Every positive
  bash-parity/golden test still exists" is not conclusively verifiable. A passing
  suite cannot detect an accidentally deleted positive test — a removed test
  simply stops running without failing. Provide an enumerated or count-based
  reference list, and phrase the criterion so a verifier compares the post-change
  inventory against that list.
- 🔵 **minor** (confidence: medium) — _Acceptance Criteria_: AC1 bundles multiple
  removals behind a broad "suite passes" outcome. The verifiable content reduces
  to the broad suite/check gate, and the circular Given/Then adds no independent
  verification. List each removed path as its own non-existence check, separate
  from the suite-green gate.

## Re-Review (Pass 2) — 2026-09-08

**Verdict:** COMMENT

Both major findings are resolved and no findings at or above the REVISE
threshold remain, so the verdict moves from REVISE to COMMENT. The work item is
acceptable for implementation; the residual findings are minor polish, several
of them introduced by the pass-1 edits (AC7/AC8 verification locations, the
`Blocked by` label wording).

### Previously Identified Issues

- 🟡 **Clarity**: Summary scope ('tests') narrower than Requirements — Resolved
  (Summary now names the `call_site_migration.py` guard removal; removal set
  enumerated consistently across sections).
- 🟡 **Testability**: "Every positive test still exists" not verifiable —
  Resolved (AC7 prescribes diffing the pre/post test-file listing rather than
  relying on a green suite).
- 🔵 **Dependency**: Sibling ordering asserted collectively — Resolved (siblings
  itemised; 0269 escalated to Open Questions and an acceptance criterion).
- 🔵 **Clarity**: 'config cluster' undefined — Resolved (glossed inline in
  Requirements).
- 🔵 **Testability**: AC1 bundles multiple removals — Resolved (split into
  per-path non-existence checks plus a separate suite-green gate).
- 🔵 **Scope**: Active guardrail bundled with dead-scaffolding removal —
  Addressed (sign-off-gated AC + Open Questions; re-review deems the bundling a
  defensible boundary judgement).
- 🔵 **Completeness**: ADR-0048 decision buried, no Open Questions — Resolved
  (Open Questions section added).
- 🔵 **Clarity**: 'key parts' vague — Resolved (replaced with the concrete
  `SURVIVING_SHELL_SOURCES` condition).
- 🔵 **Clarity**: ADR-0048 referenced without a link — Resolved via context in
  Drafting Notes. Note: an initial re-review conclusion that the ADR-0048
  citation was stale was wrong. ADR-0048 (four-toolchain split) makes Python the
  guardrail-test language for non-Rust surfaces, and `call_site_migration.py`
  cites it correctly as the authority for the guard being a Python lint — the
  same pattern used by `skill_permissions.py` and the launcher-link tests.

### New Issues Introduced

- 🔵 **Testability** (minor): AC7 baseline listing has no recorded location, so
  the diff is not independently reproducible — name how the baseline is captured
  (e.g. `git ls-files` at the parent commit).
- 🔵 **Testability** (minor): AC8 does not say where the reviewer sign-off record
  lives (PR approval, this work item, a decision log).
- 🔵 **Clarity** / **Dependency** (minor): "Blocked by: none known" reads as a
  contradiction next to "must follow" the siblings, and hides the unconfirmed
  0269 prerequisite — reorder so the reconciliation leads, or reflect 0269 in the
  Blocked-by field itself.
- 🔵 **Clarity** (minor): Terminology drift — the same tests are called
  'negative-assertion' and 'absence-assertion'; pick one.
- 🔵 **Clarity** (suggestion): 'pup' used without expanding to 'cargo-pup'.

### Assessment

The work item is ready for implementation. The two structural blockers are gone;
what remains is minor wording and two "where is it recorded" gaps on the AC7/AC8
verification hooks. These are quick to close if desired but do not gate planning.
The ADR-0048 citation in `call_site_migration.py` was initially suspected stale
but is correct: ADR-0048 makes Python the guardrail-test language for non-Rust
surfaces, so the docstring cites it appropriately. No code fix is needed.

