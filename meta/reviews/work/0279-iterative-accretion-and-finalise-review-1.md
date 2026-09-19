---
type: "work-item-review"
id: "0279-iterative-accretion-and-finalise-review-1"
title: "Work Item Review: Iterative Accretion and Finalise"
date: "2026-09-19T07:05:22+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0279"
work_item_id: "0279"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-19T15:38:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Iterative Accretion and Finalise

**Verdict:** REVISE

0279 is a well-structured, substantively complete story whose central design
decision — deriving synthesis staleness from the base `status` lifecycle rather
than a stored flag — is stated consistently across every section. The findings
cluster in two areas: several acceptance criteria state status-transition and
count outcomes without the starting-state preconditions that the
reopen-regression and finalise-refusal semantics require, producing internal
contradictions; and several criteria rest on qualitative or under-specified
checks (idempotent synthesis, "reads as one dossier", unpinned `round_count`)
that give no definitive pass/fail. A dependency gap compounds this — the design
leans repeatedly on the 0278 manifest-status collapse, yet 0278 is absent from
Blocked-by.

### Cross-Cutting Themes

- **Status-transition criteria omit starting-state preconditions** (flagged by:
  clarity, testability) — The "Lifecycle transitions" criterion states
  `outline`→`outlined` and `finalise`→`complete` unconditionally, and the
  "Pending-round revision" criterion asserts status "stays `outlined`", but both
  collide with the reopen-regression (`synthesised`→`researching`) and
  finalise-refusal criteria once a set has been reopened. A verifier applying any
  one criterion in isolation derives the wrong expected status.
- **`round_count` derivation is unpinned and divergent** (flagged by: clarity,
  testability) — The "Checkbox and count reconciliation" criterion says
  `conduct` sets `round_count` "to match the findings on disk", but `round_count`
  counts rounds, not findings; the Technical Notes give a second model
  ("`round_count` is bumped by `conduct`"). Neither pins the value a verifier
  should assert, and the two risk disagreeing in exactly the gap-fill
  re-invocation case this slice introduces.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: "Pending-round revision" asserts status stays `outlined`,
  contradicting a reachable `researching` state
  **Location**: Acceptance Criteria (Pending-round revision)
  The "highest round pending" condition is also reachable on a `researching`
  set: after `synthesise`, an `outline` appends a pending Round N+1 and regresses
  to `researching`; re-running `outline` then meets the same condition while the
  status is `researching`, not `outlined`. The criterion states an outcome that
  holds only if the set began at `outlined`, a precondition it never states.

- 🟡 **Clarity + Testability**: `round_count` derivation is unpinned and
  divergent between the AC and Technical Notes
  **Location**: Acceptance Criteria (Checkbox and count reconciliation) /
  Technical Notes
  "Match the findings on disk" is precise for `finding_count` (one file per
  finding) but ambiguous for `round_count`, which counts conducted rounds. The
  Technical Notes give a second model — "`round_count` is bumped by `conduct`" —
  reading as an increment per run, which diverges from a reconciliation in the
  gap-fill case where new findings land within an existing round, risking an
  over-counted `round_count`.

- 🟡 **Dependency**: 0278 (manifest-status collapse + manifest schema row) is an
  uncaptured upstream blocker
  **Location**: Dependencies
  The body repeatedly leans on "the 0278 manifest-status collapse", and per epic
  0121 the `(topic-research, manifest)` schema row that must admit `complete` is
  corpus-side work carried by 0278, not 0277 — yet Dependencies lists only
  "Blocked by: 0277". Without 0278's collapse and the `complete`-admitting schema
  row, `finalise`'s `status: complete` write would fail frontmatter validation.

- 🟡 **Testability**: Idempotent-synthesis criterion lacks a defined notion of
  "same dossier"
  **Location**: Acceptance Criteria (Wholesale, idempotent synthesis)
  `synthesise` runs inline as an LLM-driven wholesale prose rewrite, so it has no
  defined equivalence — byte-identical output is unrealistic and semantic
  "sameness" is subjective. A verifier cannot produce a definitive pass/fail for
  idempotency.

- 🟡 **Testability**: "Reads as one dossier" is a subjective quality judgment
  **Location**: Acceptance Criteria (Single dossier)
  "Read as one dossier" has no defined pass/fail procedure; only "no
  round-by-round narration" and "`outline.md` retains its round-grouped
  checklist" are concretely checkable. Two reviewers could reasonably disagree on
  whether the corpus reads as one dossier in a slice the epic says verifies
  "structure and mechanism only".

- 🟡 **Testability**: Lifecycle-transition criterion states unconditional
  mappings that conflict with the reopen and finalise-refusal criteria
  **Location**: Acceptance Criteria (Lifecycle transitions)
  The criterion asserts `outline`→`outlined` and `finalise`→`complete` with no
  starting-state precondition, but "Reopen" requires `outline` on a
  `synthesised`/`complete` set to regress to `researching`, and "Finalise
  refuses" requires `finalise` on a non-`synthesised` set to mutate nothing — so
  read literally the transition criterion contradicts both.

- 🟡 **Testability**: `primary` sync requirement has no verifying acceptance
  criterion
  **Location**: Requirements
  Requirements state that the build verbs keep `manifest.md`'s `primary` in sync,
  but no criterion specifies or verifies `primary` behaviour in this slice — in
  particular whether a reopen (`synthesised`→`researching`) flips `primary` back
  to `brief.md` or leaves it at `synthesis.md`.

#### Minor

- 🔵 **Dependency**: 0283 recursion engine named in Context but not captured as a
  Blocks edge
  **Location**: Context / Dependencies
  Context states "automatic intra-finding recursion is 0283", and 0283 adds
  `depth > 1` recursion inside the `conduct` verb this story substantially
  restructures — yet Blocks captures only 0282 and 0284. If 0283 builds on this
  slice's restructured `conduct`, that ordering is invisible in the dependency
  record. (The epic ties 0283's ordering to the Slice 3 gate rather than Slice 2,
  so the edge is plausible but not certain.)

#### Suggestions

- 🔵 **Scope**: Title and Summary name two capabilities joined by "and"
  **Location**: Summary
  "Iterative Accretion **and** Finalise" is the classic split signal, but the two
  are genuinely coupled: `finalise`'s gate and the reopen-regression share the
  same base-`status` lifecycle. No split needed — optionally add one line making
  the coupling explicit so the "and" reads as one deliberate increment.

- 🔵 **Completeness**: Context does not explicitly name the beneficiary or need
  being met
  **Location**: Context
  As a story, 0279's Context frames the work in terms of system capability but
  never states whose need is met or why — the "for whom". Sibling 0277 opens with
  an explicit beneficiary. The gap is small because the parent epic carries the
  user story.

- 🔵 **Clarity**: Load-bearing terms used before an inline gloss
  **Location**: Summary / Dependencies
  "Anti-changelog discipline" (Summary) is glossed only far below by the "Single
  dossier" criterion, and "the knob" (Dependencies) is an informal name for
  0282's tunable depth/breadth config. Both are recoverable from 0121, so this is
  a readability nicety, not an undefined-term defect.

- 🔵 **Testability**: "No staleness field written anywhere" has unbounded
  verification scope
  **Location**: Acceptance Criteria (Reopen is the sole staleness signal)
  "Anywhere" is unbounded, leaving the verifier without a defined set of files to
  scan to confirm the negative. Bound it to the set's documents (`manifest.md`,
  `outline.md`, `synthesis.md`, any finding).

- 🔵 **Testability**: Finalise "refuses" has no specified observable signal
  **Location**: Acceptance Criteria (Finalise refuses a non-synthesised set)
  "Mutates nothing" is verifiable by file comparison, but "refuses" has no
  specified manifestation (error message, non-zero exit) a verifier can assert
  on. Specify the refusal signal, e.g. non-zero exit with a message naming the
  current status.

- 🔵 **Dependency**: Gap-detection conduct inherits 0277's live-web (WebFetch)
  coupling, not restated here
  **Location**: Assumptions
  The "Gap-only conduct" criterion spawns researcher agents that (per 0277)
  depend on live external web access and are "coupled to external site
  availability". 0279 neither names this nor restates the caveat; add a one-line
  Assumptions note so the external dependency is visible on 0279 too.

### Strengths

- ✅ Every expected section is present and substantively populated — no empty or
  placeholder-only sections. (completeness)
- ✅ Ten atomic Given/When/Then acceptance criteria, each mapping to a distinct
  behaviour and exceeding the minimum. (completeness, testability)
- ✅ Requirements are specific and implementable — each build verb's behaviour is
  spelled out, so an implementer could start without follow-up questions.
  (completeness)
- ✅ Named actors throughout — every requirement and criterion attributes its
  action to a specific verb, with no passive construction obscuring who acts.
  (clarity)
- ✅ Concrete, observable outcomes — base `status` values, checkbox states,
  on-disk files, counts — reinforced by strong negative assertions ("leaves every
  existing finding unchanged", "mutates nothing") that yield definitive
  pass/fail. (clarity, testability)
- ✅ The derived-staleness mechanism and the single finalise gate
  (`status == synthesised`) are described identically across Summary,
  Requirements, Acceptance Criteria, Open Questions and Technical Notes.
  (clarity, scope)
- ✅ Clear in/out-of-scope boundaries — visualiser (0284), recursion (0283) and
  tunability (0282) explicitly excluded; single skill, single team. (scope)
- ✅ Clean, orchestrated decomposition with 0277 (forward lifecycle path) and the
  0281/0282/0284 edges recorded with bidirectional hygiene. (scope, dependency)
- ✅ Strong traceability — the staleness resolution and the
  `research_status`→base-`status` reconciliation are recorded with dates and
  rationale in the Drafting and Technical Notes. (completeness, clarity)

### Recommended Changes

1. **Scope each status-transition criterion to its starting state** (addresses:
   Lifecycle-transition conflict; Pending-round revision) — Rewrite "Lifecycle
   transitions" as happy-path mappings from named starting states (`outline` from
   `briefed`/`outlined` → `outlined`; `finalise` from `synthesised` →
   `complete`), and state the starting status the "Pending-round revision"
   outcome assumes plus the resulting status when the revision runs on a reopened
   `researching` set.

2. **Pin one derivation rule for `round_count`** (addresses: round_count
   divergence) — State e.g. "`round_count` equals the highest `round` stamped on
   any finding on disk", make the "Checkbox and count reconciliation" criterion
   and the Technical Notes agree, and confirm a gap-fill `conduct` within an
   existing round does not increment it.

3. **Make 0278 a visible upstream dependency** (addresses: 0278 uncaptured
   blocker) — Add 0278 to Blocked-by, or state the transitive coverage
   explicitly ("satisfied via the 0277↔0278 co-land"), so the reliance on the
   manifest-status collapse and the `complete`-admitting manifest schema row is a
   visible edge rather than buried in Technical/Drafting Notes.

4. **Replace qualitative synthesis checks with observable proxies** (addresses:
   idempotent synthesis; reads as one dossier) — For idempotency, assert
   `rounds_covered` unchanged and an identical cited-findings/source set and
   section structure rather than equal prose. For "single dossier", anchor to
   "`synthesis.md` contains no per-round section headings and no round-sequencing
   language" and defer holistic readability to the epic's Slice 3 human gate.

5. **Add or explicitly de-scope a `primary` criterion** (addresses: primary sync
   no AC) — Pin `primary`'s expected value across the relevant transitions (e.g.
   remains `synthesis.md` while a dossier exists, including after reopen), or note
   that `primary` is unchanged from 0277 and out of scope here.

6. **Tighten the two negative/observable-signal criteria** (addresses: unbounded
   staleness scope; finalise refusal signal) — Bound "no staleness field" to the
   named set documents, and specify finalise's refusal signal (non-zero exit with
   a message naming the current status).

7. **Capture the remaining dependency and clarity niceties** (addresses: 0283
   Blocks edge; live-web coupling; terms-before-gloss; beneficiary; title "and")
   — Decide whether 0283 is blocked by this slice's `conduct` changes and record
   it; restate the 0277 WebFetch coupling in Assumptions; gloss "anti-changelog
   discipline" and "the knob" on first use; add a beneficiary clause to Context;
   optionally note the finalise/reopen coupling so the title's "and" reads as one
   increment.

## Per-Lens Results

### Clarity

**Summary**: 0279 is largely clear and internally coherent: the finalise gate
and derived-staleness mechanism are stated consistently across Summary,
Requirements, Acceptance Criteria and Technical Notes, every verb is named as the
actor, and outcomes are given as concrete observable states. The main clarity
risk is an unstated starting-status precondition in the "Pending-round revision"
criterion that contradicts a reachable `researching` state, plus a divergent
update model for `round_count` between the AC and Technical Notes. Terminology is
inherited cleanly from the parent epic, with only a few load-bearing terms used
before an inline gloss.

**Strengths**:
- Named actors throughout — every requirement and criterion attributes each
  action to a specific verb, so no passive construction obscures who performs it.
- Concrete, observable outcomes — results are stated as specific base `status`
  values, checkbox states, on-disk finding files and counts rather than vague
  desired properties.
- The derived-staleness mechanism and the single finalise gate
  (`status == synthesised`) are described identically across Summary,
  Requirements, Acceptance Criteria, Open Questions and Technical Notes.
- Domain vocabulary (set, round, focus area, finding, base `status`) is used
  consistently with 0121's definitions, and the `research_status` → base `status`
  reconciliation is explicitly recorded in the Drafting Notes.

**Findings**:
- 🟡 **Major** (confidence: medium) — *"Pending-round revision" asserts status
  stays `outlined`, contradicting a reachable `researching` state* — Acceptance
  Criteria (Pending-round revision). The pending-round condition is reachable on
  a `researching` set: after `synthesise`, `outline` appends a pending Round N+1
  and regresses to `researching`; re-running `outline` meets the same condition
  at `researching`, not `outlined`. The criterion's outcome holds only if the set
  began at `outlined`, a precondition it never states. A reader cannot tell
  whether `outline` should force `status: outlined` unconditionally (contradicting
  reopen-regression) or leave a reopened set at `researching`. Suggestion: state
  the assumed starting status and the resulting status for a pending-round
  revision on a `researching` set.
- 🟡 **Major** (confidence: medium) — *Divergent update models for `round_count`:
  "match findings on disk" vs "bumped by conduct"* — Acceptance Criteria
  (Checkbox and count reconciliation) / Technical Notes. "Match the findings on
  disk" is precise for `finding_count` but ambiguous for `round_count`, which
  counts conducted rounds; the Technical Notes' "bumped by `conduct`" reads as an
  increment per run and diverges from a reconciliation for the gap-fill case,
  risking an over-counted `round_count`. Suggestion: state one derivation rule
  (e.g. highest `round` stamped on any finding) and make the AC and Technical
  Notes agree.
- 🔵 **Suggestion** (confidence: low) — *Load-bearing terms used before any inline
  gloss* — Summary / Dependencies. "Anti-changelog discipline" (Summary/
  Requirements) is glossed only far below by "Single dossier", and "the knob"
  (Dependencies) is an informal name for 0282's tunable depth/breadth config.
  Both are recoverable from 0121, so this is a readability nicety. Suggestion:
  gloss both on first use.

### Completeness

**Summary**: 0279 is a highly complete story: every expected section is present
and substantively populated, the frontmatter is well-formed with a recognised
kind and status, and it carries ten atomic Given/When/Then acceptance criteria
backed by six specific requirements. The only completeness gap worth noting is
that, unlike sibling 0277, the Context frames the work purely in terms of system
capability and does not explicitly name the beneficiary or the need being met — a
minor "for whom" omission mitigated by the parent epic (0121), which carries the
user story.

**Strengths**:
- All expected sections present and substantively populated (Summary, Context,
  Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
  Technical Notes, Drafting Notes, References) — no empty or placeholder-only
  sections.
- Ten atomic Given/When/Then acceptance criteria far exceed the minimum, each
  mapping to a specific, distinct behaviour.
- Requirements are specific and implementable — each build verb's behaviour is
  spelled out, so an implementer could start without follow-up questions.
- Frontmatter is complete and well-formed: kind ("story") and status ("draft")
  are recognised values, and all base fields plus parent/blocks/tags/external_id
  are present.
- Open Questions is handled honestly as "None" with a documented resolution
  cross-referenced to the Technical Notes.
- Drafting and Technical Notes give strong traceability — the staleness
  resolution and the `research_status`→base-status reconciliation are both
  recorded with dates and rationale.

**Findings**:
- 🔵 **Suggestion** (confidence: medium) — *Context does not explicitly name the
  beneficiary or need being met* — Context. As a story, 0279's Context frames the
  work entirely in terms of system capability but never states whose need is met
  or why. Sibling 0277 opens with an explicit beneficiary. Impact: a reader must
  infer the user and motivation from 0121 rather than seeing it stated; the gap is
  small because the epic carries the user story. Suggestion: add a short
  beneficiary clause mirroring 0277's framing.

### Dependency

**Summary**: From the dependency lens, 0279 captures its primary upstream blocker
(0277) and its main downstream consumers (0282 knob, 0284 detail page) with clear
rationale, and correctly cross-references the 0281 gap-proposal gating as recorded
on 0281's side. The significant gap is that the design leans explicitly and
repeatedly on "the 0278 manifest-status collapse" — including the
`(topic-research, manifest)` schema row that must admit `complete` — yet 0278
never appears in Blocked-by. A secondary gap is the recursion-engine child 0283,
named in Context and touching the same `conduct` verb this story restructures, but
absent from Blocks.

**Strengths**:
- The upstream blocker 0277 is explicitly captured with a note that the edge is
  recorded on 0277's `blocks` side, showing bidirectional hygiene.
- Downstream consumers 0282 and 0284 appear in both frontmatter `blocks` and the
  Dependencies prose, each with the reason it depends on this slice.
- The 0281 gap-proposal gating is captured with an explicit note that the reverse
  edge lives on 0281's `blocked_by`, avoiding a duplicated/asymmetric record.
- The `depth: 1` boundary and the deferral of intra-finding recursion to 0283 are
  drawn in Context, making the scope/ordering boundary visible.

**Findings**:
- 🟡 **Major** (confidence: medium) — *0278 (manifest-status collapse + manifest
  schema row) is an uncaptured upstream blocker* — Dependencies. The body
  repeatedly leans on "the 0278 manifest-status collapse", and per 0121 the
  `(topic-research, manifest)` schema row that must admit `complete` is
  corpus-side work carried by 0278, not 0277 — yet Dependencies lists only
  "Blocked by: 0277". Without 0278's collapse and the `complete`-admitting schema
  row, `finalise`'s `status: complete` write would fail frontmatter validation;
  the coupling is satisfied only implicitly via the 0277↔0278 co-land. Suggestion:
  add 0278 to Blocked-by or state the transitive coverage explicitly.
- 🔵 **Minor** (confidence: medium) — *0283 recursion engine named in Context but
  not captured as a Blocks edge* — Context. 0283 adds `depth > 1` recursion inside
  the `conduct` verb this story substantially restructures, but Blocks captures
  only 0282 and 0284. If 0283 builds on this slice's restructured `conduct`, that
  ordering is invisible and could be scheduled out of order. (The epic ties 0283's
  ordering to the Slice 3 gate rather than Slice 2, so the edge is plausible but
  not certain.) Suggestion: clarify whether 0283 is blocked by 0279 and record it,
  or note the recursion is independent of this slice's `conduct` changes.
- 🔵 **Suggestion** (confidence: low) — *Gap-detection conduct inherits 0277's
  live-web (WebFetch) coupling, not restated here* — Assumptions. The "Gap-only
  conduct" criterion spawns researcher agents that (per 0277) depend on live
  external web access and are "coupled to external site availability"; 0279
  neither names this nor restates the caveat. Suggestion: add a one-line
  Assumptions note so the external dependency is visible on 0279 as well as its
  blocker.

### Scope

**Summary**: 0279 is a coherent, well-bounded story that takes the single-round
engine from 0277 and makes it iterative and closable. Its six requirements all
serve one theme — multi-round accretion under a shared base-status lifecycle —
and the Summary, Requirements and ten atomic Acceptance Criteria describe the
same scope with no drift. It sits cleanly within one skill and one team, has
explicit in/out-of-scope boundaries, and its only real scope question is the
deliberate bundling of "iterative accretion" with "finalise", which is defensible
because both are bound by the same lifecycle/staleness mechanism.

**Strengths**:
- Clear boundaries stated in both directions: Context excludes visualiser work
  (0284), intra-finding recursion (0283), and holds depth at 1.
- Strong section alignment — Summary, the six Requirements, and the ten
  Given/When/Then criteria all describe the same scope, with no orphaned
  behaviour.
- Single service boundary and single team: all work lives in the `research-topic`
  skill and the corpus/manifest, with no cross-service orchestration.
- The decomposition boundary with 0277 is clean and orchestrated — 0277 delivers
  the forward lifecycle path and 0279 completes it (adds `complete` plus
  reopen-regression), with the ordering captured by the 0277→0279 block edge.
- Sizing is appropriate for a story despite touching five verbs and ten AC: the
  verbs are genuinely interdependent because they all maintain the one shared
  base-status lifecycle and manifest counters — one indivisible increment.

**Findings**:
- 🔵 **Suggestion** (confidence: medium) — *Title and Summary name two
  capabilities joined by "and"* — Summary. "Iterative Accretion **and**
  Finalise" is the classic split signal, but the two are genuinely coupled:
  `finalise`'s gate and the reopen-regression share the same base-`status`
  lifecycle, which is why no separate staleness field exists; splitting `finalise`
  out would create a trivially small standalone item. No split needed.
  Suggestion: optionally add one line making the coupling explicit so the "and"
  reads as one deliberate increment.

### Testability

**Summary**: This story is largely well-specified for verification: most
Acceptance Criteria are framed as Given/When/Then with explicit preconditions and
concrete, observable outcomes, and several lean on strong negative assertions
that give definitive pass/fail. The main testability gaps are qualitative or
under-specified criteria — the idempotency check has no defined notion of "same
dossier" for LLM-generated prose, the "single dossier" check rests on subjective
editorial judgment, the lifecycle-transition criterion states unconditional
mappings that contradict the reopen and finalise-refusal criteria, and the
`primary` sync requirement is left with no verifying criterion.

**Strengths**:
- Most Acceptance Criteria follow Given/When/Then with explicit preconditions and
  observable outcomes, giving a verifier a clear procedure per criterion.
- Strong use of concrete negative assertions ("leaves every existing finding file
  unchanged", "appends no new round", "refuses and mutates nothing") that yield
  definitive pass/fail.
- The "stamps `rounds_covered` equal to `round_count`" check is a precise,
  mechanically verifiable equality.
- Checkbox reconciliation defines finding-existence on disk as ground truth — a
  clear, testable rule.
- The derived-staleness design collapses staleness into a single verifiable gate
  (base status == synthesised), making finalise's admission/refusal condition
  unambiguous.

**Findings**:
- 🟡 **Major** (confidence: medium) — *Idempotent-synthesis criterion lacks a
  defined notion of "same dossier"* — Acceptance Criteria (Wholesale, idempotent
  synthesis). `synthesise` runs inline as an LLM-driven wholesale prose rewrite,
  so it has no defined equivalence — byte-identical output is unrealistic and
  semantic "sameness" is subjective; a verifier cannot produce a definitive
  pass/fail. Suggestion: replace "yields the same dossier" with a concrete
  equivalence check (`rounds_covered` unchanged; identical cited-findings/source
  set and section structure).
- 🟡 **Major** (confidence: medium) — *"Reads as one dossier" is a subjective
  quality judgment* — Acceptance Criteria (Single dossier). "Read as one dossier"
  has no defined pass/fail procedure; only "no round-by-round narration" and
  "`outline.md` retains its round-grouped checklist" are concretely checkable, in
  a slice the epic says verifies "structure and mechanism only". Suggestion:
  anchor to observable parts (no per-round headings, no round-sequencing language
  in `synthesis.md`) and defer holistic readability to the Slice 3 human gate.
- 🟡 **Major** (confidence: medium) — *Lifecycle-transition criterion states
  unconditional mappings that conflict with reopen and finalise-refusal criteria*
  — Acceptance Criteria (Lifecycle transitions). The criterion asserts
  `outline`→`outlined` and `finalise`→`complete` with no starting-state
  precondition, but "Reopen" requires `outline` on a `synthesised`/`complete` set
  to regress to `researching` and "Finalise refuses" requires `finalise` on a
  non-`synthesised` set to mutate nothing — so read literally the transition
  criterion contradicts both. Suggestion: scope each mapping to its starting
  state.
- 🟡 **Major** (confidence: medium) — *`primary` sync requirement has no verifying
  acceptance criterion* — Requirements. Requirements state the build verbs keep
  `primary` in sync, but no criterion verifies `primary` in this slice — notably
  whether a reopen flips `primary` back to `brief.md` or leaves it at
  `synthesis.md`. Suggestion: add a criterion pinning `primary` across the
  relevant transitions, or note it is unchanged from 0277 and out of scope here.
- 🔵 **Minor** (confidence: low) — *Expected `round_count` value not pinned in
  reconciliation criterion* — Acceptance Criteria (Checkbox and count
  reconciliation). `finding_count` has a definitive expected value, but
  `round_count` counts rounds, so "match the findings on disk" does not pin the
  value a verifier should assert. Suggestion: state how `round_count` is derived
  (e.g. highest `round` stamped on any finding). (Merged into the cross-cutting
  `round_count` finding above.)
- 🔵 **Suggestion** (confidence: low) — *"No staleness field written anywhere" has
  unbounded verification scope* — Acceptance Criteria (Reopen is the sole
  staleness signal). "Anywhere" is unbounded, leaving no defined set of files to
  scan to confirm the negative. Suggestion: bound to the set's documents
  (`manifest.md`, `outline.md`, `synthesis.md`, any finding).
- 🔵 **Suggestion** (confidence: low) — *Finalise "refuses" has no specified
  observable signal* — Acceptance Criteria (Finalise refuses a non-synthesised
  set). "Mutates nothing" is verifiable via file comparison, but "refuses" has no
  specified manifestation a verifier can assert on. Suggestion: specify the
  refusal signal (non-zero exit with a message naming the current status).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-19

**Verdict:** COMMENT

All seven majors, the minor, and every suggestion from the initial review were
addressed. The re-review across all five lenses found **0 critical, 0 major**
findings — the item drops from REVISE to COMMENT. The residual findings are
lower-severity refinements, several of them surfaced only because the edits
sharpened the criteria enough to expose finer gaps.

### Previously Identified Issues

- 🟡 **Clarity**: "Pending-round revision" assumes an `outlined` start — Resolved.
  AC now states status is unchanged (`outlined` or `researching` per reopen
  history); clarity confirms AC7/AC8 are scoped to disjoint starting states.
- 🟡 **Clarity + Testability**: `round_count` derivation unpinned / divergent —
  Resolved. Pinned to "highest `round` stamped on any finding"; Technical Notes
  reconciled; gap-fill leaves it unchanged.
- 🟡 **Dependency**: 0278 uncaptured upstream blocker — Resolved. Recorded as
  `relates_to` with rationale (0278 merged); dependency lens calls the capture
  "exemplary precision".
- 🟡 **Testability**: idempotent-synthesis "same dossier" undefined — Resolved.
  Re-anchored to unchanged `rounds_covered` + identical cited findings/sources +
  section structure, prose exempted from byte-identity.
- 🟡 **Testability**: "reads as one dossier" subjective — Resolved. Anchored to
  "no per-round headings / no round-sequencing language"; holistic readability
  deferred to the Slice 3 human gate.
- 🟡 **Testability**: lifecycle-transition mappings conflict with reopen /
  finalise-refusal — Resolved. Each mapping scoped to its starting state.
- 🟡 **Testability**: `primary` sync had no verifying criterion — Resolved. Added
  a `primary`-on-reopen criterion (stays `synthesis.md`) plus a Technical Note.
- 🔵 **Dependency**: 0283 not captured as a Blocks edge — Resolved. Noted as
  independent of this slice's `conduct`; a soft integration-ordering touchpoint
  remains (see below).
- 🔵 **Scope**: title/Summary "and" — Resolved. Coupling line added; scope lens
  now judges the bundle "not a genuine scope defect".
- 🔵 **Completeness**: Context omits the beneficiary — Resolved. Beneficiary
  clause added.
- 🔵 **Clarity**: terms used before gloss — Resolved. "Anti-changelog discipline"
  and "the knob" glossed on first use.
- 🔵 **Testability**: "no staleness field anywhere" unbounded — Resolved. Bounded
  to the named set documents.
- 🔵 **Testability**: finalise "refuses" no observable signal — Resolved. Now
  "exits non-zero with a message naming the current status".
- 🔵 **Dependency**: gap-detection `conduct` WebFetch coupling — Resolved.
  Restated in Assumptions.

### New Issues Introduced

Two were corrections to the pass-1 edits and are already fixed in this pass:
- 🔵 **Clarity**: `set` domain term collided with plain-English "same set of
  cited findings" in the idempotency criterion — Fixed (reworded to "the same
  cited findings and sources").
- 🔵 **Clarity + Completeness**: Drafting Notes said "ten" criteria but eleven
  exist — Fixed (note now records the eleventh as a Review 1 addition).

Three more were applied in the same collaborative pass:
- 🔵 **Testability** (minor): AC1 now pins the base `status` of an `outline`
  append onto a `researching`/`synthesised`/`complete` set (ends at
  `researching`) — Fixed.
- 🔵 **Testability** (minor): AC6 (single dossier) extended to cover the findings,
  not just `synthesis.md`, noting the `round` stamp is the immutable ledger, not
  the prose — Fixed.
- 🔵 **Dependency** (suggestion): the 0279/0283 shared-`conduct` integration
  touchpoint is now noted in Context (not a hard block; the later of the two
  carries a light integration cost) — Fixed.

Left open by decision (COMMENT-level, not blocking):
- 🔵 **Dependency** (minor): 0281 recorded reverse-only on its own `blocked_by`
  rather than on 0279's `blocks` — kept deliberately (author decision; pass 1
  endorsed this form), so 0279's `blocks` stays `[0282, 0284]`.
- 🔵 **Testability** (minor): AC6's "no round-sequencing language" keeps
  illustrative rather than closed examples — accepted as-is.
- 🔵 **Testability** (suggestion): AC2's "revises Round N in place" has no
  observable distinguishing a genuine revision from a no-op — accepted as-is.
- 🔵 **Scope** (suggestion): the accretion/finalise bundle is sound; if delivery
  runs long, `finalise` (verb + `complete` transition + gate) is the natural
  carve-out.

### Assessment

The work item is ready for implementation. Every blocking issue from pass 1 is
resolved, the verdict is COMMENT — acceptable as-is — and the cheap pass-2
refinements were folded in. The few items left open are deliberate (the 0281
recording convention) or accepted low-value nits; none blocks planning.

## Final Verdict — APPROVE (2026-09-19)

Reviewer override: the pass-2 analytical verdict was COMMENT (no critical/major
findings, a few accepted nits). The reviewer accepts the remaining nits and marks
the review **APPROVE** — the work item is cleared for implementation. The
frontmatter `verdict` reflects this final decision.
