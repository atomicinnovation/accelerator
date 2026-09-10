---
type: "work-item-review"
id: "0228-layered-configuration-key-model-review-1"
title: "Work Item Review: Layered Configuration Key Model"
date: "2026-09-10T01:28:32+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0228"
work_item_id: "0228"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-10T07:13:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Layered Configuration Key Model

**Verdict:** REVISE

0228 is a strong, precise work item — complete across every section, tightly
scoped to a single coherent key-model change, and internally consistent in its
scope-key / local-prefix terminology. The REVISE verdict rests entirely on
testability: two major gaps where deliberately-decided behaviours stated in
Requirements are not pinned by any acceptance criterion (the first criterion's
unverifiable "it functions", and the both-set equal-value / no-warning case).
The remaining findings are localised clarity, dependency-placement, and
coverage refinements — none blocking, most cheap to address.

### Cross-Cutting Themes

- **Behaviours stated in prose but not pinned by acceptance criteria** (flagged
  by: scope, testability) — the migration/deprecation-alias and
  `init-linear`/`init-jira` deliverables are under-represented in the Summary
  (scope), and several of their decided behaviours lack verifying criteria: the
  both-set equal-value/no-warning case, the tracker-less deprecation warning,
  and the `init` empty-section write path (testability).
- **Reliance on sibling items for meaning and relationships** (flagged by:
  clarity, dependency) — "creation-home entity" and "base scope" are load-bearing
  here but glossed only in parent 0146 (clarity), and real couplings live outside
  Dependencies: the 0220 ordering sits in Technical Notes, and the 0227 and 0230
  relationships are recorded obliquely or not at all (dependency).

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: "It functions" is not a verifiable outcome
  **Location**: Acceptance Criteria (first bullet)
  The first criterion ends "…then it functions without a work-management config."
  "Functions" has no defined observable — almost any non-crash could be argued as
  passing, so the item's central claim (integration skills work with only a scope
  key, no `work.*`) has no concrete test.

- 🟡 **Testability**: Both-set non-error / no-warning behaviour not fully covered
  **Location**: Requirements / Acceptance Criteria (fourth bullet)
  Requirements accept `work.key` alongside the scope key "whatever the values —
  including equal", and Drafting Notes promise no warning on a mirrored prefix.
  AC #4 verifies only the *divergent* case and only "no validation error"; the
  equal-value case and the absence of a *warning* go unverified.

#### Minor

- 🔵 **Clarity**: Dense clause and ambiguous "it" in the scope-key resolution requirement
  **Location**: Requirements (fifth bullet)
  "…the base scope creation and discovery consume; 0229 layers its pull block on
  top of it." omits a relative pronoun and leaves "it" with three candidate
  referents (base scope, creation-home entity, scope key), forcing a re-read.

- 🔵 **Dependency**: 0220 sequencing coupling recorded in Technical Notes, not Dependencies
  **Location**: Dependencies
  Dependencies says "Blocked by: none" while Technical Notes records the item is
  "sequenced after 0220 … this item renames that touchpoint" — the reconciliation
  0220 explicitly asked its later-shipping sibling to perform is discoverable only
  outside the Dependencies section.

- 🔵 **Testability**: Tracker-less legacy read does not verify the deprecation warning
  **Location**: Acceptance Criteria (ninth bullet)
  Requirements say reading a deprecated key emits a warning naming the removal
  release. AC #8 (tracker-backed) asserts it; AC #9 (tracker-less) checks only ID
  rendering, so a silent tracker-less migration would still pass.

#### Suggestions

- 🔵 **Scope**: `kind: task` may under-signal a story-sized increment
  **Location**: Frontmatter: kind
  A user-facing rename, a new required-field validation rule, a tracker-aware
  cross-section migration with a two-release deprecation window, and new `init-*`
  write behaviour across ten criteria reads comparable to sibling 0229 (`story`).

- 🔵 **Scope**: Summary omits the migration and init deliverables
  **Location**: Summary
  The Summary describes only the rename and the scope-key/prefix separation, but
  the migration/deprecation-alias and `init-*` overwrite behaviour make up roughly
  a third of the Requirements and three of the ten criteria.

- 🔵 **Clarity**: "creation-home entity" used without a local gloss
  **Location**: Requirements
  The term is load-bearing here but defined only in parent 0146; a reader of 0228
  alone gets only the parenthetical "Jira by identity, Linear by catalogue lookup".

- 🔵 **Clarity**: "tracker prefix" risks conflation with the local ID prefix
  **Location**: Requirements (third bullet)
  `work.key` is "the local ID prefix" throughout, yet "tracker prefix" is used for
  the scope-key value misused as a local prefix — two prefix nouns in one argument.

- 🔵 **Clarity**: Slash in "default_project_code / {project}" leaves the migration condition ambiguous
  **Location**: Acceptance Criteria (eighth bullet)
  Elsewhere "/" means "respectively", but here the two conditions are distinct
  (alias handles `default_project_code`; `work.key` materialises only where
  `id_pattern` used `{project}`), so the compound condition is under-specified.

- 🔵 **Dependency**: New `work.key` validation rule not linked to 0227's config-validate command
  **Location**: Requirements
  Sibling 0229 records a bidirectional 0227 coupling for new config surface; this
  item adds a new `work.key`-required validation rule and deprecation warning with
  no 0227 relationship, risking divergence between `config validate` and load-time
  validation.

- 🔵 **Dependency**: 0230 named as a related concern but not captured as a downstream consumer
  **Location**: Assumptions
  Assumptions defer long-lived-prefix reconciliation to 0230, and 0146 lists 0230
  as building on this key model, yet 0230 is neither a Blocks nor relates_to entry.

- 🔵 **Testability**: "IDs render identically" lacks a concrete reference example
  **Location**: Acceptance Criteria (eighth and ninth bullets)
  No before/after example anchors "identically" (e.g. `default_project_code: PP`
  with `id_pattern: {project}-{number:04d}` rendering `PP-0001`), leaving the
  baseline implicit.

- 🔵 **Testability**: `init` writing the key when none is present is unverified
  **Location**: Requirements / Acceptance Criteria (tenth bullet)
  AC #10 verifies only the overwrite-existing branch; the primary path — writing
  the discovered key into a tracker section that carries none — has no criterion.

### Strengths

- ✅ The two conflated concepts are given explicit, bolded definitions on first
  use (scope key = `linear.team_key` / `jira.project_key`; `work.key` = local ID
  prefix), and the terminology stays stable across every section.
- ✅ Exceptionally complete for a `task`: every optional section (Open Questions,
  Dependencies, Assumptions, Technical Notes, Drafting Notes, References) carries
  substantive content, not placeholders.
- ✅ Tightly scoped to a single purpose — the migration alias is correctly kept
  with the rename (indivisible), and boundaries with 0229, 0220, and 0230 are
  stated explicitly.
- ✅ The downstream consumer 0229 is captured consistently with rationale in
  frontmatter, Dependencies, and References, and the existing team-key→UUID
  resolver is flagged as present so it is not mistaken for a blocker.
- ✅ Acceptance criteria are numerous, well-differentiated Given/When/Then
  behaviours covering tracker-backed, tracker-less, divergent-key, no-`{key}`,
  precedence, and both legacy-migration paths, with the deprecation window pinned
  to concrete releases.

### Recommended Changes

1. **Replace "it functions" with a concrete observable** (addresses: "It functions"
   is not a verifiable outcome). Rewrite AC #1's outcome to, e.g., "the skill
   resolves its scope key from `<tracker>.<entity>_key` and completes its
   search/show operation without raising a missing-`work.*`-config error."

2. **Add a criterion for the both-set equal-value case, asserting no warning**
   (addresses: Both-set non-error / no-warning behaviour not fully covered). E.g.
   "Given `work.key` equal to the scope key, when configuration is validated, then
   no validation error and no warning are raised." Extend AC #4 to assert absence
   of a warning on the divergent case too.

3. **Verify the deprecation warning on the tracker-less path** (addresses:
   Tracker-less legacy read does not verify the deprecation warning). Extend AC #9
   to also require the warning naming the removal release, matching AC #8.

4. **Move the 0220 ordering into Dependencies** (addresses: 0220 sequencing coupling
   recorded in Technical Notes, not Dependencies). Add "Sequenced after 0220 (done)
   — renames the `work.default_project_code` touchpoint 0220 introduced."

5. **Disambiguate the dense scope-key resolution clause** (addresses: Dense clause
   and ambiguous "it"). Name the relative pronoun ("…the base scope that creation
   and discovery consume") and replace the trailing "it" with "this base scope".

6. **Cover the `init` empty-section write and anchor "IDs render identically"**
   (addresses: `init` writing the key when none is present is unverified; "IDs
   render identically" lacks a concrete reference example). Add an empty-section
   `init` criterion and a worked before/after ID example.

7. **Confirm and, if real, capture the 0227 and 0230 couplings** (addresses: New
   `work.key` validation rule not linked to 0227; 0230 not captured as a downstream
   consumer). Mirror how 0229 records its 0227 relationship; add 0230 as Blocks or
   relates_to if it depends on this key model.

8. **Optional housekeeping** (addresses: `kind: task` may under-signal; Summary
   omits migration and init deliverables; "creation-home entity" and "tracker
   prefix" glosses). Reconsider `story` vs `task`, extend the Summary to name the
   migration and init work, gloss "creation-home entity" on first use, and phrase
   the scope-key-as-local-prefix case without a second "prefix" noun.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: 0228 is an unusually precise and internally consistent work item: it
defines its two load-bearing terms (scope key, local ID prefix) in bold on first
use, explicitly explains what the title's word "layered" refers to, and keeps the
scope-key / `work.key` distinction stable across Summary, Requirements, and
Acceptance Criteria. The remaining clarity risks are localised — one dense,
pronoun-omitting sentence with an ambiguous trailing "it", a term ("creation-home
entity") defined only in the parent, and a couple of near-synonyms that could
momentarily be conflated with the local ID prefix.

**Strengths**:
- The two conflated concepts are given explicit, bolded definitions on first use
  in the Summary, so a reader never has to guess which "key" is meant.
- The Context section pre-empts a likely ambiguity by stating which of the two
  layerings the title's word "layered" names, then defining both.
- Acceptance Criteria are written as Given/When/Then with named actors and
  observable outcomes, so responsibility for each action is clear.
- The scope-key / `work.key` terminology is used consistently across every
  section, with no section silently redefining either term.

**Findings**:
- 🔵 minor (confidence: medium) — **Dense clause and ambiguous "it" in the
  scope-key resolution requirement** (Requirements). The fifth bullet omits its
  relative pronoun ("the base scope [that] creation and discovery consume"),
  "base scope" appears for the first time without introduction, and the trailing
  "it" has three candidate referents. Suggestion: name the pronoun and replace
  "it" with "this base scope".
- 🔵 suggestion (confidence: medium) — **"creation-home entity" used without a
  local gloss** (Requirements). Defined only in parent 0146; add a half-sentence
  gloss on first use since the concept is load-bearing here.
- 🔵 suggestion (confidence: low) — **"tracker prefix" risks conflation with the
  local ID prefix (`work.key`)** (Requirements). Two prefix nouns appear in the
  same argument; use phrasing that names the source ("the scope key's value as the
  local prefix").
- 🔵 suggestion (confidence: low) — **Slash in "default_project_code / {project}"
  leaves the migration condition ambiguous** (Acceptance Criteria). Elsewhere "/"
  means "respectively"; spell out the compound condition here.

### Completeness

**Summary**: 0228 is exceptionally complete for its `task` kind, and in fact
carries story-grade content: an unambiguous Summary, a Context that explains the
motivating problem, specific and actionable Requirements, and ten Given/When/Then
acceptance criteria covering the tracker-backed, tracker-less, precedence,
migration, and init paths. Every optional section is populated with substantive
content rather than placeholders, and the frontmatter is intact with a recognised
`kind` and appropriate `status`. No structural gaps were found.

**Strengths**:
- The Summary states the work as a clear, unambiguous action, leaving no doubt
  about the subject.
- The Context explains the forces behind the work rather than restating the
  Summary.
- Requirements are specific and implementation-ready (rename, required-`work.key`
  validation, no-silent-fallback, tracker-aware alias with pinned window, `init-*`
  overwrite).
- Acceptance criteria are numerous and well-differentiated, exceeding what a task
  kind minimally requires.
- Optional sections carry real content, capturing reasoning and existing-resolver
  facts an implementer needs.

**Findings**: none.

### Dependency

**Summary**: This task is unusually well dependency-mapped: its single downstream
consumer (0229) is captured with rationale in frontmatter, Dependencies, and
References, and the pre-existing team-key→UUID resolver is explicitly flagged as
already present so it is not mistaken for a blocker. The main gaps are placement
and completeness of already-known couplings — the 0220 sequencing lives only in
Technical Notes, and two plausible sibling couplings (0227, 0230) are referenced
obliquely or not at all. None would block the work from starting.

**Strengths**:
- The downstream consumer 0229 is captured consistently and with rationale in
  three places.
- The team-key→UUID resolver is explicitly stated to already exist, pre-empting a
  mistaken read that resolver work is an upstream blocker.
- The release-coordination coupling (1.24.0 ship, 1.25.0 removal, with a slip
  rule) is captured in Requirements, Open Questions, and Assumptions.
- The sequencing relative to bug 0220 is acknowledged in Technical Notes.

**Findings**:
- 🔵 minor (confidence: medium) — **0220 sequencing coupling recorded in Technical
  Notes, not Dependencies** (Dependencies). Add a completed/ordering note under
  Dependencies so the coupling appears where dependency relationships are tracked.
- 🔵 suggestion (confidence: medium) — **New `work.key` config-validation rule not
  linked to 0227's config-validate command** (Requirements). Assess whether 0227
  must learn this rule; if so, capture it as relates_to/Blocks, mirroring 0229.
- 🔵 suggestion (confidence: low) — **0230 named as a related concern but not
  captured as a downstream consumer** (Assumptions). Confirm whether 0230 depends
  on this key model; if so, add it as Blocks or relates_to.

### Scope

**Summary**: 0228 is a coherent, well-bounded unit of work: every requirement
serves the single purpose of introducing the layered key model — the rename, the
scope-key/prefix separation, and the migration path the rename demands. Its
boundaries with siblings are explicit and clean (0229 layers pull scope on top;
0220 deliberately targets the old field; the resolver is reused, not rebuilt). The
main observations are a possible kind mislabel (`task` for story-sized work) and a
Summary that under-represents the migration and init portions of the actual scope.

**Strengths**:
- All requirements orbit a single unified purpose; there is no unrelated second
  concern bundled in.
- The tracker-aware migration/alias is correctly kept with the rename rather than
  split out — the two are genuinely indivisible.
- Boundaries with adjacent work (0229, 0220, 0230) are stated explicitly and
  consistently.
- The item actively resists scope creep by disclaiming new machinery, framing
  itself as wiring and contract rather than new resolution.

**Findings**:
- 🔵 suggestion (confidence: medium) — **`kind: task` may under-signal a
  story-sized increment** (Frontmatter: kind). The user-facing scope across ten
  criteria reads comparable to sibling 0229 (`story`); housekeeping, not a blocker.
- 🔵 minor (confidence: medium) — **Summary omits the migration and init
  deliverables** (Summary). The migration/deprecation-alias and `init-*` behaviour
  are ~a third of the requirements and three criteria; extend the Summary so all
  sections describe the same scope.

### Testability

**Summary**: The Acceptance Criteria are overwhelmingly strong for testability:
nine of ten are framed as concrete Given/When/Then behaviours with observable
outcomes (validation failures naming a field, prefix resolution, precedence,
deprecation warning naming release 1.25.0), and the migration and precedence paths
are well-specified. Two weaknesses stand out: the first criterion rests on the
unverifiable word "functions", and several deliberately-decided behaviours (the
both-set non-error/non-warning case, the tracker-less deprecation warning) are
stated in Requirements/Drafting Notes but not pinned by any criterion.

**Strengths**:
- Most criteria are expressed as observable Given/When/Then behaviours with
  definite pass/fail outcomes.
- The deprecation window is pinned to concrete releases, and AC #8 requires the
  warning to name the removal release — a checkable outcome.
- The core distinction under test (local prefix vs scope key never deriving from
  one another) is verified from multiple angles.

**Findings**:
- 🟡 major (confidence: high) — **"It functions" is not a verifiable outcome**
  (Acceptance Criteria, first bullet). Replace with the specific observable, e.g.
  the skill resolves its scope key and completes its operation without a
  missing-`work.*`-config error.
- 🟡 major (confidence: medium) — **Both-set non-error/non-warning behaviour not
  fully covered by a criterion** (Requirements / Acceptance Criteria, fourth
  bullet). AC #4 verifies only the divergent case and only "no error"; add the
  equal-value case and assert absence of a warning.
- 🔵 minor (confidence: medium) — **Tracker-less legacy read does not verify the
  deprecation warning** (Acceptance Criteria, ninth bullet). Extend AC #9 to
  require the warning naming the removal release, matching AC #8.
- 🔵 suggestion (confidence: low) — **"IDs render identically" lacks a concrete
  reference example** (Acceptance Criteria, eighth and ninth bullets). Anchor with
  a worked before/after example.
- 🔵 suggestion (confidence: low) — **`init` writing the key when none is present
  is unverified** (Requirements / Acceptance Criteria, tenth bullet). Add a
  criterion for the empty-section case.

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** COMMENT

Every finding from pass 1 is resolved. The verdict moves from REVISE to COMMENT:
one new major surfaced (a persistence-semantics ambiguity the first pass did not
reach), which sits below the 2-major REVISE threshold. The remaining new items are
minor refinements and a couple of one-sided couplings the pass-1 edits themselves
introduced. The item is acceptable to implement as-is; the new major is worth a
one-line clarification before planning.

### Previously Identified Issues

- 🟡 **Testability**: "It functions" is not a verifiable outcome — Resolved (AC #1
  now asserts scope-key resolution and completion without a missing-`work.*` error).
- 🟡 **Testability**: Both-set non-error / no-warning behaviour not covered —
  Resolved (AC #4 asserts no warning; a new equal-value criterion added).
- 🔵 **Testability**: Tracker-less legacy read misses the deprecation warning —
  Resolved (AC #9 now requires the 1.25.0 warning).
- 🔵 **Testability**: "IDs render identically" lacks a reference example — Resolved
  (worked `PP-0001` examples in AC #8 and #9).
- 🔵 **Testability**: `init` empty-section write unverified — Resolved (dedicated
  criterion added).
- 🔵 **Clarity**: Dense clause and ambiguous "it" — Resolved (relative pronoun named,
  "it" replaced with "this base scope").
- 🔵 **Clarity**: "creation-home entity" needs a gloss — Resolved (glossed on first
  use).
- 🔵 **Clarity**: "tracker prefix" conflation — Resolved (reworded in Requirements
  and Drafting Notes).
- 🔵 **Clarity**: slash in AC #8 ambiguous — Resolved (compound condition spelled out).
- 🔵 **Dependency**: 0220 coupling in Technical Notes, not Dependencies — Resolved
  (ordering note added to Dependencies).
- 🔵 **Dependency**: 0227 coupling unlinked — Resolved (relates_to added), though the
  re-review argues for a stronger Blocks framing (see below).
- 🔵 **Dependency**: 0230 not captured — Resolved (Blocks edge added), though not yet
  reciprocated in 0230 (see below).
- 🔵 **Scope**: `kind: task` under-signals — Resolved (reclassified to story).
- 🔵 **Scope**: Summary omits migration and init — Resolved (Summary extended).

### New Issues Introduced

- 🟡 **Clarity** (major, medium): Read-time alias vs migration/materialise
  persistence semantics are ambiguous — is a legacy config resolved in-memory only,
  or rewritten to disk, and what triggers the write? "read-time alias" and
  "migration … editable value" pull in different directions (Requirements bullet 6,
  AC #8/#9).
- 🔵 **Dependency** (minor, medium): The 0228→0230 Blocks edge is one-sided — 0230's
  frontmatter carries no `blocked_by: ["work-item:0228"]`, unlike 0229.
- 🔵 **Dependency** (minor, medium): The 0227 coupling is directional (0227's
  work.key-validation portion depends on 0228) but recorded only as relates_to;
  sibling 0229 records the parallel case as a scoped Blocks.
- 🔵 **Testability** (minor, medium): The tracker-less minting criterion omits the
  `id_pattern` references `{key}` precondition its siblings state.
- 🔵 **Testability** (minor, medium): Discovery-scope outcomes ("discovery scopes to
  the entity resolved from the scope key") aren't pinned to an observable emitted
  filter, unlike 0220's criteria.
- 🔵 **Testability** (minor, medium): The tracker-backed determination signal
  (`work.integration` vs section presence, per Assumptions) is exercised by no
  criterion where the two signals disagree.
- 🔵 **Dependency / Scope** (suggestion, low): the 1.25.0 alias-removal follow-on has
  no owning work item; and `init` is the most separable sub-deliverable but its
  coupling is genuine (no split recommended).

### Assessment

Ready to implement. The pass-1 REVISE findings are all closed and the acceptance
criteria are now concrete and well-exampled. The new clarity major — whether reading
a legacy config mutates it on disk or only resolves in memory, and what triggers any
write — is the one item worth resolving before planning, since it changes the
implementation shape. The dependency reciprocity items (0230 `blocked_by`, 0227 as a
scoped Blocks) are quick frontmatter/notes fixes; the testability refinements sharpen
already-passing criteria.

## Verdict Override — 2026-09-10

**Verdict:** APPROVE (was COMMENT)

Approved by Toby Clemson. The pass-2 dependency reciprocity items were closed after
re-review — 0230 now carries `blocked_by: work-item:0228`, and 0228 records the 0227
coupling as a scoped Blocks — and the tracker-less minting criterion gained its
`id_pattern` precondition. The remaining open items (the alias-vs-migration
persistence clarification and two testability refinements) are accepted as
plan-time concerns rather than blockers; the work item is ready for implementation.
