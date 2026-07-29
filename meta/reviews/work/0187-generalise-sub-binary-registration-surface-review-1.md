---
type: work-item-review
id: "0187-generalise-sub-binary-registration-surface-review-1"
title: "Work Item Review: Generalise the Sub-Binary Registration Surface"
date: "2026-08-01T13:00:40+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0136"
target: "work-item:0187"
work_item_id: "0187"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 5
tags: [build-system, distribution, rust]
last_updated: "2026-08-01T17:40:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Generalise the Sub-Binary Registration Surface

**Verdict:** REVISE

0187 is a well-formed platform extraction with an unusually strong evidence
base: nearly every claim carries a `file:line` anchor, the fixture-token
strategy is a genuine decoupling device that lets the task land ahead of all
four consumers, and AC1's demand that the guard be demonstrably non-vacuous is
exactly the right mutation-style framing. All five lenses agreed the item is
scoped as a single coherent concern in a single component with no double
ownership against 0169. The revisions needed are concentrated in three places:
a headline contradiction between the Summary's "one-line allowlist change" and
the multi-step checklist the Requirements mandate; a registration checklist
that is narrower than the research §8 enumeration it cites; and three
mechanisms the item depends on — the fixture token, the skill-exemption
declaration, and the token→skill discovery rule — that are named but never
defined.

### Cross-Cutting Themes

- **Summary promises an outcome the body does not deliver** (flagged by:
  clarity, scope, testability, completeness) — "one-line allowlist change"
  versus a documented six-point checklist. All four lenses independently
  reached this, from different angles: ambiguity, scope-sizing, unmeasured
  success condition, and outcome mismatch. This is the single highest-signal
  finding in the review.
- **The checklist is narrower than its own cited source** (flagged by:
  completeness, dependency, scope, testability) — research §8 enumerates eight
  registration points; the Requirements cover six, silently dropping
  cross-compile staging (`tasks/build.py:37`, `:290-331`, incl.
  `_assert_static_elf`) and `version.workspace = true`. Because the checklist
  *is* the deliverable, a short checklist reproduces the rediscovery cost the
  extraction exists to remove.
- **Three load-bearing mechanisms are named but undefined** (flagged by:
  clarity, completeness, testability, dependency) — "fixture token",
  "declared skill-exempt", and the token→skill discovery rule. Each admits two
  materially different implementations, and each is depended on by an
  acceptance criterion.
- **The already-parameterised assumption is never discharged** (flagged by:
  completeness, dependency, testability, scope) — the Assumptions section says
  signing/upload/re-verify/SLSA are worth re-confirming and that any
  visualiser-shaped stage belongs in *this* task, but no criterion covers the
  re-confirmation and no ceiling bounds the absorption.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity / Scope / Testability / Completeness**: Summary's "one-line
  allowlist change" contradicts the multi-point registration checklist the
  Requirements mandate
  **Location**: Summary (vs. Requirements 2-3, Acceptance Criteria)
  The Summary states the goal is making registration "a one-line allowlist
  change", while Requirement 2 mandates documenting at least six distinct
  registration points and Requirement 3 adds a naming constraint on top. These
  bound the task very differently — an implementer taking the Summary
  literally could attempt to derive the other registration points from the
  allowlist, a build-system refactor several times the described size. The
  headline outcome also has no acceptance criterion measuring it.

- 🟡 **Completeness / Dependency / Testability / Scope**: The registration
  checklist enumerates fewer points than the research it cites, and AC4 cannot
  fail
  **Location**: Requirements (2nd bullet) / Acceptance Criteria (4th)
  Research §8 lists eight registration points; the Requirements cover six,
  omitting cross-compile staging (`tasks/build.py:37`, `:290-331`, with the
  musl `_assert_static_elf` check) and `version.workspace = true`
  (enforced by `tasks/build.py:74-101`). Neither is mentioned in the
  Assumptions' already-parameterised list either, so ownership of point 8 is
  unassigned between this task and its first consumer. AC4 asks only that the
  README "carries the registration checklist" with no enumeration, so a
  three-of-eight checklist would satisfy it.

- 🟡 **Clarity / Completeness / Testability**: "Declared skill-exempt" has no
  stated declaration mechanism, and nothing verifies the exempt path
  **Location**: Requirements (1st bullet) / Acceptance Criteria (3rd)
  Requirement 1 and AC3 both depend on an exemption declaration, but no
  section says where or how it is made — a sibling constant beside
  `DISPATCHED_SUBBINARIES`, a richer per-token structure, a config entry. AC3
  also requires no test of the exempt-and-passing path. The exemption is the
  escape hatch the whole guard depends on and the surface most likely to be
  abused later, yet its shape is invented at implementation time and
  propagates to all four consumers.

- 🟡 **Clarity / Testability**: "Fixture token" carries the whole verification
  strategy but is never defined, and neither the discovery rule nor the
  injection seam is stated
  **Location**: Requirements (4th bullet) / Acceptance Criteria (1st) /
  Technical Notes
  Two readings are equally available: a test-only value patched into
  `DISPATCHED_SUBBINARIES`, or a permanently-registered token with no real
  binary — and the latter would flow into signing, manifest generation and
  upload, silently invalidating the Assumptions. Separately, the generalised
  guard could resolve token→skill by scanning the skills tree or by consulting
  an explicit map; the two give different answers to "the binding is missing"
  for the same repo state. `DISPATCHED_SUBBINARIES` is a module constant, so
  no injection seam means monkeypatching may leave the test off the path the
  release actually runs.

- 🟡 **Dependency**: The registration checklist omits the skill-binding /
  exemption step the newly-strict guard will enforce on consumers
  **Location**: Requirements (2nd bullet) / Acceptance Criteria (4th)
  Requirement 1 makes `validate_dispatch_coherence` fail for any non-exempt
  token lacking a skill binding, and it runs on every release
  (`tasks/manifest.py:138`) — but the checklist enumerates only the six
  mechanical registration points and never says "add the skill binding, or
  declare the token exempt". A downstream story can complete every documented
  step and still fail its release on the one new constraint this task
  introduces.

- 🟡 **Dependency**: Three of the four declared downstream consumers do not
  record the reciprocal dependency, and nothing requires them to
  **Location**: Dependencies (Blocks) / Frontmatter: blocks
  Only 0169 carries `blocked_by: work-item:0187`. 0170 and 0171 have no
  `blocked_by` at all; 0173 lists only 0167. All three are `status: draft`.
  0187 has no requirement or acceptance criterion obliging the hand-off,
  unlike 0169 which has an explicit "the downstream hand-offs are raised"
  criterion. The coupling is invisible from the consuming side — reproducing
  exactly the rediscovery cost this task was extracted to eliminate.

- 🟡 **Testability**: The `allowed-tools`-mismatch failure mode is required
  but never tested
  **Location**: Acceptance Criteria (1st-2nd)
  AC1 requires the guard to fail on two conditions — a missing skill binding
  *and* an `allowed-tools` rule that does not name the subcommand — but the
  verification it specifies covers only the first, and AC2 asks for a singular
  fail path. The second is the half most likely to ship as a no-op (e.g. a
  substring match that an ancestor glob satisfies), and it is precisely the
  defect `tasks/lint/skill_permissions.py:41-44` exists to catch.

#### Minor

- 🔵 **Testability**: AC1's blanket "any token" predicate contradicts AC3's
  exemption
  **Location**: Acceptance Criteria (1st and 3rd)
  Read literally, AC1 requires a failure for a token AC3 requires to pass. The
  criteria do not define a single determinate predicate for exempt tokens.

- 🔵 **Completeness / Dependency / Testability**: The assumption
  re-confirmation is in scope but has no acceptance criterion
  **Location**: Assumptions / Acceptance Criteria
  Assumptions pulls the re-confirmation of signing/upload/re-verify/SLSA into
  scope with a conditional scope expansion, but no criterion covers it, so the
  item can close with the check never performed. The fixture-token strategy
  also cannot exercise the real signing/upload path, and no non-release
  verification route is named.

- 🔵 **Scope**: The scope-absorption clause in Assumptions is unbounded
  **Location**: Assumptions
  "If any turns out visualiser-shaped, it belongs in this task" is the right
  anti-leakage rule but has no ceiling. A `kind: task` sized at one function
  plus tests plus a doc section could quietly become release-pipeline work —
  and because 0187 blocks four stories, the absorption extends the critical
  path for all of them.

- 🔵 **Testability**: Nothing verifies the generalised guard still runs in its
  real release invocation path
  **Location**: Acceptance Criteria (1st-2nd, 5th)
  All specified verification is unit-level against
  `tests/unit/tasks/test_build.py`, and `mise run` does not exercise a
  release. A signature change (likely, given the fixture-token strategy) could
  leave `tasks/manifest.py:138` passing a stale or empty collection with every
  unit test green — the guard becoming vacuous in production, the exact
  failure AC1 works to prevent at unit level.

- 🔵 **Testability**: "Still passes unchanged" does not define what is
  compared
  **Location**: Acceptance Criteria (2nd)
  An implementation that quietly relaxes the visualiser's `allowed-tools`
  expectation to make the generalised check pass would still be defensible
  under this wording, hiding a regression in guard strictness.

- 🔵 **Dependency**: 0172 may be a fifth consumer but is absent from Blocks
  **Location**: Dependencies (Blocks)
  0172 ("Migration Engine Subdomain") sits under the same epic-0136 "Subdomain
  migrations (Phases 5-10)" heading as the other four, and ADR-0054's "one
  binary per independently-shippable subdomain" implies it registers a token
  too. If so, the dependency graph under-reports this task's reach.

- 🔵 **Dependency**: 0168 is recorded as "code landed" but its work item is
  still `ready`, with no closure action named
  **Location**: Dependencies (Related)
  0168's landed layout is load-bearing — the checklist documents the
  `cli/<token>/Cargo.toml` default and the `_SUBBINARY_MANIFESTS` override
  precisely because of the visualiser's nested placement. Sibling 0169 handles
  the identical situation for 0167 with an explicit "close out 0167's status
  before this story starts"; 0187 offers no equivalent.

- 🔵 **Clarity**: "SKILL↔producer binding" uses "producer" in a sense that
  collides with the repo's frontmatter meaning
  **Location**: Context / Summary
  Requirement 1 later restates the same concept as a skill↔subcommand binding
  and calls the skill the *consumer*, leaving "producer" to mean the binary or
  its build task — while `producer:` in this repo's frontmatter (including
  this work item's own) means something entirely unrelated.

- 🔵 **Clarity**: "Generalise it" has two candidate referents
  **Location**: Summary
  The Summary names two distinct problems then says "Generalise it". "It" can
  refer to `validate_dispatch_coherence`, to the registration surface, or to
  "the dispatched-sub-binary machinery" — three readings of differing size for
  the headline goal.

- 🔵 **Clarity**: "Documented nowhere" conflicts with the References entry
  that enumerates the registration points
  **Location**: Summary (vs. References)
  The intended meaning is presumably that no *durable developer-facing*
  documentation exists, but as written the Summary contradicts the References
  section's "Registration points enumerated in: … §8".

- 🔵 **Clarity**: The checklist ordering is called "ordered" but no ordering
  principle is stated
  **Location**: Requirements (2nd bullet)
  It is unclear whether the sequence given in the bullet *is* the required
  order or whether the implementer must determine one.

#### Suggestions

- 🔵 **Clarity**: "review-2, pass 4" and the personified "scope" are
  unresolvable provenance references
  **Location**: Context (final paragraph)
  The coordinate has no link, and "scope" as a bare noun performing an action
  reads as an undefined actor unless the reader knows it names a review lens.

- 🔵 **Testability**: Consider a drift guard so the README checklist stays
  truthful
  **Location**: Acceptance Criteria (4th)
  A documentation-only artefact with no executable check will rot, and four
  consumer stories will trust a stale checklist. A lightweight test asserting
  the README names each registration identifier would make a rename fail a
  test rather than age the docs.

### Strengths

- ✅ Almost every technical claim is anchored to an explicit `file:line`
  reference (`tasks/build.py:35`, `tasks/shared/paths.py:25`,
  `tasks/manifest.py:51-53`), so work can begin without a discovery pass.
- ✅ AC1 explicitly demands non-vacuity — "a deliberately missing binding …
  not merely passing" — closing the most common way a guard test provides
  false assurance.
- ✅ The fixture-token requirement is a genuine decoupling *mechanism*, not an
  assumption: it substantiates the no-blockers position and lets the task land
  ahead of every consumer.
- ✅ Correctly extracted as shared platform work rather than riding inside its
  first consumer, with the anti-pattern named explicitly in Context.
- ✅ Non-couplings are stated as positively as couplings — signing
  (`tasks/signing.py:50-73`), upload and re-verification
  (`tasks/github.py:218-235`, `:270-293`) and the SLSA globs are named as
  needing no work, so the surface boundary is checkable rather than guessed.
- ✅ No double ownership with the primary consumer: 0169 states "this story
  adds the token, it does not generalise the surface", and the parent epic
  0136 records the same edge — item-level and epic-level views agree.
- ✅ Blocks entries carry rationale rather than bare ids, and Related entries
  annotate current state ("0165 — done", "0168 — code landed").
- ✅ `kind: task` is proportionate: internal build-system enablement with no
  user-visible increment, sized to one function plus tests plus a doc section.
- ✅ Requirements are phrased as imperatives with an unambiguous actor, and
  Requirement 3 gives the *reason* behind the naming constraint so it cannot
  be misread as arbitrary convention.
- ✅ SLSA — the one acronym that could trip a reader — is expanded on first
  use.
- ✅ AC2 names the exact test module (`tests/unit/tasks/test_build.py`) and
  AC5 is a project-defined binary gate with no interpretive room.

### Recommended Changes

1. **Restate the Summary to match the delivered outcome** (addresses: the
   one-line-allowlist contradiction across four lenses)
   Replace "a one-line allowlist change" with something like "a mechanical,
   documented checklist rather than a rediscovery exercise". If collapsing
   registration to a single entry is genuinely wanted, scope it as a separate
   follow-up and reference it here. Also replace the ambiguous "it" in
   "Generalise it", and qualify "documented nowhere" as "documented only in
   research, not in `tasks/README.md`".

2. **Reconcile the checklist against research §8 and make AC4 enumerable**
   (addresses: checklist narrower than cited source; AC4 cannot fail)
   Either add cross-compile staging (`tasks/build.py:37`, `:290-331`, incl.
   `_assert_static_elf`) and `version.workspace = true` to the checklist, or
   state in Assumptions why they are excluded. Then rewrite AC4 to enumerate
   the required entries by name, or to require coverage of every §8
   registration point with an explicit note for any deliberate omission.

3. **Add the skill-binding / exemption step to the checklist** (addresses: the
   dependency finding on the consumer-facing checklist)
   Make "add the skill binding, or declare the token exempt" an explicit
   numbered step naming the file that holds the exemption declaration, and
   extend AC4 to require it — otherwise a consumer completes every documented
   step and still fails at release.

4. **Define the exemption mechanism and pin both its directions with a test**
   (addresses: undefined declaration mechanism; unverified exempt path)
   Name the declaration site and shape in Requirement 1 (even loosely, e.g.
   "an explicit exempt-token set in `tasks/shared/paths.py`"), and extend AC3:
   "a fixture token listed in the exemption declaration and having no skill
   passes the guard; the same token with the exemption removed fails it."

5. **Define "fixture token", the discovery rule, and the injection seam**
   (addresses: undefined fixture token; unspecified token→skill binding;
   under-specified test procedure)
   State whether the fixture token exists only within the test or is
   registered in production code; state whether the guard discovers skills by
   scanning for the subcommand invocation or reads a declared token→skill map
   (and whether multiple consuming skills are permitted); and name the seam —
   e.g. `validate_dispatch_coherence` taking the token collection and skills
   root as parameters, so tests pass fixtures rather than patching module
   state.

6. **Split AC1 so both guard failure modes are tested, and qualify it against
   AC3** (addresses: untested `allowed-tools` mismatch; AC1/AC3 contradiction)
   Split into: (a) fixture token whose skill does not exist → guard fails;
   (b) fixture token whose skill exists but whose `allowed-tools` carries only
   an ancestor glob (e.g. `Bash(accelerator:*)`) rather than naming the
   subcommand → guard fails. Change "any token" to "any **non-exempt** token".

7. **Add an acceptance criterion for the downstream hand-off** (addresses:
   0170/0171/0173 missing reciprocal edges)
   Mirror 0169's hand-off criterion: require a dated note plus
   `blocked_by: work-item:0187` on 0170, 0171 and 0173, each pointing at the
   `tasks/README.md` checklist. Also confirm whether 0172 registers a dispatch
   token — if it does, add it to `blocks`; if not, say so in a clause.

8. **Discharge the already-parameterised assumption with a criterion, and cap
   the absorption clause** (addresses: unverified assumption; unbounded scope)
   Add a criterion such as "with the fixture token registered, the
   signing-input list, upload asset list and re-verify globs each include the
   fixture token" — a non-release verification route. Then bound the clause:
   absorb visualiser-shaped findings whose fix is local to the token loop;
   anything requiring signing- or release-flow changes is raised as a sibling
   task and recorded in Dependencies.

9. **Tighten two wording gaps and add the release-path check** (addresses:
   "still passes unchanged"; guard vacuous in production)
   Restate AC2 as two observable facts (no edits to the visualiser skill's
   frontmatter or `_VISUALISE_SKILL_RELATIVE`'s target; the generalised guard
   reports the visualiser token as bound), and add a criterion that manifest
   generation still invokes the guard over the real `DISPATCHED_SUBBINARIES`.

10. **Minor cleanups** (addresses: producer/consumer terminology; 0168
    closure; provenance reference; checklist ordering)
    Define or drop "producer" in the Context's "SKILL↔producer binding"; add
    0169's closure instruction for 0168 or note its residual scope cannot move
    the visualiser crate; link or drop the "review-2, pass 4" coordinate and
    name the scope *lens* as the actor; state the ordering principle or drop
    "ordered".

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually well-anchored — nearly every claim
carries a `file:line` reference, SLSA is expanded on first use, and the
Requirements are written as imperatives with an unambiguous actor (the
implementer). The clarity weaknesses are concentrated in three
coined-but-undefined terms — "fixture token", "declared skill-exempt", and
"the SKILL↔producer binding" — each of which admits two materially different
implementations, and in a headline contradiction between the Summary's
"one-line allowlist change" and the six-point registration checklist the
Requirements mandate.

**Strengths**:

- Almost every technical claim is anchored to an explicit `file:line`
  reference (`tasks/build.py:35`, `tasks/shared/paths.py:25`,
  `tasks/manifest.py:51-53`), which removes any doubt about which code each
  statement refers to.
- The one acronym that could have tripped a reader (SLSA) is expanded on first
  use in Context.
- Requirements are phrased as imperatives directed at the implementer
  ("Generalise…", "Document…", "Make…", "Verify…"), so the performing actor is
  never in doubt.
- Acceptance Criterion 1 pre-empts a common ambiguity by stating explicitly
  that the guard must be demonstrably non-vacuous rather than merely passing.
- The Dependencies section states not just *that* this blocks
  0169/0170/0171/0173 but *why* (each adds a token rather than reworking the
  pipeline), so the blocking relationship has a single interpretation.
- Requirement 3 gives the reason behind the naming constraint (manifest-path
  defaulting plus cargo-pup whole-crate-name matching), so the constraint
  cannot be misread as arbitrary convention.

**Findings**:

- **major** / confidence high — *"One-line allowlist change" contradicts the
  multi-point registration checklist the Requirements mandate*
  **Location**: Summary (vs. Requirements 2-3)
  The Summary states the goal is to "[g]eneralise it so adding the second and
  subsequent sub-binaries is a one-line allowlist change", but Requirement 2
  mandates documenting a checklist of at least six distinct registration
  points (`DISPATCHED_SUBBINARIES`, `_SUBBINARY_MANIFESTS`, `cli/Cargo.toml`
  members, a mandatory `package.description`, `.gitignore`, and
  `manifest.example.json`) and Requirement 3 adds a package-naming constraint
  on top. These describe two different outcomes: collapsing registration to a
  single edit, versus documenting that registration is irreducibly multi-step.
  **Impact**: An implementer reading the Summary could reasonably attempt a
  refactor that derives the other registration points from the allowlist,
  which is a substantially larger change than the documentation-plus-guard
  work the Requirements and Acceptance Criteria actually describe.
  **Suggestion**: Restate the Summary's goal in terms consistent with the
  Requirements — e.g. that registration becomes a *mechanical, documented*
  checklist rather than a rediscovery exercise — or, if single-edit
  registration really is the intent, add it as an explicit requirement and
  reconcile it with the checklist.

- **major** / confidence high — *"Fixture token" is used three times but never
  defined*
  **Location**: Requirements (4th bullet) / Acceptance Criteria (1st) /
  Technical Notes
  The term carries the whole verification strategy — Requirement 4, Acceptance
  Criterion 1, and Technical Notes — but the work item never says what one is.
  Two readings are equally available: a test-only value patched into
  `DISPATCHED_SUBBINARIES` at test time, or a permanently-registered token in
  the production allowlist that has no real binary behind it.
  **Impact**: The two readings have very different consequences — a
  permanently-registered token would flow into signing, manifest generation,
  upload and re-verification, which the Assumptions section explicitly says
  need no change, so picking the wrong reading silently invalidates that
  assumption.
  **Suggestion**: Define "fixture token" on first use in Requirement 4,
  stating whether it exists only within the test (e.g. injected into the
  allowlist by the test) or is registered in production code, and where it
  lives.

- **major** / confidence high — *"Declared skill-exempt" has no stated
  declaration mechanism, and "undeclared" has two readings*
  **Location**: Requirements (1st bullet) / Acceptance Criteria (3rd)
  Requirement 1 says "[a] token with no skill consumer is a stated, explicit
  exemption rather than an unchecked gap" and Acceptance Criterion 3 says "[a]
  token may be declared skill-exempt explicitly; an *undeclared* token with no
  skill consumer fails the guard" — but neither says where or how the
  declaration is made (a separate exemption set alongside
  `DISPATCHED_SUBBINARIES`, a richer per-token structure replacing the current
  tuple, a comment, a config entry). Separately, "undeclared" can be read as
  "not declared exempt" or as "not present in `DISPATCHED_SUBBINARIES`", which
  are different conditions.
  **Impact**: The exemption is the escape hatch the guard depends on, so an
  unstated mechanism means the implementer invents the registration data shape
  — and the ambiguous "undeclared" leaves it unclear whether the guard is also
  expected to catch tokens missing from the allowlist entirely.
  **Suggestion**: Name the declaration site and shape in Requirement 1 (even
  loosely, e.g. "an explicit exempt-token set in `tasks/shared/paths.py`"),
  and replace "undeclared" in Acceptance Criterion 3 with the specific
  condition intended ("not declared exempt").

- **major** / confidence medium — *"The skill that invokes `accelerator
  <token>`" implies a one-to-one binding whose direction is unspecified*
  **Location**: Requirements (1st bullet)
  The definite article assumes exactly one skill per token, and the work item
  does not say how the check locates it: by scanning all skills for an
  `accelerator <token>` invocation (in which case "must exist" means "at least
  one skill was found"), or by a token→skill-path map generalising today's
  hardcoded `_VISUALISE_SKILL_RELATIVE` (`tasks/build.py:35`). Nothing states
  what happens when two or more skills invoke the same token.
  **Impact**: The scan-based and map-based readings produce different guard
  semantics and different registration burdens — a map adds a seventh entry to
  the registration checklist that Requirement 2 does not list, contradicting
  the claim that the checklist makes registration mechanical.
  **Suggestion**: State the binding direction explicitly — whether the guard
  discovers skills by scanning for the subcommand invocation or reads a
  declared token→skill mapping — and say whether multiple consuming skills are
  permitted.

- **minor** / confidence medium — *"SKILL↔producer binding" uses "producer" in
  a sense that collides with the repo's frontmatter meaning*
  **Location**: Context / Summary
  Context refers twice to "the SKILL↔producer binding" that
  `validate_dispatch_coherence` enforces, without saying what the "producer"
  is. Requirement 1 later restates the same concept concretely as "the skill
  that invokes `accelerator <token>` must exist and must carry a matching
  `allowed-tools` rule", i.e. a skill↔subcommand binding — and the same
  section calls the skill the *consumer*, leaving "producer" to mean the
  binary or its build task. The term also collides with `producer:` in this
  repo's document frontmatter (this very work item carries
  `producer: create-work-item`), where it means something entirely unrelated.
  **Impact**: A reader encountering "SKILL↔producer" before reaching the
  Requirements has to guess which of three entities "producer" names, and the
  frontmatter collision actively steers them wrong.
  **Suggestion**: On first use in Context, name the two sides concretely (e.g.
  "the binding between a consuming skill and the sub-binary subcommand it
  invokes") and drop or define "producer".

- **minor** / confidence medium — *The checklist enumeration diverges from the
  referenced research §8 table, and its "ordered" ordering is unstated*
  **Location**: Requirements (2nd bullet)
  Requirement 2 asks for "an ordered checklist" and enumerates six
  registration points, while the referenced source —
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §8 — tabulates eight, additionally covering `version.workspace = true`
  coherence (`tasks/build.py:74-101`) and cross-compile staging plus the musl
  `_assert_static_elf` check (`tasks/build.py:37`, `:290-331`). It is also
  unclear whether the sequence given in the bullet *is* the required order or
  whether the implementer must determine one.
  **Impact**: An implementer cannot tell whether the README checklist is
  complete when it reproduces the six listed items, or whether the omitted §8
  rows were deliberately excluded — undermining the stated goal that following
  the checklist makes registration mechanical.
  **Suggestion**: State whether the checklist must reproduce all of research
  §8's registration points (and say why any are excluded), and either give the
  intended ordering principle or drop "ordered".

- **minor** / confidence medium — *"Generalise it" has two candidate
  referents*
  **Location**: Summary
  The Summary names two distinct problems in one sentence — that
  `validate_dispatch_coherence` is hardcoded to the visualiser, and that the
  registration surface is documented nowhere — then says "Generalise it so
  adding the second and subsequent sub-binaries is a one-line allowlist
  change". "It" can plausibly refer to `validate_dispatch_coherence` (the
  immediately preceding subject), to the registration surface, or to "the
  dispatched-sub-binary machinery" from the opening clause.
  **Impact**: The sentence carries the work item's headline goal, so an
  unresolved referent leaves the top-line scope open to three readings of
  differing size.
  **Suggestion**: Replace "it" with the intended noun phrase.

- **minor** / confidence medium — *"Documented nowhere" conflicts with the
  References entry that enumerates the registration points*
  **Location**: Summary (vs. References)
  The Summary asserts that "the registration surface is documented nowhere",
  while the References section points at
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §8 as the place where "[r]egistration points [are] enumerated". The intended
  meaning is presumably that no *durable developer-facing* documentation
  exists (hence Requirement 2's `tasks/README.md` target), but as written the
  two statements contradict each other.
  **Impact**: A reader may conclude the research enumeration is unreliable or
  already superseded, or may under-scope the documentation work by assuming
  the research doc can simply be linked.
  **Suggestion**: Qualify the Summary claim (e.g. "documented only in
  research, not in `tasks/README.md`").

- **suggestion** / confidence high — *"review-2, pass 4" and the personified
  "scope" are unresolvable provenance references*
  **Location**: Context (final paragraph)
  Context closes with "Extracted from 0169 (review-2, pass 4), where scope
  observed that shared platform work delivered as a side effect of its first
  consumer both inflates that consumer and silently gates its siblings." The
  "review-2, pass 4" coordinate has no stated meaning or link, and "scope" as
  a bare noun performing an action reads as an undefined actor unless the
  reader already knows it names a review lens.
  **Impact**: A reader who wants to trace the rationale cannot locate the
  cited review pass, and momentarily misparses "scope observed" as a sentence
  fragment.
  **Suggestion**: Either link the review artefact or drop the coordinate, and
  name the actor explicitly (e.g. "the scope lens of review 2 observed…").

### Completeness

**Summary**: 0187 is a well-populated task: every template section except Open
Questions and Drafting Notes is present and substantively filled, and for a
`kind: task` the definition of the work is clear enough to start — file:line
anchors, an explicit non-vacuous-guard criterion, and a stated fixture-token
strategy that justifies landing ahead of its four consumers. Frontmatter is
complete and internally consistent (recognised `kind`, `status: ready`,
parent/blocks/relates_to all populated). The gaps are content-density ones
inside present sections: the documentation deliverable enumerates fewer
registration points than the research it cites, the skill-exemption mechanism
it makes testable is never defined, and one stated in-scope activity
(re-confirming the already-parameterised stages) has no corresponding
acceptance criterion.

**Strengths**:

- All core template sections are present and substantively populated —
  Summary, Context, Requirements, Acceptance Criteria, Dependencies,
  Assumptions, Technical Notes and References; nothing is placeholder-only.
- Frontmatter is complete and coherent for a ready task: `kind: task`,
  `status: ready`, `priority: high`, `parent: work-item:0136`, and a `blocks`
  list naming all four consumer stories, matching the parent epic's own
  listing of 0187 as unblocked.
- Context explains the forces behind the work (four sibling stories each
  rediscovering the same registration points) and records provenance precisely
  — extracted from 0169 review-2 pass 4 — rather than merely restating the
  Summary.
- Acceptance Criteria go beyond "it works": the first criterion explicitly
  demands the guard be demonstrably non-vacuous, and the second names the
  exact test module that must cover both pass and fail paths.
- Technical Notes give the implementer concrete entry points
  (`tasks/build.py:35`, `:189-208`, the `tasks/manifest.py:138` call site, and
  the related `tasks/lint/skill_permissions.py` rule), so work can begin
  without a discovery pass.
- The Dependencies section annotates each relationship with its current state
  ("0165 — done", "0168 — code landed") rather than listing bare IDs.

**Findings**:

- **major** / confidence high — *Registration checklist enumerates fewer
  points than the research it cites*
  **Location**: Requirements
  The work item's second requirement specifies the `tasks/README.md`
  registration checklist as covering six points (`DISPATCHED_SUBBINARIES`,
  `_SUBBINARY_MANIFESTS`, `cli/Cargo.toml` members, `package.description`,
  `.gitignore`, `manifest.example.json`), but the research it names as the
  enumeration source (`meta/research/codebase/2026-07-29-0169-…` §8) lists
  eight registration points — additionally cross-compile staging
  (`tasks/build.py:37`, `:290-331`, with `_assert_static_elf` for musl) and
  the `version.workspace = true` requirement on the new crate (enforced by
  `tasks/build.py:74-101`). Neither omitted point is mentioned anywhere in the
  work item, including in the Context paragraph that lists what is already
  parameterised and needs no work.
  **Impact**: The checklist is the primary deliverable and the artefact four
  downstream stories will follow; shipping it two points short reproduces
  exactly the rediscovery cost this task exists to remove, and the omission is
  invisible because the acceptance criterion only asks that a checklist
  exists.
  **Suggestion**: Either add cross-compile staging and the
  `version.workspace = true` constraint to the checklist requirement, or state
  explicitly in Context why they are excluded (e.g. already parameterised),
  and make the acceptance criterion require the checklist to cover every
  registration point enumerated in §8 of the referenced research.

- **minor** / confidence high — *Skill-exempt declaration mechanism is
  required and tested but never defined*
  **Location**: Requirements
  The first requirement states that "a token with no skill consumer is a
  stated, explicit exemption rather than an unchecked gap" and the third
  acceptance criterion makes that exemption testable ("a token may be declared
  skill-exempt explicitly"), but no section says how or where an exemption is
  declared — a companion constant beside `DISPATCHED_SUBBINARIES`, a per-token
  mapping, or an entry in the allowlist tuple itself. There is no Open
  Questions section in which this choice is deferred either.
  **Impact**: An implementer has to invent the exemption surface, and because
  that surface becomes part of the very registration checklist this task
  documents, an ad-hoc choice propagates to all four consumer stories.
  **Suggestion**: Name the intended shape of the exemption declaration in
  Requirements (even loosely, e.g. "a `SKILL_EXEMPT_SUBBINARIES` sibling
  constant in `tasks/shared/paths.py`"), or add an Open Questions section
  recording it as a deliberate implementation-time decision.

- **minor** / confidence medium — *Assumption re-confirmation is pulled in
  scope but has no acceptance criterion*
  **Location**: Acceptance Criteria
  The Assumptions section states that signing, upload, re-verification and
  SLSA provenance need no change but that this is "worth re-confirming at
  implementation time; if any turns out visualiser-shaped, it belongs in this
  task rather than in its first consumer". That is an in-scope activity with a
  conditional scope expansion, yet none of the five acceptance criteria
  mentions it, so the work item can be closed with the check never performed.
  **Impact**: The assumption is the one thing standing between this task and a
  consumer story rediscovering a visualiser-shaped stage — the outcome the
  task exists to prevent — and nothing in the definition of done records that
  it was validated.
  **Suggestion**: Add an acceptance criterion covering the re-confirmation of
  the already-parameterised stages (signing, upload, re-verify, SLSA globs)
  against a non-visualiser token, so the assumption is discharged before the
  task closes.

- **suggestion** / confidence medium — *Summary's "one-line allowlist change"
  outcome is not what the body delivers*
  **Location**: Summary
  The Summary states the goal as making "adding the second and subsequent
  sub-binaries a one-line allowlist change", but the Requirements and
  Acceptance Criteria deliver a multi-step checklist (six-plus registration
  points, a package-naming constraint, and a `_SUBBINARY_MANIFESTS` exception
  rule) — the work makes registration *mechanical and documented*, not one
  line.
  **Impact**: A reader taking the Summary at face value will expect a level of
  consolidation the work item does not scope, which can surface as a perceived
  shortfall at review or as a consumer story planning for a single-line
  change.
  **Suggestion**: Restate the Summary's outcome to match the body — e.g. "so
  adding a sub-binary is a documented, mechanical checklist rather than a
  rediscovery exercise" — or add a scoping sentence noting that collapsing the
  checklist to a single registration point is explicitly out of scope.

### Dependency

**Summary**: 0187 is unusually well-decoupled on the upstream side: the
fixture-token requirement is an explicit, stated mechanism for having no
blockers, and the components that need no change (signing, upload,
re-verification, SLSA globs) are enumerated with file:line so the boundary of
the work is legible. The downstream side is weaker — the work item's entire
value proposition rests on four named consumers inheriting a shared surface,
but three of the four (0170, 0171, 0173) do not record the reciprocal edge and
nothing in this work item requires them to be updated. Two inherited couplings
are also unassigned: the skill-binding/exemption step that the newly-strict
guard enforces is missing from the registration checklist consumers will
follow, and registration point 8 from the cited research (cross-compile
staging) is neither documented nor declared already-parameterised.

**Strengths**:

- The fixture-token requirement ("Verify the generalisation against a fixture
  token rather than a real new binary, so this task can land before any of its
  consumers") is an explicit decoupling mechanism, not an assumption — it
  substantiates the "no blockers" position rather than leaving it implicit.
- Blocks entries carry rationale rather than bare ids: each of
  0169/0170/0171/0173 is named with why it inherits the surface and what
  landing this first avoids ("adds a token rather than reworking the
  pipeline").
- Non-couplings are stated as positively as couplings — signing
  (`tasks/signing.py:50-73`), release upload and re-verification
  (`tasks/github.py:218-235`, `:270-293`) and the SLSA provenance globs are
  named as needing no work, so the surface boundary is checkable rather than
  guessed.
- The reciprocal edge with the primary consumer is coherent: 0169 lists 0187
  in its `blocked_by`, states "Registration follows 0187's checklist — this
  story adds the token, it does not generalise the surface", and carries an
  acceptance criterion asserting the generalised `validate_dispatch_coherence`
  covers the `vcs` token.
- The parent epic 0136 records the same edge ("0187 — Generalise the
  Sub-Binary Registration Surface (no blockers; unblocks 0169/0170/0171/0173)"),
  so the epic-level and item-level dependency views agree.

**Findings**:

- **major** / confidence high — *Three of the four declared downstream
  consumers do not record the reciprocal dependency, and nothing requires them
  to*
  **Location**: Dependencies (Blocks) / Frontmatter: blocks
  0187 declares `blocks: [0169, 0170, 0171, 0173]` and stakes its whole
  rationale on those four stories not being "serialised behind whichever gets
  there first", but only 0169 records the reciprocal edge (`blocked_by: […
  work-item:0187 …]`). 0170, 0171 and 0173 are all `status: draft` and carry
  no `blocked_by: work-item:0187` — 0170 and 0171 have no `blocked_by` at all,
  and 0173 lists only 0167 — and 0187 contains no requirement or acceptance
  criterion obliging the implementer to append the hand-off, unlike its
  sibling 0169 which has an explicit "the downstream hand-offs are raised"
  criterion naming each receiving item.
  **Impact**: The coupling is invisible from the consuming side, so
  0170/0171/0173 can be planned or started without discovering that a shared
  registration surface exists — reproducing exactly the rediscovery-and-refight
  cost this task was extracted to eliminate.
  **Suggestion**: Add an acceptance criterion (mirroring 0169's hand-off
  criterion) requiring a dated note plus `blocked_by: work-item:0187` to be
  recorded on 0170, 0171 and 0173, pointing each at the `tasks/README.md`
  checklist.

- **major** / confidence high — *The registration checklist omits the
  skill-binding/exemption step the newly-strict guard will enforce on
  consumers*
  **Location**: Requirements (registration checklist) / Acceptance Criteria
  Requirement 1 makes `validate_dispatch_coherence` fail for *any*
  `DISPATCHED_SUBBINARIES` token whose skill binding is missing or whose
  `allowed-tools` rule does not name the subcommand, unless the token is
  declared skill-exempt — and that check runs on every release
  (`tasks/manifest.py:138`). But the ordered checklist in Requirement 2
  enumerates only `DISPATCHED_SUBBINARIES`, `_SUBBINARY_MANIFESTS`,
  `cli/Cargo.toml` members, `package.description`, `.gitignore` and
  `manifest.example.json`, and the corresponding acceptance criterion asks
  only for "the package-naming constraint and the `_SUBBINARY_MANIFESTS`
  exception rule". Neither names "add the skill binding, or declare the token
  exempt" as a checklist step, and no location is given for where an exemption
  is declared.
  **Impact**: A downstream story (0169, 0170, 0171, 0173) can complete every
  documented checklist step and still fail its release at manifest generation
  on the one new constraint this task introduces — a hidden blocker discovered
  at the worst possible moment, on the release path.
  **Suggestion**: Add the skill-binding/exemption declaration as an explicit
  numbered step in the Requirement 2 checklist, naming the file that holds the
  exemption list, and extend the `tasks/README.md` acceptance criterion to
  require it.

- **major** / confidence medium — *Registration point 8 (cross-compile
  staging) from the cited research is neither documented nor declared
  already-parameterised*
  **Location**: Requirements (registration checklist) / Assumptions
  The referenced research §8 enumerates eight registration points for a new
  dispatched sub-binary. 0187's checklist covers points 1–6 and generalises
  point 7 (`validate_dispatch_coherence`), but point 8 — cross-compile staging
  at `tasks/build.py:37` / `:290-331`, including the musl `_assert_static_elf`
  check — appears nowhere in the Requirements, the Acceptance Criteria, or the
  Assumptions' list of already-parameterised machinery (which names only
  `tasks/signing.py`, `tasks/github.py` and the SLSA globs, matching the
  research's own "already parameterised, needing no edit" list, which likewise
  excludes build staging).
  **Impact**: Ownership of that registration point is unassigned between this
  task and its first consumer, so if the staging path turns out to be
  visualiser-shaped it lands inside whichever story ships the second
  sub-binary — the inflate-the-first-consumer failure mode this extraction
  exists to prevent. 0169 already carries an acceptance criterion depending on
  it ("the musl build passes `_assert_static_elf`").
  **Suggestion**: Either add `tasks/build.py` cross-compile staging to the
  documented checklist (and to the coherence/parameterisation work if it is
  hardcoded), or state explicitly in Assumptions that it is already
  token-parameterised, alongside signing, upload and SLSA.

- **minor** / confidence medium — *The "signing/upload/SLSA need no change"
  assumption has no named verification route, owner, or acceptance criterion*
  **Location**: Assumptions
  The Assumptions section states that signing, upload, re-verification and
  SLSA provenance "genuinely need no change" and that this is "worth
  re-confirming at implementation time; if any turns out visualiser-shaped, it
  belongs in this task rather than in its first consumer" — but no acceptance
  criterion covers that re-confirmation, and no route is named. The
  fixture-token strategy that keeps this task consumer-independent cannot
  exercise the real signing/upload path, and an end-to-end confirmation would
  need a release run, which per sibling 0169 requires the minisign signing key
  and a release owner ("Owner: whoever performs epic-0136 releases").
  **Impact**: The assumption's stated failure mode — work falling back into
  the first consumer — is precisely the outcome this task exists to prevent,
  yet nothing in the acceptance set would surface it before the consumer hits
  it.
  **Suggestion**: State how the assumption is confirmed without a release
  (e.g. an inspection or unit-level assertion that each of those three call
  sites iterates `DISPATCHED_SUBBINARIES`) and add it as an acceptance
  criterion, or name the release owner and the release run as the confirming
  event.

- **minor** / confidence medium — *0172 may be a fifth consumer of the
  registration surface but is absent from Blocks*
  **Location**: Dependencies (Blocks)
  The Blocks list names four subdomain stories (0169, 0170, 0171, 0173) but
  omits 0172 ("Migration Engine Subdomain"), which the parent epic 0136 groups
  under the same "Subdomain migrations (Phases 5–10)" heading as the other
  four. ADR-0054 — referenced by this work item — decides "one binary per
  independently-shippable subdomain", which implies 0172 registers a dispatch
  token too and therefore inherits this surface.
  **Impact**: If 0172 does ship a sub-binary, the dependency graph
  under-reports this task's reach, and 0172 is planned without visibility of
  the checklist and of the release-gating coherence guard.
  **Suggestion**: Confirm whether 0172 registers a dispatch token; if it does,
  add it to `blocks` and to the Dependencies prose, and if it deliberately
  does not, say so in one clause so the omission reads as a decision rather
  than an oversight.

- **minor** / confidence high — *0168 is recorded as "code landed" but its
  work item is still open, and no closure action is named*
  **Location**: Dependencies (Related)
  The Dependencies section records 0168 as "folded the visualiser into `cli/`
  — code landed" under Related, but 0168's work item is still `status: ready`,
  not `done`. Its landed layout is load-bearing for this task: the checklist
  documents the `cli/<token>/Cargo.toml` default and the `_SUBBINARY_MANIFESTS`
  override precisely because of the visualiser's nested
  `cli/visualiser/server/` placement, and the coherence check's one existing
  passing binding is the visualiser's. Sibling 0169 handles the identical
  situation for 0167 explicitly ("its work item is still `ready`, so **close
  out 0167's status before this story starts** … or the edge stays stale
  throughout"); 0187 offers no equivalent.
  **Impact**: If 0168 retains unlanded scope that moves or renames the
  visualiser crate, the checklist this task writes into `tasks/README.md` and
  the visualiser binding the generalised guard asserts against both go stale
  immediately after landing.
  **Suggestion**: Either note that 0168's residual scope cannot move the
  visualiser crate path, or add the same closure instruction 0169 uses —
  confirm and close 0168's status before this task starts.

### Scope

**Summary**: 0187 is a well-formed platform extraction: one coherent concern
(making the dispatched-sub-binary registration surface token-generic and
documented) delivered in a single component (the Python `tasks/` build system
plus `tasks/README.md`), with no cross-boundary work and no double ownership
against its four consumer stories. The `kind: task` declaration fits the scope
described — one function generalisation, its unit tests, an exemption
mechanism, and a README section — and the fixture-token requirement is a
deliberate, effective move to make the item deliverable standalone ahead of
every consumer. The main scope concern is internal inconsistency about the size
of the deliverable: the Summary and Dependencies promise that registering a
token becomes "a one-line allowlist change", while the Requirements deliver a
six-step manual checklist, and the checklist itself is narrower than the
registration-point enumeration in the research it cites.

**Strengths**:

- Single coherent purpose: every requirement (generalise the guard, document
  the surface, state the naming constraint, verify via fixture) serves one
  deliverable — making sub-binary registration token-generic rather than
  visualiser-shaped.
- Correctly extracted as shared platform work rather than riding inside its
  first consumer; the Context states the anti-pattern explicitly ("shared
  platform work delivered as a side effect of its first consumer both inflates
  that consumer and silently gates its siblings") and Dependencies names all
  four gated stories.
- Requirement 4 (verify against a fixture token rather than a real new binary)
  is a strong scope-independence device — it removes any dependency on
  0169/0170/0171/0173 and lets the task land first, which is the whole point
  of the extraction.
- Clear out-of-scope statement: signing, release upload, re-verification and
  SLSA provenance are named as already token-parameterised and needing no
  work, so the reviewer can state what is in and what is out.
- No double ownership with the consumer story: 0169's Requirements say
  "Registration follows 0187's checklist — this story adds the token, it does
  not generalise the surface", and its acceptance criterion references
  "`validate_dispatch_coherence` (generalised by 0187)". The boundary between
  enabler and consumer is agreed on both sides.
- Single-component, single-owner work (the `tasks/` Python toolchain and its
  docs) with no service-boundary or team-ownership spread.
- `kind: task` is a proportionate declaration — internal build-system
  enablement with no user-visible increment, sized to one function plus tests
  plus a doc section.

**Findings**:

- **major** / confidence medium — *Summary promises a one-line registration
  surface; Requirements deliver a six-step manual checklist*
  **Location**: Summary
  The Summary states the goal as making "adding the second and subsequent
  sub-binaries … a one-line allowlist change", and Dependencies repeats it
  ("each of those stories adds a token rather than reworking the pipeline").
  The Requirements, however, deliver a *documented* six-step manual checklist
  — `DISPATCHED_SUBBINARIES`, `_SUBBINARY_MANIFESTS`, `cli/Cargo.toml`
  members, a mandatory `package.description`, `.gitignore`, and
  `cli/launcher/tests/fixtures/manifest.example.json` — plus one guard
  generalisation. Documenting a six-point surface and collapsing it to one
  line are different units of work of very different size.
  **Impact**: The two readings bound the task differently. An implementer
  taking the Summary literally could attempt to consolidate the six
  registration points (deriving manifest paths, auto-generating ignore entries
  and the fixture manifest) — a build-system refactor several times the size
  of the described task — while an implementer taking the Requirements
  literally ships a README section. Neither can tell from the work item which
  is being asked for.
  **Suggestion**: Restate the Summary to match the Requirements, e.g. "make
  the coherence guard token-generic and document the registration surface as a
  mechanical checklist, so each consumer story adds a token instead of
  rediscovering the surface". If genuinely collapsing registration to a single
  allowlist entry is wanted, scope it explicitly as a separate follow-up item
  and reference it here.

- **minor** / confidence high — *Checklist scope omits a registration point
  enumerated in the research it cites*
  **Location**: Requirements
  0187's Requirements enumerate the registration checklist as six items,
  matching rows 1–6 of the eight-row registration table in its own cited
  source (§8). Row 7 is covered by the guard generalisation, but row 8 —
  cross-compile staging at `tasks/build.py:37` / `:290-331`, including the
  `_assert_static_elf` musl assertion — appears in neither the Requirements
  nor the Assumptions (which re-confirm only signing, upload, re-verification
  and SLSA). The same section's cargo-pup guidance (mirroring rule 6 to stop a
  new binary crate reaching into `crate::launch`) is cited as rationale for
  the naming constraint but is not itself a checklist step.
  **Impact**: The item's stated purpose is that consumers add a token rather
  than rediscover the surface; a checklist narrower than the enumerated
  surface leaves exactly the residue the extraction was meant to remove, and
  pushes it onto whichever of 0169/0170/0171/0173 ships first — 0169 already
  carries `_assert_static_elf` and cargo-pup in its own acceptance criteria.
  **Suggestion**: Either add the cross-compile staging point (and a line on
  whether a new pup rule is expected) to the Requirements' checklist, or state
  explicitly in the Requirements that those points are consumer-owned and out
  of scope for this task, so the boundary is declared rather than silently
  left open.

- **minor** / confidence medium — *Open-ended scope-absorption clause leaves
  the task's size contingent on implementation-time discovery*
  **Location**: Assumptions
  The Assumptions section states that signing, upload, re-verification and
  SLSA provenance need no change, but adds: "if any turns out
  visualiser-shaped, it belongs in this task rather than in its first
  consumer." That is a deliberate and sensible anti-leakage rule, but it is
  unbounded — the task absorbs whatever the release, signing and provenance
  pipelines turn out to need, with no stated ceiling.
  **Impact**: A `kind: task` sized at one function, its tests and a doc
  section could quietly become release-pipeline work, and because 0187 blocks
  four stories, an unbounded absorption clause extends the critical path for
  all of them rather than just this item.
  **Suggestion**: Cap the clause — e.g. "absorb visualiser-shaped assumptions
  found in these four areas if the fix is local to the token loop; anything
  requiring changes to the signing or release-upload flow is raised as a
  sibling task and recorded in Dependencies" — so the task's boundary stays
  statable regardless of what implementation finds.

### Testability

**Summary**: Work item 0187 has an unusually strong core criterion — the
demand that the coherence guard be demonstrably non-vacuous against a
deliberately broken fixture token is exactly the right mutation-style framing,
and the fixture-token strategy keeps verification independent of the four
consumer stories. Below that, verification thins out: the documentation
criterion has no enumerable content so it cannot be failed, the Summary's
headline outcome ("a one-line allowlist change") is measured by nothing and is
arguably contradicted by the six-item checklist the Requirements demand, and
one of the two failure modes the guard is required to detect (an
`allowed-tools` rule that does not name the subcommand) has no stated test.
The fixture-token procedure itself is under-specified — neither the token→skill
discovery rule nor the injection seam is defined, so two implementers could
produce tests that pass on materially different behaviour.

**Strengths**:

- AC1 explicitly demands non-vacuity — "a fixture token with a deliberately
  missing binding and asserts the guard fires … not merely passing" — which
  closes the most common way a guard test provides false assurance.
- The fixture-token approach (Requirement 4, Technical Notes) makes the task
  verifiable in isolation, ahead of all four consumer stories, rather than
  depending on unlanded work.
- AC2 names the exact test file (`tests/unit/tasks/test_build.py`) and
  requires both pass and fail paths, giving the verifier a concrete target
  rather than a vague "add tests".
- AC5 (`mise run` green end to end) is a project-defined, binary pass/fail
  gate with no interpretive room.
- Technical Notes supply precise `file:line` anchors (`tasks/build.py:35`,
  `:189-208`, `tasks/lint/skill_permissions.py:41-44`) so a verifier can
  locate every surface the criteria talk about.

**Findings**:

- **major** / confidence high — *"Carries the registration checklist" is not
  enumerable, so the doc criterion cannot fail*
  **Location**: Acceptance Criteria (4th)
  AC4 requires that `tasks/README.md` "carries the registration checklist,
  including the package-naming constraint and the `_SUBBINARY_MANIFESTS`
  exception rule", but never enumerates what the checklist must contain. The
  Requirements list six registration points, while the referenced research §8
  enumerates eight — including two the work item never mentions:
  `version.workspace = true` in the new crate's `Cargo.toml` (point 3) and
  cross-compile staging plus the musl `_assert_static_elf` check
  (`tasks/build.py:37`, `:290-331`, point 8).
  **Impact**: A README section containing three of eight registration points
  would satisfy the criterion as written, yet leave the surface the task
  exists to document partly undocumented — and the next sub-binary author
  (0169/0170/0171/0173) would rediscover the gaps, which is precisely the
  failure this task was extracted to prevent.
  **Suggestion**: Rewrite AC4 to enumerate the required entries by name, e.g.
  "the checklist covers all eight registration points from research §8
  (`DISPATCHED_SUBBINARIES`, `_SUBBINARY_MANIFESTS`, crate `Cargo.toml` with
  mandatory `package.description` and `version.workspace = true`,
  `cli/Cargo.toml` members, `.gitignore` `bin/<token>-*`,
  `manifest.example.json`, the coherence guard, cross-compile staging), or
  explicitly records why an omitted point needs no author action."

- **major** / confidence high — *The Summary's headline outcome ("one-line
  allowlist change") has no acceptance criterion*
  **Location**: Summary / Acceptance Criteria
  The Summary states the goal as "adding the second and subsequent
  sub-binaries is a one-line allowlist change", but no acceptance criterion
  measures that outcome — the five criteria cover the coherence guard, its
  tests, the exemption rule, the README, and `mise run`. The Requirements
  themselves describe a six-item checklist, so the stated success condition is
  both unmeasured and in tension with the specified work.
  **Impact**: The task can be signed off with the guard generalised and a
  README written while the pipeline is still visualiser-shaped somewhere
  untested — the very risk the Assumptions section flags — leaving the four
  blocked stories to discover it, so the criteria do not collectively cover
  the Summary's intent.
  **Suggestion**: Either restate the Summary's promise to match reality
  ("adding a token is a mechanical, documented N-step change"), or add a
  criterion that exercises it: with a fixture token added to
  `DISPATCHED_SUBBINARIES` and only the checklist steps followed, manifest
  generation, signing-input enumeration and re-verify glob construction each
  yield an entry for the fixture token.

- **major** / confidence medium — *The `allowed-tools`-mismatch failure mode
  is required but never tested*
  **Location**: Acceptance Criteria (1st-2nd)
  AC1 requires the guard to fail on two distinct conditions — a missing skill
  binding **and** an `allowed-tools` rule that "does not name the subcommand"
  — but the verification it specifies covers only the first ("a fixture token
  with a deliberately missing binding"), and AC2 asks for "both the pass and
  fail paths" (singular fail path). The second condition is the subtler of the
  two: per Technical Notes it must align with
  `tasks/lint/skill_permissions.py:41-44`, `:183-188`, where a `Bash(...)`
  permission relying on an ancestor glob rather than naming the subcommand is
  the exact defect being guarded against.
  **Impact**: The half of the guard most likely to be implemented as a no-op
  (e.g. a substring match that an ancestor glob satisfies) would ship with no
  non-vacuous test, so the criterion's "demonstrably non-vacuous" demand is
  only half honoured.
  **Suggestion**: Split AC1 into two criteria, each with its own fixture case:
  (a) fixture token whose skill does not exist → guard fails; (b) fixture
  token whose skill exists but whose `allowed-tools` carries only an ancestor
  glob such as `Bash(accelerator:*)` rather than `Bash(accelerator <token>:*)`
  → guard fails.

- **major** / confidence medium — *The fixture-token verification procedure is
  under-specified in two ways*
  **Location**: Requirements / Acceptance Criteria (1st)
  0187 rests its whole verification strategy on "a unit test that adds a
  fixture token", but two preconditions that determine whether such a test is
  even writable are left open. First, the token→skill mapping rule: the guard
  currently reads a hardcoded `_VISUALISE_SKILL_RELATIVE`
  (`tasks/build.py:35`), and the generalised version could either scan the
  skills tree for whichever SKILL.md invokes `accelerator <token>` or consult
  an explicit token→path map — the two give different answers to "the skill
  binding is missing" for the same repository state. Second, no injection seam
  is named: `DISPATCHED_SUBBINARIES` is a module constant
  (`tasks/shared/paths.py:25`) and the skill paths are real repo paths, so
  adding a fixture token requires either a parameterised function signature or
  monkeypatching plus a fixture skills tree.
  **Impact**: Two implementers could satisfy AC1 with tests that exercise
  materially different behaviour, and the test may end up monkeypatching so
  much that it no longer covers the code path the release actually runs.
  **Suggestion**: State the discovery rule explicitly (e.g. "the guard
  resolves a token to its skill via an explicit token→skill-path map, absence
  from which is the exemption mechanism") and state the seam (e.g.
  "`validate_dispatch_coherence` takes the token collection and the skills
  root as parameters, defaulting to `DISPATCHED_SUBBINARIES` and the repo
  root, so tests pass fixtures rather than patching module state").

- **minor** / confidence medium — *The skill-exemption criterion names no
  verification and no declaration mechanism*
  **Location**: Acceptance Criteria (3rd)
  AC3 states "A token may be declared skill-exempt explicitly; an *undeclared*
  token with no skill consumer fails the guard", but neither says how an
  exemption is declared (a constant, a map sentinel, a per-token flag) nor
  requires a test for the exempt-and-passing path — AC2's "both the pass and
  fail paths" refers to the visualiser binding and the missing-binding
  failure.
  **Impact**: The exemption escape hatch is the mechanism most likely to be
  abused later (any awkward token gets exempted), yet nothing verifies it
  behaves as specified, so a verifier cannot conclusively confirm the
  criterion.
  **Suggestion**: Extend AC3 with an explicit verification: "a fixture token
  listed in the exemption declaration and having no skill passes the guard;
  the same token with the exemption removed fails it" — a paired test that
  pins both directions, and name where the exemption is declared.

- **minor** / confidence medium — *AC1's blanket "any token" predicate
  contradicts AC3's exemption*
  **Location**: Acceptance Criteria (1st and 3rd)
  AC1 requires the guard to fail "when any token's skill binding is missing",
  while AC3 requires that an explicitly declared skill-exempt token with no
  skill consumer passes. Read literally, the two criteria specify opposite
  outcomes for the same input.
  **Impact**: A test written against AC1's wording would assert a failure that
  AC3 requires to be a pass, so the criteria do not define a single
  determinate pass/fail predicate for exempt tokens.
  **Suggestion**: Qualify AC1 to "fails when any **non-exempt** token's skill
  binding is missing…", making AC3 the sole statement of the exemption
  carve-out.

- **minor** / confidence medium — *Nothing verifies the generalised guard
  still runs in its real release invocation path*
  **Location**: Acceptance Criteria (1st-2nd, 5th)
  Technical Notes record that `validate_dispatch_coherence` is "invoked from
  `tasks/manifest.py:138` so it runs on every release", but all specified
  verification is unit-level against `tests/unit/tasks/test_build.py`, and
  AC5's `mise run` gate does not exercise a release. A generalisation that
  changes the function's signature (e.g. to accept an injected token
  collection, as the fixture-token strategy likely requires) could leave the
  call site passing a stale or empty collection with every unit test still
  green.
  **Impact**: The guard could become vacuous in production — the exact failure
  AC1 works hard to prevent at unit level — with no criterion catching it.
  **Suggestion**: Add a criterion that the manifest-generation entry point
  still invokes the generalised guard over the real `DISPATCHED_SUBBINARIES`,
  verified by a test at the `tasks/manifest.py` call site (e.g. manifest
  generation fails when a fixture token is registered without its binding).

- **minor** / confidence medium — *"Still passes unchanged" does not define
  what is compared*
  **Location**: Acceptance Criteria (2nd)
  AC2 requires that "the existing visualiser binding still passes unchanged",
  without stating what "unchanged" is measured against — the binding
  declaration itself (no edits to the visualiser SKILL.md or its
  `allowed-tools`), the guard's verdict, or the existing test's assertions.
  **Impact**: An implementation that quietly relaxes the visualiser's
  `allowed-tools` expectation to make the generalised check pass would still
  be defensible under this wording, hiding a regression in guard strictness
  behind a green criterion.
  **Suggestion**: Restate as two observable facts: "no edits are made to the
  visualiser skill's frontmatter or to `_VISUALISE_SKILL_RELATIVE`'s target
  file, and the generalised guard reports the visualiser token as bound."

- **suggestion** / confidence medium — *The signing/upload/SLSA
  re-confirmation is assigned no verification step*
  **Location**: Assumptions
  The Assumptions section says signing, upload, re-verification and SLSA
  provenance "genuinely need no change… Worth re-confirming at implementation
  time; if any turns out visualiser-shaped, it belongs in this task", but no
  acceptance criterion covers that re-confirmation, so the conditional scope
  has no trigger a verifier can evaluate.
  **Impact**: The re-confirmation is easy to skip silently, and a
  visualiser-shaped assumption in `tasks/signing.py` or `tasks/github.py`
  would then surface inside the first consumer story — reinstating exactly the
  serialisation this extraction was meant to remove.
  **Suggestion**: Add a criterion such as "with the fixture token in
  `DISPATCHED_SUBBINARIES`, the signing-input list, the upload asset list and
  the re-verify globs each include the fixture token, asserted by test or by a
  recorded manual check in the work item's completion notes."

- **suggestion** / confidence low — *Consider a drift guard so the README
  checklist stays truthful*
  **Location**: Acceptance Criteria (4th)
  0187 makes `tasks/README.md` the canonical registration checklist for four
  downstream stories, but the only verification is that the section exists —
  nothing detects the checklist drifting from the code it describes as
  `tasks/shared/paths.py`, `tasks/manifest.py` and `tasks/build.py` evolve.
  **Impact**: A documentation-only artefact with no executable check will
  silently rot, and the consumer stories will trust a stale checklist rather
  than rediscovering the surface.
  **Suggestion**: Optionally add a lightweight test asserting the README
  checklist names each registration identifier (`DISPATCHED_SUBBINARIES`,
  `_SUBBINARY_MANIFESTS`, `package.description`, the `bin/<token>-*` gitignore
  pattern, `manifest.example.json`), so renaming a registration point fails a
  test rather than aging the docs.

## Re-Review (Pass 2) — 2026-08-01

**Verdict:** REVISE

All five lenses re-ran against the revised work item. Every one of pass 1's
seven majors had its root cause addressed, and eight of the eleven minors are
fully closed. But specifying the previously-vague mechanisms exposed the next
layer of detail beneath them: choosing scan-based discovery raised "what counts
as an invocation", and the new criterion discharging the signing/upload/SLSA
assumption (AC6) is unsatisfiable under the work item's own definition of a
fixture token — a defect the revision introduced rather than inherited.

### Previously Identified Issues

**Majors**

- 🟡 **Clarity / Scope / Testability / Completeness**: "One-line allowlist
  change" contradicts the checklist — **Resolved**. No lens raised it; scope
  now lists the explicit out-of-scope paragraph as a strength ("a reviewer can
  state what is in and out without inference").
- 🟡 **Completeness / Dependency / Testability / Scope**: Checklist narrower
  than research §8; AC4 cannot fail — **Partially resolved**. The eight points
  are now enumerated inline with file:line anchors (completeness: "the work
  item is self-contained and does not require reading the research"). But AC4
  still has no pass procedure (testability, major), and dependency found two
  further §8 details outside the eight-row table that the checklist drops.
- 🟡 **Clarity / Completeness / Testability**: Exemption mechanism undefined —
  **Resolved**. `SKILL_EXEMPT_SUBBINARIES` is named and sited, and AC3 pins
  both directions. Residual: no stated bar for when an exemption is justified
  (completeness, suggestion).
- 🟡 **Clarity / Testability**: "Fixture token" undefined; discovery rule and
  seam unspecified — **Partially resolved**. The token is now defined
  (test-only, injected, never registered) and the guard's seam is specified.
  But the discovery rule's *matching form* is still open, and the fixture
  token has no route to the signing/upload stages AC6 requires it to reach.
- 🟡 **Dependency**: Checklist omits the skill-binding/exemption step —
  **Resolved**. It is checklist point 7, and AC7 requires the README to carry
  it.
- 🟡 **Dependency**: 0170/0171/0173 lack reciprocal edges — **Resolved** via
  AC8, which dependency calls out as converting the gap into a checkable
  condition. Residual: the parent epic's own annotation still reads
  "unblocks 0169/0170/0171/0173" (dependency, minor).
- 🟡 **Testability**: `allowed-tools` mismatch untested — **Resolved**. AC2
  requires its own fixture case "rather than folded into the missing-binding
  test". Residual: the multi-skill quantifier is now ambiguous (clarity,
  major) and untested (testability, minor).

**Minors**

- 🔵 AC1 "any token" vs AC3 exemption — **Resolved** (non-exempt qualifier).
- 🔵 Assumption re-confirmation has no criterion — **Resolved in intent** via
  AC6, but AC6 as written is unsatisfiable (see New Issues) and omits SLSA
  provenance, one of the four stages the assumption claims to discharge.
- 🔵 Unbounded absorption clause — **Resolved**. Residual: "local to the token
  loop" is itself undefined (clarity, minor).
- 🔵 Guard's real release-path invocation unverified — **Resolved in intent**
  via AC5; the criterion exists but names no artefact (see New Issues).
- 🔵 "Still passes unchanged" undefined — **Resolved** (two observable facts).
  Residual: it anchors on `_VISUALISE_SKILL_RELATIVE`, which the change likely
  deletes (testability, minor).
- 🔵 0172 absent from blocks — **Addressed**, but scope notes epic 0136 and
  0169 both record 0172 outside the sub-binary set, so the records now
  disagree.
- 🔵 0168 "code landed" but still `ready` — **Partially resolved**. The
  closure instruction is now prose under *Related*; dependency argues it is a
  start-gate and belongs on `blocked_by` (major, see below).
- 🔵 "SKILL↔producer" terminology — **Resolved**.
- 🔵 "Generalise it" ambiguous referent — **Resolved**.
- 🔵 "Documented nowhere" vs References — **Resolved**.
- 🔵 "Ordered" with no ordering principle — **Resolved** ("in the order an
  author would work through them").

**Suggestions**

- 🔵 "review-2, pass 4" provenance — **Resolved** (artefact linked, scope lens
  named as actor).
- 🔵 README drift guard — **Addressed** as an optional Technical Note;
  testability now argues it should be promoted into AC4 (see below).

### New Issues Introduced

- 🟡 **Clarity / Completeness / Scope / Testability**: AC6 is unsatisfiable
  under the work item's own definitions. A fixture token is defined as
  reaching code only through the guard's injected parameters and never being
  registered in `DISPATCHED_SUBBINARIES` — but the signing, upload and
  re-verify stages have no such parameter and, per Assumptions, read
  `DISPATCHED_SUBBINARIES` directly. Four lenses reached this independently.
  The fix is either a Requirements bullet extending the injectable-collection
  treatment to those three builders, or restating AC6 in terms of a seam that
  already exists.
- 🟡 **Clarity / Completeness / Testability**: "Scanning the skills tree for
  the invocation" never says what constitutes an invocation — a SKILL.md body
  line, a `!`-preprocessor line, an `allowed-tools` entry, or a skill-local
  script. Real skills use the plugin-root form
  (`${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs status`), not the bare
  `accelerator <token>` the criteria write. Worse, if `allowed-tools` is also
  the discovery signal, AC1 and AC2 become indistinguishable — a glob-only
  skill would simply never be discovered as a consumer.
- 🟡 **Testability**: Nothing distinguishes a guard that checks every token
  from one that checks only the first. Every fixture case uses a single token
  and the real collection has one entry, so an implementation inspecting only
  `[0]`, or passing vacuously on an empty collection, satisfies all nine
  criteria — the exact hazard AC5 names.
- 🟡 **Testability**: AC4's "carries the registration checklist covering all
  eight points" still has no pass procedure; eight headings with no usable
  content would satisfy it. The only mechanical check is filed as *Optional*
  in Technical Notes.
- 🟡 **Testability**: AC5's "verified at the `tasks/manifest.py:138` call
  site" reads as code inspection, which cannot regress-protect against the
  signature change the criterion exists to guard against.
- 🟡 **Clarity**: The multi-skill quantifier is ambiguous. Requirement 1 is
  singular ("**that** skill's `allowed-tools`"), the next requirement permits
  multiple consumers, and AC2 reads as a rule over every consumer — so it is
  undefined whether one correctly-scoped consumer suffices when a sibling
  skill is glob-only.
- 🟡 **Dependency**: The 0168 start-gate is prose under *Related* with no
  `blocked_by` edge. Sibling 0169 puts the identical 0167 gate on
  `blocked_by`, so the epic's convention is clear, and schedulers read the
  frontmatter rather than the prose.
- 🟡 **Dependency**: The checklist claims to cover "every registration point
  enumerated in research §8" but drops a §8 constraint stated outside the
  eight-row table: the token derives `ACCELERATOR_<TOKEN>_BIN`, so a token
  containing `_` is rejected by the launcher. 0170 ("Work-Item Subdomain") is
  the consumer most likely to reach for `work_item` and rediscover this.
- 🔵 **Dependency**: Checklist point 6 names
  `cli/launcher/tests/fixtures/manifest.example.json` but not its co-reader
  `tests/unit/tasks/test_manifest.py:38`, which §8 records as a coupled pair.
- 🔵 **Completeness**: AC6 omits SLSA provenance, one of the four stages the
  Assumptions section claims it discharges.
- 🔵 **Clarity**: The "skills root" parameter is said to default to "the repo
  root" — name and default disagree.
- 🔵 **Clarity**: "The coherence check at `tasks/build.py:74-101`" (checklist
  point 3) is the *version*-coherence check, not the guard this task
  generalises; only the line numbers distinguish them.
- 🔵 **Clarity / Completeness**: The fate of the two visualiser hardcodings
  named in Context is unstated — the Requirements address
  `_VISUALISE_SKILL_RELATIVE` but not the `"visualiser" in
  DISPATCHED_SUBBINARIES` assertion, and AC4 still refers to the constant in
  the present tense.
- 🔵 **Clarity**: "Local to the token loop" is the scope boundary but is never
  defined.
- 🔵 **Scope**: The consumer set now disagrees with its own references — epic
  0136 annotates 0187 as unblocking four stories, and 0169 groups 0172 outside
  the sub-binary set (its coupling to 0169 is the `hooks.json` rewrite).
- 🔵 **Testability**: The permitted multi-skill case has no criterion.
- 🔵 **Testability**: AC4's regression anchor (`_VISUALISE_SKILL_RELATIVE`)
  may not exist after the change.
- 🔵 **Completeness**: No Open Questions section, though the 0168 residual
  scope question is genuinely open and parked in Dependencies as an
  instruction.

### Assessment

The revision worked on the substance: the headline contradiction is gone, the
checklist is self-contained and covers the §8 table, the exemption mechanism
is named and pinned in both directions, and the four missing reciprocal edges
now have a criterion. Scope's verdict shifted markedly — it raised no majors
at all this pass, against one in pass 1.

But the work item is not yet ready. One new defect is disqualifying on its
own: AC6 cannot be satisfied by any mechanism the Requirements authorise, so
an implementer would either expand scope unplanned or fall back to the
module-patching the item elsewhere rules out. Two more are structural rather
than cosmetic — the invocation-matching rule is the single most load-bearing
undefined term in the item (a too-narrow matcher makes the guard vacuous for
every future token, the exact failure the task exists to prevent), and no
criterion distinguishes iterating the collection from inspecting its first
entry. The remaining majors are cheap: promote the 0168 gate to `blocked_by`,
add the no-underscore token constraint, state the multi-skill quantifier once,
and give AC4 and AC5 executable pass procedures.

A third pass focused on those five points should reach APPROVE. None require
rethinking the approach — the decisions taken between passes (scan-based
discovery, test-only fixture token, parameter injection) all held up under
re-review; what they need is one more level of specification.

## Re-Review (Pass 3) — 2026-08-01

**Verdict:** REVISE

Two lenses now raise no majors at all (completeness 1→2→0, scope 1→0→0), and
every pass-2 major is closed. The pass-3 majors are narrower and mostly
mechanical, but two are substantive: the permission-half requirement inverts
how `covered_by` is actually used, which would produce a guard that accepts the
ancestor-glob rule AC3 requires it to reject; and nothing asserts the
visualiser hardcoding is actually *removed*, so the task's central outcome has
no pass/fail procedure.

### Previously Identified Issues

- 🟡 AC6 unsatisfiable — **Resolved**. The three release-stage builders now get
  the same parameter-with-default shape, and AC10 states the observable per
  stage plus "no signing key and no network access are required".
- 🟡 "What counts as an invocation" undefined — **Resolved**. Defined against
  `preprocessor_commands` / `is_plugin_invocation`, with a dedicated criterion
  pinning that prose and backticked references do not bind. Clarity now lists
  the two-properties-must-survive callout as a strength.
- 🟡 Iteration not distinguished from checking `[0]` — **Resolved** by AC2,
  which testability singles out for asserting error *content* ("the error names
  the second token").
- 🟡 AC4 had no pass procedure — **Resolved**. Promoted from an optional note
  into AC12 with an enumerated literal-string list; residual gaps in *which*
  strings (below).
- 🟡 AC5 "verified at the call site" — **Resolved**. Now a spy test; testability
  calls it out as pinning the classic signature-change failure.
- 🟡 Multi-skill quantifier ambiguous — **Resolved**. Stated once in
  Requirements, confirmed by AC5 rather than contradicted.
- 🟡 0168 gate not on `blocked_by` — **Resolved** in this item's frontmatter,
  but the edge is unreciprocated (below).
- 🟡 Token no-underscore constraint missing — **Resolved** in the Requirements;
  not yet anchored in AC12's string list (below).
- 🔵 `manifest.example.json` co-reader — **Resolved**, and corrected: the real
  co-readers are `tests/unit/tasks/test_manifest_contract.py:16` and the
  `include_str!` at `cli/launcher/src/launch/outbound/resolve/manifest.rs:135`.
  Research §8's `test_manifest.py:38` is stale. Dependency flagged the
  divergence — the work item should say it is a correction, not a discrepancy.
- 🔵 SLSA omitted from the discharge — **Resolved**, and sharpened: the
  `subject-path` is only partly token-generic, now recorded as a documented
  condition.
- 🔵 "Skills root" vs repo root, "coherence check" collision, "local to the
  token loop", exemption bar, visualiser anchor by path, Open Questions
  section, epic annotation — **all resolved**.

### New Issues Introduced

- 🟡 **Clarity**: The permission half inverts `covered_by`. The requirement
  says the rule must "name the subcommand, checked with `covered_by` against a
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token>` probe" — but an ancestor glob
  *does* cover that probe, so a `covered_by`-only check passes exactly the rule
  AC3 requires to fail. In `skill_permissions.py` the helper is used the other
  way round: `covered_by(_BARE_LAUNCHER, rule)` detects a rule that is *too
  broad*. The two conditions need stating separately and in order.
- 🟡 **Testability**: No criterion verifies the hardcoding is removed. Every
  criterion would pass if a generalised guard were added *alongside* the
  retained `_VISUALISE_SKILL_RELATIVE` check. The task's headline outcome has
  no pass/fail procedure.
- 🟡 **Testability**: The "no behavioural change" claim for the three builders
  is discharged only for *injected* values. A mis-wired default (empty tuple,
  wrong constant) satisfies every criterion while silently emptying the real
  release's asset set.
- 🟡 **Testability**: AC12's literal-string list omits several
  Requirements-mandated contents — the two `manifest.example.json` co-readers,
  `ACCELERATOR_<TOKEN>_BIN`, `cli/Cargo.toml`, and the points-1-and-7
  same-change rule. The highest-consequence entries are the unpinned ones.
- 🟡 **Dependency**: The hand-off edges are written into 0170–0173 at *this
  task's acceptance* — i.e. when they stop constraining anything. While 0187 is
  in flight those four still look unblocked, which is the serialisation the
  Summary says the task exists to prevent. The `blocked_by` half should be a
  now action.
- 🟡 **Dependency**: The 0168 edge is unreciprocated and contradicts the epic.
  0136 still annotates 0187 as "*(no blockers; …)*" and places it under
  Foundations while 0168 sits under Subdomain migrations. AC13 corrects only
  the "unblocks" list; 0168 gains no reciprocal `blocks` edge.
- 🟡 **Dependency**: The launcher's built-in subcommand list (`version`,
  `config`) is now duplicated into Python with no lockstep obligation recorded
  — a future built-in would leave the guard's set stale, surfacing as a red
  release path attributed to an unrelated change.
- 🟡 **Clarity**: "Fixture manifest" has two referents — the shipped golden
  `manifest.example.json` in the Summary/checklist, and a test-constructed
  manifest in AC10.
- 🔵 **Dependency**: The visualiser skill's *permission* shape is asserted as
  an outcome, not recorded as a precondition, and both fallbacks are closed
  (no SKILL.md edits; exemption set must be empty). *(Verified during editing:
  `skills/visualisation/visualise/SKILL.md:8` carries
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser *)`, a
  subcommand-naming rule — the precondition holds and should simply be
  recorded.)*
- 🔵 **Dependency**: The new coupling on `skill_permissions.py` internals —
  including the private `_BARE_LAUNCHER` — is not recorded as a dependency.
- 🔵 **Clarity / Completeness / Scope**: The hand-off work and the
  release-stage signature changes appear in Requirements/AC but not in the
  Summary's two-deliverable framing; the hand-off has no Requirements bullet
  at all.
- 🔵 **Clarity**: AC9's "called exactly once with `DISPATCHED_SUBBINARIES` and
  `SKILL_EXEMPT_SUBBINARIES`" is ambiguous when the parameters default — passed
  explicitly, or resolved to those values?
- 🔵 **Clarity**: AC4's "different token" case does not say whether that token
  is registered; if not, the reverse-direction rule also fires and the test
  proves the wrong thing.
- 🔵 **Testability**: No criterion pins the *passing* half of the built-in
  allowance (a skill invoking `accelerator config` must pass).
- 🔵 **Testability**: AC13's "dated note" has no defined location, content or
  check across five files.
- 🔵 **Completeness**: No Validation Results section for the two
  manually-discharged items (SLSA inspection, visualiser manifest path).
- 🔵 **Scope**: The epic-annotation correction is partial — it fixes the
  "unblocks" list but leaves the now-false "no blockers" clause.

### Assessment

The trajectory is clearly convergent: seven majors in pass 1, eight in pass 2
(mostly consequences of newly-specified mechanisms), nine in pass 3 — but the
*character* has changed completely. Pass-1 majors were missing mechanisms;
pass-3 majors are one inverted helper call, three missing assertions, and three
bookkeeping edges. Two lenses now raise none at all.

Two must be fixed before implementation. The `covered_by` inversion is a
semantic defect in the specification — implemented literally it produces a
guard that accepts the exact rule shape the task exists to reject. And with no
criterion asserting the hardcoding is gone, the work could be accepted with
`_VISUALISE_SKILL_RELATIVE` intact, leaving the next five stories the surface
this task was extracted to remove.

The rest are cheap and mechanical: extend AC12's string list, add a default-path
assertion, record the visualiser permission precondition (already verified to
hold), reciprocate the 0168 edge and finish the epic annotation, move the
`blocked_by` writes to now, add a built-in-list lockstep note, and disambiguate
"fixture manifest". A fourth pass carrying those should reach APPROVE — and at
that point further passes would be polishing prose rather than reducing
implementation risk.

## Re-Review (Pass 4) — 2026-08-01

**Verdict:** REVISE

Between passes 3 and 4 the work item took the pass-3 fixes and the reciprocal
dependency edges were written into the five sibling documents. Pass 4 confirms
the substance landed: the `covered_by` inversion is corrected, the
"hardcoding is gone" criterion exists, the defaults are separately asserted,
and dependency verified the sibling edges directly ("all four carry
`blocked_by: work-item:0187` plus dated Dependencies bullets, and 0168 carries
`blocks: work-item:0187`"). Its major count fell 4→1; scope held at zero for a
second pass.

The pass-4 majors are almost entirely a new class: **internal consistency
debt created by the edits themselves**. Sections that were updated now
contradict sections that were not.

### Previously Identified Issues

- 🟡 `covered_by` inversion — **Resolved**. The permission half is now two
  conditions over a single rule, with the omission failure called out.
  Clarity lists the operational definitions as a strength.
- 🟡 No criterion asserting the hardcoding is removed — **Resolved**. Now the
  first criterion, a source-scan test. Residual: "its helpers" is an
  undefined scan scope (testability, minor).
- 🟡 Defaults not asserted for the three builders — **Resolved** by the
  "…and for the defaults" criterion, which testability singles out as
  targeting exactly the bug class a parameter-with-default refactor
  introduces. Residual: "today's visualiser asset set" is not enumerated.
- 🟡 AC12's string list incomplete — **Resolved** (fifteen literals).
  Residual: nothing from checklist point 10 is pinned.
- 🟡 Hand-off edges written at acceptance — **Resolved**. Recorded up front and
  independently verified by the dependency lens this pass.
- 🟡 0168 edge unreciprocated, epic annotation stale — **Resolved**. 0168
  carries `blocks`, the epic names the prerequisite and lists five consumers.
- 🟡 Built-in list lockstep unrecorded — **Resolved** as checklist point 10.
  Dependency now lists it as a strength ("captures a lockstep coupling that
  would otherwise be invisible").
- 🟡 "Fixture manifest" ambiguous — **Resolved** (golden fixture vs in-test
  manifest).
- 🔵 Visualiser permission precondition — **Resolved** as a dated assumption
  quoting the rule. But its fallback now collides with a criterion (below).

### New Issues Introduced

**Consistency debt from the pass-4 edits** — flagged by four lenses:

- 🟡 **Clarity / Dependency**: The Dependencies "Blocks" bullet still reads
  "Only 0169 currently records the reciprocal edge … closes that gap at
  pickup", contradicting the Hand-offs subsection headed **Done 2026-08-01**
  and the ticked criterion. Hand-offs also says "Nothing further is owed to the
  siblings at acceptance" while the criterion requires a grep re-verification.
- 🟡 **Clarity / Dependency**: The 0168 discharge condition is stated three
  ways — "cannot move the visualiser crate path" (Dependencies, Hand-offs),
  "documentation-only" (Open Questions), and a proceed-anyway default that
  neither 0168's reciprocal bullet nor the epic records. Residual scope that is
  code-bearing but path-safe satisfies one test and fails another.
- 🟡 **Completeness / Dependency / Scope / Testability**: The helper-promotion
  deliverable exists only as one sentence in Dependencies — no Requirements
  bullet, no criterion. Every behavioural criterion passes if the guard simply
  imports `_BARE_LAUNCHER` privately, which is the coupling the sentence exists
  to remove. Four lenses raised this independently.
- 🟡 **Clarity / Completeness / Scope / Testability**: The signing-extraction
  fallback silently voids two unconditional criteria and contradicts the
  Assumptions' claim that those criteria discharge the parameterisation by
  test. No alternative discharge route and no Validation Results placeholder.
- 🔵 **Clarity / Testability**: The "No edits are made to
  `skills/visualisation/visualise/SKILL.md`" criterion forbids exactly the
  scoped `allowed-tools` edit the first Assumption prescribes as its fallback.

**A logical defect in one criterion:**

- 🟡 **Testability**: The spy criterion cannot observe what it asserts. If the
  call site passes no override, the spy receives no collection argument and the
  real function's default resolution never runs — any spy reporting those
  values was handed them, making the assertion tautological. Needs splitting
  into a spy assertion (no argument passed) plus a signature-identity assertion
  on the defaults.

**Smaller gaps:**

- 🔵 The Summary says edges land on "the five consuming items"; Hand-offs
  records four consumers plus the 0168 blocker.
- 🔵 The guard Requirements are written against the module constants where the
  criteria require the injected parameters.
- 🔵 `token→consumer` is introduced in Context as a drift-preventing label and
  then never used again; `invocation→registration` is used throughout.
- 🔵 "Applied in order" describes a conjunction as if ordered; the failure mode
  glossed as "backwards" is an omission.
- 🔵 Checklist point 10 names no artefact — the one entry an author is least
  likely to know is the only one with no target and no doc-ageing literal.
- 🔵 `SKILL_EXEMPT_SUBBINARIES` "empty when this task lands" has no criterion,
  though seeding it is the named vacuity failure mode.
- 🔵 The "prose does not bind" fixture does not say whether the skill carries a
  correct permission rule; without one it fails on the permission half and
  never exercises the matcher.
- 🔵 The golden-fixture no-modification boundary is stated but unasserted,
  unlike the parallel SKILL.md boundary.
- 🔵 0170–0173 remain sequenced behind 0169 for an unrelated reason (its
  hook-envelope module decision), so the de-serialisation is partial.
- 🔵 0182 is filed as merely Related despite a stated vacuous-guard failure
  mode, with no status, owner or discharge route.
- 🔵 0172's hand-off bullet lacks the exemption-route warning its hook-only
  consumer will need — the consumer most likely to hit the exemption path is
  the one least warned.
- 🔵 SLSA is used without expansion; the hand-off re-grep has no Validation
  Results line.

### Assessment

Majors by pass: 7 → 8 → 9 → 10. The count is flat-to-rising, but it is
measuring something different each time. Pass-1 majors were missing mechanisms;
pass-3 majors were missing assertions; pass-4 majors are five contradictions
between sections and one criterion that cannot be implemented as written. Only
the spy defect and the helper-promotion gap are substantive — the other four
majors are the same edit-induced staleness in different places, fixable by
reconciling three sections against each other.

This is the signature 0169's own review recorded: "three editing passes had not
reduced its major-finding count … with most later findings being defects
introduced by the previous pass's fixes". 0187 has now reached the same regime
at a much finer grain. The work item is well past the specification bar for
implementation — fifteen criteria with named fixtures and observables, a
defined verification strategy, complete and reciprocated dependency edges. What
remains is editorial reconciliation, and each further round of it carries its
own chance of introducing the next contradiction.

Recommended stopping point: fix the spy criterion (it is wrong, not merely
imprecise), promote the helper-promotion deliverable into Requirements with a
criterion, and reconcile the three stale cross-references — Dependencies vs
Hand-offs, the 0168 discharge wording, and the two fallback-vs-criterion
collisions. Then implement. Further lens passes should be spent on the
implementation, not the document.

## Re-Review (Pass 5) — 2026-08-01, final

**Verdict:** REVISE

All three pass-4 recommendations were carried out: the spy criterion split into
a spy assertion plus an `inspect.signature` identity assertion, the
helper-promotion deliverable moved into Requirements with its own criterion, and
the stale cross-references reconciled across four documents.

**The major count fell for the first time — 7 → 8 → 9 → 10 → 6.** Completeness
and scope both returned zero majors, and testability now opens with "an
unusually testable work item", singling out the spy/signature pairing as "test
design reasoning at the specification level".

Of the six remaining majors, two are genuinely new defects, two are pass-4
minors that were left unfixed and have been re-raised at higher severity, and
two are boundary questions the edits surfaced rather than created.

### Previously Identified Issues

- 🟡 Spy criterion could not observe what it asserted — **Resolved**. Now two
  criteria; testability lists the pairing as a strength and quotes the
  work item's own explanation of why neither alone suffices.
- 🟡 Helper promotion untracked — **Resolved**. In Requirements as a
  one-symbol rename with a source-scan criterion. Scope calls the ride-along
  "bounded to a single symbol plus its one existing caller … justified by a
  stated coupling argument rather than convenience". Residual: the criterion
  asserts the new name exists but not that the old one is gone (testability,
  minor).
- 🟡 Dependencies vs Hand-offs staleness — **Resolved**. No lens re-raised it.
- 🟡 0168 discharge wording stated three ways — **Resolved** as a single
  mirrored formulation; dependency lists the mirroring as a strength. But see
  the new finding on its *scope*.
- 🟡 Signing fallback voiding two criteria — **Resolved** via the carve-out;
  testability now lists the conditional-path handling as a strength ("a
  deviation is recorded rather than argued").
- 🟡 Visualiser SKILL.md no-edits vs its fallback — **Resolved** (conditional
  criterion, Validation Results slot).
- 🔵 "Applied in order" ordering language — **Resolved**.
- 🔵 Summary deliverable count — **Resolved**.
- 🔵 Default-case cardinalities — **Resolved** (4 / 8 / 4), listed as a
  strength.

### Remaining Majors

**Genuinely new:**

- 🟡 **Clarity**: "Iterating the collection subsumes the membership assertion"
  is false. If the collection were emptied or the token dropped, the loop
  checks nothing and passes, whereas the `"visualiser" in
  DISPATCHED_SUBBINARIES` assertion would fail. The justification contradicts
  the item's own non-vacuity demand and its refusal to seed the exemption set
  on vacuity grounds. As written it licenses removing the one assertion that
  stops the release guard passing on an empty allowlist.
- 🟡 **Testability**: No criterion would fail if the guard **duplicated** the
  parser instead of reusing it. The behavioural criteria all pass against an
  inline re-implementation, and the shared-contract criterion only forbids
  underscore-prefixed imports — importing nothing at all satisfies it. The
  reuse requirement is the entire justification for the `BARE_LAUNCHER` rename
  and the Shared-artefact entry, and it is unverified. Needs a positive
  assertion that the guard imports the six names and defines no local
  equivalent.

**Pass-4 minors re-raised as majors** (left unfixed by choice):

- 🟡 **Testability**: "An imperative action line" cannot be mechanically
  decided, in a criterion titled "mechanically checkable".
- 🟡 **Testability**: The source-scan region "its helpers" is undefined, and
  `tasks/build.py` legitimately holds visualiser-specific material elsewhere
  (cross-compile staging, the version-coherence check).

**Boundary questions the edits surfaced:**

- 🟡 **Clarity**: `blocked_by: ["work-item:0168"]` versus a discharge condition
  that ends "proceed anyway". No stated circumstance prevents pickup, so the
  machine-readable edge and the prose disagree — and 0187 blocks five stories,
  so a task the prose says is never blocked can present as serialising them.
- 🟡 **Dependency**: The 0168 discharge condition is scoped to the *crate
  path*, but 0168 also moves the visualiser's `skills/.../bin` staging and
  retires its `bin/checksums.json` — surfaces checklist points 5 and 9
  document. The acceptance-time re-verification is scoped too narrowly to
  catch that.

### Notable Minors

`token→consumer` is still the only unused half of the drift-prevention labelling
Context promises; the guard Requirements still speak of the module constants
where the criteria require the injected collection; `SKILL_EXEMPT_SUBBINARIES`
being empty at landing is prose-only with no assertion; the built-in criterion
exercises `config` but never `version`; checklist point 10's content is the only
one unpinned by a literal string; the SLSA inspection has no stated pass
condition; 0172's `blocked_by` still omits 0169 (a gap 0169's own review noted);
no Drafting Notes section; and 0182 carries no reciprocal edge despite being the
one in-flight related item with a silent failure mode.

### Assessment — closing

Five passes, four editing rounds. The curve finally turned: majors fell from ten
to six, and the two lenses that assess whether the item is *the right size and
fully populated* have been silent for two passes running. What remains splits
cleanly in three.

**Worth fixing before implementation** — one item. The "subsumes" claim is
wrong, and it is wrong in the direction that matters: an implementer following
it removes an anti-vacuity assertion while believing the work item authorised
the removal as behaviour-preserving. That is a one-sentence fix.

**Worth fixing if the guard is to hold its shape over time** — the
parser-duplication gap. Everything about the shared-contract dependency and the
rename rests on reuse that nothing verifies.

**Everything else is documentation quality**, not implementation risk. The two
re-raised testability majors describe tests that would be weaker than claimed,
not code that would be wrong. The `blocked_by` tension is a metadata convention
question. The remaining minors are labelling, cross-references and assertions
that would each tighten the document without changing what gets built.

This review is closed at pass 5. The work item is specified well past the bar
for implementation: seventeen criteria with named fixtures, injected inputs and
quantitative expectations; a verification strategy independent of any consumer,
signing key or network; complete reciprocated dependency edges across six
documents; and pre-declared fallbacks with recording slots for every conditional
path. Further passes would continue to find things — that is what lenses do —
but the yield is now prose quality, and each editing round has demonstrably
carried its own risk of introducing the next inconsistency.

---

## Verdict: APPROVE — 2026-08-01

Set by the reviewer after pass 5, superseding the pass-5 lens verdict of
REVISE. Recorded here so the frontmatter and the pass sections do not read as
contradicting each other: **this is an author decision, not a sixth lens run.**

**Fixed after pass 5, before approval** — both substantive findings:

- The "iterating the collection subsumes the membership assertion" claim was
  false — an emptied allowlist makes the loop check nothing and pass. The
  Requirements now say so explicitly and require the guard to fail on an empty
  resolved collection, with a dedicated criterion. This was the one finding
  that could have caused wrong code.
- The parser-duplication gap is closed: the shared-contract criterion now
  asserts the imports **positively** (all six names imported, none shadowed, no
  local regex against `Bash(` or the `!`-preprocessor form) rather than only
  forbidding private imports. The alias hole in the same criterion was closed
  at the same time — `_BARE_LAUNCHER` must be absent under `tasks/`, so a
  retained alias cannot satisfy the rename.

**Accepted as-is** — the four remaining pass-5 majors, all documentation
quality rather than implementation risk:

- "An imperative action line" is not mechanically decidable, in a criterion
  titled "mechanically checkable" — the test will be weaker than the criterion
  claims, but the checklist content is still pinned by fifteen literal strings.
- The source-scan region "its helpers" is undefined; the implementer will scope
  it, and `tasks/build.py` holds legitimate visualiser-specific material
  (cross-compile staging, the version-coherence check) that the scan must
  exclude.
- `blocked_by: ["work-item:0168"]` versus a discharge condition ending "proceed
  anyway" — a metadata convention question. The prose is the operative
  statement: this is a confirmation gate, not a wait.
- The 0168 discharge condition covers the crate path but not the visualiser's
  `skills/.../bin` staging or its retired `checksums.json` — checklist points 5
  and 9 could go stale if 0168's residual scope touches them. The
  acceptance-time re-verification should be read as covering those points too.

The minors and suggestions across all five passes are left unactioned by
decision. They are recorded in the pass sections above and remain available if
the item is revisited.

**Final state**: 18 acceptance criteria; reciprocal dependency edges recorded
across six documents; five Validation Results slots for the conditional and
inspection-only paths. Work item status stays `ready` — review does not
transition status, and the 0168 confirmation gate is discharged at pickup.
