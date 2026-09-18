---
type: "work-item-review"
id: "0277-single-round-web-research-engine-review-1"
title: "Work Item Review: Single-Round Web Research Engine"
date: "2026-09-08T19:36:01+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0277"
work_item_id: "0277"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-08T21:28:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Single-Round Web Research Engine

**Verdict:** REVISE

0277 is a structurally complete, densely populated story whose Summary,
Requirements, and Acceptance Criteria describe one coherent scope — the
walking-skeleton research engine — with no drift, and it pre-empts its one
obvious contradiction (the effort-scaling rubric's 10+ band versus the
`breadth: 8` ceiling). It draws a REVISE on three major findings, all
verification and dependency-representation gaps rather than logic errors: the
reputation-tier contract field has no enumerated valid-value set, the
three-form set-handle resolution requirement has no acceptance criterion, and
the co-land constraint with 0278 is captured only in prose and a one-directional
`blocks` edge. The remaining findings are lexical and precision refinements —
locally undefined domain terms, implicit expected values, and process qualifiers
that only observable proxies can verify.

### Cross-Cutting Themes

- **Co-land constraint with 0278 is under-represented** (flagged by: dependency,
  scope) — The symmetric "must merge together" coupling lives only in prose and
  a one-directional `blocks` edge, with no machine-readable reciprocal marker.
  Dependency frames it as a representational gap that tooling could mis-sequence;
  scope frames it as a compromised INVEST-independence property. Both point at
  the same underlying under-capture of the pair's atomicity.
- **Set-handle resolution is both undefined and unverified** (flagged by:
  clarity, testability) — The core term "set" and its compounds are defined only
  in the linked epic (clarity), and the three-form resolution behaviour those
  compounds describe has no acceptance criterion (testability). The concept is
  central to every non-`brief` verb yet is the weakest-anchored part of the item.
- **The split `paths.research_topics` config key is thinly verified** (flagged
  by: dependency, testability) — Its default lands here while the server wiring
  is in 0278; no criterion asserts the key resolves to its default, and it is
  unstated whether the engine's own `mise run check` is green before 0278 lands.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Reputation tier has no enumerated valid-value set, so the criterion is only presence-checkable
  **Location**: Acceptance Criteria (reputation tier)
  The criterion requires every source entry to carry a valid tier, but neither
  0277 nor 0121 enumerates the closed set of tier labels — 0121 defines the term
  only conceptually. A tester can confirm *some* annotation is present, not that
  it is a *valid* tier.

- 🟡 **Testability**: Three-form set-handle resolution is a stated Requirement but has no acceptance criterion
  **Location**: Acceptance Criteria / Requirements (set-handle resolution)
  Requirements and Technical Notes specify resolution across three input forms
  (set directory, bare slug, any sub-document), yet every criterion merely opens
  "Given a set handle" and never verifies all three resolve to the set root. A
  build resolving only one form would still pass every criterion.

- 🟡 **Dependency**: Co-land constraint with 0278 encoded only as one-directional `blocks` + prose
  **Location**: Dependencies
  The 0277↔0278 relationship is a symmetric co-land, but the frontmatter records
  only `blocks: ["work-item:0278", ...]` — a one-directional edge reading "0277
  precedes 0278." The simultaneity lives solely in prose, so tooling reasoning
  from frontmatter could sequence the two or merge 0277 alone.

#### Minor

- 🔵 **Dependency**: Gated consumer 0281 named in prose but absent from the `blocks` frontmatter
  **Location**: Frontmatter: blocks
  Dependencies states 0277 "also gates 0281 (recorded on 0281's `blocked_by`)",
  yet `blocks` lists only 0278/0279/0280. Forward traversal of 0277's edges will
  silently miss 0281.

- 🔵 **Testability**: `conduct` criterion sets `round_count` / `finding_count` without stating expected values
  **Location**: Acceptance Criteria (conduct)
  The criterion sets the counts but not their expected values, leaving the
  pass/fail threshold implicit (a verifier must infer `round_count: 1` and
  `finding_count` = focus areas researched).

- 🔵 **Testability**: Finding immutability is asserted but no in-slice criterion exercises it
  **Location**: Acceptance Criteria (conduct) / Assumptions
  Findings are asserted "immutable once written", but nothing in this single-round
  slice re-runs a verb over an existing finding, so no criterion confirms a
  written finding is not overwritten (re-invocation is deferred to 0279).

- 🔵 **Testability**: `paths.research_topics` key + default has no dedicated verification beyond the aggregate check
  **Location**: Requirements / Acceptance Criteria (config key)
  No criterion verifies the key exists or that its default resolves; it is only
  implicitly covered by the final "tests green" criterion. The default path could
  be misconfigured while `mise run check` still passes.

- 🔵 **Clarity**: "Outstanding focus areas" in AC3 diverges from Requirements' "per focus area" wording
  **Location**: Acceptance Criteria
  Requirements say `conduct` writes "one immutable finding per focus area";
  AC3 says it runs over the "outstanding" focus areas and "flips the
  corresponding checkboxes" — wording that evokes the gap-detection re-invocation
  the epic defers to Slice 2.

- 🔵 **Dependency**: Config-key wiring split across 0277/0278 leaves engine-check satisfiability unstated
  **Location**: Acceptance Criteria
  The `paths.research_topics` default lands here while server-side wiring is in
  0278. The final criterion asserts `mise run check` passes "end-to-end for the
  engine" but does not state whether that check is satisfiable before 0278's
  registration lands.

#### Suggestions

- 🔵 **Clarity**: Core term "set" used pervasively but defined only in the linked epic
  **Location**: Summary
  "Set" and its compounds ("set handle", "set root", "set directory") carry the
  core vocabulary but are never defined locally — the definition lives only in
  0121. "Set" is a heavily overloaded English word, so a reader of 0277 alone
  could misread it.

- 🔵 **Clarity**: Inert `depth: 1` default stated without explaining what depth controls here
  **Location**: Requirements
  The default `depth: 1` "(made tunable in 0282)" is stated without noting what
  `depth` governs or that it is inert in this single-round slice — unlike the
  parallel `breadth: 8`, whose effect is spelled out.

- 🔵 **Completeness**: User/consumer whose need is met is only implicit for a story
  **Location**: Context
  As a story, 0277 is framed almost entirely in engineering terms; the explicit
  "as an Accelerator user, I want…" need lives only in parent epic 0121. A reader
  in isolation gets the mechanism but must open the epic to learn who benefits.

- 🔵 **Scope**: Hard co-land constraint with 0278 means 0277 is not independently deliverable
  **Location**: Dependencies
  The true atomic unit of delivery is 0277 + 0278 together, compromising INVEST
  independence. No restructuring is needed (the split is epic-blessed), but the
  pairing should be explicit in scheduling so the two always land in one
  increment.

- 🔵 **Scope**: Engine story bundles a large surface for a single story
  **Location**: Requirements
  Even after the visualiser was carved to 0278, the engine spans a config key,
  five templates + override, a researcher agent + web profile, a 4-verb skill,
  and the rubric. Likely acceptable — the four verbs form one indivisible
  pipeline — but confirm at planning time it fits a single increment.

- 🔵 **Testability**: Process and rubric qualifiers ("interactively", "parallel", rubric sizing) are only proxy-verifiable
  **Location**: Acceptance Criteria
  `brief` "interactively scopes", `conduct` "spawns parallel agents", and
  `outline` sizes "by the rubric" resist direct verification; only observable
  proxies (files, frontmatter, the count ≤ `breadth` ceiling) are testable.

- 🔵 **Testability**: The researcher-agent criterion mixes a checkable negative with an unverifiable forward reference
  **Location**: Acceptance Criteria (generic researcher agent)
  The criterion bundles a verifiable structural check (no per-source agent) with
  a claim about what Slice 3 (0280) "later proves", which cannot be verified in
  this slice.

### Strengths

- ✅ Every expected section for a story is present and carries substantive,
  non-placeholder content; frontmatter is fully formed and internally consistent
  (kind, status, parent 0121, and `blocks` all align with the prose).
- ✅ The four build-verb acceptance criteria are framed as observable
  input-output pairs anchored to concrete outputs — named files, specific
  frontmatter values, and status transitions.
- ✅ Two executable gates (`accelerator corpus frontmatter validate` over an
  enumerated document list, and `mise run check`) give unambiguous binary
  pass/fail verification.
- ✅ Scope is highly coherent: all eight requirements serve the single purpose of
  running the build loop once over web sources, with a clean 0277/0278 boundary
  and strong fidelity to the epic's Slice 1 engine portion.
- ✅ The effort-scaling rubric versus `breadth: 8` contradiction is pre-empted in
  the Drafting Notes rather than left to the reader, and "conforming to the
  artifact contract" is anchored to the field-name-level table pinned in 0121.
- ✅ As the foundational engine slice, the absence of any upstream `blocked_by` is
  correct, and downstream consumers plus the immutability/backward-compatibility
  coupling are captured.

### Recommended Changes

1. **Enumerate the reputation-tier vocabulary** (addresses: Reputation tier has
   no enumerated valid-value set)
   Add the closed set of tier labels to Requirements, or reference an enumeration
   in 0121, so the criterion reads "every source entry carries a tier drawn from
   {enumerated set}" and yields a definitive pass/fail.

2. **Add an acceptance criterion for three-form set-handle resolution**
   (addresses: Three-form set-handle resolution has no acceptance criterion)
   E.g. "a bare slug, the set directory, and an arbitrary sub-document path each
   resolve `outline` to the same `meta/research/topics/<slug>/` root."

3. **Represent the co-land constraint with 0278 in machine-readable form**
   (addresses: Co-land constraint encoded only as one-directional blocks;
   Hard co-land constraint means 0277 not independently deliverable)
   Add a reciprocal `blocked_by`/co-land marker referencing 0278, or a dedicated
   co-land field, so the merge-together constraint survives outside prose; note
   the pairing in scheduling.

4. **State expected values and satisfiability for the count and config criteria**
   (addresses: round_count/finding_count without values; config-key satisfiability;
   paths.research_topics has no dedicated verification)
   State `round_count: 1` and `finding_count` = outstanding focus areas researched;
   assert `paths.research_topics` resolves to its documented default when unset;
   and state whether `mise run check` is green in isolation or depends on 0278.

5. **Reconcile the `blocks` frontmatter with the prose gating** (addresses: Gated
   consumer 0281 absent from blocks frontmatter)
   Either add 0281 to `blocks` for parity with 0278/0279/0280, or state the
   recording convention (blocked-item side only) and apply it uniformly.

6. **Clarify the local vocabulary and immutability boundary** (addresses:
   "set" defined only in the epic; "outstanding" vs "per focus area"; inert
   depth:1; finding immutability not exercised)
   Gloss "set" on first use; align AC3's "outstanding" with Requirements or note
   that all focus areas are outstanding in this single round; note `depth` governs
   intra-finding recursion, inert at `depth: 1`; and either add an immutability
   check or note it lands with 0279.

7. **Add one beneficiary sentence to Context** (addresses: User/consumer implicit
   for a story)
   Name the Accelerator user and the value of an end-to-end, observable research
   loop, so the story's "for whom" stands on its own.

## Per-Lens Results

### Clarity

**Summary**: Work item 0277 is largely clear: its Acceptance Criteria use active
voice with each subcommand named as the actor and outcomes stated as observable
file/status states, and it explicitly pre-empts the one obvious contradiction
(the effort-scaling rubric's 10+ band versus the breadth-8 ceiling). The clarity
gaps that remain are lexical rather than logical — the core domain term "set" and
its many compounds are defined only in the linked parent epic, and a couple of
terminology choices (`conduct` over "outstanding" focus areas; the inert
`depth: 1` default) could momentarily mislead a reader of this slice alone. No
ambiguous pronouns or substantive internal contradictions were found.

**Strengths**:
- Acceptance Criteria consistently name the performing actor (the
  `brief`/`outline`/`conduct`/`synthesise` subcommands) and state each outcome as
  an observable system state — files written, a specific `research_status` value
  set, `primary` flipped — rather than a vague desired property.
- A potential contradiction between the effort-scaling rubric ("10+ for broad
  subjects") and the hardcoded `breadth: 8` ceiling is explicitly reconciled in
  the Drafting Notes, which spell out that the 10+ band is unreachable in this
  slice — an ambiguity pre-empted rather than left to the reader.
- The "3-tier override" is defined inline as "config path → user override →
  plugin default", making the term self-contained within the work item.
- Cross-references to sibling children are each given a descriptor (0278 as the
  visualiser/indexer sibling, 0280 as the Slice 3 output-quality gate / academic
  sources, 0282 as tunability), which keeps most inter-work-item referents
  resolvable without leaving the document.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — *Core term "set" used pervasively but
  defined only in the linked epic* — Location: Summary
  The domain term "set" and its compounds ("contract-conforming set", "set
  handle", "set directory", "set root", "any set document", "set-handle
  resolution") carry the core vocabulary of this work item across the Summary,
  Requirements, Acceptance Criteria and Technical Notes, but "set" is never
  defined locally — its definition ("the on-disk document collection for one
  subject, rooted at meta/research/topics/<slug>/") lives only in the linked
  parent epic 0121. Impact: "set" is a heavily overloaded common English word, so
  a reader of 0277 on its own could misread it before reaching the epic, and the
  several "set-" compounds amplify the ambiguity. Suggestion: add a one-line gloss
  of "set" on first use in the Summary or Context, even though the References link
  to 0121 technically supplies the definition.
- 🔵 **minor** (confidence: low) — *"Outstanding focus areas" in AC3 diverges from
  Requirements' "per focus area" wording* — Location: Acceptance Criteria
  The Requirements describe `conduct` as "writing one immutable finding per focus
  area" (i.e. all of the outline's focus areas), whereas Acceptance Criterion 3
  says `conduct` "runs one round over the outline's *outstanding* focus areas" and
  "flips the corresponding checkboxes" — wording that evokes the gap-detection
  re-invocation the parent epic explicitly defers to Slice 2. Impact: a reader
  could infer partial, gap-detecting conduct is in this slice's scope, when for a
  single-round engine "outstanding" simply equals "all". Suggestion: align the two
  statements — either use "outstanding" in both, or add a half-clause noting that
  in this single round every focus area is outstanding so no gap-detection is
  implied (that behaviour is Slice 2 in the parent epic).
- 🔵 **suggestion** (confidence: low) — *Inert `depth: 1` default stated without
  explaining what depth controls here* — Location: Requirements
  The Requirements establish the hardcoded default `depth: 1` "(made tunable in
  0282)", but neither the Requirements nor the Drafting Notes state what `depth`
  governs or that it is inert in this single-round slice — by contrast, the
  parallel `breadth: 8` ceiling has its behavioural effect (the rubric's 10+ band
  being unreachable) spelled out in the Drafting Notes. Impact: a reader of this
  slice alone could be left guessing what `depth: 1` does here, since recursion —
  the only behaviour depth governs — does not arrive until 0282. Suggestion: add a
  half-sentence noting that `depth` governs intra-finding recursion, which is a
  no-op at the default `depth: 1` in this single-round slice, mirroring how the
  breadth ceiling is already explained.

### Completeness

**Summary**: Work item 0277 is structurally complete and densely populated: every
expected section for a story (Summary, Context, Requirements, Acceptance Criteria,
Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes,
References) is present and carries substantive, non-placeholder content, and the
frontmatter is fully formed with a recognised kind and status. The ten acceptance
criteria and eight requirement bullets give an implementer more than enough to act
on without follow-up questions. The one soft spot is that this story is framed
almost entirely in engineering terms, with the user/consumer whose need is being
met left implicit and carried by the parent epic rather than restated here.

**Strengths**:
- The Summary is a clear, unambiguous statement of intent: it names the concrete
  deliverable (a contract-conforming research set on disk at
  meta/research/topics/<slug>/), the loop it runs (brief → outline → conduct →
  synthesise), and its relationship to the sibling child 0278.
- Acceptance Criteria are numerous (ten) and each ties to an observable done-state
  — files written, frontmatter field values, status transitions, and the
  `mise run check` gate — well beyond the two-criterion minimum.
- Requirements enumerate the actual work to build (config key + default, 3-tier
  template override, manifest.md aggregate root, generic researcher agent, skill
  dispatch, outline rubric, conduct/synthesise behaviour, hardcoded defaults) and
  complement rather than merely duplicate the acceptance criteria.
- Optional sections that could easily have been empty — Dependencies, Assumptions,
  Open Questions, Technical Notes — are all meaningfully populated, including an
  explicit "none specific to this slice" resolution for Open Questions that points
  to where the one live question (0284) is scoped.
- Frontmatter is complete and internally consistent: kind (story), status (draft),
  priority, parent (0121), and blocks (0278/0279/0280) are all present and align
  with the prose Dependencies section.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — *User/consumer whose need is met is only
  implicit for a story* — Location: Context
  As a story, 0277 is expected to identify the user or system whose need is being
  met, but its Context and Summary frame the work almost entirely in engineering
  terms (engine half, reusable-infrastructure seam, contract-conforming
  artifacts). The nearest user framing is indirect — "no single verb puts that
  question in front of a user" — and the explicit "as an Accelerator user, I want…"
  need lives only in the parent epic 0121. Impact: a reader picking up this child
  in isolation gets the mechanism clearly but must open the epic to understand who
  benefits and why the walking skeleton matters, adding a follow-up step.
  Suggestion: add one sentence to Context naming the beneficiary and the value they
  get from this slice, so the story's "for whom" stands on its own.

### Dependency

**Summary**: As the foundational Slice-1 engine, 0277 correctly carries no upstream
blockers and captures its downstream consumers (0278/0279/0280) in both frontmatter
and prose, with the pivotal co-land constraint with 0278 stated repeatedly. The
main dependency-capture gaps are representational: the symmetric co-land constraint
is encoded only as a one-directional `blocks` edge plus prose, one gated consumer
(0281) is deliberately omitted from the machine-readable `blocks` list while its
siblings are present, and the deliberate split of the config-key wiring across
0277/0278 leaves it unstated whether the engine's own acceptance checks are
satisfiable before 0278 lands.

**Strengths**:
- The co-land coupling with 0278 is explicitly and repeatedly captured — in the
  Summary, the Dependencies section ("must merge together with 0278"), and the
  Drafting Notes — so the merge-together intent is unambiguous in prose.
- Downstream consumers 0278, 0279, and 0280 are captured in both the `blocks`
  frontmatter and the Dependencies prose, and the deferred output-quality
  judgement is explicitly located in 0280.
- As the foundational engine slice, the absence of any upstream `blocked_by` is
  correct and consistent with the parent epic's statement that Slice 1 is
  foundational and gates the rest.
- The immutability/backward-compatibility coupling for future work is captured in
  Assumptions ("findings are immutable once written, so any later additions must
  be backward-compatible"), pre-empting a hidden constraint on downstream slices.

**Findings**:
- 🟡 **major** (confidence: medium) — *Co-land constraint with 0278 encoded only as
  one-directional blocks + prose* — Location: Dependencies
  The 0277↔0278 relationship is a symmetric co-land ("must merge together with 0278
  so the vertical demo is not lost"), but the machine-readable frontmatter records
  only `blocks: ["work-item:0278", ...]` — a one-directional edge that reads as
  "0277 precedes 0278." The simultaneity ("merge together") lives solely in prose,
  with no incoming `blocked_by` or co-land marker on 0277. Impact: scheduling or
  planning tooling that reasons from frontmatter alone could sequence the two or
  merge 0277 by itself, losing the whole-slice vertical demo that is the epic's
  central design-risk validation. Suggestion: represent the co-land explicitly in
  machine-readable form — a reciprocal `blocked_by`/co-land marker referencing
  0278, or a dedicated co-land field.
- 🔵 **minor** (confidence: medium) — *Gated consumer 0281 named in prose but absent
  from the blocks frontmatter* — Location: Frontmatter: blocks
  The Dependencies section states 0277 "also gates 0281 (recorded on 0281's
  `blocked_by`)", yet the frontmatter `blocks` list contains only 0278/0279/0280
  and omits 0281. Three of the four downstream consumers are captured on this
  item's outgoing `blocks` edge; the fourth is captured only via the reverse edge
  on 0281. Impact: forward traversal of 0277's `blocks` edges will silently miss
  0281. Suggestion: either add 0281 to 0277's `blocks` for parity with its
  siblings, or state the recording convention (blocked-item side only) explicitly
  and apply it uniformly.
- 🔵 **minor** (confidence: low) — *Config-key wiring split across 0277/0278 leaves
  engine-check satisfiability unstated* — Location: Acceptance Criteria
  The `paths.research_topics` config key is deliberately split — its default lands
  here while "the server-side `config_path_key` wiring belongs to 0278". The final
  acceptance criterion asserts `mise run check` passes "end-to-end for the engine,"
  but the work item does not state whether that check (or
  `accelerator corpus frontmatter validate` for the new `topic-research` shapes)
  is satisfiable before 0278's server-side registration lands. Impact: if the
  engine's own green-check criteria in fact depend on 0278's registration, that is
  a hidden inbound coupling. Suggestion: state explicitly whether 0277's
  `mise run check` / frontmatter-validate criteria are green in isolation.
- 🔵 **suggestion** (confidence: low) — *End-to-end loop depends on live external web
  access (WebFetch) with no coupling noted* — Location: Requirements
  The engine grants the generic `researcher` agent "WebFetch of arbitrary URLs,"
  and the acceptance criteria exercise the full loop producing conforming findings
  with real sourced URLs. This makes the loop's end-to-end success — and any test
  that drives it — dependent on live external web availability, a coupling not
  called out in Dependencies or Assumptions. Impact: the walking-skeleton demo and
  any full-loop verification are coupled to external site availability, with no
  note on how the loop degrades or is verified when access is unavailable.
  Suggestion: add a brief note acknowledging the external web-access dependency of
  the full-loop path.

### Scope

**Summary**: 0277 is a coherent, well-bounded story: every requirement serves the
single purpose of the walking-skeleton research engine (brief → outline → conduct
→ synthesise producing contract-conforming artifacts on disk), and its Summary,
Requirements, and Acceptance Criteria describe the same scope with no drift. The
split of epic Slice 1 into an engine child (0277) and a visualiser/indexer child
(0278) faithfully follows the seam the parent epic explicitly blessed at breakdown
time, and the boundary is drawn cleanly (skill-side path default here, server
config_path_key deferred to 0278). The only scope tensions are mild and
well-mitigated: a hard co-land constraint means 0277 is not independently
deliverable, and the engine bundles a substantial amount of surface for one story
— both are deliberate, documented tradeoffs rather than defects.

**Strengths**:
- High coherence: all eight requirements (config key, templates, manifest root,
  generic researcher agent, 4-verb skill, outline rubric, conduct/synthesise,
  hardcoded defaults) serve the single purpose of running the build loop once over
  web sources — no independent concern is bundled in.
- Summary, Requirements, and Acceptance Criteria describe the same scope
  consistently, with the visualiser concerns correctly excluded and pushed to 0278
  — no cross-section scope mismatch.
- Clean boundary between the engine (0277) and the visualiser/indexer (0278): the
  config key is deliberately split (skill-side path default here, server-side
  config_path_key in 0278), and the manifest is written here while the indexer
  that keys on it lives in 0278.
- Strong scope fidelity to the epic's Slice 1 engine portion — every engine
  element the epic enumerated is present, with nothing dropped or added.
- The four verbs form one linear pipeline (brief feeds outline feeds conduct feeds
  synthesise) that is the smallest increment producing a readable dossier —
  genuinely indivisible, so keeping them in one story is justified.
- The declared kind (story) is appropriate: this is a single increment of system
  value owned end-to-end by one team, not an epic, chore, task, or spike.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — *Hard co-land constraint with 0278 means
  0277 is not independently deliverable* — Location: Dependencies
  0277 (the research engine) carries a hard co-land constraint with sibling 0278
  (the visualiser doc-type registration + nested-manifest indexer): both the
  Summary and Dependencies state the two must merge together so the vertical demo
  lands whole. This means the true atomic unit of delivery is 0277 + 0278 together,
  and 0277 cannot be shipped or rolled back without simultaneous completion of a
  separate work item. Impact: the INVEST "independent" property is compromised —
  planning and release must treat the pair as one gated unit, and the AC (which
  read as self-contained) can mask that 0277 has no standalone shippable value
  without 0278. Suggestion: no restructuring needed — the split is deliberate and
  epic-blessed — but make the pairing explicit in scheduling, and consider stating
  in the Summary that the engine's artifacts are only reader-observable once 0278
  co-lands.
- 🔵 **suggestion** (confidence: low) — *Engine story bundles a large surface for a
  single story* — Location: Requirements
  Even after the visualiser was carved out to 0278, the engine story still spans a
  new config key, five templates plus 3-tier override and per-skill wiring, a
  generic researcher agent with a web source profile, a 4-verb skill with
  set-handle resolution, and the effort-scaling rubric — a substantial surface for
  one story. Impact: if the engine cannot be delivered within a normal story
  timebox, it risks becoming an implicit mini-epic that is hard to plan and verify.
  Suggestion: low confidence and likely acceptable — the four verbs form one linear
  pipeline where any finer split yields non-demoable fragments. Confirm at planning
  time that the engine fits a single delivery increment; if not, the natural seam
  is agent+profile+templates ahead of the skill dispatch, not splitting the
  pipeline verbs.

### Testability

**Summary**: The four build-verb acceptance criteria are largely well-framed as
Given/When/Then pairs anchored to concrete, observable outputs (named files,
specific frontmatter field values, and the pinned artifact contract from 0121),
and two executable gates (`corpus frontmatter validate`, `mise run check`) give
binary pass/fail checks. However, one binding contract element — the reputation
tier — has no enumerated valid-value set, so it is only presence-checkable, and a
substantive Requirement (three-form set-handle resolution) has no acceptance
criterion at all. Several criteria also assert process or immutability properties
whose direct verification is deferred or ambiguous, though observable file proxies
mitigate most of these.

**Strengths**:
- The four build-verb criteria (`brief`/`outline`/`conduct`/`synthesise`) are
  framed as observable input-output pairs with concrete, checkable outputs: named
  files on disk plus specific frontmatter values (`research_status`, `primary`
  pointer, `source_profiles: ["web"]`, `research_kind`, `round`, `question`,
  `source_profile: web`).
- The `breadth` ceiling criterion is a clean, deterministic invariant (focus areas
  per round <= 8) and is correctly framed as a hard bound rather than a subjective
  target.
- Two executable gates — `accelerator corpus frontmatter validate` over an
  enumerated list of documents, and `mise run check` — provide unambiguous binary
  pass/fail verification.
- "Conforming to the artifact contract" is anchored to the field-name-level
  contract table pinned in parent epic 0121, making an otherwise-vague phrase
  concretely verifiable.
- The Context correctly defers output-quality judgement to the Slice 3 gate in
  0280, so this slice's criteria appropriately verify structure and mechanism only
  — a sound testability boundary rather than an untestable quality assertion.

**Findings**:
- 🟡 **major** (confidence: high) — *Reputation tier has no enumerated valid-value
  set, so the criterion is only presence-checkable* — Location: Acceptance Criteria
  (reputation tier)
  The criterion "Every source entry in every finding carries an inline reputation
  tier ... and those tiers are carried into `synthesis.md`" requires a verifier to
  confirm each source carries a valid tier, but neither this work item nor the
  referenced parent epic 0121 enumerates the closed set of tier labels — 0121
  defines "reputation tier" only conceptually (venue standing, not claim
  correctness). A tester can confirm that *some* annotation is present, but cannot
  confirm it is a *valid* tier value. Impact: the criterion can be claimed as
  passed by any arbitrary inline annotation, providing weak verification value for
  a binding, contract-level field that later slices depend on. Suggestion:
  enumerate the valid reputation-tier vocabulary in the Requirements or reference
  it from 0121, so the criterion becomes "every source entry carries a tier drawn
  from {enumerated set}".
- 🟡 **major** (confidence: high) — *Three-form set-handle resolution is a stated
  Requirement but has no acceptance criterion* — Location: Acceptance Criteria /
  Requirements (set-handle resolution)
  The Requirements and Technical Notes specify that every verb except `brief`
  resolves a set handle in three forms — set directory, bare slug (against
  `paths.research_topics`), or any document within the set — yet every acceptance
  criterion merely opens with "Given a set handle" and never verifies that all
  three input forms resolve to the set root. Impact: a substantive, non-trivial
  piece of functionality could be shipped resolving only one form while every
  acceptance criterion still passes. Suggestion: add a criterion enumerating the
  three handle forms and asserting each resolves to `meta/research/topics/<slug>/`
  for a non-`brief` verb.
- 🔵 **minor** (confidence: medium) — *conduct criterion sets round_count /
  finding_count without stating expected values* — Location: Acceptance Criteria
  (conduct)
  The `conduct` criterion states it "sets `research_status: researching`,
  `round_count`, `finding_count`" but does not state the expected values, leaving
  the pass/fail threshold implicit. A verifier must infer round_count=1 (single
  round) and finding_count = the number of focus areas researched. Impact: an
  implicit expected value invites inconsistent interpretation (cumulative vs
  per-round). Suggestion: state the expected values, e.g. "sets `round_count: 1`
  and `finding_count` equal to the number of outstanding focus areas researched
  this round".
- 🔵 **minor** (confidence: medium) — *Finding immutability is asserted but no
  in-slice criterion exercises it* — Location: Acceptance Criteria (conduct) /
  Assumptions
  The `conduct` criterion and Assumptions assert findings are "immutable once
  written", but nothing in this single-round slice re-runs a verb over an existing
  finding, so no criterion actually verifies that a written finding is not
  overwritten (re-invocation and gap-detection are deferred to 0279). Impact: a key
  contract invariant that later slices rely on is asserted without a verification
  path within this work item's scope. Suggestion: either add a check that re-running
  `conduct` with all findings present leaves existing finding files byte-unchanged,
  or explicitly note that immutability verification lands with the re-invocation
  work in 0279.
- 🔵 **minor** (confidence: medium) — *paths.research_topics key + default has no
  dedicated verification beyond the aggregate check* — Location: Requirements /
  Acceptance Criteria (config key)
  The first Requirement is a `paths.research_topics` config key plus default "so
  the skill locates the set directory", but no acceptance criterion verifies the
  key exists or that its default resolves; it is only implicitly covered by the
  final criterion asserting the key "lands ... with tests green". Impact: the
  default path could be misconfigured or the key absent while the aggregate
  `mise run check` still passes. Suggestion: add a criterion asserting
  `paths.research_topics` resolves to its documented default when unset, and that a
  bare slug resolves against it.
- 🔵 **suggestion** (confidence: medium) — *Process and rubric qualifiers
  ("interactively", "parallel", rubric sizing) are only proxy-verifiable* —
  Location: Acceptance Criteria (brief, outline, conduct)
  Several criteria embed qualifiers that resist direct verification: `brief`
  "interactively scopes it", `conduct` "spawns parallel `researcher` agents", and
  `outline` sizes focus areas "by the rubric" (whose 1 / 2-4 / 10+ mapping is
  inherently LLM-judged). Only observable proxies are testable — the files and
  frontmatter produced, and the count <= `breadth` ceiling. Impact: these
  qualifiers cannot be conclusively confirmed by a test. Suggestion: where the
  intent is a verifiable outcome, restate it as one (e.g. "one finding is written
  per outstanding focus area" rather than "spawns parallel agents"); otherwise flag
  the process qualifiers as descriptive, non-verified context.
- 🔵 **suggestion** (confidence: medium) — *The researcher-agent criterion mixes a
  checkable negative with an unverifiable forward reference* — Location: Acceptance
  Criteria (generic researcher agent)
  The criterion "The generic `researcher` agent is specialised for web purely by
  injected profile — no per-source agent — the seam Slice 3 (0280) later proves for
  academic sources" bundles a verifiable structural check (no per-source agent file
  exists; only one generic researcher) with a claim about what a future slice
  "later proves", which cannot be verified within this work item. Impact: the
  forward-referenced portion is not falsifiable in this slice. Suggestion: reduce
  the criterion to its checkable form for this slice, e.g. "exactly one generic
  `researcher` agent exists and is spawned with the web profile injected; no
  web-specific agent definition is present", and drop the Slice 3 forward reference
  into Context.

---

## Re-Review (Pass 2) — 2026-09-08

**Verdict:** COMMENT

All three majors that drove the REVISE are resolved; one new medium-confidence
testability major surfaced (rubric-sizing tautology) but sits below the
two-major REVISE threshold, so the work item is now acceptable with residual
improvements. Re-ran all five lenses against the edited work item.

### Previously Identified Issues

- 🟡 **Testability**: Reputation tier has no enumerated valid-value set —
  **Resolved**. The closed set `{tier-1, tier-2, tier-3}` is enumerated in
  Requirements and AC 8; the re-review calls the conformance check "mechanically
  verifiable".
- 🟡 **Testability**: Three-form set-handle resolution has no acceptance criterion —
  **Resolved**. New AC 5 asserts all three forms resolve to the same set root;
  the re-review calls it "an exemplary input/output test".
- 🟡 **Dependency**: Co-land with 0278 encoded only as one-directional blocks + prose —
  **Partially resolved** (downgraded to minor). `relates_to: 0278` plus the
  cycle-avoidance rationale are now recorded; the graph edges still express
  ordering rather than enforced simultaneity, so a residual minor remains.
- 🔵 **Dependency**: Gated consumer 0281 absent from blocks frontmatter — **Resolved**.
  0281 added to `blocks`; Dependencies notes the canonical-side convention.
- 🔵 **Testability**: `conduct` sets counts without expected values — **Resolved**.
  AC 3 pins `round_count: 1` and `finding_count` = focus areas researched.
- 🔵 **Testability**: `paths.research_topics` has no dedicated verification —
  **Partially resolved**. New AC 6 checks default resolution, but the concrete
  default value is not pinned (see new issues).
- 🔵 **Testability**: Finding immutability asserted but no in-slice criterion —
  **Partially resolved**. A Technical Note now states it is first exercised in
  0279; the re-review still flags AC 3's "immutable" qualifier as untestable
  in-slice (see new issues).
- 🔵 **Dependency**: Config-key wiring split leaves engine-check satisfiability unstated —
  **Resolved**. AC 12 now states the engine's checks are green in isolation.
- 🔵 **Clarity**: AC3 "outstanding" diverges from Requirements — **Resolved**. AC 3
  now states every focus area is outstanding in this single round.
- 🔵 **Clarity**: Core term "set" defined only in the epic — **Resolved**. Glossed
  inline at first use in the Summary.
- 🔵 **Clarity**: Inert `depth: 1` default unexplained — **Resolved**. Requirements
  now note `depth` governs intra-finding recursion, inert at `depth: 1`.
- 🔵 **Completeness**: User/consumer implicit for a story — **Resolved**. Context now
  opens with the Accelerator-user beneficiary sentence.
- 🔵 **Scope**: Hard co-land means 0277 not independently deliverable — **Still
  present** (suggestion). The re-review accepts the encoding and rationale;
  no change needed if the split holds.
- 🔵 **Scope**: Engine bundles a large surface — **Resolved / accepted**. The
  re-review calls the size "genuinely indivisible", no longer a concern.
- 🔵 **Testability**: Process/rubric qualifiers proxy-verifiable — **Partially
  resolved**. The `conduct` "parallel agents" qualifier gave way to an observable
  outcome; the rubric-sizing qualifier is now elevated to a new major (below).
- 🔵 **Testability**: Researcher-agent forward reference — **Resolved**. AC trimmed
  to "exactly one generic `researcher` agent … no web-specific agent definition".
- 🔵 **Dependency**: External WebFetch coupling not noted — **Resolved**. Captured
  in Assumptions.

### New Issues Introduced

- 🟡 **Testability**: "Sized … by the rubric" has no pass/fail beyond the breadth
  ceiling (AC 2). Whether a subject is "simple" vs "a comparison" is LLM-judged,
  so any count ≤ 8 can be claimed as "what the rubric decided" — tautological
  beyond AC 7's ceiling. (medium confidence; a re-framing of a prior suggestion.)
- 🔵 **Testability**: AC 6's "documented default" path value is never stated —
  presumably `meta/research/topics`, but not pinned, so the resolved path has no
  definite expected value.
- 🔵 **Testability**: Hardcoded `breadth: 8` / `depth: 1` are not pinned by any
  criterion — a wrong default (e.g. `breadth: 4`) would pass every current check.
- 🔵 **Testability**: AC 3's "immutable" qualifier has no in-slice verification
  procedure (re-raised) — scope the criterion to the observable and cite 0279.
- 🔵 **Clarity**: In Context, the pronoun "It" ("0278 makes them browsable. It is
  built on the reusable-infrastructure seam…") has 0278 as nearest antecedent but
  means the engine (0277).
- 🔵 **Clarity**: The rubric's "10+ for broad subjects" reads as conflicting with
  the `breadth: 8` ceiling in Requirements; the reconciliation is only in Drafting
  Notes — fold it inline.
- 🔵 **Completeness**: The five template shapes' required sections are specified
  only by reference to 0121's artifact-contract table (low confidence).
- 🔵 **Testability**: AC 1's "interactively scopes it" has no automated verification
  path (suggestion) — clarify verification is over the produced artifacts.

### Assessment

The work item is ready for implementation as-is: verdict COMMENT, no critical
findings, and the single major is below the REVISE threshold. The residual major
and the concrete-default minors are cheap, high-value testability tightenings —
reframe AC 2's rubric sizing as prompt guidance with the ceiling as the pass/fail,
pin the `meta/research/topics` default and `breadth: 8`, fix the "It" pronoun, and
fold the rubric-versus-ceiling reconciliation inline — and are worth a quick
second edit pass before planning, but none blocks it.

## Approval — 2026-09-08

**Verdict: APPROVE** (reviewer override of the pass-2 COMMENT.)

After the pass-2 re-review, a second edit pass closed the one new major (AC 2
rubric sizing reframed so the `breadth` ceiling is the pass/fail) and pinned the
concrete defaults (`meta/research/topics`, `breadth: 8`, `depth: 1`), fixed the
Context "It" pronoun, and folded the rubric-versus-ceiling reconciliation into
Requirements. No critical or major findings remain. The residual minors and
suggestions — the in-slice-unverifiable immutability and interactivity
qualifiers, the co-land simultaneity that the `blocks`/`relates_to` schema cannot
enforce without a cycle, and the template shapes specified by reference to 0121 —
are all acceptable, so the reviewer approves the work item for implementation.

---
*Review generated by /accelerator:review-work-item*
