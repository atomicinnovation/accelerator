---
type: work-item-review
id: "0096-templates-view-auto-discovers-templates-review-1"
title: "Work Item Review: Templates View Auto-Discovers Available Templates"
date: "2026-06-11T13:04:17+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0096"
work_item_id: "0096"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [visualiser, templates, frontend]
last_updated: "2026-06-11T13:23:54+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Templates View Auto-Discovers Available Templates

**Verdict:** COMMENT

This is a high-quality, well-scoped story. All five lenses independently rated
it strongly: the concern is single and coherent, terms are defined inline,
the Out-of-Scope discipline is exemplary, and the acceptance criteria are
mostly framed as observable Given/When/Then behaviours. No critical or major
findings surfaced — the item is acceptable as-is. The observations below are
all minor or suggestions, clustering around two areas worth a light pass before
implementation: the glyph/0037 follow-on is described inconsistently and is
missing from the dependency surfaces, and a few acceptance criteria could pin
down their verification preconditions and the general N-templates invariant.

### Cross-Cutting Themes

- **Glyph follow-on (0037) under-represented and internally inconsistent**
  (flagged by: clarity, dependency) — The `rca` glyph fallback / 0037 follow-on
  is described three ways: Out of Scope and the Technical Notes "Glyph caveat"
  imply all five surfaced templates fall back to a blank glyph, while a later
  Technical Notes bullet corrects this to just one (`rca`). Separately, 0037 is
  named throughout the body but is absent from both the Dependencies section and
  the `relates_to` frontmatter. The same follow-on is therefore both
  mis-counted and under-linked.

- **Acceptance-criteria verification preconditions** (flagged by: testability)
  — Several criteria state the observable outcome but not the input fixture
  needed to confirm correctness (which tiers actually contain a template; the
  count invariant for an arbitrary N). The current thirteen-file snapshot is
  checkable, but the general auto-discovery invariant is not pinned to a
  dataset.

### Findings

#### Critical

_None._

#### Major

_None._

#### Minor

- 🔵 **Clarity**: Glyph-fallback claim contradicts itself across sections
  **Location**: Out of Scope vs Technical Notes
  The Out of Scope bullet and the Technical Notes "Glyph caveat" imply the
  blank-glyph fallback applies to the surfaced templates generally, but the
  final Technical Notes bullet corrects this to only `rca`. A reader gets a
  contradictory picture of how many templates lack a glyph.

- 🔵 **Dependency**: 0037 glyph follow-on named in body but absent from Dependencies
  **Location**: Dependencies
  The Technical Notes and Out of Scope name a downstream glyph follow-on coupled
  to 0037, and References lists 0037, but the Dependencies section never
  mentions it — so the 0096→0037 follow-on is invisible to anyone reading only
  Dependencies.

- 🔵 **Completeness**: Single open question left unresolved at draft stage
  **Location**: Open Questions
  The sole Open Question (whether a name/frontmatter filter is needed, or a bare
  `*.md` glob suffices) is posed but only conditionally self-resolved. An
  implementer must still seek confirmation before choosing the glob versus a
  filtered scan.

- 🔵 **Testability**: Tier-presence criterion omits the precondition needed to assert "correctly"
  **Location**: Acceptance Criteria
  The tier-presence criterion specifies the action and observable (the
  indicators) but not which tiers actually contain the template under test.
  Without that fixture, "correctly" can't be confirmed — a tier-mapping bug
  (e.g. always showing plugin-default) would pass.

- 🔵 **Testability**: General "N templates" case lacks a defined verification dataset
  **Location**: Acceptance Criteria
  The first criterion is anchored to today's thirteen-file snapshot, but the
  general "all N" claim has no procedure confirming the mapping is one-to-one
  (no dedup, ordering, or off-by-one loss). A solution that hard-codes thirteen
  names could pass the snapshot.

- 🔵 **Testability**: "No hardcoded roster remains" verifies an implementation property, not a behaviour
  **Location**: Acceptance Criteria
  The criterion is checkable but is an implementation-structure assertion rather
  than an observable behaviour, and is somewhat subjective (what counts as a
  "roster" vs a residual constant). Its value is mainly as a regression guard
  against reintroduction.

- 🔵 **Testability**: Excluded scenarios stated as scope but not as verifiable negative checks
  **Location**: Out of Scope
  Two deliberate exclusions (project-override-only templates must not appear;
  runtime additions must not surface until regeneration) are observable
  should-not-happen behaviours that currently live only in Out of Scope rather
  than as negative acceptance criteria.

#### Suggestions

- 🔵 **Scope**: S-sized single-script change may fit a task/chore better than a story
  **Location**: Frontmatter: kind
  The item is `kind: story` but sized S and described as restructuring one
  launcher script plus two test assertions, with no production code change.
  That reads closer to a task — though the maintainer-facing "templates stay in
  sync" framing is a defensible story. Judgement call, no change required.

- 🔵 **Dependency**: Same-view coordination with 0089 not characterised as ordering/coordination
  **Location**: Dependencies
  0089 is listed as "same view surface" but the section doesn't state whether
  0096 and 0089 can proceed independently or need sequencing. A one-line
  independence note would make the shared-surface coordination explicit.

- 🔵 **Dependency**: `relates_to` frontmatter omits 0037 named throughout the body
  **Location**: Frontmatter: relates_to
  `relates_to` lists 0042, 0089, 0029 but not 0037, which the body repeatedly
  names as the home of the glyph follow-on. Relation-graph tooling won't surface
  the 0096↔0037 link.

- 🔵 **Clarity**: "glyph stem table" / "stem" used without inline definition
  **Location**: Out of Scope: glyph stem table
  The term "stem" appears in Out of Scope before its meaning (the template's
  base filename used as a glyph lookup key) is inferable from later
  `STEM_TO_GLYPH` examples. A half-sentence gloss at first use would make it
  self-defining.

- 🔵 **Clarity**: "the templates view" introduced before its route is named
  **Location**: Summary
  The Summary refers to "the templates view" three times before Context pins it
  to the `/library/templates` route. Naming the route on first mention would fix
  the surface's identity immediately. (Low confidence — referent is recoverable
  in context.)

- 🔵 **Completeness**: Implementation detail density may obscure requirement-level intent
  **Location**: Technical Notes
  Technical Notes is longer than Requirements and Acceptance Criteria combined,
  embedding specific line numbers that can drift. Acceptable as-is; if trimming
  later, favour durable guidance (helpers, the jq caveat) over volatile
  line-number citations.

### Strengths

- ✅ Single, coherent, atomic unit of work: every requirement serves the one
  goal of deriving the template list from the `templates/` directory — no "and
  also" bundling. Summary, Requirements, and Acceptance Criteria are mutually
  consistent.
- ✅ Exemplary Out-of-Scope discipline: the config-CLI drift (0029), the glyph
  stem-table extension, project-override-only templates, and runtime hot-reload
  are each explicitly carved out, preserving one concern.
- ✅ Terms that could trip a reader are defined at point of use — "build time"
  is mapped to the launcher config-generation step in both Requirements and a
  dedicated Assumptions section; "tier" is consistently expanded to the same
  three values everywhere.
- ✅ Acceptance Criteria are framed as Given/When/Then behaviours with
  observable outcomes, and the first criterion anchors the abstract "N
  templates" claim to a concrete checkable instance (all thirteen current
  templates, naming the five absent today).
- ✅ Dependency status is explicitly and correctly resolved: not blocked (0042
  complete), and the prerequisite scan helper (`config_enumerate_templates`) is
  confirmed to already exist and already be sourced.
- ✅ Frontmatter is intact and consistent (recognised `kind`, appropriate
  `status`, populated relationships), and optional sections carry genuinely
  relevant content rather than empty placeholders.

### Recommended Changes

1. **Reconcile the glyph-fallback count to `rca` only** (addresses: Glyph-fallback
   claim contradicts itself across sections)
   Update the Out of Scope bullet and the Technical Notes "Glyph caveat" so they
   state a single, consistent count — only `rca` renders the blank-glyph
   fallback; `plan-review`, `pr-review`, `work-item-review`, and `note` already
   resolve.

2. **Surface 0037 in the dependency surfaces** (addresses: 0037 glyph follow-on
   absent from Dependencies; `relates_to` omits 0037)
   Add a line to the Dependencies section noting 0037 as the follow-on home for
   the `rca` glyph stem entry, and add `work-item:0037` to the `relates_to`
   frontmatter so prose and metadata agree.

3. **Resolve the Open Question inline before leaving draft** (addresses: Single
   open question left unresolved)
   Record the confirmation that `templates/` holds only user-facing templates
   (so a bare `*.md` glob is sufficient), or state the chosen filtering
   convention, so the implementer can proceed without a follow-up.

4. **Tighten the verification-sensitive acceptance criteria** (addresses:
   Tier-presence precondition; General N-templates dataset; Excluded scenarios
   as negative checks)
   - Add a concrete fixture to the tier-presence criterion (e.g. a template in
     plugin-default only vs one in plugin-default + user-override, asserting
     exactly those tiers lit).
   - Add a count-invariant criterion exercising K test files → K rows for K in
     {0, 1, many}.
   - Promote the project-override exclusion to a negative acceptance criterion
     (template present only in `.accelerator/templates/` does not appear).

5. **Optional polish** (addresses: kind label; 0089 coordination; "stem"
   definition; route naming; Technical Notes density)
   Consider whether `task` fits better than `story`; add a one-line 0089
   independence note; gloss "stem" at first use; name `/library/templates` in
   the Summary. None are blocking.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Exceptionally clear and self-consistent. Every potentially
ambiguous term (template, build time, roster, tiers) is defined inline at first
use; Summary/Context/Requirements/Acceptance Criteria describe the same narrow
concern; actors and outcomes are concrete. The only wrinkles are a minor
numeric-claim inconsistency about the glyph fallback and one unexplained domain
term ("glyph stem table").

**Strengths**:
- Domain terms that could trip a new reader are defined at point of use ("build
  time" mapped to the launcher config-generation step in both Requirements and
  Assumptions; "tier" consistently expanded to plugin-default / user-override /
  config-override everywhere).
- Scope is internally consistent across sections — the Summary's narrow framing
  is reinforced identically in Context, Requirements, Out of Scope, and Drafting
  Notes, with no contradiction between stated problem and proposed solution.
- Actors and triggers are named concretely: requirements attribute the discovery
  action to `write-visualiser-config.sh` rather than leaving it passive, and
  outcomes are stated as observable view states.

**Findings**:
- 🔵 minor / high confidence — **Glyph-fallback claim contradicts itself across
  sections** (Out of Scope vs Technical Notes): Out of Scope and the "Glyph
  caveat" imply the fallback applies to surfaced templates generally; the final
  Technical Notes bullet corrects it to only `rca`. A reader encountering the
  earlier statements first gets a contradictory picture. Suggestion: reconcile
  the earlier wording with the corrected finding so the document states one
  count (`rca` only) consistently.
- 🔵 suggestion / medium — **"glyph stem table" / "stem" used without inline
  definition** (Out of Scope): the meaning of "stem" is only inferable from the
  later `STEM_TO_GLYPH` examples. Add a half-sentence gloss at first use.
- 🔵 suggestion / low — **"the templates view" introduced before its route is
  named** (Summary): the referent is pinned to `/library/templates` only one
  section later. Optionally name the route on first mention.

### Completeness

**Summary**: Exceptionally complete story. All structurally required sections
(Summary, Context, Requirements, Acceptance Criteria) are present and
substantively populated, and the story-specific elements (the user whose need is
met, the motivating problem, verifiable done-criteria) are all explicit.
Frontmatter is intact with a recognised kind and appropriate status. Only minor
observations: one unresolved open question and unusually heavy implementation
detail.

**Strengths**:
- Summary states the work as a clear user-story noun phrase and identifies the
  beneficiary ("a maintainer of the Accelerator template set").
- Context thoroughly explains why the work is needed (the eight-vs-thirteen
  drift, naming the five never-surfaced templates) rather than restating the
  summary.
- Acceptance Criteria contains five specific Given/When/Then bullets covering the
  positive case, addition, removal, tier indicators, and roster removal — beyond
  the two-criterion minimum.
- Requirements describe the actual work and are distinct from acceptance
  criteria; frontmatter is intact and consistent; optional sections carry
  relevant content rather than placeholders.

**Findings**:
- 🔵 minor / medium — **Single open question left unresolved at draft stage**
  (Open Questions): the sole question self-resolves only conditionally; an
  implementer must seek confirmation before choosing the glob vs a filtered
  scan. Suggestion: record the confirmation inline before promoting beyond draft.
- 🔵 suggestion / low — **Implementation detail density may obscure
  requirement-level intent** (Technical Notes): the section is longer than
  Requirements and Acceptance Criteria combined and cites volatile line numbers.
  Acceptable as-is; favour durable guidance if trimming later.

### Dependency

**Summary**: Well-mapped, low-coupling story: explicitly not blocked (0042
complete), names its related items, and confirms the required scan helper
already exists and is already sourced. The only gaps are minor: a glyph follow-on
relating to 0037 is in the body but absent from Dependencies and `relates_to`,
and the same-view relationship with 0089 is named without noting any
sequencing/coordination implication. No upstream blockers, external systems, or
cross-team actions left uncaptured.

**Strengths**:
- Upstream blocking status is explicitly and correctly resolved ("0042 is
  complete, so there is no blocking dependency"), and the prerequisite
  `config_enumerate_templates` helper is confirmed to already exist and be
  sourced.
- Out of Scope actively prevents dependency confusion by separating the config
  CLI's hardcoded list (0029) as a distinct surface.
- The glyph caveat coupling to 0037 is correctly characterised as a non-blocking
  follow-on, and acceptance criteria are satisfiable even with the blank-glyph
  fallback.

**Findings**:
- 🔵 minor / high — **0037 glyph follow-on named in body but absent from
  Dependencies** (Dependencies): References lists 0037 but Dependencies never
  mentions it, so the 0096→0037 follow-on is invisible to anyone reading only
  Dependencies. Suggestion: add a Dependencies line noting 0037 as the follow-on
  for the `rca` glyph stem entry.
- 🔵 suggestion / medium — **Same-view coordination with 0089 not characterised
  as ordering/coordination** (Dependencies): 0089 is listed as "same view
  surface" but independence/sequencing is not stated. Suggestion: add a one-line
  independence (or ordering) note.
- 🔵 suggestion / low — **`relates_to` frontmatter omits 0037 named throughout
  the body** (Frontmatter: relates_to): relation-graph tooling won't surface the
  0096↔0037 link. Suggestion: add `work-item:0037` to `relates_to`.

### Scope

**Summary**: One coherent, atomic unit of work: replacing the hardcoded template
roster in `write-visualiser-config.sh` with directory-scan discovery. Summary,
Requirements, and Acceptance Criteria describe the same single concern, and the
Out of Scope section shows unusually disciplined boundary-setting. The only
scope-adjacent observation is that the declared "story" kind may be larger than
this S-sized single-script change warrants — a minor labelling judgement, not a
delivery risk.

**Strengths**:
- Single unified purpose: every requirement serves the one goal — no "and also"
  bundling.
- Summary, Requirements, and Acceptance Criteria are mutually consistent.
- Exemplary Out of Scope discipline: config-CLI drift (0029), glyph stem-table
  extension, project-override-only templates, and runtime hot-reload each
  explicitly excluded.
- Clear in/out boundary: the reviewer can state precisely what is in scope
  (discovery in one launcher script plus its two pinned tests) and what is not.

**Findings**:
- 🔵 suggestion / medium — **S-sized single-script change may fit a task/chore
  better than a story** (Frontmatter: kind): no production Rust/frontend change,
  no new user-visible metadata; reads closer to a task. The maintainer-facing
  "templates stay in sync" outcome is a legitimate story framing, so retaining
  `story` is defensible — no change required.

### Testability

**Summary**: Broadly testable: Acceptance Criteria are framed as Given/When/Then
behaviours with concrete observable outcomes (specific template names appearing,
specific files added/removed reflected in the view). The strongest criteria name
exact templates and tier-presence expectations. The main gaps are an unbounded
N-templates criterion lacking a defined dataset for the general case, a
tier-presence criterion missing its precondition, and a structural criterion
("no hardcoded roster remains") that verifies an implementation property rather
than an observable behaviour.

**Strengths**:
- Acceptance Criteria consistently expressed as Given/When/Then behaviours with
  observable outcomes, well-suited to the Story kind.
- The first criterion anchors the abstract "N templates" claim to a concrete
  instance — all thirteen current templates, naming the five absent today.
- The add/remove criteria specify both trigger and observable result, covering
  the round-trip behaviour.
- Out of Scope and Assumptions explicitly bound verification (build-time not
  runtime, plugin-default directory only), so a tester knows which scenarios
  should not change the view.

**Findings**:
- 🔵 minor / medium — **Tier-presence criterion omits the precondition needed to
  assert "correctly"** (Acceptance Criteria): without an input fixture for which
  tiers contain the template, a tier-mapping bug would pass. Suggestion: add a
  concrete fixture (plugin-default only vs plugin-default + user-override).
- 🔵 minor / medium — **General "N templates" case lacks a defined verification
  dataset beyond today's snapshot** (Acceptance Criteria): the thirteen-file
  instance is one data point, not the "all N" invariant. Suggestion: add a
  criterion exercising K test files → K rows for K in {0, 1, many}.
- 🔵 minor / high — **"No hardcoded roster remains" verifies an implementation
  property, not an observable outcome** (Acceptance Criteria): checkable but
  subjective and structural; mainly a regression guard. Suggestion: reframe
  behaviourally or make the check unambiguous, or state it as a code-review
  checklist item.
- 🔵 minor / low — **Excluded scenarios stated as scope but not as verifiable
  negative checks** (Out of Scope): the project-override exclusion and the
  no-runtime-reload behaviour are observable should-not-happen behaviours.
  Suggestion: promote the project-override exclusion to a negative Acceptance
  Criterion.

## Re-Review (Pass 2) — 2026-06-11

**Verdict:** COMMENT

Re-ran all five lenses against the edited work item. Every actionable Pass-1
finding is resolved: the dependency lens now returns **zero findings**, and the
clarity glyph-contradiction, the unresolved Open Question, and the route-naming
nit are all gone. The item remains a strong COMMENT — no critical or major
findings. The residual items are refinements of the new acceptance criteria
plus one tier-naming consistency point surfaced (not introduced) by the edits.

### Previously Identified Issues

- 🔵 **Clarity**: Glyph-fallback claim contradicts itself — **Resolved** (the
  two Technical Notes bullets merged; Out of Scope narrowed to `rca` only).
- 🔵 **Clarity**: "stem" used without inline definition — **Resolved** (gloss
  added at first use in Out of Scope; a low-confidence note suggests it could
  move even earlier, but the inline gloss now resolves the term).
- 🔵 **Clarity**: "the templates view" named before its route — **Resolved**
  (`/library/templates` now named on first mention in the Summary).
- 🔵 **Dependency**: 0037 follow-on absent from Dependencies — **Resolved**
  (Follow-on line added; dependency lens returned zero findings this pass).
- 🔵 **Dependency**: 0089 coordination not characterised — **Resolved**
  (Coordination line states the two are independent, no ordering).
- 🔵 **Dependency**: `relates_to` omits 0037 — **Resolved** (`work-item:0037`
  added to frontmatter).
- 🔵 **Completeness**: Open Question unresolved — **Resolved** (marked
  _(Resolved)_; bare `*.md` glob confirmed, no filter needed).
- 🔵 **Completeness**: Technical Notes density — **Not actioned** (acceptable by
  decision; not re-flagged this pass).
- 🔵 **Testability**: Tier-presence precondition missing — **Partially resolved**
  (a plugin-default-only vs plugin-default+user-override fixture criterion was
  added; the *user-override* setup location is still not named, so that tier is
  not yet constructable — see New Issues).
- 🔵 **Testability**: General N-templates dataset — **Partially resolved** (a
  K-in-{0,1,many} count-invariant criterion was added; "many" still lacks a
  concrete count — see New Issues).
- 🔵 **Testability**: Excluded scenarios as negative checks — **Resolved** (the
  project-override exclusion is now a negative Acceptance Criterion and was
  cited as a strength this pass).
- 🔵 **Testability**: "No hardcoded roster remains" structural — **Still present
  / by decision** (kept as a regression guard; re-flagged minor, behavioural
  add/remove criteria already cover the dynamism).
- 🔵 **Scope**: kind story-vs-task — **By decision** (kept `story`; re-flagged as
  a suggestion that explicitly says "No action required" given the user-visible
  value).

### New Issues Introduced

- 🔵 **Clarity** (suggestion): Tier naming is inconsistent — Requirements and the
  tier-presence criterion call the third tier `config-override`, while Out of
  Scope and the new project-override criterion call `.accelerator/templates/`
  the "project override directory". Surfaced (not strictly introduced) by the
  added criterion; worth standardising on one name or stating they're the same.
- 🔵 **Clarity** (suggestion): The three tier names have no single anchoring
  definition mapping each to its directory — a newcomer assembles the model from
  scattered mentions.
- 🔵 **Testability** (minor): The new count-invariant criterion's "many" has no
  concrete count to assert against — pin it (e.g. `{0, 1, 3, 13}`) or map "many"
  to the N=13 on-disk case.
- 🔵 **Testability** (minor): The new tier-presence criterion names a
  user-override precondition without stating where the user-override directory
  lives, so a verifier cannot construct that tier deterministically.
- 🔵 **Completeness** (suggestion): Requirements now overlap with the expanded
  Acceptance Criteria; could be tightened to add implementer detail beyond the
  acceptance bullets.

### Assessment

The work item is ready for implementation. All Pass-1 issues are resolved or
consciously retained by decision, and the verdict holds at COMMENT with no
blocking findings. The new items are minor polish on the freshly-added criteria
and a pre-existing tier-naming inconsistency the edits made more visible — none
of them block planning. The single highest-value optional follow-up is
standardising the `config-override` / "project override directory" naming, since
it touches both Requirements and Acceptance Criteria.

---
*Re-review generated by /accelerator:review-work-item*

## Final Decision — 2026-06-11

**Verdict:** APPROVE (reviewer override)

After Pass 2, the reviewer applied the two highest-value residual refinements —
standardising the `config-override` / "project override directory" naming across
Requirements, Out of Scope, and the Acceptance Criteria, and replacing the
open-ended "many" in the count-invariant criterion with concrete counts
(`{0, 1, 3, 13}`). With those closed, the verdict is upgraded from COMMENT to
**APPROVE**: the work item is ready for implementation, with the remaining
suggestion-level items (three-tier anchoring gloss, user-override construction
path, Requirements/AC overlap, the `story`-vs-`task` label) consciously left as
non-blocking polish.

---
*Decision recorded by /accelerator:review-work-item*
