---
type: work-item-review
id: "0116-structured-stall-on-no-decision-input-review-1"
title: "Work Item Review: Structured Stall on No Decision Input"
date: "2026-06-20T00:47:34+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0115"
target: "work-item:0116"
work_item_id: "0116"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [migrate, interactive-migration, agent-invocation]
last_updated: "2026-06-20T00:47:34+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Structured Stall on No Decision Input

**Verdict:** REVISE

This is a tightly-scoped, well-decomposed task: a single coherent concern
(fix B from the source research), all sections present and substantively
populated, clean Given/When/Then acceptance criteria, and a correctly
justified "blocked by: none". The reason for REVISE is narrow and
concentrated: the central deliverable — a "structured stall" — is asserted
but never given a verifiable shape. Two major testability findings (the
"exact resume command" has no assertable form, and "names the pending
decision key" is left ambiguous between one key and all keys) mean a verifier
cannot deterministically confirm a pass. Tightening the stall's definition
resolves the bulk of the findings across all five lenses at once.

### Cross-Cutting Themes

- **The "structured stall" is under-specified** (flagged by: testability,
  completeness, clarity) — The work item's central artifact has no defined
  shape. Testability cannot assert the "exact resume command" or distinguish
  the stall from an abort at the exit-status level; completeness notes the
  exit semantics and output framing are uncaptured; clarity notes the term
  "structured stall" is never defined. One concrete definition (exit
  behaviour + required message contents + resume-command form) closes all of
  these.
- **Decision-key cardinality is inconsistent** (flagged by: clarity,
  testability) — Requirements say "key(s) accumulated so far", AC1 says "the
  pending decision key" (singular), and Assumptions hedges "may list only the
  current key". The three disagree on how many keys the stall must name, so
  neither an implementer nor a verifier can tell whether listing only the
  current key is a pass.
- **The resume command couples to sibling 0119** (flagged by: dependency,
  scope) — The stall must print a resume command matching the pre-flight hint
  (`run-migrations.sh:90-132`), but the research (H3) notes that hint only
  covers an in-flight interactive session log, not the partial-mechanical
  failure state this stall is reached in — the resume-safe path is sibling
  0119's domain. The "actionable" stall may point at a path the current
  pre-flight refuses.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Stall output content (resume command) lacks a verifiable shape
  **Location**: Acceptance Criteria
  AC1 and AC2 require the stall to print "the exact resume command", but the
  criterion never specifies what that command is or how a verifier confirms it
  is exact. The Technical Notes only say the shape "should match" a code
  reference, which is not an assertable expected string — a wrong-but-present
  command would still pass.

- 🟡 **Testability**: "Names the pending decision key" is not bounded to an observable assertion
  **Location**: Acceptance Criteria
  AC1 requires the stall to name "the pending decision key", but Assumptions
  concedes only the current key may be available. The criterion does not state
  which outcome is a pass, so the same test could be judged pass or fail
  depending on interpretation.

#### Minor

- 🔵 **Clarity**: "fix B of 0115" and "fix A (0117)" undefined within the work item
  **Location**: Summary
  The labels "fix A"/"fix B" are only defined in the referenced research doc's
  Fix Options table, never within this work item, forcing a lookup to confirm
  this item is the stall and not the bridge.

- 🔵 **Clarity**: Inconsistent singular/plural on how many decision keys the stall names
  **Location**: Acceptance Criteria
  Requirements say "key(s) accumulated so far", AC1 says "the pending decision
  key", and Assumptions says "may list only the current key" — three different
  scopes with no reconciled normative statement.

- 🔵 **Dependency**: Resume command's viability is implicitly coupled to sibling 0119
  **Location**: Technical Notes
  The stall prints a resume command matching the pre-flight hint, but the
  research notes that hint only special-cases an interactive session log, not a
  partial mechanical run — making a partial-run resume safe is fix E / sibling
  0119. The "actionable" stall could point at a path the current pre-flight
  refuses.

- 🔵 **Testability**: Non-regression criterion lacks a defined observable signal
  **Location**: Acceptance Criteria
  AC3 ("behaviour is unchanged / no regression in the satisfiable path") states
  a no-regression goal but names no concrete observable, so it can always be
  claimed met.

- 🔵 **Testability**: No criterion distinguishes the stall from an abort at the exit-status level
  **Location**: Acceptance Criteria
  The criteria assert message content but say nothing about terminal behaviour —
  whether the stall is a non-zero abort, a distinct exit code, or a clean
  deferral — leaving the deferral semantics untestable beyond the message text.

#### Suggestions

- 🔵 **Clarity**: Central term "structured stall" is used but never defined
  **Location**: Summary
  The work item's central concept is never defined; "stall" could be read as
  "abort with a better message", "pause and wait", or "exit cleanly for later
  resume" — each implying different exit semantics.

- 🔵 **Clarity**: Second criterion's "the same structured stall" relies on a cross-bullet referent
  **Location**: Acceptance Criteria
  AC2 introduces a second baseline message ("failed to obtain re-decision") that
  Context and Summary never mention (they cite only "failed to obtain
  decision"); both baseline messages should be introduced before the criteria
  reference them.

- 🔵 **Completeness**: Stall exit semantics / output framing not captured in Requirements
  **Location**: Requirements
  Requirements specify what the stall must name but not its exit behaviour or
  output framing (e.g. non-zero exit with a named stall marker); an implementer
  must infer these from the existing emit sites.

- 🔵 **Dependency**: Sibling fix set (0118, 0119) not named in Dependencies
  **Location**: Dependencies
  The research frames B, C, D, E as a coordinated set for one failure, but only
  parent 0115, sibling 0117, and related 0069 are named — a scheduler cannot see
  this mitigation is one of several coordinated fixes.

- 🔵 **Scope**: Resume-command shape depends on a different work item's domain
  **Location**: Requirements
  The boundary of this task (emit a message) is clean, but the message content
  reuses a region (the resume command) that sibling 0119 owns and may change;
  worth noting the dependency to keep scope limited to reusing, not defining,
  the resume command.

### Strengths

- ✅ Tightly-scoped and coherent: all four requirements serve one purpose
  (detect no-input, emit a structured stall) with no bundled concerns; the two
  emit sites are the same mechanism in two places, not two features.
- ✅ Clean decomposition from the research issue — takes fix B only, explicitly
  leaving fixes A/C/E to siblings 0117/0118/0119, making the unit atomic and
  independently shippable.
- ✅ Context explains the failure mechanism (the `read_decision` input-source
  chain failing under agent invocation, surfacing as an opaque abort after
  corpus mutation) rather than restating the Summary.
- ✅ The two emit sites are disambiguated with exact `file:line` anchors
  (`interactive-lib.sh:450` and `:485`), so "both emit sites" has exactly one
  interpretation throughout.
- ✅ Acceptance criteria use a consistent Given/When/Then form with named actors
  and include a no-regression criterion for the satisfiable path, exceeding the
  two-criterion minimum.
- ✅ Frontmatter is complete and coherent (kind=task, status=draft,
  priority=high, parent and relates_to populated); Open Questions and
  Dependencies are explicitly resolved ("None." / "Blocked by: none") rather
  than left blank.
- ✅ The "blocked by: none" claim is well-justified — the research confirms fix B
  is independent and "worth shipping regardless".

### Recommended Changes

1. **Define what a "structured stall" is** (addresses: "Stall output content
   lacks a verifiable shape", "No criterion distinguishes the stall from an
   abort", "Central term 'structured stall' never defined", "Stall exit
   semantics not captured")
   Add a short definition — to Requirements or a new line in Context — pinning
   down (a) the exit behaviour (e.g. non-zero exit, distinct from the old
   abort, or a documented exit code), (b) the required message contents, and
   (c) the concrete form of the resume command (e.g.
   `ACCELERATOR_MIGRATE_DECISIONS_FILE=<path> … run-migrations.sh` with the
   placeholder fields a verifier should check). Then tighten AC1/AC2 to assert
   that concrete shape rather than "the exact resume command".

2. **Reconcile decision-key cardinality to one normative statement** (addresses:
   "Names the pending decision key is not bounded", "Inconsistent
   singular/plural on decision keys")
   Pick one rule — e.g. "the stall names at least the current pending decision
   key, and all accumulated keys when derivable" — and align Requirements, AC1,
   and Assumptions to it. Make AC1 assert the guaranteed-observable case (the
   current key) so it is deterministically verifiable, with listing additional
   keys as an optional/separate criterion.

3. **Anchor the non-regression criterion to an observable** (addresses:
   "Non-regression criterion lacks a defined observable signal")
   Replace AC3's "behaviour is unchanged" with a concrete signal — e.g. "with a
   decisions file supplied, the migration consumes each decision and completes
   with the same exit code and recorded-decision count as before", or reference
   the existing `test-migrate-interactive.sh` suite that must still pass.

4. **Surface the resume-command coupling to 0119 and the sibling set**
   (addresses: "Resume command's viability is implicitly coupled to 0119",
   "Resume-command shape depends on a different work item's domain", "Sibling
   fix set not named in Dependencies")
   Add a Dependencies/Assumptions note that the resume command is borrowed from
   the existing pre-flight session-log hint and is meaningful for that resume
   path; cross-reference siblings 0118 and 0119 as "Relates to" so the
   coordinated fix set and the partial-run-resume limitation are visible.

5. **Gloss the "fix A/B" labels and the second baseline message** (addresses:
   "fix B/fix A undefined", "the same structured stall cross-bullet referent")
   On first use, gloss "fix B — the structured-stall mitigation" and "fix A —
   the agent↔decisions bridge"; introduce the "failed to obtain re-decision"
   baseline message in Context alongside "failed to obtain decision" so both are
   defined before AC2 references them.

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear and internally consistent: the
problem in Context maps cleanly onto the Requirements and Acceptance Criteria,
line-number anchors disambiguate the two emit sites, and the central failure
mode is described unambiguously. The main clarity gaps are the undefined
references to "0115 fix B" / "0117 fix A" (which only resolve by reading the
research doc), the never-defined central term "structured stall", and a
singular/plural inconsistency about how many decision keys the stall must name
across Requirements, Acceptance Criteria, and Assumptions.

**Strengths**:
- The Context section precisely names the failure path (fd 0 → /dev/tty → bare
  fd 0) and ties the opaque message to a specific handler.
- The two emit sites are disambiguated with exact file:line anchors
  (interactive-lib.sh:450 and :485).
- Acceptance Criteria use a consistent Given/When/Then form with named actors
  and concrete observable outcomes.

**Findings**:
- 🔵 minor (high) — **"fix B of 0115" and "fix A (0117)" undefined within the
  work item** (Summary): The labels are only defined in the research doc's Fix
  Options table, forcing a lookup. Suggestion: gloss the labels inline on first
  use.
- 🔵 suggestion (medium) — **Central term "structured stall" used but never
  defined** (Summary): Meaning inferred from contrast with the bare abort;
  "stall" could mean abort-with-better-message, pause-and-wait, or
  exit-cleanly. Suggestion: add a one-line definition of exit status + message
  contents.
- 🔵 minor (medium) — **Inconsistent singular/plural on decision keys**
  (Acceptance Criteria): Requirements say "key(s)", AC1 says "key", Assumptions
  says "may list only the current key" — three scopes, no reconciliation.
  Suggestion: pick one normative statement and align all three.
- 🔵 suggestion (low) — **Second criterion's "the same structured stall" relies
  on cross-bullet referent** (Acceptance Criteria): AC2 introduces a second
  baseline message ("failed to obtain re-decision") not mentioned earlier.
  Suggestion: introduce both baseline messages in Context/Requirements.

### Completeness

**Summary**: A well-structured task with all expected sections present and
substantively populated — a clear single-statement Summary, a Context that
explains the failure mechanism, specific Requirements, three Acceptance
Criteria, and populated Dependencies, Assumptions, Technical Notes, and
References. For its kind (task), the definition of work is clear and an
implementer could begin without follow-up questions. Frontmatter integrity is
sound.

**Strengths**:
- All expected sections for a task are present and substantively populated, with
  no empty or placeholder-only sections.
- The Summary states a single, unambiguous action.
- Acceptance Criteria contains three specific given/when/then criteria including
  a no-regression criterion.
- Context explains why the work is needed rather than restating the Summary.
- Frontmatter is complete and coherent (kind=task, status=draft, priority=high,
  parent and relates_to populated).
- Open Questions and Dependencies explicitly addressed rather than left blank.

**Findings**:
- 🔵 suggestion (low) — **Stall shape / exit semantics not captured in
  Requirements** (Requirements): Requirements specify what the stall must name
  but not the exit semantics or output framing; an implementer must infer the
  exit code and framing from the existing emit sites. Suggestion: add a
  one-line requirement on expected exit behaviour and output framing, or note
  it inherits the existing abort's exit semantics.

### Dependency

**Summary**: Dependency capture is strong: the parent (0115), the related
runner-side path (0069), and the sibling fix-A work item (0117) are all named
explicitly, and the Dependencies section correctly states that 0117 builds on
the same code region without requiring this task. The work is genuinely
standalone and low-risk, so "Blocked by: none" is well-supported. The one
interpretive gap is a soft coupling between the structured stall's resume
command and whether a viable resume path actually exists.

**Strengths**:
- The relationship to sibling 0117 is explicitly captured in both Summary and
  Dependencies, including the "same code region but does not require this" note
  that prevents a false ordering dependency.
- Parent 0115 and related 0069 are consistently named across frontmatter and the
  Dependencies section, with their roles clarified.
- Technical Notes name the internal code regions and the resume-hint source,
  making the implementation coupling visible.
- The "Blocked by: none" claim is well-justified by the research.

**Findings**:
- 🔵 minor (medium) — **Resume command's viability implicitly coupled to 0119**
  (Technical Notes): The research notes the pre-flight hint only covers an
  in-flight interactive session log, not a partial mechanical run (fix E /
  sibling 0119). If the stall is reached in a partial-mechanical-failure state,
  the resume command could point at a path the current pre-flight refuses.
  Suggestion: add a Dependencies note clarifying the resume command applies to
  the interactive-session-log path and cross-reference 0119.
- 🔵 suggestion (medium) — **Sibling fix set (0118, 0119) not named in
  Dependencies** (Dependencies): The research frames B/C/D/E as a coordinated
  set; without naming the siblings, a scheduler cannot see this is one of
  several coordinated fixes. Suggestion: add 0118/0119 as "Relates to".

### Scope

**Summary**: A tightly-scoped, coherent task implementing exactly one concern
from the research issue's recommended fixes (fix B): converting the opaque
abort into a structured stall. All requirements, acceptance criteria, and the
summary describe the same single deliverable, correctly carved out from parent
0115 as the standalone low-risk mitigation. The declared kind (task) and size
(S) match the scope described; this is a well-bounded unit of work.

**Strengths**:
- All four requirements serve one unified purpose with no bundled independent
  concerns.
- The two emit sites are the same mechanism applied in two places, not two
  separate features — correct cohesion, not scope creep.
- Acceptance criteria map one-to-one onto the requirements and include a
  no-regression criterion.
- Clean decomposition: takes fix B only, leaving A/C/E to siblings.
- Dependencies confirm independent deliverability.

**Findings**:
- 🔵 suggestion (medium) — **Resume-command shape depends on a different work
  item's domain** (Requirements): The partial-failure resume path is sibling
  0119's subject; if 0119 changes the resume mechanism, the resume-command text
  could drift. The task boundary (emit a message) is clean but the message
  content references a region another work item owns. Suggestion: note in
  Dependencies/Assumptions that the resume-command shape is borrowed and may
  need reconciliation if 0119 changes it.

### Testability

**Summary**: The Acceptance Criteria are well-framed as Given/When/Then
behaviours covering the two failure emit sites and the non-regression path, and
the trigger conditions (no decisions file, no TTY, fd 0 at EOF) are concretely
specified. The main gap is that the stall's required output content — naming
the pending decision key(s) and printing the "exact resume command" — is
asserted without a defined shape, so a verifier cannot deterministically
confirm a pass. A secondary gap is that "behaviour is unchanged" is not
anchored to any observable signal.

**Strengths**:
- AC1 and AC2 fully specify precondition, action, and expected outcome, giving a
  verifier a clear pass/fail boundary.
- The criteria explicitly distinguish the two emit sites, so a tester knows both
  paths must be exercised separately.
- Negative-case framing is concrete and directly assertable against output.

**Findings**:
- 🟡 major (high) — **Stall output content (resume command) lacks a verifiable
  shape** (Acceptance Criteria): AC1/AC2 require "the exact resume command" but
  never specify what it is or how to confirm it is exact; a wrong-but-present
  command would still pass. Suggestion: add a concrete expected-output spec
  (e.g. `ACCELERATOR_MIGRATE_DECISIONS_FILE=<path> … run-migrations.sh` with
  the fields the verifier should check).
- 🟡 major (high) — **"Names the pending decision key" not bounded to an
  observable assertion** (Acceptance Criteria): Assumptions concedes only the
  current key may be available; the criterion does not state which outcome is a
  pass. Suggestion: pin the criterion to the guaranteed-observable case ("names
  at least the current pending decision key").
- 🔵 minor (high) — **Non-regression criterion lacks a defined observable
  signal** (Acceptance Criteria): AC3 states a no-regression goal but no
  concrete observable, so it can always be claimed met. Suggestion: anchor it to
  observable outcomes (same exit code + recorded-decision count) or reference
  the existing interactive test suite.
- 🔵 minor (medium) — **No criterion distinguishes the stall from an abort at
  the exit-status level** (Acceptance Criteria): The criteria assert message
  content but nothing about terminal behaviour. Suggestion: add a criterion
  specifying the expected exit status / process outcome.

---

## Re-Review (Pass 2) — 2026-06-20

**Verdict:** APPROVE

Both blocking majors from review 1 are resolved. The re-review run surfaced one
*new* major — introduced by the review-1 edits — in the all-keys acceptance
criterion; it was fixed in the same pass (the all-keys behaviour was demoted
from an acceptance criterion to a best-effort note). What remains is
minor/suggestion-level polish (jargon glosses, an actor glossary, elevating the
0119 coupling to an explicit dependency entry) that does not block
implementation. The work item is ready to plan.

### Previously Identified Issues

- 🟡 **Testability**: Stall output (resume command) lacks a verifiable shape —
  Resolved. AC1/AC2 now require a non-empty `ACCELERATOR_MIGRATE_DECISIONS_FILE`
  path, the migration id as a literal substring, and the `run-migrations.sh`
  invocation.
- 🟡 **Testability**: "Names the pending decision key" not bounded — Resolved.
  Cardinality reconciled to "at least the current key" as the firm guarantee.
- 🔵 **Clarity**: singular/plural inconsistency on decision keys — Resolved.
  Requirements, ACs, and Assumptions now agree on current-key-firm /
  all-keys-best-effort.
- 🔵 **Clarity**: "structured stall" never defined — Resolved. Defined inline in
  the Summary (non-zero halt + parseable block).
- 🔵 **Clarity**: "fix A/B" undefined; second baseline message late — Partially
  resolved. Both baseline messages now in Context and fix B defined inline; fix
  A/C/E labels still glossed only in Dependencies (now a minor).
- 🔵 **Completeness**: stall exit semantics / framing not captured — Resolved.
  Non-zero exit and output framing now in Requirements.
- 🔵 **Dependency**: resume command coupled to 0119 — Resolved. Soft coupling and
  reconciliation note added to Dependencies and Assumptions.
- 🔵 **Dependency**: sibling fix set not named — Resolved. 0117/0118/0119 added to
  `relates_to` and Dependencies.
- 🔵 **Testability**: non-regression criterion lacked an observable — Resolved.
  AC tied to exit status, recorded-decision count, and `test-migrate-interactive.sh`.
- 🔵 **Testability**: stall not distinguished from abort at exit-status level —
  Resolved. Non-zero exit now asserted in AC1/AC2.
- 🔵 **Scope**: resume-command shape depends on 0119's domain — Resolved.
  Documented as a borrowed shape with reconciliation note.

### New Issues Introduced

- 🟡 **Testability** (Acceptance Criteria): all-keys criterion (AC3) gated on an
  implementation-internal "cheaply derivable" trigger a black-box verifier
  cannot evaluate — **Fixed this pass** by demoting all-keys to a best-effort
  note and keeping only the current-key guarantee as testable. Scope and clarity
  flagged the same root (conditional second behaviour / undefined "cheaply
  derivable" threshold) — also resolved by the demotion.

### Remaining (non-blocking) suggestions

- 🔵 **Clarity**: gloss fix A/C/E labels at first mention; add a one-line actor
  glossary (driver / runner / agent / invoker / user); gloss PROMPT and
  VALIDATE_ERR "frame" terms or link the protocol docs.
- 🔵 **Dependency**: optionally elevate the `run-migrations.sh:90-132` hint-shape
  reuse and the intended 0116-before-0117 merge ordering to explicit Dependency
  entries; note 0118 as a functional precondition for end-to-end reachability.
- 🔵 **Testability**: optionally add a criterion asserting the stall conforms to a
  defined parseable shape (marker + extractable fields).

### Assessment

The work item is ready for implementation. The two original blocking issues and
the one regression introduced during iteration are all resolved; the remaining
items are optional polish that can be addressed during planning or left as
implementer discretion.

---
*Review generated by /accelerator:review-work-item*
