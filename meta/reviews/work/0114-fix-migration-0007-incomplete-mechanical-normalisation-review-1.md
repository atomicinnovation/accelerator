---
type: work-item-review
id: "0114-fix-migration-0007-incomplete-mechanical-normalisation-review-1"
title: "Work Item Review: Migration 0007 Incomplete Mechanical Normalisation"
date: "2026-06-17T21:24:56+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0114"
work_item_id: "0114"
reviewer: Toby Clemson
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-06-17T21:34:59+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Migration 0007 Incomplete Mechanical Normalisation

**Verdict:** COMMENT

This is an exceptionally complete and internally consistent bug work item: all
structural sections are present and densely populated, the reproduction /
actual / expected triad is fully captured, frontmatter is valid, and the
five-to-six gaps trace cleanly from Summary through Requirements to Acceptance
Criteria. The findings are observations rather than blockers — the dominant
theme is that several verification criteria lean on an undefined "full" or
"representative" corpus and on a "four reproduction shapes" count that does not
line up with the six gaps/criteria, which together risk a fixture set that
re-creates the original blind spot. The item is acceptable as-is, but tightening
the fixture inventory and corpus definitions would materially strengthen it.

### Cross-Cutting Themes

- **Reproduction-shape count vs. gap/criterion count mismatch** (flagged by:
  testability, completeness, clarity) — Requirements list *four* reproduction
  shapes while the Gaps-to-Close list and Acceptance Criteria cover *six*
  transforms (title-promotion, `ticket`/`ticket_id` drop, `topic` backfill,
  `meta/docs/` skip are not among the four). "All four reproduction shapes" is
  therefore an ambiguous fixture target, the `meta/docs/`/`INVALID-TYPE` trigger
  is missing from the reproduction list entirely, and a skimming reader can
  under-count the work.
- **Undefined verification corpus** (flagged by: testability ×3) — Three
  criteria depend on "the full corpus" or "a representative corpus" with no
  fixed referent. The RCA notes the dogfood corpus lacks the very shapes that
  trigger the bug, so a literal reading of these criteria can pass while proving
  nothing about the failing shapes.
- **Triplicated type-inference coupling lives only in Technical Notes** (flagged
  by: dependency, clarity) — The lockstep co-change across the migration,
  validator, and awk encodings is the single most important implementation
  coupling, but it is captured in prose rather than surfaced in Dependencies,
  and gap 1's "both copies" wording undercounts the third (awk) site.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: "All four reproduction shapes" is unbounded and its referent is ambiguous
  **Location**: Acceptance Criteria (test-migrate-0007.sh fixtures)
  The criterion uses "all four" but Requirements list four reproduction shapes
  while the Gaps-to-Close list six transforms; a verifier cannot tell whether
  fixtures for title-promotion, the `meta/docs/` skip, or the `topic` backfill
  are required, so the criterion can be claimed met while leaving exactly the
  coverage gaps that caused this bug.

#### Minor

- 🔵 **Clarity**: Validator violation codes used without definition
  **Location**: Summary
  Codes like `INVALID-TYPE`, `FORBIDDEN-OWN-ID`, `MISSING-EXTRA`,
  `BAD-LINKAGE-SHAPE` are referenced throughout but only defined in the linked
  RCA, weakening the item's ability to stand alone.

- 🔵 **Clarity**: Mixed "PR" referents — pr-description vs pr-review vs PR #N linkage
  **Location**: Context
  "PR" is overloaded across four distinct referents (`pr-description` in
  `meta/prs/`, `pr-review` in `meta/reviews/prs/`, the forbidden keys, and the
  `PR #N` linkage shape); a reader can conflate the two file types and
  misdirect the path-inference fix.

- 🔵 **Clarity**: Schema-column and function-name jargon assumed known
  **Location**: Requirements
  `forbidden_own_id_key`, `path_to_typed`, `infer_type_from_path`,
  `extras_for_type`, `FM_OPTIONAL_EXTRAS`, etc. are used in Requirements before
  being grounded in Technical Notes; a forward-pointer would help.

- 🔵 **Completeness**: meta/docs/ INVALID-TYPE trigger absent from the enumerated reproduction shapes
  **Location**: Requirements: Reproduction
  The `meta/docs/` case is the largest contributor to the original failure (per
  the RCA's violation breakdown) and has its own gap and acceptance criterion,
  yet it is not among the four enumerated reproduction shapes, so a repro corpus
  built from the list would omit it.

- 🔵 **Dependency**: Cross-file lockstep coupling captured in Technical Notes but absent from Dependencies
  **Location**: Dependencies
  `infer_type_from_path` is triplicated across migration, validator, and awk and
  must change in lockstep, but the Dependencies section lists only inter-item
  links; an implementer reading it in isolation would miss the co-change
  requirement that, if violated, re-introduces this exact bug.

- 🔵 **Dependency**: Schema TSV columns are an unnamed upstream precondition
  **Location**: Assumptions
  The fix is schema-driven (reads `forbidden_own_id_key` and derives required
  extras), assuming the TSV already declares these for `pr-description`/
  `pr-review`; if it did not, the transforms would silently no-op. This
  precondition is only implicit in Technical Notes.

- 🔵 **Testability**: "Driven by the schema column rather than a hard-coded list" is an implementation detail
  **Location**: Acceptance Criteria (forbidden own-id keys, schema-driven)
  A black-box test cannot distinguish a schema-driven drop from a hard-coded
  one; the clause specifies a mechanism rather than an observable behaviour.

- 🔵 **Testability**: "Representative corpus" for the regression guard is undefined
  **Location**: Acceptance Criteria (regression guard / validator-clean invariant)
  "Representative" is not pinned to a defined input set, so a guard over a thin
  corpus could pass while the bug-triggering shapes remain unexercised.

- 🔵 **Testability**: meta/docs/ skip criterion checks one violation code but not the broader "skipped" outcome
  **Location**: Acceptance Criteria (meta/docs/ skip)
  "Skipped as out of scope" also implies the file is left unmutated, which the
  no-`INVALID-TYPE` check alone would not catch.

- 🔵 **Testability**: End-to-end criterion depends on an unbounded "full corpus"
  **Location**: Acceptance Criteria (corpus-wide validator passes)
  "The full corpus" has no fixed referent; passing on the dogfood corpus (which
  lacks the triggering shapes) would satisfy the wording while proving nothing.

#### Suggestions

- 🔵 **Clarity**: "review_number is set to 1" vs "defaults to 1" phrased two ways
  **Location**: Acceptance Criteria
  Slightly different framings leave it momentarily unclear whether `1` is
  unconditional or applied only when absent.

- 🔵 **Clarity**: "both infer_type_from_path copies" vs "triplicated" inference
  **Location**: Requirements
  Gap 1 says "both copies" plus `path_to_typed`, while a later note says
  inference is "triplicated"; naming all three sites at the point of instruction
  removes the ambiguity.

- 🔵 **Completeness**: Environment/version details for the bug repro live in Context rather than the Reproduction block
  **Location**: Requirements: Reproduction
  A one-line environment preamble (plugin version + prerequisite migrations)
  would make the repro self-contained.

- 🔵 **Dependency**: Prior-migration coupling to 0001's ticket handling not named
  **Location**: Requirements: Gaps to close
  Gap 3's unconditional `ticket`/`ticket_id` drop overlaps 0001's scoped
  `ticket_id`→`work_item_id` rename inside `meta/tickets/`; noting the
  complementary ordering pre-empts second-guessing.

- 🔵 **Scope**: Prevention/regression-guard work folded in as one unit
  **Location**: Acceptance Criteria
  Single-sourcing the triplicated inference is arguably an independent refactor;
  confirm in Drafting Notes that it ships as one unit, or note it as a
  fast-follow so the high-priority unblock is not gated on it.

- 🔵 **Scope**: meta/docs/ out-of-scope change is a distinct concern sharing the fix window
  **Location**: Requirements: Gaps to close (item 6)
  No split needed — co-delivery is defensible — but optionally note the
  `out_of_scope` decision is a deliberate policy choice so a future reader does
  not mistake it for scope creep.

### Strengths

- ✅ Frontmatter is fully populated and valid (`kind: bug`, status, priority, id,
  title, parent, relates_to, source, tags, schema_version) — no placeholders.
- ✅ Scope is consistent across sections: the Summary's "five required
  transforms" map 1:1 onto the Gaps-to-Close list, the Acceptance Criteria, and
  the RCA's root causes, with no contradiction.
- ✅ Bug-specific content is complete — Reproduction / Actual behaviour /
  Expected behaviour triad is fully captured, with literal example inputs.
- ✅ Acceptance Criteria are numerous (eleven) and mostly Given/When/Then with
  concrete literal values, each mapping to a closeable gap.
- ✅ The "correct behaviour" boundary (structural gate aborting before
  `harness_run`) and out-of-scope items (gate reordering, 0103, 0105) are stated
  unambiguously, pre-empting false dependency and scope assumptions.
- ✅ The cross-file triplication coupling and the latent `extras_for_type`
  off-by-one are identified precisely in Technical Notes, giving the implementer
  the traps up front.
- ✅ The final corpus-wide invariant (validator exits 0, `harness_run` reached)
  is a strong, mechanically-checkable end-to-end criterion.

### Recommended Changes

1. **Enumerate the required fixture set explicitly** (addresses: "All four
   reproduction shapes" is unbounded; meta/docs/ trigger absent; "full corpus"
   unbounded; "representative corpus" undefined) — Replace "all four
   reproduction shapes" with a closed list of required fixtures, one per
   acceptance outcome: empty-`type:` `meta/prs/` file, `ticket:`-no-`topic:`
   note, `pr_title:`+`review_pass:` pr-review, `pr_title:` with and without an
   existing `title:`, fenced pr-review with filename-encoded PR number,
   `target: "PR #416"` linkage, and a `meta/docs/` file. Bind the corpus-wide
   and regression-guard criteria to this same fixture corpus so "full" and
   "representative" have a fixed referent containing the triggering shapes.

2. **Add a fifth reproduction shape for meta/docs/** (addresses: meta/docs/
   trigger absent) — Add a `meta/docs/...` example to the Reproduction block
   that triggers an `INVALID-TYPE` violation, mirroring the existing acceptance
   criterion and gap 6.

3. **Make the meta/docs/ skip outcome fully observable** (addresses: skip
   criterion checks one code only) — Add an unmutated-content assertion ("the
   file's bytes are unchanged by the migration") so "skipped" is measurable
   beyond the absence of `INVALID-TYPE`.

4. **Reframe the schema-driven criterion behaviourally** (addresses: "driven by
   the schema column" is an implementation detail) — Add a fixture exercising
   the schema-driven path (a forbidden key declared only in the TSV, absent from
   any hard-coded list, is dropped) so the property is observable.

5. **Surface the lockstep coupling and schema precondition in Dependencies/
   Assumptions** (addresses: lockstep coupling absent from Dependencies; schema
   TSV unnamed precondition; 0001 ticket coupling) — Add an internal-coupling
   note to Dependencies ("`infer_type_from_path` triplicated — all three sites
   change together or single-source") and an assumption that the schema TSV
   already declares `forbidden_own_id_key` and the required extras; optionally
   note 0007's unconditional `ticket`/`ticket_id` drop is complementary to
   0001's scoped rename.

6. **Tidy clarity nits** (addresses: violation codes undefined; PR referents
   mixed; jargon assumed known; review_number phrasing; "both copies" vs
   triplicated) — Gloss the validator violation codes on first use, add a
   one-line `pr-description` vs `pr-review` directory orientation, forward-point
   to Technical Notes for named functions/columns, state once that
   `review_number` is backfilled to `1` only when absent, and name all three
   inference sites in gap 1.

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually clear and internally consistent: Summary,
Context, Requirements, and Acceptance Criteria all describe the same five-gap
mechanical-normalisation fix, with each criterion traceable to a numbered gap.
The main clarity risks are domain-heavy jargon and acronyms (validator violation
codes, schema column names, function names) used without in-document definition,
which a reader who has not read the RCA must resolve by following links, and a
couple of referent shifts around 'PR description' file location and the meaning
of 'the validator copy'. None of these rises to a contradiction; they are
definitional gaps for an outside reader.

**Strengths**:
- Scope is consistent across sections: the Summary's "five required transforms
  are missing" is mirrored 1:1 by the six "Gaps to close", the Acceptance
  Criteria, and the RCA's five root causes plus the `meta/docs/` secondary.
- Actors and triggers are explicit throughout: Acceptance Criteria use the
  Given/When(0007 runs)/Then form, naming the migration as the actor.
- The "correct behaviour" boundary (structural gate aborting before
  `harness_run`) is stated unambiguously and consistently.

**Findings**:
- 🔵 minor (high): **Validator violation codes used without definition**
  (Summary) — Codes `INVALID-TYPE`, `FORBIDDEN-OWN-ID`, `OBSOLETE-LEGACY-KEY`,
  `MISSING-EXTRA`, `BAD-LINKAGE-SHAPE` are used across the item but only defined
  in the linked RCA's violation-breakdown table. Gloss them on first use.
- 🔵 minor (medium): **Mixed "PR" referents** (Context) — `meta/prs/`
  pr-descriptions, `pr-review` files at `meta/reviews/prs`, the forbidden keys,
  and the `PR #N` linkage shape are four distinct PR referents. Add a one-line
  directory orientation and use canonical type names.
- 🔵 minor (medium): **Schema-column and function-name jargon assumed known**
  (Requirements) — `forbidden_own_id_key`, `path_to_typed`,
  `infer_type_from_path`, `extras_for_type`, `FM_OPTIONAL_EXTRAS`,
  `normalize_paths`/`normalize_bare`, `out_of_scope` are used in Requirements
  before Technical Notes grounds them. Add a forward-pointer.
- 🔵 suggestion (medium): **"review_number is set to 1" vs "defaults to 1"**
  (Acceptance Criteria) — Phrased as both a fixed set and a default; state once
  that it is backfilled only when absent.
- 🔵 suggestion (low): **"both infer_type_from_path copies" vs "triplicated"**
  (Requirements) — Gap 1's "both copies" plus `path_to_typed` reads as two
  sites; name all three together.

### Completeness

**Summary**: This is an exceptionally complete bug work item. All structural
sections (Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes) are present and densely populated;
the bug-specific reproduction/actual/expected triad is fully captured; and
frontmatter is complete with a valid `kind: bug`. The only completeness gaps
worth noting are minor: the four enumerated reproduction shapes do not include
the `meta/docs/` / `INVALID-TYPE` scenario as a reproduction step despite it
being a documented trigger, and the bug's reproduction lacks explicit
environment/version capture in the dedicated Reproduction block.

**Strengths**:
- Frontmatter fully populated and valid (kind=bug, status, priority, id, title,
  parent, relates_to, source, tags, schema_version).
- Summary is a single unambiguous statement of the defect.
- Bug-specific content complete: Reproduction / Actual / Expected triad present.
- Acceptance Criteria are eleven concrete Given/When/Then bullets.
- Context explains the forces rather than restating the summary, and scopes out
  gate reordering.
- Open Questions, Dependencies, Assumptions, Technical Notes all carry genuine
  content; Open Questions records all three resolved.

**Findings**:
- 🔵 minor (medium): **meta/docs/ INVALID-TYPE trigger absent from the
  enumerated reproduction shapes** (Requirements: Reproduction) — The
  `meta/docs/` case is the largest contributor to the original failure (RCA: 76
  of 197 violations from `INVALID-TYPE`) and has its own gap and criterion, yet
  is not among the four reproduction shapes. Add a fifth shape.
- 🔵 suggestion (low): **Environment/version details live in Context rather than
  the Reproduction block** (Requirements: Reproduction) — Add a one-line
  environment preamble (plugin version + prerequisite migrations) for a
  self-contained repro.

### Dependency

**Summary**: As a bug with a self-contained fix confined to the migrate
subsystem plus the corpus validator, this work item maps its couplings unusually
well: the Dependencies section explicitly states no upstream blocker, names the
downstream consumer (the 1.23 migration for affected repos), and links the
related/parent items (0070, 0057). The strongest dependency signal — that the
same logic is triplicated across the migration, the validator, and the awk and
must change in lockstep — is captured in Technical Notes rather than as a
coupling note in Dependencies, and two upstream preconditions (the schema TSV's
forbidden_own_id_key/extras columns, and migration 0001's prior ticket handling)
are assumed-present but never named as prerequisites. These are minor capture
gaps, not unscheduled blockers.

**Strengths**:
- Dependencies explicit on all three axes: "Blocked by: none", a named
  downstream consumer, and related/parent links (0070, 0057).
- The triplication co-change coupling is identified precisely in Technical Notes.
- Out-of-scope adjacent efforts (0103, 0105) named as separate done items.
- The blocking relationship is correctly scoped, tying high priority to concrete
  downstream impact.

**Findings**:
- 🔵 minor (medium): **Cross-file lockstep coupling captured in Technical Notes
  but absent from Dependencies** (Dependencies) — `infer_type_from_path` is
  triplicated; an implementer reading Dependencies in isolation will miss the
  co-change requirement. Add a coupling note.
- 🔵 minor (medium): **Schema TSV columns are an unnamed upstream precondition**
  (Assumptions) — The schema-driven fix assumes the TSV declares
  `forbidden_own_id_key` and required extras for `pr-description`/`pr-review`; if
  not, transforms silently no-op. Make the precondition explicit.
- 🔵 suggestion (low): **Prior-migration coupling to 0001's ticket handling not
  named** (Requirements: Gaps to close) — Gap 3's unconditional drop overlaps
  0001's scoped `ticket_id`→`work_item_id` rename; note the complementary
  ordering.

### Scope

**Summary**: This bug work item is well-scoped and coherent: all six 'gaps to
close' serve one unified purpose — making migration 0007's mechanical passes
produce a validator-clean corpus by construction, so the structural gate stops
permanently blocking the upgrade. The work is bounded to the migrate subsystem
plus its validator, with no cross-component reach, and the declared `bug` kind
and `M` size estimate fit the described scope. The only scope-relevant questions
concern whether the prevention/regression-guard work and the secondary
`meta/docs/` change ride along as one unit versus standing alone.

**Strengths**:
- All requirements trace to a single root cause and collectively deliver one
  unit of value (the "validator-clean by construction" invariant).
- Summary, Requirements, and Acceptance Criteria describe the same scope
  consistently; gaps map 1:1 onto acceptance bullets.
- Scope boundaries stated explicitly and defensibly (gate reordering, 0103, 0105
  out of scope; no cross-component reach).
- Declared `bug` kind and `M` size are appropriate.

**Findings**:
- 🔵 suggestion (medium): **Prevention/regression-guard work folded in as one
  unit** (Acceptance Criteria) — Single-sourcing the triplicated inference is
  arguably an independent refactor; confirm in Drafting Notes it ships as one
  unit, or note as a fast-follow so the unblock is not gated on it.
- 🔵 suggestion (low): **meta/docs/ out-of-scope change is a distinct concern
  sharing the fix window** (Requirements: Gaps to close item 6) — No split
  needed; optionally note the `out_of_scope` decision is deliberate policy, not
  scope creep.

### Testability

**Summary**: This bug specification is unusually well-constructed for
testability: every reproduction shape names an exact input, the Acceptance
Criteria are almost all framed as Given/When/Then with concrete literal values,
and the final invariant (validator exits 0, harness_run reached) gives a
definitive corpus-wide pass/fail. The main gaps are a few criteria whose
expected outcome relies on the validator's verdict without an enumerated check,
one criterion using unbounded 'all four reproduction shapes' language whose
referent is ambiguous, and a missing verification path for the meta/docs/
out-of-scope behaviour and the title-promotion branch.

**Strengths**:
- The Reproduction section enumerates four exact triggering input shapes with
  literal example values.
- Most Acceptance Criteria use explicit Given/When/Then framing with concrete
  literals.
- The final corpus-wide criterion is a strong, mechanically-checkable end-to-end
  invariant.
- Bug structure is complete for verification — distinct Actual and Expected
  behaviour sections.

**Findings**:
- 🟡 major (high): **"All four reproduction shapes" is unbounded and its referent
  is ambiguous** (Acceptance Criteria — test-migrate-0007.sh fixtures) —
  Requirements list four reproduction shapes while Gaps-to-Close list six
  transforms; a verifier cannot tell whether title-promotion, `meta/docs/` skip,
  or `topic` backfill fixtures are required. Replace with an explicit fixture
  enumeration.
- 🔵 minor (medium): **"Driven by the schema column rather than a hard-coded
  list" is an implementation detail** (Acceptance Criteria — forbidden own-id
  keys) — A black-box test cannot distinguish schema-driven from hard-coded; add
  a fixture exercising the schema-driven path.
- 🔵 minor (medium): **"Representative corpus" for the regression guard is
  undefined** (Acceptance Criteria — regression guard) — Specify the minimum
  contents so the zero-violations assertion's scope is concrete.
- 🔵 minor (medium): **meta/docs/ skip criterion checks one violation code but
  not the broader "skipped" outcome** (Acceptance Criteria — meta/docs/ skip) —
  Add an unmutated-content assertion so "skipped" is measurable.
- 🔵 minor (low): **End-to-end criterion depends on an unbounded "full corpus"**
  (Acceptance Criteria — corpus-wide validator passes) — "Full corpus" has no
  fixed referent; bind it to the fixture corpus containing the triggering shapes.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-17

**Verdict:** COMMENT

The edits resolved the single major finding and the entire corpus/fixture
verification theme. Re-running all five lenses surfaced only minor and
suggestion-level items, several of which are pre-existing polish rather than
regressions introduced by the edits. The dominant remaining thread is a
five-vs-six count inconsistency (the Summary/Assumptions say "five gaps" while
Requirements/Acceptance Criteria enumerate six numbered items), independently
flagged by clarity and completeness. The work item is ready for implementation
as-is; the residual items are optional tightening.

### Previously Identified Issues

- 🟡 **Testability**: "All four reproduction shapes" is unbounded — **Resolved**.
  Replaced with an explicit eight-item fixture inventory; the schema-driven,
  full-corpus, and regression-guard criteria are now bound to that fixture corpus.
- 🔵 **Testability**: "Driven by the schema column" is an implementation detail —
  **Resolved**. Reframed as a fixture whose forbidden key is declared only in the
  TSV, so a hard-coded impl fails.
- 🔵 **Testability**: "Representative corpus" undefined — **Resolved**. Bound to
  the fixture corpus containing every reproduction shape.
- 🔵 **Testability**: meta/docs/ skip checks one code only — **Resolved**.
  Unmutated-bytes assertion added.
- 🔵 **Testability**: Unbounded "full corpus" — **Resolved**. Bound to the
  fixture corpus, with rationale (dogfood corpus lacks the triggering shapes).
- 🔵 **Completeness**: meta/docs/ trigger absent from reproduction — **Resolved**.
  Added as reproduction shape 5.
- 🔵 **Completeness**: Environment details only in Context — **Resolved**.
  Environment preamble added to the Reproduction block.
- 🔵 **Dependency**: Lockstep coupling absent from Dependencies — **Resolved**.
  Internal-coupling line added to Dependencies.
- 🔵 **Dependency**: Schema TSV precondition unnamed — **Partially resolved**.
  Added to Assumptions; dependency lens now suggests also surfacing it in
  Dependencies as a "Blocked by" prerequisite.
- 🔵 **Dependency**: 0001 ticket coupling not named — **Partially resolved**.
  Captured in gap 3; dependency lens suggests reflecting the 0007-after-0001
  ordering in the Dependencies section too.
- 🔵 **Clarity**: Validator violation codes undefined — **Resolved**. Glossed in
  Context.
- 🔵 **Clarity**: Mixed "PR" referents — **Resolved**. pr-description vs pr-review
  directory orientation added to Context.
- 🔵 **Clarity**: Schema/function jargon assumed known — **Resolved**. Grounded by
  the Context orientation; remaining terms anchored in Technical Notes.
- 🔵 **Clarity**: "review_number set to 1" phrasing — **Resolved**. Gap 4 now states
  defaults apply only when the key is absent.
- 🔵 **Clarity**: "both copies" vs triplicated — **Resolved**. Gap 1 names all three
  inference sites.
- 🔵 **Scope**: Prevention work bundling — **Resolved**. Drafting Notes confirms it
  ships as one unit, with a split-out escape hatch.
- 🔵 **Scope**: meta/docs/ policy vs creep — **Resolved**. Recorded as a deliberate
  resolved decision.

### New Issues Introduced

None are regressions caused by the edits. The following minor/suggestion items
were surfaced this pass (mostly pre-existing):

- 🔵 minor (clarity, completeness): **Five-vs-six count inconsistency** — Summary
  ("several"), Context/Assumptions ("five gaps") vs the six numbered
  Requirements/AC items (the sixth being the meta/docs/ out_of_scope change).
- 🔵 minor (dependency): **Schema TSV precondition** could be surfaced in
  Dependencies as a "Blocked by" prerequisite, not only in Assumptions.
- 🔵 minor (dependency): **0007-after-0001 ordering** could be reflected in
  Dependencies, not only in gap 3.
- 🔵 minor (testability): **Capstone criterion verifies `harness_run` reached**
  (an intermediate state) rather than full migration completion / recorded-as-applied.
- 🔵 minor (testability): **ticket/ticket_id drop criterion** verifies only
  `ticket` on a note, narrower than gap 3's "ticket and ticket_id on any type".
- 🔵 suggestions: gloss "PRI" / "dogfood corpus" / the rejected DIVERGE
  alternative; flag the consolidation refactor as bundled-but-separable in
  Dependencies; assert the fixture inventory is the exhaustive set of shapes.

### Assessment

The work item is ready for implementation. The blocking-class concern (the
ambiguous fixture target) is gone and verification is now bound to a concrete,
enumerated corpus. The remaining items are optional polish — the most worthwhile
being the five-vs-six count alignment (flagged by two lenses) and, if cheap,
broadening the ticket/ticket_id criterion to match gap 3's "any type" scope.

---
*Re-review generated by /accelerator:review-work-item*
