---
type: work-item-review
id: "0168-fold-visualiser-into-cli-workspace-review-1"
title: "Work Item Review: Fold the Visualiser into the cli/ Workspace"
date: "2026-07-19T22:07:17+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0168"
work_item_id: "0168"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 6
tags: []
last_updated: "2026-07-19T23:16:46+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Fold the Visualiser into the cli/ Workspace

**Verdict:** REVISE

This is a dense, structurally complete, and internally consistent story: every
expected section is present and substantively populated, the frontmatter is
valid, the module-by-module retire mapping is precise, and the dependency
analysis is unusually thorough (transitive blockers named, a non-dependency
explicitly fenced). The REVISE verdict is driven not by weakness but by breadth —
the story bundles two independently-shippable value streams, and its acceptance
criteria under-verify the behaviour-preserving parts of the refactor (engine
swap, security guard) and one dependency ordering (0165). Five major findings
cross the ≥2 REVISE threshold; none is critical.

### Cross-Cutting Themes

- **Multi-concern breadth vs. verifiability** (flagged by: scope, testability) —
  Scope flags that the story bundles a relocation-plus-dispatch stream with a
  large 8-module refactor; testability independently flags that the six ACs each
  cover a distinct concern and that behaviour-preserving parts of the refactor
  are not verified. The two lenses reinforce each other: the wider the story, the
  weaker "existing suites pass" (AC6) is as a backstop.
- **Behaviour-preservation is asserted but not verified** (flagged by:
  testability, scope) — The engine swap (gray_matter/serde_yml → serde-saphyr)
  and the Host/Origin security model are named as must-preserve, yet AC2 only
  checks the old modules are *gone* and no AC exercises equivalence or the
  security guard.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: 0165 (release manifest) is a blocker for AC5 but filed only
  as "Relates to"
  **Location**: Acceptance Criteria (AC5) / Dependencies
  AC5 removes `launch-server.sh` and the standalone `bin/checksums.json` and makes
  the visualiser fetched/verified against the unified release manifest — which
  0165 produces — but 0165 is classified as "Relates to", not a blocker. Removing
  the standalone checksums before the manifest lists the visualiser binary leaves
  the fetch/verify path unsatisfiable.

- 🟡 **Scope**: Story bundles sub-binary dispatch with a large independent
  refactor
  **Location**: Requirements
  Two separable value streams are combined: (1) making the visualiser a
  launcher-dispatched on-demand sub-binary (relocate + `start|stop|status`
  orchestration + distribution change), and (2) retiring ~8 duplicated corpus
  modules onto the shared 0179 crates plus an engine swap. The Drafting Notes
  themselves frame stream 2 as separate validation value. Either could ship
  without the other.

- 🟡 **Testability**: No equivalence criterion for the frontmatter/slug/doc-type
  engine swap
  **Location**: Acceptance Criteria
  The story swaps the frontmatter engine and re-homes slug/doc-type/typed-ref
  logic, but no AC asserts the replacements produce output equivalent to the
  retired code. AC2 only verifies the old modules are gone; AC6 delegates all
  behavioural verification to unspecified suites.

- 🟡 **Testability**: Host/Origin security guard is required but not verified by
  any criterion
  **Location**: Requirements
  Requirements mandate preserving "the loopback-binding + Host/Origin security
  model", but the ACs only verify the loopback bind (AC1); no criterion exercises
  the Host/Origin guard or its rejection outcome, so it could regress undetected.

- 🟡 **Testability**: AC6 delegates verification to "existing suites pass"
  without specifying coverage
  **Location**: Acceptance Criteria (AC6)
  AC6 is the only behavioural backstop yet relies on whatever the current suites
  cover; it does not assert they exercise the new `accelerator visualiser
  start|stop|status` dispatch path or the launcher-resolved config path, so it
  could pass while the story's core new behaviour is untested.

#### Minor

- 🔵 **Scope**: Six acceptance criteria across distinct concerns signal a large
  story
  **Location**: Acceptance Criteria
  The six ACs each target a different concern (lifecycle, module retirement,
  workspace membership, embed, distribution, end-to-end green); combined with the
  unresolved reconciliation tensions in Technical Notes, this is a large single
  delivery unit that tends to accumulate partial progress that cannot ship.

- 🔵 **Testability**: Idle-shutdown "same timeout as today" has no stated value
  **Location**: Acceptance Criteria (AC1)
  AC1 requires the idle server self-shut-down "on the same timeout as today" but
  gives no value, so a verifier must independently discover the current timeout
  before measuring conformance.

- 🔵 **Clarity**: "Q1" referenced as resolved without stating the question
  **Location**: Context / Assumptions
  Both Context and Assumptions cite "Q1" as resolved, but the work item never
  states what Q1 asked and no Open Question is labelled Q1 — it references a
  numbered question from the source research doc not reproduced here. The
  substance (unit move preserves embed path) is fortunately restated inline.

#### Suggestions

- 🔵 **Testability**: AC1 bundles four distinct behaviours into one checkbox
  **Location**: Acceptance Criteria (AC1)
  AC1 combines start, stop (recycle guard), status, and idle self-shutdown into
  one checkbox; each is independently verifiable, so partial completion cannot be
  tracked distinctly.

- 🔵 **Dependency**: External GitHub Releases coupling is implied but unnamed
  **Location**: Requirements
  Retiring the bespoke distribution in favour of the launcher fetching a
  pre-compiled binary couples success to GitHub Releases hosting the artefact;
  this external coupling is implied throughout but not recorded.

- 🔵 **Completeness**: Story does not identify the beneficiary whose need is met
  **Location**: Context
  Typed as a `story` but described entirely in internal mechanics; naming who
  benefits (maintainers freed from duplicated corpus logic, or the unified-release
  flow) would make the value target explicit.

- 🔵 **Clarity**: MSRV acronym used without definition or link
  **Location**: Acceptance Criteria (AC3)
  AC3 uses "MSRV" without expansion; standard Rust jargon, low risk, but a
  newer reader must guess.

- 🔵 **Clarity**: "All three are now done" where only two blockers are listed
  explicitly
  **Location**: Dependencies
  The section lists 0179 and 0164 explicitly (0178 only parenthetically) then
  asserts "all three are now done", relying on the reader to infer the set.

### Strengths

- ✅ Acceptance Criteria use Given/When/Then with an explicit actor, and AC2–AC5
  lean on concrete structural checks (named modules removed, `gray_matter`/
  `serde_yml` dropped, `version.workspace` inheritance, `cargo build -p
  accelerator-visualiser` succeeds, three `../frontend/dist` literals unchanged,
  `build.rs` fails cleanly without dist) that yield definitive pass/fail results.
- ✅ Scope is consistent across Summary, Context, Requirements, and Acceptance
  Criteria — the same three moves recur without contradiction — and the crate
  boundary (what stays async/isolated vs. what moves out) is stated explicitly.
- ✅ Technical Notes give a precise old-module → new-module mapping, so every
  named item (`DocTypeKey`, `patch_status`, `WorkItemIdScheme`) resolves to one
  referent.
- ✅ Dependencies are unusually thorough: upstream blockers captured with
  transitive chain and status, a non-dependency (0180) explicitly fenced with
  rationale, and Drafting Notes reconciling the stale 0166 citation.
- ✅ Frontmatter is complete and valid; optional sections (Open Questions,
  Assumptions, Technical Notes) carry real content rather than placeholders.

### Recommended Changes

1. **Promote 0165 to a blocker for AC5, or add an ordering note** (addresses:
   Dependency — 0165 blocker for AC5). Either reclassify 0165 from "Relates to"
   to an explicit blocker gating AC5, or add a Dependencies note (mirroring the
   0180 "Not blocked by" rationale) stating the visualiser's manifest entry from
   0165 must land before/with this story.

2. **Decide and record the scope split, or justify indivisibility** (addresses:
   Scope — bundles two streams; Scope — six ACs). Either split into a
   relocation-plus-dispatch story and a refactor story delivered in sequence, or
   add a sentence to the work item stating why the "first on-demand sub-binary"
   goal makes the two streams genuinely indivisible for this increment.

3. **Add behaviour-equivalence acceptance criteria** (addresses: Testability —
   engine-swap equivalence; Testability — Host/Origin guard). Add an AC asserting
   the refactored parsing/slug/doc-type inference produces output equivalent to
   the pre-refactor code for a representative corpus fixture, and an AC exercising
   the Host/Origin guard (non-loopback Host / cross-origin Origin rejected;
   matching loopback accepted).

4. **Strengthen AC6 with named coverage** (addresses: Testability — AC6 delegates
   without coverage). Name the behaviours the suites must cover post-move
   (launcher dispatch, config resolution, start/stop/status), or add an AC that
   new coverage exists for the `accelerator visualiser` sub-commands rather than
   leaning solely on "green end-to-end".

5. **State the idle timeout value and consider splitting AC1** (addresses:
   Testability — idle timeout value; Testability — AC1 bundles four behaviours).
   Give the concrete idle timeout (or reference the config key), and optionally
   split AC1 into separate start / stop / status / idle-shutdown criteria.

6. **Polish clarity shorthands** (addresses: Clarity — Q1; Clarity — MSRV;
   Clarity — three-vs-two count; Completeness — beneficiary; Dependency — GitHub
   Releases). Drop or gloss the "Q1" label, expand MSRV on first use, enumerate
   the three blockers explicitly, name the story's beneficiary in Context, and
   record the GitHub Releases external coupling.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: A dense but internally consistent work item whose scope holds
together across all sections; pronouns and referents almost always resolve to a
single named subject, and the Acceptance Criteria name their actor via
Given/When/Then. The only clarity gaps are a handful of unlinked shorthand
references ("Q1", "MSRV") and one mildly ambiguous dependency count.

**Strengths**:
- Acceptance Criteria use Given/When/Then with an explicit actor, so
  responsibility for each observable outcome is unambiguous.
- Scope is consistent across sections — the same three moves appear in Summary,
  Context, Requirements, and Acceptance Criteria without contradiction.
- Technical Notes give a precise old-module → new-module mapping, so each named
  term resolves to exactly one referent.

**Findings**:
- 🔵 minor (medium confidence) — **"Q1" referenced as resolved without stating
  the question** (Context): Context and Assumptions cite "Q1" as resolved, but
  the item never states what Q1 asked and no Open Question is labelled Q1; it
  references a numbered question from the source research doc not reproduced here.
  The substance (unit move preserves embed path) is restated inline. Suggestion:
  drop the "Q1" label or add a one-line note of what Q1 asked.
- 🔵 suggestion (medium confidence) — **MSRV acronym used without definition or
  link** (Acceptance Criteria): AC3 uses "MSRV" without expansion. Standard Rust
  jargon, small risk, but a newer reader must guess. Suggestion: expand on first
  use or link the workspace policy.
- 🔵 suggestion (low confidence) — **"All three are now done" where only two
  blockers are listed explicitly** (Dependencies): the section names 0179 and
  0164 explicitly (0178 only parenthetically) then asserts "all three are now
  done". Suggestion: enumerate the three items explicitly.

### Completeness

**Summary**: A structurally complete and densely populated story: every expected
section is present and substantively filled, and the frontmatter carries all
required fields with valid values. The Acceptance Criteria comfortably exceed the
minimum and the Context explains the motivating forces. The only gap worth noting
is that, framed as a `story`, it never explicitly identifies the beneficiary
whose need is served, reading instead as an internal refactor.

**Strengths**:
- Frontmatter is complete and valid — kind, status, priority, id, title, parent
  all present with recognised values.
- Acceptance Criteria contains six specific, behaviour-oriented bullets — well
  beyond the minimum density.
- Context explains the why rather than restating the Summary.
- Requirements are specific and implementation-ready, naming concrete paths,
  crates, and files.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes)
  are all populated with real content.

**Findings**:
- 🔵 suggestion (low confidence) — **Story does not identify the beneficiary
  whose need is met** (Context): typed as a `story` but described entirely in
  internal mechanics without identifying the user or system whose need is served.
  Suggestion: add a short clause naming who benefits.

### Dependency

**Summary**: The Dependencies section is unusually thorough for a story: upstream
blockers (0179→0178, 0164) are named with resolved status, a non-dependency
(0180) is explicitly disambiguated with rationale, and the Drafting Notes explain
why reverse `blocks` edges live on the blocker items. The one genuine gap is that
AC5 makes the fetch/verify/dispatch contingent on 0165's release manifest, yet
0165 is filed only as "Relates to" rather than an ordering blocker.

**Strengths**:
- Upstream blockers captured precisely with transitive chain and status (0179
  resting on 0178, 0164), all noted as `done`.
- Explicitly records a non-dependency — 0180 called out as "Not blocked by" with
  reason, preventing a false blocker assumption.
- Drafting Notes reconcile the stale 0166 citation and explain the canonical
  `blocks: 0168` edges live on 0179/0164.

**Findings**:
- 🟡 major (medium confidence) — **0165 (release manifest) is a blocker for AC5
  but filed only as "Relates to"** (Acceptance Criteria): AC5 removes
  `launch-server.sh` and the standalone `bin/checksums.json` and requires the
  visualiser be fetched/verified against the unified release manifest that 0165
  produces, but 0165 is classified only as "Relates to". Removing the standalone
  checksums before the manifest lists the visualiser binary leaves the
  fetch/verify path unsatisfiable. Suggestion: promote 0165 to an explicit
  blocker for AC5, or add a Dependencies note that its manifest entry must land
  before/with this story.
- 🔵 suggestion (low confidence) — **External GitHub Releases coupling is implied
  but unnamed** (Requirements): retiring the bespoke distribution couples success
  to GitHub Releases hosting the artefact and manifest; this external coupling is
  implied throughout but not recorded. Suggestion: add a one-line Dependencies
  note naming GitHub Releases as the external artefact source.

### Scope

**Summary**: A coherent, well-bounded increment under epic 0136 that names what
moves, what stays isolated in the crate, and what is explicitly out of scope
(0180). However, it bundles two arguably separable value streams — making the
visualiser a launcher-dispatched on-demand sub-binary, and retiring the
visualiser's duplicated corpus logic onto the shared 0179 crates — into one
heavyweight story whose six acceptance criteria each cover a distinct concern.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria describe a consistent scope, and
  the story states explicitly what stays inside the crate versus what moves out.
- Out-of-scope work is deliberately fenced ("Not blocked by 0180"), resisting
  scope creep into the atomic-store primitives.
- As a child of epic 0136 with clear parent linkage and a stated "Phase 5" role,
  the story sits correctly within a larger decomposition.

**Findings**:
- 🟡 major (medium confidence) — **Story bundles sub-binary dispatch with a large
  independent refactor** (Requirements): two value streams that could ship and
  roll back independently — (1) relocation + `start|stop|status` orchestration +
  distribution change, and (2) retiring ~8 duplicated corpus modules onto the
  shared crates plus a `gray_matter`/`serde_yml` → `document` engine swap. The
  Drafting Notes frame stream 2 as separate validation value. Impact: a single
  story spanning relocation, an 8-module refactor with unresolved async/sync
  reconciliation, an engine swap, orchestration migration, and a distribution
  change is hard to plan, review, and roll back; a failure in any stream blocks
  the others. Suggestion: split into a relocation-plus-dispatch story and a
  refactor story delivered in sequence, or confirm why the two are indivisible.
- 🔵 minor (low confidence) — **Six acceptance criteria across distinct concerns
  signal a large story** (Acceptance Criteria): the six ACs each target a
  different concern; combined with the unresolved reconciliation tensions in
  Technical Notes, this is a large delivery unit. Suggestion: re-check whether it
  warrants decomposition; if judged indivisible given the "first on-demand
  sub-binary" goal, that judgement is defensible.

### Testability

**Summary**: The Acceptance Criteria are largely well-formed for a
refactor-and-relocate story: AC1 is a clean Given/When/Then for the lifecycle,
and AC2–AC5 lean on concrete structural checks that produce definitive pass/fail
results. The main testability gaps are behavioural: the parsing/slug/doc-type
refactor swaps engines yet has no equivalence criterion, the Host/Origin security
guard is not verified by any criterion, and the sole behavioural backstop (AC6)
delegates entirely to "the existing suites pass".

**Strengths**:
- AC1 is framed as an explicit Given/When/Then lifecycle scenario covering start,
  stop (owner-PID AND start_time recycle guard), and status.
- AC2–AC5 use concrete structural checks (named modules removed, deps dropped,
  `version.workspace` inheritance, `cargo build -p accelerator-visualiser`
  succeeds, three `../frontend/dist` literals unchanged, `build.rs` fails cleanly
  without dist) confirmable definitively.
- AC5 pins removal of specific artefacts rather than describing the change
  abstractly, making the file-level outcome directly checkable.

**Findings**:
- 🟡 major (medium confidence) — **No equivalence criterion for the
  frontmatter/slug/doc-type engine swap** (Acceptance Criteria): the story swaps
  the frontmatter engine and re-homes slug/doc-type/typed-ref logic, but no AC
  asserts the replacements produce equivalent output. AC2 only verifies the old
  modules are gone; AC6 delegates behaviour to unspecified suites. Suggestion:
  add a parity criterion over a representative corpus fixture (or named
  golden/parity tests).
- 🟡 major (medium confidence) — **Host/Origin security guard is required but not
  verified by any criterion** (Requirements): Requirements mandate preserving the
  loopback + Host/Origin security model, but ACs only verify the loopback bind;
  no criterion exercises the Host/Origin guard or its rejection outcome.
  Suggestion: add a criterion that a non-loopback Host / cross-origin Origin is
  rejected while a matching loopback Host/Origin is accepted.
- 🟡 major (medium confidence) — **AC6 delegates verification to "existing suites
  pass" without specifying coverage** (Acceptance Criteria): AC6 relies on
  whatever the current suites cover and does not assert they exercise the new
  `accelerator visualiser start|stop|status` dispatch path or the launcher-
  resolved config path. Suggestion: name the behaviours the suites must cover
  post-move, or add a criterion that new coverage exists for the sub-commands.
- 🔵 minor (high confidence) — **Idle-shutdown "same timeout as today" has no
  stated value** (Acceptance Criteria): AC1 requires the idle server self-shut-
  down "on the same timeout as today" but gives no value. Suggestion: state the
  concrete idle timeout or reference the exact config key.
- 🔵 suggestion (high confidence) — **AC1 bundles four distinct behaviours into
  one checkbox** (Acceptance Criteria): AC1 combines start, stop (recycle guard),
  status, and idle self-shutdown into a single checkbox; partial completion
  cannot be tracked distinctly. Suggestion: split into separate criteria.

## Re-Review (Pass 2) — 2026-07-19

**Verdict:** REVISE

Re-ran clarity, completeness, dependency, scope, and testability against the revised
work item after the first round of edits. All five original majors resolved, but the
edits introduced three new majors (two of them regressions from the review's own
edits), which were then fixed in a second edit round.

### Previously Identified Issues

- 🟡 **Dependency**: 0165 blocker for AC5 — **Resolved**. The new ordering-constraint
  note is praised as "exactly the coupling the lens looks for".
- 🟡 **Scope**: bundles two streams — **Resolved** (downgraded to minor). The
  Drafting-Notes indivisibility justification "answers the obvious 'why not split'
  objection"; only the distribution cut-over remains a soft, well-documented seam.
- 🟡 **Testability**: engine-swap equivalence — **Resolved** (parity criterion added;
  refined further below).
- 🟡 **Testability**: Host/Origin guard unverified — **Resolved**; now a strength.
- 🟡 **Testability**: AC6 delegates to "existing suites" — **Resolved** (named-coverage
  criterion added).
- 🔵 Clarity (Q1, MSRV, three-blocker count), Completeness (beneficiary),
  Dependency (GitHub Releases) — **All resolved**; completeness returned zero findings.

### New Issues Introduced (and fixed this pass)

- 🟡 **Clarity** (high): "AC5" references in Dependencies had no resolvable referent —
  the criteria are an unnumbered checkbox list and the inserts shifted positions, so
  "AC5" pointed to the wrong criterion. **Fixed**: replaced positional "AC5" refs with
  the descriptive "distribution cut-over criterion".
- 🟡 **Clarity** (medium): the beneficiary sentence implied the whole ~15.4k-line crate
  was duplicated corpus logic, contradicting Context's "much of it". **Fixed**: reworded
  to scope the figure to the duplicated corpus slice and note the crate is retained.
- 🟡 **Testability** (medium): the added parity criterion left fixture and equivalence
  relation undefined. **Fixed**: it now names the fixture (one document per `DocTypeKey`
  variant + fence/slug/ID edge cases) and the field-for-field equivalence relation
  (whitespace/error-text out of scope).
- 🔵 **Testability** minors also addressed: the security criterion pinned to
  `403 Forbidden` with the state-changing-Origin nuance; the idle criterion given an
  "idle" definition and a short-timeout verification method; status given a concrete
  observable signal and transitions; "coverage exists" replaced with named test cases.

### Assessment

All five original majors are resolved and the three regressions introduced during
iteration have been corrected in place. The work item is materially stronger — twelve
individually-checkable acceptance criteria with concrete pass/fail signals, a
well-justified single-story scope, and a fully mapped dependency ordering. The Pass-2
REVISE verdict reflects the state *before* this pass's fixes were applied; the fixes
target every finding it raised but have not themselves been re-verified by a fresh
lens pass — a Pass 3 would confirm APPROVE. Remaining open items (scope's
distribution-cut-over seam, the 0165 downstream-edge and launcher-config-interface
couplings) are low-confidence suggestions the author can weigh, not blockers.

## Re-Review (Pass 3) — 2026-07-19

**Verdict:** COMMENT

Re-ran clarity, scope, and testability to verify the Pass-2 fixes. Both Pass-2
regressions are confirmed gone, and one merged new major (the config-path criterion
contradicting Open Question 3, flagged independently by clarity and testability) was
found and then fixed. Deduped, that single major sits below the 2-major REVISE
threshold, so the pass lands at COMMENT — an improvement from REVISE.

### Previously Identified Issues

- 🟡 **Clarity**: "AC5" dangling referent — **Resolved**; no positional-reference
  problem remains.
- 🟡 **Clarity**: 15.4k-line contradiction — **Resolved**; the figure is scoped to the
  duplicated slice and the retained crate is called out.
- 🟡 **Testability**: parity fixture/equivalence undefined — **Resolved**; the parity AC
  is now praised as "a genuinely runnable oracle".
- 🟡 **Scope**: bundles two/four streams — **Resolved to suggestion/minor**; the
  indivisibility justification is accepted, remaining findings are acknowledged
  judgement calls, no majors.

### New Issues Introduced (and fixed this pass)

- 🟡 **Clarity + Testability** (merged, medium): the added coverage criterion asserted
  "the launcher-resolved config path is honoured", but Open Question 3 leaves that
  mechanism open — if the server reads config directly, the criterion is
  unsatisfiable. **Fixed**: reworded to an outcome-based check ("the server honours
  configuration from the location it is directed to … independent of whether the
  launcher passes it in or the server reads `.accelerator/*.md` directly"), explicitly
  cross-referencing Open Question 3.
- 🔵 **Testability** minors also addressed: the status criterion now pins the
  `running`/`stopped` stdout token as the definitive signal (dropped the ambiguous
  "and/or"); the parity criterion states golden fixtures are frozen *before* the old
  modules are deleted; the cut-over criterion marks the file-removal half as verifiable
  at completion and the fetch/verify half as gated on 0165's manifest entry.
- 🔵 **Clarity** suggestion: the 0166 "relates to" wording now matches the Drafting
  Notes ("validates the shared-crate approach 0166 described; 0166 itself built
  nothing").

### Assessment

The work item is now in strong shape. After this pass's fixes there are no
outstanding major findings across any lens; the residue is a small set of
low-confidence suggestions — the "visualiser" bare-noun referent drift (crate vs.
binary vs. product), the optional extraction of the externally-gated distribution
cut-over into a follow-up story, and recording a `blocks: 0165` edge if 0165's release
validation actually depends on this fold-in. None blocks planning. The recorded
COMMENT verdict reflects the Pass-3 measurement *before* its fixes were applied; with
them applied the item is effectively APPROVE-ready and can proceed to `/create-plan`.

## Re-Review (Pass 4) — 2026-07-19

**Verdict:** COMMENT

Re-ran clarity, scope, and testability to confirm the Pass-3 fixes. Both Pass-3
regressions are confirmed gone (config-path criterion now praised as intentionally
bounded; no positional-referent or 15.4k contradiction). One major remained: the
distribution cut-over criterion bundled a verify-now clause (files removed) with a
verify-later clause (fetch/verify gated on 0165) in a *single checkbox*, so the
checkbox could not get a clean pass/fail. Deduped this is one major → COMMENT.

### Fixes applied after Pass 4

- 🟡 **Testability** (cut-over checkbox): **split into two criteria** — (a)
  `launch-server.sh` + `bin/checksums.json` removed, verifiable at completion; (b) the
  launcher fetch/verify/dispatch path, explicitly gated on 0165's manifest entry and
  verified when it lands. Also cross-referenced the gating in the Requirements bullet
  so the flat requirement and the gated dependency now describe the same conditional.
- 🔵 **Testability** (config-honouring test): named the concrete subject —
  `visualiser.idle_timeout` set to a non-default value in a fixture config, asserting
  the server's resolved timeout matches.
- 🔵 **Testability** (parity fixture): enumerated the fence-offset boundary inputs
  (leading blank lines, CRLF, no trailing newline, empty frontmatter block, no
  frontmatter at all) so the fixture set is verifiable from the criterion itself.

## Re-Review (Pass 5) — 2026-07-19

**Verdict:** APPROVE

Re-ran clarity, scope, and testability against the post-Pass-4 work item, with **no
edits made during this pass** — so this verdict reflects the work item's actual
current state. **Zero major findings across all three lenses**; the distribution
cut-over major is resolved by the checkbox split. With no criticals and no majors, the
verdict is APPROVE.

### Confirmation

- 🟡 **Testability** (cut-over): **Resolved** — the split criteria are each
  individually verifiable; the lens no longer raises a major.
- 🟢 **Clarity, Scope**: no majors; both lenses affirm the internal consistency, the
  inline-defined terms, the concrete status tokens, and the well-reasoned single-story
  scope.

### Residual (optional polish — none blocks APPROVE or planning)

- 🔵 **Testability** minors — **all four applied post-Pass-5** (2026-07-19): the
  script-removal criterion now names all five orchestration scripts (`visualiser.sh`,
  `launch-server.sh`, `stop-server.sh`, `status-server.sh`,
  `write-visualiser-config.sh`) plus `bin/checksums.json`; the launcher-dispatch test
  now names an observable (a launcher spy/test-double recording the dispatch, or a
  distinguishing side effect); AC1's precondition now specifies the required
  visualiser configuration (the fixture config the orchestration tests use); and AC3
  now pins the initial never-started status token to `stopped`.
- 🔵 **Clarity** minor/suggestions: "That duplicated corpus logic now lives in the
  shared crates" can momentarily read as already-migrated; JSONL is unexpanded; the
  intentionally-abstract config-source phrase is flagged only for the record.
- 🔵 **Scope** suggestions: the distribution cut-over remains the natural seam if the
  story is ever peeled apart, and the four-stream breadth sits near the story/epic
  boundary — both acknowledged judgement calls, not defects.

### Assessment

The work item is APPROVE-confirmed by a fresh lens pass with no outstanding majors on
any lens. It is ready for `/create-plan`. The residual items above are minor tightening
the author may apply at planning time or leave as-is.

## Re-Review (Pass 6) — 2026-07-19

**Verdict:** COMMENT

Ran after applying the four Pass-5 residual testability minors, to lens-verify the
final state. Clarity and scope returned **no majors**. Testability raised **one major**
— the same externally-gated distribution-cut-over criterion, re-flagged: an AC that is
"verified when 0165's entry lands, not before" cannot be checked at this story's
completion. This is genuine reviewer variance on a real structural point (Pass 4
flagged it, Pass 5 accepted the checkbox split, Pass 6 re-flagged the gated half).
Deduped, one major → COMMENT.

### Fixes applied after Pass 6

- 🟡 **Testability** (gated fetch/verify AC): added a **within-story stand-in** — the
  launcher's fetch/verify path is verified now against a local/test manifest fixture;
  only the equivalent assertion against the live release manifest defers to 0165. The
  criterion is no longer unverifiable at completion.
- 🔵 **Testability** (guard ordering): dropped the un-observable "same guard ordering"
  clause; replaced with two independently-testable cases (a Host-only violation and a
  state-changing Origin-only violation each rejected `403`) that exercise both guards.
- 🔵 **Testability** (8h default): added a config-resolution assertion — with no
  `visualiser.idle_timeout` set, the resolved value equals `8h` (no long-running wait).
- 🔵 **Clarity** (Open Question ordinal): numbered the Open Questions list so the
  "Open Question 3" cross-reference resolves deterministically.
- 🔵 **Clarity** (launch-server.sh dual role): stated once in Requirements that
  `launch-server.sh` has two roles retired by two mechanisms — orchestration re-homes
  into `accelerator visualiser start`, distribution/fetch is replaced by the launcher.

### Residual (not applied — reviewer-variance / judgement calls)

- 🔵 **Clarity** suggestion: "server" / "crate" / "visualiser crate" used somewhat
  interchangeably for the `accelerator-visualiser` unit (lib+bin split). Cosmetic.
- 🔵 **Scope** suggestions (recurring, acknowledged): the four-stream breadth sits near
  the story/epic boundary, and the distribution cut-over remains the natural seam if the
  story is ever peeled apart. These are the author's standing "keep as one story"
  decision, not defects.

### Assessment

This is the sixth pass. Every major raised across passes 1–6 has been resolved; the one
recurring structural point (an externally-gated acceptance criterion) now has a
within-story stand-in verification, which should settle the oscillation. The remaining
findings are reviewer-variance-level polish on a deliberately rich work item — the kind
of thing further passes will keep surfacing without materially improving the item. The
recorded COMMENT reflects the Pass-6 measurement *before* its fixes; with them applied
there are no known outstanding majors. Recommendation: treat 0168 as ready for
`/create-plan` and stop re-reviewing; the diminishing returns now outweigh further passes.

## Approval — 2026-07-19

**Final verdict: APPROVE** (author decision).

The author accepted the work item after Pass 6. Every major raised across passes 1–6 is
resolved; the one recurring structural point (the externally-gated distribution-cut-over
criterion) now carries a within-story stand-in verification. The remaining findings are
reviewer-variance-level polish and acknowledged scope judgement calls, none blocking.
The frontmatter verdict is set to APPROVE to reflect this decision; it supersedes the
Pass-6 COMMENT measurement (which was taken before Pass 6's own fixes were applied).
