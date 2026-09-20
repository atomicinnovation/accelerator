---
type: "work-item-review"
id: "0282-tunable-depth-and-breadth-review-1"
title: "Work Item Review: Tunable Depth and Breadth"
date: "2026-09-20T21:23:37+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0121"
target: "work-item:0282"
work_item_id: "0282"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-20T21:33:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Tunable Depth and Breadth

**Verdict:** COMMENT

Work item 0282 is a strong, well-scoped story: completeness found no gaps, the
two-child split from epic 0121's Slice 5 is clean, the dependency edges are
mapped with rationale, and the domain vocabulary is consistent across sections.
It is acceptable as-is — no criticals and only one major — but could be improved:
the clamp-and-warn validation surface attracts the review's only major finding
(a genuinely ambiguous acceptance criterion) plus several minor terminology and
record-precision points. Addressing the validation ambiguity before
implementation is the single change worth making.

### Cross-Cutting Themes

- **The clamp-and-warn validation surface is imprecise** (flagged by:
  testability, clarity) — the Validation requirement and its acceptance
  criterion carry an ambiguous outcome for non-integer values ≥1 (testability
  major), an undefined warning message (testability minor), and are conflated in
  the Summary with the separate depth>1 notice (clarity suggestion). Three
  findings converge on one requirement.
- **Dependency records are directionally correct but imprecise in detail**
  (flagged by: dependency) — the forward `blocks: 0283` edge and the satisfied
  0279 blocker are real, but 0282 claims a reciprocal `blocked_by` on 0283 that
  does not exist, and attributes the artefacts it modifies to 0279 when 0277
  authored them. Single-lens, but two mutually reinforcing findings.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: Validation criterion gives two conflicting outcomes for non-integer values of 1 or more
  **Location**: Acceptance Criteria (Validation)
  A value such as `1.5` satisfies both antecedents at once — it is non-integer
  (clamps to 1) and it is 1-or-more (passes unchanged) — so the criterion
  prescribes two contradictory expected outcomes for that class of input. The
  same ambiguity is present in the Requirements bullet.

#### Minor

- 🔵 **Clarity**: Config level named 'project' in Summary but 'team' in the body
  **Location**: Summary
  The Summary lists config sources as "project and personal config", while
  Requirements and Technical Notes name the same `config.md` level "team config".
  A reader must infer the two terms denote one level and could suspect a third
  level exists.

- 🔵 **Testability**: Clamp 'warning' has no defined content, so its presence cannot be conclusively verified
  **Location**: Acceptance Criteria (Validation)
  The criterion requires the invocation to "warn" on clamp but pins no message,
  unlike the dormant-depth criterion which specifies its exact notice. Almost any
  output could be argued to satisfy the warning half.

- 🔵 **Dependency**: 0282 claims a reciprocal `blocked_by` on 0283 that 0283 does not carry
  **Location**: Dependencies
  0283's frontmatter has no `blocked_by` field (its edge lives only in prose), so
  the claim "recorded on ... 0283's `blocked_by`" is not currently true and a
  reverse lookup from 0283's frontmatter would miss the blocker.

- 🔵 **Dependency**: The artefacts this knob modifies originate in 0277, not the named 0279 blocker
  **Location**: Dependencies
  The `outline` breadth ceiling, the effort-scaling rubric, and the hardcoded
  `breadth: 8`/`depth: 1` prose were delivered by Slice 1 (0277); 0279 added
  accretion and finalise. 0277 is transitively upstream and done, so this is a
  record-precision point, not a live blocker.

#### Suggestions

- 🔵 **Scope**: Live breadth knob bundled with a dormant depth knob
  **Location**: Summary
  The story ships one live knob (`breadth`) and one dormant knob (`depth`,
  registered and threaded but inert until 0283). The item's rationale (shared
  registration/validation machinery, de-risking 0283) is sound; recorded only so
  the bundling is a conscious, reviewed decision. No change required.

- 🔵 **Clarity**: 'warns' (Summary) vs 'notice' (Requirements/AC) for the depth>1 signal
  **Location**: Summary
  The Summary says `conduct` "warns" when `depth` exceeds 1, whereas Requirements
  and the acceptance criterion consistently call this a "notice", reserving
  "warning" for the clamp validation. A reader could conflate the two signal
  paths.

- 🔵 **Testability**: Flag-override criterion asserts internal state rather than an observable
  **Location**: Acceptance Criteria (Flag override)
  "Overrides the config value" is an internal-state claim; its only observable
  manifestation (the round sized to at most `n`) lives in the separate Breadth
  ceiling criterion, so the flag criterion is only verifiable by cross-reference.

### Strengths

- ✅ Structurally complete and densely populated: every expected section present
  and substantively filled, with nine specific Given/When/Then acceptance
  criteria plus a check-pass criterion — well beyond the minimum, no gaps forcing
  follow-up questions.
- ✅ Clean two-child decomposition along the exact seam epic 0121 pre-authorised:
  0282 is the independently-shippable knob, 0283 the acknowledged descope
  recursion engine, with a single unified purpose across every requirement.
- ✅ Consistent, well-defined domain vocabulary (breadth, depth, focus area,
  finding, round) drawn from the parent epic, keeping referents stable across
  every section.
- ✅ Dependency edges mapped in both directions with rationale: the 0279 blocker
  is verifiably reciprocal (0279's `blocks` lists 0282) and satisfied, the
  0283 consumer edge is on 0282's own `blocks`, and independence from 0280 is
  reasoned rather than assumed.
- ✅ Drafting Notes explicitly reconcile three premises against the parent epic
  (the deleted `config-defaults.sh`, the "first numeric tunable" claim, and the
  actual personal > team > default precedence), removing cross-document
  contradictions.
- ✅ Config-layer criteria are anchored to concrete existing machinery
  (`config dump`, `dump.golden`, `parity.rs`, the exact key-count test), giving
  them repeatable golden/unit verification with definitive pass/fail.
- ✅ The dormant-depth criterion specifies both the positive case and the exact
  negative-signal notice, giving a clear observable rather than a vague "depth
  works".

### Recommended Changes

1. **Disambiguate the non-integer validation rule** (addresses: Validation
   criterion gives two conflicting outcomes) — state precedence explicitly in
   both the Acceptance Criteria and the Requirements bullet, e.g. "any
   non-integer value clamps to 1 regardless of magnitude; only integers of 1 or
   more pass unchanged". This is the one change worth making before
   implementation.
2. **Pin the clamp warning's essential content** (addresses: Clamp 'warning' has
   no defined content) — e.g. "warns that the value was invalid and was clamped
   to 1", so a verifier has a concrete string to confirm.
3. **Reconcile the dependency records** (addresses: reciprocal `blocked_by` on
   0283; artefacts originate in 0277) — either add `blocked_by` to 0283 or soften
   0282's prose to say the edge is recorded on 0282's `blocks`; and add a note
   that the artefacts being made tunable originate in Slice 1 (0277),
   transitively upstream via 0279 and satisfied.
4. **Align terminology in the Summary** (addresses: project vs team; warns vs
   notice) — use "team config" for the `config.md` level throughout, and call the
   depth>1 signal a "notice" to match Requirements and the acceptance criterion.
5. **Restate the flag-override criterion as an observable** (addresses:
   Flag-override asserts internal state) — e.g. "with `research.breadth: 8`
   configured, `outline foo --breadth 3` sizes the round to at most 3 focus
   areas, not 8".

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Work item 0282 is unusually clear and internally consistent: the two
knobs' units, defaults, dormant-depth behaviour, and resolution order are
described the same way across Summary, Requirements, and Acceptance Criteria, and
its Drafting Notes explicitly reconcile premises against the parent epic. The only
clarity issues are minor terminology drift — the `config.md` level is called
'project' in the Summary but 'team' in the body, and the depth>1 signal is a
'warn' in the Summary but a 'notice' in the Requirements/Acceptance Criteria. No
ambiguous pronouns, undefined acronyms, or mutually contradictory requirements
were found.

**Strengths**:
- Consistent, well-defined domain vocabulary throughout (breadth, depth, focus
  area, finding, round), drawn from the parent epic 0121's Terminology section, so
  referents such as 'breadth bounds outline' and 'depth bounds conduct' stay
  stable across every section.
- The two distinct knob units are kept explicitly non-contradictory: breadth
  bounds focus areas per round (at outline time) and depth bounds recursion within
  a finding (at conduct time), stated the same way in Summary, Requirements,
  Assumptions, and Technical Notes.
- Drafting Notes explicitly reconcile three premises against the parent epic (the
  deleted `scripts/config-defaults.sh`, the 'first numeric tunable' claim, and the
  actual personal > team > default precedence), removing what would otherwise read
  as cross-document contradictions.
- Actors are named wherever behaviour matters — `outline`, `conduct`, and 'the
  SKILL.md prose' each own their actions (resolving overrides, emitting the depth
  notice, clamping invalid values), so responsibility for each action is clear
  rather than obscured by passive voice.

**Findings**:
- 🔵 **minor** (confidence: high) — Config level named 'project' in Summary but
  'team' in the body. **Location**: Summary. The Summary lists the config sources
  as "via project and personal config," while the Requirements (resolution-order
  bullet) and Technical Notes name the same two in-repo levels "team config
  (`config.md`)" and "personal config (`config.local.md`)". The level called
  "project" in the Summary is therefore called "team" everywhere else, and
  "project" is never tied to a file (the parent epic 0121's older "project/user"
  wording appears to have leaked into the Summary while the body adopted the newer
  "team/personal" terms). Impact: a reader must infer that "project config" and
  "team config" denote the same `config.md` level, and could momentarily wonder
  whether a third, distinct config level exists. Suggestion: use one name for the
  `config.md` level in every section.
- 🔵 **suggestion** (confidence: medium) — 'warns' (Summary) vs 'notice'
  (Requirements/AC) for the depth>1 signal. **Location**: Summary. The item
  defines two distinct signals: a validation *warning* when a resolved value is
  clamped to the floor of 1, and a *notice* when a resolved `depth` exceeds 1. The
  Summary blurs the two by saying "`conduct` warns when a resolved `depth` exceeds
  1," using "warns" for the case the Requirements and the 'Depth dormant with a
  signal' criterion consistently call a "notice." Impact: a reader could conflate
  the clamp-and-warn validation with the depth-dormant notice. Suggestion: call
  the depth>1 signal a "notice" in the Summary, reserving "warning" for the
  clamp-and-warn validation.

### Completeness

**Summary**: Work item 0282 is a structurally complete and densely populated
story. Every expected section is present and substantively filled — a story-form
Summary that names the beneficiary and states intent, a Context that explains why
the work is wanted, detailed Requirements, nine specific Given/When/Then
Acceptance Criteria plus a check-pass criterion, and populated Dependencies,
Assumptions, Open Questions, Technical Notes, Drafting Notes, and References.
Frontmatter is complete and internally consistent, with a recognised kind and an
appropriate status. From the completeness lens there are no gaps that would force
an implementer to ask the author follow-up questions.

**Strengths**:
- Summary is complete and kind-appropriate: it uses the story form, names the
  beneficiary, and adds a second paragraph fixing the concrete semantics (breadth
  as the outline ceiling defaulting to 8, depth as a dormant knob defaulting to 1).
- Context explains the motivation rather than restating the summary — it grounds
  the work in the existing `review.*` numeric-tunable precedent and explains why
  the knob lands adjacent to and before the recursion engine.
- Requirements are specific and actionable, naming exact files and artefacts to
  change (`catalogue.rs` RESEARCH_KEYS group, the key-count test, `dump.golden`,
  `parity.rs`, `public-api.txt`, the two SKILL.md consumers).
- Acceptance Criteria are numerous and each is a self-contained, specific
  Given/When/Then covering key visibility, default resolution, config precedence,
  flag override, validation/clamping, the breadth ceiling, breadth-not-re-checked,
  dormant-depth signalling, and docs.
- Optional sections are handled fairly and honestly: Open Questions is explicitly
  'None' with a justification; Dependencies, Assumptions, Technical Notes, and
  References are all populated with relevant, non-placeholder content.
- Frontmatter is complete and coherent: kind (story), status (draft, with the
  Drafting Notes explaining the deliberate hold), priority, parent, blocks, tags,
  and identity fields all present and set to recognised values.

**Findings**: None.

### Dependency

**Summary**: 0282 is a well-dependency-mapped story: its upstream blocker (0279),
its downstream consumer (0283), and its deliberate independence from 0280 are all
named with rationale, and the 0279 edge is verifiably reciprocated in 0279's
frontmatter `blocks`. No external-system or cross-team couplings are introduced —
the knob is pure config plus prompt prose — so nothing is missing on that axis.
Two minor record-precision gaps remain: 0282 claims the 0282→0283 edge is recorded
on 0283's `blocked_by`, but 0283 carries no such frontmatter field; and the
`outline` breadth ceiling and hardcoded `breadth: 8`/`depth: 1` prose that 0282
replaces originate in Slice 1 (0277), which is transitively upstream but unnamed
in Dependencies.

**Strengths**:
- The Dependencies section names both directions of coupling — upstream (0279) and
  downstream (0283) — each with a rationale rather than a bare id.
- The downstream edge is correctly recorded in 0282's own frontmatter
  (`blocks: ["work-item:0283"]`) and matches 0283's prose 'Blocked by ... 0282'.
- The upstream 0279 edge is verifiably reciprocal: 0279's frontmatter carries
  `blocks: ["work-item:0282", "work-item:0284"]`, so the blocker is genuinely
  satisfied (0279 is done).
- The item explicitly reasons about a NON-dependency — independence from the 0280
  output-quality gate, with the token-multiplication rationale — which prevents a
  false coupling being scheduled.
- The dormant-`depth`-through-`conduct` coupling with 0283 is captured, and the
  strict ordering (this knob lands before the recursion engine) is stated.
- The parent edge is reciprocal: 0282's `parent: work-item:0121` is mirrored by
  0121's `children` list.

**Findings**:
- 🔵 **minor** (confidence: high) — **Location**: Dependencies. The Dependencies
  section states the 0282→0283 coupling is 'recorded on this item's `blocks` and
  0283's `blocked_by`', but 0283's actual frontmatter carries no `blocked_by`
  field at all. The forward edge is genuinely captured on 0282's side and in
  0283's prose, so the dependent will not be lost — but the specific claim that the
  edge lives on 0283's `blocked_by` is not currently true. Impact: a reverse
  lookup from 0283's frontmatter would not surface this blocker. Suggestion: add
  `blocked_by: ["work-item:0282", "work-item:0280"]` to 0283, or soften 0282's
  prose to say the edge is recorded on 0282's `blocks` (0279, 0282, and 0283 all
  lack `blocked_by`, suggesting a forward-`blocks`-only convention).
- 🔵 **minor** (confidence: medium) — **Location**: Dependencies. 0282 names only
  0279 as its upstream blocker, but the artefacts it modifies — the `outline`
  breadth ceiling, the effort-scaling rubric, and the hardcoded
  `breadth: 8`/`depth: 1` prose — were delivered by Slice 1 (0277) per epic 0121,
  not by 0279. 0277 is transitively upstream (0279 is blocked_by 0277) and done,
  so this is not a live scheduling blocker. Impact: the most direct prerequisite
  is attributed to the wrong sibling. Suggestion: add a note that the artefacts
  this story replaces originate in 0277, transitively upstream via 0279 and
  already satisfied.

### Scope

**Summary**: Work item 0282 is a well-scoped, coherent story: it carves the 'knob
half' out of epic 0121's Slice 5 exactly along the seam the epic pre-authorised —
the independently-shippable config/flag knob, leaving the token-multiplying
recursion engine to sibling 0283. Its requirements all serve one unified purpose,
the Summary/Requirements/Acceptance Criteria describe the same scope consistently,
and the dependency boundaries (blocked by done-0279, blocks 0283, independent of
0280) are clean. The only scope-flavoured observation is the deliberate inclusion
of a dormant `depth` knob alongside the live `breadth` knob, which the item
justifies well and does not rise beyond a suggestion.

**Strengths**:
- Clean two-child decomposition: 0282 (knob) and 0283 (recursion engine) split
  epic 0121's Slice 5 along the exact seam the epic flagged, and the story is
  self-described as 'independently shippable'.
- Single unified purpose: every requirement serves one deliverable — tunable
  research cost knobs — sharing one config registration/resolution/validation
  mechanism modelled on the existing `review.*` tunables.
- Summary, Requirements, and Acceptance Criteria describe the same scope with no
  drift, including a consistent in-scope/out-of-scope boundary.
- Dependency boundaries are stated and coherent, so the team can deliver it
  without waiting on a parallel thread.
- Appropriate sizing for a 'story' kind: a single increment of system value
  ownable end-to-end.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — Live breadth knob bundled with a
  dormant depth knob. **Location**: Summary. The story delivers one live knob
  (`breadth`) and one dormant knob (`depth`, registered, resolved, threaded, and
  documented but with no behavioural effect until 0283, only emitting a warning
  when set above 1). This is a slight asymmetry in deliverable value; the depth
  half could alternatively land with 0283. Impact: minor — a planner might
  question whether half the surface belongs here or with the engine that activates
  it; both knobs also amend `conduct`. Suggestion: no change required — the
  rationale is sound (shared machinery; shipping `depth` dormant-but-honest
  de-risks 0283). Recorded only so the bundling is a conscious decision.

### Testability

**Summary**: Work item 0282 is largely well-specified from a testability
standpoint: most Acceptance Criteria are framed as Given/When/Then with concrete
inputs and observable outputs, and the config-layer criteria are backed by named
existing machinery that yields definitive pass/fail. The main testability gap is
the validation criterion, which admits two conflicting expected outcomes for
non-integer values of 1 or more, so a verifier cannot derive a single pass/fail
for that class of input. Secondary softness comes from criteria whose only defined
verification is 'at eval level' where the observable signal is left abstract.

**Strengths**:
- Most criteria are expressed as clear Given/When/Then pairs with specific inputs
  and observable outputs (defaults 8/1, personal-over-team precedence, at most N
  focus areas, one researcher per focus area).
- The config-layer criteria are anchored to concrete existing machinery —
  `accelerator config dump`, `dump.golden`, `parity.rs`, the exact key-count test
  — giving them repeatable golden/unit verification.
- The criteria collectively cover the Summary intent: both config levels,
  per-invocation flags, defaults, the live breadth ceiling, the dormant depth
  knob, and the docs.
- The dormant-depth criterion specifies both the positive case and the negative
  signal (an explicit notice naming 0283), giving a clear observable.
- The Technical Notes 'Testability' bullet is explicit about which criteria are
  unit/golden-testable versus eval-level prose contracts.

**Findings**:
- 🟡 **major** (confidence: medium) — Validation criterion gives two conflicting
  outcomes for non-integer values of 1 or more. **Location**: Acceptance Criteria
  (Validation). The criterion states a value that is 'zero, negative, or
  non-integer ... clamps to 1', and separately that 'a value of 1 or more passes
  unchanged'. A value such as 1.5 or 2.5 satisfies both antecedents at once, so the
  criterion prescribes two contradictory expected outcomes for that class of
  in-scope input — the same ambiguity is in the Requirements bullet. Impact: a
  verifier feeding a non-integer ≥1 cannot derive a single pass/fail, so two
  implementers could each claim the criterion is met. Suggestion: pin the
  precedence explicitly, e.g. 'any non-integer value clamps to 1 regardless of
  magnitude, and only integers of 1 or more pass unchanged' — and mirror the
  wording in the Requirements bullet.
- 🔵 **minor** (confidence: medium) — Clamp 'warning' has no defined content.
  **Location**: Acceptance Criteria (Validation). The criterion requires that on
  clamp 'the invocation warns', but neither the criterion nor the Requirements pin
  what the warning must say, unlike the dormant-depth criterion which specifies its
  exact notice. Impact: a verifier cannot distinguish a correct clamp warning from
  any unrelated output. Suggestion: state the warning's essential content, e.g.
  'warns that the value was invalid and was clamped to 1'.
- 🔵 **suggestion** (confidence: low) — Flag-override criterion asserts internal
  state rather than a directly observable outcome. **Location**: Acceptance
  Criteria (Flag override). 'Overrides the config value' is an internal-state
  claim; the only directly observable manifestation (the round sized to at most n)
  lives in the separate Breadth ceiling criterion, so this criterion is only
  verifiable by cross-referencing another. Impact: taken alone, the criterion
  names the verification category but not the observable. Suggestion: restate as an
  observable, e.g. 'with `research.breadth: 8` configured, `outline foo --breadth
  3` sizes the round to at most 3 focus areas, not 8'.

## Re-Review (Pass 2) — 2026-09-20

**Verdict:** COMMENT

The pass-1 edits resolved every previously identified finding, including the one
major. The re-review then surfaced three new issues that were direct consequences
of those edits being applied incompletely — all fixed in this pass — plus several
pre-existing terminology and integration-coupling points the deeper second read
brought up (all suggestions bar one minor). The work item is ready for
implementation; the remaining open items are optional polish, not blockers.

### Previously Identified Issues

- 🔵 **Clarity**: Config level named 'project' in Summary but 'team' in the body — Resolved (Summary now says "team config").
- 🔵 **Clarity**: 'warns' vs 'notice' for the depth>1 signal — Resolved (Summary now says `conduct` "emits a notice").
- 🔵 **Dependency**: Claimed reciprocal `blocked_by` on 0283 that 0283 does not carry — Resolved (prose now states the forward-`blocks`-only convention; verified against 0283's frontmatter).
- 🔵 **Dependency**: Artefacts modified originate in 0277, not the named 0279 blocker — Resolved (Dependencies now names 0277 as the transitively-upstream origin).
- 🟡 **Testability**: Validation criterion gave two conflicting outcomes for non-integer values ≥1 — Resolved (Requirements and AC now state a non-integer clamps regardless of magnitude; only integers ≥1 pass unchanged).
- 🔵 **Testability**: Clamp warning had no defined content — Resolved (Requirements and AC now require the warning to name the invalid value and state the clamp).
- 🔵 **Testability**: Flag-override criterion asserted internal state — Partially resolved at re-review, then fully fixed this pass (breadth restated as observable in pass 1; the depth half's missing observable fixed here — see New Issues).
- 🔵 **Scope**: Live breadth knob bundled with dormant depth knob — Still present (unchanged; the lens re-confirmed it is a defensible judgment call, not a defect — no change made).

### New Issues Introduced

- 🟡 **Testability**: Flag-override depth half had no observable outcome — Fixed this pass. The pass-1 edit made the breadth half observable but left `--depth` as an internal-state claim, and depth is dormant, so it had no standalone observable. The AC now ties `--depth 2` over `research.depth: 1` to the depth notice firing.
- 🔵 **Testability**: Validation AC under-specified the warning relative to the Requirements bullet — Fixed this pass. Pass 1 strengthened the Requirements warning ("names the invalid value") more than the AC; the AC now matches.
- 🔵 **Dependency**: Drafting Notes still said "0283 carries the `blocked_by: 0280` edge" — Fixed this pass. This contradicted the forward-`blocks`-only vocabulary the pass-1 Dependencies edit introduced; the note now uses the same forward-`blocks` framing.
- 🔵 **Dependency**: Uncaptured shared-catalogue coupling with 0280 — Open (optional). Both 0282 and 0280 register `research.*` keys touching the same key-count test, `dump.golden`, and `public-api.txt`, so whichever lands second carries a light reconciliation cost. Pre-existing; not introduced by the edits.
- 🔵 **Testability**: AC1 does not state the expected source-attribution literal — Open (optional). Pre-existing; the `review.*` dump precedent likely makes it clear in practice.
- 🔵 **Clarity**: 'eval level', 'the consumer', and 'the effort-scaling rubric / 10+ band' are undefined or defined-only-by-link — Open (optional glosses). Three low/medium suggestions; pre-existing terminology, not contradictions.

### Assessment

The work item is ready for implementation. All pass-1 findings — including the
sole major — are resolved, and the three new issues traceable to the edits are
fixed. Verdict holds at COMMENT: no criticals and one major at re-review time
(now fixed), below the REVISE thresholds. The six open items are optional polish
(one dependency minor, one testability suggestion, three clarity suggestions, one
standing scope judgment call) and none blocks implementation.
