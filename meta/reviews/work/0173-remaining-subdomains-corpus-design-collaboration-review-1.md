---
type: work-item-review
id: "0173-remaining-subdomains-corpus-design-collaboration-review-1"
title: "Work Item Review: Remaining Subdomains: corpus, design, collaboration"
date: "2026-08-05T18:55:14+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0173"
work_item_id: "0173"
reviewer: Toby Clemson
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 1
tags: []
last_updated: "2026-08-05T18:55:14+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Remaining Subdomains: corpus, design, collaboration

**Verdict:** REVISE

The item is internally consistent and unusually well-maintained for its Dependencies
section (concrete resolved-blocker evidence, a reusable registration-checklist
pointer), but two structural problems recur across lenses: it bundles three
functionally independent migration efforts (`accelerator-corpus`,
`accelerator-design`, `accelerator-collaboration`) into a single `story`, and its
Acceptance Criteria leave a stated Requirement (the skill call-site/`allowed-tools`
rewrite) unverified while two other criteria rely on subjective baselines
("launches correctly", "shells to `gh` as before"). The item's own Drafting Notes
already flag that Acceptance Criteria, Requirements, and `kind` were not reviewed
in the last enrichment pass — this review's findings confirm that pass is still
needed.

### Cross-Cutting Themes

- **Skill call-site/`allowed-tools` rewrite is unverifiable** (flagged by:
  completeness, testability) — Requirements' fourth bullet (apply the Q7
  interface-redesign principle; rewrite call sites + `allowed-tools`) has no
  matching Acceptance Criterion, so this requirement could be left undone while
  every stated AC still passes.
- **Story bundles three independently deliverable efforts** (flagged by: scope,
  echoed by dependency and completeness) — `accelerator-corpus`,
  `accelerator-design`, and `accelerator-collaboration` share no functional
  relationship beyond the shared registration pattern; each has its own source
  scripts, target crate(s), skill domain, and test suite. The Context section's
  own "may be split into three separate work items" line, and the Open
  Questions/Requirements duplication over the Playwright executor's fate, are
  symptoms of this same unresolved granularity question.
- **Acceptance Criteria lean on undefined baselines** (flagged by: testability) —
  "launches correctly" and "shells to `gh` as before" give a verifier no
  observable pass/fail threshold, compounding the risk that the bundled scope
  above makes partial completion hard to assess cleanly.

### Findings

#### Major

- 🟡 **Completeness/Testability**: No acceptance criterion covers the skill
  call-site / `allowed-tools` rewrite requirement
  **Location**: Requirements; Acceptance Criteria
  Requirements' fourth bullet mandates rewriting affected skills' call sites and
  `allowed-tools` frontmatter per the Q7/0167 contract, but none of the four
  Acceptance Criteria checks this — an implementer could satisfy every stated AC
  while leaving skill call sites unrewritten or `allowed-tools` stale.

- 🟡 **Scope**: Three independent bounded contexts bundled into a single story
  **Location**: Summary; Requirements
  `accelerator-corpus`, `accelerator-design`, and `accelerator-collaboration` are
  functionally unrelated efforts, each with its own source scripts, target
  crate(s), skill domain, and characterization/repointed test suite. They could
  be implemented, reviewed, and merged independently, risking an oversized PR or
  unclear partial-completion semantics if one binary blocks while the others are
  ready.

- 🟡 **Testability**: "Launches correctly" has no defined pass/fail threshold
  **Location**: Acceptance Criteria
  The design-binary criterion — "the Playwright executor still launches
  correctly" — doesn't define "correctly" (exit code, absence of errors, a
  specific artefact, parity with a prior invocation), so a verifier cannot
  conclusively determine pass/fail.

- 🟡 **Testability**: "Shells to gh as before" relies on an undefined baseline
  **Location**: Acceptance Criteria
  The collaboration-binary criterion offers no enumerated `gh` sub-commands,
  flags, or expected outputs, and "before" is only recoverable by reading the
  source bash scripts — not itself named as the verification mechanism.

- 🟡 **Scope**: Story kind likely undersized for the combined scope
  **Location**: Frontmatter: kind
  The combined scope requires three separate sub-binary registrations (each
  following a 13-point checklist), skill/`allowed-tools` rewrites across three
  domains, and characterization suites per binary — materially larger than the
  single-binary stories seen elsewhere in the same epic (e.g., 0179, 0169).

#### Minor

- 🔵 **Dependency**: 0179 (the crate-delivering work item) is not named alongside
  its parent 0166
  **Location**: Dependencies
  The item's own References cite 0179 as the source of the crates this binary
  sits on, and 0179 explicitly lists 0173 as a consumer it blocks — yet
  Dependencies names only the coarser parent 0166 as the resolved blocker.

- 🔵 **Dependency**: Downstream consumer 0174 not named as a Blocks entry
  **Location**: Dependencies
  Work-item 0174 lists 0173 in its own `blocked_by` and keys its floor-decrement
  work directly to this story's shell removal, but this item's Dependencies
  section has no reciprocal "Blocks" entry — the coupling is visible only from
  0174's side.

- 🔵 **Clarity**: "Q5"/"Q7" shorthand collides with the work item's own Open
  Questions section
  **Location**: Context; Requirements; Assumptions; Technical Notes
  These labels refer to numbered questions in an external research document, but
  this item has its own differently-numbered "Open Questions" section, so a
  reader unfamiliar with the epic-level document may conflate the two or fail to
  resolve the shorthand at all.

- 🔵 **Clarity**: "Open" github→collaboration rename reads as undecided, but
  Requirements state it as settled
  **Location**: Summary; Context; Requirements
  Summary/Context call it "the open github→collaboration rename" while
  Requirements states flatly "Domain named `collaboration`, not `github`" with no
  hedge — the two registers could read as describing different things.

- 🔵 **Clarity**: Playwright executor's fate stated as either/or in Requirements,
  then separately flagged as unresolved in Open Questions
  **Location**: Requirements; Open Questions
  The same undecided point (thin-wrapper vs. folded into the binary) appears
  twice in different registers, risking a reader treating it as already settled
  since it appears in Requirements rather than only as an Open Question.

- 🟡 **Completeness**: Item's own notes flag acceptance criteria, requirements,
  and kind as not yet reviewed
  **Location**: Drafting Notes
  The Drafting Notes explicitly state these sections "were not reviewed this
  round — status intentionally left at draft pending that pass," meaning the
  content just evaluated is self-flagged as provisional.

- 🔵 **Scope**: Split-or-keep decision left open without criteria
  **Location**: Context
  The item states it "may be split into three separate work items if finer
  granularity is wanted" but doesn't commit to a decision or state what would
  trigger a split, leaving the question to resurface mid-implementation when
  splitting is more disruptive.

- 🔵 **Testability**: "Characterization tests where none exist" has no defined
  coverage bar
  **Location**: Acceptance Criteria
  No minimum coverage is specified (e.g., happy path plus one failure path per
  sub-command), so a single trivial test per gap could technically satisfy the
  criterion.

- 🔵 **Testability**: Registration-checklist compliance is stated as a dependency
  but not as a verifiable criterion
  **Location**: Dependencies
  No Acceptance Criterion asserts the 13-point registration checklist was
  actually applied and verified for all three binaries, so incomplete
  registration could pass under the stated ACs.

- 🔵 **Completeness**: No identification of the beneficiary whose need is met by
  this migration
  **Location**: Context
  As a `story`, the item is expected to name who benefits; this is left implicit
  and inherited from the parent epic.

#### Suggestions

- 🔵 **Clarity**: "The relevant skills" is less specific than the named paths
  given in Acceptance Criteria
  **Location**: Requirements
  Requirements' fourth bullet doesn't name which skills are relevant, though
  Acceptance Criteria later enumerates the specific paths.

- 🔵 **Dependency**: `gh` CLI external-tool coupling not named in Dependencies
  **Location**: Requirements
  `accelerator-collaboration` shells to `gh`, a runtime availability precondition
  that isn't recorded in the Dependencies section.

- 🔵 **Clarity**: "Dependency-bleed rationale" and "typed-linkage" used without an
  in-document link
  **Location**: Context
  Both are terms of art from the epic's crate-split design, defined elsewhere
  (e.g., ADR-0034), which isn't listed in this item's References.

- 🔵 **Dependency**: Playwright executor runtime dependency not named in
  Dependencies
  **Location**: Requirements
  The design binary implies a Node/Playwright runtime precondition, pre-existing
  but unrecorded in Dependencies.

### Strengths

- ✅ The Dependencies section is unusually well-maintained: resolved blockers cite
  concrete evidence (e.g., "work-item:0187 ... merged via PR #42") rather than an
  unqualified "done", and the registration dependency is anchored to a specific,
  reusable checklist location rather than a vague ask.
- ✅ The three binary names and their constituent scripts/skills are stated
  identically across Summary, Requirements, and Acceptance Criteria, with no
  scope drift between sections, and each binary is given a clearly delineated
  boundary (specific source scripts, crates, skill directories).
- ✅ Acceptance Criteria are specific per sub-binary, naming concrete existing
  sub-commands and behaviours (ADR numbering/status, artifact metadata,
  frontmatter validation, linkage queries; inventory/gap tooling) rather than
  gesturing vaguely at "migrate the functionality".
- ✅ The removal/cleanup criterion is concretely verifiable — specific
  files/directories are named for deletion with an explicit suite-floor
  decrement requirement, giving a before/after comparison a verifier can run.
- ✅ Technical Notes names the exact source bash files being migrated, and the
  Drafting Notes transparently record the 2026-08-05 dependency-refresh pass and
  what changed, giving a clear audit trail.

### Recommended Changes

1. **Add an Acceptance Criterion for the skill call-site/`allowed-tools` rewrite**
   (addresses: "No acceptance criterion covers the skill call-site /
   `allowed-tools` rewrite requirement"). E.g., "All skills invoking the migrated
   scripts are repointed to the new `accelerator <subdomain>` sub-commands, with
   `allowed-tools` updated per the 0167 contract" — ideally naming the specific
   skills to check.

2. **Replace subjective Acceptance Criteria with observable outcomes**
   (addresses: "Launches correctly" and "Shells to gh as before" findings). State
   an exit code/artefact/parity check for the Playwright executor, and either
   enumerate the specific `gh` invocations each PR helper must issue or require
   passing characterization tests capturing the current `gh` call shape.

3. **Resolve the split-vs-single-story scope question before implementation**
   (addresses: both scope major findings and the "split-or-keep decision left
   open" minor). Either split into three stories (corpus, design, collaboration)
   mirroring the granularity used elsewhere in epic 0136, or re-kind as an epic
   with three child stories; commit to a rationale either way rather than
   leaving it as an implicit option.

4. **Add the missing Dependencies entries** (addresses: 0179/0174 findings, and
   the `gh`/Playwright external-tool suggestions). Name work-item:0179
   specifically alongside 0166, add a "Blocks: work-item:0174" entry, and
   optionally note the `gh` CLI and Playwright/Node runtime preconditions.

5. **Disambiguate the Q5/Q7 shorthand and the Playwright executor's status**
   (addresses: the three clarity minor findings). Either spell out what each
   resolved question decided inline every time it's referenced, or drop the
   labels in favour of stating the resolved rule directly; state the Playwright
   executor's open/closed status once, in one section, with the other
   cross-referencing it.

6. **Tighten the remaining testability gaps** (addresses: characterization
   coverage bar, registration-checklist criterion). Define a minimum
   characterization-test coverage expectation per sub-command, and add a
   criterion that each binary passes every item of the registration checklist.

## Per-Lens Results

### Clarity

**Summary**: 0173 is largely internally consistent — the three binaries named in
the Summary recur unchanged through Requirements and Acceptance Criteria, and the
Dependencies section is concrete about what has and hasn't resolved. The main
clarity gaps are shorthand references to decisions logged in an external document
("Q5", "Q7") that collide in name with this item's own Open Questions section, a
status word ("open") applied to the github→collaboration rename that sits
awkwardly next to a Requirements bullet that treats the rename as settled, and one
place where the Requirements text hedges between two outcomes that the Open
Questions section separately (and more explicitly) flags as undecided.

**Strengths**:
- The three binary names and their constituent script/skill groupings are stated
  identically in Summary, Requirements, and Acceptance Criteria, with no scope
  drift between sections.
- The Dependencies section names each prior blocker by ID, states its resolution
  explicitly (including a PR number), and gives a concrete pointer
  (tasks/README.md#registering-a-dispatched-sub-binary) for the registration
  step rather than a vague instruction.
- The Technical Notes section names the exact source bash files being migrated,
  leaving no ambiguity about what "the remaining script clusters" in the Summary
  actually refers to.

**Findings**:
- 🔵 minor/medium — "Q5"/"Q7" shorthand collides with the work item's own Open
  Questions section (Context, Requirements, Assumptions, Technical Notes)
- 🔵 minor/medium — "Open" github→collaboration rename reads as undecided, but
  Requirements state it as settled (Summary, Context, Requirements)
- 🔵 minor/medium — Playwright executor's fate is stated as an either/or in
  Requirements, then separately flagged as unresolved in Open Questions
  (Requirements, Open Questions)
- 🔵 suggestion/medium — "The relevant skills" is less specific than the named
  paths given in Acceptance Criteria (Requirements)
- 🔵 suggestion/low — "Dependency-bleed rationale" and "typed-linkage" used
  without an in-document link (Context)

### Completeness

**Summary**: The work item is structurally well-formed for a story: every
standard section is present and substantively populated, and the frontmatter is
fully specified with a recognised kind and status. The main gap is that one
Requirements bullet (the Q7 interface-redesign / skill call-site rewrite) has no
corresponding Acceptance Criterion, so "done" for that part of the work is left
undefined; a secondary observation is that the item's own Drafting Notes flag
that acceptance criteria, requirements, and kind have not yet been reviewed this
round.

**Strengths**:
- Every expected section for a story is present and substantively populated —
  no placeholder or empty sections.
- Acceptance Criteria are specific per sub-binary and include an explicit
  cleanup criterion (legacy script removal with suite-floor decrement).
- The Dependencies section is current and precise — it explicitly states which
  prior blockers are now resolved rather than leaving stale `blocked_by`
  references, and points to the concrete registration checklist to apply three
  times.

**Findings**:
- 🟡 major/high — No acceptance criterion covers the interface-redesign / skill
  call-site rewrite requirement (Acceptance Criteria)
- 🟡 minor/medium — Item's own notes flag acceptance criteria, requirements, and
  kind as not yet reviewed (Drafting Notes)
- 🔵 minor/low — No identification of the beneficiary whose need is met by this
  migration (Context)

### Dependency

**Summary**: The Dependencies section is unusually well-maintained for a draft
item — it names three specific resolved blockers with evidence (a merged PR
number) rather than vague status claims, and anchors the registration-surface
dependency to a concrete document location. The main gaps are precision issues:
References cite 0179 as the crate source but Dependencies names only the coarser
parent (0166); sibling item 0174 explicitly lists this story as a blocker but the
reverse Blocks entry is absent; and two runtime external-tool couplings (`gh`,
the Playwright executor) go unmentioned in Dependencies.

**Strengths**:
- The resolved-blocker list cites concrete evidence ("work-item:0187 merged via
  PR #42") rather than an unqualified "done".
- The sub-binary registration dependency is anchored to a specific, reusable
  location and explicitly notes it applies three times over.
- The Drafting Notes transparently record when this dependency refresh
  happened and what changed, giving a clear audit trail.

**Findings**:
- 🔵 minor/high — 0179 (the crate-delivering work item) is not named alongside
  its parent 0166 (Dependencies)
- 🔵 minor/high — Downstream consumer 0174 not named as a Blocks entry
  (Dependencies)
- 🔵 suggestion/medium — gh CLI external-tool coupling not named in Dependencies
  (Requirements)
- 🔵 suggestion/low — Playwright executor runtime dependency not named in
  Dependencies (Requirements)

### Scope

**Summary**: This story bundles three functionally independent migration efforts
— the accelerator-corpus, accelerator-design, and accelerator-collaboration
sub-binaries — each with its own source scripts, skill domain, registration
checklist, and test suite, with the item's own Context section acknowledging it
"may be split into three separate work items." The internal sections are
mutually consistent, but that consistency is achieved by uniformly describing
three parallel efforts rather than one coherent increment, and the declared
"story" kind understates the combined breadth of the work.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria are internally consistent with
  each other — all three consistently scope the work to the same three
  binaries.
- Each of the three binaries is given a clearly delineated boundary, which would
  make a future split into three work items straightforward if pursued.
- The Dependencies section explicitly surfaces that the registration checklist
  must be applied three times, showing the author is aware of the multiplicity.

**Findings**:
- 🟡 major/high — Three independent bounded contexts bundled into a single
  story (Summary, Requirements)
- 🟡 major/medium — Story kind likely undersized for the combined scope
  (Frontmatter: kind)
- 🔵 minor/medium — Split-or-keep decision left open without criteria (Context)

### Testability

**Summary**: The Acceptance Criteria are unusually well-grounded for a story —
they name concrete existing scripts, sub-commands, and a specific
removal/floor-decrement mechanism. However, two criteria fall back on subjective
or undefined baselines ("launches correctly", "as before"), a substantive
requirement (rewriting skills' call sites and allowed-tools) has no
corresponding criterion at all, and the characterization-test criterion has no
defined coverage bar.

**Strengths**:
- Acceptance Criteria for the corpus and design binaries enumerate the exact
  sub-commands/behaviours to reproduce rather than gesturing vaguely at
  "migrate the functionality".
- The removal criterion is concretely verifiable: specific files/directories
  are named for deletion and suite floors must be decremented in lockstep.
- Grounding verification in "repointed suites" gives a defined verification
  mechanism rather than relying on manual judgement of behavioural equivalence.

**Findings**:
- 🔴 major/high — "Launches correctly" has no defined pass/fail threshold
  (Acceptance Criteria)
- 🔴 major/high — "Shells to gh as before" relies on an undefined baseline
  (Acceptance Criteria)
- 🟡 major/medium — Skill call-site / allowed-tools rewrite has no corresponding
  Acceptance Criterion (Requirements)
- 🔵 minor/medium — "Characterization tests where none exist" has no defined
  coverage bar (Acceptance Criteria)
- 🔵 minor/medium — Registration-checklist compliance is stated as a dependency
  but not as a verifiable criterion (Dependencies)

---
*Review generated by /accelerator:review-work-item*
