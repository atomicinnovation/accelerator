---
type: work-item-review
id: "0091-typography-rem-vs-px-stance-review-1"
title: "Work Item Review: Typography rem-vs-px stance review"
date: "2026-06-12T22:08:58+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0091"
work_item_id: "0091"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-06-12T22:08:58+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Typography rem-vs-px stance review

**Verdict:** REVISE

This is an unusually strong, well-cross-referenced spike: the decide-not-migrate
intent is asserted consistently, the px/zoom-vs-default-font-size distinction is
defined precisely, the Q1–Q4 research questions map cleanly onto AC1–AC4, and the
orthogonality with 0099 (unit axis vs naming axis) is worked out on both sides.
The REVISE verdict is driven by three major findings rather than any structural
weakness: two dependency gaps (the spike gates not-yet-raised follow-on
implementation items with no Blocks entry, and the binding 0033 CI gate is absent
from Dependencies) and one testability gap (AC1's "answered with each source's
limits cited" has no completeness bar). All three are tractable edits, not
rework.

### Cross-Cutting Themes

- **Time-box / effort bound** (flagged by: clarity, completeness, scope,
  testability) — Four of the five lenses converged on the time-box. Clarity sees
  a contradiction ("A time-boxed investigation" in the Requirements preamble vs
  "Not hard-time-boxed" in the Time-box subsection); completeness, scope, and
  testability each see the *absence* of an effort bound as a watch-item, made
  sharper because AC1's research-sufficiency condition is itself soft, leaving
  the deferral path with no objectively checkable stopping point. This is the
  single most-reinforced issue in the review.
- **Downstream / follow-on coupling — capture it, but don't over-scope it**
  (flagged by: dependency, scope) — Two lenses pull in opposite directions on the
  same seam. Dependency wants the follow-on implementation work the spike gates
  made *visible* (a Blocks entry), while scope warns that AC4 + the detailed
  Technical Notes blast-radius analysis risk the spike *doing* the follow-on's
  scoping and blurring the decide/migrate line. The reconciling move is to record
  a downstream coupling without fully specifying the follow-on.
- **0099 / ADR chaining coordination** (flagged by: clarity, dependency,
  testability) — Three lenses touch the 0099 successor-ADR relationship from
  different angles: clarity flags the settled-vs-open tension (Dependencies
  states 0099 supersedes ADR-0036 as fact while Open Questions reopens ADR
  ownership), dependency notes AC3 depends on 0099's successor ADR existing if
  0099 lands first, and testability notes "chain rather than fork" has no
  observable check.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: Spike gates follow-on implementation work items but lists no Blocks entries
  **Location**: Dependencies
  This is a spike whose stated outputs (Summary; exit-criterion 3; AC4) include
  "one or more follow-on work items … to implement it", but the Dependencies
  section names only upstream/coordinating couplings (0075, 0099) with no Blocks
  entries naming the downstream work the decision gates — so the follow-on work is
  invisible in the record until the spike closes.

- 🟡 **Dependency**: 0033 CI gate is a hard constraint on the chosen stance but absent from Dependencies
  **Location**: Dependencies
  Q3 and Technical Notes make 0033's CI gate (banning inline px/rem literals,
  requiring token-only consumers and byte-identical computed sizes) a binding
  constraint that can invalidate candidate options, yet 0033 appears only in
  References and Technical Notes — not in Dependencies nor in frontmatter
  `relates_to`.

- 🟡 **Testability**: AC1 "each source's limits cited" has no completeness bar
  **Location**: Acceptance Criteria: AC1 / Requirements: Q1
  AC1 requires Q1–Q4 to be "answered … with each source's limits cited" but
  defines no bar for what counts as "answered". A generous reader could mark AC1
  done with a one-line estimate; a strict reader could reject it indefinitely —
  no objective completion check exists.

#### Minor

- 🔵 **Clarity**: Contradiction — "time-boxed investigation" vs "Not hard-time-boxed"
  **Location**: Requirements: Time-box
  The Requirements preamble opens with "A time-boxed investigation" while the
  Time-box subsection immediately below states "Not hard-time-boxed". A reader
  cannot tell whether a deadline applies.

- 🔵 **Clarity**: ~3% figure attributed to two different sources (HTTP Archive vs Internet Archive)
  **Location**: Research questions: Q1
  The load-bearing ~3% custom-default-font-size figure is cited as "(HTTP Archive
  / Internet Archive)" in Q1 but as "Internet Archive data" in Context,
  Assumptions, and the Evan Minto reference. These are distinct organisations, so
  the provenance of the named AC1 floor is ambiguous.

- 🔵 **Dependency**: AC3's dependence on 0099's successor ADR is surfaced as an open question, not captured as a blocker
  **Location**: Open Questions
  AC3 records the decision "against (ADR-0036 or 0099's successor)"; if 0099 lands
  first (as both items sequence it to), that successor ADR is a prerequisite
  artefact. The dependency is raised in Open Questions rather than captured in
  Dependencies.

- 🔵 **Scope**: Follow-on work-item scoping risks bleeding migration work into the spike
  **Location**: Outputs and exit criteria / AC4
  AC4 + the detailed blast-radius analysis in Technical Notes (the ~19 token
  definitions, gate-exemption mechanics, 0099 interaction) ask the spike to do
  enough migration scoping that the deliberately-drawn decide/migrate line is
  partially blurred.

- 🔵 **Testability**: AC2 "justified by" is verifiable only as presence-of-justification
  **Location**: Acceptance Criteria: AC2
  AC2 requires the stance be "justified by" the Q1 estimate and the Q3 check; a
  verifier can confirm a justification exists and name-drops Q1/Q3 but cannot
  objectively confirm the reasoning is sound.

- 🔵 **Testability**: Q4/AC3 "ADRs chain rather than fork" lacks an observable check
  **Location**: Requirements: Q4 / Acceptance Criteria: AC3
  "Chain rather than fork" is a structural property with no stated observable
  check (e.g. which supersedes/relates-to links must be present), so a verifier
  cannot distinguish a true chain from a second-fork ADR that merely mentions
  0099.

- 🔵 **Testability**: Absence of a time-box removes the only stopping bound when AC1's bound is itself soft
  **Location**: Requirements: Time-box
  With no duration bound and a soft AC1 research-sufficiency condition, "when is
  this spike done?" has no definitive answer on the deferral path.

#### Suggestions

- 🔵 **Completeness**: Deliberate absence of a hard time-box / effort constraint
  **Location**: Requirements: Time-box
  A documented, justified choice (bounded by exit criteria) rather than an
  oversight, but a soft effort cap would bound the Q1 triangulation, which leans
  on sparse data.

- 🔵 **Clarity**: "review" / "spike" / "this work item" used interchangeably
  **Location**: Summary (and throughout)
  Residual "review" wording from an earlier task framing (per Drafting Notes)
  could briefly read as a separate review activity rather than the spike itself.

- 🔵 **Clarity**: Tension between "0099 supersedes ADR-0036" as settled vs ADR ownership as open
  **Location**: Dependencies / Open Questions
  Dependencies states the supersession as fact while Open Questions reopens
  ownership; scoping the Open Question to the *unit*-axis ADR (Q4's distinction)
  would remove the apparent contradiction.

- 🔵 **Dependency**: 0090 name-checks this spike but the reciprocal coupling is not in Dependencies/relates_to
  **Location**: References
  A precedent reference (0090 → 0091) recorded only in References prose; adding
  0090 to `relates_to` would make the inbound link navigable. No scheduling
  action implied.

- 🔵 **Scope**: Spike with no time-box leans on exit criteria alone to bound effort
  **Location**: Frontmatter: kind / Time-box
  Exit criteria bound *what* is done but not *how much* effort; a light 1–2 day
  cap would anchor sizing for a low-priority, ~3%-cohort trade-off.

### Strengths

- ✅ The decide-not-migrate boundary — "This spike *decides* — it does not
  migrate" — is asserted in the Summary and consistently reinforced in
  Requirements, Outputs/exit criteria, and AC4, so the spike cannot be mistaken
  for an implementation task.
- ✅ The central technical distinction (px honours page zoom and so passes WCAG
  1.4.4, but ignores the browser/OS default-font-size preference which only
  rem/em/% scale) is defined precisely in Context and re-stated identically in
  Technical Notes, correcting the common "px is an accessibility failure" premise.
- ✅ The research question is specific and bounded: Q1–Q4 each name exactly what
  is being decided and converge on a single stance choice from a closed option
  set (keep px / rem headings / rem family-wide / hybrid), and they map cleanly
  onto AC1–AC4.
- ✅ Orthogonality with 0099 is explicitly worked out and mutually consistent
  (0099 = naming axis and supersedes ADR-0036; 0091 = unit axis and chains off
  0099's successor), and Technical Notes bounds the blast radius as disjoint from
  0099's rename (~19 token definitions, not ~100 consumer sites).
- ✅ The upstream blocker 0075 is captured with its resolved status and the stale
  `blocked_by` frontmatter was cleared per the Drafting Notes — the prerequisite
  and its discharge are both visible.
- ✅ Conditional branches are fully specified: AC4 defines both the change-decided
  and keep-px outcomes, and AC3 covers both decide-now and defer — no branch is
  left ambiguous about what "done" means.
- ✅ Frontmatter is intact and complete: recognised `kind: spike`, appropriate
  `draft` status, populated `relates_to`, and a thorough References section.

### Recommended Changes

1. **Resolve the time-box across the document** (addresses: clarity "time-boxed vs
   Not hard-time-boxed"; completeness, scope, and testability time-box findings)
   Pick one framing and apply it everywhere. The cleanest fix that satisfies all
   four lenses: drop "time-boxed" from the Requirements preamble in favour of
   "exit-criteria-bounded", **and** add a light soft effort cap (the work item
   already invites this: "A duration can be set if preferred"). The effort cap
   also gives the deferral path a verifiable stopping condition, which partly
   addresses the testability concern.

2. **Add the downstream and constraint couplings to Dependencies** (addresses:
   dependency "no Blocks entries" and "0033 CI gate absent")
   Add a Blocks-style note stating the spike conditionally gates one-or-more
   not-yet-raised implementation work items per AC4, so the downstream work is
   visible before the spike closes. Separately, add 0033 to Dependencies as a
   constraint coupling ("any chosen stance must pass 0033's inline-literal CI gate
   and determinism requirement") and add 0033 to frontmatter `relates_to` for a
   bidirectional link. Keep the entries to *coupling notes*, not full follow-on
   specs, to avoid the scope-bleed the scope lens flags.

3. **Harden AC1's completeness bar** (addresses: testability "AC1 has no
   completeness bar")
   Make "answered" checkable, e.g. "each of Q1–Q4 has a written answer; Q1 cites
   at least the three named source classes (general-population floor, developer
   behavioural signal, low-vision survey) and states each source's limitation; Q3
   records a pass/fail against each of its four named sub-conditions."

4. **Settle the ~3% figure's provenance** (addresses: clarity "HTTP Archive vs
   Internet Archive")
   Use a single attribution for the ~3% figure (matching what the Evan Minto
   article actually cites) consistently across Q1, Context, and Assumptions.

5. **Tighten the 0099 ADR-coordination language** (addresses: clarity
   "settled vs open" tension; dependency "AC3 depends on 0099's successor";
   testability "chain rather than fork lacks a check")
   Scope the Open Question to the *unit*-axis ADR so it doesn't reopen the settled
   naming-axis supersession; promote AC3's dependence on 0099's successor ADR into
   Dependencies as a conditional prerequisite ("chains off 0099's successor if it
   has landed, else off ADR-0036 directly"); and give AC3 an observable check for
   "chain not fork" (e.g. the new ADR's supersedes link points at the named
   predecessor and no two ADRs claim the same predecessor on the unit axis).

6. **(Optional) Standardise self-reference and link 0090** (addresses: clarity
   "review/spike interchange"; dependency "0090 reciprocal coupling")
   Standardise on "this spike" in the body, and optionally add 0090 to
   `relates_to` so the precedent link is navigable.

## Per-Lens Results

### Clarity

**Summary**: This is an unusually clear and internally well-cross-referenced work
item: the spike-decides-not-migrates intent is stated consistently across Summary,
Requirements, and Acceptance Criteria, the px/zoom-vs-default-font-size distinction
is defined precisely, and the rem/px/WCAG vocabulary is used consistently with
supporting links. Two genuine clarity issues remain: a terminology contradiction
about whether the spike is time-boxed, and an inconsistent source attribution for
the load-bearing ~3% figure that flips between "HTTP Archive" and "Internet
Archive". A few minor referent inconsistencies (review vs spike framing, the
settled-vs-open status of the ADR chain) are worth tightening but do not block
comprehension.

**Strengths**:
- The core scope statement — "This spike *decides* — it does not migrate" — is
  asserted in the Summary and consistently reinforced in the Requirements
  preamble, the Outputs/exit criteria, and AC4, so a reader cannot mistake the
  spike for an implementation task.
- The central technical distinction (px honours browser/page zoom and so passes
  WCAG 1.4.4, but ignores the browser/OS default-font-size preference which only
  rem/em/% scale) is defined precisely in Context and re-stated identically in
  Technical Notes, removing the common ambiguity around "px is an accessibility
  failure".
- Acronyms and specialised terms (WCAG 1.4.4/1.4.10 with success-criterion
  numbers, USWDS, rem/em) are each backed by a link in References or are standard
  frontend/accessibility vocabulary.
- The four research questions (Q1–Q4) map cleanly onto the four acceptance
  criteria (AC1–AC4), giving each requirement an unambiguous corresponding
  outcome.

**Findings**:
- 🔵 **minor** (confidence: high) — *Requirements: Time-box* — **Contradiction:
  "time-boxed investigation" vs "Not hard-time-boxed".** The Requirements preamble
  opens with "A time-boxed investigation whose job is to reduce uncertainty", but
  the Time-box subsection immediately below states "Not hard-time-boxed; bounded by
  the exit criteria below." A reader cannot tell whether a time limit applies.
  Suggestion: pick one framing — drop "time-boxed" from the preamble in favour of
  "exit-criteria-bounded", or state the box explicitly. The Drafting Notes already
  record the "no time-box" preference.
- 🔵 **minor** (confidence: high) — *Research questions: Q1* — **~3% figure
  attributed to two different sources (HTTP Archive vs Internet Archive).** Q1
  cites "(HTTP Archive / Internet Archive)" while Context, Assumptions, and the
  Evan Minto reference all attribute it to "Internet Archive data". These are
  distinct organisations, so which dataset backs the AC1 floor is unclear. This
  figure is the quantitative anchor for the trade-off and the named AC1 floor.
  Suggestion: settle on a single attribution matching what the Evan Minto article
  cites, used consistently in Q1, Context, and Assumptions.
- 🔵 **suggestion** (confidence: medium) — *Summary* — **"review" / "spike" /
  "this work item" used interchangeably for the same artefact.** The item refers
  to itself as a "spike", "this review" (Dependencies, Open Questions, title), and
  "this work item"; the "review" wording appears residual from an earlier task
  framing (per Drafting Notes). A reader who knows the team also runs formal
  "reviews" could briefly mistake "this review" for a separate activity.
  Suggestion: standardise on one self-reference, or confirm the title's wording is
  intentional.
- 🔵 **suggestion** (confidence: medium) — *Dependencies* — **Tension between "0099
  supersedes ADR-0036" as settled vs ADR ownership as open.** Dependencies states
  as fact that 0099 supersedes ADR-0036 and is "decided and ready", while Open
  Questions asks who owns the ADR and whether 0099 pre-empts this decision. The two
  read as partly contradictory unless the reader infers the naming-axis (settled)
  vs unit-axis (open) distinction. Suggestion: scope the Open Question explicitly
  to the unit-axis ADR (Q4 already draws this distinction).

### Completeness

**Summary**: This spike is structurally complete and well-populated for its kind:
it carries an explicit scoped question set (Q1–Q4), enumerable exit criteria,
kind-appropriate deliverables (research artefact, ADR, follow-on implementation
items), and four substantive acceptance criteria. Frontmatter is intact with a
recognised kind and an appropriate draft status. The only completeness observation
worth noting is the deliberate absence of a hard time-box/effort constraint — a
watch-item for spikes, though here it is a documented, justified choice (bounded by
exit criteria) rather than an oversight.

**Strengths**:
- All expected sections are present and substantively populated: Summary, Context,
  Requirements, Acceptance Criteria, Dependencies, Assumptions, Open Questions,
  Technical Notes, and References.
- Spike-specific content is strong: the scoped question is split into four
  explicit, enumerable research questions (Q1–Q4), and exit criteria are concrete
  deliverables/decisions rather than vague "understand X" goals.
- The Summary unambiguously states intent and explicitly bounds scope — the spike
  decides and does not migrate, with outputs named up front.
- Acceptance Criteria contain four specific items (AC1–AC4) that map cleanly to
  the research questions and exit criteria, including the conditional "keep px"
  branch.
- Context explains the motivation (the 0075 px-anchored trade-off documented as a
  known consequence in ADR-0036) rather than restating the summary.
- Frontmatter integrity is sound: kind is a recognised value (spike), status
  (draft) is appropriate, and relates_to plus a full References section are
  populated.
- Dependencies, Assumptions, and Open Questions are all populated with relevant
  content.

**Findings**:
- 🔵 **suggestion** (confidence: high) — *Requirements: Time-box* — **Deliberate
  absence of a hard time-box / effort constraint.** The spike states it is "bounded
  by the exit criteria below" (echoed in Drafting Notes). For the spike kind a
  time-box/effort constraint is an expected completeness element and its absence is
  a recognised watch-item; here it is a documented, justified choice since the
  enumerable exit criteria supply a clear completion definition. Without an effort
  ceiling, a research spike whose Q1 leans on triangulating sparse data could
  expand open-endedly. Suggestion: optionally add a soft effort cap (a
  day/half-day budget), as the work item itself invites.

### Dependency

**Summary**: The work item is unusually well-dependency-mapped for a spike: its
upstream blocker (0075, now landed) is captured as resolved, the 0099 coordination
is named with an explicit sequencing direction that matches 0099's own reciprocal
statement, and ADR-0036 is tracked in relates_to. The principal gaps are
downstream-coupling captures: as a spike whose stated output is one or more
follow-on implementation work items, it lists no Blocks entries, and the 0033
CI-gate constraint that any chosen stance must satisfy is named in
Requirements/Technical Notes but absent from the Dependencies section. A secondary
concern is that AC3 makes the spike's recorded decision depend on 0099's successor
ADR if 0099 lands first, a blocker the Open Questions surface as unresolved rather
than capture in Dependencies.

**Strengths**:
- The upstream blocker 0075 is explicitly captured in Dependencies with its
  resolved status, and the stale `blocked_by` frontmatter was cleared per the
  Drafting Notes — the prerequisite and its discharge are both visible.
- The 0099 coordination is captured with an explicit sequencing direction
  ("sequence after 0099"), reciprocally consistent with 0099's own Dependencies
  ("Sequence this remap first; 0091's later decision supersedes this ADR on the
  unit axis") — the ordering constraint is mapped on both sides without
  contradiction.
- Technical Notes explicitly bounds the blast radius of a unit change and states
  it is "disjoint from 0099's rename" (edits ~19 token definitions, not the ~100
  consumer sites), pre-empting a hidden collision.

**Findings**:
- 🟡 **major** (confidence: high) — *Dependencies* — **Spike gates follow-on
  implementation work items but lists no Blocks entries.** The spike's stated
  outputs (Summary; exit-criterion 3; AC4) include "one or more follow-on work
  items … to implement it" — a token-definition unit change, a root max()/clamp()
  anchor, and/or build-time rem emission. Dependencies names upstream/coordinating
  couplings (0075, 0099) but contains no Blocks entries naming the downstream work
  the decision gates, so the follow-on items are invisible in the record until the
  spike closes and planners cannot see what the decision unblocks. Suggestion: add
  a Dependencies note (or Blocks entries) stating the spike conditionally blocks
  one-or-more not-yet-raised implementation work items per AC4.
- 🟡 **major** (confidence: high) — *Dependencies* — **0033 CI gate is a hard
  constraint on the chosen stance but absent from Dependencies.** Q3 and Technical
  Notes make 0033's CI gate (banning inline px/rem literals; requiring token-only
  consumers and byte-identical computed sizes) a binding constraint that can
  invalidate candidate options, yet 0033 appears only in References and Technical
  Notes — not in Dependencies nor in frontmatter relates_to. Suggestion: add 0033
  to Dependencies as a constraint coupling and to relates_to for a bidirectional
  link.
- 🔵 **minor** (confidence: medium) — *Open Questions* — **AC3's dependence on
  0099's successor ADR is surfaced as an open question, not captured as a
  blocker.** AC3 records the decision against "(ADR-0036 or 0099's successor)"; if
  0099 lands first (as both items sequence it to), the 0099 successor ADR is a
  prerequisite artefact that must exist before this spike records its decision.
  The Open Questions raise this as unresolved rather than capture it as a concrete
  upstream dependency. Suggestion: promote the chaining dependency into
  Dependencies as a conditional prerequisite.
- 🔵 **suggestion** (confidence: medium) — *References* — **0090 name-checks this
  spike but the reciprocal coupling is not in Dependencies.** References notes 0090
  (radius-tokens consumption) as a "parallel token precedent; name-checks this work
  item" — an inbound precedent reference recorded only in prose. Low impact (0090
  does not block or consume this spike's output). Suggestion: optionally add 0090
  to relates_to so the precedent linkage is bidirectional.

### Scope

**Summary**: 0091 is a well-bounded spike with a single coherent purpose: decide
the px-vs-rem unit stance for the typography `--size-*` tokens and record that
decision, explicitly deferring implementation to follow-on work items. The
decide-not-migrate boundary is clean, the research question is specific (four named
sub-questions converging on one stance choice), and the orthogonality with 0099
(which both work items confirm is the *naming* axis vs this spike's *unit* axis) is
well-articulated and mutually consistent. The only genuine scope concern is that
AC4's open-ended "raise one or more follow-on work items" clause and the embedded
Technical Notes blast-radius analysis push the spike toward doing migration scoping
that belongs to the follow-on, slightly blurring the otherwise-crisp decide/migrate
line.

**Strengths**:
- The spike's research question is specific and bounded: Q1–Q4 each name exactly
  what is being decided, converging on a single stance choice rather than an
  open-ended "investigate typography" exploration.
- The decide-not-migrate boundary is stated explicitly in the Summary and
  re-stated in Outputs/exit criteria and Drafting Notes — implementation is handed
  off to follow-on work items.
- Orthogonality with 0099 is explicitly worked out and mutually consistent (0099
  owns the naming axis and supersedes ADR-0036; 0091 owns the unit axis and chains
  off 0099's successor). Both work items independently describe the same
  separation, so the two units can be delivered/rolled back separately.
- Q4 plus Dependencies and Open Questions proactively resolve the most likely
  scope-collision risk (two competing direct successors to ADR-0036) by specifying
  the ADR chains off 0099's rather than forking.

**Findings**:
- 🔵 **minor** (confidence: medium) — *Outputs and exit criteria / AC4* —
  **Follow-on work-item scoping risks bleeding migration work into the spike.** AC4
  and exit-criterion 3 require the spike to "raise one or more follow-on
  implementation work items" enumerating candidate scope, and combined with the
  detailed blast-radius analysis in Technical Notes the spike is asked to do enough
  migration scoping that the deliberate decide/migrate line is partially blurred. A
  spike that produces fully-scoped implementation items has absorbed the planning
  half of the follow-on. Suggestion: keep AC4 to "a follow-on item is raised and
  linked" (a stub capturing the decision) and let that follow-on own its
  blast-radius scoping; treat the Technical Notes detail as decision-support
  evidence, not the follow-on's spec.
- 🔵 **suggestion** (confidence: low) — *Frontmatter: kind / Time-box* — **Spike
  with no time-box leans on exit criteria alone to bound effort.** The Time-box
  section sets no duration. For a spike the time-box is a primary sizing guardrail:
  exit criteria bound *what* counts as done, not *how much* effort the research
  consumes. Without an effort ceiling, the Q1 triangulation from sparse data could
  expand beyond what a low-priority, ~3%-cohort trade-off warrants. Suggestion: add
  a light effort bound (1–2 days), which the Drafting Notes already flags as an
  option.

### Testability

**Summary**: As a spike, this work item is largely well-constructed for
verification: its exit criteria name concrete artefacts (a research artefact under
meta/research/, a recorded ADR or deferral work item, and linked follow-on work
items) whose existence is objectively checkable. The main testability gaps are in
AC1/Q1, where "with each source's limits cited" lacks a defined completeness bar
and the audience estimate has no pass/fail threshold, and in AC2, where "justified
by" the Q1 estimate is verifiable only as "a justification exists" rather than "the
justification meets a defined standard". The conditional "keep px" branch and the
deferral branch are both well-specified, so the criteria avoid the common spike
trap of an unbounded exploration goal.

**Strengths**:
- Exit criteria are enumerable artefacts and decisions rather than open-ended
  exploration — AC1 names a research artefact under meta/research/, AC3 names an
  ADR or deferral work item, and AC4 names linked follow-on items.
- Conditional branches are fully specified with both outcomes defined: AC4 states
  what happens if a change is decided AND if "keep px" is decided, and AC3 covers
  both decide-now and defer outcomes.
- Q3 carries objectively checkable sub-conditions: WCAG 1.4.4 (200% via zoom),
  1.4.10 (reflow at 320 CSS px), token-only consumers (0033 CI gate), and
  byte-identical computed sizes at the default font-size.
- AC2 enumerates the closed option space (keep px / rem headings / rem family-wide
  / hybrid), so "a stance is chosen" is verifiable against a fixed set.

**Findings**:
- 🟡 **major** (confidence: high) — *Acceptance Criteria: AC1 / Requirements: Q1* —
  **AC1 "each source's limits cited" has no completeness bar.** AC1 requires Q1–Q4
  to be "answered … with each source's limits cited" but defines no bar for what
  counts as "answered". Q1 explicitly asks for "a reasoned estimate", so a verifier
  cannot mechanically confirm sufficiency — a generous reader could mark AC1 done
  with a one-line estimate; a strict reader could reject it indefinitely.
  Suggestion: make the bar checkable, e.g. "each of Q1–Q4 has a written answer; Q1
  cites at least the three named source classes and states each source's
  limitation; Q3 records a pass/fail against each of the four named
  sub-conditions".
- 🔵 **minor** (confidence: medium) — *Acceptance Criteria: AC2* — **AC2 "justified
  by" is verifiable only as presence-of-justification.** A verifier can confirm a
  justification paragraph exists and references Q1/Q3 but cannot objectively
  confirm the reasoning is sound — "justified by" admits any rationale that
  name-drops the two inputs. Suggestion: reframe to a checkable structural
  requirement (the ADR rationale states the Q1 value/range relied on and the Q3
  pass/fail per sub-condition, and explains why the chosen option beats each
  rejected one).
- 🔵 **minor** (confidence: medium) — *Requirements: Q4 / Acceptance Criteria: AC3*
  — **Q4/AC3 "ADRs chain rather than fork" lacks an observable check.** "Chain
  rather than fork" is a structural property with no stated observable check, so a
  verifier cannot distinguish a true chain from a second-fork ADR that merely
  mentions 0099. Suggestion: define the observable (the new ADR's supersedes/amends
  link points at 0099's successor — or ADR-0036 if 0099 has not landed — and no two
  ADRs claim to be direct successors of the same predecessor on the unit axis).
- 🔵 **minor** (confidence: high) — *Requirements: Time-box* — **Absence of a
  time-box removes the only bound when AC1's bound is itself soft.** The item is
  deliberately not hard-time-boxed; the exit criteria are the sole bound, yet AC1's
  research-sufficiency bound is itself soft, so there is no objectively checkable
  stopping point on the deferral path. Suggestion: either add a duration bound (the
  Drafting Notes note one "can be added") or tighten AC1 so the research-complete
  condition is itself a definitive pass/fail.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-12

**Verdict:** COMMENT

Re-ran all five lenses against the revised work item. Every major finding
from pass 1 is resolved — the dependency lens now cites the new
Blocks/0033/0099-prerequisite couplings as *strengths*, and the testability
lens cites the hardened AC1/AC2/AC3 as *strengths*. The one downgrade was a
**new major introduced by the pass-1 Dependencies rewrite** (a "sequence
after 0099" vs "if 0099 has not landed" contradiction), which was fixed
immediately after this re-review — see Assessment. With that fix, no critical
or major findings remain; the residual items are minor polish, hence COMMENT
(acceptable as-is).

### Previously Identified Issues

- 🔵 **Clarity**: Time-box contradiction ("time-boxed" vs "Not hard-time-boxed") — **Resolved** (preamble now "exit-criteria-bounded"; soft effort guideline added).
- 🔵 **Clarity**: ~3% figure attributed to two sources (HTTP Archive vs Internet Archive) — **Resolved** (aligned to "Internet Archive, per Evan Minto's analysis" with a confirm-during-research note).
- 🔵 **Clarity**: "review"/"spike" used interchangeably — **Partially resolved** (body standardised to "this spike"; the *title* still reads "…stance review" — see New Issues / left as a deliberate decision).
- 🔵 **Clarity**: "0099 supersedes ADR-0036" settled-vs-open tension — **Resolved** (Open Question now scoped explicitly to the unit axis).
- 🔵 **Completeness**: Absence of a time-box/effort constraint — **Resolved** (soft ~1–2 day effort guideline added).
- 🟡 **Dependency**: Spike gates follow-on items but no Blocks entries — **Resolved** (conditional downstream Blocks note added; now cited as a strength).
- 🟡 **Dependency**: 0033 CI gate absent from Dependencies — **Resolved** (added as a binding constraint coupling + to `relates_to`; now cited as a strength).
- 🔵 **Dependency**: AC3's dependence on 0099's successor ADR was an open question, not a blocker — **Resolved** (promoted to a Dependencies conditional prerequisite).
- 🔵 **Dependency**: 0090 reciprocal coupling not navigable — **Partially resolved** (added to `relates_to`; still absent from the Dependencies *section* — see New Issues).
- 🔵 **Scope**: Follow-on scoping risks bleeding migration into the spike — **Partially resolved** (Dependencies Blocks note now framed as a coupling note and cited as a strength; AC4's mechanism enumeration still flagged as mild pre-scoping — see New Issues).
- 🔵 **Scope**: Spike with no time-box — **Resolved** (soft effort bound added).
- 🟡→🔵 **Testability**: AC1 "each source's limits cited" had no completeness bar — **Resolved** (AC1 now enumerates the three source classes and four Q3 sub-conditions; now cited as a strength).
- 🔵 **Testability**: AC2 "justified by" was presence-only — **Resolved** (AC2 now requires citing the Q1 value and per-sub-condition Q3 results; a narrower residual on the "why preferred over each rejected option" clause remains — see New Issues).
- 🔵 **Testability**: Q4/AC3 "chain rather than fork" had no observable check — **Resolved** (AC3 now carries a supersedes-link check; now cited as a strength).
- 🔵 **Testability**: No time-box stopping bound — **Resolved** (soft effort guideline).

### New Issues Introduced

- 🟡 **Clarity / Dependency** (major, now fixed): "Sequence after 0099" contradicted the explicit "if 0099 has not landed" fallback branch added in pass 1 — if the spike always sequences after 0099, the fallback is dead code; the dependency lens separately noted the 0099-successor-ADR had no existence trigger. **Fixed immediately after this re-review** by reframing sequence-after-0099 as preferred-but-not-guaranteed (0099 is ready but unlanded) with the conditional chaining as the fallback, and by making the successor-ADR existence assumption explicit.
- 🔵 **Clarity** (minor, open): The title "Typography rem-vs-px stance review" labels a deciding spike as a "review". Left as a deliberate decision pending author preference (renaming risks nothing structural — refs are by ID — but the title is an identity choice).
- 🔵 **Dependency** (minor, open): 0090 is in `relates_to` but still not reconciled in the Dependencies *section* (parallel precedent vs scheduling coupling unstated).
- 🔵 **Scope** (suggestion, open): AC4's enumeration of implementation mechanisms (token unit change, root anchor, rem emission) may pre-scope the deferred follow-on; could be demoted to an illustrative parenthetical.
- 🔵 **Testability** (minor, open): AC2's "explains why preferred over each rejected option" lacks an objective threshold; AC4's conditional branch can pass without an artefact unless cross-checked against AC2's stance; AC1's "byte-identical" sub-condition is an analytical prediction at spike time (no change implemented yet) and could state which evidence is expected.
- 🔵 **Completeness** (suggestion, open): `status` remains `draft` though the body states the spike is unblocked and ready — confirm whether the draft status is intentional.

### Assessment

The work item is now in implementation-ready shape. All three pass-1 majors
are resolved and confirmed as strengths by the re-review, and the single new
major the re-review surfaced (a contradiction introduced by the pass-1
Dependencies rewrite) has been fixed. The remaining findings are minor/
suggestion-level polish — none blocks planning. The two judgement-call items
worth an explicit author decision are the title wording ("review" vs "spike"/
"decision") and the `status` transition out of `draft`; the rest can be
folded in opportunistically or deferred to the research pass itself (e.g. the
byte-identical evidence framing is naturally settled when Q3 is answered).

### Approved — 2026-06-12

**Verdict overridden to APPROVE** by the author. The pass-2 re-review landed
at COMMENT only because the residual findings are minor polish; with the
introduced contradiction fixed, the work item is implementation-ready. The
residual minor items (title wording, AC4 mechanism enumeration, AC2/AC1
verification framing, 0090 in Dependencies) are accepted as non-blocking and
may be folded in opportunistically or settled during the research pass. The
`status` transition was actioned separately (draft → ready).
