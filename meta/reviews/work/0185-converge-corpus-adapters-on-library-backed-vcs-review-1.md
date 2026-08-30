---
type: "work-item-review"
id: "0185-converge-corpus-adapters-on-library-backed-vcs-review-1"
title: "Work Item Review: 0185: Converge corpus-adapters on the Library-Backed VCS Adapter"
date: "2026-08-10T00:55:56+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0185"
work_item_id: "0185"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-10T08:20:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: 0185: Converge corpus-adapters on the Library-Backed VCS Adapter

**Verdict:** REVISE

The item is a well-scoped, well-anchored task — Requirements and Acceptance
Criteria describe the same single unit of work one-to-one, file:line
references are concrete throughout, and the 2026-08-03 amendment does real
work surfacing couplings a lighter edit would have missed. The core problem
is that the amendment corrects several claims made earlier in the document
(which item delivers the adapter, whether the switch is "invisible to
callers", which file `CommandProbe` lives in, whether the item is still
blocked) without those earlier sections being edited in place, so the
document is internally contradictory unless read end-to-end and manually
reconciled. Two further consequences the item itself treats as
switch-triggered — an MPL-2.0 licence-attribution obligation and an
unhardened new coupling into the distributed visualiser binary — are
documented only as prose in Open Questions/Amendment, not as Dependencies
entries or Acceptance Criteria, so a verifier working strictly from the
Dependencies/AC sections could miss them entirely.

### Cross-Cutting Themes

- **Amendment corrections were appended, not applied in place** (flagged by:
  clarity, completeness) — the Summary/Context (adapter attributed to 0169
  instead of 0188), Requirements ("invisible to callers"), Technical Notes
  (stale `lib.rs` line references and a stale "0169 will need to alter this
  anyway" premise), References (a stale anchor into 0169), and frontmatter
  (`blocked_by: ["work-item:0188"]` despite 0188 having landed) all retain
  claims the item's own Amendment block says are wrong or stale. A reader who
  trusts the first read of any of these sections walks away with an
  incorrect picture, even though the correct information is present
  somewhere later in the same document.
- **The MPL-2.0 licence-attribution consequence is under-captured** (flagged
  by: testability, dependency, scope) — the Amendment states in directive
  language that the licence exception "has to be re-checked when `facts`
  flips" and that this item's switch is "the expected trigger" invalidating
  it, but this lives only in Open-Questions prose. It appears in no
  Acceptance Criterion (testability) and no Dependencies entry (dependency),
  and is scope-orthogonal work (Python release tooling, not the Rust VCS
  crates this task otherwise touches) that risks being resolved hastily
  in-line (scope).
- **The new, unhardened coupling into `cli/visualiser/server` is
  under-captured** (flagged by: scope, dependency) — after this switch,
  `InProcessProbe` parses repository-controlled data with no timeout, memory
  bound, or crash isolation inside a distributed, network-facing binary for
  the first time. This is treated as an open question about sizing but never
  surfaces as a Related entry to the item that owns the visualiser's
  integration boundary.
- **sha256-repository handling is unresolved and may already be decided
  elsewhere** (flagged by: testability, dependency) — the item states this
  switch is what "first exposes a user" to gix's inability to read sha256
  repositories, but 0169 (now done) recorded the identical finding about its
  own shipped hook paths, so the decision may need to be made once, in a
  shared location, rather than re-litigated independently per consumer.

### Findings

#### Major

- 🟡 **Clarity / Completeness**: Requirements' "invisible to callers"
  guarantee contradicts the Amendment's documented behavioural difference
  **Location**: Requirements; Amendment 2026-08-03, inheritance 3
  Requirements states the goal is to preserve `vcs_adapters::facts`'s
  semantics "so the change is invisible to callers," but Amendment
  inheritance 3 documents a known caller-visible difference: after the
  switch, deriving metadata stops having a write side effect on the user's
  repository, and a `RepoFacts.revision` taken with unsnapshotted edits
  present will name a different commit than today. The Requirements bullet
  is never revised to acknowledge this exception.

- 🟡 **Clarity**: Summary and Context attribute the library-backed adapter to
  0169, contradicting Dependencies, frontmatter, and the item's own later
  Amendment
  **Location**: Summary; Context; Dependencies; Frontmatter: blocked_by
  The Summary and Context both credit 0169 with introducing the
  `gix`/`jj-lib` adapter, but the frontmatter's `blocked_by`, the
  Dependencies section, and the Amendment's opening line ("Every reference
  in this item that attributes the library-backed adapter or the zero-spawn
  harness to 0169 is wrong. Both are 0188's.") all say otherwise. The
  correction is a disclaimer appended at the bottom rather than an edit to
  the Summary/Context sentences themselves.

- 🟡 **Scope**: Unresolved open questions could expand this "task" well
  beyond a wiring-plus-deletion change
  **Location**: Open Questions; Amendment 2026-08-03, inheritances 2 and 5
  The item is sized as a `task` — "a wiring change plus a deletion... with
  no new behaviour and no user-visible change" — but its own Open Questions
  describe decisions that, resolved one way, add substantial new work:
  whether `InProcessProbe` needs an equivalent containment bound before it
  runs inside `cli/visualiser/server` and the hook path, and whether the
  MPL-2.0 exception this switch invalidates requires a new third-party
  attribution artefact in the release pipeline. Neither is reflected in
  Requirements or Acceptance Criteria.

- 🟡 **Testability**: Licence re-check described as mandatory but not
  captured as an Acceptance Criterion
  **Location**: Amendment 2026-08-03, inheritance 5
  The amendment instructs to "re-run 0188's check... as part of the switch,
  not after it," but no Acceptance Criterion requires this check or records
  what a pass/fail outcome looks like. A verifier working strictly from the
  five listed ACs could mark the item complete without ever performing the
  licence re-check.

- 🟡 **Testability**: sha256 repository handling has no defined, verifiable
  outcome
  **Location**: Open Questions; Amendment 2026-08-03, inheritance 4
  The item says revision validation "must accept both widths or record
  sha256 as explicitly unsupported," and that this item's switch is what
  exposes the gap, but no Acceptance Criterion states which outcome is
  required. There is no way to write a test that conclusively confirms this
  is "done correctly."

- 🟡 **Dependency**: MPL-2.0 licence/attribution follow-up not captured as a
  Dependency
  **Location**: Open Questions; Amendment 2026-08-03, inheritance 5
  A licence-compliance action with release-pipeline implications (a new
  attribution artefact joining `_release_uploads()`, whose CI coverage
  derives from `test_workflows.py`) lives only as a paragraph inside an
  Open Questions amendment, not in the Dependencies section where planners
  look for required actions.

- 🟡 **Dependency**: New coupling to the `cli/visualiser/server` runtime not
  referenced in Dependencies
  **Location**: Open Questions
  After this switch, `InProcessProbe` parses repository-controlled data with
  no isolation inside a distributed, publicly-shipped binary for the first
  time — a substantive new coupling to a subsystem owned elsewhere in the
  epic (0168), but visible only by reading Open Questions prose rather than
  as a Related entry.

#### Minor

- 🔵 **Clarity**: "Four inheritances" header is followed by five numbered
  items
  **Location**: Amendment 2026-08-03
  The numbered list is introduced as "Four inheritances that change this
  item's sizing" but five items follow.

- 🔵 **Clarity**: File:line references for `CommandProbe` are likely stale
  after the amendment's noted module move
  **Location**: Requirements; Technical Notes; Amendment 2026-08-03,
  inheritance 1
  Requirements and Technical Notes cite `lib.rs` line numbers for
  `CommandProbe`, but inheritance 1 states the subprocess pair now lives in
  its own module, `subprocess.rs` — the original references are not updated
  or flagged as stale.

- 🔵 **Clarity**: "0169's own classifier port" is an undefined term not tied
  to the ports named elsewhere in the document
  **Location**: Amendment 2026-08-03
  The document elsewhere names `RepoRoot` and `VcsProbe` as the relevant
  ports, but "classifier port" is introduced without being reconciled
  against either.

- 🔵 **Completeness**: Technical Notes retains a premise the Amendment marks
  stale
  **Location**: Technical Notes
  Technical Notes states "0169 will need to alter this anyway," which the
  Amendment separately calls stale — the original bullet is left unedited
  alongside the correction.

- 🔵 **Completeness**: Frontmatter still shows draft/blocked despite the
  recorded blocker having landed
  **Location**: Frontmatter: status; blocked_by
  Frontmatter records `status: draft` and `blocked_by: ["work-item:0188"]`,
  but the Amendment states plainly that "0188 has landed" and the adapter
  "exists and ships unwired."

- 🔵 **Dependency**: Ordering relative to sibling Phase 11 item 0174 not
  stated
  **Location**: Dependencies
  The parent epic groups this item with 0174 ("Retire Shell Tooling and CI
  Guards") under Phase 11, sequenced first, but 0185 does not state whether
  0174 depends on this item's completion.

- 🔵 **Dependency**: sha256-repository handling is framed as this item's own
  open question despite overlapping claims in 0169
  **Location**: Open Questions
  0169 (now done) records the identical sha256 finding about its own
  shipped hook paths; treating the decision as local to each item risks
  inconsistent handling across `vcs detect`/`guard`, this item's metadata
  read, and future `vcs status`/`log` work.

- 🔵 **Testability**: Containment-bound and snapshot-side-effect questions
  left open with no criterion for how or whether they gate completion
  **Location**: Open Questions
  Both questions use language suggesting they must be resolved before the
  switch ships, but neither is reflected as an Acceptance Criterion or given
  a defined resolution procedure.

- 🔵 **Testability**: Ambiguity over whether AC5 ("`mise run` is green")
  covers the `check-zero-spawn` CI job referenced in the Amendment
  **Location**: Acceptance Criteria
  The Amendment describes a broader "strong form" assertion running in the
  `check-zero-spawn` CI job; it is not stated whether that job is part of
  the default `mise run` AC5 references.

- 🔵 **Scope**: Licence-attribution follow-up is an orthogonal concern
  spanning a different toolchain than the rest of this task
  **Location**: Open Questions; Amendment 2026-08-03, inheritance 5
  If triggered, the attribution-artefact work touches Python build-system
  tooling (`_release_uploads()`, `test_workflows.py`), not the Rust
  `vcs-adapters`/`corpus-adapters` crates the rest of this task is scoped
  to.

#### Suggestions

- 🔵 **Clarity**: "the corpus writers" is used without a defined referent
  **Location**: Open Questions; Amendment 2026-08-03, inheritance 3
  Neither the Open Questions section nor the Amendment names the specific
  module or call sites meant by "the corpus writers."

- 🔵 **Clarity**: "§3.2's notice obligation" does not name which document it
  refers to
  **Location**: Amendment 2026-08-03, inheritance 5
  Inferable from context as the MPL-2.0 licence, but never spelled out.

- 🔵 **Completeness**: References entry points to anchors the Amendment
  itself flags as no longer existing
  **Location**: References
  The first References bullet cites headings in 0169 that the Amendment
  itself says are stale ("0169 has no 'Adapter-swap boundary' heading...").

### Strengths

- ✅ Requirements and Acceptance Criteria describe the same scope
  one-to-one, with no drift between what the item asks for and how it will
  be verified.
- ✅ File:line anchors throughout Requirements and Technical Notes ground
  abstract claims in specific, checkable locations.
- ✅ Acceptance Criteria are concrete and multi-dimensional — grep-able
  invariants, named test suites, and pinned boundary cases rather than
  vague behavioural claims.
- ✅ The adapter switch and `CommandProbe` deletion are explicitly framed as
  "one atomic change" with a stated rationale, which is good scope
  discipline rather than arbitrary bundling.
- ✅ A closely related convergence effort (0125) is explicitly named and
  deliberately kept out of scope, with reasoning given.
- ✅ The Dependencies section states not just that 0188 blocks the work but
  why, and documents the repointing history when 0169 was split.
- ✅ The Amendment block is transparent about what it corrects, rather than
  silently rewriting earlier sections — the underlying problem is that the
  corrections were not also applied to those sections in place.

### Recommended Changes

1. **Edit the Summary, Context, and Requirements sections in place** to
   reflect the Amendment's corrections (addresses: Summary/Context 0169
   misattribution, Requirements "invisible to callers" contradiction)
   rather than relying on a disclaimer appended at the end of the document.
   Qualify the "invisible to callers" claim to the boundaries `detection.rs`
   actually pins, and name the known snapshot-on-read exception inline.

2. **Promote the MPL-2.0 licence re-check to a Dependency entry and an
   Acceptance Criterion** (addresses: MPL-2.0 licence/attribution follow-up
   not captured as a Dependency; Licence re-check not captured as an
   Acceptance Criterion; Licence-attribution follow-up is orthogonal). State
   explicitly what a pass/fail outcome looks like, and consider whether the
   attribution-artefact work (if triggered) should be split into its own
   follow-up item given it spans a different toolchain.

3. **Resolve or explicitly scope the sha256-handling decision**, checking
   first whether 0169 already decided how sha256 `HEAD` values are handled
   for its own shipped paths (addresses: sha256 handling has no defined
   outcome; sha256 framed as this item's own open question). If 0169's
   decision exists, reference it rather than re-opening the question; if
   not, record the decision in a shared location referenced by all
   consumers.

4. **Add a Related entry to the visualiser-owning work item** noting the new
   call-graph reachability and the containment-bound decision that the
   switch introduces (addresses: new coupling to `cli/visualiser/server`
   not referenced in Dependencies; unresolved open questions could expand
   this task).

5. **Refresh stale frontmatter and cross-references**: clear or annotate
   `blocked_by: ["work-item:0188"]` now that 0188 has landed, update the
   `CommandProbe` file:line references to point at `subprocess.rs`, fix the
   "Four inheritances" count, reconcile the "0169 will need to alter this
   anyway" bullet in Technical Notes, and correct the stale References
   anchor into 0169 (addresses: all remaining minor/suggestion findings).

## Per-Lens Results

### Clarity

**Summary**: The work item is detailed and mostly precise, with concrete
file:line references and an explicit Assumptions/Open Questions structure.
However, the 2026-08-03 amendment block corrects several claims made
earlier in the document without editing the original text, leaving the
document internally contradictory unless read end-to-end and manually
reconciled. A handful of terms introduced only in the amendment
("classifier port", "inheritance N", "corpus writers") are used without
being tied back to concepts already defined in the body, and a numeric
label mismatch ("Four inheritances" followed by five numbered items) is a
small but concrete internal-consistency slip.

**Strengths**:
- File:line anchors throughout Technical Notes and Requirements ground
  abstract claims in a specific, checkable location.
- The amendment block explicitly flags that earlier attributions to 0169
  are wrong rather than silently leaving them stale.
- Assumptions and Open Questions are phrased as falsifiable, specific
  claims rather than vague hedges.

**Findings**: See Major/Minor/Suggestions above (Clarity-attributed items).

### Completeness

**Summary**: 0185 is a structurally complete task-kind work item: every
expected section is present and substantively populated, Requirements and
Technical Notes are anchored to specific file:line references, and the
Amendment block transparently reconciles what changed once 0188 landed.
The main completeness weakness is that the amendment corrects several
claims made in the original Requirements, Technical Notes, and References
sections without those sections themselves being updated, and frontmatter
has not been refreshed to reflect that its sole recorded blocker has
landed.

**Strengths**:
- Requirements and Technical Notes are anchored to specific file:line
  references, making the work concretely actionable.
- The Context section grounds the deliberate two-implementation boundary in
  a specific, dated prior review decision.
- Acceptance Criteria are specific and multi-dimensional.
- The Open Questions are substantive and each tied to a concrete
  pre-implementation decision.
- The Amendment block explicitly separates corrections from new
  inheritances rather than silently rewriting history.

**Findings**: See Major/Minor/Suggestions above (Completeness-attributed
items).

### Dependency

**Summary**: The Dependencies section captures the primary upstream
blocker (0188) and two related items (0179, 0125) with clear rationale,
and the Amendment block does real work surfacing hidden couplings that a
lighter edit would have missed. However, two of the item's own Open
Questions describe couplings that belong in Dependencies rather than
buried in prose: the MPL-2.0 licence/attribution trigger, and the fact
that this switch is what first links `vcs-adapters` into the distributed
`accelerator-visualiser` binary's call graph.

**Strengths**:
- The Dependencies section states not just that 0188 blocks this work but
  why, and documents the repointing history when 0169 was split.
- The Related entries for 0179 and 0125 each carry a one-line rationale for
  why the relationship matters.
- The Amendment block proactively surfaces several couplings a less
  careful revision would have left implicit.

**Findings**: See Major/Minor above (Dependency-attributed items).

### Scope

**Summary**: This task is tightly and deliberately scoped: the Summary,
Requirements, and Acceptance Criteria all describe one unit of work, and
the item explicitly and correctly excludes an adjacent convergence effort
(0125) rather than bundling it in. The main scope risk is not bundling
within the stated Requirements, but four unresolved Open
Questions/inheritances that could materially expand the work beyond
"wiring plus deletion" without being reflected in the Requirements or
Acceptance Criteria.

**Strengths**:
- Requirements and Acceptance Criteria describe the same scope
  one-to-one.
- The adapter switch and `CommandProbe` deletion are explicitly framed as
  "one atomic change" with a stated rationale.
- 0125 is explicitly named and deliberately kept out of scope, with
  reasoning given.
- The Drafting Notes give an explicit, checkable rationale for the
  declared `task` kind.

**Findings**: See Major/Minor above (Scope-attributed items).

### Testability

**Summary**: The Acceptance Criteria are unusually well-specified for a
task-kind item — each is grounded in exact file:line references, named
test suites, or grep-able invariants, giving a verifier a concrete
pass/fail procedure. The main gap is that several decisions the Amendment
frames in directive language (sha256 handling, the MPL licence re-check,
the containment bound, the snapshot-on-read dependency) are left as Open
Questions rather than being converted into measurable Acceptance Criteria.

**Strengths**:
- Acceptance Criteria are anchored to exact locations and named test
  files.
- AC1 ("no `Command::new` for `jj` or `git` remains in the crate's
  non-test code") is mechanically checkable by grep with no interpretation
  required.
- AC4 enumerates exact boundary cases to preserve by reference to an
  existing pinned test file.
- The Requirements section explicitly bounds the surface to preserve
  (`.name` and `.revision` only), preventing scope creep.

**Findings**: See Major/Minor above (Testability-attributed items).

## Re-Review (Pass 2) — 2026-08-10

**Verdict:** APPROVE (overridden by reviewer from the suggested COMMENT
verdict — the one remaining minor item, the MPL-2.0 attribution-artefact
toolchain concern, was judged acceptable to track in-line rather than a
blocker)

### Previously Identified Issues

- 🟡 **Clarity/Completeness**: Requirements' "invisible to callers" guarantee
  contradicted the Amendment's documented behavioural difference — Resolved
  (Requirements now qualifies the claim to `detection.rs`'s pinned boundaries
  and names the snapshot-on-read exception inline)
- 🟡 **Clarity**: Summary/Context attributed the adapter to 0169 instead of
  0188 — Resolved (both sections edited in place)
- 🟡 **Scope**: Unresolved open questions could expand this task beyond
  wiring-plus-deletion — Partially resolved (the containment-bound and
  licence decisions are now gated by explicit Acceptance Criteria and
  Dependencies entries rather than left as free-floating prose, but the
  decisions themselves remain open — see New Issues below)
- 🟡 **Testability**: Licence re-check not captured as an Acceptance
  Criterion — Resolved
- 🟡 **Testability**: sha256 handling had no defined, verifiable outcome —
  Resolved (AC added; decision now scoped to a shared, referenceable
  location rather than resolved silently inline)
- 🟡 **Dependency**: MPL-2.0 licence/attribution follow-up not captured as a
  Dependency — Resolved
- 🟡 **Dependency**: New coupling to `cli/visualiser/server` not referenced
  in Dependencies — Resolved (0168 added to Dependencies and `relates_to`)
- 🔵 **Clarity**: "Four inheritances" header followed by five items —
  Resolved
- 🔵 **Clarity**: Stale `CommandProbe` file:line references — Resolved
- 🔵 **Clarity**: Undefined "classifier port" term — Resolved (renamed to
  `VcsProbe`)
- 🔵 **Completeness**: Technical Notes retained a stale premise — Resolved
- 🔵 **Completeness**: Frontmatter `blocked_by`/`status` stale — Partially
  resolved (`blocked_by` corrected; `status` intentionally left as `draft`
  per the reviewer's constraint not to change status during review)
- 🔵 **Dependency**: Ordering relative to 0174 not stated — Resolved
- 🔵 **Dependency**: sha256 framed as a local decision despite 0169 overlap —
  Resolved
- 🔵 **Testability**: Containment-bound/snapshot questions ungated — Resolved
  (both promoted to Acceptance Criteria)
- 🔵 **Testability**: AC5/`check-zero-spawn` ambiguity — Resolved
- 🔵 **Scope**: Licence-attribution follow-up spans a different toolchain —
  Still present (accepted as an in-line Acceptance Criterion rather than
  split into a separate item; see New Issues)
- 🔵 **Clarity/Completeness**: "corpus writers"/"§3.2" undefined referents;
  stale References anchor — Resolved

### New Issues Introduced

The edits addressing the pass-1 findings introduced their own inconsistencies,
caught by this pass and fixed in the same edit session before this write:

- 🟡 **Clarity**: the new `Blocks: 0174` bullet directly contradicted the
  pre-existing Drafting Notes claim that this item "should not block" epic
  0136's shell-retirement work — Fixed (Dependencies reworded from "Blocks"
  to a hedged "Related" entry; Drafting Notes clarified that "should not
  block" is a priority claim, not a sequencing one)
- 🟡 **Clarity**: the Amendment block's corrections were partially stale
  relative to the body they described, since several had already been
  folded into the main sections — Fixed (added an editorial note marking
  which inheritances are fully applied vs. still open)
- 🟡 **Scope**: the new sha256 Acceptance Criterion required resolving the
  policy locally, directly contradicting the Open Question's instruction to
  decide it "in a location both consumers can reference... not... locally" —
  Fixed (both the AC and Open Question now agree: this item decides the
  policy and records it in the `vcs` crate's port-contract documentation,
  since 0169 — the only other consumer — is closed)
- 🟡 **Dependency**: `blocked_by` was cleared to `[]` on the assumption that
  a landed blocker should be removed, contradicting the repo's own
  convention (confirmed against 0169, which retains completed blockers) —
  Fixed (restored to `["work-item:0188"]`)
- 🔵 **Completeness/Dependency**: `relates_to` frontmatter was not updated
  to match the new 0168 Dependencies entry — Fixed (added, along with 0198)

One new substantive finding, not introduced by this pass's edits but
surfaced by the dependency lens's re-review, independently verified against
the codebase's own `meta/work/0198-*.md`:

- 🟡 **Dependency**: Acceptance Criterion 1's claim that "no `Command::new`
  for `jj` or `git` remains in the crate's non-test code" cannot be
  satisfied crate-wide, because `cli/vcs-adapters/src/subprocess.rs` also
  hosts `status`/`log`'s separate subprocess path (`run_vcs_text`), which
  0198 owns and may retain indefinitely pending a feasibility
  investigation. Verification of 0198's content further revealed that
  `run_vcs_text` likely reuses the same capped-stdout/environment-scrubbing
  helpers this item's Requirements assumed served `CommandProbe` "solely" —
  Fixed (AC1, the zero-spawn Requirement, Technical Notes, and Dependencies
  all reworded to scope the crate-wide claim to `facts` callers only and
  flag the shared-helper risk explicitly)

### Assessment

The work item is now internally consistent: every claim traced back to the
Amendment's corrections is applied in the body, the two previously-ungated
prerequisite decisions (containment bound, snapshot-on-read dependency) are
now checkable Acceptance Criteria, the sha256 and MPL-2.0 consequences are
captured as both Dependencies and Acceptance Criteria, and the newly
discovered 0198 coupling is scoped explicitly rather than left as a latent
false-green risk in AC1. One minor, previously-identified concern remains
open by design: the MPL-2.0 attribution artefact (if triggered) spans a
different toolchain than the rest of this task, and the reviewer judged it
better tracked as an in-line Acceptance Criterion than split into a
separate item, since the licence *re-check* itself is cheap and the
attribution-artefact work only materialises conditionally. The item is
ready for implementation; no further revision is required before pickup.

---
*Review generated by /accelerator:review-work-item*
