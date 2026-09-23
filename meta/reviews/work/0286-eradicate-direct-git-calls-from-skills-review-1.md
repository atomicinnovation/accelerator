---
type: "work-item-review"
id: "0286-eradicate-direct-git-calls-from-skills-review-1"
title: "Work Item Review: Make Accelerator skills VCS-agnostic by eradicating direct git calls"
date: "2026-09-20T20:46:35+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0286"
work_item_id: "0286"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-20T21:20:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Make Accelerator skills VCS-agnostic by eradicating direct git calls

**Verdict:** REVISE

The work item is unusually well-authored on structure, clarity, and dependency
provenance — every section is present and densely populated, the core "session
VCS" vocabulary is stable, and the sole upstream blocker (spike 0200) is
captured and verified resolved. It falls short on testability: four major
findings show the behavioural acceptance criteria (AC1–AC4) lean on undefined
success language ("gathers evidence successfully", "the intended cumulative
change"), an open-ended token set, and no verification fixtures, while the one
automated gate (`mise run`) never exercises the skills it changes. A fifth
major finding notes the load-bearing SessionStart VCS Command Reference is a
coupling to a component outside the item's stated skills-only scope, yet is
never captured as a dependency.

### Cross-Cutting Themes

- **The SessionStart VCS Command Reference is under-specified** (flagged by:
  clarity, dependency, scope) — The entire approach rests on this reference
  steering the model to the correct backend command, yet it is never located or
  linked (clarity), never captured as a cross-component dependency (dependency),
  and its possible extension via Open Question 3 would silently pull work
  outside the stated skills-only scope (scope). This is the single most
  reinforced issue in the review.
- **Unresolved Open Questions block actionability and verification** (flagged
  by: completeness, testability, scope) — The diff-range and repo-root design
  decisions are deferred to Open Questions, which leaves two Requirements not
  yet actionable (completeness), makes AC2's "intended cumulative change"
  unverifiable against any reference (testability), and leaves a soft scope
  boundary (scope).
- **config/migrate is the weakest-covered in-scope change** (flagged by:
  testability, scope) — It is named as an in-scope fix but has no dedicated
  acceptance criterion, and because its `git status` is non-executing prose,
  AC1's grep clause would not catch it either (testability); it is also one of
  two cosmetic edits bundled with the one real breakage fix (scope).

### Findings

#### Critical

- None.

#### Major

- 🟡 **Testability**: AC1 combines an open-ended token set with a subjective 'session-relative' clause
  **Location**: Acceptance Criteria
  AC1 has two clauses of unequal verifiability: the grep-style sweep is testable, but the token set ends in a trailing ellipsis so the complete pattern set is unknown, and "every VCS operation is phrased in session-relative terms" is a subjective judgement with no defined pass/fail procedure.

- 🟡 **Testability**: 'Gathers evidence successfully' has no defined pass condition (AC2, AC4)
  **Location**: Acceptance Criteria
  AC2 and AC4 both hinge on a skill "gather[ing] evidence successfully" but never define what "successfully" means — no observable outcome, non-error condition, or expected evidence content that would yield a definitive pass or fail for the previously-broken pure-jj case.

- 🟡 **Testability**: AC2's 'intended cumulative change' has no defined reference and depends on an unresolved Open Question
  **Location**: Acceptance Criteria
  AC2 requires validate-plan's diff to render "the intended cumulative change under both git and jj", but there is no expected output, range, or revset to compare against, and Open Questions explicitly leaves the diff-range base unresolved.

- 🟡 **Testability**: Behavioural criteria specify no verification inputs and are not exercised by the automated gate
  **Location**: Acceptance Criteria
  AC2–AC4 verify model-driven Markdown skills yet specify no fixture repo state, commit count, or configured identity, while the only automated gate (AC5, `mise run` exits 0) covers the toolchains' format/lint/test and does not execute the skills.

- 🟡 **Dependency**: Coupling to shared SessionStart VCS Command Reference not captured
  **Location**: Dependencies
  The approach depends on the SessionStart VCS Command Reference — a shared artefact produced outside `skills/` — as its load-bearing mechanism, and Open Question 3 contemplates extending it, yet Dependencies links only work items 0169/0198/0200 and captures no coupling to the component that owns the reference.

#### Minor

- 🔵 **Testability**: config/migrate change has no dedicated verifiable criterion
  **Location**: Acceptance Criteria
  config/migrate is named as one of three in-scope fixes but has no dedicated behavioural criterion; AC1's grep clause cannot catch it because its `git status` is non-executing prose, so this change could be left undone without failing any criterion.

- 🔵 **Dependency**: refine-work-item identity resolution depends on an unplanned session-VCS idiom
  **Location**: Requirements
  AC3 requires refine-work-item to resolve author via "the session VCS user identity", but the only contemplated reference extension (Open Question 3) covers diff-range and repo-root idioms, not user identity — so this requirement leans on a documented idiom that neither exists nor is planned.

- 🔵 **Scope**: Bundles one real breakage fix with two cosmetic consistency edits under one low-priority story
  **Location**: Requirements
  validate-plan genuinely breaks in pure-jj (a real fix), whereas config/migrate is non-executing prose and refine-work-item's fallback "never breaks"; all three are unified under one theme and one `priority: low`, so the fix that matters is priority-masked by two opportunistic cleanups.

- 🔵 **Clarity**: 'The checks step' used as a definite noun phrase without introduction
  **Location**: Requirements
  "The checks step" is used as an already-known referent in Requirements and Open Questions, but no earlier sentence introduces what this step is or why it needs the repository root, so a reader unfamiliar with validate-plan's internals cannot tell which step is meant.

#### Suggestions

- 🔵 **Completeness**: Story lacks an explicit user/beneficiary statement
  **Location**: Summary
  The item is a story but its Summary describes the technical work without the canonical "for whom" statement (present in sibling story 0198), so the beneficiary is inferable from Context but never stated outright.

- 🔵 **Completeness**: Core mechanism of two requirements is deferred to Open Questions
  **Location**: Requirements / Open Questions
  The diff-by-intent and repo-root Requirements state the outcome but leave the mechanism unresolved in Open Questions, so an implementer could not begin those two pieces without first resolving those questions — acceptable for a draft, but they are not yet self-contained.

- 🔵 **Scope**: Open Question 3 leaves a soft boundary that could pull the SessionStart VCS Command Reference into scope
  **Location**: Open Questions
  Answering Open Question 3 "yes" would expand the unit of work from three SKILL.md edits into a shared reference/hooks artefact, a different surface than the enumerated skills, changing what "done" covers.

- 🔵 **Scope**: Concrete work is small; 'story' leans large for the deliverable
  **Location**: Frontmatter: kind
  The substantive work is prose rewrites in three SKILL.md files, with only validate-plan carrying non-trivial design work; this is on the small side for a story and could read as a chore/task — a labelling judgement, not a delivery risk.

- 🔵 **Clarity**: 'SessionStart VCS Command Reference' is load-bearing but never located or linked
  **Location**: Context
  The solution rests on this reference (cited in Context, Open Questions, Assumptions) yet it is never given a file path or link, unlike every other artefact in the item, so the crux of the approach is under-specified for a reader who has not seen it.

- 🔵 **Clarity**: Multiple near-synonyms used for the single core requirement
  **Location**: Summary
  The same requirement is stated as "general terms", "backend-neutral", "VCS-agnostic", and "session-relative terms" across the item; they appear synonymous, but the variation makes a careful reader pause to confirm no distinct requirements are intended.

### Strengths

- ✅ All expected story sections are present and densely populated with
  substantive, non-placeholder content, and the frontmatter is complete and
  valid (kind, status, priority, parent, relates_to, tags all set).
- ✅ Context thoroughly explains the motivation (raw git fails in pure-jj) and
  transparently records why spike 0200's `accelerator vcs diff` CLI proposal
  was rejected, so the item does not silently contradict the document that
  spawned it.
- ✅ Exemplary scope narrowing: the item shrank its own boundary from a new CLI
  subcommand to prose-only edits and dropped the `cli` tag accordingly;
  speculative expansions are parked as triggered Assumptions, not folded into
  current scope.
- ✅ Consistent core "session VCS" vocabulary, and the item preempts two
  apparent contradictions (why `git rev-parse` breaks in pure-jj while
  `git config` does not; how "eradicate every git token" reconciles with
  "preserve the git-only fallback").
- ✅ The sole upstream blocker is captured and verified resolved with a clean
  audit trail; related items 0169/0198/0200 are linked with per-relationship
  rationale and correctly typed as relates-to rather than blockers.
- ✅ AC5 (`mise run` exits 0) is a fully testable gate, and Technical Notes and
  References give exact file:line targets for every affected skill.

### Recommended Changes

1. **Rewrite AC1–AC4 with concrete, closed pass conditions** (addresses:
   AC1 open-ended token set, "gathers evidence successfully", "intended
   cumulative change", no verification inputs) — Replace AC1's ellipsis with
   the exact closed set of git subcommand tokens to sweep for and turn the
   qualitative clause into a reviewer check. For AC2/AC4, define the observable
   pass condition per repo mode (e.g. "emits a non-empty recent-commit list and
   an implementation diff, with no `fatal: not a git repository`, under each of
   git-only, colocated, and pure-jj"). Specify the fixture preconditions
   (repo mode, number of implementation commits, configured VCS identity) so
   the manual verification procedure is repeatable.
2. **Resolve the diff-range and repo-root Open Questions before leaving draft**
   (addresses: AC2 unresolved reference, deferred requirement mechanism,
   soft scope boundary) — Decide the diff base (trunk divergence, bookmark, or
   plan's first commit) and the repo-root approach, then state the expected
   cumulative-change reference so AC2 can be phrased against it and the two
   deferred Requirements become self-contained.
3. **Capture the SessionStart VCS Command Reference as an explicit dependency**
   (addresses: uncaptured coupling, reference never located, Open Question 3
   scope boundary, unplanned identity idiom) — Add the reference as a coupling
   in Dependencies with a file/link on first mention. Decide before planning
   whether extending it (Open Question 3) is in scope or spun into a follow-up,
   and if extending, cover the user-identity idiom AC3 relies on — or record
   that identity resolution is out of the reference's scope.
4. **Add a dedicated acceptance criterion for config/migrate** (addresses:
   config/migrate uncovered, bundled cosmetic edit) — e.g. "config/migrate's
   dirty-path confirmation guidance names the session VCS status command and
   contains no literal `git status`", since AC1's grep clause cannot catch
   non-executing prose. Optionally confirm the plugin-wide sweep invariant is
   the intended single deliverable that justifies bundling the one breakage fix
   with the two cosmetic rewords.
5. **Polish clarity nits** (addresses: "the checks step", near-synonyms,
   missing beneficiary) — Introduce "the checks step" on first mention,
   standardise on one phrase for the VCS-agnostic concept, and optionally add a
   one-line user-story beneficiary statement to the Summary.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Work item 0286 is unusually clear and internally consistent: it
uses a single, stable vocabulary for its core concept (the "session VCS"),
consistently distinguishes the three modes (git-only, colocated jj, pure-jj)
and the three affected skills, and actively preempts apparent contradictions.
The few clarity gaps are minor: one definite noun phrase ("the checks step") is
used without introduction, and the central named artefact the solution rests on
("SessionStart VCS Command Reference") is never located or linked, unlike every
other artefact in the item. No critical or major ambiguities or contradictions
were found.

**Strengths**:
- Consistent core vocabulary: "the session VCS" and its variants resolve to one
  stable, unambiguous concept across all sections.
- The three affected skills and the verify-only skill are named explicitly and
  used identically in every section, so "all three" and "others" resolve
  without ambiguity.
- The item preempts two apparent internal contradictions (why `git rev-parse`
  breaks in pure-jj while `git config` does not; reconciling "eradicate every
  git token" with "preserve the git-only fallback").
- The scope tension between "Rewrite all three" and the whole-directory sweep
  in AC1 is coherent, not contradictory.
- The departure from spike 0200's CLI recommendation is handled transparently.

**Findings**:
- **minor / medium** — *'The checks step' used as a definite noun phrase without
  introduction* (Requirements): "the checks step" is used as an already-known
  referent, but no earlier sentence introduces what this step is or why it needs
  the repo root; a reader unfamiliar with validate-plan's internals cannot tell
  which step is meant. Suggestion: identify the step on first mention.
- **suggestion / medium** — *'SessionStart VCS Command Reference' is load-bearing
  but never located or linked* (Context): the solution rests on this reference
  but it is never given a file path or link, unlike every other cited artefact.
  Suggestion: add a file/link reference on first mention.
- **suggestion / low** — *Multiple near-synonyms used for the single core
  requirement* (Summary): "general terms", "backend-neutral", "VCS-agnostic",
  "session-relative terms" appear synonymous but the variation adds interpretive
  load. Suggestion: standardise on one phrase.

### Completeness

**Summary**: Work item 0286 is an unusually complete story: every expected
section is present and substantively populated, frontmatter is fully valid, and
the Context richly explains why the work is needed and why the previously-planned
CLI approach was rejected. The two completeness observations are both minor: the
Summary omits the canonical story "for whom" statement, and the core mechanism
of two Requirements is deliberately deferred to Open Questions. Neither is a
missing-section gap, and both are appropriate for a draft.

**Strengths**:
- All expected story sections are present and densely populated with
  substantive, non-placeholder content.
- Frontmatter is complete and valid: kind, status, priority, parent,
  relates_to, and tags are all set.
- Context thoroughly explains the motivation and records why the
  `accelerator vcs diff` CLI proposal was rejected.
- Five specific acceptance criteria are provided, each Requirement maps onto at
  least one criterion.
- Open Questions and Assumptions are genuinely populated with unresolved design
  matters and their fallback positions.

**Findings**:
- **suggestion / medium** — *Story lacks an explicit user/beneficiary statement*
  (Summary): the Summary describes the technical work without the canonical
  "for whom" in user-story form. Suggestion: add a one-line beneficiary
  statement.
- **suggestion / low** — *Core mechanism of two requirements is deferred to Open
  Questions* (Requirements / Open Questions): the diff-by-intent and repo-root
  Requirements leave the mechanism unresolved in Open Questions, so an
  implementer could not begin without resolving those questions. Acceptable for
  a draft; resolve before leaving draft.

### Dependency

**Summary**: From the dependency lens, 0286 is well-mapped on the work-item
axis: its single upstream blocker (spike 0200) is explicitly captured and
verified resolved, related items are linked with clear per-relationship
rationale, and there are no uncaptured downstream consumers. The one real gap is
the coupling to the shared SessionStart VCS Command Reference — the load-bearing
mechanism the whole approach depends on — which is discussed in the body but not
captured as a dependency, and whose possible extension (Open Question 3) would
pull work into a component outside the item's stated skills-only scope.

**Strengths**:
- The sole upstream blocker is explicitly captured and verified resolved, with a
  clean audit trail (Dependencies + 0200's `status: done` + Drafting Notes).
- All three related items are linked with a one-line rationale each and correctly
  typed as relates-to rather than blockers.
- The item distinguishes the runtime, model-driven data-gathering from the
  `!`-injection precedent, clarifying why 0198 is a relates-to and not a runtime
  dependency.
- No uncaptured downstream consumers: research-issue is a verification-only
  regression target, not a dependant.

**Findings**:
- **major / medium** — *Coupling to shared SessionStart VCS Command Reference not
  captured* (Dependencies): the approach depends on the reference (a shared
  artefact outside `skills/`) and Open Question 3 contemplates extending it, yet
  Dependencies captures no coupling to the component that owns it. Impact: if the
  extension route is chosen, work silently grows beyond the stated scope, and the
  extension would need to land before the dependent skill rewrites — a sequencing
  coupling invisible at planning time. Suggestion: capture the reference as an
  explicit coupling and, if extending, link or spawn a work item that must
  precede the rewrites.
- **minor / medium** — *refine-work-item identity resolution depends on an
  unplanned session-VCS idiom* (Requirements): AC3 requires identity resolution
  via "the session VCS user identity", but the only contemplated reference
  extension covers diff-range and repo-root idioms, not identity. Suggestion:
  name identity as an additional idiom the reference should cover, or record that
  it is out of the reference's scope.

### Scope

**Summary**: This is a coherent, well-bounded story confined to a single
component (`skills/`) with genuinely strong scope discipline — it deliberately
narrowed from the CLI-expansion approach 0200 recommended to prose-only edits,
and dropped the `cli` tag accordingly. Its Summary, Requirements, and Acceptance
Criteria describe the same scope, and out-of-scope contingencies are parked as
triggered assumptions rather than baked in. The main scope observation is that
it bundles one genuine functional breakage fix with two cosmetic consistency
edits under a single "low" priority; a secondary soft boundary exists where Open
Question 3 could pull the shared SessionStart VCS Command Reference into scope.

**Strengths**:
- Exemplary scope narrowing: reframed away from 0200's `accelerator vcs diff`
  CLI subcommand to a prose-only fix, shrinking rather than expanding its own
  boundary.
- Clear in-scope/out-of-scope statements: research-issue is verify-only,
  refine-work-item's fallback is preserved (reframed, not deleted), speculative
  expansions are parked as triggered Assumptions.
- Single theme, single component, single ownership domain — no cross-service or
  cross-team spread, explicitly "no new CLI".
- Summary, Requirements, and Acceptance Criteria are mutually consistent about
  which skills are in scope.

**Findings**:
- **minor / medium** — *Bundles one real breakage fix with two cosmetic
  consistency edits under one low-priority story* (Requirements): validate-plan
  genuinely breaks in pure-jj, whereas config/migrate is non-executing prose and
  refine-work-item's fallback "never breaks"; all three carry one `priority: low`,
  so the fix that matters is priority-masked. Suggestion: confirm AC1's sweep
  invariant justifies bundling, or split validate-plan's fix out at higher
  priority.
- **suggestion / medium** — *Open Question 3 leaves a soft boundary that could
  pull the SessionStart VCS Command Reference into scope* (Open Questions):
  answering "yes" would expand the unit of work into a shared reference/hooks
  artefact. Suggestion: decide before planning whether the extension is in scope.
- **suggestion / low** — *Concrete work is small; 'story' leans large for the
  deliverable* (Frontmatter: kind): the substantive work is prose rewrites in
  three files; on the small side for a story. Suggestion: optionally reconsider
  against team chore/task norms.

### Testability

**Summary**: This story mixes one fully testable gate (AC5, `mise run` exits 0)
with behavioural criteria that lean on undefined success language ("gathers
evidence successfully", "renders the intended cumulative change") and unbounded
scope ("every VCS operation", a trailing ellipsis in the token list). Because
the skills under change are model-driven Markdown, the behavioural criteria
(AC2–AC4) need explicit input fixtures and defined pass conditions that are
currently absent, and the sole automated gate does not exercise them.
config/migrate, named as an in-scope fix, has no dedicated verifiable criterion.

**Strengths**:
- AC5 ("`mise run` exits 0 end-to-end") is a fully testable criterion — a
  defined command with a defined exit-code outcome.
- AC4 explicitly frames research-issue as a regression/verification target with
  no code change expected.
- AC3 names both backends and ties the git-identity fallback to observable
  session-VCS behaviour, giving a concrete comparison.
- Technical Notes and References give exact file:line targets for every affected
  skill.

**Findings**:
- **major / high** — *AC1 combines an open-ended token set with a subjective
  'session-relative' clause* (Acceptance Criteria): the token set ends in a
  trailing ellipsis so the complete pattern set is unknown, and "every VCS
  operation is phrased in session-relative terms" has no defined pass/fail
  procedure. Suggestion: state the exact closed token set and turn the
  qualitative clause into a concrete reviewer check.
- **major / high** — *'Gathers evidence successfully' has no defined pass
  condition (AC2, AC4)* (Acceptance Criteria): "successfully" is never defined —
  no observable outcome, non-error condition, or expected evidence content.
  Suggestion: define the observable pass condition per repo mode (e.g. non-empty
  commit list and diff, no `fatal: not a git repository`).
- **major / high** — *AC2's 'intended cumulative change' has no defined reference
  and depends on an unresolved Open Question* (Acceptance Criteria): no expected
  output, range, or revset to compare against, and Open Questions leaves the
  diff-range base unresolved. Suggestion: resolve the Open Question and state the
  expected cumulative-change reference.
- **major / medium** — *Behavioural criteria specify no verification inputs and
  are not exercised by the automated gate* (Acceptance Criteria): AC2–AC4 specify
  no fixture repo state, commit count, or configured identity, while AC5 does not
  execute the skills. Suggestion: specify the concrete precondition and observable
  output per criterion.
- **minor / medium** — *config/migrate change has no dedicated verifiable
  criterion* (Acceptance Criteria): only validate-plan (AC2) and refine-work-item
  (AC3) have dedicated criteria; config/migrate's reword is covered only by AC1's
  subjective clause, and AC1's grep clause would not catch non-executing prose.
  Suggestion: add a dedicated criterion for config/migrate.

## Re-Review (Pass 2) — 2026-09-20T21:13:34+00:00

**Verdict:** COMMENT

The revision resolved every review-1 major (four testability, one dependency)
and the key minors; the verdict improves from REVISE to COMMENT. The scope
expansion agreed during iteration — exposing the existing `repository_root` as
`accelerator vcs root`, anchoring the diff on the trunk divergence point, and
extending the SessionStart VCS Command Reference — closed the testability and
dependency gaps at their root. One new major remains: the reference-extension
deliverable has no acceptance criterion that verifies it directly.

### Previously Identified Issues

- 🔵 **Clarity**: "The checks step" used without introduction — Still present
- 🔵 **Clarity**: SessionStart reference never located or linked — Resolved
- 🔵 **Clarity**: Near-synonyms for the core requirement — Still present
- 🔵 **Completeness**: Story lacks a beneficiary statement — Resolved
- 🔵 **Completeness**: Core mechanism deferred to Open Questions — Resolved
- 🟡 **Dependency**: SessionStart reference coupling not captured — Resolved
- 🔵 **Dependency**: refine-work-item identity idiom unplanned — Resolved
- 🔵 **Scope**: Bundles the breakage fix with cosmetic edits under `low`
  priority — Still present
- 🔵 **Scope**: Open Question 3 soft boundary — Resolved
- 🔵 **Scope**: "story" leans large for the deliverable — Resolved (the CLI +
  reference work now fits a story)
- 🟡 **Testability**: AC1 open-ended token set + subjective clause — Resolved
- 🟡 **Testability**: "Gathers evidence successfully" has no pass condition —
  Resolved
- 🟡 **Testability**: AC2 "intended cumulative change" has no reference —
  Resolved
- 🟡 **Testability**: Behavioural criteria specify no inputs, not exercised by
  the gate — Resolved
- 🔵 **Testability**: config/migrate has no dedicated criterion — Resolved

### New Issues Introduced

- 🟡 **Testability**: No criterion verifies the SessionStart VCS Command
  Reference extension (also flagged by completeness at lower confidence) — the
  load-bearing diff-range and identity idioms could ship incomplete without
  failing any criterion, since AC3/AC4/AC6 only exercise skills that consume
  the reference, which a model-driven skill could satisfy by chance.
- 🔵 **Clarity**: Ambiguous whether refine-work-item keeps its jj-first chain
  or collapses to a single session-identity lookup.
- 🔵 **Testability**: AC6's jj-repository sub-case names no fixture or expected
  resolved author (only the git-only case is specified).
- 🔵 **Clarity**: Summary's "fail in pure-jj" generalisation overstates for
  config/migrate and refine-work-item, which never break.
- 🔵 **Testability**: AC4's exact jj revset baseline and colocated diff coverage
  are underspecified (equality checked under "both git and jj" while AC3
  enumerates three modes).
- 🔵 **Dependency**: The land-order constraint is captured in prose only, not a
  structured edge — harmless while the item lands atomically, a latent blocker
  only if the bundle is later split.

### Assessment

The item is now acceptable and close to plan-ready. Closing the one new major —
add an acceptance criterion asserting `accelerator vcs detect --descriptive`
carries both idioms with their git and jj renderings — plus tightening AC6's jj
fixture and AC4's jj revset and colocated coverage would take it to a clean
state. The remaining nits (checks-step introduction, near-synonyms, Summary
overstatement, refine-work-item resolution structure) are low-cost prose fixes.
The `low` priority on a bundle that now carries a real breakage fix and a CLI
addition is a standing judgement for the team, not a blocker.

## Approval — 2026-09-20T21:20:01+00:00

**Verdict:** APPROVE

The pass-2 findings were addressed after the re-review: an acceptance criterion
now verifies the SessionStart VCS Command Reference extension directly (closing
the one new major), AC4 spells out the exact jj revset and extends diff-equality
to all three modes, AC6 names both fixtures and expected authors, and the four
clarity nits (checks-step introduction, Summary overstatement,
refine-work-item resolution structure, standardised terminology) are fixed. No
open critical or major findings remain. The work item is approved and its status
moved to `ready`.

Two standing judgement calls are accepted as-is, not blockers:

- 🔵 **Scope**: the breakage fix, two rewords, the `vcs root` command, and the
  reference extension stay bundled under `priority: low`. The bundling is
  coherent (the "no direct git in `skills/`" invariant is the unit of value);
  the priority is left to the team.
- 🔵 **Dependency**: the "reference + `vcs root` before the skill rewrites"
  land-order is prose only. It becomes a `blocked_by` edge only if the bundle is
  split during planning.
