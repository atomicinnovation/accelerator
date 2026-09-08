---
type: "work-item-review"
id: "0269-remove-bash-references-from-jira-linear-clients-review-1"
title: "Work Item Review: Remove Bash References From Jira And Linear Clients"
date: "2026-09-06T09:57:52+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0269"
relates_to: ["work-item:0264", "work-item:0271", "work-item:0273"]
work_item_id: "0269"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-06T09:57:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Remove Bash References From Jira And Linear Clients

**Verdict:** REVISE

The work item is exemplary in completeness, clarity, and dependency mapping —
every section is populated with substantive content, referents resolve
unambiguously, and the frozen exit-code contract is anchored to a named oracle.
The REVISE verdict rests on two structural issues: the acceptance criteria have
verification gaps around the test/fixture reword and the `Cargo.toml` change, and
the work item openly bundles a low-risk vocabulary reword with a higher-risk
cross-crate classification redesign. Resolving the decomposition Open Question and
tightening two acceptance criteria would clear the path to implementation.

### Cross-Cutting Themes

- **Undefined "outcome and operation" contract** (flagged by: clarity,
  testability) — the redesigned error must carry "the outcome and operation" for
  the CLI to derive an exit code, but "operation" is never defined and the set of
  distinctions the error must represent is never enumerated. Clarity reads this as
  an ambiguous referent; testability reads it as a criterion that cannot be
  confirmed without knowing which distinctions the exit-code table demands.
- **Two-layer bundling and sibling overlap** (flagged by: scope, dependency) —
  the classification redesign is an independently deliverable, higher-risk layer
  that reshapes the same `classify` / `failure` / `client` files that siblings
  0271 and 0273 also intend to restructure. Scope frames this as a decomposition
  problem; dependency frames it as an unresolved ordering direction.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: Criterion 5 has no definitive pass/fail procedure
  **Location**: Acceptance Criteria (criterion 5)
  The criterion scopes the tests/fixtures reword to bash vocabulary "where it
  concerns exit codes" and permits other bash references to remain, so a grep over
  the test tree returns allowed and disallowed matches with no defined way to
  distinguish them. A verifier cannot conclusively confirm the criterion.

- 🟡 **Testability**: `Cargo.toml` reword is required but unverified
  **Location**: Requirements / Acceptance Criteria (criterion 1)
  Requirements and Technical Notes require rewording the `Cargo.toml` bash-era
  note in both crates, but criterion 1's grep is scoped to production `src/`, and
  `Cargo.toml` lives at the crate root outside `src/`. The change could be silently
  missed and the work still declared done against all criteria.

- 🟡 **Scope**: Safe reword bundled with risky classification redesign
  **Location**: Summary
  The Summary frames the work as two layers — a near-zero-risk cosmetic reword and
  an architectural redesign with cross-crate ripple into both `*-cli` binaries.
  These can be completed, reviewed, and rolled back independently; coupling them
  means the low-risk value cannot ship or revert without carrying the redesign's
  blast radius.

#### Minor

- 🔵 **Clarity**: "operation" and "provenance comments" left undefined
  **Location**: Requirements
  The redesigned error must carry "the outcome and operation" (Requirements bullet
  3), but "operation" is never defined — HTTP call, API operation category, or
  something else — and "provenance comments" (bullet 5) is used without gloss. If
  "operation" is read at the wrong granularity, the error may carry too little
  context for the CLI to derive the correct exit code.

- 🔵 **Testability**: Criterion 3 describes internal structure, not an outcome
  **Location**: Acceptance Criteria (criterion 3)
  "The CLI derives the exit code from the error's context" can only be assessed by
  reading the implementation; the genuinely observable outcome (exit-code values
  are unchanged) is already covered by criterion 6.

- 🔵 **Testability**: Criterion 2 not enumerable against the oracle
  **Location**: Acceptance Criteria (criterion 2)
  The error must carry "enough" outcome/operation context, but the set of outcomes
  and operations that must be representable is not enumerated, so a verifier cannot
  confirm sufficiency without the exit-code table's distinctions.

- 🔵 **Dependency**: "Consumed across crates" claimed but only two CLIs named
  **Location**: Context
  Context says the bash-named symbols are "consumed across crates" (plural), yet
  only `jira-cli` and `linear-cli` are enumerated as consumers. Any other consumer
  implied by "across crates" is left unnamed and would break at implementation
  time.

- 🔵 **Dependency**: Same-file ordering with 0271/0273 left unresolved
  **Location**: Dependencies
  Dependencies records that 0271 and 0273 refactor the same files "so coordinate
  ordering", but leaves them as symmetric "relates to" entries without deciding
  which lands first. Whichever lands second must absorb a rebase against the
  redesigned classification path.

#### Suggestions

- 🔵 **Scope**: Confirm the redesign belongs here, not in 0271/0273
  **Location**: Dependencies
  The higher-risk redesign reshapes the very files 0271 and 0273 intend to
  restructure; if the redesign lands here and is then reshaped again, the same
  structural change is paid for twice.

- 🔵 **Clarity**: Singular/plural drift for the CLI binaries
  **Location**: Summary
  The Summary alternates between "the CLI" and "the CLIs" while the work concerns
  two distinct binaries. A consistent form ("each CLI binary", or
  "the jira-cli and linear-cli binaries") removes the ambiguity.

### Strengths

- ✅ Every expected section is present with substantive, non-placeholder content;
  an implementer could begin a task of this kind without follow-up questions.
- ✅ Crates, files, symbols, and fixtures are named explicitly and used
  consistently, so "the client crates" and "the CLI boundary" never drift.
- ✅ The frozen exit-code contract is stated identically across Summary, Context,
  Requirements, Assumptions, and Acceptance Criteria, and anchored to a named
  oracle (`exit_codes_parity` / `flow_errors` suites plus
  `bridge-exit-code-tables.txt`).
- ✅ In-scope migration cruft is separated from legitimate domain vocabulary
  (`transport.rs` retry predicates), pre-empting a likely misinterpretation.
- ✅ The tracker crate's 70/71 dispatch codes and 0264's overlap are explicitly
  scoped out, resolving two plausible hidden couplings.
- ✅ Criteria 1, 2, 4, and 6 bind to concrete, reproducible procedures — a
  case-insensitive grep, symbol-name checks, a git-diff no-op, and a parity oracle.

### Recommended Changes

1. **Extend criterion 1's grep target to include `Cargo.toml`** (addresses:
   `Cargo.toml` reword is required but unverified)
   Reword criterion 1 to grep each crate's `src/` and `Cargo.toml`
   case-insensitively for `bash`, expecting zero matches, so the required
   `Cargo.toml` change is covered.

2. **Make criterion 5 verifiable** (addresses: Criterion 5 has no definitive
   pass/fail procedure)
   Enumerate the exact fixture files, field names, and message strings that must
   change, or scope a grep to the exit-code test files with an explicit allowlist
   of any remaining permitted `bash` occurrences.

3. **Resolve the decomposition Open Question** (addresses: Safe reword bundled with
   risky classification redesign; Confirm the redesign belongs here)
   Decide whether to split into a chore-sized vocabulary reword and a separate
   classification-redesign task, or state the indivisibility rationale explicitly.
   Confirm the redesign scope belongs to this task rather than 0271/0273.

4. **Define the error's required context and tie it to the oracle** (addresses:
   "operation" and "provenance comments" left undefined; Criterion 2 not enumerable)
   Define "operation" on first use (name the enum or granularity), gloss
   "provenance comments", and bind criterion 2 to the distinctions
   `bridge-exit-code-tables.txt` demands (codes 11–36).

5. **Record the 0271/0273 ordering as a hard relationship** (addresses: Same-file
   ordering with 0271/0273 left unresolved)
   Decide which item lands first and encode it as a Blocked-by / Blocks
   relationship rather than a symmetric coordination note.

6. **Enumerate the full consumer set** (addresses: "Consumed across crates" claimed
   but only two CLIs named)
   State explicitly that `jira-cli` and `linear-cli` are the complete set of
   consumers of the removed symbols, or name any others.

7. **Reframe or annotate criterion 3** (addresses: Criterion 3 describes internal
   structure, not an outcome)
   Recast as an observable check (the mapping accepts only the client error type,
   no `u16` parameter) or mark it explicitly as an inspection criterion.

8. **Fix the singular/plural CLI drift** (addresses: Singular/plural drift for the
   CLI binaries)
   Use a consistent form throughout the Summary.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually precise: crates, files, symbols, and
fixtures are named explicitly, the two-layer intent (vocabulary reword +
classification redesign) is stated consistently across Summary, Requirements, and
Acceptance Criteria, and the frozen exit-code contract is reinforced coherently in
every section. Referents ("the client crates", "the parity fixtures") resolve
unambiguously and actors are named for each behavioural change. The only clarity
gaps are a couple of lightly-defined domain terms and a minor singular/plural
inconsistency in how the CLI binaries are referred to.

**Strengths**:
- The two crates under change (`jira-client`/`linear-client`) and the CLI boundary
  files are named explicitly and used consistently.
- The frozen-contract constraint is stated identically and without contradiction
  across Summary, Context, Requirements, Assumptions, and Acceptance Criteria.
- Requirements use active, actor-named phrasing, so responsibility for each
  behavioural change is unambiguous.
- The distinction between in-scope migration cruft and legitimate domain
  vocabulary is spelled out explicitly.

**Findings**:
- 🔵 minor (confidence: medium) — Requirements. "operation" is never defined and
  "outcome" is only loosely tied to Context's "domain outcome"; "provenance
  comments" is likewise used without definition. The semantic contract the error
  must satisfy is the core of the redesign; a wrong-granularity reading of
  "operation" may carry too little context. Suggestion: define "operation" on
  first use and gloss "provenance comments".
- 🔵 suggestion (confidence: low) — Summary. The Summary alternates between "the
  CLI" (singular) and "the CLIs" (plural) while the work concerns two distinct
  binaries. A reader could momentarily read "the CLI" as one shared consumer.
  Suggestion: use a consistent form.

### Completeness

**Summary**: This is an exemplary, fully-populated task work item. Every expected
section — Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes, and References — is present with
substantive, non-placeholder content, and the frontmatter is complete and valid.
For a task kind, the definition of work to be done is unambiguous and an
implementer could begin without follow-up questions.

**Strengths**:
- The Summary gives a precise, two-layer statement of intent and explicitly flags
  the frozen exit-code contract constraint.
- Requirements are specific and enumerable (which symbols to remove, where the
  mapping moves, what to leave untouched).
- Acceptance Criteria contains six concrete, section-appropriate criteria.
- Context explains the motivation and why the exit-code contract is load-bearing,
  rather than merely restating the Summary.
- Optional sections are all genuinely populated and frontmatter fields are present
  and recognised.

**Findings**: None.

### Dependency

**Summary**: The work item is generally well dependency-mapped: it explicitly
captures the exit-code contract oracle, names the two consuming CLI crates that
must be updated, scopes out the tracker crate and its 70/71 codes, and flags the
same-file coordination risk with siblings 0271 and 0273. The main gaps are a
vaguely-scoped "consumed across crates" claim that is never fully enumerated, and
an unresolved ordering direction for the same-file conflict with 0271/0273.

**Strengths**:
- The frozen exit-code contract is captured as an explicit read-only dependency.
- The downstream CLI consumers (`cli/jira-cli`, `cli/linear-cli`) are named in
  both Requirements and Technical Notes.
- The same-file conflict coupling with 0271 and 0273 is explicitly noted.
- The tracker crate's dispatch codes and 0264's overlap are explicitly scoped out.

**Findings**:
- 🔵 minor (confidence: medium) — Context. The bash-named symbols are said to be
  "consumed across crates" (plural), yet only `jira-cli` and `linear-cli` are
  enumerated. If a consumer beyond the two CLIs references the removed symbols, its
  build breaks mid-sprint. Suggestion: enumerate every consumer or state the two
  CLIs are the complete set.
- 🔵 minor (confidence: high) — Dependencies. 0271 and 0273 refactor the same
  files, left as symmetric "relates to" entries without deciding which lands first;
  the second must absorb a rebase. Suggestion: record which precedes as an explicit
  Blocked-by / Blocks relationship.

### Scope

**Summary**: This task is coherent around a single theme — purging bash/exit-code
framing from the Jira and Linear client crates — with commendably explicit in-scope
and out-of-scope boundaries. However, it openly bundles two independently
deliverable layers: a low-risk cosmetic vocabulary reword and a higher-risk
architectural classification redesign that ripples across four crates. The work
item itself surfaces this as an Open Question, which is the central scope signal.

**Strengths**:
- Boundaries are stated with unusual precision (transport.rs retry predicates, the
  70/71 dispatch codes, and the frozen exit-code values all declared out of scope).
- The scope is anchored to a single coherent theme that matches the source backlog
  item, rather than a grab-bag of unrelated cleanups.
- The Open Questions section demonstrates the author's own awareness of the
  decomposition tension, deferring the split to planning rather than hiding it.

**Findings**:
- 🟡 major (confidence: medium) — Summary. The two layers can be completed,
  reviewed, and rolled back independently; coupling the safe reword to the risky
  redesign means the low-risk value cannot ship or revert without carrying the
  redesign's blast radius, and review/rollback of one entangles the other.
  Suggestion: resolve the Open Question in favour of splitting, or state the
  indivisibility rationale explicitly.
- 🔵 suggestion (confidence: low) — Dependencies. The redesign reshapes the same
  files 0271 and 0273 also intend to restructure; if it lands here and is reshaped
  again, the change is paid for twice. Suggestion: confirm the redesign belongs to
  this task and define the ordering.

### Testability

**Summary**: For a refactor/vocabulary task this work item is unusually
well-specified: most Acceptance Criteria bind to concrete procedures (a
case-insensitive grep, a git-diff no-op check, and a named test suite acting as
the exit-code oracle). The main testability gaps are the tests/fixtures rework
criterion, which permits non-exit-code "bash" to remain and so cannot be verified
by a clean grep, and the un-verified `Cargo.toml` rewording. One criterion
describes internal structure rather than an observable outcome.

**Strengths**:
- Criterion 1 (grep production `src/` for `bash`, expect zero matches) is a
  definitive, reproducible pass/fail procedure.
- Criterion 4 (`transport.rs` predicates unchanged) is trivially verifiable via git
  diff.
- Criterion 6 anchors the frozen exit-code contract to a named oracle, giving
  objective proof that emitted values are unchanged.
- Criterion 2 pins concrete removable symbol names (`bash_code`,
  `classify_bash_code`).

**Findings**:
- 🟡 major (confidence: high) — Acceptance Criteria (criterion 5). The criterion
  scopes the rework to bash vocabulary "where it concerns exit codes" and permits
  other bash references to remain, so a grep returns allowed and disallowed matches
  with no defined way to distinguish them. Suggestion: enumerate the exact fixture
  files, field names, and message strings, or scope a grep with an explicit
  allowlist.
- 🟡 major (confidence: high) — Requirements / Acceptance Criteria (criterion 1).
  The `Cargo.toml` reword is required but criterion 1's grep is scoped to
  production `src/`, and `Cargo.toml` lives outside `src/`, so the change has no
  verification. Suggestion: extend the grep target to include each crate's
  `Cargo.toml`.
- 🔵 minor (confidence: medium) — Acceptance Criteria (criterion 3). "The CLI
  derives the exit code from the error's context" describes internal structure and
  can only be assessed by reading the implementation; the observable outcome is
  already covered by criterion 6. Suggestion: reframe as an observable check or
  mark it as an inspection criterion.
- 🔵 minor (confidence: medium) — Acceptance Criteria (criterion 2). The set of
  outcomes and operations that must be representable is not enumerated, so
  "enough" context is not measurable. Suggestion: tie the requirement to the oracle
  (codes 11–36 in `bridge-exit-code-tables.txt`).

## Re-Review (Pass 2) — 2026-09-06

**Verdict:** COMMENT

Every major and minor from pass 1 is resolved. The two scope/testability majors
that drove the REVISE verdict are gone: the split Open Question is resolved with an
indivisibility rationale and phased delivery, and both acceptance-criteria gaps
(the `Cargo.toml` grep and the unverifiable test-sweep criterion) are closed. The
verdict moves to COMMENT — the work item is acceptable for implementation. A handful
of minor/suggestion items surfaced fresh, some as side-effects of the pass-1 edits;
the clear ones were tightened immediately, and two judgement calls (title breadth,
0271/0273 ordering direction) are left for the author.

### Previously Identified Issues

- 🟡 **Scope**: Safe reword bundled with risky redesign — Resolved (Open Questions
  now carries an indivisibility rationale; delivery phased reword-then-redesign).
- 🟡 **Testability**: Criterion 5 has no definitive pass/fail — Resolved (scoped to
  named suites; enumerated categories must be gone, any remainder justified inline).
- 🟡 **Testability**: `Cargo.toml` reword unverified — Resolved (criterion 1's grep
  now targets `src/` and `Cargo.toml`).
- 🔵 **Clarity**: "operation"/"provenance comments" undefined — Resolved (operation
  defined as the client method invoked; provenance comments glossed).
- 🔵 **Testability**: Criterion 3 describes internal structure — Resolved (recast as
  an observable check: mapping accepts only the client error type, no `u16`).
- 🔵 **Testability**: Criterion 2 not enumerable — Resolved (tied to codes 11–36).
- 🔵 **Dependency**: Consumers vaguely "across crates" — Resolved (Context names
  `jira-cli`/`linear-cli` as the only cross-crate consumers).
- 🔵 **Dependency**: 0271/0273 ordering unresolved — Partially resolved (recommended
  order recorded and cross-referenced from the `Blocks:` line; hard edge still
  deferred to scheduling by author decision).
- 🔵 **Clarity**: CLI singular/plural drift — Resolved.
- 🔵 **Scope**: Redesign overlaps 0271/0273 — Resolved (recommended landing order
  documented).

### New Issues Introduced

- 🔵 **Testability**: AC2 "no numeric exit code is computed" lacked a mechanical
  check — Addressed in re-review (now forbids a `u16` field and 11–36 integer
  literals in `classify.rs`/`failure.rs`).
- 🔵 **Testability/Clarity**: AC5 deferred its pass condition to a not-yet-existing
  "plan allowlist" (unbounded escape hatch) — Addressed in re-review (rewritten to
  require the enumerated categories gone and any remainder justified inline, no
  allowlist dependency).
- 🔵 **Clarity**: `AdfError::code -> u16` role undefined — Addressed in re-review
  (Technical Notes now marks its numeric return in scope for the u16 removal).
- 🔵 **Clarity**: Title under-represents the structural redesign — Open. Broadening
  the title is a rename left to the author.

### Assessment

The work item is ready for implementation. Both outstanding author judgement calls
were then resolved in this pass — the title was broadened to name the classification
redesign, and the 0271/0273 ordering was pinned as a hard `blocks` edge (this item
lands first). The author has accepted the work item; the review verdict is
**APPROVE**.
