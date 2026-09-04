---
type: "work-item-review"
id: "0258-help-show-subcommands-review-1"
title: "Work Item Review: Help Should Show Subcommands"
date: "2026-09-04T11:27:37+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0258"
work_item_id: "0258"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-05T11:07:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Help Should Show Subcommands

**Verdict:** REVISE

This bug is unusually well-formed — a self-contained reproduction with explicit
expected/actual outcomes, five concrete acceptance criteria, and proactively
carved boundaries against sibling items 0259 and 0260. Two structural issues
gate approval: it bundles a manifest-wide, cross-surface description rewrite
with the help-routing fix, and it leaves the exit-status behaviour for bare
`accelerator` unresolved as an Open Question, so that dimension of the criteria
has no definitive pass/fail. Neither is a defect in the analysis; both are
scoping decisions the item should settle before implementation.

### Cross-Cutting Themes

- **The manifest description rewrite is a second concern riding along** (flagged
  by: scope, dependency, testability) — scope calls it a separately deliverable
  cross-surface change bundled with the routing fix; dependency notes its shared
  manifest artefact (`BinaryEntry.description`) has unenumerated consumers;
  testability notes the positive "user-facing description" criterion has no
  enumerated expected text. All three orbit the same rewrite.
- **The 0259 boundary is coordinated but not sequenced** (flagged by: scope,
  dependency) — merging the two command groups and removing the heading is
  itself a layout change that abuts 0259's styling remit, and the item asks the
  two to "coordinate" without establishing which lands first.

### Findings

#### Major

- 🟡 **Scope**: Manifest-wide description rewrite bundled with help-routing bug fix
  **Location**: Requirements
  Beyond fixing the routing defect, the Requirements and Acceptance Criteria
  mandate replacing every command's launcher-internal phrasing with user-facing
  summaries. The Technical Notes confirm these live in manifest data, not help
  code, broadening the change and coupling two separately deliverable concerns.

- 🟡 **Testability**: Exit status for bare `accelerator` undefined, leaving that criterion unverifiable
  **Location**: Acceptance Criteria
  The Open Questions section asks whether bare `accelerator` keeps clap's
  non-zero exit or exits zero, but no criterion resolves it — a verifier running
  the bare command cannot decide whether a given exit code is a pass or a fail.

#### Minor

- 🔵 **Clarity**: "Complete intended set" of nine may conflict with dependency 0260
  **Location**: Assumptions
  The Assumptions section fixes the set at nine and says the fix "adds none and
  hides none", yet the Dependencies section notes 0260 changes the command
  surface. A reader cannot tell whether "the complete command set" is fixed at
  nine or expected to grow.

- 🔵 **Dependency**: 0259 coupling names no landing order
  **Location**: Dependencies
  The 0259 entry correctly identifies the shared help output but only asks the
  two items to "coordinate", without establishing which lands first — a
  concurrent shared-artefact coupling with no sequencing resolved, risking merge
  conflicts or silent reverts discovered at integration.

- 🔵 **Scope**: Heading-removal / list-merge overlaps the styling boundary owned by 0259
  **Location**: Dependencies
  Merging built-ins and sub-binaries into one list with no heading is a
  presentation decision, yet the item states 0259 "governs help styling only".
  Removing a heading and collapsing two groups is itself a layout change, so the
  boundary is not cleanly separable.

#### Suggestions

- 🔵 **Clarity**: ADR acronym used without expansion
  **Location**: Dependencies
  "0260 (Move ADR Into Its Own Subcommand)" uses ADR without expanding it to
  Architecture Decision Record.

- 🔵 **Dependency**: Other consumers of `BinaryEntry.description` not enumerated
  **Location**: Technical Notes
  The rewrite edits shared manifest data, but the item does not name which other
  renderers (the existing `--help` section, the visualiser) consume those
  strings, leaving the blast radius uncaptured.

- 🔵 **Dependency**: Frontmatter `relates_to` omits the "built on" items 0164 and 0187
  **Location**: Frontmatter: relates_to
  The Dependencies prose names 0164 and 0187 as items this bug is built on, but
  `relates_to` lists only 0259 and 0260, so the stronger couplings are invisible
  to tooling that reads structured frontmatter.

- 🔵 **Testability**: Positive "user-facing description" check has no enumerated expected text
  **Location**: Acceptance Criteria
  Criterion 4 is verifiable in its negative form (absence of "… sub-binary."),
  but the positive expectation gives only one worked example, so each
  replacement description's correctness is a subjective judgement.

### Strengths

- ✅ The reproduction is complete and executable: three named commands to run
  and compare, with explicit expected and actual states — exactly what a bug
  verification needs.
- ✅ All expected sections are present and substantively populated; frontmatter
  (`kind: bug`, `status: draft`, `priority: medium`) is intact and consistent
  with the body headers.
- ✅ Scope boundaries are stated explicitly — the Assumptions section defines
  what is in and out ("surfaces exactly what dispatch already knows about") and
  resolves the "merged list" ambiguity.
- ✅ Dependencies are mapped with substance: upstream mechanism items (0164,
  0187) are named and marked done, and the couplings to 0259/0260 explain their
  nature rather than being bare references.
- ✅ Criterion 5 (a fixture sub-binary appearing in all three outputs with no
  change to help code) makes the source-of-truth requirement directly testable
  rather than relying on inspection.

### Recommended Changes

1. **Decide the scope of the description rewrite** (addresses: Manifest-wide
   description rewrite bundled; Other consumers of `BinaryEntry.description` not
   enumerated; Positive "user-facing description" check has no enumerated text)
   Either split the manifest description rewrite into its own chore (or fold it
   into 0259), leaving this bug focused on rendering the identical existing
   command set across three entry points — or, if it stays, enumerate the
   expected one-line description for each of the nine sub-binaries and name every
   surface that consumes `BinaryEntry.description`.

2. **Resolve the bare-invocation exit status and pin it in a criterion**
   (addresses: Exit status for bare `accelerator` undefined) Settle the Open
   Question and add a criterion asserting the expected exit code, e.g. "bare
   `accelerator` prints the full command list and exits with status 0".

3. **Sharpen the 0258/0259 boundary and sequence it** (addresses: Heading-removal
   overlaps 0259; 0259 coupling names no landing order) Decide whether the
   heading/merge layout change lives here or in 0259, then state an explicit
   landing order (or that both must land as one coordinated change) and adjust
   0259's scope note to match.

4. **Reconcile the "nine" assumption with the dynamic source of truth**
   (addresses: "Complete intended set" of nine may conflict with 0260) Clarify in
   Assumptions that "complete" means whatever dispatch currently knows about
   (currently nine), so the set reads as dynamic rather than hardcoded.

5. **Mirror the built-on couplings into frontmatter** (addresses: `relates_to`
   omits 0164 and 0187) Add `work-item:0164` and `work-item:0187` to `relates_to`
   so the mechanism dependencies the body already states are machine-visible.

## Per-Lens Results

### Clarity

**Summary**: Unusually clear and internally coherent — Summary, Context,
Requirements, and Acceptance Criteria describe a single consistent intent, with
consistent counts and command names and unambiguous referents. The only concerns
are a soft tension between the fixed "nine sub-binaries" assumption and a
dependency that changes the command surface, and one undefined acronym.

**Strengths**:
- The Summary enumerates the exact nine sub-binaries and four built-ins, and
  Assumptions restates the count of nine, keeping the command set unambiguous.
- Requirements and Acceptance Criteria state outcomes as observable listing
  states rather than vague desired properties.
- The "merge into one list" ambiguity is explicitly resolved in Assumptions and
  AC #3 ("no heading separating built-ins from sub-binaries").
- Pronouns and noun phrases each resolve to a single clearly named antecedent.

**Findings**:
- minor (confidence: medium), Assumptions — "Complete intended set" of nine may
  conflict with dependency 0260. Assumptions says the fix adds/hides none, yet
  0260 changes the command surface; a reader cannot tell whether "complete" is
  fixed at nine or dynamic. Suggest clarifying "complete" means whatever dispatch
  currently knows about.
- suggestion (confidence: medium), Dependencies — ADR acronym used without
  expansion. Expand ADR (Architecture Decision Record) on first use, or rely on
  the linked 0260 title.

### Completeness

**Summary**: Structurally and informationally complete. All expected sections are
present and substantively populated, and it carries the kind-specific content a
bug demands — a reproduction procedure with explicit expected and actual
outcomes. Frontmatter is intact with a recognised kind, status, and priority.

**Strengths**:
- Reproduction is complete and self-contained: exact commands, expected outcome,
  and actual outcome.
- Context explains why the defect exists (deliberate divergence traced to 0164)
  and why it matters, rather than restating the Summary.
- Acceptance Criteria contains five specific, distinct done-conditions covering
  all three invocation paths plus the fixture-registration regression guard.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes)
  are all populated with genuinely relevant content.

**Findings**: none.

### Dependency

**Summary**: Dependency mapping is strong — both upstream mechanism items (0164,
0187) are named and done, and the two concurrent related items (0259, 0260) are
captured with explicit notes describing each coupling. No external systems or
cross-team actions are implied, so an empty external surface is appropriate. The
residual concerns are minor: unresolved sequencing against 0259, a shared
manifest artefact whose other consumers aren't enumerated, and a `relates_to`
list that omits the built-on items.

**Strengths**:
- Upstream mechanism dependencies (0164, 0187) are named and flagged done, so no
  hidden blocker gates the work from starting.
- The 0259/0260 relationships explain the coupling rather than being bare
  references.
- The "same source of truth as dispatch" requirement structurally decouples the
  fix from future sub-binary additions.

**Findings**:
- minor (confidence: medium), Dependencies — 0259 is a concurrent shared-artefact
  coupling with no sequencing resolved; risk of merge conflicts or silent
  reverts at integration. Suggest stating an explicit landing order or a single
  coordinated change.
- suggestion (confidence: low), Technical Notes — other consumers of
  `BinaryEntry.description` (the `--help` section, the visualiser) are not
  enumerated, leaving the rewrite's blast radius uncaptured.
- suggestion (confidence: low), Frontmatter: relates_to — prose names 0164 and
  0187 as built-on items, but `relates_to` lists only 0259 and 0260, so the
  stronger couplings are invisible to tooling.

### Scope

**Summary**: One broadly coherent goal — making all three help entry points
render the same complete, consistent command index. The core defect is
atomically sized and appropriate for a bug. However, the Requirements bundle a
manifest-wide description rewrite that edits a different surface with standalone
value, and the "merge into one list / no heading" presentation change sits on a
blurry boundary with sibling item 0259.

**Strengths**:
- Scope boundaries are stated explicitly in Assumptions, giving a clear
  inclusion/exclusion line.
- Dependencies proactively carve the boundary against 0259 (styling) and 0260
  (command surface) rather than silently overlapping.
- The primary defect is a single, well-bounded unit of value one team can own
  and verify end-to-end.

**Findings**:
- major (confidence: medium), Requirements — manifest-wide description rewrite
  bundled with the help-routing bug fix. The rewrite lives in manifest data, has
  standalone value, and could ship or roll back independently; bundling enlarges
  blast radius and hurts reviewability. Suggest splitting it into its own chore
  or folding into 0259.
- minor (confidence: medium), Dependencies — heading-removal / list-merge is a
  layout change that overlaps 0259's stated styling remit, so the boundary is not
  cleanly separable. Suggest scoping this bug to correctness only or explicitly
  claiming layout ownership and adjusting 0259.

### Testability

**Summary**: Strongly testable — the reproduction is fully specified (three named
commands to compare), expected vs. actual outcomes are stated, and the criteria
include a fixture-based regression test that verifies the source-of-truth
mechanism. The main gap is the exit-status behaviour for bare `accelerator`,
left as an unresolved Open Question, plus a positive "user-facing description"
criterion that lacks enumerated expected text.

**Strengths**:
- Requirements give a complete, executable reproduction with explicit Expected
  and Actual states.
- AC #3 defines an unambiguous, observable pass condition a diff-based check can
  confirm conclusively.
- AC #5 (fixture sub-binary appearing in all three outputs with no help-code
  change) makes the source-of-truth requirement directly testable and bounds the
  "every sub-binary" scope via the named nine-binary set.

**Findings**:
- major (confidence: medium), Acceptance Criteria — exit status for bare
  `accelerator` is undefined; the Open Question is never resolved into a
  criterion, so two implementations with opposite exit codes could both be argued
  as passing. Suggest resolving it and adding a criterion pinning the exit code.
- minor (confidence: medium), Acceptance Criteria — the positive "user-facing
  description" expectation gives only one worked example and no expected text for
  the other eight sub-binaries, making each description a subjective judgement.
  Suggest enumerating the expected string per sub-binary.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-04

**Verdict:** COMMENT

Re-ran the four lenses that carried findings (clarity, dependency, scope,
testability). Both original major findings are resolved and no major findings
remain, so the verdict moves from REVISE to COMMENT — the work item is
acceptable for implementation. The edits cleared the structural issues; a
cascade of finer-grained minor observations surfaced once they did, none
blocking.

### Previously Identified Issues

- 🟡 **Testability**: Exit status for bare `accelerator` undefined — **Resolved**
  (AC now pins the retained non-zero exit; Open Question recorded as resolved).
- 🟡 **Scope**: Manifest-wide description rewrite bundled — **Partially resolved**
  (kept here by decision and enumerated; scope now rates it minor, wanting the
  atomic-landing justification written down).
- 🔵 **Clarity**: "Complete intended set" of nine may conflict with 0260 —
  **Resolved** (Assumptions reframes the set as dynamic).
- 🔵 **Dependency**: 0259 coupling names no landing order — **Resolved**
  (explicit sequencing: 0258 first, then 0259 restyles).
- 🔵 **Scope**: Heading-removal overlaps 0259 boundary — **Partially resolved**
  (layout ownership now claimed here; scope still wants a concrete dividing line
  and confirmation 0259's scope note is amended).
- 🔵 **Testability**: Positive description check had no enumerated text —
  **Resolved** (nine descriptions enumerated; AC asserts exact match).
- 🔵 **Clarity**: ADR acronym unexpanded — **Resolved** (expanded inline).
- 🔵 **Dependency**: Other consumers of the descriptions not enumerated —
  **Partially resolved** (consumer note added; the Cargo.toml package-metadata
  consumer is still unnamed — see new issue).
- 🔵 **Dependency**: `relates_to` omitted 0164/0187 — **Resolved** (both added).

### New Issues Introduced

- 🔵 **Clarity**: Summary frames `--help` as the correct reference, but AC 4/5
  require restructuring `--help` too (heading removed, descriptions rewritten).
- 🔵 **Clarity**: Dependencies and Technical Notes locate the canonical
  description source differently (crate `Cargo.toml` vs manifest data).
- 🔵 **Dependency**: `Cargo.toml` `description` is dual-use crate package
  metadata, not solely a help string; the consumer check omits cargo/packaging.
- 🔵 **Testability**: "Identical command listings" leaves comparison dimensions
  (ordering, whitespace) undefined.
- 🔵 **Testability**: No expected description text for the four built-in commands.
- 🔵 **Testability**: AC 6 folds the unassertable "no change to help code"
  constraint into an otherwise observable check.
- 🔵 **Scope** (suggestion): `bug` kind under-represents the layout-restructure
  and nine-crate description rewrite.
- 🔵 **Dependency** (suggestion): 0260's new `adr` subcommand will need a
  user-facing description the enumerated nine do not cover.

### Assessment

The work item is ready for implementation. Every issue that gated approval is
resolved; the residual items are minor precision improvements — chiefly pinning
down what "identical" asserts, naming `Cargo.toml` `description` as dual-use
metadata, and reconciling the Summary's "already lists / nine" framing with the
restructure and dynamic-set intent. Worth a light polish pass but not required
before planning.

### Approval (2026-09-05)

**Verdict:** APPROVE

Reviewer accepts the work item. After pass 2, the two recommended edits were
applied — the Summary now states `--help` is also restructured, and the
consumer note names `Cargo.toml` `description` as dual-use package metadata.
Both re-review new-issue findings on those points are resolved; the remaining
minor observations are optional polish and do not gate planning. The verdict is
promoted from COMMENT to APPROVE.
