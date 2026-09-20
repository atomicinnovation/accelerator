---
type: "work-item-review"
id: "0292-linear-project-pull-filter-review-1"
title: "Work Item Review: Linear Pull Filters via Catalogue-Resolved Ids"
date: "2026-09-21T07:05:55+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0292"
work_item_id: "0292"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-22T08:19:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Linear Pull Filters via Catalogue-Resolved Ids

**Verdict:** REVISE

0292 is a dense, well-structured story with a genuinely unified mechanism
(catalogue-resolved id lowering shared across `project` / `label` / `assignee`),
exhaustive acceptance criteria, and clean scope boundaries with per-item
deferrals. Two major findings hold it back: a dependency claim ("Blocked by:
none") that contradicts the tracked status of 0229, on which the opening
requirement directly depends, and an acceptance criterion (AC 13) that asserts
a server-side outcome the story's own golden-fixture verification cannot check.
The remaining findings are localised — an uncaptured 0227 coupling flagged from
two lenses, a missing positive config-validation criterion, and terminology
wobbles — none structural.

### Cross-Cutting Themes

- **0227 under-referenced** (flagged by: dependency, clarity) — the fifth
  Requirement defers remote validation to "0227's concern", but 0227 appears in
  neither the References section nor `relates_to`, and dependency notes 0227's
  `config validate` command is an uncaptured downstream consumer of the new
  `project` key and the per-tracker schema split. One edit fixes both: add 0227
  to References/`relates_to` with a gloss and record it as a consumer.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: 'Blocked by: none' rests on 0229 being implemented, but 0229 is status 'ready'
  **Location**: Dependencies
  The item declares "Blocked by: none. 0229 … are implemented", yet Context and
  the first Requirement build directly on 0229's deliverables (splitting the
  shared `FILTER_SCHEMA`, converting `label` / `assignee` name-lowering). 0229
  is tracked as `ready` (not `done`) and is itself `blocked_by: 0228` —
  verified against the tracked files during this review. If 0229 has not
  shipped, 0292 has an uncaptured hard upstream blocker and cannot begin its
  opening requirement.

- 🟡 **Testability**: AC 13 asserts a server-side result-set outcome the golden-fixture strategy cannot verify
  **Location**: Acceptance Criteria
  AC 13 ("issues with no project are excluded") asserts what Linear returns,
  whereas every other criterion — and the stated verification approach (the
  hand-written golden fixture `issue-filter.txt`) — checks the constructed
  `IssueFilter` shape. Exclusion of project-less issues is Linear's server-side
  semantics; the fixture cannot pin it, so a verifier following the story's own
  procedure cannot conclusively pass or fail it.

#### Minor

- 🔵 **Scope**: Additive `project` filter bundled with behaviour-changing `label` / `assignee` id conversion
  **Location**: Summary
  The Summary joins a purely additive `project` filter with converting shipped
  `label` / `assignee` matching from live `eqIgnoreCase` to catalogue-resolved
  ids — a change carrying a one-time re-init migration and distinct rollback
  profile. The author has justified the bundle (shared mechanism, single
  migration event), so this is a judgement call rather than a delivery risk.

- 🔵 **Testability**: No positive criterion for Linear accepting `project` at config validation
  **Location**: Acceptance Criteria
  The Requirements state the split must make `project` accepted under Linear and
  rejected under Jira, but only the rejection half has a criterion (AC 9). AC 10
  presupposes an accepted `project` filter rather than verifying acceptance, so
  a regression where Linear wrongly rejected `project` at config time could pass
  the current AC set.

- 🔵 **Testability**: AC 12 'unchanged from today' is a relative baseline with no named observable
  **Location**: Acceptance Criteria
  AC 12 is defined against an implicit "today" baseline and names no concrete
  observable, so pass/fail depends on a captured reference the criterion does
  not identify — and cannot be re-checked once "today" has moved.

- 🔵 **Dependency**: 0227 config-validate consumer of the new `project` key / per-tracker split is uncaptured
  **Location**: Requirements
  0292 adds a new Linear `project` key and splits the per-tracker schema, and
  defers remote validation to 0227, yet 0227 appears in no dependency slot. Its
  `config validate` command will not learn of the new key or the split, so the
  config-validate lane can silently lag this change.

- 🔵 **Dependency**: Linear GraphQL API external-system coupling not surfaced in Dependencies
  **Location**: Dependencies
  0292 depends throughout on the Linear GraphQL API (new paginated
  `projects` / `issueLabels` / `users` fetches, id comparators,
  `EntityIdentifierIDComparator`), and Technical Notes flag raised complexity
  metering. Siblings 0048 and 0229 record the Linear API as an explicit
  external-system dependency; 0292's Dependencies section omits it.

- 🔵 **Clarity**: 'team' categorised inconsistently as both a scope noun and a filter
  **Location**: Context
  Context first separates "team scope nouns" from "the three current filters",
  then groups `team` with the filters ("`state` and `team` already are"
  id-keyed); the Drafting Notes repeat this. A reader cannot tell whether a
  `team` filter key exists in the same schema as `label` / `state` /
  `assignee` / `project`, which the Requirements never introduce.

- 🔵 **Clarity**: Example under 'several to `in`' shows a single-value `eqIgnoreCase` form
  **Location**: Context
  The clause "several to `in` (e.g. `labels: { name: { in } }`,
  `assignee: { name: { eqIgnoreCase } }`)" lists a single-value comparator
  (`eqIgnoreCase`) as an example of the `in` form, contradicting the clause it
  sits under.

#### Suggestions

- 🔵 **Testability**: 'refuses loudly' lacks a defined observable of the refusal itself
  **Location**: Acceptance Criteria
  ACs 7 and 8 require the pull to "refuse loudly". The message-content half is
  testable, but "loudly" does not define the refusal channel (non-zero exit,
  aborted pull, stderr) — an implementation that logs a warning yet proceeds
  could be argued to satisfy it.

- 🔵 **Testability**: 'Structural validation only — no remote-existence check' has no verifying criterion
  **Location**: Requirements
  The Requirements assert config-time validation stays offline, but no
  criterion confirms this negative behaviour; a regression adding a config-time
  remote lookup would pass the current AC set.

- 🔵 **Clarity**: '0227' referenced in prose but absent from References and frontmatter
  **Location**: Requirements
  The bare id `0227` carries no gloss and appears in neither References nor
  `relates_to`, unlike every other cross-referenced item.

- 🔵 **Clarity**: Remediation command hedged as `init-linear` 'or an equivalent discovery refresh'
  **Location**: Requirements
  The operator-facing remedy is left as "`init-linear` (or an equivalent
  discovery refresh)", so the exact command the criterion directs the operator
  to run is unpinned.

- 🔵 **Clarity**: Same actor referred to as 'developer', 'operator', and 'users'
  **Location**: Requirements
  The person running the pull is "a developer" (Summary), "the operator"
  (Requirements/AC), and "users" (Assumptions). No real ambiguity, but a minor
  consistency snag.

- 🔵 **Completeness**: Stale parent-epic reference noted but not tracked as a follow-up
  **Location**: Drafting Notes
  The notes acknowledge parent epic 0146's Stories entry still describes 0292 as
  `project.name` lowering and is "now stale", but the loose end is a passing
  aside rather than a tracked follow-up, so the correction may be forgotten once
  this item ships.

### Strengths

- ✅ Acceptance criteria state outcomes as concrete emitted `IssueFilter` states
  (e.g. `project: { id: { eq: <alpha-uuid> } }`, including `eq` vs `in`
  cardinality), giving unambiguous pass/fail — eleven of thirteen are
  Given/When/Then pairs with observable outputs.
- ✅ The tiered `assignee` resolution (email → full name → display name, first
  unique tier wins, any tier with two or more matches refuses) is described
  identically across Summary, Requirements, Assumptions, Drafting Notes, and
  Acceptance Criteria — no drift.
- ✅ Genuinely unified internal mechanism: the catalogue-fetch extension is
  enabling infrastructure with no standalone value, so folding it into the same
  increment correctly avoids shipping a half-feature; the story fits cleanly
  within parent epic 0146 and partitions work with 0293.
- ✅ Out-of-scope is explicit and each deferral names its owner (negation → 0293,
  Jira `project` ruled redundant, nested AND/OR → 0146 future candidates).
- ✅ The 0292→0293 blocking relationship is captured bidirectionally and
  consistently, the catalogue prior-art (0048, 0220) is genuinely resolved
  (`done`), and the cold-start / re-init migration is surfaced explicitly in
  both Requirements and Assumptions.

### Recommended Changes

1. **Reconcile the 'Blocked by: none' claim with 0229's tracked status**
   (addresses: 'Blocked by: none' rests on 0229 being implemented). 0229 is
   `ready`, not `done`, and is `blocked_by: 0228` (which is `done`). Either add
   0229 to `blocked_by` (noting 0228 sits behind it), or, if 0229 genuinely
   shipped, correct 0229's status to `done` so the claim holds.

2. **Define AC 13's verification means** (addresses: AC 13 asserts a
   server-side result-set outcome). Reframe it as a filter-shape assertion
   ("the constructed filter is a positive `project: { id }` constraint, not a
   negation"), or explicitly designate it as a live/integration test against a
   tenant containing project-less issues.

3. **Add a positive Linear-accepts-`project` config-validation criterion, and
   pin AC 12 to a concrete observable** (addresses: no positive criterion for
   Linear accepting `project`; AC 12 'unchanged from today'). Add "Given
   `project` under an active Linear integration, when configuration is
   validated, then it is accepted (no `UnsupportedFilterKey`)", and reword AC 12
   to name the no-filter `issue-filter.txt` golden fixture as its baseline.

4. **Capture the 0227 coupling** (addresses: 0227 config-validate consumer
   uncaptured; '0227' absent from References/frontmatter). Add 0227 to
   References and `relates_to` with a one-line gloss, and record it as a
   downstream consumer that must extend `config validate` to the new `project`
   key and the per-tracker accepted-set split.

5. **Add the Linear GraphQL API as an external-systems dependency line**
   (addresses: Linear GraphQL API coupling not surfaced). Mirror 0048 and 0229 —
   name the API and the added-fetch complexity / rate-limit implication in the
   Dependencies section, not only Technical Notes.

6. **Fix the two Context terminology wobbles** (addresses: 'team' categorised
   inconsistently; example under 'several to `in`'). Reserve "filter" for the
   `filters`-bag keys and describe `team` as a scope noun (clarifying that
   "uniformly id-keyed" refers to the emitted `IssueFilter`), and give the
   "several to `in`" clause only `in`-form examples.

7. **Pin the refusal observable and, optionally, the structural-validation
   criterion** (addresses: 'refuses loudly' lacks an observable; 'structural
   validation only' has no criterion). Replace "refuses loudly" with the
   concrete failure observable (non-zero exit, no issues fetched, named
   entity), and consider a criterion asserting config validation makes no
   remote call.

8. **Polish the remaining clarity/completeness items** (addresses: remediation
   command hedged; actor naming; stale parent-epic reference; scope bundle).
   Name the exact re-init command once, settle on a single actor noun, track
   the 0146 Stories-entry correction as an explicit follow-up, and state the
   `project` / `label`-`assignee` bundle as a deliberate accepted trade-off.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: 0292 is a dense but generally unambiguous work item: pronouns
resolve cleanly, the assignee-resolution tiering is described consistently
across Summary/Requirements/Assumptions/Acceptance Criteria, and outcomes are
stated as concrete emitted-GraphQL states. The clarity weaknesses are localised
terminology wobbles rather than structural contradictions: the word "filter"
shifts referent (notably whether `team` is a filter or a scope noun), one
illustrative example contradicts the clause it sits under, and a couple of
referents (`0227`, the exact re-init command) are left under-specified. None
threaten the core intent, so all findings are minor or suggestions.

**Strengths**:
- Acceptance Criteria state outcomes as observable emitted-filter states (e.g.
  `project: { id: { eq: <alpha-uuid> } }`) rather than vague properties.
- The assignee email→full-name→display-name resolution order and the "first
  unique tier wins, any tier with two or more matches refuses" rule are
  described identically in Requirements, Assumptions, Drafting Notes, and
  Acceptance Criteria — no drift.
- Requirements use active imperative voice with explicitly named
  actors/resolvers (`CatalogueProjects`, `CatalogueLabels`, `CatalogueUsers`).
- In-scope vs Out-of-scope is crisply partitioned, and cross-references to
  sibling items (0229, 0293) carry a one-line gloss.

**Findings**:
- 🔵 minor (confidence medium) — **'team' categorised inconsistently as both a
  scope noun and a filter** (Location: Context). Context first distinguishes
  "the team scope nouns" from "the three current filters", but then states the
  story leaves "all Linear filters uniformly id-keyed (`state` and `team`
  already are)", grouping `team` with the filters; the Drafting Notes repeat
  this. A reader cannot tell whether `team` is a member of the `filters` bag or
  a scope noun, and "filter" shifts between the config `filters` bag and the
  broader emitted `IssueFilter`. Impact: a reader could infer a `team` filter
  key exists in the same schema as `label` / `state` / `assignee` / `project`,
  which the Requirements and Acceptance Criteria never introduce. Suggestion:
  use one consistent term (reserve "filter" for the `filters`-bag keys, describe
  `team` as a scope noun) and rephrase "uniformly id-keyed" to refer to the
  emitted `IssueFilter` representation, where team scope is already id-keyed per
  0220.
- 🔵 minor (confidence medium) — **Example under 'several to `in`' shows a
  single-value `eqIgnoreCase` form** (Location: Context). The clause "several to
  `in` (e.g. `labels: { name: { in } }`, `assignee: { name: { eqIgnoreCase } }`)"
  lists the single-value comparator as an `in` example. Impact: the illustrative
  example contradicts the clause it sits under. Suggestion: give the "several to
  `in`" clause only `in`-form examples, or move the `eqIgnoreCase` example next
  to the "one value" clause it actually illustrates.
- 🔵 suggestion (confidence medium) — **'0227' referenced in prose but absent
  from References and frontmatter** (Location: Requirements). The fifth
  Requirement states "remote validation stays 0227's concern", but 0227 is in
  neither References (which names 0229, 0293, 0146, 0048, 0220) nor `relates_to`.
  Impact: the reader must guess what 0227 covers. Suggestion: add 0227
  (accelerator config validate command) to References with a short gloss.
- 🔵 suggestion (confidence low) — **Remediation command hedged as `init-linear`
  'or an equivalent discovery refresh'** (Location: Requirements). The remedy is
  given as "re-run `init-linear` (or an equivalent discovery refresh)" while
  Technical Notes name the underlying path as `discover_team` →
  `write_catalogue`. Impact: the observable remediation is not pinned to one
  concrete command. Suggestion: state the exact operator-facing command once and
  reuse it verbatim.
- 🔵 suggestion (confidence low) — **Same actor referred to as 'developer',
  'operator', and 'users'** (Location: Requirements). The person configuring and
  running the pull is "a developer" (Summary), "the operator" (Requirements/AC),
  and "users" (Assumptions). Impact: negligible in this CLI context. Suggestion:
  optionally settle on a single term.

### Completeness

**Summary**: An exceptionally complete story: all expected sections (Summary,
Context, Requirements with an explicit Out-of-scope subsection, Acceptance
Criteria, Dependencies, Assumptions, Technical Notes, Drafting Notes,
References) are present and substantively populated, and the frontmatter is
complete and valid (kind=story, status=draft, priority set, parent/blocks/
relates_to populated). The Summary is a full who/what/why user story, Context
explains the motivating problem, and the 13 Given/When/Then acceptance criteria
far exceed the minimum while covering positive, negative, collision,
cross-tracker, and migration cases. No critical or major completeness gaps; the
only observation is a self-documented stale parent-epic reference.

**Strengths**:
- Summary is a complete user story in as-a / I-want / so-that form.
- Context explains why the work is needed, grounding the change in 0229's shared
  filter schema and the mutable/non-unique-names problem rather than restating
  the Summary.
- Acceptance Criteria contains 13 specific Given/When/Then criteria spanning
  single/multiple projects, each assignee tier, collision refusal, absent-entity
  refusal, cross-tracker rejection, catalogue persistence, and the no-filter
  baseline.
- Requirements are implementer-ready with an explicit Out-of-scope subsection;
  Dependencies and Assumptions are both populated, including a cold-start /
  migration assumption.
- Frontmatter integrity is strong: recognised kind, appropriate status, and
  parent/blocks/relates_to all present.

**Findings**:
- 🔵 suggestion (confidence medium) — **Stale parent-epic reference noted but not
  tracked as a follow-up** (Location: Drafting Notes). The notes acknowledge
  0146's Stories entry still describes 0292 as `project.name` lowering and is
  "now stale", but this is captured only as a passing aside (no Open Questions
  section, not in Dependencies). Impact: a reader navigating from the parent epic
  meets a contradictory scope description, and nothing tracks the correction.
  Suggestion: capture the parent-epic update as an explicit follow-up.

### Dependency

**Summary**: 0292 captures its downstream consumer (0293) cleanly and
bidirectionally, correctly grounds its catalogue prior-art on the done items
0048/0220, and surfaces the cold-start re-init migration explicitly. The
principal gap is upstream: the "Blocked by: none" claim rests on 0229 being
"implemented", but the referenced 0229 is still status "ready" (and is itself
blocked_by 0228), so a hard upstream blocker may be uncaptured. Secondary
couplings — to 0227's config-validate command and to the Linear GraphQL API as
an external system — are acknowledged in prose but absent from the Dependencies
section where sibling items record them.

**Strengths**:
- The 0292→0293 blocking relationship is captured bidirectionally and
  consistently in both items' prose and frontmatter.
- The catalogue prior-art dependencies (0048, 0220) are named and genuinely
  resolved — both are status "done".
- The cold-start / migration coupling is explicitly captured in both
  Requirements and Assumptions as a documented one-time migration step.
- Internal ordering is coherent: the catalogue-discovery extension is scoped to
  precede/accompany the id-resolution lowering within the same story.

**Findings**:
- 🟡 major (confidence high) — **'Blocked by: none' rests on 0229 being
  implemented, but 0229 is status 'ready'** (Location: Dependencies). 0292's
  Dependencies declares "Blocked by: none. 0229 … are implemented", yet Context
  and the first Requirement build directly on 0229's deliverables. 0229 is
  status "ready" (not "done") and is itself `blocked_by: 0228`; done siblings
  (0048, 0220) are marked "done". Impact: if 0229 is not implemented, 0292 has
  an uncaptured hard upstream blocker (0229, transitively 0228) and cannot begin
  its opening requirement of splitting a `FILTER_SCHEMA` that 0229 has not yet
  created. Suggestion: reconcile the claim — add 0229 to `blocked_by`, or update
  0229 to "done" if it shipped. (Verified during this review: 0229 is `ready`,
  `blocked_by: 0228`; 0228 is `done`.)
- 🔵 minor (confidence medium) — **0227 config-validate consumer of the new
  `project` key / per-tracker split is uncaptured** (Location: Requirements).
  0292 adds a new Linear `project` key and splits the per-tracker schema, and
  defers remote-existence validation to 0227, yet 0227 appears nowhere in
  Dependencies, References, or `relates_to`. Impact: 0227's validator will not
  know about the new `project` key or the split, so the config-validate lane
  silently lags. Suggestion: name 0227 as a downstream consumer.
- 🔵 minor (confidence medium) — **Linear GraphQL API external-system coupling
  not surfaced in Dependencies** (Location: Dependencies). 0292 depends
  throughout on the Linear GraphQL API (new paginated fetches, id comparators,
  `EntityIdentifierIDComparator`), and Technical Notes flag raised complexity
  metering. Siblings 0048 and 0229 record the Linear API as an explicit
  external-system dependency; 0292 omits it. Suggestion: add an "External
  systems" line naming the Linear GraphQL API and the added-fetch complexity /
  rate-limit implication.

### Scope

**Summary**: 0292 is a coherent, well-bounded story: every requirement serves
one unifying mechanism — resolving Linear's name-based pull filters to stable
catalogue-resolved ids — and the enabling catalogue-fetch infrastructure has no
standalone value apart from the filters that consume it, so bundling it in is
correct. It fits cleanly inside parent epic 0146's pull-scope/filters theme and
hands off cleanly to 0293. The one scope observation is that the story couples a
purely additive new `project` filter with a behaviour-changing conversion of
already-shipped `label` / `assignee` matching; the author has explicitly
justified this bundle, so it is a judgement call rather than a delivery risk.

**Strengths**:
- The Out of scope section is explicit and each deferral names its owner
  (negation → 0293, Jira `project` ruled redundant, nested AND/OR → 0146,
  config-time name→id resolution excluded).
- Clean sequential decomposition against 0293: this story establishes the
  id-keyed lowering and 0293 layers polarity over exactly those forms.
- Coherent fit within parent epic 0146 and a genuinely unified internal
  mechanism; the catalogue-fetch extension is enabling infrastructure with no
  standalone value.
- Summary, Requirements, and Acceptance Criteria describe the same scope, with
  no criterion reaching beyond the stated boundaries.

**Findings**:
- 🔵 minor (confidence medium) — **Additive `project` filter bundled with
  behaviour-changing `label` / `assignee` id conversion** (Location: Summary).
  The Summary joins a new `project` filter (additive) with converting every
  Linear name-based filter to a stable id; the second clause changes shipped
  `label` / `assignee` matching from live `eqIgnoreCase` to catalogue-resolved
  ids, requiring a one-time re-init migration. The two concerns share the
  resolution mechanism but have distinct value and rollback profiles. Impact: a
  migration-bearing behaviour change rides in on an otherwise additive story.
  Suggestion: keep the bundle but state the deliberate coupling and its
  single-migration rationale as an accepted trade-off, or land the additive
  `project` filter first with the conversion as an immediate follow-on.

### Testability

**Summary**: This story is unusually strong on testability: eleven of thirteen
acceptance criteria are framed as Given/When/Then pairs with concrete,
observable outputs — mostly the exact GraphQL `IssueFilter` shape — and
refusal/ambiguity behaviour is explicitly enumerated per resolution tier. The
main weaknesses are two criteria that assert a different, undefined verification
level than the rest (a server-side result-set claim and a relative "unchanged
from today" baseline), plus a coverage gap where the stated Linear-accepts-
`project` requirement lacks its own positive criterion.

**Strengths**:
- Nearly all acceptance criteria are observable input→output pairs with concrete
  expected outputs (the exact lowered `IssueFilter` shapes, including eq vs in
  cardinality).
- Refusal behaviour is testable via specified message content: AC 7 requires
  naming the collision, AC 8 requires naming the missing entity and the
  init-linear remedy.
- The tiered `assignee` resolution has one criterion per tier (ACs 4–6) with
  Given clauses that isolate each tier.
- Catalogue persistence has a concrete observable target — AC 11 asserts exact
  `catalogue.json` contents — and Technical Notes name the golden fixture
  `issue-filter.txt` as the verification pin.

**Findings**:
- 🟡 major (confidence medium) — **AC 13 asserts a server-side result-set outcome
  the golden-fixture strategy cannot verify** (Location: Acceptance Criteria).
  AC 13 ("issues with no project are excluded") asserts what Linear returns,
  whereas every other criterion and the stated verification approach (the
  hand-written golden fixture) check the constructed `IssueFilter` shape.
  Exclusion of project-less issues is Linear's server-side semantics. Impact: a
  verifier following the golden-fixture procedure cannot conclusively pass or
  fail it, and it risks being claimed as met simply because a positive filter
  was emitted. Suggestion: reframe as a filter-shape assertion, or explicitly
  designate it as requiring a live/integration test.
- 🔵 minor (confidence medium) — **AC 12 'unchanged from today' is a relative
  baseline with no named observable** (Location: Acceptance Criteria). AC 12 is
  defined against an implicit "today" baseline and names no concrete observable.
  Impact: two verifiers could disagree on whether behaviour changed, and it
  cannot be re-checked after "today" has moved. Suggestion: pin it to a concrete
  observable — the no-filter `issue-filter.txt` golden fixture.
- 🔵 minor (confidence medium) — **No positive criterion for Linear accepting
  `project` at config validation** (Location: Acceptance Criteria). Only the
  rejection half has a criterion (AC 9); AC 10 presupposes acceptance rather
  than verifying it. Impact: a regression where Linear wrongly rejected
  `project` at config time could pass the current AC set. Suggestion: add
  "Given `project` under an active Linear integration, when configuration is
  validated, then it is accepted (no `UnsupportedFilterKey` error)."
- 🔵 suggestion (confidence medium) — **'refuses loudly' lacks a defined
  observable of the refusal itself** (Location: Acceptance Criteria). ACs 7 and
  8 require "refuse loudly"; the message-content half is testable, but "loudly"
  does not define the observable (non-zero exit, aborted pull, stderr). Impact:
  an implementation that logs a warning yet proceeds could be argued to satisfy
  it. Suggestion: replace with the concrete failure observable.
- 🔵 suggestion (confidence low) — **'Structural validation only — no
  remote-existence check' has no verifying criterion** (Location: Requirements).
  No criterion confirms config validation stays offline. Impact: a regression
  adding a config-time remote lookup would not be caught. Suggestion: add a
  criterion asserting validation passes with no remote call for a non-existent
  named entity, refused only at pull time.

## Re-Review (Pass 2) — 2026-09-22

**Verdict:** COMMENT

Both blocking majors from pass 1 are resolved: 0229 is now tracked `done`, so
the "Blocked by: none" claim holds, and AC 13 is reframed to a fixture-checkable
filter-shape assertion. All five pass-1 minors and every actioned suggestion are
resolved. Re-review surfaced one new major — assignee tier precedence is never
verified across conflicting tiers — but at one major against a revise threshold
of two, the item is acceptable as-is. The item is COMMENT-grade; the new major
is a cheap, worthwhile AC to add before implementation.

### Previously Identified Issues

- 🟡 **Dependency**: 'Blocked by: none' rests on 0229 being implemented —
  **Resolved**. 0229 flipped to `done`; the dependency lens re-verified the
  claim now holds with no hidden upstream blockers.
- 🟡 **Testability**: AC 13 asserts a server-side result-set outcome —
  **Resolved**. Reframed to a positive `project: { id }` filter-shape assertion;
  only a low-severity note remains that the residual "excludes project-less
  issues" clause is unverifiable background.
- 🔵 **Scope**: Additive `project` filter bundled with behaviour-changing
  conversion — **Resolved**. The deliberate coupling and single-migration
  rationale are now stated in Drafting Notes; the lens downgraded it to a
  suggestion.
- 🔵 **Testability**: No positive criterion for Linear accepting `project` —
  **Resolved**. AC added and confirmed by the lens (accepted under Linear,
  rejected under Jira, both directions covered).
- 🔵 **Testability**: AC 12 'unchanged from today' relative baseline —
  **Resolved**. Pinned to the no-filter `issue-filter.txt` golden fixture; now
  read as a negative control.
- 🔵 **Dependency**: 0227 config-validate consumer uncaptured — **Partially
  resolved**. Now in `relates_to` and a Dependencies "Consumed by" line, but the
  lens flags it is absent from the frontmatter `blocks` graph (see new issues).
- 🔵 **Dependency**: Linear GraphQL API external coupling not surfaced —
  **Resolved**. Added as an external-systems Dependencies line with the
  complexity/rate-limit implication; the lens now lists it as a strength.
- 🔵 **Clarity**: 'team' categorised inconsistently as scope noun and filter —
  **Resolved**. Reworded to id-keyed in the emitted `IssueFilter`, `team` a
  scope noun since 0220.
- 🔵 **Clarity**: Example under 'several to `in`' shows `eqIgnoreCase` —
  **Resolved**. Corrected to an `in`-form example.
- 🔵 **Testability**: 'refuses loudly' lacks a defined observable — **Resolved**.
  ACs 7/8 now assert non-zero exit and no issues fetched.
- 🔵 **Testability**: 'Structural validation only' has no criterion —
  **Resolved**. AC 14 added (validation passes with no remote call; refused only
  at pull time).
- 🔵 **Clarity**: '0227' absent from References/frontmatter — **Resolved**. Added
  to References with a gloss and to `relates_to`.
- 🔵 **Clarity**: Remediation command hedged — **Resolved**. Hedge dropped;
  `init-linear` named (superseded by a new naming-consistency note below).
- 🔵 **Clarity**: Actor referred to as developer/operator/users — **Resolved**.
  Assumptions now say "operators".
- 🔵 **Completeness**: Stale 0146 reference untracked — **Partially resolved**.
  Now a tracked follow-up in Drafting Notes; the dependency lens suggests
  surfacing it in the Dependencies graph too (see new issues).

### New Issues Introduced

- 🟡 **Testability** (major, medium): Assignee tier precedence is never verified
  across conflicting tiers. ACs 4–6 isolate each tier ("and no email", "and no
  email or full name"), so a regression where a later tier wins over an earlier
  one would pass undetected. Add an AC where one string matches user X's full
  name and user Y's display name, asserting it resolves to X's id.
- 🔵 **Testability** (minor, medium): Catalogue pagination completeness is
  unverified — no AC checks that entities spanning more than one page are all
  persisted, though the "refuse if absent" design makes a truncated fetch
  consequential.
- 🔵 **Dependency** (minor, medium): 0227 is captured in the "Consumed by" prose
  but not in the frontmatter `blocks` graph, so unblock-computing tooling will
  not surface it when 0292 closes.
- 🔵 **Clarity** (minor, medium): The init operation is named three ways — "init
  discovery", `init-linear`, and `discover_team` → `write_catalogue` — a reader
  must infer they denote one operation.
- 🔵 **Scope** (suggestion, low): The story sits at the upper bound of a single
  increment (schema split, three resolvers, discovery extension, three-tier
  resolution, 16 ACs); a prompt to confirm deliverability, not a defect.
- 🔵 **Clarity** (suggestion, low): The Summary's "every Linear name-based
  filter" momentarily reads as including `state` before Context clarifies.
- 🔵 **Dependency** (suggestion, low): The 0146 correction lives only in Drafting
  Notes prose, not the Dependencies/Blocks graph.
- 🔵 **Testability** (suggestion, low): AC 13's residual "excludes project-less
  issues" clause is an unverifiable remote-semantics note; relocate to
  Context/rationale.

### Assessment

The work item is ready for implementation as-is under the configured thresholds
(COMMENT, one sub-threshold major). The one improvement worth making first is a
cheap AC that makes assignee tier precedence observable through a distinct
resolved id — it closes the only remaining behaviour-level verification gap. The
pagination-completeness and 0227-`blocks`-edge items are optional hardening.

## Re-Review (Pass 3) — 2026-09-22

**Verdict:** COMMENT

The pass-2 major (assignee tier precedence) is resolved and no new major
appeared, so the item has stabilised: 0 critical, 0 major, 5 minor, 3
suggestions across the four re-run lenses (completeness was clean at pass 2 and
was not re-run). The three edits applied after pass 2 — the tier-precedence AC,
the pagination-completeness AC, and the bidirectional 0227 `blocks` edge — all
land as intended; the testability lens now cites precedence and pagination as
strengths. The residual findings are coverage refinements and a wording
mismatch I introduced, none blocking.

### Cross-Cutting Themes

- **0227 relationship described two ways** (flagged by: dependency, clarity) —
  the frontmatter lists 0227 under `blocks` (a hard edge, mirrored by 0227's
  `blocked_by: 0292`), but the Dependencies prose still calls it "Consumed by",
  softer than the graph. Reconcile the prose to the `blocks` set.

### Previously Identified Issues (pass-2 new issues)

- 🟡 **Testability**: Assignee tier precedence never verified across conflicting
  tiers — **Resolved**. The added AC pins precedence through the distinct
  resolved id; the lens now lists it as a strength.
- 🔵 **Testability**: Catalogue pagination completeness unverified —
  **Resolved**. The added multi-page AC gives a definitive observable; now a
  strength.
- 🔵 **Dependency**: 0227 absent from the frontmatter `blocks` graph —
  **Resolved**. Added to `blocks` and mirrored by 0227's `blocked_by: 0292`
  (superseded by a new prose/graph wording mismatch below).
- 🔵 **Clarity**: Init operation named three ways — **Partially resolved**. The
  hedge is gone, but the lens now flags "discovery" naming two processes
  (pull-time issue discovery vs init discovery) — see new issues.
- 🔵 **Clarity**: "every Linear name-based filter" reads as including `state` —
  **Resolved**. Not re-raised.
- 🔵 **Testability**: AC 13 residual remote-semantics clause — **Resolved**. Not
  re-raised.
- 🔵 **Scope**: Story sits at the upper bound of a single increment — **Still
  present** (suggestion). Re-raised, but the lens explicitly concludes no change
  is required; the single-migration rationale makes it indivisible.
- 🔵 **Dependency**: 0146 correction only in Drafting Notes prose — **Still
  present** (suggestion). The lens reiterates surfacing it as a tracked
  follow-up rather than a Drafting Note.

### New Issues Introduced

- 🔵 **Dependency / Clarity** (minor): The 0292→0227 relationship is "Consumed
  by" in the Dependencies prose but a hard `blocks` edge in frontmatter (and in
  0227's `blocked_by`). Reconcile to one characterisation.
- 🔵 **Testability** (minor, high): eq/in cardinality is only half-covered for
  `label` and `assignee` — `label` is tested only multi-value (`in`), `assignee`
  only single-value (`eq`). Add `label: [bug]` → `eq` and `assignee: [ada, bob]`
  → `in` to mirror how `project` pins both.
- 🔵 **Testability** (minor, medium): The migration-bearing refusal for the
  newly-converted `label`/`assignee` filters rests on the generic
  absent-entity criterion; a tester could satisfy it with `project` alone. Add
  explicit `label`/`assignee` absent-from-catalogue refusal criteria.
- 🔵 **Clarity** (minor): "discovery" names two processes — the Summary's
  pull-time issue narrowing and the init-time catalogue fetch. Qualify the
  Summary term.
- 🔵 **Clarity** (suggestion, low): The domain verb "lowering" (config value →
  emitted `IssueFilter`) is used without a gloss; define it once on first use or
  link to 0229.

### Assessment

The work item is ready for implementation. It has stabilised at COMMENT with no
major or critical findings after three passes. The remaining items are optional
polish: reconcile the "Consumed by"/`blocks` wording for 0227 (a one-line prose
fix), and, if fuller AC coverage is wanted, add the two cardinality criteria and
the label/assignee migration-refusal criteria. None block planning.

## Verdict Decision — APPROVE (2026-09-22)

**Verdict:** APPROVE

Toby Clemson accepted the work item for implementation after three review
passes. The item carries no critical or major findings; the residual five minor
and three suggestion findings are coverage refinements and a self-introduced
"Consumed by"/`blocks` wording nit, all judged non-blocking and left as optional
follow-ups. The work item's status was transitioned draft → ready alongside this
decision.
