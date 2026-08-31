---
type: "work-item-review"
id: "0272-relocate-insecure-local-override-marker-review-1"
title: "Work Item Review: Relocate Insecure-Local Override Marker to .accelerator"
date: "2026-08-31T17:18:16+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0272"
work_item_id: "0272"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-31T20:26:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Relocate Insecure-Local Override Marker to .accelerator

**Verdict:** COMMENT

This task is ready for implementation. All five lenses found a single, coherent,
well-bounded unit of work — relocating one marker file with a hard cutover — that
is unambiguously stated, structurally complete, thoroughly dependency-mapped, and
verifiable through Given/When/Then acceptance criteria. No critical or major
findings surfaced; the observations below are polish (naming consistency, two
acceptance-criteria coverage gaps, one release-ordering constraint left in prose)
that would strengthen the item without blocking it.

### Cross-Cutting Themes

- **Acceptance criteria verify removal and the happy path, but not the positive
  new-name or the preserved security gate** (flagged by: testability) — the grep
  criterion catches the old name's removal, and AC1 exercises the honour path, yet
  nothing confirms the docs name the *correct* new path, and no negative case at
  the new path guards the regular-file/non-symlink/VCS-tracked semantics the item
  insists stay identical.
- **"the resolver" is used without being tied to the named function** (flagged by:
  clarity, and echoed in testability's criteria) — Context names
  `refuse_insecure_personal_config`; the Acceptance Criteria and Technical Notes
  switch to "the resolver" without equating the two.

### Findings

#### Critical

_None._

#### Major

_None._

#### Minor

- 🔵 **Testability**: New basename rename has only negative verification, no positive check
  **Location**: Acceptance Criteria
  The grep AC verifies the old `insecure-local-ok` string is gone, not that the
  docstring and SKILL.md name the correct new path; a doc left with a stale or
  wrong path (e.g. `.accelerator/insecure-local-ok`) could pass every criterion,
  and `mise run check` does not validate documentation prose.

#### Suggestions

- 🔵 **Testability**: Preserved security semantics at the new path are unverified
  **Location**: Acceptance Criteria
  AC1 exercises only the happy path with a regular, VCS-tracked marker. No
  criterion confirms a symlinked, untracked, or non-regular marker at the *new*
  path is still refused, so a check accidentally relaxed during the move could pass
  all four criteria — and this is a credential-security gate.
- 🔵 **Dependency**: Implicit release-ordering constraint captured only in prose
  **Location**: Dependencies
  The hard cutover's safety rests on the insecure-local feature being unreleased,
  making this an implicit ordering constraint (must land before that feature
  ships). Dependencies records "Blocks: none" and leaves the constraint in the
  Assumptions narrative, invisible to whoever plans the release.
- 🔵 **Clarity**: "the resolver" introduced without being tied to the named function
  **Location**: Acceptance Criteria
  Context names the acting component `refuse_insecure_personal_config`; the
  Acceptance Criteria and Technical Notes refer to "the resolver" without
  equating the two, so a reader must infer they are the same code path.
- 🔵 **Scope**: Basename rename is an added scope beyond the source backlog item
  **Location**: Drafting Notes
  The source backlog asks only to move the marker under `.accelerator`; this item
  also renames the basename to `allow-insecure-local`, which the Drafting Notes
  flag as a reviewable naming call. The rename is tightly coupled to the move, so
  bundling is defensible — the only risk is a rejected name stalling the
  otherwise-mechanical relocation.
- 🔵 **Clarity**: "it pairs with" has a slightly loose referent
  **Location**: Summary
  In "aligning the basename with the `ACCELERATOR_ALLOW_INSECURE_LOCAL`
  environment variable it pairs with", the trailing "it" sits close enough to
  "environment variable" to invite a re-read; the intended reading (the marker
  pairs with the env var) is recoverable but adds friction.

### Strengths

- ✅ Single, tightly bounded purpose: every requirement traces back to moving one
  marker path, with override semantics explicitly held constant and the in/out
  boundary clear.
- ✅ Structurally complete: every expected section is present with substantive,
  kind-appropriate content, and frontmatter (`kind`, `status`, `priority`) is
  intact — no gap would force an implementer to seek clarification.
- ✅ Exceptionally dependency-mapped for its size: the three context builders, the
  three fixture sets, the paired env var, the docstring, the configure skill doc,
  and the `.accelerator/.gitignore` trackability constraint are all named.
- ✅ Strong acceptance criteria: AC1/AC2 are precise Given/When/Then input-output
  pairs pinned to the named error code `E_LOCAL_PERMS_INSECURE`; AC3 supplies a
  concrete, runnable grep with a bounded exception.
- ✅ The hard-cutover decision is stated identically across Summary, Context,
  Requirements, and Assumptions — one coherent intent with no scope drift — and
  the no-migration call is justified by an explicitly captured assumption.
- ✅ Drafting Notes pre-empt reviewer questions by flagging the basename rename as
  a deliberate, reviewable naming call.

### Recommended Changes

1. **Add a positive-name acceptance criterion** (addresses: New basename rename has
   only negative verification) — e.g. "a grep for `allow-insecure-local` returns a
   match in both `credentials.rs` (docstring) and
   `skills/config/configure/SKILL.md`", pairing the existing removal check with a
   presence check for the intended new name.
2. **Add a negative-path criterion at the new location** (addresses: Preserved
   security semantics are unverified) — e.g. "given an untracked or symlinked
   `.accelerator/allow-insecure-local`, the resolver still refuses with
   `E_LOCAL_PERMS_INSECURE`", or explicitly note that the existing semantic tests
   are re-pointed at the new path and continue to cover these cases.
3. **Surface the release-ordering constraint in Dependencies** (addresses:
   Implicit release-ordering constraint in prose) — note that this task must
   complete before the insecure-local feature is released, referencing the
   feature/release work item if one exists, so the ordering is visible to
   schedulers rather than living only in Assumptions.
4. **Unify the "resolver" terminology** (addresses: "the resolver" introduced
   without being tied) — introduce "the resolver" in Context alongside
   `refuse_insecure_personal_config`, or reuse the function name in the Acceptance
   Criteria, so both names clearly denote one code path.
5. **Tighten the Summary pronoun** (addresses: "it pairs with" loose referent) —
   recast to name the subject, e.g. "the environment variable the marker pairs
   with".

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its intent unambiguously: a single,
coherent scope (relocate one marker file, hard cutover) is stated consistently
across Summary, Context, Requirements, Acceptance Criteria, and Assumptions, with
active-voice requirements and named file paths. Referents like "the marker" resolve
cleanly, and the one octal/error-code jargon in play is anchored to concrete source
locations. The only mild wrinkle is a late-introduced term ("the resolver") that is
never explicitly tied to the function named earlier.

**Strengths**:
- The hard-cutover decision is stated identically and without contradiction across
  Summary, Context, Requirements, and Assumptions — a single coherent intent with
  no scope drift between sections.
- "The marker" has one stable referent throughout, and the old/new paths are always
  written in full rather than pronominalised, eliminating path ambiguity.
- Requirements are phrased in active voice with named targets (specific files and
  functions), so who changes what is never in question.
- Drafting Notes explicitly flag the basename rename as a reviewable naming call,
  pre-empting a reader's "why this name?" question.

**Findings**:
- **suggestion** (confidence: medium) — *"the resolver" introduced without being
  tied to the named function* — Location: Acceptance Criteria. The Acceptance
  Criteria (and Technical Notes) refer to "the resolver" running against a personal
  config, but the Context names the acting component as the function
  `refuse_insecure_personal_config`. The two are never explicitly equated, so a
  reader must infer that "the resolver" is that function. Suggestion: use one
  consistent name.
- **suggestion** (confidence: low) — *"it pairs with" has a slightly loose
  referent* — Location: Summary. In "aligning the basename with the
  `ACCELERATOR_ALLOW_INSECURE_LOCAL` environment variable it pairs with", the
  pronoun "it" sits close enough to "environment variable" to invite a re-read.
  Suggestion: recast to name the subject explicitly.

### Completeness

**Summary**: This task-kind work item is structurally complete and well populated:
every expected section (Summary, Context, Requirements, Acceptance Criteria,
Dependencies, Assumptions, Technical Notes, References) is present and carries
substantive, kind-appropriate content. The frontmatter is intact with a recognised
`kind` (task), valid `status` (draft), and `priority`. As a task it clearly defines
the work to be done, and no completeness gaps would force an implementer to seek
clarification.

**Strengths**:
- The Summary is a single unambiguous action statement, including the naming
  rationale and the hard-cutover decision.
- Context explains the motivation and why no migration is needed (unreleased
  functionality).
- Acceptance Criteria contains four specific criteria covering the honour path, the
  legacy-path refusal, a grep-based cleanup check, and the CI gate.
- Requirements enumerate the concrete work across production builders, test
  fixtures, and documentation.
- Frontmatter integrity is sound: `kind`, `status`, `priority`, and identifying
  fields are all present and set to recognised values.
- Optional sections (Dependencies, Assumptions) are populated appropriately.

**Findings**: None.

### Dependency

**Summary**: This task is exceptionally well dependency-mapped for its size: it
explicitly declares no upstream blockers and no downstream consumers, and it
enumerates every internal coupling the path move touches — the three independent
context builders, the three test-support fixture sets, the paired
`ACCELERATOR_ALLOW_INSECURE_LOCAL` env var, the docstring, the configure skill doc,
and even the `.accelerator/.gitignore` trackability constraint. The only coupling
not surfaced in the Dependencies section is the implicit scheduling constraint that
this hard cutover must land before the (currently unreleased) insecure-local
feature ships, which the Assumptions capture in prose but the Dependencies leave as
"Blocks: none".

**Strengths**:
- Dependencies section explicitly states "Blocked by: none" and "Blocks: none", and
  the surrounding content genuinely supports this.
- Every internal coupling of the path move is named.
- The `.accelerator/.gitignore` coupling is explicitly identified and verified in
  Technical Notes rather than left as a hidden assumption.
- The hard-cutover decision is justified by an explicitly captured assumption.

**Findings**:
- **suggestion** (confidence: medium) — *Implicit release-ordering constraint
  captured only in prose, not as a Blocks/scheduling dependency* — Location:
  Dependencies. The safety of the hard cutover rests on the feature being
  unreleased, so the task must land before that feature ships; the Dependencies
  section records "Blocks: none" and leaves the constraint in prose only. If the
  feature is released first, the no-migration-aid decision silently becomes wrong.
  Suggestion: note the ordering constraint in Dependencies.

### Scope

**Summary**: This task describes a single, coherent unit of work: relocating (and
renaming) the insecure-local override marker file. All requirements — the three
context-builder path changes, the fixture updates, the docstring/doc references, and
the hard cutover — serve that one purpose and would be delivered, verified, and
rolled back together. The `task` kind is appropriate for a mechanical, atomic path
change with no user-facing behavioural shift, and there are no independent concerns
bundled in.

**Strengths**:
- Tightly bounded single purpose: every requirement traces back to moving one marker
  path, with override semantics explicitly held constant.
- The `task` kind fits the mechanical, atomic nature of the change.
- Although the change touches multiple crates plus test-support and a skill doc,
  these are all facets of one coordinated rename, correctly kept as one unit.
- The Summary, Requirements, and Acceptance Criteria describe the same scope, with
  no drift.

**Findings**:
- **suggestion** (confidence: low) — *Basename rename is an added scope beyond the
  source backlog item* — Location: Drafting Notes. The source backlog asks only to
  move the marker under `.accelerator`; this item also renames the basename, flagged
  as a reviewable naming call. The rename is tightly coupled to the move, so keeping
  it in one unit is reasonable; the only risk is a rejected name stalling the
  relocation. Suggestion: confirm the rename is accepted, or defer it if contentious.

### Testability

**Summary**: The Acceptance Criteria are unusually strong for a task: two use
explicit Given/When/Then framing with a named error code
(`E_LOCAL_PERMS_INSECURE`), and a third gives a concrete grep procedure with a
bounded exception. Verification of the core path move and the removal of the old
reference is well specified. The main gaps are that the required documentation
renames have only negative verification, and the preserved security semantics at
the new path lack any confirming criterion.

**Strengths**:
- AC1 and AC2 are precise input-output pairs — each states the env-var
  precondition, the marker file present, the action, and the exact expected outcome.
- AC3 supplies a concrete, runnable verification procedure with an explicitly
  bounded exception for historical `meta/` documents.
- AC2 pins the failure to a named error code rather than a vague "it errors".

**Findings**:
- **minor** (confidence: high) — *New basename rename has only negative
  verification, no positive check* — Location: Acceptance Criteria. The Requirements
  mandate updating the docstring and SKILL.md to name
  `.accelerator/allow-insecure-local`, but the only relevant AC verifies removal of
  the old name, not presence of the correct new name. A doc left mentioning a stale
  or wrong path could pass every criterion; `mise run check` does not validate
  documentation prose. Suggestion: add a positive grep criterion for the new name.
- **suggestion** (confidence: medium) — *Preserved security semantics at the new
  path are unverified* — Location: Acceptance Criteria. The Requirements stress the
  semantics (regular-file, non-symlink, VCS-tracked gate) must stay identical, but
  AC1 exercises only the happy path. No criterion confirms a symlink, untracked, or
  non-regular marker at the new path is still refused, so a relaxed check could pass
  all four criteria — and this is a credential-security check. Suggestion: add at
  least one negative criterion at the new path.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-31

**Verdict:** COMMENT

Re-run of the four lenses that had findings (clarity, dependency, scope,
testability) after applying five edits. Every targeted finding is resolved. The
pass surfaced one new **major** testability gap — no criterion pins each of the
three context builders to the new path — plus minor framing observations, none
crossing the REVISE thresholds (revise-severity `critical`, major-count 2).

### Previously Identified Issues

- 🔵 **Clarity**: "the resolver" not tied to the named function — Resolved (Context
  now glosses `refuse_insecure_personal_config` as "the resolver" on first use).
- 🔵 **Clarity**: "it pairs with" loose referent — Resolved (Summary recast to
  "the environment variable the marker pairs with").
- 🔵 **Dependency**: release-ordering constraint only in prose — Resolved (now an
  explicit Ordering entry under Dependencies; see new issue on its directionality).
- 🔵 **Testability**: new basename rename had only negative verification — Resolved
  (AC5 greps for `allow-insecure-local` present in `credentials.rs` and SKILL.md).
- 🔵 **Testability**: preserved security semantics unverified — Resolved (AC3 adds a
  symlink/untracked negative case at the new path; re-review notes it as an OR that
  could be split).
- 🔵 **Scope**: basename rename is added scope — Still present by design; the user
  confirmed the rename is an intended part of this unit of work.

### New Issues Introduced

- 🟡 **Testability**: No criterion verifies each of the three context builders
  constructs the new path. The added AC5 checks the docstring and SKILL.md, not the
  `jira-cli`/`linear-cli`/`work-cli` builders where the path literal actually
  changes; a builder left at a wrong path (e.g. `.claude/allow-insecure-local`)
  could pass every criterion. Recommend a grep for the new literal across all three
  builder files, or a per-CLI behavioural check.
- 🔵 **Testability**: AC3 conflates the symlink and non-VCS-tracked cases into one
  OR criterion; a verifier could exercise one branch only. Split into two criteria.
- 🔵 **Testability**: AC2 omits the "against a non-`0600` personal config"
  precondition that AC1 and AC3 restate, leaving its expected refusal ambiguous.
- 🔵 **Dependency**: the new Ordering entry is one-directional — it points at a
  release/feature work item that does not yet exist, so nothing enforces the
  constraint from the release side. Create a placeholder and cross-link, or add a
  pre-release check on the feature's own tracking.
- 🔵 **Clarity**: "second half" names one part of the override without naming the
  first in the same sentence; and the fixture crates (`-client`, `tracker-support`)
  do not obviously map to the builder crates (`-cli`), notably `work-cli` →
  `tracker-support`. Both low-confidence phrasing nits.

### Assessment

The work item is stronger than at pass 1 and remains acceptable for implementation
as-is. The one substantive follow-up worth making before planning is the per-builder
path criterion — my earlier AC5 edit verifies the docs but not the three builders
that carry the actual change, so it is the highest-value gap to close. The remaining
items are optional polish.

## Verdict Override (Pass 3) — 2026-08-31

**Verdict:** APPROVE

The pass-2 major finding is closed: a per-builder acceptance criterion now asserts
each of `cli/jira-cli/src/context.rs`, `cli/linear-cli/src/context.rs`, and
`cli/work-cli/src/tracker_registry.rs` constructs `.accelerator/allow-insecure-local`.
The two testability minors are also closed — AC2 restates the non-`0600`
precondition, and the symlink/untracked criterion is split into two independent
checks. The remaining observations (one-directional ordering link, two clarity
phrasing nits) are low-confidence polish and do not block implementation. Reviewer
approved; work item transitioned to `ready`.
