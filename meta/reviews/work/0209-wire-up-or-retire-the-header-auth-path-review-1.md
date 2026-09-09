---
type: "work-item-review"
id: "0209-wire-up-or-retire-the-header-auth-path-review-1"
title: "Work Item Review: Wire Up The Browser Auth-Header Path In Design Skills"
date: "2026-09-09T23:07:27+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0196"
target: "work-item:0209"
work_item_id: "0209"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-10T00:12:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Wire Up The Browser Auth-Header Path In Design Skills

**Verdict:** REVISE

This is a structurally complete, richly populated bug item — every expected
section is present and substantive, the reproduction and expected-vs-actual
content a bug demands is there, and the acceptance criteria are mostly cast as
observable Given/When/Then outcomes. Two structural issues nonetheless warrant
revision before implementation: the wire-up-versus-retire direction is asserted
as settled and treated as still-pending in the same document, and the item's
central cross-skill promise ("every browser-driving design skill") is unbounded
over a set the item itself admits is undefined. Both are load-bearing enough
that an implementer could either start work that later reverses or claim a
criterion passed that was never genuinely verifiable.

### Cross-Cutting Themes

- **Wire-up-versus-retire direction is simultaneously settled and unsettled**
  (flagged by: clarity, scope) — The Summary states the direction as decided
  ("wires the path up rather than retiring it") and the Requirements dropped
  the retire fork, yet AC1, Assumptions, and Drafting Notes all treat the
  decision as pending confirmation. Wire-up and retire are radically different
  units of work, so an unresolved fork blocks confident planning and sizing.
- **The set of browser-driving design skills is undefined but load-bearing**
  (flagged by: testability, dependency, scope) — Open Question 2 asks which
  design skills drive a browser, while AC5 requires "every browser-driving
  design skill" to honour the header and Requirements demand covering them all.
  A verifier cannot confirm coverage of an unenumerated set, and a bypassing
  skill would silently escape the fix.
- **Cross-origin asset handling left undecided as in/out of scope** (flagged
  by: scope, testability) — Open Question 4 asks whether stripping the token on
  legitimate cross-origin resources (e.g. a CDN asset) is in scope and leaves
  it open. AC3's strip can pass while a crawl depending on cross-origin assets
  silently degrades, with nothing testing for it.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Wire-up direction stated as settled in some sections, unsettled in others
  **Location**: Summary / Acceptance Criteria / Assumptions
  The Summary asserts the direction as decided while AC1 ("The decision to wire
  up ... is recorded ... before implementation begins"), Assumptions ("Confirm
  if that framing is wrong"), and Drafting Notes ("If retire is still on the
  table, the fork should be reinstated") all treat it as pending. A reader
  cannot tell whether to proceed or first pause to re-confirm.

- 🟡 **Scope**: Chosen direction still contingent, so the whole unit of work could invert
  **Location**: Acceptance Criteria
  The item commits to wiring up, but AC1 still gates the decision and
  Assumptions/Drafting Notes treat it as unconfirmed. Wire-up adds a cross-skill
  capability; retire deletes a code path and its `resolve-auth` surface — if the
  direction flips at kickoff, essentially the entire scope and criteria are
  replaced.

- 🟡 **Testability**: 'Every browser-driving design skill' is unbounded over an undefined set
  **Location**: Acceptance Criteria
  AC5 requires "every browser-driving design skill" to honour the header, but
  the set is never enumerated — Open Question 2 explicitly asks which skills
  drive a browser. The criterion can be claimed passed or failed regardless of
  what was wired, giving no verification value for the item's core cross-skill
  intent.

- 🟡 **Testability**: Auth-gated source input is never concretely specified
  **Location**: Reproduction / Acceptance Criteria
  Both the Reproduction ("a source whose content sits behind that header") and
  AC2 ("the source is auth-gated") depend on an auth-gated crawl target, but no
  concrete fixture, server, or gating mechanism is defined. Two testers could
  build different harnesses and reach different conclusions about whether "gated
  pages appear".

#### Minor

- 🔵 **Dependency**: Downstream consumer skills of the shared daemon lib left unresolved, not enumerated
  **Location**: Dependencies
  Requirements demand covering "every design skill that drives a browser" and
  confirming none drive one by another route, but the consumer skills are left
  as Open Question 2 rather than enumerated as couplings. A skill bypassing the
  shared daemon lib would silently escape the fix, invisibly to the dependency
  record.

- 🔵 **Scope**: A net-new capability (0243) folded into an item labelled a bug
  **Location**: Frontmatter: kind / Drafting Notes
  The item is `kind: bug` but folds in 0243 (an independently-wanted capability)
  and broadens the target from `inventory-design` to every browser-driving
  skill. The bug label understates the delivery footprint and risks
  under-sizing.

- 🔵 **Scope**: Cross-origin asset handling left explicitly undecided as in/out of scope
  **Location**: Open Questions
  Open Question 4 ends with "is that in scope here?" — an unresolved
  in/out-of-scope boundary. Leaving it undecided lets scope expand mid-flight or
  get silently dropped, either of which changes what "done" means.

- 🔵 **Testability**: Recorded-decision criterion names no artefact or location
  **Location**: Acceptance Criteria
  AC1 requires the wire-up decision to be "recorded with its reasoning" but does
  not say where, so there is no defined place to check for a pass. Assumptions
  and Drafting Notes already carry the reasoning, leaving it unclear whether the
  criterion is already met.

- 🔵 **Testability**: 'Replaced to match the wired-up behaviour' lacks a defined check
  **Location**: Acceptance Criteria
  AC6's three named files are concrete and checkable, but "match the wired-up
  behaviour" has no content criterion, so a reviewer cannot objectively decide
  whether a replacement adequately matches.

#### Suggestions

- 🔵 **Clarity**: 'the resolved `[location]`' bracket notation used without in-line gloss
  **Location**: Requirements
  "Source the allowlist origin from the resolved `[location]`" uses bracket
  notation whose meaning is only recoverable from the Technical Notes or the
  linked 0196 plan. A reader working from Requirements alone may stumble before
  finding the gloss lower down.

- 🔵 **Completeness**: `kind: bug` reads substantially as an enhancement
  **Location**: Frontmatter: kind
  The item satisfies bug content requirements but wires up a never-functional
  path, folds in feature item 0243, and gates work behind a recorded decision —
  reading as net-new capability. Confirm `bug` is intended; `story` may fit
  better.

- 🔵 **Dependency**: Shared `leaked_credentials` surface with ready item 0207 names the overlap but not the ordering
  **Location**: Dependencies
  Both 0209 and 0207 are "ready" and modify `leaked_credentials.rs`, but no
  ordering or coexistence note states whether they proceed in parallel or which
  must land first — a merge collision waiting to be discovered late.

- 🔵 **Testability**: No criterion verifies legitimate cross-origin assets still load
  **Location**: Acceptance Criteria
  Open Question 4 raises that stripping the token must not break cross-origin
  asset loading, but no criterion captures it. AC3 can pass while a crawl
  depending on cross-origin assets silently degrades.

### Strengths

- ✅ Bug-kind content is fully met: explicit Reproduction steps and a dedicated
  Expected-vs-actual subsection covering both intended and observed outcomes.
- ✅ AC3 (cross-origin strip) is a model testable criterion — it mandates a
  mutation-resistant test that fails if the strip is removed, ruling out a
  happy-path-only pass.
- ✅ Pronouns and referents resolve cleanly throughout, and the primary-risk
  framing (silently-incomplete inventory over credential leakage) is stated
  identically across Context, Assumptions, and Drafting Notes.
- ✅ The upstream ordering constraint is captured: the auth-header route must
  register after the navigation-classifier route (0206), with the
  `route.fallback()` mechanism that makes the ordering load-bearing documented.
- ✅ The relationship to 0243 is fully captured — folded into scope with an
  explicit recommendation to abandon it as a duplicate.
- ✅ Optional sections that are genuinely relevant are all populated: Open
  Questions, Assumptions, Dependencies, Technical Notes, and Drafting Notes.

### Recommended Changes

1. **Reconcile the wire-up-versus-retire direction to one stance** (addresses:
   Wire-up direction stated as settled in some sections; Chosen direction still
   contingent; Recorded-decision criterion names no artefact) — Decide whether
   the direction is fixed or provisional and make every section agree. If fixed,
   reframe AC1 as recording the rationale for an already-made decision and point
   it at a concrete artefact (the Assumptions section or an ADR); soften
   Assumptions/Drafting Notes so they no longer reopen the fork. If provisional,
   resolve it as a prerequisite (or a tiny separate decision item) before this
   item is planned.

2. **Bound the "every browser-driving design skill" promise** (addresses:
   'Every browser-driving design skill' is unbounded; Downstream consumer skills
   left unresolved) — Resolve Open Question 2 during grooming. Either enumerate
   the browser-driving skills in Dependencies and test each, or restate AC5
   around the verifiable proposition already offered — "all browser-driving
   skills route through the shared Playwright daemon lib, and no skill drives a
   browser by another route" — which can be checked concretely.

3. **Pin a concrete auth-gated test fixture** (addresses: Auth-gated source
   input never concretely specified) — Name a concrete fixture in Reproduction
   or AC2 — e.g. a local server returning 401 without the bearer header and 200
   with it on a specific path — so "gated pages appear in the inventory" has a
   definite pass/fail.

4. **Decide cross-origin asset handling in or out of scope** (addresses:
   Cross-origin asset handling left undecided; No criterion verifies
   cross-origin assets still load) — Resolve Open Question 4 now. If out of
   scope, state it as a non-goal; if in scope, add a criterion asserting a crawl
   of a page with a cross-origin asset still loads that asset after the strip.

5. **Confirm the `kind` label given the folded-in capability** (addresses:
   Net-new capability folded into a bug; `kind: bug` reads as an enhancement) —
   Decide whether this is better modelled as a `story`/feature superseding 0243,
   or kept as a bug with the capability-delivery scope made explicit in the
   Summary.

6. **Add an ordering note for the shared `leaked_credentials` surface**
   (addresses: Shared surface with 0207 names the overlap but not the ordering)
   — State in Dependencies whether 0209 and 0207 are independent or one gates
   the other, so they can be sequenced deliberately rather than at merge.

7. **Gloss `[location]` at first use and tighten AC6's replacement content**
   (addresses: `[location]` bracket notation without gloss; 'Replaced to match
   the wired-up behaviour' lacks a defined check) — Keep the parenthetical gloss
   adjacent to the first `[location]` in Requirements, and state the minimum
   content AC6's replacement text must assert.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear: pronouns resolve cleanly, the risk
framing (silently-incomplete inventory) is stated consistently across Context,
Assumptions, and Drafting Notes, and the Acceptance Criteria use observable
Given/When/Then outcomes. The main clarity weakness is an internal tension over
whether the wire-up-versus-retire direction is settled or still to be confirmed,
which the Summary, Acceptance Criteria, and Assumptions treat differently.
Domain terms carried over from the linked 0196 plan (executor, location) are
defined by that reference and are not clarity problems.

**Strengths**:
- Pronouns and referents are handled well throughout — "it", "its", and "that"
  each resolve to a single previously-named subject.
- The primary-risk framing (silently-incomplete inventory over credential
  leakage) is stated identically across Context, Assumptions, and Drafting
  Notes, giving the item a single coherent intent on that axis.
- Acceptance Criteria state outcomes as observable system states in
  Given/When/Then form, and Technical Notes ground otherwise-jargon terms like
  `[location]` in concrete request fields.

**Findings**:
- 🟡 **major** (confidence: medium) — Wire-up direction stated as settled in
  some sections, unsettled in others. **Location**: Summary / Acceptance
  Criteria / Assumptions. The Summary asserts the direction as decided and the
  Requirements dropped the retire fork, yet AC1 treats the decision as pending,
  Assumptions says "Confirm if that framing is wrong", and Drafting Notes note
  "If retire is still on the table, the fork should be reinstated." A reader
  cannot tell whether the implementer may proceed or must first re-confirm.
  Suggestion: reconcile to one stance — either state the direction is fixed and
  reframe AC1 as recording rationale for an already-made decision, or state it
  is provisional and pending confirmation, but not both.
- 🔵 **suggestion** (confidence: low) — "the resolved `[location]`" bracket
  notation used without in-line gloss. **Location**: Requirements. The
  square-bracket `[location]` reads as a config-section reference whose meaning
  is only recoverable from the Technical Notes or the linked 0196 plan.
  Suggestion: keep the parenthetical ("e.g. the first `navigate` URL's origin")
  adjacent to the first `[location]` use so the Requirements section stands on
  its own.

### Completeness

**Summary**: This is a structurally complete, richly populated bug work item.
Every expected section is present and substantive: the Summary states a clear
intent, Context explains the motivating problem, Requirements carry explicit
Reproduction and Expected-vs-Actual subsections, and Acceptance Criteria, Open
Questions, Dependencies, Assumptions, Technical Notes and Drafting Notes are all
populated with real content. Frontmatter integrity is sound, with a recognised
kind, status and priority all present.

**Strengths**:
- The bug-kind content requirement is fully met: explicit Reproduction steps and
  a dedicated Expected-vs-actual subsection.
- Acceptance Criteria are plentiful and specific (seven criteria in
  given/when/then form).
- Context explains why the work is needed rather than restating the summary, and
  distinguishes the primary risk from credential leakage.
- Optional sections that are genuinely relevant are all populated — Open
  Questions, Assumptions, Dependencies, Drafting Notes.
- Frontmatter is complete and coherent — type, id, title, status, kind,
  priority, parent and relationships all present and recognised.

**Findings**:
- 🔵 **suggestion** (confidence: low) — `kind: bug` reads substantially as an
  enhancement. **Location**: Frontmatter: kind. The item satisfies bug content
  requirements but wires up a never-functional path, folds in feature item 0243,
  and gates work behind a recorded decision. A reader may expect a defect-fix
  scope but encounter net-new capability work. Suggestion: confirm `bug` is
  intended; if the primary work is delivering new capability, consider `story`.

### Dependency

**Summary**: Dependency mapping in this bug is strong: the upstream ordering
constraint (the auth-header route must register after the navigation-classifier
route from 0206, done) is captured in Technical Notes, "Blocked by: none" is
justified by parent 0196 being done, and the folding-in of 0243 with a
recommendation to abandon it as a duplicate is explicitly recorded. Two
couplings are only partially captured: the set of downstream consumer skills
sharing the Playwright daemon lib is left as an open question rather than
enumerated, and the shared-file coupling with ready item 0207 on
leaked_credentials names the overlap but not its ordering/merge implication.

**Strengths**:
- The upstream ordering constraint is explicitly captured — the auth-header
  route must register after the navigation-classifier route (0206), with the
  `route.fallback()` mechanism that makes the ordering load-bearing documented.
- "Blocked by: none" is substantiated by naming parent epic 0196 as done.
- The relationship to 0243 is fully captured — folded into scope with an
  explicit recommendation to abandon it as a duplicate.
- The shared `leaked_credentials` surface with 0207 is named in Dependencies and
  the scrub rule flagged in Technical Notes.

**Findings**:
- 🔵 **minor** (confidence: medium) — Downstream consumer skills of the shared
  daemon lib left unresolved, not enumerated. **Location**: Dependencies. The
  Requirements demand covering every browser-driving design skill and confirming
  none drive a browser by another route, but these sibling skills are left as
  Open Question 2 rather than enumerated as couplings. If a skill bypasses the
  shared daemon lib, the fix silently fails to cover it, invisibly to the
  dependency record. Suggestion: resolve Open Question 2 during grooming and
  record the concrete list (or an explicit "inventory-design is the sole
  consumer") in Dependencies.
- 🔵 **suggestion** (confidence: medium) — Shared `leaked_credentials` surface
  with ready item 0207 names the overlap but not the ordering. **Location**:
  Dependencies. Both items are "ready" and modify `leaked_credentials.rs`, but
  no ordering or coexistence note states whether they proceed in parallel or
  which must land first. Suggestion: add a one-line ordering note stating whether
  0209 and 0207 are independent or one gates the other.

### Scope

**Summary**: As a unit of work, 0209 is largely coherent: every requirement
orbits a single subject — the dead browser auth-header path — and the item
sensibly collapses the original wire-up/retire fork into one committed
direction. The main scope tensions are that the chosen direction is still
flagged as needing confirmation (a retire decision would invert the entire
scope), that a net-new capability (0243) and a broadening from one skill to
"every browser-driving design skill" have been folded into an item labelled a
bug, and that at least one boundary (cross-origin asset handling) is explicitly
left undecided as in/out of scope.

**Strengths**:
- All requirements serve one unified purpose — resurrecting and correctly
  bounding the auth-header injection path.
- The Drafting Notes record that the original "wire up OR retire" fork was
  deliberately collapsed to a single direction.
- Open Questions and Assumptions explicitly surface the scope-boundary
  uncertainties rather than leaving them implicit.

**Findings**:
- 🟡 **major** (confidence: medium) — Chosen direction still contingent, so the
  whole unit of work could invert. **Location**: Acceptance Criteria. The item
  commits to wiring up, but AC1 still requires the decision to be recorded
  "before implementation begins", and Assumptions and Drafting Notes treat the
  direction as unconfirmed. Wire-up and retire are radically different units of
  work — one adds a cross-skill capability, the other deletes a code path and
  its `resolve-auth` surface. If the direction flips at kickoff, essentially the
  entire scope is replaced. Suggestion: resolve the decision as a prerequisite
  (or a tiny separate decision item) and remove the contingency.
- 🔵 **minor** (confidence: medium) — A net-new capability (0243) folded into an
  item labelled a bug. **Location**: Frontmatter: kind / Drafting Notes. The
  item is `kind: bug` but folds in 0243 (a wanted capability) and broadens the
  target from the `inventory-design` daemon to every browser-driving skill,
  mixing a defect fix with a capability increment. A bug label understates the
  delivery footprint. Suggestion: confirm whether this is better modelled as a
  story/feature superseding 0243, or keep it a bug with capability scope explicit
  in the Summary.
- 🔵 **minor** (confidence: medium) — Cross-origin asset handling left
  explicitly undecided as in/out of scope. **Location**: Open Questions. The
  fourth Open Question ends with "is that in scope here?" — an unresolved
  in/out-of-scope boundary rather than an implementation detail. Suggestion:
  decide now; if out, state it as a non-goal, and if a follow-up is warranted,
  reference it rather than carrying the question into implementation.

### Testability

**Summary**: For a bug, the item is unusually well-framed: several acceptance
criteria are cast as Given/When/Then behaviours and one is exemplary — a
mutation-style assertion that must fail if the cross-origin strip is removed. The
main testability gaps are an unbounded "every browser-driving design skill"
criterion over a set the item itself admits is undefined, and a
reproduction/criterion pair that never pins a concrete auth-gated source to test
against. Two further criteria (the recorded decision, the warning rewrite) lack
a defined pass/fail check.

**Strengths**:
- AC3 (cross-origin strip) is a model testable criterion: precondition, expected
  outcome, and a mandated mutation-resistant test that fails if the strip is
  removed.
- Most criteria are framed as observable Given/When/Then behaviours rather than
  implementation instructions, and the Reproduction section supplies an explicit
  expected-vs-actual contrast.
- AC4 pins a verifiable negative ("that variable is no longer read"), and AC7
  ("mise run exits 0") gives an unambiguous machine-checkable gate.

**Findings**:
- 🟡 **major** (confidence: high) — "Every browser-driving design skill" is
  unbounded over an undefined set. **Location**: Acceptance Criteria. AC5
  requires every browser-driving design skill to honour the header, but the set
  is never enumerated — Open Question 2 explicitly asks which skills drive a
  browser. The criterion can be claimed passed or failed regardless of what was
  wired. Suggestion: resolve Open Question 2 and either enumerate the skills or
  restate the criterion around "all browser-driving skills route through the
  shared Playwright daemon lib, and no skill drives a browser by another route".
- 🟡 **major** (confidence: medium) — Auth-gated source input is never
  concretely specified. **Location**: Reproduction / Acceptance Criteria. Both
  the Reproduction and AC2 depend on an auth-gated crawl target, but no concrete
  fixture, server, or gating mechanism is defined. Two testers could build
  different harnesses and reach different conclusions. Suggestion: name a
  concrete fixture — e.g. a local server returning 401 without the bearer header
  and 200 with it on a specific path.
- 🔵 **minor** (confidence: medium) — Recorded-decision criterion names no
  artefact or location. **Location**: Acceptance Criteria. AC1 requires the
  decision to be "recorded with its reasoning" but does not say where, so there
  is no defined place to check for a pass. Suggestion: point the criterion at a
  concrete artefact (an ADR or the item's Decision/Assumptions section), or drop
  it if the existing Drafting Notes already satisfy it.
- 🔵 **minor** (confidence: medium) — "Replaced to match the wired-up behaviour"
  lacks a defined check. **Location**: Acceptance Criteria. AC6's three named
  files are concrete and checkable, but "match the wired-up behaviour" has no
  content criterion, so a reviewer cannot objectively decide whether a
  replacement adequately matches. Suggestion: state the minimum content the
  replacement must assert, or split AC6 into "warning removed" plus "documented
  behaviour states X".
- 🔵 **suggestion** (confidence: low) — No criterion verifies legitimate
  cross-origin assets still load. **Location**: Acceptance Criteria. Open
  Question 4 raises that stripping the token must not break asset loading, but no
  criterion captures it. AC3 can pass while a crawl that depends on cross-origin
  assets silently degrades. Suggestion: resolve Open Question 4 as out of scope
  explicitly, or add a criterion asserting a cross-origin asset still loads after
  the strip.

## Re-Review (Pass 2) — 2026-09-09

**Verdict:** REVISE

Every finding from Pass 1 is resolved. The verdict holds at REVISE only because
two new major findings surfaced, both introduced by the Pass 1 edits and both
resolvable by wording tightening rather than any scope or decision change: an
acceptance criterion that unconditionally asserts single-origin while Open
Question 1 keeps that design open, and a restated coverage criterion that leans
on an unbounded "no skill drives a browser by another route" negative. The
remaining findings are refinements (merge-order for 0207, fixture precision for
the cross-origin-asset criterion) plus low suggestions.

### Previously Identified Issues

- 🟡 **Clarity**: Wire-up direction stated as settled in some sections, unsettled in others — Resolved (direction stated as settled across Summary, Assumptions, Drafting Notes; AC1 no longer a pre-implementation gate).
- 🟡 **Scope**: Chosen direction still contingent, so the whole unit could invert — Resolved (fork closed; direction settled).
- 🟡 **Testability**: 'Every browser-driving design skill' unbounded over an undefined set — Partially resolved (restated around the shared daemon lib, but the restatement introduced a new unbounded negative — see New Issues).
- 🟡 **Testability**: Auth-gated source input never concretely specified — Resolved (AC2 pins a 401-without / 200-with fixture on a gated path).
- 🔵 **Dependency**: Downstream consumer skills left unresolved, not enumerated — Resolved (consumer coupling now a first-class Dependencies entry).
- 🔵 **Scope**: Net-new capability (0243) folded into a bug — Resolved (reclassified to story; supersession recorded).
- 🔵 **Scope**: Cross-origin asset handling left undecided — Resolved (brought in scope: new AC + Requirement; Open Question 4 removed).
- 🔵 **Testability**: Recorded-decision criterion (AC1) names no artefact — Resolved (AC1 now points at Assumptions; flagged separately as now self-referential).
- 🔵 **Testability**: AC6 'replaced to match wired-up behaviour' lacks a defined check — Resolved (replacement content now specified).
- 🔵 **Clarity**: `[location]` bracket notation without gloss — Partially resolved (glossed as "first navigate URL's origin"; the concept itself is still undefined — see New Issues).
- 🔵 **Completeness**: `kind: bug` reads as enhancement — Resolved (reclassified to story with justification).
- 🔵 **Dependency**: Shared `leaked_credentials` surface with 0207 — ordering not stated — Partially resolved (coupling recorded as merge-only; explicit merge order still unstated — see New Issues).
- 🔵 **Testability**: No criterion verifies cross-origin assets still load — Resolved (new AC added).

### New Issues Introduced

- 🟡 **Clarity** (major): AC4/single-origin design asserted as settled while Open Question 1 still flags it unresolved. Requirements and AC commit unconditionally to a single-origin allowlist ("that variable is no longer read"), but Open Question 1 keeps the single-vs-dual-origin decision open, so a reader cannot tell whether to build single-origin unconditionally.
- 🟡 **Testability** (major): AC5 rests on an unbounded negative ("no design skill drives a browser by another route") with no defined verification procedure — a manual audit, not a repeatable check. Suggested fix: enumerate browser-driving skills + a defined grep for browser-launch entry points that must return only the daemon lib.
- 🔵 **Completeness** (suggestion): AC1 is now self-referential — it checks for rationale that already exists in the same document rather than an implementation outcome; drop or recast it.
- 🔵 **Clarity** (minor): `[location]` still undefined as a concept (config section vs `resolve-auth` field); define it on first use.
- 🔵 **Dependency** (minor): 0207 merge order acknowledged but left unspecified; state first-come-first-served-with-rebase or a fixed order.
- 🔵 **Testability** (minor): AC4 cross-origin-asset fixture and success signal underspecified relative to AC2; pin the endpoint and pass condition (HTTP 200, no Authorization header on that request).
- 🔵 **Suggestions**: 0243 closure untracked as a dependency action; 0206 route-ordering understated as "relates to"; form-login coexistence could widen allowlist scope; "allowlist" names a single-origin check; actor called both "designer" and "Users".

### Assessment

The work item is materially stronger than at Pass 1 — all four original majors
and every original minor are cleared. The two residual majors are self-inflicted
by the Pass 1 edits and need only wording tightening (make the single-origin
design explicitly contingent on Open Question 1; replace AC5's negative
existential with a bounded enumerate-plus-grep procedure); neither requires a
scope or decision change. One more short iteration should reach APPROVE.

## Re-Review (Pass 3) — 2026-09-09

**Verdict:** COMMENT

Both Pass 2 majors are resolved. The verdict lifts from REVISE to COMMENT: one
lone major remained at review time (testability, AC4's conditional tail), and it
was itself an over-correction of a Pass 2 fix. That major, plus every actionable
minor from this pass, was closed by immediate follow-up edits (see below), so no
finding at or above the REVISE threshold stands. The residue is low-severity
polish.

### Previously Identified Issues (Pass 2 majors)

- 🟡 **Clarity**: AC4/single-origin asserted as settled while Open Question 1 open — Resolved (Requirements state the assumption; Assumptions carve dual-origin to a follow-up; Open Question 1 now says the item proceeds single-origin).
- 🟡 **Testability**: AC5 unbounded negative ("no skill by another route") — Resolved (restated as an enumerate-plus-search over concrete patterns).

### New Issues Introduced (and their disposition)

- 🟡 **Testability** (major): AC4's conditional tail ("Should Open Question 1 resolve to dual-origin, this criterion gains the login origin") left the pass/fail boundary indeterminate — Fixed post-review: the tail is dropped; the criterion now leads with the grep-verifiable "`ACCELERATOR_BROWSER_LOCATION_ORIGIN` is no longer read" and defers behaviour to the earlier ACs; dual-origin lives only in the Assumptions follow-up carve-out.
- 🔵 **Clarity** (minor): "Open Question 1" referenced an unnumbered list — Fixed (Open Questions now numbered 1., 2.).
- 🔵 **Testability** (minor): AC5 search pattern not pinned — Fixed (search now names the Playwright launch call and the daemon-lib import path).
- 🔵 **Testability** (minor): origin-sourcing AC phrased as implementation, not outcome — Fixed (AC now leads with the grep-verifiable negative; sourcing mechanism demoted to a gloss).
- 🔵 **Dependency** (minor): browser runtime prerequisite for the tests uncaptured — Fixed (Dependencies notes system Node ≥20 + lockhash namespace).
- 🔵 **Scope** (suggestion): "handle it explicitly" escape hatch — Fixed (a discovered bypassing launch site is now carved to a follow-up, scope fixed at the shared lib).
- 🔵 **Clarity** (suggestion): "the executor" undefined — Fixed (glossed as the design skill's Playwright-driving caller in Open Question 2).
- 🔵 **Dependency** (suggestion): 0243 dependants not checked before abandon — Fixed (supersedes note now requires confirming/re-pointing dependants).

### Residual (low, not addressed)

- 🔵 **Completeness** (suggestion): status `ready` alongside two open questions — partially eased by Open Question 1 now stating the item proceeds single-origin.
- 🔵 **Clarity** (suggestion): "resolved location" appears in Context before `[location]` is defined in Requirements.
- 🔵 **Scope** (suggestion): warning-removal spans a distinct Rust surface — flagged as a non-defect; no split needed.
- 🔵 **Testability** (suggestion): 0243 supersession has no acceptance criterion — left as a post-completion process step.

### Assessment

The work item is implementation-ready. Every original Pass 1 finding, both Pass
2 majors, and the single residual Pass 3 major are closed; what remains is
low-severity wording polish with no bearing on whether the work can be planned
or verified. A confirming Pass 4 would likely return APPROVE, but the marginal
value is low — the item can proceed to planning as it stands.

## Approval — 2026-09-10

**Verdict:** APPROVE

Reviewer approved the work item, judging the four residual low-severity
suggestions non-blocking. No further changes required; the item is ready for
planning. This overrides the Pass 3 COMMENT verdict without a further review
pass.
